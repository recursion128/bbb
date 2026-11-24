//! `/place feature` over the native retained datapack and live columns.

use super::*;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::{BlockPos, ChunkPos};

fn place_world() -> CommandTestWorld {
    let mut world = CommandTestWorld::with_builtin_data(&[]).with_position([0.0, 80.0, 0.0]);
    for x in -1..=1 {
        for z in -1..=1 {
            world = world.with_loaded_chunk(ChunkPos { x, z });
        }
    }
    world
}

#[test]
fn place_needs_gamemaster() {
    let refusal = refused(
        "place feature minecraft:end_island",
        CommandPermission::Moderator,
    );
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn place_feature_at_source_and_explicit_position_writes_end_stone() {
    let mut world = place_world();
    let run = ran("place feature minecraft:end_island", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.place.feature.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(
        arguments(&run, 0),
        vec![
            text("minecraft:end_island"),
            NbtSummaryValue::Int(0),
            NbtSummaryValue::Int(80),
            NbtSummaryValue::Int(0)
        ]
    );
    let origin = BlockPos { x: 0, y: 80, z: 0 };
    assert_eq!(
        vanilla_block_state_identity(world.block_state_at(origin).unwrap())
            .unwrap()
            .name,
        "minecraft:end_stone"
    );
    let run = ran("place feature minecraft:end_island 4 80 4", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(
        vanilla_block_state_identity(
            world
                .block_state_at(BlockPos { x: 4, y: 80, z: 4 })
                .unwrap()
        )
        .unwrap()
        .name,
        "minecraft:end_stone"
    );
}

#[test]
fn place_feature_failure_reports_the_live_attempt() {
    let mut world = place_world();
    let run = ran("place feature minecraft:blue_ice", &mut world);
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.place.feature.failed"]);
    assert_eq!(run.feedback[0].kind, CommandFeedbackKind::Failure);
}

#[test]
fn place_unknown_feature_is_invalid() {
    let run = ran(
        "place feature minecraft:missing",
        CommandTestWorld::with_builtin_data(&[]),
    );
    assert_eq!(keys(&run), ["commands.place.feature.invalid"]);
    assert_eq!(run.result, None);
}

#[test]
fn place_source_arm_needs_a_loaded_neighborhood() {
    let world = CommandTestWorld::with_builtin_data(&[]).with_position([4096.0, 80.0, 4096.0]);
    let run = ran("place feature minecraft:end_island", world);
    assert_eq!(keys(&run), ["argument.pos.unloaded"]);
}
