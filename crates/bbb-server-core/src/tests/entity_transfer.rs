//! Cross-level stored passenger-tree admission, projection and restart.

use std::collections::BTreeSet;

use super::*;
use crate::entity::{chest_boat, Entity, StoredEntityTransition};

const SOURCE: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const TARGET: Java26BuiltinDimension = Java26BuiltinDimension::Nether;

fn position(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

fn absolute_transition(position: Vec3d) -> StoredEntityTransition {
    StoredEntityTransition {
        position,
        delta_movement: Vec3d::default(),
        y_rot: 90.0,
        x_rot: -10.0,
        relatives_mask: 0,
    }
}

fn relative_transition(offset: Vec3d) -> StoredEntityTransition {
    StoredEntityTransition {
        position: offset,
        delta_movement: position(0.25, -0.5, 0.75),
        y_rot: 15.0,
        x_rot: -5.0,
        relatives_mask: PLAYER_RELATIVE_X
            | PLAYER_RELATIVE_Y
            | PLAYER_RELATIVE_Z
            | PLAYER_RELATIVE_Y_ROT
            | PLAYER_RELATIVE_X_ROT
            | PLAYER_RELATIVE_DELTA_X
            | PLAYER_RELATIVE_DELTA_Y
            | PLAYER_RELATIVE_DELTA_Z
            | bbb_protocol::packets::PLAYER_RELATIVE_ROTATE_DELTA,
    }
}

fn spawn_tree(
    world: &mut ServerWorld,
    root_position: Vec3d,
    passenger_position: Vec3d,
) -> (i32, i32) {
    world.ensure_entity_transfer_full_chunk_for_test(
        SOURCE,
        (
            root_position.x.floor() as i32 >> 4,
            root_position.z.floor() as i32 >> 4,
        ),
    );
    world
        .spawn_stored_entity_transfer_tree_for_test(SOURCE, root_position, passenger_position)
        .expect("the stored cargo tree is staged and published")
}

fn assert_cargo(entity: &Entity) {
    let cargo = chest_boat::chest_boat(entity.data())
        .expect("the replacement retains chest-boat data")
        .item(26)
        .expect("the replacement retains slot 26");
    assert_eq!(cargo.item_resource_id(), Some("minecraft:diamond"));
    assert_eq!(cargo.count(), 7);
}

fn entity_count(
    request: &Java26SaveRequest,
    dimension: Java26BuiltinDimension,
    chunk: (i32, i32),
) -> usize {
    let write = request
        .entity_writes
        .iter()
        .find(|write| {
            write.dimension == level_key(dimension) && (write.chunk_x, write.chunk_z) == chunk
        })
        .unwrap_or_else(|| panic!("entity write for {dimension:?} {chunk:?}"));
    Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
        .expect("the projected entity document is canonical")
        .entities()
        .len()
}

#[test]
fn absolute_transfer_replaces_ids_relations_and_projects_an_empty_source_chunk() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let old_root_position = position(1.5, 64.0, 1.5);
    let old_passenger_position = position(3.5, 66.0, 5.5);
    let (old_root, old_passenger) =
        spawn_tree(&mut world, old_root_position, old_passenger_position);
    world.move_stored_entity_for_transfer_test(SOURCE, old_root, old_root_position, 10.0, 20.0);
    world.move_stored_entity_for_transfer_test(
        SOURCE,
        old_passenger,
        old_passenger_position,
        40.0,
        30.0,
    );
    let root_uuid = world
        .stored_entity_for_test(SOURCE, old_root)
        .unwrap()
        .uuid();
    let passenger_uuid = world
        .stored_entity_for_test(SOURCE, old_passenger)
        .unwrap()
        .uuid();
    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (2, -2));

    let outcome = world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            old_root,
            absolute_transition(position(33.5, 70.0, -30.5)),
        )
        .unwrap();
    let new_root = outcome.id_map[&old_root];
    let new_passenger = outcome.id_map[&old_passenger];
    assert_eq!(outcome.old_root_id, old_root);
    assert_eq!(outcome.new_root_id, new_root);
    assert_ne!(new_root, old_root);
    assert_ne!(new_passenger, old_passenger);
    assert!(world.stored_entity_for_test(SOURCE, old_root).is_none());
    assert!(world
        .stored_entity_for_test(SOURCE, old_passenger)
        .is_none());
    let root = world.stored_entity_for_test(TARGET, new_root).unwrap();
    let passenger = world.stored_entity_for_test(TARGET, new_passenger).unwrap();
    assert_eq!(root.uuid(), root_uuid);
    assert_eq!(passenger.uuid(), passenger_uuid);
    assert_eq!(root.position(), position(33.5, 70.0, -30.5));
    assert_eq!(passenger.position(), position(35.5, 72.0, -26.5));
    assert_eq!((root.y_rot(), root.x_rot()), (90.0, -10.0));
    assert_eq!((passenger.y_rot(), passenger.x_rot()), (120.0, 0.0));
    assert_eq!(
        world.stored_passengers_for_test(TARGET, new_root),
        &[new_passenger]
    );
    assert_cargo(root);
    assert_eq!(
        world.saved_entity_chunks_for_transfer_test(SOURCE),
        BTreeSet::from([(0, 0)])
    );
    assert!(world.saved_chunk_tickets_for_test(TARGET).is_empty());
    assert!(world.entity_ticking_chunks(TARGET).is_empty());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(entity_count(&request, SOURCE, (0, 0)), 0);
    assert_eq!(entity_count(&request, TARGET, (2, -2)), 1);
}

