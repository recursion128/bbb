use bbb_renderer::CameraPose;
use bbb_world::{ClientEnvironmentAttributes, WorldStore};

use super::{
    argb_color, argb_srgb_lerp, argb_to_rgba8, rgb_u8_to_argb,
    sampled_environment_attributes_at_camera,
};

/// Render-thread mirror of Vanilla's `Camera.attributeProbe`.
///
/// `Camera.tick()` rebuilds the 216-point Gaussian biome interpolator once per
/// client tick. Render extraction only interpolates the probe's previous and
/// current values. Keeping a dedicated owner avoids repeating the spatial
/// query for every presented frame while retaining tick-local changes.
#[derive(Debug, Default)]
pub(crate) struct CameraEnvironmentSampleCache {
    key: Option<(u64, u64)>,
    previous: ClientEnvironmentAttributes,
    current: ClientEnvironmentAttributes,
}

impl CameraEnvironmentSampleCache {
    pub(super) fn sample(
        &mut self,
        world: &WorldStore,
        level_generation: u64,
        client_tick_index: u64,
        partial_tick: f32,
        camera_pose: Option<CameraPose>,
    ) -> ClientEnvironmentAttributes {
        self.sample_with((level_generation, client_tick_index), partial_tick, || {
            sampled_environment_attributes_at_camera(world, camera_pose)
        })
    }

    fn sample_with(
        &mut self,
        key: (u64, u64),
        partial_tick: f32,
        sample: impl FnOnce() -> ClientEnvironmentAttributes,
    ) -> ClientEnvironmentAttributes {
        if self.key != Some(key) {
            let next = sample();
            if self
                .key
                .is_some_and(|(level_generation, client_tick_index)| {
                    level_generation == key.0 && client_tick_index.checked_add(1) == Some(key.1)
                })
            {
                self.previous = std::mem::replace(&mut self.current, next);
            } else {
                self.previous = next.clone();
                self.current = next;
            }
            self.key = Some(key);
        }
        interpolate_camera_environment_attributes(
            &self.previous,
            &self.current,
            partial_tick.clamp(0.0, 1.0),
        )
    }
}

fn interpolate_camera_environment_attributes(
    previous: &ClientEnvironmentAttributes,
    current: &ClientEnvironmentAttributes,
    partial_tick: f32,
) -> ClientEnvironmentAttributes {
    ClientEnvironmentAttributes {
        fog_color: lerp_optional_rgb(previous.fog_color, current.fog_color, partial_tick),
        fog_start_distance: lerp_optional_f32(
            previous.fog_start_distance,
            current.fog_start_distance,
            partial_tick,
        ),
        fog_end_distance: lerp_optional_f32(
            previous.fog_end_distance,
            current.fog_end_distance,
            partial_tick,
        ),
        sky_fog_end_distance: lerp_optional_f32(
            previous.sky_fog_end_distance,
            current.sky_fog_end_distance,
            partial_tick,
        ),
        cloud_fog_end_distance: lerp_optional_f32(
            previous.cloud_fog_end_distance,
            current.cloud_fog_end_distance,
            partial_tick,
        ),
        water_fog_color: lerp_optional_rgb(
            previous.water_fog_color,
            current.water_fog_color,
            partial_tick,
        ),
        water_fog_start_distance: lerp_optional_f32(
            previous.water_fog_start_distance,
            current.water_fog_start_distance,
            partial_tick,
        ),
        water_fog_end_distance: lerp_optional_f32(
            previous.water_fog_end_distance,
            current.water_fog_end_distance,
            partial_tick,
        ),
        sky_color: lerp_optional_rgb(previous.sky_color, current.sky_color, partial_tick),
        sunrise_sunset_color: lerp_optional_rgba(
            previous.sunrise_sunset_color,
            current.sunrise_sunset_color,
            partial_tick,
        ),
        sun_angle_degrees: lerp_optional_degrees(
            previous.sun_angle_degrees,
            current.sun_angle_degrees,
            partial_tick,
        ),
        moon_angle_degrees: lerp_optional_degrees(
            previous.moon_angle_degrees,
            current.moon_angle_degrees,
            partial_tick,
        ),
        star_angle_degrees: lerp_optional_degrees(
            previous.star_angle_degrees,
            current.star_angle_degrees,
            partial_tick,
        ),
        moon_phase: current.moon_phase.or(previous.moon_phase),
        star_brightness: lerp_optional_f32(
            previous.star_brightness,
            current.star_brightness,
            partial_tick,
        ),
        ..ClientEnvironmentAttributes::default()
    }
}

