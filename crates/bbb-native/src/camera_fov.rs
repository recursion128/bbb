use bbb_renderer::{CameraHurtEffect, CameraPose};
use bbb_world::{BlockPos, TerrainFluidKind, TerrainFluidState, WorldStore};
use glam::Vec3;

pub(crate) const VANILLA_DEFAULT_CAMERA_FOV_DEGREES: f32 = 70.0;
pub(crate) const VANILLA_MIN_CAMERA_FOV_DEGREES: u16 = 30;
pub(crate) const VANILLA_MAX_CAMERA_FOV_DEGREES: u16 = 110;
pub(crate) const VANILLA_DEFAULT_FOV_EFFECT_SCALE: f32 = 1.0;

const VANILLA_CAMERA_NEAR_PLANE: f32 = 0.05;
const VANILLA_FLUID_FOV_SCALE: f32 = 0.857_142_87;
const VANILLA_MAX_DEATH_FOV_TICKS: f32 = 20.0;
const VANILLA_DEATH_FOV_OFFSET: f32 = 500.0;

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
            base_degrees,
            effect_scale,
        }
    }

    pub(crate) fn advance_for_world(&mut self, ticks: u32, world: &WorldStore) {
        for _ in 0..ticks {
            let target = if world.local_player().camera.follows_player {
                world.local_player_camera_fov_modifier(true, self.effect_scale)
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

    pub(crate) fn frame_for_world(
        self,
        world: &WorldStore,
        camera_pose: Option<CameraPose>,
        camera_hurt_effect: Option<CameraHurtEffect>,
        viewport: [u32; 2],
        partial_tick: f32,
    ) -> CameraFovFrame {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let modifier = self.old_modifier + (self.modifier - self.old_modifier) * partial_tick;
        let in_fov_fluid = camera_pose
            .is_some_and(|pose| camera_is_in_fov_fluid(world, pose, viewport, self.base_degrees));
        let world_degrees = modify_fov_for_death_and_fluid(
            self.base_degrees * modifier,
            camera_hurt_effect,
            in_fov_fluid,
            self.effect_scale,
        );
        let hud_degrees = modify_fov_for_death_and_fluid(
            VANILLA_DEFAULT_CAMERA_FOV_DEGREES,
            camera_hurt_effect,
            in_fov_fluid,
            self.effect_scale,
        );
        CameraFovFrame {
            world_degrees,
            culling_degrees: world_degrees.max(self.base_degrees),
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

fn camera_is_in_fov_fluid(
    world: &WorldStore,
    camera: CameraPose,
    viewport: [u32; 2],
    base_degrees: f32,
) -> bool {
    let eye = Vec3::from_array(camera.position) + Vec3::Y * camera.eye_height;
    if camera_point_in_fluid(world, eye, TerrainFluidKind::Water, false) {
        return true;
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
    [
        near_forward,
        near_forward + near_up + near_left,
        near_forward + near_up - near_left,
        near_forward - near_up + near_left,
        near_forward - near_up - near_left,
    ]
    .into_iter()
    .any(|offset| camera_point_in_fluid(world, eye + offset, TerrainFluidKind::Lava, true))
}

pub(crate) fn camera_eye_in_water(world: &WorldStore, eye: [f32; 3]) -> bool {
    camera_point_in_fluid(world, Vec3::from_array(eye), TerrainFluidKind::Water, false)
}

fn camera_point_in_fluid(
    world: &WorldStore,
    point: Vec3,
    kind: TerrainFluidKind,
    surface_is_inclusive: bool,
) -> bool {
    if !point.is_finite() {
        return false;
    }
    let pos = BlockPos {
        x: point.x.floor() as i32,
        y: point.y.floor() as i32,
        z: point.z.floor() as i32,
    };
    let Some(fluid) = world.probe_block(pos).and_then(|block| block.fluid) else {
        return false;
    };
    if fluid.kind != kind {
        return false;
    }
    let surface = f64::from(pos.y) + camera_fluid_height_at(world, pos, fluid);
    if surface_is_inclusive {
        f64::from(point.y) <= surface
    } else {
        f64::from(point.y) < surface
    }
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
}
