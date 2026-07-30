use super::*;

#[derive(Debug, Clone)]
pub struct NativeDynamicPlayerSkinDownload {
    pub url: String,
    pub skin: Option<DynamicPlayerSkinImage>,
}

#[derive(Debug, Default)]
pub(super) struct LocalDynamicPlayerSkinCache {
    entries: HashMap<String, LocalDynamicPlayerSkinEntry>,
    pending_uploads: Vec<NativeDynamicPlayerSkinDownload>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LocalDynamicPlayerSkinEntry {
    Ready(EntityDynamicPlayerSkin),
    Failed,
}

impl LocalDynamicPlayerSkinEntry {
    const fn skin(self) -> Option<EntityPlayerSkin> {
        match self {
            Self::Ready(skin) => Some(EntityPlayerSkin::Dynamic(skin)),
            Self::Failed => None,
        }
    }
}

impl LocalDynamicPlayerSkinCache {
    fn skin_for_patch(
        &mut self,
        resources: &PackResourceStack,
        profile: &ResolvableProfileSummary,
        patch: &ResourceTextureSummary,
        fallback: EntityDefaultPlayerSkin,
    ) -> Option<EntityPlayerSkin> {
        let source_id = local_player_skin_source_id(&patch.texture_path);
        if let Some(entry) = self.entries.get(&source_id) {
            return entry.skin();
        }

        let model = profile_skin_model(profile, fallback);
        match load_local_dynamic_player_skin(resources, &patch.texture_path, &source_id) {
            Ok(image) => {
                let skin = EntityDynamicPlayerSkin {
                    handle: image.handle,
                    fallback,
                    model,
                    status: EntityDynamicPlayerSkinStatus::Ready,
                };
                self.pending_uploads.push(NativeDynamicPlayerSkinDownload {
                    url: source_id.clone(),
                    skin: Some(image),
                });
                self.entries
                    .insert(source_id, LocalDynamicPlayerSkinEntry::Ready(skin));
                Some(EntityPlayerSkin::Dynamic(skin))
            }
            Err(err) => {
                tracing::warn!(
                    ?err,
                    texture_path = patch.texture_path.as_str(),
                    "failed to load player profile body resource texture patch"
                );
                self.entries
                    .insert(source_id, LocalDynamicPlayerSkinEntry::Failed);
                None
            }
        }
    }

    fn drain_results(&mut self) -> Vec<NativeDynamicPlayerSkinDownload> {
        std::mem::take(&mut self.pending_uploads)
    }
}

#[derive(Debug, Clone)]
pub struct NativeDynamicPlayerTextureDownload {
    pub kind: DynamicPlayerTextureKind,
    pub url: String,
    pub texture: Option<DynamicPlayerTextureImage>,
}

#[derive(Debug, Default)]
pub(super) struct LocalDynamicPlayerTextureCache {
    entries: HashMap<String, LocalDynamicPlayerTextureEntry>,
    pending_uploads: Vec<NativeDynamicPlayerTextureDownload>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LocalDynamicPlayerTextureEntry {
    Ready(EntityDynamicPlayerTexture),
    Failed,
}

impl LocalDynamicPlayerTextureEntry {
    const fn texture(self) -> Option<EntityDynamicPlayerTexture> {
        match self {
            Self::Ready(texture) => Some(texture),
            Self::Failed => None,
        }
    }
}

impl LocalDynamicPlayerTextureCache {
    fn texture_for_patch(
        &mut self,
        resources: &PackResourceStack,
        kind: EntityDynamicPlayerTextureKind,
        patch: &ResourceTextureSummary,
    ) -> Option<EntityDynamicPlayerTexture> {
        let source_id = local_profile_texture_source_id(kind, &patch.texture_path);
        if let Some(entry) = self.entries.get(&source_id) {
            return entry.texture();
        }

        match load_local_dynamic_player_texture(resources, kind, &patch.texture_path, &source_id) {
            Ok((texture, image)) => {
                self.pending_uploads
                    .push(NativeDynamicPlayerTextureDownload {
                        kind: dynamic_player_texture_download_kind(kind),
                        url: source_id.clone(),
                        texture: Some(image),
                    });
                self.entries
                    .insert(source_id, LocalDynamicPlayerTextureEntry::Ready(texture));
                Some(texture)
            }
            Err(err) => {
                tracing::warn!(
                    ?err,
                    texture_path = patch.texture_path.as_str(),
                    "failed to load player profile resource texture patch"
                );
                self.entries
                    .insert(source_id, LocalDynamicPlayerTextureEntry::Failed);
                None
            }
        }
    }

