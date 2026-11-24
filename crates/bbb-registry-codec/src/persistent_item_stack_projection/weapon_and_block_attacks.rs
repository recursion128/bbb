//! `minecraft:kinetic_weapon` and `minecraft:blocks_attacks`, the two
//! components whose persistent codecs nest records of their own.
//!
//! * `KineticWeapon.CODEC` is two
//!   `NON_NEGATIVE_INT`s defaulting to `10` and `0`, three optional
//!   `Condition`s, `Codec.FLOAT` defaulting to `0.0` and `1.0`, and
//!   two optional sound holders; `Condition.CODEC` is a required
//!   `NON_NEGATIVE_INT` beside two optional floats;
//! * `BlocksAttacks.CODEC` is two
//!   `NON_NEGATIVE_FLOAT`s defaulting to `0.0` and `1.0`, a
//!   `DamageReduction` list defaulting to one `90.0`/`empty`/`0.0`/`1.0` entry,
//!   an `ItemDamageFunction` defaulting to
//!   `1.0`/`0.0`/`1.0`, an optional `bypassed_by`
//!   damage-type holder set and two optional sound holders.
//!
//! Every `optionalFieldOf` here drops its field while the value equals the
//! Java default (`Objects.equals`, so `Float.equals` on the bit pattern), which
//! is the same rule the shipped encoded form follows.

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::{codec_float_value, codec_int_value};
use bbb_protocol::packets::{
    BlocksAttacksSummary, DamageReductionSummary, DataComponentPatchSummary, HolderSetSummary,
    ItemDamageFunctionSummary, KineticWeaponConditionSummary, KineticWeaponSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::encoding::{ComponentEncoder, ComponentNbtEncoder, ComponentScalar as Scalar};
use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{
    persistent_damage_type_holder_set_nbt, project_damage_type_holder_set,
    project_sound_event_with_content_registry,
};
use super::nbt_support::{
    checked_float, compound_fields, latest_field, list_entries, persistent_float_nbt,
    MAX_PERSISTENT_LIST_ITEMS,
};

/// `KineticWeapon.CODEC`'s `optionalFieldOf` defaults
/// (`KineticWeapon.java:44-45`) and the two floats' defaults (`:49-50`).
const CONTACT_COOLDOWN_TICKS: i32 = 10;
const DELAY_TICKS: i32 = 0;
const FORWARD_MOVEMENT: f32 = 0.0;
const DAMAGE_MULTIPLIER: f32 = 1.0;
/// `KineticWeapon.Condition.CODEC`'s two float defaults
/// (`KineticWeapon.java:152-153`).
const MIN_SPEED: f32 = 0.0;
const MIN_RELATIVE_SPEED: f32 = 0.0;
/// `BlocksAttacks.CODEC`'s two float defaults (`BlocksAttacks.java:39-40`).
const BLOCK_DELAY_SECONDS: f32 = 0.0;
const DISABLE_COOLDOWN_SCALE: f32 = 1.0;
/// `DamageReduction.CODEC`'s one float default (`BlocksAttacks.java:140`).
const HORIZONTAL_BLOCKING_ANGLE: f32 = 90.0;
/// `BlocksAttacks.ItemDamageFunction.DEFAULT` (`BlocksAttacks.java:188`).
const ITEM_DAMAGE_DEFAULT: (f32, f32, f32) = (1.0, 0.0, 1.0);
/// `DamageReduction.CODEC`'s `damage_reductions` default list
/// (`BlocksAttacks.java:43`): one reduction that blocks everything head-on.
const DAMAGE_REDUCTIONS_DEFAULT: (f32, f32, f32) = (90.0, 0.0, 1.0);

/// The three optional condition fields in codec order
fn kinetic_conditions(
    weapon: &KineticWeaponSummary,
) -> [(&'static str, &Option<KineticWeaponConditionSummary>); 3] {
    [
        ("dismount_conditions", &weapon.dismount_conditions),
        ("knockback_conditions", &weapon.knockback_conditions),
        ("damage_conditions", &weapon.damage_conditions),
    ]
}

/// `ExtraCodecs.NON_NEGATIVE_INT` is `intRange(0, Integer.MAX_VALUE)`
/// whose `validate` also runs while encoding.
fn checked_non_negative_int(
    type_id: i32,
    resource_id: &'static str,
    value: i32,
    what: &str,
) -> Result<i32> {
    if value < 0 {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("{what} {value} is negative"),
        ));
    }
    Ok(value)
}

