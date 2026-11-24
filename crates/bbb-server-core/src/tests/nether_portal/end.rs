use super::*;

fn end_state() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:end_portal", &BTreeMap::new()).unwrap()
}

pub(super) fn clear_cooldown(world: &mut ServerWorld, dimension: Java26BuiltinDimension, id: i32) {
    let NbtSummaryValue::Compound(mut record) = world
        .stored_entity_command_data_for_test(dimension, id)
        .unwrap()
    else {
        unreachable!()
    };
    record.retain(|entry| entry.name != "PortalCooldown");
    record.push(NbtSummaryEntry {
        name: "PortalCooldown".into(),
        value: NbtSummaryValue::Int(0),
    });
    world.load_stored_entity_record_for_test(dimension, id, &record);
}

/// FallingBlockEntity.java:157,356-363: sand landing inside an End portal
/// goes to the End and its old copy still lands, dropping a sand item.
#[test]
fn falling_sand_through_an_end_portal_leaves_a_landed_copy_behind() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(source, entry).unwrap();
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &[(BlockPos { y: 63, ..entry }, stone), (entry, end_state())],
    );
    let sand =
        vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_SAND_ID).unwrap();
    let (id, uuid) = world.spawn_stored_entity_for_test(
        source,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::FallingBlock(crate::entity::FallingBlockData::new(sand, entry)),
    );
    clear_cooldown(&mut world, source, id);
    // The dropped item sits in the portal too, so look before it follows.
    world.tick(Vec::new());
    assert!(world
        .live_entity_id_for_uuid_for_test(target, uuid)
        .is_some());
    assert!(world
        .stored_items_for_test(source)
        .into_iter()
        .any(|(_, _, id, count, _, _)| id == "minecraft:sand" && count == 1));
}

