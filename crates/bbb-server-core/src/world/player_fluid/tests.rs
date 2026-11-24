use super::*;
use crate::connection::ConnectionId;
use crate::entity::scan_body_fluid_interaction;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::packets::PlayServerbound;
use bbb_protocol::packets::{MovePlayer, Vec3d};
use bbb_server_types::food::FoodData;

mod target_inputs;
mod water_splash;

fn world() -> ServerWorld {
    world_with_mailbox().0
}

fn world_with_mailbox() -> (
    ServerWorld,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    {
        let mut position = player.position();
        position.y = 80.0;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.x = player.position().x.floor() + 0.5;
        player.set_position(position);
    }
    {
        let mut position = player.position();
        position.z = player.position().z.floor() + 0.5;
        player.set_position(position);
    }
    player.food = FoodData::from_saved(20, 0, 5.0, 0.0);
    (world, sender, receiver)
}

fn pool(world: &mut ServerWorld, depth: i32) {
    let player = world.levels.player_at(ConnectionId(1));
    let position = player.position();
    let dimension = player.dimension;
    let block =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:water").unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    let chunks = &mut world.level_mut_by_id(&dimension).chunks;
    for x in (position.x.floor() as i32 - 2)..=(position.x.floor() as i32 + 2) {
        for z in (position.z.floor() as i32 - 2)..=(position.z.floor() as i32 + 2) {
            for y in 80..80 + depth {
                assert!(chunks.set_block_state(x, y, z, state).is_some());
            }
        }
    }
}

#[test]
fn player_fluid_swimming_uses_previous_eye_contact_and_updates_pose() {
    let mut world = world();
    pool(&mut world, 2);
    world.levels[ConnectionId(1)].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, true);
    world.tick(Vec::new());
    let player = world.levels.player_at(ConnectionId(1));
    assert!(player.fluid_interaction.is_eye_in_fluid(FluidTag::Water));
    assert!(!player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING));
    world.tick(Vec::new());
    let player = world.levels.player_at(ConnectionId(1));
    assert!(player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING));
    assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_SWIMMING_ID);
    world.levels[ConnectionId(1)].abilities.flying = true;
    world.tick(Vec::new());
    let player = world.levels.player_at(ConnectionId(1));
    assert!(!player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING));
    assert_eq!(player.pose_id, VANILLA_ENTITY_POSE_STANDING_ID);
}

#[test]
fn player_fluid_movement_water_precedence_and_float_centimeters() {
    for (depth, swimming, stat, cm) in [
        (1, false, "minecraft:walk_on_water_one_cm", 300),
        (2, false, "minecraft:walk_under_water_one_cm", 500),
        (2, true, "minecraft:swim_one_cm", 500),
    ] {
        let mut world = world();
        pool(&mut world, depth);
        world.tick_player_fluid_contact(&mut Vec::new());
        let player = world.levels.player_mut(ConnectionId(1)).unwrap();
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, true);
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING, swimming);
        player.set_on_ground(true);
        player.known_movement = Vec3d {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        };
        super::super::player_packets::check_movement_statistics(
            player,
            &mut world.scoreboard,
            true,
        );
        assert_eq!(
            player.food.exhaustion_level(),
            0.01_f32 * cm as f32 * 0.01_f32
        );
        assert_eq!(
            world.player_stat_for_test(ConnectionId(1), bbb_server_types::stats::custom_stat(stat)),
            cm
        );
        assert_eq!(
            world.player_stat_for_test(
                ConnectionId(1),
                bbb_server_types::stats::custom_stat("minecraft:climb_one_cm")
            ),
            0
        );
    }
}

#[test]
fn player_fluid_accepted_packet_sprints_and_preserves_saved_exhaustion() {
    let (mut world, sender, receiver) = world_with_mailbox();
    world.tick(Vec::new());
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, true);
    let before = player.food.exhaustion_level();
    let position = Vec3d {
        x: player.position().x + 0.25,
        ..player.position()
    };
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        })
        .unwrap();
    world.tick(receiver.drain());
    let player = world.levels.player_at(ConnectionId(1));
    assert_eq!(player.position(), position);
    assert_eq!(
        player.food.exhaustion_level() - before,
        0.1_f32 * 25.0 * 0.01_f32
    );
    let expected = player.food;
    let save = world
        .save_request(
            1_786_000_000_000,
            1,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    assert_eq!(save.player_patches[0].created.food, expected);
}