/// `ExtraCodecs.NON_NEGATIVE_FLOAT` is `floatRange(0.0F, Float.MAX_VALUE)`
/// NaN fails every comparison, as Java's
/// `Comparable`-based bound does.
fn checked_non_negative_float(
    type_id: i32,
    resource_id: &'static str,
    value: f32,
    what: &str,
) -> Result<f32> {
    checked_float(type_id, resource_id, value, 0.0, f32::MAX, what)
}

/// `ExtraCodecs.POSITIVE_FLOAT` (`BlocksAttacks.java:140`).
fn checked_positive_float(
    type_id: i32,
    resource_id: &'static str,
    value: f32,
    what: &str,
) -> Result<f32> {
    if !(value > 0.0 && value <= f32::MAX) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("{what} {value} is not positive"),
        ));
    }
    Ok(value)
}

/// Whether a reduction is the one `damage_reductions` defaults to
/// which decides whether the whole field is
/// dropped.
fn is_default_damage_reduction(reduction: &DamageReductionSummary) -> bool {
    reduction.horizontal_blocking_angle_bits == DAMAGE_REDUCTIONS_DEFAULT.0.to_bits()
        && reduction.damage_types.is_none()
        && reduction.base_bits == DAMAGE_REDUCTIONS_DEFAULT.1.to_bits()
        && reduction.factor_bits == DAMAGE_REDUCTIONS_DEFAULT.2.to_bits()
}

fn is_default_item_damage(item_damage: &ItemDamageFunctionSummary) -> bool {
    item_damage.threshold_bits == ITEM_DAMAGE_DEFAULT.0.to_bits()
        && item_damage.base_bits == ITEM_DAMAGE_DEFAULT.1.to_bits()
        && item_damage.factor_bits == ITEM_DAMAGE_DEFAULT.2.to_bits()
}

/// Encoding operations used by the persistent NBT and container-click hash backends.
pub trait WeaponComponentEncoder: ComponentEncoder {
    fn non_negative_float(
        &self,
        name: &str,
        bits: u32,
    ) -> std::result::Result<Self::Value, Self::Error>;
    fn positive_float(
        &self,
        name: &str,
        bits: u32,
    ) -> std::result::Result<Self::Value, Self::Error>;
    fn non_negative_int(
        &self,
        name: &str,
        value: i32,
    ) -> std::result::Result<Self::Value, Self::Error>;
    fn reductions(
        &self,
        values: &[DamageReductionSummary],
    ) -> std::result::Result<Self::Value, Self::Error>;
    fn damage_types(
        &self,
        types: &HolderSetSummary,
    ) -> std::result::Result<Self::Value, Self::Error>;
}

