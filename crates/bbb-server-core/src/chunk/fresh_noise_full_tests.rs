use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use super::noise_generator::overworld_program;
use super::ChunkStore;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
    Java26ChunkStatus, RegionCompression, RegionFileStore,
};
use bbb_server_types::chunk_data::ChunkGenerationState;
use bbb_server_types::save::WorldSaveLimits;

const CENTER: (i32, i32) = (0, 0);
static NEXT_REGION_ID: AtomicU64 = AtomicU64::new(1);

fn store() -> ChunkStore {
    ChunkStore::new_overworld_carvers_with_structure_generation(overworld_program(true, 0), false)
        .unwrap()
}

fn full_document(store: &ChunkStore, center: (i32, i32)) -> bbb_server_types::save::NamedCompound {
    let program = overworld_program(true, 0);
    generation_state_chunk_document_with_light(
        store.column(center.0, center.1).unwrap(),
        center.0,
        center.1,
        &|id| program.biome_name(id),
        store.generation_state(center.0, center.1).unwrap(),
        store
            .structure_component(center.0, center.1)
            .map(|s| s.to_nbt())
            .as_ref(),
        store.proto_entity_records(center.0, center.1),
        &store.saved_light_sections(center),
    )
    .unwrap()
}

fn region_reopen(
    document: &bbb_server_types::save::NamedCompound,
    center: (i32, i32),
) -> Java26ChunkData {
    let id = NEXT_REGION_ID.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("bbb-fresh-noise-full-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let region = root.join("r.0.0.mca");
    RegionFileStore::new(&region, &root, WorldSaveLimits::default())
        .unwrap()
        .write_chunk(center.0, center.1, document, RegionCompression::Zlib, 17)
        .unwrap();
    let reopened = RegionFileStore::new(&region, &root, WorldSaveLimits::default())
        .unwrap()
        .read_chunk(center.0, center.1)
        .unwrap()
        .unwrap();
    let parsed = Java26ChunkData::parse(
        reopened,
        Java26ChunkContext {
            expected_x: center.0,
            expected_z: center.1,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
    parsed
}

#[test]
fn fresh_center_reaches_full_and_survives_region_reopen_and_restore() {
    let mut generated = store();
    generated
        .try_generate_fresh_noise_full_column(CENTER.0, CENTER.1)
        .unwrap();
    let Some(ChunkGenerationState::NoiseFull(full)) =
        generated.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("fresh generation did not publish FULL")
    };
    assert!(full.light().is_light_correct());
    assert!(full.spawn().original_mobs_refused());
    assert!(generated
        .proto_entity_records(CENTER.0, CENTER.1)
        .is_empty());

    let parsed = region_reopen(&full_document(&generated, CENTER), CENTER);
    assert_eq!(parsed.status(), Java26ChunkStatus::Full);
    assert!(parsed.is_light_on());
    assert!(parsed.entities().is_none());
    assert!(parsed.carving_mask().is_none());

    let column = generated.column(CENTER.0, CENTER.1).unwrap().clone();
    let mut restored = store();
    restored.insert_column(CENTER.0, CENTER.1, column);
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    let Some(ChunkGenerationState::NoiseFull(restored_full)) =
        restored.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("reopened FULL did not restore typed generation state")
    };
    assert!(restored_full.light().is_light_correct());
    assert!(!restored
        .pending_initial_persistence()
        .any(|chunk| chunk == CENTER));

    restored
        .try_ensure_served_full_column(CENTER.0, CENTER.1)
        .unwrap();
    let packet = restored
        .noise_full_chunk_packet_data(CENTER.0, CENTER.1)
        .unwrap()
        .unwrap();
    assert_eq!(packet.heightmaps.len(), 3);
    assert!(!packet.light_data.sky_y_mask.is_empty());
}

#[test]
fn initial_player_batch_keeps_final_heightmaps_current() {
    let batch = [
        (0, 0),
        (-1, 0),
        (0, -1),
        (0, 1),
        (1, 0),
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
    ];
    let mut generated = ChunkStore::new_overworld_carvers(overworld_program(true, 0)).unwrap();
    generated.try_ensure_served_full_columns(&batch).unwrap();
    let surface_y = match generated.generation_state(0, 0).unwrap() {
        ChunkGenerationState::NoiseFull(full) => full
            .light()
            .initialized()
            .features()
            .final_heightmaps()
            .first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 0, 0)
            .unwrap(),
        state => panic!("expected FULL, found {state:?}"),
    };
    let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
    generated
        .set_live_block_state(0, surface_y, 0, stone)
        .unwrap()
        .unwrap();
    generated.flush_live_relights().unwrap();
    generated.try_ensure_served_full_columns(&batch).unwrap();
    for center in batch {
        full_document(&generated, center);
        generated
            .noise_full_chunk_packet_data(center.0, center.1)
            .unwrap()
            .unwrap();
    }
}

