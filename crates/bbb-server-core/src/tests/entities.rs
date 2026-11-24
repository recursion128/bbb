//! Stored entities: the item entities a drop makes, the tracker they reach, and
//! the pickup box that hands them to a player.

use super::*;

mod allay_delivery;
mod axolotl;
mod baby_passengers;
mod bogged_shearing;
mod breed_goal;
mod bucket_capture;
mod camel_dash;
mod cat_beds;
mod copper_golem_shearing;
mod copper_golem_transport;
mod creeper_ignition;
mod dolphin_follow_vehicle;
mod enderman_blocks;
mod feeding;
mod fox_attention;
mod fox_environment;
mod fox_family;
mod fox_pounce;
mod fox_seek_shelter;
mod fox_sleep;
mod fox_village;
mod happy_ghast_flight;
mod happy_ghast_interaction;
mod happy_ghast_juvenile;
mod horse_interaction;
mod mob_doors;
mod mooshroom_shearing;
mod mooshroom_stew;
mod movement_game_events;
mod nautilus_breath;
mod nautilus_dash;
mod nautilus_interaction;
mod panda_lie;
mod panda_roll;
mod panda_sit;
mod panda_sneeze;
mod patrol_hold_ground;
mod pet_death_message;
mod player_melee;
mod post_attack;
mod projectile_rules;
mod rabbit_foraging;
mod ridden_mount_moves;
mod saddled_mounts;
mod sheep_shearing;
mod silverfish;
mod skeleton_trap;
mod sleeping_passengers;
mod sniffer_digging;
mod snow_golem_shearing;
mod summon_team;
mod tame_sit;
mod thrown_projectile_hit;
mod villager_job_site;
mod villager_life;
mod villager_trades;
mod warden_combat;
mod zombie_cure;

use crate::entity::hurting_projectile::fireball::LargeFireballData;
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item::{plain_stack, ItemEntityData};
use crate::entity::{GeneralLivingData, StoredEntityTransition};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::{RecordedGameEvent, VanillaGameEvent};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_server_types::item_stack::RuntimeItemStack;

fn install_redirectable_projectiles(world: &mut ServerWorld, members: &[&str]) {
    let snapshot = crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_string(),
        "minecraft:redirectable_projectile".to_string(),
        members.iter().map(|member| (*member).to_string()).collect(),
    )]);
    world.install_level_registry_tags(Arc::new(snapshot));
}

/// Restores the entity-region writes of one live world into the matching
/// candidate levels, as if the save had been closed and reopened.
pub(crate) fn restore_entity_writes(
    candidate: &mut Java26WorldCandidate,
    writes: &[bbb_server_persistence::Java26EntityChunkWrite],
) {
    for write in writes {
        let data =
            Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
                .unwrap();
        candidate
            .dimensions
            .get_mut(&write.dimension)
            .expect("the candidate declares the entity write's level")
            .entity_chunks
            .insert(
                (write.chunk_x, write.chunk_z),
                Java26LoadedEntityChunk {
                    timestamp: 100,
                    data,
                },
            );
    }
}

/// `ChunkMap` belongs to one `ServerLevel` (`ChunkMap.java:1186`), so a dropped
/// item is announced to the sessions watching that level and to nobody else.
///
/// This is the wiring test for the tracker: the pairing rules themselves are
/// covered in `entity::tracking`, and what is under test here is that a stack
/// which only existed in the save until now reaches a real session, and that a
/// session standing in another dimension hears none of it.
#[test]
fn a_dropped_item_is_announced_to_the_session_watching_its_level_and_to_no_other() {
    let (sender, receiver) = mailbox();
    for (connection, name, uuid) in [(1_u64, "alice", 1_u128), (2, "bob", 2)] {
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: Default::default(),
                context: JoinContext::default(),
                connection_id: ConnectionId(connection),
                profile: profile(name, uuid),
            },
        )
        .unwrap();
    }
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    world.tick(receiver.drain());
    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(2),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 1.5,
                y: 70.0,
                z: 1.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    // `ChunkMap.isChunkTracked` (`ChunkMap.java:227-229`) refuses a column the
    // session is still queued to receive, so both views have to settle first.
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 2);

    dandelion_on_dirt(&mut world);
    remove_the_support(&mut world);
    let tick = world.tick(receiver.drain());

    let item_adds: Vec<(ConnectionId, bbb_protocol::packets::AddEntity)> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID => {
                Some((*connection_id, add.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        item_adds.len(),
        1,
        "exactly the overworld session is told: {item_adds:?}"
    );
    let (told, added) = &item_adds[0];
    assert_eq!(*told, ConnectionId(1));
    // `Block.popResource` drops the stack inside the column the plant stood in.
    assert_eq!(
        (
            added.position.x.floor() as i32,
            added.position.z.floor() as i32
        ),
        (1, 1)
    );

    let told_packets: Vec<&ClientboundPlayPacket> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet,
            } => Some(packet),
            _ => None,
        })
        .collect();
    let add_index = told_packets
        .iter()
        .position(|packet| {
            matches!(
                packet,
                ClientboundPlayPacket::AddEntity(add) if add.id == added.id
            )
        })
        .expect("the item add packet is in the overworld session's stream");
    assert!(matches!(
        told_packets.get(add_index.wrapping_sub(1)),
        Some(ClientboundPlayPacket::BundleDelimiter)
    ));
    assert!(matches!(
        told_packets.get(add_index + 1),
        Some(ClientboundPlayPacket::SetEntityData(data)) if data.id == added.id
    ));
    assert!(matches!(
        told_packets.get(add_index + 2),
        Some(ClientboundPlayPacket::BundleDelimiter)
    ));

    // `ServerEntity.sendPairingData` (`ServerEntity.java:277-279`): the stack
    // itself, without which the client draws an empty item.
    let stack = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetEntityData(data),
            } if data.id == added.id => Some(data.values.clone()),
            _ => None,
        })
        .expect("the item's synched stack is published with it");
    assert_eq!(stack.len(), 1);
    assert_eq!(
        stack[0].data_id,
        bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID
    );
    let EntityDataValueKind::ItemStack(item) = &stack[0].value else {
        panic!("the item entity's synched value is a stack: {stack:?}");
    };
    assert_eq!(
        item.item_id,
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:dandelion")
                .expect("the dandelion is an item")
                .id
        )
    );
    assert_eq!(item.count, 1);

    // The nether session is told about no entity of the overworld at all; its
    // own level's entities are its business.
    let overworld_ids = world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld);
    assert!(overworld_ids.contains(&added.id));
    let leaked: Vec<i32> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet,
            } => match packet {
                ClientboundPlayPacket::AddEntity(update) => Some(update.id),
                ClientboundPlayPacket::SetEntityData(update) => Some(update.id),
                ClientboundPlayPacket::MoveEntity(update) => Some(update.id),
                ClientboundPlayPacket::EntityPositionSync(update) => Some(update.id),
                ClientboundPlayPacket::SetEntityMotion(update) => Some(update.id),
                _ => None,
            },
            _ => None,
        })
        .filter(|id| {
            overworld_ids.contains(id)
                && world
                    .stored_entity_for_test(Java26BuiltinDimension::Nether, *id)
                    .is_none()
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "a session in another level hears none of this one's entities: {leaked:?}"
    );

    // The entry, scheduled/random, block-event, pickup and entity-loop drains
    // all see an empty journal after the first publication. None of those
    // later phase boundaries pairs the same live entity again.
    let settled = world.tick(receiver.drain());
    assert!(
        !settled.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } if add.id == added.id
        )),
        "phase drains do not repeat pairing: {settled:?}"
    );
}

#[test]
fn projectile_actions_deflect_before_damage_and_explode_before_removal() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "deflector", 0xdef1ec7);
    let owner = Uuid::from_u128(0xdef1ec7);
    let make_data = |owner| {
        crate::entity::EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(owner, 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        ))
    };
    let (target_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        Vec3d {
            x: 0.5,
            y: 66.0,
            z: 3.0,
        },
        make_data(None),
    );
    let (source_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        Vec3d {
            x: 0.5,
            y: 66.0,
            z: 1.0,
        },
        make_data(Some(owner)),
    );

    world.tick(receiver.drain());
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, source_id)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        });
    world.tick(receiver.drain());

    let target = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, target_id)
        .expect("the redirected target survives hurting-projectile damage");
    let crate::entity::EntityData::LargeFireball(target_data) = target.data() else {
        unreachable!()
    };
    assert_eq!(target_data.projectile().owner(), Some(owner));
    assert_eq!(
        target_data.projectile().acceleration_power(),
        HurtingProjectileState::INITIAL_ACCELERATION_POWER
    );
    assert!(target.delta_movement().x.abs() < 1e-6);
    assert!(
        target.delta_movement().y > 0.0,
        "explosion knockback aims from center.y to target.getEyePosition().y (ServerExplosion.java:186): {:?}",
        target.delta_movement()
    );
    assert!(
        target.delta_movement().z > 1.0,
        "AIM_DEFLECT installs +Z before the following explosion adds knockback: {:?}",
        target.delta_movement()
    );
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, source_id)
            .is_none(),
        "discard runs after the ordered side effects"
    );
}

