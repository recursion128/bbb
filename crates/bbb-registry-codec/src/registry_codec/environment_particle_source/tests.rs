use super::*;
use crate::{
    registry_codec::recursive_values::RegistryCodecDecodeLimits, RegistryElementSource,
    RegistryJsonValue,
};
use bbb_protocol::NbtSummaryEntry;

fn string(value: &str) -> RegistryJsonValue {
    RegistryJsonValue::String(value.to_string().into())
}
fn number(value: &str) -> RegistryJsonValue {
    RegistryJsonValue::Number(value.to_string())
}
fn object(values: Vec<(&str, RegistryJsonValue)>) -> RegistryJsonValue {
    RegistryJsonValue::Object(
        values
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    )
}
fn decode(value: &RegistryJsonValue) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:test",
        "ParticleTypes.CODEC",
        RegistryElementSource::ServerDatapackJson,
    );
    decode_particle_options_source(
        RegistryElementValueRef::Json(value),
        "particle",
        &mut context,
    )
}
fn names(value: &CanonicalRegistryCodecValue) -> Vec<&bbb_protocol::JavaString> {
    match value.network_nbt_unchecked() {
        NbtSummaryValue::Compound(entries) => entries.iter().map(|entry| &entry.name).collect(),
        _ => panic!("not a compound"),
    }
}

#[test]
fn simple_dispatch_normalizes_and_writes_type_last() {
    let value = decode(&object(vec![("type", string("heart"))])).unwrap();
    assert_eq!(names(&value), ["type"]);
    assert!(decode(&object(vec![("type", string("example:missing"))])).is_err());
}

#[test]
fn defaults_colors_ranges_and_all_scalar_families_are_canonical() {
    let power = decode(&object(vec![("type", string("dragon_breath"))])).unwrap();
    assert_eq!(names(&power), ["type"]);
    assert!(decode(&object(vec![
        ("power", string("bad")),
        ("type", string("dragon_breath")),
    ]))
    .is_err());
    let spell = decode(&object(vec![
        (
            "color",
            RegistryJsonValue::Array(vec![number("1"), number("0"), number("0")]),
        ),
        ("type", string("effect")),
    ]))
    .unwrap();
    assert_eq!(names(&spell), ["color", "type"]);
    let dust = decode(&object(vec![
        (
            "color",
            RegistryJsonValue::Array(vec![number("1"), number("0.5"), number("0")]),
        ),
        ("scale", number("0.01")),
        ("type", string("dust")),
    ]))
    .unwrap();
    assert_eq!(names(&dust), ["color", "scale", "type"]);
    let transition = decode(&object(vec![
        ("from_color", number("1")),
        ("to_color", number("2")),
        ("scale", number("4")),
        ("type", string("dust_color_transition")),
    ]))
    .unwrap();
    assert_eq!(
        names(&transition),
        ["from_color", "to_color", "scale", "type"]
    );
    let argb = decode(&object(vec![
        (
            "color",
            RegistryJsonValue::Array(vec![number("1"), number("0"), number("0"), number("0.5")]),
        ),
        ("type", string("flash")),
    ]))
    .unwrap();
    let NbtSummaryValue::Compound(entries) = argb.network_nbt_unchecked() else {
        panic!()
    };
    assert_eq!(entries[0].value, NbtSummaryValue::Int(0x7fff0000));
    for value in [
        object(vec![
            ("roll", number("-0.0")),
            ("type", string("sculk_charge")),
        ]),
        object(vec![("delay", number("3")), ("type", string("shriek"))]),
        object(vec![("color", number("-1")), ("type", string("flash"))]),
    ] {
        assert_eq!(
            names(&decode(&value).unwrap()).last(),
            Some(&&"type".into())
        );
    }
    assert!(decode(&object(vec![
        ("color", number("1")),
        ("scale", number("0")),
        ("type", string("dust"))
    ]))
    .is_err());
}

#[test]
fn block_particles_delegate_to_the_shared_block_state_codec_writer() {
    let propertyless_state = decode(&object(vec![
        ("block_state", object(vec![("Name", string("stone"))])),
        ("type", string("block")),
    ]))
    .unwrap();
    let NbtSummaryValue::Compound(root) = propertyless_state.network_nbt_unchecked() else {
        panic!()
    };
    let NbtSummaryValue::Compound(state) = &root[0].value else {
        panic!()
    };
    assert_eq!(state.last().unwrap().name, "Name");
    let mapped = decode(&object(vec![
        (
            "block_state",
            object(vec![
                ("Properties", object(vec![("lit", string("true"))])),
                ("Name", string("redstone_torch")),
            ]),
        ),
        ("type", string("falling_dust")),
    ]))
    .unwrap();
    let NbtSummaryValue::Compound(root) = mapped.network_nbt_unchecked() else {
        panic!()
    };
    let NbtSummaryValue::Compound(state) = &root[0].value else {
        panic!()
    };
    assert_eq!(
        state.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
        ["Properties", "Name"]
    );

    let json_particle = object(vec![
        ("block_state", string("stone")),
        ("type", string("block")),
    ]);
    for source in [
        RegistryElementSource::ServerDatapackJson,
        RegistryElementSource::ClientKnownPackJson,
    ] {
        let mut context =
            RegistryCodecDecodeContext::new("minecraft:test", "ParticleTypes.CODEC", source);
        assert!(decode_particle_options_source(
            RegistryElementValueRef::Json(&json_particle),
            "particle",
            &mut context,
        )
        .is_err());
    }

    let nbt_particle = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "block_state".into(),
            value: NbtSummaryValue::String("stone".to_string().into()),
        },
        NbtSummaryEntry {
            name: "type".into(),
            value: NbtSummaryValue::String("block".to_string().into()),
        },
    ]);
    let mut context = RegistryCodecDecodeContext::new(
        "minecraft:test",
        "ParticleTypes.CODEC",
        RegistryElementSource::NetworkNbt,
    );
    assert!(decode_particle_options_source(
        RegistryElementValueRef::Nbt(&nbt_particle),
        "particle",
        &mut context,
    )
    .is_err());
}

#[test]
fn item_keeps_patch_references_and_shared_node_budget() {
    assert!(decode(&object(vec![
        ("item", object(vec![])),
        ("type", string("item")),
    ]))
    .is_err());

    let item = object(vec![
        (
            "item",
            object(vec![
                ("id", string("stone")),
                (
                    "components",
                    object(vec![("instrument", string("example:horn"))]),
                ),
            ]),
        ),
        ("type", string("item")),
    ]);
    let limits = RegistryCodecDecodeLimits {
        max_depth: 3,
        max_nodes: 6,
        max_collection: 2,
    };
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:test",
        "ParticleTypes.CODEC",
        RegistryElementSource::ServerDatapackJson,
        limits,
    )
    .unwrap();
    let decoded = decode_particle_options_source(
        RegistryElementValueRef::Json(&item),
        "particle",
        &mut context,
    )
    .unwrap();
    let references = decoded.references().collect::<Vec<_>>();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].target_registry, "minecraft:instrument");
    assert_eq!(references[0].resource_id.as_str(), "example:horn");

    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:test",
        "ParticleTypes.CODEC",
        RegistryElementSource::ServerDatapackJson,
        RegistryCodecDecodeLimits {
            max_nodes: 5,
            ..limits
        },
    )
    .unwrap();
    assert!(decode_particle_options_source(
        RegistryElementValueRef::Json(&item),
        "particle",
        &mut context,
    )
    .is_err());
}
