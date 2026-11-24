//! `/data` and the shared `DataCommands.DataProvider` readers.

use bbb_protocol::nbt_summary::codec_double_value as numeric_value;
use bbb_protocol::{
    canonical_snbt_string, command_nbt_is_too_deep, component_literal_nbt,
    merge_command_nbt_compound, CommandNbtPath, CommandNbtPathErrorKind, CommandNbtPathParseError,
    NbtSummaryValue,
};

use crate::commands::parse::{
    ArgumentValue, ExecuteDataProvider, ExecuteStoreProvider, ParseResult,
};
use crate::commands::types::CommandDataTarget;
use crate::commands::{translatable, CommandOrigin, CommandSource};
use crate::world::ServerWorld;

use super::blocks::{block_entity_data_at, loaded_block_pos, loaded_block_pos_of};
use super::{
    single_damage_entity_argument, single_damage_entity_of, CommandCx, CommandRun, Outcome,
};
use crate::commands::types::CommandDamageEntityRef;

/// The one `DataAccessor.setData` a `/data` run makes, with the line that
/// reports it (DataCommands.java:313-315, :325-327, :384-386).
type DataWrite = Option<(CommandDataTarget, NbtSummaryValue, NbtSummaryValue)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    Entity,
    Block,
    Storage,
}

#[derive(Debug, Clone)]
enum AccessorIdentity {
    Block {
        level: bbb_server_types::save::Java26LevelKey,
        pos: bbb_protocol::packets::BlockPos,
    },
    Entity {
        display_name: NbtSummaryValue,
        target: Option<CommandDataTarget>,
    },
    Storage(String),
}

#[derive(Debug, Clone)]
struct Accessor {
    data: NbtSummaryValue,
    identity: AccessorIdentity,
}

impl Accessor {
    fn mutable_target(&self) -> Result<CommandDataTarget, NbtSummaryValue> {
        match &self.identity {
            AccessorIdentity::Block { level, pos } => Ok(CommandDataTarget::Block {
                level: level.clone(),
                pos: *pos,
            }),
            AccessorIdentity::Entity {
                target: Some(target),
                ..
            } => Ok(target.clone()),
            AccessorIdentity::Entity { target: None, .. } => {
                Err(translatable("commands.data.entity.invalid", Vec::new()))
            }
            AccessorIdentity::Storage(id) => Ok(CommandDataTarget::Storage { id: id.clone() }),
        }
    }

    fn modified_success(&self) -> NbtSummaryValue {
        match &self.identity {
            AccessorIdentity::Block { pos, .. } => translatable(
                "commands.data.block.modified",
                vec![
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                ],
            ),
            AccessorIdentity::Entity { display_name, .. } => {
                translatable("commands.data.entity.modified", vec![display_name.clone()])
            }
            AccessorIdentity::Storage(id) => translatable(
                "commands.data.storage.modified",
                vec![NbtSummaryValue::string(id.clone())],
            ),
        }
    }

    fn print_success(&self, data: &NbtSummaryValue) -> NbtSummaryValue {
        let pretty = super::text::nbt::display(data, false);
        match &self.identity {
            AccessorIdentity::Block { pos, .. } => translatable(
                "commands.data.block.query",
                vec![
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                    pretty,
                ],
            ),
            AccessorIdentity::Entity { display_name, .. } => translatable(
                "commands.data.entity.query",
                vec![display_name.clone(), pretty],
            ),
            AccessorIdentity::Storage(id) => translatable(
                "commands.data.storage.query",
                vec![NbtSummaryValue::string(id.clone()), pretty],
            ),
        }
    }

    fn numeric_success(
        &self,
        path: &bbb_protocol::JavaString,
        scale: f64,
        value: i32,
    ) -> NbtSummaryValue {
        let scale = format_scale(scale);
        match &self.identity {
            AccessorIdentity::Block { pos, .. } => translatable(
                "commands.data.block.get",
                vec![
                    NbtSummaryValue::string(path),
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                    NbtSummaryValue::string(scale),
                    NbtSummaryValue::Int(value),
                ],
            ),
            AccessorIdentity::Entity { display_name, .. } => translatable(
                "commands.data.entity.get",
                vec![
                    NbtSummaryValue::string(path),
                    display_name.clone(),
                    NbtSummaryValue::string(scale),
                    NbtSummaryValue::Int(value),
                ],
            ),
            AccessorIdentity::Storage(id) => translatable(
                "commands.data.storage.get",
                vec![
                    NbtSummaryValue::string(path),
                    NbtSummaryValue::string(id.clone()),
                    NbtSummaryValue::string(scale),
                    NbtSummaryValue::Int(value),
                ],
            ),
        }
    }
}

