//! `net/minecraft/commands/execution`: the queue 26.1 runs every command on.
//!
//! [`execute_entry`] runs one entry against the live world, so each entry sees
//! what the entries before it wrote; the queue owns the order, the depth and
//! the quota.

mod trace;
use trace::TraceHandle;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bbb_protocol::{component_literal_nbt, CommandNbtPath, NbtSummaryValue};

use super::parse::{self, ParseResult};
use super::run::{self, CommandFeedback, CommandRun};
use super::types::{CommandBossBarMutation, CommandDataTarget, CommandOrigin};
use super::{catalog, translatable, CommandCx, CommandFunctionDefinition, CommandSource};
use crate::world::ServerWorld;

/// `GameRules.MAX_COMMAND_SEQUENCE_LENGTH` (Commands.java:396).
const RULE_MAX_COMMAND_SEQUENCE_LENGTH: &str = "minecraft:max_command_sequence_length";

/// `GameRules.MAX_COMMAND_FORKS` (Commands.java:397).
const RULE_MAX_COMMAND_FORKS: &str = "minecraft:max_command_forks";

/// `ExecutionContext.MAX_QUEUE_DEPTH` (ExecutionContext.java:19).
const MAX_QUEUE_DEPTH: usize = 10_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedFunction {
    pub(crate) id: String,
    /// A command-side macro instance. Plain and world-owned calls resolve by id.
    pub(crate) body: Option<Arc<CommandFunctionDefinition>>,
}

/// The `CommandSourceStack` an entry is bound to (`UnboundEntryAction.bind`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionSource {
    pub(crate) source: CommandSource,
    pub(crate) origin: CommandOrigin,
    /// `CommandSourceStack.withSuppressedOutput` (CommandSourceStack.java:240-259).
    /// Tracer sources always accept output and keep their inherited silent flag.
    pub(crate) silent: bool,
    pub(crate) trace: Option<TraceHandle>,
    /// `CommandSourceStack.entity` plus `textName`/`displayName` after
    /// `withEntity` (`CommandSourceStack.java:151-169`). `None` is the
    /// unforked default: the source itself is the entity.
    pub(crate) executing: Option<CommandSource>,
    /// `CommandSourceStack.resultCallback` (CommandSourceStack.java:63) after
    /// `withCallback` (`:235-238`). `/execute store` is its only producer, so
    /// every other source carries `EMPTY`.
    pub(crate) callback: ResultCallback,
    /// `CommandSourceStack.signingContext`: the player whose unsigned-decoded
    /// arguments this chain carries (ServerGamePacketListenerImpl.java:1536-1549).
    /// `None` is `CommandSigningContext.ANONYMOUS`, under which every message
    /// argument resolves as `PlayerChatMessage.system` (MessageArgument.java:42-52).
    pub(crate) chat_signer: Option<uuid::Uuid>,
}

impl ExecutionSource {
    /// Who `@s`, `getEntity` and the sender decoration read after `execute as`.
    pub(crate) fn run_source(&self) -> &CommandSource {
        self.executing.as_ref().unwrap_or(&self.source)
    }
}

/// `ChainModifiers` (ChainModifiers.java:3-28).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChainModifiers {
    is_forked: bool,
    is_return: bool,
}

impl ChainModifiers {
    /// `ChainModifiers.DEFAULT` (`:5`).
    const DEFAULT: Self = Self {
        is_forked: false,
        is_return: false,
    };

    /// `setForked` (`:17-19`).
    fn set_forked(self) -> Self {
        Self {
            is_forked: true,
            ..self
        }
    }

    /// `isForked` (`:13-15`).
    fn is_forked(self) -> bool {
        self.is_forked
    }

    /// `setReturn` (`:25-27`).
    fn set_return(self) -> Self {
        Self {
            is_return: true,
            ..self
        }
    }

    /// `isReturn` (`:21-23`).
    fn is_return(self) -> bool {
        self.is_return
    }
}

/// `Frame.FrameControl` (Frame.java:18-21) as the two shapes that reach it:
/// `createTopFrame`'s `commandQueue::clear` (ExecutionContext.java:40) and
/// `frameControlForDepth` (`:81-83`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameControl {
    ClearQueue,
    DiscardAtDepth(i32),
}

/// `CommandResultCallback` (CommandResultCallback.java:4-35) over the shapes
/// this dispatcher builds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResultCallback {
    /// `CommandResultCallback.EMPTY` (`:5-14`).
    Empty,
    CountSuccess(ResultAccumulator, super::CommandBlockResultTarget),
    /// `chain` (`:26-35`) once neither half is `EMPTY`; [`ResultCallback::chain`]
    /// answers the two short-circuits without building this.
    Chain(Box<ResultCallback>, Box<ResultCallback>),
    /// `ExecuteCommand.storeValue` (ExecuteCommand.java:414-438) over the
    /// target its modifier resolved.
    Store {
        target: StoreTarget,
        /// `storeResult` (`:276`): `result` stores the value the command
        /// answered, `success` stores its flag as one or zero (`:421`, `:431`).
        store_result: bool,
    },
    /// `queueFunctionsNoReturn`'s `partialResultCallback`
    /// (FunctionCommand.java:209), which sums rather than reports.
    Accumulate(ResultAccumulator),
    /// `FunctionCommand.decorateOutputIfNeeded` (FunctionCommand.java:144-151)
    /// over `FULL_CONTEXT_CALLBACKS.signalResult` (`:57-61`), which reports
    /// what a body returned to the source that asked for the function.
    SignalFunctionResult {
        function: String,
        original: Arc<CommandSource>,
        trace: Option<TraceHandle>,
        inner: Box<ResultCallback>,
    },
    /// IsolatedCall's `output` (`IsolatedCall.java:15-18`): keep the original
    /// source when `IntPredicate` accepts the function result
    /// (`ExecuteCommand.java:1096-1100`).
    FilterFunction(FunctionConditionSink),
}

/// Shared admitted-source list one IsolatedCall family and its Continuation
/// both hold (`ExecuteCommand.java:1068`, `:1096-1112`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionConditionSink {
    keep_nonzero: bool,
    source: Arc<ExecutionSource>,
    admitted: usize,
}

/// What one resolved `/execute store` arm writes, holding everything
/// `wrapStores` (ExecuteCommand.java:274-411) read while the modifier ran.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StoreTarget {
    /// The scoreboard overload (`:414-425`): one
    /// `getOrCreatePlayerScore(name, objective).set(value)` per holder, in the
    /// order `getNamesWithDefaultWildcard` answered.
    Score {
        holders: Vec<(bbb_protocol::JavaString, Option<NbtSummaryValue>)>,
        objective: bbb_protocol::JavaString,
    },
    /// The `CustomBossEvent` overload (`:427-438`), where `into_value` is
    /// `storeIntoValue` (`:432`): `setValue` rather than `setMax`.
    BossBar { id: String, into_value: bool },
    /// `storeData` (`:440-456`) over the accessor its provider resolved.
    ///
    /// `target` is `None` for an accessor whose `setData` refuses, which the
    /// `catch` at `:453-454` swallows.
    Data {
        target: Option<CommandDataTarget>,
        instance: Option<Box<crate::block_entity::BlockEntity>>,
        path: CommandNbtPath,
        tag: parse::StoreDataTag,
        scale: f64,
    },
}

