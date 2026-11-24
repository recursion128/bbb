use super::*;
use crate::inventory::item_prototype_int_component;
use crate::loot_runtime::LootWeather;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
};

use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

const CLEAR: ValueEffectLevel<'static> = ValueEffectLevel::weather_only(LootWeather {
    raining: false,
    thundering: false,
});

fn item_id(resource_id: &str) -> i32 {
    vanilla_item_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("the generated catalog carries {resource_id}"))
        .id
}

fn stack(resource_id: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id(resource_id)),
        count: 1,
        component_patch: Default::default(),
    }
}

/// A draw stream nothing in these tests depends on the values of; the exact-loop
/// arm of `RemoveBinomial` is the only reader.
fn random() -> GameRandom {
    GameRandom::with_seed(0x_D0_0D)
}

#[test]
fn armor_slot_group_includes_body_but_not_saddle() {
    assert!(equipment_slot_group_matches("armor", EquipmentSlot::Body));
    assert!(equipment_slot_group_matches("armor", EquipmentSlot::Head));
    assert!(!equipment_slot_group_matches(
        "armor",
        EquipmentSlot::Saddle
    ));
    assert!(!equipment_slot_group_matches(
        "armor",
        EquipmentSlot::MainHand
    ));
}

fn unbreaking() -> EnchantmentValueEffects {
    let document = bbb_registry_codec::parse_registry_json(UNBREAKING_JSON.as_bytes()).unwrap();
    EnchantmentValueEffects::parse(&document, ITEM_DAMAGE_COMPONENT)
        .expect("unbreaking.json declares an item_damage list this crate understands")
        .expect("unbreaking.json declares an item_damage list")
}

/// The built-in `unbreaking.json`, as `data/minecraft/enchantment/unbreaking.json`
/// ships it. Only the `effects` and `slots` members are kept: they are the two
/// this module reads.
const UNBREAKING_JSON: &str = r##"{
  "slots": ["any"],
  "effects": {
    "minecraft:item_damage": [
      {
        "effect": {
          "type": "minecraft:remove_binomial",
          "chance": {
            "type": "minecraft:fraction",
            "denominator": { "type": "minecraft:linear", "base": 10.0, "per_level_above_first": 5.0 },
            "numerator": { "type": "minecraft:linear", "base": 2.0, "per_level_above_first": 2.0 }
          }
        },
        "requirements": {
          "condition": "minecraft:match_tool",
          "predicate": { "items": "#minecraft:enchantable/armor" }
        }
      },
      {
        "effect": {
          "type": "minecraft:remove_binomial",
          "chance": {
            "type": "minecraft:fraction",
            "denominator": { "type": "minecraft:linear", "base": 2.0, "per_level_above_first": 1.0 },
            "numerator": { "type": "minecraft:linear", "base": 1.0, "per_level_above_first": 1.0 }
          }
        },
        "requirements": {
          "condition": "minecraft:inverted",
          "term": {
            "condition": "minecraft:match_tool",
            "predicate": { "items": "#minecraft:enchantable/armor" }
          }
        }
      }
    ]
  }
}"##;

/// The chance formulas the two conditional effects resolve to, derived rather
/// than written down: unbreaking on a tool is `level / (level + 1)`, and on
/// armour `2 * level / (5 * level + 5)`.
#[test]
fn the_unbreaking_chance_follows_the_level() {
    let effects = unbreaking();
    let chance = |index: usize| match &effects.entries[index].effect {
        ValueEffect::RemoveBinomial(chance) => chance.clone(),
        other => panic!("unbreaking is a remove_binomial, not {other:?}"),
    };
    let tool = chance(1);
    for (level, expected) in [(1, 1.0 / 2.0), (2, 2.0 / 3.0), (3, 3.0 / 4.0)] {
        assert_eq!(tool.calculate(level), expected, "level {level}");
    }
    let armor = chance(0);
    for (level, expected) in [(1, 2.0 / 10.0), (2, 4.0 / 15.0), (3, 6.0 / 20.0)] {
        assert_eq!(armor.calculate(level), expected, "level {level}");
    }
}

/// An enchantment with no `item_damage` member is not an error; it simply does
/// not touch durability.
#[test]
fn an_enchantment_without_item_damage_parses_as_absent() {
    let document = serde_json::json!({ "effects": { "minecraft:damage": [] } });
    assert_eq!(
        EnchantmentValueEffects::parse(
            &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&document).unwrap())
                .unwrap(),
            ITEM_DAMAGE_COMPONENT
        ),
        Ok(None)
    );
    assert_eq!(
        EnchantmentValueEffects::parse(
            &bbb_registry_codec::parse_registry_json(br#"{"weight":5}"#).unwrap(),
            ITEM_DAMAGE_COMPONENT
        ),
        Ok(None)
    );
}

