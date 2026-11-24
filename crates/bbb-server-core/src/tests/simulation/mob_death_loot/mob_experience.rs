use super::*;

/// A datapack `mob_experience` effect on the killer's weapon
/// (`EnchantmentHelper.processMobExperience`)
/// moves the reward when its requirement matches the killed entity
/// (`Enchantment.modifyMobExperience`); an
/// unnamespaced component key is read as `minecraft:`.
#[test]
fn a_mob_experience_enchantment_on_the_killers_weapon_adds_experience() {
    let config = |victim: &str| {
        datapack_config_with_enchantment_effects(
            "minecraft:sharpness",
            serde_json::json!({"mob_experience": [{
                "effect": {"type": "minecraft:add", "value": 7.0},
                "requirements": {"condition": "minecraft:entity_properties", "entity": "this",
                    "predicate": {"type": victim}}
            }]}),
        )
    };
    let reward = |victim: &str| {
        let (sender, receiver, mut world, blaze) = player_kill_fixture_in(
            config(victim),
            VANILLA_ENTITY_TYPE_BLAZE_ID,
            Some(("minecraft:diamond_sword", &[("minecraft:sharpness", 1)])),
        );
        attack_by_player(&sender, &receiver, &mut world, blaze);
        orbs(&world).iter().map(|(_, value, _)| value).sum::<i32>()
    };
    let unmatched = reward("minecraft:zombie");
    assert!(unmatched > 0);
    assert_eq!(reward("minecraft:blaze"), unmatched + 7);
}
