use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use bbb_protocol::NbtSummaryValue;
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
    Java26ChunkStatus, Java26LevelStem, RegionCompression, RegionFileStore,
};
use bbb_server_types::save::WorldSaveLimits;
use bbb_server_worldgen::noise::end_city::{END_CITY_ID, END_CITY_LOOT};
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

use super::noise_generator::test_support::{compound, fresh_vanilla_startup, vanilla_startup};
use super::noise_generator::{compile_noise_level_source, NoiseFinalDensityProgram};
use super::ChunkStore;
use bbb_server_types::chunk_data::ChunkGenerationState;

/// A chunk inside the main island, whose columns all reach solid ground.
const END_ISLAND_CHUNK: (i32, i32) = (3, 0);
static NEXT_REGION_ID: AtomicU64 = AtomicU64::new(1);

fn end_stem() -> Java26LevelStem {
    Java26LevelStem {
        dimension_type: "minecraft:the_end".to_string(),
        generator: "minecraft:noise".to_string(),
        raw_generator: compound(vec![
            ("type", NbtSummaryValue::string("minecraft:noise")),
            (
                "biome_source",
                compound(vec![("type", NbtSummaryValue::string("minecraft:the_end"))]),
            ),
            ("settings", NbtSummaryValue::string("minecraft:end")),
        ]),
    }
}

fn end_stem_with_biome_source(biome_source: NbtSummaryValue) -> Java26LevelStem {
    Java26LevelStem {
        dimension_type: "minecraft:the_end".to_string(),
        generator: "minecraft:noise".to_string(),
        raw_generator: compound(vec![
            ("type", NbtSummaryValue::string("minecraft:noise")),
            ("biome_source", biome_source),
            ("settings", NbtSummaryValue::string("minecraft:end")),
        ]),
    }
}

fn fixed_end_biome_source(biome: &str) -> NbtSummaryValue {
    compound(vec![
        ("type", NbtSummaryValue::string("minecraft:fixed")),
        ("biome", NbtSummaryValue::string(biome)),
    ])
}

fn checkerboard_end_biome_source(biomes: &[&str], scale: i32) -> NbtSummaryValue {
    compound(vec![
        ("type", NbtSummaryValue::string("minecraft:checkerboard")),
        (
            "biomes",
            NbtSummaryValue::List(
                biomes
                    .iter()
                    .map(|biome| NbtSummaryValue::string(*biome))
                    .collect(),
            ),
        ),
        ("scale", NbtSummaryValue::Int(scale)),
    ])
}

fn program() -> Arc<NoiseFinalDensityProgram> {
    static PROGRAM: OnceLock<Arc<NoiseFinalDensityProgram>> = OnceLock::new();
    Arc::clone(PROGRAM.get_or_init(|| {
        Arc::new(
            compile_noise_level_source(
                &end_stem(),
                vanilla_startup(),
                0,
                "dimensions.minecraft:the_end.generator",
            )
            .unwrap(),
        )
    }))
}

fn program_with_startup(
    startup: bbb_server_datapack::startup::DatapackStartup,
) -> Arc<NoiseFinalDensityProgram> {
    program_with_stem_startup(end_stem(), startup)
}

fn program_with_stem_startup(
    stem: Java26LevelStem,
    startup: bbb_server_datapack::startup::DatapackStartup,
) -> Arc<NoiseFinalDensityProgram> {
    Arc::new(
        compile_noise_level_source(
            &stem,
            Arc::new(startup),
            0,
            "dimensions.minecraft:the_end.generator",
        )
        .unwrap(),
    )
}

