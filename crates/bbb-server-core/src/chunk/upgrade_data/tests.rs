use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
    vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_ATTACHED_PUMPKIN_STEM_ID, VANILLA_BLOCK_PUMPKIN_ID};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::{
    generation_state_chunk_document, Java26ChunkContext, Java26ChunkData,
};
use bbb_server_types::chunk_data::ChunkGenerationState;

use super::{translate_upgrade_data, ChunkStore};

const CHUNK: (i32, i32) = (0, 0);

fn state(resource_id: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        resource_id,
        &properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
    )
    .unwrap()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn tick(id: &str, x: i32, y: i32, z: i32, delay: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("i", NbtSummaryValue::string(id)),
        entry("x", NbtSummaryValue::Int(x)),
        entry("y", NbtSummaryValue::Int(y)),
        entry("z", NbtSummaryValue::Int(z)),
        entry("t", NbtSummaryValue::Int(delay)),
        entry("p", NbtSummaryValue::Int(0)),
    ])
}

fn parsed_upgrade_document(
    store: &ChunkStore,
    column: &bbb_server_types::chunk_data::ChunkColumn,
    local_x: i32,
    sides: i8,
) -> Java26ChunkData {
    let state = ChunkGenerationState::Carvers {
        postprocessing_positions: Vec::new(),
        carving_mask: Vec::new(),
    };
    let mut raw = generation_state_chunk_document(
        column,
        CHUNK.0,
        CHUNK.1,
        &|_| Some("minecraft:plains".to_string()),
        &state,
        None,
        &[],
    )
    .unwrap();
    let relative_section = 0_i32.checked_sub(column.min_y().div_euclid(16)).unwrap();
    let packed = local_x | 1 << 4;
    raw.entries_mut().push(entry(
        "UpgradeData",
        NbtSummaryValue::Compound(vec![
            entry("Sides", NbtSummaryValue::Byte(sides)),
            entry(
                "Indices",
                NbtSummaryValue::Compound(vec![entry(
                    &relative_section.to_string(),
                    NbtSummaryValue::IntArray(vec![packed]),
                )]),
            ),
            entry(
                "neighbor_block_ticks",
                NbtSummaryValue::List(vec![tick("minecraft:air", local_x, 0, 1, 7)]),
            ),
            entry(
                "neighbor_fluid_ticks",
                NbtSummaryValue::List(vec![tick("minecraft:empty", local_x, 0, 1, 3)]),
            ),
        ]),
    ));
    Java26ChunkData::parse(
        raw,
        Java26ChunkContext {
            expected_x: CHUNK.0,
            expected_z: CHUNK.1,
            min_section_y: column.min_y().div_euclid(16),
            section_count: store.section_count(),
        },
    )
    .unwrap()
}

fn restored_store(local_x: i32, sides: i8) -> (ChunkStore, Java26ChunkData) {
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    let mut column = store.generated_column();
    let stem = state("minecraft:pumpkin_stem", &[("age", "7")]);
    column.set_block_state(local_x, 0, 1, stem).unwrap();
    if local_x < 15 {
        column
            .set_block_state(
                local_x + 1,
                0,
                1,
                vanilla_block_default_state_id(VANILLA_BLOCK_PUMPKIN_ID).unwrap(),
            )
            .unwrap();
    }
    let parsed = parsed_upgrade_document(&store, &column, local_x, sides);
    store.insert_column(CHUNK.0, CHUNK.1, column);
    store.restore_saved_generation_metadata(&parsed).unwrap();
    // A restore owner supplies this provenance before a staged whole rewrite.
    store.loaded_mut(CHUNK).storage_backed_generation = true;
    (store, parsed)
}

