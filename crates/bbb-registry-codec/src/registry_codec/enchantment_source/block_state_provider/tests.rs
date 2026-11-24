use crate::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_network_registry_element,
    registry_codec::{
        recursive_values::RegistryCodecDecodeContext, RegistryElementData, RegistryElementSource,
    },
};
use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry};

use super::*;

const OWNER_REGISTRY: &str = "minecraft:enchantment";
const OWNER_CODEC: &str = "Enchantment.DIRECT_CODEC";

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER_REGISTRY, OWNER_CODEC, source)
}
fn field_value<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("expected compound")
    };
    &fields
        .iter()
        .find(|field| field.name == name)
        .expect("field exists")
        .value
}
fn network(value: &NbtSummaryValue) -> RegistryElementData {
    decode_network_registry_element(&encode_nbt_summary(value, 1 << 20).unwrap()).unwrap()
}
fn decode(data: &RegistryElementData) -> CanonicalRegistryCodecValue {
    decode_block_state_provider_source(data.root(), "$", &mut context(data.source)).unwrap()
}
fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}
fn state(name: &str) -> NbtSummaryValue {
    compound(vec![entry(
        "Name",
        NbtSummaryValue::String(name.to_string().into()),
    )])
}
fn canonical_state(name: &str) -> NbtSummaryValue {
    state(&format!("minecraft:{name}"))
}
fn rotated_oak_log_default() -> NbtSummaryValue {
    compound(vec![
        entry(
            "Properties",
            compound(vec![entry(
                "axis",
                NbtSummaryValue::String("y".to_string().into()),
            )]),
        ),
        entry(
            "Name",
            NbtSummaryValue::String("minecraft:oak_log".to_string().into()),
        ),
    ])
}
fn provider(kind: &str, fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    let mut fields = fields;
    fields.push(entry(
        "type",
        NbtSummaryValue::String(format!("minecraft:{kind}").into()),
    ));
    compound(fields)
}
fn noise() -> NbtSummaryValue {
    compound(vec![
        entry("firstOctave", NbtSummaryValue::Int(0)),
        entry("amplitudes", NbtSummaryValue::List(vec![])),
    ])
}
fn simple() -> NbtSummaryValue {
    provider(
        "simple_state_provider",
        vec![entry("state", state("stone"))],
    )
}