    fn drain_results(&mut self) -> Vec<NativeDynamicPlayerTextureDownload> {
        std::mem::take(&mut self.pending_uploads)
    }
}

pub(super) fn current_epoch_millis() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

pub(super) fn custom_head_skull_for_resource_id(
    resource_id: &str,
    component_patch: &DataComponentPatchSummary,
    profile_resolutions: &RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_textures: &RefCell<Option<Box<AsyncDynamicTextureRuntime>>>,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> Option<EntityCustomHeadSkull> {
    if item_types::vanilla_item_resource_id_matches_type(
        resource_id,
        item_types::VANILLA_ITEM_SKELETON_SKULL_ID,
    ) {
        return Some(EntityCustomHeadSkull::Skeleton);
    }
    if item_types::vanilla_item_resource_id_matches_type(
        resource_id,
        item_types::VANILLA_ITEM_WITHER_SKELETON_SKULL_ID,
    ) {
        return Some(EntityCustomHeadSkull::WitherSkeleton);
    }
    if item_types::vanilla_item_resource_id_matches_type(
        resource_id,
        item_types::VANILLA_ITEM_PLAYER_HEAD_ID,
    ) {
        return custom_head_player_skull(
            component_patch,
            profile_resolutions,
            dynamic_textures,
            profile_texture_resources,
            local_dynamic_skins,
            profile_skins,
        );
    }
    [
        (
            item_types::VANILLA_ITEM_ZOMBIE_HEAD_ID,
            EntityCustomHeadSkull::Zombie,
        ),
        (
            item_types::VANILLA_ITEM_CREEPER_HEAD_ID,
            EntityCustomHeadSkull::Creeper,
        ),
        (
            item_types::VANILLA_ITEM_DRAGON_HEAD_ID,
            EntityCustomHeadSkull::Dragon,
        ),
        (
            item_types::VANILLA_ITEM_PIGLIN_HEAD_ID,
            EntityCustomHeadSkull::Piglin,
        ),
    ]
    .into_iter()
    .find_map(|(item_type_id, skull)| {
        item_types::vanilla_item_resource_id_matches_type(resource_id, item_type_id)
            .then_some(skull)
    })
}

pub(super) fn custom_head_player_skull(
    component_patch: &DataComponentPatchSummary,
    profile_resolutions: &RefCell<Option<AsyncProfileResolutionRuntime>>,
    dynamic_textures: &RefCell<Option<Box<AsyncDynamicTextureRuntime>>>,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> Option<EntityCustomHeadSkull> {
    if !component_patch_has_profile(component_patch) {
        return Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::SlimSteve,
        )));
    }

