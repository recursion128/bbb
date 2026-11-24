use bbb_client_clock::Elapsed as _;
use bbb_client_clock::{Instant, UNIX_EPOCH};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
    time::Duration,
};

use crate::control::{AudioCounters, NetCounters, RendererCounters};
use crate::item_models::tooltip::{
    enchantment_name_tooltip_line, styled_hover_name, tooltip_lines_for_stack_with_context,
    NativeItemHoverName, NativeItemMapTooltipData, NativeItemTooltipOptions,
};
use bbb_client_audio::AudioListenerState;
use bbb_client_net::{NetCommand, NetEvent};
use bbb_client_render::item_model::{
    ItemModelKeybindContext, NativeItemRuntime, ResolvedItemModel, ResolvedItemModelLayer,
};
use bbb_client_render::{
    hud_air_supply_pop, hud_air_supply_visible, hud_command_block_editor_layout,
    hud_jigsaw_block_editor_layout, hud_minecart_command_block_editor_layout,
    hud_structure_block_editor_layout, hud_test_block_editor_layout,
    hud_test_instance_block_editor_layout, sign_text_base_color, BlockDestroyOverlay,
    BlockEntityCrumbling, CameraPose, ClearColor, CloudEnvironment, CloudFrame, ConduitModelPart,
    DebugCuboidFace, DebugFilledBox, DebugFilledRect, DebugLine, DebugTextBillboard,
    EntityModelInstance, EntityModelKind, FogEnvironment, GuiItemLightingEntry,
    HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite, HudAdvancementLineTexture,
    HudAdvancementTabSprite, HudAdvancementWidgetFrameSprite, HudAirSupply, HudBlockItemModel,
    HudCameraOverlay, HudChatContext, HudChatRestrictionsScreen, HudChatTag, HudChatTagKind,
    HudCommandBlockEditorKind, HudCommandBlockEditorScreen, HudCommandSuggestionTooltip,
    HudCommandSuggestions, HudCommandSuggestionsLayout, HudComponentTextRun, HudConfusionOverlay,
    HudContextualBar, HudCrosshairAttackIndicator, HudDebugCrosshair, HudDebugFrameTimeChart,
    HudDebugGameModeSwitcher, HudDebugGameModeSwitcherSlot, HudDebugNetworkCharts,
    HudDebugOptionsTooltip, HudDebugOverlay, HudDebugProfilerChart, HudDebugTpsChart,
    HudDebugTpsSample, HudDemoLinkConfirmationScreen, HudDemoPopupScreen, HudEntityPreview,
    HudEntityPreviewItemDisplayContext, HudEntityPreviewItemLayer, HudEntityPreviewItemSlot,
    HudEntityPreviewRect, HudFontGlyphBook, HudFoodEffect, HudGameEventButton, HudGameEventScreen,
    HudHeartKind, HudHotbarAttackIndicator, HudHotbarMode, HudIconLayer, HudImePreedit,
    HudInventoryBackgroundLayer, HudInventoryBackgroundTexture, HudInventoryFillLayer,
    HudInventoryFillStage, HudInventoryGhostItem, HudInventoryItem, HudInventoryItemScissor,
    HudInventoryMapPreview, HudInventoryScreen, HudInventoryScrollingText, HudInventorySlot,
    HudInventoryTextBackground, HudInventoryTextLabel, HudInventoryTooltip,
    HudInventoryTooltipLine, HudItemCountLabel, HudItemDurabilityBar, HudItemFoil, HudItemIcon,
    HudJigsawBlockEditorScreen, HudJumpBar, HudLevelLoadingBackground, HudLevelLoadingScreen,
    HudMobEffect, HudOffhandSlot, HudPauseScreen, HudPauseScreenLabels, HudPauseScreenTooltip,
    HudPlayerHealth, HudPlayerList, HudPortalOverlay, HudResidentItemLayer, HudScoreboardSidebar,
    HudSignEditorKind, HudSignEditorScreen, HudSocialInteractionsFocus, HudSocialInteractionsPage,
    HudSocialInteractionsPlayerAction, HudSocialInteractionsScreen, HudSpyglassScope,
    HudStatsItemRow, HudStatsRow, HudStatsScreen, HudStructureBlockEditorScreen,
    HudStructureBlockMode, HudStyledTextRun, HudTestBlockEditorScreen,
    HudTestInstanceBlockEditorScreen, HudTextStyle, HudUvRect, HudVehicleHealth, HudWinScreen,
    HudWinScreenLine, ItemModelAssetIdentity, ItemModelAssetSource, ItemModelAtlasKind,
    ItemModelGeometryClass, LevelFrameRoute, LevelLighting, LightmapEnvironment,
    LightningBoltRenderState, Renderer, SelectionColoredBox, SelectionLine, SelectionOutline,
    SignModelAttachment, SignModelWood, SkyEnvironment, SkyMoonPhase, WeatherColumn, WeatherFrame,
    WeatherRenderState, DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_FULL_BRIGHT_LIGHT_COORDS,
    HUD_HOTBAR_SLOTS, ITEM_MODEL_NO_OVERLAY, VANILLA_DEFAULT_CLOUD_COLOR,
    VANILLA_DEFAULT_CLOUD_HEIGHT, VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR, VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR, VANILLA_MAX_RENDER_DISTANCE_CHUNKS,
    VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use bbb_client_render::{BlockSelectionPhase, BlockSelectionRenderState};
use bbb_client_world::particles::{
    ParticleEngine, ParticleKind, ParticleLocalPlayerScopeContext, ParticleParams,
    ParticlePlayerMotionContext, ParticleSound, ParticleSpawn, ParticleTickContext,
};
use bbb_client_world::{
    java_string_hash_code, sign_wood_and_form_for_block_name, vanilla_entity_type_is_living,
    BlockPos, CartographyAdditionalItemKind, ChunkPos, ClientEnvironmentAttributes, ContainerState,
    EnchantmentTooltipDefinition, EntityClientMovementEffectState, EntityClientTickWork,
    EnvironmentBiomeSampling, HudCameraPlayerVitals, LocalPlayerCrosshairAttackIndicatorState,
    LocalSoundEventState, MerchantOfferState, MerchantOffersState, MobEffectState,
    MountArmorSlotKind, MountInventoryKind, SignBlockForm, SoundEventState, SoundHolderState,
    TerrainFluidKind, TerrainFluidState, TerrainLight, TerrainMaterialClass,
    VanillaEnchantmentIdentity, WorldLevelInfo, WorldLevelLoadReason, WorldSkybox, WorldStore,
    WorldWeatherState,
};
use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    equipment_slots::EquipmentSlot,
    heightmap_types::{
        vanilla_heightmap_type, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
    },
    packets::{
        AdvancementFrameType, CommandBlockMode, GameType, ItemCostSummary, ItemStackSummary,
        JigsawJoint, MapPostProcessingSummary, RecipeBookType, RemoteDebugSampleType,
        StructureBlockMirror, StructureBlockMode, StructureBlockRotation, TestBlockMode,
        TestInstanceBlockRotation, Vec3d,
    },
    ComponentStyle, RichTextComponent, StyledTextRun, MC_VERSION,
};
use bbb_render_types::{color as packed_color, VanillaLightCoords};
use glam::{Quat, Vec3};
use tokio::sync::mpsc;
use winit::dpi::PhysicalSize;

