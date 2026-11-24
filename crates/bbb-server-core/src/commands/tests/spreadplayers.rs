//! `/spreadplayers` over live production flat terrain and entities.

use super::*;
use crate::tick_rate::java_fixed_decimals;
use bbb_protocol::packets::ChunkPos;

fn floor_world(players: &[NameAndId]) -> CommandTestWorld {
    floor_world_in(players, Java26BuiltinDimension::Overworld.into())
}

fn floor_world_in(players: &[NameAndId], dimension: Java26LevelKey) -> CommandTestWorld {
    let mut world = world(players).with_level(dimension);
    for x in -1..=1 {
        for z in -1..=1 {
            world = world.with_loaded_chunk(ChunkPos { x, z });
        }
    }
    world
}

fn player_pose(world: &CommandTestWorld, name: &str) -> ([f64; 3], [f32; 2]) {
    let player = world
        .world()
        .command_players()
        .find(|player| player.name == name)
        .unwrap();
    (player.position, player.rotation)
}

#[test]
fn under_below_min_y_is_refused_before_any_spread() {
    let players = [user("alice", 1)];
    let run = ran(
        "spreadplayers 0 0 1 8 under -100 false alice",
        world(&players),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.spreadplayers.failed.invalid.height"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(-100), NbtSummaryValue::Int(-64)]
    );
}

#[test]
fn empty_targets_report_the_entity_not_found_key() {
    let run = ran(
        "spreadplayers 0 0 1 8 false @e",
        CommandTestWorld::default(),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}

#[test]
fn no_safe_ground_below_max_height_fails_to_spread() {
    let players = [user("alice", 1)];
    let run = ran(
        "spreadplayers 4096 4096 1 8 under -64 false alice",
        world(&players),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.spreadplayers.failed.entities"]);
    assert_eq!(
        arguments(&run, 0)[..2],
        [
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Float(4096.5f32.to_bits())
        ]
    );
}

#[test]
fn one_player_lands_on_the_floor_and_keeps_rotation() {
    let mut world = floor_world(&[user("alice", 1)]).with_player_rotation("alice", [45.0, -10.0]);
    let run = ran("spreadplayers 0 0 0 8 false alice", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.spreadplayers.success.entities"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::Float(0.5f32.to_bits()),
            NbtSummaryValue::Float(0.5f32.to_bits()),
            text(&java_fixed_decimals(0.0, 2))
        ]
    );
    let (position, rotation) = player_pose(&world, "alice");
    assert_eq!(position[1], 64.0);
    assert!((-8.0..=9.0).contains(&position[0]));
    assert!((-8.0..=9.0).contains(&position[2]));
    assert_eq!(position[0].fract().abs(), 0.5);
    assert_eq!(position[2].fract().abs(), 0.5);
    assert_eq!(rotation, [45.0, -10.0]);
}

#[test]
fn a_stored_entity_is_teleported_on_its_own_level() {
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 1.0],
        crate::entity::EntityData::ExperienceOrb(crate::entity::ExperienceOrbData::new(3)),
    )];
    let dimension = level_key(Java26BuiltinDimension::Nether);
    let mut world =
        floor_world_in(&[], dimension.clone()).with_entities_in(dimension.clone(), &entities);
    let run = ran("spreadplayers 0 0 0 8 false @e", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.spreadplayers.success.entities"]);
    {
        let reads = world.world().entity_reads();
        let row = reads
            .stored_entities()
            .find(|row| row.entity.uuid() == Uuid::from_u128(9))
            .unwrap();
        assert_eq!(*row.dimension, dimension);
        let position = row.entity.position();
        assert_eq!(position.y, 64.0);
        assert_eq!(position.x.fract().abs(), 0.5);
        assert_eq!(position.z.fract().abs(), 0.5);
    }
}

#[test]
fn respect_teams_without_live_teams_shares_one_position() {
    let mut world = floor_world(&[user("alice", 1), user("bob", 2)]);
    let run = ran("spreadplayers 0 0 4 16 true @a", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.spreadplayers.success.teams"]);
    assert_eq!(player_pose(&world, "alice").0, player_pose(&world, "bob").0);
}

#[test]
fn under_uses_the_typed_max_height() {
    let mut world = floor_world(&[user("alice", 1)]);
    let run = ran("spreadplayers 0 0 0 8 under 80 false alice", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(player_pose(&world, "alice").0[1], 64.0);
}

#[test]
fn respect_teams_counts_the_distinct_teams_the_targets_are_on() {
    let mut world = floor_world(&[user("alice", 1), user("bob", 2), user("cara", 3)]);
    world.setup("team add red");
    world.setup("team add blue");
    world.setup("team join red alice");
    world.setup("team join blue bob");
    assert_eq!(
        ran("spreadplayers 0 0 4 16 true @a", &mut world).result,
        Some(3)
    );
    let alice = player_pose(&world, "alice").0;
    let bob = player_pose(&world, "bob").0;
    let cara = player_pose(&world, "cara").0;
    assert_ne!(alice, bob, "distinct teams must land at distinct positions");
    assert_ne!(
        alice, cara,
        "teamed and teamless targets must not share a slot"
    );
    assert_ne!(bob, cara, "distinct teams must land at distinct positions");
    world.setup("team join red bob");
    world.setup("team join red cara");
    let one_team = ran("spreadplayers 0 0 4 16 true @a", &mut world);
    assert_eq!(one_team.result, Some(1));
    assert_eq!(
        arguments(&one_team, 0)[3],
        text(&java_fixed_decimals(0.0, 2)),
        "single-team spread must report 0.00 distance rather than f64::MAX"
    );
    assert_eq!(player_pose(&world, "alice").0, player_pose(&world, "bob").0);
    assert_eq!(
        player_pose(&world, "alice").0,
        player_pose(&world, "cara").0
    );
    assert_eq!(
        ran("spreadplayers 0 0 4 16 false @a", &mut world).result,
        Some(3)
    );
}

#[test]
fn repeated_spreadplayers_in_same_tick_draws_fresh_positions() {
    let mut world = floor_world(&[user("alice", 1)]);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..8 {
        assert_eq!(
            ran("spreadplayers 0 0 0 12 false alice", &mut world).result,
            Some(1)
        );
        let pos = player_pose(&world, "alice").0;
        seen.insert((pos[0].to_bits(), pos[2].to_bits()));
    }
    assert!(
        seen.len() > 1,
        "repeated calls in the same tick must advance live level RNG: {seen:?}"
    );
}