pub fn encode_blocks_attacks<E: WeaponComponentEncoder>(
    encoder: &E,
    value: &BlocksAttacksSummary,
) -> std::result::Result<E::Value, E::Error> {
    let mut fields = encoder.fields(0);
    for (name, bits, default) in [
        (
            "block_delay_seconds",
            value.block_delay_seconds_bits,
            BLOCK_DELAY_SECONDS,
        ),
        (
            "disable_cooldown_scale",
            value.disable_cooldown_scale_bits,
            DISABLE_COOLDOWN_SCALE,
        ),
    ] {
        if bits != default.to_bits() {
            encoder.field(&mut fields, name, encoder.non_negative_float(name, bits)?);
        }
    }
    if !matches!(value.damage_reductions.as_slice(), [reduction] if is_default_damage_reduction(reduction))
    {
        encoder.field(
            &mut fields,
            "damage_reductions",
            encoder.reductions(&value.damage_reductions)?,
        );
    }
    if !is_default_item_damage(&value.item_damage) {
        let item = &value.item_damage;
        let mut damage = encoder.fields(3);
        for (name, bits) in [
            ("threshold", item.threshold_bits),
            ("base", item.base_bits),
            ("factor", item.factor_bits),
        ] {
            encoder.scalar(&mut damage, name, Scalar::Float(bits));
        }
        encoder.field(&mut fields, "item_damage", encoder.record(damage));
    }
    if let Some(types) = &value.bypassed_by {
        encoder.field(&mut fields, "bypassed_by", encoder.damage_types(types)?);
    }
    for (name, sound) in [
        ("block_sound", &value.block_sound),
        ("disabled_sound", &value.disable_sound),
    ] {
        if let Some(sound) = sound {
            encoder.field(&mut fields, name, encoder.sound(sound)?);
        }
    }
    Ok(encoder.record(fields))
}

#[inline(never)]
pub fn encode_damage_reduction<E: WeaponComponentEncoder>(
    encoder: &E,
    value: &DamageReductionSummary,
) -> std::result::Result<E::Value, E::Error> {
    let mut fields = encoder.fields(0);
    if value.horizontal_blocking_angle_bits != HORIZONTAL_BLOCKING_ANGLE.to_bits() {
        encoder.field(
            &mut fields,
            "horizontal_blocking_angle",
            encoder.positive_float(
                "horizontal_blocking_angle",
                value.horizontal_blocking_angle_bits,
            )?,
        );
    }
    if let Some(types) = &value.damage_types {
        encoder.field(&mut fields, "type", encoder.damage_types(types)?);
    }
    encoder.scalar(&mut fields, "base", Scalar::Float(value.base_bits));
    encoder.scalar(&mut fields, "factor", Scalar::Float(value.factor_bits));
    Ok(encoder.record(fields))
}

#[inline(never)]
fn encode_kinetic_condition<E: WeaponComponentEncoder>(
    encoder: &E,
    condition: &KineticWeaponConditionSummary,
) -> std::result::Result<E::Value, E::Error> {
    let speeds = [
        ("min_speed", condition.min_speed_bits, MIN_SPEED),
        (
            "min_relative_speed",
            condition.min_relative_speed_bits,
            MIN_RELATIVE_SPEED,
        ),
    ]
    .into_iter()
    .filter(|(_, bits, default)| *bits != default.to_bits());
    let mut fields = encoder.fields(1 + speeds.clone().count());
    encoder.field(
        &mut fields,
        "max_duration_ticks",
        encoder.non_negative_int("max_duration_ticks", condition.max_duration_ticks)?,
    );
    for (name, bits, _) in speeds {
        encoder.scalar(&mut fields, name, Scalar::Float(bits));
    }
    Ok(encoder.record(fields))
}

pub fn encode_kinetic_weapon<E: WeaponComponentEncoder>(
    encoder: &E,
    value: &KineticWeaponSummary,
) -> std::result::Result<E::Value, E::Error> {
    let mut fields = encoder.fields(0);
    for (name, number, default) in [
        (
            "contact_cooldown_ticks",
            value.contact_cooldown_ticks,
            CONTACT_COOLDOWN_TICKS,
        ),
        ("delay_ticks", value.delay_ticks, DELAY_TICKS),
    ] {
        if number != default {
            encoder.field(&mut fields, name, encoder.non_negative_int(name, number)?);
        }
    }
    for (name, condition) in kinetic_conditions(value) {
        if let Some(condition) = condition {
            encoder.field(
                &mut fields,
                name,
                encode_kinetic_condition(encoder, condition)?,
            );
        }
    }
    for (name, bits, default) in [
        (
            "forward_movement",
            value.forward_movement_bits,
            FORWARD_MOVEMENT,
        ),
        (
            "damage_multiplier",
            value.damage_multiplier_bits,
            DAMAGE_MULTIPLIER,
        ),
    ] {
        if bits != default.to_bits() {
            encoder.scalar(&mut fields, name, Scalar::Float(bits));
        }
    }
    for (name, sound) in [("sound", &value.sound), ("hit_sound", &value.hit_sound)] {
        if let Some(sound) = sound {
            encoder.field(&mut fields, name, encoder.sound(sound)?);
        }
    }
    Ok(encoder.record(fields))
}