#[test]
fn noise_full_packet_projection_keeps_heightmaps_and_light() {
    let mut flat = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    assert_eq!(flat.sea_level(), -63);
    flat.try_ensure_served_full_column(CENTER.0, CENTER.1)
        .unwrap();
    assert!(matches!(
        flat.generation_state(CENTER.0, CENTER.1),
        Some(ChunkGenerationState::Full { .. })
    ));
    assert!(flat
        .noise_full_chunk_packet_data(CENTER.0, CENTER.1)
        .unwrap()
        .is_none());

    let mut generated = store();
    assert_eq!(generated.sea_level(), 63);
    generated
        .try_ensure_served_full_column(CENTER.0, CENTER.1)
        .unwrap();
    let packet = generated
        .noise_full_chunk_packet_data(CENTER.0, CENTER.1)
        .unwrap()
        .unwrap();
    assert_eq!(
        packet
            .heightmaps
            .iter()
            .map(|heightmap| heightmap.kind_id)
            .collect::<Vec<_>>(),
        vec![
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        ]
    );
    let Some(ChunkGenerationState::NoiseFull(full)) =
        generated.generation_state(CENTER.0, CENTER.1)
    else {
        panic!("fresh generation did not publish FULL")
    };
    let column = generated.column(CENTER.0, CENTER.1).unwrap();
    let world_height = column.section_count() * 16;
    for heightmap in &packet.heightmaps {
        assert_eq!(
            heightmap.data,
            full.light()
                .initialized()
                .features()
                .final_heightmaps()
                .packed_values(heightmap.kind_id, column.min_y(), world_height, "test")
                .unwrap()
        );
    }
    assert_eq!(
        packet.light_data,
        generated
            .light_storage
            .packet_data(
                CENTER,
                column.min_y().div_euclid(16) - 1,
                column.section_count() + 2,
            )
            .unwrap()
    );

    let packet_before_registration = packet.clone();
    generated.register_ticks(CENTER, 0);
    assert_eq!(
        generated
            .noise_full_chunk_packet_data(CENTER.0, CENTER.1)
            .unwrap()
            .unwrap(),
        packet_before_registration
    );
}

