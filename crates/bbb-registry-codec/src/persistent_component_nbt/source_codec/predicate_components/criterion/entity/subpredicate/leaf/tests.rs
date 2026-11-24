use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "EntitySubPredicate.CODEC";

type Decoder = fn(
    crate::RegistryElementValueRef<'_>,
    &str,
    &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_data(
    decoder: Decoder,
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decoder(data.root(), "$", &mut context(data.source))
}

#[test]
fn criterion_entity_subpredicate_leaf_writes_exact_trees_for_all_three_sources() {
    let cases: [(&[u8], NbtSummaryValue, &str, Decoder); 4] = [
        (
            br#"{"in_open_water":false}"#,
            compound(vec![entry("in_open_water", NbtSummaryValue::Byte(0))]),
            r#"{"in_open_water":false}"#,
            decode_fishing_hook,
        ),
        (
            br#"{"size":{"max":4,"min":2}}"#,
            compound(vec![entry(
                "size",
                compound(vec![
                    entry("min", NbtSummaryValue::Int(2)),
                    entry("max", NbtSummaryValue::Int(4)),
                ]),
            )]),
            r#"{"size":{"min":2,"max":4}}"#,
            decode_slime,
        ),
        (
            br#"{"is_captain":true,"has_raid":true}"#,
            compound(vec![
                entry("has_raid", NbtSummaryValue::Byte(1)),
                entry("is_captain", NbtSummaryValue::Byte(1)),
            ]),
            r#"{"has_raid":true,"is_captain":true}"#,
            decode_raider,
        ),
        (
            br#"{"sheared":false}"#,
            compound(vec![entry("sheared", NbtSummaryValue::Byte(0))]),
            r#"{"sheared":false}"#,
            decode_sheep,
        ),
    ];

    for (json, expected, _, decoder) in cases {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
            network(&expected),
        ] {
            let decoded = decode_data(decoder, &data).unwrap();
            assert_eq!(decoded.network_nbt_unchecked(), &expected);
            assert_eq!(decoded.references().count(), 0);
        }
    }
}

#[test]
fn criterion_entity_subpredicate_leaf_preserves_optional_false_and_omits_codec_defaults() {
    for decoder in [
        decode_fishing_hook as Decoder,
        decode_slime,
        decode_raider,
        decode_sheep,
    ] {
        let data = decode_direct_registry_json(br#"{}"#).unwrap();
        let decoded = decode_data(decoder, &data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &compound(Vec::new()));
    }

    for (decoder, json, field) in [
        (
            decode_fishing_hook as Decoder,
            br#"{"in_open_water":false}"#.as_slice(),
            "in_open_water",
        ),
        (
            decode_sheep as Decoder,
            br#"{"sheared":false}"#.as_slice(),
            "sheared",
        ),
    ] {
        let data = decode_direct_registry_json(json).unwrap();
        let decoded = decode_data(decoder, &data).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &compound(vec![entry(field, NbtSummaryValue::Byte(0))])
        );
    }

    for json in [
        br#"{"has_raid":false}"#.as_slice(),
        br#"{"is_captain":false}"#.as_slice(),
        br#"{"has_raid":false,"is_captain":false}"#.as_slice(),
    ] {
        let data = decode_direct_registry_json(json).unwrap();
        let decoded = decode_data(decode_raider, &data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &compound(Vec::new()));
    }

    let any_size = decode_direct_registry_json(br#"{"size":{}}"#).unwrap();
    let decoded = decode_data(decode_slime, &any_size).unwrap();
    assert_eq!(decoded.network_nbt_unchecked(), &compound(Vec::new()));
}
