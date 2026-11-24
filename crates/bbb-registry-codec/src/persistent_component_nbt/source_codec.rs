//! Source-aware persistent component patches and item stacks.

use crate::component_source_codec::decode_component_source;
use crate::registry_codec::{
    native_outcome,
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};
use bbb_facts::data_component_types::{self};
use bbb_protocol::{
    content_registry::ContentRegistry, normalize_resource_location, NbtSummaryValue,
};

mod component_patch;
mod holders;
mod instrument_component;
mod item_stack_template;
pub(crate) mod loot_holder_parameters;
mod predicate_components;
mod trim_components;
pub use loot_holder_parameters::decode_loot_holder_set_parameter_source;
mod loot_book_parameters;
pub use loot_book_parameters::decode_loot_book_parameter_source;

pub(crate) use component_patch::decode_data_component_patch_source;
pub(crate) use holders::{
    decode_fixed_holder_keyed_map as decode_fixed_holder_keyed_map_source,
    decode_fixed_holder_reference as decode_fixed_holder_reference_source,
    decode_sound_event_file_holder as decode_sound_event_holder_source,
};
pub(crate) use instrument_component::decode_inline_instrument as decode_instrument_direct_source;
pub(crate) use item_stack_template::decode_item_stack_template_source;
pub(crate) use predicate_components::{
    criterion::{
        decode_damage_source_predicate as decode_criterion_damage_source_predicate_source,
        decode_entity_predicate as decode_criterion_entity_predicate_source,
        decode_location_predicate as decode_criterion_location_predicate_source,
    },
    decode_item_predicate as decode_criterion_item_predicate_source,
    decode_state_properties_predicate as decode_criterion_state_properties_predicate_source,
};
pub(crate) use trim_components::{
    decode_trim_material_direct as decode_trim_material_direct_source,
    decode_trim_pattern_direct as decode_trim_pattern_direct_source,
};

pub fn decode_loot_component_parameter_source(
    type_id: i32,
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    decode_loot_component_parameter_source_with_content_registry(
        type_id,
        input,
        source,
        limits,
        ContentRegistry::vanilla(),
    )
}

pub fn decode_loot_component_parameter_source_with_content_registry(
    type_id: i32,
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:loot_function",
        "DataComponent.CODEC",
        source,
        limits,
    )?
    .with_content_registry(content_registry);
    native_outcome::validate_raw_input(input, "parameter", &mut context)?;
    let descriptor = data_component_types::vanilla_data_component_type(type_id)
        .ok_or_else(|| context.codec_error("parameter", "unknown component type"))?;
    decode_component_value_source(descriptor, input, "parameter", &mut context)
        .map(|value| value.into_parts().0)
}

pub fn decode_item_stack_template_parameter_source(
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
    component_limits: super::PersistentNbtDecodeLimits,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    decode_item_stack_template_parameter_source_with_content_registry(
        input,
        source,
        limits,
        component_limits,
        ContentRegistry::vanilla(),
    )
}

pub fn decode_item_stack_template_parameter_source_with_content_registry(
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
    component_limits: super::PersistentNbtDecodeLimits,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:advancement",
        "ItemStackTemplate.CODEC",
        source,
        limits,
    )?
    .with_content_registry(content_registry);
    native_outcome::validate_raw_input(input, "icon", &mut context)?;
    if let Some(components) = input.field("components") {
        let mut component_context = RegistryCodecDecodeContext::with_limits(
            "minecraft:advancement",
            "DataComponentPatch.CODEC",
            source,
            component_limits,
        )?
        .with_content_registry(content_registry);
        native_outcome::validate_raw_input(components, "icon.components", &mut component_context)?;
    }
    decode_item_stack_template_source(input, "icon", &mut context).map(|value| value.into_parts().0)
}

pub fn decode_loot_text_parameter_source(
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    decode_loot_text_parameter_source_with_content_registry(
        input,
        source,
        limits,
        ContentRegistry::vanilla(),
    )
}

