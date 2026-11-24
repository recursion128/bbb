//! `/place`.

use bbb_protocol::{component_literal_nbt, packets::BlockPos, NbtSummaryValue};

use crate::chunk::ChunkStore;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandOrigin};
use crate::world::ServerWorld;

use super::blocks::loaded_block_pos;
use super::{CommandCx, CommandRun, Outcome};

/// `PlaceCommand.placeFeature` (`PlaceCommand.java:249-260`) after
/// `ResourceKeyArgument.getConfiguredFeature` (`ResourceKeyArgument.java:87-89`).
pub(super) fn place_feature(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    has_pos: bool,
) -> Outcome {
    let feature_id = feature_resource(parse);
    let (level, pos) = feature_origin(parse, cx.world, &cx.origin, feature_id, has_pos)?;
    let level = cx.world.require_level_id(&level);
    if !cx
        .world
        .place_live_configured_feature(level, feature_id, pos)
    {
        return Err(translatable("commands.place.feature.failed", Vec::new()));
    }
    run.succeed(
        translatable(
            "commands.place.feature.success",
            vec![
                component_literal_nbt(feature_id),
                NbtSummaryValue::Int(pos.x),
                NbtSummaryValue::Int(pos.y),
                NbtSummaryValue::Int(pos.z),
            ],
        ),
        true,
    );
    Ok(1)
}

fn feature_origin(
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    feature_id: &str,
    has_pos: bool,
) -> Result<(bbb_server_types::save::Java26LevelKey, BlockPos), NbtSummaryValue> {
    if !world
        .startup_registry_ids("minecraft:worldgen/configured_feature")
        .iter()
        .any(|id| id == feature_id)
    {
        return Err(translatable(
            "commands.place.feature.invalid",
            vec![component_literal_nbt(feature_id)],
        ));
    }
    let blocks = world.command_blocks(&at.level);
    let pos = if has_pos {
        loaded_block_pos(parse, "pos", blocks, at)?
    } else {
        BlockPos {
            x: at.position[0].floor() as i32,
            y: at.position[1].floor() as i32,
            z: at.position[2].floor() as i32,
        }
    };
    check_loaded_neighborhood(blocks, pos)?;
    Ok((at.level.clone(), pos))
}

fn feature_resource(parse: &ParseResult) -> &str {
    match parse.argument("feature") {
        Some(ArgumentValue::Resource(id)) => id,
        other => unreachable!("feature is a resource-key argument, not {other:?}"),
    }
}

