use bbb_protocol::{
    block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    },
    block_types::vanilla_block_type_for_resource_id,
    packets::BlockPos,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use crate::chunk::ChunkStore;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_persistence::generation_state_chunk_document;
use bbb_server_persistence::{
    Java26ChunkContext, Java26ChunkData, RegionCompression, RegionFileStore,
};
use bbb_server_types::chunk_data::ChunkGenerationState;
use bbb_server_types::save::WorldSaveLimits;
use bbb_server_worldgen::noise::beardifier::JigsawBeardifier;
use bbb_server_worldgen::noise::buried_treasure::{
    BuriedTreasureGenerationStub, BuriedTreasureStructureStart, BURIED_TREASURE_ID,
};
use bbb_server_worldgen::noise::create_normal_noise;
use bbb_server_worldgen::noise::mineshaft::MineshaftChestMinecartRecord;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::structure_pieces::{JigsawBoundingBox, JigsawHeightAccess};
use bbb_server_worldgen::noise::structure_starts::{
    JigsawStructureChunkData, JigsawStructureStart, NoiseStructureChunkData,
};
use bbb_server_worldgen::noise::structures::{NoiseStructurePlan, StructureHeightmap};

use super::{
    compile_noise_final_density, compile_noise_level_source, Java26LevelStem, NoiseGeneratedChunk,
};

mod beardifier_density;
mod builtin_surface;
mod cache_all_in_cell;
mod custom_aquifer_default_fluid;
mod custom_carvers;
mod custom_nether_carvers;
mod custom_registered_buried_treasure_ids;
mod fresh_base;
mod inline_settings;
pub mod mineshaft;
mod native_world;
mod registered_structure_sets;
pub mod structure_status;

pub use super::test_support::{
    compound, ephemeral_options, fresh_overworld_program, fresh_overworld_startup,
    fresh_vanilla_startup, noise_stem, overworld_noise_stem, overworld_program, overworld_startup,
    vanilla_startup,
};

use super::test_support::publish_worldgen_element;

/// Puts a configured carver into both the raw registry and the typed
/// registry generation reads, replacing an existing element with the same id.
fn put_configured_carver(startup: &mut Arc<DatapackStartup>, id: &str, source: serde_json::Value) {
    let registries = &mut Arc::get_mut(startup)
        .expect("the test has not shared its startup generation")
        .registries;
    let elements = &mut registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/configured_carver")
        .unwrap()
        .elements;
    let index = elements
        .iter()
        .position(|element| element.id == id)
        .unwrap_or_else(|| {
            let mut element = elements[0].clone();
            element.id = id.to_string();
            elements.push(element);
            elements.len() - 1
        });
    let element = &mut elements[index];
    element.source_json = serde_json::to_vec(&source).unwrap().into();
    element.source = None;
    element.codec_value = None;
    let json = bbb_registry_codec::parse_registry_json(&element.source_json).unwrap();
    let typed = bbb_server_datapack::worldgen::ConfiguredCarver::decode(&json).unwrap();
    registries
        .worldgen
        .configured_carvers
        .insert(id.to_string(), Arc::new(typed));
}

fn configured_carver_json(startup: &Arc<DatapackStartup>, id: &str) -> serde_json::Value {
    let element = startup
        .registries
        .element("minecraft:worldgen/configured_carver", id)
        .unwrap();
    serde_json::from_slice(&element.source_json).unwrap()
}

fn edit_configured_carver(
    startup: &mut Arc<DatapackStartup>,
    id: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) {
    let mut source = configured_carver_json(startup, id);
    edit(&mut source);
    put_configured_carver(startup, id, source);
}

fn direct_biomes_for(biome: NbtSummaryValue) -> NbtSummaryValue {
    let parameter = |name: &str, value: f32| NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Float(value.to_bits()),
    };
    NbtSummaryValue::List(vec![compound(vec![
        ("biome", biome),
        (
            "parameters",
            NbtSummaryValue::Compound(vec![
                parameter("temperature", 0.0),
                parameter("humidity", 0.0),
                parameter("continentalness", 0.0),
                parameter("erosion", 0.0),
                parameter("depth", 0.0),
                parameter("weirdness", 0.0),
                parameter("offset", 0.0),
            ]),
        ),
    ])])
}

