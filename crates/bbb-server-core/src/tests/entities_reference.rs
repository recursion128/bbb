//! `EntityReference` across the levels a server runs
//! (`ServerLevel.getEntityInAnyDimension`, `ServerLevel.java:1378-1394`).
//!
//! A saved reference is one UUID, and which entity it names is the lookup's
//! answer. The three cases here are the two entity-side readers of that answer
//! -- the tick's death clear (`LivingEntity.java:470-477`) and the kill credit
//! `createWitherRose` reads (`:1429`, `:1449`, `:1946-1952`) -- with the target
//! standing in the level beside the victim's, over a real save and reload.

use super::*;

use crate::entity::{EntityData, GeneralLivingData, LastHurtByMob, StoredEntityTransition};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use uuid::Uuid;

const SOURCE: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const TARGET: Java26BuiltinDimension = Java26BuiltinDimension::Nether;
/// The cell the fixture mobs stand in, which `createWitherRose` recentres to
/// (`LivingEntity.java:1464-1469`).
const MOB_POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
const MOB_BLOCK: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("bbb-{name}-{}", std::process::id()))
}

fn spawn_mob(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    entity_type_id: i32,
    position: Vec3d,
) -> i32 {
    let data = EntityData::GeneralLiving(
        GeneralLivingData::new(entity_type_id).expect("the fixture type is living"),
    );
    world
        .spawn_stored_entity_for_test(dimension, entity_type_id, position, data)
        .0
}

fn stored_uuid(world: &ServerWorld, dimension: Java26BuiltinDimension, entity_id: i32) -> Uuid {
    world
        .stored_entity_for_test(dimension, entity_id)
        .expect("the fixture entity is stored")
        .uuid()
}

fn stored_living(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
    entity_id: i32,
) -> &crate::entity::LivingEntityState {
    world
        .stored_entity_for_test(dimension, entity_id)
        .and_then(crate::entity::Entity::living)
        .expect("the fixture entity is living")
}

/// What one fixture mob remembers, as the record and the lookup leave it.
fn remembered(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
    entity_id: i32,
) -> Option<LastHurtByMob> {
    stored_living(world, dimension, entity_id).last_hurt_by_mob()
}

fn cross_level_transition(position: Vec3d) -> StoredEntityTransition {
    StoredEntityTransition {
        position,
        delta_movement: Vec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        relatives_mask: 0,
    }
}

/// The `CommandDamageSourceEntity` a `/damage ... by <stored mob>` resolves to.
fn stored_mob_damage_source(
    uuid: Uuid,
    entity_id: i32,
    entity_resource_id: &str,
) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Stored {
        entity_id,
        uuid: Some(uuid),
        position: Some(Vec3d::default()),
        entity_resource_id: entity_resource_id.to_string(),
        is_mob: true,
        is_living_non_player: true,
        display_name: NbtSummaryValue::string(entity_resource_id),
    }
}

/// The `CommandDamageSourceEntity` a `/damage ... by <player>` resolves to.
fn player_damage_source(uuid: Uuid, entity_id: i32) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Player {
        user: crate::admin::NameAndId {
            id: uuid,
            name: "alice".into(),
        }
        .into(),
        entity_id,
        position: Vec3d::default(),
        instabuild: false,
        may_build: true,
        spectator: false,
        display_name: NbtSummaryValue::string("alice"),
    }
}

