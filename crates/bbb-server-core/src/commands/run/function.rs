//! `/function`.

use std::sync::Arc;

use bbb_protocol::{
    canonical_snbt_java_string, component_literal_nbt, JavaString, NbtSummaryValue,
};

use crate::commands::execution::{ExecutionControl, ExecutionSource};
use crate::commands::parse::{self, ArgumentValue, ParseResult};
use crate::commands::types::{
    CommandFunctionBody, CommandFunctionDefinition, CommandFunctionEntry, CommandFunctionTemplate,
};
use crate::commands::{
    format_list, translatable, CommandPermission, CommandSource, QueuedFunction,
};
use crate::world::ServerWorld;

use super::{function_arguments, CommandRun};

/// `CommandFunction.checkCommandLineLength` (CommandFunction.java:89-93).
const MAX_FUNCTION_COMMAND_UTF16: usize = 2_000_000;

/// `FunctionCommand.FunctionCustomExecutor.runGuarded`
/// (FunctionCommand.java:235-265).
///
/// The order is the method's: resolve the argument, refuse an empty tag, report
/// what is about to run, and only then queue it. That order is what a caller
/// sees when the queue then refuses the call — the `commands.function.scheduled`
/// line has already left, because `sendSuccess` (`:251-262`) runs before
/// `queueFunctions` (`:264`).
pub(super) fn call_function(
    run: &mut CommandRun,
    path: &[&str],
    parse: &ParseResult,
    source: &Arc<ExecutionSource>,
    is_return: bool,
    control: &mut ExecutionControl<'_, '_>,
) -> Result<(), NbtSummaryValue> {
    let (tag, name) = match parse.argument("name") {
        Some(ArgumentValue::Function { tag, id }) => (*tag, id.as_str()),
        other => unreachable!("the function argument parsed as {other:?}"),
    };
    let functions = resolve_functions(tag, name, control.world())?;
    // `:244-246`. Only the tag branch can reach it: the plain-name branch
    // already threw for an id the library does not hold.
    if functions.is_empty() {
        return Err(translatable(
            "commands.function.scheduled.no_functions",
            vec![component_literal_nbt(name)],
        ));
    }
    // `FunctionCustomExecutor.runGuarded` resolves the function collection
    // before asking its executor for arguments (FunctionCommand.java:241-249).
    let argument_tag = match parse.argument("arguments") {
        Some(ArgumentValue::CompoundTag(value)) => Some(value.clone()),
        None => function_arguments(
            path,
            parse,
            source.run_source(),
            control.world_mut(),
            &source.origin,
        )?,
        other => unreachable!("the arguments argument parsed as {other:?}"),
    };
    if let Some(value) = &argument_tag {
        if !matches!(value, NbtSummaryValue::Compound(_)) {
            return Err(translatable(
                "commands.function.error.argument_not_compound",
                vec![NbtSummaryValue::string(nbt_type_name(value))],
            ));
        }
    }
    if functions.len() == 1 {
        run.succeed(
            translatable(
                "commands.function.scheduled.single",
                vec![component_literal_nbt(&functions[0])],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.function.scheduled.multiple",
                vec![format_list(
                    functions.iter().map(component_literal_nbt).collect(),
                )],
            ),
            true,
        );
    }
    let mut queued = Vec::with_capacity(functions.len());
    for id in functions {
        let function = match instantiate_function(&id, argument_tag.as_ref(), control.world()) {
            Ok(function) => function,
            Err(InstantiationError::Refused(reason)) => {
                control.queue_functions(
                    queued,
                    &Arc::new(modify_sender_for_execution(source)),
                    source,
                    is_return,
                    false,
                );
                return Err(translatable(
                    "commands.function.instantiationFailure",
                    vec![component_literal_nbt(&id), reason],
                ));
            }
            Err(InstantiationError::CommandTooLong) => {
                control.queue_functions(
                    queued,
                    &Arc::new(modify_sender_for_execution(source)),
                    source,
                    is_return,
                    false,
                );
                return Err(translatable("command.failed", Vec::new()));
            }
        };
        queued.push(function);
    }
    control.queue_functions(
        queued,
        &Arc::new(modify_sender_for_execution(source)),
        source,
        is_return,
        true,
    );
    Ok(())
}

