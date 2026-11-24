//! Source-aware codecs shared by synchronized environment registries.
//!
//! `Biome`, `DimensionType`, and `Timeline` all route values through the
//! built-in environment-attribute catalog.  Vanilla decodes the complete
//! direct value before applying its synchronized filter, so this owner keeps
//! filtered references long enough for the frozen generation to validate
//! them and never substitutes the older client NBT projection.

mod attributes;
pub mod audio;
mod bed_rule;
pub mod color;
pub mod particles;
mod values;

pub(crate) use attributes::{decode_environment_attribute_map_source, EnvironmentAttributeMapMode};
pub(crate) use values::{decode_attribute_value_source, decode_modifier_argument_source};

pub fn canonical_bed_rule_source(
    input: super::RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: crate::PersistentNbtDecodeLimits,
) -> Result<bbb_protocol::NbtSummaryValue, super::RegistryCodecError> {
    use super::recursive_values::RegistryCodecDecodeContext;
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:environment_attribute_check",
        "BedRule.CODEC",
        source,
        limits,
    )?;
    super::native_outcome::validate_raw_input(input, "value", &mut context)?;
    bed_rule::decode_bed_rule_source(input, "value", &mut context).map(|value| value.into_parts().0)
}

fn canonical_attribute_source(
    input: super::RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: crate::PersistentNbtDecodeLimits,
    attribute: &str,
    codec: &str,
) -> Result<bbb_protocol::NbtSummaryValue, super::RegistryCodecError> {
    canonical_attribute_source_with_content_registry(
        input,
        source,
        limits,
        attribute,
        codec,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn canonical_attribute_source_with_content_registry(
    input: super::RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: crate::PersistentNbtDecodeLimits,
    attribute: &str,
    codec: &str,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<bbb_protocol::NbtSummaryValue, super::RegistryCodecError> {
    use super::recursive_values::RegistryCodecDecodeContext;
    let descriptor =
        bbb_protocol::environment_attribute_types::vanilla_environment_attribute(attribute)
            .expect("environment attribute is built in");
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:environment_attribute_check",
        codec,
        source,
        limits,
    )?
    .with_content_registry(content_registry);
    super::native_outcome::validate_raw_input(input, "value", &mut context)?;
    values::decode_attribute_value_source(descriptor, input, "value", &mut context)
        .map(|value| value.into_parts().0)
}
