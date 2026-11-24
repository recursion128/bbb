use std::borrow::Cow;

use bbb_protocol::nbt_summary::compound_field as nbt_field;
use bbb_protocol::{
    attribute_modifier_types::attribute_modifier_operation_id, packets::ItemEnchantmentSummary,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};

use bbb_registry_codec::{
    enchantment_value_effect::EnchantmentValueEffect, level_based_value::LevelBasedValue,
};

use crate::{
    FrozenRegistryAccess, FrozenRegistryMembership, LevelEventSoundRandomState, WorldStore,
};

use super::canonical_description;

mod generated_26_1;

pub use generated_26_1::{VanillaEnchantmentIdentity, VANILLA_ENCHANTMENT_TOOLTIP_ORDER};

pub const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

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
    pub description_nbt: Option<NbtSummaryValue>,
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
pub(super) struct MaterializedEnchantmentDefinition {
    tooltip: EnchantmentTooltipDefinition,
    attributes: Option<EnchantmentDefinition>,
    crossbow_charge_time: Option<EnchantmentValueEffect>,
    trident_spin_attack_strength: Option<EnchantmentValueEffect>,
    prevents_armor_change: bool,
}

pub(super) fn materialize_enchantment_definition(
    value: NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<MaterializedEnchantmentDefinition, String> {
    let NbtSummaryValue::Compound(fields) = &value else {
        unreachable!("canonical enchantment compound");
    };
    for (name, registry) in [
        ("supported_items", "minecraft:item"),
        ("primary_items", "minecraft:item"),
        ("exclusive_set", ENCHANTMENT_REGISTRY),
    ] {
        let Some(NbtSummaryValue::String(holder)) = nbt_field(fields, name) else {
            continue;
        };
        let missing_tag = holder
            .with_utf8(|holder| {
                holder.strip_prefix('#').is_some_and(|tag| {
                    registry_access.tag_membership(registry, tag)
                        != FrozenRegistryMembership::Present
                })
            })
            .expect("canonical enchantment holder identifier");
        if missing_tag {
            return Err(format!("{name}: missing holder-set tag"));
        }
    }
    Ok(MaterializedEnchantmentDefinition {
        attributes: decode_enchantment_definition(&value),
        crossbow_charge_time: decode_unfiltered_value_effect(
            &value,
            "minecraft:crossbow_charge_time",
        ),
        trident_spin_attack_strength: decode_unfiltered_value_effect(
            &value,
            "minecraft:trident_spin_attack_strength",
        ),
        prevents_armor_change: nbt_field(fields, "effects")
            .and_then(|value| match value {
                NbtSummaryValue::Compound(effects) => Some(effects.as_slice()),
                _ => None,
            })
            .is_some_and(|effects| nbt_field(effects, "minecraft:prevent_armor_change").is_some()),
        tooltip: decode_enchantment_tooltip_definition(value),
    })
}

impl crate::RegistrySet {
    /// Resolves `EnchantmentEffectComponents.PREVENT_ARMOR_CHANGE` from the
    /// synchronized enchantment definition. Stubbed built-in entries use the
    /// generated vanilla identity fallback; server-provided custom definitions
    /// are read from their materialized `effects` compound.
    pub(crate) fn enchantment_prevents_armor_change(&self, holder_id: i32) -> Option<bool> {
        let entry = self.entry_by_holder_id(ENCHANTMENT_REGISTRY, holder_id)?;
        if let Some(definition) = self.typed.enchantments.get(&entry.id) {
            return Some(definition.prevents_armor_change);
        }
        (!entry.has_data).then(|| {
            VanillaEnchantmentIdentity::for_resource_id(&entry.id)
                == Some(VanillaEnchantmentIdentity::BindingCurse)
        })
    }
}

impl WorldStore {
    pub(crate) fn enchantment_prevents_armor_change(&self, holder_id: i32) -> Option<bool> {
        self.registries.enchantment_prevents_armor_change(holder_id)
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
                .map(|entry| {
                    self.registries
                        .typed
                        .enchantments
                        .get(&entry.id)
                        .map(|definition| definition.tooltip.clone())
                })
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
            let Some(definition) = self
                .registries
                .typed
                .enchantments
                .get(&entry.id)
                .and_then(|definition| definition.attributes.as_ref())
            else {
                continue;
            };
            for effect in &definition.attribute_effects {
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
    /// enchantments and replacements of the vanilla resource id. This
    /// ownerless compatibility query leaves random-dependent effects unchanged;
    /// living-owner presentation uses
    /// [`Self::crossbow_charge_duration_ticks_with_random`].
    pub fn crossbow_charge_duration_ticks(&self, enchantments: &[ItemEnchantmentSummary]) -> i32 {
        self.crossbow_charge_duration_ticks_with_optional_random::<LevelEventSoundRandomState>(
            enchantments,
            None,
        )
    }

    /// Vanilla `EnchantmentHelper.modifyCrossbowChargingTime`, using the
    /// living holder's mutable `Entity.random` projection.
    pub fn crossbow_charge_duration_ticks_with_random<
        R: bbb_registry_codec::random_source::RandomSource,
    >(
        &self,
        enchantments: &[ItemEnchantmentSummary],
        random: &mut R,
    ) -> i32 {
        self.crossbow_charge_duration_ticks_with_optional_random(enchantments, Some(random))
    }

    fn crossbow_charge_duration_ticks_with_optional_random<
        R: bbb_registry_codec::random_source::RandomSource,
    >(
        &self,
        enchantments: &[ItemEnchantmentSummary],
        mut random: Option<&mut R>,
    ) -> i32 {
        let mut duration_seconds = 1.25_f32;
        for enchantment in enchantments {
            let Some(entry) =
                self.registry_entry_by_holder_id(ENCHANTMENT_REGISTRY, enchantment.holder_id)
            else {
                continue;
            };
            let effect = match self.registries.typed.enchantments.get(&entry.id) {
                Some(definition) => definition.crossbow_charge_time.as_ref().map(Cow::Borrowed),
                None if !entry.has_data => {
                    vanilla_crossbow_charge_time_effect(&entry.id).map(Cow::Owned)
                }
                None => None,
            };
            if let Some(effect) = effect {
                duration_seconds = process_enchantment_value_effect_safe(
                    &effect,
                    enchantment.level,
                    duration_seconds,
                    &mut random,
                );
            }
        }
        (duration_seconds.max(0.0) * 20.0).floor() as i32
    }

    /// Applies synchronized
    /// `EnchantmentEffectComponents.TRIDENT_SPIN_ATTACK_STRENGTH` effects in
    /// stack order. Stubbed Riptide entries use the generated Vanilla identity
    /// fallback; synchronized definitions remain authoritative.
    #[cfg(test)]
    pub(crate) fn trident_spin_attack_strength(
        &self,
        enchantments: &[ItemEnchantmentSummary],
    ) -> f32 {
        process_trident_spin_attack_strength(
            trident_spin_attack_effects(&self.registries, enchantments),
            None,
        )
    }

    pub(crate) fn local_player_trident_spin_attack_strength(
        &mut self,
        enchantments: &[ItemEnchantmentSummary],
    ) -> f32 {
        let entity_id = self.local_player_id();
        let effects = trident_spin_attack_effects(&self.registries, enchantments);
        let Some(entity_id) = entity_id else {
            return process_trident_spin_attack_strength(effects, None);
        };
        self.entities
            .with_entity_random_mut(entity_id, |random| {
                process_trident_spin_attack_strength(effects, Some(random))
            })
            .unwrap_or_default()
    }
}

fn trident_spin_attack_effects<'a>(
    registries: &'a super::RegistrySet,
    enchantments: &'a [ItemEnchantmentSummary],
) -> impl Iterator<Item = (i32, Cow<'a, EnchantmentValueEffect>)> + 'a {
    enchantments.iter().filter_map(move |enchantment| {
        let entry = registries.entry_by_holder_id(ENCHANTMENT_REGISTRY, enchantment.holder_id)?;
        let effect = match registries.typed.enchantments.get(&entry.id) {
            Some(definition) => definition
                .trident_spin_attack_strength
                .as_ref()
                .map(Cow::Borrowed),
            None if !entry.has_data => {
                vanilla_trident_spin_attack_strength(&entry.id).map(Cow::Owned)
            }
            None => None,
        }?;
        Some((enchantment.level, effect))
    })
}

fn process_trident_spin_attack_strength<'a>(
    effects: impl IntoIterator<Item = (i32, Cow<'a, EnchantmentValueEffect>)>,
    mut random: Option<&mut LevelEventSoundRandomState>,
) -> f32 {
    effects.into_iter().fold(0.0, |strength, (level, effect)| {
        process_enchantment_value_effect_safe(&effect, level, strength, &mut random)
    })
}

/// Evaluates an `EnchantmentValueEffect` while guarding `RemoveBinomial`
/// against non-finite or `> i32::MAX` counts and degenerate `chance` values
/// that would overflow or loop in `while (y as f32) < n`.
fn process_enchantment_value_effect_safe<R: bbb_registry_codec::random_source::RandomSource>(
    effect: &EnchantmentValueEffect,
    level: i32,
    input: f32,
    random: &mut Option<&mut R>,
) -> f32 {
    match effect {
        EnchantmentValueEffect::AllOf(effects) => effects.iter().fold(input, |value, child| {
            process_enchantment_value_effect_safe(child, level, value, random)
        }),
        EnchantmentValueEffect::RemoveBinomial(chance_value) => {
            let Some(random) = random.as_deref_mut() else {
                return input;
            };
            let chance = chance_value.calculate(level);
            if !input.is_finite() || input <= 0.0 || !chance.is_finite() || chance <= 0.0 {
                return input;
            }
            if chance >= 1.0 {
                return 0.0;
            }
            const MAX_EXACT_BERNOULLI_TRIALS: u32 = 16_384;
            let n = input;
            let drop = if n <= 128.0
                || ((n * chance < 20.0 || n * (1.0 - chance) < 20.0)
                    && n <= MAX_EXACT_BERNOULLI_TRIALS as f32)
            {
                let trials = n.ceil().clamp(0.0, MAX_EXACT_BERNOULLI_TRIALS as f32) as u32;
                let mut drop = 0_u32;
                for _ in 0..trials {
                    drop += u32::from(random.next_float() < chance);
                }
                drop as f32
            } else {
                let n64 = f64::from(n);
                let p64 = f64::from(chance);
                let mean = (n64 * p64).floor();
                let sigma = (n64 * p64 * (1.0 - p64)).max(0.0).sqrt();
                (mean + random.next_gaussian() * sigma + 0.5)
                    .floor()
                    .clamp(0.0, n64) as f32
            };
            (n - drop).max(0.0)
        }
        other => other.process(level, input, random),
    }
}

fn decode_enchantment_tooltip_definition(value: NbtSummaryValue) -> EnchantmentTooltipDefinition {
    let NbtSummaryValue::Compound(fields) = value else {
        unreachable!("canonical enchantment compound");
    };
    let NbtSummaryValue::Int(max_level) =
        nbt_field(&fields, "max_level").expect("canonical enchantment level")
    else {
        unreachable!("canonical enchantment integer");
    };
    let max_level = *max_level;
    let (description, description_styled, description_nbt) = canonical_description(fields);
    let description_fields = nbt_compound(Some(&description_nbt));
    let description_translation_key =
        description_fields.and_then(|fields| nbt_string(nbt_field(fields, "translate")));
    let description_translation_fallback =
        description_fields.and_then(|fields| nbt_string(nbt_field(fields, "fallback")));
    EnchantmentTooltipDefinition {
        description,
        description_styled,
        description_nbt: Some(description_nbt),
        description_translation_key,
        description_translation_fallback,
        max_level,
    }
}

fn decode_unfiltered_value_effect(
    value: &NbtSummaryValue,
    effect_id: &str,
) -> Option<EnchantmentValueEffect> {
    let fields = nbt_compound(Some(value)).expect("canonical enchantment compound");
    let effects =
        nbt_compound(Some(nbt_field(fields, "effects")?)).expect("canonical enchantment effects");
    nbt_field(effects, effect_id).map(EnchantmentValueEffect::from_canonical)
}

fn vanilla_crossbow_charge_time_effect(resource_id: &str) -> Option<EnchantmentValueEffect> {
    (VanillaEnchantmentIdentity::for_resource_id(resource_id)
        == Some(VanillaEnchantmentIdentity::QuickCharge))
    .then_some(EnchantmentValueEffect::Add(LevelBasedValue::Linear {
        base: -0.25,
        per_level_above_first: -0.25,
    }))
}

fn vanilla_trident_spin_attack_strength(resource_id: &str) -> Option<EnchantmentValueEffect> {
    (VanillaEnchantmentIdentity::for_resource_id(resource_id)
        == Some(VanillaEnchantmentIdentity::Riptide))
    .then_some(EnchantmentValueEffect::Add(LevelBasedValue::Linear {
        base: 1.5,
        per_level_above_first: 0.75,
    }))
}

fn decode_enchantment_definition(value: &NbtSummaryValue) -> Option<EnchantmentDefinition> {
    let fields = nbt_compound(Some(value)).expect("canonical enchantment compound");
    let effects =
        nbt_compound(Some(nbt_field(fields, "effects")?)).expect("canonical enchantment effects");
    let attribute_effects = nbt_list(Some(nbt_field(effects, "minecraft:attributes")?))
        .expect("canonical attribute effects")
        .iter()
        .map(parse_attribute_effect)
        .collect();
    let slots = nbt_list(nbt_field(fields, "slots"))
        .expect("canonical enchantment slots")
        .iter()
        .map(|value| nbt_string(Some(value)).expect("canonical enchantment slot"))
        .collect();
    Some(EnchantmentDefinition {
        slots,
        attribute_effects,
    })
}

fn parse_attribute_effect(value: &NbtSummaryValue) -> EnchantmentAttributeEffect {
    let fields = nbt_compound(Some(value)).expect("canonical attribute effect");
    let field = |name| nbt_field(fields, name).expect("canonical attribute effect field");
    let NbtSummaryValue::String(operation) = field("operation") else {
        unreachable!("canonical attribute operation");
    };
    EnchantmentAttributeEffect {
        id: nbt_string(Some(field("id"))).expect("canonical attribute modifier id"),
        attribute: nbt_string(Some(field("attribute"))).expect("canonical attribute id"),
        amount: LevelBasedValue::from_canonical(field("amount")),
        operation_id: operation
            .with_utf8(attribute_modifier_operation_id)
            .flatten()
            .expect("canonical attribute operation"),
    }
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

fn nbt_string(value: Option<&NbtSummaryValue>) -> Option<String> {
    let NbtSummaryValue::String(value) = value? else {
        return None;
    };
    value.try_to_string().ok()
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        packets::{AddEntity, RegistryData, RegistryDataEntry, Vec3d},
        ComponentStyle,
    };
    use uuid::Uuid;

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
                "weight":1,
                "max_level":1,
                "min_cost":{"base":0,"per_level_above_first":0},
                "max_cost":{"base":0,"per_level_above_first":0},
                "anvil_cost":0,
                "slots":["body"],
                "effects":{"minecraft:prevent_armor_change":{}}
            }"#,
        )
        .unwrap();
        let value = super::super::revision::exact_network_value(
            ENCHANTMENT_REGISTRY,
            &data,
            world.frozen_registry_access(),
        )
        .unwrap();
        let custom =
            materialize_enchantment_definition(value, world.frozen_registry_access()).unwrap();
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
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

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
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

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
                description_nbt: Some(NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "color".into(),
                        value: NbtSummaryValue::String("#123456".into()),
                    },
                    bbb_protocol::NbtSummaryEntry {
                        name: "translate".into(),
                        value: NbtSummaryValue::String("enchantment.example.dual_effect".into()),
                    },
                ])),
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
                "supported_items": "minecraft:stick",
                "weight": 1,
                "max_level": 3,
                "min_cost": {"base": 0, "per_level_above_first": 0},
                "max_cost": {"base": 0, "per_level_above_first": 0},
                "anvil_cost": 0,
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

        let world = WorldStore::new();
        let value = super::super::revision::exact_network_value(
            ENCHANTMENT_REGISTRY,
            &data,
            world.frozen_registry_access(),
        )
        .unwrap();
        let definition = decode_enchantment_definition(&value).unwrap();
        let tooltip = decode_enchantment_tooltip_definition(value);
        assert_eq!(
            tooltip.description_translation_key.as_deref(),
            Some("enchantment.example.known")
        );
        assert_eq!(tooltip.max_level, 3);
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
            LevelBasedValue::LevelsSquared { added: 0.0 }.calculate(46_341),
            -2_147_479_040.0_f32
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
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();
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
    fn exponential_crossbow_charge_time_rounds_after_the_complete_effect() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "example:exponential_charge".to_string(),
                raw_data: Some(crossbow_effect_enchantment_nbt(compound_payload(vec![
                    string_tag("type", "minecraft:exponential"),
                    float_tag("base", 0.447_213_6),
                    float_tag("exponent", 2.0),
                ]))),
            }],
            raw_payload_len: 0,
        });
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

        assert_eq!(
            world.crossbow_charge_duration_ticks(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 1,
            }]),
            5
        );
    }

    #[test]
    fn crossbow_charge_time_server_definition_overrides_vanilla_resource_fallback() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:quick_charge".to_string(),
                raw_data: Some(minimal_enchantment_nbt(vec![compound_tag(
                    "effects",
                    Vec::new(),
                )])),
            }],
            raw_payload_len: 0,
        });
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();

        assert_eq!(
            world.crossbow_charge_duration_ticks(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            }]),
            25
        );
    }

    #[test]
    fn trident_spin_attack_strength_applies_stub_fallback_and_server_override() {
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:riptide".to_string(),
                raw_data: None,
            }],
            raw_payload_len: 0,
        });
        assert_eq!(
            world.trident_spin_attack_strength(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 1,
            }]),
            1.5
        );
        assert_eq!(
            world.trident_spin_attack_strength(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            }]),
            3.0
        );

        let mut overridden = WorldStore::new();
        overridden.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:riptide".to_string(),
                raw_data: Some(minimal_enchantment_nbt(vec![compound_tag(
                    "effects",
                    Vec::new(),
                )])),
            }],
            raw_payload_len: 0,
        });
        overridden.begin_registry_configuration();
        overridden.finish_registry_configuration().unwrap();
        assert_eq!(
            overridden.trident_spin_attack_strength(&[ItemEnchantmentSummary {
                holder_id: 0,
                level: 3,
            }]),
            0.0
        );
    }

    #[test]
    fn value_effects_cover_every_vanilla_26_1_variant() {
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

        assert_eq!(
            effect.process::<LevelEventSoundRandomState>(2, 99.0, &mut None),
            1.5
        );
        assert_eq!(
            EnchantmentValueEffect::from_canonical(&NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:remove_binomial"),
                },
                NbtSummaryEntry {
                    name: "chance".into(),
                    value: NbtSummaryValue::Float(0.5_f32.to_bits()),
                },
            ])),
            EnchantmentValueEffect::RemoveBinomial(LevelBasedValue::Constant(0.5))
        );
    }

    #[test]
    fn remove_binomial_matches_vanilla_small_and_gaussian_random_advancement() {
        let effect = EnchantmentValueEffect::RemoveBinomial(LevelBasedValue::Constant(0.5));
        let mut small_random = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(effect.process(1, 5.0, &mut Some(&mut small_random)), 4.0);
        assert_eq!(small_random.next_float(), 0.309_050_56);

        let mut gaussian_random = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(
            effect.process(1, 200.0, &mut Some(&mut gaussian_random)),
            94.0
        );
        assert_eq!(
            effect.process(1, 200.0, &mut Some(&mut gaussian_random)),
            106.0
        );
        assert_eq!(gaussian_random.next_float(), 0.637_417_4);
    }

    #[test]
    fn crossbow_random_effect_uses_one_mutable_holder_cursor() {
        let effect = compound_payload(vec![
            string_tag("type", "minecraft:all_of"),
            compound_list_tag(
                "effects",
                vec![
                    compound_payload(vec![
                        string_tag("type", "minecraft:set"),
                        float_tag("value", 5.0),
                    ]),
                    compound_payload(vec![
                        string_tag("type", "minecraft:remove_binomial"),
                        float_tag("chance", 0.5),
                    ]),
                ],
            ),
        ]);
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "example:random_loading".to_string(),
                raw_data: Some(crossbow_effect_enchantment_nbt(effect)),
            }],
            raw_payload_len: 0,
        });
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();
        let enchantments = [ItemEnchantmentSummary {
            holder_id: 0,
            level: 1,
        }];

        assert_eq!(world.crossbow_charge_duration_ticks(&enchantments), 100);
        let mut random = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(
            world.crossbow_charge_duration_ticks_with_random(&enchantments, &mut random),
            80
        );
        assert_eq!(
            world.crossbow_charge_duration_ticks_with_random(&enchantments, &mut random),
            60
        );
    }

    #[test]
    fn local_trident_random_effect_advances_the_canonical_player_cursor() {
        let effect = compound_payload(vec![
            string_tag("type", "minecraft:all_of"),
            compound_list_tag(
                "effects",
                vec![
                    compound_payload(vec![
                        string_tag("type", "minecraft:set"),
                        float_tag("value", 5.0),
                    ]),
                    compound_payload(vec![
                        string_tag("type", "minecraft:remove_binomial"),
                        float_tag("chance", 0.5),
                    ]),
                ],
            ),
        ]);
        let mut world = WorldStore::new();
        world.record_registry_data(RegistryData {
            registry: ENCHANTMENT_REGISTRY.to_string(),
            entries: vec![RegistryDataEntry {
                id: "example:random_riptide".to_string(),
                raw_data: Some(trident_effect_enchantment_nbt(effect)),
            }],
            raw_payload_len: 0,
        });
        world.begin_registry_configuration();
        world.finish_registry_configuration().unwrap();
        world.apply_add_entity(AddEntity {
            id: 7,
            uuid: Uuid::from_u128(7),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        world.local_player_id = Some(7);
        world
            .entities
            .with_entity_random_mut(7, |random| {
                *random = LevelEventSoundRandomState::with_seed(0);
            })
            .unwrap();
        let enchantments = [ItemEnchantmentSummary {
            holder_id: 0,
            level: 1,
        }];

        assert_eq!(
            world.local_player_trident_spin_attack_strength(&enchantments),
            4.0
        );
        assert_eq!(
            world.local_player_trident_spin_attack_strength(&enchantments),
            3.0
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
            string_tag("supported_items", "minecraft:stick"),
            int_tag("weight", 1),
            int_tag("max_level", 7),
            compound_tag(
                "min_cost",
                vec![int_tag("base", 0), int_tag("per_level_above_first", 0)],
            ),
            compound_tag(
                "max_cost",
                vec![int_tag("base", 0), int_tag("per_level_above_first", 0)],
            ),
            int_tag("anvil_cost", 0),
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
        minimal_enchantment_nbt(vec![compound_tag(
            "effects",
            vec![named_compound_payload(
                "minecraft:crossbow_charge_time",
                effect_payload,
            )],
        )])
    }

    fn trident_effect_enchantment_nbt(effect_payload: Vec<u8>) -> Vec<u8> {
        minimal_enchantment_nbt(vec![compound_tag(
            "effects",
            vec![named_compound_payload(
                "minecraft:trident_spin_attack_strength",
                effect_payload,
            )],
        )])
    }

    fn minimal_enchantment_nbt(mut fields: Vec<Vec<u8>>) -> Vec<u8> {
        fields.extend([
            string_tag("description", "Example"),
            string_tag("supported_items", "minecraft:stick"),
            int_tag("weight", 1),
            int_tag("max_level", 1),
            compound_tag(
                "min_cost",
                vec![int_tag("base", 0), int_tag("per_level_above_first", 0)],
            ),
            compound_tag(
                "max_cost",
                vec![int_tag("base", 0), int_tag("per_level_above_first", 0)],
            ),
            int_tag("anvil_cost", 0),
            string_list_tag("slots", &["mainhand"]),
        ]);
        compound_payload(fields)
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
