use crate::{DamageEffects, WorldStore};
use bbb_protocol::damage_types::vanilla_damage_type_for_resource_id;

pub const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";

impl WorldStore {
    pub fn damage_type_effects_by_holder_id(&self, holder_id: i32) -> Option<DamageEffects> {
        let resource_id =
            self.registry_resource_id_by_holder_id(DAMAGE_TYPE_REGISTRY, holder_id)?;
        self.registries
            .typed
            .damage_types
            .get(resource_id)
            .copied()
            .or_else(|| {
                vanilla_damage_type_for_resource_id(resource_id)
                    .and_then(|entry| DamageEffects::from_codec_name(entry.effect))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DamageScaling, DeathMessageType, RegistryFieldError, RegistryPacketEntry};
    use bbb_registry_codec::{
        registry_codec::{
            direct_leaf::{
                decode_direct_leaf_registry_element, DamageEffects, DamageTypeElement,
                DirectLeafRegistryElement,
            },
            RegistryCodecError,
        },
        RegistryElementData,
    };

    fn decode_damage_type_data(
        data: &RegistryElementData,
    ) -> Result<DamageTypeElement, RegistryCodecError> {
        let DirectLeafRegistryElement::DamageType(element) =
            decode_direct_leaf_registry_element(DAMAGE_TYPE_REGISTRY, data)?
        else {
            unreachable!("damage-type registry route returned another leaf element")
        };
        Ok(element)
    }

    fn decode_damage_type(raw_data: &[u8]) -> Result<DamageTypeElement, RegistryFieldError> {
        let data = super::super::element_value::decode_network_element(raw_data).unwrap();
        decode_damage_type_data(&data).map_err(RegistryFieldError::from)
    }

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
            DamageTypeElement {
                message_id: "custom.fire".into(),
                scaling: DamageScaling::Always,
                exhaustion: 0.25,
                effects: DamageEffects::Burning,
                death_message_type: DeathMessageType::IntentionalGameDesign,
            }
        );
        let defaults =
            decode_damage_type(&damage_type_nbt("custom.generic", "never", 0.0, None, None))
                .unwrap();
        assert_eq!(defaults.effects, DamageEffects::Hurt);
        assert_eq!(defaults.death_message_type, DeathMessageType::Default);
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
            decode_damage_type_data(&data).unwrap(),
            DamageTypeElement {
                message_id: "custom.freeze".into(),
                scaling: DamageScaling::Always,
                exhaustion: 0.125,
                effects: DamageEffects::Freezing,
                death_message_type: DeathMessageType::Default,
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
            RegistryFieldError {
                field: "message_id".to_string(),
                reason: "required field is missing".to_string(),
            }
        );
        assert_eq!(
            decode_damage_type(&damage_type_nbt("custom.bad", "sometimes", 0.0, None, None,))
                .unwrap_err(),
            RegistryFieldError {
                field: "scaling".to_string(),
                reason: "expected one of never, when_caused_by_living_non_player, always"
                    .to_string(),
            }
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
            RegistryFieldError {
                field: "effects".to_string(),
                reason: "expected one of hurt, thorns, drowning, burning, poking, freezing"
                    .to_string(),
            }
        );
    }

    #[test]
    fn source_aware_damage_projection_preserves_float_rounding_and_null_defaults() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "message_id":"custom.rounding",
                "scaling":"never",
                "exhaustion":1.0000000596046447753906250000000000001,
                "effects":null,
                "death_message_type":null,
                "unknown":{"nested":null}
            }"#,
        )
        .unwrap();

        let presentation = decode_damage_type_data(&data).unwrap();
        assert_eq!(presentation.exhaustion.to_bits(), 0x3f80_0001);
        assert_eq!(presentation.effects, DamageEffects::Hurt);
        assert_eq!(presentation.death_message_type, DeathMessageType::Default);
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
        store.begin_registry_configuration();
        store.finish_registry_configuration().unwrap();

        assert_eq!(
            store.damage_type_effects_by_holder_id(0),
            Some(DamageEffects::Freezing)
        );
        assert_eq!(
            store.damage_type_effects_by_holder_id(1),
            Some(DamageEffects::Burning)
        );
        assert_eq!(
            store.damage_type_effects_by_holder_id(2),
            Some(DamageEffects::Drowning)
        );
        assert_eq!(store.damage_type_effects_by_holder_id(3), None);

        // With no server registry the holder id is the loader-order index, so
        // `minecraft:drown` sits at 6 (its bootstrap index is 8).
        let vanilla = WorldStore::new();
        assert_eq!(
            vanilla.damage_type_effects_by_holder_id(6),
            Some(DamageEffects::Drowning)
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