#[test]
fn upgrade_data_restore_executes_relative_sections_ticks_and_clears_once() {
    let (mut store, parsed) = restored_store(1, 0);
    let translated = translate_upgrade_data(&store, CHUNK, parsed.upgrade_data().unwrap()).unwrap();
    assert_eq!(translated.inside[0].section_y, 0);
    assert_eq!(translated.sides, [false; 8]);

    store
        .require_legacy_generation_work_completed(CHUNK)
        .unwrap();
    assert_eq!(
        vanilla_block_state_block_type_id(store.get_block_state(1, 0, 1).unwrap()),
        Some(VANILLA_BLOCK_ATTACHED_PUMPKIN_STEM_ID)
    );
    assert!(store.pending_legacy_generation_work(0, 0).is_none());
    let ticks = store.column(0, 0).unwrap().block_ticks().pack_saved(0);
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].id, "minecraft:attached_pumpkin_stem");
    assert_eq!(ticks[0].delay, 7);
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_property(
            store.get_block_state(1, 0, 1).unwrap(),
            "facing"
        ),
        Some("east")
    );
    let saved = bbb_server_persistence::chunk_writer::full_chunk_document(
        store.column(0, 0).unwrap(),
        0,
        0,
        &|_| Some("minecraft:plains".to_owned()),
        store.min_y(),
    )
    .unwrap();
    let reopened = Java26ChunkData::parse(
        saved,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: store.min_y().div_euclid(16),
            section_count: store.section_count(),
        },
    )
    .unwrap();
    assert!(reopened.upgrade_data().is_none());
    assert_eq!(
        reopened.block_ticks()[0].id,
        "minecraft:attached_pumpkin_stem"
    );
    assert_eq!(reopened.block_ticks()[0].delay, 7);
    assert_eq!(reopened.fluid_ticks()[0].id, "minecraft:empty");
    assert_eq!(reopened.fluid_ticks()[0].delay, 3);

    store
        .require_legacy_generation_work_completed(CHUNK)
        .unwrap();
}

#[test]
fn upgrade_data_restore_maps_direction8_bits_and_skips_unloaded_neighbors() {
    let (mut store, parsed) = restored_store(0, -127);
    let translated = translate_upgrade_data(&store, CHUNK, parsed.upgrade_data().unwrap()).unwrap();
    assert_eq!(
        translated.sides,
        [true, false, false, false, false, false, false, true]
    );
    store
        .require_legacy_generation_work_completed(CHUNK)
        .unwrap();
    assert_eq!(
        vanilla_block_state_block_type_id(store.get_block_state(0, 0, 1).unwrap()),
        Some(VANILLA_BLOCK_ATTACHED_PUMPKIN_STEM_ID)
    );
    assert!(store.pending_legacy_generation_work(0, 0).is_none());
}

