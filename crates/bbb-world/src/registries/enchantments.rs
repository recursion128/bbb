use bbb_protocol::{
    attribute_modifier_types::attribute_modifier_operation_id,
    equipment_slots::vanilla_equipment_slot_group_for_serialized_name,
    packets::ItemEnchantmentSummary, styled_component_runs_from_nbt_value,
    styled_runs_summary_text, NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};

use crate::{FrozenRegistryAccess, RegistryHolderSetIdentity, WorldStore};

use super::{decode_persistent_sound_event, normalize_codec_identifier};

mod generated_26_1;

pub use generated_26_1::{VanillaEnchantmentIdentity, VANILLA_ENCHANTMENT_TOOLTIP_ORDER};

pub(super) const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
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

/// Tooltip-relevant fields decoded from one synchronized enchantment element.
///
/// The holder-order resource identity remains in [`RegistrySet`]. This value
/// only carries the dynamic definition data that cannot be inferred from the
/// resource id, while a missing entry lets consumers use their vanilla fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantmentTooltipDefinition {
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    pub description_translation_key: Option<String>,
    pub description_translation_fallback: Option<String>,
    pub max_level: i32,
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

/// Deterministic variants of vanilla `EnchantmentValueEffect`.
///
/// `remove_binomial` consumes the holder entity's `RandomSource`; bbb does not
/// yet mirror that per-entity random stream, so that variant is deliberately
/// rejected instead of inventing a result.
#[derive(Debug, Clone, PartialEq)]
enum EnchantmentValueEffect {
    Add(LevelBasedValue),
    AllOf(Vec<Self>),
    Multiply(LevelBasedValue),
    Exponential {
        base: LevelBasedValue,
        exponent: LevelBasedValue,
    },
    Set(LevelBasedValue),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MaterializedEnchantmentDefinition {
    tooltip: Option<EnchantmentTooltipDefinition>,
    attributes: Option<EnchantmentDefinition>,
    crossbow_charge_time: Option<EnchantmentValueEffect>,
    prevents_armor_change: bool,
    holder_sets: EnchantmentHolderSetDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnchantmentHolderSetDefinition {
    pub supported_items: RegistryHolderSetIdentity,
    pub primary_items: Option<RegistryHolderSetIdentity>,
    pub exclusive_enchantments: RegistryHolderSetIdentity,
}

pub(super) fn materialize_enchantment_definition(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<MaterializedEnchantmentDefinition, String> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err("<root>: expected compound".to_string());
    };
    let supported_items = nbt_field(fields, "supported_items")
        .ok_or_else(|| "supported_items: missing required holder set".to_string())?;
    let supported_items = registry_access
        .persistent_holder_set_from_nbt("minecraft:item", supported_items)
        .map_err(|error| format!("supported_items: {error}"))?
        .identity();
    let primary_items = nbt_field(fields, "primary_items")
        .map(|value| {
            registry_access
                .persistent_holder_set_from_nbt("minecraft:item", value)
                .map(|holder_set| holder_set.identity())
                .map_err(|error| format!("primary_items: {error}"))
        })
        .transpose()?;
    let exclusive_enchantments = match nbt_field(fields, "exclusive_set") {
        Some(value) => registry_access
            .persistent_holder_set_from_nbt(ENCHANTMENT_REGISTRY, value)
            .map_err(|error| format!("exclusive_set: {error}"))?
            .identity(),
        None => RegistryHolderSetIdentity::Direct {
            resource_ids: Vec::new(),
        },
    };
    if let Some(effects) = nbt_field(fields, "effects") {
        validate_enchantment_effect_holder_sets(effects, registry_access, "effects")?;
    }

    Ok(MaterializedEnchantmentDefinition {
        tooltip: decode_enchantment_tooltip_definition(value),
        attributes: decode_enchantment_definition(value),
        crossbow_charge_time: decode_crossbow_charge_time_effect(value).flatten(),
        prevents_armor_change: nbt_field(fields, "effects")
            .and_then(|value| match value {
                NbtSummaryValue::Compound(effects) => Some(effects.as_slice()),
                _ => None,
            })
            .is_some_and(|effects| nbt_field(effects, "minecraft:prevent_armor_change").is_some()),
        holder_sets: EnchantmentHolderSetDefinition {
            supported_items,
            primary_items,
            exclusive_enchantments,
        },
    })
}

impl WorldStore {
    /// Resolves `EnchantmentEffectComponents.PREVENT_ARMOR_CHANGE` from the
    /// synchronized enchantment definition. Stubbed built-in entries use the
    /// generated vanilla identity fallback; server-provided custom definitions
    /// are read from their materialized `effects` compound.
    pub(crate) fn enchantment_prevents_armor_change(&self, holder_id: i32) -> Option<bool> {
        let entry = self.registry_entry_by_holder_id(ENCHANTMENT_REGISTRY, holder_id)?;
        if let Some(definition) = self.registries.typed.enchantments.get(&entry.id) {
            return Some(definition.prevents_armor_change);
        }
        if let Some(data) = entry.element_data() {
            let access = self.frozen_registry_access();
            return materialize_enchantment_definition(&data.value, access)
                .ok()
                .map(|definition| definition.prevents_armor_change);
        }
        (!entry.has_data).then(|| {
            VanillaEnchantmentIdentity::for_resource_id(&entry.id)
                == Some(VanillaEnchantmentIdentity::BindingCurse)
        })
    }