impl WeaponComponentEncoder for ComponentNbtEncoder<'_, &dyn Fn(&str, i32) -> Option<String>> {
    fn non_negative_float(&self, name: &str, bits: u32) -> Result<Self::Value> {
        Ok(persistent_float_nbt(
            checked_non_negative_float(self.type_id, self.resource_id, f32::from_bits(bits), name)?
                .to_bits(),
        ))
    }

    fn positive_float(&self, name: &str, bits: u32) -> Result<Self::Value> {
        Ok(persistent_float_nbt(
            checked_positive_float(self.type_id, self.resource_id, f32::from_bits(bits), name)?
                .to_bits(),
        ))
    }

    fn non_negative_int(&self, name: &str, value: i32) -> Result<Self::Value> {
        Ok(NbtSummaryValue::Int(checked_non_negative_int(
            self.type_id,
            self.resource_id,
            value,
            name,
        )?))
    }

    fn reductions(&self, values: &[DamageReductionSummary]) -> Result<Self::Value> {
        let mut reductions = Vec::with_capacity(values.len());
        for value in values {
            reductions.push(encode_damage_reduction(self, value)?);
        }
        Ok(NbtSummaryValue::List(reductions))
    }

    fn damage_types(&self, types: &HolderSetSummary) -> Result<Self::Value> {
        persistent_damage_type_holder_set_nbt(
            self.type_id,
            self.resource_id,
            &types.registry_ids,
            types.tag.as_deref(),
            self.holder_resource_id,
        )
    }
}

/// `KineticWeapon.CODEC` / `BlocksAttacks.CODEC` in the encode direction
pub(super) fn persistent_weapon_and_block_attacks_nbt(
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
    let encoder =
        ComponentNbtEncoder::new(type_id, resource_id, holder_resource_id, content_registry);
    match type_id {
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => {
            encode_kinetic_weapon(&encoder, patch.kinetic_weapon.as_ref().ok_or_else(missing)?)
        }
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            encode_blocks_attacks(&encoder, patch.blocks_attacks.as_ref().ok_or_else(missing)?)
        }
        _ => unreachable!("weapon dispatch is guarded by the caller"),
    }
}

fn project_int(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
    name: &str,
    default: i32,
) -> Result<i32> {
    let Some(value) = latest_field(fields, name) else {
        return Ok(default);
    };
    let value = codec_int_value(value).ok_or_else(|| {
        invalid_persistent_component(type_id, resource_id, format!("{name} is not an integer"))
    })?;
    checked_non_negative_int(type_id, resource_id, value, name)
}

fn project_float_bits(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
    name: &str,
    default: f32,
) -> Result<u32> {
    let Some(value) = latest_field(fields, name) else {
        return Ok(default.to_bits());
    };
    codec_float_value(value).map(f32::to_bits).ok_or_else(|| {
        invalid_persistent_component(type_id, resource_id, format!("{name} is not a float"))
    })
}

fn project_kinetic_condition(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<KineticWeaponConditionSummary> {
    let fields = compound_fields(value).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "kinetic weapon condition is not a compound".to_string(),
        )
    })?;
    let Some(max_duration_ticks) = latest_field(fields, "max_duration_ticks") else {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            "kinetic weapon condition carries no max_duration_ticks".to_string(),
        ));
    };
    let max_duration_ticks = codec_int_value(max_duration_ticks).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "max_duration_ticks is not an integer".to_string(),
        )
    })?;
    Ok(KineticWeaponConditionSummary {
        max_duration_ticks: checked_non_negative_int(
            type_id,
            resource_id,
            max_duration_ticks,
            "max_duration_ticks",
        )?,
        min_speed_bits: project_float_bits(type_id, resource_id, fields, "min_speed", MIN_SPEED)?,
        min_relative_speed_bits: project_float_bits(
            type_id,
            resource_id,
            fields,
            "min_relative_speed",
            MIN_RELATIVE_SPEED,
        )?,
    })
}

