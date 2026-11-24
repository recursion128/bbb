use super::*;
use crate::entity::GroundPathType;
use crate::tests::entities::console_command_answer;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_server_types::geometry::Vec3Ops;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn production_world() -> ServerWorld {
    spawn_pinned_restore(datapack_config(), &restore_candidate(0, None, Vec::new()))
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
                SET_BLOCK_UPDATE_ALL,
            ));
        }
    }
}

fn spawn(world: &mut ServerWorld, entity_type_id: i32, position: Vec3d) -> (i32, Uuid) {
    world.spawn_stored_entity_for_test(
        DIMENSION,
        entity_type_id,
        position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(entity_type_id).unwrap(),
        ),
    )
}

fn running_goals(world: &ServerWorld, entity_id: i32) -> Vec<String> {
    world
        .mob_goal_debug_for_test(DIMENSION, entity_id)
        .expect("the mob keeps its goal selector")
        .goals
        .into_iter()
        .filter(|goal| goal.is_running)
        .map(|goal| goal.name)
        .collect()
}

/// Ticks until the mob holds a path, which only its flying stroll can request.
fn tick_until_path(
    world: &mut ServerWorld,
    entity_id: i32,
    limit: usize,
) -> crate::entity::GroundPath {
    for _ in 0..limit {
        world.tick(Vec::new());
        if let Some(path) = world.ground_path_for_test(DIMENSION, entity_id) {
            return path;
        }
    }
    panic!("no flying path within {limit} ticks");
}

/// With no player in reach `noActionTime` passes 100 and stops the stroll; a
/// persistent mob keeps it at zero.
fn hold_still_time_at_zero(world: &mut ServerWorld, entity_id: i32) {
    write_record_entry(
        world,
        entity_id,
        "PersistenceRequired",
        NbtSummaryValue::Byte(1),
    );
}

fn assert_flying_path(path: &crate::entity::GroundPath) {
    assert!(!path.nodes().is_empty());
    for node in path.nodes() {
        assert!(
            matches!(
                node.path_type,
                GroundPathType::Open | GroundPathType::Walkable
            ),
            "a flying path only crosses open or standable air: {node:?}"
        );
        assert!(node.cost_malus >= 0.0);
    }
}

