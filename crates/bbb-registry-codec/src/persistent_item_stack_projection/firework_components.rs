//! Firework component records shared by saved stacks and resource predicates.

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
};
use bbb_facts::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::{data_components::MAX_FIREWORK_EXPLOSIONS, FireworkExplosionSummary};
use bbb_protocol::NbtSummaryValue;

use crate::registry_codec::RegistryElementValueRef;

use super::error::{invalid_persistent_component, Result};
use super::nbt_support::ComponentReader;

fn int_list(
    c: ComponentReader,
    value: RegistryElementValueRef<'_>,
    name: &str,
) -> Result<Vec<i32>> {
    value
        .field(name)
        .map(|values| {
            let values = values
                .list_elements()
                .ok_or_else(|| c.error(format!("{name} is not a list")))?;
            values.map(|value| c.int(value, name)).collect()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

pub fn read_explosion(value: RegistryElementValueRef<'_>) -> Result<FireworkExplosionSummary> {
    let c = ComponentReader::new(
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        "minecraft:firework_explosion",
    );
    if !value.is_compound() {
        return Err(c.error("firework explosion is not a compound"));
    }
    let shape = value
        .field("shape")
        .and_then(RegistryElementValueRef::java_string_ref)
        .and_then(|value| {
            value
                .with_utf8(FireworkExplosionShapeSummary::from_serialized_name)
                .flatten()
        })
        .ok_or_else(|| c.error("invalid explosion shape"))?;
    let flag = |name: &str| {
        value
            .field(name)
            .map(|value| c.flag(value, name))
            .transpose()
            .map(Option::unwrap_or_default)
    };
    Ok(FireworkExplosionSummary {
        shape,
        colors: int_list(c, value, "colors")?,
        fade_colors: int_list(c, value, "fade_colors")?,
        has_trail: flag("has_trail")?,
        has_twinkle: flag("has_twinkle")?,
    })
}

pub fn read_fireworks(
    value: RegistryElementValueRef<'_>,
) -> Result<(i32, Vec<FireworkExplosionSummary>)> {
    let c = ComponentReader::new(
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        "minecraft:fireworks",
    );
    if !value.is_compound() {
        return Err(c.error("fireworks is not a compound"));
    }
    let flight = value
        .field("flight_duration")
        .map(|value| {
            c.int(value, "flight duration")
                .map(|value| i32::from(value as i8 as u8))
        })
        .transpose()?
        .unwrap_or(0);
    let explosions = value
        .field("explosions")
        .map(|values| {
            let values = values
                .list_elements()
                .ok_or_else(|| c.error("explosions is not a list"))?;
            if values.len() > MAX_FIREWORK_EXPLOSIONS {
                return Err(c.error("more than 256 firework explosions"));
            }
            values.map(read_explosion).collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok((flight, explosions))
}

pub(super) fn write_explosion(explosion: &FireworkExplosionSummary) -> NbtSummaryValue {
    let mut fields = vec![entry(
        "shape",
        NbtSummaryValue::String(explosion.shape.as_str().into()),
    )];
    if !explosion.colors.is_empty() {
        fields.push(entry(
            "colors",
            NbtSummaryValue::List(
                explosion
                    .colors
                    .iter()
                    .copied()
                    .map(NbtSummaryValue::Int)
                    .collect(),
            ),
        ));
    }
    if !explosion.fade_colors.is_empty() {
        fields.push(entry(
            "fade_colors",
            NbtSummaryValue::List(
                explosion
                    .fade_colors
                    .iter()
                    .copied()
                    .map(NbtSummaryValue::Int)
                    .collect(),
            ),
        ));
    }
    if explosion.has_trail {
        fields.push(entry("has_trail", NbtSummaryValue::Byte(1)));
    }
    if explosion.has_twinkle {
        fields.push(entry("has_twinkle", NbtSummaryValue::Byte(1)));
    }
    NbtSummaryValue::Compound(fields)
}

/// `Fireworks.CODEC`: `flight_duration` is an
/// `UNSIGNED_BYTE` defaulting to `0` and `explosions` a list of at most
/// `MAX_EXPLOSIONS` (256) records defaulting to empty.
pub(super) fn write_fireworks(
    type_id: i32,
    resource_id: &'static str,
    flight_duration: i32,
    explosions: &[FireworkExplosionSummary],
) -> Result<NbtSummaryValue> {
    // `ExtraCodecs.UNSIGNED_BYTE` refuses a value above 255 while encoding
    if !(0..=255).contains(&flight_duration) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("value must be within range [0;255]: {flight_duration}"),
        ));
    }
    if explosions.len() > MAX_FIREWORK_EXPLOSIONS {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("more than {MAX_FIREWORK_EXPLOSIONS} firework explosions"),
        ));
    }
    let mut fields = Vec::new();
    // `Fireworks.java:22`, `ExtraCodecs.java:143-147`: `UNSIGNED_BYTE` encodes as `ByteTag`.
    if flight_duration != 0 {
        fields.push(entry(
            "flight_duration",
            NbtSummaryValue::Byte(flight_duration as i8),
        ));
    }
    if !explosions.is_empty() {
        fields.push(entry(
            "explosions",
            NbtSummaryValue::List(explosions.iter().map(write_explosion).collect()),
        ));
    }
    Ok(NbtSummaryValue::Compound(fields))
}
