use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use bbb_protocol::packets::{
    decode_profile_textures_from_properties, GameProfilePropertySummary, PlayerSkinPatchSummary,
    ResolvableProfileKindSummary, ResolvableProfileSummary,
};
use serde::Deserialize;
use uuid::Uuid;

pub(crate) trait GameProfileFetcher {
    fn fetch_id_by_name(&mut self, name: &str) -> Result<Option<NameAndId>>;

    fn fetch_profile_by_id(&mut self, id: Uuid) -> Result<Option<ResolvedGameProfile>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NameAndId {
    pub(crate) uuid: Uuid,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedGameProfile {
    pub(crate) uuid: Uuid,
    pub(crate) name: String,
    pub(crate) properties: Vec<GameProfilePropertySummary>,
}

impl ResolvedGameProfile {
    pub(crate) fn into_resolvable(
        self,
        skin_patch: PlayerSkinPatchSummary,
    ) -> ResolvableProfileSummary {
        let profile_textures = decode_profile_textures_from_properties(
            self.properties
                .iter()
                .map(|property| (property.name.as_str(), property.value.as_str())),
        );
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(self.uuid),
            name: Some(self.name),
            properties: self.properties,
            profile_textures,
            skin_patch,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvableProfileResolver<F> {
    fetcher: F,
    profile_cache_by_name: HashMap<String, Option<Uuid>>,
    profile_cache_by_id: HashMap<Uuid, Option<ResolvedGameProfile>>,
}

impl<F> ResolvableProfileResolver<F> {
    pub(crate) fn new(fetcher: F) -> Self {
        Self {
            fetcher,
            profile_cache_by_name: HashMap::new(),
            profile_cache_by_id: HashMap::new(),
        }
    }
}

impl<F: GameProfileFetcher> ResolvableProfileResolver<F> {
    #[cfg(test)]
    pub(crate) fn resolve_or_partial(
        &mut self,
        profile: ResolvableProfileSummary,
    ) -> ResolvableProfileSummary {
        if !is_dynamic_resolvable_profile(&profile) {
            return profile;
        }

        let resolved = match (profile.name.as_deref(), profile.uuid) {
            (Some(name), None) => self.resolve_by_name(name),
            (None, Some(uuid)) => self.resolve_by_id(uuid),
            _ => Ok(None),
        };
        match resolved {
            Ok(Some(resolved)) => resolved.into_resolvable(profile.skin_patch),
            Ok(None) | Err(_) => profile,
        }
    }

    pub(crate) fn resolve_key(
        &mut self,
        key: &ProfileResolutionKey,
    ) -> Result<Option<ResolvedGameProfile>> {
        match key {
            ProfileResolutionKey::Name(name) => self.resolve_by_name(name),
            ProfileResolutionKey::Uuid(uuid) => self.resolve_by_id(*uuid),
        }
    }

    fn resolve_by_name(&mut self, name: &str) -> Result<Option<ResolvedGameProfile>> {
        if !valid_player_name(name) {
            return Ok(None);
        }

        if let Some(cached) = self.profile_cache_by_name.get(name).cloned() {
            return self.resolve_optional_id(cached);
        }

        let fetched = self.fetcher.fetch_id_by_name(name)?;
        let uuid = fetched.as_ref().map(|name_and_id| name_and_id.uuid);
        self.profile_cache_by_name.insert(name.to_string(), uuid);
        if let Some(name_and_id) = fetched {
            self.profile_cache_by_name
                .insert(name_and_id.name, Some(name_and_id.uuid));
        }
        self.resolve_optional_id(uuid)
    }

    fn resolve_optional_id(&mut self, id: Option<Uuid>) -> Result<Option<ResolvedGameProfile>> {
        match id {
            Some(id) => self.resolve_by_id(id),
            None => Ok(None),
        }
    }

    fn resolve_by_id(&mut self, id: Uuid) -> Result<Option<ResolvedGameProfile>> {
        if !self.profile_cache_by_id.contains_key(&id) {
            let fetched = self.fetcher.fetch_profile_by_id(id)?;
            self.profile_cache_by_id.insert(id, fetched);
        }
        Ok(self.profile_cache_by_id.get(&id).cloned().flatten())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ProfileResolutionKey {
    Name(String),
    Uuid(Uuid),
}

impl ProfileResolutionKey {
    fn from_profile(profile: &ResolvableProfileSummary) -> Option<Self> {
        if !is_dynamic_resolvable_profile(profile) || profile.profile_textures.is_some() {
            return None;
        }

        match (profile.name.as_deref(), profile.uuid) {
            (Some(name), None) if valid_player_name(name) => Some(Self::Name(name.to_string())),
            (None, Some(uuid)) => Some(Self::Uuid(uuid)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct AsyncProfileResolutionRuntime {
    entries: HashMap<ProfileResolutionKey, ProfileResolutionEntry>,
    request_tx: Sender<ProfileResolutionKey>,
    result_rx: Receiver<ProfileResolutionResult>,
}

impl AsyncProfileResolutionRuntime {
    pub(crate) fn new(fetcher: impl GameProfileFetcher + Send + 'static) -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut resolver = ResolvableProfileResolver::new(fetcher);
            for key in request_rx {
                let resolved = resolver.resolve_key(&key).ok().flatten();
                if result_tx
                    .send(ProfileResolutionResult { key, resolved })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            entries: HashMap::new(),
            request_tx,
            result_rx,
        }
    }

    pub(crate) fn resolve_or_queue(
        &mut self,
        profile: &ResolvableProfileSummary,
    ) -> ResolvableProfileSummary {
        self.drain_results();
        let Some(key) = ProfileResolutionKey::from_profile(profile) else {
            return profile.clone();
        };

        match self.entries.get(&key) {
            Some(ProfileResolutionEntry::Resolved(resolved)) => {
                return resolved.clone().into_resolvable(profile.skin_patch.clone());
            }
            Some(ProfileResolutionEntry::Pending | ProfileResolutionEntry::Failed) => {
                return profile.clone();
            }
            None => {}
        }

        self.entries
            .insert(key.clone(), ProfileResolutionEntry::Pending);
        if self.request_tx.send(key.clone()).is_err() {
            self.entries.insert(key, ProfileResolutionEntry::Failed);
        }
        profile.clone()
    }

    pub(crate) fn drain_results(&mut self) -> usize {
        let mut drained = 0usize;
        while let Ok(result) = self.result_rx.try_recv() {
            drained += 1;
            let entry = result
                .resolved
                .map(ProfileResolutionEntry::Resolved)
                .unwrap_or(ProfileResolutionEntry::Failed);
            self.entries.insert(result.key, entry);
        }
        drained
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfileResolutionEntry {
    Pending,
    Resolved(ResolvedGameProfile),
    Failed,
}

#[derive(Debug)]
struct ProfileResolutionResult {
    key: ProfileResolutionKey,
    resolved: Option<ResolvedGameProfile>,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpGameProfileFetcher {
    client: reqwest::blocking::Client,
    api_base_url: String,
    session_base_url: String,
}

impl HttpGameProfileFetcher {
    pub(crate) fn new() -> Result<Self> {
        Self::with_base_urls("https://api.mojang.com", "https://sessionserver.mojang.com")
    }

    fn with_base_urls(api_base_url: &str, session_base_url: &str) -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("bbb-native")
                .build()
                .context("create profile resolver HTTP client")?,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            session_base_url: session_base_url.trim_end_matches('/').to_string(),
        })
    }

    fn get_optional_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<Option<T>> {
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("request HTTP profile {url}"))?;
        let status = response.status();
        if status == reqwest::StatusCode::NO_CONTENT || status == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("fetch HTTP profile {url}"))?;
        let body = response
            .text()
            .with_context(|| format!("read HTTP profile {url}"))?;
        serde_json::from_str(&body)
            .with_context(|| format!("decode HTTP profile JSON from {url}"))
            .map(Some)
    }
}

impl GameProfileFetcher for HttpGameProfileFetcher {
    fn fetch_id_by_name(&mut self, name: &str) -> Result<Option<NameAndId>> {
        let url = format!("{}/users/profiles/minecraft/{name}", self.api_base_url);
        let Some(response) = self.get_optional_json::<NameToIdResponse>(&url)? else {
            return Ok(None);
        };
        let uuid = Uuid::parse_str(&response.id)
            .with_context(|| format!("parse profile id {}", response.id))?;
        Ok(Some(NameAndId {
            uuid,
            name: response.name,
        }))
    }

    fn fetch_profile_by_id(&mut self, id: Uuid) -> Result<Option<ResolvedGameProfile>> {
        let url = format!(
            "{}/session/minecraft/profile/{}?unsigned=false",
            self.session_base_url,
            id.as_simple()
        );
        let Some(response) = self.get_optional_json::<SessionProfileResponse>(&url)? else {
            return Ok(None);
        };
        let uuid = Uuid::parse_str(&response.id)
            .with_context(|| format!("parse session profile id {}", response.id))?;
        Ok(Some(ResolvedGameProfile {
            uuid,
            name: response.name,
            properties: response
                .properties
                .into_iter()
                .map(|property| GameProfilePropertySummary {
                    name: property.name,
                    value: property.value,
                    signature: property.signature,
                })
                .collect(),
        }))
    }
}

#[derive(Deserialize)]
struct NameToIdResponse {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct SessionProfileResponse {
    id: String,
    name: String,
    #[serde(default)]
    properties: Vec<SessionProfileProperty>,
}

#[derive(Deserialize)]
struct SessionProfileProperty {
    name: String,
    value: String,
    signature: Option<String>,
}

fn is_dynamic_resolvable_profile(profile: &ResolvableProfileSummary) -> bool {
    profile.kind == ResolvableProfileKindSummary::Partial
        && profile.properties.is_empty()
        && profile.name.is_some() != profile.uuid.is_some()
}

fn valid_player_name(name: &str) -> bool {
    name.len() <= 16 && name.chars().all(|ch| (ch as u32) > 32 && (ch as u32) < 127)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        PlayerModelTypeSummary, ProfileSkinTextureSummary, ProfileTexturesSummary,
    };
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

    #[test]
    fn resolver_resolves_partial_name_and_caches_name_and_id() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let mut fetcher = FakeGameProfileFetcher::default();
        fetcher.name_results.insert(
            "Alex".to_string(),
            Some(NameAndId {
                uuid,
                name: "Alex".to_string(),
            }),
        );
        fetcher.id_results.insert(
            uuid,
            Some(ResolvedGameProfile {
                uuid,
                name: "Alex".to_string(),
                properties: vec![textures_property()],
            }),
        );
        let mut resolver = ResolvableProfileResolver::new(fetcher);

        let first = resolver.resolve_or_partial(partial_name_profile("Alex"));
        let second = resolver.resolve_or_partial(partial_name_profile("Alex"));

        assert_eq!(first.kind, ResolvableProfileKindSummary::GameProfile);
        assert_eq!(first.uuid, Some(uuid));
        assert_eq!(first.name.as_deref(), Some("Alex"));
        assert_eq!(first.properties, vec![textures_property()]);
        assert_eq!(
            first.profile_textures,
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
        assert_eq!(second, first);
        assert_eq!(resolver.fetcher.name_calls, 1);
        assert_eq!(resolver.fetcher.id_calls, 1);
    }

    #[test]
    fn resolver_resolves_partial_uuid_without_name_lookup_and_preserves_patch() {
        let uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        let mut fetcher = FakeGameProfileFetcher::default();
        fetcher.id_results.insert(
            uuid,
            Some(ResolvedGameProfile {
                uuid,
                name: "Steve".to_string(),
                properties: vec![textures_property()],
            }),
        );
        let mut resolver = ResolvableProfileResolver::new(fetcher);
        let mut profile = partial_uuid_profile(uuid);
        profile.skin_patch.model = Some(PlayerModelTypeSummary::Wide);

        let resolved = resolver.resolve_or_partial(profile);

        assert_eq!(resolved.kind, ResolvableProfileKindSummary::GameProfile);
        assert_eq!(resolved.uuid, Some(uuid));
        assert_eq!(resolved.name.as_deref(), Some("Steve"));
        assert_eq!(
            resolved.skin_patch.model,
            Some(PlayerModelTypeSummary::Wide)
        );
        assert_eq!(
            resolved.profile_textures.unwrap().skin.unwrap().model,
            PlayerModelTypeSummary::Slim
        );
        assert_eq!(resolver.fetcher.name_calls, 0);
        assert_eq!(resolver.fetcher.id_calls, 1);
    }

    #[test]
    fn resolver_leaves_static_or_non_dynamic_profiles_unchanged() {
        let uuid = Uuid::from_u128(7);
        let profiles = [
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
        ];
        let mut resolver = ResolvableProfileResolver::new(FakeGameProfileFetcher::default());

        for profile in profiles {
            let resolved = resolver.resolve_or_partial(profile.clone());
            assert_eq!(resolved, profile);
        }
        assert_eq!(resolver.fetcher.name_calls, 0);
        assert_eq!(resolver.fetcher.id_calls, 0);
    }

    #[test]
    fn resolver_returns_partial_on_invalid_name_or_miss() {
        let mut fetcher = FakeGameProfileFetcher::default();
        fetcher.name_results.insert("Missing".to_string(), None);
        let mut resolver = ResolvableProfileResolver::new(fetcher);

        let too_long = partial_name_profile("abcdefghijklmnopq");
        let with_control = partial_name_profile("Bad\nName");
        let missing = partial_name_profile("Missing");

        assert_eq!(resolver.resolve_or_partial(too_long.clone()), too_long);
        assert_eq!(
            resolver.resolve_or_partial(with_control.clone()),
            with_control
        );
        assert_eq!(resolver.resolve_or_partial(missing.clone()), missing);
        assert_eq!(resolver.resolve_or_partial(missing.clone()), missing);
        assert_eq!(resolver.fetcher.name_calls, 1);
        assert_eq!(resolver.fetcher.id_calls, 0);
    }

    #[test]
    fn async_profile_resolution_runtime_returns_fallback_then_resolved_profile() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let counters = AsyncFetchCounters::default();
        let mut runtime = AsyncProfileResolutionRuntime::new(AsyncFakeGameProfileFetcher {
            uuid,
            name: "Alex".to_string(),
            profile: Some(ResolvedGameProfile {
                uuid,
                name: "Alex".to_string(),
                properties: vec![textures_property()],
            }),
            counters: counters.clone(),
        });
        let partial = partial_name_profile("Alex");

        assert_eq!(runtime.resolve_or_queue(&partial), partial);
        drain_until_profile_result(&mut runtime);
        let resolved = runtime.resolve_or_queue(&partial);
        let resolved_again = runtime.resolve_or_queue(&partial);

        assert_eq!(resolved.kind, ResolvableProfileKindSummary::GameProfile);
        assert_eq!(resolved.uuid, Some(uuid));
        assert_eq!(resolved.name.as_deref(), Some("Alex"));
        assert_eq!(resolved.properties, vec![textures_property()]);
        assert_eq!(
            resolved
                .profile_textures
                .as_ref()
                .unwrap()
                .skin
                .as_ref()
                .unwrap()
                .url,
            "https://textures.minecraft.net/texture/skinhash"
        );
        assert_eq!(resolved_again, resolved);
        assert_eq!(counters.name_calls.load(Ordering::Relaxed), 1);
        assert_eq!(counters.id_calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn async_profile_resolution_runtime_caches_misses_and_ignores_invalid_names() {
        let counters = AsyncFetchCounters::default();
        let mut runtime = AsyncProfileResolutionRuntime::new(AsyncFakeGameProfileFetcher {
            uuid: Uuid::from_u128(1),
            name: "Alex".to_string(),
            profile: None,
            counters: counters.clone(),
        });
        let missing = partial_name_profile("Missing");
        let invalid = partial_name_profile("Bad\nName");

        assert_eq!(runtime.resolve_or_queue(&invalid), invalid);
        assert_eq!(runtime.resolve_or_queue(&missing), missing);
        drain_until_profile_result(&mut runtime);
        assert_eq!(runtime.resolve_or_queue(&missing), missing);

        assert_eq!(counters.name_calls.load(Ordering::Relaxed), 1);
        assert_eq!(counters.id_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn http_profile_fetcher_parses_loopback_name_and_session_profile() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let base_url = spawn_http_routes(vec![
            HttpRoute {
                path: "/users/profiles/minecraft/Alex".to_string(),
                status: 200,
                reason: "OK".to_string(),
                body: format!(r#"{{"id":"{}","name":"Alex"}}"#, uuid.as_simple()),
            },
            HttpRoute {
                path: format!(
                    "/session/minecraft/profile/{}?unsigned=false",
                    uuid.as_simple()
                ),
                status: 200,
                reason: "OK".to_string(),
                body: format!(
                    r#"{{"id":"{}","name":"Alex","properties":[{{"name":"textures","value":"{}","signature":"sig"}}]}}"#,
                    uuid.as_simple(),
                    SLIM_TEXTURES_PROPERTY
                ),
            },
        ]);
        let mut fetcher = HttpGameProfileFetcher::with_base_urls(&base_url, &base_url).unwrap();

        let name_and_id = fetcher.fetch_id_by_name("Alex").unwrap().unwrap();
        let profile = fetcher.fetch_profile_by_id(uuid).unwrap().unwrap();

        assert_eq!(
            name_and_id,
            NameAndId {
                uuid,
                name: "Alex".to_string(),
            }
        );
        assert_eq!(profile.uuid, uuid);
        assert_eq!(profile.name, "Alex");
        assert_eq!(
            profile.properties,
            vec![GameProfilePropertySummary {
                name: "textures".to_string(),
                value: SLIM_TEXTURES_PROPERTY.to_string(),
                signature: Some("sig".to_string()),
            }]
        );
    }

    #[test]
    fn http_profile_fetcher_builds_default_client() {
        let _fetcher = HttpGameProfileFetcher::new().unwrap();
    }

    #[test]
    fn http_profile_fetcher_maps_no_content_and_not_found_to_miss() {
        let uuid = Uuid::from_u128(0x01234567_89ab_cdef_0123_456789abcdef);
        let base_url = spawn_http_routes(vec![
            HttpRoute {
                path: "/users/profiles/minecraft/Missing".to_string(),
                status: 204,
                reason: "No Content".to_string(),
                body: String::new(),
            },
            HttpRoute {
                path: format!(
                    "/session/minecraft/profile/{}?unsigned=false",
                    uuid.as_simple()
                ),
                status: 404,
                reason: "Not Found".to_string(),
                body: "missing".to_string(),
            },
        ]);
        let mut fetcher = HttpGameProfileFetcher::with_base_urls(&base_url, &base_url).unwrap();

        assert!(fetcher.fetch_id_by_name("Missing").unwrap().is_none());
        assert!(fetcher.fetch_profile_by_id(uuid).unwrap().is_none());
    }

    #[derive(Default)]
    struct FakeGameProfileFetcher {
        name_results: HashMap<String, Option<NameAndId>>,
        id_results: HashMap<Uuid, Option<ResolvedGameProfile>>,
        name_calls: usize,
        id_calls: usize,
    }

    impl GameProfileFetcher for FakeGameProfileFetcher {
        fn fetch_id_by_name(&mut self, name: &str) -> Result<Option<NameAndId>> {
            self.name_calls += 1;
            Ok(self.name_results.get(name).cloned().flatten())
        }

        fn fetch_profile_by_id(&mut self, id: Uuid) -> Result<Option<ResolvedGameProfile>> {
            self.id_calls += 1;
            Ok(self.id_results.get(&id).cloned().flatten())
        }
    }

    #[derive(Clone, Default)]
    struct AsyncFetchCounters {
        name_calls: Arc<AtomicUsize>,
        id_calls: Arc<AtomicUsize>,
    }

    struct AsyncFakeGameProfileFetcher {
        uuid: Uuid,
        name: String,
        profile: Option<ResolvedGameProfile>,
        counters: AsyncFetchCounters,
    }

    impl GameProfileFetcher for AsyncFakeGameProfileFetcher {
        fn fetch_id_by_name(&mut self, name: &str) -> Result<Option<NameAndId>> {
            self.counters.name_calls.fetch_add(1, Ordering::Relaxed);
            if name == self.name {
                Ok(Some(NameAndId {
                    uuid: self.uuid,
                    name: self.name.clone(),
                }))
            } else {
                Ok(None)
            }
        }

        fn fetch_profile_by_id(&mut self, id: Uuid) -> Result<Option<ResolvedGameProfile>> {
            self.counters.id_calls.fetch_add(1, Ordering::Relaxed);
            if id == self.uuid {
                Ok(self.profile.clone())
            } else {
                Ok(None)
            }
        }
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
            name: "textures".to_string(),
            value: SLIM_TEXTURES_PROPERTY.to_string(),
            signature: Some("signature".to_string()),
        }
    }

    fn drain_until_profile_result(runtime: &mut AsyncProfileResolutionRuntime) {
        for _ in 0..100 {
            if runtime.drain_results() > 0 {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for profile resolution result");
    }

    #[derive(Clone)]
    struct HttpRoute {
        path: String,
        status: u16,
        reason: String,
        body: String,
    }

    fn spawn_http_routes(routes: Vec<HttpRoute>) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for _ in 0..routes.len() {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 2048];
                let len = stream.read(&mut request).unwrap_or(0);
                let path = http_request_path(&request[..len]).unwrap_or("/");
                let route = routes
                    .iter()
                    .find(|route| route.path == path)
                    .cloned()
                    .unwrap_or_else(|| HttpRoute {
                        path: path.to_string(),
                        status: 404,
                        reason: "Not Found".to_string(),
                        body: String::new(),
                    });
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    route.status,
                    route.reason,
                    route.body.len(),
                    route.body
                )
                .unwrap();
            }
        });
        format!("http://{addr}")
    }

    fn http_request_path(request: &[u8]) -> Option<&str> {
        let request = std::str::from_utf8(request).ok()?;
        let first_line = request.lines().next()?;
        first_line.split_whitespace().nth(1)
    }
}