fn source_exact_registered_end_city_program(
    biome_source: NbtSummaryValue,
    structure_biomes: &[&str],
) -> Arc<NoiseFinalDensityProgram> {
    let mut startup = fresh_vanilla_startup();
    let structure = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == END_CITY_ID)
        .unwrap();
    let mut structure_json: serde_json::Value =
        serde_json::from_slice(&structure.source_json).unwrap();
    structure_json["biomes"] = serde_json::Value::Array(
        structure_biomes
            .iter()
            .map(|biome| serde_json::Value::String((*biome).to_string()))
            .collect(),
    );
    structure.id = "test:source_exact_city".to_string();
    structure.source_pack = "file/source_exact_end_city".to_string();
    structure.source_json = serde_json::to_vec(&structure_json).unwrap().into();
    structure.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(structure_json)).unwrap(),
        )
        .unwrap(),
    );
    structure.codec_value = None;

    let set = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure_set")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end_cities")
        .unwrap();
    let mut set_json: serde_json::Value = serde_json::from_slice(&set.source_json).unwrap();
    set_json["structures"][0]["structure"] = serde_json::json!("test:source_exact_city");
    set_json["placement"]["salt"] = serde_json::json!(4_242_424);
    set_json["placement"]["spacing"] = serde_json::json!(18);
    set_json["placement"]["separation"] = serde_json::json!(7);
    set_json["placement"]["spread_type"] = serde_json::json!("linear");
    set.id = "test:source_exact_cities".to_string();
    set.source_pack = "file/source_exact_end_city".to_string();
    set.source_json = serde_json::to_vec(&set_json).unwrap().into();
    set.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(set_json)).unwrap())
            .unwrap(),
    );
    set.codec_value = None;

    startup
        .structures
        .templates
        .iter_mut()
        .find(|template| template.id == "minecraft:end_city/base_floor")
        .unwrap()
        .source_pack = "file/source_exact_end_city".to_string();

    program_with_stem_startup(end_stem_with_biome_source(biome_source), startup)
}

fn custom_registered_end_city_program() -> Arc<NoiseFinalDensityProgram> {
    let mut startup = fresh_vanilla_startup();
    let structure = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end_city")
        .unwrap();
    structure.id = "test:void_city".to_string();
    structure.source_pack = "file/custom_registered_templates".to_string();
    structure.codec_value = None;

    let set = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure_set")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:end_cities")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&set.source_json).unwrap();
    json["structures"][0]["structure"] = serde_json::json!("test:void_city");
    set.id = "test:void_cities".to_string();
    set.source_pack = "file/custom_registered_templates".to_string();
    set.source_json = serde_json::to_vec(&json).unwrap().into();
    set.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
            .unwrap(),
    );
    set.codec_value = None;
    program_with_startup(startup)
}

fn first_registered_end_city_start(
    program: &NoiseFinalDensityProgram,
    set_id: &str,
    structure_id: &str,
    radius: i32,
) -> Option<(
    [i32; 2],
    Vec<bbb_server_worldgen::noise::end_city::EndCityStructureStart>,
)> {
    for ring in 0..=radius {
        let mut chunks = Vec::new();
        if ring == 0 {
            chunks.push([0, 0]);
        } else {
            for chunk_x in -ring..=ring {
                chunks.push([chunk_x, -ring]);
                chunks.push([chunk_x, ring]);
            }
            for chunk_z in (-ring + 1)..ring {
                chunks.push([-ring, chunk_z]);
                chunks.push([ring, chunk_z]);
            }
        }
        for [chunk_x, chunk_z] in chunks {
            if !program.end_city_set_candidate_for_test(set_id, chunk_x, chunk_z) {
                continue;
            }
            let starts = program
                .generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])
                .unwrap();
            if starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            {
                return Some(([chunk_x, chunk_z], starts));
            }
        }
    }
    None
}

fn store(generate_structures: bool) -> ChunkStore {
    ChunkStore::new_end_carvers_with_structure_generation(program(), generate_structures).unwrap()
}

fn ship_city() -> (
    [i32; 2],
    Vec<bbb_server_worldgen::noise::end_city::EndCityStructureStart>,
) {
    static CITY: OnceLock<(
        [i32; 2],
        Vec<bbb_server_worldgen::noise::end_city::EndCityStructureStart>,
    )> = OnceLock::new();
    CITY.get_or_init(|| {
        program()
            .first_end_city_start_in_window(256, true)
            .unwrap()
            .expect("seed-0 End City with a ship in the 256-chunk window")
    })
    .clone()
}

fn full_document(store: &ChunkStore, chunk: (i32, i32)) -> bbb_server_types::save::NamedCompound {
    generation_state_chunk_document_with_light(
        store.column(chunk.0, chunk.1).unwrap(),
        chunk.0,
        chunk.1,
        &|id| program().biome_name(id),
        store.generation_state(chunk.0, chunk.1).unwrap(),
        store
            .structure_component(chunk.0, chunk.1)
            .map(|s| s.to_nbt())
            .as_ref(),
        store.proto_entity_records(chunk.0, chunk.1),
        &store.saved_light_sections(chunk),
    )
    .unwrap()
}

