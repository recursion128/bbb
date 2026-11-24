//! Native entity and session push outcomes.

use super::*;
use crate::connection::ConnectionId;
use crate::entity::level::ScoreboardTeams;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityData, GeneralLivingData};
use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use bbb_facts::scoreboard_team_options::TeamCollisionRule;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, Vec3d};

// Entity.java:1785-1800: the pair's impulse at a tenth of a block separation.
const ONE_TENTH_APART_IMPULSE: f64 = 0.015_811_388_536_449_91;
// LivingEntity.java:3171: damage from one cramming charge.
const CRAMMING_DAMAGE: f32 = 6.0;
const CRAMMING_STEP_LIMIT: usize = 64;
type Side = Option<(u8, TeamCollisionRule)>;
const SESSION_NAME: &str = "session";

fn command(
    native: &mut NativeLevel,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    text: &str,
) {
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: text.into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: text.into(),
            },
        })
        .unwrap();
    let result = native.world.tick(receiver.drain());
    assert!(result.events.iter().any(
        |event| matches!(event, crate::world::WorldEvent::Admin(outcome) if matches!(&outcome.result, Ok(crate::admin::AdminResult::Command(report)) if report.result.is_some()))
    ), "{text}: {:?}", result.events);
}

fn run_push_step(store: &mut EntityStore, id: i32) -> Option<()> {
    run_push_step_with(store, &mut NativeLevel::new(), id)
}
fn run_push_step_with(store: &mut EntityStore, native: &mut NativeLevel, id: i32) -> Option<()> {
    let mut body = store.detach(id).unwrap();
    let result = native.with_level(|level| push_entities(&mut body, store, level));
    store.attach(body);
    result
}
fn charged(store: &mut EntityStore, native: &mut NativeLevel, id: i32) -> bool {
    let before = health(store, id);
    for _ in 0..CRAMMING_STEP_LIMIT {
        assert_eq!(run_push_step_with(store, native, id), Some(()));
        if health(store, id) != before {
            return true;
        }
    }
    false
}

fn session_level(
    x: f64,
    mode: GameType,
) -> (NativeLevel, WorldCommandSender, WorldCommandReceiver) {
    session_on_level(NativeLevel::new(), x, mode)
}

fn session_on_level(
    mut native: NativeLevel,
    x: f64,
    mode: GameType,
) -> (NativeLevel, WorldCommandSender, WorldCommandReceiver) {
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut native.world,
        1,
        SESSION_NAME,
        5_551_024,
    );
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        mode,
        bbb_protocol::packets::PlayerAbilities::for_game_type(mode),
    );
    native
        .world
        .set_player_position_for_test(ConnectionId(1), position(x));
    native.world.tick(receiver.drain());
    (native, sender, receiver)
}
fn spawn_world_mob(native: &mut NativeLevel, uuid: u128, kind: i32, pos: Vec3d) -> i32 {
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    native
        .world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid::Uuid::from_u128(uuid),
            kind,
            pos,
            EntityData::GeneralLiving(data),
        )
        .unwrap()
}
fn world_delta(native: &NativeLevel, id: i32) -> Vec3d {
    native
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .delta_movement()
}
fn player_delta(native: &NativeLevel) -> Vec3d {
    native.world.nearby_players_for_test(DIMENSION)[0].delta_movement
}
fn world_crowd(native: &mut NativeLevel, count: u128) -> Vec<i32> {
    (0..count)
        .map(|i| spawn_world_mob(native, i + 2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position(4.5)))
        .collect()
}

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 8.5 }
}

fn store_of(bodies: &[(u128, i32, f64)]) -> (EntityStore, Vec<i32>) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let ids = bodies
        .iter()
        .map(|(uuid, entity_type_id, x)| {
            store
                .restore(
                    uuid::Uuid::from_u128(*uuid),
                    *entity_type_id,
                    position(*x),
                    EntityData::GeneralLiving(
                        GeneralLivingData::new(*entity_type_id).expect("both types are living"),
                    ),
                )
                .expect("the body restores")
        })
        .collect();
    (store, ids)
}

