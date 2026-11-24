use super::*;
use crate::entity::{spawn_armor_stand, EntityVisibility};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::{ServerWorld, ServerWorldConfig};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_server_persistence::{Java26WorldStorage, RegionCompression};
use bbb_server_types::save::{Java26BuiltinDimension, WorldSaveLimits, WorldSavePaths};

#[test]
fn loading_visibility_limits_simulation_beyond_server_view() {
    let mut world = ServerWorld::new(ServerWorldConfig {
        chunk_radius: 2,
        simulation_distance: 5,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "loading", 91);
    let dimension = Java26BuiltinDimension::Overworld.into();
    let player = world.levels.player_at(crate::connection::ConnectionId(1));
    let center = crate::world::player_packets::player_chunk(player.position());
    let far = (center.0 + 5, center.1);
    world
        .prepare_full_chunk_columns(&dimension, &[far])
        .unwrap();
    let position = Vec3d {
        x: f64::from(far.0 * 16) + 0.5,
        y: 80.0,
        z: f64::from(far.1 * 16) + 0.5,
    };
    let id = spawn_armor_stand(&mut world.level_mut_by_id(&dimension).entities, position).unwrap();
    let uuid = world
        .level_by_id(&dimension)
        .entities
        .get(id)
        .unwrap()
        .uuid();
    world.tick(receiver.drain());
    let level = world.level_by_id(&dimension);
    assert!(level.simulation_levels.is_entity_ticking(far));
    assert_eq!(
        level.chunk_lifecycle.status(far),
        FullChunkStatus::Inaccessible
    );
    assert!(!world
        .entity_ticking_chunks_by_key(&dimension)
        .contains(&far));
    assert!(!level.entities.is_ticking(id));
    assert!(level.entities.get_visible(id).is_none());
    assert_eq!(level.entities.get(id).unwrap().position(), position);
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .add_unknown(far, 33);
    world.refresh_loading_chunks(&dimension).unwrap();
    let level = world.level_by_id(&dimension);
    assert_eq!(level.chunk_lifecycle.status(far), FullChunkStatus::Full);
    assert_eq!(
        level
            .entities
            .section_visibility(crate::entity::SectionKey::new(far.0, 5, far.1)),
        EntityVisibility::Tracked
    );
    assert_eq!(level.entities.get_visible(id).unwrap().uuid(), uuid);
    assert!(!level.entities.is_ticking(id));
    for _ in 0..2 {
        world.tick(receiver.drain());
    }
    let level = world.level_by_id(&dimension);
    assert!(level.entities.get_visible(id).is_none());
    assert_eq!(level.entities.get(id).unwrap().uuid(), uuid);
}

#[test]
fn runtime_unload_saves_native_crafter_and_reloads_latest_entities() {
    let root = std::env::temp_dir().join(format!("bbb-runtime-unload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let candidate = create_saved_column(&storage);
    let mut world = crate::tests::restore_stored(ServerWorldConfig::default(), &candidate);
    let dimension = Java26BuiltinDimension::Overworld.into();
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    let block =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:crafter").unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    world.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
    let item = bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_LOG_ID),
        count: 3,
        component_patch: Default::default(),
    });
    world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .crafter_mut()
        .unwrap()
        .items_mut()[0] = item;
    let id = spawn_armor_stand(
        &mut world.level_mut_by_id(&dimension).entities,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    )
    .unwrap();
    let uuid = world
        .level_by_id(&dimension)
        .entities
        .get(id)
        .unwrap()
        .uuid();
    assert!(world
        .level_by_id(&dimension)
        .chunk_lifecycle
        .entities_loaded((0, 0)));
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .purge_unknown();
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .purge_unknown();
    world.refresh_loading_chunks(&dimension).unwrap();
    assert!(world.needs_runtime_unload_save());
    let request = world.save_request(2, 2, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .entity_loads
        .insert((0, 0), EntityChunkLoadStatus::Pending);
    world.retain_runtime_unload_columns(
        &world.level_key(dimension).clone(),
        vec![(
            (0, 0),
            storage
                .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )],
    );
    assert!(!world
        .level_by_id(&dimension)
        .chunks
        .has_saved_column_input((0, 0)));
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .entity_loads
        .insert((0, 0), EntityChunkLoadStatus::Loaded);
    world.retain_runtime_unload_columns(
        &world.level_key(dimension).clone(),
        vec![(
            (0, 0),
            storage
                .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )],
    );
    world.acknowledge_save_success(&request);
    assert!(world.level_by_id(&dimension).entities.get(id).is_none());
    assert!(!world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    assert!(world
        .level_by_id(&dimension)
        .chunks
        .cold_column(0, 0)
        .is_none());
    assert!(!world
        .level_by_id(&dimension)
        .chunks
        .has_saved_column_input((0, 0)));
    assert_eq!(
        crate::world::stored_block_state(&world.level_by_id(&dimension).chunks, pos),
        state
    );
    let subsequent = world.save_request(3, 3, RegionCompression::Zlib).unwrap();
    assert!(!subsequent.entity_writes.iter().any(|write| write.dimension
        == world.level_key(dimension).clone()
        && (write.chunk_x, write.chunk_z) == (0, 0)));
    storage.save_snapshot(&candidate, &subsequent).unwrap();
    world.acknowledge_save_success(&subsequent);
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .add_unknown((0, 0), 33);
    // A world that waits for its generation reads the column itself.
    world.refresh_loading_chunks(&dimension).unwrap();
    assert!(world.pending_runtime_columns().is_empty());
    assert!(world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    world.refresh_loading_chunks(&dimension).unwrap();
    world
        .complete_runtime_entity_chunk(
            &world.level_key(dimension).clone(),
            (0, 0),
            storage
                .read_runtime_entity_chunk(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )
        .unwrap();
    let level = world.level_by_id(&dimension);
    let restored = level.chunks.block_entities().get(pos).unwrap();
    let stack = &restored.crafter().unwrap().items()[0];
    assert_eq!(stack.count(), 3);
    assert_eq!(
        stack.item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_OAK_LOG_ID)
    );
    let restored_id = level.entities.visible_entity_id_for_uuid(uuid).unwrap();
    assert!(!level.entities.is_ticking(restored_id));
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .purge_unknown();
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .purge_unknown();
    world.refresh_loading_chunks(&dimension).unwrap();
    let request = world.save_request(4, 4, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let cached = storage
        .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
        .unwrap();
    world
        .retain_runtime_unload_columns(&world.level_key(dimension).clone(), vec![((0, 0), cached)]);
    world.acknowledge_save_success(&request);
    assert!(!world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    let written = BlockPos { x: 9, y: 65, z: 8 };
    let stone =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone").unwrap();
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(stone.id).unwrap();
    assert!(world.with_level(dimension, |level| {
        level.set_block(
            written,
            stone,
            SET_BLOCK_UPDATE_ALL,
            crate::world::UPDATE_LIMIT,
        )
    }));
    let native = world
        .level_by_id(&dimension)
        .chunks
        .block_entities()
        .get(pos)
        .unwrap();
    assert_eq!(native.crafter().unwrap().items()[0].count(), 3);
    let request = world.save_request(5, 5, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let latest = storage
        .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
        .unwrap();
    let mut verified = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    verified
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .evicted_columns
        .insert((0, 0));
    assert!(verified
        .complete_runtime_columns(
            &verified.level_key(dimension).clone(),
            vec![((0, 0), Ok(latest))],
        )
        .is_empty());
    assert_eq!(
        verified.block_state_by_key(&dimension, written),
        Some(stone)
    );
    assert_eq!(
        verified
            .get_block_entity_by_key(&dimension, pos)
            .unwrap()
            .crafter()
            .unwrap()
            .items()[0]
            .count(),
        3
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn create_saved_column(
    storage: &Java26WorldStorage,
) -> bbb_server_persistence::Java26WorldCandidate {
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let column = crate::world_creation::default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = crate::world_creation::created_save(
        &bbb_server_persistence::Java26WorldCreation {
            level_name: "runtime-unload".into(),
            game_type: 0,
            difficulty: "normal".into(),
            spawn_position: [0, 64, 0],
            last_played: 1,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 1)
        .unwrap();
    storage.load_candidate().unwrap()
}

#[test]
fn save_off_keeps_native_column_but_stores_hidden_entities() {
    let root = std::env::temp_dir().join(format!("bbb-runtime-save-off-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let candidate = create_saved_column(&storage);
    let level_before = std::fs::read(storage.paths().level_dat()).unwrap();
    let mut world = crate::tests::restore_stored(ServerWorldConfig::default(), &candidate);
    let dimension = Java26BuiltinDimension::Overworld.into();
    let pos = BlockPos { x: 8, y: 64, z: 8 };
    let block =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:crafter").unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    world.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
    let id = spawn_armor_stand(
        &mut world.level_mut_by_id(&dimension).entities,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    )
    .unwrap();
    world.auto_save = false;
    for _ in 0..2 {
        world
            .level_mut_by_id(&dimension)
            .chunk_lifecycle
            .purge_unknown();
    }
    world.refresh_loading_chunks(&dimension).unwrap();
    let request = world
        .runtime_unload_save_request(2, RegionCompression::Zlib)
        .unwrap();
    assert!(request.chunk_writes.is_empty());
    assert!(request.poi_edits.is_empty() && request.player_patches.is_empty());
    assert!(request
        .entity_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (0, 0)));
    storage.save_runtime_unloads(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    assert!(world.level_by_id(&dimension).entities.get(id).is_none());
    assert!(world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    assert!(world
        .level_by_id(&dimension)
        .chunks
        .block_entities()
        .get(pos)
        .is_some());
    assert_eq!(
        std::fs::read(storage.paths().level_dat()).unwrap(),
        level_before
    );
    assert!(!world.needs_runtime_unload_save());
    world
        .level_mut_by_id(&dimension)
        .chunk_lifecycle
        .add_unknown((0, 0), 33);
    world.refresh_loading_chunks(&dimension).unwrap();
    assert!(!world
        .level_by_id(&dimension)
        .chunk_lifecycle
        .column_unloads
        .contains(&(0, 0)));
    world
        .complete_runtime_entity_chunk(
            &world.level_key(dimension).clone(),
            (0, 0),
            storage
                .read_runtime_entity_chunk(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )
        .unwrap();
    for _ in 0..2 {
        world
            .level_mut_by_id(&dimension)
            .chunk_lifecycle
            .purge_unknown();
    }
    world.refresh_loading_chunks(&dimension).unwrap();
    world.auto_save = true;
    assert!(world.needs_runtime_unload_save());
    let request = world
        .runtime_unload_save_request(3, RegionCompression::Zlib)
        .unwrap();
    storage.save_runtime_unloads(&candidate, &request).unwrap();
    world.retain_runtime_unload_columns(
        &world.level_key(dimension).clone(),
        vec![(
            (0, 0),
            storage
                .read_runtime_column(&candidate, &world.level_key(dimension).clone(), 0, 0)
                .unwrap(),
        )],
    );
    world.acknowledge_save_success(&request);
    assert!(!world.level_by_id(&dimension).chunks.is_loaded(0, 0));
    assert_eq!(
        std::fs::read(storage.paths().level_dat()).unwrap(),
        level_before
    );
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_ticket_admission_changes_on_move_after_mode_and_teleport() {
    use crate::connection::ConnectionId;
    use bbb_protocol::game_types::GameType;
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "admission", 92);
    let connection = ConnectionId(1);
    let dimension = world.levels[connection].dimension;
    let filed = world.levels[connection].cold.last_section_pos;
    let mut events = Vec::new();
    world.set_game_rule_value(
        crate::world::RULE_SPECTATORS_GENERATE_CHUNKS,
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
        &mut events,
    );
    world.set_player_game_mode(connection, GameType::Spectator, &mut events);
    world.refresh_simulation_chunk_levels_by_key(&dimension);
    assert!(world
        .level_by_id(&dimension)
        .simulation_levels
        .is_entity_ticking((filed.0, filed.2)));
    assert!(!world.levels[connection].cold.filed_ticket_ignored);
    world.move_player_chunk_tracking(connection, &mut events);
    assert!(world.levels[connection].cold.filed_ticket_ignored);
    world.refresh_simulation_chunk_levels_by_key(&dimension);
    assert!(!world
        .level_by_id(&dimension)
        .simulation_levels
        .is_entity_ticking((filed.0, filed.2)));
    world.set_player_game_mode(connection, GameType::Survival, &mut events);
    let old = world.levels.player_at(connection);
    let target = Vec3d {
        x: old.position().x + 256.0,
        ..old.position()
    };
    world.teleport_set_position(
        connection,
        &crate::world::TeleportTransition {
            new_level: dimension,
            position: target,
            y_rot: old.rotation().y_rot,
            x_rot: old.rotation().x_rot,
            relatives_mask: 0,
        },
        &mut events,
    );
    world.update_chunk_tracking(connection, &mut events);
    assert_eq!(world.levels[connection].cold.last_section_pos, filed);
    assert!(world.levels[connection].cold.filed_ticket_ignored);
    let id = world.levels[connection].awaiting_teleport.unwrap();
    world.apply_play(
        connection,
        bbb_protocol::packets::PlayServerbound::AcceptTeleportation { id },
        &mut events,
    );
    assert!(!world.levels[connection].cold.filed_ticket_ignored);
    let new = world.levels[connection].cold.last_section_pos;
    assert_ne!(new.0, filed.0);
    world.refresh_simulation_chunk_levels_by_key(&dimension);
    assert!(world
        .level_by_id(&dimension)
        .simulation_levels
        .is_entity_ticking((new.0, new.2)));
    assert!(!world
        .level_by_id(&dimension)
        .simulation_levels
        .is_entity_ticking((filed.0, filed.2)));
}