#[test]
fn a_live_parrot_wanders_through_flying_navigation_on_production_ticks() {
    let mut world = production_world();
    install_floor(&mut world);
    let (parrot, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    hold_still_time_at_zero(&mut world, parrot);
    let path = tick_until_path(&mut world, parrot, 600);
    assert_eq!(running_goals(&world, parrot), ["RandomStrollGoal"]);
    assert_flying_path(&path);
    // With no perch in reach the wander hovers one to three blocks over the
    // stone (`HoverRandomPos.java:35-41`), or samples open air when every hover
    // attempt fails.
    let target = path.target();
    assert!((65..=72).contains(&target.y), "{target:?}");

    let start = world
        .stored_entity_for_test(DIMENSION, parrot)
        .unwrap()
        .position();
    let mut flew = false;
    for _ in 0..40 {
        world.tick(Vec::new());
        let body = world.stored_entity_for_test(DIMENSION, parrot).unwrap();
        flew |= body.no_gravity();
        // `Parrot.checkFallDamage` is empty (`Parrot.java:313-314`).
        assert_eq!(body.fall_distance(), 0.0);
        if world.ground_path_for_test(DIMENSION, parrot).is_none() {
            break;
        }
    }
    // `FlyingMoveControl` lifts gravity while it steers (`FlyingMoveControl.java:21`).
    assert!(flew);
    let moved = world
        .stored_entity_for_test(DIMENSION, parrot)
        .unwrap()
        .position();
    assert!(
        moved.distance_to_sqr(start) > 0.25,
        "{start:?} -> {moved:?}"
    );
}

#[test]
fn a_live_wither_strolls_through_flying_navigation_on_production_ticks() {
    let mut world = production_world();
    install_floor(&mut world);
    let (wither, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let path = tick_until_path(&mut world, wither, 400);
    assert!(running_goals(&world, wither).contains(&"RandomStrollGoal".to_string()));
    assert_flying_path(&path);

    let start = world
        .stored_entity_for_test(DIMENSION, wither)
        .unwrap()
        .position();
    let mut flew = false;
    for _ in 0..40 {
        world.tick(Vec::new());
        flew |= world
            .stored_entity_for_test(DIMENSION, wither)
            .unwrap()
            .no_gravity();
    }
    assert!(flew);
    let moved = world
        .stored_entity_for_test(DIMENSION, wither)
        .unwrap()
        .position();
    assert!(
        moved.distance_to_sqr(start) > 0.25,
        "{start:?} -> {moved:?}"
    );
}

#[test]
fn a_flying_parrot_reloads_mid_flight_and_wanders_again() {
    let root = std::env::temp_dir().join(format!(
        "bbb-flying-navigation-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-flying-navigation-restart");
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    install_floor(&mut world);
    let (parrot, parrot_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    hold_still_time_at_zero(&mut world, parrot);
    tick_until_path(&mut world, parrot, 600);
    for _ in 0..3 {
        world.tick(Vec::new());
    }
    let saved = world.stored_entity_for_test(DIMENSION, parrot).unwrap();
    let (saved_position, saved_no_gravity) = (saved.position(), saved.no_gravity());

    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut reloaded = storage.load_candidate().unwrap();
    install_forced_ticket(&mut reloaded);
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, parrot_uuid)
        .expect("the parrot is restored from its entity region");
    let body = restored
        .stored_entity_for_test(DIMENSION, restored_id)
        .unwrap();
    assert_eq!(body.position(), saved_position);
    // `NoGravity` is saved data (`Entity.java:2000-2002`, `:2084`); the path is not.
    assert_eq!(body.no_gravity(), saved_no_gravity);
    assert_eq!(restored.ground_path_for_test(DIMENSION, restored_id), None);

    let path = tick_until_path(&mut restored, restored_id, 600);
    assert_eq!(running_goals(&restored, restored_id), ["RandomStrollGoal"]);
    assert_flying_path(&path);
    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn write_record_entry(world: &mut ServerWorld, entity_id: i32, name: &str, value: NbtSummaryValue) {
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(DIMENSION, entity_id)
        .expect("the mob answers its record")
    else {
        panic!("an entity record is a compound");
    };
    entries.retain(|entry| entry.name != name);
    entries.push(NbtSummaryEntry {
        name: name.into(),
        value,
    });
    world.load_stored_entity_record_for_test(DIMENSION, entity_id, &entries);
}

/// Writes `Owner` through the record load, which is how
/// `TamableAnimal.readAdditionalSaveData` tames a body (`TamableAnimal.java:67-78`).
fn tame_to(world: &mut ServerWorld, entity_id: i32, owner: Uuid) {
    let bytes = owner.as_bytes();
    let words = (0..4)
        .map(|word| i32::from_be_bytes(bytes[word * 4..word * 4 + 4].try_into().unwrap()))
        .collect();
    write_record_entry(world, entity_id, "Owner", NbtSummaryValue::IntArray(words));
}

fn block_of(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

/// Ticks until the follow-owner goal runs and holds a path. The parrot's
/// `LookAtPlayerGoal` sits one priority above it on the shared LOOK flag
/// (`Parrot.java:165-167`), so a glance at the owner can hold the follow back.
fn tick_until_following(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    entity_id: i32,
    limit: usize,
) -> crate::entity::GroundPath {
    for _ in 0..limit {
        world.tick(receiver.drain());
        let following = running_goals(world, entity_id).contains(&"FollowOwnerGoal".to_string());
        if let Some(path) = world
            .ground_path_for_test(DIMENSION, entity_id)
            .filter(|_| following)
        {
            return path;
        }
    }
    panic!("the parrot never followed its owner within {limit} ticks");
}

#[test]
fn a_tamed_parrot_flies_to_its_session_owner() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    let owner = world.publication().players[0].position;
    let (parrot, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        Vec3d {
            x: owner.x + 8.0,
            ..owner
        },
    );
    tame_to(&mut world, parrot, Uuid::from_u128(0xfeed));

    let path = tick_until_following(&mut world, &receiver, parrot, 400);
    // `navigation.moveTo(owner, speedModifier)` paths to the owner's block
    // (`FollowOwnerGoal.java:88`, `FlyingPathNavigation.java:40-42`).
    assert_eq!(path.target(), block_of(owner));
    assert_flying_path(&path);

    for _ in 0..400 {
        world.tick(receiver.drain());
        let body = world.stored_entity_for_test(DIMENSION, parrot).unwrap();
        if body.position().distance_to_sqr(owner) <= 9.0 {
            return;
        }
    }
    panic!(
        "the parrot never reached its owner: {:?}",
        world
            .stored_entity_for_test(DIMENSION, parrot)
            .unwrap()
            .position()
    );
}

#[test]
fn a_tamed_parrot_teleports_beside_a_far_session_owner() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    let owner = world.publication().players[0].position;
    let (parrot, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        Vec3d {
            x: owner.x + 14.0,
            ..owner
        },
    );
    tame_to(&mut world, parrot, Uuid::from_u128(0xfeed));

    let owner_block = block_of(owner);
    for _ in 0..40 {
        world.tick(receiver.drain());
        let position = world
            .stored_entity_for_test(DIMENSION, parrot)
            .unwrap()
            .position();
        if position.distance_to_sqr(owner) < 144.0 {
            // `maybeTeleportTo` snaps to a block centre two or three blocks out
            // on the owner's walkable layer (`TamableAnimal.java:253-274`).
            let landing = block_of(position);
            assert_eq!(
                (
                    position.x - position.x.floor(),
                    position.z - position.z.floor()
                ),
                (0.5, 0.5)
            );
            assert_eq!(landing.y, owner_block.y);
            let (dx, dz) = (landing.x - owner_block.x, landing.z - owner_block.z);
            assert!(dx.abs() <= 3 && dz.abs() <= 3 && (dx.abs() >= 2 || dz.abs() >= 2));
            assert_eq!(world.ground_path_for_test(DIMENSION, parrot), None);
            return;
        }
    }
    panic!("the parrot never teleported to its owner");
}

#[test]
fn a_live_parrot_follows_a_nearby_mob_and_hovers_beside_it() {
    let mut world = production_world();
    install_floor(&mut world);
    let (parrot, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let cow_position = Vec3d {
        x: 9.5,
        y: 65.0,
        z: 8.5,
    };
    let (cow, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID, cow_position);
    write_record_entry(&mut world, cow, "NoAI", NbtSummaryValue::Byte(1));

    // The first full selector pass finds the cow six blocks out inside the
    // seven-block search and paths to its block (`FollowMobGoal.java:39-51`,
    // `:83`).
    let mut followed = None;
    for _ in 0..40 {
        world.tick(Vec::new());
        if running_goals(&world, parrot).contains(&"FollowMobGoal".to_string()) {
            followed = world.ground_path_for_test(DIMENSION, parrot);
            break;
        }
    }
    let path = followed.expect("the parrot follows the cow");
    assert_eq!(path.target(), block_of(cow_position));
    assert_flying_path(&path);

    // It flies in to within the stop distance (`FollowMobGoal.java:55`,
    // `:82-92`) without accruing a fall.
    let mut closest = f64::MAX;
    for _ in 0..200 {
        world.tick(Vec::new());
        let body = world.stored_entity_for_test(DIMENSION, parrot).unwrap();
        closest = closest.min(body.position().distance_to_sqr(cow_position));
        assert_eq!(body.fall_distance(), 0.0);
    }
    assert!(closest <= 9.0, "the parrot stayed {closest} away");
}

#[test]
fn tame_parrots_ride_both_shoulders_save_and_return_on_admitted_zero_damage() {
    let root = std::env::temp_dir().join(format!("bbb-parrot-shoulders-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-parrot-shoulders");
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 100_000;
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 0xfeed);
    let disabled = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule spawn_mobs false",
    );
    assert!(console_command_answer(&disabled).0.is_some());
    install_floor(&mut world);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let parrots = [(1, 9.5), (3, 7.5)].map(|(variant, x)| {
        let (id, uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_PARROT_ID,
            Vec3d { x, y: 65.0, z: 8.5 },
        );
        tame_to(&mut world, id, Uuid::from_u128(0xfeed));
        write_record_entry(&mut world, id, "Variant", NbtSummaryValue::Int(variant));
        write_record_entry(&mut world, id, "Sitting", NbtSummaryValue::Byte(1));
        write_record_entry(
            &mut world,
            id,
            "data",
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "shoulder_marker".into(),
                value: NbtSummaryValue::Int(variant),
            }]),
        );
        (id, uuid, variant)
    });
    let (imitated_mob, _) = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        },
    );
    write_record_entry(&mut world, imitated_mob, "NoAI", NbtSummaryValue::Byte(1));
    let heard = |tick: &TickResult, source: SoundSource, resource: &str, volume: f32| {
        tick.events.iter().any(|event| matches!(
            event.packet_for(ConnectionId(1)),
            Some(ClientboundPlayPacket::Sound(sound))
                if sound.source == source && sound.volume == volume && (0.8..=1.2).contains(&sound.pitch)
                    && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(registry_id) == Some(resource))
        ))
    };
    let mut live_ambient = false;
    let mut live_imitation = false;
    for elapsed in 0..12_000 {
        let tick = world.tick(receiver.drain());
        live_ambient |= heard(
            &tick,
            SoundSource::Neutral,
            "minecraft:entity.parrot.ambient",
            1.0,
        );
        live_imitation |= heard(
            &tick,
            SoundSource::Neutral,
            "minecraft:entity.parrot.imitate.shulker",
            0.7,
        );
        if elapsed >= 120 && live_ambient && live_imitation {
            break;
        }
    }
    assert!(
        live_ambient && live_imitation,
        "the live seated parrots emit ambient and nearby-mob imitation sounds"
    );
    let player = world.player_entity_id_for_test(ConnectionId(1));
    let metadata = |tick: &TickResult,
                    entity_id: i32,
                    expected: [Option<i32>; 2],
                    recipient: u64| {
        tick.events.iter().any(|event| matches!(
            event.packet_for(ConnectionId(recipient)),
            Some(ClientboundPlayPacket::SetEntityData(data))
                if data.id == entity_id
                    && [bbb_protocol::entity_synched_data::VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                        bbb_protocol::entity_synched_data::VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID]
                    .into_iter().zip(expected).all(|(id, variant)| data.values.iter().any(|row|
                        row.data_id == id && row.value == EntityDataValueKind::OptionalUnsignedInt(variant)))
        ))
    };
    let mut expected = [None; 2];
    for (side, (id, _, variant)) in parrots.iter().enumerate() {
        let clicked = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: *id,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        expected[side] = Some(*variant);
        let mut synced = metadata(&clicked, player, expected, 1);
        for _ in 0..40 {
            let tick = if let Some(parrot) = world.stored_entity_for_test(DIMENSION, *id) {
                let position = Vec3d {
                    y: 65.0,
                    ..parrot.position()
                };
                place_player(&sender, &receiver, &mut world, 1, position, None)
            } else {
                world.tick(receiver.drain())
            };
            synced |= metadata(&tick, player, expected, 1);
            if synced && world.stored_entity_for_test(DIMENSION, *id).is_none() {
                break;
            }
        }
        assert!(
            world.stored_entity_for_test(DIMENSION, *id).is_none(),
            "the live parrot landed on shoulder {side}"
        );
        assert!(synced, "shoulder {side} metadata reached its owner");
    }
    let mut shoulder_ambient = false;
    let mut shoulder_imitation = false;
    for _ in 0..12_000 {
        let tick = world.tick(receiver.drain());
        shoulder_ambient |= heard(
            &tick,
            SoundSource::Players,
            "minecraft:entity.parrot.ambient",
            1.0,
        );
        shoulder_imitation |= heard(
            &tick,
            SoundSource::Players,
            "minecraft:entity.parrot.imitate.shulker",
            0.7,
        );
        if shoulder_ambient && shoulder_imitation {
            break;
        }
    }
    assert!(
        shoulder_ambient && shoulder_imitation,
        "the shoulder parrots use the same sounds with the player's category"
    );
    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
        .unwrap();
    let saved_shoulders = request.player_patches[0].shoulder_entities.clone();
    for (shoulder, (_, _, variant)) in saved_shoulders.iter().zip(parrots) {
        assert!(shoulder
            .iter()
            .any(|entry| entry.name == "Variant" && entry.value == NbtSummaryValue::Int(variant)));
        assert!(shoulder.iter().any(|entry| entry.name == "data"));
    }
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let departed = world
        .save_request(1_786_000_020_001, 201, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        departed.player_patches[0].shoulder_entities,
        saved_shoulders
    );
    storage.save_snapshot(&candidate, &departed).unwrap();
    drop(world);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut reloaded = storage.load_candidate().unwrap();
    install_forced_ticket(&mut reloaded);
    assert_eq!(
        reloaded
            .load_player(Uuid::from_u128(0xfeed))
            .unwrap()
            .unwrap()
            .1
            .shoulder_entities()
            .map(<[_]>::to_vec),
        saved_shoulders
    );
    let mut restored = spawn_pinned_restore(config, &reloaded);
    let (joined, handshake) =
        join_loaded_ticks(&sender, &receiver, &mut restored, 1, "owner", 0xfeed);
    let player = restored.player_entity_id_for_test(ConnectionId(1));
    assert!(
        metadata(&joined, player, expected, 1) || metadata(&handshake, player, expected, 1),
        "restored shoulders reach their own session"
    );
    let (observer, handshake) =
        join_loaded_ticks(&sender, &receiver, &mut restored, 2, "observer", 2);
    let mut paired =
        metadata(&observer, player, expected, 2) || metadata(&handshake, player, expected, 2);
    let owner_position = restored
        .publication()
        .players
        .iter()
        .find(|player| player.uuid == Uuid::from_u128(0xfeed))
        .unwrap()
        .position;
    let approached = place_player(
        &sender,
        &receiver,
        &mut restored,
        2,
        Vec3d {
            x: owner_position.x + 2.0,
            ..owner_position
        },
        None,
    );
    paired |= metadata(&approached, player, expected, 2);
    for _ in 0..20 {
        paired |= metadata(&restored.tick(receiver.drain()), player, expected, 2);
    }
    assert!(
        paired,
        "a nearby new observer receives both restored shoulder variants: players={:?}, shoulders={:?}",
        restored.publication().players,
        restored.save_request(1_786_000_020_001, 201, RegionCompression::Zlib).unwrap().player_patches.iter().find(|patch| patch.uuid == Uuid::from_u128(0xfeed)).unwrap().shoulder_entities,
    );
    let hit = console_command(
        &sender,
        &receiver,
        &mut restored,
        2,
        "damage owner 0 minecraft:out_of_world",
    );
    let mut cleared = metadata(&hit, player, [None; 2], 1);
    for _ in 0..30 {
        cleared |= metadata(&restored.tick(receiver.drain()), player, [None; 2], 1);
    }
    assert!(
        cleared,
        "admitted zero damage clears the owner's shoulder metadata"
    );
    for (_, uuid, variant) in parrots {
        let id = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .expect("the original parrot UUID is live again");
        let NbtSummaryValue::Compound(entries) = restored
            .stored_entity_command_data_for_test(DIMENSION, id)
            .unwrap()
        else {
            panic!("parrot record");
        };
        for name in ["Variant", "Owner", "data"] {
            let original = saved_shoulders
                .iter()
                .find(|shoulder| {
                    shoulder.iter().any(|entry| {
                        entry.name == "Variant" && entry.value == NbtSummaryValue::Int(variant)
                    })
                })
                .unwrap();
            assert_eq!(
                entries.iter().find(|entry| entry.name == name),
                original.iter().find(|entry| entry.name == name),
                "{name} survives shoulder restoration"
            );
        }
    }
    let cleared = restored
        .save_request(1_786_000_020_002, 202, RegionCompression::Zlib)
        .unwrap();
    assert!(cleared
        .player_patches
        .iter()
        .find(|patch| patch.uuid == Uuid::from_u128(0xfeed))
        .unwrap()
        .shoulder_entities
        .iter()
        .all(Vec::is_empty));
    storage.save_snapshot(&reloaded, &cleared).unwrap();
    drop(restored);
    let cleared = storage.load_candidate().unwrap();
    assert!(cleared
        .load_player(Uuid::from_u128(0xfeed))
        .unwrap()
        .unwrap()
        .1
        .raw()
        .entries()
        .iter()
        .all(|entry| entry.name != "ShoulderEntityLeft" && entry.name != "ShoulderEntityRight"));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
