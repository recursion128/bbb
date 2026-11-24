use super::*;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn trim_with_modifier_amount(amount: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("asset_name", string("quartz")),
        entry(
            "description",
            NbtSummaryValue::Compound(vec![
                entry("text", string("x")),
                entry(
                    "hover_event",
                    NbtSummaryValue::Compound(vec![
                        entry("action", string("show_item")),
                        entry("id", string("stone")),
                        entry(
                            "components",
                            NbtSummaryValue::Compound(vec![entry(
                                "minecraft:attribute_modifiers",
                                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                                    entry("type", string("max_health")),
                                    entry("id", string("test")),
                                    entry("amount", amount),
                                    entry("operation", string("add_value")),
                                ])]),
                            )]),
                        ),
                    ]),
                ),
            ]),
        ),
    ])
}

fn begin_candidate(store: &mut WorldStore, resource_id: &str, amount: NbtSummaryValue) {
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(store, None);
    store.record_registry_entries(
        "minecraft:trim_material",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            resource_id,
            bbb_protocol::encode_nbt_summary(&trim_with_modifier_amount(amount), 1 << 20).unwrap(),
        )],
    );
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("expected compound containing {name}");
    };
    &fields
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn published_amount(store: &WorldStore) -> f64 {
    let material = store.synchronized_trim_material_presentation(0).unwrap();
    assert_eq!(material.asset_name, "quartz");
    assert_eq!(material.description, "x");
    let description = material.description_nbt.as_ref().unwrap();
    let hover = field(description, "hover_event");
    assert_eq!(field(hover, "action"), &string("show_item"));
    assert_eq!(field(hover, "id"), &string("minecraft:stone"));
    let modifiers = field(field(hover, "components"), "minecraft:attribute_modifiers");
    let NbtSummaryValue::List(modifiers) = modifiers else {
        panic!("attribute modifiers must retain their canonical list");
    };
    assert_eq!(modifiers.len(), 1);
    let modifier = &modifiers[0];
    assert_eq!(field(modifier, "type"), &string("minecraft:max_health"));
    assert_eq!(field(modifier, "id"), &string("minecraft:test"));
    assert_eq!(field(modifier, "operation"), &string("add_value"));
    let NbtSummaryValue::Double(bits) = field(modifier, "amount") else {
        panic!("modifier amount must publish as a DoubleTag");
    };
    f64::from_bits(*bits)
}

fn assert_network_amount_publishes(resource_id: &str, amount: f64) {
    let mut store = WorldStore::new();
    begin_candidate(
        &mut store,
        "example:active",
        NbtSummaryValue::Double(1.25_f64.to_bits()),
    );
    store.finish_registry_configuration().unwrap();
    assert_eq!(published_amount(&store), 1.25);
    let active_revision = store.registry_access_revision();

    begin_candidate(
        &mut store,
        resource_id,
        NbtSummaryValue::Double(amount.to_bits()),
    );
    store.finish_registry_configuration().unwrap();
    assert_ne!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:trim_material", 0),
        Some(resource_id)
    );
    let actual = published_amount(&store);
    if amount.is_nan() {
        assert!(actual.is_nan());
    } else {
        let expected = if amount == 0.0 { 0.0 } else { amount };
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn component_numeric_network_nan_publishes() {
    assert_network_amount_publishes("example:nan", f64::NAN);
}

#[test]
fn component_numeric_network_positive_infinity_publishes() {
    assert_network_amount_publishes("example:positive_infinity", f64::INFINITY);
}

#[test]
fn component_numeric_network_negative_infinity_publishes() {
    assert_network_amount_publishes("example:negative_infinity", f64::NEG_INFINITY);
}

#[test]
fn component_numeric_network_signed_zero_and_finite_publishes() {
    for (resource_id, amount) in [
        ("example:negative_zero", -0.0),
        ("example:positive_zero", 0.0),
        ("example:finite", -2.5),
    ] {
        assert_network_amount_publishes(resource_id, amount);
    }
}
