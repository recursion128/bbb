//! Pure value types shared between `bbb-renderer` and the native projection crates, so consumers
//! like `bbb-item-model` can name renderer-facing data without depending on the renderer itself.
//! `bbb-renderer` re-exports everything here at its existing paths.

mod entity_textures;
mod hud_glyphs;
mod map_backgrounds;
mod map_decorations;
pub mod player_skin;
mod sprites;

pub use entity_textures::{
    EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityEquipmentLayerTexture, EntityModelTextureRef, EntityPlayerSkin, EntityPlayerSkinModel,
};
pub use hud_glyphs::{
    HudAsciiGlyph, HudDigitGlyph, HudUvRect, HUD_ASCII_FIRST_GLYPH, HUD_ASCII_GLYPH_COUNT,
    HUD_ASCII_LAST_GLYPH,
};
pub use map_backgrounds::{FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture};
pub use map_decorations::ItemFrameMapDecorationTexture;
pub use player_skin::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage,
};
pub use sprites::{ItemSpriteRect, SpriteAlphaMask};
