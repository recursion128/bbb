use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};

use crate::{
    read_client_resource_pack_metadata, ClientPackMetadata, PackResourceStack, PackRoots,
    ServerPackFormatRange,
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
                ClientPackSource::Startup { .. } | ClientPackSource::Server { .. }
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
