mod biomes;
mod block_states;
mod enchantments;
mod environment_attributes;
mod instruments;
mod state;
mod store;

pub use biomes::{
    BiomeClientProjection, BiomeGrassColorModifier, BiomeProjectionError, BiomeTemperatureModifier,
};
pub use block_states::{
    vanilla_air_block_state_id, vanilla_block_state_id, BlockStateInfo, BlockStateRegistry,
};
pub use enchantments::EnchantmentAttributeModifier;
pub use environment_attributes::ClientEnvironmentAttributes;
pub(crate) use environment_attributes::{
    decode_client_environment_attributes, EnvironmentAttributeProjectionError,
};
pub use instruments::VanillaInstrumentIdentity;
pub use state::{
    RegistryContentState, RegistryEntrySource, RegistryPacket, RegistryPacketEntry, RegistrySet,
    RegistryTagState, ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};

#[cfg(test)]
mod tests;
