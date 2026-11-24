use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
    Java26ChunkStatus, RegionCompression, RegionFileStore,
};
use bbb_server_types::save::WorldSaveLimits;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use bbb_server_worldgen::noise::structures::StructureStartSetOwner;

use super::noise_generator::test_support::{
    fresh_vanilla_startup, nether_noise_stem, nether_program,
};
use super::noise_generator::{compile_noise_level_source, NoiseFinalDensityProgram};
use super::ChunkStore;
use bbb_server_types::chunk_data::ChunkGenerationState;

const CENTER: (i32, i32) = (0, 0);
static NEXT_REGION_ID: AtomicU64 = AtomicU64::new(1);
const CUSTOM_FORTRESS_ID: &str = "test:infernal_keep";
const CUSTOM_COMPLEX_SET_ID: &str = "test:infernal_complexes";
const CUSTOM_FOSSIL_ID: &str = "test:ancient_remains";
const CUSTOM_FOSSIL_SET_ID: &str = "test:ancient_remains_set";

fn store(generate_structures: bool) -> ChunkStore {
    ChunkStore::new_nether_carvers_with_structure_generation(nether_program(0), generate_structures)
        .unwrap()
}

fn custom_nether_program(include_mixed_set: bool) -> Arc<NoiseFinalDensityProgram> {
    let mut startup = fresh_vanilla_startup();
    let structures = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure")
        .unwrap();
    let fortress = structures
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:fortress")
        .unwrap();
    fortress.id = CUSTOM_FORTRESS_ID.to_string();
    fortress.source_pack = "file/custom_registered_nether".to_string();
    fortress.codec_value = None;
    let fossil = structures
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:nether_fossil")
        .unwrap();
    fossil.id = CUSTOM_FOSSIL_ID.to_string();
    fossil.source_pack = "file/custom_registered_nether".to_string();
    fossil.codec_value = None;

    let sets = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/structure_set")
        .unwrap();
    let complex = sets
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:nether_complexes")
        .unwrap();
    let mut complex_json: serde_json::Value = serde_json::from_slice(&complex.source_json).unwrap();
    complex_json["structures"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|selection| selection["structure"] == "minecraft:fortress")
        .unwrap()["structure"] = serde_json::json!(CUSTOM_FORTRESS_ID);
    if include_mixed_set {
        complex_json["structures"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "structure": CUSTOM_FOSSIL_ID,
                "weight": 1
            }));
    }
    complex.id = CUSTOM_COMPLEX_SET_ID.to_string();
    complex.source_pack = "file/custom_registered_nether".to_string();
    complex.source_json = serde_json::to_vec(&complex_json).unwrap().into();
    complex.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(complex_json)).unwrap(),
        )
        .unwrap(),
    );
    complex.codec_value = None;

    let fossil_set = sets
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:nether_fossils")
        .unwrap();
    let mut fossil_json: serde_json::Value =
        serde_json::from_slice(&fossil_set.source_json).unwrap();
    fossil_json["structures"][0]["structure"] = serde_json::json!(CUSTOM_FOSSIL_ID);
    fossil_set.id = CUSTOM_FOSSIL_SET_ID.to_string();
    fossil_set.source_pack = "file/custom_registered_nether".to_string();
    fossil_set.source_json = serde_json::to_vec(&fossil_json).unwrap().into();
    fossil_set.source = Some(
        bbb_registry_codec::decode_direct_registry_json(
            &serde_json::to_vec(&(fossil_json)).unwrap(),
        )
        .unwrap(),
    );
    fossil_set.codec_value = None;

    Arc::new(
        compile_noise_level_source(
            &nether_noise_stem(),
            Arc::new(startup),
            0,
            "dimensions.minecraft:the_nether.generator",
        )
        .unwrap(),
    )
}

fn custom_store(program: Arc<NoiseFinalDensityProgram>) -> ChunkStore {
    ChunkStore::new_nether_carvers_with_structure_generation(program, true).unwrap()
}