    let profile = component_patch.profile.as_ref()?;
    let profile = profile_resolutions
        .borrow_mut()
        .as_mut()
        .map(|profile_resolutions| profile_resolutions.resolve_or_queue(profile))
        .unwrap_or_else(|| profile.clone());
    let player_skin = player_skin_for_profile(
        &profile,
        profile_texture_resources,
        local_dynamic_skins,
        profile_skins,
    );
    queue_dynamic_profile_texture_downloads(&profile, player_skin, dynamic_textures);
    Some(EntityCustomHeadSkull::Player(player_skin))
}

pub(super) fn player_skin_for_profile(
    profile: &ResolvableProfileSummary,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_skins: &RefCell<LocalDynamicPlayerSkinCache>,
    profile_skins: &RefCell<ProfileSkinCache>,
) -> EntityPlayerSkin {
    let fallback = default_player_skin_for_profile(profile);
    if let Some(body) = profile.skin_patch.body.as_ref() {
        if EntityDefaultPlayerSkin::from_texture_path(&body.texture_path).is_none() {
            if let Some(skin) = local_dynamic_skins.borrow_mut().skin_for_patch(
                profile_texture_resources,
                profile,
                body,
                fallback,
            ) {
                return skin;
            }
        }
    }
    profile_skins.borrow_mut().player_skin_for_profile(profile)
}

pub(super) fn queue_dynamic_profile_texture_downloads(
    profile: &ResolvableProfileSummary,
    player_skin: EntityPlayerSkin,
    dynamic_textures: &RefCell<Option<Box<AsyncDynamicTextureRuntime>>>,
) {
    let mut dynamic_textures = dynamic_textures.borrow_mut();
    let Some(dynamic_textures) = dynamic_textures.as_mut() else {
        return;
    };
    if let EntityPlayerSkin::Dynamic(skin) = player_skin {
        if skin.status == EntityDynamicPlayerSkinStatus::Loading {
            if let Some(url) = profile
                .profile_textures
                .as_ref()
                .and_then(|textures| textures.skin.as_ref())
                .map(|skin| skin.url.as_str())
            {
                dynamic_textures.queue_skin(skin.handle, url);
            }
        }
    }

    let Some(textures) = profile.profile_textures.as_ref() else {
        return;
    };
    if profile.skin_patch.cape.is_none() {
        if let Some(cape) = textures.cape.as_ref() {
            dynamic_textures.queue_profile(
                DynamicPlayerTextureKind::Cape,
                profile_texture_handle(&cape.url),
                &cape.url,
            );
        }
    }
    if profile.skin_patch.elytra.is_none() {
        if let Some(elytra) = textures.elytra.as_ref() {
            dynamic_textures.queue_profile(
                DynamicPlayerTextureKind::Elytra,
                profile_texture_handle(&elytra.url),
                &elytra.url,
            );
        }
    }
}

pub(super) fn dynamic_player_texture_for_profile(
    profile: &ResolvableProfileSummary,
    kind: EntityDynamicPlayerTextureKind,
    profile_texture_resources: &PackResourceStack,
    local_dynamic_textures: &RefCell<LocalDynamicPlayerTextureCache>,
) -> Option<EntityDynamicPlayerTexture> {
    if let Some(patch) = match kind {
        EntityDynamicPlayerTextureKind::Cape => profile.skin_patch.cape.as_ref(),
        EntityDynamicPlayerTextureKind::Elytra => profile.skin_patch.elytra.as_ref(),
    } {
        return local_dynamic_textures.borrow_mut().texture_for_patch(
            profile_texture_resources,
            kind,
            patch,
        );
    }

    let textures = profile.profile_textures.as_ref()?;
    let url = match kind {
        EntityDynamicPlayerTextureKind::Cape => textures.cape.as_ref()?.url.as_str(),
        EntityDynamicPlayerTextureKind::Elytra => textures.elytra.as_ref()?.url.as_str(),
    };
    Some(EntityDynamicPlayerTexture {
        handle: profile_texture_handle(url),
        kind,
    })
}

pub(super) fn load_local_dynamic_player_skin(
    resources: &PackResourceStack,
    texture_path: &str,
    source_id: &str,
) -> Result<DynamicPlayerSkinImage> {
    let location = ResourceLocation::parse(texture_path).with_context(|| {
        format!("parse player profile body resource texture path {texture_path}")
    })?;
    let resource = resources
        .get_resource(&location)
        .with_context(|| format!("missing player profile body resource texture {texture_path}"))?;
    let image = SpriteImage::from_png_file_with_resource_stack(
        source_id.to_string(),
        resource.path,
        resources,
    )
    .with_context(|| format!("load player profile body resource texture {texture_path}"))?;
    let [width, height] = DynamicPlayerSkinImage::SIZE;
    anyhow::ensure!(
        image.width == width && image.height == height,
        "player profile body resource texture has size {}x{}, expected {}x{}",
        image.width,
        image.height,
        width,
        height
    );
    Ok(DynamicPlayerSkinImage {
        handle: profile_texture_handle(source_id),
        standalone_sampling: Some(StandaloneTextureSampling::new(
            image.texture_metadata.blur,
            image.texture_metadata.clamp,
        )),
        rgba: image.rgba,
    })
}

pub(super) fn local_player_skin_source_id(texture_path: &str) -> String {
    format!("resource:body:{texture_path}")
}

pub(super) fn local_profile_texture_source_id(
    kind: EntityDynamicPlayerTextureKind,
    texture_path: &str,
) -> String {
    let kind = match kind {
        EntityDynamicPlayerTextureKind::Cape => "cape",
        EntityDynamicPlayerTextureKind::Elytra => "elytra",
    };
    format!("resource:{kind}:{texture_path}")
}

pub(super) fn dynamic_player_texture_download_kind(
    kind: EntityDynamicPlayerTextureKind,
) -> DynamicPlayerTextureKind {
    match kind {
        EntityDynamicPlayerTextureKind::Cape => DynamicPlayerTextureKind::Cape,
        EntityDynamicPlayerTextureKind::Elytra => DynamicPlayerTextureKind::Elytra,
    }
}

pub(super) fn profile_skin_model(
    profile: &ResolvableProfileSummary,
    fallback: EntityDefaultPlayerSkin,
) -> EntityPlayerSkinModel {
    profile
        .skin_patch
        .model
        .map(entity_player_skin_model)
        .or_else(|| {
            profile
                .profile_textures
                .as_ref()
                .and_then(|textures| textures.skin.as_ref())
                .map(|skin| entity_player_skin_model(skin.model))
        })
        .unwrap_or_else(|| fallback.model())
}

pub(super) fn component_patch_has_profile(component_patch: &DataComponentPatchSummary) -> bool {
    component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID)
        && !component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID)
}

