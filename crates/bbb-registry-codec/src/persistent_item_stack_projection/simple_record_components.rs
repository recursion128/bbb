//! Persistent projections for `minecraft:attack_range`,
//! `minecraft:swing_animation`, `minecraft:use_effects`,
//! `minecraft:death_protection`, `minecraft:damage_type`,
//! `minecraft:instrument`, and `minecraft:jukebox_playable`.

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
};
use bbb_facts::item_component_value_types::SwingAnimationTypeSummary;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry as field;
use bbb_protocol::packets::{DataComponentPatchSummary, UseEffectsSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::consumable_components::{persistent_consume_effect_nbt, project_consume_effect};
use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{
    persistent_inline_instrument_nbt, project_inline_instrument, resolve_holder_id,
    DAMAGE_TYPE_REGISTRY,
};
use super::nbt_support::{
    checked_float, compound_fields, latest_field, nbt_string, normalize_resource_id_units,
    persistent_flag_nbt, persistent_float_nbt, ComponentReader,
};

/// The seven components this module owns, so the two dispatch tables agree on
/// one list.
pub(super) fn is_simple_record_component(type_id: i32) -> bool {
    matches!(
        type_id,
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID
            | VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID
            | VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID
            | VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID
            | VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID
            | VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
            | VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID
    )
}

pub(super) const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";
pub(super) const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";

/// `SwingAnimation.DEFAULT` (`SwingAnimation.java:12`).
const SWING_ANIMATION_DEFAULT_TYPE: SwingAnimationTypeSummary = SwingAnimationTypeSummary::Whack;
const SWING_ANIMATION_DEFAULT_DURATION: i32 = 6;
/// `UseEffects.DEFAULT` (`UseEffects.java:10`).
const USE_EFFECTS_DEFAULT: UseEffectsSummary = UseEffectsSummary {
    can_sprint: false,
    interact_vibrations: true,
    speed_multiplier: 0.2,
};
/// `AttackRange.CODEC`'s six `optionalFieldOf` defaults
/// (`AttackRange.java:30-35`), each paired with its `floatRange` bound.
const ATTACK_RANGE_FIELDS: &[(&str, f32, f32, f32)] = &[
    ("min_reach", 0.0, 0.0, 64.0),
    ("max_reach", 3.0, 0.0, 64.0),
    ("min_creative_reach", 0.0, 0.0, 64.0),
    ("max_creative_reach", 5.0, 0.0, 64.0),
    ("hitbox_margin", 0.3, 0.0, 1.0),
    ("mob_factor", 1.0, 0.0, 2.0),
];

fn attack_range_values(range: &bbb_protocol::packets::AttackRangeSummary) -> [f32; 6] {
    [
        range.min_reach,
        range.max_reach,
        range.min_creative_reach,
        range.max_creative_reach,
        range.hitbox_margin,
        range.mob_factor,
    ]
}

pub(super) fn persistent_simple_record_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let missing = || ItemStackPersistentProjectionError::UnsupportedComponent {
        type_id,
        resource_id,
    };
    Ok(match type_id {
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => {
            let range = patch.attack_range.as_ref().ok_or_else(missing)?;
            let values = attack_range_values(range);
            let mut fields = Vec::new();
            for ((name, default, min, max), value) in ATTACK_RANGE_FIELDS.iter().zip(values) {
                let value = checked_float(type_id, resource_id, value, *min, *max, name)?;
                if value.to_bits() != default.to_bits() {
                    fields.push(field(*name, persistent_float_nbt(value.to_bits())));
                }
            }
            NbtSummaryValue::Compound(fields)
        }
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => {
            let animation = patch.swing_animation.as_ref().ok_or_else(missing)?;
            if SwingAnimationTypeSummary::from_id(animation.animation_type.id())
                != animation.animation_type
            {
                return Err(invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!(
                        "unknown swing animation type id {}",
                        animation.animation_type.id()
                    ),
                ));
            }
            let mut fields = Vec::new();
            if animation.animation_type != SWING_ANIMATION_DEFAULT_TYPE {
                fields.push(field(
                    "type",
                    NbtSummaryValue::String(animation.animation_type.as_str().into()),
                ));
            }
            if animation.duration != SWING_ANIMATION_DEFAULT_DURATION {
                // `ExtraCodecs.POSITIVE_INT`.
                if animation.duration <= 0 {
                    return Err(invalid_persistent_component(
                        type_id,
                        resource_id,
                        format!("value must be positive: {}", animation.duration),
                    ));
                }
                fields.push(field("duration", NbtSummaryValue::Int(animation.duration)));
            }
            NbtSummaryValue::Compound(fields)
        }
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => {
            let effects = patch.use_effects.as_ref().ok_or_else(missing)?;
            let speed = checked_float(
                type_id,
                resource_id,
                effects.speed_multiplier,
                0.0,
                1.0,
                "speed_multiplier",
            )?;
            let mut fields = Vec::new();
            if effects.can_sprint != USE_EFFECTS_DEFAULT.can_sprint {
                fields.push(field("can_sprint", persistent_flag_nbt(effects.can_sprint)));
            }
            if effects.interact_vibrations != USE_EFFECTS_DEFAULT.interact_vibrations {
                fields.push(field(
                    "interact_vibrations",
                    persistent_flag_nbt(effects.interact_vibrations),
                ));
            }
            if speed.to_bits() != USE_EFFECTS_DEFAULT.speed_multiplier.to_bits() {
                fields.push(field(
                    "speed_multiplier",
                    persistent_float_nbt(speed.to_bits()),
                ));
            }
            NbtSummaryValue::Compound(fields)
        }
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
            let mut fields = Vec::new();
            if !patch.death_protection_effects.is_empty() {
                fields.push(field(
                    "death_effects",
                    NbtSummaryValue::List(
                        patch
                            .death_protection_effects
                            .iter()
                            .map(|effect| {
                                persistent_consume_effect_nbt(
                                    type_id,
                                    resource_id,
                                    effect,
                                    content_registry,
                                )
                            })
                            .collect::<Result<Vec<_>>>()?,
                    ),
                ));
            }
            NbtSummaryValue::Compound(fields)
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => {
            let holder_id = patch.damage_type_id.ok_or_else(missing)?;
            NbtSummaryValue::String(
                named_holder(
                    type_id,
                    resource_id,
                    DAMAGE_TYPE_REGISTRY,
                    holder_id,
                    holder_resource_id,
                )?
                .into(),
            )
        }
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => {
            if let Some(instrument) = patch.instrument.direct.as_deref() {
                if patch.instrument.registry_id.is_some() {
                    return Err(invalid_persistent_component(
                        type_id,
                        resource_id,
                        "instrument has named and inline holders".to_string(),
                    ));
                }
                return persistent_inline_instrument_nbt(
                    type_id,
                    resource_id,
                    instrument,
                    content_registry,
                );
            }
            let holder_id = patch.instrument.registry_id.ok_or_else(missing)?;
            NbtSummaryValue::String(
                named_holder(
                    type_id,
                    resource_id,
                    INSTRUMENT_REGISTRY,
                    holder_id,
                    holder_resource_id,
                )?
                .into(),
            )
        }
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => {
            if patch.jukebox_direct_song.is_some() {
                return Err(invalid_persistent_component(
                    type_id,
                    resource_id,
                    "an inline jukebox song has no writer-bound identifier".to_string(),
                ));
            }
            let holder_id = patch.jukebox_song_id.ok_or_else(missing)?;
            NbtSummaryValue::String(
                named_holder(
                    type_id,
                    resource_id,
                    JUKEBOX_SONG_REGISTRY,
                    holder_id,
                    holder_resource_id,
                )?
                .into(),
            )
        }
        _ => unreachable!("simple record dispatch is guarded by the caller"),
    })
}

