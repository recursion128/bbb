use bbb_protocol::packets::{PlayerModelTypeSummary, ResolvableProfileSummary};
use bbb_render_types::{
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityPlayerSkin, EntityPlayerSkinModel,
};

use crate::item_model::skin_runtime::{
    default_player_skin_for_profile, ProfileTextureAssetKind, ProfileTextureSetCache,
    MAX_PROFILE_TEXTURE_ASSET_ENTRIES, MAX_PROFILE_TEXTURE_PROPERTY_BYTES,
    MAX_PROFILE_TEXTURE_SET_ENTRIES, PROFILE_TEXTURE_SET_CACHE_DURATION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        GameProfilePropertySummary, PlayerSkinPatchSummary, ProfileSkinTextureSummary,
        ProfileTextureSummary, ProfileTexturesSummary,
    };
    use std::time::{Duration, Instant};

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
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 3);

        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 11);
        cache.mark_resolved(ProfileTextureAssetKind::Cape, cape_url, 12);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
        assert_eq!(
            cache.player_texture_for_profile(
                &profile,
                ProfileTextureAssetKind::Cape,
                false,
                || false,
            ),
            None
        );
        cache.mark_resolved(ProfileTextureAssetKind::Elytra, elytra_url, 13);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, || false),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: 11,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Slim,
                status: EntityDynamicPlayerSkinStatus::Ready,
            })
        );
        assert_eq!(
            cache.player_texture_for_profile(
                &profile,
                ProfileTextureAssetKind::Cape,
                false,
                || false,
            ),
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
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 9001);
        cache.mark_failed(ProfileTextureAssetKind::Cape, cape_url);
        assert_eq!(
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
        assert_eq!(
            cache.player_texture_for_profile(
                &profile,
                ProfileTextureAssetKind::Cape,
                false,
                || false,
            ),
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
            cache.player_skin_for_profile(&model_patched, false, || false),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 44);
        assert_eq!(
            cache.player_skin_for_profile(&model_patched, false, || false),
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

        assert_eq!(
            cache.player_skin_for_profile(&profile, true, || false),
            fallback
        );
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 55);
        assert_eq!(
            cache.player_skin_for_profile(&profile, true, || false),
            fallback
        );
        assert!(matches!(
            cache.player_skin_for_profile(&profile, false, || false),
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
        let skin_url = "https://textures.minecraft.net/texture/trust-upgrade";
        let profile = remote_profile(skin_url, None, None, PlayerModelTypeSummary::Slim);
        let mut cache = ProfileTextureSetCache::default();

        cache.player_skin_for_profile(&profile, false, || false);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 56);
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, || false),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, || true),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 56, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&profile, true, || false),
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

        cache.player_skin_for_profile_at(&profile, false, || false, started);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, skin_url, 77);
        cache.player_skin_for_profile_at(
            &profile,
            false,
            || false,
            started + PROFILE_TEXTURE_SET_CACHE_DURATION - Duration::from_millis(1),
        );
        assert_eq!(cache.set_count(), 1);
        let later = started + PROFILE_TEXTURE_SET_CACHE_DURATION * 2;
        assert!(matches!(
            cache.player_skin_for_profile_at(&profile, false, || false, later),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 77, .. })
        ));
        assert_eq!(cache.set_count(), 1);
        assert_eq!(cache.asset_count(), 1);
    }

    #[test]
    fn profile_texture_set_rejects_oversized_property_before_retention_or_download() {
        let mut profile = remote_profile(
            "https://textures.minecraft.net/texture/oversized-property",
            None,
            None,
            PlayerModelTypeSummary::Slim,
        );
        profile.properties[0].value = "x"
            .repeat(MAX_PROFILE_TEXTURE_PROPERTY_BYTES / 2 + 1)
            .into();
        let fallback = EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(&profile));
        let mut cache = ProfileTextureSetCache::default();

        assert_eq!(
            cache.player_skin_for_profile(&profile, false, || false),
            fallback
        );
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
            cache.player_skin_for_profile(&profile, false, || false);
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

        cache.player_skin_for_profile(&first, false, || false);
        cache.player_skin_for_profile(&second, false, || false);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, first_url, 91);

        assert!(matches!(
            cache.player_skin_for_profile(&first, false, || false),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 91, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&second, false, || false),
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

        cache.player_skin_for_profile(&first, false, || false);
        cache.player_skin_for_profile(&second, false, || false);
        cache.mark_resolved(ProfileTextureAssetKind::Skin, first_url, 92);

        assert!(matches!(
            cache.player_skin_for_profile(&first, false, || false),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin { handle: 92, .. })
        ));
        assert!(matches!(
            cache.player_skin_for_profile(&second, false, || false),
            EntityPlayerSkin::ProfiledDefault(_)
        ));
        assert_eq!(cache.asset_count(), 1);
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
                name: "textures".into(),
                value: format!("packed:{url}").into(),
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
