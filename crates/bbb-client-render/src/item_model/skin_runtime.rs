use bbb_client_clock::Instant;
use std::{
    borrow::Cow,
    collections::{hash_map::RandomState, HashMap},
    time::Duration,
};

use bbb_protocol::{
    packets::{
        decode_profile_textures_from_properties, GameProfilePropertySummary,
        PlayerModelTypeSummary, PlayerSkinPatchSummary, ProfileTexturesSummary,
        ResolvableProfileSummary,
    },
    profile_texture_handle, vanilla_profile_texture_hash, DynamicPlayerTextureKind,
    DynamicTextureFetchKind, DynamicTextureFetchRequest, DynamicTextureFetchResult,
    MAX_PROFILE_TEXTURE_URL_BYTES,
};
use bbb_render_types::{
    decode_dynamic_player_skin_png, decode_dynamic_player_texture_png, DynamicPlayerSkinImage,
    DynamicPlayerTextureImage, EntityDefaultPlayerSkin, EntityDynamicPlayerSkin,
    EntityDynamicPlayerSkinStatus, EntityDynamicPlayerTexture, EntityDynamicPlayerTextureKind,
    EntityPlayerSkin, EntityPlayerSkinModel, EntityPlayerTextures,
};

pub fn default_player_skin_for_profile(
    profile: &ResolvableProfileSummary,
) -> EntityDefaultPlayerSkin {
    if let Some(body) = profile.skin_patch.body.as_ref() {
        if let Some(skin) = EntityDefaultPlayerSkin::from_texture_path(&body.texture_path) {
            return skin;
        }
    }

    default_player_skin_for_profile_id(
        ProfileTextureSource::Resolvable(profile)
            .profile_id()
            .as_u128(),
    )
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

#[derive(Clone, Copy)]
pub(crate) enum ProfileTextureSource<'a> {
    Resolvable(&'a ResolvableProfileSummary),
    PlayerInfo(&'a bbb_render_types::PlayerInfoProfileState),
}

impl<'a> From<&'a ResolvableProfileSummary> for ProfileTextureSource<'a> {
    fn from(profile: &'a ResolvableProfileSummary) -> Self {
        Self::Resolvable(profile)
    }
}

impl<'a> ProfileTextureSource<'a> {
    fn profile_id(self) -> uuid::Uuid {
        match self {
            Self::PlayerInfo(profile) => profile.uuid,
            Self::Resolvable(profile) => profile
                .uuid
                .or_else(|| {
                    profile
                        .name
                        .as_deref()
                        .map(bbb_protocol::codec::offline_player_uuid)
                })
                .unwrap_or_else(uuid::Uuid::nil),
        }
    }

    fn properties(self) -> &'a [GameProfilePropertySummary] {
        match self {
            Self::Resolvable(profile) => &profile.properties,
            Self::PlayerInfo(profile) => &profile.properties,
        }
    }

    pub(super) fn skin_patch(self) -> Option<&'a PlayerSkinPatchSummary> {
        match self {
            Self::Resolvable(profile) => Some(&profile.skin_patch),
            Self::PlayerInfo(_) => None,
        }
    }

    pub(super) fn textures(self) -> Option<Cow<'a, ProfileTexturesSummary>> {
        match self {
            Self::Resolvable(profile) => profile.profile_textures.as_ref().map(Cow::Borrowed),
            Self::PlayerInfo(profile) => decode_profile_textures_from_properties(
                profile
                    .properties
                    .iter()
                    .map(|property| (&property.name, &property.value)),
            )
            .map(Cow::Owned),
        }
    }

    pub(super) fn fallback(self) -> EntityDefaultPlayerSkin {
        match self {
            Self::Resolvable(profile) => default_player_skin_for_profile(profile),
            Self::PlayerInfo(profile) => default_player_skin_for_profile_id(profile.uuid.as_u128()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ProfileTextureSetKey {
    profile_id: uuid::Uuid,
    packed_textures: Option<(bbb_protocol::JavaString, Option<bbb_protocol::JavaString>)>,
}

#[derive(Clone, Copy, Hash)]
struct ProfileTextureSetKeyRef<'a> {
    profile_id: uuid::Uuid,
    packed_textures: Option<(
        &'a bbb_protocol::JavaString,
        Option<&'a bbb_protocol::JavaString>,
    )>,
}

impl ProfileTextureSetKey {
    fn as_ref(&self) -> ProfileTextureSetKeyRef<'_> {
        ProfileTextureSetKeyRef {
            profile_id: self.profile_id,
            packed_textures: self
                .packed_textures
                .as_ref()
                .map(|(value, signature)| (value, signature.as_ref())),
        }
    }
}

impl hashbrown::Equivalent<ProfileTextureSetKey> for ProfileTextureSetKeyRef<'_> {
    fn equivalent(&self, other: &ProfileTextureSetKey) -> bool {
        self.profile_id == other.profile_id
            && self.packed_textures == other.as_ref().packed_textures
    }
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
    signature_verified: bool,
    failed: bool,
    last_access: Instant,
}

