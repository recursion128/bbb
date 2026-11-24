use super::*;

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn persistent() -> ServerMapPersistentData {
    ServerMapPersistentData {
        dimension: "minecraft:overworld".to_owned(),
        center_x: 0,
        center_z: 0,
        scale: 0,
        colors: vec![0; MAP_PIXELS],
        tracking_position: true,
        unlimited_tracking: false,
        locked: false,
        banners: Vec::new(),
        frames: Vec::new(),
    }
}

fn carrier(id: u64, name: &str) -> ServerMapCarrier {
    ServerMapCarrier {
        connection_id: ConnectionId(id),
        plain_name: name.to_owned(),
        dimension: "minecraft:overworld".to_owned(),
        x: 0.0,
        z: 0.0,
        y_rot: 0.0,
        removed: false,
        holds_matching_map: true,
        wears_map_invisibility_equipment: false,
    }
}

fn static_decoration(id: impl Into<JavaString>, type_id: i32, x: f64) -> ItemMapDecorationSummary {
    ItemMapDecorationSummary {
        id: id.into(),
        decoration_type_id: type_id,
        decoration_type_resource_id: String::new(),
        x_bits: x.to_bits(),
        z_bits: 0.0_f64.to_bits(),
        rotation_bits: 0.0_f32.to_bits(),
    }
}

fn loaded_store(map_id: i32, data: ServerMapPersistentData) -> ServerMapStore {
    ServerMapStore::from_persistent_parts([(map_id, data)], -1).unwrap()
}

#[test]
fn server_maps_rebuild_persisted_banners_then_frames_with_vanilla_collision_rules() {
    let mut data = persistent();
    data.banners = vec![
        ServerMapBanner {
            pos: pos(1, 2, 3),
            color: "white".to_owned(),
            name: Some(RichTextComponent::plain("first")),
        },
        ServerMapBanner {
            pos: pos(1, 2, 3),
            color: "black".to_owned(),
            name: Some(RichTextComponent::plain("last")),
        },
        ServerMapBanner {
            pos: pos(100, 2, 100),
            color: "red".to_owned(),
            name: None,
        },
    ];
    data.frames = vec![
        ServerMapFrame {
            pos: pos(4, 5, 6),
            rotation: 0,
            entity_id: 40,
        },
        ServerMapFrame {
            pos: pos(4, 5, 6),
            rotation: 90,
            entity_id: 41,
        },
    ];

    let store = loaded_store(7, data);
    let map = &store.maps[&7];
    assert_eq!(map.banners.len(), 2);
    assert_eq!(map.banners[&(1, 2, 3)].color, "black");
    assert_eq!(map.frames.len(), 1);
    assert_eq!(map.frames[&(4, 5, 6)].entity_id, 41);
    assert_eq!(map.decoration("banner-1,2,3").unwrap().type_id, 25);
    assert!(map.decoration("banner-100,2,100").is_none());
    assert!(map.decoration("frame-40").is_some());
    assert!(map.decoration("frame-41").is_some());
    assert_eq!(map.tracked_decoration_count, 3);
    assert!(store.dirty_persistent_snapshots().is_empty());
}

#[test]
fn server_maps_track_mixed_decoration_replacement_and_removal() {
    let mut map = ServerMapSavedData::from_persistent(persistent()).unwrap();
    map.add_decoration(4, None, "marker", [0.0, 0.0, 0.0], None)
        .unwrap();
    assert_eq!(map.tracked_decoration_count, 0);
    map.add_decoration(2, None, "marker", [0.0, 0.0, 0.0], None)
        .unwrap();
    assert_eq!(map.tracked_decoration_count, 1);
    map.add_decoration(5, None, "marker", [0.0, 0.0, 0.0], None)
        .unwrap();
    assert_eq!(map.tracked_decoration_count, 0);
    map.remove_decoration("marker");
    assert_eq!(map.tracked_decoration_count, 0);
}

