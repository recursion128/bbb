use std::time::{Duration, Instant};

use bbb_net::NetCommand;
use bbb_world::{WorldLevelLoadPhase, WorldStore};
use tokio::sync::mpsc;

const CLIENT_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub(crate) struct LevelLoadRuntime {
    observed_load: Option<(u64, usize)>,
    timeout_after: Option<Instant>,
}

impl LevelLoadRuntime {
    pub(crate) fn tick(
        &mut self,
        now: Instant,
        world: &mut WorldStore,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        player_section_compiled_and_visible: bool,
    ) {
        self.synchronize_generation(now, world);
        if world.level_load().phase != WorldLevelLoadPhase::WaitingForPlayerChunk {
            return;
        }

        let timed_out = self.timeout_after.is_some_and(|timeout| now > timeout);
        let player_shortcut_ready = world.local_player_is_spectator()
            || world.local_player_is_dead()
            || world
                .local_player_pose()
                .is_some_and(|pose| !world.dimension().contains_y(pose.position.y.floor() as i32));
        if !timed_out && !player_shortcut_ready && !player_section_compiled_and_visible {
            return;
        }

        let Some(tx) = net_commands else {
            return;
        };
        if tx.try_send(NetCommand::PlayerLoaded).is_err() {
            return;
        }
        world.complete_client_level_load(timed_out);
        self.timeout_after = None;
    }