fn delta_x(store: &EntityStore, entity_id: i32) -> f64 {
    store
        .get(entity_id)
        .expect("the body is stored")
        .delta_movement()
        .x
}

fn stacked_crowd(pusher_type_id: i32, crowd: u128) -> (EntityStore, i32, Vec<i32>) {
    let mut bodies = vec![(1, pusher_type_id, 4.5)];
    for index in 0..crowd {
        bodies.push((2 + index, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5));
    }
    let (store, ids) = store_of(&bodies);
    (store, ids[0], ids[1..].to_vec())
}

fn health(store: &EntityStore, entity_id: i32) -> f32 {
    store
        .get(entity_id)
        .expect("the body is stored")
        .living()
        .expect("both fixture types are living")
        .health()
}

fn collision_cases() -> Vec<(Side, Side, bool)> {
    use TeamCollisionRule::{Always, Never, PushOtherTeams, PushOwnTeam};
    vec![
        (None, None, true),
        (Some((0, Always)), Some((0, Always)), true),
        (Some((0, Never)), Some((0, Never)), false),
        (Some((0, Never)), None, false),
        (None, Some((0, Never)), false),
        (Some((0, PushOwnTeam)), Some((0, PushOwnTeam)), false),
        (Some((0, PushOwnTeam)), Some((1, Always)), true),
        (None, Some((0, PushOwnTeam)), true),
        (Some((0, PushOtherTeams)), Some((0, PushOtherTeams)), true),
        (Some((0, PushOtherTeams)), Some((1, Always)), false),
        (None, Some((0, PushOtherTeams)), false),
        (Some((0, Always)), Some((1, PushOtherTeams)), false),
    ]
}

fn teams_for(pusher: (&str, Side), candidate: (&str, Side)) -> ScoreboardTeams {
    let mut teams = ScoreboardTeams::default();
    for index in 0..2 {
        let members: Vec<(String, TeamCollisionRule)> = [pusher, candidate]
            .into_iter()
            .filter_map(|(name, side)| {
                side.filter(|(team, _)| *team == index)
                    .map(|(_, rule)| (name.to_string(), rule))
            })
            .collect();
        if let Some((_, rule)) = members.first() {
            let names: Vec<bbb_protocol::JavaString> =
                members.iter().map(|(name, _)| name.into()).collect();
            teams.add_team(&names, *rule);
        }
    }
    teams
}

#[test]
fn two_pushable_bodies_apart_by_a_tenth_push_each_other() {
    let (mut store, ids) = store_of(&[
        (1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5),
        (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.6),
    ]);
    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    assert_eq!(delta_x(&store, ids[0]), -ONE_TENTH_APART_IMPULSE);
    assert_eq!(delta_x(&store, ids[1]), ONE_TENTH_APART_IMPULSE);
}

#[test]
fn a_frozen_creaking_neither_moves_nor_pushes_the_body_in_it() {
    let (mut store, ids) = store_of(&[
        (1, VANILLA_ENTITY_TYPE_CREAKING_ID, 4.5),
        (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.6),
    ]);
    assert!(store
        .get_mut(ids[0])
        .expect("the protector is stored")
        .set_creaking_can_move(false));

    assert_eq!(run_push_step(&mut store, ids[1]), Some(()));
    assert_eq!(delta_x(&store, ids[0]), 0.0);
    assert_eq!(delta_x(&store, ids[1]), 0.0);

    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    assert_eq!(delta_x(&store, ids[0]), 0.0);
    assert_eq!(delta_x(&store, ids[1]), ONE_TENTH_APART_IMPULSE);
}

