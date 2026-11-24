use std::collections::BTreeSet;

use bbb_protocol::item_types::VANILLA_ITEM_TYPES;

use super::ITEM_USE_ROWS;

/// The table is keyed by the generated catalog's `use_override` column, so
/// every row must name a class at least one catalog item carries -- a row for
/// a class no item reaches is dead -- and no class may own two rows, because
/// the first match would silently win.
#[test]
fn the_item_use_table_names_unique_generated_use_override_classes() {
    let generated: BTreeSet<String> = VANILLA_ITEM_TYPES
        .iter()
        .filter_map(|entry| entry.use_override.map(|class| format!("{class:?}")))
        .collect();
    let mut declared = BTreeSet::new();
    for row in ITEM_USE_ROWS {
        let class = format!("{:?}", row.class);
        assert!(
            generated.contains(&class),
            "{class} is the use_override of no generated item"
        );
        assert!(declared.insert(class.clone()), "{class} owns two rows");
    }
    assert_eq!(declared.len(), ITEM_USE_ROWS.len());
}
