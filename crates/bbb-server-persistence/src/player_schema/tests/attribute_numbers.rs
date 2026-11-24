use super::*;

fn attribute(base: NbtSummaryValue, amount: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        e("id", NbtSummaryValue::String("minecraft:max_health".into())),
        e("base", base),
        e(
            "modifiers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                e("id", NbtSummaryValue::String("minecraft:test".into())),
                e("amount", amount),
                e("operation", NbtSummaryValue::String("add_value".into())),
            ])]),
        ),
    ])
}

#[test]
fn nonfinite_attribute_doubles_survive_created_and_retained_player_documents() {
    let uuid = Uuid::from_u128(0xa77);
    let values = [
        0x7ff8_0000_0000_0001,
        0xfff8_0000_0000_002a,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
    ];
    let retained = Java26PlayerData::parse(canonical(uuid), uuid).unwrap();
    for base in values {
        for amount in values {
            let mut patch = created_patch(uuid);
            patch.created.attributes = vec![attribute(
                NbtSummaryValue::Double(base),
                NbtSummaryValue::Double(amount),
            )];
            let expected = NbtSummaryValue::List(patch.created.attributes.clone());
            let created = created_player_document(&patch).unwrap();
            let saved = retained
                .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
                .unwrap();
            for document in [created, saved] {
                assert_eq!(field_of(&document, "attributes"), &expected);
                assert_eq!(
                    Java26PlayerData::parse(document, uuid)
                        .unwrap()
                        .attributes_input(),
                    Some(&expected),
                );
            }
        }
    }
}
