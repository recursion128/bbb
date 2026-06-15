use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bbb_audio::{AudioListenerState, EntitySoundPosition, TickEntitySoundPositionsCommand};
use bbb_control::{
    CodeOfConductControlRequest, NetCounters, PlayerPose, RendererCounters, SharedSnapshot,
};
use bbb_net::{NetCommand, NetEvent};
use bbb_renderer::{CameraPose, ClearColor};
use bbb_world::WorldStore;
use tokio::sync::mpsc;

use crate::{
    audio_runtime::AudioEventSink,
    code_of_conduct::CodeOfConductAcceptance,
    crosshair::selection_outline_from_crosshair,
    input::{advance_player_input, ClientInputState},
    particle_runtime::ParticleEventSink,
    terrain_runtime::{
        maybe_upload_decoded_terrain, maybe_upload_terrain_texture_animation, TerrainTextureState,
        TerrainUploadState,
    },
};

mod events;

const CLIENT_ENTITY_ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) use events::{
    local_player_pose_from_player_pose, player_pose_from_local_player_pose,
    player_position_state_from_local_player_pose,
};

#[derive(Debug, Default)]
pub(crate) struct ClientAnimationTickState {
    last_entity_animation_at: Option<Instant>,
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

pub(crate) fn pump_control_net_requests(
    snapshot: &SharedSnapshot,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    world: &WorldStore,
    code_of_conduct: Option<&mut CodeOfConductAcceptance>,
) {
    let requests = snapshot
        .write()
        .map(|mut guard| std::mem::take(&mut guard.code_of_conduct_requests))
        .unwrap_or_default();

    let mut code_of_conduct = code_of_conduct;
    for request in requests {
        match request {
            CodeOfConductControlRequest::Accept { remember } => {
                let Some(tx) = net_commands else {
                    continue;
                };
                if tx.try_send(NetCommand::AcceptCodeOfConduct).is_err() {
                    break;
                }
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    let result = if remember {
                        code_of_conduct.persist_current_world_acceptance(world)
                    } else {
                        code_of_conduct.clear_connected_server_acceptance()
                    };
                    if let Err(err) = result {
                        tracing::warn!(
                            ?err,
                            remember,
                            "failed to update code-of-conduct acceptance store"
                        );
                    }
                }
            }
            CodeOfConductControlRequest::Decline => {
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                        tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                    }
                }
                if let Some(tx) = net_commands {
                    if tx.try_send(NetCommand::Disconnect).is_err() {
                        break;
                    }
                }
            }
            CodeOfConductControlRequest::ClearAcceptance => {
                if let Some(code_of_conduct) = code_of_conduct.as_deref_mut() {
                    if let Err(err) = code_of_conduct.clear_connected_server_acceptance() {
                        tracing::warn!(?err, "failed to clear code-of-conduct acceptance");
                    }
                }
            }
        }
    }
}