struct ProfileTextureSetView<'a> {
    entry: &'a ProfileTextureSetEntry,
    assets: &'a HashMap<ProfileTextureAssetKey, ProfileTextureAssetEntry>,
}

impl ProfileTextureSetView<'_> {
    fn texture(&self, kind: ProfileTextureAssetKind) -> Option<u64> {
        let key = match kind {
            ProfileTextureAssetKind::Skin => self.entry.skin.as_ref(),
            ProfileTextureAssetKind::Cape => self.entry.cape.as_ref(),
            ProfileTextureAssetKind::Elytra => self.entry.elytra.as_ref(),
        }?;
        let ProfileTextureAssetState::Ready(handle) = self
            .assets
            .get(key)
            .expect("registered profile asset")
            .state
        else {
            unreachable!("ready profile set has a non-ready asset")
        };
        Some(handle)
    }

    fn skin(
        &self,
        profile: ProfileTextureSource<'_>,
        fallback: EntityDefaultPlayerSkin,
    ) -> EntityPlayerSkin {
        self.texture(ProfileTextureAssetKind::Skin).map_or(
            EntityPlayerSkin::ProfiledDefault(fallback),
            |handle| {
                EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
                    handle,
                    fallback,
                    model: profile
                        .skin_patch()
                        .and_then(|patch| patch.model)
                        .map(entity_player_skin_model)
                        .unwrap_or(self.entry.model),
                    status: EntityDynamicPlayerSkinStatus::Ready,
                })
            },
        )
    }
}

impl ProfileTextureSetEntry {
    fn allows(&self, require_secure: bool) -> bool {
        !self.failed && (!require_secure || self.signature_verified)
    }

    fn ready<'a>(
        &'a self,
        assets: &'a HashMap<ProfileTextureAssetKey, ProfileTextureAssetEntry>,
        require_secure: bool,
    ) -> Option<ProfileTextureSetView<'a>> {
        (self.allows(require_secure)
            && [&self.skin, &self.cape, &self.elytra]
                .into_iter()
                .flatten()
                .all(|key| {
                    assets.get(key).is_some_and(|asset| {
                        matches!(asset.state, ProfileTextureAssetState::Ready(_))
                    })
                }))
        .then_some(ProfileTextureSetView {
            entry: self,
            assets,
        })
    }

    fn queue_downloads(
        &self,
        assets: &mut HashMap<ProfileTextureAssetKey, ProfileTextureAssetEntry>,
        require_secure: bool,
        runtime: &mut AsyncDynamicTextureRuntime,
    ) {
        if !self.allows(require_secure) {
            return;
        }
        for key in [&self.skin, &self.cape, &self.elytra].into_iter().flatten() {
            let asset = assets.get_mut(key).expect("registered profile asset");
            if asset.state == ProfileTextureAssetState::Loading && !asset.queued {
                asset.queued = match key.kind {
                    ProfileTextureAssetKind::Skin => {
                        runtime.queue_skin(asset.request_handle, &asset.url)
                    }
                    ProfileTextureAssetKind::Cape => runtime.queue_profile(
                        DynamicPlayerTextureKind::Cape,
                        asset.request_handle,
                        &asset.url,
                    ),
                    ProfileTextureAssetKind::Elytra => runtime.queue_profile(
                        DynamicPlayerTextureKind::Elytra,
                        asset.request_handle,
                        &asset.url,
                    ),
                };
            }
        }
    }
}

