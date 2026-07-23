mod animal_variants;
mod armor_trims;
mod banner_patterns;
mod biomes;
mod block_states;
mod chat_types;
mod damage_types;
mod dimension_types;
mod element_value;
mod enchantments;
mod environment_attribute_catalog;
mod environment_attributes;
mod game_tests;
mod instrument_values;
mod instruments;
mod int_providers;
mod item_component_initializers;
mod jukebox_songs;
mod painting_variants;
mod revision;
mod sound_events;
mod sound_variants;
mod state;
mod store;
mod timelines;
mod trim_materials;
mod trim_patterns;

pub use animal_variants::{
    CatVariantPresentation, CatVariantProjectionError, FarmAnimalVariantModel,
    FarmAnimalVariantPresentation, FarmAnimalVariantProjectionError, FrogVariantPresentation,
    FrogVariantProjectionError, WolfVariantAssetPresentation, WolfVariantPresentation,
    WolfVariantProjectionError, ZombieNautilusVariantModelType, ZombieNautilusVariantPresentation,
    ZombieNautilusVariantProjectionError, CAT_VARIANT_REGISTRY, CHICKEN_VARIANT_REGISTRY,
    COW_VARIANT_REGISTRY, FROG_VARIANT_REGISTRY, PIG_VARIANT_REGISTRY, WOLF_VARIANT_REGISTRY,
    ZOMBIE_NAUTILUS_VARIANT_REGISTRY,
};
pub(crate) use armor_trims::{armor_trim_sprite, ArmorTrimLayer};
pub use armor_trims::{
    direct_trim_material_presentation, direct_trim_pattern_presentation, ArmorTrimSprite,
    TrimMaterialPresentation, TrimPatternPresentation, TrimRegistryProjectionError,
    TRIM_MATERIAL_REGISTRY, TRIM_PATTERN_REGISTRY,
};
pub use banner_patterns::{
    direct_banner_pattern_presentation, BannerPatternPresentation, BannerPatternProjectionError,
    BANNER_PATTERN_REGISTRY,
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
pub use damage_types::{
    DamageTypeDeathMessageType, DamageTypeEffects, DamageTypePresentation,
    DamageTypeProjectionError, DamageTypeScaling, DAMAGE_TYPE_REGISTRY,
};
pub use dimension_types::DIMENSION_TYPE_REGISTRY;
pub use element_value::{RegistryElementData, RegistryElementDecodeError, RegistryElementSource};
pub use enchantments::{
    EnchantmentAttributeModifier, EnchantmentHolderSetDefinition, EnchantmentTooltipDefinition,
    VanillaEnchantmentIdentity, VANILLA_ENCHANTMENT_TOOLTIP_ORDER,
};
pub(crate) use environment_attributes::{
    decode_client_environment_attributes, validate_command_particle_fields,
    EnvironmentAttributeProjectionError,
};
pub use environment_attributes::{
    ClientAmbientAddition, ClientAmbientMood, ClientAmbientParticle, ClientAmbientSounds,
    ClientBackgroundMusic, ClientEnvironmentAttributes, ClientMoonPhase, ClientMusic,
    ClientParticleOptions, ClientSoundEventHolder,
};
pub use game_tests::{
    GameTestInstanceDefinition, GameTestInstanceKind, GameTestRegistryProjectionError,
    TestEnvironmentDefinition, TestEnvironmentHolder, TestEnvironmentKind, TestGameRuleTypedValue,
    TestGameRuleValue, TestRegistryReference, TestRotation, TestWeather, TEST_ENVIRONMENT_REGISTRY,
    TEST_INSTANCE_REGISTRY,
};
pub use instrument_values::{
    InstrumentPresentation, InstrumentProjectionError, InstrumentSoundEvent, INSTRUMENT_REGISTRY,
};
pub use instruments::VanillaInstrumentIdentity;
pub use int_providers::{WorldIntProvider, WorldWeightedIntProvider};
pub use jukebox_songs::{
    JukeboxSongPresentation, JukeboxSongProjectionError, JukeboxSoundEvent, JUKEBOX_SONG_REGISTRY,
};
pub use painting_variants::{
    painting_variant_translation_key, PaintingVariantClientProjection, PaintingVariantPresentation,
    PaintingVariantProjectionError, PAINTING_VARIANT_REGISTRY,
};
pub(crate) use sound_events::{decode_persistent_sound_event, PersistentSoundEvent};
pub(crate) use sound_variants::{
    RegistrySoundEvent, RegistrySoundKind, CAT_SOUND_VARIANT_REGISTRY,
    CHICKEN_SOUND_VARIANT_REGISTRY, COW_SOUND_VARIANT_REGISTRY, PIG_SOUND_VARIANT_REGISTRY,
    WOLF_SOUND_VARIANT_REGISTRY,
};
pub(crate) use state::PersistentHolderOrInline;
pub use state::{
    FrozenRegistryAccess, FrozenRegistryEntry, FrozenRegistryHolderSet,
    FrozenRegistryHolderSetKind, FrozenRegistryMembership, PersistentHolderDecodeError,
    PersistentHolderSetDecodeError, RegistryAccessRevision, RegistryContentState,
    RegistryEntryPayload, RegistryEntrySource, RegistryHolderSetIdentity, RegistryPacket,
    RegistryPacketEntry, RegistryResolutionError, RegistrySet, RegistryTagState,
    ResolvedRegistryEntry, UnresolvedRegistryTagHolder,
};
pub use store::RegistryValidationError;
pub use timelines::{
    TimelineClockReference, TimelineDefinition, TimelineEasing, TimelineKeyframe, TimelineModifier,
    TimelineProjectionError, TimelineTimeMarker, TimelineTrackDefinition, WorldClockDefinition,
    TIMELINE_REGISTRY, WORLD_CLOCK_REGISTRY,
};
pub use trim_materials::VanillaTrimMaterialIdentity;
pub use trim_patterns::VanillaTrimPatternIdentity;

pub(crate) fn normalize_codec_identifier(raw: &str) -> Option<String> {
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
