//! `ServerFunctionManager` (ServerFunctionManager.java:22-107) and the world's
//! half of the execution queue.
//!
//! Three vanilla pieces meet here and none of them belongs to the dispatcher.
//! The manager itself decides *when* a datapack function runs: the
//! `minecraft:load` tag once after every reload and the `minecraft:tick` tag
//! every tick, both from `tickChildren`'s first arm
//! (MinecraftServer.java:1086-1088). `crate::commands::ExecutionContext` decides
//! *how much* runs. And [`crate::commands`] decides what each line means,
//! because a function body is nothing but commands and this module must not
//! become a second reading of them.
//!
//! The drive lives here because every entry runs against `&mut ServerWorld`:
//! each body command reads the level as the command before it left it.

use std::sync::Arc;

use bbb_protocol::packets::SystemChat;

use crate::commands::{
    self, CommandCx, CommandFeedback, CommandFunctionBody, CommandFunctionDefinition,
    CommandFunctionEntry, CommandFunctionTemplate, CommandOrigin, CommandPermission, CommandRun,
    CommandSource, CommandSourceIdentity, EntryOutput, ExecutionContext, ExecutionSource,
    ResultCallback,
};
use bbb_server_datapack::function::FunctionLibrary;
use bbb_server_datapack::tag_loader::build_registry_tags;

use super::{command_feedback_channel, CommandFeedbackChannel, ServerWorld, WorldEvent};

/// `ServerFunctionManager.TICK_FUNCTION_TAG` (ServerFunctionManager.java:24).
const TICK_FUNCTION_TAG: &str = "minecraft:tick";

/// `ServerFunctionManager.LOAD_FUNCTION_TAG` (ServerFunctionManager.java:25).
const LOAD_FUNCTION_TAG: &str = "minecraft:load";

/// `ServerFunctionLibrary` as the manager reads it, with every function already
/// compiled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CompiledFunctionLibrary {
    /// Shared so a draining queue keeps the generation it opened on.
    functions: Arc<rustc_hash::FxHashMap<String, Arc<CommandFunctionDefinition>>>,
    /// `ServerFunctionManager.getTagNames()` (`:104-106`) beside the member
    /// list `getTag` (`:96-98`) answers with.
    tags: rustc_hash::FxHashMap<String, Arc<[String]>>,
}

impl CompiledFunctionLibrary {
    /// `ServerFunctionManager.getFunctionNames()` (`:100-102`).
    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        self.functions.keys().map(String::as_str)
    }

    pub(crate) fn tag_names(&self) -> impl Iterator<Item = &str> {
        self.tags.keys().map(String::as_str)
    }

    /// `ServerFunctionLibrary.getFunction` (ServerFunctionLibrary.java:47-49).
    pub(crate) fn definition(&self, id: &str) -> Option<&CommandFunctionDefinition> {
        self.functions.get(id).map(Arc::as_ref)
    }

    /// `ServerFunctionLibrary.getTag`. The library answers `List.of()` for a
    /// tag it does not hold (ServerFunctionLibrary.java:55-57), so `None` is
    /// `FunctionArgument.getFunctionTag`'s null check (FunctionArgument.java:82).
    pub(crate) fn tag(&self, id: &str) -> Option<&[String]> {
        self.tags.get(id).map(Arc::as_ref)
    }

    fn tag_members_arc(&self, id: &str) -> Arc<[String]> {
        self.tags
            .get(id)
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::from([]))
    }
}

#[derive(Clone, Debug)]
struct EffectFunctionCall {
    id: String,
    sender: ExecutionSource,
}

type FunctionLineParseCache =
    rustc_hash::FxHashMap<(Arc<[u16]>, u8), Arc<dyn std::any::Any + Send + Sync>>;

