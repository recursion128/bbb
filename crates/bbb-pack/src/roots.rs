use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    asset_index::VanillaAssetIndex,
    atlas_sources::load_atlas_texture_entries_with_budget,
    block_destroy_profiles::BlockDestroyProfileCatalog,
    block_models::BlockModelCatalog,
    block_occlusion_profiles::BlockOcclusionProfileCatalog,
    block_sound_profiles::BlockSoundProfileCatalog,
    colors::{BiomeColorCatalog, ColorMapImage, TerrainColorMaps},
    equipment_assets::EquipmentAssetCatalog,
    image::{
        ResourceImageGenerationBudget, ResourceImageMemoryObservation,
        MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
    },
    item_cuboid_models::ItemCuboidModelCatalog,
    item_models::ItemModelCatalog,
    item_registry::ItemRegistryCatalog,
    language::{LanguageCatalog, DEFAULT_LANGUAGE_CODE},
    metadata::PackMetadataCatalog,
    particle_definitions::ParticleDefinitionCatalog,
    particle_sprites::ParticleSpriteCatalog,
    periodic_notifications::PeriodicNotificationCatalog,
    resource_budget::{
        ResourceDataGenerationBudget, ResourceDataMemoryObservation,
        MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
    },
    resources::{PackResourceStack, ResourceLocation},
    selected_packs::{ClientResourceGeneration, SelectedClientPackStack},
    sounds::SoundCatalog,
    sprites::{SpriteImage, SpriteSource},
    tags::TagCatalog,
    waypoint_styles::WaypointStyleCatalog,
};

pub const MC_VERSION: &str = "26.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRoots {
    pub vanilla_root: PathBuf,
    pub sources_dir: PathBuf,
    pub assets_dir: PathBuf,
    #[serde(skip, default)]
    indexed_vanilla_assets: Option<Arc<VanillaAssetIndex>>,
    #[serde(default)]
    resource_pack_dirs: Vec<PathBuf>,
    #[serde(skip)]
    client_resource_generation: Option<Arc<ClientResourceGeneration>>,
    #[serde(skip, default = "default_resource_image_generation_budget")]
    resource_image_budget: Arc<ResourceImageGenerationBudget>,
    #[serde(skip, default = "default_resource_data_generation_budget")]
    resource_data_budget: Arc<ResourceDataGenerationBudget>,
}

