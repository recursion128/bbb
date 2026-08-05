//! Vanilla `Gui` vignette tick state and world-border warning projection.

use bbb_renderer::HudVignette;
use bbb_world::{BlockPos, WorldBorderState, WorldStore};

const VIGNETTE_BRIGHTNESS_SMOOTHING: f32 = 0.01;
const FULL_RAW_LIGHT: u8 = 15;

#[derive(Debug)]
pub(super) struct VignetteRuntime {
    brightness: f32,
}

impl Default for VignetteRuntime {
    fn default() -> Self {
        Self { brightness: 1.0 }
    }
}

impl VignetteRuntime {
    /// Mirrors the `Gui.tick` camera branch. Raw GUI ticks continue while the
    /// level tick-rate manager is frozen, and a missing camera leaves the
    /// previous brightness untouched.
    pub(super) fn advance(&mut self, ticks: u32, world: &WorldStore, camera_eye: Option<[f32; 3]>) {
        let Some(camera_eye) =
            camera_eye.filter(|position| position.iter().all(|coordinate| coordinate.is_finite()))
        else {
            return;
        };
        let target = vignette_target_brightness(world, camera_eye);
        for _ in 0..ticks {
            self.brightness += (target - self.brightness) * VIGNETTE_BRIGHTNESS_SMOOTHING;
        }
    }

    /// Mirrors `Gui.extractVignette`. The draw is hidden by F1 or by the
    /// `vignette` option, while the smoothed tick state above remains live.
    pub(super) fn extract(
        &self,
        world: &WorldStore,
        camera_position: Option<[f32; 3]>,
        hide_gui: bool,
        vignette_option: bool,
    ) -> Option<HudVignette> {
        if hide_gui || !vignette_option || world.level_info().is_none() {
            return None;
        }
        Some(vignette_for_border(
            self.brightness,
            world.world_border(),
            camera_position,
        ))
    }
}

fn vignette_target_brightness(world: &WorldStore, camera_eye: [f32; 3]) -> f32 {
    let pos = BlockPos {
        x: camera_eye[0].floor() as i32,
        y: camera_eye[1].floor() as i32,
        z: camera_eye[2].floor() as i32,
    };
    let raw = max_local_raw_brightness(world, pos);
    let ambient_light = world
        .level_info()
        .map_or(0.0, |level| level.ambient_light());
    (1.0 - vanilla_lightmap_brightness(ambient_light, raw)).clamp(0.0, 1.0)
}

fn max_local_raw_brightness(world: &WorldStore, pos: BlockPos) -> u8 {
    if pos.x < -30_000_000 || pos.z < -30_000_000 || pos.x >= 30_000_000 || pos.z >= 30_000_000 {
        return FULL_RAW_LIGHT;
    }
    let sky_darken = super::entity_shadow_sky_darken(world);
    world
        .sample_block_light(pos)
        .map_or(FULL_RAW_LIGHT, |light| {
            light.block.max(light.sky.saturating_sub(sky_darken))
        })
}

fn vanilla_lightmap_brightness(ambient_light: f32, raw_light: u8) -> f32 {
    let value = f32::from(raw_light.min(FULL_RAW_LIGHT)) / f32::from(FULL_RAW_LIGHT);
    let curved = value / (4.0 - 3.0 * value);
    ambient_light + (1.0 - ambient_light) * curved
}

fn vignette_for_border(
    brightness: f32,
    border: &WorldBorderState,
    camera_position: Option<[f32; 3]>,
) -> HudVignette {
    let brightness = brightness.clamp(0.0, 1.0);
    let border_warning_strength = camera_position
        .filter(|position| position.iter().all(|coordinate| coordinate.is_finite()))
        .map_or(0.0, |position| {
            border_warning_strength(border, f64::from(position[0]), f64::from(position[2]))
        });
    let color = if border_warning_strength > 0.0 {
        let red = brightness * (1.0 - border_warning_strength);
        let green_blue = brightness + (1.0 - brightness) * border_warning_strength;
        [red, green_blue, green_blue]
    } else {
        [brightness; 3]
    };
    HudVignette {
        brightness,
        border_warning_strength,
        color,
    }
}