#[test]
fn server_maps_admission_allows_256_and_rejects_only_when_caller_sees_257() {
    let mut data = persistent();
    data.tracking_position = false;
    let mut store = loaded_store(1, data);
    let decorations: Vec<_> = (0..256)
        .map(|index| static_decoration(format!("marker-{index}"), 2, 0.0))
        .collect();
    store
        .tick_carried_by(1, carrier(1, "one"), &[], None, &decorations, 0)
        .unwrap();
    assert_eq!(
        store.admission(1),
        FramedMapAdmission::Present {
            tracked_decoration_count: 256
        }
    );
    assert!(!store
        .dirty_persistent_snapshots()
        .iter()
        .any(|s| s.map_id == 1));

    store
        .maps
        .get_mut(&1)
        .unwrap()
        .add_decoration(2, None, "marker-256", [0.0, 0.0, 0.0], None)
        .unwrap();
    assert_eq!(
        store.admission(1),
        FramedMapAdmission::Present {
            tracked_decoration_count: 257
        }
    );
    assert_eq!(store.admission(404), FramedMapAdmission::SavedDataMissing);

    store.insert_new(2, persistent()).unwrap();
    assert_eq!(store.dirty_persistent_snapshots()[0].map_id, 2);
}

#[test]
fn server_maps_frame_attach_is_tick_owned_and_position_collision_removes_old_entity() {
    let mut store = loaded_store(3, persistent());
    assert_eq!(
        store.admission(3),
        FramedMapAdmission::Present {
            tracked_decoration_count: 0
        }
    );
    assert!(store.dirty_persistent_snapshots().is_empty());

    let first = ServerMapPlacedFrame {
        entity_id: 10,
        pos: pos(1, 2, 3),
        direction: ServerMapFrameDirection::South,
    };
    store
        .tick_carried_by(3, carrier(1, "one"), &[], Some(first), &[], 0)
        .unwrap();
    let replacement = ServerMapPlacedFrame {
        entity_id: 11,
        ..first
    };
    store
        .tick_carried_by(3, carrier(1, "one"), &[], Some(replacement), &[], 0)
        .unwrap();

    let map = &store.maps[&3];
    assert!(map.decoration("frame-10").is_none());
    assert_eq!(map.decoration("frame-11").unwrap().rot & 15, 0);
    assert_eq!(map.frames[&(1, 2, 3)].entity_id, 11);
    assert_eq!(map.tracked_decoration_count, 1);
}

#[test]
fn server_maps_frame_direction_rotation_includes_vertical_minus_one() {
    let mut store = loaded_store(4, persistent());
    let directions = [
        (ServerMapFrameDirection::Down, 12),
        (ServerMapFrameDirection::Up, 12),
        (ServerMapFrameDirection::North, 8),
        (ServerMapFrameDirection::South, 0),
        (ServerMapFrameDirection::West, 4),
        (ServerMapFrameDirection::East, 12),
    ];
    for (index, (direction, expected)) in directions.into_iter().enumerate() {
        store
            .tick_carried_by(
                4,
                carrier(1, "one"),
                &[],
                Some(ServerMapPlacedFrame {
                    entity_id: 100 + index as i32,
                    pos: pos(index as i32, 0, 0),
                    direction,
                }),
                &[],
                0,
            )
            .unwrap();
        assert_eq!(
            store.maps[&4]
                .decoration(&format!("frame-{}", 100 + index))
                .unwrap()
                .rot
                & 15,
            expected
        );
    }
}

#[test]
fn server_maps_unlimited_tracking_and_nether_rotation_match_java_arithmetic() {
    let mut limited = persistent();
    limited.tracking_position = false;
    let mut limited_store = loaded_store(1, limited);
    limited_store
        .tick_carried_by(
            1,
            carrier(1, "one"),
            &[],
            None,
            &[static_decoration("far", 2, 400.0)],
            0,
        )
        .unwrap();
    assert!(limited_store.maps[&1].decoration("far").is_none());

    let mut unlimited = persistent();
    unlimited.dimension = NETHER_DIMENSION.to_owned();
    unlimited.unlimited_tracking = true;
    let mut store = loaded_store(2, unlimited);
    store
        .tick_carried_by(
            2,
            carrier(1, "one"),
            &[],
            None,
            &[static_decoration("far", 2, 400.0)],
            i64::from(i32::MAX) * 10,
        )
        .unwrap();
    let s = i32::MAX;
    let expected = ((s
        .wrapping_mul(s)
        .wrapping_mul(34_187_121)
        .wrapping_add(s.wrapping_mul(121))
        >> 15)
        & 15) as u8;
    let far = store.maps[&2].decoration("far").unwrap();
    assert_eq!(far.x, 127);
    assert_eq!(far.rot, expected);

    let mut player = carrier(2, "far-player");
    player.x = 400.0;
    player.dimension = NETHER_DIMENSION.to_owned();
    store.tick_carried_by(2, player, &[], None, &[], 0).unwrap();
    assert_eq!(
        store.maps[&2].decoration("far-player").unwrap().type_id,
        PLAYER_OFF_LIMITS_DECORATION_TYPE
    );
}