/// `ServerFunctionManager` (ServerFunctionManager.java:22-107).
#[derive(Clone, Debug, Default)]
pub(super) struct ServerFunctionManager {
    library: CompiledFunctionLibrary,
    effect_calls: Option<Vec<EffectFunctionCall>>,
    /// `ServerFunctionManager.ticking` (`:27`), the `minecraft:tick` tag copied
    /// at reload (`:84`) rather than looked up every tick.
    ticking: Arc<[String]>,
    /// `ServerFunctionManager.postReload` (`:28`).
    post_reload: bool,
    /// Per-generation cache of parsed `.mcfunction` lines keyed by `(line, permission)`.
    parse_cache: FunctionLineParseCache,
}

/// What a caller that answers for the whole command collects from the queue
/// instead of delivering each line to the source.
#[derive(Default)]
pub(super) struct CommandContinuationReport {
    pub(super) run: CommandRun,
    pub(super) command_block: Option<(CommandSource, bool)>,
    pub(super) last_output: Option<bbb_protocol::RichTextComponent>,
}

impl ServerFunctionManager {
    /// `ServerFunctionManager.postReload` (ServerFunctionManager.java:83-86).
    fn post_reload(&mut self, library: CompiledFunctionLibrary) {
        self.ticking = library.tag_members_arc(TICK_FUNCTION_TAG);
        self.library = library;
        self.parse_cache.clear();
        self.post_reload = true;
    }
}

/// `CommandFunction.fromLines`'s dispatcher half (CommandFunction.java:29-87)
/// over a library the line surface already produced.
///
/// `datapack::function::load_function_library` is `ServerFunctionLibrary.reload`
/// minus this pass: it reads the files, folds the continuations, drops the
/// comments and resolves the tags, and stops where the dispatcher would have
/// been needed. This is that dispatcher, run once per library rather than once
/// per call, and it drops a function exactly where vanilla's loader does — a
/// line that will not parse throws out of `fromLines` (`:80`) and
/// `ServerFunctionLibrary` logs the failed future and leaves the id out of the
/// map (ServerFunctionLibrary.java:104-113).
///
/// Because this server publishes a third of vanilla's commands, that rule bites
/// far harder here than it does on the official server: a datapack function
/// naming any command outside `commands::catalog::IMPLEMENTED_COMMANDS` is not
/// in the library at all, exactly as if the line were a typo.
pub(super) fn compile_library(
    library: &FunctionLibrary,
    world: &ServerWorld,
) -> CompiledFunctionLibrary {
    let mut functions = rustc_hash::FxHashMap::default();
    let mut compiled_ids_owned = Vec::new();
    for element in &library.functions {
        let mut body = if element.source.has_macros() {
            CommandFunctionBody::Macro {
                entries: Vec::new(),
                parameters: Vec::new(),
            }
        } else {
            CommandFunctionBody::Plain(Vec::new())
        };
        let mut compiled = true;
        for line in &element.source.lines {
            if line.is_macro() {
                // `FunctionBuilder.addMacro` (FunctionBuilder.java:45-64) does
                // not parse the command: `StringTemplate.fromString` splits it
                // and the command parse waits for substitution
                // (`MacroFunction.MacroEntry.instantiate`, MacroFunction.java:136-146).
                let CommandFunctionBody::Macro {
                    entries,
                    parameters,
                } = &mut body
                else {
                    unreachable!("a dollar-prefixed source selects a macro body")
                };
                let Some(template) = parse_function_template(Arc::clone(&line.line), parameters)
                else {
                    compiled = false;
                    break;
                };
                entries.push(CommandFunctionEntry::Macro(template));
                continue;
            }
            if !commands::function_line_compiles(&line.line, world) {
                compiled = false;
                break;
            }
            match &mut body {
                CommandFunctionBody::Plain(entries) => entries.push(Arc::clone(&line.line)),
                CommandFunctionBody::Macro { entries, .. } => {
                    entries.push(CommandFunctionEntry::Plain(Arc::clone(&line.line)))
                }
            }
        }
        if !compiled {
            continue;
        }
        compiled_ids_owned.push(element.id.as_str());
        functions.insert(
            element.id.clone(),
            Arc::new(CommandFunctionDefinition {
                id: element.id.clone(),
                body,
            }),
        );
    }
    // `TagLoader.build` runs after the function map has been replaced
    // (ServerFunctionLibrary.java:113-114), so it resolves against the functions
    // that compiled and spends the `required` flag here: an optional entry
    // naming a function this pass dropped is skipped, a required one drops its
    // whole tag (`tryBuildTag`, TagLoader.java:76-87), and a `#` reference reads
    // the tags this same pass rebuilt. Vanilla's report of a dropped tag is a
    // `LOGGER.error` (`:107-111`) and this crate has no sink for one.
    let tags: rustc_hash::FxHashMap<String, Arc<[String]>> = library
        .tags
        .as_ref()
        .and_then(|entries| build_registry_tags(entries, &compiled_ids_owned, &mut Vec::new()))
        .map(|built| {
            built
                .tags
                .into_iter()
                .map(|(tag, members)| (tag, Arc::from(members)))
                .collect()
        })
        .unwrap_or_default();
    CompiledFunctionLibrary {
        functions: Arc::new(functions),
        tags,
    }
}

