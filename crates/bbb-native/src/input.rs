use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::{
    attribute_types::vanilla_attribute_type,
    chat_formattings::VANILLA_CHAT_FORMATTINGS,
    entity_types::{
        vanilla_entity_resource_id_for_type_id, VANILLA_ENTITY_TYPE_ALLAY_ID,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ARROW_ID,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BEE_ID,
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
        VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_CAT_ID,
        VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREAKING_ID,
        VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_EGG_ID,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_ENDERMITE_ID,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_FROG_ID,
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
        VANILLA_ENTITY_TYPE_GIANT_ID, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
        VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
        VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
        VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
        VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
        VANILLA_ENTITY_TYPE_SILVERFISH_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_SNIFFER_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
        VANILLA_ENTITY_TYPE_SQUID_ID, VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
        VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        VANILLA_ENTITY_TYPE_TRIDENT_ID, VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        VANILLA_ENTITY_TYPE_WARDEN_ID, VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    },
    mob_effect_types::vanilla_mob_effect_type,
    packets::{
        BlockEntityTagQuery, BlockPos as ProtocolBlockPos, ChangeGameModeCommand, CommandBlockMode,
        ConsumeEffectSummary, Direction as ProtocolDirection, EntityDataEnumSerializer,
        EntityDataRegistryHolder, EntityDataValueKind, EntityTagQuery,
        FireworkExplosionShapeSummary, FireworkExplosionSummary, GameType, InteractionHand,
        ItemStackSummary, MobEffectDetailsSummary, MobEffectInstanceSummary, NbtSummaryValue,
        PlayerActionKind, PlayerCommandAction, PlayerInput, PlayerModelTypeSummary, RecipeBookType,
        ResolvableProfileData, ResolvableProfileKind, ResolvableProfileKindSummary,
        ResolvableProfileSummary, SeenAdvancements, SetCommandBlock, SetCommandMinecart,
        SignUpdate, MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS,
    },
    ComponentClickEvent, ComponentStyle, StyledTextRun, MC_BUILD_TIME, MC_DATA_PACK_FORMAT,
    MC_DATA_VERSION, MC_DATA_VERSION_SERIES, MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION,
    PROTOCOL_VERSION,
};
use bbb_world::sign_wood_and_form_for_block_name;
use bbb_world::vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_abstract_horse_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_age_locked_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_armadillo_state_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_armor_stand_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_bat_flags_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_bee_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_cat_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_companion_variant_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_copper_golem_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_creeper_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_end_crystal_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_entity_accessors;
use bbb_world::vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_fox_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_frog_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_goat_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_horse_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_interaction_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_llama_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_mob_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_panda_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_parrot_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_phantom_size_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_primed_tnt_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_salmon_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_sheep_wool_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_shulker_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_slime_size_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_tamable_animal_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_turtle_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type;
use bbb_world::vanilla_entity_metadata_wolf_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_zombie_accessors_for_entity_type;
use bbb_world::vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type;
use bbb_world::{
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_dolphin_accessors_for_entity_type,
    vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_painting_variant_accessor_for_entity_type, BlockPos,
    CommandSuggestionState, CommandSuggestionsResultState, EntityState, EntityVec3,
    LocalBlockAttackEffect, LocalPlayerInputState, LocalPlayerPoseState, SignBlockForm,
    TagQueryResponseState, VanillaArmorStandMetadataAccessors,
    VanillaFireworkRocketMetadataAccessors, WorldStore,
};
use bbb_world::{
    vanilla_painting_variant_resource_id,
    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID as COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
};
use bbb_world::{
    VANILLA_ARMADILLO_STATE_ROLLING_ID as ARMADILLO_STATE_ROLLING_ID,
    VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID as ARMADILLO_STATE_UNROLLING_ID,
};
use bbb_world::{
    VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID as COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID as COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID as COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::debug_entries::{
    DebugScreenEntryId, DebugScreenEntryList, DebugScreenEntryStatus, DebugScreenProfile,
};
use crate::particle_runtime::{LocalDestroyHitParticleState, LocalPlayerActionParticleState};

mod book;
mod bundle;
mod commands;
mod component_patch;
pub(crate) mod death_screen;
pub(crate) mod game_event_screen;
mod game_rules;
mod inventory;
mod jigsaw_block;
mod minecart_command_block;
mod mouse;
mod movement;
pub(crate) mod spectator_menu;
mod structure_block;
mod test_block;
mod test_instance_block;
pub(crate) mod text_edit;

use crate::camera_pose::camera_pose_from_world;
use crate::crosshair::protocol_block_pos_from_world;
use crate::crosshair::{crosshair_target_from_camera_at_partial_tick, CrosshairTarget};
use crate::terrain_runtime::TerrainUploadState;
use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_protocol::potion_types::vanilla_potion;
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;
use bbb_renderer::{
    hud_command_block_editor_layout, AxolotlModelVariant, BannerPatternKind, CatModelVariant,
    ChickenModelVariant, CowModelVariant, FoxModelVariant, FrogModelVariant, MooshroomVariant,
    ParrotModelVariant, PigModelVariant, VillagerModelType, WolfModelVariant,
    ZombieNautilusModelVariant,
};
use bbb_world::vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type;
use bbb_world::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, VanillaEnchantmentIdentity, VanillaInstrumentIdentity,
    VanillaTrimMaterialIdentity, VanillaTrimPatternIdentity, WolfSoundVariantIdentity,
};
#[cfg(test)]
pub(crate) use book::{
    handle_book_screen_mouse_input, BOOK_MENU_BUTTON_Y, BOOK_MENU_DONE_BUTTON_X,
    BOOK_PAGE_BUTTON_Y, BOOK_PAGE_FORWARD_BUTTON_X, BOOK_SCREEN_WIDTH,
};
pub(crate) use book::{BookInputMode, BookInputState};
pub(crate) use bundle::select_bundle_item;
use commands::*;
pub(crate) use commands::{
    queue_block_entity_tag_query_command, queue_change_difficulty_command,
    queue_change_game_mode_command, queue_chat_command, queue_chat_message_command,
    queue_command_suggestion_request, queue_container_button_click_command,
    queue_container_click_command, queue_container_close_request_command,
    queue_container_slot_state_changed_command, queue_edit_book_command,
    queue_entity_tag_query_command, queue_jigsaw_generate_command, queue_lock_difficulty_command,
    queue_perform_respawn_command, queue_place_recipe_command, queue_player_abilities_command,
    queue_recipe_book_change_settings_command, queue_recipe_book_seen_recipe_command,
    queue_rename_item_command, queue_request_game_rule_values_command, queue_request_stats_command,
    queue_seen_advancements_command, queue_select_trade_command, queue_set_beacon_command,
    queue_set_command_block_command, queue_set_creative_mode_slot_command,
    queue_set_game_rule_command, queue_set_jigsaw_block_command, queue_set_structure_block_command,
    queue_set_test_block_command, queue_sign_update_command, queue_spectate_entity_command,
    queue_teleport_to_entity_command, queue_vehicle_move_command, select_hotbar_slot,
};
use component_patch::{encode_added_component_value_snbt_with_context, PersistentSnbtContext};
pub(crate) use game_rules::{GameRuleSearchEntry, GameRulesInteractionRow, GameRulesScreenFocus};
#[cfg(test)]
pub(crate) use inventory::handle_inventory_mouse_input;
pub(crate) use inventory::{
    anvil_rename_entry_consumes_key, handle_inventory_cursor_moved, handle_inventory_key_input,
    handle_inventory_mouse_input_with_item_runtime, handle_inventory_mouse_wheel,
    handle_inventory_text_input, inventory_screen_layout, inventory_screen_layout_for_surface,
    inventory_screen_selected_hotbar_slot_id, maybe_close_narrow_recipe_book,
    recipe_book_button_position, recipe_book_main_gui_offset,
    recipe_book_search_entry_consumes_key, recipe_book_tab_count_for_background,
    recipe_book_type_for_background, recipe_book_type_settings, recipe_book_visible_tab_indices,
    sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
    sync_stonecutter_recipe_scroll_state, InventoryScreenBackground, InventorySlotLayout,
    LOOM_NO_ITEM_REQUIRED_PATTERN_COUNT, LOOM_PATTERN_BUTTON_COLUMNS, LOOM_PATTERN_BUTTON_ROWS,
    LOOM_PATTERN_BUTTON_SIZE, LOOM_PATTERN_BUTTON_X, LOOM_PATTERN_BUTTON_Y,
    LOOM_PATTERN_ITEM_PATTERN_COUNT, LOOM_SCROLLER_HEIGHT, LOOM_SCROLLER_MAX_OFFSET,
    LOOM_SCROLLER_WIDTH, LOOM_SCROLLER_X, LOOM_SCROLLER_Y, LOOM_SELECTED_PATTERN_DATA_ID,
    RECIPE_BOOK_BUTTON_HEIGHT, RECIPE_BOOK_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_HEIGHT,
    RECIPE_BOOK_FILTER_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y,
    RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X, RECIPE_BOOK_PAGE_BUTTON_HEIGHT,
    RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
    RECIPE_BOOK_RECIPE_BUTTON_COLUMNS, RECIPE_BOOK_RECIPE_BUTTON_SIZE, RECIPE_BOOK_RECIPE_BUTTON_X,
    RECIPE_BOOK_RECIPE_BUTTON_Y, RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH,
    RECIPE_BOOK_SEARCH_BOX_X, RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
    RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET, RECIPE_BOOK_SELECTED_TAB_X_OFFSET, RECIPE_BOOK_TAB_HEIGHT,
    RECIPE_BOOK_TAB_STRIDE_Y, RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X, RECIPE_BOOK_TAB_Y,
    STONECUTTER_RECIPE_BUTTON_COLUMNS, STONECUTTER_RECIPE_BUTTON_HEIGHT,
    STONECUTTER_RECIPE_BUTTON_ROWS, STONECUTTER_RECIPE_BUTTON_SPRITE_Y,
    STONECUTTER_RECIPE_BUTTON_WIDTH, STONECUTTER_RECIPE_BUTTON_X, STONECUTTER_RECIPE_ITEM_Y,
    STONECUTTER_SCROLLER_HEIGHT, STONECUTTER_SCROLLER_MAX_OFFSET, STONECUTTER_SCROLLER_WIDTH,
    STONECUTTER_SCROLLER_X, STONECUTTER_SCROLLER_Y, STONECUTTER_SELECTED_RECIPE_DATA_ID,
    STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT,
};
use minecart_command_block::MinecartCommandBlockEditorInputState;
pub(crate) use mouse::{
    advance_destroying_block_at_partial_tick, advance_using_item_at_partial_tick,
    handle_mouse_input_at_partial_tick, handle_mouse_motion, handle_mouse_wheel,
};
#[cfg(test)]
pub(crate) use movement::advance_player_input;
pub(crate) use movement::advance_player_input_for_client_ticks;

pub(crate) trait TerrainReloadRequestSink {
    fn request_reload_all_chunks(&mut self);
}

impl TerrainReloadRequestSink for TerrainUploadState {
    fn request_reload_all_chunks(&mut self) {
        TerrainUploadState::request_reload_all_chunks(self);
    }
}

const CREATIVE_FLIGHT_JUMP_TRIGGER_TICKS: u8 = 7;
const CREATIVE_FLIGHT_TICK_SECONDS: f64 = 0.05;
const SPRINT_TRIGGER_TICKS: u8 = 7;
const SPRINT_TRIGGER_TICK_SECONDS: f64 = 0.05;
const DEBUG_CRASH_TIME: Duration = Duration::from_secs(10);
const DEBUG_CRASH_REPORT_INTERVAL: Duration = Duration::from_secs(1);
const CHAT_ENTRY_MAX_LENGTH: usize = 256;
pub(crate) const DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH: &str = "screenshots/debug";
pub(crate) const DEBUG_PROFILING_RESULTS_RELATIVE_DIR: &str = "debug/profiling";
const VANILLA_DEBUG_FEEDBACK_COLOR: u32 = 0xFFFF55;
const DEBUG_OPTIONS_HEADER_HEIGHT: i32 = 61;
const DEBUG_OPTIONS_FOOTER_HEIGHT: i32 = 33;
const DEBUG_OPTIONS_ROW_WIDTH: i32 = 350;
const DEBUG_OPTIONS_ROW_HEIGHT: i32 = 20;
const DEBUG_OPTIONS_STATUS_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH: i32 = 120;
const DEBUG_OPTIONS_DONE_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_FOOTER_BUTTON_SPACING: i32 = 8;
const DEBUG_OPTIONS_SCROLLBAR_WIDTH: i32 = 6;
const DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT: i32 = 32;
const DEBUG_OPTIONS_SEARCH_WIDTH: i32 = DEBUG_OPTIONS_ROW_WIDTH / 3;
const DEBUG_OPTIONS_SEARCH_HEIGHT: i32 = 20;
const DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET: i32 = 4;
const DEBUG_OPTIONS_SEARCH_INNER_WIDTH: i32 = DEBUG_OPTIONS_SEARCH_WIDTH - 8;
const DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE: i32 = 6;
const DEBUG_OPTIONS_SEARCH_MAX_LENGTH: usize = 32;
const DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);
const DIALOG_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);
const DIALOG_IME_PREEDIT_MAX_CHARS: usize = 256;
const DIALOG_IME_PREEDIT_CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(300);

pub(crate) trait DebugOptionsSearchTextMeasurer {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        let _ = (search_text, scroll_to, width);
        0
    }

    fn debug_options_search_cursor_for_text_offset_from_display_start(
        &self,
        search_text: &str,
        display_start: usize,
        offset: i32,
    ) -> usize {
        debug_options_search_cursor_for_text_offset_from_display_start_fallback(
            search_text,
            display_start,
            offset,
        )
    }
}

#[cfg(test)]
pub(crate) struct DebugOptionsFallbackSearchTextMeasurer;

#[cfg(test)]
impl DebugOptionsSearchTextMeasurer for DebugOptionsFallbackSearchTextMeasurer {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        debug_options_search_display_start_for_width_fallback(search_text, scroll_to, width)
    }
}

const PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH: i32 = 204;
const PAUSE_SCREEN_HALF_BUTTON_WIDTH: i32 = 98;
const PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT: i32 = 20;
const PAUSE_SCREEN_RETURN_TO_GAME_TOP_OFFSET: i32 = 8;
const PAUSE_SCREEN_SECOND_ROW_TOP_OFFSET: i32 = 32;
const PAUSE_SCREEN_THIRD_ROW_TOP_OFFSET: i32 = 56;
const PAUSE_SCREEN_FOURTH_ROW_TOP_OFFSET: i32 = 80;
const PAUSE_SCREEN_DISCONNECT_ROW_TOP_OFFSET: i32 = 128;
const STATS_SCREEN_DONE_BUTTON_WIDTH: i32 = 200;
const STATS_SCREEN_DONE_BUTTON_HEIGHT: i32 = 20;
const STATS_SCREEN_FOOTER_HEIGHT: i32 = 33;
const STATS_SCREEN_ROW_HEIGHT: i32 = 14;
const STATS_SCREEN_TAB_Y: i32 = 22;
const STATS_SCREEN_TAB_HEIGHT: i32 = 20;
const STATS_SCREEN_LIST_WIDTH: i32 = 280;
const STATS_SCREEN_LIST_TOP: i32 = 46;
const STATS_SCREEN_LIST_ENTRY_PADDING: i32 = 2;
const STATS_SCREEN_SCROLLBAR_WIDTH: i32 = 6;
const STATS_SCREEN_ITEM_COLUMN_FIRST_X: i32 = 57;
const STATS_SCREEN_ITEM_COLUMN_STRIDE: i32 = 40;
const STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE: i32 = 18;
const PAUSE_SCREEN_RELEASE_FEEDBACK_URL: &str = "https://aka.ms/javafeedback?ref=game";
const PAUSE_SCREEN_SNAPSHOT_FEEDBACK_URL: &str = "https://aka.ms/snapshotfeedback?ref=game";
const PAUSE_SCREEN_SNAPSHOT_BUGS_URL: &str = "https://aka.ms/snapshotbugs?ref=game";
/// `flipped` is changed only by the server branches of old/new minecart
/// movement and is not synchronized, so a remote client retains `false`.
const MINECART_CLIENT_FLIPPED_ROTATION: bool = false;
/// `MinecartHopper.enabled` changes only from the server-side activator-rail
/// callback or NBT load and is not synchronized to the client.
const TNT_MINECART_DEFAULT_FUSE: i32 = -1;
const AGEABLE_MOB_CLIENT_ADULT_AGE: i32 = 1;
const AGEABLE_MOB_CLIENT_BABY_AGE: i32 = -1;
const LLAMA_MAX_VARIANT: i32 = 3;
const SLIME_MIN_SIZE: i32 = 1;
const SLIME_MAX_SIZE: i32 = 127;
const RAVAGER_DEFAULT_ATTACK_TICK: i32 = 0;
const RAVAGER_DEFAULT_STUN_TICK: i32 = 0;
const RAVAGER_DEFAULT_ROAR_TICK: i32 = 0;
const STANDING_SIGN_MAX_LINE_WIDTH: u32 = 90;
const HANGING_SIGN_MAX_LINE_WIDTH: u32 = 60;
const DEBUG_GAME_MODE_SWITCHER_SLOT_AREA: i32 = 26;
const DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED: i32 = 31;
const DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH: i32 = 4 * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED - 5;
const DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET: i32 = 31;
const ADVANCEMENTS_FOOTER_HEIGHT: i32 = 33;
const ADVANCEMENTS_WINDOW_WIDTH: i32 = 252;
const ADVANCEMENTS_WINDOW_HEIGHT: i32 = 140;
const ADVANCEMENTS_DONE_BUTTON_WIDTH: i32 = 200;
const ADVANCEMENTS_DONE_BUTTON_HEIGHT: i32 = 20;
const ADVANCEMENTS_TAB_ABOVE_WIDTH: i32 = 28;
const ADVANCEMENTS_TAB_ABOVE_HEIGHT: i32 = 32;
const ADVANCEMENTS_TAB_ABOVE_MAX: usize = 8;
const ADVANCEMENTS_TAB_BELOW_WIDTH: i32 = 28;
const ADVANCEMENTS_TAB_BELOW_HEIGHT: i32 = 32;
const ADVANCEMENTS_TAB_BELOW_MAX: usize = 8;
const ADVANCEMENTS_TAB_LEFT_WIDTH: i32 = 32;
const ADVANCEMENTS_TAB_LEFT_HEIGHT: i32 = 28;
const ADVANCEMENTS_TAB_LEFT_MAX: usize = 5;
const ADVANCEMENTS_TAB_RIGHT_WIDTH: i32 = 32;
const ADVANCEMENTS_TAB_RIGHT_HEIGHT: i32 = 28;
const ADVANCEMENTS_TAB_RIGHT_MAX: usize = 5;

pub(crate) trait DebugClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool;

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        None
    }
}

struct DebugNetContext<'a> {
    counters: &'a mut NetCounters,
    net_commands: &'a Option<mpsc::Sender<NetCommand>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DebugGameModeSwitcherState {
    selected: GameType,
    first_mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DebugPauseScreenState {
    pub(crate) show_pause_menu: bool,
    pub(crate) cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PauseScreenLinkRequest {
    SendFeedback { url: &'static str },
    ReportBugs { url: &'static str },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DebugOptionsScreenState {
    search_text: String,
    search_cursor: usize,
    search_selection: usize,
    search_selecting: bool,
    last_left_click_at: Option<Instant>,
    scroll_row: usize,
    scrollbar_drag_y: Option<i32>,
    cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenHudState {
    pub(crate) search_text: String,
    pub(crate) search_cursor: usize,
    pub(crate) search_selection: usize,
    pub(crate) rows: Vec<DebugOptionsScreenHudRow>,
    pub(crate) tooltip: Option<DebugOptionsScreenTooltip>,
    pub(crate) scroll_row: usize,
    pub(crate) total_rows: usize,
    pub(crate) visible_rows: usize,
    pub(crate) default_profile_active: bool,
    pub(crate) default_profile_hovered: bool,
    pub(crate) performance_profile_active: bool,
    pub(crate) performance_profile_hovered: bool,
    pub(crate) done_hovered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DebugOptionsScreenHudRow {
    Category {
        label: String,
    },
    Entry {
        entry: DebugScreenEntryId,
        path: String,
        status: DebugScreenEntryStatus,
        hovered_status: Option<DebugScreenEntryStatus>,
        allowed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenTooltip {
    pub(crate) text: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugOptionsScreenRow {
    Category { label: &'static str },
    Entry(DebugScreenEntryId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugFrustumRequest {
    Capture,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugFeatureCountRequest {
    Log,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugProfilingToggleRequest {
    Start,
    Stop,
}

/// Focus path owned by the native StatsScreen interaction model. This mirrors the
/// concrete vanilla widgets without leaking localized labels into input state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum StatsScreenFocus {
    #[default]
    None,
    Tabs,
    List,
    GeneralRow(usize),
    ItemHeader(usize),
    ItemRow(usize),
    MobRow(usize),
    Done,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientInputState {
    focused: bool,
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint: bool,
    destroy_block_held: bool,
    use_item_held: bool,
    pending_local_player_action_particles: Vec<LocalPlayerActionParticleState>,
    shift_left_down: bool,
    shift_right_down: bool,
    control_left_down: bool,
    control_right_down: bool,
    pressed_keys: HashSet<KeyCode>,
    mouse_left_down: bool,
    mouse_right_down: bool,
    mouse_middle_down: bool,
    attack_miss_time_ticks: u16,
    use_item_repeat_delay_ticks: u8,
    mouse_delta_x: f64,
    mouse_delta_y: f64,
    scroll_accumulated_x: f64,
    scroll_accumulated_y: f64,
    bundle_scroll_accumulated_x: f64,
    bundle_scroll_accumulated_y: f64,
    inventory_hovered_slot: Option<i16>,
    inventory_cursor_position: Option<(i32, i32)>,
    inventory_last_click_slot: Option<i16>,
    inventory_last_click_button_num: Option<i8>,
    inventory_last_click_at: Option<Instant>,
    dialog_last_click_button: Option<MouseButton>,
    dialog_last_click_at: Option<Instant>,
    dialog_last_click_screen_revision: Option<u64>,
    dialog_ime_preedit: Option<DialogImePreeditState>,
    client_text_filtering_enabled: bool,
    book_input: Option<BookInputState>,
    dialog_screen_was_open: bool,
    dialog_native_previous_screen: Option<NativeDialogPreviousScreenState>,
    inventory_quick_craft_button_num: Option<i8>,
    inventory_quick_craft_slots: Vec<i16>,
    stonecutter_recipe_scroll_row: i32,
    stonecutter_recipe_scroll_input_item_id: Option<i32>,
    stonecutter_recipe_scrolling: bool,
    loom_pattern_scroll_row: i32,
    loom_pattern_scrolling: bool,
    loom_pattern_selection_container_id: Option<i32>,
    loom_pattern_selection_dirty: bool,
    loom_selected_pattern_index: Option<i32>,
    beacon_effect_selection_container_id: Option<i32>,
    beacon_effect_selection_dirty: bool,
    beacon_primary_effect: Option<i32>,
    beacon_secondary_effect: Option<i32>,
    anvil_rename_input: Option<AnvilRenameInputSignature>,
    anvil_rename_text: String,
    anvil_rename_cursor: usize,
    anvil_rename_selection: usize,
    anvil_rename_hover_name: String,
    recipe_book_search_text: String,
    recipe_book_search_cursor: usize,
    recipe_book_search_selection: usize,
    recipe_book_search_focused: bool,
    recipe_book_search_suppress_open_key_commit: bool,
    recipe_book_crafting_tab_index: usize,
    recipe_book_furnace_tab_index: usize,
    recipe_book_blast_furnace_tab_index: usize,
    recipe_book_smoker_tab_index: usize,
    recipe_book_crafting_page: usize,
    recipe_book_furnace_page: usize,
    recipe_book_blast_furnace_page: usize,
    recipe_book_smoker_page: usize,
    recipe_book_overlay: Option<RecipeBookOverlayHudState>,
    recipe_book_last_placed_recipe: Option<(i32, i32)>,
    advancement_scroll_deltas: BTreeMap<String, (f64, f64)>,
    advancement_hover_fade: f32,
    advancement_mouse_left_down: bool,
    advancement_is_scrolling: bool,
    stats_screen_cursor_position: Option<(i32, i32)>,
    stats_screen_scroll_amount: f64,
    stats_screen_max_scroll_amount: f64,
    stats_screen_scroller_height: i32,
    stats_screen_scrolling: bool,
    stats_screen_tab: usize,
    stats_screen_item_sort_column: Option<usize>,
    stats_screen_item_sort_order: i8,
    stats_screen_item_effective_sort: Option<(usize, i8)>,
    stats_screen_tab_row_counts: [usize; 3],
    stats_screen_total_rows: usize,
    stats_screen_focus: StatsScreenFocus,
    stats_screen_was_loading: bool,
    game_rules_screen: Option<game_rules::GameRulesScreenInputState>,
    pause_screen_link_requests: Vec<PauseScreenLinkRequest>,
    pause_screen_disconnect_requested: bool,
    debug_entries: DebugScreenEntryList,
    debug_profile_store_path: Option<PathBuf>,
    debug_modifier_down: bool,
    debug_modifier_used: bool,
    debug_profiler_chart_visible: bool,
    debug_fps_charts_visible: bool,
    debug_network_charts_visible: bool,
    debug_lightmap_texture_visible: bool,
    debug_advanced_item_tooltips: bool,
    debug_show_local_server_entity_hit_boxes: bool,
    debug_hotkeys_enabled: bool,
    debug_feature_count_enabled: bool,
    remote_debug_renderers: crate::remote_debug::RemoteDebugRendererSelection,
    debug_fog_enabled: bool,
    debug_smart_cull_enabled: bool,
    debug_wireframe_enabled: bool,
    /// Vanilla `Options.hideGui`, toggled by the default `key.toggleGui` (F1).
    hide_gui: bool,
    debug_frustum_requests: Vec<DebugFrustumRequest>,
    debug_feature_count_requests: Vec<DebugFeatureCountRequest>,
    debug_pause_on_lost_focus: bool,
    debug_resource_pack_reload_requests: u32,
    debug_dynamic_texture_dump_requests: u32,
    debug_profiling_recording: bool,
    debug_profiling_toggle_requests: Vec<DebugProfilingToggleRequest>,
    debug_profiler_chart_navigation_requests: Vec<u8>,
    debug_options_screen_requests: u32,
    debug_pause_without_menu_requests: u32,
    debug_options_screen: Option<DebugOptionsScreenState>,
    debug_pause_screen: Option<DebugPauseScreenState>,
    debug_game_mode_switcher: Option<DebugGameModeSwitcherState>,
    debug_recreate_server_query_requests: Vec<DebugRecreateServerQueryRequest>,
    pending_debug_recreate_server_query: Option<PendingDebugRecreateServerQuery>,
    debug_query_transaction_id: i32,
    debug_crash_started_at: Option<Instant>,
    debug_crash_last_reported_at: Option<Instant>,
    debug_crash_report_count: u32,
    sign_editor: Option<SignEditorInputState>,
    dismissed_sign_editor: Option<SignEditorInputSignature>,
    sign_editor_cursor_position: Option<(i32, i32)>,
    sign_editor_ime_preedit: Option<SignEditorImePreeditState>,
    merchant_trade_scrolling: bool,
    merchant_focused_trade_row: Option<i32>,
    chat_entry: Option<ChatEntryState>,
    chat_scroll_lines: usize,
    structure_block_editor: Option<structure_block::StructureBlockEditorInputState>,
    jigsaw_block_editor: Option<jigsaw_block::JigsawBlockEditorInputState>,
    test_block_editor: Option<test_block::TestBlockEditorInputState>,
    test_instance_block_editor: Option<test_instance_block::TestInstanceBlockEditorInputState>,
    #[cfg(test)]
    last_step: Option<Instant>,
    #[cfg(test)]
    local_player_movement_tick_accumulator_seconds: f64,
    last_move_command_at: Option<Instant>,
    move_position_reminder_ticks: u32,
    last_move_command_pose: Option<LocalPlayerPoseState>,
    last_vehicle_command_at: Option<Instant>,
    riding_jump_charge_seconds: Option<f64>,
    sprint_trigger_ticks: u8,
    sprint_trigger_elapsed_seconds: f64,
    creative_flight_jump_trigger_ticks: u8,
    creative_flight_jump_trigger_elapsed_seconds: f64,
    spectator_gui: spectator_menu::SpectatorGuiInputState,
    death_screen: death_screen::DeathScreenInputState,
    game_event_screen: game_event_screen::GameEventScreenInputState,
}

/// Immutable input projection consumed by render and event-loop readers.
/// Gameplay movement accumulators, outbound request queues, click timing, and
/// crash/query state remain owned only by the simulation-side input owner.
#[derive(Debug, Clone)]
pub(crate) struct RenderInputPublication(ClientInputState);

impl RenderInputPublication {
    pub(crate) fn as_state(&self) -> &ClientInputState {
        &self.0
    }
}

impl std::ops::Deref for RenderInputPublication {
    type Target = ClientInputState;

    fn deref(&self) -> &Self::Target {
        self.as_state()
    }
}

#[derive(Debug, Clone)]
struct DialogImePreeditState {
    text: String,
    caret: Option<usize>,
    focused_start: usize,
    focused_end: usize,
    screen_revision: u64,
    initialized_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogImePreeditHudState {
    pub(crate) text: String,
    pub(crate) caret: Option<usize>,
    pub(crate) focused_start: usize,
    pub(crate) focused_end: usize,
    pub(crate) screen_revision: u64,
    pub(crate) caret_visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChatEntryState {
    text: String,
    cursor: usize,
    selection: usize,
    last_suggestion_request_text: Option<String>,
    suggestion_list: Option<ChatSuggestionListState>,
    hidden_suggestion_id: Option<i32>,
    suggestion_narration: Option<CommandSuggestionNarrationState>,
    mouse_position: Option<(i32, i32)>,
    suppress_open_key_commit: bool,
    owner: ChatEntryOwner,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum ChatEntryOwner {
    #[default]
    Chat,
    CommandBlock(CommandBlockEditorInputState),
    MinecartCommandBlock(MinecartCommandBlockEditorInputState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandBlockEditorInputState {
    open_count: usize,
    authoritative_revision: usize,
    pos: BlockPos,
    mode: CommandBlockMode,
    conditional: bool,
    automatic: bool,
    track_output: bool,
    previous_output: String,
    controls_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeDialogPreviousScreenState {
    Chat(ChatEntryState),
    DebugOptions(DebugOptionsScreenState),
    DebugPause {
        screen: DebugPauseScreenState,
        disconnect_requested: bool,
    },
    Sign(SignEditorInputState),
}

const COMMAND_SUGGESTION_LINE_HEIGHT: i32 = 12;
const COMMAND_SUGGESTION_LINE_LIMIT: usize = 10;
const COMMAND_BLOCK_SUGGESTION_LINE_LIMIT: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatSuggestionListState {
    result_id: i32,
    result: CommandSuggestionsResultState,
    original_contents: String,
    sorted_indices: Vec<usize>,
    current: usize,
    offset: usize,
    tab_cycles: bool,
    rect: Option<CommandSuggestionRect>,
}

/// Language-neutral payload for vanilla's translatable suggestion narration.
/// A future narrator owner can localize `narration.suggestion*` without the
/// input owner pretending that an English string was spoken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionNarrationState {
    pub(crate) position: usize,
    pub(crate) total: usize,
    pub(crate) text: String,
    pub(crate) tooltip: Option<String>,
    pub(crate) tooltip_translation_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandSuggestionRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl CommandSuggestionRect {
    fn contains(self, x: i32, y: i32) -> bool {
        let right = self.x.saturating_add_unsigned(self.width);
        let bottom = self.y.saturating_add_unsigned(self.height);
        x >= self.x && x <= right && y >= self.y && y <= bottom
    }

    fn hovered_line(self, x: i32, y: i32) -> Option<usize> {
        let right = self.x.saturating_add_unsigned(self.width);
        if x <= self.x || x >= right {
            return None;
        }
        let lines = usize::try_from(self.height / COMMAND_SUGGESTION_LINE_HEIGHT as u32).ok()?;
        (0..lines).find(|line| {
            let top = self.y.saturating_add(
                i32::try_from(*line)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(COMMAND_SUGGESTION_LINE_HEIGHT),
            );
            y > top && y < top.saturating_add(COMMAND_SUGGESTION_LINE_HEIGHT)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionHudEntry {
    pub(crate) text: String,
    pub(crate) selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionHudTooltip {
    pub(crate) text: String,
    pub(crate) translation_key: Option<String>,
    pub(crate) runs: Vec<StyledTextRun>,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandSuggestionHudState {
    pub(crate) input_text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) suffix: Option<String>,
    pub(crate) rect: CommandSuggestionRect,
    pub(crate) entries: Vec<CommandSuggestionHudEntry>,
    pub(crate) has_previous: bool,
    pub(crate) has_next: bool,
    pub(crate) tooltip: Option<CommandSuggestionHudTooltip>,
    pub(crate) narration: Option<CommandSuggestionNarrationState>,
    pub(crate) command_block_layout: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandBlockEditorHudState {
    pub(crate) mode: CommandBlockMode,
    pub(crate) conditional: bool,
    pub(crate) automatic: bool,
    pub(crate) track_output: bool,
    pub(crate) previous_output: String,
    pub(crate) controls_enabled: bool,
    pub(crate) mouse_position: Option<(i32, i32)>,
}

pub(crate) trait CommandSuggestionTextMeasurer {
    fn command_suggestion_text_width(&self, text: &str) -> u32;
    fn command_suggestion_display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize;
    fn command_suggestion_cursor_for_width(
        &self,
        text: &str,
        display_start: usize,
        width: u32,
    ) -> usize;
}

impl CommandSuggestionTextMeasurer for bbb_renderer::Renderer {
    fn command_suggestion_text_width(&self, text: &str) -> u32 {
        self.hud_plain_text_width(text)
    }

    fn command_suggestion_display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        self.hud_plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or(0)
    }

    fn command_suggestion_cursor_for_width(
        &self,
        text: &str,
        display_start: usize,
        width: u32,
    ) -> usize {
        self.hud_plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or(display_start)
    }
}

pub(crate) struct FallbackTextMeasurer;

impl CommandSuggestionTextMeasurer for FallbackTextMeasurer {
    fn command_suggestion_text_width(&self, text: &str) -> u32 {
        u32::try_from(text.chars().count())
            .unwrap_or(u32::MAX)
            .saturating_mul(6)
    }

    fn command_suggestion_display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        scroll_to
            .min(text.chars().count())
            .saturating_sub(usize::try_from(width / 6).unwrap_or(usize::MAX))
    }

    fn command_suggestion_cursor_for_width(
        &self,
        text: &str,
        display_start: usize,
        width: u32,
    ) -> usize {
        display_start
            .saturating_add(usize::try_from(width / 6).unwrap_or(usize::MAX))
            .min(text.chars().count())
    }
}

impl ChatEntryState {
    fn commands_only(&self) -> bool {
        matches!(
            self.owner,
            ChatEntryOwner::CommandBlock(_) | ChatEntryOwner::MinecartCommandBlock(_)
        )
    }

    fn suggestion_line_limit(&self) -> usize {
        if self.commands_only() {
            COMMAND_BLOCK_SUGGESTION_LINE_LIMIT
        } else {
            COMMAND_SUGGESTION_LINE_LIMIT
        }
    }

    fn max_utf16_units(&self) -> usize {
        if self.commands_only() {
            MAX_COMMAND_BLOCK_COMMAND_UTF16_UNITS
        } else {
            CHAT_ENTRY_MAX_LENGTH
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnvilRenameInputSignature {
    container_id: i32,
    item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignEditorInputSignature {
    open_count: usize,
    pos: ProtocolBlockPos,
    is_front_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignEditorInputState {
    signature: SignEditorInputSignature,
    lines: [String; 4],
    line: usize,
    cursor: usize,
    selection: usize,
    max_line_width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignEditorHudState {
    pub(crate) pos: BlockPos,
    pub(crate) is_front_text: bool,
    pub(crate) lines: [String; 4],
    pub(crate) line: usize,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) preedit: Option<SignEditorImePreeditHudState>,
}

#[derive(Debug, Clone)]
struct SignEditorImePreeditState {
    text: String,
    caret: Option<usize>,
    focused_start: usize,
    focused_end: usize,
    signature: SignEditorInputSignature,
    initialized_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignEditorImePreeditHudState {
    pub(crate) text: String,
    pub(crate) caret: Option<usize>,
    pub(crate) focused_start: usize,
    pub(crate) focused_end: usize,
    pub(crate) caret_visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookSearchHudState {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) focused: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookTabSelectionHudState {
    pub(crate) crafting: usize,
    pub(crate) furnace: usize,
    pub(crate) blast_furnace: usize,
    pub(crate) smoker: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookPageHudState {
    pub(crate) crafting: usize,
    pub(crate) furnace: usize,
    pub(crate) blast_furnace: usize,
    pub(crate) smoker: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecipeBookOverlayHudState {
    pub(crate) book_type: RecipeBookType,
    pub(crate) tab_index: usize,
    pub(crate) page_index: usize,
    pub(crate) button_index: usize,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugRecreateServerQueryRequest {
    BlockEntityTag {
        transaction_id: i32,
        pos: ProtocolBlockPos,
    },
    EntityTag {
        transaction_id: i32,
        entity_id: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugRecreateTarget {
    Block(BlockPos),
    Entity(i32),
}

#[derive(Debug, Clone, PartialEq)]
struct PendingDebugRecreateServerQuery {
    transaction_id: i32,
    client_level_generation: u64,
    target: PendingDebugRecreateServerQueryTarget,
}

#[derive(Debug, Clone, PartialEq)]
enum PendingDebugRecreateServerQueryTarget {
    Block {
        pos: BlockPos,
        description: String,
    },
    Entity {
        entity_id: i32,
        entity_uuid: uuid::Uuid,
        entity_type: String,
        position: [f64; 3],
    },
}

impl ClientInputState {
    fn advance_attack_miss_time(&mut self, world: &WorldStore, client_ticks: u32) {
        if client_ticks == 0 {
            return;
        }
        if self.gameplay_input_screen_is_open(world) {
            self.attack_miss_time_ticks = 10_000;
            return;
        }
        self.attack_miss_time_ticks = self
            .attack_miss_time_ticks
            .saturating_sub(client_ticks.min(u32::from(u16::MAX)) as u16);
    }

    fn gameplay_input_screen_is_open(&self, world: &WorldStore) -> bool {
        self.command_entry_is_active()
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open()
            || self.debug_game_mode_switcher_is_open()
            || world.open_container_id().is_some()
            || world.dialog_screen_is_open()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.game_rules_screen().is_some()
            || world.death_screen().is_some()
            || world.game_event_screen().is_some()
            || world.command_block_editor().is_some()
            || world.minecart_command_block_editor().is_some()
            || world.structure_block_editor().is_some()
            || world.jigsaw_block_editor().is_some()
            || world.test_block_editor().is_some()
            || world.test_instance_block_editor().is_some()
    }

    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
            debug_pause_on_lost_focus: true,
            debug_fog_enabled: true,
            debug_smart_cull_enabled: true,
            debug_query_transaction_id: -1,
            ..Self::default()
        }
    }

    fn queue_local_block_attack_effect(&mut self, effect: LocalBlockAttackEffect) {
        self.pending_local_player_action_particles
            .push(LocalPlayerActionParticleState::BlockAttack(effect));
    }

    fn queue_local_destroy_hit_particles(&mut self, state: LocalDestroyHitParticleState) {
        self.pending_local_player_action_particles
            .push(LocalPlayerActionParticleState::DestroyHit(state));
    }

    pub(crate) fn take_pending_local_player_action_particles(
        &mut self,
    ) -> Vec<LocalPlayerActionParticleState> {
        std::mem::take(&mut self.pending_local_player_action_particles)
    }

    pub(crate) fn set_client_text_filtering_enabled(&mut self, enabled: bool) {
        self.client_text_filtering_enabled = enabled;
    }

    pub(crate) fn client_text_filtering_enabled(&self) -> bool {
        self.client_text_filtering_enabled
    }

    /// Builds the immutable subset consumed by render and event-loop readers.
    ///
    /// This is intentionally an explicit projection instead of `self.clone()`:
    /// movement accumulators, click timers, outbound command requests, crash
    /// state, and query transactions remain private to the mutable simulation
    /// owner and therefore cannot add heap work to every publication.
    pub(crate) fn render_publication(&self) -> RenderInputPublication {
        RenderInputPublication(Self {
            focused: self.focused,
            shift_left_down: self.shift_left_down,
            shift_right_down: self.shift_right_down,
            control_left_down: self.control_left_down,
            control_right_down: self.control_right_down,
            pressed_keys: self.pressed_keys.clone(),
            mouse_left_down: self.mouse_left_down,
            mouse_right_down: self.mouse_right_down,
            mouse_middle_down: self.mouse_middle_down,
            inventory_hovered_slot: self.inventory_hovered_slot,
            inventory_cursor_position: self.inventory_cursor_position,
            dialog_ime_preedit: self.dialog_ime_preedit.clone(),
            client_text_filtering_enabled: self.client_text_filtering_enabled,
            book_input: self.book_input.clone(),
            dialog_native_previous_screen: self.dialog_native_previous_screen.clone(),
            inventory_quick_craft_button_num: self.inventory_quick_craft_button_num,
            inventory_quick_craft_slots: self.inventory_quick_craft_slots.clone(),
            stonecutter_recipe_scroll_row: self.stonecutter_recipe_scroll_row,
            loom_pattern_scroll_row: self.loom_pattern_scroll_row,
            loom_selected_pattern_index: self.loom_selected_pattern_index,
            beacon_primary_effect: self.beacon_primary_effect,
            beacon_secondary_effect: self.beacon_secondary_effect,
            anvil_rename_input: self.anvil_rename_input.clone(),
            anvil_rename_text: self.anvil_rename_text.clone(),
            anvil_rename_cursor: self.anvil_rename_cursor,
            anvil_rename_selection: self.anvil_rename_selection,
            anvil_rename_hover_name: self.anvil_rename_hover_name.clone(),
            recipe_book_search_text: self.recipe_book_search_text.clone(),
            recipe_book_search_cursor: self.recipe_book_search_cursor,
            recipe_book_search_selection: self.recipe_book_search_selection,
            recipe_book_search_focused: self.recipe_book_search_focused,
            recipe_book_crafting_tab_index: self.recipe_book_crafting_tab_index,
            recipe_book_furnace_tab_index: self.recipe_book_furnace_tab_index,
            recipe_book_blast_furnace_tab_index: self.recipe_book_blast_furnace_tab_index,
            recipe_book_smoker_tab_index: self.recipe_book_smoker_tab_index,
            recipe_book_crafting_page: self.recipe_book_crafting_page,
            recipe_book_furnace_page: self.recipe_book_furnace_page,
            recipe_book_blast_furnace_page: self.recipe_book_blast_furnace_page,
            recipe_book_smoker_page: self.recipe_book_smoker_page,
            recipe_book_overlay: self.recipe_book_overlay,
            advancement_scroll_deltas: self.advancement_scroll_deltas.clone(),
            advancement_hover_fade: self.advancement_hover_fade,
            stats_screen_cursor_position: self.stats_screen_cursor_position,
            stats_screen_scroll_amount: self.stats_screen_scroll_amount,
            stats_screen_max_scroll_amount: self.stats_screen_max_scroll_amount,
            stats_screen_scroller_height: self.stats_screen_scroller_height,
            stats_screen_scrolling: self.stats_screen_scrolling,
            stats_screen_tab: self.stats_screen_tab,
            stats_screen_item_sort_column: self.stats_screen_item_sort_column,
            stats_screen_item_sort_order: self.stats_screen_item_sort_order,
            stats_screen_item_effective_sort: self.stats_screen_item_effective_sort,
            stats_screen_tab_row_counts: self.stats_screen_tab_row_counts,
            stats_screen_total_rows: self.stats_screen_total_rows,
            stats_screen_focus: self.stats_screen_focus,
            stats_screen_was_loading: self.stats_screen_was_loading,
            game_rules_screen: self.game_rules_screen.clone(),
            pause_screen_disconnect_requested: self.pause_screen_disconnect_requested,
            debug_entries: self.debug_entries.clone(),
            debug_profiler_chart_visible: self.debug_profiler_chart_visible,
            debug_fps_charts_visible: self.debug_fps_charts_visible,
            debug_network_charts_visible: self.debug_network_charts_visible,
            debug_lightmap_texture_visible: self.debug_lightmap_texture_visible,
            debug_advanced_item_tooltips: self.debug_advanced_item_tooltips,
            debug_show_local_server_entity_hit_boxes: self.debug_show_local_server_entity_hit_boxes,
            remote_debug_renderers: self.remote_debug_renderers,
            debug_fog_enabled: self.debug_fog_enabled,
            debug_smart_cull_enabled: self.debug_smart_cull_enabled,
            debug_wireframe_enabled: self.debug_wireframe_enabled,
            debug_pause_on_lost_focus: self.debug_pause_on_lost_focus,
            debug_profiling_recording: self.debug_profiling_recording,
            debug_options_screen: self.debug_options_screen.clone(),
            debug_pause_screen: self.debug_pause_screen.clone(),
            debug_game_mode_switcher: self.debug_game_mode_switcher.clone(),
            sign_editor: self.sign_editor.clone(),
            dismissed_sign_editor: self.dismissed_sign_editor.clone(),
            sign_editor_cursor_position: self.sign_editor_cursor_position,
            sign_editor_ime_preedit: self.sign_editor_ime_preedit.clone(),
            merchant_trade_scrolling: self.merchant_trade_scrolling,
            merchant_focused_trade_row: self.merchant_focused_trade_row,
            chat_entry: self.chat_entry.clone(),
            chat_scroll_lines: self.chat_scroll_lines,
            structure_block_editor: self.structure_block_editor.clone(),
            jigsaw_block_editor: self.jigsaw_block_editor.clone(),
            test_block_editor: self.test_block_editor.clone(),
            test_instance_block_editor: self.test_instance_block_editor.clone(),
            riding_jump_charge_seconds: self.riding_jump_charge_seconds,
            spectator_gui: self.spectator_gui.clone(),
            death_screen: self.death_screen.clone(),
            game_event_screen: self.game_event_screen.clone(),
            ..Self::default()
        })
    }

    pub(crate) fn dialog_mouse_click_is_double(
        &mut self,
        button: MouseButton,
        state: ElementState,
        screen_revision: u64,
    ) -> bool {
        self.dialog_mouse_click_is_double_at(button, state, screen_revision, Instant::now())
    }

    fn dialog_mouse_click_is_double_at(
        &mut self,
        button: MouseButton,
        state: ElementState,
        screen_revision: u64,
        now: Instant,
    ) -> bool {
        if state != ElementState::Pressed {
            return false;
        }
        let double_click = self.dialog_last_click_button == Some(button)
            && self.dialog_last_click_screen_revision == Some(screen_revision)
            && self
                .dialog_last_click_at
                .and_then(|last| now.checked_duration_since(last))
                .is_some_and(|elapsed| elapsed < DIALOG_DOUBLE_CLICK_THRESHOLD);
        self.dialog_last_click_button = Some(button);
        self.dialog_last_click_at = Some(now);
        self.dialog_last_click_screen_revision = Some(screen_revision);
        double_click
    }

    pub(crate) fn handle_dialog_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        screen_revision: u64,
    ) -> bool {
        self.handle_dialog_ime_preedit_at(text, cursor_range, screen_revision, Instant::now())
    }

    fn handle_dialog_ime_preedit_at(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        screen_revision: u64,
        now: Instant,
    ) -> bool {
        if text.is_empty() {
            self.dialog_ime_preedit = None;
            return true;
        }
        let original_text = text;
        let text = original_text
            .chars()
            .filter(|ch| !ch.is_control())
            .take(DIALOG_IME_PREEDIT_MAX_CHARS)
            .collect::<String>();
        if text.is_empty() {
            self.dialog_ime_preedit = None;
            return true;
        }
        let text_len = text.chars().count();
        let (caret, focused_start, focused_end) =
            cursor_range.map_or((None, 0, text_len), |(begin, end)| {
                let begin = dialog_ime_preedit_char_index_for_byte(original_text, begin);
                let end = dialog_ime_preedit_char_index_for_byte(original_text, end);
                let start = begin.min(end);
                let finish = begin.max(end);
                if start == finish {
                    (Some(end), 0, text_len)
                } else {
                    (Some(end), start, finish)
                }
            });
        self.dialog_ime_preedit = Some(DialogImePreeditState {
            text,
            caret,
            focused_start,
            focused_end,
            screen_revision,
            initialized_at: now,
        });
        true
    }

    pub(crate) fn clear_dialog_ime_preedit(&mut self) {
        self.dialog_ime_preedit = None;
    }

    pub(crate) fn dialog_ime_preedit_hud_state(&self) -> Option<DialogImePreeditHudState> {
        self.dialog_ime_preedit_hud_state_at(Instant::now())
    }

    fn dialog_ime_preedit_hud_state_at(&self, now: Instant) -> Option<DialogImePreeditHudState> {
        let state = self.dialog_ime_preedit.as_ref()?;
        let elapsed = now
            .checked_duration_since(state.initialized_at)
            .unwrap_or_default();
        Some(DialogImePreeditHudState {
            text: state.text.clone(),
            caret: state.caret,
            focused_start: state.focused_start,
            focused_end: state.focused_end,
            screen_revision: state.screen_revision,
            caret_visible: (elapsed.as_millis()
                / DIALOG_IME_PREEDIT_CURSOR_BLINK_INTERVAL.as_millis())
                % 2
                == 0,
        })
    }

    fn clear_pressed(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint = false;
        self.destroy_block_held = false;
        self.use_item_held = false;
        self.pressed_keys.clear();
        self.mouse_left_down = false;
        self.mouse_right_down = false;
        self.mouse_middle_down = false;
        self.attack_miss_time_ticks = 0;
        self.use_item_repeat_delay_ticks = 0;
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.scroll_accumulated_x = 0.0;
        self.scroll_accumulated_y = 0.0;
        self.bundle_scroll_accumulated_x = 0.0;
        self.bundle_scroll_accumulated_y = 0.0;
        self.inventory_hovered_slot = None;
        self.inventory_cursor_position = None;
        self.inventory_last_click_slot = None;
        self.inventory_last_click_button_num = None;
        self.inventory_last_click_at = None;
        self.dialog_last_click_button = None;
        self.dialog_last_click_at = None;
        self.dialog_last_click_screen_revision = None;
        self.dialog_ime_preedit = None;
        self.sign_editor_cursor_position = None;
        self.sign_editor_ime_preedit = None;
        self.inventory_quick_craft_button_num = None;
        self.inventory_quick_craft_slots.clear();
        self.loom_pattern_scrolling = false;
        self.merchant_trade_scrolling = false;
        self.merchant_focused_trade_row = None;
        self.stonecutter_recipe_scrolling = false;
        self.anvil_rename_input = None;
        self.anvil_rename_text.clear();
        self.anvil_rename_cursor = 0;
        self.anvil_rename_selection = 0;
        self.anvil_rename_hover_name.clear();
        self.recipe_book_search_focused = false;
        self.recipe_book_search_suppress_open_key_commit = false;
        self.recipe_book_overlay = None;
        self.advancement_hover_fade = 0.0;
        self.advancement_mouse_left_down = false;
        self.advancement_is_scrolling = false;
        self.reset_debug_crash_hold();
        if self
            .chat_entry
            .as_ref()
            .is_none_or(|entry| !entry.commands_only())
        {
            self.chat_entry = None;
        }
        #[cfg(test)]
        {
            self.local_player_movement_tick_accumulator_seconds = 0.0;
        }
        self.last_vehicle_command_at = None;
        self.riding_jump_charge_seconds = None;
        self.clear_sprint_trigger();
        self.creative_flight_jump_trigger_ticks = 0;
        self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
    }

    fn clear_modifiers(&mut self) {
        self.shift_left_down = false;
        self.shift_right_down = false;
        self.control_left_down = false;
        self.control_right_down = false;
    }

    fn clear_debug_key_state(&mut self) {
        self.debug_modifier_down = false;
        self.debug_modifier_used = false;
        self.debug_game_mode_switcher = None;
        self.reset_debug_crash_hold();
    }

    fn set_shift_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::ShiftLeft => self.shift_left_down = pressed,
            KeyCode::ShiftRight => self.shift_right_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn shift_down(&self) -> bool {
        self.shift_left_down || self.shift_right_down
    }

    pub(crate) fn alt_down(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::AltLeft)
            || self.pressed_keys.contains(&KeyCode::AltRight)
    }

    pub(crate) fn update_screen_modifier_keys(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) {
        if let PhysicalKey::Code(code) = physical_key {
            let pressed = matches!(state, ElementState::Pressed);
            self.set_shift_key(code, pressed);
            self.set_control_key(code, pressed);
        }
    }

    pub(crate) fn player_list_key_down(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::Tab)
    }

    pub(crate) fn hide_gui(&self) -> bool {
        self.hide_gui
    }

    fn set_key_down(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            self.pressed_keys.insert(code);
        } else {
            self.pressed_keys.remove(&code);
        }
    }

    fn set_mouse_button_down(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.mouse_left_down = pressed,
            MouseButton::Right => self.mouse_right_down = pressed,
            MouseButton::Middle => self.mouse_middle_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn item_model_keybind_context(&self) -> ItemModelKeybindContext {
        ItemModelKeybindContext {
            forward: self.pressed_keys.contains(&KeyCode::KeyW),
            left: self.pressed_keys.contains(&KeyCode::KeyA),
            backward: self.pressed_keys.contains(&KeyCode::KeyS),
            right: self.pressed_keys.contains(&KeyCode::KeyD),
            jump: self.pressed_keys.contains(&KeyCode::Space),
            sneak: self.pressed_keys.contains(&KeyCode::ShiftLeft),
            sprint: self.pressed_keys.contains(&KeyCode::ControlLeft),
            attack: self.mouse_left_down,
            use_item: self.mouse_right_down,
            pick_item: self.mouse_middle_down,
            inventory: self.pressed_keys.contains(&KeyCode::KeyE),
            swap_offhand: self.pressed_keys.contains(&KeyCode::KeyF),
            drop: self.pressed_keys.contains(&KeyCode::KeyQ),
            chat: self.pressed_keys.contains(&KeyCode::KeyT),
            command: self.pressed_keys.contains(&KeyCode::Slash),
            player_list: self.pressed_keys.contains(&KeyCode::Tab),
            social_interactions: self.pressed_keys.contains(&KeyCode::KeyP),
            screenshot: self.pressed_keys.contains(&KeyCode::F2),
            toggle_perspective: self.pressed_keys.contains(&KeyCode::F5),
            fullscreen: self.pressed_keys.contains(&KeyCode::F11),
            advancements: self.pressed_keys.contains(&KeyCode::KeyL),
            quick_actions: self.pressed_keys.contains(&KeyCode::KeyG),
            toggle_gui: self.pressed_keys.contains(&KeyCode::F1),
            toggle_spectator_shader_effects: self.pressed_keys.contains(&KeyCode::F4),
            save_toolbar_activator: self.pressed_keys.contains(&KeyCode::KeyC),
            load_toolbar_activator: self.pressed_keys.contains(&KeyCode::KeyX),
            spectator_hotbar: self.mouse_middle_down,
            hotbar: [
                self.pressed_keys.contains(&KeyCode::Digit1),
                self.pressed_keys.contains(&KeyCode::Digit2),
                self.pressed_keys.contains(&KeyCode::Digit3),
                self.pressed_keys.contains(&KeyCode::Digit4),
                self.pressed_keys.contains(&KeyCode::Digit5),
                self.pressed_keys.contains(&KeyCode::Digit6),
                self.pressed_keys.contains(&KeyCode::Digit7),
                self.pressed_keys.contains(&KeyCode::Digit8),
                self.pressed_keys.contains(&KeyCode::Digit9),
            ],
        }
    }

    fn set_control_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::ControlLeft => self.control_left_down = pressed,
            KeyCode::ControlRight => self.control_right_down = pressed,
            _ => {}
        }
    }

    pub(crate) fn control_down(&self) -> bool {
        self.control_left_down || self.control_right_down
    }

    pub(crate) fn command_entry_is_active(&self) -> bool {
        self.chat_entry.is_some()
    }

    pub(crate) fn command_suggestion_hud_state(
        &mut self,
        world: &WorldStore,
        measurer: &impl CommandSuggestionTextMeasurer,
        surface_size: PhysicalSize<u32>,
    ) -> Option<CommandSuggestionHudState> {
        let entry = self.chat_entry.as_mut()?;
        sync_chat_suggestion_list(entry, world);
        update_chat_suggestion_list_rect(entry, measurer, surface_size);
        Some(chat_suggestion_hud_state(entry))
    }

    pub(crate) fn command_suggestions_active(&self) -> bool {
        self.chat_entry.is_some()
    }

    pub(crate) fn command_block_editor_is_active(&self) -> bool {
        self.chat_entry
            .as_ref()
            .is_some_and(|entry| matches!(entry.owner, ChatEntryOwner::CommandBlock(_)))
    }

    pub(crate) fn command_block_editor_hud_state(&self) -> Option<CommandBlockEditorHudState> {
        let entry = self.chat_entry.as_ref()?;
        let ChatEntryOwner::CommandBlock(editor) = &entry.owner else {
            return None;
        };
        Some(CommandBlockEditorHudState {
            mode: editor.mode,
            conditional: editor.conditional,
            automatic: editor.automatic,
            track_output: editor.track_output,
            previous_output: editor.previous_output.clone(),
            controls_enabled: editor.controls_enabled,
            mouse_position: entry.mouse_position,
        })
    }

    pub(crate) fn sync_command_block_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if world.dialog_screen_is_open() {
            return;
        }
        let replaced_by_another_screen = world.open_container_id().is_some()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.game_rules_screen().is_some()
            || world.structure_block_editor().is_some()
            || world.jigsaw_block_editor().is_some()
            || world.test_block_editor().is_some()
            || world.test_instance_block_editor().is_some()
            || world.minecart_command_block_editor().is_some()
            || self.sign_editor_is_active_or_pending(world)
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open();
        if replaced_by_another_screen {
            world.close_command_block_editor();
        }
        let Some(authoritative) = world.command_block_editor().cloned() else {
            if self.command_block_editor_is_active() {
                self.chat_entry = None;
            }
            return;
        };
        let open_count = world.command_block_editor_open_count();
        let current_signature = self
            .chat_entry
            .as_ref()
            .and_then(|entry| match &entry.owner {
                ChatEntryOwner::CommandBlock(editor) => {
                    Some((editor.open_count, editor.authoritative_revision))
                }
                ChatEntryOwner::Chat | ChatEntryOwner::MinecartCommandBlock(_) => None,
            });
        if current_signature == Some((open_count, authoritative.authoritative_revision)) {
            return;
        }

        if current_signature.is_none_or(|(current_open_count, _)| current_open_count != open_count)
        {
            release_active_input(self, world, counters, net_commands);
        }
        let cursor = text_edit::char_len(&authoritative.data.command);
        self.chat_entry = Some(ChatEntryState {
            text: authoritative.data.command,
            cursor,
            selection: cursor,
            owner: ChatEntryOwner::CommandBlock(CommandBlockEditorInputState {
                open_count,
                authoritative_revision: authoritative.authoritative_revision,
                pos: authoritative.pos,
                mode: authoritative.mode,
                conditional: authoritative.conditional,
                automatic: authoritative.data.automatic,
                track_output: authoritative.data.track_output,
                previous_output: authoritative
                    .data
                    .last_output
                    .as_ref()
                    .map(|output| output.text().to_string())
                    .unwrap_or_default(),
                controls_enabled: authoritative.controls_enabled,
            }),
            ..ChatEntryState::default()
        });
        self.chat_scroll_lines = 0;
        maybe_queue_command_suggestion_request(self, counters, world, net_commands);
    }

    pub(crate) fn chat_scroll_lines(&self) -> usize {
        self.chat_scroll_lines
    }

    pub(crate) fn scroll_chat_lines(&mut self, delta: i32) {
        if delta >= 0 {
            self.chat_scroll_lines = self
                .chat_scroll_lines
                .saturating_add(delta as usize)
                .min(100);
        } else {
            self.chat_scroll_lines = self
                .chat_scroll_lines
                .saturating_sub(delta.unsigned_abs() as usize);
        }
    }

    pub(crate) fn replace_chat_entry_text(&mut self, text: String) -> bool {
        let Some(entry) = self.chat_entry.as_mut() else {
            return false;
        };
        entry.text = text;
        entry.cursor = text_edit::char_len(&entry.text);
        entry.selection = entry.cursor;
        entry.suggestion_list = None;
        entry.suggestion_narration = None;
        entry.hidden_suggestion_id = None;
        entry.last_suggestion_request_text = None;
        true
    }

    pub(crate) fn handle_command_suggestion_cursor_moved(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(entry) = self.chat_entry.as_mut() else {
            return false;
        };
        sync_chat_suggestion_list(entry, world);
        let previous = entry.mouse_position;
        entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if previous == entry.mouse_position {
            return true;
        }
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return true;
        };
        let Some(list) = entry.suggestion_list.as_ref() else {
            return true;
        };
        let Some(line) = list
            .rect
            .and_then(|rect| rect.hovered_line(mouse_x, mouse_y))
        else {
            return true;
        };
        let selected = list.offset.saturating_add(line);
        select_chat_suggestion(entry, selected, true);
        true
    }

    pub(crate) fn handle_command_suggestion_mouse_input(
        &mut self,
        world: &WorldStore,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(entry) = self.chat_entry.as_mut() else {
            return false;
        };
        sync_chat_suggestion_list(entry, world);
        entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return false;
        }
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return false;
        };
        let Some(list) = entry.suggestion_list.as_ref() else {
            return false;
        };
        let Some(rect) = list.rect.filter(|rect| rect.contains(mouse_x, mouse_y)) else {
            return false;
        };
        let line = usize::try_from((mouse_y - rect.y) / COMMAND_SUGGESTION_LINE_HEIGHT)
            .unwrap_or_default();
        let selected = list.offset.saturating_add(line);
        if selected >= list.sorted_indices.len() {
            return false;
        }
        select_chat_suggestion(entry, selected, true);
        use_selected_chat_suggestion(entry);
        true
    }

    pub(crate) fn handle_command_block_editor_mouse_input(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        if !self.command_block_editor_is_active() {
            return false;
        }
        if let Some(entry) = self.chat_entry.as_mut() {
            entry.mouse_position = cursor_position.and_then(physical_position_floor_i32);
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let Some((mouse_x, mouse_y)) = self
            .chat_entry
            .as_ref()
            .and_then(|entry| entry.mouse_position)
        else {
            return true;
        };
        let layout = hud_command_block_editor_layout(surface_size);
        if layout.command_input.contains(mouse_x, mouse_y) {
            let Some(entry) = self.chat_entry.as_mut() else {
                return true;
            };
            let display_start = measurer.command_suggestion_display_start(
                &entry.text,
                entry.cursor,
                layout.command_input.width.saturating_sub(8),
            );
            let text_offset = u32::try_from(
                mouse_x
                    .saturating_sub(layout.command_input.x)
                    .saturating_sub(4)
                    .max(0),
            )
            .unwrap_or_default();
            let cursor = measurer.command_suggestion_cursor_for_width(
                &entry.text,
                display_start,
                text_offset,
            );
            set_chat_entry_cursor(entry, cursor);
            return true;
        }
        if layout.cancel.contains(mouse_x, mouse_y) {
            self.chat_entry = None;
            self.chat_scroll_lines = 0;
            world.close_command_block_editor();
            return true;
        }
        let controls_enabled = self.chat_entry.as_ref().is_some_and(|entry| {
            matches!(
                &entry.owner,
                ChatEntryOwner::CommandBlock(editor) if editor.controls_enabled
            )
        });
        if !controls_enabled {
            return true;
        }
        if layout.done.contains(mouse_x, mouse_y) {
            submit_chat_entry(self, counters, world, net_commands);
            return true;
        }
        let Some(editor) = self
            .chat_entry
            .as_mut()
            .and_then(|entry| match &mut entry.owner {
                ChatEntryOwner::CommandBlock(editor) => Some(editor),
                ChatEntryOwner::Chat | ChatEntryOwner::MinecartCommandBlock(_) => None,
            })
        else {
            return true;
        };
        if layout.mode.contains(mouse_x, mouse_y) {
            editor.mode = match editor.mode {
                CommandBlockMode::Sequence => CommandBlockMode::Auto,
                CommandBlockMode::Auto => CommandBlockMode::Redstone,
                CommandBlockMode::Redstone => CommandBlockMode::Sequence,
            };
        } else if layout.conditional.contains(mouse_x, mouse_y) {
            editor.conditional = !editor.conditional;
        } else if layout.automatic.contains(mouse_x, mouse_y) {
            editor.automatic = !editor.automatic;
        } else if layout.track_output.contains(mouse_x, mouse_y) {
            editor.track_output = !editor.track_output;
            world.set_command_block_editor_track_output(editor.track_output);
        }
        true
    }

    pub(crate) fn handle_command_suggestion_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
    ) -> bool {
        let Some(entry) = self.chat_entry.as_mut() else {
            return false;
        };
        let line_limit = entry.suggestion_line_limit();
        let Some((mouse_x, mouse_y)) = entry.mouse_position else {
            return false;
        };
        let Some(list) = entry.suggestion_list.as_mut() else {
            return false;
        };
        let Some(_rect) = list.rect.filter(|rect| rect.contains(mouse_x, mouse_y)) else {
            return false;
        };
        let scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y),
            MouseScrollDelta::PixelDelta(position) => position.y,
        }
        .clamp(-1.0, 1.0);
        let max_offset = list.sorted_indices.len().saturating_sub(line_limit);
        list.offset =
            ((list.offset as f64 - scroll) as isize).clamp(0, max_offset as isize) as usize;
        true
    }

    pub(crate) fn command_suggestion_narration(&self) -> Option<&CommandSuggestionNarrationState> {
        self.chat_entry
            .as_ref()
            .and_then(|entry| entry.suggestion_narration.as_ref())
    }

    pub(crate) fn sign_editor_is_active_or_pending(&self, world: &WorldStore) -> bool {
        if self.sign_editor.is_some() {
            return true;
        }
        sign_editor_signature_from_world(world).is_some_and(|signature| {
            !self.dialog_suspends_sign_editor(&signature)
                && Some(&signature) != self.dismissed_sign_editor.as_ref()
        })
    }

    /// Synchronizes the world-owned open request and enforces Vanilla's
    /// per-tick sign-presence/player-range lifetime guard. Every exit funnels
    /// through the one-shot `removed()` packet path.
    pub(crate) fn sync_sign_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        let mut changed = sync_sign_editor_input(self, counters, world, net_commands);
        let invalid = self.sign_editor.as_ref().is_some_and(|editor| {
            !world.sign_editor_is_valid_at(world_block_pos_from_protocol(editor.signature.pos))
        });
        if invalid {
            submit_sign_editor(self, counters, net_commands);
            changed = true;
        }
        changed
    }

    pub(crate) fn sign_editor_hud_state(&self, world: &WorldStore) -> Option<SignEditorHudState> {
        let current_signature = sign_editor_signature_from_world(world);
        if current_signature
            .as_ref()
            .is_some_and(|signature| self.dialog_suspends_sign_editor(signature))
        {
            return None;
        }
        if let Some(editor) = &self.sign_editor {
            if current_signature.as_ref() == Some(&editor.signature) {
                return Some(sign_editor_hud_state_from_editor(
                    editor,
                    self.sign_editor_ime_preedit_hud_state(editor, Instant::now()),
                ));
            }
        }

        let signature = current_signature?;
        if self.dismissed_sign_editor.as_ref() == Some(&signature) {
            return None;
        }
        let lines = sign_editor_initial_lines(self, world);
        let cursor = sign_line_char_len(&lines[0]);
        Some(SignEditorHudState {
            pos: world_block_pos_from_protocol(signature.pos),
            is_front_text: signature.is_front_text,
            lines,
            line: 0,
            cursor,
            selection: cursor,
            preedit: None,
        })
    }

    pub(crate) fn handle_sign_editor_cursor_moved(
        &mut self,
        position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        if self.sign_editor.is_none() {
            return false;
        }
        self.sign_editor_cursor_position = position.and_then(|position| {
            (position.x.is_finite() && position.y.is_finite())
                .then_some((position.x.floor() as i32, position.y.floor() as i32))
        });
        true
    }

    pub(crate) fn handle_sign_editor_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        sync_sign_editor_input(self, counters, world, net_commands);
        if self.sign_editor.is_none() {
            return false;
        }
        self.handle_sign_editor_cursor_moved(cursor_position);
        if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
            && self.sign_editor_done_button_hovered(surface_size)
        {
            submit_sign_editor(self, counters, net_commands);
        }
        true
    }

    pub(crate) fn sign_editor_done_button_hovered(&self, surface_size: PhysicalSize<u32>) -> bool {
        self.sign_editor_cursor_position.is_some_and(|(x, y)| {
            bbb_renderer::hud_sign_editor_done_button_rect(surface_size).contains(x, y)
        })
    }

    pub(crate) fn handle_sign_editor_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) -> bool {
        self.handle_sign_editor_ime_preedit_at(text, cursor_range, Instant::now())
    }

    fn handle_sign_editor_ime_preedit_at(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        now: Instant,
    ) -> bool {
        let Some(editor) = self.sign_editor.as_ref() else {
            self.sign_editor_ime_preedit = None;
            return false;
        };
        if text.is_empty() {
            self.sign_editor_ime_preedit = None;
            return true;
        }
        let original_text = text;
        let text = original_text
            .chars()
            .filter(|ch| !ch.is_control())
            .take(DIALOG_IME_PREEDIT_MAX_CHARS)
            .collect::<String>();
        if text.is_empty() {
            self.sign_editor_ime_preedit = None;
            return true;
        }
        let text_len = text.chars().count();
        let (caret, focused_start, focused_end) =
            cursor_range.map_or((None, 0, text_len), |(begin, end)| {
                let begin = dialog_ime_preedit_char_index_for_byte(original_text, begin);
                let end = dialog_ime_preedit_char_index_for_byte(original_text, end);
                let start = begin.min(end);
                let finish = begin.max(end);
                if start == finish {
                    (Some(end), 0, text_len)
                } else {
                    (Some(end), start, finish)
                }
            });
        self.sign_editor_ime_preedit = Some(SignEditorImePreeditState {
            text,
            caret,
            focused_start,
            focused_end,
            signature: editor.signature.clone(),
            initialized_at: now,
        });
        true
    }

    pub(crate) fn clear_sign_editor_ime_preedit(&mut self) {
        self.sign_editor_ime_preedit = None;
    }

    fn sign_editor_ime_preedit_hud_state(
        &self,
        editor: &SignEditorInputState,
        now: Instant,
    ) -> Option<SignEditorImePreeditHudState> {
        let state = self.sign_editor_ime_preedit.as_ref()?;
        if state.signature != editor.signature {
            return None;
        }
        let elapsed = now
            .checked_duration_since(state.initialized_at)
            .unwrap_or_default();
        Some(SignEditorImePreeditHudState {
            text: state.text.clone(),
            caret: state.caret,
            focused_start: state.focused_start,
            focused_end: state.focused_end,
            caret_visible: (elapsed.as_millis()
                / DIALOG_IME_PREEDIT_CURSOR_BLINK_INTERVAL.as_millis())
                % 2
                == 0,
        })
    }

    pub(crate) fn chat_entry_is_active(&self) -> bool {
        self.chat_entry
            .as_ref()
            .is_some_and(|entry| matches!(entry.owner, ChatEntryOwner::Chat))
    }

    /// Moves native-owned screen state below the world-owned Dialog stack and
    /// restores it only when that stack returns without another screen taking
    /// ownership in the meantime.
    pub(crate) fn sync_dialog_native_previous_screen(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let dialog_screen_is_open = world.dialog_screen_is_open();
        match (self.dialog_screen_was_open, dialog_screen_is_open) {
            (false, true) => {
                self.dialog_native_previous_screen = self
                    .take_debug_screen_for_dialog()
                    .or_else(|| {
                        self.take_sign_editor_for_dialog(world).map(|editor| {
                            queue_sign_editor_removed(counters, net_commands, &editor);
                            NativeDialogPreviousScreenState::Sign(editor)
                        })
                    })
                    .or_else(|| {
                        self.chat_entry
                            .take()
                            .map(NativeDialogPreviousScreenState::Chat)
                    });
            }
            (true, false) => {
                let screen_replaced = self.native_screen_replaced_dialog(world);
                let previous = self.dialog_native_previous_screen.take();
                if screen_replaced {
                    if let Some(NativeDialogPreviousScreenState::Sign(editor)) = previous {
                        self.dismissed_sign_editor = Some(editor.signature);
                    }
                } else {
                    match previous {
                        Some(NativeDialogPreviousScreenState::Chat(entry))
                            if self.chat_entry.is_none() =>
                        {
                            self.chat_entry = Some(entry);
                        }
                        Some(NativeDialogPreviousScreenState::DebugOptions(screen))
                            if self.debug_options_screen.is_none()
                                && self.debug_pause_screen.is_none() =>
                        {
                            self.debug_options_screen = Some(screen);
                        }
                        Some(NativeDialogPreviousScreenState::DebugPause {
                            screen,
                            disconnect_requested,
                        }) if self.debug_options_screen.is_none()
                            && self.debug_pause_screen.is_none() =>
                        {
                            self.debug_pause_screen = Some(screen);
                            self.pause_screen_disconnect_requested = disconnect_requested;
                        }
                        Some(NativeDialogPreviousScreenState::Sign(editor))
                            if self.sign_editor.is_none() =>
                        {
                            self.dismissed_sign_editor = None;
                            self.sign_editor = Some(editor);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        self.dialog_screen_was_open = dialog_screen_is_open;
    }

    fn take_debug_screen_for_dialog(&mut self) -> Option<NativeDialogPreviousScreenState> {
        if let Some(screen) = self.debug_options_screen.take() {
            return Some(NativeDialogPreviousScreenState::DebugOptions(screen));
        }
        self.debug_pause_screen
            .take()
            .map(|screen| NativeDialogPreviousScreenState::DebugPause {
                screen,
                disconnect_requested: std::mem::take(&mut self.pause_screen_disconnect_requested),
            })
    }

    fn take_sign_editor_for_dialog(&mut self, world: &WorldStore) -> Option<SignEditorInputState> {
        let signature = sign_editor_signature_from_world(world)?;
        if self.dismissed_sign_editor.as_ref() == Some(&signature) {
            return None;
        }
        self.chat_entry = None;
        self.sign_editor_ime_preedit = None;
        if let Some(editor) = self
            .sign_editor
            .take()
            .filter(|editor| editor.signature == signature)
        {
            return Some(editor);
        }
        Some(sign_editor_input_state_from_world(self, world, signature))
    }

    fn dialog_suspends_sign_editor(&self, signature: &SignEditorInputSignature) -> bool {
        matches!(
            self.dialog_native_previous_screen.as_ref(),
            Some(NativeDialogPreviousScreenState::Sign(editor))
                if editor.signature == *signature
        )
    }

    fn native_screen_replaced_dialog(&self, world: &WorldStore) -> bool {
        world.open_container_id().is_some()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.game_rules_screen().is_some()
            || world.death_screen().is_some()
            || world.game_event_screen().is_some()
            || self.debug_options_screen_is_open()
            || self.debug_pause_screen_is_open()
            || self.sign_editor_is_active_or_pending(world)
    }

    pub(crate) fn inventory_hovered_slot(&self) -> Option<i16> {
        self.inventory_hovered_slot
    }

    pub(crate) fn inventory_cursor_position(&self) -> Option<(i32, i32)> {
        self.inventory_cursor_position
    }

    pub(crate) fn merchant_focused_trade_row(&self) -> Option<i32> {
        self.merchant_focused_trade_row
    }

    pub(crate) fn update_screen_cursor_position(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) {
        self.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        self.inventory_hovered_slot = None;
    }

    pub(crate) fn inventory_quick_craft_button_num(&self) -> Option<i8> {
        self.inventory_quick_craft_button_num
    }

    pub(crate) fn inventory_quick_craft_slots(&self) -> &[i16] {
        &self.inventory_quick_craft_slots
    }

    pub(crate) fn stonecutter_recipe_scroll_row(&self) -> i32 {
        self.stonecutter_recipe_scroll_row
    }

    pub(crate) fn loom_pattern_scroll_row(&self) -> i32 {
        self.loom_pattern_scroll_row
    }

    pub(crate) fn advancement_scroll_delta(
        &self,
        selected_tab: Option<&str>,
    ) -> Option<(f64, f64)> {
        selected_tab.and_then(|tab| self.advancement_scroll_deltas.get(tab).copied())
    }

    pub(crate) fn update_advancement_hover_fade(&mut self, hovering: bool) {
        if hovering {
            self.advancement_hover_fade = (self.advancement_hover_fade + 0.02).clamp(0.0, 0.3);
        } else {
            self.advancement_hover_fade = (self.advancement_hover_fade - 0.04).clamp(0.0, 1.0);
        }
    }

    pub(crate) fn reset_advancement_hover_fade(&mut self) {
        self.advancement_hover_fade = 0.0;
    }

    pub(crate) fn advancement_hover_fade(&self) -> f32 {
        self.advancement_hover_fade
    }

    pub(crate) fn debug_overlay_visible(&self) -> bool {
        self.debug_entries.is_overlay_visible()
    }

    pub(crate) fn debug_profiler_chart_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_profiler_chart_visible
    }

    pub(crate) fn debug_fps_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_fps_charts_visible
    }

    pub(crate) fn debug_network_charts_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_network_charts_visible
    }

    pub(crate) fn debug_lightmap_texture_visible(&self) -> bool {
        self.debug_overlay_visible() && self.debug_lightmap_texture_visible
    }

    #[cfg(test)]
    pub(crate) fn debug_entity_hitboxes_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::EntityHitboxes, false)
    }

    #[cfg(test)]
    pub(crate) fn debug_chunk_borders_visible(&self) -> bool {
        self.debug_screen_entry_enabled(DebugScreenEntryId::ChunkBorders, false)
    }

    pub(crate) fn debug_entity_hitboxes_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::EntityHitboxes,
            world.local_player_has_reduced_debug_info(),
        )
    }

    pub(crate) fn debug_chunk_borders_visible_for_world(&self, world: &WorldStore) -> bool {
        self.debug_screen_entry_enabled(
            DebugScreenEntryId::ChunkBorders,
            world.local_player_has_reduced_debug_info(),
        )
    }

    pub(crate) fn debug_screen_entry_enabled(
        &self,
        entry: DebugScreenEntryId,
        reduced_debug_info: bool,
    ) -> bool {
        self.debug_entries
            .is_currently_enabled(entry, reduced_debug_info)
    }

    #[cfg(test)]
    pub(crate) fn set_debug_screen_entry_status(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.set_debug_screen_entry_status_inner(entry, status);
    }

    #[cfg(test)]
    pub(crate) fn debug_screen_entry_status(
        &self,
        entry: DebugScreenEntryId,
    ) -> DebugScreenEntryStatus {
        self.debug_entries.status(entry)
    }

    pub(crate) fn load_debug_screen_profile(&mut self, profile: DebugScreenProfile) {
        self.debug_entries.load_profile(profile);
        self.persist_debug_screen_entries();
    }

    pub(crate) fn set_debug_screen_entries(&mut self, entries: DebugScreenEntryList) {
        self.debug_entries = entries;
    }

    pub(crate) fn set_debug_profile_store_path(&mut self, path: PathBuf) {
        self.debug_profile_store_path = Some(path);
    }

    fn toggle_debug_screen_entry_status(&mut self, entry: DebugScreenEntryId) -> bool {
        let enabled = self.debug_entries.toggle_status(entry);
        self.persist_debug_screen_entries();
        enabled
    }

    fn set_debug_screen_entry_status_inner(
        &mut self,
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    ) {
        self.debug_entries.set_status(entry, status);
        self.persist_debug_screen_entries();
    }

    fn persist_debug_screen_entries(&self) {
        let Some(path) = &self.debug_profile_store_path else {
            return;
        };
        if let Err(err) = self.debug_entries.save_to_debug_profile_file(path) {
            tracing::warn!(?err, path = %path.display(), "failed to save debug profile store");
        }
    }

    pub(crate) fn debug_advanced_item_tooltips(&self) -> bool {
        self.debug_advanced_item_tooltips
    }

    pub(crate) fn set_debug_advanced_item_tooltips(&mut self, enabled: bool) {
        self.debug_advanced_item_tooltips = enabled;
    }

    pub(crate) fn debug_show_local_server_entity_hit_boxes(&self) -> bool {
        self.debug_show_local_server_entity_hit_boxes
    }

    pub(crate) fn set_debug_show_local_server_entity_hit_boxes(&mut self, enabled: bool) {
        self.debug_show_local_server_entity_hit_boxes = enabled;
    }

    pub(crate) fn set_debug_hotkeys_enabled(&mut self, enabled: bool) {
        self.debug_hotkeys_enabled = enabled;
    }

    pub(crate) fn set_debug_feature_count_enabled(&mut self, enabled: bool) {
        self.debug_feature_count_enabled = enabled;
    }

    pub(crate) fn set_remote_debug_renderers(
        &mut self,
        renderers: crate::remote_debug::RemoteDebugRendererSelection,
    ) {
        self.remote_debug_renderers = renderers;
    }

    pub(crate) fn remote_debug_renderers(
        &self,
    ) -> crate::remote_debug::RemoteDebugRendererSelection {
        self.remote_debug_renderers
    }

    pub(crate) fn debug_fog_enabled(&self) -> bool {
        self.debug_fog_enabled
    }

    pub(crate) fn debug_smart_cull_enabled(&self) -> bool {
        self.debug_smart_cull_enabled
    }

    pub(crate) fn debug_pause_on_lost_focus(&self) -> bool {
        self.debug_pause_on_lost_focus
    }

    pub(crate) fn take_debug_resource_pack_reload_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_resource_pack_reload_requests)
    }

    pub(crate) fn take_debug_dynamic_texture_dump_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_dynamic_texture_dump_requests)
    }

    pub(crate) fn take_debug_profiling_toggle_requests(
        &mut self,
    ) -> Vec<DebugProfilingToggleRequest> {
        std::mem::take(&mut self.debug_profiling_toggle_requests)
    }

    pub(crate) fn take_debug_profiler_chart_navigation_requests(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.debug_profiler_chart_navigation_requests)
    }

    pub(crate) fn take_debug_options_screen_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_options_screen_requests)
    }

    pub(crate) fn take_debug_pause_without_menu_requests(&mut self) -> u32 {
        std::mem::take(&mut self.debug_pause_without_menu_requests)
    }

    pub(crate) fn take_debug_frustum_requests(&mut self) -> Vec<DebugFrustumRequest> {
        std::mem::take(&mut self.debug_frustum_requests)
    }

    pub(crate) fn take_debug_feature_count_requests(&mut self) -> Vec<DebugFeatureCountRequest> {
        std::mem::take(&mut self.debug_feature_count_requests)
    }

    pub(crate) fn open_debug_options_screen(&mut self) {
        self.debug_pause_screen = None;
        self.debug_game_mode_switcher = None;
        self.debug_options_screen = Some(DebugOptionsScreenState::default());
    }

    pub(crate) fn close_debug_options_screen(&mut self) {
        self.debug_options_screen = None;
    }

    pub(crate) fn debug_options_screen_is_open(&self) -> bool {
        self.debug_options_screen.is_some()
    }

    pub(crate) fn debug_options_screen_hud_state(
        &self,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> Option<DebugOptionsScreenHudState> {
        let screen = self.debug_options_screen.as_ref()?;
        let rows = debug_options_screen_rows(&screen.search_text);
        let total_rows = rows.len();
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(total_rows, visible_rows));
        let hovered_status_button = screen.cursor_position.and_then(|(mouse_x, mouse_y)| {
            self.debug_options_status_button_at(mouse_x, mouse_y, surface_size, reduced_debug_info)
        });
        let hovered_profile_button = screen.cursor_position.and_then(|(mouse_x, mouse_y)| {
            debug_options_profile_button_at(mouse_x, mouse_y, surface_size)
        });
        let done_hovered = screen.cursor_position.is_some_and(|(mouse_x, mouse_y)| {
            debug_options_done_button_contains(mouse_x, mouse_y, surface_size)
        });
        let rows = rows
            .into_iter()
            .skip(scroll_row)
            .take(visible_rows)
            .map(|row| match row {
                DebugOptionsScreenRow::Category { label } => DebugOptionsScreenHudRow::Category {
                    label: label.to_string(),
                },
                DebugOptionsScreenRow::Entry(entry) => DebugOptionsScreenHudRow::Entry {
                    entry,
                    path: entry.path().to_string(),
                    status: self.debug_entries.status(entry),
                    hovered_status: hovered_status_button
                        .filter(|(hovered_entry, _)| *hovered_entry == entry)
                        .map(|(_, status)| status),
                    allowed: entry.is_allowed(reduced_debug_info),
                },
            })
            .collect();
        let tooltip = screen
            .cursor_position
            .and_then(|(mouse_x, mouse_y)| {
                debug_options_not_allowed_tooltip_at(
                    &screen.search_text,
                    scroll_row,
                    mouse_x,
                    mouse_y,
                    surface_size,
                    reduced_debug_info,
                )
            })
            .map(|(x, y)| DebugOptionsScreenTooltip {
                text: "Not visible when debug info is reduced".to_string(),
                x,
                y,
            });
        Some(DebugOptionsScreenHudState {
            search_text: screen.search_text.clone(),
            search_cursor: screen
                .search_cursor
                .min(text_edit::char_len(&screen.search_text)),
            search_selection: screen
                .search_selection
                .min(text_edit::char_len(&screen.search_text)),
            rows,
            tooltip,
            scroll_row,
            total_rows,
            visible_rows,
            default_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Default),
            default_profile_hovered: hovered_profile_button == Some(DebugScreenProfile::Default),
            performance_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Performance),
            performance_profile_hovered: hovered_profile_button
                == Some(DebugScreenProfile::Performance),
            done_hovered,
        })
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        self.handle_debug_options_screen_key_with_clipboard(physical_key, state, None)
    }

    pub(crate) fn handle_debug_options_screen_key_with_clipboard(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        mut clipboard: Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if self.debug_options_screen.is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        if !pressed {
            return true;
        }
        match code {
            KeyCode::Escape => {
                self.set_key_down(code, false);
                self.close_debug_options_screen();
            }
            KeyCode::KeyA if self.control_down() && !self.shift_down() => {
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    select_debug_options_search_text(screen);
                }
            }
            KeyCode::KeyC if self.control_down() && !self.shift_down() => {
                if let (Some(screen), Some(clipboard)) =
                    (self.debug_options_screen.as_ref(), clipboard.as_deref_mut())
                {
                    clipboard.set_debug_clipboard_text(&debug_options_search_selected_text(screen));
                }
            }
            KeyCode::KeyV if self.control_down() && !self.shift_down() => {
                if let (Some(text), Some(screen)) = (
                    clipboard
                        .as_deref_mut()
                        .and_then(|clipboard| clipboard.get_debug_clipboard_text()),
                    self.debug_options_screen.as_mut(),
                ) {
                    insert_debug_options_search_text(screen, &text);
                }
            }
            KeyCode::KeyX if self.control_down() && !self.shift_down() => {
                if let (Some(screen), Some(clipboard)) =
                    (self.debug_options_screen.as_mut(), clipboard.as_deref_mut())
                {
                    let selected = debug_options_search_selected_text(screen);
                    if clipboard.set_debug_clipboard_text(&selected) {
                        let before = screen.search_text.clone();
                        delete_debug_options_search_selection(screen);
                        if screen.search_text != before {
                            screen.scroll_row = 0;
                        }
                    }
                }
            }
            KeyCode::ArrowLeft => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = if control_down {
                        text_edit::word_position(&screen.search_text, screen.search_cursor, -1)
                    } else {
                        screen.search_cursor.saturating_sub(1)
                    };
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::ArrowRight => {
                let control_down = self.control_down();
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = if control_down {
                        text_edit::word_position(&screen.search_text, screen.search_cursor, 1)
                    } else {
                        (screen.search_cursor + 1).min(text_edit::char_len(&screen.search_text))
                    };
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::Home => {
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    move_debug_options_search_cursor(screen, 0, shift_down);
                }
            }
            KeyCode::End => {
                let shift_down = self.shift_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let cursor = text_edit::char_len(&screen.search_text);
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
            }
            KeyCode::Backspace => {
                let control_down = self.control_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let before = screen.search_text.clone();
                    let deleted_selection = delete_debug_options_search_selection(screen);
                    if !deleted_selection && control_down {
                        text_edit::remove_word_before_cursor(
                            &mut screen.search_text,
                            &mut screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    } else if !deleted_selection {
                        remove_debug_options_search_char_before_cursor(
                            &mut screen.search_text,
                            &mut screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    }
                    if screen.search_text != before {
                        screen.scroll_row = 0;
                    }
                }
            }
            KeyCode::Delete => {
                let control_down = self.control_down();
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    let before = screen.search_text.clone();
                    let deleted_selection = delete_debug_options_search_selection(screen);
                    if !deleted_selection && control_down {
                        text_edit::remove_word_at_cursor(
                            &mut screen.search_text,
                            screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    } else if !deleted_selection {
                        remove_debug_options_search_char_at_cursor(
                            &mut screen.search_text,
                            screen.search_cursor,
                        );
                        screen.search_selection = screen.search_cursor;
                    }
                    if screen.search_text != before {
                        screen.scroll_row = 0;
                    }
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn handle_debug_options_screen_text_input(&mut self, text: &str) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        insert_debug_options_search_text(screen, text);
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.handle_debug_options_screen_cursor_moved_with_text_measurer(
            cursor_position,
            surface_size,
            &DebugOptionsFallbackSearchTextMeasurer,
        )
    }

    pub(crate) fn handle_debug_options_screen_cursor_moved_with_text_measurer(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        text_measurer: &dyn DebugOptionsSearchTextMeasurer,
    ) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        let position = cursor_position.and_then(physical_position_floor_i32);
        screen.cursor_position = position;
        if let (Some(_), Some((_, mouse_y))) = (screen.scrollbar_drag_y, position) {
            drag_debug_options_scrollbar(screen, mouse_y, surface_size);
            return true;
        }
        if let (true, Some((mouse_x, _))) = (screen.search_selecting, position) {
            let cursor = debug_options_search_cursor_for_mouse_x_with_text_measurer(
                screen,
                mouse_x,
                surface_size,
                text_measurer,
            );
            move_debug_options_search_cursor(screen, cursor, true);
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> bool {
        self.handle_debug_options_screen_mouse_input_with_text_measurer(
            button,
            state,
            cursor_position,
            surface_size,
            reduced_debug_info,
            &DebugOptionsFallbackSearchTextMeasurer,
        )
    }

    pub(crate) fn handle_debug_options_screen_mouse_input_with_text_measurer(
        &mut self,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
        text_measurer: &dyn DebugOptionsSearchTextMeasurer,
    ) -> bool {
        if self.debug_options_screen.is_none() {
            return false;
        }
        if let Some(screen) = self.debug_options_screen.as_mut() {
            screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        }
        if matches!((button, state), (MouseButton::Left, ElementState::Released)) {
            if let Some(screen) = self.debug_options_screen.as_mut() {
                screen.search_selecting = false;
                screen.scrollbar_drag_y = None;
            }
            return true;
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let now = Instant::now();
        let double_click = self
            .debug_options_screen
            .as_ref()
            .is_some_and(|screen| debug_options_screen_is_double_click(screen, now));
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(physical_position_floor_i32) else {
            if let Some(screen) = self.debug_options_screen.as_mut() {
                screen.search_selecting = false;
                screen.last_left_click_at = None;
            }
            return true;
        };
        let search_cursor = self.debug_options_screen.as_ref().and_then(|screen| {
            debug_options_search_cursor_at_with_text_measurer(
                screen,
                mouse_x,
                mouse_y,
                surface_size,
                text_measurer,
            )
        });
        if let Some(screen) = self.debug_options_screen.as_mut() {
            screen.last_left_click_at = Some(now);
        }
        if let Some(cursor) = search_cursor {
            let shift_down = self.shift_down();
            if let Some(screen) = self.debug_options_screen.as_mut() {
                if double_click {
                    select_debug_options_search_word_at(screen, cursor);
                } else {
                    move_debug_options_search_cursor(screen, cursor, shift_down);
                }
                screen.search_selecting = true;
            }
            return true;
        }
        if let Some(screen) = self.debug_options_screen.as_mut() {
            screen.search_selecting = false;
            screen.scrollbar_drag_y = None;
        }
        let over_scrollbar = self.debug_options_screen.as_ref().is_some_and(|screen| {
            debug_options_scrollbar_contains(mouse_x, mouse_y, &screen.search_text, surface_size)
        });
        if over_scrollbar {
            if let Some(screen) = self.debug_options_screen.as_mut() {
                screen.scrollbar_drag_y = Some(mouse_y);
            }
            return true;
        }
        if let Some(profile) = debug_options_profile_button_at(mouse_x, mouse_y, surface_size) {
            if !self.debug_entries.is_using_profile(profile) {
                self.load_debug_screen_profile(profile);
                if let Some(screen) = self.debug_options_screen.as_mut() {
                    screen.scroll_row = 0;
                }
            }
            return true;
        }
        if debug_options_done_button_contains(mouse_x, mouse_y, surface_size) {
            self.close_debug_options_screen();
            return true;
        }
        if let Some((entry, status)) =
            self.debug_options_status_button_at(mouse_x, mouse_y, surface_size, reduced_debug_info)
        {
            self.set_debug_screen_entry_status_inner(entry, status);
        }
        true
    }

    pub(crate) fn handle_debug_options_screen_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(screen) = self.debug_options_screen.as_mut() else {
            return false;
        };
        let rows = debug_options_screen_rows(&screen.search_text);
        let visible_rows = debug_options_visible_row_count(surface_size);
        let max_scroll = debug_options_max_scroll_row(rows.len(), visible_rows);
        let delta_rows = debug_options_scroll_delta_rows(delta);
        if delta_rows < 0 {
            screen.scroll_row = screen
                .scroll_row
                .saturating_add(delta_rows.unsigned_abs() as usize);
        } else if delta_rows > 0 {
            screen.scroll_row = screen.scroll_row.saturating_sub(delta_rows as usize);
        }
        screen.scroll_row = screen.scroll_row.min(max_scroll);
        true
    }

    pub(crate) fn open_debug_pause_screen_without_menu(&mut self) {
        self.debug_options_screen = None;
        self.pause_screen_disconnect_requested = false;
        self.debug_pause_screen = Some(DebugPauseScreenState {
            show_pause_menu: false,
            cursor_position: None,
        });
    }

    pub(crate) fn open_debug_pause_screen_with_menu(&mut self) {
        self.debug_options_screen = None;
        self.pause_screen_disconnect_requested = false;
        self.debug_pause_screen = Some(DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: None,
        });
    }

    pub(crate) fn close_debug_pause_screen(&mut self) {
        self.debug_pause_screen = None;
        self.pause_screen_disconnect_requested = false;
    }

    pub(crate) fn debug_pause_screen(&self) -> Option<DebugPauseScreenState> {
        self.debug_pause_screen
    }

    pub(crate) fn debug_pause_screen_is_open(&self) -> bool {
        self.debug_pause_screen.is_some()
    }

    pub(crate) fn handle_debug_pause_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        if self.debug_pause_screen.is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(state, ElementState::Pressed) && matches!(code, KeyCode::Escape) {
            self.set_key_down(code, false);
            self.close_debug_pause_screen();
            return true;
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        self.set_key_down(code, false);
        true
    }

    pub(crate) fn handle_debug_pause_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(screen) = self.debug_pause_screen.as_mut() else {
            return false;
        };
        screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        true
    }

    pub(crate) fn handle_debug_pause_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some((show_pause_menu, cursor_position)) =
            self.debug_pause_screen.as_mut().map(|screen| {
                screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
                (screen.show_pause_menu, screen.cursor_position)
            })
        else {
            return false;
        };
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        if show_pause_menu {
            if pause_screen_return_to_game_button_contains(cursor_position, surface_size) {
                self.close_debug_pause_screen();
            } else if pause_screen_advancements_button_contains(cursor_position, surface_size) {
                self.close_debug_pause_screen();
                open_advancements_screen(self, counters, world, net_commands);
            } else if pause_screen_stats_button_contains(cursor_position, surface_size) {
                self.close_debug_pause_screen();
                open_stats_screen(self, counters, world, net_commands);
            } else if pause_screen_send_feedback_button_contains(cursor_position, surface_size) {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::SendFeedback {
                        url: pause_screen_feedback_url(),
                    });
            } else if pause_screen_report_bugs_button_contains(cursor_position, surface_size)
                && pause_screen_report_bugs_button_enabled()
            {
                self.pause_screen_link_requests
                    .push(PauseScreenLinkRequest::ReportBugs {
                        url: PAUSE_SCREEN_SNAPSHOT_BUGS_URL,
                    });
            } else if pause_screen_disconnect_button_contains(cursor_position, surface_size) {
                self.pause_screen_disconnect_requested = true;
            }
        }
        true
    }

    pub(crate) fn pause_screen_disconnect_requested(&self) -> bool {
        self.pause_screen_disconnect_requested
    }

    pub(crate) fn take_pause_screen_link_requests(&mut self) -> Vec<PauseScreenLinkRequest> {
        std::mem::take(&mut self.pause_screen_link_requests)
    }

    pub(crate) fn stats_screen_cursor_position(&self) -> Option<(i32, i32)> {
        self.stats_screen_cursor_position
    }

    pub(crate) fn stats_screen_scroll_amount(&self) -> f64 {
        self.stats_screen_scroll_amount
    }

    pub(crate) fn sync_stats_screen_scroll_metrics(
        &mut self,
        max_scroll_amount: i32,
        scroller_height: i32,
    ) {
        self.stats_screen_max_scroll_amount = f64::from(max_scroll_amount.max(0));
        self.stats_screen_scroller_height = scroller_height.max(0);
        self.stats_screen_scroll_amount = self
            .stats_screen_scroll_amount
            .clamp(0.0, self.stats_screen_max_scroll_amount);
        if self.stats_screen_max_scroll_amount == 0.0 {
            self.stats_screen_scrolling = false;
        }
    }

    pub(crate) fn reset_stats_screen_scroll(&mut self) {
        self.stats_screen_scroll_amount = 0.0;
        self.stats_screen_scrolling = false;
    }

    pub(crate) fn keep_stats_screen_focus_visible(
        &mut self,
        viewport_height: i32,
        row_height: i32,
    ) {
        let entry = match self.stats_screen_focus {
            StatsScreenFocus::GeneralRow(row) | StatsScreenFocus::MobRow(row) => row,
            StatsScreenFocus::ItemHeader(_) => 0,
            StatsScreenFocus::ItemRow(row) => row.saturating_add(1),
            _ => return,
        };
        let entry = i32::try_from(entry).unwrap_or(i32::MAX / row_height.max(1));
        let top = STATS_SCREEN_LIST_ENTRY_PADDING.saturating_add(entry.saturating_mul(row_height));
        let bottom = top.saturating_add(row_height);
        let scroll = self.stats_screen_scroll_amount as i32;
        if top < scroll {
            self.stats_screen_scroll_amount = f64::from(top.max(0));
        } else if bottom > scroll.saturating_add(viewport_height) {
            self.stats_screen_scroll_amount = f64::from(
                bottom
                    .saturating_sub(viewport_height)
                    .clamp(0, self.stats_screen_max_scroll_amount as i32),
            );
        }
    }

    pub(crate) fn stats_screen_tab(&self) -> usize {
        self.stats_screen_tab
    }

    pub(crate) fn stats_screen_item_sort(&self) -> (Option<usize>, i8) {
        (
            self.stats_screen_item_sort_column,
            self.stats_screen_item_sort_order,
        )
    }

    pub(crate) fn stats_screen_item_effective_sort(&self) -> Option<(usize, i8)> {
        self.stats_screen_item_effective_sort
    }

    pub(crate) fn stats_screen_focus(&self) -> StatsScreenFocus {
        self.stats_screen_focus
    }

    /// Synchronizes the concrete focus path after the runtime has projected the
    /// active tab's rows. Vanilla replaces the loading tabs with real widgets on
    /// the first stats response and explicitly focuses the tab navigation bar.
    pub(crate) fn sync_stats_screen_interaction(
        &mut self,
        is_loading: bool,
        active_tab: usize,
        total_rows: usize,
    ) {
        if active_tab < self.stats_screen_tab_row_counts.len() {
            self.stats_screen_tab_row_counts[active_tab] = total_rows;
        }
        self.stats_screen_total_rows = total_rows;
        if is_loading {
            self.stats_screen_was_loading = true;
            self.stats_screen_focus = StatsScreenFocus::None;
            return;
        }
        if self.stats_screen_was_loading {
            self.stats_screen_was_loading = false;
            self.stats_screen_focus = StatsScreenFocus::Tabs;
        }
        self.stats_screen_focus = match self.stats_screen_focus {
            StatsScreenFocus::None => StatsScreenFocus::None,
            StatsScreenFocus::GeneralRow(row) if active_tab == 0 && row < total_rows => {
                StatsScreenFocus::GeneralRow(row)
            }
            StatsScreenFocus::ItemHeader(column) if active_tab == 1 && column < 6 => {
                StatsScreenFocus::ItemHeader(column)
            }
            StatsScreenFocus::ItemRow(row) if active_tab == 1 && row < total_rows => {
                StatsScreenFocus::ItemRow(row)
            }
            StatsScreenFocus::MobRow(row) if active_tab == 2 && row < total_rows => {
                StatsScreenFocus::MobRow(row)
            }
            StatsScreenFocus::List | StatsScreenFocus::Tabs | StatsScreenFocus::Done => {
                self.stats_screen_focus
            }
            _ => StatsScreenFocus::List,
        };
    }

    pub(crate) fn sync_stats_screen_tab_row_counts(&mut self, row_counts: [usize; 3]) {
        self.stats_screen_tab_row_counts = row_counts;
        self.stats_screen_total_rows = row_counts[self.stats_screen_tab.min(2)];
    }

    pub(crate) fn stats_screen_item_hovered_column(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        (self.stats_screen_tab == 1)
            .then(|| {
                stats_screen_item_column_at_position(
                    self.stats_screen_cursor_position,
                    surface_size,
                    self.stats_screen_scroll_amount as i32,
                )
            })
            .flatten()
    }

    pub(crate) fn stats_screen_hovered_tab(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        stats_screen_tab_at_position(self.stats_screen_cursor_position, surface_size)
    }

    pub(crate) fn stats_screen_hovered_item_row(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        if self.stats_screen_tab != 1 {
            return None;
        }
        let (mouse_x, mouse_y) = self.stats_screen_cursor_position?;
        let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
        let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
        let list_bottom = i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
        if !(list_x..list_x + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE).contains(&mouse_x)
            || !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
        {
            return None;
        }
        let content_y = mouse_y - STATS_SCREEN_LIST_TOP - STATS_SCREEN_LIST_ENTRY_PADDING
            + self.stats_screen_scroll_amount as i32;
        if content_y < 0 {
            return None;
        }
        let entry = content_y / 22;
        (entry > 0 && content_y % 22 < STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE)
            .then(|| usize::try_from(entry - 1).ok())
            .flatten()
    }

    pub(crate) fn stats_screen_hovered_row(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        if self.stats_screen_tab == 1 {
            return self.stats_screen_hovered_item_row(surface_size);
        }
        stats_screen_row_at_position(
            self.stats_screen_cursor_position,
            surface_size,
            self.stats_screen_tab,
            self.stats_screen_scroll_amount as i32,
        )
        .filter(|row| *row < self.stats_screen_total_rows)
    }

    pub(crate) fn handle_stats_screen_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
    ) -> bool {
        if !world.stats_screen_is_open() {
            return false;
        }
        let row_height = stats_screen_row_height(self.stats_screen_tab);
        let delta_pixels = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y) * f64::from((row_height / 2).max(1)),
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        self.stats_screen_scroll_amount = (self.stats_screen_scroll_amount - delta_pixels)
            .clamp(0.0, self.stats_screen_max_scroll_amount);
        true
    }

    pub(crate) fn handle_stats_screen_cursor_moved(
        &mut self,
        world: &WorldStore,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        if !world.stats_screen_is_open() {
            return false;
        }
        let previous = self.stats_screen_cursor_position;
        self.stats_screen_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if self.stats_screen_scrolling {
            let Some((_, mouse_y)) = self.stats_screen_cursor_position else {
                return true;
            };
            let list_bottom = i32::try_from(surface_size.height)
                .unwrap_or(i32::MAX)
                .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
            if mouse_y < STATS_SCREEN_LIST_TOP {
                self.stats_screen_scroll_amount = 0.0;
            } else if mouse_y > list_bottom {
                self.stats_screen_scroll_amount = self.stats_screen_max_scroll_amount;
            } else if let Some((_, previous_y)) = previous {
                let viewport_height = (list_bottom - STATS_SCREEN_LIST_TOP).max(1);
                let thumb_travel = (viewport_height - self.stats_screen_scroller_height).max(1);
                let drag_scale =
                    (self.stats_screen_max_scroll_amount / f64::from(thumb_travel)).max(1.0);
                self.stats_screen_scroll_amount = (self.stats_screen_scroll_amount
                    + f64::from(mouse_y - previous_y) * drag_scale)
                    .clamp(0.0, self.stats_screen_max_scroll_amount);
            }
        }
        true
    }

    pub(crate) fn handle_stats_screen_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        if !world.stats_screen_is_open() {
            return false;
        }
        self.stats_screen_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if matches!((button, state), (MouseButton::Left, ElementState::Released)) {
            self.stats_screen_scrolling = false;
        }
        if matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            if self.stats_screen_max_scroll_amount > 0.0
                && stats_screen_scrollbar_contains(self.stats_screen_cursor_position, surface_size)
            {
                self.stats_screen_scrolling = true;
                self.stats_screen_focus = StatsScreenFocus::List;
                return true;
            }
            let item_column = (self.stats_screen_tab == 1 && !world.stats_screen_is_loading())
                .then(|| {
                    stats_screen_item_column_at_position(
                        self.stats_screen_cursor_position,
                        surface_size,
                        self.stats_screen_scroll_amount as i32,
                    )
                })
                .flatten();
            if let Some(column) = item_column {
                self.activate_stats_screen_item_column(column);
                self.stats_screen_focus = StatsScreenFocus::ItemHeader(column);
            } else if let Some(tab) =
                stats_screen_tab_at_position(self.stats_screen_cursor_position, surface_size)
            {
                if stats_screen_tab_is_enabled(tab, world) {
                    self.stats_screen_tab = tab;
                    self.stats_screen_total_rows = self.stats_screen_tab_row_counts[tab];
                    self.stats_screen_scroll_amount = 0.0;
                    self.stats_screen_focus = StatsScreenFocus::Tabs;
                }
            } else if stats_screen_done_button_contains(
                self.stats_screen_cursor_position,
                surface_size,
            ) {
                self.stats_screen_focus = StatsScreenFocus::Done;
                close_stats_screen(self, counters, world, net_commands);
            } else if let Some(row) = self.stats_screen_hovered_row(surface_size) {
                self.stats_screen_focus = match self.stats_screen_tab {
                    1 => StatsScreenFocus::ItemRow(row),
                    2 => StatsScreenFocus::MobRow(row),
                    _ => StatsScreenFocus::GeneralRow(row),
                };
            }
        }
        true
    }

    fn activate_stats_screen_item_column(&mut self, column: usize) {
        match (
            self.stats_screen_item_sort_column,
            self.stats_screen_item_sort_order,
        ) {
            (Some(current), -1) if current == column => {
                self.stats_screen_item_sort_order = 1;
                self.stats_screen_item_effective_sort = Some((column, 1));
            }
            (Some(current), 1) if current == column => {
                self.stats_screen_item_sort_column = None;
                self.stats_screen_item_sort_order = 0;
            }
            _ => {
                self.stats_screen_item_sort_column = Some(column);
                self.stats_screen_item_sort_order = -1;
                self.stats_screen_item_effective_sort = Some((column, -1));
            }
        }
        self.stats_screen_scroll_amount = 0.0;
    }

    pub(crate) fn debug_game_mode_switcher_selected(&self) -> Option<GameType> {
        self.debug_game_mode_switcher
            .as_ref()
            .map(|switcher| switcher.selected)
    }

    pub(crate) fn debug_game_mode_switcher_is_open(&self) -> bool {
        self.debug_game_mode_switcher.is_some()
    }

    pub(crate) fn handle_debug_game_mode_switcher_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(switcher) = self.debug_game_mode_switcher.as_mut() else {
            return false;
        };
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(debug_game_mode_switcher_mouse_pos)
        else {
            return true;
        };
        let mouse_pos = (mouse_x, mouse_y);
        let Some(first_mouse_position) = switcher.first_mouse_position else {
            switcher.first_mouse_position = Some(mouse_pos);
            return true;
        };
        if first_mouse_position == mouse_pos {
            return true;
        }
        if let Some(hovered) = debug_game_mode_switcher_hovered_game_type(mouse_pos, surface_size) {
            switcher.selected = hovered;
        }
        true
    }

    fn debug_options_status_button_at(
        &self,
        mouse_x: i32,
        mouse_y: i32,
        surface_size: PhysicalSize<u32>,
        _reduced_debug_info: bool,
    ) -> Option<(DebugScreenEntryId, DebugScreenEntryStatus)> {
        let screen = self.debug_options_screen.as_ref()?;
        let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
        let rows = debug_options_screen_rows(&screen.search_text);
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(rows.len(), visible_rows));
        let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
            return None;
        };
        let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
            - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
        if mouse_x < buttons_start_x {
            return None;
        }
        let button_index =
            ((mouse_x - buttons_start_x) / DEBUG_OPTIONS_STATUS_BUTTON_WIDTH).clamp(0, 2);
        let status = match button_index {
            0 => DebugScreenEntryStatus::Never,
            1 => DebugScreenEntryStatus::InOverlay,
            _ => DebugScreenEntryStatus::AlwaysOn,
        };
        Some((entry, status))
    }

    pub(crate) fn take_debug_recreate_server_query_requests(
        &mut self,
    ) -> Vec<DebugRecreateServerQueryRequest> {
        std::mem::take(&mut self.debug_recreate_server_query_requests)
    }

    pub(crate) fn take_debug_recreate_server_query_requests_for_level(
        &mut self,
        client_level_generation: u64,
    ) -> Vec<DebugRecreateServerQueryRequest> {
        if self
            .pending_debug_recreate_server_query
            .as_ref()
            .is_some_and(|pending| pending.client_level_generation != client_level_generation)
        {
            self.pending_debug_recreate_server_query = None;
            self.debug_recreate_server_query_requests.clear();
            return Vec::new();
        }
        self.take_debug_recreate_server_query_requests()
    }

    pub(crate) fn consume_debug_recreate_server_query_response(
        &mut self,
        world: &mut WorldStore,
        clipboard: &mut dyn DebugClipboard,
    ) -> bool {
        let Some(pending) = self.pending_debug_recreate_server_query.clone() else {
            return false;
        };
        if pending.client_level_generation != world.client_level_generation() {
            self.pending_debug_recreate_server_query = None;
            push_debug_feedback_chat_message(
                Some(world),
                "Debug recreate level changed before server response",
            );
            return true;
        }
        let Some(response) = world.last_tag_query().cloned() else {
            return false;
        };
        if response.transaction_id != pending.transaction_id {
            return false;
        }

        self.pending_debug_recreate_server_query = None;
        if let PendingDebugRecreateServerQueryTarget::Entity {
            entity_id,
            entity_uuid,
            ..
        } = &pending.target
        {
            if world
                .probe_entity(*entity_id)
                .is_none_or(|entity| entity.uuid != *entity_uuid)
            {
                push_debug_feedback_chat_message(
                    Some(world),
                    "Debug recreate target disappeared before server response",
                );
                return true;
            }
        }
        if !response.tag_present {
            push_debug_feedback_chat_message(Some(world), "Server returned no recreate data");
            return true;
        }
        let copy = match debug_copy_recreate_server_response_command(&pending.target, &response) {
            Ok(copy) => copy,
            Err(_) => {
                push_debug_feedback_chat_message(
                    Some(world),
                    "Failed to decode server-side recreate data",
                );
                return true;
            }
        };
        let Some(copy) = copy else {
            return true;
        };
        if clipboard.set_debug_clipboard_text(&copy.command) {
            push_debug_feedback_chat_message(Some(world), copy.feedback_message);
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_overlay_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
    ) -> bool {
        self.handle_debug_overlay_key_with_clipboard(
            physical_key,
            state,
            world,
            terrain_upload,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_overlay_key_with_clipboard(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        clipboard: Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            world,
            terrain_upload,
            clipboard,
            None,
            None,
        )
    }

    pub(crate) fn handle_debug_overlay_key_with_clipboard_and_net(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        clipboard: Option<&mut dyn DebugClipboard>,
        item_runtime: Option<&NativeItemRuntime>,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        self.handle_debug_overlay_key_inner(
            physical_key,
            state,
            world,
            terrain_upload,
            clipboard,
            item_runtime,
            Some(DebugNetContext {
                counters,
                net_commands,
            }),
        )
    }

    fn handle_debug_overlay_key_inner(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        mut world: Option<&mut WorldStore>,
        terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        clipboard: Option<&mut dyn DebugClipboard>,
        item_runtime: Option<&NativeItemRuntime>,
        mut net_context: Option<DebugNetContext<'_>>,
    ) -> bool {
        if !self.focused {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return false;
        };

        if code == KeyCode::F3 {
            match state {
                ElementState::Pressed => {
                    self.debug_modifier_down = true;
                }
                ElementState::Released => {
                    self.debug_modifier_down = false;
                    self.reset_debug_crash_hold();
                    if let Some(switcher) = self.debug_game_mode_switcher.take() {
                        self.debug_modifier_used = false;
                        if let (Some(world), Some(context)) =
                            (world.as_deref_mut(), net_context.as_mut())
                        {
                            queue_debug_game_mode_switcher_selection(
                                world,
                                context,
                                switcher.selected,
                            );
                        }
                        return true;
                    }
                    if self.debug_modifier_used {
                        self.debug_modifier_used = false;
                    } else {
                        self.debug_entries.toggle_overlay();
                    }
                }
            }
            return true;
        }

        if matches!(state, ElementState::Released) && code == KeyCode::KeyC {
            self.reset_debug_crash_hold();
        }

        if matches!(state, ElementState::Pressed)
            && self.debug_modifier_down
            && self.handle_debug_overlay_modifier_key(
                code,
                world,
                terrain_upload,
                clipboard,
                item_runtime,
                net_context.as_mut(),
            )
        {
            self.debug_modifier_used = true;
            return true;
        }

        if self.debug_game_mode_switcher.is_some() {
            return true;
        }

        false
    }

    fn handle_debug_overlay_modifier_key(
        &mut self,
        code: KeyCode,
        mut world: Option<&mut WorldStore>,
        mut terrain_upload: Option<&mut dyn TerrainReloadRequestSink>,
        mut clipboard: Option<&mut dyn DebugClipboard>,
        item_runtime: Option<&NativeItemRuntime>,
        net_context: Option<&mut DebugNetContext<'_>>,
    ) -> bool {
        if self.debug_hotkeys_enabled && self.handle_shared_debug_hotkey(code, world.as_deref_mut())
        {
            return true;
        }
        if self.debug_feature_count_enabled && self.handle_debug_feature_count_key(code) {
            return true;
        }
        match code {
            KeyCode::Escape => {
                self.debug_pause_without_menu_requests =
                    self.debug_pause_without_menu_requests.saturating_add(1);
                true
            }
            KeyCode::KeyA => {
                let Some(terrain_upload) = terrain_upload.as_deref_mut() else {
                    return false;
                };
                terrain_upload.request_reload_all_chunks();
                push_debug_feedback_chat_message(world.as_deref_mut(), "Reloading all chunks");
                true
            }
            KeyCode::Digit1 => {
                self.toggle_debug_profiler_chart();
                true
            }
            KeyCode::Digit2 => {
                self.toggle_debug_fps_charts();
                true
            }
            KeyCode::Digit3 => {
                self.toggle_debug_network_charts();
                true
            }
            KeyCode::Digit4 => {
                self.toggle_debug_lightmap_texture();
                true
            }
            KeyCode::KeyD => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                world.clear_client_chat_display_messages();
                true
            }
            KeyCode::KeyB => {
                if !Self::debug_world_status_toggles_allowed(world.as_deref()) {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes);
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if enabled {
                        "Hitboxes: shown"
                    } else {
                        "Hitboxes: hidden"
                    },
                );
                true
            }
            KeyCode::KeyG => {
                if !Self::debug_world_status_toggles_allowed(world.as_deref()) {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkBorders);
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if enabled {
                        "Chunk borders: shown"
                    } else {
                        "Chunk borders: hidden"
                    },
                );
                true
            }
            KeyCode::KeyH => {
                self.debug_advanced_item_tooltips = !self.debug_advanced_item_tooltips;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_advanced_item_tooltips {
                        "Advanced tooltips: shown"
                    } else {
                        "Advanced tooltips: hidden"
                    },
                );
                true
            }
            KeyCode::KeyP => {
                self.debug_pause_on_lost_focus = !self.debug_pause_on_lost_focus;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_pause_on_lost_focus {
                        "Pause on lost focus: enabled"
                    } else {
                        "Pause on lost focus: disabled"
                    },
                );
                true
            }
            KeyCode::KeyN => {
                let Some(world) = world.as_deref_mut() else {
                    return true;
                };
                if !world.local_player_has_gamemaster_permission() {
                    push_debug_feedback_chat_message(
                        Some(world),
                        "Unable to switch game mode; no permission",
                    );
                } else if let Some(context) = net_context {
                    queue_change_game_mode_command(
                        context.counters,
                        context.net_commands,
                        ChangeGameModeCommand {
                            game_mode: debug_spectate_target_game_mode(world),
                        },
                    );
                }
                true
            }
            KeyCode::F4 => {
                if let Some(switcher) = self.debug_game_mode_switcher.as_mut() {
                    switcher.selected = next_debug_game_mode_icon(switcher.selected);
                    switcher.first_mouse_position = None;
                    return true;
                }
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if !self.debug_game_mode_switcher_can_open(world) {
                    return false;
                }
                if !world.local_player_has_gamemaster_permission() {
                    push_debug_feedback_chat_message(
                        Some(world),
                        "Unable to open game mode switcher; no permission",
                    );
                } else {
                    self.debug_game_mode_switcher = Some(DebugGameModeSwitcherState {
                        selected: default_debug_game_mode_switcher_selection(world),
                        first_mouse_position: None,
                    });
                }
                true
            }
            KeyCode::KeyV => {
                if let Some(world) = world.as_deref_mut() {
                    push_debug_version_chat_messages(world);
                }
                true
            }
            KeyCode::KeyT => {
                self.debug_resource_pack_reload_requests =
                    self.debug_resource_pack_reload_requests.saturating_add(1);
                push_debug_feedback_chat_message(world.as_deref_mut(), "Reloaded resource packs");
                true
            }
            KeyCode::KeyS => {
                self.debug_dynamic_texture_dump_requests =
                    self.debug_dynamic_texture_dump_requests.saturating_add(1);
                push_debug_dynamic_texture_dump_feedback(world.as_deref_mut());
                true
            }
            KeyCode::KeyI => {
                let pull_from_server = !self.shift_down();
                let add_nbt = world
                    .as_deref()
                    .is_some_and(WorldStore::local_player_has_gamemaster_permission);
                if add_nbt && pull_from_server {
                    if let Some(world) = world.as_deref() {
                        if let Some(target) = debug_recreate_target(world) {
                            let transaction_id = self.next_debug_query_transaction_id();
                            if let Some(pending) =
                                pending_debug_recreate_server_query(world, target, transaction_id)
                            {
                                self.pending_debug_recreate_server_query = Some(pending);
                                self.debug_recreate_server_query_requests.push(
                                    debug_recreate_server_query_request(target, transaction_id),
                                );
                            }
                        }
                    }
                } else {
                    let command = world.as_deref().and_then(|world| {
                        debug_copy_recreate_command(world, item_runtime, add_nbt)
                    });
                    if let (Some(copy), Some(clipboard)) = (command, clipboard.as_deref_mut()) {
                        if clipboard.set_debug_clipboard_text(&copy.command) {
                            push_debug_feedback_chat_message(
                                world.as_deref_mut(),
                                copy.feedback_message,
                            );
                        }
                    }
                }
                true
            }
            KeyCode::KeyL => {
                if self.debug_profiling_recording {
                    self.debug_profiling_recording = false;
                    self.debug_profiling_toggle_requests
                        .push(DebugProfilingToggleRequest::Stop);
                    push_debug_profiling_stop_feedback(world.as_deref_mut());
                } else {
                    self.debug_profiling_recording = true;
                    self.debug_profiling_toggle_requests
                        .push(DebugProfilingToggleRequest::Start);
                    push_debug_feedback_chat_message(
                        world.as_deref_mut(),
                        "Profiling started for 10 seconds. Use F3 + L to stop early",
                    );
                }
                true
            }
            KeyCode::F6 => {
                self.debug_options_screen_requests =
                    self.debug_options_screen_requests.saturating_add(1);
                true
            }
            KeyCode::KeyC => {
                if let (Some(command), Some(clipboard)) = (
                    world.as_deref().and_then(debug_copy_location_command),
                    clipboard.as_deref_mut(),
                ) {
                    if clipboard.set_debug_clipboard_text(&command) {
                        push_debug_feedback_chat_message(
                            world.as_deref_mut(),
                            "Copied location to clipboard",
                        );
                    }
                }
                // Vanilla shares C between copy-location and the manual crash key,
                // so F3+C still counts as a debug action even when no copy happens.
                true
            }
            _ => false,
        }
    }

    fn handle_shared_debug_hotkey(
        &mut self,
        code: KeyCode,
        mut world: Option<&mut WorldStore>,
    ) -> bool {
        match code {
            KeyCode::KeyE => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionPaths);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "SectionPath: shown"
                    } else {
                        "SectionPath: hidden"
                    },
                );
                true
            }
            KeyCode::KeyF => {
                self.debug_fog_enabled = !self.debug_fog_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_fog_enabled {
                        "Fog: enabled"
                    } else {
                        "Fog: disabled"
                    },
                );
                true
            }
            KeyCode::KeyL => {
                self.debug_smart_cull_enabled = !self.debug_smart_cull_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_smart_cull_enabled {
                        "SmartCull: enabled"
                    } else {
                        "SmartCull: disabled"
                    },
                );
                true
            }
            KeyCode::KeyO => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled =
                    self.toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionOctree);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "Frustum culling Octree: enabled"
                    } else {
                        "Frustum culling Octree: disabled"
                    },
                );
                true
            }
            KeyCode::KeyU => {
                let (request, feedback) = if self.shift_down() {
                    (DebugFrustumRequest::Kill, "Killed frustum")
                } else {
                    (DebugFrustumRequest::Capture, "Captured frustum")
                };
                self.debug_frustum_requests.push(request);
                push_debug_feedback_chat_message(world.as_deref_mut(), feedback);
                true
            }
            KeyCode::KeyV => {
                let Some(world) = world.as_deref_mut() else {
                    return false;
                };
                if world.local_player_id().is_none() {
                    return false;
                }
                let enabled = self
                    .toggle_debug_screen_entry_status(DebugScreenEntryId::ChunkSectionVisibility);
                push_debug_feedback_chat_message(
                    Some(world),
                    if enabled {
                        "SectionVisibility: enabled"
                    } else {
                        "SectionVisibility: disabled"
                    },
                );
                true
            }
            KeyCode::KeyW => {
                self.debug_wireframe_enabled = !self.debug_wireframe_enabled;
                push_debug_feedback_chat_message(
                    world.as_deref_mut(),
                    if self.debug_wireframe_enabled {
                        "WireFrame: enabled"
                    } else {
                        "WireFrame: disabled"
                    },
                );
                true
            }
            _ => false,
        }
    }

    fn handle_debug_feature_count_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::KeyL => {
                self.debug_feature_count_requests
                    .push(DebugFeatureCountRequest::Log);
                true
            }
            KeyCode::KeyR => {
                self.debug_feature_count_requests
                    .push(DebugFeatureCountRequest::Clear);
                true
            }
            _ => false,
        }
    }

    fn reset_debug_crash_hold(&mut self) {
        self.debug_crash_started_at = None;
        self.debug_crash_last_reported_at = None;
        self.debug_crash_report_count = 0;
    }

    fn advance_debug_crash_hold(&mut self, world: &mut WorldStore, now: Instant) {
        if !(self.debug_modifier_down && self.pressed_keys.contains(&KeyCode::KeyC)) {
            self.reset_debug_crash_hold();
            return;
        }

        let started_at = match self.debug_crash_started_at {
            Some(started_at) => started_at,
            None => {
                self.debug_crash_started_at = Some(now);
                self.debug_crash_last_reported_at = Some(now);
                self.debug_crash_report_count = 0;
                return;
            }
        };
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed > DEBUG_CRASH_TIME {
            panic!("Manually triggered debug crash");
        }

        let last_reported_at = self.debug_crash_last_reported_at.unwrap_or(started_at);
        if now.saturating_duration_since(last_reported_at) < DEBUG_CRASH_REPORT_INTERVAL {
            return;
        }

        if self.debug_crash_report_count == 0 {
            push_debug_feedback_chat_message(
                Some(world),
                "F3 + C is held down. This will crash the game unless released.",
            );
        } else {
            let remaining = DEBUG_CRASH_TIME.saturating_sub(elapsed);
            let remaining_seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
            push_debug_feedback_chat_message(
                Some(world),
                &format!("Crashing in {remaining_seconds}..."),
            );
        }

        self.debug_crash_last_reported_at = Some(now);
        self.debug_crash_report_count = self.debug_crash_report_count.saturating_add(1);
    }

    fn debug_world_status_toggles_allowed(world: Option<&WorldStore>) -> bool {
        world.is_some_and(|world| {
            world.local_player_id().is_some() && !world.local_player_has_reduced_debug_info()
        })
    }

    fn debug_game_mode_switcher_can_open(&self, world: &WorldStore) -> bool {
        world.level_info().is_some() && !self.debug_game_mode_switcher_blocked_by_screen(world)
    }

    fn debug_game_mode_switcher_blocked_by_screen(&self, world: &WorldStore) -> bool {
        self.command_entry_is_active()
            || self.sign_editor_is_active_or_pending(world)
            || world.open_container_id().is_some()
            || world.dialog_screen_is_open()
            || world.current_book().is_some()
            || world.advancements_screen_is_open()
            || world.stats_screen_is_open()
            || world.game_rules_screen().is_some()
    }

    fn next_debug_query_transaction_id(&mut self) -> i32 {
        self.debug_query_transaction_id = self.debug_query_transaction_id.saturating_add(1);
        self.debug_query_transaction_id
    }

    fn toggle_debug_profiler_chart(&mut self) {
        self.debug_profiler_chart_visible =
            !self.debug_overlay_visible() || !self.debug_profiler_chart_visible;
        if self.debug_profiler_chart_visible {
            self.debug_entries.set_overlay_visible(true);
        }
    }

    fn toggle_debug_fps_charts(&mut self) {
        self.debug_fps_charts_visible =
            !self.debug_overlay_visible() || !self.debug_fps_charts_visible;
        if self.debug_fps_charts_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_network_charts_visible = false;
            self.debug_lightmap_texture_visible = false;
        }
    }

    fn toggle_debug_network_charts(&mut self) {
        self.debug_network_charts_visible =
            !self.debug_overlay_visible() || !self.debug_network_charts_visible;
        if self.debug_network_charts_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_fps_charts_visible = false;
            self.debug_lightmap_texture_visible = false;
        }
    }

    fn toggle_debug_lightmap_texture(&mut self) {
        self.debug_lightmap_texture_visible =
            !self.debug_overlay_visible() || !self.debug_lightmap_texture_visible;
        if self.debug_lightmap_texture_visible {
            self.debug_entries.set_overlay_visible(true);
            self.debug_fps_charts_visible = false;
            self.debug_network_charts_visible = false;
        }
    }

    fn record_debug_profiler_chart_navigation_key(&mut self, code: KeyCode) {
        if !self.debug_profiler_chart_visible() || self.debug_modifier_down {
            return;
        }
        if let Some(digit) = debug_profiler_chart_digit(code) {
            self.debug_profiler_chart_navigation_requests.push(digit);
        }
    }
}

fn debug_profiler_chart_digit(code: KeyCode) -> Option<u8> {
    Some(match code {
        KeyCode::Digit0 => 0,
        KeyCode::Digit1 => 1,
        KeyCode::Digit2 => 2,
        KeyCode::Digit3 => 3,
        KeyCode::Digit4 => 4,
        KeyCode::Digit5 => 5,
        KeyCode::Digit6 => 6,
        KeyCode::Digit7 => 7,
        KeyCode::Digit8 => 8,
        KeyCode::Digit9 => 9,
        _ => return None,
    })
}

fn debug_spectate_target_game_mode(world: &WorldStore) -> GameType {
    if world.local_player_is_spectator() {
        world
            .gameplay()
            .previous_game_type
            .map(GameType::from_id)
            .unwrap_or(GameType::Creative)
    } else {
        GameType::Spectator
    }
}

fn debug_options_screen_rows(search: &str) -> Vec<DebugOptionsScreenRow> {
    let mut rows = Vec::new();
    let mut current_category = None;
    for entry in DebugScreenEntryId::all_debug_options_ordered()
        .iter()
        .copied()
        .filter(|entry| entry.path().contains(search))
    {
        let category = entry.category_label();
        if current_category != Some(category) {
            rows.push(DebugOptionsScreenRow::Category { label: category });
            current_category = Some(category);
        }
        rows.push(DebugOptionsScreenRow::Entry(entry));
    }
    rows
}

fn insert_debug_options_search_text(screen: &mut DebugOptionsScreenState, text: &str) {
    let before = screen.search_text.clone();
    delete_debug_options_search_selection(screen);
    screen.search_cursor = screen
        .search_cursor
        .min(text_edit::char_len(&screen.search_text));
    let mut remaining = DEBUG_OPTIONS_SEARCH_MAX_LENGTH
        .saturating_sub(debug_options_search_len(&screen.search_text));
    for ch in text.chars().filter(|ch| is_chat_text_char(*ch)) {
        let len = ch.len_utf16();
        if len > remaining {
            break;
        }
        let insert_at = text_edit::byte_index(&screen.search_text, screen.search_cursor);
        screen.search_text.insert(insert_at, ch);
        screen.search_cursor += 1;
        remaining -= len;
    }
    screen.search_selection = screen.search_cursor;
    if screen.search_text != before {
        screen.scroll_row = 0;
    }
}

fn move_debug_options_search_cursor(
    screen: &mut DebugOptionsScreenState,
    cursor: usize,
    keep_selection: bool,
) {
    screen.search_cursor = cursor.min(text_edit::char_len(&screen.search_text));
    if !keep_selection {
        screen.search_selection = screen.search_cursor;
    }
}

fn select_debug_options_search_word_at(screen: &mut DebugOptionsScreenState, cursor: usize) {
    let word_start = text_edit::word_position(&screen.search_text, cursor, -1);
    let word_end = text_edit::word_position(&screen.search_text, cursor, 1);
    move_debug_options_search_cursor(screen, word_start, false);
    move_debug_options_search_cursor(screen, word_end, true);
}

fn select_debug_options_search_text(screen: &mut DebugOptionsScreenState) {
    screen.search_selection = 0;
    screen.search_cursor = text_edit::char_len(&screen.search_text);
}

fn debug_options_search_selected_text(screen: &DebugOptionsScreenState) -> String {
    if screen.search_selection == screen.search_cursor {
        return String::new();
    }
    let start = screen.search_selection.min(screen.search_cursor);
    let end = screen.search_selection.max(screen.search_cursor);
    let start_byte = text_edit::byte_index(&screen.search_text, start);
    let end_byte = text_edit::byte_index(&screen.search_text, end);
    screen.search_text[start_byte..end_byte].to_string()
}

fn delete_debug_options_search_selection(screen: &mut DebugOptionsScreenState) -> bool {
    if screen.search_selection == screen.search_cursor {
        return false;
    }
    let start = screen.search_selection.min(screen.search_cursor);
    let end = screen.search_selection.max(screen.search_cursor);
    let start_byte = text_edit::byte_index(&screen.search_text, start);
    let end_byte = text_edit::byte_index(&screen.search_text, end);
    screen.search_text.replace_range(start_byte..end_byte, "");
    screen.search_cursor = start;
    screen.search_selection = start;
    true
}

fn remove_debug_options_search_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(current, *cursor - 1);
    let end = text_edit::byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_debug_options_search_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(current) {
        return;
    }
    let start = text_edit::byte_index(current, cursor);
    let end = text_edit::byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn debug_options_search_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn debug_options_visible_row_count(surface_size: PhysicalSize<u32>) -> usize {
    (debug_options_list_height(surface_size) / DEBUG_OPTIONS_ROW_HEIGHT).max(0) as usize
}

fn debug_options_max_scroll_row(total_rows: usize, visible_rows: usize) -> usize {
    total_rows.saturating_sub(visible_rows)
}

fn debug_options_list_height(surface_size: PhysicalSize<u32>) -> i32 {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (height - DEBUG_OPTIONS_HEADER_HEIGHT - DEBUG_OPTIONS_FOOTER_HEIGHT).max(0)
}

fn debug_options_content_x(surface_size: PhysicalSize<u32>) -> i32 {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    width / 2 - DEBUG_OPTIONS_ROW_WIDTH / 2
}

fn debug_options_search_box_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    (
        debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
            - DEBUG_OPTIONS_SEARCH_WIDTH,
        6,
        DEBUG_OPTIONS_SEARCH_WIDTH,
        DEBUG_OPTIONS_SEARCH_HEIGHT,
    )
}

fn debug_options_scrollbar_x(surface_size: PhysicalSize<u32>) -> i32 {
    debug_options_content_x(surface_size)
        + DEBUG_OPTIONS_ROW_WIDTH
        + DEBUG_OPTIONS_SCROLLBAR_WIDTH
        + 2
}

fn debug_options_scrollbar_contains(
    mouse_x: i32,
    mouse_y: i32,
    search_text: &str,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let total_rows = debug_options_screen_rows(search_text).len();
    let list_height = debug_options_list_height(surface_size);
    if !debug_options_scrollbar_scrollable(total_rows, list_height) {
        return false;
    }
    let scrollbar_x = debug_options_scrollbar_x(surface_size);
    mouse_x >= scrollbar_x
        && mouse_x <= scrollbar_x + DEBUG_OPTIONS_SCROLLBAR_WIDTH
        && mouse_y >= DEBUG_OPTIONS_HEADER_HEIGHT
        && mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT + list_height
}

fn drag_debug_options_scrollbar(
    screen: &mut DebugOptionsScreenState,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) {
    let Some(last_y) = screen.scrollbar_drag_y else {
        return;
    };
    let total_rows = debug_options_screen_rows(&screen.search_text).len();
    screen.scroll_row = debug_options_scrollbar_drag_scroll_row(
        screen.scroll_row,
        total_rows,
        surface_size,
        last_y,
        mouse_y,
    );
    screen.scrollbar_drag_y = Some(mouse_y);
}

fn debug_options_scrollbar_drag_scroll_row(
    scroll_row: usize,
    total_rows: usize,
    surface_size: PhysicalSize<u32>,
    last_y: i32,
    mouse_y: i32,
) -> usize {
    let visible_rows = debug_options_visible_row_count(surface_size);
    let max_scroll_row = debug_options_max_scroll_row(total_rows, visible_rows);
    if max_scroll_row == 0 {
        return 0;
    }
    let list_height = debug_options_list_height(surface_size);
    if mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT {
        return 0;
    }
    if mouse_y > DEBUG_OPTIONS_HEADER_HEIGHT + list_height {
        return max_scroll_row;
    }
    let content_height = debug_options_scrollbar_content_height(total_rows);
    let max_scroll_amount = (content_height - list_height).max(0);
    if max_scroll_amount == 0 {
        return 0;
    }
    let scroller_height = debug_options_scroller_height(list_height, content_height);
    let y_drag_scale =
        f64::from(max_scroll_amount.max(1)) / f64::from((list_height - scroller_height).max(1));
    let y_drag_scale = y_drag_scale.max(1.0);
    let scroll_amount = i32::try_from(scroll_row)
        .unwrap_or(i32::MAX / DEBUG_OPTIONS_ROW_HEIGHT)
        .saturating_mul(DEBUG_OPTIONS_ROW_HEIGHT)
        .min(max_scroll_amount);
    let dragged = f64::from(scroll_amount) + f64::from(mouse_y - last_y) * y_drag_scale;
    ((dragged.clamp(0.0, f64::from(max_scroll_amount)) / f64::from(DEBUG_OPTIONS_ROW_HEIGHT))
        .round() as usize)
        .min(max_scroll_row)
}

fn debug_options_scrollbar_scrollable(total_rows: usize, list_height: i32) -> bool {
    list_height > 8 && debug_options_scrollbar_content_height(total_rows) > list_height
}

fn debug_options_scrollbar_content_height(total_rows: usize) -> i32 {
    i32::try_from(total_rows)
        .unwrap_or(i32::MAX / DEBUG_OPTIONS_ROW_HEIGHT)
        .saturating_mul(DEBUG_OPTIONS_ROW_HEIGHT)
}

fn debug_options_scroller_height(list_height: i32, content_height: i32) -> i32 {
    let max_scroller_height = (list_height - 8).max(1);
    let min_scroller_height = DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT.min(max_scroller_height);
    list_height
        .saturating_mul(list_height)
        .checked_div(content_height.max(1))
        .unwrap_or(max_scroller_height)
        .clamp(min_scroller_height, max_scroller_height)
}

fn debug_options_search_cursor_at_with_text_measurer(
    screen: &DebugOptionsScreenState,
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DebugOptionsSearchTextMeasurer,
) -> Option<usize> {
    let (x, y, width, height) = debug_options_search_box_rect(surface_size);
    if mouse_x < x || mouse_x >= x + width || mouse_y < y || mouse_y >= y + height {
        return None;
    }
    Some(debug_options_search_cursor_for_mouse_x_with_text_measurer(
        screen,
        mouse_x,
        surface_size,
        text_measurer,
    ))
}

fn debug_options_search_cursor_for_mouse_x_with_text_measurer(
    screen: &DebugOptionsScreenState,
    mouse_x: i32,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DebugOptionsSearchTextMeasurer,
) -> usize {
    let (x, _, _, _) = debug_options_search_box_rect(surface_size);
    let position_in_text = (mouse_x - x - DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET)
        .clamp(0, DEBUG_OPTIONS_SEARCH_INNER_WIDTH);
    let display_start = debug_options_search_display_start(screen, text_measurer);
    text_measurer.debug_options_search_cursor_for_text_offset_from_display_start(
        &screen.search_text,
        display_start,
        position_in_text,
    )
}

fn debug_options_search_display_start(
    screen: &DebugOptionsScreenState,
    text_measurer: &dyn DebugOptionsSearchTextMeasurer,
) -> usize {
    let scroll_to = if screen.search_selection != screen.search_cursor {
        screen.search_selection
    } else {
        screen.search_cursor
    };
    text_measurer.debug_options_search_display_start_for_width(
        &screen.search_text,
        scroll_to,
        DEBUG_OPTIONS_SEARCH_INNER_WIDTH,
    )
}

fn debug_options_screen_is_double_click(screen: &DebugOptionsScreenState, now: Instant) -> bool {
    screen
        .last_left_click_at
        .and_then(|last| now.checked_duration_since(last))
        .is_some_and(|elapsed| elapsed < DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD)
}

pub(crate) fn debug_options_search_cursor_for_text_offset_fallback(
    search_text: &str,
    offset: i32,
) -> usize {
    let mut width = 0;
    let mut cursor = 0;
    for ch in search_text.chars() {
        let advance = debug_options_search_char_advance(ch);
        if width + advance > offset {
            break;
        }
        width += advance;
        cursor += 1;
    }
    cursor
}

pub(crate) fn debug_options_search_cursor_for_text_offset_from_display_start_fallback(
    search_text: &str,
    display_start: usize,
    offset: i32,
) -> usize {
    let display_start = display_start.min(text_edit::char_len(search_text));
    let start = text_edit::byte_index(search_text, display_start);
    display_start
        + debug_options_search_cursor_for_text_offset_fallback(&search_text[start..], offset)
}

pub(crate) fn debug_options_search_display_start_for_width_fallback(
    search_text: &str,
    scroll_to: usize,
    width: i32,
) -> usize {
    let scroll_to = scroll_to.min(text_edit::char_len(search_text));
    if debug_options_search_prefix_width(search_text, scroll_to) <= width {
        return 0;
    }

    let chars = search_text.chars().take(scroll_to).collect::<Vec<_>>();
    let mut start = chars.len();
    let mut used_width = 0;
    while start > 0 {
        let advance = debug_options_search_char_advance(chars[start - 1]);
        if used_width + advance > width {
            break;
        }
        used_width += advance;
        start -= 1;
    }
    start
}

fn debug_options_search_prefix_width(search_text: &str, char_count: usize) -> i32 {
    search_text
        .chars()
        .take(char_count)
        .map(debug_options_search_char_advance)
        .sum()
}

fn debug_options_search_char_advance(ch: char) -> i32 {
    if ch == ' ' {
        4
    } else {
        DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE
    }
}

fn debug_options_row_index_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<usize> {
    let x = debug_options_content_x(surface_size);
    if mouse_x < x || mouse_x >= x + DEBUG_OPTIONS_ROW_WIDTH {
        return None;
    }
    if mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT {
        return None;
    }
    let row_index = (mouse_y - DEBUG_OPTIONS_HEADER_HEIGHT) / DEBUG_OPTIONS_ROW_HEIGHT;
    (row_index >= 0 && (row_index as usize) < debug_options_visible_row_count(surface_size))
        .then_some(row_index as usize)
}

fn debug_options_not_allowed_tooltip_at(
    search_text: &str,
    scroll_row: usize,
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
    reduced_debug_info: bool,
) -> Option<(i32, i32)> {
    let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
    let rows = debug_options_screen_rows(search_text);
    let visible_rows = debug_options_visible_row_count(surface_size);
    let scroll_row = scroll_row.min(debug_options_max_scroll_row(rows.len(), visible_rows));
    let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
        return None;
    };
    if entry.is_allowed(reduced_debug_info) {
        return None;
    }
    let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
        - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    (mouse_x < buttons_start_x).then_some((mouse_x, mouse_y))
}

fn debug_options_footer_button_y(surface_size: PhysicalSize<u32>) -> i32 {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    height - DEBUG_OPTIONS_FOOTER_HEIGHT + 6
}

fn debug_options_footer_button_xs(surface_size: PhysicalSize<u32>) -> (i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let total_width = DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH * 2
        + DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING * 2;
    let default_x = width / 2 - total_width / 2;
    let performance_x =
        default_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    let done_x =
        performance_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH + DEBUG_OPTIONS_FOOTER_BUTTON_SPACING;
    (default_x, performance_x, done_x)
}

fn debug_options_profile_button_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<DebugScreenProfile> {
    let y = debug_options_footer_button_y(surface_size);
    if mouse_y < y || mouse_y >= y + DEBUG_OPTIONS_ROW_HEIGHT {
        return None;
    }
    let (default_x, performance_x, _) = debug_options_footer_button_xs(surface_size);
    if mouse_x >= default_x && mouse_x < default_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Default);
    }
    if mouse_x >= performance_x && mouse_x < performance_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Performance);
    }
    None
}

fn debug_options_done_button_contains(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let y = debug_options_footer_button_y(surface_size);
    let (_, _, done_x) = debug_options_footer_button_xs(surface_size);
    mouse_x >= done_x
        && mouse_x < done_x + DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        && mouse_y >= y
        && mouse_y < y + DEBUG_OPTIONS_ROW_HEIGHT
}

pub(crate) fn pause_screen_return_to_game_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_return_to_game_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_advancements_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_advancements_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_stats_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_stats_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_send_feedback_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_send_feedback_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_report_bugs_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_report_bugs_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_report_bugs_button_enabled() -> bool {
    MC_DATA_VERSION_SERIES == "main"
}

pub(crate) fn pause_screen_options_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_options_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_options_button_enabled() -> bool {
    false
}

pub(crate) fn pause_screen_player_reporting_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_player_reporting_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_player_reporting_button_enabled() -> bool {
    false
}

pub(crate) fn pause_screen_disconnect_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = pause_screen_disconnect_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

pub(crate) fn pause_screen_feedback_url() -> &'static str {
    if MC_STABLE {
        PAUSE_SCREEN_RELEASE_FEEDBACK_URL
    } else {
        PAUSE_SCREEN_SNAPSHOT_FEEDBACK_URL
    }
}

pub(crate) fn stats_screen_done_button_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let (x, y, width, height) = stats_screen_done_button_rect(surface_size);
    mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
}

fn pause_screen_return_to_game_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + PAUSE_SCREEN_RETURN_TO_GAME_TOP_OFFSET,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_advancements_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + PAUSE_SCREEN_SECOND_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_stats_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        height / 4 + PAUSE_SCREEN_SECOND_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_send_feedback_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + PAUSE_SCREEN_THIRD_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_report_bugs_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        height / 4 + PAUSE_SCREEN_THIRD_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_options_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + PAUSE_SCREEN_FOURTH_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_player_reporting_button_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 + 4,
        height / 4 + PAUSE_SCREEN_FOURTH_ROW_TOP_OFFSET,
        PAUSE_SCREEN_HALF_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn pause_screen_disconnect_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH / 2,
        height / 4 + PAUSE_SCREEN_DISCONNECT_ROW_TOP_OFFSET,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_WIDTH,
        PAUSE_SCREEN_RETURN_TO_GAME_BUTTON_HEIGHT,
    )
}

fn stats_screen_done_button_rect(surface_size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (
        width / 2 - STATS_SCREEN_DONE_BUTTON_WIDTH / 2,
        height - STATS_SCREEN_FOOTER_HEIGHT
            + (STATS_SCREEN_FOOTER_HEIGHT - STATS_SCREEN_DONE_BUTTON_HEIGHT) / 2,
        STATS_SCREEN_DONE_BUTTON_WIDTH,
        STATS_SCREEN_DONE_BUTTON_HEIGHT,
    )
}

fn stats_screen_tab_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    if !(STATS_SCREEN_TAB_Y..STATS_SCREEN_TAB_Y + STATS_SCREEN_TAB_HEIGHT).contains(&mouse_y) {
        return None;
    }
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let start_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
    if !(start_x..start_x + STATS_SCREEN_LIST_WIDTH).contains(&mouse_x) {
        return None;
    }
    let relative_x = mouse_x - start_x;
    let tab_width = STATS_SCREEN_LIST_WIDTH / 3;
    Some(if relative_x < tab_width {
        0
    } else if relative_x < tab_width * 2 {
        1
    } else {
        2
    })
}

fn stats_screen_item_column_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    let list_bottom = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
    if !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
        || !(STATS_SCREEN_LIST_TOP + STATS_SCREEN_LIST_ENTRY_PADDING + 1
            ..STATS_SCREEN_LIST_TOP
                + STATS_SCREEN_LIST_ENTRY_PADDING
                + 1
                + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE)
            .contains(&(mouse_y.saturating_add(scroll_amount)))
    {
        return None;
    }
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
    (0..6).find(|column| {
        let x = list_x
            + STATS_SCREEN_ITEM_COLUMN_FIRST_X
            + i32::try_from(*column).unwrap_or(i32::MAX) * STATS_SCREEN_ITEM_COLUMN_STRIDE;
        (x..x + STATS_SCREEN_ITEM_COLUMN_BUTTON_SIZE).contains(&mouse_x)
    })
}

fn stats_screen_row_at_position(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    tab: usize,
    scroll_amount: i32,
) -> Option<usize> {
    let (mouse_x, mouse_y) = cursor_position?;
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let list_x = width / 2 - STATS_SCREEN_LIST_WIDTH / 2;
    let list_bottom = i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(STATS_SCREEN_FOOTER_HEIGHT);
    if !(list_x..list_x + STATS_SCREEN_LIST_WIDTH).contains(&mouse_x)
        || !(STATS_SCREEN_LIST_TOP..list_bottom).contains(&mouse_y)
    {
        return None;
    }
    let content_y =
        mouse_y - STATS_SCREEN_LIST_TOP - STATS_SCREEN_LIST_ENTRY_PADDING + scroll_amount;
    let row_height = stats_screen_row_height(tab);
    (content_y >= 0)
        .then(|| usize::try_from(content_y / row_height).ok())
        .flatten()
}

fn stats_screen_row_height(tab: usize) -> i32 {
    match tab {
        1 => 22,
        2 => 36,
        _ => STATS_SCREEN_ROW_HEIGHT,
    }
}

fn stats_screen_scrollbar_contains(
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some((mouse_x, mouse_y)) = cursor_position else {
        return false;
    };
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let x = width / 2 + STATS_SCREEN_LIST_WIDTH / 2 + STATS_SCREEN_SCROLLBAR_WIDTH + 2;
    (x..=x + STATS_SCREEN_SCROLLBAR_WIDTH).contains(&mouse_x)
        && (STATS_SCREEN_LIST_TOP..height.saturating_sub(STATS_SCREEN_FOOTER_HEIGHT))
            .contains(&mouse_y)
}

fn debug_options_scroll_delta_rows(delta: MouseScrollDelta) -> i32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => y.round() as i32,
        MouseScrollDelta::PixelDelta(position) => {
            (position.y / f64::from(DEBUG_OPTIONS_ROW_HEIGHT))
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        }
    }
}

fn default_debug_game_mode_switcher_selection(world: &WorldStore) -> GameType {
    if let Some(previous_game_type) = world.gameplay().previous_game_type {
        GameType::from_id(previous_game_type)
    } else if GameType::from_id(world.gameplay().game_type) == GameType::Creative {
        GameType::Survival
    } else {
        GameType::Creative
    }
}

fn next_debug_game_mode_icon(game_type: GameType) -> GameType {
    match game_type {
        GameType::Creative => GameType::Survival,
        GameType::Survival => GameType::Adventure,
        GameType::Adventure => GameType::Spectator,
        GameType::Spectator => GameType::Creative,
    }
}

fn debug_game_mode_switcher_mouse_pos(position: PhysicalPosition<f64>) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

fn debug_game_mode_switcher_hovered_game_type(
    mouse_pos: (i32, i32),
    surface_size: PhysicalSize<u32>,
) -> Option<GameType> {
    const MODES: [GameType; 4] = [
        GameType::Creative,
        GameType::Survival,
        GameType::Adventure,
        GameType::Spectator,
    ];

    let center_x = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let center_y = i32::try_from(surface_size.height / 2).unwrap_or(i32::MAX);
    let slot_start_x = center_x - DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH / 2;
    let slot_y = center_y - DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET;
    let (mouse_x, mouse_y) = mouse_pos;
    MODES.iter().enumerate().find_map(|(index, mode)| {
        let slot_x = slot_start_x.saturating_add(
            i32::try_from(index).unwrap_or(i32::MAX) * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED,
        );
        let slot_right = slot_x.saturating_add(DEBUG_GAME_MODE_SWITCHER_SLOT_AREA);
        let slot_bottom = slot_y.saturating_add(DEBUG_GAME_MODE_SWITCHER_SLOT_AREA);
        (mouse_x >= slot_x && mouse_x < slot_right && mouse_y >= slot_y && mouse_y < slot_bottom)
            .then_some(*mode)
    })
}

fn queue_debug_game_mode_switcher_selection(
    world: &WorldStore,
    context: &mut DebugNetContext<'_>,
    selected: GameType,
) {
    if !world.local_player_has_gamemaster_permission()
        || GameType::from_id(world.gameplay().game_type) == selected
    {
        return;
    }
    queue_change_game_mode_command(
        context.counters,
        context.net_commands,
        ChangeGameModeCommand {
            game_mode: selected,
        },
    );
}

fn push_debug_version_chat_messages(world: &mut WorldStore) {
    push_debug_feedback_chat_message(Some(world), "Client version info:");
    world.push_client_system_chat_message(format!("id = {MC_VERSION}"));
    world.push_client_system_chat_message(format!("name = {MC_VERSION}"));
    world.push_client_system_chat_message(format!("data = {MC_DATA_VERSION}"));
    world.push_client_system_chat_message(format!("series = {MC_DATA_VERSION_SERIES}"));
    world.push_client_system_chat_message(format!(
        "protocol = {PROTOCOL_VERSION} (0x{PROTOCOL_VERSION:x})"
    ));
    world.push_client_system_chat_message(format!("build_time = {MC_BUILD_TIME}"));
    world.push_client_system_chat_message(format!(
        "pack_resource = {}",
        MC_RESOURCE_PACK_FORMAT.to_vanilla_string()
    ));
    world.push_client_system_chat_message(format!(
        "pack_data = {}",
        MC_DATA_PACK_FORMAT.to_vanilla_string()
    ));
    world.push_client_system_chat_message(if MC_STABLE {
        "stable = yes"
    } else {
        "stable = no"
    });
}

fn push_debug_feedback_chat_message(world: Option<&mut WorldStore>, message: &str) {
    push_debug_feedback_chat_runs(
        world,
        vec![StyledTextRun {
            text: message.to_string(),
            style: ComponentStyle::default(),
        }],
    );
}

fn push_debug_dynamic_texture_dump_feedback(world: Option<&mut WorldStore>) {
    push_debug_feedback_chat_runs(
        world,
        vec![
            StyledTextRun {
                text: "Saved dynamic textures to ".to_string(),
                style: ComponentStyle::default(),
            },
            StyledTextRun {
                text: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
                style: ComponentStyle {
                    underlined: Some(true),
                    click_event: Some(ComponentClickEvent::OpenFile {
                        path: DEBUG_DYNAMIC_TEXTURE_DUMP_RELATIVE_PATH.to_string(),
                    }),
                    ..ComponentStyle::default()
                },
            },
        ],
    );
}

fn push_debug_profiling_stop_feedback(world: Option<&mut WorldStore>) {
    push_debug_feedback_chat_runs(
        world,
        vec![
            StyledTextRun {
                text: "Profiling ended. Results folder ".to_string(),
                style: ComponentStyle::default(),
            },
            StyledTextRun {
                text: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
                style: ComponentStyle {
                    underlined: Some(true),
                    click_event: Some(ComponentClickEvent::OpenFile {
                        path: DEBUG_PROFILING_RESULTS_RELATIVE_DIR.to_string(),
                    }),
                    ..ComponentStyle::default()
                },
            },
        ],
    );
}

fn push_debug_feedback_chat_runs(world: Option<&mut WorldStore>, body_runs: Vec<StyledTextRun>) {
    if let Some(world) = world {
        world.push_styled_client_system_chat_message(debug_feedback_styled_runs(body_runs));
    }
}

fn debug_feedback_styled_runs(body_runs: Vec<StyledTextRun>) -> Vec<StyledTextRun> {
    let mut runs = vec![
        StyledTextRun {
            text: "[Debug]:".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                color: Some(VANILLA_DEBUG_FEEDBACK_COLOR),
                ..ComponentStyle::default()
            },
        },
        StyledTextRun {
            text: " ".to_string(),
            style: ComponentStyle::default(),
        },
    ];
    runs.extend(body_runs);
    runs
}

pub(crate) fn queue_debug_recreate_server_query_request(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: DebugRecreateServerQueryRequest,
) {
    match request {
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos,
        } => queue_block_entity_tag_query_command(
            counters,
            net_commands,
            BlockEntityTagQuery {
                transaction_id,
                pos,
            },
        ),
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        } => queue_entity_tag_query_command(
            counters,
            net_commands,
            EntityTagQuery {
                transaction_id,
                entity_id,
            },
        ),
    }
}

fn debug_copy_location_command(world: &WorldStore) -> Option<String> {
    if !ClientInputState::debug_world_status_toggles_allowed(Some(world)) {
        return None;
    }
    let level = world.level_info()?;
    let pose = world.local_player_pose()?;
    Some(format!(
        "/execute in {} run tp @s {:.2} {:.2} {:.2} {:.2} {:.2}",
        level.dimension, pose.position.x, pose.position.y, pose.position.z, pose.y_rot, pose.x_rot
    ))
}

struct DebugRecreateCopy {
    command: String,
    feedback_message: &'static str,
}

fn debug_copy_recreate_command(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    match debug_recreate_target(world)? {
        DebugRecreateTarget::Block(pos) => debug_copy_recreate_block_command(world, pos, add_nbt),
        DebugRecreateTarget::Entity(entity_id) => {
            debug_copy_recreate_entity_command(world, item_runtime, entity_id, add_nbt)
        }
    }
}

fn debug_copy_recreate_server_response_command(
    target: &PendingDebugRecreateServerQueryTarget,
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    match target {
        PendingDebugRecreateServerQueryTarget::Block { pos, description } => {
            debug_copy_recreate_server_block_command(*pos, description, response)
        }
        PendingDebugRecreateServerQueryTarget::Entity {
            entity_type,
            position,
            ..
        } => debug_copy_recreate_server_entity_command(entity_type, *position, response),
    }
}

fn debug_recreate_target(world: &WorldStore) -> Option<DebugRecreateTarget> {
    if world.local_player_id().is_none() || world.local_player_has_reduced_debug_info() {
        return None;
    }
    let target =
        crosshair_target_from_camera_at_partial_tick(world, camera_pose_from_world(world), 1.0)?;
    match target {
        CrosshairTarget::Block(hit) => Some(DebugRecreateTarget::Block(hit.pos)),
        CrosshairTarget::Entity(hit) => Some(DebugRecreateTarget::Entity(hit.entity_id)),
    }
}

fn debug_recreate_server_query_request(
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> DebugRecreateServerQueryRequest {
    match target {
        DebugRecreateTarget::Block(pos) => DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos: protocol_block_pos_from_world(pos),
        },
        DebugRecreateTarget::Entity(entity_id) => DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        },
    }
}

fn pending_debug_recreate_server_query(
    world: &WorldStore,
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> Option<PendingDebugRecreateServerQuery> {
    let target = match target {
        DebugRecreateTarget::Block(pos) => {
            let block = world.probe_block(pos)?;
            let block_name = block.block_name.as_deref()?;
            PendingDebugRecreateServerQueryTarget::Block {
                pos,
                description: debug_block_state_description(block_name, &block.block_properties),
            }
        }
        DebugRecreateTarget::Entity(entity_id) => {
            let entity = world.probe_entity(entity_id)?;
            let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
            PendingDebugRecreateServerQueryTarget::Entity {
                entity_id,
                entity_uuid: entity.uuid,
                entity_type: entity_type.to_string(),
                position: [entity.position.x, entity.position.y, entity.position.z],
            }
        }
    };
    Some(PendingDebugRecreateServerQuery {
        transaction_id,
        client_level_generation: world.client_level_generation(),
        target,
    })
}

fn debug_copy_recreate_block_command(
    world: &WorldStore,
    pos: BlockPos,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let block = world.probe_block(pos)?;
    let block_name = block.block_name.as_deref()?;
    let description = debug_block_state_description(block_name, &block.block_properties);
    let mut command = format!("/setblock {} {} {} {}", pos.x, pos.y, pos.z, description);
    if add_nbt {
        if let Some(snbt) = debug_local_block_entity_compact_snbt(world, pos) {
            command.push_str(&snbt);
        }
    }
    Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied client-side block data to clipboard",
    })
}

fn debug_local_block_entity_compact_snbt(world: &WorldStore, pos: BlockPos) -> Option<String> {
    let raw_nbt = world.block_entity_raw_nbt_at(pos)?;
    let response = TagQueryResponseState {
        transaction_id: 0,
        tag_present: raw_nbt.first().is_some_and(|tag_id| *tag_id != 0),
        raw_nbt: raw_nbt.to_vec(),
    };
    response.compact_snbt().ok().flatten()
}

fn debug_copy_recreate_server_block_command(
    pos: BlockPos,
    description: &str,
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    let mut command = format!("/setblock {} {} {} {}", pos.x, pos.y, pos.z, description);
    if let Some(snbt) = response.compact_snbt()? {
        command.push_str(&snbt);
    }
    Ok(Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied server-side block data to clipboard",
    }))
}

fn debug_copy_recreate_entity_command(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let entity = world.probe_entity(entity_id)?;
    let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
    let mut command = format!(
        "/summon {} {:.2} {:.2} {:.2}",
        entity_type, entity.position.x, entity.position.y, entity.position.z
    );
    if add_nbt {
        if let Some(snbt) = debug_local_entity_pretty_snbt(world, item_runtime, &entity) {
            command.push(' ');
            command.push_str(&snbt);
        }
    }
    Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied client-side entity data to clipboard",
    })
}

fn debug_local_entity_pretty_snbt(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity: &EntityState,
) -> Option<String> {
    let accessors = vanilla_entity_metadata_entity_accessors()?;
    let mut fields = Vec::new();
    if let Some(motion) = debug_snbt_vec3d("Motion", entity.delta_movement) {
        fields.push(motion);
    }
    if let Some(rotation) = debug_snbt_rotation(entity.y_rot, entity.x_rot) {
        fields.push(rotation);
    }
    if let Some(air) = debug_entity_data_int_present(entity, accessors.air_supply_data_id) {
        fields.push(format!("Air: {}s", air as i16));
    }
    if let Some(on_ground) = entity.on_ground {
        fields.push(format!("OnGround: {}b", if on_ground { 1 } else { 0 }));
    }
    if let Some(custom_name) =
        debug_entity_data_optional_component_present(entity, accessors.custom_name_data_id)
    {
        fields.push(format!("CustomName: {}", debug_snbt_string(custom_name)));
    }
    if debug_entity_data_bool_present(entity, accessors.custom_name_visible_data_id)
        .is_some_and(|visible| visible)
    {
        fields.push("CustomNameVisible: 1b".to_string());
    }
    if debug_entity_data_bool_present(entity, accessors.silent_data_id).is_some_and(|silent| silent)
    {
        fields.push("Silent: 1b".to_string());
    }
    if debug_entity_data_bool_present(entity, accessors.no_gravity_data_id)
        .is_some_and(|no_gravity| no_gravity)
    {
        fields.push("NoGravity: 1b".to_string());
    }
    if debug_entity_data_byte_present(entity, accessors.shared_flags_data_id)
        .is_some_and(|flags| flags & accessors.glowing_mask != 0)
    {
        fields.push("Glowing: 1b".to_string());
    }
    if let Some(ticks_frozen) =
        debug_entity_data_int_present(entity, accessors.ticks_frozen_data_id)
    {
        if ticks_frozen > 0 {
            fields.push(format!("TicksFrozen: {ticks_frozen}"));
        }
    }
    debug_push_entity_additional_save_data(world, item_runtime, entity, &mut fields);
    (!fields.is_empty()).then(|| format!("{{{}}}", fields.join(", ")))
}

fn debug_push_entity_additional_save_data(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    if bbb_world::vanilla_entity_type_is_minecart(entity.entity_type_id) {
        debug_push_minecart_additional_save_data(world, entity, fields);
    }
    if bbb_world::vanilla_entity_type_is_hurting_projectile(entity.entity_type_id) {
        debug_push_abstract_hurting_projectile_additional_save_data(entity, fields);
    }
    if bbb_world::vanilla_entity_metadata_type_is_abstract_chested_horse(entity.entity_type_id) {
        debug_push_mob_additional_save_data(world, entity, fields);
        debug_push_ageable_mob_additional_save_data(entity, fields);
        debug_push_animal_additional_save_data(fields);
        debug_push_abstract_horse_additional_save_data(entity, fields);
        debug_push_abstract_chested_horse_additional_save_data(entity, fields);
    }

    match entity.entity_type_id {
        VANILLA_ENTITY_TYPE_VILLAGER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_abstract_villager_additional_save_data(fields);
            debug_push_villager_data_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zombie_additional_save_data(entity, fields);
            debug_push_zombie_villager_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_WARDEN_ID => {
            // Warden's own anger/vibration save state is server-private; only
            // the shared Mob layer is client-observable.
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ender_dragon_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID => {
            debug_push_area_effect_cloud_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID => {
            debug_push_eye_of_ender_additional_save_data(world, entity, item_runtime, fields);
        }
        VANILLA_ENTITY_TYPE_ITEM_ID => {
            debug_push_item_entity_additional_save_data(world, entity, item_runtime, fields);
        }
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID => {
            debug_push_ominous_item_spawner_additional_save_data(
                world,
                entity,
                item_runtime,
                fields,
            );
        }
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID => {
            debug_push_block_display_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID => {
            debug_push_item_display_additional_save_data(world, entity, item_runtime, fields);
        }
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID => {
            debug_push_text_display_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID => {
            // Mannequin is an Avatar, not a Mob, so no shared Mob layer.
            debug_push_mannequin_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_CREEPER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_creeper_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_CREAKING_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_creaking_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SHULKER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_shulker_additional_save_data(entity, fields);
        }
        _ if bbb_world::vanilla_entity_metadata_type_is_slime(entity.entity_type_id) => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_slime_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_snow_golem_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_BAT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_bat_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ALLAY_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_allay_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_BEE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_bee_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_armadillo_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_axolotl_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_BLAZE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_BREEZE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        _ if bbb_world::vanilla_entity_metadata_type_is_camel(entity.entity_type_id) => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_abstract_horse_additional_save_data(entity, fields);
            debug_push_camel_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_CAT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_tamable_animal_additional_save_data(entity, fields);
            debug_push_cat_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_CHICKEN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_chicken_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_copper_golem_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_COD_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_fish_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_dolphin_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_DROWNED_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zombie_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_FOX_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_fox_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_FROG_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_frog_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_COW_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_cow_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_mooshroom_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_NAUTILUS_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_tamable_animal_additional_save_data(entity, fields);
        }
        _ if bbb_world::vanilla_entity_type_is_spider(entity.entity_type_id) => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_SILVERFISH_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_SKELETON_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_skeleton_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_abstract_horse_additional_save_data(entity, fields);
            debug_push_skeleton_horse_additional_save_data(fields);
        }
        _ if bbb_world::vanilla_entity_metadata_type_is_guardian(entity.entity_type_id) => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_happy_ghast_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_HOGLIN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_hoglin_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_HORSE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_abstract_horse_additional_save_data(entity, fields);
            debug_push_horse_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_HUSK_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zombie_additional_save_data(entity, fields);
        }
        _ if bbb_world::vanilla_entity_metadata_type_is_llama(entity.entity_type_id) => {
            debug_push_llama_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_BOGGED_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_bogged_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ENDERMITE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_endermite_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_neutral_mob_additional_save_data(fields);
        }
        _ if bbb_world::vanilla_entity_type_is_spellcaster_illager(entity.entity_type_id) => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_patrolling_monster_additional_save_data(fields);
            debug_push_raider_additional_save_data(fields);
            debug_push_spellcaster_illager_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID => {
            debug_push_end_crystal_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_GHAST_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ghast_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_GIANT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_glow_squid_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_GOAT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_goat_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SQUID_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_TADPOLE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_tadpole_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_INTERACTION_ID => {
            debug_push_interaction_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => {
            debug_push_evoker_fangs_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_FIREBALL_ID => {
            debug_push_fireball_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID => {
            debug_push_firework_rocket_additional_save_data(world, entity, item_runtime, fields);
        }
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID => {
            debug_push_falling_block_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => {
            debug_push_command_block_minecart_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => {
            debug_push_furnace_minecart_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => {
            debug_push_hopper_minecart_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID => {
            debug_push_spawner_minecart_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => {
            debug_push_tnt_minecart_additional_save_data(entity, fields);
        }
        _ if bbb_world::vanilla_entity_type_is_item_frame(entity.entity_type_id) => {
            debug_push_item_frame_additional_save_data(world, entity, item_runtime, fields);
        }
        VANILLA_ENTITY_TYPE_PAINTING_ID => {
            debug_push_painting_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_TNT_ID => {
            debug_push_primed_tnt_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID => {
            debug_push_experience_orb_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID => {
            debug_push_armor_stand_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ARROW_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_abstract_arrow_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_abstract_arrow_additional_save_data(entity, fields);
            debug_push_spectral_arrow_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_TRIDENT_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_abstract_arrow_additional_save_data(entity, fields);
            debug_push_trident_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_shulker_bullet_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID => {
            debug_push_item_stack_id(fields, "Item", "minecraft:fire_charge");
        }
        VANILLA_ENTITY_TYPE_EGG_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:egg");
        }
        VANILLA_ENTITY_TYPE_ENDER_PEARL_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:ender_pearl");
        }
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:experience_bottle");
        }
        VANILLA_ENTITY_TYPE_LINGERING_POTION_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:lingering_potion");
        }
        VANILLA_ENTITY_TYPE_SNOWBALL_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:snowball");
        }
        VANILLA_ENTITY_TYPE_SPLASH_POTION_ID => {
            debug_push_projectile_additional_save_data(entity, fields);
            debug_push_item_stack_id(fields, "Item", "minecraft:splash_potion");
        }
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_iron_golem_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_OCELOT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_ocelot_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PANDA_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_panda_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PARROT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_tamable_animal_additional_save_data(entity, fields);
            debug_push_parrot_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PHANTOM_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_phantom_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PIG_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_pig_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_piglin_additional_save_data(entity, fields);
            debug_push_piglin_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_piglin_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_PILLAGER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_patrolling_monster_additional_save_data(fields);
            debug_push_raider_additional_save_data(fields);
            debug_push_inventory_carrier_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_neutral_mob_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_fish_additional_save_data(entity, fields);
            debug_push_pufferfish_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_RABBIT_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_rabbit_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SALMON_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_fish_additional_save_data(entity, fields);
            debug_push_salmon_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SHEEP_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_sheep_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_SNIFFER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_PARCHED_ID
        | VANILLA_ENTITY_TYPE_STRAY_ID
        | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_abstract_fish_additional_save_data(entity, fields);
            debug_push_tropical_fish_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_TURTLE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_turtle_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_RAVAGER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_patrolling_monster_additional_save_data(fields);
            debug_push_raider_additional_save_data(fields);
            debug_push_ravager_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_VEX_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_patrolling_monster_additional_save_data(fields);
            debug_push_raider_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_abstract_villager_additional_save_data(fields);
            debug_push_wandering_trader_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_WITCH_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_patrolling_monster_additional_save_data(fields);
            debug_push_raider_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_WOLF_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_tamable_animal_additional_save_data(entity, fields);
            debug_push_wolf_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_WITHER_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_wither_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => {
            debug_push_wither_skull_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zombie_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_abstract_horse_additional_save_data(entity, fields);
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_ageable_mob_additional_save_data(entity, fields);
            debug_push_animal_additional_save_data(fields);
            debug_push_tamable_animal_additional_save_data(entity, fields);
            debug_push_zombie_nautilus_additional_save_data(world, entity, fields);
        }
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zombie_additional_save_data(entity, fields);
            debug_push_neutral_mob_additional_save_data(fields);
        }
        VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
            debug_push_mob_additional_save_data(world, entity, fields);
            debug_push_zoglin_additional_save_data(entity, fields);
        }
        _ if debug_entity_type_writes_abstract_boat_save_data(entity.entity_type_id) => {
            debug_push_abstract_boat_additional_save_data(world, entity, fields);
        }
        _ => {}
    }
}

fn debug_push_abstract_boat_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    debug_push_leash_additional_save_data(world, entity, fields);
}

fn debug_entity_type_writes_abstract_boat_save_data(entity_type_id: i32) -> bool {
    bbb_world::vanilla_entity_type_is_boat(entity_type_id)
}

fn debug_push_evoker_fangs_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_projectile_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let lifecycle = entity.projectile_lifecycle;
    if let Some(owner_uuid) = lifecycle.and_then(|projectile| projectile.owner_uuid) {
        fields.push(format!("Owner: {}", debug_snbt_uuid_int_array(owner_uuid)));
    }
    if lifecycle.is_some_and(|projectile| projectile.left_owner) {
        fields.push(format!("LeftOwner: {}", debug_snbt_bool(true)));
    }
    if let Some(lifecycle) = lifecycle {
        fields.push(format!(
            "HasBeenShot: {}",
            debug_snbt_bool(lifecycle.has_been_shot)
        ));
    }
}

fn debug_push_abstract_arrow_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(in_ground) = debug_entity_data_bool_present(entity, accessors.in_ground_data_id) {
        fields.push(format!("inGround: {}", debug_snbt_bool(in_ground)));
    }
    if let Some(flags) = debug_entity_data_byte_present(entity, accessors.flags_data_id) {
        fields.push(format!(
            "crit: {}",
            debug_snbt_bool(flags & accessors.crit_mask != 0)
        ));
    }
    if let Some(pierce_level) =
        debug_entity_data_byte_present(entity, accessors.pierce_level_data_id)
    {
        fields.push(format!("PierceLevel: {pierce_level}b"));
    }
}

fn debug_push_spectral_arrow_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_trident_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    if let Some(projectile) = entity.projectile_lifecycle {
        fields.push(format!(
            "DealtDamage: {}",
            debug_snbt_bool(projectile.trident_dealt_damage)
        ));
    }
}

fn debug_push_abstract_hurting_projectile_additional_save_data(
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    debug_push_projectile_additional_save_data(entity, fields);
    if let Some(acceleration_power) = entity
        .hurting_projectile
        .and_then(|projectile| debug_snbt_double(projectile.acceleration_power))
    {
        fields.push(format!("acceleration_power: {acceleration_power}"));
    }
}

fn debug_push_fireball_additional_save_data(fields: &mut Vec<String>) {
    debug_push_item_stack_id(fields, "Item", "minecraft:fire_charge");
}

fn debug_push_firework_rocket_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    let Some(accessors) =
        vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    debug_push_projectile_additional_save_data(entity, fields);
    fields.push(format!(
        "Life: {}",
        entity.client_animations.age_ticks.min(i32::MAX as u32)
    ));
    if debug_entity_data_item_stack_present(entity, accessors.item_data_id).is_some() {
        fields.push(format!(
            "FireworksItem: {}",
            debug_firework_rocket_item_stack(
                entity,
                accessors,
                PersistentSnbtContext {
                    item_runtime,
                    registry_access: Some(world.frozen_registry_access()),
                },
            )
        ));
    }
    if let Some(shot_at_angle) =
        debug_entity_data_bool_present(entity, accessors.shot_at_angle_data_id)
    {
        fields.push(format!("ShotAtAngle: {}", debug_snbt_bool(shot_at_angle)));
    }
}

fn debug_firework_rocket_item_stack(
    entity: &EntityState,
    accessors: VanillaFireworkRocketMetadataAccessors,
    context: PersistentSnbtContext<'_>,
) -> String {
    let stack = debug_entity_data_item_stack_present(entity, accessors.item_data_id);
    let resource_id = stack
        .and_then(|stack| stack.item_id)
        .and_then(|item_id| {
            context
                .item_runtime
                .and_then(|runtime| runtime.item_resource_id(item_id))
        })
        .unwrap_or(accessors.item_default_resource_id);
    stack
        .and_then(|stack| {
            debug_item_stack_snbt_with_context(resource_id, stack.count, Some(stack), context)
        })
        .unwrap_or_else(|| {
            debug_item_stack_snbt_with_context(accessors.item_default_resource_id, 1, None, context)
                .expect("default firework item stack is valid")
        })
}

#[cfg(test)]
fn debug_item_stack_snbt(
    resource_id: &str,
    count: i32,
    stack: Option<&ItemStackSummary>,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_item_stack_snbt_with_context(
        resource_id,
        count,
        stack,
        PersistentSnbtContext {
            item_runtime,
            ..PersistentSnbtContext::default()
        },
    )
}

fn debug_item_stack_snbt_with_context(
    resource_id: &str,
    count: i32,
    stack: Option<&ItemStackSummary>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if resource_id == "minecraft:air" || !(1..=99).contains(&count) {
        return None;
    }
    let mut fields = vec![format!("id: {}", debug_snbt_string(resource_id))];
    if count != 1 {
        fields.push(format!("count: {count}"));
    }
    if let Some(components) = stack
        .map(|stack| &stack.component_patch)
        .and_then(|patch| debug_supported_component_patch_snbt_with_context(patch, context))
    {
        fields.push(format!("components: {components}"));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

#[cfg(test)]
fn debug_supported_component_patch_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_supported_component_patch_snbt_with_context(
        patch,
        PersistentSnbtContext {
            item_runtime,
            ..PersistentSnbtContext::default()
        },
    )
}

fn debug_supported_component_patch_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let mut fields = patch
        .added_type_ids
        .iter()
        .filter_map(|type_id| debug_supported_added_component_snbt(*type_id, patch, context))
        .collect::<Vec<_>>();
    fields.extend(patch.removed_type_ids.iter().filter_map(|type_id| {
        let descriptor =
            bbb_protocol::data_component_types::vanilla_data_component_descriptor(*type_id)?;
        descriptor.is_persistent().then(|| {
            format!(
                "{}: {{}}",
                debug_snbt_string(&format!("!{}", descriptor.resource_id()))
            )
        })
    }));
    (!fields.is_empty()).then(|| format!("{{{}}}", fields.join(", ")))
}

fn debug_supported_added_component_snbt(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let descriptor =
        bbb_protocol::data_component_types::vanilla_data_component_descriptor(type_id)?;
    let value = encode_added_component_value_snbt_with_context(type_id, patch, context)?;
    Some(format!(
        "{}: {value}",
        debug_snbt_string(descriptor.resource_id())
    ))
}

fn debug_nbt_summary_snbt(value: &NbtSummaryValue) -> String {
    match value {
        NbtSummaryValue::Byte(value) => format!("{value}b"),
        NbtSummaryValue::Short(value) => format!("{value}s"),
        NbtSummaryValue::Int(value) => value.to_string(),
        NbtSummaryValue::Long(value) => format!("{value}L"),
        NbtSummaryValue::Float(bits) => {
            format!("{}f", debug_nbt_java_float_string(f32::from_bits(*bits)))
        }
        NbtSummaryValue::Double(bits) => {
            format!("{}d", debug_nbt_java_double_string(f64::from_bits(*bits)))
        }
        NbtSummaryValue::ByteArray(values) => format!(
            "[B;{}]",
            debug_typed_nbt_array_body(values, |value| format!("{value}B"))
        ),
        NbtSummaryValue::String(value) => debug_snbt_string(value),
        NbtSummaryValue::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(debug_nbt_summary_snbt)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        NbtSummaryValue::Compound(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|entry| format!(
                    "{}: {}",
                    debug_snbt_compound_key(&entry.name),
                    debug_nbt_summary_snbt(&entry.value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        NbtSummaryValue::IntArray(values) => {
            format!("[I;{}]", debug_typed_nbt_array_body(values, i32::to_string))
        }
        NbtSummaryValue::LongArray(values) => format!(
            "[L;{}]",
            debug_typed_nbt_array_body(values, |value| format!("{value}L"))
        ),
    }
}

fn debug_typed_nbt_array_body<T>(values: &[T], format_value: impl Fn(&T) -> String) -> String {
    if values.is_empty() {
        String::new()
    } else {
        format!(
            " {}",
            values
                .iter()
                .map(format_value)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn debug_entity_data_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let resource_id = bbb_protocol::entity_types::vanilla_entity_resource_id_for_type_id(
        patch.entity_data_entity_type_id?,
    )?;
    debug_typed_entity_data_component_snbt(&resource_id, patch.entity_data_tag.as_ref()?)
}

fn debug_block_entity_data_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let resource_id =
        bbb_protocol::block_entity_types::vanilla_block_entity_resource_id_for_type_id(
            patch.block_entity_data_type_id?,
        )?;
    debug_typed_entity_data_component_snbt(resource_id, patch.block_entity_data_tag.as_ref()?)
}

fn debug_typed_entity_data_component_snbt(
    resource_id: &str,
    tag: &NbtSummaryValue,
) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = tag else {
        return None;
    };
    let mut fields = entries
        .iter()
        .filter(|entry| entry.name != "id")
        .map(|entry| {
            format!(
                "{}: {}",
                debug_snbt_compound_key(&entry.name),
                debug_nbt_summary_snbt(&entry.value)
            )
        })
        .collect::<Vec<_>>();
    fields.push(format!("id: {}", debug_snbt_string(resource_id)));
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_bees_component_snbt(bees: &[bbb_protocol::packets::BeeOccupantSummary]) -> Option<String> {
    let bees = bees
        .iter()
        .map(|bee| {
            let resource_id = bbb_protocol::entity_types::vanilla_entity_resource_id_for_type_id(
                bee.entity_type_id,
            )?;
            let entity_data = debug_typed_entity_data_component_snbt(
                &resource_id,
                bee.entity_data_tag.as_ref()?,
            )?;
            Some(format!(
                "{{entity_data: {entity_data}, ticks_in_hive: {}, min_ticks_in_hive: {}}}",
                bee.ticks_in_hive, bee.min_ticks_in_hive
            ))
        })
        .collect::<Option<Vec<_>>>()?
        .join(", ");
    Some(format!("[{bees}]"))
}

fn debug_nbt_java_float_string(value: f32) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f32::INFINITY {
        "Infinity".to_string()
    } else if value == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{value:?}")
    }
}

fn debug_nbt_java_double_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{value:?}")
    }
}

fn debug_use_effects_component_snbt(
    effects: bbb_protocol::packets::UseEffectsSummary,
) -> Option<String> {
    if !(0.0..=1.0).contains(&effects.speed_multiplier) {
        return None;
    }
    let mut fields = Vec::new();
    if effects.can_sprint {
        fields.push(format!("can_sprint: {}", debug_snbt_bool(true)));
    }
    if !effects.interact_vibrations {
        fields.push(format!("interact_vibrations: {}", debug_snbt_bool(false)));
    }
    if effects.speed_multiplier.to_bits() != 0.2_f32.to_bits() {
        fields.push(format!(
            "speed_multiplier: {}f",
            debug_nbt_java_float_string(effects.speed_multiplier)
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_food_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let nutrition = patch.food_nutrition.filter(|value| *value >= 0)?;
    let saturation = f32::from_bits(patch.food_saturation_bits?);
    let mut fields = vec![
        format!("nutrition: {nutrition}"),
        format!("saturation: {}f", debug_nbt_java_float_string(saturation)),
    ];
    if patch.food_can_always_eat {
        fields.push(format!("can_always_eat: {}", debug_snbt_bool(true)));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

#[cfg(test)]
fn debug_consumable_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    debug_consumable_component_snbt_with_context(patch, PersistentSnbtContext::default())
}

fn debug_consumable_component_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if patch.consumable_effect_count? != patch.consumable_effects.len() {
        return None;
    }
    let on_consume_effects = debug_consume_effects_snbt(&patch.consumable_effects, context)?;
    let consumable = patch.consumable?;
    let consume_seconds = debug_range_checked_float_component_snbt(
        consumable.consume_seconds.to_bits(),
        0.0,
        Some(f32::MAX),
    )?;
    let sound = patch.consumable_sound.as_ref()?;
    let has_consume_particles = patch.consumable_has_consume_particles?;
    let mut fields = Vec::new();
    if consumable.consume_seconds.to_bits() != 1.6_f32.to_bits() {
        fields.push(format!("consume_seconds: {consume_seconds}"));
    }
    if consumable.animation != bbb_protocol::packets::ItemUseAnimationSummary::Eat {
        fields.push(format!(
            "animation: {}",
            debug_snbt_string(consumable.animation.as_str())
        ));
    }
    let is_default_sound = sound.registry_id.is_some_and(|registry_id| {
        vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.generic.eat")
    });
    if !is_default_sound {
        fields.push(format!(
            "sound: {}",
            debug_sound_event_component_snbt(sound)?
        ));
    }
    if !has_consume_particles {
        fields.push("has_consume_particles: 0b".to_string());
    }
    if !patch.consumable_effects.is_empty() {
        fields.push(format!("on_consume_effects: {on_consume_effects}"));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

#[cfg(test)]
fn debug_death_protection_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    debug_death_protection_component_snbt_with_context(patch, PersistentSnbtContext::default())
}

fn debug_death_protection_component_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let death_effects = debug_consume_effects_snbt(&patch.death_protection_effects, context)?;
    if patch.death_protection_effects.is_empty() {
        Some("{}".to_string())
    } else {
        Some(format!("{{death_effects: {death_effects}}}"))
    }
}

fn debug_consume_effects_snbt(
    effects: &[ConsumeEffectSummary],
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let effects = effects
        .iter()
        .map(|effect| debug_consume_effect_snbt(effect, context))
        .collect::<Option<Vec<_>>>()?;
    Some(format!("[{}]", effects.join(", ")))
}

fn debug_consume_effect_snbt(
    effect: &ConsumeEffectSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    match effect {
        ConsumeEffectSummary::ApplyEffects {
            effects,
            probability_bits,
        } => {
            let probability =
                debug_range_checked_float_component_snbt(*probability_bits, 0.0, Some(1.0))?;
            let effects = effects
                .iter()
                .map(debug_mob_effect_instance_snbt)
                .collect::<Option<Vec<_>>>()?;
            let mut fields = vec![
                format!("type: {}", debug_snbt_string("minecraft:apply_effects")),
                format!("effects: [{}]", effects.join(", ")),
            ];
            if *probability_bits != 1.0_f32.to_bits() {
                fields.push(format!("probability: {probability}"));
            }
            Some(format!("{{{}}}", fields.join(", ")))
        }
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } => Some(format!(
            "{{type: {}, effects: {}}}",
            debug_snbt_string("minecraft:remove_effects"),
            debug_mob_effect_holder_set_snbt(effect_registry_ids, effect_tag.as_deref(), context)?
        )),
        ConsumeEffectSummary::ClearAllEffects => Some(format!(
            "{{type: {}}}",
            debug_snbt_string("minecraft:clear_all_effects")
        )),
        ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            let diameter = f32::from_bits(*diameter_bits);
            if !diameter.is_finite() || !diameter.total_cmp(&0.0).is_gt() {
                return None;
            }
            let mut fields = vec![format!(
                "type: {}",
                debug_snbt_string("minecraft:teleport_randomly")
            )];
            if *diameter_bits != 16.0_f32.to_bits() {
                fields.push(format!(
                    "diameter: {}f",
                    debug_nbt_java_float_string(diameter)
                ));
            }
            Some(format!("{{{}}}", fields.join(", ")))
        }
        ConsumeEffectSummary::PlaySound { sound } => Some(format!(
            "{{type: {}, sound: {}}}",
            debug_snbt_string("minecraft:play_sound"),
            debug_sound_event_component_snbt(sound)?
        )),
    }
}

fn debug_mob_effect_holder_set_snbt(
    effect_registry_ids: &[i32],
    effect_tag: Option<&str>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_holder_set_snbt(
        "minecraft:mob_effect",
        effect_registry_ids,
        effect_tag,
        |holder_id| vanilla_mob_effect_type(holder_id).map(|effect| effect.resource_id.to_string()),
    )
}

fn debug_mob_effect_instance_snbt(effect: &MobEffectInstanceSummary) -> Option<String> {
    let mut fields = vec![format!(
        "id: {}",
        debug_snbt_string(vanilla_mob_effect_type(effect.effect_id)?.resource_id)
    )];
    fields.extend(debug_mob_effect_details_fields(
        effect.amplifier,
        effect.duration,
        effect.ambient,
        effect.show_particles,
        effect.show_icon,
        effect.hidden_effect.as_deref(),
        0,
    )?);
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_mob_effect_details_snbt(
    details: &MobEffectDetailsSummary,
    depth: usize,
) -> Option<String> {
    let fields = debug_mob_effect_details_fields(
        details.amplifier,
        details.duration,
        details.ambient,
        details.show_particles,
        details.show_icon,
        details.hidden_effect.as_deref(),
        depth,
    )?;
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_mob_effect_details_fields(
    amplifier: i32,
    duration: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
    hidden_effect: Option<&MobEffectDetailsSummary>,
    depth: usize,
) -> Option<Vec<String>> {
    if depth > 16 || !(0..=255).contains(&amplifier) {
        return None;
    }
    let mut fields = Vec::new();
    if amplifier != 0 {
        fields.push(format!("amplifier: {amplifier}"));
    }
    if duration != 0 {
        fields.push(format!("duration: {duration}"));
    }
    if ambient {
        fields.push("ambient: 1b".to_string());
    }
    if !show_particles {
        fields.push("show_particles: 0b".to_string());
    }
    fields.push(format!("show_icon: {}", debug_snbt_bool(show_icon)));
    if let Some(hidden_effect) = hidden_effect {
        fields.push(format!(
            "hidden_effect: {}",
            debug_mob_effect_details_snbt(hidden_effect, depth + 1)?
        ));
    }
    Some(fields)
}

fn debug_potion_contents_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    if patch.potion_custom_effect_count? != patch.potion_custom_effects.len() {
        return None;
    }

    let mut fields = Vec::new();
    if let Some(potion_id) = patch.potion_id {
        fields.push(format!(
            "potion: {}",
            debug_snbt_string(vanilla_potion(potion_id)?.resource_id)
        ));
    }
    if let Some(custom_color) = patch.potion_custom_color {
        fields.push(format!("custom_color: {custom_color}"));
    }
    if !patch.potion_custom_effects.is_empty() {
        let custom_effects = patch
            .potion_custom_effects
            .iter()
            .map(debug_mob_effect_instance_snbt)
            .collect::<Option<Vec<_>>>()?;
        fields.push(format!("custom_effects: [{}]", custom_effects.join(", ")));
    }
    if let Some(custom_name) = patch.potion_custom_name.as_deref() {
        fields.push(format!("custom_name: {}", debug_snbt_string(custom_name)));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_range_checked_float_component_snbt(
    bits: u32,
    min: f32,
    max: Option<f32>,
) -> Option<String> {
    let value = f32::from_bits(bits);
    if !value.is_finite()
        || value.total_cmp(&min).is_lt()
        || max.is_some_and(|max| value.total_cmp(&max).is_gt())
    {
        return None;
    }
    Some(format!("{}f", debug_nbt_java_float_string(value)))
}

fn debug_use_cooldown_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let seconds = f32::from_bits(patch.use_cooldown_seconds_bits?);
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    let mut fields = vec![format!(
        "seconds: {}f",
        debug_nbt_java_float_string(seconds)
    )];
    if let Some(cooldown_group) = &patch.use_cooldown_group {
        fields.push(format!(
            "cooldown_group: {}",
            debug_snbt_string(cooldown_group)
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_weapon_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let item_damage_per_attack = patch
        .weapon_item_damage_per_attack
        .filter(|value| *value >= 0)?;
    let disable_blocking_for_seconds = debug_range_checked_float_component_snbt(
        patch.weapon_disable_blocking_seconds_bits?,
        0.0,
        Some(f32::MAX),
    )?;
    let mut fields = Vec::new();
    if item_damage_per_attack != 1 {
        fields.push(format!("item_damage_per_attack: {item_damage_per_attack}"));
    }
    if patch.weapon_disable_blocking_seconds_bits != Some(0.0_f32.to_bits()) {
        fields.push(format!(
            "disable_blocking_for_seconds: {disable_blocking_for_seconds}"
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_attack_range_component_snbt(
    range: bbb_protocol::packets::AttackRangeSummary,
) -> Option<String> {
    if !(0.0..=64.0).contains(&range.min_reach)
        || !(0.0..=64.0).contains(&range.max_reach)
        || !(0.0..=64.0).contains(&range.min_creative_reach)
        || !(0.0..=64.0).contains(&range.max_creative_reach)
        || !(0.0..=1.0).contains(&range.hitbox_margin)
        || !(0.0..=2.0).contains(&range.mob_factor)
    {
        return None;
    }
    let mut fields = Vec::new();
    debug_push_nondefault_snbt_float(&mut fields, "min_reach", range.min_reach, 0.0);
    debug_push_nondefault_snbt_float(&mut fields, "max_reach", range.max_reach, 3.0);
    debug_push_nondefault_snbt_float(
        &mut fields,
        "min_creative_reach",
        range.min_creative_reach,
        0.0,
    );
    debug_push_nondefault_snbt_float(
        &mut fields,
        "max_creative_reach",
        range.max_creative_reach,
        5.0,
    );
    debug_push_nondefault_snbt_float(&mut fields, "hitbox_margin", range.hitbox_margin, 0.3);
    debug_push_nondefault_snbt_float(&mut fields, "mob_factor", range.mob_factor, 1.0);
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_push_nondefault_snbt_float(
    fields: &mut Vec<String>,
    name: &str,
    value: f32,
    default: f32,
) {
    if value.to_bits() != default.to_bits() {
        fields.push(format!("{name}: {}f", debug_nbt_java_float_string(value)));
    }
}

fn debug_swing_animation_component_snbt(
    animation: bbb_protocol::packets::SwingAnimationSummary,
) -> Option<String> {
    if animation.duration <= 0 {
        return None;
    }
    let mut fields = Vec::new();
    if animation.animation_type != bbb_protocol::packets::SwingAnimationTypeSummary::Whack {
        fields.push(format!(
            "type: {}",
            debug_snbt_string(animation.animation_type.as_str())
        ));
    }
    if animation.duration != 6 {
        fields.push(format!("duration: {}", animation.duration));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_custom_model_data_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> String {
    let mut fields = Vec::new();
    if !patch.custom_model_data_floats.is_empty() {
        fields.push(format!(
            "floats: [{}]",
            patch
                .custom_model_data_floats
                .iter()
                .map(|value| format!("{}f", debug_nbt_java_float_string(*value)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !patch.custom_model_data_flags.is_empty() {
        fields.push(format!(
            "flags: [{}]",
            patch
                .custom_model_data_flags
                .iter()
                .map(|value| debug_snbt_bool(*value))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !patch.custom_model_data_strings.is_empty() {
        fields.push(format!(
            "strings: [{}]",
            patch
                .custom_model_data_strings
                .iter()
                .map(|value| debug_snbt_string(value))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !patch.custom_model_data_colors.is_empty() {
        fields.push(format!(
            "colors: [{}]",
            patch
                .custom_model_data_colors
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    format!("{{{}}}", fields.join(", "))
}

fn debug_tooltip_display_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    let hidden_components = patch
        .tooltip_hidden_component_type_ids
        .iter()
        .map(|type_id| {
            bbb_protocol::data_component_types::vanilla_data_component_resource_id_for_type_id(
                *type_id,
            )
            .map(debug_snbt_string)
        })
        .collect::<Option<Vec<_>>>()?;
    let mut fields = Vec::new();
    if patch.tooltip_hide_tooltip {
        fields.push(format!("hide_tooltip: {}", debug_snbt_bool(true)));
    }
    if !hidden_components.is_empty() {
        fields.push(format!(
            "hidden_components: [{}]",
            hidden_components.join(", ")
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_item_stack_template_list_snbt(
    templates: &[bbb_protocol::packets::ItemStackTemplateSummary],
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let items = templates
        .iter()
        .map(|template| debug_item_stack_template_snbt(template, context))
        .collect::<Option<Vec<_>>>()?
        .join(", ");
    Some(format!("[{items}]"))
}

#[cfg(test)]
fn debug_repairable_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_repairable_component_snbt_with_context(
        patch,
        PersistentSnbtContext {
            item_runtime,
            ..Default::default()
        },
    )
}

fn debug_repairable_component_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let items = context.registry_holder_set_snbt(
        "minecraft:item",
        &patch.repairable_item_registry_ids,
        patch.repairable_item_tag.as_deref(),
        |holder_id| {
            context
                .item_runtime
                .and_then(|runtime| runtime.item_resource_id(holder_id))
                .map(str::to_string)
        },
    )?;
    Some(format!("{{items: {items}}}"))
}

fn debug_damage_type_resource_id(
    holder_id: i32,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_resource_id("minecraft:damage_type", holder_id, |holder_id| {
        bbb_protocol::damage_types::vanilla_damage_type_resource_id(holder_id).map(str::to_string)
    })
}

fn debug_damage_type_holder_set_snbt(
    registry_ids: &[i32],
    tag: Option<&str>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_holder_set_snbt("minecraft:damage_type", registry_ids, tag, |holder_id| {
        bbb_protocol::damage_types::vanilla_damage_type_resource_id(holder_id).map(str::to_string)
    })
}

#[cfg(test)]
fn debug_damage_resistant_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    debug_damage_resistant_component_snbt_with_context(patch, PersistentSnbtContext::default())
}

fn debug_damage_resistant_component_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let types = debug_damage_type_holder_set_snbt(
        &patch.damage_resistant_type_registry_ids,
        patch.damage_resistant_type_tag.as_deref(),
        context,
    )?;
    Some(format!("{{types: {types}}}"))
}

#[cfg(test)]
fn debug_provides_banner_patterns_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<String> {
    debug_provides_banner_patterns_component_snbt_with_context(
        patch,
        PersistentSnbtContext::default(),
    )
}

fn debug_provides_banner_patterns_component_snbt_with_context(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_banner_pattern_holder_set_snbt(
        &patch.provides_banner_pattern_registry_ids,
        patch.provides_banner_pattern_tag.as_deref(),
        context,
    )
}

fn debug_banner_pattern_resource_id(
    holder_id: i32,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_resource_id("minecraft:banner_pattern", holder_id, |holder_id| {
        BannerPatternKind::for_vanilla_registry_id(holder_id)
            .map(BannerPatternKind::resource_id)
            .map(str::to_string)
    })
}

fn debug_banner_pattern_holder_set_snbt(
    registry_ids: &[i32],
    tag: Option<&str>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_holder_set_snbt("minecraft:banner_pattern", registry_ids, tag, |holder_id| {
        BannerPatternKind::for_vanilla_registry_id(holder_id)
            .map(BannerPatternKind::resource_id)
            .map(str::to_string)
    })
}

fn debug_item_stack_template_snbt(
    template: &bbb_protocol::packets::ItemStackTemplateSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if !(1..=99).contains(&template.count) {
        return None;
    }
    let resource_id = context.item_runtime?.item_resource_id(template.item_id)?;
    if resource_id == "minecraft:air" {
        return None;
    }
    let mut fields = vec![format!("id: {}", debug_snbt_string(resource_id))];
    if template.count != 1 {
        fields.push(format!("count: {}", template.count));
    }
    if let Some(components) =
        debug_supported_component_patch_snbt_with_context(&template.component_patch, context)
    {
        fields.push(format!("components: {components}"));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_writable_book_content_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> String {
    if patch.writable_book_pages.is_empty() {
        return "{}".to_string();
    }
    let pages = patch
        .writable_book_pages
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let mut fields = vec![format!("raw: {}", debug_snbt_string(raw))];
            if let Some(filtered) = patch
                .writable_book_page_filters
                .get(index)
                .and_then(Option::as_deref)
            {
                fields.push(format!("filtered: {}", debug_snbt_string(filtered)));
            }
            format!("{{{}}}", fields.join(", "))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{pages: [{pages}]}}")
}

fn debug_lodestone_tracker_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> String {
    let mut fields = Vec::new();
    if let Some(target) = &patch.lodestone_target {
        fields.push(format!(
            "target: {{dimension: {}, pos: [I; {}, {}, {}]}}",
            debug_snbt_string(&target.dimension),
            target.pos.x,
            target.pos.y,
            target.pos.z
        ));
    }
    if !patch.lodestone_tracked.unwrap_or(true) {
        fields.push(format!("tracked: {}", debug_snbt_bool(false)));
    }
    format!("{{{}}}", fields.join(", "))
}

fn debug_profile_component_snbt(profile: &ResolvableProfileSummary) -> Option<String> {
    let mut fields = Vec::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            fields.push(format!("id: {}", debug_snbt_uuid_int_array(profile.uuid?)));
            fields.push(format!(
                "name: {}",
                debug_snbt_string(profile.name.as_ref()?)
            ));
        }
        ResolvableProfileKindSummary::Partial => {
            if let Some(name) = &profile.name {
                fields.push(format!("name: {}", debug_snbt_string(name)));
            }
            if let Some(uuid) = profile.uuid {
                fields.push(format!("id: {}", debug_snbt_uuid_int_array(uuid)));
            }
        }
    }
    if !profile.properties.is_empty() {
        let properties = profile
            .properties
            .iter()
            .map(|property| {
                let mut fields = vec![
                    format!("name: {}", debug_snbt_string(&property.name)),
                    format!("value: {}", debug_snbt_string(&property.value)),
                ];
                if let Some(signature) = &property.signature {
                    fields.push(format!("signature: {}", debug_snbt_string(signature)));
                }
                format!("{{{}}}", fields.join(", "))
            })
            .collect::<Vec<_>>()
            .join(", ");
        fields.push(format!("properties: [{properties}]"));
    }
    if let Some(texture) = &profile.skin_patch.body {
        fields.push(format!("texture: {}", debug_snbt_string(&texture.asset_id)));
    }
    if let Some(cape) = &profile.skin_patch.cape {
        fields.push(format!("cape: {}", debug_snbt_string(&cape.asset_id)));
    }
    if let Some(elytra) = &profile.skin_patch.elytra {
        fields.push(format!("elytra: {}", debug_snbt_string(&elytra.asset_id)));
    }
    if let Some(model) = profile.skin_patch.model {
        fields.push(format!(
            "model: {}",
            debug_snbt_string(match model {
                PlayerModelTypeSummary::Slim => "slim",
                PlayerModelTypeSummary::Wide => "wide",
            })
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_pot_decorations_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let mut items = patch
        .pot_decorations_item_ids
        .iter()
        .map(|item_id| {
            item_runtime?
                .item_resource_id(*item_id)
                .map(debug_snbt_string)
        })
        .collect::<Option<Vec<_>>>()?;
    while items.len() < 4 {
        items.push(debug_snbt_string("minecraft:brick"));
    }
    Some(format!("[{}]", items.join(", ")))
}

fn debug_item_container_contents_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if patch.container_slots.is_empty() && !patch.container_items.is_empty() {
        return None;
    }
    let slots = patch
        .container_slots
        .iter()
        .map(|slot| {
            let item = debug_item_stack_template_snbt(&slot.item, context)?;
            Some(format!("{{slot: {}, item: {item}}}", slot.slot))
        })
        .collect::<Option<Vec<_>>>()?
        .join(", ");
    Some(format!("[{slots}]"))
}

fn debug_sound_event_component_snbt(
    sound_event: &bbb_protocol::packets::SoundEventSummary,
) -> Option<String> {
    if let Some(registry_id) = sound_event.registry_id {
        let resource_id = vanilla_sound_event_resource_id(registry_id)?;
        return Some(debug_snbt_string(resource_id));
    }

    let mut fields = vec![format!(
        "sound_id: {}",
        debug_snbt_string(sound_event.sound_id.as_deref()?)
    )];
    if let Some(bits) = sound_event.fixed_range_bits {
        fields.push(format!(
            "range: {}f",
            debug_nbt_java_float_string(f32::from_bits(bits))
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_piercing_weapon_component_snbt(
    weapon: &bbb_protocol::packets::PiercingWeaponSummary,
) -> Option<String> {
    let mut fields = Vec::new();
    if !weapon.deals_knockback {
        fields.push("deals_knockback: 0b".to_string());
    }
    if weapon.dismounts {
        fields.push("dismounts: 1b".to_string());
    }
    if let Some(sound) = weapon.sound.as_ref() {
        fields.push(format!(
            "sound: {}",
            debug_sound_event_component_snbt(sound)?
        ));
    }
    if let Some(sound) = weapon.hit_sound.as_ref() {
        fields.push(format!(
            "hit_sound: {}",
            debug_sound_event_component_snbt(sound)?
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_kinetic_weapon_component_snbt(
    weapon: &bbb_protocol::packets::KineticWeaponSummary,
) -> Option<String> {
    if weapon.contact_cooldown_ticks < 0 || weapon.delay_ticks < 0 {
        return None;
    }
    let mut fields = Vec::new();
    if weapon.contact_cooldown_ticks != 10 {
        fields.push(format!(
            "contact_cooldown_ticks: {}",
            weapon.contact_cooldown_ticks
        ));
    }
    if weapon.delay_ticks != 0 {
        fields.push(format!("delay_ticks: {}", weapon.delay_ticks));
    }
    for (name, condition) in [
        ("dismount_conditions", weapon.dismount_conditions),
        ("knockback_conditions", weapon.knockback_conditions),
        ("damage_conditions", weapon.damage_conditions),
    ] {
        if let Some(condition) = condition {
            fields.push(format!(
                "{name}: {}",
                debug_kinetic_weapon_condition_snbt(condition)?
            ));
        }
    }
    if weapon.forward_movement_bits != 0.0_f32.to_bits() {
        fields.push(format!(
            "forward_movement: {}f",
            debug_nbt_java_float_string(f32::from_bits(weapon.forward_movement_bits))
        ));
    }
    if weapon.damage_multiplier_bits != 1.0_f32.to_bits() {
        fields.push(format!(
            "damage_multiplier: {}f",
            debug_nbt_java_float_string(f32::from_bits(weapon.damage_multiplier_bits))
        ));
    }
    if let Some(sound) = weapon.sound.as_ref() {
        fields.push(format!(
            "sound: {}",
            debug_sound_event_component_snbt(sound)?
        ));
    }
    if let Some(sound) = weapon.hit_sound.as_ref() {
        fields.push(format!(
            "hit_sound: {}",
            debug_sound_event_component_snbt(sound)?
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_kinetic_weapon_condition_snbt(
    condition: bbb_protocol::packets::KineticWeaponConditionSummary,
) -> Option<String> {
    if condition.max_duration_ticks < 0 {
        return None;
    }
    let mut fields = vec![format!(
        "max_duration_ticks: {}",
        condition.max_duration_ticks
    )];
    if condition.min_speed_bits != 0.0_f32.to_bits() {
        fields.push(format!(
            "min_speed: {}f",
            debug_nbt_java_float_string(f32::from_bits(condition.min_speed_bits))
        ));
    }
    if condition.min_relative_speed_bits != 0.0_f32.to_bits() {
        fields.push(format!(
            "min_relative_speed: {}f",
            debug_nbt_java_float_string(f32::from_bits(condition.min_relative_speed_bits))
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn debug_fireworks_component_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> String {
    let mut fireworks = Vec::new();
    let flight_duration = patch.fireworks_flight_duration.unwrap_or(0);
    if flight_duration != 0 {
        fireworks.push(format!("flight_duration: {flight_duration}"));
    }
    if !patch.fireworks_explosions.is_empty() {
        let explosions = patch
            .fireworks_explosions
            .iter()
            .map(debug_firework_explosion_snbt)
            .collect::<Vec<_>>()
            .join(", ");
        fireworks.push(format!("explosions: [{explosions}]"));
    }
    format!("{{{}}}", fireworks.join(", "))
}

fn debug_firework_explosion_snbt(explosion: &FireworkExplosionSummary) -> String {
    let mut fields = vec![format!(
        "shape: {}",
        debug_snbt_string(debug_firework_explosion_shape_name(explosion.shape))
    )];
    if !explosion.colors.is_empty() {
        fields.push(format!(
            "colors: [{}]",
            explosion
                .colors
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !explosion.fade_colors.is_empty() {
        fields.push(format!(
            "fade_colors: [{}]",
            explosion
                .fade_colors
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if explosion.has_trail {
        fields.push(format!("has_trail: {}", debug_snbt_bool(true)));
    }
    if explosion.has_twinkle {
        fields.push(format!("has_twinkle: {}", debug_snbt_bool(true)));
    }
    format!("{{{}}}", fields.join(", "))
}

fn debug_firework_explosion_shape_name(shape: FireworkExplosionShapeSummary) -> &'static str {
    match shape {
        FireworkExplosionShapeSummary::SmallBall => "small_ball",
        FireworkExplosionShapeSummary::LargeBall => "large_ball",
        FireworkExplosionShapeSummary::Star => "star",
        FireworkExplosionShapeSummary::Creeper => "creeper",
        FireworkExplosionShapeSummary::Burst => "burst",
    }
}

fn debug_push_item_stack_id(fields: &mut Vec<String>, key: &str, item_id: &str) {
    fields.push(format!("{key}: {{id: {}}}", debug_snbt_string(item_id)));
}

fn debug_push_falling_block_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    if let Some(block_state) = world.falling_block_state(entity.id) {
        fields.push(format!(
            "BlockState: {}",
            debug_snbt_block_state(&block_state.block.name, &block_state.block.properties)
        ));
    }
    fields.push(format!(
        "Time: {}",
        entity.client_animations.age_ticks.min(i32::MAX as u32)
    ));
}

fn debug_push_minecart_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if debug_entity_data_optional_block_state_present(
        entity,
        accessors.custom_display_block_data_id,
    )
    .flatten()
    .is_some()
    {
        if let Some(display) = world.minecart_display_block_state(entity.id) {
            fields.push(format!(
                "DisplayState: {}",
                debug_snbt_block_state(&display.block.name, &display.block.properties)
            ));
        }
    }
    let display_offset = debug_entity_data_int_present(entity, accessors.display_offset_data_id)
        .unwrap_or(accessors.display_offset_default);
    if display_offset != accessors.display_offset_default {
        fields.push(format!("DisplayOffset: {display_offset}"));
    }
    fields.push(format!(
        "FlippedRotation: {}",
        debug_snbt_bool(MINECART_CLIENT_FLIPPED_ROTATION)
    ));
    // Despite its key name, vanilla writes the internal `Entity.firstTick`
    // field directly: true before the first minecart tick, false afterwards.
    fields.push(format!(
        "HasTicked: {}",
        debug_snbt_bool(entity.client_animations.age_ticks == 0)
    ));
}

fn debug_push_command_block_minecart_additional_save_data(
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    if let Some(command) =
        debug_entity_data_string_present(entity, COMMAND_BLOCK_MINECART_COMMAND_DATA_ID)
    {
        fields.push(format!("Command: {}", debug_snbt_string(command)));
    }
}

fn debug_push_furnace_minecart_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_hopper_minecart_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_spawner_minecart_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_tnt_minecart_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let fuse = entity
        .client_animations
        .minecart_tnt_fuse
        .map_or(TNT_MINECART_DEFAULT_FUSE, |fuse| fuse.fuse_remaining_ticks);
    fields.push(format!("fuse: {fuse}"));
    // `explosionPowerBase` and `explosionSpeedFactor` are server-private NBT
    // fields. Their client constructor defaults are exactly the values for
    // which vanilla omits both optional save keys.
}

fn debug_push_item_frame_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    let Some(entity_accessors) = vanilla_entity_metadata_entity_accessors() else {
        return;
    };
    let Some(accessors) =
        vanilla_entity_metadata_item_frame_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(block_pos) = debug_snbt_block_pos("block_pos", entity.position_base) {
        fields.push(block_pos);
    }
    if let Some((stack, resource_id)) =
        debug_entity_data_item_stack_present(entity, accessors.item_data_id).and_then(|stack| {
            let resource_id = stack
                .item_id
                .and_then(|item_id| item_runtime?.item_resource_id(item_id))?;
            Some((stack, resource_id))
        })
    {
        if let Some(item) = debug_item_stack_snbt_with_context(
            resource_id,
            stack.count,
            Some(stack),
            PersistentSnbtContext {
                item_runtime,
                registry_access: Some(world.frozen_registry_access()),
            },
        ) {
            fields.push(format!("Item: {item}"));
        }
    }
    let facing = debug_hanging_facing_legacy_id(entity, accessors.direction_data_id);
    if let Some(rotation) = debug_entity_data_int_present(entity, accessors.rotation_data_id) {
        fields.push(format!("ItemRotation: {}b", rotation.rem_euclid(8) as i8));
    }
    fields.push(format!("Facing: {facing}b"));
    if let Some(flags) =
        debug_entity_data_byte_present(entity, entity_accessors.shared_flags_data_id)
    {
        fields.push(format!(
            "Invisible: {}",
            debug_snbt_bool(flags & entity_accessors.invisible_mask != 0)
        ));
    }
}

fn debug_hanging_facing_legacy_id(entity: &EntityState, direction_data_id: u8) -> i32 {
    debug_entity_data_direction_legacy_id_present(entity, direction_data_id)
        .unwrap_or_else(|| debug_vanilla_direction_legacy_id(entity.data))
}

fn debug_push_painting_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessor) =
        vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(block_pos) = debug_snbt_block_pos("block_pos", entity.position_base) {
        fields.push(block_pos);
    }
    fields.push(format!(
        "facing: {}b",
        debug_painting_facing_legacy_2d_id(entity, accessor.direction_data_id)
    ));
    if let Some(resource_id) = debug_entity_data_painting_variant_named_reference(entity)
        .and_then(|reference| debug_registry_resource_id(reference, "minecraft:painting_variant"))
    {
        fields.push(format!("variant: {}", debug_snbt_string(resource_id)));
    }
}

fn debug_painting_facing_legacy_2d_id(entity: &EntityState, direction_data_id: u8) -> i32 {
    match debug_hanging_facing_legacy_id(entity, direction_data_id) {
        3 => 0, // south
        4 => 1, // west
        2 => 2, // north
        5 => 3, // east
        _ => 0,
    }
}

fn debug_entity_data_painting_variant_named_reference(
    entity: &EntityState,
) -> Option<&bbb_protocol::packets::EntityDataRegistryReference> {
    let data_id =
        vanilla_entity_metadata_painting_variant_accessor_for_entity_type(entity.entity_type_id)?;
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::PaintingVariant(variant) if variant.direct.is_none() => {
                variant.registry.as_ref()
            }
            _ => None,
        })
}

fn debug_vanilla_direction_legacy_id(data: i32) -> i32 {
    (data % 6).abs()
}

fn debug_push_primed_tnt_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessors) =
        vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let fuse = debug_entity_data_int_present(entity, accessors.fuse_data_id)
        .unwrap_or(accessors.fuse_default) as i16;
    fields.push(format!("fuse: {fuse}s"));
    if let Some(block_state) = world.primed_tnt_block_state(entity.id) {
        fields.push(format!(
            "block_state: {}",
            debug_snbt_block_state(&block_state.name, &block_state.properties)
        ));
    }
    // Vanilla writes `explosion_power` only when its server-private field differs
    // from 4.0. Add-entity and metadata never carry it, and client tick never
    // mutates it, so a remote client's save correctly omits the default field.
}

fn debug_push_wither_skull_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let dangerous = vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
        entity.entity_type_id,
    )
    .map(|(data_id, default)| debug_entity_data_bool_present(entity, data_id).unwrap_or(default))
    .unwrap_or_default();
    fields.push(format!("dangerous: {}", debug_snbt_bool(dangerous)));
}

fn debug_push_shulker_bullet_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_experience_orb_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let value = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!("Value: {value}s"));
}

fn debug_push_armor_stand_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(entity_accessors) = vanilla_entity_metadata_entity_accessors() else {
        return;
    };
    let Some(accessors) =
        vanilla_entity_metadata_armor_stand_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(shared_flags) =
        debug_entity_data_byte_present(entity, entity_accessors.shared_flags_data_id)
    {
        fields.push(format!(
            "Invisible: {}",
            debug_snbt_bool(shared_flags & entity_accessors.invisible_mask != 0)
        ));
    }
    if let Some(client_flags) =
        debug_entity_data_byte_present(entity, accessors.client_flags_data_id)
    {
        fields.push(format!(
            "Small: {}",
            debug_snbt_bool(client_flags & accessors.small_mask != 0)
        ));
        fields.push(format!(
            "ShowArms: {}",
            debug_snbt_bool(client_flags & accessors.show_arms_mask != 0)
        ));
        fields.push(format!(
            "NoBasePlate: {}",
            debug_snbt_bool(client_flags & accessors.no_baseplate_mask != 0)
        ));
        if client_flags & accessors.marker_mask != 0 {
            fields.push("Marker: 1b".to_string());
        }
    }
    if let Some(pose) = debug_armor_stand_pose_snbt(entity, accessors) {
        fields.push(format!("Pose: {pose}"));
    }
}

fn debug_armor_stand_pose_snbt(
    entity: &EntityState,
    accessors: VanillaArmorStandMetadataAccessors,
) -> Option<String> {
    let pose_fields = [
        (
            "Head",
            debug_entity_data_rotations_present(entity, accessors.head_pose_data_id)
                .unwrap_or(accessors.head_pose_default),
            accessors.head_pose_default,
        ),
        (
            "Body",
            debug_entity_data_rotations_present(entity, accessors.body_pose_data_id)
                .unwrap_or(accessors.body_pose_default),
            accessors.body_pose_default,
        ),
        (
            "LeftArm",
            debug_entity_data_rotations_present(entity, accessors.left_arm_pose_data_id)
                .unwrap_or(accessors.left_arm_pose_default),
            accessors.left_arm_pose_default,
        ),
        (
            "RightArm",
            debug_entity_data_rotations_present(entity, accessors.right_arm_pose_data_id)
                .unwrap_or(accessors.right_arm_pose_default),
            accessors.right_arm_pose_default,
        ),
        (
            "LeftLeg",
            debug_entity_data_rotations_present(entity, accessors.left_leg_pose_data_id)
                .unwrap_or(accessors.left_leg_pose_default),
            accessors.left_leg_pose_default,
        ),
        (
            "RightLeg",
            debug_entity_data_rotations_present(entity, accessors.right_leg_pose_data_id)
                .unwrap_or(accessors.right_leg_pose_default),
            accessors.right_leg_pose_default,
        ),
    ];
    let mut fields = Vec::new();
    for (name, value, default) in pose_fields {
        if value != default {
            fields.push(format!("{name}: {}", debug_snbt_rotations(value)?));
        }
    }
    (!fields.is_empty()).then(|| format!("{{{}}}", fields.join(", ")))
}

fn debug_push_mob_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    debug_push_leash_additional_save_data(world, entity, fields);
    let Some(accessors) =
        vanilla_entity_metadata_mob_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(flags) = debug_entity_data_byte_present(entity, accessors.flags_data_id) {
        fields.push(format!(
            "LeftHanded: {}",
            debug_snbt_bool(flags & accessors.left_handed_mask != 0)
        ));
        if flags & accessors.no_ai_mask != 0 {
            fields.push("NoAI: 1b".to_string());
        }
    }
}

fn debug_push_leash_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(holder_id) = entity.leash_holder_id else {
        return;
    };
    let Some(holder) = world.probe_entity(holder_id) else {
        return;
    };
    fields.push(format!(
        "leash: {{UUID: {}}}",
        debug_snbt_uuid_int_array(holder.uuid)
    ));
}

fn debug_push_ageable_mob_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    if let Some((baby_data_id, _default)) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(entity.entity_type_id)
    {
        if let Some(is_baby) = debug_entity_data_bool_present(entity, baby_data_id) {
            let age = if is_baby {
                AGEABLE_MOB_CLIENT_BABY_AGE
            } else {
                AGEABLE_MOB_CLIENT_ADULT_AGE
            };
            fields.push(format!("Age: {age}"));
        }
    }
    debug_push_age_locked_additional_save_data(entity, fields);
}

fn debug_push_age_locked_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, _default)) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(age_locked) = debug_entity_data_bool_present(entity, data_id) {
        fields.push(format!("AgeLocked: {}", debug_snbt_bool(age_locked)));
    }
}

fn debug_push_animal_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_allay_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_abstract_horse_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(flags) = debug_entity_data_byte_present(entity, accessors.flags_data_id) {
        fields.push(format!(
            "EatingHaystack: {}",
            debug_snbt_bool(flags & accessors.eating_mask != 0)
        ));
        fields.push(format!(
            "Bred: {}",
            debug_snbt_bool(flags & accessors.bred_mask != 0)
        ));
        fields.push(format!(
            "Tame: {}",
            debug_snbt_bool(flags & accessors.tame_mask != 0)
        ));
    }
}

fn debug_push_abstract_chested_horse_additional_save_data(
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some((chest_data_id, _)) =
        vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(has_chest) = debug_entity_data_bool_present(entity, chest_data_id) {
        fields.push(format!("ChestedHorse: {}", debug_snbt_bool(has_chest)));
    }
}

fn debug_push_horse_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((variant_data_id, variant_default)) =
        vanilla_entity_metadata_horse_variant_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, variant_data_id).unwrap_or(variant_default);
    fields.push(format!("Variant: {variant}"));
}

fn debug_push_skeleton_horse_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_camel_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((last_pose_change_tick_data_id, last_pose_change_tick_default)) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let last_pose_tick = debug_entity_data_long_present(entity, last_pose_change_tick_data_id)
        .unwrap_or(last_pose_change_tick_default);
    fields.push(format!("LastPoseTick: {last_pose_tick}L"));
}

fn debug_push_llama_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_llama_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, accessors.variant_data_id)
        .map(|variant| debug_llama_variant_save_id(variant, accessors.variant_default))
        .unwrap_or(accessors.variant_default);
    let strength = debug_entity_data_int_present(entity, accessors.strength_data_id)
        .unwrap_or(accessors.strength_default);
    fields.push(format!("Variant: {variant}"));
    fields.push(format!("Strength: {strength}"));
}

fn debug_llama_variant_save_id(variant: i32, default: i32) -> i32 {
    variant.clamp(default, LLAMA_MAX_VARIANT)
}

fn debug_push_tamable_animal_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let accessors =
        vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity.entity_type_id);
    let flags = accessors
        .map(|accessors| {
            debug_entity_data_byte_present(entity, accessors.flags_data_id)
                .unwrap_or(accessors.flags_default)
        })
        .unwrap_or(0);
    fields.push(format!(
        "Sitting: {}",
        debug_snbt_bool(accessors.is_some_and(|accessors| flags & accessors.sitting_mask != 0))
    ));
}

fn debug_registry_resource_id<'a>(
    reference: &'a bbb_protocol::packets::EntityDataRegistryReference,
    registry: &str,
) -> Option<&'a str> {
    reference
        .identity()
        .filter(|identity| identity.registry() == registry)
        .map(|identity| identity.resource_id())
}

fn debug_push_cat_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(variant_accessors) =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let Some(accessors) =
        vanilla_entity_metadata_cat_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        variant_accessors.variant_data_id,
        EntityDataRegistryHolder::CatVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:cat_variant"))
    .unwrap_or(variant_accessors.variant_default_resource_id);
    let sound_variant = debug_entity_data_registry_reference_present(
        entity,
        variant_accessors.sound_variant_data_id,
        EntityDataRegistryHolder::CatSoundVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:cat_sound_variant"))
    .unwrap_or(variant_accessors.sound_variant_default_resource_id);
    let collar_color = debug_entity_data_int_present(entity, accessors.collar_color_data_id)
        .map(debug_dye_color_legacy_save_id)
        .unwrap_or(accessors.collar_color_default);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
    fields.push(format!(
        "sound_variant: {}",
        debug_snbt_string(sound_variant)
    ));
    fields.push(format!("CollarColor: {collar_color}b"));
}

fn debug_push_fox_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_fox_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let flags = debug_entity_data_byte_present(entity, accessors.flags_data_id)
        .unwrap_or(accessors.flags_default);
    let trusted = accessors
        .trusted_data_ids
        .into_iter()
        .filter_map(|data_id| debug_entity_data_optional_living_reference_present(entity, data_id))
        .map(debug_snbt_uuid_int_array)
        .collect::<Vec<_>>()
        .join(", ");
    let variant = debug_entity_data_int_present(entity, accessors.type_data_id)
        .unwrap_or(accessors.type_default);
    fields.push(format!("Trusted: [{trusted}]"));
    fields.push(format!(
        "Sleeping: {}",
        debug_snbt_bool(flags & accessors.sleeping_mask != 0)
    ));
    fields.push(format!(
        "Type: {}",
        debug_snbt_string(FoxModelVariant::from_id(variant).serialized_name())
    ));
    fields.push(format!(
        "Sitting: {}",
        debug_snbt_bool(flags & accessors.sitting_mask != 0)
    ));
    fields.push(format!(
        "Crouching: {}",
        debug_snbt_bool(flags & accessors.crouching_mask != 0)
    ));
}

fn debug_push_wolf_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(variant_accessors) =
        vanilla_entity_metadata_companion_variant_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let Some(accessors) =
        vanilla_entity_metadata_wolf_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let collar_color = debug_entity_data_int_present(entity, accessors.collar_color_data_id)
        .map(debug_dye_color_legacy_save_id)
        .unwrap_or(accessors.collar_color_default);
    let variant = debug_entity_data_registry_reference_present(
        entity,
        variant_accessors.variant_data_id,
        EntityDataRegistryHolder::WolfVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:wolf_variant"))
    .unwrap_or(variant_accessors.variant_default_resource_id);
    let anger_end_time = debug_entity_data_long_present(entity, accessors.anger_end_time_data_id)
        .unwrap_or(accessors.anger_end_time_default);
    let sound_variant = debug_entity_data_registry_reference_present(
        entity,
        variant_accessors.sound_variant_data_id,
        EntityDataRegistryHolder::WolfSoundVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:wolf_sound_variant"))
    .unwrap_or(variant_accessors.sound_variant_default_resource_id);
    fields.push(format!("CollarColor: {collar_color}b"));
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
    fields.push(format!("anger_end_time: {anger_end_time}L"));
    fields.push(format!(
        "sound_variant: {}",
        debug_snbt_string(sound_variant)
    ));
}

fn debug_push_zombie_nautilus_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some((variant_data_id, variant_default_resource_id)) =
        vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        variant_data_id,
        EntityDataRegistryHolder::ZombieNautilusVariant,
    )
    .and_then(|reference| {
        debug_registry_resource_id(reference, "minecraft:zombie_nautilus_variant")
    })
    .unwrap_or(variant_default_resource_id);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
}

fn debug_dye_color_legacy_save_id(color: i32) -> i32 {
    match color {
        0..=15 => color,
        _ => 0,
    }
}

fn debug_push_armadillo_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let state_id = debug_entity_data_enum_id_present(
        entity,
        data_id,
        EntityDataEnumSerializer::ArmadilloState,
    )
    .unwrap_or(default);
    fields.push(format!(
        "state: {}",
        debug_snbt_string(debug_armadillo_state_name(state_id))
    ));
}

fn debug_armadillo_state_name(state_id: i32) -> &'static str {
    match state_id {
        ARMADILLO_STATE_ROLLING_ID => "rolling",
        ARMADILLO_STATE_SCARED_ID => "scared",
        ARMADILLO_STATE_UNROLLING_ID => "unrolling",
        _ => "idle",
    }
}

fn debug_push_copper_golem_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let weather_state_id = debug_entity_data_enum_id_present(
        entity,
        accessors.weather_state_data_id,
        EntityDataEnumSerializer::WeatheringCopperState,
    )
    .unwrap_or(accessors.weather_state_default);
    fields.push(format!(
        "weather_state: {}",
        debug_snbt_string(debug_copper_golem_weather_state_name(weather_state_id))
    ));
}

fn debug_copper_golem_weather_state_name(state_id: i32) -> &'static str {
    match state_id {
        COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => "exposed",
        COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => "weathered",
        COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID.. => "oxidized",
        _ => "unaffected",
    }
}

fn debug_push_turtle_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_turtle_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    // Vanilla client-side Shift+F3+I calls `Turtle.saveWithoutId`. `homePos` starts at
    // `BlockPos.ZERO` and is not synchronized to remote clients, so the client-owned save value
    // is zero; ordinary F3+I uses the server tag-query path for the authoritative value.
    fields.push("home_pos: [I; 0, 0, 0]".to_string());
    let has_egg = debug_entity_data_bool_present(entity, accessors.has_egg_data_id)
        .unwrap_or(accessors.has_egg_default);
    fields.push(format!("has_egg: {}", debug_snbt_bool(has_egg)));
}

fn debug_push_axolotl_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((variant_data_id, default_variant)) =
        vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, variant_data_id).unwrap_or(default_variant);
    let variant = AxolotlModelVariant::from_id(variant).vanilla_id();
    let Some((from_bucket_data_id, from_bucket_default)) =
        vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let from_bucket =
        debug_entity_data_bool_present(entity, from_bucket_data_id).unwrap_or(from_bucket_default);
    fields.push(format!("Variant: {variant}"));
    fields.push(format!("FromBucket: {}", debug_snbt_bool(from_bucket)));
}

fn debug_push_abstract_fish_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let from_bucket = debug_entity_data_bool_present(entity, data_id).unwrap_or(default);
    fields.push(format!("FromBucket: {}", debug_snbt_bool(from_bucket)));
}

fn debug_push_chicken_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessors) = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        entity.entity_type_id,
    ) else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.variant_data_id,
        EntityDataRegistryHolder::ChickenVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:chicken_variant"))
    .unwrap_or(accessors.variant_default_resource_id);
    let sound_variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.sound_variant_data_id,
        EntityDataRegistryHolder::ChickenSoundVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:chicken_sound_variant"))
    .unwrap_or(accessors.sound_variant_default_resource_id);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
    fields.push(format!(
        "sound_variant: {}",
        debug_snbt_string(sound_variant)
    ));
}

fn debug_push_cow_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessors) = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        entity.entity_type_id,
    ) else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.variant_data_id,
        EntityDataRegistryHolder::CowVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:cow_variant"))
    .unwrap_or(accessors.variant_default_resource_id);
    let sound_variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.sound_variant_data_id,
        EntityDataRegistryHolder::CowSoundVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:cow_sound_variant"))
    .unwrap_or(accessors.sound_variant_default_resource_id);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
    fields.push(format!(
        "sound_variant: {}",
        debug_snbt_string(sound_variant)
    ));
}

fn debug_push_mooshroom_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let variant = vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
        entity.entity_type_id,
    )
    .map(|(data_id, default)| debug_entity_data_int_present(entity, data_id).unwrap_or(default))
    .unwrap_or_default();
    fields.push(format!(
        "Type: {}",
        debug_snbt_string(MooshroomVariant::from_vanilla_id(variant).serialized_name())
    ));
}

fn debug_push_goat_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_goat_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let is_screaming = debug_entity_data_bool_present(entity, accessors.screaming_data_id)
        .unwrap_or(accessors.screaming_default);
    let has_left_horn = debug_entity_data_bool_present(entity, accessors.left_horn_data_id)
        .unwrap_or(accessors.left_horn_default);
    let has_right_horn = debug_entity_data_bool_present(entity, accessors.right_horn_data_id)
        .unwrap_or(accessors.right_horn_default);
    fields.push(format!(
        "IsScreamingGoat: {}",
        debug_snbt_bool(is_screaming)
    ));
    fields.push(format!("HasLeftHorn: {}", debug_snbt_bool(has_left_horn)));
    fields.push(format!("HasRightHorn: {}", debug_snbt_bool(has_right_horn)));
}

fn debug_push_dolphin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_dolphin_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let got_fish = debug_entity_data_bool_present(entity, accessors.got_fish_data_id)
        .unwrap_or(accessors.got_fish_default);
    let moistness = debug_entity_data_int_present(entity, accessors.moistness_data_id)
        .unwrap_or(accessors.moistness_default);
    fields.push(format!("GotFish: {}", debug_snbt_bool(got_fish)));
    fields.push(format!("Moistness: {moistness}"));
}

fn debug_push_frog_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some((variant_data_id, variant_default_resource_id)) =
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        variant_data_id,
        EntityDataRegistryHolder::FrogVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:frog_variant"))
    .unwrap_or(variant_default_resource_id);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
}

fn debug_push_panda_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_panda_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let main_gene = debug_entity_data_byte_present(entity, accessors.main_gene_data_id)
        .map(i32::from)
        .unwrap_or(i32::from(accessors.main_gene_default));
    let hidden_gene = debug_entity_data_byte_present(entity, accessors.hidden_gene_data_id)
        .map(i32::from)
        .unwrap_or(i32::from(accessors.hidden_gene_default));
    fields.push(format!(
        "MainGene: {}",
        debug_snbt_string(debug_panda_gene_name(main_gene))
    ));
    fields.push(format!(
        "HiddenGene: {}",
        debug_snbt_string(debug_panda_gene_name(hidden_gene))
    ));
}

fn debug_panda_gene_name(gene: i32) -> &'static str {
    match gene {
        1 => "lazy",
        2 => "worried",
        3 => "playful",
        4 => "brown",
        5 => "weak",
        6 => "aggressive",
        _ => "normal",
    }
}

fn debug_push_parrot_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!(
        "Variant: {}",
        ParrotModelVariant::from_id(variant).vanilla_id()
    ));
}

fn debug_push_creeper_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_creeper_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let powered = debug_entity_data_bool_present(entity, accessors.powered_data_id)
        .unwrap_or(accessors.powered_default);
    let ignited = debug_entity_data_bool_present(entity, accessors.ignited_data_id)
        .unwrap_or(accessors.ignited_default);
    fields.push(format!("powered: {}", debug_snbt_bool(powered)));
    fields.push(format!("ignited: {}", debug_snbt_bool(ignited)));
}

fn debug_push_creaking_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(data_id) =
        bbb_world::vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    if let Some(home_pos) = debug_entity_data_optional_block_pos_present(entity, data_id) {
        fields.push(format!(
            "home_pos: [I; {}, {}, {}]",
            home_pos.x, home_pos.y, home_pos.z
        ));
    }
}

fn debug_push_shulker_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_shulker_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let attach_face = debug_entity_data_direction_present(entity, accessors.attach_face_data_id)
        .unwrap_or(accessors.attach_face_default as i8);
    let peek = debug_entity_data_byte_present(entity, accessors.peek_data_id)
        .unwrap_or(accessors.peek_default);
    let color = debug_entity_data_byte_present(entity, accessors.color_data_id)
        .unwrap_or(accessors.color_default);
    fields.push(format!("AttachFace: {attach_face}b"));
    fields.push(format!("Peek: {peek}b"));
    fields.push(format!("Color: {color}b"));
}

fn debug_push_sheep_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default, sheared_mask, color_mask)) =
        vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let wool = debug_entity_data_byte_present(entity, data_id).unwrap_or(default) as u8;
    fields.push(format!(
        "Sheared: {}",
        debug_snbt_bool(wool & sheared_mask as u8 != 0)
    ));
    fields.push(format!("Color: {}b", wool & color_mask));
}

fn debug_push_slime_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let size = debug_entity_data_int_present(entity, data_id)
        .unwrap_or(default)
        .clamp(SLIME_MIN_SIZE, SLIME_MAX_SIZE);
    fields.push(format!("Size: {}", size - 1));
}

fn debug_push_snow_golem_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default, pumpkin_mask)) =
        vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let flags = debug_entity_data_byte_present(entity, data_id).unwrap_or(default);
    fields.push(format!(
        "Pumpkin: {}",
        debug_snbt_bool(flags & pumpkin_mask != 0)
    ));
}

fn debug_push_bat_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let flags = vanilla_entity_metadata_bat_flags_accessor_for_entity_type(entity.entity_type_id)
        .map(|(data_id, default, _resting_mask)| {
            debug_entity_data_byte_present(entity, data_id).unwrap_or(default)
        })
        .unwrap_or(0);
    fields.push(format!("BatFlags: {flags}b"));
}

fn debug_push_bee_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_bee_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let flags = debug_entity_data_byte_present(entity, accessors.flags_data_id)
        .unwrap_or(accessors.flags_default);
    let has_nectar = flags & accessors.has_nectar_mask != 0;
    let has_stung = flags & accessors.has_stung_mask != 0;
    fields.push(format!("HasNectar: {}", debug_snbt_bool(has_nectar)));
    fields.push(format!("HasStung: {}", debug_snbt_bool(has_stung)));
    if let Some(anger_end_time) =
        debug_entity_data_long_present(entity, accessors.anger_end_time_data_id)
    {
        debug_push_neutral_mob_additional_save_data_with_end_time(fields, anger_end_time);
    }
}

fn debug_push_bogged_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let sheared =
        vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(entity.entity_type_id)
            .map(|(data_id, default)| {
                debug_entity_data_bool_present(entity, data_id).unwrap_or(default)
            })
            .unwrap_or_default();
    fields.push(format!("sheared: {}", debug_snbt_bool(sheared)));
}

fn debug_push_endermite_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_end_crystal_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(beam_target) =
        debug_entity_data_optional_block_pos_present(entity, accessors.beam_target_data_id)
    {
        fields.push(format!(
            "beam_target: [I; {}, {}, {}]",
            beam_target.x, beam_target.y, beam_target.z
        ));
    }
    let show_bottom = debug_entity_data_bool_present(entity, accessors.show_bottom_data_id)
        .unwrap_or(accessors.show_bottom_default);
    fields.push(format!("ShowBottom: {}", debug_snbt_bool(show_bottom)));
}

fn debug_push_ghast_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_glow_squid_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let dark_ticks = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!("DarkTicksRemaining: {dark_ticks}"));
}

fn debug_push_happy_ghast_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_hoglin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        bbb_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let immune_to_zombification =
        debug_entity_data_bool_present(entity, data_id).unwrap_or(default);
    fields.push(format!(
        "IsImmuneToZombification: {}",
        debug_snbt_bool(immune_to_zombification)
    ));
}

fn debug_push_abstract_piglin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        bbb_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let immune_to_zombification =
        debug_entity_data_bool_present(entity, data_id).unwrap_or(default);
    fields.push(format!(
        "IsImmuneToZombification: {}",
        debug_snbt_bool(immune_to_zombification)
    ));
}

fn debug_push_piglin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, _default)) =
        bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    if let Some(is_baby) = debug_entity_data_bool_present(entity, data_id) {
        fields.push(format!("IsBaby: {}", debug_snbt_bool(is_baby)));
    }
}

fn debug_push_interaction_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_interaction_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let width = debug_entity_data_float_present(entity, accessors.width_data_id)
        .unwrap_or(accessors.width_default);
    let height = debug_entity_data_float_present(entity, accessors.height_data_id)
        .unwrap_or(accessors.height_default);
    let response = debug_entity_data_bool_present(entity, accessors.response_data_id)
        .unwrap_or(accessors.response_default);
    if let Some(width) = debug_snbt_float(width) {
        fields.push(format!("width: {width}"));
    }
    if let Some(height) = debug_snbt_float(height) {
        fields.push(format!("height: {height}"));
    }
    fields.push(format!("response: {}", debug_snbt_bool(response)));
}

fn debug_push_iron_golem_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let (flags, player_created_mask) =
        vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type(entity.entity_type_id)
            .map(|(data_id, default, player_created_mask)| {
                (
                    debug_entity_data_byte_present(entity, data_id).unwrap_or(default),
                    player_created_mask,
                )
            })
            .unwrap_or((0, 0));
    fields.push(format!(
        "PlayerCreated: {}",
        debug_snbt_bool(flags & player_created_mask != 0)
    ));
}

fn debug_push_neutral_mob_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_neutral_mob_additional_save_data_with_end_time(
    fields: &mut Vec<String>,
    anger_end_time: i64,
) {
    fields.push(format!("anger_end_time: {anger_end_time}L"));
}

fn debug_push_ocelot_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let trusting = debug_entity_data_bool_present(entity, data_id).unwrap_or(default);
    fields.push(format!("Trusting: {}", debug_snbt_bool(trusting)));
}

fn debug_push_patrolling_monster_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_inventory_carrier_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_abstract_villager_additional_save_data(fields: &mut Vec<String>) {
    debug_push_inventory_carrier_additional_save_data(fields);
}

fn debug_push_phantom_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_phantom_size_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let size = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!("size: {size}"));
}

fn debug_push_pig_additional_save_data(
    _world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some(accessors) = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        entity.entity_type_id,
    ) else {
        return;
    };
    let variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.variant_data_id,
        EntityDataRegistryHolder::PigVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:pig_variant"))
    .unwrap_or(accessors.variant_default_resource_id);
    let sound_variant = debug_entity_data_registry_reference_present(
        entity,
        accessors.sound_variant_data_id,
        EntityDataRegistryHolder::PigSoundVariant,
    )
    .and_then(|reference| debug_registry_resource_id(reference, "minecraft:pig_sound_variant"))
    .unwrap_or(accessors.sound_variant_default_resource_id);
    fields.push(format!("variant: {}", debug_snbt_string(variant)));
    fields.push(format!(
        "sound_variant: {}",
        debug_snbt_string(sound_variant)
    ));
}

fn debug_push_pufferfish_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let puff_state = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!("PuffState: {puff_state}"));
}

fn debug_push_rabbit_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let rabbit_type = debug_entity_data_int_present(entity, data_id)
        .map(|rabbit_type| debug_rabbit_type_save_id(rabbit_type, default))
        .unwrap_or(default);
    fields.push(format!("RabbitType: {rabbit_type}"));
}

fn debug_rabbit_type_save_id(rabbit_type: i32, default: i32) -> i32 {
    match rabbit_type {
        0..=5 | 99 => rabbit_type,
        _ => default,
    }
}

fn debug_push_salmon_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, data_id).unwrap_or(default);
    fields.push(format!(
        "type: {}",
        debug_snbt_string(debug_salmon_variant_name(variant))
    ));
}

fn debug_salmon_variant_name(variant: i32) -> &'static str {
    match variant {
        i32::MIN..=0 => "small",
        1 => "medium",
        _ => "large",
    }
}

fn debug_push_skeleton_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_tadpole_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    debug_push_age_locked_additional_save_data(entity, fields);
}

fn debug_push_tropical_fish_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) = vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
        entity.entity_type_id,
    ) else {
        return;
    };
    let variant = debug_entity_data_int_present(entity, accessors.variant_data_id)
        .unwrap_or(accessors.variant_default);
    fields.push(format!("Variant: {variant}"));
}

fn debug_push_raider_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_spellcaster_illager_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_ravager_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let ravager = entity.client_animations.ravager;
    fields.push(format!(
        "AttackTick: {}",
        ravager.map_or(RAVAGER_DEFAULT_ATTACK_TICK, |state| state.attack_tick)
    ));
    fields.push(format!(
        "StunTick: {}",
        ravager.map_or(RAVAGER_DEFAULT_STUN_TICK, |state| state.stunned_tick)
    ));
    fields.push(format!(
        "RoarTick: {}",
        ravager.map_or(RAVAGER_DEFAULT_ROAR_TICK, |state| state.roar_tick)
    ));
}

fn debug_push_wither_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((invulnerable_ticks_data_id, invulnerable_ticks_default)) =
        vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let invulnerable_ticks = debug_entity_data_int_present(entity, invulnerable_ticks_data_id)
        .unwrap_or(invulnerable_ticks_default);
    fields.push(format!("Invul: {invulnerable_ticks}"));
}

fn debug_push_wandering_trader_additional_save_data(_fields: &mut Vec<String>) {}

fn debug_push_zombie_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(accessors) =
        vanilla_entity_metadata_zombie_accessors_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let is_baby = debug_entity_data_bool_present(entity, accessors.baby_data_id)
        .unwrap_or(accessors.baby_default);
    fields.push(format!("IsBaby: {}", debug_snbt_bool(is_baby)));
}

fn debug_entity_data_villager_data_present(
    entity: &EntityState,
    data_id: u8,
) -> Option<(&str, &str, i32)> {
    entity
        .data_values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::VillagerData {
                villager_type,
                profession,
                level,
            } => Some((
                debug_registry_resource_id(villager_type, "minecraft:villager_type")?,
                debug_registry_resource_id(profession, "minecraft:villager_profession")?,
                *level,
            )),
            _ => None,
        })
}

fn debug_push_villager_data_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some(data_id) = bbb_world::vanilla_entity_metadata_villager_data_accessor_for_entity_type(
        entity.entity_type_id,
    ) else {
        return;
    };
    if let Some((villager_type, profession, level)) =
        debug_entity_data_villager_data_present(entity, data_id)
    {
        fields.push(format!(
            "VillagerData: {{type: {}, profession: {}, level: {level}}}",
            debug_snbt_string(villager_type),
            debug_snbt_string(profession),
        ));
    }
    if let Some((finalized_id, default)) =
        bbb_world::vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type(
            entity.entity_type_id,
        )
    {
        let finalized = debug_entity_data_bool_present(entity, finalized_id).unwrap_or(default);
        fields.push(format!(
            "VillagerDataFinalized: {}",
            debug_snbt_bool(finalized)
        ));
    }
}

fn debug_push_zombie_villager_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    debug_push_villager_data_save_data(entity, fields);
    let Some((converting_id, default)) =
        bbb_world::vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let converting = debug_entity_data_bool_present(entity, converting_id).unwrap_or(default);
    // The conversion timer value is server-private; the client entity holds
    // zero while the synced converting flag is set, so the vanilla
    // `isConverting() ? villagerConversionTime : -1` projection collapses to
    // these two observable values.
    fields.push(format!(
        "ConversionTime: {}",
        if converting { 0 } else { -1 }
    ));
}

fn debug_push_ender_dragon_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((phase_id, default)) =
        bbb_world::vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    let phase = debug_entity_data_int_present(entity, phase_id).unwrap_or(default);
    fields.push(format!("DragonPhase: {phase}"));
    // The death animation timer only advances after the server-authoritative
    // kill; a probed live dragon holds the client entity's zero.
    fields.push("DragonDeathTime: 0".to_string());
}

fn debug_push_area_effect_cloud_additional_save_data(
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    let Some((radius_id, default)) =
        bbb_world::vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    // Age mirrors the client-advanced `Entity.tickCount`; Duration, wait and
    // reapplication state, potion contents and the owner reference are
    // server-private and stay absent.
    fields.push(format!("Age: {}", entity.client_animations.age_ticks));
    if let Some(radius) =
        debug_snbt_float(debug_entity_data_float_present(entity, radius_id).unwrap_or(default))
    {
        fields.push(format!("Radius: {radius}"));
    }
}

fn debug_push_item_stack_field(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    stack_data_id: u8,
    field_name: &str,
    fields: &mut Vec<String>,
) {
    let Some((stack, resource_id)) = debug_entity_data_item_stack_present(entity, stack_data_id)
        .and_then(|stack| {
            let resource_id = stack
                .item_id
                .and_then(|item_id| item_runtime?.item_resource_id(item_id))?;
            Some((stack, resource_id))
        })
    else {
        return;
    };
    if let Some(item) = debug_item_stack_snbt_with_context(
        resource_id,
        stack.count,
        Some(stack),
        PersistentSnbtContext {
            item_runtime,
            registry_access: Some(world.frozen_registry_access()),
        },
    ) {
        fields.push(format!("{field_name}: {item}"));
    }
}

fn debug_push_eye_of_ender_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    let Some(stack_id) =
        bbb_world::vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    debug_push_item_stack_field(world, entity, item_runtime, stack_id, "Item", fields);
}

fn debug_push_item_entity_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    let Some(stack_id) =
        bbb_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    // Age mirrors the client-advanced `ItemEntity.age`; Health, PickupDelay
    // and the thrower/owner references are server-private and stay absent.
    fields.push(format!(
        "Age: {}s",
        entity.client_animations.age_ticks as i16
    ));
    debug_push_item_stack_field(world, entity, item_runtime, stack_id, "Item", fields);
}

fn debug_push_ominous_item_spawner_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    let Some(stack_id) =
        bbb_world::vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
            entity.entity_type_id,
        )
    else {
        return;
    };
    // Vanilla only stores a non-empty spawner stack, and the spawn delay is
    // server-private.
    debug_push_item_stack_field(world, entity, item_runtime, stack_id, "item", fields);
}

fn debug_display_int_field(entity: &EntityState, java_name: &str) -> Option<i32> {
    let entry = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        java_name,
    )?;
    let default = match entry.typed_default {
        Some(bbb_world::VanillaEntityMetadataDefault::Int(value)) => value,
        _ => 0,
    };
    Some(debug_entity_data_int_present(entity, entry.data_id).unwrap_or(default))
}

fn debug_display_float_field(entity: &EntityState, java_name: &str) -> Option<f32> {
    let entry = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        java_name,
    )?;
    let default = match entry.typed_default {
        Some(bbb_world::VanillaEntityMetadataDefault::Float(value)) => value,
        _ => 0.0,
    };
    Some(debug_entity_data_float_present(entity, entry.data_id).unwrap_or(default))
}

fn debug_display_byte_field(entity: &EntityState, java_name: &str) -> Option<i8> {
    let entry = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        java_name,
    )?;
    let default = match entry.typed_default {
        Some(bbb_world::VanillaEntityMetadataDefault::Byte(value)) => value,
        _ => 0,
    };
    Some(debug_entity_data_byte_present(entity, entry.data_id).unwrap_or(default))
}

fn debug_entity_data_vector3f_present(entity: &EntityState, data_id: u8) -> Option<[f32; 3]> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Vector3f { x, y, z } => Some([*x, *y, *z]),
            _ => None,
        })
}

fn debug_entity_data_quaternionf_present(entity: &EntityState, data_id: u8) -> Option<[f32; 4]> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Quaternionf { x, y, z, w } => Some([*x, *y, *z, *w]),
            _ => None,
        })
}

fn debug_snbt_float_list(values: &[f32]) -> Option<String> {
    let rendered: Option<Vec<String>> = values
        .iter()
        .map(|value| debug_snbt_float(*value))
        .collect();
    Some(format!("[{}]", rendered?.join(", ")))
}

fn debug_push_display_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let vector = |java_name: &str, default: [f32; 3]| -> Option<[f32; 3]> {
        let entry = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
            entity.entity_type_id,
            java_name,
        )?;
        Some(debug_entity_data_vector3f_present(entity, entry.data_id).unwrap_or(default))
    };
    let quaternion = |java_name: &str| -> Option<[f32; 4]> {
        let entry = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
            entity.entity_type_id,
            java_name,
        )?;
        // Vanilla defines both rotations as the identity quaternion.
        Some(
            debug_entity_data_quaternionf_present(entity, entry.data_id)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
        )
    };
    if let (Some(translation), Some(left_rotation), Some(scale), Some(right_rotation)) = (
        vector("DATA_TRANSLATION_ID", [0.0, 0.0, 0.0]),
        quaternion("DATA_LEFT_ROTATION_ID"),
        vector("DATA_SCALE_ID", [1.0, 1.0, 1.0]),
        quaternion("DATA_RIGHT_ROTATION_ID"),
    ) {
        if let (Some(translation), Some(left_rotation), Some(scale), Some(right_rotation)) = (
            debug_snbt_float_list(&translation),
            debug_snbt_float_list(&left_rotation),
            debug_snbt_float_list(&scale),
            debug_snbt_float_list(&right_rotation),
        ) {
            fields.push(format!(
                "transformation: {{translation: {translation}, left_rotation: {left_rotation}, \
                 scale: {scale}, right_rotation: {right_rotation}}}"
            ));
        }
    }
    if let Some(billboard) =
        debug_display_byte_field(entity, "DATA_BILLBOARD_RENDER_CONSTRAINTS_ID")
    {
        let name = match billboard {
            1 => "vertical",
            2 => "horizontal",
            3 => "center",
            _ => "fixed",
        };
        fields.push(format!("billboard: {}", debug_snbt_string(name)));
    }
    if let Some(duration) =
        debug_display_int_field(entity, "DATA_TRANSFORMATION_INTERPOLATION_DURATION_ID")
    {
        fields.push(format!("interpolation_duration: {duration}"));
    }
    if let Some(duration) =
        debug_display_int_field(entity, "DATA_POS_ROT_INTERPOLATION_DURATION_ID")
    {
        fields.push(format!("teleport_duration: {duration}"));
    }
    for (field_name, java_name) in [
        ("view_range", "DATA_VIEW_RANGE_ID"),
        ("shadow_radius", "DATA_SHADOW_RADIUS_ID"),
        ("shadow_strength", "DATA_SHADOW_STRENGTH_ID"),
        ("width", "DATA_WIDTH_ID"),
        ("height", "DATA_HEIGHT_ID"),
    ] {
        if let Some(rendered) =
            debug_display_float_field(entity, java_name).and_then(debug_snbt_float)
        {
            fields.push(format!("{field_name}: {rendered}"));
        }
    }
    if let Some(glow_color_override) =
        debug_display_int_field(entity, "DATA_GLOW_COLOR_OVERRIDE_ID")
    {
        fields.push(format!("glow_color_override: {glow_color_override}"));
    }
    // Vanilla stores the nullable brightness override only when set; the
    // packed value mirrors LightCoordsUtil (block << 4 | sky << 20).
    if let Some(packed) = debug_display_int_field(entity, "DATA_BRIGHTNESS_OVERRIDE_ID") {
        if packed >= 0 {
            fields.push(format!(
                "brightness: {{block: {}, sky: {}}}",
                (packed >> 4) & 0xF,
                (packed >> 20) & 0xF
            ));
        }
    }
}

fn debug_push_block_display_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    fields: &mut Vec<String>,
) {
    debug_push_display_additional_save_data(entity, fields);
    let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_BLOCK_STATE_ID",
    ) else {
        return;
    };
    let state_id = entity
        .data_values
        .iter()
        .find(|value| value.data_id == entry.data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::BlockState(id) => Some(*id),
            _ => None,
        })
        // Vanilla defines the synced state as the air default.
        .unwrap_or(0);
    let Some(state) = world.registries().block_state(state_id) else {
        return;
    };
    let mut block_state = format!("{{Name: {}", debug_snbt_string(&state.name));
    if !state.properties.is_empty() {
        let properties: Vec<String> = state
            .properties
            .iter()
            .map(|(key, value)| format!("{key}: {}", debug_snbt_string(value)))
            .collect();
        block_state.push_str(&format!(", Properties: {{{}}}", properties.join(", ")));
    }
    block_state.push('}');
    fields.push(format!("block_state: {block_state}"));
}

fn debug_push_item_display_additional_save_data(
    world: &WorldStore,
    entity: &EntityState,
    item_runtime: Option<&NativeItemRuntime>,
    fields: &mut Vec<String>,
) {
    debug_push_display_additional_save_data(entity, fields);
    if let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_ITEM_STACK_ID",
    ) {
        // Vanilla only stores a non-empty display stack.
        debug_push_item_stack_field(world, entity, item_runtime, entry.data_id, "item", fields);
    }
    if let Some(context) = debug_display_byte_field(entity, "DATA_ITEM_DISPLAY_ID") {
        let name = match context {
            1 => "thirdperson_lefthand",
            2 => "thirdperson_righthand",
            3 => "firstperson_lefthand",
            4 => "firstperson_righthand",
            5 => "head",
            6 => "gui",
            7 => "ground",
            8 => "fixed",
            9 => "on_shelf",
            _ => "none",
        };
        fields.push(format!("item_display: {}", debug_snbt_string(name)));
    }
}

fn debug_push_text_display_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    debug_push_display_additional_save_data(entity, fields);
    let text = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_TEXT_ID",
    )
    .and_then(|entry| {
        entity
            .data_values
            .iter()
            .find(|value| value.data_id == entry.data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Component(component) => Some(component.text()),
                _ => None,
            })
    })
    .unwrap_or("");
    fields.push(format!("text: {}", debug_snbt_string(text)));
    if let Some(line_width) = debug_display_int_field(entity, "DATA_LINE_WIDTH_ID") {
        fields.push(format!("line_width: {line_width}"));
    }
    if let Some(background) = debug_display_int_field(entity, "DATA_BACKGROUND_COLOR_ID") {
        fields.push(format!("background: {background}"));
    }
    if let Some(text_opacity) = debug_display_byte_field(entity, "DATA_TEXT_OPACITY_ID") {
        fields.push(format!("text_opacity: {text_opacity}b"));
    }
    if let Some(flags) = debug_display_byte_field(entity, "DATA_STYLE_FLAGS_ID") {
        fields.push(format!("shadow: {}", debug_snbt_bool(flags & 1 != 0)));
        fields.push(format!("see_through: {}", debug_snbt_bool(flags & 2 != 0)));
        fields.push(format!(
            "default_background: {}",
            debug_snbt_bool(flags & 4 != 0)
        ));
        let alignment = if flags & 8 != 0 {
            "left"
        } else if flags & 16 != 0 {
            "right"
        } else {
            "center"
        };
        fields.push(format!("alignment: {}", debug_snbt_string(alignment)));
    }
}

fn debug_snbt_resolvable_profile(profile: Option<&ResolvableProfileData>) -> String {
    // The network summary keeps only a property count, so profile properties
    // are never projected; vanilla writes name before id.
    let (name, id) = match profile.map(|profile| &profile.profile) {
        Some(ResolvableProfileKind::GameProfile { id, name, .. }) => {
            (Some(name.as_str()), Some(*id))
        }
        Some(ResolvableProfileKind::Partial { name, id, .. }) => (name.as_deref(), *id),
        None => (None, None),
    };
    let mut entries = Vec::new();
    if let Some(name) = name {
        entries.push(format!("name: {}", debug_snbt_string(name)));
    }
    if let Some(id) = id {
        entries.push(format!("id: {}", debug_snbt_uuid_int_array(id)));
    }
    format!("{{{}}}", entries.join(", "))
}

fn debug_mannequin_description_is_default(component: &bbb_protocol::RichTextComponent) -> bool {
    match component.component() {
        NbtSummaryValue::Compound(entries) => entries.iter().any(|entry| {
            entry.name == "translate"
                && entry.value == NbtSummaryValue::String("entity.minecraft.mannequin.label".into())
        }),
        _ => false,
    }
}

fn debug_push_mannequin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let profile = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_PROFILE",
    )
    .and_then(|entry| {
        entity
            .data_values
            .iter()
            .find(|value| value.data_id == entry.data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::ResolvableProfile(profile) => Some(profile),
                _ => None,
            })
    });
    fields.push(format!(
        "profile: {}",
        debug_snbt_resolvable_profile(profile)
    ));
    if let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_PLAYER_MODE_CUSTOMISATION",
    ) {
        // Mannequin's constructor raises the Avatar default to all layers
        // shown, which vanilla encodes as the inverted hidden-part list.
        let mask = debug_entity_data_byte_present(entity, entry.data_id).unwrap_or(0x7F);
        let hidden: Vec<&str> = [
            (0x01, "cape"),
            (0x02, "jacket"),
            (0x04, "left_sleeve"),
            (0x08, "right_sleeve"),
            (0x10, "left_pants_leg"),
            (0x20, "right_pants_leg"),
            (0x40, "hat"),
        ]
        .into_iter()
        .filter(|(part_mask, _)| mask & part_mask == 0)
        .map(|(_, name)| name)
        .collect();
        let rendered: Vec<String> = hidden.iter().map(|name| debug_snbt_string(name)).collect();
        fields.push(format!("hidden_layers: [{}]", rendered.join(", ")));
    }
    if let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_PLAYER_MAIN_HAND",
    ) {
        let arm = entity
            .data_values
            .iter()
            .find(|value| value.data_id == entry.data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::HumanoidArm(id) => Some(*id),
                _ => None,
            })
            // Vanilla DEFAULT_MAIN_HAND is HumanoidArm.RIGHT (id 1).
            .unwrap_or(1);
        let name = if arm == 0 { "left" } else { "right" };
        fields.push(format!("main_hand: {}", debug_snbt_string(name)));
    }
    if let Some(entity_accessors) = vanilla_entity_metadata_entity_accessors() {
        let pose = entity
            .data_values
            .iter()
            .find(|value| value.data_id == entity_accessors.pose_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Pose(id) => Some(*id),
                _ => None,
            })
            .unwrap_or(entity_accessors.pose_default);
        // Mannequin's POSE_CODEC only admits these five poses.
        let name = match pose {
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID => "fall_flying",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID => "sleeping",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID => "swimming",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID => "crouching",
            _ => "standing",
        };
        fields.push(format!("pose: {}", debug_snbt_string(name)));
    }
    if let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_IMMOVABLE",
    ) {
        let immovable = debug_entity_data_bool_present(entity, entry.data_id).unwrap_or(false);
        fields.push(format!("immovable: {}", debug_snbt_bool(immovable)));
    }
    if let Some(entry) = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        entity.entity_type_id,
        "DATA_DESCRIPTION",
    ) {
        // Synced empty means hide_description; the default translatable label
        // is omitted like vanilla's DEFAULT_DESCRIPTION comparison.
        match entity
            .data_values
            .iter()
            .find(|value| value.data_id == entry.data_id)
            .map(|value| &value.value)
        {
            Some(EntityDataValueKind::OptionalComponent(Some(component))) => {
                if !debug_mannequin_description_is_default(component) {
                    fields.push(format!(
                        "description: {}",
                        debug_snbt_string(component.text())
                    ));
                }
            }
            Some(EntityDataValueKind::OptionalComponent(None)) => {
                fields.push("hide_description: 1b".to_string());
            }
            _ => {}
        }
    }
}

fn debug_push_zoglin_additional_save_data(entity: &EntityState, fields: &mut Vec<String>) {
    let Some((data_id, default)) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(entity.entity_type_id)
    else {
        return;
    };
    let is_baby = debug_entity_data_bool_present(entity, data_id).unwrap_or(default);
    fields.push(format!("IsBaby: {}", debug_snbt_bool(is_baby)));
}

fn debug_entity_data_byte_present(entity: &EntityState, data_id: u8) -> Option<i8> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_int_present(entity: &EntityState, data_id: u8) -> Option<i32> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_long_present(entity: &EntityState, data_id: u8) -> Option<i64> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Long(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_bool_present(entity: &EntityState, data_id: u8) -> Option<bool> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_item_stack_present(
    entity: &EntityState,
    data_id: u8,
) -> Option<&ItemStackSummary> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::ItemStack(stack) => Some(stack),
            _ => None,
        })
}

fn debug_entity_data_string_present(entity: &EntityState, data_id: u8) -> Option<&str> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::String(value) => Some(value.as_str()),
            _ => None,
        })
}

fn debug_entity_data_direction_legacy_id_present(entity: &EntityState, data_id: u8) -> Option<i32> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Direction(value) => {
                Some(debug_vanilla_direction_legacy_id(*value))
            }
            _ => None,
        })
}

fn debug_entity_data_registry_reference_present(
    entity: &EntityState,
    data_id: u8,
    expected_serializer: EntityDataRegistryHolder,
) -> Option<&bbb_protocol::packets::EntityDataRegistryReference> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::RegistryId { serializer, value }
                if *serializer == expected_serializer =>
            {
                Some(value)
            }
            _ => None,
        })
}

fn debug_entity_data_enum_id_present(
    entity: &EntityState,
    data_id: u8,
    expected_serializer: EntityDataEnumSerializer,
) -> Option<i32> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId { serializer, id }
                if *serializer == expected_serializer =>
            {
                Some(*id)
            }
            _ => None,
        })
}

fn debug_entity_data_float_present(entity: &EntityState, data_id: u8) -> Option<f32> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_rotations_present(entity: &EntityState, data_id: u8) -> Option<[f32; 3]> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Rotations { x, y, z } => Some([*x, *y, *z]),
            _ => None,
        })
}

fn debug_entity_data_direction_present(entity: &EntityState, data_id: u8) -> Option<i8> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Direction(value) => Some(value.rem_euclid(6) as i8),
            _ => None,
        })
}

fn debug_entity_data_optional_block_pos_present(
    entity: &EntityState,
    data_id: u8,
) -> Option<ProtocolBlockPos> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(*pos),
            _ => None,
        })
}

fn debug_entity_data_optional_block_state_present(
    entity: &EntityState,
    data_id: u8,
) -> Option<Option<i32>> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalBlockState(value) => Some(*value),
            _ => None,
        })
}

fn debug_entity_data_optional_component_present(entity: &EntityState, data_id: u8) -> Option<&str> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.text()),
            _ => None,
        })
}

fn debug_entity_data_optional_living_reference_present(
    entity: &EntityState,
    data_id: u8,
) -> Option<uuid::Uuid> {
    entity
        .data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalLivingEntityReference(Some(value)) => Some(*value),
            _ => None,
        })
}

fn debug_snbt_vec3d(name: &str, value: EntityVec3) -> Option<String> {
    Some(format!(
        "{}: [{}, {}, {}]",
        name,
        debug_snbt_double(value.x)?,
        debug_snbt_double(value.y)?,
        debug_snbt_double(value.z)?
    ))
}

fn debug_snbt_rotation(y_rot: f32, x_rot: f32) -> Option<String> {
    Some(format!(
        "Rotation: [{}, {}]",
        debug_snbt_float(y_rot)?,
        debug_snbt_float(x_rot)?
    ))
}

fn debug_snbt_rotations(value: [f32; 3]) -> Option<String> {
    Some(format!(
        "[{}, {}, {}]",
        debug_snbt_float(value[0])?,
        debug_snbt_float(value[1])?,
        debug_snbt_float(value[2])?
    ))
}

fn debug_snbt_block_pos(name: &str, position: EntityVec3) -> Option<String> {
    Some(format!(
        "{name}: [I; {}, {}, {}]",
        debug_block_coord_i32(position.x)?,
        debug_block_coord_i32(position.y)?,
        debug_block_coord_i32(position.z)?
    ))
}

fn debug_block_coord_i32(value: f64) -> Option<i32> {
    if !value.is_finite() {
        return None;
    }
    let block = value.floor();
    if block < f64::from(i32::MIN) || block > f64::from(i32::MAX) {
        return None;
    }
    Some(block as i32)
}

fn debug_snbt_double(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Some(debug_snbt_number_text(value.to_string(), 'd'))
}

fn debug_snbt_float(value: f32) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Some(debug_snbt_number_text(value.to_string(), 'f'))
}

fn debug_snbt_string(value: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut body = String::new();
    for c in value.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '"' | '\'' => {
                if quote.is_none() {
                    quote = Some(if c == '"' { '\'' } else { '"' });
                }
                if quote == Some(c) {
                    body.push('\\');
                }
                body.push(c);
            }
            '\u{0008}' => body.push_str("\\b"),
            '\t' => body.push_str("\\t"),
            '\n' => body.push_str("\\n"),
            '\u{000c}' => body.push_str("\\f"),
            '\r' => body.push_str("\\r"),
            c if c < ' ' => body.push_str(&format!("\\x{:02x}", c as u8)),
            _ => body.push(c),
        }
    }
    let quote = quote.unwrap_or('"');
    out.push(quote);
    out.push_str(&body);
    out.push(quote);
    out
}

fn debug_snbt_compound_key(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
    {
        value.to_string()
    } else {
        debug_snbt_string(value)
    }
}

fn debug_snbt_block_state(name: &str, properties: &BTreeMap<String, String>) -> String {
    let mut fields = vec![format!("Name: {}", debug_snbt_string(name))];
    if !properties.is_empty() {
        let properties = properties
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}: {}",
                    debug_snbt_compound_key(key),
                    debug_snbt_string(value)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        fields.push(format!("Properties: {{{properties}}}"));
    }
    format!("{{{}}}", fields.join(", "))
}

fn debug_snbt_uuid_int_array(uuid: uuid::Uuid) -> String {
    let (most, least) = uuid.as_u64_pair();
    let ints = [
        (most >> 32) as i32,
        most as i32,
        (least >> 32) as i32,
        least as i32,
    ];
    format!("[I; {}, {}, {}, {}]", ints[0], ints[1], ints[2], ints[3])
}

fn debug_snbt_number_text(mut text: String, suffix: char) -> String {
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text.push(suffix);
    text
}

fn debug_snbt_bool(value: bool) -> &'static str {
    if value {
        "1b"
    } else {
        "0b"
    }
}

fn debug_copy_recreate_server_entity_command(
    entity_type: &str,
    position: [f64; 3],
    response: &TagQueryResponseState,
) -> bbb_world::Result<Option<DebugRecreateCopy>> {
    let command = if let Some(snbt) = response.pretty_snbt_without_root_keys(&["UUID", "Pos"])? {
        format!(
            "/summon {} {:.2} {:.2} {:.2} {}",
            entity_type, position[0], position[1], position[2], snbt
        )
    } else {
        format!(
            "/summon {} {:.2} {:.2} {:.2}",
            entity_type, position[0], position[1], position[2]
        )
    };
    Ok(Some(DebugRecreateCopy {
        command,
        feedback_message: "Copied server-side entity data to clipboard",
    }))
}

fn debug_block_state_description(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> String {
    if properties.is_empty() {
        return block_name.to_string();
    }
    let properties = properties
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{block_name}[{properties}]")
}

impl ClientInputState {
    pub(crate) fn loom_selected_pattern_index(&self) -> Option<i32> {
        self.loom_selected_pattern_index
    }

    pub(crate) fn beacon_effect_selection(&self) -> (Option<i32>, Option<i32>) {
        (self.beacon_primary_effect, self.beacon_secondary_effect)
    }

    pub(crate) fn anvil_rename_text(&self) -> &str {
        &self.anvil_rename_text
    }

    pub(crate) fn recipe_book_search_hud_state(&self) -> RecipeBookSearchHudState {
        RecipeBookSearchHudState {
            text: self.recipe_book_search_text.clone(),
            cursor: self.recipe_book_search_cursor,
            selection: self.recipe_book_search_selection,
            focused: self.recipe_book_search_focused,
        }
    }

    pub(crate) fn recipe_book_tab_selection_hud_state(&self) -> RecipeBookTabSelectionHudState {
        RecipeBookTabSelectionHudState {
            crafting: self.recipe_book_crafting_tab_index,
            furnace: self.recipe_book_furnace_tab_index,
            blast_furnace: self.recipe_book_blast_furnace_tab_index,
            smoker: self.recipe_book_smoker_tab_index,
        }
    }

    pub(crate) fn recipe_book_page_hud_state(&self) -> RecipeBookPageHudState {
        RecipeBookPageHudState {
            crafting: self.recipe_book_crafting_page,
            furnace: self.recipe_book_furnace_page,
            blast_furnace: self.recipe_book_blast_furnace_page,
            smoker: self.recipe_book_smoker_page,
        }
    }

    pub(crate) fn recipe_book_overlay_hud_state(&self) -> Option<RecipeBookOverlayHudState> {
        self.recipe_book_overlay
    }

    fn advance_creative_flight_jump_trigger(&mut self, dt_seconds: f64) {
        if self.creative_flight_jump_trigger_ticks == 0 {
            self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
            return;
        }

        self.creative_flight_jump_trigger_elapsed_seconds += dt_seconds.max(0.0);
        while self.creative_flight_jump_trigger_elapsed_seconds + f64::EPSILON
            >= CREATIVE_FLIGHT_TICK_SECONDS
            && self.creative_flight_jump_trigger_ticks > 0
        {
            self.creative_flight_jump_trigger_elapsed_seconds -= CREATIVE_FLIGHT_TICK_SECONDS;
            self.creative_flight_jump_trigger_ticks -= 1;
        }

        if self.creative_flight_jump_trigger_ticks == 0 {
            self.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        }
    }

    fn prime_sprint_trigger(&mut self) {
        self.sprint_trigger_ticks = SPRINT_TRIGGER_TICKS;
        self.sprint_trigger_elapsed_seconds = 0.0;
    }

    fn clear_sprint_trigger(&mut self) {
        self.sprint_trigger_ticks = 0;
        self.sprint_trigger_elapsed_seconds = 0.0;
    }

    fn advance_sprint_trigger(&mut self, dt_seconds: f64) {
        if self.sprint_trigger_ticks == 0 {
            self.sprint_trigger_elapsed_seconds = 0.0;
            return;
        }

        self.sprint_trigger_elapsed_seconds += dt_seconds.max(0.0);
        while self.sprint_trigger_elapsed_seconds + f64::EPSILON >= SPRINT_TRIGGER_TICK_SECONDS
            && self.sprint_trigger_ticks > 0
        {
            self.sprint_trigger_elapsed_seconds -= SPRINT_TRIGGER_TICK_SECONDS;
            self.sprint_trigger_ticks -= 1;
        }

        if self.sprint_trigger_ticks == 0 {
            self.sprint_trigger_elapsed_seconds = 0.0;
        }
    }
}

pub(crate) fn release_active_input(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let before = player_input_from_state(input);
    let before_sprinting = effective_sprinting_from_state(world, input);
    let riding_jump_charge_seconds = input.riding_jump_charge_seconds.take();
    input.clear_pressed();
    let after = player_input_from_state(input);
    let after_sprinting = effective_sprinting_from_state(world, input);
    if after != before {
        queue_player_input_command(counters, net_commands, after);
        if before_sprinting != after_sprinting {
            queue_sprint_command(counters, world, net_commands, after_sprinting);
        }
    }
    movement::queue_released_riding_jump_command(
        riding_jump_charge_seconds,
        world,
        counters,
        net_commands,
    );
    if let Some(pos) = world.take_local_destroying_block() {
        queue_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::AbortDestroyBlock,
            pos,
            ProtocolDirection::Down,
            0,
        );
    }
    if world.take_local_using_item() {
        queue_zero_pos_player_action_command(
            counters,
            net_commands,
            PlayerActionKind::ReleaseUseItem,
        );
    }
    if world.local_player_root_boat_vehicle_id().is_some() {
        queue_paddle_boat_command(counters, net_commands, false, false);
    }
}

fn dialog_ime_preedit_char_index_for_byte(text: &str, byte_index: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < byte_index.min(text.len()))
        .filter(|(_, ch)| !ch.is_control())
        .take(DIALOG_IME_PREEDIT_MAX_CHARS)
        .count()
}

pub(crate) fn handle_focus_change(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    focused: bool,
) {
    if !focused {
        release_active_input(input, world, counters, net_commands);
        input.clear_modifiers();
        input.clear_debug_key_state();
    }
    input.focused = focused;
}

fn handle_advancements_screen_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    if pressed && matches!(code, KeyCode::Escape | KeyCode::KeyL) {
        close_advancements_screen_and_queue(input, counters, world, net_commands);
    }
    true
}

fn handle_stats_screen_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) -> bool {
    if !world.stats_screen_is_open() {
        return false;
    }
    if pressed {
        if matches!(code, KeyCode::Escape) {
            close_stats_screen(input, counters, world, net_commands);
        } else if input.control_down() {
            let requested = stats_screen_digit_tab(code).or_else(|| match code {
                KeyCode::Tab => {
                    let direction = if input.shift_down() { -1 } else { 1 };
                    next_enabled_stats_screen_tab(input.stats_screen_tab, direction, world)
                }
                _ => None,
            });
            if let Some(tab) = requested.filter(|tab| stats_screen_tab_is_enabled(*tab, world)) {
                let old_tab = input.stats_screen_tab;
                input.stats_screen_tab = tab;
                input.stats_screen_total_rows = input.stats_screen_tab_row_counts[tab];
                input.stats_screen_scroll_amount = 0.0;
                input.stats_screen_scrolling = false;
                if tab != old_tab
                    && matches!(
                        input.stats_screen_focus,
                        StatsScreenFocus::List
                            | StatsScreenFocus::GeneralRow(_)
                            | StatsScreenFocus::ItemHeader(_)
                            | StatsScreenFocus::ItemRow(_)
                            | StatsScreenFocus::MobRow(_)
                    )
                {
                    // `TabManager` removes the old list widget and `Screen.removeWidget`
                    // clears focus when the shortcut is used outside the tab bar.
                    input.stats_screen_focus = StatsScreenFocus::None;
                }
            }
        } else {
            match code {
                KeyCode::Tab => stats_screen_move_tab_focus(input, !input.shift_down()),
                KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown => stats_screen_move_arrow_focus(input, world, code),
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    if stats_screen_activate_focus(input) {
                        close_stats_screen(input, counters, world, net_commands);
                    }
                }
                _ => {}
            }
        }
    }
    true
}

fn stats_screen_move_tab_focus(input: &mut ClientInputState, forward: bool) {
    let last_list_focus = || match input.stats_screen_tab {
        1 if input.stats_screen_total_rows > 0 => {
            StatsScreenFocus::ItemRow(input.stats_screen_total_rows - 1)
        }
        1 => StatsScreenFocus::ItemHeader(5),
        2 if input.stats_screen_total_rows > 0 => {
            StatsScreenFocus::MobRow(input.stats_screen_total_rows - 1)
        }
        0 if input.stats_screen_total_rows > 0 => {
            StatsScreenFocus::GeneralRow(input.stats_screen_total_rows - 1)
        }
        _ => StatsScreenFocus::List,
    };
    let first_list_focus = || match input.stats_screen_tab {
        1 => StatsScreenFocus::ItemHeader(0),
        2 if input.stats_screen_total_rows > 0 => StatsScreenFocus::MobRow(0),
        0 if input.stats_screen_total_rows > 0 => StatsScreenFocus::GeneralRow(0),
        _ => StatsScreenFocus::List,
    };
    input.stats_screen_focus = if forward {
        match input.stats_screen_focus {
            StatsScreenFocus::None => StatsScreenFocus::Tabs,
            StatsScreenFocus::Tabs => first_list_focus(),
            StatsScreenFocus::ItemHeader(column) if column < 5 => {
                StatsScreenFocus::ItemHeader(column + 1)
            }
            StatsScreenFocus::ItemHeader(_) if input.stats_screen_total_rows > 0 => {
                StatsScreenFocus::ItemRow(0)
            }
            StatsScreenFocus::ItemRow(row) if row + 1 < input.stats_screen_total_rows => {
                StatsScreenFocus::ItemRow(row + 1)
            }
            StatsScreenFocus::List
            | StatsScreenFocus::GeneralRow(_)
            | StatsScreenFocus::ItemHeader(_)
            | StatsScreenFocus::ItemRow(_)
            | StatsScreenFocus::MobRow(_) => StatsScreenFocus::Done,
            StatsScreenFocus::Done => StatsScreenFocus::Tabs,
        }
    } else {
        match input.stats_screen_focus {
            StatsScreenFocus::None => StatsScreenFocus::Done,
            StatsScreenFocus::Tabs => StatsScreenFocus::Done,
            StatsScreenFocus::ItemHeader(column) if column > 0 => {
                StatsScreenFocus::ItemHeader(column - 1)
            }
            StatsScreenFocus::ItemRow(0) => StatsScreenFocus::ItemHeader(5),
            StatsScreenFocus::ItemRow(row) => StatsScreenFocus::ItemRow(row - 1),
            StatsScreenFocus::List
            | StatsScreenFocus::GeneralRow(_)
            | StatsScreenFocus::ItemHeader(_)
            | StatsScreenFocus::MobRow(_) => StatsScreenFocus::Tabs,
            StatsScreenFocus::Done => last_list_focus(),
        }
    };
}

fn stats_screen_move_arrow_focus(input: &mut ClientInputState, world: &WorldStore, code: KeyCode) {
    input.stats_screen_focus = match (input.stats_screen_focus, code) {
        (StatsScreenFocus::Tabs, KeyCode::ArrowLeft) => {
            let tab = next_enabled_stats_screen_tab(input.stats_screen_tab, -1, world)
                .unwrap_or(input.stats_screen_tab);
            input.stats_screen_tab = tab;
            input.stats_screen_total_rows = input.stats_screen_tab_row_counts[tab];
            input.stats_screen_scroll_amount = 0.0;
            StatsScreenFocus::Tabs
        }
        (StatsScreenFocus::Tabs, KeyCode::ArrowRight) => {
            let tab = next_enabled_stats_screen_tab(input.stats_screen_tab, 1, world)
                .unwrap_or(input.stats_screen_tab);
            input.stats_screen_tab = tab;
            input.stats_screen_total_rows = input.stats_screen_tab_row_counts[tab];
            input.stats_screen_scroll_amount = 0.0;
            StatsScreenFocus::Tabs
        }
        (StatsScreenFocus::Tabs, KeyCode::ArrowDown) => match input.stats_screen_tab {
            1 => StatsScreenFocus::ItemHeader(0),
            2 if input.stats_screen_total_rows > 0 => StatsScreenFocus::MobRow(0),
            0 if input.stats_screen_total_rows > 0 => StatsScreenFocus::GeneralRow(0),
            _ => StatsScreenFocus::List,
        },
        (StatsScreenFocus::GeneralRow(0), KeyCode::ArrowUp)
        | (StatsScreenFocus::MobRow(0), KeyCode::ArrowUp)
        | (StatsScreenFocus::ItemHeader(_), KeyCode::ArrowUp) => StatsScreenFocus::Tabs,
        (StatsScreenFocus::GeneralRow(row), KeyCode::ArrowUp) => {
            StatsScreenFocus::GeneralRow(row - 1)
        }
        (StatsScreenFocus::MobRow(row), KeyCode::ArrowUp) => StatsScreenFocus::MobRow(row - 1),
        (StatsScreenFocus::ItemRow(0), KeyCode::ArrowUp) => StatsScreenFocus::ItemHeader(0),
        (StatsScreenFocus::ItemRow(row), KeyCode::ArrowUp) => StatsScreenFocus::ItemRow(row - 1),
        (StatsScreenFocus::GeneralRow(row), KeyCode::ArrowDown)
            if row + 1 < input.stats_screen_total_rows =>
        {
            StatsScreenFocus::GeneralRow(row + 1)
        }
        (StatsScreenFocus::MobRow(row), KeyCode::ArrowDown)
            if row + 1 < input.stats_screen_total_rows =>
        {
            StatsScreenFocus::MobRow(row + 1)
        }
        (StatsScreenFocus::ItemHeader(_), KeyCode::ArrowDown)
            if input.stats_screen_total_rows > 0 =>
        {
            StatsScreenFocus::ItemRow(0)
        }
        (StatsScreenFocus::ItemRow(row), KeyCode::ArrowDown)
            if row + 1 < input.stats_screen_total_rows =>
        {
            StatsScreenFocus::ItemRow(row + 1)
        }
        (
            StatsScreenFocus::GeneralRow(_)
            | StatsScreenFocus::MobRow(_)
            | StatsScreenFocus::ItemRow(_),
            KeyCode::ArrowDown,
        ) => StatsScreenFocus::Done,
        (StatsScreenFocus::ItemHeader(column), KeyCode::ArrowLeft) if column > 0 => {
            StatsScreenFocus::ItemHeader(column - 1)
        }
        (StatsScreenFocus::ItemHeader(column), KeyCode::ArrowRight) if column < 5 => {
            StatsScreenFocus::ItemHeader(column + 1)
        }
        (StatsScreenFocus::Done, KeyCode::ArrowUp) => match input.stats_screen_tab {
            1 if input.stats_screen_total_rows > 0 => {
                StatsScreenFocus::ItemRow(input.stats_screen_total_rows - 1)
            }
            1 => StatsScreenFocus::ItemHeader(0),
            2 if input.stats_screen_total_rows > 0 => {
                StatsScreenFocus::MobRow(input.stats_screen_total_rows - 1)
            }
            0 if input.stats_screen_total_rows > 0 => {
                StatsScreenFocus::GeneralRow(input.stats_screen_total_rows - 1)
            }
            _ => StatsScreenFocus::List,
        },
        (focus, _) => focus,
    };
}

/// Returns whether the focused Done button should close the screen.
fn stats_screen_activate_focus(input: &mut ClientInputState) -> bool {
    match input.stats_screen_focus {
        StatsScreenFocus::ItemHeader(column) => {
            input.activate_stats_screen_item_column(column);
            false
        }
        StatsScreenFocus::Done => true,
        _ => false,
    }
}

fn stats_screen_digit_tab(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
        KeyCode::Digit0
        | KeyCode::Digit3
        | KeyCode::Digit4
        | KeyCode::Digit5
        | KeyCode::Digit6
        | KeyCode::Digit7
        | KeyCode::Digit8
        | KeyCode::Digit9
        | KeyCode::Numpad0
        | KeyCode::Numpad3
        | KeyCode::Numpad4
        | KeyCode::Numpad5
        | KeyCode::Numpad6
        | KeyCode::Numpad7
        | KeyCode::Numpad8
        | KeyCode::Numpad9 => Some(2),
        _ => None,
    }
}

fn stats_screen_tab_is_enabled(tab: usize, world: &WorldStore) -> bool {
    if tab == 0 {
        return true;
    }
    if world.stats_screen_is_loading() {
        return false;
    }
    match tab {
        1 => [
            "minecraft:mined",
            "minecraft:broken",
            "minecraft:crafted",
            "minecraft:used",
            "minecraft:picked_up",
            "minecraft:dropped",
        ]
        .into_iter()
        .any(|stat_type| world.has_positive_stats_for_type(stat_type)),
        2 => {
            world.has_positive_stats_for_type("minecraft:killed")
                || world.has_positive_stats_for_type("minecraft:killed_by")
        }
        _ => false,
    }
}

fn next_enabled_stats_screen_tab(
    current: usize,
    direction: i32,
    world: &WorldStore,
) -> Option<usize> {
    let mut next = current;
    for _ in 0..3 {
        next = usize::try_from((next as i32 + direction).rem_euclid(3)).ok()?;
        if stats_screen_tab_is_enabled(next, world) {
            return Some(next);
        }
    }
    None
}

fn open_advancements_screen(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.open_advancements_screen() {
        release_active_input(input, world, counters, net_commands);
        if let Some(tab) = world.ensure_advancements_screen_selected_tab() {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
    }
}

fn open_stats_screen(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.open_stats_screen() {
        release_active_input(input, world, counters, net_commands);
        input.stats_screen_cursor_position = None;
        input.stats_screen_scroll_amount = 0.0;
        input.stats_screen_max_scroll_amount = 0.0;
        input.stats_screen_scroller_height = 0;
        input.stats_screen_scrolling = false;
        input.stats_screen_tab = 0;
        input.stats_screen_item_sort_column = None;
        input.stats_screen_item_sort_order = 0;
        input.stats_screen_item_effective_sort = None;
        input.stats_screen_tab_row_counts = [0; 3];
        input.stats_screen_total_rows = 0;
        input.stats_screen_focus = StatsScreenFocus::None;
        input.stats_screen_was_loading = true;
        queue_request_stats_command(counters, net_commands);
    }
}

fn close_stats_screen(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.close_stats_screen() {
        release_active_input(input, world, counters, net_commands);
        input.stats_screen_cursor_position = None;
        input.stats_screen_scroll_amount = 0.0;
        input.stats_screen_max_scroll_amount = 0.0;
        input.stats_screen_scroller_height = 0;
        input.stats_screen_scrolling = false;
        input.stats_screen_tab = 0;
        input.stats_screen_item_sort_column = None;
        input.stats_screen_item_sort_order = 0;
        input.stats_screen_item_effective_sort = None;
        input.stats_screen_tab_row_counts = [0; 3];
        input.stats_screen_total_rows = 0;
        input.stats_screen_focus = StatsScreenFocus::None;
        input.stats_screen_was_loading = false;
    }
}

fn close_advancements_screen_and_queue(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if world.close_advancements_screen() {
        release_active_input(input, world, counters, net_commands);
        queue_seen_advancements_command(counters, net_commands, SeenAdvancements::ClosedScreen);
    }
}

pub(crate) fn handle_advancements_screen_mouse_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    input.inventory_cursor_position =
        advancements_screen_cursor_position(cursor_position, surface_size);
    input.inventory_hovered_slot = None;
    if matches!(state, ElementState::Released) {
        input.advancement_is_scrolling = false;
        if matches!(button, MouseButton::Left) {
            input.advancement_mouse_left_down = false;
        }
        return true;
    }
    if !matches!(button, MouseButton::Left) {
        return true;
    }
    input.advancement_mouse_left_down = true;
    input.advancement_is_scrolling = false;
    if let Some(tab) = advancements_tab_at_position(world, cursor_position, surface_size) {
        if let Some(tab) = world.select_advancements_root_tab(&tab) {
            queue_seen_advancements_command(
                counters,
                net_commands,
                SeenAdvancements::OpenedTab { tab },
            );
        }
    } else if advancements_done_button_contains(cursor_position, surface_size) {
        close_advancements_screen_and_queue(input, counters, world, net_commands);
    }
    true
}

pub(crate) fn handle_advancements_screen_cursor_moved(
    input: &mut ClientInputState,
    world: &WorldStore,
    previous_position: Option<PhysicalPosition<f64>>,
    cursor_position: Option<PhysicalPosition<f64>>,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    input.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
    input.inventory_hovered_slot = None;
    if !input.advancement_mouse_left_down {
        return true;
    }
    let (Some(previous), Some(current)) = (previous_position, cursor_position) else {
        return true;
    };
    if !input.advancement_is_scrolling {
        input.advancement_is_scrolling = true;
        return true;
    }
    let Some(tab) = world.selected_advancements_tab() else {
        return true;
    };
    let entry = input
        .advancement_scroll_deltas
        .entry(tab.to_string())
        .or_default();
    entry.0 += current.x - previous.x;
    entry.1 += current.y - previous.y;
    true
}

pub(crate) fn handle_advancements_screen_mouse_wheel(
    input: &mut ClientInputState,
    world: &WorldStore,
    delta: MouseScrollDelta,
) -> bool {
    if !world.advancements_screen_is_open() {
        return false;
    }
    let Some(tab) = world.selected_advancements_tab() else {
        return true;
    };
    let Some((wheel_x, wheel_y)) = advancement_wheel_steps_from_scroll(input, delta) else {
        return true;
    };
    let entry = input
        .advancement_scroll_deltas
        .entry(tab.to_string())
        .or_default();
    entry.0 += f64::from(wheel_x) * 16.0;
    entry.1 += f64::from(wheel_y) * 16.0;
    true
}

fn advancement_wheel_steps_from_scroll(
    input: &mut ClientInputState,
    delta: MouseScrollDelta,
) -> Option<(i32, i32)> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
    };
    if input.scroll_accumulated_x != 0.0
        && advancement_scroll_signum(x) != advancement_scroll_signum(input.scroll_accumulated_x)
    {
        input.scroll_accumulated_x = 0.0;
    }
    if input.scroll_accumulated_y != 0.0
        && advancement_scroll_signum(y) != advancement_scroll_signum(input.scroll_accumulated_y)
    {
        input.scroll_accumulated_y = 0.0;
    }

    input.scroll_accumulated_x += x;
    input.scroll_accumulated_y += y;
    let wheel_x = input.scroll_accumulated_x as i32;
    let wheel_y = input.scroll_accumulated_y as i32;
    if wheel_x == 0 && wheel_y == 0 {
        return None;
    }

    input.scroll_accumulated_x -= f64::from(wheel_x);
    input.scroll_accumulated_y -= f64::from(wheel_y);
    Some((wheel_x, wheel_y))
}

fn advancements_screen_cursor_position(
    cursor_position: Option<PhysicalPosition<f64>>,
    _surface_size: PhysicalSize<u32>,
) -> Option<(i32, i32)> {
    cursor_position.and_then(physical_position_floor_i32)
}

fn physical_position_floor_i32(position: PhysicalPosition<f64>) -> Option<(i32, i32)> {
    (position.x.is_finite() && position.y.is_finite())
        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
}

fn advancement_scroll_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn advancements_tab_at_position(
    world: &WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<String> {
    let cursor = cursor_position?;
    let tabs = world.advancement_root_tabs();
    if tabs.len() <= 1 {
        return None;
    }
    let window_x =
        (f64::from(surface_size.width.max(1)) - f64::from(ADVANCEMENTS_WINDOW_WIDTH)) * 0.5;
    let window_y =
        (f64::from(surface_size.height.max(1)) - f64::from(ADVANCEMENTS_WINDOW_HEIGHT)) * 0.5;
    tabs.into_iter().find_map(|tab| {
        let (x, y, width, height) = advancements_tab_bounds(tab.display_index, window_x, window_y)?;
        (cursor.x > x && cursor.x < x + width && cursor.y > y && cursor.y < y + height)
            .then_some(tab.id)
    })
}

fn advancements_tab_bounds(
    display_index: usize,
    window_x: f64,
    window_y: f64,
) -> Option<(f64, f64, f64, f64)> {
    let (x, y, width, height) = if display_index < ADVANCEMENTS_TAB_ABOVE_MAX {
        let index = i32::try_from(display_index).ok()?;
        (
            (ADVANCEMENTS_TAB_ABOVE_WIDTH + 4) * index,
            -ADVANCEMENTS_TAB_ABOVE_HEIGHT + 4,
            ADVANCEMENTS_TAB_ABOVE_WIDTH,
            ADVANCEMENTS_TAB_ABOVE_HEIGHT,
        )
    } else if display_index < ADVANCEMENTS_TAB_ABOVE_MAX + ADVANCEMENTS_TAB_BELOW_MAX {
        let index = i32::try_from(display_index - ADVANCEMENTS_TAB_ABOVE_MAX).ok()?;
        (
            (ADVANCEMENTS_TAB_BELOW_WIDTH + 4) * index,
            136,
            ADVANCEMENTS_TAB_BELOW_WIDTH,
            ADVANCEMENTS_TAB_BELOW_HEIGHT,
        )
    } else if display_index
        < ADVANCEMENTS_TAB_ABOVE_MAX + ADVANCEMENTS_TAB_BELOW_MAX + ADVANCEMENTS_TAB_LEFT_MAX
    {
        let index =
            i32::try_from(display_index - ADVANCEMENTS_TAB_ABOVE_MAX - ADVANCEMENTS_TAB_BELOW_MAX)
                .ok()?;
        (
            -ADVANCEMENTS_TAB_LEFT_WIDTH + 4,
            ADVANCEMENTS_TAB_LEFT_HEIGHT * index,
            ADVANCEMENTS_TAB_LEFT_WIDTH,
            ADVANCEMENTS_TAB_LEFT_HEIGHT,
        )
    } else if display_index
        < ADVANCEMENTS_TAB_ABOVE_MAX
            + ADVANCEMENTS_TAB_BELOW_MAX
            + ADVANCEMENTS_TAB_LEFT_MAX
            + ADVANCEMENTS_TAB_RIGHT_MAX
    {
        let index = i32::try_from(
            display_index
                - ADVANCEMENTS_TAB_ABOVE_MAX
                - ADVANCEMENTS_TAB_BELOW_MAX
                - ADVANCEMENTS_TAB_LEFT_MAX,
        )
        .ok()?;
        (
            248,
            ADVANCEMENTS_TAB_RIGHT_HEIGHT * index,
            ADVANCEMENTS_TAB_RIGHT_WIDTH,
            ADVANCEMENTS_TAB_RIGHT_HEIGHT,
        )
    } else {
        return None;
    };
    Some((
        window_x + f64::from(x),
        window_y + f64::from(y),
        f64::from(width),
        f64::from(height),
    ))
}

fn advancements_done_button_contains(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let x =
        (f64::from(surface_size.width.max(1)) - f64::from(ADVANCEMENTS_DONE_BUTTON_WIDTH)) * 0.5;
    let y = f64::from(surface_size.height.max(1)) - f64::from(ADVANCEMENTS_FOOTER_HEIGHT)
        + f64::from(ADVANCEMENTS_FOOTER_HEIGHT - ADVANCEMENTS_DONE_BUTTON_HEIGHT) * 0.5;
    cursor.x >= x
        && cursor.x < x + f64::from(ADVANCEMENTS_DONE_BUTTON_WIDTH)
        && cursor.y >= y
        && cursor.y < y + f64::from(ADVANCEMENTS_DONE_BUTTON_HEIGHT)
}

#[cfg(test)]
pub(crate) fn handle_key_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    physical_key: PhysicalKey,
    state: ElementState,
) {
    handle_key_input_with_item_runtime(
        input,
        counters,
        world,
        net_commands,
        None,
        physical_key,
        state,
        PhysicalSize::new(1280, 720),
        &FallbackTextMeasurer,
        None,
    );
}

pub(crate) fn handle_key_input_with_item_runtime(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    physical_key: PhysicalKey,
    state: ElementState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    clipboard: Option<&mut dyn DebugClipboard>,
) {
    if !input.focused {
        return;
    }

    let pressed = matches!(state, ElementState::Pressed);
    let PhysicalKey::Code(code) = physical_key else {
        return;
    };
    let just_pressed = pressed && !input.pressed_keys.contains(&code);
    input.set_key_down(code, pressed);
    if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
        input.set_shift_key(code, pressed);
    }
    if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
        input.set_control_key(code, pressed);
    }

    if input.handle_death_screen_key(counters, world, net_commands, code, just_pressed) {
        return;
    }

    if input.handle_game_event_screen_key(
        counters,
        world,
        net_commands,
        code,
        pressed,
        just_pressed,
    ) {
        return;
    }

    if input.handle_debug_pause_screen_key(physical_key, state) {
        return;
    }

    if input.handle_debug_overlay_key_with_clipboard_and_net(
        physical_key,
        state,
        Some(world),
        None,
        None,
        item_runtime,
        counters,
        net_commands,
    ) {
        return;
    }

    if input.debug_pause_screen_is_open() {
        return;
    }

    if input.handle_game_rules_screen_key(
        counters,
        world,
        net_commands,
        code,
        pressed,
        surface_size,
    ) {
        return;
    }

    if input.handle_structure_block_editor_key(counters, world, net_commands, code, pressed) {
        return;
    }

    if input.handle_jigsaw_block_editor_key(counters, world, net_commands, code, pressed) {
        return;
    }

    if input.handle_test_block_editor_key(world, code, pressed) {
        return;
    }

    if input.handle_test_instance_block_editor_key(counters, world, net_commands, code, pressed) {
        return;
    }

    let mut clipboard = clipboard;
    if handle_sign_editor_key(
        input,
        counters,
        world,
        net_commands,
        code,
        pressed,
        text_measurer,
        &mut clipboard,
    ) {
        return;
    }

    if input.command_entry_is_active() {
        handle_chat_entry_key(input, counters, world, net_commands, code, pressed);
        return;
    }

    if input.handle_book_screen_key(
        counters,
        world,
        net_commands,
        code,
        pressed,
        text_measurer,
        &mut clipboard,
    ) {
        return;
    }

    if handle_advancements_screen_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if handle_stats_screen_key(input, counters, world, net_commands, code, pressed) {
        return;
    }

    if world.dialog_screen_is_open() {
        return;
    }

    if world.local_player_is_dead() {
        if pressed && matches!(code, KeyCode::Enter | KeyCode::Space) {
            queue_perform_respawn_command(counters, net_commands);
        }
        return;
    }

    if world.local_player_is_sleeping() {
        if pressed && matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::Space) {
            queue_player_command_action(
                counters,
                world,
                net_commands,
                PlayerCommandAction::StopSleeping,
                0,
            );
        }
        return;
    }

    if pressed {
        if matches!(code, KeyCode::Escape)
            && maybe_close_narrow_recipe_book(input, world, counters, net_commands, surface_size)
        {
            return;
        }
        if matches!(code, KeyCode::Escape)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
        if matches!(code, KeyCode::KeyE)
            && !anvil_rename_entry_consumes_key(world, code)
            && !recipe_book_search_entry_consumes_key(input, world, code)
            && queue_container_close_command(counters, world, net_commands)
        {
            return;
        }
    }
    if inventory_screen_layout(world).is_some() {
        if pressed {
            handle_inventory_key_input(input, world, counters, net_commands, item_runtime, code);
        }
        return;
    }
    if world.open_container_id().is_some() {
        return;
    }

    if pressed {
        if matches!(code, KeyCode::Escape) {
            input.set_key_down(code, false);
            input.open_debug_pause_screen_with_menu();
            return;
        }
        input.record_debug_profiler_chart_navigation_key(code);
        if just_pressed && code == KeyCode::F1 {
            input.hide_gui = !input.hide_gui;
            return;
        }
        if let Some(slot) = hotbar_slot_for_key(code) {
            if world.local_player_is_spectator() {
                input.handle_spectator_hotbar_selected(world, counters, net_commands, slot);
                return;
            }
            select_hotbar_slot(counters, world, net_commands, slot);
            return;
        }
        match code {
            KeyCode::KeyQ => {
                if world.local_player_is_spectator() {
                    return;
                }
                let drop_all = input.control_down();
                let action = if drop_all {
                    PlayerActionKind::DropAllItems
                } else {
                    PlayerActionKind::DropItem
                };
                let dropped_item = world.drop_local_selected_hotbar_item(drop_all);
                queue_zero_pos_player_action_command(counters, net_commands, action);
                if dropped_item {
                    queue_swing_command(world, counters, net_commands, InteractionHand::MainHand);
                }
                return;
            }
            KeyCode::KeyF => {
                if world.local_player_is_spectator() {
                    return;
                }
                queue_zero_pos_player_action_command(
                    counters,
                    net_commands,
                    PlayerActionKind::SwapItemWithOffhand,
                );
                return;
            }
            KeyCode::KeyE => {
                if world
                    .local_player_server_controlled_inventory_vehicle_id()
                    .is_some()
                {
                    queue_player_command_action(
                        counters,
                        world,
                        net_commands,
                        PlayerCommandAction::OpenInventory,
                        0,
                    );
                } else {
                    world.open_local_inventory();
                }
                return;
            }
            KeyCode::KeyL => {
                open_advancements_screen(input, counters, world, net_commands);
                return;
            }
            KeyCode::KeyT => {
                open_chat_entry(input, world, counters, net_commands, true);
                return;
            }
            _ => {}
        }
    }

    let before = player_input_from_state(input);
    let before_sprinting = effective_sprinting_from_state(world, input);
    let handled = match code {
        KeyCode::KeyW | KeyCode::ArrowUp => {
            input.forward = pressed;
            if pressed && !before.forward {
                maybe_apply_forward_sprint_trigger(input, world);
            }
            true
        }
        KeyCode::KeyS | KeyCode::ArrowDown => {
            input.backward = pressed;
            if pressed {
                input.clear_sprint_trigger();
            }
            true
        }
        KeyCode::KeyA | KeyCode::ArrowLeft => {
            input.left = pressed;
            true
        }
        KeyCode::KeyD | KeyCode::ArrowRight => {
            input.right = pressed;
            true
        }
        KeyCode::Space => {
            input.jump = pressed;
            true
        }
        KeyCode::ShiftLeft | KeyCode::ShiftRight => {
            input.sneak = input.shift_down();
            if input.sneak {
                input.clear_sprint_trigger();
            }
            true
        }
        KeyCode::ControlLeft | KeyCode::ControlRight => {
            input.sprint = pressed;
            if pressed {
                input.clear_sprint_trigger();
            }
            true
        }
        _ => false,
    };
    if handled {
        let after = player_input_from_state(input);
        let after_sprinting = effective_sprinting_from_state(world, input);
        if after != before {
            queue_player_input_command(counters, net_commands, after);
            if before_sprinting != after_sprinting {
                queue_sprint_command(counters, world, net_commands, after_sprinting);
            }
            let just_toggled_creative_flight =
                maybe_toggle_creative_flight(input, counters, world, net_commands, before, after);
            if should_queue_start_fall_flying(world, before, after) {
                if !just_toggled_creative_flight {
                    queue_player_command_action(
                        counters,
                        world,
                        net_commands,
                        PlayerCommandAction::StartFallFlying,
                        0,
                    );
                }
            }
        }
    }
}

fn maybe_apply_forward_sprint_trigger(input: &mut ClientInputState, world: &WorldStore) {
    if input.sprint {
        input.clear_sprint_trigger();
        return;
    }
    let mut sprint_candidate = local_player_input_from_state(input);
    sprint_candidate.sprint = true;
    if !world.local_player_effective_sprint(sprint_candidate) {
        return;
    }
    if input.sprint_trigger_ticks > 0 {
        input.sprint = true;
        input.clear_sprint_trigger();
    } else {
        input.prime_sprint_trigger();
    }
}

fn maybe_toggle_creative_flight(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    before: PlayerInput,
    after: PlayerInput,
) -> bool {
    if world.local_player_is_spectator() {
        return false;
    }
    if !after.jump || before.jump || !world.local_player().abilities.is_some_and(|a| a.can_fly) {
        return false;
    }

    if world.local_player_root_vehicle_id().is_some()
        && world.local_player_rideable_jumping_vehicle_id().is_none()
    {
        input.creative_flight_jump_trigger_ticks = 0;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        return false;
    }

    if input.creative_flight_jump_trigger_ticks == 0 {
        input.creative_flight_jump_trigger_ticks = CREATIVE_FLIGHT_JUMP_TRIGGER_TICKS;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
        return false;
    }

    let flying = !world.local_player().abilities.is_some_and(|a| a.flying);
    let toggled = queue_player_abilities_command(counters, world, net_commands, flying);
    if toggled {
        input.creative_flight_jump_trigger_ticks = 0;
        input.creative_flight_jump_trigger_elapsed_seconds = 0.0;
    }
    toggled
}

fn should_queue_start_fall_flying(
    world: &WorldStore,
    before: PlayerInput,
    after: PlayerInput,
) -> bool {
    after.jump
        && !before.jump
        && world.local_player_root_vehicle_id().is_none()
        && world
            .local_player_pose()
            .is_some_and(|pose| !pose.on_ground)
        && world.local_player_has_equipped_elytra()
}

#[cfg(test)]
pub(crate) fn handle_text_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
) {
    handle_text_input_with_item_runtime(
        input,
        counters,
        world,
        net_commands,
        None,
        text,
        &FallbackTextMeasurer,
    );
}

pub(crate) fn handle_text_input_with_item_runtime(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) {
    if !input.focused {
        return;
    }

    if input.handle_game_rules_screen_text(world, text) {
        return;
    }

    if input.handle_structure_block_editor_text(world, text) {
        return;
    }

    if input.handle_jigsaw_block_editor_text(world, text) {
        return;
    }

    if input.handle_test_block_editor_text(world, text) {
        return;
    }

    if input.handle_test_instance_block_editor_text(counters, world, net_commands, text) {
        return;
    }

    if handle_sign_editor_text(input, counters, world, net_commands, text, text_measurer) {
        return;
    }

    if input.handle_book_screen_text(world, text, text_measurer) {
        return;
    }

    if world.dialog_screen_is_open()
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.advancements_screen_is_open()
        || world.stats_screen_is_open()
        || world.game_rules_screen().is_some()
    {
        return;
    }

    if handle_inventory_text_input(input, world, counters, net_commands, item_runtime, text) {
        return;
    }

    if input.chat_entry.is_none() {
        if !text.starts_with('/') {
            return;
        }
        open_chat_entry(input, world, counters, net_commands, false);
    }

    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if entry.suppress_open_key_commit {
        entry.suppress_open_key_commit = false;
        if matches!(text, "t" | "T") {
            return;
        }
    }
    let before = entry.text.clone();
    insert_chat_entry_text(entry, text);
    update_chat_entry_after_text_change(input, counters, world, net_commands, before.as_str());
}

fn open_chat_entry(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    suppress_open_key_commit: bool,
) {
    release_active_input(input, world, counters, net_commands);
    input.chat_entry = Some(ChatEntryState {
        text: String::new(),
        cursor: 0,
        selection: 0,
        last_suggestion_request_text: None,
        suggestion_list: None,
        hidden_suggestion_id: None,
        suggestion_narration: None,
        mouse_position: None,
        suppress_open_key_commit,
        owner: ChatEntryOwner::Chat,
    });
    input.chat_scroll_lines = 0;
}

fn handle_chat_entry_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
) {
    if !pressed {
        return;
    }

    if let Some(entry) = input.chat_entry.as_mut() {
        sync_chat_suggestion_list(entry, world);
    }

    match code {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            submit_chat_entry(input, counters, world, net_commands)
        }
        KeyCode::Escape => {
            let hidden = input.chat_entry.as_mut().and_then(|entry| {
                let list = entry.suggestion_list.take()?;
                entry.hidden_suggestion_id = Some(list.result_id);
                entry.suggestion_narration = None;
                Some(())
            });
            if hidden.is_none() {
                let close_owner = input.chat_entry.as_ref().map(|entry| &entry.owner);
                let closes_command_block =
                    matches!(close_owner, Some(ChatEntryOwner::CommandBlock(_)));
                let closes_minecart_command_block =
                    matches!(close_owner, Some(ChatEntryOwner::MinecartCommandBlock(_)));
                input.chat_entry = None;
                input.chat_scroll_lines = 0;
                if closes_command_block {
                    world.close_command_block_editor();
                }
                if closes_minecart_command_block {
                    world.close_minecart_command_block_editor();
                }
            }
        }
        KeyCode::PageUp => {
            if input
                .chat_entry
                .as_ref()
                .is_none_or(|entry| !entry.commands_only())
            {
                input.chat_scroll_lines = input.chat_scroll_lines.saturating_add(19).min(100);
            }
        }
        KeyCode::PageDown => {
            if input
                .chat_entry
                .as_ref()
                .is_none_or(|entry| !entry.commands_only())
            {
                input.chat_scroll_lines = input.chat_scroll_lines.saturating_sub(19);
            }
        }
        KeyCode::Tab => {
            let shift_down = input.shift_down();
            let revealed_list = input
                .chat_entry
                .as_mut()
                .is_some_and(|entry| show_hidden_chat_suggestion_list(entry, world));
            if revealed_list {
                return;
            }
            let used_list = input.chat_entry.as_mut().is_some_and(|entry| {
                let Some(tab_cycles) = entry.suggestion_list.as_ref().map(|list| list.tab_cycles)
                else {
                    return false;
                };
                if tab_cycles {
                    cycle_chat_suggestion(entry, if shift_down { -1 } else { 1 });
                }
                use_selected_chat_suggestion(entry);
                true
            });
            if !used_list {
                apply_latest_chat_suggestion(input, world);
            }
        }
        KeyCode::KeyA if input.control_down() && !input.shift_down() => {
            if let Some(entry) = &mut input.chat_entry {
                select_chat_entry_text(entry);
            }
        }
        KeyCode::ArrowUp => {
            if let Some(entry) = input.chat_entry.as_mut() {
                if entry.suggestion_list.is_some() {
                    cycle_chat_suggestion(entry, -1);
                    if let Some(list) = entry.suggestion_list.as_mut() {
                        list.tab_cycles = false;
                    }
                }
            }
        }
        KeyCode::ArrowDown => {
            if let Some(entry) = input.chat_entry.as_mut() {
                if entry.suggestion_list.is_some() {
                    cycle_chat_suggestion(entry, 1);
                    if let Some(list) = entry.suggestion_list.as_mut() {
                        list.tab_cycles = false;
                    }
                }
            }
        }
        KeyCode::ArrowLeft => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let cursor = if control_down {
                    text_edit::word_position(&entry.text, entry.cursor, -1)
                } else {
                    entry.cursor.saturating_sub(1)
                };
                set_chat_entry_cursor(entry, cursor);
            }
        }
        KeyCode::ArrowRight => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let cursor = if control_down {
                    text_edit::word_position(&entry.text, entry.cursor, 1)
                } else {
                    (entry.cursor + 1).min(text_edit::char_len(&entry.text))
                };
                set_chat_entry_cursor(entry, cursor);
            }
        }
        KeyCode::Home => {
            if let Some(entry) = &mut input.chat_entry {
                set_chat_entry_cursor(entry, 0);
            }
        }
        KeyCode::End => {
            if let Some(entry) = &mut input.chat_entry {
                set_chat_entry_cursor(entry, text_edit::char_len(&entry.text));
            }
        }
        KeyCode::Backspace => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                if entry.text.is_empty() && !entry.commands_only() {
                    input.chat_entry = None;
                    return;
                }
                let before = entry.text.clone();
                let deleted_selection = delete_chat_entry_selection(entry);
                if !deleted_selection && control_down {
                    text_edit::remove_word_before_cursor(&mut entry.text, &mut entry.cursor);
                    entry.selection = entry.cursor;
                } else if !deleted_selection {
                    remove_chat_entry_char_before_cursor(&mut entry.text, &mut entry.cursor);
                    entry.selection = entry.cursor;
                }
                update_chat_entry_after_text_change(
                    input,
                    counters,
                    world,
                    net_commands,
                    before.as_str(),
                );
            }
        }
        KeyCode::Delete => {
            let control_down = input.control_down();
            if let Some(entry) = &mut input.chat_entry {
                let before = entry.text.clone();
                let deleted_selection = delete_chat_entry_selection(entry);
                if !deleted_selection && control_down {
                    text_edit::remove_word_at_cursor(&mut entry.text, entry.cursor);
                    entry.selection = entry.cursor;
                } else if !deleted_selection {
                    remove_chat_entry_char_at_cursor(&mut entry.text, entry.cursor);
                    entry.selection = entry.cursor;
                }
                update_chat_entry_after_text_change(
                    input,
                    counters,
                    world,
                    net_commands,
                    before.as_str(),
                );
            }
        }
        _ => {}
    }
}

fn apply_latest_chat_suggestion(input: &mut ClientInputState, world: &WorldStore) {
    let Some(entry) = input.chat_entry.as_ref() else {
        return;
    };
    let Some((updated_text, updated_cursor)) =
        latest_chat_suggestion_text(world, &entry.text, entry.cursor, entry.commands_only())
    else {
        return;
    };
    let is_command = entry.commands_only() || entry.text.starts_with('/');
    if let Some(entry) = &mut input.chat_entry {
        entry.text = updated_text;
        set_chat_entry_cursor(entry, updated_cursor);
        if is_command {
            entry.last_suggestion_request_text = Some(entry.text.clone());
        }
    }
}

fn sync_chat_suggestion_list(entry: &mut ChatEntryState, world: &WorldStore) {
    if !entry.commands_only() && !entry.text.starts_with('/') {
        return;
    }
    if entry
        .suggestion_list
        .as_ref()
        .is_some_and(|list| list.tab_cycles)
    {
        return;
    }
    let completion = world.complete_command(&entry.text);
    let result = if completion.request_server {
        let suggestions = world.command_suggestions();
        let request = match suggestions.last_request.as_ref() {
            Some(request) if request.command == entry.text => request,
            _ => {
                entry.suggestion_list = None;
                return;
            }
        };
        match suggestions
            .last_id
            .and_then(|id| suggestions.by_id.get(&id))
        {
            Some(result) if result.id == request.id => result.clone(),
            _ => {
                entry.suggestion_list = None;
                return;
            }
        }
    } else {
        CommandSuggestionsResultState {
            // Server request ids are always non-negative and wrap to zero.
            id: -1,
            start: i32::try_from(completion.start).unwrap_or(i32::MAX),
            length: i32::try_from(completion.length).unwrap_or(i32::MAX),
            suggestions: completion
                .suggestions
                .into_iter()
                .map(|suggestion| CommandSuggestionState {
                    text: suggestion.text,
                    tooltip: suggestion.tooltip,
                    tooltip_translation_key: suggestion.tooltip_translation_key,
                    tooltip_styled: None,
                    tooltip_component: None,
                })
                .collect(),
        }
    };

    if entry.hidden_suggestion_id == Some(result.id) || result.suggestions.is_empty() {
        entry.suggestion_list = None;
        return;
    }
    if entry.suggestion_list.as_ref().is_some_and(|list| {
        list.original_contents == entry.text && list.result_id == result.id && list.result == result
    }) {
        return;
    }

    let cursor_byte = text_edit::byte_index(
        &entry.text,
        entry.cursor.min(text_edit::char_len(&entry.text)),
    );
    let partial = &entry.text[..cursor_byte];
    let last_word_start = partial
        .char_indices()
        .filter_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .last()
        .unwrap_or(0);
    let last_word = partial[last_word_start..].to_lowercase();
    let (direct, partial): (Vec<_>, Vec<_>) = (0..result.suggestions.len()).partition(|index| {
        let text = result.suggestions[*index].text.as_str();
        text.starts_with(&last_word)
            || text
                .strip_prefix("minecraft:")
                .is_some_and(|path| path.starts_with(&last_word))
    });
    let sorted_indices = direct.into_iter().chain(partial).collect();
    entry.suggestion_list = Some(ChatSuggestionListState {
        result_id: result.id,
        result,
        original_contents: entry.text.clone(),
        sorted_indices,
        current: 0,
        offset: 0,
        tab_cycles: false,
        rect: None,
    });
}

fn show_hidden_chat_suggestion_list(entry: &mut ChatEntryState, world: &WorldStore) -> bool {
    let Some(hidden_id) = entry.hidden_suggestion_id.take() else {
        return false;
    };
    sync_chat_suggestion_list(entry, world);
    let shown = entry
        .suggestion_list
        .as_ref()
        .is_some_and(|list| list.result_id == hidden_id);
    if shown {
        update_chat_suggestion_narration(entry);
    } else {
        entry.hidden_suggestion_id = Some(hidden_id);
    }
    shown
}

fn cycle_chat_suggestion(entry: &mut ChatEntryState, direction: i32) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let len = list.sorted_indices.len();
    if len == 0 {
        return;
    }
    let current = if direction < 0 {
        list.current.checked_sub(1).unwrap_or(len - 1)
    } else {
        (list.current + 1) % len
    };
    select_chat_suggestion(entry, current, true);
}

fn select_chat_suggestion(entry: &mut ChatEntryState, index: usize, narrate: bool) {
    let line_limit = entry.suggestion_line_limit();
    let Some(list) = entry.suggestion_list.as_mut() else {
        return;
    };
    let len = list.sorted_indices.len();
    if len == 0 {
        return;
    }
    let current = index % len;
    let changed = list.current != current;
    list.current = current;
    let last_visible = list.offset.saturating_add(line_limit.saturating_sub(1));
    if current < list.offset {
        list.offset = current.min(len.saturating_sub(line_limit));
    } else if current > last_visible {
        list.offset = current
            .saturating_add(1)
            .saturating_sub(line_limit)
            .min(len.saturating_sub(line_limit));
    }
    if !narrate || !changed {
        return;
    }
    update_chat_suggestion_narration(entry);
}

fn update_chat_suggestion_narration(entry: &mut ChatEntryState) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let len = list.sorted_indices.len();
    let current = list.current;
    let Some(suggestion) = list
        .sorted_indices
        .get(current)
        .and_then(|index| list.result.suggestions.get(*index))
    else {
        return;
    };
    entry.suggestion_narration = Some(CommandSuggestionNarrationState {
        position: current + 1,
        total: len,
        text: suggestion.text.clone(),
        tooltip: suggestion.tooltip.clone(),
        tooltip_translation_key: suggestion.tooltip_translation_key.clone(),
    });
}

fn use_selected_chat_suggestion(entry: &mut ChatEntryState) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let Some(suggestion) = list
        .sorted_indices
        .get(list.current)
        .and_then(|index| list.result.suggestions.get(*index))
    else {
        return;
    };
    let start = match usize::try_from(list.result.start) {
        Ok(start) => start,
        Err(_) => return,
    };
    let Some(updated) = apply_command_suggestion_text(
        &list.original_contents,
        list.result.start,
        list.result.length,
        &suggestion.text,
    ) else {
        return;
    };
    let Some(prefix) = list.original_contents.get(..start) else {
        return;
    };
    let cursor = text_edit::char_len(prefix).saturating_add(text_edit::char_len(&suggestion.text));
    entry.text = updated;
    set_chat_entry_cursor(entry, cursor);
    entry.last_suggestion_request_text = Some(entry.text.clone());
    if let Some(list) = entry.suggestion_list.as_mut() {
        list.tab_cycles = true;
    }
}

fn update_chat_suggestion_list_rect(
    entry: &mut ChatEntryState,
    measurer: &impl CommandSuggestionTextMeasurer,
    surface_size: PhysicalSize<u32>,
) {
    let command_block_layout = entry.commands_only();
    let Some(list) = entry.suggestion_list.as_mut() else {
        return;
    };
    let max_width = list
        .sorted_indices
        .iter()
        .filter_map(|index| list.result.suggestions.get(*index))
        .map(|suggestion| measurer.command_suggestion_text_width(&suggestion.text))
        .max()
        .unwrap_or_default();
    let start = usize::try_from(list.result.start).unwrap_or_default();
    let prefix_width = list
        .original_contents
        .get(..start)
        .map(|prefix| measurer.command_suggestion_text_width(prefix))
        .unwrap_or_default();
    let x = if command_block_layout {
        let input_x = i32::try_from(surface_size.width / 2)
            .unwrap_or(i32::MAX)
            .saturating_sub(150);
        let max_x = input_x
            .saturating_add(296)
            .saturating_sub_unsigned(max_width);
        input_x
            .saturating_add(4)
            .saturating_add_unsigned(prefix_width)
            .clamp(0, max_x)
    } else {
        let max_x = i32::try_from(surface_size.width.saturating_sub(max_width)).unwrap_or(i32::MAX);
        // ChatScreen's unbordered edit box starts at x=4. Vanilla first clamps
        // getScreenX(start), then SuggestionsList shifts that anchor one pixel
        // left (including at the right edge).
        (4i32.saturating_add_unsigned(prefix_width))
            .clamp(0, max_x)
            .saturating_sub(1)
    };
    let line_limit = if command_block_layout {
        COMMAND_BLOCK_SUGGESTION_LINE_LIMIT
    } else {
        COMMAND_SUGGESTION_LINE_LIMIT
    };
    let visible = list.sorted_indices.len().min(line_limit);
    let height = u32::try_from(visible)
        .unwrap_or_default()
        .saturating_mul(COMMAND_SUGGESTION_LINE_HEIGHT as u32);
    let y = if command_block_layout {
        71
    } else {
        i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(15)
            .saturating_sub_unsigned(height)
    };
    list.rect = Some(CommandSuggestionRect {
        x,
        y,
        width: max_width.saturating_add(1),
        height,
    });
}

fn chat_suggestion_hud_state(entry: &ChatEntryState) -> CommandSuggestionHudState {
    let empty_rect = CommandSuggestionRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let Some(list) = entry.suggestion_list.as_ref() else {
        return CommandSuggestionHudState {
            input_text: entry.text.clone(),
            cursor: entry.cursor,
            selection: entry.selection,
            suffix: None,
            rect: empty_rect,
            entries: Vec::new(),
            has_previous: false,
            has_next: false,
            tooltip: None,
            narration: entry.suggestion_narration.clone(),
            command_block_layout: entry.commands_only(),
        };
    };
    let result = &list.result;
    let visible_end = list
        .offset
        .saturating_add(entry.suggestion_line_limit())
        .min(list.sorted_indices.len());
    let entries = list.sorted_indices[list.offset..visible_end]
        .iter()
        .enumerate()
        .filter_map(|(visible, index)| {
            result
                .suggestions
                .get(*index)
                .map(|suggestion| CommandSuggestionHudEntry {
                    text: suggestion.text.clone(),
                    selected: list.offset + visible == list.current,
                })
        })
        .collect();
    let selected = list
        .sorted_indices
        .get(list.current)
        .and_then(|index| result.suggestions.get(*index));
    let suffix = selected
        .and_then(|suggestion| {
            apply_command_suggestion_text(
                &list.original_contents,
                result.start,
                result.length,
                &suggestion.text,
            )
        })
        .and_then(|suggested| suggested.strip_prefix(&entry.text).map(str::to_string));
    let hovered = entry
        .mouse_position
        .zip(list.rect)
        .is_some_and(|((x, y), rect)| rect.hovered_line(x, y).is_some());
    let tooltip = if hovered {
        selected
            .and_then(|suggestion| {
                suggestion.tooltip.clone().map(|text| {
                    let runs = suggestion.tooltip_styled.clone().unwrap_or_else(|| {
                        vec![StyledTextRun {
                            text: text.clone(),
                            style: ComponentStyle::default(),
                        }]
                    });
                    (text, suggestion.tooltip_translation_key.clone(), runs)
                })
            })
            .and_then(|(text, translation_key, runs)| {
                entry
                    .mouse_position
                    .map(|(x, y)| CommandSuggestionHudTooltip {
                        text,
                        translation_key,
                        runs,
                        x,
                        y,
                    })
            })
    } else {
        None
    };

    CommandSuggestionHudState {
        input_text: entry.text.clone(),
        cursor: entry.cursor,
        selection: entry.selection,
        suffix,
        rect: list.rect.unwrap_or(empty_rect),
        entries,
        has_previous: list.offset > 0,
        has_next: visible_end < list.sorted_indices.len(),
        tooltip,
        narration: entry.suggestion_narration.clone(),
        command_block_layout: entry.commands_only(),
    }
}

fn latest_chat_suggestion_text(
    world: &WorldStore,
    current_text: &str,
    cursor: usize,
    commands_only: bool,
) -> Option<(String, usize)> {
    if commands_only || current_text.starts_with('/') {
        let updated = latest_command_suggestion_text(world, current_text)?;
        let cursor = text_edit::char_len(&updated);
        return Some((updated, cursor));
    }
    latest_custom_chat_completion_text(world, current_text, cursor)
}

fn latest_command_suggestion_text(world: &WorldStore, current_text: &str) -> Option<String> {
    let suggestions = world.command_suggestions();
    let request = suggestions.last_request.as_ref()?;
    if request.command != current_text {
        return None;
    }

    let result = suggestions
        .last_id
        .and_then(|id| suggestions.by_id.get(&id))?;
    if result.id != request.id {
        return None;
    }
    let suggestion = result.suggestions.first()?;
    apply_command_suggestion_text(current_text, result.start, result.length, &suggestion.text)
}

fn latest_custom_chat_completion_text(
    world: &WorldStore,
    current_text: &str,
    cursor: usize,
) -> Option<(String, usize)> {
    if current_text.trim().is_empty() {
        return None;
    }
    let cursor_byte =
        text_edit::byte_index(current_text, cursor.min(text_edit::char_len(current_text)));
    let partial = &current_text[..cursor_byte];
    let last_word_start = partial
        .char_indices()
        .filter_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .last()
        .unwrap_or(0);
    let lower_prefix = partial[last_word_start..].to_lowercase();

    let mut candidates: BTreeSet<&str> = world
        .player_info()
        .entries
        .values()
        .map(|entry| entry.profile.name.as_str())
        .collect();
    candidates.extend(world.custom_chat_completions().iter().map(String::as_str));
    let suggestion = candidates
        .into_iter()
        .filter(|candidate| matches_suggestion_substring(&lower_prefix, &candidate.to_lowercase()))
        .min_by(|left, right| {
            (!is_direct_chat_suggestion(left, &lower_prefix))
                .cmp(&(!is_direct_chat_suggestion(right, &lower_prefix)))
                .then_with(|| left.to_lowercase().cmp(&right.to_lowercase()))
                .then_with(|| left.cmp(right))
        })?;

    let mut updated = String::with_capacity(
        current_text.len() - (cursor_byte - last_word_start) + suggestion.len(),
    );
    updated.push_str(&current_text[..last_word_start]);
    updated.push_str(suggestion);
    updated.push_str(&current_text[cursor_byte..]);
    let updated_cursor = text_edit::char_len(&current_text[..last_word_start])
        .saturating_add(text_edit::char_len(suggestion));
    Some((updated, updated_cursor))
}

fn is_direct_chat_suggestion(candidate: &str, lower_prefix: &str) -> bool {
    candidate.starts_with(lower_prefix)
        || candidate
            .strip_prefix("minecraft:")
            .is_some_and(|path| path.starts_with(lower_prefix))
}

fn matches_suggestion_substring(pattern: &str, input: &str) -> bool {
    input.starts_with(pattern)
        || input.char_indices().any(|(index, ch)| {
            matches!(ch, '.' | '_' | '/') && input[index + ch.len_utf8()..].starts_with(pattern)
        })
}

fn apply_command_suggestion_text(
    current_text: &str,
    start: i32,
    length: i32,
    suggestion: &str,
) -> Option<String> {
    let start = usize::try_from(start).ok()?;
    let length = usize::try_from(length).ok()?;
    let end = start.checked_add(length)?;
    if end > current_text.len()
        || !current_text.is_char_boundary(start)
        || !current_text.is_char_boundary(end)
    {
        return None;
    }

    let mut updated = String::with_capacity(current_text.len() - length + suggestion.len());
    updated.push_str(&current_text[..start]);
    updated.push_str(suggestion);
    updated.push_str(&current_text[end..]);
    Some(updated)
}

fn update_chat_entry_after_text_change(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    before: &str,
) {
    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if entry.text.is_empty() && !entry.commands_only() {
        input.chat_entry = None;
        return;
    }
    if entry.text == before {
        return;
    }
    entry.suggestion_list = None;
    entry.hidden_suggestion_id = None;
    entry.suggestion_narration = None;
    if entry.commands_only() || entry.text.starts_with('/') {
        maybe_queue_command_suggestion_request(input, counters, world, net_commands);
    }
}

fn insert_chat_entry_text(entry: &mut ChatEntryState, text: &str) {
    delete_chat_entry_selection(entry);
    let max_utf16_units = entry.max_utf16_units();
    let current = &mut entry.text;
    entry.cursor = entry.cursor.min(text_edit::char_len(current));
    let mut remaining = max_utf16_units.saturating_sub(current.encode_utf16().count());
    for ch in text.chars().filter(|ch| is_chat_text_char(*ch)) {
        let units = ch.len_utf16();
        if units > remaining {
            break;
        }
        let insert_at = text_edit::byte_index(current, entry.cursor);
        current.insert(insert_at, ch);
        entry.cursor += 1;
        remaining -= units;
    }
    entry.selection = entry.cursor;
}

fn set_chat_entry_cursor(entry: &mut ChatEntryState, cursor: usize) {
    entry.cursor = cursor.min(text_edit::char_len(&entry.text));
    entry.selection = entry.cursor;
}

fn select_chat_entry_text(entry: &mut ChatEntryState) {
    entry.selection = 0;
    entry.cursor = text_edit::char_len(&entry.text);
}

fn delete_chat_entry_selection(entry: &mut ChatEntryState) -> bool {
    if entry.selection == entry.cursor {
        return false;
    }
    let start = entry.selection.min(entry.cursor);
    let end = entry.selection.max(entry.cursor);
    let start_byte = text_edit::byte_index(&entry.text, start);
    let end_byte = text_edit::byte_index(&entry.text, end);
    entry.text.replace_range(start_byte..end_byte, "");
    entry.cursor = start;
    entry.selection = start;
    true
}

fn remove_chat_entry_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(current, *cursor - 1);
    let end = text_edit::byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_chat_entry_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(current) {
        return;
    }
    let start = text_edit::byte_index(current, cursor);
    let end = text_edit::byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn submit_chat_entry(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = input.chat_entry.take() else {
        return;
    };
    if let ChatEntryOwner::CommandBlock(editor) = &entry.owner {
        if !editor.controls_enabled {
            input.chat_entry = Some(entry);
            return;
        }
        let pos = editor.pos;
        let mode = editor.mode;
        let track_output = editor.track_output;
        let conditional = editor.conditional;
        let automatic = editor.automatic;
        queue_set_command_block_command(
            counters,
            net_commands,
            SetCommandBlock {
                pos: protocol_block_pos_from_world(pos),
                command: entry.text,
                mode,
                track_output,
                conditional,
                automatic,
            },
        );
        if !track_output {
            world.clear_command_block_editor_last_output();
        }
        world.close_command_block_editor();
        return;
    }
    if let ChatEntryOwner::MinecartCommandBlock(editor) = &entry.owner {
        let track_output = editor.track_output;
        queue_set_command_minecart_command(
            counters,
            net_commands,
            SetCommandMinecart {
                entity_id: editor.entity_id,
                command: entry.text,
                track_output,
            },
        );
        if !track_output {
            world.clear_minecart_command_block_editor_last_output();
        }
        world.close_minecart_command_block_editor();
        return;
    }
    let Some(command) = normalize_command_entry(&entry.text) else {
        let Some(message) = normalize_chat_entry(&entry.text) else {
            return;
        };
        queue_chat_message_command(counters, world, net_commands, message);
        return;
    };
    queue_chat_command(counters, world, net_commands, command);
}

fn normalize_command_entry(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let command = normalized.strip_prefix('/')?;
    if command.is_empty() {
        return None;
    }
    Some(command.to_string())
}

fn normalize_chat_entry(text: &str) -> Option<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() || normalized.starts_with('/') {
        return None;
    }
    Some(normalized)
}

fn maybe_queue_command_suggestion_request(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = &mut input.chat_entry else {
        return;
    };
    if !world.complete_command(&entry.text).request_server {
        entry.last_suggestion_request_text = None;
        return;
    }
    if entry.last_suggestion_request_text.as_deref() == Some(entry.text.as_str()) {
        return;
    }
    let text = entry.text.clone();
    entry.last_suggestion_request_text = Some(text.clone());
    let request = world.next_command_suggestion_request(text);
    queue_command_suggestion_request(counters, net_commands, request.id, request.command);
}

fn is_chat_text_char(ch: char) -> bool {
    ch >= ' ' && ch != '\u{7f}' && ch != '\u{a7}'
}

fn handle_sign_editor_text(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) -> bool {
    sync_sign_editor_input(input, counters, world, net_commands);
    let Some(editor) = &mut input.sign_editor else {
        return false;
    };

    insert_sign_line_text(editor, text, text_measurer);
    true
}

fn handle_sign_editor_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    sync_sign_editor_input(input, counters, world, net_commands);
    if input.sign_editor.is_none() {
        return false;
    }
    if !pressed {
        return true;
    }

    let control_down = input.control_down();
    let shift_down = input.shift_down();
    match code {
        KeyCode::Escape => submit_sign_editor(input, counters, net_commands),
        KeyCode::KeyA if control_down => {
            if let Some(editor) = &mut input.sign_editor {
                select_sign_editor_line(editor);
            }
        }
        KeyCode::KeyC if control_down => {
            if let Some(editor) = &input.sign_editor {
                let selected = selected_sign_line_text(editor);
                if let Some(clipboard) = clipboard.as_deref_mut() {
                    clipboard.set_debug_clipboard_text(&selected);
                }
            }
        }
        KeyCode::KeyX if control_down => {
            if let Some(editor) = &mut input.sign_editor {
                let selected = selected_sign_line_text(editor);
                if let Some(clipboard) = clipboard.as_deref_mut() {
                    clipboard.set_debug_clipboard_text(&selected);
                }
                delete_sign_selection(editor);
            }
        }
        KeyCode::KeyV if control_down => {
            let pasted = clipboard
                .as_deref_mut()
                .and_then(DebugClipboard::get_debug_clipboard_text)
                .map(|text| sanitize_sign_clipboard_text(&text));
            if let (Some(editor), Some(pasted)) = (&mut input.sign_editor, pasted) {
                insert_sign_line_text(editor, &pasted, text_measurer);
            }
        }
        KeyCode::ArrowUp => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_line(editor, (editor.line + 3) % editor.lines.len());
            }
        }
        KeyCode::ArrowDown | KeyCode::Enter | KeyCode::NumpadEnter => {
            if let Some(editor) = &mut input.sign_editor {
                set_sign_editor_line(editor, (editor.line + 1) % editor.lines.len());
            }
        }
        KeyCode::ArrowLeft => {
            if let Some(editor) = &mut input.sign_editor {
                let cursor = if control_down {
                    text_edit::word_position(&editor.lines[editor.line], editor.cursor, -1)
                } else {
                    editor.cursor.saturating_sub(1)
                };
                move_sign_editor_cursor(editor, cursor, shift_down);
            }
        }
        KeyCode::ArrowRight => {
            if let Some(editor) = &mut input.sign_editor {
                let cursor = if control_down {
                    text_edit::word_position(&editor.lines[editor.line], editor.cursor, 1)
                } else {
                    (editor.cursor + 1).min(sign_line_char_len(&editor.lines[editor.line]))
                };
                move_sign_editor_cursor(editor, cursor, shift_down);
            }
        }
        KeyCode::Home => {
            if let Some(editor) = &mut input.sign_editor {
                move_sign_editor_cursor(editor, 0, shift_down);
            }
        }
        KeyCode::End => {
            if let Some(editor) = &mut input.sign_editor {
                move_sign_editor_cursor(
                    editor,
                    sign_line_char_len(&editor.lines[editor.line]),
                    shift_down,
                );
            }
        }
        KeyCode::Backspace => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                if delete_sign_selection(editor) {
                    return true;
                } else if control_down {
                    text_edit::remove_word_before_cursor(
                        &mut editor.lines[editor.line],
                        &mut editor.cursor,
                    );
                    editor.selection = editor.cursor;
                } else {
                    remove_sign_char_before_cursor(
                        &mut editor.lines[editor.line],
                        &mut editor.cursor,
                    );
                    editor.selection = editor.cursor;
                }
            }
        }
        KeyCode::Delete => {
            let control_down = input.control_down();
            if let Some(editor) = &mut input.sign_editor {
                if delete_sign_selection(editor) {
                    return true;
                } else if control_down {
                    text_edit::remove_word_at_cursor(&mut editor.lines[editor.line], editor.cursor);
                    editor.selection = editor.cursor;
                } else {
                    remove_sign_char_at_cursor(&mut editor.lines[editor.line], editor.cursor);
                    editor.selection = editor.cursor;
                }
            }
        }
        _ => {}
    }
    true
}

fn sync_sign_editor_input(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> bool {
    let Some(signature) = sign_editor_signature_from_world(world) else {
        let Some(editor) = input.sign_editor.take() else {
            return false;
        };
        queue_sign_editor_removed(counters, net_commands, &editor);
        input.sign_editor_ime_preedit = None;
        return true;
    };
    if input.dialog_suspends_sign_editor(&signature) {
        input.sign_editor = None;
        return false;
    }
    if input
        .sign_editor
        .as_ref()
        .is_some_and(|editor| editor.signature == signature)
    {
        return false;
    }
    if input.dismissed_sign_editor.as_ref() == Some(&signature) {
        input.sign_editor = None;
        return false;
    }

    if let Some(editor) = input.sign_editor.take() {
        queue_sign_editor_removed(counters, net_commands, &editor);
    }
    input.sign_editor_ime_preedit = None;
    let editor = sign_editor_input_state_from_world(input, world, signature);
    input.sign_editor = Some(editor);
    release_active_input(input, world, counters, net_commands);
    true
}

fn sign_editor_input_state_from_world(
    input: &ClientInputState,
    world: &WorldStore,
    signature: SignEditorInputSignature,
) -> SignEditorInputState {
    let lines = sign_editor_initial_lines(input, world);
    let cursor = sign_line_char_len(&lines[0]);
    let max_line_width = sign_editor_max_line_width(world, signature.pos);
    SignEditorInputState {
        signature,
        lines,
        line: 0,
        cursor,
        selection: cursor,
        max_line_width,
    }
}

fn sign_editor_signature_from_world(world: &WorldStore) -> Option<SignEditorInputSignature> {
    let editor = world.last_open_sign_editor()?;
    let open_count = world.counters().open_sign_editor_updates_applied;
    if open_count == 0 {
        return None;
    }
    Some(SignEditorInputSignature {
        open_count,
        pos: protocol_block_pos_from_world(editor.pos),
        is_front_text: editor.is_front_text,
    })
}

fn sign_editor_initial_lines(input: &ClientInputState, world: &WorldStore) -> [String; 4] {
    world
        .last_open_sign_editor()
        .map(|editor| {
            editor
                .lines_for_filter(input.client_text_filtering_enabled())
                .clone()
        })
        .unwrap_or_else(|| std::array::from_fn(|_| String::new()))
}

fn sign_editor_max_line_width(world: &WorldStore, pos: ProtocolBlockPos) -> u32 {
    let hanging = world
        .probe_block(world_block_pos_from_protocol(pos))
        .and_then(|probe| probe.block_name)
        .and_then(|name| sign_wood_and_form_for_block_name(&name))
        .is_some_and(|(_, form)| {
            matches!(
                form,
                SignBlockForm::HangingCeiling | SignBlockForm::HangingWall
            )
        });
    if hanging {
        HANGING_SIGN_MAX_LINE_WIDTH
    } else {
        STANDING_SIGN_MAX_LINE_WIDTH
    }
}

fn sign_editor_hud_state_from_editor(
    editor: &SignEditorInputState,
    preedit: Option<SignEditorImePreeditHudState>,
) -> SignEditorHudState {
    SignEditorHudState {
        pos: world_block_pos_from_protocol(editor.signature.pos),
        is_front_text: editor.signature.is_front_text,
        lines: editor.lines.clone(),
        line: editor.line,
        cursor: editor.cursor,
        selection: editor.selection,
        preedit,
    }
}

fn world_block_pos_from_protocol(pos: ProtocolBlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn submit_sign_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(editor) = input.sign_editor.take() else {
        return;
    };
    input.sign_editor_ime_preedit = None;
    input.dismissed_sign_editor = Some(editor.signature.clone());
    queue_sign_editor_removed(counters, net_commands, &editor);
}

fn queue_sign_editor_removed(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    editor: &SignEditorInputState,
) {
    queue_sign_update_command(
        counters,
        net_commands,
        SignUpdate {
            pos: editor.signature.pos,
            is_front_text: editor.signature.is_front_text,
            lines: editor.lines.clone(),
        },
    );
}

fn set_sign_editor_line(editor: &mut SignEditorInputState, line: usize) {
    editor.line = line;
    move_sign_editor_cursor(
        editor,
        sign_line_char_len(&editor.lines[editor.line]),
        false,
    );
}

fn insert_sign_line_text(
    editor: &mut SignEditorInputState,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) {
    let inserted = text
        .chars()
        .filter(|ch| is_sign_text_char(*ch))
        .collect::<String>();
    let current = &editor.lines[editor.line];
    let start = editor
        .selection
        .min(editor.cursor)
        .min(sign_line_char_len(current));
    let end = editor
        .selection
        .max(editor.cursor)
        .min(sign_line_char_len(current));
    let start_byte = sign_line_byte_index(current, start);
    let end_byte = sign_line_byte_index(current, end);
    let mut candidate = String::with_capacity(
        current
            .len()
            .saturating_sub(end_byte.saturating_sub(start_byte))
            .saturating_add(inserted.len()),
    );
    candidate.push_str(&current[..start_byte]);
    candidate.push_str(&inserted);
    candidate.push_str(&current[end_byte..]);
    if text_measurer.command_suggestion_text_width(&candidate) > editor.max_line_width {
        return;
    }
    editor.lines[editor.line] = candidate;
    editor.cursor = start.saturating_add(inserted.chars().count());
    editor.selection = editor.cursor;
}

fn move_sign_editor_cursor(editor: &mut SignEditorInputState, cursor: usize, selecting: bool) {
    editor.cursor = cursor.min(sign_line_char_len(&editor.lines[editor.line]));
    if !selecting {
        editor.selection = editor.cursor;
    }
}

fn select_sign_editor_line(editor: &mut SignEditorInputState) {
    editor.selection = 0;
    editor.cursor = sign_line_char_len(&editor.lines[editor.line]);
}

fn selected_sign_line_text(editor: &SignEditorInputState) -> String {
    let start = editor.selection.min(editor.cursor);
    let end = editor.selection.max(editor.cursor);
    let line = &editor.lines[editor.line];
    line[sign_line_byte_index(line, start)..sign_line_byte_index(line, end)].to_string()
}

fn sanitize_sign_clipboard_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            continue;
        }
        if ch == '\u{a7}' {
            let formatting = chars.peek().copied().is_some_and(|code| {
                let code = code.to_ascii_lowercase();
                VANILLA_CHAT_FORMATTINGS
                    .iter()
                    .any(|entry| entry.legacy_code == code)
            });
            if formatting {
                chars.next();
                continue;
            }
        }
        sanitized.push(ch);
    }
    sanitized
}

fn delete_sign_selection(editor: &mut SignEditorInputState) -> bool {
    if editor.selection == editor.cursor {
        return false;
    }
    let start = editor.selection.min(editor.cursor);
    let end = editor.selection.max(editor.cursor);
    let line = &mut editor.lines[editor.line];
    let start_byte = sign_line_byte_index(line, start);
    let end_byte = sign_line_byte_index(line, end);
    line.replace_range(start_byte..end_byte, "");
    editor.cursor = start;
    editor.selection = start;
    true
}

fn is_sign_text_char(ch: char) -> bool {
    ch != '\u{a7}' && ch >= ' ' && ch != '\u{7f}'
}

fn sign_line_char_len(text: &str) -> usize {
    text.chars().count()
}

fn sign_line_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn remove_sign_char_before_cursor(current: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = sign_line_byte_index(current, *cursor - 1);
    let end = sign_line_byte_index(current, *cursor);
    current.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_sign_char_at_cursor(current: &mut String, cursor: usize) {
    if cursor >= sign_line_char_len(current) {
        return;
    }
    let start = sign_line_byte_index(current, cursor);
    let end = sign_line_byte_index(current, cursor + 1);
    current.replace_range(start..end, "");
}

fn player_input_from_state(input: &ClientInputState) -> PlayerInput {
    PlayerInput {
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        shift: input.sneak,
        sprint: input.sprint,
    }
}

fn local_player_input_from_state(input: &ClientInputState) -> LocalPlayerInputState {
    LocalPlayerInputState {
        focused: input.focused,
        forward: input.forward,
        backward: input.backward,
        left: input.left,
        right: input.right,
        jump: input.jump,
        sneak: input.sneak,
        sprint: input.sprint,
        mouse_delta_x: input.mouse_delta_x,
        mouse_delta_y: input.mouse_delta_y,
    }
}

fn effective_sprinting_from_state(world: &WorldStore, input: &ClientInputState) -> bool {
    world.local_player_effective_sprint(local_player_input_from_state(input))
}

#[cfg(test)]
mod tests;
