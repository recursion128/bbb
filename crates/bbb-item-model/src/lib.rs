//! Value-aware item-model resolution engine for the native client.
//!
//! Owns icon selection, item model definition consumption, and display
//! transforms, plus the profile/skin resolution runtimes that feed player
//! item and entity rendering. Consumed by the `bbb-native` runtime, scene,
//! and HUD paths.

// This engine was extracted verbatim from the `bbb-native` binary. Several
// resolution helpers are reachable only through the binary or exercised only by
// this crate's own tests, so dead-code analysis of the standalone library flags
// them. Allow dead_code to keep the extracted code unchanged rather than delete
// or restructure it as part of a mechanical move.
#![allow(dead_code)]

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
