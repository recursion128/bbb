mod block_states;
mod enchantments;
mod state;
mod store;

pub use block_states::{
    vanilla_air_block_state_id, vanilla_block_state_id, BlockStateInfo, BlockStateRegistry,
};
pub use enchantments::EnchantmentAttributeModifier;
pub use state::{
    RegistryContentState, RegistryEntrySource, RegistryPacket, RegistryPacketEntry, RegistrySet,
    RegistryTagState, ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};

#[cfg(test)]
mod tests;
