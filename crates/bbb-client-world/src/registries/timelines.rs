use std::borrow::Cow;

use bbb_protocol::nbt_summary::compound_field as field;
use bbb_protocol::{nbt_summary::checked_int_value, NbtSummaryEntry, NbtSummaryValue};

use crate::{ClientEnvironmentAttributes, WorldStore};

use super::{
    environment_attributes::{
        apply_timeline_argument, decode_timeline_argument, interpolate_timeline_argument,
        EnvironmentModifier, EnvironmentTrackArgument,
    },
    RegistryProjectionError,
};

pub const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
pub const TIMELINE_REGISTRY: &str = "minecraft:timeline";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TimelineDefinition {
    pub(super) resource_id: String,
    pub(super) clock_id: i32,
    pub(super) period_ticks: Option<i32>,
    pub(super) tracks: Vec<TimelineTrackDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TimelineTrackDefinition {
    pub(super) attribute: String,
    pub(super) modifier: EnvironmentModifier,
    pub(super) keyframes: Vec<TimelineKeyframe>,
    pub(super) easing: TimelineEasing,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TimelineKeyframe {
    pub(super) ticks: i32,
    pub(super) argument: EnvironmentTrackArgument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TimelineEasing {
    Simple(String),
    CubicBezier {
        x1_bits: u32,
        y1_bits: u32,
        x2_bits: u32,
        y2_bits: u32,
    },
}

impl WorldStore {
    fn synchronized_timeline_definition(&self, holder_id: i32) -> Option<&TimelineDefinition> {
        let entry = self.registry_entry_by_holder_id(TIMELINE_REGISTRY, holder_id)?;
        self.registries.typed.timelines.get(&entry.id)
    }

    pub(super) fn visit_resolved_dimension_timelines(
        &self,
        mut visitor: impl FnMut(&TimelineDefinition, i64) -> Result<(), RegistryProjectionError>,
    ) -> Result<(), RegistryProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(());
        };
        let dimension = self
            .registries
            .dimension_type_client_projection(level.dimension_type_id);
        let timeline_resource_ids = dimension
            .as_ref()
            .map(|projection| projection.timeline_resource_ids.as_slice())
            .unwrap_or_else(|| level.timeline_resource_ids());
        for timeline_resource_id in timeline_resource_ids {
            let holder_id = self
                .registry_holder_id_by_resource_id(TIMELINE_REGISTRY, timeline_resource_id)
                .ok_or_else(|| {
                    error(
                        timeline_resource_id,
                        "<holder>",
                        "timeline is absent from the current session registry",
                    )
                })?;
            let timeline = self
                .synchronized_timeline_definition(holder_id)
                .ok_or_else(|| {
                    error(
                        timeline_resource_id,
                        "<root>",
                        "timeline has no synchronized typed value",
                    )
                })?;
            let total_ticks = self
                .world_time()
                .and_then(|time| {
                    time.clock_updates
                        .iter()
                        .find(|clock| clock.clock_id == timeline.clock_id)
                })
                .map(|clock| clock.total_ticks)
                .unwrap_or(0);
            visitor(timeline, total_ticks)?;
        }
        Ok(())
    }
}

pub(super) fn apply_environment_track(
    attributes: &mut ClientEnvironmentAttributes,
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<(), RegistryProjectionError> {
    let argument = sample_track_argument(timeline, track, total_ticks)?;
    apply_timeline_argument(
        attributes,
        &track.attribute,
        track.modifier,
        argument.as_ref(),
    )
    .map_err(|error| track_error(timeline, track, "modifier", error.to_string()))
}

fn sample_track_argument<'a>(
    timeline: &TimelineDefinition,
    track: &'a TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<Cow<'a, EnvironmentTrackArgument>, RegistryProjectionError> {
    if track.keyframes.len() == 1 {
        return Ok(Cow::Borrowed(&track.keyframes[0].argument));
    }
    let sample_ticks = timeline
        .period_ticks
        .map(|period| total_ticks.rem_euclid(i64::from(period)))
        .unwrap_or(total_ticks);
    let first = &track.keyframes[0];
    let last_index = track.keyframes.len() - 1;
    let last = &track.keyframes[last_index];
    let (from_index, from_ticks, to_index, to_ticks) = if let Some(period) = timeline.period_ticks {
        let period = i64::from(period);
        if sample_ticks < i64::from(first.ticks) {
            (
                last_index,
                i64::from(last.ticks) - period,
                0,
                i64::from(first.ticks),
            )
        } else if let Some(index) = track
            .keyframes
            .windows(2)
            .position(|pair| sample_ticks < i64::from(pair[1].ticks))
        {
            (
                index,
                i64::from(track.keyframes[index].ticks),
                index + 1,
                i64::from(track.keyframes[index + 1].ticks),
            )
        } else {
            (
                last_index,
                i64::from(last.ticks),
                0,
                i64::from(first.ticks) + period,
            )
        }
    } else if sample_ticks < i64::from(first.ticks) {
        return Ok(Cow::Borrowed(&first.argument));
    } else if let Some(index) = track
        .keyframes
        .windows(2)
        .position(|pair| sample_ticks < i64::from(pair[1].ticks))
    {
        (
            index,
            i64::from(track.keyframes[index].ticks),
            index + 1,
            i64::from(track.keyframes[index + 1].ticks),
        )
    } else {
        return Ok(Cow::Borrowed(&last.argument));
    };
    if sample_ticks <= from_ticks {
        return Ok(Cow::Borrowed(&track.keyframes[from_index].argument));
    }
    if sample_ticks >= to_ticks {
        return Ok(Cow::Borrowed(&track.keyframes[to_index].argument));
    }
    let alpha = (sample_ticks - from_ticks) as f32 / (to_ticks - from_ticks) as f32;
    let alpha = apply_easing(timeline, track, alpha)?;
    Ok(Cow::Owned(interpolate_timeline_argument(
        &track.attribute,
        &track.keyframes[from_index].argument,
        &track.keyframes[to_index].argument,
        alpha,
    )))
}

fn apply_easing(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    alpha: f32,
) -> Result<f32, RegistryProjectionError> {
    match &track.easing {
        TimelineEasing::Simple(name) => match name.as_str() {
            "constant" => Ok(0.0),
            "linear" => Ok(alpha),
            name => simple_easing(name, alpha).ok_or_else(|| {
                track_error(
                    timeline,
                    track,
                    "ease",
                    format!("unsupported easing {name}"),
                )
            }),
        },
        TimelineEasing::CubicBezier {
            x1_bits,
            y1_bits,
            x2_bits,
            y2_bits,
        } => Ok(sample_cubic_bezier(
            alpha,
            f32::from_bits(*x1_bits),
            f32::from_bits(*y1_bits),
            f32::from_bits(*x2_bits),
            f32::from_bits(*y2_bits),
        )),
    }
}

fn simple_easing(name: &str, x: f32) -> Option<f32> {
    Some(match name {
        "in_back" => x * x * (2.70158 * x - 1.70158),
        "in_bounce" => 1.0 - out_bounce(1.0 - x),
        "in_circ" => -(1.0 - x * x).sqrt() + 1.0,
        "in_cubic" => x * x * x,
        "in_elastic" => in_elastic(x),
        "in_expo" => {
            if x == 0.0 {
                0.0
            } else {
                2.0_f32.powf(10.0 * x - 10.0)
            }
        }
        "in_quad" => x * x,
        "in_quart" => x * x * x * x,
        "in_quint" => x * x * x * x * x,
        "in_sine" => 1.0 - (x * std::f32::consts::FRAC_PI_2).cos(),
        "in_out_back" => in_out_back(x),
        "in_out_bounce" => {
            if x < 0.5 {
                (1.0 - out_bounce(1.0 - 2.0 * x)) / 2.0
            } else {
                (1.0 + out_bounce(2.0 * x - 1.0)) / 2.0
            }
        }
        "in_out_circ" => {
            if x < 0.5 {
                (1.0 - (1.0 - (2.0 * x).powi(2)).sqrt()) / 2.0
            } else {
                ((1.0 - (-2.0 * x + 2.0).powi(2)).sqrt() + 1.0) / 2.0
            }
        }
        "in_out_cubic" => {
            if x < 0.5 {
                4.0 * x * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
            }
        }
        "in_out_elastic" => in_out_elastic(x),
        "in_out_expo" => {
            if x < 0.5 {
                if x == 0.0 {
                    0.0
                } else {
                    2.0_f32.powf(20.0 * x - 10.0) / 2.0
                }
            } else if x == 1.0 {
                1.0
            } else {
                (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
            }
        }
        "in_out_quad" => {
            if x < 0.5 {
                2.0 * x * x
            } else {
                1.0 - (-2.0 * x + 2.0).powi(2) / 2.0
            }
        }
        "in_out_quart" => {
            if x < 0.5 {
                8.0 * x.powi(4)
            } else {
                1.0 - (-2.0 * x + 2.0).powi(4) / 2.0
            }
        }
        "in_out_quint" => {
            if x < 0.5 {
                16.0 * x.powi(5)
            } else {
                1.0 - (-2.0 * x + 2.0).powi(5) / 2.0
            }
        }
        "in_out_sine" => -((std::f32::consts::PI * x).cos() - 1.0) / 2.0,
        "out_back" => 1.0 + 2.70158 * (x - 1.0).powi(3) + 1.70158 * (x - 1.0).powi(2),
        "out_bounce" => out_bounce(x),
        "out_circ" => (1.0 - (x - 1.0).powi(2)).sqrt(),
        "out_cubic" => 1.0 - (1.0 - x).powi(3),
        "out_elastic" => out_elastic(x),
        "out_expo" => {
            if x == 1.0 {
                1.0
            } else {
                1.0 - 2.0_f32.powf(-10.0 * x)
            }
        }
        "out_quad" => 1.0 - (1.0 - x).powi(2),
        "out_quart" => 1.0 - (1.0 - x).powi(4),
        "out_quint" => 1.0 - (1.0 - x).powi(5),
        "out_sine" => (x * std::f32::consts::FRAC_PI_2).sin(),
        _ => return None,
    })
}

fn out_bounce(x: f32) -> f32 {
    if x < 0.363_636_37 {
        7.5625 * x * x
    } else if x < 0.727_272_75 {
        7.5625 * (x - 0.545_454_56).powi(2) + 0.75
    } else if x < 0.909_090_94 {
        7.5625 * (x - 0.818_181_8).powi(2) + 0.9375
    } else {
        7.5625 * (x - 0.954_545_44).powi(2) + 0.984_375
    }
}

fn in_elastic(x: f32) -> f32 {
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let c4 = std::f32::consts::TAU / 3.0;
    -2.0_f32.powf(10.0 * x - 10.0) * ((x * 10.0 - 10.75) * c4).sin()
}

fn out_elastic(x: f32) -> f32 {
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let c4 = std::f32::consts::TAU / 3.0;
    2.0_f32.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0
}

fn in_out_elastic(x: f32) -> f32 {
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let c5 = std::f32::consts::PI * 4.0 / 9.0;
    let sin = ((20.0 * x - 11.125) * c5).sin();
    if x < 0.5 {
        -(2.0_f32.powf(20.0 * x - 10.0) * sin) / 2.0
    } else {
        2.0_f32.powf(-20.0 * x + 10.0) * sin / 2.0 + 1.0
    }
}

fn in_out_back(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * (7.189_819 * x - 2.594_909_4) / 2.0
    } else {
        let delta = 2.0 * x - 2.0;
        (delta * delta * (3.594_909_4 * delta + 2.594_909_4) + 2.0) / 2.0
    }
}

fn sample_cubic_bezier(alpha: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let x_curve = cubic_curve(x1, x2);
    let y_curve = cubic_curve(y1, y2);
    let mut parameter = alpha;
    for _ in 0..4 {
        let gradient = cubic_gradient(x_curve, parameter);
        if gradient < 1.0e-5 {
            break;
        }
        parameter -= (cubic_sample(x_curve, parameter) - alpha) / gradient;
    }
    cubic_sample(y_curve, parameter)
}

fn cubic_curve(first: f32, second: f32) -> [f32; 3] {
    [
        3.0 * first - 3.0 * second + 1.0,
        -6.0 * first + 3.0 * second,
        3.0 * first,
    ]
}

fn cubic_sample(curve: [f32; 3], value: f32) -> f32 {
    ((curve[0] * value + curve[1]) * value + curve[2]) * value
}

fn cubic_gradient(curve: [f32; 3], value: f32) -> f32 {
    (3.0 * curve[0] * value + 2.0 * curve[1]) * value + curve[2]
}

fn track_error(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    field: impl AsRef<str>,
    reason: impl Into<String>,
) -> RegistryProjectionError {
    error(
        &timeline.resource_id,
        format!("tracks.{}.{}", track.attribute, field.as_ref()),
        reason,
    )
}

/// Projects the exact timeline codec's canonical network value.
pub(super) fn decode_timeline_definition(
    resource_id: &str,
    value: &NbtSummaryValue,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<TimelineDefinition, RegistryProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        unreachable!("canonical timeline is a compound");
    };
    let clock = field(fields, "clock").expect("canonical timeline clock");
    let clock_entry = registry_access
        .persistent_holder_from_nbt(WORLD_CLOCK_REGISTRY, clock)
        .map_err(|resolution| error(resource_id, "clock", resolution.to_string()))?;
    let period_ticks = field(fields, "period_ticks")
        .map(|value| checked_int_value(value).expect("canonical period ticks"));
    let tracks = match field(fields, "tracks") {
        Some(NbtSummaryValue::Compound(entries)) => {
            let mut tracks = Vec::with_capacity(entries.len());
            for track in entries {
                tracks.push(decode_track(resource_id, track, registry_access)?);
            }
            tracks
        }
        None => Vec::new(),
        Some(_) => unreachable!("canonical tracks are a compound"),
    };
    Ok(TimelineDefinition {
        resource_id: resource_id.to_string(),
        clock_id: clock_entry.holder_id,
        period_ticks,
        tracks,
    })
}

fn decode_track(
    resource_id: &str,
    track: &NbtSummaryEntry,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<TimelineTrackDefinition, RegistryProjectionError> {
    let attribute = track.name.try_to_string().expect("canonical attribute id");
    let NbtSummaryValue::Compound(fields) = &track.value else {
        unreachable!("canonical track is a compound");
    };
    let modifier = match field(fields, "modifier") {
        Some(NbtSummaryValue::String(value)) => value
            .with_utf8(EnvironmentModifier::from_codec_name)
            .flatten()
            .expect("canonical attribute modifier"),
        None => EnvironmentModifier::Override,
        Some(_) => unreachable!("canonical modifier is a string"),
    };
    let Some(NbtSummaryValue::List(values)) = field(fields, "keyframes") else {
        unreachable!("canonical keyframes are a non-empty list");
    };
    let mut keyframes = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let NbtSummaryValue::Compound(fields) = value else {
            unreachable!("canonical keyframe is a compound");
        };
        let ticks = field(fields, "ticks")
            .and_then(checked_int_value)
            .expect("canonical keyframe ticks");
        let keyframe_value = field(fields, "value").expect("canonical keyframe value");
        let argument =
            decode_timeline_argument(&attribute, modifier, keyframe_value, registry_access)
                .map_err(|projection_error| {
                    error(
                        resource_id,
                        format!("tracks.{attribute}.keyframes[{index}].value"),
                        projection_error.to_string(),
                    )
                })?;
        keyframes.push(TimelineKeyframe { ticks, argument });
    }
    let easing = match field(fields, "ease") {
        None => TimelineEasing::Simple("linear".to_string()),
        Some(NbtSummaryValue::String(value)) => {
            TimelineEasing::Simple(value.try_to_string().expect("validated easing name"))
        }
        Some(NbtSummaryValue::Compound(easing)) => decode_cubic_bezier(easing),
        Some(_) => unreachable!("canonical easing is a name or cubic_bezier compound"),
    };
    Ok(TimelineTrackDefinition {
        attribute,
        modifier,
        keyframes,
        easing,
    })
}

fn decode_cubic_bezier(fields: &[NbtSummaryEntry]) -> TimelineEasing {
    let Some(NbtSummaryValue::List(values)) = field(fields, "cubic_bezier") else {
        unreachable!("canonical bezier controls are a four-float list");
    };
    let bits = |index: usize| {
        let NbtSummaryValue::Float(bits) = &values[index] else {
            unreachable!("canonical bezier control is a Float");
        };
        *bits
    };
    TimelineEasing::CubicBezier {
        x1_bits: bits(0),
        y1_bits: bits(1),
        x2_bits: bits(2),
        y2_bits: bits(3),
    }
}

fn error(
    resource_id: impl Into<String>,
    field: impl Into<String>,
    reason: impl Into<String>,
) -> RegistryProjectionError {
    RegistryProjectionError {
        resource_id: resource_id.into(),
        field: field.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClientEnvironmentAttributes, RegistryPacketEntry, WorldCardinalLighting, WorldLevelInfo,
        WorldSkybox,
    };

    #[test]
    fn projects_known_pack_timeline_against_reordered_session_clocks() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test("example:clock", b"{}".to_vec()),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "minecraft:overworld",
                    b"{}".to_vec(),
                ),
                RegistryPacketEntry::with_raw_data("example:network_clock", vec![10, 0]),
            ],
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:visuals",
                    br##"{
                    "clock": "example:clock",
                    "period_ticks": 24000,
                    "tracks": {
                        "minecraft:visual/sky_light_factor": {
                            "modifier": "multiply",
                            "keyframes": [
                                {"ticks": 0, "value": 1.0},
                                {"ticks": 12000, "value": 0.25}
                            ],
                            "ease": {"cubic_bezier": [0.25, 0.1, 0.75, 0.9]}
                        }
                    },
                    "time_markers": {
                        "example:noon": {"ticks": 6000, "show_in_commands": true},
                        "example:night": 13000
                    }
                }"##
                    .to_vec(),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:network",
                    network_timeline_nbt("example:network_clock"),
                ),
            ],
        );

        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        let timeline = store.synchronized_timeline_definition(0).unwrap();
        assert_eq!(timeline.resource_id, "example:visuals");
        assert_eq!(timeline.clock_id, 0);
        assert_eq!(timeline.period_ticks, Some(24_000));
        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(
            timeline.tracks[0].attribute,
            "minecraft:visual/sky_light_factor"
        );
        assert_eq!(timeline.tracks[0].modifier, EnvironmentModifier::Multiply);
        assert_eq!(timeline.tracks[0].keyframes.len(), 2);
        assert!(matches!(
            timeline.tracks[0].easing,
            TimelineEasing::CubicBezier { .. }
        ));
        let network = store.synchronized_timeline_definition(1).unwrap();
        assert_eq!(network.clock_id, 2);
    }

    #[test]
    fn rejects_invalid_period_keyframes_markers_and_clock_references() {
        let invalid = [
            (
                "example:period",
                r#"{"clock":"example:clock","period_ticks":0}"#,
            ),
            (
                "example:keyframe",
                r#"{"clock":"example:clock","period_ticks":10,"tracks":{"example:value":{"keyframes":[{"ticks":11,"value":1}]}}}"#,
            ),
            (
                "example:marker",
                r#"{"clock":"example:clock","period_ticks":10,"time_markers":{"example:end":10}}"#,
            ),
            ("example:clock_ref", r#"{"clock":"example:missing"}"#),
        ];
        for (id, json) in invalid {
            let mut store = WorldStore::new();
            store.record_registry_entries(
                WORLD_CLOCK_REGISTRY,
                0,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:clock",
                    b"{}".to_vec(),
                )],
            );
            store.record_registry_entries(
                TIMELINE_REGISTRY,
                0,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    id,
                    json.as_bytes().to_vec(),
                )],
            );
            store.begin_registry_configuration();
            assert!(store.finish_registry_configuration().is_err());
        }
    }

    #[test]
    fn samples_float_tracks_by_clock_period_and_dimension_layer_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:clock",
                b"{}".to_vec(),
            )],
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:override",
                    br#"{
                        "clock":"example:clock",
                        "period_ticks":200,
                        "tracks":{
                            "minecraft:visual/sky_light_factor":{
                                "keyframes":[
                                    {"ticks":0,"value":0.0},
                                    {"ticks":100,"value":1.0}
                                ]
                            }
                        }
                    }"#
                    .to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:multiply",
                    br#"{
                        "clock":"example:clock",
                        "tracks":{
                            "minecraft:visual/sky_light_factor":{
                                "modifier":"multiply",
                                "keyframes":[{"ticks":0,"value":0.5}],
                                "ease":"constant"
                            }
                        }
                    }"#
                    .to_vec(),
                ),
            ],
        );
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        store.level = Some(test_level_info(vec![
            "example:override".to_string(),
            "example:multiply".to_string(),
        ]));
        store.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 250,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 250,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        });

        let sampled = store.sampled_client_environment_attributes().unwrap();
        assert_eq!(sampled.sky_light_factor, Some(0.25));

        store.level.as_mut().unwrap().timeline_resource_ids = vec![
            "example:multiply".to_string(),
            "example:override".to_string(),
        ];
        let reordered = store.sampled_client_environment_attributes().unwrap();
        assert_eq!(reordered.sky_light_factor, Some(0.5));
    }

    #[test]
    fn validates_then_filters_non_syncable_tracks() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:clock",
                b"{}".to_vec(),
            )],
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:valid",
                br#"{
                        "clock":"example:clock",
                        "tracks":{
                            "minecraft:gameplay/villager_activity":{
                                "keyframes":[{"ticks":0,"value":"idle"}]
                            },
                            "minecraft:visual/sky_light_factor":{
                                "keyframes":[{"ticks":0,"value":0.5}]
                            }
                        }
                    }"#
                .to_vec(),
            )],
        );

        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        let valid = store.synchronized_timeline_definition(0).unwrap();
        assert_eq!(valid.tracks.len(), 1);
        assert_eq!(
            valid.tracks[0].attribute,
            "minecraft:visual/sky_light_factor"
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:invalid",
                br#"{
                        "clock":"example:clock",
                        "tracks":{
                            "minecraft:gameplay/villager_activity":{
                                "keyframes":[{"ticks":0,"value":"example:unknown"}]
                            }
                        }
                    }"#
                .to_vec(),
            )],
        );
        store.begin_registry_configuration();
        assert!(store.finish_registry_configuration().is_err());
    }

    #[test]
    fn sanitizes_only_after_all_timeline_layers() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:clock",
                b"{}".to_vec(),
            )],
        );
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:subtract",
                br#"{
                    "clock":"example:clock",
                    "tracks":{
                        "minecraft:visual/sky_light_factor":{
                            "modifier":"subtract",
                            "keyframes":[{"ticks":0,"value":1.0}]
                        }
                    }
                }"#
                .to_vec(),
            )],
        );
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();
        let mut level = test_level_info(vec!["example:subtract".to_string()]);
        level.environment_attributes.sky_light_factor = Some(2.0);
        store.level = Some(level);

        let sampled = store.sampled_client_environment_attributes().unwrap();
        assert_eq!(sampled.sky_light_factor, Some(1.0));
    }

    fn network_timeline_nbt(clock: &str) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "clock");
        write_nbt_string(&mut out, clock);
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn test_level_info(timeline_resource_ids: Vec<String>) -> WorldLevelInfo {
        WorldLevelInfo {
            dimension: "example:dimension".to_string(),
            dimension_type_id: 0,
            dimension_type_name: Some("example:dimension_type".to_string()),
            dimension_type_properties: crate::WorldDimensionTypeProperties::default(),
            biome_zoom_seed: 0,
            default_clock_resource_id: Some("example:clock".to_string()),
            timeline_resource_ids,
            has_sky_light: true,
            has_ceiling: false,
            cardinal_lighting: WorldCardinalLighting::Default,
            skybox: WorldSkybox::Overworld,
            ambient_light: 0.0,
            environment_attributes: ClientEnvironmentAttributes::default(),
            last_death_location: None,
            sea_level: 63,
            is_debug: false,
            is_flat: false,
        }
    }
}
