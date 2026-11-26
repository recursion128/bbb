mod asset_index;
mod atlas;
mod atlas_sources;
mod block_models;
mod colors;
mod component_text;
mod content_assets;
pub mod entity_rigs;
mod equipment_assets;
mod freeze_immune_wearables;
mod furnace_fuels;
mod image;
mod item_cuboid_models;
mod item_models;
mod item_registry;
mod jukebox_song_registry;
mod known_packs;
mod language;
mod metadata;
mod mipmap;
#[cfg(not(target_arch = "wasm32"))]
mod native_library;
mod panorama;
mod particle_definitions;
mod particle_sprites;
mod periodic_notifications;
mod post_effects;
mod resource_budget;
mod resource_json;
mod resource_textures;
mod resources;
mod roots;
mod selected_packs;
mod server_pack;
mod shader_sources;
mod sound_event_registry;
mod sounds;
mod sprites;
mod tags;
pub mod virtual_store;
mod waypoint_styles;
mod win_screen;
mod window_icons;
mod zip_pack;

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub use crate::asset_index::VanillaAssetIndexObservation;
pub use crate::atlas::{
    AtlasAnimationMipCache, AtlasImage, AtlasLayout, AtlasMipImage, AtlasMipLevel, AtlasPacker,
    AtlasSprite,
};
pub use crate::block_models::{
    BlockFaceTextures, BlockModelBox, BlockModelCatalog, BlockModelCross, BlockModelDisplayContext,
    BlockModelDisplayTransform, BlockModelDisplayTransforms, BlockModelFace, BlockModelGuiLight,
    BlockModelQuad, BlockModelShape, BlockRenderModel, BlockStateModelChoices,
};
pub use crate::colors::{
    BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, ColorMapImage,
    GrassColorModifier, TerrainColorMaps,
};
pub use crate::content_assets::{
    ContentRegistryAssetProjection, ModAssetModelKind, ModAssetTint, ModBlockAssetProjection,
    ModItemAssetProjection, ModParticleAssetProjection, ModSoundEventAssetProjection,
    MISSING_TEXTURE_ID,
};
pub use crate::entity_rigs::{
    builtin_inferno_warden_rig_spec, builtin_rig_texture_rgba, EntityRigCatalog,
};
pub use crate::equipment_assets::{
    EquipmentAsset, EquipmentAssetCatalog, EquipmentDyeable, EquipmentLayer, EquipmentLayerType,
};
pub use crate::freeze_immune_wearables::FreezeImmuneWearableCatalog;
pub use crate::furnace_fuels::{FurnaceFuelCatalog, FurnaceFuelSourceRule};
pub use crate::image::{
    ResourceImageMemoryObservation, MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
    MAX_RESOURCE_IMAGE_PROCESS_RGBA_BYTES,
};
pub use crate::item_cuboid_models::{
    ItemCuboidModel, ItemCuboidModelCatalog, ItemCuboidModelSet, ItemCuboidTexture,
    ItemCuboidTextureImageCatalog, ItemCuboidTextureImageSet, ITEM_CUBOID_TEXTURE_ATLASES,
};
pub use crate::item_models::{
    holder_set_matches, int_bounds_match, AttributeModifierPredicate, ClientItemDefinition,
    ClientItemProperties, CollectionPredicate, ComponentSelectWhen, CopperGolemStatuePose,
    DoubleBounds, EnchantmentPredicate, ExactAttributeModifier, ExactJukeboxPlayable,
    ExactTrimMaterial, ExactTrimPattern, FireworkExplosionPredicate, IntBounds,
    ItemComponentPredicate, ItemExactComponent, ItemModelCatalog, ItemModelCompassTarget,
    ItemModelCondition, ItemModelDefinition, ItemModelHolder, ItemModelHolderSet,
    ItemModelRangeProperty, ItemModelScalar, ItemModelSelectProperty, ItemModelSelectWhen,
    ItemModelTimeSource, ItemPredicate, ItemSpecialBedPart, ItemSpecialChestType,
    ItemSpecialEndCubeEffect, ItemSpecialHangingAttachment, ItemSpecialModel, ItemSpecialSkullKind,
    ItemSpecialStandingAttachment, ItemTintSource, RangeDispatchEntry, SelectCase,
};
pub use crate::item_registry::ItemRegistryCatalog;
pub use crate::jukebox_song_registry::{JukeboxSongRegistry, JukeboxSongRegistryEntry};
pub use crate::known_packs::{KnownPackDataSource, KnownPacksManager};
pub use crate::language::{LanguageCatalog, DEFAULT_LANGUAGE_CODE};
pub use crate::metadata::{LanguageInfo, PackMetadataCatalog};
pub use crate::mipmap::generate_sprite_mip_levels;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::native_library::materialize_verified_openal_library;
pub use crate::panorama::PanoramaCatalog;
pub use crate::particle_definitions::{ParticleDefinition, ParticleDefinitionCatalog};
pub use crate::particle_sprites::ParticleSpriteCatalog;
pub use crate::periodic_notifications::{PeriodicNotification, PeriodicNotificationCatalog};
pub use crate::post_effects::{
    PostEffectCatalog, PostEffectLoadFailure, PostEffectShaderImport, PostEffectShaderKey,
    PostEffectShaderSource, PostEffectShaderStage,
};
pub use crate::resource_budget::{
    ResourceDataMemoryObservation, MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
    MAX_RESOURCE_DATA_PROCESS_LOGICAL_BYTES,
};
pub use crate::resource_textures::ResourceTextureImage;
pub use crate::resources::{
    PackResource, PackResourceStack, PackRootResource, ResourceIntegrity, ResourceLocation,
};
pub use crate::roots::{PackRoots, MC_VERSION};
pub use crate::selected_packs::{
    ClientPackCompatibility, ClientPackOptionsReconciliation, ClientPackPosition,
    ClientPackSelection, ClientPackSource, ClientResourceGeneration, SelectedClientPack,
    SelectedClientPackStack,
};
pub use crate::server_pack::{
    prepare_pack_archive, read_client_resource_pack_format_range,
    read_client_resource_pack_metadata, ClientPackMetadata, PreparedPackArchive, ServerPackFormat,
    ServerPackFormatRange,
};
pub use crate::shader_sources::ShaderSourceCatalog;
pub use crate::sound_event_registry::SoundEventRegistry;
pub use crate::sounds::{
    SoundCatalog, SoundEntry, SoundEntryKind, SoundEventDefinition, SoundPreload,
};
pub use crate::sprites::{
    SpriteGuiMetadata, SpriteGuiScaling, SpriteImage, SpriteMipmapStrategy, SpriteNineSliceBorder,
    SpriteSource, SpriteTextureMetadata, SpriteTransparency,
};
pub use crate::tags::{TagCatalog, TagDefinition};
pub use crate::virtual_store::{
    get_virtual_file, has_virtual_file, insert_virtual_file, is_virtual_path, open_resource_file,
    remove_virtual_prefix, resource_file_len, ResourceReader, VIRTUAL_PATH_PREFIX,
};
pub use crate::waypoint_styles::{WaypointStyle, WaypointStyleCatalog};
pub use crate::win_screen::WinScreenTextResources;
pub use crate::window_icons::{
    MacOsWindowIcon, StandardWindowIcon, WindowIconCatalog, WindowIconFamily, WindowIconSet,
};
pub use bbb_render_types::{
    AtlasGpuAnimationFrame, AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasMipPatch,
    AtlasRect, CoreShaderSourceProjection, DataDrivenRigAnimation, DataDrivenRigBone,
    DataDrivenRigBoneChannels, DataDrivenRigChannel, DataDrivenRigCube, DataDrivenRigInterpolation,
    DataDrivenRigKeyframe, DataDrivenRigSpec, DataDrivenRigTarget, DynamicLightsMode,
    DynamicPointLight, PostEffectGraph, PostEffectInput, PostEffectInternalTarget, PostEffectPass,
    PostEffectResourceProjection, PostEffectTextureImage, PostEffectUniform,
    PostEffectUniformValue, ShaderSource, ShaderSourceImport, ShaderSourceKey,
    ShaderSourceLoadFailure, ShaderStage, SpriteAnimation, SpriteAnimationFrame,
    SpriteAnimationFrameTick, SpriteMipLevel, MAX_DYNAMIC_POINT_LIGHTS,
};
pub use component_text::{
    localized_component_runs, visit_localized_template, ComponentTextLookup, LocalizedTemplatePart,
};

pub(crate) use crate::image::{rgba_len, rgba_offset};
pub use image::ResourcePngContentInvalid;
pub use resource_json::{ResourceJsonObject, ResourceJsonString, ResourceJsonValue};

/// Returns whether Rayon parallel iterators may run on the current thread.
///
/// On `wasm32`, calling Rayon from outside a Rayon worker (such as the browser
/// `Window` main thread) enters `LockLatch::wait_and_reset` -> `memory.atomic.wait32`,
/// which traps unconditionally in browsers.
#[inline]
pub(crate) fn can_use_rayon() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        rayon::current_thread_index().is_some()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        true
    }
}