/// Process-lifetime owner for Vanilla's profile-result cache and three
/// type-specific texture caches. A profile result is visible only after every
/// referenced base texture has reached the GPU; any failed member falls back
/// to the complete default set.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileTextureSetCache {
    sets: hashbrown::HashMap<ProfileTextureSetKey, ProfileTextureSetEntry, RandomState>,
    assets: HashMap<ProfileTextureAssetKey, ProfileTextureAssetEntry>,
    retained_key_bytes: usize,
}

impl ProfileTextureSetCache {
    pub(crate) fn player_skin_for_profile<'a>(
        &mut self,
        profile: impl Into<ProfileTextureSource<'a>>,
        require_secure: bool,
        verify_signature: impl FnOnce() -> bool,
    ) -> EntityPlayerSkin {
        let profile = profile.into();
        self.player_skin_for_profile_at(
            profile,
            require_secure,
            verify_signature,
            bbb_client_clock::now(),
        )
    }

    pub(crate) fn player_skin_for_profile_at<'a>(
        &mut self,
        profile: impl Into<ProfileTextureSource<'a>>,
        require_secure: bool,
        verify_signature: impl FnOnce() -> bool,
        now: Instant,
    ) -> EntityPlayerSkin {
        let profile = profile.into();
        let key = self.get_or_register(profile, verify_signature, now);
        let fallback = profile.fallback();
        self.sets
            .get(&key)
            .expect("registered profile texture set")
            .ready(&self.assets, require_secure)
            .map_or(EntityPlayerSkin::ProfiledDefault(fallback), |set| {
                set.skin(profile, fallback)
            })
    }

    pub(crate) fn player_texture_for_profile<'a>(
        &mut self,
        profile: impl Into<ProfileTextureSource<'a>>,
        kind: ProfileTextureAssetKind,
        require_secure: bool,
        verify_signature: impl FnOnce() -> bool,
    ) -> Option<u64> {
        let key = self.get_or_register(profile.into(), verify_signature, bbb_client_clock::now());
        self.sets
            .get(&key)
            .expect("registered profile texture set")
            .ready(&self.assets, require_secure)?
            .texture(kind)
    }

    pub(crate) fn player_textures_for_profile(
        &mut self,
        profile: ProfileTextureSource<'_>,
        require_secure: bool,
        verify_signature: impl FnOnce() -> bool,
        runtime: Option<&mut AsyncDynamicTextureRuntime>,
    ) -> EntityPlayerTextures {
        let key = self.get_or_register(profile, verify_signature, bbb_client_clock::now());
        let entry = self.sets.get(&key).expect("registered profile texture set");
        let fallback = profile.fallback();
        let mut textures = EntityPlayerTextures {
            skin: EntityPlayerSkin::ProfiledDefault(fallback),
            cape: None,
            elytra: None,
        };
        if let Some(set) = entry.ready(&self.assets, require_secure) {
            textures.skin = set.skin(profile, fallback);
            textures.cape = set.texture(ProfileTextureAssetKind::Cape).map(|handle| {
                EntityDynamicPlayerTexture {
                    handle,
                    kind: EntityDynamicPlayerTextureKind::Cape,
                }
            });
            textures.elytra = set.texture(ProfileTextureAssetKind::Elytra).map(|handle| {
                EntityDynamicPlayerTexture {
                    handle,
                    kind: EntityDynamicPlayerTextureKind::Elytra,
                }
            });
        }
        if let Some(runtime) = runtime {
            entry.queue_downloads(&mut self.assets, require_secure, runtime);
        }
        textures
    }

    pub(crate) fn queue_downloads_for_profile(
        &mut self,
        profile: ProfileTextureSource<'_>,
        require_secure: bool,
        verify_signature: impl FnOnce() -> bool,
        runtime: &mut AsyncDynamicTextureRuntime,
    ) {
        let key = self.get_or_register(profile, verify_signature, bbb_client_clock::now());
        self.sets
            .get(&key)
            .expect("registered profile texture set")
            .queue_downloads(&mut self.assets, require_secure, runtime);
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

    fn get_or_register<'a>(
        &mut self,
        profile: ProfileTextureSource<'a>,
        verify_signature: impl FnOnce() -> bool,
        now: Instant,
    ) -> ProfileTextureSetKeyRef<'a> {
        let mut released_key_bytes = 0usize;
        self.sets.retain(|key, entry| {
            let retain = now.saturating_duration_since(entry.last_access)
                < PROFILE_TEXTURE_SET_CACHE_DURATION;
            if !retain {
                released_key_bytes =
                    released_key_bytes.saturating_add(profile_texture_set_key_bytes(key.as_ref()));
            }
            retain
        });
        self.retained_key_bytes = self.retained_key_bytes.saturating_sub(released_key_bytes);
        let (key, property_within_budget) = profile_texture_set_key(profile);
        if let Some(entry) = self.sets.get_mut(&key) {
            entry.last_access = now;
            if !entry.signature_verified && verify_signature() {
                entry.signature_verified = true;
            }
            return key;
        }
        let signature_verified = verify_signature();
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
                    .saturating_sub(profile_texture_set_key_bytes(evicted.as_ref()));
            }
        }
        let textures = property_within_budget.then(|| profile.textures()).flatten();
        let textures = textures.as_deref();
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
            signature_verified,
            failed: !property_within_budget || skin_failed || cape_failed || elytra_failed,
            last_access: now,
        };
        self.retained_key_bytes = self
            .retained_key_bytes
            .saturating_add(profile_texture_set_key_bytes(key));
        self.sets.insert(
            ProfileTextureSetKey {
                profile_id: key.profile_id,
                packed_textures: key
                    .packed_textures
                    .map(|(value, signature)| (value.clone(), signature.cloned())),
            },
            entry,
        );
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