pub(super) fn world_item_mining_profile(profile: &PackItemMiningProfile) -> WorldItemMiningProfile {
    WorldItemMiningProfile {
        default_mining_speed_thousandths: profile.default_mining_speed_thousandths,
        rules: profile.rules.iter().map(world_item_mining_rule).collect(),
        can_destroy_blocks_in_creative: profile.can_destroy_blocks_in_creative,
    }
}

impl NativeItemRuntime {
    pub fn custom_head_skull_for_stack(
        &self,
        stack: &ItemStackSummary,
    ) -> Option<EntityCustomHeadSkull> {
        let registry = self.registry.as_ref()?;
        let protocol_id = stack.item_id?;
        custom_head_skull_for_resource_id(
            registry.resource_id(protocol_id)?,
            &stack.component_patch,
            &self.profile_resolutions,
            &self.dynamic_textures,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        )
    }

    pub fn player_skin_for_profile(&self, profile: &ResolvableProfileSummary) -> EntityPlayerSkin {
        let profile = self.resolved_or_pending_profile(profile);
        let player_skin = player_skin_for_profile(
            &profile,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        );
        queue_dynamic_profile_texture_downloads(&profile, player_skin, &self.dynamic_textures);
        player_skin
    }

    pub fn player_profile_texture_for_profile(
        &self,
        profile: &ResolvableProfileSummary,
        kind: EntityDynamicPlayerTextureKind,
    ) -> Option<EntityDynamicPlayerTexture> {
        let profile = self.resolved_or_pending_profile(profile);
        let player_skin = player_skin_for_profile(
            &profile,
            &self.profile_texture_resources,
            &self.local_dynamic_skins,
            &self.profile_skins,
        );
        queue_dynamic_profile_texture_downloads(&profile, player_skin, &self.dynamic_textures);
        dynamic_player_texture_for_profile(
            &profile,
            kind,
            &self.profile_texture_resources,
            &self.local_dynamic_textures,
        )
    }