/// One block off a fresh pickaxe costs the `Tool.damagePerBlock` the generated
/// prototype carries, which for every `ToolMaterial` tool is one
/// (`ToolMaterial.java:54`).
#[test]
fn mining_one_block_costs_one_durability() {
    let mut pickaxe = stack("minecraft:diamond_pickaxe");
    let outcome = mine_block(&mut pickaxe, 1.5, false, false, &[], &random(), &CLEAR);
    assert!(
        outcome.used,
        "a tool stack answers true from Item.mineBlock"
    );
    assert_eq!(outcome.broken_item_id, None);
    assert_eq!(pickaxe.component_patch.damage, Some(1));
}

/// `Item.mineBlock`'s `state.getDestroySpeed(level, pos) != 0.0F` guard
/// (`Item.java:263`): a zero-hardness block still counts as a use but costs no
/// durability.
#[test]
fn a_zero_hardness_block_costs_nothing() {
    let mut pickaxe = stack("minecraft:diamond_pickaxe");
    let outcome = mine_block(&mut pickaxe, 0.0, false, false, &[], &random(), &CLEAR);
    assert!(outcome.used);
    assert_eq!(pickaxe.component_patch.damage, None);
}

/// `ShearsItem.mineBlock` (`ShearsItem.java:54`) wears on everything outside
/// `#minecraft:fire`, zero hardness included.
#[test]
fn shears_wear_on_zero_hardness_blocks_but_not_on_fire() {
    let mut shears = stack("minecraft:shears");
    assert!(mine_block(&mut shears, 0.0, false, false, &[], &random(), &CLEAR).used);
    assert_eq!(shears.component_patch.damage, Some(1));
    assert!(mine_block(&mut shears, 0.0, true, false, &[], &random(), &CLEAR).used);
    assert_eq!(shears.component_patch.damage, Some(1));
}

/// `Item.mineBlock` returns false for a stack with no `DataComponents.TOOL`
/// (`Item.java:259-261`), which is what keeps `Stats.ITEM_USED` from moving for
/// a player who punches a block.
#[test]
fn a_stack_without_a_tool_is_not_used_and_takes_no_damage() {
    let mut dirt = stack("minecraft:dirt");
    let outcome = mine_block(&mut dirt, 1.5, false, false, &[], &random(), &CLEAR);
    assert!(!outcome.used);
    assert_eq!(dirt.component_patch.damage, None);

    let mut empty = ItemStackSummary::empty();
    let outcome = mine_block(&mut empty, 1.5, false, false, &[], &random(), &CLEAR);
    assert!(!outcome.used);
}

/// `processDurabilityChange`'s creative guard (`ItemStack.java:455-456`).
#[test]
fn a_creative_player_wears_nothing_out() {
    let mut pickaxe = stack("minecraft:diamond_pickaxe");
    let outcome = mine_block(&mut pickaxe, 1.5, false, true, &[], &random(), &CLEAR);
    assert!(outcome.used, "the use is still counted");
    assert_eq!(pickaxe.component_patch.damage, None);
}

/// `ItemStack.isDamageableItem` (`ItemStack.java:417-419`): an `UNBREAKABLE`
/// stack takes no damage at all.
#[test]
fn an_unbreakable_tool_takes_no_damage() {
    let mut pickaxe = stack("minecraft:diamond_pickaxe");
    pickaxe.component_patch.unbreakable = true;
    pickaxe
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID);
    let outcome = mine_block(&mut pickaxe, 1.5, false, false, &[], &random(), &CLEAR);
    assert!(outcome.used);
    assert_eq!(pickaxe.component_patch.damage, None);
}

/// `applyDamage`'s break branch (`ItemStack.java:468-472`): the stack shrinks by
/// one, which empties a single tool, and the broken item is reported so the
/// caller can award `Stats.ITEM_BROKEN` and broadcast the equipment event.
#[test]
fn the_last_point_of_durability_breaks_the_tool() {
    let max_damage = item_prototype_int_component(
        item_id("minecraft:wooden_pickaxe"),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    )
    .expect("a wooden pickaxe declares a max damage");
    let mut pickaxe = stack("minecraft:wooden_pickaxe");
    set_damage(&mut pickaxe, max_damage - 1);
    let outcome = mine_block(&mut pickaxe, 1.5, false, false, &[], &random(), &CLEAR);
    assert_eq!(
        outcome.broken_item_id,
        Some(item_id("minecraft:wooden_pickaxe"))
    );
    assert_eq!(pickaxe.item_id, None);
    assert_eq!(pickaxe.count, 0);
}