mod hud_presentations;
#[cfg(test)]
use hud_presentations::HudAirBubblePopSound;
pub(crate) use hud_presentations::HudRenderRuntime;
use hud_presentations::{
    push_capped_hud_debug_sample, HudHotbarBlockItemModelsCacheKey, HudPlayerListCacheKey,
    HudScoreboardSidebarCacheKey, HudScreensCacheKey, HudStaticUiCacheKey,
    HUD_DEBUG_FRAME_TIME_SAMPLE_CAPACITY, HUD_DEBUG_NETWORK_SAMPLE_CAPACITY,
    HUD_DEBUG_NETWORK_SAMPLE_INTERVAL, HUD_DEBUG_TPS_SAMPLE_CAPACITY,
};
#[cfg(test)]
use hud_presentations::{HudFrameDeltaTicks, HudPlayerHealthFrame};
mod debug_samplers;
pub(crate) use debug_samplers::{
    DebugCollisionCache, DebugSupportingBlockCache, HudDebugFpsSampler, HudDebugNetworkSampler,
    HudDebugTpsSampler,
};
#[cfg(test)]
use debug_samplers::{
    DEBUG_COLLISION_REFRESH_INTERVAL, DEBUG_SUPPORTING_BLOCK_QUERY_RADIUS,
    DEBUG_SUPPORTING_BLOCK_REFRESH_INTERVAL,
};
mod lightmap;
use lightmap::{
    apply_overworld_timeline_lightmap_environment, apply_weather_lightmap_environment,
    dimension_lightmap_environment, local_player_lightmap_effects, mob_effect_ends_within,
    sanitize_weather_lightmap_level, vanilla_lightmap_dimension_kind, vanilla_night_vision_scale,
    VANILLA_LIGHTMAP_DEFAULT_DAY_TIME, VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK,
};
use lightmap::{
    camera_entity_effect, camera_entity_is_living, camera_entity_is_spectator,
    camera_mob_effect_blocks_sky, camera_player_heart_type, local_player_effect,
};
pub(crate) use lightmap::{
    entity_shadow_sky_darken, CameraSpinningEffectTickState, LightmapTickState,
    VanillaLightmapDimensionKind,
};
mod color;
use color::{
    argb_alpha, argb_alpha_blend, argb_blue, argb_channel_from_unit, argb_color, argb_green,
    argb_multiply, argb_opaque, argb_red, argb_srgb_lerp, argb_to_rgba8, rgb01_to_argb, rgb24,
    rgba01_to_argb, rgba32, sample_periodic_argb_keyframes, sample_periodic_float_keyframes,
};
mod simulation_step;
#[cfg(test)]
use simulation_step::disconnected_screen_title_for_state;
use simulation_step::optional_profile_elapsed;
#[cfg(test)]
use simulation_step::remote_disconnect_stops_audio;
use simulation_step::world_ticking_milliseconds_per_tick;
#[cfg(test)]
use simulation_step::{
    advance_entity_client_animations, advance_first_multiplayer_server_tutorial,
    advance_native_client_ticks, queue_client_tick_end_commands,
    record_entity_client_tick_observation,
};
pub(crate) use simulation_step::{
    advance_simulation, world_client_tick_interval, SimulationFrameState, SimulationStepContext,
};
mod frame;
pub(crate) use frame::update_renderer;
mod particles;
use particles::{
    emit_particle_sounds, particle_light_block_pos, particle_local_player_scope_context,
    particle_player_motion_contexts, particle_target, submit_entity_client_tick_effects,
    submit_local_player_action_particles, submit_local_player_sprint_particles,
    submit_ominous_item_spawner_particles, submit_primed_tnt_smoke_particles,
};
mod block_overlays;
#[cfg(test)]
use block_overlays::block_entity_model_accepts_crumbling;
use block_overlays::{
    apply_block_entity_crumbling, block_destroy_overlays_from_stages, block_selection_render_state,
};
mod hud_items;
use hud_items::{
    hotbar_block_item_models, hotbar_item_icons_with_input_context, hud_offhand_block_item_model,
    hud_offhand_slot,
};
#[cfg(test)]
use hud_items::{hotbar_item_icons, hotbar_item_icons_with_extended_view, hud_offhand_slot_side};
mod hud_overlays;
use hud_overlays::{
    hud_camera_overlays, hud_confusion_overlay, hud_crosshair_attack_indicator,
    hud_debug_game_mode_switcher, hud_hotbar_attack_indicator, hud_level_loading_screen,
    hud_portal_overlay, VANILLA_SPYGLASS_SCOPE_RESET_SCALE, VANILLA_SPYGLASS_SCOPE_TARGET_SCALE,
};
#[cfg(test)]
use hud_overlays::{
    hud_game_mode_switcher_help_runs, hud_game_mode_switcher_icon_resource_id,
    DEBUG_GAME_MODE_SWITCHER_KEY_MAPPING, DEBUG_GAME_MODE_SWITCHER_MODES,
};
mod stats_screen;
#[cfg(test)]
use stats_screen::{
    format_vanilla_stat_value, stats_screen_first_data_row, stats_screen_first_data_row_offset,
    stats_screen_projected_row_count, stats_screen_scroller_height, stats_screen_viewport_height,
};
use stats_screen::{hud_stats_screen, stats_screen_narration_frame};
mod screen_narration;
#[cfg(test)]
use screen_narration::format_localized_string_arguments;
use screen_narration::{
    book_screen_narration_frame, chat_screen_narration_frame, command_block_editor_narration_frame,
    container_inventory_narration_frame, death_screen_narration_frame,
    dialog_screen_narration_frame, disconnected_screen_narration_frame,
    game_event_screen_narration_frame, jigsaw_block_editor_narration_frame,
    localized_gui_text_with_args, pause_screen_narration_frame, sign_editor_narration_frame,
    social_interactions_narration_frame, structure_block_editor_narration_frame,
};
pub(crate) use screen_narration::{
    narrate_command_suggestion_change, narrate_player_list_visibility_change,
};
mod screen_projection;
use screen_projection::release_input_if_screen_opened;
use screen_projection::{
    hud_chat_restrictions_screen, hud_command_block_editor_screen, hud_command_suggestions,
    hud_game_event_screen, hud_jigsaw_block_editor_screen, hud_pause_screen,
    hud_sign_editor_screen, hud_structure_block_editor_screen, hud_test_block_editor_screen,
    hud_test_instance_block_editor_screen,
};
#[cfg(test)]
use screen_projection::{hud_command_suggestion_tooltip_runs, jigsaw_priority_tooltip_key};
mod hud_text;
#[cfg(test)]
use hud_text::{hud_mob_effect_duration_text, hud_system_toast, hud_system_toasts};
use hud_text::{
    hud_mob_effects_from_world, hud_styled_runs_from_component_runs,
    hud_styled_runs_from_rich_text, humanize_resource_path, localized_gui_text,
    mob_effect_level_fallback, server_pack_bytes_human_readable,
};
mod debug_geometry;
#[cfg(test)]
use debug_geometry::DEBUG_HEIGHTMAP_BOX_HEIGHT;
use debug_geometry::{
    collect_debug_collision_lines, collect_debug_supporting_block_lines,
    debug_chunk_border_outline, debug_collision_entry_enabled, debug_entity_scene_outline,
    debug_heightmap_boxes, debug_light_level_billboards, debug_selection_text_billboards,
    debug_solid_face_rects, debug_supporting_block_entry_enabled, debug_water_amount_billboards,
    debug_water_boxes, debug_water_samples,
};
#[cfg(test)]
use debug_geometry::{
    debug_supporting_block_entity_bias, CHUNK_BORDER_CELL_COLOR, CHUNK_BORDER_MAJOR_COLOR,
    CHUNK_BORDER_NEIGHBOR_COLOR, CHUNK_BORDER_THICK_LINE_WIDTH, CHUNK_BORDER_THIN_LINE_WIDTH,
    CHUNK_BORDER_YELLOW_COLOR, DEBUG_COLLISION_COLOR, DEBUG_COLLISION_QUERY_RADIUS,
    DEBUG_LIGHT_SCAN_RADIUS, DEBUG_LIGHT_SECTION_LABEL_COLOR, DEBUG_LIGHT_SECTION_LABEL_SCALE,
    DEBUG_SOLID_FACE_COLOR, DEBUG_SOLID_FACE_INFLATE, DEBUG_SOLID_FACE_SCAN_RADIUS,
    DEBUG_SUPPORTING_BLOCK_ENTITY_COLOR, DEBUG_SUPPORTING_BLOCK_LEGACY_OFFSET,
    DEBUG_SUPPORTING_BLOCK_OUTLINE_ALPHA, DEBUG_SUPPORTING_BLOCK_PLAYER_OFFSET,
    DEBUG_WATER_BOX_COLOR, DEBUG_WATER_BOX_INSET, DEBUG_WATER_SCAN_RADIUS,
};
mod inventory_screen;
#[cfg(test)]
use inventory_screen::{
    approximate_inventory_hud_fonts, hud_inventory_screen, hud_inventory_screen_with_local_state,
    hud_inventory_screen_with_local_state_for_surface, inventory_screen_slot_uses_fake_item,
};
use inventory_screen::{
    hud_inventory_screen_for_surface, update_advancement_hover_fade, InventoryHudLocalState,
};
mod recipe_book;
use recipe_book::{
    hud_recipe_book_background_layer, hud_recipe_book_button_layer,
    hud_recipe_book_filter_button_layer, hud_recipe_book_ghost_layers_and_items,
    hud_recipe_book_ghost_tooltip, hud_recipe_book_overlay_icon_items,
    hud_recipe_book_overlay_layers, hud_recipe_book_page_button_layers,
    hud_recipe_book_page_text_label, hud_recipe_book_recipe_button_icon_items,
    hud_recipe_book_recipe_button_layers, hud_recipe_book_search_box_layer,
    hud_recipe_book_search_text_label, hud_recipe_book_tab_icon_items, hud_recipe_book_tab_layers,
    hud_recipe_book_tooltip, recipe_book_narration_frame,
};
#[cfg(test)]
use recipe_book::{RECIPE_BOOK_GHOST_POST_ITEM_TINT, RECIPE_BOOK_GHOST_PRE_ITEM_TINT};
mod inventory_preview;
#[cfg(test)]
use inventory_preview::{hud_enchantment_book_preview, quaternion_mul, quaternion_x, quaternion_z};
use inventory_preview::{
    hud_entity_in_inventory_follows_mouse_preview, hud_inventory_entity_previews,
};
mod advancements;
use advancements::{
    advancement_hover_description_text_color, advancement_hover_progress_box_state,
    advancement_hovered_widget, advancement_widget_frame_sprite, advancements_widget_scroll,
    hud_advancements_screen, push_advancement_hover_progress_box_segment,
    ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH, ADVANCEMENTS_HOVER_DESCRIPTION_PADDING_TOP,
    ADVANCEMENTS_HOVER_FRAME_SIZE, ADVANCEMENTS_HOVER_LINE_HEIGHT,
    ADVANCEMENTS_HOVER_PROGRESS_PADDING, ADVANCEMENTS_HOVER_PROGRESS_PREFIX_WIDTH,
    ADVANCEMENTS_HOVER_RIGHT_MARGIN, ADVANCEMENTS_HOVER_TITLE_MAX_WIDTH,
    ADVANCEMENTS_HOVER_TITLE_MIN_WIDTH, ADVANCEMENTS_HOVER_TITLE_PADDING_BOTTOM,
    ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT, ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT,
    ADVANCEMENTS_HOVER_TITLE_PADDING_TOP, ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR,
    ADVANCEMENTS_HOVER_TITLE_X, ADVANCEMENTS_WIDGET_FRAME_HEIGHT,
    ADVANCEMENTS_WIDGET_FRAME_OFFSET_X, ADVANCEMENTS_WIDGET_FRAME_WIDTH,
    ADVANCEMENTS_WINDOW_INSIDE_HEIGHT, ADVANCEMENTS_WINDOW_INSIDE_X, ADVANCEMENTS_WINDOW_INSIDE_Y,
};
pub(crate) use advancements::{
    advancements_done_button_origin_for_surface, advancements_window_origin_for_surface,
};
#[cfg(test)]
use advancements::{
    advancements_screen_text_labels, advancements_widget_connection_layers,
    advancements_widget_icon_items, clipped_advancement_widget_frame_layer,
    ADVANCEMENTS_DONE_BUTTON_HEIGHT, ADVANCEMENTS_DONE_BUTTON_TEXT_Y_OFFSET,
    ADVANCEMENTS_DONE_BUTTON_WIDTH, ADVANCEMENTS_DONE_TEXT, ADVANCEMENTS_EMPTY_BACKGROUND_TINT,
    ADVANCEMENTS_EMPTY_TEXT, ADVANCEMENTS_EMPTY_TEXT_CENTER_X, ADVANCEMENTS_EMPTY_TEXT_COLOR,
    ADVANCEMENTS_EMPTY_TEXT_Y, ADVANCEMENTS_HOVER_CHALLENGE_DESCRIPTION_TEXT_COLOR,
    ADVANCEMENTS_HOVER_FADE_TINT, ADVANCEMENTS_HOVER_MAX_FADE, ADVANCEMENTS_SAD_TEXT,
    ADVANCEMENTS_SAD_TEXT_Y, ADVANCEMENTS_TITLE_TEXT, ADVANCEMENTS_TITLE_TEXT_COLOR,
    ADVANCEMENTS_WINDOW_HEIGHT, ADVANCEMENTS_WINDOW_INSIDE_WIDTH, ADVANCEMENTS_WINDOW_TITLE_X,
    ADVANCEMENTS_WINDOW_TITLE_Y, ADVANCEMENTS_WINDOW_WIDTH,
};
mod inventory_text;
use inventory_text::{
    hud_ascii_approx_char_width, hud_ascii_approx_text_width, hud_beacon_tooltip,
    hud_bundle_tooltip_for_stack, hud_enchantment_table_tooltip, hud_inventory_bundle_tooltip,
    hud_inventory_text_labels, hud_inventory_tooltip_with_inspector,
    hud_merchant_trade_tooltip_with_inspector, hud_stonecutter_recipe_tooltip_with_inspector,
};
#[cfg(test)]
use inventory_text::{
    push_component_container_label, CONTAINER_CENTERED_LABEL_WIDTH, CONTAINER_LABEL_TEXT_COLOR,
    CONTAINER_LABEL_WIDTH,
};
mod inventory_items;
#[cfg(test)]
use inventory_items::StonecutterRecipeOptionStack;
use inventory_items::{
    hud_inventory_floating_items, hud_item_tooltip, hud_item_tooltip_with_inspector,
    item_stacks_match_by_item_and_components, push_hud_inventory_cursor_item,
    push_stonecutter_recipe_layers, stonecutter_visible_recipe_option_stacks,
    HudItemTooltipContent,
};
mod inventory_background;
#[cfg(test)]
use inventory_background::BEACON_LEVELS_DATA_ID;
use inventory_background::{
    beacon_button_contains, beacon_effect_selection_from_world, beacon_primary_effect,
    cartography_table_presentation, hud_inventory_background_layer,
    hud_inventory_background_layers, hud_merchant_discount_layers, item_cost_stack,
    merchant_offer_base_cost_a_stack, merchant_offer_cost_a_stack, merchant_offers_state,
    merchant_scroll_offset, merchant_selected_offer, open_container_slot_has_item,
    open_container_slot_item, smithing_should_show_error, ANVIL_COST_BACKGROUND_HEIGHT,
    ANVIL_COST_BACKGROUND_TINT, ANVIL_COST_BACKGROUND_Y, ANVIL_COST_DATA_ID,
    ANVIL_COST_ERROR_TEXT_COLOR, ANVIL_COST_LABEL_RIGHT, ANVIL_COST_LABEL_Y, ANVIL_COST_TEXT_COLOR,
    ANVIL_RENAME_TEXT_COLOR, ANVIL_RESULT_SLOT, ANVIL_TOO_EXPENSIVE_LEVEL_COST,
    BEACON_ACTION_BUTTON_Y, BEACON_CANCEL_BUTTON_X, BEACON_CONFIRM_BUTTON_X,
    BEACON_EXAMPLE_PAYMENT_ITEMS, BEACON_LABEL_TEXT_COLOR, BEACON_LABEL_Y,
    BEACON_PRIMARY_LABEL_CENTER_X, BEACON_SECONDARY_LABEL_CENTER_X,
    ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR, ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR,
    ENCHANTING_TABLE_COST_TEXT_X_RIGHT, ENCHANTING_TABLE_COST_TEXT_Y_OFFSET,
    ENCHANTING_TABLE_NAME_TEXT_COLOR, ENCHANTING_TABLE_NAME_TEXT_DISABLED_COLOR,
    ENCHANTING_TABLE_NAME_TEXT_HIGHLIGHTED_COLOR, MERCHANT_DEPRECATED_TOOLTIP_HEIGHT,
    MERCHANT_DEPRECATED_TOOLTIP_WIDTH, MERCHANT_DEPRECATED_TOOLTIP_X,
    MERCHANT_DEPRECATED_TOOLTIP_Y, MERCHANT_TRADE_BUTTON_HEIGHT, MERCHANT_TRADE_BUTTON_WIDTH,
    MERCHANT_TRADE_BUTTON_X, MERCHANT_TRADE_BUTTON_Y, MERCHANT_TRADE_COST_A_X,
    MERCHANT_TRADE_COST_B_X, MERCHANT_TRADE_ITEM_Y, MERCHANT_TRADE_RESULT_X,
    MERCHANT_TRADE_ROW_HEIGHT, MERCHANT_VISIBLE_OFFER_COUNT,
};
#[cfg(test)]
use inventory_background::{
    cartography_additional_item_for_resource_id, CartographyResultMode,
    BEACON_PRIMARY_EFFECT_DATA_ID,
};
mod item_projection;
use item_projection::{
    bundle_contents_weight_fraction, effective_bundle_contents, hud_block_item_model,
    hud_item_cooldown_progress_for_stack, hud_item_count_label_for_stack,
    hud_item_durability_bar_for_stack, hud_item_icon, is_bundle_item_stack, item_stack_is_empty,
    local_player_fishing_rod_casts_item, registry_key_id, HudItemModelContext, HudItemSlotState,
};
#[cfg(test)]
use item_projection::{hud_item_foil_for_stack, item_cooldown_group, vanilla_hsv_to_rgb_unit};
mod weather;
#[cfg(test)]
pub(crate) use weather::clear_color_for_day_time;
use weather::terrain_camera_position;
pub(crate) use weather::CameraEnvironmentColors;
use weather::{
    apply_atmospheric_rain_fog_distance, apply_fog_brightening, apply_weather_cloud_color_layers,
    apply_weather_sky_color_layers, apply_weather_star_brightness_layers,
    apply_weather_sunrise_sunset_color_layers, argb_scale_rgb,
    atmospheric_fog_distance_for_dimension, atmospheric_rain_fog_target_multiplier,
    atmospheric_sky_fog_end, camera_environment_colors_from_cached_sample,
    clear_color_for_atmospheric_environment, clear_color_from_argb, clear_color_to_argb,
    clear_color_with_sky_flash, client_moon_phase_index, dimension_fog_color_for_kind,
    dimension_sky_color_for_kind, overworld_moon_angle, overworld_moon_phase, overworld_star_angle,
    overworld_sun_angle, rain_weather_column, rgb_u8_to_argb,
    sampled_camera_fog_and_sky_attributes_at_camera, sampled_full_environment_attributes_at_camera,
    snow_weather_column, weather_motion_blocking_height, weather_precipitation_at,
    world_can_have_weather, AtmosphericClearColorInputs, CameraFogType, WeatherPrecipitation,
    VANILLA_DEFAULT_CLOUD_FOG_END_DISTANCE, VANILLA_DEFAULT_WATER_FOG_COLOR,
    VANILLA_DEFAULT_WATER_FOG_END_DISTANCE, VANILLA_DEFAULT_WATER_FOG_START_DISTANCE,
    VANILLA_LAVA_FOG_COLOR, VANILLA_NETHER_FOG_END_DISTANCE, VANILLA_NETHER_FOG_START_DISTANCE,
    VANILLA_POWDER_SNOW_FOG_COLOR, VANILLA_RAIN_FOG_SMOOTHING_PER_TICK,
    VANILLA_SKY_FLASH_SKY_COLOR, VANILLA_SKY_FLASH_SKY_COLOR_ALPHA,
};
#[cfg(test)]
use weather::{
    atmospheric_clear_color, camera_block_position,
    camera_environment_colors_for_default_projection,
    clear_color_for_day_time_with_environment_colors,
    clear_color_for_day_time_with_environment_colors_and_camera,
    VANILLA_ATMOSPHERIC_FOG_RENDER_DISTANCE_CHUNKS, VANILLA_DEFAULT_FOG_END_DISTANCE,
    VANILLA_DEFAULT_FOG_START_DISTANCE, VANILLA_DEFAULT_SKY_FOG_END_DISTANCE,
    VANILLA_END_FOG_COLOR, VANILLA_OVERWORLD_FOG_COLOR, VANILLA_OVERWORLD_SKY_COLOR,
    VANILLA_WEATHER_RENDER_RADIUS,
};
mod frame_counters;
pub(crate) use frame_counters::{control_renderer_counters, project_dynamic_texture_counters};

