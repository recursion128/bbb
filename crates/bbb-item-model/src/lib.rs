//! Value-aware item-model resolution engine for the native client.
//!
//! Owns icon selection, item model definition consumption, and display
//! transforms, plus the profile/skin resolution runtimes that feed player
//! item and entity rendering. Consumed by the `bbb-native` runtime, scene,
//! and HUD paths.

pub mod font;

mod item_runtime;
mod profile_resolver;
mod skin_runtime;

pub use item_runtime::{
    default_player_skin_for_profile, default_player_skin_for_profile_id, GeneratedItemLayer,
    ItemAtlasIcon, ItemAtlasIconLayer, ItemAtlasSpriteUv, ItemAtlasUvRect, ItemModelCompassContext,
    ItemModelCompassTarget, ItemModelKeybindContext, ItemModelLeafTransformation,
    ItemModelTimeContext, ItemModelUseContext, NativeDynamicPlayerSkinDownload,
    NativeDynamicPlayerTextureDownload, NativeItemHoverName, NativeItemMapTooltipData,
    NativeItemRuntime, NativeItemTooltipLine, NativeItemTooltipOptions, ResolvedItemModel,
    ResolvedItemModelLayer,
};
pub use skin_runtime::{
    default_player_skin_cache_dir, DynamicPlayerTextureKind, DynamicTextureCacheObservation,
};

fn install_http_crypto_provider() {
    // Keep reqwest's rustls provider explicit and portable across BBB's
    // declared Linux and macOS builds.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
mod tests {
    #[test]
    fn http_clients_use_the_explicit_ring_provider() {
        super::install_http_crypto_provider();
        let provider = rustls::crypto::CryptoProvider::get_default()
            .expect("BBB must install a rustls provider before constructing clients");
        let ring = rustls::crypto::ring::default_provider();
        assert_eq!(provider.cipher_suites, ring.cipher_suites);
        assert!(reqwest::blocking::Client::builder().build().is_ok());
    }
}
