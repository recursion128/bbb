use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
        RegistryReferenceKind,
    },
};
use bbb_protocol::{encode_nbt_summary, nbt_summary::entry, NbtSummaryEntry};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}

fn decode(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_block_predicate_source(data.root(), "$", &mut context(data.source)).unwrap()
}

fn network_data(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn predicate(type_id: &str, mut fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    fields.push(entry(
        "type",
        NbtSummaryValue::String(format!("minecraft:{type_id}").into()),
    ));
    compound(fields)
}

fn offset(values: [i32; 3]) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(values.to_vec())
}

fn state(name: &str) -> NbtSummaryValue {
    compound(vec![entry(
        "Name",
        NbtSummaryValue::String(name.to_string().into()),
    )])
}

#[test]
fn all_thirteen_families_write_exact_payload_order_and_normalized_type_last() {
    let cases = vec![
        (
            br#"{"type":"matching_blocks","offset":[1,2,3],"blocks":"stone","ignored":true}"#
                .to_vec(),
            predicate(
                "matching_blocks",
                vec![
                    entry("offset", offset([1, 2, 3])),
                    entry(
                        "blocks",
                        NbtSummaryValue::String("minecraft:stone".to_string().into()),
                    ),
                ],
            ),
        ),
        (
            br#"{"type":"matching_block_tag","tag":"logs"}"#.to_vec(),
            predicate(
                "matching_block_tag",
                vec![entry(
                    "tag",
                    NbtSummaryValue::String("minecraft:logs".to_string().into()),
                )],
            ),
        ),
        (
            br#"{"type":"matching_fluids","offset":[-1,0,1],"fluids":["water","lava"]}"#.to_vec(),
            predicate(
                "matching_fluids",
                vec![
                    entry("offset", offset([-1, 0, 1])),
                    entry(
                        "fluids",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::String("minecraft:water".to_string().into()),
                            NbtSummaryValue::String("minecraft:lava".to_string().into()),
                        ]),
                    ),
                ],
            ),
        ),
        (
            br#"{"type":"has_sturdy_face","direction":"west"}"#.to_vec(),
            predicate(
                "has_sturdy_face",
                vec![entry(
                    "direction",
                    NbtSummaryValue::String("west".to_string().into()),
                )],
            ),
        ),
        (
            br#"{"type":"solid","offset":[0,0,0]}"#.to_vec(),
            predicate("solid", Vec::new()),
        ),
        (
            br#"{"type":"replaceable","offset":[0,1,0]}"#.to_vec(),
            predicate("replaceable", vec![entry("offset", offset([0, 1, 0]))]),
        ),
        (
            br#"{"type":"would_survive","offset":[0,-1,0],"state":{"Name":"stone"}}"#.to_vec(),
            predicate(
                "would_survive",
                vec![
                    entry("offset", offset([0, -1, 0])),
                    entry("state", state("minecraft:stone")),
                ],
            ),
        ),
        (
            br#"{"type":"inside_world_bounds","offset":[1,0,0]}"#.to_vec(),
            predicate(
                "inside_world_bounds",
                vec![entry("offset", offset([1, 0, 0]))],
            ),
        ),
        (
            br#"{"type":"any_of","predicates":[]}"#.to_vec(),
            predicate(
                "any_of",
                vec![entry("predicates", NbtSummaryValue::List(Vec::new()))],
            ),
        ),
        (
            br#"{"type":"all_of","predicates":[{"type":"true"}]}"#.to_vec(),
            predicate(
                "all_of",
                vec![entry(
                    "predicates",
                    NbtSummaryValue::List(vec![predicate("true", Vec::new())]),
                )],
            ),
        ),
        (
            br#"{"type":"not","predicate":{"type":"true"}}"#.to_vec(),
            predicate(
                "not",
                vec![entry("predicate", predicate("true", Vec::new()))],
            ),
        ),
        (
            br#"{"type":"true","ignored":1}"#.to_vec(),
            predicate("true", Vec::new()),
        ),
        (
            br#"{"type":"unobstructed","offset":[-2147483648,2147483647,16]}"#.to_vec(),
            predicate(
                "unobstructed",
                vec![entry("offset", offset([i32::MIN, i32::MAX, 16]))],
            ),
        ),
    ];

    for (json, expected) in cases {
        for data in [
            decode_direct_registry_json(&json).unwrap(),
            decode_client_known_pack_registry_json(&json).unwrap(),
            network_data(&expected),
        ] {
            let decoded = decode(&data);
            assert_eq!(decoded.network_nbt_unchecked(), &expected, "{json:?}");
        }
    }
}