    pub fn synchronized_enchantment_holder_sets(
        &self,
        holder_id: i32,
    ) -> Option<EnchantmentHolderSetDefinition> {
        let entry = self.registry_entry_by_holder_id(ENCHANTMENT_REGISTRY, holder_id)?;
        let access = self.frozen_registry_access();
        let definition = self
            .registries
            .typed
            .enchantments
            .get(&entry.id)
            .cloned()
            .or_else(|| {
                entry
                    .element_data()
                    .and_then(|data| materialize_enchantment_definition(&data.value, access).ok())
            })?;
        access
            .holder_set_by_identity("minecraft:item", &definition.holder_sets.supported_items)
            .ok()?;
        if let Some(primary_items) = &definition.holder_sets.primary_items {
            access
                .holder_set_by_identity("minecraft:item", primary_items)
                .ok()?;
        }
        access
            .holder_set_by_identity(
                ENCHANTMENT_REGISTRY,
                &definition.holder_sets.exclusive_enchantments,
            )
            .ok()?;
        Some(definition.holder_sets)
    }

    /// Decodes synchronized enchantment tooltip definitions in network holder order.
    /// Stub and malformed elements stay as `None`; consumers may then apply a
    /// known-vanilla fallback without inventing definitions for custom keys.
    pub fn enchantment_tooltip_definitions_in_holder_order(
        &self,
    ) -> Option<Vec<Option<EnchantmentTooltipDefinition>>> {
        Some(
            self.registry_content(ENCHANTMENT_REGISTRY)?
                .entries
                .iter()
                .map(
                    |entry| match self.registries.typed.enchantments.get(&entry.id) {
                        Some(definition) => definition.tooltip.clone(),
                        None => entry
                            .element_data()
                            .and_then(|data| decode_enchantment_tooltip_definition(&data.value)),
                    },
                )
                .collect(),
        )
    }