#[test]
fn every_family_has_one_exact_canonical_tree_for_all_three_sources() {
    let cases = [
        (
            r#"{"type":"simple_state_provider","state":{"Name":"stone"}}"#,
            simple(),
            provider(
                "simple_state_provider",
                vec![entry("state", canonical_state("stone"))],
            ),
        ),
        (
            r#"{"type":"weighted_state_provider","entries":[{"data":{"Name":"stone"},"weight":1}]}"#,
            provider(
                "weighted_state_provider",
                vec![entry(
                    "entries",
                    NbtSummaryValue::List(vec![compound(vec![
                        entry("data", state("stone")),
                        entry("weight", NbtSummaryValue::Int(1)),
                    ])]),
                )],
            ),
            provider(
                "weighted_state_provider",
                vec![entry(
                    "entries",
                    NbtSummaryValue::List(vec![compound(vec![
                        entry("data", canonical_state("stone")),
                        entry("weight", NbtSummaryValue::Int(1)),
                    ])]),
                )],
            ),
        ),
        (
            r#"{"type":"noise_threshold_provider","seed":1,"noise":{"firstOctave":0,"amplitudes":[]},"scale":1,"threshold":0,"high_chance":0,"default_state":{"Name":"stone"},"low_states":[{"Name":"dirt"}],"high_states":[{"Name":"stone"}]}"#,
            provider(
                "noise_threshold_provider",
                vec![
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("threshold", NbtSummaryValue::Float(0)),
                    entry("high_chance", NbtSummaryValue::Float(0)),
                    entry("default_state", state("stone")),
                    entry("low_states", NbtSummaryValue::List(vec![state("dirt")])),
                    entry("high_states", NbtSummaryValue::List(vec![state("stone")])),
                ],
            ),
            provider(
                "noise_threshold_provider",
                vec![
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("threshold", NbtSummaryValue::Float(0)),
                    entry("high_chance", NbtSummaryValue::Float(0)),
                    entry("default_state", canonical_state("stone")),
                    entry(
                        "low_states",
                        NbtSummaryValue::List(vec![canonical_state("dirt")]),
                    ),
                    entry(
                        "high_states",
                        NbtSummaryValue::List(vec![canonical_state("stone")]),
                    ),
                ],
            ),
        ),
        (
            r#"{"type":"noise_provider","seed":1,"noise":{"firstOctave":0,"amplitudes":[]},"scale":1,"states":[{"Name":"stone"}]}"#,
            provider(
                "noise_provider",
                vec![
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("states", NbtSummaryValue::List(vec![state("stone")])),
                ],
            ),
            provider(
                "noise_provider",
                vec![
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry(
                        "states",
                        NbtSummaryValue::List(vec![canonical_state("stone")]),
                    ),
                ],
            ),
        ),
        (
            r#"{"type":"dual_noise_provider","variety":[2,3],"slow_noise":{"firstOctave":0,"amplitudes":[]},"slow_scale":1,"seed":1,"noise":{"firstOctave":0,"amplitudes":[]},"scale":1,"states":[{"Name":"stone"}]}"#,
            provider(
                "dual_noise_provider",
                vec![
                    entry(
                        "variety",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Int(2),
                            NbtSummaryValue::Int(3),
                        ]),
                    ),
                    entry("slow_noise", noise()),
                    entry("slow_scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("states", NbtSummaryValue::List(vec![state("stone")])),
                ],
            ),
            provider(
                "dual_noise_provider",
                vec![
                    entry(
                        "variety",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Int(2),
                            NbtSummaryValue::Int(3),
                        ]),
                    ),
                    entry("slow_noise", noise()),
                    entry("slow_scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry("seed", NbtSummaryValue::Long(1)),
                    entry("noise", noise()),
                    entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry(
                        "states",
                        NbtSummaryValue::List(vec![canonical_state("stone")]),
                    ),
                ],
            ),
        ),
        (
            r#"{"type":"rotated_block_provider","state":{"Name":"oak_log","Properties":{"axis":"x"}}}"#,
            provider(
                "rotated_block_provider",
                vec![entry(
                    "state",
                    compound(vec![
                        entry(
                            "Name",
                            NbtSummaryValue::String("oak_log".to_string().into()),
                        ),
                        entry(
                            "Properties",
                            compound(vec![entry(
                                "axis",
                                NbtSummaryValue::String("x".to_string().into()),
                            )]),
                        ),
                    ]),
                )],
            ),
            provider(
                "rotated_block_provider",
                vec![entry("state", rotated_oak_log_default())],
            ),
        ),
        (
            r#"{"type":"randomized_int_state_provider","source":{"type":"simple_state_provider","state":{"Name":"stone"}},"property":"not_a_real_property","values":-2147483648}"#,
            provider(
                "randomized_int_state_provider",
                vec![
                    entry("source", simple()),
                    entry(
                        "property",
                        NbtSummaryValue::String("not_a_real_property".to_string().into()),
                    ),
                    entry("values", NbtSummaryValue::Int(i32::MIN)),
                ],
            ),
            provider(
                "randomized_int_state_provider",
                vec![
                    entry(
                        "source",
                        provider(
                            "simple_state_provider",
                            vec![entry("state", canonical_state("stone"))],
                        ),
                    ),
                    entry(
                        "property",
                        NbtSummaryValue::String("not_a_real_property".to_string().into()),
                    ),
                    entry("values", NbtSummaryValue::Int(i32::MIN)),
                ],
            ),
        ),
        (
            r#"{"type":"rule_based_state_provider","rules":[]}"#,
            provider(
                "rule_based_state_provider",
                vec![entry("rules", NbtSummaryValue::List(vec![]))],
            ),
            provider(
                "rule_based_state_provider",
                vec![entry("rules", NbtSummaryValue::List(vec![]))],
            ),
        ),
    ];
    for (json, input_nbt, expected) in cases {
        for data in [
            decode_direct_registry_json(json.as_bytes()).unwrap(),
            decode_client_known_pack_registry_json(json.as_bytes()).unwrap(),
            network(&input_nbt),
        ] {
            assert_eq!(decode(&data).network_nbt_unchecked(), &expected, "{json}");
        }
    }
}

#[test]
fn variety_accepts_all_interval_shapes_and_canonicalizes_equal_ranges() {
    let dual = |variety| {
        provider(
            "dual_noise_provider",
            vec![
                entry("variety", variety),
                entry("slow_noise", noise()),
                entry("slow_scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                entry("seed", NbtSummaryValue::Long(0)),
                entry("noise", noise()),
                entry("scale", NbtSummaryValue::Float(1.0_f32.to_bits())),
                entry("states", NbtSummaryValue::List(vec![state("stone")])),
            ],
        )
    };
    let forms = [
        (NbtSummaryValue::Int(2), NbtSummaryValue::Int(2)),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(2)]),
            NbtSummaryValue::Int(2),
        ),
        (
            compound(vec![
                entry("min_inclusive", NbtSummaryValue::Int(2)),
                entry("max_inclusive", NbtSummaryValue::Int(2)),
            ]),
            NbtSummaryValue::Int(2),
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(5)]),
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(5)]),
        ),
        (
            compound(vec![
                entry("min_inclusive", NbtSummaryValue::Int(2)),
                entry("max_inclusive", NbtSummaryValue::Int(5)),
            ]),
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(2), NbtSummaryValue::Int(5)]),
        ),
    ];
    for (input, expected) in forms {
        assert_eq!(
            field_value(
                decode(&network(&dual(input))).network_nbt_unchecked(),
                "variety"
            ),
            &expected
        );
    }
    for input in [
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(2)]),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Int(2),
            NbtSummaryValue::Int(3),
            NbtSummaryValue::Int(4),
        ]),
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(5), NbtSummaryValue::Int(2)]),
        NbtSummaryValue::Int(0),
        NbtSummaryValue::Int(65),
    ] {
        let data = network(&dual(input));
        assert!(
            decode_block_state_provider_source(data.root(), "$", &mut context(data.source))
                .is_err()
        );
    }
}
