//! `CreakingHeartBlockEntity.serverTick`'s countdown half
//! reached through the real block
//! entity tick: the ticker, the state the logs and the attribute pick, the
//! protector spawn with its binding, and the three release conditions.

use super::*;

use crate::entity::projectile_action::{PendingProjectileAction, PendingProjectileHurt};
use crate::level_event::PendingHurtEntity;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID;
use bbb_protocol::entity_synched_data::VANILLA_CREAKING_HOME_POS_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
use bbb_protocol::packets::EntityDataValueKind;

use super::helpers::{self, state_with as state_of};

/// `CreakingHeartBlockEntity.TICKS_GRACE_PERIOD` (`:57`).
const TICKS_GRACE_PERIOD: usize = 30;

fn heart_world(
    config: ServerWorldConfig,
    game_time: i64,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_game_time_for_test(game_time);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(game_time),
        &mut Vec::new(),
    );
    (sender, receiver, world)
}

/// A heart standing between two pale oak logs on its own axis, dormant so the
/// block-entity ticker has a state to move.
fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    helpers::creaking_place_heart(world, at, "dormant");
}

/// The heart's own state name.
fn heart_state(world: &mut ServerWorld, at: BlockPos) -> Option<String> {
    let state = world.block_state_by_key(
        &world.require_level_id(&Java26LevelKey::from_builtin(
            Java26BuiltinDimension::Overworld,
        )),
        at,
    )?;
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("creaking_heart_state")
        .cloned()
}

fn protector_uuid(world: &mut ServerWorld, at: BlockPos) -> Option<Uuid> {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.creaking_heart())
        .and_then(|heart| heart.protector_uuid())
}

/// The live creaking entities of the overworld as `(entity id, uuid)`.
fn creakings(world: &ServerWorld) -> Vec<(i32, Uuid)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            (entity.entity_type_id() == VANILLA_ENTITY_TYPE_CREAKING_ID)
                .then(|| (id, entity.uuid()))
        })
        .collect()
}

/// `serverTick`: an awake heart with
/// pale oak logs, monsters spawning and a player inside 32 blocks spawns a
/// protector and binds it.
#[test]
fn an_awake_heart_spawns_and_binds_a_protector() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    let at = BlockPos { x: 0, y: 64, z: 1 };
    // `SpawnUtil.trySpawnMob` lands anywhere within 16 blocks of the heart and
    // the entity sound only carries 16 blocks, so four listeners at
    // (+-8, +-8) keep one of them in range of every spawn spot.
    for (connection_id, name, dx, dz) in [
        (1, "alice", -8.0, -8.0),
        (2, "bob", -8.0, 8.0),
        (3, "carol", 8.0, -8.0),
        (4, "dave", 8.0, 8.0),
    ] {
        join_loaded_stocked(
            &sender,
            &receiver,
            &mut world,
            connection_id,
            name,
            u128::from(connection_id),
        );
        world.set_player_position_for_test(
            ConnectionId(connection_id),
            Vec3d {
                x: f64::from(at.x) + 0.5 + dx,
                y: f64::from(at.y),
                z: f64::from(at.z) + 0.5 + dz,
            },
        );
    }
    // `EnvironmentAttributes.CREAKING_ACTIVE` is the overworld timeline's
    // window, which the fixture clock starts outside.
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    place_heart(&mut world, at);

    // The first pass only arms the ticker (`:98`).
    world.tick(receiver.drain());
    assert!(creakings(&world).is_empty(), "the ticker has not fired yet");
    assert_eq!(heart_state(&mut world, at).as_deref(), Some("dormant"));

    let second = world.tick(receiver.drain());
    // A block sound queued from the block-entity phase is flushed on the next
    // tick: `flush_level_events` runs before `tick_block_entities`.
    let third = world.tick(receiver.drain());
    let published: Vec<&WorldEvent> = second.events.iter().chain(third.events.iter()).collect();
    assert_eq!(heart_state(&mut world, at).as_deref(), Some("awake"));
    let spawned = creakings(&world);
    assert_eq!(spawned.len(), 1, "{spawned:?}");
    let bound = protector_uuid(&mut world, at);
    assert_eq!(
        bound,
        Some(spawned[0].1),
        "the heart binds the protector it spawned"
    );
    // `:207-213`: the place event, the spawn animation and both sounds.
    assert!(
        published.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::EntityEvent(event))
                if event.entity_id == spawned[0].0 && event.event_id == 60
        )),
        "{published:?}"
    );
    let sounds: Vec<&bbb_protocol::packets::SoundEvent> = published
        .iter()
        .filter_map(|event| match event.sent_packet() {
            Some(ClientboundPlayPacket::Sound(sound)) => Some(sound),
            _ => None,
        })
        .collect();
    for resource_id in [
        "minecraft:entity.creaking.spawn",
        "minecraft:block.creaking_heart.spawn",
    ] {
        let registry_id =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
                .expect("the sound is in the catalog");
        assert!(
            sounds.iter().any(|sound| matches!(
                &sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
                    if *id == registry_id
            )),
            "missing {resource_id} in {sounds:?}"
        );
    }
}