/// `StringTemplate.fromString` (`StringTemplate.java:7-45`).
fn parse_function_template(
    source: Arc<[u16]>,
    parameters: &mut Vec<bbb_protocol::JavaString>,
) -> Option<CommandFunctionTemplate> {
    let mut segments = Vec::new();
    let mut parameter_slots = Vec::new();
    let mut start = 1;
    let mut search = start;
    while let Some(relative) = source[search..]
        .iter()
        .position(|unit| *unit == u16::from(b'$'))
    {
        let index = search + relative;
        if source.get(index + 1) != Some(&u16::from(b'(')) {
            search = index + 1;
            continue;
        }
        segments.push(start..index);
        let variable_start = index + 2;
        let variable_end = variable_start
            + source[variable_start..]
                .iter()
                .position(|unit| *unit == u16::from(b')'))?;
        let variable = &source[variable_start..variable_end];
        if !variable.iter().all(|unit| {
            *unit == u16::from(b'_') || bbb_protocol::java_char_is_letter_or_digit(*unit)
        }) {
            return None;
        }
        let parameter = parameters
            .iter()
            .position(|held| held.units() == variable)
            .unwrap_or_else(|| {
                parameters.push(variable.into());
                parameters.len() - 1
            });
        parameter_slots.push(parameter);
        start = variable_end + 1;
        search = start;
    }
    if parameter_slots.is_empty() {
        return None;
    }
    if start != source.len() {
        segments.push(start..source.len());
    }
    Some(CommandFunctionTemplate {
        source,
        segments,
        parameter_slots,
    })
}

impl ServerWorld {
    /// `MinecraftServer.getFunctions()` as a command reads it.
    pub(crate) fn command_functions(&self) -> &CompiledFunctionLibrary {
        &self.cold.functions.library
    }