#[test]
fn stored_end_portal_tick_creates_platform_and_returns_to_world_spawn() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(source, entry).unwrap();
    set_portal_blocks(&mut world, source, &[(entry, end_state())]);
    let (id, uuid) = world.spawn_stored_entity_for_test(
        source,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    clear_cooldown(&mut world, source, id);
    world
        .stored_entity_mut_for_test(source, id)
        .unwrap()
        .set_rot(30.0, 10.0);
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(source, id)
            .unwrap()
            .portal_process()
            .map(|p| p.kind()),
        Some(crate::entity::portal::PortalKind::End)
    );
    world.tick(Vec::new());
    assert!(
        world.stored_entity_for_test(source, id).is_none(),
        "{:?}",
        world.stored_entity_for_test(source, id)
    );
    let id = world
        .live_entity_id_for_uuid_for_test(target, uuid)
        .unwrap();
    let body = world.stored_entity_for_test(target, id).unwrap();
    assert_eq!((body.position().x, body.position().z), (100.5, 0.5));
    assert!((49.0..=50.0).contains(&body.position().y));
    assert_eq!((body.y_rot(), body.x_rot()), (90.0, 10.0));
    for dz in -2..=2 {
        for dx in -2..=2 {
            assert_eq!(
                live_state(
                    &world,
                    target,
                    BlockPos {
                        x: 100 + dx,
                        y: 48,
                        z: dz
                    }
                ),
                vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap()
            );
            for y in 49..=51 {
                assert_eq!(
                    live_state(
                        &world,
                        target,
                        BlockPos {
                            x: 100 + dx,
                            y,
                            z: dz
                        }
                    ),
                    bbb_server_types::fluid::air_block_state_id()
                );
            }
        }
    }
    assert!(world
        .saved_chunk_tickets_for_test(target)
        .iter()
        .any(|(chunk, level, _)| *chunk == (6, 0) && *level == 30));

    let exit = BlockPos {
        x: 100,
        y: 50,
        z: 0,
    };
    set_portal_blocks(&mut world, target, &[(exit, end_state())]);
    world.set_world_spawn(
        &target.into(),
        BlockPos {
            x: 100,
            y: 49,
            z: 0,
        },
        13.0,
        4.0,
        &mut Vec::new(),
    );
    clear_cooldown(&mut world, target, id);
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert_eq!(
        world.live_entity_id_for_uuid_for_test(target, uuid),
        Some(id)
    );
    let body = world.stored_entity_for_test(target, id).unwrap();
    assert_eq!((body.y_rot(), body.x_rot()), (103.0, 14.0));
    let height = world
        .chunk_store_for_test(target)
        .final_heightmap_first_available(
            6,
            0,
            bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            4,
            0,
        )
        .unwrap();
    assert_eq!(
        body.position(),
        Vec3d {
            x: 100.5,
            y: f64::from(height),
            z: 0.5
        }
    );
    let exit = body.block_position();
    set_portal_blocks(&mut world, target, &[(exit, end_state())]);
    world.set_world_spawn(
        &Java26BuiltinDimension::Nether.into(),
        BlockPos { x: 0, y: 64, z: 0 },
        0.0,
        0.0,
        &mut Vec::new(),
    );
    assert!(world.set_game_rule_for_test(
        "minecraft:allow_entering_nether_using_portals",
        GameRuleValue::Boolean(false),
    ));
    clear_cooldown(&mut world, target, id);
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert_eq!(
        world.live_entity_id_for_uuid_for_test(target, uuid),
        Some(id)
    );
    assert_eq!(
        world
            .stored_entity_for_test(target, id)
            .unwrap()
            .portal_cooldown(),
        300
    );
    assert!(world
        .saved_chunk_tickets_for_test(Java26BuiltinDimension::Nether)
        .is_empty());
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    world.set_world_spawn(
        &source.into(),
        BlockPos { x, y, z },
        0.0,
        0.0,
        &mut Vec::new(),
    );
    clear_cooldown(&mut world, target, id);
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(target, id)
            .unwrap()
            .portal_process()
            .map(|p| p.kind()),
        Some(crate::entity::portal::PortalKind::End)
    );
    world.tick(Vec::new());
    assert!(world.stored_entity_for_test(target, id).is_none());
    let returned = world
        .live_entity_id_for_uuid_for_test(source, uuid)
        .unwrap();
    let body = world.stored_entity_for_test(source, returned).unwrap();
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let spawn = BlockPos { x, y, z };
    let height = world
        .chunk_store_for_test(source)
        .final_heightmap_first_available(
            spawn.x >> 4,
            spawn.z >> 4,
            bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            spawn.x & 15,
            spawn.z & 15,
        )
        .unwrap();
    assert_eq!(
        body.position(),
        Vec3d {
            x: f64::from(spawn.x) + 0.5,
            y: f64::from(height),
            z: f64::from(spawn.z) + 0.5
        }
    );
    assert_eq!((body.y_rot(), body.x_rot()), (103.0, 14.0));
}