fn project_optional_kinetic_condition(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
    name: &str,
) -> Result<Option<KineticWeaponConditionSummary>> {
    latest_field(fields, name)
        .map(|value| project_kinetic_condition(type_id, resource_id, value))
        .transpose()
}

fn project_damage_reduction(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<DamageReductionSummary> {
    let fields = compound_fields(value).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "damage reduction is not a compound".to_string(),
        )
    })?;
    let angle = f32::from_bits(project_float_bits(
        type_id,
        resource_id,
        fields,
        "horizontal_blocking_angle",
        HORIZONTAL_BLOCKING_ANGLE,
    )?);
    let angle = checked_positive_float(type_id, resource_id, angle, "horizontal_blocking_angle")?;
    let damage_types = latest_field(fields, "type")
        .map(|value| project_damage_type_holder_set(type_id, resource_id, value, holder_id))
        .transpose()?;
    for (name, what) in [("base", "base"), ("factor", "factor")] {
        if latest_field(fields, name).is_none() {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("damage reduction carries no {what}"),
            ));
        }
    }
    Ok(DamageReductionSummary {
        horizontal_blocking_angle_bits: angle.to_bits(),
        damage_types,
        base_bits: project_float_bits(type_id, resource_id, fields, "base", 0.0)?,
        factor_bits: project_float_bits(type_id, resource_id, fields, "factor", 0.0)?,
    })
}

fn project_blocks_attacks(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
) -> Result<BlocksAttacksSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("blocks_attacks is not a compound".to_string()))?;
    let block_delay_seconds = f32::from_bits(project_float_bits(
        type_id,
        resource_id,
        fields,
        "block_delay_seconds",
        BLOCK_DELAY_SECONDS,
    )?);
    let block_delay_seconds = checked_non_negative_float(
        type_id,
        resource_id,
        block_delay_seconds,
        "block_delay_seconds",
    )?;
    let disable_cooldown_scale = f32::from_bits(project_float_bits(
        type_id,
        resource_id,
        fields,
        "disable_cooldown_scale",
        DISABLE_COOLDOWN_SCALE,
    )?);
    let disable_cooldown_scale = checked_non_negative_float(
        type_id,
        resource_id,
        disable_cooldown_scale,
        "disable_cooldown_scale",
    )?;
    let damage_reductions = match latest_field(fields, "damage_reductions") {
        Some(value) => {
            let entries = list_entries(value)
                .ok_or_else(|| invalid("damage_reductions is not a list".to_string()))?;
            if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
                return Err(invalid(format!(
                    "damage_reductions has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                    entries.len()
                )));
            }
            entries
                .iter()
                .map(|entry| project_damage_reduction(type_id, resource_id, &entry, holder_id))
                .collect::<Result<Vec<_>>>()?
        }
        None => vec![DamageReductionSummary {
            horizontal_blocking_angle_bits: DAMAGE_REDUCTIONS_DEFAULT.0.to_bits(),
            damage_types: None,
            base_bits: DAMAGE_REDUCTIONS_DEFAULT.1.to_bits(),
            factor_bits: DAMAGE_REDUCTIONS_DEFAULT.2.to_bits(),
        }],
    };
    let item_damage = match latest_field(fields, "item_damage") {
        Some(value) => {
            let item_fields = compound_fields(value)
                .ok_or_else(|| invalid("item_damage is not a compound".to_string()))?;
            for name in ["threshold", "base", "factor"] {
                if latest_field(item_fields, name).is_none() {
                    return Err(invalid(format!("item_damage carries no {name}")));
                }
            }
            let threshold_bits =
                project_float_bits(type_id, resource_id, item_fields, "threshold", 0.0)?;
            checked_non_negative_float(
                type_id,
                resource_id,
                f32::from_bits(threshold_bits),
                "threshold",
            )?;
            ItemDamageFunctionSummary {
                threshold_bits,
                base_bits: project_float_bits(type_id, resource_id, item_fields, "base", 0.0)?,
                factor_bits: project_float_bits(type_id, resource_id, item_fields, "factor", 0.0)?,
            }
        }
        None => ItemDamageFunctionSummary {
            threshold_bits: ITEM_DAMAGE_DEFAULT.0.to_bits(),
            base_bits: ITEM_DAMAGE_DEFAULT.1.to_bits(),
            factor_bits: ITEM_DAMAGE_DEFAULT.2.to_bits(),
        },
    };
    Ok(BlocksAttacksSummary {
        block_delay_seconds_bits: block_delay_seconds.to_bits(),
        disable_cooldown_scale_bits: disable_cooldown_scale.to_bits(),
        damage_reductions,
        item_damage,
        bypassed_by: latest_field(fields, "bypassed_by")
            .map(|value| project_damage_type_holder_set(type_id, resource_id, value, holder_id))
            .transpose()?,
        block_sound: latest_field(fields, "block_sound")
            .map(|value| {
                project_sound_event_with_content_registry(
                    type_id,
                    resource_id,
                    value,
                    content_registry,
                )
            })
            .transpose()?,
        disable_sound: latest_field(fields, "disabled_sound")
            .map(|value| {
                project_sound_event_with_content_registry(
                    type_id,
                    resource_id,
                    value,
                    content_registry,
                )
            })
            .transpose()?,
    })
}

