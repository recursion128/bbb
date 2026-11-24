use super::*;
use crate::{
    registry_codec::{decode_block_state_id_nbt, RegistryElementSource},
    PersistentNbtDecodeLimits,
};

const FIELD: &str = "inBlockState";

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn state(name: &str, properties: Option<NbtSummaryValue>) -> NbtSummaryValue {
    let mut entries = vec![entry("Name", NbtSummaryValue::String(name.into()))];
    if let Some(properties) = properties {
        entries.push(entry("Properties", properties));
    }
    NbtSummaryValue::Compound(entries)
}

fn default_id(name: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(name).unwrap();
    vanilla_block_default_state_id(block.id).unwrap()
}

fn decode(raw: &NbtSummaryValue) -> Result<i32, RegistryCodecError> {
    decode_block_state_id_nbt(raw, FIELD, PersistentNbtDecodeLimits::DEFAULT)
}

fn properties(value: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry("lit", value)])
}

#[test]
fn native_state_ids_reuse_generated_defaults_and_lenient_property_owner() {
    let torch_default = default_id("minecraft:redstone_torch");
    let singleton_default = default_id("minecraft:stone");
    for raw in [
        state("redstone_torch", None),
        state(
            "redstone_torch",
            Some(NbtSummaryValue::String("wrong shape".into())),
        ),
        state("redstone_torch", Some(properties(NbtSummaryValue::Int(0)))),
        state(
            "redstone_torch",
            Some(properties(NbtSummaryValue::String("invalid".into()))),
        ),
        state(
            "redstone_torch",
            Some(NbtSummaryValue::Compound(vec![entry(
                "unknown",
                NbtSummaryValue::String("false".into()),
            )])),
        ),
    ] {
        assert_eq!(decode(&raw).unwrap(), torch_default);
    }
    for value in [
        NbtSummaryValue::String("ignored".into()),
        properties(NbtSummaryValue::String("false".into())),
    ] {
        assert_eq!(
            decode(&state("stone", Some(value))).unwrap(),
            singleton_default
        );
    }
    let raw = state(
        "redstone_torch",
        Some(NbtSummaryValue::Compound(vec![
            entry("unknown", NbtSummaryValue::Int(7)),
            entry("lit", NbtSummaryValue::String("false".into())),
        ])),
    );
    let id = decode(&raw).unwrap();
    assert_ne!(id, torch_default);
    let data = RegistryElementValueRef::Nbt(&raw);
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:block",
        "BlockState.CODEC",
        RegistryElementSource::NetworkNbt,
    );
    let canonical = decode_block_state_source(data, FIELD, &mut context).unwrap();
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:block",
        "BlockState.CODEC",
        RegistryElementSource::NetworkNbt,
    );
    assert_eq!(
        decode_block_state_id(
            RegistryElementValueRef::Nbt(canonical.network_nbt_unchecked()),
            FIELD,
            &mut context
        )
        .unwrap(),
        id
    );
    assert_eq!(
        vanilla_block_state_name(id),
        Some("minecraft:redstone_torch")
    );
    assert_eq!(
        vanilla_block_state_properties(id)
            .find(|(key, _)| *key == "lit")
            .map(|(_, value)| value),
        Some("false")
    );
}