/// The same heart outside the attribute's window goes dormant and spawns
/// nothing, and one without logs uproots.
#[test]
fn the_state_gates_the_spawn() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 1_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    assert_eq!(heart_state(&mut world, at).as_deref(), Some("dormant"));
    assert!(creakings(&world).is_empty());

    // `updateCreakingState` (`:137-146`): no logs, no protector, no root.
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        state_of(
            "minecraft:creaking_heart",
            &[
                ("axis", "y"),
                ("creaking_heart_state", "dormant"),
                ("natural", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    assert_eq!(heart_state(&mut world, at).as_deref(), Some("uprooted"));
    assert!(creakings(&world).is_empty());
}

/// `updateCreakingState`'s `level.setBlock(pos, updated, 3)` (`:101-104`)
/// keeps the block entity, so the binding survives a state flip.
#[test]
fn a_state_update_keeps_the_binding() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    let bound = Uuid::from_u128(0xfeed_face_dead_beef_feed_face_dead_beef);
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::creaking_heart_mut)
        .expect("the heart's block entity")
        .set_protector_for_test(bound);

    world.tick(receiver.drain());
    world.tick(receiver.drain());
    assert_eq!(heart_state(&mut world, at).as_deref(), Some("awake"));
    assert_eq!(
        protector_uuid(&mut world, at),
        Some(bound),
        "the state write kept the block entity"
    );
}

/// `:117-125`: the attribute clearing releases the protector through
/// `removeProtector(null)`.
#[test]
fn the_attribute_clearing_releases_the_protector() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let spawned = creakings(&world);
    assert_eq!(spawned.len(), 1, "{spawned:?}");

    // The death sound carries 16 blocks, so the protector is brought next to
    // the listener wherever `SpawnUtil` put it.
    world.move_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        spawned[0].0,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    // Dusk ends the window, so the next fired tick
    // releases the protector it bound; the ticker was rolled to 20 + 0..4 by
    // the spawn itself, so this needs that many ticks.
    world.set_game_time_for_test(23_500);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(23_500),
        &mut Vec::new(),
    );
    let mut released = world.tick(receiver.drain());
    for _ in 0..24 {
        if creakings(&world).is_empty() {
            break;
        }
        released = world.tick(receiver.drain());
    }
    assert!(creakings(&world).is_empty(), "the protector was torn down");
    assert_eq!(
        protector_uuid(&mut world, at),
        None,
        "the binding was cleared"
    );
    let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.creaking.death",
    )
    .expect("the death sound is in the catalog");
    assert!(
        released.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } if matches!(
                &sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
                    if *id == registry_id
            )
        )),
        "the tear-down sound reaches the client"
    );
}

