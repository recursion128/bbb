//! `/summon` against the world's entity constructors and NBT reader.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::vanilla_entity_type_for_resource_id;
use bbb_protocol::{component_literal_nbt, NbtSummaryEntry, NbtSummaryValue};

use crate::commands::catalog;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandCx, CommandOrigin};
use crate::world::ServerWorld;

use super::teleport::is_in_spawnable_bounds;
use super::{CommandRun, Outcome};

/// `SummonCommand.spawnEntity` (`SummonCommand.java:105-111`, `:71-103`).
pub(super) fn summon_entity(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    has_pos: bool,
) -> Outcome {
    let resource_id = entity_resource(parse);
    let position = if has_pos {
        match parse.argument("pos") {
            Some(ArgumentValue::Coordinates(coordinates)) => coordinates.position(&cx.origin),
            other => unreachable!("pos is a vec3 argument, not {other:?}"),
        }
    } else {
        cx.origin.position
    };
    let nbt = match parse.argument("nbt") {
        Some(ArgumentValue::CompoundTag(NbtSummaryValue::Compound(entries))) => {
            Some(entries.as_slice())
        }
        None => None,
        other => unreachable!("nbt is a compound argument, not {other:?}"),
    };
    let id = create_entity(cx.world, &cx.origin, resource_id, position, nbt)?;
    // `SummonCommand.spawnEntity` (`SummonCommand.java:109-111`).
    let name = cx.world.with_entity_reads(|reads| {
        reads
            .command_entity_by_id(&cx.origin.level, id)
            .expect("a summoned entity is published in its source level")
            .display_name(reads)
    });
    run.succeed(translatable("commands.summon.success", vec![name]), true);
    Ok(1)
}

pub(in crate::commands) fn create_entity(
    world: &mut ServerWorld,
    origin: &CommandOrigin,
    resource_id: &str,
    position: [f64; 3],
    nbt: Option<&[NbtSummaryEntry]>,
) -> Result<i32, NbtSummaryValue> {
    let (entity_type_id, can_summon, allowed_in_peaceful) =
        if let Some(entry) = vanilla_entity_type_for_resource_id(resource_id) {
            (entry.id, entry.can_summon, entry.allowed_in_peaceful)
        } else if let Some(spec) = world
            .plugins()
            .content_registry()
            .entity_by_resource_id(resource_id)
        {
            (spec.id, spec.can_summon, spec.allowed_in_peaceful)
        } else {
            return Err(translatable(
                "argument.resource.not_found",
                vec![
                    component_literal_nbt(resource_id),
                    component_literal_nbt(catalog::ENTITY_TYPE_REGISTRY),
                ],
            ));
        };
    // `ResourceArgument.getSummonableEntityType` (`ResourceArgument.java:88-94`).
    if !can_summon {
        return Err(translatable(
            "entity.not_summonable",
            vec![component_literal_nbt(resource_id)],
        ));
    }
    // `BlockPos.containing` then `Level.isInSpawnableBounds` (`:74-77`).
    let block = [
        position[0].floor() as i32,
        position[1].floor() as i32,
        position[2].floor() as i32,
    ];
    if !position.iter().all(|axis| axis.is_finite()) || !is_in_spawnable_bounds(block) {
        return Err(translatable("commands.summon.invalidPosition", Vec::new()));
    }
    if world.difficulty() == Difficulty::Peaceful && !allowed_in_peaceful {
        return Err(translatable("commands.summon.failed.peaceful", Vec::new()));
    }
    world
        .summon_by_command(&origin.level, entity_type_id, position, nbt)
        .ok_or_else(|| {
            // `:99`: the add refused a UUID already loaded in the level.
            let key = if nbt.is_some_and(|nbt| world.summon_uuid_taken(&origin.level, nbt)) {
                "commands.summon.failed.uuid"
            } else {
                "commands.summon.failed"
            };
            translatable(key, Vec::new())
        })
}

fn entity_resource(parse: &ParseResult) -> &str {
    match parse.argument("entity") {
        Some(ArgumentValue::Resource(id)) => id,
        other => unreachable!("entity is a resource argument, not {other:?}"),
    }
}
