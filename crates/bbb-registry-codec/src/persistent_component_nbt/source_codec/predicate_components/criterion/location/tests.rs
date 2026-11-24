use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementData, RegistryElementSource, RegistryReferenceKind,
    },
};
use bbb_protocol::{
    encode_nbt_summary, nbt_summary::canonical_double_tag_bits, NbtSummaryEntry, NbtSummaryValue,
};

use super::*;

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "LocationPredicate.CODEC";

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(canonical_double_tag_bits(value))
}

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_data(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    crate::persistent_component_nbt::source_codec::decode_criterion_location_predicate_source(
        data.root(),
        "$",
        &mut context(data.source),
    )
}

#[test]
fn criterion_location_predicate_writes_exact_tree_for_all_three_sources() {
    let json = br##"{
        "can_see_sky":true,
        "fluid":{"state":{"level":{"max":"3","min":"0"}},"fluids":"water"},
        "block":{"components":{"custom_name":"hello"},"blocks":"stone"},
        "light":{"light":{"min":4}},
        "smokey":false,
        "dimension":"overworld",
        "structures":"#village",
        "biomes":["plains","desert"],
        "position":{"z":{"max":4.0,"min":-2.0},"y":{},"x":1.5}
    }"##;
    let expected = compound(vec![
        entry(
            "position",
            compound(vec![
                entry("x", double(1.5)),
                entry(
                    "z",
                    compound(vec![entry("min", double(-2.0)), entry("max", double(4.0))]),
                ),
            ]),
        ),
        entry(
            "biomes",
            NbtSummaryValue::List(vec![string("minecraft:plains"), string("minecraft:desert")]),
        ),
        entry("structures", string("#minecraft:village")),
        entry("dimension", string("minecraft:overworld")),
        entry("smokey", NbtSummaryValue::Byte(0)),
        entry(
            "light",
            compound(vec![entry(
                "light",
                compound(vec![entry("min", NbtSummaryValue::Int(4))]),
            )]),
        ),
        entry(
            "block",
            compound(vec![
                entry("blocks", string("minecraft:stone")),
                entry(
                    "components",
                    compound(vec![entry("minecraft:custom_name", string("hello"))]),
                ),
            ]),
        ),
        entry(
            "fluid",
            compound(vec![
                entry("fluids", string("minecraft:water")),
                entry(
                    "state",
                    compound(vec![entry(
                        "level",
                        compound(vec![entry("min", string("0")), entry("max", string("3"))]),
                    )]),
                ),
            ]),
        ),
        entry("can_see_sky", NbtSummaryValue::Byte(1)),
    ]);

    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded = decode_data(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 5);
        assert_eq!(references[0].target_registry, BIOME_REGISTRY);
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "minecraft:plains");
        assert_eq!(references[1].resource_id.as_str(), "minecraft:desert");
        assert_eq!(references[2].target_registry, STRUCTURE_REGISTRY);
        assert_eq!(references[2].kind, RegistryReferenceKind::Tag);
        assert_eq!(references[3].target_registry, "minecraft:block");
        assert_eq!(references[4].target_registry, FLUID_REGISTRY);
    }
}

#[test]
fn criterion_location_predicate_reuses_block_component_recursive_owner() {
    let data = decode_direct_registry_json(br#"{"block":{"components":{"custom_name":"hello"}}}"#)
        .unwrap();
    let decoded = decode_data(&data).unwrap();
    assert_eq!(
        decoded.network_nbt_unchecked(),
        &compound(vec![entry(
            "block",
            compound(vec![entry(
                "components",
                compound(vec![entry("minecraft:custom_name", string("hello"))]),
            )]),
        )])
    );
}
