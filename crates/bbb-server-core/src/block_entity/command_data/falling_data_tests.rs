use super::*;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn falling_block_data_overlay_replaces_nested_root_and_preserves_unsupplied_payload() {
    let pos = bbb_protocol::packets::BlockPos { x: 1, y: 64, z: 2 };
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_COMPARATOR_ID,
    )
    .unwrap();
    let mut entity = super::super::new_block_entity(pos, state).unwrap();
    entity
        .load_command_data_in_place_with_resolver(
            state,
            &[
                entry("OutputSignal", NbtSummaryValue::Int(13)),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:custom_data",
                        NbtSummaryValue::Compound(vec![entry("old", NbtSummaryValue::Int(1))]),
                    )]),
                ),
            ],
            crate::block_entity::test_item_resolver(),
        )
        .unwrap();
    let replacement = vec![entry(
        "minecraft:custom_data",
        NbtSummaryValue::Compound(vec![entry("new", NbtSummaryValue::Int(2))]),
    )];
    entity
        .merge_falling_block_data_with_resolver(
            &[
                entry("components", NbtSummaryValue::Compound(replacement.clone())),
                entry("id", NbtSummaryValue::String("minecraft:chest".into())),
                entry("x", NbtSummaryValue::Int(-90)),
            ],
            crate::block_entity::test_item_resolver(),
        )
        .unwrap();
    assert_eq!(entity.block_pos(), pos);
    assert_eq!(entity.block_state(), state);
    assert_eq!(entity.comparator().unwrap().output_signal(), 13);
    let NbtSummaryValue::Compound(saved) = entity.command_data().unwrap() else {
        unreachable!()
    };
    assert_eq!(
        unique_field(&saved, "components"),
        Some(&NbtSummaryValue::Compound(replacement))
    );
    assert_eq!(
        unique_field(&saved, "id"),
        Some(&NbtSummaryValue::String("minecraft:comparator".into()))
    );
}
