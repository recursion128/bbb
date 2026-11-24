//! DebugCommand trace output and its execution-context lifetime.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use bbb_protocol::{JavaString, NbtSummaryValue};

use super::{
    run, CommandFunctionDefinition, CommandQueueEntry, CommandRun, EntryAction, ExecutionContext,
    ExecutionSource, Frame, ResultCallback, ServerWorld,
};
use crate::commands::{
    component_literal_nbt, translatable, CommandFeedbackKind, CommandPermission, CommandSource,
    CommandSourceIdentity,
};

impl ExecutionContext {
    // DebugCommand.java:100-169; only the live drive runs this suspended entry.
    pub(super) fn start_trace(
        &mut self,
        frame: &Frame,
        functions: Vec<String>,
        source: Arc<ExecutionSource>,
        world: &ServerWorld,
        bodies: &dyn Fn(&str) -> Option<Arc<CommandFunctionDefinition>>,
    ) -> CommandRun {
        let filename = TraceHandle::filename();
        let mut count = 0i32;
        let mut run = CommandRun::default();
        match TraceHandle::open(world.debug_trace_directory(), &filename) {
            Ok(tracer) => {
                self.tracer = Some(tracer.clone());
                let cap =
                    |permission: CommandPermission| permission.min(CommandPermission::GameMaster);
                let body_source = Arc::new(ExecutionSource {
                    source: CommandSource {
                        permission: cap(source.source.permission),
                        ..source.source.clone()
                    },
                    executing: source.executing.as_ref().map(|executing| CommandSource {
                        permission: cap(executing.permission),
                        ..executing.clone()
                    }),
                    trace: Some(tracer),
                    chat_signer: None,
                    ..(*source).clone()
                });
                for id in &functions {
                    let function = match run::instantiate_function(id, None, world) {
                        Ok(function) => function,
                        Err(run::InstantiationError::Refused(message)) => {
                            run.feedback.push(run::failure_feedback(message));
                            continue;
                        }
                        Err(run::InstantiationError::CommandTooLong) => {
                            run.feedback.push(run::failure_feedback(translatable(
                                "command.failed",
                                Vec::new(),
                            )));
                            continue;
                        }
                    };
                    let Some(body) = function.body.or_else(|| bodies(id)) else {
                        continue;
                    };
                    count = count.wrapping_add(body.plain_entries().len() as i32);
                    self.queue_next(CommandQueueEntry {
                        frame: frame.clone(),
                        action: EntryAction::CallFunction {
                            function: id.clone(),
                            body: Some(body),
                            source: Arc::clone(&body_source),
                            result: ResultCallback::Empty,
                            return_parent_frame: false,
                            trace_root: true,
                        },
                    });
                }
            }
            Err(_) => run.feedback.push(run::failure_feedback(translatable(
                "commands.debug.function.traceFailed",
                Vec::new(),
            ))),
        }
        let message = if functions.len() == 1 {
            translatable(
                "commands.debug.function.success.single",
                vec![
                    NbtSummaryValue::Int(count),
                    component_literal_nbt(&functions[0]),
                    component_literal_nbt(filename),
                ],
            )
        } else {
            translatable(
                "commands.debug.function.success.multiple",
                vec![
                    NbtSummaryValue::Int(count),
                    NbtSummaryValue::Int(functions.len() as i32),
                    component_literal_nbt(filename),
                ],
            )
        };
        self.queue_next(CommandQueueEntry {
            frame: frame.clone(),
            action: EntryAction::TraceComplete {
                source: Arc::clone(&source),
                message,
            },
        });
        run
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TraceHandle(Arc<Mutex<TraceWriter>>);

impl PartialEq for TraceHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
struct TraceWriter {
    output: Option<BufWriter<File>>,
    last_indent: i32,
    waiting_for_result: bool,
    callback_messages: Vec<NbtSummaryValue>,
}

impl TraceHandle {
    // DebugCommand.java:120-129; Util.java:102, :149-151.
    pub(super) fn filename() -> String {
        format!(
            "debug-trace-{}.txt",
            chrono::Local::now().format("%Y-%m-%d_%H.%M.%S")
        )
    }

    pub(super) fn open(directory: &Path, filename: &str) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let output = BufWriter::new(File::create(directory.join(filename))?);
        Ok(Self(Arc::new(Mutex::new(TraceWriter {
            output: Some(output),
            last_indent: 0,
            waiting_for_result: false,
            callback_messages: Vec::new(),
        }))))
    }

    pub(super) fn root(&self, function: &str) {
        self.0
            .lock()
            .expect("trace writer")
            .write(&format!("{function}\n"));
    }

    // DebugCommand.java:200-231.
    pub(super) fn command(&self, depth: i32, command: &[u16]) {
        let mut writer = self.0.lock().expect("trace writer");
        writer.new_line();
        writer.indent_and_save(depth);
        writer.write(&format!("[C] {}", String::from_utf16_lossy(command)));
        writer.waiting_for_result = true;
    }

    pub(crate) fn returned(&self, depth: i32, command: &[u16], result: i32) {
        let mut writer = self.0.lock().expect("trace writer");
        if writer.waiting_for_result {
            writer.write(&format!(" -> {result}\n"));
            writer.waiting_for_result = false;
        } else {
            writer.indent_and_save(depth);
            writer.write(&format!(
                "[R = {result}] {}\n",
                String::from_utf16_lossy(command)
            ));
        }
    }

    pub(super) fn call(&self, depth: i32, function: &str, size: usize) {
        let mut writer = self.0.lock().expect("trace writer");
        writer.new_line();
        writer.indent_and_save(depth);
        writer.write(&format!("[F] {function} size={size}\n"));
    }

    pub(super) fn error(&self, message: &NbtSummaryValue) {
        self.0.lock().expect("trace writer").error(message);
    }

    pub(super) fn callback_message(&self, message: NbtSummaryValue) {
        self.0
            .lock()
            .expect("trace writer")
            .callback_messages
            .push(message);
    }

    pub(crate) fn record_run(&self, run: &mut CommandRun, source: Option<&ExecutionSource>) {
        let mut writer = self.0.lock().expect("trace writer");
        let mut errors = std::mem::take(&mut run.trace_errors).into_iter().peekable();
        let mut retained = Vec::new();
        for (index, line) in std::mem::take(&mut run.feedback).into_iter().enumerate() {
            while errors
                .peek()
                .is_some_and(|(position, _)| *position <= index)
            {
                writer.error(&errors.next().expect("pending error").1);
            }
            // CommandSourceStack.java:442-461 sends chat/system straight to an
            // attached player; success/failure use the tracer as CommandSource.
            let traced = source.is_some_and(|source| {
                source.trace.is_some()
                    && !source.silent
                    && (matches!(
                        line.kind,
                        CommandFeedbackKind::Success | CommandFeedbackKind::Failure
                    ) || !matches!(source.run_source().identity, CommandSourceIdentity::Player))
            });
            if traced {
                writer.message(line.message.component());
            } else {
                retained.push(line);
            }
        }
        for (_, message) in errors {
            writer.error(&message);
        }
        run.feedback = retained;
        for message in std::mem::take(&mut writer.callback_messages) {
            writer.message(&message);
        }
    }

    // ExecutionContext.java:141-145; DebugCommand.java:270-272.
    pub(super) fn close(&self) {
        let mut writer = self.0.lock().expect("trace writer");
        if let Some(mut output) = writer.output.take() {
            let _ = output.flush();
        }
    }
}

impl TraceWriter {
    fn write(&mut self, text: &str) {
        // PrintWriter records IO errors without throwing from trace callbacks.
        if let Some(output) = &mut self.output {
            let _ = output.write_all(text.as_bytes());
        }
    }

    fn indent(&mut self, depth: i32) {
        for _ in 0..depth + 1 {
            self.write("    ");
        }
    }

    fn indent_and_save(&mut self, depth: i32) {
        self.indent(depth);
        self.last_indent = depth;
    }

    fn new_line(&mut self) {
        if self.waiting_for_result {
            self.write("\n");
            self.waiting_for_result = false;
        }
    }

    // DebugCommand.java:234-247 intentionally leaves the error without a newline.
    fn error(&mut self, message: &NbtSummaryValue) {
        self.new_line();
        self.indent_and_save(self.last_indent + 1);
        self.write(&format!("[E] {}", plain_text(message)));
    }

    fn message(&mut self, message: &NbtSummaryValue) {
        self.new_line();
        self.indent(self.last_indent + 1);
        self.write(&format!("[M] {}\n", plain_text(message)));
    }
}

fn plain_text(component: &NbtSummaryValue) -> String {
    JavaString::from(bbb_server_datapack::language::component_plain_text(
        component,
    ))
    .to_jdk_string()
}
