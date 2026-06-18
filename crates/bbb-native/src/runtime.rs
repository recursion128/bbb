use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{AudioCounters, NetCounters, RendererCounters, SharedSnapshot};
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::{
    codec::Decoder,
    packets::{ItemCostSummary, ItemStackSummary, MapPostProcessingSummary, SlotDisplaySummary},
};
use bbb_renderer::{
    BlockDestroyOverlay, CameraPose, ClearColor, HudIconLayer, HudInventoryBackgroundLayer,
    HudInventoryBackgroundTexture, HudInventoryItem, HudInventoryScreen, HudInventorySlot,
    HudInventoryTextBackground, HudInventoryTextLabel, HudInventoryTooltip, HudItemCountLabel,
    HudItemDurabilityBar, HudItemIcon, HudUvRect, HUD_HOTBAR_SLOTS,
};
use bbb_world::{
    ContainerState, MerchantOfferState, MerchantOffersState, MountArmorSlotKind,
    MountInventoryKind, WorldStore,
};
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    camera_pose::camera_pose_from_world,
    code_of_conduct::CodeOfConductAcceptance,
    crosshair::{entity_target_outline_from_camera_at_partial_tick, selection_outline_from_camera},
    entity_scene::entity_scene_outline_from_world_at_partial_tick,
    input::{
        advance_destroying_block_at_partial_tick, advance_player_input,
        advance_using_item_at_partial_tick, inventory_screen_layout,
        sync_beacon_effect_selection_state, sync_loom_pattern_state_for_hud,
        sync_stonecutter_recipe_scroll_state, ClientInputState, InventoryScreenBackground,
    },
    item_entities::item_entity_billboards_from_world,
    item_runtime::NativeItemRuntime,
    particle_runtime::ParticleEventSink,
    terrain_runtime::{
        maybe_upload_decoded_terrain, maybe_upload_terrain_texture_animation, TerrainTextureState,
        TerrainUploadState,
    },
};

mod control_requests;
mod events;

const CLIENT_ENTITY_ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(50);
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
}

pub(crate) use control_requests::pump_control_net_requests;

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

pub(crate) fn take_control_screenshot(snapshot: &SharedSnapshot) -> Option<PathBuf> {
    snapshot
        .write()
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
    terrain_upload: &mut TerrainUploadState,
    terrain_textures: &TerrainTextureState,
    item_runtime: Option<&NativeItemRuntime>,
    snapshot: &SharedSnapshot,
    code_of_conduct: Option<&mut CodeOfConductAcceptance>,
) -> bool {
    let mut audio_events = audio_events;
    let mut particle_events = particle_events;
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
        );
    }
    pump_control_net_requests(snapshot, net_commands, net_counters, world, code_of_conduct);
    let now = Instant::now();
    let advanced_ticks = advance_entity_client_animations(world, client_animation_ticks, now);
    let entity_partial_tick = client_animation_ticks.entity_partial_tick(now);
    advance_block_destruction_render_ticks(world, advanced_ticks);
    world.advance_item_cooldowns(advanced_ticks);
    renderer.advance_particles(advanced_ticks);
    advance_player_input(input, world, net_counters, net_commands, now);
    advance_destroying_block_at_partial_tick(
        input,
        world,
        net_counters,
        net_commands,
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
    let local_player = world.local_player();
    renderer.set_hud_health(local_player.health.map(|health| health.health));
    renderer.set_hud_food(local_player.health.map(|health| health.food));
    renderer.set_hud_experience_progress(
        local_player
            .experience
            .map(|experience| experience.progress),
    );
    renderer.set_hud_selected_slot(local_player.selected_hotbar_slot);
    renderer.set_hud_hotbar_item_icons(hotbar_item_icons(world, item_runtime, entity_partial_tick));
    sync_stonecutter_recipe_scroll_state(input, world);
    sync_beacon_effect_selection_state(input, world);
    sync_loom_pattern_state_for_hud(input, world);
    renderer.set_hud_inventory_screen(hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        input.inventory_hovered_slot(),
        InventoryHudLocalState {
            stonecutter_recipe_scroll_row: Some(input.stonecutter_recipe_scroll_row()),
            beacon_effect_selection: Some(input.beacon_effect_selection()),
            loom_pattern_scroll_row: Some(input.loom_pattern_scroll_row()),
            loom_selected_pattern_index: input.loom_selected_pattern_index(),
            anvil_rename_text: Some(input.anvil_rename_text().to_string()),
        },
        entity_partial_tick,
    ));
    renderer.set_item_entity_billboards(item_entity_billboards_from_world(world, item_runtime));
    let camera_pose = camera_pose_from_world(world);
    renderer.set_camera_pose(camera_pose);
    renderer.set_selection_outline(selection_outline_from_camera(world, camera_pose));
    renderer.set_entity_scene_outline(entity_scene_outline_from_world_at_partial_tick(
        world,
        entity_partial_tick,
    ));
    renderer.set_entity_target_outline(entity_target_outline_from_camera_at_partial_tick(
        world,
        camera_pose,
        entity_partial_tick,
    ));
    renderer.set_block_destroy_overlays(block_destroy_overlays_from_world(world, terrain_textures));
    maybe_upload_terrain_texture_animation(renderer, terrain_upload, terrain_textures);
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
        renderer.counters(),
        net_counters,
        &audio_counters,
        world,
    )
}

fn advance_block_destruction_render_ticks(world: &mut WorldStore, advanced_ticks: u32) -> usize {
    let running_ticks = world.consume_running_render_ticks(advanced_ticks);
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

fn hotbar_item_icons(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    partial_tick: f32,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    let mut icons = std::array::from_fn(|_| None);
    for (slot_index, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        icons[slot_index] = hud_item_icon_for_stack(
            world,
            item_runtime,
            &item.item,
            item.local_selected_bundle_item_index(),
            partial_tick,
        );
    }

    icons
}

fn hud_inventory_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_inventory_screen_with_local_state(
        world,
        item_runtime,
        hovered_slot_id,
        InventoryHudLocalState::default(),
        partial_tick,
    )
}

fn hud_inventory_screen_with_local_state(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
    local_state: InventoryHudLocalState,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let layout = inventory_screen_layout(world)?;
    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };

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
                        partial_tick,
                    )
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
            layout.background,
            local_state.stonecutter_recipe_scroll_row,
            partial_tick,
        ),
        text_labels: hud_inventory_text_labels(world, layout.background, &local_state),
        hovered_slot_id: hovered_slot_id.and_then(|slot| u16::try_from(slot).ok()),
        tooltip: hud_inventory_tooltip(item_runtime, hovered_slot_id, &layout.slots, container),
    })
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
                });
            }
            if let Some(label) = anvil_cost_text_label(world) {
                labels.push(label);
            }
            labels
        }
        InventoryScreenBackground::EnchantmentTable => enchanting_table_cost_text_labels(world),
        _ => Vec::new(),
    }
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
    })
}

fn hud_ascii_approx_text_width(text: &str) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(if ch == ' ' { 4 } else { 6 })?;
    }
    (width > 0).then_some(width)
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
        lines,
    })
}

