//! Exact source codec for Vanilla 26.1 world-generation block predicates.
//!
//! The generated dispatch catalog exhausts all 13 registrations and every
//! payload follows its owning `MapCodec` field order before the normalized
//! `type` discriminator (`BlockPredicate`,
//! `BlockPredicateType`). State-testing offsets use
//! `Vec3i.offsetCodec(16)`, while `unobstructed` deliberately uses unbounded
//! `Vec3i.CODEC` (`StateTestingPredicate`).

use crate::{
    codec_dispatch_types::{block_predicate_codec_type, BlockPredicateCodecFamily},
    persistent_component_nbt::source_codec::decode_homogeneous_fixed_holder_set_source,
    registry_codec::{
        block_state_source::decode_block_state_source,
        recursive_values::{
            decode_list, required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_protocol::NbtSummaryValue;

use super::value::{child, decode_dispatch_type};

const BLOCK_REGISTRY: &str = "minecraft:block";
const FLUID_REGISTRY: &str = "minecraft:fluid";
const OFFSET_BOUND: i32 = 16;
const DIRECTIONS: &[&str] = &["down", "up", "north", "south", "west", "east"];

/// Decodes the complete six-value `Direction.CODEC` string surface.
///
/// World-generation owners such as `BlockColumnConfiguration` use the full
/// enum codec, while `EnvironmentScanPlacement` deliberately uses
/// `Direction.VERTICAL_CODEC`; keeping this helper beside the existing
/// generated block-predicate consumer prevents those two contracts from
/// being conflated.
pub(crate) fn decode_direction_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let direction = input
        .java_string_ref()
        .filter(|value| DIRECTIONS.iter().any(|name| *value == *name))
        .ok_or_else(|| context.codec_error(field, "unknown direction"))?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
        direction.clone(),
    )))
}

/// Decodes `BlockPredicate.CODEC` from server JSON, known-pack JSON, or NBT.
pub(crate) fn decode_block_predicate_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "block predicate")?;
    let (type_id, family) =
        decode_dispatch_type(input, field, context, "block predicate", |resource_id| {
            block_predicate_codec_type(resource_id).map(|entry| entry.codec_family)
        })?;
    let mut output = RegistryCodecRecord::default();

    match family {
        // `MatchingBlocksPredicate` and
        // `MatchingFluidsPredicate` append their holder set after
        // the shared state-testing offset.
        BlockPredicateCodecFamily::MatchingBlocksPredicate => {
            output.push_bounded_offset(input, field, context)?;
            output.push_holder_set(input, "blocks", field, context, BLOCK_REGISTRY)?;
        }
        // This is `TagKey.codec`, not the `#`-prefixed holder-set codec, so it
        // is a normalized identifier rather than a membership reference
        BlockPredicateCodecFamily::MatchingBlockTagPredicate => {
            output.push_bounded_offset(input, field, context)?;
            output.push_identifier(input, "tag", field, context)?;
        }
        BlockPredicateCodecFamily::MatchingFluidsPredicate => {
            output.push_bounded_offset(input, field, context)?;
            output.push_holder_set(input, "fluids", field, context, FLUID_REGISTRY)?;
        }
        // Vanilla freezes offset before direction in the record group
        BlockPredicateCodecFamily::HasSturdyFacePredicate => {
            output.push_bounded_offset(input, field, context)?;
            output.push_direction(input, "direction", field, context)?;
        }
        BlockPredicateCodecFamily::SolidPredicate
        | BlockPredicateCodecFamily::ReplaceablePredicate
        | BlockPredicateCodecFamily::InsideWorldBoundsPredicate => {
            // These owners carry only the shared offset
            output.push_bounded_offset(input, field, context)?;
        }
        BlockPredicateCodecFamily::WouldSurvivePredicate => {
            // Offset precedes the shared exact BlockState owner
            output.push_bounded_offset(input, field, context)?;
            let state_field = child(field, "state");
            let state = required_field(input, "state", field, context)?;
            let state = context.with_nested(&state_field, |context| {
                decode_block_state_source(state, &state_field, context)
            })?;
            output.push("state", state);
        }
        BlockPredicateCodecFamily::AnyOfPredicate | BlockPredicateCodecFamily::AllOfPredicate => {
            // Both combinators own the same recursive list map codec
            let predicates_field = child(field, "predicates");
            let predicates = required_field(input, "predicates", field, context)?;
            let predicates = context.with_nested(&predicates_field, |context| {
                decode_list(
                    predicates,
                    &predicates_field,
                    context,
                    decode_block_predicate_source,
                )
            })?;
            output.push("predicates", predicates);
        }
        BlockPredicateCodecFamily::NotPredicate => {
            // `NotPredicate` owns one recursive predicate field.
            let predicate_field = child(field, "predicate");
            let predicate = required_field(input, "predicate", field, context)?;
            let predicate = context.with_nested(&predicate_field, |context| {
                decode_block_predicate_source(predicate, &predicate_field, context)
            })?;
            output.push("predicate", predicate);
        }
        // The unit map contributes no payload.
        BlockPredicateCodecFamily::TrueBlockPredicate => {}
        BlockPredicateCodecFamily::UnobstructedPredicate => {
            output.push_optional_offset(input, field, context, false)?;
        }
    }

    Ok(output.finish_typed(type_id))
}

