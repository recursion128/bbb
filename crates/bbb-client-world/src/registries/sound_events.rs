use bbb_protocol::nbt_summary::{codec_float_value, compound_last_field as field};
use bbb_protocol::NbtSummaryValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RegistrySoundEvent {
    Reference {
        resource_id: String,
    },
    Direct {
        resource_id: String,
        fixed_range: Option<f32>,
    },
}

impl RegistrySoundEvent {
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

pub(crate) fn project_validated_sound_event(value: &NbtSummaryValue) -> RegistrySoundEvent {
    match value {
        NbtSummaryValue::String(raw) => RegistrySoundEvent::Reference {
            resource_id: raw.try_to_string().expect("canonical sound identifier"),
        },
        NbtSummaryValue::Compound(entries) => {
            let Some(NbtSummaryValue::String(sound_id)) = field(entries, "sound_id") else {
                unreachable!("canonical direct sound identifier");
            };
            RegistrySoundEvent::Direct {
                resource_id: sound_id
                    .try_to_string()
                    .expect("canonical sound identifier"),
                fixed_range: field(entries, "range")
                    .map(|value| codec_float_value(value).expect("canonical sound range")),
            }
        }
        _ => unreachable!("canonical sound holder"),
    }
}
