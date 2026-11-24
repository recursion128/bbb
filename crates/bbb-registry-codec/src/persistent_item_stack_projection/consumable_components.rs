//! `minecraft:food` and `minecraft:consumable` in both directions, transcribed
//! from `FoodProperties.DIRECT_CODEC` and
//! `Consumable.CODEC` over `ConsumeEffect.CODEC`

use crate::registry_codec::RegistryElementValueRef;
use bbb_facts::consume_effect_types::ConsumeEffectType;
use bbb_facts::item_component_value_types::ItemUseAnimationSummary;
use bbb_facts::mob_effect_types::{
    vanilla_mob_effect_type, vanilla_mob_effect_type_for_resource_id,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::{codec_float_value, codec_int_value, NbtSummaryDecodeLimits};
use bbb_protocol::packets::{
    ConsumableSummary, ConsumeEffectSummary, MobEffectInstanceSummary, SoundEventSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::encoding::{ComponentEncoder, ComponentNbtEncoder, ComponentScalar as Scalar};
use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{
    project_sound_event_with_content_registry, sound_event_resource_id_in_registry,
    MOB_EFFECT_REGISTRY,
};
use super::mob_effect_details::{append_details, project_details};
use super::nbt_support::{
    checked_float, compound_fields, latest_field, list_entries, nbt_string, persistent_float_nbt,
    ComponentReader, MAX_PERSISTENT_LIST_ITEMS,
};

/// `Consumable.DEFAULT_CONSUME_SECONDS` (`Consumable.java:35`).
const PERSISTENT_CONSUME_SECONDS_DEFAULT_BITS: u32 = 1.6_f32.to_bits();
/// `SoundEvents.GENERIC_EAT`, the `optionalFieldOf("sound", ...)` default
/// (`Consumable.java:42`).
const PERSISTENT_CONSUME_SOUND_DEFAULT: &str = "minecraft:entity.generic.eat";
/// `TeleportRandomlyConsumeEffect.CODEC`'s `optionalFieldOf("diameter", 16.0F)`
/// (`TeleportRandomlyConsumeEffect.java:23-25`).
const PERSISTENT_TELEPORT_DIAMETER_DEFAULT_BITS: u32 = 16.0_f32.to_bits();
pub trait ConsumableComponentEncoder: ComponentEncoder {
    fn sound_is_default(&self, sound: &SoundEventSummary, resource_id: &str) -> bool;
    fn mob_effect(&self, holder_id: i32) -> std::result::Result<Self::Value, Self::Error>;
    fn mob_effect_set(
        &self,
        ids: &[i32],
        tag: Option<&str>,
    ) -> std::result::Result<Self::Value, Self::Error>;
}

#[inline(always)]
pub fn encode_consumable<E: ConsumableComponentEncoder>(
    encoder: &E,
    value: ConsumableSummary,
    sound: &SoundEventSummary,
    has_consume_particles: bool,
    effects: &[ConsumeEffectSummary],
) -> std::result::Result<E::Value, E::Error> {
    let mut fields = encoder.fields(0);
    if value.consume_seconds.to_bits() != PERSISTENT_CONSUME_SECONDS_DEFAULT_BITS {
        encoder.scalar(
            &mut fields,
            "consume_seconds",
            Scalar::Float(value.consume_seconds.to_bits()),
        );
    }
    if value.animation != ItemUseAnimationSummary::Eat {
        encoder.scalar(
            &mut fields,
            "animation",
            Scalar::String(value.animation.as_str()),
        );
    }
    if !encoder.sound_is_default(sound, PERSISTENT_CONSUME_SOUND_DEFAULT) {
        encoder.field(&mut fields, "sound", encoder.sound(sound)?);
    }
    if !has_consume_particles {
        encoder.scalar(&mut fields, "has_consume_particles", Scalar::Boolean(false));
    }
    if !effects.is_empty() {
        encoder.field(
            &mut fields,
            "on_consume_effects",
            encoder.list(
                effects
                    .iter()
                    .map(|effect| encode_consume_effect(encoder, effect)),
            )?,
        );
    }
    Ok(encoder.record(fields))
}

fn tagged<E: ComponentEncoder>(encoder: &E, kind: &str, capacity: usize) -> E::Fields {
    let mut fields = encoder.fields(capacity);
    encoder.scalar(&mut fields, "type", Scalar::String(kind));
    fields
}

pub fn encode_consume_effect<E: ConsumableComponentEncoder>(
    encoder: &E,
    effect: &ConsumeEffectSummary,
) -> std::result::Result<E::Value, E::Error> {
    let fields = match effect {
        ConsumeEffectSummary::ApplyEffects {
            effects,
            probability_bits,
        } => {
            let probability = f32::from_bits(*probability_bits);
            encoder.check(
                (0.0..=1.0).contains(&probability),
                "effect probability outside 0.0..=1.0",
            )?;
            let mut fields = tagged(
                encoder,
                "minecraft:apply_effects",
                2 + usize::from(probability != 1.0),
            );
            encoder.field(
                &mut fields,
                "effects",
                encoder.list(effects.iter().map(|effect| {
                    encode_mob_effect_instance(
                        encoder,
                        effect,
                        effect.hidden_effect.as_deref(),
                        NbtSummaryDecodeLimits::default().max_depth,
                    )
                }))?,
            );
            if probability != 1.0 {
                encoder.scalar(&mut fields, "probability", Scalar::Float(*probability_bits));
            }
            fields
        }
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } => {
            let effects = encoder.mob_effect_set(effect_registry_ids, effect_tag.as_deref())?;
            let mut fields = tagged(encoder, "minecraft:remove_effects", 2);
            encoder.field(&mut fields, "effects", effects);
            fields
        }
        ConsumeEffectSummary::ClearAllEffects => tagged(encoder, "minecraft:clear_all_effects", 1),
        ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            let diameter = f32::from_bits(*diameter_bits);
            encoder.check(
                diameter > 0.0 && diameter <= f32::MAX,
                "teleport diameter must be finite and positive",
            )?;
            let present = *diameter_bits != PERSISTENT_TELEPORT_DIAMETER_DEFAULT_BITS;
            let mut fields = tagged(
                encoder,
                "minecraft:teleport_randomly",
                1 + usize::from(present),
            );
            if present {
                encoder.scalar(&mut fields, "diameter", Scalar::Float(*diameter_bits));
            }
            fields
        }
        ConsumeEffectSummary::PlaySound { sound } => {
            let sound = encoder.sound(sound)?;
            let mut fields = tagged(encoder, "minecraft:play_sound", 2);
            encoder.field(&mut fields, "sound", sound);
            fields
        }
    };
    Ok(encoder.record(fields))
}

