use super::*;

fn fill_sky(storage: &mut ChunkLightStorage, section: LightSectionPos, value: u8) {
    storage.ensure_empty_section_for_restore(section);
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                storage.set_sky_level(
                    (section.0 * 16 + x, section.1 * 16 + y, section.2 * 16 + z),
                    value,
                );
            }
        }
    }
}

#[test]
fn an_enabled_column_allocates_a_new_sky_shell_at_fifteen() {
    let mut storage = ChunkLightStorage::default();
    storage.enable_column((0, 0));

    storage.register_non_empty_section((0, 0, 0));

    assert_eq!(storage.stored_sky_level((0, 0, 0)), 15);
    assert_eq!(storage.stored_sky_level((15, 31, 15)), 15);
    assert_eq!(
        storage
            .saved_sections((0, 0))
            .into_iter()
            .find(|section| section.y == 1)
            .unwrap()
            .sky_light,
        Some(vec![-1; LIGHT_ARRAY_BYTES])
    );
}

#[test]
fn a_new_lower_sky_shell_repeats_the_nearest_upper_bottom_plane() {
    let mut storage = ChunkLightStorage::default();
    storage.ensure_empty_section_for_restore((0, 2, 0));
    for z in 0..16 {
        for x in 0..16 {
            storage.set_sky_level((x, 32, z), ((x + z) % 16) as u8);
            storage.set_sky_level((x, 33, z), 15);
        }
    }

    storage.ensure_empty_section_for_restore((0, 0, 0));

    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                assert_eq!(storage.stored_sky_level((x, y, z)), ((x + z) % 16) as u8);
            }
        }
    }
}

#[test]
fn queued_saved_layers_are_visible_to_packet_projection() {
    let mut storage = ChunkLightStorage::default();
    storage.queue_saved_sections(
        (0, 0),
        &[SavedLightSection {
            y: 7,
            block_light: Some(vec![0; LIGHT_ARRAY_BYTES]),
            sky_light: Some(vec![0x21; LIGHT_ARRAY_BYTES]),
        }],
    );

    assert!(!storage.storing((0, 7, 0)));
    let packet = storage.packet_data((0, 0), 7, 1).unwrap();
    assert_eq!(packet.sky_y_mask, vec![1]);
    assert_eq!(packet.block_y_mask, vec![1]);
    assert!(packet.empty_sky_y_mask.is_empty());
    assert!(packet.empty_block_y_mask.is_empty());
    assert_eq!(packet.sky_updates, vec![vec![0x21; LIGHT_ARRAY_BYTES]]);
    assert_eq!(packet.block_updates, vec![vec![0; LIGHT_ARRAY_BYTES]]);
}

#[test]
fn implicit_sky_uses_nearest_same_column_bottom_plane_below_global_bottom() {
    let mut storage = ChunkLightStorage::default();
    let chunk = (-1, -2);
    storage.enable_column(chunk);
    for (section, value) in [
        ((-1, 2, -2), 12),
        ((-1, -1, -2), 3),
        ((-1, -2, -1), 14),
        ((0, -2, -2), 15),
    ] {
        fill_sky(&mut storage, section, value);
    }
    storage.set_sky_level((-15, -1, -30), 9);
    assert_eq!(storage.bottom_section_y(), Some(-2));

    assert_eq!(storage.sky_level((-15, -1, -30)), 9);
    assert_eq!(storage.sky_level((-15, -16, -30)), 3);
    // Missing layers read local Y zero from the next layer (`SkyLightSectionStorage.java:33-51`).
    assert_eq!(storage.sky_level((-15, -17, -30)), 3);
    assert_eq!(storage.sky_level((-15, -33, -30)), 3);
    assert_eq!(storage.sky_level((-15, -1601, -30)), 3);
    assert_eq!(storage.sky_level((-15, 15, -30)), 12);
}

#[test]
fn implicit_sky_preserves_empty_and_disabled_column_fallbacks() {
    let mut storage = ChunkLightStorage::default();
    assert_eq!(storage.sky_level((-1, -1, -1)), 0);
    storage.enable_column((-1, -1));
    assert_eq!(storage.sky_level((-1, -1, -1)), 15);
    assert_eq!(storage.sky_level((16, -1, -1)), 0);

    fill_sky(&mut storage, (0, 0, 0), 7);
    assert_eq!(storage.sky_level((0, 0, 0)), 7);
    assert_eq!(storage.sky_level((0, 16, 0)), 0);
    assert_eq!(storage.sky_level((16, -1, -1)), 0);
    assert_eq!(storage.sky_level((-1, -1, -1)), 15);
}

