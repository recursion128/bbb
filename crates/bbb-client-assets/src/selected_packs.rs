use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use bbb_protocol::content_registry::{
    ContentRegistry, ModBlockSpec, ModItemSpec, ModParticleSpec, ModSoundEventSpec,
};
use bbb_render_types::wasm_sync::SpinRwLock;
use sha1::{Digest, Sha1};

use crate::{
    atlas_sources::{decode_atlas_texture_images, load_atlas_texture_entries_with_budget},
    block_models::{load_cuboid_model_resources, RawBlockModel},
    content_assets::{
        project_content_registry_from_generation, project_single_mod_block,
        project_single_mod_item, project_single_mod_particle, project_single_mod_sound_event,
        ContentRegistryAssetProjection, ModBlockAssetProjection, ModItemAssetProjection,
        ModParticleAssetProjection, ModSoundEventAssetProjection,
    },
    prepare_pack_archive, read_client_resource_pack_metadata, ClientPackMetadata,
    PackResourceStack, PackRoots, PreparedPackArchive, ServerPackFormatRange, SoundEventRegistry,
    SpriteImage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPackCompatibility {
    Compatible,
    TooOld,
    TooNew,
}

impl ClientPackCompatibility {
    fn for_supported_formats(supported_formats: ServerPackFormatRange) -> Self {
        if supported_formats.supports_current_client() {
            Self::Compatible
        } else if supported_formats.max < crate::server_pack::client_resource_pack_format() {
            Self::TooOld
        } else {
            Self::TooNew
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPackPosition {
    Bottom,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientPackSelection {
    pub required: bool,
    pub default_position: ClientPackPosition,
    pub fixed_position: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientPackSource {
    Vanilla,
    BuiltIn { id: String },
    Startup { argument_index: usize },
    OptionsRepository { file_name: String },
    Server { pack_id: String, push_sequence: u64 },
    Unmanaged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedClientPack {
    pub id: String,
    pub source: ClientPackSource,
    pub root: PathBuf,
    pub supported_formats: ServerPackFormatRange,
    pub compatibility: ClientPackCompatibility,
    pub selection: ClientPackSelection,
    pub metadata: Option<ClientPackMetadata>,
    pub(crate) indexed_vanilla_assets: Option<Arc<crate::asset_index::VanillaAssetIndex>>,
    pub(crate) archive: Option<PreparedPackArchive>,
}

impl SelectedClientPack {
    pub(crate) fn vanilla(roots: &PackRoots) -> Self {
        let current = crate::server_pack::client_resource_pack_format();
        Self {
            id: "vanilla".to_string(),
            source: ClientPackSource::Vanilla,
            root: roots.sources_dir.clone(),
            supported_formats: ServerPackFormatRange {
                min: current,
                max: current,
            },
            compatibility: ClientPackCompatibility::Compatible,
            selection: ClientPackSelection {
                required: true,
                default_position: ClientPackPosition::Bottom,
                fixed_position: false,
            },
            metadata: roots
                .vanilla_archive()
                .map(|archive| archive.metadata.clone()),
            indexed_vanilla_assets: roots.indexed_vanilla_assets(),
            archive: roots.vanilla_archive().cloned(),
        }
    }

    pub fn startup(argument_index: usize, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let is_archive_file = if crate::virtual_store::is_virtual_path(&root) {
            crate::virtual_store::has_virtual_file(&root)
        } else {
            root.is_file()
        };
        if is_archive_file {
            let cache_root = startup_archive_cache_root(&root);
            let archive = prepare_pack_archive(&root, &cache_root)?;
            let metadata = archive.metadata.clone();
            let supported_formats = metadata.supported_formats;
            return Ok(Self {
                id: format!("startup/{argument_index}"),
                source: ClientPackSource::Startup { argument_index },
                root,
                supported_formats,
                compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
                selection: ClientPackSelection {
                    required: false,
                    default_position: ClientPackPosition::Top,
                    fixed_position: false,
                },
                metadata: Some(metadata),
                indexed_vanilla_assets: None,
                archive: Some(archive),
            });
        }
        validate_pack_root(&root)?;
        let metadata = read_client_resource_pack_metadata(&root)?;
        let supported_formats = metadata.supported_formats;
        Ok(Self {
            id: format!("startup/{argument_index}"),
            source: ClientPackSource::Startup { argument_index },
            root,
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: false,
                default_position: ClientPackPosition::Top,
                fixed_position: false,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: None,
        })
    }

    fn built_in_archive(id: String, archive: PreparedPackArchive) -> Self {
        let metadata = archive.metadata.clone();
        let supported_formats = metadata.supported_formats;
        Self {
            id: id.clone(),
            source: ClientPackSource::BuiltIn { id },
            root: archive.archive_path.clone(),
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: false,
                default_position: ClientPackPosition::Top,
                fixed_position: false,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: Some(archive),
        }
    }

    fn options_repository(file_name: String, root: PathBuf) -> Result<Self> {
        validate_pack_root(&root)?;
        let metadata = read_client_resource_pack_metadata(&root)?;
        Self::options_repository_with_metadata(file_name, root, metadata)
    }

    fn options_repository_with_metadata(
        file_name: String,
        root: PathBuf,
        metadata: ClientPackMetadata,
    ) -> Result<Self> {
        let supported_formats = metadata.supported_formats;
        Ok(Self {
            id: format!("file/{file_name}"),
            source: ClientPackSource::OptionsRepository { file_name },
            root,
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: false,
                default_position: ClientPackPosition::Top,
                fixed_position: false,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: None,
        })
    }

    fn options_repository_archive(file_name: String, archive: PreparedPackArchive) -> Result<Self> {
        let metadata = archive.metadata.clone();
        let supported_formats = metadata.supported_formats;
        Ok(Self {
            id: format!("file/{file_name}"),
            source: ClientPackSource::OptionsRepository { file_name },
            root: archive.archive_path.clone(),
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: false,
                default_position: ClientPackPosition::Top,
                fixed_position: false,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: Some(archive),
        })
    }

    pub fn server(
        pack_id: impl Into<String>,
        push_sequence: u64,
        load_serial: u32,
        root: impl Into<PathBuf>,
        metadata: ClientPackMetadata,
    ) -> Result<Self> {
        let pack_id = pack_id.into();
        let root = root.into();
        if pack_id.is_empty() {
            bail!("server client pack id cannot be empty");
        }
        let supported_formats = metadata.supported_formats;
        Ok(Self {
            id: format!("server/{load_serial:08X}/{pack_id}"),
            source: ClientPackSource::Server {
                pack_id,
                push_sequence,
            },
            root,
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: true,
                default_position: ClientPackPosition::Top,
                fixed_position: true,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: None,
        })
    }

    pub fn server_archive(
        pack_id: impl Into<String>,
        push_sequence: u64,
        load_serial: u32,
        archive: PreparedPackArchive,
    ) -> Result<Self> {
        let pack_id = pack_id.into();
        if pack_id.is_empty() {
            bail!("server client pack id cannot be empty");
        }
        let metadata = archive.metadata.clone();
        let supported_formats = metadata.supported_formats;
        Ok(Self {
            id: format!("server/{load_serial:08X}/{pack_id}"),
            source: ClientPackSource::Server {
                pack_id,
                push_sequence,
            },
            root: archive.archive_path.clone(),
            supported_formats,
            compatibility: ClientPackCompatibility::for_supported_formats(supported_formats),
            selection: ClientPackSelection {
                required: true,
                default_position: ClientPackPosition::Top,
                fixed_position: true,
            },
            metadata: Some(metadata),
            indexed_vanilla_assets: None,
            archive: Some(archive),
        })
    }

    pub(crate) fn unmanaged(id: String, root: PathBuf) -> Self {
        let current = crate::server_pack::client_resource_pack_format();
        Self {
            id,
            source: ClientPackSource::Unmanaged,
            root,
            supported_formats: ServerPackFormatRange {
                min: current,
                max: current,
            },
            compatibility: ClientPackCompatibility::Compatible,
            selection: ClientPackSelection {
                required: false,
                default_position: ClientPackPosition::Top,
                fixed_position: false,
            },
            metadata: None,
            indexed_vanilla_assets: None,
            archive: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedClientPackStack {
    packs: Vec<SelectedClientPack>,
}

/// The in-memory mutations made by Vanilla `Options.loadSelectedResourcePacks`.
/// The selected stack uses canonical repository ids, while the persisted list
/// deliberately keeps a legacy id without `file/` when that fallback resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientPackOptionsReconciliation {
    pub selected_packs: SelectedClientPackStack,
    pub resource_packs: Vec<String>,
    pub incompatible_resource_packs: Vec<String>,
    pub removed_missing: Vec<String>,
    pub removed_incompatible: Vec<String>,
    pub became_compatible: Vec<String>,
}

struct ClearableCache<T> {
    slot: SpinRwLock<Option<Arc<T>>>,
}

impl<T> fmt::Debug for ClearableCache<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClearableCache").finish_non_exhaustive()
    }
}

impl<T> Default for ClearableCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn get_or_try_init_spin<T, F>(slot: &SpinRwLock<Option<Arc<T>>>, init: F) -> Result<Arc<T>>
where
    F: FnOnce() -> Result<Arc<T>>,
{
    if let Some(existing) = slot.read().as_ref() {
        return Ok(Arc::clone(existing));
    }
    // Run I/O and decoding outside the spin lock so readers/writers never contend on work.
    let loaded = init()?;
    let (winner, loser) = {
        let mut guard = slot.write();
        if let Some(existing) = guard.as_ref() {
            (Arc::clone(existing), Some(loaded))
        } else {
            *guard = Some(Arc::clone(&loaded));
            (loaded, None)
        }
    };
    drop(loser);
    Ok(winner)
}

impl<T> ClearableCache<T> {
    fn new() -> Self {
        Self {
            slot: SpinRwLock::new(None),
        }
    }

    fn get_or_try_init<F>(&self, init: F) -> Result<Arc<T>>
    where
        F: FnOnce() -> Result<Arc<T>>,
    {
        get_or_try_init_spin(&self.slot, init)
    }

    fn clear(&self) {
        let old = self.slot.write().take();
        drop(old);
    }
}

type CachedCuboidModels = Arc<SpinRwLock<Option<Arc<HashMap<String, RawBlockModel>>>>>;

#[derive(Debug, Clone)]
pub struct ClientResourceGeneration {
    generation: u64,
    selected_packs: Arc<SelectedClientPackStack>,
    resources: PackResourceStack,
    // Non-blocking spin-locked lazy caches so wasm32 main thread never calls Atomics.wait.
    cuboid_models: CachedCuboidModels,
    block_texture_images: Arc<ClearableCache<Vec<SpriteImage>>>,
}

impl ClientResourceGeneration {
    pub fn prepare(generation: u64, selected_packs: SelectedClientPackStack) -> Result<Self> {
        Self::prepare_with_limits(
            generation,
            selected_packs,
            crate::image::MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
            crate::resource_budget::MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
        )
    }

    pub(crate) fn prepare_with_limits(
        generation: u64,
        selected_packs: SelectedClientPackStack,
        max_resource_image_rgba_bytes: u64,
        max_resource_data_logical_bytes: u64,
    ) -> Result<Self> {
        let data_budget = crate::resource_budget::ResourceDataGenerationBudget::new(
            max_resource_data_logical_bytes,
        );
        for pack in selected_packs.packs() {
            if let Some(metadata) = &pack.metadata {
                data_budget.admit_preparsed_json(
                    metadata.source_bytes(),
                    metadata.document(),
                    &format!("selected pack metadata {}", pack.id),
                )?;
            }
        }
        let resources = PackResourceStack::prepare_selected_client_packs_with_budgets(
            selected_packs.packs(),
            crate::image::ResourceImageGenerationBudget::new(max_resource_image_rgba_bytes),
            data_budget,
        )?;
        Ok(Self {
            generation,
            selected_packs: Arc::new(selected_packs),
            resources,
            cuboid_models: Arc::new(SpinRwLock::new(None)),
            block_texture_images: Arc::new(ClearableCache::new()),
        })
    }

    pub(crate) fn cuboid_models(&self) -> Result<Arc<HashMap<String, RawBlockModel>>> {
        get_or_try_init_spin(&self.cuboid_models, || {
            Ok(Arc::new(load_cuboid_model_resources(&self.resources)?))
        })
    }

    pub(crate) fn block_texture_images(&self) -> Result<Arc<Vec<SpriteImage>>> {
        self.block_texture_images.get_or_try_init(|| {
            let mut budget = self.resources.atlas_png_decode_budget();
            let entries =
                load_atlas_texture_entries_with_budget(&self.resources, "blocks", &mut budget)?;
            Ok(Arc::new(decode_atlas_texture_images(
                entries,
                &mut budget,
                &self.resources,
            )?))
        })
    }

    /// Drops the cached decoded `"blocks"` atlas sprite images once terrain and item
    /// atlases have finished stitching, reclaiming steady-state CPU RGBA memory.
    pub fn release_block_texture_images(&self) {
        self.block_texture_images.clear();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn selected_packs(&self) -> &SelectedClientPackStack {
        &self.selected_packs
    }

    pub fn resources(&self) -> &PackResourceStack {
        &self.resources
    }

    pub fn project_content_registry(
        &self,
        content_registry: &ContentRegistry,
    ) -> Result<ContentRegistryAssetProjection> {
        project_content_registry_from_generation(self, content_registry)
    }

    pub fn project_mod_block(&self, spec: &ModBlockSpec) -> Result<ModBlockAssetProjection> {
        project_single_mod_block(self, spec)
    }

    pub fn project_mod_item(&self, spec: &ModItemSpec) -> Result<ModItemAssetProjection> {
        project_single_mod_item(self, spec)
    }

    pub fn project_mod_sound_event(
        &self,
        spec: &ModSoundEventSpec,
    ) -> Result<ModSoundEventAssetProjection> {
        project_single_mod_sound_event(self, spec)
    }

    pub fn project_mod_particle(
        &self,
        spec: &ModParticleSpec,
    ) -> Result<ModParticleAssetProjection> {
        project_single_mod_particle(self, spec)
    }

    pub fn sound_event_registry(&self, content_registry: &ContentRegistry) -> SoundEventRegistry {
        SoundEventRegistry::with_content_registry(content_registry)
    }
}

impl SelectedClientPackStack {
    pub fn with_startup_pack(&self, root: PathBuf) -> Result<Self> {
        let mut packs = self.packs.clone();
        packs.push(SelectedClientPack::startup(packs.len() - 1, root)?);
        Self::new(packs)
    }

    pub fn from_startup_dirs(
        roots: &PackRoots,
        dirs: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self> {
        let mut packs = vec![SelectedClientPack::vanilla(roots)];
        for (argument_index, root) in dirs.into_iter().enumerate() {
            packs.push(
                SelectedClientPack::startup(argument_index, root)
                    .with_context(|| format!("admit --resource-pack-dir #{argument_index}"))?,
            );
        }
        Self::new(packs)
    }

    /// Discover selected file packs from the Vanilla `resourcepacks` repository
    /// and replay `Options.loadSelectedResourcePacks` against the two stored
    /// lists. Invalid repository entries are unavailable, matching
    /// `Pack.readMetaAndCreate` returning null. Selected ZIPs remain the canonical backing store;
    /// their repository id remains `file/<archive-name>`.
    pub fn from_options_repository(
        roots: &PackRoots,
        repository: &Path,
        resource_packs: &[String],
        incompatible_resource_packs: &[String],
    ) -> Result<ClientPackOptionsReconciliation> {
        let mut available = BTreeMap::new();
        discover_builtin_optional_packs(roots, resource_packs, &mut available);
        if !crate::virtual_store::is_virtual_path(repository) {
            fs::create_dir_all(repository).with_context(|| {
                format!("create resource pack repository {}", repository.display())
            })?;
            let requested_file_names = resource_packs
                .iter()
                .filter(|id| id.as_str() != "vanilla")
                .map(|id| id.strip_prefix("file/").unwrap_or(id))
                .collect::<BTreeSet<_>>();
            let archive_cache = repository
                .parent()
                .unwrap_or(repository)
                .join(".bbb")
                .join("resource-pack-cache");
            for entry in fs::read_dir(repository).with_context(|| {
                format!("list resource pack repository {}", repository.display())
            })? {
                let Ok(entry) = entry else {
                    continue;
                };
                let path = entry.path();
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !requested_file_names.contains(file_name.as_str()) {
                    continue;
                }
                let pack = if file_type.is_dir() {
                    SelectedClientPack::options_repository(file_name, path)
                } else if file_type.is_file() && file_name.ends_with(".zip") {
                    prepare_options_repository_archive(&path, &archive_cache).and_then(|archive| {
                        SelectedClientPack::options_repository_archive(file_name, archive)
                    })
                } else {
                    continue;
                };
                let Ok(pack) = pack else {
                    continue;
                };
                available.insert(pack.id.clone(), pack);
            }
        }

        let mut packs = vec![SelectedClientPack::vanilla(roots)];
        let mut selected_ids = BTreeSet::new();
        selected_ids.insert("vanilla".to_owned());
        let mut retained = Vec::new();
        let mut incompatible = incompatible_resource_packs.to_vec();
        let mut removed_missing = Vec::new();
        let mut removed_incompatible = Vec::new();
        let mut became_compatible = Vec::new();

        for stored_id in resource_packs {
            let pack = if stored_id == "vanilla" {
                None
            } else {
                available.get(stored_id).or_else(|| {
                    (!stored_id.starts_with("file/"))
                        .then(|| available.get(&format!("file/{stored_id}")))
                        .flatten()
                })
            };
            if stored_id != "vanilla" && pack.is_none() {
                removed_missing.push(stored_id.clone());
                continue;
            }

            let compatibility = pack
                .map(|pack| pack.compatibility)
                .unwrap_or(ClientPackCompatibility::Compatible);
            let accepted_incompatible = incompatible.iter().any(|id| id == stored_id);
            if compatibility != ClientPackCompatibility::Compatible && !accepted_incompatible {
                removed_incompatible.push(stored_id.clone());
                continue;
            }
            if compatibility == ClientPackCompatibility::Compatible && accepted_incompatible {
                if let Some(index) = incompatible.iter().position(|id| id == stored_id) {
                    incompatible.remove(index);
                }
                became_compatible.push(stored_id.clone());
            }

            retained.push(stored_id.clone());
            if let Some(pack) = pack {
                if selected_ids.insert(pack.id.clone()) {
                    packs.push(pack.clone());
                }
            }
        }

        Ok(ClientPackOptionsReconciliation {
            selected_packs: Self::new(packs)?,
            resource_packs: retained,
            incompatible_resource_packs: incompatible,
            removed_missing,
            removed_incompatible,
            became_compatible,
        })
    }

    pub fn with_server_packs(
        &self,
        server_packs_in_repository_order: impl IntoIterator<Item = SelectedClientPack>,
    ) -> Result<Self> {
        let mut packs = self
            .packs
            .iter()
            .filter(|pack| !matches!(pack.source, ClientPackSource::Server { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let mut previous_push_sequence = None;
        let mut server_ids = std::collections::BTreeSet::new();
        for pack in server_packs_in_repository_order {
            let ClientPackSource::Server {
                ref pack_id,
                push_sequence,
            } = pack.source
            else {
                bail!("non-server pack {} supplied as a server pack", pack.id);
            };
            if previous_push_sequence.is_some_and(|previous| previous <= push_sequence) {
                bail!("server client packs are not in strict reverse-push repository order");
            }
            if !server_ids.insert(pack_id.clone()) {
                bail!("duplicate active server client pack id {pack_id}");
            }
            if pack.metadata.is_none() {
                bail!("active server client pack {pack_id} is missing metadata");
            }
            previous_push_sequence = Some(push_sequence);
            packs.push(pack);
        }
        Self::new(packs)
    }

    pub fn packs(&self) -> &[SelectedClientPack] {
        &self.packs
    }

    pub(crate) fn from_unmanaged_roots(roots: &PackRoots) -> Self {
        let mut packs = vec![SelectedClientPack::vanilla(roots)];
        packs.extend(
            roots
                .resource_pack_dirs()
                .iter()
                .enumerate()
                .map(|(index, root)| {
                    SelectedClientPack::unmanaged(format!("unmanaged/{index}"), root.clone())
                }),
        );
        Self { packs }
    }

    fn new(packs: Vec<SelectedClientPack>) -> Result<Self> {
        let Some(vanilla) = packs.first() else {
            bail!("selected client pack stack is missing vanilla");
        };
        if !matches!(vanilla.source, ClientPackSource::Vanilla) {
            bail!("selected client pack stack must start with vanilla");
        }
        let mut ids = std::collections::BTreeSet::new();
        for pack in &packs {
            if !ids.insert(pack.id.as_str()) {
                bail!("duplicate selected client pack id {}", pack.id);
            }
            if matches!(
                pack.source,
                ClientPackSource::BuiltIn { .. }
                    | ClientPackSource::Startup { .. }
                    | ClientPackSource::OptionsRepository { .. }
                    | ClientPackSource::Server { .. }
            ) {
                let metadata = pack.metadata.as_ref().with_context(|| {
                    format!("selected client pack {} is missing metadata", pack.id)
                })?;
                if metadata.supported_formats != pack.supported_formats {
                    bail!("selected client pack {} metadata format mismatch", pack.id);
                }
            }
        }
        Ok(Self { packs })
    }
}

const BUILTIN_OPTIONAL_PACK_IDS: [&str; 2] = ["programmer_art", "high_contrast"];
const MAX_BUILTIN_PACK_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;

fn discover_builtin_optional_packs(
    roots: &PackRoots,
    requested_packs: &[String],
    available: &mut BTreeMap<String, SelectedClientPack>,
) {
    let requested_builtins = requested_packs
        .iter()
        .filter(|id| BUILTIN_OPTIONAL_PACK_IDS.contains(&id.as_str()))
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if requested_builtins.is_empty() {
        return;
    }
    let Ok(vanilla_stack) =
        PackResourceStack::prepare_selected_client_packs(&[SelectedClientPack::vanilla(roots)])
    else {
        return;
    };
    for id in requested_builtins {
        let Ok(location) =
            crate::ResourceLocation::new("minecraft", format!("resourcepacks/{id}.zip"))
        else {
            continue;
        };
        let Some(resource) = vanilla_stack.get_resource(&location) else {
            continue;
        };
        if vanilla_stack
            .validate_resource(&resource, "built-in resource pack")
            .is_err()
        {
            continue;
        }
        let prepared = if resource.is_archive_backed() {
            let Ok(bytes) = resource
                .read_bounded_bytes(MAX_BUILTIN_PACK_ARCHIVE_BYTES, "built-in resource pack")
            else {
                continue;
            };
            let virtual_archive = PathBuf::from(format!("/__bbb_mem__/builtin-packs/{id}.zip"));
            let virtual_cache = PathBuf::from(format!("/__bbb_mem__/builtin-packs/{id}-cache"));
            crate::virtual_store::insert_virtual_file(&virtual_archive, Arc::from(bytes));
            prepare_pack_archive(&virtual_archive, &virtual_cache)
        } else {
            let cache_root = if crate::virtual_store::is_virtual_path(&resource.path) {
                PathBuf::from(format!("/__bbb_mem__/builtin-packs/{id}-cache"))
            } else {
                std::env::temp_dir()
                    .join(format!("bbb-builtin-pack-cache-{}", crate::MC_VERSION))
                    .join(id)
            };
            prepare_pack_archive(&resource.path, &cache_root)
        };
        if let Ok(archive) = prepared {
            available.insert(
                id.to_owned(),
                SelectedClientPack::built_in_archive(id.to_owned(), archive),
            );
        }
    }
}

fn startup_archive_cache_root(archive: &Path) -> PathBuf {
    if crate::virtual_store::is_virtual_path(archive) {
        return archive.join("cache");
    }
    std::env::temp_dir()
        .join(format!("bbb-startup-pack-cache-{}", crate::MC_VERSION))
        .join(archive_identity_digest(archive))
}

fn prepare_options_repository_archive(
    archive: &Path,
    cache_root: &Path,
) -> Result<PreparedPackArchive> {
    let output = cache_root.join(archive_identity_digest(archive));
    prepare_pack_archive(archive, &output)
}

fn archive_identity_digest(archive: &Path) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(archive.to_string_lossy().as_bytes());
    if let Ok(metadata) = fs::metadata(archive) {
        sha1.update(metadata.len().to_le_bytes());
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.duration_since(std::time::UNIX_EPOCH) {
                sha1.update(elapsed.as_nanos().to_le_bytes());
            }
        }
    }
    crate::hex_lower(&sha1.finalize())
}

fn validate_pack_root(root: &Path) -> Result<()> {
    if !root.is_dir() {
        bail!("client pack root is not a directory: {}", root.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceLocation;
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    #[test]
    fn prepared_resource_generation_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ClientResourceGeneration>();
    }

    #[test]
    fn generation_image_budget_is_shared_by_root_clones_and_decode_scopes() {
        let root = unique_temp_dir("generation-image-budget");
        let sources = root.join("sources/26.1");
        write_png(
            &sources.join("assets/minecraft/textures/item/widget.png"),
            [1, 2, 3, 255],
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots
            .with_client_resource_generation_image_limit(7, selected, 4)
            .unwrap();
        let clone = roots.clone();

        let image = roots
            .load_resource_texture_image("minecraft:item/widget")
            .unwrap();
        assert_eq!(image.size, [1, 1]);
        assert_eq!(
            roots.resource_image_memory_observation(),
            crate::ResourceImageMemoryObservation {
                max_rgba_bytes: 4,
                charged_rgba_bytes: 4,
                peak_charged_rgba_bytes: 4,
            }
        );
        let err = clone
            .load_resource_texture_image("minecraft:item/widget")
            .unwrap_err()
            .to_string();
        assert!(err.contains("currently charged 4, requested 4"), "{err}");
        assert_eq!(
            clone.resource_image_memory_observation(),
            roots.resource_image_memory_observation()
        );

        let next = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&next, []).unwrap();
        let next = next
            .with_client_resource_generation_image_limit(8, selected, 4)
            .unwrap();
        assert_eq!(
            next.resource_image_memory_observation().charged_rgba_bytes,
            0
        );
        assert!(next
            .load_resource_texture_image("minecraft:item/widget")
            .is_ok());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generation_data_budget_charges_selected_metadata_and_is_shared_by_clones() {
        let root = unique_temp_dir("generation-data-budget");
        let sources = root.join("sources/26.1");
        std::fs::create_dir_all(&sources).unwrap();
        let pack = root.join("pack");
        write_metadata(&pack, 84);
        write_file(
            &pack.join("assets/minecraft/test/value.json"),
            br#"{"value":"shared"}"#,
        );
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, [pack]).unwrap();
        let roots = roots
            .with_client_resource_generation_limits(7, selected, 4, 4_096)
            .unwrap();
        let metadata_charge = roots.resource_data_memory_observation();
        assert_eq!(metadata_charge.json_documents, 1);
        assert!(metadata_charge.json_source_bytes > 0);

        let clone = roots.clone();
        let resource = roots
            .resource_stack()
            .get_resource(&ResourceLocation::parse("minecraft:test/value.json").unwrap())
            .unwrap();
        let _: serde_json::Value = clone
            .resource_stack()
            .read_json_resource(&resource, "shared fixture")
            .unwrap();
        let after = roots.resource_data_memory_observation();
        assert_eq!(after.json_documents, 2);
        assert!(after.charged_logical_bytes > metadata_charge.charged_logical_bytes);
        assert_eq!(after, clone.resource_data_memory_observation());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_selected_generation_composes_custom_model_texture_font_sound_and_data_resources() {
        let root = unique_temp_dir("custom-resource-composition");
        let sources = root.join("sources/26.1");
        let low = root.join("packs/low");
        let high = root.join("packs/high");
        std::fs::create_dir_all(sources.join("assets/minecraft")).unwrap();
        write_metadata(&low, 84);
        write_metadata(&high, 84);
        write_file(
            &low.join("assets/example/items/widget.json"),
            br#"{"model":{"type":"minecraft:model","model":"example:item/low"}}"#,
        );
        write_file(
            &high.join("assets/example/items/widget.json"),
            br#"{"model":{"type":"minecraft:model","model":"example:item/high"}}"#,
        );
        write_file(
            &low.join("assets/example/models/item/widget.json"),
            br#"{"gui_light":"side"}"#,
        );
        write_file(
            &high.join("assets/example/models/item/widget.json"),
            br#"{"parent":"example:shared/widget"}"#,
        );
        write_file(
            &low.join("assets/example/models/shared/widget.json"),
            br#"{"gui_light":"side","textures":{"particle":"example:item/low"}}"#,
        );
        write_file(
            &high.join("assets/example/models/shared/widget.json"),
            br#"{"gui_light":"front","textures":{"particle":"example:item/widget"}}"#,
        );
        write_file(
            &high.join("assets/example/blockstates/widget.json"),
            br#"{"variants":{"":{"model":"example:shared/widget"}}}"#,
        );
        write_png(
            &low.join("assets/example/textures/item/widget.png"),
            [1, 2, 3, 255],
        );
        write_png(
            &high.join("assets/example/textures/item/widget.png"),
            [7, 11, 13, 255],
        );
        write_file(
            &low.join("assets/example/font/default.json"),
            br#"{"providers":[{"type":"space","advances":{" ":3}}]}"#,
        );
        write_file(
            &high.join("assets/example/font/default.json"),
            br#"{"providers":[{"type":"space","advances":{" ":5}}]}"#,
        );
        write_file(
            &low.join("assets/example/sounds.json"),
            br#"{"widget":{"sounds":[{"type":"event","name":"example:low"}]}}"#,
        );
        write_file(
            &high.join("assets/example/sounds.json"),
            br#"{"widget":{"replace":true,"sounds":[{"type":"event","name":"example:high"}]}}"#,
        );
        write_file(
            &low.join("data/example/test_registry/widget.json"),
            br#"{"source":"low"}"#,
        );
        write_file(
            &high.join("data/example/test_registry/widget.json"),
            br#"{"source":"high"}"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, [low, high]).unwrap();
        let roots = roots.with_client_resource_generation(37, selected).unwrap();
        let resources = roots.resource_stack();

        for id in ["example:font/default.json", "example:sounds.json"] {
            let resource = resources
                .get_resource(&crate::ResourceLocation::parse(id).unwrap())
                .unwrap();
            assert_eq!(resource.pack_id, "startup/1");
        }
        assert_eq!(
            roots
                .load_item_model_catalog()
                .unwrap()
                .model_references("example:widget")
                .unwrap(),
            vec!["example:item/high"]
        );
        let block = roots
            .clone()
            .load_block_model_catalog()
            .unwrap()
            .block_render_model("example:widget", &BTreeMap::new())
            .unwrap();
        assert_eq!(block.gui_light, crate::BlockModelGuiLight::Front);
        assert_eq!(
            block.particle_texture.as_deref(),
            Some("example:item/widget")
        );
        assert_eq!(
            roots
                .load_item_cuboid_model_catalog()
                .unwrap()
                .model("example:item/widget")
                .unwrap()
                .gui_light,
            crate::BlockModelGuiLight::Front
        );
        let texture = roots
            .load_resource_texture_image("example:item/widget")
            .unwrap();
        assert_eq!(texture.size, [1, 1]);
        assert_eq!(texture.rgba, [7, 11, 13, 255]);
        let sound = roots
            .load_sound_catalog()
            .unwrap()
            .event("example:widget")
            .unwrap()
            .clone();
        assert!(sound.replace);
        assert_eq!(sound.sounds.len(), 1);
        assert_eq!(sound.sounds[0].name, "example:high");
        let data = resources
            .get_data_resource(
                &crate::ResourceLocation::parse("example:test_registry/widget.json").unwrap(),
            )
            .unwrap();
        assert_eq!(data.pack_id, "startup/1");
        assert_eq!(std::fs::read(data.path).unwrap(), br#"{"source":"high"}"#);
        assert_eq!(roots.client_resource_generation().unwrap().generation(), 37);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_stack_records_identity_policy_and_compatibility() {
        let root = unique_temp_dir("startup-stack");
        let sources = root.join("sources/26.1");
        let pack = root.join("pack");
        std::fs::create_dir_all(sources.join("assets/minecraft")).unwrap();
        write_metadata(&pack, 84);
        let roots = PackRoots::from_root(&root).unwrap();

        let stack = SelectedClientPackStack::from_startup_dirs(&roots, [pack.clone()]).unwrap();

        assert_eq!(stack.packs.len(), 2);
        assert_eq!(stack.packs[0].id, "vanilla");
        assert_eq!(stack.packs[0].root, sources);
        assert_eq!(
            stack.packs[0].selection,
            ClientPackSelection {
                required: true,
                default_position: ClientPackPosition::Bottom,
                fixed_position: false,
            }
        );
        assert_eq!(stack.packs[1].id, "startup/0");
        assert_eq!(stack.packs[1].root, pack);
        assert_eq!(
            stack.packs[1].compatibility,
            ClientPackCompatibility::Compatible
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn options_repository_reconciles_missing_compatibility_legacy_ids_and_order() {
        let root = unique_temp_dir("options-repository");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let repository = root.join("resourcepacks");
        write_metadata(&repository.join("legacy"), 84);
        write_metadata(&repository.join("approved-old"), 63);
        write_metadata(&repository.join("rejected-old"), 63);
        write_metadata(&repository.join("recovered"), 84);
        write_metadata(&repository.join("current"), 84);
        write_pack_zip(&repository.join("archive.zip"));
        std::fs::create_dir_all(repository.join("malformed")).unwrap();
        std::fs::write(repository.join("malformed/pack.mcmeta"), b"not-json").unwrap();
        let roots = PackRoots::from_root(&root).unwrap();

        let reconciliation = SelectedClientPackStack::from_options_repository(
            &roots,
            &repository,
            &[
                "missing".to_owned(),
                "legacy".to_owned(),
                "file/approved-old".to_owned(),
                "file/rejected-old".to_owned(),
                "file/recovered".to_owned(),
                "file/current".to_owned(),
                "file/current".to_owned(),
                "file/archive.zip".to_owned(),
                "../outside".to_owned(),
            ],
            &[
                "file/approved-old".to_owned(),
                "file/recovered".to_owned(),
                "stale".to_owned(),
            ],
        )
        .unwrap();

        assert_eq!(
            reconciliation.resource_packs,
            vec![
                "legacy",
                "file/approved-old",
                "file/recovered",
                "file/current",
                "file/current",
                "file/archive.zip",
            ]
        );
        assert_eq!(
            reconciliation.incompatible_resource_packs,
            vec!["file/approved-old", "stale"]
        );
        assert_eq!(
            reconciliation.removed_missing,
            vec!["missing", "../outside"]
        );
        assert_eq!(
            reconciliation.removed_incompatible,
            vec!["file/rejected-old"]
        );
        assert_eq!(reconciliation.became_compatible, vec!["file/recovered"]);
        assert_eq!(
            reconciliation
                .selected_packs
                .packs()
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "vanilla",
                "file/legacy",
                "file/approved-old",
                "file/recovered",
                "file/current",
                "file/archive.zip",
            ]
        );
        assert!(matches!(
            &reconciliation.selected_packs.packs()[1].source,
            ClientPackSource::OptionsRepository { file_name } if file_name == "legacy"
        ));
        assert_eq!(
            reconciliation.selected_packs.packs()[2].compatibility,
            ClientPackCompatibility::TooOld
        );
        let archive = reconciliation.selected_packs.packs().last().unwrap();
        assert_eq!(archive.root, repository.join("archive.zip"));
        let archive = archive.archive.as_ref().unwrap();
        assert!(!archive.cache_root.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn options_zip_is_indexed_directly_and_materializes_only_consumed_entries() {
        let root = unique_temp_dir("options-zip-lazy");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let repository = root.join("resourcepacks");
        std::fs::create_dir_all(&repository).unwrap();
        let archive_path = repository.join("lazy.zip");
        let file = File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let metadata = br#"{
          "pack":{"description":"test","min_format":[84,0],"max_format":[84,0]},
          "overlays":{"entries":[{"directory":"overlay","formats":84}]}
        }"#;
        for (name, bytes) in [
            ("pack.mcmeta".to_owned(), metadata.as_slice()),
            (
                "assets/minecraft/test/value.txt".to_owned(),
                b"primary".as_slice(),
            ),
            (
                "overlay/assets/minecraft/test/value.txt".to_owned(),
                b"overlay".as_slice(),
            ),
            ("assets/minecraft/collision".to_owned(), b"file".as_slice()),
            (
                "assets/minecraft/collision/child.txt".to_owned(),
                b"child".as_slice(),
            ),
            (
                format!("assets/minecraft/{}.txt", "a".repeat(1_100)),
                b"long".as_slice(),
            ),
        ] {
            archive.start_file(name, options).unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive
            .start_file("assets/minecraft/unused.txt", options)
            .unwrap();
        archive.write_all(&vec![0_u8; 128 * 1024]).unwrap();
        archive.finish().unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let reconciliation = SelectedClientPackStack::from_options_repository(
            &roots,
            &repository,
            &["file/lazy.zip".to_owned()],
            &[],
        )
        .unwrap();
        let selected_archive = reconciliation.selected_packs.packs()[1]
            .archive
            .as_ref()
            .unwrap();
        let cache_root = selected_archive.cache_root.clone();
        assert_eq!(selected_archive.archive_path, archive_path);
        assert!(!cache_root.exists());

        let generation =
            ClientResourceGeneration::prepare(1, reconciliation.selected_packs.clone()).unwrap();
        let resources = generation.resources();
        let all_text = resources.list_resources("", ".txt").unwrap();
        assert_eq!(all_text.len(), 4);
        let listed = resources.list_resources("test", ".txt").unwrap();
        assert_eq!(listed.len(), 1);
        assert!(!cache_root.exists(), "listing must retain lazy suppliers");

        let value = resources
            .get_resource(&ResourceLocation::parse("test/value.txt").unwrap())
            .unwrap();
        assert_eq!(
            resources
                .read_text_resource(&value, "overlay value")
                .unwrap(),
            "overlay"
        );
        assert!(
            !cache_root.exists(),
            "direct ZIP reads must not materialize entries onto disk"
        );

        for (id, expected) in [
            ("collision", "file"),
            ("collision/child.txt", "child"),
            (&format!("{}.txt", "a".repeat(1_100)), "long"),
        ] {
            let resource = resources
                .get_resource(&ResourceLocation::parse(id).unwrap())
                .unwrap();
            assert_eq!(
                resources.read_text_resource(&resource, id).unwrap(),
                expected
            );
        }
        assert!(!cache_root.exists());
        let materialized = value.materialized_path().unwrap();
        assert_eq!(std::fs::read_to_string(materialized).unwrap(), "overlay");
        assert_eq!(std::fs::read_dir(&cache_root).unwrap().count(), 1);
        assert!(!cache_root.join("assets").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn supports_launcher_jar_root_startup_zip_and_builtin_optional_packs() {
        let root = unique_temp_dir("launcher-jar-and-builtins");
        let jar_path = root.join("versions/26.1/26.1.jar");
        std::fs::create_dir_all(jar_path.parent().unwrap()).unwrap();

        let builtin_zip_bytes = {
            let mut cursor = std::io::Cursor::new(Vec::new());
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("pack.mcmeta", opts).unwrap();
            zip.write_all(
                br#"{"pack":{"description":"high contrast","pack_format":29,"supported_formats":[29,64],"min_format":29,"max_format":84}}"#,
            )
            .unwrap();
            zip.start_file("assets/minecraft/lang/en_us.json", opts)
                .unwrap();
            zip.write_all(br#"{"ui.label":"contrast"}"#).unwrap();
            zip.finish().unwrap();
            cursor.into_inner()
        };

        let jar_file = File::create(&jar_path).unwrap();
        let mut jar = ZipWriter::new(jar_file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        jar.start_file("version.json", opts).unwrap();
        jar.write_all(br#"{"id":"26.1","protocol_version":775}"#)
            .unwrap();
        jar.start_file("assets/minecraft/lang/en_us.json", opts)
            .unwrap();
        jar.write_all(br#"{"ui.label":"vanilla","keep.key":"base"}"#)
            .unwrap();
        jar.start_file("assets/minecraft/resourcepacks/high_contrast.zip", opts)
            .unwrap();
        jar.write_all(&builtin_zip_bytes).unwrap();
        jar.finish().unwrap();

        let startup_zip_path = root.join("custom-startup.zip");
        let startup_file = File::create(&startup_zip_path).unwrap();
        let mut startup_zip = ZipWriter::new(startup_file);
        startup_zip.start_file("pack.mcmeta", opts).unwrap();
        startup_zip
            .write_all(
                br#"{"pack":{"description":"startup zip","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
        startup_zip
            .start_file("assets/minecraft/lang/en_us.json", opts)
            .unwrap();
        startup_zip
            .write_all(br#"{"ui.label":"startup-zip"}"#)
            .unwrap();
        startup_zip.finish().unwrap();

        assert!(PackRoots::has_usable_root(&root));
        let roots = PackRoots::from_root(&root).unwrap();

        let startup_stack =
            SelectedClientPackStack::from_startup_dirs(&roots, [startup_zip_path]).unwrap();
        let startup_roots = roots
            .clone()
            .with_selected_client_packs(startup_stack)
            .unwrap();
        let lang = startup_roots.load_language_catalog("en_us").unwrap();
        assert_eq!(lang.get("ui.label"), Some("startup-zip"));
        assert_eq!(lang.get("keep.key"), Some("base"));

        let repo = root.join("resourcepacks");
        let reconciled = SelectedClientPackStack::from_options_repository(
            &roots,
            &repo,
            &["vanilla".to_owned(), "high_contrast".to_owned()],
            &[],
        )
        .unwrap();
        assert_eq!(
            reconciled.resource_packs,
            vec!["vanilla".to_owned(), "high_contrast".to_owned()]
        );
        let builtin_roots = roots
            .with_selected_client_packs(reconciled.selected_packs)
            .unwrap();
        let builtin_lang = builtin_roots.load_language_catalog("en_us").unwrap();
        assert_eq!(builtin_lang.get("ui.label"), Some("contrast"));
        assert_eq!(builtin_lang.get("keep.key"), Some("base"));

        crate::virtual_store::remove_virtual_prefix(Path::new("/__bbb_mem__/builtin-packs"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn absent_options_repository_is_created_with_only_required_vanilla_selected() {
        let root = unique_temp_dir("options-repository-create");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let repository = root.join("resourcepacks");

        let reconciliation =
            SelectedClientPackStack::from_options_repository(&roots, &repository, &[], &[])
                .unwrap();

        assert!(repository.is_dir());
        assert_eq!(reconciliation.selected_packs.packs().len(), 1);
        assert_eq!(reconciliation.selected_packs.packs()[0].id, "vanilla");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_incompatible_startup_pack_is_retained_and_diagnosed() {
        let root = unique_temp_dir("startup-incompatible");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let pack = root.join("pack");
        write_metadata(&pack, 63);
        let roots = PackRoots::from_root(&root).unwrap();

        let stack = SelectedClientPackStack::from_startup_dirs(&roots, [pack]).unwrap();

        assert_eq!(
            stack.packs[1].compatibility,
            ClientPackCompatibility::TooOld
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_startup_metadata_is_rejected_before_selection() {
        let root = unique_temp_dir("startup-invalid");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let pack = root.join("pack");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), b"not-json").unwrap();
        let roots = PackRoots::from_root(&root).unwrap();

        let error = SelectedClientPackStack::from_startup_dirs(&roots, [pack]).unwrap_err();

        assert!(error.to_string().contains("admit --resource-pack-dir #0"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_packs_follow_vanilla_reverse_push_repository_order() {
        let root = unique_temp_dir("server-order");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let first_root = root.join("first");
        let second_root = root.join("second");
        std::fs::create_dir_all(&first_root).unwrap();
        std::fs::create_dir_all(&second_root).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let base = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let metadata = current_metadata();

        let stack = base
            .with_server_packs([
                SelectedClientPack::server("second", 9, 0, &second_root, metadata.clone()).unwrap(),
                SelectedClientPack::server("first", 4, 1, &first_root, metadata).unwrap(),
            ])
            .unwrap();

        assert_eq!(
            stack
                .packs()
                .iter()
                .map(|pack| pack.root.as_path())
                .collect::<Vec<_>>(),
            vec![
                roots.sources_dir.as_path(),
                second_root.as_path(),
                first_root.as_path()
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_pack_stack_rejects_non_monotonic_or_reused_identity() {
        let root = unique_temp_dir("server-invalid-order");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let first_root = root.join("first");
        let second_root = root.join("second");
        std::fs::create_dir_all(&first_root).unwrap();
        std::fs::create_dir_all(&second_root).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let base = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let metadata = current_metadata();

        let non_monotonic = base.with_server_packs([
            SelectedClientPack::server("first", 1, 0, &first_root, metadata.clone()).unwrap(),
            SelectedClientPack::server("second", 2, 1, &second_root, metadata.clone()).unwrap(),
        ]);
        assert!(non_monotonic
            .unwrap_err()
            .to_string()
            .contains("strict reverse-push repository order"));

        let reused = base.with_server_packs([
            SelectedClientPack::server("same", 2, 2, &second_root, metadata.clone()).unwrap(),
            SelectedClientPack::server("same", 1, 3, &first_root, metadata).unwrap(),
        ]);
        assert!(reused
            .unwrap_err()
            .to_string()
            .contains("duplicate active server client pack id"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepared_generation_reuses_compiled_filter_and_overlay_metadata() {
        let root = unique_temp_dir("prepared-metadata");
        let sources = root.join("sources/26.1");
        let pack = root.join("pack");
        write_file(
            &sources.join("assets/minecraft/test/blocked.txt"),
            b"vanilla",
        );
        write_file(
            &pack.join("overlay/assets/minecraft/test/value.txt"),
            b"overlay",
        );
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{
              "pack":{"description":"test","min_format":[84,0],"max_format":[84,0]},
              "filter":{"block":[{"namespace":"minecraft","path":"test/blocked\\.txt"}]},
              "overlays":{"entries":[{"directory":"overlay","min_format":[84,0],"max_format":[84,0]}]}
            }"#,
        )
        .unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, [pack.clone()]).unwrap();
        let roots = roots.with_client_resource_generation(12, selected).unwrap();
        std::fs::remove_file(pack.join("pack.mcmeta")).unwrap();

        let resources = roots.resource_stack();
        let blocked = crate::ResourceLocation::parse("minecraft:test/blocked.txt").unwrap();
        let value = crate::ResourceLocation::parse("minecraft:test/value.txt").unwrap();
        assert!(resources.get_resource(&blocked).is_none());
        let resolved = resources.get_resource(&value).unwrap();
        assert_eq!(std::fs::read(resolved.path).unwrap(), b"overlay");
        assert_eq!(resolved.pack_id, "startup/0");
        assert_eq!(roots.client_resource_generation().unwrap().generation(), 12);

        write_metadata(&pack, 84);
        let next_roots = PackRoots::from_root(&root).unwrap();
        let next_selected =
            SelectedClientPackStack::from_startup_dirs(&next_roots, [pack.clone()]).unwrap();
        let next_roots = next_roots
            .with_client_resource_generation(13, next_selected)
            .unwrap();
        assert!(next_roots.resource_stack().get_resource(&blocked).is_some());
        assert!(next_roots.resource_stack().get_resource(&value).is_none());
        assert!(roots.resource_stack().get_resource(&blocked).is_none());
        assert!(roots.resource_stack().get_resource(&value).is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepared_generation_rejects_invalid_filter_before_consumers_run() {
        let root = unique_temp_dir("prepared-invalid-filter");
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let pack = root.join("pack");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{
              "pack":{"description":"test","min_format":[84,0],"max_format":[84,0]},
              "filter":{"block":[{"path":"["}]}
            }"#,
        )
        .unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, [pack]).unwrap();

        let error = roots.with_selected_client_packs(selected).unwrap_err();

        assert!(format!("{error:#}").contains("compile client pack resource filter"));
        std::fs::remove_dir_all(root).unwrap();
    }

    fn write_metadata(root: &Path, pack_format: u32) {
        std::fs::create_dir_all(root).unwrap();
        let metadata = if pack_format > 64 {
            format!(
                r#"{{"pack":{{"description":"test","min_format":[{pack_format},0],"max_format":[{pack_format},0]}}}}"#
            )
        } else {
            format!(r#"{{"pack":{{"description":"test","pack_format":{pack_format}}}}}"#)
        };
        std::fs::write(root.join("pack.mcmeta"), metadata).unwrap();
    }

    fn write_pack_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(
                "pack.mcmeta",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        archive
            .write_all(
                br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
        archive.finish().unwrap();
    }

    fn current_metadata() -> ClientPackMetadata {
        ClientPackMetadata::parse(
            br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
        )
        .unwrap()
    }

    fn write_file(path: &Path, payload: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, payload).unwrap();
    }

    fn write_png(path: &Path, rgba: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, &rgba, 1, 1, image::ColorType::Rgba8).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-selected-client-packs-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