mod advancement_text;
mod camera_environment_probe;

#[cfg(test)]
use advancement_text::advancement_wrap_text_lines;
use advancement_text::push_advancement_hover_boxes_and_text;

pub(crate) use camera_environment_probe::CameraEnvironmentSampleCache;

use crate::options::derived::effective_render_distance;
use crate::{
    audio_runtime::AudioEventSink,
    banner_scene::banner_model_instances_from_world_at_partial_tick,
    beacon_scene::beacon_beam_model_instances_from_world_at_partial_tick,
    bed_scene::bed_model_instances_from_world,
    bell_scene::bell_model_instances_from_world_at_partial_tick,
    block_entity_visibility::{retain_visible_block_entity_instances, BlockEntityVisibility},
    bounding_box_gizmos::bounding_box_gizmo_lines,
    camera_fov::{camera_fluid_in_camera, CameraFluid, CameraFovTickState},
    camera_pose::{
        camera_eye_position, camera_forward_vector, camera_hurt_effect_from_world,
        camera_pose_from_world, camera_pose_from_world_at_partial_tick,
        camera_view_bob_effect_from_world, current_post_effect_id_from_world,
        frame_camera_from_world_at_partial_tick, render_camera_pose_from_world_at_partial_tick,
        CameraEyeHeightTickState,
    },
    cartography_preview::hud_map_preview_models,
    chest_scene::chest_model_instances_from_world_at_partial_tick,
    code_of_conduct::CodeOfConductAcceptance,
    component_text::{localized_component_runs, visit_localized_template, LocalizedTemplatePart},
    conduit_scene::conduit_model_instances_from_world_at_partial_tick,
    copper_golem_statue_scene::copper_golem_statue_model_instances_from_world,
    crosshair::{
        block_selection_from_hit, can_render_crosshair_for_spectator, crosshair_pick_from_camera,
        crosshair_target_from_camera, debug_looking_at_block_hit_from_camera,
        debug_looking_at_fluid_hit_from_camera, debug_targeted_entity_from_camera,
        BlockSelectionProjection, CrosshairTarget,
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
        advance_creative_inventory_commands,
        advance_destroying_block_at_partial_tick_with_biome_blend_radius,
        advance_local_using_item_ticks_with_effects, advance_player_input_for_client_ticks,
        advance_using_item_at_partial_tick_with_effects_and_presentation, beacon_effect_buttons,
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
        pause_screen_options_button_enabled, pause_screen_report_bugs_button_contains,
        pause_screen_report_bugs_button_enabled, pause_screen_return_to_game_button_contains,
        pause_screen_send_feedback_button_contains, pause_screen_stats_button_contains,
        recipe_book_button_position, recipe_book_main_gui_offset,
        recipe_book_recipe_button_position, recipe_book_tab_count_for_background,
        recipe_book_type_for_background, recipe_book_type_settings,
        recipe_book_visible_tab_indices, release_active_input, stats_screen_done_button_contains,
        sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
        sync_recipe_book_presentation, sync_stonecutter_recipe_scroll_state, AdvancementTabType,
        AdvancementsScreen, ClientInputState, CommandSuggestionHudTooltip,
        CommandSuggestionNarrationState, InventoryScreenBackground, PauseScreenPage,
        RecipeBookOverlayHudState, RecipeBookPageHudState, RecipeBookPresentationHudState,
        RecipeBookSearchHudState, RecipeBookTabSelectionHudState, StatsScreen, StatsScreenFocus,
        BEACON_EFFECT_BUTTON_SIZE, RECIPE_BOOK_BUTTON_HEIGHT, RECIPE_BOOK_BUTTON_WIDTH,
        RECIPE_BOOK_FILTER_BUTTON_HEIGHT, RECIPE_BOOK_FILTER_BUTTON_WIDTH,
        RECIPE_BOOK_FILTER_BUTTON_X, RECIPE_BOOK_FILTER_BUTTON_Y,
        RECIPE_BOOK_PAGE_BACKWARD_BUTTON_X, RECIPE_BOOK_PAGE_BUTTON_HEIGHT,
        RECIPE_BOOK_PAGE_BUTTON_WIDTH, RECIPE_BOOK_PAGE_BUTTON_Y,
        RECIPE_BOOK_PAGE_FORWARD_BUTTON_X, RECIPE_BOOK_RECIPE_BUTTON_SIZE,
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
        block_entity_item_models, display_entity_item_models, dropped_item_models,
        entity_block_models, first_person_item_models_for_frame, first_person_player_arms,
        held_item_models_with_variant_textures, item_pickup_particle_item_models,
        ominous_item_spawner_models, smithing_entity_preview_item_models, world_attribute_keys,
        world_banner_pattern_tooltip_context, world_enchantment_keys,
        world_instrument_tooltip_context, world_jukebox_song_tooltip_context,
        world_painting_variant_tooltip_context, world_trim_material_tooltip_context,
        world_trim_pattern_tooltip_context, FirstPersonHandPresentation, ItemRegistryKeySet,
        MovingPistonVisibility,
    },
    lectern_book_scene::lectern_book_model_instances_from_world,
    level_load_runtime::LevelLoadRuntime,
    music_runtime::MusicRuntime,
    narrator_runtime::{NarratedElementKind, ScreenNarrationFrame, SystemNarrator},
    particle_runtime::{LocalPlayerActionParticleState, NativeParticleRuntime, ParticleTarget},
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
        effects::PlatformEffects, AttackIndicatorOption, SimulationRenderContext, SimulationView,
    },
    skull_scene::skull_model_instances_from_world_at_partial_tick,
    spawner_scene::spawner_display_entity_submissions_from_world_at_partial_tick,
    startup::VANILLA_UNLIMITED_FRAMERATE_LIMIT,
    terrain_upload::{
        maybe_upload_decoded_terrain, maybe_upload_terrain_texture_animation,
        TerrainVisibilityContext,
    },
    test_instance_gizmos::test_instance_error_gizmos,
};
use bbb_client_render::scene::{
    biome_tint::biome_height_adjusted_temperature, terrain_textures::TerrainTextureState,
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
    VANILLA_MOB_EFFECT_NAUSEA_ID, VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
};