    /// Look up or populate the per-datapack-generation parsed-line cache for
    /// `.mcfunction` execution.
    pub(crate) fn cached_function_parse<T: Send + Sync + 'static>(
        &mut self,
        line: &Arc<[u16]>,
        permission: CommandPermission,
        build: impl FnOnce(&ServerWorld) -> T,
    ) -> Arc<T> {
        const MAX_CACHED_FUNCTION_LINES: usize = 4096;
        let perm = match permission {
            CommandPermission::All => 0,
            CommandPermission::Moderator => 1,
            CommandPermission::GameMaster => 2,
            CommandPermission::Admin => 3,
            CommandPermission::Owner => 4,
        };
        let key = (Arc::clone(line), perm);
        if let Some(existing) = self.cold.functions.parse_cache.get(&key) {
            if let Ok(typed) = Arc::clone(existing).downcast::<T>() {
                return typed;
            }
        }
        let parsed = Arc::new(build(self));
        if self.cold.functions.parse_cache.len() >= MAX_CACHED_FUNCTION_LINES {
            self.cold.functions.parse_cache.clear();
        }
        self.cold.functions.parse_cache.insert(
            key,
            Arc::clone(&parsed) as Arc<dyn std::any::Any + Send + Sync>,
        );
        parsed
    }

    /// `MinecraftServer.reloadResources`'s
    /// `this.functions.replaceLibrary(..)` (ServerFunctionManager.java:78-81),
    /// which is also how the manager is first filled: the constructor
    /// (`:31-35`) calls the same `postReload`, so the `minecraft:load` tag runs
    /// once at startup and once after every reload.
    pub(crate) fn replace_function_library(&mut self, library: &FunctionLibrary) {
        let compiled = compile_library(library, self);
        self.cold.functions.post_reload(compiled);
    }

    /// `TimerQueue.tick` (TimerQueue.java:51-63) from `ServerLevel.tickTime`
    /// (`ServerLevel.java:465-466`): each due callback is
    /// `ServerFunctionManager.execute` on `getGameLoopSender`
    /// (FunctionCallback.java:16-20, FunctionTagCallback.java:16-21).
    pub(super) fn tick_scheduled_events(&mut self, events: &mut Vec<WorldEvent>) {
        while let Some(callback) = self.cold.scheduled_events.pop_due(self.game_time) {
            match callback {
                commands::CommandScheduledCallback::Function(id) => {
                    let sender = self.game_loop_sender();
                    self.execute_function(&id, &sender, events);
                }
                commands::CommandScheduledCallback::FunctionTag(id) => {
                    let members = self.cold.functions.library.tag_members_arc(&id);
                    if !members.is_empty() {
                        self.execute_tag_functions(&members, events);
                    }
                }
            }
        }
    }

    /// `ServerFunctionManager.tick` (ServerFunctionManager.java:41-51) from
    /// `tickChildren`'s `commandFunctions` arm (MinecraftServer.java:1086-1088),
    /// which is the first thing that arm does and therefore runs before the
    /// clocks, the time synchronisation and every level.
    pub(super) fn tick_command_functions(&mut self, events: &mut Vec<WorldEvent>) {
        if !self.tick_rate.runs_normally() {
            return;
        }
        if std::mem::take(&mut self.cold.functions.post_reload) {
            let load = self
                .cold
                .functions
                .library
                .tag_members_arc(LOAD_FUNCTION_TAG);
            if !load.is_empty() {
                self.execute_tag_functions(&load, events);
            }
        }
        if !self.cold.functions.ticking.is_empty() {
            let ticking = Arc::clone(&self.cold.functions.ticking);
            self.execute_tag_functions(&ticking, events);
        }
    }

    /// `ServerFunctionManager.executeTagFunctions` (ServerFunctionManager.java:53-61).
    ///
    /// Each member goes through `execute` on its own, and `execute` opens its
    /// own `ExecutionContext` (`:69`), so the quota is per member: one runaway
    /// function in the tag does not eat the budget of the ones after it.
    fn execute_tag_functions(&mut self, functions: &[String], events: &mut Vec<WorldEvent>) {
        let sender = self.game_loop_sender();
        for id in functions {
            self.execute_function(id, &sender, events);
        }
    }

    /// `ServerFunctionManager.getGameLoopSender` (ServerFunctionManager.java:88-90):
    /// `createCommandSourceStack` (MinecraftServer.java:1677-1690) narrowed to
    /// `GAMEMASTER` and silenced.
    ///
    /// `withPermission` replaces the set rather than widening it
    /// (CommandSourceStack.java:261-280), so the game loop runs *below* the
    /// console's own `OWNER`: a `/stop` or an `/op` in a `minecraft:tick`
    /// function does not parse.
    fn game_loop_sender(&self) -> ExecutionSource {
        ExecutionSource {
            // `Component.literal("Server")` (MinecraftServer.java:1685-1686);
            // the console holds no profile.
            source: CommandSource::server("Server", CommandPermission::GameMaster),
            origin: self.live_console_origin(),
            silent: true,
            trace: None,
            executing: None,
            callback: ResultCallback::Empty,
            chat_signer: None,
        }
    }

    /// `ServerFunctionManager.execute` (ServerFunctionManager.java:63-76): one
    /// top-level call, with its own `ExecutionContext` and therefore its own
    /// quota.
    ///
    /// The `FunctionInstantiationException` arm (`:70`) is the empty one in
    /// vanilla too: a macro function called with no arguments is dropped
    /// without a word, which the queue answers by finding no body for the id.
    fn execute_function(
        &mut self,
        id: &str,
        sender: &ExecutionSource,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut context = ExecutionContext::open(self);
        context.queue_initial_function_call(id, sender.clone());
        self.drain_execution(&mut context, events);
    }

    /// `Commands.performCommand` (Commands.java:319-356) run to the end of its
    /// queue. With a `report` the lines the source was told are collected
    /// there; without one they are delivered as each entry finishes.
    pub(super) fn perform_command(
        &mut self,
        command: Arc<[u16]>,
        source: &CommandSource,
        origin: &CommandOrigin,
        callback: ResultCallback,
        report: Option<&mut CommandContinuationReport>,
        events: &mut Vec<WorldEvent>,
    ) {
        self.perform_signed_command(command, source, origin, callback, report, None, events);
    }

    /// [`Self::perform_command`] under a player's signing context.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn perform_signed_command(
        &mut self,
        command: Arc<[u16]>,
        source: &CommandSource,
        origin: &CommandOrigin,
        callback: ResultCallback,
        report: Option<&mut CommandContinuationReport>,
        chat_signer: Option<uuid::Uuid>,
        events: &mut Vec<WorldEvent>,
    ) {
        let prepared =
            commands::prepare_command(command, source, self, origin, callback, chat_signer);
        match prepared {
            Ok(mut context) => self.drain_execution_with_report(&mut context, events, report),
            Err(run) => {
                let source = Arc::new(ExecutionSource {
                    source: source.clone(),
                    origin: origin.clone(),
                    silent: false,
                    trace: None,
                    executing: None,
                    callback: ResultCallback::Empty,
                    chat_signer: None,
                });
                self.apply_entry_output(
                    EntryOutput {
                        source: Some(source),
                        run,
                        signalled: Vec::new(),
                        tracer: None,
                        trace_return: None,
                    },
                    events,
                    report,
                );
            }
        }
    }

    fn queue_effect_function(&mut self, context: &mut ExecutionContext, call: EffectFunctionCall) {
        context.queue_initial_function_call(&call.id, call.sender);
    }

    /// `ExecutionContext.runCommandQueue` (ExecutionContext.java:85-110) over a
    /// projection rebuilt for every entry.
    fn drain_execution(&mut self, context: &mut ExecutionContext, events: &mut Vec<WorldEvent>) {
        self.drain_execution_with_report(context, events, None);
    }

    fn drain_execution_with_report(
        &mut self,
        context: &mut ExecutionContext,
        events: &mut Vec<WorldEvent>,
        mut report: Option<&mut CommandContinuationReport>,
    ) {
        // `ServerFunctionLibrary.getFunction` as the generation this drive
        // opened on: nothing reloads the library while a queue is draining.
        //
        // Tick, load, schedule and execute-condition calls pass a null argument
        // compound, so only plain definitions have a body on this lookup.
        let library = Arc::clone(&self.cold.functions.library.functions);
        let bodies = move |id: &str| {
            library
                .get(id)
                .filter(|function| !function.is_macro())
                .cloned()
        };
        let outer_calls = self.cold.functions.effect_calls.replace(Vec::new());
        commands::run_command_queue(context, |context, entry| {
            let origin = self.live_console_origin();
            let mut cx = CommandCx {
                world: &mut *self,
                events: &mut *events,
                origin,
                chat_signer: None,
            };
            let output = commands::execute_entry(context, entry, &mut cx, &bodies);
            self.apply_entry_output(output, events, report.as_deref_mut());
            // Command-triggered gameplay functions share the active execution quota.
            for call in self
                .cold
                .functions
                .effect_calls
                .as_mut()
                .map(std::mem::take)
                .unwrap_or_default()
            {
                self.queue_effect_function(context, call);
            }
        });
        self.cold.functions.effect_calls = outer_calls;
    }

    pub(super) fn execute_enchantment_function(
        &mut self,
        id: &str,
        executing: CommandSource,
        origin: CommandOrigin,
        events: &mut Vec<WorldEvent>,
    ) {
        self.execute_effect_function(
            id,
            ExecutionSource {
                source: CommandSource::server("Server", CommandPermission::GameMaster),
                origin,
                silent: true,
                trace: None,
                executing: Some(executing),
                callback: ResultCallback::Empty,
                chat_signer: None,
            },
            events,
        );
    }

    pub(super) fn execute_effect_function(
        &mut self,
        id: &str,
        sender: ExecutionSource,
        events: &mut Vec<WorldEvent>,
    ) {
        // Missing functions and macros without arguments have no body to execute.
        if !self
            .cold
            .functions
            .library
            .functions
            .get(id)
            .is_some_and(|function| !function.is_macro())
        {
            return;
        }
        if let Some(calls) = &mut self.cold.functions.effect_calls {
            calls.push(EffectFunctionCall {
                id: id.to_string(),
                sender,
            });
        } else {
            self.execute_function(id, &sender, events);
        }
    }

    /// Applies what one queue entry produced.
    fn apply_entry_output(
        &mut self,
        mut output: EntryOutput,
        events: &mut Vec<WorldEvent>,
        mut report: Option<&mut CommandContinuationReport>,
    ) {
        if let Some(tracer) = &output.tracer {
            tracer.record_run(&mut output.run, output.source.as_deref());
        }
        let trace_result = output.run.result;
        let EntryOutput {
            source,
            run,
            signalled,
            tracer,
            trace_return,
        } = output;
        // The feedback of a silenced source is dropped rather than sent
        // (CommandSourceStack.java:465, `:496`), so a refusal inside a function
        // body is invisible; it does not stop the body either, because
        // `handleError` (`:571-580`) sends and returns without discarding the
        // frame.
        let collect = source
            .as_ref()
            .is_some_and(|source| !source.silent && source.trace.is_none())
            && report.is_some();
        if collect {
            let report = report
                .as_deref_mut()
                .expect("the reporting context is present");
            if run.result.is_some() {
                report.run.result = run.result;
            }
            report.run.successful_contexts += run.successful_contexts;
            report.run.feedback.extend(run.feedback.iter().cloned());
            report.run.broadcast.extend(run.broadcast.iter().cloned());
            if let Some((cb_source, track_output)) = &report.command_block {
                let speaker = source
                    .as_ref()
                    .map_or(cb_source, |source| source.run_source())
                    .clone();
                let output = self.apply_command_block_run(speaker, run, *track_output, events);
                if output.is_some() {
                    report.last_output = output;
                }
            } else {
                self.apply_command_run(run, None, events);
            }
        } else {
            let feedback = source
                .as_ref()
                .filter(|source| !source.silent)
                .map(|source| {
                    let recipient = if source.trace.is_some() {
                        source.run_source().clone()
                    } else {
                        source.source.clone()
                    };
                    (recipient, source.run_source().clone())
                });
            self.apply_command_run(
                run,
                feedback
                    .as_ref()
                    .map(|(recipient, speaker)| (recipient, speaker)),
                events,
            );
        }
        for (source, line) in signalled {
            if let Some(report) = report
                .as_deref_mut()
                .filter(|report| report.command_block.is_some())
            {
                let (command_source, track_output) = report
                    .command_block
                    .as_ref()
                    .expect("report target checked");
                let output = self.apply_command_block_run(
                    command_source.clone(),
                    CommandRun {
                        feedback: vec![line],
                        ..Default::default()
                    },
                    *track_output,
                    events,
                );
                if output.is_some() {
                    report.last_output = output;
                }
            } else {
                self.deliver_command_feedback(&source, &source, line, events);
            }
        }
        if let (Some(tracer), Some((depth, command)), Some(result)) =
            (tracer, trace_return, trace_result)
        {
            tracer.returned(depth, &command, result);
        }
    }

    /// One entry's lines, then its broadcasts, then the effects vanilla
    /// performs after reporting them.
    fn apply_command_run(
        &mut self,
        run: CommandRun,
        feedback_source: Option<(&CommandSource, &CommandSource)>,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some((recipient, speaker)) = feedback_source {
            for line in run.feedback {
                self.deliver_command_feedback(recipient, speaker, line, events);
            }
        }
        // `/say` broadcasts through `PlayerList.broadcastChatMessage`
        // (SayCommand.java:18-19) whoever asked for it, so a silenced source
        // still reaches everyone.
        for message in run.broadcast {
            self.broadcast_command_message(message, events);
        }
    }

    /// `CommandSourceStack.sendSuccess` (CommandSourceStack.java:464-477) and
    /// `sendFailure` (`:495-499`) for one line the queue produced.
    fn deliver_command_feedback(
        &mut self,
        recipient: &CommandSource,
        speaker: &CommandSource,
        feedback: CommandFeedback,
        events: &mut Vec<WorldEvent>,
    ) {
        let accepts_success = self.accepts_command_success();
        // `:465`: only `sendSuccess` reads `acceptsSuccess`.
        let send_line = accepts_success
            || command_feedback_channel(&feedback) != CommandFeedbackChannel::Success;
        let is_server = matches!(recipient.identity, CommandSourceIdentity::Server);
        // `sendSystemMessage` (`:453-462`): a player is told through their own
        // session, and anybody else through `this.source`, which for the
        // console is `MinecraftServer.sendSystemMessage`. A command block's
        // `BaseCommandBlock` has no channel here and is told nothing.
        let player = self
            .command_source_connection(recipient)
            .and_then(|target| self.levels.player(target));
        let connection = player.map(|player| player.connection_id);
        let visibility = player.map(|player| player.cold.client_information.chat_visibility);
        if send_line {
            if let Some(connection) = connection {
                self.send_command_feedback_message(
                    connection,
                    &feedback,
                    visibility.expect("resolved original player"),
                    events,
                );
            } else if is_server {
                events.push(WorldEvent::ConsoleMessage(feedback.message.clone()));
            }
        }
        if !feedback.inform_admins {
            return;
        }
        // `broadcastToAdmins` (`:479-493`) attributes the message to
        // `this.displayName` (`CommandSourceStack.java:151-169,480`), which
        // follows `execute as`.
        let copy = commands::admin_broadcast_copy(
            speaker,
            &feedback.message,
            self.scoreboard(),
            self.plugins().content_registry(),
        );
        if accepts_success {
            let operators = self
                .levels
                .player_entries()
                .filter(|(other, state)| {
                    Some(**other) != connection
                        && self
                            .admin
                            .lists()
                            .operators
                            .contains_key(&state.profile.uuid)
                })
                .map(|(other, _)| *other)
                .collect::<Vec<_>>();
            for other in operators {
                self.send_system_message(
                    other,
                    SystemChat::from_component(copy.clone(), false),
                    events,
                );
            }
        }
        // `:490-492`: `this.source != this.server` is what keeps the console
        // from being told twice about its own line.
        if !is_server && self.logs_admin_commands() {
            events.push(WorldEvent::ConsoleMessage(copy));
        }
    }
}

#[cfg(test)]
mod tests;