pub fn encode_mob_effect_instance<E: ConsumableComponentEncoder>(
    encoder: &E,
    effect: &MobEffectInstanceSummary,
    hidden: Option<&bbb_protocol::packets::MobEffectDetailsSummary>,
    max_hidden_depth: usize,
) -> std::result::Result<E::Value, E::Error> {
    let id = encoder.mob_effect(effect.effect_id)?;
    let mut fields = encoder.fields(1);
    encoder.field(&mut fields, "id", id);
    append_details(
        encoder,
        &mut fields,
        effect.amplifier,
        effect.duration,
        effect.ambient,
        effect.show_particles,
        effect.show_icon,
        hidden,
        0,
        max_hidden_depth,
    )?;
    Ok(encoder.record(fields))
}

impl ConsumableComponentEncoder for ComponentNbtEncoder<'_, ()> {
    fn sound_is_default(&self, sound: &SoundEventSummary, resource_id: &str) -> bool {
        sound.registry_id.is_some_and(|id| {
            sound_event_resource_id_in_registry(id, self.content_registry) == Some(resource_id)
        })
    }

    fn mob_effect(&self, holder_id: i32) -> Result<Self::Value> {
        let named = vanilla_mob_effect_type(holder_id).ok_or_else(|| {
            invalid_persistent_component(
                self.type_id,
                self.resource_id,
                format!("unknown {MOB_EFFECT_REGISTRY} holder id {holder_id}"),
            )
        })?;
        Ok(NbtSummaryValue::String(named.resource_id.into()))
    }

    fn mob_effect_set(&self, ids: &[i32], tag: Option<&str>) -> Result<Self::Value> {
        persistent_holder_set_nbt(self.type_id, self.resource_id, ids, tag)
    }
}