#[test]
fn emptied_sections_drop_their_shell_and_unload_keeps_neighbor_layers() {
    let mut storage = ChunkLightStorage::default();
    storage.register_non_empty_section((0, 0, 0));
    storage.register_non_empty_section((1, 3, 0));
    storage.unregister_non_empty_section((0, 0, 0));
    assert_eq!(storage.bottom_section_y(), Some(2));

    storage.unload_column((1, 0));
    assert!(!storage.storing((0, 3, 0)));
    assert_eq!(storage.top_section_y((1, 0)), None);
}

#[test]
fn data_layer_uses_vanilla_low_nibble_first_and_preserves_materialized_zero() {
    let mut layer = LightDataLayer::Uniform(0);
    assert!(layer.is_empty());
    layer.set(0, 0, 0, 3);
    layer.set(1, 0, 0, 12);
    layer.set(0, 0, 1, 7);
    let bytes = layer.wire_bytes();
    assert_eq!(bytes[0], 0xc3);
    assert_eq!(bytes[8], 0x07);
    layer.set(0, 0, 0, 0);
    layer.set(1, 0, 0, 0);
    layer.set(0, 0, 1, 0);
    assert!(!layer.is_empty());
    assert_eq!(layer.wire_bytes(), Box::new([0; LIGHT_ARRAY_BYTES]));

    let full = LightDataLayer::Uniform(15);
    assert_eq!(full.wire_bytes(), Box::new([0xff; LIGHT_ARRAY_BYTES]));
}

#[test]
fn enable_restored_all_air_column_fills_sky_fifteen_and_unload_prunes_neighbors() {
    let mut storage = ChunkLightStorage::default();
    storage.ensure_empty_section_for_restore((0, 0, 0));
    storage.ensure_empty_section_for_restore((0, 1, 0));
    storage.enable_restored_column((0, 0), &[i32::MIN; 256]);
    assert_eq!(storage.stored_sky_level((0, 0, 0)), 15);
    assert_eq!(storage.stored_sky_level((15, 31, 15)), 15);

    let mut unload_storage = ChunkLightStorage::default();
    unload_storage.register_non_empty_section((1, 3, 0));
    unload_storage.unload_column((1, 0));
    assert_eq!(unload_storage.bottom_section_y(), None);
    assert_eq!(unload_storage.top_section_y((0, 0)), None);
}

#[test]
fn unload_column_preserves_neighbor_border_shell_and_reloads_cleanly() {
    let mut storage = ChunkLightStorage::default();
    storage.enable_column((0, 0));
    storage.enable_column((1, 0));
    storage.register_non_empty_section((0, 0, 0));
    storage.register_non_empty_section((0, 1, 0));
    storage.register_non_empty_section((1, 0, 0));
    storage.set_block_level((15, 4, 8), 11);

    storage.unload_column((0, 0));

    assert!(!storage.is_enabled((0, 0)));
    // Sections adjacent to loaded (1, 0, 0) stay alive as its 26-neighbor border shell.
    assert!(storage.storing((0, -1, 0)));
    assert!(storage.storing((0, 0, 0)));
    assert!(storage.storing((0, 1, 0)));
    assert!(!storage.storing((0, 2, 0)));
    assert_eq!(storage.stored_block_level((15, 4, 8)), 11);
    assert_eq!(storage.section((0, 0, 0)).unwrap().neighbor_count, 1);
    assert_eq!(storage.section((0, 1, 0)).unwrap().neighbor_count, 1);
    assert_eq!(storage.section((1, 0, 0)).unwrap().neighbor_count, 0);
    // Remote border column (-1, 0) had only (0, 0) as neighbor and is pruned.
    assert_eq!(storage.top_section_y((-1, 0)), None);

    // Reloading (0, 0) restores exact neighbor counts without drifting or panicking.
    storage.enable_column((0, 0));
    storage.register_non_empty_section((0, 0, 0));
    storage.register_non_empty_section((0, 1, 0));
    assert_eq!(storage.section((0, 0, 0)).unwrap().neighbor_count, 2);
    assert_eq!(storage.section((0, 1, 0)).unwrap().neighbor_count, 2);
    assert_eq!(storage.section((1, 0, 0)).unwrap().neighbor_count, 2);

    // Unloading both columns empties storage completely.
    storage.unload_column((0, 0));
    storage.unload_column((1, 0));
    assert_eq!(storage.bottom_section_y(), None);
    assert_eq!(storage.top_section_y((0, 0)), None);
    assert_eq!(storage.top_section_y((1, 0)), None);
}