impl StoreTarget {
    fn store(&self, value: i32, cx: &mut CommandCx<'_>) {
        match self {
            Self::Score { holders, objective } => {
                let board = cx.world.scoreboard_mut();
                for (owner, display_name) in holders {
                    board.set_score(owner, display_name.as_ref(), objective, value);
                }
                cx.world.flush_command_scoreboard(cx.events);
            }
            Self::BossBar { id, into_value } => cx.world.mutate_boss_bar(
                if *into_value {
                    CommandBossBarMutation::SetValue {
                        id: id.clone(),
                        value,
                    }
                } else {
                    CommandBossBarMutation::SetMax {
                        id: id.clone(),
                        max: value,
                    }
                },
                cx.events,
            ),
            Self::Data {
                target,
                instance,
                path,
                tag,
                scale,
            } => {
                if let Some(target) = target {
                    cx.world.store_command_data(
                        target,
                        instance.as_deref(),
                        path,
                        tag.tag(value, *scale),
                    );
                }
            }
        }
    }
}

/// `queueFunctionsNoReturn`'s `Accumulator` (FunctionCommand.java:200-222),
/// which every call of one multi-function batch shares with the entry queued
/// behind them.
#[derive(Debug, Clone)]
pub(crate) struct ResultAccumulator(Arc<AtomicU64>);

impl ResultAccumulator {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    /// `Accumulator.add` (`:205-208`), whose sum wraps like the `int` it is.
    fn add(&self, result: i32) {
        // World-thread execution mutates the accumulator sequentially without contention.
        let sum = (self.0.load(Ordering::Relaxed) as u32 as i32).wrapping_add(result);
        self.0
            .store((1_u64 << 32) | u64::from(sum as u32), Ordering::Relaxed);
    }

    /// `anyResult` and `sum` as the trailing task reads them (`:218-222`).
    pub(crate) fn take(&self) -> (bool, i32) {
        let raw = self.0.load(Ordering::Relaxed);
        ((raw >> 32) != 0, raw as u32 as i32)
    }
}

impl PartialEq for ResultAccumulator {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl ResultCallback {
    /// `chain` (CommandResultCallback.java:26-35), whose two `EMPTY`
    /// short-circuits are why most sources never carry a [`Self::Chain`].
    fn chain(first: Self, second: Self) -> Self {
        match (first, second) {
            (Self::Empty, second) => second,
            (first, Self::Empty) => first,
            (first, second) => Self::Chain(Box::new(first), Box::new(second)),
        }
    }

    /// `onResult` (CommandResultCallback.java:16), which `onSuccess` and
    /// `onFailure` (`:18-24`) are the two callers of.
    fn fire(
        &self,
        success: bool,
        result: i32,
        cx: &mut CommandCx<'_>,
        signalled: &mut Vec<(Arc<CommandSource>, CommandFeedback)>,
        admitted_batches: &mut [Vec<Arc<ExecutionSource>>],
    ) {
        match self {
            Self::Empty => {}
            Self::CountSuccess(counter, target) => {
                if success {
                    counter.add(1);
                    cx.world.increment_command_block_success(target);
                }
            }
            Self::Chain(first, second) => {
                first.fire(success, result, cx, signalled, admitted_batches);
                second.fire(success, result, cx, signalled, admitted_batches);
            }
            Self::Store {
                target,
                store_result,
            } => {
                let value = if *store_result {
                    result
                } else {
                    i32::from(success)
                };
                target.store(value, cx);
            }
            Self::Accumulate(accumulator) => accumulator.add(result),
            Self::SignalFunctionResult {
                function,
                original,
                trace,
                inner,
            } => {
                let line = run::success_feedback(
                    super::component(translatable(
                        "commands.function.result",
                        vec![
                            component_literal_nbt(function),
                            NbtSummaryValue::Int(result),
                        ],
                    )),
                    true,
                );
                if let Some(trace) = trace {
                    trace.callback_message(line.message.component().clone());
                } else {
                    signalled.push((Arc::clone(original), line));
                }
                inner.fire(success, result, cx, signalled, admitted_batches);
            }
            Self::FilterFunction(sink) => {
                let keep = if sink.keep_nonzero {
                    result != 0
                } else {
                    result == 0
                };
                if keep {
                    admitted_batches[sink.admitted].push(Arc::clone(&sink.source));
                }
            }
        }
    }
}

/// `Frame` (Frame.java:5-21).
#[derive(Debug, Clone, PartialEq)]
struct Frame {
    depth: i32,
    /// `Frame.returnValueConsumer`.
    result: ResultCallback,
    control: FrameControl,
}

/// `CommandQueueEntry` (CommandQueueEntry.java:3-7).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandQueueEntry {
    frame: Frame,
    action: EntryAction,
}

