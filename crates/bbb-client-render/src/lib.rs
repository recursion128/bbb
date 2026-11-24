use crate::item_render::generated_item;
use crate::render_core::{
    camera, core_shaders, counters, frame_buffers, frame_flight, gpu, gpu_timing, pipeline_builder,
    sheeted_decal, texture_sampling,
};
use crate::scene::{
    block_destroy, clouds, debug_filled_boxes, debug_text_billboards, item_entities, lightmap,
    outline, panorama, screen_blur, screen_effects, selection, sky, terrain_animation, vignette,
    weather, world_border,
};

mod debug_textures;
mod entity_models;
mod hud;
pub mod item_model;
mod item_models;
pub mod item_render;
#[cfg(not(target_arch = "wasm32"))]
mod offscreen;
pub mod particles;
mod player_skin;
mod post_effect_graph;
mod render;
pub mod render_core;
mod renderer;
mod resource_reload;
pub mod scene;
mod screenshot;
mod text_mesh;
mod transparency;
mod wgsl;

pub mod terrain;

pub use crate::block_destroy::BlockDestroyOverlay;
pub use crate::camera::{
    AabbFrustumIntersection, CameraFrustum, CameraHurtEffect, CameraPose, CameraSpinningEffect,
    CameraViewBobEffect, ClearColor, FogEnvironment, LevelLighting, LightmapEnvironment,
    VANILLA_DEFAULT_LIGHTMAP_AMBIENT_COLOR, VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_BLOCK_LIGHT_TINT, VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_NIGHT_VISION_COLOR, VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR, VANILLA_DEFAULT_RENDER_DISTANCE_CHUNKS,
    VANILLA_MAX_RENDER_DISTANCE_CHUNKS, VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
pub use crate::clouds::{
    CloudEnvironment, CloudFrame, CloudShape, CloudTextureImage, VANILLA_DEFAULT_CLOUD_COLOR,
    VANILLA_DEFAULT_CLOUD_HEIGHT, VANILLA_DEFAULT_CLOUD_RANGE_CHUNKS,
    VANILLA_MAX_CLOUD_RANGE_CHUNKS, VANILLA_MIN_CLOUD_RANGE_CHUNKS,
};
pub use crate::counters::{RendererCounters, TerrainMeshWorkerCounters};
pub use crate::debug_filled_boxes::{DebugCuboidFace, DebugFilledBox, DebugFilledRect};
pub use crate::debug_text_billboards::DebugTextBillboard;
pub use crate::debug_textures::DynamicTextureDumpSummary;
pub use crate::entity_models::BannerPatternTexture;
pub use crate::entity_models::{
    allay_hand_attach_transform, copper_golem_antenna_block_transform,
    copper_golem_hand_attach_transform, custom_head_item_transform, custom_head_item_transforms,
    dolphin_carried_item_transform, enderman_carried_block_transform, falling_block_transform,
    fox_held_item_transform, humanoid_hand_attach_transform, humanoid_hand_attach_transforms,
    iron_golem_flower_block_transform, minecart_display_block_transform,
    minecart_tnt_display_block_transform, mooshroom_mushroom_block_transforms,
    panda_held_item_transform, player_held_to_eye_transform, primed_tnt_block_transform,
    snow_golem_head_block_transform, villager_crossed_arms_item_transform,
    witch_held_item_transform,
};
pub use crate::entity_models::{
    bake_entity_flame_surface, bake_entity_shadow_surface, entity_model_texture_refs,
    ArmorStandModelPose, ArrowModelTexture, AxolotlModelVariant, BannerPatternKind,
    BannerPatternLayer, BeaconBeamRenderState, BedModelPart, BellShakeDirection,
    BlockEntityCrumbling, BlockEntityRenderState, BoatModelFamily, CamelModelFamily,
    CatModelVariant, ChestModelHalf, ChestModelTexture, ChickenModelVariant, ConduitModelPart,
    CopperGolemWeathering, CowModelVariant, DecoratedPotPattern, DecoratedPotWobble,
    DonkeyModelFamily, DragonFlightHistorySample, EndCrystalBeamRenderState,
    EndGatewayBeamRenderState, EndPortalModelFace, EndPortalModelFaces, EndPortalModelKind,
    EnderDragonAnimationRenderState, EnderDragonBeamRenderState, EntityArmorTrim,
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDyeColor,
    EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture,
    EntityDynamicPlayerTextureKind, EntityDynamicTexture, EntityEquipmentAsset,
    EntityEquipmentAssetCatalog, EntityEquipmentAssetId, EntityEquipmentAssetLayer,
    EntityEquipmentAssetLayerTexture, EntityEquipmentDyeable, EntityEquipmentLayerType,
    EntityFlameSubmission, EntityFlameSurface, EntityLeashRopesRenderState, EntityModelInstance,
    EntityModelKind, EntityModelOwnedTextureImage, EntityModelOwnedTextureRef,
    EntityModelTextureImage, EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel,
    EntityRenderState, EntityShadowPiece, EntityShadowSubmission, EntityShadowSurface,
    FirstPersonPlayerArm, FoxModelVariant, FrogModelVariant, GuardianBeamRenderState,
    GuardianEyeTrackingRenderState, HoglinModelFamily, HorseColorVariant, HorseMarkings,
    IllagerModelFamily, IronGolemCrackiness, LeashRopeRenderState, LlamaModelFamily, LlamaVariant,
    MooshroomVariant, PandaModelVariant, ParrotModelVariant, PigModelVariant, PiglinModelFamily,
    PlayerModelPartVisibility, RabbitModelVariant, SalmonModelSize, SheepHeadEatPose,
    SheepWoolColor, SignModelAttachment, SignModelWood, SkeletonModelFamily,
    SkullBlockModelAttachment, SleepingPose, SpawnerDisplayRenderState, SpearKineticUseParams,
    SpearKineticWeapon, TropicalFishModelShape, TropicalFishPattern, UndeadHorseModelFamily,
    VillagerModelData, VillagerModelProfession, VillagerModelType, WolfArmorCrackiness,
    WolfModelVariant, ZombieNautilusModelVariant, ZombieVariantModelFamily,
    DEFAULT_ARMOR_STAND_MODEL_POSE, ENTITY_DEFAULT_OUTLINE_COLOR, ENTITY_FLAME_FIRE_0_SPRITE,
    ENTITY_FLAME_FIRE_1_SPRITE, ENTITY_FULL_BRIGHT_LIGHT_COORDS, ENTITY_SHADOW_TEXTURE_PATH,
    PLAYER_MODEL_PARTS_ALL_HIDDEN, PLAYER_MODEL_PARTS_ALL_VISIBLE,
};
pub use crate::generated_item::{bake_generated_item_quads, ItemSpriteRect, SpriteAlphaMask};
pub use crate::gpu_timing::{RendererGpuStageProfile, RendererGpuTimingProfile};
pub use crate::hud::HudTutorialToastLines;
pub use crate::hud::GUI_TEXTURES;
pub use crate::hud::{
    hud_air_supply_pop, hud_air_supply_visible, hud_book_layout,
    hud_game_rules_confirmation_buttons, hud_game_rules_layout,
    hud_inventory_text_field_ime_cursor_area, hud_plain_text_line_ranges,
    hud_text_field_preedit_overlay,
};
pub use crate::hud::{hud_coded_string_runs, hud_coded_string_width};
pub use crate::hud::{
    hud_command_block_editor_layout, hud_creative_inventory_background_layers,
    hud_creative_inventory_scroller_y, hud_creative_inventory_slot_layouts,
    hud_creative_inventory_tab_hit_y, hud_creative_inventory_tab_x, hud_jigsaw_block_editor_layout,
    hud_minecart_command_block_editor_layout, hud_sign_editor_done_button_rect,
    hud_social_interactions_layout, hud_social_interactions_max_scroll_amount,
    hud_structure_block_editor_layout, hud_test_block_editor_layout,
    hud_test_instance_block_editor_layout, hud_wrap_styled_text, HudActionBarText,
    HudAdvancementBackgroundTexture, HudAdvancementHoverBoxSprite, HudAdvancementLineTexture,
    HudAdvancementTabSprite, HudAdvancementToast, HudAdvancementWidgetFrameSprite, HudAirBubblePop,
    HudAirSupply, HudAsciiGlyph, HudBookHitTarget, HudBookInteraction, HudBookLayout, HudBookMode,
    HudBookScreen, HudBossBar, HudBossBarColor, HudBossBarOverlay, HudBundleTooltip,
    HudCameraOverlay, HudChatAppendedLines, HudChatContext, HudChatHitTarget, HudChatHitTargetKind,
    HudChatInputView, HudChatInteraction, HudChatMessage, HudChatOptions,
    HudChatRestrictionsScreen, HudChatScroll, HudChatScrollPage, HudChatTag, HudChatTagKind,
    HudCodeOfConductRect, HudCodeOfConductScreen, HudCommandBlockEditorKind,
    HudCommandBlockEditorLayout, HudCommandBlockEditorRect, HudCommandBlockEditorScreen,
    HudCommandSuggestionTooltip, HudCommandSuggestions, HudCommandSuggestionsLayout,
    HudComponentTextRun, HudConfusionOverlay, HudContextualBar, HudCreativeInventoryBackground,
    HudCreativeInventoryChrome, HudCreativeInventorySlot, HudCreativeInventorySlotLayout,
    HudCreativeInventorySlotLayoutKind, HudCreativeInventoryTab, HudCreativeInventoryTabRow,
    HudCrosshairAttackIndicator, HudDeathLinkConfirmationScreen, HudDeathMainScreen,
    HudDeathQuitConfirmationScreen, HudDeathScreen, HudDeathScreenButton, HudDeathScreenHitTarget,
    HudDeathScreenHitTargetKind, HudDebugCrosshair, HudDebugFrameTimeChart,
    HudDebugGameModeSwitcher, HudDebugGameModeSwitcherSlot, HudDebugNetworkCharts,
    HudDebugOptionsEntryStatus, HudDebugOptionsRow, HudDebugOptionsScreen, HudDebugOptionsTooltip,
    HudDebugOverlay, HudDebugProfilerChart, HudDebugProfilerSlice, HudDebugTpsChart,
    HudDebugTpsSample, HudDemoLinkConfirmationScreen, HudDemoPopupScreen, HudDigitGlyph,
    HudDisconnectedScreen, HudDisconnectedScreenButton, HudDisconnectedScreenHitTarget,
    HudEntityPreview, HudEntityPreviewItemDisplayContext, HudEntityPreviewItemLayer,
    HudEntityPreviewItemSlot, HudEntityPreviewRect, HudFontGlyphBook, HudFontGlyphMap,
    HudFoodEffect, HudGameEventButton, HudGameEventHitTarget, HudGameEventScreen,
    HudGameRulesDiscardConfirmation, HudGameRulesLayout, HudGameRulesRect, HudGameRulesRow,
    HudGameRulesScreen, HudGuiSpriteScaling, HudHeartKind, HudHotbarAttackIndicator, HudHotbarMode,
    HudIconLayer, HudImePreedit, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryBannerPatternPreview, HudInventoryFillLayer, HudInventoryFillStage,
    HudInventoryGhostItem, HudInventoryItem, HudInventoryItemScissor, HudInventoryMapPreview,
    HudInventoryPreeditOverlay, HudInventoryScreen, HudInventoryScrollArea, HudInventoryScrollbar,
    HudInventoryScrollingText, HudInventorySlot, HudInventoryTextArea, HudInventoryTextBackground,
    HudInventoryTextInputDecoration, HudInventoryTextLabel, HudInventoryTooltip,
    HudInventoryTooltipLine, HudItemCountLabel, HudItemDurabilityBar, HudItemFoil, HudItemIcon,
    HudJigsawBlockEditorLayout, HudJigsawBlockEditorScreen, HudJumpBar, HudLevelLoadingBackground,
    HudLevelLoadingScreen, HudLocatorBar, HudLocatorMarker, HudLocatorPitch, HudLocatorStyle,
    HudMobEffect, HudNineSliceScaling, HudNowPlayingToast, HudNowPlayingToastPlacement,
    HudOffhandSlot, HudPauseScreen, HudPauseScreenLabels, HudPauseScreenTooltip, HudPlayerHealth,
    HudPlayerList, HudPlayerListEntry, HudPlayerListPingSprite, HudPlayerListScore,
    HudPortalOverlay, HudRecipeToast, HudRichTooltip, HudRichTooltipLine, HudScoreboardSidebar,
    HudScoreboardSidebarEntry, HudSelectedItemName, HudSignEditorKind, HudSignEditorScreen,
    HudSleepOverlay, HudSmithingSlotIcon, HudSocialInteractionsFocus,
    HudSocialInteractionsHitTarget, HudSocialInteractionsHitTargetKind,
    HudSocialInteractionsLayout, HudSocialInteractionsLinkConfirmationScreen,
    HudSocialInteractionsMainScreen, HudSocialInteractionsPage, HudSocialInteractionsPlayer,
    HudSocialInteractionsPlayerAction, HudSocialInteractionsRect, HudSocialInteractionsScreen,
    HudSoundSubtitleDirection, HudSoundSubtitleRow, HudSoundSubtitles, HudSpectatorGui,
    HudSpectatorGuiSlot, HudSpectatorIcon, HudSpyglassScope, HudStatsItemRow, HudStatsRow,
    HudStatsScreen, HudStatsScrollbarSprite, HudStatsSprite, HudStructureBlockEditorLayout,
    HudStructureBlockEditorScreen, HudStructureBlockMode, HudStyledTextRun, HudSystemToast,
    HudSystemToastInstance, HudTestBlockEditorLayout, HudTestBlockEditorScreen,
    HudTestInstanceBlockEditorLayout, HudTestInstanceBlockEditorScreen, HudTextField, HudTextStyle,
    HudTitleText, HudToast, HudTutorialToast, HudTutorialToastIcon, HudUvRect, HudVehicleHealth,
    HudWinScreen, HudWinScreenLine, HUD_BOOK_HEIGHT, HUD_BOOK_LINE_HEIGHT, HUD_BOOK_MAX_LINES,
    HUD_BOOK_PAGE_BACK_X, HUD_BOOK_PAGE_BUTTON_HEIGHT, HUD_BOOK_PAGE_BUTTON_WIDTH,
    HUD_BOOK_PAGE_BUTTON_Y, HUD_BOOK_PAGE_FORWARD_X, HUD_BOOK_PAGE_INDICATOR_RIGHT,
    HUD_BOOK_PAGE_INDICATOR_Y, HUD_BOOK_TEXT_WIDTH, HUD_BOOK_TEXT_X, HUD_BOOK_TEXT_Y, HUD_BOOK_TOP,
    HUD_BOOK_WIDTH, HUD_CREATIVE_INVENTORY_HEIGHT, HUD_CREATIVE_INVENTORY_WIDTH,
    HUD_CREATIVE_PICKER_COLUMNS, HUD_CREATIVE_PICKER_ROWS, HUD_CREATIVE_PICKER_SLOT_COUNT,
    HUD_CREATIVE_SCROLLER_X, HUD_CREATIVE_SCROLLER_Y, HUD_CREATIVE_SEARCH_HEIGHT,
    HUD_CREATIVE_SEARCH_WIDTH, HUD_CREATIVE_SEARCH_X, HUD_CREATIVE_SEARCH_Y,
    HUD_CREATIVE_TAB_HEIGHT, HUD_CREATIVE_TAB_WIDTH, HUD_CREATIVE_TRASH_X, HUD_CREATIVE_TRASH_Y,
    HUD_FONT_BASELINE, HUD_GAME_RULES_FOOTER_HEIGHT, HUD_GAME_RULES_HEADER_HEIGHT,
    HUD_GAME_RULES_LIST_WIDTH, HUD_GAME_RULES_ROW_HEIGHT, HUD_GAME_RULES_SEARCH_HEIGHT,
    HUD_GAME_RULES_SEARCH_WIDTH, HUD_GAME_RULES_VALUE_HEIGHT, HUD_GAME_RULES_VALUE_WIDTH,
    HUD_HOTBAR_SLOTS, HUD_SOUND_SUBTITLE_MAX_ROWS, HUD_TOAST_SLOTS,
    VANILLA_DEFAULT_TEXT_BACKGROUND_OPACITY,
};
pub use crate::hud::{
    hud_component_text_width, hud_formatted_text_width, hud_formatted_utf16_line_ranges,
    hud_formatted_utf16_width, hud_logical_component_text_width_f32,
    hud_resolve_formatted_component_text, hud_wrap_component_text,
    hud_wrap_formatted_component_text,
};
pub use crate::hud::{hud_resolve_formatted_utf16_text, hud_stats_tab_rects, HUD_STATS_LIST_TOP};
pub use crate::item_entities::{
    ItemEntityBillboard, ItemEntityBillboardLayer, ItemEntityBillboardOrientation, ItemEntityUvRect,
};
pub use crate::item_models::{
    bake_display_text_surface, bake_entity_name_tag_surfaces,
    bake_first_person_map_background_surface, bake_first_person_map_decoration_surface,
    bake_first_person_map_text_surface, bake_hud_map_decoration_surface, bake_hud_map_text_surface,
    bake_item_frame_map_decoration_surface, bake_item_frame_map_surface,
    bake_item_frame_map_text_surface, bake_sign_text_surface, entity_name_tag_root_position,
    item_frame_map_decoration_type, item_frame_map_text_width, sign_line_runs_width,
    sign_max_text_line_width, sign_text_base_color, sign_text_dark_color, sign_text_line_height,
    sign_text_scaled_rgb, sign_text_transformation, truncate_sign_line_runs_to_width,
    DisplayTextAlignment, DisplayTextLayerSubmission, DisplayTextLine, DisplayTextRenderType,
    DisplayTextSubmission, DisplayTextSurface, EntityNameTagLayerSubmission, EntityNameTagLineKind,
    EntityNameTagRenderType, EntityNameTagSubmission, EntityNameTagSurface,
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundSubmission,
    FirstPersonMapBackgroundSurface, FirstPersonMapBackgroundTexture,
    FirstPersonMapBackgroundTextureRef, GuiItemLightingEntry, HudBlockItemModel,
    HudMapPreviewSurface, HudResidentItemLayer, ItemFrameMapDecorationSubmission,
    ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture, ItemFrameMapDecorationTextureRef,
    ItemFrameMapDecorationType, ItemFrameMapRenderType, ItemFrameMapSubmission,
    ItemFrameMapSurface, ItemFrameMapTextSubmission, ItemFrameMapTextSurface,
    ItemFrameMapTextTextureRef, ItemFrameMapTexture, ItemFrameMapTexturePatch,
    ItemFrameMapTextureRef, ItemModelAssetIdentity, ItemModelAssetKey, ItemModelAssetRequest,
    ItemModelAssetSource, ItemModelAtlasGeneration, ItemModelAtlasKind, ItemModelDrawPhase,
    ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass, ItemModelInstanceRecord,
    ItemModelInstanceRequest, ItemModelMesh, ItemModelQuad, ItemModelResidencyFrame,
    ItemModelResidencyOutcome, ItemModelResourceGeneration, ItemModelScissorRect,
    SignTextOutlineContext, SignTextSubmission, SignTextSurface, SpecialItemBannerAttachment,
    SpecialItemCopperGolemPose, SpecialItemEndCubeEffect, SpecialItemHeadKind,
    SpecialItemInstanceRequest, SpecialItemModel, SpecialItemModelBounds, SpecialItemPatternLayer,
    SpecialItemTexture, WorldTextRun, DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY,
    DISPLAY_TEXT_FONT_TEXTURE, DISPLAY_TEXT_LINE_HEIGHT, ENTITY_NAME_TAG_BACKGROUND_OPACITY,
    ENTITY_NAME_TAG_FONT_TEXTURE, ENTITY_NAME_TAG_SCALE, HANGING_SIGN_MAX_TEXT_LINE_WIDTH,
    HANGING_SIGN_TEXT_LINE_HEIGHT, ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
    SIGN_MAX_TEXT_LINE_WIDTH, SIGN_TEXT_LINE_HEIGHT,
};
pub use crate::particles::{
    ItemPickupParticleRenderState, ParticleItemOptionState, ParticleItemPickupProjectileKind,
    ParticleSpriteUv, ParticleUvRect,
};
pub use crate::player_skin::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicEntityTextureImage,
    DynamicPlayerSkinImage, DynamicPlayerTextureImage,
};
#[cfg(feature = "lifecycle-soak")]
pub use crate::renderer::LifecycleSoakSurfaceError;
pub use crate::renderer::{
    set_force_software_adapter, LevelFrameRoute, Renderer, RendererCameraFrame,
    RendererFrameProfile, RendererInputLatencyProfile, RendererInputLatencyStamp,
    TerrainChunkMeshOwner, TerrainSectionMeshOwner, TerrainSectionRenderState,
    TerrainTextureFiltering, NAGA_VERSION, WGPU_CORE_VERSION, WGPU_HAL_VERSION,
    WGPU_METAL_FENCE_STRATEGY, WGPU_METAL_FENCE_UPSTREAM_COMMIT, WGPU_TYPES_VERSION, WGPU_VERSION,
};
pub use crate::scene::lod_terrain::LodTerrainVertex;
pub use crate::screen_blur::{
    VANILLA_DEFAULT_MENU_BACKGROUND_BLURRINESS, VANILLA_MAX_MENU_BACKGROUND_BLURRINESS,
};
pub use crate::screen_effects::{ScreenEffectQuad, ScreenEffectTexture};
#[cfg(target_arch = "wasm32")]
pub use crate::screenshot::set_web_png_encode_dispatcher;
pub use crate::screenshot::{CapturedScreenshot, RenderOutcome, ScreenshotPixels};
pub use crate::selection::{
    BlockSelectionPhase, BlockSelectionRenderState, DebugLine, SelectionBox, SelectionColoredBox,
    SelectionLine, SelectionOutline, SelectionPoint, SelectionTextLabel,
};
pub use crate::sky::{
    CelestialTextureImage, CelestialTextureKind, SkyEnvironment, SkyMoonPhase, SkyboxKind,
};
pub use crate::text_mesh::{next_text_layout_frame, push_world_text};
pub use crate::vignette::{HudVignette, VIGNETTE_TEXTURE_PATH};
pub use crate::weather::{
    LightningBoltRenderState, WeatherColumn, WeatherFrame, WeatherRenderState, WeatherTextureImage,
    WeatherTextureKind, WEATHER_RAIN_TEXTURE_PATH, WEATHER_SNOW_TEXTURE_PATH,
};
pub use crate::world_border::{WorldBorderRenderState, WORLD_BORDER_FORCEFIELD_TEXTURE_PATH};
pub use bbb_render_types::{
    DataDrivenRigSpec, DynamicLightsMode, DynamicPointLight, EntityPlayerTextures,
    HudCommandInputRange, HudCommandInputSyntax, HudCommandSuggestionEntry,
    StandaloneTextureSampling, MAX_DYNAMIC_POINT_LIGHTS,
};

pub use crate::hud::{
    HudCommandConfirmation, HudCommandConfirmationButton, HudCommandConfirmationHitTarget,
};
pub use hud::{
    hud_debug_options_content_x, hud_debug_options_footer_button_xs,
    hud_debug_options_footer_button_y, hud_debug_options_search_box_rects,
    hud_pause_advancements_button_rect, hud_pause_disconnect_button_rect,
    hud_pause_feedback_back_button_rect, hud_pause_feedback_report_bugs_button_rect,
    hud_pause_feedback_send_feedback_button_rect, hud_pause_options_button_rect,
    hud_pause_report_bugs_button_rect, hud_pause_return_to_game_button_rect,
    hud_pause_send_feedback_button_rect, hud_pause_stats_button_rect, hud_stats_done_button_rect,
};

/// Seed for obfuscated (`§k`) glyphs. Vanilla draws fresh random glyphs every render; the
/// seed comes from the injected clock so a replay at the same virtual time draws the same ones.
pub fn text_obfuscation_seed() -> u64 {
    bbb_client_clock::system_now()
        .duration_since(bbb_client_clock::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}
