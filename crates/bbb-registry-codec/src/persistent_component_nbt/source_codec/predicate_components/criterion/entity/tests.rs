use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementData, RegistryElementSource, RegistryReferenceKind,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "EntityPredicate.CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

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

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_data(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    crate::persistent_component_nbt::source_codec::decode_criterion_entity_predicate_source(
        data.root(),
        "$",
        &mut context(data.source),
    )
}

#[test]
fn criterion_entity_root_three_sources_write_exact_full_field_order() {
    let json = br#"{
      "predicates":{"damage":{}},
      "components":{"custom_name":"hello"},
      "slots":{},"team":"blue","targeted_entity":{},"passenger":{},"vehicle":{},
      "periodic_tick":2,"type_specific":{"type":"raider","has_raid":true},
      "equipment":{},"flags":{},"nbt":"{}","effects":{},
      "movement_affected_by":{},"stepping_on":{},"location":{},
      "movement":{},"distance":{},"type":"zombie"
    }"#;
    let expected = compound(vec![
        entry("type", string("minecraft:zombie")),
        entry("distance", compound(Vec::new())),
        entry("movement", compound(Vec::new())),
        entry("location", compound(Vec::new())),
        entry("stepping_on", compound(Vec::new())),
        entry("movement_affected_by", compound(Vec::new())),
        entry("effects", compound(Vec::new())),
        entry("nbt", string("{}")),
        entry("flags", compound(Vec::new())),
        entry("equipment", compound(Vec::new())),
        entry(
            "type_specific",
            compound(vec![
                entry("has_raid", NbtSummaryValue::Byte(1)),
                entry("type", string("minecraft:raider")),
            ]),
        ),
        entry("periodic_tick", NbtSummaryValue::Int(2)),
        entry("vehicle", compound(Vec::new())),
        entry("passenger", compound(Vec::new())),
        entry("targeted_entity", compound(Vec::new())),
        entry("team", string("blue")),
        entry("slots", compound(Vec::new())),
        entry(
            "components",
            compound(vec![entry("minecraft:custom_name", string("hello"))]),
        ),
        entry(
            "predicates",
            compound(vec![entry("minecraft:damage", compound(Vec::new()))]),
        ),
    ]);

    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded = decode_data(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].target_registry, "minecraft:entity_type");
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "minecraft:zombie");
    }
}

#[test]
fn criterion_damage_source_facade_uses_the_real_recursive_entity_owner() {
    let json = br#"{
      "is_direct":true,
      "source_entity":{"vehicle":{"team":"mount"}},
      "direct_entity":{"type_specific":{"type":"sheep","sheared":false}},
      "tags":[]
    }"#;
    let expected = compound(vec![
        entry(
            "direct_entity",
            compound(vec![entry(
                "type_specific",
                compound(vec![
                    entry("sheared", NbtSummaryValue::Byte(0)),
                    entry("type", string("minecraft:sheep")),
                ]),
            )]),
        ),
        entry(
            "source_entity",
            compound(vec![entry(
                "vehicle",
                compound(vec![entry("team", string("mount"))]),
            )]),
        ),
        entry("is_direct", NbtSummaryValue::Byte(1)),
    ]);

    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let decoded = crate::persistent_component_nbt::source_codec::decode_criterion_damage_source_predicate_source(
            data.root(),
            "$",
            &mut context(data.source),
        )
        .unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
    }
}