/// `Creaking.hurtServer` over a real player attack:
/// the heart-bound protector swallows the hit, plays event 66 and the sway
/// sound, and its heart starts the emitter and spreads resin.
#[test]
fn a_player_hit_on_the_protector_reaches_its_heart() {
    let (sender, receiver) = mailbox();
    // A creative attacker is `isInvulnerableTo`, so this
    // player fights in survival.
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..datapack_config()
    });
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));

    let health_before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, protector_id)
        .and_then(crate::entity::Entity::living)
        .map(crate::entity::LivingEntityState::health);
    // `SpawnUtil.trySpawnMob` searches ±16 horizontally, so the protector is
    // brought into the player's attack range, near enough to the heart that
    // the player also hears its resin.
    world.move_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        protector_id,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: protector_id,
        }),
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, protector_id)
            .and_then(crate::entity::Entity::living)
            .map(crate::entity::LivingEntityState::health),
        health_before,
        "the heart-bound protector takes no damage"
    );
    assert!(
        attacked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(event),
                ..
            } if event.entity_id == protector_id && event.event_id == 66
        )),
        "the hit animation reaches the client: {:?}",
        attacked.events
    );
    let sounds: Vec<&bbb_protocol::packets::SoundEvent> = attacked
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } => Some(sound),
            _ => None,
        })
        .collect();
    for resource_id in [
        "minecraft:entity.creaking.sway",
        "minecraft:block.resin.place",
    ] {
        let registry_id =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
                .expect("the sound is in the catalog");
        assert!(
            sounds.iter().any(|sound| matches!(
                &sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
                    if *id == registry_id
            )),
            "missing {resource_id} in {sounds:?}"
        );
    }
    // `spreadResin` (`:245-278`) attaches clumps to the logs around the heart,
    // each carrying the face that touches its log.
    let mut placed = Vec::new();
    for x in -2..=2 {
        for y in 62..=66 {
            for z in -2..=3 {
                let pos = BlockPos { x, y, z };
                let Some(state) = world.block_state(Java26BuiltinDimension::Overworld, pos) else {
                    continue;
                };
                let Some(identity) =
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                else {
                    continue;
                };
                if identity.name == "minecraft:resin_clump" {
                    assert!(
                        identity.properties.values().any(|value| value == "true"),
                        "a placed clump has a face: {:?}",
                        identity.properties
                    );
                    placed.push(pos);
                }
            }
        }
    }
    assert!(!placed.is_empty(), "the hit spreads resin onto the logs");
}

/// The same absorption from the other stored-damage entries: a `/damage`
/// command with a player behind it is swallowed, and one with neither a living
/// direct cause nor a responsible player is not.
#[test]
fn a_command_hit_follows_the_same_absorption_rules() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));

    let health = |world: &ServerWorld| {
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, protector_id)
            .and_then(crate::entity::Entity::living)
            .map(crate::entity::LivingEntityState::health)
    };
    let before = health(&world);
    let player_uuid = Uuid::from_u128(1);
    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            protector_id,
            "minecraft:generic",
            5.0,
            Some(crate::commands::CommandDamageSourceEntity::Player {
                user: crate::admin::NameAndId {
                    id: player_uuid,
                    name: "alice".into(),
                }
                .into(),
                entity_id: 2,
                position: Vec3d::default(),
                instabuild: false,
                may_build: true,
                spectator: false,
                display_name: bbb_protocol::NbtSummaryValue::string("alice"),
            }),
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        }),
    );
    assert_eq!(health(&world), before, "the player's damage is swallowed");

    // No direct living or projectile cause and no responsible player: the
    // branch answers false, so the hit does not land
    // and `DamageCommand` reports nothing hurt.
    let hurt = world.hurt_stored_entity_by_command_for_test(
        Java26BuiltinDimension::Overworld,
        protector_id,
        "minecraft:generic",
        5.0,
        None,
    );
    assert_eq!(
        hurt,
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        })
    );
    assert_eq!(
        health(&world),
        before,
        "an unattributed hit does nothing either"
    );

    // `:154-156`: a source that bypasses invulnerability still runs `super`.
    let hurt = world.hurt_stored_entity_by_command_for_test(
        Java26BuiltinDimension::Overworld,
        protector_id,
        "minecraft:generic_kill",
        5.0,
        None,
    );
    assert!(hurt.is_some(), "the bypassing damage type is applied");
    assert_eq!(health(&world), Some(0.0), "`/kill` still kills it");
}