#[test]
fn upgraded_double_chests_swap_real_slots_and_preserve_them_when_saved() {
    use crate::block_entity::{new_block_entity, BlockEntity};
    use bbb_protocol::packets::{BlockPos, ItemStackSummary};
    use bbb_server_types::item_stack::RuntimeItemStack;

    let chest = state(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "true"),
        ],
    );
    let at = BlockPos { x: 8, y: 0, z: 1 };
    for (other_x, own_type, other_type) in [(9, "left", "right"), (7, "right", "left")] {
        let other = BlockPos { x: other_x, ..at };
        let mut store = ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            0,
        );
        let mut column = store.generated_column();
        column.set_block_state(at.x, at.y, at.z, chest).unwrap();
        column
            .set_block_state(other.x, other.y, other.z, chest)
            .unwrap();
        let parsed = parsed_upgrade_document(&store, &column, at.x, 0);
        store.insert_column(0, 0, column);
        store.restore_saved_generation_metadata(&parsed).unwrap();
        store.loaded_mut(CHUNK).storage_backed_generation = true;
        let mut own_items = vec![RuntimeItemStack::empty(); 27];
        own_items[0] = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 12,
            component_patch: Default::default(),
        });
        let mut other_items = own_items.clone();
        other_items[0].set_count(3);
        for (pos, items) in [(at, own_items.clone()), (other, other_items.clone())] {
            let mut entity = new_block_entity(pos, chest).unwrap();
            assert!(entity.chest_mut().unwrap().set_items(items));
            store.block_entities_mut().add_and_register(entity);
        }
        store
            .require_legacy_generation_work_completed(CHUNK)
            .unwrap();
        for (pos, kind, items) in [
            (at, own_type, &other_items),
            (other, other_type, &own_items),
        ] {
            let live = store.get_block_state(pos.x, pos.y, pos.z).unwrap();
            assert_eq!(
                bbb_protocol::block_state_types::vanilla_block_state_property(live, "type"),
                Some(kind)
            );
            let entity = store.block_entities().get(pos).unwrap();
            assert_eq!(&entity.chest().unwrap().items(), items);
            let record = entity.save_with_full_metadata().unwrap();
            let restored = BlockEntity::load_static(pos, live, &record)
                .unwrap()
                .unwrap();
            assert_eq!(&restored.chest().unwrap().items(), items);
        }
        let ticks = store.column(0, 0).unwrap();
        assert_eq!(ticks.block_ticks().pack_saved(0)[0].id, "minecraft:chest");
        let fluid = ticks.fluid_ticks().pack_saved(0);
        assert_eq!(fluid[0].id, "minecraft:water");
        assert_eq!(fluid[0].delay, 3);
        assert!(store.pending_legacy_generation_work(0, 0).is_none());
    }
}

#[test]
fn upgraded_leaves_schedule_the_real_tick_that_recomputes_log_distance() {
    use bbb_protocol::packets::BlockPos;
    let at = BlockPos { x: 8, y: 0, z: 1 };
    let leaf = state(
        "minecraft:oak_leaves",
        &[
            ("distance", "6"),
            ("persistent", "false"),
            ("waterlogged", "false"),
        ],
    );
    let log = state("minecraft:oak_log", &[("axis", "y")]);
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    let mut column = store.generated_column();
    column.set_block_state(at.x, at.y, at.z, leaf).unwrap();
    column.set_block_state(at.x + 1, at.y, at.z, log).unwrap();
    let parsed = parsed_upgrade_document(&store, &column, at.x, 0);
    store.insert_column(0, 0, column);
    store.restore_saved_generation_metadata(&parsed).unwrap();
    store.loaded_mut(CHUNK).storage_backed_generation = true;
    store
        .require_legacy_generation_work_completed(CHUNK)
        .unwrap();
    let scheduled = store.column(0, 0).unwrap().block_ticks().pack_saved(0);
    assert_eq!(scheduled[0].id, "minecraft:oak_leaves");
    assert_eq!(scheduled[0].delay, 1);
    let column = store.column(0, 0).unwrap();
    let mut raw = generation_state_chunk_document(
        column,
        CHUNK.0,
        CHUNK.1,
        &|_| Some("minecraft:plains".into()),
        &ChunkGenerationState::Full {
            surface_y: 0,
            postprocessing_positions: Vec::new(),
        },
        None,
        &[],
    )
    .unwrap();
    bbb_server_persistence::write_chunk_ticks(
        &mut raw,
        0,
        &scheduled,
        &column.fluid_ticks().pack_saved(0),
    );
    let saved = Java26ChunkData::parse(
        raw,
        Java26ChunkContext {
            expected_x: CHUNK.0,
            expected_z: CHUNK.1,
            min_section_y: column.min_y().div_euclid(16),
            section_count: store.section_count(),
        },
    )
    .unwrap();
    let candidate = crate::tests::restore_candidate(0, Some(saved), Vec::new());
    let mut world = crate::world::ServerWorld::restore(Default::default(), &candidate).unwrap();
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_property(
            world
                .block_state(
                    bbb_server_types::save::Java26BuiltinDimension::Overworld,
                    at
                )
                .unwrap(),
            "distance"
        ),
        Some("1")
    );
}