#[test]
fn live_living_tick_applies_blocks_crossed_during_travel() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "walker", 0x0005_e3e7);
    let dimension = Java26BuiltinDimension::Overworld;
    let player = world.publication().players[0].position;
    let start = Vec3d {
        x: player.x - 3.5,
        y: player.y,
        z: player.z,
    };
    let rose = BlockPos {
        x: start.x.floor() as i32 + 2,
        y: start.y.floor() as i32,
        z: start.z.floor() as i32,
    };
    assert!(world.set_block(
        dimension,
        BlockPos {
            y: rose.y - 1,
            ..rose
        },
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        dimension,
        rose,
        block_state_id("minecraft:wither_rose"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow_id, _) = world.spawn_stored_entity_for_test(
        dimension,
        cow_type,
        start,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    world
        .stored_entity_mut_for_test(dimension, cow_id)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 4.0,
            y: 0.0,
            z: 0.0,
        });

    world.tick(receiver.drain());

    let cow = world.stored_entity_for_test(dimension, cow_id).unwrap();
    assert!(
        cow.bounding_box().min_x > f64::from(rose.x + 1),
        "the final box must have left the crossed cell: {:?}",
        cow.bounding_box()
    );
    let wither = cow
        .living()
        .unwrap()
        .active_effect("minecraft:wither")
        .expect("LivingEntity applies block effects after travel");
    assert_eq!(wither.duration(), 40);
}

fn ground_navigation_fixture() -> (ServerWorld, i32, BlockPos) {
    let chunk = restore_chunk_with_ticks(Vec::new(), Vec::new(), Vec::new());
    let candidate = restore_candidate(100, Some(chunk), Vec::new());
    let mut world = spawn_pinned_restore(creative_fixture_config(), &candidate);
    let dimension = Java26BuiltinDimension::Overworld;
    for x in 1..=11 {
        for z in 1..=7 {
            assert!(world.set_block(
                dimension,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            ));
        }
    }
    let witch_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
    let (witch_id, _) = world.spawn_stored_entity_for_test(
        dimension,
        witch_type,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 4.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(witch_type).unwrap(),
        ),
    );
    world.tick(Vec::new());
    world.tick(Vec::new());
    let witch = world.stored_entity_for_test(dimension, witch_id).unwrap();
    assert!(witch.on_ground(), "witch after landing tick: {witch:?}");
    let target = BlockPos { x: 10, y: 65, z: 4 };
    assert!(world.move_witch_ground_to_for_test(dimension, witch_id, target, 0, 1.0));
    (world, witch_id, target)
}

#[test]
fn ground_navigation_recompute_drains_the_external_level_boundary() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (mut world, witch_id, _) = ground_navigation_fixture();
    let straight = world.ground_path_for_test(dimension, witch_id).unwrap();
    assert!(straight.nodes().iter().all(|node| node.pos.z == 4));

    let obstacles = [
        BlockPos { x: 6, y: 65, z: 4 },
        BlockPos { x: 6, y: 66, z: 4 },
    ];
    world.with_level_for_test(dimension, |level| {
        for obstacle in obstacles {
            assert!(level.set_block(
                obstacle,
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            ));
        }
    });
    assert!(!world.set_block(
        dimension,
        obstacles[0],
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));

    let detour = world.ground_path_for_test(dimension, witch_id).unwrap();
    assert_ne!(detour, straight);
    assert!(detour.nodes().iter().any(|node| node.pos.z != 4));
}

#[test]
fn player_attack_routes_redirectable_projectiles_through_aim_deflect() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "batter", 0xba77e2);
    let owner = Uuid::from_u128(0xba77e2);
    let (projectile_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 2.0,
        },
        crate::entity::EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(None, 0.3),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    );
    world.tick(receiver.drain());

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: projectile_id,
        }),
    );
    let projectile = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, projectile_id)
        .expect("the deflected fireball remains live");
    let crate::entity::EntityData::LargeFireball(data) = projectile.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));
    assert_eq!(
        data.projectile().acceleration_power(),
        HurtingProjectileState::INITIAL_ACCELERATION_POWER
    );
    assert!((projectile.delta_movement().z - 1.045).abs() < 1e-6);
}

#[test]
fn player_attack_uses_the_installed_redirectable_projectile_generation() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "batter", 0xba77e3);
    install_redirectable_projectiles(&mut world, &["minecraft:snowball"]);
    let owner = Uuid::from_u128(0xba77e3);
    let (snowball_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 2.0,
        },
        crate::entity::EntityData::Snowball(crate::entity::SnowballData::new(
            plain_stack("minecraft:snowball", 1).unwrap(),
        )),
    );
    world.tick(receiver.drain());

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: snowball_id,
        }),
    );
    let snowball = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, snowball_id)
        .unwrap();
    let crate::entity::EntityData::Snowball(data) = snowball.data() else {
        unreachable!()
    };
    assert_eq!(data.projectile().owner(), Some(owner));
}

#[test]
fn attacking_an_arrow_removed_from_the_live_tag_disconnects() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "batter", 0xba77e4);
    install_redirectable_projectiles(&mut world, &[]);
    let (arrow_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 2.0,
        },
        crate::entity::EntityData::Arrow(crate::entity::ArrowData::new(
            plain_stack("minecraft:arrow", 1).unwrap(),
        )),
    );
    world.tick(receiver.drain());

    let attacked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: arrow_id,
        }),
    );
    assert!(attacked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::InvalidEntityAttacked,
        }
    )));
}

#[test]
fn player_attack_damage_uses_the_installed_damage_tag_generation() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "batter", 0xba77e5);
    let snapshot = crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:damage_type".to_string(),
        "minecraft:bypasses_invulnerability".to_string(),
        vec!["minecraft:player_attack".to_string()],
    )]);
    world.install_level_registry_tags(Arc::new(snapshot));
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        cow_type,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.0,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    let cow = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, cow_id)
        .unwrap();
    cow.set_invulnerable(true);
    let health_before = cow.living().unwrap().health();

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: cow_id }),
    );
    let health_after = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow_id)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(
        health_after < health_before,
        "{health_before} -> {health_after}"
    );
}

/// A block destroyed by a shape update puts its loot on the ground, at the
/// position and with the pickup delay `Block.popResource` (`Block.java:417-424`,
/// `:442-448`) gives every drop.
#[test]
fn a_plant_that_loses_its_support_becomes_an_item_entity_on_the_ground() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    dandelion_on_dirt(&mut world);

    remove_the_support(&mut world);

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert_eq!(entities.len(), 1, "one drop, one item entity: {entities:?}");
    let item = &entities[0];
    assert_eq!(
        field(item, "id"),
        &NbtSummaryValue::string("minecraft:item")
    );

    // `minecraft:blocks/dandelion` is one item entry rolled once, so the stack
    // is the flower itself.
    let NbtSummaryValue::Compound(stack) = field(item, "Item") else {
        panic!("Item is a compound");
    };
    assert_eq!(
        field(stack, "id"),
        &NbtSummaryValue::string("minecraft:dandelion")
    );
    assert_eq!(field(stack, "count"), &NbtSummaryValue::Int(1));

    // `new ItemEntity(level, x, y, z, stack)` (`ItemEntity.java:60-65`) gives
    // the drop `0.2` upward and a horizontal nudge inside +-0.1.
    let motion = doubles(field(item, "Motion"));
    assert_eq!(motion[1], 0.2);
    assert!(motion[0] >= -0.1 && motion[0] < 0.1, "{motion:?}");
    assert!(motion[2] >= -0.1 && motion[2] < 0.1, "{motion:?}");

    // `entity.setDefaultPickUpDelay()` (`Block.java:445`), which is ten ticks
    // (`ItemEntity.java:398-400`).
    assert_eq!(field(item, "PickupDelay"), &NbtSummaryValue::Short(10));
}

/// `Level.setBlock`'s `updateFlags & -34` (`Level.java:253`) clears
/// `UPDATE_SUPPRESS_DROPS` before the shape cascade, so a write that suppresses
/// its own drops does **not** suppress the drops of the blocks its cascade
/// destroys.
///
/// `-34` is `~(UPDATE_SUPPRESS_DROPS | UPDATE_NEIGHBORS)` (`Block.java:91`,
/// `:96`), and the drop half of it is the observable one: the plant's
/// `updateShape` answers air, `Block.updateOrDestroy` (`Block.java:234`) turns
/// that into `destroyBlock(pos, (updateFlags & 32) == 0, ..)`, and the flags it
/// reads are the cleared ones. A mask that kept bit 32 -- `& -2`, the obvious
/// mis-transcription, since the comment beside it is about neighbour updates --
/// would leave the flower on the ground as nothing at all.
///
/// The second half of the pair is `Block.updateOrDestroy`'s own `& -33`
/// (`Block.java:237`), which clears the drop suppression and **keeps**
/// `UPDATE_NEIGHBORS`: the two masks differ by exactly that bit.
#[test]
fn a_drop_suppressing_write_still_drops_what_its_shape_cascade_destroys() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    dandelion_on_dirt(&mut world);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 1, y: 64, z: 1 },
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL | crate::world::set_block_update_suppress_drops(),
    ));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert_eq!(
        entities.len(),
        1,
        "the cascade's own destroy drops: {entities:?}"
    );
}