/// The victim in the overworld remembers a blow; the attacker leaves for the
/// nether before the save, and the world is reloaded from a real
/// `Java26WorldStorage` round trip.
#[test]
fn a_cross_dimension_last_hurt_by_mob_reference_clears_the_memory_when_its_target_dies() {
    let root = fixture_root("bbb-entity-cross-level-clear");
    let storage = created_fixture_save(&root, "bbb-entity-cross-level-clear");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    };
    let mut world = crate::tests::restore_stored(config.clone(), &candidate);
    let victim = spawn_mob(
        &mut world,
        SOURCE,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let attacker = spawn_mob(
        &mut world,
        SOURCE,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let victim_uuid = stored_uuid(&world, SOURCE, victim);
    let attacker_uuid = stored_uuid(&world, SOURCE, attacker);

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            SOURCE,
            victim,
            "minecraft:generic",
            1.0,
            Some(stored_mob_damage_source(
                attacker_uuid,
                attacker,
                "minecraft:zombie",
            )),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        remembered(&world, SOURCE, victim).and_then(LastHurtByMob::uuid),
        Some(attacker_uuid)
    );

    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (0, 0));
    world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            attacker,
            cross_level_transition(Vec3d {
                x: 3.5,
                y: 70.0,
                z: 3.5,
            }),
        )
        .unwrap();
    assert!(world.stored_entity_for_test(SOURCE, attacker).is_none());

    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = forceload_target_origin(storage.load_candidate().unwrap());
    let mut restored = crate::tests::restore_stored(config, &saved);
    let reloaded_victim = restored
        .live_entity_id_for_uuid_for_test(SOURCE, victim_uuid)
        .expect("the victim reloads in the level it stayed in");
    let reloaded_attacker = restored
        .live_entity_id_for_uuid_for_test(TARGET, attacker_uuid)
        .expect("the attacker reloads in the dimension it moved to");
    assert_eq!(
        remembered(&restored, SOURCE, reloaded_victim),
        Some(LastHurtByMob::Uuid(attacker_uuid)),
        "the record carries the UUID alone, so the lookup is what places it"
    );

    // A session keeps the victim's column entity-ticking, which a fixture save
    // files no ticket for.
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    restored.tick(receiver.drain());
    assert_eq!(
        remembered(&restored, SOURCE, reloaded_victim),
        Some(LastHurtByMob::Uuid(attacker_uuid)),
        "the target is alive in the nether, well inside the window"
    );

    let outcome = restored
        .hurt_stored_entity_by_command_for_test(
            TARGET,
            reloaded_attacker,
            "minecraft:generic",
            40.0,
            None,
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    assert!(
        stored_living(&restored, TARGET, reloaded_attacker).is_dead_or_dying(),
        "the attacker died in the nether"
    );

    restored.tick(receiver.drain());
    assert_eq!(
        remembered(&restored, SOURCE, reloaded_victim),
        None,
        "the death clear fires for a target standing in the dimension beside it"
    );

    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// A cow in the overworld remembers a wither's blow; the wither moves to the
/// nether, the world is saved and reopened, and the kill that follows has no
/// attacker behind it, so the only credit there is comes from the reference.
#[test]
fn a_cross_dimension_wither_kill_credit_places_the_rose_after_a_real_restart() {
    let root = fixture_root("bbb-entity-cross-level-rose");
    let storage = created_fixture_save(&root, "bbb-entity-cross-level-rose");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    };
    let mut world = crate::tests::restore_stored(config.clone(), &candidate);
    let cow = spawn_mob(&mut world, SOURCE, VANILLA_ENTITY_TYPE_COW_ID, MOB_POSITION);
    let wither = spawn_mob(
        &mut world,
        SOURCE,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let cow_uuid = stored_uuid(&world, SOURCE, cow);
    let wither_uuid = stored_uuid(&world, SOURCE, wither);

    // The cell the rose goes on: air over a block `supports_wither_rose` holds.
    assert!(world.set_block(
        SOURCE,
        BlockPos { x: 0, y: 63, z: 0 },
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, MOB_BLOCK),
        Some(block_state_id("minecraft:air"))
    );

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            SOURCE,
            cow,
            "minecraft:generic",
            1.0,
            Some(stored_mob_damage_source(
                wither_uuid,
                wither,
                "minecraft:wither",
            )),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        remembered(&world, SOURCE, cow),
        Some(LastHurtByMob::Stored {
            entity_id: wither,
            uuid: Some(wither_uuid),
        })
    );

    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (0, 0));
    world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            wither,
            cross_level_transition(Vec3d {
                x: 3.5,
                y: 70.0,
                z: 3.5,
            }),
        )
        .unwrap();

    let request = world
        .save_request(1_786_000_004_001, 205, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = forceload_target_origin(storage.load_candidate().unwrap());
    let mut restored = crate::tests::restore_stored(config, &saved);
    let reloaded_cow = restored
        .live_entity_id_for_uuid_for_test(SOURCE, cow_uuid)
        .expect("the cow reloads in the level it stayed in");
    assert_eq!(
        remembered(&restored, SOURCE, reloaded_cow),
        Some(LastHurtByMob::Uuid(wither_uuid)),
        "the record carries the UUID alone, so the lookup is what places it"
    );
    assert!(
        restored
            .live_entity_id_for_uuid_for_test(TARGET, wither_uuid)
            .is_some(),
        "the wither reloads in the dimension it moved to"
    );
    assert_eq!(
        live_block(&mut restored, MOB_BLOCK),
        Some(block_state_id("minecraft:air")),
        "the cell the rose goes on round-trips too"
    );

    let outcome = restored
        .hurt_stored_entity_by_command_for_test(
            SOURCE,
            reloaded_cow,
            "minecraft:generic",
            40.0,
            None,
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        live_block(&mut restored, MOB_BLOCK),
        Some(block_state_id("minecraft:wither_rose")),
        "`createWitherRose` placed the rose for a killer in the nether"
    );
    assert!(
        !restored
            .stored_items_for_test(SOURCE)
            .iter()
            .any(|item| item.2 == "minecraft:wither_rose"),
        "the block arm leaves no rose item"
    );

    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// A session is a stored entity of its own level in Vanilla
/// (`ServerLevel.addPlayer`, `ServerLevel.java:969-978`), so a reference that
/// names one resolves through the level it now stands in.
#[test]
fn a_session_target_that_moved_to_another_dimension_clears_the_memory_when_it_dies() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1), ("bob", 2)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let floor = bbb_server_types::fixture::FlatFixture::new()
        .unwrap()
        .floor_y();
    let victim = spawn_mob(
        &mut world,
        SOURCE,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 0.5,
            y: f64::from(floor + 1),
            z: 0.5,
        },
    );
    let alice = Uuid::from_u128(1);
    let alice_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            SOURCE,
            victim,
            "minecraft:generic",
            1.0,
            Some(player_damage_source(alice, alice_entity_id)),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
    assert_eq!(
        remembered(&world, SOURCE, victim),
        Some(LastHurtByMob::Uuid(alice)),
        "a session is not a stored entity, so the reference holds its UUID"
    );

    // Alice leaves the overworld. Bob stays, so the victim's column keeps
    // ticking while the session it remembers stands in the nether.
    let mut moved = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: TARGET.into(),
            position: Vec3d {
                x: 8.5,
                y: 70.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut moved,
    ));
    // `ServerPlayer.isInvulnerableTo`'s dimension-change arm
    // (`ServerPlayer.java:1266`) stays open until the session acknowledges the
    // position the transition landed it at, which is what gates `/damage`
    // below (`commands.damage.invulnerable`).
    let teleport_id = moved
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(update.id),
            _ => None,
        })
        .expect("a cross-level transition sends the position it landed at");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    world.tick(receiver.drain());
    assert_eq!(
        remembered(&world, SOURCE, victim),
        Some(LastHurtByMob::Uuid(alice)),
        "alice is alive in the nether, well inside the window"
    );

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    let health = world.player_health_for_test(ConnectionId(1));
    assert!(
        health <= 0.0,
        "alice died in the nether: health {health}, chats {:?}, healths {:?}",
        system_chats(&killed),
        set_healths(&killed)
    );
    world.tick(receiver.drain());
    assert_eq!(
        remembered(&world, SOURCE, victim),
        None,
        "the death clear fires for a session standing in the dimension beside it"
    );
}

