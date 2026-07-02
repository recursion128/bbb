//! Value-aware item-model resolution engine for the native client.
//!
//! Owns icon selection, item model definition consumption, and display
//! transforms, plus the profile/skin resolution runtimes that feed player
//! item and entity rendering. Consumed by the `bbb-native` runtime, scene,
//! and HUD paths.

pub mod ascii_font;

mod item_runtime;
mod profile_resolver;
mod skin_runtime;

pub use item_runtime::{
    default_player_skin_for_profile_id, GeneratedItemLayer, ItemAtlasIcon, ItemAtlasIconLayer,
    ItemAtlasUvRect, ItemModelCompassContext, ItemModelCompassTarget, ItemModelKeybindContext,
    ItemModelTimeContext, ItemModelUseContext, NativeDynamicPlayerSkinDownload,
    NativeDynamicPlayerTextureDownload, NativeItemRuntime, NativeItemTooltipLine,
};
pub use skin_runtime::{default_player_skin_cache_dir, DynamicPlayerTextureKind};