fn direct_biomes() -> NbtSummaryValue {
    direct_biomes_for(NbtSummaryValue::string("plains"))
}

fn direct_noise_stem(biomes: NbtSummaryValue) -> Java26LevelStem {
    let mut stem = overworld_noise_stem();
    let source = biome_source_fields_mut(&mut stem);
    source.retain(|entry| entry.name != "preset");
    source.push(NbtSummaryEntry {
        name: "biomes".into(),
        value: biomes,
    });
    stem
}

fn biome_source_fields_mut(stem: &mut Java26LevelStem) -> &mut Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(source) = &mut generator
        .iter_mut()
        .find(|entry| entry.name == "biome_source")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    source
}

fn end_noise_stem() -> Java26LevelStem {
    let mut stem = noise_stem(NbtSummaryValue::string("minecraft:end"));
    stem.dimension_type = "minecraft:the_end".to_string();
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    let biome_source = generator
        .iter_mut()
        .find(|entry| entry.name == "biome_source")
        .unwrap();
    biome_source.value = compound(vec![("type", NbtSummaryValue::string("minecraft:the_end"))]);
    stem
}

#[test]
fn saved_nether_stem_compiles_the_built_in_final_density_program() {
    let startup = vanilla_startup();
    let program = compile_noise_final_density(
        &noise_stem(NbtSummaryValue::string("minecraft:nether")),
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:the_nether.generator",
    )
    .unwrap();
    assert_eq!(program.dimension_type_id(), "minecraft:the_nether");
    assert_eq!(program.settings_id(), "minecraft:nether");
    assert_eq!((program.min_y(), program.height()), (0, 128));
    assert_eq!((program.cell_width(), program.cell_height()), (4, 8));
    for (x, y, z) in [(0, 0, 0), (16, 32, -16), (-73, 64, 91)] {
        assert!(program.compute_single_point(x, y, z).is_finite());
    }
}

