//! Pure value types shared between `bbb-renderer` and the native projection crates, so consumers
//! like `bbb-item-model` can name renderer-facing data without depending on the renderer itself.
//! `bbb-renderer` re-exports everything here at its existing paths.

mod banner_pattern_types;
pub mod cat_variants;
mod entity_textures;
mod hud_glyphs;
mod map_backgrounds;
mod map_decorations;
pub mod player_skin;
mod sprites;
pub mod temperature_animal_variants;
mod villager_model_professions;
mod villager_model_types;
pub mod wolf_variants;
pub mod zombie_nautilus_variants;

pub use banner_pattern_types::BannerPatternKind;
pub use entity_textures::{
    EntityArmorTrim, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityDynamicTexture, EntityEquipmentLayerTexture, EntityModelTextureRef, EntityPlayerSkin,
    EntityPlayerSkinModel,
};
pub use hud_glyphs::{
    HudAsciiGlyph, HudComponentTextRun, HudDigitGlyph, HudEffectKind, HudEffectRect,
    HudFontGlyphBook, HudFontGlyphMap, HudGlyphQuad, HudObfuscatedGlyphPool, HudObfuscatedRandom,
    HudStyledTextRun, HudTextStyle, HudUvRect, HUD_FONT_BASELINE, HUD_FONT_BOLD_EXTRA_THICKNESS,
    HUD_FONT_BOLD_OFFSET, HUD_FONT_SHADOW_OFFSET,
};
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
pub use sprites::{ItemSpriteRect, SpriteAlphaMask};
pub use villager_model_professions::VillagerModelProfession;
pub use villager_model_types::VillagerModelType;
