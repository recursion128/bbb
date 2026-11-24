use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{recursive_values::RegistryCodecDecodeContext, RegistryElementData},
    RegistryElementSource,
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const OWNER: &str = "minecraft:enchantment";
const CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn network_data(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_state(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_block_state_source(data.root(), "state", &mut context(data.source)).unwrap()
}

fn decode_default(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_block_default_state_source(data.root(), "state", &mut context(data.source)).unwrap()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn redstone_torch_nbt(lit: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry(
            "Properties",
            NbtSummaryValue::Compound(vec![entry(
                "lit",
                NbtSummaryValue::String(lit.to_string().into()),
            )]),
        ),
        entry(
            "Name",
            NbtSummaryValue::String("minecraft:redstone_torch".to_string().into()),
        ),
    ])
}

#[test]
fn all_three_sources_write_canonical_state_definition_order() {
    let json = br#"{
        "ignored": true,
        "Name": "redstone_torch",
        "Properties": {"unknown": "kept-out", "lit": "false"}
    }"#;
    let network = redstone_torch_nbt("false");
    let cases = [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network_data(&network),
    ];
    for data in &cases {
        let decoded = decode_state(data);
        assert_eq!(decoded.network_nbt_unchecked(), &network);
        assert_eq!(decoded.references().count(), 0);
    }
}

#[test]
fn rotated_provider_shape_discards_valid_non_default_properties() {
    let data =
        decode_direct_registry_json(br#"{"Properties":{"lit":"false"},"Name":"redstone_torch"}"#)
            .unwrap();
    let ordinary = decode_state(&data);
    let default = decode_default(&data);
    assert_ne!(
        ordinary.network_nbt_unchecked(),
        default.network_nbt_unchecked()
    );

    let NbtSummaryValue::Compound(entries) = default.network_nbt_unchecked() else {
        panic!("default state is not a compound")
    };
    let NbtSummaryValue::Compound(properties) = &entries[0].value else {
        panic!("default state properties are not a compound")
    };
    assert_eq!(
        properties,
        &[entry(
            "lit",
            NbtSummaryValue::String("true".to_string().into())
        )]
    );
    assert_eq!(entries[1].name, "Name");
}
