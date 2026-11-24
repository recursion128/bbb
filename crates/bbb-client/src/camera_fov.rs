use bbb_client_render::{CameraHurtEffect, CameraPose};
use bbb_client_world::{BlockPos, TerrainFluidKind, TerrainFluidState, WorldStore};
use glam::Vec3;

pub(crate) const VANILLA_DEFAULT_CAMERA_FOV_DEGREES: f32 = 70.0;
pub(crate) const VANILLA_MIN_CAMERA_FOV_DEGREES: u16 = 30;
pub(crate) const VANILLA_MAX_CAMERA_FOV_DEGREES: u16 = 110;
pub(crate) const VANILLA_DEFAULT_FOV_EFFECT_SCALE: f32 = 1.0;

const VANILLA_CAMERA_NEAR_PLANE: f32 = 0.05;
const VANILLA_FLUID_FOV_SCALE: f32 = 0.857_142_87;
const VANILLA_MAX_DEATH_FOV_TICKS: f32 = 20.0;
const VANILLA_DEATH_FOV_OFFSET: f32 = 500.0;

/// Vanilla `FogType` values produced by `Camera.getFluidInCamera`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CameraFluid {
    #[default]
    None,
    Water,
    Lava,
    PowderSnow,
}

impl CameraFluid {
    fn modifies_fov(self) -> bool {
        matches!(self, Self::Water | Self::Lava)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CameraFovFrame {
    pub(crate) world_degrees: f32,
    pub(crate) culling_degrees: f32,
    pub(crate) hud_degrees: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraFovTickState {
    old_modifier: f32,
    modifier: f32,
    old_zoom_factor: f32,
    zoom_factor: f32,
    base_degrees: f32,
    effect_scale: f32,
}

impl Default for CameraFovTickState {
    fn default() -> Self {
        Self::new(
            VANILLA_DEFAULT_CAMERA_FOV_DEGREES,
            VANILLA_DEFAULT_FOV_EFFECT_SCALE,
        )
    }
}

impl CameraFovTickState {
    pub(crate) fn new(base_degrees: f32, effect_scale: f32) -> Self {
        debug_assert!((f32::from(VANILLA_MIN_CAMERA_FOV_DEGREES)
            ..=f32::from(VANILLA_MAX_CAMERA_FOV_DEGREES))
            .contains(&base_degrees));
        debug_assert!((0.0..=1.0).contains(&effect_scale));
        Self {
            old_modifier: 1.0,
            modifier: 1.0,
            old_zoom_factor: 1.0,
            zoom_factor: 1.0,
            base_degrees,
            effect_scale,
        }
    }

    pub(crate) fn step_zoom(&mut self, target_factor: f32) {
        let target = target_factor.clamp(0.10, 1.0);
        self.old_zoom_factor = self.zoom_factor;
        self.zoom_factor += (target - self.zoom_factor) * 0.45;
        if (self.zoom_factor - target).abs() < 1e-3 {
            self.zoom_factor = target;
        }
    }

    pub(crate) fn advance_for_world(
        &mut self,
        ticks: u32,
        world: &WorldStore,
        first_person: bool,
        effective_sprinting: bool,
    ) {
        for _ in 0..ticks {
            let target = if world.local_player().camera.follows_player {
                world.local_player_camera_fov_modifier(
                    first_person,
                    self.effect_scale,
                    effective_sprinting,
                )
            } else {
                1.0
            };
            self.old_modifier = self.modifier;
            self.modifier += (target - self.modifier) * 0.5;
            self.modifier = self.modifier.clamp(0.1, 1.5);
        }
    }

    pub(crate) fn base_degrees(self) -> f32 {
        self.base_degrees
    }

    pub(crate) fn frame(
        self,
        camera_hurt_effect: Option<CameraHurtEffect>,
        camera_fluid: CameraFluid,
        partial_tick: f32,
    ) -> CameraFovFrame {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let modifier = self.old_modifier + (self.modifier - self.old_modifier) * partial_tick;
        let base_world_degrees = modify_fov_for_death_and_fluid(
            self.base_degrees * modifier,
            camera_hurt_effect,
            camera_fluid.modifies_fov(),
            self.effect_scale,
        );
        let (world_degrees, culling_degrees) =
            if self.old_zoom_factor < 1.0 || self.zoom_factor < 1.0 {
                let zf =
                    self.old_zoom_factor + (self.zoom_factor - self.old_zoom_factor) * partial_tick;
                let zoomed = (base_world_degrees * zf).clamp(1.0, 179.0);
                (zoomed, zoomed.max(self.base_degrees * zf))
            } else {
                (
                    base_world_degrees,
                    base_world_degrees.max(self.base_degrees),
                )
            };
        let hud_degrees = modify_fov_for_death_and_fluid(
            VANILLA_DEFAULT_CAMERA_FOV_DEGREES,
            camera_hurt_effect,
            camera_fluid.modifies_fov(),
            self.effect_scale,
        );
        CameraFovFrame {
            world_degrees,
            culling_degrees,
            hud_degrees,
        }
    }

    #[cfg(test)]
    pub(crate) fn unmodified_fov_degrees(self, partial_tick: f32) -> f32 {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let modifier = self.old_modifier + (self.modifier - self.old_modifier) * partial_tick;
        self.base_degrees * modifier
    }
}

fn modify_fov_for_death_and_fluid(
    mut degrees: f32,
    camera_hurt_effect: Option<CameraHurtEffect>,
    in_fov_fluid: bool,
    effect_scale: f32,
) -> f32 {
    if let Some(effect) = camera_hurt_effect.filter(|effect| effect.is_dead_or_dying) {
        let duration = effect.death_time.min(VANILLA_MAX_DEATH_FOV_TICKS);
        degrees /=
            (1.0 - VANILLA_DEATH_FOV_OFFSET / (duration + VANILLA_DEATH_FOV_OFFSET)) * 2.0 + 1.0;
    }
    if in_fov_fluid {
        degrees *= 1.0 + (VANILLA_FLUID_FOV_SCALE - 1.0) * effect_scale;
    }
    degrees
}

/// Mirrors Vanilla 26.1 `Camera.getFluidInCamera`: Water is tested only at
/// the eye, then each base-FOV near-plane point tests Lava before powder snow.
pub(crate) fn camera_fluid_in_camera(
    world: &WorldStore,
    camera: CameraPose,
    viewport: [u32; 2],
    base_degrees: f32,
) -> CameraFluid {
    let eye = Vec3::from_array(camera.position) + Vec3::Y * camera.eye_height;
    if !eye.is_finite() {
        return CameraFluid::None;
    }
    let mut pos = camera_block_pos(eye);
    let mut block = world.block_view(pos);
    if let Some(water) = block
        .and_then(|block| block.fluid)
        .filter(|fluid| fluid.kind == TerrainFluidKind::Water)
    {
        if f64::from(eye.y) < f64::from(pos.y) + camera_fluid_height_at(world, pos, water) {
            return CameraFluid::Water;
        }
    }

    let yaw = camera.y_rot.to_radians();
    let pitch = camera.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::Z
    };
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let right = if right.length_squared() > 0.0 {
        right
    } else {
        Vec3::X
    };
    let up = right.cross(forward).normalize_or_zero();
    let up = if up.length_squared() > 0.0 {
        up
    } else {
        Vec3::Y
    };
    let aspect = viewport[0] as f32 / viewport[1].max(1) as f32;
    let plane_height = (base_degrees.to_radians() * 0.5).tan() * VANILLA_CAMERA_NEAR_PLANE;
    let plane_width = plane_height * aspect;
    let near_forward = forward * VANILLA_CAMERA_NEAR_PLANE;
    let near_left = -right * plane_width;
    let near_up = up * plane_height;
    for offset in [
        near_forward,
        near_forward + near_up + near_left,
        near_forward + near_up - near_left,
        near_forward - near_up + near_left,
        near_forward - near_up - near_left,
    ] {
        let point = eye + offset;
        if !point.is_finite() {
            continue;
        }
        let sample_pos = camera_block_pos(point);
        if sample_pos != pos {
            pos = sample_pos;
            block = world.block_view(pos);
        }
        let Some(sample) = block else { continue };
        if let Some(lava) = sample
            .fluid
            .filter(|fluid| fluid.kind == TerrainFluidKind::Lava)
        {
            if f64::from(point.y) <= f64::from(pos.y) + camera_fluid_height_at(world, pos, lava) {
                return CameraFluid::Lava;
            }
        }
        if sample.block_name == Some("minecraft:powder_snow") {
            return CameraFluid::PowderSnow;
        }
    }
    CameraFluid::None
}

fn camera_block_pos(point: Vec3) -> BlockPos {
    BlockPos {
        x: point.x.floor() as i32,
        y: point.y.floor() as i32,
        z: point.z.floor() as i32,
    }
}

fn camera_fluid_height_at(world: &WorldStore, pos: BlockPos, fluid: TerrainFluidState) -> f64 {
    let same_fluid_above = pos.y.checked_add(1).is_some_and(|above_y| {
        world
            .block_view(BlockPos {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hurt_effect(death_time: f32) -> CameraHurtEffect {
        CameraHurtEffect {
            hurt_time: 0.0,
            hurt_duration: 0.0,
            hurt_dir: 0.0,
            death_time,
            is_dead_or_dying: true,
            damage_tilt_strength: 1.0,
        }
    }

    #[test]
    fn camera_fov_death_and_fluid_formula_matches_vanilla_order() {
        let death_only = modify_fov_for_death_and_fluid(70.0, Some(hurt_effect(20.0)), false, 1.0);
        assert!((death_only - 65.0).abs() < 1.0e-5);

        let fluid_only = modify_fov_for_death_and_fluid(70.0, None, true, 1.0);
        assert!((fluid_only - 60.0).abs() < 1.0e-5);

        let both = modify_fov_for_death_and_fluid(70.0, Some(hurt_effect(20.0)), true, 1.0);
        assert!((both - 55.714_287).abs() < 1.0e-5);
    }

    #[test]
    fn camera_fov_effect_scale_controls_only_effect_contributions() {
        assert_eq!(modify_fov_for_death_and_fluid(70.0, None, true, 0.0), 70.0);
        assert!((modify_fov_for_death_and_fluid(70.0, None, true, 0.5) - 65.0).abs() < 1.0e-5);
    }

    #[test]
    fn camera_fov_zoom_narrows_world_only_and_restores_smoothly() {
        let mut state = CameraFovTickState::default();
        let baseline = state.frame(None, CameraFluid::None, 1.0);
        assert_eq!(baseline.world_degrees, VANILLA_DEFAULT_CAMERA_FOV_DEGREES);
        assert_eq!(baseline.culling_degrees, VANILLA_DEFAULT_CAMERA_FOV_DEGREES);
        assert_eq!(baseline.hud_degrees, VANILLA_DEFAULT_CAMERA_FOV_DEGREES);

        for _ in 0..32 {
            state.step_zoom(0.25);
        }
        let zoomed = state.frame(None, CameraFluid::None, 1.0);
        assert!((zoomed.world_degrees - 17.5).abs() < 0.05);
        assert_eq!(zoomed.hud_degrees, VANILLA_DEFAULT_CAMERA_FOV_DEGREES);

        for _ in 0..32 {
            state.step_zoom(1.0);
        }
        let restored = state.frame(None, CameraFluid::None, 1.0);
        assert!((restored.world_degrees - VANILLA_DEFAULT_CAMERA_FOV_DEGREES).abs() < 0.05);
    }
}