/// The one item entity lying in the world, as its saved document reports it:
/// `(item id, count, PickupDelay, Pos)`.
///
/// This can be asked once a tick to watch a drop count its delay down.
fn only_ground_item(world: &mut ServerWorld) -> Option<(String, i32, i16, [f64; 3])> {
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    let entity = match entities.len() {
        0 => return None,
        1 => &entities[0],
        _ => panic!("these tests drop one stack at a time: {entities:?}"),
    };
    let NbtSummaryValue::Compound(stack) = field(entity, "Item") else {
        panic!("Item is a compound");
    };
    let NbtSummaryValue::String(id) = field(stack, "id") else {
        panic!("an item stack names its item");
    };
    let NbtSummaryValue::Int(count) = field(stack, "count") else {
        panic!("an item stack carries a count");
    };
    let NbtSummaryValue::Short(delay) = field(entity, "PickupDelay") else {
        panic!("an item entity carries a pickup delay");
    };
    let pos = doubles(field(entity, "Pos"));
    Some((
        id.try_to_string().unwrap(),
        *count,
        *delay,
        [pos[0], pos[1], pos[2]],
    ))
}

/// Half of an entity type's declared width, which is how far its bounding box
/// reaches from the position it stands at.
fn half_width(entity_type_id: i32) -> f64 {
    let [width, _height] =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity_type_id)
            .expect("the generated catalog carries this entity type");
    f64::from(width / 2.0)
}

/// The block the session stands on top of when it waits over
/// [`dandelion_on_dirt`]'s drop: the plant sat at `(1, 65, 1)` on dirt at
/// `(1, 64, 1)`, and pulling the dirt out leaves the drop falling down that
/// column.
const OVER_THE_DROP: Vec3d = Vec3d {
    x: 1.5,
    y: 64.0,
    z: 1.5,
};

/// A world with one session standing in the column a plant is about to drop
/// into, and the drop already on the ground with its delay untouched.
///
/// The drop is made with [`remove_the_support`], which writes the block outside
/// a tick, so no tick has run between the drop and the first observation and
/// the delay still reads what `Block.popResource` set.
fn a_session_waiting_over_a_fresh_drop() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld)
{
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(&sender, &receiver, &mut world, 1, OVER_THE_DROP);
    dandelion_on_dirt(&mut world);
    remove_the_support(&mut world);
    (sender, receiver, world)
}

/// `ItemEntity.playerTouch` (`ItemEntity.java:339-355`) end to end.
///
/// The `pickupDelay` gate at `:344` is the whole of this test: the delay
/// `Block.popResource` gives every drop (`Block.java:445` through
/// `ItemEntity.setDefaultPickUpDelay`, `ItemEntity.java:398-400`) keeps the
/// stack on the ground for ten ticks with the session standing right on it,
/// and the first tick that reads the delay as zero moves the stack into the
/// inventory and takes the entity with it (`:346-347`).
#[test]
fn a_drop_reaches_the_inventory_only_once_its_pickup_delay_has_run_out() {
    let (_sender, receiver, mut world) = a_session_waiting_over_a_fresh_drop();

    let mut delays = Vec::new();
    let mut ticks_waited = 0;
    while let Some((id, count, delay, _)) = only_ground_item(&mut world) {
        assert_eq!((id.as_str(), count), ("minecraft:dandelion", 1));
        delays.push(delay);
        let tick = world.tick(receiver.drain());
        ticks_waited += 1;
        assert!(ticks_waited < 40, "the drop was never picked up");
        if only_ground_item(&mut world).is_some() {
            // Nothing reaches the inventory while the delay is still running:
            // the session still holds only the stack the fixture gave it.
            assert_eq!(
                snapshot_player(&tick, 1)
                    .inventory
                    .slots
                    .iter()
                    .filter(|item| item.item_id.is_some())
                    .count(),
                1
            );
        }
    }
    assert_eq!(
        delays,
        vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        "the delay counts down from ten and the tick that reads zero is the pickup"
    );
    assert_eq!(ticks_waited, 11);

    // `Inventory.addResource(ItemStack)` (`Inventory.java:194-201`) finds no
    // slot with room for a dandelion -- the fixture's own stack is stone --
    // so `getFreeSlot` (`:101-109`) hands it the first empty one.
    let settled = world.tick(receiver.drain());
    assert_eq!(
        snapshot_player(&settled, 1).inventory.slots[1],
        ItemStackSummary {
            item_id: Some(item_id("minecraft:dandelion")),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    );
}

/// The packets one pickup sends, and the one it does not.
///
/// `LivingEntity.take` (LivingEntity.java:3253-3261) reaches every session
/// tracking the picked-up entity through `sendToTrackingPlayers`, and
/// `ServerPlayer.take` (ServerPlayer.java:1166-1169) follows it with the open
/// menu's `broadcastChanges`, which for a session with nothing else open is a
/// `ClientboundContainerSetSlotPacket` on the player inventory menu
/// (`ServerPlayer.java:299-301`).
///
/// No sound is sent. The pickup sound is the *client's*, played the moment the
/// take packet lands (`ClientPacketListener.java:999-1009`), so a server that
/// sent one would have it played twice.
#[test]
fn a_pickup_sends_the_take_packet_then_the_slot_it_filled_and_no_sound() {
    let (sender, receiver, mut world) = a_session_waiting_over_a_fresh_drop();

    let existing = RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:dandelion".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(60),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:rarity".into(),
                    value: NbtSummaryValue::String("common".into()),
                }]),
            },
        ]),
        &world.item_stack_resolver(),
    )
    .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 37,
            item: existing.summary().clone(),
        }),
    );

    let mut picked = None;
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        if tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::TakeItemEntity(_),
                    ..
                }
            )
        }) {
            picked = Some(tick);
            break;
        }
    }
    let picked = picked.expect("the drop is picked up");

    let takes: Vec<(ConnectionId, bbb_protocol::packets::TakeItemEntity)> = picked
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::TakeItemEntity(take),
            } => Some((*connection_id, *take)),
            _ => None,
        })
        .collect();
    assert_eq!(
        takes.len(),
        1,
        "the one session tracking the drop is told once: {takes:?}"
    );
    let (told, take) = takes[0];
    // The session that walked into the item is tracking it by definition, so
    // `sendToTrackingPlayers` reaches it without a second, self-addressed send.
    assert_eq!(told, ConnectionId(1));
    assert_eq!(take.player_id, snapshot_player(&picked, 1).entity_id);
    // `orgCount` (`ItemEntity.java:343`): the whole stack left the ground.
    assert_eq!(take.amount, 1);

    let position_of = |wanted: fn(&WorldEvent) -> bool| {
        picked
            .events
            .iter()
            .position(wanted)
            .unwrap_or_else(|| panic!("{:?}", picked.events))
    };
    let take_at = position_of(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::TakeItemEntity(_),
                ..
            }
        )
    });
    let slot_at = position_of(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
            } if update.item.item_id.is_some()
        )
    });
    let removal_at = picked
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::RemoveEntities(update),
                } if update.entity_ids == vec![take.item_id]
            )
        })
        .unwrap_or_else(|| panic!("{:?}", picked.events));
    assert!(
        take_at < removal_at,
        "the pickup animation precedes the discard callback's removal: {:?}",
        picked.events
    );
    let WorldEvent::Send {
        packet: ClientboundPlayPacket::ContainerSetSlot(slot),
        ..
    } = &picked.events[slot_at]
    else {
        unreachable!("the position was found by matching this shape")
    };
    // The two numberings are not the same walk: `InventoryMenu` puts the
    // hotbar *last*, at menu slots 36..45 (`InventoryMenu.java:26-27`), and the
    // storage rows before it at 9..36 (`:24-25`), so `Inventory` slot one --
    // the second hotbar slot -- is menu slot thirty-seven. The menu itself is
    // container zero (`InventoryMenu.java:14`).
    assert_eq!((slot.container_id, slot.slot), (0, 37));
    assert_eq!(slot.item.item_id, Some(item_id("minecraft:dandelion")));
    assert_eq!(slot.item.count, 61);
    assert_eq!(
        snapshot_player(&picked, 1)
            .inventory
            .slots
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:dandelion")))
            .map(|stack| stack.count)
            .sum::<i32>(),
        61,
    );

    assert!(
        !picked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(_),
                ..
            }
        )),
        "the pickup sound is the client's own: {:?}",
        picked.events
    );
}