/// Unbreaking reduces the change through `RemoveBinomial`, and the reduction is
/// the tool arm of `unbreaking.json` rather than the armour one: a pickaxe is
/// not in `#minecraft:enchantable/armor`, so the inverted requirement matches.
///
/// With one point of damage per block and a chance of `level / (level + 1)`,
/// the outcome is either zero or one, so the assertion is over a run of blocks
/// rather than over a single draw.
#[test]
fn unbreaking_reduces_the_durability_a_tool_loses() {
    let effects = unbreaking();
    let enchanted: Vec<(i32, &EnchantmentValueEffects)> = vec![(3, &effects)];
    let random = random();
    let mut damage_taken = 0;
    for _ in 0..400 {
        let mut pickaxe = stack("minecraft:diamond_pickaxe");
        mine_block(&mut pickaxe, 1.5, false, false, &enchanted, &random, &CLEAR);
        damage_taken += pickaxe.component_patch.damage.unwrap_or(0);
    }
    // Unbreaking III drops three of every four points; four hundred unenchanted
    // blocks would cost four hundred.
    assert!(
        (40..=160).contains(&damage_taken),
        "unbreaking III took {damage_taken} of 400 points"
    );
}

/// The same enchantment on a helmet takes the *armour* arm, which is a much
/// weaker reduction -- the point of the two conditional effects.
#[test]
fn unbreaking_uses_the_armour_arm_for_armour() {
    let effects = unbreaking();
    let helmet = stack("minecraft:diamond_helmet");
    let pickaxe = stack("minecraft:diamond_pickaxe");
    let matches = |entry: usize, stack: &ItemStackSummary| {
        let requirement = effects.entries[entry]
            .requirement
            .as_ref()
            .expect("both entries are conditional");
        effects
            .with_requirements(1, ValueContext::Item(stack), &random(), &CLEAR, |matches| {
                matches(requirement)
            })
            .unwrap()
    };
    assert!(matches(0, &helmet));
    assert!(!matches(0, &pickaxe));
    assert!(!matches(1, &helmet));
    assert!(matches(1, &pickaxe));
}

/// A datapack `item_damage` effect reads its pack's tag and current level weather.
#[test]
fn datapack_item_damage_requirements_read_item_tags_and_weather() {
    let dir = std::env::temp_dir().join(format!("bbb-item-damage-{}", uuid::Uuid::new_v4()));
    let pack = dir.join("test");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/tags/item")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"item damage","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/test/tags/item/hardy.json"),
        r#"{"values":["minecraft:diamond_pickaxe"]}"#,
    )
    .unwrap();
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:unbreaking")
        .unwrap();
    let mut document =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()
            .clone();
    document["effects"] = serde_json::json!({"item_damage": [{
        "effect": {"type": "minecraft:set", "value": 0.0},
        "requirements": {"condition": "minecraft:all_of", "terms": [
            {"condition": "minecraft:match_tool", "predicate": {"items": "#test:hardy"}},
            {"condition": "minecraft:weather_check", "raining": true},
            {"condition": "minecraft:random_chance", "chance": 1.0}
        ]}
    }]});
    std::fs::write(
        pack.join("data/minecraft/enchantment/unbreaking.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(dir.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let startup = Arc::new(startup);
    let table = crate::enchantment_facts::load_enchantment_value_effects(
        Some(&startup),
        ITEM_DAMAGE_COMPONENT,
    );
    let index = startup
        .registries
        .registry("minecraft:enchantment")
        .unwrap()
        .elements
        .iter()
        .position(|element| element.id == "minecraft:unbreaking")
        .unwrap();
    let effects = table[index]
        .as_ref()
        .expect("unbreaking declares item_damage");
    let enchanted = [(1, effects)];
    let mut hardy = stack("minecraft:diamond_pickaxe");
    let mut dry = hardy.clone();
    let rain = ValueEffectLevel::weather_only(LootWeather {
        raining: true,
        ..CLEAR.weather
    });
    mine_block(&mut hardy, 1.5, false, false, &enchanted, &random(), &rain);
    mine_block(&mut dry, 1.5, false, false, &enchanted, &random(), &CLEAR);
    assert_eq!(damage_and_max(&dry).unwrap().0, 1);
    assert_eq!(hardy.component_patch.damage.unwrap_or(0), 0);
    let mut plain = stack("minecraft:iron_pickaxe");
    mine_block(&mut plain, 1.5, false, false, &enchanted, &random(), &rain);
    assert_eq!(plain.component_patch.damage, Some(1));
    std::fs::remove_dir_all(dir).unwrap();
}