pub(in crate::commands) enum InstantiationError {
    Refused(NbtSummaryValue),
    CommandTooLong,
}

pub(in crate::commands) fn instantiate_function(
    id: &str,
    arguments: Option<&NbtSummaryValue>,
    world: &ServerWorld,
) -> Result<QueuedFunction, InstantiationError> {
    let Some(definition) = world.command_functions().definition(id) else {
        return Ok(QueuedFunction {
            id: id.to_string(),
            body: None,
        });
    };
    let CommandFunctionBody::Macro {
        entries: templates,
        parameters,
    } = &definition.body
    else {
        return Ok(QueuedFunction {
            id: id.to_string(),
            body: None,
        });
    };
    let Some(NbtSummaryValue::Compound(arguments)) = arguments else {
        return Err(InstantiationError::Refused(missing_arguments(id)));
    };
    let mut values = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let Some(value) = arguments
            .iter()
            .find(|entry| entry.name == *parameter)
            .map(|entry| &entry.value)
        else {
            return Err(InstantiationError::Refused(translatable(
                "commands.function.error.missing_argument",
                vec![
                    component_literal_nbt(id),
                    NbtSummaryValue::string(parameter.clone()),
                ],
            )));
        };
        values.push(macro_stringify(value));
    }
    let mut entries = Vec::with_capacity(templates.len());
    for entry in templates {
        let line = match entry {
            CommandFunctionEntry::Plain(line) => Arc::clone(line),
            CommandFunctionEntry::Macro(template) => {
                let line = substitute(template, &values)?;
                let parsed =
                    parse::parse(line.units(), world.function_compilation_permission(), world);
                let failure = parsed
                    .failure(line.units())
                    .map(|failure| failure.message)
                    .or_else(|| {
                        parsed
                            .executable_node()
                            .is_none()
                            .then(|| translatable("command.unknown.command", Vec::new()))
                    });
                if let Some(reason) = failure {
                    return Err(InstantiationError::Refused(translatable(
                        "commands.function.error.parse",
                        vec![
                            component_literal_nbt(&definition.id),
                            NbtSummaryValue::string(line),
                            reason,
                        ],
                    )));
                }
                Arc::from(line.units())
            }
        };
        entries.push(line);
    }
    Ok(QueuedFunction {
        id: id.to_string(),
        body: Some(Arc::new(CommandFunctionDefinition {
            id: id.to_string(),
            body: CommandFunctionBody::Plain(entries),
        })),
    })
}

fn substitute(
    template: &CommandFunctionTemplate,
    values: &[JavaString],
) -> Result<JavaString, InstantiationError> {
    let mut result = Vec::new();
    for (index, parameter) in template.parameter_slots.iter().enumerate() {
        let segment = &template.source[template.segments[index].clone()];
        let value = values[*parameter].units();
        let remaining = MAX_FUNCTION_COMMAND_UTF16 - result.len();
        if segment.len() > remaining || value.len() > remaining - segment.len() {
            return Err(InstantiationError::CommandTooLong);
        }
        result.extend_from_slice(segment);
        result.extend_from_slice(value);
    }
    if template.segments.len() > template.parameter_slots.len() {
        let segment = &template.source[template
            .segments
            .last()
            .expect("a trailing segment exists")
            .clone()];
        if segment.len() > MAX_FUNCTION_COMMAND_UTF16 - result.len() {
            return Err(InstantiationError::CommandTooLong);
        }
        result.extend_from_slice(segment);
    }
    Ok(result.into())
}

fn missing_arguments(id: &str) -> NbtSummaryValue {
    translatable(
        "commands.function.error.missing_arguments",
        vec![component_literal_nbt(id)],
    )
}

