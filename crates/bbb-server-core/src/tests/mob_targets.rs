//! `Mob.target` on live ticks: target goals acquiring, keeping and dropping
//! stored mobs and session players, and `NeutralMob` anger over a restart.

use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn spawn(world: &mut ServerWorld, entity_type_id: i32, position: Vec3d) -> (i32, Uuid) {
    world.spawn_stored_entity_for_test(
        DIMENSION,
        entity_type_id,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(entity_type_id).unwrap()),
    )
}

fn floor_top() -> f64 {
    f64::from(
        bbb_server_types::fixture::FlatFixture::new()
            .unwrap()
            .floor_y()
            + 1,
    )
}

fn install_forced_ticket(candidate: &mut Java26WorldCandidate) {
    candidate.chunk_tickets.insert(
        level_key(DIMENSION),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
}

fn install_floor(world: &mut ServerWorld) {
    let stone = block_state_id("minecraft:stone");
    for x in 0..=15 {
        for z in 0..=15 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                stone,
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
}

fn target_of(world: &ServerWorld, entity_id: i32) -> Option<Uuid> {
    world.mob_target_for_test(DIMENSION, entity_id)
}

/// `anger_end_time` and `angry_at` as the entity's record carries them.
fn saved_anger(world: &ServerWorld, entity_id: i32) -> (Option<i64>, Option<Uuid>) {
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, entity_id)
    else {
        panic!("a stored mob answers with a record");
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.value.clone())
    };
    let end_time = match field("anger_end_time") {
        Some(NbtSummaryValue::Long(end_time)) => Some(end_time),
        _ => None,
    };
    let angry_at = match field("angry_at") {
        Some(NbtSummaryValue::IntArray(words)) => match words[..] {
            [a, b, c, d] => Some(Uuid::from_u128(
                (u128::from(a as u32) << 96)
                    | (u128::from(b as u32) << 64)
                    | (u128::from(c as u32) << 32)
                    | u128::from(d as u32),
            )),
            _ => None,
        },
        _ => None,
    };
    (end_time, angry_at)
}

#[track_caller]
fn tick_until(world: &mut ServerWorld, limit: usize, mut done: impl FnMut(&ServerWorld) -> bool) {
    for _ in 0..limit {
        world.tick(Vec::new());
        if done(world) {
            return;
        }
    }
    panic!("the condition did not hold within {limit} ticks");
}

/// A husk searches for iron golems (`Zombie.java:126`) and a golem for any
/// enemy but a creeper (`IronGolem.java:77-80`), so each takes the other; an
/// invulnerable golem is no valid target (`LivingEntity.java:927-933`), and
/// the husk lets it go on its next full selector tick.
#[test]
fn a_husk_and_an_iron_golem_take_each_other_and_the_husk_drops_an_invulnerable_golem() {
    let mut world =
        spawn_pinned_restore(datapack_config(), &restore_candidate(0, None, Vec::new()));
    install_floor(&mut world);
    let y = 65.0;
    let (husk, husk_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_HUSK_ID,
        Vec3d { x: 4.5, y, z: 8.5 },
    );
    let (golem, golem_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d { x: 10.5, y, z: 8.5 },
    );

    tick_until(&mut world, 200, |world| {
        target_of(world, husk).is_some() && target_of(world, golem).is_some()
    });
    assert_eq!(target_of(&world, husk), Some(golem_uuid));
    assert_eq!(target_of(&world, golem), Some(husk_uuid));
    for _ in 0..10 {
        world.tick(Vec::new());
        assert_eq!(
            target_of(&world, husk),
            Some(golem_uuid),
            "the husk keeps its target"
        );
    }

    world
        .stored_entity_mut_for_test(DIMENSION, golem)
        .unwrap()
        .set_invulnerable(true);
    tick_until(&mut world, 4, |world| target_of(world, husk).is_none());
    assert_eq!(target_of(&world, golem), Some(husk_uuid));
}