fn border_warning_strength(border: &WorldBorderState, camera_x: f64, camera_z: f64) -> f32 {
    let distance = border.distance_to_border(camera_x, camera_z) as f32;
    let lerp_speed = if border.lerp_time > 0 && border.lerp_duration != 0 {
        (border.size - border.lerp_target).abs() / border.lerp_duration as f64
    } else {
        0.0
    };
    let moving_blocks_threshold = (lerp_speed * f64::from(border.warning_time))
        .min((border.lerp_target - border.current_size).abs());
    let warning_distance = f64::from(border.warning_blocks).max(moving_blocks_threshold);
    if f64::from(distance) < warning_distance {
        (1.0 - (f64::from(distance) / warning_distance) as f32).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        CommonPlayerSpawnInfo, InitializeBorder, PlayLogin, SetBorderWarningDelay,
        SetBorderWarningDistance,
    };

    fn logged_in_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 1,
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
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world
    }

    fn initialized_border(
        old_size: f64,
        new_size: f64,
        lerp_time: i64,
        warning_blocks: i32,
        warning_time: i32,
    ) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_initialize_border(InitializeBorder {
            new_center_x: 0.0,
            new_center_z: 0.0,
            old_size,
            new_size,
            lerp_time,
            new_absolute_max_size: 29_999_984,
            warning_blocks,
            warning_time,
        });
        world
    }

    #[test]
    fn vignette_brightness_curve_and_tick_smoothing_match_vanilla() {
        assert_eq!(vanilla_lightmap_brightness(0.0, 0), 0.0);
        assert_eq!(vanilla_lightmap_brightness(0.25, 0), 0.25);
        assert_eq!(vanilla_lightmap_brightness(0.0, 15), 1.0);
        assert!((vanilla_lightmap_brightness(0.0, 5) - (1.0 / 9.0)).abs() < 1.0e-6);

        let mut runtime = VignetteRuntime::default();
        runtime.brightness += (0.0 - runtime.brightness) * VIGNETTE_BRIGHTNESS_SMOOTHING;
        assert!((runtime.brightness - 0.99).abs() < 1.0e-6);
        for _ in 1..100 {
            runtime.brightness += (0.0 - runtime.brightness) * VIGNETTE_BRIGHTNESS_SMOOTHING;
        }
        assert!((runtime.brightness - 0.99_f32.powi(100)).abs() < 1.0e-6);
    }

    #[test]
    fn vignette_runtime_keeps_brightness_without_camera_and_ticks_while_hidden() {
        let world = WorldStore::new();
        let mut runtime = VignetteRuntime::default();
        runtime.advance(5, &world, None);
        assert_eq!(runtime.brightness, 1.0);

        // An unloaded camera-light sample is explicitly full-bright, so its
        // target darkness is zero.
        runtime.advance(1, &world, Some([0.0, 64.0, 0.0]));
        assert!((runtime.brightness - 0.99).abs() < 1.0e-6);
        assert_eq!(runtime.extract(&world, Some([0.0; 3]), true, true), None);
    }

    #[test]
    fn the_vignette_option_hides_the_draw_while_the_tick_state_keeps_smoothing() {
        let world = logged_in_world();
        let mut runtime = VignetteRuntime::default();
        runtime.advance(1, &world, Some([0.0, 64.0, 0.0]));
        let smoothed = runtime.brightness;

        assert_eq!(runtime.extract(&world, Some([0.0; 3]), false, false), None);
        assert!(runtime
            .extract(&world, Some([0.0; 3]), false, true)
            .is_some());

        runtime.advance(1, &world, Some([0.0, 64.0, 0.0]));
        assert!(runtime.brightness < smoothed);
    }

    #[test]
    fn vignette_border_warning_uses_static_distance_and_tint_formula() {
        let world = initialized_border(100.0, 100.0, 0, 10, 15);
        let vignette = vignette_for_border(0.4, world.world_border(), Some([45.0, 64.0, 0.0]));
        assert!((vignette.border_warning_strength - 0.5).abs() < 1.0e-6);
        assert!((vignette.color[0] - 0.2).abs() < 1.0e-6);
        assert!((vignette.color[1] - 0.7).abs() < 1.0e-6);
        assert!((vignette.color[2] - 0.7).abs() < 1.0e-6);

        let no_camera = vignette_for_border(0.4, world.world_border(), None);
        assert_eq!(no_camera.border_warning_strength, 0.0);
        assert_eq!(no_camera.color, [0.4; 3]);
    }

    #[test]
    fn vignette_moving_threshold_uses_speed_time_and_remaining_delta() {
        // Speed = 200 / 100 = 2 blocks/tick; warning time 15 makes the
        // moving threshold 30, larger than warningBlocks 5.
        let mut world = initialized_border(200.0, 0.0, 100, 5, 15);
        assert!((border_warning_strength(world.world_border(), 85.0, 0.0) - 0.5).abs() < 1.0e-6);

        // Once only 20 blocks remain, min(speed*time, remaining delta) narrows
        // warning distance to 20. The partial-tick-zero border bounds still
        // use the previous tick's size 22, so x=5 is 6 blocks from the edge.
        world.advance_world_border(90);
        assert!((border_warning_strength(world.world_border(), 5.0, 0.0) - 0.7).abs() < 1.0e-6);

        world.apply_set_border_warning_delay(SetBorderWarningDelay { warning_delay: 2 });
        world.apply_set_border_warning_distance(SetBorderWarningDistance { warning_blocks: 7 });
        // speed*time = 4, so warningBlocks wins.
        assert!((border_warning_strength(world.world_border(), 7.5, 0.0) - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn vignette_outside_border_clamps_warning_strength_to_one() {
        let world = initialized_border(100.0, 100.0, 0, 5, 15);
        assert_eq!(
            border_warning_strength(world.world_border(), 60.0, 0.0),
            1.0
        );
    }
}
