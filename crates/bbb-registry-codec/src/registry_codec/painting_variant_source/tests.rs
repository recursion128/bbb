use super::*;
use crate::registry_codec::{
    decode_client_known_pack_registry_json, decode_direct_registry_json, RegistryElementSource,
};
use bbb_protocol::NbtSummaryEntry;

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn network_data(value: NbtSummaryValue) -> RegistryElementData {
    RegistryElementData::from_network_nbt(value)
}

fn basic_network_value() -> NbtSummaryValue {
    compound(vec![
        entry("width", NbtSummaryValue::Int(1)),
        entry("height", NbtSummaryValue::Int(16)),
        entry("asset_id", string("minecraft:art")),
    ])
}

fn names(value: &NbtSummaryValue) -> Vec<&bbb_protocol::JavaString> {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected compound")
    };
    entries.iter().map(|entry| &entry.name).collect()
}

#[test]
fn all_three_sources_round_trip_in_direct_codec_order_and_ignore_unknowns() {
    let json = br#"{
        "author":null,
        "asset_id":"art",
        "unknown":{"deep":[1,2,3]},
        "height":16,
        "title":null,
        "width":1
    }"#;
    let expected = basic_network_value();
    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network_data(expected.clone()),
    ] {
        let decoded = decode_painting_variant_element(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        assert_eq!(
            names(decoded.network_nbt_unchecked()),
            ["width", "height", "asset_id"]
        );
    }
    let round_trip = decode_painting_variant_element(&network_data(expected.clone())).unwrap();
    assert_eq!(round_trip.network_nbt_unchecked(), &expected);
    assert_eq!(
        synchronized_registry_element_codec(
            PAINTING_VARIANT_REGISTRY,
            RegistryElementSource::ClientKnownPackJson
        )
        .unwrap(),
        "PaintingVariant.DIRECT_CODEC"
    );
}