/// The other arm of the same lookup: `getKillCredit` resolves a remembered mob
/// through `getEntityInAnyDimension` (`EntityReference.java:78-82`,
/// `ServerLevel.java:1378-1394`), which walks every level, so the mob that
/// softened a session is still its killer after walking into another dimension.
#[test]
fn a_mob_that_moved_to_another_dimension_is_still_the_session_killer() {
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let zombie = spawn_mob(
        &mut world,
        SOURCE,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let zombie_uuid = stored_uuid(&world, SOURCE, zombie);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage alice 1 minecraft:generic by {zombie_uuid}"),
    );

    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (0, 0));
    world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            zombie,
            cross_level_transition(Vec3d {
                x: 3.5,
                y: 70.0,
                z: 3.5,
            }),
        )
        .unwrap();
    assert!(world.stored_entity_for_test(SOURCE, zombie).is_none());

    let killed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "damage alice 40 minecraft:generic",
    );
    let health = world.player_health_for_test(ConnectionId(1));
    assert!(health <= 0.0, "alice died: {:?}", set_healths(&killed));
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID
            )
        ),
        1,
        "`ServerPlayer.die` (`:913-914`) counts the credited mob's own type"
    );
}

/// The target column as `/forceload add 0 0` in the nether leaves it on disk,
/// so the entity there is loaded once the world restarts.
fn forceload_target_origin(
    mut candidate: bbb_server_persistence::Java26WorldCandidate,
) -> bbb_server_persistence::Java26WorldCandidate {
    candidate.chunk_tickets.insert(
        Java26LevelKey::from(TARGET),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    candidate
}