fn lerp_optional_f32(previous: Option<f32>, current: Option<f32>, alpha: f32) -> Option<f32> {
    lerp_optional(previous, current, |previous, current| {
        previous + alpha * (current - previous)
    })
}

fn lerp_optional_degrees(previous: Option<f32>, current: Option<f32>, alpha: f32) -> Option<f32> {
    lerp_optional(previous, current, |previous, current| {
        let delta = (current - previous + 180.0).rem_euclid(360.0) - 180.0;
        if delta.abs() >= 90.0 {
            current
        } else {
            previous + alpha * delta
        }
    })
}

fn lerp_optional_rgb(
    previous: Option<[u8; 3]>,
    current: Option<[u8; 3]>,
    alpha: f32,
) -> Option<[u8; 3]> {
    lerp_optional(previous, current, |previous, current| {
        let rgba = argb_to_rgba8(argb_srgb_lerp(
            alpha,
            rgb_u8_to_argb(previous),
            rgb_u8_to_argb(current),
        ));
        [rgba[0], rgba[1], rgba[2]]
    })
}

fn lerp_optional_rgba(
    previous: Option<[u8; 4]>,
    current: Option<[u8; 4]>,
    alpha: f32,
) -> Option<[u8; 4]> {
    lerp_optional(previous, current, |previous, current| {
        argb_to_rgba8(argb_srgb_lerp(
            alpha,
            argb_color(
                i32::from(previous[3]),
                i32::from(previous[0]),
                i32::from(previous[1]),
                i32::from(previous[2]),
            ),
            argb_color(
                i32::from(current[3]),
                i32::from(current[0]),
                i32::from(current[1]),
                i32::from(current[2]),
            ),
        ))
    })
}

fn lerp_optional<T: Copy>(
    previous: Option<T>,
    current: Option<T>,
    lerp: impl FnOnce(T, T) -> T,
) -> Option<T> {
    match (previous, current) {
        (Some(previous), Some(current)) => Some(lerp(previous, current)),
        (None, current) => current,
        (previous, None) => previous,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use bbb_world::{ClientEnvironmentAttributes, ClientMoonPhase};

    use super::CameraEnvironmentSampleCache;

    #[test]
    fn samples_once_per_tick_and_interpolates_cached_values() {
        let calls = Cell::new(0);
        let mut cache = CameraEnvironmentSampleCache::default();
        let initial = ClientEnvironmentAttributes {
            fog_color: Some([10, 20, 30]),
            fog_start_distance: Some(10.0),
            sun_angle_degrees: Some(350.0),
            moon_phase: Some(ClientMoonPhase::FullMoon),
            ..ClientEnvironmentAttributes::default()
        };
        let first = cache.sample_with((7, 20), 0.25, || {
            calls.set(calls.get() + 1);
            initial.clone()
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(first.fog_color, initial.fog_color);
        assert_eq!(first.fog_start_distance, initial.fog_start_distance);

        let repeated = cache.sample_with((7, 20), 1.0, || {
            panic!("the same client tick must reuse the Gaussian sample")
        });
        assert_eq!(repeated.fog_color, initial.fog_color);

        let current = ClientEnvironmentAttributes {
            fog_color: Some([30, 40, 50]),
            fog_start_distance: Some(30.0),
            sun_angle_degrees: Some(10.0),
            moon_phase: Some(ClientMoonPhase::NewMoon),
            ..ClientEnvironmentAttributes::default()
        };
        let midpoint = cache.sample_with((7, 21), 0.5, || {
            calls.set(calls.get() + 1);
            current.clone()
        });
        assert_eq!(calls.get(), 2);
        assert_eq!(midpoint.fog_color, Some([20, 30, 40]));
        assert_eq!(midpoint.fog_start_distance, Some(20.0));
        assert_eq!(midpoint.sun_angle_degrees, Some(360.0));
        assert_eq!(midpoint.moon_phase, current.moon_phase);

        let skipped = cache.sample_with((7, 23), 0.0, || ClientEnvironmentAttributes {
            fog_color: Some([70, 80, 90]),
            ..ClientEnvironmentAttributes::default()
        });
        assert_eq!(skipped.fog_color, Some([70, 80, 90]));

        let reset = cache.sample_with((8, 0), 0.0, || ClientEnvironmentAttributes {
            fog_color: Some([90, 100, 110]),
            ..ClientEnvironmentAttributes::default()
        });
        assert_eq!(reset.fog_color, Some([90, 100, 110]));
    }
}
