//! Vanilla 26.1 DataComponentType registry ids.
//!
//! The order follows the register calls in
//! net.minecraft.core.component.DataComponents.

const VANILLA_DATA_COMPONENT_TYPE_RESOURCE_IDS: &[&str] = &[
    "minecraft:custom_data",
    "minecraft:max_stack_size",
    "minecraft:max_damage",
    "minecraft:damage",
    "minecraft:unbreakable",
    "minecraft:use_effects",
    "minecraft:custom_name",
    "minecraft:minimum_attack_charge",
    "minecraft:damage_type",
    "minecraft:item_name",
    "minecraft:item_model",
    "minecraft:lore",
    "minecraft:rarity",
    "minecraft:enchantments",
    "minecraft:can_place_on",
    "minecraft:can_break",
    "minecraft:attribute_modifiers",
    "minecraft:custom_model_data",
    "minecraft:tooltip_display",
    "minecraft:repair_cost",
    "minecraft:creative_slot_lock",
    "minecraft:enchantment_glint_override",
    "minecraft:intangible_projectile",
    "minecraft:food",
    "minecraft:consumable",
    "minecraft:use_remainder",
    "minecraft:use_cooldown",
    "minecraft:damage_resistant",
    "minecraft:tool",
    "minecraft:weapon",
    "minecraft:attack_range",
    "minecraft:enchantable",
    "minecraft:equippable",
    "minecraft:repairable",
    "minecraft:glider",
    "minecraft:tooltip_style",
    "minecraft:death_protection",
    "minecraft:blocks_attacks",
    "minecraft:piercing_weapon",
    "minecraft:kinetic_weapon",
    "minecraft:swing_animation",
    "minecraft:additional_trade_cost",
    "minecraft:stored_enchantments",
    "minecraft:dye",
    "minecraft:dyed_color",
    "minecraft:map_color",
    "minecraft:map_id",
    "minecraft:map_decorations",
    "minecraft:map_post_processing",
    "minecraft:charged_projectiles",
    "minecraft:bundle_contents",
    "minecraft:potion_contents",
    "minecraft:potion_duration_scale",
    "minecraft:suspicious_stew_effects",
    "minecraft:writable_book_content",
    "minecraft:written_book_content",
    "minecraft:trim",
    "minecraft:debug_stick_state",
    "minecraft:entity_data",
    "minecraft:bucket_entity_data",
    "minecraft:block_entity_data",
    "minecraft:instrument",
    "minecraft:provides_trim_material",
    "minecraft:ominous_bottle_amplifier",
    "minecraft:jukebox_playable",
    "minecraft:provides_banner_patterns",
    "minecraft:recipes",
    "minecraft:lodestone_tracker",
    "minecraft:firework_explosion",
    "minecraft:fireworks",
    "minecraft:profile",
    "minecraft:note_block_sound",
    "minecraft:banner_patterns",
    "minecraft:base_color",
    "minecraft:pot_decorations",
    "minecraft:container",
    "minecraft:block_state",
    "minecraft:bees",
    "minecraft:lock",
    "minecraft:container_loot",
    "minecraft:break_sound",
    "minecraft:villager/variant",
    "minecraft:wolf/variant",
    "minecraft:wolf/sound_variant",
    "minecraft:wolf/collar",
    "minecraft:fox/variant",
    "minecraft:salmon/size",
    "minecraft:parrot/variant",
    "minecraft:tropical_fish/pattern",
    "minecraft:tropical_fish/base_color",
    "minecraft:tropical_fish/pattern_color",
    "minecraft:mooshroom/variant",
    "minecraft:rabbit/variant",
    "minecraft:pig/variant",
    "minecraft:pig/sound_variant",
    "minecraft:cow/variant",
    "minecraft:cow/sound_variant",
    "minecraft:chicken/variant",
    "minecraft:chicken/sound_variant",
    "minecraft:zombie_nautilus/variant",
    "minecraft:frog/variant",
    "minecraft:horse/variant",
    "minecraft:painting/variant",
    "minecraft:llama/variant",
    "minecraft:axolotl/variant",
    "minecraft:cat/variant",
    "minecraft:cat/sound_variant",
    "minecraft:cat/collar",
    "minecraft:sheep/color",
    "minecraft:shulker/color",
];

pub fn vanilla_data_component_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {
    usize::try_from(type_id)
        .ok()
        .and_then(|index| VANILLA_DATA_COMPONENT_TYPE_RESOURCE_IDS.get(index).copied())
}

pub fn vanilla_data_component_type_is_persistent(type_id: i32) -> bool {
    vanilla_data_component_resource_id_for_type_id(type_id).is_some()
        && !matches!(type_id, 20 | 41 | 48)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn data_component_resource_ids_follow_vanilla_registration_order() {
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(0),
            Some("minecraft:custom_data")
        );
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(18),
            Some("minecraft:tooltip_display")
        );
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(77),
            Some("minecraft:bees")
        );
        assert_eq!(
            vanilla_data_component_resource_id_for_type_id(109),
            Some("minecraft:shulker/color")
        );
        assert_eq!(vanilla_data_component_resource_id_for_type_id(-1), None);
        assert_eq!(vanilla_data_component_resource_id_for_type_id(110), None);
        assert!(vanilla_data_component_type_is_persistent(0));
        assert!(!vanilla_data_component_type_is_persistent(20));
        assert!(!vanilla_data_component_type_is_persistent(41));
        assert!(!vanilla_data_component_type_is_persistent(48));
        assert!(vanilla_data_component_type_is_persistent(109));
        assert!(!vanilla_data_component_type_is_persistent(110));
    }

    #[test]
    fn data_component_resource_ids_match_local_vanilla_registration_order() {
        let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mc-code/sources/26.1/net/minecraft/core/component")
            .join("DataComponents.java");
        if !source_path.exists() {
            eprintln!(
                "skipping local DataComponents source check; missing {}",
                source_path.display()
            );
            return;
        }

        let source = std::fs::read_to_string(source_path).unwrap();
        let mut remaining = source.as_str();
        let mut registered = Vec::new();
        while let Some(index) = remaining.find("= register(") {
            remaining = &remaining[index + "= register(".len()..];
            let start = remaining.find('"').unwrap() + 1;
            let tail = &remaining[start..];
            let end = tail.find('"').unwrap();
            registered.push(format!("minecraft:{}", &tail[..end]));
            remaining = &tail[end + 1..];
        }

        assert_eq!(registered, VANILLA_DATA_COMPONENT_TYPE_RESOURCE_IDS);
        let transient = source
            .split("= register(")
            .skip(1)
            .take(VANILLA_DATA_COMPONENT_TYPE_RESOURCE_IDS.len())
            .enumerate()
            .filter_map(|(type_id, definition)| {
                (!definition.contains(".persistent(")).then_some(type_id as i32)
            })
            .collect::<Vec<_>>();
        assert_eq!(transient, vec![20, 41, 48]);
    }
}
