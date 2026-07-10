use bbb_protocol::packets::ItemEnchantmentSummary;

use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    WorldStore,
};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const EQUIPMENT_SLOT_GROUPS: &[&str] = &[
    "any", "mainhand", "offhand", "hand", "feet", "legs", "chest", "head", "armor", "body",
    "saddle",
];

/// A concrete attribute modifier contributed by one enchanted stack entry.
///
/// Slot and attribute names retain the vanilla serialized identifiers so the
/// item presentation layer can apply the synced registry definition without
/// duplicating dynamic-registry ownership.
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentAttributeModifier {
    pub attribute: String,
    pub modifier_id: String,
    pub amount: f64,
    pub operation_id: i32,
    pub slot: String,
}

#[derive(Debug, Clone, PartialEq)]
struct EnchantmentDefinition {
    slots: Vec<String>,
    attribute_effects: Vec<EnchantmentAttributeEffect>,
}

#[derive(Debug, Clone, PartialEq)]
struct EnchantmentAttributeEffect {
    id: String,
    attribute: String,
    amount: LevelBasedValue,
    operation_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
enum LevelBasedValue {
    Constant(f32),
    Clamped {
        value: Box<Self>,
        min: f32,
        max: f32,
    },
    Exponent {
        base: Box<Self>,
        power: Box<Self>,
    },
    Fraction {
        numerator: Box<Self>,
        denominator: Box<Self>,
    },
    LevelsSquared {
        added: f32,
    },
    Linear {
        base: f32,
        per_level_above_first: f32,
    },
    Lookup {
        values: Vec<f32>,
        fallback: Box<Self>,
    },
}

impl WorldStore {
    /// Resolves vanilla `EnchantmentHelper.forEachModifier` for the stack's
    /// synced enchantment holder ids and levels. Malformed or stub registry
    /// entries contribute no modifiers, matching the existing fail-closed
    /// registry projection policy.
    pub fn enchantment_attribute_modifiers(
        &self,
        enchantments: &[ItemEnchantmentSummary],
    ) -> Vec<EnchantmentAttributeModifier> {
        let Some(registry) = self.registry_content(ENCHANTMENT_REGISTRY) else {
            return Vec::new();
        };
        let mut modifiers = Vec::new();
        for enchantment in enchantments {
            let Some(entry) = usize::try_from(enchantment.holder_id)
                .ok()
                .and_then(|index| registry.entries.get(index))
            else {
                continue;
            };
            let Some(definition) = entry.raw_data().and_then(decode_enchantment_definition) else {
                continue;
            };
            for effect in definition.attribute_effects {
                let amount = f64::from(effect.amount.calculate(enchantment.level));
                for (index, slot) in definition.slots.iter().enumerate() {
                    if definition.slots[..index].contains(slot) {
                        continue;
                    }
                    modifiers.push(EnchantmentAttributeModifier {
                        attribute: effect.attribute.clone(),
                        modifier_id: format!("{}/{}", effect.id, slot),
                        amount,
                        operation_id: effect.operation_id,
                        slot: slot.clone(),
                    });
                }
            }
        }
        modifiers
    }
}

impl LevelBasedValue {
    fn calculate(&self, level: i32) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Clamped { value, min, max } => {
                let value = value.calculate(level);
                if value < *min {
                    *min
                } else if value > *max {
                    *max
                } else {
                    value
                }
            }
            Self::Exponent { base, power } => {
                f64::from(base.calculate(level)).powf(f64::from(power.calculate(level))) as f32
            }
            Self::Fraction {
                numerator,
                denominator,
            } => {
                let denominator = denominator.calculate(level);
                if denominator == 0.0 {
                    0.0
                } else {
                    numerator.calculate(level) / denominator
                }
            }
            Self::LevelsSquared { added } => {
                let level = level as f32;
                level * level + added
            }
            Self::Linear {
                base,
                per_level_above_first,
            } => base + per_level_above_first * (level - 1) as f32,
            Self::Lookup { values, fallback } => usize::try_from(level - 1)
                .ok()
                .and_then(|index| values.get(index))
                .copied()
                .unwrap_or_else(|| fallback.calculate(level)),
        }
    }
}

fn decode_enchantment_definition(raw_data: &[u8]) -> Option<EnchantmentDefinition> {
    let root = decode_nbt_root(raw_data).ok()??;
    let NbtValue::Compound(fields) = root else {
        return None;
    };
    let slots = nbt_list(find_nbt_entry(&fields, "slots"))?
        .iter()
        .map(|value| {
            let slot = nbt_string(Some(value))?;
            EQUIPMENT_SLOT_GROUPS
                .contains(&slot)
                .then(|| slot.to_string())
        })
        .collect::<Option<Vec<_>>>()?;
    let effects = nbt_compound(find_nbt_entry(&fields, "effects"))?;
    let attribute_effects = match find_nbt_entry(effects, "minecraft:attributes") {
        Some(value) => nbt_list(Some(value))?
            .iter()
            .map(parse_attribute_effect)
            .collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };
    Some(EnchantmentDefinition {
        slots,
        attribute_effects,
    })
}