#[cfg(test)]
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_REGENERATION_ID, VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_STRENGTH_ID,
};

#[cfg(test)]
use crate::terrain_upload::TerrainUploadState;

mod book_screen;
mod component_hover;
mod toast;
pub(crate) use book_screen::hud_book_component_click_event;
mod control_requests;
mod creative_inventory;
mod debug_light_sections;
mod debug_options_screen;
mod hud_debug;
use hud_debug::{hud_debug_overlay, HudDebugOverlayInputs};
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
use creative_inventory::{
    creative_inventory_narration_frame, hud_creative_inventory_bundle_tooltip,
    hud_creative_inventory_screen,
};
pub(crate) use debug_light_sections::DebugSkyLightSectionCache;
use debug_options_screen::{debug_options_screen_narration_frame, hud_debug_options_screen};
#[cfg(test)]
pub(crate) use dialog_screen::handle_dialog_mouse_input;
pub(crate) use dialog_screen::{
    dialog_ime_cursor_area, handle_dialog_cursor_moved_with_measurer,
    handle_dialog_key_input_with_modifiers_for_surface_and_measurer,
    handle_dialog_mouse_input_with_modifiers_and_measurer, handle_dialog_mouse_wheel,
    handle_dialog_text_input_with_measurer, DialogTextMeasurer,
};
use dialog_screen::{
    hud_dialog_confirm_link_screen, hud_dialog_warning_screen, hud_waiting_response_screen,
};
use game_rules_screen::{hud_game_rules_screen, sync_game_rules_screen_projection};
use overworld_day_timeline::*;
use waypoint_locator::hud_locator_bar_from_world;