#[test]
fn a_pair_inside_the_push_epsilon_is_left_alone() {
    let (mut store, ids) = store_of(&[
        (1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5),
        (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.505),
    ]);
    assert!(store
        .get(ids[0])
        .expect("the body is stored")
        .bounding_box()
        .intersects(
            store
                .get(ids[1])
                .expect("the body is stored")
                .bounding_box()
        ));
    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    assert_eq!(delta_x(&store, ids[0]), 0.0);
    assert_eq!(delta_x(&store, ids[1]), 0.0);
}

#[test]
fn a_body_outside_the_pushers_box_is_never_pushed() {
    let (mut store, ids) = store_of(&[
        (1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5),
        (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 5.5),
    ]);
    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    assert_eq!(delta_x(&store, ids[0]), 0.0);
    assert_eq!(delta_x(&store, ids[1]), 0.0);
}

#[test]
fn a_crowd_above_the_threshold_charges_the_pusher_the_vanilla_six() {
    let (mut store, pusher, crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREEPER_ID, 24);
    assert_eq!(crowd.len(), 24);
    let mut native = NativeLevel::new();
    let before = health(&store, pusher);
    assert!(charged(&mut store, &mut native, pusher));
    assert_eq!(
        before - health(&store, pusher),
        CRAMMING_DAMAGE,
        "the charge is `hurtServer(serverLevel, damageSources().cramming(), 6.0F)`"
    );
}

#[test]
fn a_crowd_at_the_threshold_takes_nothing() {
    let (mut store, pusher, crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREEPER_ID, 23);
    assert_eq!(crowd.len(), 23);
    let mut native = NativeLevel::new();
    assert!(!charged(&mut store, &mut native, pusher));
    assert_eq!(health(&store, pusher), 20.0);
}

#[test]
fn a_crowd_never_counts_a_rider() {
    let (mut store, pusher, crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREEPER_ID, 24);
    assert!(store.link_existing_passenger_forced(crowd[0], crowd[1]));
    let mut native = NativeLevel::new();
    assert!(!charged(&mut store, &mut native, pusher));
    assert_eq!(health(&store, pusher), 20.0);
}

#[test]
fn a_rule_of_zero_switches_the_whole_arm_off() {
    let (mut store, pusher, _crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREEPER_ID, 24);
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    command(
        &mut native,
        &sender,
        &receiver,
        "gamerule minecraft:max_entity_cramming 0",
    );
    assert!(!charged(&mut store, &mut native, pusher));
    assert_eq!(health(&store, pusher), 20.0);
}

#[test]
fn a_bound_protector_refuses_the_cramming_charge_an_unbound_one_takes() {
    let (mut store, protector, _crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREAKING_ID, 24);
    let mut native = NativeLevel::new();
    assert!(store
        .get_mut(protector)
        .expect("the protector is stored")
        .set_creaking_transient(BlockPos { x: 0, y: 64, z: 0 }));
    assert!(!charged(&mut store, &mut native, protector));
    assert_eq!(
        health(&store, protector),
        1.0,
        "a bound protector refuses damage without a direct cause"
    );

    let (mut store, protector, _crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREAKING_ID, 24);
    assert!(charged(&mut store, &mut native, protector));
    assert_eq!(health(&store, protector), 0.0);
}

#[test]
fn the_crowd_count_drops_every_passenger_and_keeps_their_vehicle() {
    let (mut store, pusher, crowd) = stacked_crowd(VANILLA_ENTITY_TYPE_CREEPER_ID, 25);
    assert!(store.link_existing_passenger_forced(crowd[0], crowd[1]));
    assert!(store.link_existing_passenger_forced(crowd[0], crowd[2]));
    let mut native = NativeLevel::new();
    assert!(!charged(&mut store, &mut native, pusher));
}