fn default_state(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .and_then(|block| vanilla_block_default_state_id(block.id))
        .unwrap_or_else(|| panic!("{name} must have a Vanilla 26.1 default state"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn noise_chunk_state_facts(chunk: &NoiseGeneratedChunk) -> (String, serde_json::Value) {
    noise_chunk_state_facts_in_height(chunk, -64, 384)
}

fn noise_chunk_state_facts_in_height(
    chunk: &NoiseGeneratedChunk,
    min_y: i32,
    height: i32,
) -> (String, serde_json::Value) {
    let mut bytes = Vec::with_capacity(16 * height as usize * 16 * 4);
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for y in min_y..min_y + height {
        for z in 0..16 {
            for x in 0..16 {
                let state = chunk.column().get_block_state(x, y, z).unwrap();
                bytes.extend_from_slice(&state.to_le_bytes());
                *counts.entry(state.to_string()).or_default() += 1;
            }
        }
    }
    (sha256_hex(&bytes), serde_json::to_value(counts).unwrap())
}

#[test]
fn custom_noise_dimension_type_no_skylight_binds_store_automatically() {
    let mut startup = fresh_vanilla_startup();
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap();
    let mut custom = registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:the_nether")
        .unwrap()
        .clone();
    custom.id = "example:no_skylight".to_string();
    custom.source_pack = "file/custom_dimension".to_string();
    registry.elements.push(custom);
    let mut stem = super::test_support::nether_noise_stem();
    stem.dimension_type = "example:no_skylight".to_string();
    let program = Arc::new(
        compile_noise_final_density(
            &stem,
            Arc::new(startup),
            0,
            "dimensions.example:no_skylight.generator",
        )
        .unwrap(),
    );
    assert!(!program.dimension_type_shape().has_skylight);

    let store = ChunkStore::new_noise_carvers_with_structure_generation(program, false).unwrap();
    assert!(!store.has_skylight());
}

fn custom_noise_surface_startup(surface_rule: serde_json::Value) -> Arc<DatapackStartup> {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    let registry = Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap();
    let mut custom = registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap()
        .clone();
    let mut settings: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    settings["surface_rule"] = surface_rule;
    custom.id = "example:custom_surface".to_string();
    custom.source_pack = "file/custom_surface".to_string();
    custom.source_json = serde_json::to_vec(&settings).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(settings)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        custom,
    );
    startup
}

fn custom_shift_density_startup(final_density: serde_json::Value) -> Arc<DatapackStartup> {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    let generation = &mut Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries;
    let settings_registry = generation
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap();
    let mut custom = settings_registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap()
        .clone();
    let mut settings: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    settings["legacy_random_source"] = serde_json::json!(false);
    settings["noise_router"]["final_density"] = final_density;
    custom.id = "example:shift_density".to_string();
    custom.source_pack = "file/shift_density".to_string();
    custom.source_json = serde_json::to_vec(&settings).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(settings)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    publish_worldgen_element(generation, "minecraft:worldgen/noise_settings", custom);

    let noise_registry = generation
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise")
        .unwrap();
    let mut argument = noise_registry.elements[0].clone();
    let parameters = serde_json::json!({"firstOctave": -3, "amplitudes": [1.0]});
    argument.id = "example:shift_argument".to_string();
    argument.source_pack = "file/shift_density".to_string();
    argument.source_json = serde_json::to_vec(&parameters).unwrap().into();
    argument.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(parameters)).unwrap(),
        )
        .unwrap(),
    );
    argument.codec_value = None;
    publish_worldgen_element(generation, "minecraft:worldgen/noise", argument);
    startup
}

fn with_settings(mut stem: Java26LevelStem, settings: &str) -> Java26LevelStem {
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    generator
        .iter_mut()
        .find(|entry| entry.name == "settings")
        .unwrap()
        .value = NbtSummaryValue::string(settings);
    stem
}

#[test]
fn custom_noise_surface_uses_the_named_rule() {
    let surface_rule = serde_json::json!({
        "type": "minecraft:block",
        "result_state": {"Name": "minecraft:bedrock"}
    });
    let startup = custom_noise_surface_startup(surface_rule);
    let stem = with_settings(direct_noise_stem(direct_biomes()), "example:custom_surface");
    let program = compile_noise_final_density(&stem, startup, 0, "generator").unwrap();
    let base = program.generate_fresh_base_chunk(0, 0).unwrap();
    let surface = program.generate_fresh_surface_chunk(0, 0).unwrap();
    assert_ne!(
        noise_chunk_state_facts(&base).0,
        noise_chunk_state_facts(&surface).0
    );

    for settings in [
        "minecraft:large_biomes",
        "minecraft:amplified",
        "minecraft:caves",
        "minecraft:floating_islands",
    ] {
        let program = compile_noise_final_density(
            &with_settings(overworld_noise_stem(), settings),
            vanilla_startup(),
            0,
            "generator",
        )
        .unwrap();
        program.generate_fresh_surface_chunk(0, 0).unwrap();
    }
}

#[test]
fn alternate_overworld_settings_reach_carvers_features_and_full() {
    for settings in [
        "minecraft:large_biomes",
        "minecraft:amplified",
        "minecraft:caves",
        "minecraft:floating_islands",
    ] {
        let program = compile_noise_level_source(
            &with_settings(overworld_noise_stem(), settings),
            vanilla_startup(),
            0,
            "generator",
        )
        .unwrap();
        assert_eq!(program.settings_id(), settings);
        let mut store =
            ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false)
                .unwrap();
        store.try_generate_fresh_noise_full_column(0, 0).unwrap();
        assert!(matches!(
            store.generation_state(0, 0),
            Some(ChunkGenerationState::NoiseFull(_))
        ));
        assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());
    }
}

