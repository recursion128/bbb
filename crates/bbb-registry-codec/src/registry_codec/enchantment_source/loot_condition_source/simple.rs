//! Exact non-recursive `LootItemCondition.DIRECT_CODEC` map-codec families.
//!
//! The generated dispatch membership comes from `LootItemConditions.bootstrap`;
//! payload order and validation facts come from each owning `MAP_CODEC` and
//! `LootContextUser`/`Validatable` implementation

use crate::{
    codec_dispatch_types::{loot_condition_codec_type, LootConditionCodecFamily},
    loot_context_types::{vanilla_loot_entity_target, VanillaLootContextParam},
    persistent_component_nbt::source_codec::{
        decode_fixed_holder_reference_source, java_float_compare,
    },
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::{
    nbt_summary::canonical_float_tag_bits, normalize_resource_location, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::super::{
    int_range::decode_int_range_source,
    loot_context_value::{extend_context_params, LootContextCodecValue},
    number_provider::decode_number_provider_source,
    value::{child, decode_level_based_value_source},
};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";

/// Decodes the ten non-recursive loot-condition families owned by this module.
///
/// Dispatch inspection is deliberately uncharged. The enclosing root can call
/// multiple disjoint family modules without making the safety budget depend on
/// their order. Once this function recognizes an owned family, its root record
/// and discriminator are charged exactly once.
pub(super) fn decode_simple_condition_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<LootContextCodecValue, RegistryCodecError>> {
    let raw_condition = input.field("condition")?.utf8_string()?;
    let condition = normalize_resource_location(raw_condition).ok()?;
    let family = loot_condition_codec_type(&condition)?.codec_family;
    if !is_owned(family) {
        return None;
    }
    Some(decode_owned(family, condition, input, field, context))
}

fn is_owned(family: LootConditionCodecFamily) -> bool {
    matches!(
        family,
        LootConditionCodecFamily::LootItemRandomChanceCondition
            | LootConditionCodecFamily::LootItemRandomChanceWithEnchantedBonusCondition
            | LootConditionCodecFamily::LootItemKilledByPlayerCondition
            | LootConditionCodecFamily::EntityHasScoreCondition
            | LootConditionCodecFamily::BonusLevelTableCondition
            | LootConditionCodecFamily::ExplosionCondition
            | LootConditionCodecFamily::WeatherCheck
            | LootConditionCodecFamily::TimeCheck
            | LootConditionCodecFamily::ValueCheckCondition
            | LootConditionCodecFamily::EnchantmentActiveCheck
    )
}

fn decode_owned(
    family: LootConditionCodecFamily,
    condition: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "loot condition")?;
    context.charge_node(&child(field, "condition"))?;
    let mut output = ConditionOutput::default();
    match family {
        LootConditionCodecFamily::LootItemRandomChanceCondition => {
            let chance = decode_required_number(input, "chance", field, context)?;
            // This owner deliberately does not override validate or
            // getReferencedContextParams: NumberProvider usage is not reported.
            output.record.push("chance", chance.canonical);
        }
        LootConditionCodecFamily::LootItemRandomChanceWithEnchantedBonusCondition => {
            let chance = decode_float_field(input, "unenchanted_chance", field, context)?;
            if java_float_compare(chance, 0.0).is_lt() || java_float_compare(chance, 1.0).is_gt() {
                return Err(context.codec_error(
                    &child(field, "unenchanted_chance"),
                    "float is outside 0..=1",
                ));
            }
            output
                .record
                .push_value("unenchanted_chance", float(chance));
            let enchanted = decode_required_level(input, "enchanted_chance", field, context)?;
            output.record.push("enchanted_chance", enchanted);
            output.record.push(
                "enchantment",
                decode_required_holder(input, "enchantment", field, context, ENCHANTMENT_REGISTRY)?,
            );
            output.push_context_param(VanillaLootContextParam::AttackingEntity);
        }
        LootConditionCodecFamily::LootItemKilledByPlayerCondition => {
            output.push_context_param(VanillaLootContextParam::LastDamagePlayer);
        }
        LootConditionCodecFamily::EntityHasScoreCondition => {
            let scores = decode_scores(input, field, context)?;
            let entity = decode_entity_target(input, field, context)?;
            output.push_context_param(entity.context_param);
            output.push_loot_value("scores", scores);
            output.record.push_value(
                "entity",
                NbtSummaryValue::String(entity.serialized_name.into()),
            );
        }
        LootConditionCodecFamily::BonusLevelTableCondition => {
            output.record.push(
                "enchantment",
                decode_required_holder(input, "enchantment", field, context, ENCHANTMENT_REGISTRY)?,
            );
            output
                .record
                .push("chances", decode_chances(input, field, context)?);
            output.push_context_param(VanillaLootContextParam::Tool);
        }
        LootConditionCodecFamily::ExplosionCondition => {
            output.push_context_param(VanillaLootContextParam::ExplosionRadius);
        }
        LootConditionCodecFamily::WeatherCheck => {
            output.push_optional_bool(input, "raining", field, context)?;
            output.push_optional_bool(input, "thundering", field, context)?;
        }
        LootConditionCodecFamily::TimeCheck => {
            output.record.push(
                "clock",
                decode_required_holder(input, "clock", field, context, WORLD_CLOCK_REGISTRY)?,
            );
            output.push_optional_long(input, "period", field, context)?;
            let value = decode_required_range(input, "value", field, context)?;
            output.push_loot_value("value", value);
        }
        LootConditionCodecFamily::ValueCheckCondition => {
            let value = decode_required_number(input, "value", field, context)?;
            output.push_loot_value("value", value);
            let range = decode_required_range(input, "range", field, context)?;
            output.push_loot_value("range", range);
        }
        LootConditionCodecFamily::EnchantmentActiveCheck => {
            output.push_required_bool(input, "active", field, context)?;
            output.push_context_param(VanillaLootContextParam::EnchantmentActive);
        }
        _ => unreachable!("only owned loot-condition families reach decode_owned"),
    }
    Ok(output.finish(condition))
}

#[derive(Default)]
struct ConditionOutput {
    record: RegistryCodecRecord,
    context_params: Vec<VanillaLootContextParam>,
}

impl ConditionOutput {
    fn push_loot_value(&mut self, name: &str, value: LootContextCodecValue) {
        self.record.push(name, value.canonical);
        extend_context_params(&mut self.context_params, value.referenced_context_params);
    }

    fn push_context_param(&mut self, param: VanillaLootContextParam) {
        extend_context_params(&mut self.context_params, [param]);
    }

    fn push_optional_bool(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let Some(input) = input.field(name) else {
            return Ok(());
        };
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = input
            .as_bool()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.BOOL input"))?;
        self.record
            .push_value(name, NbtSummaryValue::Byte(i8::from(value)));
        Ok(())
    }

    fn push_required_bool(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = required_field(input, name, field, context)?
            .as_bool()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.BOOL input"))?;
        self.record
            .push_value(name, NbtSummaryValue::Byte(i8::from(value)));
        Ok(())
    }

    fn push_optional_long(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let Some(input) = input.field(name) else {
            return Ok(());
        };
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = input
            .codec_i64()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.LONG input"))?;
        self.record.push_value(name, NbtSummaryValue::Long(value));
        Ok(())
    }

    fn finish(mut self, condition: String) -> LootContextCodecValue {
        self.record
            .push_value("condition", NbtSummaryValue::String(condition.into()));
        let canonical = self.record.finish();
        LootContextCodecValue {
            canonical,
            referenced_context_params: self.context_params,
            semantic_constant_value_bits: None,
        }
    }
}

fn decode_required_number(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    context.with_nested(&nested, |context| {
        decode_number_provider_source(value, &nested, context)
    })
}

fn decode_required_range(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    context.with_nested(&nested, |context| {
        decode_int_range_source(value, &nested, context)
    })
}

fn decode_required_level(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    context.with_nested(&nested, |context| {
        decode_level_based_value_source(value, &nested, context)
    })
}

fn decode_required_holder(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    context.with_nested(&nested, |context| {
        decode_fixed_holder_reference_source(value, &nested, context, registry)
    })
}

fn decode_float_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let nested = child(field, name);
    context.charge_node(&nested)?;
    required_field(input, name, field, context)?
        .codec_f32()
        .ok_or_else(|| context.codec_error(&nested, "expected Codec.FLOAT input"))
}

