//! `Creaking`'s brain and freeze semantics on the real tick path: the
//! `NEAREST_PLAYERS` sensor, `Creaking.canMove`/`checkCanMove`
//! and the `CAN_MOVE`/`IS_ACTIVE` synched rows,
//! reached through a spawned protector and a session player's own movement
//! packets.

use super::*;

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_synched_data::{
    VANILLA_CREAKING_CAN_MOVE_DATA_ID, VANILLA_CREAKING_IS_ACTIVE_DATA_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

use super::helpers::{
    creaking_fixture_with_teams as fixture_with_teams, creaking_lay_corridor as lay_corridor,
    creaking_lay_floor, creaking_move_player as move_player, creaking_place_heart,
    creaking_protector as protector, creaking_spawn_protector as spawn_protector,
    creaking_synched_rows as synched_rows, level_sounds, uuid_scanning_players_immediately,
    CREAKING_HEART as HEART, CREAKING_LOOKING_WEST as LOOKING_WEST,
    CREAKING_PROTECTOR_POS as PROTECTOR_POSITION, CREAKING_WATCHER_POS as WATCHER_POSITION,
};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
/// The same rotation turned half around, so the watcher looks away from it.
const LOOKING_EAST: f32 = 270.0;

fn action_origins(world: &ServerWorld, entity_id: i32) -> Vec<Vec3d> {
    world
        .positioned_game_events_for_test(DIMENSION)
        .into_iter()
        .filter(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_ACTION
                && event.recorded.source_entity_id == Some(entity_id)
        })
        .map(|event| event.position)
        .collect()
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    fixture_with_teams(Vec::new())
}

fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    creaking_place_heart(world, at, "awake");
}

/// The floor the far protector stands on.
fn lay_far_corridor(world: &mut ServerWorld) {
    creaking_lay_floor(world, 42..=48, 66);
    let key = Java26LevelKey::from_builtin(DIMENSION);
    assert_eq!(
        world.block_state_by_key(
            &world.require_level_id(&key),
            BlockPos {
                x: FAR_PROTECTOR_POSITION.x as i32,
                y: 64,
                z: 4,
            }
        ),
        Some(block_state_id("minecraft:stone"))
    );
}