#[test]
fn player_fluid_unloaded_spectator_preserves_contact_and_skips_food_and_regeneration() {
    let mut world = world();
    pool(&mut world, 2);
    world
        .admin
        .set_difficulty(bbb_protocol::difficulties::Difficulty::Peaceful);
    world.tick_player_fluid_contact(&mut Vec::new());
    let effect = crate::entity::ActiveMobEffect::new("minecraft:hunger", 100, 0).unwrap();
    assert_eq!(
        world.add_player_mob_effect(ConnectionId(1), effect.clone(), &mut Vec::new()),
        Some(true)
    );
    let before_stat = world.player_stat_for_test(
        ConnectionId(1),
        bbb_server_types::stats::custom_stat("minecraft:play_time"),
    );
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    let old = player.fluid_interaction;
    player.game_type = GameType::Spectator;
    {
        let mut position = player.position();
        position.x = 1_000_000.0;
        player.set_position(position);
    }
    player.food = FoodData::from_saved(10, 0, 3.0, 5.0);
    player.tick_count = 19;
    player.damage.health = 17.0;
    player.damage.invulnerable_time = 7;
    player.damage.hurt_time = 5;
    player.remaining_fire_ticks = 20;
    player.set_air_supply(-19);
    player.set_ticks_frozen(140);
    let tick = world.tick(Vec::new());
    assert!(tick.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::SetHealth(packet), .. } if packet.health == 17.0
    )));
    assert_eq!(
        world.player_active_effect_for_test(ConnectionId(1), "minecraft:hunger"),
        Some(effect)
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:play_time")
        ),
        before_stat
    );
    let player = world.levels.player_at(ConnectionId(1));
    assert!(!player.body_tick_enabled);
    assert_eq!(player.damage.invulnerable_time, 6);
    assert_eq!(player.damage.hurt_time, 5);
    assert_eq!(player.remaining_fire_ticks, 20);
    assert_eq!(player.air_supply, -19);
    assert_eq!(player.ticks_frozen, 140);
    assert_eq!(player.fluid_interaction, old);
    assert_eq!(player.food, FoodData::from_saved(10, 0, 3.0, 5.0));
    world.levels[ConnectionId(1)].game_type = GameType::Survival;
    world.tick_player_fluid_contact(&mut Vec::new());
    let player = world.levels.player_at(ConnectionId(1));
    assert!(player.body_tick_enabled);
    assert!(!player.fluid_interaction.is_in_fluid(FluidTag::Water));
}

#[test]
fn player_food_jump_cost_precedes_collision_refusal_and_uses_last_good_height() {
    use super::super::{
        player_movement::PlayerMoveProbe,
        player_packets::{apply_movement, MovementOutcome},
    };
    let mut world = world();
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.last_good = player.position();
    player.set_on_ground(true);
    let movement = MovePlayer {
        position: Some(Vec3d {
            y: player.position().y + 0.5,
            ..player.position()
        }),
        rotation: None,
        on_ground: false,
        horizontal_collision: false,
    };
    let probe = PlayerMoveProbe {
        jump_factor: Some(1.0),
        collides_with_new_geometry: true,
        ..Default::default()
    };
    assert!(matches!(
        apply_movement(player, &mut world.scoreboard, movement, probe),
        MovementOutcome::MovedWrongly(_)
    ));
    assert_eq!(player.food.exhaustion_level(), 0.05);
    player.last_good.y += 1.0;
    assert!(matches!(
        apply_movement(player, &mut world.scoreboard, movement, probe),
        MovementOutcome::MovedWrongly(_)
    ));
    assert_eq!(player.food.exhaustion_level(), 0.05);
}

#[test]
fn player_food_sprint_respects_abilities_and_passenger_gates() {
    let mut world = world();
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.known_movement = Vec3d {
        x: 0.005,
        y: 0.0,
        z: 0.0,
    };
    player.set_on_ground(true);
    player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING, true);
    player.abilities.invulnerable = true;
    super::super::player_packets::check_movement_statistics(player, &mut world.scoreboard, false);
    assert_eq!(player.food.exhaustion_level(), 0.0);
    player.abilities.invulnerable = false;
    player.cold.last_vehicle = Some(123);
    super::super::player_packets::check_movement_statistics(player, &mut world.scoreboard, false);
    assert_eq!(player.food.exhaustion_level(), 0.0);
    player.cold.last_vehicle = None;
    super::super::player_packets::check_movement_statistics(player, &mut world.scoreboard, false);
    assert_eq!(player.food.exhaustion_level(), 0.1_f32 * 0.01_f32);
}

#[test]
fn player_fluid_shared_scan_respects_boat_rim() {
    let mut world = world();
    pool(&mut world, 1);
    let player = world.levels.player_at(ConnectionId(1));
    let dimension = player.dimension;
    let position = player.position();
    let boat = crate::entity::spawn_boat(
        &mut world.level_mut_by_id(&dimension).entities,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        position,
        0.0,
    )
    .unwrap();
    let level = world.level_by_id(&dimension);
    let bounds = player_pose_bounding_box(
        position,
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DYING_ID,
        1.0,
    )
    .unwrap();
    let eye = Vec3d {
        y: position.y + 0.1,
        ..position
    };
    assert!(
        scan_body_fluid_interaction(bounds, eye, None, &level.chunks)
            .unwrap()
            .is_in_fluid(FluidTag::Water)
    );
    assert!(
        !scan_body_fluid_interaction(bounds, eye, level.entities.get(boat), &level.chunks)
            .unwrap()
            .is_in_fluid(FluidTag::Water)
    );
}
mod cross_level_targets;
