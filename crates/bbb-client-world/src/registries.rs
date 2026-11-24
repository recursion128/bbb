use bbb_protocol::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, NbtSummaryEntry,
    NbtSummaryValue, StyledTextRun,
};

mod animal_variants;
mod armor_trims;
mod banner_patterns;
mod biomes;
pub(crate) mod block_states;
mod chat_types;
mod damage_types;
mod dimension_types;
pub(crate) mod element_value;
mod enchantments;
mod environment_attributes;
mod environment_particle_options;
mod environment_sampling;
mod game_tests;
mod instrument_values;
mod instruments;
mod item_component_initializers;
pub(crate) mod item_component_values;
mod jukebox_songs;
mod loader_order;
mod painting_variants;
mod revision;
mod sound_events;
mod sound_variants;
pub(crate) mod state;
mod store;
mod timelines;
mod trim_materials;
mod trim_patterns;

pub use animal_variants::{
    CatVariantPresentation, FarmAnimalVariantModel, FarmAnimalVariantPresentation,
    FrogVariantPresentation, WolfVariantAssetPresentation, WolfVariantPresentation,
    ZombieNautilusVariantModelType, ZombieNautilusVariantPresentation, CAT_VARIANT_REGISTRY,
    CHICKEN_VARIANT_REGISTRY, COW_VARIANT_REGISTRY, FROG_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY,
    WOLF_VARIANT_REGISTRY, ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
};
pub(crate) use armor_trims::{armor_trim_sprite, ArmorTrimLayer};
pub use armor_trims::{
    direct_trim_material_presentation, direct_trim_pattern_presentation, ArmorTrimSprite,
    TrimMaterialPresentation, TrimPatternPresentation, TRIM_MATERIAL_REGISTRY,
    TRIM_PATTERN_REGISTRY,
};
pub use banner_patterns::{direct_banner_pattern_presentation, BANNER_PATTERN_REGISTRY};
pub use bbb_registry_codec::registry_codec::direct_leaf::{
    BannerPatternElement, DamageEffects, DamageScaling, DamageTypeElement, DeathMessageType,
};
#[cfg(test)]
pub(crate) use biomes::test_biome_element_nbt;
pub use biomes::{
    BiomeClientProjection, BiomeGrassColorModifier, BiomeProjectionError, BiomeTemperatureModifier,
};
pub use block_states::{
    vanilla_air_block_state_id, vanilla_block_state_id, BlockStateInfo, BlockStateRegistry,
};
pub(crate) use chat_types::{resolve_chat_type, ResolvedChatType};
pub use chat_types::{
    ChatTypeDecoration, ChatTypeDecorationParameter, ChatTypeProjectionError, CHAT_TYPE_REGISTRY,
};
pub use damage_types::DAMAGE_TYPE_REGISTRY;
pub use dimension_types::DIMENSION_TYPE_REGISTRY;
pub use element_value::{RegistryElementData, RegistryElementDecodeError, RegistryElementSource};
pub use enchantments::{
    EnchantmentAttributeModifier, EnchantmentTooltipDefinition, VanillaEnchantmentIdentity,
    ENCHANTMENT_REGISTRY, VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
};
pub(crate) use environment_attributes::decode_client_environment_attributes;
pub use environment_attributes::{
    AmbientLoopSoundState, ClientEnvironmentAttributes, ClientMoonPhase,
};
pub(crate) use environment_particle_options::validate_command_particle_fields;

pub use bbb_registry_codec::{
    IntProvider as WorldIntProvider, WeightedIntProvider as WorldWeightedIntProvider,
};
pub use environment_sampling::EnvironmentBiomeSampling;
pub use game_tests::{TEST_ENVIRONMENT_REGISTRY, TEST_INSTANCE_REGISTRY};
pub use instrument_values::{InstrumentPresentation, INSTRUMENT_REGISTRY};
pub use instruments::VanillaInstrumentIdentity;
pub use jukebox_songs::{JukeboxSongPresentation, JUKEBOX_SONG_REGISTRY};
pub use loader_order::loader_ordered_resource_id;
pub use painting_variants::{
    painting_variant_translation_key, PaintingVariantClientProjection, PaintingVariantPresentation,
    PAINTING_VARIANT_REGISTRY,
};
pub(crate) use sound_events::project_validated_sound_event;
pub use sound_events::RegistrySoundEvent;
pub(crate) use sound_variants::RegistrySoundKind;
pub use state::{
    FrozenRegistryAccess, FrozenRegistryEntry, FrozenRegistryHolderSet,
    FrozenRegistryHolderSetKind, FrozenRegistryMembership, PersistentHolderDecodeError,
    PersistentHolderSetDecodeError, RegistryAccessRevision, RegistryContentState,
    RegistryEntryPayload, RegistryEntrySource, RegistryHolderSetIdentity, RegistryPacket,
    RegistryPacketEntry, RegistryResolutionError, RegistrySet, RegistryTagState,
    ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};
pub use store::RegistryValidationError;
pub use timelines::{TIMELINE_REGISTRY, WORLD_CLOCK_REGISTRY};
pub use trim_materials::VanillaTrimMaterialIdentity;
pub use trim_patterns::VanillaTrimPatternIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryProjectionError {
    pub resource_id: String,
    pub field: String,
    pub reason: String,
}

impl std::fmt::Display for RegistryProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for RegistryProjectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryFieldError {
    pub field: String,
    pub reason: String,
}

impl std::fmt::Display for RegistryFieldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl From<bbb_registry_codec::RegistryCodecError> for RegistryFieldError {
    fn from(error: bbb_registry_codec::RegistryCodecError) -> Self {
        Self {
            field: error.field,
            reason: error.reason,
        }
    }
}

fn canonical_description(
    entries: Vec<NbtSummaryEntry>,
) -> (String, Vec<StyledTextRun>, NbtSummaryValue) {
    let description_nbt = entries
        .into_iter()
        .find(|entry| entry.name == "description")
        .expect("canonical registry description")
        .value;
    let description_styled = styled_component_runs_from_nbt_value(&description_nbt);
    let description = styled_runs_summary_text(&description_styled);
    (description, description_styled, description_nbt)
}

pub fn normalize_codec_identifier(raw: &str) -> Option<String> {
    bbb_protocol::normalize_resource_location(raw).ok()
}

#[cfg(test)]
pub(crate) mod tests;
