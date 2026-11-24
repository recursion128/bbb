//! Source-aware criterion `LocationPredicate.CODEC`.
//!
//! Vanilla 26.1 sources: `LocationPredicate`,
//! `net/minecraft/advancements/criterion/BlockPredicate`,
//! `net/minecraft/advancements/criterion/FluidPredicate`, and
//! `StatePropertiesPredicate`.

use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

use super::super::super::holders::decode_homogeneous_fixed_holder_set;
use super::super::{
    child, decode_block_predicate, decode_bool_value, decode_double_bounds,
    decode_identifier_value, decode_int_bounds, decode_state_properties_predicate,
    RegistryCodecRecord,
};

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
const FLUID_REGISTRY: &str = "minecraft:fluid";

pub(crate) fn decode(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "location predicate")?;
    let mut output = RegistryCodecRecord::default();

    if let Some(position) = input.field("position") {
        let child = child(field, "position");
        let decoded =
            context.with_nested(&child, |context| decode_position(position, &child, context))?;
        output.push("position", decoded);
    }
    if let Some(biomes) = input.field("biomes") {
        let child = child(field, "biomes");
        let decoded = context.with_nested(&child, |context| {
            decode_homogeneous_fixed_holder_set(biomes, &child, context, BIOME_REGISTRY, false)
        })?;
        output.push("biomes", decoded);
    }
    if let Some(structures) = input.field("structures") {
        let child = child(field, "structures");
        let decoded = context.with_nested(&child, |context| {
            decode_homogeneous_fixed_holder_set(
                structures,
                &child,
                context,
                STRUCTURE_REGISTRY,
                false,
            )
        })?;
        output.push("structures", decoded);
    }
    if let Some(dimension) = input.field("dimension") {
        let child = child(field, "dimension");
        let decoded = context.with_nested(&child, |context| {
            decode_identifier_value(dimension, &child, context)
        })?;
        output.push("dimension", decoded);
    }
    if let Some(smokey) = input.field("smokey") {
        let child = child(field, "smokey");
        let decoded =
            context.with_nested(&child, |context| decode_bool_value(smokey, &child, context))?;
        output.push("smokey", decoded);
    }
    if let Some(light) = input.field("light") {
        let child = child(field, "light");
        let decoded =
            context.with_nested(&child, |context| decode_light(light, &child, context))?;
        output.push("light", decoded);
    }
    if let Some(block) = input.field("block") {
        let child = child(field, "block");
        let decoded = context.with_nested(&child, |context| {
            decode_block_predicate(block, &child, context)
        })?;
        output.push("block", decoded);
    }
    if let Some(fluid) = input.field("fluid") {
        let child = child(field, "fluid");
        let decoded =
            context.with_nested(&child, |context| decode_fluid(fluid, &child, context))?;
        output.push("fluid", decoded);
    }
    if let Some(can_see_sky) = input.field("can_see_sky") {
        let child = child(field, "can_see_sky");
        let decoded = context.with_nested(&child, |context| {
            decode_bool_value(can_see_sky, &child, context)
        })?;
        output.push("can_see_sky", decoded);
    }

    Ok(output.finish())
}

fn decode_position(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "location position")?;
    let mut output = RegistryCodecRecord::default();
    for name in ["x", "y", "z"] {
        if let Some(bounds) = input.field(name) {
            let child = child(field, name);
            let decoded = context.with_nested(&child, |context| {
                decode_double_bounds(bounds, &child, context)
            })?;
            if !decoded.is_any {
                output.push(name, decoded.value);
            }
        }
    }
    Ok(output.finish())
}

fn decode_light(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "light predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(light) = input.field("light") {
        let child = child(field, "light");
        let decoded =
            context.with_nested(&child, |context| decode_int_bounds(light, &child, context))?;
        if !decoded.is_any {
            output.push("light", decoded.value);
        }
    }
    Ok(output.finish())
}

fn decode_fluid(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "fluid predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(fluids) = input.field("fluids") {
        let child = child(field, "fluids");
        let decoded = context.with_nested(&child, |context| {
            decode_homogeneous_fixed_holder_set(fluids, &child, context, FLUID_REGISTRY, false)
        })?;
        output.push("fluids", decoded);
    }
    if let Some(state) = input.field("state") {
        let child = child(field, "state");
        let decoded = context.with_nested(&child, |context| {
            decode_state_properties_predicate(state, &child, context)
        })?;
        output.push("state", decoded);
    }
    Ok(output.finish())
}

#[cfg(test)]
mod tests;
