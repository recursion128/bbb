//! Pure value types shared between `bbb-renderer` and the native projection crates, so consumers
//! like `bbb-item-model` can name renderer-facing data without depending on the renderer itself.
//! `bbb-renderer` re-exports everything here at its existing paths.

mod banner_pattern_types;
mod entity_textures;
mod hud_glyphs;
mod map_backgrounds;
mod map_decorations;
pub mod player_skin;
mod sprites;
mod villager_model_types;

pub use banner_pattern_types::BannerPatternKind;
pub use entity_textures::{
    EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityEquipmentLayerTexture, EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel,
};
pub use hud_glyphs::{
    HudAsciiGlyph, HudDigitGlyph, HudEffectKind, HudEffectRect, HudFontGlyphMap, HudGlyphQuad,
    HudObfuscatedGlyphPool, HudObfuscatedRandom, HudStyledTextRun, HudTextStyle, HudUvRect,
    HUD_FONT_BASELINE, HUD_FONT_BOLD_EXTRA_THICKNESS, HUD_FONT_BOLD_OFFSET, HUD_FONT_SHADOW_OFFSET,
};
pub use map_backgrounds::{FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture};
pub use map_decorations::ItemFrameMapDecorationTexture;
pub use player_skin::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage,
};
pub use sprites::{ItemSpriteRect, SpriteAlphaMask};
pub use villager_model_types::VillagerModelType;