    /// Resolves vanilla `EnchantmentHelper.forEachModifier` for the stack's
    /// synced enchantment holder ids and levels. Malformed or stub registry
    /// entries contribute no modifiers, matching the existing fail-closed
    /// registry projection policy.
    pub fn enchantment_attribute_modifiers(
        &self,
        enchantments: &[ItemEnchantmentSummary],
    ) -> Vec<EnchantmentAttributeModifier> {
        let mut modifiers = Vec::new();
        for enchantment in enchantments {
            let Some(entry) =
                self.registry_entry_by_holder_id(ENCHANTMENT_REGISTRY, enchantment.holder_id)
            else {
                continue;
            };
            let definition = match self.registries.typed.enchantments.get(&entry.id) {
                Some(definition) => definition.attributes.clone(),
                None => entry
                    .element_data()
                    .and_then(|data| decode_enchantment_definition(&data.value)),
            };
            let Some(definition) = definition else {
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

    /// Applies synchronized `minecraft:crossbow_charge_time` effects in stack
    /// enchantment order and returns vanilla's floored tick duration.
    ///
    /// A stubbed vanilla quick-charge definition uses the generated identity
    /// fallback. Server-provided data always wins, including custom
    /// enchantments and replacements of the vanilla resource id. Malformed or
    /// random-dependent effects fail closed and leave the duration unchanged.
    pub fn crossbow_charge_duration_ticks(&self, enchantments: &[ItemEnchantmentSummary]) -> i32 {
        let mut duration_seconds = 1.25_f32;
        for enchantment in enchantments {
            let Some(entry) =
                self.registry_entry_by_holder_id(ENCHANTMENT_REGISTRY, enchantment.holder_id)
            else {
                continue;
            };
            let effect = match self.registries.typed.enchantments.get(&entry.id) {
                Some(definition) => definition.crossbow_charge_time.clone(),
                None => match entry.element_data() {
                    Some(data) => decode_crossbow_charge_time_effect(&data.value).flatten(),
                    None if !entry.has_data => vanilla_crossbow_charge_time_effect(&entry.id),
                    None => None,
                },
            };
            if let Some(effect) = effect {
                duration_seconds = effect.process(enchantment.level, duration_seconds);
            }
        }
        (duration_seconds.max(0.0) * 20.0).floor() as i32
    }
}

fn validate_enchantment_effect_holder_sets(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<(), String> {
    match value {
        NbtSummaryValue::Compound(fields) => {
            let effect_type = nbt_string(nbt_field(fields, "type"));
            let holder_set = match effect_type {
                Some("minecraft:apply_mob_effect" | "apply_mob_effect") => {
                    Some(("to_apply", "minecraft:mob_effect", true))
                }
                Some("minecraft:explode" | "explode") => {
                    Some(("immune_blocks", "minecraft:block", false))
                }
                Some("minecraft:summon_entity" | "summon_entity") => {
                    Some(("entity", "minecraft:entity_type", true))
                }
                _ => None,
            };
            if let Some((field, registry, required)) = holder_set {
                match nbt_field(fields, field) {
                    Some(value) => {
                        registry_access
                            .persistent_holder_set_from_nbt(registry, value)
                            .map_err(|error| format!("{path}.{field}: {error}"))?;
                    }
                    None if required => return Err(format!("{path}.{field}: missing holder set")),
                    None => {}
                }
            }
            let holder_reference = match effect_type {
                Some("minecraft:damage_entity" | "damage_entity") => {
                    Some(("damage_type", "minecraft:damage_type", true))
                }
                Some("minecraft:explode" | "explode") => {
                    Some(("damage_type", "minecraft:damage_type", false))
                }
                Some(
                    "minecraft:replace_block"
                    | "replace_block"
                    | "minecraft:replace_disk"
                    | "replace_disk"
                    | "minecraft:set_block_properties"
                    | "set_block_properties",
                ) => Some(("trigger_game_event", "minecraft:game_event", false)),
                _ => None,
            };
            if let Some((field, registry, required)) = holder_reference {
                match nbt_field(fields, field) {
                    Some(value) => {
                        registry_access
                            .persistent_holder_from_nbt(registry, value)
                            .map_err(|error| format!("{path}.{field}: {error}"))?;
                    }
                    None if required => {
                        return Err(format!("{path}.{field}: missing holder reference"))
                    }
                    None => {}
                }
            }
            match effect_type {
                Some("minecraft:explode" | "explode") => {
                    validate_required_sound_event(fields, "sound", registry_access, path)?;
                }
                Some("minecraft:play_sound" | "play_sound") => {
                    let field = "sound";
                    let sound = nbt_field(fields, field)
                        .ok_or_else(|| format!("{path}.{field}: missing sound holder"))?;
                    validate_compact_sound_event_list(
                        sound,
                        registry_access,
                        &format!("{path}.{field}"),
                    )?;
                }
                _ => {}
            }
            for entry in fields {
                if normalize_codec_identifier(&entry.name).as_deref()
                    == Some("minecraft:trident_sound")
                {
                    validate_sound_event_list(
                        &entry.value,
                        registry_access,
                        &format!("{path}.{}", entry.name),
                    )?;
                }
            }
            for entry in fields {
                validate_enchantment_effect_holder_sets(
                    &entry.value,
                    registry_access,
                    &format!("{path}.{}", entry.name),
                )?;
            }
        }
        NbtSummaryValue::List(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_enchantment_effect_holder_sets(
                    value,
                    registry_access,
                    &format!("{path}[{index}]"),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_required_sound_event(
    fields: &[NbtSummaryEntry],
    field: &str,
    registry_access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<(), String> {
    let value =
        nbt_field(fields, field).ok_or_else(|| format!("{path}.{field}: missing sound holder"))?;
    validate_sound_event(value, registry_access, &format!("{path}.{field}"))
}

fn validate_compact_sound_event_list(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<(), String> {
    match value {
        NbtSummaryValue::List(values) => {
            if values.len() > 255 {
                return Err(format!("{path}: sound holder list exceeds 255 entries"));
            }
            for (index, value) in values.iter().enumerate() {
                validate_sound_event(value, registry_access, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        _ => validate_sound_event(value, registry_access, path),
    }
}

fn validate_sound_event_list(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<(), String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{path}: expected sound holder list"));
    };
    for (index, value) in values.iter().enumerate() {
        validate_sound_event(value, registry_access, &format!("{path}[{index}]"))?;
    }
    Ok(())
}

fn validate_sound_event(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
    path: &str,
) -> Result<(), String> {
    decode_persistent_sound_event(value, Some(registry_access))
        .map(|_| ())
        .map_err(|error| format!("{path}.{}: {}", error.field, error.reason))
}

fn decode_enchantment_tooltip_definition(
    value: &NbtSummaryValue,
) -> Option<EnchantmentTooltipDefinition> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let description = nbt_field(fields, "description")?;
    let description_fields = nbt_compound(Some(description));
    let description_translation_key = description_fields
        .and_then(|fields| nbt_string(nbt_field(fields, "translate")))
        .map(str::to_string);
    let description_translation_fallback = description_fields
        .and_then(|fields| nbt_string(nbt_field(fields, "fallback")))
        .map(str::to_string);
    let description_styled = styled_component_runs_from_nbt_value(description);
    let description = styled_runs_summary_text(&description_styled);
    let max_level = nbt_i32(nbt_field(fields, "max_level")?)?;
    if !(1..=255).contains(&max_level) {
        return None;
    }
    Some(EnchantmentTooltipDefinition {
        description,
        description_styled,
        description_translation_key,
        description_translation_fallback,
        max_level,
    })
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

impl EnchantmentValueEffect {
    fn process(&self, level: i32, input: f32) -> f32 {
        match self {
            Self::Add(value) => input + value.calculate(level),
            Self::AllOf(effects) => effects
                .iter()
                .fold(input, |value, effect| effect.process(level, value)),
            Self::Multiply(factor) => input * factor.calculate(level),
            Self::Exponential { base, exponent } => {
                input
                    * f64::from(base.calculate(level)).powf(f64::from(exponent.calculate(level)))
                        as f32
            }
            Self::Set(value) => value.calculate(level),
        }
    }
}

fn decode_crossbow_charge_time_effect(
    value: &NbtSummaryValue,
) -> Option<Option<EnchantmentValueEffect>> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let Some(effects) = nbt_field(fields, "effects") else {
        return Some(None);
    };
    let effects = nbt_compound(Some(effects))?;
    let Some(effect) = nbt_field(effects, "minecraft:crossbow_charge_time") else {
        return Some(None);
    };
    Some(Some(parse_enchantment_value_effect(effect)?))
}

fn vanilla_crossbow_charge_time_effect(resource_id: &str) -> Option<EnchantmentValueEffect> {
    (VanillaEnchantmentIdentity::for_resource_id(resource_id)
        == Some(VanillaEnchantmentIdentity::QuickCharge))
    .then_some(EnchantmentValueEffect::Add(LevelBasedValue::Linear {
        base: -0.25,
        per_level_above_first: -0.25,
    }))
}

fn parse_enchantment_value_effect(value: &NbtSummaryValue) -> Option<EnchantmentValueEffect> {
    let fields = nbt_compound(Some(value))?;
    let effect_type = nbt_string(nbt_field(fields, "type"))?;
    let effect_type = effect_type
        .strip_prefix("minecraft:")
        .unwrap_or(effect_type);
    match effect_type {
        "add" => Some(EnchantmentValueEffect::Add(parse_level_based_value(
            nbt_field(fields, "value")?,
        )?)),
        "all_of" => Some(EnchantmentValueEffect::AllOf(
            nbt_list(nbt_field(fields, "effects"))?
                .iter()
                .map(parse_enchantment_value_effect)
                .collect::<Option<Vec<_>>>()?,
        )),
        "multiply" => Some(EnchantmentValueEffect::Multiply(parse_level_based_value(
            nbt_field(fields, "factor")?,
        )?)),
        "exponential" => Some(EnchantmentValueEffect::Exponential {
            base: parse_level_based_value(nbt_field(fields, "base")?)?,
            exponent: parse_level_based_value(nbt_field(fields, "exponent")?)?,
        }),
        "set" => Some(EnchantmentValueEffect::Set(parse_level_based_value(
            nbt_field(fields, "value")?,
        )?)),
        // Vanilla `RemoveBinomial` needs the holder's mutable RandomSource.
        "remove_binomial" => None,
        _ => None,
    }
}

fn decode_enchantment_definition(value: &NbtSummaryValue) -> Option<EnchantmentDefinition> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let slots = nbt_list(nbt_field(fields, "slots"))?
        .iter()
        .map(|value| {
            let slot = nbt_string(Some(value))?;
            vanilla_equipment_slot_group_for_serialized_name(slot).map(|_| slot.to_string())
        })
        .collect::<Option<Vec<_>>>()?;
    let effects = nbt_compound(nbt_field(fields, "effects"))?;
    let attribute_effects = match nbt_field(effects, "minecraft:attributes") {
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

fn parse_attribute_effect(value: &NbtSummaryValue) -> Option<EnchantmentAttributeEffect> {
    let fields = nbt_compound(Some(value))?;
    Some(EnchantmentAttributeEffect {
        id: nbt_string(nbt_field(fields, "id"))?.to_string(),
        attribute: nbt_string(nbt_field(fields, "attribute"))?.to_string(),
        amount: parse_level_based_value(nbt_field(fields, "amount")?)?,
        operation_id: attribute_modifier_operation_id(nbt_string(nbt_field(fields, "operation"))?)?,
    })
}

fn parse_level_based_value(value: &NbtSummaryValue) -> Option<LevelBasedValue> {
    if let Some(value) = nbt_f32(value) {
        return Some(LevelBasedValue::Constant(value));
    }
    let fields = nbt_compound(Some(value))?;
    let value_type = nbt_string(nbt_field(fields, "type"))?;
    let value_type = value_type.strip_prefix("minecraft:").unwrap_or(value_type);
    match value_type {
        "constant" => Some(LevelBasedValue::Constant(nbt_f32(nbt_field(
            fields, "value",
        )?)?)),
        "clamped" => {
            let min = nbt_f32(nbt_field(fields, "min")?)?;
            let max = nbt_f32(nbt_field(fields, "max")?)?;
            let value = parse_level_based_value(nbt_field(fields, "value")?)?;
            (max > min).then_some(LevelBasedValue::Clamped {
                value: Box::new(value),
                min,
                max,
            })
        }
        "exponent" => Some(LevelBasedValue::Exponent {
            base: Box::new(parse_level_based_value(nbt_field(fields, "base")?)?),
            power: Box::new(parse_level_based_value(nbt_field(fields, "power")?)?),
        }),
        "fraction" => Some(LevelBasedValue::Fraction {
            numerator: Box::new(parse_level_based_value(nbt_field(fields, "numerator")?)?),
            denominator: Box::new(parse_level_based_value(nbt_field(fields, "denominator")?)?),
        }),
        "levels_squared" => Some(LevelBasedValue::LevelsSquared {
            added: nbt_f32(nbt_field(fields, "added")?)?,
        }),
        "linear" => Some(LevelBasedValue::Linear {
            base: nbt_f32(nbt_field(fields, "base")?)?,
            per_level_above_first: nbt_f32(nbt_field(fields, "per_level_above_first")?)?,
        }),
        "lookup" => Some(LevelBasedValue::Lookup {
            values: nbt_list(nbt_field(fields, "values"))?
                .iter()
                .map(nbt_f32)
                .collect::<Option<Vec<_>>>()?,
            fallback: Box::new(parse_level_based_value(nbt_field(fields, "fallback")?)?),
        }),
        _ => None,
    }
}

fn nbt_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn nbt_compound(value: Option<&NbtSummaryValue>) -> Option<&[NbtSummaryEntry]> {
    let NbtSummaryValue::Compound(entries) = value? else {
        return None;
    };
    Some(entries)
}

fn nbt_list(value: Option<&NbtSummaryValue>) -> Option<&[NbtSummaryValue]> {
    let NbtSummaryValue::List(values) = value? else {
        return None;
    };
    Some(values)
}

fn nbt_string(value: Option<&NbtSummaryValue>) -> Option<&str> {
    let NbtSummaryValue::String(value) = value? else {
        return None;
    };
    Some(value)
}

fn nbt_f32(value: &NbtSummaryValue) -> Option<f32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(f32::from(*value)),
        NbtSummaryValue::Short(value) => Some(f32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value as f32),
        NbtSummaryValue::Long(value) => Some(*value as f32),
        NbtSummaryValue::Float(bits) => Some(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => Some(f64::from_bits(*bits) as f32),
        _ => None,
    }
}

fn nbt_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{RegistryData, RegistryDataEntry},
        ComponentStyle,
    };

    use super::*;

    #[test]
    fn resolves_prevent_armor_change_for_vanilla_and_custom_definitions() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:binding_curse".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "minecraft:sharpness".to_string(),
                    raw_data: None,
                },
            ],
            raw_payload_len: 0,
        });
        let binding_curse = world
            .registry_holder_id_by_resource_id(ENCHANTMENT_REGISTRY, "minecraft:binding_curse")
            .unwrap();
        let sharpness = world
            .registry_holder_id_by_resource_id(ENCHANTMENT_REGISTRY, "minecraft:sharpness")
            .unwrap();
        assert_eq!(
            world.enchantment_prevents_armor_change(binding_curse),
            Some(true)
        );
        assert_eq!(
            world.enchantment_prevents_armor_change(sharpness),
            Some(false)
        );

        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "description":{"text":"Lock"},
                "supported_items":"minecraft:saddle",
                "max_level":1,
                "slots":["body"],
                "effects":{"minecraft:prevent_armor_change":{}}
            }"#,
        )
        .unwrap();
        let custom =
            materialize_enchantment_definition(&data.value, world.frozen_registry_access())
                .unwrap();
        assert!(custom.prevents_armor_change);
    }

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
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                    slot: "mainhand".to_string(),
                },
                EnchantmentAttributeModifier {
                    attribute: "minecraft:sweeping_damage_ratio".to_string(),
                    modifier_id: "example:fraction/mainhand".to_string(),
                    amount: 0.75,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                    slot: "mainhand".to_string(),
                },
            ]
        );
    }

    #[test]
    fn decodes_synced_enchantment_tooltip_definitions_in_holder_order() {
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

        let definitions = world
            .enchantment_tooltip_definitions_in_holder_order()
            .unwrap();
        assert_eq!(definitions.len(), 2);
        assert_eq!(definitions[0], None);
        assert_eq!(
            definitions[1],
            Some(EnchantmentTooltipDefinition {
                description: "enchantment.example.dual_effect".to_string(),
                description_styled: vec![StyledTextRun {
                    text: "enchantment.example.dual_effect".to_string(),
                    style: ComponentStyle {
                        color: Some(0x12_34_56),
                        ..ComponentStyle::default()
                    },
                }],
                description_translation_key: Some("enchantment.example.dual_effect".to_string()),
                description_translation_fallback: None,
                max_level: 7,
            })
        );
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_enchantment_projections() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "description": {"translate": "enchantment.example.known"},
                "max_level": 3,
                "slots": ["mainhand"],
                "effects": {
                    "minecraft:attributes": [{
                        "id": "example:known",
                        "attribute": "minecraft:mining_efficiency",
                        "amount": {
                            "type": "minecraft:linear",
                            "base": 1.0,
                            "per_level_above_first": 0.5
                        },
                        "operation": "add_value"
                    }]
                }
            }"#,
        )
        .unwrap();

        let tooltip = decode_enchantment_tooltip_definition(&data.value).unwrap();
        assert_eq!(
            tooltip.description_translation_key.as_deref(),
            Some("enchantment.example.known")
        );
        assert_eq!(tooltip.max_level, 3);
        let definition = decode_enchantment_definition(&data.value).unwrap();
        assert_eq!(definition.slots, ["mainhand"]);
        assert_eq!(definition.attribute_effects.len(), 1);
        assert_eq!(definition.attribute_effects[0].amount.calculate(3), 2.0);
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

    #[test]
    fn crossbow_charge_time_applies_stub_fallback_and_custom_effects_in_stack_order() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:quick_charge".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "example:patient_loading".to_string(),
                    raw_data: Some(crossbow_effect_enchantment_nbt(compound_payload(vec![
                        string_tag("type", "minecraft:all_of"),
                        compound_list_tag(
                            "effects",
                            vec![
                                compound_payload(vec![
                                    string_tag("type", "minecraft:multiply"),
                                    float_tag("factor", 2.0),
                                ]),
                                compound_payload(vec![
                                    string_tag("type", "minecraft:add"),
                                    compound_tag(
                                        "value",
                                        vec![
                                            string_tag("type", "minecraft:linear"),
                                            float_tag("base", 0.25),
                                            float_tag("per_level_above_first", 0.25),
                                        ],
                                    ),
                                ]),
                            ],
                        ),
                    ]))),
                },
            ],
            raw_payload_len: 0,
        });
        let quick_charge = ItemEnchantmentSummary {
            holder_id: 0,
            level: 2,
        };
        let custom = ItemEnchantmentSummary {
            holder_id: 1,
            level: 2,
        };

        assert_eq!(
            world.crossbow_charge_duration_ticks(&[quick_charge.clone(), custom.clone()]),
            40
        );
        assert_eq!(
            world.crossbow_charge_duration_ticks(&[custom, quick_charge]),
            50
        );
    }

    #[test]
    fn crossbow_charge_time_server_definition_overrides_vanilla_resource_fallback() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:quick_charge".to_string(),
                raw_data: Some(compound_payload(vec![compound_tag("effects", Vec::new())])),
            }],
            raw_payload_len: 0,
        });

        assert_eq!(
            world.crossbow_charge_duration_ticks(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            }]),
            25
        );
    }

    #[test]
    fn deterministic_value_effects_cover_remaining_vanilla_variants() {
        let effect = EnchantmentValueEffect::AllOf(vec![
            EnchantmentValueEffect::Set(LevelBasedValue::Constant(0.5)),
            EnchantmentValueEffect::Exponential {
                base: LevelBasedValue::Constant(2.0),
                exponent: LevelBasedValue::Constant(2.0),
            },
            EnchantmentValueEffect::Multiply(LevelBasedValue::Constant(0.5)),
            EnchantmentValueEffect::Add(LevelBasedValue::Linear {
                base: 0.25,
                per_level_above_first: 0.25,
            }),
        ]);

        assert_eq!(effect.process(2, 99.0), 1.5);
        assert!(
            parse_enchantment_value_effect(&NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:remove_binomial".to_string()),
                },
                NbtSummaryEntry {
                    name: "chance".to_string(),
                    value: NbtSummaryValue::Double(0.5_f64.to_bits()),
                },
            ]))
            .is_none()
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
            compound_tag(
                "description",
                vec![
                    string_tag("translate", "enchantment.example.dual_effect"),
                    string_tag("color", "#123456"),
                ],
            ),
            int_tag("max_level", 7),
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

    fn crossbow_effect_enchantment_nbt(effect_payload: Vec<u8>) -> Vec<u8> {
        compound_payload(vec![compound_tag(
            "effects",
            vec![named_compound_payload(
                "minecraft:crossbow_charge_time",
                effect_payload,
            )],
        )])
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

    fn int_tag(name: &str, value: i32) -> Vec<u8> {
        let mut out = vec![3];
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
