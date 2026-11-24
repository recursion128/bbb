//! `/fillbiome` over the loaded chunk's canonical biome cells.

use super::*;
use bbb_protocol::packets::{BlockPos, ChunkPos};

fn loaded_world() -> CommandTestWorld {
    CommandTestWorld::with_builtin_data(&[]).with_loaded_chunk(ChunkPos { x: 0, z: 0 })
}

#[test]
fn fillbiome_needs_gamemaster() {
    let refusal = refused(
        "fillbiome 0 64 0 0 64 0 minecraft:plains",
        CommandPermission::Moderator,
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn fillbiome_writes_the_quantized_quart_cell_and_counts_an_unchanged_cell() {
    let mut world = loaded_world();
    let same = ran("fillbiome 0 64 0 0 64 0 minecraft:plains", &mut world);
    assert_eq!(same.result, Some(1));
    let run = ran(
        "fillbiome 0 64 0 3 67 3 minecraft:nether_wastes",
        &mut world,
    );
    assert_eq!(run.result, Some(1));
    for pos in [
        BlockPos { x: 0, y: 64, z: 0 },
        BlockPos { x: 3, y: 67, z: 3 },
    ] {
        assert_eq!(
            world.biome_name(pos).as_deref(),
            Some("minecraft:nether_wastes")
        );
    }
    assert_eq!(
        world.biome_name(BlockPos { x: 4, y: 64, z: 0 }).as_deref(),
        Some("minecraft:plains")
    );
    assert_eq!(keys(&run), ["commands.fillbiome.success.count"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Int(64),
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Int(64),
            NbtSummaryValue::Int(0)
        ]
    );
    assert!(run.feedback[0].inform_admins);
}

#[test]
fn fillbiome_replace_keeps_cells_the_filter_rejects() {
    let mut world = loaded_world();
    world.setup("fillbiome 4 64 0 4 64 0 minecraft:nether_wastes");
    let run = ran(
        "fillbiome 0 64 0 7 64 0 minecraft:plains replace minecraft:nether_wastes",
        &mut world,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        world.biome_name(BlockPos { x: 4, y: 64, z: 0 }).as_deref(),
        Some("minecraft:plains")
    );
    world.setup("fillbiome 4 64 0 4 64 0 minecraft:nether_wastes");
    let tagged = ran(
        "fillbiome 0 64 0 7 64 0 minecraft:nether_wastes replace #minecraft:is_overworld",
        &mut world,
    );
    assert_eq!(tagged.result, Some(1));
    assert_eq!(
        world.biome_name(BlockPos { x: 0, y: 64, z: 0 }).as_deref(),
        Some("minecraft:nether_wastes")
    );
    assert_eq!(
        world.biome_name(BlockPos { x: 4, y: 64, z: 0 }).as_deref(),
        Some("minecraft:nether_wastes")
    );
}

#[test]
fn fillbiome_refuses_a_quantized_box_past_the_gamerule() {
    let world = loaded_world().with_rules(&[(
        "minecraft:max_block_modifications",
        GameRuleValue::Integer(1),
    )]);
    let run = ran("fillbiome 0 64 0 4 64 0 minecraft:plains", world);
    assert_eq!(keys(&run), ["commands.fillbiome.toobig"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(1), NbtSummaryValue::Long(5)]
    );
    assert_eq!(run.result, None);
}

#[test]
fn fillbiome_unloaded_from_is_argument_pos_unloaded() {
    let run = ran(
        "fillbiome 4096 64 4096 4096 64 4096 minecraft:plains",
        CommandTestWorld::with_builtin_data(&[]),
    );
    assert_eq!(keys(&run), ["argument.pos.unloaded"]);
    assert_eq!(run.result, None);
}

#[test]
fn fillbiome_unknown_biome_is_resource_not_found() {
    let run = ran(
        "fillbiome 0 64 0 0 64 0 minecraft:missing",
        CommandTestWorld::with_builtin_data(&[]),
    );
    assert_eq!(keys(&run), ["argument.resource.not_found", CURSOR_CONTEXT]);
    assert_eq!(run.result, None);
}