/// The projectile entry: an arrow a player fired is a living-or-projectile
/// direct cause with a responsible player, so the protector swallows it and
/// its heart answers.
#[test]
fn a_players_arrow_is_swallowed_too() {
    let (_sender, receiver, mut world) = heart_world(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..datapack_config()
        },
        13_000,
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));

    let dimension = Java26BuiltinDimension::Overworld;
    let registry_tags =
        crate::entity::EntityRegistryTagView::Frozen(world.level_registry_tags_for_test(dimension));
    world.queue_projectile_action_for_test(
        dimension,
        PendingProjectileAction::Hurt(PendingProjectileHurt {
            target: PendingHurtEntity::Stored {
                entity_id: protector_id,
            },
            damage_type: "minecraft:arrow",
            amount: 4.0,
            causing: Some(PendingHurtEntity::Player {
                uuid: Uuid::from_u128(1),
            }),
            direct: Some(PendingHurtEntity::Player {
                uuid: Uuid::from_u128(1),
            }),
            direct_resource_id: "minecraft:player",
            registry_tags,
        }),
    );
    let before = world
        .stored_entity_for_test(dimension, protector_id)
        .and_then(crate::entity::Entity::living)
        .map(crate::entity::LivingEntityState::health);
    world.tick(receiver.drain());
    world.tick(receiver.drain());

    assert_eq!(
        world
            .stored_entity_for_test(dimension, protector_id)
            .and_then(crate::entity::Entity::living)
            .map(crate::entity::LivingEntityState::health),
        before,
        "the arrow's damage never lands"
    );
}

/// The protector's own half of the binding: `Creaking.tick`
/// kills it once the heart at its home position no
/// longer names it, and `setTransient` is what put the home there.
#[test]
fn a_protector_dies_when_its_heart_stops_protecting_it() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let bound = protector_uuid(&mut world, at).expect("the heart bound a protector");
    let (id, _) = creakings(&world)
        .into_iter()
        .find(|(_, uuid)| *uuid == bound)
        .expect("the protector is live");
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .and_then(crate::entity::Entity::creaking_home_pos),
        Some(at),
        "`setTransient` binds the protector to the heart's cell"
    );

    // The heart drops the binding while its block entity stays: the awake
    // heart then raises a second protector, and the first one is no longer the
    // heart's protector.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
        .expect("the heart block entity")
        .clear_protector();
    world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .expect("the protector is still in the store while it dies")
            .living()
            .map(crate::entity::LivingEntityState::health),
        Some(0.0),
        "`setHealth(0)` at the head of the tick kills the unbound protector"
    );
    // `LivingEntity.tickDeath` removes it after its 20-tick death animation
    // (`Creaking.tickDeath` takes the super path while it is not tearing down).
    for _ in 0..24 {
        world.tick(receiver.drain());
    }
    let live = creakings(&world);
    assert!(
        !live.iter().any(|(_, uuid)| *uuid == bound),
        "the unbound protector is gone: {live:?}"
    );
}

