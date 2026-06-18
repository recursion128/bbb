use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{AudioCounters, NetCounters, RendererCounters, SharedSnapshot};
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{
    BlockDestroyOverlay, CameraPose, ClearColor, HudIconLayer, HudInventoryBackgroundLayer,
    HudInventoryBackgroundTexture, HudInventoryScreen, HudInventorySlot, HudItemCountLabel,
    HudItemIcon, HudUvRect, HUD_HOTBAR_SLOTS,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    camera_pose::camera_pose_from_world,
    code_of_conduct::CodeOfConductAcceptance,
    crosshair::{entity_target_outline_from_camera_at_partial_tick, selection_outline_from_camera},
    entity_scene::entity_scene_outline_from_world_at_partial_tick,
    input::{
        advance_destroying_block_at_partial_tick, advance_player_input,
        advance_using_item_at_partial_tick, inventory_screen_layout, ClientInputState,
        InventoryScreenBackground,
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
const FURNACE_LIT_TIME_DATA_ID: i16 = 0;
const FURNACE_LIT_DURATION_DATA_ID: i16 = 1;
const FURNACE_COOKING_PROGRESS_DATA_ID: i16 = 2;
const FURNACE_COOKING_TOTAL_TIME_DATA_ID: i16 = 3;
const FURNACE_DEFAULT_LIT_DURATION: i16 = 200;
const FURNACE_LIT_PROGRESS_SPRITE_SIZE: u32 = 14;
const FURNACE_BURN_PROGRESS_SPRITE_WIDTH: u32 = 24;
const FURNACE_BURN_PROGRESS_SPRITE_HEIGHT: u32 = 16;

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
    renderer.set_hud_hotbar_item_icons(hotbar_item_icons(world, item_runtime));
    renderer.set_hud_inventory_screen(hud_inventory_screen(
        world,
        item_runtime,
        input.inventory_hovered_slot(),
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
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    let mut icons = std::array::from_fn(|_| None);
    for (slot_index, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        icons[slot_index] = hud_item_icon_for_stack(
            item_runtime,
            &item.item,
            item.local_selected_bundle_item_index(),
        );
    }

    icons
}

fn hud_inventory_screen(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_slot_id: Option<i16>,
) -> Option<HudInventoryScreen> {
    let layout = inventory_screen_layout(world)?;
    let container = if world.local_inventory_is_open() {
        &world.inventory().inventory_menu
    } else {
        world.inventory().open_container.as_ref()?
    };

    let slots = layout
        .slots
        .into_iter()
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
                        item_runtime,
                        &slot.item,
                        (slot.local_selected_bundle_item_index >= 0)
                            .then_some(slot.local_selected_bundle_item_index),
                    )
                }),
            }
        })
        .collect();

    Some(HudInventoryScreen {
        width: u32::try_from(layout.width).unwrap_or_default(),
        height: u32::try_from(layout.height).unwrap_or_default(),
        background_layers: hud_inventory_background_layers(world, layout.background),
        slots,
        hovered_slot_id: hovered_slot_id.and_then(|slot| u16::try_from(slot).ok()),
    })
}

fn hud_inventory_background_layers(
    world: &WorldStore,
    background: InventoryScreenBackground,
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
    }
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
    item_runtime: Option<&NativeItemRuntime>,
    item: &bbb_protocol::packets::ItemStackSummary,
    local_selected_bundle_item_index: Option<i32>,
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
        assert_eq!(hud_inventory_screen(&world, None, Some(36)), None);

        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: 0,
            item: bbb_protocol::packets::ItemStackSummary {
                item_id: Some(42),
                count: 3,
                component_patch: Default::default(),
            },
        });
        assert!(world.open_local_inventory());

        let screen = hud_inventory_screen(&world, None, Some(36)).unwrap();

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

        let screen = hud_inventory_screen(&world, None, Some(89)).unwrap();

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

        let screen = hud_inventory_screen(&world, None, Some(44)).unwrap();

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

            let screen = hud_inventory_screen(&world, None, Some(38)).unwrap();

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

        let screen = hud_inventory_screen(&world, None, None).unwrap();

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

        let screen = hud_inventory_screen(&world, None, Some(40)).unwrap();

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

        let screen = hud_inventory_screen(&world, None, Some(62)).unwrap();

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

    fn item_stack(item_id: i32, count: i32) -> bbb_protocol::packets::ItemStackSummary {
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }
}