#[test]
fn every_collision_rule_filters_a_stored_pusher_and_a_stored_candidate() {
    for (pusher, candidate, pushed) in collision_cases() {
        let (mut store, ids) = store_of(&[
            (1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5),
            (2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.6),
        ]);
        let mut native = NativeLevel::new();
        native.set_scoreboard_teams(teams_for(
            (&uuid::Uuid::from_u128(1).to_string(), pusher),
            (&uuid::Uuid::from_u128(2).to_string(), candidate),
        ));
        assert_eq!(
            run_push_step_with(&mut store, &mut native, ids[0]),
            Some(())
        );
        let expected = if pushed { ONE_TENTH_APART_IMPULSE } else { 0.0 };
        assert_eq!(
            (delta_x(&store, ids[0]), delta_x(&store, ids[1])),
            (-expected, expected),
            "{pusher:?} pushing {candidate:?}"
        );
    }
}

#[test]
fn a_session_player_and_a_stored_body_push_each_other_only_inside_the_box() {
    for (x, mode, pushed) in [
        (4.6, GameType::Survival, true),
        (7.5, GameType::Survival, false),
        (4.6, GameType::Spectator, false),
    ] {
        let (mut native, _, receiver) = session_level(4.5, mode);
        let body = spawn_world_mob(&mut native, 2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position(x));
        native.world.tick(receiver.drain());
        if pushed {
            assert!(world_delta(&native, body).x > 0.0);
            assert!(player_delta(&native).x < 0.0);
        } else {
            assert_eq!(world_delta(&native, body).x, 0.0);
            assert_eq!(player_delta(&native).x, 0.0);
        }
    }
}

#[test]
fn a_parrot_never_pushes_a_player_itself() {
    let mut deltas = Vec::new();
    for kind in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID,
    ] {
        let (mut native, _, receiver) = session_level(4.5, GameType::Survival);
        spawn_world_mob(&mut native, 2, kind, position(4.6));
        native.world.tick(receiver.drain());
        deltas.push(player_delta(&native).x);
    }
    assert!(deltas[0] < deltas[1] && deltas[1] < 0.0, "{deltas:?}");
}

#[test]
fn an_iron_golem_may_target_a_pushed_monster_but_never_a_creeper() {
    let mut targeted = [false; 2];
    for (index, kind) in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
    ]
    .into_iter()
    .enumerate()
    {
        let mut native = NativeLevel::new();
        let golem = spawn_world_mob(
            &mut native,
            1,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            position(4.5),
        );
        // Coincident bodies take no impulse, so the pair overlaps every tick.
        let other = spawn_world_mob(&mut native, 2, kind, position(4.5));
        let other_uuid = native
            .world
            .stored_entity_for_test(DIMENSION, other)
            .unwrap()
            .uuid();
        for _ in 0..200 {
            native.world.tick(Vec::new());
            if native.world.mob_target_for_test(DIMENSION, golem) == Some(other_uuid) {
                targeted[index] = true;
                break;
            }
        }
    }
    assert_eq!(targeted, [true, false]);
}

#[test]
fn a_frozen_protector_takes_no_player_impulse_and_is_not_pushable_by_the_player() {
    let (mut native, sender, receiver) = session_level(4.6, GameType::Survival);
    let body = spawn_world_mob(
        &mut native,
        2,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        position(4.5),
    );
    let EntityData::GeneralLiving(data) = &mut native
        .world
        .stored_entity_mut_for_test(DIMENSION, body)
        .unwrap()
        .data
    else {
        panic!("living");
    };
    data.set_no_ai(false);
    for _ in 0..64 {
        let actor = native
            .world
            .stored_entity_for_test(DIMENSION, body)
            .unwrap();
        let pos = actor.position();
        let pitch = -((f64::from(actor.eye_height()) - 1.62) / 0.1)
            .atan()
            .to_degrees() as f32;
        sender
            .try_send(crate::world::WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::MovePlayer(
                    bbb_protocol::packets::MovePlayer {
                        position: Some(Vec3d {
                            x: pos.x + 0.1,
                            ..pos
                        }),
                        rotation: Some(bbb_protocol::packets::PlayerRotation {
                            y_rot: 90.0,
                            x_rot: pitch,
                        }),
                        on_ground: true,
                        horizontal_collision: false,
                    },
                ),
            })
            .unwrap();
        native.world.tick(receiver.drain());
        if !native
            .world
            .stored_entity_for_test(DIMENSION, body)
            .unwrap()
            .creaking_can_move()
        {
            break;
        }
    }
    assert!(!native
        .world
        .stored_entity_for_test(DIMENSION, body)
        .unwrap()
        .creaking_can_move());
    assert_eq!(world_delta(&native, body).x, 0.0);
    assert!(player_delta(&native).x > 0.0);
}