/// `EntryAction` (EntryAction.java:4-6) over the tasks this dispatcher queues.
#[derive(Debug, Clone, PartialEq)]
enum EntryAction {
    /// `BuildContexts.TopLevel` (BuildContexts.java:153-166) and
    /// `BuildContexts.Continuation` (`:133-151`), which differ only in the
    /// sources and modifiers they open with and the stage they resume at.
    BuildContexts {
        command: Arc<[u16]>,
        parse: Arc<ParseResult>,
        /// Never empty: a continuation over no source is never queued.
        sources: Vec<Arc<ExecutionSource>>,
        modifiers: ChainModifiers,
        /// `ContextChain.nextStage`: the modifier stages already applied.
        consumed_stages: usize,
    },
    /// `ExecuteCommand` (ExecuteCommand.java:24-39).
    ExecuteCommand {
        command: Arc<[u16]>,
        parse: Arc<ParseResult>,
        source: Arc<ExecutionSource>,
        /// `BuildContexts.java:109-113`: a returning chain chains the frame's
        /// return consumer onto the source's callback.
        reports_to_frame: bool,
        /// `ChainModifiers.isForked`, which silences the command's own
        /// exception (ContextChain.java:68-73, ExecuteCommand.java:34-35).
        forked: bool,
    },
    /// `CallFunction` (CallFunction.java:24-36) over the id the argument
    /// resolved to; the compiled body is the world's library entry.
    CallFunction {
        function: String,
        body: Option<Arc<CommandFunctionDefinition>>,
        source: Arc<ExecutionSource>,
        result: ResultCallback,
        return_parent_frame: bool,
        trace_root: bool,
    },
    TraceComplete {
        source: Arc<ExecutionSource>,
        message: NbtSummaryValue,
    },
    /// `BuildContexts.Unbound` (BuildContexts.java:168-177) over one compiled
    /// body line, whose `ContextChain` this server rebuilds by parsing the line
    /// against the world the line before it left.
    FunctionLine {
        function: Arc<CommandFunctionDefinition>,
        index: usize,
        source: Arc<ExecutionSource>,
    },
    /// `ContinuationTask` (ContinuationTask.java:22-28): the rest of a body of
    /// three entries or more, held as one entry so the queue stays short
    /// however deep the call tree goes.
    BodyContinuation {
        function: Arc<CommandFunctionDefinition>,
        next: usize,
        source: Arc<ExecutionSource>,
    },
    /// `FallthroughTask` (FallthroughTask.java:16-19).
    Fallthrough,
    /// The `UnboundEntryAction` `queueFunctionsNoReturn` queues behind a
    /// multi-function batch whose caller carries a callback
    /// (FunctionCommand.java:218-222).
    FunctionResultSum {
        accumulator: ResultAccumulator,
        callback: ResultCallback,
    },
    /// `IsolatedCall` (IsolatedCall.java:21-25) for
    /// `scheduleFunctionConditionsAndTest` (`ExecuteCommand.java:1101-1107`).
    IsolatedCall {
        functions: Vec<String>,
        function_source: Arc<ExecutionSource>,
        sink: FunctionConditionSink,
    },
    /// `BuildContexts.Continuation` queued after the IsolatedCalls
    /// (`ExecuteCommand.java:1110-1112`).
    FunctionConditionContinuation {
        command: Arc<[u16]>,
        parse: Arc<ParseResult>,
        modifiers: ChainModifiers,
        consumed_stages: usize,
        admitted: usize,
    },
}

/// `ExecutionContext` (ExecutionContext.java:18-146).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionContext {
    /// `ExecutionContext.commandLimit` (`:21`).
    command_limit: i32,
    /// `ExecutionContext.forkLimit` (`:22`).
    fork_limit: i32,
    /// `ExecutionContext.commandQuota` (`:25`).
    command_quota: i32,
    /// `ExecutionContext.commandQueue` (`:27`).
    queue: VecDeque<CommandQueueEntry>,
    /// `ExecutionContext.newTopCommands` (`:28`).
    new_top: Vec<CommandQueueEntry>,
    /// `ExecutionContext.queueOverflow` (`:26`).
    overflow: bool,
    /// `ExecutionContext.currentFrameDepth` (`:29`).
    current_frame_depth: i32,
    tracer: Option<TraceHandle>,
    admitted_batches: Vec<Vec<Arc<ExecutionSource>>>,
}

impl ExecutionContext {
    /// `Commands.executeCommandInContext` (Commands.java:394-399): both quotas
    /// are read off the level's game rules when the context opens, so a
    /// `/gamerule` inside it cannot change the budget it is spending.
    pub(crate) fn open(world: &ServerWorld) -> Self {
        let command_limit = world
            .integer_game_rule(RULE_MAX_COMMAND_SEQUENCE_LENGTH)
            .max(1);
        Self {
            command_limit,
            fork_limit: world.integer_game_rule(RULE_MAX_COMMAND_FORKS),
            command_quota: command_limit,
            queue: VecDeque::new(),
            new_top: Vec::new(),
            overflow: false,
            current_frame_depth: 0,
            tracer: None,
            admitted_batches: Vec::new(),
        }
    }

    fn allocate_admitted_batch(&mut self) -> usize {
        let batch = self.admitted_batches.len();
        self.admitted_batches.push(Vec::new());
        batch
    }

    #[cfg(test)]
    pub(crate) fn quotas(&self) -> (i32, i32) {
        (self.command_limit, self.fork_limit)
    }

    /// `ExecutionContext.createTopFrame` (ExecutionContext.java:38-45).
    fn create_top_frame(&self) -> Frame {
        if self.current_frame_depth == 0 {
            return Frame {
                depth: 0,
                result: ResultCallback::Empty,
                control: FrameControl::ClearQueue,
            };
        }
        let depth = self.current_frame_depth + 1;
        Frame {
            depth,
            result: ResultCallback::Empty,
            control: FrameControl::DiscardAtDepth(depth),
        }
    }

    /// `ExecutionContext.queueInitialCommandExecution` (`:53-57`).
    pub(super) fn queue_initial_command_execution(
        &mut self,
        command: Arc<[u16]>,
        parse: ParseResult,
        source: ExecutionSource,
    ) {
        let frame = self.create_top_frame();
        self.queue_next(CommandQueueEntry {
            frame,
            action: EntryAction::BuildContexts {
                command,
                parse: Arc::new(parse),
                sources: vec![Arc::new(source)],
                modifiers: ChainModifiers::DEFAULT,
                consumed_stages: 0,
            },
        });
    }

    /// `ExecutionContext.queueInitialFunctionCall` (`:47-51`).
    pub(crate) fn queue_initial_function_call(&mut self, function: &str, source: ExecutionSource) {
        let frame = self.create_top_frame();
        self.queue_next(CommandQueueEntry {
            frame,
            action: EntryAction::CallFunction {
                function: function.to_string(),
                body: None,
                source: Arc::new(source),
                result: ResultCallback::Empty,
                return_parent_frame: false,
                trace_root: false,
            },
        });
    }

    /// `ExecutionContext.queueNext` (`:65-73`) with `handleQueueOverflow`
    /// (`:59-63`).
    fn queue_next(&mut self, entry: CommandQueueEntry) {
        if self.new_top.len() + self.queue.len() > MAX_QUEUE_DEPTH {
            self.overflow = true;
            self.new_top.clear();
            self.queue.clear();
        }
        if !self.overflow {
            self.new_top.push(entry);
        }
    }

    /// `ExecutionContext.pushNewCommands` (`:112-118`): the entries a step
    /// asked for go to the front in the order they were asked for, which is
    /// what makes a call tree run depth-first.
    fn push_new_commands(&mut self) {
        for entry in self.new_top.drain(..).rev() {
            self.queue.push_front(entry);
        }
    }

    /// `ExecutionContext.discardAtDepthOrHigher` (`:75-79`).
    fn discard_at_depth_or_higher(&mut self, depth: i32) {
        while self
            .queue
            .front()
            .is_some_and(|entry| entry.frame.depth >= depth)
        {
            self.queue.pop_front();
        }
    }

    /// `Frame.discard` (Frame.java:14-16) through the control the frame holds.
    fn discard_frame(&mut self, frame: &Frame) {
        match frame.control {
            FrameControl::ClearQueue => self.queue.clear(),
            FrameControl::DiscardAtDepth(depth) => self.discard_at_depth_or_higher(depth),
        }
    }

    /// `ExecutionContext.incrementCost` (`:136-138`).
    fn increment_cost(&mut self) {
        self.command_quota -= 1;
    }