#[test]
fn custom_direct_noise_settings_reach_full() {
    let surface_rule = serde_json::json!({
        "type": "minecraft:block",
        "result_state": {"Name": "minecraft:bedrock"}
    });
    let stem = with_settings(direct_noise_stem(direct_biomes()), "example:custom_surface");
    let program = compile_noise_level_source(
        &stem,
        custom_noise_surface_startup(surface_rule.clone()),
        0,
        "generator",
    )
    .unwrap();
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(matches!(
        store.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());

    let mut invalid = custom_noise_surface_startup(surface_rule);
    let biome = Arc::get_mut(&mut invalid)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:plains")
        .unwrap();
    let mut source: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
    source["carvers"] = serde_json::json!(["example:missing"]);
    biome.source_json = serde_json::to_vec(&source).unwrap().into();
    biome.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(source)).unwrap())
            .unwrap(),
    );
    biome.codec_value = None;
    let error = compile_noise_level_source(&stem, invalid, 0, "generator")
        .err()
        .expect("an incompatible direct-biome carver list must fail closed")
        .to_string();
    assert!(error.contains("minecraft:plains.carvers"), "{error}");
}

#[test]
fn source_exact_shift_density_full() {
    let seed = 42;
    let density = serde_json::json!({
        "type": "minecraft:shift",
        "argument": "example:shift_argument"
    });
    let stem = with_settings(direct_noise_stem(direct_biomes()), "example:shift_density");
    let program = compile_noise_level_source(
        &stem,
        custom_shift_density_startup(density),
        seed,
        "generator",
    )
    .unwrap();
    let expected_noise = create_normal_noise(seed, "example:shift_argument", -3, &[1.0]).unwrap();
    let (x, y, z) = (11, 23, -7);
    let expected = expected_noise.get_value(
        f64::from(x) * 0.25,
        f64::from(y) * 0.25,
        f64::from(z) * 0.25,
    ) * 4.0;
    assert_eq!(
        program.compute_single_point(x, y, z).to_bits(),
        expected.to_bits()
    );
    let shift_a_value =
        expected_noise.get_value(f64::from(x) * 0.25, 0.0, f64::from(z) * 0.25) * 4.0;
    assert_ne!(expected.to_bits(), shift_a_value.to_bits());

    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(matches!(
        store.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());

    let inline = serde_json::json!({
        "type": "minecraft:shift",
        "argument": {"firstOctave": -3, "amplitudes": [1.0]}
    });
    assert!(compile_noise_level_source(
        &stem,
        custom_shift_density_startup(inline),
        seed,
        "generator",
    )
    .is_err());
}

#[test]
fn overworld_surface_stage_changes_material_without_claiming_serving() {
    let program = overworld_program(true, 0);
    let base = program.generate_fresh_base_chunk(0, 0).unwrap();
    let surface = program.generate_fresh_surface_chunk(0, 0).unwrap();
    let grass = default_state("minecraft:grass_block");
    let dirt = default_state("minecraft:dirt");
    let changed = (-64..320)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| (x, y, z))))
        .filter(|&(x, y, z)| {
            base.column().get_block_state(x, y, z) != surface.column().get_block_state(x, y, z)
        })
        .count();
    let surface_states = (-64..320)
        .flat_map(|y| (0..16).flat_map(move |z| (0..16).map(move |x| (x, y, z))))
        .filter_map(|(x, y, z)| surface.column().get_block_state(x, y, z))
        .filter(|&state| state == grass || state == dirt)
        .count();
    assert!(changed > 0);
    assert!(surface_states > 0);
}

struct StructureFlatHeight;

impl JigsawHeightAccess for StructureFlatHeight {
    fn first_free_height(&self, _x: i32, _z: i32, _heightmap: StructureHeightmap) -> Option<i32> {
        Some(64)
    }
}

