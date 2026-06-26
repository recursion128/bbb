use std::collections::HashMap;

use bbb_protocol::packets::{PlayerModelTypeSummary, ResolvableProfileSummary};
use bbb_renderer::{
    EntityDefaultPlayerSkin, EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityPlayerSkin, EntityPlayerSkinModel,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileSkinCache {
    entries: HashMap<String, ProfileSkinCacheEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileSkinCacheEntry {
    pub(crate) request_handle: u64,
    pub(crate) resolved_handle: Option<u64>,
    pub(crate) status: EntityDynamicPlayerSkinStatus,
}

impl ProfileSkinCache {
    pub(crate) fn player_skin_for_profile(
        &mut self,
        profile: &ResolvableProfileSummary,
    ) -> EntityPlayerSkin {
        let fallback = profile_default_player_skin(profile);
        if profile.skin_patch.body.is_some() {
            return EntityPlayerSkin::ProfiledDefault(fallback);
        }

        let Some(skin) = profile
            .profile_textures
            .as_ref()
            .and_then(|textures| textures.skin.as_ref())
        else {
            return EntityPlayerSkin::ProfiledDefault(fallback);
        };

        let model = profile
            .skin_patch
            .model
            .map(entity_player_skin_model)
            .unwrap_or_else(|| entity_player_skin_model(skin.model));
        let entry = self.entry_or_loading(&skin.url);
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: entry.resolved_handle.unwrap_or(entry.request_handle),
            fallback,
            model,
            status: entry.status,
        })
    }

    pub(crate) fn mark_resolved(&mut self, url: &str, texture_handle: u64) {
        let entry = self.entry_or_loading(url);
        entry.resolved_handle = Some(texture_handle);
        entry.status = EntityDynamicPlayerSkinStatus::Ready;
    }

    pub(crate) fn mark_failed(&mut self, url: &str) {
        let entry = self.entry_or_loading(url);
        entry.resolved_handle = None;
        entry.status = EntityDynamicPlayerSkinStatus::Failed;
    }

    #[cfg(test)]
    fn entry(&self, url: &str) -> Option<&ProfileSkinCacheEntry> {
        self.entries.get(url)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry_or_loading(&mut self, url: &str) -> &mut ProfileSkinCacheEntry {
        self.entries
            .entry(url.to_string())
            .or_insert_with(|| ProfileSkinCacheEntry {
                request_handle: profile_texture_handle(url),
                resolved_handle: None,
                status: EntityDynamicPlayerSkinStatus::Loading,
            })
    }
}

fn profile_default_player_skin(profile: &ResolvableProfileSummary) -> EntityDefaultPlayerSkin {
    if let Some(body) = profile.skin_patch.body.as_ref() {
        if let Some(skin) = EntityDefaultPlayerSkin::from_texture_path(&body.texture_path) {
            return skin;
        }
    }

    let profile_id = profile
        .uuid
        .map(|uuid| uuid.as_u128())
        .or_else(|| {
            profile
                .name
                .as_deref()
                .map(|name| bbb_protocol::codec::offline_player_uuid(name).as_u128())
        })
        .unwrap_or(0);
    default_player_skin_for_profile_id(profile_id)
}

pub(crate) fn default_player_skin_for_profile_id(profile_id: u128) -> EntityDefaultPlayerSkin {
    EntityDefaultPlayerSkin::from_vanilla_index(default_player_skin_index(profile_id))
}

fn entity_player_skin_model(model: PlayerModelTypeSummary) -> EntityPlayerSkinModel {
    match model {
        PlayerModelTypeSummary::Slim => EntityPlayerSkinModel::Slim,
        PlayerModelTypeSummary::Wide => EntityPlayerSkinModel::Wide,
    }
}

pub(crate) fn profile_texture_handle(url: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in url.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn default_player_skin_index(profile_id: u128) -> usize {
    java_uuid_hash_code(profile_id).rem_euclid(18) as usize
}

fn java_uuid_hash_code(profile_id: u128) -> i32 {
    let most = (profile_id >> 64) as u64;
    let least = profile_id as u64;
    ((most >> 32) as i32) ^ (most as i32) ^ ((least >> 32) as i32) ^ (least as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        PlayerSkinPatchSummary, ProfileSkinTextureSummary, ProfileTexturesSummary,
        ResourceTextureSummary,
    };

    #[test]
    fn profile_skin_cache_registers_loading_remote_skin_once() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let profile = remote_profile(skin_url, PlayerModelTypeSummary::Slim);
        let mut cache = ProfileSkinCache::default();

        let first = cache.player_skin_for_profile(&profile);
        let second = cache.player_skin_for_profile(&profile);

        let expected = EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle: profile_texture_handle(skin_url),
            fallback: EntityDefaultPlayerSkin::SlimAlex,
            model: EntityPlayerSkinModel::Slim,
            status: EntityDynamicPlayerSkinStatus::Loading,
        });
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.entry(skin_url).unwrap(),
            &ProfileSkinCacheEntry {
                request_handle: profile_texture_handle(skin_url),
                resolved_handle: None,
                status: EntityDynamicPlayerSkinStatus::Loading,
            }
        );
    }

    #[test]
    fn profile_skin_cache_uses_resolved_texture_handle_and_failure_state() {
        let ready_url = "https://textures.minecraft.net/texture/ready-skin";
        let failed_url = "https://textures.minecraft.net/texture/failed-skin";
        let mut cache = ProfileSkinCache::default();

        cache.mark_resolved(ready_url, 9001);
        cache.mark_failed(failed_url);

        assert_eq!(
            cache.player_skin_for_profile(&remote_profile(ready_url, PlayerModelTypeSummary::Wide)),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: 9001,
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Ready,
            })
        );
        assert_eq!(
            cache
                .player_skin_for_profile(&remote_profile(failed_url, PlayerModelTypeSummary::Slim)),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: profile_texture_handle(failed_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Slim,
                status: EntityDynamicPlayerSkinStatus::Failed,
            })
        );
    }

    #[test]
    fn profile_skin_cache_honors_patch_body_and_model_override() {
        let skin_url = "https://textures.minecraft.net/texture/profile-skin";
        let mut cache = ProfileSkinCache::default();
        let mut patched = remote_profile(skin_url, PlayerModelTypeSummary::Slim);
        patched.skin_patch.body = Some(ResourceTextureSummary {
            asset_id: "minecraft:entity/player/wide/steve".to_string(),
            texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
        });
        assert_eq!(
            cache.player_skin_for_profile(&patched),
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve)
        );
        assert_eq!(cache.len(), 0);

        let mut model_patched = remote_profile(skin_url, PlayerModelTypeSummary::Slim);
        model_patched.skin_patch.model = Some(PlayerModelTypeSummary::Wide);
        assert_eq!(
            cache.player_skin_for_profile(&model_patched),
            EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                handle: profile_texture_handle(skin_url),
                fallback: EntityDefaultPlayerSkin::SlimAlex,
                model: EntityPlayerSkinModel::Wide,
                status: EntityDynamicPlayerSkinStatus::Loading,
            })
        );
    }

    fn remote_profile(url: &str, model: PlayerModelTypeSummary) -> ResolvableProfileSummary {
        ResolvableProfileSummary {
            kind: bbb_protocol::packets::ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: None,
            properties: Vec::new(),
            profile_textures: Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: url.to_string(),
                    model,
                }),
                cape: None,
                elytra: None,
            }),
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    }
}