    /// `ContinuationTask.schedule` (ContinuationTask.java:30-47): a body of no
    /// entries queues nothing, one or two queue themselves, and three or more
    /// queue one continuation that hands them out a line at a time.
    fn schedule_body(
        &mut self,
        function: &Arc<CommandFunctionDefinition>,
        frame: &Frame,
        source: &Arc<ExecutionSource>,
    ) {
        let line = |index: usize| CommandQueueEntry {
            frame: frame.clone(),
            action: EntryAction::FunctionLine {
                function: Arc::clone(function),
                index,
                source: Arc::clone(source),
            },
        };
        match function.plain_entries().len() {
            0 => {}
            1 => self.queue_next(line(0)),
            2 => {
                self.queue_next(line(0));
                self.queue_next(line(1));
            }
            _ => self.queue_next(CommandQueueEntry {
                frame: frame.clone(),
                action: EntryAction::BodyContinuation {
                    function: Arc::clone(function),
                    next: 0,
                    source: Arc::clone(source),
                },
            }),
        }
    }
}

/// What one entry produced.
pub(crate) struct EntryOutput {
    /// The source the entry's own command reported to, absent for an entry
    /// that ran none.
    pub(crate) source: Option<Arc<ExecutionSource>>,
    pub(crate) run: CommandRun,
    /// `FULL_CONTEXT_CALLBACKS.signalResult` lines, each for the source that
    /// asked for the function rather than for the body's own.
    pub(crate) signalled: Vec<(Arc<CommandSource>, CommandFeedback)>,
    pub(crate) tracer: Option<TraceHandle>,
    pub(crate) trace_return: Option<(i32, Arc<[u16]>)>,
}

impl EntryOutput {
    fn none() -> Self {
        Self {
            source: None,
            run: CommandRun::default(),
            signalled: Vec::new(),
            tracer: None,
            trace_return: None,
        }
    }

    fn ran(source: &Arc<ExecutionSource>, run: CommandRun) -> Self {
        Self {
            source: Some(Arc::clone(source)),
            run,
            signalled: Vec::new(),
            tracer: None,
            trace_return: None,
        }
    }
}

/// `ExecutionContext.runCommandQueue` (ExecutionContext.java:85-110).
pub(crate) fn run_command_queue<F>(context: &mut ExecutionContext, mut step: F)
where
    F: FnMut(&mut ExecutionContext, CommandQueueEntry),
{
    context.push_new_commands();
    loop {
        if context.command_quota <= 0 {
            // `:89-92`. Vanilla's whole report is one `LOGGER.info` on the
            // server console, which this crate owns no channel for.
            break;
        }
        let Some(entry) = context.queue.pop_front() else {
            break;
        };
        context.current_frame_depth = entry.frame.depth;
        step(context, entry);
        if context.overflow {
            // `:101-104`, the other `LOGGER`-only break.
            break;
        }
        context.push_new_commands();
    }
    context.current_frame_depth = 0;
    if let Some(tracer) = &context.tracer {
        tracer.close();
    }
}

/// `CommandQueueEntry.execute` (CommandQueueEntry.java:4-6) against the live
/// world.
///
/// `bodies` is `ServerFunctionLibrary.getFunction`: a `CallFunction` names an
/// id and the body it schedules is looked up here.
pub(crate) fn execute_entry(
    context: &mut ExecutionContext,
    entry: CommandQueueEntry,
    cx: &mut CommandCx<'_>,
    bodies: &dyn Fn(&str) -> Option<Arc<CommandFunctionDefinition>>,
) -> EntryOutput {
    let trace_return = match &entry.action {
        EntryAction::ExecuteCommand { command, .. } => {
            Some((entry.frame.depth, Arc::clone(command)))
        }
        _ => None,
    };
    let mut output = execute_entry_inner(context, entry, cx, bodies);
    output.tracer = context.tracer.clone();
    output.trace_return = trace_return;
    output
}