#[test]
fn holder_sets_keep_scalar_list_and_tag_shapes_with_encounter_ordered_references() {
    let cases = [
        (
            br#"{"type":"matching_blocks","blocks":"stone"}"#.as_slice(),
            NbtSummaryValue::String("minecraft:stone".to_string().into()),
            vec![(
                "minecraft:block",
                RegistryReferenceKind::Element,
                "minecraft:stone",
            )],
        ),
        (
            br#"{"type":"matching_blocks","blocks":["stone","example:custom"]}"#.as_slice(),
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("minecraft:stone".to_string().into()),
                NbtSummaryValue::String("example:custom".to_string().into()),
            ]),
            vec![
                (
                    "minecraft:block",
                    RegistryReferenceKind::Element,
                    "minecraft:stone",
                ),
                (
                    "minecraft:block",
                    RegistryReferenceKind::Element,
                    "example:custom",
                ),
            ],
        ),
        (
            br##"{"type":"matching_blocks","blocks":"#logs"}"##.as_slice(),
            NbtSummaryValue::String("#minecraft:logs".to_string().into()),
            vec![(
                "minecraft:block",
                RegistryReferenceKind::Tag,
                "minecraft:logs",
            )],
        ),
    ];

    for (json, holder_value, expected_references) in cases {
        let decoded = decode(&decode_direct_registry_json(json).unwrap());
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &predicate("matching_blocks", vec![entry("blocks", holder_value)])
        );
        let actual = decoded
            .references()
            .map(|reference| {
                (
                    reference.target_registry,
                    reference.kind,
                    reference.resource_id.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected_references);
    }

    let recursive = decode_direct_registry_json(
        br#"{
            "type":"all_of",
            "predicates":[
                {"type":"matching_blocks","blocks":["stone","dirt"]},
                {"type":"matching_fluids","fluids":"water"}
            ]
        }"#,
    )
    .unwrap();
    let decoded = decode(&recursive);
    let actual = decoded
        .references()
        .map(|reference| {
            (
                reference.target_registry,
                reference.kind,
                reference.resource_id.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (
                "minecraft:block",
                RegistryReferenceKind::Element,
                "minecraft:stone"
            ),
            (
                "minecraft:block",
                RegistryReferenceKind::Element,
                "minecraft:dirt"
            ),
            (
                "minecraft:fluid",
                RegistryReferenceKind::Element,
                "minecraft:water"
            ),
        ]
    );
}

#[test]
fn vec3i_is_an_exact_three_int_stream_and_zero_defaults_are_omitted() {
    for type_id in [
        "matching_blocks",
        "matching_block_tag",
        "matching_fluids",
        "has_sturdy_face",
        "solid",
        "replaceable",
        "would_survive",
        "inside_world_bounds",
        "unobstructed",
    ] {
        let payload = match type_id {
            "matching_blocks" => r#","blocks":"stone""#,
            "matching_block_tag" => r#","tag":"logs""#,
            "matching_fluids" => r#","fluids":"water""#,
            "has_sturdy_face" => r#","direction":"up""#,
            "would_survive" => r#","state":{"Name":"stone"}"#,
            _ => "",
        };
        let json = format!(r#"{{"type":"{type_id}","offset":[0,0,0]{payload}}}"#);
        let decoded = decode(&decode_direct_registry_json(json.as_bytes()).unwrap());
        assert!(matches!(
            decoded.network_nbt_unchecked(),
            NbtSummaryValue::Compound(fields)
                if fields.iter().all(|entry| entry.name != "offset")
        ));
    }

    for json in [
        br#"{"type":"solid","offset":[0,0]}"#.as_slice(),
        br#"{"type":"solid","offset":[0,0,0,0]}"#.as_slice(),
        br#"{"type":"solid","offset":[0,"x",0]}"#.as_slice(),
        br#"{"type":"solid","offset":{}}"#.as_slice(),
    ] {
        let data = decode_direct_registry_json(json).unwrap();
        assert!(
            decode_block_predicate_source(data.root(), "$", &mut context(data.source)).is_err()
        );
    }
}

#[test]
fn all_directions_are_exact_and_has_sturdy_face_writes_offset_before_direction() {
    for direction in DIRECTIONS {
        let json =
            format!(r#"{{"type":"has_sturdy_face","offset":[1,2,3],"direction":"{direction}"}}"#);
        let decoded = decode(&decode_direct_registry_json(json.as_bytes()).unwrap());
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &predicate(
                "has_sturdy_face",
                vec![
                    entry("offset", offset([1, 2, 3])),
                    entry(
                        "direction",
                        NbtSummaryValue::String((*direction).to_string().into())
                    )
                ]
            )
        );
    }
    for value in ["UP", "northeast", "minecraft:up"] {
        let json = format!(r#"{{"type":"has_sturdy_face","direction":"{value}"}}"#);
        let data = decode_direct_registry_json(json.as_bytes()).unwrap();
        assert!(
            decode_block_predicate_source(data.root(), "$", &mut context(data.source)).is_err()
        );
    }
}

#[test]
fn would_survive_delegates_to_the_shared_exact_block_state_owner() {
    let known = decode(
        &decode_direct_registry_json(
            br#"{"type":"would_survive","state":{"Name":"oak_log","Properties":{"axis":"x"}}}"#,
        )
        .unwrap(),
    );
    assert_eq!(
        known.network_nbt_unchecked(),
        &predicate(
            "would_survive",
            vec![entry(
                "state",
                compound(vec![
                    entry(
                        "Properties",
                        compound(vec![entry(
                            "axis",
                            NbtSummaryValue::String("x".to_string().into())
                        )])
                    ),
                    entry(
                        "Name",
                        NbtSummaryValue::String("minecraft:oak_log".to_string().into())
                    )
                ])
            )]
        )
    );

    for json in [
        br#"{"type":"would_survive","state":{"Name":"example:missing"}}"#.as_slice(),
        br#"{"type":"would_survive","state":{}}"#.as_slice(),
        br#"{"type":"would_survive","state":"stone"}"#.as_slice(),
    ] {
        let data = decode_direct_registry_json(json).unwrap();
        assert!(
            decode_block_predicate_source(data.root(), "$", &mut context(data.source)).is_err()
        );
    }
}