#[test]
fn server_maps_static_decorations_only_fill_missing_keys_and_unknowns_fail_closed() {
    let mut data = persistent();
    data.tracking_position = false;
    let mut store = loaded_store(5, data);
    store
        .tick_carried_by(
            5,
            carrier(1, "one"),
            &[],
            None,
            &[static_decoration("same", 4, 0.0)],
            0,
        )
        .unwrap();
    store
        .tick_carried_by(
            5,
            carrier(1, "one"),
            &[],
            None,
            &[static_decoration("same", 2, 10.0)],
            0,
        )
        .unwrap();
    assert_eq!(store.maps[&5].decoration("same").unwrap().type_id, 4);
}

#[test]
fn server_maps_holding_players_get_independent_full_first_packets_and_empty_decorations() {
    let mut data = persistent();
    data.tracking_position = false;
    data.colors[0] = 9;
    let mut store = loaded_store(6, data);
    let one = carrier(1, "one");
    let two = carrier(2, "two");
    store
        .tick_carried_by(6, one.clone(), std::slice::from_ref(&two), None, &[], 0)
        .unwrap();
    store.tick_carried_by(6, two, &[one], None, &[], 0).unwrap();

    for connection_id in [ConnectionId(1), ConnectionId(2)] {
        let packet = store.next_update_packet(6, connection_id).unwrap();
        assert_eq!(packet.decorations, Some(Vec::new()));
        let patch = packet.color_patch.unwrap();
        assert_eq!((patch.width, patch.height), (128, 128));
        assert_eq!(patch.colors[0], 9);
        assert!(store.next_update_packet(6, connection_id).is_none());
    }
    assert!(store.next_update_packet(6, ConnectionId(99)).is_none());
}

#[test]
fn server_maps_decoration_updates_wait_for_tick_modulo_five_and_keep_none_distinct() {
    let mut data = persistent();
    data.tracking_position = false;
    let mut store = loaded_store(7, data);
    store
        .tick_carried_by(7, carrier(1, "one"), &[], None, &[], 0)
        .unwrap();
    let first = store.next_update_packet(7, ConnectionId(1)).unwrap();
    assert_eq!(first.decorations, Some(Vec::new()));
    assert!(store.next_update_packet(7, ConnectionId(1)).is_none());

    store.maps.get_mut(&7).unwrap().remove_decoration("absent");
    for _ in 0..4 {
        assert!(store.next_update_packet(7, ConnectionId(1)).is_none());
    }
    let fifth = store.next_update_packet(7, ConnectionId(1)).unwrap();
    assert_eq!(fifth.decorations, Some(Vec::new()));
    assert_eq!(fifth.color_patch, None);
    assert!(store.next_update_packet(7, ConnectionId(1)).is_none());
}

#[test]
fn server_maps_carrier_scan_removes_stale_and_invisible_players_and_disconnects() {
    let mut store = loaded_store(8, persistent());
    let one = carrier(1, "one");
    store
        .tick_carried_by(8, one.clone(), &[], None, &[], 0)
        .unwrap();
    let mut invisible = carrier(2, "two");
    invisible.wears_map_invisibility_equipment = true;
    store
        .tick_carried_by(
            8,
            invisible.clone(),
            std::slice::from_ref(&one),
            None,
            &[],
            0,
        )
        .unwrap();
    store
        .tick_carried_by(8, one, &[invisible], None, &[], 0)
        .unwrap();
    assert!(store.maps[&8].decoration("two").is_none());
    assert!(store.maps[&8].decoration("one").is_some());

    store.remove_player(ConnectionId(1));
    assert!(store.maps[&8].decoration("one").is_none());
    assert!(store.next_update_packet(8, ConnectionId(1)).is_none());

    let mut removed = carrier(2, "two");
    removed.removed = true;
    store
        .tick_carried_by(8, removed, &[], None, &[], 0)
        .unwrap();
    assert!(!store.maps[&8]
        .holding_players
        .contains_key(&ConnectionId(2)));
}