/// `this.getBoundingBox().inflate(1.0, 0.5, 1.0)` (`Player.java:462`): the
/// pickup box is the player's own box grown by a whole block horizontally, and
/// a stack a millionth of a block beyond that is left where it lies.
///
/// Both probes are computed from the drop's own saved position and from the
/// two entity types' declared widths, so what the test measures is the
/// inflation rather than a coordinate someone chose.
#[test]
fn a_drop_just_beyond_the_pickup_box_is_left_where_it_lies() {
    let (sender, receiver, mut world) = a_session_waiting_over_a_fresh_drop();
    // Wait the delay out well clear of the drop, so the only thing left to
    // decide the pickup is where the session stands.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            ..OVER_THE_DROP
        },
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    let (_, _, delay, _) = only_ground_item(&mut world).expect("nothing out there picked it up");
    assert_eq!(delay, 0, "the delay is spent, so only the reach is left");

    // The two boxes touch when the gap between the positions is the inflation
    // plus both half-widths.
    let reach = crate::world::PICKUP_INFLATION[0]
        + half_width(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID)
        + half_width(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID);
    let hair = 1.0e-6;

    let (_, _, _, pos) = only_ground_item(&mut world).unwrap();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: pos[0] + reach + hair,
            y: OVER_THE_DROP.y,
            z: pos[2],
        },
    );
    assert!(
        only_ground_item(&mut world).is_some(),
        "a hair past the box is out of reach"
    );

    let (_, _, _, pos) = only_ground_item(&mut world).unwrap();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: pos[0] + reach - hair,
            y: OVER_THE_DROP.y,
            z: pos[2],
        },
    );
    assert!(
        only_ground_item(&mut world).is_none(),
        "a hair inside the box is a pickup"
    );
}

/// `Inventory.add(int, ItemStack)`'s two answers for a session with nowhere to
/// put the stack (`Inventory.java:286-291`): a survival player takes nothing
/// and the drop stays lying, and a creative one is told it succeeded and the
/// stack simply ceases to exist.
#[test]
fn a_full_inventory_leaves_the_drop_lying_unless_the_player_is_creative() {
    for creative in [false, true] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(mining_config()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
        // Every storage slot and the off-hand hold a full stack of something a
        // dandelion will not merge into. `InventoryMenu` numbers the storage
        // rows 9..36, the hotbar 36..45 and the off-hand 45
        // (`InventoryMenu.java:23-27`).
        for menu_slot in 9..=45 {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                    slot_num: menu_slot,
                    item: ItemStackSummary {
                        item_id: Some(item_id("minecraft:dirt")),
                        count: 64,
                        component_patch: DataComponentPatchSummary::default().into(),
                    },
                }),
            );
        }
        if !creative {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "defaultgamemode survival",
            );
        }
        stand_at(&sender, &receiver, &mut world, 1, OVER_THE_DROP);
        dandelion_on_dirt(&mut world);
        remove_the_support(&mut world);
        for _ in 0..20 {
            world.tick(receiver.drain());
        }
        if creative {
            assert!(
                only_ground_item(&mut world).is_none(),
                "a creative pickup swallows what will not fit"
            );
        } else {
            let (id, count, delay, _) =
                only_ground_item(&mut world).expect("a full survival inventory takes nothing");
            assert_eq!((id.as_str(), count, delay), ("minecraft:dandelion", 1, 0));
        }
    }
}

/// `MinecraftServer.setInitialSpawn` (`MinecraftServer.java:492-498`) files the
/// spawn at `ChunkGenerator.getSpawnHeight`, which the `minecraft:flat`
/// generator answers as `minY + min(worldHeight, layers.size())`
/// (`FlatLevelSource.java:62-64`). This world's layers are the fixture's own
/// solid column, so the answer is the first free y above its floor -- and a
/// player who joins stands *on* the floor rather than hovering a block above
/// it, which is what puts a drop lying at their feet inside the pickup box.
#[test]
fn the_default_spawn_stands_on_the_fixture_floor() {
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    assert_eq!(
        crate::world::SPAWN_Y,
        f64::from(fixture.floor_y() + 1),
        "the spawn is the first free y above the flat generator's layers"
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let settled = world.tick(receiver.drain());
    assert_eq!(
        snapshot_player(&settled, 1).position.y,
        f64::from(fixture.floor_y() + 1)
    );
}

/// The reach a session has without moving at all.
///
/// An item at rest sits on the same surface the player stands on, so its
/// `0.25`-tall box starts exactly at the player's feet -- and
/// `getBoundingBox().inflate(1.0, 0.5, 1.0)` (`Player.java:462`) reaches half
/// a block below those feet, so the two overlap. A spawn one block above the
/// floor puts the whole item under that half block and no session picks
/// anything up without walking down first, which is why this is asserted
/// against the fixture's floor rather than against a coordinate.
#[test]
fn a_drop_resting_at_the_spawning_players_feet_is_inside_the_pickup_box() {
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let surface = f64::from(fixture.floor_y() + 1);
    let spawn = Vec3d {
        x: crate::world::SPAWN_X,
        y: crate::world::SPAWN_Y,
        z: crate::world::SPAWN_Z,
    };
    let pickup = bbb_server_types::interaction::player_bounding_box(spawn)
        .unwrap()
        .inflate(
            crate::world::PICKUP_INFLATION[0],
            crate::world::PICKUP_INFLATION[1],
            crate::world::PICKUP_INFLATION[2],
        );
    let [width, height] = bbb_protocol::entity_types::vanilla_entity_type_dimensions(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .unwrap();
    let resting = bbb_server_types::geometry::Aabb::of_entity(
        Vec3d {
            y: surface,
            ..spawn
        },
        width,
        height,
    );
    assert!(
        pickup.intersects(resting),
        "a drop lying where the session stands must be in reach: {pickup:?} vs {resting:?}"
    );
}

#[test]
fn the_entity_seam_answers_both_light_inputs_from_the_level_it_runs() {
    let mut world = spawn_pinned_world(creative_fixture_config());
    let lit = BlockPos { x: 0, y: 96, z: 0 };
    for (dimension, ambient_light) in [
        (Java26BuiltinDimension::Overworld, 0.0),
        (Java26BuiltinDimension::Nether, 0.1),
        (Java26BuiltinDimension::End, 0.25),
    ] {
        assert_eq!(
            world.entity_level_light_for_test(dimension, lit).1,
            Some(ambient_light),
            "{dimension:?} ambient light"
        );
    }

    // `LevelReader.getMaxLocalRawBrightness` answers a flat 15 outside the
    // horizontal limits (`LevelReader.java:167-171`).
    let dimension = Java26BuiltinDimension::Nether;
    assert_eq!(
        world.entity_level_light_for_test(dimension, lit).0,
        Some(0),
        "the roofed nether column is unlit"
    );
    assert_eq!(
        world
            .entity_level_light_for_test(
                dimension,
                BlockPos {
                    x: 30_000_000,
                    ..lit
                }
            )
            .0,
        Some(15)
    );
    assert_eq!(
        world
            .entity_level_light_for_test(
                dimension,
                BlockPos {
                    z: -30_000_001,
                    ..lit
                }
            )
            .0,
        Some(15)
    );
}

/// A cow on the flat fixture floor with a session standing beside it, which is
/// what keeps `Mob.checkDespawn` resetting its `noActionTime` (`Mob.java:670-674`)
/// -- the counter `RandomStrollGoal.canUse` refuses above 100
/// (`RandomStrollGoal.java:43-45`).
fn a_cow_beside_a_session() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Vec3d,
) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let floor = bbb_server_types::fixture::FlatFixture::new()
        .unwrap()
        .floor_y();
    let stand = Vec3d {
        x: 0.5,
        y: f64::from(floor + 1),
        z: 0.5,
    };
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        cow_type,
        stand,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    (sender, receiver, world, cow_id, stand)
}

/// The whole stroll family reaches the level through two inputs, so a mob that
/// walks somewhere it picked is the proof both are answered: the ambient light
/// decides whether a stroll input is prepared at all, and the sampled position's
/// brightness is what `Animal.getWalkTargetValue` weighs it by
/// (`Animal.java:92-94`).
#[test]
fn a_cow_beside_a_session_strolls_to_a_position_it_sampled() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (_sender, receiver, mut world, cow_id, stand) = a_cow_beside_a_session();
    let mut path = None;
    // `RandomStrollGoal.canUse` draws against `reducedTickDelay(120)`
    // (`RandomStrollGoal.java:47-49`) on the ticks the selector runs `canUse` at
    // all (`Mob.java:688-689`).
    for _ in 0..600 {
        world.tick(receiver.drain());
        if let Some(found) = world.ground_path_for_test(dimension, cow_id) {
            path = Some(found);
            break;
        }
    }
    let path = path.expect("the cow strolls somewhere it picked for itself");
    let target = path.nodes().last().expect("a path holds its target").pos;
    let from = BlockPos {
        x: stand.x.floor() as i32,
        y: stand.y.floor() as i32,
        z: stand.z.floor() as i32,
    };
    assert_ne!(target, from, "the stroll leaves the block it stands on");
    // The sampler is called with ten out and seven up
    // (`WaterAvoidingRandomStrollGoal.java:22-29`).
    assert!(
        (target.x - from.x).abs() <= 10
            && (target.z - from.z).abs() <= 10
            && (target.y - from.y).abs() <= 7,
        "the sample stays inside the box it drew from: {target:?} from {from:?}"
    );
}