/// `FoodProperties.DIRECT_CODEC`:
/// `NON_NEGATIVE_INT nutrition`, `Codec.FLOAT saturation` and
/// `Codec.BOOL.optionalFieldOf("can_always_eat", false)`, which drops the flag
/// while encoding when it is `false`.
pub(super) fn persistent_food_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let missing = || ItemStackPersistentProjectionError::UnsupportedComponent {
        type_id,
        resource_id,
    };
    let nutrition = patch.food_nutrition.ok_or_else(missing)?;
    if nutrition < 0 {
        return Err(invalid(format!("value must be non-negative: {nutrition}")));
    }
    let saturation_bits = patch.food_saturation_bits.ok_or_else(missing)?;
    let mut fields = vec![
        NbtSummaryEntry {
            name: "nutrition".into(),
            value: NbtSummaryValue::Int(nutrition),
        },
        NbtSummaryEntry {
            name: "saturation".into(),
            value: persistent_float_nbt(saturation_bits),
        },
    ];
    if patch.food_can_always_eat {
        fields.push(NbtSummaryEntry {
            name: "can_always_eat".into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// The wire projection of `FoodProperties.DIRECT_CODEC`
pub(super) fn project_food(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    patch: &mut bbb_protocol::packets::DataComponentPatchSummary,
) -> Result<()> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields =
        compound_fields(value).ok_or_else(|| invalid("food is not a compound".to_string()))?;
    patch.food_nutrition = Some(
        latest_field(fields, "nutrition")
            .and_then(codec_int_value)
            .filter(|nutrition| *nutrition >= 0)
            .ok_or_else(|| invalid("food nutrition is not a non-negative integer".to_string()))?,
    );
    patch.food_saturation_bits = Some(
        latest_field(fields, "saturation")
            .and_then(codec_float_value)
            .ok_or_else(|| invalid("food saturation is not a float".to_string()))?
            .to_bits(),
    );
    patch.food_can_always_eat = latest_field(fields, "can_always_eat")
        .map(|value| ComponentReader::new(type_id, resource_id).flag(value, "can_always_eat"))
        .transpose()?
        .unwrap_or(false);
    Ok(())
}

/// `Consumable.CODEC`: every field is an
/// `optionalFieldOf` whose default is dropped while encoding --
/// `consume_seconds` at `1.6F`, `animation` at `EAT`, `sound` at
/// `SoundEvents.GENERIC_EAT`, `has_consume_particles` at `true` and
/// `on_consume_effects` at the empty list.
pub(super) fn persistent_consumable_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let missing = || ItemStackPersistentProjectionError::UnsupportedComponent {
        type_id,
        resource_id,
    };
    let consumable = patch.consumable.ok_or_else(missing)?;
    // `ExtraCodecs.NON_NEGATIVE_FLOAT` (`Consumable`,
    // `ExtraCodecs`) also rejects NaN, infinity, and -0.0 while encoding.
    checked_float(
        type_id,
        resource_id,
        consumable.consume_seconds,
        0.0,
        f32::MAX,
        "consume_seconds",
    )?;
    let sound = patch.consumable_sound.as_ref().ok_or_else(missing)?;
    let has_consume_particles = patch.consumable_has_consume_particles.ok_or_else(missing)?;
    encode_consumable(
        &ComponentNbtEncoder::new(type_id, resource_id, (), content_registry),
        consumable,
        sound,
        has_consume_particles,
        &patch.consumable_effects,
    )
}

/// `ConsumeEffect.CODEC` is `BuiltInRegistries.CONSUME_EFFECT_TYPE.byNameCodec()
/// .dispatch(ConsumeEffect::getType, ConsumeEffect.Type::codec)`
/// so each entry is a compound carrying the type's
/// registry name.
pub(super) fn persistent_consume_effect_nbt(
    type_id: i32,
    resource_id: &'static str,
    effect: &ConsumeEffectSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    encode_consume_effect(
        &ComponentNbtEncoder::new(type_id, resource_id, (), content_registry),
        effect,
    )
}

/// `MobEffectInstance.CODEC` over
/// `MobEffectInstance.Details.MAP_CODEC`:
/// the effect's identifier beside `amplifier` (`UNSIGNED_BYTE`, default `0`),
/// `duration` (default `0`), `ambient` (default `false`), `show_particles`
/// (default `true`), `show_icon` (`Optional`, which the record always answers
/// with a value, so it is always written) and a recursive
/// `hidden_effect`.
pub(super) fn persistent_mob_effect_instance_nbt(
    type_id: i32,
    resource_id: &'static str,
    effect: &MobEffectInstanceSummary,
    max_hidden_depth: usize,
) -> Result<NbtSummaryValue> {
    encode_mob_effect_instance(
        &ComponentNbtEncoder::new(type_id, resource_id, (), ContentRegistry::vanilla()),
        effect,
        effect.hidden_effect.as_deref(),
        max_hidden_depth,
    )
}

/// `RegistryCodecs.homogeneousList(Registries.MOB_EFFECT)`
/// the `HolderSetCodec` of a `"#tag"`
/// or
/// `ExtraCodecs.compactListCodec` -- the bare element for a one-holder set and a
/// list otherwise.
fn persistent_holder_set_nbt(
    type_id: i32,
    resource_id: &'static str,
    registry_ids: &[i32],
    tag: Option<&str>,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if let Some(tag) = tag {
        if !registry_ids.is_empty() {
            return Err(invalid(format!(
                "holder set carries both #{tag} and direct holders"
            )));
        }
        let tag = bbb_protocol::normalize_resource_location(tag)
            .map_err(|error| invalid(error.to_string()))?;
        return Ok(NbtSummaryValue::String(format!("#{tag}").into()));
    }
    let mut effects = Vec::with_capacity(registry_ids.len());
    for &holder_id in registry_ids {
        let named = vanilla_mob_effect_type(holder_id).ok_or_else(|| {
            invalid(format!(
                "unknown {MOB_EFFECT_REGISTRY} holder id {holder_id}"
            ))
        })?;
        effects.push(NbtSummaryValue::String(named.resource_id.into()));
    }
    if effects.len() == 1 {
        return Ok(effects.pop().expect("one-element holder set"));
    }
    Ok(NbtSummaryValue::List(effects))
}

/// The wire projection of `Consumable.CODEC`, whose
/// every absent field holds the `optionalFieldOf` default above.
pub(super) fn project_consumable(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut bbb_protocol::packets::DataComponentPatchSummary,
) -> Result<()> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("consumable is not a compound".to_string()))?;
    let consume_seconds = latest_field(fields, "consume_seconds")
        .map(|value| {
            ComponentReader::new(type_id, resource_id).float_in(
                value,
                "consume_seconds",
                0.0,
                f32::MAX,
            )
        })
        .transpose()?
        .unwrap_or(1.6);
    let animation = latest_field(fields, "animation")
        .map(|value| {
            let name = nbt_string(value)
                .ok_or_else(|| invalid("animation is not a string".to_string()))?;
            name.with_utf8(ItemUseAnimationSummary::from_serialized_name)
                .flatten()
                .ok_or_else(|| invalid(format!("unknown item-use animation {name}")))
        })
        .transpose()?
        .unwrap_or(ItemUseAnimationSummary::Eat);
    let sound = latest_field(fields, "sound")
        .map(|value| {
            project_sound_event_with_content_registry(type_id, resource_id, value, content_registry)
        })
        .transpose()?
        .unwrap_or_else(default_eat_sound);
    let has_consume_particles = latest_field(fields, "has_consume_particles")
        .map(|value| {
            codec_int_value(value)
                .map(|value| (value as i8) != 0)
                .ok_or_else(|| invalid("has_consume_particles is not numeric".to_string()))
        })
        .transpose()?
        .unwrap_or(true);
    let effects = match latest_field(fields, "on_consume_effects") {
        Some(value) => {
            let entries = list_entries(value)
                .ok_or_else(|| invalid("on_consume_effects is not a list".to_string()))?;
            if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
                return Err(invalid(format!(
                    "on_consume_effects has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                    entries.len()
                )));
            }
            entries
                .iter()
                .map(|effect| {
                    project_consume_effect(type_id, resource_id, &effect, content_registry)
                })
                .collect::<Result<Vec<_>>>()?
        }
        None => Vec::new(),
    };
    patch.consumable = Some(ConsumableSummary {
        consume_seconds,
        animation,
    });
    patch.consumable_sound = Some(sound);
    patch.consumable_has_consume_particles = Some(has_consume_particles);
    patch.consumable_effects = effects;
    Ok(())
}