/// The binding round-trips a save: the record carries the protector UUID, and
/// a world restored from that record resumes with the binding and clears it
/// through the grace period once the protector is gone.
#[test]
fn the_binding_survives_a_save_and_the_countdown_resumes() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let bound = protector_uuid(&mut world, at).expect("the heart bound a protector");

    let request = world
        .save_request(1_786_000_000_777, 300, RegionCompression::Zlib)
        .expect("the save is produced");
    // A created world publishes whole chunk documents rather than per-cell
    // edits, so the heart's record is read out of the chunk the save writes.
    let written = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the heart's chunk is saved");
    let record = written
        .document
        .entries()
        .iter()
        .find(|entry| entry.name == "block_entities")
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::List(values) => values.iter().find_map(|value| {
                let NbtSummaryValue::Compound(entries) = value else {
                    return None;
                };
                let position = |name: &str| {
                    entries.iter().find_map(|entry| {
                        (entry.name == name).then_some(match entry.value {
                            NbtSummaryValue::Int(value) => Some(value),
                            _ => None,
                        })?
                    })
                };
                (position("x") == Some(at.x)
                    && position("y") == Some(at.y)
                    && position("z") == Some(at.z))
                .then(|| entries.clone())
            }),
            _ => None,
        })
        .expect("the heart's record is saved");
    let creaking = record
        .iter()
        .find(|entry| entry.name == "creaking")
        .expect("the record carries the binding");
    let NbtSummaryValue::IntArray(ints) = &creaking.value else {
        panic!("the binding is an int array: {:?}", creaking.value);
    };
    assert_eq!(ints.len(), 4, "{ints:?}");
    let [first, second, third, fourth] = <[i32; 4]>::try_from(ints.clone()).expect("four ints");
    let (high, low) = bound.as_u64_pair();
    assert_eq!(
        [first, second, third, fourth],
        [
            (high >> 32) as i32,
            high as i32,
            (low >> 32) as i32,
            low as i32
        ]
    );

    // A restored chunk carries the same states and the saved record, with no
    // live protector behind the UUID.
    let mut raw = restore_fixture_chunk_raw(vec![
        restore_section_with_cells(
            3,
            "minecraft:air",
            &[([0, 15, 1], "minecraft:pale_oak_log", &[("axis", "y")])],
        ),
        restore_section_with_cells(
            4,
            "minecraft:air",
            &[
                (
                    [0, 0, 1],
                    "minecraft:creaking_heart",
                    &[
                        ("axis", "y"),
                        ("creaking_heart_state", "awake"),
                        ("natural", "false"),
                    ],
                ),
                ([0, 1, 1], "minecraft:pale_oak_log", &[("axis", "y")]),
            ],
        ),
    ]);
    for entry in raw.entries_mut() {
        if entry.name == "block_entities" {
            entry.value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(record.clone())]);
        }
    }
    let chunk = bbb_server_persistence::Java26ChunkData::parse(raw, restore_chunk_context())
        .expect("the restored chunk parses");
    let candidate = restore_candidate(13_000, Some(chunk), Vec::new());
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    assert_eq!(
        protector_uuid(&mut restored, at),
        Some(bound),
        "the restored heart keeps the binding"
    );
    // `getCreakingProtector` (`:188-190`): a UUID that names nobody for the
    // grace period is dropped.
    for _ in 0..TICKS_GRACE_PERIOD {
        restored.tick(Vec::new());
    }
    assert_eq!(
        protector_uuid(&mut restored, at),
        None,
        "the restored countdown clears the binding"
    );
}

/// The resin clumps `spreadResin` (`:245-278`) placed around the heart.
fn resin_clumps(world: &mut ServerWorld, at: BlockPos) -> Vec<BlockPos> {
    let mut placed = Vec::new();
    for x in at.x - 2..=at.x + 2 {
        for y in at.y - 2..=at.y + 2 {
            for z in at.z - 2..=at.z + 2 {
                let pos = BlockPos { x, y, z };
                let Some(state) = world.block_state(Java26BuiltinDimension::Overworld, pos) else {
                    continue;
                };
                let Some(identity) =
                    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                else {
                    continue;
                };
                if identity.name == "minecraft:resin_clump" {
                    placed.push(pos);
                }
            }
        }
    }
    placed
}

/// `Creaking.setTransient`'s `HOME_POS` write (`Creaking`)
/// reaches the client as synched data, which is what
/// `Creaking.getHurtSound` reads on the client for event 66.
#[test]
fn the_spawned_protector_publishes_its_binding() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    let mut published = Vec::new();
    for _ in 0..4 {
        published.extend(world.tick(receiver.drain()).events);
    }
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));
    let row = published
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } if update.id == protector_id => update
                .values
                .iter()
                .find(|value| value.data_id == VANILLA_CREAKING_HOME_POS_DATA_ID),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the binding is not published: {published:?}"));
    assert_eq!(
        row.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID
    );
    assert_eq!(row.value, EntityDataValueKind::OptionalBlockPos(Some(at)));
}

