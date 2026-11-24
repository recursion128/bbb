use super::*;
use crate::world_creation::default_overworld_level_source;
use bbb_server_types::chunk_data::ChunkPreCarversStatus;

#[test]
fn biome_source_flat_and_debug_queries_do_not_load_columns() {
    let flat = ChunkStore::new(default_overworld_level_source().unwrap(), 17);
    let debug = ChunkStore::new_debug_with_biome_registry_and_skylight(
        DebugLevelSource::new(-64, 384).unwrap(),
        19,
        64,
        true,
    );
    for (store, expected) in [(flat, 17), (debug, 19)] {
        for y in [i32::MIN, 0, i32::MAX] {
            assert_eq!(store.biome_id_at_quart_or_source(-5, y, 37), Some(expected));
            assert_eq!(store.biome_id_at_quart(-5, y, 37), None);
        }
    }
}

#[test]
fn biome_source_loaded_override_clamps_but_pre_biomes_placeholder_is_ignored() {
    let mut store = ChunkStore::new(default_overworld_level_source().unwrap(), 17);
    let mut column = store.generated_column();
    let min_y = column.min_y().div_euclid(4);
    let max_y = min_y + column.section_count() as i32 * 4 - 1;
    column.set_noise_biome(3, min_y, 3, 21).unwrap();
    column.set_noise_biome(3, max_y, 3, 22).unwrap();
    store.insert_column(-1, -1, column);
    for status in [
        ChunkPreCarversStatus::Empty,
        ChunkPreCarversStatus::StructureStarts,
        ChunkPreCarversStatus::StructureReferences,
        ChunkPreCarversStatus::Biomes,
        ChunkPreCarversStatus::Noise,
        ChunkPreCarversStatus::Surface,
    ] {
        store.loaded_mut((-1, -1)).pre_carvers = Some(ChunkPreCarversState::new(status, vec![]));

        let ready = status.is_at_least(&ChunkPreCarversStatus::Biomes);
        assert_eq!(
            store.biome_id_at_quart_or_source(-1, i32::MIN, -1),
            Some(if ready { 21 } else { 17 })
        );
        assert_eq!(
            store.biome_id_at_quart_or_source(-1, i32::MAX, -1),
            Some(if ready { 22 } else { 17 })
        );
    }
    store.loaded_mut((-1, -1)).pre_carvers = None;
    assert_eq!(
        store.biome_id_at_quart_or_source(-1, i32::MIN, -1),
        Some(21)
    );
}

#[test]
fn biome_source_noise_uses_unclamped_climate_without_terrain_publication() {
    use bbb_protocol::NbtSummaryValue as N;
    use noise_generator::test_support::{compound, fresh_overworld_startup, overworld_noise_stem};
    let mut startup = fresh_overworld_startup(true);
    let settings = startup
        .registries
        .registries
        .iter_mut()
        .find(|r| r.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|e| e.id == "minecraft:overworld")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&settings.source_json).unwrap();
    json["noise_router"]["depth"] = serde_json::json!({"type":"minecraft:y_clamped_gradient",
        "from_y":-1024,"to_y":1024,"from_value":-1.0,"to_value":1.0});
    settings.source_json = serde_json::to_vec(&json).unwrap().into();
    settings.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
            .unwrap(),
    );
    let settings = settings.clone();
    noise_generator::test_support::publish_worldgen_element(
        &mut startup.registries,
        "minecraft:worldgen/noise_settings",
        settings,
    );
    let mut stem = overworld_noise_stem();
    let N::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    generator
        .iter_mut()
        .find(|e| e.name == "biome_source")
        .unwrap()
        .value = compound(vec![
        ("type", N::String("minecraft:multi_noise".into())),
        (
            "biomes",
            N::List(
                [(-1.0, "minecraft:plains"), (0.0, "minecraft:desert")]
                    .into_iter()
                    .map(|(depth, biome)| {
                        compound(vec![
                            ("biome", N::String(biome.into())),
                            (
                                "parameters",
                                compound(vec![
                                    ("temperature", N::Float(0.0_f32.to_bits())),
                                    ("humidity", N::Float(0.0_f32.to_bits())),
                                    ("continentalness", N::Float(0.0_f32.to_bits())),
                                    ("erosion", N::Float(0.0_f32.to_bits())),
                                    ("depth", N::Float((depth as f32).to_bits())),
                                    ("weirdness", N::Float(0.0_f32.to_bits())),
                                    ("offset", N::Float(0.0_f32.to_bits())),
                                ]),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    let program = Arc::new(
        noise_generator::compile_noise_final_density(&stem, Arc::new(startup), 0, "test.biome")
            .unwrap(),
    );
    let store = ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), false)
        .unwrap();

    let max_y = store.min_y() / 4 + store.section_count() as i32 * 4 - 1;
    let mut distinguishes_clamp = false;
    for x in [-1, 0, 100] {
        for y in [-1024, -128, -8, 0, 8, 256, 1024] {
            let actual = store.biome_id_at_quart_or_source(x, y, -x);
            assert_eq!(actual, program.uncached_noise_biome(x, y, -x));
            assert!(actual.is_some());
            distinguishes_clamp |=
                actual != program.uncached_noise_biome(x, y.clamp(store.min_y() / 4, max_y), -x);
            assert_eq!(store.biome_id_at_quart(x, y, -x), None);
        }
    }
    assert!(
        distinguishes_clamp,
        "the climate fixture must detect accidental Y clamping"
    );
}
