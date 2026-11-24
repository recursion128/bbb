use super::*;

const UUID: Uuid = Uuid::from_u128(0xe271);

fn parsed_input(input: NbtSummaryValue) -> Java26PlayerData {
    let mut document = canonical(UUID);
    replace(&mut document, "EnderItems", input.clone());
    let parsed = Java26PlayerData::parse(document.clone(), UUID).unwrap();
    assert_eq!(parsed.ender_items_input(), Some(&input));
    assert_eq!(parsed.raw(), &document);
    let roundtrip = Java26PlayerData::parse(parsed.raw().clone(), UUID).unwrap();
    assert_eq!(roundtrip.raw(), &document);
    assert_eq!(roundtrip.ender_items_input(), Some(&input));
    parsed
}

#[test]
fn ender_input_absent_is_handed_off_as_none_without_authoring_an_empty_field() {
    let mut document = canonical(UUID);
    document
        .entries_mut()
        .retain(|entry| entry.name != "EnderItems");
    let parsed = Java26PlayerData::parse(document.clone(), UUID).unwrap();
    assert!(parsed.ender_items_input().is_none());
    assert_eq!(parsed.raw(), &document);
    document
        .entries_mut()
        .retain(|entry| entry.name != "Inventory");
    let parsed = Java26PlayerData::parse(document.clone(), UUID).unwrap();
    assert!(parsed.inventory_input().is_none());
    assert_eq!(parsed.raw(), &document);
}

/// `ItemStack.CODEC`'s `count` is `orElse(1)` (`ItemStack.java:116`), so any count the
/// reader cannot use reads back as one.
#[test]
fn ender_input_unusable_count_reads_as_vanilla_default_one() {
    for count in [
        None,
        Some(NbtSummaryValue::String("bad".into())),
        Some(NbtSummaryValue::Int(0)),
        Some(NbtSummaryValue::Int(100)),
        Some(NbtSummaryValue::IntArray(vec![1, 2])),
    ] {
        let mut fields = vec![
            e("Slot", NbtSummaryValue::Byte(26)),
            e("id", NbtSummaryValue::String("minecraft:stone".into())),
        ];
        if let Some(count) = count {
            fields.push(e("count", count));
        }
        let input = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(fields)]);
        let parsed = parsed_input(input.clone());
        let Some(NbtSummaryValue::List(items)) = parsed.ender_items_input() else {
            panic!("ender list");
        };
        assert_eq!(saved_stack(&items[0]).count, 1);
        assert_eq!(parsed.ender_items_input(), Some(&input));
    }
}

#[test]
fn ender_input_created_and_retained_writer_output_remains_lazily_inspectable() {
    let mut patch = created_patch(UUID);
    let mut slots = vec![ItemStackSummary::empty(); 27];
    slots[0] = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 3,
        ..ItemStackSummary::empty()
    };
    slots[26] = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID),
        count: 1,
        ..ItemStackSummary::empty()
    };
    patch.created.ender_items = slots.into();
    let created = created_player_document(&patch).unwrap();
    let baseline = Java26PlayerData::parse(canonical(UUID), UUID).unwrap();
    let retained = baseline
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let mut inputs = Vec::new();
    for document in [created, retained] {
        let parsed = Java26PlayerData::parse(document.clone(), UUID).unwrap();
        assert_eq!(
            parsed.ender_items_input(),
            Some(&NbtSummaryValue::List(vec![
                item(0, "minecraft:stone", 3),
                item(26, "minecraft:dirt", 1)
            ]))
        );
        inputs.push(parsed.ender_items_input().unwrap().clone());
        assert_eq!(parsed.raw(), &document);
    }
    assert_eq!(inputs[0], inputs[1]);
}
