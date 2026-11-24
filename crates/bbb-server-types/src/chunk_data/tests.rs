use super::*;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::codec::Decoder;
use std::collections::BTreeMap;

#[test]
fn encoded_section_fluid_count_includes_liquids_and_waterlogged_blocks() {
    let state = |name: &str, properties: &[(&str, &str)]| {
        vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    };
    let air = state("minecraft:air", &[]);
    let mut blocks = SectionBlocks::Single(air);
    let cells = [
        state("minecraft:water", &[("level", "0")]),
        state("minecraft:water", &[("level", "15")]),
        state("minecraft:lava", &[("level", "0")]),
        state(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "true")],
        ),
        state("minecraft:stone", &[]),
        state(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "false")],
        ),
    ];
    for (x, block) in cells.into_iter().enumerate() {
        blocks.set(x, 0, 0, block);
    }
    let mut column = ChunkColumn::new(0, 1, air, 0);
    column.replace_section_blocks(0, blocks);
    let bytes = column.encode_section_data();
    let mut decoder = Decoder::new(&bytes);
    assert_eq!(decoder.read_i16().unwrap(), 6);
    assert_eq!(decoder.read_i16().unwrap(), 4);
}

#[test]
fn read_cell_matches_packed_values_across_all_supported_bit_widths() {
    for bits in [4_u8, 5, 6, 7, 8, 9, 10, 11, 12, 15] {
        let per_word = 64 / bits as usize;
        let words = SECTION_VOLUME.div_ceil(per_word);
        let max_val = (1_usize << bits) - 1;
        let mut data = vec![0_u64; words];
        let mut expected = Vec::with_capacity(SECTION_VOLUME);
        for index in 0..SECTION_VOLUME {
            let value = (index * 37 + usize::from(bits)) & max_val;
            expected.push(value);
            let word = index / per_word;
            let shift = (index % per_word) * bits as usize;
            data[word] |= (value as u64) << shift;
        }
        for (index, &want) in expected.iter().enumerate() {
            assert_eq!(
                read_cell(&data, bits, index),
                want,
                "bits={bits} index={index}"
            );
        }
    }
}

#[test]
fn single_pass_section_counters_and_has_only_air_match_incremental_updates() {
    let state = |name: &str, properties: &[(&str, &str)]| {
        vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    };
    let air = state("minecraft:air", &[]);
    let cave_air = state("minecraft:cave_air", &[]);
    let stone = state("minecraft:stone", &[]);
    let grass = state("minecraft:grass_block", &[("snowy", "false")]);
    let lava = state("minecraft:lava", &[("level", "0")]);
    let water = state("minecraft:water", &[("level", "0")]);

    let mut column = ChunkColumn::new(0, 1, air, 0);
    assert!(column.section_has_only_air(0));
    assert!(column.section(0).has_only_air());
    assert!(!column.section(0).any_non_air());

    // Place a mix of ticking/fluid/solid blocks and compare with single-pass constructor.
    let mut blocks = SectionBlocks::Single(air);
    blocks.set(0, 0, 0, stone);
    blocks.set(1, 0, 0, grass);
    blocks.set(2, 0, 0, lava);
    blocks.set(3, 0, 0, water);
    blocks.set(4, 0, 0, cave_air);
    let biomes = column.section_biomes(0).clone();
    let rebuilt = ChunkSection::with_biomes_and_air(blocks.clone(), biomes, air);

    // Compare single-pass construction against incremental `set_block_state` updates.
    let mut incremental = ChunkColumn::new(0, 1, air, 0);
    incremental.set_block_state(0, 0, 0, stone);
    incremental.set_block_state(1, 0, 0, grass);
    incremental.set_block_state(2, 0, 0, lava);
    incremental.set_block_state(3, 0, 0, water);
    incremental.set_block_state(4, 0, 0, cave_air);
    let expected = incremental.chunk_section(0);

    assert_eq!(rebuilt.non_air_count(), 4);
    assert_eq!(rebuilt.non_air_count(), expected.non_air_count());
    assert_eq!(
        rebuilt.ticking_block_count(),
        expected.ticking_block_count()
    );
    assert_eq!(
        rebuilt.ticking_fluid_count(),
        expected.ticking_fluid_count()
    );
    assert_eq!(rebuilt.ticking_fluid_count(), 1);
    assert!(!rebuilt.has_only_air(air));
    assert!(blocks.any_non_air());

    // Overwrite all non-air blocks back to air/cave_air (leaving stale non-air palette entries).
    column.replace_section_blocks(0, blocks);
    assert!(!column.section_has_only_air(0));
    column.set_block_state(0, 0, 0, air);
    column.set_block_state(1, 0, 0, air);
    column.set_block_state(2, 0, 0, air);
    column.set_block_state(3, 0, 0, air);
    // Cave air is excluded from `non_air_count` (Vanilla `BlockState.isAir()`, LevelChunkSection.java:72-100).
    assert_eq!(column.chunk_section(0).non_air_count(), 0);
    assert!(column.section_has_only_air(0));
    assert!(column.section(0).has_only_air());

    // Transitioning cave_air -> void_air -> air keeps `non_air_count == 0`.
    let void_air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:void_air")
            .unwrap()
            .id,
    )
    .unwrap();
    column.set_block_state(4, 0, 0, void_air);
    assert_eq!(column.chunk_section(0).non_air_count(), 0);
    assert!(column.section_has_only_air(0));

    column.set_block_state(4, 0, 0, air);
    assert_eq!(column.chunk_section(0).non_air_count(), 0);
    assert!(column.section_has_only_air(0));
}