/// `Creaking.hurtServer`'s responsible-player gate
/// over the real projectile entry: a living direct cause with no player behind
/// it is still swallowed and still broadcasts the animation, but the heart
/// never answers.
#[test]
fn a_mobs_hit_is_swallowed_without_reaching_the_heart() {
    let (_sender, receiver, mut world) = heart_world(datapack_config(), 13_000);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));

    let dimension = Java26BuiltinDimension::Overworld;
    // A zombie's `mobAttack` is a living direct cause with no responsible
    // player (`LivingEntity.resolvePlayerResponsibleForDamage`).
    let (zombie_id, _) = world.spawn_stored_entity_for_test(
        dimension,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                .expect("zombie is living"),
        ),
    );
    let registry_tags =
        crate::entity::EntityRegistryTagView::Frozen(world.level_registry_tags_for_test(dimension));
    world.queue_projectile_action_for_test(
        dimension,
        PendingProjectileAction::Hurt(PendingProjectileHurt {
            target: PendingHurtEntity::Stored {
                entity_id: protector_id,
            },
            damage_type: "minecraft:mob_attack",
            amount: 4.0,
            causing: None,
            direct: Some(PendingHurtEntity::Stored {
                entity_id: zombie_id,
            }),
            direct_resource_id: "minecraft:zombie",
            registry_tags,
        }),
    );
    let before = world
        .stored_entity_for_test(dimension, protector_id)
        .and_then(crate::entity::Entity::living)
        .map(crate::entity::LivingEntityState::health);
    let hurt = world.tick(receiver.drain());

    assert_eq!(
        world
            .stored_entity_for_test(dimension, protector_id)
            .and_then(crate::entity::Entity::living)
            .map(crate::entity::LivingEntityState::health),
        before,
        "the hit is swallowed"
    );
    assert!(
        hurt.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(event),
                ..
            } if event.entity_id == protector_id && event.event_id == 66
        )),
        "the animation is not gated: {:?}",
        hurt.events
    );
    let resin_sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:block.resin.place",
    )
    .expect("the sound is in the catalog");
    assert!(
        !hurt.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } if matches!(
                &sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
                    if *id == resin_sound
            )
        )),
        "the heart never answers: {:?}",
        hurt.events
    );
    assert!(
        resin_clumps(&mut world, at).is_empty(),
        "no resin is spread without a responsible player"
    );
}

/// A fixture world with one awake heart at `at` and the protector it bound.
fn world_with_protector(
    config: ServerWorldConfig,
    at: BlockPos,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver, mut world) = heart_world(config, 13_000);
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));
    (sender, receiver, world, protector_id)
}

fn creaking_health(world: &ServerWorld, entity_id: i32) -> f32 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .and_then(crate::entity::Entity::living)
        .map(crate::entity::LivingEntityState::health)
        .expect("the creaking is living")
}

/// The `CommandDamageSourceEntity::Player` a `/damage` names as its cause.
fn player_cause(instabuild: bool) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: Uuid::from_u128(1),
            name: "alice".into(),
        }
        .into(),
        entity_id: 2,
        position: Vec3d::default(),
        instabuild,
        may_build: true,
        spectator: false,
        display_name: bbb_protocol::NbtSummaryValue::string("alice"),
    }
}

/// A datapack that adds the protector's type to `#minecraft:fall_damage_immune`,
/// which is the only way `Entity.isInvulnerableToBase` can
/// answer for a creaking.
fn fall_immune_config(root: &std::path::Path) -> ServerWorldConfig {
    let pack = root.join("datapacks/fall_immune");
    std::fs::create_dir_all(pack.join("data/minecraft/tags/entity_type")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"test","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(
        pack.join("data/minecraft/tags/entity_type/fall_damage_immune.json"),
        serde_json::to_vec(&serde_json::json!({ "values": ["minecraft:creaking"] })).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    }
}

