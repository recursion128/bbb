use std::time::{Duration, Instant};

use bbb_audio::AudioListenerState;
use bbb_renderer::{HudSoundSubtitleDirection, HudSoundSubtitleRow, HudSoundSubtitles};

use crate::audio_runtime::SoundSubtitleEvent;

const MAX_SUBTITLE_GROUPS: usize = 64;
const MAX_LOCATIONS_PER_GROUP: usize = 16;
const MAX_SUBTITLE_KEY_CHARS: usize = 256;
const BASE_DISPLAY_TIME: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
pub(crate) struct SoundSubtitleRuntime {
    groups: Vec<SoundSubtitleGroup>,
}

#[derive(Debug)]
struct SoundSubtitleGroup {
    subtitle_key: String,
    range: f64,
    locations: Vec<SoundSubtitleLocation>,
}

#[derive(Debug, Clone, Copy)]
struct SoundSubtitleLocation {
    position: [f64; 3],
    played_at: Instant,
}

impl SoundSubtitleRuntime {
    pub(crate) fn record(&mut self, event: SoundSubtitleEvent, now: Instant) {
        let Some(subtitle_key) = sanitize_subtitle_key(&event.subtitle_key) else {
            return;
        };
        if !event.position.into_iter().all(f64::is_finite)
            || event.range.is_nan()
            || event.range <= 0.0
        {
            return;
        }
        let location = SoundSubtitleLocation {
            position: event.position,
            played_at: now,
        };
        if let Some(group) = self
            .groups
            .iter_mut()
            .find(|group| group.subtitle_key == subtitle_key)
        {
            group
                .locations
                .retain(|existing| existing.position != location.position);
            push_bounded_location(&mut group.locations, location);
            return;
        }
        if self.groups.len() == MAX_SUBTITLE_GROUPS {
            self.groups.remove(0);
        }
        self.groups.push(SoundSubtitleGroup {
            subtitle_key,
            range: event.range,
            locations: vec![location],
        });
    }

    pub(crate) fn project(
        &mut self,
        listener: Option<AudioListenerState>,
        display_time_multiplier: f32,
        now: Instant,
        mut localize: impl FnMut(&str) -> String,
    ) -> HudSoundSubtitles {
        let Some(listener) = listener.filter(valid_listener) else {
            return HudSoundSubtitles::default();
        };
        let Some(max_age) = display_time(display_time_multiplier) else {
            return HudSoundSubtitles::default();
        };
        let audible_groups = self
            .groups
            .iter()
            .enumerate()
            .filter_map(|(index, group)| {
                group_is_audible(group, listener.position).then_some(index)
            })
            .collect::<Vec<_>>();
        let (forward, right) = listener_axes(listener);
        let mut rows = Vec::with_capacity(audible_groups.len());
        for index in audible_groups {
            let group = &mut self.groups[index];
            group
                .locations
                .retain(|location| age_at(now, location.played_at) <= max_age);
            let Some(location) = closest_location(&group.locations, listener.position) else {
                continue;
            };
            rows.push(HudSoundSubtitleRow {
                text: localize(&group.subtitle_key),
                brightness: subtitle_brightness(age_at(now, location.played_at), max_age),
                direction: subtitle_direction(listener.position, location.position, forward, right),
            });
        }
        HudSoundSubtitles { rows }
    }
}

fn sanitize_subtitle_key(key: &str) -> Option<String> {
    let key = key
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_SUBTITLE_KEY_CHARS)
        .collect::<String>();
    let key = key.trim();
    (!key.is_empty()).then(|| key.to_string())
}

fn push_bounded_location(
    locations: &mut Vec<SoundSubtitleLocation>,
    location: SoundSubtitleLocation,
) {
    if locations.len() == MAX_LOCATIONS_PER_GROUP {
        locations.remove(0);
    }
    locations.push(location);
}

fn valid_listener(listener: &AudioListenerState) -> bool {
    listener.position.into_iter().all(f64::is_finite)
        && listener.y_rot.is_finite()
        && listener.x_rot.is_finite()
}