#[test]
fn server_maps_live_scan_removes_non_ticking_stale_and_missing_holders() {
    let mut store = loaded_store(11, persistent());
    let one = carrier(1, "one");
    let two = carrier(2, "two");
    store
        .tick_carried_by(11, one.clone(), std::slice::from_ref(&two), None, &[], 0)
        .unwrap();
    store
        .tick_carried_by(11, two.clone(), std::slice::from_ref(&one), None, &[], 0)
        .unwrap();
    assert!(store.maps[&11].decoration("two").is_some());

    let mut dropped = two.clone();
    dropped.holds_matching_map = false;
    let frame = ServerMapPlacedFrame {
        entity_id: 30,
        pos: pos(0, 1, 0),
        direction: ServerMapFrameDirection::South,
    };
    store
        .tick_carried_by(11, one.clone(), &[dropped.clone()], Some(frame), &[], 0)
        .unwrap();
    assert!(store.maps[&11]
        .holding_players
        .contains_key(&ConnectionId(2)));
    assert!(store.maps[&11].decoration("two").is_some());

    store
        .tick_carried_by(11, one.clone(), &[dropped], None, &[], 0)
        .unwrap();
    assert!(!store.maps[&11]
        .holding_players
        .contains_key(&ConnectionId(2)));
    assert!(store.maps[&11].decoration("two").is_none());

    store
        .tick_carried_by(11, two, std::slice::from_ref(&one), None, &[], 0)
        .unwrap();
    assert!(store.maps[&11]
        .holding_players
        .contains_key(&ConnectionId(2)));
    store.tick_carried_by(11, one, &[], None, &[], 0).unwrap();
    assert!(!store.maps[&11]
        .holding_players
        .contains_key(&ConnectionId(2)));
    assert!(store.maps[&11].decoration("two").is_none());
}

#[test]
fn server_maps_removed_frame_and_color_changes_use_exact_dirty_epoch_ack() {
    let mut store = loaded_store(9, persistent());
    store
        .tick_carried_by(
            9,
            carrier(1, "one"),
            &[],
            Some(ServerMapPlacedFrame {
                entity_id: 20,
                pos: pos(2, 3, 4),
                direction: ServerMapFrameDirection::East,
            }),
            &[],
            0,
        )
        .unwrap();
    let attach = store.dirty_persistent_snapshots();
    assert_eq!(attach.len(), 1);

    assert!(store.removed_from_frame(9, pos(2, 3, 4), 20));
    let removed = store.dirty_persistent_snapshots();
    assert!(removed[0].epoch > attach[0].epoch);
    assert!(removed[0].data.frames.is_empty());
    assert!(!store.acknowledge_persistent_snapshot(9, attach[0].epoch));

    assert!(store.update_color(9, 3, 4, 12).unwrap());
    let colored = store.dirty_persistent_snapshots();
    assert_eq!(colored[0].data.colors[3 + 4 * MAP_SIZE], 12);
    assert!(!store.acknowledge_persistent_snapshot(9, removed[0].epoch));
    assert!(store.acknowledge_persistent_snapshot(9, colored[0].epoch));
    assert!(store.dirty_persistent_snapshots().is_empty());

    assert!(store.removed_from_frame(9, pos(2, 3, 4), 20));
    assert_eq!(store.dirty_persistent_snapshots().len(), 1);
    assert!(!store.removed_from_frame(404, pos(0, 0, 0), 1));
}

#[test]
fn server_maps_color_updates_send_one_pixel_patch_after_first_packet() {
    let mut data = persistent();
    data.tracking_position = false;
    let mut store = loaded_store(10, data);
    store
        .tick_carried_by(10, carrier(1, "one"), &[], None, &[], 0)
        .unwrap();
    store.next_update_packet(10, ConnectionId(1)).unwrap();
    assert!(store.update_color(10, 5, 6, 31).unwrap());
    let packet = store.next_update_packet(10, ConnectionId(1)).unwrap();
    assert_eq!(packet.decorations, None);
    assert_eq!(
        packet.color_patch,
        Some(MapColorPatch {
            start_x: 5,
            start_y: 6,
            width: 1,
            height: 1,
            colors: vec![31],
        })
    );
    assert!(!store.update_color(10, 5, 6, 31).unwrap());
}

