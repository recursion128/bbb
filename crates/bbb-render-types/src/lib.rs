//! Pure value types shared between `bbb-renderer` and the native projection crates, so consumers
//! like `bbb-item-model` can name renderer-facing data without depending on the renderer itself.
//! `bbb-renderer` re-exports everything here at its existing paths.

mod atlas_animation;
mod banner_pattern_types;
pub mod cat_variants;
mod dye_colors;
mod entity_textures;
mod hud_glyphs;
mod light_coords;
mod map_backgrounds;
mod map_decorations;
pub mod player_skin;
mod resource_shaders;
mod sprites;
pub mod temperature_animal_variants;
mod text_visual_order;
mod texture_sampling;
mod villager_model_professions;
mod villager_model_types;
pub mod wolf_variants;
pub mod zombie_nautilus_variants;

pub use atlas_animation::{
    AtlasGpuAnimationFrame, AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasMipPatch,
    AtlasRect, SpriteAnimation, SpriteAnimationFrame, SpriteAnimationFrameTick, SpriteMipLevel,
};
pub use banner_pattern_types::BannerPatternKind;
pub use dye_colors::{
    VanillaDyeColor, VanillaDyeColorEntry, VANILLA_DYE_COLORS, VANILLA_DYE_COLOR_COUNT,
    VANILLA_SHEEP_COLOR_DURATION_TICKS,
};
pub use entity_textures::{
    EntityArmorTrim, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityDynamicTexture, EntityEquipmentAssetId, EntityModelTextureRef, EntityPlayerSkin,
    EntityPlayerSkinModel,
};
pub use hud_glyphs::{
    HudAsciiGlyph, HudComponentTextRun, HudDigitGlyph, HudEffectKind, HudEffectRect,
    HudFontGlyphBook, HudFontGlyphMap, HudGlyphQuad, HudObfuscatedGlyphPool, HudObfuscatedRandom,
    HudStyledTextRun, HudTextStyle, HudUvRect, HUD_FONT_BASELINE, HUD_FONT_BOLD_EXTRA_THICKNESS,
    HUD_FONT_BOLD_OFFSET, HUD_FONT_SHADOW_OFFSET,
};
pub use light_coords::VanillaLightCoords;
pub use map_backgrounds::{FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture};
pub use map_decorations::{
    vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id,
    vanilla_map_decoration_types, ItemFrameMapDecorationTexture, VanillaMapDecorationType,
};
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
pub use sprites::{ItemSpriteRect, SpriteAlphaMask};
pub use text_visual_order::visual_order;
pub use texture_sampling::StandaloneTextureSampling;
pub use villager_model_professions::VillagerModelProfession;
pub use villager_model_types::VillagerModelType;
