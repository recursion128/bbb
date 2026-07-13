mod animal_variants;
mod biomes;
mod block_states;
mod enchantments;
mod environment_attributes;
mod instruments;
mod sound_variants;
mod state;
mod store;
mod trim_materials;
mod trim_patterns;

pub use animal_variants::{
    FarmAnimalVariantModel, FarmAnimalVariantPresentation, FarmAnimalVariantProjectionError,
    CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY,
};
pub use biomes::{
    BiomeClientProjection, BiomeGrassColorModifier, BiomeProjectionError, BiomeTemperatureModifier,
};
pub use block_states::{
    vanilla_air_block_state_id, vanilla_block_state_id, BlockStateInfo, BlockStateRegistry,
};
pub use enchantments::{
    EnchantmentAttributeModifier, EnchantmentTooltipDefinition, VanillaEnchantmentIdentity,
    VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
};
pub use environment_attributes::ClientEnvironmentAttributes;
pub(crate) use environment_attributes::{
    decode_client_environment_attributes, EnvironmentAttributeProjectionError,
};
pub use instruments::VanillaInstrumentIdentity;
pub(crate) use sound_variants::{
    RegistrySoundEvent, CAT_SOUND_VARIANT_REGISTRY, CHICKEN_SOUND_VARIANT_REGISTRY,
    COW_SOUND_VARIANT_REGISTRY, PIG_SOUND_VARIANT_REGISTRY, WOLF_SOUND_VARIANT_REGISTRY,
};
pub use state::{
    RegistryContentState, RegistryEntrySource, RegistryPacket, RegistryPacketEntry, RegistrySet,
    RegistryTagState, ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};
pub use trim_materials::VanillaTrimMaterialIdentity;
pub use trim_patterns::VanillaTrimPatternIdentity;

fn normalize_codec_identifier(raw: &str) -> Option<String> {
    let (namespace, path) = match raw.find(':') {
        Some(0) => ("minecraft", &raw[1..]),
        Some(separator) => (&raw[..separator], &raw[separator + 1..]),
        None => ("minecraft", raw),
    };
    let valid_namespace = namespace != ".."
        && namespace.chars().all(|ch| {
            ch == '_' || ch == '-' || ch == '.' || ch.is_ascii_lowercase() || ch.is_ascii_digit()
        });
    let valid_path = path.chars().all(|ch| {
        ch == '_'
            || ch == '-'
            || ch == '/'
            || ch == '.'
            || ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
    });
    (valid_namespace && valid_path).then(|| format!("{namespace}:{path}"))
}

#[cfg(test)]
mod tests;