impl RegistryCodecRecord {
    fn push_bounded_offset(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        self.push_optional_offset(input, field, context, true)
    }

    fn push_optional_offset(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
        bounded: bool,
    ) -> Result<(), RegistryCodecError> {
        let Some(offset) = input.field("offset") else {
            return Ok(());
        };
        let offset_field = child(field, "offset");
        let offset = context.with_nested(&offset_field, |context| {
            decode_vec3i(offset, &offset_field, context, bounded)
        })?;
        if offset.network_nbt_unchecked() != &NbtSummaryValue::IntArray(vec![0, 0, 0]) {
            self.push("offset", offset);
        }
        Ok(())
    }

    fn push_holder_set(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
        target_registry: &'static str,
    ) -> Result<(), RegistryCodecError> {
        let holder_field = child(field, name);
        let holder_set = required_field(input, name, field, context)?;
        let holder_set = context.with_nested(&holder_field, |context| {
            decode_homogeneous_fixed_holder_set_source(
                holder_set,
                &holder_field,
                context,
                target_registry,
            )
        })?;
        self.push(name, holder_set);
        Ok(())
    }

    fn push_identifier(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let value_field = child(field, name);
        context.charge_node(&value_field)?;
        let raw = required_field(input, name, field, context)?
            .java_string_ref()
            .ok_or_else(|| context.codec_error(&value_field, "expected an identifier string"))?;
        let identifier = RegistryResourceIdentifier::parse_java_string(raw)
            .ok_or_else(|| context.codec_error(&value_field, "invalid identifier"))?;
        self.push(
            name,
            CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
                identifier.into_string().into(),
            )),
        );
        Ok(())
    }

    fn push_direction(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let value_field = child(field, name);
        let direction = required_field(input, name, field, context)?;
        self.push(
            name,
            decode_direction_source(direction, &value_field, context)?,
        );
        Ok(())
    }
}

pub(super) fn decode_vec3i(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    bounded: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected a Vec3i integer stream"))?;
    context.check_collection(field, len)?;
    if len != 3 {
        return Err(context.codec_error(field, "expected exactly 3 Vec3i integers"));
    }
    let mut values = Vec::with_capacity(3);
    for index in 0..len {
        let item_field = format!("{field}[{index}]");
        context.charge_node(&item_field)?;
        let value = input
            .list_element(index)
            .and_then(RegistryElementValueRef::codec_i32)
            .ok_or_else(|| context.codec_error(&item_field, "expected a Codec.INT value"))?;
        values.push(value);
    }
    // Mirror Java `Math.abs(int)`: `MIN_VALUE` overflows back to itself and
    // therefore passes the `< 16` predicate.
    if bounded
        && values
            .iter()
            .any(|value| value.wrapping_abs() >= OFFSET_BOUND)
    {
        return Err(context.codec_error(field, "Vec3i offset is outside the exclusive ±16 bound"));
    }
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::IntArray(
        values,
    )))
}

#[cfg(test)]
mod tests;
