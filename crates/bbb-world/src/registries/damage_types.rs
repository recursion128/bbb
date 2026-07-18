use bbb_protocol::{
    damage_types::vanilla_damage_type_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

pub const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageTypeScaling {
    Never,
    WhenCausedByLivingNonPlayer,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageTypeEffects {
    Hurt,
    Thorns,
    Drowning,
    Burning,
    Poking,
    Freezing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageTypeDeathMessageType {
    Default,
    FallVariants,
    IntentionalGameDesign,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageTypePresentation {
    pub message_id: String,
    pub scaling: DamageTypeScaling,
    pub exhaustion: f32,
    pub effects: DamageTypeEffects,
    pub death_message_type: DamageTypeDeathMessageType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageTypeProjectionError {
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for DamageTypeProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl WorldStore {
    pub fn synchronized_damage_type_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<DamageTypePresentation>, DamageTypeProjectionError> {
        let Some(data) = self
            .registry_entry_by_holder_id(DAMAGE_TYPE_REGISTRY, holder_id)
            .and_then(|entry| entry.element_data())
        else {
            return Ok(None);
        };
        decode_damage_type_value(&data.value).map(Some)
    }

    pub fn damage_type_effects_by_holder_id(
        &self,
        holder_id: i32,
    ) -> Result<Option<DamageTypeEffects>, DamageTypeProjectionError> {
        if let Some(presentation) = self.synchronized_damage_type_presentation(holder_id)? {
            return Ok(Some(presentation.effects));
        }
        let Some(resource_id) = self
            .resolved_registry_entry_by_holder_id(DAMAGE_TYPE_REGISTRY, holder_id)
            .map(|entry| entry.resource_id)
        else {
            return Ok(None);
        };
        Ok(vanilla_damage_type_for_resource_id(resource_id)
            .and_then(|entry| DamageTypeEffects::from_codec_name(entry.effect)))
    }
}

fn decode_damage_type_value(
    value: &NbtSummaryValue,
) -> Result<DamageTypePresentation, DamageTypeProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error("<root>", "expected compound"));
    };
    let message_id = required_string(fields, "message_id")?.to_string();
    let scaling = match required_string(fields, "scaling")? {
        "never" => DamageTypeScaling::Never,
        "when_caused_by_living_non_player" => DamageTypeScaling::WhenCausedByLivingNonPlayer,
        "always" => DamageTypeScaling::Always,
        _ => return Err(error("scaling", "unknown enum value")),
    };
    let exhaustion = match field_value(fields, "exhaustion") {
        Some(value) => numeric_f32(value)
            .filter(|value| value.is_finite())
            .ok_or_else(|| error("exhaustion", "expected finite number"))?,
        None => return Err(error("exhaustion", "missing required field")),
    };
    let effects = match optional_string(fields, "effects")? {
        None => DamageTypeEffects::Hurt,
        Some(value) => DamageTypeEffects::from_codec_name(value)
            .ok_or_else(|| error("effects", "unknown enum value"))?,
    };
    let death_message_type = match optional_string(fields, "death_message_type")? {
        None | Some("default") => DamageTypeDeathMessageType::Default,
        Some("fall_variants") => DamageTypeDeathMessageType::FallVariants,
        Some("intentional_game_design") => DamageTypeDeathMessageType::IntentionalGameDesign,
        Some(_) => return Err(error("death_message_type", "unknown enum value")),
    };
    Ok(DamageTypePresentation {
        message_id,
        scaling,
        exhaustion,
        effects,
        death_message_type,
    })
}

impl DamageTypeEffects {
    fn from_codec_name(value: &str) -> Option<Self> {
        match value {
            "hurt" => Some(Self::Hurt),
            "thorns" => Some(Self::Thorns),
            "drowning" => Some(Self::Drowning),
            "burning" => Some(Self::Burning),
            "poking" => Some(Self::Poking),
            "freezing" => Some(Self::Freezing),
            _ => None,
        }
    }
}

fn required_string<'a>(
    fields: &'a [NbtSummaryEntry],
    field: &str,
) -> Result<&'a str, DamageTypeProjectionError> {
    optional_string(fields, field)?.ok_or_else(|| error(field, "missing required field"))
}

fn optional_string<'a>(
    fields: &'a [NbtSummaryEntry],
    field: &str,
) -> Result<Option<&'a str>, DamageTypeProjectionError> {
    match field_value(fields, field) {
        Some(NbtSummaryValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(field, "expected string")),
        None => Ok(None),
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

fn field_value<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn error(field: impl Into<String>, reason: &'static str) -> DamageTypeProjectionError {
    DamageTypeProjectionError {
        field: field.into(),
        reason,
    }
}

#[cfg(test)]
fn decode_damage_type(
    raw_data: &[u8],
) -> Result<DamageTypePresentation, DamageTypeProjectionError> {
    let data = super::element_value::decode_network_element(raw_data).unwrap();
    decode_damage_type_value(&data.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_direct_codec_fields_and_defaults() {
        assert_eq!(
            decode_damage_type(&damage_type_nbt(
                "custom.fire",
                "always",
                0.25,
                Some("burning"),
                Some("intentional_game_design"),
            ))
            .unwrap(),
            DamageTypePresentation {
                message_id: "custom.fire".to_string(),
                scaling: DamageTypeScaling::Always,
                exhaustion: 0.25,
                effects: DamageTypeEffects::Burning,
                death_message_type: DamageTypeDeathMessageType::IntentionalGameDesign,
            }
        );
        let defaults =
            decode_damage_type(&damage_type_nbt("custom.generic", "never", 0.0, None, None))
                .unwrap();
        assert_eq!(defaults.effects, DamageTypeEffects::Hurt);
        assert_eq!(
            defaults.death_message_type,
            DamageTypeDeathMessageType::Default
        );
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_damage_projection() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "message_id": "custom.freeze",
                "scaling": "always",
                "exhaustion": 0.125,
                "effects": "freezing"
            }"#,
        )
        .unwrap();

        assert_eq!(
            decode_damage_type_value(&data.value).unwrap(),
            DamageTypePresentation {
                message_id: "custom.freeze".to_string(),
                scaling: DamageTypeScaling::Always,
                exhaustion: 0.125,
                effects: DamageTypeEffects::Freezing,
                death_message_type: DamageTypeDeathMessageType::Default,
            }
        );
    }

    #[test]
    fn rejects_invalid_required_and_enum_fields() {
        let mut missing_message = vec![10];
        write_string(&mut missing_message, "scaling", "never");
        write_float(&mut missing_message, "exhaustion", 0.0);
        missing_message.push(0);
        assert_eq!(
            decode_damage_type(&missing_message).unwrap_err(),
            error("message_id", "missing required field")
        );
        assert_eq!(
            decode_damage_type(&damage_type_nbt("custom.bad", "sometimes", 0.0, None, None,))
                .unwrap_err(),
            error("scaling", "unknown enum value")
        );
        assert_eq!(
            decode_damage_type(&damage_type_nbt(
                "custom.bad",
                "never",
                0.0,
                Some("sparkling"),
                None,
            ))
            .unwrap_err(),
            error("effects", "unknown enum value")
        );
    }

    #[test]
    fn world_query_obeys_server_holder_order_and_catalog_stubs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DAMAGE_TYPE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:freeze",
                    damage_type_nbt("custom.freeze", "always", 0.0, Some("freezing"), None),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:fire",
                    damage_type_nbt("custom.fire", "never", 0.1, Some("burning"), None),
                ),
                RegistryPacketEntry::stub("minecraft:drown"),
            ],
        );

        assert_eq!(
            store.damage_type_effects_by_holder_id(0).unwrap(),
            Some(DamageTypeEffects::Freezing)
        );
        assert_eq!(
            store.damage_type_effects_by_holder_id(1).unwrap(),
            Some(DamageTypeEffects::Burning)
        );
        assert_eq!(
            store.damage_type_effects_by_holder_id(2).unwrap(),
            Some(DamageTypeEffects::Drowning)
        );
        assert_eq!(store.damage_type_effects_by_holder_id(3).unwrap(), None);

        let vanilla = WorldStore::new();
        assert_eq!(
            vanilla.damage_type_effects_by_holder_id(8).unwrap(),
            Some(DamageTypeEffects::Drowning)
        );
    }

    fn damage_type_nbt(
        message_id: &str,
        scaling: &str,
        exhaustion: f32,
        effects: Option<&str>,
        death_message_type: Option<&str>,
    ) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "message_id", message_id);
        write_string(&mut out, "scaling", scaling);
        write_float(&mut out, "exhaustion", exhaustion);
        if let Some(effects) = effects {
            write_string(&mut out, "effects", effects);
        }
        if let Some(death_message_type) = death_message_type {
            write_string(&mut out, "death_message_type", death_message_type);
        }
        out.push(0);
        out
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_text(out, name);
        write_text(out, value);
    }

    fn write_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_text(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_text(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