#[test]
fn adjacent_noise_full_columns_execute_fallen_tree_decorators() {
    let mut generated = store();
    generated
        .try_ensure_served_full_columns(&[CENTER, (0, 1)])
        .unwrap();
    for chunk in [CENTER, (0, 1)] {
        assert!(matches!(
            generated.generation_state(chunk.0, chunk.1),
            Some(ChunkGenerationState::NoiseFull(_))
        ));
        assert!(generated
            .noise_full_chunk_packet_data(chunk.0, chunk.1)
            .unwrap()
            .is_some());
    }

    generated.try_ensure_served_full_columns(&[(0, 2)]).unwrap();
    assert!(matches!(
        generated.generation_state(0, 2),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
}

#[test]
fn overlapping_full_scheduler_preserves_saved_and_repeated_full_results() {
    let mut idempotent = store();
    idempotent.try_ensure_noise_full_column(0, 0).unwrap();

    let before = full_document(&idempotent, CENTER);
    idempotent.try_ensure_noise_full_column(0, 0).unwrap();
    assert_eq!(full_document(&idempotent, CENTER), before);

    let mut acknowledged = idempotent;
    let mut fresh_continuation = store();
    fresh_continuation
        .try_ensure_noise_full_column(0, 0)
        .unwrap();
    fresh_continuation
        .try_ensure_noise_full_columns(&[(0, 0), (0, 1)])
        .unwrap();
    acknowledged.mark_persisted(0, 0);
    acknowledged.try_ensure_noise_full_column(0, 0).unwrap();
    acknowledged
        .noise_full_chunk_packet_data(0, 0)
        .unwrap()
        .unwrap();
    acknowledged
        .try_ensure_noise_full_columns(&[(0, 0), (0, 1)])
        .unwrap();
    for chunk in [(0, 0), (0, 1)] {
        assert_eq!(
            acknowledged.column(chunk.0, chunk.1),
            fresh_continuation.column(chunk.0, chunk.1)
        );
        acknowledged
            .noise_full_chunk_packet_data(chunk.0, chunk.1)
            .unwrap()
            .unwrap();
    }

    let mut resumable = store();
    resumable
        .generate_fresh_initialize_light_window((0, 0))
        .unwrap();
    resumable.try_ensure_noise_full_column(0, 0).unwrap();
    assert!(matches!(
        resumable.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));

    let mut proto_entity = store();
    proto_entity
        .generate_fresh_initialize_light_window((0, 0))
        .unwrap();
    proto_entity.loaded_mut((0, 0)).proto_entities.extend(vec![
        bbb_server_worldgen::noise::proto_chunk_entity::StructureMobRecord::new(
            bbb_server_worldgen::noise::proto_chunk_entity::StructureMobKind::Witch,
            [0.5, 64.0, 0.5],
            [1, 2, 3, 4],
        )
        .unwrap()
        .entries(),
    ]);
    proto_entity.try_ensure_noise_full_column(0, 0).unwrap();
    assert!(proto_entity.proto_entity_records(0, 0).is_empty());
    assert!(proto_entity
        .published_worldgen_entity_records(0, 0)
        .iter()
        .flatten()
        .any(|entry| entry.name == "id"));
}

#[test]
fn persisted_carvers_window_advances_with_generation_rewrite_intents() {
    let mut persisted = store();
    for x in -2..=2 {
        for z in -2..=2 {
            persisted.try_load_column(x, z).unwrap();
            persisted.mark_persisted(x, z);
        }
    }

    persisted.try_ensure_noise_full_column(0, 0).unwrap();
    let rewritten = persisted
        .pending_whole_rewrites()
        .map(|intent| intent.chunk)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!rewritten.is_empty());
    assert!(rewritten.iter().any(|chunk| {
        !matches!(
            persisted.generation_state(chunk.0, chunk.1),
            Some(ChunkGenerationState::NoiseFull(_))
        )
    }));
    persisted
        .noise_full_chunk_packet_data(0, 0)
        .unwrap()
        .unwrap();
}

#[test]
fn persisted_intermediate_statuses_promote_with_new_rewrite_epochs() {
    let mut initialized = store();
    initialized
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();

    let mut features = store();
    features
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();
    let ChunkGenerationState::InitializeLight(initialized_center) = features.state(CENTER).clone()
    else {
        panic!("fresh center did not reach INITIALIZE_LIGHT")
    };
    features.set_generation(
        CENTER,
        ChunkGenerationState::Features(initialized_center.features),
    );
    features.mark_persisted(CENTER.0, CENTER.1);
    let earlier = features.mark_whole_rewrite_pending(CENTER).unwrap();

    let mut light = store();
    light
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();
    light.light_chunk(CENTER).unwrap();
    let mut spawn = store();
    spawn
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();
    spawn.light_chunk(CENTER).unwrap();
    spawn.spawn_chunk(CENTER).unwrap();

    for (label, mut candidate, earlier_epoch) in [
        ("features", features, Some(earlier.epoch)),
        ("initialize_light", initialized, None),
        ("light", light, None),
        ("spawn", spawn, None),
    ] {
        if earlier_epoch.is_none() {
            candidate.mark_persisted(CENTER.0, CENTER.1);
        }
        candidate
            .try_ensure_noise_full_column(CENTER.0, CENTER.1)
            .unwrap();
        assert!(
            matches!(
                candidate.generation_state(CENTER.0, CENTER.1),
                Some(ChunkGenerationState::NoiseFull(_))
            ),
            "{label} did not reach FULL"
        );
        let intent = candidate
            .pending_whole_rewrites()
            .find(|intent| intent.chunk == CENTER)
            .unwrap();
        if let Some(earlier_epoch) = earlier_epoch {
            assert!(intent.epoch > earlier_epoch);
        }
        assert!(candidate.acknowledge_whole_rewrite(intent));
        candidate
            .noise_full_chunk_packet_data(CENTER.0, CENTER.1)
            .unwrap()
            .unwrap();
    }
}

#[test]
fn overlapping_full_scheduler_relights_completed_centers() {
    let mut initialized = store();
    initialized
        .generate_fresh_initialize_light_window((0, 0))
        .unwrap();
    let mut light = store();
    light
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();
    light.light_chunk((0, 0)).unwrap();
    let mut spawn = store();
    spawn
        .generate_fresh_initialize_light_window(CENTER)
        .unwrap();
    spawn.light_chunk(CENTER).unwrap();
    spawn.spawn_chunk((0, 0)).unwrap();
    let mut full = store();
    full.generate_fresh_initialize_light_window(CENTER).unwrap();
    full.light_chunk(CENTER).unwrap();
    full.spawn_chunk(CENTER).unwrap();
    full.full_noise_chunk((0, 0)).unwrap();

    for (status, mut store) in [("LIGHT", light), ("SPAWN", spawn), ("FULL", full)] {
        let prior = store.generation_state(0, 0).unwrap().clone();
        store.try_ensure_noise_full_column(1, 0).unwrap();
        assert_eq!(
            std::mem::discriminant(store.generation_state(0, 0).unwrap()),
            std::mem::discriminant(&prior),
            "{status}"
        );
        assert!(matches!(
            store.generation_state(1, 0),
            Some(ChunkGenerationState::NoiseFull(_))
        ));
    }

    initialized.try_ensure_noise_full_column(1, 0).unwrap();
    initialized.try_ensure_noise_full_column(0, 0).unwrap();
    assert!(matches!(
        initialized.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
}

#[test]
fn overlapping_full_scheduler_passes_through_live_structure_owners() {
    let mut store = ChunkStore::new_overworld_carvers_with_structure_generation(
        overworld_program(true, 0),
        true,
    )
    .unwrap();
    store.try_ensure_noise_full_column(0, 0).unwrap();
    let first = full_document(&store, (0, 0));

    store.try_ensure_noise_full_column(4, 0).unwrap();
    assert_eq!(full_document(&store, (0, 0)), first);
}