#[test]
fn server_maps_exploration_creation_uses_native_target_and_fuzzy_biome_preview() {
    use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
    let registries = crate::registry_sync::SynchronizedRegistries::load(None).unwrap();
    let resolver = super::super::registry_runtime::PlayerItemStackResolver::new(None, &registries);
    let ocean = registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:ocean")
        .unwrap();
    let plains = registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .unwrap();
    let mut chunks = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        ocean,
    );
    chunks.install_registry_tags(registries.registry_tags_shared());
    let seed = (0..100)
        .find(|&seed| {
            let zoom = biome_zoom_seed(seed);
            let sample = fuzzy_biome_quart_position(zoom, 192, 0, 192);
            sample != (48, 0, 48)
                && (-1..=1).all(|dx| {
                    (-1..=1).all(|dz| {
                        (dx == 0 && dz == 0)
                            || fuzzy_biome_quart_position(zoom, 192 + dx * 4, 0, 192 + dz * 4)
                                != sample
                    })
                })
        })
        .unwrap();
    let quart = fuzzy_biome_quart_position(biome_zoom_seed(seed), 192, 0, 192);
    let mut column = chunks.generated_column();
    column
        .set_noise_biome(quart.0 & 3, quart.1, quart.2 & 3, plains)
        .unwrap();
    chunks.insert_column(quart.0 >> 2, quart.2 >> 2, column);
    let mut store = ServerMapStore::default();
    let stack = store
        .create_exploration_map(
            &chunks,
            seed,
            ExplorationMapSpec {
                dimension: "minecraft:overworld",
                target: pos(0, 0, 0),
                zoom: 2,
                decoration: "mansion",
            },
            &resolver,
        )
        .unwrap();
    let summary = stack.summary();
    assert_eq!(
        summary.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID)
    );
    assert_eq!(summary.count, 1);
    assert_eq!(summary.component_patch.map_id, Some(0));
    assert_eq!(summary.component_patch.map_color, Some(5393476));
    let target = &summary.component_patch.map_decorations.as_ref().unwrap()[0];
    assert_eq!(target.id, JavaString::from("+"));
    assert_eq!(target.decoration_type_resource_id, "minecraft:mansion");
    assert_eq!(target.x_bits, 0.0_f64.to_bits());
    assert_eq!(target.z_bits, 0.0_f64.to_bits());
    assert_eq!(target.rotation_bits, 180.0_f32.to_bits());
    assert_eq!(chunks.column_count(), 1);
    let map = &store.maps[&0];
    assert_eq!((map.center_x, map.center_z, map.scale), (192, 192, 2));
    assert!(map.tracking_position && map.unlimited_tracking && !map.locked);
    assert_eq!(map.colors[64 * MAP_SIZE + 64], 105);
    assert_eq!(map.colors[2 * MAP_SIZE + 1], 60);
    assert_eq!(map.colors[3 * MAP_SIZE + 1], 0);
    assert_eq!(map.colors[0], 0);
    assert_eq!(map.colors[MAP_PIXELS - 1], 0);
    assert_eq!(store.dirty_index(), Some(0));
    assert_eq!(store.dirty_persistent_snapshots().len(), 1);
    store.update_color(0, 5, 5, 44).unwrap();
    store.render_biome_preview(0, &[true; MAP_PIXELS]).unwrap();
    assert_eq!(store.maps[&0].colors[5 * MAP_SIZE + 5], 44);
    let red_x = store
        .create_exploration_map(
            &chunks,
            seed,
            ExplorationMapSpec {
                dimension: "minecraft:overworld",
                target: pos(0, 0, 0),
                zoom: 2,
                decoration: "red_x",
            },
            &resolver,
        )
        .unwrap();
    assert_eq!(red_x.summary().component_patch.map_id, Some(1));
    assert_eq!(red_x.summary().component_patch.map_color, None);
}
