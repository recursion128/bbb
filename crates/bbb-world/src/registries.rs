mod biomes;
mod block_states;
mod enchantments;
mod environment_attributes;
mod instruments;
mod state;
mod store;
mod trim_materials;
mod trim_patterns;

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
pub use state::{
    RegistryContentState, RegistryEntrySource, RegistryPacket, RegistryPacketEntry, RegistrySet,
    RegistryTagState, ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};
pub use trim_materials::VanillaTrimMaterialIdentity;
pub use trim_patterns::VanillaTrimPatternIdentity;

#[cfg(test)]
mod tests;