/// Vanilla `Gui.POWDER_SNOW_OUTLINE_LOCATION`.
pub(crate) const VANILLA_POWDER_SNOW_OUTLINE_TEXTURE: &str =
    "minecraft:textures/misc/powder_snow_outline.png";
const VANILLA_CLIENT_TICK_INTERVAL: Duration = Duration::from_millis(50);
const VANILLA_MAX_CLIENT_TICKS_PER_STEP: u32 = 10;
pub(crate) use control_requests::{
    apply_code_of_conduct_request, apply_control_net_request, input_key_code, input_mouse_button,
};
pub(crate) use events::LevelEventSoundRandomState;
use periodic_notifications::{PeriodicNotificationRuntime, PeriodicNotificationSchedule};
#[cfg(test)]
use toast::ClientToast;
use toast::{ClientToastRuntime, NowPlayingToastRuntime};

#[derive(Debug, Default)]
pub(crate) struct ClientAnimationTickState {
    last_entity_animation_at: Option<Instant>,
    entity_animation_tick_residual: f64,
    last_entity_tick_work: EntityClientTickWork,
    last_running_level_ticks: u32,
}

impl ClientAnimationTickState {
    pub(crate) fn entity_partial_tick(&self) -> f32 {
        self.entity_animation_tick_residual.clamp(0.0, 1.0) as f32
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
) -> (
    bbb_client_assets::SoundEventRegistry,
    bbb_client_assets::JukeboxSongRegistry,
) {
    events::audio_registry_snapshot_from_world(world)
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

pub(crate) fn localized_command_suggestion_narration(
    item_runtime: Option<&NativeItemRuntime>,
    narration: &CommandSuggestionNarrationState,
) -> String {
    let position = narration.position.to_string();
    let total = narration.total.to_string();
    // The localized template takes plain arguments, so the candidate's units
    // become text only at this entry.
    let text = String::from_utf16_lossy(narration.text.units());
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
            &[&position, &total, &text, tooltip],
        ),
        None => localized_gui_text_with_args(
            item_runtime,
            "narration.suggestion",
            "Selected suggestion %1$s out of %2$s: %3$s",
            &[&position, &total, &text],
        ),
    }
}