/// The result and the translate keys of one `command.execute` answer.
pub(crate) fn console_command_answer(tick: &TickResult) -> (Option<i32>, Vec<String>) {
    let Ok(AdminResult::Command(report)) = &admin_outcome(tick).result else {
        panic!("a command.execute answers with a command report");
    };
    let keys = report
        .messages
        .iter()
        .map(|message| {
            let NbtSummaryValue::Compound(entries) = message.component() else {
                panic!("a command message is a component compound")
            };
            match entries.iter().find(|entry| entry.name == "translate") {
                Some(NbtSummaryEntry {
                    value: NbtSummaryValue::String(key),
                    ..
                }) => key.try_to_string().unwrap(),
                other => panic!("a translatable message, not {other:?}"),
            }
        })
        .collect();
    (report.result, keys)
}

pub(crate) fn console_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(
                operation_id,
                &format!("{operation_id}:{command}"),
                AdminScope::Command,
            ),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// The `ENTITY_DIE` rows one level recorded, oldest first.
fn entity_die_rows(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
) -> Vec<RecordedGameEvent> {
    world
        .game_events(dimension)
        .0
        .into_iter()
        .filter(|row| row.game_event == VanillaGameEvent::ENTITY_DIE)
        .collect()
}

/// Every `ClientboundRemoveEntities` a tick sent, as `(recipient, ids)`.
fn published_removals(tick: &TickResult) -> Vec<(ConnectionId, Vec<i32>)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::RemoveEntities(remove),
            } => Some((*connection_id, remove.entity_ids.clone())),
            _ => None,
        })
        .collect()
}

/// `Entity.kill(ServerLevel)` (`Entity.java:399-402`) for a non-living stored
/// entity: the removal (`:400`), which the victim's tracker turns into
/// `ClientboundRemoveEntities`, and the `ENTITY_DIE` (`:401`) with the victim
/// as its own `GameEvent.Context.sourceEntity` (`:1335-1337`).
///
/// This is the half `/kill` could not reach while every stored victim went
/// through `LivingEntity.kill`'s `genericKill` hurt (`LivingEntity.java:303-305`):
/// an item entity's own `hurtServer` (`ItemEntity.java:284-306`) answered the
/// source and discarded the drop, but nothing in that body raises `ENTITY_DIE`,
/// and the kinds whose damage rules refuse the source (`Marker`, `PrimedTnt`,
/// `Display`; `projectile_action.rs::prepare_nonliving_hurt_plan`) were not
/// removed at all.
#[test]
fn a_killed_item_entity_leaves_the_store_and_tells_its_tracker() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);

    let (entity_id, _uuid) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 0.5,
        },
        EntityData::Item(ItemEntityData::new(
            plain_stack("minecraft:stone", 1).expect("a plain stack decodes"),
            0.0,
        )),
    );
    let announced = world.tick(receiver.drain());
    assert!(
        announced.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.id == entity_id
        )),
        "the session has to be watching the drop before its removal means anything"
    );
    // `Entity.gameEvent` raises the event at `this.position` (`:1332`), which
    // the entity may have moved since the spawn.
    let died_at = world
        .stored_entity_for_test(dimension, entity_id)
        .expect("the drop is still stored")
        .block_position();

    let killed = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "kill @e[type=minecraft:item]",
    );
    assert_eq!(
        console_command_answer(&killed),
        (Some(1), vec!["commands.kill.success.single".to_owned()])
    );
    assert!(world.stored_entity_for_test(dimension, entity_id).is_none());
    assert_eq!(
        published_removals(&killed),
        vec![(ConnectionId(1), vec![entity_id])]
    );
    assert_eq!(
        entity_die_rows(&world, dimension),
        vec![RecordedGameEvent {
            game_event: VanillaGameEvent::ENTITY_DIE,
            pos: died_at,
            source_entity_id: Some(entity_id),
            affected_state: None,
        }]
    );
}

/// `ArmorStand.kill` (`ArmorStand.java:461-464`) repeats `Entity.kill`
/// verbatim, so `/kill` on a stand removes it and raises `ENTITY_DIE` -- the
/// event `bbb-server-entity`'s standalone `armor_stand.rs::kill` stops short of,
/// which is what left the `ArmorStand.hurtServer` arm that removes a stand for
/// `minecraft:generic_kill` (`:287-290`, through `kill`) silent.
#[test]
fn a_killed_armor_stand_raises_entity_die_with_the_stand_as_its_source() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:armor_stand 3.5 65 0.5",
    );
    assert_eq!(
        console_command_answer(&summoned),
        (Some(1), vec!["commands.summon.success".to_owned()])
    );
    let [stand] = world.stored_entity_ids_for_test(dimension)[..] else {
        panic!("one summoned stand");
    };
    let stand_block = world
        .stored_entity_for_test(dimension, stand)
        .expect("the summoned stand is filed in the level")
        .block_position();

    let killed = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "kill @e[type=minecraft:armor_stand]",
    );
    assert_eq!(
        console_command_answer(&killed),
        (Some(1), vec!["commands.kill.success.single".to_owned()])
    );
    assert!(world.stored_entity_for_test(dimension, stand).is_none());
    assert_eq!(
        published_removals(&killed),
        vec![(ConnectionId(1), vec![stand])]
    );
    assert_eq!(
        entity_die_rows(&world, dimension),
        vec![RecordedGameEvent {
            game_event: VanillaGameEvent::ENTITY_DIE,
            pos: stand_block,
            source_entity_id: Some(stand),
            affected_state: None,
        }]
    );
}