    fn synchronize_generation(&mut self, now: Instant, world: &WorldStore) {
        // `handleRespawn` starts a fresh LevelLoadTracker even when it keeps
        // the same ClientLevel. Pair the exact level identity with the
        // canonical load-start sequence instead of overloading the level
        // generation with LocalPlayer respawn lifetime.
        let load = (world.client_level_generation(), world.level_load().starts);
        if self.observed_load == Some(load) {
            return;
        }
        self.observed_load = Some(load);
        self.timeout_after = matches!(
            world.level_load().phase,
            WorldLevelLoadPhase::WaitingForServer | WorldLevelLoadPhase::WaitingForPlayerChunk
        )
        .then(|| now + CLIENT_WAIT_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        packets::{CommonPlayerSpawnInfo, GameEvent, PlayLogin, PlayerHealth, Respawn, Vec3d},
    };
    use bbb_world::LocalPlayerPoseState;

    #[test]
    fn player_loaded_waits_for_server_and_player_section_then_sends_once() {
        let start = Instant::now();
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut world = loading_world(0, 64.0);
        let mut runtime = LevelLoadRuntime::default();

        runtime.tick(start, &mut world, &commands, true);
        assert!(rx.try_recv().is_err());

        notify_loading_packets(&mut world);
        runtime.tick(start, &mut world, &commands, false);
        assert!(rx.try_recv().is_err());

        let (closed_tx, closed_rx) = mpsc::channel(1);
        drop(closed_rx);
        runtime.tick(start, &mut world, &Some(closed_tx), true);
        assert_eq!(
            world.level_load().phase,
            WorldLevelLoadPhase::WaitingForPlayerChunk
        );

        runtime.tick(start, &mut world, &commands, true);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::PlayerLoaded);
        assert_eq!(world.level_load().phase, WorldLevelLoadPhase::ClientLoaded);
        assert_eq!(world.level_load().loads_completed, 1);

        runtime.tick(start, &mut world, &commands, true);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn player_loaded_uses_vanilla_shortcuts_and_strict_timeout() {
        for (game_type, y, dead) in [(3, 64.0, false), (0, 64.0, true), (0, 400.0, false)] {
            let start = Instant::now();
            let (tx, mut rx) = mpsc::channel(1);
            let commands = Some(tx);
            let mut world = loading_world(game_type, y);
            if dead {
                world.apply_player_health(PlayerHealth {
                    health: 0.0,
                    food: 20,
                    saturation: 5.0,
                });
            }
            notify_loading_packets(&mut world);
            let mut runtime = LevelLoadRuntime::default();

            runtime.tick(start, &mut world, &commands, false);
            assert_eq!(rx.try_recv().unwrap(), NetCommand::PlayerLoaded);
            assert_eq!(world.level_load().timeouts, 0);
        }

        let start = Instant::now();
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let mut world = loading_world(0, 64.0);
        let mut runtime = LevelLoadRuntime::default();
        runtime.tick(start, &mut world, &commands, false);
        notify_loading_packets(&mut world);

        runtime.tick(start + CLIENT_WAIT_TIMEOUT, &mut world, &commands, false);
        assert!(rx.try_recv().is_err());
        runtime.tick(
            start + CLIENT_WAIT_TIMEOUT + Duration::from_millis(1),
            &mut world,
            &commands,
            false,
        );
        assert_eq!(rx.try_recv().unwrap(), NetCommand::PlayerLoaded);
        assert_eq!(world.level_load().timeouts, 1);
    }

    #[test]
    fn same_dimension_respawn_restarts_load_timeout_without_replacing_level_identity() {
        let start = Instant::now();
        let mut world = loading_world(0, 64.0);
        let mut runtime = LevelLoadRuntime::default();
        runtime.tick(start, &mut world, &None, false);
        let level_generation = world.client_level_generation();

        world.apply_respawn(&Respawn {
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 1,
                game_type: 0,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 0,
        });
        assert_eq!(world.client_level_generation(), level_generation);

        let respawn_start = start + Duration::from_secs(5);
        runtime.tick(respawn_start, &mut world, &None, false);
        assert_eq!(
            runtime.timeout_after,
            Some(respawn_start + CLIENT_WAIT_TIMEOUT)
        );
    }

    #[test]
    fn production_checks_level_readiness_after_terrain_upload() {
        let source = include_str!("runtime.rs");
        let upload = source
            .find("    maybe_upload_decoded_terrain(")
            .expect("runtime should upload decoded terrain");
        let readiness = source
            .find("SimulationCommand::TerrainReadiness")
            .expect("render publication should return level readiness");

        assert!(upload < readiness);

        let simulation_tick = source
            .find("level_load_runtime.tick(now, world, net_commands, player_section_ready);")
            .expect("simulation should own native level-load completion");
        let render_entry = source
            .find("pub(crate) fn update_renderer_from_publication(")
            .expect("publication render entry");
        assert!(simulation_tick < render_entry);
    }

    #[test]
    fn respawn_local_consumers_cannot_restore_level_generation_as_their_only_identity() {
        let level_load = include_str!("level_load_runtime.rs");
        let ambient_audio = include_str!("environment_audio_runtime.rs");
        let movement = include_str!("input/movement.rs");
        let runtime = include_str!("runtime.rs");
        let sprint_baseline = movement
            .split_once("pub(super) fn synchronize_local_player_sprint_generation(")
            .expect("sprint baseline owner")
            .1
            .split_once("fn reconcile_local_player_sprint_state")
            .expect("sprint baseline owner end")
            .0;
        let simulation_level_load = runtime
            .split_once("let player_section_ready = input_runtime")
            .expect("simulation level-load projection")
            .1
            .split_once("level_load_runtime.tick")
            .expect("simulation level-load projection end")
            .0;
        let render_readiness = runtime
            .split_once("terrain_upload.take_readiness_edge(")
            .expect("render readiness projection")
            .1
            .split_once(")\n    {")
            .expect("render readiness projection end")
            .0;

        for (owner, production) in [
            (
                "level load",
                level_load
                    .split("\n#[cfg(test)]\nmod tests")
                    .next()
                    .expect("level-load production source"),
            ),
            (
                "ambient audio",
                ambient_audio
                    .split("\n#[cfg(test)]\nmod tests")
                    .next()
                    .expect("ambient-audio production source"),
            ),
            ("sprint baseline", sprint_baseline),
            ("simulation level load", simulation_level_load),
            ("terrain readiness", render_readiness),
        ] {
            assert!(
                production.contains("world.level_load().starts"),
                "{owner} must include the canonical same-dimension respawn edge"
            );
        }

        let level = include_str!("../../bbb-world/src/level.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("world level production source");
        let respawn = level
            .split_once("pub fn apply_respawn(")
            .expect("respawn owner")
            .1
            .split_once("pub fn apply_world_time(")
            .expect("respawn owner end")
            .0;
        assert_eq!(
            respawn
                .matches("self.advance_client_level_generation();")
                .count(),
            1
        );
        assert!(respawn.contains("if dimension_changed {"));
    }

    fn loading_world(game_type: i8, y: f64) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d { x: 0.0, y, z: 0.0 },
            ..LocalPlayerPoseState::default()
        });
        world
    }

    fn notify_loading_packets(world: &mut WorldStore) {
        world.apply_game_event(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        });
    }
}
