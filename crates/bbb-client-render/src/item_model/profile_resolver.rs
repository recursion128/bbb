use std::{borrow::Cow, collections::HashMap};

use bbb_protocol::{
    packets::ResolvableProfileSummary, ProfileResolutionKey, ProfileResolutionResult,
    ResolvedGameProfile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfileResolutionEntry {
    Pending,
    Resolved(ResolvedGameProfile),
    Failed,
}

#[derive(Debug, Default)]
pub(crate) struct AsyncProfileResolutionRuntime {
    entries: HashMap<ProfileResolutionKey, ProfileResolutionEntry>,
    outbound_requests: Vec<ProfileResolutionKey>,
}

impl AsyncProfileResolutionRuntime {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn resolve_or_queue<'a>(
        &mut self,
        profile: &'a ResolvableProfileSummary,
    ) -> Cow<'a, ResolvableProfileSummary> {
        let Some(key) = ProfileResolutionKey::from_profile(profile) else {
            return Cow::Borrowed(profile);
        };

        match self.entries.get(&key) {
            Some(ProfileResolutionEntry::Resolved(resolved)) => {
                return Cow::Owned(resolved.clone().into_resolvable(profile.skin_patch.clone()));
            }
            Some(ProfileResolutionEntry::Pending | ProfileResolutionEntry::Failed) => {
                return Cow::Borrowed(profile);
            }
            None => {}
        }

        self.entries
            .insert(key.clone(), ProfileResolutionEntry::Pending);
        self.outbound_requests.push(key);
        Cow::Borrowed(profile)
    }

    pub(crate) fn take_pending_requests(&mut self) -> Vec<ProfileResolutionKey> {
        std::mem::take(&mut self.outbound_requests)
    }

    pub(crate) fn apply_results(
        &mut self,
        results: impl IntoIterator<Item = ProfileResolutionResult>,
    ) -> usize {
        let mut applied = 0usize;
        for result in results {
            applied += 1;
            let entry = result
                .resolved
                .map(ProfileResolutionEntry::Resolved)
                .unwrap_or(ProfileResolutionEntry::Failed);
            self.entries.insert(result.key, entry);
        }
        applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        GameProfilePropertySummary, PlayerModelTypeSummary, PlayerSkinPatchSummary,
        ProfileSkinTextureSummary, ProfileTexturesSummary, ResolvableProfileKindSummary,
    };
    use uuid::Uuid;

    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

    #[test]
    fn profile_resolution_runtime_queues_partial_name_and_applies_resolved_profile() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let mut runtime = AsyncProfileResolutionRuntime::new();
        let partial = partial_name_profile("Alex");

        assert_eq!(runtime.resolve_or_queue(&partial).as_ref(), &partial);
        assert_eq!(runtime.resolve_or_queue(&partial).as_ref(), &partial);
        assert_eq!(
            runtime.take_pending_requests(),
            vec![ProfileResolutionKey::Name("Alex".to_string())]
        );

        assert_eq!(
            runtime.apply_results([ProfileResolutionResult {
                key: ProfileResolutionKey::Name("Alex".to_string()),
                resolved: Some(ResolvedGameProfile {
                    uuid,
                    name: "Alex".into(),
                    properties: vec![textures_property()],
                }),
            }]),
            1
        );

        let resolved = runtime.resolve_or_queue(&partial);
        let resolved_again = runtime.resolve_or_queue(&partial);
        assert_eq!(resolved.kind, ResolvableProfileKindSummary::GameProfile);
        assert_eq!(resolved.uuid, Some(uuid));
        assert_eq!(resolved.name.as_deref(), Some("Alex"));
        assert_eq!(resolved.properties, vec![textures_property()]);
        assert_eq!(
            resolved.profile_textures,
            Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: "https://textures.minecraft.net/texture/skinhash".to_string(),
                    model: PlayerModelTypeSummary::Slim,
                }),
                cape: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/capehash".to_string(),
                }),
                elytra: Some(bbb_protocol::packets::ProfileTextureSummary {
                    url: "https://textures.minecraft.net/texture/elytrahash".to_string(),
                }),
            })
        );
        assert_eq!(resolved_again, resolved);
        assert!(runtime.take_pending_requests().is_empty());
    }

    #[test]
    fn profile_resolution_runtime_resolves_partial_uuid_and_preserves_skin_patch() {
        let uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        let mut runtime = AsyncProfileResolutionRuntime::new();
        let mut profile = partial_uuid_profile(uuid);
        profile.skin_patch.model = Some(PlayerModelTypeSummary::Wide);

        assert_eq!(runtime.resolve_or_queue(&profile).as_ref(), &profile);
        assert_eq!(
            runtime.take_pending_requests(),
            vec![ProfileResolutionKey::Uuid(uuid)]
        );
        runtime.apply_results([ProfileResolutionResult {
            key: ProfileResolutionKey::Uuid(uuid),
            resolved: Some(ResolvedGameProfile {
                uuid,
                name: "Steve".into(),
                properties: vec![textures_property()],
            }),
        }]);

        let resolved = runtime.resolve_or_queue(&profile);
        assert_eq!(resolved.kind, ResolvableProfileKindSummary::GameProfile);
        assert_eq!(resolved.uuid, Some(uuid));
        assert_eq!(resolved.name.as_deref(), Some("Steve"));
        assert_eq!(
            resolved.skin_patch.model,
            Some(PlayerModelTypeSummary::Wide)
        );
        assert_eq!(
            resolved
                .profile_textures
                .as_ref()
                .unwrap()
                .skin
                .as_ref()
                .unwrap()
                .model,
            PlayerModelTypeSummary::Slim
        );
    }

    #[test]
    fn profile_resolution_runtime_leaves_static_profiles_unchanged_and_caches_misses() {
        let uuid = Uuid::from_u128(7);
        let static_profiles = [
            ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::GameProfile,
                uuid: Some(uuid),
                name: Some("Alex".to_string()),
                properties: vec![textures_property()],
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            },
            ResolvableProfileSummary {
                properties: vec![textures_property()],
                ..partial_name_profile("Alex")
            },
            ResolvableProfileSummary {
                uuid: Some(uuid),
                ..partial_name_profile("Alex")
            },
            partial_name_profile("abcdefghijklmnopq"),
            partial_name_profile("Bad\nName"),
        ];
        let mut runtime = AsyncProfileResolutionRuntime::new();
        for profile in &static_profiles {
            assert_eq!(runtime.resolve_or_queue(profile).as_ref(), profile);
        }
        assert!(runtime.take_pending_requests().is_empty());

        let missing = partial_name_profile("Missing");
        assert_eq!(runtime.resolve_or_queue(&missing).as_ref(), &missing);
        assert_eq!(
            runtime.take_pending_requests(),
            vec![ProfileResolutionKey::Name("Missing".to_string())]
        );
        runtime.apply_results([ProfileResolutionResult {
            key: ProfileResolutionKey::Name("Missing".to_string()),
            resolved: None,
        }]);
        assert_eq!(runtime.resolve_or_queue(&missing).as_ref(), &missing);
        assert!(runtime.take_pending_requests().is_empty());
    }

    fn partial_name_profile(name: &str) -> ResolvableProfileSummary {
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some(name.to_string()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    }

    fn partial_uuid_profile(uuid: Uuid) -> ResolvableProfileSummary {
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: Some(uuid),
            name: None,
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    }

    fn textures_property() -> GameProfilePropertySummary {
        GameProfilePropertySummary {
            name: "textures".into(),
            value: SLIM_TEXTURES_PROPERTY.to_string().into(),
            signature: Some("signature".into()),
        }
    }
}