fn named_holder(
    type_id: i32,
    resource_id: &'static str,
    registry_id: &'static str,
    holder_id: i32,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<String> {
    let name = holder_resource_id(registry_id, holder_id).ok_or(
        ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
            registry_id,
            holder_id,
        },
    )?;
    bbb_protocol::normalize_resource_location(name)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))
}

/// One `optionalFieldOf` float read back, with its bound re-checked because the
/// save form may hold a value the codec's `validate` would reject.
fn project_bounded_float(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
    name: &str,
    default: f32,
    min: f32,
    max: f32,
) -> Result<f32> {
    let Some(value) = latest_field(fields, name) else {
        return Ok(default);
    };
    let value = bbb_protocol::nbt_summary::codec_float_value(value).ok_or_else(|| {
        invalid_persistent_component(type_id, resource_id, format!("{name} is not a float"))
    })?;
    checked_float(type_id, resource_id, value, min, max, name)
}

pub(super) fn project_simple_record(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    match type_id {
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID if nbt_string(value).is_none() => {
            let instrument = project_inline_instrument(
                super::nbt_support::ComponentReader::new(type_id, resource_id),
                value,
                content_registry,
            )?;
            patch.instrument.direct = Some(Box::new(instrument));
            Ok(())
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
        | VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => {
            let registry_id = match type_id {
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => DAMAGE_TYPE_REGISTRY,
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => INSTRUMENT_REGISTRY,
                _ => JUKEBOX_SONG_REGISTRY,
            };
            let named = nbt_string(value).ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    "holder is not a resource id".to_string(),
                )
            })?;
            let holder = resolve_holder_id(
                registry_id,
                &normalize_resource_id_units(named.units())?,
                holder_id,
            )?;
            match type_id {
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => patch.damage_type_id = Some(holder),
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => {
                    patch.instrument.registry_id = Some(holder)
                }
                _ => patch.jukebox_song_id = Some(holder),
            }
            Ok(())
        }
        _ => {
            let fields = compound_fields(value).ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    "record is not a compound".to_string(),
                )
            })?;
            match type_id {
                VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => {
                    let mut values = [0.0; 6];
                    for (value, (name, default, min, max)) in
                        values.iter_mut().zip(ATTACK_RANGE_FIELDS)
                    {
                        *value = project_bounded_float(
                            type_id,
                            resource_id,
                            fields,
                            name,
                            *default,
                            *min,
                            *max,
                        )?;
                    }
                    patch.attack_range = Some(bbb_protocol::packets::AttackRangeSummary {
                        min_reach: values[0],
                        max_reach: values[1],
                        min_creative_reach: values[2],
                        max_creative_reach: values[3],
                        hitbox_margin: values[4],
                        mob_factor: values[5],
                    });
                }
                VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => {
                    let animation_type = match latest_field(fields, "type") {
                        Some(value) => {
                            let name = nbt_string(value).ok_or_else(|| {
                                invalid_persistent_component(
                                    type_id,
                                    resource_id,
                                    "swing animation type is not a name".to_string(),
                                )
                            })?;
                            name.with_utf8(SwingAnimationTypeSummary::from_serialized_name)
                                .flatten()
                                .ok_or_else(|| {
                                    invalid_persistent_component(
                                        type_id,
                                        resource_id,
                                        format!("unknown enum value {name}"),
                                    )
                                })?
                        }
                        None => SWING_ANIMATION_DEFAULT_TYPE,
                    };
                    let duration = match latest_field(fields, "duration") {
                        Some(value) => {
                            let duration = bbb_protocol::nbt_summary::codec_int_value(value)
                                .ok_or_else(|| {
                                    invalid_persistent_component(
                                        type_id,
                                        resource_id,
                                        "duration is not an int".to_string(),
                                    )
                                })?;
                            if duration <= 0 {
                                return Err(invalid_persistent_component(
                                    type_id,
                                    resource_id,
                                    format!("value must be positive: {duration}"),
                                ));
                            }
                            duration
                        }
                        None => SWING_ANIMATION_DEFAULT_DURATION,
                    };
                    patch.swing_animation = Some(bbb_protocol::packets::SwingAnimationSummary {
                        animation_type,
                        duration,
                    });
                }
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => {
                    patch.use_effects = Some(UseEffectsSummary {
                        can_sprint: ComponentReader::new(type_id, resource_id).optional_flag(
                            fields,
                            "can_sprint",
                            USE_EFFECTS_DEFAULT.can_sprint,
                        )?,
                        interact_vibrations: ComponentReader::new(type_id, resource_id)
                            .optional_flag(
                                fields,
                                "interact_vibrations",
                                USE_EFFECTS_DEFAULT.interact_vibrations,
                            )?,
                        speed_multiplier: project_bounded_float(
                            type_id,
                            resource_id,
                            fields,
                            "speed_multiplier",
                            USE_EFFECTS_DEFAULT.speed_multiplier,
                            0.0,
                            1.0,
                        )?,
                    });
                }
                VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
                    let mut effects = Vec::new();
                    if let Some(list) = latest_field(fields, "death_effects") {
                        let entries = super::nbt_support::list_entries(list).ok_or_else(|| {
                            invalid_persistent_component(
                                type_id,
                                resource_id,
                                "death_effects is not a list".to_string(),
                            )
                        })?;
                        if entries.len() > super::nbt_support::MAX_PERSISTENT_LIST_ITEMS {
                            return Err(invalid_persistent_component(
                                type_id,
                                resource_id,
                                format!(
                                    "death_effects has {} entries, maximum is {}",
                                    entries.len(),
                                    super::nbt_support::MAX_PERSISTENT_LIST_ITEMS
                                ),
                            ));
                        }
                        for entry in entries.iter() {
                            effects.push(project_consume_effect(
                                type_id,
                                resource_id,
                                &entry,
                                content_registry,
                            )?);
                        }
                    }
                    patch.death_protection_effects = effects;
                }
                _ => unreachable!("simple record dispatch is guarded by the caller"),
            }
            Ok(())
        }
    }
}
