use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{
    AudioCounters, NetCounters, RendererCounters, SharedControlRequests, SharedSnapshot,
};
use bbb_item_model::{ItemModelKeybindContext, NativeItemRuntime};
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::{
    codec::Decoder,
    packets::{
        ItemCostSummary, ItemStackSummary, MapPostProcessingSummary, SlotDisplaySummary, Vec3d,
    },
};
use bbb_renderer::{
    BlockDestroyOverlay, CameraPose, ClearColor, CloudEnvironment, CloudFrame, EntityModelInstance,
    FogEnvironment, GuiItemLightingEntry, HudBlockItemModel, HudEntityPreview,
    HudEntityPreviewItemDisplayContext, HudEntityPreviewItemLayer, HudEntityPreviewItemSlot,
    HudEntityPreviewRect, HudIconLayer, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryItem, HudInventoryScreen, HudInventorySlot, HudInventoryTextBackground,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine, HudItemCountLabel,
    HudItemDurabilityBar, HudItemFoil, HudItemIcon, HudUvRect, LevelLighting, LightmapEnvironment,
    LightningBoltRenderState, ParticleBlockFluidSurfaceSample, ParticleEntityTargetContext,
    ParticleFluidKind, ParticleLocalPlayerScopeContext, ParticlePlayerMotionContext,
    ParticleSoundEvent, ParticleSpawnBatch, ParticleSpawnCommand, Renderer, SkyEnvironment,
    SkyMoonPhase, WeatherColumn, WeatherFrame, WeatherRenderState, DEFAULT_ARMOR_STAND_MODEL_POSE,
    ENTITY_FULL_BRIGHT_LIGHT_COORDS, HUD_HOTBAR_SLOTS, ITEM_MODEL_NO_OVERLAY,
    VANILLA_DEFAULT_CLOUD_COLOR, VANILLA_DEFAULT_CLOUD_HEIGHT,
    VANILLA_DEFAULT_LIGHTMAP_BLOCK_FACTOR, VANILLA_DEFAULT_LIGHTMAP_BRIGHTNESS_FACTOR,
    VANILLA_DEFAULT_LIGHTMAP_SKY_FACTOR, VANILLA_DEFAULT_LIGHTMAP_SKY_LIGHT_COLOR,
    VANILLA_MAX_RENDER_DISTANCE_CHUNKS, VANILLA_MIN_RENDER_DISTANCE_CHUNKS,
};
use bbb_world::{
    BlockPos, BookScreenState, ContainerState, EvokerFangsCritParticleState,
    FireworkRocketTrailParticleState, ItemEquipmentSlot, MerchantOfferState, MerchantOffersState,
    MobEffectState, MountArmorSlotKind, MountInventoryKind, OminousItemSpawnerParticleState,
    PrimedTntSmokeParticleState, RavagerStunParticleState, SoundEventState, SoundHolderState,
    TerrainFluidKind, TerrainFluidState, TerrainLight, TerrainMaterialClass, WorldLevelInfo,
    WorldStore, WorldWeatherState,
};
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    biome_tint::biome_height_adjusted_temperature,
    camera_pose::camera_pose_from_world,
    code_of_conduct::CodeOfConductAcceptance,
    crosshair::{entity_target_outline_from_camera_at_partial_tick, selection_outline_from_camera},
    entity_scene::{
        armor_material, entity_model_instance_from_world_entity_at_partial_tick,
        entity_model_instances_from_world_at_partial_tick,
        entity_scene_outline_from_world_at_partial_tick,
    },
    input::{
        advance_destroying_block_at_partial_tick, advance_player_input,
        advance_using_item_at_partial_tick, inventory_screen_layout,
        inventory_screen_selected_hotbar_slot_id, release_active_input,
        sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
        sync_stonecutter_recipe_scroll_state, ClientInputState, InventoryScreenBackground,
    },
    item_entities::item_entity_billboards_from_world,
    item_frames::item_frame_models,
    item_models::{
        dropped_item_models, entity_block_models, first_person_item_models,
        first_person_player_arms, held_item_models, item_pickup_particle_item_models,
        ominous_item_spawner_models,
    },
    particle_runtime::{
        ParticleEventSink, CRIT_PARTICLE_TYPE_ID, ENTITY_EFFECT_PARTICLE_TYPE_ID,
        SMOKE_PARTICLE_TYPE_ID,
    },
    terrain_runtime::{
        maybe_upload_decoded_terrain, maybe_upload_terrain_texture_animation, TerrainTextureState,
        TerrainUploadState,
    },
};
use bbb_protocol::entity_types::*;

mod control_requests;
mod render_extract;
pub(crate) use render_extract::*;
mod events;

const CLIENT_ENTITY_ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(50);
const VANILLA_OVERWORLD_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-16_119_286);
const VANILLA_NETHER_SKY_LIGHT_COLOR: [f32; 3] = [122.0 / 255.0, 122.0 / 255.0, 1.0];
const VANILLA_NETHER_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-13_621_215);
const VANILLA_END_SKY_LIGHT_COLOR: [f32; 3] = rgb24(-5_480_243);
const VANILLA_END_AMBIENT_LIGHT_COLOR: [f32; 3] = rgb24(-12_630_209);
const VANILLA_LIGHTMAP_DEFAULT_DAY_TIME: i64 = 6_000;
const VANILLA_LIGHTMAP_DAY_PERIOD_TICKS: i64 = 24_000;
const VANILLA_TIMELINE_NIGHT_SKY_LIGHT_COLOR: i32 = argb_color(255, 122, 122, 255);
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
const VANILLA_TIMELINE_NIGHT_SKY_COLOR_MULTIPLIER: i32 = argb_color(255, 0, 0, 0);
const VANILLA_TIMELINE_NIGHT_FOG_COLOR_MULTIPLIER: i32 = argb_color(255, 15, 15, 22);
const VANILLA_TIMELINE_NIGHT_CLOUD_COLOR_MULTIPLIER: i32 = argb_color(255, 25, 25, 38);
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
const VANILLA_GAUSSIAN_SAMPLE_KERNEL: [f64; 7] = [0.0, 1.0, 4.0, 6.0, 4.0, 1.0, 0.0];
const VANILLA_SKY_FLASH_SKY_COLOR: i32 = argb_color(255, 204, 204, 255);
const VANILLA_SKY_FLASH_SKY_COLOR_ALPHA: f32 = 0.22;
const VANILLA_PARTICLE_MISSING_CHUNK_LIGHT: TerrainLight = TerrainLight { sky: 15, block: 15 };
const VANILLA_WORLD_CLOCK_THE_END_ID: i32 = 1;
const VANILLA_END_FLASH_INTERVAL_TICKS: i64 = 600;
const VANILLA_END_FLASH_MAX_OFFSET_TICKS: i32 = 200;
const VANILLA_END_FLASH_MIN_DURATION_TICKS: i32 = 100;
const VANILLA_END_FLASH_MAX_DURATION_TICKS: i32 = 380;
const VANILLA_BOSS_OVERLAY_DARKEN_IN_STEP: f32 = 0.05;
const VANILLA_BOSS_OVERLAY_DARKEN_OUT_STEP: f32 = 0.0125;
const VANILLA_OVERWORLD_SKY_LIGHT_COLOR_KEYFRAMES: [(i64, i32); 4] = [
    (730, -1),
    (11_270, -1),
    (13_140, VANILLA_TIMELINE_NIGHT_SKY_LIGHT_COLOR),
    (22_860, VANILLA_TIMELINE_NIGHT_SKY_LIGHT_COLOR),
];
const VANILLA_OVERWORLD_SKY_LIGHT_FACTOR_KEYFRAMES: [(i64, f32); 4] =
    [(730, 1.0), (11_270, 1.0), (13_140, 0.24), (22_860, 0.24)];
const VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES: [(i64, i32); 4] = [
    (133, -1),
    (11_867, -1),
    (13_670, VANILLA_TIMELINE_NIGHT_SKY_COLOR_MULTIPLIER),
    (22_330, VANILLA_TIMELINE_NIGHT_SKY_COLOR_MULTIPLIER),
];
const VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES: [(i64, i32); 4] = [
    (133, -1),
    (11_867, -1),
    (13_670, VANILLA_TIMELINE_NIGHT_FOG_COLOR_MULTIPLIER),
    (22_330, VANILLA_TIMELINE_NIGHT_FOG_COLOR_MULTIPLIER),
];
const VANILLA_OVERWORLD_CLOUD_COLOR_MULTIPLIER_KEYFRAMES: [(i64, i32); 4] = [
    (133, -1),
    (11_867, -1),
    (13_670, VANILLA_TIMELINE_NIGHT_CLOUD_COLOR_MULTIPLIER),
    (22_330, VANILLA_TIMELINE_NIGHT_CLOUD_COLOR_MULTIPLIER),
];
const VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES: [(i64, i32); 32] = [
    (71, 1_609_540_403),
    (310, 703_969_843),
    (565, 117_167_155),
    (730, 16_770_355),
    (11_270, 16_770_355),
    (11_397, 83_679_283),
    (11_522, 268_028_723),
    (11_690, 703_969_843),
    (11_929, 1_609_540_403),
    (12_243, -1_310_226_637),
    (12_358, -857_440_717),
    (12_512, -371_166_669),
    (12_613, -153_261_261),
    (12_732, -19_242_189),
    (12_841, -19_440_589),
    (13_035, -321_760_973),
    (13_252, -1_043_577_037),
    (13_775, 918_435_635),
    (13_888, 532_362_547),
    (14_039, 163_001_139),
    (14_192, 11_744_051),
    (21_807, 11_678_515),
    (21_961, 163_001_139),
    (22_112, 532_362_547),
    (22_225, 918_435_635),
    (22_748, -1_043_577_037),
    (22_965, -321_760_973),
    (23_159, -19_440_589),
    (23_272, -19_242_189),
    (23_488, -371_166_669),
    (23_642, -857_440_717),
    (23_757, -1_310_226_637),
];
const VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES: [(i64, f32); 12] = [
    (92, 0.037),
    (627, 0.0),
    (11_373, 0.0),
    (11_732, 0.016),
    (11_959, 0.044),
    (12_399, 0.143),
    (12_729, 0.258),
    (13_228, 0.5),
    (22_772, 0.5),
    (23_032, 0.364),
    (23_356, 0.225),
    (23_758, 0.101),
];
const VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK: f32 = 1.0;
const VANILLA_MOB_EFFECT_NIGHT_VISION_ID: i32 = 15;
const VANILLA_MOB_EFFECT_CONDUIT_POWER_ID: i32 = 28;
const VANILLA_MOB_EFFECT_DARKNESS_ID: i32 = 32;
const VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS: i32 = 200;
const VANILLA_DARKNESS_BLEND_OUT_ADVANCE_TICKS: i32 = 22;
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
const BEACON_EFFECT_SPEED_ID: i32 = 0;
const BEACON_EFFECT_HASTE_ID: i32 = 2;
const BEACON_EFFECT_STRENGTH_ID: i32 = 4;
const BEACON_EFFECT_JUMP_BOOST_ID: i32 = 7;
const BEACON_EFFECT_REGENERATION_ID: i32 = 9;
const BEACON_EFFECT_RESISTANCE_ID: i32 = 10;
const BEACON_PRIMARY_EFFECT_ROWS: [&[i32]; 3] = [
    &[BEACON_EFFECT_SPEED_ID, BEACON_EFFECT_HASTE_ID],
    &[BEACON_EFFECT_RESISTANCE_ID, BEACON_EFFECT_JUMP_BOOST_ID],
    &[BEACON_EFFECT_STRENGTH_ID],
];
const BEACON_SECONDARY_EFFECTS: &[i32] = &[BEACON_EFFECT_REGENERATION_ID];
const BEACON_CONFIRM_BUTTON_X: i32 = 164;
const BEACON_CANCEL_BUTTON_X: i32 = 190;
const BEACON_ACTION_BUTTON_Y: i32 = 107;
const BEACON_ACTION_BUTTON_SIZE: u32 = 22;
const BEACON_ACTION_ICON_OFFSET: i32 = 2;
const BEACON_ACTION_ICON_SIZE: u32 = 18;
const ENCHANTING_TABLE_LAPIS_SLOT_SPRITE_SIZE: u32 = 16;
const ENCHANTING_TABLE_OPTION_COUNT: i16 = 3;
const ENCHANTING_TABLE_OPTION_X: i32 = 60;
const ENCHANTING_TABLE_OPTION_Y: i32 = 14;
const ENCHANTING_TABLE_OPTION_WIDTH: u32 = 108;
const ENCHANTING_TABLE_OPTION_HEIGHT: u32 = 19;
const ENCHANTING_TABLE_OPTION_SPACING: i32 = 19;
const ENCHANTING_TABLE_LEVEL_ICON_X_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_Y_OFFSET: i32 = 1;
const ENCHANTING_TABLE_LEVEL_ICON_SIZE: u32 = 16;
const ENCHANTING_TABLE_COST_TEXT_X_RIGHT: i32 = 166;
const ENCHANTING_TABLE_COST_TEXT_Y_OFFSET: i32 = 23;
const ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR: [f32; 4] = [128.0 / 255.0, 1.0, 32.0 / 255.0, 1.0];
const ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR: [f32; 4] =
    [64.0 / 255.0, 127.0 / 255.0, 16.0 / 255.0, 1.0];