fn decode_entity_target(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<&'static crate::loot_context_types::VanillaLootEntityTarget, RegistryCodecError> {
    let nested = child(field, "entity");
    context.charge_node(&nested)?;
    let raw = required_field(input, "entity", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(&nested, "expected an entity target string"))?;
    vanilla_loot_entity_target(&raw)
        .ok_or_else(|| context.codec_error(&nested, format!("unknown entity target `{raw}`")))
}

fn decode_scores(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let nested = child(field, "scores");
    let scores = required_field(input, "scores", field, context)?;
    context.with_nested(&nested, |context| {
        let len = context.charge_record(scores, &nested, "score range map")?;
        let visible = scores.visible_map_entries(len);
        let mut fields = Vec::with_capacity(visible.len());
        let mut references = Vec::new();
        let mut referenced_context_params = Vec::new();
        for (name, value) in visible {
            let value_field = format!("{nested}[{}]", name.to_display_string());
            let decoded = context.with_nested(&value_field, |context| {
                decode_int_range_source(value, &value_field, context)
            })?;
            let (canonical, nested_references) = decoded.canonical.into_parts();
            fields.push(NbtSummaryEntry {
                name: name.clone(),
                value: canonical,
            });
            references.extend(nested_references);
            extend_context_params(
                &mut referenced_context_params,
                decoded.referenced_context_params,
            );
        }
        let canonical =
            CanonicalRegistryCodecValue::from_parts(NbtSummaryValue::Compound(fields), references);
        Ok(LootContextCodecValue {
            canonical,
            referenced_context_params,
            semantic_constant_value_bits: None,
        })
    })
}

fn decode_chances(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let nested = child(field, "chances");
    let chances = required_field(input, "chances", field, context)?;
    context.with_nested(&nested, |context| {
        context.charge_node(&nested)?;
        let len = chances
            .list_len()
            .ok_or_else(|| context.codec_error(&nested, "expected a Codec.FLOAT list"))?;
        context.check_collection(&nested, len)?;
        if len == 0 {
            return Err(context.codec_error(&nested, "expected a non-empty list"));
        }
        let mut values = Vec::with_capacity(len);
        for index in 0..len {
            let item_field = format!("{nested}[{index}]");
            let item = chances
                .list_element(index)
                .expect("chance list length and indexed values stay consistent");
            let value = context.with_nested(&item_field, |context| {
                context.charge_node(&item_field)?;
                item.codec_f32()
                    .ok_or_else(|| context.codec_error(&item_field, "expected Codec.FLOAT input"))
            })?;
            values.push(float(value));
        }
        Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::List(
            values,
        )))
    })
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

#[cfg(test)]
mod tests;