pub(crate) fn flush_command_confirmation_narrations(
    input: &mut ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    platform: &mut PlatformEffects,
) {
    for snapshot in input.take_command_confirmation_narrations() {
        if let Some(hud) = hud_command_confirmation_snapshot(&snapshot, item_runtime) {
            platform.submit(
                crate::simulation::effects::PlatformEffect::CommandConfirmationInitialized {
                    frame: Box::new(command_confirmation_snapshot_narration_frame(
                        &snapshot,
                        &hud,
                        item_runtime,
                    )),
                    last_input_type: snapshot.last_input_type,
                    at: snapshot.at,
                },
            );
        }
    }
}

/// Wall-clock milliseconds standing in for vanilla `Util.getMillis()`; render
/// animations only consume it modulo a fixed period (e.g. the forcefield UV
/// scroll's `% 3000L`, `WorldBorderRenderer`), so the epoch is
/// irrelevant.
fn wall_clock_millis() -> u64 {
    bbb_client_clock::system_now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn wall_clock_millis_i64() -> i64 {
    i64::try_from(wall_clock_millis()).unwrap_or(i64::MAX)
}

pub(crate) fn monotonic_millis() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    (START.get_or_init(Instant::now).clock_elapsed().as_millis() as u64).saturating_add(1)
}

fn audio_entity_position_from_world(world: &WorldStore, entity_id: i32) -> Option<[f64; 3]> {
    world
        .probe_entity_transform(entity_id)
        .map(|entity| [entity.position.x, entity.position.y, entity.position.z])
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
            pose.position.y + world.local_player_eye_height(pose),
            pose.position.z,
        ],
        y_rot: pose.y_rot,
        x_rot: pose.x_rot,
    })
}

#[cfg(test)]
mod tests;
