use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::{RegistryElementSource, RegistryEntrySource, WorldStore};

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
    use crate::RegistryPacketEntry;

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
}
