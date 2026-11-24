use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;

fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn packed(extra: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    let mut fields = vec![field("id", NbtSummaryValue::String("max_health".into()))];
    fields.extend(extra);
    NbtSummaryValue::Compound(fields)
}

fn modifier(id: &str, amount: NbtSummaryValue, operation: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        field("id", NbtSummaryValue::String(id.into())),
        field("amount", amount),
        field("operation", NbtSummaryValue::String(operation.into())),
    ])
}

#[test]
fn numeric_base_and_amount_accept_every_nbt_number() {
    for number in [
        NbtSummaryValue::Byte(-3),
        NbtSummaryValue::Short(-3),
        NbtSummaryValue::Int(-3),
        NbtSummaryValue::Long(-3),
        NbtSummaryValue::Float((-3.0_f32).to_bits()),
        NbtSummaryValue::Double((-3.0_f64).to_bits()),
    ] {
        let input = packed(vec![
            field("base", number.clone()),
            field(
                "modifiers",
                NbtSummaryValue::List(vec![modifier(":bonus", number, "add_value")]),
            ),
        ]);
        let decoded = PackedAttribute::read_saved_codec(&input);
        let value = decoded.unwrap();
        assert_eq!(value.attribute_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID);
        assert_eq!(value.base_value, -3.0);
        assert_eq!(value.modifiers[0].id, "minecraft:bonus");
        assert_eq!(value.modifiers[0].amount, -3.0);
    }
}
