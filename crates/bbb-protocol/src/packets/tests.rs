use super::*;

use crate::{
    builtin_enum_component_values::{
        VANILLA_AXOLOTL_VARIANT_BLUE_ID, VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_RED_ID,
        VANILLA_FOX_VARIANT_SNOW_ID, VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
        VANILLA_LLAMA_VARIANT_GRAY_ID, VANILLA_MOOSHROOM_VARIANT_BROWN_ID,
        VANILLA_PARROT_VARIANT_GRAY_ID, VANILLA_RABBIT_VARIANT_EVIL_ID,
        VANILLA_SALMON_VARIANT_LARGE_ID,
    },
    codec::{offline_player_uuid, Decoder, Encoder, ProtocolError},
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID, VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
    },
    entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
    ids,
    item_component_value_types::MapPostProcessingSummary,
    item_types::{VANILLA_ITEM_BRICK_ID, VANILLA_ITEM_STONE_ID},
    nbt_summary::decode_nbt_summary_from_decoder,
    ComponentStyle, NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};

use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

fn assert_client_information_payload(payload: &[u8], expected: &ClientInformation) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_string(16).unwrap(), expected.language.as_str());
    assert_eq!(decoder.read_i8().unwrap(), expected.view_distance);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.chat_visibility {
            ClientChatVisibility::Full => 0,
            ClientChatVisibility::System => 1,
            ClientChatVisibility::Hidden => 2,
        }
    );
    assert_eq!(decoder.read_bool().unwrap(), expected.chat_colors);
    assert_eq!(decoder.read_u8().unwrap(), expected.displayed_skin_parts);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.main_hand {
            ClientMainHand::Left => 0,
            ClientMainHand::Right => 1,
        }
    );
    assert_eq!(
        decoder.read_bool().unwrap(),
        expected.text_filtering_enabled
    );
    assert_eq!(decoder.read_bool().unwrap(), expected.allows_listing);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.particle_status {
            ClientParticleStatus::All => 0,
            ClientParticleStatus::Decreased => 1,
            ClientParticleStatus::Minimal => 2,
        }
    );
    assert!(decoder.is_empty());
}

fn assert_cookie_response_payload(payload: &[u8], key: &str, expected: Option<&[u8]>) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_string(32767).unwrap(), key);
    match expected {
        Some(expected) => {
            assert!(decoder.read_bool().unwrap());
            let len = decoder.read_len().unwrap();
            assert_eq!(
                decoder.read_exact(len, "cookie response").unwrap(),
                expected
            );
        }
        None => assert!(!decoder.read_bool().unwrap()),
    }
    assert!(decoder.is_empty());
}

fn encode_block_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut payload = vec![8];
    payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload
}

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_nbt_string(&mut payload, name);
    write_nbt_string(&mut payload, value);
    payload.push(0);
    payload
}

fn nbt_list_of_floats(values: &[f32]) -> Vec<u8> {
    let mut payload = vec![9, 5];
    payload.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

mod batch_01;
mod batch_02;
mod batch_03;
mod batch_04;
mod content_registry;
