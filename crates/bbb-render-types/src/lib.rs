//! Pure value types shared between `bbb-client-render` and upstream client crates
//! (`bbb-client-assets`, `bbb-client-world`, and `bbb-client`) so consumers can name
//! renderer-facing data without depending on the renderer itself.

mod atlas_animation;
mod command_suggestions;
pub mod dynamic_lights;
mod entity_model_families;
pub mod entity_render_state;
pub mod entity_rigs;
mod entity_textures;
mod hud_glyphs;
mod item_defaults;
mod light_coords;
mod map_backgrounds;
mod map_decorations;
mod particles;
pub mod player_skin;
mod resource_shaders;
mod selection;
mod sprites;
mod terrain;
mod text_decomposer;
mod text_visual_order;
mod texture_sampling;
pub mod wasm_sync;

pub use atlas_animation::{
    AtlasGpuAnimationFrame, AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasMipPatch,
    AtlasRect, SpriteAnimation, SpriteAnimationFrame, SpriteAnimationFrameTick, SpriteMipLevel,
};
pub use bbb_facts::{
    cat_variants, color, temperature_animal_variants, wolf_variants, zombie_nautilus_variants,
    BannerPatternKind, VanillaDyeColor, VillagerModelProfession, VillagerModelType,
    VANILLA_DYE_COLORS, VANILLA_DYE_COLOR_COUNT, VANILLA_SHEEP_COLOR_DURATION_TICKS,
};
pub use command_suggestions::{
    HudCommandInputRange, HudCommandInputSyntax, HudCommandSuggestionEntry,
};
pub use dynamic_lights::{DynamicLightsMode, DynamicPointLight, MAX_DYNAMIC_POINT_LIGHTS};
pub use entity_model_families::{
    BoatModelFamily, LlamaModelFamily, PiglinModelFamily, UndeadHorseModelFamily,
};
pub use entity_render_state::*;
pub use entity_rigs::{
    DataDrivenRigAnimation, DataDrivenRigBone, DataDrivenRigBoneChannels, DataDrivenRigChannel,
    DataDrivenRigCube, DataDrivenRigInterpolation, DataDrivenRigKeyframe, DataDrivenRigSpec,
    DataDrivenRigTarget,
};
pub use entity_textures::{
    EntityArmorTrim, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityDynamicTexture, EntityEquipmentAssetId, EntityModelTextureRef, EntityPlayerSkin,
    EntityPlayerSkinModel, EntityPlayerTextures,
};
pub use hud_glyphs::{
    HudAsciiGlyph, HudComponentTextRun, HudDigitGlyph, HudDynamicAtlasSnapshot,
    HudDynamicAtlasUpdate, HudDynamicFontAtlas, HudDynamicGlyphUpload, HudEffectKind,
    HudEffectRect, HudFontGlyphBook, HudFontGlyphMap, HudGlyphArea, HudGlyphQuad,
    HudObfuscatedGlyphPool, HudObfuscatedRandom, HudStyledTextRun, HudSubsequence,
    HudSubsequenceKey, HudTextStyle, HudUvRect, HUD_FONT_BASELINE, HUD_FONT_BOLD_EXTRA_THICKNESS,
    HUD_FONT_BOLD_OFFSET, HUD_FONT_SHADOW_OFFSET,
};
pub use item_defaults::{ItemComponentDefaults, ItemComponentDefaultsView};
pub use light_coords::VanillaLightCoords;
pub use map_backgrounds::{FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture};
pub use map_decorations::{
    vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id,
    vanilla_map_decoration_types, ItemFrameMapDecorationTexture, MapDecorationTypeCatalogEntry,
    VanillaMapDecorationType,
};
pub use particles::{ParticleFacingCameraMode, ParticleTextureAtlasKind};
pub use player_skin::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicEntityTextureImage,
    DynamicPlayerSkinImage, DynamicPlayerTextureImage, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
    MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
};
pub use resource_shaders::{
    CoreShaderSourceProjection, PostEffectGraph, PostEffectInput, PostEffectInternalTarget,
    PostEffectPass, PostEffectResourceProjection, PostEffectTextureImage, PostEffectUniform,
    PostEffectUniformValue, ShaderSource, ShaderSourceImport, ShaderSourceKey,
    ShaderSourceLoadFailure, ShaderStage,
};
pub use selection::{
    BlockSelectionPhase, BlockSelectionRenderState, DebugLine, SelectionBox, SelectionColoredBox,
    SelectionLine, SelectionOutline, SelectionPoint, SelectionTextLabel, SELECTION_OUTLINE_ALPHA,
};
pub use sprites::{ItemSpriteRect, SpriteAlphaMask};
pub use terrain::{
    packed_long_len, palette_kind, terrain_skip_rendering_from_vanilla_facts, PackedLightArray,
    PaletteDomain, PaletteKind, PaletteValue, PalettedContainerData, TerrainBaseStateFactsFn,
    TerrainBlockRenderContext, TerrainBlockRenderState, TerrainChunkSource, TerrainFluidKind,
    TerrainFluidState, TerrainLight, TerrainMaterialClass, TerrainSectionLightRef,
    TerrainSectionView, TerrainSkipRendering, LIGHT_ARRAY_BYTES,
};
pub use text_decomposer::{
    visit_legacy_text, visit_legacy_utf16, LegacyTextFormat, SourceTextChar,
};
pub use text_visual_order::{visual_order, visual_order_indexed};
pub use texture_sampling::StandaloneTextureSampling;
