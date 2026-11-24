//! `FuelValues` derived from `FuelValues.java` and the furnace block entities
//! that read it, over the built-in pack's own item tags.

use std::sync::Arc;

use bbb_protocol::item_types::{
    vanilla_item_type_for_resource_id, VANILLA_ITEM_BAMBOO_ID, VANILLA_ITEM_BLAZE_ROD_ID,
    VANILLA_ITEM_CHARCOAL_ID, VANILLA_ITEM_COAL_BLOCK_ID, VANILLA_ITEM_COAL_ID,
    VANILLA_ITEM_CRIMSON_PLANKS_ID, VANILLA_ITEM_DRIED_KELP_BLOCK_ID, VANILLA_ITEM_LAVA_BUCKET_ID,
    VANILLA_ITEM_OAK_BOAT_ID, VANILLA_ITEM_OAK_HANGING_SIGN_ID, VANILLA_ITEM_OAK_PLANKS_ID,
    VANILLA_ITEM_OAK_SIGN_ID, VANILLA_ITEM_OAK_SLAB_ID, VANILLA_ITEM_STICK_ID,
    VANILLA_ITEM_WHITE_CARPET_ID,
};
use bbb_protocol::packets::ItemStackSummary;

use crate::registry_data_loader::{load_dynamic_registries, DatapackLoadLimits};
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::tag_loader::load_tags_for_startup;
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

use super::ingredient::item_stack;
use super::*;

