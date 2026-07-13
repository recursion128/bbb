use bbb_protocol::{
    sound_event_types::vanilla_sound_event_registry_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::component_f32_bits;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogSoundEventHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        sound_id: String,
        #[serde(default)]
        fixed_range_bits: Option<u32>,
    },
}

pub(super) fn decode_sound_event_holder(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogSoundEventHolderState, String> {
    match value {
        NbtSummaryValue::String(raw) => decode_named_sound_event(raw, what),
        NbtSummaryValue::Compound(entries) => decode_direct_sound_event(entries, what),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct sound event"
        )),
    }
}

fn decode_named_sound_event(raw: &str, what: &str) -> Result<DialogSoundEventHolderState, String> {
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let registry_id = vanilla_sound_event_registry_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:sound_event resource {resource_id} for {what}")
    })?;
    Ok(DialogSoundEventHolderState::Named {
        registry_id,
        resource_id,
    })
}

fn decode_direct_sound_event(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogSoundEventHolderState, String> {
    let sound_id = entries
        .iter()
        .find(|entry| entry.name == "sound_id")
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} direct holder is missing field sound_id"))?;
    let NbtSummaryValue::String(raw_sound_id) = sound_id else {
        return Err(format!(
            "{what} direct holder field sound_id is not a resource id"
        ));
    };
    let sound_id = normalize_codec_identifier(raw_sound_id)
        .ok_or_else(|| format!("invalid {what} direct holder field sound_id {raw_sound_id}"))?;
    let fixed_range_bits = entries
        .iter()
        .find(|entry| entry.name == "range")
        .and_then(|entry| {
            component_f32_bits(&entry.value, &format!("{what} direct holder field range")).ok()
        });
    Ok(DialogSoundEventHolderState::Direct {
        sound_id,
        fixed_range_bits,
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        sound_event_types::vanilla_sound_event_registry_id,
    };

    use super::super::{decode_supported_component_value, DialogDataComponentValueState};
    use super::*;

    #[test]
    fn decodes_named_and_direct_break_sound_holders() {
        let named = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &NbtSummaryValue::String("entity.item.break".to_string()),
            None,
        )
        .unwrap();
        assert!(matches!(
            named,
            Some(DialogDataComponentValueState::BreakSound {
                sound: DialogSoundEventHolderState::Named {
                    registry_id,
                    resource_id,
                },
            }) if registry_id == vanilla_sound_event_registry_id("minecraft:entity.item.break").unwrap()
                && resource_id == "minecraft:entity.item.break"
        ));

        let direct = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &compound([
                (
                    "sound_id",
                    NbtSummaryValue::String("example:glass".to_string()),
                ),
                ("range", NbtSummaryValue::Double(12.5f64.to_bits())),
            ]),
            None,
        )
        .unwrap();
        assert!(matches!(
            direct,
            Some(DialogDataComponentValueState::BreakSound {
                sound: DialogSoundEventHolderState::Direct {
                    sound_id,
                    fixed_range_bits: Some(bits),
                },
            }) if sound_id == "example:glass" && f32::from_bits(bits) == 12.5
        ));
    }

    #[test]
    fn direct_break_sound_range_is_lenient_and_optional() {
        for (range, expected) in [
            (None, None),
            (Some(NbtSummaryValue::Int(8)), Some(8.0f32.to_bits())),
            (Some(NbtSummaryValue::String("invalid".to_string())), None),
        ] {
            let mut entries = vec![(
                "sound_id",
                NbtSummaryValue::String("minecraft:entity.item.break".to_string()),
            )];
            if let Some(range) = range {
                entries.push(("range", range));
            }
            let decoded = decode_sound_event_holder(&compound(entries), "break sound").unwrap();
            assert!(matches!(
                decoded,
                DialogSoundEventHolderState::Direct {
                    fixed_range_bits,
                    ..
                } if fixed_range_bits == expected
            ));
        }
    }

    #[test]
    fn rejects_malformed_or_unknown_break_sound_holders() {
        for (value, expected) in [
            (
                NbtSummaryValue::String("minecraft:missing".to_string()),
                "missing minecraft:sound_event resource",
            ),
            (NbtSummaryValue::String("Invalid".to_string()), "invalid"),
            (NbtSummaryValue::Int(0), "neither a resource id"),
            (compound([]), "missing field sound_id"),
            (
                compound([("sound_id", NbtSummaryValue::Int(0))]),
                "field sound_id is not a resource id",
            ),
            (
                compound([("sound_id", NbtSummaryValue::String("Invalid".to_string()))]),
                "invalid",
            ),
        ] {
            let error = decode_sound_event_holder(&value, "break sound").unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    fn compound<'a>(
        entries: impl IntoIterator<Item = (&'a str, NbtSummaryValue)>,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.to_string(),
                    value,
                })
                .collect(),
        )
    }
}