#[test]
fn overworld_structure_starts_use_frozen_biomes_and_generated_heights() {
    let program = overworld_program(true, 0);
    let target = program.jigsaw_start_chunk("minecraft:trial_chambers", "minecraft:trial_chambers");
    let starts = program
        .generate_jigsaw_structure_starts_component(target[0], target[1], &[])
        .unwrap();
    assert_eq!(starts.chunk(), target);
    assert!(starts.references().is_empty());
    assert_eq!(
        starts
            .starts()
            .iter()
            .map(JigsawStructureStart::structure_id)
            .collect::<Vec<_>>(),
        ["minecraft:trial_chambers"]
    );
    let chunk = starts.chunk();
    let stable = program
        .generate_jigsaw_structure_starts_component(chunk[0], chunk[1], starts.starts())
        .unwrap();
    assert_eq!(stable, starts);
}

#[test]
fn overworld_buried_treasure_start_uses_frozen_biome_and_ocean_floor() {
    let program = overworld_program(true, 0);
    let target = program.buried_treasure_chunk();
    let start = program
        .generate_buried_treasure_start_component(target[0], target[1], false)
        .unwrap()
        .unwrap();
    let [chunk_x, chunk_z] = start.source_chunk();

    assert_eq!(start.source_chunk(), target);
    assert_eq!(start.generation_anchor().x, chunk_x * 16 + 8);
    assert_eq!(start.generation_anchor().z, chunk_z * 16 + 8);
    assert_eq!(start.piece().position().x, chunk_x * 16 + 9);
    assert_eq!(start.piece().position().y, 90);
    assert_eq!(start.piece().position().z, chunk_z * 16 + 9);
    assert!(program
        .generate_buried_treasure_start_component(chunk_x, chunk_z, true)
        .unwrap()
        .is_none());
}

#[test]
fn complete_structure_source_window_precedes_beardified_noise() {
    let program = overworld_program(true, 0);
    let trial = program.jigsaw_start_chunk("minecraft:trial_chambers", "minecraft:trial_chambers");
    let target_starts = program
        .generate_jigsaw_structure_starts_component(trial[0], trial[1], &[])
        .unwrap();
    let target = target_starts.chunk();
    let higher_status_target = JigsawStructureChunkData::from_supplied_starts(
        target[0],
        target[1],
        target_starts.starts().iter().cloned(),
        target_starts.starts(),
    );
    let mut queried = std::collections::BTreeSet::new();
    let (generated, references) = program
        .generate_fresh_carved_chunk_from_complete_jigsaw_source_window(
            &target_starts,
            |source_x, source_z| {
                assert!(queried.insert([source_x, source_z]));
                Ok::<_, &str>(if [source_x, source_z] == target {
                    higher_status_target.clone()
                } else {
                    JigsawStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
                })
            },
        )
        .unwrap();
    assert_eq!(queried.len(), 17 * 17);
    assert!(queried.contains(&[target[0] - 8, target[1] - 8]));
    assert!(queried.contains(&[target[0] + 8, target[1] + 8]));
    assert_eq!(generated.chunk_x(), target[0]);
    assert_eq!(generated.chunk_z(), target[1]);
    assert_eq!(references.starts(), target_starts.starts());
    assert_eq!(
        references.references()["minecraft:trial_chambers"],
        [((u64::from(target[0] as u32)) | (u64::from(target[1] as u32) << 32)) as i64]
    );

    let mut failed_window = std::collections::BTreeSet::new();
    let error = program
        .generate_fresh_carved_chunk_from_complete_jigsaw_source_window(
            &target_starts,
            |source_x, source_z| {
                assert!(failed_window.insert([source_x, source_z]));
                if [source_x, source_z] == [target[0] + 8, target[1] + 8] {
                    Err("source has not completed STRUCTURE_STARTS")
                } else {
                    Ok(if [source_x, source_z] == target {
                        target_starts.clone()
                    } else {
                        JigsawStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
                    })
                }
            },
        )
        .unwrap_err();
    assert_eq!(failed_window.len(), 17 * 17);
    assert!(error
        .to_string()
        .contains("source has not completed STRUCTURE_STARTS"));
}