pub(crate) fn pump_network_and_terrain(
    net_events: &mut Option<mpsc::Receiver<NetEvent>>,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    audio_events: Option<&mut dyn AudioEventSink>,
    particle_events: Option<&mut dyn ParticleEventSink>,
    input: &mut ClientInputState,
    world: &mut WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    net_counters: &mut NetCounters,
    client_animation_ticks: &mut ClientAnimationTickState,
    terrain_upload: &mut TerrainUploadState,
    terrain_textures: &TerrainTextureState,
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
    pump_control_net_requests(snapshot, net_commands, world, code_of_conduct);
    let now = Instant::now();
    let advanced_ticks = advance_entity_client_animations(world, client_animation_ticks, now);
    renderer.advance_particles(advanced_ticks);
    advance_player_input(input, world, net_counters, net_commands, now);
    let local_player = world.local_player();
    let player_pose = local_player.pose.map(player_pose_from_local_player_pose);
    renderer.set_hud_health(local_player.health.map(|health| health.health));
    renderer.set_hud_food(local_player.health.map(|health| health.food));
    renderer.set_hud_experience_progress(
        local_player
            .experience
            .map(|experience| experience.progress),
    );
    renderer.set_hud_selected_slot(local_player.selected_hotbar_slot);
    renderer.set_camera_pose(player_pose.map(camera_pose_from_player));
    renderer.set_selection_outline(selection_outline_from_crosshair(world, player_pose));
    maybe_upload_terrain_texture_animation(renderer, terrain_upload, terrain_textures);
    maybe_upload_decoded_terrain(world, renderer, terrain_upload, terrain_textures);
    if let Some(audio_events) = audio_events.as_mut() {
        audio_events.tick_entity_sound_positions(audio_scene_command_from_world(world));
    }
    publish_snapshot(snapshot, renderer.counters(), net_counters, world)
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

fn camera_pose_from_player(player: PlayerPose) -> CameraPose {
    CameraPose {
        position: [
            player.position.x as f32,
            player.position.y as f32,
            player.position.z as f32,
        ],
        y_rot: player.y_rot,
        x_rot: player.x_rot,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    }
}

fn audio_scene_command_from_world(world: &WorldStore) -> TickEntitySoundPositionsCommand {
    TickEntitySoundPositionsCommand {
        listener: world.local_player_pose().map(|pose| AudioListenerState {
            position: [
                pose.position.x,
                pose.position.y + f64::from(CameraPose::STANDING_EYE_HEIGHT),
                pose.position.z,
            ],
            y_rot: pose.y_rot,
            x_rot: pose.x_rot,
        }),
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

pub(crate) fn publish_snapshot(
    snapshot: &SharedSnapshot,
    renderer: RendererCounters,
    net: &NetCounters,
    world: &WorldStore,
) -> bool {
    if let Ok(mut guard) = snapshot.write() {
        guard.renderer = renderer;
        guard.net = net.clone();
        guard.world = world.counters();
        guard.world_store = world.clone();
        guard.app.running
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_pose_uses_standing_eye_height() {
        let pose = camera_pose_from_player(PlayerPose {
            position: bbb_control::NetVec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            y_rot: 45.0,
            x_rot: -10.0,
            ..PlayerPose::default()
        });

        assert_eq!(pose.position, [1.0, 2.0, 3.0]);
        assert_eq!(pose.y_rot, 45.0);
        assert_eq!(pose.x_rot, -10.0);
        assert_eq!(pose.eye_height, CameraPose::STANDING_EYE_HEIGHT);
    }

    #[test]
    fn pump_control_net_requests_queues_code_of_conduct_accept_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let world = WorldStore::new();

        pump_control_net_requests(&snapshot, &Some(tx), &world, None);

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        assert!(snapshot.read().unwrap().code_of_conduct_requests.is_empty());
    }

    #[test]
    fn pump_control_net_requests_persists_current_code_of_conduct_hash() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: true });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let text = "Keep the server friendly.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());
        acceptance.set_connected_server(&options);

        pump_control_net_requests(&snapshot, &Some(tx.clone()), &world, Some(&mut acceptance));

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(
            loaded.accepted_hash_for_options(&options),
            Some(bbb_world::code_of_conduct_text_hash(text))
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_non_persistent_accept_clears_existing_hash() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Accept { remember: false });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();

        pump_control_net_requests(&snapshot, &Some(tx.clone()), &world, Some(&mut acceptance));

        assert_eq!(rx.try_recv().unwrap(), NetCommand::AcceptCodeOfConduct);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_decline_clears_hash_and_disconnects() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::Decline);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();

        pump_control_net_requests(&snapshot, &Some(tx.clone()), &world, Some(&mut acceptance));

        assert_eq!(rx.try_recv().unwrap(), NetCommand::Disconnect);
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn pump_control_net_requests_clear_acceptance_does_not_send_accept_command() {
        let snapshot = bbb_control::shared_snapshot("test");
        snapshot
            .write()
            .unwrap()
            .code_of_conduct_requests
            .push(CodeOfConductControlRequest::ClearAcceptance);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let path = unique_code_of_conduct_store_path();
        let mut acceptance = CodeOfConductAcceptance::load(&path).unwrap();
        let options = bbb_net::ConnectionOptions::offline("example.org:25565", "bbb").unwrap();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep the server friendly.".to_string());
        acceptance.set_connected_server(&options);
        acceptance.persist_current_world_acceptance(&world).unwrap();

        pump_control_net_requests(&snapshot, &Some(tx.clone()), &world, Some(&mut acceptance));

        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        let loaded = CodeOfConductAcceptance::load(&path).unwrap();
        assert_eq!(loaded.accepted_hash_for_options(&options), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn audio_scene_command_tracks_listener_and_entity_positions() {
        let mut world = WorldStore::new();
        world.set_local_player_pose(local_player_pose_from_player_pose(PlayerPose {
            position: bbb_control::NetVec3 {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            y_rot: 90.0,
            x_rot: -10.0,
            ..PlayerPose::default()
        }));
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

    fn unique_code_of_conduct_store_path() -> std::path::PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-runtime-code-of-conduct-{}-{id}-{nanos}.json",
            std::process::id()
        ))
    }
}