fn hud_inventory_floating_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    stonecutter_recipe_scroll_row: Option<i32>,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    match background {
        InventoryScreenBackground::Merchant => {
            hud_merchant_trade_items(world, item_runtime, partial_tick)
        }
        InventoryScreenBackground::Stonecutter => hud_stonecutter_recipe_items(
            world,
            item_runtime,
            stonecutter_recipe_scroll_row.unwrap_or_default(),
            partial_tick,
        ),
        _ => Vec::new(),
    }
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
    partial_tick: f32,
    items: &mut Vec<HudInventoryItem>,
    x: i32,
    y: i32,
    item: ItemStackSummary,
) {
    if let Some(icon) = hud_item_icon_for_stack(world, item_runtime, &item, None, partial_tick) {
        items.push(HudInventoryItem { x, y, icon });
    }
}

fn hud_stonecutter_recipe_items(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    scroll_row: i32,
    partial_tick: f32,
) -> Vec<HudInventoryItem> {
    let mut items = Vec::new();
    for option in stonecutter_visible_recipe_option_stacks(world, scroll_row) {
        if let Some(icon) =
            hud_item_icon_for_stack(world, item_runtime, &option.stack, None, partial_tick)
        {
            items.push(HudInventoryItem {
                x: option.x,
                y: option.y,
                icon,
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

fn hud_item_icon_for_stack(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
    partial_tick: f32,
) -> Option<HudItemIcon> {
    let icon = item_runtime?
        .icon_for_stack_with_bundle_selected_item(item, local_selected_bundle_item_index)?;
    Some(HudItemIcon {
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
        count_label: hud_item_count_label_for_stack(item),
        durability_bar: hud_item_durability_bar_for_stack(item),
        cooldown_progress: hud_item_cooldown_progress_for_stack(
            world,
            item_runtime,
            item,
            partial_tick,
        ),
    })
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
    let cooldown_group = item_cooldown_group(item_runtime, item)?;
    let progress = world.item_cooldown_percent(&cooldown_group, partial_tick);
    (progress > 0.0).then_some(progress)
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
    item: &bbb_protocol::packets::ItemStackSummary,
) -> Option<HudItemDurabilityBar> {
    if item_stack_is_empty(item) || item.component_patch.unbreakable {
        return None;
    }

    let max_damage = item.component_patch.max_damage?;
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

pub(crate) fn clear_color_for_world(world: &WorldStore) -> ClearColor {
    let day_time = world.world_time().map(|time| time.day_time).unwrap_or(6000);
    let weather = world.weather();
    let rain = weather.rain_level.clamp(0.0, 1.0) as f64;
    let thunder = weather.thunder_level.clamp(0.0, 1.0) as f64;
    clear_color_for_day_time(day_time, rain, thunder)
}

fn clear_color_for_day_time(day_time: i64, rain_level: f64, thunder_level: f64) -> ClearColor {
    let phase = day_time.rem_euclid(24_000) as f64 / 24_000.0;
    let noon_aligned = (phase - 0.25) * std::f64::consts::TAU;
    let daylight = ((noon_aligned.cos() + 1.0) * 0.5).powf(0.65);
    let weather_dim = (1.0 - rain_level * 0.25 - thunder_level * 0.45).clamp(0.25, 1.0);
    let night = [0.015, 0.025, 0.055];
    let day = [0.50, 0.72, 0.95];
    ClearColor {
        r: (night[0] + (day[0] - night[0]) * daylight) * weather_dim,
        g: (night[1] + (day[1] - night[1]) * daylight) * weather_dim,
        b: (night[2] + (day[2] - night[2]) * daylight) * weather_dim,
        a: 1.0,
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
            pose.position.y + f64::from(CameraPose::STANDING_EYE_HEIGHT),
            pose.position.z,
        ],
        y_rot: pose.y_rot,
        x_rot: pose.x_rot,
    })
}

pub(crate) fn publish_snapshot(
    snapshot: &SharedSnapshot,
    renderer: RendererCounters,
    net: &NetCounters,
    audio: &AudioCounters,
    world: &WorldStore,
) -> bool {
    if let Ok(mut guard) = snapshot.write() {
        guard.renderer = renderer;
        guard.net = net.clone();
        guard.audio = audio.clone();
        guard.world_store = world.clone();
        guard.app.running
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use bbb_protocol::packets::{MerchantOffer, MerchantOffers};
    use bbb_world::LocalPlayerPoseState;

    #[test]
    fn camera_pose_uses_standing_eye_height() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            y_rot: 45.0,
            x_rot: -10.0,
            ..LocalPlayerPoseState::default()
        });
        let pose = camera_pose_from_world(&world).unwrap();

        assert_eq!(pose.position, [1.0, 2.0, 3.0]);
        assert_eq!(pose.y_rot, 45.0);
        assert_eq!(pose.x_rot, -10.0);
        assert_eq!(pose.eye_height, CameraPose::STANDING_EYE_HEIGHT);
    }

    #[test]
    fn entity_animation_partial_tick_tracks_time_since_last_client_tick() {
        let now = Instant::now();
        let mut ticks = ClientAnimationTickState::default();
        let mut world = WorldStore::new();

        assert_eq!(ticks.entity_partial_tick(now), 1.0);
        assert_eq!(
            advance_entity_client_animations(&mut world, &mut ticks, now),
            0
        );
        assert_eq!(ticks.entity_partial_tick(now), 0.0);
        assert_eq!(
            ticks.entity_partial_tick(now + Duration::from_millis(25)),
            0.5
        );
        assert_eq!(
            ticks.entity_partial_tick(now + Duration::from_millis(75)),
            1.0
        );
    }

    #[test]
    fn renderer_camera_pose_follows_active_camera_entity() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 123,
            uuid: uuid::Uuid::from_u128(123),
            entity_type_id: 7,
            position: bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: -15.0,
            y_rot: 30.0,
            y_head_rot: 30.0,
            data: 0,
        });

        assert_eq!(
            camera_pose_from_world(&world),
            Some(CameraPose {
                position: [10.0, 64.0, -5.0],
                y_rot: 90.0,
                x_rot: -10.0,
                eye_height: CameraPose::STANDING_EYE_HEIGHT,
            })
        );

        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
        assert_eq!(
            camera_pose_from_world(&world),
            Some(CameraPose {
                position: [1.0, 2.0, 3.0],
                y_rot: 30.0,
                x_rot: -15.0,
                eye_height: 0.2751,
            })
        );

        assert_eq!(
            world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
                entity_ids: vec![123],
            }),
            1
        );
        assert_eq!(
            camera_pose_from_world(&world),
            Some(CameraPose {
                position: [10.0, 64.0, -5.0],
                y_rot: 90.0,
                x_rot: -10.0,
                eye_height: CameraPose::STANDING_EYE_HEIGHT,
            })
        );
    }

    #[test]
    fn audio_scene_command_tracks_listener_and_entity_positions() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(local_player_pose([10.0, 64.0, -5.0], 90.0, -10.0));
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id: 123,
            uuid: uuid::Uuid::from_u128(123),
            entity_type_id: 7,
            position: bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });

        let command = audio_scene_command_from_world(&world);

        assert_eq!(
            command.listener,
            Some(AudioListenerState {
                position: [
                    10.0,
                    64.0 + f64::from(CameraPose::STANDING_EYE_HEIGHT),
                    -5.0
                ],
                y_rot: 90.0,
                x_rot: -10.0,
            })
        );
        assert_eq!(
            command.entities,
            vec![EntitySoundPosition {
                entity_id: 123,
                position: [1.0, 2.0, 3.0],
            }]
        );

        assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
        let command = audio_scene_command_from_world(&world);
        assert_eq!(
            command.listener,
            Some(AudioListenerState {
                position: [1.0, 2.0 + f64::from(0.2751_f32), 3.0],
                y_rot: 0.0,
                x_rot: 0.0,
            })
        );

        assert_eq!(
            world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
                entity_ids: vec![123],
            }),
            1
        );
        let command = audio_scene_command_from_world(&world);
        assert!(command.entities.is_empty());
        assert_eq!(
            command.listener,
            Some(AudioListenerState {
                position: [
                    10.0,
                    64.0 + f64::from(CameraPose::STANDING_EYE_HEIGHT),
                    -5.0
                ],
                y_rot: 90.0,
                x_rot: -10.0,
            })
        );
    }

    #[test]
    fn hud_inventory_screen_projects_open_local_inventory_layout() {
        let mut world = WorldStore::new();
        assert_eq!(hud_inventory_screen(&world, None, Some(36), 0.0), None);

        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: bbb_protocol::packets::ItemStackSummary {
                item_id: Some(42),
                count: 3,
                component_patch: Default::default(),
            },
        });
        assert!(world.open_local_inventory());

        let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Inventory,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(36));
        assert_eq!(screen.tooltip, None);
        assert_eq!(screen.slots.len(), 46);
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
        assert_eq!(hotbar.x, 8);
        assert_eq!(hotbar.y, 142);
        assert!(hotbar.icon.is_none());
        let offhand = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
        assert_eq!(offhand.x, 77);
        assert_eq!(offhand.y, 62);
    }

    #[test]
    fn hud_inventory_screen_projects_hovered_item_tooltip_name() {
        let root = unique_runtime_temp_dir("inventory-tooltip");
        write_runtime_tooltip_item_assets(&root);
        let item_runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: item_stack(0, 1),
        });
        assert!(world.open_local_inventory());

        let screen = hud_inventory_screen(&world, Some(&item_runtime), Some(36), 0.0).unwrap();

        assert_eq!(
            screen.tooltip,
            Some(HudInventoryTooltip {
                slot_id: 36,
                x: 8,
                y: 142,
                lines: vec!["Test Combo".to_string()],
            })
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hud_inventory_screen_projects_generic_container_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 5,
            title: "Large Chest".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 90],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(89), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 222);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GenericContainer,
                    0,
                    0,
                    176,
                    125,
                    [0.0, 0.0],
                    [176.0 / 256.0, 125.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GenericContainer,
                    0,
                    125,
                    176,
                    96,
                    [0.0, 126.0 / 256.0],
                    [176.0 / 256.0, 222.0 / 256.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(89));
        assert_eq!(screen.slots.len(), 90);
        let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((first_container.x, first_container.y), (8, 18));
        let player_inventory = screen.slots.iter().find(|slot| slot.slot_id == 54).unwrap();
        assert_eq!((player_inventory.x, player_inventory.y), (8, 139));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 89).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 197));
    }

    #[test]
    fn hud_inventory_screen_projects_generic_3x3_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 6,
            title: "Dispenser".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 45],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(44), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Dispenser,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(44));
        assert_eq!(screen.slots.len(), 45);
        let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((first_container.x, first_container.y), (62, 17));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_crafter_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 7,
            title: "Crafter".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Crafter,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CrafterUnpoweredRedstone,
                    97,
                    35,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(45));
        assert_eq!(screen.slots.len(), 46);
        let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((first_grid.x, first_grid.y), (26, 17));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
        assert_eq!((result.x, result.y), (134, 35));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 44).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_crafter_state_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 7,
            title: "Crafter".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 0,
            value: 1,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 8,
            value: 1,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 9,
            value: 1,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Crafter,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CrafterDisabledSlot,
                    25,
                    16,
                    18,
                    18,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CrafterDisabledSlot,
                    61,
                    52,
                    18,
                    18,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CrafterPoweredRedstone,
                    97,
                    35,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_crafting_table_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 12,
            title: "Crafting".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 46],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(45), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CraftingTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(45));
        assert_eq!(screen.slots.len(), 46);
        let result = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((result.x, result.y), (124, 35));
        let first_grid = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((first_grid.x, first_grid.y), (30, 17));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 45).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_enchanting_table_layout_and_lapis_slot_layer() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 13,
            title: "Enchanting Table".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(37), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::EnchantingTable,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::EnchantingTableLapisSlot,
                    35,
                    47,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                    60,
                    14,
                    108,
                    19,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                    60,
                    33,
                    108,
                    19,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                    60,
                    52,
                    108,
                    19,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(37));
        assert_eq!(screen.slots.len(), 38);
        let item = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((item.x, item.y), (15, 47));
        let lapis = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((lapis.x, lapis.y), (35, 47));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_enchanting_table_enabled_option_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 13,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
        items[1] = item_stack(42, 2);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 1,
            value: 12,
        });
        world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
            progress: 0.0,
            level: 12,
            total: 0,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlot,
                60,
                33,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableLevel2,
                61,
                34,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 154,
                y: 42,
                width: 12,
                text: "12".to_string(),
                tint: ENCHANTING_TABLE_COST_TEXT_ENABLED_COLOR,
                background: None,
            }]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_enchanting_table_disabled_cost_label() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 13,
            title: "Enchanting Table".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
        items[1] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 2,
            value: 30,
        });
        world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
            progress: 0.0,
            level: 40,
            total: 0,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableEnchantmentSlotDisabled,
                60,
                52,
                108,
                19,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::EnchantingTableLevel3Disabled,
                61,
                53,
                16,
                16,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 154,
                y: 61,
                width: 12,
                text: "30".to_string(),
                tint: ENCHANTING_TABLE_COST_TEXT_DISABLED_COLOR,
                background: None,
            }]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_anvil_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 8,
            title: "Anvil".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
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
                    HudInventoryBackgroundTexture::AnvilTextFieldDisabled,
                    59,
                    20,
                    110,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((input.x, input.y), (27, 47));
        let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((additional.x, additional.y), (76, 47));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (134, 47));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_beacon_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 9,
            title: "Beacon".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 37],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(36), 0.0).unwrap();

        assert_eq!(screen.width, 230);
        assert_eq!(screen.height, 219);
        assert_eq!(screen.background_layers.len(), 17);
        assert_eq!(
            screen.background_layers[0],
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Beacon,
                0,
                0,
                230,
                219,
                [0.0, 0.0],
                [230.0 / 256.0, 219.0 / 256.0],
            )
        );
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButtonDisabled,
                53,
                22,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconEffectSpeed,
                55,
                24,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconEffectRegeneration,
                146,
                49,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButtonDisabled,
                164,
                107,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconConfirm,
                166,
                109,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButton,
                190,
                107,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconCancel,
                192,
                109,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert_eq!(screen.hovered_slot_id, Some(36));
        assert_eq!(screen.slots.len(), 37);
        let payment = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((payment.x, payment.y), (136, 110));
        let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((first_inventory.x, first_inventory.y), (36, 137));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 36).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (180, 195));
    }

    #[test]
    fn hud_inventory_screen_projects_active_beacon_confirm_button() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 9,
            title: "Beacon".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: BEACON_PRIMARY_EFFECT_DATA_ID,
            value: 5,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButton,
                164,
                107,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(!screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButtonDisabled,
                164,
                107,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
    }

    #[test]
    fn hud_inventory_screen_projects_local_beacon_effect_selection() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 9,
            title: "Beacon".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 37];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: BEACON_LEVELS_DATA_ID,
            value: 4,
        });

        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            None,
            InventoryHudLocalState {
                beacon_effect_selection: Some((
                    Some(BEACON_EFFECT_STRENGTH_ID),
                    Some(BEACON_EFFECT_STRENGTH_ID),
                )),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(screen.background_layers.len(), 19);
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButtonSelected,
                65,
                72,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconEffectStrength,
                67,
                74,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButtonSelected,
                168,
                47,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconEffectStrength,
                170,
                49,
                18,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BeaconButton,
                164,
                107,
                22,
                22,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
    }

    #[test]
    fn hud_inventory_screen_projects_anvil_text_field_and_error_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 8,
            title: "Anvil".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
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
                    HudInventoryBackgroundTexture::AnvilTextField,
                    59,
                    20,
                    110,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::AnvilError,
                    99,
                    45,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_anvil_rename_text_label() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 8,
            title: "Anvil".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            None,
            InventoryHudLocalState {
                anvil_rename_text: Some("Sharp Pick".to_string()),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 62,
                y: 24,
                width: 103,
                text: "Sharp Pick".to_string(),
                tint: ANVIL_RENAME_TEXT_COLOR,
                background: None,
            }]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_anvil_cost_label() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 8,
            title: "Anvil".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        items[2] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: ANVIL_COST_DATA_ID,
            value: 7,
        });
        world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
            progress: 0.0,
            level: 8,
            total: 0,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 126,
                y: ANVIL_COST_LABEL_Y,
                width: 40,
                text: "Cost: 7".to_string(),
                tint: ANVIL_COST_TEXT_COLOR,
                background: Some(HudInventoryTextBackground {
                    x: 124,
                    y: ANVIL_COST_BACKGROUND_Y,
                    width: 44,
                    height: ANVIL_COST_BACKGROUND_HEIGHT,
                    tint: ANVIL_COST_BACKGROUND_TINT,
                }),
            }]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_anvil_too_expensive_label() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 8,
            title: "Anvil".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: ANVIL_COST_DATA_ID,
            value: ANVIL_TOO_EXPENSIVE_LEVEL_COST,
        });
        world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
            progress: 0.0,
            level: 100,
            total: 0,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.text_labels,
            vec![HudInventoryTextLabel {
                x: 84,
                y: ANVIL_COST_LABEL_Y,
                width: 82,
                text: "Too Expensive!".to_string(),
                tint: ANVIL_COST_ERROR_TEXT_COLOR,
                background: Some(HudInventoryTextBackground {
                    x: 82,
                    y: ANVIL_COST_BACKGROUND_Y,
                    width: 86,
                    height: ANVIL_COST_BACKGROUND_HEIGHT,
                    tint: ANVIL_COST_BACKGROUND_TINT,
                }),
            }]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_brewing_stand_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 11,
            title: "Brewing Stand".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::BrewingStand,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(40));
        assert_eq!(screen.slots.len(), 41);
        let bottle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((bottle.x, bottle.y), (56, 51));
        let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
        assert_eq!((ingredient.x, ingredient.y), (79, 17));
        let fuel = screen.slots.iter().find(|slot| slot.slot_id == 4).unwrap();
        assert_eq!((fuel.x, fuel.y), (17, 17));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_brewing_stand_progress_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 11,
            title: "Brewing Stand".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 0,
            value: 200,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 1,
            value: 10,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::BrewingStand,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::BrewingStandFuelLength,
                    60,
                    44,
                    9,
                    4,
                    [0.0, 0.0],
                    [9.0 / 18.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::BrewingStandBrewProgress,
                    97,
                    16,
                    9,
                    14,
                    [0.0, 0.0],
                    [1.0, 14.0 / 28.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::BrewingStandBubbles,
                    63,
                    23,
                    12,
                    20,
                    [0.0, 9.0 / 29.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_furnace_like_layouts() {
        for (menu_type_id, title, texture) in [
            (
                10,
                "Blast Furnace",
                HudInventoryBackgroundTexture::BlastFurnace,
            ),
            (14, "Furnace", HudInventoryBackgroundTexture::Furnace),
            (22, "Smoker", HudInventoryBackgroundTexture::Smoker),
        ] {
            let mut world = WorldStore::new();
            world.apply_open_screen(bbb_protocol::packets::OpenScreen {
                container_id: 7,
                menu_type_id,
                title: title.to_string(),
            });
            world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
                container_id: 7,
                state_id: 12,
                items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
                carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
            });

            let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

            assert_eq!(screen.width, 176);
            assert_eq!(screen.height, 166);
            assert_eq!(
                screen.background_layers,
                vec![hud_inventory_background_layer(
                    texture,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                )]
            );
            assert_eq!(screen.hovered_slot_id, Some(38));
            assert_eq!(screen.slots.len(), 39);
            let ingredient = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
            assert_eq!((ingredient.x, ingredient.y), (56, 17));
            let fuel = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
            assert_eq!((fuel.x, fuel.y), (56, 53));
            let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
            assert_eq!((result.x, result.y), (116, 35));
            let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
            assert_eq!((hotbar.x, hotbar.y), (152, 142));
        }
    }

    #[test]
    fn hud_inventory_screen_projects_furnace_progress_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 14,
            title: "Furnace".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 0,
            value: 50,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 1,
            value: 200,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 2,
            value: 25,
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 3,
            value: 100,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Furnace,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::FurnaceLitProgress,
                    56,
                    45,
                    14,
                    5,
                    [0.0, 9.0 / 14.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::FurnaceBurnProgress,
                    79,
                    34,
                    6,
                    16,
                    [0.0, 0.0],
                    [6.0 / 24.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_grindstone_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 15,
            title: "Grindstone".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Grindstone,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((input.x, input.y), (49, 19));
        let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((additional.x, additional.y), (49, 40));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (129, 34));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_grindstone_error_layer() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 15,
            title: "Grindstone".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Grindstone,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::GrindstoneError,
                    92,
                    31,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_hopper_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 16,
            title: "Hopper".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 41],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(40), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 133);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Hopper,
                0,
                0,
                176,
                133,
                [0.0, 0.0],
                [176.0 / 256.0, 133.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(40));
        assert_eq!(screen.slots.len(), 41);
        let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((first_container.x, first_container.y), (44, 20));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 40).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 109));
    }

    #[test]
    fn hud_inventory_screen_projects_mount_horse_layout() {
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(42, 66));
        world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });

        let screen = hud_inventory_screen(&world, None, Some(16), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Horse,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
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
                    HudInventoryBackgroundTexture::MountHorseArmorSlot,
                    8,
                    36,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::MountChestSlots,
                    79,
                    17,
                    90,
                    54,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(16));
        assert_eq!(screen.slots.len(), 53);
        let saddle = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((saddle.x, saddle.y), (8, 18));
        let last_mount_slot = screen.slots.iter().find(|slot| slot.slot_id == 16).unwrap();
        assert_eq!((last_mount_slot.x, last_mount_slot.y), (152, 54));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 52).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_mount_nautilus_slot_placeholders() {
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(42, 88));
        world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
            id: 42,
            values: vec![test_byte_data(18, 4)],
        });
        world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
            container_id: 7,
            inventory_columns: 5,
            entity_id: 42,
        });

        let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Nautilus,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
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
                    HudInventoryBackgroundTexture::MountNautilusArmorSlot,
                    8,
                    36,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.slots.len(), 38);
        let armor = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((armor.x, armor.y), (8, 36));
    }

    #[test]
    fn hud_inventory_screen_hides_inactive_mount_equipment_slot_layers() {
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(42, 36));
        world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Horse,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::MountChestSlots,
                    79,
                    17,
                    54,
                    54,
                    [0.0, 0.0],
                    [0.6, 1.0],
                ),
            ]
        );
        assert!(screen.slots.iter().all(|slot| slot.slot_id != 0));
        assert!(screen.slots.iter().all(|slot| slot.slot_id != 1));
        assert_eq!(screen.slots[0].slot_id, 2);
    }

    #[test]
    fn hud_inventory_screen_uses_mount_llama_armor_slot_placeholder() {
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(42, 78));
        world.apply_mount_screen_open(bbb_protocol::packets::MountScreenOpen {
            container_id: 7,
            inventory_columns: 3,
            entity_id: 42,
        });

        let screen = hud_inventory_screen(&world, None, Some(1), 0.0).unwrap();

        assert!(screen.background_layers.iter().any(|layer| layer.texture
            == HudInventoryBackgroundTexture::MountLlamaArmorSlot
            && (layer.x, layer.y, layer.width, layer.height) == (8, 36, 16, 16)));
    }

    #[test]
    fn hud_inventory_screen_projects_lectern_book_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 17,
            title: "Lectern".to_string(),
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(screen.width, 192);
        assert_eq!(screen.height, 192);
        assert!(screen.slots.is_empty());
        assert_eq!(
            screen.background_layers,
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
        );
    }

    #[test]
    fn hud_inventory_screen_projects_shulker_box_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 20,
            title: "Shulker Box".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 63],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(62), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 167);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::ShulkerBox,
                0,
                0,
                176,
                167,
                [0.0, 0.0],
                [176.0 / 256.0, 167.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(62));
        assert_eq!(screen.slots.len(), 63);
        let first_container = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((first_container.x, first_container.y), (8, 18));
        let last_container = screen.slots.iter().find(|slot| slot.slot_id == 26).unwrap();
        assert_eq!((last_container.x, last_container.y), (152, 54));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 62).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_loom_layout_and_empty_slot_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 18,
            title: "Loom".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Loom,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::LoomBannerSlot,
                    13,
                    26,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::LoomDyeSlot,
                    33,
                    26,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::LoomPatternSlot,
                    23,
                    45,
                    16,
                    16,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::LoomScrollerDisabled,
                    119,
                    13,
                    12,
                    15,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(39));
        assert_eq!(screen.slots.len(), 40);
        let banner = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((banner.x, banner.y), (13, 26));
        let dye = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((dye.x, dye.y), (33, 26));
        let pattern = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((pattern.x, pattern.y), (23, 45));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
        assert_eq!((result.x, result.y), (143, 57));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_loom_pattern_grid_and_scroller() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 18,
            title: "Loom".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 40];
        items[0] = item_stack(42, 1);
        items[1] = item_stack(43, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            None,
            InventoryHudLocalState {
                loom_pattern_scroll_row: Some(2),
                loom_selected_pattern_index: Some(10),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(screen.background_layers.len(), 19);
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomScroller,
                119,
                33,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomPattern,
                60,
                13,
                14,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomPatternSelected,
                88,
                13,
                14,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::LoomPattern,
                102,
                55,
                14,
                14,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
    }

    #[test]
    fn hud_inventory_screen_projects_merchant_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 19,
            title: "Merchant".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 276);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Villager,
                0,
                0,
                276,
                166,
                [0.0, 0.0],
                [276.0 / 512.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        assert!(screen.floating_items.is_empty());
        let payment_a = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((payment_a.x, payment_a.y), (136, 37));
        let payment_b = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((payment_b.x, payment_b.y), (162, 37));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (220, 37));
        let first_inventory = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
        assert_eq!((first_inventory.x, first_inventory.y), (108, 84));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (252, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_merchant_trade_layers() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 19,
            title: "Merchant".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        assert!(world.apply_merchant_offers(merchant_offers(7, 8, Some(2))));
        assert!(world.set_local_merchant_selected_offer(2));
        assert!(world.scroll_local_merchant_offers(1));

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 276);
        assert_eq!(screen.height, 166);
        assert!(screen.floating_items.is_empty());
        assert_eq!(
            screen.background_layers[0],
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Villager,
                0,
                0,
                276,
                166,
                [0.0, 0.0],
                [276.0 / 512.0, 166.0 / 256.0],
            )
        );
        assert_eq!(
            screen.background_layers[1],
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerTradeArrow,
                60,
                22,
                10,
                9,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        );
        assert_eq!(
            screen.background_layers[2],
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerTradeArrowOutOfStock,
                60,
                42,
                10,
                9,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        );
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerScroller,
                94,
                131,
                6,
                27,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerOutOfStock,
                182,
                35,
                28,
                21,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerExperienceBarBackground,
                136,
                16,
                102,
                5,
                [0.0, 0.0],
                [1.0, 1.0],
            )));
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::VillagerExperienceBarCurrent,
                136,
                16,
                63,
                5,
                [0.0, 0.0],
                [63.0 / 102.0, 1.0],
            )));
    }

    #[test]
    fn merchant_offer_cost_a_stack_uses_vanilla_modified_cost_count() {
        let mut world = WorldStore::new();
        world.set_default_item_max_stack_sizes(std::collections::BTreeMap::from([(42, 16)]));
        let offer = bbb_world::MerchantOfferState {
            buy_a: item_cost(42, 10),
            sell: item_stack(99, 1),
            buy_b: None,
            is_out_of_stock: false,
            uses: 0,
            max_uses: 12,
            xp: 8,
            special_price_diff: 5,
            price_multiplier: 0.5,
            demand: 2,
        };

        assert_eq!(
            merchant_offer_cost_a_stack(&world, &offer),
            item_stack(42, 16)
        );
    }

    #[test]
    fn hud_inventory_screen_projects_smithing_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 21,
            title: "Smithing".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(39), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::Smithing,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(39));
        assert_eq!(screen.slots.len(), 40);
        let template = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((template.x, template.y), (8, 48));
        let base = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((base.x, base.y), (26, 48));
        let additional = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((additional.x, additional.y), (44, 48));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 3).unwrap();
        assert_eq!((result.x, result.y), (98, 48));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 39).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_smithing_error_layer() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 21,
            title: "Smithing".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 40],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 0,
            value: 1,
        });

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Smithing,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::SmithingError,
                    65,
                    46,
                    28,
                    21,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_cartography_table_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 23,
            title: "Cartography Table".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 39],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(38), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![hud_inventory_background_layer(
                HudInventoryBackgroundTexture::CartographyTable,
                0,
                0,
                176,
                166,
                [0.0, 0.0],
                [176.0 / 256.0, 166.0 / 256.0],
            )]
        );
        assert_eq!(screen.hovered_slot_id, Some(38));
        assert_eq!(screen.slots.len(), 39);
        let map = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((map.x, map.y), (15, 15));
        let additional = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((additional.x, additional.y), (15, 52));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 2).unwrap();
        assert_eq!((result.x, result.y), (145, 39));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 38).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_cartography_table_map_frame() {
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
        items[0] = map_stack(100, 1, 42, None);
        let world = cartography_table_world_with_items(items);

        let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTable,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableMap,
                    67,
                    13,
                    66,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
    }

    #[test]
    fn hud_inventory_screen_projects_cartography_table_result_modes() {
        let cases = [
            (
                map_stack(
                    100,
                    1,
                    42,
                    Some(bbb_protocol::packets::MapPostProcessingSummary::Scale),
                ),
                vec![hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::CartographyTableScaledMap,
                    67,
                    13,
                    66,
                    66,
                    [0.0, 0.0],
                    [1.0, 1.0],
                )],
            ),
            (
                map_stack(100, 2, 42, None),
                vec![
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                        83,
                        13,
                        50,
                        66,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::CartographyTableDuplicatedMap,
                        67,
                        29,
                        50,
                        66,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ],
            ),
            (
                map_stack(
                    100,
                    1,
                    42,
                    Some(bbb_protocol::packets::MapPostProcessingSummary::Lock),
                ),
                vec![
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::CartographyTableMap,
                        67,
                        13,
                        66,
                        66,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                    hud_inventory_background_layer(
                        HudInventoryBackgroundTexture::CartographyTableLocked,
                        118,
                        60,
                        10,
                        14,
                        [0.0, 0.0],
                        [1.0, 1.0],
                    ),
                ],
            ),
        ];

        for (result_stack, expected_layers) in cases {
            let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 39];
            items[0] = map_stack(100, 1, 42, None);
            items[2] = result_stack;
            let world = cartography_table_world_with_items(items);

            let screen = hud_inventory_screen(&world, None, None, 0.0).unwrap();

            assert_eq!(&screen.background_layers[1..], expected_layers.as_slice());
        }
    }

    #[test]
    fn cartography_additional_item_resource_ids_match_vanilla_items() {
        assert_eq!(
            cartography_additional_item_for_resource_id("minecraft:paper"),
            Some(CartographyAdditionalItem::Paper)
        );
        assert_eq!(
            cartography_additional_item_for_resource_id("minecraft:map"),
            Some(CartographyAdditionalItem::EmptyMap)
        );
        assert_eq!(
            cartography_additional_item_for_resource_id("minecraft:glass_pane"),
            Some(CartographyAdditionalItem::GlassPane)
        );
        assert_eq!(
            cartography_additional_item_for_resource_id("minecraft:filled_map"),
            None
        );
    }

    #[test]
    fn hud_inventory_screen_projects_stonecutter_layout() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 24,
            title: "Stonecutter".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 38],
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });

        let screen = hud_inventory_screen(&world, None, Some(37), 0.0).unwrap();

        assert_eq!(screen.width, 176);
        assert_eq!(screen.height, 166);
        assert_eq!(
            screen.background_layers,
            vec![
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::Stonecutter,
                    0,
                    0,
                    176,
                    166,
                    [0.0, 0.0],
                    [176.0 / 256.0, 166.0 / 256.0],
                ),
                hud_inventory_background_layer(
                    HudInventoryBackgroundTexture::StonecutterScrollerDisabled,
                    119,
                    15,
                    12,
                    15,
                    [0.0, 0.0],
                    [1.0, 1.0],
                ),
            ]
        );
        assert_eq!(screen.hovered_slot_id, Some(37));
        assert_eq!(screen.slots.len(), 38);
        let input = screen.slots.iter().find(|slot| slot.slot_id == 0).unwrap();
        assert_eq!((input.x, input.y), (20, 33));
        let result = screen.slots.iter().find(|slot| slot.slot_id == 1).unwrap();
        assert_eq!((result.x, result.y), (143, 33));
        let hotbar = screen.slots.iter().find(|slot| slot.slot_id == 37).unwrap();
        assert_eq!((hotbar.x, hotbar.y), (152, 142));
    }

    #[test]
    fn hud_inventory_screen_projects_stonecutter_recipe_buttons_and_scroller() {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 24,
            title: "Stonecutter".to_string(),
        });
        let mut items = vec![bbb_protocol::packets::ItemStackSummary::empty(); 38];
        items[0] = item_stack(42, 1);
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world.apply_update_recipes(bbb_protocol::packets::UpdateRecipes {
            property_sets: Vec::new(),
            stonecutter_recipes: (0..16)
                .map(|index| stonecutter_recipe_display(42, 100 + index, 1))
                .collect(),
        });
        world.apply_container_set_data(bbb_protocol::packets::ContainerSetData {
            container_id: 7,
            id: 0,
            value: 5,
        });

        let screen = hud_inventory_screen_with_local_state(
            &world,
            None,
            None,
            InventoryHudLocalState {
                stonecutter_recipe_scroll_row: Some(1),
                ..InventoryHudLocalState::default()
            },
            0.0,
        )
        .unwrap();

        assert_eq!(
            screen.background_layers[1],
            hud_inventory_background_layer(
                HudInventoryBackgroundTexture::StonecutterScroller,
                119,
                56,
                12,
                15,
                [0.0, 0.0],
                [1.0, 1.0],
            )
        );
        let recipe_layers: Vec<_> = screen
            .background_layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::StonecutterRecipe
                        | HudInventoryBackgroundTexture::StonecutterRecipeSelected
                )
            })
            .collect();
        assert_eq!(recipe_layers.len(), 12);
        assert!(screen
            .background_layers
            .contains(&hud_inventory_background_layer(
                HudInventoryBackgroundTexture::StonecutterRecipeSelected,
                68,
                15,
                16,
                18,
                [0.0, 0.0],
                [1.0, 1.0],
            )));

        let option_stacks = stonecutter_visible_recipe_option_stacks(&world, 1);
        assert_eq!(option_stacks.len(), 12);
        assert_eq!(
            option_stacks[0],
            StonecutterRecipeOptionStack {
                x: 52,
                y: 16,
                stack: item_stack(104, 1),
            }
        );
        assert_eq!(
            option_stacks[11],
            StonecutterRecipeOptionStack {
                x: 100,
                y: 52,
                stack: item_stack(115, 1),
            }
        );
    }

    #[test]
    fn stonecutter_slot_display_item_stack_projects_direct_item_displays() {
        assert_eq!(
            stonecutter_slot_display_item_stack(&stonecutter_item_display(77)),
            Some(item_stack(77, 1))
        );
        assert_eq!(
            stonecutter_slot_display_item_stack(&stonecutter_item_stack_display(78, 3)),
            Some(item_stack(78, 3))
        );
        assert_eq!(
            stonecutter_slot_display_item_stack(&bbb_protocol::packets::SlotDisplaySummary {
                display_type_id: 6,
                raw_payload: vec![6, 4, b't', b'e', b's', b't'],
            }),
            None
        );
    }

    #[test]
    fn hud_item_count_label_follows_vanilla_stack_count_rule() {
        assert_eq!(
            hud_item_count_label_for_stack(&item_stack(42, 64)),
            Some(HudItemCountLabel::new("64"))
        );
        assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 1)), None);
        assert_eq!(hud_item_count_label_for_stack(&item_stack(42, 0)), None);
        assert_eq!(
            hud_item_count_label_for_stack(&bbb_protocol::packets::ItemStackSummary::empty()),
            None
        );
    }

    #[test]
    fn hud_item_cooldown_progress_uses_world_cooldown_group_state() {
        let mut world = WorldStore::new();
        world.apply_cooldown(bbb_protocol::packets::Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        });
        world.advance_item_cooldowns(5);
        let mut stack = item_stack(42, 1);
        stack.component_patch.use_cooldown_group = Some("minecraft:ender_pearl".to_string());

        assert_eq!(
            hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.5),
            Some(0.725)
        );
        assert_eq!(
            hud_item_cooldown_progress_for_stack(&world, None, &stack, 1.5),
            Some(0.7)
        );

        stack.component_patch.use_cooldown_group = Some("minecraft:wind_charge".to_string());
        assert_eq!(
            hud_item_cooldown_progress_for_stack(&world, None, &stack, 0.0),
            None
        );
        assert_eq!(
            hud_item_cooldown_progress_for_stack(
                &world,
                None,
                &bbb_protocol::packets::ItemStackSummary::empty(),
                0.0
            ),
            None
        );
    }

    #[test]
    fn item_cooldown_group_requires_runtime_for_default_item_group() {
        let stack = item_stack(42, 1);
        assert_eq!(item_cooldown_group(None, &stack), None);

        let mut explicit_group = stack;
        explicit_group.component_patch.use_cooldown_group =
            Some("minecraft:custom_group".to_string());
        assert_eq!(
            item_cooldown_group(None, &explicit_group),
            Some("minecraft:custom_group".to_string())
        );
    }

    #[test]
    fn hud_item_durability_bar_follows_vanilla_damage_formula() {
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 25, false)),
            Some(HudItemDurabilityBar::new(10, [127.0 / 255.0, 1.0, 0.0]))
        );
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 100, false)),
            Some(HudItemDurabilityBar::new(0, [1.0, 0.0, 0.0]))
        );
    }

    #[test]
    fn hud_item_durability_bar_requires_damageable_damaged_stack() {
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 0, false)),
            None
        );
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, -5, false)),
            None
        );
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 1, 100, 25, true)),
            None
        );
        assert_eq!(
            hud_item_durability_bar_for_stack(&item_stack_with_damage(42, 0, 100, 25, false)),
            None
        );
        let mut missing_damage = item_stack(42, 1);
        missing_damage.component_patch.max_damage = Some(100);
        assert_eq!(hud_item_durability_bar_for_stack(&missing_damage), None);

        let mut missing_max_damage = item_stack(42, 1);
        missing_max_damage.component_patch.damage = Some(25);
        assert_eq!(hud_item_durability_bar_for_stack(&missing_max_damage), None);

        let mut non_damageable = item_stack_with_damage(42, 1, 0, 25, false);
        assert_eq!(hud_item_durability_bar_for_stack(&non_damageable), None);
        non_damageable.component_patch.max_damage = Some(-1);
        assert_eq!(hud_item_durability_bar_for_stack(&non_damageable), None);
    }

    #[test]
    fn block_destroy_overlays_include_server_progress_and_keep_highest_per_position() {
        let mut world = WorldStore::new();
        let textures = destroy_stage_test_textures();
        let pos = bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 };
        assert!(
            world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
                id: 10,
                pos,
                progress: 2,
            })
        );
        assert!(
            world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
                id: 11,
                pos,
                progress: 7,
            })
        );
        assert!(
            world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
                id: 12,
                pos: bbb_protocol::packets::BlockPos { x: 1, y: 3, z: 4 },
                progress: 3,
            })
        );

        let overlays = block_destroy_overlays_from_world(&world, &textures);

        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0].pos, [1, 3, 4]);
        assert_eq!(overlays[0].uv, textures.destroy_stage_uv_rect(3).unwrap());
        assert_eq!(overlays[1].pos, [2, 3, 4]);
        assert_eq!(overlays[1].uv, textures.destroy_stage_uv_rect(7).unwrap());
    }

    #[test]
    fn block_destroy_overlays_skip_missing_destroy_stage_textures() {
        let mut world = WorldStore::new();
        let textures = TerrainTextureState::default();
        assert!(
            world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
                id: 10,
                pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
                progress: 2,
            })
        );

        assert!(block_destroy_overlays_from_world(&world, &textures).is_empty());
    }

    #[test]
    fn block_destroy_render_ticks_respect_frozen_world_ticking_state() {
        let mut world = WorldStore::new();
        assert!(
            world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
                id: 10,
                pos: bbb_protocol::packets::BlockPos { x: 2, y: 3, z: 4 },
                progress: 2,
            })
        );
        world.apply_ticking_state(bbb_protocol::packets::TickingState {
            tick_rate: 20.0,
            frozen: true,
        });

        assert_eq!(advance_block_destruction_render_ticks(&mut world, 420), 0);
        assert_eq!(world.block_destructions().len(), 1);

        world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 420 });
        assert_eq!(advance_block_destruction_render_ticks(&mut world, 420), 1);
        assert!(world.block_destructions().is_empty());
        assert_eq!(world.ticking().frozen_ticks_to_run, 0);
    }

    #[test]
    fn entity_client_animations_advance_at_vanilla_tick_interval() {
        let start = Instant::now();
        let mut ticks = ClientAnimationTickState::default();
        let mut world = WorldStore::new();
        world.apply_add_entity(test_add_entity(123, 104));
        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 123,
                values: vec![test_bool_data(18, true)],
            })
        );

        assert_eq!(
            world.probe_entity_pick_bounds(123),
            Some(bbb_world::EntityPickBoundsState::from_base_size(
                1.4, 1.4, 0.0
            ))
        );

        assert_eq!(
            advance_entity_client_animations(&mut world, &mut ticks, start),
            0
        );
        assert_eq!(
            advance_entity_client_animations(
                &mut world,
                &mut ticks,
                start + Duration::from_millis(49),
            ),
            0
        );
        assert_eq!(
            world.probe_entity_pick_bounds(123),
            Some(bbb_world::EntityPickBoundsState::from_base_size(
                1.4, 1.4, 0.0
            ))
        );

        assert_eq!(
            advance_entity_client_animations(
                &mut world,
                &mut ticks,
                start + Duration::from_millis(50),
            ),
            1
        );
        assert_eq!(
            advance_entity_client_animations(
                &mut world,
                &mut ticks,
                start + Duration::from_millis(50),
            ),
            0
        );
        assert_eq!(
            world.probe_entity_pick_bounds(123),
            Some(bbb_world::EntityPickBoundsState::from_base_size(
                1.4, 1.4, 0.0
            ))
        );

        assert_eq!(
            advance_entity_client_animations(
                &mut world,
                &mut ticks,
                start + Duration::from_millis(100),
            ),
            1
        );
        assert_eq!(
            world.probe_entity_pick_bounds(123),
            Some(bbb_world::EntityPickBoundsState::from_base_size(
                1.4,
                1.4 * (1.0 + 1.0 / 6.0),
                0.0,
            ))
        );

        assert_eq!(
            advance_entity_client_animations(
                &mut world,
                &mut ticks,
                start + Duration::from_millis(350),
            ),
            5
        );
        assert_eq!(
            world.probe_entity_pick_bounds(123),
            Some(bbb_world::EntityPickBoundsState::from_base_size(
                1.4, 2.8, 0.0
            ))
        );
    }

    #[test]
    fn publish_snapshot_includes_audio_runtime_counters() {
        let snapshot = bbb_control::shared_snapshot("test");
        let audio = AudioCounters {
            enabled: true,
            catalog_events: 1902,
            registry_entries: 1902,
            commands_submitted: 3,
            submit_failures: 1,
            last_submit_error: Some("failed to submit audio command".to_string()),
            ..AudioCounters::default()
        };
        let net = NetCounters::default();
        let world = WorldStore::new();

        assert!(publish_snapshot(
            &snapshot,
            RendererCounters::default(),
            &net,
            &audio,
            &world,
        ));

        assert_eq!(snapshot.read().unwrap().audio, audio);
    }

    fn local_player_pose(position: [f64; 3], y_rot: f32, x_rot: f32) -> LocalPlayerPoseState {
        LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            y_rot,
            x_rot,
            ..LocalPlayerPoseState::default()
        }
    }

    fn destroy_stage_test_textures() -> TerrainTextureState {
        let images = (0..10)
            .map(|stage| {
                bbb_pack::SpriteImage::new(
                    format!("minecraft:block/destroy_stage_{stage}"),
                    1,
                    1,
                    vec![255, 255, 255, 255],
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let atlas = bbb_pack::AtlasPacker::new(16, 1)
            .unwrap()
            .stitch(&images)
            .unwrap();
        TerrainTextureState::from_layout(&atlas.layout, None, None, None)
    }

    fn test_add_entity(id: i32, entity_type_id: i32) -> bbb_protocol::packets::AddEntity {
        bbb_protocol::packets::AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(id as u128),
            entity_type_id,
            position: bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            delta_movement: bbb_protocol::packets::Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn test_bool_data(data_id: u8, value: bool) -> bbb_protocol::packets::EntityDataValue {
        bbb_protocol::packets::EntityDataValue {
            data_id,
            serializer_id: 8,
            value: bbb_protocol::packets::EntityDataValueKind::Boolean(value),
        }
    }

    fn test_byte_data(data_id: u8, value: i8) -> bbb_protocol::packets::EntityDataValue {
        bbb_protocol::packets::EntityDataValue {
            data_id,
            serializer_id: 0,
            value: bbb_protocol::packets::EntityDataValueKind::Byte(value),
        }
    }

    fn merchant_offers(
        container_id: i32,
        offer_count: usize,
        out_of_stock_index: Option<usize>,
    ) -> MerchantOffers {
        MerchantOffers {
            container_id,
            offers: (0..offer_count)
                .map(|index| MerchantOffer {
                    buy_a: item_cost(42 + index as i32, 3),
                    sell: item_stack(99 + index as i32, 1),
                    buy_b: (index % 2 == 0).then(|| item_cost(52 + index as i32, 2)),
                    is_out_of_stock: out_of_stock_index == Some(index),
                    uses: if out_of_stock_index == Some(index) {
                        12
                    } else {
                        1
                    },
                    max_uses: 12,
                    xp: 8,
                    special_price_diff: -2,
                    price_multiplier: 0.05,
                    demand: 6,
                })
                .collect(),
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: true,
        }
    }

    fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
        ItemCostSummary {
            item_id,
            count,
            component_predicate: Default::default(),
        }
    }

    fn stonecutter_recipe_display(
        input_item_id: i32,
        result_item_id: i32,
        result_count: i32,
    ) -> bbb_protocol::packets::StonecutterSelectableRecipeSummary {
        bbb_protocol::packets::StonecutterSelectableRecipeSummary {
            input: bbb_protocol::packets::IngredientSummary {
                tag: None,
                item_ids: vec![input_item_id],
            },
            option_display: stonecutter_item_stack_display(result_item_id, result_count),
        }
    }

    fn stonecutter_item_display(item_id: i32) -> bbb_protocol::packets::SlotDisplaySummary {
        let mut raw_payload = Vec::new();
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 4);
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
        bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: 4,
            raw_payload,
        }
    }

    fn stonecutter_item_stack_display(
        item_id: i32,
        count: i32,
    ) -> bbb_protocol::packets::SlotDisplaySummary {
        let mut raw_payload = Vec::new();
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 5);
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, item_id);
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, count);
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
        bbb_protocol::codec::write_var_i32_to(&mut raw_payload, 0);
        bbb_protocol::packets::SlotDisplaySummary {
            display_type_id: 5,
            raw_payload,
        }
    }

    fn cartography_table_world_with_items(
        items: Vec<bbb_protocol::packets::ItemStackSummary>,
    ) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 7,
            menu_type_id: 23,
            title: "Cartography Table".to_string(),
        });
        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 7,
            state_id: 12,
            items,
            carried_item: bbb_protocol::packets::ItemStackSummary::empty(),
        });
        world
    }

    fn map_stack(
        item_id: i32,
        count: i32,
        map_id: i32,
        post_processing: Option<bbb_protocol::packets::MapPostProcessingSummary>,
    ) -> bbb_protocol::packets::ItemStackSummary {
        let mut item = item_stack(item_id, count);
        item.component_patch.map_id = Some(map_id);
        item.component_patch
            .added_type_ids
            .push(MAP_ID_DATA_COMPONENT_TYPE_ID);
        if let Some(post_processing) = post_processing {
            item.component_patch.map_post_processing = Some(post_processing);
            item.component_patch.added_type_ids.push(48);
        }
        item
    }

    fn item_stack(item_id: i32, count: i32) -> bbb_protocol::packets::ItemStackSummary {
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn item_stack_with_damage(
        item_id: i32,
        count: i32,
        max_damage: i32,
        damage: i32,
        unbreakable: bool,
    ) -> bbb_protocol::packets::ItemStackSummary {
        let mut item = item_stack(item_id, count);
        item.component_patch.max_damage = Some(max_damage);
        item.component_patch.damage = Some(damage);
        item.component_patch.unbreakable = unbreakable;
        item
    }

    fn write_runtime_tooltip_item_assets(root: &Path) {
        let assets = runtime_assets_dir(root);
        write_runtime_json(
            &assets.join("atlases").join("items.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "item/",
                        "source": "item"
                    }
                ]
            }"#,
        );
        write_runtime_json(
            &assets.join("atlases").join("blocks.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "block/",
                        "source": "block"
                    }
                ]
            }"#,
        );
        write_runtime_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_combo"
                }
            }"#,
        );
        write_runtime_json(
            &assets.join("models").join("item").join("test_combo.json"),
            r#"{
                "textures": {
                    "layer0": "minecraft:item/test_combo"
                }
            }"#,
        );
        write_runtime_json(
            &assets.join("lang").join("en_us.json"),
            r#"{
                "item.minecraft.test_combo": "Test Combo"
            }"#,
        );
        write_runtime_png(
            &assets.join("textures").join("item").join("test_combo.png"),
            &[80, 120, 160, 255],
        );
        write_runtime_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            r#"public class Items {
                public static final Item TEST_COMBO = registerItem("test_combo");
            }"#,
        );
    }

    fn runtime_assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn write_runtime_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_runtime_png(path: &Path, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, rgba, 1, 1, image::ColorType::Rgba8).unwrap();
    }

    fn unique_runtime_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-native-runtime-{label}-{nanos}"))
    }
}