fn saved_rider_level() -> NativeLevel {
    let bodies = (2..=25)
        .map(|uuid| (uuid, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5))
        .collect::<Vec<_>>();
    let (mut store, ids) = store_of(&bodies);
    for id in &ids {
        let EntityData::GeneralLiving(data) = &mut store.get_mut(*id).unwrap().data else {
            panic!("living");
        };
        data.set_no_ai(true);
    }
    assert!(store.link_existing_passenger_forced(ids[0], ids[1]));
    let data = crate::entity::write_entity_chunk(&store, (0, 0)).unwrap();
    let root =
        std::env::temp_dir().join(format!("bbb-native-rider-cramming-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "Native rider cramming");
    let mut candidate = storage.load_candidate().unwrap();
    candidate
        .dimensions
        .get_mut(&DIMENSION.into())
        .unwrap()
        .entity_chunks
        .insert(
            (0, 0),
            bbb_server_persistence::Java26LoadedEntityChunk { timestamp: 0, data },
        );
    candidate.chunk_tickets.insert(
        DIMENSION.into(),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    let world = crate::world::ServerWorld::restore(
        crate::world::ServerWorldConfig {
            datapacks: Some(crate::registry_sync::builtin_datapack_startup_shared()),
            ..Default::default()
        },
        &candidate,
    )
    .unwrap();
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        world
            .stored_entity_ids_for_test(DIMENSION)
            .into_iter()
            .filter(|&id| {
                world
                    .stored_entity_for_test(DIMENSION, id)
                    .is_some_and(|entity| {
                        entity.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_ID
                            && matches!(entity.data(), EntityData::GeneralLiving(_))
                    })
            })
            .count(),
        24,
    );
    let vehicle = world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid::Uuid::from_u128(2))
        .unwrap();
    let rider = world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid::Uuid::from_u128(3))
        .unwrap();
    assert_eq!(
        world.stored_passengers_for_test(DIMENSION, vehicle),
        &[rider]
    );
    NativeLevel { world }
}

#[test]
fn actual_session_cramming_respects_the_rule_and_riders() {
    for (rule, riders, charged) in [(24, false, true), (40, false, false), (24, true, false)] {
        let (mut native, sender, receiver) = if riders {
            session_on_level(saved_rider_level(), 4.5, GameType::Survival)
        } else {
            session_level(4.5, GameType::Survival)
        };
        command(
            &mut native,
            &sender,
            &receiver,
            &format!("gamerule minecraft:max_entity_cramming {rule}"),
        );
        if !riders {
            world_crowd(&mut native, 24);
        }
        native
            .world
            .set_player_food_for_test(ConnectionId(1), 0, 0.0);
        let before = native.world.player_health_for_test(ConnectionId(1));
        for _ in 0..CRAMMING_STEP_LIMIT {
            native.world.tick(receiver.drain());
            if native.world.player_health_for_test(ConnectionId(1)) != before {
                break;
            }
        }
        let after = native.world.player_health_for_test(ConnectionId(1));
        if charged {
            assert_eq!(before - after, CRAMMING_DAMAGE);
        } else {
            assert_eq!(before, after, "rule={rule}, riders={riders}");
        }
    }
}

