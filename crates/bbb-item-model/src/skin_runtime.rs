use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bbb_protocol::packets::{
    GameProfilePropertySummary, PlayerModelTypeSummary, ProfileTexturesSummary,
    ResolvableProfileSummary,
};
use bbb_render_types::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityPlayerSkin, EntityPlayerSkinModel,
    MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES, MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
};
use rsa::{pkcs8::DecodePublicKey, Pkcs1v15Sign, RsaPublicKey};
use sha1::{Digest, Sha1};

const MAX_PROFILE_TEXTURE_SIGNATURE_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileTextureSignatureState {
    Signed,
    Unsigned,
    Invalid,
}

pub(crate) fn profile_texture_signature_state(
    properties: &[GameProfilePropertySummary],
    profile_property_keys: &[Vec<u8>],
) -> ProfileTextureSignatureState {
    let Some(property) = properties
        .iter()
        .find(|property| property.name == "textures")
    else {
        return ProfileTextureSignatureState::Unsigned;
    };
    let Some(encoded_signature) = property.signature.as_deref() else {
        return ProfileTextureSignatureState::Unsigned;
    };
    let Ok(signature) = STANDARD.decode(encoded_signature) else {
        return ProfileTextureSignatureState::Invalid;
    };
    if signature.len() > MAX_PROFILE_TEXTURE_SIGNATURE_BYTES {
        return ProfileTextureSignatureState::Invalid;
    }
    let digest = Sha1::digest(property.value.as_bytes());
    if profile_property_keys.iter().any(|der| {
        RsaPublicKey::from_public_key_der(der).is_ok_and(|key| {
            key.verify(Pkcs1v15Sign::new::<Sha1>(), &digest, &signature)
                .is_ok()
        })
    }) {
        ProfileTextureSignatureState::Signed
    } else {
        ProfileTextureSignatureState::Invalid
    }
}

/// Authlib `TextureUrlChecker` plus `MinecraftProfileTexture.getHash()`.
/// The checker accepts only HTTP(S) on the exact texture CDN host; the hash
/// is the final path basename with its final extension removed.
pub(crate) fn vanilla_profile_texture_hash(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str()? != "textures.minecraft.net"
    {
        return None;
    }
    let filename = parsed.path().rsplit('/').next().unwrap_or_default();
    let basename = filename
        .rfind('.')
        .map_or(filename, |extension| &filename[..extension]);
    Some(basename.to_string())
}