pub fn decode_loot_text_parameter_source_with_content_registry(
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::PersistentNbtDecodeLimits,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:loot_function",
        "ComponentSerialization.CODEC",
        source,
        limits,
    )?
    .with_content_registry(content_registry);
    native_outcome::validate_raw_input(input, "text", &mut context)?;
    decode_component_source(input, "text", &mut context).map(|value| value.into_parts().0)
}

/// Decodes `ExtraCodecs.RGB_COLOR_CODEC` for non-component source codecs.
///
/// The component family remains the single owner of the integer/vector
/// alternative and its exact channel conversion; registry records reuse this
/// narrow facade instead of copying the codec.
pub(crate) fn decode_rgb_color_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    if let Some(value) = input.codec_i32() {
        context.charge_node(field)?;
        return Ok(value);
    }
    context.with_nested(field, |context| {
        context.charge_node(field)?;
        let channels = input
            .list_elements()
            .filter(|items| items.len() == 3)
            .and_then(|items| {
                items
                    .map(|value| value.codec_f32())
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(|| {
                context.codec_error(field, "RGB color is neither Codec.INT nor a float vector")
            })?;
        // ARGB: Mth.floor(value * 255.0F) masked to eight bits.
        let channel = |value: f32| ((value * 255.0_f32).floor() as i32 as u32) & 0xff;
        Ok((0xff00_0000_u32
            | (channel(channels[0]) << 16)
            | (channel(channels[1]) << 8)
            | channel(channels[2])) as i32)
    })
}

/// Decodes the compact named-holder set used by world-generation codecs.
///
/// This is the `alwaysUseList = false` `RegistryFixedCodec` shape: a direct
/// singleton writes as a scalar, larger direct sets write as lists, and a tag
/// keeps its `#id` spelling.  The persistent-component holder owner remains
/// the only implementation so every registry consumer retains references in
/// the same encounter order (`RegistryCodecs`).
pub(crate) fn decode_homogeneous_fixed_holder_set_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    target_registry: &'static str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    holders::decode_homogeneous_fixed_holder_set(input, field, context, target_registry, false)
}

/// One component value, read by the item reader: the value is kept as the NBT
/// it converts to, with the dynamic-registry holders it names as references.
pub(super) fn decode_component_value_source(
    descriptor: &'static data_component_types::DataComponentTypeCatalogEntry,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if !descriptor.persistent {
        return Err(context.codec_error(
            field,
            format!("{} is a transient data component", descriptor.resource_id),
        ));
    }
    let value = crate::json_nbt::element_value_to_nbt(input)
        .map_err(|reason| context.codec_error(field, reason))?;
    crate::persistent_item_stack_projection::canonical_persistent_component(
        descriptor.type_id,
        descriptor.resource_id,
        &value,
        context.owner_registry(),
        context.content_registry(),
    )
    .map_err(|error| context.codec_error(field, error.to_string()))
}

pub(crate) fn java_float_compare(left: f32, right: f32) -> std::cmp::Ordering {
    if left < right {
        return std::cmp::Ordering::Less;
    }
    if left > right {
        return std::cmp::Ordering::Greater;
    }
    let bits = |value: f32| {
        if value.is_nan() {
            0x7fc0_0000_u32 as i32
        } else {
            value.to_bits() as i32
        }
    };
    bits(left).cmp(&bits(right))
}

pub(super) fn decode_identifier(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryResourceIdentifier, RegistryCodecError> {
    context.charge_node(field)?;
    let raw = input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(field, "expected a resource identifier string"))?;
    RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(field, format!("invalid resource identifier {raw}")))
}

pub(super) fn normalize_component_identifier(
    raw: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    normalize_resource_location(raw)
        .map_err(|_| context.codec_error(field, format!("invalid data component id {raw}")))
}
