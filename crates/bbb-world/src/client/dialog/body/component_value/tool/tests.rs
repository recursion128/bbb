use std::collections::BTreeMap;

use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    packets::inventory::{DataComponentPatchSummary, ToolRuleSummary, ToolSummary},
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::RegistryTagState;

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

#[test]
fn decodes_tool_defaults_and_block_holder_rules() {
    let decoded = decode(
        &compound([("rules", NbtSummaryValue::List(Vec::new()))]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Tool { tool }) = decoded else {
        panic!("expected tool component");
    };
    assert!(tool.rules.is_empty());
    assert_eq!(tool.default_mining_speed_bits, 1.0f32.to_bits());
    assert_eq!(tool.damage_per_block, 1);
    assert!(tool.can_destroy_blocks_in_creative);

    let mut registries = RegistrySet::default();
    let stone_id = registries
        .holder_id_by_resource_id(BLOCK_REGISTRY, "minecraft:stone")
        .unwrap();
    let dirt_id = registries
        .holder_id_by_resource_id(BLOCK_REGISTRY, "minecraft:dirt")
        .unwrap();
    registries.tags.insert(
        BLOCK_REGISTRY.to_string(),
        RegistryTagState {
            registry: BLOCK_REGISTRY.to_string(),
            tags: BTreeMap::from([("example:mineable".to_string(), vec![stone_id, dirt_id])]),
        },
    );
    let nan_bits = f32::NAN.to_bits();
    let decoded = decode(
        &compound([
            (
                "rules",
                NbtSummaryValue::List(vec![
                    compound([
                        ("blocks", string("#example:mineable")),
                        ("speed", float(4.0)),
                        ("correct_for_drops", NbtSummaryValue::Byte(1)),
                    ]),
                    compound([
                        (
                            "blocks",
                            NbtSummaryValue::List(vec![string("dirt"), string("stone")]),
                        ),
                        ("correct_for_drops", NbtSummaryValue::Byte(0)),
                    ]),
                ]),
            ),
            ("default_mining_speed", NbtSummaryValue::Float(nan_bits)),
            ("damage_per_block", NbtSummaryValue::Long(0)),
            ("can_destroy_blocks_in_creative", NbtSummaryValue::Int(0)),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Tool { tool }) = decoded else {
        panic!("expected tool component");
    };
    assert_eq!(tool.default_mining_speed_bits, nan_bits);
    assert_eq!(tool.damage_per_block, 0);
    assert!(!tool.can_destroy_blocks_in_creative);
    assert_eq!(tool.rules.len(), 2);
    assert_eq!(tool.rules[0].blocks.holder_set_kind, "named");
    assert_eq!(
        tool.rules[0].blocks.tag.as_deref(),
        Some("example:mineable")
    );
    assert_eq!(tool.rules[0].speed_bits, Some(4.0f32.to_bits()));
    assert_eq!(tool.rules[0].correct_for_drops, Some(true));
    assert_eq!(
        tool.rules[1]
            .blocks
            .entries
            .iter()
            .map(|entry| entry.holder_id)
            .collect::<Vec<_>>(),
        [dirt_id, stone_id]
    );
    assert_eq!(tool.rules[1].speed_bits, None);
    assert_eq!(tool.rules[1].correct_for_drops, Some(false));

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        &DialogDataComponentValueState::Tool { tool: tool.clone() },
        &mut patch,
    ));
    assert_eq!(
        patch.tool,
        Some(ToolSummary {
            rules: vec![
                ToolRuleSummary {
                    block_registry_ids: vec![stone_id, dirt_id],
                    block_tag: Some("example:mineable".to_string()),
                    speed_bits: Some(4.0f32.to_bits()),
                    correct_for_drops: Some(true),
                },
                ToolRuleSummary {
                    block_registry_ids: vec![dirt_id, stone_id],
                    block_tag: None,
                    speed_bits: None,
                    correct_for_drops: Some(false),
                },
            ],
            default_mining_speed_bits: nan_bits,
            damage_per_block: 0,
            can_destroy_blocks_in_creative: false,
        })
    );

    let mut invalid = tool;
    invalid.rules[0].blocks = DialogRegistryHolderSetState {
        holder_set_kind: "named".to_string(),
        tag: None,
        entries: Vec::new(),
    };
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        &DialogDataComponentValueState::Tool { tool: invalid },
        &mut DataComponentPatchSummary::default(),
    ));
}

#[test]
fn decodes_context_free_tool_rule_with_empty_block_set() {
    let decoded = decode(
        &compound([(
            "rules",
            NbtSummaryValue::List(vec![compound([(
                "blocks",
                NbtSummaryValue::List(Vec::new()),
            )])]),
        )]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Tool { tool }) = decoded else {
        panic!("expected tool component");
    };
    assert!(tool.rules[0].blocks.entries.is_empty());
}

#[test]
fn rejects_malformed_tool_records_rules_and_ranges() {
    let registries = RegistrySet::default();
    for (value, context, expected) in [
        (NbtSummaryValue::Int(0), None, "is not a compound"),
        (compound([]), None, "missing field rules"),
        (
            compound([("rules", NbtSummaryValue::Int(0))]),
            None,
            "rules is not a list",
        ),
        (
            compound([(
                "rules",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            )]),
            None,
            "rules[0] is not a compound",
        ),
        (
            compound([("rules", NbtSummaryValue::List(vec![compound([])]))]),
            None,
            "missing field blocks",
        ),
        (
            tool_with_rule([("blocks", string("stone"))]),
            None,
            "without registry context",
        ),
        (
            tool_with_rule([("blocks", string("example:missing"))]),
            Some(&registries),
            "missing minecraft:block resource",
        ),
        (
            tool_with_rule([("blocks", NbtSummaryValue::Int(0))]),
            Some(&registries),
            "neither a tag, resource id, nor list",
        ),
        (
            tool_with_rule([("blocks", string("stone")), ("speed", float(0.0))]),
            Some(&registries),
            "speed 0 is outside",
        ),
        (
            tool_with_rule([
                ("blocks", string("stone")),
                ("speed", NbtSummaryValue::Float(f32::INFINITY.to_bits())),
            ]),
            Some(&registries),
            "speed inf is outside",
        ),
        (
            tool_with_rule([
                ("blocks", string("stone")),
                ("correct_for_drops", string("yes")),
            ]),
            Some(&registries),
            "correct_for_drops is not a boolean",
        ),
        (
            compound([
                ("rules", NbtSummaryValue::List(Vec::new())),
                ("damage_per_block", NbtSummaryValue::Int(-1)),
            ]),
            None,
            "damage_per_block -1 is outside",
        ),
        (
            compound([
                ("rules", NbtSummaryValue::List(Vec::new())),
                ("default_mining_speed", string("fast")),
            ]),
            None,
            "default_mining_speed is not numeric",
        ),
        (
            compound([
                ("rules", NbtSummaryValue::List(Vec::new())),
                ("can_destroy_blocks_in_creative", string("yes")),
            ]),
            None,
            "can_destroy_blocks_in_creative is not a boolean",
        ),
    ] {
        let error = decode(&value, context).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn tool_with_rule<const N: usize>(
    rule_entries: [(&'static str, NbtSummaryValue); N],
) -> NbtSummaryValue {
    compound([("rules", NbtSummaryValue::List(vec![compound(rule_entries)]))])
}

fn decode(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        "minecraft:tool",
        value,
        registries,
    )
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.to_string(),
                value,
            })
            .collect(),
    )
}