#[test]
fn relative_transfer_preflights_every_resulting_chunk_and_preserves_offsets() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let root_position = position(1.5, 64.0, 1.5);
    let passenger_position = position(18.5, 66.0, 1.5);
    let (old_root, old_passenger) = spawn_tree(&mut world, root_position, passenger_position);
    world.move_stored_entity_for_transfer_test(SOURCE, old_root, root_position, 10.0, 20.0);
    world.move_stored_entity_for_transfer_test(
        SOURCE,
        old_passenger,
        passenger_position,
        40.0,
        30.0,
    );
    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (2, -1));
    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (3, -1));

    let outcome = world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            old_root,
            relative_transition(position(32.0, 4.0, -16.0)),
        )
        .unwrap();
    let root = world
        .stored_entity_for_test(TARGET, outcome.id_map[&old_root])
        .unwrap();
    let passenger = world
        .stored_entity_for_test(TARGET, outcome.id_map[&old_passenger])
        .unwrap();
    assert_eq!(root.position(), position(33.5, 68.0, -14.5));
    assert_eq!(passenger.position(), position(50.5, 70.0, -14.5));
    assert_eq!((root.y_rot(), root.x_rot()), (25.0, 15.0));
    assert_eq!((passenger.y_rot(), passenger.x_rot()), (55.0, 25.0));
    assert_eq!(
        world.saved_entity_chunks_for_transfer_test(SOURCE),
        BTreeSet::from([(0, 0), (1, 0)])
    );
}

#[test]
fn transferred_cargo_tree_survives_real_storage_restart_only_in_target_level() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-entity-transfer-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-entity-transfer-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    };
    let mut world = crate::tests::restore_stored(config.clone(), &candidate);
    let (source_root, source_passenger) = spawn_tree(
        &mut world,
        position(1.5, 64.0, 1.5),
        position(2.5, 65.0, 1.5),
    );
    let root_uuid = world
        .stored_entity_for_test(SOURCE, source_root)
        .unwrap()
        .uuid();
    let passenger_uuid = world
        .stored_entity_for_test(SOURCE, source_passenger)
        .unwrap()
        .uuid();
    world.ensure_entity_transfer_full_chunk_for_test(TARGET, (0, 0));
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let source_saved = storage.load_candidate().unwrap();
    let mut world = crate::tests::restore_stored(config.clone(), &source_saved);
    let source_root = world
        .live_entity_id_for_uuid_for_test(SOURCE, root_uuid)
        .expect("the source tree reopened before transfer");
    world
        .transfer_stored_entity_tree_cross_level(
            SOURCE,
            TARGET,
            source_root,
            absolute_transition(position(3.5, 70.0, 3.5)),
        )
        .unwrap();
    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&source_saved, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let target_saved = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert_eq!(
        target_saved.dimensions[&level_key(SOURCE)].entity_chunks[&(0, 0)]
            .data
            .entities()
            .len(),
        0
    );
    assert_eq!(
        target_saved.dimensions[&level_key(TARGET)].entity_chunks[&(0, 0)]
            .data
            .entities()
            .len(),
        1
    );
    let restored = crate::tests::restore_stored(config, &target_saved);
    assert!(restored
        .live_entity_id_for_uuid_for_test(SOURCE, root_uuid)
        .is_none());
    assert!(restored
        .live_entity_id_for_uuid_for_test(SOURCE, passenger_uuid)
        .is_none());
    let target_root = restored
        .live_entity_id_for_uuid_for_test(TARGET, root_uuid)
        .expect("the target root reopened");
    let target_passenger = restored
        .live_entity_id_for_uuid_for_test(TARGET, passenger_uuid)
        .expect("the target passenger reopened");
    assert_eq!(
        restored.stored_passengers_for_test(TARGET, target_root),
        &[target_passenger]
    );
    assert_cargo(
        restored
            .stored_entity_for_test(TARGET, target_root)
            .unwrap(),
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