fn reopen(document: &bbb_server_types::save::NamedCompound, chunk: (i32, i32)) -> Java26ChunkData {
    let id = NEXT_REGION_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("bbb-end-full-{}-{id}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let region = root.join(format!(
        "r.{}.{}.mca",
        chunk.0.div_euclid(32),
        chunk.1.div_euclid(32)
    ));
    RegionFileStore::new(&region, &root, WorldSaveLimits::default())
        .unwrap()
        .write_chunk(chunk.0, chunk.1, document, RegionCompression::Zlib, 17)
        .unwrap();
    let reopened = RegionFileStore::new(&region, &root, WorldSaveLimits::default())
        .unwrap()
        .read_chunk(chunk.0, chunk.1)
        .unwrap()
        .unwrap();
    let parsed = Java26ChunkData::parse(
        reopened,
        Java26ChunkContext {
            expected_x: chunk.0,
            expected_z: chunk.1,
            min_section_y: 0,
            section_count: 16,
        },
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    parsed
}

#[test]
fn end_feature_window_reaches_full_and_reopens_with_exact_packet_state() {
    let mut generated = store(false);
    generated
        .try_ensure_noise_full_column(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1)
        .unwrap();
    assert!(matches!(
        generated.generation_state(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert_eq!(generated.min_y(), 0);
    assert_eq!(generated.section_count(), 16);
    let expected_packet = generated
        .noise_full_chunk_packet_data(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1)
        .unwrap()
        .unwrap();
    assert!(generated.has_skylight());
    assert!(!expected_packet.light_data.sky_y_mask.is_empty());
    assert!(!expected_packet.light_data.sky_updates.is_empty());

    let parsed = reopen(
        &full_document(&generated, END_ISLAND_CHUNK),
        END_ISLAND_CHUNK,
    );
    assert_eq!(parsed.status(), Java26ChunkStatus::Full);
    let column = generated
        .column(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1)
        .unwrap()
        .clone();
    let mut restored = store(false);
    restored.insert_column(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1, column.clone());
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored
        .try_ensure_served_full_column(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1)
        .unwrap();
    assert_eq!(
        restored.column(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1),
        Some(&column)
    );
    let actual_packet = restored
        .noise_full_chunk_packet_data(END_ISLAND_CHUNK.0, END_ISLAND_CHUNK.1)
        .unwrap()
        .unwrap();
    assert_eq!(actual_packet.heightmaps, expected_packet.heightmaps);
    assert_eq!(
        actual_packet.light_data.sky_y_mask,
        expected_packet.light_data.sky_y_mask
    );
    assert_eq!(
        actual_packet.light_data.block_y_mask,
        expected_packet.light_data.block_y_mask
    );
    assert_eq!(
        actual_packet.light_data.empty_sky_y_mask,
        expected_packet.light_data.empty_sky_y_mask
    );
    assert_eq!(
        actual_packet.light_data.empty_block_y_mask,
        expected_packet.light_data.empty_block_y_mask
    );
    assert!(
        actual_packet.light_data.sky_updates == expected_packet.light_data.sky_updates,
        "sky update arrays differ: actual {}, expected {}",
        actual_packet.light_data.sky_updates.len(),
        expected_packet.light_data.sky_updates.len()
    );
    assert_eq!(
        actual_packet.light_data.block_updates,
        expected_packet.light_data.block_updates
    );
}

#[test]
fn end_spike_chunk_reaches_full_and_publishes_canonical_crystal_handoff() {
    let spike = bbb_server_worldgen::end_spike::spikes_for_seed(0)[0];
    let chunk = (spike.center_x.div_euclid(16), spike.center_z.div_euclid(16));
    let mut generated = store(false);
    generated
        .try_ensure_noise_full_column(chunk.0, chunk.1)
        .unwrap();
    assert!(matches!(
        generated.generation_state(chunk.0, chunk.1),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(generated.proto_entity_records(chunk.0, chunk.1).is_empty());
    let records = generated.published_worldgen_entity_records(chunk.0, chunk.1);
    assert_eq!(records.len(), 1);
    assert!(records[0].iter().any(|entry| {
        entry.name == "id" && entry.value == NbtSummaryValue::string("minecraft:end_crystal")
    }));
    assert!(records[0]
        .iter()
        .any(|entry| entry.name == "ShowBottom" && entry.value == NbtSummaryValue::Byte(1)));
    assert!(generated
        .noise_full_chunk_packet_data(chunk.0, chunk.1)
        .unwrap()
        .is_some());
}

#[test]
fn structure_enabled_end_city_reaches_full_reopens_and_publishes_markers() {
    let (source, starts) = ship_city();
    let only_city = NoiseStructureChunkData::from_supplied_starts(source[0], source[1], [], [])
        .with_end_city_starts(starts.clone());
    assert!(
        !only_city.is_empty(),
        "EndCity-only payload must be nonempty"
    );

    let mut generated = store(true);
    generated
        .try_ensure_noise_full_column(source[0], source[1])
        .unwrap();
    assert!(matches!(
        generated.generation_state(source[0], source[1]),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    let source_structures = generated.structure_component(source[0], source[1]).unwrap();
    assert_eq!(source_structures.end_city_starts(), starts);
    assert_eq!(
        source_structures
            .references()
            .get(END_CITY_ID)
            .map(Vec::as_slice),
        Some(
            &[bbb_server_worldgen::noise::structure_starts::nbt::pack_chunk(source[0], source[1])]
                [..]
        )
    );

    let mut targets = BTreeSet::from([source]);
    for piece in starts.iter().flat_map(|start| start.pieces()) {
        let bounds = piece.bounding_box();
        for chunk_z in bounds.min.z.div_euclid(16)..=bounds.max.z.div_euclid(16) {
            for chunk_x in bounds.min.x.div_euclid(16)..=bounds.max.x.div_euclid(16) {
                targets.insert([chunk_x, chunk_z]);
            }
        }
    }
    let mut saw_chest = false;
    let mut saw_shulker = false;
    let mut saw_item_frame = false;
    for target in targets {
        let prior_full_handoffs = generated
            .chunks
            .iter()
            .filter(|&(&_chunk, loaded)| {
                matches!(loaded.generation, Some(ChunkGenerationState::NoiseFull(_)))
            })
            .map(|(&chunk, loaded)| (chunk, loaded.published_worldgen_entities.clone()))
            .collect::<BTreeMap<_, _>>();
        generated
            .try_ensure_noise_full_column(target[0], target[1])
            .unwrap();
        for (chunk, expected) in prior_full_handoffs {
            assert_eq!(
                generated.published_worldgen_entity_records(chunk.0, chunk.1),
                expected.as_slice(),
                "relighting prior FULL chunk {chunk:?} changed its entity handoff"
            );
            assert!(generated.proto_entity_records(chunk.0, chunk.1).is_empty());
        }
        saw_chest |= generated
            .block_entities()
            .chunk((target[0], target[1]))
            .any(|entity| {
                crate::block_entity::saved_record(entity)
                    .unwrap()
                    .iter()
                    .any(|entry| {
                        entry.name == "LootTable"
                            && entry.value == NbtSummaryValue::string(END_CITY_LOOT)
                    })
            });
        for record in generated.published_worldgen_entity_records(target[0], target[1]) {
            saw_shulker |= record.iter().any(|entry| {
                entry.name == "id" && entry.value == NbtSummaryValue::string("minecraft:shulker")
            }) && record.iter().any(|entry| {
                entry.name == "Health" && entry.value == NbtSummaryValue::Float(30.0_f32.to_bits())
            });
            saw_item_frame |= record.iter().any(|entry| {
                entry.name == "id"
                    && entry.value == NbtSummaryValue::string("minecraft:item_frame")
            }) && record.iter().any(|entry| {
                entry.name == "Item"
                    && matches!(&entry.value, NbtSummaryValue::Compound(item) if item.iter().any(|field| {
                        field.name == "id"
                            && field.value == NbtSummaryValue::string("minecraft:elytra")
                    }))
            }) && record.iter().any(|entry| entry.name == "Facing");
        }
        assert!(generated
            .proto_entity_records(target[0], target[1])
            .is_empty());
        if saw_chest && saw_shulker && saw_item_frame {
            break;
        }
    }
    assert!(saw_chest, "generated End City omitted its chest marker");
    assert!(saw_shulker, "generated End City omitted its Shulker record");
    assert!(
        saw_item_frame,
        "generated End City ship omitted its ItemFrame record"
    );

    let expected_packet = generated
        .noise_full_chunk_packet_data(source[0], source[1])
        .unwrap()
        .unwrap();
    let parsed = reopen(
        &full_document(&generated, (source[0], source[1])),
        (source[0], source[1]),
    );
    assert_eq!(parsed.status(), Java26ChunkStatus::Full);
    let source_column = generated.column(source[0], source[1]).unwrap().clone();
    let mut restored = store(true);
    restored.insert_column(source[0], source[1], source_column.clone());
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored
        .try_ensure_served_full_column(source[0], source[1])
        .unwrap();
    assert_eq!(restored.column(source[0], source[1]), Some(&source_column));
    assert_eq!(
        restored
            .structure_component(source[0], source[1])
            .unwrap()
            .end_city_starts(),
        starts
    );
    let actual_packet = restored
        .noise_full_chunk_packet_data(source[0], source[1])
        .unwrap()
        .unwrap();
    assert_eq!(actual_packet, expected_packet);
}

#[test]
fn custom_registered_template_structure_ids_end_city_reaches_full_and_reopens() {
    let program = custom_registered_end_city_program();
    assert!(program.owns_end_city_set_for_test("test:void_cities"));
    let mut selected = None;
    'search: for ring in 0..=256 {
        for chunk_x in -ring..=ring {
            for chunk_z in [-ring, ring] {
                if !program.end_city_set_candidate_for_test("test:void_cities", chunk_x, chunk_z) {
                    continue;
                }
                let starts = program
                    .generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])
                    .unwrap();
                if starts
                    .iter()
                    .any(|start| start.structure_id() == "test:void_city")
                    && starts
                        .iter()
                        .all(|start| start.structure_id() != END_CITY_ID)
                {
                    selected = Some([chunk_x, chunk_z]);
                    break 'search;
                }
            }
        }
        for chunk_z in (-ring + 1)..ring {
            for chunk_x in [-ring, ring] {
                if !program.end_city_set_candidate_for_test("test:void_cities", chunk_x, chunk_z) {
                    continue;
                }
                let starts = program
                    .generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])
                    .unwrap();
                if starts
                    .iter()
                    .any(|start| start.structure_id() == "test:void_city")
                    && starts
                        .iter()
                        .all(|start| start.structure_id() != END_CITY_ID)
                {
                    selected = Some([chunk_x, chunk_z]);
                    break 'search;
                }
            }
        }
    }
    let [source_x, source_z] =
        selected.expect("custom End City placement has an eligible outer island");
    let source = (source_x, source_z);
    let mut generated =
        ChunkStore::new_end_carvers_with_structure_generation(Arc::clone(&program), true).unwrap();
    generated
        .try_ensure_noise_full_column(source.0, source.1)
        .unwrap();
    assert!(matches!(
        generated.generation_state(source.0, source.1),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    let structures = generated.structure_component(source.0, source.1).unwrap();
    assert!(structures
        .end_city_starts()
        .iter()
        .any(|start| start.structure_id() == "test:void_city"));

    let expected_packet = generated
        .noise_full_chunk_packet_data(source.0, source.1)
        .unwrap()
        .unwrap();
    let document = generation_state_chunk_document_with_light(
        generated.column(source.0, source.1).unwrap(),
        source.0,
        source.1,
        &|id| program.biome_name(id),
        generated.generation_state(source.0, source.1).unwrap(),
        Some(&structures.to_nbt()),
        generated.proto_entity_records(source.0, source.1),
        &generated.saved_light_sections(source),
    )
    .unwrap();
    let parsed = reopen(&document, source);
    let source_column = generated.column(source.0, source.1).unwrap().clone();
    let mut restored =
        ChunkStore::new_end_carvers_with_structure_generation(Arc::clone(&program), true).unwrap();
    restored.insert_column(source.0, source.1, source_column);
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored
        .try_ensure_served_full_column(source.0, source.1)
        .unwrap();
    assert!(restored
        .structure_component(source.0, source.1)
        .unwrap()
        .end_city_starts()
        .iter()
        .any(|start| start.structure_id() == "test:void_city"));
    assert_eq!(
        restored
            .noise_full_chunk_packet_data(source.0, source.1)
            .unwrap()
            .unwrap(),
        expected_packet
    );
}

#[test]
fn source_exact_end_city_profiles_fixed_custom_settings_reach_full_and_reopen() {
    let program = source_exact_registered_end_city_program(
        fixed_end_biome_source("minecraft:the_end"),
        &["minecraft:the_end"],
    );
    assert!(program.owns_end_city_set_for_test("test:source_exact_cities"));
    let ([source_x, source_z], starts) = first_registered_end_city_start(
        &program,
        "test:source_exact_cities",
        "test:source_exact_city",
        256,
    )
    .expect("the fixed End source has an eligible registered End City");
    assert!(starts
        .iter()
        .all(|start| start.structure_id() == "test:source_exact_city"));
    assert_eq!(
        program
            .generate_end_city_structure_starts_component(source_x, source_z, &[])
            .unwrap(),
        starts
    );

    let source = (source_x, source_z);
    let mut generated =
        ChunkStore::new_end_carvers_with_structure_generation(Arc::clone(&program), true).unwrap();
    generated
        .try_ensure_noise_full_column(source.0, source.1)
        .unwrap();
    assert!(generated
        .structure_component(source.0, source.1)
        .unwrap()
        .end_city_starts()
        .iter()
        .any(|start| start.structure_id() == "test:source_exact_city"));
    let expected_packet = generated
        .noise_full_chunk_packet_data(source.0, source.1)
        .unwrap()
        .unwrap();
    let document = generation_state_chunk_document_with_light(
        generated.column(source.0, source.1).unwrap(),
        source.0,
        source.1,
        &|id| program.biome_name(id),
        generated.generation_state(source.0, source.1).unwrap(),
        generated
            .structure_component(source.0, source.1)
            .map(|s| s.to_nbt())
            .as_ref(),
        generated.proto_entity_records(source.0, source.1),
        &generated.saved_light_sections(source),
    )
    .unwrap();
    let parsed = reopen(&document, source);
    let mut restored =
        ChunkStore::new_end_carvers_with_structure_generation(Arc::clone(&program), true).unwrap();
    restored.insert_column(
        source.0,
        source.1,
        generated.column(source.0, source.1).unwrap().clone(),
    );
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored
        .try_ensure_served_full_column(source.0, source.1)
        .unwrap();
    assert!(restored
        .structure_component(source.0, source.1)
        .unwrap()
        .end_city_starts()
        .iter()
        .any(|start| start.structure_id() == "test:source_exact_city"));
    assert_eq!(
        restored
            .noise_full_chunk_packet_data(source.0, source.1)
            .unwrap()
            .unwrap(),
        expected_packet
    );
}

#[test]
fn source_exact_end_city_profiles_checkerboard_uses_actual_biome_gate() {
    let program = source_exact_registered_end_city_program(
        checkerboard_end_biome_source(&["minecraft:end_highlands", "minecraft:end_barrens"], 0),
        &["minecraft:end_highlands"],
    );
    assert!(program.owns_end_city_set_for_test("test:source_exact_cities"));
    let mut eligible = None;
    let mut gated_miss = false;
    for chunk_x in -128..=128 {
        for chunk_z in -128..=128 {
            if !program.end_city_set_candidate_for_test(
                "test:source_exact_cities",
                chunk_x,
                chunk_z,
            ) {
                continue;
            }
            let starts = program
                .generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])
                .unwrap();
            if (chunk_x.wrapping_add(chunk_z)).rem_euclid(2) == 1 {
                assert!(starts.is_empty());
                gated_miss = true;
            } else if !starts.is_empty() {
                assert_eq!(
                    program
                        .generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])
                        .unwrap(),
                    starts
                );
                eligible = Some([chunk_x, chunk_z]);
            }
            if gated_miss && eligible.is_some() {
                break;
            }
        }
        if gated_miss && eligible.is_some() {
            break;
        }
    }
    assert!(gated_miss, "a candidate in the barrens band must be gated");
    assert!(
        eligible.is_some(),
        "a highlands candidate must retain End City"
    );
}

#[test]
fn end_saved_nonempty_structures_restore_when_new_generation_is_disabled() {
    let (source, _) = ship_city();
    let source = (source[0], source[1]);
    let mut generated = store(true);
    generated
        .try_ensure_noise_full_column(source.0, source.1)
        .unwrap();
    let parsed = reopen(&full_document(&generated, source), source);
    let mut destination = store(false);
    destination.insert_column(
        source.0,
        source.1,
        generated.column(source.0, source.1).unwrap().clone(),
    );
    destination
        .restore_saved_generation_metadata(&parsed)
        .unwrap();
    assert_eq!(
        destination.structure_component(source.0, source.1),
        generated.structure_component(source.0, source.1)
    );
}
