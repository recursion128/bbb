use std::collections::BTreeSet;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::{ClientEnvironmentAttributes, RegistryElementSource, RegistryEntrySource, WorldStore};

use super::{
    environment_attributes::{
        apply_timeline_argument, decode_timeline_argument, interpolate_timeline_argument,
        is_syncable_environment_attribute, validate_timeline_argument_references,
        EnvironmentModifier, EnvironmentTrackArgument,
    },
    normalize_codec_identifier,
};

pub const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
pub const TIMELINE_REGISTRY: &str = "minecraft:timeline";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClockDefinition {
    pub resource_id: String,
    pub source: RegistryEntrySource,
    pub element_source: Option<RegistryElementSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineDefinition {
    pub resource_id: String,
    pub element_source: RegistryElementSource,
    pub clock: TimelineClockReference,
    pub period_ticks: Option<i32>,
    pub tracks: Vec<TimelineTrackDefinition>,
    pub time_markers: Vec<TimelineTimeMarker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineClockReference {
    pub holder_id: i32,
    pub resource_id: String,
    pub source: RegistryEntrySource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrackDefinition {
    pub attribute: String,
    pub modifier: TimelineModifier,
    pub keyframes: Vec<TimelineKeyframe>,
    pub easing: TimelineEasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineModifier {
    Override,
    AlphaBlend,
    Add,
    Subtract,
    Multiply,
    BlendToGray,
    Minimum,
    Maximum,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineKeyframe {
    pub ticks: i32,
    pub value: NbtSummaryValue,
    pub(super) argument: EnvironmentTrackArgument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineEasing {
    Simple(String),
    CubicBezier {
        x1_bits: u32,
        y1_bits: u32,
        x2_bits: u32,
        y2_bits: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineTimeMarker {
    pub resource_id: String,
    pub ticks: i32,
    pub show_in_commands: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: String,
}

impl std::fmt::Display for TimelineProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for TimelineProjectionError {}

impl WorldStore {
    pub fn synchronized_world_clock_definition(
        &self,
        holder_id: i32,
    ) -> Result<Option<WorldClockDefinition>, TimelineProjectionError> {
        let Some(entry) = self
            .frozen_registry_access()
            .entry_by_holder_id(WORLD_CLOCK_REGISTRY, holder_id)
        else {
            return Ok(None);
        };
        if let Some(definition) = self.registries.typed.world_clocks.get(entry.resource_id) {
            return Ok(Some(definition.clone()));
        }
        if entry.source == RegistryEntrySource::VanillaFallback {
            return Ok(Some(WorldClockDefinition {
                resource_id: entry.resource_id.to_string(),
                source: entry.source,
                element_source: None,
            }));
        }
        let Some(data) = entry.element_data else {
            return Ok(None);
        };
        validate_world_clock_value(entry.resource_id, &data.value)?;
        Ok(Some(WorldClockDefinition {
            resource_id: entry.resource_id.to_string(),
            source: entry.source,
            element_source: Some(data.source),
        }))
    }

    pub fn synchronized_timeline_definition(
        &self,
        holder_id: i32,
    ) -> Result<Option<TimelineDefinition>, TimelineProjectionError> {
        let registry_access = self.frozen_registry_access();
        let Some(entry) = registry_access.entry_by_holder_id(TIMELINE_REGISTRY, holder_id) else {
            return Ok(None);
        };
        if let Some(definition) = self.registries.typed.timelines.get(entry.resource_id) {
            return Ok(Some(definition.clone()));
        }
        let Some(data) = entry.element_data else {
            return Ok(None);
        };
        decode_timeline_definition(entry.resource_id, data.source, &data.value, registry_access)
            .map(Some)
    }

    pub fn sampled_timeline_sky_light_factor(
        &self,
    ) -> Result<Option<f32>, TimelineProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(None);
        };
        let dimension = self
            .registries
            .dimension_type_client_projection(level.dimension_type_id);
        let mut value = dimension
            .as_ref()
            .and_then(|projection| projection.environment_attributes.sky_light_factor)
            .or_else(|| level.environment_attributes().sky_light_factor)
            .unwrap_or(1.0);
        let mut applied = false;
        for (timeline, total_ticks) in self.resolved_dimension_timelines()? {
            for track in &timeline.tracks {
                if track.attribute != "minecraft:visual/sky_light_factor" {
                    continue;
                }
                let argument = sample_track_argument(&timeline, track, total_ticks)?;
                let mut sampled = ClientEnvironmentAttributes {
                    sky_light_factor: Some(value),
                    ..ClientEnvironmentAttributes::default()
                };
                apply_timeline_argument(
                    &mut sampled,
                    &track.attribute,
                    track.modifier.environment_modifier(),
                    argument,
                )
                .map_err(|error| track_error(&timeline, track, "modifier", error.to_string()))?;
                value = sampled.sky_light_factor.unwrap_or(value);
                applied = true;
            }
        }
        Ok(applied.then(|| value.clamp(0.0, 1.0)))
    }

    pub(super) fn resolved_dimension_timelines(
        &self,
    ) -> Result<Vec<(TimelineDefinition, i64)>, TimelineProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(Vec::new());
        };
        let dimension = self
            .registries
            .dimension_type_client_projection(level.dimension_type_id);
        let timeline_resource_ids = dimension
            .as_ref()
            .map(|projection| projection.timeline_resource_ids.as_slice())
            .unwrap_or_else(|| level.timeline_resource_ids());
        timeline_resource_ids
            .iter()
            .map(|timeline_resource_id| {
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
                    .synchronized_timeline_definition(holder_id)?
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
                            .find(|clock| clock.clock_id == timeline.clock.holder_id)
                    })
                    .map(|clock| clock.total_ticks)
                    .unwrap_or(0);
                Ok((timeline, total_ticks))
            })
            .collect()
    }
}

pub(super) fn validate_world_clock_value(
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<(), TimelineProjectionError> {
    if matches!(value, NbtSummaryValue::Compound(_)) {
        Ok(())
    } else {
        Err(error(
            resource_id,
            "<root>",
            "WorldClock.DIRECT_CODEC expects a compound",
        ))
    }
}

pub(super) fn apply_environment_track(
    attributes: &mut ClientEnvironmentAttributes,
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<(), TimelineProjectionError> {
    let argument = sample_track_argument(timeline, track, total_ticks)?;
    apply_timeline_argument(
        attributes,
        &track.attribute,
        track.modifier.environment_modifier(),
        argument,
    )
    .map_err(|error| track_error(timeline, track, "modifier", error.to_string()))
}

fn sample_track_argument(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<EnvironmentTrackArgument, TimelineProjectionError> {
    let values = track
        .keyframes
        .iter()
        .map(|keyframe| keyframe.argument.clone())
        .collect::<Vec<_>>();
    if values.len() == 1 {
        return Ok(values[0].clone());
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
        return Ok(values[0].clone());
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
        return Ok(values[last_index].clone());
    };
    if sample_ticks <= from_ticks {
        return Ok(values[from_index].clone());
    }
    if sample_ticks >= to_ticks {
        return Ok(values[to_index].clone());
    }
    let alpha = (sample_ticks - from_ticks) as f32 / (to_ticks - from_ticks) as f32;
    let alpha = apply_easing(timeline, track, alpha)?;
    Ok(interpolate_timeline_argument(
        &track.attribute,
        &values[from_index],
        &values[to_index],
        alpha,
    ))
}

fn apply_easing(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    alpha: f32,
) -> Result<f32, TimelineProjectionError> {
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
) -> TimelineProjectionError {
    error(
        &timeline.resource_id,
        format!("tracks.{}.{}", track.attribute, field.as_ref()),
        reason,
    )
}

pub(super) fn decode_timeline_definition(
    resource_id: &str,
    element_source: RegistryElementSource,
    value: &NbtSummaryValue,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<TimelineDefinition, TimelineProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error(resource_id, "<root>", "expected compound"));
    };
    let clock = field(fields, "clock")
        .ok_or_else(|| error(resource_id, "clock", "missing required field"))?;
    let clock_entry = registry_access
        .persistent_holder_from_nbt(WORLD_CLOCK_REGISTRY, clock)
        .map_err(|resolution| error(resource_id, "clock", resolution.to_string()))?;
    let period_ticks = optional_positive_int(resource_id, fields, "period_ticks")?;
    let tracks = decode_tracks(resource_id, fields, period_ticks, registry_access)?;
    for track in &tracks {
        for (index, keyframe) in track.keyframes.iter().enumerate() {
            validate_timeline_argument_references(&keyframe.argument, registry_access).map_err(
                |reference_error| {
                    error(
                        resource_id,
                        format!("tracks.{}.keyframes[{index}].value", track.attribute),
                        reference_error,
                    )
                },
            )?;
        }
    }
    let time_markers = decode_time_markers(resource_id, fields, period_ticks)?;
    Ok(TimelineDefinition {
        resource_id: resource_id.to_string(),
        element_source,
        clock: TimelineClockReference {
            holder_id: clock_entry.holder_id,
            resource_id: clock_entry.resource_id.to_string(),
            source: clock_entry.source,
        },
        period_ticks,
        tracks,
        time_markers,
    })
}

fn decode_tracks(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    period_ticks: Option<i32>,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<Vec<TimelineTrackDefinition>, TimelineProjectionError> {
    let Some(value) = field(fields, "tracks") else {
        return Ok(Vec::new());
    };
    let NbtSummaryValue::Compound(tracks) = value else {
        return Err(error(resource_id, "tracks", "expected compound map"));
    };
    let mut decoded = Vec::with_capacity(tracks.len());
    let mut seen = BTreeSet::new();
    for track in tracks {
        let track = decode_track(resource_id, track, period_ticks, registry_access)?;
        if !seen.insert(track.attribute.clone()) {
            return Err(error(
                resource_id,
                format!("tracks.{}", track.attribute),
                "duplicate environment attribute track",
            ));
        }
        if is_syncable_environment_attribute(&track.attribute) {
            decoded.push(track);
        }
    }
    Ok(decoded)
}

fn decode_track(
    resource_id: &str,
    track: &NbtSummaryEntry,
    period_ticks: Option<i32>,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<TimelineTrackDefinition, TimelineProjectionError> {
    let attribute = normalize_codec_identifier(&track.name).ok_or_else(|| {
        error(
            resource_id,
            format!("tracks.{}", track.name),
            "invalid environment attribute identifier",
        )
    })?;
    let path = format!("tracks.{attribute}");
    let NbtSummaryValue::Compound(fields) = &track.value else {
        return Err(error(resource_id, path, "expected track compound"));
    };
    let modifier = match optional_string(resource_id, fields, "modifier", &path)? {
        Some(value) => TimelineModifier::from_codec_name(value).ok_or_else(|| {
            error(
                resource_id,
                format!("{path}.modifier"),
                "unknown attribute modifier",
            )
        })?,
        None => TimelineModifier::Override,
    };
    let keyframes_value = field(fields, "keyframes").ok_or_else(|| {
        error(
            resource_id,
            format!("{path}.keyframes"),
            "missing required field",
        )
    })?;
    let NbtSummaryValue::List(values) = keyframes_value else {
        return Err(error(
            resource_id,
            format!("{path}.keyframes"),
            "expected list",
        ));
    };
    if values.is_empty() {
        return Err(error(
            resource_id,
            format!("{path}.keyframes"),
            "keyframes must not be empty",
        ));
    }
    let mut keyframes = Vec::with_capacity(values.len());
    let mut ordered_previous_tick = None;
    let mut repeated_previous_tick = values.last().and_then(|value| match value {
        NbtSummaryValue::Compound(fields) => field(fields, "ticks").and_then(numeric_i32),
        _ => None,
    });
    let mut repeat_count = 0usize;
    for (index, value) in values.iter().enumerate() {
        let keyframe_path = format!("{path}.keyframes[{index}]");
        let NbtSummaryValue::Compound(fields) = value else {
            return Err(error(resource_id, keyframe_path, "expected compound"));
        };
        let ticks = required_non_negative_int(resource_id, fields, "ticks", &keyframe_path)?;
        if let Some(period_ticks) = period_ticks {
            if ticks > period_ticks {
                return Err(error(
                    resource_id,
                    format!("{keyframe_path}.ticks"),
                    format!("must be in range [0; {period_ticks}]"),
                ));
            }
        }
        if ordered_previous_tick.is_some_and(|previous| ticks < previous) {
            return Err(error(
                resource_id,
                format!("{keyframe_path}.ticks"),
                "keyframes must be ordered by ticks",
            ));
        }
        match repeated_previous_tick {
            Some(previous) if ticks == previous => {
                repeat_count += 1;
                if repeat_count > 2 {
                    return Err(error(
                        resource_id,
                        format!("{keyframe_path}.ticks"),
                        "more than two keyframes share one tick",
                    ));
                }
            }
            _ => repeat_count = 0,
        }
        ordered_previous_tick = Some(ticks);
        repeated_previous_tick = Some(ticks);
        let keyframe_value = field(fields, "value").ok_or_else(|| {
            error(
                resource_id,
                format!("{keyframe_path}.value"),
                "missing required field",
            )
        })?;
        let argument = decode_timeline_argument(
            &attribute,
            modifier.environment_modifier(),
            keyframe_value,
            registry_access,
        )
        .map_err(|projection_error| {
            error(
                resource_id,
                format!("{keyframe_path}.value"),
                projection_error.to_string(),
            )
        })?;
        keyframes.push(TimelineKeyframe {
            ticks,
            value: keyframe_value.clone(),
            argument,
        });
    }
    let easing = match field(fields, "ease") {
        None => TimelineEasing::Simple("linear".to_string()),
        Some(NbtSummaryValue::String(value)) if is_simple_easing(value) => {
            TimelineEasing::Simple(value.clone())
        }
        Some(NbtSummaryValue::String(_)) => {
            return Err(error(
                resource_id,
                format!("{path}.ease"),
                "unknown easing name",
            ))
        }
        Some(NbtSummaryValue::Compound(easing)) => decode_cubic_bezier(resource_id, easing, &path)?,
        Some(_) => {
            return Err(error(
                resource_id,
                format!("{path}.ease"),
                "expected easing name or cubic_bezier compound",
            ))
        }
    };
    Ok(TimelineTrackDefinition {
        attribute,
        modifier,
        keyframes,
        easing,
    })
}

fn decode_cubic_bezier(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    track_path: &str,
) -> Result<TimelineEasing, TimelineProjectionError> {
    let path = format!("{track_path}.ease.cubic_bezier");
    let Some(NbtSummaryValue::List(values)) = field(fields, "cubic_bezier") else {
        return Err(error(resource_id, path, "expected four-float list"));
    };
    if values.len() != 4 {
        return Err(error(resource_id, path, "expected four-float list"));
    }
    let mut controls = [0.0_f32; 4];
    for (index, value) in values.iter().enumerate() {
        controls[index] = numeric_f32(value)
            .ok_or_else(|| error(resource_id, format!("{path}[{index}]"), "expected float"))?;
    }
    if controls[0] < 0.0 || controls[0] > 1.0 || controls[2] < 0.0 || controls[2] > 1.0 {
        return Err(error(
            resource_id,
            path,
            "x controls must be in range [0; 1]",
        ));
    }
    Ok(TimelineEasing::CubicBezier {
        x1_bits: controls[0].to_bits(),
        y1_bits: controls[1].to_bits(),
        x2_bits: controls[2].to_bits(),
        y2_bits: controls[3].to_bits(),
    })
}

fn decode_time_markers(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    period_ticks: Option<i32>,
) -> Result<Vec<TimelineTimeMarker>, TimelineProjectionError> {
    let Some(value) = field(fields, "time_markers") else {
        return Ok(Vec::new());
    };
    let NbtSummaryValue::Compound(markers) = value else {
        return Err(error(resource_id, "time_markers", "expected compound map"));
    };
    let mut decoded = Vec::with_capacity(markers.len());
    let mut seen = BTreeSet::new();
    for marker in markers {
        let marker_resource_id = normalize_codec_identifier(&marker.name).ok_or_else(|| {
            error(
                resource_id,
                format!("time_markers.{}", marker.name),
                "invalid time marker identifier",
            )
        })?;
        if !seen.insert(marker_resource_id.clone()) {
            return Err(error(
                resource_id,
                format!("time_markers.{marker_resource_id}"),
                "duplicate time marker",
            ));
        }
        let path = format!("time_markers.{marker_resource_id}");
        let (ticks, show_in_commands) = match &marker.value {
            value if numeric_i32(value).is_some() => {
                let ticks = numeric_i32(value).expect("guarded numeric value");
                if ticks < 0 {
                    return Err(error(resource_id, &path, "ticks must be non-negative"));
                }
                (ticks, false)
            }
            NbtSummaryValue::Compound(fields) => (
                required_non_negative_int(resource_id, fields, "ticks", &path)?,
                optional_bool(resource_id, fields, "show_in_commands", &path)?.unwrap_or(false),
            ),
            _ => {
                return Err(error(
                    resource_id,
                    path,
                    "expected non-negative ticks or marker compound",
                ))
            }
        };
        if let Some(period_ticks) = period_ticks {
            if ticks >= period_ticks {
                return Err(error(
                    resource_id,
                    path,
                    format!("ticks must be in range [0; {period_ticks})"),
                ));
            }
        }
        decoded.push(TimelineTimeMarker {
            resource_id: marker_resource_id,
            ticks,
            show_in_commands,
        });
    }
    Ok(decoded)
}

impl TimelineModifier {
    fn from_codec_name(value: &str) -> Option<Self> {
        Some(match value {
            "override" => Self::Override,
            "alpha_blend" => Self::AlphaBlend,
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "multiply" => Self::Multiply,
            "blend_to_gray" => Self::BlendToGray,
            "minimum" => Self::Minimum,
            "maximum" => Self::Maximum,
            "and" => Self::And,
            "nand" => Self::Nand,
            "or" => Self::Or,
            "nor" => Self::Nor,
            "xor" => Self::Xor,
            "xnor" => Self::Xnor,
            _ => return None,
        })
    }

    fn environment_modifier(self) -> EnvironmentModifier {
        match self {
            Self::Override => EnvironmentModifier::Override,
            Self::AlphaBlend => EnvironmentModifier::AlphaBlend,
            Self::Add => EnvironmentModifier::Add,
            Self::Subtract => EnvironmentModifier::Subtract,
            Self::Multiply => EnvironmentModifier::Multiply,
            Self::BlendToGray => EnvironmentModifier::BlendToGray,
            Self::Minimum => EnvironmentModifier::Minimum,
            Self::Maximum => EnvironmentModifier::Maximum,
            Self::And => EnvironmentModifier::And,
            Self::Nand => EnvironmentModifier::Nand,
            Self::Or => EnvironmentModifier::Or,
            Self::Nor => EnvironmentModifier::Nor,
            Self::Xor => EnvironmentModifier::Xor,
            Self::Xnor => EnvironmentModifier::Xnor,
        }
    }
}

fn is_simple_easing(value: &str) -> bool {
    matches!(
        value,
        "constant"
            | "linear"
            | "in_back"
            | "in_bounce"
            | "in_circ"
            | "in_cubic"
            | "in_elastic"
            | "in_expo"
            | "in_quad"
            | "in_quart"
            | "in_quint"
            | "in_sine"
            | "in_out_back"
            | "in_out_bounce"
            | "in_out_circ"
            | "in_out_cubic"
            | "in_out_elastic"
            | "in_out_expo"
            | "in_out_quad"
            | "in_out_quart"
            | "in_out_quint"
            | "in_out_sine"
            | "out_back"
            | "out_bounce"
            | "out_circ"
            | "out_cubic"
            | "out_elastic"
            | "out_expo"
            | "out_quad"
            | "out_quart"
            | "out_quint"
            | "out_sine"
    )
}

fn optional_positive_int(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    name: &str,
) -> Result<Option<i32>, TimelineProjectionError> {
    match field(fields, name) {
        None => Ok(None),
        Some(value) => match numeric_i32(value) {
            Some(value) if value > 0 => Ok(Some(value)),
            _ => Err(error(resource_id, name, "expected positive integer")),
        },
    }
}

fn required_non_negative_int(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    name: &str,
    parent: &str,
) -> Result<i32, TimelineProjectionError> {
    match field(fields, name).and_then(numeric_i32) {
        Some(value) if value >= 0 => Ok(value),
        Some(_) => Err(error(
            resource_id,
            format!("{parent}.{name}"),
            "expected non-negative integer",
        )),
        None => Err(error(
            resource_id,
            format!("{parent}.{name}"),
            "missing or invalid non-negative integer",
        )),
    }
}

fn optional_string<'a>(
    resource_id: &str,
    fields: &'a [NbtSummaryEntry],
    name: &str,
    parent: &str,
) -> Result<Option<&'a str>, TimelineProjectionError> {
    match field(fields, name) {
        Some(NbtSummaryValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(
            resource_id,
            format!("{parent}.{name}"),
            "expected string",
        )),
        None => Ok(None),
    }
}

fn optional_bool(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    name: &str,
    parent: &str,
) -> Result<Option<bool>, TimelineProjectionError> {
    match field(fields, name) {
        None => Ok(None),
        Some(value) => bool_value(value)
            .map(Some)
            .ok_or_else(|| error(resource_id, format!("{parent}.{name}"), "expected boolean")),
    }
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

fn bool_value(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        _ => return None,
    })
}

fn error(
    resource_id: impl Into<String>,
    field: impl Into<String>,
    reason: impl Into<String>,
) -> TimelineProjectionError {
    TimelineProjectionError {
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

        let clock = store
            .synchronized_world_clock_definition(0)
            .unwrap()
            .unwrap();
        assert_eq!(clock.resource_id, "example:clock");
        assert_eq!(
            clock.element_source,
            Some(RegistryElementSource::KnownPackJson)
        );

        let timeline = store.synchronized_timeline_definition(0).unwrap().unwrap();
        assert_eq!(timeline.resource_id, "example:visuals");
        assert_eq!(timeline.clock.holder_id, 0);
        assert_eq!(timeline.clock.resource_id, "example:clock");
        assert_eq!(timeline.period_ticks, Some(24_000));
        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(
            timeline.tracks[0].attribute,
            "minecraft:visual/sky_light_factor"
        );
        assert_eq!(timeline.tracks[0].modifier, TimelineModifier::Multiply);
        assert_eq!(timeline.tracks[0].keyframes.len(), 2);
        assert!(matches!(
            timeline.tracks[0].easing,
            TimelineEasing::CubicBezier { .. }
        ));
        assert_eq!(timeline.time_markers.len(), 2);
        assert!(
            timeline
                .time_markers
                .iter()
                .find(|marker| marker.resource_id == "example:noon")
                .unwrap()
                .show_in_commands
        );
        assert_eq!(
            timeline
                .time_markers
                .iter()
                .find(|marker| marker.resource_id == "example:night")
                .unwrap()
                .ticks,
            13_000
        );

        let network = store.synchronized_timeline_definition(1).unwrap().unwrap();
        assert_eq!(network.element_source, RegistryElementSource::NetworkNbt);
        assert_eq!(network.clock.holder_id, 2);
        assert_eq!(network.clock.resource_id, "example:network_clock");
    }

    #[test]
    fn rejects_invalid_period_keyframes_markers_and_clock_references() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            WORLD_CLOCK_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:clock",
                b"{}".to_vec(),
            )],
        );
        let invalid = [
            (
                "example:period",
                r#"{"clock":"example:clock","period_ticks":0}"#,
                "period_ticks: expected positive integer",
            ),
            (
                "example:keyframe",
                r#"{"clock":"example:clock","period_ticks":10,"tracks":{"example:value":{"keyframes":[{"ticks":11,"value":1}]}}}"#,
                "must be in range [0; 10]",
            ),
            (
                "example:marker",
                r#"{"clock":"example:clock","period_ticks":10,"time_markers":{"example:end":10}}"#,
                "ticks must be in range [0; 10)",
            ),
            (
                "example:clock_ref",
                r#"{"clock":"example:missing"}"#,
                "missing minecraft:world_clock resource example:missing",
            ),
        ];
        store.record_registry_entries(
            TIMELINE_REGISTRY,
            0,
            invalid
                .iter()
                .map(|(id, json, _)| {
                    RegistryPacketEntry::with_known_pack_data_for_test(
                        *id,
                        json.as_bytes().to_vec(),
                    )
                })
                .collect(),
        );

        for (holder_id, (_, _, expected)) in invalid.iter().enumerate() {
            let error = store
                .synchronized_timeline_definition(holder_id as i32)
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error:?}");
        }
    }

    #[test]
    fn duplicate_tick_validation_matches_vanilla_counter_order() {
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
                    "example:ordinary_run",
                    br#"{
                        "clock":"example:clock",
                        "tracks":{"minecraft:visual/cloud_height":{"keyframes":[
                            {"ticks":0,"value":1.0},
                            {"ticks":0,"value":2.0},
                            {"ticks":0,"value":3.0},
                            {"ticks":1,"value":4.0}
                        ]}}
                    }"#
                    .to_vec(),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:all_same",
                    br#"{
                        "clock":"example:clock",
                        "tracks":{"minecraft:visual/cloud_height":{"keyframes":[
                            {"ticks":0,"value":1.0},
                            {"ticks":0,"value":2.0},
                            {"ticks":0,"value":3.0}
                        ]}}
                    }"#
                    .to_vec(),
                ),
            ],
        );

        assert!(store.synchronized_timeline_definition(0).is_ok());
        assert!(store.synchronized_timeline_definition(1).is_err());
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
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
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
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
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
                ),
            ],
        );

        let valid = store.synchronized_timeline_definition(0).unwrap().unwrap();
        assert_eq!(valid.tracks.len(), 1);
        assert_eq!(
            valid.tracks[0].attribute,
            "minecraft:visual/sky_light_factor"
        );
        assert!(store.synchronized_timeline_definition(1).is_err());
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
