use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::{ClientEnvironmentAttributes, RegistryElementSource, RegistryEntrySource, WorldStore};

use super::normalize_codec_identifier;

pub const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
pub const TIMELINE_REGISTRY: &str = "minecraft:timeline";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClockDefinition {
    pub resource_id: String,
    pub source: RegistryEntrySource,
    pub element_source: Option<RegistryElementSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineKeyframe {
    pub ticks: i32,
    pub value: NbtSummaryValue,
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
        if !matches!(data.value, NbtSummaryValue::Compound(_)) {
            return Err(error(
                entry.resource_id,
                "<root>",
                "WorldClock.DIRECT_CODEC expects a compound",
            ));
        }
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
        let Some(data) = entry.element_data else {
            return Ok(None);
        };
        decode_timeline_definition(entry.resource_id, data.source, &data.value, registry_access)
            .map(Some)
    }

    pub fn sampled_client_environment_attributes(
        &self,
    ) -> Result<ClientEnvironmentAttributes, TimelineProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(ClientEnvironmentAttributes::default());
        };
        let mut attributes = level.environment_attributes();
        for (timeline, total_ticks) in self.resolved_dimension_timelines()? {
            for track in &timeline.tracks {
                apply_float_environment_track(&mut attributes, &timeline, track, total_ticks)?;
            }
        }
        sanitize_float_environment_attributes(&mut attributes);
        Ok(attributes)
    }

    pub fn sampled_timeline_sky_light_factor(
        &self,
    ) -> Result<Option<f32>, TimelineProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(None);
        };
        let mut value = level
            .environment_attributes()
            .sky_light_factor
            .unwrap_or(1.0);
        let mut applied = false;
        for (timeline, total_ticks) in self.resolved_dimension_timelines()? {
            for track in &timeline.tracks {
                if track.attribute != "minecraft:visual/sky_light_factor" {
                    continue;
                }
                let argument = sample_float_track(&timeline, track, total_ticks)?;
                value = apply_float_modifier(&timeline, track, value, argument)?;
                applied = true;
            }
        }
        Ok(applied.then(|| value.clamp(0.0, 1.0)))
    }

    fn resolved_dimension_timelines(
        &self,
    ) -> Result<Vec<(TimelineDefinition, i64)>, TimelineProjectionError> {
        let Some(level) = self.level_info() else {
            return Ok(Vec::new());
        };
        level
            .timeline_resource_ids()
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

fn apply_float_environment_track(
    attributes: &mut ClientEnvironmentAttributes,
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<(), TimelineProjectionError> {
    let (slot, default_value) = match track.attribute.as_str() {
        "minecraft:visual/fog_start_distance" => (&mut attributes.fog_start_distance, 0.0),
        "minecraft:visual/fog_end_distance" => (&mut attributes.fog_end_distance, 1024.0),
        "minecraft:visual/sky_fog_end_distance" => (&mut attributes.sky_fog_end_distance, 512.0),
        "minecraft:visual/cloud_fog_end_distance" => {
            (&mut attributes.cloud_fog_end_distance, 2048.0)
        }
        "minecraft:visual/water_fog_start_distance" => {
            (&mut attributes.water_fog_start_distance, -8.0)
        }
        "minecraft:visual/water_fog_end_distance" => (&mut attributes.water_fog_end_distance, 96.0),
        "minecraft:visual/sky_light_factor" => (&mut attributes.sky_light_factor, 1.0),
        _ => return Ok(()),
    };
    let argument = sample_float_track(timeline, track, total_ticks)?;
    let base = slot.unwrap_or(default_value);
    *slot = Some(apply_float_modifier(timeline, track, base, argument)?);
    Ok(())
}

fn sample_float_track(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    total_ticks: i64,
) -> Result<f32, TimelineProjectionError> {
    let values = track
        .keyframes
        .iter()
        .enumerate()
        .map(|(index, keyframe)| {
            numeric_f32(&keyframe.value)
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    track_error(
                        timeline,
                        track,
                        format!("keyframes[{index}].value"),
                        "float track expects a finite number",
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() == 1 {
        return Ok(values[0]);
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
    } else if sample_ticks <= i64::from(first.ticks) {
        return Ok(values[0]);
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
        return Ok(values[last_index]);
    };
    if sample_ticks <= from_ticks {
        return Ok(values[from_index]);
    }
    if sample_ticks >= to_ticks {
        return Ok(values[to_index]);
    }
    let alpha = (sample_ticks - from_ticks) as f32 / (to_ticks - from_ticks) as f32;
    let alpha = apply_easing(timeline, track, alpha)?;
    Ok(values[from_index] + alpha * (values[to_index] - values[from_index]))
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
            _ => Err(track_error(
                timeline,
                track,
                "ease",
                format!("unsupported easing {name}"),
            )),
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

fn apply_float_modifier(
    timeline: &TimelineDefinition,
    track: &TimelineTrackDefinition,
    base: f32,
    argument: f32,
) -> Result<f32, TimelineProjectionError> {
    Ok(match track.modifier {
        TimelineModifier::Override => argument,
        TimelineModifier::Add => base + argument,
        TimelineModifier::Subtract => base - argument,
        TimelineModifier::Multiply => base * argument,
        TimelineModifier::Minimum => base.min(argument),
        TimelineModifier::Maximum => base.max(argument),
        _ => {
            return Err(track_error(
                timeline,
                track,
                "modifier",
                "modifier does not accept scalar float keyframes",
            ))
        }
    })
}

fn sanitize_float_environment_attributes(attributes: &mut ClientEnvironmentAttributes) {
    attributes.fog_end_distance = attributes.fog_end_distance.map(|value| value.max(0.0));
    attributes.sky_fog_end_distance = attributes.sky_fog_end_distance.map(|value| value.max(0.0));
    attributes.cloud_fog_end_distance = attributes
        .cloud_fog_end_distance
        .map(|value| value.max(0.0));
    attributes.water_fog_end_distance = attributes
        .water_fog_end_distance
        .map(|value| value.max(0.0));
    attributes.sky_light_factor = attributes
        .sky_light_factor
        .map(|value| value.clamp(0.0, 1.0));
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

fn decode_timeline_definition(
    resource_id: &str,
    element_source: RegistryElementSource,
    value: &NbtSummaryValue,
    registry_access: crate::FrozenRegistryAccess<'_>,
) -> Result<TimelineDefinition, TimelineProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error(resource_id, "<root>", "expected compound"));
    };
    let clock_resource_id = required_string(resource_id, fields, "clock")?;
    let clock_resource_id = normalize_codec_identifier(clock_resource_id)
        .ok_or_else(|| error(resource_id, "clock", "invalid world clock identifier"))?;
    let clock_entry = registry_access
        .require_entry_by_resource_id(WORLD_CLOCK_REGISTRY, &clock_resource_id)
        .map_err(|resolution| error(resource_id, "clock", resolution.to_string()))?;
    let period_ticks = optional_positive_int(resource_id, fields, "period_ticks")?;
    let tracks = decode_tracks(resource_id, fields, period_ticks)?;
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
) -> Result<Vec<TimelineTrackDefinition>, TimelineProjectionError> {
    let Some(value) = field(fields, "tracks") else {
        return Ok(Vec::new());
    };
    let NbtSummaryValue::Compound(tracks) = value else {
        return Err(error(resource_id, "tracks", "expected compound map"));
    };
    tracks
        .iter()
        .map(|track| decode_track(resource_id, track, period_ticks))
        .collect()
}

fn decode_track(
    resource_id: &str,
    track: &NbtSummaryEntry,
    period_ticks: Option<i32>,
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
    let mut previous_tick = None;
    let mut same_tick_count = 0usize;
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
        match previous_tick {
            Some(previous) if ticks < previous => {
                return Err(error(
                    resource_id,
                    format!("{keyframe_path}.ticks"),
                    "keyframes must be ordered by ticks",
                ))
            }
            Some(previous) if ticks == previous => {
                same_tick_count += 1;
                if same_tick_count > 2 {
                    return Err(error(
                        resource_id,
                        format!("{keyframe_path}.ticks"),
                        "more than two keyframes share one tick",
                    ));
                }
            }
            _ => same_tick_count = 1,
        }
        previous_tick = Some(ticks);
        let keyframe_value = field(fields, "value").ok_or_else(|| {
            error(
                resource_id,
                format!("{keyframe_path}.value"),
                "missing required field",
            )
        })?;
        keyframes.push(TimelineKeyframe {
            ticks,
            value: keyframe_value.clone(),
        });
    }
    let easing = match field(fields, "ease") {
        None => TimelineEasing::Simple("linear".to_string()),
        Some(NbtSummaryValue::String(value)) => TimelineEasing::Simple(value.clone()),
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
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                error(
                    resource_id,
                    format!("{path}[{index}]"),
                    "expected finite float",
                )
            })?;
    }
    if !(0.0..=1.0).contains(&controls[0]) || !(0.0..=1.0).contains(&controls[2]) {
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
    markers
        .iter()
        .map(|marker| {
            let marker_resource_id = normalize_codec_identifier(&marker.name).ok_or_else(|| {
                error(
                    resource_id,
                    format!("time_markers.{}", marker.name),
                    "invalid time marker identifier",
                )
            })?;
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
            Ok(TimelineTimeMarker {
                resource_id: marker_resource_id,
                ticks,
                show_in_commands,
            })
        })
        .collect()
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

fn required_string<'a>(
    resource_id: &str,
    fields: &'a [NbtSummaryEntry],
    name: &str,
) -> Result<&'a str, TimelineProjectionError> {
    match field(fields, name) {
        Some(NbtSummaryValue::String(value)) => Ok(value),
        Some(_) => Err(error(resource_id, name, "expected string")),
        None => Err(error(resource_id, name, "missing required field")),
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