/// `/kill` on a plain living mob is still `LivingEntity.kill`
/// (`LivingEntity.java:303-305`): one `genericKill` hurt for
/// `Float.MAX_VALUE`, so the mob dies through `LivingEntity.die` and drops its
/// loot table. `minecraft:entities/iron_golem` is two pools the loot engine
/// evaluates whole, and the iron pool always draws.
///
/// The dying mob stays in the store, which is the sharpest difference from the
/// two tests above: `Entity.kill` removes at once, while `LivingEntity.die`
/// (`:1426-1457`) leaves the body for `tickDeath`'s twenty ticks (`:549-555`)
/// before that removal.
#[test]
fn a_killed_living_mob_still_hurts_and_drops_its_loot() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    use bbb_protocol::entity_types::{
        vanilla_entity_type_passenger_attachments, vanilla_entity_type_vehicle_attachment,
    };

    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    let (golem, _uuid) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID)
                .expect("an iron golem is living"),
        ),
    );

    let killed = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "kill @e[type=minecraft:iron_golem]",
    );
    assert_eq!(
        console_command_answer(&killed),
        (Some(1), vec!["commands.kill.success.single".to_owned()])
    );
    let dying = world
        .stored_entity_for_test(dimension, golem)
        .and_then(crate::entity::Entity::living)
        .is_some_and(crate::entity::LivingEntityState::is_dead_or_dying);
    assert!(dying, "the genericKill hurt was lethal");
    assert_eq!(
        entity_die_rows(&world, dimension)
            .iter()
            .map(|row| row.source_entity_id)
            .collect::<Vec<_>>(),
        vec![Some(golem)]
    );
    let dropped = world.stored_items_for_test(dimension);
    let mut ids: Vec<&str> = dropped.iter().map(|item| item.2.as_str()).collect();
    ids.sort_unstable();
    assert!(
        ids == ["minecraft:iron_ingot"] || ids == ["minecraft:iron_ingot", "minecraft:poppy"],
        "the iron pool always draws and the poppy pool may: {dropped:?}"
    );
    let iron = dropped
        .iter()
        .find(|item| item.2 == "minecraft:iron_ingot")
        .expect("the iron pool always draws");
    assert!((3..=5).contains(&iron.3), "{iron:?}");
    assert_eq!(iron.4, 10, "`spawnAtLocation` sets the default delay");

    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    console_command(&sender, &receiver, &mut world, 2, "tick freeze");
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let origin = world.publication().players[0].position;
    let mut operation = 2;
    let mut command = |world: &mut ServerWorld, text: &str| {
        operation += 1;
        let tick = console_command(&sender, &receiver, world, operation, text);
        assert!(console_command_answer(&tick).0.is_some(), "{text}");
        tick
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamemode spectator observer");
    for (index, kind) in ["cow", "horse"].into_iter().enumerate() {
        let x = origin.x.floor() as i32 + 3 + index as i32 * 5;
        let y = origin.y.floor() as i32 + 2;
        let z = origin.z.floor() as i32;
        let center_x = f64::from(x) + 0.5;
        let center_z = f64::from(z) + 0.5;
        let uuid_bits = 100 + index as u128 * 2;
        let vehicle_uuid = Uuid::from_u128(uuid_bits);
        let rider_uuid = Uuid::from_u128(uuid_bits + 1);
        let mut events = Vec::new();
        for text in [
            format!(
                "fill {} {} {} {} {} {} minecraft:stone",
                x - 2, y - 1, z - 2, x + 2, y - 1, z + 2
            ),
            format!(
                "summon minecraft:{kind} {center_x} {y} {center_z} {{UUID:[I;0,0,0,{uuid_bits}],Age:0,NoAI:1b,Rotation:[90f,0f]}}"
            ),
            format!("attribute {vehicle_uuid} minecraft:scale base set 2"),
            format!(
                "summon minecraft:armor_stand {center_x} {y} {center_z} {{UUID:[I;0,0,0,{}],Small:1b,NoGravity:1b,Invulnerable:1b}}",
                uuid_bits + 1
            ),
            format!("ride {rider_uuid} mount {vehicle_uuid}"),
        ] {
            events.extend(command(&mut world, &text).events);
        }
        let vehicle = world
            .live_entity_id_for_uuid_for_test(dimension, vehicle_uuid)
            .unwrap();
        let rider = world
            .live_entity_id_for_uuid_for_test(dimension, rider_uuid)
            .unwrap();
        let passenger = world.stored_entity_for_test(dimension, rider).unwrap();
        assert!(matches!(passenger.data(), EntityData::ArmorStand(data) if data.is_small()));
        assert!(passenger.no_gravity() && passenger.invulnerable());
        let rider_vehicles = |world: &ServerWorld| {
            let reads = world.entity_reads();
            let source = reads
                .living_command_entity_by_uuid(rider_uuid)
                .unwrap()
                .as_command_source(CommandPermission::Owner);
            reads
                .related_command_entities(&source, crate::commands::types::ExecuteRelation::Vehicle)
                .into_iter()
                .map(|entity| entity.uuid())
                .collect::<Vec<_>>()
        };
        let assert_mounted = |world: &ServerWorld| {
            assert_eq!(
                world.stored_passengers_for_test(dimension, vehicle),
                &[rider]
            );
            assert_eq!(rider_vehicles(world), [vehicle_uuid]);
            let body = world.stored_entity_for_test(dimension, vehicle).unwrap();
            let passenger = world.stored_entity_for_test(dimension, rider).unwrap();
            assert!(matches!(body.data(), EntityData::GeneralLiving(data) if !data.is_baby()));
            assert!(passenger.is_alive_with_health());
            let scale = body
                .living()
                .unwrap()
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
                as f32;
            assert_eq!(scale, 2.0);
            let point = vanilla_entity_type_passenger_attachments(body.entity_type_id()).unwrap()
                [0]
            .map(|coordinate| coordinate * scale);
            let rider_point =
                vanilla_entity_type_vehicle_attachment(passenger.entity_type_id()).unwrap();
            let rotated = |point: [f32; 3], yaw: f32| {
                let (sin, cos) = (-f64::from(yaw).to_radians()).sin_cos();
                let x = f64::from(point[0]);
                let z = f64::from(point[2]);
                [x * cos + z * sin, f64::from(point[1]), z * cos - x * sin]
            };
            let point = rotated(point, body.y_rot());
            let rider_point = rotated(rider_point, passenger.y_rot());
            for ((actual, origin), (point, rider_point)) in [
                (passenger.position().x, body.position().x),
                (passenger.position().y, body.position().y),
                (passenger.position().z, body.position().z),
            ]
            .into_iter()
            .zip(point.into_iter().zip(rider_point))
            {
                let expected = origin + point - rider_point;
                assert!(
                    (actual - expected).abs() < 1.0e-6,
                    "{kind}: {actual} != {expected}"
                );
            }
        };
        assert!(world.tick_rate_mut().step_game_if_paused(1));
        events.extend(world.tick(receiver.drain()).events);
        assert_eq!(
            world
                .stored_entity_for_test(dimension, vehicle)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
        assert_mounted(&world);
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetPassengers(packet),
            } if packet.vehicle_id == vehicle && packet.passenger_ids == [rider]
        )));

        events.extend(command(&mut world, &format!("kill {vehicle_uuid}")).events);
        assert_mounted(&world);
        let mut dying_ticks = 0;
        let mut saw_dying = false;
        for _ in 0..40 {
            assert!(world.tick_rate_mut().step_game_if_paused(1));
            events.extend(world.tick(receiver.drain()).events);
            let Some(body) = world.stored_entity_for_test(dimension, vehicle) else {
                break;
            };
            assert!(body.living().unwrap().is_dead_or_dying());
            saw_dying |= body.pose_id() == VANILLA_ENTITY_POSE_DYING_ID;
            dying_ticks += 1;
            assert_mounted(&world);
        }
        assert!(
            saw_dying && dying_ticks >= 2,
            "{kind}: the mounted death remains visible"
        );
        assert!(world.stored_entity_for_test(dimension, vehicle).is_none());
        assert!(world
            .stored_passengers_for_test(dimension, vehicle)
            .is_empty());
        assert!(rider_vehicles(&world).is_empty());
        assert!(world
            .stored_entity_for_test(dimension, rider)
            .unwrap()
            .is_alive_with_health());
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::RemoveEntities(packet),
            } if packet.entity_ids.contains(&vehicle)
        )));
        assert!(!events.iter().any(|event| matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::RemoveEntities(packet),
            } if packet.entity_ids.contains(&rider)
        )));
    }
}

/// The movement `ServerEntity.sendChanges` published for one entity over these
/// ticks, with the session it reached.
fn published_movement(
    ticks: &[TickResult],
    entity_id: i32,
) -> Vec<(ConnectionId, ClientboundPlayPacket)> {
    ticks
        .iter()
        .flat_map(|tick| tick.events.iter())
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } => match packet {
                ClientboundPlayPacket::MoveEntity(move_entity) if move_entity.id == entity_id => {
                    Some((*connection_id, packet.clone()))
                }
                ClientboundPlayPacket::EntityPositionSync(sync) if sync.id == entity_id => {
                    Some((*connection_id, packet.clone()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// One command tick plus the interval a tracker may wait before it publishes
/// (`ServerEntity.sendChanges`, ServerEntity.java:120-122).
fn ticks_through_tracker_interval(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    command: TickResult,
) -> Vec<TickResult> {
    let mut ticks = vec![command];
    for _ in 0..3 {
        ticks.push(world.tick(receiver.drain()));
    }
    ticks
}

/// `/teleport <targets> <location>` names a stored entity
/// (`EntityArgument.getEntities`, TeleportCommand.java:63), `performTeleport`
/// moves it (`:252`), and the tracker publishes the move the way
/// `ServerEntity.sendChanges` publishes one of that size.
///
/// The world is frozen so that the armor stand's own tick neither moves it nor
/// rewrites what the teleport left behind.
#[test]
fn a_commanded_teleport_moves_a_stored_entity_and_reaches_its_tracker() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    assert!(world.teleport_player(
        ConnectionId(2),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 7.5,
                y: 64.0,
                z: 0.5
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    ));
    drain_chunk_deliveries(&sender, &receiver, &mut world, 2);
    console_command(&sender, &receiver, &mut world, 1, "tick freeze");
    console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "summon minecraft:armor_stand 1.5 64 0.5",
    );
    let ids = world.stored_entity_ids_for_test(dimension);
    let [stand] = ids.as_slice() else {
        panic!("one summoned stand, not {ids:?}")
    };
    let stand = *stand;

    let first = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "teleport @e[type=minecraft:armor_stand,limit=1] 7.5 64 0.5",
    );
    assert_eq!(
        console_command_answer(&first),
        (
            Some(1),
            vec!["commands.teleport.success.location.single".to_owned()]
        )
    );
    let moved = world
        .stored_entity_for_test(dimension, stand)
        .expect("the stand is still filed in the level");
    assert_eq!(
        moved.position(),
        Vec3d {
            x: 7.5,
            y: 64.0,
            z: 0.5
        }
    );
    // `TeleportCommand.java:258-259`.
    assert_eq!(moved.delta_movement(), Vec3d::default());
    assert!(moved.on_ground());
    let _ = ticks_through_tracker_interval(&receiver, &mut world, first);

    let near = console_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "teleport @e[type=minecraft:armor_stand,limit=1] 9.5 64 0.5",
    );
    let published = published_movement(
        &ticks_through_tracker_interval(&receiver, &mut world, near),
        stand,
    );
    // `ServerEntity.java:158-165`: two blocks fit a relative move.
    match published.as_slice() {
        [(ConnectionId(1), ClientboundPlayPacket::MoveEntity(step))] => {
            assert!(step.has_position);
            assert_eq!((step.delta_x, step.delta_y, step.delta_z), (8_192, 0, 0));
        }
        other => panic!("one relative move to the watching session, not {other:?}"),
    }

    let far = console_command(
        &sender,
        &receiver,
        &mut world,
        5,
        "teleport @e[type=minecraft:armor_stand,limit=1] -4.5 64 0.5",
    );
    let published = published_movement(
        &ticks_through_tracker_interval(&receiver, &mut world, far),
        stand,
    );
    // `:148-157`: fourteen blocks do not, so the resync carries the position.
    match published.as_slice() {
        [(ConnectionId(1), ClientboundPlayPacket::EntityPositionSync(sync))] => {
            assert_eq!(
                sync.position,
                Vec3d {
                    x: -4.5,
                    y: 64.0,
                    z: 0.5
                }
            );
        }
        other => panic!("one position resync to the watching session, not {other:?}"),
    }

    let uuid = world
        .stored_entity_for_test(dimension, stand)
        .unwrap()
        .uuid();
    let crossed = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        "execute as @e[type=minecraft:armor_stand,limit=1] at @s in minecraft:the_nether run tp @s ~8 ~1 ~2 ~90 ~30",
    );
    assert_eq!(console_command_answer(&crossed).0, Some(1));
    assert!(world.stored_entity_for_test(dimension, stand).is_none());
    let target = Java26BuiltinDimension::Nether;
    let new_stand = world
        .stored_entity_ids_for_test(target)
        .into_iter()
        .find(|id| world.stored_entity_for_test(target, *id).unwrap().uuid() == uuid)
        .expect("the same UUID arrives in the target level");
    let moved = world.stored_entity_for_test(target, new_stand).unwrap();
    assert_eq!(
        moved.position(),
        Vec3d {
            x: 7.4375,
            y: 65.0,
            z: 2.0625
        }
    );
    assert_eq!((moved.y_rot(), moved.x_rot()), (90.0, 30.0));
    assert_eq!(moved.delta_movement(), Vec3d::default());
    assert!(moved.on_ground());
    assert!(published_removals(&crossed)
        .iter()
        .any(|(connection, ids)| { *connection == ConnectionId(1) && ids.contains(&stand) }));
    assert!(crossed.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::AddEntity(add),
        } if add.id == new_stand && add.uuid == uuid
    )));

    console_command(
        &sender,
        &receiver,
        &mut world,
        7,
        "execute in minecraft:the_nether as @e[type=minecraft:armor_stand,limit=1] in minecraft:overworld run tp @s 7.5 64 0.5 facing 7.5 64 10.5",
    );
    let returned = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| world.stored_entity_for_test(dimension, *id).unwrap().uuid() == uuid)
        .expect("facing is applied to the replacement in the target level");
    let returned = world.stored_entity_for_test(dimension, returned).unwrap();
    assert_eq!((returned.y_rot(), returned.x_rot()), (0.0, 0.0));
}