    fn resolved_or_pending_profile(
        &self,
        profile: &ResolvableProfileSummary,
    ) -> ResolvableProfileSummary {
        let mut profile_resolutions = self.profile_resolutions.borrow_mut();
        let Some(profile_resolutions) = profile_resolutions.as_mut() else {
            return profile.clone();
        };
        let drained = profile_resolutions.drain_results();
        let resolved = profile_resolutions.resolve_or_queue(profile);
        if drained > 0 {
            self.advance_profile_presentation_revision();
        }
        resolved
    }

    pub fn enable_http_profile_resolution(&self) {
        let mut profile_resolutions = self.profile_resolutions.borrow_mut();
        if profile_resolutions.is_some() {
            return;
        }
        match HttpGameProfileFetcher::new() {
            Ok(fetcher) => {
                *profile_resolutions = Some(AsyncProfileResolutionRuntime::new(fetcher));
            }
            Err(err) => {
                tracing::warn!(?err, "continuing without async profile resolution");
            }
        }
    }

    pub fn drain_profile_resolution_results(&self) -> usize {
        let drained = self
            .profile_resolutions
            .borrow_mut()
            .as_mut()
            .map(AsyncProfileResolutionRuntime::drain_results)
            .unwrap_or(0);
        if drained > 0 {
            self.advance_profile_presentation_revision();
        }
        drained
    }

    pub fn enable_http_player_skin_downloads(&self, cache_dir: impl Into<PathBuf>) {
        let cache_dir = cache_dir.into();
        let mut dynamic_textures = self.dynamic_textures.borrow_mut();
        if dynamic_textures.is_none() {
            match HttpSkinPngFetcher::new() {
                Ok(fetcher) => {
                    let mut runtime = AsyncDynamicTextureRuntime::new(
                        self.resource_generation,
                        cache_dir,
                        fetcher,
                    );
                    runtime.reset_session_generation(
                        self.dynamic_texture_session_generation.get(),
                        self.dynamic_texture_session
                            .get()
                            .map_or(true, |(_, connected)| connected),
                    );
                    *dynamic_textures = Some(Box::new(runtime));
                }
                Err(err) => {
                    tracing::warn!(?err, "continuing without async player texture downloads");
                }
            }
        }
    }

    pub fn drain_dynamic_player_skin_download_results(
        &self,
    ) -> Vec<NativeDynamicPlayerSkinDownload> {
        let mut local_results = self.local_dynamic_skins.borrow_mut().drain_results();
        let results = self
            .dynamic_textures
            .borrow_mut()
            .as_mut()
            .map(|runtime| runtime.drain_skin_results())
            .unwrap_or_default();
        for result in &results {
            if result.skin.is_none() {
                self.mark_profile_skin_failed(&result.url);
            }
        }
        local_results.extend(
            results
                .into_iter()
                .map(|result| NativeDynamicPlayerSkinDownload {
                    url: result.url,
                    skin: result.skin,
                }),
        );
        local_results
    }

    pub fn drain_dynamic_player_texture_download_results(
        &self,
    ) -> Vec<NativeDynamicPlayerTextureDownload> {
        let mut results = self.local_dynamic_textures.borrow_mut().drain_results();
        results.extend(
            self.dynamic_textures
                .borrow_mut()
                .as_mut()
                .map(|runtime| runtime.drain_profile_results())
                .unwrap_or_default()
                .into_iter()
                .map(|result| NativeDynamicPlayerTextureDownload {
                    kind: result.kind,
                    url: result.url,
                    texture: result.texture,
                }),
        );
        results
    }