fn parse_attribute_effect(value: &NbtValue) -> Option<EnchantmentAttributeEffect> {
    let fields = nbt_compound(Some(value))?;
    Some(EnchantmentAttributeEffect {
        id: nbt_string(find_nbt_entry(fields, "id"))?.to_string(),
        attribute: nbt_string(find_nbt_entry(fields, "attribute"))?.to_string(),
        amount: parse_level_based_value(find_nbt_entry(fields, "amount")?)?,
        operation_id: match nbt_string(find_nbt_entry(fields, "operation"))? {
            "add_value" => 0,
            "add_multiplied_base" => 1,
            "add_multiplied_total" => 2,
            _ => return None,
        },
    })
}

fn parse_level_based_value(value: &NbtValue) -> Option<LevelBasedValue> {
    if let Some(value) = nbt_f32(value) {
        return Some(LevelBasedValue::Constant(value));
    }
    let fields = nbt_compound(Some(value))?;
    let value_type = nbt_string(find_nbt_entry(fields, "type"))?;
    let value_type = value_type.strip_prefix("minecraft:").unwrap_or(value_type);
    match value_type {
        "constant" => Some(LevelBasedValue::Constant(nbt_f32(find_nbt_entry(
            fields, "value",
        )?)?)),
        "clamped" => {
            let min = nbt_f32(find_nbt_entry(fields, "min")?)?;
            let max = nbt_f32(find_nbt_entry(fields, "max")?)?;
            let value = parse_level_based_value(find_nbt_entry(fields, "value")?)?;
            (max > min).then_some(LevelBasedValue::Clamped {
                value: Box::new(value),
                min,
                max,
            })
        }
        "exponent" => Some(LevelBasedValue::Exponent {
            base: Box::new(parse_level_based_value(find_nbt_entry(fields, "base")?)?),
            power: Box::new(parse_level_based_value(find_nbt_entry(fields, "power")?)?),
        }),
        "fraction" => Some(LevelBasedValue::Fraction {
            numerator: Box::new(parse_level_based_value(find_nbt_entry(
                fields,
                "numerator",
            )?)?),
            denominator: Box::new(parse_level_based_value(find_nbt_entry(
                fields,
                "denominator",
            )?)?),
        }),
        "levels_squared" => Some(LevelBasedValue::LevelsSquared {
            added: nbt_f32(find_nbt_entry(fields, "added")?)?,
        }),
        "linear" => Some(LevelBasedValue::Linear {
            base: nbt_f32(find_nbt_entry(fields, "base")?)?,
            per_level_above_first: nbt_f32(find_nbt_entry(fields, "per_level_above_first")?)?,
        }),
        "lookup" => Some(LevelBasedValue::Lookup {
            values: nbt_list(find_nbt_entry(fields, "values"))?
                .iter()
                .map(nbt_f32)
                .collect::<Option<Vec<_>>>()?,
            fallback: Box::new(parse_level_based_value(find_nbt_entry(
                fields, "fallback",
            )?)?),
        }),
        _ => None,
    }
}

fn nbt_compound(value: Option<&NbtValue>) -> Option<&[(String, NbtValue)]> {
    let NbtValue::Compound(entries) = value? else {
        return None;
    };
    Some(entries)
}

fn nbt_list(value: Option<&NbtValue>) -> Option<&[NbtValue]> {
    let NbtValue::List(values) = value? else {
        return None;
    };
    Some(values)
}

fn nbt_string(value: Option<&NbtValue>) -> Option<&str> {
    let NbtValue::String(value) = value? else {
        return None;
    };
    Some(value)
}

