use bbb_protocol::packets::{PlayerModelTypeSummary, ResolvableProfileSummary};
use bbb_render_types::{
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityPlayerSkin, EntityPlayerSkinModel,
};

use crate::skin_runtime::{
    default_player_skin_for_profile, ProfileTextureAssetKind, ProfileTextureSetCache,
    MAX_PROFILE_TEXTURE_ASSET_ENTRIES, MAX_PROFILE_TEXTURE_PROPERTY_BYTES,
    MAX_PROFILE_TEXTURE_SET_ENTRIES, PROFILE_TEXTURE_SET_CACHE_DURATION,
};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        GameProfilePropertySummary, PlayerSkinPatchSummary, ProfileSkinTextureSummary,
        ProfileTextureSummary, ProfileTexturesSummary,
    };

    #[test]
    fn profile_texture_set_returns_default_until_every_member_is_ready() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let cape_url = "https://textures.minecraft.net/texture/profile-cape";
        let elytra_url = "https://textures.minecraft.net/texture/profile-elytra";
        let profile = remote_profile(
            skin_url,
            Some(cape_url),
            Some(elytra_url),
            PlayerModelTypeSummary::Slim,
        );
        let fallback = EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile));
        let mut cache = ProfileTextureSetCache::default();

        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 3);
        assert_eq!(
            cache
                .pending_downloads_for_profile(&profile, false, &[])
                .len(),
            3
        );
        assert!(cache
            .pending_downloads_for_profile(&profile, false, &[])
            .is_empty());

        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 11);
        cache.mark_resolved(ProfileTextureAssetKind::Cape, cape_url, 12);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        assert_eq!(
            cache.player_texture_for_profile(&profile, ProfileTextureAssetKind::Cape, false, &[],),
            None
        );
        cache.mark_resolved(ProfileTextureAssetKind::Elytra, elytra_url, 13);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: 11,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Slim,
                status: EntityDynamicPlayerSkinStatus::Ready,
            })
        );
        assert_eq!(
            cache.player_texture_for_profile(&profile, ProfileTextureAssetKind::Cape, false, &[],),
            Some(12)
        );
    }

    #[test]
    fn profile_texture_set_failure_falls_back_as_one_unit() {
        let skin_url = "https://textures.minecraft.net/texture/ready-skin";
        let cape_url = "https://textures.minecraft.net/texture/failed-cape";
        let profile = remote_profile(skin_url, Some(cape_url), None, PlayerModelTypeSummary::Wide);
        let fallback = EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile));
        let mut cache = ProfileTextureSetCache::default();

        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 9001);
        cache.mark_failed(ProfileTextureAssetKind::Cape, cape_url);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        assert_eq!(
            cache.player_texture_for_profile(&profile, ProfileTextureAssetKind::Cape, false, &[],),
            None
        );
    }

    #[test]
    fn profile_texture_set_model_patch_applies_after_base_completion() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let mut cache = ProfileTextureSetCache::default();
        let mut model_patched = remote_profile(skin_url, None, None, PlayerModelTypeSummary::Slim);
        model_patched.skin_patch.model = Some(PlayerModelTypeSummary::Wide);
        assert!(matches!(
            cache.player_skin_for_profile(&model_patched, false, &[]),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 44);
        assert_eq!(
            cache.player_skin_for_profile(&model_patched, false, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: 44,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Ready,
            })
        );
    }

    #[test]
    fn profile_texture_set_secure_filter_does_not_create_a_second_download_path() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let profile = remote_profile(skin_url, None, None, PlayerModelTypeSummary::Slim);
        let fallback = EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile));
        let mut cache = ProfileTextureSetCache::default();

        assert_eq!(cache.player_skin_for_profile(&profile, true, &[]), fallback);
        assert!(cache
            .pending_downloads_for_profile(&profile, true, &[])
            .is_empty());
        assert_eq!(
            cache
                .pending_downloads_for_profile(&profile, false, &[])
                .len(),
            1
        );
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 55);
        assert_eq!(cache.player_skin_for_profile(&profile, true, &[]), fallback);
        assert!(matches!(
            cache.player_skin_for_profile(&profile, false, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: 55,
                status: EntityDynamicPlayerSkinStatus::Ready,
                ..
            })
        ));
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 1);
    }

    #[test]
    fn profile_texture_set_can_upgrade_trust_after_an_insecure_consumer_lookup() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use rsa::{pkcs8::EncodePublicKey, Pkcs1v15Sign, RsaPrivateKey};
        use sha1::{Digest, Sha1};

        let skin_url = "https://textures.minecraft.net/texture/trust-upgrade";
        let mut profile = remote_profile(skin_url, None, None, PlayerModelTypeSummary::Slim);
        let mut random = crate::RsaTestRng::seeded(0x5a17_26_2);
        let private_key = RsaPrivateKey::new(&mut random, 1024).unwrap();
        let public_der = private_key.to_public_key().to_public_key_der().unwrap();
        profile.properties[0].signature = Some(
            STANDARD.encode(
                private_key
                    .sign(
                        Pkcs1v15Sign::new::<Sha1>(),
                        &Sha1::digest(profile.properties[0].value.as_bytes()),
                    )
                    .unwrap(),
            ),
        );
        let mut cache = ProfileTextureSetCache::default();

        cache.player_skin_for_profile(&profile, false, &[]);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 56);
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, &[]),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, &[public_der.as_bytes().to_vec()],),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 56, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 56, .. })
        ));
    }

    #[test]
    fn profile_texture_set_key_expires_after_fifteen_seconds_but_texture_cache_survives() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let profile = remote_profile(
            "https://textures.minecraft.net/texture/profile-skin",
            None,
            None,
            PlayerModelTypeSummary::Slim,
        );
        let mut cache = ProfileTextureSetCache::default();
        let started = Instant::now();

        cache.player_skin_for_profile_at(&profile, false, &[], started);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 77);
        cache.player_skin_for_profile_at(
            &profile,
            false,
            &[],
            started + PROFILE_TEXTURE_SET_CACHE_DURATION - Duration::from_millis(1),
        );
        assert_eq!(cache.set_count(), 1);
        let later = started + PROFILE_TEXTURE_SET_CACHE_DURATION * 2;
        assert!(matches!(
            cache.player_skin_for_profile_at(&profile, false, &[], later),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 77, .. })
        ));
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 1);
        assert!(cache
            .pending_downloads_for_profile(&profile, false, &[])
            .is_empty());
    }

    #[test]
    fn profile_texture_set_rejects_oversized_property_before_retention_or_download() {
        let mut profile = remote_profile(
            "https://textures.minecraft.net/texture/oversized-property",
            None,
            None,
            PlayerModelTypeSummary::Slim,
        );
        profile.properties[0].value = "x".repeat(MAX_PROFILE_TEXTURE_PROPERTY_BYTES + 1);
        let fallback = EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile));
        let mut cache = ProfileTextureSetCache::default();

        assert_eq!(
            cache.player_skin_for_profile(&profile, false, &[]),
            fallback
        );
        assert!(cache
            .pending_downloads_for_profile(&profile, false, &[])
            .is_empty());
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 0);
        assert_eq!(
            cache.retained_key_bytes(),
            std::mem::size_of::<uuid::Uuid>()
        );
    }

    #[test]
    fn profile_texture_set_bounds_result_and_asset_counts() {
        let mut cache = ProfileTextureSetCache::default();
        for index in 0..=MAX_PROFILE_TEXTURE_SET_ENTRIES {
            let url = format!("https://textures.minecraft.net/texture/bounded-{index}");
            let profile = remote_profile(&url, None, None, PlayerModelTypeSummary::Slim);
            cache.player_skin_for_profile(&profile, false, &[]);
        }

        assert_eq!(cache.set_count(), MAX_PROFILE_TEXTURE_SET_ENTRIES);
        assert_eq!(cache.asset_count(), MAX_PROFILE_TEXTURE_ASSET_ENTRIES);
        assert!(cache.retained_key_bytes() <= MAX_PROFILE_TEXTURE_SET_ENTRIES * 256);
    }

    #[test]
    fn late_texture_result_only_changes_the_matching_full_profile_key() {
        let first_url = "https://textures.minecraft.net/texture/first-key";
        let second_url = "https://textures.minecraft.net/texture/second-key";
        let first = remote_profile(first_url, None, None, PlayerModelTypeSummary::Slim);
        let second = remote_profile(second_url, None, None, PlayerModelTypeSummary::Wide);
        let mut cache = ProfileTextureSetCache::default();

        cache.player_skin_for_profile(&first, false, &[]);
        cache.player_skin_for_profile(&second, false, &[]);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, first_url, 91);

        assert!(matches!(
            cache.player_skin_for_profile(&first, false, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 91, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&second, false, &[]),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
    }

    #[test]
    fn same_type_and_hash_with_conflicting_full_url_fails_the_later_set() {
        let first_url = "https://textures.minecraft.net/texture/shared-hash";
        let conflicting_url = "https://textures.minecraft.net/other/shared-hash";
        let first = remote_profile(first_url, None, None, PlayerModelTypeSummary::Slim);
        let second = remote_profile(conflicting_url, None, None, PlayerModelTypeSummary::Slim);
        let mut cache = ProfileTextureSetCache::default();

        cache.player_skin_for_profile(&first, false, &[]);
        cache.player_skin_for_profile(&second, false, &[]);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, first_url, 92);

        assert!(matches!(
            cache.player_skin_for_profile(&first, false, &[]),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 92, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&second, false, &[]),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        assert_eq!(cache.asset_count(), 1);
    }

    #[test]
    fn production_sources_keep_one_texture_set_owner_and_no_generation_keyed_shadow_cache() {
        let runtime = include_str!("../item_runtime.rs");
        let callers = include_str!("profiles.rs")
            .split_once("\n#[cfg(test)]")
            .map_or(include_str!("profiles.rs"), |(production, _)| production);
        let transport = include_str!("../skin_runtime.rs")
            .split_once("\n#[cfg(test)]")
            .unwrap()
            .0;
        let key = transport
            .split_once("struct DynamicTextureKey {")
            .unwrap()
            .1
            .split_once('}')
            .unwrap()
            .0;

        assert_eq!(
            transport.matches("struct ProfileTextureSetCache").count(),
            1
        );
        assert!(!runtime.contains("struct ProfileTextureSetCache"));
        assert!(!runtime.contains("struct ProfileSkinCache"));
        assert!(!callers.contains("= ProfileTextureSetCache::default()"));
        assert!(!callers.contains("pub fn profile_texture_sets"));
        assert!(!key.contains("resource_generation"));
        assert!(!key.contains("session_generation"));
    }

    fn remote_profile(
        url: &str,
        cape_url: Option<&str>,
        elytra_url: Option<&str>,
        model: PlayerModelTypeSummary,
    ) -> ResolvableProfileSummary {
        ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: vec![GameProfilePropertySummary {
                name: "textures".to_string(),
                value: format!("packed:{url}"),
                signature: None,
            }],
            profile_textures: Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: url.to_string(),
                    model,
                }),
                cape: cape_url.map(|url| ProfileTextureSummary {
                    url: url.to_string(),
                }),
                elytra: elytra_url.map(|url| ProfileTextureSummary {
                    url: url.to_string(),
                }),
            }),
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    }
}
