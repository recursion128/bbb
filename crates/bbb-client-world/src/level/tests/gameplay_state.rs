use super::*;

#[test]
fn clear_client_level_removes_level_bound_state_without_resetting_packet_counters() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 256,
    });
    store.local_player_old_pose = Some(LocalPlayerPoseState::default());
    store.level = Some(WorldLevelInfo {
        dimension: "minecraft:the_nether".to_string(),
        dimension_type_id: 1,
        dimension_type_name: Some("minecraft:the_nether".to_string()),
        dimension_type_properties: WorldDimensionTypeProperties::default(),
        biome_zoom_seed: 0,
        default_clock_resource_id: None,
        timeline_resource_ids: Vec::new(),
        has_sky_light: false,
        has_ceiling: true,
        cardinal_lighting: WorldCardinalLighting::Nether,
        skybox: WorldSkybox::None,
        ambient_light: 0.1,
        environment_attributes: ClientEnvironmentAttributes::default(),
        last_death_location: None,
        sea_level: 32,
        is_debug: false,
        is_flat: false,
    });
    store
        .available_levels
        .insert("minecraft:the_nether".to_string());
    store.server_enforces_secure_chat = Some(true);
    store.chunks.push(stale_chunk().into());
    store.first_chunk = Some(ChunkPos { x: 1, z: -2 });
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_block_destruction(ProtocolBlockDestruction {
        id: 4,
        pos: ProtocolBlockPos {
            x: 12,
            y: 64,
            z: -5,
        },
        progress: 6,
    }));
    store.apply_block_event(ProtocolBlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    });
    store.apply_level_event(ProtocolLevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    });
    store.apply_level_event(ProtocolLevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 6, y: 7, z: 8 },
        data: 12,
        global: false,
    });
    assert_eq!(store.playing_jukebox_songs().len(), 1);
    store.apply_world_time(ProtocolPlayTime {
        game_time: 123,
        clock_updates: Vec::new(),
    });
    store.apply_game_event(ProtocolGameEvent {
        event_id: 7,
        param: 0.5,
    });
    store.apply_ticking_state(ProtocolTickingState {
        tick_rate: 0.25,
        frozen: true,
    });
    store.apply_ticking_step(ProtocolTickingStep { tick_steps: 7 });
    store.local_player_id = Some(123);
    store.local_player_vehicle_id = Some(456);
    store.set_local_using_item(true);
    store.counters.play_logins_received = 1;

    store.clear_client_level();

    assert_eq!(store.dimension(), WorldDimension::default());
    assert_eq!(store.client_level_generation(), 1);
    assert!(store.level_info().is_none());
    assert!(store.available_levels().is_empty());
    assert_eq!(store.server_enforces_secure_chat(), None);
    assert_eq!(store.chunk_count(), 0);
    assert_eq!(store.first_chunk(), None);
    assert_eq!(store.entity_count(), 0);
    assert!(store.block_destructions.is_empty());
    assert!(store.block_events.is_empty());
    assert!(store.level_events.is_empty());
    assert!(store.playing_jukebox_songs().is_empty());
    assert!(store.world_time().is_none());
    assert_eq!(store.weather(), WorldWeatherState::default());
    assert_eq!(store.ticking(), WorldTickingState::default());
    assert_eq!(store.gameplay(), &WorldGameplayState::default());
    assert_eq!(store.local_player_id(), None);
    assert_eq!(store.local_player_vehicle_id(), None);
    assert_eq!(store.local_player(), &crate::LocalPlayerState::default());
    assert_eq!(store.local_player_old_pose, None);

    let counters = store.counters();
    assert_eq!(counters.play_logins_received, 1);
    assert_eq!(counters.entities_tracked, 0);
    assert_eq!(counters.block_destructions_tracked, 0);
    assert_eq!(counters.block_events_tracked, 0);
    assert_eq!(counters.level_events_tracked, 0);
}

#[test]
fn clearing_client_level_closes_game_event_screen() {
    let mut store = WorldStore::new();
    store.apply_game_event(ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
        param: 0.0,
    });
    store.take_screen_requests();

    store.clear_client_level();

    assert_eq!(
        store.screen_requests(),
        [crate::ScreenRequest::CloseLevelScreens]
    );
}