fn default_eat_sound() -> SoundEventSummary {
    SoundEventSummary {
        registry_id: bbb_facts::sound_event_types::vanilla_sound_event_registry_id(
            PERSISTENT_CONSUME_SOUND_DEFAULT,
        ),
        sound_id: None,
        fixed_range_bits: None,
    }
}

pub(super) fn project_consume_effect(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<ConsumeEffectSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("consume effect is not a compound".to_string()))?;
    let type_name = nbt_string(
        latest_field(fields, "type")
            .ok_or_else(|| invalid("consume effect carries no type".to_string()))?,
    )
    .ok_or_else(|| invalid("consume effect type is not a string".to_string()))?;
    let kind = ConsumeEffectType::from_resource_id(
        &super::nbt_support::normalize_resource_id_units(type_name.units())?,
    )
    .ok_or_else(|| invalid(format!("unknown consume effect type {type_name}")))?;
    Ok(match kind {
        ConsumeEffectType::ApplyEffects => {
            let entries = list_entries(
                latest_field(fields, "effects")
                    .ok_or_else(|| invalid("apply_effects carries no effects".to_string()))?,
            )
            .ok_or_else(|| invalid("apply_effects effects are not a list".to_string()))?;
            if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
                return Err(invalid(format!(
                    "apply_effects has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                    entries.len()
                )));
            }
            let effects = entries
                .iter()
                .map(|effect| {
                    project_mob_effect_instance(
                        type_id,
                        resource_id,
                        RegistryElementValueRef::Nbt(effect.as_ref()),
                        NbtSummaryDecodeLimits::default().max_depth,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            ConsumeEffectSummary::ApplyEffects {
                effects,
                probability_bits: latest_field(fields, "probability")
                    .map(|value| {
                        ComponentReader::new(type_id, resource_id)
                            .float_in(value, "probability", 0.0, 1.0)
                            .map(f32::to_bits)
                    })
                    .transpose()?
                    .unwrap_or(1.0_f32.to_bits()),
            }
        }
        ConsumeEffectType::RemoveEffects => {
            let (effect_registry_ids, effect_tag) = project_holder_set(
                type_id,
                resource_id,
                latest_field(fields, "effects")
                    .ok_or_else(|| invalid("remove_effects carries no effects".to_string()))?,
            )?;
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids,
                effect_tag,
            }
        }
        ConsumeEffectType::ClearAllEffects => ConsumeEffectSummary::ClearAllEffects,
        ConsumeEffectType::TeleportRandomly => ConsumeEffectSummary::TeleportRandomly {
            diameter_bits: latest_field(fields, "diameter")
                .map(|value| {
                    codec_float_value(value)
                        .filter(|value| *value > 0.0 && *value <= f32::MAX)
                        .map(|value| value.to_bits())
                        .ok_or_else(|| invalid("teleport diameter is not positive".to_string()))
                })
                .transpose()?
                .unwrap_or(PERSISTENT_TELEPORT_DIAMETER_DEFAULT_BITS),
        },
        ConsumeEffectType::PlaySound => ConsumeEffectSummary::PlaySound {
            sound: project_sound_event_with_content_registry(
                type_id,
                resource_id,
                latest_field(fields, "sound")
                    .ok_or_else(|| invalid("play_sound carries no sound".to_string()))?,
                content_registry,
            )?,
        },
    })
}