fn profile_texture_set_key<'a>(
    profile: ProfileTextureSource<'a>,
) -> (ProfileTextureSetKeyRef<'a>, bool) {
    let profile_id = profile.profile_id();
    let property = profile
        .properties()
        .iter()
        .find(|property| property.name == "textures");
    let property_bytes = property.map_or(0, |property| {
        property
            .value
            .units()
            .len()
            .saturating_mul(2)
            .saturating_add(
                property
                    .signature
                    .as_ref()
                    .map_or(0, |value| value.units().len().saturating_mul(2)),
            )
    });
    let property_within_budget = property_bytes <= MAX_PROFILE_TEXTURE_PROPERTY_BYTES;
    let packed_textures = property
        .filter(|_| property_within_budget)
        .map(|property| (&property.value, property.signature.as_ref()));
    (
        ProfileTextureSetKeyRef {
            profile_id,
            packed_textures,
        },
        property_within_budget,
    )
}

fn profile_texture_set_key_bytes(key: ProfileTextureSetKeyRef<'_>) -> usize {
    std::mem::size_of::<uuid::Uuid>().saturating_add(key.packed_textures.as_ref().map_or(
        0,
        |(value, signature)| {
            value.units().len().saturating_mul(2).saturating_add(
                signature
                    .as_ref()
                    .map_or(0, |value| value.units().len().saturating_mul(2)),
            )
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

const DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES: usize = 2_048;
const DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY: usize = 64;
const DEFAULT_DYNAMIC_TEXTURE_MAX_RESIDENT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DynamicTextureKey {
    kind: DynamicTextureFetchKind,
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
    outbound_requests: Vec<DynamicTextureFetchRequest>,
    pending_capacity: usize,
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
    peak_worker_result_bytes: usize,
    recent_delivery_bytes: usize,
    peak_delivery_bytes: usize,
}

impl AsyncDynamicTextureRuntime {
    pub(crate) fn new(resource_generation: u64) -> Self {
        Self::with_limits(
            resource_generation,
            DEFAULT_DYNAMIC_TEXTURE_MAX_ENTRIES,
            DEFAULT_DYNAMIC_TEXTURE_PENDING_CAPACITY,
        )
    }

    fn with_limits(resource_generation: u64, max_entries: usize, pending_capacity: usize) -> Self {
        Self::with_limits_and_resident_bytes(
            resource_generation,
            max_entries,
            pending_capacity,
            DEFAULT_DYNAMIC_TEXTURE_MAX_RESIDENT_BYTES,
        )
    }

    fn with_limits_and_resident_bytes(
        resource_generation: u64,
        max_entries: usize,
        pending_capacity: usize,
        max_resident_bytes: usize,
    ) -> Self {
        Self {
            resource_generation,
            session_generation: 0,
            accepting_requests: true,
            entries: HashMap::new(),
            outbound_requests: Vec::new(),
            pending_capacity: pending_capacity.max(1),
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
            peak_worker_result_bytes: 0,
            recent_delivery_bytes: 0,
            peak_delivery_bytes: 0,
        }
    }

    pub(crate) fn queue_skin(&mut self, handle: u64, url: &str) -> bool {
        self.queue(DynamicTextureFetchKind::Skin, handle, url)
    }

    pub(crate) fn queue_profile(
        &mut self,
        kind: DynamicPlayerTextureKind,
        handle: u64,
        url: &str,
    ) -> bool {
        self.queue(kind.into(), handle, url)
    }

    fn queue(&mut self, kind: DynamicTextureFetchKind, handle: u64, identity: &str) -> bool {
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
        if self.pending_entries >= self.pending_capacity {
            self.queue_full = self.queue_full.saturating_add(1);
            return false;
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
        self.outbound_requests.push(DynamicTextureFetchRequest {
            kind: key.kind,
            url: key.identity,
            handle: key.handle,
        });
        true
    }

    pub(crate) fn take_pending_requests(&mut self) -> Vec<DynamicTextureFetchRequest> {
        std::mem::take(&mut self.outbound_requests)
    }

    /// Decodes fetched PNG bytes into bounded resident RGBA images and returns
    /// `(url, png_bytes)` for network-fetched textures whose decode succeeded so
    /// the network owner only persists validated PNGs to disk.
    pub(crate) fn apply_fetch_results(
        &mut self,
        results: impl IntoIterator<Item = DynamicTextureFetchResult>,
    ) -> Vec<(String, Vec<u8>)> {
        let mut validated_for_disk_cache = Vec::new();
        for result in results {
            let key = DynamicTextureKey {
                kind: result.kind,
                identity: result.url,
                handle: result.handle,
            };
            let is_current_pending = self
                .entries
                .get(&key)
                .is_some_and(|entry| matches!(entry.state, AsyncDynamicTextureState::Pending));
            if !is_current_pending {
                self.stale_results = self.stale_results.saturating_add(1);
                continue;
            }
            self.pending_entries = self.pending_entries.saturating_sub(1);

            let decoded = result.png_bytes.and_then(|raw_bytes| {
                let decoded = match key.kind {
                    DynamicTextureFetchKind::Skin => {
                        decode_dynamic_player_skin_png(key.handle, &raw_bytes)
                            .map(DownloadedDynamicTexture::Skin)
                    }
                    DynamicTextureFetchKind::Cape | DynamicTextureFetchKind::Elytra => {
                        decode_dynamic_player_texture_png(key.handle, &raw_bytes)
                            .map(DownloadedDynamicTexture::Profile)
                    }
                }
                .map_err(|err| format!("{err:#}"))?;
                Ok((decoded, raw_bytes))
            });

            match decoded {
                Ok((texture, raw_bytes)) => {
                    let logical_bytes = texture.logical_bytes();
                    self.peak_worker_result_bytes =
                        self.peak_worker_result_bytes.max(logical_bytes);
                    if result.from_network {
                        validated_for_disk_cache.push((key.identity.clone(), raw_bytes));
                    }
                    if self.reserve_resident_bytes(logical_bytes, &key) {
                        let entry = self
                            .entries
                            .get_mut(&key)
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
                            .get_mut(&key)
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
                        .get_mut(&key)
                        .expect("failed dynamic texture entry");
                    entry.state = AsyncDynamicTextureState::Failed;
                    entry.delivery_pending = self.accepting_requests;
                    self.failed_entries = self.failed_entries.saturating_add(1);
                }
            }
        }
        validated_for_disk_cache
    }

    pub(crate) fn drain_skin_results(&mut self) -> Vec<AsyncDynamicPlayerSkinResult> {
        let key = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                key.kind == DynamicTextureFetchKind::Skin && entry.delivery_pending
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

    pub(crate) fn reset_session_generation(
        &mut self,
        session_generation: u64,
        accepting_requests: bool,
    ) {
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
            worker_result_bytes: 0,
            peak_worker_result_bytes: self.peak_worker_result_bytes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES;
    use std::io::Cursor;

    fn fulfill_requests(
        runtime: &mut AsyncDynamicTextureRuntime,
        png_bytes: &[u8],
        from_network: bool,
    ) -> Vec<(String, Vec<u8>)> {
        let requests = runtime.take_pending_requests();
        runtime.apply_fetch_results(requests.into_iter().map(|req| DynamicTextureFetchResult {
            kind: req.kind,
            url: req.url,
            handle: req.handle,
            png_bytes: Ok(png_bytes.to_vec()),
            from_network,
        }))
    }

    #[test]
    fn async_dynamic_skin_runtime_decodes_validates_and_reuses_result() {
        let mut runtime = AsyncDynamicTextureRuntime::new(0);
        let url = "https://textures.minecraft.net/texture/async-ready";
        let png = rgba_png(64, 64, |x, y| [x as u8, y as u8, 19, 255]);

        assert!(runtime.queue_skin(0x55, url));
        let validated = fulfill_requests(&mut runtime, &png, true);
        assert_eq!(validated, vec![(url.to_string(), png)]);
        let results = runtime.drain_skin_results();
        assert!(runtime.queue_skin(0x55, url));
        assert!(runtime.take_pending_requests().is_empty());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        let skin = results[0].skin.as_ref().unwrap();
        assert_eq!(skin.handle, 0x55);
        assert_eq!(skin.rgba.len(), 64 * 64 * 4);
        assert_eq!(runtime.downloaded_skin_count(), 1);
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
    }

    #[test]
    fn async_dynamic_skin_runtime_rejects_bad_or_oversized_png_from_disk_cache() {
        let mut runtime = AsyncDynamicTextureRuntime::new(0);
        let bad_url = "https://textures.minecraft.net/texture/async-failed";
        assert!(runtime.queue_skin(0x66, bad_url));
        let validated = fulfill_requests(&mut runtime, b"not a png", true);
        assert!(validated.is_empty());
        let results = runtime.drain_skin_results();
        assert!(!runtime.queue_skin(0x66, bad_url));
        assert!(runtime.take_pending_requests().is_empty());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skin, None);

        let oversized_url = "https://textures.minecraft.net/texture/oversized-fetch";
        assert!(runtime.queue_skin(0x67, oversized_url));
        let oversized = vec![0_u8; MAX_DYNAMIC_PLAYER_SKIN_PNG_BYTES + 1];
        let validated = fulfill_requests(&mut runtime, &oversized, true);
        assert!(validated.is_empty());
        let results = runtime.drain_skin_results();
        assert_eq!(results[0].skin, None);
        assert_eq!(runtime.downloaded_skin_count(), 0);
    }

    #[test]
    fn async_dynamic_texture_runtime_decodes_profile_cape_and_caches_failure() {
        let mut runtime = AsyncDynamicTextureRuntime::new(0);
        let url = "https://textures.minecraft.net/texture/async-cape";
        let png = rgba_png(64, 32, |x, y| [x as u8, y as u8, 71, 255]);

        assert!(runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url));
        let validated = fulfill_requests(&mut runtime, &png, false);
        assert!(validated.is_empty());
        let results = runtime.drain_profile_results();
        assert!(runtime.queue_profile(DynamicPlayerTextureKind::Cape, 0x77, url));
        assert!(runtime.take_pending_requests().is_empty());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DynamicPlayerTextureKind::Cape);
        assert_eq!(results[0].url, url);
        let texture = results[0].texture.as_ref().unwrap();
        assert_eq!(texture.handle, 0x77);
        assert_eq!(texture.size, [64, 32]);
        assert_eq!(runtime.downloaded_texture_count(), 1);
    }

    #[test]
    fn unified_dynamic_texture_runtime_retains_and_redelivers_across_session_generation() {
        let mut runtime = AsyncDynamicTextureRuntime::new(7);
        let url = "https://textures.minecraft.net/texture/stale-generation";
        let png = rgba_png(64, 64, |x, y| [x as u8, y as u8, 93, 255]);

        assert!(runtime.queue_skin(0x91, url));
        let requests = runtime.take_pending_requests();
        runtime.reset_session_generation(8, false);
        runtime.apply_fetch_results(requests.into_iter().map(|req| DynamicTextureFetchResult {
            kind: req.kind,
            url: req.url,
            handle: req.handle,
            png_bytes: Ok(png.clone()),
            from_network: true,
        }));
        assert!(runtime.drain_skin_results().is_empty());
        assert_eq!(runtime.observation().ready, 1);

        assert_eq!(runtime.stale_results(), 0);
        assert_eq!(runtime.downloaded_skin_count(), 1);
        runtime.reset_session_generation(9, true);
        let results = runtime.drain_skin_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, url);
        assert!(results[0].skin.is_some());
        assert!(runtime.take_pending_requests().is_empty());
    }

    #[test]
    fn unified_dynamic_texture_runtime_bounds_all_pending_entries() {
        let mut runtime = AsyncDynamicTextureRuntime::with_limits(3, 1, 1);
        let png = rgba_png(64, 64, |x, y| [x as u8, y as u8, 17, 255]);

        assert!(runtime.queue_skin(1, "https://textures.minecraft.net/texture/one"));
        assert!(!runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/two",
        ));
        assert_eq!(runtime.entries.len(), 1);
        assert_eq!(runtime.queue_full(), 1);
        fulfill_requests(&mut runtime, &png, true);
        assert_eq!(runtime.drain_skin_results().len(), 1);
    }

    #[test]
    fn unified_dynamic_texture_resident_budget_evicts_protects_and_rejects_owners() {
        let image_bytes = 4 * 4 * 4;
        let png = rgba_png(4, 4, |x, y| [x as u8, y as u8, 41, 255]);
        let mut runtime =
            AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(9, 4, 2, image_bytes);

        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/first",
        ));
        fulfill_requests(&mut runtime, &png, true);
        assert_eq!(runtime.drain_profile_results().len(), 1);

        assert!(runtime.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/second",
        ));
        fulfill_requests(&mut runtime, &png, true);
        assert_eq!(runtime.drain_profile_results().len(), 1);

        let observation = runtime.observation();
        assert_eq!(observation.resident_bytes, image_bytes);
        assert_eq!(observation.resident_evictions, 1);
        assert_eq!(observation.resident_rejections, 0);
        assert_eq!(observation.entries, 1);

        let mut protected =
            AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(10, 4, 2, image_bytes);
        assert!(protected.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/first",
        ));
        assert!(protected.queue_profile(
            DynamicPlayerTextureKind::Cape,
            2,
            "https://textures.minecraft.net/texture/second",
        ));
        fulfill_requests(&mut protected, &png, true);
        let obs = protected.observation();
        assert_eq!(obs.resident_bytes, image_bytes);
        assert_eq!(obs.resident_evictions, 0);
        assert_eq!(obs.resident_rejections, 1);

        let mut too_small =
            AsyncDynamicTextureRuntime::with_limits_and_resident_bytes(11, 2, 1, image_bytes - 1);
        assert!(too_small.queue_profile(
            DynamicPlayerTextureKind::Cape,
            1,
            "https://textures.minecraft.net/texture/oversized",
        ));
        fulfill_requests(&mut too_small, &png, true);
        assert!(too_small.drain_profile_results()[0].texture.is_none());
        let obs = too_small.observation();
        assert_eq!(obs.resident_bytes, 0);
        assert_eq!(obs.resident_rejections, 1);
        assert_eq!(obs.peak_worker_result_bytes, image_bytes);
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
}