/// The watch: a protector whose watcher looks at it freezes on the tick the
/// `NEAREST_PLAYERS` scan reaches `checkCanMove`, and thaws the tick that
/// watcher looks away.
///
/// The protector's first tick builds the brain and runs both sensors
/// (`uuid_scanning_players_immediately`), so the memory exists at the end of
/// it and the next tick is the first whose `checkCanMove` reads a player. The
/// tracker publishes a synched row the tick after the entity writes it, which
/// is why the row lands one tick after the sound.
#[test]
fn a_watched_protector_freezes_and_thaws_when_the_watcher_looks_away() {
    let (sender, receiver, mut world) = fixture();
    lay_corridor(&mut world);
    place_heart(&mut world, HEART);
    let entity_id = spawn_protector(
        &mut world,
        PROTECTOR_POSITION,
        uuid_scanning_players_immediately(),
        HEART,
    );

    // Tick one for the protector: the watcher's own move packet lands first,
    // the brain is built, and both sensors scan with a zero delay.
    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    assert!(
        level_sounds(&first).is_empty(),
        "{:?}",
        level_sounds(&first)
    );
    assert!(protector(&world, entity_id).creaking_can_move());
    assert!(!protector(&world, entity_id).creaking_is_active());
    assert_eq!(
        synched_rows(&first, entity_id)
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![VANILLA_CREAKING_HOME_POS_DATA_ID],
        "only the heartbeat binding has moved off its define-time default"
    );

    // Tick two: `checkCanMove` reads the player, activates and freezes.
    let before_freeze = protector(&world, entity_id).position();
    let freeze_tick = world.tick(receiver.drain());
    assert_eq!(
        level_sounds(&freeze_tick),
        vec![
            "minecraft:entity.creaking.activate".to_string(),
            "minecraft:entity.creaking.freeze".to_string(),
        ],
        "`activate` plays ACTIVATE (`Creaking.java:494`), the changed row then FREEZE (`:232`)"
    );
    assert!(!protector(&world, entity_id).creaking_can_move());
    assert!(protector(&world, entity_id).creaking_is_active());
    assert_eq!(
        action_origins(&world, entity_id),
        [before_freeze, before_freeze]
    );
    assert!(
        synched_rows(&freeze_tick, entity_id).is_empty(),
        "a row written during the tick is published by the next one"
    );

    // Tick three: the two rows reach the client.
    let published = world.tick(receiver.drain());
    let rows = synched_rows(&published, entity_id);
    assert_eq!(
        rows.get(&VANILLA_CREAKING_CAN_MOVE_DATA_ID),
        Some(&EntityDataValueKind::Boolean(false)),
        "`entityData.set(CAN_MOVE, nowCanMove)` (`Creaking.java:236`)"
    );
    assert_eq!(
        rows.get(&VANILLA_CREAKING_IS_ACTIVE_DATA_ID),
        Some(&EntityDataValueKind::Boolean(true)),
        "`setIsActive(true)` inside `activate` (`Creaking.java:495`)"
    );

    // The protector holds its position while the watcher stares.
    let frozen_at = protector(&world, entity_id).position();
    for _ in 0..5 {
        let held = world.tick(receiver.drain());
        assert!(
            level_sounds(&held).is_empty(),
            "a frozen protector plays nothing: {:?}",
            level_sounds(&held)
        );
        assert_eq!(protector(&world, entity_id).position(), frozen_at);
    }

    // The watcher turns around: `checkCanMove` answers true again, which is
    // `:228-229`'s unfreeze sound.
    let before_thaw = protector(&world, entity_id).position();
    let thaw_tick = move_player(&sender, &receiver, &mut world, None, LOOKING_EAST);
    assert_eq!(
        level_sounds(&thaw_tick),
        vec!["minecraft:entity.creaking.unfreeze".to_string()]
    );
    assert!(protector(&world, entity_id).creaking_can_move());
    assert_eq!(action_origins(&world, entity_id)[2], before_thaw);
    assert!(
        protector(&world, entity_id).creaking_is_active(),
        "the watcher is still a potential target, so the protector stays active"
    );
    let row_tick = world.tick(receiver.drain());
    assert_eq!(
        synched_rows(&row_tick, entity_id).get(&VANILLA_CREAKING_CAN_MOVE_DATA_ID),
        Some(&EntityDataValueKind::Boolean(true))
    );
    // Check pursuit without pinning the navigation's first movement tick.
    let mut walked = Vec::new();
    for _ in 0..12 {
        world.tick(receiver.drain());
        let position = protector(&world, entity_id).position();
        assert_eq!(position.y, before_thaw.y);
        assert!((position.z - before_thaw.z).abs() < 0.5);
        walked.push(position.x);
    }
    assert!(
        walked[11] > before_thaw.x + 0.3,
        "the thawed route makes sustained progress: {walked:?}"
    );
    let before_deactivate = protector(&world, entity_id).position();
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Spectator,
        PlayerAbilities::for_game_type(GameType::Spectator),
    );
    world.tick(receiver.drain());
    assert!(!protector(&world, entity_id).creaking_is_active());
    assert_eq!(action_origins(&world, entity_id)[3], before_deactivate);
}

/// `EntityDataAccessor.HOME_POS`, the heartbeat binding the protector carries.
const VANILLA_CREAKING_HOME_POS_DATA_ID: u8 = 19;

/// A wall of `state_id` across the sight line between the two bodies: the
/// corridor's floor is at y=64 and both bodies stand at y=65, so every
/// gaze-height ray `isLookingAtMe` fires crosses the x=5 column in one of the
/// two cells above the floor.
fn wall_across_the_sight_line(world: &mut ServerWorld, state_id: i32) {
    for y in [65, 66] {
        for z in 3..=5 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x: 5, y, z },
                state_id,
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
}

