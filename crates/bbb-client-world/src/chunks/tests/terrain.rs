use super::{
    synthetic_local_palette_chunk_packet, synthetic_sparse_vertical_chunk_packet,
    terrain_cell_index, test_block_state_id,
};
use crate::{BlockPos, ChunkPos, WorldDimension, WorldStore};

#[test]
fn extracts_terrain_chunk_summary() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    let summary = terrain.summary();
    assert_eq!(summary.total_blocks, 4096);
    assert_eq!(summary.opaque_blocks, 4096);
    assert_eq!(summary.empty_blocks, 0);
    assert_eq!(summary.cutout_blocks, 0);
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].biome_id,
        Some(4)
    );
}

#[test]
fn extracted_terrain_carries_exact_render_occlusion_facts() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    for (
        x,
        resource_id,
        properties,
        expected_solid_render,
        expected_can_occlude,
        expected_view_blocking,
        expected_light_dampening,
        expected_shade_brightness,
        expected_face_occlusion_shapes,
        expected_fluid_overlay_neighbor,
    ) in [
        (
            34,
            "minecraft:stone",
            &[][..],
            true,
            true,
            true,
            15,
            51,
            [1; 6],
            false,
        ),
        (
            35,
            "minecraft:glass",
            &[][..],
            false,
            false,
            false,
            0,
            255,
            [0; 6],
            true,
        ),
        (
            36,
            "minecraft:oak_leaves",
            &[
                ("distance", "1"),
                ("persistent", "true"),
                ("waterlogged", "true"),
            ][..],
            false,
            false,
            false,
            1,
            51,
            [0; 6],
            true,
        ),
    ] {
        assert!(
            store.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x, y: 1, z: -45 },
                block_state_id: test_block_state_id(resource_id, properties),
            })
        );
        let cell = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .cells
            .into_iter()
            .find(|cell| cell.block_name.as_deref() == Some(resource_id))
            .unwrap();
        assert_eq!(cell.solid_render, expected_solid_render);
        assert_eq!(cell.can_occlude, expected_can_occlude);
        assert_eq!(cell.view_blocking, expected_view_blocking);
        assert_eq!(cell.light_dampening, expected_light_dampening);
        assert_eq!(cell.shade_brightness, expected_shade_brightness);
        assert_eq!(
            cell.face_occlusion_shape_indices,
            expected_face_occlusion_shapes
        );
        assert_eq!(cell.fluid_overlay_neighbor, expected_fluid_overlay_neighbor);
        if resource_id == "minecraft:glass" {
            assert!(cell.skip_rendering.same_block_culls_all_faces);
        }
        if resource_id == "minecraft:oak_leaves" {
            assert!(cell.skip_rendering.leaves_block);
        }
        assert_eq!(
            store
                .probe_block(BlockPos { x, y: 1, z: -45 })
                .unwrap()
                .block_name
                .as_deref(),
            Some(resource_id)
        );
    }
}

#[test]
fn terrain_skip_rendering_uses_current_session_bars_tag_membership() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let pos = bbb_protocol::packets::BlockPos {
        x: 34,
        y: 1,
        z: -45,
    };
    assert!(
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos,
            block_state_id: test_block_state_id(
                "minecraft:copper_bars",
                &[
                    ("east", "true"),
                    ("north", "false"),
                    ("south", "false"),
                    ("waterlogged", "false"),
                    ("west", "true"),
                ],
            ),
        })
    );

    let extract = |store: &WorldStore| {
        store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .cells
            .into_iter()
            .find(|cell| cell.block_name.as_deref() == Some("minecraft:copper_bars"))
            .unwrap()
            .skip_rendering
    };
    let before = extract(&store);
    assert!(before.iron_bars_block);
    assert!(!before.bars_tag);
    assert!(before.east);
    assert!(before.west);

    store.apply_update_tags(bbb_protocol::packets::UpdateTags {
        registries: vec![bbb_protocol::packets::RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![bbb_protocol::packets::TagNetworkPayload {
                tag: "minecraft:bars".to_string(),
                entries: vec![bbb_protocol::block_types::VANILLA_BLOCK_COPPER_BARS_ID],
            }],
        }],
    });

    assert!(extract(&store).bars_tag);
}

