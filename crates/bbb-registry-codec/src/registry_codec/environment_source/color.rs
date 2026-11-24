//! Packed projection of canonical environment colors.

use bbb_protocol::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentAttributeType,
};
use bbb_protocol::NbtSummaryValue;

use crate::registry_codec::recursive_values::RegistryCodecDecodeContext;
use crate::registry_codec::{RegistryCodecError, RegistryElementValueRef};
use crate::{PersistentNbtDecodeLimits, RegistryElementSource};

pub fn color_from_canonical(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::Int(value) => Some(*value as u32),
        NbtSummaryValue::String(value) => value
            .with_utf8(|raw| {
                let digits = raw.strip_prefix('#')?;
                let color = u32::from_str_radix(digits, 16).ok()?;
                match digits.len() {
                    6 => Some(color | 0xff00_0000),
                    8 => Some(color),
                    _ => None,
                }
            })
            .flatten(),
        _ => None,
    }
}

pub fn color_from_source(
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
    limits: PersistentNbtDecodeLimits,
    attribute: &'static VanillaEnvironmentAttribute,
) -> Result<u32, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:environment_attribute_check",
        "Color",
        source,
        limits,
    )?;
    if !matches!(
        attribute.value_type,
        VanillaEnvironmentAttributeType::RgbColor | VanillaEnvironmentAttributeType::ArgbColor
    ) {
        return Err(context.codec_error("value", "attribute is not a color"));
    }
    crate::registry_codec::native_outcome::validate_raw_input(input, "value", &mut context)?;
    super::values::decode_color(
        input,
        "value",
        &mut context,
        attribute.value_type == VanillaEnvironmentAttributeType::ArgbColor,
    )
}