impl PackRoots {
    /// Test/maintenance adapter for the root resolved by `scripts/vanilla`.
    /// Production startup passes `--vanilla-root` to [`Self::from_root`].
    pub fn from_environment() -> Result<Self> {
        let root = std::env::var_os("BBB_VANILLA_ROOT")
            .map(PathBuf::from)
            .context("BBB_VANILLA_ROOT is required; run scripts/vanilla path 26.1 --kind root")?;
        Self::from_root(root)
    }

    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let vanilla_root = root.into();
        let sources_dir = vanilla_root.join("sources").join(MC_VERSION);
        let assets_dir = sources_dir.join("assets").join("minecraft");
        if !sources_dir.is_dir() {
            bail!("missing vanilla source directory {}", sources_dir.display());
        }
        let indexed_vanilla_assets = VanillaAssetIndex::discover(&vanilla_root)?;
        let mut roots = Self::from_paths(vanilla_root, sources_dir, assets_dir);
        roots.indexed_vanilla_assets = indexed_vanilla_assets.map(Arc::new);
        Ok(roots)
    }

    pub fn from_paths(
        vanilla_root: impl Into<PathBuf>,
        sources_dir: impl Into<PathBuf>,
        assets_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            vanilla_root: vanilla_root.into(),
            sources_dir: sources_dir.into(),
            assets_dir: assets_dir.into(),
            indexed_vanilla_assets: None,
            resource_pack_dirs: Vec::new(),
            client_resource_generation: None,
            resource_image_budget: default_resource_image_generation_budget(),
            resource_data_budget: default_resource_data_generation_budget(),
        }
    }

    pub fn with_resource_pack_dirs(
        mut self,
        dirs: impl IntoIterator<Item = impl Into<PathBuf>>,
    ) -> Self {
        self.resource_pack_dirs = dirs.into_iter().map(Into::into).collect();
        self.client_resource_generation = None;
        self.resource_image_budget = default_resource_image_generation_budget();
        self.resource_data_budget = default_resource_data_generation_budget();
        self
    }

    pub fn with_selected_client_packs(mut self, packs: SelectedClientPackStack) -> Result<Self> {
        self = self.with_client_resource_generation(0, packs)?;
        Ok(self)
    }

    pub fn with_client_resource_generation(
        mut self,
        generation: u64,
        packs: SelectedClientPackStack,
    ) -> Result<Self> {
        self = self.with_client_resource_generation_limits(
            generation,
            packs,
            MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES,
            MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
        )?;
        Ok(self)
    }

    pub fn with_client_resource_generation_image_limit(
        self,
        generation: u64,
        packs: SelectedClientPackStack,
        max_resource_image_rgba_bytes: u64,
    ) -> Result<Self> {
        self.with_client_resource_generation_limits(
            generation,
            packs,
            max_resource_image_rgba_bytes,
            MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES,
        )
    }

    pub fn with_client_resource_generation_limits(
        mut self,
        generation: u64,
        packs: SelectedClientPackStack,
        max_resource_image_rgba_bytes: u64,
        max_resource_data_logical_bytes: u64,
    ) -> Result<Self> {
        let vanilla = packs
            .packs()
            .first()
            .context("selected client pack stack is missing vanilla")?;
        if vanilla.root != self.sources_dir {
            bail!(
                "selected vanilla root {} does not match {}",
                vanilla.root.display(),
                self.sources_dir.display()
            );
        }
        self.resource_pack_dirs = packs
            .packs()
            .iter()
            .skip(1)
            .map(|pack| pack.root.clone())
            .collect();
        self.client_resource_generation =
            Some(Arc::new(ClientResourceGeneration::prepare_with_limits(
                generation,
                packs,
                max_resource_image_rgba_bytes,
                max_resource_data_logical_bytes,
            )?));
        self.resource_image_budget = self
            .client_resource_generation
            .as_ref()
            .expect("client resource generation was just installed")
            .resources()
            .image_budget();
        self.resource_data_budget = self
            .client_resource_generation
            .as_ref()
            .expect("client resource generation was just installed")
            .resources()
            .data_budget();
        Ok(self)
    }

    pub fn resource_pack_dirs(&self) -> &[PathBuf] {
        &self.resource_pack_dirs
    }

    pub fn selected_client_packs(&self) -> Option<&SelectedClientPackStack> {
        self.client_resource_generation()
            .map(ClientResourceGeneration::selected_packs)
    }

    pub fn client_resource_generation(&self) -> Option<&ClientResourceGeneration> {
        self.client_resource_generation.as_deref()
    }

    pub fn selected_client_packs_or_unmanaged(&self) -> SelectedClientPackStack {
        self.selected_client_packs()
            .cloned()
            .unwrap_or_else(|| SelectedClientPackStack::from_unmanaged_roots(self))
    }

    pub fn resource_stack(&self) -> PackResourceStack {
        if let Some(generation) = self.client_resource_generation() {
            return generation.resources().clone();
        }
        PackResourceStack::prepare_selected_client_packs_with_budgets(
            self.selected_client_packs_or_unmanaged().packs(),
            Arc::clone(&self.resource_image_budget),
            Arc::clone(&self.resource_data_budget),
        )
        .expect("lenient unmanaged resource stack preparation cannot fail")
    }

    pub(crate) fn indexed_vanilla_assets(&self) -> Option<Arc<VanillaAssetIndex>> {
        self.indexed_vanilla_assets.as_ref().map(Arc::clone)
    }

    pub fn vanilla_asset_index_observation(&self) -> Option<crate::VanillaAssetIndexObservation> {
        self.indexed_vanilla_assets
            .as_ref()
            .map(|index| index.observation())
    }

    pub fn resource_image_memory_observation(&self) -> ResourceImageMemoryObservation {
        self.resource_image_budget.observation()
    }

    pub fn resource_data_memory_observation(&self) -> ResourceDataMemoryObservation {
        self.resource_data_budget.observation()
    }

    pub(crate) fn resource_data_budget(&self) -> Arc<ResourceDataGenerationBudget> {
        Arc::clone(&self.resource_data_budget)
    }

    pub(crate) fn resource_image_budget(&self) -> Arc<ResourceImageGenerationBudget> {
        Arc::clone(&self.resource_image_budget)
    }

    pub fn ensure_resource_data_admitted(&self) -> Result<()> {
        self.resource_data_budget.ensure_admitted()
    }

    pub fn vanilla_source(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.sources_dir.join(relative)
    }

    pub fn block_textures_dir(&self) -> PathBuf {
        self.assets_dir.join("textures").join("block")
    }

    pub fn blockstates_dir(&self) -> PathBuf {
        self.assets_dir.join("blockstates")
    }

    pub fn block_models_dir(&self) -> PathBuf {
        self.assets_dir.join("models").join("block")
    }

    pub fn atlases_dir(&self) -> PathBuf {
        self.assets_dir.join("atlases")
    }

    pub fn atlas_definition(&self, name: &str) -> PathBuf {
        self.atlases_dir().join(format!("{name}.json"))
    }

    pub fn biomes_dir(&self) -> PathBuf {
        self.sources_dir
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome")
    }

    pub fn colormap_texture(&self, name: &str) -> PathBuf {
        self.assets_dir
            .join("textures")
            .join("colormap")
            .join(format!("{name}.png"))
    }

    pub fn gui_sprite_texture(&self, name: &str) -> PathBuf {
        self.assets_dir
            .join("textures")
            .join("gui")
            .join("sprites")
            .join(format!("{name}.png"))
    }

    pub fn load_gui_sprite_image(&self, name: &str) -> Result<SpriteImage> {
        self.load_atlas_texture_image("gui", &ResourceLocation::parse(name)?.id())
    }

    pub fn load_atlas_texture_image(
        &self,
        atlas_name: &str,
        sprite_id: &str,
    ) -> Result<SpriteImage> {
        let sprite_id = ResourceLocation::parse(sprite_id)?.id();
        self.load_atlas_texture_images(atlas_name)?
            .into_iter()
            .find(|image| image.id == sprite_id)
            .ok_or_else(|| anyhow::anyhow!("missing sprite {sprite_id} in atlas {atlas_name}"))
    }

    pub fn load_atlas_texture_sources(&self, atlas_name: &str) -> Result<Vec<SpriteSource>> {
        let stack = self.resource_stack();
        let mut budget = stack.atlas_png_decode_budget();
        load_atlas_texture_entries_with_budget(self, atlas_name, &mut budget)?
            .into_iter()
            .map(|entry| entry.into_source_with_budget(&mut budget, &stack))
            .collect()
    }

    pub fn load_atlas_texture_images(&self, atlas_name: &str) -> Result<Vec<SpriteImage>> {
        let stack = self.resource_stack();
        let mut budget = stack.atlas_png_decode_budget();
        load_atlas_texture_entries_with_budget(self, atlas_name, &mut budget)?
            .into_iter()
            .map(|entry| entry.into_image_with_budget(&mut budget, &stack))
            .collect()
    }

    pub fn load_block_texture_sources(&self) -> Result<Vec<SpriteSource>> {
        self.load_atlas_texture_sources("blocks")
    }

    pub fn load_block_texture_images(&self) -> Result<Vec<SpriteImage>> {
        self.load_atlas_texture_images("blocks")
    }

    pub fn load_particle_texture_sources(&self) -> Result<Vec<SpriteSource>> {
        self.load_atlas_texture_sources("particles")
    }

    pub fn load_particle_texture_images(&self) -> Result<Vec<SpriteImage>> {
        self.load_atlas_texture_images("particles")
    }

    pub fn load_block_model_catalog(&self) -> Result<BlockModelCatalog> {
        BlockModelCatalog::load(self)
    }

    pub fn load_item_model_catalog(&self) -> Result<ItemModelCatalog> {
        ItemModelCatalog::load(self)
    }

    pub fn load_item_cuboid_model_catalog(&self) -> Result<ItemCuboidModelCatalog> {
        ItemCuboidModelCatalog::load(self)
    }

    pub fn load_item_registry_catalog(&self) -> Result<ItemRegistryCatalog> {
        ItemRegistryCatalog::load(self)
    }

    pub fn load_block_destroy_profile_catalog(&self) -> Result<BlockDestroyProfileCatalog> {
        BlockDestroyProfileCatalog::load(self)
    }

    pub fn load_block_occlusion_profile_catalog(&self) -> Result<BlockOcclusionProfileCatalog> {
        BlockOcclusionProfileCatalog::load(self)
    }

    pub fn load_block_sound_profile_catalog(&self) -> Result<BlockSoundProfileCatalog> {
        BlockSoundProfileCatalog::load(self)
    }

    pub fn load_equipment_asset_catalog(&self) -> Result<EquipmentAssetCatalog> {
        EquipmentAssetCatalog::load(self)
    }

    pub fn load_waypoint_style_catalog(&self) -> Result<WaypointStyleCatalog> {
        WaypointStyleCatalog::load(self)
    }

    pub fn load_particle_definition_catalog(&self) -> Result<ParticleDefinitionCatalog> {
        ParticleDefinitionCatalog::load(self)
    }

    pub fn load_particle_sprite_catalog(&self) -> Result<ParticleSpriteCatalog> {
        ParticleSpriteCatalog::load(self)
    }

    pub fn load_periodic_notification_catalog(&self) -> Result<PeriodicNotificationCatalog> {
        PeriodicNotificationCatalog::load(self)
    }

    pub fn load_terrain_colormaps(&self) -> Result<TerrainColorMaps> {
        let stack = self.resource_stack();
        Ok(TerrainColorMaps {
            grass: load_terrain_colormap(&stack, "grass")?,
            foliage: load_terrain_colormap(&stack, "foliage")?,
            dry_foliage: Some(load_terrain_colormap(&stack, "dry_foliage")?),
        })
    }

    pub fn load_biome_color_catalog(&self) -> Result<BiomeColorCatalog> {
        BiomeColorCatalog::load_vanilla_26_1(self)
    }

    pub fn load_sound_catalog(&self) -> Result<SoundCatalog> {
        let mut catalog = SoundCatalog::load_resource_stack(&self.resource_stack())?;
        catalog.set_resource_generation(
            self.client_resource_generation()
                .map_or(0, ClientResourceGeneration::generation),
        );
        Ok(catalog)
    }

    pub fn load_required_sound_catalog(&self) -> Result<SoundCatalog> {
        let catalog = self
            .load_sound_catalog()
            .context("load required sound catalog minecraft:sounds.json")?;
        if catalog.is_empty() {
            bail!(
                "load required sound catalog: required sound catalog is empty; \
                 minecraft:sounds.json resolved no events"
            );
        }
        Ok(catalog)
    }

    pub fn load_language_catalog(&self, language_code: &str) -> Result<LanguageCatalog> {
        if language_code == DEFAULT_LANGUAGE_CODE {
            self.load_language_stack([DEFAULT_LANGUAGE_CODE])
        } else {
            self.load_language_stack([DEFAULT_LANGUAGE_CODE, language_code])
        }
    }

    pub fn load_client_language_catalog(
        &self,
        selected_language_code: &str,
    ) -> Result<LanguageCatalog> {
        let metadata = self.load_pack_metadata_catalog();
        let mut catalog =
            self.load_language_stack(metadata.language_stack(selected_language_code))?;
        catalog.set_bidirectional(metadata.selected_bidirectional(selected_language_code));
        Ok(catalog)
    }

    pub fn load_pack_metadata_catalog(&self) -> PackMetadataCatalog {
        PackMetadataCatalog::load_resource_stack(&self.resource_stack())
    }

    fn load_language_stack(
        &self,
        language_codes: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<LanguageCatalog> {
        let language_codes = language_codes
            .into_iter()
            .map(|code| code.as_ref().to_string())
            .collect::<Vec<_>>();
        let stack = self.resource_stack();
        let mut catalog = LanguageCatalog::load_resource_stack(&stack, &language_codes)?;
        let deprecated_path = self.assets_dir.join("lang").join("deprecated.json");
        if deprecated_path.is_file() {
            // Vanilla loads deprecated translations from the default resource, not overlays.
            if let Ok(info) = stack.read_json_file(&deprecated_path, "deprecated translations") {
                let _ = catalog.apply_deprecated_info(info);
            }
        }
        Ok(catalog)
    }

    pub fn load_tag_catalog(&self, registry_path: &str) -> Result<TagCatalog> {
        TagCatalog::load_resource_stack(&self.resource_stack(), registry_path)
    }
}