#[test]
fn the_actual_spectator_refuses_cramming_damage() {
    let (mut native, _, receiver) = session_level(4.5, GameType::Spectator);
    world_crowd(&mut native, 24);
    let before = native.world.player_health_for_test(ConnectionId(1));
    for _ in 0..CRAMMING_STEP_LIMIT {
        native.world.tick(receiver.drain());
    }
    assert_eq!(native.world.player_health_for_test(ConnectionId(1)), before);
}

#[test]
fn a_real_session_counts_towards_the_stored_pushers_crowd() {
    let (mut native, _, receiver) = session_level(4.5, GameType::Survival);
    let pusher = spawn_world_mob(
        &mut native,
        1,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        position(4.5),
    );
    world_crowd(&mut native, 23);
    let before = native
        .world
        .stored_entity_for_test(DIMENSION, pusher)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut after = before;
    for _ in 0..CRAMMING_STEP_LIMIT {
        native.world.tick(receiver.drain());
        after = native
            .world
            .stored_entity_for_test(DIMENSION, pusher)
            .unwrap()
            .living()
            .unwrap()
            .health();
        if after != before {
            break;
        }
    }
    assert!(after < before);
}

mod player_pose;
mod sleep_acceptance;
mod source_query_acceptance;

fn spawn_world_vehicle(native: &mut NativeLevel, uuid: u128, kind: i32, pos: Vec3d) -> i32 {
    let data = match kind {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID => {
            EntityData::Minecart(crate::entity::minecart::MinecartData::new())
        }
        _ => EntityData::Boat(crate::entity::BoatData::new()),
    };
    native
        .world
        .restore_stored_entity_for_test(DIMENSION, uuid::Uuid::from_u128(uuid), kind, pos, data)
        .unwrap()
}

fn along_z(z: f64) -> Vec3d {
    Vec3d { x: 4.5, y: 64.0, z }
}

/// A north-south strip of `block` on stone, the shape a default rail takes.
fn strip(native: &mut NativeLevel, block: &str) {
    for z in 0..14 {
        for (y, block) in [(63, "minecraft:stone"), (64, block)] {
            native.world.set_block(
                DIMENSION,
                BlockPos { x: 4, y, z },
                state(block),
                crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
            );
        }
    }
}

#[test]
fn a_moving_minecart_seats_the_villager_it_reaches_and_pushes_a_cart_ahead() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    };
    let mut native = NativeLevel::new();
    strip(&mut native, "minecraft:rail");
    let cart = spawn_world_vehicle(
        &mut native,
        1,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        along_z(1.5),
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, cart)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.3,
        });
    let villager = spawn_world_mob(
        &mut native,
        2,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        along_z(2.6),
    );
    let parked = spawn_world_vehicle(
        &mut native,
        3,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        along_z(7.5),
    );
    let mut parked_moved = false;
    for _ in 0..40 {
        native.world.tick(Vec::new());
        parked_moved |= world_delta(&native, parked).z > 0.0;
    }
    assert_eq!(
        native.world.stored_passengers_for_test(DIMENSION, cart),
        &[villager]
    );
    assert!(
        parked_moved,
        "the riding cart hands its motion to the cart ahead"
    );
}

#[test]
fn a_moving_minecart_never_seats_a_warden() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
    };
    let mut native = NativeLevel::new();
    strip(&mut native, "minecraft:rail");
    let cart = spawn_world_vehicle(
        &mut native,
        1,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        along_z(1.5),
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, cart)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.3,
        });
    spawn_world_mob(&mut native, 2, VANILLA_ENTITY_TYPE_WARDEN_ID, along_z(2.6));
    for _ in 0..20 {
        native.world.tick(Vec::new());
    }
    assert!(native
        .world
        .stored_passengers_for_test(DIMENSION, cart)
        .is_empty());
}