fn nbt_f32(value: &NbtValue) -> Option<f32> {
    match value {
        NbtValue::Byte(value) => Some(f32::from(*value)),
        NbtValue::Short(value) => Some(f32::from(*value)),
        NbtValue::Int(value) => Some(*value as f32),
        NbtValue::Long(value) => Some(*value as f32),
        NbtValue::Float(value) => Some(*value),
        NbtValue::Double(value) => Some(*value as f32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{RegistryData, RegistryDataEntry};

    use super::*;

    #[test]
    fn resolves_synced_enchantment_attribute_effects_in_stack_and_effect_order() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:stub".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "example:dual_effect".to_string(),
                    raw_data: Some(test_enchantment_nbt()),
                },
            ],
            raw_payload_len: 0,
        });

        assert_eq!(
            world.enchantment_attribute_modifiers(&[
                ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 5,
                },
                ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 3,
                },
            ]),
            vec![
                EnchantmentAttributeModifier {
                    attribute: "minecraft:mining_efficiency".to_string(),
                    modifier_id: "example:linear/mainhand".to_string(),
                    amount: 2.0,
                    operation_id: 0,
                    slot: "mainhand".to_string(),
                },
                EnchantmentAttributeModifier {
                    attribute: "minecraft:sweeping_damage_ratio".to_string(),
                    modifier_id: "example:fraction/mainhand".to_string(),
                    amount: 0.75,
                    operation_id: 2,
                    slot: "mainhand".to_string(),
                },
            ]
        );
    }

    #[test]
    fn level_based_values_cover_every_vanilla_26_1_variant() {
        let linear = LevelBasedValue::Linear {
            base: 2.0,
            per_level_above_first: 0.5,
        };
        assert_eq!(linear.calculate(3), 3.0);
        assert_eq!(LevelBasedValue::Constant(4.0).calculate(7), 4.0);
        assert_eq!(
            LevelBasedValue::Clamped {
                value: Box::new(linear.clone()),
                min: 0.0,
                max: 2.5,
            }
            .calculate(3),
            2.5
        );
        assert_eq!(
            LevelBasedValue::Exponent {
                base: Box::new(LevelBasedValue::Constant(2.0)),
                power: Box::new(LevelBasedValue::Constant(3.0)),
            }
            .calculate(1),
            8.0
        );
        assert_eq!(
            LevelBasedValue::Fraction {
                numerator: Box::new(LevelBasedValue::Constant(2.0)),
                denominator: Box::new(LevelBasedValue::Constant(0.0)),
            }
            .calculate(1),
            0.0
        );
        assert_eq!(
            LevelBasedValue::LevelsSquared { added: 1.0 }.calculate(3),
            10.0
        );
        assert_eq!(
            LevelBasedValue::Lookup {
                values: vec![1.0, 4.0],
                fallback: Box::new(linear),
            }
            .calculate(2),
            4.0
        );
        assert_eq!(
            LevelBasedValue::Lookup {
                values: vec![1.0, 4.0],
                fallback: Box::new(LevelBasedValue::Constant(9.0)),
            }
            .calculate(3),
            9.0
        );
    }

    fn test_enchantment_nbt() -> Vec<u8> {
        let linear = compound_payload(vec![
            string_tag("type", "minecraft:linear"),
            float_tag("base", 1.0),
            float_tag("per_level_above_first", 0.5),
        ]);
        let numerator = compound_tag(
            "numerator",
            vec![
                string_tag("type", "minecraft:linear"),
                float_tag("base", 1.0),
                float_tag("per_level_above_first", 1.0),
            ],
        );
        let denominator = compound_tag(
            "denominator",
            vec![
                string_tag("type", "minecraft:linear"),
                float_tag("base", 2.0),
                float_tag("per_level_above_first", 1.0),
            ],
        );
        let fraction = compound_payload(vec![
            string_tag("type", "minecraft:fraction"),
            numerator,
            denominator,
        ]);
        let effects = vec![
            attribute_effect_payload(
                "example:linear",
                "minecraft:mining_efficiency",
                linear,
                "add_value",
            ),
            attribute_effect_payload(
                "example:fraction",
                "minecraft:sweeping_damage_ratio",
                fraction,
                "add_multiplied_total",
            ),
        ];
        compound_payload(vec![
            string_list_tag("slots", &["mainhand", "mainhand"]),
            compound_tag(
                "effects",
                vec![compound_list_tag("minecraft:attributes", effects)],
            ),
        ])
    }

    fn attribute_effect_payload(
        id: &str,
        attribute: &str,
        amount_payload: Vec<u8>,
        operation: &str,
    ) -> Vec<u8> {
        compound_payload(vec![
            named_compound_payload("amount", amount_payload),
            string_tag("attribute", attribute),
            string_tag("id", id),
            string_tag("operation", operation),
        ])
    }

    fn compound_payload(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![10];
        for entry in entries {
            out.extend(entry);
        }
        out.push(0);
        out
    }

    fn compound_tag(name: &str, entries: Vec<Vec<u8>>) -> Vec<u8> {
        named_compound_payload(name, compound_payload(entries))
    }

    fn named_compound_payload(name: &str, payload: Vec<u8>) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_string(&mut out, name);
        out.extend(payload.into_iter().skip(1));
        out
    }

    fn compound_list_tag(name: &str, payloads: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![9];
        write_nbt_string(&mut out, name);
        out.push(10);
        out.extend((payloads.len() as i32).to_be_bytes());
        for payload in payloads {
            out.extend(payload.into_iter().skip(1));
        }
        out
    }

    fn string_list_tag(name: &str, values: &[&str]) -> Vec<u8> {
        let mut out = vec![9];
        write_nbt_string(&mut out, name);
        out.push(8);
        out.extend((values.len() as i32).to_be_bytes());
        for value in values {
            write_nbt_string(&mut out, value);
        }
        out
    }

    fn string_tag(name: &str, value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_nbt_string(&mut out, name);
        write_nbt_string(&mut out, value);
        out
    }

    fn float_tag(name: &str, value: f32) -> Vec<u8> {
        let mut out = vec![5];
        write_nbt_string(&mut out, name);
        out.extend(value.to_be_bytes());
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend((bytes.len() as u16).to_be_bytes());
        out.extend(bytes);
    }
}