fn full_document(store: &ChunkStore, chunk: (i32, i32)) -> bbb_server_types::save::NamedCompound {
    let program = nether_program(0);
    generation_state_chunk_document_with_light(
        store.column(chunk.0, chunk.1).unwrap(),
        chunk.0,
        chunk.1,
        &|id| program.biome_name(id),
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

fn full_document_for_program(
    store: &ChunkStore,
    program: &NoiseFinalDensityProgram,
    chunk: (i32, i32),
) -> bbb_server_types::save::NamedCompound {
    generation_state_chunk_document_with_light(
        store.column(chunk.0, chunk.1).unwrap(),
        chunk.0,
        chunk.1,
        &|id| program.biome_name(id),
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
    let root = std::env::temp_dir().join(format!(
        "bbb-client-nether-full-{}-{id}",
        std::process::id()
    ));
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
fn nether_feature_window_reaches_full_and_reopens_with_exact_packet_state() {
    let mut generated = store(false);
    generated
        .try_ensure_noise_full_column(CENTER.0, CENTER.1)
        .unwrap();
    assert!(matches!(
        generated.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert_eq!(generated.min_y(), 0);
    assert_eq!(generated.section_count(), 16);
    let expected_packet = generated
        .noise_full_chunk_packet_data(0, 0)
        .unwrap()
        .unwrap();
    assert!(!generated.has_skylight());
    assert!(generated
        .saved_light_sections(CENTER)
        .iter()
        .all(|section| section.sky_light.is_none()));
    assert!(expected_packet.light_data.sky_y_mask.is_empty());
    assert!(expected_packet.light_data.empty_sky_y_mask.is_empty());
    assert!(expected_packet.light_data.sky_updates.is_empty());
    assert!(!expected_packet.light_data.block_y_mask.is_empty());
    assert!(!expected_packet.light_data.block_updates.is_empty());

    let parsed = reopen(&full_document(&generated, CENTER), CENTER);
    assert_eq!(parsed.status(), Java26ChunkStatus::Full);
    let column = generated.column(0, 0).unwrap().clone();
    let mut restored = store(false);
    restored.insert_column(0, 0, column.clone());
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored.try_ensure_served_full_column(0, 0).unwrap();
    assert_eq!(restored.column(0, 0), Some(&column));
    let actual_packet = restored
        .noise_full_chunk_packet_data(0, 0)
        .unwrap()
        .unwrap();
    assert!(actual_packet.light_data.sky_y_mask.is_empty());
    assert!(actual_packet.light_data.empty_sky_y_mask.is_empty());
    assert!(actual_packet.light_data.sky_updates.is_empty());
    assert_eq!(actual_packet, expected_packet);
}

#[test]
fn nether_structures_enabled_reaches_full_and_restores_typed_structure_state() {
    let program = nether_program(0);
    let structure_chunk = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| (chunk_x, chunk_z)))
        .find(|&(chunk_x, chunk_z)| {
            let (_, fortress) = program
                .generate_nether_complex_structure_starts_component(chunk_x, chunk_z, &[], &[])
                .unwrap();
            !fortress.is_empty()
        })
        .expect("seed zero must select a fortress in the first four regions");
    let mut generated = store(true);
    generated
        .try_ensure_noise_full_column(structure_chunk.0, structure_chunk.1)
        .unwrap();
    let component = generated
        .structure_component(structure_chunk.0, structure_chunk.1)
        .unwrap();
    assert_eq!(component.fortress_starts().len(), 1);
    assert!(!component.references().is_empty());
    let expected_structures = component.clone();
    let document = full_document(&generated, structure_chunk);
    let parsed = reopen(&document, structure_chunk);
    let column = generated
        .column(structure_chunk.0, structure_chunk.1)
        .unwrap()
        .clone();
    let expected_packet = generated
        .noise_full_chunk_packet_data(structure_chunk.0, structure_chunk.1)
        .unwrap();
    let mut restored = store(true);
    restored.insert_column(structure_chunk.0, structure_chunk.1, column.clone());
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    assert_eq!(
        restored
            .structure_component(structure_chunk.0, structure_chunk.1)
            .unwrap(),
        &expected_structures
    );
    restored
        .try_ensure_served_full_column(structure_chunk.0, structure_chunk.1)
        .unwrap();
    assert_eq!(
        restored.column(structure_chunk.0, structure_chunk.1),
        Some(&column)
    );
    assert_eq!(
        restored
            .noise_full_chunk_packet_data(structure_chunk.0, structure_chunk.1)
            .unwrap(),
        expected_packet
    );
    generated.mark_persisted(structure_chunk.0, structure_chunk.1);
    generated
        .try_ensure_noise_full_columns(&[
            structure_chunk,
            (structure_chunk.0, structure_chunk.1 + 1),
        ])
        .unwrap();
    for chunk in [structure_chunk, (structure_chunk.0, structure_chunk.1 + 1)] {
        let parsed = reopen(&full_document(&generated, chunk), chunk);
        assert_eq!(parsed.status(), Java26ChunkStatus::Full);
        assert!(generated
            .noise_full_chunk_packet_data(chunk.0, chunk.1)
            .unwrap()
            .is_some());
    }
    assert_eq!(
        generated
            .structure_component(structure_chunk.0, structure_chunk.1)
            .unwrap()
            .fortress_starts(),
        expected_structures.fortress_starts()
    );
}

#[test]
fn nether_fossil_start_round_trips_through_typed_structure_persistence() {
    let program = nether_program(0);
    let generate = |[x, z]: [i32; 2]| {
        program
            .generate_nether_fossil_structure_starts_component(x, z, &[])
            .unwrap()
    };
    let chunk = program
        .find_structure_candidate("minecraft:nether_fossils", 128, |chunk| {
            !generate(chunk).is_empty()
        })
        .expect("seed zero has a viable Nether fossil");
    let starts = generate(chunk);
    let component = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
        .with_nether_fossil_starts(starts);
    let restored = program
        .restored_structure_chunk_data(&component.to_nbt(), chunk[0], chunk[1])
        .unwrap();
    assert_eq!(restored, component);
}

#[test]
fn nether_bastion_start_uses_the_shared_feature_window() {
    let program = nether_program(0);
    let bastion_chunk = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| (chunk_x, chunk_z)))
        .find(|&(chunk_x, chunk_z)| {
            let (jigsaw, _) = program
                .generate_nether_complex_structure_starts_component(chunk_x, chunk_z, &[], &[])
                .unwrap();
            !jigsaw.is_empty()
        })
        .expect("seed zero must select a bastion in the first four regions");
    let mut generated = store(true);
    generated
        .try_ensure_noise_full_column(bastion_chunk.0, bastion_chunk.1)
        .unwrap();
    let component = generated
        .structure_component(bastion_chunk.0, bastion_chunk.1)
        .unwrap();
    assert!(component
        .starts()
        .iter()
        .any(|start| start.structure_id() == "minecraft:bastion_remnant"));
    assert!(matches!(
        generated.generation_state(bastion_chunk.0, bastion_chunk.1),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
}

#[test]
fn nether_saved_nonempty_structures_restore_when_new_generation_is_disabled() {
    let program = nether_program(0);
    let structure_chunk = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| (chunk_x, chunk_z)))
        .find(|&(chunk_x, chunk_z)| {
            let (_, fortress) = program
                .generate_nether_complex_structure_starts_component(chunk_x, chunk_z, &[], &[])
                .unwrap();
            !fortress.is_empty()
        })
        .expect("seed zero must select a fortress in the first four regions");
    let mut generated = store(true);
    generated
        .try_ensure_noise_full_column(structure_chunk.0, structure_chunk.1)
        .unwrap();
    let parsed = reopen(&full_document(&generated, structure_chunk), structure_chunk);
    let mut destination = store(false);
    destination.insert_column(
        structure_chunk.0,
        structure_chunk.1,
        generated
            .column(structure_chunk.0, structure_chunk.1)
            .unwrap()
            .clone(),
    );
    destination
        .restore_saved_generation_metadata(&parsed)
        .unwrap();
    assert_eq!(
        destination.structure_component(structure_chunk.0, structure_chunk.1),
        generated.structure_component(structure_chunk.0, structure_chunk.1)
    );
}

#[test]
fn custom_registered_nether_structure_ids_reach_full_and_restore_exactly() {
    let program = custom_nether_program(false);
    assert_eq!(
        program.nether_structure_set_owner_for_test(CUSTOM_COMPLEX_SET_ID),
        Some(StructureStartSetOwner::NetherComplex)
    );
    assert_eq!(
        program.nether_structure_set_owner_for_test(CUSTOM_FOSSIL_SET_ID),
        Some(StructureStartSetOwner::NetherFossil)
    );
    assert!(program.owns_fortress_structure(CUSTOM_FORTRESS_ID));
    assert!(program.owns_nether_fossil_structure(CUSTOM_FOSSIL_ID));

    let fortress = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| [chunk_x, chunk_z]))
        .find_map(|chunk| {
            let (_, starts) = program
                .generate_nether_complex_structure_starts_component(chunk[0], chunk[1], &[], &[])
                .unwrap();
            starts
                .into_iter()
                .find(|start| start.structure_id() == CUSTOM_FORTRESS_ID)
                .map(|start| (chunk, start))
        })
        .expect("custom fortress must retain the registered set placement");
    let fossil = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| [chunk_x, chunk_z]))
        .find_map(|chunk| {
            program
                .generate_nether_fossil_structure_starts_component(chunk[0], chunk[1], &[])
                .unwrap()
                .into_iter()
                .find(|start| start.structure_id() == CUSTOM_FOSSIL_ID)
                .map(|start| (chunk, start))
        })
        .expect("custom Nether fossil must retain the registered set placement");

    let fortress_data =
        NoiseStructureChunkData::from_supplied_starts(fortress.0[0], fortress.0[1], [], [])
            .with_fortress_starts([fortress.1.clone()]);
    assert_eq!(
        program
            .restored_structure_chunk_data(&fortress_data.to_nbt(), fortress.0[0], fortress.0[1],)
            .unwrap(),
        fortress_data
    );
    let fossil_data =
        NoiseStructureChunkData::from_supplied_starts(fossil.0[0], fossil.0[1], [], [])
            .with_nether_fossil_starts([fossil.1.clone()]);
    assert_eq!(
        program
            .restored_structure_chunk_data(&fossil_data.to_nbt(), fossil.0[0], fossil.0[1])
            .unwrap(),
        fossil_data
    );

    for (chunk, expected_id) in [
        ((fortress.0[0], fortress.0[1]), CUSTOM_FORTRESS_ID),
        ((fossil.0[0], fossil.0[1]), CUSTOM_FOSSIL_ID),
    ] {
        let mut generated = custom_store(Arc::clone(&program));
        generated
            .try_ensure_noise_full_column(chunk.0, chunk.1)
            .unwrap();
        let expected_structures = generated
            .structure_component(chunk.0, chunk.1)
            .unwrap()
            .clone();
        assert!(
            expected_structures
                .fortress_starts()
                .iter()
                .any(|start| start.structure_id() == expected_id)
                || expected_structures
                    .nether_fossil_starts()
                    .iter()
                    .any(|start| start.structure_id() == expected_id)
        );
        let expected_packet = generated
            .noise_full_chunk_packet_data(chunk.0, chunk.1)
            .unwrap()
            .is_some();
        let parsed = reopen(
            &full_document_for_program(&generated, &program, chunk),
            chunk,
        );
        let column = generated.column(chunk.0, chunk.1).unwrap().clone();
        let mut restored = custom_store(Arc::clone(&program));
        restored.insert_column(chunk.0, chunk.1, column.clone());
        restored.restore_saved_generation_metadata(&parsed).unwrap();
        assert_eq!(
            restored.structure_component(chunk.0, chunk.1),
            Some(&expected_structures)
        );
        restored
            .try_ensure_served_full_column(chunk.0, chunk.1)
            .unwrap();
        assert_eq!(restored.column(chunk.0, chunk.1), Some(&column));
        assert_eq!(
            restored
                .noise_full_chunk_packet_data(chunk.0, chunk.1)
                .unwrap()
                .is_some(),
            expected_packet
        );
    }
}