/// `PlaceCommand.checkLoaded` (`PlaceCommand.java:361-365`) over the 3×3
/// neighborhood `placeFeature` asks for (`:252-253`).
fn check_loaded_neighborhood(blocks: &ChunkStore, pos: BlockPos) -> Result<(), NbtSummaryValue> {
    let chunk_x = pos.x >> 4;
    let chunk_z = pos.z >> 4;
    for dz in -1..=1 {
        for dx in -1..=1 {
            let sample = BlockPos {
                x: (chunk_x + dx) << 4,
                y: 0,
                z: (chunk_z + dz) << 4,
            };
            if !blocks.has_chunk_at(sample) {
                return Err(translatable("argument.pos.unloaded", Vec::new()));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceCommandRequest {
    pub(crate) level: bbb_server_types::save::Java26LevelKey,
    pub(crate) pos: BlockPos,
    pub(crate) kind: PlaceCommandKind,
    pub(crate) check_position: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlaceCommandKind {
    Structure {
        id: String,
    },
    Jigsaw {
        pool: String,
        target: String,
        max_depth: i32,
    },
    Template {
        id: String,
        rotation: bbb_server_worldgen::noise::structure_pieces::JigsawRotation,
        mirror: bbb_server_worldgen::noise::ruined_portal::RuinedPortalMirror,
        integrity: f32,
        seed: i32,
        strict: bool,
    },
}

impl PlaceCommandRequest {
    fn success_feedback(&self) -> NbtSummaryValue {
        let (key, id) = match &self.kind {
            PlaceCommandKind::Structure { id } => ("commands.place.structure.success", Some(id)),
            PlaceCommandKind::Jigsaw { .. } => ("commands.place.jigsaw.success", None),
            PlaceCommandKind::Template { id, .. } => ("commands.place.template.success", Some(id)),
        };
        let mut args = Vec::new();
        if let Some(id) = id {
            args.push(component_literal_nbt(id));
        }
        args.extend([
            NbtSummaryValue::Int(self.pos.x),
            NbtSummaryValue::Int(self.pos.y),
            NbtSummaryValue::Int(self.pos.z),
        ]);
        translatable(key, args)
    }
}

pub(super) fn place_command(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    branch: &str,
    strict: bool,
) -> Outcome {
    let request = place_request(parse, cx.world, &cx.origin, branch, strict)?;
    cx.world.place_live_command(&request)?;
    run.succeed(request.success_feedback(), true);
    Ok(1)
}

fn place_request(
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    branch: &str,
    strict: bool,
) -> Result<PlaceCommandRequest, NbtSummaryValue> {
    use bbb_server_worldgen::noise::ruined_portal::RuinedPortalMirror;
    use bbb_server_worldgen::noise::structure_pieces::JigsawRotation;
    let resource = |name| match parse.argument(name) {
        Some(ArgumentValue::Resource(id) | ArgumentValue::Identifier(id)) => id.clone(),
        other => unreachable!("placement resource {name}: {other:?}"),
    };
    let integer = |name, default| match parse.argument(name) {
        Some(ArgumentValue::Integer(value)) => *value,
        None => default,
        _ => unreachable!(),
    };
    let text = |name| match parse.argument(name) {
        Some(ArgumentValue::Text(value)) => Some(value),
        None => None,
        _ => unreachable!(),
    };
    let (kind, pos_name) = if branch == "jigsaw" {
        (
            PlaceCommandKind::Jigsaw {
                pool: resource("pool"),
                target: resource("target"),
                max_depth: integer("max_depth", 1),
            },
            "position",
        )
    } else if branch == "structure" {
        (
            PlaceCommandKind::Structure {
                id: resource("structure"),
            },
            "pos",
        )
    } else {
        let rotation = match text("rotation") {
            None => JigsawRotation::None,
            Some(value) if value == "none" => JigsawRotation::None,
            Some(value) if value == "clockwise_90" => JigsawRotation::Clockwise90,
            Some(value) if value == "180" => JigsawRotation::Clockwise180,
            Some(value) if value == "counterclockwise_90" => JigsawRotation::CounterClockwise90,
            _ => unreachable!(),
        };
        let mirror = match text("mirror") {
            None => RuinedPortalMirror::None,
            Some(value) if value == "none" => RuinedPortalMirror::None,
            Some(value) if value == "left_right" => RuinedPortalMirror::LeftRight,
            Some(value) if value == "front_back" => RuinedPortalMirror::FrontBack,
            _ => unreachable!(),
        };
        let integrity = match parse.argument("integrity") {
            Some(ArgumentValue::Float(value)) => *value,
            None => 1.0,
            _ => unreachable!(),
        };
        (
            PlaceCommandKind::Template {
                id: resource("template"),
                rotation,
                mirror,
                integrity,
                seed: integer("seed", 0),
                strict,
            },
            "pos",
        )
    };
    let check_position = branch != "template" && parse.argument(pos_name).is_some();
    let pos = if let Some(ArgumentValue::BlockPos(coordinates)) = parse.argument(pos_name) {
        if branch == "template" {
            loaded_block_pos(parse, pos_name, world.command_blocks(&at.level), at)?
        } else {
            coordinates.block_pos(at)
        }
    } else {
        BlockPos {
            x: at.position[0].floor() as i32,
            y: at.position[1].floor() as i32,
            z: at.position[2].floor() as i32,
        }
    };
    Ok(PlaceCommandRequest {
        level: at.level.clone(),
        pos,
        kind,
        check_position,
    })
}