fn project_holder_set(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<(Vec<i32>, Option<String>)> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    match value {
        NbtSummaryValue::String(raw) => match raw.units().strip_prefix(&[b'#' as u16]) {
            Some(tag) => Ok((
                Vec::new(),
                Some(super::nbt_support::normalize_resource_id_units(tag)?),
            )),
            None => Ok((vec![mob_effect_holder_id(type_id, resource_id, raw)?], None)),
        },
        NbtSummaryValue::List(effects) => effects
            .iter()
            .map(|effect| {
                mob_effect_holder_id(
                    type_id,
                    resource_id,
                    nbt_string(effect)
                        .ok_or_else(|| invalid("holder set element is not a string".to_string()))?,
                )
            })
            .collect::<Result<Vec<_>>>()
            .map(|effects| (effects, None)),
        _ => Err(invalid(
            "holder set is neither a tag nor a list".to_string(),
        )),
    }
}

fn mob_effect_holder_id(
    type_id: i32,
    resource_id: &'static str,
    raw: &bbb_protocol::JavaString,
) -> Result<i32> {
    let effect = super::nbt_support::normalize_resource_id_units(raw.units())?;
    vanilla_mob_effect_type_for_resource_id(&effect)
        .map(|effect| effect.id)
        .ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {MOB_EFFECT_REGISTRY} holder {effect}"),
            )
        })
}

pub(super) fn project_mob_effect_instance(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
    max_hidden_depth: usize,
) -> Result<MobEffectInstanceSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if !value.is_compound() {
        return Err(invalid("mob effect instance is not a compound".to_string()));
    }
    let effect_id = mob_effect_holder_id(
        type_id,
        resource_id,
        value
            .field("id")
            .ok_or_else(|| invalid("mob effect instance carries no id".to_string()))?
            .java_string_ref()
            .ok_or_else(|| invalid("mob effect instance id is not a string".to_string()))?,
    )?;
    let details = project_details(type_id, resource_id, value, max_hidden_depth)?;
    Ok(MobEffectInstanceSummary {
        effect_id,
        amplifier: details.amplifier,
        duration: details.duration,
        ambient: details.ambient,
        show_particles: details.show_particles,
        show_icon: details.show_icon,
        hidden_effect: details.hidden_effect,
    })
}