#[test]
fn stored_end_portal_cargo_tree_platform_drops_survive_storage_restart() {
    let root = std::env::temp_dir().join(format!("bbb-end-portal-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-end-portal-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    };
    let mut world = ServerWorld::restore(config.clone(), &candidate).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 4, y: 64, z: 4 };
    world.place_portal_ticket(source, entry).unwrap();
    set_portal_blocks(&mut world, source, &[(entry, end_state())]);
    world
        .place_portal_ticket(
            target,
            BlockPos {
                x: 100,
                y: 50,
                z: 0,
            },
        )
        .unwrap();
    set_portal_blocks(
        &mut world,
        target,
        &[(
            BlockPos {
                x: 100,
                y: 49,
                z: 0,
            },
            vanilla_block_state_id_by_name_and_properties("minecraft:dirt", &BTreeMap::new())
                .unwrap(),
        )],
    );
    let (boat, rider) = world
        .spawn_stored_entity_transfer_tree_for_test(
            source,
            Vec3d {
                x: 4.5,
                y: 64.75,
                z: 4.5,
            },
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 4.5,
            },
        )
        .unwrap();
    let boat_uuid = world.stored_entity_for_test(source, boat).unwrap().uuid();
    let rider_uuid = world.stored_entity_for_test(source, rider).unwrap().uuid();
    world.tick(Vec::new());
    for (key, (x, z)) in world.pending_runtime_entity_chunks() {
        let record = storage
            .read_runtime_entity_chunk(&candidate, &key, x, z)
            .unwrap();
        world
            .complete_runtime_entity_chunk(&key, (x, z), record)
            .unwrap();
    }
    assert_eq!(
        world
            .stored_entity_for_test(source, boat)
            .unwrap()
            .portal_process()
            .map(|p| p.kind()),
        Some(crate::entity::portal::PortalKind::End)
    );
    world.tick(Vec::new());
    for (key, (x, z)) in world.pending_runtime_entity_chunks() {
        let record = storage
            .read_runtime_entity_chunk(&candidate, &key, x, z)
            .unwrap();
        world
            .complete_runtime_entity_chunk(&key, (x, z), record)
            .unwrap();
    }
    assert!(world
        .live_entity_id_for_uuid_for_test(source, boat_uuid)
        .is_none());
    assert!(world
        .live_entity_id_for_uuid_for_test(source, rider_uuid)
        .is_none());
    let boat = world
        .live_entity_id_for_uuid_for_test(target, boat_uuid)
        .unwrap();
    let rider = world
        .live_entity_id_for_uuid_for_test(target, rider_uuid)
        .unwrap();
    assert_eq!(world.stored_passengers_for_test(target, boat), &[rider]);
    assert!(world
        .stored_items_for_test(target)
        .iter()
        .any(|(_, _, item, count, _, _)| item == "minecraft:dirt" && *count == 1));
    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = storage.load_candidate().unwrap();
    let restored = ServerWorld::restore(config, &saved).unwrap();
    assert!(restored
        .live_entity_id_for_uuid_for_test(source, boat_uuid)
        .is_none());
    let boat = restored
        .live_entity_id_for_uuid_for_test(target, boat_uuid)
        .expect("restored boat");
    let rider = restored
        .live_entity_id_for_uuid_for_test(target, rider_uuid)
        .unwrap();
    assert_eq!(restored.stored_passengers_for_test(target, boat), &[rider]);
    let cargo = crate::entity::chest_boat::chest_boat(
        restored
            .stored_entity_for_test(target, boat)
            .unwrap()
            .data(),
    )
    .unwrap()
    .item(26)
    .unwrap();
    assert_eq!(cargo.item_resource_id(), Some("minecraft:diamond"));
    assert_eq!(cargo.count(), 7);
    assert_eq!(
        live_state(
            &restored,
            target,
            BlockPos {
                x: 100,
                y: 48,
                z: 0
            }
        ),
        vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap()
    );
    assert!(restored
        .stored_items_for_test(target)
        .iter()
        .any(|(_, _, item, count, _, _)| item == "minecraft:dirt" && *count == 1));
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stored_end_portal_same_level_tnt_uses_portal_explosion_behavior() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(dimension, entry).unwrap();
    set_portal_blocks(&mut world, dimension, &[(entry, end_state())]);
    world.set_world_spawn(
        &dimension.into(),
        BlockPos {
            x: 100,
            y: 50,
            z: 0,
        },
        0.0,
        0.0,
        &mut Vec::new(),
    );
    let (id, uuid) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::Tnt(crate::entity::TntData::new()),
    );
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert_eq!(
        world.live_entity_id_for_uuid_for_test(dimension, uuid),
        Some(id)
    );
    let body = world.stored_entity_for_test(dimension, id).unwrap();
    assert_eq!((body.position().x, body.position().z), (100.5, 0.5));
    let crate::entity::EntityData::Tnt(tnt) = body.data() else {
        unreachable!()
    };
    assert!(tnt.used_portal());
}
