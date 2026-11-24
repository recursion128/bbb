//! `/fillbiome`.

use bbb_protocol::{component_literal_nbt, packets::BlockPos, NbtSummaryValue};
use bbb_server_types::bounding_box::BoundingBox;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandOrigin};
use crate::world::ServerWorld;

use super::blocks::loaded_block_pos;
use super::{CommandCx, CommandRun, Outcome};

const RULE_MAX_BLOCK_MODIFICATIONS: &str = "minecraft:max_block_modifications";
const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

/// `FillBiomeCommand.fill` (`FillBiomeCommand.java:163-173`, `:110-161`).
pub(super) fn fill_biome(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    filtered: bool,
) -> Outcome {
    let (count, placements, resend) = plan_fill_biome(run, parse, cx.world, &cx.origin, filtered)?;
    cx.world
        .fill_command_biomes(&cx.origin.level, &placements, &resend, cx.events);
    Ok(count)
}

/// The quart-cell biome writes and the columns `resendBiomesForChunks` resends.
type BiomeWrite = (i32, Vec<(BlockPos, i32)>, Vec<(i32, i32)>);

fn plan_fill_biome(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    filtered: bool,
) -> Result<BiomeWrite, NbtSummaryValue> {
    let blocks = world.command_blocks(&at.level);
    let from = loaded_block_pos(parse, "from", blocks, at)?;
    let to = loaded_block_pos(parse, "to", blocks, at)?;
    let biome = biome_resource(parse);
    let biome_ids = world.registry_ids(BIOME_REGISTRY);
    let Some(target) = biome_ids
        .iter()
        .position(|id| id == biome)
        .and_then(|index| i32::try_from(index).ok())
    else {
        return Err(translatable(
            "argument.resource.not_found",
            vec![
                component_literal_nbt(biome),
                component_literal_nbt(BIOME_REGISTRY),
            ],
        ));
    };
    let filter = filtered.then(|| resource_or_tag(parse, "filter"));
    // `quantize` (`FillBiomeCommand.java:81-87`) then `fromCorners` (`:119-120`).
    let region = BoundingBox::from_corners(quantize(from), quantize(to));
    let volume =
        i64::from(region.x_span()) * i64::from(region.y_span()) * i64::from(region.z_span());
    let limit = world.integer_game_rule(RULE_MAX_BLOCK_MODIFICATIONS);
    if volume > i64::from(limit) {
        // `ERROR_VOLUME_TOO_LARGE` (`:37-39`, `:123-125`).
        return Err(translatable(
            "commands.fillbiome.toobig",
            vec![NbtSummaryValue::Int(limit), NbtSummaryValue::Long(volume)],
        ));
    }
    let min = region.min();
    let max = region.max();
    // `getChunk(..., FULL, false)` over the quantized box (`:129-138`).
    let mut resend = Vec::new();
    let mut chunk_z = min.z >> 4;
    while chunk_z <= max.z >> 4 {
        let mut chunk_x = min.x >> 4;
        while chunk_x <= max.x >> 4 {
            let sample = BlockPos {
                x: chunk_x << 4,
                y: min.y,
                z: chunk_z << 4,
            };
            if !blocks.has_chunk_at(sample) {
                return Err(translatable("argument.pos.unloaded", Vec::new()));
            }
            resend.push((chunk_x, chunk_z));
            chunk_x += 1;
        }
        chunk_z += 1;
    }
    let mut placements = Vec::new();
    let mut quart_z = min.z >> 2;
    while quart_z <= max.z >> 2 {
        let mut quart_y = min.y >> 2;
        while quart_y <= max.y >> 2 {
            let mut quart_x = min.x >> 2;
            while quart_x <= max.x >> 2 {
                let pos = BlockPos {
                    x: quart_x << 2,
                    y: quart_y << 2,
                    z: quart_z << 2,
                };
                // `makeResolver` (`:92-103`): count and replace only when the
                // quart's block corner sits inside the quantized region.
                if region.is_inside(pos) {
                    if let Some(current) = blocks.biome_id_at(pos.x, pos.y, pos.z) {
                        if let Some(name) = biome_ids.get(current as usize) {
                            let admitted = match filter {
                                None => true,
                                Some((false, id)) => name == id,
                                Some((true, tag)) => world.biome_in_tag(tag, name),
                            };
                            if admitted {
                                placements.push((pos, target));
                            }
                        }
                    }
                }
                quart_x += 1;
            }
            quart_y += 1;
        }
        quart_z += 1;
    }
    let count = i32::try_from(placements.len()).unwrap_or(i32::MAX);
    run.succeed(
        translatable(
            "commands.fillbiome.success.count",
            vec![
                NbtSummaryValue::Int(count),
                NbtSummaryValue::Int(min.x),
                NbtSummaryValue::Int(min.y),
                NbtSummaryValue::Int(min.z),
                NbtSummaryValue::Int(max.x),
                NbtSummaryValue::Int(max.y),
                NbtSummaryValue::Int(max.z),
            ],
        ),
        true,
    );
    Ok((count, placements, resend))
}

/// `QuartPos.toBlock(QuartPos.fromBlock)` (`FillBiomeCommand.java:81-83`).
fn quantize(pos: BlockPos) -> BlockPos {
    BlockPos {
        x: (pos.x >> 2) << 2,
        y: (pos.y >> 2) << 2,
        z: (pos.z >> 2) << 2,
    }
}

fn biome_resource(parse: &ParseResult) -> &str {
    match parse.argument("biome") {
        Some(ArgumentValue::Resource(id)) => id,
        other => unreachable!("biome is a resource argument, not {other:?}"),
    }
}

fn resource_or_tag<'a>(parse: &'a ParseResult, name: &str) -> (bool, &'a str) {
    match parse.argument(name) {
        Some(ArgumentValue::ResourceOrTag { tag, id }) => (*tag, id.as_str()),
        other => unreachable!("{name} is a resource-or-tag argument, not {other:?}"),
    }
}
