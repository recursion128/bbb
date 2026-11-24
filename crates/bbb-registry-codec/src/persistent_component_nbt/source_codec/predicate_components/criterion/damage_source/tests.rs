use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::{
            CanonicalRegistryCodecValue, OwnedRegistryCodecReference, RegistryCodecDecodeContext,
        },
        validate_registry_codec_references, RegistryCodecError, RegistryElementData,
        RegistryElementSource, RegistryElementValueRef, RegistryReferenceKind,
        RegistryReferenceLookup, RegistryResourceIdentifier,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};

use super::*;

const OWNER: &str = "minecraft:advancement";
const CODEC: &str = "DamageSourcePredicate.CODEC";
const STUB_REGISTRY: &str = "minecraft:entity_type";

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

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn decode_entity_stub(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "stub entity predicate")?;
    let Some(marker) = input.field("marker") else {
        return Ok(RegistryCodecRecord::default().finish());
    };
    let marker_field = child(field, "marker");
    let marker = context.with_nested(&marker_field, |context| {
        decode_identifier_value(marker, &marker_field, context)
    })?;
    let resource_id = match marker.network_nbt_unchecked() {
        NbtSummaryValue::String(value) => value
            .with_utf8(RegistryResourceIdentifier::parse)
            .flatten()
            .unwrap(),
        _ => unreachable!("identifier decoder emits a string"),
    };
    let mut output = RegistryCodecRecord::default();
    output.push("marker", marker);
    let decoded = output.finish();
    let (network_nbt, _) = decoded.into_parts();
    let mut canonical = CanonicalRegistryCodecValue::new(network_nbt);
    canonical.push_reference(OwnedRegistryCodecReference::element(
        STUB_REGISTRY,
        resource_id,
    ));
    Ok(canonical)
}

fn decode_data(
    data: &RegistryElementData,
) -> Result<(CanonicalRegistryCodecValue, Vec<String>), RegistryCodecError> {
    let mut calls = Vec::new();
    let decoded = decode(
        data.root(),
        "$",
        &mut context(data.source),
        |input, field, context| {
            calls.push(field.to_string());
            decode_entity_stub(input, field, context)
        },
    )?;
    Ok((decoded, calls))
}

#[test]
fn criterion_damage_source_predicate_writes_exact_tree_for_all_three_sources() {
    let json = br#"{
        "is_direct":false,
        "source_entity":{"marker":"zombie"},
        "tags":[{"expected":false,"id":"bypasses_armor"},{"id":"is_fire","expected":true}],
        "direct_entity":{"marker":"arrow"}
    }"#;
    let expected = compound(vec![
        entry(
            "tags",
            NbtSummaryValue::List(vec![
                compound(vec![
                    entry("id", string("minecraft:bypasses_armor")),
                    entry("expected", NbtSummaryValue::Byte(0)),
                ]),
                compound(vec![
                    entry("id", string("minecraft:is_fire")),
                    entry("expected", NbtSummaryValue::Byte(1)),
                ]),
            ]),
        ),
        entry(
            "direct_entity",
            compound(vec![entry("marker", string("minecraft:arrow"))]),
        ),
        entry(
            "source_entity",
            compound(vec![entry("marker", string("minecraft:zombie"))]),
        ),
        entry("is_direct", NbtSummaryValue::Byte(0)),
    ]);

    for data in [
        decode_direct_registry_json(json).unwrap(),
        decode_client_known_pack_registry_json(json).unwrap(),
        network(&expected),
    ] {
        let (decoded, calls) = decode_data(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &expected);
        assert_eq!(calls, ["$.direct_entity", "$.source_entity"]);
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].target_registry, STUB_REGISTRY);
        assert_eq!(references[0].kind, RegistryReferenceKind::Element);
        assert_eq!(references[0].resource_id.as_str(), "minecraft:arrow");
        assert_eq!(references[1].resource_id.as_str(), "minecraft:zombie");
    }
}

#[test]
fn criterion_damage_source_predicate_omits_default_tags_but_keeps_explicit_entities_and_false() {
    for json in [br#"{}"#.as_slice(), br#"{"tags":[]}"#.as_slice()] {
        let data = decode_direct_registry_json(json).unwrap();
        let (decoded, calls) = decode_data(&data).unwrap();
        assert_eq!(decoded.network_nbt_unchecked(), &compound(Vec::new()));
        assert!(calls.is_empty());
    }

    let data = decode_direct_registry_json(
        br#"{"direct_entity":{},"source_entity":{},"is_direct":false}"#,
    )
    .unwrap();
    let (decoded, calls) = decode_data(&data).unwrap();
    assert_eq!(
        decoded.network_nbt_unchecked(),
        &compound(vec![
            entry("direct_entity", compound(Vec::new())),
            entry("source_entity", compound(Vec::new())),
            entry("is_direct", NbtSummaryValue::Byte(0)),
        ])
    );
    assert_eq!(calls, ["$.direct_entity", "$.source_entity"]);
}

struct RejectAll;

impl RegistryReferenceLookup for RejectAll {
    fn contains_element(&self, _: &str, _: &str) -> bool {
        false
    }

    fn contains_tag(&self, _: &str, _: &str) -> bool {
        false
    }
}

#[test]
fn criterion_damage_source_predicate_tag_keys_are_plain_ids_without_frozen_references() {
    let data = decode_direct_registry_json(
        br#"{"tags":[{"id":"is_projectile","expected":true},{"id":"custom:no_magic","expected":false}]}"#,
    )
    .unwrap();
    let (decoded, _) = decode_data(&data).unwrap();
    assert_eq!(decoded.references().count(), 0);
    assert_eq!(
        decoded.network_nbt_unchecked(),
        &compound(vec![entry(
            "tags",
            NbtSummaryValue::List(vec![
                compound(vec![
                    entry("id", string("minecraft:is_projectile")),
                    entry("expected", NbtSummaryValue::Byte(1)),
                ]),
                compound(vec![
                    entry("id", string("custom:no_magic")),
                    entry("expected", NbtSummaryValue::Byte(0)),
                ]),
            ]),
        )])
    );
    validate_registry_codec_references(OWNER, CODEC, decoded.references(), &RejectAll).unwrap();
}