/// `Entity.isInvulnerableToBase` exempts a creative
/// player from the invulnerable term, so a creative attacker's swing still runs
/// the heart-bound branch and the protector reacts.
#[test]
fn a_creative_attackers_swing_is_absorbed() {
    let (sender, receiver, mut world) = heart_world(
        ServerWorldConfig {
            game_type: GameType::Creative,
            ..datapack_config()
        },
        13_000,
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    place_heart(&mut world, at);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    let (protector_id, bound) = creakings(&world).into_iter().next().expect("the protector");
    assert_eq!(protector_uuid(&mut world, at), Some(bound));

    let before = creaking_health(&world, protector_id);
    let protector_position = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, protector_id)
        .expect("the protector is live")
        .position();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: protector_position.x + 1.5,
            y: protector_position.y,
            z: protector_position.z,
        },
    );
    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: protector_id,
        }),
    );
    assert_eq!(
        creaking_health(&world, protector_id),
        before,
        "the heart-bound protector takes no damage"
    );
    assert!(
        attacked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(event),
                ..
            } if event.entity_id == protector_id && event.event_id == 66
        )),
        "the hit animation reaches the client: {:?}",
        attacked.events
    );
    let sway = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.creaking.sway",
    )
    .expect("the sound is in the catalog");
    assert!(
        attacked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound),
                ..
            } if matches!(
                &sound.sound,
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id: id }
                    if *id == sway
            )
        )),
        "the heart answers the creative attacker: {:?}",
        attacked.events
    );
}

/// `IS_FIRE && fireImmune()`: a heart-bound protector is
/// fire-immune, so fire is refused while the same
/// player's ordinary damage is still absorbed.
#[test]
fn fire_damage_is_refused_by_a_fire_immune_protector() {
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let (_sender, _receiver, mut world, protector_id) = world_with_protector(datapack_config(), at);
    let dimension = Java26BuiltinDimension::Overworld;
    let before = creaking_health(&world, protector_id);

    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            dimension,
            protector_id,
            "minecraft:on_fire",
            5.0,
            Some(player_cause(false)),
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        }),
        "`minecraft:on_fire` is `#minecraft:is_fire` and the protector is fire-immune"
    );
    assert_eq!(creaking_health(&world, protector_id), before);

    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            dimension,
            protector_id,
            "minecraft:generic",
            5.0,
            Some(player_cause(false)),
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        }),
        "the same attacker's non-fire damage is still swallowed"
    );
    assert_eq!(creaking_health(&world, protector_id), before);
}

/// `IS_FALL && this.is(EntityTypeTags.FALL_DAMAGE_IMMUNE)`
/// a protector is not fall-immune by type, so a
/// datapack that adds `minecraft:creaking` to the tag is what turns the same
/// player-caused fall from an absorbed hit into a refusal.
#[test]
fn fall_damage_is_refused_only_while_the_protector_is_fall_immune() {
    let root =
        std::env::temp_dir().join(format!("bbb-creaking-fall-immune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let dimension = Java26BuiltinDimension::Overworld;

    let (_sender, _receiver, mut world, protector_id) = world_with_protector(datapack_config(), at);
    let before = creaking_health(&world, protector_id);
    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            dimension,
            protector_id,
            "minecraft:fall",
            5.0,
            Some(player_cause(false)),
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        }),
        "without the tag the fall reaches the branch and is swallowed"
    );
    assert_eq!(creaking_health(&world, protector_id), before);

    let (_sender, _receiver, mut world, protector_id) =
        world_with_protector(fall_immune_config(&root), at);
    let before = creaking_health(&world, protector_id);
    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            dimension,
            protector_id,
            "minecraft:fall",
            5.0,
            Some(player_cause(false)),
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        }),
        "the tag makes the protector invulnerable to `#minecraft:is_fall`"
    );
    assert_eq!(creaking_health(&world, protector_id), before);
    let _ = std::fs::remove_dir_all(&root);
}

/// `Creaking`: a creaking no heart bound has no branch to answer
/// with, so `super.hurtServer` runs and the damage lands.
#[test]
fn an_unbound_creaking_takes_the_command_damage() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let (creaking_id, _) = world.spawn_stored_entity_for_test(
        dimension,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                .expect("creaking is living"),
        ),
    );
    let before = creaking_health(&world, creaking_id);
    assert_eq!(
        world.hurt_stored_entity_by_command_for_test(
            dimension,
            creaking_id,
            "minecraft:generic",
            // A creaking's whole health bar is one point (`Creaking` keeps the
            // `Monster` default of 1.0), so a five-point hit would only prove
            // that it died.
            0.5,
            None,
        ),
        Some(crate::entity::StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        })
    );
    assert_eq!(creaking_health(&world, creaking_id), before - 0.5);
}