/// `KineticWeapon.CODEC` / `BlocksAttacks.CODEC` in the decode direction.
pub(super) fn project_weapon_and_block_attacks(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    match type_id {
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => {
            let invalid = |detail: String| {
                invalid_persistent_component(type_id, resource_id, detail.to_string())
            };
            let fields = compound_fields(value)
                .ok_or_else(|| invalid("kinetic_weapon is not a compound".to_string()))?;
            patch.kinetic_weapon = Some(KineticWeaponSummary {
                contact_cooldown_ticks: project_int(
                    type_id,
                    resource_id,
                    fields,
                    "contact_cooldown_ticks",
                    CONTACT_COOLDOWN_TICKS,
                )?,
                delay_ticks: project_int(type_id, resource_id, fields, "delay_ticks", DELAY_TICKS)?,
                dismount_conditions: project_optional_kinetic_condition(
                    type_id,
                    resource_id,
                    fields,
                    "dismount_conditions",
                )?,
                knockback_conditions: project_optional_kinetic_condition(
                    type_id,
                    resource_id,
                    fields,
                    "knockback_conditions",
                )?,
                damage_conditions: project_optional_kinetic_condition(
                    type_id,
                    resource_id,
                    fields,
                    "damage_conditions",
                )?,
                forward_movement_bits: project_float_bits(
                    type_id,
                    resource_id,
                    fields,
                    "forward_movement",
                    FORWARD_MOVEMENT,
                )?,
                damage_multiplier_bits: project_float_bits(
                    type_id,
                    resource_id,
                    fields,
                    "damage_multiplier",
                    DAMAGE_MULTIPLIER,
                )?,
                sound: latest_field(fields, "sound")
                    .map(|value| {
                        project_sound_event_with_content_registry(
                            type_id,
                            resource_id,
                            value,
                            content_registry,
                        )
                    })
                    .transpose()?,
                hit_sound: latest_field(fields, "hit_sound")
                    .map(|value| {
                        project_sound_event_with_content_registry(
                            type_id,
                            resource_id,
                            value,
                            content_registry,
                        )
                    })
                    .transpose()?,
            });
        }
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            patch.blocks_attacks = Some(project_blocks_attacks(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
            )?);
        }
        _ => unreachable!("weapon dispatch is guarded by the caller"),
    }
    Ok(())
}