fn execute_entry_inner(
    context: &mut ExecutionContext,
    entry: CommandQueueEntry,
    cx: &mut CommandCx<'_>,
    bodies: &dyn Fn(&str) -> Option<Arc<CommandFunctionDefinition>>,
) -> EntryOutput {
    let CommandQueueEntry { frame, action } = entry;
    match action {
        EntryAction::BuildContexts {
            command,
            parse,
            sources,
            modifiers,
            consumed_stages,
        } => {
            if consumed_stages == 0 {
                if let Some(tracer) = &context.tracer {
                    tracer.command(frame.depth, &command);
                }
            }
            build_contexts(
                context,
                &frame,
                &command,
                &parse,
                &sources,
                modifiers,
                consumed_stages,
                cx,
                bodies,
            )
        }
        EntryAction::TraceComplete { source, message } => {
            let mut run = CommandRun::default();
            run.feedback
                .push(run::success_feedback(super::component(message), true));
            EntryOutput::ran(&source, run)
        }
        EntryAction::ExecuteCommand {
            command: _,
            parse,
            source,
            reports_to_frame,
            forked,
        } => {
            // `ExecuteCommand.execute` (ExecuteCommand.java:24-39).
            context.increment_cost();
            cx.origin = source.origin.clone();
            cx.chat_signer = source.chat_signer;
            let run = run::run(&parse, source.run_source(), cx, forked);
            let mut signalled = Vec::new();
            // `ExecutionCommandSource.resultConsumer`
            // (ExecutionCommandSource.java:31-33) fires the source's own
            // callback, onto which `BuildContexts.java:109-113` chained the
            // frame's return consumer when the chain is returning.
            let callback = if reports_to_frame {
                ResultCallback::chain(source.callback.clone(), frame.result)
            } else {
                source.callback.clone()
            };
            let (success, result) = match run.result {
                Some(result) => (true, result),
                None => (false, 0),
            };
            callback.fire(
                success,
                result,
                cx,
                &mut signalled,
                &mut context.admitted_batches,
            );
            EntryOutput {
                source: Some(source),
                run,
                signalled,
                tracer: None,
                trace_return: None,
            }
        }
        EntryAction::CallFunction {
            function,
            body,
            source,
            result,
            return_parent_frame,
            trace_root,
        } => {
            // `CallFunction.execute` (CallFunction.java:24-35).
            context.increment_cost();
            let Some(function) = body.or_else(|| bodies(&function)) else {
                return EntryOutput::none();
            };
            if let Some(tracer) = &context.tracer {
                if trace_root {
                    tracer.root(&function.id);
                }
                tracer.call(frame.depth, &function.id, function.plain_entries().len());
            }
            let depth = frame.depth + 1;
            let body_frame = Frame {
                depth,
                result,
                control: if return_parent_frame {
                    frame.control
                } else {
                    FrameControl::DiscardAtDepth(depth)
                },
            };
            context.schedule_body(&function, &body_frame, &source);
            EntryOutput::none()
        }
        EntryAction::FunctionLine {
            function,
            index,
            source,
        } => {
            // `BuildContexts.Unbound.execute` (BuildContexts.java:173-176) over
            // a line parsed now rather than at load, because the world a body
            // command reads is the one the line before it left.
            let line: Arc<[u16]> = Arc::clone(&function.plain_entries()[index]);
            let permission = cx.world.function_compilation_permission();
            let parse = cx.world.cached_function_parse(&line, permission, |world| {
                parse::parse(&line, permission, world)
            });
            if let Some(tracer) = &context.tracer {
                tracer.command(frame.depth, &line);
            }
            build_contexts(
                context,
                &frame,
                &line,
                &parse,
                std::slice::from_ref(&source),
                ChainModifiers::DEFAULT,
                0,
                cx,
                bodies,
            )
        }
        EntryAction::BodyContinuation {
            function,
            next,
            source,
        } => {
            // `ContinuationTask.execute` (ContinuationTask.java:22-28).
            context.queue_next(CommandQueueEntry {
                frame: frame.clone(),
                action: EntryAction::FunctionLine {
                    function: Arc::clone(&function),
                    index: next,
                    source: Arc::clone(&source),
                },
            });
            if next + 1 < function.plain_entries().len() {
                context.queue_next(CommandQueueEntry {
                    frame,
                    action: EntryAction::BodyContinuation {
                        function,
                        next: next + 1,
                        source,
                    },
                });
            }
            EntryOutput::none()
        }
        EntryAction::Fallthrough => {
            // `FallthroughTask.execute` (FallthroughTask.java:16-19).
            let mut signalled = Vec::new();
            frame
                .result
                .fire(false, 0, cx, &mut signalled, &mut context.admitted_batches);
            context.discard_frame(&frame);
            EntryOutput {
                source: None,
                run: CommandRun::default(),
                signalled,
                tracer: None,
                trace_return: None,
            }
        }
        EntryAction::FunctionResultSum {
            accumulator,
            callback,
        } => {
            // `queueFunctionsNoReturn`'s trailing task
            // (FunctionCommand.java:218-222), which spends no quota of its own.
            let (any_result, sum) = accumulator.take();
            let mut signalled = Vec::new();
            if any_result {
                callback.fire(true, sum, cx, &mut signalled, &mut context.admitted_batches);
            }
            EntryOutput {
                source: None,
                run: CommandRun::default(),
                signalled,
                tracer: None,
                trace_return: None,
            }
        }
        EntryAction::IsolatedCall {
            functions,
            function_source,
            sink,
        } => {
            // IsolatedCall.java:21-25: a new frame whose return consumer is
            // the filter, then CallFunction + Fallthrough on that frame
            // (ExecuteCommand.java:1101-1107).
            let depth = frame.depth + 1;
            let isolated = Frame {
                depth,
                result: ResultCallback::FilterFunction(sink),
                control: FrameControl::DiscardAtDepth(depth),
            };
            for function in functions {
                context.queue_next(CommandQueueEntry {
                    frame: isolated.clone(),
                    action: EntryAction::CallFunction {
                        function,
                        body: None,
                        source: Arc::clone(&function_source),
                        result: isolated.result.clone(),
                        return_parent_frame: true,
                        trace_root: false,
                    },
                });
            }
            context.queue_next(CommandQueueEntry {
                frame: isolated,
                action: EntryAction::Fallthrough,
            });
            EntryOutput::none()
        }
        EntryAction::FunctionConditionContinuation {
            command,
            parse,
            modifiers,
            consumed_stages,
            admitted,
        } => {
            // `BuildContexts.Continuation` (BuildContexts.java:133-151) over
            // every admitted source at once, so the next stage tests them all
            // before any of them runs.
            let admitted = std::mem::take(&mut context.admitted_batches[admitted]);
            build_contexts(
                context,
                &frame,
                &command,
                &parse,
                &admitted,
                modifiers,
                consumed_stages,
                cx,
                bodies,
            )
        }
    }
}

fn fork_limit_reached(next: usize, added: usize, fork_limit: i32) -> bool {
    next.saturating_add(added) >= usize::try_from(fork_limit).unwrap_or(0)
}

/// A selector predicate that throws inside a fork's `getOptionalEntities` is no
/// `CommandSyntaxException`, so the forked flag cannot silence it: it reaches
/// `Commands.performCommand`'s catch-all (Commands.java:333-349).
fn unanswered_fork(exec: ExecutionSource, message: NbtSummaryValue) -> EntryOutput {
    let mut run = CommandRun::default();
    run.fail(message);
    EntryOutput::ran(&Arc::new(exec), run)
}