/// A player's blow angers a zombified piglin at the player and alerts its kin
/// (`HurtByTargetGoal.java:60-115`); the anger follows the target
/// (`NeutralMob.java:58-89`), and a player turned creative is dropped and
/// forgiven.
#[test]
fn a_hurt_zombified_piglin_takes_the_player_alerts_its_kin_and_forgives_a_creative_player() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = Uuid::from_u128(1);
    let alice_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let y = floor_top();
    let (victim, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        Vec3d { x: 3.5, y, z: 0.5 },
    );
    let (kin, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        Vec3d { x: 5.5, y, z: 2.5 },
    );
    world.tick(receiver.drain());
    assert_eq!(target_of(&world, victim), None);

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            DIMENSION,
            victim,
            "minecraft:generic",
            1.0,
            Some(crate::commands::CommandDamageSourceEntity::Player {
                user: crate::admin::NameAndId {
                    id: alice,
                    name: "alice".into(),
                }
                .into(),
                entity_id: alice_entity_id,
                position: Vec3d::default(),
                instabuild: false,
                may_build: true,
                spectator: false,
                display_name: NbtSummaryValue::string("alice"),
            }),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);

    tick_until(&mut world, 4, |world| {
        target_of(world, victim).is_some() && target_of(world, kin).is_some()
    });
    assert_eq!(target_of(&world, victim), Some(alice));
    assert_eq!(target_of(&world, kin), Some(alice));
    world.tick(receiver.drain());
    let game_time = world.game_time();
    for piglin in [victim, kin] {
        let (end_time, angry_at) = saved_anger(&world, piglin);
        assert_eq!(angry_at, Some(alice));
        let remaining = end_time.unwrap() - game_time;
        // `PERSISTENT_ANGER_TIME` restarts every tick the target stays
        // (`ZombifiedPiglin.java:58`, `NeutralMob.java:75-77`).
        assert!((399..=780).contains(&remaining), "{remaining}");
    }

    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    tick_until(&mut world, 4, |world| {
        target_of(world, victim).is_none() && target_of(world, kin).is_none()
    });
    world.tick(receiver.drain());
    for piglin in [victim, kin] {
        assert_eq!(saved_anger(&world, piglin), (Some(-1), None));
    }
}