const BOOK_PAGE_INDICATOR_X_RIGHT: i32 = 148;
const BOOK_PAGE_INDICATOR_Y: i32 = 16;
const BOOK_PAGE_TEXT_X: i32 = 36;
const BOOK_PAGE_TEXT_Y: i32 = 30;
const BOOK_PAGE_TEXT_WIDTH: u32 = 114;
const BOOK_PAGE_TEXT_HEIGHT: u32 = 128;
const BOOK_PAGE_LINE_HEIGHT: i32 = 9;
const BOOK_TEXT_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
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
const MAP_ID_DATA_COMPONENT_TYPE_ID: i32 = 41;
const CARTOGRAPHY_TABLE_MAP_SLOT: i16 = 0;
const CARTOGRAPHY_TABLE_ADDITIONAL_SLOT: i16 = 1;
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
const LOOM_MENU_TYPE_ID: i32 = 18;
const LOOM_SLOT_SPRITE_SIZE: u32 = 16;
const LOOM_SELECTED_PATTERN_DATA_ID: i16 = 0;
const LOOM_PATTERN_BUTTON_X: i32 = 60;
const LOOM_PATTERN_BUTTON_Y: i32 = 13;
const LOOM_PATTERN_BUTTON_COLUMNS: i32 = 4;
const LOOM_PATTERN_BUTTON_ROWS: i32 = 4;
const LOOM_PATTERN_BUTTON_SIZE: u32 = 14;
const LOOM_SCROLLER_WIDTH: u32 = 12;
const LOOM_SCROLLER_HEIGHT: u32 = 15;
const LOOM_SCROLLER_X: i32 = 119;
const LOOM_SCROLLER_Y: i32 = 13;
const LOOM_SCROLLER_MAX_OFFSET: i32 = 41;
const LOOM_NO_ITEM_REQUIRED_PATTERN_COUNT: i32 = 32;
const LOOM_PATTERN_ITEM_PATTERN_COUNT: i32 = 1;
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
const MERCHANT_TRADE_COST_A_X: i32 = 10;
const MERCHANT_TRADE_COST_B_X: i32 = 40;
const MERCHANT_TRADE_RESULT_X: i32 = 73;
const MERCHANT_TRADE_ITEM_Y: i32 = 19;
const MERCHANT_TRADE_ROW_HEIGHT: i32 = 20;
const MERCHANT_TRADE_ARROW_X: i32 = 60;
const MERCHANT_TRADE_ARROW_Y: i32 = 22;
const MERCHANT_TRADE_ARROW_WIDTH: u32 = 10;
const MERCHANT_TRADE_ARROW_HEIGHT: u32 = 9;
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
const MERCHANT_XP_BAR_X: i32 = 136;
const MERCHANT_XP_BAR_Y: i32 = 16;
const MERCHANT_XP_BAR_WIDTH: u32 = 102;
const MERCHANT_XP_BAR_HEIGHT: u32 = 5;
const STONECUTTER_SELECTED_RECIPE_DATA_ID: i16 = 0;
const STONECUTTER_VISIBLE_RECIPE_BUTTON_COUNT: usize = 12;
const STONECUTTER_RECIPE_BUTTON_COLUMNS: i32 = 4;
const STONECUTTER_RECIPE_BUTTON_ROWS: i32 = 3;
const STONECUTTER_RECIPE_BUTTON_X: i32 = 52;
const STONECUTTER_RECIPE_BUTTON_Y: i32 = 15;
const STONECUTTER_RECIPE_BUTTON_WIDTH: u32 = 16;
const STONECUTTER_RECIPE_BUTTON_HEIGHT: u32 = 18;
const STONECUTTER_RECIPE_ITEM_Y_OFFSET: i32 = 1;
const STONECUTTER_SCROLLER_X: i32 = 119;
const STONECUTTER_SCROLLER_Y: i32 = 15;
const STONECUTTER_SCROLLER_WIDTH: u32 = 12;
const STONECUTTER_SCROLLER_HEIGHT: u32 = 15;
const STONECUTTER_SCROLLER_MAX_OFFSET: i32 = 41;
const VILLAGER_NEXT_LEVEL_XP_THRESHOLDS: [i32; 5] = [0, 10, 70, 150, 250];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CartographyAdditionalItem {
    Paper,
    EmptyMap,
    GlassPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CartographyResultMode {
    Map,
    Scaled,
    Duplicated,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BeaconEffectButton {
    primary: bool,
    tier: i16,
    effect_id: i32,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct InventoryHudLocalState {
    stonecutter_recipe_scroll_row: Option<i32>,
    beacon_effect_selection: Option<(Option<i32>, Option<i32>)>,
    loom_pattern_scroll_row: Option<i32>,
    loom_selected_pattern_index: Option<i32>,
    anvil_rename_text: Option<String>,
    cursor_position: Option<(i32, i32)>,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: Vec<i16>,
    shift_down: bool,
    keybind_context: ItemModelKeybindContext,
}

pub(crate) use control_requests::pump_control_net_requests;
pub(crate) use events::LevelEventSoundRandomState;

#[derive(Debug, Default)]
pub(crate) struct ClientAnimationTickState {
    last_entity_animation_at: Option<Instant>,
}

impl ClientAnimationTickState {
    pub(crate) fn entity_partial_tick(&self, now: Instant) -> f32 {
        let Some(last) = self.last_entity_animation_at else {
            return 1.0;
        };
        let elapsed = now.saturating_duration_since(last);
        (elapsed.as_secs_f32() / CLIENT_ENTITY_ANIMATION_TICK_INTERVAL.as_secs_f32())
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug)]
pub(crate) struct LightmapTickState {
    random: LevelEventSoundRandomState,
    block_light_flicker: f32,
    brightness_factor: f32,
    client_tick_count: u64,
    night_vision_effect: LightmapEffectDurationState,
    darkness_effect: LightmapEffectBlendState,
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
            night_vision_effect: LightmapEffectDurationState::default(),
            darkness_effect: LightmapEffectBlendState::default(),
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
            night_vision_effect: LightmapEffectDurationState::default(),
            darkness_effect: LightmapEffectBlendState::default(),
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
            night_vision_effect: LightmapEffectDurationState::default(),
            darkness_effect: LightmapEffectBlendState::default(),
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
        self.night_vision_effect.sync(local_player_effect_snapshot(
            world,
            VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
        ));
        self.darkness_effect.sync(local_player_effect_snapshot(
            world,
            VANILLA_MOB_EFFECT_DARKNESS_ID,
        ));
        for _ in 0..ticks {
            self.tick();
            self.night_vision_effect.tick_duration();
            self.darkness_effect.tick();
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
            self.darkness_effect
                .factor(VANILLA_LIGHTMAP_RENDER_PARTIAL_TICK),
            self.night_vision_effect.effect(),
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

        self.end_flash_state.tick(world_clock_total_ticks(
            world,
            VANILLA_WORLD_CLOCK_THE_END_ID,
        ));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalPlayerMobEffectSnapshot {
    entity_id: i32,
    effect: MobEffectState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightmapEffectSync {
    Unchanged,
    Added(MobEffectState),
    Updated,
    Removed,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LightmapEffectDurationState {
    subject: Option<(i32, i32)>,
    observed_effect: Option<MobEffectState>,
    active_effect: Option<MobEffectState>,
}

impl LightmapEffectDurationState {
    fn sync(&mut self, snapshot: Option<LocalPlayerMobEffectSnapshot>) -> LightmapEffectSync {
        let Some(snapshot) = snapshot else {
            if self.active_effect.is_none() {
                return LightmapEffectSync::Unchanged;
            }
            self.subject = None;
            self.observed_effect = None;
            self.active_effect = None;
            return LightmapEffectSync::Removed;
        };

        let subject = (snapshot.entity_id, snapshot.effect.effect_id);
        let is_new_subject = self.subject != Some(subject);
        if is_new_subject {
            self.subject = Some(subject);
            self.observed_effect = Some(snapshot.effect);
            self.active_effect = Some(snapshot.effect);
            return LightmapEffectSync::Added(snapshot.effect);
        }

        if self.observed_effect != Some(snapshot.effect) {
            self.observed_effect = Some(snapshot.effect);
            self.active_effect = Some(snapshot.effect);
            return LightmapEffectSync::Updated;
        }

        LightmapEffectSync::Unchanged
    }

    fn tick_duration(&mut self) {
        let Some(effect) = &mut self.active_effect else {
            return;
        };
        if effect.duration_ticks != -1 && effect.duration_ticks > 0 {
            effect.duration_ticks -= 1;
        }
    }

    fn effect(&self) -> Option<MobEffectState> {
        self.active_effect
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct LightmapEffectBlendState {
    duration: LightmapEffectDurationState,
    factor: f32,
    factor_previous_frame: f32,
}

impl LightmapEffectBlendState {
    fn sync(&mut self, snapshot: Option<LocalPlayerMobEffectSnapshot>) {
        match self.duration.sync(snapshot) {
            LightmapEffectSync::Added(effect) => {
                let factor = if effect.blend {
                    0.0
                } else {
                    vanilla_darkness_effect_factor(effect)
                };
                self.factor = factor;
                self.factor_previous_frame = factor;
            }
            LightmapEffectSync::Removed => {
                self.factor = 0.0;
                self.factor_previous_frame = 0.0;
            }
            LightmapEffectSync::Updated | LightmapEffectSync::Unchanged => {}
        }
    }

    fn tick(&mut self) {
        self.duration.tick_duration();
        self.factor_previous_frame = self.factor;
        let target = self
            .duration
            .effect()
            .map(vanilla_darkness_effect_factor)
            .unwrap_or(0.0);
        let max_delta = 1.0 / VANILLA_DARKNESS_BLEND_OUT_ADVANCE_TICKS as f32;
        self.factor += (target - self.factor).clamp(-max_delta, max_delta);
    }

    fn factor(&self, partial_tick: f32) -> f32 {
        let partial_tick = sanitize_lightmap_partial_tick(partial_tick);
        self.factor_previous_frame + (self.factor - self.factor_previous_frame) * partial_tick
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

fn local_player_effect_snapshot(
    world: &WorldStore,
    effect_id: i32,
) -> Option<LocalPlayerMobEffectSnapshot> {
    let entity_id = world.local_player_id()?;
    Some(LocalPlayerMobEffectSnapshot {
        entity_id,
        effect: world.entity_effect(entity_id, effect_id)?,
    })
}

fn local_player_effect(world: &WorldStore, effect_id: i32) -> Option<MobEffectState> {
    world
        .local_player_id()
        .and_then(|player_id| world.entity_effect(player_id, effect_id))
}

fn vanilla_night_vision_scale(effect: MobEffectState, partial_tick: f32) -> f32 {
    if !mob_effect_ends_within(effect, VANILLA_NIGHT_VISION_FULL_STRENGTH_TICKS) {
        1.0
    } else {
        0.7 + ((effect.duration_ticks as f32 - partial_tick) * std::f32::consts::PI * 0.2).sin()
            * 0.3
    }
}

fn vanilla_darkness_effect_factor(effect: MobEffectState) -> f32 {
    if mob_effect_ends_within(effect, VANILLA_DARKNESS_BLEND_OUT_ADVANCE_TICKS) {
        0.0
    } else {
        1.0
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

fn sanitize_weather_lightmap_level(level: f32) -> f32 {
    if level.is_finite() {
        level.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn world_clock_total_ticks(world: &WorldStore, clock_id: i32) -> i64 {
    world
        .world_time()
        .and_then(|time| {
            time.clock_updates
                .iter()
                .find(|clock| clock.clock_id == clock_id)
                .map(|clock| clock.total_ticks)
        })
        .unwrap_or(0)
}

fn random_between_inclusive(
    random: &mut LevelEventSoundRandomState,
    min: i32,
    max_inclusive: i32,
) -> i32 {
    random.next_int_bound(max_inclusive - min + 1) + min
}

fn dimension_lightmap_environment(level: &WorldLevelInfo) -> LightmapEnvironment {
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
    let dimension = level.dimension.as_str();
    let dimension_type = level.dimension_type_name.as_deref();
    match (level.dimension_type_id, dimension, dimension_type) {
        (1, _, _) | (_, "minecraft:the_nether", _) | (_, _, Some("minecraft:the_nether")) => {
            VanillaLightmapDimensionKind::Nether
        }
        (2, _, _) | (_, "minecraft:the_end", _) | (_, _, Some("minecraft:the_end")) => {
            VanillaLightmapDimensionKind::End
        }
        (0, _, _)
        | (3, _, _)
        | (_, "minecraft:overworld", _)
        | (_, "minecraft:overworld_caves", _)
        | (_, _, Some("minecraft:overworld"))
        | (_, _, Some("minecraft:overworld_caves")) => VanillaLightmapDimensionKind::Overworld,
        _ => VanillaLightmapDimensionKind::Other,
    }
}

pub(crate) fn snapshot_is_running(snapshot: &SharedSnapshot) -> bool {
    snapshot
        .read()
        .map(|guard| guard.app.running)
        .unwrap_or(false)
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

pub(crate) fn take_control_screenshot(requests: &SharedControlRequests) -> Option<PathBuf> {
    requests
        .lock()
        .ok()?
        .screenshot_request
        .take()
        .map(PathBuf::from)
}

pub(crate) fn pump_network_and_terrain(
    net_events: &mut Option<mpsc::Receiver<NetEvent>>,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    audio_status: &AudioCounters,
    particle_events: Option<&mut dyn ParticleEventSink>,
    input: &mut ClientInputState,
    world: &mut WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    net_counters: &mut NetCounters,
    client_animation_ticks: &mut ClientAnimationTickState,
    lightmap_ticks: &mut LightmapTickState,
    level_event_sound_random: &mut LevelEventSoundRandomState,
    terrain_upload: &mut TerrainUploadState,
    terrain_textures: &TerrainTextureState,
    item_runtime: Option<&NativeItemRuntime>,
    snapshot: &SharedSnapshot,
    control_requests: &SharedControlRequests,
    code_of_conduct: Option<&mut CodeOfConductAcceptance>,
    render_distance_chunks: u32,
    hide_lightning_flash: bool,
) -> bool {
    let mut audio_events = audio_events;
    let mut particle_events = particle_events;
    let input_screen_was_open = input_screen_is_open(input, world);
    if let Some(rx) = net_events.as_mut() {
        let audio_events_for_drain = audio_events
            .as_mut()
            .map(|audio_events| &mut **audio_events as &mut dyn AudioEventSink);
        let particle_events_for_drain = particle_events
            .as_mut()
            .map(|particle_events| &mut **particle_events as &mut dyn ParticleEventSink);
        events::drain_net_events_with_sinks(
            rx,
            world,
            net_counters,
            net_commands,
            audio_events_for_drain,
            particle_events_for_drain,
            Some(renderer),
            item_runtime,
            level_event_sound_random,
        );
    }
    release_input_if_screen_opened(
        input_screen_was_open,
        input,
        world,
        net_counters,
        net_commands,
    );
    pump_control_net_requests(
        control_requests,
        net_commands,
        net_counters,
        world,
        code_of_conduct,
    );
    let now = Instant::now();
    let advanced_ticks = advance_entity_client_animations(world, client_animation_ticks, now);
    let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);
    let running_ticks = world.consume_running_render_ticks(advanced_ticks);
    // Vanilla `ClientLevel.tick` runs `this.getWorldBorder().tick()` right
    // before `this.tickTime()` when the tick rate manager runs normally
    // (ClientLevel.java:276-281); the border lerp therefore advances on the
    // same running ticks as the client clock, before render extraction reads
    // the interpolated border bounds.
    world.advance_world_border(running_ticks);
    world.advance_client_time(running_ticks);
    // Vanilla `Minecraft.tick` calls `Gui.tick` once per client tick (Gui.java:
    // 1145-1166), outside the tick-rate manager's freeze gate, so the action
    // bar / title countdowns advance on raw client ticks, not running ticks.
    world.advance_hud_text_ticks(advanced_ticks);
    lightmap_ticks.advance_for_world(advanced_ticks, world);
    lightmap_ticks.advance_rain_fog_for_world(advanced_ticks, world, terrain_textures);
    let water_vision = lightmap_ticks.water_vision(world);
    let rain_fog_multiplier = lightmap_ticks.rain_fog_multiplier();
    // Vanilla `Minecraft.tick` runs `ClientLevel.tick` before `GameRenderer.extract`;
    // `ClientLevel.tick` decrements `skyFlashTime`, and the render extract reads
    // the resulting EnvironmentAttributes / lightmap state.
    world.advance_sky_flash_time(advanced_ticks);
    let lightmap_environment = lightmap_ticks.environment_for_world(world);
    let clear_color = clear_color_for_world_at_camera_with_water_vision(
        world,
        terrain_textures,
        camera_pose_from_world(world),
        render_distance_chunks,
        water_vision,
        hide_lightning_flash,
    );
    let fog_environment = fog_environment_for_world_at_camera(
        world,
        terrain_textures,
        camera_pose_from_world(world),
        render_distance_chunks,
        water_vision,
        rain_fog_multiplier,
        hide_lightning_flash,
    );
    let sky_environment = sky_environment_for_world_at_camera(
        world,
        terrain_textures,
        camera_pose_from_world(world),
        hide_lightning_flash,
    );
    let cloud_environment = cloud_environment_for_world(world);
    advance_block_destruction_render_ticks(world, running_ticks);
    world.advance_item_cooldowns(advanced_ticks);
    advance_player_input(input, world, net_counters, net_commands, now);
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
    advance_using_item_at_partial_tick(
        input,
        world,
        net_counters,
        net_commands,
        entity_partial_tick,
        advanced_ticks,
    );
    world.advance_local_using_item_ticks(advanced_ticks);
    let particle_camera_pose = camera_pose_from_world(world);
    let particle_scope_context =
        particle_local_player_scope_context(world, item_runtime, particle_camera_pose);
    let particle_sound_camera_position =
        particle_camera_pose.map(|camera| camera_eye_position(camera).map(f64::from));
    let particle_player_motion_contexts = particle_player_motion_contexts(world);
    let particle_entity_target_contexts = particle_entity_target_contexts(world);
    submit_primed_tnt_smoke_particles(renderer, world, advanced_ticks);
    submit_entity_client_tick_particles(renderer, world, &mut particle_events);
    submit_ominous_item_spawner_particles(renderer, world, &mut particle_events);
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
        &particle_player_motion_contexts,
        &particle_entity_target_contexts,
        particle_sound_camera_position,
    );
    let particle_sound_events = renderer.drain_particle_sound_events();
    emit_particle_sound_events(&mut audio_events, particle_sound_events);
    // Vanilla handles gameplay keybinds during `Minecraft.tick`, then `GameRenderer.extractGui`
    // calls `Gui.extractRenderState`; HUD values therefore read after input and use-item updates.
    let local_player = world.local_player();
    let hud_health = local_player.health.map(|health| health.health);
    let hud_food = local_player.health.map(|health| health.food);
    let hud_experience_progress = local_player
        .experience
        .map(|experience| experience.progress);
    let hud_selected_slot = local_player.selected_hotbar_slot;
    // Vanilla `Gui.extractOverlayMessage` / `extractTitle` read the post-tick
    // countdowns with the frame partial tick; the renderer resolves fade
    // alpha per frame from these projected timers.
    let hud_action_bar_text = hud_action_bar_text_from_world(world, entity_partial_tick);
    let hud_title_text = hud_title_text_from_world(world, entity_partial_tick);
    // Vanilla `Gui.extractBossOverlay` re-walks the tracked boss events every
    // frame with no countdown state, so the projection is pure world state.
    let hud_boss_bars = hud_boss_bars_from_world(world);
    let item_model_keybind_context = input.item_model_keybind_context();
    let hud_hotbar_item_icons = hotbar_item_icons_with_input_context(
        world,
        item_runtime,
        entity_partial_tick,
        input.shift_down(),
        item_model_keybind_context,
    );
    let hud_hotbar_block_item_models =
        hotbar_block_item_models(world, item_runtime, terrain_textures);
    sync_stonecutter_recipe_scroll_state(input, world);
    sync_beacon_effect_selection_state(input, world);
    sync_loom_pattern_state_for_hud(input, world);
    let hud_inventory_screen = hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        terrain_textures,
        input.inventory_hovered_slot(),
        InventoryHudLocalState {
            stonecutter_recipe_scroll_row: Some(input.stonecutter_recipe_scroll_row()),
            beacon_effect_selection: Some(input.beacon_effect_selection()),
            loom_pattern_scroll_row: Some(input.loom_pattern_scroll_row()),
            loom_selected_pattern_index: input.loom_selected_pattern_index(),
            anvil_rename_text: Some(input.anvil_rename_text().to_string()),
            cursor_position: input.inventory_cursor_position(),
            quick_craft_button_num: input.inventory_quick_craft_button_num(),
            quick_craft_slots: input.inventory_quick_craft_slots().to_vec(),
            shift_down: input.shift_down(),
            keybind_context: item_model_keybind_context,
        },
        entity_partial_tick,
    );
    // Vanilla extracts item/entity render state after `Minecraft.tick` has advanced keybinds,
    // `gameRenderer.tick`, and `level.tickEntities`; these projections read the post-tick snapshot.
    // Dropped block-items render as 3D block-item models (replacing their billboard); the animation
    // clock is the world game time plus the partial tick.
    let item_model_age_ticks = world
        .world_time()
        .map(|time| time.game_time as f32)
        .unwrap_or(0.0)
        + entity_partial_tick;
    let trim_material_keys = world_trim_material_keys(world);
    let enchantment_keys = world_enchantment_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let dropped_item_models = dropped_item_models(
        world,
        item_runtime,
        terrain_textures,
        item_model_age_ticks,
        trim_material_keys.as_deref(),
        enchantment_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let ominous_item_spawner_models = ominous_item_spawner_models(
        world,
        item_runtime,
        terrain_textures,
        entity_partial_tick,
        trim_material_keys.as_deref(),
        enchantment_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let item_pickup_particle_states = renderer.item_pickup_particle_render_states();
    let item_pickup_particle_models = item_pickup_particle_item_models(
        &item_pickup_particle_states,
        item_runtime,
        terrain_textures,
        trim_material_keys.as_deref(),
        enchantment_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let item_entity_billboards = item_entity_billboards_from_world(
        world,
        item_runtime,
        &dropped_item_models.handled_entity_ids,
    );
    // Held items render as 3D models at each player's hand, on top of the dropped-item models (sharing
    // the two atlas draws).
    let entity_instances =
        entity_model_instances_from_world_at_partial_tick(world, item_runtime, entity_partial_tick);
    let held_item_models =
        held_item_models(&entity_instances, world, item_runtime, terrain_textures);
    let camera_pose = camera_pose_from_world(world);
    let first_person_item_models = first_person_item_models(
        world,
        item_runtime,
        terrain_textures,
        camera_pose,
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
        item_runtime,
        local_player_model_instance,
        camera_pose,
        entity_partial_tick,
    );
    // Item frames render their wooden border + framed item into the same two atlas draws.
    let item_frame_models = item_frame_models(
        world,
        item_runtime,
        terrain_textures,
        trim_material_keys.as_deref(),
        enchantment_keys.as_deref(),
        attribute_keys.as_deref(),
    );
    let entity_block_meshes = entity_block_models(
        &entity_instances,
        world,
        item_runtime,
        terrain_textures,
        entity_partial_tick,
    );
    let mut block_item_meshes = dropped_item_models.block_meshes;
    block_item_meshes.extend(ominous_item_spawner_models.block_meshes);
    block_item_meshes.extend(held_item_models.block_meshes);
    block_item_meshes.extend(item_frame_models.block_meshes);
    block_item_meshes.extend(entity_block_meshes);
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
    let selection_outline = selection_outline_from_camera(world, camera_pose);
    let entity_scene_outline =
        entity_scene_outline_from_world_at_partial_tick(world, entity_partial_tick);
    let entity_target_outline =
        entity_target_outline_from_camera_at_partial_tick(world, camera_pose, entity_partial_tick);
    // Vanilla `LevelRenderer.extractBlockDestroyAnimation` reads block-breaking state during
    // render extract, after the client tick; local destroy overlay ticks are advanced above.
    let block_destroy_overlays = block_destroy_overlays_from_world(world, terrain_textures);
    // Vanilla `ParticleEngine.extract` calls `SingleQuadParticle.getLightCoords(partialTicks)`
    // during level render extraction; sample world light for the current particle positions before
    // the renderer later collects particle vertices.
    renderer.refresh_particle_lights(|position| particle_light_for_world(world, position));
    apply_renderer_frame(
        renderer,
        RendererFrame {
            lightmap_environment,
            clear_color,
            fog_environment,
            sky_environment,
            cloud_environment,
            hud_health,
            hud_food,
            hud_experience_progress,
            hud_selected_slot,
            hud_hotbar_item_icons,
            hud_hotbar_block_item_models,
            hud_inventory_screen,
            hud_action_bar_text,
            hud_title_text,
            hud_boss_bars,
            item_entity_billboards,
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
            item_frame_map_textures,
            item_frame_map_surfaces,
            item_frame_map_decoration_textures,
            item_frame_map_decoration_surfaces,
            item_frame_map_text_surfaces,
            entity_model_instances: entity_instances,
            camera_pose,
            cloud_frame,
            weather_render_state,
            world_border_render_state,
            selection_outline,
            entity_scene_outline,
            entity_target_outline,
            block_destroy_overlays,
        },
    );
    maybe_upload_terrain_texture_animation(renderer, terrain_upload, terrain_textures);
    if let Some(particle_events) = particle_events.as_mut() {
        particle_events.maybe_upload_particle_atlas_animation(renderer);
    }
    maybe_upload_decoded_terrain(world, renderer, terrain_upload, terrain_textures);
    if let Some(audio_events) = audio_events.as_mut() {
        audio_events.tick_entity_sound_positions(audio_scene_command_from_world(world));
    }
    let audio_counters = audio_events
        .as_deref()
        .map(AudioEventSink::counters)
        .unwrap_or_else(|| audio_status.clone());
    publish_snapshot(
        snapshot,
        control_renderer_counters(renderer.counters()),
        net_counters,
        &audio_counters,
    )
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

fn submit_primed_tnt_smoke_particles(renderer: &mut Renderer, world: &WorldStore, ticks: u32) {
    if ticks == 0 {
        return;
    }
    let batch = primed_tnt_smoke_particle_batch(world.primed_tnt_smoke_particle_states(), ticks);
    renderer.submit_particle_spawns(batch);
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

fn submit_entity_client_tick_particles(
    renderer: &mut Renderer,
    world: &mut WorldStore,
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
) {
    let firework_rocket_trail_particles: Vec<FireworkRocketTrailParticleState> =
        world.take_firework_rocket_trail_particle_states();
    let batch = entity_client_tick_particle_batch(
        world.take_ravager_stun_particle_states(),
        world.take_evoker_fangs_crit_particle_states(),
    );
    renderer.submit_particle_spawns(batch);

    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    for state in firework_rocket_trail_particles {
        renderer
            .submit_particle_spawns(particle_events.spawn_firework_rocket_trail_particles(state));
    }
}

fn submit_ominous_item_spawner_particles(
    renderer: &mut Renderer,
    world: &mut WorldStore,
    particle_events: &mut Option<&mut dyn ParticleEventSink>,
) {
    let ominous_item_spawner_particles: Vec<OminousItemSpawnerParticleState> =
        world.take_ominous_item_spawner_particle_states();
    let Some(particle_events) = particle_events.as_deref_mut() else {
        return;
    };
    for state in ominous_item_spawner_particles {
        renderer
            .submit_particle_spawns(particle_events.spawn_ominous_item_spawner_particles(state));
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
            ENTITY_EFFECT_PARTICLE_TYPE_ID,
            "minecraft:entity_effect",
            [state.position.x, state.position.y, state.position.z],
            [0.0, 0.0, 0.0],
        );
        command.option_color = Some([0.49803922, 0.5137255, 0.57254905, 1.0]);
        command
    }));
    commands.extend(evoker_fangs_crit_particles.into_iter().map(|state| {
        direct_particle_spawn_command(
            CRIT_PARTICLE_TYPE_ID,
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
        SMOKE_PARTICLE_TYPE_ID,
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

fn block_destroy_overlays_from_world(
    world: &WorldStore,
    textures: &TerrainTextureState,
) -> Vec<BlockDestroyOverlay> {
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
        .into_iter()
        .filter_map(|(pos, stage)| {
            Some(BlockDestroyOverlay {
                pos: [pos.x, pos.y, pos.z],
                uv: textures.destroy_stage_uv_rect(stage)?,
            })
        })
        .collect()
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
fn block_item_3d_model(
    item: &bbb_protocol::packets::ItemStackSummary,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> Option<HudBlockItemModel> {
    let item_runtime = item_runtime?;
    let item_id = item.item_id?;
    let resource_id = item_runtime.item_resource_id(item_id)?;
    let quads = terrain_textures.block_item_quads(resource_id, &BTreeMap::new())?;
    if quads.is_empty() {
        return None;
    }
    let gui = item_runtime
        .item_display_transform_for_stack(item, bbb_pack::BlockModelDisplayContext::Gui)
        .unwrap_or_default();
    Some(HudBlockItemModel {
        quads,
        gui_display: crate::item_models::display_matrix(&gui, false),
        lighting: GuiItemLightingEntry::Items3d,
        foil: item.has_foil(),
    })
}

/// The hotbar's 3D block items (vanilla inventory item rendering): for each slot holding a block item,
/// its [`block_item_3d_model`], so the renderer draws it as a 3D icon instead of the flat 2D sprite.
/// `None` for empty slots and flat items (which keep the 2D sprite drawn by the HUD layer).
fn hotbar_block_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> Vec<Option<HudBlockItemModel>> {
    let mut models: Vec<Option<HudBlockItemModel>> = (0..HUD_HOTBAR_SLOTS).map(|_| None).collect();
    for (slot, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        if slot >= HUD_HOTBAR_SLOTS {
            break;
        }
        models[slot] = block_item_3d_model(&item.item, item_runtime, terrain_textures);
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
        release_active_input(input, world, counters, net_commands);
    }
}

fn input_screen_is_open(input: &ClientInputState, world: &WorldStore) -> bool {
    world.open_container_id().is_some()
        || world.current_dialog().is_some()
        || world.current_book().is_some()
        || input.sign_editor_is_active_or_pending(world)
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

fn hud_inventory_screen_with_local_state(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    if let Some(book) = world.current_book() {
        return Some(hud_book_screen(book));
    }

    let layout = inventory_screen_layout(world)?;
    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };

    let selected_hotbar_slot_id = inventory_screen_selected_hotbar_slot_id(world);
    let slots = layout
        .slots
        .iter()
        .copied()
        .map(|layout| {
            let inventory_slot = container
                .slots
                .iter()
                .find(|slot| slot.slot == layout.slot_id);
            HudInventorySlot {
                slot_id: u16::try_from(layout.slot_id).unwrap_or_default(),
                x: layout.x,
                y: layout.y,
                icon: inventory_slot.and_then(|slot| {
                    hud_item_icon_for_stack(
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
                    )
                }),
                block_model: inventory_slot.and_then(|slot| {
                    block_item_3d_model(&slot.item, item_runtime, terrain_textures)
                }),
            }
        })
        .collect();

    Some(HudInventoryScreen {
        width: u32::try_from(layout.width).unwrap_or_default(),
        height: u32::try_from(layout.height).unwrap_or_default(),
        background_layers: hud_inventory_background_layers(
            world,
            item_runtime,
            layout.background,
            &local_state,
        ),
        slots,
        floating_items: hud_inventory_floating_items(
            world,
            item_runtime,
            terrain_textures,
            layout.background,
            local_state.stonecutter_recipe_scroll_row,
            local_state.cursor_position,
            local_state.quick_craft_button_num,
            &local_state.quick_craft_slots,
            local_state.shift_down,
            local_state.keybind_context,
            partial_tick,
        ),
        entity_previews: hud_inventory_entity_previews(
            world,
            item_runtime,
            layout.background,
            &local_state,
            partial_tick,
        ),
        text_labels: hud_inventory_text_labels(world, layout.background, &local_state),
        hovered_slot_id: hovered_slot_id.and_then(|slot| u16::try_from(slot).ok()),
        tooltip: hud_inventory_tooltip(item_runtime, hovered_slot_id, &layout.slots, container),
    })
}

fn hud_inventory_entity_previews(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
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
        local_state,
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
        local_state,
        partial_tick,
        [X0, Y0, X1, Y1],
        SCALE,
        OFFSET_Y,
    )
}

fn hud_entity_in_inventory_follows_mouse_preview(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    local_state: &InventoryHudLocalState,
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
    let (mouse_x, mouse_y) = local_state
        .cursor_position
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

    match item_runtime.item_equipment_slot(item_id) {
        Some(ItemEquipmentSlot::Head) if item_runtime.item_has_humanoid_armor_asset(item_id) => {
            entity.render_state.head_armor =
                armor_material(item_runtime.item_armor_material(item_id));
            entity.render_state.head_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.head_armor_foil =
                entity.render_state.head_armor.is_some() && stack.has_foil();
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
            entity.render_state.chest_armor =
                armor_material(item_runtime.item_armor_material(item_id));
            entity.render_state.chest_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.chest_armor_foil =
                entity.render_state.chest_armor.is_some() && stack.has_foil();
            entity.render_state.chest_wings_layer =
                item_runtime.item_equipment_wings_layer(item_id);
            entity.render_state.chest_equipment_has_wings =
                item_runtime.item_equipment_asset_has_wings_layer(item_id);
            entity.render_state.chest_equipment_has_humanoid =
                item_runtime.item_equipment_asset_has_humanoid_layer(item_id);
        }
        Some(ItemEquipmentSlot::Legs) => {
            entity.render_state.legs_armor =
                armor_material(item_runtime.item_armor_material(item_id));
            entity.render_state.legs_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.legs_armor_foil =
                entity.render_state.legs_armor.is_some() && stack.has_foil();
        }
        Some(ItemEquipmentSlot::Feet) => {
            entity.render_state.feet_armor =
                armor_material(item_runtime.item_armor_material(item_id));
            entity.render_state.feet_armor_dye =
                stack.component_patch.dyed_color.map(|dye| dye as u32);
            entity.render_state.feet_armor_foil =
                entity.render_state.feet_armor.is_some() && stack.has_foil();
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

fn hud_book_screen(book: &BookScreenState) -> HudInventoryScreen {
    HudInventoryScreen {
        width: 192,
        height: 192,
        background_layers: book_screen_background_layers(book),
        slots: Vec::new(),
        floating_items: Vec::new(),
        entity_previews: Vec::new(),
        text_labels: book_screen_text_labels(book),
        hovered_slot_id: None,
        tooltip: None,
    }
}

fn book_screen_background_layers(book: &BookScreenState) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = vec![hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Book,
        0,
        0,
        192,
        192,
        [0.0, 0.0],
        [192.0 / 256.0, 192.0 / 256.0],
    )];
    if book.current_page > 0 {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::PageBackward,
            43,
            157,
            23,
            13,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    if book.current_page + 1 < book.pages.len() {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::PageForward,
            116,
            157,
            23,
            13,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
    layers
}

fn book_screen_text_labels(book: &BookScreenState) -> Vec<HudInventoryTextLabel> {
    book_page_text_labels(&book.pages, book.current_page)
}

fn hud_inventory_text_labels(
    world: &WorldStore,
    background: InventoryScreenBackground,
    local_state: &InventoryHudLocalState,
) -> Vec<HudInventoryTextLabel> {
    match background {
        InventoryScreenBackground::Anvil => {
            let mut labels = Vec::new();
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
                    shadow: false,
                    runs: Vec::new(),
                });
            }
            if let Some(label) = anvil_cost_text_label(world) {
                labels.push(label);
            }
            labels
        }
        InventoryScreenBackground::EnchantmentTable => enchanting_table_cost_text_labels(world),
        InventoryScreenBackground::Lectern => lectern_book_text_labels(world),
        _ => Vec::new(),
    }
}

fn lectern_book_text_labels(world: &WorldStore) -> Vec<HudInventoryTextLabel> {
    let pages = lectern_book_pages(world);
    let current_page = lectern_current_page(world, pages.len());
    book_page_text_labels(&pages, current_page)
}

fn book_page_text_labels(pages: &[String], current_page: usize) -> Vec<HudInventoryTextLabel> {
    let page_count = pages.len();
    let current_page = current_page.min(page_count.saturating_sub(1));
    let mut labels = Vec::new();

    let page_indicator = format!("Page {} of {}", current_page + 1, page_count.max(1));
    if let Some(width) = hud_ascii_approx_text_width(&page_indicator) {
        if let Ok(width_i32) = i32::try_from(width) {
            labels.push(HudInventoryTextLabel {
                x: BOOK_PAGE_INDICATOR_X_RIGHT - width_i32,
                y: BOOK_PAGE_INDICATOR_Y,
                width,
                text: page_indicator,
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
                runs: Vec::new(),
            });
        }
    }

    if let Some(page) = pages.get(current_page) {
        for (line_index, line) in lectern_book_page_lines(page).into_iter().enumerate() {
            let Some(text) = line else {
                continue;
            };
            labels.push(HudInventoryTextLabel {
                x: BOOK_PAGE_TEXT_X,
                y: BOOK_PAGE_TEXT_Y + line_index as i32 * BOOK_PAGE_LINE_HEIGHT,
                width: BOOK_PAGE_TEXT_WIDTH,
                text,
                tint: BOOK_TEXT_COLOR,
                background: None,
                shadow: false,
                runs: Vec::new(),
            });
        }
    }

    labels
}

fn lectern_book_pages(world: &WorldStore) -> Vec<String> {
    let Some(book) = open_container_slot_item(world, 0) else {
        return Vec::new();
    };
    if let Some(written) = &book.component_patch.written_book {
        return written.pages.clone();
    }
    book.component_patch.writable_book_pages.clone()
}

fn lectern_current_page(world: &WorldStore, page_count: usize) -> usize {
    if page_count == 0 {
        return 0;
    }
    let page = world
        .open_container_data_value(0)
        .map(i32::from)
        .unwrap_or_default();
    usize::try_from(page.clamp(0, page_count.saturating_sub(1) as i32)).unwrap_or_default()
}

fn lectern_book_page_lines(page: &str) -> Vec<Option<String>> {
    let max_lines =
        usize::try_from(BOOK_PAGE_TEXT_HEIGHT / BOOK_PAGE_LINE_HEIGHT as u32).unwrap_or_default();
    let mut lines = Vec::new();
    for paragraph in page.split('\n') {
        if lines.len() >= max_lines {
            break;
        }
        if paragraph.is_empty() {
            lines.push(None);
            continue;
        }
        let mut current = String::new();
        let mut width = 0u32;
        for ch in paragraph.chars().filter(|ch| !ch.is_control()) {
            let advance = hud_ascii_approx_char_width(ch);
            if !current.is_empty() && width.saturating_add(advance) > BOOK_PAGE_TEXT_WIDTH {
                lines.push(Some(current.trim_end().to_string()));
                if lines.len() >= max_lines {
                    current.clear();
                    break;
                }
                current.clear();
                width = 0;
            }
            current.push(ch);
            width = width.saturating_add(advance);
        }
        if lines.len() < max_lines && !current.is_empty() {
            lines.push(Some(current.trim_end().to_string()));
        }
    }
    lines.truncate(max_lines);
    lines
}

fn enchanting_table_cost_text_labels(world: &WorldStore) -> Vec<HudInventoryTextLabel> {
    let mut labels = Vec::new();
    for index in 0..ENCHANTING_TABLE_OPTION_COUNT {
        let cost = world.open_container_data_value(index).unwrap_or_default();
        if cost <= 0 {
            continue;
        }
        let text = cost.to_string();
        let Some(text_width) = hud_ascii_approx_text_width(&text) else {
            continue;
        };
        let Some(text_width_i32) = i32::try_from(text_width).ok() else {
            continue;
        };
        labels.push(HudInventoryTextLabel {
            x: ENCHANTING_TABLE_COST_TEXT_X_RIGHT - text_width_i32,
            y: ENCHANTING_TABLE_COST_TEXT_Y_OFFSET
                + i32::from(index) * ENCHANTING_TABLE_OPTION_SPACING,
            width: text_width,
            text,
            tint: if enchanting_table_option_is_enabled(world, index, cost) {
                ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR
            } else {
                ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR
            },
            background: None,
            shadow: false,
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

    let creative = world.gameplay().game_type == 1;
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
        shadow: false,
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
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    layout_slots: &[crate::input::InventorySlotLayout],
    container: &ContainerState,
) -> Option<HudInventoryTooltip> {
    let slot_id = hovered_slot_id?;
    let layout = layout_slots
        .iter()
        .find(|layout| layout.slot_id == slot_id)?;
    let slot = container.slots.iter().find(|slot| slot.slot == slot_id)?;
    let lines = item_runtime?.tooltip_lines_for_stack(&slot.item)?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(slot_id).ok()?,
        x: layout.x,
        y: layout.y,
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
        _ => Vec::new(),
    };
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
    let block_model = block_item_3d_model(&item, item_runtime, terrain_textures);
    items.push(HudInventoryItem {
        x: cursor_x - 8,
        y: cursor_y - 8,
        icon,
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
            push_enchanting_table_state_layers(world, &mut layers);
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
            push_beacon_effect_button_layers(world, &mut layers, selection);
            push_beacon_action_button_layers(world, &mut layers, selection);
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
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_cartography_table_result_layers(world, item_runtime, &mut layers);
            layers
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
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Book,
                    0,
                    0,
                    192,
                    192,
                    [0.0, 0.0],
                    [192.0 / 256.0, 192.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::PageBackward,
                    43,
                    157,
                    23,
                    13,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::PageForward,
                    116,
                    157,
                    23,
                    13,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        }
        InventoryScreenBackground::Loom => {
            let mut layers = vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Loom,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )];
            push_loom_state_layers(
                world,
                &mut layers,
                local_state.loom_pattern_scroll_row.unwrap_or_default(),
                local_state.loom_selected_pattern_index,
            );
            layers
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
            push_merchant_trade_layers(world, &mut layers);
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
            merchant_offer_cost_a_stack(world, offer),
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
) {
    if let Some(icon) = hud_item_icon_for_stack(
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
        let block_model = block_item_3d_model(&item, item_runtime, terrain_textures);
        items.push(HudInventoryItem {
            x,
            y,
            icon,
            block_model,
        });
    }
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
            let block_model = block_item_3d_model(&option.stack, item_runtime, terrain_textures);
            items.push(HudInventoryItem {
                x: option.x,
                y: option.y,
                icon,
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
            let stack = stonecutter_slot_display_item_stack(&recipe.option_display)?;
            let column = position as i32 % STONECUTTER_RECIPE_BUTTON_COLUMNS;
            let row = position as i32 / STONECUTTER_RECIPE_BUTTON_COLUMNS;
            Some(StonecutterRecipeOptionStack {
                x: STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH as i32,
                y: STONECUTTER_RECIPE_BUTTON_Y
                    + STONECUTTER_RECIPE_ITEM_Y_OFFSET
                    + row * STONECUTTER_RECIPE_BUTTON_HEIGHT as i32,
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
        STONECUTTER_SCROLLER_WIDTH,
        STONECUTTER_SCROLLER_HEIGHT,
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
            STONECUTTER_RECIPE_BUTTON_X + column * STONECUTTER_RECIPE_BUTTON_WIDTH as i32,
            STONECUTTER_RECIPE_BUTTON_Y + row * STONECUTTER_RECIPE_BUTTON_HEIGHT as i32,
            STONECUTTER_RECIPE_BUTTON_WIDTH,
            STONECUTTER_RECIPE_BUTTON_HEIGHT,
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

fn stonecutter_slot_display_item_stack(display: &SlotDisplaySummary) -> Option<ItemStackSummary> {
    let mut decoder = Decoder::new(&display.raw_payload);
    let display_type_id = decoder.read_var_i32().ok()?;
    if display_type_id != display.display_type_id {
        return None;
    }
    match display_type_id {
        4 => {
            let item_id = decoder.read_var_i32().ok()?;
            (item_id >= 0).then_some(ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: Default::default(),
            })
        }
        5 => {
            let item_id = decoder.read_var_i32().ok()?;
            let count = decoder.read_var_i32().ok()?;
            if item_id < 0 || count <= 0 {
                return None;
            }
            Some(ItemStackSummary {
                item_id: Some(item_id),
                count,
                component_patch: Default::default(),
            })
        }
        _ => None,
    }
}

fn push_merchant_trade_layers(world: &WorldStore, layers: &mut Vec<HudInventoryBackgroundLayer>) {
    let Some(offers) = merchant_offers_state(world) else {
        return;
    };
    if offers.offers.is_empty() {
        return;
    }

    for (row, offer) in offers
        .offers
        .iter()
        .skip(merchant_scroll_offset(offers))
        .take(MERCHANT_VISIBLE_OFFER_COUNT)
        .enumerate()
    {
        layers.push(hud_inventory_background_layer(
            if offer.is_out_of_stock {
                HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock
            } else {
                HudInventoryBackgroundTexture::VillagerTradeArrow
            },
            MERCHANT_TRADE_ARROW_X,
            MERCHANT_TRADE_ARROW_Y + row as i32 * MERCHANT_TRADE_ROW_HEIGHT,
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
        }
    }
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
    let max_stack_size = world.item_max_stack_size_for_protocol_id(offer.buy_a.item_id);
    let demand_diff = (offer.buy_a.count as f32 * offer.demand as f32 * offer.price_multiplier)
        .floor()
        .max(0.0) as i32;
    let count =
        (offer.buy_a.count + demand_diff + offer.special_price_diff).clamp(1, max_stack_size);
    item_cost_stack(&offer.buy_a, count)
}

fn item_cost_stack(cost: &ItemCostSummary, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(cost.item_id),
        count,
        component_patch: Default::default(),
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

fn anvil_input_slot_has_item(world: &WorldStore) -> bool {
    open_container_slot_has_item(world, 0)
}

fn push_beacon_action_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
) {
    let confirm_texture = if beacon_confirm_button_is_active(world, selection) {
        HudInventoryBackgroundTexture::BeaconButton
    } else {
        HudInventoryBackgroundTexture::BeaconButtonDisabled
    };
    push_beacon_action_button_layer(
        layers,
        confirm_texture,
        HudInventoryBackgroundTexture::BeaconConfirm,
        BEACON_CONFIRM_BUTTON_X,
    );
    push_beacon_action_button_layer(
        layers,
        HudInventoryBackgroundTexture::BeaconButton,
        HudInventoryBackgroundTexture::BeaconCancel,
        BEACON_CANCEL_BUTTON_X,
    );
}

fn push_beacon_effect_button_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    selection: (Option<i32>, Option<i32>),
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
    for (tier, effects) in BEACON_PRIMARY_EFFECT_ROWS.iter().enumerate() {
        let total_width = effects.len() as i32 * BEACON_EFFECT_BUTTON_SIZE as i32
            + (effects.len() as i32 - 1) * 2;
        for (column, effect_id) in effects.iter().enumerate() {
            buttons.push(BeaconEffectButton {
                primary: true,
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

    let count = BEACON_SECONDARY_EFFECTS.len() as i32 + 1;
    let total_width = count * BEACON_EFFECT_BUTTON_SIZE as i32 + (count - 1) * 2;
    for (column, effect_id) in BEACON_SECONDARY_EFFECTS.iter().enumerate() {
        buttons.push(BeaconEffectButton {
            primary: false,
            tier: 3,
            effect_id: *effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(column).unwrap_or_default() * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    if let Some(effect_id) = primary_effect {
        buttons.push(BeaconEffectButton {
            primary: false,
            tier: 3,
            effect_id,
            x: BEACON_SECONDARY_EFFECT_CENTER_X
                + i32::try_from(BEACON_SECONDARY_EFFECTS.len()).unwrap_or_default()
                    * BEACON_EFFECT_BUTTON_SPACING
                - total_width / 2,
            y: BEACON_SECONDARY_EFFECT_Y,
        });
    }
    buttons
}

fn beacon_effect_icon_texture(effect_id: i32) -> Option<HudInventoryBackgroundTexture> {
    match effect_id {
        BEACON_EFFECT_SPEED_ID => Some(HudInventoryBackgroundTexture::BeaconEffectSpeed),
        BEACON_EFFECT_HASTE_ID => Some(HudInventoryBackgroundTexture::BeaconEffectHaste),
        BEACON_EFFECT_RESISTANCE_ID => Some(HudInventoryBackgroundTexture::BeaconEffectResistance),
        BEACON_EFFECT_JUMP_BOOST_ID => Some(HudInventoryBackgroundTexture::BeaconEffectJumpBoost),
        BEACON_EFFECT_STRENGTH_ID => Some(HudInventoryBackgroundTexture::BeaconEffectStrength),
        BEACON_EFFECT_REGENERATION_ID => {
            Some(HudInventoryBackgroundTexture::BeaconEffectRegeneration)
        }
        _ => None,
    }
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
    open_container_slot_has_item(world, 0) && selection.0.is_some()
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

fn push_loom_state_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    scroll_row: i32,
    selected_pattern_index: Option<i32>,
) {
    for (slot, texture, x, y) in [
        (0, HudInventoryBackgroundTexture::LoomBannerSlot, 13, 26),
        (1, HudInventoryBackgroundTexture::LoomDyeSlot, 33, 26),
        (2, HudInventoryBackgroundTexture::LoomPatternSlot, 23, 45),
    ] {
        if open_container_slot_has_item(world, slot) {
            continue;
        }
        layers.push(hud_inventory_background_layer(
            texture,
            x,
            y,
            LOOM_SLOT_SPRITE_SIZE,
            LOOM_SLOT_SPRITE_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let selectable_count = loom_selectable_pattern_count(world);
    let can_scroll = selectable_count
        .is_some_and(|count| count > LOOM_PATTERN_BUTTON_COLUMNS * LOOM_PATTERN_BUTTON_ROWS);
    layers.push(hud_inventory_background_layer(
        if can_scroll {
            HudInventoryBackgroundTexture::LoomScroller
        } else {
            HudInventoryBackgroundTexture::LoomScrollerDisabled
        },
        LOOM_SCROLLER_X,
        LOOM_SCROLLER_Y
            + if let Some(count) = selectable_count {
                loom_scroller_offset(count, scroll_row)
            } else {
                0
            },
        LOOM_SCROLLER_WIDTH,
        LOOM_SCROLLER_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    let Some(selectable_count) = selectable_count else {
        return;
    };
    let selected_pattern_index =
        selected_pattern_index.or_else(|| loom_selected_pattern_index(world));
    let start_index = loom_pattern_start_index(selectable_count, scroll_row).unwrap_or_default();
    for position in 0..(LOOM_PATTERN_BUTTON_COLUMNS * LOOM_PATTERN_BUTTON_ROWS) {
        let pattern_index = start_index + position;
        if pattern_index >= selectable_count {
            break;
        }
        let column = position % LOOM_PATTERN_BUTTON_COLUMNS;
        let row = position / LOOM_PATTERN_BUTTON_COLUMNS;
        layers.push(hud_inventory_background_layer(
            if selected_pattern_index == Some(pattern_index) {
                HudInventoryBackgroundTexture::LoomPatternSelected
            } else {
                HudInventoryBackgroundTexture::LoomPattern
            },
            LOOM_PATTERN_BUTTON_X + column * LOOM_PATTERN_BUTTON_SIZE as i32,
            LOOM_PATTERN_BUTTON_Y + row * LOOM_PATTERN_BUTTON_SIZE as i32,
            LOOM_PATTERN_BUTTON_SIZE,
            LOOM_PATTERN_BUTTON_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }
}

fn loom_selectable_pattern_count(world: &WorldStore) -> Option<i32> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(LOOM_MENU_TYPE_ID) {
        return None;
    }
    if !open_container_slot_has_item(world, 0) || !open_container_slot_has_item(world, 1) {
        return None;
    }
    if open_container_slot_has_item(world, 2) {
        Some(LOOM_PATTERN_ITEM_PATTERN_COUNT)
    } else {
        Some(LOOM_NO_ITEM_REQUIRED_PATTERN_COUNT)
    }
}

fn loom_selected_pattern_index(world: &WorldStore) -> Option<i32> {
    world
        .open_container_data_value(LOOM_SELECTED_PATTERN_DATA_ID)
        .and_then(|value| (value >= 0).then_some(i32::from(value)))
}

fn loom_pattern_start_index(selectable_count: i32, scroll_row: i32) -> Option<i32> {
    if selectable_count <= 0 {
        return None;
    }
    let max_scroll_row =
        (loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0);
    Some(scroll_row.clamp(0, max_scroll_row) * LOOM_PATTERN_BUTTON_COLUMNS)
}

fn loom_pattern_row_count(selectable_count: i32) -> i32 {
    if selectable_count <= 0 {
        0
    } else {
        (selectable_count + LOOM_PATTERN_BUTTON_COLUMNS - 1) / LOOM_PATTERN_BUTTON_COLUMNS
    }
}

fn loom_scroller_offset(selectable_count: i32, scroll_row: i32) -> i32 {
    let max_scroll_row =
        (loom_pattern_row_count(selectable_count) - LOOM_PATTERN_BUTTON_ROWS).max(0);
    if max_scroll_row <= 0 {
        return 0;
    }
    let scroll_row = scroll_row.clamp(0, max_scroll_row);
    ((scroll_row as f32 / max_scroll_row as f32) * LOOM_SCROLLER_MAX_OFFSET as f32) as i32
}

fn push_enchanting_table_state_layers(
    world: &WorldStore,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    if open_container_slot_has_item(world, 1) {
        for slot in enchanting_table_option_layers(world) {
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
    for slot in enchanting_table_option_layers(world) {
        layers.push(slot);
    }
}

fn enchanting_table_option_layers(world: &WorldStore) -> Vec<HudInventoryBackgroundLayer> {
    let mut layers = Vec::with_capacity((ENCHANTING_TABLE_OPTION_COUNT as usize) * 2);
    for index in 0..ENCHANTING_TABLE_OPTION_COUNT {
        let y = ENCHANTING_TABLE_OPTION_Y + i32::from(index) * ENCHANTING_TABLE_OPTION_SPACING;
        let cost = world.open_container_data_value(index).unwrap_or_default();
        if cost <= 0 {
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
        let enabled = enchanting_table_option_is_enabled(world, index, cost);

        layers.push(hud_inventory_background_layer(
            if enabled {
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlot
            } else {
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled
            },
            ENCHANTING_TABLE_OPTION_X,
            y,
            ENCHANTING_TABLE_OPTION_WIDTH,
            ENCHANTING_TABLE_OPTION_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
        layers.push(hud_inventory_background_layer(
            enchanting_table_level_texture(index, enabled),
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

fn enchanting_table_option_is_enabled(world: &WorldStore, index: i16, cost: i16) -> bool {
    if world.gameplay().game_type == 1 {
        return true;
    }
    let required_lapis = i32::from(index) + 1;
    let lapis_count = open_container_slot_item(world, 1)
        .filter(|item| !item_stack_is_empty(item))
        .map(|item| item.count)
        .unwrap_or_default();
    if lapis_count < required_lapis {
        return false;
    }
    world.local_player().experience.is_some_and(|experience| {
        experience.level >= i32::from(cost) && experience.level >= required_lapis
    })
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

fn push_cartography_table_result_layers(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    layers: &mut Vec<HudInventoryBackgroundLayer>,
) {
    let Some(input_map) = open_container_slot_item(world, CARTOGRAPHY_TABLE_MAP_SLOT) else {
        return;
    };
    let Some(map_id) = item_stack_map_id(input_map) else {
        return;
    };

    let additional_item = open_container_slot_item(world, CARTOGRAPHY_TABLE_ADDITIONAL_SLOT);
    let additional =
        additional_item.and_then(|item| cartography_additional_item(item_runtime, item));
    let map_state = world.map_item(map_id);
    let invalid_transform = match additional {
        Some(CartographyAdditionalItem::Paper) => {
            map_state.is_some_and(|map| map.locked || map.scale >= 4)
        }
        Some(CartographyAdditionalItem::GlassPane) => map_state.is_some_and(|map| map.locked),
        _ => false,
    };

    if invalid_transform {
        layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CartographyTableError,
            CARTOGRAPHY_TABLE_ERROR_X,
            CARTOGRAPHY_TABLE_ERROR_Y,
            CARTOGRAPHY_TABLE_ERROR_WIDTH,
            CARTOGRAPHY_TABLE_ERROR_HEIGHT,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let result_slot_mode = open_container_slot_item(world, 2).and_then(cartography_result_mode);
    let mode = if invalid_transform {
        CartographyResultMode::Map
    } else {
        match additional {
            Some(CartographyAdditionalItem::Paper) => CartographyResultMode::Scaled,
            Some(CartographyAdditionalItem::EmptyMap) => CartographyResultMode::Duplicated,
            Some(CartographyAdditionalItem::GlassPane) => CartographyResultMode::Locked,
            None => result_slot_mode.unwrap_or(CartographyResultMode::Map),
        }
    };
    push_cartography_table_mode_layers(layers, mode);
}

fn push_cartography_table_mode_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    mode: CartographyResultMode,
) {
    match mode {
        CartographyResultMode::Map => push_cartography_table_map_layer(
            layers,
            CARTOGRAPHY_TABLE_MAP_X,
            CARTOGRAPHY_TABLE_MAP_Y,
        ),
        CartographyResultMode::Scaled => layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::CartographyTableScaledMap,
            CARTOGRAPHY_TABLE_MAP_X,
            CARTOGRAPHY_TABLE_MAP_Y,
            CARTOGRAPHY_TABLE_MAP_SIZE,
            CARTOGRAPHY_TABLE_MAP_SIZE,
            [0.0, 0.0],
            [1.0, 1.0],
        )),
        CartographyResultMode::Duplicated => {
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                CARTOGRAPHY_TABLE_MAP_X + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                CARTOGRAPHY_TABLE_MAP_Y,
                CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
            layers.push(hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y + CARTOGRAPHY_TABLE_DUPLICATED_MAP_OFFSET,
                CARTOGRAPHY_TABLE_DUPLICATED_MAP_WIDTH,
                CARTOGRAPHY_TABLE_DUPLICATED_MAP_HEIGHT,
                [0.0, 0.0],
                [1.0, 1.0],
            ));
        }
        CartographyResultMode::Locked => {
            push_cartography_table_map_layer(
                layers,
                CARTOGRAPHY_TABLE_MAP_X,
                CARTOGRAPHY_TABLE_MAP_Y,
            );
            layers.push(hud_inventory_background_layer(
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
    item_runtime: Option<&NativeItemRuntime>,
    item: &ItemStackSummary,
) -> Option<CartographyAdditionalItem> {
    if item_stack_is_empty(item) {
        return None;
    }
    cartography_additional_item_for_resource_id(
        item_runtime?.item_resource_id_for_protocol_id(item.item_id?)?,
    )
}

fn cartography_additional_item_for_resource_id(
    resource_id: &str,
) -> Option<CartographyAdditionalItem> {
    match resource_id {
        "minecraft:paper" => Some(CartographyAdditionalItem::Paper),
        "minecraft:map" => Some(CartographyAdditionalItem::EmptyMap),
        "minecraft:glass_pane" => Some(CartographyAdditionalItem::GlassPane),
        _ => None,
    }
}

fn item_stack_map_id(item: &ItemStackSummary) -> Option<i32> {
    if item_stack_is_empty(item)
        || item
            .component_patch
            .removed_type_ids
            .contains(&MAP_ID_DATA_COMPONENT_TYPE_ID)
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

/// The `minecraft:trim_material` registry keys by holder id (registration
/// order), projected from the world's synced dynamic registry so trimmed-armor
/// icons select their trim model (vanilla `TrimMaterialProperty`).
fn world_trim_material_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:trim_material")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
}

/// The `minecraft:enchantment` registry keys by holder id (registration
/// order), projected from the world's synced dynamic registry so item-model
/// use properties can apply vanilla enchantment effects such as Quick Charge.
fn world_enchantment_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:enchantment")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
}

/// The `minecraft:attribute` registry keys by holder id (registration order),
/// projected from the world's synced dynamic registry so item-model
/// `minecraft:attribute_modifiers` predicates can match entry attributes.
fn world_attribute_keys(world: &WorldStore) -> Option<Vec<String>> {
    world
        .registry_content("minecraft:attribute")
        .map(|registry| {
            registry
                .entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
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
    let item_runtime = item_runtime?;
    let trim_material_keys = world_trim_material_keys(world);
    let enchantment_keys = world_enchantment_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let owner_main_hand_left = world.local_player_main_arm_left();
    let context_entity_type = Some("minecraft:player");
    let context_dimension = world.level_info().map(|level| level.dimension.as_str());
    let time_context = world
        .world_time()
        .map(|time| bbb_item_model::ItemModelTimeContext {
            game_time: time.game_time,
            day_time: time.day_time,
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
    // Vanilla `Cooldown.get` uses `getCooldownPercent(itemStack, 0.0F)` for
    // item-model range dispatch. The HUD overlay below still uses render
    // partial tick.
    let item_model_cooldown_progress =
        item_cooldown_percent_for_stack(world, Some(item_runtime), item, 0.0).unwrap_or(0.0);
    let use_context = if using_item {
        item_runtime.item_model_use_context_for_stack_with_enchantment_keys(
            item,
            world.local_player().interaction.using_item_ticks,
            enchantment_keys.as_deref(),
        )
    } else {
        bbb_item_model::ItemModelUseContext::inactive()
    };
    let icon = item_runtime
        .icon_for_stack_with_context_and_use_context_time_state_and_fishing_rod_cast_with_registry_context(
            item,
            local_selected_bundle_item_index,
            using_item,
            use_context,
            bbb_pack::BlockModelDisplayContext::Gui,
            item_model_cooldown_progress,
            trim_material_keys.as_deref(),
            enchantment_keys.as_deref(),
            attribute_keys.as_deref(),
            owner_main_hand_left,
            context_entity_type,
            context_dimension,
            time_context,
            compass_context,
            selected_item,
            carried_item,
            true,
            shift_down,
            keybind_context,
            fishing_rod_cast,
            item_model_seed,
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
    let Some(last) = ticks.last_entity_animation_at else {
        ticks.last_entity_animation_at = Some(now);
        return 0;
    };
    let elapsed = now.saturating_duration_since(last);
    let raw_ticks = elapsed.as_millis() / CLIENT_ENTITY_ANIMATION_TICK_INTERVAL.as_millis();
    if raw_ticks == 0 {
        return 0;
    }

    let advanced_ticks = u32::try_from(raw_ticks).unwrap_or(u32::MAX);
    world.advance_entity_client_animations(advanced_ticks);
    let advanced = Duration::from_millis(
        u64::from(advanced_ticks)
            .saturating_mul(CLIENT_ENTITY_ANIMATION_TICK_INTERVAL.as_millis() as u64),
    );
    ticks.last_entity_animation_at = last.checked_add(advanced).or(Some(now));
    advanced_ticks
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CameraEnvironmentColors {
    sky_color: Option<[u8; 3]>,
    fog_color: Option<[u8; 3]>,
    water_fog_color: Option<[u8; 3]>,
    water_fog_end_distance: Option<f32>,
    camera_forward: Option<[f32; 3]>,
    fog_type: CameraFogType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CameraFogType {
    #[default]
    Atmospheric,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BiomeRgbAttribute {
    Sky,
    Fog,
    WaterFog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BiomeFloatAttribute {
    WaterFogEndDistance,
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

fn lightning_bolt_seed(uuid: uuid::Uuid) -> i64 {
    // Vanilla keeps LightningBolt.seed client-local and does not sync it in AddEntity.
    let bytes = uuid.as_u128().to_le_bytes();
    i64::from_le_bytes(bytes[..8].try_into().expect("uuid lower 64 bits"))
}

fn world_can_have_weather(world: &WorldStore) -> bool {
    world.level_info().is_some_and(|level| {
        level.dimension == "minecraft:overworld" || level.dimension_type_id == 0
    })
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

#[cfg(test)]
fn camera_biome_sky_color(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
) -> Option<[u8; 3]> {
    camera_biome_rgb_color(world, terrain_textures, camera_pose, BiomeRgbAttribute::Sky)
}

#[cfg(test)]
fn camera_biome_fog_color(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
) -> Option<[u8; 3]> {
    camera_biome_rgb_color(world, terrain_textures, camera_pose, BiomeRgbAttribute::Fog)
}

#[cfg(test)]
fn camera_biome_water_fog_color(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
) -> Option<[u8; 3]> {
    camera_biome_rgb_color(
        world,
        terrain_textures,
        camera_pose,
        BiomeRgbAttribute::WaterFog,
    )
}

fn camera_environment_colors(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
) -> CameraEnvironmentColors {
    let Some(camera) = camera_pose else {
        return CameraEnvironmentColors::default();
    };
    let eye = camera_eye_position(camera);
    CameraEnvironmentColors {
        sky_color: gaussian_biome_rgb_color(world, terrain_textures, eye, BiomeRgbAttribute::Sky),
        fog_color: gaussian_biome_rgb_color(world, terrain_textures, eye, BiomeRgbAttribute::Fog),
        water_fog_color: gaussian_biome_rgb_color(
            world,
            terrain_textures,
            eye,
            BiomeRgbAttribute::WaterFog,
        ),
        water_fog_end_distance: gaussian_biome_float_attribute(
            world,
            terrain_textures,
            eye,
            BiomeFloatAttribute::WaterFogEndDistance,
            VANILLA_DEFAULT_WATER_FOG_END_DISTANCE,
        ),
        camera_forward: Some(camera_forward_vector(camera)),
        fog_type: if camera_eye_in_water(world, eye) {
            CameraFogType::Water
        } else {
            CameraFogType::Atmospheric
        },
    }
}

#[cfg(test)]
fn camera_biome_rgb_color(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    attribute: BiomeRgbAttribute,
) -> Option<[u8; 3]> {
    gaussian_biome_rgb_color(
        world,
        terrain_textures,
        camera_eye_position(camera_pose?),
        attribute,
    )
}

fn camera_eye_position(camera: CameraPose) -> [f32; 3] {
    [
        camera.position[0],
        camera.position[1] + camera.eye_height,
        camera.position[2],
    ]
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

fn particle_local_player_scope_context(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    camera_pose: Option<CameraPose>,
) -> Option<ParticleLocalPlayerScopeContext> {
    let item_id = world.local_using_item_item_id()?;
    let scoping = item_runtime.and_then(|items| items.item_resource_id(item_id))
        == Some("minecraft:spyglass");
    if !scoping {
        return None;
    }
    let camera = camera_pose?;
    let eye_position = camera_eye_position(camera).map(f64::from);
    Some(ParticleLocalPlayerScopeContext {
        eye_position,
        first_person: world.local_player().camera.follows_player,
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

fn camera_forward_vector(camera: CameraPose) -> [f32; 3] {
    let yaw = camera.y_rot.to_radians();
    let pitch = camera.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    [-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch]
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

fn gaussian_biome_rgb_color(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    eye: [f32; 3],
    attribute: BiomeRgbAttribute,
) -> Option<[u8; 3]> {
    if !eye.into_iter().all(f32::is_finite) {
        return None;
    }

    let position = [
        f64::from(eye[0]) * 0.25 - 0.5,
        f64::from(eye[1]) * 0.25 - 0.5,
        f64::from(eye[2]) * 0.25 - 0.5,
    ];
    let integral = [
        position[0].floor() as i32,
        position[1].floor() as i32,
        position[2].floor() as i32,
    ];
    let relative = [
        position[0] - f64::from(integral[0]),
        position[1] - f64::from(integral[1]),
        position[2] - f64::from(integral[2]),
    ];
    let mut samples: Vec<([u8; 3], f64)> = Vec::new();

    for z in 0..6 {
        let weight_z = gaussian_axis_weight(z, relative[2]);
        let sample_z = integral[2] - 2 + z as i32;
        for x in 0..6 {
            let weight_x = gaussian_axis_weight(x, relative[0]);
            let sample_x = integral[0] - 2 + x as i32;
            for y in 0..6 {
                let weight_y = gaussian_axis_weight(y, relative[1]);
                let sample_y = integral[1] - 2 + y as i32;
                let weight = weight_x * weight_y * weight_z;
                if weight <= 0.0 {
                    continue;
                }
                let Some(color) = biome_rgb_color_at_quart(
                    world,
                    terrain_textures,
                    sample_x,
                    sample_y,
                    sample_z,
                    attribute,
                ) else {
                    continue;
                };
                accumulate_weighted_rgb_color(&mut samples, color, weight);
            }
        }
    }

    lerp_weighted_rgb_colors(&samples)
}

fn gaussian_biome_float_attribute(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    eye: [f32; 3],
    attribute: BiomeFloatAttribute,
    base_value: f32,
) -> Option<f32> {
    if !eye.into_iter().all(f32::is_finite) {
        return None;
    }

    let position = [
        f64::from(eye[0]) * 0.25 - 0.5,
        f64::from(eye[1]) * 0.25 - 0.5,
        f64::from(eye[2]) * 0.25 - 0.5,
    ];
    let integral = [
        position[0].floor() as i32,
        position[1].floor() as i32,
        position[2].floor() as i32,
    ];
    let relative = [
        position[0] - f64::from(integral[0]),
        position[1] - f64::from(integral[1]),
        position[2] - f64::from(integral[2]),
    ];
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for z in 0..6 {
        let weight_z = gaussian_axis_weight(z, relative[2]);
        let sample_z = integral[2] - 2 + z as i32;
        for x in 0..6 {
            let weight_x = gaussian_axis_weight(x, relative[0]);
            let sample_x = integral[0] - 2 + x as i32;
            for y in 0..6 {
                let weight_y = gaussian_axis_weight(y, relative[1]);
                let sample_y = integral[1] - 2 + y as i32;
                let weight = weight_x * weight_y * weight_z;
                if weight <= 0.0 {
                    continue;
                }
                let Some(value) = biome_float_attribute_at_quart(
                    world,
                    terrain_textures,
                    sample_x,
                    sample_y,
                    sample_z,
                    attribute,
                    base_value,
                ) else {
                    continue;
                };
                weighted_sum += f64::from(value) * weight;
                total_weight += weight;
            }
        }
    }

    (total_weight > 0.0).then_some((weighted_sum / total_weight) as f32)
}

fn gaussian_axis_weight(index: usize, relative: f64) -> f64 {
    let start = VANILLA_GAUSSIAN_SAMPLE_KERNEL[index + 1];
    let end = VANILLA_GAUSSIAN_SAMPLE_KERNEL[index];
    start + relative * (end - start)
}

fn biome_rgb_color_at_quart(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    quart_x: i32,
    quart_y: i32,
    quart_z: i32,
    attribute: BiomeRgbAttribute,
) -> Option<[u8; 3]> {
    let pos = BlockPos {
        x: quart_x.saturating_mul(4),
        y: quart_y.saturating_mul(4),
        z: quart_z.saturating_mul(4),
    };
    let biome_id = world.probe_block(pos)?.biome_id;
    match attribute {
        BiomeRgbAttribute::Sky => terrain_textures.biome_sky_color(biome_id),
        BiomeRgbAttribute::Fog => terrain_textures.biome_fog_color(biome_id),
        BiomeRgbAttribute::WaterFog => terrain_textures.biome_water_fog_color(biome_id),
    }
}

fn biome_float_attribute_at_quart(
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    quart_x: i32,
    quart_y: i32,
    quart_z: i32,
    attribute: BiomeFloatAttribute,
    base_value: f32,
) -> Option<f32> {
    let pos = BlockPos {
        x: quart_x.saturating_mul(4),
        y: quart_y.saturating_mul(4),
        z: quart_z.saturating_mul(4),
    };
    let biome_id = world.probe_block(pos)?.biome_id;
    match attribute {
        BiomeFloatAttribute::WaterFogEndDistance => {
            terrain_textures.biome_water_fog_end_distance(biome_id, base_value)
        }
    }
}

fn accumulate_weighted_rgb_color(samples: &mut Vec<([u8; 3], f64)>, color: [u8; 3], weight: f64) {
    if let Some((_, sample_weight)) = samples
        .iter_mut()
        .find(|(sample_color, _)| *sample_color == color)
    {
        *sample_weight += weight;
    } else {
        samples.push((color, weight));
    }
}

fn lerp_weighted_rgb_colors(samples: &[([u8; 3], f64)]) -> Option<[u8; 3]> {
    let mut total_weight = 0.0;
    let mut result = None;
    for (color, weight) in samples {
        if *weight <= 0.0 {
            continue;
        }
        total_weight += weight;
        let color = rgb_u8_to_argb(*color);
        result = Some(match result {
            Some(result) => argb_srgb_lerp((*weight / total_weight) as f32, result, color),
            None => color,
        });
    }
    result.map(rgb_u8_from_argb)
}

fn camera_eye_in_water(world: &WorldStore, eye: [f32; 3]) -> bool {
    if !eye.into_iter().all(f32::is_finite) {
        return false;
    }
    let pos = BlockPos {
        x: eye[0].floor() as i32,
        y: eye[1].floor() as i32,
        z: eye[2].floor() as i32,
    };
    let Some(fluid) = world.probe_block(pos).and_then(|block| block.fluid) else {
        return false;
    };
    fluid.kind == TerrainFluidKind::Water
        && f64::from(eye[1]) < f64::from(pos.y) + camera_fluid_height_at(world, pos, fluid)
}

fn camera_fluid_height_at(world: &WorldStore, pos: BlockPos, fluid: TerrainFluidState) -> f64 {
    let same_fluid_above = pos.y.checked_add(1).is_some_and(|above_y| {
        world
            .probe_block(BlockPos {
                x: pos.x,
                y: above_y,
                z: pos.z,
            })
            .and_then(|block| block.fluid)
            .is_some_and(|above| above.kind == fluid.kind)
    });
    if same_fluid_above {
        1.0
    } else {
        fluid.own_height()
    }
}

fn rgb_u8_to_argb(color: [u8; 3]) -> i32 {
    argb_color(
        255,
        i32::from(color[0]),
        i32::from(color[1]),
        i32::from(color[2]),
    )
}

fn rgb_u8_from_argb(color: i32) -> [u8; 3] {
    [
        argb_red(color) as u8,
        argb_green(color) as u8,
        argb_blue(color) as u8,
    ]
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
    }
}

fn audio_listener_state_from_world(world: &WorldStore) -> Option<AudioListenerState> {
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
        entity_target_draw_calls: counters.entity_target_draw_calls,
        entity_scene_boxes: counters.entity_scene_boxes,
        item_entity_billboards: counters.item_entity_billboards,
        hud_draw_calls: counters.hud_draw_calls,
        pipeline_switches: counters.pipeline_switches,
        screenshots_written: counters.screenshots_written,
        queued_sections: counters.queued_sections,
        meshed_sections: counters.meshed_sections,
        uploaded_sections: counters.uploaded_sections,
        visible_sections: counters.visible_sections,
        upload_bytes: counters.upload_bytes,
        resident_bytes: counters.resident_bytes,
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
    }
}

pub(crate) fn publish_snapshot(
    snapshot: &SharedSnapshot,
    renderer: RendererCounters,
    net: &NetCounters,
    audio: &AudioCounters,
) -> bool {
    if let Ok(mut guard) = snapshot.write() {
        guard.renderer = renderer;
        guard.net = net.clone();
        guard.audio = audio.clone();
        guard.app.running
    } else {
        false
    }
}

#[cfg(test)]
mod tests;
