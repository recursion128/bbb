use super::*;

pub fn decode_loot_holder_set_parameter_source(
    registry: &'static str,
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::super::PersistentNbtDecodeLimits,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    decode(registry, input, source, limits).map(|value| value.into_parts().0)
}

pub(crate) fn decode(
    registry: &'static str,
    input: RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
    limits: super::super::PersistentNbtDecodeLimits,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut context = RegistryCodecDecodeContext::with_limits(
        "minecraft:loot_function",
        "HolderSet.CODEC",
        source,
        limits,
    )?;
    native_outcome::validate_raw_input(input, "options", &mut context)?;
    holders::decode_homogeneous_fixed_holder_set(input, "options", &mut context, registry, false)
}