#[test]
fn structure_status_tasks_refuse_unowned_candidates_and_honor_disabled_generation() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let ring = *rings["minecraft:strongholds"].iter().next().unwrap();
    let mineshaft_chunk = program
        .find_structure_candidate("minecraft:mineshafts", 256, |_| true)
        .unwrap();

    let mineshaft = program
        .generate_structure_starts_status_task(mineshaft_chunk[0], mineshaft_chunk[1], true, &rings)
        .unwrap();
    assert!(mineshaft.starts().is_empty());
    assert_eq!(mineshaft.mineshaft_starts().len(), 1);
    assert_eq!(
        mineshaft.mineshaft_starts()[0].structure_id(),
        "minecraft:mineshaft"
    );
    assert!(mineshaft.mineshaft_starts()[0].pieces().len() > 1);
    assert!(mineshaft.references().is_empty());

    let target = mineshaft.chunk();
    let (references, sources) = NoiseStructureChunkData::references_from_complete_source_window(
        &mineshaft,
        |source_x, source_z| {
            Ok::<_, &str>(if [source_x, source_z] == target {
                mineshaft.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap();
    assert_eq!(sources.len(), 17 * 17);
    assert_eq!(
        references.references()["minecraft:mineshaft"],
        [((u64::from(target[0] as u32)) | (u64::from(target[1] as u32) << 32)) as i64]
    );
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&references.to_nbt(), target[0], target[1])
        .unwrap();
    assert_eq!(restored, references);

    let stronghold = program
        .generate_structure_starts_status_task(ring[0], ring[1], true, &rings)
        .unwrap();
    assert_eq!(stronghold.stronghold_starts().len(), 1);
    assert_eq!(
        stronghold.stronghold_starts()[0].structure_id(),
        "minecraft:stronghold"
    );
    assert_eq!(stronghold.stronghold_starts()[0].start_chunk(), ring);
    assert!(stronghold.stronghold_starts()[0]
        .pieces()
        .iter()
        .any(|piece| piece.piece_id() == "minecraft:shpr"));

    let disabled = program
        .generate_structure_starts_status_task(
            mineshaft_chunk[0],
            mineshaft_chunk[1],
            false,
            &BTreeMap::new(),
        )
        .unwrap();
    assert!(disabled.starts().is_empty());
    assert!(disabled.buried_treasure_start().is_none());
    assert!(disabled.mineshaft_starts().is_empty());
    assert!(disabled.shipwreck_starts().is_empty());
    assert!(disabled.ocean_ruin_starts().is_empty());
    assert!(disabled.igloo_starts().is_empty());
    assert!(disabled.scattered_starts().is_empty());
    assert!(disabled.ruined_portal_starts().is_empty());
    assert!(disabled.ocean_monument_starts().is_empty());
    assert!(disabled.woodland_mansion_starts().is_empty());
    assert!(disabled.stronghold_starts().is_empty());
    assert!(disabled.references().is_empty());
}

#[test]
fn typed_structure_start_region_reopen_restores_pieces_and_references() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 42);
    let possible_biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let plan = NoiseStructurePlan::from_startup(&startup, &possible_biomes).unwrap();
    let chunk = [-3, -5];
    let graph = plan
        .jigsaw_piece_graph(
            &startup,
            "minecraft:village_plains",
            42,
            chunk[0],
            chunk[1],
            -64,
            384,
            &StructureFlatHeight,
        )
        .unwrap();
    let start = JigsawStructureStart::from_graph(&plan, graph, 0).unwrap();
    let mineshaft = MineshaftStructureStart::from_graph(
        &plan,
        plan.mineshaft_piece_graph(
            "minecraft:mineshaft",
            42,
            chunk[0],
            chunk[1],
            -64,
            63,
            &StructureFlatHeight,
        )
        .unwrap(),
        0,
    )
    .unwrap();
    let buried = BuriedTreasureStructureStart::from_generation_stub(
        BuriedTreasureGenerationStub::new(
            chunk,
            BlockPos {
                x: chunk[0] * 16 + 8,
                y: 64,
                z: chunk[1] * 16 + 8,
            },
            BlockPos {
                x: chunk[0] * 16 + 9,
                y: 90,
                z: chunk[1] * 16 + 9,
            },
        ),
        0,
    );
    let target_starts =
        JigsawStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [start.clone()], [])
            .with_buried_treasure_start(buried.clone())
            .with_mineshaft_starts([mineshaft.clone()]);
    let (structures, source_window) =
        JigsawStructureChunkData::references_from_complete_source_window(
            &target_starts,
            |source_x, source_z| {
                Ok::<_, &str>(if [source_x, source_z] == chunk {
                    target_starts.clone()
                } else {
                    JigsawStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
                })
            },
        )
        .unwrap();
    assert_eq!(source_window.len(), 17 * 17);

    let unadjusted = program
        .generate_fresh_carved_chunk(chunk[0], chunk[1])
        .unwrap();
    let generated = program
        .generate_fresh_carved_chunk_from_jigsaw_structure_references(
            &structures,
            |_, _| -> Result<JigsawStructureChunkData, &str> {
                panic!("the target's own start must resolve without a source callback")
            },
        )
        .unwrap();
    let changed_blocks = (-64..320)
        .flat_map(|y| (0..16).flat_map(move |x| (0..16).map(move |z| (x, y, z))))
        .filter(|&(x, y, z)| {
            generated.column().get_block_state(x, y, z)
                != unadjusted.column().get_block_state(x, y, z)
        })
        .count();
    assert!(
        changed_blocks > 0,
        "the village start did not beardify its chunk"
    );
    let restored = JigsawStructureChunkData::from_nbt(
        &structures.to_nbt(),
        chunk[0],
        chunk[1],
        &plan,
        &startup,
    );
    assert!(restored.has_valid_start("minecraft:village_plains"));
    assert!(restored.has_valid_start(BURIED_TREASURE_ID));
    assert!(restored.has_valid_start("minecraft:mineshaft"));
    assert_eq!(
        restored.references()["minecraft:village_plains"],
        [((u64::from(chunk[0] as u32)) | (u64::from(chunk[1] as u32) << 32)) as i64]
    );
    assert_eq!(
        restored.references()[BURIED_TREASURE_ID],
        [((u64::from(chunk[0] as u32)) | (u64::from(chunk[1] as u32) << 32)) as i64]
    );
    assert_eq!(
        restored.references()["minecraft:mineshaft"],
        [((u64::from(chunk[0] as u32)) | (u64::from(chunk[1] as u32) << 32)) as i64]
    );
}

#[test]
fn built_in_overworld_carvers_load_by_coordinate_and_retain_persistence_state() {
    let program = overworld_program(true, 42);
    let expected = program.generate_fresh_carved_chunk(-8, 8).unwrap();
    let expected_postprocessing = expected.postprocessing_positions().to_vec();
    let expected_mask = expected.carving_mask().unwrap().to_vec();
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();

    store.try_load_column(-8, 8).unwrap();
    assert_eq!(
        store.get_block_state(-128, -64, 128),
        expected.column().get_block_state(0, -64, 0)
    );
    let ChunkGenerationState::Carvers {
        postprocessing_positions,
        carving_mask,
    } = store.generation_state(-8, 8).unwrap()
    else {
        panic!("storage-only Overworld chunks must retain CARVERS state");
    };
    assert_eq!(postprocessing_positions, &expected_postprocessing);
    assert_eq!(carving_mask, &expected_mask);
    assert_eq!(
        store.pending_initial_persistence().collect::<Vec<_>>(),
        [(-8, 8)]
    );

    store.mark_persisted(-8, 8);
    assert_eq!(store.pending_initial_persistence().count(), 0);
}
