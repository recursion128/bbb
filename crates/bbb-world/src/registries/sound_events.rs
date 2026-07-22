use bbb_protocol::{
    sound_event_types::vanilla_sound_event_registry_id, NbtSummaryEntry, NbtSummaryValue,
};

use crate::FrozenRegistryAccess;

use super::normalize_codec_identifier;

const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PersistentSoundEvent {
    Named {
        holder_id: i32,
        resource_id: String,
    },
    Direct {
        resource_id: String,
        fixed_range: Option<f32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistentSoundEventDecodeError {
    pub field: &'static str,
    pub reason: String,
}

pub(crate) fn decode_persistent_sound_event(
    value: &NbtSummaryValue,
    access: Option<FrozenRegistryAccess<'_>>,
) -> Result<PersistentSoundEvent, PersistentSoundEventDecodeError> {
    match value {
        NbtSummaryValue::String(raw) => {
            let resource_id = normalize_codec_identifier(raw)
                .ok_or_else(|| error("holder", format!("invalid resource identifier {raw}")))?;
            let holder_id = match access {
                Some(access) => access
                    .persistent_holder_from_nbt(SOUND_EVENT_REGISTRY, value)
                    .map(|entry| entry.holder_id)
                    .map_err(|failure| error("holder", failure.to_string()))?,
                None => vanilla_sound_event_registry_id(&resource_id).ok_or_else(|| {
                    error(
                        "holder",
                        format!("missing {SOUND_EVENT_REGISTRY} resource {resource_id}"),
                    )
                })?,
            };
            Ok(PersistentSoundEvent::Named {
                holder_id,
                resource_id,
            })
        }
        NbtSummaryValue::Compound(entries) => decode_direct(entries),
        _ => Err(error(
            "holder",
            "expected resource identifier or direct compound",
        )),
    }
}

fn decode_direct(
    entries: &[NbtSummaryEntry],
) -> Result<PersistentSoundEvent, PersistentSoundEventDecodeError> {
    let sound_id = match field(entries, "sound_id") {
        Some(NbtSummaryValue::String(raw)) => normalize_codec_identifier(raw)
            .ok_or_else(|| error("sound_id", format!("invalid resource identifier {raw}")))?,
        Some(_) => return Err(error("sound_id", "expected resource identifier")),
        None => return Err(error("sound_id", "missing required field")),
    };
    // SoundEvent.DIRECT_CODEC deliberately uses lenientOptionalFieldOf: an
    // unreadable optional range is treated as absent rather than rejecting the
    // enclosing holder.
    let fixed_range = field(entries, "range").and_then(numeric_f32);
    Ok(PersistentSoundEvent::Direct {
        resource_id: sound_id,
        fixed_range,
    })
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
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

fn error(field: &'static str, reason: impl Into<String>) -> PersistentSoundEventDecodeError {
    PersistentSoundEventDecodeError {
        field,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldStore;

    #[test]
    fn decodes_named_and_direct_sound_events_through_one_registry_boundary() {
        let fallback = decode_persistent_sound_event(
            &NbtSummaryValue::String("ambient.cave".to_string()),
            None,
        )
        .unwrap();
        assert!(matches!(
            fallback,
            PersistentSoundEvent::Named { resource_id, .. }
                if resource_id == "minecraft:ambient.cave"
        ));

        let world = WorldStore::new();
        let named = decode_persistent_sound_event(
            &NbtSummaryValue::String("minecraft:entity.item.break".to_string()),
            Some(world.frozen_registry_access()),
        )
        .unwrap();
        assert_eq!(
            named,
            PersistentSoundEvent::Named {
                holder_id: vanilla_sound_event_registry_id("minecraft:entity.item.break").unwrap(),
                resource_id: "minecraft:entity.item.break".to_string(),
            }
        );

        let direct = decode_persistent_sound_event(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "sound_id".to_string(),
                    value: NbtSummaryValue::String("example:signal".to_string()),
                },
                NbtSummaryEntry {
                    name: "range".to_string(),
                    value: NbtSummaryValue::String("lenient".to_string()),
                },
            ]),
            Some(world.frozen_registry_access()),
        )
        .unwrap();
        assert_eq!(
            direct,
            PersistentSoundEvent::Direct {
                resource_id: "example:signal".to_string(),
                fixed_range: None,
            }
        );
    }

    #[test]
    fn rejects_unknown_named_and_malformed_direct_sound_events() {
        assert_eq!(
            decode_persistent_sound_event(
                &NbtSummaryValue::String("example:missing".to_string()),
                None,
            )
            .unwrap_err()
            .reason,
            "missing minecraft:sound_event resource example:missing"
        );
        assert_eq!(
            decode_persistent_sound_event(&NbtSummaryValue::Compound(Vec::new()), None,)
                .unwrap_err(),
            PersistentSoundEventDecodeError {
                field: "sound_id",
                reason: "missing required field".to_string(),
            }
        );
    }
}
