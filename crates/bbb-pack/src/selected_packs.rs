use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use sha1::{Digest, Sha1};

use crate::{
    materialize_server_pack_archive, read_client_resource_pack_metadata, ClientPackMetadata,
    PackResourceStack, PackRoots, ServerPackArchiveLimits, ServerPackFormatRange,
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
}

impl SelectedClientPack {
    fn vanilla(roots: &PackRoots) -> Self {
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
            metadata: None,
            indexed_vanilla_assets: roots.indexed_vanilla_assets(),
        }
    }

    pub fn startup(argument_index: usize, root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
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
        })
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
        })
    }

    pub fn server(
        pack_id: impl Into<String>,
        push_sequence: u64,
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
            id: format!("server/{push_sequence:020}/{pack_id}"),
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

#[derive(Debug, Clone)]
pub struct ClientResourceGeneration {
    generation: u64,
    selected_packs: Arc<SelectedClientPackStack>,
    resources: PackResourceStack,
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
        })
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
}

impl SelectedClientPackStack {
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
    /// `Pack.readMetaAndCreate` returning null. Selected ZIPs are materialized
    /// through the same bounded, atomic path used by downloaded server packs;
    /// their repository id remains `file/<archive-name>`.
    pub fn from_options_repository(
        roots: &PackRoots,
        repository: &Path,
        resource_packs: &[String],
        incompatible_resource_packs: &[String],
    ) -> Result<ClientPackOptionsReconciliation> {
        fs::create_dir_all(repository)
            .with_context(|| format!("create resource pack repository {}", repository.display()))?;
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
        let mut available = BTreeMap::new();
        for entry in fs::read_dir(repository)
            .with_context(|| format!("list resource pack repository {}", repository.display()))?
        {
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
                materialize_options_repository_archive(&path, &archive_cache).and_then(
                    |(root, metadata)| {
                        SelectedClientPack::options_repository_with_metadata(
                            file_name, root, metadata,
                        )
                    },
                )
            } else {
                continue;
            };
            let Ok(pack) = pack else {
                continue;
            };
            available.insert(pack.id.clone(), pack);
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
        server_packs_in_push_order: impl IntoIterator<Item = SelectedClientPack>,
    ) -> Result<Self> {
        let mut packs = self
            .packs
            .iter()
            .filter(|pack| !matches!(pack.source, ClientPackSource::Server { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let mut previous_push_sequence = None;
        let mut server_ids = std::collections::BTreeSet::new();
        for pack in server_packs_in_push_order {
            let ClientPackSource::Server {
                ref pack_id,
                push_sequence,
            } = pack.source
            else {
                bail!("non-server pack {} supplied as a server pack", pack.id);
            };
            if previous_push_sequence.is_some_and(|previous| previous >= push_sequence) {
                bail!("server client packs are not in strict push order");
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
                ClientPackSource::Startup { .. }
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

fn materialize_options_repository_archive(
    archive: &Path,
    cache_root: &Path,
) -> Result<(PathBuf, ClientPackMetadata)> {
    let limits = ServerPackArchiveLimits::default();
    let archive_bytes = fs::metadata(archive)
        .with_context(|| format!("read options resource pack archive {}", archive.display()))?
        .len();
    if archive_bytes > limits.max_archive_bytes {
        bail!(
            "options resource pack archive is {archive_bytes} bytes, exceeding the {} byte limit",
            limits.max_archive_bytes
        );
    }
    let mut file = fs::File::open(archive)
        .with_context(|| format!("open options resource pack archive {}", archive.display()))?;
    let mut sha1 = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("hash options resource pack archive {}", archive.display()))?;
        if read == 0 {
            break;
        }
        sha1.update(&buffer[..read]);
    }
    let output = cache_root.join(format!("{:x}", sha1.finalize()));
    if output.is_dir() {
        let metadata = read_client_resource_pack_metadata(&output)?;
        return Ok((output, metadata));
    }
    fs::create_dir_all(cache_root).with_context(|| {
        format!(
            "create options resource pack cache {}",
            cache_root.display()
        )
    })?;
    match materialize_server_pack_archive(archive, &output, limits) {
        Ok(materialized) => Ok((materialized.root, materialized.metadata)),
        Err(_error) if output.is_dir() => {
            let metadata = read_client_resource_pack_metadata(&output)?;
            Ok((output, metadata))
        }
        Err(error) => Err(error),
    }
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
            br#"{"gui_light":"front"}"#,
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
        assert_ne!(archive.root, repository.join("archive.zip"));
        assert!(archive.root.join("pack.mcmeta").is_file());
        std::fs::remove_dir_all(root).unwrap();
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
    fn server_packs_follow_startup_packs_in_push_order() {
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
                SelectedClientPack::server("first", 4, &first_root, metadata.clone()).unwrap(),
                SelectedClientPack::server("second", 9, &second_root, metadata).unwrap(),
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
                first_root.as_path(),
                second_root.as_path()
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
            SelectedClientPack::server("first", 2, &first_root, metadata.clone()).unwrap(),
            SelectedClientPack::server("second", 1, &second_root, metadata.clone()).unwrap(),
        ]);
        assert!(non_monotonic
            .unwrap_err()
            .to_string()
            .contains("strict push order"));

        let reused = base.with_server_packs([
            SelectedClientPack::server("same", 1, &first_root, metadata.clone()).unwrap(),
            SelectedClientPack::server("same", 2, &second_root, metadata).unwrap(),
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
