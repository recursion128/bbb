use bbb_protocol::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_component_nbt,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};

use crate::{RegistryElementSource, WorldStore};

use super::normalize_codec_identifier;

pub const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";

#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentSoundEvent {
    Reference {
        resource_id: String,
    },
    Direct {
        resource_id: String,
        fixed_range: Option<f32>,
    },
}

impl InstrumentSoundEvent {
    pub fn resource_id(&self) -> &str {
        match self {
            Self::Reference { resource_id } | Self::Direct { resource_id, .. } => resource_id,
        }
    }

    pub fn fixed_range(&self) -> Option<f32> {
        match self {
            Self::Reference { .. } => None,
            Self::Direct { fixed_range, .. } => *fixed_range,
        }
    }
}

/// Client-consumed fields from vanilla 26.1 `Instrument.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentPresentation {
    pub element_source: RegistryElementSource,
    pub sound_event: InstrumentSoundEvent,
    pub use_duration: f32,
    pub range: f32,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    pub description_nbt: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for InstrumentProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for InstrumentProjectionError {}

impl WorldStore {
    /// Projects one instrument through the current synchronized holder order.
    ///
    /// This method never interprets a server holder through the vanilla bootstrap
    /// ordinal. Data-less stubs and absent registries remain explicit `None` so the
    /// consumer can apply only its documented source-aware fallback.
    pub fn synchronized_instrument_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<InstrumentPresentation>, InstrumentProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(INSTRUMENT_REGISTRY, holder_id) else {
            return Ok(None);
        };
        if let Some(presentation) = self.registries.typed.instruments.get(&entry.id) {
            return Ok(Some(presentation.clone()));
        }
        let Some(data) = entry.element_data() else {
            if entry.has_data {
                return Err(InstrumentProjectionError {
                    resource_id: entry.id.clone(),
                    field: "<root>".to_string(),
                    reason: "typed payload unavailable",
                });
            }
            return Ok(None);
        };
        decode_instrument(&data.value, data.source)
            .map(Some)
            .map_err(|failure| InstrumentProjectionError {
                resource_id: entry.id.clone(),
                field: failure.field.to_string(),
                reason: failure.reason,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DecodeFailure {
    pub field: &'static str,
    pub reason: &'static str,
}

pub(super) fn decode_instrument(
    value: &NbtSummaryValue,
    element_source: RegistryElementSource,
) -> Result<InstrumentPresentation, DecodeFailure> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(failure("<root>", "expected compound"));
    };
    let sound_event = decode_sound_event(required_field(entries, "sound_event")?)?;
    let use_duration = required_positive_float(entries, "use_duration")?;
    let range = required_positive_float(entries, "range")?;
    let description_nbt = required_field(entries, "description")?.clone();
    validate_component_nbt(&description_nbt, "instrument description")
        .map_err(|_| failure("description", "invalid Component"))?;
    let description_styled = styled_component_runs_from_nbt_value(&description_nbt);
    let description = styled_runs_summary_text(&description_styled);
    Ok(InstrumentPresentation {
        element_source,
        sound_event,
        use_duration,
        range,
        description,
        description_styled,
        description_nbt,
    })
}

fn decode_sound_event(value: &NbtSummaryValue) -> Result<InstrumentSoundEvent, DecodeFailure> {
    match value {
        NbtSummaryValue::String(resource_id) => Ok(InstrumentSoundEvent::Reference {
            resource_id: normalize_codec_identifier(resource_id)
                .ok_or_else(|| failure("sound_event", "invalid reference identifier"))?,
        }),
        NbtSummaryValue::Compound(entries) => {
            let resource_id = match field(entries, "sound_id") {
                Some(NbtSummaryValue::String(resource_id)) => {
                    normalize_codec_identifier(resource_id)
                        .ok_or_else(|| failure("sound_event.sound_id", "invalid identifier"))?
                }
                Some(_) => return Err(failure("sound_event.sound_id", "expected string")),
                None => return Err(failure("sound_event.sound_id", "missing required field")),
            };
            let fixed_range = field(entries, "range")
                .map(|value| {
                    numeric_f32(value)
                        .ok_or_else(|| failure("sound_event.range", "expected number"))
                })
                .transpose()?;
            Ok(InstrumentSoundEvent::Direct {
                resource_id,
                fixed_range,
            })
        }
        _ => Err(failure(
            "sound_event",
            "expected resource identifier or direct compound",
        )),
    }
}

fn required_positive_float(
    entries: &[NbtSummaryEntry],
    name: &'static str,
) -> Result<f32, DecodeFailure> {
    let value = numeric_f32(required_field(entries, name)?)
        .ok_or_else(|| failure(name, "expected number"))?;
    if value > 0.0 && value <= f32::MAX {
        Ok(value)
    } else {
        Err(failure(name, "float outside (0, Float.MAX_VALUE]"))
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &'static str,
) -> Result<&'a NbtSummaryValue, DecodeFailure> {
    field(entries, name).ok_or_else(|| failure(name, "missing required field"))
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

const fn failure(field: &'static str, reason: &'static str) -> DecodeFailure {
    DecodeFailure { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn projects_network_and_known_pack_instruments_in_session_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            INSTRUMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:network_horn",
                    instrument_nbt("example:network", 7.0, 128.0, "Network horn"),
                ),
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:known_horn",
                    br#"{
                        "sound_event":{"sound_id":"example:known","range":24.0},
                        "use_duration":3.5,
                        "range":64.0,
                        "description":{"text":"Known horn","italic":true}
                    }"#
                    .to_vec(),
                ),
            ],
        );

        let network = store
            .synchronized_instrument_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(network.element_source, RegistryElementSource::NetworkNbt);
        assert_eq!(network.sound_event.resource_id(), "example:network");
        assert_eq!(network.use_duration, 7.0);
        assert_eq!(network.range, 128.0);
        assert_eq!(network.description, "Network horn");

        let known = store
            .synchronized_instrument_presentation(1)
            .unwrap()
            .unwrap();
        assert_eq!(known.element_source, RegistryElementSource::KnownPackJson);
        assert_eq!(known.sound_event.resource_id(), "example:known");
        assert_eq!(known.sound_event.fixed_range(), Some(24.0));
        assert_eq!(known.description_styled[0].style.italic, Some(true));
    }

    #[test]
    fn rejects_invalid_instrument_fields_without_bootstrap_fallback() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            INSTRUMENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:broken",
                    instrument_nbt("example:sound", 0.0, 16.0, "Broken"),
                ),
                RegistryPacketEntry::stub("minecraft:sing_goat_horn"),
            ],
        );

        assert_eq!(
            store.synchronized_instrument_presentation(0).unwrap_err(),
            InstrumentProjectionError {
                resource_id: "example:broken".to_string(),
                field: "use_duration".to_string(),
                reason: "float outside (0, Float.MAX_VALUE]",
            }
        );
        assert_eq!(store.synchronized_instrument_presentation(1), Ok(None));
        assert_eq!(store.synchronized_instrument_presentation(2), Ok(None));
    }

    fn instrument_nbt(
        sound_event: &str,
        use_duration: f32,
        range: f32,
        description: &str,
    ) -> Vec<u8> {
        let mut out = vec![10, 8];
        write_nbt_string(&mut out, "sound_event");
        write_nbt_string(&mut out, sound_event);
        out.push(5);
        write_nbt_string(&mut out, "use_duration");
        out.extend_from_slice(&use_duration.to_bits().to_be_bytes());
        out.push(5);
        write_nbt_string(&mut out, "range");
        out.extend_from_slice(&range.to_bits().to_be_bytes());
        out.push(8);
        write_nbt_string(&mut out, "description");
        write_nbt_string(&mut out, description);
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