fn display_time(multiplier: f32) -> Option<Duration> {
    (multiplier.is_finite() && (0.5..=10.0).contains(&multiplier))
        .then(|| BASE_DISPLAY_TIME.mul_f32(multiplier))
}

fn group_is_audible(group: &SoundSubtitleGroup, listener: [f64; 3]) -> bool {
    if group.range.is_infinite() {
        return group.range.is_sign_positive();
    }
    let Some(closest) = closest_location(&group.locations, listener) else {
        return false;
    };
    distance_squared(listener, closest.position) < group.range * group.range
}

fn closest_location(
    locations: &[SoundSubtitleLocation],
    listener: [f64; 3],
) -> Option<SoundSubtitleLocation> {
    locations.iter().copied().min_by(|left, right| {
        distance_squared(listener, left.position)
            .total_cmp(&distance_squared(listener, right.position))
    })
}

fn listener_axes(listener: AudioListenerState) -> ([f64; 3], [f64; 3]) {
    let yaw = listener.y_rot.to_radians();
    let pitch = listener.x_rot.to_radians();
    let (yaw_sin, yaw_cos) = yaw.sin_cos();
    let (pitch_sin, pitch_cos) = pitch.sin_cos();
    (
        [
            f64::from(-yaw_sin * pitch_cos),
            f64::from(-pitch_sin),
            f64::from(yaw_cos * pitch_cos),
        ],
        [f64::from(-yaw_cos), 0.0, f64::from(-yaw_sin)],
    )
}

fn subtitle_direction(
    listener: [f64; 3],
    location: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
) -> HudSoundSubtitleDirection {
    let delta = [
        location[0] - listener[0],
        location[1] - listener[1],
        location[2] - listener[2],
    ];
    let length_squared = dot(delta, delta);
    if length_squared < 1.0e-8 {
        return HudSoundSubtitleDirection::None;
    }
    let inverse_length = length_squared.sqrt().recip();
    let direction = [
        delta[0] * inverse_length,
        delta[1] * inverse_length,
        delta[2] * inverse_length,
    ];
    if dot(forward, direction) > 0.5 {
        return HudSoundSubtitleDirection::None;
    }
    let rightness = dot(right, direction);
    if rightness > 0.0 {
        HudSoundSubtitleDirection::Right
    } else if rightness < 0.0 {
        HudSoundSubtitleDirection::Left
    } else {
        HudSoundSubtitleDirection::None
    }
}

fn subtitle_brightness(age: Duration, max_age: Duration) -> u8 {
    let progress = (age.as_secs_f64() / max_age.as_secs_f64()).clamp(0.0, 1.0);
    (255.0 + progress * (75.0 - 255.0)).floor() as u8
}

fn age_at(now: Instant, played_at: Instant) -> Duration {
    now.checked_duration_since(played_at)
        .unwrap_or(Duration::ZERO)
}