#[test]
fn render_extraction_crops_empty_vertical_sections_but_keeps_neighbor_sampling_bands() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: -32,
        height: 80,
    });
    store
        .insert_level_chunk_with_light(synthetic_sparse_vertical_chunk_packet(Some(2)))
        .unwrap();

    let pos = ChunkPos { x: 2, z: -3 };
    let (min_y, height, source_min_section_y, source) =
        store.extract_terrain_chunk_source(pos).unwrap();
    assert_eq!(min_y, -16);
    assert_eq!(height, 48);

    let full = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    let full_offset = usize::try_from(min_y - full.min_y).unwrap() * 16 * 16;
    let context = store.terrain_block_render_context();
    let mut index_in_crop = 0;
    for section_y in (min_y / 16)..((min_y + height as i32) / 16) {
        let section_index = usize::try_from(section_y - source_min_section_y).unwrap();
        let view = source.section(section_index).unwrap();
        for y in 0..16_u8 {
            for z in 0..16_u8 {
                for x in 0..16_u8 {
                    let block_index = crate::section_block_index(x, y, z);
                    let quart_index = crate::section_biome_index(x / 4, y / 4, z / 4);
                    let full_cell = &full.cells[full_offset + index_in_crop];
                    index_in_crop += 1;
                    let state = context.block_state(view.block_state_id_at(block_index) as i32);
                    assert_eq!(view.biome_id_at(quart_index), full_cell.biome_id);
                    assert_eq!(view.light_at(block_index), full_cell.light);
                    assert_eq!(state.block_state_id, full_cell.block_state_id);
                    assert_eq!(state.block_name, full_cell.block_name.as_deref());
                    assert_eq!(state.block_properties, &full_cell.block_properties);
                    assert_eq!(state.material, full_cell.material);
                    assert_eq!(state.solid_render, full_cell.solid_render);
                    assert_eq!(state.can_occlude, full_cell.can_occlude);
                    assert_eq!(state.view_blocking, full_cell.view_blocking);
                    assert_eq!(state.light_dampening, full_cell.light_dampening);
                    assert_eq!(state.shade_brightness, full_cell.shade_brightness);
                    assert_eq!(state.skip_rendering, full_cell.skip_rendering);
                    assert_eq!(
                        state.face_occlusion_shape_indices,
                        full_cell.face_occlusion_shape_indices
                    );
                    assert_eq!(
                        state.fluid_overlay_neighbor,
                        full_cell.fluid_overlay_neighbor
                    );
                    assert_eq!(state.state_light_emission, full_cell.state_light_emission);
                    assert_eq!(state.emissive_rendering, full_cell.emissive_rendering);
                    assert_eq!(state.fluid, full_cell.fluid);
                }
            }
        }
    }
    assert_eq!(index_in_crop, 16 * 48 * 16);
}

#[test]
fn render_extraction_keeps_one_empty_section_for_an_empty_neighborhood() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: -32,
        height: 80,
    });
    store
        .insert_level_chunk_with_light(synthetic_sparse_vertical_chunk_packet(None))
        .unwrap();

    let pos = ChunkPos { x: 2, z: -3 };
    let range = store.terrain_render_section_range(pos).unwrap();
    assert_eq!(range, -2..=-2);
    let (_, _, source_min_section_y, source) = store.extract_terrain_chunk_source(pos).unwrap();
    let section_index = usize::try_from(-2 - source_min_section_y).unwrap();
    let view = source.section(section_index).unwrap();
    let context = store.terrain_block_render_context();
    for block_index in 0..4096 {
        assert!(matches!(
            context
                .block_state(view.block_state_id_at(block_index) as i32)
                .material,
            crate::TerrainMaterialClass::Empty
        ));
    }
}