/// `addPersistentAngerSaveData` writes the anger into the record and
/// `readPersistentAngerSaveData` reads it back and retargets its subject
/// (`NeutralMob.java:34-56`), over a real save and restart.
#[test]
fn zombified_piglin_anger_at_a_mob_survives_a_restart_and_retargets_it() {
    let root = std::env::temp_dir().join(format!(
        "bbb-mob-target-anger-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-mob-target-anger-restart");
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    install_floor(&mut world);
    let y = 65.0;
    let (piglin, piglin_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        Vec3d { x: 2.5, y, z: 2.5 },
    );
    let (husk, husk_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_HUSK_ID,
        Vec3d { x: 5.5, y, z: 2.5 },
    );
    // A blow at `tickCount` 0 matches `HurtByTargetGoal`'s initial `timestamp`
    // (`HurtByTargetGoal.java:22`, `:36`), so the piglin lives one tick first.
    world.tick(Vec::new());
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            DIMENSION,
            piglin,
            "minecraft:generic",
            1.0,
            Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: husk,
                uuid: Some(husk_uuid),
                position: Some(Vec3d::default()),
                entity_resource_id: "minecraft:husk".to_string(),
                is_mob: true,
                is_living_non_player: true,
                display_name: NbtSummaryValue::string("minecraft:husk"),
            }),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    tick_until(&mut world, 4, |world| {
        saved_anger(world, piglin).1 == Some(husk_uuid)
    });
    assert_eq!(target_of(&world, piglin), Some(husk_uuid));
    let saved = saved_anger(&world, piglin);

    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut reloaded = storage.load_candidate().unwrap();
    install_forced_ticket(&mut reloaded);
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    let piglin = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, piglin_uuid)
        .expect("the piglin reloads");
    assert_eq!(saved_anger(&restored, piglin), saved);
    assert_eq!(target_of(&restored, piglin), None);
    tick_until(&mut restored, 5, |world| target_of(world, piglin).is_some());
    assert_eq!(target_of(&restored, piglin), Some(husk_uuid));
    assert_eq!(saved_anger(&restored, piglin).1, Some(husk_uuid));

    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_ocelot_crouches_then_sprints_after_real_prey_and_keeps_its_rider_attached() {
    use crate::tests::entities::{console_command, console_command_answer};
    use bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID;
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, VANILLA_ATTRIBUTE_SCALE_ID,
    };
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    use bbb_protocol::entity_synched_data::{
        VANILLA_ENTITY_POSE_DATA_ID, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
        VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
    };
    use bbb_protocol::entity_types::{
        vanilla_entity_type_passenger_attachments, vanilla_entity_type_vehicle_attachment,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
    };
    use bbb_protocol::packets::EntityDataValueKind;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    install_floor(&mut world);
    let command = |world: &mut ServerWorld, text: &str| {
        let operation = world.game_time() as u64 + 100;
        let tick = console_command(&sender, &receiver, world, operation, text);
        assert!(console_command_answer(&tick).0.is_some(), "{text}");
        tick
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamemode spectator observer");
    let (ocelot, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &mut world,
        "summon armor_stand 3.5 65 8.5 {UUID:[I;0,0,0,2],Small:1b,NoGravity:1b,Invulnerable:1b}",
    );
    let rider_uuid = Uuid::from_u128(2);
    let rider = world
        .live_entity_id_for_uuid_for_test(DIMENSION, rider_uuid)
        .unwrap();
    command(&mut world, &format!("ride {rider_uuid} mount {uuid}"));
    command(
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    let assert_rider = |world: &ServerWorld| {
        assert_eq!(
            world.stored_passengers_for_test(DIMENSION, ocelot),
            &[rider]
        );
        let body = world.stored_entity_for_test(DIMENSION, ocelot).unwrap();
        assert!(matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_no_ai()));
        let passenger = world.stored_entity_for_test(DIMENSION, rider).unwrap();
        let scale = body
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
        assert_eq!(scale, 2.0);
        let [x, y, z] =
            vanilla_entity_type_passenger_attachments(VANILLA_ENTITY_TYPE_OCELOT_ID).unwrap()[0];
        let [rx, ry, rz] =
            vanilla_entity_type_vehicle_attachment(passenger.entity_type_id()).unwrap();
        let (sin, cos) = (-f64::from(body.y_rot()).to_radians()).sin_cos();
        let (rsin, rcos) = (-f64::from(passenger.y_rot()).to_radians()).sin_cos();
        let expected = Vec3d {
            x: body.position().x + f64::from(x * scale) * cos + f64::from(z * scale) * sin
                - f64::from(rx) * rcos
                - f64::from(rz) * rsin,
            y: body.position().y + f64::from(y * scale) - f64::from(ry),
            z: body.position().z + f64::from(z * scale) * cos
                - f64::from(x * scale) * sin
                - f64::from(rz) * rcos
                + f64::from(rx) * rsin,
        };
        for (actual, expected) in [
            (passenger.position().x, expected.x),
            (passenger.position().y, expected.y),
            (passenger.position().z, expected.z),
        ] {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "{actual}, expected {expected}"
            );
        }
    };
    let start = world
        .stored_entity_for_test(DIMENSION, ocelot)
        .unwrap()
        .position();
    let base_speed = world
        .stored_entity_for_test(DIMENSION, ocelot)
        .unwrap()
        .living()
        .unwrap()
        .attributes()
        .base_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap();
    let chicken_uuid = Uuid::from_u128(3);
    // Invulnerable prey cannot be acquired by combat targeting.
    let prey = command(
        &mut world,
        "summon chicken 9.5 65 8.5 {UUID:[I;0,0,0,3],NoAI:1b}",
    );
    let shared_flag_update = |events: &[WorldEvent], sprinting: bool| {
        events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetEntityData(update) }
                if update.id == ocelot && update.values.iter().any(|value|
                    value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Byte(flags)
                            if (flags & VANILLA_ENTITY_SHARED_FLAG_SPRINTING != 0) == sprinting))))
    };
    let mut events = prey.events;
    let mut crouched = false;
    let mut sprinted = false;
    let mut sprint_events_from = 0;
    for _ in 0..300 {
        let body = world.stored_entity_for_test(DIMENSION, ocelot).unwrap();
        let sprint = body.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING);
        if body.pose_id() == VANILLA_ENTITY_POSE_CROUCHING_ID && !sprint {
            assert_eq!(target_of(&world, ocelot), Some(chicken_uuid));
            let moved = (body.position().x - start.x).hypot(body.position().z - start.z);
            if moved > 0.05 {
                assert_rider(&world);
                assert!(!body
                    .living()
                    .unwrap()
                    .attributes()
                    .has_modifier(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, "minecraft:sprinting"));
                if !crouched {
                    sprint_events_from = events.len();
                }
                crouched = true;
            }
        }
        if crouched && body.pose_id() == VANILLA_ENTITY_POSE_STANDING_ID && sprint {
            assert_eq!(target_of(&world, ocelot), Some(chicken_uuid));
            assert_rider(&world);
            let living = body.living().unwrap();
            let modifier = living
                .attributes()
                .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                .unwrap()
                .modifier("minecraft:sprinting")
                .unwrap();
            assert_eq!(modifier.amount, f64::from(0.3_f32));
            assert_eq!(
                modifier.operation.id(),
                VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
            );
            assert!(
                (living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                    - base_speed * (1.0 + f64::from(0.3_f32)))
                .abs()
                    < 1.0e-10
            );
            sprinted = true;
        }
        if sprinted && shared_flag_update(&events[sprint_events_from..], true) {
            break;
        }
        events.extend(world.tick(receiver.drain()).events);
    }
    assert!(
        crouched && sprinted,
        "natural hunting exposes both movement poses"
    );
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetEntityData(update) }
            if update.id == ocelot && update.values.iter().any(|value|
                value.data_id == VANILLA_ENTITY_POSE_DATA_ID
                    && value.value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_CROUCHING_ID)))));
    assert!(shared_flag_update(&events[sprint_events_from..], true));
    assert!(events[sprint_events_from..].iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetEntityData(update) }
            if update.id == ocelot && update.values.iter().any(|value|
                value.data_id == VANILLA_ENTITY_POSE_DATA_ID
                    && value.value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID)))));
    assert!(events[sprint_events_from..].iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::UpdateAttributes(update) }
            if update.entity_id == ocelot && update.attributes.iter().any(|attribute|
                attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID
                    && attribute.modifiers.iter().any(|modifier| modifier.id == "minecraft:sprinting"
                        && modifier.amount == f64::from(0.3_f32)
                        && modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID)))));

    let removed_prey = command(&mut world, &format!("kill {chicken_uuid}"));
    let mut reset_events = removed_prey.events;
    let mut reset = false;
    for _ in 0..100 {
        let body = world.stored_entity_for_test(DIMENSION, ocelot).unwrap();
        if target_of(&world, ocelot).is_none()
            && body.pose_id() == VANILLA_ENTITY_POSE_STANDING_ID
            && !body.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING)
        {
            assert_rider(&world);
            assert!(!body
                .living()
                .unwrap()
                .attributes()
                .has_modifier(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, "minecraft:sprinting"));
            assert!(
                (body
                    .living()
                    .unwrap()
                    .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
                    - base_speed)
                    .abs()
                    < 1.0e-10
            );
            reset = true;
        }
        if reset && shared_flag_update(&reset_events, false) {
            break;
        }
        reset_events.extend(world.tick(receiver.drain()).events);
    }
    assert!(
        reset,
        "losing real prey clears the movement pose and sprint modifier"
    );
    assert!(shared_flag_update(&reset_events, false));
    assert!(reset_events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::UpdateAttributes(update) }
            if update.entity_id == ocelot && update.attributes.iter().any(|attribute|
                attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID
                    && attribute.modifiers.iter().all(|modifier| modifier.id != "minecraft:sprinting")))));
}