fn builtin_item_tags() -> ItemTagIndex {
    let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
    let pack = snapshot.vanilla_pack();
    let inputs = [PackOverlayInput {
        pack_id: "vanilla",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let generation = load_dynamic_registries(&view, &inputs, DatapackLoadLimits::default())
        .expect("builtin registries load");
    let loaded = load_tags_for_startup(&inputs, &generation, DatapackLoadLimits::default());
    ItemTagIndex::from_loaded(loaded.registry("minecraft:item").unwrap())
}

fn item(resource_id: &str) -> i32 {
    vanilla_item_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a registered item"))
        .id
}

fn position(values: &FuelValues, item_id: i32) -> usize {
    values
        .fuel_items()
        .position(|candidate| candidate == item_id)
        .expect("the item is fuel")
}

#[test]
fn the_vanilla_table_burns_each_row_for_the_duration_that_row_computes() {
    let tags = builtin_item_tags();
    let table = FuelValues::vanilla_burn_times(&tags);
    assert_eq!(table.refusals, Vec::new());
    assert_eq!(table.values.len(), 280);

    // The five direct rows `vanillaBurnTimes` opens with, in its own order.
    assert_eq!(
        table.values.fuel_items().take(5).collect::<Vec<_>>(),
        [
            VANILLA_ITEM_LAVA_BUCKET_ID,
            VANILLA_ITEM_COAL_BLOCK_ID,
            VANILLA_ITEM_BLAZE_ROD_ID,
            VANILLA_ITEM_COAL_ID,
            VANILLA_ITEM_CHARCOAL_ID,
        ]
    );

    for (item_id, ticks) in [
        (VANILLA_ITEM_LAVA_BUCKET_ID, 20_000),
        (VANILLA_ITEM_COAL_BLOCK_ID, 16_000),
        (VANILLA_ITEM_BLAZE_ROD_ID, 2_400),
        (VANILLA_ITEM_COAL_ID, 1_600),
        (VANILLA_ITEM_CHARCOAL_ID, 1_600),
        (VANILLA_ITEM_DRIED_KELP_BLOCK_ID, 4_001),
        (VANILLA_ITEM_OAK_BOAT_ID, 1_200),
        (VANILLA_ITEM_OAK_HANGING_SIGN_ID, 800),
        (VANILLA_ITEM_OAK_PLANKS_ID, 300),
        (VANILLA_ITEM_OAK_SIGN_ID, 200),
        (VANILLA_ITEM_OAK_SLAB_ID, 150),
        (VANILLA_ITEM_STICK_ID, 100),
        (VANILLA_ITEM_WHITE_CARPET_ID, 67),
        (VANILLA_ITEM_BAMBOO_ID, 50),
    ] {
        let stack = item_stack(item_id, 1);
        assert!(table.values.is_fuel(&stack));
        assert_eq!(table.values.burn_duration(&stack), ticks);
    }
}

#[test]
fn a_re_added_item_keeps_the_position_its_first_row_gave_it() {
    let tags = builtin_item_tags();
    let table = FuelValues::vanilla_burn_times(&tags);

    // `#saplings` carries the azaleas, so their own later rows only re-put them.
    let azalea = item("minecraft:azalea");
    assert_eq!(
        table.values.fuel_items().filter(|id| *id == azalea).count(),
        1
    );
    assert!(position(&table.values, azalea) < position(&table.values, item("minecraft:composter")));

    let mut synthetic = ItemTagIndex::new();
    synthetic.insert("test:first", [VANILLA_ITEM_COAL_ID]);
    let rows = [
        FuelRow::Tag("test:first", BurnTime::new(0, 1, 1)),
        FuelRow::Item("minecraft:stick", BurnTime::new(0, 2, 1)),
        FuelRow::Item("minecraft:coal", BurnTime::new(0, 3, 1)),
    ];
    let table = FuelValues::build(&rows, 100, &synthetic);
    assert_eq!(
        table.values.fuel_items().collect::<Vec<_>>(),
        [VANILLA_ITEM_COAL_ID, VANILLA_ITEM_STICK_ID]
    );
    assert_eq!(
        table
            .values
            .burn_duration(&item_stack(VANILLA_ITEM_COAL_ID, 1)),
        300
    );
}

#[test]
fn the_removal_row_runs_after_every_add() {
    let tags = builtin_item_tags();
    let table = FuelValues::vanilla_burn_times(&tags);

    // `#planks` and `#logs` both carry nether wood the last row takes back out.
    for resource_id in [
        "minecraft:crimson_planks",
        "minecraft:warped_planks",
        "minecraft:crimson_stem",
        "minecraft:warped_hyphae",
    ] {
        let stack = item_stack(item(resource_id), 1);
        assert!(!table.values.is_fuel(&stack));
        assert_eq!(table.values.burn_duration(&stack), 0);
    }
    assert!(tags.contains("minecraft:planks", VANILLA_ITEM_CRIMSON_PLANKS_ID));
}

#[test]
fn an_empty_stack_is_no_fuel_and_burns_nothing() {
    let tags = builtin_item_tags();
    let table = FuelValues::vanilla_burn_times(&tags);

    for stack in [
        ItemStackSummary::empty(),
        item_stack(VANILLA_ITEM_COAL_ID, 0),
    ] {
        assert!(!table.values.is_fuel(&stack));
        assert_eq!(table.values.burn_duration(&stack), 0);
        assert_eq!(
            table
                .values
                .burn_duration_for(FurnaceFuelKind::Furnace, &stack),
            0
        );
    }
}

#[test]
fn the_blast_furnace_and_smoker_halve_what_the_furnace_burns() {
    let tags = builtin_item_tags();
    let table = FuelValues::vanilla_burn_times(&tags);

    for (item_id, furnace) in [
        (VANILLA_ITEM_COAL_ID, 1_600),
        (VANILLA_ITEM_WHITE_CARPET_ID, 67),
    ] {
        let stack = item_stack(item_id, 1);
        assert_eq!(
            table
                .values
                .burn_duration_for(FurnaceFuelKind::Furnace, &stack),
            furnace
        );
        for kind in [FurnaceFuelKind::BlastFurnace, FurnaceFuelKind::Smoker] {
            assert_eq!(table.values.burn_duration_for(kind, &stack), furnace / 2);
        }
    }
}

#[test]
fn the_base_unit_is_the_only_number_the_rows_are_computed_from() {
    let tags = builtin_item_tags();
    let table = FuelValues::build(VANILLA_BURN_TIME_ROWS, 400, &tags);
    assert_eq!(table.refusals, Vec::new());
    assert_eq!(table.values.len(), 280);

    for (item_id, ticks) in [
        (VANILLA_ITEM_COAL_ID, 3_200),
        (VANILLA_ITEM_OAK_SLAB_ID, 300),
        (VANILLA_ITEM_WHITE_CARPET_ID, 134),
        (VANILLA_ITEM_DRIED_KELP_BLOCK_ID, 8_001),
    ] {
        assert_eq!(
            table.values.burn_duration(&item_stack(item_id, 1)),
            ticks,
            "{item_id}"
        );
    }
    assert_eq!(VANILLA_FUEL_BASE_UNIT, 200);
}

#[test]
fn fuel_values_with_content_registry_resolves_custom_mod_items_and_tags() {
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModItemRegistration};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "example:blaze_coal".to_string(),
        max_stack_size: 64,
        is_block_item: false,
    });
    builder.register_item(ModItemRegistration {
        resource_id: "example:charred_log".to_string(),
        max_stack_size: 64,
        is_block_item: true,
    });
    let content_registry = builder.freeze_for_session().unwrap();
    let coal_id = content_registry
        .item_by_resource_id("example:blaze_coal")
        .unwrap()
        .id;
    let log_id = content_registry
        .item_by_resource_id("example:charred_log")
        .unwrap()
        .id;

    let loaded_tags = crate::tag_loader::LoadedRegistryTags {
        registry_id: "minecraft:item".to_string(),
        tags: std::collections::BTreeMap::from([(
            "example:mod_logs".to_string(),
            vec!["example:charred_log".to_string()],
        )]),
    };
    let tags = ItemTagIndex::from_loaded_with_content_registry(&loaded_tags, &content_registry);
    let rows = [
        FuelRow::Item("example:blaze_coal", BurnTime::new(0, 12, 1)),
        FuelRow::Tag("example:mod_logs", BurnTime::new(0, 2, 1)),
    ];
    let table = FuelValues::build_with_content_registry(
        &rows,
        VANILLA_FUEL_BASE_UNIT,
        &tags,
        &content_registry,
    );
    assert!(table.refusals.is_empty());
    assert_eq!(table.values.burn_duration(&item_stack(coal_id, 1)), 2_400);
    assert_eq!(table.values.burn_duration(&item_stack(log_id, 1)), 400);
}
