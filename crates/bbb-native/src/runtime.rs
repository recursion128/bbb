use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{AudioCounters, NetCounters, RendererCounters, SharedSnapshot};
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{
    CameraPose, ClearColor, HudIconLayer, HudItemIcon, HudUvRect, HUD_HOTBAR_SLOTS,
};
use bbb_world::WorldStore;
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    camera_pose::camera_pose_from_world,
    code_of_conduct::CodeOfConductAcceptance,
    crosshair::selection_outline_from_camera,
    input::{advance_destroying_block_at_partial_tick, advance_player_input, ClientInputState},
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
    renderer.advance_particles(advanced_ticks);
    advance_player_input(input, world, net_counters, net_commands, now);
    advance_destroying_block_at_partial_tick(
        input,
        world,
        net_counters,
        net_commands,
        client_animation_ticks.entity_partial_tick(now),
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
    let camera_pose = camera_pose_from_world(world);
    renderer.set_camera_pose(camera_pose);
    renderer.set_selection_outline(selection_outline_from_camera(world, camera_pose));
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

fn hotbar_item_icons(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> [Option<HudItemIcon>; HUD_HOTBAR_SLOTS] {
    let mut icons = std::array::from_fn(|_| None);
    let Some(item_runtime) = item_runtime else {
        return icons;
    };

    for (slot_index, item) in world.inventory().hotbar_item_states().iter().enumerate() {
        let Some(icon) = item_runtime.icon_for_stack_with_bundle_selected_item(
            &item.item,
            item.local_selected_bundle_item_index(),
        ) else {
            continue;
        };
        icons[slot_index] = Some(HudItemIcon {
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
        });
    }

    icons
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
}