#[test]
fn a_magma_cube_bumping_an_iron_golem_hurts_it() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    };
    let mut native = NativeLevel::new();
    strip(&mut native, "minecraft:air");
    let golem = spawn_world_mob(
        &mut native,
        1,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        along_z(4.5),
    );
    native
        .world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid::Uuid::from_u128(2),
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            along_z(4.9),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID).unwrap(),
            ),
        )
        .unwrap();
    let health = |native: &NativeLevel| {
        native
            .world
            .stored_entity_for_test(DIMENSION, golem)
            .unwrap()
            .living()
            .unwrap()
            .health()
    };
    let before = health(&native);
    for _ in 0..5 {
        native.world.tick(Vec::new());
    }
    assert!(health(&native) < before);
}

#[test]
fn a_cow_inside_a_boats_reach_is_seated() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID};
    let mut native = NativeLevel::new();
    strip(&mut native, "minecraft:air");
    let boat = spawn_world_vehicle(
        &mut native,
        1,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        along_z(4.5),
    );
    let cow = spawn_world_mob(&mut native, 2, VANILLA_ENTITY_TYPE_COW_ID, along_z(5.4));
    native.world.tick(Vec::new());
    assert_eq!(
        native.world.stored_passengers_for_test(DIMENSION, boat),
        &[cow]
    );
}

#[test]
fn a_mob_walking_into_a_minecart_pushes_it_by_the_carts_own_rule() {
    let (mut store, ids) = store_of(&[(1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5)]);
    let cart = store
        .restore(
            uuid::Uuid::from_u128(2),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            position(4.6),
            EntityData::Minecart(crate::entity::minecart::MinecartData::new()),
        )
        .unwrap();
    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    // AbstractMinecart.java:500-520 at a tenth of a block.
    let impulse = f64::from(0.1_f32) * 0.5;
    assert!((delta_x(&store, cart) - impulse).abs() < 1.0e-9);
    assert!((delta_x(&store, ids[0]) + impulse / 4.0).abs() < 1.0e-9);
}

#[test]
fn an_armor_stand_is_never_pushed_and_only_pushes_a_rideable_cart_on_it() {
    // ArmorStand.java:168-183.
    let stand_at = |store: &mut EntityStore| {
        store
            .restore(
                uuid::Uuid::from_u128(2),
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                position(4.6),
                EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
            )
            .unwrap()
    };
    let (mut store, ids) = store_of(&[(1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 4.5)]);
    let stand = stand_at(&mut store);
    assert_eq!(run_push_step(&mut store, ids[0]), Some(()));
    assert_eq!(
        (delta_x(&store, stand), delta_x(&store, ids[0])),
        (0.0, 0.0)
    );

    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let stand = stand_at(&mut store);
    let cart = store
        .restore(
            uuid::Uuid::from_u128(3),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            position(4.9),
            EntityData::Minecart(crate::entity::minecart::MinecartData::new()),
        )
        .unwrap();
    let mut body = store.detach(stand).unwrap();
    armor_stand_push_minecarts(&mut body, &mut store);
    store.attach(body);
    // `AbstractMinecart.java:519-520`: the cart's own rule moves both.
    assert!(delta_x(&store, cart) > 0.0 && delta_x(&store, stand) < 0.0);
}

#[test]
fn a_mob_above_a_boats_floor_never_pushes_it() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    };
    // AbstractBoat.java:175-183: only a body whose floor is at or below the
    // boat's floor pushes it.
    for (golem_y, pushed) in [(64.0, true), (64.3, false)] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let golem = store
            .restore(
                uuid::Uuid::from_u128(1),
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                Vec3d {
                    y: golem_y,
                    ..position(4.5)
                },
                EntityData::GeneralLiving(
                    GeneralLivingData::new(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID).unwrap(),
                ),
            )
            .unwrap();
        let boat = store
            .restore(
                uuid::Uuid::from_u128(2),
                VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                position(4.6),
                EntityData::Boat(crate::entity::BoatData::new()),
            )
            .unwrap();
        assert_eq!(run_push_step(&mut store, golem), Some(()));
        assert_eq!(delta_x(&store, boat) > 0.0, pushed, "golem at y={golem_y}");
    }
}
