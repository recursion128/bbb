use super::*;
use crate::registry_codec::{
    decode_direct_registry_json, RegistryElementSource, RegistryJsonValue, RegistryReferenceKind,
};
use bbb_protocol::NbtSummaryEntry;

fn string(value: &str) -> RegistryJsonValue {
    RegistryJsonValue::String(value.to_string().into())
}

fn number(value: &str) -> RegistryJsonValue {
    RegistryJsonValue::Number(value.to_string())
}

fn list(values: Vec<RegistryJsonValue>) -> RegistryJsonValue {
    RegistryJsonValue::Array(values)
}

fn object(values: Vec<(&str, RegistryJsonValue)>) -> RegistryJsonValue {
    RegistryJsonValue::Object(
        values
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    )
}

fn json_data(value: RegistryJsonValue, source: RegistryElementSource) -> RegistryElementData {
    RegistryElementData::from_json_value(value, source)
}

fn decode(data: RegistryElementData) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_timeline_element(&data)
}

fn root_names(value: &CanonicalRegistryCodecValue) -> Vec<&bbb_protocol::JavaString> {
    let NbtSummaryValue::Compound(entries) = value.network_nbt_unchecked() else {
        panic!("timeline root is not a compound")
    };
    entries.iter().map(|entry| &entry.name).collect()
}

#[test]
fn network_writer_filters_direct_and_network_tracks_but_keeps_their_references() {
    let value = object(vec![
        ("clock", string("minecraft:overworld")),
        (
            "tracks",
            object(vec![
                (
                    "minecraft:visual/fog_color",
                    object(vec![(
                        "keyframes",
                        list(vec![object(vec![
                            ("ticks", number("0")),
                            ("value", string("#112233")),
                        ])]),
                    )]),
                ),
                (
                    "minecraft:gameplay/bed_rule",
                    object(vec![(
                        "keyframes",
                        list(vec![object(vec![
                            ("ticks", number("0")),
                            (
                                "value",
                                object(vec![
                                    ("can_sleep", string("when_dark")),
                                    ("can_set_spawn", string("always")),
                                ]),
                            ),
                        ])]),
                    )]),
                ),
            ]),
        ),
    ]);
    for source in [
        RegistryElementSource::ServerDatapackJson,
        RegistryElementSource::ClientKnownPackJson,
    ] {
        let decoded = decode(json_data(value.clone(), source)).unwrap();
        let NbtSummaryValue::Compound(entries) = decoded.network_nbt_unchecked() else {
            panic!()
        };
        let tracks = entries.iter().find(|entry| entry.name == "tracks").unwrap();
        let NbtSummaryValue::Compound(track_entries) = &tracks.value else {
            panic!()
        };
        assert_eq!(track_entries.len(), 1);
        assert_eq!(track_entries[0].name, "minecraft:visual/fog_color");
        let refs = decoded.references().collect::<Vec<_>>();
        assert!(refs.iter().any(|reference| {
            reference.target_registry == "minecraft:world_clock"
                && reference.kind == RegistryReferenceKind::Element
                && reference.resource_id.as_str() == "minecraft:overworld"
        }));
    }
}

#[test]
fn world_clock_reference_defaults_and_root_field_order_are_canonical() {
    let decoded = decode(
        decode_direct_registry_json(
            br#"{
          "time_markers":{"day":0},
          "clock":"minecraft:the_end",
          "tracks":{},
          "period_ticks":20
        }"#,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        root_names(&decoded),
        ["clock", "period_ticks", "time_markers"]
    );
    let refs = decoded.references().collect::<Vec<_>>();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target_registry, WORLD_CLOCK_REGISTRY);
    assert_eq!(refs[0].resource_id.as_str(), "minecraft:the_end");
}

#[test]
fn explicit_default_linear_easing_is_omitted_by_the_canonical_writer() {
    let decoded = decode(
        decode_direct_registry_json(
            br#"{
                "clock":"minecraft:overworld",
                "tracks":{
                    "minecraft:visual/fog_start_distance":{
                        "keyframes":[{"ticks":0,"value":0.0}],
                        "ease":"linear"
                    }
                }
            }"#,
        )
        .unwrap(),
    )
    .unwrap();
    let NbtSummaryValue::Compound(root) = decoded.network_nbt_unchecked() else {
        panic!()
    };
    let NbtSummaryValue::Compound(tracks) = &root
        .iter()
        .find(|entry| entry.name == "tracks")
        .unwrap()
        .value
    else {
        panic!()
    };
    let NbtSummaryValue::Compound(track) = &tracks[0].value else {
        panic!()
    };
    assert_eq!(
        track.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
        ["keyframes"]
    );
}

#[test]
fn time_markers_compact_full_and_period_validation_match_vanilla() {
    let compact = decode(decode_direct_registry_json(
        br#"{"clock":"minecraft:overworld","period_ticks":10,"time_markers":{"day":0,"night":{"ticks":3},"debug":{"ticks":4,"show_in_commands":true}}}"#,
    )
    .unwrap())
    .unwrap();
    let NbtSummaryValue::Compound(entries) = compact.network_nbt_unchecked() else {
        panic!()
    };
    let markers = entries
        .iter()
        .find(|entry| entry.name == "time_markers")
        .unwrap();
    let NbtSummaryValue::Compound(values) = &markers.value else {
        panic!()
    };
    assert_eq!(values[0].value, NbtSummaryValue::Int(0));
    assert_eq!(values[1].value, NbtSummaryValue::Int(3));
    assert_eq!(
        values[2].value,
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "ticks".into(),
                value: NbtSummaryValue::Int(4),
            },
            NbtSummaryEntry {
                name: "show_in_commands".into(),
                value: NbtSummaryValue::Byte(1),
            },
        ])
    );
    decode(
        decode_direct_registry_json(
            br#"{"clock":"minecraft:overworld","period_ticks":4,"time_markers":{"late":4}}"#,
        )
        .unwrap(),
    )
    .unwrap_err();
}

#[test]
fn direct_timeline_retains_server_tracks_and_keeps_network_projection_filtered() {
    let source = br#"{
        "clock":"overworld", "period_ticks":24000,
        "tracks":{"gameplay/can_pillager_patrol_spawn":{
            "modifier":"and", "keyframes":[{"ticks":0,"value":false},{"ticks":100,"value":true}]
        }}, "time_markers":{"roll_village_siege":18000}
    }"#;
    let NbtSummaryValue::Compound(direct) =
        canonical_direct_timeline(&decode_direct_registry_json(source).unwrap()).unwrap()
    else {
        panic!("timeline");
    };
    assert_eq!(
        direct[0],
        entry(
            "clock",
            NbtSummaryValue::String("minecraft:overworld".into())
        )
    );
    let tracks = direct.iter().find(|field| field.name == "tracks").unwrap();
    let NbtSummaryValue::Compound(tracks) = &tracks.value else {
        panic!("tracks");
    };
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].name,
        "minecraft:gameplay/can_pillager_patrol_spawn"
    );
    let NbtSummaryValue::Compound(track) = &tracks[0].value else {
        panic!("track");
    };
    assert!(track
        .iter()
        .any(|field| field == &entry("modifier", NbtSummaryValue::String("and".into()))));
    let network = decode(decode_direct_registry_json(source).unwrap()).unwrap();
    let NbtSummaryValue::Compound(network) = network.network_nbt_unchecked() else {
        panic!("network");
    };
    assert!(!network.iter().any(|field| field.name == "tracks"));
    assert_eq!(
        direct.iter().find(|field| field.name == "time_markers"),
        network.iter().find(|field| field.name == "time_markers")
    );
}
