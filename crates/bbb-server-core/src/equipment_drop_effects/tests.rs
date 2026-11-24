use crate::random::GameRandom;
use bbb_protocol::equipment_slots::EquipmentSlot;
use serde_json::json;

use super::{EquipmentDropEffects, EquipmentDropTarget};

#[test]
fn builtin_looting_adds_per_level_only_for_player_requirement_and_attacker_target() {
    let (index, _) = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .enumerate()
        .find(|(_, id)| id.as_str() == "minecraft:looting")
        .unwrap();
    let effects = EquipmentDropEffects::parse(
        &crate::enchantment_facts::builtin_enchantment_documents()[index],
        None,
    )
    .unwrap()
    .unwrap();
    let random = GameRandom::with_seed(71);
    let increased = effects
        .apply(
            EquipmentSlot::MainHand,
            EquipmentDropTarget::Attacker,
            3,
            0.1,
            &random,
            |_| Ok(true),
        )
        .unwrap();
    assert!((increased - 0.13).abs() < 0.000_001);
    assert_eq!(
        effects
            .apply(
                EquipmentSlot::OffHand,
                EquipmentDropTarget::Attacker,
                3,
                0.1,
                &random,
                |_| panic!("wrong slot cannot evaluate a requirement"),
            )
            .unwrap(),
        0.1
    );
    assert_eq!(
        effects
            .apply(
                EquipmentSlot::MainHand,
                EquipmentDropTarget::Victim,
                3,
                0.1,
                &random,
                |_| panic!("wrong target cannot evaluate a requirement"),
            )
            .unwrap(),
        0.1
    );
    assert_eq!(
        effects
            .apply(
                EquipmentSlot::MainHand,
                EquipmentDropTarget::Attacker,
                3,
                0.1,
                &random,
                |_| Ok(false),
            )
            .unwrap(),
        0.1
    );
}

#[test]
fn targeted_effects_apply_in_list_order() {
    let document = json!({
        "slots": ["armor"],
        "effects": {"minecraft:equipment_drops": [
            {"enchanted": "attacker", "effect": {"type": "minecraft:multiply", "factor": 2}},
            {"enchanted": "victim", "effect": {"type": "minecraft:add", "value": 0.2}},
            {"enchanted": "attacker", "effect": {"type": "minecraft:add", "value": 0.1},
             "requirements": {"condition": "minecraft:random_chance", "chance": 0.5}}
        ]}
    });
    let effects = EquipmentDropEffects::parse(
        &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&document).unwrap()).unwrap(),
        None,
    )
    .unwrap()
    .unwrap();
    let random = GameRandom::with_seed(72);
    let victim = effects
        .apply(
            EquipmentSlot::Body,
            EquipmentDropTarget::Victim,
            1,
            0.1,
            &random,
            |_| panic!("victim effect has no requirement"),
        )
        .unwrap();
    assert!((victim - 0.3).abs() < 0.000_001);
    let attacker = effects
        .apply(
            EquipmentSlot::Body,
            EquipmentDropTarget::Attacker,
            1,
            victim,
            &random,
            |_| Ok(true),
        )
        .unwrap();
    assert!((attacker - 0.7).abs() < 0.000_001);
    assert_eq!(
        effects
            .apply(
                EquipmentSlot::Saddle,
                EquipmentDropTarget::Attacker,
                1,
                0.1,
                &random,
                |_| panic!("armor excludes saddle"),
            )
            .unwrap(),
        0.1
    );
}
