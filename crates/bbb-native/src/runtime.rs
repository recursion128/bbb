use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{AudioCounters, NetCounters, RendererCounters};
use bbb_item_model::{
    ItemModelKeybindContext, NativeItemMapTooltipData, NativeItemRuntime, NativeItemTooltipOptions,
    ResolvedItemModel, ResolvedItemModelLayer,
};
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    heightmap_types::{
        vanilla_heightmap_type, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
    },
    packets::{
        AdvancementFrameType, CommandBlockMode, GameType, ItemCostSummary, ItemStackSummary,
        JigsawJoint, MapPostProcessingSummary, RecipeBookType, RemoteDebugSampleType,
        StructureBlockMirror, StructureBlockMode, StructureBlockRotation, TestBlockMode,
        TestInstanceBlockRotation, Vec3d,
    },
    particle_types::{
        VANILLA_PARTICLE_TYPE_CRIT_ID, VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        VANILLA_PARTICLE_TYPE_SMOKE_ID,
    },
    ComponentStyle, RichTextComponent, StyledTextRun, MC_VERSION,
};
use bbb_renderer::{
    hud_air_supply_pop, hud_air_supply_visible, hud_command_block_editor_layout,
    hud_jigsaw_block_editor_layout, hud_minecart_command_block_editor_layout,
    hud_structure_block_editor_layout, hud_test_block_editor_layout,
    hud_test_instance_block_editor_layout, sign_text_base_color, BlockDestroyOverlay,
    BlockEntityCrumbling, CameraPose, ClearColor, CloudEnvironment, CloudFrame, ConduitModelPart,
    DebugCuboidFace, DebugFilledBox, DebugFilledRect, DebugLine, DebugTextBillboard,
    EntityModelInstance, EntityModelKind, FogEnvironment, GuiItemLightingEntry,
    HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite, HudAdvancementLineTexture,
    HudAdvancementTabSprite, HudAdvancementWidgetFrameSprite, HudAirSupply, HudBlockItemModel,
    HudChatContext, HudChatMessage, HudChatRestrictionsScreen, HudChatTag, HudChatTagKind,
    HudCommandBlockEditorKind, HudCommandBlockEditorScreen, HudCommandSuggestionEntry,
    HudCommandSuggestionTooltip, HudCommandSuggestions, HudCommandSuggestionsLayout,
    HudComponentTextRun, HudContextualBar, HudCrosshairAttackIndicator, HudDebugCrosshair,
    HudDebugFrameTimeChart, HudDebugGameModeSwitcher, HudDebugGameModeSwitcherSlot,
    HudDebugNetworkCharts, HudDebugOptionsTooltip, HudDebugOverlay, HudDebugProfilerChart,
    HudDebugTpsChart, HudDebugTpsSample, HudDemoLinkConfirmationScreen, HudDemoPopupScreen,
    HudEntityPreview, HudEntityPreviewItemDisplayContext, HudEntityPreviewItemLayer,
    HudEntityPreviewItemSlot, HudEntityPreviewRect, HudFontGlyphBook, HudFoodEffect,
    HudGameEventButton, HudGameEventScreen, HudGameModeSwitcherMode, HudHeartKind, HudHotbarMode,
    HudIconLayer, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryFillLayer, HudInventoryFillStage, HudInventoryGhostItem, HudInventoryItem,
    HudInventoryItemScissor, HudInventoryMapPreview, HudInventoryScreen, HudInventoryScrollingText,
    HudInventorySlot, HudInventoryTextBackground, HudInventoryTextInputDecoration,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine, HudItemCountLabel,
    HudItemDurabilityBar, HudItemFoil, HudItemIcon, HudJigsawBlockEditorScreen,
    HudJigsawBlockEditorTextField, HudJumpBar, HudMobEffect, HudPauseScreen, HudPauseScreenTooltip,
    HudPlayerHealth, HudPlayerList, HudPlayerReportingFocus, HudResidentItemLayer,
    HudScoreboardSidebar, HudSignEditorKind, HudSignEditorPreedit, HudSignEditorScreen,
    HudSocialInteractionsFocus, HudSocialInteractionsPage, HudSocialInteractionsPlayerAction,
    HudSocialInteractionsScreen, HudSpyglassScope, HudStatsItemRow, HudStatsRow, HudStatsScreen,
    HudStructureBlockEditorScreen, HudStructureBlockEditorTextField, HudStructureBlockMode,
    HudStyledTextRun, HudSystemToast, HudTestBlockEditorScreen, HudTestBlockEditorTextField,
    HudTestInstanceBlockEditorScreen, HudTestInstanceBlockEditorTextField, HudTextStyle, HudUvRect,
    HudVehicleHealth, HudWinScreen, HudWinScreenLine, ItemModelAssetIdentity, ItemModelAssetSource,
    ItemModelAtlasKind, ItemModelGeometryClass, ItemModelQuad, LevelLighting, LightmapEnvironment,
    LightningBoltRenderState, ParticleBlockFluidSurfaceSample, ParticleEntityTargetContext,
    ParticleFluidKind, ParticleLocalPlayerScopeContext, ParticlePlayerMotionContext,
    ParticleSoundEvent, ParticleSpawnBatch, ParticleSpawnCommand, Renderer, SelectionColoredBox,
    SelectionLine, SelectionOutline, SignModelAttachment, SignModelWood, SkyEnvironment,
    SkyMoonPhase, WeatherColumn, WeatherFrame, WeatherRenderState, DEFAULT_ARMOR_STAND_MODEL_POSE,
    ENTITY_FULL_BRIGHT_LIGHT_COORDS, HUD_HOTBAR_SLOTS, ITEM_MODEL_NO_OVERLAY,
    VANILLA_DEFAULT_CLOUD_COLOR, VANILLA_DEFAULT_CLOUD_HEIGHT,
    VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR, VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR, VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    VANILLA_MAX_RENDER_DISTANCE_CHUNKS, VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use bbb_world::{
    java_string_hash_code, sign_wood_and_form_for_block_name, vanilla_entity_type_is_living,
    BlockPos, CartographyAdditionalItemKind, ChunkPos, ClientEnvironmentAttributes, ContainerState,
    EnchantmentTooltipDefinition, EntityClientMovementEffectState, EntityClientTickWork,
    EnvironmentBiomeSampling, EvokerFangsCritParticleState, ItemEquipmentSlot,
    LocalPlayerCrosshairAttackIndicatorState, LocalSoundEventState, MerchantOfferState,
    MerchantOffersState, MobEffectState, MountArmorSlotKind, MountInventoryKind,
    OminousItemSpawnerParticleState, PrimedTntSmokeParticleState, RavagerStunParticleState,
    SignBlockForm, SoundEventState, SoundHolderState, TerrainFluidKind, TerrainFluidState,
    TerrainLight, TerrainMaterialClass, VanillaEnchantmentIdentity, WorldLevelInfo, WorldSkybox,
    WorldStore, WorldWeatherState,
};
use glam::{Quat, Vec3};
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    banner_scene::banner_model_instances_from_world_at_partial_tick,
    bed_scene::bed_model_instances_from_world,
    bell_scene::bell_model_instances_from_world_at_partial_tick,
    biome_tint::biome_height_adjusted_temperature,
    block_entity_visibility::{retain_visible_block_entity_instances, BlockEntityVisibility},
    camera_fov::{camera_fluid_in_camera, CameraFluid, CameraFovTickState},
    camera_pose::{
        camera_eye_position, camera_forward_vector, camera_hurt_effect_from_world,
        camera_pose_from_world, camera_pose_from_world_at_partial_tick,
        camera_view_bob_effect_from_world, current_post_effect_id_from_world,
        render_camera_pose_from_world_at_partial_tick, CameraEyeHeightTickState,
    },
    cartography_preview::hud_map_preview_models,
    chest_scene::chest_model_instances_from_world_at_partial_tick,
    code_of_conduct::CodeOfConductAcceptance,
    component_text::{localized_component_runs, visit_localized_template, LocalizedTemplatePart},
    conduit_scene::conduit_model_instances_from_world_at_partial_tick,
    crosshair::{
        crosshair_target_from_camera_at_partial_tick, debug_looking_at_block_hit_from_camera,
        debug_looking_at_fluid_hit_from_camera, debug_targeted_entity_from_camera_at_partial_tick,
        selection_outline_from_camera, CrosshairTarget,
    },
    debug_entries::DebugScreenEntryId,
    decorated_pot_scene::decorated_pot_model_instances_from_world_at_partial_tick,
    enchanting_table_book_scene::enchanting_table_book_model_instances_from_world_at_partial_tick,
    end_portal_scene::end_portal_model_instances_from_world_at_partial_tick,
    entity_features::{entity_flame_surfaces, entity_shadow_surfaces},
    entity_name_tags::entity_name_tag_surfaces,
    entity_scene::{
        entity_model_instance_from_world_entity_at_partial_tick,
        entity_scene_outline_from_world_at_partial_tick,
        entity_scene_outline_from_world_at_partial_tick_with_server_details,
        visible_display_entity_render_sources_at_partial_tick,
        visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick,
        visible_entity_name_tag_render_sources, visible_entity_shadow_render_sources,
        EntityRenderVisibilityContext,
    },
    entity_variant_textures::NativeEntityVariantTextureRuntime,
    game_test_highlights::game_test_highlight_gizmos,
    input::{
        advance_creative_inventory_commands, advance_destroying_block_at_partial_tick,
        advance_local_using_item_ticks_with_effects, advance_player_input_for_client_ticks,
        advance_using_item_at_partial_tick_with_effects_and_presentation,
        death_screen::DeathScreenRuntime,
        enchantment_screen::{
            enchantment_option_hud_states, EnchantmentBookHudState, EnchantmentOptionHudState,
            ENCHANTING_TABLE_OPTION_HEIGHT, ENCHANTING_TABLE_OPTION_SPACING,
            ENCHANTING_TABLE_OPTION_TEXT_WIDTH, ENCHANTING_TABLE_OPTION_WIDTH,
            ENCHANTING_TABLE_OPTION_X, ENCHANTING_TABLE_OPTION_Y, ENCHANTMENT_ALT_FONT_ID,
            ENCHANTMENT_OPTION_COUNT,
        },
        game_event_screen::{DemoHelpFocus, DemoLinkConfirmationFocus, GameEventScreenRuntime},
        inventory_screen_layout_for_surface, inventory_screen_selected_hotbar_slot_id,
        local_player_hands_are_busy, native_pause_freezes_client_ticks,
        pause_screen_advancements_button_contains, pause_screen_disconnect_button_contains,
        pause_screen_feedback_back_button_contains,
        pause_screen_feedback_report_bugs_button_contains,
        pause_screen_feedback_send_feedback_button_contains, pause_screen_options_button_contains,
        pause_screen_options_button_enabled, pause_screen_player_reporting_button_contains,
        pause_screen_player_reporting_button_enabled, pause_screen_report_bugs_button_contains,
        pause_screen_report_bugs_button_enabled, pause_screen_return_to_game_button_contains,
        pause_screen_send_feedback_button_contains, pause_screen_stats_button_contains,
        recipe_book_button_position, recipe_book_main_gui_offset,
        recipe_book_tab_count_for_background, recipe_book_type_for_background,
        recipe_book_type_settings, recipe_book_visible_tab_indices, release_active_input,
        stats_screen_done_button_contains, sync_beacon_effect_selection_state,
        sync_loom_pattern_state_for_hud, sync_recipe_book_presentation,
        sync_stonecutter_recipe_scroll_state, ClientInputState, CommandSuggestionHudTooltip,
        CommandSuggestionNarrationState, CreativeInventoryHudState, DialogImePreeditHudState,
        InventoryScreenBackground, PauseScreenPage, RecipeBookOverlayHudState,
        RecipeBookPageHudState, RecipeBookPresentationHudState, RecipeBookSearchHudState,
        RecipeBookTabSelectionHudState, StatsScreenFocus, RECIPE_BOOK_BUTTON_HEIGHT,
        RECIPE_BOOK_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_HEIGHT,
        RECIPE_BOOK_FILTER_BUTTON_WIDTH, RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y,
        RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X, RECIPE_BOOK_PAGE_BUTTON_HEIGHT,
        RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y,
        RECIPE_BOOK_PAGE_FORWARD_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_COLUMNS,
        RECIPE_BOOK_RECIPE_BUTTON_SIZE, RECIPE_BOOK_RECIPE_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_Y,
        RECIPE_BOOK_SEARCH_BOX_HEIGHT, RECIPE_BOOK_SEARCH_BOX_WIDTH, RECIPE_BOOK_SEARCH_BOX_X,
        RECIPE_BOOK_SEARCH_BOX_Y, RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
        RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET, RECIPE_BOOK_SELECTED_TAB_X_OFFSET,
        RECIPE_BOOK_TAB_HEIGHT, RECIPE_BOOK_TAB_STRIDE_Y, RECIPE_BOOK_TAB_WIDTH, RECIPE_BOOK_TAB_X,
        RECIPE_BOOK_TAB_Y, STONECUTTER_RECIPE_BUTTON_COLUMNS, STONECUTTER_RECIPE_BUTTON_HEIGHT,
        STONECUTTER_RECIPE_BUTTON_ROWS, STONECUTTER_RECIPE_BUTTON_SPRITE_Y,
        STONECUTTER_RECIPE_BUTTON_WIDTH, STONECUTTER_RECIPE_BUTTON_X, STONECUTTER_RECIPE_ITEM_Y,
        STONECUTTER_SCROLLER_HEIGHT, STONECUTTER_SCROLLER_MAX_OFFSET, STONECUTTER_SCROLLER_WIDTH,
        STONECUTTER_SCROLLER_X, STONECUTTER_SCROLLER_Y, STONECUTTER_SELECTED_RECIPE_DATA_ID,
        STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT,
    },
    item_entities::item_entity_billboards_from_world,
    item_frames::item_frame_models_with_map_cache,
    item_models::{
        display_entity_item_models, dropped_item_models, entity_block_models,
        first_person_item_models_with_map_cache, first_person_player_arms,
        held_item_models_with_variant_textures, item_pickup_particle_item_models,
        ominous_item_spawner_models, smithing_entity_preview_item_models, world_attribute_keys,
        world_banner_pattern_tooltip_context, world_damage_type_keys, world_enchantment_keys,
        world_instrument_tooltip_context, world_jukebox_song_tooltip_context,
        world_painting_variant_tooltip_context, world_trim_material_keys,
        world_trim_material_tooltip_context, world_trim_pattern_keys,
        world_trim_pattern_tooltip_context, FirstPersonHandPresentation,
    },
    lectern_book_scene::lectern_book_model_instances_from_world,
    level_load_runtime::LevelLoadRuntime,
    music_runtime::MusicRuntime,
    narrator_runtime::{NarratedElementKind, ScreenNarrationFrame, SystemNarrator},
    particle_runtime::{LocalPlayerActionParticleState, NativeParticleRuntime, ParticleEventSink},
    recipe_book_ui::{
        clamped_recipe_book_page, crafting_recipe_book_collections,
        furnace_recipe_book_collections, recipe_book_overlay_entry_position,
        recipe_book_overlay_max_row, recipe_book_overlay_rows, recipe_book_page_count,
        recipe_book_slot_select_index, RecipeBookCraftingGrid, RecipeBookFurnaceFamily,
        RecipeBookOverlayEntry, RecipeBookUiCollection, RECIPE_BOOK_ITEMS_PER_PAGE,
        RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER, RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE,
        RECIPE_BOOK_OVERLAY_BUTTON_SIZE, RECIPE_BOOK_OVERLAY_ITEM_SCALE,
    },
    remote_debug::remote_debug_gizmos,
    resource_pack_runtime::ServerResourcePackRuntime,
    shulker_box_scene::shulker_box_model_instances_from_world_at_partial_tick,
    sign_scene::{
        sign_model_attachment, sign_model_wood, sign_scene_from_world, sign_text_outline_context,
    },
    simulation::{
        effects::ParticleBatchSink, publication::SimulationPublication, SimulationRenderContext,
    },
    skull_scene::skull_model_instances_from_world_at_partial_tick,
    spawner_scene::spawner_display_entity_submissions_from_world_at_partial_tick,
    startup::VANILLA_UNLIMITED_FRAMERATE_LIMIT,
    terrain_runtime::{
        maybe_upload_decoded_terrain, maybe_upload_terrain_texture_animation, BlockRenderPosition,
        TerrainTextureState, TerrainVisibilityContext,
    },
};
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
};
use bbb_protocol::entity_types::*;
use bbb_protocol::item_types::{
    vanilla_item_type, VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_EMERALD_ID,
    VANILLA_ITEM_GOLD_INGOT_ID, VANILLA_ITEM_IRON_INGOT_ID, VANILLA_ITEM_NETHERITE_INGOT_ID,
};
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type, MobEffectCategory, VANILLA_BEACON_EFFECT_TIERS,
    VANILLA_MOB_EFFECT_BLINDNESS_ID, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
    VANILLA_MOB_EFFECT_DARKNESS_ID, VANILLA_MOB_EFFECT_FIRE_RESISTANCE_ID,
    VANILLA_MOB_EFFECT_HUNGER_ID, VANILLA_MOB_EFFECT_NAUSEA_ID, VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
    VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_REGENERATION_ID, VANILLA_MOB_EFFECT_WITHER_ID,
};

#[cfg(test)]
use bbb_protocol::mob_effect_types::{VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_STRENGTH_ID};

#[cfg(test)]
use crate::terrain_runtime::TerrainUploadState;

mod book_screen;
mod component_hover;
mod toast;
pub(crate) use book_screen::hud_book_component_click_event;
mod control_requests;
mod creative_inventory;
mod debug_light_sections;
mod debug_options_screen;
mod dialog_screen;
mod game_rules_screen;
mod loom_screen;
mod overworld_day_timeline;
mod periodic_notifications;
mod render_extract;
mod smithing_screen;
mod vignette;
pub(crate) use render_extract::*;
mod events;
mod waypoint_locator;
#[cfg(test)]
pub(crate) use events::minimal_non_empty_synchronized_registry_data;
pub(crate) use events::queue_chat_acknowledgement;

use book_screen::hud_book_screen;
use creative_inventory::{creative_inventory_narration_frame, hud_creative_inventory_screen};
pub(crate) use debug_light_sections::DebugSkyLightSectionCache;
use debug_options_screen::{debug_options_screen_narration_frame, hud_debug_options_screen};
pub(crate) use dialog_screen::{
    dialog_ime_cursor_area, handle_dialog_cursor_moved_with_measurer,
    handle_dialog_key_input_with_modifiers_for_surface_and_measurer,
    handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
    handle_dialog_text_input_with_measurer, DialogTextMeasurer,
};
use dialog_screen::{
    hud_dialog_confirm_link_screen, hud_dialog_screen_with_measurer, hud_dialog_warning_screen,
    hud_waiting_response_screen,
};
use game_rules_screen::hud_game_rules_screen;
use overworld_day_timeline::*;
use waypoint_locator::hud_locator_bar_from_world;

const VANILLA_CLIENT_TICK_INTERVAL: Duration = Duration::from_millis(50);
const VANILLA_MAX_CLIENT_TICKS_PER_STEP: u32 = 10;
const VANILLA_OVERWORLD_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-16_119_286);
const VANILLA_NETHER_SKY_LIGHT_COLOR: [f32; 3] = [122.0 / 255.0, 122.0 / 255.0, 1.0];
const VANILLA_NETHER_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-13_621_215);
const VANILLA_END_SKY_LIGHT_COLOR: [f32; 3] = rgb24(-5_480_243);
const VANILLA_END_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-12_630_209);
const VANILLA_LIGHTMAP_DEFAULT_DAY_TIME: i64 = 6_000;
const VANILLA_WEATHER_SKY_LIGHT_FACTOR: f32 = 0.24;
const VANILLA_WEATHER_RAIN_ALPHA: f32 = 0.3125;
const VANILLA_WEATHER_THUNDER_ALPHA: f32 = 0.52734375;
const VANILLA_WEATHER_RAIN_SKY_LIGHT_COLOR: i32 = argb_color(79, 122, 122, 255);
const VANILLA_WEATHER_THUNDER_SKY_LIGHT_COLOR: i32 = argb_color(134, 122, 122, 255);
const VANILLA_WEATHER_RENDER_RADIUS: u32 = 10;
const VANILLA_WEATHER_SNOW_TEMPERATURE_THRESHOLD: f32 = 0.15;
const VANILLA_OVERWORLD_SKY_COLOR: [u8; 3] = [0x78, 0xa7, 0xff];
const VANILLA_OVERWORLD_FOG_COLOR: [u8; 3] = [0xc0, 0xd8, 0xff];
const VANILLA_END_FOG_COLOR: [u8; 3] = [0x18, 0x13, 0x18];
const VANILLA_DEFAULT_WATER_FOG_COLOR: [u8; 3] = [0x05, 0x05, 0x33];
const VANILLA_LAVA_FOG_COLOR: i32 = argb_color(255, 0x99, 0x19, 0x00);
const VANILLA_WEATHER_RAIN_FOG_COLOR_MULTIPLIER: i32 = argb_color(255, 127, 127, 153);
const VANILLA_WEATHER_THUNDER_FOG_COLOR_MULTIPLIER: i32 = argb_color(255, 63, 63, 76);
const VANILLA_WEATHER_RAIN_CLOUD_GRAY_BRIGHTNESS: f32 = 0.24;
const VANILLA_WEATHER_RAIN_CLOUD_GRAY_FACTOR: f32 = 0.5;
const VANILLA_WEATHER_THUNDER_CLOUD_GRAY_BRIGHTNESS: f32 = 0.095;
const VANILLA_WEATHER_THUNDER_CLOUD_GRAY_FACTOR: f32 = 0.94;
#[cfg(test)]
const VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS: f32 = 12.0;
const VANILLA_SUNRISE_SUNSET_MIN_RENDER_DISTANCE_CHUNKS: u32 = 4;
const VANILLA_DEFAULT_FOG_START_DISTANCE: f32 = 0.0;
const VANILLA_DEFAULT_FOG_END_DISTANCE: f32 = 1024.0;
const VANILLA_NETHER_FOG_START_DISTANCE: f32 = 10.0;
const VANILLA_NETHER_FOG_END_DISTANCE: f32 = 96.0;
const VANILLA_DEFAULT_SKY_FOG_END_DISTANCE: f32 = 512.0;
const VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE: f32 = 2048.0;
const VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS: f32 = 128.0;
const VANILLA_DEFAULT_WATER_FOG_START_DISTANCE: f32 = -8.0;
const VANILLA_DEFAULT_WATER_FOG_END_DISTANCE: f32 = 96.0;
const VANILLA_RAIN_FOG_MIN_SKY_LIGHT: f32 = 8.0;
const VANILLA_RAIN_FOG_SKY_LIGHT_RANGE: f32 = 7.0;
const VANILLA_RAIN_FOG_START_OFFSET: f32 = -160.0;
const VANILLA_RAIN_FOG_END_OFFSET: f32 = -256.0;
const VANILLA_RAIN_FOG_SMOOTHING_PER_TICK: f32 = 0.2;
const VANILLA_SKY_FLASH_SKY_COLOR: i32 = argb_color(255, 204, 204, 255);
const VANILLA_SKY_FLASH_SKY_COLOR_ALPHA: f32 = 0.22;
const VANILLA_PARTICLE_MISSING_CHUNK_LIGHT: TerrainLight = TerrainLight { sky: 15, block: 15 };
const VANILLA_END_FLASH_INTERVAL_TICKS: i64 = 600;
const VANILLA_END_FLASH_MAX_OFFSET_TICKS: i32 = 200;
const VANILLA_END_FLASH_MIN_DURATION_TICKS: i32 = 100;
const VANILLA_END_FLASH_MAX_DURATION_TICKS: i32 = 380;
const VANILLA_BOSS_OVERLAY_DARKEN_IN_STEP: f32 = 0.05;
const VANILLA_BOSS_OVERLAY_DARKEN_OUT_STEP: f32 = 0.0125;
const VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK: f32 = 1.0;
const VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS: i32 = 200;
const VANILLA_DARKNESS_EFFECT_SCALE_OPTION: f32 = 1.0;
const VANILLA_WATER_VISION_MAX_TICKS: i32 = 600;
const VANILLA_WATER_VISION_FAST_RAMP_TICKS: f32 = 100.0;
const VANILLA_WATER_VISION_SLOW_RAMP_TICKS: f32 = 500.0;
const VANILLA_WATER_VISION_FAST_WEIGHT: f32 = 0.6;
const VANILLA_WATER_VISION_SLOW_WEIGHT: f32 = 0.39999998;
const CRAFTER_GRID_SLOT_COUNT: i16 = 9;
const CRAFTER_POWERED_DATA_ID: i16 = 9;
const CRAFTER_DISABLED_SLOT_SPRITE_SIZE: u32 = 18;
const CRAFTER_REDSTONE_SPRITE_SIZE: u32 = 16;
const BEACON_LEVELS_DATA_ID: i16 = 0;
const BEACON_PRIMARY_EFFECT_DATA_ID: i16 = 1;
const BEACON_SECONDARY_EFFECT_DATA_ID: i16 = 2;
const BEACON_EFFECT_BUTTON_SIZE: u32 = 22;
const BEACON_EFFECT_BUTTON_SPACING: i32 = 24;
const BEACON_PRIMARY_EFFECT_CENTER_X: i32 = 76;
const BEACON_PRIMARY_EFFECT_Y: i32 = 22;
const BEACON_PRIMARY_EFFECT_ROW_SPACING: i32 = 25;
const BEACON_SECONDARY_EFFECT_CENTER_X: i32 = 167;
const BEACON_SECONDARY_EFFECT_Y: i32 = 47;
const BEACON_CONFIRM_BUTTON_X: i32 = 164;
const BEACON_CANCEL_BUTTON_X: i32 = 190;
const BEACON_ACTION_BUTTON_Y: i32 = 107;
const BEACON_ACTION_BUTTON_SIZE: u32 = 22;
const BEACON_ACTION_ICON_OFFSET: i32 = 2;
const BEACON_ACTION_ICON_SIZE: u32 = 18;
const BEACON_PRIMARY_LABEL_CENTER_X: i32 = 62;
const BEACON_SECONDARY_LABEL_CENTER_X: i32 = 169;
const BEACON_LABEL_Y: i32 = 10;
const BEACON_LABEL_TEXT_COLOR: [f32; 4] = rgba32(-2_039_584);
const BEACON_EXAMPLE_PAYMENT_ITEMS: [(i32, i32, i32); 5] = [
    (VANILLA_ITEM_NETHERITE_INGOT_ID, 20, 109),
    (VANILLA_ITEM_EMERALD_ID, 41, 109),
    (VANILLA_ITEM_DIAMOND_ID, 63, 109),
    (VANILLA_ITEM_GOLD_INGOT_ID, 86, 109),
    (VANILLA_ITEM_IRON_INGOT_ID, 108, 109),
];
const ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE: u32 = 16;
const ENCHANTING_TABLE_LEVEL_ICON_X_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_Y_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_SIZE: u32 = 16;
const ENCHANTING_TABLE_COST_TEXT_X_RIGHT: i32 = 166;
const ENCHANTING_TABLE_COST_TEXT_Y_OFFSET: i32 = 23;
const ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR: [f32; 4] = [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];
const ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR: [f32; 4] =
    [64.0 / 255.0, 127.0 / 255.0, 16.0 / 255.0, 1.0];
const ENCHANTING_TABLE_NAME_TEXT_COLOR: [f32; 4] = [104.0 / 255.0, 94.0 / 255.0, 74.0 / 255.0, 1.0];
const ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR: [f32; 4] =
    [52.0 / 255.0, 47.0 / 255.0, 37.0 / 255.0, 1.0];
const ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR: [f32; 4] = [1.0, 1.0, 128.0 / 255.0, 1.0];
const CONTAINER_LABEL_TEXT_COLOR: [f32; 4] = rgba32(-12_566_464);
const CONTAINER_LABEL_WIDTH: u32 = 512;
const CONTAINER_CENTERED_LABEL_WIDTH: u32 = 512;
const CONTAINER_TEXT_LINE_HEIGHT: u32 = 9;
const ADVANCEMENTS_WINDOW_WIDTH: u32 = 252;
const ADVANCEMENTS_WINDOW_HEIGHT: u32 = 140;
const ADVANCEMENTS_WINDOW_INSIDE_X: i32 = 9;
const ADVANCEMENTS_WINDOW_INSIDE_Y: i32 = 18;
const ADVANCEMENTS_WINDOW_INSIDE_WIDTH: u32 = 234;
const ADVANCEMENTS_WINDOW_INSIDE_HEIGHT: u32 = 113;
const ADVANCEMENTS_WINDOW_TITLE_X: i32 = 8;
const ADVANCEMENTS_WINDOW_TITLE_Y: i32 = 6;
const ADVANCEMENTS_EMPTY_TEXT_CENTER_X: i32 = 126;
const ADVANCEMENTS_EMPTY_TEXT_Y: i32 = 70;
const ADVANCEMENTS_SAD_TEXT_Y: i32 = 122;
const ADVANCEMENTS_FOOTER_HEIGHT: i32 = 33;
const ADVANCEMENTS_DONE_BUTTON_WIDTH: u32 = 200;
const ADVANCEMENTS_DONE_BUTTON_HEIGHT: u32 = 20;
const ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET: i32 = 6;
const ADVANCEMENTS_TITLE_TEXT: &str = "Advancements";
const ADVANCEMENTS_EMPTY_TEXT: &str = "There doesn't seem to be anything here...";
const ADVANCEMENTS_SAD_TEXT: &str = ":(";
const ADVANCEMENTS_DONE_TEXT: &str = "Done";
const ADVANCEMENTS_TITLE_TEXT_COLOR: [f32; 4] = rgba32(-12_566_464);
const ADVANCEMENTS_EMPTY_TEXT_COLOR: [f32; 4] = rgba32(-1);
const ADVANCEMENTS_DONE_TEXT_COLOR: [f32; 4] = rgba32(-1);
const ADVANCEMENTS_EMPTY_BACKGROUND_TINT: [f32; 4] = rgba32(-16_777_216);
const ADVANCEMENTS_BACKGROUND_TILE_SIZE: i32 = 16;
const ADVANCEMENTS_TAB_ABOVE_WIDTH: u32 = 28;
const ADVANCEMENTS_TAB_ABOVE_HEIGHT: u32 = 32;
const ADVANCEMENTS_TAB_ABOVE_MAX: usize = 8;
const ADVANCEMENTS_TAB_BELOW_WIDTH: u32 = 28;
const ADVANCEMENTS_TAB_BELOW_HEIGHT: u32 = 32;
const ADVANCEMENTS_TAB_BELOW_MAX: usize = 8;
const ADVANCEMENTS_TAB_LEFT_WIDTH: u32 = 32;
const ADVANCEMENTS_TAB_LEFT_HEIGHT: u32 = 28;
const ADVANCEMENTS_TAB_LEFT_MAX: usize = 5;
const ADVANCEMENTS_TAB_RIGHT_WIDTH: u32 = 32;
const ADVANCEMENTS_TAB_RIGHT_HEIGHT: u32 = 28;
const ADVANCEMENTS_TAB_RIGHT_MAX: usize = 5;
const ADVANCEMENTS_TAB_ABOVE_ICON_OFFSET: (i32, i32) = (6, 9);
const ADVANCEMENTS_TAB_BELOW_ICON_OFFSET: (i32, i32) = (6, 6);
const ADVANCEMENTS_TAB_LEFT_ICON_OFFSET: (i32, i32) = (10, 5);
const ADVANCEMENTS_TAB_RIGHT_ICON_OFFSET: (i32, i32) = (6, 5);
const ADVANCEMENTS_WIDGET_FRAME_OFFSET_X: i32 = 3;
const ADVANCEMENTS_WIDGET_FRAME_WIDTH: u32 = 26;
const ADVANCEMENTS_WIDGET_FRAME_HEIGHT: u32 = 26;
const ADVANCEMENTS_WIDGET_ICON_OFFSET: (i32, i32) = (8, 5);
const ADVANCEMENTS_WIDGET_ICON_SIZE: i32 = 16;
const ADVANCEMENTS_WIDGET_BOUNDS_WIDTH: i32 = 28;
const ADVANCEMENTS_WIDGET_BOUNDS_HEIGHT: i32 = 27;
const ADVANCEMENTS_HOVER_MAX_FADE: f32 = 0.3;
const ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH: u32 = 200;
const ADVANCEMENTS_HOVER_FRAME_SIZE: u32 = 26;
const ADVANCEMENTS_HOVER_TITLE_MIN_WIDTH: u32 = 80;
const ADVANCEMENTS_HOVER_TITLE_MAX_WIDTH: u32 = 163;
const ADVANCEMENTS_HOVER_TITLE_X: i32 = 32;
const ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT: u32 = 3;
const ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT: u32 = 5;
const ADVANCEMENTS_HOVER_TITLE_PADDING_TOP: i32 = 9;
const ADVANCEMENTS_HOVER_TITLE_PADDING_BOTTOM: i32 = 8;
const ADVANCEMENTS_HOVER_DESCRIPTION_PADDING_TOP: i32 = 6;
const ADVANCEMENTS_HOVER_LINE_HEIGHT: i32 = 9;
const ADVANCEMENTS_HOVER_PROGRESS_PADDING: u32 = 8;
const ADVANCEMENTS_HOVER_PROGRESS_PREFIX_WIDTH: u32 = 29;
const ADVANCEMENTS_HOVER_RIGHT_MARGIN: i32 = 6;
const ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR: [f32; 4] = rgba32(-1);
const ADVANCEMENTS_HOVER_TASK_DESCRIPTION_TEXT_COLOR: [f32; 4] =
    [85.0 / 255.0, 1.0, 85.0 / 255.0, 1.0];
const ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR: [f32; 4] =
    [170.0 / 255.0, 0.0, 170.0 / 255.0, 1.0];
const ADVANCEMENTS_HOVER_FADE_TINT: [f32; 4] = [0.0, 0.0, 0.0, ADVANCEMENTS_HOVER_MAX_FADE];
const RECIPE_BOOK_SEARCH_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const RECIPE_BOOK_SEARCH_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const RECIPE_BOOK_SEARCH_MAX_LENGTH: usize = 50;
const RECIPE_BOOK_RECIPE_ICON_OFFSET: i32 = 4;
const RECIPE_BOOK_PAGE_TEXT_CENTER_X: i32 = 73;
const RECIPE_BOOK_PAGE_TEXT_Y: i32 = 141;
const RECIPE_BOOK_GHOST_PRE_ITEM_TINT: [f32; 4] = [1.0, 0.0, 0.0, 48.0 / 255.0];
const RECIPE_BOOK_GHOST_POST_ITEM_TINT: [f32; 4] = [1.0, 1.0, 1.0, 48.0 / 255.0];
const ANVIL_COST_DATA_ID: i16 = 0;
const ANVIL_RESULT_SLOT: i16 = 2;
const ANVIL_TOO_EXPENSIVE_LEVEL_COST: i16 = 40;
const ANVIL_RENAME_TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ANVIL_COST_TEXT_COLOR: [f32; 4] = [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];
const ANVIL_COST_ERROR_TEXT_COLOR: [f32; 4] = [1.0, 96.0 / 255.0, 96.0 / 255.0, 1.0];
const ANVIL_COST_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 79.0 / 255.0];
const ANVIL_COST_LABEL_RIGHT: i32 = 168;
const ANVIL_COST_LABEL_Y: i32 = 69;
const ANVIL_COST_BACKGROUND_Y: i32 = 67;
const ANVIL_COST_BACKGROUND_HEIGHT: u32 = 12;
const CARTOGRAPHY_TABLE_MAP_SLOT: i16 = 0;
const CARTOGRAPHY_TABLE_ADDITIONAL_SLOT: i16 = 1;
const CARTOGRAPHY_TABLE_RESULT_SLOT: i16 = 2;
const CARTOGRAPHY_TABLE_ERROR_X: i32 = 35;
const CARTOGRAPHY_TABLE_ERROR_Y: i32 = 31;
const CARTOGRAPHY_TABLE_ERROR_WIDTH: u32 = 28;
const CARTOGRAPHY_TABLE_ERROR_HEIGHT: u32 = 21;
const CARTOGRAPHY_TABLE_MAP_X: i32 = 67;
const CARTOGRAPHY_TABLE_MAP_Y: i32 = 13;
const CARTOGRAPHY_TABLE_MAP_SIZE: u32 = 66;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH: u32 = 50;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT: u32 = 66;
const CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET: i32 = 16;
const CARTOGRAPHY_TABLE_LOCKED_X: i32 = 118;
const CARTOGRAPHY_TABLE_LOCKED_Y: i32 = 60;
const CARTOGRAPHY_TABLE_LOCKED_WIDTH: u32 = 10;
const CARTOGRAPHY_TABLE_LOCKED_HEIGHT: u32 = 14;
const SMITHING_RECIPE_ERROR_DATA_ID: i16 = 0;
const BREWING_STAND_BREW_TIME_DATA_ID: i16 = 0;
const BREWING_STAND_FUEL_DATA_ID: i16 = 1;
const BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH: u32 = 18;
const BREWING_STAND_FUEL_LENGTH_SPRITE_HEIGHT: u32 = 4;
const BREWING_STAND_BREW_PROGRESS_SPRITE_WIDTH: u32 = 9;
const BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT: u32 = 28;
const BREWING_STAND_BUBBLES_SPRITE_WIDTH: u32 = 12;
const BREWING_STAND_BUBBLES_SPRITE_HEIGHT: u32 = 29;
const BREWING_STAND_BREW_TOTAL_TICKS: f32 = 400.0;
const BREWING_STAND_BUBBLE_LENGTHS: [u32; 7] = [29, 24, 20, 16, 11, 6, 0];
const FURNACE_LIT_TIME_DATA_ID: i16 = 0;
const FURNACE_LIT_DURATION_DATA_ID: i16 = 1;
const FURNACE_COOKING_PROGRESS_DATA_ID: i16 = 2;
const FURNACE_COOKING_TOTAL_TIME_DATA_ID: i16 = 3;
const FURNACE_DEFAULT_LIT_DURATION: i16 = 200;
const FURNACE_LIT_PROGRESS_SPRITE_SIZE: u32 = 14;
const FURNACE_BURN_PROGRESS_SPRITE_WIDTH: u32 = 24;
const FURNACE_BURN_PROGRESS_SPRITE_HEIGHT: u32 = 16;
const ITEM_DURABILITY_BAR_MAX_WIDTH: i32 = 13;
const MERCHANT_VISIBLE_OFFER_COUNT: usize = 7;
const MERCHANT_TRADE_BUTTON_X: i32 = 5;
const MERCHANT_TRADE_BUTTON_Y: i32 = 18;
const MERCHANT_TRADE_BUTTON_WIDTH: u32 = 88;
const MERCHANT_TRADE_BUTTON_HEIGHT: u32 = 20;
const MERCHANT_TRADE_COST_A_X: i32 = 10;
const MERCHANT_TRADE_COST_B_X: i32 = 40;
const MERCHANT_TRADE_RESULT_X: i32 = 73;
const MERCHANT_TRADE_ITEM_Y: i32 = 19;
const MERCHANT_TRADE_ROW_HEIGHT: i32 = 20;
const MERCHANT_TRADE_ARROW_X: i32 = 60;
const MERCHANT_TRADE_ARROW_Y: i32 = 22;
const MERCHANT_TRADE_ARROW_WIDTH: u32 = 10;
const MERCHANT_TRADE_ARROW_HEIGHT: u32 = 9;
const MERCHANT_DISCOUNT_STRIKETHROUGH_X: i32 = 17;
const MERCHANT_DISCOUNT_STRIKETHROUGH_Y: i32 = 31;
const MERCHANT_DISCOUNT_STRIKETHROUGH_WIDTH: u32 = 9;
const MERCHANT_DISCOUNT_STRIKETHROUGH_HEIGHT: u32 = 2;
const MERCHANT_SCROLLER_X: i32 = 94;
const MERCHANT_SCROLLER_Y: i32 = 18;
const MERCHANT_SCROLLER_WIDTH: u32 = 6;
const MERCHANT_SCROLLER_HEIGHT: u32 = 27;
const MERCHANT_SCROLLER_TRACK_HEIGHT: i32 = 139;
const MERCHANT_SCROLLER_MAX_OFFSET: i32 = 113;
const MERCHANT_OUT_OF_STOCK_X: i32 = 182;
const MERCHANT_OUT_OF_STOCK_Y: i32 = 35;
const MERCHANT_OUT_OF_STOCK_WIDTH: u32 = 28;
const MERCHANT_OUT_OF_STOCK_HEIGHT: u32 = 21;
const MERCHANT_DEPRECATED_TOOLTIP_X: i32 = 186;
const MERCHANT_DEPRECATED_TOOLTIP_Y: i32 = 35;
const MERCHANT_DEPRECATED_TOOLTIP_WIDTH: i32 = 22;
const MERCHANT_DEPRECATED_TOOLTIP_HEIGHT: i32 = 21;
const MERCHANT_XP_BAR_X: i32 = 136;
const MERCHANT_XP_BAR_Y: i32 = 16;
const MERCHANT_XP_BAR_WIDTH: u32 = 102;
const MERCHANT_XP_BAR_HEIGHT: u32 = 5;
const VILLAGER_NEXT_LEVEL_XP_THRESHOLDS: [i32; 5] = [0, 10, 70, 150, 250];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CartographyResultMode {
    Map,
    Scaled,
    Duplicated,
    Locked,
}

#[derive(Debug, Clone, PartialEq)]
struct CartographyTablePresentation {
    background_layers: Vec<HudInventoryBackgroundLayer>,
    map_previews: Vec<HudInventoryMapPreview>,
    mode: Option<CartographyResultMode>,
    invalid_transform: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BeaconEffectButton {
    primary: bool,
    upgrade: bool,
    tier: i16,
    effect_id: i32,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct InventoryHudLocalState {
    creative_inventory: Option<CreativeInventoryHudState>,
    stonecutter_recipe_scroll_row: Option<i32>,
    beacon_effect_selection: Option<(Option<i32>, Option<i32>)>,
    loom_pattern_scroll_row: Option<i32>,
    loom_selected_pattern_index: Option<i32>,
    anvil_rename_text: Option<String>,
    recipe_book_search: RecipeBookSearchHudState,
    recipe_book_tabs: RecipeBookTabSelectionHudState,
    recipe_book_pages: RecipeBookPageHudState,
    recipe_book_presentation: RecipeBookPresentationHudState,
    recipe_book_overlay: Option<RecipeBookOverlayHudState>,
    advancement_scroll_delta: Option<(f64, f64)>,
    advancement_hover_fade: f32,
    dialog_ime_preedit: Option<DialogImePreeditHudState>,
    cursor_position: Option<(i32, i32)>,
    merchant_focused_trade_row: Option<i32>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: Vec<i16>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    advanced_item_tooltips: bool,
    smithing: smithing_screen::SmithingScreenFrame,
}

pub(crate) use control_requests::{
    apply_code_of_conduct_request, apply_control_input_request_with_effects,
    apply_control_net_request,
};
pub(crate) use events::LevelEventSoundRandomState;
use periodic_notifications::{PeriodicNotificationRuntime, PeriodicNotificationSchedule};
#[cfg(test)]
use toast::ClientToast;
use toast::ClientToastRuntime;

const HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY: usize = 240;
const HUD_DEBUG_NETWORK_SAMPLE_CAPACITY: usize = 240;
const HUD_DEBUG_TPS_SAMPLE_CAPACITY: usize = 240;
const HUD_DEBUG_NETWORK_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const BOSS_BAR_PROGRESS_LERP_DURATION: Duration = Duration::from_millis(100);
const HUD_HEALTH_DISPLAY_HOLD_DURATION: Duration = Duration::from_millis(1_000);
const CHUNK_BORDER_NEIGHBOR_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 0.5];
const CHUNK_BORDER_CELL_COLOR: [f32; 4] = [0.0, 155.0 / 255.0, 155.0 / 255.0, 1.0];
const CHUNK_BORDER_YELLOW_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const CHUNK_BORDER_MAJOR_COLOR: [f32; 4] = [0.25, 0.25, 1.0, 1.0];
const CHUNK_BORDER_THICK_LINE_WIDTH: f32 = 4.0;
const CHUNK_BORDER_THIN_LINE_WIDTH: f32 = 1.0;
const DEBUG_HEIGHTMAP_CHUNK_DISTANCE: i32 = 2;
const DEBUG_HEIGHTMAP_BOX_HEIGHT: f32 = 3.0 / 32.0;
const DEBUG_WATER_SCAN_RADIUS: i32 = 10;
const DEBUG_WATER_BOX_INSET: f32 = 0.01;
const DEBUG_WATER_BOX_COLOR: [u8; 4] = [0, 255, 0, 38];
const VANILLA_SPYGLASS_SCOPE_TARGET_SCALE: f32 = 1.125;
const VANILLA_SPYGLASS_SCOPE_RESET_SCALE: f32 = 0.5;

fn push_capped_hud_debug_sample<T>(samples: &mut Vec<T>, value: T, capacity: usize) {
    samples.push(value);
    if samples.len() > capacity {
        let overflow = samples.len() - capacity;
        samples.drain(0..overflow);
    }
}

#[derive(Debug, Default)]
pub(crate) struct ClientAnimationTickState {
    last_entity_animation_at: Option<Instant>,
    entity_animation_tick_residual: f64,
    last_entity_tick_work: EntityClientTickWork,
    last_running_level_ticks: u32,
}

#[derive(Debug, Default)]
pub(crate) struct HudRenderRuntime {
    insecure_chat_toast: SystemToastAnimationState,
    resource_pack_load_failure_toast: SystemToastAnimationState,
    low_disk_space_toast: SystemToastAnimationState,
    periodic_notification_toast: SystemToastAnimationState,
    periodic_notifications: PeriodicNotificationRuntime,
    client_toasts: ClientToastRuntime,
    debug_options_screen_narration: Option<ScreenNarrationFrame>,
    stats_screen_narration: Option<ScreenNarrationFrame>,
    recipe_book_narration: Option<ScreenNarrationFrame>,
    social_interactions_narration: Option<ScreenNarrationFrame>,
    player_reporting_narration: Option<ScreenNarrationFrame>,
    disconnected_screen_narration: Option<ScreenNarrationFrame>,
    chat_screen_narration: Option<ScreenNarrationFrame>,
    death_screen_narration: Option<ScreenNarrationFrame>,
    game_event_screen_narration: Option<ScreenNarrationFrame>,
    win_screen_resource_owner: Option<Arc<crate::win_screen_resources::WinScreenResources>>,
    win_screen_lines: Arc<[HudWinScreenLine]>,
    last_ui_frame_at: Option<Instant>,
    panorama_spin_degrees: f32,
    chat_cache_key: Option<HudChatCacheKey>,
    hotbar_block_item_models_cache_key: Option<HudHotbarBlockItemModelsCacheKey>,
    scoreboard_sidebar_cache_key: Option<HudScoreboardSidebarCacheKey>,
    player_list_cache_key: Option<HudPlayerListCacheKey>,
    boss_connection_generation: Option<u64>,
    boss_progress: BTreeMap<uuid::Uuid, BossBarProgressPresentation>,
    contextual: waypoint_locator::HudContextualRuntime,
    vignette: vignette::VignetteRuntime,
    scope_scale: f32,
    player_health: HudPlayerHealthPresentation,
    last_bubble_pop_sound_played: i32,
    smithing_screen: smithing_screen::SmithingScreenRuntime,
}

#[derive(Debug, Default)]
struct HudPlayerHealthPresentation {
    last_health: i32,
    display_health: i32,
    last_health_at: Option<Instant>,
    blink_until_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudPlayerHealthFrame {
    display_health: i32,
    blinking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HudAirBubblePopSound {
    volume: f32,
    pitch: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct HudFrameDeltaTicks {
    game_time: f32,
    realtime: f32,
}

#[derive(Debug)]
struct BossBarProgressPresentation {
    instance_revision: u64,
    progress_revision: u64,
    start_progress: f32,
    target_progress: f32,
    started_at: Instant,
    published_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudChatCacheKey {
    level_generation: u64,
    delivery_revision: u64,
    resource_generation: u64,
    resources_loaded: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HudHotbarBlockItemModelsCacheKey {
    ordinary_hotbar: bool,
    resource_generation: u64,
    resources_loaded: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HudScoreboardSidebarCacheKey {
    level_generation: u64,
    display_revision: u64,
    resource_generation: u64,
    resources_loaded: bool,
    hidden: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HudPlayerListCacheKey {
    level_generation: u64,
    display_revision: u64,
    profile_presentation_revision: u64,
    resource_generation: u64,
    resources_loaded: bool,
    visible: bool,
    show_heads: bool,
}

impl ClientAnimationTickState {
    pub(crate) fn entity_partial_tick(&self) -> f32 {
        self.entity_animation_tick_residual.clamp(0.0, 1.0) as f32
    }
}

impl HudRenderRuntime {
    pub(crate) fn prepare_periodic_notifications(
        catalog: &bbb_pack::PeriodicNotificationCatalog,
    ) -> anyhow::Result<Option<PeriodicNotificationSchedule>> {
        PeriodicNotificationSchedule::prepare(
            catalog,
            periodic_notifications::host_country_iso3().as_deref(),
        )
    }

    pub(crate) fn install_periodic_notifications(
        &mut self,
        generation: u64,
        schedule: Option<PeriodicNotificationSchedule>,
        now: Instant,
    ) {
        self.periodic_notifications
            .install(generation, schedule, now);
        self.periodic_notification_toast = SystemToastAnimationState::default();
    }

    pub(crate) fn debug_options_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.debug_options_screen_narration.as_ref()
    }

    pub(crate) fn stats_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.stats_screen_narration.as_ref()
    }

    pub(crate) fn recipe_book_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.recipe_book_narration.as_ref()
    }

    pub(crate) fn social_interactions_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.social_interactions_narration.as_ref()
    }

    pub(crate) fn player_reporting_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.player_reporting_narration.as_ref()
    }

    pub(crate) fn game_event_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.game_event_screen_narration.as_ref()
    }

    pub(crate) fn death_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.death_screen_narration.as_ref()
    }

    pub(crate) fn disconnected_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.disconnected_screen_narration.as_ref()
    }

    pub(crate) fn chat_screen_narration(&self) -> Option<&ScreenNarrationFrame> {
        self.chat_screen_narration.as_ref()
    }

    fn win_screen_lines(
        &mut self,
        resources: &Arc<crate::win_screen_resources::WinScreenResources>,
    ) -> Arc<[HudWinScreenLine]> {
        let unchanged = self
            .win_screen_resource_owner
            .as_ref()
            .is_some_and(|previous| Arc::ptr_eq(previous, resources));
        if !unchanged {
            self.win_screen_resource_owner = Some(Arc::clone(resources));
            self.win_screen_lines = resources
                .lines
                .iter()
                .map(|line| HudWinScreenLine {
                    runs: line.runs.clone(),
                    centered: line.centered,
                })
                .collect::<Vec<_>>()
                .into();
        }
        Arc::clone(&self.win_screen_lines)
    }

    fn frame_delta_ticks(&mut self, now: Instant, tick_interval: Duration) -> HudFrameDeltaTicks {
        let Some(previous) = self.last_ui_frame_at.replace(now) else {
            return HudFrameDeltaTicks::default();
        };
        let elapsed_seconds = now.saturating_duration_since(previous).as_secs_f32();
        if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 {
            return HudFrameDeltaTicks::default();
        }
        let tick_interval_seconds = tick_interval.as_secs_f32();
        let game_time = if tick_interval_seconds.is_finite() && tick_interval_seconds > 0.0 {
            let delta = elapsed_seconds / tick_interval_seconds;
            if delta.is_finite() && delta > 0.0 {
                delta
            } else {
                0.0
            }
        } else {
            0.0
        };
        let realtime_ticks = elapsed_seconds * 20.0;
        let realtime = if !realtime_ticks.is_finite() || realtime_ticks <= 0.0 {
            0.0
        } else if realtime_ticks > 7.0 {
            0.5
        } else {
            realtime_ticks
        };
        HudFrameDeltaTicks {
            game_time,
            realtime,
        }
    }

    fn panorama_rotation_degrees(&mut self, visible: bool, frame_delta: HudFrameDeltaTicks) -> f32 {
        if visible {
            self.panorama_spin_degrees = panorama_wrap_degrees(
                self.panorama_spin_degrees + frame_delta.realtime.max(0.0) * 0.1,
            );
        }
        -self.panorama_spin_degrees
    }

    fn player_health_frame(
        &mut self,
        now: Instant,
        tick_count: u64,
        current_health: i32,
        invulnerable_time: i32,
    ) -> HudPlayerHealthFrame {
        // Vanilla computes this frame's phase before observing a health edge,
        // so a newly armed deadline cannot blink until a later extraction.
        let remaining_blink_ticks = self
            .player_health
            .blink_until_tick
            .saturating_sub(tick_count);
        let blinking = self.player_health.blink_until_tick > tick_count
            && (remaining_blink_ticks / 3) % 2 == 1;

        if invulnerable_time > 0 {
            let blink_duration = if current_health < self.player_health.last_health {
                Some(20)
            } else if current_health > self.player_health.last_health {
                Some(10)
            } else {
                None
            };
            if let Some(blink_duration) = blink_duration {
                self.player_health.last_health_at = Some(now);
                self.player_health.blink_until_tick = tick_count.saturating_add(blink_duration);
            }
        }

        let display_hold_expired = self.player_health.last_health_at.map_or(true, |last| {
            now.saturating_duration_since(last) > HUD_HEALTH_DISPLAY_HOLD_DURATION
        });
        if display_hold_expired {
            self.player_health.display_health = current_health;
            self.player_health.last_health_at = Some(now);
        }
        self.player_health.last_health = current_health;

        HudPlayerHealthFrame {
            display_health: self.player_health.display_health,
            blinking,
        }
    }

    fn air_bubble_pop_sound(
        &mut self,
        state: HudAirSupply,
        extracted: bool,
    ) -> Option<HudAirBubblePopSound> {
        if !extracted || !hud_air_supply_visible(state) {
            return None;
        }
        if !state.eye_in_water {
            self.last_bubble_pop_sound_played = 0;
            return None;
        }
        let pop = hud_air_supply_pop(state)?;
        if self.last_bubble_pop_sound_played == pop.bubble {
            return None;
        }
        self.last_bubble_pop_sound_played = pop.bubble;
        Some(HudAirBubblePopSound {
            volume: 0.5 + 0.1 * (pop.empty_bubbles - 2).max(0) as f32,
            pitch: 1.0 + 0.1 * (pop.empty_bubbles - 4).max(0) as f32,
        })
    }

    fn win_screen_realtime_delta_ticks(
        generation: Option<u64>,
        frame_delta: HudFrameDeltaTicks,
    ) -> Option<(u64, f32)> {
        let generation = generation?;
        (frame_delta.realtime > 0.0).then_some((generation, frame_delta.realtime))
    }

    fn spyglass_scope(
        &mut self,
        game_time_delta_ticks: f32,
        hide_gui: bool,
        first_person: bool,
        scoping: bool,
    ) -> Option<HudSpyglassScope> {
        if hide_gui {
            return None;
        }
        let delta_ticks = if game_time_delta_ticks.is_finite() && game_time_delta_ticks > 0.0 {
            game_time_delta_ticks
        } else {
            0.0
        };
        self.scope_scale +=
            0.5 * delta_ticks * (VANILLA_SPYGLASS_SCOPE_TARGET_SCALE - self.scope_scale);
        if !first_person {
            return None;
        }
        if scoping {
            Some(HudSpyglassScope {
                scale: self.scope_scale,
            })
        } else {
            self.scope_scale = VANILLA_SPYGLASS_SCOPE_RESET_SCALE;
            None
        }
    }

    fn chat_messages_update(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> RendererFrameUpdate<Vec<HudChatMessage>> {
        let key = HudChatCacheKey {
            level_generation: world.client_level_generation(),
            delivery_revision: world.client_chat().delivery_revision,
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            resources_loaded: item_runtime.is_some(),
        };
        if self.chat_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.chat_cache_key = Some(key);
        RendererFrameUpdate::Replace(hud_chat_messages_from_world(world, item_runtime))
    }

    fn hotbar_block_item_models_update(
        &mut self,
        stable_world_unchanged: bool,
        key: HudHotbarBlockItemModelsCacheKey,
        project: impl FnOnce() -> Vec<Option<HudBlockItemModel>>,
    ) -> RendererFrameUpdate<Vec<Option<HudBlockItemModel>>> {
        if stable_world_unchanged && self.hotbar_block_item_models_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.hotbar_block_item_models_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    fn scoreboard_sidebar_update(
        &mut self,
        key: HudScoreboardSidebarCacheKey,
        project: impl FnOnce() -> Option<HudScoreboardSidebar>,
    ) -> RendererFrameUpdate<Option<HudScoreboardSidebar>> {
        if self.scoreboard_sidebar_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.scoreboard_sidebar_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    fn player_list_update(
        &mut self,
        key: HudPlayerListCacheKey,
        project: impl FnOnce() -> Option<HudPlayerList>,
    ) -> RendererFrameUpdate<Option<HudPlayerList>> {
        if self.player_list_cache_key == Some(key) {
            return RendererFrameUpdate::Unchanged;
        }
        self.player_list_cache_key = Some(key);
        RendererFrameUpdate::Replace(project())
    }

    fn boss_bars_update(
        &mut self,
        world: &WorldStore,
        connection_generation: u64,
        change_anchor: Instant,
        render_now: Instant,
        stable_world_unchanged: bool,
    ) -> RendererFrameUpdate<Vec<bbb_renderer::HudBossBar>> {
        let connection_changed = self.boss_connection_generation != Some(connection_generation);
        if connection_changed {
            self.boss_connection_generation = Some(connection_generation);
            self.boss_progress.clear();
        }
        self.boss_progress
            .retain(|id, _| world.boss_bar(*id).is_some());

        let mut must_replace = connection_changed || !stable_world_unchanged;
        let mut bars = Vec::with_capacity(world.boss_bars().len());
        for bar in world.boss_bars() {
            let presentation =
                self.boss_progress
                    .entry(bar.id)
                    .or_insert_with(|| BossBarProgressPresentation {
                        instance_revision: bar.instance_revision,
                        progress_revision: bar.progress_revision,
                        start_progress: bar.progress,
                        target_progress: bar.progress,
                        started_at: change_anchor,
                        published_complete: true,
                    });
            if presentation.instance_revision != bar.instance_revision {
                *presentation = BossBarProgressPresentation {
                    instance_revision: bar.instance_revision,
                    progress_revision: bar.progress_revision,
                    start_progress: bar.progress,
                    target_progress: bar.progress,
                    started_at: change_anchor,
                    published_complete: true,
                };
            } else if presentation.progress_revision != bar.progress_revision {
                let visible_progress = presentation.progress_at(change_anchor);
                presentation.progress_revision = bar.progress_revision;
                presentation.start_progress = visible_progress;
                presentation.target_progress = bar.progress;
                presentation.started_at = change_anchor;
                presentation.published_complete = false;
            }

            let progress = presentation.progress_at(render_now);
            let active = presentation.is_active_at(render_now);
            if active || !presentation.published_complete {
                must_replace = true;
            }
            if !active {
                presentation.published_complete = true;
            }
            if let Some(projected) = hud_boss_bar_from_world_state(bar, progress) {
                bars.push(projected);
            }
        }

        if must_replace {
            RendererFrameUpdate::Replace(bars)
        } else {
            RendererFrameUpdate::Unchanged
        }
    }
}

impl BossBarProgressPresentation {
    fn progress_at(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        let fraction =
            (elapsed.as_secs_f32() / BOSS_BAR_PROGRESS_LERP_DURATION.as_secs_f32()).clamp(0.0, 1.0);
        self.start_progress + fraction * (self.target_progress - self.start_progress)
    }

    fn is_active_at(&self, now: Instant) -> bool {
        self.start_progress != self.target_progress
            && now.saturating_duration_since(self.started_at) < BOSS_BAR_PROGRESS_LERP_DURATION
    }
}

#[derive(Debug, Default)]
struct SystemToastAnimationState {
    seen_warning_count: usize,
    animation_started_at: Option<Instant>,
    became_fully_visible_at: Option<Instant>,
    last_changed_fully_visible_ms: u64,
    visible_portion: f32,
    showing: bool,
    changed: bool,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SystemToastAnimationFrame {
    visible_portion: f32,
    visibility_sound: Option<&'static str>,
}

impl SystemToastAnimationState {
    fn update(&mut self, warning_count: usize, now: Instant) -> Option<SystemToastAnimationFrame> {
        if warning_count < self.seen_warning_count {
            *self = Self::default();
        }
        if warning_count > self.seen_warning_count {
            if self.finished || self.animation_started_at.is_none() {
                *self = Self {
                    seen_warning_count: warning_count,
                    changed: true,
                    ..Self::default()
                };
            } else {
                self.seen_warning_count = warning_count;
                self.changed = true;
            }
        }
        if self.seen_warning_count == 0 {
            return None;
        }
        let showing_before_update = self.showing;
        let animation_started_at = *self.animation_started_at.get_or_insert_with(|| {
            self.showing = true;
            now
        });
        if self.showing
            && now.saturating_duration_since(animation_started_at) <= Duration::from_millis(600)
        {
            self.became_fully_visible_at = Some(now);
        }
        let became_fully_visible_at = self.became_fully_visible_at.unwrap_or(now);
        let fully_visible_for_ms = now
            .saturating_duration_since(became_fully_visible_at)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let progress = (now
            .saturating_duration_since(animation_started_at)
            .as_secs_f32()
            / 0.6)
            .clamp(0.0, 1.0)
            .powi(2);
        self.visible_portion = if self.showing {
            progress
        } else {
            1.0 - progress
        };
        if self.changed {
            self.last_changed_fully_visible_ms = fully_visible_for_ms;
            self.changed = false;
        }
        let wanted_showing =
            fully_visible_for_ms.saturating_sub(self.last_changed_fully_visible_ms) < 10_000;
        if wanted_showing != self.showing {
            let reverse_ms = ((1.0 - self.visible_portion) * 600.0) as u64;
            self.animation_started_at = Some(now - Duration::from_millis(reverse_ms));
            self.showing = wanted_showing;
        }
        self.finished = !self.showing
            && now.saturating_duration_since(
                self.animation_started_at
                    .expect("toast animation start is initialized"),
            ) > Duration::from_millis(600);
        (!self.finished).then_some(SystemToastAnimationFrame {
            visible_portion: self.visible_portion.clamp(0.0, 1.0),
            visibility_sound: (showing_before_update != self.showing).then_some(if self.showing {
                "minecraft:ui.toast.in"
            } else {
                "minecraft:ui.toast.out"
            }),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugFpsSampler {
    window_started_at: Option<Instant>,
    last_frame_at: Option<Instant>,
    frames_in_window: u32,
    last_reported_fps: u32,
    frame_time_nanos: Vec<u64>,
}

impl HudDebugFpsSampler {
    pub(crate) fn record_frame(&mut self, now: Instant) {
        if let Some(last_frame_at) = self.last_frame_at {
            let nanos = now
                .saturating_duration_since(last_frame_at)
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64;
            self.frame_time_nanos.push(nanos);
            if self.frame_time_nanos.len() > HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY {
                let overflow = self.frame_time_nanos.len() - HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY;
                self.frame_time_nanos.drain(0..overflow);
            }
        }
        self.last_frame_at = Some(now);

        let Some(started_at) = self.window_started_at else {
            self.window_started_at = Some(now);
            self.frames_in_window = 1;
            return;
        };

        self.frames_in_window = self.frames_in_window.saturating_add(1);
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed < Duration::from_secs(1) {
            return;
        }

        let fps = self.frames_in_window as f64 / elapsed.as_secs_f64();
        self.last_reported_fps = fps.round().clamp(0.0, u32::MAX as f64) as u32;
        self.window_started_at = Some(now);
        self.frames_in_window = 0;
    }

    pub(crate) fn fps(&self) -> u32 {
        self.last_reported_fps
    }

    pub(crate) fn frame_time_nanos(&self) -> Vec<u64> {
        self.frame_time_nanos.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugNetworkSampler {
    last_sample_at: Option<Instant>,
    pending_bandwidth_bytes: u64,
    ping_millis: Vec<u64>,
    bandwidth_bytes_per_tick: Vec<u64>,
}

impl HudDebugNetworkSampler {
    fn record_net_event(&mut self, event: &NetEvent) {
        match event {
            NetEvent::PacketSeen { len, .. } => {
                self.record_received_packet(*len);
            }
            NetEvent::Play(bbb_protocol::packets::PlayClientbound::PongResponse(response)) => {
                self.record_pong_response(response.time, wall_clock_millis_i64());
            }
            NetEvent::Disconnected { .. } => {
                self.reset();
            }
            _ => {}
        }
    }

    fn record_received_packet(&mut self, len: usize) {
        self.pending_bandwidth_bytes = self
            .pending_bandwidth_bytes
            .saturating_add(u64::try_from(len).unwrap_or(u64::MAX));
    }

    fn record_pong_response(&mut self, sent_millis: i64, received_millis: i64) {
        let latency = received_millis.saturating_sub(sent_millis).max(0) as u64;
        push_capped_hud_debug_sample(
            &mut self.ping_millis,
            latency,
            HUD_DEBUG_NETWORK_SAMPLE_CAPACITY,
        );
    }

    fn advance_tick(
        &mut self,
        input: &ClientInputState,
        net_counters: &NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        now: Instant,
        now_millis: i64,
    ) {
        if !net_counters.connected {
            self.reset();
            return;
        }

        let ticks = match self.last_sample_at {
            Some(last_sample_at) => {
                let elapsed = now.saturating_duration_since(last_sample_at);
                let ticks = elapsed.as_millis() / HUD_DEBUG_NETWORK_SAMPLE_INTERVAL.as_millis();
                if ticks == 0 {
                    return;
                }
                ticks.min(HUD_DEBUG_NETWORK_SAMPLE_CAPACITY as u128) as usize
            }
            None => 1,
        };
        self.last_sample_at = Some(now);

        for tick in 0..ticks {
            let bytes = if tick == 0 {
                std::mem::take(&mut self.pending_bandwidth_bytes)
            } else {
                0
            };
            push_capped_hud_debug_sample(
                &mut self.bandwidth_bytes_per_tick,
                bytes,
                HUD_DEBUG_NETWORK_SAMPLE_CAPACITY,
            );
            if input.debug_network_charts_visible() && net_counters.state.as_deref() == Some("Play")
            {
                if let Some(tx) = net_commands {
                    let _ = tx.try_send(NetCommand::PingRequest(now_millis));
                }
            }
        }
    }

    fn reset(&mut self) {
        self.last_sample_at = None;
        self.pending_bandwidth_bytes = 0;
        self.ping_millis.clear();
        self.bandwidth_bytes_per_tick.clear();
    }

    fn ping_millis(&self) -> Vec<u64> {
        self.ping_millis.clone()
    }

    fn bandwidth_bytes_per_tick(&self) -> Vec<u64> {
        self.bandwidth_bytes_per_tick.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HudDebugTpsSampler {
    samples: Vec<HudDebugTpsSample>,
}

const DEBUG_COLLISION_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
const DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
const DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS: f64 = 16.0;

#[derive(Debug, Default)]
pub(crate) struct DebugCollisionCache {
    last_update_at: Option<Instant>,
    lines: Vec<DebugLine>,
}

#[derive(Debug, Default)]
pub(crate) struct DebugSupportingBlockCache {
    last_update_at: Option<Instant>,
    entity_ids: Vec<i32>,
}

impl DebugCollisionCache {
    fn lines(
        &mut self,
        input: &ClientInputState,
        world: &WorldStore,
        now: Instant,
    ) -> Vec<DebugLine> {
        if !debug_collision_entry_enabled(input, world) {
            self.reset();
            return Vec::new();
        }
        let refresh = self.last_update_at.is_none_or(|last_update_at| {
            now.saturating_duration_since(last_update_at) > DEBUG_COLLISION_REFRESH_INTERVAL
        });
        if refresh {
            self.last_update_at = Some(now);
            self.lines = collect_debug_collision_lines(world);
        }
        self.lines.clone()
    }

    fn reset(&mut self) {
        self.last_update_at = None;
        self.lines.clear();
    }
}

impl DebugSupportingBlockCache {
    fn lines(
        &mut self,
        input: &ClientInputState,
        world: &WorldStore,
        now: Instant,
    ) -> Vec<DebugLine> {
        if !debug_supporting_block_entry_enabled(input, world) {
            self.reset();
            return Vec::new();
        }
        let refresh = self.last_update_at.is_none_or(|last_update_at| {
            now.saturating_duration_since(last_update_at) > DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL
        });
        if refresh {
            self.last_update_at = Some(now);
            self.entity_ids =
                world.debug_supporting_block_entity_ids(DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS);
        }
        collect_debug_supporting_block_lines(world, &self.entity_ids)
    }

    fn reset(&mut self) {
        self.last_update_at = None;
        self.entity_ids.clear();
    }
}

impl HudDebugTpsSampler {
    fn record_net_event(&mut self, event: &NetEvent) {
        match event {
            NetEvent::Play(bbb_protocol::packets::PlayClientbound::DebugSample(sample))
                if sample.sample_type == RemoteDebugSampleType::TickTime =>
            {
                self.record_debug_sample(&sample.sample);
            }
            NetEvent::Disconnected { .. } => {
                self.reset();
            }
            _ => {}
        }
    }

    fn record_debug_sample(&mut self, sample: &[i64]) {
        let value =
            |index: usize| -> u64 { sample.get(index).copied().unwrap_or_default().max(0) as u64 };
        push_capped_hud_debug_sample(
            &mut self.samples,
            HudDebugTpsSample {
                full_tick_nanos: value(0),
                tick_server_method_nanos: value(1),
                scheduled_tasks_nanos: value(2),
                idle_nanos: value(3),
            },
            HUD_DEBUG_TPS_SAMPLE_CAPACITY,
        );
    }

    fn sync_subscriptions_and_tick(
        &mut self,
        input: &ClientInputState,
        world: &mut WorldStore,
        net_counters: &NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if !net_counters.connected {
            self.reset();
            world.clear_debug_subscriber();
            return;
        }
        if net_counters.state.as_deref() != Some("Play") {
            return;
        }

        let desired = input
            .remote_debug_renderers()
            .subscription_ids(input.debug_fps_charts_visible());
        if world
            .requested_debug_subscriptions()
            .iter()
            .copied()
            .eq(desired.iter().copied())
        {
            // `ClientDebugSubscriber.tick` changes the request set before
            // purging against the current pre-increment level game time.
            let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
            world.tick_debug_subscriber(game_time);
            return;
        }
        if let Some(tx) = net_commands {
            if tx
                .try_send(NetCommand::DebugSubscriptionRequest {
                    subscription_ids: desired.clone(),
                })
                .is_ok()
            {
                world
                    .set_requested_debug_subscriptions(&desired)
                    .expect("startup-derived debug subscriptions must use generated ids");
            }
        }
        let game_time = world.world_time().map(|time| time.game_time).unwrap_or(0);
        world.tick_debug_subscriber(game_time);
    }

    fn chart(&self, world: &WorldStore) -> Option<HudDebugTpsChart> {
        (!self.samples.is_empty()).then(|| HudDebugTpsChart {
            samples: self.samples.clone(),
            milliseconds_per_tick: world_ticking_milliseconds_per_tick(world),
        })
    }

    #[cfg(test)]
    fn samples(&self) -> Vec<HudDebugTpsSample> {
        self.samples.clone()
    }

    fn reset(&mut self) {
        self.samples.clear();
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CameraSpinningEffectTickState {
    time_degrees: f32,
    speed_degrees_per_tick: f32,
}

impl CameraSpinningEffectTickState {
    fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) {
        for _ in 0..ticks {
            let nausea_intensity = local_player_effect(world, VANILLA_MOB_EFFECT_NAUSEA_ID)
                .map(|effect| effect.blend_factor(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK))
                .unwrap_or(0.0);
            if nausea_intensity > 0.0 {
                // With no portal contribution yet, vanilla's weighted
                // `(portal * 20 + nausea * 7) / (portal + nausea)` is exactly 7.
                self.speed_degrees_per_tick = 7.0;
                self.time_degrees += self.speed_degrees_per_tick;
            } else {
                self.speed_degrees_per_tick = 0.0;
            }
        }
    }

    fn effect_for_world(
        self,
        world: &WorldStore,
        partial_tick: f32,
    ) -> Option<bbb_renderer::CameraSpinningEffect> {
        let intensity =
            local_player_effect(world, VANILLA_MOB_EFFECT_NAUSEA_ID)?.blend_factor(partial_tick);
        (intensity > 0.0).then_some(bbb_renderer::CameraSpinningEffect {
            intensity,
            angle_degrees: self.time_degrees
                + partial_tick.clamp(0.0, 1.0) * self.speed_degrees_per_tick,
        })
    }
}

#[derive(Debug)]
pub(crate) struct LightmapTickState {
    random: LevelEventSoundRandomState,
    block_light_flicker: f32,
    brightness_factor: f32,
    client_tick_count: u64,
    water_vision_time: i32,
    end_flash_state: EndFlashLightmapState,
    boss_overlay_world_darkening: f32,
    boss_overlay_world_darkening_previous_frame: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
}

impl Default for LightmapTickState {
    fn default() -> Self {
        Self {
            random: LevelEventSoundRandomState::default(),
            block_light_flicker: 0.0,
            brightness_factor: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }
}

impl LightmapTickState {
    pub(crate) fn client_tick_count(&self) -> u64 {
        self.client_tick_count
    }

    pub(crate) fn with_brightness_factor_and_hide_lightning_flash(
        brightness_factor: f32,
        hide_lightning_flash: bool,
    ) -> Self {
        Self {
            brightness_factor: sanitize_lightmap_brightness_factor(brightness_factor),
            hide_lightning_flash,
            ..Self::default()
        }
    }

    #[cfg(test)]
    fn with_seed(seed: i64) -> Self {
        Self {
            random: LevelEventSoundRandomState::with_seed(seed),
            block_light_flicker: 0.0,
            brightness_factor: VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }

    #[cfg(test)]
    fn with_seed_and_brightness(seed: i64, brightness_factor: f32) -> Self {
        Self {
            random: LevelEventSoundRandomState::with_seed(seed),
            block_light_flicker: 0.0,
            brightness_factor: sanitize_lightmap_brightness_factor(brightness_factor),
            client_tick_count: 0,
            water_vision_time: 0,
            end_flash_state: EndFlashLightmapState::default(),
            boss_overlay_world_darkening: 0.0,
            boss_overlay_world_darkening_previous_frame: 0.0,
            rain_fog_multiplier: 0.0,
            hide_lightning_flash: false,
        }
    }

    #[cfg(test)]
    fn advance(&mut self, ticks: u32) -> f32 {
        for _ in 0..ticks {
            self.tick();
        }
        self.client_tick_count = self.client_tick_count.saturating_add(ticks as u64);
        self.block_factor()
    }

    fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) -> f32 {
        for _ in 0..ticks {
            self.tick();
            self.tick_water_vision(world);
            self.tick_end_flash(world);
            self.tick_boss_overlay_world_darkening(world);
        }
        self.client_tick_count = self.client_tick_count.saturating_add(ticks as u64);
        self.block_factor()
    }

    fn advance_rain_fog_for_world(
        &mut self,
        ticks: u32,
        world: &WorldStore,
        terrain_textures: &TerrainTextureState,
    ) {
        for _ in 0..ticks {
            self.tick_rain_fog(world, terrain_textures);
        }
    }

    fn environment_for_world(&self, world: &WorldStore) -> LightmapEnvironment {
        lightmap_environment_for_world_with_effects(
            world,
            self.brightness_factor,
            self.block_light_flicker + VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR,
            self.client_tick_count,
            VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
            local_player_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID)
                .map(|effect| effect.blend_factor(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK))
                .unwrap_or(0.0),
            local_player_effect(world, VANILLA_MOB_EFFECT_NIGHT_VISION_ID),
            local_player_effect(world, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID),
            self.water_vision(world),
            self.end_flash_sky_factor(world),
            self.boss_overlay_world_darkening(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK),
            self.sky_flash_visible(world),
        )
    }

    fn tick(&mut self) {
        // Vanilla `LightmapRenderStateExtractor.tick`: the random source is a
        // LegacyRandomSource from `RandomSource.create()`.
        let delta = (self.random.next_float() - self.random.next_float())
            * self.random.next_float()
            * self.random.next_float()
            * 0.1;
        self.block_light_flicker = (self.block_light_flicker + delta) * 0.9;
    }

    fn block_factor(&self) -> f32 {
        self.block_light_flicker + VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR
    }

    fn tick_water_vision(&mut self, world: &WorldStore) {
        if world.local_player_eye_in_water() {
            let speed = if world.local_player_is_spectator() {
                10
            } else {
                1
            };
            self.water_vision_time =
                (self.water_vision_time + speed).clamp(0, VANILLA_WATER_VISION_MAX_TICKS);
        } else if self.water_vision_time > 0 {
            self.water_vision_time =
                (self.water_vision_time - 10).clamp(0, VANILLA_WATER_VISION_MAX_TICKS);
        }
    }

    fn water_vision(&self, world: &WorldStore) -> f32 {
        if !world.local_player_eye_in_water() {
            return 0.0;
        }
        if self.water_vision_time >= VANILLA_WATER_VISION_MAX_TICKS {
            return 1.0;
        }

        let water_vision_time = self.water_vision_time as f32;
        let fast = (water_vision_time / VANILLA_WATER_VISION_FAST_RAMP_TICKS).clamp(0.0, 1.0);
        let slow = if self.water_vision_time < VANILLA_WATER_VISION_FAST_RAMP_TICKS as i32 {
            0.0
        } else {
            ((water_vision_time - VANILLA_WATER_VISION_FAST_RAMP_TICKS)
                / VANILLA_WATER_VISION_SLOW_RAMP_TICKS)
                .clamp(0.0, 1.0)
        };
        fast * VANILLA_WATER_VISION_FAST_WEIGHT + slow * VANILLA_WATER_VISION_SLOW_WEIGHT
    }

    fn tick_end_flash(&mut self, world: &WorldStore) {
        if world.level_info().map(vanilla_lightmap_dimension_kind)
            != Some(VanillaLightmapDimensionKind::End)
        {
            self.end_flash_state = EndFlashLightmapState::default();
            return;
        }

        self.end_flash_state
            .tick(world.world_time().map(|time| time.day_time).unwrap_or(0));
    }

    fn end_flash_sky_factor(&self, world: &WorldStore) -> f32 {
        if self.hide_lightning_flash {
            return 0.0;
        }

        if world.level_info().map(vanilla_lightmap_dimension_kind)
            == Some(VanillaLightmapDimensionKind::End)
        {
            let intensity = self
                .end_flash_state
                .intensity(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK)
                .clamp(0.0, 1.0);
            if world.boss_overlay_should_create_world_fog() {
                intensity / 3.0
            } else {
                intensity
            }
        } else {
            0.0
        }
    }

    fn sky_flash_visible(&self, world: &WorldStore) -> bool {
        !self.hide_lightning_flash && world.sky_flash_time() > 0
    }

    fn tick_boss_overlay_world_darkening(&mut self, world: &WorldStore) {
        self.boss_overlay_world_darkening_previous_frame = self.boss_overlay_world_darkening;
        if world.boss_overlay_should_darken_screen() {
            self.boss_overlay_world_darkening += VANILLA_BOSS_OVERLAY_DARKEN_IN_STEP;
            if self.boss_overlay_world_darkening > 1.0 {
                self.boss_overlay_world_darkening = 1.0;
            }
        } else if self.boss_overlay_world_darkening > 0.0 {
            self.boss_overlay_world_darkening -= VANILLA_BOSS_OVERLAY_DARKEN_OUT_STEP;
        }
    }

    fn boss_overlay_world_darkening(&self, partial_tick: f32) -> f32 {
        let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
        self.boss_overlay_world_darkening_previous_frame
            + (self.boss_overlay_world_darkening - self.boss_overlay_world_darkening_previous_frame)
                * partial_tick
    }

    fn tick_rain_fog(&mut self, world: &WorldStore, terrain_textures: &TerrainTextureState) {
        let target = atmospheric_rain_fog_target_multiplier(world, terrain_textures);
        self.rain_fog_multiplier +=
            (target - self.rain_fog_multiplier) * VANILLA_RAIN_FOG_SMOOTHING_PER_TICK;
    }

    fn rain_fog_multiplier(&self) -> f32 {
        self.rain_fog_multiplier.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct EndFlashLightmapState {
    flash_seed: i64,
    offset: i32,
    duration: i32,
    intensity: f32,
    old_intensity: f32,
}

impl EndFlashLightmapState {
    fn tick(&mut self, clock_time: i64) {
        self.calculate_flash_parameters(clock_time);
        self.old_intensity = self.intensity;
        self.intensity = self.calculate_intensity(clock_time);
    }

    fn calculate_flash_parameters(&mut self, clock_time: i64) {
        let new_seed = clock_time / VANILLA_END_FLASH_INTERVAL_TICKS;
        if new_seed == self.flash_seed {
            return;
        }

        let mut random = LevelEventSoundRandomState::with_seed(new_seed);
        random.next_float();
        self.offset = random_between_inclusive(&mut random, 0, VANILLA_END_FLASH_MAX_OFFSET_TICKS);
        let max_duration = VANILLA_END_FLASH_MAX_DURATION_TICKS.min(600 - self.offset);
        self.duration = random_between_inclusive(
            &mut random,
            VANILLA_END_FLASH_MIN_DURATION_TICKS,
            max_duration,
        );
        self.flash_seed = new_seed;
    }

    fn calculate_intensity(&self, clock_time: i64) -> f32 {
        if self.duration <= 0 {
            return 0.0;
        }

        let clock_time_within_interval = clock_time % VANILLA_END_FLASH_INTERVAL_TICKS;
        let offset = i64::from(self.offset);
        let duration = i64::from(self.duration);
        if clock_time_within_interval >= offset && clock_time_within_interval <= offset + duration {
            ((clock_time_within_interval - offset) as f32 * std::f32::consts::PI
                / self.duration as f32)
                .sin()
        } else {
            0.0
        }
    }

    fn intensity(&self, partial_tick: f32) -> f32 {
        let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
        self.old_intensity + (self.intensity - self.old_intensity) * partial_tick
    }
}

const fn rgb24(color: i32) -> [f32; 3] {
    let rgb = color as u32;
    [
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
    ]
}

const fn rgba32(color: i32) -> [f32; 4] {
    let argb = color as u32;
    [
        ((argb >> 16) & 0xff) as f32 / 255.0,
        ((argb >> 8) & 0xff) as f32 / 255.0,
        (argb & 0xff) as f32 / 255.0,
        ((argb >> 24) & 0xff) as f32 / 255.0,
    ]
}

const fn argb_to_rgba8(color: i32) -> [u8; 4] {
    let argb = color as u32;
    [
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        ((argb >> 24) & 0xff) as u8,
    ]
}

const fn argb_color(alpha: i32, red: i32, green: i32, blue: i32) -> i32 {
    ((((alpha & 0xff) as u32) << 24)
        | (((red & 0xff) as u32) << 16)
        | (((green & 0xff) as u32) << 8)
        | ((blue & 0xff) as u32)) as i32
}

fn rgb01_to_argb(color: [f32; 3]) -> i32 {
    argb_color(
        255,
        argb_channel_from_unit(color[0]),
        argb_channel_from_unit(color[1]),
        argb_channel_from_unit(color[2]),
    )
}

fn rgba01_to_argb(color: [f32; 4]) -> i32 {
    argb_color(
        argb_channel_from_unit(color[3]),
        argb_channel_from_unit(color[0]),
        argb_channel_from_unit(color[1]),
        argb_channel_from_unit(color[2]),
    )
}

fn argb_channel_from_unit(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 255.0).floor() as i32
}

fn argb_alpha(color: i32) -> i32 {
    ((color as u32) >> 24) as i32
}

fn argb_red(color: i32) -> i32 {
    (((color as u32) >> 16) & 0xff) as i32
}

fn argb_green(color: i32) -> i32 {
    (((color as u32) >> 8) & 0xff) as i32
}

fn argb_blue(color: i32) -> i32 {
    ((color as u32) & 0xff) as i32
}

fn argb_opaque(color: i32) -> i32 {
    argb_color(255, argb_red(color), argb_green(color), argb_blue(color))
}

fn argb_lerp_int(alpha: f32, from: i32, to: i32) -> i32 {
    from + (alpha * (to - from) as f32).floor() as i32
}

fn argb_srgb_lerp(alpha: f32, from: i32, to: i32) -> i32 {
    argb_color(
        argb_lerp_int(alpha, argb_alpha(from), argb_alpha(to)),
        argb_lerp_int(alpha, argb_red(from), argb_red(to)),
        argb_lerp_int(alpha, argb_green(from), argb_green(to)),
        argb_lerp_int(alpha, argb_blue(from), argb_blue(to)),
    )
}

fn argb_multiply(lhs: i32, rhs: i32) -> i32 {
    if lhs == -1 {
        rhs
    } else if rhs == -1 {
        lhs
    } else {
        argb_color(
            argb_alpha(lhs) * argb_alpha(rhs) / 255,
            argb_red(lhs) * argb_red(rhs) / 255,
            argb_green(lhs) * argb_green(rhs) / 255,
            argb_blue(lhs) * argb_blue(rhs) / 255,
        )
    }
}

fn argb_alpha_blend(destination: i32, source: i32) -> i32 {
    let destination_alpha = argb_alpha(destination);
    let source_alpha = argb_alpha(source);
    if source_alpha == 255 {
        return source;
    }
    if source_alpha == 0 {
        return destination;
    }

    let alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
    argb_color(
        alpha,
        argb_alpha_blend_channel(alpha, source_alpha, argb_red(destination), argb_red(source)),
        argb_alpha_blend_channel(
            alpha,
            source_alpha,
            argb_green(destination),
            argb_green(source),
        ),
        argb_alpha_blend_channel(
            alpha,
            source_alpha,
            argb_blue(destination),
            argb_blue(source),
        ),
    )
}

fn argb_alpha_blend_channel(
    result_alpha: i32,
    source_alpha: i32,
    destination: i32,
    source: i32,
) -> i32 {
    (source * source_alpha + destination * (result_alpha - source_alpha)) / result_alpha
}

fn argb_greyscale(color: i32) -> i32 {
    let greyscale = (argb_red(color) as f32 * 0.3
        + argb_green(color) as f32 * 0.59
        + argb_blue(color) as f32 * 0.11) as i32;
    argb_color(argb_alpha(color), greyscale, greyscale, greyscale)
}

fn sample_periodic_float_keyframes(day_time: i64, keyframes: &[(i64, f32)], period: i64) -> f32 {
    sample_periodic_keyframes(day_time, keyframes, period, |alpha, from, to| {
        from + alpha * (to - from)
    })
}

fn sample_periodic_argb_keyframes(day_time: i64, keyframes: &[(i64, i32)], period: i64) -> i32 {
    sample_periodic_keyframes(day_time, keyframes, period, argb_srgb_lerp)
}

fn sample_periodic_keyframes<T: Copy>(
    day_time: i64,
    keyframes: &[(i64, T)],
    period: i64,
    lerp: impl Fn(f32, T, T) -> T,
) -> T {
    debug_assert!(!keyframes.is_empty());
    if keyframes.len() == 1 {
        return keyframes[0].1;
    }

    let sample_tick = day_time.rem_euclid(period);
    let first = keyframes[0];
    let last = keyframes[keyframes.len() - 1];
    if sample_tick < first.0 {
        return sample_keyframe_segment(
            sample_tick,
            last.0 - period,
            last.1,
            first.0,
            first.1,
            &lerp,
        );
    }

    for window in keyframes.windows(2) {
        let from = window[0];
        let to = window[1];
        if sample_tick < to.0 {
            return sample_keyframe_segment(sample_tick, from.0, from.1, to.0, to.1, &lerp);
        }
    }

    sample_keyframe_segment(
        sample_tick,
        last.0,
        last.1,
        first.0 + period,
        first.1,
        &lerp,
    )
}

fn sample_keyframe_segment<T: Copy>(
    sample_tick: i64,
    from_tick: i64,
    from_value: T,
    to_tick: i64,
    to_value: T,
    lerp: &impl Fn(f32, T, T) -> T,
) -> T {
    if sample_tick <= from_tick {
        return from_value;
    }
    if sample_tick >= to_tick {
        return to_value;
    }
    let alpha = (sample_tick - from_tick) as f32 / (to_tick - from_tick) as f32;
    lerp(alpha, from_value, to_value)
}

fn sanitize_lightmap_brightness_factor(factor: f32) -> f32 {
    if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LocalPlayerLightmapEffects {
    brightness_factor: f32,
    darkness_scale: f32,
    night_vision_factor: f32,
}

fn local_player_lightmap_effects(
    brightness_factor: f32,
    camera_tick_count: u64,
    partial_tick: f32,
    darkness_effect_factor: f32,
    night_vision_effect: Option<MobEffectState>,
    conduit_power_effect: Option<MobEffectState>,
    water_vision: f32,
) -> LocalPlayerLightmapEffects {
    let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
    let base_brightness = sanitize_lightmap_brightness_factor(brightness_factor);
    let darkness_modifier =
        darkness_effect_factor.clamp(0.0, 1.0) * VANILLA_DARKNESS_EFFECT_SCALE_OPTION;
    let brightness_factor = (base_brightness - darkness_modifier).max(0.0);
    let darkness_scale = vanilla_darkness_scale(camera_tick_count, darkness_modifier, partial_tick)
        * VANILLA_DARKNESS_EFFECT_SCALE_OPTION;
    let water_vision = water_vision.clamp(0.0, 1.0);
    let night_vision_factor = if let Some(effect) = night_vision_effect {
        vanilla_night_vision_scale(effect, partial_tick)
    } else if water_vision > 0.0 && conduit_power_effect.is_some() {
        water_vision
    } else {
        0.0
    };

    LocalPlayerLightmapEffects {
        brightness_factor,
        darkness_scale,
        night_vision_factor,
    }
}

fn local_player_effect(world: &WorldStore, effect_id: i32) -> Option<MobEffectState> {
    world
        .local_player_id()
        .and_then(|player_id| world.entity_effect(player_id, effect_id))
}

fn camera_entity_id(world: &WorldStore) -> Option<i32> {
    let camera = world.local_player().camera;
    if camera.follows_player {
        world.local_player_id()
    } else {
        camera.entity_id
    }
}

fn camera_entity_effect(world: &WorldStore, effect_id: i32) -> Option<MobEffectState> {
    camera_entity_id(world).and_then(|entity_id| world.entity_effect(entity_id, effect_id))
}

fn camera_entity_is_living(world: &WorldStore) -> bool {
    camera_entity_id(world)
        .and_then(|entity_id| world.entity_type_id(entity_id))
        .is_some_and(vanilla_entity_type_is_living)
}

fn camera_entity_is_spectator(world: &WorldStore) -> bool {
    let camera = world.local_player().camera;
    if camera.follows_player {
        return world.local_player_is_spectator();
    }
    let Some(entity_id) = camera.entity_id else {
        return false;
    };
    world.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
        && world
            .entity_uuid(entity_id)
            .and_then(|uuid| world.player_info_entry(uuid))
            .is_some_and(|info| info.is_spectator())
}

fn camera_mob_effect_blocks_sky(world: &WorldStore) -> bool {
    camera_entity_effect(world, VANILLA_MOB_EFFECT_BLINDNESS_ID).is_some()
        || camera_entity_effect(world, VANILLA_MOB_EFFECT_DARKNESS_ID).is_some()
}

/// Vanilla `Gui.HeartType.forPlayer` (Gui.java:1438-1450): the base heart
/// sprite family, in precedence order poison > wither > fully-frozen > normal.
fn local_player_heart_type(world: &WorldStore) -> HudHeartKind {
    if local_player_effect(world, VANILLA_MOB_EFFECT_POISON_ID).is_some() {
        HudHeartKind::Poisoned
    } else if local_player_effect(world, VANILLA_MOB_EFFECT_WITHER_ID).is_some() {
        HudHeartKind::Withered
    } else if world.local_player_is_fully_frozen() {
        HudHeartKind::Frozen
    } else {
        HudHeartKind::Normal
    }
}

fn vanilla_night_vision_scale(effect: MobEffectState, partial_tick: f32) -> f32 {
    if !mob_effect_ends_within(effect, VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS) {
        1.0
    } else {
        0.7 + ((effect.duration_ticks as f32 - partial_tick) * std::f32::consts::PI * 0.2).sin()
            * 0.3
    }
}

fn vanilla_darkness_scale(camera_tick_count: u64, darkness_gamma: f32, partial_tick: f32) -> f32 {
    let darkness = 0.45 * darkness_gamma;
    (((camera_tick_count as f32 - partial_tick) * std::f32::consts::PI * 0.025).cos() * darkness)
        .max(0.0)
}

fn mob_effect_ends_within(effect: MobEffectState, ticks: i32) -> bool {
    effect.duration_ticks != -1 && effect.duration_ticks <= ticks
}

fn sanitize_lightmap_partial_tick(partial_tick: f32) -> f32 {
    if partial_tick.is_finite() {
        partial_tick.clamp(0.0, 1.0)
    } else {
        VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK
    }
}

fn apply_overworld_timeline_lightmap_environment(
    environment: &mut LightmapEnvironment,
    day_time: i64,
) {
    let sky_factor = sample_periodic_float_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SKY_LIGHT_FACTOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    let sky_light_color = sample_periodic_argb_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SKY_LIGHT_COLOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    environment.sky_factor *= sky_factor;
    environment.sky_light_color = rgb24(argb_multiply(
        rgb01_to_argb(environment.sky_light_color),
        sky_light_color,
    ));
}

fn apply_weather_lightmap_environment(
    environment: &mut LightmapEnvironment,
    weather: WorldWeatherState,
) {
    let rain_level = sanitize_weather_lightmap_level(weather.rain_level);
    let thunder_level =
        (sanitize_weather_lightmap_level(weather.thunder_level) * rain_level).clamp(0.0, 1.0);
    let rain_level = (rain_level - thunder_level).max(0.0);

    apply_weather_lightmap_layer(
        environment,
        rain_level,
        VANILLA_WEATHER_RAIN_ALPHA,
        VANILLA_WEATHER_RAIN_SKY_LIGHT_COLOR,
    );
    apply_weather_lightmap_layer(
        environment,
        thunder_level,
        VANILLA_WEATHER_THUNDER_ALPHA,
        VANILLA_WEATHER_THUNDER_SKY_LIGHT_COLOR,
    );
}

fn apply_weather_lightmap_layer(
    environment: &mut LightmapEnvironment,
    level: f32,
    alpha: f32,
    sky_light_color: i32,
) {
    if level <= 0.0 {
        return;
    }

    let modified_sky_factor = environment.sky_factor
        + alpha * (VANILLA_WEATHER_SKY_LIGHT_FACTOR - environment.sky_factor);
    environment.sky_factor += level * (modified_sky_factor - environment.sky_factor);

    let base_color = rgb01_to_argb(environment.sky_light_color);
    let modified_color = argb_alpha_blend(base_color, sky_light_color);
    environment.sky_light_color = rgb24(argb_srgb_lerp(level, base_color, modified_color));
}

/// Vanilla `Level.skyDarken`, sourced from the gameplay sky-light-level attribute rather than
/// the visual lightmap factor. Built-in timeline/weather layers are applied when registry data
/// does not provide the dimension timeline, matching `EnvironmentAttributeSystem` ordering.
pub(crate) fn entity_shadow_sky_darken(world: &WorldStore) -> u8 {
    let Some(level) = world.level_info() else {
        return 0;
    };
    let sampled_attributes = world
        .sampled_client_environment_attributes()
        .unwrap_or_else(|_| level.environment_attributes());
    let mut sky_light_level = sampled_attributes.sky_light_level.unwrap_or(15.0);
    let uses_builtin_dimension_fallback = world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_none();
    if vanilla_lightmap_dimension_kind(level) == VanillaLightmapDimensionKind::Overworld
        && uses_builtin_dimension_fallback
    {
        let day_time = world
            .world_time()
            .map(|time| time.day_time)
            .unwrap_or(VANILLA_LIGHTMAP_DEFAULT_DAY_TIME);
        sky_light_level *= sample_periodic_float_keyframes(
            day_time,
            &VANILLA_OVERWORLD_SKY_LIGHT_LEVEL_KEYFRAMES,
            VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
        );
    }
    if level.can_have_weather() {
        let weather = world.weather();
        let rain = sanitize_weather_lightmap_level(weather.rain_level);
        let thunder =
            (sanitize_weather_lightmap_level(weather.thunder_level) * rain).clamp(0.0, 1.0);
        let rain = (rain - thunder).max(0.0);
        sky_light_level =
            apply_weather_sky_light_level(sky_light_level, rain, VANILLA_WEATHER_RAIN_ALPHA);
        sky_light_level =
            apply_weather_sky_light_level(sky_light_level, thunder, VANILLA_WEATHER_THUNDER_ALPHA);
    }
    (15.0 - sky_light_level.clamp(0.0, 15.0))
        .trunc()
        .clamp(0.0, 15.0) as u8
}

fn apply_weather_sky_light_level(level: f32, weather_level: f32, alpha: f32) -> f32 {
    level + weather_level * alpha * (4.0 - level)
}

fn sanitize_weather_lightmap_level(level: f32) -> f32 {
    if level.is_finite() {
        level.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn random_between_inclusive(
    random: &mut LevelEventSoundRandomState,
    min: i32,
    max_inclusive: i32,
) -> i32 {
    random.next_int_bound(max_inclusive - min + 1) + min
}

fn dimension_lightmap_environment(
    level: &WorldLevelInfo,
    attributes: bbb_world::ClientEnvironmentAttributes,
) -> LightmapEnvironment {
    let mut environment = LightmapEnvironment::default();
    match vanilla_lightmap_dimension_kind(level) {
        VanillaLightmapDimensionKind::Nether => {
            environment.sky_factor = 0.0;
            environment.sky_light_color = VANILLA_NETHER_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_NETHER_AMBIENT_LIGHT_COLOR;
            environment.level_lighting = LevelLighting::Nether;
        }
        VanillaLightmapDimensionKind::End => {
            environment.sky_factor = 0.0;
            environment.sky_light_color = VANILLA_END_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_END_AMBIENT_LIGHT_COLOR;
        }
        VanillaLightmapDimensionKind::Overworld => {
            environment.sky_factor = VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR;
            environment.sky_light_color = VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR;
            environment.ambient_color = VANILLA_OVERWORLD_AMBIENT_LIGHT_COLOR;
        }
        VanillaLightmapDimensionKind::Other => {}
    }
    if let Some(value) = attributes.block_light_tint {
        environment.block_light_tint = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.sky_light_color {
        environment.sky_light_color = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.sky_light_factor {
        environment.sky_factor = value;
    }
    if let Some(value) = attributes.ambient_light_color {
        environment.ambient_color = rgb24(rgb_u8_to_argb(value));
    }
    if let Some(value) = attributes.night_vision_color {
        environment.night_vision_color = rgb24(rgb_u8_to_argb(value));
    }
    environment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VanillaLightmapDimensionKind {
    Overworld,
    Nether,
    End,
    Other,
}

fn vanilla_lightmap_dimension_kind(level: &WorldLevelInfo) -> VanillaLightmapDimensionKind {
    if let Some(dimension_type) = level.dimension_type_name.as_deref() {
        return match dimension_type {
            "minecraft:the_nether" => VanillaLightmapDimensionKind::Nether,
            "minecraft:the_end" => VanillaLightmapDimensionKind::End,
            "minecraft:overworld" | "minecraft:overworld_caves" => {
                VanillaLightmapDimensionKind::Overworld
            }
            _ => VanillaLightmapDimensionKind::Other,
        };
    }
    let dimension = level.dimension.as_str();
    match (level.dimension_type_id, dimension) {
        (1, _) | (_, "minecraft:the_nether") => VanillaLightmapDimensionKind::Nether,
        (2, _) | (_, "minecraft:the_end") => VanillaLightmapDimensionKind::End,
        (0, _) | (3, _) | (_, "minecraft:overworld") | (_, "minecraft:overworld_caves") => {
            VanillaLightmapDimensionKind::Overworld
        }
        _ => VanillaLightmapDimensionKind::Other,
    }
}

pub(crate) fn request_net_disconnect(
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    requested: &mut bool,
) {
    if *requested {
        return;
    }
    *requested = true;
    if let Some(tx) = net_commands {
        let _ = tx.try_send(NetCommand::Disconnect);
    }
}

pub(crate) fn audio_registry_snapshot_from_world(
    world: &WorldStore,
) -> (bbb_pack::SoundEventRegistry, bbb_pack::JukeboxSongRegistry) {
    events::audio_registry_snapshot_from_world(world)
}

fn queue_client_tick_end_commands(
    world: &WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    client_ticks: u32,
) {
    if world.local_player_id().is_none() {
        return;
    }
    let Some(tx) = net_commands else {
        return;
    };
    for _ in 0..client_ticks {
        // The render thread must never park on the command channel: under a
        // saturated channel the connection is already unusable, and a dropped
        // tick-end is harmless while a blocked render thread would stop
        // draining net events entirely.
        if tx.try_send(NetCommand::ClientTickEnd).is_err() {
            break;
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SimulationProfileSection {
    pub(crate) path: &'static [&'static str],
    pub(crate) duration: Duration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NativeRenderProfile {
    pub(crate) extract: Duration,
    pub(crate) toasts: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct SimulationFrameState {
    pump_started_at: Instant,
    pump_net_drained_at: Instant,
    pump_control_synced_at: Instant,
    pump_client_ticked_at: Instant,
    now: Instant,
    advanced_ticks: u32,
    camera_spinning_ticks: CameraSpinningEffectTickState,
    lightmap_environment: LightmapEnvironment,
    water_vision: f32,
    rain_fog_multiplier: f32,
    hide_lightning_flash: bool,
    cloud_environment: CloudEnvironment,
    particle_scope_context: Option<ParticleLocalPlayerScopeContext>,
    particle_player_motion_contexts: Vec<ParticlePlayerMotionContext>,
    particle_entity_target_contexts: Vec<ParticleEntityTargetContext>,
    particle_sound_camera_position: Option<[f64; 3]>,
    terrain_textures_changed: bool,
    profiling_sections: Vec<SimulationProfileSection>,
}

impl SimulationFrameState {
    pub(crate) fn tick_anchor(&self) -> Instant {
        self.now
    }

    pub(crate) fn advanced_ticks(&self) -> u32 {
        self.advanced_ticks
    }

    pub(crate) fn profiling_sections(&self) -> &[SimulationProfileSection] {
        &self.profiling_sections
    }
}

fn optional_profile_elapsed(started_at: Option<Instant>) -> Duration {
    started_at.map_or(Duration::ZERO, |started_at| started_at.elapsed())
}

fn push_simulation_profile_section(
    sections: &mut Vec<SimulationProfileSection>,
    path: &'static [&'static str],
    started_at: Option<Instant>,
) {
    if let Some(started_at) = started_at {
        sections.push(SimulationProfileSection {
            path,
            duration: started_at.elapsed(),
        });
    }
}

fn push_simulation_profile_duration(
    sections: &mut Vec<SimulationProfileSection>,
    path: &'static [&'static str],
    duration: Duration,
    enabled: bool,
) {
    if enabled {
        sections.push(SimulationProfileSection { path, duration });
    }
}

pub(crate) fn advance_simulation(
    net_events: &mut Option<mpsc::Receiver<NetEvent>>,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    server_resource_packs: &mut ServerResourcePackRuntime,
    services_block_lists: &mut crate::services_block_list_runtime::ServicesBlockListRuntime,
    audio_events: Option<&mut dyn AudioEventSink>,
    particle_runtime: &mut Option<NativeParticleRuntime>,
    particle_batches: &mut dyn ParticleBatchSink,
    input: &mut ClientInputState,
    first_person_hand_presentation: &mut FirstPersonHandPresentation,
    world: &mut WorldStore,
    net_counters: &mut NetCounters,
    play_session_encrypted: &mut bool,
    client_animation_ticks: &mut ClientAnimationTickState,
    camera_fov_ticks: &mut CameraFovTickState,
    camera_eye_height_ticks: &mut CameraEyeHeightTickState,
    camera_spinning_ticks: &mut CameraSpinningEffectTickState,
    hud_debug_network_sampler: &mut HudDebugNetworkSampler,
    hud_debug_tps_sampler: &mut HudDebugTpsSampler,
    lightmap_ticks: &mut LightmapTickState,
    level_event_sound_random: &mut LevelEventSoundRandomState,
    environment_audio: &mut crate::environment_audio_runtime::EnvironmentAudioRuntime,
    environment_animate: &mut crate::environment_animate_runtime::EnvironmentAnimateRuntime,
    weather_effects: &mut crate::weather_effect_runtime::WeatherEffectRuntime,
    music: &mut MusicRuntime,
    win_screen_resources: &mut Arc<crate::win_screen_resources::WinScreenResources>,
    terrain_textures: &mut TerrainTextureState,
    item_runtime: &mut Option<NativeItemRuntime>,
    code_of_conduct: &mut CodeOfConductAcceptance,
    server_list: &mut Option<crate::server_list::ServerListPersistence>,
    commands: Vec<crate::simulation::commands::SequencedSimulationCommand>,
    input_runtime: &mut crate::simulation::input_commands::SimulationInputRuntime,
    level_load_runtime: &mut LevelLoadRuntime,
    platform_effects: &mut crate::simulation::effects::PlatformEffectCollector<'_>,
    hide_lightning_flash: bool,
    now: Instant,
) -> SimulationFrameState {
    let pump_started_at = Instant::now();
    let wall_clock_epoch_seconds = wall_clock_millis_i64().div_euclid(1_000);
    let mut audio_events = audio_events;
    let input_screen_was_open = input_screen_is_open(input, world);
    let mut remote_disconnect = None;
    if let Some(rx) = net_events.as_mut() {
        let audio_events_for_drain = audio_events
            .as_mut()
            .map(|audio_events| &mut **audio_events as &mut dyn AudioEventSink);
        let particle_events_for_drain = particle_runtime
            .as_mut()
            .map(|particle_events| particle_events as &mut dyn ParticleEventSink);
        events::drain_net_events_with_sinks_and_debug(
            rx,
            world,
            net_counters,
            play_session_encrypted,
            net_commands,
            audio_events_for_drain,
            particle_events_for_drain,
            Some(&mut *particle_batches),
            item_runtime.as_ref(),
            Some(hud_debug_network_sampler),
            Some(hud_debug_tps_sampler),
            server_list.as_mut(),
            level_event_sound_random,
            Some(&mut remote_disconnect),
            Some(services_block_lists),
            wall_clock_epoch_seconds,
        );
    }
    if net_events
        .as_ref()
        .is_some_and(|events| events.is_closed() && events.is_empty())
    {
        *net_events = None;
    }
    if let Some(reason) = remote_disconnect {
        let listener_state = net_counters.state.as_deref();
        let title = disconnected_screen_title_for_state(listener_state);
        let stop_audio = remote_disconnect_stops_audio(listener_state);
        world.end_connection_session();
        input.open_disconnected_screen(
            title,
            crate::input::disconnected_screen::DisconnectedScreenDetails {
                reason,
                report_directory: None,
                bug_report_link: None,
            },
        );
        if stop_audio {
            net_counters.simulation_remote_disconnect_audio_stop_requests = net_counters
                .simulation_remote_disconnect_audio_stop_requests
                .saturating_add(1);
            if let Some(audio_events) = audio_events.as_deref_mut() {
                audio_events.stop_all_sounds();
            }
        }
        input_runtime.request_terminal_screen_cursor(platform_effects);
    }
    world.expire_game_test_highlight_markers_at(monotonic_millis());
    world.poll_queued_light_updates();
    let pump_net_drained_at = Instant::now();
    server_resource_packs.synchronize(world, net_commands, net_counters.connected);
    input.synchronize_camera_post_effect(
        world.client_level_generation(),
        world.counters().set_camera_updates_applied,
        current_post_effect_id_from_world(world),
    );
    input.sync_dialog_native_previous_screen(world, net_counters, net_commands);
    input.sync_game_event_screen(world, net_counters, net_commands);
    input.sync_death_screen(world, net_counters, net_commands);
    input.sync_player_reporting_screen(world);
    services_block_lists.synchronize_player_report_submissions(input, world);
    input.sync_social_interactions_screen(world);
    input.sync_sign_editor(world, net_counters, net_commands);
    input.sync_book_screen(world);
    input.sync_command_block_editor(world, net_counters, net_commands);
    input.sync_minecart_command_block_editor(world, net_counters, net_commands);
    input.sync_structure_block_editor(world, net_counters, net_commands);
    input.sync_jigsaw_block_editor(world, net_counters, net_commands);
    input.sync_test_block_editor(world, net_counters, net_commands);
    input.sync_test_instance_block_editor(world, net_counters, net_commands);
    let _ = hud_game_rules_screen(
        input,
        world,
        item_runtime.as_ref(),
        input_runtime.surface_size(),
    );
    let terrain_textures_changed = terrain_textures.synchronize_biome_registry(world);
    if terrain_textures_changed {
        if let Some(particle_events) = particle_runtime.as_mut() {
            particle_events.synchronize_terrain_textures(terrain_textures);
        }
    }
    release_input_if_screen_opened(
        input_screen_was_open,
        input,
        world,
        net_counters,
        net_commands,
    );
    let input_profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let input_profile = input_runtime.apply_commands(
        commands,
        &mut crate::simulation::input_commands::SimulationInputContext {
            input,
            first_person_hand_presentation,
            world,
            net_counters,
            net_commands,
            server_resource_packs,
            services_block_lists,
            particle_runtime,
            item_runtime,
            terrain_textures,
            code_of_conduct: Some(code_of_conduct),
            win_screen_resources,
            entity_partial_tick: client_animation_ticks.entity_partial_tick(),
            level_random: level_event_sound_random,
            audio_events: &mut audio_events,
            platform: platform_effects,
        },
        input_profiling_enabled,
    );
    services_block_lists.tick(wall_clock_epoch_seconds);
    services_block_lists.synchronize_player_report_submissions(input, world);
    sync_stonecutter_recipe_scroll_state(input, world);
    sync_beacon_effect_selection_state(input, world);
    sync_loom_pattern_state_for_hud(input, world);
    input.sync_creative_inventory_presentation(world, item_runtime.as_ref());
    let advancement_scroll_delta =
        input.advancement_scroll_delta(world.selected_advancements_tab());
    advancement_hover_fade_for_hud(
        input,
        world,
        advancement_scroll_delta,
        input_runtime.surface_size(),
    );
    let _ = hud_stats_screen(
        input,
        world,
        item_runtime.as_ref(),
        terrain_textures,
        input.item_model_keybind_context(),
        client_animation_ticks.entity_partial_tick(),
        input_runtime.surface_size(),
    );
    let pump_control_synced_at = Instant::now();
    let profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let mut profiling_sections = Vec::new();
    let level_services_started_at = profiling_enabled.then(Instant::now);
    let player_section_ready = input_runtime.player_section_ready(world.client_level_generation());
    level_load_runtime.tick(now, world, net_commands, player_section_ready);
    hud_debug_network_sampler.advance_tick(
        input,
        net_counters,
        net_commands,
        now,
        wall_clock_millis_i64(),
    );
    hud_debug_tps_sampler.sync_subscriptions_and_tick(input, world, net_counters, net_commands);
    let mut level_profile_duration = optional_profile_elapsed(level_services_started_at);
    let entities_profile_started_at = profiling_enabled.then(Instant::now);
    let entity_tick_started_at = Instant::now();
    let advanced_ticks = advance_native_client_ticks(input, world, client_animation_ticks, now);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "entities"],
        entities_profile_started_at,
    );
    let gui_profile_started_at = profiling_enabled.then(Instant::now);
    sync_recipe_book_presentation(
        input,
        world,
        net_counters,
        net_commands,
        item_runtime.as_ref(),
        advanced_ticks,
    );
    if advanced_ticks > 0 {
        record_entity_client_tick_observation(
            net_counters,
            entity_tick_started_at.elapsed(),
            client_animation_ticks.last_entity_tick_work,
        );
    }
    let running_ticks = client_animation_ticks.last_running_level_ticks;
    let entity_partial_tick = client_animation_ticks.entity_partial_tick();
    camera_eye_height_ticks.advance_for_world(advanced_ticks, world);
    world.advance_tickable_entity_sounds(advanced_ticks, running_ticks);
    // Vanilla `Minecraft.tick` calls `Gui.tick` once per client tick (Gui.java:
    // 1145-1166), outside the tick-rate manager's freeze gate, so the action
    // bar / title countdowns advance on raw client ticks, not running ticks.
    world.advance_hud_text_ticks(advanced_ticks);
    world.advance_chat_gui_ticks(advanced_ticks);
    for _ in 0..advanced_ticks {
        for acknowledgement in world.advance_chat_listener(monotonic_millis(), false) {
            events::queue_chat_acknowledgement(net_commands, net_counters, acknowledgement);
        }
    }
    world.advance_dialog_waiting_ticks(advanced_ticks);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gui"],
        gui_profile_started_at,
    );
    let level_environment_started_at = profiling_enabled.then(Instant::now);
    lightmap_ticks.advance_for_world(advanced_ticks, world);
    lightmap_ticks.advance_rain_fog_for_world(advanced_ticks, world, terrain_textures);
    let water_vision = lightmap_ticks.water_vision(world);
    let rain_fog_multiplier = lightmap_ticks.rain_fog_multiplier();
    // Vanilla `Minecraft.tick` runs `ClientLevel.tick` before `GameRenderer.extract`;
    // `ClientLevel.tick` decrements `skyFlashTime`, and the render extract reads
    // the resulting EnvironmentAttributes / lightmap state.
    world.advance_sky_flash_time(advanced_ticks);
    let lightmap_environment = lightmap_ticks.environment_for_world(world);
    let render_camera_pose = render_camera_pose_from_world_at_partial_tick(
        world,
        entity_partial_tick,
        input.camera_type(),
        *camera_eye_height_ticks,
    );
    let cloud_environment = cloud_environment_for_world(world);
    // Vanilla `GameRenderer.tick` calls `LevelRenderer.tick` before entity and
    // block-entity ticks. Inside `LevelRenderer.tick`, weather sampling precedes
    // stale block-breaking cleanup (`LevelRenderer.java:1148-1167`). Canonical
    // entity mutation is already batched above, but submit these client-local
    // effects before the later entity/animate effect drains to preserve the
    // observable effect order.
    weather_effects.advance_for_ticks(
        world,
        running_ticks,
        render_camera_pose.map(|camera| camera_eye_position(camera).map(f64::from)),
        particle_runtime
            .as_mut()
            .map(|runtime| runtime as &mut dyn ParticleEventSink),
        particle_batches,
        &mut audio_events,
        item_runtime.as_ref(),
    );
    advance_block_destruction_render_ticks(world, running_ticks);
    level_profile_duration = level_profile_duration
        .saturating_add(optional_profile_elapsed(level_environment_started_at));
    push_simulation_profile_duration(
        &mut profiling_sections,
        &["tick", "level"],
        level_profile_duration,
        profiling_enabled,
    );
    let block_entities_profile_started_at = profiling_enabled.then(Instant::now);
    // Vanilla `ChestBlockEntity.lidAnimateTick` is a client block-entity ticker
    // (`ClientLevel.tickBlockEntities`), gated by the tick-rate manager like the
    // rest of the level tick, so the lids advance on running ticks.
    world.advance_chest_lid_ticks(running_ticks);
    // Vanilla `BellBlockEntity.clientTick` is the same kind of client
    // block-entity ticker, so the bell shakes advance on running ticks too.
    world.advance_bell_shake_ticks(running_ticks);
    // Vanilla `ShulkerBoxBlockEntity.tick` (updateAnimation) is the same kind
    // of client block-entity ticker, so the box lids advance on running ticks.
    world.advance_shulker_box_lid_ticks(running_ticks);
    // The decorated pot wobble clock is vanilla `gameTime -
    // wobbleStartedAtTick`, re-expressed as running ticks since the block event.
    world.advance_decorated_pot_wobble_ticks(running_ticks);
    // Vanilla `EnchantingTableBlockEntity.bookAnimationTick` is another client
    // block-entity ticker, so the hovering books advance on running ticks too
    // (reconciled every call so freshly loaded tables gain state).
    world.advance_enchanting_table_book_ticks(running_ticks);
    // Vanilla `ConduitBlockEntity.clientTick` is another client block-entity
    // ticker: it advances tickCount/activeRotation and refreshes the water +
    // prismarine frame every 40 game ticks.
    world.advance_conduit_ticks(running_ticks);
    // Vanilla `TheEndGatewayBlockEntity.beamAnimationTick` advances age and
    // cooldown on the client block-entity ticker.
    world.advance_end_gateway_ticks(running_ticks);
    // Vanilla `SkullBlockEntity.animation` is the client ticker for powered
    // dragon/piglin skull/head blocks.
    world.advance_skull_block_ticks(running_ticks);
    // Vanilla `BaseSpawner.clientTick` advances the ordinary spawner's display
    // entity spin only while a nearby player exists.
    world.advance_spawner_block_ticks(running_ticks);
    // Vanilla `TrialSpawner.tickClient` advances the cached display entity's
    // state-dependent spin independently of the ordinary nearby-player gate.
    world.advance_trial_spawner_block_ticks(running_ticks);
    // Vanilla `VaultBlockEntity.Client.tick` advances display-item spin before
    // its local particles and ambient sound.
    world.advance_vault_block_ticks(running_ticks);
    // `PistonMovingBlockEntity.tick` is a client block-entity ticker too: it
    // advances only with running level ticks, before scene extraction samples
    // the previous/current progress pair at the frame partial tick.
    world.advance_moving_piston_ticks(running_ticks);
    environment_animate.advance_block_entity_effects_for_ticks(
        world,
        net_counters.connection_generation,
        running_ticks,
        particle_runtime
            .as_mut()
            .map(|runtime| runtime as &mut dyn ParticleEventSink),
        particle_batches,
        &mut audio_events,
        item_runtime.as_ref(),
    );
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "blockEntities"],
        block_entities_profile_started_at,
    );
    let screen_profile_started_at = profiling_enabled.then(Instant::now);
    world.advance_item_cooldowns(advanced_ticks);
    input.advance_death_screen_ticks(advanced_ticks);
    // Vanilla `EnchantmentScreen.containerTick` advances its screen-local
    // book once per raw client tick, alongside the other active screen ticks.
    input.advance_enchantment_screen(world, advanced_ticks);
    advance_creative_inventory_commands(input, world, net_counters, net_commands, advanced_ticks);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "screen"],
        screen_profile_started_at,
    );
    let keybindings_profile_started_at = profiling_enabled.then(Instant::now);
    advance_player_input_for_client_ticks(
        input,
        world,
        net_counters,
        net_commands,
        now,
        advanced_ticks,
    );
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "Keybindings"],
        keybindings_profile_started_at,
    );
    let animate_tick_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        // `LocalPlayer.tick` advances underwater, bubble-column, and biome
        // ambient handlers after player movement. Player entities keep ticking
        // while the level is frozen, so these follow raw local-player ticks.
        environment_audio.advance_for_ticks(
            world,
            net_counters.connection_generation,
            advanced_ticks,
            audio_events,
        );
    } else {
        // Audio may be disabled by a headless/test platform. Transition edges
        // are per-tick effects and must not replay if a sink appears later.
        world.take_pending_local_player_ambient_audio_ticks();
    }
    environment_animate.advance_for_ticks(
        world,
        net_counters.connection_generation,
        running_ticks,
        particle_runtime
            .as_mut()
            .map(|runtime| runtime as &mut dyn ParticleEventSink),
        particle_batches,
        &mut audio_events,
        item_runtime.as_ref(),
    );
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "animateTick"],
        animate_tick_profile_started_at,
    );
    let music_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        // `Minecraft.tick` advances `MusicManager` on every raw client tick
        // after the running-level animate loop.
        music.advance_for_ticks(
            world,
            net_counters.connection_generation,
            advanced_ticks,
            audio_events,
        );
    }
    let mut sound_profile_duration = optional_profile_elapsed(music_profile_started_at);
    let game_renderer_profile_started_at = profiling_enabled.then(Instant::now);
    // Vanilla Minecraft.tick handles keybinds before GameRenderer.tick, whose
    // Camera.tickFov eases the current camera's target once per client tick.
    camera_fov_ticks.advance_for_world(
        advanced_ticks,
        world,
        input.camera_type().is_first_person(),
        world.local_player_is_sprinting(),
    );
    camera_spinning_ticks.advance_for_world(advanced_ticks, world);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gameRenderer"],
        game_renderer_profile_started_at,
    );
    let game_mode_profile_started_at = profiling_enabled.then(Instant::now);
    let audio_events_for_destroy = audio_events
        .as_mut()
        .map(|audio_events| &mut **audio_events as &mut dyn AudioEventSink);
    advance_destroying_block_at_partial_tick(
        input,
        world,
        net_counters,
        net_commands,
        audio_events_for_destroy,
        entity_partial_tick,
        advanced_ticks,
    );
    submit_local_player_action_particles(particle_batches, world, input, particle_runtime.as_mut());
    advance_using_item_at_partial_tick_with_effects_and_presentation(
        input,
        first_person_hand_presentation,
        world,
        net_counters,
        net_commands,
        entity_partial_tick,
        advanced_ticks,
        level_event_sound_random,
        &mut audio_events,
    );
    advance_local_using_item_ticks_with_effects(
        world,
        net_counters,
        net_commands,
        advanced_ticks,
        level_event_sound_random,
        &mut audio_events,
    );
    for _ in 0..advanced_ticks {
        if let Some(CrosshairTarget::Block(hit)) = crosshair_target_from_camera_at_partial_tick(
            world,
            camera_pose_from_world(world),
            entity_partial_tick,
        ) {
            world.tutorial_on_look_at_block(hit.pos);
        }
        world.advance_tutorial_ticks(1);
    }
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "gameMode"],
        game_mode_profile_started_at,
    );
    let particles_profile_started_at = profiling_enabled.then(Instant::now);
    first_person_hand_presentation.advance_for_ticks(
        world,
        item_runtime.as_ref(),
        local_player_hands_are_busy(input, world),
        advanced_ticks,
    );
    let particle_camera_pose = render_camera_pose;
    let particle_scope_context = particle_local_player_scope_context(
        world,
        particle_camera_pose,
        input.camera_type().is_first_person(),
    );
    let particle_sound_camera_position =
        particle_camera_pose.map(|camera| camera_eye_position(camera).map(f64::from));
    let particle_player_motion_contexts = particle_player_motion_contexts(world);
    let particle_entity_target_contexts = particle_entity_target_contexts(world);
    if advanced_ticks > 0 {
        if let Some(particle_runtime) = particle_runtime.as_mut() {
            particle_batches.submit_particle_batch(particle_runtime.tick_explosion_particles(
                world,
                level_event_sound_random,
                particle_sound_camera_position,
                item_runtime.as_ref(),
            ));
        }
    }
    submit_primed_tnt_smoke_particles(particle_batches, world, advanced_ticks);
    let mut particle_events = particle_runtime
        .as_mut()
        .map(|runtime| runtime as &mut dyn ParticleEventSink);
    submit_entity_client_tick_effects(
        particle_batches,
        world,
        &mut particle_events,
        &mut audio_events,
    );
    submit_ominous_item_spawner_particles(particle_batches, world, &mut particle_events);
    push_simulation_profile_section(
        &mut profiling_sections,
        &["tick", "particles"],
        particles_profile_started_at,
    );
    let sound_profile_started_at = profiling_enabled.then(Instant::now);
    if let Some(audio_events) = audio_events.as_deref_mut() {
        audio_events.tick_entity_sound_positions(
            audio_scene_command_from_world(world),
            &world.tickable_entity_sounds(),
        );
    }
    sound_profile_duration =
        sound_profile_duration.saturating_add(optional_profile_elapsed(sound_profile_started_at));
    push_simulation_profile_duration(
        &mut profiling_sections,
        &["sound"],
        sound_profile_duration,
        profiling_enabled,
    );
    // Vanilla sends the marker after logical particle spawning. Backend
    // particle execution consumes the owned batches on the render side and
    // must not delay the connection command.
    queue_client_tick_end_commands(world, net_commands, advanced_ticks);
    for narration in world.take_pending_chat_narrations() {
        let text = chat_narration_text(&narration, item_runtime.as_ref());
        if text.is_empty() {
            continue;
        }
        platform_effects.submit(crate::simulation::effects::PlatformEffect::ChatNarration {
            kind: narration.kind,
            text,
        });
    }
    let pump_client_ticked_at = Instant::now();
    if profiling_enabled {
        if !input_profile.keyboard.is_zero() {
            profiling_sections.push(SimulationProfileSection {
                path: &["tick", "keyboard"],
                duration: input_profile.keyboard,
            });
        }
        if !input_profile.mouse.is_zero() {
            profiling_sections.push(SimulationProfileSection {
                path: &["mouse"],
                duration: input_profile.mouse,
            });
        }
        profiling_sections.extend([
            SimulationProfileSection {
                path: &["scheduledPacketProcessing"],
                duration: pump_net_drained_at - pump_started_at,
            },
            SimulationProfileSection {
                path: &["scheduledExecutables"],
                duration: (pump_control_synced_at - pump_net_drained_at)
                    .saturating_sub(input_profile.keyboard)
                    .saturating_sub(input_profile.mouse),
            },
            SimulationProfileSection {
                path: &["tick"],
                duration: (pump_client_ticked_at - pump_control_synced_at)
                    .saturating_sub(sound_profile_duration),
            },
        ]);
    }
    SimulationFrameState {
        pump_started_at,
        pump_net_drained_at,
        pump_control_synced_at,
        pump_client_ticked_at,
        now,
        advanced_ticks,
        camera_spinning_ticks: *camera_spinning_ticks,
        lightmap_environment,
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
        cloud_environment,
        particle_scope_context,
        particle_player_motion_contexts,
        particle_entity_target_contexts,
        particle_sound_camera_position,
        terrain_textures_changed,
        profiling_sections,
    }
}

fn disconnected_screen_title_for_state(
    state: Option<&str>,
) -> crate::input::disconnected_screen::DisconnectedScreenTitle {
    if matches!(state, Some("Configuration" | "Play")) {
        crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost
    } else {
        crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectFailed
    }
}

fn remote_disconnect_stops_audio(state: Option<&str>) -> bool {
    matches!(state, Some("Configuration" | "Play"))
}

pub(crate) fn update_renderer_from_publication(
    publication: &SimulationPublication,
    context: SimulationRenderContext<'_>,
) -> NativeRenderProfile {
    let profile_started_at = Instant::now();
    let SimulationRenderContext {
        audio_events,
        renderer,
        mailbox,
        terrain_upload,
        terrain_textures,
        entity_variant_textures,
        entity_render_extraction,
        non_entity_render_extraction,
        item_runtime,
        hud_render_runtime,
        hud_debug_fps_sampler,
        debug_collision_cache,
        debug_supporting_block_cache,
        debug_sky_light_section_cache,
        remote_debug_selection_cache,
        render_distance_chunks,
        cloud_range_chunks,
        client_framerate_limit,
        client_vsync,
        damage_tilt_strength,
        view_bobbing,
        entity_shadows,
        hud_debug_profiler_chart,
    } = context;
    let state = publication
        .frame
        .as_ref()
        .expect("rendered simulation publication must contain a frame");
    let pump_started_at = state.pump_started_at;
    let pump_net_drained_at = state.pump_net_drained_at;
    let pump_control_synced_at = state.pump_control_synced_at;
    let pump_client_ticked_at = state.pump_client_ticked_at;
    let now = state.now;
    let advanced_ticks = state.advanced_ticks;
    let camera_spinning_ticks = state.camera_spinning_ticks;
    let lightmap_environment = state.lightmap_environment;
    let water_vision = state.water_vision;
    let rain_fog_multiplier = state.rain_fog_multiplier;
    let hide_lightning_flash = state.hide_lightning_flash;
    let cloud_environment = state
        .cloud_environment
        .with_range_chunks(cloud_range_chunks);
    let particle_scope_context = state.particle_scope_context;
    let particle_player_motion_contexts = &state.particle_player_motion_contexts;
    let particle_entity_target_contexts = &state.particle_entity_target_contexts;
    let particle_sound_camera_position = state.particle_sound_camera_position;
    let terrain_textures_changed = state.terrain_textures_changed;
    let input = publication.input.as_state();
    let profiling_enabled =
        input.debug_profiler_chart_visible() || input.debug_profiling_recording();
    let world = publication.world.as_ref();
    let stable_world_unchanged =
        non_entity_render_extraction.begin_stable_world_frame(&publication.world);
    if non_entity_render_extraction
        .synchronize_map_texture_level(publication.client_level_generation)
    {
        renderer.invalidate_map_texture_residency();
    }
    debug_assert_eq!(publication.terrain_revision, world.terrain_revision());
    debug_assert_eq!(publication.frozen, world.ticking().frozen);
    let surface_size = publication.input_runtime.surface_size;
    let net_counters = &publication.net_counters;
    let audio_status = &publication.audio_counters;
    let camera_fov_ticks = publication.camera_fov_ticks;
    let camera_eye_height_ticks = publication.camera_eye_height_ticks;
    let hud_debug_network_sampler = &publication.hud_debug_network_sampler;
    let hud_debug_tps_sampler = &publication.hud_debug_tps_sampler;
    let render_started_at = Instant::now();
    let entity_partial_tick = publication.partial_tick_at(render_started_at);
    let hud_frame_delta =
        hud_render_runtime.frame_delta_ticks(render_started_at, publication.tick_interval);
    let panorama_rotation_degrees = hud_render_runtime.panorama_rotation_degrees(
        input.disconnected_screen_runtime().is_some(),
        hud_frame_delta,
    );
    let win_generation = match input.game_event_screen_runtime() {
        Some(GameEventScreenRuntime::Win(screen)) => Some(screen.generation),
        _ => None,
    };
    if let Some((generation, delta_ticks)) =
        HudRenderRuntime::win_screen_realtime_delta_ticks(win_generation, hud_frame_delta)
    {
        mailbox.enqueue_or_report(
            crate::simulation::commands::SimulationCommand::GameEventScreenRenderDelta {
                generation,
                delta_ticks,
            },
            "game event screen render delta",
        );
    }
    hud_debug_fps_sampler.record_frame(render_started_at);
    let mut audio_events = audio_events;
    entity_variant_textures
        .synchronize_session(net_counters.connection_generation, net_counters.connected);
    entity_variant_textures.synchronize(world, renderer);
    let render_terrain_textures_changed = terrain_textures.synchronize_biome_registry(world);
    if terrain_textures_changed || render_terrain_textures_changed {
        terrain_upload.request_reload_all_chunks();
    }
    // Vanilla `Minecraft.tick` handles gameplay input before `ParticleEngine.tick`; render
    // extraction samples light from the particle positions advanced here. Player-coupled
    // particles sample the same post-input local player state (plus the current remote
    // player transforms as nearest-player candidates) during particle tick.
    renderer.advance_particles_with_world_and_particle_contexts_and_sound_camera(
        advanced_ticks,
        |query| {
            world.clip_particle_collision_movement(
                query.position,
                query.movement,
                query.half_width,
                query.height,
            )
        },
        |query| renderer_particle_block_fluid_surface_sample(world, query.position),
        particle_scope_context,
        particle_player_motion_contexts,
        particle_entity_target_contexts,
        particle_sound_camera_position,
    );
    let particle_sound_events = renderer.drain_particle_sound_events();
    emit_particle_sound_events(&mut audio_events, particle_sound_events);
    // Vanilla handles gameplay keybinds during `Minecraft.tick`, then `GameRenderer.extractGui`
    // calls `Gui.extractRenderState`; HUD values therefore read after input and use-item updates.
    let hud_started_at = Instant::now();
    let local_player = world.local_player();
    // Vanilla reaches `Gui.extractPlayerHealth` only while the GUI is visible
    // and `GameMode.canHurtPlayer()` is true (survival/adventure). The synced
    // health then pairs with the MAX_HEALTH attribute, absorption, hardcore,
    // `HeartType.forPlayer`, Regeneration, and the client tick.
    let local_player_invulnerable_time = world
        .local_player_id()
        .and_then(|entity_id| world.entity_damage_invulnerable_time(entity_id))
        .unwrap_or(0);
    let game_mode_can_hurt_player = matches!(
        GameType::from_id(world.gameplay().game_type),
        GameType::Survival | GameType::Adventure
    );
    let hud_player_health = (!input.hide_gui() && game_mode_can_hurt_player)
        .then_some(local_player.health)
        .flatten()
        .filter(|health| health.health.is_finite())
        .map(|health| {
            let presentation = hud_render_runtime.player_health_frame(
                render_started_at,
                publication.client_tick_index,
                health.health.ceil() as i32,
                local_player_invulnerable_time,
            );
            HudPlayerHealth {
                health: health.health,
                display_health: presentation.display_health,
                blinking: presentation.blinking,
                max_health: world.local_player_max_health(),
                absorption: world.local_player_absorption(),
                heart_type: local_player_heart_type(world),
                hardcore: world.is_hardcore(),
                regen: local_player_effect(world, VANILLA_MOB_EFFECT_REGENERATION_ID).is_some(),
                tick_count: publication.client_tick_index,
            }
        });
    // Unlike ordinary HUD content, Vanilla extracts the sleep overlay outside
    // the `Options.hideGui` branch and does not partial-tick interpolate it.
    let hud_sleep_overlay = hud_sleep_overlay_from_world(world);
    // `Gui.extractHotbarAndDecorations` reaches `extractFood` only through
    // `GameMode.canHurtPlayer`; spectator/creative HUDs must not retain the
    // ordinary food row merely because synced health is still present.
    let hud_food = game_mode_can_hurt_player
        .then_some(local_player.health)
        .flatten()
        .map(|health| health.food);
    // Vanilla `Gui.extractArmor` reads `player.getArmorValue()` (Gui.java:799),
    // the floor of the ARMOR attribute; the renderer gates the row on `armor > 0`.
    let hud_armor = Some(world.local_player_armor_value());
    // Vanilla `Gui.extractAirBubbles` inputs (Gui.java:887-890): the synced
    // `DATA_AIR_SUPPLY_ID` int, the fixed 300-tick `getMaxAirSupply()`, the
    // `isEyeInFluid(WATER)` gate, and the client tick for the all-empty
    // wobble; the renderer applies the `isUnderWater || air < max` visibility.
    let hud_air_state = HudAirSupply {
        air: world.local_player_air_supply(),
        max_air: world.local_player_max_air_supply(),
        eye_in_water: world.local_player_eye_in_water(),
        tick_count: publication.client_tick_index,
    };
    if let Some(position) = world.local_player_pose().map(|pose| pose.position) {
        if let Some(sound) = hud_render_runtime.air_bubble_pop_sound(
            hud_air_state,
            !input.hide_gui() && game_mode_can_hurt_player,
        ) {
            mailbox.enqueue_or_report(
                crate::simulation::commands::SimulationCommand::HudAirBubblePopSound {
                    level_generation: world.client_level_generation(),
                    position,
                    volume: sound.volume,
                    pitch: sound.pitch,
                },
                "HUD air bubble sound",
            );
        }
    }
    let hud_air = Some(hud_air_state);
    // Vanilla `Gui.extractVehicleHealth` inputs (Gui.java:974-979): the local
    // player's living vehicle health + MAX_HEALTH attribute; the renderer
    // derives the heart count (`getVehicleMaxHearts`) and replaces the food
    // row while it is non-zero.
    let hud_vehicle_health = world
        .local_player_vehicle_health()
        .map(|vehicle| HudVehicleHealth {
            health: vehicle.health,
            max_health: vehicle.max_health as f32,
        });
    // Vanilla `Gui.nextContextualInfoState` selects the jumpable-vehicle bar
    // over the experience bar when the local controlled mount implements
    // `PlayerRideableJumping.canJump()`. The cooldown gate is the integer
    // `getJumpCooldown() > 0` (not partial-tick interpolated).
    let hud_jump_bar = world.local_player_rideable_jumping_vehicle_id().map(|_| {
        let cooldown = world
            .local_player_rideable_jumping_vehicle_cooldown(0.0)
            .is_some_and(|cooldown| cooldown > 0.0);
        HudJumpBar {
            progress: input.riding_jump_scale().unwrap_or(0.0),
            cooldown,
        }
    });
    // Vanilla `Gui.extractFood` starvation-shake / hunger-swap inputs
    // (Gui.java:948,958): saturation-empty gate, the Hunger potion flag, and
    // the client `tickCount` (bbb's `LightmapTickState.client_tick_count`, the
    // same per-client-tick counter the lightmap uses).
    let hud_food_effect = HudFoodEffect {
        saturation_empty: local_player
            .health
            .map(|health| health.saturation <= 0.0)
            .unwrap_or(false),
        hunger_effect: local_player_effect(world, VANILLA_MOB_EFFECT_HUNGER_ID).is_some(),
        tick_count: publication.client_tick_index,
    };
    let hud_mob_effects = hud_mob_effects_from_world(world, item_runtime);
    let hud_inventory_shows_mob_effects = world.local_inventory_is_open();
    let hud_experience_progress = local_player
        .experience
        .map(|experience| experience.progress);
    // Vanilla draws the level number only when `experienceLevel > 0`
    // (Gui.java:533); the renderer setter applies that gate.
    let hud_experience_level = local_player.experience.map(|experience| experience.level);
    // Vanilla `Gui.extractHotbarAndDecorations` substitutes `SpectatorGui`
    // for the ordinary item hotbar and samples its wall-clock fade during
    // render extraction, after input publication.
    let hud_hotbar_mode = hud_hotbar_mode_from_world(input, world, item_runtime, render_started_at);
    let hud_selected_slot = local_player.selected_hotbar_slot;
    // Vanilla `Gui.extractOverlayMessage` / `extractTitle` read the post-tick
    // countdowns with the frame partial tick; the renderer resolves fade
    // alpha per frame from these projected timers.
    let hud_action_bar_text =
        hud_action_bar_text_from_world(world, item_runtime, entity_partial_tick);
    let hud_title_text = hud_title_text_from_world(world, item_runtime, entity_partial_tick);
    let resource_generation = item_runtime.map_or(0, NativeItemRuntime::resource_generation);
    let resources_loaded = item_runtime.is_some();
    let hud_scoreboard_sidebar = hud_render_runtime.scoreboard_sidebar_update(
        HudScoreboardSidebarCacheKey {
            level_generation: world.client_level_generation(),
            display_revision: world.scoreboard_display_revision(),
            resource_generation,
            resources_loaded,
            hidden: input.hide_gui(),
        },
        || hud_scoreboard_sidebar_from_world(input, world, item_runtime),
    );
    let hud_player_list_visible = hud_player_list_is_visible(input, world);
    let hud_player_list = hud_render_runtime.player_list_update(
        HudPlayerListCacheKey {
            level_generation: world.client_level_generation(),
            display_revision: world.player_list_display_revision(),
            profile_presentation_revision: item_runtime
                .map_or(0, NativeItemRuntime::profile_presentation_revision),
            resource_generation,
            resources_loaded,
            visible: hud_player_list_visible,
            show_heads: publication.play_session_encrypted,
        },
        || {
            hud_player_list_from_world(
                input,
                world,
                item_runtime,
                publication.client_tick_index,
                publication.play_session_encrypted,
            )
        },
    );
    let hud_chat_messages = hud_render_runtime.chat_messages_update(world, item_runtime);
    let queued_chat_messages = world.client_chat().delayed_messages.len();
    let queued_chat_messages_text = (queued_chat_messages > 0).then(|| {
        localized_gui_text_with_args(
            item_runtime,
            "chat.queue",
            "%s messages waiting",
            &[&queued_chat_messages.to_string()],
        )
    });
    let hud_cursor_position = publication
        .input_runtime
        .cursor_position
        .and_then(|position| {
            (position.x.is_finite() && position.y.is_finite())
                .then(|| (position.x.floor() as i32, position.y.floor() as i32))
        });
    let hud_chat_context = HudChatContext {
        level_generation: world.client_level_generation(),
        delivery_revision: world.client_chat().delivery_revision,
        chat_screen_revision: input.chat_screen_revision(),
        resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
        resources_loaded: item_runtime.is_some(),
        gui_ticks: world.client_chat().gui_ticks,
        focused: input.chat_entry_is_active(),
        advanced_item_tooltips: input.debug_advanced_item_tooltips(),
        scroll_lines: input.chat_scroll_lines(),
        cursor_position: hud_cursor_position,
        queued_messages: queued_chat_messages,
        queue_text: queued_chat_messages_text,
        restricted_text: (input.chat_entry_is_active()
            && world.chat_presentation_config().visibility != bbb_world::ChatVisibilityState::Full)
            .then(|| {
                localized_gui_text(
                    item_runtime,
                    "chat_screen.restricted",
                    "Chat is restricted. Click this message for details.",
                )
            }),
    };
    let hud_chat_restrictions_screen =
        hud_chat_restrictions_screen(input, world, item_runtime, hud_cursor_position);
    // `LerpingBossEvent` owns a 100ms wall-clock presentation curve outside
    // canonical packet state. Anchor newly published targets at the
    // simulation change time, then sample them at this render's wall clock.
    let hud_boss_bars = hud_render_runtime.boss_bars_update(
        world,
        net_counters.connection_generation,
        now,
        render_started_at,
        stable_world_unchanged,
    );
    let item_model_keybind_context = input.item_model_keybind_context();
    let ordinary_hotbar = matches!(&hud_hotbar_mode, HudHotbarMode::Ordinary);
    let hud_hotbar_item_icons = if ordinary_hotbar {
        hotbar_item_icons_with_input_context(
            world,
            item_runtime,
            entity_partial_tick,
            input.shift_down(),
            item_model_keybind_context,
        )
    } else {
        std::array::from_fn(|_| None)
    };
    // Vanilla `Gui.extractSlot` resolves the GUI item model after applying
    // the partial-tick pop pose (Gui.java:1117-1139). Keep that pose in the
    // frame-local icon projection above; only retain the static 3D block-item
    // model while the exact world plus every model-selector input is stable.
    let hud_hotbar_block_item_models = hud_render_runtime.hotbar_block_item_models_update(
        stable_world_unchanged,
        HudHotbarBlockItemModelsCacheKey {
            ordinary_hotbar,
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            resources_loaded: item_runtime.is_some(),
            shift_down: input.shift_down(),
            keybind_context: item_model_keybind_context,
        },
        || {
            if ordinary_hotbar {
                hotbar_block_item_models(
                    world,
                    item_runtime,
                    terrain_textures,
                    input.shift_down(),
                    item_model_keybind_context,
                )
            } else {
                Vec::new()
            }
        },
    );
    let advancement_scroll_delta =
        input.advancement_scroll_delta(world.selected_advancements_tab());
    let advancement_hover_fade = input.advancement_hover_fade();
    let hud_disconnected_screen =
        hud_disconnected_screen(input, item_runtime, panorama_rotation_degrees);
    let hud_death_screen = hud_disconnected_screen
        .is_none()
        .then(|| hud_death_screen(input, world, item_runtime))
        .flatten();
    if hud_death_screen.is_some() {
        hud_render_runtime.game_event_screen_narration = None;
    }
    let hud_game_event_screen = hud_death_screen
        .is_none()
        .then(|| {
            hud_game_event_screen(
                hud_render_runtime,
                input,
                &publication.win_screen_resources,
                item_runtime,
            )
        })
        .flatten();
    let hud_player_reporting_screen = (hud_death_screen.is_none()
        && hud_game_event_screen.is_none())
    .then(|| {
        hud_player_reporting_screen(input, world, item_runtime, renderer.counters().frame_index)
    })
    .flatten();
    let hud_social_interactions_screen = (hud_death_screen.is_none()
        && hud_game_event_screen.is_none()
        && hud_player_reporting_screen.is_none())
    .then(|| {
        hud_social_interactions_screen(
            input,
            world,
            item_runtime,
            publication
                .input_runtime
                .cursor_position
                .and_then(|position| {
                    (position.x.is_finite() && position.y.is_finite())
                        .then(|| (position.x.floor() as i32, position.y.floor() as i32))
                }),
            renderer.counters().frame_index,
            Some(now),
        )
    })
    .flatten();
    let exclusive_game_screen_present = hud_disconnected_screen.is_some()
        || hud_chat_restrictions_screen.is_some()
        || hud_death_screen.is_some()
        || hud_game_event_screen.is_some()
        || hud_player_reporting_screen.is_some()
        || hud_social_interactions_screen.is_some();
    hud_render_runtime.death_screen_narration =
        death_screen_narration_frame(input, world, item_runtime);
    hud_render_runtime.disconnected_screen_narration =
        disconnected_screen_narration_frame(input, item_runtime);
    hud_render_runtime.chat_screen_narration =
        chat_screen_narration_frame(input, world, item_runtime);
    hud_render_runtime.social_interactions_narration = hud_social_interactions_screen
        .is_some()
        .then(|| social_interactions_narration_frame(input, world, item_runtime))
        .flatten();
    hud_render_runtime.player_reporting_narration = hud_player_reporting_screen
        .is_some()
        .then(|| player_reporting_narration_frame(input, world, item_runtime))
        .flatten();
    let hud_debug_options_screen = if exclusive_game_screen_present {
        None
    } else {
        hud_debug_options_screen(input, world, item_runtime, surface_size)
    };
    hud_render_runtime.debug_options_screen_narration =
        hud_debug_options_screen.as_ref().map(|screen| {
            debug_options_screen_narration_frame(input, screen, item_runtime, surface_size)
        });
    let hud_command_suggestions = if !exclusive_game_screen_present
        && hud_debug_options_screen.is_none()
        && input.command_suggestions_active()
    {
        let mut suggestion_input = input.clone();
        hud_command_suggestions(
            &mut suggestion_input,
            world,
            renderer,
            item_runtime,
            surface_size,
        )
    } else {
        None
    };
    let hud_pause_screen = if exclusive_game_screen_present || hud_debug_options_screen.is_some() {
        None
    } else {
        hud_pause_screen(input, item_runtime, surface_size)
    };
    let hud_game_rules_screen = if exclusive_game_screen_present
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
    {
        None
    } else {
        let mut game_rules_input = input.clone();
        hud_game_rules_screen(&mut game_rules_input, world, item_runtime, surface_size)
    };
    let hud_stats_screen = if exclusive_game_screen_present
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || !world.stats_screen_is_open()
    {
        None
    } else {
        let mut stats_input = input.clone();
        hud_stats_screen(
            &mut stats_input,
            world,
            item_runtime,
            terrain_textures,
            item_model_keybind_context,
            entity_partial_tick,
            surface_size,
        )
    };
    hud_render_runtime.stats_screen_narration = hud_stats_screen
        .as_ref()
        .map(|screen| stats_screen_narration_frame(input, screen, item_runtime, surface_size));
    let toasts_started_at = profiling_enabled.then(Instant::now);
    let (hud_system_toast, hud_system_toast_sound) =
        hud_system_toast(hud_render_runtime, world, item_runtime, now);
    if let (Some(audio_events), Some(sound)) =
        (audio_events.as_deref_mut(), hud_system_toast_sound.as_ref())
    {
        audio_events.play_local_sound(sound);
    }
    let advancement_toast_sounds = hud_render_runtime.client_toasts.update_with_resources(
        world,
        item_runtime,
        renderer.hud_text_measurements_ref(),
        now,
    );
    if let Some(audio_events) = audio_events.as_deref_mut() {
        for sound in &advancement_toast_sounds {
            audio_events.play_local_sound(sound);
        }
    }
    let hud_advancement_toasts = hud_render_runtime.client_toasts.project_advancements(
        world,
        item_runtime,
        terrain_textures,
        item_model_keybind_context,
        entity_partial_tick,
    );
    let hud_recipe_toasts = hud_render_runtime.client_toasts.project_recipes(
        world,
        item_runtime,
        terrain_textures,
        item_model_keybind_context,
        entity_partial_tick,
    );
    let hud_tutorial_toasts = hud_render_runtime.client_toasts.project_tutorials();
    let toasts_duration = optional_profile_elapsed(toasts_started_at);
    let hud_sign_editor_screen = if exclusive_game_screen_present
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_sign_editor_screen(input, world, surface_size)
    };
    let hud_command_block_editor_screen = if exclusive_game_screen_present
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_command_block_editor_screen(input, item_runtime, surface_size)
    };
    let hud_structure_block_editor_screen = if exclusive_game_screen_present
        || world.dialog_screen_is_open()
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_structure_block_editor_screen(input, item_runtime, surface_size)
    };
    let hud_jigsaw_block_editor_screen = if exclusive_game_screen_present
        || world.dialog_screen_is_open()
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_jigsaw_block_editor_screen(input, item_runtime, surface_size)
    };
    let hud_test_block_editor_screen = if exclusive_game_screen_present
        || world.dialog_screen_is_open()
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_test_block_editor_screen(input, item_runtime, surface_size)
    };
    let hud_test_instance_block_editor_screen = if exclusive_game_screen_present
        || world.dialog_screen_is_open()
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
    {
        None
    } else {
        hud_test_instance_block_editor_screen(input, item_runtime, surface_size)
    };
    let hud_book_screen = if exclusive_game_screen_present
        || world.dialog_screen_is_open()
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
        || hud_sign_editor_screen.is_some()
        || hud_command_block_editor_screen.is_some()
        || hud_structure_block_editor_screen.is_some()
        || hud_jigsaw_block_editor_screen.is_some()
        || hud_test_block_editor_screen.is_some()
        || hud_test_instance_block_editor_screen.is_some()
    {
        None
    } else {
        hud_book_screen(input, world, item_runtime)
    };
    let hud_inventory_screen = if exclusive_game_screen_present
        || hud_debug_options_screen.is_some()
        || hud_pause_screen.is_some()
        || hud_game_rules_screen.is_some()
        || hud_stats_screen.is_some()
        || hud_sign_editor_screen.is_some()
        || hud_command_block_editor_screen.is_some()
        || hud_structure_block_editor_screen.is_some()
        || hud_jigsaw_block_editor_screen.is_some()
        || hud_test_block_editor_screen.is_some()
        || hud_test_instance_block_editor_screen.is_some()
        || hud_book_screen.is_some()
    {
        None
    } else {
        hud_inventory_screen_with_local_state_for_surface_and_measurer(
            world,
            item_runtime,
            terrain_textures,
            input.inventory_hovered_slot(),
            InventoryHudLocalState {
                creative_inventory: input.creative_inventory_hud_state(world),
                stonecutter_recipe_scroll_row: Some(input.stonecutter_recipe_scroll_row()),
                beacon_effect_selection: Some(input.beacon_effect_selection()),
                loom_pattern_scroll_row: Some(input.loom_pattern_scroll_row()),
                loom_selected_pattern_index: input.loom_selected_pattern_index(),
                anvil_rename_text: Some(input.anvil_rename_text().to_string()),
                recipe_book_search: input.recipe_book_search_hud_state(),
                recipe_book_tabs: input.recipe_book_tab_selection_hud_state(),
                recipe_book_pages: input.recipe_book_page_hud_state(),
                recipe_book_presentation: input.recipe_book_presentation_hud_state(),
                recipe_book_overlay: input.recipe_book_overlay_hud_state(),
                advancement_scroll_delta,
                advancement_hover_fade,
                dialog_ime_preedit: input.dialog_ime_preedit_hud_state(),
                cursor_position: input.inventory_cursor_position(),
                merchant_focused_trade_row: input.merchant_focused_trade_row(),
                quick_craft_button_num: input.inventory_quick_craft_button_num(),
                quick_craft_slots: input.inventory_quick_craft_slots().to_vec(),
                shift_down: input.shift_down(),
                keybind_context: item_model_keybind_context,
                advanced_item_tooltips: input.debug_advanced_item_tooltips(),
                smithing: hud_render_runtime.smithing_screen.frame(
                    world,
                    item_runtime,
                    publication.client_tick_index,
                    entity_partial_tick,
                ),
            },
            surface_size,
            renderer,
            renderer.hud_font_book_ref(),
            Some(entity_variant_textures),
            input.enchantment_book_hud_state(entity_partial_tick),
            entity_partial_tick,
        )
    };
    hud_render_runtime.recipe_book_narration = hud_inventory_screen
        .is_some()
        .then(|| {
            if let Some(state) = input.creative_inventory_hud_state(world) {
                creative_inventory_narration_frame(world, item_runtime, &state)
            } else {
                recipe_book_narration_frame(
                    input,
                    world,
                    item_runtime,
                    surface_size,
                    entity_partial_tick,
                )
            }
        })
        .flatten();
    // Dialog, in-world game-rules, and demo popup/confirmation screens inherit
    // `Screen.extractBlurredBackground`; container/book/advancement/stats screens
    // remain vanilla in-game UI, while poem `WinScreen` owns its portal background.
    let hud_screen_blur = hud_disconnected_screen.is_some()
        || hud_chat_restrictions_screen.is_some()
        || matches!(
            hud_death_screen.as_ref(),
            Some(
                bbb_renderer::HudDeathScreen::DraftConfirmation(_)
                    | bbb_renderer::HudDeathScreen::LinkConfirmation(_)
            )
        )
        || hud_screen_blur_for_world(
            world,
            hud_inventory_screen.is_some()
                || hud_game_rules_screen.is_some()
                || hud_social_interactions_screen.is_some()
                || hud_player_reporting_screen.is_some()
                || hud_game_event_screen.is_some(),
        );
    let hud_duration = hud_started_at.elapsed();
    let item_pre_started_at = Instant::now();
    // Vanilla extracts item/entity render state after `Minecraft.tick` has advanced keybinds,
    // `gameRenderer.tick`, and `level.tickEntities`; these projections read the post-tick snapshot.
    // Dropped block-items render as 3D block-item models (replacing their billboard); the animation
    // clock is the world game time plus the partial tick.
    let item_model_age_ticks = world
        .world_time()
        .map(|time| time.game_time as f32)
        .unwrap_or(0.0)
        + entity_partial_tick;
    let shader_game_time_ticks = world
        .world_time()
        .map(|time| time.game_time as f64)
        .unwrap_or(0.0)
        + f64::from(entity_partial_tick);
    let item_registry_keys = non_entity_render_extraction.item_registry_keys(world);
    let trim_material_keys = item_registry_keys.trim_materials;
    let trim_pattern_keys = item_registry_keys.trim_patterns;
    let enchantment_keys = item_registry_keys.enchantments;
    let damage_type_keys = item_registry_keys.damage_types;
    let attribute_keys = item_registry_keys.attributes;
    let camera_entity_pose = camera_pose_from_world_at_partial_tick(world, entity_partial_tick)
        .map(|mut pose| {
            pose.eye_height = camera_eye_height_ticks.frame_eye_height(entity_partial_tick);
            pose
        });
    let camera_pose = render_camera_pose_from_world_at_partial_tick(
        world,
        entity_partial_tick,
        input.camera_type(),
        camera_eye_height_ticks,
    );
    let sign_text_outline_context = sign_text_outline_context(
        camera_entity_pose,
        input.camera_type().is_first_person(),
        world.local_player_is_scoping(),
    );
    let first_person_camera_pose = input
        .camera_type()
        .is_first_person()
        .then_some(camera_entity_pose)
        .flatten();
    hud_render_runtime.vignette.advance(
        advanced_ticks,
        world,
        render_camera_pose_from_world_at_partial_tick(
            world,
            1.0,
            input.camera_type(),
            camera_eye_height_ticks,
        )
        .map(camera_eye_position),
    );
    let hud_vignette = hud_render_runtime.vignette.extract(
        world,
        camera_pose.map(camera_eye_position),
        input.hide_gui(),
    );
    let hud_spyglass_scope = hud_render_runtime.spyglass_scope(
        hud_frame_delta.game_time,
        input.hide_gui(),
        input.camera_type().is_first_person(),
        world.local_player_is_scoping(),
    );
    let camera_hurt_effect =
        camera_hurt_effect_from_world(world, entity_partial_tick, damage_tilt_strength);
    let camera_view_bob_effect =
        camera_view_bob_effect_from_world(world, entity_partial_tick, view_bobbing);
    let camera_spinning_effect = camera_spinning_ticks.effect_for_world(world, entity_partial_tick);
    // Vanilla `GameRenderer.extract` samples the current interpolated camera
    // once per rendered frame, after the client tick. FOV, color, fog, and sky
    // must consume this same `Camera.getFluidInCamera` result.
    let camera_fluid = camera_pose.map_or(CameraFluid::None, |camera| {
        camera_fluid_in_camera(
            world,
            camera,
            [surface_size.width, surface_size.height],
            camera_fov_ticks.base_degrees(),
        )
    });
    let camera_environment_colors = camera_environment_colors(world, camera_pose, camera_fluid);
    let clear_color = clear_color_for_world_with_environment_colors_and_water_vision(
        world,
        camera_environment_colors,
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    );
    let fog_environment = if input.debug_fog_enabled() {
        fog_environment_for_world_with_environment_colors(
            world,
            camera_environment_colors,
            render_distance_chunks,
            water_vision,
            rain_fog_multiplier,
            hide_lightning_flash,
        )
    } else {
        FogEnvironment::disabled()
    };
    let sky_environment = sky_environment_for_world_with_environment_colors(
        world,
        camera_environment_colors,
        hide_lightning_flash,
    );
    let camera_fov = camera_fov_ticks.frame(camera_hurt_effect, camera_fluid, entity_partial_tick);
    let hud_locator_bar = hud_locator_bar_from_world(
        world,
        camera_pose,
        camera_fov.world_degrees,
        entity_partial_tick,
    );
    let hud_contextual_bar = hud_render_runtime.contextual.select(
        world,
        world.client_level_generation(),
        publication.client_tick_index,
        hud_locator_bar,
        hud_jump_bar,
        hud_experience_progress,
    );
    // Selection and active state are one immutable simulation publication:
    // F5 may clear/reselect independently of the canonical camera entity, while
    // a later SetCamera refresh may select again even in third person.
    let current_post_effect_id = input.camera_post_effect_id();
    let terrain_visibility_context = TerrainVisibilityContext::new(
        camera_pose,
        [surface_size.width, surface_size.height],
        camera_fov.culling_degrees,
        render_distance_chunks,
        input.debug_smart_cull_enabled(),
    );
    // Vanilla `LevelRenderer.cullTerrain` applies the asynchronously published
    // graph when it changes or camera rotation crosses a two-degree bucket.
    // The terrain owner additionally invalidates on viewport/FOV changes.
    terrain_upload.refresh_frame_visibility(terrain_visibility_context);
    let renderer_counters = renderer.counters();
    let gpu_utilization = renderer.gpu_utilization();
    let hud_audio_counters = audio_events
        .as_deref()
        .map(AudioEventSink::counters)
        .unwrap_or_else(|| audio_status.clone());
    let hud_debug_overlay = hud_debug_overlay_with_profiler_chart(
        hud_debug_overlay_at_partial_tick_with_camera_poses(
            input,
            world,
            camera_pose,
            camera_entity_pose,
            entity_partial_tick,
            item_runtime,
            terrain_textures,
            &renderer_counters,
            gpu_utilization,
            render_distance_chunks,
            surface_size,
            hud_debug_fps_sampler,
            client_framerate_limit,
            client_vsync,
            hud_debug_network_sampler,
            hud_debug_tps_sampler,
            net_counters,
            &hud_audio_counters,
            current_post_effect_id,
        ),
        input
            .debug_profiler_chart_visible()
            .then_some(hud_debug_profiler_chart)
            .flatten(),
    );
    let dropped_item_models = dropped_item_models(
        world,
        item_runtime,
        terrain_textures,
        Some(renderer),
        Some(entity_variant_textures),
        item_model_age_ticks,
        trim_material_keys.as_deref(),
        trim_pattern_keys.as_deref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let ominous_item_spawner_models = ominous_item_spawner_models(
        world,
        item_runtime,
        terrain_textures,
        entity_partial_tick,
        trim_material_keys.as_deref(),
        trim_pattern_keys.as_deref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let item_pickup_particle_states = renderer.item_pickup_particle_render_states();
    let item_pickup_particle_models = item_pickup_particle_item_models(
        world,
        &item_pickup_particle_states,
        item_runtime,
        terrain_textures,
        trim_material_keys.as_deref(),
        trim_pattern_keys.as_deref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let item_entity_billboards = item_entity_billboards_from_world(
        world,
        item_runtime,
        &dropped_item_models.handled_entity_ids,
    );
    let item_pre_duration = item_pre_started_at.elapsed();
    // Held items render as 3D models at each player's hand, on top of the dropped-item models (sharing
    // the two atlas draws).
    let mut entity_instances =
        visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(
            world,
            item_runtime,
            entity_variant_textures,
            entity_partial_tick,
            EntityRenderVisibilityContext {
                camera_pose,
                camera_detached: !input.camera_type().is_first_person(),
                surface_size: [surface_size.width, surface_size.height],
                camera_fov_degrees: camera_fov.culling_degrees,
                base_camera_fov_degrees: camera_fov_ticks.base_degrees(),
                render_distance_chunks,
                terrain_upload,
                entity_shadows,
                shadow_sky_darken: entity_shadow_sky_darken(world),
            },
            entity_render_extraction,
        );
    // Vanilla `LevelRenderer.extractVisibleEntities` submits generic name
    // displays from the same culled entity generation immediately after each
    // entity renderer extracts its model state. Camera distance/crosshair and
    // `Options.hideGui` are frame/input facts, so native resolves them here.
    let targeted_entity_id = match crosshair_target_from_camera_at_partial_tick(
        world,
        camera_entity_pose,
        entity_partial_tick,
    ) {
        Some(CrosshairTarget::Entity(hit)) => Some(hit.entity_id),
        _ => None,
    };
    let hud_crosshair_attack_indicator = hud_crosshair_attack_indicator(
        input.camera_type().is_first_person() && !world.local_player_is_spectator(),
        world.local_player_crosshair_attack_indicator_state(targeted_entity_id),
    );
    // Vanilla `Gui.extractCrosshair` hides the ordinary first-person crosshair
    // for spectators unless the current hit is an interactive MenuProvider.
    // The controlled replay has no such target, so its normal build crosshair
    // must remain hidden just like the selection outline.
    let hud_crosshair_visible =
        input.camera_type().is_first_person() && !world.local_player_is_spectator();
    let entity_name_tag_surfaces = entity_name_tag_surfaces(
        visible_entity_name_tag_render_sources(entity_render_extraction),
        &entity_instances,
        item_runtime,
        camera_pose,
        targeted_entity_id,
        world.local_player().camera.entity_id,
        !input.hide_gui(),
        renderer_counters.frame_index,
    );
    let entity_flame_surfaces =
        entity_flame_surfaces(&entity_instances, terrain_textures, camera_pose);
    let entity_shadow_surfaces = entity_shadow_surfaces(
        visible_entity_shadow_render_sources(entity_render_extraction),
        &entity_instances,
    );
    let display_entity_item_models = display_entity_item_models(
        visible_display_entity_render_sources_at_partial_tick(
            world,
            entity_partial_tick,
            entity_render_extraction,
        ),
        world,
        item_runtime,
        terrain_textures,
        camera_pose,
        renderer_counters.frame_index,
        trim_material_keys.as_deref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        attribute_keys.as_deref(),
        Some(entity_variant_textures),
    );
    let display_text_surfaces = display_entity_item_models.text_surfaces;
    let item_mid_started_at = Instant::now();
    let mut held_item_models = held_item_models_with_variant_textures(
        &entity_instances,
        world,
        item_runtime,
        terrain_textures,
        Some(entity_variant_textures),
    );
    let item_mid_duration = item_mid_started_at.elapsed();
    let block_entity_started_at = Instant::now();
    let block_destroy_stages = block_destroy_stages_from_world(world);
    let block_entity_model_start = entity_instances.len();
    let block_entity_visibility = BlockEntityVisibility::new(
        camera_pose,
        [surface_size.width, surface_size.height],
        camera_fov.culling_degrees,
    );
    let christmas_chest_textures = non_entity_render_extraction.christmas_chest_textures();
    let block_entity_revision = world.stable_block_entity_render_revision();
    let dynamic_block_entities =
        non_entity_render_extraction.dynamic_block_entity_presence(block_entity_revision, || {
            DynamicBlockEntityPresence {
                chest: !world.chest_model_source_states(0.0).is_empty(),
                bell: !world.bell_model_source_states(0.0).is_empty(),
                shulker_box: !world.shulker_box_model_source_states(0.0).is_empty(),
                decorated_pot: !world.decorated_pot_model_source_states(0.0).is_empty(),
                banner: !world.banner_model_source_states(0.0).is_empty(),
                enchanting_table: !world.enchanting_table_book_source_states().is_empty(),
                conduit: !world.conduit_model_source_states(0.0).is_empty(),
                end_portal: !world.end_portal_model_source_states(0.0).is_empty(),
                skull: !world.skull_model_source_states(0.0).is_empty(),
                spawner: !world.spawner_display_entity_source_states(0.0).is_empty()
                    || !world
                        .trial_spawner_display_entity_source_states(0.0)
                        .is_empty(),
            }
        });
    let StableBlockEntityFrame {
        sign_instances,
        bed_instances,
        lectern_instances,
        sign_text_surfaces,
    } = non_entity_render_extraction.stable_block_entity_frame(
        StableBlockEntityCacheKey {
            revision: block_entity_revision,
            text_filtering_enabled: input.client_text_filtering_enabled(),
        },
        || {
            let sign_scene =
                sign_scene_from_world(world, item_runtime, input.client_text_filtering_enabled());
            (
                sign_scene.instances,
                sign_scene.text_surfaces,
                bed_model_instances_from_world(world),
                lectern_book_model_instances_from_world(world),
            )
        },
    );
    let visible_sign_text_positions = sign_instances
        .iter()
        .filter_map(|instance| {
            block_entity_visibility.visible_block_position(instance, world, terrain_upload)
        })
        .collect();
    // Chest block-entity models join the same single entity-model submission
    // stream after held-item baking (they have no hands to bake).
    if dynamic_block_entities.chest {
        entity_instances.extend(chest_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
            christmas_chest_textures,
        ));
    }
    // Sign block-entity models ride the same stream; their face text bakes
    // into world-space glyph quads drawn with the map-label font atlas.
    entity_instances.extend_from_slice(sign_instances);
    // Bed and bell block-entity models ride the same stream: beds are static
    // per-half meshes, bells carry the shake ticks for the renderer-side
    // `BellModel.setupAnim` swing.
    entity_instances.extend_from_slice(bed_instances);
    if dynamic_block_entities.bell {
        entity_instances.extend(bell_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
        ));
    }
    // Shulker box and decorated pot block-entity models ride the same stream:
    // boxes carry the lerped lid progress, pots their sherd patterns and the
    // wobble style/progress.
    if dynamic_block_entities.shulker_box {
        entity_instances.extend(shulker_box_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
        ));
    }
    if dynamic_block_entities.decorated_pot {
        entity_instances.extend(decorated_pot_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
        ));
    }
    // Banners carry their base color, tinted pattern stack, and flag swing
    // phase.
    if dynamic_block_entities.banner {
        entity_instances.extend(banner_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
            Some(entity_variant_textures),
        ));
    }
    // The enchanting-table hovering book (animated, nearest-player-facing) and
    // the lectern's static open book share the same `BookModel` in the stream.
    if dynamic_block_entities.enchanting_table {
        entity_instances.extend(
            enchanting_table_book_model_instances_from_world_at_partial_tick(
                world,
                entity_partial_tick,
            ),
        );
    }
    entity_instances.extend_from_slice(lectern_instances);
    if dynamic_block_entities.conduit {
        entity_instances.extend(conduit_model_instances_from_world_at_partial_tick(
            world,
            camera_pose,
            entity_partial_tick,
        ));
    }
    if dynamic_block_entities.end_portal {
        entity_instances.extend(end_portal_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
        ));
    }
    if dynamic_block_entities.skull {
        entity_instances.extend(skull_model_instances_from_world_at_partial_tick(
            world,
            entity_partial_tick,
            item_runtime,
        ));
    }
    if dynamic_block_entities.spawner {
        for submission in spawner_display_entity_submissions_from_world_at_partial_tick(
            world,
            item_runtime,
            Some(entity_variant_textures),
            entity_partial_tick,
        ) {
            if !block_entity_visibility.is_visible(&submission.instance, world, terrain_upload) {
                continue;
            }
            if let Some(entity_world) = submission.entity_world.as_ref() {
                held_item_models.append(held_item_models_with_variant_textures(
                    std::slice::from_ref(&submission.instance),
                    entity_world,
                    item_runtime,
                    terrain_textures,
                    Some(entity_variant_textures),
                ));
            }
            entity_instances.push(submission.instance);
        }
    }
    retain_visible_block_entity_instances(
        &mut entity_instances,
        block_entity_model_start,
        block_entity_visibility,
        world,
        terrain_upload,
    );
    apply_block_entity_crumbling(
        &mut entity_instances[block_entity_model_start..],
        &block_destroy_stages,
        terrain_textures,
    );
    let block_entity_duration = block_entity_started_at.elapsed();
    let item_tail_started_at = Instant::now();
    let map_texture_cache = non_entity_render_extraction.begin_map_texture_frame();
    let hud_map_preview_models = hud_map_preview_models(
        world,
        item_runtime,
        hud_inventory_screen.as_ref(),
        surface_size.width,
        surface_size.height,
        map_texture_cache,
    );
    let first_person_item_models = first_person_item_models_with_map_cache(
        world,
        &publication.first_person_hand_presentation,
        item_runtime,
        terrain_textures,
        Some(entity_variant_textures),
        map_texture_cache,
        first_person_camera_pose,
        entity_partial_tick,
    );
    // Vanilla `GameRenderer.renderItemInHand` samples the local player's visible arm state,
    // held stacks, frame light, and partial-tick attack swing during the first-person hand pass,
    // after the client tick has advanced input/use-item/entity animation state.
    let local_player_model_instance = world.local_player_id().and_then(|id| {
        entity_instances
            .iter()
            .find(|instance| instance.entity_id == id)
    });
    let first_person_player_arms = first_person_player_arms(
        world,
        &publication.first_person_hand_presentation,
        item_runtime,
        local_player_model_instance,
        first_person_camera_pose,
        entity_partial_tick,
    );
    // Item frames render their wooden border + framed item into the same two atlas draws.
    let item_frame_models = item_frame_models_with_map_cache(
        world,
        item_runtime,
        terrain_textures,
        map_texture_cache,
        trim_material_keys.as_deref(),
        trim_pattern_keys.as_deref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        attribute_keys.as_deref(),
        Some(entity_variant_textures),
    );
    let entity_block_models = entity_block_models(
        &entity_instances,
        world,
        item_runtime,
        terrain_textures,
        entity_partial_tick,
    );
    let smithing_preview_item_models = smithing_entity_preview_item_models(
        world,
        item_runtime,
        terrain_textures,
        Some(entity_variant_textures),
        hud_inventory_screen.as_ref(),
        enchantment_keys.as_deref(),
        damage_type_keys.as_deref(),
        trim_material_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let mut item_model_residency = dropped_item_models.residency;
    item_model_residency.append(display_entity_item_models.residency);
    item_model_residency.append(ominous_item_spawner_models.residency);
    item_model_residency.append(item_pickup_particle_models.residency);
    item_model_residency.append(held_item_models.residency);
    item_model_residency.append(first_person_item_models.residency);
    item_model_residency.append(item_frame_models.residency);
    item_model_residency.append(entity_block_models);
    item_model_residency.append(smithing_preview_item_models);
    let mut block_item_meshes = dropped_item_models.block_meshes;
    block_item_meshes.extend(ominous_item_spawner_models.block_meshes);
    block_item_meshes.extend(held_item_models.block_meshes);
    block_item_meshes.extend(item_frame_models.block_meshes);
    let block_item_z_offset_forward_meshes = item_frame_models.block_z_offset_forward_meshes;
    let mut block_item_translucent_meshes = dropped_item_models.block_translucent_meshes;
    block_item_translucent_meshes.extend(ominous_item_spawner_models.block_translucent_meshes);
    block_item_translucent_meshes.extend(held_item_models.block_translucent_meshes);
    block_item_translucent_meshes.extend(item_frame_models.block_translucent_meshes);
    let mut item_model_glint_meshes = dropped_item_models.block_glint_meshes;
    item_model_glint_meshes.extend(ominous_item_spawner_models.block_glint_meshes);
    item_model_glint_meshes.extend(held_item_models.block_glint_meshes);
    item_model_glint_meshes.extend(item_frame_models.block_glint_meshes);
    let mut item_model_glint_translucent_meshes =
        dropped_item_models.block_glint_translucent_meshes;
    item_model_glint_translucent_meshes
        .extend(ominous_item_spawner_models.block_glint_translucent_meshes);
    item_model_glint_translucent_meshes.extend(held_item_models.block_glint_translucent_meshes);
    item_model_glint_translucent_meshes.extend(item_frame_models.block_glint_translucent_meshes);
    let mut flat_item_meshes = dropped_item_models.flat_meshes;
    flat_item_meshes.extend(ominous_item_spawner_models.flat_meshes);
    flat_item_meshes.extend(held_item_models.flat_meshes);
    flat_item_meshes.extend(item_frame_models.flat_meshes);
    let mut flat_item_translucent_meshes = dropped_item_models.flat_translucent_meshes;
    flat_item_translucent_meshes.extend(ominous_item_spawner_models.flat_translucent_meshes);
    flat_item_translucent_meshes.extend(held_item_models.flat_translucent_meshes);
    flat_item_translucent_meshes.extend(item_frame_models.flat_translucent_meshes);
    item_model_glint_meshes.extend(dropped_item_models.flat_glint_meshes);
    item_model_glint_meshes.extend(ominous_item_spawner_models.flat_glint_meshes);
    item_model_glint_meshes.extend(held_item_models.flat_glint_meshes);
    item_model_glint_meshes.extend(item_frame_models.flat_glint_meshes);
    item_model_glint_translucent_meshes.extend(dropped_item_models.flat_glint_translucent_meshes);
    item_model_glint_translucent_meshes
        .extend(ominous_item_spawner_models.flat_glint_translucent_meshes);
    item_model_glint_translucent_meshes.extend(held_item_models.flat_glint_translucent_meshes);
    item_model_glint_translucent_meshes.extend(item_frame_models.flat_glint_translucent_meshes);
    let item_frame_map_textures = item_frame_models.map_textures;
    let item_frame_map_surfaces = item_frame_models.map_surfaces;
    let item_frame_map_decoration_textures = item_frame_models.map_decoration_textures;
    let item_frame_map_decoration_surfaces = item_frame_models.map_decoration_surfaces;
    let item_frame_map_text_surfaces = item_frame_models.map_text_surfaces;
    let item_duration = item_pre_duration + item_mid_duration + item_tail_started_at.elapsed();
    let world_debug_started_at = Instant::now();
    // Vanilla `LevelRenderer.renderLevel` samples `level.getGameTime()`, camera position, and the
    // frame partial tick for the cloud pass after the client tick has advanced the level clock.
    let cloud_frame = cloud_frame_for_world(world, camera_pose, entity_partial_tick);
    // Vanilla `LevelRenderer.extractLevel` calls `WeatherEffectRenderer.extractRenderState` with
    // level ticks, `deltaPartialTick`, and camera position during render extraction.
    let weather_render_state =
        weather_render_state_for_world(world, terrain_textures, camera_pose, entity_partial_tick);
    // Vanilla `LevelRenderer` extraction runs `worldBorderRenderer.extract` in
    // the "border" profiler section right after the weather/sky extraction
    // (LevelRenderer.java:573-585), after `ClientLevel.tick` has advanced the
    // border lerp above; the forcefield UV scroll samples `Util.getMillis()`
    // (WorldBorderRenderer.java:134).
    let world_border_render_state = world_border_render_state_for_world(
        world,
        camera_pose,
        render_distance_chunks,
        entity_partial_tick,
        wall_clock_millis(),
    );
    // Vanilla `Minecraft.renderFrame` calls `pick(partialTicks)` before
    // `GameRenderer.extract`; block/entity outline extraction reads that post-input camera state.
    let selection_outline = (!input.hide_gui())
        .then(|| selection_outline_from_camera(world, camera_entity_pose))
        .flatten();
    let chunk_border_outline = debug_chunk_border_outline(input, world, camera_pose);
    let entity_scene_outline = debug_entity_scene_outline(input, world, entity_partial_tick);
    let remote_debug_selection = input.remote_debug_renderers();
    let selected_remote_debug_entity =
        remote_debug_selection_cache.selected_for_generation(world.client_level_generation());
    let targeted_remote_debug_entity = (remote_debug_selection
        .contains(crate::remote_debug::RemoteDebugRenderer::Brain)
        && !world.local_player_is_spectator())
    .then(|| {
        debug_targeted_entity_from_camera_at_partial_tick(
            world,
            camera_entity_pose,
            entity_partial_tick,
        )
        .and_then(|id| world.probe_entity_transform(id))
        .map(|entity| entity.uuid)
    })
    .flatten();
    let remote_debug_gizmos = remote_debug_gizmos(
        world,
        remote_debug_selection,
        camera_pose,
        selected_remote_debug_entity,
    );
    remote_debug_selection_cache.finish_frame(
        remote_debug_selection,
        world.local_player_is_spectator(),
        targeted_remote_debug_entity,
    );
    let (game_test_highlight_boxes, game_test_highlight_text_billboards) =
        non_entity_render_extraction.game_test_highlight_update(
            world.client_level_generation(),
            world.game_test_highlight_render_revision(),
            || game_test_highlight_gizmos(world),
        );
    let reduced_debug_info = world.local_player_has_reduced_debug_info();
    let chunk_section_debug_gizmos = terrain_upload.chunk_section_debug_gizmos(
        terrain_visibility_context,
        input.debug_screen_entry_enabled(DebugScreenEntryId::ChunkSectionPaths, reduced_debug_info),
        input
            .debug_screen_entry_enabled(DebugScreenEntryId::ChunkSectionOctree, reduced_debug_info),
        input.debug_screen_entry_enabled(
            DebugScreenEntryId::ChunkSectionVisibility,
            reduced_debug_info,
        ),
    );
    let mut debug_buffers = non_entity_render_extraction.begin_debug_frame();
    // `DebugRenderer.refreshRendererList` registers `OctreeDebugRenderer`
    // before water/heightmap/collision renderers.
    debug_buffers
        .lines
        .extend(chunk_section_debug_gizmos.octree_lines);
    debug_buffers
        .text_billboards
        .extend(chunk_section_debug_gizmos.octree_text_billboards);
    // Vanilla `DebugRenderer.refreshRendererList` registers WaterDebugRenderer
    // before HeightMapRenderer, so their shared debugFilledBox submissions keep
    // that renderer order when both custom entries are enabled.
    let debug_water_samples = debug_water_samples(input, world);
    debug_buffers
        .filled_boxes
        .extend(debug_water_boxes(&debug_water_samples));
    debug_buffers
        .filled_boxes
        .extend(debug_heightmap_boxes(input, world, camera_pose));
    debug_buffers
        .filled_boxes
        .extend(remote_debug_gizmos.filled_boxes);
    debug_buffers
        .filled_rects
        .extend(debug_solid_face_rects(input, world, camera_pose));
    debug_buffers
        .text_billboards
        .extend(debug_water_amount_billboards(&debug_water_samples));
    debug_buffers
        .text_billboards
        .extend(debug_light_level_billboards(input, world, camera_pose));
    debug_buffers
        .text_billboards
        .extend(debug_selection_text_billboards(
            entity_scene_outline.as_ref(),
        ));
    debug_buffers
        .text_billboards
        .extend(remote_debug_gizmos.text_billboards);
    debug_buffers
        .lines
        .extend(debug_collision_cache.lines(input, world, now));
    debug_buffers
        .lines
        .extend(debug_supporting_block_cache.lines(input, world, now));
    let (debug_sky_light_section_rects, debug_sky_light_section_lines) =
        debug_sky_light_section_cache.gizmos(input, world, now);
    debug_buffers
        .filled_rects
        .extend(debug_sky_light_section_rects);
    debug_buffers.lines.extend(debug_sky_light_section_lines);
    debug_buffers.lines.extend(remote_debug_gizmos.lines);
    // `ChunkCullingDebugRenderer` is always the final Vanilla debug renderer.
    debug_buffers
        .filled_boxes
        .extend(chunk_section_debug_gizmos.culling_filled_boxes);
    debug_buffers
        .lines
        .extend(chunk_section_debug_gizmos.culling_lines);
    // Vanilla `LevelRenderer.extractBlockDestroyAnimation` reads block-breaking state during
    // render extract, after the client tick; local destroy overlay ticks are advanced above.
    let block_destroy_overlays = if stable_world_unchanged {
        RendererFrameUpdate::Unchanged
    } else {
        RendererFrameUpdate::Replace(block_destroy_overlays_from_stages(
            world,
            terrain_textures,
            &block_destroy_stages,
        ))
    };
    let world_debug_duration = world_debug_started_at.elapsed();
    // Vanilla `ParticleEngine.extract` calls `SingleQuadParticle.getLightCoords(partialTicks)`
    // during level render extraction; sample world light for the current particle positions before
    // the renderer later collects particle vertices.
    renderer.refresh_particle_lights(|position| particle_light_for_world(world, position));
    non_entity_render_extraction.record_debug_capacity_growths(&debug_buffers);
    let DebugRenderExtractionBuffers {
        filled_boxes: debug_filled_boxes,
        filled_rects: debug_filled_rects,
        text_billboards: debug_text_billboards,
        lines: debug_lines,
        ..
    } = debug_buffers;
    let apply_started_at = Instant::now();
    let recycled_debug_buffers = apply_renderer_frame(
        renderer,
        RendererFrame {
            lightmap_environment,
            clear_color,
            fog_environment,
            sky_environment,
            cloud_environment,
            hide_gui: input.hide_gui(),
            hud_crosshair_visible,
            hud_crosshair_attack_indicator,
            hud_player_health,
            hud_sleep_overlay,
            hud_food,
            hud_food_effect,
            hud_mob_effects,
            hud_inventory_shows_mob_effects,
            hud_armor,
            hud_air,
            hud_vehicle_health,
            hud_vignette,
            hud_spyglass_scope,
            hud_contextual_bar,
            hud_experience_level,
            hud_hotbar_mode,
            hud_selected_slot,
            hud_hotbar_item_icons,
            hud_hotbar_block_item_models,
            hud_inventory_screen,
            hud_book_screen,
            hud_screen_blur,
            hud_sign_editor_screen,
            hud_command_block_editor_screen,
            hud_structure_block_editor_screen,
            hud_jigsaw_block_editor_screen,
            hud_test_block_editor_screen,
            hud_test_instance_block_editor_screen,
            hud_pause_screen,
            hud_game_rules_screen,
            hud_disconnected_screen,
            hud_death_screen,
            hud_game_event_screen,
            hud_social_interactions_screen,
            hud_player_reporting_screen,
            hud_stats_screen,
            hud_system_toast,
            hud_advancement_toasts,
            hud_recipe_toasts,
            hud_tutorial_toasts,
            hud_debug_options_screen,
            hud_action_bar_text,
            hud_title_text,
            hud_scoreboard_sidebar,
            hud_player_list,
            hud_player_list_tick_count: publication.client_tick_index,
            hud_chat_messages,
            hud_chat_context,
            hud_chat_restrictions_screen,
            hud_command_suggestions,
            hud_debug_overlay,
            hud_boss_bars,
            item_entity_billboards,
            item_model_residency,
            block_item_model_meshes: block_item_meshes,
            block_item_model_z_offset_forward_meshes: block_item_z_offset_forward_meshes,
            block_item_model_translucent_meshes: block_item_translucent_meshes,
            flat_item_model_meshes: flat_item_meshes,
            flat_item_model_translucent_meshes: flat_item_translucent_meshes,
            item_model_glint_meshes,
            item_model_glint_translucent_meshes,
            item_pickup_block_item_model_meshes: item_pickup_particle_models.block_meshes,
            item_pickup_block_item_model_translucent_meshes: item_pickup_particle_models
                .block_translucent_meshes,
            item_pickup_flat_item_model_meshes: item_pickup_particle_models.flat_meshes,
            item_pickup_flat_item_model_translucent_meshes: item_pickup_particle_models
                .flat_translucent_meshes,
            item_pickup_item_model_glint_meshes: item_pickup_particle_models
                .block_glint_meshes
                .into_iter()
                .chain(item_pickup_particle_models.flat_glint_meshes)
                .collect(),
            item_pickup_item_model_glint_translucent_meshes: item_pickup_particle_models
                .block_glint_translucent_meshes
                .into_iter()
                .chain(item_pickup_particle_models.flat_glint_translucent_meshes)
                .collect(),
            first_person_block_item_model_meshes: first_person_item_models.block_meshes,
            first_person_block_item_model_translucent_meshes: first_person_item_models
                .block_translucent_meshes,
            first_person_flat_item_model_meshes: first_person_item_models.flat_meshes,
            first_person_flat_item_model_translucent_meshes: first_person_item_models
                .flat_translucent_meshes,
            first_person_item_model_glint_meshes: first_person_item_models
                .block_glint_meshes
                .into_iter()
                .chain(first_person_item_models.flat_glint_meshes)
                .collect(),
            first_person_item_model_glint_translucent_meshes: first_person_item_models
                .block_glint_translucent_meshes
                .into_iter()
                .chain(first_person_item_models.flat_glint_translucent_meshes)
                .collect(),
            first_person_player_arms,
            first_person_map_background_textures: first_person_item_models.map_background_textures,
            first_person_map_background_surfaces: first_person_item_models.map_background_surfaces,
            first_person_map_textures: first_person_item_models.map_textures,
            first_person_map_surfaces: first_person_item_models.map_surfaces,
            first_person_map_decoration_textures: first_person_item_models.map_decoration_textures,
            first_person_map_decoration_surfaces: first_person_item_models.map_decoration_surfaces,
            first_person_map_text_surfaces: first_person_item_models.map_text_surfaces,
            hud_map_preview_textures: hud_map_preview_models.textures,
            hud_map_preview_surfaces: hud_map_preview_models.surfaces,
            hud_map_preview_decoration_textures: hud_map_preview_models.decoration_textures,
            hud_map_preview_decoration_surfaces: hud_map_preview_models.decoration_surfaces,
            hud_map_preview_text_surfaces: hud_map_preview_models.text_surfaces,
            item_frame_map_textures,
            item_frame_map_surfaces,
            item_frame_map_decoration_textures,
            item_frame_map_decoration_surfaces,
            item_frame_map_text_surfaces,
            sign_text_surfaces,
            visible_sign_text_positions,
            sign_text_outline_context,
            display_text_surfaces,
            entity_name_tag_surfaces,
            entity_flame_surfaces,
            entity_shadow_surfaces,
            entity_model_instances: entity_instances,
            // Vanilla consumes the asynchronously published
            // `SectionOcclusionGraph.currentGraph` during frame visibility
            // extraction (`SectionOcclusionGraph.addSectionsInFrustum`).
            terrain_visible_sections: terrain_upload.renderer_visible_sections(),
            camera_pose,
            camera_hurt_effect,
            camera_view_bob_effect,
            camera_spinning_effect,
            camera_fov_degrees: camera_fov.world_degrees,
            camera_hud_fov_degrees: camera_fov.hud_degrees,
            current_post_effect_id,
            camera_post_effect_active: input.camera_post_effect_active(),
            shader_game_time_ticks,
            cloud_frame,
            weather_render_state,
            world_border_render_state,
            selection_outline,
            chunk_border_outline,
            entity_scene_outline,
            debug_filled_boxes,
            debug_filled_rects,
            debug_text_billboards,
            debug_lines,
            game_test_highlight_boxes,
            game_test_highlight_text_billboards,
            block_destroy_overlays,
        },
        entity_render_extraction,
    );
    non_entity_render_extraction.recycle_debug_buffers(recycled_debug_buffers);
    non_entity_render_extraction.record_frame(
        hud_duration,
        item_duration,
        block_entity_duration,
        world_debug_duration,
        apply_started_at.elapsed(),
    );
    let pump_scene_extracted_at = Instant::now();
    maybe_upload_terrain_texture_animation(renderer, terrain_upload, terrain_textures);
    maybe_upload_decoded_terrain(
        world,
        renderer,
        terrain_upload,
        terrain_textures,
        terrain_visibility_context,
    );
    let player_section_compiled_and_visible =
        terrain_upload.is_player_section_compiled_and_visible(world);
    if let Some((level_generation, player_section_compiled_and_visible)) = terrain_upload
        .take_readiness_edge(
            publication.client_level_generation,
            player_section_compiled_and_visible,
        )
    {
        mailbox.enqueue_or_report(
            crate::simulation::commands::SimulationCommand::TerrainReadiness {
                level_generation,
                player_section_compiled_and_visible,
            },
            "terrain readiness",
        );
    }
    let pump_elapsed = pump_started_at.elapsed();
    // Section attribution for frames slow enough to starve the per-frame
    // synthetic-input and net-command consumers.
    if pump_elapsed > std::time::Duration::from_secs(1) {
        tracing::warn!(
            pump_millis = pump_elapsed.as_millis() as u64,
            net_drain_millis = (pump_net_drained_at - pump_started_at).as_millis() as u64,
            control_sync_millis = (pump_control_synced_at - pump_net_drained_at).as_millis() as u64,
            client_tick_millis =
                (pump_client_ticked_at - pump_control_synced_at).as_millis() as u64,
            extract_millis = (pump_scene_extracted_at - pump_client_ticked_at).as_millis() as u64,
            terrain_upload_millis =
                (pump_elapsed - (pump_scene_extracted_at - pump_started_at)).as_millis() as u64,
            "slow pump sections"
        );
    }
    NativeRenderProfile {
        extract: profile_started_at.elapsed().saturating_sub(toasts_duration),
        toasts: toasts_duration,
    }
}

fn renderer_particle_block_fluid_surface_sample(
    world: &WorldStore,
    position: [f64; 3],
) -> ParticleBlockFluidSurfaceSample {
    let sample = world.particle_block_fluid_surface_sample(position);
    ParticleBlockFluidSurfaceSample {
        block_collision_height: sample.block_collision_height,
        fluid_height: sample.fluid_height,
        fluid_kind: sample.fluid_kind.map(|kind| match kind {
            TerrainFluidKind::Water => ParticleFluidKind::Water,
            TerrainFluidKind::Lava => ParticleFluidKind::Lava,
        }),
        block_is_air: sample.block_is_air,
    }
}

fn emit_particle_sound_events(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    sound_events: Vec<ParticleSoundEvent>,
) {
    let Some(audio_events) = audio_events.as_deref_mut() else {
        return;
    };
    for event in sound_events {
        let state = particle_sound_event_state(event);
        audio_events.play_positioned_sound(&state);
    }
}

fn submit_local_player_action_particles(
    batches: &mut dyn ParticleBatchSink,
    world: &WorldStore,
    input: &mut ClientInputState,
    particle_runtime: Option<&mut NativeParticleRuntime>,
) {
    let particles = input.take_pending_local_player_action_particles();
    let Some(particle_runtime) = particle_runtime else {
        return;
    };
    for state in particles {
        let batch = match state {
            LocalPlayerActionParticleState::BlockAttack(effect) => {
                particle_runtime.local_block_attack_particle_batch(world, effect)
            }
            LocalPlayerActionParticleState::DestroyHit(state) => {
                particle_runtime.local_destroy_hit_particle_batch(state)
            }
            LocalPlayerActionParticleState::Interaction(action) => {
                particle_runtime.local_interaction_particle_batch(world, action)
            }
        };
        batches.submit_particle_batch(batch);
    }
}

fn submit_primed_tnt_smoke_particles(
    batches: &mut dyn ParticleBatchSink,
    world: &WorldStore,
    ticks: u32,
) {
    if ticks == 0 {
        return;
    }
    let batch = primed_tnt_smoke_particle_batch(world.primed_tnt_smoke_particle_states(), ticks);
    batches.submit_particle_batch(batch);
}

fn primed_tnt_smoke_particle_batch(
    states: Vec<PrimedTntSmokeParticleState>,
    ticks: u32,
) -> ParticleSpawnBatch {
    if ticks == 0 || states.is_empty() {
        return ParticleSpawnBatch::default();
    }

    let mut commands = Vec::with_capacity(states.len().saturating_mul(ticks as usize));
    for _ in 0..ticks {
        commands.extend(
            states
                .iter()
                .map(|state| primed_tnt_smoke_particle_command(*state)),
        );
    }
    ParticleSpawnBatch {
        commands,
        ..ParticleSpawnBatch::default()
    }
}

fn submit_entity_client_tick_effects(
    batches: &mut dyn ParticleBatchSink,
    world: &mut WorldStore,
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    let batch = entity_client_tick_particle_batch(
        world.take_ravager_stun_particle_states(),
        world.take_evoker_fangs_crit_particle_states(),
    );
    batches.submit_particle_batch(batch);

    let mut effects = world.take_entity_client_movement_effect_states();
    for effect in effects.drain(..) {
        match effect {
            EntityClientMovementEffectState::Particle(state) => {
                let Some(particle_events) = particle_events.as_deref_mut() else {
                    continue;
                };
                batches.submit_particle_batch(
                    particle_events.spawn_entity_client_movement_particles(state),
                );
            }
            EntityClientMovementEffectState::Sound(state) => {
                if let Some(sound) = world.record_entity_client_movement_sound_if_audible(state) {
                    if let Some(audio_events) = audio_events.as_deref_mut() {
                        audio_events.play_positioned_sound(&sound);
                    }
                }
            }
        }
    }
    world.recycle_entity_client_movement_effect_states(effects);
}

fn submit_ominous_item_spawner_particles(
    batches: &mut dyn ParticleBatchSink,
    world: &mut WorldStore,
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
) {
    let ominous_item_spawner_particles: Vec<OminousItemSpawnerParticleState> =
        world.take_ominous_item_spawner_particle_states();
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    for state in ominous_item_spawner_particles {
        batches.submit_particle_batch(particle_events.spawn_ominous_item_spawner_particles(state));
    }
}

fn entity_client_tick_particle_batch(
    ravager_stun_particles: Vec<RavagerStunParticleState>,
    evoker_fangs_crit_particles: Vec<EvokerFangsCritParticleState>,
) -> ParticleSpawnBatch {
    if ravager_stun_particles.is_empty() && evoker_fangs_crit_particles.is_empty() {
        return ParticleSpawnBatch::default();
    }

    let mut commands =
        Vec::with_capacity(ravager_stun_particles.len() + evoker_fangs_crit_particles.len());
    commands.extend(ravager_stun_particles.into_iter().map(|state| {
        let mut command = direct_particle_spawn_command(
            VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
            "minecraft:entity_effect",
            [state.position.x, state.position.y, state.position.z],
            [0.0, 0.0, 0.0],
        );
        command.option_color = Some([0.49803922, 0.5137255, 0.57254905, 1.0]);
        command
    }));
    commands.extend(evoker_fangs_crit_particles.into_iter().map(|state| {
        direct_particle_spawn_command(
            VANILLA_PARTICLE_TYPE_CRIT_ID,
            "minecraft:crit",
            [state.position.x, state.position.y, state.position.z],
            [state.velocity.x, state.velocity.y, state.velocity.z],
        )
    }));

    ParticleSpawnBatch {
        commands,
        ..ParticleSpawnBatch::default()
    }
}

fn primed_tnt_smoke_particle_command(state: PrimedTntSmokeParticleState) -> ParticleSpawnCommand {
    direct_particle_spawn_command(
        VANILLA_PARTICLE_TYPE_SMOKE_ID,
        "minecraft:smoke",
        [state.position.x, state.position.y + 0.5, state.position.z],
        [0.0, 0.0, 0.0],
    )
}

fn direct_particle_spawn_command(
    particle_type_id: i32,
    particle_id: &str,
    position: [f64; 3],
    velocity: [f64; 3],
) -> ParticleSpawnCommand {
    ParticleSpawnCommand {
        particle_type_id,
        particle_id: particle_id.to_string(),
        sprite_ids: Vec::new(),
        position,
        velocity,
        override_limiter: false,
        always_show: false,
        raw_options_len: 0,
        initial_delay_ticks: 0,
        child_spawn_templates: Vec::new(),
        option_color: None,
        option_color_to: None,
        option_scale: None,
        option_power: None,
        option_target: None,
        option_entity_target_source: None,
        option_duration_ticks: None,
        option_roll: None,
        option_block: None,
        option_item: None,
        option_item_pickup_source_entity_id: None,
        option_item_pickup_age_ticks: None,
        option_item_pickup_bob_offset: None,
        option_item_pickup_light: None,
        option_item_pickup_experience_orb_icon: None,
        option_item_pickup_component_patch: None,
        option_item_pickup_projectile_model: None,
        option_firework_trail: false,
        option_firework_twinkle: false,
        option_firework_half_lifetime_age: false,
    }
}

fn particle_sound_event_state(event: ParticleSoundEvent) -> SoundEventState {
    SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(event.sound_event_id),
            fixed_range: None,
        },
        source: event.source,
        position: Vec3d {
            x: event.position[0],
            y: event.position[1],
            z: event.position[2],
        },
        volume: event.volume,
        pitch: event.pitch,
        seed: event.seed,
        distance_delay: event.distance_delay,
    }
}

fn particle_light_block_pos(position: [f64; 3]) -> BlockPos {
    BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    }
}

fn advance_block_destruction_render_ticks(world: &mut WorldStore, running_ticks: u32) -> usize {
    world.advance_block_destruction_render_ticks(running_ticks)
}

fn block_destroy_stages_from_world(world: &WorldStore) -> Vec<(BlockPos, u8)> {
    let mut stages = Vec::new();
    for progress in world.block_destructions() {
        if progress.progress < 10 {
            merge_block_destroy_stage(&mut stages, progress.pos, progress.progress);
        }
    }

    let interaction = &world.local_player().interaction;
    if let (Some(pos), Some(stage)) = (
        interaction.destroying_block,
        interaction.destroying_block_stage,
    ) {
        merge_block_destroy_stage(&mut stages, pos, stage);
    }

    stages.sort_by_key(|(pos, _stage)| (pos.x, pos.y, pos.z));
    stages
}

fn block_destroy_overlays_from_stages(
    world: &WorldStore,
    textures: &TerrainTextureState,
    stages: &[(BlockPos, u8)],
) -> Vec<BlockDestroyOverlay> {
    stages
        .iter()
        .filter_map(|(pos, stage)| {
            let uv = textures.destroy_stage_uv_rect(*stage)?;
            Some(BlockDestroyOverlay {
                pos: [pos.x, pos.y, pos.z],
                uv,
                shape: block_destroy_render_shape(world, textures, *pos),
            })
        })
        .collect()
}

/// Projects Vanilla `BlockEntityRenderState.breakProgress` onto the block-entity model instances
/// that actually receive a crumbling overlay from their renderer. The `stages` slice is the same
/// sorted, highest-stage-per-position projection used by the terrain block overlay path above.
fn apply_block_entity_crumbling(
    instances: &mut [EntityModelInstance],
    stages: &[(BlockPos, u8)],
    textures: &TerrainTextureState,
) {
    for instance in instances {
        instance.render_state.block_entity_crumbling = None;
        if !block_entity_model_accepts_crumbling(instance.kind) {
            continue;
        }
        let pos = BlockPos {
            x: instance.position[0].floor() as i32,
            y: instance.position[1].floor() as i32,
            z: instance.position[2].floor() as i32,
        };
        let Ok(index) = stages.binary_search_by_key(&(pos.x, pos.y, pos.z), |(pos, _stage)| {
            (pos.x, pos.y, pos.z)
        }) else {
            continue;
        };
        let Some(uv) = textures.destroy_stage_uv_rect(stages[index].1) else {
            continue;
        };
        instance.render_state.block_entity_crumbling = Some(BlockEntityCrumbling {
            origin: [pos.x as f32, pos.y as f32, pos.z as f32],
            uv,
        });
    }
}

fn block_entity_model_accepts_crumbling(kind: EntityModelKind) -> bool {
    matches!(
        kind,
        EntityModelKind::Chest { .. }
            | EntityModelKind::Sign { .. }
            | EntityModelKind::Bed { .. }
            | EntityModelKind::Bell
            | EntityModelKind::ShulkerBox { .. }
            | EntityModelKind::Banner { .. }
            | EntityModelKind::EnchantingBook
            | EntityModelKind::LecternBook
            | EntityModelKind::SkullBlock { .. }
            | EntityModelKind::Conduit {
                part: ConduitModelPart::Shell | ConduitModelPart::Cage
            }
    )
}

/// The render shape of the block being broken at `pos`, reused from the chunk mesher's
/// `TerrainTextureState::block_render_shape` so the crumbling decal follows the block's real
/// geometry (stairs/slab/fence/cross). Falls back to a full cube when the chunk is not loaded.
fn block_destroy_render_shape(
    world: &WorldStore,
    textures: &TerrainTextureState,
    pos: bbb_world::BlockPos,
) -> bbb_renderer::terrain::TerrainRenderShape {
    let Some(probe) = world.probe_block(pos) else {
        return bbb_renderer::terrain::TerrainRenderShape::Cube;
    };
    textures.block_render_shape(
        probe.block_name.as_deref(),
        &probe.block_properties,
        probe.material,
        BlockRenderPosition {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
    )
}

fn merge_block_destroy_stage(
    stages: &mut Vec<(bbb_world::BlockPos, u8)>,
    pos: bbb_world::BlockPos,
    stage: u8,
) {
    if let Some((_pos, existing)) = stages
        .iter_mut()
        .find(|(existing_pos, _)| *existing_pos == pos)
    {
        *existing = (*existing).max(stage);
    } else {
        stages.push((pos, stage));
    }
}

#[cfg(test)]
fn hotbar_item_icons(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    hotbar_item_icons_with_input_context(
        world,
        item_runtime,
        partial_tick,
        false,
        ItemModelKeybindContext::default(),
    )
}

#[cfg(test)]
fn hotbar_item_icons_with_extended_view(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
    shift_down: bool,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    hotbar_item_icons_with_input_context(
        world,
        item_runtime,
        partial_tick,
        shift_down,
        ItemModelKeybindContext::default(),
    )
}

fn hotbar_item_icons_with_input_context(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    let mut icons = std::array::from_fn(|_| None);
    let selected_slot = usize::from(world.local_player().selected_hotbar_slot.min(8));
    let using_selected_item = world.local_player().interaction.using_item;
    for (slot_index, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        let selected_item = slot_index == selected_slot;
        let fishing_rod_cast =
            local_player_fishing_rod_casts_item(world, item_runtime, &item.item, selected_item);
        icons[slot_index] = hud_item_icon_for_stack(
            world,
            item_runtime,
            &item.item,
            item.local_selected_bundle_item_index(),
            using_selected_item && selected_item,
            selected_item,
            false,
            fishing_rod_cast,
            shift_down,
            keybind_context,
            i32::try_from(slot_index + 1).unwrap_or(0),
            partial_tick,
        );
    }

    icons
}

/// The 3D inventory-icon model for a stack, when the item is a block (vanilla 3D inventory item
/// rendering): its block model quads (atlas-absolute over the blocks atlas) plus its `gui` display
/// transform. `None` for an empty stack or a flat / generated item (which keeps its 2D sprite). Shared
/// by every GUI consumer (hotbar + inventory-screen slots + floating items).
#[allow(clippy::too_many_arguments)]
fn block_item_3d_model(
    world: &WorldStore,
    item: &bbb_protocol::packets::ItemStackSummary,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
) -> Option<HudBlockItemModel> {
    block_item_3d_model_with_owner_team_color(
        world,
        item,
        item_runtime,
        terrain_textures,
        local_selected_bundle_item_index,
        using_item,
        selected_item,
        carried_item,
        fishing_rod_cast,
        shift_down,
        keybind_context,
        item_model_seed,
        world.local_player_team_color_rgb(),
    )
}

#[allow(clippy::too_many_arguments)]
fn fake_block_item_3d_model(
    world: &WorldStore,
    item: &bbb_protocol::packets::ItemStackSummary,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
) -> Option<HudBlockItemModel> {
    block_item_3d_model_with_owner_team_color(
        world,
        item,
        item_runtime,
        terrain_textures,
        local_selected_bundle_item_index,
        using_item,
        selected_item,
        carried_item,
        fishing_rod_cast,
        shift_down,
        keybind_context,
        item_model_seed,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn block_item_3d_model_with_owner_team_color(
    world: &WorldStore,
    item: &bbb_protocol::packets::ItemStackSummary,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    owner_team_color_rgb: Option<u32>,
) -> Option<HudBlockItemModel> {
    let item_runtime = item_runtime?;
    let item_id = item.item_id?;
    let resolved = hud_resolved_item_model_for_stack(
        world,
        item_runtime,
        item,
        local_selected_bundle_item_index,
        using_item,
        selected_item,
        carried_item,
        fishing_rod_cast,
        shift_down,
        keybind_context,
        item_model_seed,
        owner_team_color_rgb,
    );
    if resolved.as_ref().is_some_and(|resolved| {
        resolved
            .layers
            .iter()
            .any(|layer| matches!(layer, ResolvedItemModelLayer::Special { .. }))
    }) {
        let gui_fallback = item_runtime
            .item_display_transform_for_stack(item, bbb_pack::BlockModelDisplayContext::Gui)
            .unwrap_or_default();
        let mut layers = Vec::new();
        for layer in resolved.into_iter().flat_map(|resolved| resolved.layers) {
            match layer {
                ResolvedItemModelLayer::Ordinary {
                    icon,
                    transformation,
                    display_transform,
                } => {
                    let transform =
                        crate::item_models::display_matrix(
                            &display_transform.unwrap_or(gui_fallback),
                            false,
                        ) * crate::item_models::special_item_leaf_matrix(transformation);
                    layers.extend(
                        item_runtime
                            .generated_item_layers_for_icon(icon)
                            .into_iter()
                            .map(|layer| HudResidentItemLayer::Geometry {
                                atlas: ItemModelAtlasKind::Items,
                                identity: ItemModelAssetIdentity::new(
                                    layer.asset_id,
                                    Arc::<str>::from("generated"),
                                    0,
                                ),
                                geometry_class: if layer.translucent {
                                    ItemModelGeometryClass::Translucent
                                } else {
                                    ItemModelGeometryClass::Solid
                                },
                                source: ItemModelAssetSource::Generated {
                                    mask: layer.mask,
                                    rect: layer.rect,
                                    translucent: layer.translucent,
                                },
                                transform,
                                tint: layer.tint,
                            }),
                    );
                }
                ResolvedItemModelLayer::Special {
                    model,
                    transformation,
                    display_transform,
                } => {
                    if let Some(model) = crate::item_models::project_special_item_model(
                        &model,
                        item,
                        item_runtime,
                        None,
                        None,
                    ) {
                        layers.push(HudResidentItemLayer::Special {
                            model,
                            transform: crate::item_models::display_matrix(
                                &display_transform.unwrap_or(gui_fallback),
                                false,
                            ) * crate::item_models::special_item_leaf_matrix(
                                transformation,
                            ),
                        });
                    }
                }
            }
        }
        if !layers.is_empty() {
            return Some(HudBlockItemModel {
                layers: layers.into(),
                lighting: GuiItemLightingEntry::Items3d,
                foil: item.has_foil(),
            });
        }
    }
    let resource_id = item_runtime.item_resource_id(item_id)?;
    let quads = terrain_textures.block_item_quads(resource_id, &BTreeMap::new())?;
    if quads.is_empty() {
        return None;
    }
    let gui = item_runtime
        .item_display_transform_for_stack(item, bbb_pack::BlockModelDisplayContext::Gui)
        .unwrap_or_default();
    let transform = crate::item_models::display_matrix(&gui, false);
    let layers = hud_block_geometry_layers(resource_id, quads, transform);
    Some(HudBlockItemModel {
        layers,
        lighting: GuiItemLightingEntry::Items3d,
        foil: item.has_foil(),
    })
}

fn hud_block_geometry_layers(
    resource_id: &str,
    quads: Arc<[ItemModelQuad]>,
    transform: glam::Mat4,
) -> Arc<[HudResidentItemLayer]> {
    let identity = ItemModelAssetIdentity::new(resource_id, "default", 0);
    [
        ItemModelGeometryClass::Solid,
        ItemModelGeometryClass::Translucent,
    ]
    .into_iter()
    .filter(|geometry_class| {
        let translucent = *geometry_class == ItemModelGeometryClass::Translucent;
        quads.iter().any(|quad| quad.translucent == translucent)
    })
    .map(|geometry_class| HudResidentItemLayer::Geometry {
        atlas: ItemModelAtlasKind::Blocks,
        identity: identity.clone(),
        geometry_class,
        source: ItemModelAssetSource::Quads(Arc::clone(&quads)),
        transform,
        tint: [1.0; 4],
    })
    .collect::<Vec<_>>()
    .into()
}

/// The hotbar's 3D block items (vanilla inventory item rendering): for each slot holding a block item,
/// its [`block_item_3d_model`], so the renderer draws it as a 3D icon instead of the flat 2D sprite.
/// `None` for empty slots and flat items (which keep the 2D sprite drawn by the HUD layer).
fn hotbar_block_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
) -> Vec<Option<HudBlockItemModel>> {
    let mut models: Vec<Option<HudBlockItemModel>> = (0..HUD_HOTBAR_SLOTS).map(|_| None).collect();
    let selected_slot = usize::from(world.local_player().selected_hotbar_slot.min(8));
    let using_selected_item = world.local_player().interaction.using_item;
    for (slot, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        if slot >= HUD_HOTBAR_SLOTS {
            break;
        }
        let selected_item = slot == selected_slot;
        let fishing_rod_cast =
            local_player_fishing_rod_casts_item(world, item_runtime, &item.item, selected_item);
        models[slot] = block_item_3d_model(
            world,
            &item.item,
            item_runtime,
            terrain_textures,
            item.local_selected_bundle_item_index(),
            using_selected_item && selected_item,
            selected_item,
            false,
            fishing_rod_cast,
            shift_down,
            keybind_context,
            i32::try_from(slot + 1).unwrap_or(0),
        );
    }
    models
}

fn release_input_if_screen_opened(
    input_screen_was_open: bool,
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if !input_screen_was_open && input_screen_is_open(input, world) {
        input.interrupt_chat_screen();
        release_active_input(input, world, counters, net_commands);
    }
}

fn input_screen_is_open(input: &ClientInputState, world: &WorldStore) -> bool {
    input.disconnected_screen_runtime().is_some()
        || input.chat_restrictions_screen_is_open()
        || world.open_container_id().is_some()
        || world.death_screen().is_some()
        || world.game_event_screen().is_some()
        || world.dialog_screen_is_open()
        || world.current_book().is_some()
        || world.advancements_screen_is_open()
        || world.stats_screen_is_open()
        || world.social_interactions_screen().is_some()
        || world.game_rules_screen().is_some()
        || world.command_block_editor().is_some()
        || world.minecart_command_block_editor().is_some()
        || world.structure_block_editor().is_some()
        || world.jigsaw_block_editor().is_some()
        || world.test_block_editor().is_some()
        || world.test_instance_block_editor().is_some()
        || input.debug_options_screen_is_open()
        || input.debug_pause_screen_is_open()
        || input.sign_editor_is_active_or_pending(world)
}

fn hud_pause_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudPauseScreen> {
    let state = input.debug_pause_screen()?;
    let feedback_subscreen = state.page == PauseScreenPage::Feedback;
    let return_to_game_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_return_to_game_button_contains(state.cursor_position, surface_size);
    let advancements_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_advancements_button_contains(state.cursor_position, surface_size);
    let stats_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_stats_button_contains(state.cursor_position, surface_size);
    let send_feedback_hovered = state.show_pause_menu
        && if feedback_subscreen {
            pause_screen_feedback_send_feedback_button_contains(state.cursor_position, surface_size)
        } else {
            pause_screen_send_feedback_button_contains(state.cursor_position, surface_size)
        };
    let report_bugs_hovered = state.show_pause_menu
        && pause_screen_report_bugs_button_enabled()
        && if feedback_subscreen {
            pause_screen_feedback_report_bugs_button_contains(state.cursor_position, surface_size)
        } else {
            state.custom_dialog.is_none()
                && pause_screen_report_bugs_button_contains(state.cursor_position, surface_size)
        };
    let custom_dialog_hovered = state.show_pause_menu
        && !feedback_subscreen
        && state.custom_dialog.is_some()
        && pause_screen_report_bugs_button_contains(state.cursor_position, surface_size);
    let feedback_back_hovered = state.show_pause_menu
        && feedback_subscreen
        && pause_screen_feedback_back_button_contains(state.cursor_position, surface_size);
    let options_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_options_button_enabled()
        && pause_screen_options_button_contains(state.cursor_position, surface_size);
    let player_reporting_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_player_reporting_button_enabled()
        && pause_screen_player_reporting_button_contains(state.cursor_position, surface_size);
    let disconnect_enabled = !input.pause_screen_disconnect_requested();
    let disconnect_hovered = state.show_pause_menu
        && !feedback_subscreen
        && disconnect_enabled
        && pause_screen_disconnect_button_contains(state.cursor_position, surface_size);
    let custom_dialog_label = state
        .custom_dialog
        .as_ref()
        .map(|dialog| dialog.external_title().text.clone());
    let custom_dialog_tooltip = custom_dialog_hovered.then(|| {
        let (x, y) = state.cursor_position.unwrap_or_default();
        HudPauseScreenTooltip {
            text: localized_gui_text(
                item_runtime,
                "menu.custom_options.tooltip",
                "Note: Custom options are provided by third-party servers and/or content.\nHandle with care!",
            ),
            x,
            y,
        }
    });
    Some(HudPauseScreen {
        title: if feedback_subscreen {
            localized_gui_text(item_runtime, "menu.feedback.title", "Feedback")
        } else if state.show_pause_menu {
            localized_gui_text(item_runtime, "menu.game", "Game Menu")
        } else {
            localized_gui_text(item_runtime, "menu.paused", "Game Paused")
        },
        show_pause_menu: state.show_pause_menu,
        feedback_subscreen,
        return_to_game_hovered,
        advancements_hovered,
        stats_hovered,
        send_feedback_hovered,
        report_bugs_hovered,
        report_bugs_enabled: pause_screen_report_bugs_button_enabled(),
        custom_dialog_label,
        custom_dialog_hovered,
        custom_dialog_tooltip,
        feedback_back_hovered,
        options_hovered,
        options_enabled: pause_screen_options_button_enabled(),
        player_reporting_hovered,
        player_reporting_enabled: pause_screen_player_reporting_button_enabled(),
        disconnect_hovered,
        disconnect_enabled,
    })
}

fn hud_stats_screen(
    input: &mut ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudStatsScreen> {
    world.stats_screen_is_open().then(|| {
        let is_loading = world.stats_screen_is_loading();
        let mut general_rows = bbb_protocol::stat_types::VANILLA_CUSTOM_STATS
            .iter()
            .map(|entry| {
                let translation_key = format!("stat.{}", entry.resource_id.replace(':', "."));
                let label = localized_gui_text(item_runtime, &translation_key, entry.resource_id);
                let amount = world
                    .client_stats()
                    .identities
                    .iter()
                    .find(|identity| {
                        identity.stat_type_resource_id.as_deref() == Some("minecraft:custom")
                            && identity.value_resource_id.as_deref() == Some(entry.resource_id)
                    })
                    .and_then(|identity| world.stat_value(identity.stat_type_id, identity.value_id))
                    .unwrap_or_default();
                HudStatsRow {
                    label,
                    value: format_vanilla_stat_value(entry.formatter, amount),
                    secondary_value: None,
                }
            })
            .collect::<Vec<_>>();
        general_rows.sort_by(|left, right| left.label.cmp(&right.label));
        let mut mob_rows = bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
            .iter()
            .filter_map(|entry| {
                let kills =
                    stat_amount_by_resource_identity(world, "minecraft:killed", entry.resource_id);
                let killed_by = stat_amount_by_resource_identity(
                    world,
                    "minecraft:killed_by",
                    entry.resource_id,
                );
                (kills > 0 || killed_by > 0).then(|| {
                    let name_key = format!("entity.{}", entry.resource_id.replace(':', "."));
                    let name = localized_gui_text(item_runtime, &name_key, entry.resource_id);
                    let kills_text = if kills == 0 {
                        localized_gui_text_with_args(
                            item_runtime,
                            "stat_type.minecraft.killed.none",
                            "You have never killed %1$s",
                            &[&name],
                        )
                    } else {
                        localized_gui_text_with_args(
                            item_runtime,
                            "stat_type.minecraft.killed",
                            "You killed %1$s %2$s",
                            &[&format_grouped_integer(kills), &name],
                        )
                    };
                    let killed_by_text = if killed_by == 0 {
                        localized_gui_text_with_args(
                            item_runtime,
                            "stat_type.minecraft.killed_by.none",
                            "You have never been killed by %1$s",
                            &[&name],
                        )
                    } else {
                        localized_gui_text_with_args(
                            item_runtime,
                            "stat_type.minecraft.killed_by",
                            "%1$s killed you %2$s time(s)",
                            &[&name, &format_grouped_integer(killed_by)],
                        )
                    };
                    HudStatsRow {
                        label: name,
                        value: kills_text,
                        secondary_value: Some(killed_by_text),
                    }
                })
            })
            .collect::<Vec<_>>();
        mob_rows.sort_by(|left, right| left.label.cmp(&right.label));
        let mut item_rows = stats_item_rows(
            input,
            world,
            item_runtime,
            terrain_textures,
            keybind_context,
            partial_tick,
        );
        let items_tab_enabled = !item_rows.is_empty();
        let mobs_tab_enabled = !mob_rows.is_empty();
        input.sync_stats_screen_tab_row_counts([
            general_rows.len(),
            item_rows.len(),
            mob_rows.len(),
        ]);
        let active_tab = match input.stats_screen_tab() {
            1 if !is_loading && items_tab_enabled => 1,
            2 if !is_loading && mobs_tab_enabled => 2,
            _ => 0,
        };
        if active_tab != input.stats_screen_tab() {
            input.reset_stats_screen_scroll();
        }
        let row_height = match active_tab {
            1 => 22,
            2 => 36,
            _ => 14,
        };
        let rows = match active_tab {
            2 => mob_rows,
            0 => general_rows,
            _ => Vec::new(),
        };
        let total_rows = if active_tab == 1 {
            item_rows.len()
        } else {
            rows.len()
        };
        input.sync_stats_screen_interaction(is_loading, active_tab, total_rows);
        let viewport_height = stats_screen_viewport_height(surface_size, row_height);
        let content_entries = if is_loading {
            0
        } else {
            total_rows.saturating_add(usize::from(active_tab == 1))
        };
        let content_height = if content_entries == 0 {
            0
        } else {
            i32::try_from(content_entries)
                .unwrap_or(i32::MAX / row_height)
                .saturating_mul(row_height)
                .saturating_add(4)
        };
        let max_scroll_amount = content_height.saturating_sub(viewport_height).max(0);
        let scroller_height =
            stats_screen_scroller_height(viewport_height, content_height, max_scroll_amount);
        input.sync_stats_screen_scroll_metrics(max_scroll_amount, scroller_height);
        input.keep_stats_screen_focus_visible(viewport_height, row_height);
        let scroll_amount = input.stats_screen_scroll_amount() as i32;
        let scroll_row =
            stats_screen_first_data_row(active_tab, scroll_amount, row_height).min(total_rows);
        let first_row_offset =
            stats_screen_first_data_row_offset(active_tab, scroll_row, scroll_amount, row_height);
        let visible_rows = if is_loading {
            0
        } else {
            stats_screen_projected_row_count(
                total_rows.saturating_sub(scroll_row),
                viewport_height,
                first_row_offset,
                row_height,
            )
        };
        let (item_sort_column, item_sort_order) = input.stats_screen_item_sort();
        let item_column_labels = [
            localized_gui_text(item_runtime, "stat_type.minecraft.mined", "Mined"),
            localized_gui_text(item_runtime, "stat_type.minecraft.broken", "Broken"),
            localized_gui_text(item_runtime, "stat_type.minecraft.crafted", "Crafted"),
            localized_gui_text(item_runtime, "stat_type.minecraft.used", "Used"),
            localized_gui_text(item_runtime, "stat_type.minecraft.picked_up", "Picked Up"),
            localized_gui_text(item_runtime, "stat_type.minecraft.dropped", "Dropped"),
        ];
        let item_hovered_column = input.stats_screen_item_hovered_column(surface_size);
        let item_tooltip_text = if active_tab == 1 {
            input
                .stats_screen_hovered_item_row(surface_size)
                .and_then(|row| item_rows.get(row))
                .map(|row| row.label.clone())
                .or_else(|| item_hovered_column.map(|column| item_column_labels[column].clone()))
        } else {
            None
        };
        let rows = if is_loading || active_tab == 1 {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(scroll_row)
                .take(visible_rows)
                .collect()
        };
        item_rows = if active_tab == 1 {
            item_rows
                .into_iter()
                .skip(scroll_row)
                .take(visible_rows)
                .collect()
        } else {
            Vec::new()
        };
        let tooltip_text = item_tooltip_text.or_else(|| {
            (!is_loading)
                .then(|| match input.stats_screen_hovered_tab(surface_size) {
                    Some(1) if !items_tab_enabled => Some(localized_gui_text(
                        item_runtime,
                        "gui.stats.none_found",
                        "No statistics found",
                    )),
                    Some(2) if !mobs_tab_enabled => Some(localized_gui_text(
                        item_runtime,
                        "gui.stats.none_found",
                        "No statistics found",
                    )),
                    _ => None,
                })
                .flatten()
        });
        let tooltip = tooltip_text.and_then(|text| {
            input
                .stats_screen_cursor_position()
                .map(|(x, y)| HudDebugOptionsTooltip { text, x, y })
        });
        HudStatsScreen {
            title: localized_gui_text(item_runtime, "gui.stats", "Stats"),
            loading_text: localized_gui_text(
                item_runtime,
                "multiplayer.downloadingStats",
                "Downloading statistics...",
            ),
            is_loading,
            general_tab: localized_gui_text(item_runtime, "stat.generalButton", "General"),
            items_tab: localized_gui_text(item_runtime, "stat.itemsButton", "Items"),
            mobs_tab: localized_gui_text(item_runtime, "stat.mobsButton", "Mobs"),
            active_tab,
            items_tab_enabled: items_tab_enabled && !is_loading,
            mobs_tab_enabled: mobs_tab_enabled && !is_loading,
            rows,
            item_rows,
            item_column_labels,
            item_no_value: localized_gui_text(item_runtime, "stats.none", "-"),
            item_sort_column,
            item_sort_order,
            item_hovered_column,
            item_focused_column: match input.stats_screen_focus() {
                StatsScreenFocus::ItemHeader(column) => Some(column),
                _ => None,
            },
            scroll_row,
            scroll_amount,
            max_scroll_amount,
            scroller_height,
            total_rows,
            visible_rows,
            row_height,
            tooltip,
            done_hovered: stats_screen_done_button_contains(
                input.stats_screen_cursor_position(),
                surface_size,
            ),
            done_focused: input.stats_screen_focus() == StatsScreenFocus::Done,
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsNarrationWidget {
    Tabs,
    List,
    Done,
}

fn stats_screen_narration_frame(
    input: &ClientInputState,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());

    let focus = input.stats_screen_focus();
    let focused_widget = match focus {
        StatsScreenFocus::Tabs => Some(StatsNarrationWidget::Tabs),
        StatsScreenFocus::List
        | StatsScreenFocus::GeneralRow(_)
        | StatsScreenFocus::ItemHeader(_)
        | StatsScreenFocus::ItemRow(_)
        | StatsScreenFocus::MobRow(_) => Some(StatsNarrationWidget::List),
        StatsScreenFocus::Done => Some(StatsNarrationWidget::Done),
        StatsScreenFocus::None => None,
    };
    let hovered_tab = input.stats_screen_hovered_tab(surface_size);
    let hovered_row = input.stats_screen_hovered_row(surface_size);
    let list_hovered = hovered_row.is_some() || screen.item_hovered_column.is_some();
    let selected_widget = focused_widget.or_else(|| {
        hovered_tab
            .map(|_| StatsNarrationWidget::Tabs)
            .or_else(|| list_hovered.then_some(StatsNarrationWidget::List))
            .or_else(|| screen.done_hovered.then_some(StatsNarrationWidget::Done))
    });
    let active_widget_count = if screen.is_loading { 2 } else { 3 };
    if let Some(widget) = selected_widget {
        let position = match widget {
            StatsNarrationWidget::Tabs => 1,
            StatsNarrationWidget::List => 2,
            StatsNarrationWidget::Done if screen.is_loading => 2,
            StatsNarrationWidget::Done => 3,
        };
        frame.add(
            NarratedElementKind::Position,
            0,
            localized_gui_text_with_args(
                item_runtime,
                "narrator.position.screen",
                "Screen element %1$s out of %2$s",
                &[&position.to_string(), &active_widget_count.to_string()],
            ),
        );
        if focused_widget == Some(widget) {
            frame.add(
                NarratedElementKind::Usage,
                0,
                localized_gui_text(
                    item_runtime,
                    "narration.component_list.usage",
                    "Press Tab to navigate to the next element",
                ),
            );
        }
        match widget {
            StatsNarrationWidget::Tabs => stats_screen_tab_narration(
                &mut frame,
                screen,
                item_runtime,
                hovered_tab.unwrap_or(screen.active_tab),
                focus == StatsScreenFocus::Tabs,
            ),
            StatsNarrationWidget::List if !screen.is_loading => {
                stats_screen_list_narration(&mut frame, input, screen, item_runtime, hovered_row)
            }
            StatsNarrationWidget::Done => stats_screen_done_narration(
                &mut frame,
                item_runtime,
                focus == StatsScreenFocus::Done,
            ),
            StatsNarrationWidget::List => {}
        }
    } else {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narrator.screen.usage",
                "Use mouse cursor or Tab button to select element",
            ),
        );
    }
    frame
}

fn stats_screen_tab_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    tab: usize,
    focused: bool,
) {
    let tab = tab.min(2);
    let labels = [
        screen.general_tab.as_str(),
        screen.items_tab.as_str(),
        screen.mobs_tab.as_str(),
    ];
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.tab", "%1$s tab", &[labels[tab]]),
    );
    frame.add(
        NarratedElementKind::Position,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.tab",
            "Selected tab %1$s out of %2$s",
            &[&(tab + 1).to_string(), "3"],
        ),
    );
    let tab_enabled =
        tab == 0 || (tab == 1 && screen.items_tab_enabled) || (tab == 2 && screen.mobs_tab_enabled);
    if screen.is_loading {
        frame.add(NarratedElementKind::Hint, 1, screen.loading_text.clone());
    } else if !tab_enabled {
        frame.add(
            NarratedElementKind::Hint,
            1,
            localized_gui_text(item_runtime, "gui.stats.none_found", "No statistics found"),
        );
    }
    if focused {
        frame.add(
            NarratedElementKind::Usage,
            1,
            localized_gui_text(
                item_runtime,
                "narration.tab_navigation.usage",
                "Press Ctrl and Tab to switch between tabs",
            ),
        );
    }
}

fn stats_screen_list_narration(
    frame: &mut ScreenNarrationFrame,
    input: &ClientInputState,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_row: Option<usize>,
) {
    let list_focused = matches!(
        input.stats_screen_focus(),
        StatsScreenFocus::List
            | StatsScreenFocus::GeneralRow(_)
            | StatsScreenFocus::ItemHeader(_)
            | StatsScreenFocus::ItemRow(_)
            | StatsScreenFocus::MobRow(_)
    );
    if screen.active_tab == 1 {
        let hovered_header = screen.item_hovered_column;
        if let Some(row) = hovered_row {
            stats_screen_item_row_narration(frame, screen, item_runtime, row);
        } else if let Some(column) = hovered_header {
            let column = match input.stats_screen_focus() {
                StatsScreenFocus::ItemHeader(focused) => focused,
                _ => column,
            };
            stats_screen_item_header_narration(
                frame,
                screen,
                item_runtime,
                column,
                input.stats_screen_focus() == StatsScreenFocus::ItemHeader(column),
            );
        } else {
            match input.stats_screen_focus() {
                StatsScreenFocus::ItemHeader(column) => {
                    stats_screen_item_header_narration(frame, screen, item_runtime, column, true)
                }
                StatsScreenFocus::ItemRow(row) => {
                    stats_screen_item_row_narration(frame, screen, item_runtime, row)
                }
                _ => {}
            }
        }
        return;
    }

    let focused_row = match input.stats_screen_focus() {
        StatsScreenFocus::GeneralRow(row) | StatsScreenFocus::MobRow(row) => Some(row),
        _ => None,
    };
    if let Some(row_index) = hovered_row.or(focused_row) {
        if let Some(row) = row_index
            .checked_sub(screen.scroll_row)
            .and_then(|visible| screen.rows.get(visible))
        {
            let selection = match row.secondary_value.as_deref() {
                Some(secondary) => format!("{}. {}", row.value, secondary),
                None => format!("{} {}", row.label, row.value),
            };
            frame.add(
                NarratedElementKind::Title,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "narrator.select",
                    "Selected: %1$s",
                    &[&selection],
                ),
            );
            add_stats_list_position(frame, item_runtime, row_index + 1, screen.total_rows, 2);
        }
    }
    if list_focused {
        frame.add(
            NarratedElementKind::Usage,
            1,
            localized_gui_text(
                item_runtime,
                "narration.selection.usage",
                "Press the up and down buttons to move to another entry",
            ),
        );
    }
}

fn stats_screen_item_header_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    column: usize,
    focused: bool,
) {
    let column = column.min(5);
    add_stats_list_position(frame, item_runtime, 1, screen.total_rows + 1, 1);
    frame.add(
        NarratedElementKind::Title,
        3,
        localized_gui_text_with_args(
            item_runtime,
            "gui.narrate.button",
            "%1$s button",
            &[&screen.item_column_labels[column]],
        ),
    );
    frame.add(
        NarratedElementKind::Position,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.object_list",
            "Selected row element %1$s out of %2$s",
            &[&(column + 1).to_string(), "6"],
        ),
    );
    frame.add(
        NarratedElementKind::Hint,
        3,
        screen.item_column_labels[column].clone(),
    );
    frame.add(
        NarratedElementKind::Usage,
        3,
        localized_gui_text(
            item_runtime,
            if focused {
                "narration.button.usage.focused"
            } else {
                "narration.button.usage.hovered"
            },
            if focused {
                "Press Enter to activate"
            } else {
                "Left click to activate"
            },
        ),
    );
}

fn stats_screen_item_row_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    row: usize,
) {
    let Some(item) = row
        .checked_sub(screen.scroll_row)
        .and_then(|visible| screen.item_rows.get(visible))
    else {
        return;
    };
    add_stats_list_position(frame, item_runtime, row + 2, screen.total_rows + 1, 1);
    frame.add(
        NarratedElementKind::Title,
        3,
        localized_gui_text_with_args(item_runtime, "narration.item", "Item: %1$s", &[&item.label]),
    );
}

fn add_stats_list_position(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    position: usize,
    total: usize,
    depth: u8,
) {
    if total <= 1 {
        return;
    }
    frame.add(
        NarratedElementKind::Position,
        depth,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.list",
            "Selected list row %1$s out of %2$s",
            &[&position.to_string(), &total.to_string()],
        ),
    );
}

fn stats_screen_done_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    focused: bool,
) {
    let done = localized_gui_text(item_runtime, "gui.done", "Done");
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[&done]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            if focused {
                "narration.button.usage.focused"
            } else {
                "narration.button.usage.hovered"
            },
            if focused {
                "Press Enter to activate"
            } else {
                "Left click to activate"
            },
        ),
    );
}

fn localized_gui_text(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
) -> String {
    item_runtime
        .map(|runtime| runtime.localized_text(translation_key))
        .filter(|text| *text != translation_key)
        .unwrap_or(fallback)
        .to_string()
}

fn chat_screen_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let status = input.chat_screen_status(world.chat_presentation_config().visibility)?;
    if status.screen == bbb_control::ChatScreenControlKind::Restrictions {
        let screen = hud_chat_restrictions_screen(input, world, item_runtime, None)?;
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Title, 0, screen.title);
        frame.add(NarratedElementKind::Hint, 0, screen.reason.text);
        frame.add(
            NarratedElementKind::Hint,
            1,
            screen
                .permissions
                .iter()
                .map(|permission| permission.text.as_str())
                .collect::<Vec<_>>()
                .join(". "),
        );
        add_game_event_button_narration(&mut frame, item_runtime, &screen.done, 1, 1);
        return Some(frame);
    }

    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        localized_gui_text(item_runtime, "chat_screen.title", "Chat screen"),
    );
    let usage = localized_gui_text(
        item_runtime,
        "chat_screen.usage",
        "Input message and press Enter to send",
    );
    let usage = if status.can_send_messages && status.can_send_commands {
        usage
    } else {
        [
            usage,
            localized_gui_text(
                item_runtime,
                "chat_screen.restricted.narration",
                "Chat is restricted. Go to the Restrictions screen in World Options.",
            ),
        ]
        .join(". ")
    };
    frame.add(NarratedElementKind::Usage, 0, usage);
    if !status.input_text.is_empty() {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "chat_screen.message",
                "Message to send: %1$s",
                &[&status.input_text],
            ),
        );
    }
    let edit_box_label = localized_gui_text(item_runtime, "chat.editBox", "chat");
    let mut edit_box = localized_gui_text_with_args(
        item_runtime,
        "gui.narrate.editBox",
        "%1$s edit box: %2$s",
        &[&edit_box_label, &status.input_text],
    );
    if let Some(suggestion) = input.command_suggestion_narration() {
        edit_box.push_str(". ");
        edit_box.push_str(&localized_command_suggestion_narration(
            item_runtime,
            suggestion,
        ));
    }
    frame.add(NarratedElementKind::Title, 2, edit_box);
    Some(frame)
}

fn disconnected_screen_narration_frame(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let screen = hud_disconnected_screen(input, item_runtime, 0.0)?;
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    let reason = screen
        .reason_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    if !reason.is_empty() {
        frame.add(NarratedElementKind::Hint, 0, reason);
    }
    let (label, position, total) = match screen.focused {
        bbb_renderer::HudDisconnectedScreenButton::ReportToServer => (
            screen.report_to_server_label.as_deref()?,
            1,
            1 + usize::from(screen.open_report_directory_label.is_some()) + 1,
        ),
        bbb_renderer::HudDisconnectedScreenButton::OpenReportDirectory => (
            screen.open_report_directory_label.as_deref()?,
            1 + usize::from(screen.report_to_server_label.is_some()),
            usize::from(screen.report_to_server_label.is_some()) + 2,
        ),
        bbb_renderer::HudDisconnectedScreenButton::Back => (
            screen.back_label.as_str(),
            usize::from(screen.report_to_server_label.is_some())
                + usize::from(screen.open_report_directory_label.is_some())
                + 1,
            usize::from(screen.report_to_server_label.is_some())
                + usize::from(screen.open_report_directory_label.is_some())
                + 1,
        ),
    };
    add_game_event_button_narration(&mut frame, item_runtime, label, position, total);
    Some(frame)
}

fn death_screen_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let screen = world.death_screen()?;
    let runtime = input
        .death_screen_runtime()
        .filter(|runtime| runtime.generation() == screen.generation)?;
    let mut frame = ScreenNarrationFrame::default();
    match runtime {
        DeathScreenRuntime::Main(state) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    if screen.hardcore {
                        "deathScreen.title.hardcore"
                    } else {
                        "deathScreen.title"
                    },
                    if screen.hardcore {
                        "Game Over!"
                    } else {
                        "You Died!"
                    },
                ),
            );
            let (label, position) = match state.focus {
                bbb_renderer::HudDeathScreenButton::Respawn => (
                    localized_gui_text(
                        item_runtime,
                        if screen.hardcore {
                            "deathScreen.spectate"
                        } else {
                            "deathScreen.respawn"
                        },
                        if screen.hardcore {
                            "Spectate World"
                        } else {
                            "Respawn"
                        },
                    ),
                    1,
                ),
                bbb_renderer::HudDeathScreenButton::TitleScreen => (
                    localized_gui_text(item_runtime, "deathScreen.titleScreen", "Title Screen"),
                    2,
                ),
                _ => return Some(frame),
            };
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
        }
        DeathScreenRuntime::DraftConfirmation(state) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    localized_gui_text(
                        item_runtime,
                        "gui.abuseReport.draft.quittotitle.title",
                        "You have a draft chat report that will be lost if you quit",
                    ),
                    localized_gui_text(
                        item_runtime,
                        "gui.abuseReport.draft.quittotitle.content",
                        "Would you like to continue editing it or discard it?",
                    ),
                ]
                .join(". "),
            );
            let (key, fallback, position) = match state.focus {
                bbb_renderer::HudDeathScreenButton::DraftEdit => {
                    ("gui.abuseReport.draft.edit", "Edit Draft", 1)
                }
                bbb_renderer::HudDeathScreenButton::DraftDiscard => {
                    ("gui.abuseReport.draft.discard", "Discard Draft", 2)
                }
                _ => return Some(frame),
            };
            let label = localized_gui_text(item_runtime, key, fallback);
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
        }
        DeathScreenRuntime::QuitConfirmation(state) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    "deathScreen.quit.confirm",
                    "Are you sure you want to quit?",
                ),
            );
            let (label, position) = match state.focus {
                bbb_renderer::HudDeathScreenButton::QuitTitleScreen => (
                    localized_gui_text(item_runtime, "deathScreen.titleScreen", "Title Screen"),
                    1,
                ),
                bbb_renderer::HudDeathScreenButton::QuitRespawn => (
                    localized_gui_text(item_runtime, "deathScreen.respawn", "Respawn"),
                    2,
                ),
                _ => return Some(frame),
            };
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
        }
        DeathScreenRuntime::LinkConfirmation(state) => {
            let confirmation = world.dialog_link_confirmation()?;
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    localized_gui_text(
                        item_runtime,
                        "chat.link.confirmTrusted",
                        "Do you want to open this link or copy it to your clipboard?",
                    ),
                    confirmation.url.clone(),
                ]
                .join(". "),
            );
            let (key, fallback, position) = match state.focus {
                bbb_renderer::HudDeathScreenButton::LinkOpen => {
                    ("chat.link.open", "Open in Browser", 1)
                }
                bbb_renderer::HudDeathScreenButton::LinkCopy => {
                    ("chat.copy", "Copy to Clipboard", 2)
                }
                bbb_renderer::HudDeathScreenButton::LinkCancel => ("gui.cancel", "Cancel", 3),
                _ => return Some(frame),
            };
            let label = localized_gui_text(item_runtime, key, fallback);
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 3);
        }
    }
    Some(frame)
}

fn social_interactions_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let runtime = input.social_interactions_runtime()?;
    let screen = hud_social_interactions_screen(input, world, item_runtime, None, 0, None)?;
    let mut frame = ScreenNarrationFrame::default();
    match screen {
        HudSocialInteractionsScreen::LinkConfirmation(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title, screen.message].join(". "),
            );
            let label = match runtime.focus() {
                HudSocialInteractionsFocus::LinkOpen => screen.open_label,
                HudSocialInteractionsFocus::LinkCopy => screen.copy_label,
                HudSocialInteractionsFocus::LinkCancel => screen.cancel_label,
                _ => return Some(frame),
            };
            frame.add(NarratedElementKind::Title, 1, label);
            frame.add(
                NarratedElementKind::Usage,
                1,
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.focused",
                    "Press Enter to activate",
                ),
            );
        }
        HudSocialInteractionsScreen::Main(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title.clone(), screen.server_label.clone()].join(". "),
            );
            match runtime.focus() {
                HudSocialInteractionsFocus::Search => {
                    let search = if screen.search_text.is_empty() {
                        screen.search_hint
                    } else {
                        screen.search_text
                    };
                    frame.add(NarratedElementKind::Title, 1, search);
                    if let Some(empty) = screen.empty_text {
                        frame.add(NarratedElementKind::Hint, 1, empty);
                    }
                    frame.add(
                        NarratedElementKind::Usage,
                        1,
                        localized_gui_text(
                            item_runtime,
                            "narration.edit_box.usage",
                            "Type to search",
                        ),
                    );
                }
                HudSocialInteractionsFocus::Tab(page) => {
                    let index = match page {
                        HudSocialInteractionsPage::All => 0,
                        HudSocialInteractionsPage::Hidden => 1,
                        HudSocialInteractionsPage::Blocked => 2,
                    };
                    frame.add(
                        NarratedElementKind::Title,
                        1,
                        screen.tab_labels[index].clone(),
                    );
                    frame.add(
                        NarratedElementKind::Position,
                        1,
                        localized_gui_text_with_args(
                            item_runtime,
                            "narrator.position.object_list",
                            "%1$s out of %2$s",
                            &[&(index + 1).to_string(), "3"],
                        ),
                    );
                }
                HudSocialInteractionsFocus::Player { profile_id, action } => {
                    let Some((index, player)) = screen
                        .rows
                        .iter()
                        .enumerate()
                        .find(|(_, player)| player.profile_id == profile_id)
                    else {
                        return Some(frame);
                    };
                    let action_text = localized_gui_text_with_args(
                        item_runtime,
                        match action {
                            HudSocialInteractionsPlayerAction::Hide => {
                                "gui.socialInteractions.narration.hide"
                            }
                            HudSocialInteractionsPlayerAction::Show => {
                                "gui.socialInteractions.narration.show"
                            }
                            HudSocialInteractionsPlayerAction::Report => {
                                "gui.socialInteractions.narration.report"
                            }
                        },
                        match action {
                            HudSocialInteractionsPlayerAction::Hide => "Hide messages from %s",
                            HudSocialInteractionsPlayerAction::Show => "Show messages from %s",
                            HudSocialInteractionsPlayerAction::Report => "Report %s",
                        },
                        &[&player.name],
                    );
                    frame.add(
                        NarratedElementKind::Title,
                        1,
                        [
                            Some(player.name.clone()),
                            player.status.clone(),
                            Some(action_text),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(", "),
                    );
                    frame.add(
                        NarratedElementKind::Position,
                        1,
                        localized_gui_text_with_args(
                            item_runtime,
                            "narrator.position.object_list",
                            "%1$s out of %2$s",
                            &[&(index + 1).to_string(), &screen.rows.len().to_string()],
                        ),
                    );
                    frame.add(
                        NarratedElementKind::Usage,
                        1,
                        localized_gui_text(
                            item_runtime,
                            "narration.button.usage.focused",
                            "Press Enter to activate",
                        ),
                    );
                }
                HudSocialInteractionsFocus::BlockingHelp => {
                    frame.add(NarratedElementKind::Title, 1, screen.blocking_help_label);
                }
                HudSocialInteractionsFocus::Done => {
                    frame.add(NarratedElementKind::Title, 1, screen.done_label);
                }
                HudSocialInteractionsFocus::LinkOpen
                | HudSocialInteractionsFocus::LinkCopy
                | HudSocialInteractionsFocus::LinkCancel => {}
            }
        }
    }
    Some(frame)
}

fn player_reporting_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let screen = hud_player_reporting_screen(input, world, item_runtime, 0)?;
    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        [
            Some(screen.title.clone()),
            screen.message.clone(),
            screen.status.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(". "),
    );
    if let Some((index, row)) = screen
        .rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.focus == screen.focus)
    {
        frame.add(
            NarratedElementKind::Title,
            1,
            [Some(row.title.clone()), row.detail.clone()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(". "),
        );
        frame.add(
            NarratedElementKind::Position,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "narrator.position.object_list",
                "%1$s out of %2$s",
                &[&(index + 1).to_string(), &screen.rows.len().to_string()],
            ),
        );
    } else if let Some(control) = screen
        .controls
        .iter()
        .find(|control| control.focus == screen.focus)
    {
        frame.add(NarratedElementKind::Title, 1, control.label.clone());
    }
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            if screen.focus == HudPlayerReportingFocus::Comments {
                "narration.edit_box.usage"
            } else {
                "narration.button.usage.focused"
            },
            if screen.focus == HudPlayerReportingFocus::Comments {
                "Type to edit comments"
            } else {
                "Press Enter to activate"
            },
        ),
    );
    Some(frame)
}

fn hud_game_event_screen(
    state: &mut HudRenderRuntime,
    input: &ClientInputState,
    resources: &Arc<crate::win_screen_resources::WinScreenResources>,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudGameEventScreen> {
    state.game_event_screen_narration = None;
    let screen = match input.game_event_screen_runtime()? {
        GameEventScreenRuntime::Win(screen) => HudGameEventScreen::Win(HudWinScreen {
            generation: screen.generation,
            scroll: screen.scroll,
            show_easter_egg_logo: screen.show_easter_egg_logo,
            lines: state.win_screen_lines(resources),
        }),
        GameEventScreenRuntime::DemoHelp(screen) => {
            let movement = localized_gui_text_with_args(
                item_runtime,
                "demo.help.movementShort",
                "Move by pressing the %1$s, %2$s, %3$s, %4$s keys",
                &["W", "A", "S", "D"],
            );
            let mouse = localized_gui_text(
                item_runtime,
                "demo.help.movementMouse",
                "Look around using the mouse",
            );
            let jump = localized_gui_text_with_args(
                item_runtime,
                "demo.help.jump",
                "Jump by pressing the %1$s key",
                &["Space"],
            );
            let inventory = localized_gui_text_with_args(
                item_runtime,
                "demo.help.inventory",
                "Use the %1$s key to open your inventory",
                &["E"],
            );
            HudGameEventScreen::DemoPopup(HudDemoPopupScreen {
                generation: screen.generation,
                title: localized_gui_text(
                    item_runtime,
                    "demo.help.title",
                    "Minecraft Demo Mode",
                ),
                controls_message: [movement, mouse, jump, inventory].join("\n"),
                duration_message: localized_gui_text(
                    item_runtime,
                    "demo.help.fullWrapped",
                    "This demo will last 5 in-game days (about 1 hour and 40 minutes of real time). Check the advancements for hints! Have fun!",
                ),
                buy_label: localized_gui_text(
                    item_runtime,
                    "demo.help.buy",
                    "Purchase Now!",
                ),
                later_label: localized_gui_text(
                    item_runtime,
                    "demo.help.later",
                    "Continue Playing!",
                ),
                focused: match screen.focus {
                    DemoHelpFocus::Buy => HudGameEventButton::DemoBuy,
                    DemoHelpFocus::Later => HudGameEventButton::DemoLater,
                },
                hovered: screen.hovered.map(|hovered| match hovered {
                    DemoHelpFocus::Buy => HudGameEventButton::DemoBuy,
                    DemoHelpFocus::Later => HudGameEventButton::DemoLater,
                }),
            })
        }
        GameEventScreenRuntime::DemoLinkConfirmation(screen) => {
            HudGameEventScreen::DemoLinkConfirmation(HudDemoLinkConfirmationScreen {
                generation: screen.generation,
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: crate::input::game_event_screen::BUY_MINECRAFT_JAVA_URL.to_string(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focused: match screen.focus {
                    DemoLinkConfirmationFocus::Open => HudGameEventButton::LinkOpen,
                    DemoLinkConfirmationFocus::Copy => HudGameEventButton::LinkCopy,
                    DemoLinkConfirmationFocus::Cancel => HudGameEventButton::LinkCancel,
                },
                hovered: screen.hovered.map(|hovered| match hovered {
                    DemoLinkConfirmationFocus::Open => HudGameEventButton::LinkOpen,
                    DemoLinkConfirmationFocus::Copy => HudGameEventButton::LinkCopy,
                    DemoLinkConfirmationFocus::Cancel => HudGameEventButton::LinkCancel,
                }),
            })
        }
    };
    state.game_event_screen_narration = Some(game_event_screen_narration_frame(
        &screen,
        resources,
        item_runtime,
    ));
    Some(screen)
}

fn game_event_screen_narration_frame(
    screen: &HudGameEventScreen,
    resources: &crate::win_screen_resources::WinScreenResources,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    match screen {
        HudGameEventScreen::Win(_) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                resources.narration_components.join(". "),
            );
            frame.add(
                NarratedElementKind::Usage,
                0,
                localized_gui_text(
                    item_runtime,
                    "narrator.screen.usage",
                    "Use mouse cursor or Tab button to select element",
                ),
            );
        }
        HudGameEventScreen::DemoPopup(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    screen.title.as_str(),
                    screen.controls_message.as_str(),
                    screen.duration_message.as_str(),
                ]
                .join(". "),
            );
            add_game_event_button_narration(
                &mut frame,
                item_runtime,
                match screen.focused {
                    HudGameEventButton::DemoBuy => &screen.buy_label,
                    HudGameEventButton::DemoLater => &screen.later_label,
                    _ => return frame,
                },
                match screen.focused {
                    HudGameEventButton::DemoBuy => 4,
                    HudGameEventButton::DemoLater => 5,
                    _ => return frame,
                },
                5,
            );
        }
        HudGameEventScreen::DemoLinkConfirmation(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title.as_str(), screen.message.as_str()].join(". "),
            );
            let (label, position) = match screen.focused {
                HudGameEventButton::LinkOpen => (&screen.open_label, 3),
                HudGameEventButton::LinkCopy => (&screen.copy_label, 4),
                HudGameEventButton::LinkCancel => (&screen.cancel_label, 5),
                _ => return frame,
            };
            add_game_event_button_narration(&mut frame, item_runtime, label, position, 5);
        }
    }
    frame
}

fn add_game_event_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
    position: usize,
    total: usize,
) {
    frame.add(
        NarratedElementKind::Position,
        0,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.screen",
            "Screen element %1$s out of %2$s",
            &[&position.to_string(), &total.to_string()],
        ),
    );
    frame.add(
        NarratedElementKind::Usage,
        0,
        localized_gui_text(
            item_runtime,
            "narration.component_list.usage",
            "Press Tab to navigate to the next element",
        ),
    );
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            "narration.button.usage.focused",
            "Press Enter to activate",
        ),
    );
}

fn hud_mob_effects_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudMobEffect> {
    let Some(local_player_id) = world.local_player_id() else {
        return Vec::new();
    };
    world
        .entity_effects(local_player_id)
        .unwrap_or_default()
        .into_values()
        .filter_map(|effect| {
            let descriptor = vanilla_mob_effect_type(effect.effect_id)?;
            let resource_path = descriptor
                .resource_id
                .strip_prefix("minecraft:")
                .unwrap_or(descriptor.resource_id);
            let name_key = format!("effect.minecraft.{resource_path}");
            let fallback_name = humanize_resource_path(resource_path);
            let mut display_name =
                localized_gui_text(item_runtime, &name_key, &fallback_name).to_string();
            if (1..=9).contains(&effect.amplifier) {
                let level = effect.amplifier + 1;
                let level_key = format!("enchantment.level.{level}");
                let fallback_level = mob_effect_level_fallback(level);
                let localized_level = localized_gui_text(item_runtime, &level_key, fallback_level);
                display_name.push(' ');
                display_name.push_str(&localized_level);
            }
            Some(HudMobEffect {
                effect_id: effect.effect_id,
                beneficial: descriptor.category == MobEffectCategory::Beneficial,
                color: descriptor.color,
                amplifier: effect.amplifier,
                duration_ticks: effect.duration_ticks,
                ambient: effect.ambient,
                show_icon: effect.show_icon,
                display_name,
                duration_text: hud_mob_effect_duration_text(
                    item_runtime,
                    effect.duration_ticks,
                    world.ticking().tick_rate,
                ),
            })
        })
        .collect()
}

fn humanize_resource_path(resource_path: &str) -> String {
    resource_path
        .split(['_', '/'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| {
                    first
                        .to_uppercase()
                        .chain(characters.flat_map(char::to_lowercase))
                        .collect::<String>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn mob_effect_level_fallback(level: i32) -> &'static str {
    match level {
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        _ => "",
    }
}

fn hud_mob_effect_duration_text(
    item_runtime: Option<&NativeItemRuntime>,
    duration_ticks: i32,
    tick_rate: f32,
) -> String {
    if duration_ticks == -1 {
        return localized_gui_text(item_runtime, "effect.duration.infinite", "∞").to_string();
    }
    let tick_rate = if tick_rate.is_finite() && tick_rate > 0.0 {
        tick_rate
    } else {
        20.0
    };
    let total_seconds = ((duration_ticks.max(0) as f32) / tick_rate).floor() as u64;
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3_600;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn hud_styled_runs_from_rich_text(component: &RichTextComponent) -> Vec<HudStyledTextRun> {
    hud_styled_runs_from_component_runs(component.styled())
}

fn hud_styled_runs_from_component_runs(runs: &[StyledTextRun]) -> Vec<HudStyledTextRun> {
    runs.iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| HudStyledTextRun {
            text: run.text.clone(),
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect()
}

fn hud_system_toast(
    state: &mut HudRenderRuntime,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    now: Instant,
) -> (Option<HudSystemToast>, Option<LocalSoundEventState>) {
    let periodic_notification = state.periodic_notifications.update(now).cloned();
    let (frame, title, message, multiline) = if let Some(frame) = state
        .insecure_chat_toast
        .update(world.insecure_chat_warning_count(), now)
    {
        (
            frame,
            localized_gui_text(
                item_runtime,
                "multiplayer.unsecureserver.toast.title",
                "Chat messages can't be verified",
            ),
            localized_gui_text(
                item_runtime,
                "multiplayer.unsecureserver.toast",
                "Messages sent on this server may be modified and might not reflect the original message",
            ),
            true,
        )
    } else if let Some(frame) = state
        .resource_pack_load_failure_toast
        .update(world.resource_pack_load_failure_count(), now)
    {
        (
            frame,
            localized_gui_text(
                item_runtime,
                "resourcePack.load_fail",
                "Resource reload failed",
            ),
            String::new(),
            false,
        )
    } else if let Some(frame) = state
        .low_disk_space_toast
        .update(world.low_disk_space_warning_count(), now)
    {
        (
            frame,
            localized_gui_text(item_runtime, "chunk.toast.lowDiskSpace", "Low disk space!"),
            localized_gui_text(
                item_runtime,
                "chunk.toast.lowDiskSpace.description",
                "Might not be able to save the world.",
            ),
            false,
        )
    } else if let Some(notification) = periodic_notification {
        let Some(frame) = state
            .periodic_notification_toast
            .update(notification.sequence, now)
        else {
            return (None, None);
        };
        let elapsed_periods = notification.elapsed_periods.to_string();
        (
            frame,
            localized_gui_text_with_args(
                item_runtime,
                &notification.title_key,
                &notification.title_key,
                &[&elapsed_periods],
            ),
            localized_gui_text_with_args(
                item_runtime,
                &notification.message_key,
                &notification.message_key,
                &[&elapsed_periods],
            ),
            false,
        )
    } else {
        return (None, None);
    };
    (
        Some(HudSystemToast {
            title,
            message,
            multiline,
            visible_portion: frame.visible_portion,
        }),
        frame.visibility_sound.map(|location| LocalSoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some(location.to_string()),
                fixed_range: None,
            },
            source: "master".to_string(),
            volume: 1.0,
            pitch: 1.0,
            seed: 0,
        }),
    )
}

fn localized_gui_text_with_args(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
    args: &[&str],
) -> String {
    format_localized_string_arguments(
        &localized_gui_text(item_runtime, translation_key, fallback),
        args,
    )
}

fn format_localized_string_arguments(template: &str, args: &[&str]) -> String {
    let bytes = template.as_bytes();
    let mut result = String::with_capacity(template.len());
    let mut offset = 0;
    let mut sequential = 0usize;
    while offset < bytes.len() {
        let Some(relative_percent) = bytes[offset..].iter().position(|byte| *byte == b'%') else {
            result.push_str(&template[offset..]);
            break;
        };
        let percent = offset + relative_percent;
        result.push_str(&template[offset..percent]);
        if bytes.get(percent + 1) == Some(&b'%') {
            result.push('%');
            offset = percent + 2;
            continue;
        }
        if bytes.get(percent + 1) == Some(&b's') {
            if let Some(argument) = args.get(sequential) {
                result.push_str(argument);
            } else {
                result.push_str("%s");
            }
            sequential += 1;
            offset = percent + 2;
            continue;
        }

        let mut cursor = percent + 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let positional = (cursor > percent + 1
            && bytes.get(cursor) == Some(&b'$')
            && bytes.get(cursor + 1) == Some(&b's'))
        .then(|| {
            template[percent + 1..cursor]
                .parse::<usize>()
                .ok()
                .and_then(|index| index.checked_sub(1))
        })
        .flatten();
        if let Some(index) = positional {
            if let Some(argument) = args.get(index) {
                result.push_str(argument);
            } else {
                result.push_str(&template[percent..cursor + 2]);
            }
            offset = cursor + 2;
        } else {
            result.push('%');
            offset = percent + 1;
        }
    }
    result
}

pub(crate) fn localized_command_suggestion_narration(
    item_runtime: Option<&NativeItemRuntime>,
    narration: &CommandSuggestionNarrationState,
) -> String {
    let position = narration.position.to_string();
    let total = narration.total.to_string();
    let tooltip = narration.tooltip.as_deref().map(|fallback| {
        narration
            .tooltip_translation_key
            .as_deref()
            .map(|key| localized_gui_text(item_runtime, key, fallback))
            .unwrap_or_else(|| fallback.to_string())
    });
    match tooltip.as_deref() {
        Some(tooltip) => localized_gui_text_with_args(
            item_runtime,
            "narration.suggestion.tooltip",
            "Selected suggestion %1$s out of %2$s: %3$s (%4$s)",
            &[&position, &total, &narration.text, tooltip],
        ),
        None => localized_gui_text_with_args(
            item_runtime,
            "narration.suggestion",
            "Selected suggestion %1$s out of %2$s: %3$s",
            &[&position, &total, &narration.text],
        ),
    }
}

pub(crate) fn narrate_command_suggestion_change(
    narration: Option<&CommandSuggestionNarrationState>,
    last_narration: &mut Option<CommandSuggestionNarrationState>,
    item_runtime: Option<&NativeItemRuntime>,
    narrator: Option<&mut dyn SystemNarrator>,
) {
    let current = narration.cloned();
    if current == *last_narration {
        return;
    }
    if let (Some(narrator), Some(narration)) = (narrator, current.as_ref()) {
        narrator.say_system_now(&localized_command_suggestion_narration(
            item_runtime,
            narration,
        ));
    }
    *last_narration = current;
}

pub(crate) fn narrate_player_list_visibility_change(
    visible: bool,
    was_visible: &mut bool,
    narrator: Option<&mut dyn SystemNarrator>,
    narration: impl FnOnce() -> String,
) {
    if visible && !*was_visible {
        if let Some(narrator) = narrator {
            narrator.say_system_now(&narration());
        }
    }
    *was_visible = visible;
}

fn stat_amount_by_resource_identity(
    world: &WorldStore,
    stat_type_resource_id: &str,
    value_resource_id: &str,
) -> i32 {
    world
        .client_stats()
        .identities
        .iter()
        .find(|identity| {
            identity.stat_type_resource_id.as_deref() == Some(stat_type_resource_id)
                && identity.value_resource_id.as_deref() == Some(value_resource_id)
        })
        .and_then(|identity| world.stat_value(identity.stat_type_id, identity.value_id))
        .unwrap_or_default()
}

const STATS_ITEM_COLUMN_TYPES: [&str; 6] = [
    "minecraft:mined",
    "minecraft:broken",
    "minecraft:crafted",
    "minecraft:used",
    "minecraft:picked_up",
    "minecraft:dropped",
];

struct StatsItemRowCandidate {
    raw_values: [Option<i32>; 6],
    row: HudStatsItemRow,
}

fn stats_item_rows(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudStatsItemRow> {
    let mut candidates = BTreeSet::new();
    for identity in &world.client_stats().identities {
        let Some(stat_type) = identity.stat_type_resource_id.as_deref() else {
            continue;
        };
        let Some(column) = STATS_ITEM_COLUMN_TYPES
            .iter()
            .position(|candidate| *candidate == stat_type)
        else {
            continue;
        };
        let Some(amount) = world.stat_value(identity.stat_type_id, identity.value_id) else {
            continue;
        };
        if amount <= 0 {
            continue;
        }
        let Some(value_resource) = identity.value_resource_id.as_deref() else {
            continue;
        };
        let item_resource = if column == 0 {
            item_runtime.and_then(|runtime| runtime.item_resource_id_for_block(value_resource))
        } else {
            Some(value_resource)
        };
        if let Some(item_resource) = item_resource.filter(|item| *item != "minecraft:air") {
            candidates.insert(item_resource.to_string());
        }
    }

    let mut rows = candidates
        .into_iter()
        .filter_map(|resource_id| {
            let item_id = item_runtime
                .and_then(|runtime| runtime.item_protocol_id(&resource_id))
                .or_else(|| {
                    bbb_protocol::item_types::vanilla_item_type_for_resource_id(&resource_id)
                        .map(|entry| entry.id)
                })?;
            let mined = item_runtime
                .and_then(|runtime| runtime.primary_block_resource_id_for_item(&resource_id))
                .map(|block| {
                    stat_amount_by_resource_identity(world, STATS_ITEM_COLUMN_TYPES[0], block)
                });
            let values = [
                mined,
                Some(stat_amount_by_resource_identity(
                    world,
                    STATS_ITEM_COLUMN_TYPES[1],
                    &resource_id,
                )),
                Some(stat_amount_by_resource_identity(
                    world,
                    STATS_ITEM_COLUMN_TYPES[2],
                    &resource_id,
                )),
                Some(stat_amount_by_resource_identity(
                    world,
                    STATS_ITEM_COLUMN_TYPES[3],
                    &resource_id,
                )),
                Some(stat_amount_by_resource_identity(
                    world,
                    STATS_ITEM_COLUMN_TYPES[4],
                    &resource_id,
                )),
                Some(stat_amount_by_resource_identity(
                    world,
                    STATS_ITEM_COLUMN_TYPES[5],
                    &resource_id,
                )),
            ];
            let stack = ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: Default::default(),
            };
            let icon = hud_item_icon_for_stack(
                world,
                item_runtime,
                &stack,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
                partial_tick,
            );
            let block_model = block_item_3d_model(
                world,
                &stack,
                item_runtime,
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
            );
            let label = item_runtime
                .map(|runtime| runtime.localized_item_name(&resource_id))
                .unwrap_or_else(|| resource_id.clone());
            Some(StatsItemRowCandidate {
                raw_values: values,
                row: HudStatsItemRow {
                    item_id,
                    resource_id,
                    label,
                    values: values.map(|value| value.map(format_grouped_integer)),
                    icon,
                    block_model,
                },
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|candidate| candidate.row.item_id);
    if let Some((column, order)) = input.stats_screen_item_effective_sort() {
        rows.sort_by(|left, right| {
            let left_value = left.raw_values[column].unwrap_or(-1);
            let right_value = right.raw_values[column].unwrap_or(-1);
            let ordering = left_value
                .cmp(&right_value)
                .then_with(|| left.row.item_id.cmp(&right.row.item_id));
            if order < 0 {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    rows.into_iter().map(|candidate| candidate.row).collect()
}

fn stats_screen_viewport_height(
    surface_size: winit::dpi::PhysicalSize<u32>,
    row_height: i32,
) -> i32 {
    const LIST_TOP: i32 = 46;
    const FOOTER_HEIGHT: i32 = 33;
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (height - FOOTER_HEIGHT - LIST_TOP).max(row_height)
}

fn stats_screen_scroller_height(
    viewport_height: i32,
    content_height: i32,
    max_scroll_amount: i32,
) -> i32 {
    if max_scroll_amount == 0 || content_height <= 0 {
        return 0;
    }
    viewport_height
        .saturating_mul(viewport_height)
        .checked_div(content_height)
        .unwrap_or(viewport_height)
        .clamp(32, (viewport_height - 8).max(32))
}

fn stats_screen_first_data_row(active_tab: usize, scroll_amount: i32, row_height: i32) -> usize {
    let entry =
        usize::try_from(scroll_amount.saturating_sub(2).max(0) / row_height).unwrap_or_default();
    if active_tab == 1 {
        entry.saturating_sub(1)
    } else {
        entry
    }
}

fn stats_screen_first_data_row_offset(
    active_tab: usize,
    scroll_row: usize,
    scroll_amount: i32,
    row_height: i32,
) -> i32 {
    let header_height = i32::from(active_tab == 1) * row_height;
    2i32.saturating_add(header_height)
        .saturating_add(
            i32::try_from(scroll_row)
                .unwrap_or(i32::MAX / row_height)
                .saturating_mul(row_height),
        )
        .saturating_sub(scroll_amount)
}

fn stats_screen_projected_row_count(
    remaining_rows: usize,
    viewport_height: i32,
    first_row_offset: i32,
    row_height: i32,
) -> usize {
    if remaining_rows == 0 || first_row_offset >= viewport_height {
        return 0;
    }
    let visible_height = viewport_height.saturating_sub(first_row_offset);
    let count = visible_height
        .saturating_add(row_height - 1)
        .checked_div(row_height)
        .unwrap_or_default();
    usize::try_from(count)
        .unwrap_or(remaining_rows)
        .min(remaining_rows)
}

fn format_vanilla_stat_value(formatter: &str, value: i32) -> String {
    match formatter {
        "DIVIDE_BY_TEN" => format!("{:.2}", f64::from(value) * 0.1),
        "DISTANCE" => {
            let meters = f64::from(value) / 100.0;
            let kilometers = meters / 1000.0;
            if kilometers > 0.5 {
                format!("{kilometers:.2} km")
            } else if meters > 0.5 {
                format!("{meters:.2} m")
            } else {
                format!("{value} cm")
            }
        }
        "TIME" => {
            let seconds = f64::from(value) / 20.0;
            let minutes = seconds / 60.0;
            let hours = minutes / 60.0;
            let days = hours / 24.0;
            let years = days / 365.0;
            if years > 0.5 {
                format!("{years:.2} y")
            } else if days > 0.5 {
                format!("{days:.2} d")
            } else if hours > 0.5 {
                format!("{hours:.2} h")
            } else if minutes > 0.5 {
                format!("{minutes:.2} min")
            } else {
                format!("{} s", java_double_string(seconds))
            }
        }
        _ => format_grouped_integer(value),
    }
}

fn java_double_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

fn format_grouped_integer(value: i32) -> String {
    let negative = value < 0;
    let digits = i64::from(value).abs().to_string();
    let mut grouped =
        String::with_capacity(digits.len() + digits.len() / 3 + usize::from(negative));
    if negative {
        grouped.push('-');
    }
    let first = digits.len() % 3;
    if first != 0 {
        grouped.push_str(&digits[..first]);
        if digits.len() > first {
            grouped.push(',');
        }
    }
    for (index, chunk) in digits.as_bytes()[first..].chunks(3).enumerate() {
        if index > 0 {
            grouped.push(',');
        }
        grouped.push_str(std::str::from_utf8(chunk).expect("decimal digits are valid UTF-8"));
    }
    grouped
}

fn hud_command_suggestions(
    input: &mut ClientInputState,
    world: &WorldStore,
    renderer: &Renderer,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudCommandSuggestions> {
    let state = input.command_suggestion_hud_state(world, renderer, surface_size)?;
    let tooltip = state.tooltip.map(|tooltip| HudCommandSuggestionTooltip {
        x: tooltip.x,
        y: tooltip.y,
        runs: hud_command_suggestion_tooltip_runs(tooltip, item_runtime),
    });
    Some(HudCommandSuggestions {
        input_text: state.input_text,
        draft: state.draft,
        cursor: state.cursor,
        selection: state.selection,
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
        suffix: state.suffix,
        list_x: state.rect.x,
        list_y: state.rect.y,
        list_width: state.rect.width,
        list_height: state.rect.height,
        entries: state
            .entries
            .into_iter()
            .map(|entry| HudCommandSuggestionEntry {
                text: entry.text,
                selected: entry.selected,
            })
            .collect(),
        has_previous: state.has_previous,
        has_next: state.has_next,
        tooltip,
        layout: if state.command_block_layout {
            HudCommandSuggestionsLayout::CommandBlock
        } else {
            HudCommandSuggestionsLayout::Chat
        },
    })
}

fn hud_chat_restrictions_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudChatRestrictionsScreen> {
    let generation = input.chat_restrictions_screen_generation()?;
    let visibility = world.chat_presentation_config().visibility;
    let (reason_key, reason_fallback) = match visibility {
        bbb_world::ChatVisibilityState::System => (
            "chat_restriction.chat_disabled_by_options",
            "Player chat is restricted in client settings.",
        ),
        bbb_world::ChatVisibilityState::Hidden => (
            "chat_restriction.chat_and_commands_disabled_by_options",
            "Chat is restricted in client settings.",
        ),
        bbb_world::ChatVisibilityState::Full => return None,
    };
    let permission =
        |id: &str, allowed: bool, allowed_fallback: &str, denied_fallback: &str| HudStyledTextRun {
            text: localized_gui_text(
                item_runtime,
                &format!(
                    "restrictions_screen.permission.{id}.{}",
                    if allowed { "allowed" } else { "denied" }
                ),
                if allowed {
                    allowed_fallback
                } else {
                    denied_fallback
                },
            ),
            style: Default::default(),
            color: Some(if allowed { 0x55FF55 } else { 0xFF5555 }),
        };
    let can_send_messages = visibility == bbb_world::ChatVisibilityState::Full;
    let can_send_commands = visibility != bbb_world::ChatVisibilityState::Hidden;
    Some(HudChatRestrictionsScreen {
        generation,
        title: localized_gui_text(item_runtime, "restrictions_screen.title", "Restrictions"),
        reason: HudStyledTextRun {
            text: localized_gui_text(item_runtime, reason_key, reason_fallback),
            style: bbb_renderer::HudTextStyle {
                bold: true,
                ..Default::default()
            },
            color: Some(0xFF5555),
        },
        permissions: vec![
            permission(
                "send_messages",
                can_send_messages,
                "You can send chat messages",
                "You can't send chat messages",
            ),
            permission(
                "send_commands",
                can_send_commands,
                "You can send commands",
                "You can't send commands",
            ),
            permission(
                "receive_system_messages",
                can_send_commands,
                "You can receive system messages from the server",
                "You can't receive system messages from the server",
            ),
            permission(
                "receive_player_messages",
                can_send_messages,
                "You can receive messages from players",
                "You can't receive messages from players",
            ),
        ],
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cursor_position,
    })
}

fn hud_command_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudCommandBlockEditorScreen> {
    let Some(state) = input.command_block_editor_hud_state() else {
        return hud_minecart_command_block_editor_screen(input, item_runtime, surface_size);
    };
    let layout = hud_command_block_editor_layout(surface_size);
    let mouse_position = state.mouse_position;
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        mouse_position.is_some_and(|(x, y)| rect.contains(x, y))
    };
    Some(HudCommandBlockEditorScreen {
        kind: HudCommandBlockEditorKind::Block,
        title: localized_gui_text(
            item_runtime,
            "advMode.setCommand",
            "Set Console Command for Block",
        ),
        command_label: localized_gui_text(item_runtime, "advMode.command", "Console Command"),
        previous_output_label: localized_gui_text(
            item_runtime,
            "advMode.previousOutput",
            "Previous Output",
        ),
        previous_output: if state.track_output {
            state.previous_output
        } else {
            "-".to_string()
        },
        mode: match state.mode {
            CommandBlockMode::Sequence => {
                localized_gui_text(item_runtime, "advMode.mode.sequence", "Chain")
            }
            CommandBlockMode::Auto => {
                localized_gui_text(item_runtime, "advMode.mode.auto", "Repeat")
            }
            CommandBlockMode::Redstone => {
                localized_gui_text(item_runtime, "advMode.mode.redstone", "Impulse")
            }
        },
        conditional: localized_gui_text(
            item_runtime,
            if state.conditional {
                "advMode.mode.conditional"
            } else {
                "advMode.mode.unconditional"
            },
            if state.conditional {
                "Conditional"
            } else {
                "Unconditional"
            },
        ),
        automatic: localized_gui_text(
            item_runtime,
            if state.automatic {
                "advMode.mode.autoexec.bat"
            } else {
                "advMode.mode.redstoneTriggered"
            },
            if state.automatic {
                "Always Active"
            } else {
                "Needs Redstone"
            },
        ),
        track_output: if state.track_output { "O" } else { "X" }.to_string(),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        controls_enabled: state.controls_enabled,
        mode_hovered: hovered(layout.mode),
        conditional_hovered: hovered(layout.conditional),
        automatic_hovered: hovered(layout.automatic),
        track_output_hovered: hovered(layout.track_output),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

fn hud_minecart_command_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudCommandBlockEditorScreen> {
    let state = input.minecart_command_block_editor_hud_state()?;
    let layout = hud_minecart_command_block_editor_layout(surface_size);
    let mouse_position = state.mouse_position;
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        mouse_position.is_some_and(|(x, y)| rect.contains(x, y))
    };
    Some(HudCommandBlockEditorScreen {
        kind: HudCommandBlockEditorKind::Minecart,
        title: localized_gui_text(
            item_runtime,
            "advMode.setCommand",
            "Set Console Command for Block",
        ),
        command_label: localized_gui_text(item_runtime, "advMode.command", "Console Command"),
        previous_output_label: localized_gui_text(
            item_runtime,
            "advMode.previousOutput",
            "Previous Output",
        ),
        previous_output: if state.track_output {
            state.previous_output
        } else {
            "-".to_string()
        },
        mode: String::new(),
        conditional: String::new(),
        automatic: String::new(),
        track_output: if state.track_output { "O" } else { "X" }.to_string(),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        controls_enabled: true,
        mode_hovered: false,
        conditional_hovered: false,
        automatic_hovered: false,
        track_output_hovered: hovered(layout.track_output),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

fn hud_structure_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudStructureBlockEditorScreen> {
    let state = input.structure_block_editor_hud_state()?;
    let layout = hud_structure_block_editor_layout(surface_size);
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let fields = std::array::from_fn(|index| {
        let field = &state.fields[index];
        HudStructureBlockEditorTextField {
            text: field.text.clone(),
            cursor: field.cursor,
            selection: field.selection,
            focused: state.focus_index == index,
            hovered: hovered(layout.fields[index]),
            max_length: field.max_utf16_units,
        }
    });
    let (mode, mode_key, mode_fallback) = match state.mode {
        StructureBlockMode::Save => (HudStructureBlockMode::Save, "save", "Save"),
        StructureBlockMode::Load => (HudStructureBlockMode::Load, "load", "Load"),
        StructureBlockMode::Corner => (HudStructureBlockMode::Corner, "corner", "Corner"),
        StructureBlockMode::Data => (HudStructureBlockMode::Data, "data", "Data"),
    };
    let (mirror_key, mirror_fallback) = match state.mirror {
        StructureBlockMirror::None => ("mirror.none", "|"),
        StructureBlockMirror::LeftRight => ("mirror.left_right", "< >"),
        StructureBlockMirror::FrontBack => ("mirror.front_back", "^ v"),
    };
    let rotation = match state.rotation {
        StructureBlockRotation::None => "0",
        StructureBlockRotation::Clockwise90 => "90",
        StructureBlockRotation::Clockwise180 => "180",
        StructureBlockRotation::Counterclockwise90 => "270",
    };
    let on_off = |value: bool| {
        localized_gui_text(
            item_runtime,
            if value { "options.on" } else { "options.off" },
            if value { "On" } else { "Off" },
        )
    };
    let (option_key, option_fallback) = if state.mode == StructureBlockMode::Load {
        ("structure_block.show_boundingbox", "Show Bounding Box")
    } else {
        ("structure_block.show_air", "Show Air")
    };

    Some(HudStructureBlockEditorScreen {
        title: localized_gui_text(
            item_runtime,
            "block.minecraft.structure_block",
            "Structure Block",
        ),
        name_label: localized_gui_text(
            item_runtime,
            "structure_block.structure_name",
            "Structure Name",
        ),
        position_label: localized_gui_text(
            item_runtime,
            "structure_block.position",
            "Relative Position",
        ),
        size_label: localized_gui_text(item_runtime, "structure_block.size", "Structure Size"),
        integrity_label: localized_gui_text(
            item_runtime,
            "structure_block.integrity",
            "Structure Integrity and Seed",
        ),
        data_label: localized_gui_text(
            item_runtime,
            "structure_block.custom_data",
            "Custom Data Tag Name",
        ),
        include_entities_label: localized_gui_text(
            item_runtime,
            "structure_block.include_entities",
            "Include Entities",
        ),
        strict_label: localized_gui_text(item_runtime, "structure_block.strict", "Strict"),
        detect_label: localized_gui_text(
            item_runtime,
            "structure_block.detect_size",
            "Detect Structure Size and Position",
        ),
        option_label: localized_gui_text(item_runtime, option_key, option_fallback),
        mode_description: localized_gui_text(
            item_runtime,
            &format!("structure_block.mode_info.{mode_key}"),
            mode_fallback,
        ),
        fields,
        mode,
        mode_value: localized_gui_text(
            item_runtime,
            &format!("structure_block.mode.{mode_key}"),
            mode_fallback,
        ),
        mirror_value: localized_gui_text(item_runtime, mirror_key, mirror_fallback),
        rotation: rotation.to_string(),
        include_entities: on_off(state.include_entities),
        strict: on_off(state.strict),
        option: on_off(if state.mode == StructureBlockMode::Load {
            state.show_bounding_box
        } else {
            state.show_air
        }),
        action: localized_gui_text(
            item_runtime,
            if state.mode == StructureBlockMode::Load {
                "structure_block.button.load"
            } else {
                "structure_block.button.save"
            },
            if state.mode == StructureBlockMode::Load {
                "Load"
            } else {
                "Save"
            },
        ),
        detect: localized_gui_text(item_runtime, "structure_block.button.detect_size", "Detect"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        mode_hovered: hovered(layout.mode),
        mirror_hovered: hovered(layout.mirror),
        rotation_hovered: layout.rotations.map(hovered),
        include_entities_hovered: hovered(layout.include_entities),
        strict_hovered: hovered(layout.strict),
        option_hovered: hovered(layout.option),
        action_hovered: hovered(layout.action),
        detect_hovered: hovered(layout.detect),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
    })
}

fn hud_jigsaw_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudJigsawBlockEditorScreen> {
    let state = input.jigsaw_block_editor_hud_state()?;
    let layout = hud_jigsaw_block_editor_layout(surface_size);
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let fields = std::array::from_fn(|index| {
        let field = &state.fields[index];
        HudJigsawBlockEditorTextField {
            text: field.text.clone(),
            cursor: field.cursor,
            selection: field.selection,
            focused: state.focus_index == index,
            hovered: hovered(layout.fields[index]),
            max_length: field.max_utf16_units,
        }
    });
    let joint = match state.joint {
        JigsawJoint::Rollable => {
            localized_gui_text(item_runtime, "jigsaw_block.joint.rollable", "Rollable")
        }
        JigsawJoint::Aligned => {
            localized_gui_text(item_runtime, "jigsaw_block.joint.aligned", "Aligned")
        }
    };
    let keep_label = localized_gui_text(item_runtime, "jigsaw_block.keep_jigsaws", "Keep Jigsaws");
    let keep_value = localized_gui_text(
        item_runtime,
        if state.keep_jigsaws {
            "options.on"
        } else {
            "options.off"
        },
        if state.keep_jigsaws { "ON" } else { "OFF" },
    );
    let tooltip = state.mouse_position.and_then(|(x, y)| {
        let (key, fallback) = jigsaw_priority_tooltip_key(layout, (x, y))?;
        Some(HudDebugOptionsTooltip {
            text: localized_gui_text(item_runtime, key, fallback),
            x,
            y,
        })
    });

    Some(HudJigsawBlockEditorScreen {
        labels: [
            localized_gui_text(item_runtime, "jigsaw_block.pool", "Target Pool:"),
            localized_gui_text(item_runtime, "jigsaw_block.name", "Name:"),
            localized_gui_text(item_runtime, "jigsaw_block.target", "Target Name:"),
            localized_gui_text(item_runtime, "jigsaw_block.final_state", "Turns into:"),
            localized_gui_text(
                item_runtime,
                "jigsaw_block.selection_priority",
                "Selection Priority:",
            ),
            localized_gui_text(
                item_runtime,
                "jigsaw_block.placement_priority",
                "Placement Priority:",
            ),
        ],
        joint_label: localized_gui_text(item_runtime, "jigsaw_block.joint_label", "Joint Type:"),
        fields,
        joint_visible: state.joint_editable,
        joint,
        levels: localized_gui_text_with_args(
            item_runtime,
            "jigsaw_block.levels",
            "Levels: %s",
            &[&state.levels.to_string()],
        ),
        slider_value_bits: state.slider_value_bits,
        keep_jigsaws: localized_gui_text_with_args(
            item_runtime,
            "options.generic_value",
            "%s: %s",
            &[&keep_label, &keep_value],
        ),
        generate: localized_gui_text(item_runtime, "jigsaw_block.generate", "Generate"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        controls_enabled: state.controls_enabled,
        joint_hovered: state.joint_editable && hovered(layout.joint),
        levels_hovered: state.dragging_levels || hovered(layout.levels),
        keep_jigsaws_hovered: hovered(layout.keep_jigsaws),
        generate_hovered: hovered(layout.generate),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
        tooltip,
    })
}

fn jigsaw_priority_tooltip_key(
    layout: bbb_renderer::HudJigsawBlockEditorLayout,
    mouse_position: (i32, i32),
) -> Option<(&'static str, &'static str)> {
    let (x, y) = mouse_position;
    if layout.fields[4].contains(x, y) {
        Some((
            "jigsaw_block.selection_priority.tooltip",
            "Controls the order in which this Jigsaw attempts to connect.",
        ))
    } else if layout.fields[5].contains(x, y) {
        Some((
            "jigsaw_block.placement_priority.tooltip",
            "Controls the order in which the connected piece is processed.",
        ))
    } else {
        None
    }
}

fn hud_test_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudTestBlockEditorScreen> {
    let state = input.test_block_editor_hud_state()?;
    let layout = hud_test_block_editor_layout(surface_size);
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let (mode_key, mode_fallback, info_key, info_fallback) = match state.mode {
        TestBlockMode::Start => (
            "test_block.mode.start",
            "Start",
            "test_block.mode_info.start",
            "Start Mode - The starting point for a test",
        ),
        TestBlockMode::Log => (
            "test_block.mode.log",
            "Log",
            "test_block.mode_info.log",
            "Log Mode - Log a message",
        ),
        TestBlockMode::Fail => (
            "test_block.mode.fail",
            "Fail",
            "test_block.mode_info.fail",
            "Fail Mode - Fail the test",
        ),
        TestBlockMode::Accept => (
            "test_block.mode.accept",
            "Accept",
            "test_block.mode_info.accept",
            "Accept Mode - Accept success for (part of) a test",
        ),
    };
    let message_visible = state.mode != TestBlockMode::Start;
    Some(HudTestBlockEditorScreen {
        title: localized_gui_text(item_runtime, "block.minecraft.test_block", "Test Block"),
        message_label: localized_gui_text(item_runtime, "test_block.message", "Message:"),
        message: HudTestBlockEditorTextField {
            text: state.message,
            cursor: state.cursor,
            selection: state.selection,
            focused: message_visible,
            hovered: message_visible && hovered(layout.message),
            max_length: bbb_protocol::packets::MAX_TEST_BLOCK_MESSAGE_UTF16_UNITS,
        },
        message_visible,
        mode: localized_gui_text(item_runtime, mode_key, mode_fallback),
        mode_info: localized_gui_text(item_runtime, info_key, info_fallback),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        mode_hovered: hovered(layout.mode),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
    })
}

fn hud_test_instance_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudTestInstanceBlockEditorScreen> {
    let state = input.test_instance_block_editor_hud_state()?;
    let layout = hud_test_instance_block_editor_layout(surface_size);
    let hovered = |rect: bbb_renderer::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let field = |index: usize, rect: bbb_renderer::HudCommandBlockEditorRect| {
        let state_field = &state.fields[index];
        HudTestInstanceBlockEditorTextField {
            text: state_field.text.clone(),
            cursor: state_field.cursor,
            selection: state_field.selection,
            focused: state.focus_index == index,
            hovered: hovered(rect),
            max_length: state_field.max_utf16_units,
        }
    };

    let mut info_runs = Vec::new();
    if state.local_invalid_id {
        info_runs.push(HudStyledTextRun {
            text: localized_gui_text(
                item_runtime,
                "test_instance.description.invalid_id",
                "Invalid test ID",
            ),
            style: HudTextStyle::default(),
            color: Some(0xFF5555),
        });
    } else if let Some(status) = &state.response_status {
        if let Some(error) = &state.error_message {
            let failed = localized_gui_text(
                item_runtime,
                "test_instance.description.failed",
                "Failed: %s",
            )
            .replace("%1$s", "")
            .replace("%s", "");
            info_runs.push(HudStyledTextRun {
                text: failed,
                style: HudTextStyle::default(),
                color: None,
            });
            info_runs.extend(
                hud_styled_runs_from_rich_text(error)
                    .into_iter()
                    .map(|mut run| {
                        if run.color.is_none() {
                            run.color = Some(0xFF5555);
                        }
                        run
                    }),
            );
            info_runs.push(HudStyledTextRun::plain("\n\n"));
        }
        info_runs.extend(hud_styled_runs_from_rich_text(status));
    }

    Some(HudTestInstanceBlockEditorScreen {
        title: localized_gui_text(
            item_runtime,
            "block.minecraft.test_instance_block",
            "Test Instance Block",
        ),
        id_label: localized_gui_text(
            item_runtime,
            "test_instance_block.test_id",
            "Test Instance ID",
        ),
        size_label: localized_gui_text(
            item_runtime,
            "test_instance_block.size",
            "Test Structure Size",
        ),
        rotation_label: localized_gui_text(
            item_runtime,
            "test_instance_block.rotation",
            "Rotation:",
        ),
        entities_label: localized_gui_text(
            item_runtime,
            "test_instance_block.entities",
            "Entities:",
        ),
        id: field(0, layout.id),
        size: [
            field(1, layout.size[0]),
            field(2, layout.size[1]),
            field(3, layout.size[2]),
        ],
        info_runs,
        rotation: match state.rotation {
            TestInstanceBlockRotation::None => "0",
            TestInstanceBlockRotation::Clockwise90 => "90",
            TestInstanceBlockRotation::Clockwise180 => "180",
            TestInstanceBlockRotation::Counterclockwise90 => "270",
        }
        .to_string(),
        include_entities: localized_gui_text(
            item_runtime,
            if state.include_entities {
                "options.on"
            } else {
                "options.off"
            },
            if state.include_entities { "On" } else { "Off" },
        ),
        reset: localized_gui_text(item_runtime, "test_instance.action.reset", "Reset and Load"),
        save: localized_gui_text(item_runtime, "test_instance.action.save", "Save Structure"),
        run: localized_gui_text(item_runtime, "test_instance.action.run", "Load and Run"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        save_enabled: state.save_enabled,
        rotation_hovered: hovered(layout.rotation),
        include_entities_hovered: hovered(layout.include_entities),
        reset_hovered: hovered(layout.reset),
        save_hovered: hovered(layout.save),
        run_hovered: hovered(layout.run),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
    })
}

fn hud_command_suggestion_tooltip_runs(
    tooltip: CommandSuggestionHudTooltip,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    if let Some(translation_key) = tooltip.translation_key.as_deref() {
        return vec![HudStyledTextRun {
            text: localized_gui_text(item_runtime, translation_key, &tooltip.text),
            style: HudTextStyle::default(),
            color: None,
        }];
    }
    tooltip
        .runs
        .into_iter()
        .map(|run| HudStyledTextRun {
            text: run.text,
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect()
}

fn hud_sign_editor_screen(
    input: &ClientInputState,
    world: &WorldStore,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudSignEditorScreen> {
    let state = input.sign_editor_hud_state(world)?;
    let kind = hud_sign_editor_kind(world, state.pos);
    let title = match kind {
        HudSignEditorKind::Hanging { .. } => "Edit Hanging Sign Message",
        HudSignEditorKind::Standing { .. } => "Edit Sign Message",
    }
    .to_string();
    Some(HudSignEditorScreen {
        kind,
        sign_preview: hud_sign_editor_preview(kind),
        title,
        lines: state.lines,
        line: state.line,
        cursor: state.cursor,
        selection: state.selection,
        cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
        text_tint: hud_sign_editor_text_tint(world, state.pos, state.is_front_text),
        done: "Done".to_string(),
        done_hovered: input.sign_editor_done_button_hovered(surface_size),
        preedit: state.preedit.map(|preedit| HudSignEditorPreedit {
            text: preedit.text,
            focused_start: preedit.focused_start,
            focused_end: preedit.focused_end,
            caret: preedit.caret,
            caret_visible: preedit.caret_visible,
        }),
    })
}

fn hud_sign_editor_kind(world: &WorldStore, pos: BlockPos) -> HudSignEditorKind {
    let Some((wood, form)) = world
        .probe_block(pos)
        .and_then(|probe| probe.block_name)
        .and_then(|name| sign_wood_and_form_for_block_name(&name))
    else {
        return HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        };
    };
    let wood = sign_model_wood(wood);
    match form {
        SignBlockForm::Standing => HudSignEditorKind::Standing {
            wood,
            attachment: sign_model_attachment(bbb_world::SignModelAttachment::Standing),
        },
        SignBlockForm::Wall => HudSignEditorKind::Standing {
            wood,
            attachment: sign_model_attachment(bbb_world::SignModelAttachment::Wall),
        },
        SignBlockForm::HangingCeiling | SignBlockForm::HangingWall => {
            HudSignEditorKind::Hanging { wood }
        }
    }
}

fn hud_sign_editor_preview(kind: HudSignEditorKind) -> Option<HudEntityPreview> {
    const ENTITY_ID: i32 = -1;
    const WIDTH: u32 = 96;
    const HEIGHT: u32 = 102;
    const SIGN_SCALE: f32 = 62.500_004;

    let HudSignEditorKind::Standing { wood, attachment } = kind else {
        return None;
    };
    let mut entity = EntityModelInstance::sign(ENTITY_ID, [0.0, 0.0, 0.0], 0.0, wood, attachment)
        .with_light_coords(ENTITY_FULL_BRIGHT_LIGHT_COORDS);
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;
    Some(HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        },
        scissor: None,
        translation: [0.0, HEIGHT as f32 / (2.0 * SIGN_SCALE) - 0.75, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: SIGN_SCALE,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    })
}

fn hud_sign_editor_text_tint(world: &WorldStore, pos: BlockPos, is_front_text: bool) -> [f32; 4] {
    let color = world
        .sign_text_state_at(pos)
        .map(|text| {
            let side = text.side(is_front_text);
            sign_text_base_color(side.color.text_color(), side.has_glowing_text)
        })
        .unwrap_or_default();
    let [red, green, blue] = rgb24(color as i32);
    [red, green, blue, 1.0]
}

/// Test helper: the inventory screen with default local state and no resident terrain atlas (so block
/// items resolve no 3D model — the tests assert the 2D icon / layout path).
#[cfg(test)]
fn hud_inventory_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        &TerrainTextureState::default(),
        hovered_slot_id,
        InventoryHudLocalState::default(),
        partial_tick,
    )
}

#[cfg(test)]
fn hud_inventory_screen_with_local_state(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_inventory_screen_with_local_state_for_surface(
        world,
        item_runtime,
        terrain_textures,
        hovered_slot_id,
        local_state,
        winit::dpi::PhysicalSize::new(1280, 720),
        partial_tick,
    )
}

fn advancement_hover_fade_for_hud(
    input: &mut ClientInputState,
    world: &WorldStore,
    scroll_delta: Option<(f64, f64)>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> f32 {
    if !world.advancements_screen_is_open() {
        input.reset_advancement_hover_fade();
        return 0.0;
    }

    let widgets = world.selected_advancement_widgets();
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(&widgets, scroll_delta) else {
        input.update_advancement_hover_fade(false);
        return input.advancement_hover_fade();
    };
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let hovering = advancement_hovered_widget(
        &widgets,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        input.inventory_cursor_position(),
    )
    .is_some();
    input.update_advancement_hover_fade(hovering);
    input.advancement_hover_fade()
}

fn world_ticking_milliseconds_per_tick(world: &WorldStore) -> f32 {
    let tick_rate = world.ticking().tick_rate;
    if tick_rate.is_finite() && tick_rate > 0.0 {
        1000.0 / tick_rate
    } else {
        50.0
    }
}

#[cfg(test)]
fn hud_debug_overlay(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    surface_size: winit::dpi::PhysicalSize<u32>,
    fps_sampler: &HudDebugFpsSampler,
    client_framerate_limit: u32,
    client_vsync: bool,
    network_sampler: &HudDebugNetworkSampler,
    tps_sampler: &HudDebugTpsSampler,
    net_counters: &NetCounters,
) -> Option<HudDebugOverlay> {
    let audio_counters = AudioCounters::default();
    let terrain_textures = TerrainTextureState::default();
    hud_debug_overlay_at_partial_tick(
        input,
        world,
        camera_pose,
        1.0,
        None,
        &terrain_textures,
        &bbb_renderer::RendererCounters::default(),
        None,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
        surface_size,
        fps_sampler,
        client_framerate_limit,
        client_vsync,
        network_sampler,
        tps_sampler,
        net_counters,
        &audio_counters,
        input.camera_post_effect_id(),
    )
}

#[cfg(test)]
fn hud_debug_overlay_at_partial_tick(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    entity_partial_tick: f32,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    renderer_counters: &bbb_renderer::RendererCounters,
    gpu_utilization: Option<f64>,
    render_distance_chunks: u32,
    surface_size: winit::dpi::PhysicalSize<u32>,
    fps_sampler: &HudDebugFpsSampler,
    client_framerate_limit: u32,
    client_vsync: bool,
    network_sampler: &HudDebugNetworkSampler,
    tps_sampler: &HudDebugTpsSampler,
    net_counters: &NetCounters,
    audio_counters: &AudioCounters,
    current_post_effect_id: Option<&str>,
) -> Option<HudDebugOverlay> {
    hud_debug_overlay_at_partial_tick_with_camera_poses(
        input,
        world,
        camera_pose,
        camera_pose,
        entity_partial_tick,
        item_runtime,
        terrain_textures,
        renderer_counters,
        gpu_utilization,
        render_distance_chunks,
        surface_size,
        fps_sampler,
        client_framerate_limit,
        client_vsync,
        network_sampler,
        tps_sampler,
        net_counters,
        audio_counters,
        current_post_effect_id,
    )
}

fn hud_debug_overlay_at_partial_tick_with_camera_poses(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    interaction_camera_pose: Option<CameraPose>,
    entity_partial_tick: f32,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    renderer_counters: &bbb_renderer::RendererCounters,
    gpu_utilization: Option<f64>,
    render_distance_chunks: u32,
    surface_size: winit::dpi::PhysicalSize<u32>,
    fps_sampler: &HudDebugFpsSampler,
    client_framerate_limit: u32,
    client_vsync: bool,
    network_sampler: &HudDebugNetworkSampler,
    tps_sampler: &HudDebugTpsSampler,
    net_counters: &NetCounters,
    audio_counters: &AudioCounters,
    current_post_effect_id: Option<&str>,
) -> Option<HudDebugOverlay> {
    let reduced_debug_info = world.local_player_has_reduced_debug_info();
    let entry_enabled = |entry| input.debug_screen_entry_enabled(entry, reduced_debug_info);

    let mut left_lines = Vec::new();
    if entry_enabled(DebugScreenEntryId::GameVersion) {
        left_lines.push(hud_debug_version_line());
    }
    if entry_enabled(DebugScreenEntryId::Fps) {
        left_lines.push(hud_debug_fps_line(
            fps_sampler.fps(),
            client_framerate_limit,
            client_vsync,
        ));
    }
    if entry_enabled(DebugScreenEntryId::Tps) {
        if let Some(tps_line) = hud_debug_tps_line(world) {
            left_lines.push(tps_line);
        }
    }
    if entry_enabled(DebugScreenEntryId::DayCount) {
        if let Some(day_count_line) = hud_debug_day_count_line(world) {
            left_lines.push(day_count_line);
        }
    }
    if entry_enabled(DebugScreenEntryId::LightLevels) {
        if let Some(camera_pose) = camera_pose {
            if let Some(light_line) = hud_debug_light_line(world, camera_pose) {
                left_lines.push(light_line);
            }
        }
    }
    if entry_enabled(DebugScreenEntryId::Heightmap) {
        if let Some(camera_pose) = camera_pose {
            if let Some(heightmap_lines) = hud_debug_heightmap_lines(world, camera_pose) {
                left_lines.extend(heightmap_lines);
            }
        }
    }
    if entry_enabled(DebugScreenEntryId::Biome) {
        if let Some(camera_pose) = camera_pose {
            if let Some(biome_line) = hud_debug_biome_line(world, camera_pose) {
                left_lines.push(biome_line);
            }
        }
    }
    if entry_enabled(DebugScreenEntryId::LocalDifficulty) {
        if let Some(local_difficulty_line) = hud_debug_local_difficulty_line(world, camera_pose) {
            left_lines.push(local_difficulty_line);
        }
    }
    if entry_enabled(DebugScreenEntryId::EntitySpawnCounts) {
        if let Some(entity_spawn_counts_line) =
            hud_debug_entity_spawn_counts_line(world, camera_pose)
        {
            left_lines.push(entity_spawn_counts_line);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtBlockState) {
        if let Some(looking_at_lines) =
            hud_debug_looking_at_block_state_lines(world, interaction_camera_pose)
        {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtBlockTags) {
        if let Some(looking_at_lines) =
            hud_debug_looking_at_block_tag_lines(world, interaction_camera_pose)
        {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtFluidState) {
        if let Some(looking_at_lines) =
            hud_debug_looking_at_fluid_state_lines(world, interaction_camera_pose)
        {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtFluidTags) {
        if let Some(looking_at_lines) =
            hud_debug_looking_at_fluid_tag_lines(world, interaction_camera_pose)
        {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtEntity) {
        if let Some(looking_at_lines) =
            hud_debug_looking_at_entity_lines(world, interaction_camera_pose, entity_partial_tick)
        {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::LookingAtEntityTags) {
        if let Some(looking_at_lines) = hud_debug_looking_at_entity_tag_lines(
            world,
            interaction_camera_pose,
            entity_partial_tick,
        ) {
            left_lines.extend(looking_at_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::ChunkRenderStats) {
        left_lines.push(hud_debug_chunk_render_stats_line(
            renderer_counters,
            render_distance_chunks,
            input.debug_smart_cull_enabled(),
        ));
    }
    if entry_enabled(DebugScreenEntryId::ChunkGenerationStats) {
        if let Some(chunk_generation_lines) =
            hud_debug_chunk_generation_stats_lines(world, camera_pose)
        {
            left_lines.extend(chunk_generation_lines);
        }
    }
    if entry_enabled(DebugScreenEntryId::EntityRenderStats) {
        left_lines.push(hud_debug_entity_render_stats_line(world));
    }
    if entry_enabled(DebugScreenEntryId::ParticleRenderStats) {
        left_lines.push(hud_debug_particle_render_stats_line(renderer_counters));
    }
    if entry_enabled(DebugScreenEntryId::ChunkSourceStats) {
        left_lines.push(hud_debug_chunk_source_stats_line(world));
    }
    if entry_enabled(DebugScreenEntryId::SoundCache) {
        left_lines.push(hud_debug_sound_cache_line(audio_counters));
    }
    if entry_enabled(DebugScreenEntryId::SoundMood) {
        left_lines.push(hud_debug_sound_mood_line(audio_counters));
    }
    if entry_enabled(DebugScreenEntryId::PostEffect) {
        if let Some(post_effect_line) = hud_debug_post_effect_line(current_post_effect_id) {
            left_lines.push(post_effect_line);
        }
    }
    let debug_crosshair = camera_pose
        .filter(|_| entry_enabled(DebugScreenEntryId::ThreeDimensionalCrosshair))
        .map(hud_debug_crosshair);
    if let Some(camera_pose) = camera_pose {
        let mut position_lines = Vec::new();
        if entry_enabled(DebugScreenEntryId::PlayerPosition) {
            position_lines.extend(hud_debug_position_lines(world, camera_pose));
        }
        if entry_enabled(DebugScreenEntryId::PlayerSectionPosition) {
            position_lines.push(hud_debug_section_position_line(camera_pose));
        }
        if !position_lines.is_empty() {
            if !left_lines.is_empty() {
                left_lines.push("".to_string());
            }
            left_lines.extend(position_lines);
        }
    }
    if input.debug_overlay_visible() {
        left_lines.push("".to_string());
        left_lines.push(format!(
            "Debug charts: [F3+1] Profiler {}; [F3+2] FPS {};",
            hud_debug_visibility_label(input.debug_profiler_chart_visible()),
            hud_debug_visibility_label(input.debug_fps_charts_visible())
        ));
        left_lines.push(format!(
            "[F3+3] Network {}; [F3+4] Lightmap {}",
            hud_debug_visibility_label(input.debug_network_charts_visible()),
            hud_debug_visibility_label(input.debug_lightmap_texture_visible())
        ));
        left_lines.push(format!(
            "Debug toggles: [F3+B] Hitboxes {}; [F3+G] Chunks {}",
            hud_debug_visibility_label(input.debug_entity_hitboxes_visible_for_world(world)),
            hud_debug_visibility_label(input.debug_chunk_borders_visible_for_world(world))
        ));
        left_lines.push(format!(
            "Debug options: [F3+H] Tooltips {}; [F3+P] Focus pause {}",
            hud_debug_enabled_label(input.debug_advanced_item_tooltips()),
            hud_debug_enabled_label(input.debug_pause_on_lost_focus())
        ));
        left_lines.push("Debug actions: [F3+A] Reload chunks; [F3+C] Copy location".to_string());
        left_lines.push("Debug actions: [F3+D] Clear chat; [F3+S] Dump textures".to_string());
        left_lines.push("Debug actions: [F3+T] Reload packs; [F3+V] Version".to_string());
        left_lines.push("Game mode: [F3+N] Spectator; [F3+F4] Switcher".to_string());
        left_lines.push("To edit: press [F3+F6]".to_string());
    }
    let mut right_lines = Vec::new();
    if entry_enabled(DebugScreenEntryId::Memory) {
        right_lines.extend(hud_debug_memory_lines(hud_debug_memory_snapshot()));
    }
    if entry_enabled(DebugScreenEntryId::DetailedMemory) {
        right_lines.extend(hud_debug_detailed_memory_lines(
            hud_debug_detailed_memory_snapshot(),
        ));
    }
    if entry_enabled(DebugScreenEntryId::SystemSpecs) {
        if !right_lines.is_empty() {
            right_lines.push("".to_string());
        }
        right_lines.extend(hud_debug_system_lines(surface_size));
    }
    if entry_enabled(DebugScreenEntryId::GpuUtilization) {
        if !right_lines.is_empty() {
            right_lines.push("".to_string());
        }
        right_lines.push(hud_debug_gpu_utilization_line(gpu_utilization));
    }
    if entry_enabled(DebugScreenEntryId::SimplePerformanceImpactors) {
        if !right_lines.is_empty() {
            right_lines.push("".to_string());
        }
        right_lines.extend(hud_debug_simple_performance_impactor_lines());
    }
    let game_mode_switcher = hud_debug_game_mode_switcher(
        input,
        world,
        item_runtime,
        terrain_textures,
        surface_size,
        entity_partial_tick,
    );
    if left_lines.is_empty()
        && right_lines.is_empty()
        && debug_crosshair.is_none()
        && game_mode_switcher.is_none()
    {
        return None;
    }

    Some(HudDebugOverlay {
        left_lines,
        right_lines,
        debug_crosshair,
        game_mode_switcher,
        profiler_chart: None,
        fps_chart: input
            .debug_fps_charts_visible()
            .then(|| HudDebugFrameTimeChart {
                frame_time_nanos: fps_sampler.frame_time_nanos(),
                configured_framerate_limit: hud_debug_configured_framerate_limit(
                    client_framerate_limit,
                ),
            }),
        tps_chart: input
            .debug_fps_charts_visible()
            .then(|| tps_sampler.chart(world))
            .flatten(),
        network_charts: (input.debug_network_charts_visible() && net_counters.connected).then(
            || HudDebugNetworkCharts {
                ping_millis: network_sampler.ping_millis(),
                bandwidth_bytes_per_tick: network_sampler.bandwidth_bytes_per_tick(),
                show_bandwidth: true,
            },
        ),
        show_lightmap_preview: input.debug_lightmap_texture_visible(),
    })
}

fn hud_debug_overlay_with_profiler_chart(
    overlay: Option<HudDebugOverlay>,
    profiler_chart: Option<HudDebugProfilerChart>,
) -> Option<HudDebugOverlay> {
    let Some(profiler_chart) = profiler_chart else {
        return overlay;
    };
    match overlay {
        Some(mut overlay) => {
            overlay.profiler_chart = Some(profiler_chart);
            Some(overlay)
        }
        None => Some(HudDebugOverlay {
            profiler_chart: Some(profiler_chart),
            ..HudDebugOverlay::default()
        }),
    }
}

const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_WIDTH: i32 = 125;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_HEIGHT: i32 = 75;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_X_OFFSET: i32 = 62;
const DEBUG_GAME_MODE_SWITCHER_BACKGROUND_Y_OFFSET: i32 = 58;
const DEBUG_GAME_MODE_SWITCHER_SLOT_AREA: i32 = 26;
const DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED: i32 = 31;
const DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH: i32 = 4 * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED - 5;
const DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET: i32 = 31;

fn hud_debug_game_mode_switcher(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudDebugGameModeSwitcher> {
    let selected = input.debug_game_mode_switcher_selected()?;
    let center_x = i32::try_from(surface_size.width / 2).unwrap_or(i32::MAX);
    let center_y = i32::try_from(surface_size.height / 2).unwrap_or(i32::MAX);
    let selected_mode = hud_game_mode_switcher_mode(selected);
    let slot_start_x = center_x - DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH / 2;
    let slot_y = center_y - DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET;
    let shift_down = input.shift_down();
    let keybind_context = input.item_model_keybind_context();
    let slots = DEBUG_GAME_MODE_SWITCHER_MODES
        .iter()
        .enumerate()
        .map(|(index, mode)| {
            let stack = hud_game_mode_switcher_icon_stack(*mode, item_runtime);
            let icon = stack.as_ref().and_then(|stack| {
                hud_game_mode_switcher_icon_for_stack(
                    world,
                    item_runtime,
                    stack,
                    shift_down,
                    keybind_context,
                    i32::try_from(index + 1).unwrap_or(0),
                    partial_tick,
                )
            });
            let block_model = stack.and_then(|stack| {
                block_item_3d_model(
                    world,
                    &stack,
                    item_runtime,
                    terrain_textures,
                    None,
                    false,
                    false,
                    false,
                    false,
                    shift_down,
                    keybind_context,
                    i32::try_from(index + 1).unwrap_or(0),
                )
            });
            HudDebugGameModeSwitcherSlot {
                mode: *mode,
                x: slot_start_x
                    + i32::try_from(index).unwrap_or(0) * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED,
                y: slot_y,
                width: DEBUG_GAME_MODE_SWITCHER_SLOT_AREA,
                height: DEBUG_GAME_MODE_SWITCHER_SLOT_AREA,
                selected: *mode == selected_mode,
                icon,
                block_model,
            }
        })
        .collect();
    Some(HudDebugGameModeSwitcher {
        selected: selected_mode,
        title: hud_game_mode_switcher_title(selected_mode).to_string(),
        help_text: "Select next: F4".to_string(),
        background_x: center_x - DEBUG_GAME_MODE_SWITCHER_BACKGROUND_X_OFFSET,
        background_y: center_y - DEBUG_GAME_MODE_SWITCHER_BACKGROUND_Y_OFFSET,
        background_width: DEBUG_GAME_MODE_SWITCHER_BACKGROUND_WIDTH,
        background_height: DEBUG_GAME_MODE_SWITCHER_BACKGROUND_HEIGHT,
        slots,
    })
}

fn hud_game_mode_switcher_icon_stack(
    mode: HudGameModeSwitcherMode,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ItemStackSummary> {
    let item_id = item_runtime?.item_protocol_id(hud_game_mode_switcher_icon_resource_id(mode))?;
    Some(ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    })
}

fn hud_game_mode_switcher_icon_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    stack: &ItemStackSummary,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    partial_tick: f32,
) -> Option<HudItemIcon> {
    let mut icon = hud_item_icon_for_stack(
        world,
        item_runtime,
        stack,
        None,
        false,
        false,
        false,
        false,
        shift_down,
        keybind_context,
        item_model_seed,
        partial_tick,
    )?;
    icon.count_label = None;
    icon.durability_bar = None;
    icon.cooldown_progress = None;
    Some(icon)
}

const DEBUG_GAME_MODE_SWITCHER_MODES: [HudGameModeSwitcherMode; 4] = [
    HudGameModeSwitcherMode::Creative,
    HudGameModeSwitcherMode::Survival,
    HudGameModeSwitcherMode::Adventure,
    HudGameModeSwitcherMode::Spectator,
];

fn hud_game_mode_switcher_mode(game_type: GameType) -> HudGameModeSwitcherMode {
    match game_type {
        GameType::Creative => HudGameModeSwitcherMode::Creative,
        GameType::Survival => HudGameModeSwitcherMode::Survival,
        GameType::Adventure => HudGameModeSwitcherMode::Adventure,
        GameType::Spectator => HudGameModeSwitcherMode::Spectator,
    }
}

fn hud_game_mode_switcher_title(mode: HudGameModeSwitcherMode) -> &'static str {
    match mode {
        HudGameModeSwitcherMode::Creative => "Creative Mode",
        HudGameModeSwitcherMode::Survival => "Survival Mode",
        HudGameModeSwitcherMode::Adventure => "Adventure Mode",
        HudGameModeSwitcherMode::Spectator => "Spectator Mode",
    }
}

fn hud_game_mode_switcher_icon_resource_id(mode: HudGameModeSwitcherMode) -> &'static str {
    match mode {
        HudGameModeSwitcherMode::Creative => "minecraft:grass_block",
        HudGameModeSwitcherMode::Survival => "minecraft:iron_sword",
        HudGameModeSwitcherMode::Adventure => "minecraft:map",
        HudGameModeSwitcherMode::Spectator => "minecraft:ender_eye",
    }
}

fn hud_debug_crosshair(camera_pose: CameraPose) -> HudDebugCrosshair {
    HudDebugCrosshair {
        x_rot_degrees: camera_pose.x_rot,
        y_rot_degrees: camera_pose.y_rot,
        gui_scale: 1,
    }
}

fn hud_crosshair_attack_indicator(
    visible: bool,
    state: Option<LocalPlayerCrosshairAttackIndicatorState>,
) -> HudCrosshairAttackIndicator {
    if !visible {
        return HudCrosshairAttackIndicator::None;
    }
    match state {
        Some(state) if state.full => HudCrosshairAttackIndicator::Full,
        Some(state) if state.strength_scale < 1.0 => HudCrosshairAttackIndicator::Progress {
            strength_scale: state.strength_scale,
        },
        _ => HudCrosshairAttackIndicator::None,
    }
}

fn debug_entity_scene_outline(
    input: &ClientInputState,
    world: &WorldStore,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    if !input.debug_entity_hitboxes_visible_for_world(world) {
        return None;
    }
    if input.debug_show_local_server_entity_hit_boxes() {
        entity_scene_outline_from_world_at_partial_tick_with_server_details(
            world,
            entity_partial_tick,
            true,
        )
    } else {
        entity_scene_outline_from_world_at_partial_tick(world, entity_partial_tick)
    }
}

fn debug_chunk_border_outline(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<SelectionOutline> {
    if !input.debug_chunk_borders_visible_for_world(world) {
        return None;
    }
    world.level_info()?;
    let camera_pose = camera_pose?;
    let dimension = world.dimension();
    let min_y = dimension.min_y;
    let max_y = min_y + dimension.height;
    if max_y <= min_y {
        return None;
    }
    let x_start = chunk_min_block(camera_pose.position[0]);
    let z_start = chunk_min_block(camera_pose.position[2]);
    let y_start = chunk_min_block(camera_pose.position[1]);
    let mut lines = Vec::new();

    for x in (-16..=32).step_by(16) {
        for z in (-16..=32).step_by(16) {
            push_chunk_border_line(
                &mut lines,
                [x_start + x, min_y, z_start + z],
                [x_start + x, max_y, z_start + z],
                CHUNK_BORDER_NEIGHBOR_COLOR,
                CHUNK_BORDER_THICK_LINE_WIDTH,
            );
        }
    }

    for x in (2..16).step_by(2) {
        let color = if x % 4 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_line(
            &mut lines,
            [x_start + x, min_y, z_start],
            [x_start + x, max_y, z_start],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
        push_chunk_border_line(
            &mut lines,
            [x_start + x, min_y, z_start + 16],
            [x_start + x, max_y, z_start + 16],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for z in (2..16).step_by(2) {
        let color = if z % 4 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_line(
            &mut lines,
            [x_start, min_y, z_start + z],
            [x_start, max_y, z_start + z],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
        push_chunk_border_line(
            &mut lines,
            [x_start + 16, min_y, z_start + z],
            [x_start + 16, max_y, z_start + z],
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for y in (min_y..=max_y).step_by(2) {
        let color = if y % 8 == 0 {
            CHUNK_BORDER_CELL_COLOR
        } else {
            CHUNK_BORDER_YELLOW_COLOR
        };
        push_chunk_border_horizontal_ring(
            &mut lines,
            x_start,
            z_start,
            y,
            color,
            CHUNK_BORDER_THIN_LINE_WIDTH,
        );
    }

    for x in (0..=16).step_by(16) {
        for z in (0..=16).step_by(16) {
            push_chunk_border_line(
                &mut lines,
                [x_start + x, min_y, z_start + z],
                [x_start + x, max_y, z_start + z],
                CHUNK_BORDER_MAJOR_COLOR,
                CHUNK_BORDER_THICK_LINE_WIDTH,
            );
        }
    }

    for y in (min_y..=max_y).step_by(16) {
        push_chunk_border_horizontal_ring(
            &mut lines,
            x_start,
            z_start,
            y,
            CHUNK_BORDER_MAJOR_COLOR,
            CHUNK_BORDER_THICK_LINE_WIDTH,
        );
    }

    Some(SelectionOutline {
        boxes: Vec::new(),
        colored_boxes: vec![SelectionColoredBox {
            min: [x_start as f32, y_start as f32, z_start as f32],
            max: [
                (x_start + 16) as f32,
                (y_start + 16) as f32,
                (z_start + 16) as f32,
            ],
            color: CHUNK_BORDER_MAJOR_COLOR,
            line_width: CHUNK_BORDER_THIN_LINE_WIDTH,
            always_on_top: true,
        }],
        lines,
        points: Vec::new(),
        text_labels: Vec::new(),
    })
}

fn debug_heightmap_boxes(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugFilledBox> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeHeightmap,
        world.local_player_has_reduced_debug_info(),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    let Some(camera_pose) = camera_pose else {
        return Vec::new();
    };
    if !camera_pose.position[0].is_finite() || !camera_pose.position[2].is_finite() {
        return Vec::new();
    }

    let center = ChunkPos {
        x: (camera_pose.position[0].floor() as i32).div_euclid(16),
        z: (camera_pose.position[2].floor() as i32).div_euclid(16),
    };
    let mut boxes = Vec::new();
    for chunk_offset_x in -DEBUG_HEIGHTMAP_CHUNK_DISTANCE..=DEBUG_HEIGHTMAP_CHUNK_DISTANCE {
        for chunk_offset_z in -DEBUG_HEIGHTMAP_CHUNK_DISTANCE..=DEBUG_HEIGHTMAP_CHUNK_DISTANCE {
            let chunk_pos = ChunkPos {
                x: center.x.saturating_add(chunk_offset_x),
                z: center.z.saturating_add(chunk_offset_z),
            };
            let Some(sampler) = world.chunk_heightmap_sampler(chunk_pos) else {
                continue;
            };
            let chunk_min_x = chunk_pos.x.saturating_mul(16);
            let chunk_min_z = chunk_pos.z.saturating_mul(16);
            for kind_id in sampler.effective_kind_ids() {
                let Some(color) = vanilla_heightmap_type(kind_id)
                    .map(|heightmap_type| heightmap_type.debug_color_rgba)
                else {
                    continue;
                };
                for local_x in 0_u8..16 {
                    for local_z in 0_u8..16 {
                        let Some(first_available) =
                            sampler.first_available(kind_id, local_x, local_z)
                        else {
                            continue;
                        };
                        let x = chunk_min_x.saturating_add(i32::from(local_x)) as f32;
                        let z = chunk_min_z.saturating_add(i32::from(local_z)) as f32;
                        let y =
                            first_available as f32 + kind_id as f32 * DEBUG_HEIGHTMAP_BOX_HEIGHT;
                        boxes.push(DebugFilledBox {
                            min: [x + 0.25, y, z + 0.25],
                            max: [x + 0.75, y + DEBUG_HEIGHTMAP_BOX_HEIGHT, z + 0.75],
                            color,
                        });
                    }
                }
            }
        }
    }
    boxes
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DebugWaterSample {
    pos: BlockPos,
    amount: u8,
    height: f64,
}

fn debug_water_samples(input: &ClientInputState, world: &WorldStore) -> Vec<DebugWaterSample> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeWaterLevels,
        world.local_player_has_reduced_debug_info(),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    // Vanilla WaterDebugRenderer anchors the 21^3 scan on
    // `minecraft.player.blockPosition()`, not the active camera entity.
    let Some(player_pose) = world.local_player_pose() else {
        return Vec::new();
    };
    let player_position = [
        player_pose.position.x,
        player_pose.position.y,
        player_pose.position.z,
    ];
    if !player_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return Vec::new();
    }
    let center = BlockPos {
        x: player_position[0].floor() as i32,
        y: player_position[1].floor() as i32,
        z: player_position[2].floor() as i32,
    };
    let min = BlockPos {
        x: center.x.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
        y: center.y.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
        z: center.z.saturating_sub(DEBUG_WATER_SCAN_RADIUS),
    };
    let max = BlockPos {
        x: center.x.saturating_add(DEBUG_WATER_SCAN_RADIUS),
        y: center.y.saturating_add(DEBUG_WATER_SCAN_RADIUS),
        z: center.z.saturating_add(DEBUG_WATER_SCAN_RADIUS),
    };

    let mut samples = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let pos = BlockPos { x, y, z };
                let Some((fluid, height)) = world.fluid_state_and_height_at(pos) else {
                    continue;
                };
                if fluid.kind != TerrainFluidKind::Water {
                    continue;
                }
                samples.push(DebugWaterSample {
                    pos,
                    amount: fluid.amount,
                    height,
                });
            }
        }
    }
    samples
}

fn debug_water_boxes(samples: &[DebugWaterSample]) -> Vec<DebugFilledBox> {
    samples
        .iter()
        .map(|sample| {
            let BlockPos { x, y, z } = sample.pos;
            DebugFilledBox {
                min: [
                    x as f32 + DEBUG_WATER_BOX_INSET,
                    y as f32 + DEBUG_WATER_BOX_INSET,
                    z as f32 + DEBUG_WATER_BOX_INSET,
                ],
                max: [
                    x as f32 + (1.0 - DEBUG_WATER_BOX_INSET),
                    y as f32 + sample.height as f32,
                    z as f32 + (1.0 - DEBUG_WATER_BOX_INSET),
                ],
                color: DEBUG_WATER_BOX_COLOR,
            }
        })
        .collect()
}

fn debug_water_amount_billboards(samples: &[DebugWaterSample]) -> Vec<DebugTextBillboard> {
    samples
        .iter()
        .map(|sample| {
            let BlockPos { x, y, z } = sample.pos;
            DebugTextBillboard::centered(
                [
                    x as f32 + 0.5,
                    y as f32 + sample.height as f32,
                    z as f32 + 0.5,
                ],
                sample.amount.to_string(),
                [0, 0, 0, 255],
            )
        })
        .collect()
}

const DEBUG_LIGHT_SCAN_RADIUS: i32 = 10;
const DEBUG_LIGHT_SECTION_LABEL_SCALE: f32 = 4.8;
const DEBUG_LIGHT_SECTION_LABEL_COLOR: [u8; 4] = [255, 0, 0, 255];
const DEBUG_LIGHT_SKY_FROM_COLOR: i32 = argb_color(255, 0, 0, 255);
const DEBUG_LIGHT_SKY_TO_COLOR: i32 = argb_color(255, 0, 255, 255);
const DEBUG_LIGHT_BLOCK_FROM_COLOR: i32 = argb_color(255, 170, 0, 0);
const DEBUG_LIGHT_BLOCK_TO_COLOR: i32 = argb_color(255, 255, 255, 0);

fn debug_light_level_billboards(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugTextBillboard> {
    let reduced_debug_info = world.local_player_has_reduced_debug_info();
    let show_block_light = input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeBlockLightLevels,
        reduced_debug_info,
    );
    let show_sky_light = input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeSkyLightLevels,
        reduced_debug_info,
    );
    if (!show_block_light && !show_sky_light) || world.level_info().is_none() {
        return Vec::new();
    }
    let Some(camera_eye) = camera_pose.map(camera_eye_position) else {
        return Vec::new();
    };
    if !camera_eye.iter().all(|coordinate| coordinate.is_finite()) {
        return Vec::new();
    }
    let center = BlockPos {
        x: camera_eye[0].floor() as i32,
        y: camera_eye[1].floor() as i32,
        z: camera_eye[2].floor() as i32,
    };
    let has_sky_light = world
        .level_info()
        .is_some_and(bbb_world::WorldLevelInfo::has_sky_light);
    let mut seen_sections = Vec::new();
    let mut billboards = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in center.z.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
        ..=center.z.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
    {
        for y in center.y.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
            ..=center.y.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
        {
            for x in center.x.saturating_sub(DEBUG_LIGHT_SCAN_RADIUS)
                ..=center.x.saturating_add(DEBUG_LIGHT_SCAN_RADIUS)
            {
                let pos = BlockPos { x, y, z };
                let section = [x.div_euclid(16), y.div_euclid(16), z.div_euclid(16)];
                if !seen_sections.contains(&section) {
                    seen_sections.push(section);
                    let mut billboard = DebugTextBillboard::centered(
                        section.map(|coordinate| {
                            coordinate.saturating_mul(16).saturating_add(8) as f32
                        }),
                        world.debug_sky_light_section_status(section),
                        DEBUG_LIGHT_SECTION_LABEL_COLOR,
                    );
                    billboard.scale = DEBUG_LIGHT_SECTION_LABEL_SCALE;
                    billboards.push(billboard);
                }

                let light = world
                    .sample_debug_light(pos)
                    .unwrap_or(TerrainLight::FULL_BRIGHT);
                let sky_brightness = if has_sky_light { light.sky } else { 0 };
                if show_sky_light && sky_brightness != 15 {
                    let color = argb_srgb_lerp(
                        f32::from(sky_brightness) / 15.0,
                        DEBUG_LIGHT_SKY_FROM_COLOR,
                        DEBUG_LIGHT_SKY_TO_COLOR,
                    );
                    billboards.push(DebugTextBillboard::centered(
                        [x as f32 + 0.5, y as f32 + 0.25, z as f32 + 0.5],
                        sky_brightness.to_string(),
                        argb_to_rgba8(color),
                    ));
                }
                if show_block_light && light.block != 0 {
                    let color = argb_srgb_lerp(
                        f32::from(light.block) / 15.0,
                        DEBUG_LIGHT_BLOCK_FROM_COLOR,
                        DEBUG_LIGHT_BLOCK_TO_COLOR,
                    );
                    billboards.push(DebugTextBillboard::centered(
                        [x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5],
                        light.block.to_string(),
                        argb_to_rgba8(color),
                    ));
                }
            }
        }
    }
    billboards
}

fn debug_selection_text_billboards(outline: Option<&SelectionOutline>) -> Vec<DebugTextBillboard> {
    outline
        .into_iter()
        .flat_map(|outline| &outline.text_labels)
        .map(|label| {
            let mut billboard = DebugTextBillboard::centered(
                label.position,
                label.text.clone(),
                argb_to_rgba8(rgba01_to_argb(label.color)),
            );
            if !label.centered {
                billboard.adjust_left = Some(0.0);
            }
            billboard
        })
        .collect()
}

const DEBUG_SOLID_FACE_SCAN_RADIUS: i32 = 6;
const DEBUG_SOLID_FACE_INFLATE: f32 = 0.002;
const DEBUG_SOLID_FACE_COLOR: [u8; 4] = [255, 0, 0, 128];
const DEBUG_SOLID_FACE_DIRECTIONS: [DebugCuboidFace; 6] = [
    DebugCuboidFace::West,
    DebugCuboidFace::South,
    DebugCuboidFace::East,
    DebugCuboidFace::North,
    DebugCuboidFace::Down,
    DebugCuboidFace::Up,
];

fn debug_solid_face_rects(
    input: &ClientInputState,
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Vec<DebugFilledRect> {
    if !input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeSolidFaces,
        world.local_player_has_reduced_debug_info(),
    ) || world.level_info().is_none()
    {
        return Vec::new();
    }
    let Some(camera_position) = camera_pose.map(camera_eye_position) else {
        return Vec::new();
    };
    if !camera_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return Vec::new();
    }
    let center = BlockPos {
        x: camera_position[0].floor() as i32,
        y: camera_position[1].floor() as i32,
        z: camera_position[2].floor() as i32,
    };
    let mut rects = Vec::new();
    // `BlockPos.betweenClosed`: x advances fastest, then y, then z.
    for z in center.z.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
        ..=center.z.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
    {
        for y in center.y.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
            ..=center.y.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
        {
            for x in center.x.saturating_sub(DEBUG_SOLID_FACE_SCAN_RADIUS)
                ..=center.x.saturating_add(DEBUG_SOLID_FACE_SCAN_RADIUS)
            {
                let pos = BlockPos { x, y, z };
                let Some(probe) = world.probe_block(pos) else {
                    continue;
                };
                // Vanilla skips only `Blocks.AIR`; the other air variants
                // naturally contribute no outline AABBs.
                if probe.block_name.as_deref() == Some("minecraft:air") {
                    continue;
                }
                let sturdy_faces = world.debug_block_sturdy_faces(&probe);
                if !sturdy_faces.contains(&true) {
                    continue;
                }
                let outline_boxes = if probe.block_name.as_deref() == Some("minecraft:scaffolding")
                {
                    world.debug_scaffolding_outline_shape_boxes(&probe)
                } else {
                    crate::block_outline::block_probe_shape_boxes(&probe)
                };
                let Some(outline_boxes) = outline_boxes else {
                    continue;
                };
                for (outline_min, outline_max) in outline_boxes {
                    let min = [
                        x as f32 + outline_min[0] as f32 - DEBUG_SOLID_FACE_INFLATE,
                        y as f32 + outline_min[1] as f32 - DEBUG_SOLID_FACE_INFLATE,
                        z as f32 + outline_min[2] as f32 - DEBUG_SOLID_FACE_INFLATE,
                    ];
                    let max = [
                        x as f32 + outline_max[0] as f32 + DEBUG_SOLID_FACE_INFLATE,
                        y as f32 + outline_max[1] as f32 + DEBUG_SOLID_FACE_INFLATE,
                        z as f32 + outline_max[2] as f32 + DEBUG_SOLID_FACE_INFLATE,
                    ];
                    for (face, sturdy) in DEBUG_SOLID_FACE_DIRECTIONS.into_iter().zip(sturdy_faces)
                    {
                        if sturdy {
                            rects.push(DebugFilledRect::cuboid_face(
                                min,
                                max,
                                face,
                                DEBUG_SOLID_FACE_COLOR,
                            ));
                        }
                    }
                }
            }
        }
    }
    rects
}

const DEBUG_COLLISION_QUERY_RADIUS: f64 = 6.0;
const DEBUG_COLLISION_COLOR: [u8; 4] = [255; 4];
const DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA: u8 = 102;
const DEBUG_SUPPORTING_BLOCK_PLAYER_COLOR: [u8; 4] = [255, 0, 0, 255];
const DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR: [u8; 4] = [0, 255, 0, 255];
const DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR: [u8; 4] = [0, 255, 255, 255];
const DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET: f64 = 0.02;
const DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET: f64 = 0.04;

fn debug_collision_entry_enabled(input: &ClientInputState, world: &WorldStore) -> bool {
    input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeCollisionBoxes,
        world.local_player_has_reduced_debug_info(),
    )
}

fn collect_debug_collision_lines(world: &WorldStore) -> Vec<DebugLine> {
    world
        .debug_camera_collision_boxes(DEBUG_COLLISION_QUERY_RADIUS)
        .into_iter()
        .flat_map(|collision_box| {
            DebugLine::cuboid(
                collision_box.min.map(|coordinate| coordinate as f32),
                collision_box.max.map(|coordinate| coordinate as f32),
                DEBUG_COLLISION_COLOR,
                DebugLine::DEFAULT_WIDTH,
            )
        })
        .collect()
}

fn debug_supporting_block_entry_enabled(input: &ClientInputState, world: &WorldStore) -> bool {
    input.debug_screen_entry_enabled(
        DebugScreenEntryId::VisualizeEntitySupportingBlocks,
        world.local_player_has_reduced_debug_info(),
    )
}

fn collect_debug_supporting_block_lines(
    world: &WorldStore,
    cached_entity_ids: &[i32],
) -> Vec<DebugLine> {
    let mut lines = Vec::new();
    if let Some(debug) = world.debug_local_player_supporting_blocks() {
        append_debug_supporting_block_highlight(
            &mut lines,
            &debug.supporting,
            DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET,
            DEBUG_SUPPORTING_BLOCK_PLAYER_COLOR,
        );
        if let Some(legacy_effect) = debug.legacy_effect.as_ref() {
            append_debug_supporting_block_highlight(
                &mut lines,
                legacy_effect,
                DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET,
                DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR,
            );
        }
    }
    for entity_id in cached_entity_ids {
        let Some(debug) = world.debug_entity_supporting_blocks(*entity_id) else {
            continue;
        };
        let bias = debug_supporting_block_entity_bias(*entity_id);
        append_debug_supporting_block_highlight(
            &mut lines,
            &debug.supporting,
            DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET + bias,
            DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR,
        );
        if let Some(legacy_effect) = debug.legacy_effect.as_ref() {
            append_debug_supporting_block_highlight(
                &mut lines,
                legacy_effect,
                DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET + bias,
                DEBUG_SUPPORTING_BLOCK_LEGACY_COLOR,
            );
        }
    }
    lines
}

fn debug_supporting_block_entity_bias(entity_id: i32) -> f64 {
    let hash_input = (f64::from(entity_id) + 0.132_453_657).to_string();
    0.02 * f64::from(java_string_hash_code(&hash_input) % 1000) / 1000.0
}

fn append_debug_supporting_block_highlight(
    lines: &mut Vec<DebugLine>,
    highlight: &bbb_world::DebugSupportingBlockHighlight,
    offset: f64,
    color: [u8; 4],
) {
    let BlockPos { x, y, z } = highlight.pos;
    let outline_offset = 2.0 * offset;
    let outline_color = [
        color[0],
        color[1],
        color[2],
        DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA,
    ];
    lines.extend(DebugLine::cuboid(
        [
            (f64::from(x) - outline_offset) as f32,
            (f64::from(y) - outline_offset) as f32,
            (f64::from(z) - outline_offset) as f32,
        ],
        [
            (f64::from(x) + 1.0 + outline_offset) as f32,
            (f64::from(y) + 1.0 + outline_offset) as f32,
            (f64::from(z) + 1.0 + outline_offset) as f32,
        ],
        outline_color,
        DebugLine::DEFAULT_WIDTH,
    ));
    lines.extend(highlight.collision_boxes.iter().flat_map(|collision_box| {
        DebugLine::cuboid(
            collision_box.min.map(|coordinate| coordinate as f32),
            collision_box.max.map(|coordinate| coordinate as f32),
            color,
            DebugLine::DEFAULT_WIDTH,
        )
    }));
}

fn chunk_min_block(position: f32) -> i32 {
    (position.floor() as i32).div_euclid(16) * 16
}

fn push_chunk_border_horizontal_ring(
    lines: &mut Vec<SelectionLine>,
    x_start: i32,
    z_start: i32,
    y: i32,
    color: [f32; 4],
    width: f32,
) {
    push_chunk_border_line(
        lines,
        [x_start, y, z_start],
        [x_start, y, z_start + 16],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start, y, z_start + 16],
        [x_start + 16, y, z_start + 16],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start + 16, y, z_start + 16],
        [x_start + 16, y, z_start],
        color,
        width,
    );
    push_chunk_border_line(
        lines,
        [x_start + 16, y, z_start],
        [x_start, y, z_start],
        color,
        width,
    );
}

fn push_chunk_border_line(
    lines: &mut Vec<SelectionLine>,
    from: [i32; 3],
    to: [i32; 3],
    color: [f32; 4],
    width: f32,
) {
    lines.push(SelectionLine {
        from: [from[0] as f32, from[1] as f32, from[2] as f32],
        to: [to[0] as f32, to[1] as f32, to[2] as f32],
        color,
        width,
    });
}

fn hud_debug_version_line() -> String {
    format!("Minecraft {MC_VERSION} ({MC_VERSION}/bbb-native)")
}

fn hud_debug_fps_line(fps: u32, client_framerate_limit: u32, client_vsync: bool) -> String {
    let limit = if client_framerate_limit == VANILLA_UNLIMITED_FRAMERATE_LIMIT {
        "inf".to_string()
    } else {
        client_framerate_limit.to_string()
    };
    let vsync = if client_vsync { " vsync" } else { "" };
    format!("{fps} fps T: {limit}{vsync}")
}

fn hud_debug_configured_framerate_limit(client_framerate_limit: u32) -> Option<u32> {
    (1..=250)
        .contains(&client_framerate_limit)
        .then_some(client_framerate_limit)
}

fn hud_debug_tps_line(world: &WorldStore) -> Option<String> {
    world.level_info()?;
    let ticking = world.ticking();
    let run_status = if ticking.frozen_ticks_to_run > 0 {
        " (frozen - stepping)"
    } else if ticking.frozen {
        " (frozen)"
    } else {
        ""
    };
    let brand = world.server_brand().unwrap_or("unknown");

    Some(format!("\"{brand}\" server{run_status}, 0 tx, 0 rx"))
}

fn hud_debug_day_count_line(world: &WorldStore) -> Option<String> {
    let day_time = world.world_time()?.day_time;
    Some(format!(
        "Day #{}",
        day_time / VANILLA_OVERWORLD_DAY_PERIOD_TICKS
    ))
}

fn hud_debug_light_line(world: &WorldStore, camera_pose: CameraPose) -> Option<String> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    let light = world.sample_block_light(feet_pos)?;
    let raw = light.sky.max(light.block);
    Some(format!(
        "Client Light: {raw} ({} sky, {} block)",
        light.sky, light.block
    ))
}

fn hud_debug_heightmap_lines(world: &WorldStore, camera_pose: CameraPose) -> Option<Vec<String>> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    world.probe_chunk(ChunkPos {
        x: feet_pos.x.div_euclid(16),
        z: feet_pos.z.div_euclid(16),
    })?;
    let client_height = |kind_id| hud_debug_heightmap_value(world, kind_id, feet_pos.x, feet_pos.z);
    let server_height =
        |kind_id| hud_debug_heightmap_server_value(world, kind_id, feet_pos.x, feet_pos.z);
    Some(vec![
        format!(
            "CH S: {} M: {} ML: {}",
            client_height(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID),
            client_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID),
            client_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID)
        ),
        format!(
            "SH S: {} O: {} M: {} ML: {}",
            server_height(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID),
            server_height(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID),
            server_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID),
            server_height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID)
        ),
    ])
}

fn hud_debug_heightmap_value(world: &WorldStore, kind_id: i32, x: i32, z: i32) -> String {
    world
        .sample_heightmap_first_available(kind_id, x, z)
        .map(|height| height.to_string())
        .unwrap_or_else(|| "??".to_string())
}

fn hud_debug_heightmap_server_value(
    _world: &WorldStore,
    _kind_id: i32,
    _x: i32,
    _z: i32,
) -> String {
    "??".to_string()
}

fn hud_debug_biome_line(world: &WorldStore, camera_pose: CameraPose) -> Option<String> {
    let feet_pos = camera_feet_block_position(camera_pose)?;
    let biome_id = world.probe_block(feet_pos)?.biome_id?;
    let biome = world
        .registry_resource_id_by_holder_id("minecraft:worldgen/biome", biome_id)
        .map(str::to_string)
        .unwrap_or_else(|| format!("[unregistered {biome_id}]"));
    Some(format!("Biome: {biome}"))
}

fn hud_debug_local_difficulty_line(
    _world: &WorldStore,
    _camera_pose: Option<CameraPose>,
) -> Option<String> {
    // Vanilla only displays local difficulty from an integrated ServerLevel and
    // server chunk. bbb has no local-server difficulty mirror yet.
    None
}

fn hud_debug_entity_spawn_counts_line(
    _world: &WorldStore,
    _camera_pose: Option<CameraPose>,
) -> Option<String> {
    // Vanilla only displays this entry from an integrated ServerLevel with a
    // populated NaturalSpawner SpawnState. bbb has no local-server mob-cap
    // mirror yet.
    None
}

fn hud_debug_looking_at_block_state_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_block_hit_from_camera(world, camera_pose)?;
    let probe = world.probe_block(hit.pos)?;
    let block_name = probe.block_name?;
    let mut lines = vec![
        format!(
            "Targeted Block: {}, {}, {}",
            hit.pos.x, hit.pos.y, hit.pos.z
        ),
        block_name,
    ];
    lines.extend(
        probe
            .block_properties
            .into_iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    Some(lines)
}

fn hud_debug_looking_at_block_tag_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_block_hit_from_camera(world, camera_pose)?;
    let block_name = world.probe_block(hit.pos)?.block_name?;
    let block_id = world.registry_holder_id_by_resource_id("minecraft:block", &block_name)?;
    Some(
        world
            .registry_tag_names_containing_holder_id("minecraft:block", block_id)?
            .map(|tag| format!("#{tag}"))
            .collect(),
    )
}

fn hud_debug_looking_at_fluid_state_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_fluid_hit_from_camera(world, camera_pose)?;
    let fluid = world.probe_block(hit.pos)?.fluid?;
    let mut lines = vec![
        format!(
            "Targeted Fluid: {}, {}, {}",
            hit.pos.x, hit.pos.y, hit.pos.z
        ),
        hud_debug_fluid_state_name(fluid).to_string(),
        format!("falling: {}", fluid.falling),
    ];
    if fluid.amount < 8 || fluid.falling {
        lines.push(format!("level: {}", fluid.amount));
    }
    Some(lines)
}

fn hud_debug_looking_at_fluid_tag_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    let hit = debug_looking_at_fluid_hit_from_camera(world, camera_pose)?;
    let fluid = world.probe_block(hit.pos)?.fluid?;
    let fluid_name = hud_debug_fluid_state_name(fluid);
    let fluid_id = world.registry_holder_id_by_resource_id("minecraft:fluid", fluid_name)?;
    Some(
        world
            .registry_tag_names_containing_holder_id("minecraft:fluid", fluid_id)?
            .map(|tag| format!("#{tag}"))
            .collect(),
    )
}

fn hud_debug_looking_at_entity_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    entity_partial_tick: f32,
) -> Option<Vec<String>> {
    let entity = hud_debug_looking_at_entity(world, camera_pose, entity_partial_tick)?;
    Some(vec![
        "Targeted Entity".to_string(),
        hud_debug_entity_type_name(world, entity.entity_type_id)?,
    ])
}

fn hud_debug_looking_at_entity_tag_lines(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    entity_partial_tick: f32,
) -> Option<Vec<String>> {
    let entity = hud_debug_looking_at_entity(world, camera_pose, entity_partial_tick)?;
    Some(
        world
            .registry_tag_names_containing_holder_id(
                "minecraft:entity_type",
                entity.entity_type_id,
            )?
            .map(|tag| format!("#{tag}"))
            .collect(),
    )
}

fn hud_debug_looking_at_entity(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    entity_partial_tick: f32,
) -> Option<bbb_world::EntityState> {
    let CrosshairTarget::Entity(hit) =
        crosshair_target_from_camera_at_partial_tick(world, camera_pose, entity_partial_tick)?
    else {
        return None;
    };
    world.probe_entity(hit.entity_id)
}

fn hud_debug_chunk_render_stats_line(
    counters: &bbb_renderer::RendererCounters,
    render_distance_chunks: u32,
    smart_cull: bool,
) -> String {
    let smart_cull = if smart_cull { "(s) " } else { "" };
    format!(
        "C: {}/{} {}D: {}, pC: {:03}, aB: {:02}",
        counters.visible_sections,
        counters.uploaded_sections,
        smart_cull,
        render_distance_chunks,
        counters.queued_sections,
        0
    )
}

fn hud_debug_chunk_generation_stats_lines(
    _world: &WorldStore,
    _camera_pose: Option<CameraPose>,
) -> Option<Vec<String>> {
    // Vanilla only displays this entry when the debug level is an integrated
    // ServerLevel. bbb does not own a local-server ChunkGenerator mirror yet.
    None
}

fn hud_debug_entity_render_stats_line(world: &WorldStore) -> String {
    let entity_count = world.entity_count();
    let simulation_distance = world.local_player().simulation_distance.unwrap_or(0);
    format!("E: {entity_count}/{entity_count}, SD: {simulation_distance}")
}

fn hud_debug_particle_render_stats_line(counters: &bbb_renderer::RendererCounters) -> String {
    format!("P: {}", counters.active_particle_instances)
}

fn hud_debug_chunk_source_stats_line(world: &WorldStore) -> String {
    let chunk_count = world.chunk_count();
    let entity_count = world.entity_count();
    format!(
        "Chunks[C] W: {}, {} E: {},0,{}",
        hud_debug_client_chunk_storage_slots(world.chunk_cache_radius()),
        chunk_count,
        entity_count,
        chunk_count
    )
}

fn hud_debug_sound_cache_line(counters: &AudioCounters) -> String {
    format!(
        "Sound cache: {} buffers, {} MiB",
        counters.sound_cache_buffers,
        hud_debug_sound_cache_bytes_to_mebibytes(counters.sound_cache_bytes)
    )
}

fn hud_debug_sound_mood_line(counters: &AudioCounters) -> String {
    format!(
        "Sounds: {}/{} + {}/{} (Mood {}%)",
        counters.sound_static_channels_used,
        counters.sound_static_channels_capacity,
        counters.sound_streaming_channels_used,
        counters.sound_streaming_channels_capacity,
        counters.sound_mood_percent.min(100)
    )
}

fn hud_debug_post_effect_line(current_post_effect_id: Option<&str>) -> Option<String> {
    Some(format!("Post: {}", current_post_effect_id?))
}

fn hud_debug_sound_cache_bytes_to_mebibytes(bytes: u64) -> u64 {
    if bytes == 0 {
        0
    } else {
        bytes.saturating_add(1024 * 1024 - 1) / (1024 * 1024)
    }
}

fn hud_debug_client_chunk_storage_slots(radius: Option<i32>) -> usize {
    let Some(radius) = radius else {
        return 0;
    };
    let storage_range = usize::try_from(radius.max(2))
        .unwrap_or(2)
        .saturating_add(3);
    let view_range = storage_range.saturating_mul(2).saturating_add(1);
    view_range.saturating_mul(view_range)
}

fn hud_debug_entity_type_name(world: &WorldStore, entity_type_id: i32) -> Option<String> {
    world
        .registry_resource_id_by_holder_id("minecraft:entity_type", entity_type_id)
        .map(str::to_string)
}

fn hud_debug_fluid_state_name(fluid: TerrainFluidState) -> &'static str {
    match (fluid.kind, fluid.amount >= 8 && !fluid.falling) {
        (TerrainFluidKind::Water, true) => "minecraft:water",
        (TerrainFluidKind::Water, false) => "minecraft:flowing_water",
        (TerrainFluidKind::Lava, true) => "minecraft:lava",
        (TerrainFluidKind::Lava, false) => "minecraft:flowing_lava",
    }
}

fn camera_feet_block_position(camera: CameraPose) -> Option<BlockPos> {
    let [x, y, z] = camera.position;
    if ![x, y, z].into_iter().all(f32::is_finite) {
        return None;
    }
    Some(BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    })
}

fn hud_debug_visibility_label(visible: bool) -> &'static str {
    if visible {
        "visible"
    } else {
        "hidden"
    }
}

fn hud_debug_enabled_label(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudDebugMemorySnapshot {
    used_mib: u64,
    max_mib: u64,
    allocated_mib: u64,
    allocation_rate_mib_per_s: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudDebugMemoryUsageSnapshot {
    init_mib: u64,
    used_mib: u64,
    committed_mib: u64,
    max_mib: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudDebugDetailedMemorySnapshot {
    heap: HudDebugMemoryUsageSnapshot,
    non_heap: HudDebugMemoryUsageSnapshot,
}

fn hud_debug_memory_snapshot() -> HudDebugMemorySnapshot {
    let used_kib = proc_kib_field("/proc/self/status", "VmRSS:").unwrap_or(0);
    let allocated_kib = proc_kib_field("/proc/self/status", "VmSize:").unwrap_or(used_kib);
    let max_kib =
        proc_kib_field("/proc/meminfo", "MemTotal:").unwrap_or_else(|| allocated_kib.max(used_kib));

    HudDebugMemorySnapshot {
        used_mib: kib_to_mib(used_kib),
        max_mib: kib_to_mib(max_kib),
        allocated_mib: kib_to_mib(allocated_kib),
        allocation_rate_mib_per_s: 0,
    }
}

fn hud_debug_memory_lines(snapshot: HudDebugMemorySnapshot) -> Vec<String> {
    vec![
        format!(
            "Mem: {:2}% {:03}/{:03}MiB",
            debug_percent(snapshot.used_mib, snapshot.max_mib),
            snapshot.used_mib,
            snapshot.max_mib
        ),
        format!(
            "Allocation rate: {:03}MiB/s",
            snapshot.allocation_rate_mib_per_s
        ),
        format!(
            "Allocated: {:2}% {:03}MiB",
            debug_percent(snapshot.allocated_mib, snapshot.max_mib),
            snapshot.allocated_mib
        ),
    ]
}

fn hud_debug_detailed_memory_snapshot() -> HudDebugDetailedMemorySnapshot {
    let status_path = "/proc/self/status";
    let used_kib = proc_kib_field(status_path, "VmRSS:").unwrap_or(0);
    let committed_kib = proc_kib_field(status_path, "VmSize:").unwrap_or(used_kib);
    let max_kib =
        proc_kib_field("/proc/meminfo", "MemTotal:").unwrap_or_else(|| committed_kib.max(used_kib));
    let non_heap_used_kib = proc_kib_field(status_path, "VmExe:")
        .unwrap_or(0)
        .saturating_add(proc_kib_field(status_path, "VmLib:").unwrap_or(0))
        .saturating_add(proc_kib_field(status_path, "VmStk:").unwrap_or(0));

    HudDebugDetailedMemorySnapshot {
        heap: HudDebugMemoryUsageSnapshot {
            init_mib: 0,
            used_mib: kib_to_mib(used_kib),
            committed_mib: kib_to_mib(committed_kib),
            max_mib: kib_to_mib(max_kib),
        },
        non_heap: HudDebugMemoryUsageSnapshot {
            init_mib: 0,
            used_mib: kib_to_mib(non_heap_used_kib),
            committed_mib: kib_to_mib(non_heap_used_kib),
            max_mib: 0,
        },
    }
}

fn hud_debug_detailed_memory_lines(snapshot: HudDebugDetailedMemorySnapshot) -> Vec<String> {
    vec![
        hud_debug_memory_usage_line("heap", snapshot.heap),
        hud_debug_memory_usage_line("non-heap", snapshot.non_heap),
    ]
}

fn hud_debug_memory_usage_line(kind: &str, usage: HudDebugMemoryUsageSnapshot) -> String {
    format!(
        "Memory ({kind}): i={:03}MiB u={:03}MiB c={:03}MiB m={:03}MiB",
        usage.init_mib, usage.used_mib, usage.committed_mib, usage.max_mib
    )
}

fn hud_debug_system_lines(surface_size: winit::dpi::PhysicalSize<u32>) -> Vec<String> {
    let cpu_threads = std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(1);

    vec![
        format!("Java: n/a (bbb-native {})", env!("CARGO_PKG_VERSION")),
        format!(
            "CPU: {}x {}-{}",
            cpu_threads,
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
        format!(
            "Display: {}x{} (wgpu)",
            surface_size.width, surface_size.height
        ),
        "bbb renderer".to_string(),
        "wgpu native".to_string(),
    ]
}

fn hud_debug_gpu_utilization_line(gpu_utilization: Option<f64>) -> String {
    let Some(utilization) = gpu_utilization.filter(|value| value.is_finite()) else {
        return "GPU: N/A".to_string();
    };
    format!("GPU: {:.0}%", utilization.clamp(0.0, 100.0))
}

fn hud_debug_simple_performance_impactor_lines() -> Vec<String> {
    vec!["B: 2".to_string(), "Filtering: Nearest".to_string()]
}

fn debug_percent(value: u64, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        value.saturating_mul(100) / total
    }
}

fn kib_to_mib(kib: u64) -> u64 {
    kib / 1024
}

fn proc_kib_field(path: &str, field: &str) -> Option<u64> {
    fs::read_to_string(path).ok()?.lines().find_map(|line| {
        let rest = line.strip_prefix(field)?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

fn hud_debug_position_lines(world: &WorldStore, camera_pose: CameraPose) -> Vec<String> {
    let x = f64::from(camera_pose.position[0]);
    let y = f64::from(camera_pose.position[1]);
    let z = f64::from(camera_pose.position[2]);
    let block_x = x.floor() as i32;
    let block_y = y.floor() as i32;
    let block_z = z.floor() as i32;
    let chunk_x = block_x.div_euclid(16);
    let chunk_z = block_z.div_euclid(16);
    let section_y = block_y.div_euclid(16);
    let (direction, face_string) = hud_debug_facing(camera_pose.y_rot);
    let dimension = world
        .level_info()
        .map(|level| level.dimension.as_str())
        .unwrap_or("minecraft:overworld");

    vec![
        format!("XYZ: {x:.3} / {y:.5} / {z:.3}"),
        format!("Block: {block_x} {block_y} {block_z}"),
        format!(
            "Chunk: {chunk_x} {section_y} {chunk_z} [{} {} in r.{}.{}.mca]",
            chunk_x.rem_euclid(32),
            chunk_z.rem_euclid(32),
            chunk_x.div_euclid(32),
            chunk_z.div_euclid(32)
        ),
        format!(
            "Facing: {direction} ({face_string}) ({:.1} / {:.1})",
            hud_debug_wrap_degrees(camera_pose.y_rot),
            hud_debug_wrap_degrees(camera_pose.x_rot)
        ),
        format!("{dimension} FC: 0"),
    ]
}

fn hud_debug_section_position_line(camera_pose: CameraPose) -> String {
    let block_x = f64::from(camera_pose.position[0]).floor() as i32;
    let block_y = f64::from(camera_pose.position[1]).floor() as i32;
    let block_z = f64::from(camera_pose.position[2]).floor() as i32;

    format!(
        "Section-relative: {:02} {:02} {:02}",
        block_x & 15,
        block_y & 15,
        block_z & 15
    )
}

fn hud_debug_facing(y_rot: f32) -> (&'static str, &'static str) {
    let wrapped = hud_debug_wrap_degrees(y_rot);
    if (-45.0..45.0).contains(&wrapped) {
        ("south", "Towards positive Z")
    } else if (45.0..135.0).contains(&wrapped) {
        ("west", "Towards negative X")
    } else if (-135.0..-45.0).contains(&wrapped) {
        ("east", "Towards positive X")
    } else {
        ("north", "Towards negative Z")
    }
}

fn panorama_wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn hud_debug_wrap_degrees(degrees: f32) -> f32 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 {
        180.0
    } else {
        wrapped
    }
}

#[cfg(test)]
fn hud_inventory_screen_with_local_state_for_surface(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let hud_fonts = approximate_inventory_hud_fonts();
    hud_inventory_screen_with_local_state_for_surface_and_measurer(
        world,
        item_runtime,
        terrain_textures,
        hovered_slot_id,
        local_state,
        surface_size,
        &dialog_screen::ApproximateDialogTextMeasurer,
        &hud_fonts,
        None,
        None,
        partial_tick,
    )
}

#[cfg(test)]
fn approximate_inventory_hud_fonts() -> HudFontGlyphBook {
    let mut glyphs = bbb_renderer::HudFontGlyphMap::new();
    for ch in ' '..='~' {
        let advance = hud_ascii_approx_char_width(ch);
        glyphs.insert_first_wins(
            ch,
            bbb_renderer::HudAsciiGlyph {
                width: advance.saturating_sub(1),
                height: 8,
                advance,
                ..bbb_renderer::HudAsciiGlyph::default()
            },
        );
    }
    HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([
            ("minecraft:default".to_string(), glyphs.clone()),
            (ENCHANTMENT_ALT_FONT_ID.to_string(), glyphs),
        ]),
    )
}

fn hud_inventory_screen_with_local_state_for_surface_and_measurer(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    surface_size: winit::dpi::PhysicalSize<u32>,
    dialog_text_measurer: &dyn dialog_screen::DialogTextMeasurer,
    hud_fonts: &HudFontGlyphBook,
    entity_variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    enchantment_book: Option<EnchantmentBookHudState>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    if let Some(confirmation) = world.dialog_link_confirmation() {
        return Some(hud_dialog_confirm_link_screen(
            confirmation,
            world.dialog_link_focused_button(),
            local_state.cursor_position,
            surface_size,
        ));
    }
    if world.dialog_warning_is_open() {
        return Some(hud_dialog_warning_screen(
            world.dialog_warning_focused_button(),
            local_state.cursor_position,
            surface_size,
        ));
    }
    if let Some(dialog) = world.current_dialog() {
        return hud_dialog_screen_with_measurer(
            world,
            dialog,
            item_runtime,
            terrain_textures,
            &local_state,
            surface_size,
            dialog_text_measurer,
            partial_tick,
        );
    }
    if let Some(waiting_ticks) = world.dialog_waiting_ticks() {
        return Some(hud_waiting_response_screen(
            waiting_ticks,
            local_state.cursor_position,
            surface_size,
        ));
    }

    if world.advancements_screen_is_open() {
        return Some(hud_advancements_screen(
            world,
            item_runtime,
            terrain_textures,
            local_state.advancement_scroll_delta,
            local_state.advancement_hover_fade,
            surface_size,
            local_state.cursor_position,
            local_state.keybind_context,
            partial_tick,
        ));
    }

    if world.stats_screen_is_open() {
        return None;
    }

    if world.game_rules_screen().is_some() {
        return None;
    }

    if world.creative_inventory_screen().is_some() {
        return hud_creative_inventory_screen(
            world,
            item_runtime,
            terrain_textures,
            local_state.creative_inventory?,
            local_state.shift_down,
            local_state.keybind_context,
            local_state.advanced_item_tooltips,
            partial_tick,
        );
    }

    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    if layout.background == InventoryScreenBackground::Lectern {
        return None;
    }
    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };
    let enchantment_options =
        enchantment_option_hud_states(world, local_state.cursor_position, hud_fonts);
    let main_offset_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.main_gui_x_offset)
        .unwrap_or_default();
    let recipe_book_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.x)
        .unwrap_or_default();
    let recipe_book_cursor_position = local_state
        .cursor_position
        .map(|(x, y)| (x - recipe_book_x, y));

    let selected_hotbar_slot_id = inventory_screen_selected_hotbar_slot_id(world);
    let background = layout.background;
    let loom_presentation = (background == InventoryScreenBackground::Loom).then(|| {
        loom_screen::loom_table_presentation(
            world,
            item_runtime,
            entity_variant_textures,
            local_state.loom_pattern_scroll_row.unwrap_or_default(),
            local_state.loom_selected_pattern_index,
            local_state.cursor_position,
        )
    });
    let slots = layout
        .slots
        .iter()
        .copied()
        .map(|layout| {
            let inventory_slot = container
                .slots
                .iter()
                .find(|slot| slot.slot == layout.slot_id);
            let owner_team_color_rgb =
                if inventory_screen_slot_uses_fake_item(background, layout.slot_id) {
                    None
                } else {
                    world.local_player_team_color_rgb()
                };
            HudInventorySlot {
                slot_id: u16::try_from(layout.slot_id).unwrap_or_default(),
                x: layout.x,
                y: layout.y,
                icon: inventory_slot.and_then(|slot| {
                    hud_item_icon_for_stack_with_owner_team_color(
                        world,
                        item_runtime,
                        &slot.item,
                        (slot.local_selected_bundle_item_index >= 0)
                            .then_some(slot.local_selected_bundle_item_index),
                        false,
                        selected_hotbar_slot_id == Some(layout.slot_id),
                        false,
                        false,
                        local_state.shift_down,
                        local_state.keybind_context,
                        0,
                        partial_tick,
                        owner_team_color_rgb,
                    )
                }),
                block_model: inventory_slot.and_then(|slot| {
                    block_item_3d_model_with_owner_team_color(
                        world,
                        &slot.item,
                        item_runtime,
                        terrain_textures,
                        (slot.local_selected_bundle_item_index >= 0)
                            .then_some(slot.local_selected_bundle_item_index),
                        false,
                        selected_hotbar_slot_id == Some(layout.slot_id),
                        false,
                        false,
                        local_state.shift_down,
                        local_state.keybind_context,
                        0,
                        owner_team_color_rgb,
                    )
                }),
            }
        })
        .collect();

    let mut background_layers = hud_inventory_background_layers(
        world,
        item_runtime,
        layout.background,
        &local_state,
        enchantment_options.as_ref(),
    );
    if background == InventoryScreenBackground::Smithing {
        background_layers.extend(local_state.smithing.background_layers.iter().copied());
    }
    if let Some(presentation) = &loom_presentation {
        background_layers.extend(presentation.background_layers.iter().copied());
    }
    if main_offset_x != 0 {
        for layer in &mut background_layers {
            layer.x += main_offset_x;
        }
    }
    if let Some(recipe_book) = layout.recipe_book {
        let mut layer = hud_recipe_book_background_layer();
        layer.x += recipe_book.x;
        if recipe_book.narrow {
            background_layers.push(layer);
        } else {
            background_layers.insert(0, layer);
        }
    }
    let mut recipe_book_tab_layers = hud_recipe_book_tab_layers(
        world,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_presentation,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_tab_layers, recipe_book_x);
    background_layers.extend(recipe_book_tab_layers);
    if let Some(layer) =
        hud_recipe_book_search_box_layer(world, layout.background, &local_state.recipe_book_search)
    {
        let mut layers = vec![layer];
        offset_hud_inventory_background_layers(&mut layers, recipe_book_x);
        background_layers.extend(layers);
    }
    let mut recipe_book_button_layers = hud_recipe_book_recipe_button_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        &local_state.recipe_book_presentation,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_button_layers, recipe_book_x);
    background_layers.extend(recipe_book_button_layers);
    let mut recipe_book_page_layers = hud_recipe_book_page_button_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        recipe_book_cursor_position,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_page_layers, recipe_book_x);
    background_layers.extend(recipe_book_page_layers);
    if let Some(layer) = hud_recipe_book_button_layer(
        layout.background,
        main_offset_x,
        local_state.cursor_position,
    ) {
        background_layers.push(layer);
    }
    if let Some(layer) =
        hud_recipe_book_filter_button_layer(world, layout.background, recipe_book_cursor_position)
    {
        let mut layers = vec![layer];
        offset_hud_inventory_background_layers(&mut layers, recipe_book_x);
        background_layers.extend(layers);
    }
    let mut recipe_book_overlay_layers = hud_recipe_book_overlay_layers(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        recipe_book_cursor_position,
        partial_tick,
    );
    offset_hud_inventory_background_layers(&mut recipe_book_overlay_layers, recipe_book_x);
    background_layers.extend(recipe_book_overlay_layers);
    let mut floating_items = hud_inventory_floating_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        main_offset_x,
        local_state.stonecutter_recipe_scroll_row,
        local_state.cursor_position,
        local_state.quick_craft_button_num,
        &local_state.quick_craft_slots,
        local_state.shift_down,
        local_state.keybind_context,
        partial_tick,
    );
    let mut recipe_book_tab_icon_items = hud_recipe_book_tab_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_presentation,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_tab_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_tab_icon_items);
    let mut recipe_book_recipe_button_icon_items = hud_recipe_book_recipe_button_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        &local_state.recipe_book_presentation,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_recipe_button_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_recipe_button_icon_items);
    let mut recipe_book_overlay_icon_items = hud_recipe_book_overlay_icon_items(
        world,
        item_runtime,
        terrain_textures,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        local_state.keybind_context,
        partial_tick,
    );
    offset_hud_inventory_items(&mut recipe_book_overlay_icon_items, recipe_book_x);
    floating_items.extend(recipe_book_overlay_icon_items);
    let (fill_layers, ghost_items) = hud_recipe_book_ghost_layers_and_items(
        world,
        item_runtime,
        layout.background,
        &layout.slots,
        local_state.keybind_context,
        partial_tick,
    );
    let mut entity_previews = hud_inventory_entity_previews(
        world,
        item_runtime,
        layout.background,
        &local_state,
        enchantment_book,
        partial_tick,
    );
    if let Some(result) = loom_presentation
        .as_ref()
        .and_then(|presentation| presentation.result.as_ref())
    {
        entity_previews.push(HudEntityPreview {
            entity: EntityModelInstance::banner_flag(-1, result.base_color, result.layers)
                .with_light_coords(ENTITY_FULL_BRIGHT_LIGHT_COORDS),
            lighting: GuiItemLightingEntry::ItemsFlat,
            rect: HudEntityPreviewRect {
                x: 141,
                y: 8,
                width: 20,
                height: 40,
            },
            scissor: None,
            translation: [0.0, 0.25, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            override_camera_rotation: None,
            scale: 16.0,
            depth_isolated: true,
            after_background_layer: Some(result.after_background_layer),
            item_layers: Vec::new(),
        });
    }
    offset_hud_entity_previews(&mut entity_previews, main_offset_x);
    let mut map_previews = if layout.background == InventoryScreenBackground::CartographyTable {
        cartography_table_presentation(world, item_runtime).map_previews
    } else {
        Vec::new()
    };
    offset_hud_inventory_map_previews(&mut map_previews, main_offset_x);
    let mut banner_pattern_previews = loom_presentation
        .as_ref()
        .map(|presentation| presentation.pattern_previews.clone())
        .unwrap_or_default();
    for preview in &mut banner_pattern_previews {
        preview.x += main_offset_x;
    }
    let mut text_labels = hud_inventory_text_labels(
        world,
        item_runtime,
        layout.background,
        layout.height,
        &local_state,
        enchantment_options.as_ref(),
    );
    offset_hud_inventory_text_labels(&mut text_labels, main_offset_x);
    let mut recipe_book_text_labels = Vec::new();
    if let Some(label) =
        hud_recipe_book_search_text_label(world, layout.background, &local_state.recipe_book_search)
    {
        recipe_book_text_labels.push(label);
    }
    if let Some(label) = hud_recipe_book_page_text_label(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
    ) {
        recipe_book_text_labels.push(label);
    }
    offset_hud_inventory_text_labels(&mut recipe_book_text_labels, recipe_book_x);
    text_labels.extend(recipe_book_text_labels);
    let foreground_layers = if layout.background == InventoryScreenBackground::Merchant {
        hud_merchant_discount_layers(world)
    } else {
        Vec::new()
    };
    let creative = world.gameplay().game_type == GameType::Creative.id();
    let inventory_tooltip = hud_recipe_book_tooltip(
        world,
        item_runtime,
        layout.background,
        &local_state.recipe_book_tabs,
        &local_state.recipe_book_pages,
        &local_state.recipe_book_search,
        local_state.recipe_book_overlay,
        recipe_book_cursor_position,
        recipe_book_x,
        local_state.advanced_item_tooltips,
        creative,
        partial_tick,
    )
    .or_else(|| {
        hud_recipe_book_ghost_tooltip(
            world,
            item_runtime,
            layout.background,
            hovered_slot_id,
            &layout.slots,
            local_state.advanced_item_tooltips,
            creative,
            partial_tick,
        )
    })
    .or_else(|| {
        hud_inventory_tooltip(
            world,
            item_runtime,
            hovered_slot_id,
            &layout.slots,
            container,
            local_state.advanced_item_tooltips,
            creative,
        )
    });
    let tooltip = match layout.background {
        InventoryScreenBackground::EnchantmentTable => hud_enchantment_table_tooltip(
            world,
            item_runtime,
            local_state.cursor_position,
            enchantment_options.as_ref(),
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Merchant => hud_merchant_trade_tooltip(
            world,
            item_runtime,
            local_state.cursor_position,
            local_state.advanced_item_tooltips,
            creative,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Stonecutter => hud_stonecutter_recipe_tooltip(
            world,
            item_runtime,
            local_state.cursor_position,
            local_state
                .stonecutter_recipe_scroll_row
                .unwrap_or_default(),
            local_state.advanced_item_tooltips,
            creative,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Loom => loom_presentation
            .as_ref()
            .and_then(|presentation| presentation.tooltip.clone())
            .or(inventory_tooltip),
        InventoryScreenBackground::Smithing => smithing_screen::tooltip(
            world,
            item_runtime,
            hovered_slot_id,
            local_state.cursor_position,
            hud_fonts,
        )
        .or(inventory_tooltip),
        InventoryScreenBackground::Beacon => hud_beacon_tooltip(
            item_runtime,
            local_state.cursor_position,
            local_state
                .beacon_effect_selection
                .unwrap_or_else(|| beacon_effect_selection_from_world(world)),
        )
        .or(inventory_tooltip),
        _ => inventory_tooltip,
    };

    Some(HudInventoryScreen {
        width: u32::try_from(layout.width).unwrap_or_default(),
        height: u32::try_from(layout.height).unwrap_or_default(),
        background_layers,
        foreground_layers,
        fill_layers,
        slots,
        floating_items,
        foreground_items: Vec::new(),
        ghost_items,
        entity_previews,
        map_previews,
        banner_pattern_previews,
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: hovered_slot_id.and_then(|slot| u16::try_from(slot).ok()),
        tooltip,
    })
}

fn inventory_screen_slot_uses_fake_item(
    background: InventoryScreenBackground,
    slot_id: i16,
) -> bool {
    matches!(
        (background, slot_id),
        (
            InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable,
            0
        ) | (InventoryScreenBackground::Crafter, 45)
    )
}

pub(crate) fn hud_screen_blur_for_world(world: &WorldStore, screen_present: bool) -> bool {
    screen_present
        && (world.dialog_warning_is_open()
            || world.current_dialog().is_some()
            || world.dialog_waiting_ticks().is_some()
            || world.game_rules_screen().is_some()
            || matches!(
                world.game_event_screen(),
                Some(bbb_world::ClientGameEventScreenState::DemoHelp { .. })
            ))
}

fn hud_recipe_book_background_layer() -> HudInventoryBackgroundLayer {
    hud_inventory_background_layer(
        HudInventoryBackgroundTexture::RecipeBook,
        0,
        0,
        147,
        166,
        [1.0 / 256.0, 1.0 / 256.0],
        [148.0 / 256.0, 167.0 / 256.0],
    )
}

#[derive(Debug, Clone, Copy)]
struct RecipeBookTabIconSpec {
    primary: &'static str,
    secondary: Option<&'static str>,
}

const CRAFTING_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 5] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:iron_axe",
        secondary: Some("minecraft:golden_sword"),
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:bricks",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:lava_bucket",
        secondary: Some("minecraft:apple"),
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:redstone",
        secondary: None,
    },
];

const FURNACE_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 4] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:porkchop",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:stone",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:lava_bucket",
        secondary: Some("minecraft:emerald"),
    },
];

const BLAST_FURNACE_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 3] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:redstone_ore",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:iron_shovel",
        secondary: Some("minecraft:golden_leggings"),
    },
];

const SMOKER_RECIPE_BOOK_TAB_ICONS: [RecipeBookTabIconSpec; 2] = [
    RecipeBookTabIconSpec {
        primary: "minecraft:compass",
        secondary: None,
    },
    RecipeBookTabIconSpec {
        primary: "minecraft:porkchop",
        secondary: None,
    },
];
const RECIPE_BOOK_TAB_ANIMATION_TIME: f32 = 15.0;
const RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y: f32 = 12.0;
const RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X: f32 = 8.0;
const RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y: f32 = 12.0;

fn hud_recipe_book_tab_layers(
    world: &WorldStore,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    presentation: &RecipeBookPresentationHudState,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return Vec::new();
    }
    let visible_tabs = recipe_book_visible_tab_indices(world, background);
    if visible_tabs.is_empty() {
        return Vec::new();
    };
    let selected_index = recipe_book_selected_tab_index(background, tabs).unwrap_or_default();
    visible_tabs
        .into_iter()
        .enumerate()
        .map(|(visible_index, tab_index)| {
            let selected = tab_index == selected_index;
            let scale_y = recipe_book_animation_scale(
                presentation
                    .tab_animation_ticks
                    .get(tab_index)
                    .copied()
                    .unwrap_or_default(),
                partial_tick,
            );
            let y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
            let (y, height) =
                scaled_recipe_book_tab_y_and_height(y, RECIPE_BOOK_TAB_HEIGHT, scale_y);
            hud_inventory_background_layer(
                if selected {
                    HudInventoryBackgroundTexture::RecipeBookTabSelected
                } else {
                    HudInventoryBackgroundTexture::RecipeBookTab
                },
                RECIPE_BOOK_TAB_X
                    + if selected {
                        RECIPE_BOOK_SELECTED_TAB_X_OFFSET
                    } else {
                        0
                    },
                y,
                u32::try_from(RECIPE_BOOK_TAB_WIDTH).unwrap_or_default(),
                height,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

fn recipe_book_animation_scale(remaining: u8, partial_tick: f32) -> f32 {
    if remaining == 0 {
        return 1.0;
    }
    let animation_time = (remaining as f32 - partial_tick.clamp(0.0, 1.0)).max(0.0);
    if animation_time <= 0.0 {
        return 1.0;
    }
    1.0 + 0.1 * (animation_time / RECIPE_BOOK_TAB_ANIMATION_TIME * std::f32::consts::PI).sin()
}

fn scaled_recipe_book_tab_y_and_height(y: i32, height: i32, scale_y: f32) -> (i32, u32) {
    if (scale_y - 1.0).abs() <= f32::EPSILON {
        return (y, u32::try_from(height).unwrap_or_default());
    }
    let pivot_y = y as f32 + RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y;
    let top = pivot_y + (y as f32 - pivot_y) * scale_y;
    let bottom = pivot_y + ((y + height) as f32 - pivot_y) * scale_y;
    let scaled_y = top.round() as i32;
    let scaled_height = (bottom - top).round().max(1.0) as u32;
    (scaled_y, scaled_height)
}

fn scaled_recipe_book_tab_child_y(tab_y: i32, child_y: i32, scale_y: f32) -> i32 {
    if (scale_y - 1.0).abs() <= f32::EPSILON {
        return child_y;
    }
    let pivot_y = tab_y as f32 + RECIPE_BOOK_TAB_ANIMATION_PIVOT_Y;
    (pivot_y + (child_y as f32 - pivot_y) * scale_y).round() as i32
}

fn hud_recipe_book_tab_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    presentation: &RecipeBookPresentationHudState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return Vec::new();
    }
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some(selected_index) = recipe_book_selected_tab_index(background, tabs) else {
        return Vec::new();
    };
    let Some(icons) = recipe_book_tab_icons(background) else {
        return Vec::new();
    };
    let visible_tabs = recipe_book_visible_tab_indices(world, background);
    let mut items = Vec::new();
    for (visible_index, tab_index) in visible_tabs.into_iter().enumerate() {
        let Some(icon) = icons.get(tab_index) else {
            continue;
        };
        let move_left = if tab_index == selected_index {
            RECIPE_BOOK_SELECTED_TAB_X_OFFSET
        } else {
            0
        };
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        let scale_y = recipe_book_animation_scale(
            presentation
                .tab_animation_ticks
                .get(tab_index)
                .copied()
                .unwrap_or_default(),
            partial_tick,
        );
        let y = scaled_recipe_book_tab_child_y(tab_y, tab_y + 5, scale_y);
        if let Some(secondary) = icon.secondary {
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                icon.primary,
                RECIPE_BOOK_TAB_X + 3 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                secondary,
                RECIPE_BOOK_TAB_X + 14 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
        } else {
            push_recipe_book_tab_icon_item(
                &mut items,
                world,
                item_runtime,
                terrain_textures,
                icon.primary,
                RECIPE_BOOK_TAB_X + 9 + move_left,
                y,
                scale_y,
                keybind_context,
                partial_tick,
            );
        }
    }
    items
}

fn push_recipe_book_tab_icon_item(
    items: &mut Vec<HudInventoryItem>,
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    resource_id: &str,
    x: i32,
    y: i32,
    scale_y: f32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) {
    let Some(item_id) = item_runtime.item_protocol_id(resource_id) else {
        return;
    };
    let stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    };
    let Some(icon) = hud_fake_item_icon_for_stack(
        world,
        Some(item_runtime),
        &stack,
        None,
        false,
        false,
        false,
        false,
        false,
        keybind_context,
        0,
        partial_tick,
    ) else {
        return;
    };
    items.push(HudInventoryItem {
        x,
        y,
        scale: 1.0,
        scale_y,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model: fake_block_item_3d_model(
            world,
            &stack,
            Some(item_runtime),
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        ),
    });
}

fn recipe_book_selected_tab_index(
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
) -> Option<usize> {
    let tab_count = recipe_book_tab_count_for_background(background)?;
    if tab_count == 0 {
        return None;
    }
    let index = match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            tabs.crafting
        }
        InventoryScreenBackground::Furnace => tabs.furnace,
        InventoryScreenBackground::BlastFurnace => tabs.blast_furnace,
        InventoryScreenBackground::Smoker => tabs.smoker,
        _ => return None,
    };
    Some(index.min(tab_count - 1))
}

fn recipe_book_selected_page_index(
    background: InventoryScreenBackground,
    pages: &RecipeBookPageHudState,
) -> Option<usize> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(pages.crafting)
        }
        InventoryScreenBackground::Furnace => Some(pages.furnace),
        InventoryScreenBackground::BlastFurnace => Some(pages.blast_furnace),
        InventoryScreenBackground::Smoker => Some(pages.smoker),
        _ => None,
    }
}

fn recipe_book_tab_icons(
    background: InventoryScreenBackground,
) -> Option<&'static [RecipeBookTabIconSpec]> {
    match background {
        InventoryScreenBackground::LocalInventory | InventoryScreenBackground::CraftingTable => {
            Some(&CRAFTING_RECIPE_BOOK_TAB_ICONS)
        }
        InventoryScreenBackground::Furnace => Some(&FURNACE_RECIPE_BOOK_TAB_ICONS),
        InventoryScreenBackground::BlastFurnace => Some(&BLAST_FURNACE_RECIPE_BOOK_TAB_ICONS),
        InventoryScreenBackground::Smoker => Some(&SMOKER_RECIPE_BOOK_TAB_ICONS),
        _ => None,
    }
}

fn hud_recipe_book_recipe_button_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    presentation: &RecipeBookPresentationHudState,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    hud_recipe_book_visible_collections(world, item_runtime, background, tabs, pages, search)
        .into_iter()
        .enumerate()
        .map(|(index, collection)| {
            let (x, y) = recipe_book_recipe_button_position(index);
            let scale = recipe_book_animation_scale(
                presentation
                    .button_animation_ticks
                    .get(index)
                    .copied()
                    .unwrap_or_default(),
                partial_tick,
            );
            let (x, y, width, height) = scaled_recipe_book_button_rect(
                x,
                y,
                RECIPE_BOOK_RECIPE_BUTTON_SIZE,
                RECIPE_BOOK_RECIPE_BUTTON_SIZE,
                scale,
            );
            hud_inventory_background_layer(
                match (
                    collection.has_craftable(),
                    collection.has_multiple_recipes(),
                ) {
                    (true, true) => HudInventoryBackgroundTexture::RecipeBookSlotManyCraftable,
                    (true, false) => HudInventoryBackgroundTexture::RecipeBookSlotCraftable,
                    (false, true) => HudInventoryBackgroundTexture::RecipeBookSlotManyUncraftable,
                    (false, false) => HudInventoryBackgroundTexture::RecipeBookSlotUncraftable,
                },
                x,
                y,
                width,
                height,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

fn hud_recipe_book_recipe_button_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    presentation: &RecipeBookPresentationHudState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let collections = hud_recipe_book_visible_collections(
        world,
        Some(item_runtime),
        background,
        tabs,
        pages,
        search,
    );
    let mut items = Vec::new();
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    for (index, collection) in collections.into_iter().enumerate() {
        let Some(stack) = collection.result_stack_at_slot_select_index(slot_select_index) else {
            continue;
        };
        let Some(icon) = hud_fake_item_icon_for_stack(
            world,
            Some(item_runtime),
            stack,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
            partial_tick,
        ) else {
            continue;
        };
        let (button_x, button_y) = recipe_book_recipe_button_position(index);
        let scale = recipe_book_animation_scale(
            presentation
                .button_animation_ticks
                .get(index)
                .copied()
                .unwrap_or_default(),
            partial_tick,
        );
        let block_model = fake_block_item_3d_model(
            world,
            stack,
            Some(item_runtime),
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        );
        if collection.has_multiple_recipes() && collection.all_result_stacks_same() {
            let (x, y) = scaled_recipe_book_button_child_position(
                button_x,
                button_y,
                button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET + 1,
                button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET + 1,
                scale,
            );
            items.push(HudInventoryItem {
                x,
                y,
                scale,
                scale_y: scale,
                icon: icon.clone(),
                scissor: None,
                draw_decorations: true,
                block_model: block_model.clone(),
            });
            let (x, y) = scaled_recipe_book_button_child_position(
                button_x,
                button_y,
                button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET - 1,
                button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET - 1,
                scale,
            );
            items.push(HudInventoryItem {
                x,
                y,
                scale,
                scale_y: scale,
                icon,
                scissor: None,
                draw_decorations: true,
                block_model,
            });
            continue;
        }
        let (x, y) = scaled_recipe_book_button_child_position(
            button_x,
            button_y,
            button_x + RECIPE_BOOK_RECIPE_ICON_OFFSET,
            button_y + RECIPE_BOOK_RECIPE_ICON_OFFSET,
            scale,
        );
        items.push(HudInventoryItem {
            x,
            y,
            scale,
            scale_y: scale,
            icon,
            scissor: None,
            draw_decorations: true,
            block_model,
        });
    }
    items
}

fn scaled_recipe_book_button_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: f32,
) -> (i32, i32, u32, u32) {
    let pivot_x = x as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X;
    let pivot_y = y as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y;
    let left = pivot_x + (x as f32 - pivot_x) * scale;
    let top = pivot_y + (y as f32 - pivot_y) * scale;
    let right = pivot_x + ((x + width) as f32 - pivot_x) * scale;
    let bottom = pivot_y + ((y + height) as f32 - pivot_y) * scale;
    (
        left.round() as i32,
        top.round() as i32,
        (right - left).round().max(1.0) as u32,
        (bottom - top).round().max(1.0) as u32,
    )
}

fn scaled_recipe_book_button_child_position(
    button_x: i32,
    button_y: i32,
    child_x: i32,
    child_y: i32,
    scale: f32,
) -> (i32, i32) {
    let pivot_x = button_x as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_X;
    let pivot_y = button_y as f32 + RECIPE_BOOK_BUTTON_ANIMATION_PIVOT_Y;
    (
        (pivot_x + (child_x as f32 - pivot_x) * scale).round() as i32,
        (pivot_y + (child_y as f32 - pivot_y) * scale).round() as i32,
    )
}

fn hud_recipe_book_overlay_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    cursor_position: Option<(i32, i32)>,
    partial_tick: f32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some((overlay, entries)) = hud_recipe_book_overlay_entries(
        world,
        item_runtime,
        background,
        tabs,
        pages,
        search,
        overlay,
        partial_tick,
    ) else {
        return Vec::new();
    };
    let columns = entries
        .len()
        .min(recipe_book_overlay_max_row(entries.len()));
    let rows = recipe_book_overlay_rows(entries.len());
    let mut layers = vec![hud_inventory_background_layer(
        HudInventoryBackgroundTexture::RecipeBookOverlayRecipe,
        overlay.x,
        overlay.y,
        u32::try_from(
            i32::try_from(columns).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE
                + RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER,
        )
        .unwrap_or_default(),
        u32::try_from(
            i32::try_from(rows).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE
                + RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER,
        )
        .unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    )];
    let furnace_overlay = recipe_book_furnace_family_for_background(background).is_some();
    for (index, entry) in entries.iter().enumerate() {
        let (x, y) = recipe_book_overlay_entry_position(overlay.x, overlay.y, index, entries.len());
        let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
            cursor_x >= x
                && cursor_x < x + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE
                && cursor_y >= y
                && cursor_y < y + RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE
        });
        layers.push(hud_inventory_background_layer(
            recipe_book_overlay_button_texture(furnace_overlay, entry.craftable, highlighted),
            x,
            y,
            u32::try_from(RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE).unwrap_or_default(),
            u32::try_from(RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn hud_recipe_book_overlay_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some((overlay, entries)) = hud_recipe_book_overlay_entries(
        world,
        Some(item_runtime),
        background,
        tabs,
        pages,
        search,
        overlay,
        partial_tick,
    ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let (button_x, button_y) =
            recipe_book_overlay_entry_position(overlay.x, overlay.y, index, entries.len());
        for overlay_item in &entry.items {
            let Some(icon) = hud_fake_item_icon_for_stack(
                world,
                Some(item_runtime),
                &overlay_item.stack,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
                partial_tick,
            ) else {
                continue;
            };
            items.push(HudInventoryItem {
                x: button_x + overlay_item.x,
                y: button_y + overlay_item.y,
                scale: RECIPE_BOOK_OVERLAY_ITEM_SCALE,
                scale_y: RECIPE_BOOK_OVERLAY_ITEM_SCALE,
                icon,
                scissor: None,
                draw_decorations: false,
                block_model: fake_block_item_3d_model(
                    world,
                    &overlay_item.stack,
                    Some(item_runtime),
                    terrain_textures,
                    None,
                    false,
                    false,
                    false,
                    false,
                    false,
                    keybind_context,
                    0,
                ),
            });
        }
    }
    items
}

fn hud_recipe_book_overlay_entries<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    partial_tick: f32,
) -> Option<(RecipeBookOverlayHudState, Vec<RecipeBookOverlayEntry<'a>>)> {
    let overlay = overlay?;
    if recipe_book_type_for_background(background)? != overlay.book_type {
        return None;
    }
    if recipe_book_selected_tab_index(background, tabs)? != overlay.tab_index {
        return None;
    }
    if recipe_book_selected_page_index(background, pages)? != overlay.page_index {
        return None;
    }
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page = clamped_recipe_book_page(overlay.page_index, collections.len());
    let collection_index = page * RECIPE_BOOK_ITEMS_PER_PAGE + overlay.button_index;
    let item_tag_entries = world.registry_tags("minecraft:item").map(|tags| &tags.tags);
    let entries = collections.get(collection_index)?.overlay_entries(
        item_tag_entries,
        recipe_book_slot_select_index(world, partial_tick),
    );
    (entries.len() > 1).then_some((overlay, entries))
}

fn recipe_book_overlay_button_texture(
    furnace_overlay: bool,
    craftable: bool,
    highlighted: bool,
) -> HudInventoryBackgroundTexture {
    match (furnace_overlay, craftable, highlighted) {
        (false, true, false) => HudInventoryBackgroundTexture::RecipeBookCraftingOverlay,
        (false, true, true) => HudInventoryBackgroundTexture::RecipeBookCraftingOverlayHighlighted,
        (false, false, false) => HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabled,
        (false, false, true) => {
            HudInventoryBackgroundTexture::RecipeBookCraftingOverlayDisabledHighlighted
        }
        (true, true, false) => HudInventoryBackgroundTexture::RecipeBookFurnaceOverlay,
        (true, true, true) => HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayHighlighted,
        (true, false, false) => HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayDisabled,
        (true, false, true) => {
            HudInventoryBackgroundTexture::RecipeBookFurnaceOverlayDisabledHighlighted
        }
    }
}

fn hud_recipe_book_ghost_layers_and_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    layout_slots: &[crate::input::InventorySlotLayout],
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> (Vec<HudInventoryFillLayer>, Vec<HudInventoryGhostItem>) {
    let Some(item_runtime) = item_runtime else {
        return (Vec::new(), Vec::new());
    };
    let Some(ghost_recipe) = world.last_ghost_recipe() else {
        return (Vec::new(), Vec::new());
    };
    if world.open_container_id() != Some(ghost_recipe.container_id) {
        return (Vec::new(), Vec::new());
    }
    if recipe_book_type_for_background(background).is_none() {
        return (Vec::new(), Vec::new());
    }
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    let mut fill_layers = Vec::new();
    let mut ghost_items = Vec::new();
    for ghost_slot in &ghost_recipe.slots {
        let Some(stack) = ghost_slot
            .items
            .get(slot_select_index % ghost_slot.items.len().max(1))
        else {
            continue;
        };
        let Some(layout_slot) = layout_slots
            .iter()
            .find(|layout_slot| layout_slot.slot_id == ghost_slot.slot_id)
        else {
            continue;
        };
        let Some(mut icon) = hud_fake_item_icon_for_stack(
            world,
            Some(item_runtime),
            stack,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
            partial_tick,
        ) else {
            continue;
        };
        if !ghost_slot.is_result {
            icon.count_label = None;
            icon.durability_bar = None;
            icon.cooldown_progress = None;
        }

        let (pre_fill_x, pre_fill_y, pre_fill_width, pre_fill_height) =
            if ghost_slot.is_result && recipe_book_result_slot_fill_is_big(background) {
                (layout_slot.x - 4, layout_slot.y - 4, 24, 24)
            } else {
                (layout_slot.x, layout_slot.y, 16, 16)
            };
        fill_layers.push(HudInventoryFillLayer {
            x: pre_fill_x,
            y: pre_fill_y,
            width: pre_fill_width,
            height: pre_fill_height,
            tint: RECIPE_BOOK_GHOST_PRE_ITEM_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        });
        ghost_items.push(HudInventoryGhostItem {
            x: layout_slot.x,
            y: layout_slot.y,
            icon,
            draw_decorations: ghost_slot.is_result,
        });
        fill_layers.push(HudInventoryFillLayer {
            x: layout_slot.x,
            y: layout_slot.y,
            width: 16,
            height: 16,
            tint: RECIPE_BOOK_GHOST_POST_ITEM_TINT,
            stage: HudInventoryFillStage::AfterGhostItem,
        });
    }

    (fill_layers, ghost_items)
}

fn recipe_book_result_slot_fill_is_big(background: InventoryScreenBackground) -> bool {
    matches!(background, InventoryScreenBackground::CraftingTable)
}

fn hud_recipe_book_visible_collections<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
) -> Vec<RecipeBookUiCollection<'a>> {
    let Some(collections) =
        hud_recipe_book_collections(world, item_runtime, background, tabs, search)
    else {
        return Vec::new();
    };
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    collections
        .into_iter()
        .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
        .take(RECIPE_BOOK_ITEMS_PER_PAGE)
        .collect()
}

fn hud_recipe_book_collections<'a>(
    world: &'a WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    search: &RecipeBookSearchHudState,
) -> Option<Vec<RecipeBookUiCollection<'a>>> {
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    let book_type = recipe_book_type_for_background(background)?;
    let selected_tab = recipe_book_selected_tab_index(background, tabs)?;
    let only_craftable = recipe_book_type_settings(world, book_type).filtering;
    if let Some(grid) = recipe_book_crafting_grid_for_background(background) {
        return Some(crafting_recipe_book_collections(
            world,
            grid,
            selected_tab,
            only_craftable,
            &search.text,
            item_runtime,
        ));
    }
    recipe_book_furnace_family_for_background(background).map(|family| {
        furnace_recipe_book_collections(
            world,
            family,
            selected_tab,
            only_craftable,
            &search.text,
            item_runtime,
        )
    })
}

fn recipe_book_crafting_grid_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookCraftingGrid> {
    match background {
        InventoryScreenBackground::LocalInventory => Some(RecipeBookCraftingGrid {
            width: 2,
            height: 2,
        }),
        InventoryScreenBackground::CraftingTable => Some(RecipeBookCraftingGrid {
            width: 3,
            height: 3,
        }),
        _ => None,
    }
}

fn recipe_book_furnace_family_for_background(
    background: InventoryScreenBackground,
) -> Option<RecipeBookFurnaceFamily> {
    match background {
        InventoryScreenBackground::Furnace => Some(RecipeBookFurnaceFamily::Furnace),
        InventoryScreenBackground::BlastFurnace => Some(RecipeBookFurnaceFamily::BlastFurnace),
        InventoryScreenBackground::Smoker => Some(RecipeBookFurnaceFamily::Smoker),
        _ => None,
    }
}

fn recipe_book_recipe_button_position(index: usize) -> (i32, i32) {
    (
        RECIPE_BOOK_RECIPE_BUTTON_X
            + RECIPE_BOOK_RECIPE_BUTTON_SIZE
                * i32::try_from(index % RECIPE_BOOK_RECIPE_BUTTON_COLUMNS).unwrap_or_default(),
        RECIPE_BOOK_RECIPE_BUTTON_Y
            + RECIPE_BOOK_RECIPE_BUTTON_SIZE
                * i32::try_from(index / RECIPE_BOOK_RECIPE_BUTTON_COLUMNS).unwrap_or_default(),
    )
}

fn hud_recipe_book_page_button_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    cursor_position: Option<(i32, i32)>,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some(collections) =
        hud_recipe_book_collections(world, item_runtime, background, tabs, search)
    else {
        return Vec::new();
    };
    let page_count = recipe_book_page_count(collections.len());
    if page_count <= 1 {
        return Vec::new();
    }
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    let mut layers = Vec::new();
    if page + 1 < page_count {
        layers.push(hud_recipe_book_page_button_layer(
            RecipeBookPageTurn::Next,
            cursor_position,
        ));
    }
    if page > 0 {
        layers.push(hud_recipe_book_page_button_layer(
            RecipeBookPageTurn::Previous,
            cursor_position,
        ));
    }
    layers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecipeBookPageTurn {
    Previous,
    Next,
}

fn hud_recipe_book_page_button_layer(
    turn: RecipeBookPageTurn,
    cursor_position: Option<(i32, i32)>,
) -> HudInventoryBackgroundLayer {
    let x = match turn {
        RecipeBookPageTurn::Previous => RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X,
        RecipeBookPageTurn::Next => RECIPE_BOOK_PAGE_FORWARD_BUTTON_X,
    };
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= x
            && cursor_x < x + RECIPE_BOOK_PAGE_BUTTON_WIDTH
            && cursor_y >= RECIPE_BOOK_PAGE_BUTTON_Y
            && cursor_y < RECIPE_BOOK_PAGE_BUTTON_Y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT
    });
    hud_inventory_background_layer(
        match (turn, highlighted) {
            (RecipeBookPageTurn::Previous, false) => {
                HudInventoryBackgroundTexture::RecipeBookPageBackward
            }
            (RecipeBookPageTurn::Previous, true) => {
                HudInventoryBackgroundTexture::RecipeBookPageBackwardHighlighted
            }
            (RecipeBookPageTurn::Next, false) => {
                HudInventoryBackgroundTexture::RecipeBookPageForward
            }
            (RecipeBookPageTurn::Next, true) => {
                HudInventoryBackgroundTexture::RecipeBookPageForwardHighlighted
            }
        },
        x,
        RECIPE_BOOK_PAGE_BUTTON_Y,
        u32::try_from(RECIPE_BOOK_PAGE_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_PAGE_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    )
}

fn hud_recipe_book_search_box_layer(
    world: &WorldStore,
    background: InventoryScreenBackground,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryBackgroundLayer> {
    recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    Some(hud_inventory_background_layer(
        if search.focused {
            HudInventoryBackgroundTexture::WidgetTextFieldHighlighted
        } else {
            HudInventoryBackgroundTexture::WidgetTextField
        },
        RECIPE_BOOK_SEARCH_BOX_X,
        RECIPE_BOOK_SEARCH_BOX_Y,
        u32::try_from(RECIPE_BOOK_SEARCH_BOX_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_SEARCH_BOX_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

fn hud_recipe_book_button_layer(
    background: InventoryScreenBackground,
    main_offset_x: i32,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudInventoryBackgroundLayer> {
    let (button_x, button_y) = recipe_book_button_position(background)?;
    let x = main_offset_x + button_x;
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= x
            && cursor_x < x + RECIPE_BOOK_BUTTON_WIDTH
            && cursor_y >= button_y
            && cursor_y < button_y + RECIPE_BOOK_BUTTON_HEIGHT
    });
    Some(hud_inventory_background_layer(
        if highlighted {
            HudInventoryBackgroundTexture::RecipeBookButtonHighlighted
        } else {
            HudInventoryBackgroundTexture::RecipeBookButton
        },
        x,
        button_y,
        u32::try_from(RECIPE_BOOK_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

fn hud_recipe_book_filter_button_layer(
    world: &WorldStore,
    background: InventoryScreenBackground,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudInventoryBackgroundLayer> {
    let book_type = recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0 {
        return None;
    }
    let filtering = recipe_book_type_settings(world, book_type).filtering;
    let highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= RECIPE_BOOK_FILTER_BUTTON_X
            && cursor_x < RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH
            && cursor_y >= RECIPE_BOOK_FILTER_BUTTON_Y
            && cursor_y < RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT
    });
    Some(hud_inventory_background_layer(
        hud_recipe_book_filter_texture(background, filtering, highlighted),
        RECIPE_BOOK_FILTER_BUTTON_X,
        RECIPE_BOOK_FILTER_BUTTON_Y,
        u32::try_from(RECIPE_BOOK_FILTER_BUTTON_WIDTH).unwrap_or_default(),
        u32::try_from(RECIPE_BOOK_FILTER_BUTTON_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}

fn hud_recipe_book_filter_texture(
    background: InventoryScreenBackground,
    filtering: bool,
    highlighted: bool,
) -> HudInventoryBackgroundTexture {
    let furnace_filter = matches!(
        background,
        InventoryScreenBackground::Furnace
            | InventoryScreenBackground::BlastFurnace
            | InventoryScreenBackground::Smoker
    );
    match (furnace_filter, filtering, highlighted) {
        (false, true, false) => HudInventoryBackgroundTexture::RecipeBookFilterEnabled,
        (false, false, false) => HudInventoryBackgroundTexture::RecipeBookFilterDisabled,
        (false, true, true) => HudInventoryBackgroundTexture::RecipeBookFilterEnabledHighlighted,
        (false, false, true) => HudInventoryBackgroundTexture::RecipeBookFilterDisabledHighlighted,
        (true, true, false) => HudInventoryBackgroundTexture::RecipeBookFurnaceFilterEnabled,
        (true, false, false) => HudInventoryBackgroundTexture::RecipeBookFurnaceFilterDisabled,
        (true, true, true) => {
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterEnabledHighlighted
        }
        (true, false, true) => {
            HudInventoryBackgroundTexture::RecipeBookFurnaceFilterDisabledHighlighted
        }
    }
}

fn hud_recipe_book_search_text_label(
    world: &WorldStore,
    background: InventoryScreenBackground,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryTextLabel> {
    recipe_book_type_for_background(background)?;
    if recipe_book_main_gui_offset(world, background) == 0
        || (search.text.is_empty() && !search.focused)
    {
        return None;
    }
    Some(HudInventoryTextLabel {
        x: RECIPE_BOOK_SEARCH_BOX_X + RECIPE_BOOK_SEARCH_TEXT_X_OFFSET,
        y: RECIPE_BOOK_SEARCH_BOX_Y + RECIPE_BOOK_SEARCH_TEXT_Y_OFFSET,
        width: u32::try_from(RECIPE_BOOK_SEARCH_BOX_WIDTH - 8).unwrap_or_default(),
        text: search.text.clone(),
        tint: RECIPE_BOOK_SEARCH_TEXT_COLOR,
        background: None,
        input: Some(HudInventoryTextInputDecoration {
            cursor: search.cursor,
            selection: search.selection,
            scroll_to: if search.selection != search.cursor {
                search.selection
            } else {
                search.cursor
            },
            max_length: RECIPE_BOOK_SEARCH_MAX_LENGTH,
            cursor_visible: search.focused && (wall_clock_millis() / 300) % 2 == 0,
            cursor_tint: RECIPE_BOOK_SEARCH_TEXT_COLOR,
            selection_tint: RECIPE_BOOK_SEARCH_SELECTION_TINT,
        }),
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
}

fn hud_recipe_book_page_text_label(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
) -> Option<HudInventoryTextLabel> {
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page_count = recipe_book_page_count(collections.len());
    if page_count <= 1 {
        return None;
    }
    let page = recipe_book_selected_page_index(background, pages)
        .map(|page| clamped_recipe_book_page(page, collections.len()))
        .unwrap_or_default();
    let text = format!("{}/{}", page + 1, page_count);
    let width = hud_ascii_approx_text_width(&text)?;
    let width_i32 = i32::try_from(width).ok()?;
    Some(HudInventoryTextLabel {
        x: RECIPE_BOOK_PAGE_TEXT_CENTER_X - width_i32 / 2,
        y: RECIPE_BOOK_PAGE_TEXT_Y,
        width,
        text,
        tint: RECIPE_BOOK_SEARCH_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
}

fn offset_hud_entity_previews(previews: &mut [HudEntityPreview], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for preview in previews {
        preview.rect.x += x_offset;
        if let Some(scissor) = &mut preview.scissor {
            scissor.x += x_offset;
        }
    }
}

fn offset_hud_inventory_map_previews(previews: &mut [HudInventoryMapPreview], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for preview in previews {
        preview.x += x_offset;
    }
}

fn offset_hud_inventory_background_layers(
    layers: &mut [HudInventoryBackgroundLayer],
    x_offset: i32,
) {
    if x_offset == 0 {
        return;
    }
    for layer in layers {
        layer.x += x_offset;
    }
}

fn offset_hud_inventory_items(items: &mut [HudInventoryItem], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for item in items {
        item.x += x_offset;
    }
}

fn offset_hud_inventory_text_labels(labels: &mut [HudInventoryTextLabel], x_offset: i32) {
    if x_offset == 0 {
        return;
    }
    for label in labels {
        label.x += x_offset;
        if let Some(background) = &mut label.background {
            background.x += x_offset;
        }
        if let Some(scrolling) = &mut label.scrolling {
            scrolling.x += x_offset;
        }
    }
}

fn hud_inventory_entity_previews(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
    enchantment_book: Option<EnchantmentBookHudState>,
    partial_tick: f32,
) -> Vec<HudEntityPreview> {
    match background {
        InventoryScreenBackground::LocalInventory => {
            hud_local_inventory_entity_preview(world, item_runtime, local_state, partial_tick)
                .into_iter()
                .collect()
        }
        InventoryScreenBackground::Mount { .. } => {
            hud_mount_inventory_entity_preview(world, item_runtime, local_state, partial_tick)
                .into_iter()
                .collect()
        }
        InventoryScreenBackground::Smithing => {
            vec![hud_smithing_entity_preview(world, item_runtime)]
        }
        InventoryScreenBackground::EnchantmentTable => enchantment_book
            .map(hud_enchantment_book_preview)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn hud_local_inventory_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_state: &InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudEntityPreview> {
    const X0: i32 = 26;
    const Y0: i32 = 8;
    const X1: i32 = 75;
    const Y1: i32 = 78;
    const SCALE: f32 = 30.0;
    const OFFSET_Y: f32 = 0.0625;

    let local_player_id = world.local_player_id()?;
    hud_entity_in_inventory_follows_mouse_preview(
        world,
        item_runtime,
        local_player_id,
        local_state.cursor_position,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

fn hud_mount_inventory_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_state: &InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudEntityPreview> {
    const X0: i32 = 26;
    const Y0: i32 = 18;
    const X1: i32 = 78;
    const Y1: i32 = 70;
    const SCALE: f32 = 17.0;
    const OFFSET_Y: f32 = 0.25;

    let mount_entity_id = world.inventory().open_container.as_ref()?.mount?.entity_id;
    hud_entity_in_inventory_follows_mouse_preview(
        world,
        item_runtime,
        mount_entity_id,
        local_state.cursor_position,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

fn hud_enchantment_book_preview(state: EnchantmentBookHudState) -> HudEntityPreview {
    const ENTITY_ID: i32 = -1;
    const GUI_BOOK_SCALE: f32 = 40.0;
    const GUI_BOOK_Y: f32 = 17.0;
    const WORLD_BOOK_ROOT_Z_ROTATION_DEGREES: f32 = 80.0;

    let open = state.open.clamp(0.0, 1.0);
    let page_flip_1 = ((fraction(state.flip + 0.25) * 1.6) - 0.3).clamp(0.0, 1.0);
    let page_flip_2 = ((fraction(state.flip + 0.75) * 1.6) - 0.3).clamp(0.0, 1.0);
    let mut entity = EntityModelInstance::enchanting_book(ENTITY_ID, [-0.5, -0.75, -0.5], 0.0)
        .with_book_progress(0.0)
        .with_book_open(open)
        .with_book_page_flip_1(page_flip_1)
        .with_book_page_flip_2(page_flip_2)
        .with_book_float_y(0.0);
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;

    // `EntityModelKind::EnchantingBook` carries the in-world `Rz(80)` root.
    // Cancel that final rotation and fold `GuiBookModelRenderer`'s exact rigid
    // pose into the existing entity-PIP camera.
    let gui_base =
        Quat::from_rotation_y(std::f32::consts::PI) * Quat::from_rotation_x(25.0_f32.to_radians());
    let closed = 1.0 - open;
    let gui_translation = gui_base * Vec3::new(closed * 0.2, closed * 0.1, closed * 0.25);
    let rotation = gui_base
        * Quat::from_rotation_y((-(closed * 90.0) - 90.0).to_radians())
        * Quat::from_rotation_x(std::f32::consts::PI)
        * Quat::from_rotation_z(-WORLD_BOOK_ROOT_Z_ROTATION_DEGREES.to_radians());

    HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: 14,
            y: 14,
            width: 38,
            height: 31,
        },
        scissor: None,
        translation: (gui_translation
            + Vec3::new(0.0, (GUI_BOOK_Y - 31.0 / 2.0) / GUI_BOOK_SCALE, 0.0))
        .to_array(),
        rotation: rotation.to_array(),
        override_camera_rotation: None,
        scale: GUI_BOOK_SCALE,
        depth_isolated: true,
        after_background_layer: Some(0),
        item_layers: Vec::new(),
    }
}

fn fraction(value: f32) -> f32 {
    value - value.floor()
}

fn hud_entity_in_inventory_follows_mouse_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    cursor_position: Option<(i32, i32)>,
    partial_tick: f32,
    rect: [i32; 4],
    scale: f32,
    offset_y: f32,
) -> Option<HudEntityPreview> {
    const MOUSE_FOLLOW_DIVISOR: f32 = 40.0;
    const ROTATION_SCALE_DEGREES: f32 = 20.0;

    let [x0, y0, x1, y1] = rect;
    let mut entity = entity_model_instance_from_world_entity_at_partial_tick(
        world,
        item_runtime,
        entity_id,
        partial_tick,
    )?;
    let bounds = world.probe_entity_pick_bounds(entity_id)?;
    let height = bounds.max[1] - bounds.min[1];
    let center_x = (x0 + x1) as f32 / 2.0;
    let center_y = (y0 + y1) as f32 / 2.0;
    let (mouse_x, mouse_y) = cursor_position
        .map(|(x, y)| (x as f32, y as f32))
        .unwrap_or((center_x, center_y));
    let x_angle = ((center_x - mouse_x) / MOUSE_FOLLOW_DIVISOR).atan();
    let y_angle = ((center_y - mouse_y) / MOUSE_FOLLOW_DIVISOR).atan();
    let yaw_degrees = x_angle * ROTATION_SCALE_DEGREES;
    let pitch_degrees = y_angle * ROTATION_SCALE_DEGREES;
    let camera_x_rotation = quaternion_x(pitch_degrees.to_radians());

    entity.render_state.body_rot = 180.0 + yaw_degrees;
    entity.render_state.head_yaw = yaw_degrees;
    entity.render_state.head_pitch = -pitch_degrees;
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;

    Some(HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: x0,
            y: y0,
            width: u32::try_from(x1 - x0).ok()?,
            height: u32::try_from(y1 - y0).ok()?,
        },
        scissor: None,
        translation: [0.0, height / 2.0 + offset_y, 0.0],
        rotation: quaternion_mul([0.0, 0.0, 1.0, 0.0], camera_x_rotation),
        override_camera_rotation: Some(camera_x_rotation),
        scale,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    })
}

fn hud_smithing_entity_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> HudEntityPreview {
    const ENTITY_ID: i32 = -1;
    const X0: i32 = 121;
    const Y0: i32 = 20;
    const X1: i32 = 161;
    const Y1: i32 = 80;
    const SCALE: f32 = 25.0;
    const X_ROT_DEGREES: f32 = 25.0;
    const BODY_ROT_DEGREES: f32 = 210.0;
    const ARMOR_STAND_X_ROT_RADIANS: f32 = 0.43633232;

    let mut entity = EntityModelInstance::armor_stand(
        ENTITY_ID,
        [0.0, 0.0, 0.0],
        BODY_ROT_DEGREES,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_head_look(0.0, X_ROT_DEGREES);
    entity.render_state.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;
    let item_layers = apply_smithing_result_equipment(&mut entity, world, item_runtime);

    HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: X0,
            y: Y0,
            width: u32::try_from(X1 - X0).unwrap_or_default(),
            height: u32::try_from(Y1 - Y0).unwrap_or_default(),
        },
        scissor: None,
        translation: [0.0, 1.0, 0.0],
        rotation: quaternion_mul(
            quaternion_x(ARMOR_STAND_X_ROT_RADIANS),
            quaternion_z(std::f32::consts::PI),
        ),
        override_camera_rotation: None,
        scale: SCALE,
        depth_isolated: true,
        after_background_layer: None,
        item_layers,
    }
}

fn apply_smithing_result_equipment(
    entity: &mut EntityModelInstance,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudEntityPreviewItemLayer> {
    const SMITHING_RESULT_SLOT: i16 = 3;
    const ARMOR_STAND_ITEM_IN_HAND_LAYER_SEQUENCE: u32 = 1;
    const ARMOR_STAND_CUSTOM_HEAD_LAYER_SEQUENCE: u32 = 2;

    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some(stack) = open_container_slot_item(world, SMITHING_RESULT_SLOT) else {
        return Vec::new();
    };
    if item_stack_is_empty(stack) {
        return Vec::new();
    }
    let Some(item_id) = stack.item_id else {
        return Vec::new();
    };

    let equipment_asset_present = item_runtime.item_equipment_asset_id(item_id).is_some();
    match item_runtime.item_equipment_slot(item_id) {
        Some(ItemEquipmentSlot::Head) if equipment_asset_present => {
            entity.render_state.head_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.head_armor_foil = stack.has_foil();
        }
        Some(ItemEquipmentSlot::Head) => {
            if smithing_result_is_custom_head_skull(item_runtime, item_id) {
                entity.render_state.custom_head_skull =
                    item_runtime.custom_head_skull_for_stack(stack);
            } else {
                return vec![smithing_preview_item_layer(
                    HudEntityPreviewItemSlot::Head,
                    HudEntityPreviewItemDisplayContext::Head,
                    stack,
                    ARMOR_STAND_CUSTOM_HEAD_LAYER_SEQUENCE,
                )];
            }
        }
        Some(ItemEquipmentSlot::Chest) => {
            entity.render_state.chest_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.chest_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(ItemEquipmentSlot::Legs) => {
            entity.render_state.legs_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.legs_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(ItemEquipmentSlot::Feet) => {
            entity.render_state.feet_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.feet_armor_foil = equipment_asset_present && stack.has_foil();
        }
        Some(
            ItemEquipmentSlot::MainHand
            | ItemEquipmentSlot::OffHand
            | ItemEquipmentSlot::Body
            | ItemEquipmentSlot::Saddle,
        )
        | None => {
            return vec![smithing_preview_item_layer(
                HudEntityPreviewItemSlot::LeftHand,
                HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
                stack,
                ARMOR_STAND_ITEM_IN_HAND_LAYER_SEQUENCE,
            )];
        }
    }
    Vec::new()
}

fn smithing_preview_item_layer(
    slot: HudEntityPreviewItemSlot,
    display_context: HudEntityPreviewItemDisplayContext,
    stack: &ItemStackSummary,
    submit_sequence: u32,
) -> HudEntityPreviewItemLayer {
    HudEntityPreviewItemLayer {
        slot,
        display_context,
        item_id: stack.item_id.unwrap_or_default(),
        count: stack.count,
        foil: stack.has_foil(),
        light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
        overlay: ITEM_MODEL_NO_OVERLAY,
        order: 0,
        submit_sequence,
    }
}

fn smithing_result_is_custom_head_skull(item_runtime: &NativeItemRuntime, item_id: i32) -> bool {
    item_runtime
        .item_resource_id(item_id)
        .is_some_and(|resource_id| {
            matches!(
                resource_id,
                "minecraft:skeleton_skull"
                    | "minecraft:wither_skeleton_skull"
                    | "minecraft:zombie_head"
                    | "minecraft:creeper_head"
                    | "minecraft:piglin_head"
                    | "minecraft:dragon_head"
                    | "minecraft:player_head"
            )
        })
}

fn quaternion_x(angle_radians: f32) -> [f32; 4] {
    let half = angle_radians / 2.0;
    [half.sin(), 0.0, 0.0, half.cos()]
}

fn quaternion_z(angle_radians: f32) -> [f32; 4] {
    let half = angle_radians / 2.0;
    [0.0, 0.0, half.sin(), half.cos()]
}

fn quaternion_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn hud_advancements_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    advancement_scroll_delta: Option<(f64, f64)>,
    advancement_hover_fade: f32,
    surface_size: winit::dpi::PhysicalSize<u32>,
    cursor_position: Option<(i32, i32)>,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> HudInventoryScreen {
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (window_x, window_y) = advancements_window_origin_for_surface(surface_size);
    let (done_button_x, done_button_y) = advancements_done_button_origin_for_surface(surface_size);
    let root_tabs = world.advancement_root_tabs();
    let selected_tab = world.selected_advancement_root_tab();
    let selected_widgets = world.selected_advancement_widgets();
    let selected_tab_id = selected_tab.as_ref().map(|tab| tab.id.as_str());
    let show_root_tabs = root_tabs.len() > 1;
    let done_button_highlighted = cursor_position.is_some_and(|(cursor_x, cursor_y)| {
        cursor_x >= done_button_x
            && cursor_x < done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32
            && cursor_y >= done_button_y
            && cursor_y < done_button_y + ADVANCEMENTS_DONE_BUTTON_HEIGHT as i32
    });
    let done_button_texture = if done_button_highlighted {
        HudInventoryBackgroundTexture::WidgetButtonHighlighted
    } else {
        HudInventoryBackgroundTexture::WidgetButton
    };
    let mut background_layers = vec![hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementsWindow,
        window_x,
        window_y,
        ADVANCEMENTS_WINDOW_WIDTH,
        ADVANCEMENTS_WINDOW_HEIGHT,
        [0.0, 0.0],
        [
            ADVANCEMENTS_WINDOW_WIDTH as f32 / 256.0,
            ADVANCEMENTS_WINDOW_HEIGHT as f32 / 256.0,
        ],
    )];
    background_layers.extend(advancements_background_layers(
        selected_tab.as_ref(),
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.extend(advancements_widget_connection_layers(
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.extend(advancements_widget_frame_layers(
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
    ));
    background_layers.push(hud_inventory_background_layer(
        done_button_texture,
        done_button_x,
        done_button_y,
        ADVANCEMENTS_DONE_BUTTON_WIDTH,
        ADVANCEMENTS_DONE_BUTTON_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    if show_root_tabs {
        background_layers.extend(advancements_tab_background_layers(
            &root_tabs,
            selected_tab_id,
            window_x,
            window_y,
        ));
    }
    let mut floating_items = Vec::new();
    if show_root_tabs {
        floating_items.extend(advancements_tab_icon_items(
            world,
            item_runtime,
            terrain_textures,
            &root_tabs,
            window_x,
            window_y,
            keybind_context,
            partial_tick,
        ));
    }
    floating_items.extend(advancements_widget_icon_items(
        world,
        item_runtime,
        terrain_textures,
        &selected_widgets,
        advancement_scroll_delta,
        window_x,
        window_y,
        keybind_context,
        partial_tick,
    ));
    let hover = advancements_hover_projection(
        world,
        item_runtime,
        terrain_textures,
        &selected_widgets,
        advancement_scroll_delta,
        advancement_hover_fade,
        window_x,
        window_y,
        cursor_position,
        screen_width,
        keybind_context,
        partial_tick,
    );
    let mut fill_layers = if selected_tab.is_some() {
        Vec::new()
    } else {
        vec![HudInventoryFillLayer {
            x: window_x + ADVANCEMENTS_WINDOW_INSIDE_X,
            y: window_y + ADVANCEMENTS_WINDOW_INSIDE_Y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: ADVANCEMENTS_EMPTY_BACKGROUND_TINT,
            stage: HudInventoryFillStage::BeforeGhostItem,
        }]
    };
    fill_layers.extend(hover.fill_layers);
    let mut text_labels = advancements_screen_text_labels(
        selected_tab.as_ref().map(|tab| &tab.title),
        window_x,
        window_y,
        done_button_x,
        done_button_y,
    );
    text_labels.extend(hover.text_labels);
    HudInventoryScreen {
        width: screen_width,
        height: screen_height,
        background_layers,
        foreground_layers: hover.background_layers,
        fill_layers,
        slots: Vec::new(),
        floating_items,
        foreground_items: hover.foreground_items,
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdvancementTabType {
    Above,
    Below,
    Left,
    Right,
}

impl AdvancementTabType {
    fn for_display_index(mut display_index: usize) -> Option<(Self, usize)> {
        if display_index < ADVANCEMENTS_TAB_ABOVE_MAX {
            return Some((Self::Above, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_ABOVE_MAX;
        if display_index < ADVANCEMENTS_TAB_BELOW_MAX {
            return Some((Self::Below, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_BELOW_MAX;
        if display_index < ADVANCEMENTS_TAB_LEFT_MAX {
            return Some((Self::Left, display_index));
        }
        display_index -= ADVANCEMENTS_TAB_LEFT_MAX;
        (display_index < ADVANCEMENTS_TAB_RIGHT_MAX).then_some((Self::Right, display_index))
    }

    fn max(self) -> usize {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_MAX,
            Self::Below => ADVANCEMENTS_TAB_BELOW_MAX,
            Self::Left => ADVANCEMENTS_TAB_LEFT_MAX,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_MAX,
        }
    }

    fn width(self) -> u32 {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_WIDTH,
            Self::Below => ADVANCEMENTS_TAB_BELOW_WIDTH,
            Self::Left => ADVANCEMENTS_TAB_LEFT_WIDTH,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_WIDTH,
        }
    }

    fn height(self) -> u32 {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_HEIGHT,
            Self::Below => ADVANCEMENTS_TAB_BELOW_HEIGHT,
            Self::Left => ADVANCEMENTS_TAB_LEFT_HEIGHT,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_HEIGHT,
        }
    }

    fn origin(self, index: usize) -> (i32, i32) {
        match self {
            Self::Above => (
                ((self.width() + 4) * index as u32) as i32,
                -(self.height() as i32) + 4,
            ),
            Self::Below => (((self.width() + 4) * index as u32) as i32, 136),
            Self::Left => (
                -(self.width() as i32) + 4,
                (self.height() * index as u32) as i32,
            ),
            Self::Right => (248, (self.height() * index as u32) as i32),
        }
    }

    fn icon_offset(self) -> (i32, i32) {
        match self {
            Self::Above => ADVANCEMENTS_TAB_ABOVE_ICON_OFFSET,
            Self::Below => ADVANCEMENTS_TAB_BELOW_ICON_OFFSET,
            Self::Left => ADVANCEMENTS_TAB_LEFT_ICON_OFFSET,
            Self::Right => ADVANCEMENTS_TAB_RIGHT_ICON_OFFSET,
        }
    }

    fn sprite(self, index: usize, selected: bool) -> HudAdvancementTabSprite {
        let first = index == 0;
        let last = index == self.max() - 1;
        match (self, selected, first, last) {
            (Self::Above, true, true, _) => HudAdvancementTabSprite::AboveLeftSelected,
            (Self::Above, true, _, true) => HudAdvancementTabSprite::AboveRightSelected,
            (Self::Above, true, _, _) => HudAdvancementTabSprite::AboveMiddleSelected,
            (Self::Above, false, true, _) => HudAdvancementTabSprite::AboveLeft,
            (Self::Above, false, _, true) => HudAdvancementTabSprite::AboveRight,
            (Self::Above, false, _, _) => HudAdvancementTabSprite::AboveMiddle,
            (Self::Below, true, true, _) => HudAdvancementTabSprite::BelowLeftSelected,
            (Self::Below, true, _, true) => HudAdvancementTabSprite::BelowRightSelected,
            (Self::Below, true, _, _) => HudAdvancementTabSprite::BelowMiddleSelected,
            (Self::Below, false, true, _) => HudAdvancementTabSprite::BelowLeft,
            (Self::Below, false, _, true) => HudAdvancementTabSprite::BelowRight,
            (Self::Below, false, _, _) => HudAdvancementTabSprite::BelowMiddle,
            (Self::Left, true, true, _) => HudAdvancementTabSprite::LeftTopSelected,
            (Self::Left, true, _, true) => HudAdvancementTabSprite::LeftBottomSelected,
            (Self::Left, true, _, _) => HudAdvancementTabSprite::LeftMiddleSelected,
            (Self::Left, false, true, _) => HudAdvancementTabSprite::LeftTop,
            (Self::Left, false, _, true) => HudAdvancementTabSprite::LeftBottom,
            (Self::Left, false, _, _) => HudAdvancementTabSprite::LeftMiddle,
            (Self::Right, true, true, _) => HudAdvancementTabSprite::RightTopSelected,
            (Self::Right, true, _, true) => HudAdvancementTabSprite::RightBottomSelected,
            (Self::Right, true, _, _) => HudAdvancementTabSprite::RightMiddleSelected,
            (Self::Right, false, true, _) => HudAdvancementTabSprite::RightTop,
            (Self::Right, false, _, true) => HudAdvancementTabSprite::RightBottom,
            (Self::Right, false, _, _) => HudAdvancementTabSprite::RightMiddle,
        }
    }
}

fn advancement_tab_geometry(
    display_index: usize,
    window_x: i32,
    window_y: i32,
) -> Option<(AdvancementTabType, usize, i32, i32, u32, u32)> {
    let (tab_type, tab_index) = AdvancementTabType::for_display_index(display_index)?;
    let (x, y) = tab_type.origin(tab_index);
    Some((
        tab_type,
        tab_index,
        window_x + x,
        window_y + y,
        tab_type.width(),
        tab_type.height(),
    ))
}

fn advancements_tab_background_layers(
    root_tabs: &[bbb_world::AdvancementRootTabSummary],
    selected_tab_id: Option<&str>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::new();
    for tab in root_tabs {
        let Some((tab_type, tab_index, x, y, width, height)) =
            advancement_tab_geometry(tab.display_index, window_x, window_y)
        else {
            continue;
        };
        let selected = selected_tab_id == Some(tab.id.as_str());
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementTab(tab_type.sprite(tab_index, selected)),
            x,
            y,
            width,
            height,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn advancements_background_layers(
    selected_tab: Option<&bbb_world::AdvancementRootTabSummary>,
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some(selected_tab) = selected_tab else {
        return Vec::new();
    };
    let texture = selected_tab
        .background
        .as_deref()
        .and_then(HudAdvancementBackgroundTexture::from_resource_id)
        .unwrap_or(HudAdvancementBackgroundTexture::Missing);
    let (scroll_x, scroll_y) = advancements_widget_scroll(widgets, scroll_delta).unwrap_or((0, 0));
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let left = scroll_x % ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let top = scroll_y % ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let mut layers = Vec::new();
    for tile_x in -1..=15 {
        for tile_y in -1..=8 {
            let x = inside_x + left + ADVANCEMENTS_BACKGROUND_TILE_SIZE * tile_x;
            let y = inside_y + top + ADVANCEMENTS_BACKGROUND_TILE_SIZE * tile_y;
            if let Some(layer) =
                clipped_advancement_background_tile(texture, x, y, inside_x, inside_y)
            {
                layers.push(layer);
            }
        }
    }
    layers
}

fn clipped_advancement_background_tile(
    texture: HudAdvancementBackgroundTexture,
    tile_x: i32,
    tile_y: i32,
    inside_x: i32,
    inside_y: i32,
) -> Option<HudInventoryBackgroundLayer> {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let tile_right = tile_x + ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let tile_bottom = tile_y + ADVANCEMENTS_BACKGROUND_TILE_SIZE;
    let x = tile_x.max(inside_x);
    let y = tile_y.max(inside_y);
    let right = tile_right.min(inside_right);
    let bottom = tile_bottom.min(inside_bottom);
    if x >= right || y >= bottom {
        return None;
    }

    let uv_min = [
        (x - tile_x) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
        (y - tile_y) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
    ];
    let uv_max = [
        (right - tile_x) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
        (bottom - tile_y) as f32 / ADVANCEMENTS_BACKGROUND_TILE_SIZE as f32,
    ];
    Some(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementBackground(texture),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        uv_min,
        uv_max,
    ))
}

fn advancements_widget_connection_layers(
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return Vec::new();
    };
    let by_id: BTreeMap<&str, &bbb_world::AdvancementWidgetSummary> = widgets
        .iter()
        .map(|widget| (widget.id.as_str(), widget))
        .collect();
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let mut layers = Vec::new();
    for widget in widgets {
        let Some(parent_id) = widget.parent_id.as_deref() else {
            continue;
        };
        let Some(parent) = by_id.get(parent_id).copied() else {
            continue;
        };
        let dep_x = inside_x + scroll_x + parent.x + 13;
        let split_x = inside_x + scroll_x + parent.x + 26 + 4;
        let dep_y = inside_y + scroll_y + parent.y + 13;
        let my_x = inside_x + scroll_x + widget.x + 13;
        let my_y = inside_y + scroll_y + widget.y + 13;
        push_advancement_connection_layers(
            &mut layers,
            HudAdvancementLineTexture::Background,
            dep_x,
            split_x,
            dep_y,
            my_x,
            my_y,
            inside_x,
            inside_y,
        );
        push_advancement_connection_layers(
            &mut layers,
            HudAdvancementLineTexture::Foreground,
            dep_x,
            split_x,
            dep_y,
            my_x,
            my_y,
            inside_x,
            inside_y,
        );
    }
    layers
}

#[allow(clippy::too_many_arguments)]
fn push_advancement_connection_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    dep_x: i32,
    split_x: i32,
    dep_y: i32,
    my_x: i32,
    my_y: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if matches!(texture, HudAdvancementLineTexture::Background) {
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x,
            dep_x,
            dep_y - 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x + 1,
            dep_x,
            dep_y,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            split_x,
            dep_x,
            dep_y + 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y - 1,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y,
            inside_x,
            inside_y,
        );
        push_advancement_horizontal_line(
            layers,
            texture,
            my_x,
            split_x - 1,
            my_y + 1,
            inside_x,
            inside_y,
        );
        push_advancement_vertical_line(
            layers,
            texture,
            split_x - 1,
            my_y,
            dep_y,
            inside_x,
            inside_y,
        );
        push_advancement_vertical_line(
            layers,
            texture,
            split_x + 1,
            my_y,
            dep_y,
            inside_x,
            inside_y,
        );
    } else {
        push_advancement_horizontal_line(
            layers, texture, split_x, dep_x, dep_y, inside_x, inside_y,
        );
        push_advancement_horizontal_line(layers, texture, my_x, split_x, my_y, inside_x, inside_y);
        push_advancement_vertical_line(layers, texture, split_x, my_y, dep_y, inside_x, inside_y);
    }
}

fn push_advancement_horizontal_line(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    mut x0: i32,
    mut x1: i32,
    y: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if x1 < x0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    push_clipped_advancement_line_layer(layers, texture, x0, y, x1 + 1, y + 1, inside_x, inside_y);
}

fn push_advancement_vertical_line(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    x: i32,
    mut y0: i32,
    mut y1: i32,
    inside_x: i32,
    inside_y: i32,
) {
    if y1 < y0 {
        std::mem::swap(&mut y0, &mut y1);
    }
    push_clipped_advancement_line_layer(layers, texture, x, y0 + 1, x + 1, y1, inside_x, inside_y);
}

#[allow(clippy::too_many_arguments)]
fn push_clipped_advancement_line_layer(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    texture: HudAdvancementLineTexture,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    inside_x: i32,
    inside_y: i32,
) {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let x = x0.max(inside_x);
    let y = y0.max(inside_y);
    let right = x1.min(inside_right);
    let bottom = y1.min(inside_bottom);
    if x >= right || y >= bottom {
        return;
    }
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementLine(texture),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn advancements_tab_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    root_tabs: &[bbb_world::AdvancementRootTabSummary],
    window_x: i32,
    window_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for tab in root_tabs {
        let Some((tab_type, _, x, y, _, _)) =
            advancement_tab_geometry(tab.display_index, window_x, window_y)
        else {
            continue;
        };
        let stack = ItemStackSummary {
            item_id: Some(tab.icon.item_id),
            count: tab.icon.count,
            component_patch: tab.icon.component_patch.clone(),
        };
        let Some(icon) = hud_fake_item_icon_for_stack(
            world,
            Some(item_runtime),
            &stack,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
            partial_tick,
        ) else {
            continue;
        };
        let (icon_x, icon_y) = tab_type.icon_offset();
        items.push(HudInventoryItem {
            x: x + icon_x,
            y: y + icon_y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: false,
            block_model: fake_block_item_3d_model(
                world,
                &stack,
                Some(item_runtime),
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
            ),
        });
    }
    items
}

fn advancements_widget_frame_layers(
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::new();
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return layers;
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    for widget in widgets
        .iter()
        .filter(|widget| advancement_widget_visible(widget))
    {
        let frame_x = inside_x + scroll_x + widget.x + ADVANCEMENTS_WIDGET_FRAME_OFFSET_X;
        let frame_y = inside_y + scroll_y + widget.y;
        if let Some(layer) = clipped_advancement_widget_frame_layer(
            advancement_widget_frame_sprite(widget.frame_type, widget.done),
            frame_x,
            frame_y,
            inside_x,
            inside_y,
        ) {
            layers.push(layer);
        }
    }
    layers
}

fn clipped_advancement_widget_frame_layer(
    sprite: HudAdvancementWidgetFrameSprite,
    frame_x: i32,
    frame_y: i32,
    inside_x: i32,
    inside_y: i32,
) -> Option<HudInventoryBackgroundLayer> {
    let inside_right = inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32;
    let inside_bottom = inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let frame_right = frame_x + ADVANCEMENTS_WIDGET_FRAME_WIDTH as i32;
    let frame_bottom = frame_y + ADVANCEMENTS_WIDGET_FRAME_HEIGHT as i32;
    let x = frame_x.max(inside_x);
    let y = frame_y.max(inside_y);
    let right = frame_right.min(inside_right);
    let bottom = frame_bottom.min(inside_bottom);
    if x >= right || y >= bottom {
        return None;
    }
    let uv_min = [
        (x - frame_x) as f32 / ADVANCEMENTS_WIDGET_FRAME_WIDTH as f32,
        (y - frame_y) as f32 / ADVANCEMENTS_WIDGET_FRAME_HEIGHT as f32,
    ];
    let uv_max = [
        (right - frame_x) as f32 / ADVANCEMENTS_WIDGET_FRAME_WIDTH as f32,
        (bottom - frame_y) as f32 / ADVANCEMENTS_WIDGET_FRAME_HEIGHT as f32,
    ];
    Some(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementWidgetFrame(sprite),
        x,
        y,
        (right - x) as u32,
        (bottom - y) as u32,
        uv_min,
        uv_max,
    ))
}

fn advancements_widget_icon_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    window_x: i32,
    window_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return Vec::new();
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let mut items = Vec::new();
    for widget in widgets
        .iter()
        .filter(|widget| advancement_widget_visible(widget))
    {
        let (icon_x, icon_y) = ADVANCEMENTS_WIDGET_ICON_OFFSET;
        let x = inside_x + scroll_x + widget.x + icon_x;
        let y = inside_y + scroll_y + widget.y + icon_y;
        if !advancement_widget_icon_intersects_content(x, y, inside_x, inside_y) {
            continue;
        }
        let stack = ItemStackSummary {
            item_id: Some(widget.icon.item_id),
            count: widget.icon.count,
            component_patch: widget.icon.component_patch.clone(),
        };
        let Some(icon) = hud_fake_item_icon_for_stack(
            world,
            Some(item_runtime),
            &stack,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
            partial_tick,
        ) else {
            continue;
        };
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: Some(advancement_widget_content_scissor(window_x, window_y)),
            draw_decorations: false,
            block_model: fake_block_item_3d_model(
                world,
                &stack,
                Some(item_runtime),
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
            ),
        });
    }
    items
}

fn advancement_widget_content_scissor(window_x: i32, window_y: i32) -> HudInventoryItemScissor {
    HudInventoryItemScissor {
        x: ADVANCEMENTS_WINDOW_INSIDE_X + window_x,
        y: ADVANCEMENTS_WINDOW_INSIDE_Y + window_y,
        width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
        height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
    }
}

fn advancement_widget_icon_intersects_content(
    x: i32,
    y: i32,
    inside_x: i32,
    inside_y: i32,
) -> bool {
    let right = x + ADVANCEMENTS_WIDGET_ICON_SIZE;
    let bottom = y + ADVANCEMENTS_WIDGET_ICON_SIZE;
    right > inside_x
        && x < inside_x + ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
        && bottom > inside_y
        && y < inside_y + ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
}

#[derive(Debug, Default)]
struct AdvancementHoverProjection {
    fill_layers: Vec<HudInventoryFillLayer>,
    background_layers: Vec<HudInventoryBackgroundLayer>,
    text_labels: Vec<HudInventoryTextLabel>,
    foreground_items: Vec<HudInventoryItem>,
}

#[allow(clippy::too_many_arguments)]
fn advancements_hover_projection(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
    hover_fade: f32,
    window_x: i32,
    window_y: i32,
    cursor_position: Option<(i32, i32)>,
    screen_width: u32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> AdvancementHoverProjection {
    let mut projection = AdvancementHoverProjection::default();
    let Some((scroll_x, scroll_y)) = advancements_widget_scroll(widgets, scroll_delta) else {
        return projection;
    };
    let inside_x = window_x + ADVANCEMENTS_WINDOW_INSIDE_X;
    let inside_y = window_y + ADVANCEMENTS_WINDOW_INSIDE_Y;
    let hover_fade_tint = advancement_hover_fade_tint(hover_fade);
    if hover_fade_tint[3] > 0.0 {
        projection.fill_layers.push(HudInventoryFillLayer {
            x: inside_x,
            y: inside_y,
            width: ADVANCEMENTS_WINDOW_INSIDE_WIDTH,
            height: ADVANCEMENTS_WINDOW_INSIDE_HEIGHT,
            tint: hover_fade_tint,
            stage: HudInventoryFillStage::Foreground,
        });
    }
    let Some(widget) = advancement_hovered_widget(
        widgets,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        cursor_position,
    ) else {
        return projection;
    };
    push_advancement_hover_boxes_and_text(
        &mut projection.background_layers,
        &mut projection.text_labels,
        widget,
        scroll_x,
        scroll_y,
        window_x,
        inside_x,
        inside_y,
        screen_width,
    );
    if let Some(item) = advancement_hover_icon_item(
        world,
        item_runtime,
        terrain_textures,
        widget,
        scroll_x,
        scroll_y,
        inside_x,
        inside_y,
        keybind_context,
        partial_tick,
    ) {
        projection.foreground_items.push(item);
    }
    projection
}

fn advancement_hover_fade_tint(hover_fade: f32) -> [f32; 4] {
    let alpha = if hover_fade.is_finite() {
        hover_fade.clamp(0.0, ADVANCEMENTS_HOVER_MAX_FADE)
    } else {
        0.0
    };
    let mut tint = ADVANCEMENTS_HOVER_FADE_TINT;
    tint[3] = alpha;
    tint
}

fn advancement_hovered_widget(
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_x: i32,
    scroll_y: i32,
    inside_x: i32,
    inside_y: i32,
    cursor_position: Option<(i32, i32)>,
) -> Option<&bbb_world::AdvancementWidgetSummary> {
    let (cursor_x, cursor_y) = cursor_position?;
    let mouse_x = cursor_x - inside_x;
    let mouse_y = cursor_y - inside_y;
    if mouse_x <= 0
        || mouse_x >= ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32
        || mouse_y <= 0
        || mouse_y >= ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32
    {
        return None;
    }
    widgets.iter().find(|widget| {
        if !advancement_widget_visible(widget) {
            return false;
        }
        let x0 = scroll_x + widget.x;
        let y0 = scroll_y + widget.y;
        mouse_x >= x0
            && mouse_x <= x0 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            && mouse_y >= y0
            && mouse_y <= y0 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
    })
}

#[allow(clippy::too_many_arguments)]
fn push_advancement_hover_boxes_and_text(
    background_layers: &mut Vec<HudInventoryBackgroundLayer>,
    text_labels: &mut Vec<HudInventoryTextLabel>,
    widget: &bbb_world::AdvancementWidgetSummary,
    scroll_x: i32,
    scroll_y: i32,
    window_x: i32,
    inside_x: i32,
    inside_y: i32,
    screen_width: u32,
) {
    let title_lines = advancement_title_lines(&widget.title);
    let progress_text = advancement_progress_text(widget);
    let progress_width = progress_text
        .as_deref()
        .and_then(hud_ascii_approx_text_width)
        .unwrap_or_default();
    let title_width = title_lines
        .iter()
        .filter_map(|line| hud_ascii_approx_text_width(&line.text))
        .max()
        .unwrap_or_default()
        .max(ADVANCEMENTS_HOVER_TITLE_MIN_WIDTH);
    let max_progress_width = advancement_max_progress_width(widget);
    let preferred_description_width = ADVANCEMENTS_HOVER_PROGRESS_PREFIX_WIDTH
        .saturating_add(title_width)
        .saturating_add(max_progress_width);
    let description_lines =
        advancement_wrap_text_lines(&widget.description, preferred_description_width);
    let tooltip_width = description_lines
        .iter()
        .filter_map(|line| hud_ascii_approx_text_width(&line.text))
        .fold(preferred_description_width, u32::max)
        .saturating_add(
            ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT + ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT,
        );
    let Ok(tooltip_width_i32) = i32::try_from(tooltip_width) else {
        return;
    };
    let title_bar_height = i32::try_from(title_lines.len())
        .unwrap_or_default()
        .saturating_mul(ADVANCEMENTS_HOVER_LINE_HEIGHT)
        .saturating_add(ADVANCEMENTS_HOVER_TITLE_PADDING_TOP)
        .saturating_add(ADVANCEMENTS_HOVER_TITLE_PADDING_BOTTOM);
    let description_text_height = i32::try_from(description_lines.len())
        .unwrap_or_default()
        .saturating_mul(ADVANCEMENTS_HOVER_LINE_HEIGHT);
    let description_height = ADVANCEMENTS_HOVER_DESCRIPTION_PADDING_TOP + description_text_height;
    let title_top =
        scroll_y + widget.y + (ADVANCEMENTS_HOVER_FRAME_SIZE as i32 - title_bar_height) / 2;
    let title_bar_bottom = title_top + title_bar_height;
    let left_side =
        window_x + scroll_x + widget.x + tooltip_width_i32 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            >= i32::try_from(screen_width).unwrap_or(i32::MAX);
    let top_side =
        title_bar_bottom + description_height >= ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let title_left = if left_side {
        scroll_x + widget.x - tooltip_width_i32
            + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            + ADVANCEMENTS_HOVER_RIGHT_MARGIN
    } else {
        scroll_x + widget.x
    };
    let title_bar_height_u32 = u32::try_from(title_bar_height.max(1)).unwrap_or_default();

    if !description_lines.is_empty() {
        let background_height = title_bar_height + description_height;
        let background_y = if top_side {
            title_bar_bottom - background_height
        } else {
            title_top
        };
        background_layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementHoverBox(HudAdvancementHoverBoxSprite::Title),
            inside_x + title_left,
            inside_y + background_y,
            tooltip_width,
            u32::try_from(background_height.max(1)).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let (first_box, second_box, first_half_width, frame_obtained) =
        advancement_hover_progress_box_state(widget, tooltip_width);
    let second_half_width = tooltip_width.saturating_sub(first_half_width);
    if first_box == second_box {
        background_layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::AdvancementHoverBox(first_box),
            inside_x + title_left,
            inside_y + title_top,
            tooltip_width,
            title_bar_height_u32,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    } else {
        push_advancement_hover_progress_box_segment(
            background_layers,
            first_box,
            inside_x + title_left,
            inside_y + title_top,
            first_half_width,
            title_bar_height_u32,
            0,
            first_half_width,
        );
        push_advancement_hover_progress_box_segment(
            background_layers,
            second_box,
            inside_x + title_left + i32::try_from(first_half_width).unwrap_or_default(),
            inside_y + title_top,
            second_half_width,
            title_bar_height_u32,
            ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH.saturating_sub(second_half_width),
            ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH,
        );
    }
    background_layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementWidgetFrame(advancement_widget_frame_sprite(
            widget.frame_type,
            frame_obtained,
        )),
        inside_x + scroll_x + widget.x + ADVANCEMENTS_WIDGET_FRAME_OFFSET_X,
        inside_y + scroll_y + widget.y,
        ADVANCEMENTS_WIDGET_FRAME_WIDTH,
        ADVANCEMENTS_WIDGET_FRAME_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    let title_y = inside_y + title_top + ADVANCEMENTS_HOVER_TITLE_PADDING_TOP;
    let title_x = if left_side {
        inside_x + title_left + ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT as i32 + 2
    } else {
        inside_x + scroll_x + widget.x + ADVANCEMENTS_HOVER_TITLE_X
    };
    push_advancement_hover_text_lines(
        text_labels,
        &title_lines,
        title_x,
        title_y,
        ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR,
    );
    if let Some(progress_text) = progress_text {
        let progress_x = if left_side {
            inside_x + scroll_x + widget.x - i32::try_from(progress_width).unwrap_or_default()
        } else {
            inside_x + scroll_x + widget.x + tooltip_width_i32
                - i32::try_from(progress_width).unwrap_or_default()
                - ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT as i32
        };
        push_advancement_hover_text_label(
            text_labels,
            progress_text,
            progress_x,
            title_y,
            progress_width,
            ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR,
            Vec::new(),
        );
    }

    let description_y = if top_side {
        inside_y + title_top - description_text_height + 1
    } else {
        inside_y + title_bar_bottom
    };
    push_advancement_hover_text_lines(
        text_labels,
        &description_lines,
        inside_x + title_left + ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT as i32,
        description_y,
        advancement_hover_description_text_color(widget.frame_type),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_advancement_hover_progress_box_segment(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    sprite: HudAdvancementHoverBoxSprite,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    source_x0: u32,
    source_x1: u32,
) {
    if width == 0 {
        return;
    }
    let source_width = ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH.max(1) as f32;
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::AdvancementHoverBox(sprite),
        x,
        y,
        width,
        height,
        [source_x0 as f32 / source_width, 0.0],
        [source_x1 as f32 / source_width, 1.0],
    ));
}

fn advancement_hover_progress_box_state(
    widget: &bbb_world::AdvancementWidgetSummary,
    tooltip_width: u32,
) -> (
    HudAdvancementHoverBoxSprite,
    HudAdvancementHoverBoxSprite,
    u32,
    bool,
) {
    let amount = advancement_progress_percent(widget);
    let mut first_half_width = (amount * tooltip_width as f32).floor() as u32;
    if amount >= 1.0 {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Obtained,
            first_half_width,
            true,
        )
    } else if first_half_width < 2 {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Unobtained,
            HudAdvancementHoverBoxSprite::Unobtained,
            first_half_width,
            false,
        )
    } else if first_half_width > tooltip_width.saturating_sub(2) {
        first_half_width = tooltip_width / 2;
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Obtained,
            first_half_width,
            false,
        )
    } else {
        (
            HudAdvancementHoverBoxSprite::Obtained,
            HudAdvancementHoverBoxSprite::Unobtained,
            first_half_width,
            false,
        )
    }
}

fn advancement_progress_percent(widget: &bbb_world::AdvancementWidgetSummary) -> f32 {
    if widget.progress_total == 0 {
        0.0
    } else {
        (widget.progress_done as f32 / widget.progress_total as f32).clamp(0.0, 1.0)
    }
}

fn advancement_progress_text(widget: &bbb_world::AdvancementWidgetSummary) -> Option<String> {
    (widget.progress_total > 1).then(|| {
        format!(
            "{}/{}",
            widget.progress_done.min(widget.progress_total),
            widget.progress_total
        )
    })
}

fn advancement_max_progress_width(widget: &bbb_world::AdvancementWidgetSummary) -> u32 {
    if widget.progress_total <= 1 {
        return 0;
    }
    let text = format!("{}/{}", widget.progress_total, widget.progress_total);
    hud_ascii_approx_text_width(&text)
        .unwrap_or_default()
        .saturating_add(ADVANCEMENTS_HOVER_PROGRESS_PADDING)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdvancementTextLine {
    text: String,
    runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy)]
struct AdvancementStyledChar {
    ch: char,
    style: HudTextStyle,
    color: Option<u32>,
}

fn advancement_title_lines(title: &RichTextComponent) -> Vec<AdvancementTextLine> {
    let mut lines = advancement_wrap_text_lines(title, ADVANCEMENTS_HOVER_TITLE_MAX_WIDTH);
    if lines.is_empty() {
        lines.push(AdvancementTextLine {
            text: String::new(),
            runs: Vec::new(),
        });
    }
    lines
}

fn advancement_wrap_text_lines(
    text: &RichTextComponent,
    max_width: u32,
) -> Vec<AdvancementTextLine> {
    let max_width = max_width.max(1);
    let chars = hud_styled_runs_from_rich_text(text)
        .into_iter()
        .flat_map(|run| {
            run.text
                .chars()
                .map(move |ch| AdvancementStyledChar {
                    ch,
                    style: run.style,
                    color: run.color,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut line = Vec::new();
    let mut width = 0u32;
    let mut last_break = None;
    for styled in chars {
        if styled.ch == '\n' {
            trim_advancement_line_end(&mut line);
            if !line.is_empty() {
                result.push(advancement_chars_to_line(&line));
            }
            line.clear();
            width = 0;
            last_break = None;
            continue;
        }
        let advance = hud_ascii_approx_char_width(styled.ch);
        if !line.is_empty() && width.saturating_add(advance) > max_width {
            if let Some(break_index) = last_break {
                let mut carry = line.split_off(break_index);
                trim_advancement_line_end(&mut line);
                if !line.is_empty() {
                    result.push(advancement_chars_to_line(&line));
                }
                trim_advancement_line_start(&mut carry);
                line = carry;
            } else {
                result.push(advancement_chars_to_line(&line));
                line.clear();
            }
            width = line
                .iter()
                .fold(0u32, |width, value: &AdvancementStyledChar| {
                    width.saturating_add(hud_ascii_approx_char_width(value.ch))
                });
            last_break = line
                .iter()
                .rposition(|value| value.ch.is_whitespace())
                .map(|index| index + 1);
        }
        if line.is_empty() && styled.ch.is_whitespace() {
            continue;
        }
        line.push(styled);
        width = width.saturating_add(advance);
        if styled.ch.is_whitespace() {
            last_break = Some(line.len());
        }
    }
    trim_advancement_line_end(&mut line);
    if !line.is_empty() {
        result.push(advancement_chars_to_line(&line));
    }
    result
}

fn trim_advancement_line_start(line: &mut Vec<AdvancementStyledChar>) {
    let first = line
        .iter()
        .position(|value| !value.ch.is_whitespace())
        .unwrap_or(line.len());
    line.drain(..first);
}

fn trim_advancement_line_end(line: &mut Vec<AdvancementStyledChar>) {
    while line.last().is_some_and(|value| value.ch.is_whitespace()) {
        line.pop();
    }
}

fn advancement_chars_to_line(chars: &[AdvancementStyledChar]) -> AdvancementTextLine {
    let mut runs = Vec::<HudStyledTextRun>::new();
    let mut text = String::new();
    for value in chars {
        text.push(value.ch);
        if let Some(run) = runs
            .last_mut()
            .filter(|run| run.style == value.style && run.color == value.color)
        {
            run.text.push(value.ch);
        } else {
            runs.push(HudStyledTextRun {
                text: value.ch.to_string(),
                style: value.style,
                color: value.color,
            });
        }
    }
    AdvancementTextLine { text, runs }
}

fn push_advancement_hover_text_lines(
    labels: &mut Vec<HudInventoryTextLabel>,
    lines: &[AdvancementTextLine],
    x: i32,
    y: i32,
    tint: [f32; 4],
) {
    for (line_index, line) in lines.iter().enumerate() {
        push_advancement_hover_text_label(
            labels,
            line.text.clone(),
            x,
            y + i32::try_from(line_index).unwrap_or_default() * ADVANCEMENTS_HOVER_LINE_HEIGHT,
            hud_ascii_approx_text_width(&line.text).unwrap_or(1),
            tint,
            line.runs.clone(),
        );
    }
}

fn push_advancement_hover_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: String,
    x: i32,
    y: i32,
    width: u32,
    tint: [f32; 4],
    runs: Vec<HudStyledTextRun>,
) {
    labels.push(HudInventoryTextLabel {
        x,
        y,
        width,
        text,
        tint,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs,
    });
}

fn advancement_hover_description_text_color(frame_type: AdvancementFrameType) -> [f32; 4] {
    match frame_type {
        AdvancementFrameType::Task | AdvancementFrameType::Goal => {
            ADVANCEMENTS_HOVER_TASK_DESCRIPTION_TEXT_COLOR
        }
        AdvancementFrameType::Challenge => ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR,
    }
}

#[allow(clippy::too_many_arguments)]
fn advancement_hover_icon_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    widget: &bbb_world::AdvancementWidgetSummary,
    scroll_x: i32,
    scroll_y: i32,
    inside_x: i32,
    inside_y: i32,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Option<HudInventoryItem> {
    let item_runtime = item_runtime?;
    let stack = ItemStackSummary {
        item_id: Some(widget.icon.item_id),
        count: widget.icon.count,
        component_patch: widget.icon.component_patch.clone(),
    };
    let icon = hud_fake_item_icon_for_stack(
        world,
        Some(item_runtime),
        &stack,
        None,
        false,
        false,
        false,
        false,
        false,
        keybind_context,
        0,
        partial_tick,
    )?;
    let (icon_x, icon_y) = ADVANCEMENTS_WIDGET_ICON_OFFSET;
    Some(HudInventoryItem {
        x: inside_x + scroll_x + widget.x + icon_x,
        y: inside_y + scroll_y + widget.y + icon_y,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: false,
        block_model: fake_block_item_3d_model(
            world,
            &stack,
            Some(item_runtime),
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
        ),
    })
}

fn advancements_widget_scroll(
    widgets: &[bbb_world::AdvancementWidgetSummary],
    scroll_delta: Option<(f64, f64)>,
) -> Option<(i32, i32)> {
    let mut iter = widgets.iter();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut max_x = first.x + ADVANCEMENTS_WIDGET_BOUNDS_WIDTH;
    let mut min_y = first.y;
    let mut max_y = first.y + ADVANCEMENTS_WIDGET_BOUNDS_HEIGHT;
    for widget in iter {
        min_x = min_x.min(widget.x);
        max_x = max_x.max(widget.x + ADVANCEMENTS_WIDGET_BOUNDS_WIDTH);
        min_y = min_y.min(widget.y);
        max_y = max_y.max(widget.y + ADVANCEMENTS_WIDGET_BOUNDS_HEIGHT);
    }
    let center_x = ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32 / 2 - (max_x + min_x) / 2;
    let center_y = ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32 / 2 - (max_y + min_y) / 2;
    let (delta_x, delta_y) = scroll_delta.unwrap_or((0.0, 0.0));
    let scroll_x = if max_x - min_x > ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32 {
        (f64::from(center_x) + delta_x).clamp(
            -f64::from(max_x - ADVANCEMENTS_WINDOW_INSIDE_WIDTH as i32),
            0.0,
        )
    } else {
        f64::from(center_x)
    };
    let scroll_y = if max_y - min_y > ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32 {
        (f64::from(center_y) + delta_y).clamp(
            -f64::from(max_y - ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32),
            0.0,
        )
    } else {
        f64::from(center_y)
    };
    Some((scroll_x.floor() as i32, scroll_y.floor() as i32))
}

fn advancement_widget_visible(widget: &bbb_world::AdvancementWidgetSummary) -> bool {
    !widget.hidden || widget.done
}

fn advancement_widget_frame_sprite(
    frame_type: AdvancementFrameType,
    done: bool,
) -> HudAdvancementWidgetFrameSprite {
    match (frame_type, done) {
        (AdvancementFrameType::Task, true) => HudAdvancementWidgetFrameSprite::TaskObtained,
        (AdvancementFrameType::Task, false) => HudAdvancementWidgetFrameSprite::TaskUnobtained,
        (AdvancementFrameType::Challenge, true) => {
            HudAdvancementWidgetFrameSprite::ChallengeObtained
        }
        (AdvancementFrameType::Challenge, false) => {
            HudAdvancementWidgetFrameSprite::ChallengeUnobtained
        }
        (AdvancementFrameType::Goal, true) => HudAdvancementWidgetFrameSprite::GoalObtained,
        (AdvancementFrameType::Goal, false) => HudAdvancementWidgetFrameSprite::GoalUnobtained,
    }
}

fn advancements_window_origin_for_surface(
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> (i32, i32) {
    let screen_width = i32::try_from(surface_size.width.max(1)).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX);
    (
        (screen_width - ADVANCEMENTS_WINDOW_WIDTH as i32) / 2,
        (screen_height - ADVANCEMENTS_WINDOW_HEIGHT as i32) / 2,
    )
}

fn advancements_done_button_origin_for_surface(
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> (i32, i32) {
    let screen_width = i32::try_from(surface_size.width.max(1)).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX);
    (
        (screen_width - ADVANCEMENTS_DONE_BUTTON_WIDTH as i32) / 2,
        screen_height - ADVANCEMENTS_FOOTER_HEIGHT
            + (ADVANCEMENTS_FOOTER_HEIGHT - ADVANCEMENTS_DONE_BUTTON_HEIGHT as i32) / 2,
    )
}

fn advancements_screen_text_labels(
    selected_title: Option<&RichTextComponent>,
    window_x: i32,
    window_y: i32,
    done_button_x: i32,
    done_button_y: i32,
) -> Vec<HudInventoryTextLabel> {
    let title = selected_title
        .map(RichTextComponent::text)
        .unwrap_or(ADVANCEMENTS_TITLE_TEXT);
    let mut labels = vec![HudInventoryTextLabel {
        x: window_x + ADVANCEMENTS_WINDOW_TITLE_X,
        y: window_y + ADVANCEMENTS_WINDOW_TITLE_Y,
        width: hud_ascii_approx_text_width(title).unwrap_or_default(),
        text: title.to_string(),
        tint: ADVANCEMENTS_TITLE_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: selected_title
            .map(hud_styled_runs_from_rich_text)
            .unwrap_or_default(),
    }];
    if selected_title.is_none() {
        push_centered_advancements_label(
            &mut labels,
            ADVANCEMENTS_EMPTY_TEXT,
            window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
            window_y + ADVANCEMENTS_EMPTY_TEXT_Y,
        );
        push_centered_advancements_label(
            &mut labels,
            ADVANCEMENTS_SAD_TEXT,
            window_x + ADVANCEMENTS_EMPTY_TEXT_CENTER_X,
            window_y + ADVANCEMENTS_SAD_TEXT_Y,
        );
    }
    push_centered_advancements_label(
        &mut labels,
        ADVANCEMENTS_DONE_TEXT,
        done_button_x + ADVANCEMENTS_DONE_BUTTON_WIDTH as i32 / 2,
        done_button_y + ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
    );
    labels
}

fn push_centered_advancements_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &str,
    center_x: i32,
    y: i32,
) {
    let Some(width) = hud_ascii_approx_text_width(text) else {
        return;
    };
    let Ok(width_i32) = i32::try_from(width) else {
        return;
    };
    labels.push(HudInventoryTextLabel {
        x: center_x - width_i32 / 2,
        y,
        width,
        text: text.to_string(),
        tint: if text == ADVANCEMENTS_DONE_TEXT {
            ADVANCEMENTS_DONE_TEXT_COLOR
        } else {
            ADVANCEMENTS_EMPTY_TEXT_COLOR
        },
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    });
}

fn hud_inventory_text_labels(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    screen_height: i32,
    local_state: &InventoryHudLocalState,
    enchantment_options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Vec<HudInventoryTextLabel> {
    let mut labels = if background == InventoryScreenBackground::Beacon {
        hud_beacon_text_labels(item_runtime)
    } else {
        hud_server_opened_container_labels(world, item_runtime, background, screen_height)
    };
    match background {
        InventoryScreenBackground::Anvil => {
            if let Some(text) = local_state
                .anvil_rename_text
                .as_ref()
                .filter(|text| !text.is_empty())
            {
                labels.push(HudInventoryTextLabel {
                    x: 62,
                    y: 24,
                    width: 103,
                    text: text.clone(),
                    tint: ANVIL_RENAME_TEXT_COLOR,
                    background: None,
                    input: None,
                    scrolling: None,
                    shadow: false,
                    font: None,
                    runs: Vec::new(),
                });
            }
            if let Some(label) = anvil_cost_text_label(world) {
                labels.push(label);
            }
        }
        InventoryScreenBackground::EnchantmentTable => {
            labels.extend(enchanting_table_text_labels(
                enchantment_options.map_or(&[], |options| options.as_slice()),
            ));
        }
        InventoryScreenBackground::Lectern => {}
        _ => {}
    }
    labels
}

fn hud_beacon_text_labels(item_runtime: Option<&NativeItemRuntime>) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::with_capacity(2);
    for (translation_key, fallback, center_x) in [
        (
            "block.minecraft.beacon.primary",
            "Primary Power",
            BEACON_PRIMARY_LABEL_CENTER_X,
        ),
        (
            "block.minecraft.beacon.secondary",
            "Secondary Power",
            BEACON_SECONDARY_LABEL_CENTER_X,
        ),
    ] {
        let text = localized_gui_text(item_runtime, translation_key, fallback);
        let bounds_x =
            center_x - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap_or(i32::MAX) / 2;
        labels.push(HudInventoryTextLabel {
            x: bounds_x,
            y: BEACON_LABEL_Y,
            width: CONTAINER_LABEL_WIDTH,
            text: text.clone(),
            tint: BEACON_LABEL_TEXT_COLOR,
            background: None,
            input: None,
            scrolling: Some(HudInventoryScrollingText {
                x: bounds_x,
                y: BEACON_LABEL_Y - 1,
                width: CONTAINER_CENTERED_LABEL_WIDTH,
                height: CONTAINER_TEXT_LINE_HEIGHT,
            }),
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    labels
}

fn hud_server_opened_container_labels(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    screen_height: i32,
) -> Vec<HudInventoryTextLabel> {
    let Some(container) = world.inventory().open_container.as_ref() else {
        return Vec::new();
    };
    let Some(title) = container.title.as_ref() else {
        return Vec::new();
    };
    if matches!(
        background,
        InventoryScreenBackground::LocalInventory
            | InventoryScreenBackground::Mount { .. }
            | InventoryScreenBackground::Lectern
    ) {
        return Vec::new();
    }

    let title_runs = localized_component_runs(title, &ComponentStyle::default(), item_runtime);
    let mut labels = Vec::with_capacity(if background == InventoryScreenBackground::Merchant {
        3
    } else {
        2
    });
    if background == InventoryScreenBackground::Merchant {
        let title_runs = hud_merchant_title_runs(world, item_runtime, title_runs);
        push_centered_container_label(&mut labels, title_runs, 187, 6);
        push_plain_container_label(
            &mut labels,
            localized_gui_text(item_runtime, "container.inventory", "Inventory"),
            107,
            screen_height - 94,
        );
        push_centered_container_label(
            &mut labels,
            vec![StyledTextRun {
                text: localized_gui_text(item_runtime, "merchant.trades", "Trades"),
                style: ComponentStyle::default(),
            }],
            53,
            6,
        );
        return labels;
    }

    match background {
        InventoryScreenBackground::Generic3x3
        | InventoryScreenBackground::Crafter
        | InventoryScreenBackground::BlastFurnace
        | InventoryScreenBackground::BrewingStand
        | InventoryScreenBackground::Furnace
        | InventoryScreenBackground::Smoker => {
            push_centered_container_label(&mut labels, title_runs, 88, 6);
        }
        InventoryScreenBackground::Anvil => {
            push_component_container_label(&mut labels, title_runs, 60, 6);
        }
        InventoryScreenBackground::CraftingTable => {
            push_component_container_label(&mut labels, title_runs, 29, 6);
        }
        InventoryScreenBackground::Loom | InventoryScreenBackground::CartographyTable => {
            push_component_container_label(&mut labels, title_runs, 8, 4);
        }
        InventoryScreenBackground::Smithing => {
            push_component_container_label(&mut labels, title_runs, 44, 15);
        }
        InventoryScreenBackground::Stonecutter => {
            push_component_container_label(&mut labels, title_runs, 8, 5);
        }
        InventoryScreenBackground::Generic9xRows { .. }
        | InventoryScreenBackground::Beacon
        | InventoryScreenBackground::EnchantmentTable
        | InventoryScreenBackground::Grindstone
        | InventoryScreenBackground::Hopper
        | InventoryScreenBackground::ShulkerBox => {
            push_component_container_label(&mut labels, title_runs, 8, 6);
        }
        InventoryScreenBackground::LocalInventory
        | InventoryScreenBackground::Mount { .. }
        | InventoryScreenBackground::Lectern
        | InventoryScreenBackground::Merchant => {
            unreachable!("non-standard container labels returned above")
        }
    }
    push_plain_container_label(
        &mut labels,
        localized_gui_text(item_runtime, "container.inventory", "Inventory"),
        8,
        screen_height - 94,
    );
    labels
}

fn hud_merchant_title_runs(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    title_runs: Vec<StyledTextRun>,
) -> Vec<StyledTextRun> {
    let Some(offers) = merchant_offers_state(world)
        .filter(|offers| offers.show_progress && (1..=5).contains(&offers.villager_level))
    else {
        return title_runs;
    };
    let level_key = format!("merchant.level.{}", offers.villager_level);
    let level_fallback = match offers.villager_level {
        1 => "Novice",
        2 => "Apprentice",
        3 => "Journeyman",
        4 => "Expert",
        5 => "Master",
        _ => unreachable!("merchant level was range checked"),
    };
    let level_runs = vec![StyledTextRun {
        text: localized_gui_text(item_runtime, &level_key, level_fallback),
        style: ComponentStyle::default(),
    }];
    let template = localized_gui_text(item_runtime, "merchant.title", "%s - %s");
    let arguments = [&title_runs, &level_runs];
    let mut runs = Vec::new();
    visit_localized_template(&template, |part| match part {
        LocalizedTemplatePart::Literal(text) => {
            push_component_text_run(&mut runs, text, &ComponentStyle::default());
        }
        LocalizedTemplatePart::Argument { index, raw } => {
            if let Some(argument) = arguments.get(index) {
                for run in *argument {
                    push_component_text_run(&mut runs, &run.text, &run.style);
                }
            } else {
                push_component_text_run(&mut runs, raw, &ComponentStyle::default());
            }
        }
    });
    runs
}

fn push_plain_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: String,
    x: i32,
    y: i32,
) {
    push_component_container_label(
        labels,
        vec![StyledTextRun {
            text,
            style: ComponentStyle::default(),
        }],
        x,
        y,
    );
}

fn push_component_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<StyledTextRun>,
    x: i32,
    y: i32,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    if text.is_empty() {
        return;
    }
    labels.push(HudInventoryTextLabel {
        x,
        y,
        width: CONTAINER_LABEL_WIDTH,
        text,
        tint: CONTAINER_LABEL_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: hud_styled_runs_from_component_runs(&runs),
    });
}

fn push_centered_container_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<StyledTextRun>,
    center_x: i32,
    y: i32,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    if text.is_empty() {
        return;
    }
    let bounds_x = center_x - i32::try_from(CONTAINER_CENTERED_LABEL_WIDTH).unwrap_or(i32::MAX) / 2;
    labels.push(HudInventoryTextLabel {
        x: bounds_x,
        y,
        width: CONTAINER_LABEL_WIDTH,
        text,
        tint: CONTAINER_LABEL_TEXT_COLOR,
        background: None,
        input: None,
        scrolling: Some(HudInventoryScrollingText {
            x: bounds_x,
            y: y - 1,
            width: CONTAINER_CENTERED_LABEL_WIDTH,
            height: CONTAINER_TEXT_LINE_HEIGHT,
        }),
        shadow: false,
        font: None,
        runs: hud_styled_runs_from_component_runs(&runs),
    });
}

fn push_component_text_run(runs: &mut Vec<StyledTextRun>, text: &str, style: &ComponentStyle) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut().filter(|previous| previous.style == *style) {
        previous.text.push_str(text);
    } else {
        runs.push(StyledTextRun {
            text: text.to_string(),
            style: style.clone(),
        });
    }
}

fn enchanting_table_text_labels(
    options: &[EnchantmentOptionHudState],
) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::with_capacity(options.len() * 2);
    for option in options {
        if option.cost <= 0 {
            continue;
        }
        let row = i32::try_from(option.index).expect("three enchanting rows fit i32");
        labels.push(HudInventoryTextLabel {
            x: ENCHANTING_TABLE_OPTION_X + 20,
            y: ENCHANTING_TABLE_OPTION_Y + 2 + row * ENCHANTING_TABLE_OPTION_SPACING,
            width: option.name_max_width,
            text: option.name.clone(),
            tint: if !option.enabled {
                ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR
            } else if option.highlighted {
                ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR
            } else {
                ENCHANTING_TABLE_NAME_TEXT_COLOR
            },
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: Some(ENCHANTMENT_ALT_FONT_ID.to_string()),
            runs: Vec::new(),
        });

        let text = option.cost.to_string();
        let text_width = ENCHANTING_TABLE_OPTION_TEXT_WIDTH.saturating_sub(option.name_max_width);
        let text_width_i32 = i32::try_from(text_width).expect("enchanting cost width fits i32");
        labels.push(HudInventoryTextLabel {
            x: ENCHANTING_TABLE_COST_TEXT_X_RIGHT - text_width_i32,
            y: ENCHANTING_TABLE_COST_TEXT_Y_OFFSET + row * ENCHANTING_TABLE_OPTION_SPACING,
            width: text_width,
            text,
            tint: if option.enabled {
                ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR
            } else {
                ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR
            },
            background: None,
            input: None,
            scrolling: None,
            shadow: false,
            font: None,
            runs: Vec::new(),
        });
    }
    labels
}

fn anvil_cost_text_label(world: &WorldStore) -> Option<HudInventoryTextLabel> {
    let cost = world.open_container_data_value(ANVIL_COST_DATA_ID)?;
    if cost <= 0 {
        return None;
    }

    let creative = world.gameplay().game_type == GameType::Creative.id();
    let (text, tint) = if cost >= ANVIL_TOO_EXPENSIVE_LEVEL_COST && !creative {
        ("Too Expensive!".to_string(), ANVIL_COST_ERROR_TEXT_COLOR)
    } else {
        if !open_container_slot_has_item(world, ANVIL_RESULT_SLOT) {
            return None;
        }
        let may_pickup = creative
            || world
                .local_player()
                .experience
                .is_some_and(|experience| experience.level >= i32::from(cost));
        (
            format!("Cost: {cost}"),
            if may_pickup {
                ANVIL_COST_TEXT_COLOR
            } else {
                ANVIL_COST_ERROR_TEXT_COLOR
            },
        )
    };

    let text_width = hud_ascii_approx_text_width(&text)?;
    let x = ANVIL_COST_LABEL_RIGHT - 2 - i32::try_from(text_width).ok()?;
    Some(HudInventoryTextLabel {
        x,
        y: ANVIL_COST_LABEL_Y,
        width: text_width,
        text,
        tint,
        background: Some(HudInventoryTextBackground {
            x: x - 2,
            y: ANVIL_COST_BACKGROUND_Y,
            width: text_width.saturating_add(4),
            height: ANVIL_COST_BACKGROUND_HEIGHT,
            tint: ANVIL_COST_BACKGROUND_TINT,
        }),
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        runs: Vec::new(),
    })
}

fn hud_ascii_approx_text_width(text: &str) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(hud_ascii_approx_char_width(ch))?;
    }
    (width > 0).then_some(width)
}

fn hud_ascii_approx_char_width(ch: char) -> u32 {
    if ch == ' ' {
        4
    } else {
        6
    }
}

fn hud_inventory_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    layout_slots: &[crate::input::InventorySlotLayout],
    container: &ContainerState,
    advanced: bool,
    creative: bool,
) -> Option<HudInventoryTooltip> {
    let slot_id = hovered_slot_id?;
    let layout = layout_slots
        .iter()
        .find(|layout| layout.slot_id == slot_id)?;
    let slot = container.slots.iter().find(|slot| slot.slot == slot_id)?;
    let tooltip = hud_item_tooltip(world, item_runtime, &slot.item, advanced, creative)?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x: layout.x,
        y: layout.y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

#[allow(clippy::too_many_arguments)]
fn hud_recipe_book_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    tabs: &RecipeBookTabSelectionHudState,
    pages: &RecipeBookPageHudState,
    search: &RecipeBookSearchHudState,
    overlay: Option<RecipeBookOverlayHudState>,
    cursor_position: Option<(i32, i32)>,
    tooltip_x_offset: i32,
    advanced: bool,
    creative: bool,
    partial_tick: f32,
) -> Option<HudInventoryTooltip> {
    if overlay.is_some() {
        return None;
    }
    let book_type = recipe_book_type_for_background(background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let (cursor_x, cursor_y) = cursor_position?;
    let collections = hud_recipe_book_collections(world, item_runtime, background, tabs, search)?;
    let page = clamped_recipe_book_page(
        recipe_book_selected_page_index(background, pages).unwrap_or_default(),
        collections.len(),
    );
    for (index, collection) in collections
        .iter()
        .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
        .take(RECIPE_BOOK_ITEMS_PER_PAGE)
        .enumerate()
    {
        let (x, y) = recipe_book_recipe_button_position(index);
        if cursor_x < x
            || cursor_x >= x + RECIPE_BOOK_RECIPE_BUTTON_SIZE
            || cursor_y < y
            || cursor_y >= y + RECIPE_BOOK_RECIPE_BUTTON_SIZE
        {
            continue;
        }
        let stack = collection.result_stack_at_slot_select_index(recipe_book_slot_select_index(
            world,
            partial_tick,
        ))?;
        let mut tooltip = hud_item_tooltip(world, item_runtime, stack, advanced, creative)?;
        if collection.has_multiple_recipes() {
            tooltip.lines.push(HudInventoryTooltipLine {
                text: localized_gui_text(
                    item_runtime,
                    "gui.recipebook.moreRecipes",
                    "Right Click for More",
                ),
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: Vec::new(),
            });
        }
        return Some(HudInventoryTooltip {
            slot_id: u16::MAX,
            x: cursor_x + tooltip_x_offset,
            y: cursor_y,
            style: tooltip.style,
            lines: tooltip.lines,
        });
    }

    if cursor_x >= RECIPE_BOOK_FILTER_BUTTON_X
        && cursor_x < RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH
        && cursor_y >= RECIPE_BOOK_FILTER_BUTTON_Y
        && cursor_y < RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT
    {
        let filtering = recipe_book_type_settings(world, book_type).filtering;
        let (key, fallback) = recipe_book_filter_tooltip_text(book_type, filtering);
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            key,
            fallback,
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }

    let page_count = recipe_book_page_count(collections.len());
    if page + 1 < page_count
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X)
    {
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            "gui.recipebook.next_page",
            "Next Page",
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }
    if page > 0
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X)
    {
        return Some(hud_recipe_book_control_tooltip(
            item_runtime,
            "gui.recipebook.previous_page",
            "Previous Page",
            cursor_x + tooltip_x_offset,
            cursor_y,
        ));
    }
    None
}

fn recipe_book_page_control_contains(cursor_x: i32, cursor_y: i32, button_x: i32) -> bool {
    cursor_x >= button_x
        && cursor_x < button_x + RECIPE_BOOK_PAGE_BUTTON_WIDTH
        && cursor_y >= RECIPE_BOOK_PAGE_BUTTON_Y
        && cursor_y < RECIPE_BOOK_PAGE_BUTTON_Y + RECIPE_BOOK_PAGE_BUTTON_HEIGHT
}

fn hud_recipe_book_control_tooltip(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
    x: i32,
    y: i32,
) -> HudInventoryTooltip {
    HudInventoryTooltip {
        slot_id: u16::MAX,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text: localized_gui_text(item_runtime, translation_key, fallback),
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    }
}

fn hud_beacon_tooltip(
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    selection: (Option<i32>, Option<i32>),
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    for button in beacon_effect_buttons(selection.0) {
        if !beacon_button_contains(cursor_position, button.x, button.y) {
            continue;
        }
        let descriptor = vanilla_mob_effect_type(button.effect_id)?;
        let resource_path = descriptor
            .resource_id
            .strip_prefix("minecraft:")
            .unwrap_or(descriptor.resource_id);
        let translation_key = format!("effect.minecraft.{resource_path}");
        let fallback = humanize_resource_path(resource_path);
        let mut text = localized_gui_text(item_runtime, &translation_key, &fallback);
        if button.upgrade {
            text.push_str(" II");
        }
        return Some(hud_beacon_control_tooltip(text, cursor_x, cursor_y));
    }

    let action = if beacon_button_contains(
        cursor_position,
        BEACON_CONFIRM_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        Some(("gui.done", "Done"))
    } else if beacon_button_contains(
        cursor_position,
        BEACON_CANCEL_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        Some(("gui.cancel", "Cancel"))
    } else {
        None
    };
    let (translation_key, fallback) = action?;
    Some(hud_beacon_control_tooltip(
        localized_gui_text(item_runtime, translation_key, fallback),
        cursor_x,
        cursor_y,
    ))
}

fn hud_beacon_control_tooltip(text: String, x: i32, y: i32) -> HudInventoryTooltip {
    HudInventoryTooltip {
        slot_id: u16::MAX,
        x,
        y,
        style: None,
        lines: vec![HudInventoryTooltipLine {
            text,
            tint: [1.0, 1.0, 1.0, 1.0],
            runs: Vec::new(),
        }],
    }
}

fn recipe_book_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<ScreenNarrationFrame> {
    let layout = inventory_screen_layout_for_surface(world, surface_size)?;
    let book_type = recipe_book_type_for_background(layout.background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let search = input.recipe_book_search_hud_state();
    let mut frame = ScreenNarrationFrame::default();
    if search.focused {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%s edit box: %s",
                &[
                    &localized_gui_text(item_runtime, "itemGroup.search", "Search Items"),
                    &search.text,
                ],
            ),
        );
        return Some(frame);
    }

    let recipe_book_x = layout
        .recipe_book
        .map(|recipe_book| recipe_book.x)
        .unwrap_or_default();
    let cursor = input
        .inventory_cursor_position()
        .map(|(x, y)| (x - recipe_book_x, y));
    let Some((cursor_x, cursor_y)) = cursor else {
        return Some(frame);
    };
    let tabs = input.recipe_book_tab_selection_hud_state();
    let pages = input.recipe_book_page_hud_state();
    let collections =
        hud_recipe_book_collections(world, item_runtime, layout.background, &tabs, &search)?;
    let page = clamped_recipe_book_page(
        recipe_book_selected_page_index(layout.background, &pages).unwrap_or_default(),
        collections.len(),
    );

    if input.recipe_book_overlay_hud_state().is_none() {
        for (index, collection) in collections
            .iter()
            .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
            .take(RECIPE_BOOK_ITEMS_PER_PAGE)
            .enumerate()
        {
            let (x, y) = recipe_book_recipe_button_position(index);
            if cursor_x < x
                || cursor_x >= x + RECIPE_BOOK_RECIPE_BUTTON_SIZE
                || cursor_y < y
                || cursor_y >= y + RECIPE_BOOK_RECIPE_BUTTON_SIZE
            {
                continue;
            }
            let stack = collection.result_stack_at_slot_select_index(
                recipe_book_slot_select_index(world, partial_tick),
            )?;
            let item_name = hud_item_tooltip(world, item_runtime, stack, false, false)?
                .lines
                .into_iter()
                .next()?
                .text;
            frame.add(
                NarratedElementKind::Title,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "narration.recipe",
                    "Recipe for %s",
                    &[&item_name],
                ),
            );
            frame.add(
                NarratedElementKind::Usage,
                1,
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.hovered",
                    "Left click to activate",
                ),
            );
            if collection.has_multiple_recipes() {
                frame.add(
                    NarratedElementKind::Usage,
                    2,
                    localized_gui_text(
                        item_runtime,
                        "narration.recipe.usage.more",
                        "Right click to show more recipes",
                    ),
                );
            }
            return Some(frame);
        }
    }

    if cursor_x >= RECIPE_BOOK_SEARCH_BOX_X
        && cursor_x < RECIPE_BOOK_SEARCH_BOX_X + RECIPE_BOOK_SEARCH_BOX_WIDTH
        && cursor_y >= RECIPE_BOOK_SEARCH_BOX_Y
        && cursor_y < RECIPE_BOOK_SEARCH_BOX_Y + RECIPE_BOOK_SEARCH_BOX_HEIGHT
    {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%s edit box: %s",
                &[
                    &localized_gui_text(item_runtime, "itemGroup.search", "Search Items"),
                    &search.text,
                ],
            ),
        );
        return Some(frame);
    }

    if cursor_x >= RECIPE_BOOK_FILTER_BUTTON_X
        && cursor_x < RECIPE_BOOK_FILTER_BUTTON_X + RECIPE_BOOK_FILTER_BUTTON_WIDTH
        && cursor_y >= RECIPE_BOOK_FILTER_BUTTON_Y
        && cursor_y < RECIPE_BOOK_FILTER_BUTTON_Y + RECIPE_BOOK_FILTER_BUTTON_HEIGHT
    {
        let filtering = recipe_book_type_settings(world, book_type).filtering;
        let (key, fallback) = recipe_book_filter_tooltip_text(book_type, filtering);
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(item_runtime, key, fallback),
        );
        return Some(frame);
    }

    let visible_tabs = recipe_book_visible_tab_indices(world, layout.background);
    for (visible_index, _tab_index) in visible_tabs.into_iter().enumerate() {
        let tab_y = RECIPE_BOOK_TAB_Y + RECIPE_BOOK_TAB_STRIDE_Y * visible_index as i32;
        if cursor_x >= RECIPE_BOOK_TAB_X
            && cursor_x < RECIPE_BOOK_TAB_X + RECIPE_BOOK_TAB_WIDTH
            && cursor_y >= tab_y
            && cursor_y < tab_y + RECIPE_BOOK_TAB_HEIGHT
        {
            return Some(frame);
        }
    }

    let page_count = recipe_book_page_count(collections.len());
    if page + 1 < page_count
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_FORWARD_BUTTON_X)
    {
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(item_runtime, "gui.recipebook.next_page", "Next Page"),
        );
    } else if page > 0
        && recipe_book_page_control_contains(cursor_x, cursor_y, RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X)
    {
        add_recipe_book_button_narration(
            &mut frame,
            item_runtime,
            &localized_gui_text(
                item_runtime,
                "gui.recipebook.previous_page",
                "Previous Page",
            ),
        );
    }
    Some(frame)
}

fn recipe_book_filter_tooltip_text(
    book_type: RecipeBookType,
    filtering: bool,
) -> (&'static str, &'static str) {
    if !filtering {
        return ("gui.recipebook.toggleRecipes.all", "Showing All");
    }
    match book_type {
        RecipeBookType::Crafting => (
            "gui.recipebook.toggleRecipes.craftable",
            "Showing Craftable",
        ),
        RecipeBookType::Furnace => (
            "gui.recipebook.toggleRecipes.smeltable",
            "Showing Smeltable",
        ),
        RecipeBookType::BlastFurnace => (
            "gui.recipebook.toggleRecipes.blastable",
            "Showing Blastable",
        ),
        RecipeBookType::Smoker => ("gui.recipebook.toggleRecipes.smokable", "Showing Smokable"),
    }
}

fn add_recipe_book_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
) {
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            "narration.button.usage.hovered",
            "Left click to activate",
        ),
    );
}

fn hud_recipe_book_ghost_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    hovered_slot_id: Option<i16>,
    layout_slots: &[crate::input::InventorySlotLayout],
    advanced: bool,
    creative: bool,
    partial_tick: f32,
) -> Option<HudInventoryTooltip> {
    let book_type = recipe_book_type_for_background(background)?;
    if !recipe_book_type_settings(world, book_type).open {
        return None;
    }
    let ghost_recipe = world.last_ghost_recipe()?;
    if world.open_container_id() != Some(ghost_recipe.container_id) {
        return None;
    }
    let slot_id = hovered_slot_id?;
    let ghost_slot = ghost_recipe
        .slots
        .iter()
        .find(|slot| slot.slot_id == slot_id)?;
    let slot_select_index = recipe_book_slot_select_index(world, partial_tick);
    let item = ghost_slot
        .items
        .get(slot_select_index % ghost_slot.items.len().max(1))?;
    let layout = layout_slots
        .iter()
        .find(|layout| layout.slot_id == slot_id)?;
    let tooltip = hud_item_tooltip(world, item_runtime, item, advanced, creative)?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x: layout.x,
        y: layout.y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

fn hud_merchant_trade_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    advanced: bool,
    creative: bool,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let offers = merchant_offers_state(world)?;
    if offers.can_restock
        && merchant_selected_offer(offers).is_some_and(|offer| offer.is_out_of_stock)
        && cursor_x >= MERCHANT_DEPRECATED_TOOLTIP_X
        && cursor_x < MERCHANT_DEPRECATED_TOOLTIP_X + MERCHANT_DEPRECATED_TOOLTIP_WIDTH
        && cursor_y >= MERCHANT_DEPRECATED_TOOLTIP_Y
        && cursor_y < MERCHANT_DEPRECATED_TOOLTIP_Y + MERCHANT_DEPRECATED_TOOLTIP_HEIGHT
    {
        return Some(HudInventoryTooltip {
            slot_id: u16::MAX,
            x: cursor_x,
            y: cursor_y,
            style: None,
            lines: vec![HudInventoryTooltipLine {
                text: localized_gui_text(
                    item_runtime,
                    "merchant.deprecated",
                    "Villagers restock up to two times per day.",
                ),
                tint: [1.0, 1.0, 1.0, 1.0],
                runs: Vec::new(),
            }],
        });
    }

    if cursor_x < MERCHANT_TRADE_BUTTON_X
        || cursor_x >= MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH as i32
        || cursor_y < MERCHANT_TRADE_BUTTON_Y
    {
        return None;
    }
    let row = (cursor_y - MERCHANT_TRADE_BUTTON_Y) / MERCHANT_TRADE_ROW_HEIGHT;
    if !(0..MERCHANT_VISIBLE_OFFER_COUNT as i32).contains(&row)
        || cursor_y
            >= MERCHANT_TRADE_BUTTON_Y
                + row * MERCHANT_TRADE_ROW_HEIGHT
                + MERCHANT_TRADE_BUTTON_HEIGHT as i32
    {
        return None;
    }
    let offer_index = merchant_scroll_offset(offers).checked_add(usize::try_from(row).ok()?)?;
    let offer = offers.offers.get(offer_index)?;
    let item = if cursor_x < MERCHANT_TRADE_BUTTON_X + 20 {
        merchant_offer_cost_a_stack(world, offer)
    } else if cursor_x > MERCHANT_TRADE_BUTTON_X + 30 && cursor_x < MERCHANT_TRADE_BUTTON_X + 50 {
        let cost_b = offer.buy_b.as_ref()?;
        item_cost_stack(cost_b, cost_b.count)
    } else if cursor_x > MERCHANT_TRADE_BUTTON_X + 65 {
        offer.sell.clone()
    } else {
        return None;
    };
    let tooltip = hud_item_tooltip(world, item_runtime, &item, advanced, creative)?;
    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

fn hud_stonecutter_recipe_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    scroll_row: i32,
    advanced: bool,
    creative: bool,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let option = stonecutter_visible_recipe_option_stacks(world, scroll_row)
        .into_iter()
        .find(|option| {
            cursor_x >= option.x
                && cursor_x < option.x + STONECUTTER_RECIPE_BUTTON_WIDTH
                && cursor_y >= option.y
                && cursor_y < option.y + STONECUTTER_RECIPE_BUTTON_HEIGHT
        })?;
    let tooltip = hud_item_tooltip(world, item_runtime, &option.stack, advanced, creative)?;
    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: tooltip.style,
        lines: tooltip.lines,
    })
}

fn hud_enchantment_table_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Option<HudInventoryTooltip> {
    const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
    const TOOLTIP_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
    const TOOLTIP_GRAY: [f32; 4] = [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];
    const TOOLTIP_RED: [f32; 4] = [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0];

    let (cursor_x, cursor_y) = cursor_position?;
    let option = options?
        .iter()
        .find(|option| option.tooltip_hovered && option.cost > 0)?;
    let holder_id = option.clue_holder_id?;
    let enchantment_level = option.clue_level?;
    let resource_id = world.registry_resource_id_by_holder_id(ENCHANTMENT_REGISTRY, holder_id)?;
    let definitions = world.enchantment_tooltip_definitions_in_holder_order();
    let definition = usize::try_from(holder_id)
        .ok()
        .and_then(|holder_id| definitions.as_ref()?.get(holder_id))
        .and_then(Option::as_ref);
    if definition.is_none() && VanillaEnchantmentIdentity::for_resource_id(resource_id).is_none() {
        return None;
    }

    let enchantment_line = item_runtime
        .map(|runtime| {
            let line =
                runtime.enchantment_name_tooltip_line(resource_id, enchantment_level, definition);
            HudInventoryTooltipLine {
                text: line.text,
                tint: line.tint,
                runs: line.runs,
            }
        })
        .unwrap_or_else(|| {
            fallback_enchantment_name_tooltip_line(resource_id, enchantment_level, definition)
        });
    let mut lines = vec![enchantment_clue_tooltip_line(
        item_runtime,
        enchantment_line,
    )];

    let infinite_materials = world
        .local_player()
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    if !infinite_materials {
        lines.push(HudInventoryTooltipLine {
            text: String::new(),
            tint: TOOLTIP_WHITE,
            runs: Vec::new(),
        });
        let required_lapis =
            i32::try_from(option.index + 1).expect("three enchanting rows fit i32");
        let experience_level = world
            .local_player()
            .experience
            .map(|experience| experience.level)
            .unwrap_or_default();
        if experience_level < i32::from(option.cost) {
            let cost = option.cost.to_string();
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    "container.enchant.level.requirement",
                    "Level Requirement: %s",
                    &[&cost],
                ),
                tint: TOOLTIP_RED,
                runs: Vec::new(),
            });
        } else {
            let lapis_count = open_container_slot_item(world, 1)
                .filter(|stack| !item_stack_is_empty(stack))
                .map(|stack| stack.count)
                .unwrap_or_default();
            let resource_cost = required_lapis.to_string();
            let (lapis_key, lapis_fallback) = if required_lapis == 1 {
                ("container.enchant.lapis.one", "1 Lapis Lazuli")
            } else {
                ("container.enchant.lapis.many", "%s Lapis Lazuli")
            };
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    lapis_key,
                    lapis_fallback,
                    &[&resource_cost],
                ),
                tint: if lapis_count >= required_lapis {
                    TOOLTIP_GRAY
                } else {
                    TOOLTIP_RED
                },
                runs: Vec::new(),
            });
            let (level_key, level_fallback) = if required_lapis == 1 {
                ("container.enchant.level.one", "1 Enchantment Level")
            } else {
                ("container.enchant.level.many", "%s Enchantment Levels")
            };
            lines.push(HudInventoryTooltipLine {
                text: localized_gui_text_with_args(
                    item_runtime,
                    level_key,
                    level_fallback,
                    &[&resource_cost],
                ),
                tint: TOOLTIP_GRAY,
                runs: Vec::new(),
            });
        }
    }

    Some(HudInventoryTooltip {
        slot_id: u16::MAX,
        x: cursor_x,
        y: cursor_y,
        style: None,
        lines,
    })
}

fn enchantment_clue_tooltip_line(
    item_runtime: Option<&NativeItemRuntime>,
    enchantment: HudInventoryTooltipLine,
) -> HudInventoryTooltipLine {
    let template = localized_gui_text(item_runtime, "container.enchant.clue", "%s . . . ?");
    let mut text = String::new();
    let mut runs = Vec::new();
    visit_localized_template(&template, |part| match part {
        LocalizedTemplatePart::Literal(literal) => {
            text.push_str(literal);
            runs.push(HudComponentTextRun::plain(literal));
        }
        LocalizedTemplatePart::Argument { index: 0, .. } => {
            text.push_str(&enchantment.text);
            runs.extend(enchantment.runs.clone());
        }
        LocalizedTemplatePart::Argument { raw, .. } => {
            text.push_str(raw);
            runs.push(HudComponentTextRun::plain(raw));
        }
    });
    HudInventoryTooltipLine {
        text,
        tint: [1.0, 1.0, 1.0, 1.0],
        runs,
    }
}

fn fallback_enchantment_name_tooltip_line(
    resource_id: &str,
    level: i32,
    definition: Option<&EnchantmentTooltipDefinition>,
) -> HudInventoryTooltipLine {
    let (mut text, mut runs) = if let Some(definition) = definition {
        let mut text = definition.description.clone();
        let mut styled = definition.description_styled.clone();
        if let Some(translation_key) = definition.description_translation_key.as_deref() {
            let fallback = definition
                .description_translation_fallback
                .as_deref()
                .unwrap_or(translation_key);
            let encoded_text = definition
                .description_translation_fallback
                .as_deref()
                .filter(|fallback| text.contains(fallback))
                .unwrap_or(translation_key);
            if let Some(run) = styled.iter_mut().find(|run| run.text == encoded_text) {
                run.text = fallback.to_string();
                text = styled.iter().map(|run| run.text.as_str()).collect();
            } else if text.contains(encoded_text) {
                text = text.replacen(encoded_text, fallback, 1);
            }
        }
        (
            text,
            styled
                .into_iter()
                .map(component_hover::hud_component_run)
                .collect::<Vec<_>>(),
        )
    } else {
        let path = resource_id
            .split_once(':')
            .map(|(_, path)| path)
            .unwrap_or(resource_id);
        (humanize_resource_path(path), Vec::new())
    };
    if runs.is_empty() && !text.is_empty() {
        runs.push(HudComponentTextRun::plain(text.clone()));
    }
    let max_level = definition
        .map(|definition| definition.max_level)
        .or_else(|| {
            VanillaEnchantmentIdentity::for_resource_id(resource_id)
                .map(VanillaEnchantmentIdentity::max_level)
        })
        .unwrap_or(1);
    if level != 1 || max_level != 1 {
        let level_text = match level {
            1 => "I".to_string(),
            2..=10 => mob_effect_level_fallback(level).to_string(),
            _ => level.to_string(),
        };
        text.push(' ');
        text.push_str(&level_text);
        let mut suffix = runs
            .first()
            .cloned()
            .unwrap_or_else(|| HudComponentTextRun::plain(""));
        suffix.visual.text = format!(" {level_text}");
        runs.push(suffix);
    }
    let curse = matches!(
        resource_id,
        "minecraft:binding_curse" | "minecraft:vanishing_curse"
    );
    HudInventoryTooltipLine {
        text,
        tint: if curse {
            [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0]
        } else {
            [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0]
        },
        runs,
    }
}

struct HudItemTooltipContent {
    style: Option<String>,
    lines: Vec<HudInventoryTooltipLine>,
}

fn hud_item_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
    advanced: bool,
    creative: bool,
) -> Option<HudItemTooltipContent> {
    let style = world.item_stack_tooltip_style(item).map(str::to_string);
    let effective_item = world.effective_item_stack(item);
    let item = effective_item.as_ref();
    let map_data = item
        .component_patch
        .map_id
        .and_then(|map_id| world.map_item(map_id))
        .map(|map| NativeItemMapTooltipData {
            scale: map.scale,
            locked: map.locked,
        });
    let enchantment_keys = world_enchantment_keys(world);
    let trim_material_context = world_trim_material_tooltip_context(world);
    let trim_pattern_context = world_trim_pattern_tooltip_context(world);
    let instrument_context = world_instrument_tooltip_context(world);
    let jukebox_song_context = world_jukebox_song_tooltip_context(world);
    let banner_pattern_context = world_banner_pattern_tooltip_context(world);
    let painting_variant_presentations = world_painting_variant_tooltip_context(world);
    let enchantment_definitions = world.enchantment_tooltip_definitions_in_holder_order();
    let enchantment_attribute_modifiers =
        world.enchantment_attribute_modifiers(&item.component_patch.enchantments);
    let attribute_keys = world_attribute_keys(world);
    let attack_damage_id = registry_key_id(
        attribute_keys.as_deref(),
        "minecraft:attack_damage",
        VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
    );
    let attack_speed_id = registry_key_id(
        attribute_keys.as_deref(),
        "minecraft:attack_speed",
        VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
    );
    let peaceful = world.difficulty().difficulty == "peaceful";
    item_runtime?
        .tooltip_lines_for_stack_with_context(
            item,
            NativeItemTooltipOptions {
                advanced,
                creative,
                peaceful,
                map_data,
                trim_material_keys: trim_material_context
                    .as_ref()
                    .map(|(keys, _)| keys.as_slice()),
                trim_material_presentations: trim_material_context
                    .as_ref()
                    .map(|(_, presentations)| presentations.as_slice()),
                trim_pattern_keys: trim_pattern_context
                    .as_ref()
                    .map(|(keys, _)| keys.as_slice()),
                trim_pattern_presentations: trim_pattern_context
                    .as_ref()
                    .map(|(_, presentations)| presentations.as_slice()),
                instrument_keys: instrument_context.as_ref().map(|(keys, _)| keys.as_slice()),
                instrument_presentations: instrument_context
                    .as_ref()
                    .map(|(_, presentations)| presentations.as_slice()),
                jukebox_song_keys: jukebox_song_context
                    .as_ref()
                    .map(|(keys, _)| keys.as_slice()),
                jukebox_song_presentations: jukebox_song_context
                    .as_ref()
                    .map(|(_, presentations)| presentations.as_slice()),
                banner_pattern_keys: banner_pattern_context
                    .as_ref()
                    .map(|(keys, _)| keys.as_slice()),
                banner_pattern_presentations: banner_pattern_context
                    .as_ref()
                    .map(|(_, presentations)| presentations.as_slice()),
                painting_variant_presentations: painting_variant_presentations.as_deref(),
                enchantment_keys: enchantment_keys.as_deref(),
                enchantment_definitions: enchantment_definitions.as_deref(),
                enchantment_attribute_modifiers: Some(&enchantment_attribute_modifiers),
                attribute_keys: attribute_keys.as_deref(),
                player_base_attack_damage: Some(
                    world
                        .local_player_attribute_base_value(attack_damage_id)
                        .unwrap_or(1.0),
                ),
                player_base_attack_speed: Some(
                    world
                        .local_player_attribute_base_value(attack_speed_id)
                        .unwrap_or(4.0),
                ),
                gamemaster_permission: world.local_player_has_gamemaster_permission(),
                item_disabled: !world.is_feature_enabled("minecraft:vanilla"),
                tooltip_tick_rate: Some(world.ticking().tick_rate),
            },
        )
        .map(|lines| HudItemTooltipContent {
            style,
            lines: lines
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: line.tint,
                    runs: line.runs,
                })
                .collect(),
        })
}

fn hud_inventory_floating_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    background: InventoryScreenBackground,
    main_offset_x: i32,
    stonecutter_recipe_scroll_row: Option<i32>,
    cursor_position: Option<(i32, i32)>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: &[i16],
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let mut items = match background {
        InventoryScreenBackground::Merchant => hud_merchant_trade_items(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
        ),
        InventoryScreenBackground::Stonecutter => hud_stonecutter_recipe_items(
            world,
            item_runtime,
            terrain_textures,
            stonecutter_recipe_scroll_row.unwrap_or_default(),
            shift_down,
            keybind_context,
            partial_tick,
        ),
        InventoryScreenBackground::Beacon => hud_beacon_example_payment_items(
            world,
            item_runtime,
            terrain_textures,
            keybind_context,
            partial_tick,
        ),
        _ => Vec::new(),
    };
    if background == InventoryScreenBackground::Merchant {
        items.extend(hud_merchant_trade_decorations(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
        ));
    }
    if main_offset_x != 0 {
        for item in &mut items {
            item.x += main_offset_x;
        }
    }
    push_hud_inventory_cursor_item(
        world,
        item_runtime,
        terrain_textures,
        cursor_position,
        quick_craft_button_num,
        quick_craft_slots,
        shift_down,
        keybind_context,
        partial_tick,
        &mut items,
    );
    items
}

fn hud_beacon_example_payment_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(item_runtime) = item_runtime else {
        return Vec::new();
    };
    let mut items = Vec::with_capacity(BEACON_EXAMPLE_PAYMENT_ITEMS.len());
    for (vanilla_item_id, x, y) in BEACON_EXAMPLE_PAYMENT_ITEMS {
        let Some(resource_id) =
            vanilla_item_type(vanilla_item_id).map(|item_type| item_type.resource_id)
        else {
            continue;
        };
        let Some(item_id) = item_runtime.item_protocol_id(resource_id) else {
            continue;
        };
        let stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        };
        let Some(icon) = hud_fake_item_icon_for_stack(
            world,
            Some(item_runtime),
            &stack,
            None,
            false,
            false,
            false,
            false,
            false,
            keybind_context,
            0,
            partial_tick,
        ) else {
            continue;
        };
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: false,
            block_model: fake_block_item_3d_model(
                world,
                &stack,
                Some(item_runtime),
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                false,
                keybind_context,
                0,
            ),
        });
    }
    items
}

fn push_hud_inventory_cursor_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    cursor_position: Option<(i32, i32)>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: &[i16],
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    items: &mut Vec<HudInventoryItem>,
) {
    let Some((cursor_x, cursor_y)) = cursor_position else {
        return;
    };
    let mut item = world.inventory().cursor_item.clone();
    if item_stack_is_empty(&item) {
        return;
    }
    let count_label_override =
        hud_inventory_quick_craft_cursor_count(world, quick_craft_button_num, quick_craft_slots)
            .and_then(|count| {
                item.count = count;
                (count == 0).then(|| HudItemCountLabel::new("0"))
            });
    let Some(icon) = hud_item_icon_for_stack(
        world,
        item_runtime,
        &item,
        None,
        false,
        false,
        true,
        false,
        shift_down,
        keybind_context,
        0,
        partial_tick,
    ) else {
        return;
    };
    let mut icon = icon;
    if let Some(count_label) = count_label_override {
        icon.count_label = Some(count_label);
    }
    let block_model = block_item_3d_model(
        world,
        &item,
        item_runtime,
        terrain_textures,
        None,
        false,
        false,
        true,
        false,
        shift_down,
        keybind_context,
        0,
    );
    items.push(HudInventoryItem {
        x: cursor_x - 8,
        y: cursor_y - 8,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model,
    });
}

fn hud_inventory_quick_craft_cursor_count(
    world: &WorldStore,
    button_num: Option<i8>,
    slots: &[i16],
) -> Option<i32> {
    if !world.local_inventory_is_open() || slots.len() <= 1 {
        return None;
    }
    let button_num = button_num?;
    if !matches!(button_num, 0 | 1) {
        return None;
    }
    let source = &world.inventory().cursor_item;
    if item_stack_is_empty(source) {
        return None;
    }

    let slot_count = i32::try_from(slots.len()).ok()?;
    let place_count = match button_num {
        0 => source.count / slot_count,
        1 => 1,
        _ => return None,
    };
    let mut remaining = source.count;
    for slot_num in slots {
        let Some(slot_item) = world
            .inventory()
            .inventory_menu
            .slots
            .iter()
            .find(|slot| slot.slot == *slot_num)
            .map(|slot| &slot.item)
        else {
            continue;
        };
        if !hud_inventory_quick_craft_slot_can_accept(world, *slot_num, slot_item, source) {
            continue;
        }

        let carry = if item_stack_is_empty(slot_item) {
            0
        } else {
            slot_item.count
        };
        let max_size = hud_inventory_slot_max_stack_size(world, *slot_num, source);
        let new_count = (place_count + carry).min(max_size);
        remaining -= new_count - carry;
    }

    Some(remaining.max(0))
}

fn hud_inventory_quick_craft_slot_can_accept(
    world: &WorldStore,
    slot_num: i16,
    slot_item: &ItemStackSummary,
    cursor: &ItemStackSummary,
) -> bool {
    if hud_inventory_slot_max_stack_size(world, slot_num, cursor) <= 0 {
        return false;
    }
    item_stack_is_empty(slot_item) || item_stacks_match_by_item_and_components(slot_item, cursor)
}

fn hud_inventory_slot_max_stack_size(
    world: &WorldStore,
    slot_num: i16,
    stack: &ItemStackSummary,
) -> i32 {
    let item_max_stack_size = hud_inventory_item_max_stack_size(world, stack);
    let slot_max_stack_size = match slot_num {
        0 => 0,
        5..=8 => 1,
        _ => 64,
    };
    item_max_stack_size.min(slot_max_stack_size)
}

fn hud_inventory_item_max_stack_size(world: &WorldStore, stack: &ItemStackSummary) -> i32 {
    if item_stack_is_empty(stack) {
        return 0;
    }
    if let Some(max_stack_size) = stack.component_patch.max_stack_size {
        return max_stack_size.clamp(1, 99);
    }
    if stack.component_patch.max_damage.is_some() || stack.component_patch.damage.is_some() {
        return 1;
    }
    stack
        .item_id
        .map(|item_id| world.item_max_stack_size_for_protocol_id(item_id))
        .unwrap_or(64)
}

fn item_stacks_match_by_item_and_components(
    left: &ItemStackSummary,
    right: &ItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

fn hud_inventory_background_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
    enchantment_options: Option<&[EnchantmentOptionHudState; ENCHANTMENT_OPTION_COUNT]>,
) -> Vec<HudInventoryBackgroundLayer> {
    match background {
        InventoryScreenBackground::LocalInventory => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Inventory,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Generic9xRows { rows } => {
            let top_height = u32::from(rows) * 18 + 17;
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GenericContainer,
                    0,
                    0,
                    176,
                    top_height,
                    [0.0, 0.0],
                    [176.0 / 256.0, top_height as f32 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GenericContainer,
                    0,
                    i32::try_from(top_height).unwrap_or_default(),
                    176,
                    96,
                    [0.0, 126.0 / 256.0],
                    [176.0 / 256.0, 222.0 / 256.0],
                ),
            ]
        }
        InventoryScreenBackground::Generic3x3 => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Dispenser,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::CraftingTable => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CraftingTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::EnchantmentTable => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_enchanting_table_state_layers(
                world,
                enchantment_options.map_or(&[], |options| options.as_slice()),
                &mut layers,
            );
            layers
        }
        InventoryScreenBackground::Crafter => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Crafter,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_crafter_state_layers(world, &mut layers);
            layers
        }
        InventoryScreenBackground::Anvil => {
            let mut layers = vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Anvil,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    if anvil_input_slot_has_item(world) {
                        HudInventoryBackgroundTexture::AnvilTextField
                    } else {
                        HudInventoryBackgroundTexture::AnvilTextFieldDisabled
                    },
                    59,
                    20,
                    110,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ];
            if anvil_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::AnvilError,
                    99,
                    45,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Beacon => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Beacon,
                0,
                0,
                230,
                219,
                [0.0, 0.0],
                [230.0 / 256.0, 219.0 / 256.0],
            )];
            let selection = local_state
                .beacon_effect_selection
                .unwrap_or_else(|| beacon_effect_selection_from_world(world));
            push_beacon_effect_button_layers(
                world,
                &mut layers,
                selection,
                local_state.cursor_position,
            );
            push_beacon_action_button_layers(
                world,
                &mut layers,
                selection,
                local_state.cursor_position,
            );
            layers
        }
        InventoryScreenBackground::BrewingStand => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStand,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_brewing_stand_progress_layers(world, &mut layers);
            layers
        }
        InventoryScreenBackground::CartographyTable => {
            cartography_table_presentation(world, item_runtime).background_layers
        }
        InventoryScreenBackground::BlastFurnace => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BlastFurnace,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::BlastFurnaceLitProgress,
                HudInventoryBackgroundTexture::BlastFurnaceBurnProgress,
            );
            layers
        }
        InventoryScreenBackground::Furnace => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Furnace,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::FurnaceLitProgress,
                HudInventoryBackgroundTexture::FurnaceBurnProgress,
            );
            layers
        }
        InventoryScreenBackground::Grindstone => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Grindstone,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if grindstone_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GrindstoneError,
                    92,
                    31,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Smithing => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Smithing,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if smithing_should_show_error(world) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::SmithingError,
                    65,
                    46,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Hopper => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Hopper,
                0,
                0,
                176,
                133,
                [0.0, 0.0],
                [176.0 / 256.0, 133.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Mount {
            kind,
            inventory_columns,
        } => {
            let equipment_slots = world
                .open_mount_equipment_slot_visibility()
                .unwrap_or_default();
            let mut layers = vec![hud_inventory_background_layer(
                match kind {
                    MountInventoryKind::Horse => HudInventoryBackgroundTexture::Horse,
                    MountInventoryKind::Nautilus => HudInventoryBackgroundTexture::Nautilus,
                },
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            if equipment_slots.saddle {
                layers.extend([
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::MountSlot,
                        7,
                        17,
                        18,
                        18,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::MountSaddleSlot,
                        8,
                        18,
                        16,
                        16,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ]);
            }
            if let Some(armor_slot) = equipment_slots.body {
                layers.extend([
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::MountSlot,
                        7,
                        35,
                        18,
                        18,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        mount_armor_slot_texture(armor_slot),
                        8,
                        36,
                        16,
                        16,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ]);
            }
            if inventory_columns > 0 {
                let chest_width = u32::from(inventory_columns) * 18;
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::MountChestSlots,
                    79,
                    17,
                    chest_width,
                    54,
                    [0.0, 0.0],
                    [chest_width as f32 / 90.0, 1.0],
                ));
            }
            layers
        }
        InventoryScreenBackground::Lectern => {
            unreachable!("lectern presentation is owned by HudBookScreen")
        }
        InventoryScreenBackground::Loom => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Loom,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Merchant => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Villager,
                0,
                0,
                276,
                166,
                [0.0, 0.0],
                [276.0 / 512.0, 166.0 / 256.0],
            )];
            push_merchant_trade_layers(
                world,
                &mut layers,
                local_state.cursor_position,
                local_state.merchant_focused_trade_row,
            );
            layers
        }
        InventoryScreenBackground::ShulkerBox => {
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::ShulkerBox,
                0,
                0,
                176,
                167,
                [0.0, 0.0],
                [176.0 / 256.0, 167.0 / 256.0],
            )]
        }
        InventoryScreenBackground::Smoker => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Smoker,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_furnace_progress_layers(
                world,
                &mut layers,
                HudInventoryBackgroundTexture::SmokerLitProgress,
                HudInventoryBackgroundTexture::SmokerBurnProgress,
            );
            layers
        }
        InventoryScreenBackground::Stonecutter => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Stonecutter,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_stonecutter_recipe_layers(
                world,
                &mut layers,
                local_state
                    .stonecutter_recipe_scroll_row
                    .unwrap_or_default(),
            );
            layers
        }
    }
}

fn mount_armor_slot_texture(kind: MountArmorSlotKind) -> HudInventoryBackgroundTexture {
    match kind {
        MountArmorSlotKind::Horse => HudInventoryBackgroundTexture::MountHorseArmorSlot,
        MountArmorSlotKind::Llama => HudInventoryBackgroundTexture::MountLlamaArmorSlot,
        MountArmorSlotKind::Nautilus => HudInventoryBackgroundTexture::MountNautilusArmorSlot,
    }
}

fn hud_merchant_trade_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let scroll_offset = merchant_scroll_offset(offers);
    for (row, offer) in offers
        .offers
        .iter()
        .skip(scroll_offset)
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
    {
        let row_y = MERCHANT_TRADE_ITEM_Y + row as i32 * MERCHANT_TRADE_ROW_HEIGHT;
        let cost_a = merchant_offer_cost_a_stack(world, offer);
        let base_cost_a = merchant_offer_base_cost_a_stack(offer);
        let cost_a_is_modified = cost_a.count != base_cost_a.count;
        push_merchant_trade_item(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
            &mut items,
            MERCHANT_TRADE_COST_A_X,
            row_y,
            cost_a,
            !cost_a_is_modified,
        );
        if let Some(cost_b) = offer.buy_b.as_ref() {
            push_merchant_trade_item(
                world,
                item_runtime,
                terrain_textures,
                shift_down,
                keybind_context,
                partial_tick,
                &mut items,
                MERCHANT_TRADE_COST_B_X,
                row_y,
                item_cost_stack(cost_b, cost_b.count),
                true,
            );
        }
        push_merchant_trade_item(
            world,
            item_runtime,
            terrain_textures,
            shift_down,
            keybind_context,
            partial_tick,
            &mut items,
            MERCHANT_TRADE_RESULT_X,
            row_y,
            offer.sell.clone(),
            true,
        );
    }
    items
}

fn push_merchant_trade_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    items: &mut Vec<HudInventoryItem>,
    x: i32,
    y: i32,
    item: ItemStackSummary,
    draw_decorations: bool,
) {
    if let Some(icon) = hud_fake_item_icon_for_stack(
        world,
        item_runtime,
        &item,
        None,
        false,
        false,
        false,
        false,
        shift_down,
        keybind_context,
        0,
        partial_tick,
    ) {
        let block_model = fake_block_item_3d_model(
            world,
            &item,
            item_runtime,
            terrain_textures,
            None,
            false,
            false,
            false,
            false,
            shift_down,
            keybind_context,
            0,
        );
        items.push(HudInventoryItem {
            x,
            y,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations,
            block_model,
        });
    }
}

fn hud_merchant_trade_decorations(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    let mut decorations = Vec::new();
    for (row, offer) in offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
    {
        let base_cost = merchant_offer_base_cost_a_stack(offer);
        let modified_cost = merchant_offer_cost_a_stack(world, offer);
        if base_cost.count == modified_cost.count {
            continue;
        }
        let y = MERCHANT_TRADE_ITEM_Y + row as i32 * MERCHANT_TRADE_ROW_HEIGHT;
        push_merchant_trade_decoration(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
            &mut decorations,
            MERCHANT_TRADE_COST_A_X,
            y,
            &base_cost,
        );
        push_merchant_trade_decoration(
            world,
            item_runtime,
            shift_down,
            keybind_context,
            partial_tick,
            &mut decorations,
            MERCHANT_TRADE_COST_A_X + 14,
            y,
            &modified_cost,
        );
    }
    decorations
}

#[allow(clippy::too_many_arguments)]
fn push_merchant_trade_decoration(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    decorations: &mut Vec<HudInventoryItem>,
    x: i32,
    y: i32,
    item: &ItemStackSummary,
) {
    let Some(mut icon) = hud_fake_item_icon_for_stack(
        world,
        item_runtime,
        item,
        None,
        false,
        false,
        false,
        false,
        shift_down,
        keybind_context,
        0,
        partial_tick,
    ) else {
        return;
    };
    icon.layers.clear();
    icon.foil = HudItemFoil::None;
    if item.count == 1 {
        icon.count_label = Some(HudItemCountLabel::new("1"));
    }
    decorations.push(HudInventoryItem {
        x,
        y,
        scale: 1.0,
        scale_y: 1.0,
        icon,
        scissor: None,
        draw_decorations: true,
        block_model: None,
    });
}

fn hud_stonecutter_recipe_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    scroll_row: i32,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let mut items = Vec::new();
    for option in stonecutter_visible_recipe_option_stacks(world, scroll_row) {
        if let Some(icon) = hud_item_icon_for_stack(
            world,
            item_runtime,
            &option.stack,
            None,
            false,
            false,
            false,
            false,
            shift_down,
            keybind_context,
            0,
            partial_tick,
        ) {
            let block_model = block_item_3d_model(
                world,
                &option.stack,
                item_runtime,
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                shift_down,
                keybind_context,
                0,
            );
            items.push(HudInventoryItem {
                x: option.x,
                y: option.y,
                scale: 1.0,
                scale_y: 1.0,
                icon,
                scissor: None,
                draw_decorations: true,
                block_model,
            });
        }
    }
    items
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StonecutterRecipeOptionStack {
    x: i32,
    y: i32,
    stack: ItemStackSummary,
}

fn stonecutter_visible_recipe_option_stacks(
    world: &WorldStore,
    scroll_row: i32,
) -> Vec<StonecutterRecipeOptionStack> {
    let recipes = stonecutter_visible_recipes(world);
    let start_index = stonecutter_recipe_start_index(recipes.len(), scroll_row).unwrap_or_default();
    recipes
        .into_iter()
        .skip(start_index)
        .take(STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT)
        .enumerate()
        .filter_map(|(position, recipe)| {
            let stack = world
                .resolve_slot_display_first_stack(&recipe.option_display)
                .ok()
                .flatten()?;
            let column = position as i32 % STONECUTTER_RECIPE_BUTTON_COLUMNS;
            let row = position as i32 / STONECUTTER_RECIPE_BUTTON_COLUMNS;
            Some(StonecutterRecipeOptionStack {
                x: STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH,
                y: STONECUTTER_RECIPE_ITEM_Y + row * STONECUTTER_RECIPE_BUTTON_HEIGHT,
                stack,
            })
        })
        .collect()
}

fn push_stonecutter_recipe_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    scroll_row: i32,
) {
    let visible_recipe_count = stonecutter_visible_recipes(world).len();
    let can_scroll = visible_recipe_count > STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT;
    layers.push(hud_inventory_background_layer(
        if can_scroll {
            HudInventoryBackgroundTexture::StonecutterScroller
        } else {
            HudInventoryBackgroundTexture::StonecutterScrollerDisabled
        },
        STONECUTTER_SCROLLER_X,
        STONECUTTER_SCROLLER_Y
            + if can_scroll {
                stonecutter_scroller_offset(visible_recipe_count, scroll_row)
            } else {
                0
            },
        u32::try_from(STONECUTTER_SCROLLER_WIDTH).unwrap_or_default(),
        u32::try_from(STONECUTTER_SCROLLER_HEIGHT).unwrap_or_default(),
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    let Some(start_index) = stonecutter_recipe_start_index(visible_recipe_count, scroll_row) else {
        return;
    };
    let selected_recipe = world
        .open_container_data_value(STONECUTTER_SELECTED_RECIPE_DATA_ID)
        .map(i32::from);
    for position in 0..STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT {
        let recipe_index = start_index + position;
        if recipe_index >= visible_recipe_count {
            break;
        }
        let column = position as i32 % STONECUTTER_RECIPE_BUTTON_COLUMNS;
        let row = position as i32 / STONECUTTER_RECIPE_BUTTON_COLUMNS;
        layers.push(hud_inventory_background_layer(
            if selected_recipe == Some(recipe_index as i32) {
                HudInventoryBackgroundTexture::StonecutterRecipeSelected
            } else {
                HudInventoryBackgroundTexture::StonecutterRecipe
            },
            STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH,
            STONECUTTER_RECIPE_BUTTON_SPRITE_Y + row * STONECUTTER_RECIPE_BUTTON_HEIGHT,
            u32::try_from(STONECUTTER_RECIPE_BUTTON_WIDTH).unwrap_or_default(),
            u32::try_from(STONECUTTER_RECIPE_BUTTON_HEIGHT).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
}

fn stonecutter_visible_recipes(
    world: &WorldStore,
) -> Vec<&bbb_protocol::packets::StonecutterSelectableRecipeSummary> {
    let Some(input_item_id) = stonecutter_input_item_id(world) else {
        return Vec::new();
    };
    world
        .recipes()
        .stonecutter_recipes
        .iter()
        .filter(|recipe| recipe.input.item_ids.contains(&input_item_id))
        .collect()
}

fn stonecutter_input_item_id(world: &WorldStore) -> Option<i32> {
    let item = world
        .inventory()
        .open_container
        .as_ref()?
        .slots
        .iter()
        .find(|slot| slot.slot == 0)
        .map(|slot| &slot.item)?;
    if item_stack_is_empty(item) {
        return None;
    }
    item.item_id
}

fn stonecutter_recipe_start_index(visible_recipe_count: usize, scroll_row: i32) -> Option<usize> {
    if visible_recipe_count == 0 {
        return None;
    }
    let row = stonecutter_clamped_recipe_scroll_row(visible_recipe_count, scroll_row);
    usize::try_from(row * STONECUTTER_RECIPE_BUTTON_COLUMNS).ok()
}

fn stonecutter_clamped_recipe_scroll_row(visible_recipe_count: usize, scroll_row: i32) -> i32 {
    scroll_row.clamp(0, stonecutter_recipe_max_scroll_row(visible_recipe_count))
}

fn stonecutter_recipe_max_scroll_row(visible_recipe_count: usize) -> i32 {
    (stonecutter_recipe_row_count(visible_recipe_count) - STONECUTTER_RECIPE_BUTTON_ROWS).max(0)
}

fn stonecutter_recipe_row_count(visible_recipe_count: usize) -> i32 {
    if visible_recipe_count == 0 {
        0
    } else {
        ((visible_recipe_count as i32) + STONECUTTER_RECIPE_BUTTON_COLUMNS - 1)
            / STONECUTTER_RECIPE_BUTTON_COLUMNS
    }
}

fn stonecutter_scroller_offset(visible_recipe_count: usize, scroll_row: i32) -> i32 {
    let max_scroll_row = stonecutter_recipe_max_scroll_row(visible_recipe_count);
    if max_scroll_row <= 0 {
        return 0;
    }
    let scroll_offs = stonecutter_clamped_recipe_scroll_row(visible_recipe_count, scroll_row)
        as f32
        / max_scroll_row as f32;
    (STONECUTTER_SCROLLER_MAX_OFFSET as f32 * scroll_offs) as i32
}

fn push_merchant_trade_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    cursor_position: Option<(i32, i32)>,
    focused_trade_row: Option<i32>,
) {
    let Some(offers) = merchant_offers_state(world) else {
        return;
    };
    if offers.offers.is_empty() {
        return;
    }

    let visible_offers = offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate();
    for (row, _) in visible_offers.clone() {
        let row = i32::try_from(row).unwrap_or_default();
        let row_y = MERCHANT_TRADE_BUTTON_Y + row * MERCHANT_TRADE_ROW_HEIGHT;
        let highlighted = focused_trade_row == Some(row)
            || cursor_position.is_some_and(|(cursor_x, cursor_y)| {
                cursor_x >= MERCHANT_TRADE_BUTTON_X
                    && cursor_x < MERCHANT_TRADE_BUTTON_X + MERCHANT_TRADE_BUTTON_WIDTH as i32
                    && cursor_y >= row_y
                    && cursor_y < row_y + MERCHANT_TRADE_BUTTON_HEIGHT as i32
            });
        layers.push(hud_inventory_background_layer(
            if highlighted {
                HudInventoryBackgroundTexture::WidgetButtonHighlighted
            } else {
                HudInventoryBackgroundTexture::WidgetButton
            },
            MERCHANT_TRADE_BUTTON_X,
            row_y,
            MERCHANT_TRADE_BUTTON_WIDTH,
            MERCHANT_TRADE_BUTTON_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    for (row, offer) in visible_offers {
        let row = i32::try_from(row).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            if offer.is_out_of_stock {
                HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock
            } else {
                HudInventoryBackgroundTexture::VillagerTradeArrow
            },
            MERCHANT_TRADE_ARROW_X,
            MERCHANT_TRADE_ARROW_Y + row * MERCHANT_TRADE_ROW_HEIGHT,
            MERCHANT_TRADE_ARROW_WIDTH,
            MERCHANT_TRADE_ARROW_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    layers.push(hud_inventory_background_layer(
        if offers.offers.len() > MERCHANT_VISIBLE_OFFER_COUNT {
            HudInventoryBackgroundTexture::VillagerScroller
        } else {
            HudInventoryBackgroundTexture::VillagerScrollerDisabled
        },
        MERCHANT_SCROLLER_X,
        MERCHANT_SCROLLER_Y + merchant_scroller_offset(offers, offers.local_scroll_offset),
        MERCHANT_SCROLLER_WIDTH,
        MERCHANT_SCROLLER_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    if merchant_selected_offer(offers).is_some_and(|offer| offer.is_out_of_stock) {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerOutOfStock,
            MERCHANT_OUT_OF_STOCK_X,
            MERCHANT_OUT_OF_STOCK_Y,
            MERCHANT_OUT_OF_STOCK_WIDTH,
            MERCHANT_OUT_OF_STOCK_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    if offers.show_progress && offers.villager_level < 5 {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::VillagerExperienceBarBackground,
            MERCHANT_XP_BAR_X,
            MERCHANT_XP_BAR_Y,
            MERCHANT_XP_BAR_WIDTH,
            MERCHANT_XP_BAR_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        if let Some(current_width) = merchant_xp_current_width(offers) {
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerExperienceBarCurrent,
                MERCHANT_XP_BAR_X,
                MERCHANT_XP_BAR_Y,
                current_width,
                MERCHANT_XP_BAR_HEIGHT,
                [0.0, 0.0],
                [current_width as f32 / MERCHANT_XP_BAR_WIDTH as f32, 1.0],
            ));
            if let Some(result_width) = merchant_xp_result_width(world, offers, current_width) {
                layers.push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::VillagerExperienceBarResult,
                    MERCHANT_XP_BAR_X + current_width as i32,
                    MERCHANT_XP_BAR_Y,
                    result_width,
                    MERCHANT_XP_BAR_HEIGHT,
                    [current_width as f32 / MERCHANT_XP_BAR_WIDTH as f32, 0.0],
                    [
                        (current_width + result_width) as f32 / MERCHANT_XP_BAR_WIDTH as f32,
                        1.0,
                    ],
                ));
            }
        }
    }
}

fn hud_merchant_discount_layers(world: &WorldStore) -> Vec<HudInventoryBackgroundLayer> {
    let Some(offers) = merchant_offers_state(world) else {
        return Vec::new();
    };
    offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
        .filter(|(_, offer)| {
            merchant_offer_base_cost_a_stack(offer).count
                != merchant_offer_cost_a_stack(world, offer).count
        })
        .map(|(row, _)| {
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerDiscountStrikethrough,
                MERCHANT_DISCOUNT_STRIKETHROUGH_X,
                MERCHANT_DISCOUNT_STRIKETHROUGH_Y
                    + i32::try_from(row).unwrap_or_default() * MERCHANT_TRADE_ROW_HEIGHT,
                MERCHANT_DISCOUNT_STRIKETHROUGH_WIDTH,
                MERCHANT_DISCOUNT_STRIKETHROUGH_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        })
        .collect()
}

fn merchant_offers_state(world: &WorldStore) -> Option<&MerchantOffersState> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.merchant_offers.as_ref())
}

fn merchant_selected_offer(offers: &MerchantOffersState) -> Option<&MerchantOfferState> {
    usize::try_from(offers.local_selected_offer_index)
        .ok()
        .and_then(|index| offers.offers.get(index))
}

fn merchant_scroll_offset(offers: &MerchantOffersState) -> usize {
    let max_scroll_offset = offers
        .offers
        .len()
        .saturating_sub(MERCHANT_VISIBLE_OFFER_COUNT);
    usize::try_from(offers.local_scroll_offset)
        .unwrap_or_default()
        .min(max_scroll_offset)
}

fn merchant_offer_cost_a_stack(world: &WorldStore, offer: &MerchantOfferState) -> ItemStackSummary {
    let max_stack_size = offer
        .buy_a
        .component_predicate
        .components
        .max_stack_size
        .unwrap_or_else(|| world.item_max_stack_size_for_protocol_id(offer.buy_a.item_id))
        .clamp(
            1,
            bbb_protocol::item_types::VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE,
        );
    let demand_diff = (offer.buy_a.count as f32 * offer.demand as f32 * offer.price_multiplier)
        .floor()
        .max(0.0) as i32;
    let count = offer
        .buy_a
        .count
        .wrapping_add(demand_diff)
        .wrapping_add(offer.special_price_diff)
        .clamp(1, max_stack_size);
    item_cost_stack(&offer.buy_a, count)
}

fn merchant_offer_base_cost_a_stack(offer: &MerchantOfferState) -> ItemStackSummary {
    item_cost_stack(&offer.buy_a, offer.buy_a.count)
}

fn item_cost_stack(cost: &ItemCostSummary, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(cost.item_id),
        count,
        component_patch: cost.component_predicate.display_patch(),
    }
}

fn merchant_scroller_offset(offers: &MerchantOffersState, scroll_offset: i32) -> i32 {
    let steps = offers.offers.len() as i32 + 1 - MERCHANT_VISIBLE_OFFER_COUNT as i32;
    if steps <= 1 {
        return 0;
    }
    let left_over = MERCHANT_SCROLLER_TRACK_HEIGHT
        - (MERCHANT_SCROLLER_HEIGHT as i32 + (steps - 1) * MERCHANT_SCROLLER_TRACK_HEIGHT / steps);
    let step_height = 1 + left_over / steps + MERCHANT_SCROLLER_TRACK_HEIGHT / steps;
    let scroller_offset = (scroll_offset * step_height).min(MERCHANT_SCROLLER_MAX_OFFSET);
    if scroll_offset == steps - 1 {
        MERCHANT_SCROLLER_MAX_OFFSET
    } else {
        scroller_offset
    }
}

fn merchant_xp_current_width(offers: &MerchantOffersState) -> Option<u32> {
    let level = usize::try_from(offers.villager_level).ok()?;
    if !(1..5).contains(&level) {
        return None;
    }
    let min_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level - 1];
    let max_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level];
    if offers.villager_xp < min_xp || max_xp <= min_xp {
        return None;
    }
    let multiplier = MERCHANT_XP_BAR_WIDTH as f32 / (max_xp - min_xp) as f32;
    let width = (multiplier * (offers.villager_xp - min_xp) as f32).floor() as u32;
    Some(width.min(MERCHANT_XP_BAR_WIDTH))
}

fn merchant_xp_result_width(
    world: &WorldStore,
    offers: &MerchantOffersState,
    current_width: u32,
) -> Option<u32> {
    let offer = merchant_selected_offer(offers)?;
    if offer.is_out_of_stock || offer.xp <= 0 {
        return None;
    }
    let result = open_container_slot_item(world, 2)?;
    if item_stack_is_empty(result) || result != &offer.sell {
        return None;
    }
    let level = usize::try_from(offers.villager_level).ok()?;
    if !(1..5).contains(&level) {
        return None;
    }
    let min_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level - 1];
    let max_xp = VILLAGER_NEXT_LEVEL_XP_THRESHOLDS[level];
    if max_xp <= min_xp {
        return None;
    }
    let multiplier = MERCHANT_XP_BAR_WIDTH as f32 / (max_xp - min_xp) as f32;
    let width = (offer.xp as f32 * multiplier).floor() as u32;
    Some(width.min(MERCHANT_XP_BAR_WIDTH.saturating_sub(current_width))).filter(|width| *width > 0)
}

fn anvil_input_slot_has_item(world: &WorldStore) -> bool {
    open_container_slot_has_item(world, 0)
}

fn push_beacon_action_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
    cursor_position: Option<(i32, i32)>,
) {
    let confirm_active = beacon_confirm_button_is_active(world, selection);
    let confirm_texture = if !confirm_active {
        HudInventoryBackgroundTexture::BeaconButtonDisabled
    } else if beacon_button_contains(
        cursor_position,
        BEACON_CONFIRM_BUTTON_X,
        BEACON_ACTION_BUTTON_Y,
    ) {
        HudInventoryBackgroundTexture::BeaconButtonHighlighted
    } else {
        HudInventoryBackgroundTexture::BeaconButton
    };
    push_beacon_action_button_layer(
        layers,
        confirm_texture,
        HudInventoryBackgroundTexture::BeaconConfirm,
        BEACON_CONFIRM_BUTTON_X,
    );
    push_beacon_action_button_layer(
        layers,
        if beacon_button_contains(
            cursor_position,
            BEACON_CANCEL_BUTTON_X,
            BEACON_ACTION_BUTTON_Y,
        ) {
            HudInventoryBackgroundTexture::BeaconButtonHighlighted
        } else {
            HudInventoryBackgroundTexture::BeaconButton
        },
        HudInventoryBackgroundTexture::BeaconCancel,
        BEACON_CANCEL_BUTTON_X,
    );
}

fn push_beacon_effect_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
    cursor_position: Option<(i32, i32)>,
) {
    let levels = beacon_levels(world);
    for button in beacon_effect_buttons(selection.0) {
        let selected = if button.primary {
            selection.0 == Some(button.effect_id)
        } else {
            selection.1 == Some(button.effect_id)
        };
        let button_texture = if button.tier >= levels {
            HudInventoryBackgroundTexture::BeaconButtonDisabled
        } else if selected {
            HudInventoryBackgroundTexture::BeaconButtonSelected
        } else if beacon_button_contains(cursor_position, button.x, button.y) {
            HudInventoryBackgroundTexture::BeaconButtonHighlighted
        } else {
            HudInventoryBackgroundTexture::BeaconButton
        };
        layers.push(hud_inventory_background_layer(
            button_texture,
            button.x,
            button.y,
            BEACON_EFFECT_BUTTON_SIZE,
            BEACON_EFFECT_BUTTON_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        if let Some(icon_texture) = beacon_effect_icon_texture(button.effect_id) {
            layers.push(hud_inventory_background_layer(
                icon_texture,
                button.x + BEACON_ACTION_ICON_OFFSET,
                button.y + BEACON_ACTION_ICON_OFFSET,
                BEACON_ACTION_ICON_SIZE,
                BEACON_ACTION_ICON_SIZE,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
        }
    }
}

fn beacon_effect_buttons(primary_effect: Option<i32>) -> Vec<BeaconEffectButton> {
    let mut buttons = Vec::with_capacity(7);
    for (tier, effects) in VANILLA_BEACON_EFFECT_TIERS[..3].iter().enumerate() {
        let total_width = effects.len() as i32 * BEACON_EFFECT_BUTTON_SIZE as i32
            + (effects.len() as i32 - 1) * 2;
        for (column, effect_id) in effects.iter().enumerate() {
            buttons.push(BeaconEffectButton {
                primary: true,
                upgrade: false,
                tier: i16::try_from(tier).unwrap_or_default(),
                effect_id: *effect_id,
                x: BEACON_PRIMARY_EFFECT_CENTER_X
                    + i32::try_from(column).unwrap_or_default() * BEACON_EFFECT_BUTTON_SPACING
                    - total_width / 2,
                y: BEACON_PRIMARY_EFFECT_Y
                    + i32::try_from(tier).unwrap_or_default() * BEACON_PRIMARY_EFFECT_ROW_SPACING,
            });
        }
    }

    let secondary_effects = VANILLA_BEACON_EFFECT_TIERS[3];
    let count = secondary_effects.len() as i32 + 1;
    let total_width = count * BEACON_EFFECT_BUTTON_SIZE as i32 + (count - 1) * 2;
    for (column, effect_id) in secondary_effects.iter().enumerate() {
        buttons.push(BeaconEffectButton {
            primary: false,
            upgrade: false,
            tier: 3,
            effect_id: *effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(column).unwrap_or_default() * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    if let Some(effect_id) = primary_effect.filter(|effect_id| beacon_primary_effect(*effect_id)) {
        buttons.push(BeaconEffectButton {
            primary: false,
            upgrade: true,
            tier: 3,
            effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(secondary_effects.len()).unwrap_or_default()
                    * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    buttons
}

fn beacon_primary_effect(effect_id: i32) -> bool {
    VANILLA_BEACON_EFFECT_TIERS[..3]
        .iter()
        .copied()
        .flatten()
        .any(|candidate| *candidate == effect_id)
}

fn beacon_button_contains(cursor_position: Option<(i32, i32)>, x: i32, y: i32) -> bool {
    let Some((cursor_x, cursor_y)) = cursor_position else {
        return false;
    };
    let size = i32::try_from(BEACON_EFFECT_BUTTON_SIZE).unwrap_or_default();
    cursor_x >= x && cursor_x < x + size && cursor_y >= y && cursor_y < y + size
}

fn beacon_effect_icon_texture(effect_id: i32) -> Option<HudInventoryBackgroundTexture> {
    VANILLA_BEACON_EFFECT_TIERS
        .iter()
        .copied()
        .flatten()
        .any(|candidate| *candidate == effect_id)
        .then_some(HudInventoryBackgroundTexture::MobEffect(effect_id))
}

fn push_beacon_action_button_layer(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    button_texture: HudInventoryBackgroundTexture,
    icon_texture: HudInventoryBackgroundTexture,
    x: i32,
) {
    layers.push(hud_inventory_background_layer(
        button_texture,
        x,
        BEACON_ACTION_BUTTON_Y,
        BEACON_ACTION_BUTTON_SIZE,
        BEACON_ACTION_BUTTON_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    layers.push(hud_inventory_background_layer(
        icon_texture,
        x + BEACON_ACTION_ICON_OFFSET,
        BEACON_ACTION_BUTTON_Y + BEACON_ACTION_ICON_OFFSET,
        BEACON_ACTION_ICON_SIZE,
        BEACON_ACTION_ICON_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn beacon_confirm_button_is_active(
    world: &WorldStore,
    selection: (Option<i32>, Option<i32>),
) -> bool {
    open_container_slot_has_item(world, 0) && selection.0.is_some_and(beacon_primary_effect)
}

fn beacon_effect_selection_from_world(world: &WorldStore) -> (Option<i32>, Option<i32>) {
    (
        beacon_data_effect_id(world, BEACON_PRIMARY_EFFECT_DATA_ID),
        beacon_data_effect_id(world, BEACON_SECONDARY_EFFECT_DATA_ID),
    )
}

fn beacon_data_effect_id(world: &WorldStore, data_id: i16) -> Option<i32> {
    let value = world.open_container_data_value(data_id)?;
    (value > 0).then_some(i32::from(value) - 1)
}

fn beacon_levels(world: &WorldStore) -> i16 {
    world
        .open_container_data_value(BEACON_LEVELS_DATA_ID)
        .unwrap_or_default()
}

fn push_crafter_state_layers(world: &WorldStore, layers: &mut Vec<HudInventoryBackgroundLayer>) {
    for slot in 0..CRAFTER_GRID_SLOT_COUNT {
        if world.open_container_data_value(slot).unwrap_or_default() != 1 {
            continue;
        }
        let x = i32::from(slot % 3);
        let y = i32::from(slot / 3);
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CrafterDisabledSlot,
            25 + x * 18,
            16 + y * 18,
            CRAFTER_DISABLED_SLOT_SPRITE_SIZE,
            CRAFTER_DISABLED_SLOT_SPRITE_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let redstone_texture = if world
        .open_container_data_value(CRAFTER_POWERED_DATA_ID)
        .unwrap_or_default()
        == 1
    {
        HudInventoryBackgroundTexture::CrafterPoweredRedstone
    } else {
        HudInventoryBackgroundTexture::CrafterUnpoweredRedstone
    };
    layers.push(hud_inventory_background_layer(
        redstone_texture,
        97,
        35,
        CRAFTER_REDSTONE_SPRITE_SIZE,
        CRAFTER_REDSTONE_SPRITE_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn push_enchanting_table_state_layers(
    world: &WorldStore,
    options: &[EnchantmentOptionHudState],
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    if open_container_slot_has_item(world, 1) {
        for slot in enchanting_table_option_layers(options) {
            layers.push(slot);
        }
        return;
    }
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::EnchantingTableLapisSlot,
        35,
        47,
        ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE,
        ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
    for slot in enchanting_table_option_layers(options) {
        layers.push(slot);
    }
}

fn enchanting_table_option_layers(
    options: &[EnchantmentOptionHudState],
) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::with_capacity(options.len() * 2);
    for option in options {
        let index = i16::try_from(option.index).expect("three enchanting rows fit i16");
        let y = ENCHANTING_TABLE_OPTION_Y + i32::from(index) * ENCHANTING_TABLE_OPTION_SPACING;
        if option.cost <= 0 {
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                ENCHANTING_TABLE_OPTION_X,
                y,
                ENCHANTING_TABLE_OPTION_WIDTH,
                ENCHANTING_TABLE_OPTION_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
            continue;
        }

        layers.push(hud_inventory_background_layer(
            if !option.enabled {
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled
            } else if option.highlighted {
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotHighlighted
            } else {
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlot
            },
            ENCHANTING_TABLE_OPTION_X,
            y,
            ENCHANTING_TABLE_OPTION_WIDTH,
            ENCHANTING_TABLE_OPTION_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        layers.push(hud_inventory_background_layer(
            enchanting_table_level_texture(index, option.enabled),
            ENCHANTING_TABLE_OPTION_X + ENCHANTING_TABLE_LEVEL_ICON_X_OFFSET,
            y + ENCHANTING_TABLE_LEVEL_ICON_Y_OFFSET,
            ENCHANTING_TABLE_LEVEL_ICON_SIZE,
            ENCHANTING_TABLE_LEVEL_ICON_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn enchanting_table_level_texture(index: i16, enabled: bool) -> HudInventoryBackgroundTexture {
    match (index, enabled) {
        (0, true) => HudInventoryBackgroundTexture::EnchantingTableLevel1,
        (1, true) => HudInventoryBackgroundTexture::EnchantingTableLevel2,
        (_, true) => HudInventoryBackgroundTexture::EnchantingTableLevel3,
        (0, false) => HudInventoryBackgroundTexture::EnchantingTableLevel1Disabled,
        (1, false) => HudInventoryBackgroundTexture::EnchantingTableLevel2Disabled,
        (_, false) => HudInventoryBackgroundTexture::EnchantingTableLevel3Disabled,
    }
}

fn cartography_table_presentation(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> CartographyTablePresentation {
    let mut presentation = CartographyTablePresentation {
        background_layers: vec![hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CartographyTable,
            0,
            0,
            176,
            166,
            [0.0, 0.0],
            [176.0 / 256.0, 166.0 / 256.0],
        )],
        map_previews: Vec::new(),
        mode: None,
        invalid_transform: false,
    };
    let Some(input_map) = open_container_slot_item(world, CARTOGRAPHY_TABLE_MAP_SLOT) else {
        return presentation;
    };
    let Some(map_id) = item_stack_map_id(input_map) else {
        return presentation;
    };

    let additional_item = open_container_slot_item(world, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT);
    let additional =
        additional_item.and_then(|item| cartography_additional_item(world, item_runtime, item));
    let map_state = world.map_item(map_id);
    let result_slot_mode = open_container_slot_item(world, CARTOGRAPHY_TABLE_RESULT_SLOT)
        .and_then(cartography_result_mode);
    let invalid_transform = match additional {
        Some(CartographyAdditionalItemKind::Paper) => {
            map_state.is_some_and(|map| map.locked || map.scale >= 4)
        }
        Some(CartographyAdditionalItemKind::GlassPane) => map_state.is_some_and(|map| map.locked),
        _ => false,
    };

    if invalid_transform {
        presentation
            .background_layers
            .push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTableError,
                CARTOGRAPHY_TABLE_ERROR_X,
                CARTOGRAPHY_TABLE_ERROR_Y,
                CARTOGRAPHY_TABLE_ERROR_WIDTH,
                CARTOGRAPHY_TABLE_ERROR_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
    }

    let mode = match additional {
        Some(CartographyAdditionalItemKind::Paper) if invalid_transform => {
            CartographyResultMode::Map
        }
        Some(CartographyAdditionalItemKind::Paper) => CartographyResultMode::Scaled,
        Some(CartographyAdditionalItemKind::EmptyMap) => CartographyResultMode::Duplicated,
        Some(CartographyAdditionalItemKind::GlassPane) => CartographyResultMode::Locked,
        None => result_slot_mode.unwrap_or(CartographyResultMode::Map),
    };
    presentation.mode = Some(mode);
    presentation.invalid_transform = invalid_transform;
    push_cartography_table_mode_layers(&mut presentation, map_state.map(|_| map_id), mode);
    presentation
}

fn push_cartography_table_mode_layers(
    presentation: &mut CartographyTablePresentation,
    map_id: Option<i32>,
    mode: CartographyResultMode,
) {
    match mode {
        CartographyResultMode::Map => {
            push_cartography_table_map_layer(
                &mut presentation.background_layers,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y,
            );
            push_cartography_table_map_preview(presentation, map_id, 71, 17, 0.45);
        }
        CartographyResultMode::Scaled => {
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableScaledMap,
                    CARTOGRAPHY_TABLE_MAP_X,
                    CARTOGRAPHY_TABLE_MAP_Y,
                    CARTOGRAPHY_TABLE_MAP_SIZE,
                    CARTOGRAPHY_TABLE_MAP_SIZE,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 85, 31, 0.226);
        }
        CartographyResultMode::Duplicated => {
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                    CARTOGRAPHY_TABLE_MAP_X + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                    CARTOGRAPHY_TABLE_MAP_Y,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 86, 16, 0.34);
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                    CARTOGRAPHY_TABLE_MAP_X,
                    CARTOGRAPHY_TABLE_MAP_Y + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                    CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
            push_cartography_table_map_preview(presentation, map_id, 70, 32, 0.34);
        }
        CartographyResultMode::Locked => {
            push_cartography_table_map_layer(
                &mut presentation.background_layers,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y,
            );
            push_cartography_table_map_preview(presentation, map_id, 71, 17, 0.45);
            presentation
                .background_layers
                .push(hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableLocked,
                    CARTOGRAPHY_TABLE_LOCKED_X,
                    CARTOGRAPHY_TABLE_LOCKED_Y,
                    CARTOGRAPHY_TABLE_LOCKED_WIDTH,
                    CARTOGRAPHY_TABLE_LOCKED_HEIGHT,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ));
        }
    }
}

fn push_cartography_table_map_preview(
    presentation: &mut CartographyTablePresentation,
    map_id: Option<i32>,
    x: i32,
    y: i32,
    scale: f32,
) {
    let Some(map_id) = map_id else {
        return;
    };
    presentation.map_previews.push(HudInventoryMapPreview {
        map_id,
        x,
        y,
        scale,
        after_background_layer: presentation.background_layers.len().saturating_sub(1),
    });
}

fn push_cartography_table_map_layer(layers: &mut Vec<HudInventoryBackgroundLayer>, x: i32, y: i32) {
    layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::CartographyTableMap,
        x,
        y,
        CARTOGRAPHY_TABLE_MAP_SIZE,
        CARTOGRAPHY_TABLE_MAP_SIZE,
        [0.0, 0.0],
        [1.0, 1.0],
    ));
}

fn cartography_result_mode(item: &ItemStackSummary) -> Option<CartographyResultMode> {
    if item_stack_is_empty(item) || item_stack_map_id(item).is_none() {
        return None;
    }
    match item.component_patch.map_post_processing {
        Some(MapPostProcessingSummary::Scale) => Some(CartographyResultMode::Scaled),
        Some(MapPostProcessingSummary::Lock) => Some(CartographyResultMode::Locked),
        None if item.count >= 2 => Some(CartographyResultMode::Duplicated),
        None => Some(CartographyResultMode::Map),
    }
}

fn cartography_additional_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
) -> Option<CartographyAdditionalItemKind> {
    if item_stack_is_empty(item) {
        return None;
    }
    let item_id = item.item_id?;
    world.cartography_additional_item_kind(item_id).or_else(|| {
        cartography_additional_item_for_resource_id(
            item_runtime?.item_resource_id_for_protocol_id(item_id)?,
        )
    })
}

fn cartography_additional_item_for_resource_id(
    resource_id: &str,
) -> Option<CartographyAdditionalItemKind> {
    match resource_id {
        "minecraft:paper" => Some(CartographyAdditionalItemKind::Paper),
        "minecraft:map" => Some(CartographyAdditionalItemKind::EmptyMap),
        "minecraft:glass_pane" => Some(CartographyAdditionalItemKind::GlassPane),
        _ => None,
    }
}

fn item_stack_map_id(item: &ItemStackSummary) -> Option<i32> {
    if item_stack_is_empty(item)
        || item
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
    {
        return None;
    }
    item.component_patch.map_id
}

fn anvil_should_show_error(world: &WorldStore) -> bool {
    (open_container_slot_has_item(world, 0) || open_container_slot_has_item(world, 1))
        && !open_container_slot_has_item(world, 2)
}

fn open_container_slot_has_item(world: &WorldStore, slot_num: i16) -> bool {
    open_container_slot_item(world, slot_num).is_some_and(|item| !item_stack_is_empty(item))
}

fn open_container_slot_item(world: &WorldStore, slot_num: i16) -> Option<&ItemStackSummary> {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| container.slots.iter().find(|slot| slot.slot == slot_num))
        .map(|slot| &slot.item)
}

fn push_brewing_stand_progress_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    let fuel = world
        .open_container_data_value(BREWING_STAND_FUEL_DATA_ID)
        .unwrap_or_default();
    let max_fuel_length = i32::try_from(BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH).unwrap_or_default();
    let fuel_length = ((max_fuel_length * i32::from(fuel) + 20 - 1) / 20).clamp(0, max_fuel_length);
    if fuel_length > 0 {
        let width = u32::try_from(fuel_length).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BrewingStandFuelLength,
            60,
            44,
            width,
            BREWING_STAND_FUEL_LENGTH_SPRITE_HEIGHT,
            [0.0, 0.0],
            [
                width as f32 / BREWING_STAND_FUEL_LENGTH_SPRITE_WIDTH as f32,
                1.0,
            ],
        ));
    }

    let brew_ticks = world
        .open_container_data_value(BREWING_STAND_BREW_TIME_DATA_ID)
        .unwrap_or_default();
    if brew_ticks <= 0 {
        return;
    }

    let brew_length = (BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT as f32
        * (1.0 - f32::from(brew_ticks) / BREWING_STAND_BREW_TOTAL_TICKS))
        as i32;
    if brew_length > 0 {
        let height = u32::try_from(brew_length).unwrap_or_default();
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BrewingStandBrewProgress,
            97,
            16,
            BREWING_STAND_BREW_PROGRESS_SPRITE_WIDTH,
            height,
            [0.0, 0.0],
            [
                1.0,
                height as f32 / BREWING_STAND_BREW_PROGRESS_SPRITE_HEIGHT as f32,
            ],
        ));
    }

    let bubble_length =
        BREWING_STAND_BUBBLE_LENGTHS[(usize::try_from(brew_ticks).unwrap_or_default() / 2)
            % BREWING_STAND_BUBBLE_LENGTHS.len()];
    if bubble_length > 0 {
        let y_offset = BREWING_STAND_BUBBLES_SPRITE_HEIGHT - bubble_length;
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::BrewingStandBubbles,
            63,
            14 + i32::try_from(y_offset).unwrap_or_default(),
            BREWING_STAND_BUBBLES_SPRITE_WIDTH,
            bubble_length,
            [
                0.0,
                y_offset as f32 / BREWING_STAND_BUBBLES_SPRITE_HEIGHT as f32,
            ],
            [1.0, 1.0],
        ));
    }
}

fn grindstone_should_show_error(world: &WorldStore) -> bool {
    let Some(container) = world.inventory().open_container.as_ref() else {
        return false;
    };
    let input_has_item = [0, 1].into_iter().any(|slot_num| {
        container
            .slots
            .iter()
            .find(|slot| slot.slot == slot_num)
            .is_some_and(|slot| !item_stack_is_empty(&slot.item))
    });
    let result_has_item = container
        .slots
        .iter()
        .find(|slot| slot.slot == 2)
        .is_some_and(|slot| !item_stack_is_empty(&slot.item));
    input_has_item && !result_has_item
}

fn smithing_should_show_error(world: &WorldStore) -> bool {
    world
        .open_container_data_value(SMITHING_RECIPE_ERROR_DATA_ID)
        .unwrap_or_default()
        > 0
}

fn push_furnace_progress_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    lit_texture: HudInventoryBackgroundTexture,
    burn_texture: HudInventoryBackgroundTexture,
) {
    let lit_time = world
        .open_container_data_value(FURNACE_LIT_TIME_DATA_ID)
        .unwrap_or_default();
    if lit_time > 0 {
        let lit_duration = world
            .open_container_data_value(FURNACE_LIT_DURATION_DATA_ID)
            .filter(|duration| *duration != 0)
            .unwrap_or(FURNACE_DEFAULT_LIT_DURATION);
        let lit_progress = (f32::from(lit_time) / f32::from(lit_duration)).clamp(0.0, 1.0);
        let lit_height = (lit_progress * 13.0).ceil() as u32 + 1;
        let lit_offset = FURNACE_LIT_PROGRESS_SPRITE_SIZE - lit_height;
        layers.push(hud_inventory_background_layer(
            lit_texture,
            56,
            36 + i32::try_from(lit_offset).unwrap_or_default(),
            FURNACE_LIT_PROGRESS_SPRITE_SIZE,
            lit_height,
            [
                0.0,
                lit_offset as f32 / FURNACE_LIT_PROGRESS_SPRITE_SIZE as f32,
            ],
            [1.0, 1.0],
        ));
    }

    let burn_current = world
        .open_container_data_value(FURNACE_COOKING_PROGRESS_DATA_ID)
        .unwrap_or_default();
    let burn_total = world
        .open_container_data_value(FURNACE_COOKING_TOTAL_TIME_DATA_ID)
        .unwrap_or_default();
    let burn_progress = if burn_total != 0 && burn_current != 0 {
        (f32::from(burn_current) / f32::from(burn_total)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let burn_width = (burn_progress * FURNACE_BURN_PROGRESS_SPRITE_WIDTH as f32).ceil() as u32;
    if burn_width > 0 {
        layers.push(hud_inventory_background_layer(
            burn_texture,
            79,
            34,
            burn_width,
            FURNACE_BURN_PROGRESS_SPRITE_HEIGHT,
            [0.0, 0.0],
            [
                burn_width as f32 / FURNACE_BURN_PROGRESS_SPRITE_WIDTH as f32,
                1.0,
            ],
        ));
    }
}

fn hud_inventory_background_layer(
    texture: HudInventoryBackgroundTexture,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
) -> HudInventoryBackgroundLayer {
    HudInventoryBackgroundLayer {
        texture,
        x,
        y,
        width,
        height,
        uv: HudUvRect {
            min: uv_min,
            max: uv_max,
        },
    }
}

fn registry_key_id(keys: Option<&[String]>, key: &str, fallback: i32) -> i32 {
    keys.and_then(|keys| keys.iter().position(|candidate| candidate == key))
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(fallback)
}

fn local_player_fishing_rod_casts_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    selected_item: bool,
) -> bool {
    selected_item
        && world.local_player_fishing_bobber_id().is_some()
        && item.item_id.is_some_and(|item_id| {
            item_runtime
                .and_then(|runtime| runtime.item_resource_id(item_id))
                .is_some_and(|resource_id| resource_id == "minecraft:fishing_rod")
        })
}

struct HudItemWorldModelContext<'a> {
    trim_material_keys: Option<Vec<String>>,
    trim_pattern_keys: Option<Vec<String>>,
    enchantment_keys: Option<Vec<String>>,
    damage_type_keys: Option<Vec<String>>,
    attribute_keys: Option<Vec<String>>,
    owner_main_hand_left: Option<bool>,
    context_dimension: Option<&'a str>,
    time_context: Option<bbb_item_model::ItemModelTimeContext>,
    compass_context: Option<bbb_item_model::ItemModelCompassContext<'a>>,
}

fn hud_item_world_model_context(world: &WorldStore) -> HudItemWorldModelContext<'_> {
    let trim_material_keys = world_trim_material_keys(world);
    let trim_pattern_keys = world_trim_pattern_keys(world);
    let enchantment_keys = world_enchantment_keys(world);
    let damage_type_keys = world_damage_type_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let owner_main_hand_left = world.local_player_main_arm_left();
    let context_dimension = world.level_info().map(|level| level.dimension.as_str());
    let environment_attributes = world
        .local_player_pose()
        .map_or_else(
            || world.sampled_client_environment_attributes(),
            |pose| {
                world.sampled_client_environment_attributes_at(
                    [pose.position.x, pose.position.y, pose.position.z],
                    EnvironmentBiomeSampling::Direct,
                )
            },
        )
        .ok();
    let has_authoritative_dimension_type = world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_some();
    let time_context = world
        .world_time()
        .map(|time| bbb_item_model::ItemModelTimeContext {
            game_time: time.game_time,
            day_time: time.day_time,
            sun_angle_degrees: environment_attributes
                .as_ref()
                .and_then(|attributes| attributes.sun_angle_degrees)
                .or_else(|| has_authoritative_dimension_type.then_some(0.0)),
            moon_phase_index: environment_attributes
                .as_ref()
                .and_then(|attributes| attributes.moon_phase)
                .map(client_moon_phase_index)
                .or_else(|| has_authoritative_dimension_type.then_some(0)),
        });
    let compass_spawn = world.local_player().default_spawn.as_ref().map(|spawn| {
        bbb_item_model::ItemModelCompassTarget {
            dimension: spawn.dimension.as_str(),
            pos: [spawn.pos.x, spawn.pos.y, spawn.pos.z],
        }
    });
    let compass_recovery = world.level_info().and_then(|level| {
        level
            .last_death_location
            .as_ref()
            .map(|target| bbb_item_model::ItemModelCompassTarget {
                dimension: target.dimension.as_str(),
                pos: [target.pos.x, target.pos.y, target.pos.z],
            })
    });
    let compass_context = context_dimension.and_then(|level_dimension| {
        world
            .local_player_pose()
            .map(|pose| bbb_item_model::ItemModelCompassContext {
                game_time: world.world_time().map(|time| time.game_time).unwrap_or(0),
                level_dimension,
                owner_position: [pose.position.x, pose.position.y, pose.position.z],
                owner_y_rot_degrees: pose.y_rot,
                spawn: compass_spawn,
                recovery: compass_recovery,
            })
    });
    HudItemWorldModelContext {
        trim_material_keys,
        trim_pattern_keys,
        enchantment_keys,
        damage_type_keys,
        attribute_keys,
        owner_main_hand_left,
        context_dimension,
        time_context,
        compass_context,
    }
}

fn hud_item_model_use_context(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
    using_item: bool,
) -> bbb_item_model::ItemModelUseContext {
    let is_crossbow = item
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .is_some_and(|resource_id| {
            bbb_protocol::item_types::vanilla_item_resource_id_matches_type(
                resource_id,
                bbb_protocol::item_types::VANILLA_ITEM_CROSSBOW_ID,
            )
        });
    if using_item && is_crossbow {
        let charge_duration = world
            .local_player_id()
            .and_then(|entity_id| {
                item_runtime.crossbow_charge_duration_ticks_for_living_owner(
                    world,
                    &item.component_patch.enchantments,
                    entity_id,
                )
            })
            .unwrap_or_else(|| {
                world.crossbow_charge_duration_ticks(&item.component_patch.enchantments)
            });
        item_runtime.item_model_use_context_for_stack_with_crossbow_charge_duration(
            item,
            world.local_player().interaction.using_item_ticks,
            Some(charge_duration),
        )
    } else if using_item {
        item_runtime.item_model_use_context_for_stack(
            item,
            world.local_player().interaction.using_item_ticks,
        )
    } else {
        bbb_item_model::ItemModelUseContext::inactive()
    }
}

#[allow(clippy::too_many_arguments)]
fn hud_resolved_item_model_for_stack(
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    owner_team_color_rgb: Option<u32>,
) -> Option<ResolvedItemModel> {
    let effective_item = world.effective_item_stack(item);
    let item = effective_item.as_ref();
    let context = hud_item_world_model_context(world);
    let cooldown_progress =
        item_cooldown_percent_for_stack(world, Some(item_runtime), item, 0.0).unwrap_or(0.0);
    let use_context = hud_item_model_use_context(world, item_runtime, item, using_item);
    item_runtime
        .resolved_item_model_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast_with_registry_context(
            item,
            local_selected_bundle_item_index,
            using_item,
            use_context,
            bbb_pack::BlockModelDisplayContext::Gui,
            cooldown_progress,
            context.trim_material_keys.as_deref(),
            context.trim_pattern_keys.as_deref(),
            context.enchantment_keys.as_deref(),
            context.damage_type_keys.as_deref(),
            context.attribute_keys.as_deref(),
            context.owner_main_hand_left,
            Some("minecraft:player"),
            owner_team_color_rgb,
            context.context_dimension,
            context.time_context,
            context.compass_context,
            selected_item,
            carried_item,
            true,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            item_model_seed,
            world.frozen_registry_access(),
        )
}

fn hud_item_icon_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    partial_tick: f32,
) -> Option<HudItemIcon> {
    hud_item_icon_for_stack_with_owner_team_color(
        world,
        item_runtime,
        item,
        local_selected_bundle_item_index,
        using_item,
        selected_item,
        carried_item,
        fishing_rod_cast,
        shift_down,
        keybind_context,
        item_model_seed,
        partial_tick,
        world.local_player_team_color_rgb(),
    )
}

#[allow(clippy::too_many_arguments)]
fn hud_fake_item_icon_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    partial_tick: f32,
) -> Option<HudItemIcon> {
    hud_item_icon_for_stack_with_owner_team_color(
        world,
        item_runtime,
        item,
        local_selected_bundle_item_index,
        using_item,
        selected_item,
        carried_item,
        fishing_rod_cast,
        shift_down,
        keybind_context,
        item_model_seed,
        partial_tick,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn hud_item_icon_for_stack_with_owner_team_color(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
    using_item: bool,
    selected_item: bool,
    carried_item: bool,
    fishing_rod_cast: bool,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
    item_model_seed: i32,
    partial_tick: f32,
    owner_team_color_rgb: Option<u32>,
) -> Option<HudItemIcon> {
    let item_runtime = item_runtime?;
    let effective_item = world.effective_item_stack(item);
    let item = effective_item.as_ref();
    let context = hud_item_world_model_context(world);
    // Vanilla `Cooldown.get` uses `getCooldownPercent(itemStack, 0.0F)` for
    // item-model range dispatch. The HUD overlay below still uses render
    // partial tick.
    let item_model_cooldown_progress =
        item_cooldown_percent_for_stack(world, Some(item_runtime), item, 0.0).unwrap_or(0.0);
    let use_context = hud_item_model_use_context(world, item_runtime, item, using_item);
    let icon = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast_with_registry_context(
            item,
            local_selected_bundle_item_index,
            using_item,
            use_context,
            bbb_pack::BlockModelDisplayContext::Gui,
            item_model_cooldown_progress,
            context.trim_material_keys.as_deref(),
            context.trim_pattern_keys.as_deref(),
            context.enchantment_keys.as_deref(),
            context.damage_type_keys.as_deref(),
            context.attribute_keys.as_deref(),
            context.owner_main_hand_left,
            Some("minecraft:player"),
            owner_team_color_rgb,
            context.context_dimension,
            context.time_context,
            context.compass_context,
            selected_item,
            carried_item,
            true,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            item_model_seed,
            world.frozen_registry_access(),
        )?;
    Some(HudItemIcon {
        lighting: GuiItemLightingEntry::ItemsFlat,
        layers: icon
            .layers
            .into_iter()
            .map(|layer| {
                HudIconLayer::new(
                    HudUvRect {
                        min: layer.uv.min,
                        max: layer.uv.max,
                    },
                    layer.tint,
                )
            })
            .collect(),
        foil: hud_item_foil_for_stack(item_runtime, item),
        count_label: hud_item_count_label_for_stack(item),
        durability_bar: hud_item_durability_bar_for_stack(world, item),
        cooldown_progress: hud_item_cooldown_progress_for_stack(
            world,
            Some(item_runtime),
            item,
            partial_tick,
        ),
    })
}

fn hud_item_foil_for_stack(
    item_runtime: &NativeItemRuntime,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> HudItemFoil {
    if !item.has_foil() {
        return HudItemFoil::None;
    }
    if item_runtime.item_stack_uses_special_foil_texture(item) {
        HudItemFoil::Special
    } else {
        HudItemFoil::Standard
    }
}

fn hud_item_count_label_for_stack(
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<HudItemCountLabel> {
    (!item_stack_is_empty(item) && item.count != 1)
        .then(|| HudItemCountLabel::new(item.count.to_string()))
}

fn item_stack_is_empty(item: &bbb_protocol::packets::ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

fn hud_item_cooldown_progress_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    partial_tick: f32,
) -> Option<f32> {
    let progress = item_cooldown_percent_for_stack(world, item_runtime, item, partial_tick)?;
    (progress > 0.0).then_some(progress)
}

fn item_cooldown_percent_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    partial_tick: f32,
) -> Option<f32> {
    let cooldown_group = item_cooldown_group(item_runtime, item)?;
    Some(world.item_cooldown_percent(&cooldown_group, partial_tick))
}

fn item_cooldown_group(
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<String> {
    if item_stack_is_empty(item) {
        return None;
    }
    if let Some(group) = item.component_patch.use_cooldown_group.as_ref() {
        return Some(group.clone());
    }
    let item_id = item_runtime?.item_resource_id_for_protocol_id(item.item_id?)?;
    Some(item_id.to_string())
}

fn hud_item_durability_bar_for_stack(
    world: &WorldStore,
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<HudItemDurabilityBar> {
    if item_stack_is_empty(item) || item.component_patch.unbreakable {
        return None;
    }

    // Vanilla `ItemStack.getMaxDamage()` is `getOrDefault(MAX_DAMAGE, 0)`: for a
    // damageable item the protocol patch usually only carries `damage`, since
    // `max_damage` is a registry default that doesn't get re-sent per stack.
    let max_damage = item.component_patch.max_damage.or_else(|| {
        item.item_id
            .and_then(|item_id| world.item_max_damage_for_protocol_id(item_id))
    })?;
    if max_damage <= 0 {
        return None;
    }
    let damage = item.component_patch.damage?.clamp(0, max_damage);
    if damage <= 0 {
        return None;
    }

    let width = (ITEM_DURABILITY_BAR_MAX_WIDTH as f32
        - damage as f32 * ITEM_DURABILITY_BAR_MAX_WIDTH as f32 / max_damage as f32)
        .round() as i32;
    let health_percentage = ((max_damage - damage) as f32 / max_damage as f32).max(0.0);
    Some(HudItemDurabilityBar::new(
        width.clamp(0, ITEM_DURABILITY_BAR_MAX_WIDTH) as u32,
        vanilla_hsv_to_rgb_unit(health_percentage / 3.0, 1.0, 1.0),
    ))
}

fn vanilla_hsv_to_rgb_unit(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let h = ((hue * 6.0) as i32) % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (red, green, blue) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        5 => (value, p, q),
        _ => (value, t, p),
    };
    [
        vanilla_hsv_color_component(red),
        vanilla_hsv_color_component(green),
        vanilla_hsv_color_component(blue),
    ]
}

fn vanilla_hsv_color_component(component: f32) -> f32 {
    ((component * 255.0) as i32).clamp(0, 255) as f32 / 255.0
}

fn advance_entity_client_animations(
    world: &mut WorldStore,
    ticks: &mut ClientAnimationTickState,
    now: Instant,
) -> u32 {
    ticks.last_entity_tick_work = EntityClientTickWork::default();
    ticks.last_running_level_ticks = 0;
    let Some(last) = ticks.last_entity_animation_at else {
        ticks.last_entity_animation_at = Some(now);
        return 0;
    };
    let elapsed = now.saturating_duration_since(last);
    ticks.last_entity_animation_at = Some(now);
    let tick_interval = world_client_tick_interval(world);
    // Vanilla `DeltaTracker.Timer.advanceGameTime` converts only the time since
    // the previous frame with the current target MSPT, then carries a
    // dimensionless residual. Keeping that shape matters when a TickingState
    // update changes the target between two frames.
    ticks.entity_animation_tick_residual += elapsed.as_secs_f64() / tick_interval.as_secs_f64();
    let raw_ticks = ticks.entity_animation_tick_residual.floor();
    if raw_ticks == 0.0 {
        return 0;
    }

    // `Minecraft.runTick` caps one catch-up batch at ten `Minecraft.tick`
    // calls. Keep any excess in the accumulator so a following simulation step
    // can continue without allowing one slow frame to create an unbounded
    // mutation batch.
    let advanced_ticks = raw_ticks.min(f64::from(VANILLA_MAX_CLIENT_TICKS_PER_STEP)) as u32;
    ticks.entity_animation_tick_residual -= f64::from(advanced_ticks);
    (ticks.last_entity_tick_work, ticks.last_running_level_ticks) =
        world.advance_client_level_entity_ticks_with_observation(advanced_ticks);
    advanced_ticks
}

fn advance_native_client_ticks(
    input: &ClientInputState,
    world: &mut WorldStore,
    ticks: &mut ClientAnimationTickState,
    now: Instant,
) -> u32 {
    if native_pause_freezes_client_ticks(input, world) {
        ticks.last_entity_tick_work = EntityClientTickWork::default();
        ticks.last_running_level_ticks = 0;
        ticks.last_entity_animation_at = Some(now);
        return 0;
    }
    advance_entity_client_animations(world, ticks, now)
}

fn record_entity_client_tick_observation(
    counters: &mut NetCounters,
    duration: Duration,
    work: EntityClientTickWork,
) {
    let duration_nanos = |duration: Duration| duration.as_nanos().min(u128::from(u64::MAX)) as u64;
    let nanos = duration_nanos(duration);
    counters.simulation_entity_client_tick_nanos = nanos;
    counters.simulation_entity_client_tick_peak_nanos =
        counters.simulation_entity_client_tick_peak_nanos.max(nanos);
    let preparation_nanos = duration_nanos(work.preparation_duration);
    counters.simulation_entity_client_tick_preparation_nanos = preparation_nanos;
    counters.simulation_entity_client_tick_preparation_peak_nanos = counters
        .simulation_entity_client_tick_preparation_peak_nanos
        .max(preparation_nanos);
    let schedule_nanos = duration_nanos(work.schedule_duration);
    counters.simulation_entity_client_tick_schedule_nanos = schedule_nanos;
    counters.simulation_entity_client_tick_schedule_peak_nanos = counters
        .simulation_entity_client_tick_schedule_peak_nanos
        .max(schedule_nanos);
    let projectile_nanos = duration_nanos(work.projectile_duration);
    counters.simulation_entity_client_tick_projectile_nanos = projectile_nanos;
    counters.simulation_entity_client_tick_projectile_peak_nanos = counters
        .simulation_entity_client_tick_projectile_peak_nanos
        .max(projectile_nanos);
    let interpolation_nanos = duration_nanos(work.interpolation_duration);
    counters.simulation_entity_client_tick_interpolation_nanos = interpolation_nanos;
    counters.simulation_entity_client_tick_interpolation_peak_nanos = counters
        .simulation_entity_client_tick_interpolation_peak_nanos
        .max(interpolation_nanos);
    let block_effect_nanos = duration_nanos(work.block_effect_duration);
    counters.simulation_entity_client_tick_block_effect_nanos = block_effect_nanos;
    counters.simulation_entity_client_tick_block_effect_peak_nanos = counters
        .simulation_entity_client_tick_block_effect_peak_nanos
        .max(block_effect_nanos);
    counters.simulation_entity_client_tick_block_effect_entities = work.block_effect_entities;
    counters.simulation_entity_client_tick_block_effect_blocks_visited =
        work.block_effect_blocks_visited;
    counters.simulation_entity_client_tick_block_effect_callbacks_applied =
        work.block_effect_callbacks_applied;
    let animation_nanos = duration_nanos(work.animation_duration);
    counters.simulation_entity_client_tick_animation_nanos = animation_nanos;
    counters.simulation_entity_client_tick_animation_peak_nanos = counters
        .simulation_entity_client_tick_animation_peak_nanos
        .max(animation_nanos);
    counters.simulation_entity_client_tick_scanned_entities = work.scanned_entities;
    counters.simulation_entity_client_tick_mutated_entities = work.mutated_entities;
}

pub(crate) fn world_client_tick_interval(world: &WorldStore) -> Duration {
    let ticking = world.ticking();
    // `Minecraft.getTickTargetMillis` keeps the base 20 Hz timer while frozen;
    // otherwise it returns max(base MSPT, TickRateManager.millisecondsPerTick).
    // A server tick rate above 20 therefore never accelerates client ticks.
    if ticking.frozen || !ticking.tick_rate.is_finite() || ticking.tick_rate >= 20.0 {
        return VANILLA_CLIENT_TICK_INTERVAL;
    }

    let nanos_per_tick = (1_000_000_000.0_f64 / f64::from(ticking.tick_rate)) as u64;
    Duration::from_nanos(nanos_per_tick).max(VANILLA_CLIENT_TICK_INTERVAL)
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CameraEnvironmentColors {
    sky_color: Option<[u8; 3]>,
    fog_color: Option<[u8; 3]>,
    sunrise_sunset_color: Option<[u8; 4]>,
    sun_angle_degrees: Option<f32>,
    moon_angle_degrees: Option<f32>,
    moon_phase: Option<bbb_world::ClientMoonPhase>,
    star_angle_degrees: Option<f32>,
    star_brightness: Option<f32>,
    fog_start_distance: Option<f32>,
    fog_end_distance: Option<f32>,
    sky_fog_end_distance: Option<f32>,
    cloud_fog_end_distance: Option<f32>,
    water_fog_color: Option<[u8; 3]>,
    water_fog_start_distance: Option<f32>,
    water_fog_end_distance: Option<f32>,
    camera_forward: Option<[f32; 3]>,
    fog_type: CameraFogType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CameraFogType {
    #[default]
    Atmospheric,
    Water,
    Lava,
}

fn apply_weather_cloud_color_layers(color: i32, rain_level: f64, thunder_level: f64) -> i32 {
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let rain_level = (sanitize_weather_color_level(rain_level) - thunder_level).max(0.0);
    let color = apply_weather_cloud_color_layer(
        color,
        rain_level,
        VANILLA_WEATHER_RAIN_CLOUD_GRAY_BRIGHTNESS,
        VANILLA_WEATHER_RAIN_CLOUD_GRAY_FACTOR,
    );
    apply_weather_cloud_color_layer(
        color,
        thunder_level,
        VANILLA_WEATHER_THUNDER_CLOUD_GRAY_BRIGHTNESS,
        VANILLA_WEATHER_THUNDER_CLOUD_GRAY_FACTOR,
    )
}

fn apply_weather_cloud_color_layer(
    color: i32,
    level: f32,
    gray_brightness: f32,
    gray_factor: f32,
) -> i32 {
    if level <= 0.0 {
        return color;
    }
    let gray = argb_scale_rgb(
        argb_greyscale(color),
        gray_brightness,
        gray_brightness,
        gray_brightness,
    );
    argb_srgb_lerp(level, color, argb_srgb_lerp(gray_factor, color, gray))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeatherPrecipitation {
    Rain,
    Snow,
}

fn world_can_have_weather(world: &WorldStore) -> bool {
    world
        .level_info()
        .is_some_and(WorldLevelInfo::can_have_weather)
}

fn weather_precipitation_at(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    pos: BlockPos,
    sea_level: i32,
) -> Option<WeatherPrecipitation> {
    let biome_id = world.probe_block(pos).and_then(|block| block.biome_id);
    if !terrain_textures
        .biome_has_precipitation(biome_id)
        .unwrap_or(true)
    {
        return None;
    }
    let temperature = terrain_textures.biome_temperature(biome_id).unwrap_or(0.5);
    let temperature_modifier = terrain_textures
        .biome_temperature_modifier(biome_id)
        .unwrap_or_default();
    if weather_cold_enough_to_snow(temperature, temperature_modifier, pos, sea_level) {
        Some(WeatherPrecipitation::Snow)
    } else {
        Some(WeatherPrecipitation::Rain)
    }
}

fn weather_cold_enough_to_snow(
    base_temperature: f32,
    temperature_modifier: bbb_pack::BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> bool {
    let adjusted_temperature =
        biome_height_adjusted_temperature(base_temperature, temperature_modifier, pos, sea_level);
    adjusted_temperature < VANILLA_WEATHER_SNOW_TEMPERATURE_THRESHOLD
}

fn weather_motion_blocking_height(world: &WorldStore, x: i32, z: i32) -> Option<i32> {
    if let Some(height) = world.sample_motion_blocking_height(x, z) {
        return Some(height);
    }
    let dimension = world.dimension();
    let top_y = dimension.min_y + dimension.height - 1;
    for y in (dimension.min_y..=top_y).rev() {
        let probe = world.probe_block(BlockPos { x, y, z })?;
        if weather_motion_blocking_material(probe.material) || probe.fluid.is_some() {
            return Some(y + 1);
        }
    }
    Some(dimension.min_y)
}

fn weather_motion_blocking_material(material: TerrainMaterialClass) -> bool {
    !matches!(
        material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Invisible
    )
}

fn rain_weather_column(
    x: i32,
    z: i32,
    bottom_y: i32,
    top_y: i32,
    light: TerrainLight,
    ticks: i32,
    partial_tick: f32,
) -> WeatherColumn {
    let wrapped_ticks = ticks & 131_071;
    let tick_offset = weather_column_offset_seed(x, z) & 0xff;
    let mut random = WeatherLegacyRandom::new(i64::from(weather_column_xor_seed(x, z)));
    let block_pos_rain_speed = 3.0 + random.next_float();
    let texture_offset =
        -((wrapped_ticks + tick_offset) as f32 + partial_tick) / 32.0 * block_pos_rain_speed;
    WeatherColumn::new(
        x,
        z,
        bottom_y,
        top_y,
        0.0,
        texture_offset % 32.0,
        weather_light_coords(light),
    )
}

fn snow_weather_column(
    x: i32,
    z: i32,
    bottom_y: i32,
    top_y: i32,
    light: TerrainLight,
    ticks: i32,
    partial_tick: f32,
) -> WeatherColumn {
    let time = ticks as f32 + partial_tick;
    let mut random = WeatherLegacyRandom::new(i64::from(weather_column_xor_seed(x, z)));
    let u = random.next_double() as f32 + time * 0.01 * random.next_gaussian() as f32;
    let v = random.next_double() as f32 + time * random.next_gaussian() as f32 * 0.001;
    let v_offset = -((ticks & 511) as f32 + partial_tick) / 512.0;
    WeatherColumn::new(
        x,
        z,
        bottom_y,
        top_y,
        u,
        v_offset + v,
        weather_snow_light_coords(light),
    )
}

fn weather_column_xor_seed(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971))
        ^ z.wrapping_mul(z)
            .wrapping_mul(418_711)
            .wrapping_add(z.wrapping_mul(13_761))
}

fn weather_column_offset_seed(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45_238_971))
        .wrapping_add(z.wrapping_mul(z).wrapping_mul(418_711))
        .wrapping_add(z.wrapping_mul(13_761))
}

fn weather_light_coords(light: TerrainLight) -> [f32; 2] {
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
}

fn weather_snow_light_coords(light: TerrainLight) -> [f32; 2] {
    [
        ((u16::from(light.block.min(15)) * 3 + 15) / 4) as f32 / 15.0,
        ((u16::from(light.sky.min(15)) * 3 + 15) / 4) as f32 / 15.0,
    ]
}

#[derive(Debug, Clone)]
struct WeatherLegacyRandom {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl WeatherLegacyRandom {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        let mut random = Self {
            seed: 0,
            next_gaussian: None,
        };
        random.set_seed(seed);
        random
    }

    fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK;
        self.next_gaussian = None;
    }

    fn next(&mut self, bits: u8) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1u32 << 24) as f32
    }

    fn next_double(&mut self) -> f64 {
        let upper = u64::from(self.next(26));
        let lower = u64::from(self.next(27));
        ((upper << 27) + lower) as f64 / (1u64 << 53) as f64
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let x = 2.0 * self.next_double() - 1.0;
            let y = 2.0 * self.next_double() - 1.0;
            let radius_sq = x * x + y * y;
            if radius_sq >= 1.0 || radius_sq == 0.0 {
                continue;
            }
            let multiplier = (-2.0 * radius_sq.ln() / radius_sq).sqrt();
            self.next_gaussian = Some(y * multiplier);
            return x * multiplier;
        }
    }
}

fn atmospheric_sky_fog_end(render_distance_chunks: u32) -> f32 {
    vanilla_render_distance_blocks(render_distance_chunks).min(VANILLA_DEFAULT_SKY_FOG_END_DISTANCE)
}

fn atmospheric_cloud_fog_end() -> f32 {
    (VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS * 16.0).min(VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE)
}

fn vanilla_render_distance_blocks(render_distance_chunks: u32) -> f32 {
    render_distance_chunks.clamp(
        VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
        VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    ) as f32
        * 16.0
}

fn atmospheric_fog_distance_for_dimension(kind: VanillaLightmapDimensionKind) -> (f32, f32) {
    match kind {
        VanillaLightmapDimensionKind::Nether => (
            VANILLA_NETHER_FOG_START_DISTANCE,
            VANILLA_NETHER_FOG_END_DISTANCE,
        ),
        VanillaLightmapDimensionKind::Overworld
        | VanillaLightmapDimensionKind::End
        | VanillaLightmapDimensionKind::Other => (
            VANILLA_DEFAULT_FOG_START_DISTANCE,
            VANILLA_DEFAULT_FOG_END_DISTANCE,
        ),
    }
}

fn apply_atmospheric_rain_fog_distance(
    environmental_start: &mut f32,
    environmental_end: &mut f32,
    rain_fog_multiplier: f32,
) {
    let rain_fog_multiplier = rain_fog_multiplier.clamp(0.0, 1.0);
    if rain_fog_multiplier <= 0.0 {
        return;
    }

    *environmental_start += VANILLA_RAIN_FOG_START_OFFSET * rain_fog_multiplier;
    let min_rain_fog_end = VANILLA_NETHER_FOG_END_DISTANCE.min(*environmental_end);
    *environmental_end = min_rain_fog_end
        .max(*environmental_end + VANILLA_RAIN_FOG_END_OFFSET * rain_fog_multiplier);
}

fn atmospheric_rain_fog_target_multiplier(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
) -> f32 {
    if world.level_info().map(vanilla_lightmap_dimension_kind)
        != Some(VanillaLightmapDimensionKind::Overworld)
    {
        return 0.0;
    }

    let rain_level = sanitize_weather_lightmap_level(world.weather().rain_level);
    if rain_level <= 0.0 {
        return 0.0;
    }

    let Some(block_pos) = camera_block_position(world) else {
        return 0.0;
    };
    let sky_light = world
        .sample_block_light(block_pos)
        .map(|light| light.sky)
        .unwrap_or(15) as f32;
    let sky_light_multiplier = ((sky_light - VANILLA_RAIN_FOG_MIN_SKY_LIGHT)
        / VANILLA_RAIN_FOG_SKY_LIGHT_RANGE)
        .clamp(0.0, 1.0);
    let precipitation_multiplier =
        if camera_biome_has_precipitation(world, terrain_textures, block_pos) {
            1.0
        } else {
            0.5
        };

    rain_level * sky_light_multiplier * precipitation_multiplier
}

fn dimension_sky_color_for_kind(kind: VanillaLightmapDimensionKind) -> Option<[u8; 3]> {
    match kind {
        VanillaLightmapDimensionKind::Overworld => Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Nether | VanillaLightmapDimensionKind::End => Some([0, 0, 0]),
        VanillaLightmapDimensionKind::Other => None,
    }
}

fn dimension_fog_color_for_kind(kind: VanillaLightmapDimensionKind) -> Option<[u8; 3]> {
    match kind {
        VanillaLightmapDimensionKind::Overworld => Some(VANILLA_OVERWORLD_FOG_COLOR),
        VanillaLightmapDimensionKind::End => Some(VANILLA_END_FOG_COLOR),
        VanillaLightmapDimensionKind::Nether | VanillaLightmapDimensionKind::Other => None,
    }
}

#[cfg(test)]
pub(crate) fn clear_color_for_day_time(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
) -> ClearColor {
    clear_color_for_day_time_with_environment_colors(
        day_time,
        rain_level,
        thunder_level,
        Some(VANILLA_OVERWORLD_FOG_COLOR),
        Some(VANILLA_OVERWORLD_SKY_COLOR),
        VanillaLightmapDimensionKind::Overworld,
    )
}

#[cfg(test)]
fn clear_color_for_day_time_with_environment_colors(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
    fog_color: Option<[u8; 3]>,
    sky_color: Option<[u8; 3]>,
    dimension_kind: VanillaLightmapDimensionKind,
) -> ClearColor {
    clear_color_for_day_time_with_environment_colors_and_camera(
        day_time,
        rain_level,
        thunder_level,
        fog_color,
        sky_color,
        dimension_kind,
        None,
        VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS as u32,
    )
}

fn clear_color_for_day_time_with_environment_colors_and_camera(
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
    fog_color: Option<[u8; 3]>,
    sky_color: Option<[u8; 3]>,
    dimension_kind: VanillaLightmapDimensionKind,
    camera_forward: Option<[f32; 3]>,
    render_distance_chunks: u32,
) -> ClearColor {
    let mut fog_color = rgb_u8_to_argb(fog_color.unwrap_or([0, 0, 0]));
    let mut sky_color = rgb_u8_to_argb(sky_color.unwrap_or([0, 0, 0]));
    if dimension_kind == VanillaLightmapDimensionKind::Overworld {
        fog_color = argb_multiply(
            fog_color,
            sample_periodic_argb_keyframes(
                day_time,
                &VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
        sky_color = argb_multiply(
            sky_color,
            sample_periodic_argb_keyframes(
                day_time,
                &VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES,
                VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
            ),
        );
    }
    fog_color = apply_weather_fog_color_layers(fog_color, rain_level, thunder_level);
    fog_color = apply_sunrise_sunset_fog_color(
        fog_color,
        day_time,
        rain_level,
        thunder_level,
        camera_forward,
        render_distance_chunks,
        dimension_kind,
    );
    sky_color = apply_atmospheric_sky_weather_darken(sky_color, rain_level, thunder_level);
    atmospheric_clear_color(fog_color, sky_color, render_distance_chunks)
}

fn apply_weather_fog_color_layers(color: i32, rain_level: f64, thunder_level: f64) -> i32 {
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let rain_level = (sanitize_weather_color_level(rain_level) - thunder_level).max(0.0);
    let color =
        apply_weather_fog_color_layer(color, rain_level, VANILLA_WEATHER_RAIN_FOG_COLOR_MULTIPLIER);
    apply_weather_fog_color_layer(
        color,
        thunder_level,
        VANILLA_WEATHER_THUNDER_FOG_COLOR_MULTIPLIER,
    )
}

fn apply_weather_fog_color_layer(color: i32, level: f32, multiplier: i32) -> i32 {
    if level <= 0.0 {
        return color;
    }
    argb_srgb_lerp(level, color, argb_multiply(color, multiplier))
}

fn apply_sunrise_sunset_fog_color(
    fog_color: i32,
    day_time: i64,
    rain_level: f64,
    thunder_level: f64,
    camera_forward: Option<[f32; 3]>,
    render_distance_chunks: u32,
    dimension_kind: VanillaLightmapDimensionKind,
) -> i32 {
    if dimension_kind != VanillaLightmapDimensionKind::Overworld
        || render_distance_chunks < VANILLA_SUNRISE_SUNSET_MIN_RENDER_DISTANCE_CHUNKS
    {
        return fog_color;
    }

    let Some(camera_forward) = camera_forward else {
        return fog_color;
    };
    let sun_angle = overworld_sun_angle(day_time).to_radians();
    let sun_x = if sun_angle.sin() > 0.0 { -1.0 } else { 1.0 };
    let looking_at_sun = camera_forward[0] * sun_x;
    if looking_at_sun <= 0.0 {
        return fog_color;
    }

    let mut sunrise_color = sample_periodic_argb_keyframes(
        day_time,
        &VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES,
        VANILLA_LIGHTMAP_DAY_PERIOD_TICKS,
    );
    sunrise_color =
        apply_weather_sunrise_sunset_color_layers(sunrise_color, rain_level, thunder_level);
    let alpha = argb_alpha(sunrise_color) as f32 / 255.0;
    if alpha <= 0.0 {
        return fog_color;
    }

    argb_srgb_lerp(
        looking_at_sun * alpha,
        fog_color,
        argb_opaque(sunrise_color),
    )
}

fn apply_weather_sunrise_sunset_color_layers(
    color: i32,
    rain_level: f64,
    thunder_level: f64,
) -> i32 {
    apply_weather_fog_color_layers(color, rain_level, thunder_level)
}

fn overworld_sun_angle(day_time: i64) -> f32 {
    ((day_time - VANILLA_LIGHTMAP_DEFAULT_DAY_TIME) as f32 * 360.0
        / VANILLA_LIGHTMAP_DAY_PERIOD_TICKS as f32)
        .rem_euclid(360.0)
}

fn overworld_moon_angle(day_time: i64) -> f32 {
    (overworld_sun_angle(day_time) + 180.0).rem_euclid(360.0)
}

fn overworld_star_angle(day_time: i64) -> f32 {
    overworld_sun_angle(day_time)
}

fn overworld_moon_phase(day_time: i64) -> SkyMoonPhase {
    let phase = day_time
        .rem_euclid(VANILLA_LIGHTMAP_DAY_PERIOD_TICKS * SkyMoonPhase::ALL.len() as i64)
        / VANILLA_LIGHTMAP_DAY_PERIOD_TICKS;
    SkyMoonPhase::from_vanilla_index(phase as usize)
}

fn client_moon_phase_index(phase: bbb_world::ClientMoonPhase) -> u8 {
    match phase {
        bbb_world::ClientMoonPhase::FullMoon => 0,
        bbb_world::ClientMoonPhase::WaningGibbous => 1,
        bbb_world::ClientMoonPhase::ThirdQuarter => 2,
        bbb_world::ClientMoonPhase::WaningCrescent => 3,
        bbb_world::ClientMoonPhase::NewMoon => 4,
        bbb_world::ClientMoonPhase::WaxingCrescent => 5,
        bbb_world::ClientMoonPhase::FirstQuarter => 6,
        bbb_world::ClientMoonPhase::WaxingGibbous => 7,
    }
}

fn apply_weather_star_brightness_layers(
    brightness: f32,
    rain_level: f32,
    thunder_level: f32,
) -> f32 {
    let thunder_level = thunder_level.clamp(0.0, 1.0);
    let rain_without_thunder = (rain_level.clamp(0.0, 1.0) - thunder_level).max(0.0);
    brightness * (1.0 - rain_without_thunder) * (1.0 - thunder_level)
}

fn apply_atmospheric_sky_weather_darken(color: i32, rain_level: f64, thunder_level: f64) -> i32 {
    let rain_level = sanitize_weather_color_level(rain_level);
    let thunder_level = sanitize_weather_color_level(thunder_level);
    let mut color = color;
    if rain_level > 0.0 {
        color = argb_scale_rgb(
            color,
            1.0 - rain_level * 0.5,
            1.0 - rain_level * 0.5,
            1.0 - rain_level * 0.4,
        );
    }
    if thunder_level > 0.0 {
        color = argb_scale_rgb(
            color,
            1.0 - thunder_level * 0.5,
            1.0 - thunder_level * 0.5,
            1.0 - thunder_level * 0.5,
        );
    }
    color
}

fn apply_fog_brightening(color: i32, brighten_factor: f32) -> i32 {
    if argb_red(color) == 0 || argb_green(color) == 0 || argb_blue(color) == 0 {
        return color;
    }
    let brighten_factor = brighten_factor.clamp(0.0, 1.0);
    if brighten_factor <= 0.0 {
        return color;
    }

    let red = argb_red(color) as f32 / 255.0;
    let green = argb_green(color) as f32 / 255.0;
    let blue = argb_blue(color) as f32 / 255.0;
    let target_scale = 1.0 / red.max(green).max(blue);
    argb_color(
        argb_alpha(color),
        argb_channel_from_unit(red + (red * target_scale - red) * brighten_factor),
        argb_channel_from_unit(green + (green * target_scale - green) * brighten_factor),
        argb_channel_from_unit(blue + (blue * target_scale - blue) * brighten_factor),
    )
}

fn atmospheric_clear_color(
    fog_color: i32,
    sky_color: i32,
    render_distance_chunks: u32,
) -> ClearColor {
    let sky_fog_end =
        (VANILLA_DEFAULT_SKY_FOG_END_DISTANCE / 16.0).min(render_distance_chunks as f32);
    let sky_color_mix = clamped_lerp(sky_fog_end / 32.0, 0.25, 1.0);
    let sky_color_mix = 1.0 - sky_color_mix.powf(0.25);
    clear_color_from_argb(argb_srgb_lerp(sky_color_mix, fog_color, sky_color))
}

fn sanitize_weather_color_level(level: f64) -> f32 {
    if level.is_finite() {
        level.clamp(0.0, 1.0) as f32
    } else {
        0.0
    }
}

fn clamped_lerp(factor: f32, min: f32, max: f32) -> f32 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

fn argb_scale_rgb(color: i32, red: f32, green: f32, blue: f32) -> i32 {
    argb_color(
        argb_alpha(color),
        ((argb_red(color) as f32 * red) as i32).clamp(0, 255),
        ((argb_green(color) as f32 * green) as i32).clamp(0, 255),
        ((argb_blue(color) as f32 * blue) as i32).clamp(0, 255),
    )
}

fn camera_environment_colors(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    camera_fluid: CameraFluid,
) -> CameraEnvironmentColors {
    let attributes = sampled_environment_attributes_at_camera(world, camera_pose);
    CameraEnvironmentColors {
        sky_color: attributes.sky_color,
        fog_color: attributes.fog_color,
        sunrise_sunset_color: attributes.sunrise_sunset_color,
        sun_angle_degrees: attributes.sun_angle_degrees,
        moon_angle_degrees: attributes.moon_angle_degrees,
        moon_phase: attributes.moon_phase,
        star_angle_degrees: attributes.star_angle_degrees,
        star_brightness: attributes.star_brightness,
        fog_start_distance: attributes.fog_start_distance,
        fog_end_distance: attributes.fog_end_distance,
        sky_fog_end_distance: attributes.sky_fog_end_distance,
        cloud_fog_end_distance: attributes.cloud_fog_end_distance,
        water_fog_color: attributes.water_fog_color,
        water_fog_start_distance: attributes.water_fog_start_distance,
        water_fog_end_distance: attributes.water_fog_end_distance,
        camera_forward: camera_pose.map(camera_forward_vector),
        fog_type: match camera_fluid {
            CameraFluid::None => CameraFogType::Atmospheric,
            CameraFluid::Water => CameraFogType::Water,
            CameraFluid::Lava => CameraFogType::Lava,
        },
    }
}

#[cfg(test)]
fn camera_environment_colors_for_default_projection(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> CameraEnvironmentColors {
    let camera_fluid = camera_pose.map_or(CameraFluid::None, |camera| {
        camera_fluid_in_camera(
            world,
            camera,
            [1920, 1080],
            crate::camera_fov::VANILLA_DEFAULT_CAMERA_FOV_DEGREES,
        )
    });
    camera_environment_colors(world, camera_pose, camera_fluid)
}

fn sampled_environment_attributes_at_camera(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
) -> ClientEnvironmentAttributes {
    let sampled = camera_pose.map(camera_eye_position).map_or_else(
        || world.sampled_client_environment_attributes(),
        |eye| {
            world.sampled_client_environment_attributes_at(
                eye.map(f64::from),
                EnvironmentBiomeSampling::Gaussian,
            )
        },
    );
    sampled.unwrap_or_else(|_| {
        world
            .level_info()
            .map(WorldLevelInfo::environment_attributes)
            .unwrap_or_default()
    })
}

/// Wall-clock milliseconds standing in for vanilla `Util.getMillis()`; render
/// animations only consume it modulo a fixed period (e.g. the forcefield UV
/// scroll's `% 3000L`, `WorldBorderRenderer.java:134`), so the epoch is
/// irrelevant.
fn wall_clock_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn wall_clock_millis_i64() -> i64 {
    i64::try_from(wall_clock_millis()).unwrap_or(i64::MAX)
}

pub(crate) fn monotonic_millis() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    (START.get_or_init(Instant::now).elapsed().as_millis() as u64).saturating_add(1)
}

fn particle_local_player_scope_context(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    first_person: bool,
) -> Option<ParticleLocalPlayerScopeContext> {
    let scoping = world.local_player_is_scoping();
    if !scoping {
        return None;
    }
    let camera = camera_pose?;
    let eye_position = camera_eye_position(camera).map(f64::from);
    Some(ParticleLocalPlayerScopeContext {
        eye_position,
        first_person,
        scoping,
    })
}

/// Candidate players for the PlayerCloud / Sneeze per-particle
/// `level.getNearestPlayer(x, y, z, 2.0, false)` resolution
/// (PlayerCloudParticle.java:51): the local player plus every remote player
/// entity, minus spectators (`EntitySelector.NO_SPECTATORS` — creative
/// players stay in, EntityGetter.java:95-98). The nearest-candidate pick
/// itself happens in the renderer particle tick, which knows each particle's
/// position.
fn particle_player_motion_contexts(world: &WorldStore) -> Vec<ParticlePlayerMotionContext> {
    let mut contexts = Vec::new();
    if !world.local_player_is_spectator() {
        if let Some(pose) = world.local_player_pose() {
            contexts.push(ParticlePlayerMotionContext {
                position: [pose.position.x, pose.position.y, pose.position.z],
                delta_movement: [
                    pose.delta_movement.x,
                    pose.delta_movement.y,
                    pose.delta_movement.z,
                ],
            });
        }
    }
    for transform in world.entity_transforms() {
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            continue;
        }
        if world
            .player_info_entry(transform.uuid)
            .is_some_and(|info| info.is_spectator())
        {
            continue;
        }
        contexts.push(ParticlePlayerMotionContext {
            position: [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            delta_movement: [
                transform.delta_movement.x,
                transform.delta_movement.y,
                transform.delta_movement.z,
            ],
        });
    }
    contexts
}

fn particle_entity_target_contexts(world: &WorldStore) -> Vec<ParticleEntityTargetContext> {
    world
        .entity_transforms()
        .into_iter()
        .map(|transform| ParticleEntityTargetContext {
            entity_id: transform.id,
            position: [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
        })
        .collect()
}

fn camera_block_position(world: &WorldStore) -> Option<BlockPos> {
    let eye = camera_eye_position(camera_pose_from_world(world)?);
    if !eye.into_iter().all(f32::is_finite) {
        return None;
    }
    Some(BlockPos {
        x: eye[0].floor() as i32,
        y: eye[1].floor() as i32,
        z: eye[2].floor() as i32,
    })
}

fn camera_biome_has_precipitation(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    block_pos: BlockPos,
) -> bool {
    let biome_id = world
        .probe_block(block_pos)
        .and_then(|block| block.biome_id);
    terrain_textures
        .biome_has_precipitation(biome_id)
        .unwrap_or(true)
}

fn rgb_u8_to_argb(color: [u8; 3]) -> i32 {
    argb_color(
        255,
        i32::from(color[0]),
        i32::from(color[1]),
        i32::from(color[2]),
    )
}

fn clear_color_with_sky_flash(clear: ClearColor) -> ClearColor {
    clear_color_from_argb(argb_srgb_lerp(
        VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
        clear_color_to_argb(clear),
        VANILLA_SKY_FLASH_SKY_COLOR,
    ))
}

fn clear_color_to_argb(clear: ClearColor) -> i32 {
    argb_color(
        255,
        argb_channel_from_unit(clear.r as f32),
        argb_channel_from_unit(clear.g as f32),
        argb_channel_from_unit(clear.b as f32),
    )
}

fn clear_color_from_argb(color: i32) -> ClearColor {
    ClearColor {
        r: argb_red(color) as f64 / 255.0,
        g: argb_green(color) as f64 / 255.0,
        b: argb_blue(color) as f64 / 255.0,
        a: argb_alpha(color) as f64 / 255.0,
    }
}

fn audio_scene_command_from_world(world: &WorldStore) -> TickEntitySoundPositionsCommand {
    TickEntitySoundPositionsCommand {
        listener: audio_listener_state_from_world(world),
        entities: world
            .entity_transforms()
            .into_iter()
            .map(|entity| EntitySoundPosition {
                entity_id: entity.id,
                position: [entity.position.x, entity.position.y, entity.position.z],
            })
            .collect(),
        tickable_sounds: Vec::new(),
    }
}

pub(crate) fn audio_listener_state_from_world(world: &WorldStore) -> Option<AudioListenerState> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) = world.probe_entity_camera_pose(camera_id) {
                return Some(AudioListenerState {
                    position: [
                        camera_pose.position.x,
                        camera_pose.position.y + f64::from(camera_pose.eye_height),
                        camera_pose.position.z,
                    ],
                    y_rot: camera_pose.y_rot,
                    x_rot: camera_pose.x_rot,
                });
            }
        }
    }

    world.local_player_pose().map(|pose| AudioListenerState {
        position: [
            pose.position.x,
            pose.position.y + pose.eye_height(),
            pose.position.z,
        ],
        y_rot: pose.y_rot,
        x_rot: pose.x_rot,
    })
}

/// Project the renderer-owned counter block into the control/snapshot mirror.
///
/// The renderer owns the authoritative `bbb_renderer::RendererCounters`; the
/// control crate keeps a serialization-facing mirror so it never has to depend
/// on the renderer. This field-by-field copy is the boundary crossing, matching
/// the repo's established projection philosophy (as with `NetCounters`).
pub(crate) fn control_renderer_counters(
    counters: bbb_renderer::RendererCounters,
) -> RendererCounters {
    RendererCounters {
        frame_index: counters.frame_index,
        width: counters.width,
        height: counters.height,
        surface_resizes: counters.surface_resizes,
        surface_zero_size_skips: counters.surface_zero_size_skips,
        surface_lost: counters.surface_lost,
        surface_outdated: counters.surface_outdated,
        surface_timeouts: counters.surface_timeouts,
        surface_reconfigurations: counters.surface_reconfigurations,
        surface_frames_skipped: counters.surface_frames_skipped,
        window_resize_requests: counters.window_resize_requests,
        window_minimize_requests: counters.window_minimize_requests,
        window_restore_requests: counters.window_restore_requests,
        draw_calls: counters.draw_calls,
        opaque_draw_calls: counters.opaque_draw_calls,
        cutout_draw_calls: counters.cutout_draw_calls,
        translucent_draw_calls: counters.translucent_draw_calls,
        block_destroy_overlay_draw_calls: counters.block_destroy_overlay_draw_calls,
        sky_draw_calls: counters.sky_draw_calls,
        particle_draw_calls: counters.particle_draw_calls,
        weather_draw_calls: counters.weather_draw_calls,
        item_entity_draw_calls: counters.item_entity_draw_calls,
        selection_draw_calls: counters.selection_draw_calls,
        entity_scene_draw_calls: counters.entity_scene_draw_calls,
        chunk_border_draw_calls: counters.chunk_border_draw_calls,
        chunk_border_boxes: counters.chunk_border_boxes,
        entity_scene_boxes: counters.entity_scene_boxes,
        item_entity_billboards: counters.item_entity_billboards,
        item_model_residency_hits: counters.item_model_residency_hits,
        item_model_residency_misses: counters.item_model_residency_misses,
        item_model_residency_builds: counters.item_model_residency_builds,
        item_model_residency_stale: counters.item_model_residency_stale,
        item_model_lookup_nanos: counters.item_model_lookup_nanos,
        item_model_lookup_peak_nanos: counters.item_model_lookup_peak_nanos,
        item_instance_extraction_nanos: counters.item_instance_extraction_nanos,
        item_instance_extraction_peak_nanos: counters.item_instance_extraction_peak_nanos,
        item_gpu_apply_nanos: counters.item_gpu_apply_nanos,
        item_gpu_apply_peak_nanos: counters.item_gpu_apply_peak_nanos,
        item_gpu_owners_created: counters.item_gpu_owners_created,
        item_gpu_owners_reused: counters.item_gpu_owners_reused,
        item_gpu_owners_replaced: counters.item_gpu_owners_replaced,
        item_model_vertex_upload_bytes: counters.item_model_vertex_upload_bytes,
        item_model_index_upload_bytes: counters.item_model_index_upload_bytes,
        item_instance_upload_bytes: counters.item_instance_upload_bytes,
        item_instance_upload_writes: counters.item_instance_upload_writes,
        item_instance_buffer_replacements: counters.item_instance_buffer_replacements,
        item_map_upload_bytes: counters.item_map_upload_bytes,
        item_transient_buffer_creations: counters.item_transient_buffer_creations,
        item_full_residency_replacements: counters.item_full_residency_replacements,
        item_model_resident_bytes: counters.item_model_resident_bytes,
        item_model_peak_resident_bytes: counters.item_model_peak_resident_bytes,
        item_model_instances: counters.item_model_instances,
        item_model_fallbacks: counters.item_model_fallbacks,
        map_textures_created: counters.map_textures_created,
        map_textures_updated: counters.map_textures_updated,
        map_textures_reused: counters.map_textures_reused,
        map_textures_stale: counters.map_textures_stale,
        map_texture_fallbacks: counters.map_texture_fallbacks,
        map_full_residency_replacements: counters.map_full_residency_replacements,
        map_texture_resident_bytes: counters.map_texture_resident_bytes,
        map_texture_peak_resident_bytes: counters.map_texture_peak_resident_bytes,
        map_instance_upload_bytes: counters.map_instance_upload_bytes,
        map_surface_instances: counters.map_surface_instances,
        item_extraction_scratch_capacity_growths: counters.item_extraction_scratch_capacity_growths,
        panorama_resource_ready: counters.panorama_resource_ready,
        panorama_draw_calls: counters.panorama_draw_calls,
        panorama_resident_bytes: counters.panorama_resident_bytes,
        hud_draw_calls: counters.hud_draw_calls,
        screen_blur_draw_calls: counters.screen_blur_draw_calls,
        post_effect_draw_calls: counters.post_effect_draw_calls,
        pipeline_switches: counters.pipeline_switches,
        screenshots_written: counters.screenshots_written,
        queued_sections: counters.queued_sections,
        meshed_sections: counters.meshed_sections,
        uploaded_sections: counters.uploaded_sections,
        visible_sections: counters.visible_sections,
        upload_bytes: counters.upload_bytes,
        resident_bytes: counters.resident_bytes,
        terrain_changed_owners: counters.terrain_changed_owners,
        terrain_uploaded_owners: counters.terrain_uploaded_owners,
        terrain_removed_owners: counters.terrain_removed_owners,
        terrain_vertex_upload_bytes: counters.terrain_vertex_upload_bytes,
        terrain_index_upload_bytes: counters.terrain_index_upload_bytes,
        terrain_translucent_resort_owners: counters.terrain_translucent_resort_owners,
        terrain_translucent_resort_index_bytes: counters.terrain_translucent_resort_index_bytes,
        terrain_full_replacements: counters.terrain_full_replacements,
        terrain_peak_resident_bytes: counters.terrain_peak_resident_bytes,
        terrain_selected_chunks: counters.terrain_selected_chunks,
        terrain_resident_chunks: counters.terrain_resident_chunks,
        terrain_selected_sections: counters.terrain_selected_sections,
        terrain_resident_sections: counters.terrain_resident_sections,
        terrain_pending_jobs: counters.terrain_pending_jobs,
        terrain_cache_ready: counters.terrain_cache_ready,
        terrain_mesh_jobs_queued: counters.terrain_mesh_jobs_queued,
        terrain_mesh_jobs_completed: counters.terrain_mesh_jobs_completed,
        terrain_mesh_jobs_stale: counters.terrain_mesh_jobs_stale,
        terrain_mesh_sections_queued: counters.terrain_mesh_sections_queued,
        terrain_mesh_sections_completed: counters.terrain_mesh_sections_completed,
        terrain_mesh_sections_stale: counters.terrain_mesh_sections_stale,
        terrain_mesh_queue_full: counters.terrain_mesh_queue_full,
        terrain_mesh_queue_depth: counters.terrain_mesh_queue_depth,
        terrain_mesh_peak_queue_depth: counters.terrain_mesh_peak_queue_depth,
        terrain_mesh_recent_build_nanos: counters.terrain_mesh_recent_build_nanos,
        terrain_mesh_peak_build_nanos: counters.terrain_mesh_peak_build_nanos,
        terrain_mesh_recent_build_sections: counters.terrain_mesh_recent_build_sections,
        terrain_mesh_peak_build_sections: counters.terrain_mesh_peak_build_sections,
        atlas_pages: counters.atlas_pages,
        atlas_reallocations: counters.atlas_reallocations,
        atlas_width: counters.atlas_width,
        atlas_height: counters.atlas_height,
        hud_crosshair_width: counters.hud_crosshair_width,
        hud_crosshair_height: counters.hud_crosshair_height,
        terrain_vertices: counters.terrain_vertices,
        terrain_indices: counters.terrain_indices,
        opaque_faces: counters.opaque_faces,
        cutout_faces: counters.cutout_faces,
        translucent_faces: counters.translucent_faces,
        culled_faces: counters.culled_faces,
        particle_spawn_batches: counters.particle_spawn_batches,
        particle_spawn_commands: counters.particle_spawn_commands,
        particle_missing_definitions: counters.particle_missing_definitions,
        particle_missing_sprites: counters.particle_missing_sprites,
        particle_unknown_types: counters.particle_unknown_types,
        particle_unsupported_options: counters.particle_unsupported_options,
        last_particle_spawn_count: counters.last_particle_spawn_count,
        pending_particle_spawns: counters.pending_particle_spawns,
        dropped_particle_spawns: counters.dropped_particle_spawns,
        active_particle_instances: counters.active_particle_instances,
        last_particle_intake_count: counters.last_particle_intake_count,
        last_particle_tick_count: counters.last_particle_tick_count,
        last_particle_expired_count: counters.last_particle_expired_count,
        last_particle_active_drop_count: counters.last_particle_active_drop_count,
        last_particle_limited_drop_count: counters.last_particle_limited_drop_count,
        particle_runtime_ticks: counters.particle_runtime_ticks,
        particle_instances_created: counters.particle_instances_created,
        particle_instances_expired: counters.particle_instances_expired,
        dropped_active_particle_instances: counters.dropped_active_particle_instances,
        dropped_limited_particle_instances: counters.dropped_limited_particle_instances,
        entity_render_candidates: 0,
        entity_render_survivors: 0,
        entity_render_projected: 0,
        entity_candidate_nanos: 0,
        entity_candidate_peak_nanos: 0,
        entity_cull_nanos: 0,
        entity_cull_peak_nanos: 0,
        entity_full_projection_nanos: 0,
        entity_full_projection_peak_nanos: 0,
        entity_model_build_nanos: 0,
        entity_model_build_peak_nanos: 0,
        entity_extraction_capacity_growths: 0,
        non_entity_hud_nanos: 0,
        non_entity_hud_peak_nanos: 0,
        non_entity_item_nanos: 0,
        non_entity_item_peak_nanos: 0,
        non_entity_block_entity_nanos: 0,
        non_entity_block_entity_peak_nanos: 0,
        non_entity_world_debug_nanos: 0,
        non_entity_world_debug_peak_nanos: 0,
        non_entity_apply_nanos: 0,
        non_entity_apply_peak_nanos: 0,
        non_entity_extraction_capacity_growths: 0,
        non_entity_cache_hits: 0,
        non_entity_cache_misses: 0,
        non_entity_rebuilt_owners: 0,
        dynamic_texture_cache_entries: 0,
        profile_texture_set_entries: 0,
        profile_texture_asset_entries: 0,
        profile_texture_key_bytes: 0,
        dynamic_texture_cache_pending: 0,
        dynamic_texture_cache_ready: 0,
        dynamic_texture_cache_failed: 0,
        dynamic_texture_cache_stale_results: 0,
        dynamic_texture_cache_queue_full: 0,
        dynamic_texture_cache_max_resident_bytes: 0,
        dynamic_texture_cache_resident_bytes: 0,
        dynamic_texture_cache_peak_resident_bytes: 0,
        dynamic_texture_cache_resident_evictions: 0,
        dynamic_texture_cache_resident_rejections: 0,
        dynamic_texture_worker_result_bytes: 0,
        dynamic_texture_peak_worker_result_bytes: 0,
        dynamic_texture_recent_delivery_bytes: 0,
        dynamic_texture_peak_delivery_bytes: 0,
    }
}

pub(crate) fn project_dynamic_texture_counters(
    counters: &mut RendererCounters,
    items: Option<&NativeItemRuntime>,
) {
    let observation = items
        .map(NativeItemRuntime::dynamic_texture_cache_observation)
        .unwrap_or_default();
    counters.dynamic_texture_cache_entries = observation.entries;
    counters.profile_texture_set_entries = observation.profile_sets;
    counters.profile_texture_asset_entries = observation.profile_assets;
    counters.profile_texture_key_bytes =
        observation.profile_key_bytes.try_into().unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_pending = observation.pending;
    counters.dynamic_texture_cache_ready = observation.ready;
    counters.dynamic_texture_cache_failed = observation.failed;
    counters.dynamic_texture_cache_stale_results = observation.stale_results;
    counters.dynamic_texture_cache_queue_full = observation.queue_full;
    counters.dynamic_texture_cache_max_resident_bytes = observation
        .max_resident_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_resident_bytes =
        observation.resident_bytes.try_into().unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_peak_resident_bytes = observation
        .peak_resident_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_cache_resident_evictions = observation.resident_evictions;
    counters.dynamic_texture_cache_resident_rejections = observation.resident_rejections;
    counters.dynamic_texture_worker_result_bytes = observation
        .worker_result_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_peak_worker_result_bytes = observation
        .peak_worker_result_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_recent_delivery_bytes = observation
        .recent_delivery_bytes
        .try_into()
        .unwrap_or(u64::MAX);
    counters.dynamic_texture_peak_delivery_bytes = observation
        .peak_delivery_bytes
        .try_into()
        .unwrap_or(u64::MAX);
}

#[cfg(test)]
mod tests;
