use super::*;
use crate::block_entity::BlockEntity;

fn gateway(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    pos: BlockPos,
    exit: Option<BlockPos>,
    exact: bool,
) {
    world.place_portal_ticket(dimension, pos).unwrap();
    assert!(world.set_block(
        dimension,
        pos,
        vanilla_block_state_id_by_name_and_properties("minecraft:end_gateway", &BTreeMap::new())
            .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    if let Some(exit) = exit {
        world
            .level_block_entity_for_test(dimension, pos)
            .unwrap()
            .end_gateway_mut()
            .unwrap()
            .set_exit_position(exit, exact);
    }
}

#[test]
fn stored_end_gateway_contact_marks_cools_and_teleports_next_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    let entry = BlockPos { x: 0, y: 80, z: 0 };
    let exit = BlockPos { x: 64, y: 80, z: 0 };
    gateway(&mut world, dimension, entry, Some(exit), true);
    let (id, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 80.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    super::end::clear_cooldown(&mut world, dimension, id);
    world
        .stored_entity_mut_for_test(dimension, id)
        .unwrap()
        .set_rot(35.0, 12.0);
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(dimension, id)
            .unwrap()
            .portal_process()
            .unwrap()
            .kind(),
        crate::entity::portal::PortalKind::EndGateway
    );
    assert!(world
        .level_block_entity_for_test(dimension, entry)
        .unwrap()
        .end_gateway()
        .unwrap()
        .is_cooling_down());
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, entry)
            .unwrap()
            .end_gateway()
            .unwrap()
            .age(),
        1
    );
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(dimension, id).unwrap();
    assert_eq!((body.position().x, body.position().z), (64.5, 0.5));
    assert_eq!((body.y_rot(), body.x_rot()), (35.0, 12.0));
    assert_eq!(body.portal_cooldown(), 300);
    assert!(world
        .saved_chunk_tickets_for_test(dimension)
        .iter()
        .any(|ticket| ticket.0 == (4, 0)));
}

#[test]
fn stored_end_gateway_creates_return_structure_and_saves_both_links() {
    let root = std::env::temp_dir().join(format!("bbb-end-gateway-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-end-gateway-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    };
    let mut world = ServerWorld::restore(config.clone(), &candidate).unwrap();
    let dimension = Java26BuiltinDimension::End;
    let entry = BlockPos { x: 0, y: 80, z: 0 };
    gateway(&mut world, dimension, entry, None, false);
    let (id, uuid) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 80.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    super::end::clear_cooldown(&mut world, dimension, id);
    world.tick(Vec::new());
    world.tick(Vec::new());
    let exit = world
        .level_block_entity_for_test(dimension, entry)
        .and_then(BlockEntity::end_gateway_mut)
        .unwrap()
        .exit_portal()
        .unwrap();
    let returned = world
        .level_block_entity_for_test(dimension, exit)
        .unwrap()
        .end_gateway()
        .unwrap();
    assert_eq!(returned.exit_portal(), Some(entry));
    assert!(!returned.exact_teleport());
    let body = world.stored_entity_for_test(dimension, id).unwrap();
    assert_eq!(body.portal_cooldown(), 300);
    let arrival = body.position();
    for (key, (x, z)) in world.pending_runtime_entity_chunks() {
        let record = storage
            .read_runtime_entity_chunk(&candidate, &key, x, z)
            .unwrap();
        world
            .complete_runtime_entity_chunk(&key, (x, z), record)
            .unwrap();
    }

    storage
        .save_snapshot(
            &candidate,
            &world
                .save_request(1786000000000, 1, RegionCompression::Zlib)
                .unwrap(),
        )
        .unwrap();
    storage.close().unwrap();
    drop(world);
    let reopened = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = reopened.load_candidate().unwrap();
    let mut world = ServerWorld::restore(config, &candidate).unwrap();
    world.place_portal_ticket(dimension, entry).unwrap();
    world.place_portal_ticket(dimension, exit).unwrap();
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, entry)
            .unwrap()
            .end_gateway()
            .unwrap()
            .exit_portal(),
        Some(exit)
    );
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, exit)
            .unwrap()
            .end_gateway()
            .unwrap()
            .exit_portal(),
        Some(entry)
    );
    assert!(!world
        .level_block_entity_for_test(dimension, entry)
        .unwrap()
        .end_gateway()
        .unwrap()
        .is_cooling_down());
    let id = world
        .live_entity_id_for_uuid_for_test(dimension, uuid)
        .unwrap();
    assert_eq!(
        world
            .stored_entity_for_test(dimension, id)
            .unwrap()
            .position(),
        arrival
    );
    reopened.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stored_end_gateway_in_nether_respects_entry_rule_after_cooldown() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::Nether;
    let entry = BlockPos { x: 0, y: 80, z: 0 };
    let exit = BlockPos { x: 64, y: 80, z: 0 };
    gateway(&mut world, dimension, entry, Some(exit), true);
    assert!(world.set_game_rule_for_test(
        "minecraft:allow_entering_nether_using_portals",
        GameRuleValue::Boolean(false),
    ));
    let (id, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 80.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    super::end::clear_cooldown(&mut world, dimension, id);
    world.tick(Vec::new());
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(dimension, id).unwrap();
    assert_eq!((body.position().x, body.position().z), (0.5, 0.5));
    assert_eq!(body.portal_cooldown(), 300);
    assert!(!world
        .saved_chunk_tickets_for_test(dimension)
        .iter()
        .any(|ticket| ticket.0 == (4, 0)));
}