fn macro_stringify(value: &NbtSummaryValue) -> JavaString {
    match value {
        NbtSummaryValue::Byte(value) => value.to_string().into(),
        NbtSummaryValue::Short(value) => value.to_string().into(),
        NbtSummaryValue::Int(value) => value.to_string().into(),
        NbtSummaryValue::Long(value) => value.to_string().into(),
        NbtSummaryValue::Float(bits) => decimal_format(f64::from(f32::from_bits(*bits))).into(),
        NbtSummaryValue::Double(bits) => decimal_format(f64::from_bits(*bits)).into(),
        NbtSummaryValue::String(value) => value.clone(),
        value => canonical_snbt_java_string(value),
    }
}

fn decimal_format(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value == f64::INFINITY {
        return "∞".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-∞".to_string();
    }
    let mut value = format!("{value:.15}");
    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }
    value
}

fn nbt_type_name(value: &NbtSummaryValue) -> &'static str {
    match value {
        NbtSummaryValue::Byte(_) => "BYTE",
        NbtSummaryValue::Short(_) => "SHORT",
        NbtSummaryValue::Int(_) => "INT",
        NbtSummaryValue::Long(_) => "LONG",
        NbtSummaryValue::Float(_) => "FLOAT",
        NbtSummaryValue::Double(_) => "DOUBLE",
        NbtSummaryValue::ByteArray(_) => "BYTE[]",
        NbtSummaryValue::String(_) => "STRING",
        NbtSummaryValue::List(_) => "LIST",
        NbtSummaryValue::Compound(_) => "COMPOUND",
        NbtSummaryValue::IntArray(_) => "INT[]",
        NbtSummaryValue::LongArray(_) => "LONG[]",
    }
}

/// `FunctionArgument.Result.unwrapToCollection` (FunctionArgument.java:49-51
/// for the tag branch, `:69-71` for the plain one), which is what
/// `getFunctionCollection` (`:99-103`) hands the executor.
///
/// The two refusals are not the same shape. A tag the library does not hold at
/// all is `arguments.function.tag.unknown` (`:80-84`), while a tag it holds but
/// that resolved to nothing is a perfectly good empty collection — vanilla
/// distinguishes them on the null the map lookup returned, which is why an
/// absent tag and an empty one part company here rather than in the caller.
pub fn resolve_functions(
    tag: bool,
    name: &str,
    world: &ServerWorld,
) -> Result<Vec<String>, NbtSummaryValue> {
    if tag {
        return world
            .command_functions()
            .tag(name)
            .map(<[String]>::to_vec)
            .ok_or_else(|| {
                translatable(
                    "arguments.function.tag.unknown",
                    vec![component_literal_nbt(name)],
                )
            });
    }
    if world.command_functions().definition(name).is_some() {
        return Ok(vec![name.to_string()]);
    }
    Err(translatable(
        "arguments.function.unknown",
        vec![component_literal_nbt(name)],
    ))
}

/// `FunctionCommand.modifySenderForExecution` (`FunctionCommand.java:106-108`).
///
/// `withMaximumPermission` calls `this.permissions.union(GAMEMASTER)`
/// (`CommandSourceStack.java:282-284`); on `LevelBasedPermissionSet`, `union`
/// picks the *lower* level (`LevelBasedPermissionSet.java:23-29`,
/// `PermissionLevel.java:26-28`), capping permissions at `GameMaster`.
/// `withSuppressedOutput` (`:240-259`) is the other half.
pub fn modify_sender_for_execution(source: &ExecutionSource) -> ExecutionSource {
    let cap = |permission: CommandPermission| permission.min(CommandPermission::GameMaster);
    ExecutionSource {
        source: CommandSource {
            permission: cap(source.source.permission),
            ..source.source.clone()
        },
        executing: source.executing.as_ref().map(|executing| CommandSource {
            permission: cap(executing.permission),
            ..executing.clone()
        }),
        silent: source.silent || source.trace.is_none(),
        // A body line's message arguments are not the caller's signed ones.
        chat_signer: None,
        ..source.clone()
    }
}