fn distance_squared(left: [f64; 3], right: [f64; 3]) -> f64 {
    let delta = [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
    dot(delta, delta)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener() -> AudioListenerState {
        AudioListenerState {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
        }
    }

    fn event(key: &str, position: [f64; 3], range: f64) -> SoundSubtitleEvent {
        SoundSubtitleEvent {
            subtitle_key: key.to_string(),
            position,
            range,
        }
    }

    #[test]
    fn sound_subtitle_merge_refresh_preserves_first_range_and_row_order() {
        let now = Instant::now();
        let mut runtime = SoundSubtitleRuntime::default();
        runtime.record(event("first", [2.0, 0.0, 0.0], 1.0), now);
        runtime.record(event("second", [0.0, 0.0, 1.0], 16.0), now);
        runtime.record(
            event("first", [0.0, 0.0, 1.0], 100.0),
            now + Duration::from_millis(1),
        );

        let projected = runtime.project(
            Some(listener()),
            1.0,
            now + Duration::from_millis(2),
            str::to_string,
        );

        assert_eq!(
            projected
                .rows
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>(),
            ["second"]
        );
        assert_eq!(runtime.groups[0].range, 1.0);
        assert_eq!(runtime.groups[0].locations.len(), 2);
        assert_eq!(runtime.groups[1].subtitle_key, "second");
    }

    #[test]
    fn sound_subtitle_projection_uses_strict_range_and_listener_directions() {
        let now = Instant::now();
        let mut runtime = SoundSubtitleRuntime::default();
        runtime.record(event("edge", [0.0, 0.0, 16.0], 16.0), now);
        runtime.record(event("ahead", [0.0, 0.0, 10.0], 16.0), now);
        runtime.record(event("right", [-10.0, 0.0, 0.0], 16.0), now);
        runtime.record(event("left", [10.0, 0.0, 0.0], 16.0), now);

        let projected = runtime.project(Some(listener()), 1.0, now, str::to_string);

        assert_eq!(
            projected.rows,
            vec![
                HudSoundSubtitleRow {
                    text: "ahead".to_string(),
                    brightness: 255,
                    direction: HudSoundSubtitleDirection::None,
                },
                HudSoundSubtitleRow {
                    text: "right".to_string(),
                    brightness: 255,
                    direction: HudSoundSubtitleDirection::Right,
                },
                HudSoundSubtitleRow {
                    text: "left".to_string(),
                    brightness: 255,
                    direction: HudSoundSubtitleDirection::Left,
                },
            ]
        );
    }

    #[test]
    fn sound_subtitle_expiry_brightness_and_localization_match_vanilla() {
        let now = Instant::now();
        let mut runtime = SoundSubtitleRuntime::default();
        runtime.record(event("subtitles.test", [0.0, 0.0, 1.0], 16.0), now);

        let halfway = runtime.project(
            Some(listener()),
            1.0,
            now + Duration::from_millis(1_500),
            |key| format!("localized:{key}"),
        );
        assert_eq!(halfway.rows[0].text, "localized:subtitles.test");
        assert_eq!(halfway.rows[0].brightness, 165);

        let boundary = runtime.project(
            Some(listener()),
            1.0,
            now + Duration::from_secs(3),
            str::to_string,
        );
        assert_eq!(boundary.rows[0].brightness, 75);

        let expired = runtime.project(
            Some(listener()),
            1.0,
            now + Duration::from_secs(3) + Duration::from_nanos(1),
            str::to_string,
        );
        assert!(expired.rows.is_empty());
        assert_eq!(runtime.groups.len(), 1);
        assert!(runtime.groups[0].locations.is_empty());
    }

    #[test]
    fn sound_subtitle_bounds_drop_oldest_groups_and_locations() {
        let now = Instant::now();
        let mut runtime = SoundSubtitleRuntime::default();
        for index in 0..=MAX_SUBTITLE_GROUPS {
            runtime.record(
                event(
                    &format!("subtitle.{index}"),
                    [f64::from(index as u32), 0.0, 0.0],
                    f64::INFINITY,
                ),
                now,
            );
        }
        assert_eq!(runtime.groups.len(), MAX_SUBTITLE_GROUPS);
        assert_eq!(runtime.groups[0].subtitle_key, "subtitle.1");

        for index in 0..=MAX_LOCATIONS_PER_GROUP {
            runtime.record(
                event(
                    "subtitle.64",
                    [f64::from(index as u32), 1.0, 0.0],
                    f64::INFINITY,
                ),
                now + Duration::from_millis(index as u64),
            );
        }
        let group = runtime.groups.last().unwrap();
        assert_eq!(group.locations.len(), MAX_LOCATIONS_PER_GROUP);
        assert_eq!(group.locations[0].position, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn sound_subtitle_invalid_inputs_fail_closed() {
        let now = Instant::now();
        let mut runtime = SoundSubtitleRuntime::default();
        runtime.record(event("\n", [0.0; 3], 16.0), now);
        runtime.record(event("nan-position", [f64::NAN, 0.0, 0.0], 16.0), now);
        runtime.record(event("bad-range", [0.0; 3], f64::NAN), now);
        assert!(runtime.groups.is_empty());
        assert!(runtime
            .project(Some(listener()), f32::NAN, now, str::to_string)
            .rows
            .is_empty());
        assert!(runtime
            .project(Some(listener()), 0.49, now, str::to_string)
            .rows
            .is_empty());
        assert!(runtime
            .project(None, 1.0, now, str::to_string)
            .rows
            .is_empty());
    }
}