/// `BuildContexts.execute` (BuildContexts.java:40-119) over the chain a parse
/// flattened to.
#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per BuildContexts field"
)]
fn build_contexts(
    context: &mut ExecutionContext,
    frame: &Frame,
    command: &Arc<[u16]>,
    parse: &Arc<ParseResult>,
    sources: &[Arc<ExecutionSource>],
    modifiers: ChainModifiers,
    consumed_stages: usize,
    cx: &mut CommandCx<'_>,
    bodies: &dyn Fn(&str) -> Option<Arc<CommandFunctionDefinition>>,
) -> EntryOutput {
    let world = &mut *cx.world;
    let Some(node) = parse.executable_node() else {
        // A body line that no longer flattens, which `finishParsing`
        // (Commands.java:363-388) reports the same way a typed one is.
        return EntryOutput {
            source: sources.first().cloned(),
            run: super::parse_failure_run(command, parse),
            signalled: Vec::new(),
            tracer: None,
            trace_return: None,
        };
    };
    let mut modifiers = modifiers;
    let mut current = sources
        .iter()
        .map(|source| (**source).clone())
        .collect::<Vec<_>>();
    for (index, stage) in parse::chain_stages(parse)
        .into_iter()
        .enumerate()
        .skip(consumed_stages)
    {
        match stage {
            parse::ChainStage::ExecuteIn(dimension) => {
                // `:64-65`: a redirect modifier costs one before it is applied.
                context.increment_cost();
                let forked = modifiers.is_forked();
                let mut next = Vec::new();
                for prep in current {
                    match parse::apply_execute_in(&dimension, world, prep.origin.clone()) {
                        Ok(moved) => next.push(ExecutionSource {
                            origin: moved,
                            ..prep
                        }),
                        Err(message) => {
                            // `:78-82`: `handleError` sends only when the
                            // chain is not already forked.
                            if !forked {
                                let mut run = CommandRun::default();
                                run.fail(message);
                                return EntryOutput::ran(&Arc::new(prep), run);
                            } else if let Some(tracer) = &context.tracer {
                                tracer.error(&message);
                            }
                        }
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecutePositioned(coordinates) => {
                context.increment_cost();
                current = current
                    .into_iter()
                    .map(|prep| {
                        let origin =
                            parse::apply_execute_positioned(coordinates, prep.origin.clone());
                        ExecutionSource { origin, ..prep }
                    })
                    .collect();
            }
            parse::ChainStage::ExecutePositionedOver(heightmap_kind_id) => {
                context.increment_cost();
                let forked = modifiers.is_forked();
                let mut next = Vec::new();
                for prep in current {
                    match parse::apply_execute_positioned_over(
                        heightmap_kind_id,
                        world,
                        prep.origin.clone(),
                    ) {
                        Ok(moved) => next.push(ExecutionSource {
                            origin: moved,
                            ..prep
                        }),
                        Err(message) => {
                            if !forked {
                                let mut run = CommandRun::default();
                                run.fail(message);
                                return EntryOutput::ran(&Arc::new(prep), run);
                            } else if let Some(tracer) = &context.tracer {
                                tracer.error(&message);
                            }
                        }
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteRotated(coordinates) => {
                context.increment_cost();
                current = current
                    .into_iter()
                    .map(|prep| {
                        let origin = parse::apply_execute_rotated(coordinates, prep.origin.clone());
                        ExecutionSource { origin, ..prep }
                    })
                    .collect();
            }
            parse::ChainStage::ExecuteFacing(coordinates) => {
                context.increment_cost();
                current = current
                    .into_iter()
                    .map(|prep| {
                        let origin = parse::apply_execute_facing(coordinates, prep.origin.clone());
                        ExecutionSource { origin, ..prep }
                    })
                    .collect();
            }
            parse::ChainStage::ExecuteAlign { x, y, z } => {
                context.increment_cost();
                current = current
                    .into_iter()
                    .map(|prep| {
                        let origin = parse::apply_execute_align(x, y, z, prep.origin.clone());
                        ExecutionSource { origin, ..prep }
                    })
                    .collect();
            }
            parse::ChainStage::ExecuteAnchored(anchor) => {
                context.increment_cost();
                current = current
                    .into_iter()
                    .map(|prep| {
                        let origin = parse::apply_execute_anchored(anchor, prep.origin.clone());
                        ExecutionSource { origin, ..prep }
                    })
                    .collect();
            }
            parse::ChainStage::ExecuteConditional {
                expected,
                condition,
            } => {
                // `:54-56`: `addConditional` builds a forked node, so the flag
                // is set before the modifier runs and a refusal inside it never
                // reaches the source (`CommandSourceStack.java:577-579`).
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    // `expect` (`ExecuteCommand.java:873-875`) keeps the one
                    // source or none of it.
                    match run::execute_condition_holds(
                        &condition,
                        prep.run_source(),
                        world,
                        &prep.origin,
                    ) {
                        Ok(held) if held == expected => next.push(prep),
                        Err(message) => {
                            if let Some(tracer) = &context.tracer {
                                tracer.error(&message);
                            }
                        }
                        Ok(_) => {}
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteStore {
                store_result,
                target,
            } => {
                // `wrapStores` registers a plain `redirect`, so the modifier is
                // not forked and a refusal inside it still reaches the source.
                context.increment_cost();
                let forked = modifiers.is_forked();
                let mut next = Vec::new();
                for prep in current {
                    match run::resolve_execute_store(
                        &target,
                        prep.run_source(),
                        world,
                        &prep.origin,
                    ) {
                        Ok(target) => {
                            // `withCallback(.., CommandResultCallback::chain)`
                            // (`ExecuteCommand.java:418`, `:430`), so two stores
                            // in one chain both fire, in the order they were
                            // typed.
                            let callback = ResultCallback::chain(
                                prep.callback.clone(),
                                ResultCallback::Store {
                                    target,
                                    store_result,
                                },
                            );
                            next.push(ExecutionSource { callback, ..prep });
                        }
                        Err(message) => {
                            if !forked {
                                let mut run = CommandRun::default();
                                run.fail(message);
                                return EntryOutput::ran(&Arc::new(prep), run);
                            } else if let Some(tracer) = &context.tracer {
                                tracer.error(&message);
                            }
                        }
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteAs(selector) => {
                // `:54-56`: a forked node sets the flag before the modifier runs.
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let entities = match world.with_entity_reads(|reads| {
                        run::optional_command_entities(
                            &selector,
                            prep.run_source(),
                            &prep.origin,
                            reads,
                        )
                        .map(|entities| {
                            entities
                                .into_iter()
                                .map(|entity| {
                                    (
                                        run::command_entity_source(
                                            entity,
                                            prep.source.permission,
                                            reads,
                                        ),
                                        entity.eye_height(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    }) {
                        Ok(entities) => entities,
                        Err(message) => return unanswered_fork(prep, message),
                    };
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        // `:72-74` through `handleError(..., forked=true)`
                        // (`CommandSourceStack.java:577-579`): no sendFailure.
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for (executing, eye_height) in entities {
                        // `withEntity` (`CommandSourceStack.java:151-169`)
                        // replaces the entity `EYES` reads its height off.
                        next.push(ExecutionSource {
                            origin: CommandOrigin {
                                eye_height: Some(eye_height),
                                ..prep.origin.clone()
                            },
                            executing: Some(executing),
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteSummon(resource_id) => {
                context.increment_cost();
                let forked = modifiers.is_forked();
                let mut next = Vec::new();
                for prep in current {
                    match run::summon::create_entity(
                        world,
                        &prep.origin,
                        &resource_id,
                        prep.origin.position,
                        None,
                    ) {
                        Ok(id) => {
                            let reads = world.entity_reads();
                            let entity = reads
                                .command_entity_by_id(&prep.origin.level, id)
                                .expect("a summoned entity is published in its source level");
                            next.push(ExecutionSource {
                                origin: CommandOrigin {
                                    eye_height: Some(entity.eye_height()),
                                    ..prep.origin.clone()
                                },
                                executing: Some(run::command_entity_source(
                                    entity,
                                    prep.source.permission,
                                    &reads,
                                )),
                                ..prep
                            });
                        }
                        Err(message) => {
                            if !forked {
                                let mut run = CommandRun::default();
                                run.fail(message);
                                return EntryOutput::ran(&Arc::new(prep), run);
                            } else if let Some(tracer) = &context.tracer {
                                tracer.error(&message);
                            }
                        }
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteOn(relation) => {
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let reads = world.entity_reads();
                    let entities = reads.related_command_entities(prep.run_source(), relation);
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for entity in entities {
                        next.push(ExecutionSource {
                            origin: CommandOrigin {
                                eye_height: Some(entity.eye_height()),
                                ..prep.origin.clone()
                            },
                            executing: Some(run::command_entity_source(
                                entity,
                                prep.source.permission,
                                &reads,
                            )),
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteAt(selector) => {
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let entities = match world.with_entity_reads(|reads| {
                        run::optional_command_entities(
                            &selector,
                            prep.run_source(),
                            &prep.origin,
                            reads,
                        )
                        .map(|entities| {
                            entities
                                .into_iter()
                                .map(|entity| {
                                    (
                                        entity.dimension().clone(),
                                        entity.position(),
                                        entity.rotation(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    }) {
                        Ok(entities) => entities,
                        Err(message) => return unanswered_fork(prep, message),
                    };
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for (dimension, position, rotation) in entities {
                        let origin = prep
                            .origin
                            .clone()
                            .with_level(world, &dimension)
                            .with_position(position)
                            .with_rotation(rotation);
                        next.push(ExecutionSource {
                            origin,
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecutePositionedAs(selector) => {
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let entities = match world.with_entity_reads(|reads| {
                        run::optional_command_entities(
                            &selector,
                            prep.run_source(),
                            &prep.origin,
                            reads,
                        )
                        .map(|entities| {
                            entities
                                .into_iter()
                                .map(|entity| {
                                    run::apply_execute_positioned_as_entity(
                                        entity,
                                        prep.origin.clone(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    }) {
                        Ok(entities) => entities,
                        Err(message) => return unanswered_fork(prep, message),
                    };
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for origin in entities {
                        next.push(ExecutionSource {
                            origin,
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteRotatedAs(selector) => {
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let entities = match world.with_entity_reads(|reads| {
                        run::optional_command_entities(
                            &selector,
                            prep.run_source(),
                            &prep.origin,
                            reads,
                        )
                        .map(|entities| {
                            entities
                                .into_iter()
                                .map(|entity| {
                                    run::apply_execute_rotated_as_entity(
                                        entity,
                                        prep.origin.clone(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    }) {
                        Ok(entities) => entities,
                        Err(message) => return unanswered_fork(prep, message),
                    };
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for origin in entities {
                        next.push(ExecutionSource {
                            origin,
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ExecuteFacingEntity { targets, anchor } => {
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let mut next = Vec::new();
                for prep in current {
                    let entities = match world.with_entity_reads(|reads| {
                        run::optional_command_entities(
                            &targets,
                            prep.run_source(),
                            &prep.origin,
                            reads,
                        )
                        .map(|entities| {
                            entities
                                .into_iter()
                                .map(|entity| {
                                    run::apply_execute_facing_entity(
                                        entity,
                                        anchor,
                                        prep.origin.clone(),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    }) {
                        Ok(entities) => entities,
                        Err(message) => return unanswered_fork(prep, message),
                    };
                    if fork_limit_reached(next.len(), entities.len(), context.fork_limit) {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&translatable(
                                "command.forkLimit",
                                vec![NbtSummaryValue::Int(context.fork_limit)],
                            ));
                        }
                        return EntryOutput::none();
                    }
                    for origin in entities {
                        next.push(ExecutionSource {
                            origin,
                            ..prep.clone()
                        });
                    }
                }
                current = next;
            }
            parse::ChainStage::ReturnRun => {
                // `ReturnCommand.ReturnFromCommandCustomModifier.apply`
                // (ReturnCommand.java:41-59).
                if current.is_empty() {
                    if modifiers.is_return() {
                        context.queue_next(CommandQueueEntry {
                            frame: frame.clone(),
                            action: EntryAction::Fallthrough,
                        });
                    }
                    return EntryOutput::none();
                }
                context.discard_frame(frame);
                context.queue_next(CommandQueueEntry {
                    frame: frame.clone(),
                    action: EntryAction::BuildContexts {
                        command: Arc::clone(command),
                        parse: Arc::clone(parse),
                        sources: current.into_iter().map(Arc::new).collect(),
                        modifiers: modifiers.set_return(),
                        consumed_stages: index + 1,
                    },
                });
                return EntryOutput::none();
            }
            parse::ChainStage::ExecuteIfFunction { expected, tag, id } => {
                // `ExecuteIfFunctionCustomModifier.apply`
                // (`ExecuteCommand.java:1133-1150`) through
                // `scheduleFunctionConditionsAndTest` (`:1057-1113`).
                modifiers = modifiers.set_forked();
                context.increment_cost();
                let functions = match current.first() {
                    Some(_) => run::resolve_functions(tag, &id, world),
                    None => Ok(Vec::new()),
                };
                let functions = match functions {
                    Ok(functions) => functions,
                    Err(message) => {
                        if let Some(tracer) = &context.tracer {
                            tracer.error(&message);
                        }
                        return EntryOutput::none();
                    }
                };
                if functions.is_empty() {
                    return EntryOutput::none();
                }
                let functions: Vec<String> = match current.first() {
                    Some(_) => {
                        let mut instantiated = Vec::new();
                        for id in functions {
                            match run::instantiate_function(&id, None, world) {
                                Ok(_) => instantiated.push(id),
                                Err(reason) => {
                                    let message = match reason {
                                        run::InstantiationError::Refused(message) => message,
                                        run::InstantiationError::CommandTooLong => {
                                            translatable("command.failed", Vec::new())
                                        }
                                    };
                                    if let Some(tracer) = &context.tracer {
                                        tracer.error(&translatable(
                                            "commands.execute.function.instantiationFailure",
                                            vec![component_literal_nbt(&id), message],
                                        ));
                                    }
                                    break;
                                }
                            }
                        }
                        instantiated
                    }
                    None => functions,
                };
                let admitted = context.allocate_admitted_batch();
                for prep in &current {
                    let original = Arc::new(prep.clone());
                    // `source.clearCallbacks()` (`:1095`): the condition's own
                    // bodies fire none of the caller's callback, while the
                    // source it admits back into the chain keeps all of it.
                    let function_source = Arc::new(ExecutionSource {
                        callback: ResultCallback::Empty,
                        ..run::modify_sender_for_execution(&original)
                    });
                    context.queue_next(CommandQueueEntry {
                        frame: frame.clone(),
                        action: EntryAction::IsolatedCall {
                            functions: functions.clone(),
                            function_source,
                            sink: FunctionConditionSink {
                                keep_nonzero: expected,
                                source: Arc::clone(&original),
                                admitted,
                            },
                        },
                    });
                }
                context.queue_next(CommandQueueEntry {
                    frame: frame.clone(),
                    action: EntryAction::FunctionConditionContinuation {
                        command: Arc::clone(command),
                        parse: Arc::clone(parse),
                        modifiers,
                        consumed_stages: index + 1,
                        admitted,
                    },
                });
                return EntryOutput::none();
            }
        }
    }
    if current.is_empty() {
        // `:96-99`.
        if modifiers.is_return() {
            context.queue_next(CommandQueueEntry {
                frame: frame.clone(),
                action: EntryAction::Fallthrough,
            });
        }
        return EntryOutput::none();
    }
    if catalog::is_custom_executor(node) {
        // `:102-107`: a `CustomCommandExecutor` runs here rather than through a
        // queued `ExecuteCommand`, which is why it spends no command quota.
        let mut combined = CommandRun::default();
        let mut last = None;
        let mut signalled = Vec::new();
        for prep in current {
            let source = Arc::new(prep);
            let mut control = ExecutionControl {
                context: &mut *context,
                frame: frame.clone(),
                cx: &mut *cx,
                bodies,
                signalled: &mut signalled,
            };
            let run = run::run_custom(
                node,
                parse,
                &source,
                modifiers.is_return(),
                modifiers.is_forked(),
                &mut control,
            );
            combined.absorb(run);
            last = Some(source);
        }
        return EntryOutput {
            source: last,
            run: combined,
            signalled,
            tracer: None,
            trace_return: None,
        };
    }
    if modifiers.is_return() {
        // `:109-113`: a returning chain runs its first source alone.
        current.truncate(1);
    }
    // `:115-116`: one `ExecuteCommand` per remaining source
    // (`ContinuationTask.schedule`).
    for prep in current {
        context.queue_next(CommandQueueEntry {
            frame: frame.clone(),
            action: EntryAction::ExecuteCommand {
                command: Arc::clone(command),
                parse: Arc::clone(parse),
                source: Arc::new(prep),
                reports_to_frame: modifiers.is_return(),
                forked: modifiers.is_forked(),
            },
        });
    }
    EntryOutput::none()
}

/// `ExecutionControl` (ExecutionControl.java:6-37): what a custom executor
/// holds to reach the frame it is running in.
pub(crate) struct ExecutionControl<'a, 'w> {
    context: &'a mut ExecutionContext,
    frame: Frame,
    cx: &'a mut CommandCx<'w>,
    bodies: &'a dyn Fn(&str) -> Option<Arc<CommandFunctionDefinition>>,
    signalled: &'a mut Vec<(Arc<CommandSource>, CommandFeedback)>,
}

impl ExecutionControl<'_, '_> {
    pub(crate) fn world(&self) -> &ServerWorld {
        self.cx.world
    }

    pub(crate) fn world_mut(&mut self) -> &mut ServerWorld {
        self.cx.world
    }

    pub(super) fn check_trace_allowed(&self, is_return: bool) -> Result<(), NbtSummaryValue> {
        if is_return {
            return Err(translatable(
                "commands.debug.function.noReturnRun",
                Vec::new(),
            ));
        }
        if self.context.tracer.is_some() {
            return Err(translatable(
                "commands.debug.function.noRecursion",
                Vec::new(),
            ));
        }
        Ok(())
    }

    pub(super) fn trace_functions(
        &mut self,
        functions: Vec<String>,
        source: &Arc<ExecutionSource>,
    ) -> CommandRun {
        self.context.start_trace(
            &self.frame,
            functions,
            Arc::clone(source),
            self.cx.world,
            self.bodies,
        )
    }

    /// `Frame.discard` (Frame.java:14-16) over `ExecutionControl.currentFrame`
    /// (`:32-35`).
    pub(crate) fn discard_current_frame(&mut self) {
        self.context.discard_frame(&self.frame);
    }

    /// `sender.callback().onSuccess(..)` / `.onFailure()`
    /// (CommandResultCallback.java:18-24) as a custom executor fires it.
    ///
    /// The source keeps its own consumer here: only the `ExecuteCommand` branch
    /// chains the frame onto it (`BuildContexts.java:102-113`), so a custom
    /// executor reaches the two separately.
    pub(crate) fn source_result(&mut self, source: &ExecutionSource, success: bool, result: i32) {
        source.callback.fire(
            success,
            result,
            self.cx,
            self.signalled,
            &mut self.context.admitted_batches,
        );
    }

    /// `Frame.returnSuccess` (Frame.java:6-8).
    pub(crate) fn return_success(&mut self, value: i32) {
        self.frame.result.fire(
            true,
            value,
            self.cx,
            self.signalled,
            &mut self.context.admitted_batches,
        );
    }

    /// `Frame.returnFailure` (Frame.java:10-12).
    pub(crate) fn return_failure(&mut self) {
        self.frame.result.fire(
            false,
            0,
            self.cx,
            self.signalled,
            &mut self.context.admitted_batches,
        );
    }

    /// `FunctionCommand.queueFunctions` (FunctionCommand.java:110-124) over the
    /// two shapes its `modifiers.isReturn()` chooses between.
    pub(crate) fn queue_functions(
        &mut self,
        functions: Vec<QueuedFunction>,
        function_source: &Arc<ExecutionSource>,
        original: &Arc<ExecutionSource>,
        is_return: bool,
        completed: bool,
    ) {
        // `clearCallbacks` (`:162`, `:183`): the caller's own callback answers
        // for the `/function` command, never for a line of the body.
        let body_source = Arc::new(ExecutionSource {
            callback: ResultCallback::Empty,
            ..(**function_source).clone()
        });
        let sender = Arc::new(original.source.clone());
        let silent = original.silent;
        let queue = |control: &mut Self, function: QueuedFunction, inner: ResultCallback| {
            let function_id = function.id;
            // `decorateOutputIfNeeded` (`:144-151`).
            let result = if silent {
                inner
            } else {
                ResultCallback::SignalFunctionResult {
                    function: function_id.clone(),
                    original: Arc::clone(&sender),
                    trace: original.trace.clone(),
                    inner: Box::new(inner),
                }
            };
            control.queue_next(EntryAction::CallFunction {
                function: function_id,
                body: function.body,
                source: Arc::clone(&body_source),
                result,
                return_parent_frame: is_return,
                trace_root: false,
            });
        };
        if is_return {
            // `queueFunctionsAsReturn` (`:152-171`) chains the frame's return
            // consumer onto the original source's own callback (`:163`).
            let chained =
                ResultCallback::chain(original.callback.clone(), self.frame.result.clone());
            for function in functions {
                queue(self, function, chained.clone());
            }
            if completed {
                // `queueFunctionsAsReturn` (`:171`).
                self.queue_next(EntryAction::Fallthrough);
            }
            return;
        }
        // `queueFunctionsNoReturn` (`:173-224`).
        if functions.is_empty() {
            return;
        }
        let original_callback = original.callback.clone();
        if functions.len() == 1 || original_callback == ResultCallback::Empty {
            // `:185-196`: each body reports straight to the caller's callback,
            // which the single-function arm reaches whatever that callback is.
            for function in functions {
                queue(self, function, original_callback.clone());
            }
            return;
        }
        // `:197-222`: several bodies and a caller that is listening, so the
        // results are summed once rather than reported one at a time.
        let accumulator = ResultAccumulator::new();
        for function in functions {
            queue(
                self,
                function,
                ResultCallback::Accumulate(accumulator.clone()),
            );
        }
        self.queue_next(EntryAction::FunctionResultSum {
            accumulator,
            callback: original_callback,
        });
    }

    /// `ExecutionControl.queueNext` (ExecutionControl.java:18-20).
    fn queue_next(&mut self, action: EntryAction) {
        let frame = self.frame.clone();
        self.context.queue_next(CommandQueueEntry { frame, action });
    }
}