/// The stare is `ClipContext.Block.VISUAL`: glass has an
/// empty visual shape, so a watcher behind it
/// freezes the protector exactly as one in the open does.
#[test]
fn a_watcher_staring_through_glass_still_freezes_the_protector() {
    let (sender, receiver, mut world) = fixture();
    lay_corridor(&mut world);
    wall_across_the_sight_line(&mut world, block_state_id("minecraft:glass"));
    place_heart(&mut world, HEART);
    let entity_id = spawn_protector(
        &mut world,
        PROTECTOR_POSITION,
        uuid_scanning_players_immediately(),
        HEART,
    );

    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    assert!(
        level_sounds(&first).is_empty(),
        "{:?}",
        level_sounds(&first)
    );
    let freeze_tick = world.tick(receiver.drain());
    assert_eq!(
        level_sounds(&freeze_tick),
        vec![
            "minecraft:entity.creaking.activate".to_string(),
            "minecraft:entity.creaking.freeze".to_string(),
        ],
        "`seeThroughTransparentBlocks` is true, so the glass does not hide the stare"
    );
    assert!(!protector(&world, entity_id).creaking_can_move());
    assert!(protector(&world, entity_id).creaking_is_active());
}

/// The contrast inside the same fixture: a stone wall's visual shape is the
/// collider, so the same stare stops on it and the protector stays unfrozen.
#[test]
fn a_watcher_behind_a_stone_wall_does_not_freeze_the_protector() {
    let (sender, receiver, mut world) = fixture();
    lay_corridor(&mut world);
    wall_across_the_sight_line(&mut world, block_state_id("minecraft:stone"));
    place_heart(&mut world, HEART);
    let entity_id = spawn_protector(
        &mut world,
        PROTECTOR_POSITION,
        uuid_scanning_players_immediately(),
        HEART,
    );

    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    assert!(
        level_sounds(&first).is_empty(),
        "{:?}",
        level_sounds(&first)
    );
    for _ in 0..4 {
        let tick = world.tick(receiver.drain());
        assert!(
            level_sounds(&tick).is_empty(),
            "an unseen watcher freezes nothing: {:?}",
            level_sounds(&tick)
        );
        assert!(protector(&world, entity_id).creaking_can_move());
    }
    assert!(
        !protector(&world, entity_id).creaking_is_active(),
        "the stare never reached the body"
    );
}

/// Where the unwatched protector stands: beyond the thirty-two block follow
/// range of a watcher at the world spawn. Its heart stands beside it, because
/// a protector more than thirty-four blocks from its heart is released and torn
/// down (`CreakingHeartBlockEntity.java:121`, the radius `:63`).
const FAR_HEART: BlockPos = BlockPos { x: 44, y: 65, z: 4 };
const FAR_PROTECTOR_POSITION: Vec3d = Vec3d {
    x: 44.5,
    y: 65.0,
    z: 4.5,
};

/// The contrast: with no player inside the follow range the `NEAREST_PLAYERS`
/// memory is empty, so `checkCanMove` answers true without writing a row or
/// playing a sound.
#[test]
fn a_protector_with_no_player_in_range_never_freezes() {
    let (_sender, receiver, mut world) = fixture();
    lay_corridor(&mut world);
    lay_far_corridor(&mut world);
    place_heart(&mut world, FAR_HEART);
    let entity_id = spawn_protector(
        &mut world,
        FAR_PROTECTOR_POSITION,
        uuid_scanning_players_immediately(),
        FAR_HEART,
    );

    for _ in 0..8 {
        let result = world.tick(receiver.drain());
        assert!(
            level_sounds(&result).is_empty(),
            "nothing freezes without a watcher: {:?}",
            level_sounds(&result)
        );
        let rows = synched_rows(&result, entity_id);
        assert_eq!(
            rows.get(&VANILLA_CREAKING_CAN_MOVE_DATA_ID),
            None,
            "`CAN_MOVE` never moves off its define-time `true` (`Creaking.java:121`)"
        );
        assert_eq!(rows.get(&VANILLA_CREAKING_IS_ACTIVE_DATA_ID), None);
    }
    assert!(protector(&world, entity_id).creaking_can_move());
    assert!(!protector(&world, entity_id).creaking_is_active());
    assert!(protector(&world, entity_id).is_heart_bound());
}