#[test]
fn a_commanded_cross_level_teleport_detaches_the_selected_passenger() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    console_command(&sender, &receiver, &mut world, 1, "tick freeze");
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    world.ensure_entity_transfer_full_chunk_for_test(source, (0, 0));
    let position = Vec3d {
        x: 1.5,
        y: 64.0,
        z: 0.5,
    };
    let (boat, stand) = world
        .spawn_stored_entity_transfer_tree_for_test(source, position, position)
        .unwrap();
    let uuid = world.stored_entity_for_test(source, stand).unwrap().uuid();
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "execute as @e[type=minecraft:armor_stand,limit=1] in minecraft:the_nether run tp @s 40.5 70 40.5 45 20",
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert!(world.stored_entity_for_test(source, stand).is_none());
    assert!(world.stored_passengers_for_test(source, boat).is_empty());
    assert_eq!(
        world
            .stored_entity_for_test(source, boat)
            .unwrap()
            .position(),
        position
    );
    let moved = world
        .stored_entity_ids_for_test(target)
        .into_iter()
        .map(|id| world.stored_entity_for_test(target, id).unwrap())
        .find(|entity| entity.uuid() == uuid)
        .expect("the selected passenger arrives without its vehicle");
    assert_eq!(
        moved.position(),
        Vec3d {
            x: 40.5,
            y: 70.0,
            z: 40.5
        }
    );
    assert_eq!((moved.y_rot(), moved.x_rot()), (45.0, 20.0));
}

/// The move the entity-runtime acceptance probe ends with. `/teleport` writes a
/// painting's attachment block (`BlockAttachedEntity.setPos`,
/// `BlockAttachedEntity.java:141-145`), and the survival check the next hundred
/// ticks reach (`:39-50`) drops the painting that no longer hangs on anything.
#[test]
fn a_commanded_teleport_leaves_a_painting_for_its_survival_check() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule spawn_mobs false",
    );
    console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "setblock 0 90 -1 minecraft:stone",
    );
    let (painting, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PAINTING_ID,
        Vec3d {
            x: 0.5,
            y: 90.5,
            z: 0.5,
        },
        crate::entity::EntityData::Painting(crate::entity::PaintingData::new(
            BlockPos { x: 0, y: 90, z: 0 },
            Direction::South,
            crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
        )),
    );
    // `HangingEntity.recalculateBoundingBox` (`HangingEntity.java:70-77`), which
    // the generic store spawn above does not run.
    crate::entity::hanging::recalculate_bounding_box(
        world
            .stored_entity_mut_for_test(dimension, painting)
            .expect("the painting is filed in the level"),
    );
    // One whole `checkInterval` period on the wall it was hung on.
    for _ in 0..=101 {
        world.tick(receiver.drain());
    }
    assert!(world.stored_entity_for_test(dimension, painting).is_some());

    console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "teleport @e[type=minecraft:painting,limit=1] -4 66 3",
    );
    let moved = world
        .stored_entity_for_test(dimension, painting)
        .expect("the painting outlives the teleport itself");
    let crate::entity::EntityData::Painting(data) = moved.data() else {
        panic!("the spawned entity is a painting")
    };
    assert_eq!(data.pose().attached(), BlockPos { x: -4, y: 66, z: 3 });

    let mut removed = Vec::new();
    for _ in 0..=101 {
        let tick = world.tick(receiver.drain());
        removed.extend(tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::RemoveEntities(remove),
            } if remove.entity_ids.contains(&painting) => Some(*connection_id),
            _ => None,
        }));
    }
    assert!(world.stored_entity_for_test(dimension, painting).is_none());
    assert_eq!(removed, vec![ConnectionId(1)]);
}

fn delivered_entity_additions(
    tick: &TickResult,
    entity_id: i32,
    column: (i32, i32),
    delivered: &mut bool,
) -> usize {
    let mut additions = 0;
    for event in &tick.events {
        let WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet,
        } = event
        else {
            continue;
        };
        match packet {
            ClientboundPlayPacket::LevelChunkWithLight(chunk) if (chunk.x, chunk.z) == column => {
                *delivered = true;
            }
            ClientboundPlayPacket::AddEntity(add) if add.id == entity_id => {
                assert!(*delivered, "the client receives the entity's column first");
                additions += 1;
            }
            _ => {}
        }
    }
    additions
}

/// The entity-runtime acceptance gate's world shape: the stand is already in
/// the save when the server starts, its column carries the `TicketType.FORCED`
/// ticket `/forceload` left behind, and the only session joins afterwards.
///
/// The publication gate `ChunkMap.tick` closes over (`ChunkMap.java:1202-1204`)
/// still opens here, on its `inEntityTickingRange` clause: a level-31 `FORCED`
/// ticket is entity-ticking on its own column, so a commanded teleport that
/// changes no section is published all the same.
#[test]
fn a_commanded_teleport_reaches_a_session_that_joined_after_a_restored_stand() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut source = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut source, 1, "stager", 1);
    console_command(
        &sender,
        &receiver,
        &mut source,
        1,
        "summon minecraft:armor_stand 1.5 64 0.5",
    );
    console_command(
        &sender,
        &receiver,
        &mut source,
        2,
        "data merge entity @e[type=minecraft:armor_stand,limit=1] {NoGravity:1b}",
    );
    assert_eq!(source.stored_entity_ids_for_test(dimension).len(), 1);
    let request = source
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(saved_entities(&request).len(), 1);

    let mut candidate = restore_candidate(10, None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut world = crate::tests::restore_stored(datapack_config(), &candidate);
    world.set_test_respawn_radius(0);
    let ids = world.stored_entity_ids_for_test(dimension);
    let [stand] = ids.as_slice() else {
        panic!("one restored stand, not {ids:?}")
    };
    let stand = *stand;
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    let acknowledged = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    let mut delivered = false;
    let additions: usize = [&joined, &handshake, &acknowledged]
        .into_iter()
        .map(|tick| delivered_entity_additions(tick, stand, (0, 0), &mut delivered))
        .sum();
    assert!(delivered);
    assert_eq!(
        additions, 1,
        "the stationary client receives the restored stand once"
    );
    for _ in 0..4 {
        let idle = world.tick(receiver.drain());
        assert_eq!(
            delivered_entity_additions(&idle, stand, (0, 0), &mut delivered),
            0
        );
    }
    assert!(
        world.entity_ticking_chunks(dimension).contains(&(0, 0)),
        "the restored stand's column is entity-ticking: {:?}",
        world.entity_ticking_chunks(dimension)
    );
    assert_eq!(
        world.player_tracking_viewers_for_test(dimension, stand),
        vec![ConnectionId(1)],
        "the session that joined after the stand watches it"
    );

    let moved = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "teleport @e[type=minecraft:armor_stand,limit=1] 1.5 65 0.5",
    );
    assert_eq!(
        console_command_answer(&moved),
        (
            Some(1),
            vec!["commands.teleport.success.location.single".to_owned()]
        )
    );
    let published = published_movement(
        &ticks_through_tracker_interval(&receiver, &mut world, moved),
        stand,
    );
    match published.as_slice() {
        [(ConnectionId(1), ClientboundPlayPacket::EntityPositionSync(sync))] => {
            assert_eq!(
                sync.position,
                Vec3d {
                    x: 1.5,
                    y: 65.0,
                    z: 0.5
                }
            );
        }
        other => panic!("one position resync to the watching session, not {other:?}"),
    }
}