mod avoid_panic;
mod bee_targets;
mod breeze;
mod camel_daily;
mod camel_sitting;
mod dolphin;
mod dolphin_play_items;
mod dolphin_treasure;
mod enderman_freeze;
mod enderman_look;
mod evoker_vexes;
mod fox;
mod fox_trusted;
mod goat_daily;
mod goat_idle;
mod goat_ram;
mod goat_tempt;
mod guardian;
mod guardian_runtime;
mod herd_tempt;
mod hoglin_groups;
mod hoglin_idle;
mod hoglin_repellents;
mod hoglin_retreat;
mod idle_ai;
mod iron_golem_flower;
mod iron_golem_village;
mod johnny;
mod llama_caravan;
mod melee;
mod melee_charge;
mod melee_effects;
mod melee_enchantments;
mod melee_weapon;
mod mob_sounds;
mod nautilus;
mod piglin_celebration;
mod piglin_combat;
mod piglin_environment;
mod piglin_idle;
mod piglin_retreat;
mod piglin_riding;
mod ranged;
mod shulker_peek;
mod spider;
mod strider_daily;
mod wandering_trader;
mod wandering_trader_drink;

mod neutral_load;

mod command_list_codecs;
mod player_pose;
mod pufferfish;
mod world_pose_consumers;