pub(super) fn data_command(
    run: &mut CommandRun,
    path: &[&str],
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let mut io = None;
    let (world, at) = (&mut *cx.world, &cx.origin);
    let result = match path.get(1).copied() {
        Some("merge") => merge_data(&mut io, provider(path[2]), parse, source, world, at),
        Some("get") => get_data(run, provider(path[2]), parse, source, world, at),
        Some("remove") => remove_data(&mut io, provider(path[2]), parse, source, world, at),
        Some("modify") => modify_data(&mut io, path, parse, source, world, at),
        _ => unreachable!("no /data executor for {path:?}"),
    };
    let result = result?;
    if let Some((target, data, feedback)) = io {
        cx.world.set_command_data(&target, &data)?;
        run.succeed(feedback, true);
    }
    Ok(result)
}

/// `FunctionCommand`'s `with` source tree (FunctionCommand.java:64-77).
pub(super) fn function_arguments(
    path: &[&str],
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<NbtSummaryValue>, NbtSummaryValue> {
    if path.get(2) != Some(&"with") {
        return Ok(None);
    }
    let accessor = access(provider(path[3]), "source", parse, source, world, at)?;
    match path_argument(parse, "path") {
        Some(path) => single_tag(path, &accessor.data).map(Some),
        None => Ok(Some(accessor.data)),
    }
}

fn merge_data(
    io: &mut DataWrite,
    provider: Provider,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let mut accessor = access(provider, "target", parse, source, world, at)?;
    let Some(ArgumentValue::CompoundTag(NbtSummaryValue::Compound(source))) = parse.argument("nbt")
    else {
        unreachable!("nbt is a compound argument")
    };
    if command_nbt_is_too_deep(&NbtSummaryValue::Compound(source.clone()), 0) {
        return Err(translatable("arguments.nbtpath.too_deep", Vec::new()));
    }
    let NbtSummaryValue::Compound(target) = &mut accessor.data else {
        unreachable!("data accessors always return compounds")
    };
    if !merge_command_nbt_compound(target, source) {
        return Err(unchanged());
    }
    commit(io, &accessor, 1)
}

fn get_data(
    run: &mut CommandRun,
    provider: Provider,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let accessor = access(provider, "target", parse, source, world, at)?;
    let Some(path) = path_argument(parse, "path") else {
        run.succeed(accessor.print_success(&accessor.data), false);
        return Ok(1);
    };
    let value = single_tag(path, &accessor.data)?;
    let Some(ArgumentValue::Double(scale)) = parse.argument("scale") else {
        let result = value_result(&value, path.source())?;
        run.succeed(accessor.print_success(&value), false);
        return Ok(result);
    };
    let Some(value) = numeric_value(&value) else {
        return Err(translatable(
            "commands.data.get.invalid",
            vec![NbtSummaryValue::string(path.source())],
        ));
    };
    let result = java_floor_to_int(value * scale);
    run.succeed(
        accessor.numeric_success(path.source(), *scale, result),
        false,
    );
    Ok(result)
}

fn remove_data(
    io: &mut DataWrite,
    provider: Provider,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let mut accessor = access(provider, "target", parse, source, world, at)?;
    let path = required_path(parse, "path");
    let changed = path.remove(&mut accessor.data);
    if changed == 0 {
        return Err(unchanged());
    }
    commit(io, &accessor, usize_to_i32(changed))
}

fn modify_data(
    io: &mut DataWrite,
    path: &[&str],
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let target_path = required_path(parse, "targetPath");
    let operation_index = 5;
    let operation = path[operation_index];
    let source_start = operation_index + if operation == "insert" { 2 } else { 1 };
    // Java evaluates the source-list argument before `manipulateData` enters
    // and opens its target accessor (DataCommands.java:192-215, :299-307).
    let values = source_values(&path[source_start..], parse, source, world, at)?;
    let mut target = access(provider(path[2]), "target", parse, source, world, at)?;
    let changed = match operation {
        "insert" => target_path.insert(&mut target.data, integer_argument(parse, "index"), &values),
        "prepend" => target_path.insert(&mut target.data, 0, &values),
        "append" => target_path.insert(&mut target.data, -1, &values),
        "set" => target_path.set(
            &mut target.data,
            values.last().expect("every source is nonempty").clone(),
        ),
        "merge" => merge_at_path(target_path, &mut target.data, &values).map_err(|err| *err),
        _ => unreachable!("unknown /data modify operation {operation}"),
    }
    .map_err(path_error)?;
    if changed == 0 {
        return Err(unchanged());
    }
    commit(io, &target, usize_to_i32(changed))
}

fn source_values(
    path: &[&str],
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Vec<NbtSummaryValue>, NbtSummaryValue> {
    if path.first() == Some(&"value") {
        let Some(ArgumentValue::Tag(value)) = parse.argument("value") else {
            unreachable!("value is an nbt tag argument")
        };
        return Ok(vec![value.clone()]);
    }
    let stringify = path.first() == Some(&"string");
    debug_assert!(matches!(path.first(), Some(&"from" | &"string")));
    let source_provider = provider(path[1]);
    let accessor = access(source_provider, "source", parse, source, world, at)?;
    let mut values = match path_argument(parse, "sourcePath") {
        Some(path) => path.get(&accessor.data).map_err(path_error)?,
        None => vec![accessor.data],
    };
    if stringify {
        let start = optional_integer_argument(parse, "start");
        let end = optional_integer_argument(parse, "end");
        values = values
            .into_iter()
            .map(|value| stringify_value(value, start, end))
            .collect::<Result<_, _>>()?;
    }
    Ok(values)
}

fn merge_at_path(
    path: &CommandNbtPath,
    target: &mut NbtSummaryValue,
    values: &[NbtSummaryValue],
) -> Result<usize, Box<CommandNbtPathParseError>> {
    let error = |kind, context| {
        let prefix = path.source().to_display_string();
        Box::new(CommandNbtPathParseError {
            kind,
            node_index: path.nodes().len().checked_sub(1),
            offset: prefix.len(),
            prefix,
            context,
            snbt: None,
        })
    };
    let mut combined = Vec::new();
    for value in values {
        if command_nbt_is_too_deep(value, 0) {
            return Err(error(
                CommandNbtPathErrorKind::TooDeep,
                "source is too deeply nested".to_string(),
            ));
        }
        let NbtSummaryValue::Compound(entries) = value else {
            return Err(error(
                CommandNbtPathErrorKind::InvalidNode,
                format!("expected compound: {}", canonical_snbt_string(value)),
            ));
        };
        merge_command_nbt_compound(&mut combined, entries);
    }
    let selected = path
        .get_or_create(target, NbtSummaryValue::Compound(Vec::new()))
        .map_err(Box::new)?;
    if let Some(value) = selected
        .iter()
        .find(|value| !matches!(value, NbtSummaryValue::Compound(_)))
    {
        return Err(error(
            CommandNbtPathErrorKind::InvalidNode,
            format!("expected compound: {}", canonical_snbt_string(value)),
        ));
    }
    path.merge_compound(target, &combined).map_err(Box::new)
}

fn access(
    provider: Provider,
    prefix: &str,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Accessor, NbtSummaryValue> {
    match provider {
        Provider::Block => {
            let argument = if prefix == "target" {
                "targetPos"
            } else {
                "sourcePos"
            };
            let blocks = world.command_blocks(&at.level);
            block_accessor(loaded_block_pos(parse, argument, blocks, at)?, world, at)
        }
        Provider::Entity => {
            let identity = world.with_entity_reads(|reads| {
                Ok::<_, NbtSummaryValue>(entity_identity(
                    single_damage_entity_argument(parse, prefix, source, at, reads)?,
                    reads,
                ))
            })?;
            entity_accessor(identity, world)
        }
        Provider::Storage => storage_accessor(identifier_argument(parse, prefix), world),
    }
}

/// `BlockDataAccessor.PROVIDER.access` (BlockDataAccessor.java:32-40) past
/// `getLoadedBlockPos`, whose data is `getData` (`:70-72`) --
/// `saveWithFullMetadata(level.registryAccess())`, so the record every `/data`
/// verb reads and merges back carries the stacks whose components name a
/// dynamic-registry holder.
fn block_accessor(
    pos: bbb_protocol::packets::BlockPos,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Result<Accessor, NbtSummaryValue> {
    let resolver = world.item_stack_resolver();
    let blocks = world.command_blocks(&at.level);
    let Some(data) = block_entity_data_at(blocks, pos, Some(resolver.as_dyn()))? else {
        return Err(translatable("commands.data.block.invalid", Vec::new()));
    };
    require_compound(
        data,
        AccessorIdentity::Block {
            level: at.level.clone(),
            pos,
        },
    )
}

fn entity_identity(
    entity: CommandDamageEntityRef<'_>,
    reads: &crate::world::EntityReads<'_>,
) -> (uuid::Uuid, AccessorIdentity) {
    let (uuid, target) = match entity {
        CommandDamageEntityRef::Player(player) => (player.id, None),
        CommandDamageEntityRef::Stored(stored) => (
            stored.entity.uuid(),
            Some(CommandDataTarget::StoredEntity {
                level: stored.dimension.clone(),
                entity_id: stored.entity.id(),
            }),
        ),
    };
    (
        uuid,
        AccessorIdentity::Entity {
            display_name: entity.display_name(reads),
            target,
        },
    )
}

/// `EntityDataAccessor.PROVIDER.access` (EntityDataAccessor.java:31-33) past
/// `EntityArgument.getEntity`, whose data is `getData` (`:64-66`).
fn entity_accessor(
    (uuid, identity): (uuid::Uuid, AccessorIdentity),
    world: &mut ServerWorld,
) -> Result<Accessor, NbtSummaryValue> {
    if let AccessorIdentity::Entity {
        target: Some(CommandDataTarget::StoredEntity { level, entity_id }),
        ..
    } = &identity
    {
        world
            .materialize_merchant_command_data(level, *entity_id)
            .map_err(component_literal_nbt)?;
    }
    let data = world
        .entity_reads()
        .entity_data(uuid)
        .map_err(component_literal_nbt)?
        .ok_or_else(|| translatable("argument.entity.notfound.entity", Vec::new()))?;
    require_compound(data, identity)
}

/// `storeData`'s accessor (ExecuteCommand.java:440-456) as one `store` arm's
/// provider resolved it, split into what the callback needs: the target its
/// `setData` writes, the compound `getData` answers, and the storage namespace
/// that read publishes.
///
/// `None` for the target is an accessor whose `setData` refuses --
/// `EntityDataAccessor.ERROR_NO_PLAYERS` (EntityDataAccessor.java:28, thrown at
/// `:52`) -- which the callback's own `catch` swallows without a message.
pub(in crate::commands::run) fn store_accessor(
    provider: &ExecuteStoreProvider,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<CommandDataTarget>, NbtSummaryValue> {
    let accessor = match provider {
        ExecuteStoreProvider::Block(coordinates) => block_accessor(
            loaded_block_pos_of(*coordinates, world.command_blocks(&at.level), at)?,
            world,
            at,
        )?,
        ExecuteStoreProvider::Entity(selector) => {
            let identity = world.with_entity_reads(|reads| {
                Ok::<_, NbtSummaryValue>(entity_identity(
                    single_damage_entity_of(selector, source, at, reads)?,
                    reads,
                ))
            })?;
            entity_accessor(identity, world)?
        }
        ExecuteStoreProvider::Storage(id) => storage_accessor(id, world)?,
    };
    Ok(accessor.mutable_target().ok())
}

/// `DataAccessor.getData` for a target `store_accessor` resolved, as `world`
/// holds it now. `None` is a target that is gone, which only the accessor
/// object vanilla captured still answers for.
pub(crate) fn accessor_data(
    target: &CommandDataTarget,
    world: &mut ServerWorld,
) -> Result<Option<NbtSummaryValue>, NbtSummaryValue> {
    match target {
        CommandDataTarget::Block { level, pos } => {
            if world.command_level(level).is_none() {
                return Ok(None);
            }
            let resolver = world.item_stack_resolver();
            block_entity_data_at(world.command_blocks(level), *pos, Some(resolver.as_dyn()))
        }
        CommandDataTarget::StoredEntity { level, entity_id } => {
            let uuid = {
                let reads = world.entity_reads();
                let uuid = reads
                    .command_entity_by_id(level, *entity_id)
                    .and_then(|entity| match entity {
                        CommandDamageEntityRef::Stored(stored) => Some(stored.entity.uuid()),
                        CommandDamageEntityRef::Player(_) => None,
                    });
                uuid
            };
            let Some(uuid) = uuid else {
                return Ok(None);
            };
            world
                .materialize_merchant_command_data(level, *entity_id)
                .map_err(component_literal_nbt)?;
            world
                .entity_reads()
                .entity_data(uuid)
                .map_err(component_literal_nbt)
        }
        CommandDataTarget::Storage { id } => Ok(Some(
            world
                .storage_data(id)
                .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new())),
        )),
    }
}

/// `checkMatchingData` (ExecuteCommand.java:826-828) over the
/// `DataCommands.SOURCE_PROVIDERS` accessor an `if data` arm names, read off
/// the values the arm carries out of its own node rather than off an argument
/// name.
pub(in crate::commands::run) fn matching_data(
    provider: &ExecuteDataProvider,
    path: &CommandNbtPath,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<i32, NbtSummaryValue> {
    let accessor = match provider {
        ExecuteDataProvider::Block(coordinates) => block_accessor(
            loaded_block_pos_of(*coordinates, world.command_blocks(&at.level), at)?,
            world,
            at,
        )?,
        ExecuteDataProvider::Entity(selector) => {
            let identity = world.with_entity_reads(|reads| {
                Ok::<_, NbtSummaryValue>(entity_identity(
                    single_damage_entity_of(selector, source, at, reads)?,
                    reads,
                ))
            })?;
            entity_accessor(identity, world)?
        }
        ExecuteDataProvider::Storage(id) => storage_accessor(id, world)?,
    };
    Ok(usize_to_i32(path.count_matching(&accessor.data)))
}

/// `StorageDataAccessor.getData` (StorageDataAccessor.java:56-58), whose read
/// publishes the namespace through `CommandStorage.getContainer`
/// (CommandStorage.java:30-42).
fn storage_accessor(id: &str, world: &ServerWorld) -> Result<Accessor, NbtSummaryValue> {
    let data = world
        .storage_data(id)
        .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
    require_compound(data, AccessorIdentity::Storage(id.to_string()))
}

fn require_compound(
    data: NbtSummaryValue,
    identity: AccessorIdentity,
) -> Result<Accessor, NbtSummaryValue> {
    if matches!(data, NbtSummaryValue::Compound(_)) {
        Ok(Accessor { data, identity })
    } else {
        Err(translatable("commands.data.get.invalid", Vec::new()))
    }
}

fn commit(io: &mut DataWrite, accessor: &Accessor, result: i32) -> Outcome {
    *io = Some((
        accessor.mutable_target()?,
        accessor.data.clone(),
        accessor.modified_success(),
    ));
    Ok(result)
}

fn single_tag(
    path: &CommandNbtPath,
    data: &NbtSummaryValue,
) -> Result<NbtSummaryValue, NbtSummaryValue> {
    let values = path.get(data).map_err(path_error)?;
    if values.len() != 1 {
        return Err(translatable("commands.data.get.multiple", Vec::new()));
    }
    Ok(values.into_iter().next().expect("one value"))
}

fn value_result(
    value: &NbtSummaryValue,
    path: &bbb_protocol::JavaString,
) -> Result<i32, NbtSummaryValue> {
    if let Some(value) = numeric_value(value) {
        return Ok(java_floor_to_int(value));
    }
    let len = match value {
        NbtSummaryValue::String(value) => value.units().len(),
        NbtSummaryValue::List(value) => value.len(),
        NbtSummaryValue::Compound(value) => value.len(),
        NbtSummaryValue::ByteArray(value) => value.len(),
        NbtSummaryValue::IntArray(value) => value.len(),
        NbtSummaryValue::LongArray(value) => value.len(),
        _ => {
            return Err(translatable(
                "commands.data.get.unknown",
                vec![NbtSummaryValue::string(path)],
            ))
        }
    };
    Ok(usize_to_i32(len))
}

fn stringify_value(
    value: NbtSummaryValue,
    start: Option<i32>,
    end: Option<i32>,
) -> Result<NbtSummaryValue, NbtSummaryValue> {
    let value = match value {
        NbtSummaryValue::String(value) => value,
        value if numeric_value(&value).is_some() => {
            bbb_protocol::canonical_snbt_java_string(&value)
        }
        value => {
            return Err(translatable(
                "commands.data.modify.expected_value",
                vec![component_literal_nbt(canonical_snbt_string(&value))],
            ))
        }
    };
    let Some(start) = start else {
        return Ok(NbtSummaryValue::String(value));
    };
    let units = value.units();
    let absolute_start = offset(start, units.len());
    let absolute_end = end.map_or(units.len() as i64, |end| offset(end, units.len()));
    if absolute_start < 0
        || absolute_end < absolute_start
        || absolute_end > i64::try_from(units.len()).unwrap_or(i64::MAX)
    {
        return Err(translatable(
            "commands.data.modify.invalid_substring",
            vec![
                NbtSummaryValue::Int(i32::try_from(absolute_start).unwrap_or(i32::MIN)),
                NbtSummaryValue::Int(i32::try_from(absolute_end).unwrap_or(i32::MAX)),
            ],
        ));
    }
    let selected = &units[absolute_start as usize..absolute_end as usize];
    Ok(NbtSummaryValue::string_from_utf16_slice(selected))
}

fn offset(index: i32, len: usize) -> i64 {
    if index >= 0 {
        i64::from(index)
    } else {
        i64::try_from(len).unwrap_or(i64::MAX) + i64::from(index)
    }
}

fn java_floor_to_int(value: f64) -> i32 {
    let narrowed = value as i32;
    if value < f64::from(narrowed) {
        narrowed.wrapping_sub(1)
    } else {
        narrowed
    }
}

fn format_scale(scale: f64) -> String {
    if scale.is_nan() {
        "NaN".to_string()
    } else if scale == f64::INFINITY {
        "Infinity".to_string()
    } else if scale == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if scale.abs() * 100.0 >= i64::MAX as f64 {
        format!("{scale:.2}")
    } else {
        let formatted = crate::tick_rate::java_fixed_decimals(scale, 2);
        if scale.is_sign_negative() && !formatted.starts_with('-') {
            format!("-{formatted}")
        } else {
            formatted
        }
    }
}

fn provider(value: &str) -> Provider {
    match value {
        "entity" => Provider::Entity,
        "block" => Provider::Block,
        "storage" => Provider::Storage,
        _ => unreachable!("unknown data provider {value}"),
    }
}

fn path_argument<'a>(parse: &'a ParseResult, name: &str) -> Option<&'a CommandNbtPath> {
    match parse.argument(name) {
        Some(ArgumentValue::NbtPath(path)) => Some(path),
        None => None,
        other => unreachable!("{name} is an NBT path argument, not {other:?}"),
    }
}

fn required_path<'a>(parse: &'a ParseResult, name: &str) -> &'a CommandNbtPath {
    path_argument(parse, name).expect("required NBT path")
}

