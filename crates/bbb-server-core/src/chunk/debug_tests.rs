use crate::world_creation::default_overworld_level_source;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BARRIER_ID};
use bbb_server_datapack::debug_generator::DebugLevelSource;

use super::ChunkStore;
use bbb_server_types::chunk_data::{ChunkGenerationState, VANILLA_26_1_BIOME_REGISTRY_LEN};

const PLAINS_ID: i32 = 0;

fn debug_source() -> DebugLevelSource {
    DebugLevelSource::new(-64, 384).unwrap()
}

fn debug_store(has_skylight: bool) -> ChunkStore {
    ChunkStore::new_debug_with_biome_registry_and_skylight(
        debug_source(),
        PLAINS_ID,
        VANILLA_26_1_BIOME_REGISTRY_LEN,
        has_skylight,
    )
}

#[test]
fn debug_chunks_follow_the_registry_grid_across_chunk_boundaries() {
    let mut store = debug_store(true);
    let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID).unwrap();
    store
        .try_ensure_served_full_columns(&[(0, 0), (1, 0), (21, 14), (-1, -1)])
        .unwrap();

    assert_eq!(store.sea_level(), 63);
    assert_eq!(store.biome_id(), PLAINS_ID);
    assert!(store.debug_generator().is_some());
    assert!(store.flat_generator().is_none());
    assert!(!store.is_debug());
    assert_eq!(
        store.generated_column().get_block_state(0, 60, 0),
        Some(barrier)
    );
    assert_eq!(store.generated_column().get_block_state(1, 70, 1), Some(0));
    assert_eq!(store.get_block_state(3, 70, 1), Some(173));
    assert_eq!(store.get_block_state(15, 70, 1), Some(1_211));
    assert_eq!(store.get_block_state(16, 70, 1), Some(0));
    assert_eq!(store.get_block_state(17, 70, 1), Some(1_384));
    assert_eq!(store.get_block_state(345, 70, 233), Some(29_872));
    assert_eq!(store.get_block_state(344, 70, 233), Some(0));
    assert_eq!(store.get_block_state(-1, 70, -1), Some(0));
}

#[test]
fn fresh_debug_full_tracks_exact_preimages_and_initial_persistence() {
    let mut store = debug_store(true);
    store.try_ensure_served_full_column(0, 0).unwrap();
    store.try_ensure_served_full_column(1, 0).unwrap();

    assert!(store.is_ticking_full(0, 0));
    assert!(matches!(
        store.generation_state(0, 0),
        Some(ChunkGenerationState::Full { surface_y: 70, .. })
    ));
    assert_eq!(store.generated_block_state(3, 70, 1), Some(173));
    assert_eq!(store.generated_block_state(17, 70, 1), Some(1_384));
    assert_eq!(store.set_block_state(17, 70, 1, 42), Some(1_384));
    assert_eq!(store.get_block_state(17, 70, 1), Some(42));
    assert_eq!(store.generated_block_state(17, 70, 1), Some(1_384));

    let pending = store.pending_initial_persistence().collect::<Vec<_>>();
    assert_eq!(pending, [(0, 0), (1, 0)]);
    store.mark_persisted(0, 0);
    assert_eq!(
        store.pending_initial_persistence().collect::<Vec<_>>(),
        [(1, 0)]
    );
}

#[test]
fn debug_live_write_returns_previous_and_updates_underlying_state() {
    let mut store = debug_store(true);
    store.try_ensure_served_full_column(0, 0).unwrap();

    assert_eq!(store.get_block_state(3, 70, 1), Some(173));
    assert_eq!(store.set_live_block_state(3, 70, 1, 42).unwrap(), Some(173));
    assert_eq!(
        store.column(0, 0).unwrap().get_block_state(3, 70, 1),
        Some(42)
    );
    assert_eq!(store.get_block_state(3, 70, 1), Some(42));
}

#[test]
fn debug_full_surface_hint_stays_inside_the_source_shape() {
    for (source, expected) in [
        (DebugLevelSource::new(0, 64).unwrap(), 60),
        (DebugLevelSource::new(-64, 16).unwrap(), -64),
    ] {
        let mut store = ChunkStore::new_debug_with_biome_registry_and_skylight(
            source,
            PLAINS_ID,
            VANILLA_26_1_BIOME_REGISTRY_LEN,
            true,
        );
        store.try_load_column(0, 0).unwrap();
        assert!(matches!(
            store.generation_state(0, 0),
            Some(ChunkGenerationState::Full { surface_y, .. }) if *surface_y == expected
        ));
    }
}

#[test]
fn installed_debug_view_overrides_only_loaded_reads() {
    let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
    let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID).unwrap();
    let mut store = ChunkStore::new(default_overworld_level_source().unwrap(), PLAINS_ID);
    store.load_column(0, 0);
    let underlying = store.column(0, 0).unwrap().clone();
    let generated_at_70 = store.generated_block_state(3, 70, 1);

    store.install_debug_view(debug_source());

    assert!(store.is_debug());
    assert_eq!(store.get_block_state(3, 59, 1), Some(air));
    assert_eq!(store.get_block_state(3, 60, 1), Some(barrier));
    assert_eq!(store.get_block_state(3, 61, 1), Some(air));
    assert_eq!(store.get_block_state(3, 70, 1), Some(173));
    assert_eq!(store.get_block_state(3, 71, 1), Some(air));
    assert_eq!(store.get_block_state(3, 400, 1), Some(air));
    assert_eq!(store.get_block_state(16, 60, 1), None);
    assert_eq!(store.column(0, 0), Some(&underlying));
    assert_eq!(store.generated_block_state(3, 70, 1), generated_at_70);

    assert_eq!(store.set_block_state(3, 70, 1, 42), Some(air));
    assert_eq!(
        store.column(0, 0).unwrap().get_block_state(3, 70, 1),
        Some(42)
    );
    assert_eq!(store.get_block_state(3, 70, 1), Some(173));
}

#[test]
fn debug_direct_full_lighting_respects_display_blocks_and_dimension_skylight() {
    let mut skylit = debug_store(true);
    skylit.try_load_column(0, 0).unwrap();
    assert_eq!(skylit.block_light_level(3, 70, 1), 0);
    assert_eq!(skylit.sky_light_level(3, 70, 1), 0);
    assert_eq!(skylit.sky_light_level(3, 71, 1), 15);
    // BarrierBlock.java:40-42: a dry barrier passes skylight.
    assert_eq!(skylit.sky_light_level(2, 59, 1), 15);
    assert_eq!(skylit.sky_light_level(3, 59, 1), 14);

    let mut unskylit = debug_store(false);
    unskylit.try_load_column(0, 0).unwrap();
    assert_eq!(unskylit.block_light_level(3, 70, 1), 0);
    assert_eq!(unskylit.sky_light_level(3, 70, 1), 0);
}