#[test]
fn a_stationary_client_pairs_a_stand_once_when_its_later_chunk_batch_is_delivered() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        chunk_radius: 4,
        ..datapack_config()
    });
    world.tick_rate_mut().set_frozen(true);
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:armor_stand 40.5 64 0.5 {NoGravity:1b}",
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let ids = world.stored_entity_ids_for_test(dimension);
    let [stand] = ids.as_slice() else {
        panic!("one stationary stand, not {ids:?}");
    };
    let stand = *stand;
    let position = world
        .stored_entity_for_test(dimension, stand)
        .unwrap()
        .position();
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    let column = (2, 0);
    let mut delivered = false;
    for tick in [&joined, &handshake] {
        assert_eq!(
            delivered_entity_additions(tick, stand, column, &mut delivered),
            0
        );
    }
    assert!(!delivered, "the stand is outside the actual first batch");
    for _ in 0..3 {
        let idle = world.tick(receiver.drain());
        assert!(delivered_chunk_positions(&idle).is_empty());
        assert_eq!(
            delivered_entity_additions(&idle, stand, column, &mut delivered),
            0
        );
    }
    for _ in 0..32 {
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick: 1.0,
            },
        );
        let arrivals = delivered_chunk_positions(&tick);
        let additions = delivered_entity_additions(&tick, stand, column, &mut delivered);
        if arrivals.contains(&column) {
            assert_eq!(
                additions, 1,
                "the delivery tick also pairs the existing stand"
            );
            break;
        }
        assert_eq!(additions, 0, "an unsent column cannot spawn its stand");
    }
    assert!(
        delivered,
        "the acknowledged batches eventually deliver the stand's column"
    );
    assert_eq!(
        world.player_tracking_viewers_for_test(dimension, stand),
        vec![ConnectionId(1)]
    );
    for _ in 0..3 {
        let acknowledged = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick: 1.0,
            },
        );
        let idle = world.tick(receiver.drain());
        for tick in [&acknowledged, &idle] {
            assert_eq!(
                delivered_entity_additions(tick, stand, column, &mut delivered),
                0
            );
        }
    }
    assert_eq!(
        world
            .stored_entity_for_test(dimension, stand)
            .unwrap()
            .position(),
        position
    );
}

/// `EntityReference` over a real restart (`EntityReference.java:51-53`,
/// `:92-100`; `LivingEntity.java:744-746`, `:814-815`): a record carries the
/// UUID the reference itself holds, so an attacker that changed dimension
/// before the save round-trips that UUID, and the world still files that entity
/// under it in the level it moved to. The same-level reference resolves to the
/// entity it names; the cross-dimension one keeps the UUID, because
/// `ServerLevel.getEntityInAnyDimension` (`ServerLevel.java:1378-1394`) has no
/// counterpart the entity side can reach.
#[test]
fn a_cross_dimension_last_hurt_by_mob_reference_round_trips_a_real_restart() {
    const SOURCE: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    const TARGET: Java26BuiltinDimension = Java26BuiltinDimension::Nether;
    let root = std::env::temp_dir().join(format!(
        "bbb-entity-last-hurt-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-entity-last-hurt-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..creative_fixture_config()
    };
    let mut world = crate::tests::restore_stored(config.clone(), &candidate);
    let victim = spawn_fixture_zombie(&mut world, SOURCE, 1.5);
    let attacker = spawn_fixture_zombie(&mut world, SOURCE, 2.5);
    let staying_victim = spawn_fixture_zombie(&mut world, SOURCE, 3.5);
    let staying_attacker = spawn_fixture_zombie(&mut world, SOURCE, 4.5);
    let victim_uuid = stored_uuid(&world, SOURCE, victim);
    let attacker_uuid = stored_uuid(&world, SOURCE, attacker);
    let staying_victim_uuid = stored_uuid(&world, SOURCE, staying_victim);

    for (hurt, by) in [(victim, attacker), (staying_victim, staying_attacker)] {
        let outcome = world
            .hurt_stored_entity_by_command_for_test(
                SOURCE,
                hurt,
                "minecraft:generic",
                1.0,
                Some(stored_mob_damage_source(SOURCE, &world, by)),
            )
            .expect("the fixture mob takes command damage");
        assert!(outcome.applied && outcome.succeeded);
    }

    // The attacker leaves the level, and the level it left no longer holds it.
    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (0, 0));
    world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            attacker,
            StoredEntityTransition {
                position: Vec3d {
                    x: 3.5,
                    y: 70.0,
                    z: 3.5,
                },
                delta_movement: Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
        )
        .unwrap();
    assert!(world.stored_entity_for_test(SOURCE, attacker).is_none());

    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = storage.load_candidate().unwrap();
    let restored = crate::tests::restore_stored(config, &saved);
    let reloaded_victim = restored
        .live_entity_id_for_uuid_for_test(SOURCE, victim_uuid)
        .expect("the victim reloads in the level it stayed in");
    let reference = stored_living(&restored, SOURCE, reloaded_victim)
        .last_hurt_by_mob()
        .expect("the victim still remembers the blow");
    assert_eq!(
        reference.uuid(),
        Some(attacker_uuid),
        "the record carries the UUID the reference held, wherever its entity stands"
    );
    assert!(
        restored
            .live_entity_id_for_uuid_for_test(TARGET, attacker_uuid)
            .is_some(),
        "the attacker reloads in the dimension it moved to"
    );

    let reloaded_staying_victim = restored
        .live_entity_id_for_uuid_for_test(SOURCE, staying_victim_uuid)
        .expect("the staying victim reloads");
    let staying_reference = *stored_living(&restored, SOURCE, reloaded_staying_victim)
        .last_hurt_by_mob()
        .as_ref()
        .expect("the staying victim still remembers the blow");
    let staying_attacker_uuid = stored_uuid(&world, SOURCE, staying_attacker);
    assert!(
        restored
            .live_entity_id_for_uuid_for_test(SOURCE, staying_attacker_uuid)
            .is_some(),
        "the staying attacker reloads"
    );
    // Restored entities of a chunk nobody loads stay hidden, so the reference
    // keeps the UUID until a lookup can resolve it.
    assert_eq!(staying_reference.uuid(), Some(staying_attacker_uuid));

    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// One fixture zombie in `dimension`, at `x` on the fixture floor.
fn spawn_fixture_zombie(world: &mut ServerWorld, dimension: Java26BuiltinDimension, x: f64) -> i32 {
    world
        .spawn_stored_entity_for_test(
            dimension,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d { x, y: 64.0, z: 0.5 },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                    .expect("a zombie is living"),
            ),
        )
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

/// The `CommandDamageSourceEntity` a `/damage ... by <stored mob>` resolves to,
/// carrying the source's own UUID the way command resolution captures it.
fn stored_mob_damage_source(
    dimension: Java26BuiltinDimension,
    world: &ServerWorld,
    entity_id: i32,
) -> crate::commands::CommandDamageSourceEntity {
    crate::commands::CommandDamageSourceEntity::Stored {
        entity_id,
        uuid: Some(stored_uuid(world, dimension, entity_id)),
        position: Some(Vec3d::default()),
        entity_resource_id: "minecraft:zombie".to_string(),
        is_mob: true,
        is_living_non_player: true,
        display_name: NbtSummaryValue::string("zombie"),
    }
}

mod living_fall_particles;

mod native_ground_command_discard;

mod zombification;

mod armadillo_brushing;
mod armadillo_threats;