    #[cfg(test)]
    pub(super) fn enable_dynamic_texture_downloads_for_test(
        &self,
        runtime: AsyncDynamicTextureRuntime,
    ) {
        *self.dynamic_textures.borrow_mut() = Some(Box::new(runtime));
    }

    #[cfg(test)]
    pub(super) fn downloaded_player_skin_count(&self) -> usize {
        self.dynamic_textures
            .borrow()
            .as_ref()
            .map(|runtime| runtime.downloaded_skin_count())
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(super) fn downloaded_player_texture_count(&self) -> usize {
        self.dynamic_textures
            .borrow()
            .as_ref()
            .map(|runtime| runtime.downloaded_texture_count())
            .unwrap_or(0)
    }

    pub fn mark_profile_skin_resolved(&self, url: &str, texture_handle: u64) {
        self.profile_skins
            .borrow_mut()
            .mark_resolved(url, texture_handle);
        self.advance_profile_presentation_revision();
    }

    pub fn mark_profile_skin_failed(&self, url: &str) {
        self.profile_skins.borrow_mut().mark_failed(url);
        self.advance_profile_presentation_revision();
    }

    pub fn profile_presentation_revision(&self) -> u64 {
        self.profile_presentation_revision.get()
    }

    pub fn dynamic_texture_resource_generation(&self) -> u64 {
        self.resource_generation
    }

    pub fn dynamic_texture_cache_observation(&self) -> DynamicTextureCacheObservation {
        self.dynamic_textures
            .borrow_mut()
            .as_mut()
            .map(|runtime| runtime.observation())
            .unwrap_or(DynamicTextureCacheObservation {
                resource_generation: self.resource_generation,
                session_generation: self.dynamic_texture_session_generation.get(),
                accepting_requests: false,
                ..DynamicTextureCacheObservation::default()
            })
    }

    /// Aligns remote and pack-backed dynamic texture caches with the exact
    /// network session visible in the immutable publication. A disconnect is
    /// a distinct owner boundary even before the next connection increments
    /// `connection_generation`, so late worker results cannot revive handles
    /// from the previous level/session.
    pub fn synchronize_dynamic_texture_session(
        &self,
        connection_generation: u64,
        connected: bool,
    ) -> bool {
        let next = (connection_generation, connected);
        if self.dynamic_texture_session.replace(Some(next)) == Some(next) {
            return false;
        }

        let generation = self
            .dynamic_texture_session_generation
            .get()
            .saturating_add(1);
        self.dynamic_texture_session_generation.set(generation);
        self.advance_profile_presentation_revision();
        *self.profile_skins.borrow_mut() = ProfileSkinCache::default();
        *self.local_dynamic_skins.borrow_mut() = LocalDynamicPlayerSkinCache::default();
        *self.local_dynamic_textures.borrow_mut() = LocalDynamicPlayerTextureCache::default();
        if let Some(runtime) = self.dynamic_textures.borrow_mut().as_mut() {
            runtime.reset_session_generation(generation, connected);
        }
        true
    }

    fn advance_profile_presentation_revision(&self) {
        self.profile_presentation_revision
            .set(self.profile_presentation_revision.get().saturating_add(1));
    }

    pub fn item_mining_profiles_by_protocol_id(&self) -> BTreeMap<i32, WorldItemMiningProfile> {
        let mut profiles = BTreeMap::new();
        let Some(registry) = &self.registry else {
            return profiles;
        };
        for (protocol_id, resource_id) in registry.resource_ids().iter().enumerate() {
            let Some(profile) = registry.mining_profile(resource_id) else {
                continue;
            };
            profiles.insert(protocol_id as i32, world_item_mining_profile(profile));
        }
        profiles
    }

    pub fn item_mining_profile_count(&self) -> usize {
        self.item_mining_profiles_by_protocol_id().len()
    }
}