/// Guava `Hashing.sha1().hashUnencodedChars`: UTF-16 code units in the
/// primitive sink's little-endian order.
pub(crate) fn vanilla_profile_texture_cache_hash(texture_hash: &str) -> String {
    let mut digest = Sha1::new();
    for code_unit in texture_hash.encode_utf16() {
        digest.update(code_unit.to_le_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn default_player_skin_for_profile(
    profile: &ResolvableProfileSummary,
) -> EntityDefaultPlayerSkin {
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

pub fn default_player_skin_for_profile_id(profile_id: u128) -> EntityDefaultPlayerSkin {
    EntityDefaultPlayerSkin::from_vanilla_index(default_player_skin_index(profile_id))
}

pub(crate) fn entity_player_skin_model(model: PlayerModelTypeSummary) -> EntityPlayerSkinModel {
    match model {
        PlayerModelTypeSummary::Slim => EntityPlayerSkinModel::Slim,
        PlayerModelTypeSummary::Wide => EntityPlayerSkinModel::Wide,
    }
}

pub(crate) fn profile_texture_handle(url: &str) -> u64 {
    if let Some(texture_hash) = vanilla_profile_texture_hash(url) {
        let cache_hash = vanilla_profile_texture_cache_hash(&texture_hash);
        if let Ok(handle) = u64::from_str_radix(&cache_hash[..16], 16) {
            return handle;
        }
    }
    let mut hash = 0xcbf29ce484222325u64;
    for byte in url.as_bytes() {
        hash ^= u64::from(*byte);
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

pub(crate) const PROFILE_TEXTURE_SET_CACHE_DURATION: Duration = Duration::from_secs(15);
pub(crate) const MAX_PROFILE_TEXTURE_SET_ENTRIES: usize = 2_048;
pub(crate) const MAX_PROFILE_TEXTURE_ASSET_ENTRIES: usize = 2_048;
const MAX_PROFILE_TEXTURE_URL_BYTES: usize = 2_048;
pub(crate) const MAX_PROFILE_TEXTURE_PROPERTY_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ProfileTextureAssetKind {
    Skin,
    Cape,
    Elytra,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProfileTextureAssetKey {
    kind: ProfileTextureAssetKind,
    texture_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProfileTextureSetKey {
    profile_id: uuid::Uuid,
    packed_textures: Option<(String, Option<String>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileTextureAssetState {
    Loading,
    Ready(u64),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileTextureAssetEntry {
    url: String,
    request_handle: u64,
    state: ProfileTextureAssetState,
    queued: bool,
}

#[derive(Debug, Clone)]
struct ProfileTextureSetEntry {
    skin: Option<ProfileTextureAssetKey>,
    cape: Option<ProfileTextureAssetKey>,
    elytra: Option<ProfileTextureAssetKey>,
    model: EntityPlayerSkinModel,
    signature_state: ProfileTextureSignatureState,
    failed: bool,
    last_access: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileTextureDownloadRequest {
    pub(crate) kind: ProfileTextureAssetKind,
    pub(crate) url: String,
    pub(crate) handle: u64,
}

/// Process-lifetime owner for Vanilla's profile-result cache and three
/// type-specific texture caches. A profile result is visible only after every
/// referenced base texture has reached the GPU; any failed member falls back
/// to the complete default set.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileTextureSetCache {
    sets: HashMap<ProfileTextureSetKey, ProfileTextureSetEntry>,
    assets: HashMap<ProfileTextureAssetKey, ProfileTextureAssetEntry>,
    retained_key_bytes: usize,
}

impl ProfileTextureSetCache {
    pub(crate) fn player_skin_for_profile(
        &mut self,
        profile: &ResolvableProfileSummary,
        require_secure: bool,
        profile_property_keys: &[Vec<u8>],
    ) -> EntityPlayerSkin {
        self.player_skin_for_profile_at(
            profile,
            require_secure,
            profile_property_keys,
            Instant::now(),
        )
    }

    pub(crate) fn player_skin_for_profile_at(
        &mut self,
        profile: &ResolvableProfileSummary,
        require_secure: bool,
        profile_property_keys: &[Vec<u8>],
        now: Instant,
    ) -> EntityPlayerSkin {
        let key = self.get_or_register(profile, profile_property_keys, now);
        let fallback = default_player_skin_for_profile(profile);
        let entry = self.sets.get(&key).expect("registered profile texture set");
        if require_secure && entry.signature_state != ProfileTextureSignatureState::Signed {
            return EntityPlayerSkin::ProfiledDefault(fallback);
        }
        if entry.failed || !self.set_is_ready(entry) {
            return EntityPlayerSkin::ProfiledDefault(fallback);
        }
        let Some(asset_key) = entry.skin.as_ref() else {
            return EntityPlayerSkin::ProfiledDefault(fallback);
        };
        let asset = self.assets.get(asset_key).expect("registered skin asset");
        let ProfileTextureAssetState::Ready(handle) = asset.state else {
            unreachable!("ready profile set has a non-ready skin")
        };
        EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
            handle,
            fallback,
            model: profile
                .skin_patch
                .model
                .map(entity_player_skin_model)
                .unwrap_or(entry.model),
            status: EntityDynamicPlayerSkinStatus::Ready,
        })
    }

    pub(crate) fn player_texture_for_profile(
        &mut self,
        profile: &ResolvableProfileSummary,
        kind: ProfileTextureAssetKind,
        require_secure: bool,
        profile_property_keys: &[Vec<u8>],
    ) -> Option<u64> {
        let key = self.get_or_register(profile, profile_property_keys, Instant::now());
        let entry = self.sets.get(&key).expect("registered profile texture set");
        if require_secure && entry.signature_state != ProfileTextureSignatureState::Signed {
            return None;
        }
        if entry.failed || !self.set_is_ready(entry) {
            return None;
        }
        let asset_key = match kind {
            ProfileTextureAssetKind::Skin => entry.skin.as_ref(),
            ProfileTextureAssetKind::Cape => entry.cape.as_ref(),
            ProfileTextureAssetKind::Elytra => entry.elytra.as_ref(),
        }?;
        match self.assets.get(asset_key)?.state {
            ProfileTextureAssetState::Ready(handle) => Some(handle),
            ProfileTextureAssetState::Loading | ProfileTextureAssetState::Failed => None,
        }
    }

    pub(crate) fn pending_downloads_for_profile(
        &mut self,
        profile: &ResolvableProfileSummary,
        require_secure: bool,
        profile_property_keys: &[Vec<u8>],
    ) -> Vec<ProfileTextureDownloadRequest> {
        let set_key = self.get_or_register(profile, profile_property_keys, Instant::now());
        let set = self
            .sets
            .get(&set_key)
            .expect("registered profile texture set");
        if set.failed
            || (require_secure && set.signature_state != ProfileTextureSignatureState::Signed)
        {
            return Vec::new();
        }
        let asset_keys = [&set.skin, &set.cape, &set.elytra]
            .into_iter()
            .filter_map(Clone::clone)
            .collect::<Vec<_>>();
        let mut pending = asset_keys
            .into_iter()
            .filter_map(|key| {
                let entry = self.assets.get_mut(&key).expect("registered profile asset");
                (entry.state == ProfileTextureAssetState::Loading && !entry.queued).then(|| {
                    entry.queued = true;
                    ProfileTextureDownloadRequest {
                        kind: key.kind,
                        url: entry.url.clone(),
                        handle: entry.request_handle,
                    }
                })
            })
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| {
            profile_texture_asset_kind_order(left.kind)
                .cmp(&profile_texture_asset_kind_order(right.kind))
                .then_with(|| left.url.cmp(&right.url))
        });
        pending
    }

    pub(crate) fn retry_download(&mut self, kind: ProfileTextureAssetKind, url: &str) {
        if let Some(entry) = self.asset_entry_mut(kind, url) {
            if entry.state == ProfileTextureAssetState::Loading {
                entry.queued = false;
            }
        }
    }

    pub(crate) fn mark_resolved(
        &mut self,
        kind: ProfileTextureAssetKind,
        url: &str,
        texture_handle: u64,
    ) -> bool {
        if let Some(entry) = self.asset_entry_mut(kind, url) {
            let changed = entry.state != ProfileTextureAssetState::Ready(texture_handle);
            entry.state = ProfileTextureAssetState::Ready(texture_handle);
            return changed;
        }
        false
    }

    pub(crate) fn mark_failed(&mut self, kind: ProfileTextureAssetKind, url: &str) -> bool {
        if let Some(entry) = self.asset_entry_mut(kind, url) {
            let changed = entry.state != ProfileTextureAssetState::Failed;
            entry.state = ProfileTextureAssetState::Failed;
            return changed;
        }
        false
    }

    fn get_or_register(
        &mut self,
        profile: &ResolvableProfileSummary,
        profile_property_keys: &[Vec<u8>],
        now: Instant,
    ) -> ProfileTextureSetKey {
        let mut released_key_bytes = 0usize;
        self.sets.retain(|key, entry| {
            let retain = now.saturating_duration_since(entry.last_access)
                < PROFILE_TEXTURE_SET_CACHE_DURATION;
            if !retain {
                released_key_bytes =
                    released_key_bytes.saturating_add(profile_texture_set_key_bytes(key));
            }
            retain
        });
        self.retained_key_bytes = self.retained_key_bytes.saturating_sub(released_key_bytes);
        let (key, property_within_budget) = profile_texture_set_key(profile);
        let signature_state =
            profile_texture_signature_state(&profile.properties, profile_property_keys);
        if let Some(entry) = self.sets.get_mut(&key) {
            entry.last_access = now;
            if signature_state == ProfileTextureSignatureState::Signed {
                entry.signature_state = ProfileTextureSignatureState::Signed;
            }
            return key;
        }
        if self.sets.len() >= MAX_PROFILE_TEXTURE_SET_ENTRIES {
            if let Some(evicted) = self
                .sets
                .iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(key, _)| key.clone())
            {
                self.sets.remove(&evicted);
                self.retained_key_bytes = self
                    .retained_key_bytes
                    .saturating_sub(profile_texture_set_key_bytes(&evicted));
            }
        }
        let textures = property_within_budget
            .then_some(profile.profile_textures.as_ref())
            .flatten();
        let fallback = default_player_skin_for_profile_id(key.profile_id.as_u128());
        let model = textures
            .and_then(|textures| textures.skin.as_ref())
            .map(|skin| entity_player_skin_model(skin.model))
            .unwrap_or_else(|| fallback.model());
        let (skin, skin_failed) = self.register_asset(
            ProfileTextureAssetKind::Skin,
            texture_url(textures, ProfileTextureAssetKind::Skin),
        );
        let (cape, cape_failed) = self.register_asset(
            ProfileTextureAssetKind::Cape,
            texture_url(textures, ProfileTextureAssetKind::Cape),
        );
        let (elytra, elytra_failed) = self.register_asset(
            ProfileTextureAssetKind::Elytra,
            texture_url(textures, ProfileTextureAssetKind::Elytra),
        );
        let entry = ProfileTextureSetEntry {
            skin,
            cape,
            elytra,
            model,
            signature_state,
            failed: !property_within_budget || skin_failed || cape_failed || elytra_failed,
            last_access: now,
        };
        self.retained_key_bytes = self
            .retained_key_bytes
            .saturating_add(profile_texture_set_key_bytes(&key));
        self.sets.insert(key.clone(), entry);
        key
    }

    fn register_asset(
        &mut self,
        kind: ProfileTextureAssetKind,
        url: Option<&str>,
    ) -> (Option<ProfileTextureAssetKey>, bool) {
        let Some(url) = url else {
            return (None, false);
        };
        if url.len() > MAX_PROFILE_TEXTURE_URL_BYTES {
            return (None, true);
        }
        let Some(texture_hash) = vanilla_profile_texture_hash(url) else {
            return (None, true);
        };
        let key = ProfileTextureAssetKey { kind, texture_hash };
        if let Some(existing) = self.assets.get(&key) {
            return (Some(key), existing.url != url);
        }
        if self.assets.len() >= MAX_PROFILE_TEXTURE_ASSET_ENTRIES {
            return (None, true);
        }
        self.assets
            .entry(key.clone())
            .or_insert_with(|| ProfileTextureAssetEntry {
                url: url.to_string(),
                request_handle: profile_texture_handle(url),
                state: ProfileTextureAssetState::Loading,
                queued: false,
            });
        (Some(key), false)
    }

    fn set_is_ready(&self, entry: &ProfileTextureSetEntry) -> bool {
        [&entry.skin, &entry.cape, &entry.elytra]
            .into_iter()
            .filter_map(Option::as_ref)
            .all(|key| {
                self.assets
                    .get(key)
                    .is_some_and(|asset| matches!(asset.state, ProfileTextureAssetState::Ready(_)))
            })
    }

    fn asset_entry_mut(
        &mut self,
        kind: ProfileTextureAssetKind,
        url: &str,
    ) -> Option<&mut ProfileTextureAssetEntry> {
        let key = ProfileTextureAssetKey {
            kind,
            texture_hash: vanilla_profile_texture_hash(url)?,
        };
        self.assets.get_mut(&key)
    }

    pub(crate) fn set_count(&self) -> usize {
        self.sets.len()
    }

    pub(crate) fn asset_count(&self) -> usize {
        self.assets.len()
    }

    pub(crate) fn retained_key_bytes(&self) -> usize {
        self.retained_key_bytes
    }

    pub(crate) fn accounting(&self) -> (usize, usize, usize) {
        (
            self.set_count(),
            self.asset_count(),
            self.retained_key_bytes(),
        )
    }
}

fn profile_texture_asset_kind_order(kind: ProfileTextureAssetKind) -> u8 {
    match kind {
        ProfileTextureAssetKind::Skin => 0,
        ProfileTextureAssetKind::Cape => 1,
        ProfileTextureAssetKind::Elytra => 2,
    }
}

fn profile_texture_set_key(profile: &ResolvableProfileSummary) -> (ProfileTextureSetKey, bool) {
    let profile_id = profile
        .uuid
        .or_else(|| {
            profile
                .name
                .as_deref()
                .map(bbb_protocol::codec::offline_player_uuid)
        })
        .unwrap_or_else(uuid::Uuid::nil);
    let property = profile
        .properties
        .iter()
        .find(|property| property.name == "textures");
    let property_bytes = property.map_or(0, |property| {
        property
            .value
            .len()
            .saturating_add(property.signature.as_ref().map_or(0, String::len))
    });
    let property_within_budget = property_bytes <= MAX_PROFILE_TEXTURE_PROPERTY_BYTES;
    let packed_textures = property
        .filter(|_| property_within_budget)
        .map(|property| (property.value.clone(), property.signature.clone()));
    (
        ProfileTextureSetKey {
            profile_id,
            packed_textures,
        },
        property_within_budget,
    )
}

fn profile_texture_set_key_bytes(key: &ProfileTextureSetKey) -> usize {
    std::mem::size_of::<uuid::Uuid>().saturating_add(key.packed_textures.as_ref().map_or(
        0,
        |(value, signature)| {
            value
                .len()
                .saturating_add(signature.as_ref().map_or(0, String::len))
        },
    ))
}

fn texture_url(
    textures: Option<&ProfileTexturesSummary>,
    kind: ProfileTextureAssetKind,
) -> Option<&str> {
    let textures = textures?;
    match kind {
        ProfileTextureAssetKind::Skin => textures.skin.as_ref().map(|skin| skin.url.as_str()),
        ProfileTextureAssetKind::Cape => textures.cape.as_ref().map(|texture| texture.url.as_str()),
        ProfileTextureAssetKind::Elytra => {
            textures.elytra.as_ref().map(|texture| texture.url.as_str())
        }
    }
}

pub(crate) trait SkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicPlayerTextureKind {
    Cape,
    Elytra,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpSkinPngFetcher {
    client: reqwest::blocking::Client,
}

impl HttpSkinPngFetcher {
    pub(crate) fn new() -> Result<Self> {
        crate::install_http_crypto_provider();
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("bbb-native")
                .build()
                .context("create player skin HTTP client")?,
        })
    }
}

impl SkinPngFetcher for HttpSkinPngFetcher {
    fn fetch_skin_png(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("request HTTP player skin {url}"))?
            .error_for_status()
            .with_context(|| format!("fetch HTTP player skin {url}"))?;
        let content_length = response.content_length();
        read_bounded_png_body(
            response,
            content_length,
            max_bytes,
            &format!("HTTP player texture {url}"),
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DynamicPlayerSkinRuntime {
    cache_dir: PathBuf,
    skins: HashMap<String, DynamicPlayerSkinImage>,
}

impl DynamicPlayerSkinRuntime {
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            skins: HashMap::new(),
        }
    }

    pub(crate) fn get_or_fetch_player_skin(
        &mut self,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<&DynamicPlayerSkinImage> {
        if !self.skins.contains_key(url) {
            let (bytes, fetched) = self.cached_or_fetched_bytes(url, fetcher)?;
            let skin = decode_dynamic_player_skin_png(handle, &bytes)
                .with_context(|| format!("process player skin texture from {url}"))?;
            if fetched {
                let path = self.player_skin_path(url)?;
                if let Err(err) = write_player_skin_cache(&path, &bytes) {
                    tracing::warn!(
                        ?err,
                        path = %path.display(),
                        "failed to cache player skin texture"
                    );
                }
            }
            self.skins.insert(url.to_string(), skin);
        }
        Ok(self
            .skins
            .get(url)
            .expect("player skin inserted before lookup"))
    }

    fn take_or_fetch_player_skin(
        &mut self,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<DynamicPlayerSkinImage> {
        self.get_or_fetch_player_skin(handle, url, fetcher)?;
        Ok(self
            .skins
            .remove(url)
            .expect("player skin inserted before owned extraction"))
    }

    #[cfg(test)]
    fn cached_skin_count(&self) -> usize {
        self.skins.len()
    }

    fn cached_or_fetched_bytes(
        &self,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<(Vec<u8>, bool)> {
        let path = self.player_skin_path(url)?;
        if path.is_file() {
            return read_bounded_png_file(
                &path,
                MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
                "cached player skin",
            )
            .map(|bytes| (bytes, false));
        }

        let bytes = fetcher
            .fetch_skin_png(url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
            .with_context(|| format!("fetch player skin texture from {url}"))?;
        validate_png_body_len(
            bytes.len(),
            MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES,
            "fetched player skin",
        )?;
        Ok((bytes, true))
    }

    fn player_skin_path(&self, url: &str) -> Result<PathBuf> {
        player_skin_cache_path(&self.cache_dir, url)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DynamicPlayerTextureRuntime {
    cache_dir: PathBuf,
    textures: HashMap<(DynamicPlayerTextureKind, String), DynamicPlayerTextureImage>,
}

impl DynamicPlayerTextureRuntime {
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            textures: HashMap::new(),
        }
    }

    pub(crate) fn get_or_fetch_player_texture(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<&DynamicPlayerTextureImage> {
        let key = (kind, url.to_string());
        if !self.textures.contains_key(&key) {
            let (bytes, fetched) = self.cached_or_fetched_bytes(kind, url, fetcher)?;
            let texture = decode_dynamic_player_texture_png(handle, &bytes)
                .with_context(|| format!("process player {kind:?} texture from {url}"))?;
            if fetched {
                let path = self.player_texture_path(url)?;
                if let Err(err) = write_player_skin_cache(&path, &bytes) {
                    tracing::warn!(
                        ?err,
                        path = %path.display(),
                        kind = ?kind,
                        "failed to cache player profile texture"
                    );
                }
            }
            self.textures.insert(key.clone(), texture);
        }
        Ok(self
            .textures
            .get(&key)
            .expect("player profile texture inserted before lookup"))
    }

    fn take_or_fetch_player_texture(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<DynamicPlayerTextureImage> {
        self.get_or_fetch_player_texture(kind, handle, url, fetcher)?;
        Ok(self
            .textures
            .remove(&(kind, url.to_string()))
            .expect("player profile texture inserted before owned extraction"))
    }

    #[cfg(test)]
    fn cached_texture_count(&self) -> usize {
        self.textures.len()
    }

    fn cached_or_fetched_bytes(
        &self,
        kind: DynamicPlayerTextureKind,
        url: &str,
        fetcher: &mut impl SkinPngFetcher,
    ) -> Result<(Vec<u8>, bool)> {
        let path = self.player_texture_path(url)?;
        if path.is_file() {
            return read_bounded_png_file(
                &path,
                MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
                &format!("cached player {kind:?} texture"),
            )
            .map(|bytes| (bytes, false));
        }

        let bytes = fetcher
            .fetch_skin_png(url, MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES)
            .with_context(|| format!("fetch player {kind:?} texture from {url}"))?;
        validate_png_body_len(
            bytes.len(),
            MAX_DYNAMIC_PLAYER_TEXTURE_PNG_BYTES,
            &format!("fetched player {kind:?} texture"),
        )?;
        Ok((bytes, true))
    }

    fn player_texture_path(&self, url: &str) -> Result<PathBuf> {
        player_texture_cache_path(&self.cache_dir, url)
    }
}

const DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES: usize = 2_048;
const DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY: usize = 64;
const DEFAULT_DYNAMIC_TEXTURE_MAX_RESIDENT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DynamicTextureRequestKind {
    Skin,
    Cape,
    Elytra,
}

impl From<DynamicPlayerTextureKind> for DynamicTextureRequestKind {
    fn from(kind: DynamicPlayerTextureKind) -> Self {
        match kind {
            DynamicPlayerTextureKind::Cape => Self::Cape,
            DynamicPlayerTextureKind::Elytra => Self::Elytra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DynamicTextureKey {
    kind: DynamicTextureRequestKind,
    identity: String,
    handle: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadedDynamicTexture {
    Skin(DynamicPlayerSkinImage),
    Profile(DynamicPlayerTextureImage),
}

impl DownloadedDynamicTexture {
    fn logical_bytes(&self) -> usize {
        match self {
            Self::Skin(image) => image.rgba.len(),
            Self::Profile(image) => image.rgba.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicTextureCacheObservation {
    pub resource_generation: u64,
    pub session_generation: u64,
    pub accepting_requests: bool,
    pub profile_sets: usize,
    pub profile_assets: usize,
    pub profile_key_bytes: usize,
    pub entries: usize,
    pub pending: usize,
    pub ready: usize,
    pub failed: usize,
    pub stale_results: u64,
    pub queue_full: u64,
    pub max_resident_bytes: usize,
    pub resident_bytes: usize,
    pub peak_resident_bytes: usize,
    pub resident_evictions: u64,
    pub resident_rejections: u64,
    pub worker_result_bytes: usize,
    pub peak_worker_result_bytes: usize,
    pub recent_delivery_bytes: usize,
    pub peak_delivery_bytes: usize,
}

#[derive(Debug, Default)]
struct DynamicTextureWorkerBytes {
    current: AtomicUsize,
    peak: AtomicUsize,
}

impl DynamicTextureWorkerBytes {
    fn set(&self, bytes: usize) {
        self.current.store(bytes, Ordering::Release);
        self.peak.fetch_max(bytes, Ordering::Relaxed);
    }

    fn current(&self) -> usize {
        self.current.load(Ordering::Acquire)
    }

    fn peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AsyncDynamicTextureState {
    Pending,
    Ready(DownloadedDynamicTexture),
    Failed,
}

#[derive(Debug, Clone)]
struct AsyncDynamicTextureEntry {
    state: AsyncDynamicTextureState,
    last_used: u64,
    delivery_pending: bool,
}

/// One bounded process owner for every remote profile texture request. Exact
/// keys keep semantic kind, full URL identity and render handle; presentation
/// generations only request re-delivery and never invalidate retained CPU
/// images or force another network fetch.
#[derive(Debug)]
pub(crate) struct AsyncDynamicTextureRuntime {
    resource_generation: u64,
    session_generation: u64,
    accepting_requests: bool,
    entries: HashMap<DynamicTextureKey, AsyncDynamicTextureEntry>,
    request_tx: SyncSender<DynamicTextureRequest>,
    result_rx: Receiver<DynamicTextureResult>,
    worker_bytes: Arc<DynamicTextureWorkerBytes>,
    sequence: u64,
    max_entries: usize,
    max_resident_bytes: usize,
    pending_entries: usize,
    ready_entries: usize,
    failed_entries: usize,
    stale_results: u64,
    queue_full: u64,
    resident_bytes: usize,
    peak_resident_bytes: usize,
    resident_evictions: u64,
    resident_rejections: u64,
    recent_delivery_bytes: usize,
    peak_delivery_bytes: usize,
}

impl AsyncDynamicTextureRuntime {
    pub(crate) fn new(
        resource_generation: u64,
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
    ) -> Self {
        Self::with_limits(
            resource_generation,
            cache_dir,
            fetcher,
            DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES,
            DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY,
        )
    }

    fn with_limits(
        resource_generation: u64,
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
        max_entries: usize,
        pending_capacity: usize,
    ) -> Self {
        Self::with_limits_and_resident_bytes(
            resource_generation,
            cache_dir,
            fetcher,
            max_entries,
            pending_capacity,
            DEFAULT_DYNAMIC_TEXTURE_MAX_RESIDENT_BYTES,
        )
    }

    fn with_limits_and_resident_bytes(
        resource_generation: u64,
        cache_dir: impl Into<PathBuf>,
        fetcher: impl SkinPngFetcher + Send + 'static,
        max_entries: usize,
        pending_capacity: usize,
        max_resident_bytes: usize,
    ) -> Self {
        let (request_tx, request_rx) =
            mpsc::sync_channel::<DynamicTextureRequest>(pending_capacity);
        let (result_tx, result_rx) = mpsc::sync_channel::<DynamicTextureResult>(0);
        let worker_bytes = Arc::new(DynamicTextureWorkerBytes::default());
        let thread_worker_bytes = Arc::clone(&worker_bytes);
        let cache_dir = cache_dir.into();
        thread::spawn(move || {
            let mut skins = DynamicPlayerSkinRuntime::new(&cache_dir);
            let mut profiles = DynamicPlayerTextureRuntime::new(cache_dir);
            let mut fetcher = fetcher;
            for request in request_rx {
                let result = match request.key.kind {
                    DynamicTextureRequestKind::Skin => skins
                        .take_or_fetch_player_skin(
                            request.key.handle,
                            &request.key.identity,
                            &mut fetcher,
                        )
                        .map(DownloadedDynamicTexture::Skin),
                    DynamicTextureRequestKind::Cape | DynamicTextureRequestKind::Elytra => {
                        let kind = request
                            .key
                            .kind
                            .profile_kind()
                            .expect("profile texture request kind");
                        profiles
                            .take_or_fetch_player_texture(
                                kind,
                                request.key.handle,
                                &request.key.identity,
                                &mut fetcher,
                            )
                            .map(DownloadedDynamicTexture::Profile)
                    }
                }
                .map_err(|error| format!("{error:#}"));
                // Move the decoded image out of the worker's synchronous
                // helper cache. The bounded outer owner is the only retained
                // CPU-pixel owner; disk cache remains available after LRU
                // eviction.
                thread_worker_bytes.set(
                    result
                        .as_ref()
                        .map_or(0, DownloadedDynamicTexture::logical_bytes),
                );
                if result_tx
                    .send(DynamicTextureResult {
                        key: request.key,
                        result,
                    })
                    .is_err()
                {
                    thread_worker_bytes.set(0);
                    break;
                }
                thread_worker_bytes.set(0);
            }
        });
        Self {
            resource_generation,
            session_generation: 0,
            accepting_requests: true,
            entries: HashMap::new(),
            request_tx,
            result_rx,
            worker_bytes,
            sequence: 0,
            max_entries: max_entries.max(1),
            max_resident_bytes,
            pending_entries: 0,
            ready_entries: 0,
            failed_entries: 0,
            stale_results: 0,
            queue_full: 0,
            resident_bytes: 0,
            peak_resident_bytes: 0,
            resident_evictions: 0,
            resident_rejections: 0,
            recent_delivery_bytes: 0,
            peak_delivery_bytes: 0,
        }
    }

    pub(crate) fn queue_skin(&mut self, handle: u64, url: &str) -> bool {
        self.queue(DynamicTextureRequestKind::Skin, handle, url)
    }

    pub(crate) fn queue_profile(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
    ) -> bool {
        self.queue(kind.into(), handle, url)
    }

    fn queue(&mut self, kind: DynamicTextureRequestKind, handle: u64, identity: &str) -> bool {
        self.drain_worker_results();
        if !self.accepting_requests {
            return false;
        }
        self.sequence = self.sequence.saturating_add(1);
        let key = DynamicTextureKey {
            kind,
            identity: identity.to_owned(),
            handle,
        };
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.sequence;
            return !matches!(entry.state, AsyncDynamicTextureState::Failed);
        }
        if self.entries.len() >= self.max_entries {
            let evicted = self
                .entries
                .iter()
                .filter(|(_, entry)| {
                    !matches!(entry.state, AsyncDynamicTextureState::Pending)
                        && !entry.delivery_pending
                })
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone());
            let Some(evicted) = evicted else {
                self.queue_full = self.queue_full.saturating_add(1);
                return false;
            };
            if self.remove_entry(&evicted) {
                self.resident_evictions = self.resident_evictions.saturating_add(1);
            }
        }
        self.entries.insert(
            key.clone(),
            AsyncDynamicTextureEntry {
                state: AsyncDynamicTextureState::Pending,
                last_used: self.sequence,
                delivery_pending: false,
            },
        );
        self.pending_entries = self.pending_entries.saturating_add(1);
        match self
            .request_tx
            .try_send(DynamicTextureRequest { key: key.clone() })
        {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.queue_full = self.queue_full.saturating_add(1);
                self.remove_entry(&key);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.queue_full = self.queue_full.saturating_add(1);
                let entry = self
                    .entries
                    .get_mut(&key)
                    .expect("queued dynamic texture entry");
                entry.state = AsyncDynamicTextureState::Failed;
                self.pending_entries = self.pending_entries.saturating_sub(1);
                self.failed_entries = self.failed_entries.saturating_add(1);
                false
            }
        }
    }

    pub(crate) fn drain_skin_results(&mut self) -> Vec<AsyncDynamicPlayerSkinResult> {
        self.drain_worker_results();
        let key = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                key.kind == DynamicTextureRequestKind::Skin && entry.delivery_pending
            })
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone());
        let Some(key) = key else {
            return Vec::new();
        };
        let entry = self.entries.get_mut(&key).expect("selected skin delivery");
        entry.delivery_pending = false;
        let skin = match &entry.state {
            AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(skin)) => {
                Some(skin.clone())
            }
            AsyncDynamicTextureState::Failed => None,
            AsyncDynamicTextureState::Pending
            | AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(_)) => {
                unreachable!("skin delivery key has incompatible state")
            }
        };
        self.record_delivery(skin.as_ref().map_or(0, |skin| skin.rgba.len()));
        vec![AsyncDynamicPlayerSkinResult {
            url: key.identity,
            skin,
        }]
    }

    pub(crate) fn drain_profile_results(&mut self) -> Vec<AsyncDynamicPlayerTextureResult> {
        self.drain_worker_results();
        let key = self
            .entries
            .iter()
            .filter(|(key, entry)| key.kind.profile_kind().is_some() && entry.delivery_pending)
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone());
        let Some(key) = key else {
            return Vec::new();
        };
        let kind = key.kind.profile_kind().expect("selected profile delivery");
        let entry = self
            .entries
            .get_mut(&key)
            .expect("selected profile texture delivery");
        entry.delivery_pending = false;
        let texture = match &entry.state {
            AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(texture)) => {
                Some(texture.clone())
            }
            AsyncDynamicTextureState::Failed => None,
            AsyncDynamicTextureState::Pending
            | AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(_)) => {
                unreachable!("profile delivery key has incompatible state")
            }
        };
        self.record_delivery(texture.as_ref().map_or(0, |texture| texture.rgba.len()));
        vec![AsyncDynamicPlayerTextureResult {
            kind,
            url: key.identity,
            texture,
        }]
    }

    fn drain_worker_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            let is_current_pending = self
                .entries
                .get(&result.key)
                .is_some_and(|entry| matches!(entry.state, AsyncDynamicTextureState::Pending));
            if !is_current_pending {
                self.stale_results = self.stale_results.saturating_add(1);
                continue;
            }
            self.pending_entries = self.pending_entries.saturating_sub(1);
            match result.result {
                Ok(texture) => {
                    let logical_bytes = texture.logical_bytes();
                    if self.reserve_resident_bytes(logical_bytes, &result.key) {
                        let entry = self
                            .entries
                            .get_mut(&result.key)
                            .expect("admitted dynamic texture entry");
                        entry.state = AsyncDynamicTextureState::Ready(texture);
                        entry.delivery_pending = self.accepting_requests;
                        self.ready_entries = self.ready_entries.saturating_add(1);
                        self.resident_bytes = self.resident_bytes.saturating_add(logical_bytes);
                        self.peak_resident_bytes =
                            self.peak_resident_bytes.max(self.resident_bytes);
                    } else {
                        let entry = self
                            .entries
                            .get_mut(&result.key)
                            .expect("rejected dynamic texture entry");
                        entry.state = AsyncDynamicTextureState::Failed;
                        entry.delivery_pending = self.accepting_requests;
                        self.failed_entries = self.failed_entries.saturating_add(1);
                        self.resident_rejections = self.resident_rejections.saturating_add(1);
                    }
                }
                Err(_) => {
                    let entry = self
                        .entries
                        .get_mut(&result.key)
                        .expect("failed dynamic texture entry");
                    entry.state = AsyncDynamicTextureState::Failed;
                    entry.delivery_pending = self.accepting_requests;
                    self.failed_entries = self.failed_entries.saturating_add(1);
                }
            }
        }
    }

    pub(crate) fn reset_session_generation(
        &mut self,
        session_generation: u64,
        accepting_requests: bool,
    ) {
        self.drain_worker_results();
        self.session_generation = session_generation;
        self.accepting_requests = accepting_requests;
        for entry in self.entries.values_mut() {
            entry.delivery_pending =
                accepting_requests && matches!(entry.state, AsyncDynamicTextureState::Ready(_));
        }
    }

    pub(crate) fn reset_resource_generation(&mut self, resource_generation: u64) {
        self.resource_generation = resource_generation;
        for entry in self.entries.values_mut() {
            entry.delivery_pending = self.accepting_requests
                && matches!(entry.state, AsyncDynamicTextureState::Ready(_));
        }
    }

    pub(crate) fn observation(&mut self) -> DynamicTextureCacheObservation {
        let worker_result_bytes = self.worker_bytes.current();
        self.drain_worker_results();
        DynamicTextureCacheObservation {
            resource_generation: self.resource_generation,
            session_generation: self.session_generation,
            accepting_requests: self.accepting_requests,
            profile_sets: 0,
            profile_assets: 0,
            profile_key_bytes: 0,
            entries: self.entries.len(),
            pending: self.pending_entries,
            ready: self.ready_entries,
            failed: self.failed_entries,
            stale_results: self.stale_results,
            queue_full: self.queue_full,
            max_resident_bytes: self.max_resident_bytes,
            resident_bytes: self.resident_bytes,
            peak_resident_bytes: self.peak_resident_bytes,
            resident_evictions: self.resident_evictions,
            resident_rejections: self.resident_rejections,
            worker_result_bytes,
            peak_worker_result_bytes: self.worker_bytes.peak(),
            recent_delivery_bytes: self.recent_delivery_bytes,
            peak_delivery_bytes: self.peak_delivery_bytes,
        }
    }

    fn reserve_resident_bytes(
        &mut self,
        incoming_bytes: usize,
        protected: &DynamicTextureKey,
    ) -> bool {
        if incoming_bytes > self.max_resident_bytes {
            return false;
        }
        let target_resident_bytes = self.max_resident_bytes - incoming_bytes;
        while self.resident_bytes > target_resident_bytes {
            let evicted = self
                .entries
                .iter()
                .filter(|(key, entry)| {
                    *key != protected
                        && !entry.delivery_pending
                        && matches!(entry.state, AsyncDynamicTextureState::Ready(_))
                })
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone());
            let Some(evicted) = evicted else {
                return false;
            };
            let removed_ready = self.remove_entry(&evicted);
            debug_assert!(removed_ready);
            self.resident_evictions = self.resident_evictions.saturating_add(1);
        }
        true
    }

    fn record_delivery(&mut self, bytes: usize) {
        self.recent_delivery_bytes = bytes;
        self.peak_delivery_bytes = self.peak_delivery_bytes.max(bytes);
    }

    fn remove_entry(&mut self, key: &DynamicTextureKey) -> bool {
        let Some(entry) = self.entries.remove(key) else {
            return false;
        };
        match entry.state {
            AsyncDynamicTextureState::Pending => {
                self.pending_entries = self.pending_entries.saturating_sub(1);
                false
            }
            AsyncDynamicTextureState::Ready(texture) => {
                self.ready_entries = self.ready_entries.saturating_sub(1);
                self.resident_bytes = self.resident_bytes.saturating_sub(texture.logical_bytes());
                true
            }
            AsyncDynamicTextureState::Failed => {
                self.failed_entries = self.failed_entries.saturating_sub(1);
                false
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn downloaded_skin_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Skin(_))
                )
            })
            .count()
    }

    #[cfg(test)]
    pub(crate) fn downloaded_texture_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(
                    entry.state,
                    AsyncDynamicTextureState::Ready(DownloadedDynamicTexture::Profile(_))
                )
            })
            .count()
    }

    #[cfg(test)]
    fn stale_results(&self) -> u64 {
        self.stale_results
    }

    #[cfg(test)]
    fn queue_full(&self) -> u64 {
        self.queue_full
    }
}

impl DynamicTextureRequestKind {
    const fn profile_kind(self) -> Option<DynamicPlayerTextureKind> {
        match self {
            Self::Skin => None,
            Self::Cape => Some(DynamicPlayerTextureKind::Cape),
            Self::Elytra => Some(DynamicPlayerTextureKind::Elytra),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncDynamicPlayerSkinResult {
    pub(crate) url: String,
    pub(crate) skin: Option<DynamicPlayerSkinImage>,
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncDynamicPlayerTextureResult {
    pub(crate) kind: DynamicPlayerTextureKind,
    pub(crate) url: String,
    pub(crate) texture: Option<DynamicPlayerTextureImage>,
}

#[derive(Debug, Clone)]
struct DynamicTextureRequest {
    key: DynamicTextureKey,
}

#[derive(Debug)]
struct DynamicTextureResult {
    key: DynamicTextureKey,
    result: std::result::Result<DownloadedDynamicTexture, String>,
}

pub fn default_player_skin_cache_dir() -> PathBuf {
    std::env::temp_dir().join("bbb-native-player-skins")
}

pub(crate) fn player_skin_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    vanilla_profile_texture_cache_path(cache_dir, url)
}

pub(crate) fn player_texture_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    vanilla_profile_texture_cache_path(cache_dir, url)
}

fn vanilla_profile_texture_cache_path(cache_dir: &Path, url: &str) -> Result<PathBuf> {
    let texture_hash = vanilla_profile_texture_hash(url)
        .with_context(|| format!("reject non-Vanilla profile texture URL {url}"))?;
    let cache_hash = vanilla_profile_texture_cache_hash(&texture_hash);
    let prefix = cache_hash.get(..2).unwrap_or("xx");
    Ok(cache_dir.join(prefix).join(cache_hash))
}

fn read_bounded_png_file(path: &Path, max_bytes: usize, label: &str) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let declared_bytes = file
        .metadata()
        .with_context(|| format!("read {label} metadata {}", path.display()))?
        .len();
    read_bounded_png_body(file, Some(declared_bytes), max_bytes, label)
        .with_context(|| format!("read {label} {}", path.display()))
}

fn read_bounded_png_body(
    reader: impl Read,
    declared_bytes: Option<u64>,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>> {
    let max_bytes_u64 = u64::try_from(max_bytes).context("dynamic texture byte limit overflow")?;
    if let Some(declared_bytes) = declared_bytes {
        if declared_bytes > max_bytes_u64 {
            bail!("{label} declares {declared_bytes} bytes, exceeding the {max_bytes} byte limit");
        }
    }
    let read_limit = max_bytes_u64
        .checked_add(1)
        .context("dynamic texture read limit overflow")?;
    let capacity = declared_bytes
        .and_then(|bytes| usize::try_from(bytes).ok())
        .unwrap_or(0)
        .min(max_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} body"))?;
    validate_png_body_len(bytes.len(), max_bytes, label)?;
    Ok(bytes)
}

fn validate_png_body_len(actual_bytes: usize, max_bytes: usize, label: &str) -> Result<()> {
    if actual_bytes > max_bytes {
        bail!("{label} is {actual_bytes} bytes, exceeding the {max_bytes} byte limit");
    }
    Ok(())
}

fn write_player_skin_cache(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create player skin cache directory {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("write player skin cache {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
    use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey};
    use std::{
        io::{Cursor, Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::Duration,
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn profile_texture_property_trust_matches_services_signature_states() {
        let mut random = StdRng::seed_from_u64(0x5a17_cafe);
        let private_key = RsaPrivateKey::new(&mut random, 1024).unwrap();
        let public_key = private_key.to_public_key();
        let public_der = public_key.to_public_key_der().unwrap();
        let value = "packed-profile-textures";
        let signature = private_key
            .sign(Pkcs1v15Sign::new::<Sha1>(), &Sha1::digest(value.as_bytes()))
            .unwrap();
        let signed = GameProfilePropertySummary {
            name: "textures".to_string(),
            value: value.to_string(),
            signature: Some(STANDARD.encode(signature)),
        };

        assert_eq!(
            profile_texture_signature_state(
                std::slice::from_ref(&signed),
                &[public_der.as_bytes().to_vec()],
            ),
            ProfileTextureSignatureState::Signed
        );
        assert_eq!(
            profile_texture_signature_state(std::slice::from_ref(&signed), &[]),
            ProfileTextureSignatureState::Invalid
        );

        let mut unsigned = signed.clone();
        unsigned.signature = None;
        assert_eq!(
            profile_texture_signature_state(&[unsigned], &[]),
            ProfileTextureSignatureState::Unsigned
        );
        let mut invalid = signed;
        invalid.signature = Some("not base64".to_string());
        assert_eq!(
            profile_texture_signature_state(&[invalid], &[public_der.as_bytes().to_vec()]),
            ProfileTextureSignatureState::Invalid
        );
    }

    #[test]
    fn profile_texture_url_and_cache_identity_match_authlib_and_guava() {
        assert_eq!(
            vanilla_profile_texture_hash(
                "https://textures.minecraft.net/texture/skinhash?ignored=yes"
            ),
            Some("skinhash".to_string())
        );
        assert_eq!(
            vanilla_profile_texture_hash("http://textures.minecraft.net/path/abc.png"),
            Some("abc".to_string())
        );
        assert_eq!(
            vanilla_profile_texture_hash("ftp://textures.minecraft.net/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_hash("https://textures.minecraft.net.example/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_hash("https://example.com/texture/skin"),
            None
        );
        assert_eq!(
            vanilla_profile_texture_cache_hash("skinhash"),
            "18aa6cdcc5d1ab4d4952af5973d144d5e69d0881"
        );
        assert_eq!(
            vanilla_profile_texture_cache_hash("abc"),
            "9f04f41a848514162050e3d68c1a7abb441dc2b5"
        );
    }

    #[test]
    fn dynamic_skin_runtime_fetches_processes_writes_and_reuses_memory_cache() {
        let root = unique_temp_dir("dynamic-skin-runtime-memory");
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let png = rgba_png(64, 64, |x, y| [x as u8, y as u8, 7, 255]);
        let mut fetcher = StaticSkinFetcher::new(png);

        let first = runtime
            .get_or_fetch_player_skin(
                0xabc,
                "https://textures.minecraft.net/texture/skin",
                &mut fetcher,
            )
            .unwrap()
            .clone();
        let second = runtime
            .get_or_fetch_player_skin(
                0xabc,
                "https://textures.minecraft.net/texture/skin",
                &mut fetcher,
            )
            .unwrap()
            .clone();

        assert_eq!(first, second);
        assert_eq!(first.handle, 0xabc);
        assert_eq!(first.rgba.len(), 64 * 64 * 4);
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_skin_count(), 1);
        assert!(
            player_skin_cache_path(&root, "https://textures.minecraft.net/texture/skin")
                .unwrap()
                .is_file()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_skin_runtime_reads_disk_cache_before_fetching() {
        let root = unique_temp_dir("dynamic-skin-runtime-disk");
        let handle = 0x1234;
        let url = "https://textures.minecraft.net/texture/cached-skin";
        let path = player_skin_cache_path(&root, url).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, rgba_png(64, 32, |x, y| [x as u8, y as u8, 11, 255])).unwrap();

        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());
        let skin = runtime
            .get_or_fetch_player_skin(handle, url, &mut fetcher)
            .unwrap();

        assert_eq!(skin.handle, handle);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(fetcher.calls, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_skin_runtime_does_not_cache_failed_fetch_or_decode() {
        let root = unique_temp_dir("dynamic-skin-runtime-failed");
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(b"not a png".to_vec());

        let err = runtime
            .get_or_fetch_player_skin(
                7,
                "https://textures.minecraft.net/texture/bad",
                &mut fetcher,
            )
            .unwrap_err();

        assert!(err.to_string().contains("process player skin texture"));
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_skin_count(), 0);
        assert!(
            !player_skin_cache_path(&root, "https://textures.minecraft.net/texture/bad")
                .unwrap()
                .exists()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_runtime_fetches_processes_writes_and_reuses_memory_cache() {
        let root = unique_temp_dir("dynamic-texture-runtime-memory");
        let mut runtime = DynamicPlayerTextureRuntime::new(&root);
        let png = rgba_png(64, 32, |x, y| [x as u8, y as u8, 29, 255]);
        let mut fetcher = StaticSkinFetcher::new(png);
        let url = "https://textures.minecraft.net/texture/cape";

        let first = runtime
            .get_or_fetch_player_texture(DynamicPlayerTextureKind::Cape, 0xcafe, url, &mut fetcher)
            .unwrap()
            .clone();
        let second = runtime
            .get_or_fetch_player_texture(DynamicPlayerTextureKind::Cape, 0xcafe, url, &mut fetcher)
            .unwrap()
            .clone();

        assert_eq!(first, second);
        assert_eq!(first.handle, 0xcafe);
        assert_eq!(first.size, [64, 32]);
        assert_eq!(first.rgba.len(), 64 * 32 * 4);
        assert_eq!(fetcher.calls, 1);
        assert_eq!(runtime.cached_texture_count(), 1);
        assert!(player_texture_cache_path(&root, url).unwrap().is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_runtime_shares_vanilla_disk_identity_across_kinds() {
        let root = unique_temp_dir("dynamic-texture-runtime-kinds");
        let handle = 0x1234;
        let url = "https://textures.minecraft.net/texture/shared";
        let cape_path = player_texture_cache_path(&root, url).unwrap();
        let elytra_path = player_texture_cache_path(&root, url).unwrap();
        assert_eq!(cape_path, elytra_path);
        fs::create_dir_all(cape_path.parent().unwrap()).unwrap();
        fs::write(&cape_path, rgba_png(64, 32, |_, _| [1, 2, 3, 4])).unwrap();

        let mut runtime = DynamicPlayerTextureRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());
        let cape = runtime
            .get_or_fetch_player_texture(DynamicPlayerTextureKind::Cape, handle, url, &mut fetcher)
            .unwrap()
            .clone();
        let elytra = runtime
            .get_or_fetch_player_texture(
                DynamicPlayerTextureKind::Elytra,
                handle,
                url,
                &mut fetcher,
            )
            .unwrap()
            .clone();

        assert_eq!(cape.size, [64, 32]);
        assert_eq!(elytra.size, [64, 32]);
        assert_eq!(fetcher.calls, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_skin_runtime_downloads_processes_and_reuses_result() {
        let root = unique_temp_dir("async-dynamic-skin-runtime-ready");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 19, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-ready";

        runtime.queue_skin(0x55, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue_skin(0x55, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        let skin = results[0].skin.as_ref().unwrap();
        assert_eq!(skin.handle, 0x55);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(runtime.downloaded_skin_count(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let observation = runtime.observation();
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 1, 0)
        );
        assert_eq!(observation.resident_bytes, 64 * 64 * 4);
        assert_eq!(observation.peak_resident_bytes, 64 * 64 * 4);
        assert_eq!(
            observation.max_resident_bytes,
            DEFAULT_DYNAMIC_TEXTURE_MAX_RESIDENT_BYTES
        );
        assert_eq!(observation.peak_worker_result_bytes, 64 * 64 * 4);
        assert_eq!(observation.recent_delivery_bytes, 64 * 64 * 4);
        assert_eq!(observation.peak_delivery_bytes, 64 * 64 * 4);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_skin_runtime_caches_failed_download_or_decode() {
        let root = unique_temp_dir("async-dynamic-skin-runtime-failed");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-failed";

        runtime.queue_skin(0x66, url);
        let results = drain_until_async_skin_result(&mut runtime);
        runtime.queue_skin(0x66, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].skin, None);
        assert_eq!(runtime.downloaded_skin_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let observation = runtime.observation();
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 0, 1)
        );
        assert_eq!(observation.resident_bytes, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_texture_runtime_downloads_processes_and_reuses_result() {
        let root = unique_temp_dir("async-dynamic-texture-runtime-ready");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 32, |x, y| [x as u8, y as u8, 71, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/async-cape";

        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        let texture = results[0].texture.as_ref().unwrap();
        assert_eq!(texture.handle, 0x77);
        assert_eq!(texture.size, [64, 32]);
        assert_eq!(runtime.downloaded_texture_count(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn async_dynamic_texture_runtime_caches_failed_download_or_decode() {
        let root = unique_temp_dir("async-dynamic-texture-runtime-failed");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            0,
            &root,
            AsyncStaticSkinFetcher::new(b"not a png".to_vec(), calls.clone()),
        );
        let url = "https://textures.minecraft.net/texture/async-cape-failed";

        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x78, url);
        let results = drain_until_async_texture_result(&mut runtime);
        runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x78, url);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        assert_eq!(results[0].texture, None);
        assert_eq!(runtime.downloaded_texture_count(), 0);
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_runtime_retains_and_redelivers_across_session_generation() {
        let root = unique_temp_dir("async-dynamic-texture-stale-generation");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::new(
            7,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 93, 255]),
                calls.clone(),
            ),
        );
        let url = "https://textures.minecraft.net/texture/stale-generation";

        assert!(runtime.queue_skin(0x91, url));
        for _ in 0..100 {
            if calls.load(Ordering::Relaxed) != 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        runtime.reset_session_generation(8, false);
        for _ in 0..100 {
            assert!(runtime.drain_skin_results().is_empty());
            if runtime.observation().ready == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(runtime.stale_results(), 0);
        assert_eq!(runtime.downloaded_skin_count(), 1);
        runtime.reset_session_generation(9, true);
        let results = runtime.drain_skin_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        assert!(results[0].skin.is_some());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_runtime_bounds_all_pending_entries() {
        let root = unique_temp_dir("async-dynamic-texture-bounded");
        let calls = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel();
        let mut runtime = AsyncDynamicTextureRuntime::with_limits(
            3,
            &root,
            BlockingSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 17, 255]),
                calls.clone(),
                release_rx,
            ),
            1,
            1,
        );

        assert!(runtime.queue_skin(1, "https://textures.minecraft.net/texture/one"));
        for _ in 0..100 {
            if calls.load(Ordering::Relaxed) == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/two",
        ));
        assert_eq!(runtime.entries.len(), 1);
        assert_eq!(runtime.queue_full(), 1);
        release_tx.send(()).unwrap();
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_observation_tracks_ready_eviction_without_byte_growth() {
        let root = unique_temp_dir("async-dynamic-texture-observed-eviction");
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = AsyncDynamicTextureRuntime::with_limits(
            5,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(64, 64, |x, y| [x as u8, y as u8, 31, 255]),
                calls.clone(),
            ),
            1,
            1,
        );

        assert!(runtime.queue_skin(1, "https://textures.minecraft.net/texture/one"));
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);
        assert!(runtime.queue_skin(2, "https://textures.minecraft.net/texture/two"));
        assert_eq!(drain_until_async_skin_result(&mut runtime).len(), 1);

        let observation = runtime.observation();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(observation.entries, 1);
        assert_eq!(
            (observation.pending, observation.ready, observation.failed),
            (0, 1, 0)
        );
        assert_eq!(observation.resident_bytes, 64 * 64 * 4);
        assert_eq!(observation.peak_resident_bytes, 64 * 64 * 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_resident_budget_evicts_delivered_lru_owner() {
        let root = unique_temp_dir("async-dynamic-texture-byte-eviction");
        let calls = Arc::new(AtomicUsize::new(0));
        let image_bytes = 4 * 4 * 4;
        let mut runtime = AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(
            9,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(4, 4, |x, y| [x as u8, y as u8, 41, 255]),
                calls.clone(),
            ),
            4,
            2,
            image_bytes,
        );

        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/first",
        ));
        for _ in 0..100 {
            if runtime.worker_bytes.current() == image_bytes {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(runtime.worker_bytes.current(), image_bytes);
        assert_eq!(drain_until_async_texture_result(&mut runtime).len(), 1);
        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/second",
        ));
        assert_eq!(drain_until_async_texture_result(&mut runtime).len(), 1);

        let observation = runtime.observation();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(observation.max_resident_bytes, image_bytes);
        assert_eq!(observation.resident_bytes, image_bytes);
        assert_eq!(observation.peak_resident_bytes, image_bytes);
        assert_eq!(observation.resident_evictions, 1);
        assert_eq!(observation.resident_rejections, 0);
        assert_eq!(observation.entries, 1);
        assert_eq!(observation.peak_worker_result_bytes, image_bytes);
        assert_eq!(observation.recent_delivery_bytes, image_bytes);
        assert_eq!(observation.peak_delivery_bytes, image_bytes);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_resident_budget_protects_pending_delivery() {
        let root = unique_temp_dir("async-dynamic-texture-protected-delivery");
        let calls = Arc::new(AtomicUsize::new(0));
        let image_bytes = 4 * 4 * 4;
        let mut runtime = AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(
            10,
            &root,
            AsyncStaticSkinFetcher::new(
                rgba_png(4, 4, |x, y| [x as u8, y as u8, 43, 255]),
                calls.clone(),
            ),
            4,
            2,
            image_bytes,
        );

        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/first",
        ));
        wait_for_observation(&mut runtime, |observation| observation.ready == 1);
        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/second",
        ));
        let observation = wait_for_observation(&mut runtime, |observation| {
            observation.pending == 0 && observation.failed == 1
        });
        assert_eq!(observation.resident_bytes, image_bytes);
        assert_eq!(observation.resident_evictions, 0);
        assert_eq!(observation.resident_rejections, 1);

        let first = runtime.drain_profile_results();
        let second = runtime.drain_profile_results();
        assert_eq!(first.len(), 1);
        assert!(first[0].texture.is_some());
        assert_eq!(second.len(), 1);
        assert!(second[0].texture.is_none());
        assert!(runtime.drain_profile_results().is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_dynamic_texture_resident_budget_rejects_oversized_owner() {
        let root = unique_temp_dir("async-dynamic-texture-byte-rejection");
        let calls = Arc::new(AtomicUsize::new(0));
        let image_bytes = 4 * 4 * 4;
        let mut runtime = AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(
            11,
            &root,
            AsyncStaticSkinFetcher::new(rgba_png(4, 4, |x, y| [x as u8, y as u8, 47, 255]), calls),
            2,
            1,
            image_bytes - 1,
        );

        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/oversized",
        ));
        let result = drain_until_async_texture_result(&mut runtime);
        assert_eq!(result.len(), 1);
        assert!(result[0].texture.is_none());
        let observation = runtime.observation();
        assert_eq!(observation.resident_bytes, 0);
        assert_eq!(observation.peak_resident_bytes, 0);
        assert_eq!(observation.resident_rejections, 1);
        assert_eq!(observation.peak_worker_result_bytes, image_bytes);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn http_skin_fetcher_reads_successful_response_bytes() {
        let body = b"player skin bytes".to_vec();
        let url = spawn_http_response(200, "OK", body.clone());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        assert_eq!(
            fetcher
                .fetch_skin_png(&url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
                .unwrap(),
            body
        );
    }

    #[test]
    fn http_skin_fetcher_rejects_non_success_status() {
        let url = spawn_http_response(404, "Not Found", b"missing".to_vec());
        let mut fetcher = HttpSkinPngFetcher::new().unwrap();

        let err = fetcher
            .fetch_skin_png(&url, MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES)
            .unwrap_err();

        assert!(err.to_string().contains("fetch HTTP player skin"));
    }

    #[test]
    fn bounded_dynamic_texture_body_rejects_declared_and_streamed_overflow() {
        let declared = read_bounded_png_body(Cursor::new(b"ok"), Some(5), 4, "declared fixture")
            .unwrap_err()
            .to_string();
        assert!(declared.contains("declares 5 bytes"), "{declared}");

        let streamed = read_bounded_png_body(Cursor::new(b"12345"), None, 4, "streamed fixture")
            .unwrap_err()
            .to_string();
        assert!(streamed.contains("is 5 bytes"), "{streamed}");
    }

    #[test]
    fn oversized_cached_skin_is_rejected_before_body_allocation_or_fetch() {
        let root = unique_temp_dir("dynamic-skin-runtime-oversized-cache");
        let handle = 0xdead;
        let url = "https://textures.minecraft.net/texture/oversized-cache";
        let path = player_skin_cache_path(&root, url).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_len(MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES as u64 + 1)
            .unwrap();
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(Vec::new());

        let err = format!(
            "{:#}",
            runtime
                .get_or_fetch_player_skin(handle, url, &mut fetcher,)
                .unwrap_err()
        );

        assert!(err.contains("declares 1048577 bytes"), "{err}");
        assert_eq!(fetcher.calls, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_fetched_skin_is_not_written_to_cache() {
        let root = unique_temp_dir("dynamic-skin-runtime-oversized-fetch");
        let handle = 0xbeef;
        let mut runtime = DynamicPlayerSkinRuntime::new(&root);
        let mut fetcher = StaticSkinFetcher::new(vec![0_u8; MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES + 1]);

        let err = runtime
            .get_or_fetch_player_skin(
                handle,
                "https://textures.minecraft.net/texture/oversized-fetch",
                &mut fetcher,
            )
            .unwrap_err()
            .to_string();

        assert!(err.contains("exceeding the 1048576 byte limit"), "{err}");
        assert_eq!(fetcher.calls, 1);
        assert!(!player_skin_cache_path(
            &root,
            "https://textures.minecraft.net/texture/oversized-fetch"
        )
        .unwrap()
        .exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dynamic_texture_transport_does_not_restore_unbounded_body_reads() {
        let production = include_str!("skin_runtime.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("dynamic texture production source");
        assert!(!production.contains(".bytes()"));
        assert!(!production.contains("fs::read("));
        assert!(production.contains(".take(read_limit)"));
        assert!(production.contains("sync_channel::<DynamicTextureResult>(0)"));
    }

    struct StaticSkinFetcher {
        bytes: Vec<u8>,
        calls: usize,
    }

    impl StaticSkinFetcher {
        fn new(bytes: Vec<u8>) -> Self {
            Self { bytes, calls: 0 }
        }
    }

    impl SkinPngFetcher for StaticSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls += 1;
            Ok(self.bytes.clone())
        }
    }

    struct AsyncStaticSkinFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
    }

    struct BlockingSkinFetcher {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
        release_rx: mpsc::Receiver<()>,
    }

    impl BlockingSkinFetcher {
        fn new(bytes: Vec<u8>, calls: Arc<AtomicUsize>, release_rx: mpsc::Receiver<()>) -> Self {
            Self {
                bytes,
                calls,
                release_rx,
            }
        }
    }

    impl SkinPngFetcher for BlockingSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.release_rx.recv().context("release test skin fetch")?;
            Ok(self.bytes.clone())
        }
    }

    impl AsyncStaticSkinFetcher {
        fn new(bytes: Vec<u8>, calls: Arc<AtomicUsize>) -> Self {
            Self { bytes, calls }
        }
    }

    impl SkinPngFetcher for AsyncStaticSkinFetcher {
        fn fetch_skin_png(&mut self, _url: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(self.bytes.clone())
        }
    }

    fn rgba_png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut image = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                image.put_pixel(x, y, image::Rgba(pixel(x, y)));
            }
        }
        let mut cursor = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn drain_until_async_skin_result(
        runtime: &mut AsyncDynamicTextureRuntime,
    ) -> Vec<AsyncDynamicPlayerSkinResult> {
        for _ in 0..100 {
            let results = runtime.drain_skin_results();
            if !results.is_empty() {
                return results;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async skin result");
    }

    fn drain_until_async_texture_result(
        runtime: &mut AsyncDynamicTextureRuntime,
    ) -> Vec<AsyncDynamicPlayerTextureResult> {
        for _ in 0..100 {
            let results = runtime.drain_profile_results();
            if !results.is_empty() {
                return results;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for async player texture result");
    }

    fn wait_for_observation(
        runtime: &mut AsyncDynamicTextureRuntime,
        ready: impl Fn(DynamicTextureCacheObservation) -> bool,
    ) -> DynamicTextureCacheObservation {
        for _ in 0..100 {
            let observation = runtime.observation();
            if ready(observation) {
                return observation;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for dynamic texture observation");
    }

    fn spawn_http_response(status: u16, reason: &str, body: Vec<u8>) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let reason = reason.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{addr}/skin.png")
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