fn default_resource_image_generation_budget() -> Arc<ResourceImageGenerationBudget> {
    ResourceImageGenerationBudget::new(MAX_RESOURCE_IMAGE_GENERATION_RGBA_BYTES)
}

fn default_resource_data_generation_budget() -> Arc<ResourceDataGenerationBudget> {
    ResourceDataGenerationBudget::new(MAX_RESOURCE_DATA_GENERATION_LOGICAL_BYTES)
}

fn load_terrain_colormap(stack: &PackResourceStack, name: &str) -> Result<ColorMapImage> {
    let location = ResourceLocation::parse(&format!("textures/colormap/{name}.png"))?;
    let resource = stack
        .get_resource(&location)
        .ok_or_else(|| anyhow::anyhow!("missing terrain colormap {}", location.id()))?;
    ColorMapImage::from_png_file_with_resource_stack(resource.path, stack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::{Digest, Sha1};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{
        colors::VANILLA_BIOME_ORDER, AtlasPacker, GrassColorModifier, SpriteGuiScaling,
        SpriteMipmapStrategy, SpriteNineSliceBorder, SpriteSource,
    };

    #[test]
    fn indexed_vanilla_provider_replaces_source_stub_but_selected_pack_wins() {
        let root = unique_temp_dir("indexed-vanilla-precedence");
        let source = root.join("sources/26.1/assets/minecraft/font/test.zip");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"source stub").unwrap();
        std::fs::write(source.with_extension("zip.mcmeta"), b"source metadata").unwrap();

        let indexed = b"indexed complete archive";
        let indexed_metadata = br#"{"texture":{"blur":true}}"#;
        write_asset_index_fixture(
            &root,
            &[
                ("minecraft/font/test.zip", indexed, true),
                ("minecraft/font/test.zip.mcmeta", indexed_metadata, true),
                ("minecraft/sounds/unrelated-missing.ogg", b"missing", false),
                ("realms/lang/en_us.json", br#"{"realm":"Realm"}"#, true),
                ("icons/icon_16x16.png", b"icon", true),
                ("pack.mcmeta", br#"{"language":{}}"#, true),
            ],
        );

        let location = ResourceLocation::parse("minecraft:font/test.zip").unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let stack = roots.resource_stack();
        assert_eq!(stack.namespaces().unwrap(), ["minecraft", "realms"]);
        let resource = stack.get_resource(&location).unwrap();
        assert_eq!(resource.pack_id, "vanilla");
        assert_eq!(
            stack
                .read_text_resource(&resource, "indexed test")
                .unwrap()
                .as_bytes(),
            indexed
        );
        assert_eq!(
            std::fs::read(resource.metadata_path.as_ref().unwrap()).unwrap(),
            indexed_metadata
        );
        stack
            .validate_resource_metadata(&resource, "indexed test metadata")
            .unwrap();
        let listed = stack.list_resources("font", ".zip").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].location, location);
        let root_resources = stack.root_resource_stack("pack.mcmeta").unwrap();
        assert_eq!(root_resources.len(), 1);
        let root_metadata: serde_json::Value = stack
            .read_json_root_resource(&root_resources[0], "Vanilla root metadata")
            .unwrap();
        assert_eq!(root_metadata, serde_json::json!({"language": {}}));
        let observation = stack.vanilla_asset_index_observation().unwrap();
        assert_eq!(observation.index_id, "30");
        assert_eq!(observation.object_count, 6);
        assert_eq!(observation.exposed_resource_count, 4);
        assert_eq!(observation.root_resource_count, 2);
        assert_eq!(roots.vanilla_asset_index_observation(), Some(observation));
        let realms = stack
            .get_resource(&ResourceLocation::parse("realms:lang/en_us.json").unwrap())
            .unwrap();
        assert_eq!(realms.pack_id, "vanilla");
        let charged_before = roots
            .resource_data_memory_observation()
            .derived_logical_bytes;
        let _second_stack = roots.resource_stack();
        assert_eq!(
            roots
                .resource_data_memory_observation()
                .derived_logical_bytes,
            charged_before,
            "the shared generation must retain-charge the index only once"
        );

        let high = root.join("packs/high");
        let high_resource = high.join("assets/minecraft/font/test.zip");
        std::fs::create_dir_all(high_resource.parent().unwrap()).unwrap();
        std::fs::write(&high_resource, b"selected override").unwrap();
        let roots = roots.with_resource_pack_dirs([high]);
        let resource = roots.resource_stack().get_resource(&location).unwrap();
        assert_eq!(std::fs::read(resource.path).unwrap(), b"selected override");
        let stack = roots.resource_stack().get_resource_stack(&location);
        assert_eq!(
            stack
                .iter()
                .map(|resource| resource.pack_id.as_str())
                .collect::<Vec<_>>(),
            ["vanilla", "unmanaged/0"]
        );

        // Pack identity, rather than filesystem ancestry, decides precedence.
        // A selected pack is allowed to live below the source checkout.
        let nested_high = root.join("sources/selected-high");
        let nested_resource = nested_high.join("assets/minecraft/font/test.zip");
        std::fs::create_dir_all(nested_resource.parent().unwrap()).unwrap();
        std::fs::write(&nested_resource, b"nested selected override").unwrap();
        let roots = roots.with_resource_pack_dirs([nested_high]);
        let resource = roots.resource_stack().get_resource(&location).unwrap();
        assert_eq!(
            std::fs::read(resource.path).unwrap(),
            b"nested selected override"
        );

        let filter = root.join("packs/filter");
        write_json(
            &filter.join("pack.mcmeta"),
            r#"{
              "filter": {
                "block": [
                  {"namespace":"minecraft","path":"font/test\\.zip"}
                ]
              }
            }"#,
        );
        let filtered = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([filter]);
        assert!(filtered.resource_stack().get_resource(&location).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    fn write_asset_index_fixture(root: &Path, objects: &[(&str, &[u8], bool)]) {
        let mut document_objects = serde_json::Map::new();
        for (name, bytes, materialize) in objects {
            let hash = format!("{:x}", Sha1::digest(bytes));
            document_objects.insert(
                (*name).to_string(),
                serde_json::json!({"hash": hash, "size": bytes.len()}),
            );
            if *materialize {
                let object_path = root
                    .join("downloads/26.1/assets/objects")
                    .join(&hash[..2])
                    .join(&hash);
                std::fs::create_dir_all(object_path.parent().unwrap()).unwrap();
                std::fs::write(object_path, bytes).unwrap();
            }
        }
        let index =
            serde_json::to_string(&serde_json::json!({"objects": document_objects})).unwrap();
        let index_hash = format!("{:x}", Sha1::digest(index.as_bytes()));
        write_json(
            &root.join("downloads/26.1/26.1.json"),
            &format!(
                r#"{{"assetIndex":{{"id":"30","sha1":"{index_hash}","size":{}}}}}"#,
                index.len()
            ),
        );
        write_json(&root.join("downloads/26.1/assets/indexes/30.json"), &index);
    }

    #[test]
    fn pack_roots_loads_block_texture_sources_from_vanilla_atlas() {
        let root = unique_temp_dir("pack-roots");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let block_dir = assets_dir.join("textures").join("block");
        write_test_png(&block_dir.join("z_stone.png"), 16, 16);
        write_test_png(&block_dir.join("a_hd_overlay.png"), 64, 32);
        write_test_png(&block_dir.join("sub").join("deepslate.png"), 8, 8);
        std::fs::write(block_dir.join("a_hd_overlay.png.mcmeta"), "{}").unwrap();
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("conduit")
                .join("base.png"),
            32,
            16,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("bell")
                .join("bell_body.png"),
            32,
            32,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("entity")
                .join("enchantment")
                .join("enchanting_table_book.png"),
            48,
            16,
        );
        write_json(
            &assets_dir.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "block/",
                  "source": "block"
                },
                {
                  "type": "minecraft:directory",
                  "prefix": "entity/conduit/",
                  "source": "entity/conduit"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:entity/bell/bell_body"
                },
                {
                  "type": "minecraft:single",
                  "resource": "minecraft:entity/enchantment/enchanting_table_book",
                  "sprite": "minecraft:custom/book"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("minecraft:block/a_hd_overlay", 64, 32),
                SpriteSource::new("minecraft:block/sub/deepslate", 8, 8),
                SpriteSource::new("minecraft:block/z_stone", 16, 16),
                SpriteSource::new("minecraft:entity/conduit/base", 32, 16),
                SpriteSource::new("minecraft:entity/bell/bell_body", 32, 32),
                SpriteSource::new("minecraft:custom/book", 48, 16),
            ]
        );
        let images = roots.load_block_texture_images().unwrap();
        assert_eq!(images.len(), sources.len());
        assert_eq!(images.last().unwrap().id, "minecraft:custom/book");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_directory_atlas_sources_scan_all_namespaces() {
        let root = unique_temp_dir("pack-roots-directory-namespaces");
        let sources_dir = root.join("sources").join(MC_VERSION);
        let assets_root = sources_dir.join("assets");
        let minecraft_assets = assets_root.join("minecraft");
        let example_assets = assets_root.join("example");
        write_test_png(
            &example_assets
                .join("textures")
                .join("block")
                .join("gem.png"),
            16,
            16,
        );
        write_test_png(
            &example_assets
                .join("textures")
                .join("block")
                .join("sub")
                .join("ore.png"),
            8,
            8,
        );
        write_test_png(
            &minecraft_assets
                .join("textures")
                .join("block")
                .join("stone.png"),
            16,
            16,
        );
        write_json(
            &minecraft_assets.join("atlases").join("blocks.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "block/",
                  "source": "block"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let custom_pack = root.join("packs/custom");
        std::fs::create_dir_all(custom_pack.join("assets")).unwrap();
        std::fs::rename(&example_assets, custom_pack.join("assets/example")).unwrap();
        let sources = roots
            .with_resource_pack_dirs([custom_pack])
            .load_block_texture_sources()
            .unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("example:block/gem", 16, 16),
                SpriteSource::new("example:block/sub/ore", 8, 8),
                SpriteSource::new("minecraft:block/stone", 16, 16),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn separate_atlases_share_one_generation_image_budget() {
        let root = unique_temp_dir("generation-atlas-budget");
        let sources_dir = root.join("sources").join(MC_VERSION);
        let assets_dir = sources_dir.join("assets/minecraft");
        for name in ["first", "second"] {
            write_test_png(&assets_dir.join(format!("textures/test/{name}.png")), 1, 1);
            write_json(
                &assets_dir.join(format!("atlases/{name}.json")),
                &format!(
                    r#"{{"sources":[{{"type":"minecraft:single","resource":"minecraft:test/{name}"}}]}}"#
                ),
            );
        }
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots
            .with_client_resource_generation_image_limit(3, selected, 4)
            .unwrap();

        assert_eq!(roots.load_atlas_texture_sources("first").unwrap().len(), 2);
        let err = roots
            .load_atlas_texture_sources("second")
            .unwrap_err()
            .to_string();
        assert!(err.contains("currently charged 4, requested 4"), "{err}");
        assert_eq!(
            roots.resource_image_memory_observation().charged_rgba_bytes,
            4
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_gui_sprite_images_from_vanilla_atlas() {
        let root = unique_temp_dir("gui-atlas");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_test_png(
            &assets_dir
                .join("textures")
                .join("gui")
                .join("sprites")
                .join("hud")
                .join("crosshair.png"),
            15,
            15,
        );
        write_test_png(
            &assets_dir
                .join("textures")
                .join("mob_effect")
                .join("speed.png"),
            18,
            18,
        );
        write_json(
            &assets_dir.join("atlases").join("gui.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "gui/sprites"
                },
                {
                  "type": "minecraft:directory",
                  "prefix": "mob_effect/",
                  "source": "mob_effect"
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let images = roots.load_atlas_texture_images("gui").unwrap();
        let ids = images
            .iter()
            .map(|image| image.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "minecraft:missingno",
                "minecraft:hud/crosshair",
                "minecraft:mob_effect/speed"
            ]
        );
        let crosshair = roots.load_gui_sprite_image("hud/crosshair").unwrap();
        assert_eq!(crosshair.id, "minecraft:hud/crosshair");
        assert_eq!((crosshair.width, crosshair.height), (15, 15));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_uses_indexed_sound_assets_when_source_sounds_are_absent() {
        let root = unique_temp_dir("indexed-sounds");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION)).unwrap();
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.click": {
                "sounds": ["random/click"]
              }
            }"#,
                    true,
                ),
                ("minecraft/sounds/random/click.ogg", b"indexed ogg", true),
            ],
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_required_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();

        assert_eq!(event.sounds.len(), 1);
        assert_eq!(
            std::fs::read(event.sounds[0].ogg_path.as_ref().unwrap()).unwrap(),
            b"indexed ogg"
        );
        assert!(event.sounds[0].ogg_integrity.is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_sound_definition_replaces_the_source_stub() {
        let root = unique_temp_dir("indexed-sounds-before-source");
        let source_assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let source_ogg = source_assets
            .join("sounds")
            .join("random")
            .join("source.ogg");
        write_file(&source_ogg);
        write_json(
            &source_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": ["random/source"]
              }
            }"#,
        );
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.generated": {
                "sounds": ["random/generated"]
              }
            }"#,
                    true,
                ),
                (
                    "minecraft/sounds/random/generated.ogg",
                    b"indexed generated",
                    true,
                ),
            ],
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_required_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.generated").unwrap();

        assert_eq!(catalog.len(), 1);
        assert!(catalog.event("minecraft:ui.button.click").is_none());
        assert_ne!(event.sounds[0].ogg_path, Some(source_ogg));
        assert!(event.sounds[0].ogg_integrity.is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_sound_assets_keep_resource_pack_overlays() {
        let root = unique_temp_dir("indexed-sounds-pack-overlays");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION)).unwrap();
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.click": {
                "sounds": ["random/base"]
              }
            }"#,
                    true,
                ),
                ("minecraft/sounds/random/base.ogg", b"indexed base", true),
            ],
        );

        let overlay = root.join("overlay");
        let overlay_assets = overlay.join("assets").join("minecraft");
        let overlay_ogg = overlay_assets
            .join("sounds")
            .join("random")
            .join("overlay.ogg");
        write_file(&overlay_ogg);
        write_json(
            &overlay_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "replace": true,
                "sounds": ["random/overlay"]
              },
              "ui.button.vanilla_reference": {
                "sounds": ["random/base"]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_required_sound_catalog().unwrap();
        let click = catalog.event("minecraft:ui.button.click").unwrap();
        let vanilla_reference = catalog
            .event("minecraft:ui.button.vanilla_reference")
            .unwrap();

        assert!(click.replace);
        assert_eq!(click.sounds.len(), 1);
        assert_eq!(click.sounds[0].name, "minecraft:random/overlay");
        assert_eq!(click.sounds[0].ogg_path, Some(overlay_ogg));
        assert_eq!(vanilla_reference.sounds.len(), 1);
        assert_eq!(vanilla_reference.sounds[0].name, "minecraft:random/base");
        assert_eq!(
            std::fs::read(vanilla_reference.sounds[0].ogg_path.as_ref().unwrap()).unwrap(),
            b"indexed base"
        );
        assert!(vanilla_reference.sounds[0].ogg_integrity.is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_sound_assets_append_resource_pack_sounds_without_replace() {
        let root = unique_temp_dir("indexed-sounds-pack-append");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION)).unwrap();
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.click": {
                "subtitle": "subtitles.ui.button.base",
                "sounds": ["random/base"]
              }
            }"#,
                    true,
                ),
                ("minecraft/sounds/random/base.ogg", b"indexed base", true),
            ],
        );

        let overlay = root.join("overlay");
        let overlay_assets = overlay.join("assets").join("minecraft");
        let overlay_ogg = overlay_assets
            .join("sounds")
            .join("random")
            .join("overlay.ogg");
        write_file(&overlay_ogg);
        write_json(
            &overlay_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "subtitle": "subtitles.ui.button.overlay",
                "sounds": ["random/overlay"]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_required_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();

        assert!(!event.replace);
        assert_eq!(event.subtitle.as_deref(), Some("subtitles.ui.button.base"));
        assert_eq!(event.sounds.len(), 2);
        assert_eq!(
            std::fs::read(event.sounds[0].ogg_path.as_ref().unwrap()).unwrap(),
            b"indexed base"
        );
        assert!(event.sounds[0].ogg_integrity.is_some());
        assert_eq!(event.sounds[1].ogg_path, Some(overlay_ogg));
        assert!(event.sounds[1].ogg_integrity.is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_sound_assets_honor_resource_pack_filter_for_sounds_json() {
        let root = unique_temp_dir("indexed-sounds-filter-json");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION)).unwrap();
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.click": {
                "sounds": ["random/base"]
              }
            }"#,
                    true,
                ),
                ("minecraft/sounds/random/base.ogg", b"indexed base", true),
            ],
        );
        let filter_pack = root.join("filter-pack");
        write_json(
            &filter_pack.join("pack.mcmeta"),
            r#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "sounds\\.json"
                  }
                ]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([filter_pack]);
        let err = roots.load_required_sound_catalog().unwrap_err();
        let message = format!("{err:#}");

        assert!(message.contains("required sound catalog is empty"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_sound_assets_honor_resource_pack_filter_for_ogg_files() {
        let root = unique_temp_dir("indexed-sounds-filter-ogg");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION)).unwrap();
        write_asset_index_fixture(
            &root,
            &[
                (
                    "minecraft/sounds.json",
                    br#"{
              "ui.button.click": {
                "sounds": ["random/base"]
              }
            }"#,
                    true,
                ),
                ("minecraft/sounds/random/base.ogg", b"indexed base", true),
            ],
        );
        let filter_pack = root.join("filter-pack");
        write_json(
            &filter_pack.join("pack.mcmeta"),
            r#"{
              "filter": {
                "block": [
                  {
                    "namespace": "minecraft",
                    "path": "sounds/random/base\\.ogg"
                  }
                ]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([filter_pack]);
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();

        assert!(event.sounds.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_paletted_permutation_atlas_images() {
        let root = unique_temp_dir("paletted-permutations");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_test_rgba_png(
            &assets_dir.join("textures").join("palette").join("key.png"),
            2,
            1,
            &[10, 10, 10, 255, 20, 20, 20, 0],
        );
        write_test_rgba_png(
            &assets_dir.join("textures").join("palette").join("ruby.png"),
            2,
            1,
            &[100, 10, 20, 128, 200, 200, 200, 255],
        );
        write_test_rgba_png(
            &assets_dir.join("textures").join("pattern").join("base.png"),
            3,
            1,
            &[10, 10, 10, 200, 20, 20, 20, 77, 10, 10, 10, 0],
        );
        write_json(
            &assets_dir.join("atlases").join("items.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:paletted_permutations",
                  "palette_key": "minecraft:palette/key",
                  "permutations": {
                    "ruby": "minecraft:palette/ruby"
                  },
                  "textures": [
                    "minecraft:pattern/base"
                  ]
                }
              ]
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let sources = roots.load_atlas_texture_sources("items").unwrap();
        assert_eq!(
            sources,
            vec![
                SpriteSource::new("minecraft:missingno", 16, 16),
                SpriteSource::new("minecraft:pattern/base_ruby", 3, 1)
            ]
        );
        let images = roots.load_atlas_texture_images("items").unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].id, "minecraft:missingno");
        assert_eq!(images[1].id, "minecraft:pattern/base_ruby");
        assert_eq!(
            images[1].rgba,
            vec![100, 10, 20, 100, 20, 20, 20, 77, 10, 10, 10, 0]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_terrain_colormaps() {
        let root = unique_temp_dir("terrain-colormaps");
        let colormap_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        write_test_png(&colormap_dir.join("grass.png"), 4, 4);
        write_test_png(&colormap_dir.join("foliage.png"), 4, 4);
        write_test_png(&colormap_dir.join("dry_foliage.png"), 4, 4);

        let roots = PackRoots::from_root(&root).unwrap();
        let colormaps = roots.load_terrain_colormaps().unwrap();
        assert_eq!((colormaps.grass.width, colormaps.grass.height), (4, 4));
        assert_eq!((colormaps.foliage.width, colormaps.foliage.height), (4, 4));
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| (colormap.width, colormap.height)),
            Some((4, 4))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_terrain_colormaps_use_resource_pack_precedence() {
        let root = unique_temp_dir("terrain-colormap-pack-precedence");
        let base_colormap_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        let pack = root.join("pack");
        let pack_colormap_dir = pack
            .join("assets")
            .join("minecraft")
            .join("textures")
            .join("colormap");
        write_test_rgba_png(&base_colormap_dir.join("grass.png"), 1, 1, &[1, 2, 3, 255]);
        write_test_rgba_png(
            &base_colormap_dir.join("foliage.png"),
            1,
            1,
            &[40, 50, 60, 255],
        );
        write_test_rgba_png(
            &base_colormap_dir.join("dry_foliage.png"),
            1,
            1,
            &[70, 80, 90, 255],
        );
        write_test_rgba_png(
            &pack_colormap_dir.join("grass.png"),
            1,
            1,
            &[10, 20, 30, 255],
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack]);
        let colormaps = roots.load_terrain_colormaps().unwrap();

        assert_eq!(colormaps.grass.rgb_at(0, 0), [10, 20, 30]);
        assert_eq!(colormaps.foliage.rgb_at(0, 0), [40, 50, 60]);
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| colormap.rgb_at(0, 0)),
            Some([70, 80, 90])
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_loads_biome_color_catalog_by_vanilla_id() {
        let root = unique_temp_dir("biome-color-catalog");
        let biome_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        write_json(
            &biome_dir.join("plains.json"),
            r##"{
              "attributes": {
                "minecraft:visual/fog_color": "#c0d8ff",
                "minecraft:visual/sky_color": "#78a7ff",
                "minecraft:visual/water_fog_color": "#050533"
              },
              "temperature": 0.8,
              "downfall": 0.4,
              "has_precipitation": true,
              "effects": {
                "water_color": "#123456"
              }
            }"##,
        );
        write_json(
            &biome_dir.join("swamp.json"),
            r##"{
              "attributes": {
                "minecraft:visual/water_fog_end_distance": {
                  "argument": 0.85,
                  "modifier": "multiply"
                }
              },
              "temperature": 0.8,
              "temperature_modifier": "frozen",
              "downfall": 0.9,
              "has_precipitation": true,
              "effects": {
                "dry_foliage_color": "#7b5334",
                "foliage_color": "#6a7039",
                "grass_color_modifier": "swamp",
                "water_color": "#617b64"
              }
            }"##,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_biome_color_catalog().unwrap();
        let plains = catalog.profile(1).unwrap();
        assert_eq!(plains.name, "minecraft:plains");
        assert_eq!(plains.temperature, 0.8);
        assert_eq!(plains.downfall, 0.4);
        assert!(plains.has_precipitation);
        assert_eq!(plains.water_color, Some([0x12, 0x34, 0x56]));

        let swamp = catalog.profile(6).unwrap();
        assert_eq!(swamp.name, "minecraft:swamp");
        assert_eq!(
            swamp.temperature_modifier,
            crate::colors::BiomeTemperatureModifier::Frozen
        );
        assert_eq!(swamp.foliage_color, Some([0x6a, 0x70, 0x39]));
        assert_eq!(swamp.dry_foliage_color, Some([0x7b, 0x53, 0x34]));
        assert_eq!(swamp.water_color, Some([0x61, 0x7b, 0x64]));
        assert_eq!(swamp.grass_color_modifier, GrassColorModifier::Swamp);
        assert!(catalog.profile(0).is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pack_roots_biome_color_catalog_uses_data_pack_precedence() {
        let root = unique_temp_dir("biome-color-pack-precedence");
        let base_biome_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        let pack = root.join("pack");
        let pack_biome_dir = pack
            .join("data")
            .join("minecraft")
            .join("worldgen")
            .join("biome");
        write_json(
            &base_biome_dir.join("plains.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.4,
              "has_precipitation": true,
              "effects": {
                "water_color": "#123456"
              }
            }"##,
        );
        write_json(
            &base_biome_dir.join("swamp.json"),
            r##"{
              "temperature": 0.8,
              "downfall": 0.9,
              "has_precipitation": true,
              "effects": {
                "water_color": "#617b64"
              }
            }"##,
        );
        write_json(
            &pack_biome_dir.join("plains.json"),
            r##"{
              "temperature": 0.7,
              "downfall": 0.3,
              "has_precipitation": false,
              "effects": {
                "water_color": "#abcdef",
                "grass_color": "#010203"
              }
            }"##,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack]);
        let catalog = roots.load_biome_color_catalog().unwrap();
        let plains = catalog.profile(1).unwrap();
        let swamp = catalog.profile(6).unwrap();

        assert_eq!(plains.temperature, 0.7);
        assert_eq!(plains.downfall, 0.3);
        assert!(!plains.has_precipitation);
        assert_eq!(plains.water_color, Some([0xab, 0xcd, 0xef]));
        assert_eq!(plains.grass_color, Some([1, 2, 3]));
        assert_eq!(swamp.water_color, Some([0x61, 0x7b, 0x64]));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_resource_data_within_generation_budget() {
        let roots = PackRoots::from_environment().unwrap();
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots.with_client_resource_generation(1, selected).unwrap();

        let _ = crate::KnownPacksManager::load(&roots).unwrap();
        let _ = roots.load_block_model_catalog().unwrap();
        let _ = roots.load_item_model_catalog().unwrap();
        let _ = roots.load_item_cuboid_model_catalog().unwrap();
        let _ = roots.load_item_registry_catalog().unwrap();
        let _ = roots.load_equipment_asset_catalog().unwrap();
        let _ = roots.load_waypoint_style_catalog().unwrap();
        let _ = roots.load_particle_definition_catalog().unwrap();
        let _ = roots.load_biome_color_catalog().unwrap();
        let _ = roots.load_sound_catalog().unwrap();
        let _ = roots
            .load_client_language_catalog(DEFAULT_LANGUAGE_CODE)
            .unwrap();
        let _ = roots.load_pack_metadata_catalog();
        let _ = roots.load_tag_catalog("block").unwrap();
        let _ = roots.load_tag_catalog("item").unwrap();
        roots.ensure_resource_data_admitted().unwrap();

        let observation = roots.resource_data_memory_observation();
        eprintln!("vanilla resource data logical charge: {observation:?}");
        assert!(observation.charged_logical_bytes > 0);
        assert!(observation.charged_logical_bytes < observation.max_logical_bytes);
        assert_eq!(observation.rejections, 0);
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_all_local_vanilla_atlases() {
        let roots = PackRoots::from_environment().unwrap();
        let expected = [
            ("armor_trims", 577),
            ("banner_patterns", 45),
            ("beds", 17),
            ("blocks", 1_122),
            ("celestials", 11),
            ("chests", 23),
            ("decorated_pot", 26),
            ("gui", 487),
            ("items", 857),
            ("map_decorations", 36),
            ("paintings", 53),
            ("particles", 255),
            ("shield_patterns", 46),
            ("shulker_boxes", 19),
            ("signs", 25),
        ];

        for (atlas_name, expected_count) in expected {
            let sources = roots.load_atlas_texture_sources(atlas_name).unwrap();
            assert_eq!(
                sources.len(),
                expected_count,
                "unexpected sprite count for {atlas_name} atlas"
            );
            assert!(
                sources
                    .iter()
                    .all(|source| source.width > 0 && source.height > 0),
                "atlas {atlas_name} contains an empty sprite"
            );
        }
        let observation = roots.resource_image_memory_observation();
        eprintln!("vanilla atlas logical RGBA charge: {observation:?}");
        assert!(observation.charged_rgba_bytes > 0);
        assert!(observation.charged_rgba_bytes < observation.max_rgba_bytes);
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_block_texture_dimensions() {
        let roots = PackRoots::from_environment().unwrap();
        let sources = roots.load_block_texture_sources().unwrap();
        assert_eq!(sources.len(), 1_122);
        let biome_colors = roots.load_biome_color_catalog().unwrap();
        assert_eq!(biome_colors.len(), VANILLA_BIOME_ORDER.len());
        let plains = biome_colors.profile(1).unwrap();
        assert_eq!(plains.name, "minecraft:plains");
        assert_eq!(plains.water_color, Some([0x3f, 0x76, 0xe4]));
        let swamp = biome_colors.profile(6).unwrap();
        assert_eq!(swamp.name, "minecraft:swamp");
        assert_eq!(swamp.grass_color_modifier, GrassColorModifier::Swamp);
        assert_eq!(swamp.foliage_color, Some([0x6a, 0x70, 0x39]));
        let colormaps = roots.load_terrain_colormaps().unwrap();
        assert_eq!((colormaps.grass.width, colormaps.grass.height), (256, 256));
        assert_eq!(
            (colormaps.foliage.width, colormaps.foliage.height),
            (256, 256)
        );
        assert_eq!(
            colormaps
                .dry_foliage
                .as_ref()
                .map(|colormap| (colormap.width, colormap.height)),
            Some((256, 256))
        );

        let stone = sources
            .iter()
            .find(|source| source.id == "minecraft:block/stone")
            .unwrap();
        assert_eq!((stone.width, stone.height), (16, 16));
        assert_eq!(
            stone.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Auto
        );

        let torchflower = sources
            .iter()
            .find(|source| source.id == "minecraft:block/torchflower")
            .unwrap();
        assert_eq!(
            torchflower.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::StrictCutout
        );
        let glass = sources
            .iter()
            .find(|source| source.id == "minecraft:block/glass")
            .unwrap();
        assert_eq!(
            glass.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::Mean
        );
        let oak_leaves = sources
            .iter()
            .find(|source| source.id == "minecraft:block/oak_leaves")
            .unwrap();
        assert_eq!(
            oak_leaves.texture_metadata.mipmap_strategy,
            SpriteMipmapStrategy::DarkCutout
        );

        let water = sources
            .iter()
            .find(|source| source.id == "minecraft:block/water_still")
            .unwrap();
        assert_eq!((water.width, water.height), (16, 16));
        let water_animation = water.animation.as_ref().unwrap();
        assert_eq!(water_animation.frame_count, 32);
        assert_eq!(water_animation.default_frame_time, 2);
        assert_eq!(water_animation.frames.len(), 32);
        let water_image = roots
            .load_atlas_texture_image("blocks", "minecraft:block/water_still")
            .unwrap();
        assert_eq!(water_image.animation_frames_rgba.len(), 32);
        assert_eq!(water_image.frame_rgba(31).unwrap().len(), 16 * 16 * 4);
        let water_flow = sources
            .iter()
            .find(|source| source.id == "minecraft:block/water_flow")
            .unwrap();
        assert_eq!((water_flow.width, water_flow.height), (32, 32));
        let lava = sources
            .iter()
            .find(|source| source.id == "minecraft:block/lava_still")
            .unwrap();
        assert_eq!((lava.width, lava.height), (16, 16));
        let lava_animation = lava.animation.as_ref().unwrap();
        assert_eq!(lava_animation.frame_count, 20);
        assert_eq!(lava_animation.default_frame_time, 2);
        assert_eq!(lava_animation.frames.len(), 38);
        assert_eq!(lava_animation.frames[20].index, 18);
        let fire = sources
            .iter()
            .find(|source| source.id == "minecraft:block/fire_0")
            .unwrap();
        assert_eq!((fire.width, fire.height), (16, 16));
        let conduit = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/conduit/base")
            .unwrap();
        assert_eq!((conduit.width, conduit.height), (32, 16));
        let conduit_wind = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/conduit/wind")
            .unwrap();
        assert_eq!((conduit_wind.width, conduit_wind.height), (64, 32));
        let bell = sources
            .iter()
            .find(|source| source.id == "minecraft:entity/bell/bell_body")
            .unwrap();
        assert_eq!((bell.width, bell.height), (32, 32));

        let layout = AtlasPacker::new(4096, 1)
            .unwrap()
            .pack(&sources[..64])
            .unwrap();
        assert!(layout.width <= 4096);
        assert_eq!(layout.sprites.len(), 64);
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn stitches_local_vanilla_block_texture_mip_atlas() {
        let roots = PackRoots::from_environment().unwrap();
        let images = roots.load_block_texture_images().unwrap();
        assert_eq!(images.len(), 1_122);

        // Mirrors the official 26.1 `blocks.png-atlas` creation log. The
        // dimensions are shader-observable through terrain RGSS LOD selection.
        let packer = AtlasPacker::new(2048, 1).unwrap();
        let atlas = packer.stitch_mips_with_max_level(&images, 4).unwrap();
        assert_eq!((atlas.layout.width, atlas.layout.height), (2048, 2048));
        assert_eq!(atlas.levels.len(), atlas.mip_level() as usize + 1);
        assert!(atlas.mip_level() > 0);
        assert!(atlas.mip_level() <= 4);
        for (level, mip) in atlas.levels.iter().enumerate() {
            assert_eq!(mip.level, level as u32);
            assert_eq!(mip.width, atlas.layout.width >> level);
            assert_eq!(mip.height, atlas.layout.height >> level);
            assert_eq!(mip.rgba.len(), (mip.width * mip.height * 4) as usize);
        }

        let animation_atlas = packer
            .stitch_animation_frame_mips_with_max_level(&images, 2, 4)
            .unwrap();
        assert_eq!(animation_atlas.mip_level(), atlas.mip_level());
        assert_eq!(animation_atlas.levels.len(), atlas.levels.len());
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_and_armor_trim_atlases() {
        let roots = PackRoots::from_environment().unwrap();
        let item_sources = roots.load_atlas_texture_sources("items").unwrap();
        assert_eq!(item_sources.len(), 857);
        let helmet_trim = item_sources
            .iter()
            .find(|source| source.id == "minecraft:trims/items/helmet_trim_diamond")
            .unwrap();
        assert_eq!((helmet_trim.width, helmet_trim.height), (16, 16));
        let apple = item_sources
            .iter()
            .find(|source| source.id == "minecraft:item/apple")
            .unwrap();
        assert_eq!((apple.width, apple.height), (16, 16));

        let armor_sources = roots.load_atlas_texture_sources("armor_trims").unwrap();
        assert_eq!(armor_sources.len(), 577);
        let sentry = armor_sources
            .iter()
            .find(|source| source.id == "minecraft:trims/entity/humanoid/sentry_diamond")
            .unwrap();
        assert_eq!((sentry.width, sentry.height), (64, 32));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_gui_atlas() {
        let roots = PackRoots::from_environment().unwrap();
        let gui_sources = roots.load_atlas_texture_sources("gui").unwrap();
        assert_eq!(gui_sources.len(), 487);
        let crosshair = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/crosshair")
            .unwrap();
        assert_eq!((crosshair.width, crosshair.height), (15, 15));
        let hotbar = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/hotbar")
            .unwrap();
        assert_eq!((hotbar.width, hotbar.height), (182, 22));
        let speed = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:mob_effect/speed")
            .unwrap();
        assert_eq!((speed.width, speed.height), (18, 18));
        let locator_arrow = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/locator_bar_arrow_down")
            .unwrap();
        assert_eq!((locator_arrow.width, locator_arrow.height), (7, 5));
        let locator_animation = locator_arrow.animation.as_ref().unwrap();
        assert_eq!(locator_animation.frame_count, 2);
        assert_eq!(locator_animation.frames[0].time, 10);
        assert_eq!(locator_animation.frames[1].time, 4);

        let crosshair_image = roots.load_gui_sprite_image("hud/crosshair").unwrap();
        assert_eq!(crosshair_image.id, "minecraft:hud/crosshair");
        assert_eq!((crosshair_image.width, crosshair_image.height), (15, 15));

        let button = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:widget/button")
            .unwrap();
        assert_eq!(
            button.gui_metadata.scaling,
            SpriteGuiScaling::NineSlice {
                width: 200,
                height: 20,
                border: SpriteNineSliceBorder::uniform(3),
                stretch_inner: false,
            }
        );
        let locator_background = gui_sources
            .iter()
            .find(|source| source.id == "minecraft:hud/locator_bar_background")
            .unwrap();
        assert_eq!(
            locator_background.gui_metadata.scaling,
            SpriteGuiScaling::NineSlice {
                width: 12,
                height: 5,
                border: SpriteNineSliceBorder {
                    left: 5,
                    right: 5,
                    top: 1,
                    bottom: 1,
                },
                stretch_inner: false,
            }
        );
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        write_test_rgba_png(
            path,
            width,
            height,
            &[1, 2, 3, 255].repeat((width * height) as usize),
        );
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec()).unwrap();
        image.save(path).unwrap();
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, []).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{name}-{}-{nonce}", std::process::id()))
    }
}