fn integer_argument(parse: &ParseResult, name: &str) -> i32 {
    match parse.argument(name) {
        Some(ArgumentValue::Integer(value)) => *value,
        other => unreachable!("{name} is an integer argument, not {other:?}"),
    }
}

fn optional_integer_argument(parse: &ParseResult, name: &str) -> Option<i32> {
    match parse.argument(name) {
        Some(ArgumentValue::Integer(value)) => Some(*value),
        None => None,
        other => unreachable!("{name} is an integer argument, not {other:?}"),
    }
}

fn identifier_argument<'a>(parse: &'a ParseResult, name: &str) -> &'a str {
    match parse.argument(name) {
        Some(ArgumentValue::Identifier(id)) => id,
        other => unreachable!("{name} is a resource-location argument, not {other:?}"),
    }
}

fn path_error(error: CommandNbtPathParseError) -> NbtSummaryValue {
    match error.kind {
        CommandNbtPathErrorKind::NothingFound => translatable(
            "arguments.nbtpath.nothing_found",
            vec![NbtSummaryValue::string(error.prefix)],
        ),
        CommandNbtPathErrorKind::ExpectedList => translatable(
            "commands.data.modify.expected_list",
            vec![NbtSummaryValue::string(error.context)],
        ),
        CommandNbtPathErrorKind::InvalidIndex => translatable(
            "commands.data.modify.invalid_index",
            vec![NbtSummaryValue::string(error.context)],
        ),
        CommandNbtPathErrorKind::TooDeep => translatable("arguments.nbtpath.too_deep", Vec::new()),
        CommandNbtPathErrorKind::InvalidNode if error.context.starts_with("expected compound:") => {
            translatable(
                "commands.data.modify.expected_object",
                vec![NbtSummaryValue::String(
                    error
                        .context
                        .trim_start_matches("expected compound: ")
                        .to_string()
                        .into(),
                )],
            )
        }
        CommandNbtPathErrorKind::InvalidNode => {
            translatable("arguments.nbtpath.node.invalid", Vec::new())
        }
    }
}

fn unchanged() -> NbtSummaryValue {
    translatable("commands.data.merge.failed", Vec::new())
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