#[test]
fn global_palette_section_and_block_tick_type_support_custom_content_registry() {
    use crate::chunk_ticks::{BlockTickType, TickType};
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

    // Push total block-state count past 2^15 (32_768) so global palette needs 16 bits.
    let mut builder = ContentRegistryBuilder::new();
    builder.register_block(ModBlockRegistration {
        resource_id: "example_mod:many_states".to_string(),
        state_count: 4_000,
        light_emission: 0,
        destroy_speed: 1.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let registry = builder.freeze().unwrap();
    let total_states = registry.total_block_state_count();
    assert!(total_states > (1 << 15));
    let mod_block = registry
        .block_by_resource_id("example_mod:many_states")
        .unwrap();
    let high_mod_state = mod_block.first_state_id + i32::from(mod_block.state_count) - 1;
    assert!(high_mod_state >= (1 << 15));

    // Put >256 distinct states (including the >32767 mod state) into one section to trigger global wire form.
    let mut column = ChunkColumn::new(0, 1, 0, 0);
    for i in 0..260 {
        let state = if i == 259 { high_mod_state } else { i };
        column.set_block_state(i % 16, i / 256, (i / 16) % 16, state);
    }
    let bytes = column.encode_section_data_with_block_state_count(total_states);
    let decoded = ChunkColumn::decode_section_data_with_registries(
        0,
        &bytes,
        VANILLA_26_1_BIOME_REGISTRY_LEN,
        total_states,
    )
    .unwrap();
    assert_eq!(
        decoded.get_block_state(259 % 16, 259 / 256, (259 / 16) % 16),
        Some(high_mod_state)
    );

    let tick_from_state =
        BlockTickType::of_block_state_with_registry(high_mod_state, &registry).unwrap();
    let tick_from_id =
        BlockTickType::of_block_type_with_registry(mod_block.block_id, &registry).unwrap();
    let tick_from_name =
        BlockTickType::from_resource_id_with_registry("example_mod:many_states", &registry)
            .unwrap();
    assert_eq!(tick_from_state, tick_from_id);
    assert_eq!(tick_from_id, tick_from_name);
    assert_eq!(tick_from_state.resource_id(), "example_mod:many_states");
}
