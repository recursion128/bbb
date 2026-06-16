use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasImage, AtlasLayout, AtlasPacker, AtlasSprite, ItemCuboidModel, ItemCuboidModelCatalog,
    ItemCuboidModelSet, ItemCuboidTextureImageCatalog, ItemModelCatalog, ItemRegistryCatalog,
    PackRoots, SpriteImage,
};

const ITEM_ATLAS_MAX_WIDTH: u32 = 4096;
const MISSING_TEXTURE_ID: &str = "minecraft:missingno";

#[derive(Debug, Clone)]
pub(crate) struct NativeItemRuntime {
    item_definition_count: usize,
    item_registry_count: usize,
    resolved_model_count: usize,
    missing_model_ids: BTreeSet<String>,
    missing_texture_ids: BTreeSet<String>,
    item_texture_indices: HashMap<String, u32>,
    registry: Option<ItemRegistryCatalog>,
    textures: ItemTextureState,
}

impl NativeItemRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        let item_models = roots
            .load_item_model_catalog()
            .context("load item model catalog")?;
        let cuboid_models = roots
            .load_item_cuboid_model_catalog()
            .context("load item cuboid model catalog")?;
        let texture_images = ItemCuboidTextureImageCatalog::load(roots)
            .context("load item cuboid texture images")?;
        let registry = roots
            .load_item_registry_catalog()
            .context("load item registry catalog")
            .map_err(|err| {
                tracing::warn!(?err, "continuing without native item registry catalog");
                err
            })
            .ok();
        Self::from_loaded(item_models, cuboid_models, texture_images, registry)
    }

    fn from_loaded(
        item_models: ItemModelCatalog,
        cuboid_models: ItemCuboidModelCatalog,
        texture_images: ItemCuboidTextureImageCatalog,
        registry: Option<ItemRegistryCatalog>,
    ) -> Result<Self> {
        let mut texture_ids = BTreeSet::new();
        let mut item_texture_ids = HashMap::new();
        let mut missing_model_ids = BTreeSet::new();
        let mut missing_texture_ids = BTreeSet::new();
        let mut resolved_model_count = 0usize;

        for item_id in item_models.definitions().keys() {
            let Some(models) = cuboid_models.models_for_item(&item_models, item_id) else {
                continue;
            };
            resolved_model_count += models.models.len();
            texture_ids.extend(models.texture_ids());
            if let Some(texture_id) = item_icon_texture_id(&models) {
                item_texture_ids.insert(item_id.clone(), texture_id);
            }
            missing_model_ids.extend(models.missing_model_ids);
        }

        let mut images = Vec::new();
        if let Some(image) = texture_images.image(MISSING_TEXTURE_ID) {
            images.push(image.clone());
        } else {
            missing_texture_ids.insert(MISSING_TEXTURE_ID.to_string());
        }
        for texture_id in texture_ids {
            if texture_id == MISSING_TEXTURE_ID {
                continue;
            }
            match texture_images.image(&texture_id) {
                Some(image) => images.push(image.clone()),
                None => {
                    missing_texture_ids.insert(texture_id);
                }
            }
        }

        let textures = ItemTextureState::from_images(images)?;
        let item_texture_indices = item_texture_ids
            .into_iter()
            .map(|(item_id, texture_id)| (item_id, textures.texture_index(&texture_id)))
            .collect();

        Ok(Self {
            item_definition_count: item_models.len(),
            item_registry_count: registry.as_ref().map_or(0, ItemRegistryCatalog::len),
            resolved_model_count,
            missing_model_ids,
            missing_texture_ids,
            item_texture_indices,
            registry,
            textures,
        })
    }

    pub(crate) fn item_definition_count(&self) -> usize {
        self.item_definition_count
    }

    pub(crate) fn item_registry_count(&self) -> usize {
        self.item_registry_count
    }

    pub(crate) fn resolved_model_count(&self) -> usize {
        self.resolved_model_count
    }

    pub(crate) fn missing_model_count(&self) -> usize {
        self.missing_model_ids.len()
    }

    pub(crate) fn missing_texture_count(&self) -> usize {
        self.missing_texture_ids.len()
    }

    pub(crate) fn texture_count(&self) -> usize {
        self.textures.texture_count()
    }

    pub(crate) fn icon_texture_count(&self) -> usize {
        self.item_texture_indices.len()
    }

    pub(crate) fn atlas_size(&self) -> (u32, u32) {
        self.textures.atlas_size()
    }

    pub(crate) fn atlas_rgba(&self) -> &[u8] {
        self.textures.atlas_rgba()
    }

    pub(crate) fn texture_index(&self, texture_id: &str) -> u32 {
        self.textures.texture_index(texture_id)
    }

    pub(crate) fn icon_texture_index_for_protocol_id(&self, protocol_id: i32) -> Option<u32> {
        let item_id = self.registry.as_ref()?.resource_id(protocol_id)?;
        Some(
            self.item_texture_indices
                .get(item_id)
                .copied()
                .unwrap_or(self.textures.fallback_index()),
        )
    }

    pub(crate) fn icon_uv_for_protocol_id(&self, protocol_id: i32) -> Option<ItemAtlasUvRect> {
        self.textures
            .texture_uv_rect(self.icon_texture_index_for_protocol_id(protocol_id)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemAtlasUvRect {
    pub(crate) min: [f32; 2],
    pub(crate) max: [f32; 2],
}

#[derive(Debug, Clone)]
struct ItemTextureState {
    atlas: AtlasImage,
    texture_indices: HashMap<String, u32>,
    fallback_index: u32,
}

impl ItemTextureState {
    fn from_images(images: Vec<SpriteImage>) -> Result<Self> {
        let packer = AtlasPacker::new(ITEM_ATLAS_MAX_WIDTH, 1)?;
        let atlas = packer.stitch(&images)?;
        let mut texture_indices = HashMap::new();
        for (index, sprite) in atlas.layout.sprites.iter().enumerate() {
            texture_indices.insert(sprite.id.clone(), index as u32);
        }
        let fallback_index = texture_indices
            .get(MISSING_TEXTURE_ID)
            .copied()
            .unwrap_or(0);
        Ok(Self {
            atlas,
            texture_indices,
            fallback_index,
        })
    }

    fn texture_count(&self) -> usize {
        self.atlas.layout.sprites.len()
    }

    fn atlas_size(&self) -> (u32, u32) {
        (self.atlas.layout.width, self.atlas.layout.height)
    }

    fn atlas_rgba(&self) -> &[u8] {
        &self.atlas.rgba
    }

    fn fallback_index(&self) -> u32 {
        self.fallback_index
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.texture_indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    fn texture_uv_rect(&self, texture_index: u32) -> Option<ItemAtlasUvRect> {
        let sprite = self.atlas.layout.sprites.get(texture_index as usize)?;
        Some(item_uv_rect(&self.atlas.layout, sprite))
    }
}

fn item_icon_texture_id(models: &ItemCuboidModelSet) -> Option<String> {
    // First pass: pick a stable flat base texture. Layered, tinted, and special GUI item
    // rendering will need a richer icon representation.
    models
        .models
        .iter()
        .find_map(layer0_texture_id)
        .or_else(|| models.models.iter().find_map(first_texture_id))
}

fn layer0_texture_id(model: &ItemCuboidModel) -> Option<String> {
    model
        .texture_slots
        .get("layer0")
        .map(|texture| texture.id.clone())
}

fn first_texture_id(model: &ItemCuboidModel) -> Option<String> {
    model
        .texture_slots
        .values()
        .next()
        .map(|texture| texture.id.clone())
        .or_else(|| {
            model
                .face_textures
                .as_ref()
                .map(|textures| textures.textures[0].clone())
        })
}

fn item_uv_rect(layout: &AtlasLayout, sprite: &AtlasSprite) -> ItemAtlasUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    ItemAtlasUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn item_texture_state_indexes_textures_and_uses_missing_fallback() {
        let missing = SpriteImage::new("minecraft:missingno", 1, 1, vec![0, 0, 0, 255]).unwrap();
        let apple = SpriteImage::new("minecraft:item/apple", 1, 1, vec![255, 0, 0, 255]).unwrap();

        let state = ItemTextureState::from_images(vec![missing, apple]).unwrap();

        assert_eq!(state.texture_count(), 2);
        assert_ne!(
            state.texture_index("minecraft:item/apple"),
            state.texture_index("minecraft:missingno")
        );
        assert_eq!(
            state.texture_index("custom:item/missing"),
            state.texture_index(MISSING_TEXTURE_ID)
        );
    }

    #[test]
    fn native_item_runtime_loads_fixture_and_keeps_missingno_fallback() {
        let root = unique_temp_dir("item-runtime");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_item_registry_sources(&root);
        write_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:composite",
                    "models": [
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/test_sword"
                        },
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/missing_model"
                        }
                    ]
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r##"{
                "textures": {
                    "layer0": "minecraft:item/test_sword",
                    "layer1": {
                        "sprite": "custom:item/missing_overlay",
                        "force_translucent": true
                    }
                }
            }"##,
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("test_sword.png"),
            1,
            1,
            &[255, 0, 0, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1);
        assert_eq!(runtime.item_registry_count(), 1);
        assert_eq!(runtime.resolved_model_count(), 1);
        assert_eq!(runtime.missing_model_count(), 1);
        assert_eq!(runtime.missing_texture_count(), 1);
        assert_eq!(runtime.texture_count(), 2);
        assert_eq!(runtime.icon_texture_count(), 1);
        assert_ne!(
            runtime.texture_index("minecraft:item/test_sword"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.texture_index("custom:item/missing_overlay"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.texture_index("unknown:item/texture"),
            runtime.texture_index(MISSING_TEXTURE_ID)
        );
        assert_eq!(
            runtime.icon_texture_index_for_protocol_id(0),
            Some(runtime.texture_index("minecraft:item/test_sword"))
        );
        assert_eq!(runtime.icon_texture_index_for_protocol_id(1), None);
        assert!(runtime.icon_uv_for_protocol_id(0).is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_item_runtime_loads_assets_when_registry_source_is_missing() {
        let root = unique_temp_dir("item-runtime-no-registry");
        let assets = assets_dir(&root);
        write_item_atlases(&assets);
        write_json(
            &assets.join("items").join("test_combo.json"),
            r#"{
                "model": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/test_sword"
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r#"{
                "textures": {
                    "layer0": "minecraft:item/test_sword"
                }
            }"#,
        );
        write_test_rgba_png(
            &assets.join("textures").join("item").join("test_sword.png"),
            1,
            1,
            &[255, 0, 0, 255],
        );

        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1);
        assert_eq!(runtime.item_registry_count(), 0);
        assert_eq!(runtime.texture_count(), 2);
        assert_eq!(runtime.icon_texture_count(), 1);
        assert!(!runtime.atlas_rgba().is_empty());
        assert_eq!(runtime.icon_texture_index_for_protocol_id(0), None);
        assert_eq!(runtime.icon_uv_for_protocol_id(0), None);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_runtime_assets() {
        let runtime = NativeItemRuntime::load(&PackRoots::discover().unwrap()).unwrap();

        assert_eq!(runtime.item_definition_count(), 1506);
        assert_eq!(runtime.item_registry_count(), 1506);
        assert_eq!(runtime.texture_count(), 1576);
        assert_eq!(runtime.icon_texture_count(), 1506);
        assert_eq!(runtime.missing_model_count(), 0);
        assert_eq!(runtime.missing_texture_count(), 0);
        assert!(runtime.icon_uv_for_protocol_id(1).is_some());
        assert!(runtime.resolved_model_count() > runtime.item_definition_count());
        assert!(runtime.atlas_size().0 > 0);
        assert!(runtime.atlas_size().1 > 0);
    }

    fn assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn write_item_atlases(assets: &Path) {
        write_json(
            &assets.join("atlases").join("items.json"),
            r#"{
                "sources": [
                    {
                        "type": "minecraft:directory",
                        "prefix": "item/",
                        "source": "item"
                    }
                ]
            }"#,
        );
        write_json(
            &assets.join("atlases").join("blocks.json"),
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
    }

    fn write_item_registry_sources(root: &Path) {
        write_json(
            &root
                .join("sources")
                .join(bbb_pack::MC_VERSION)
                .join("net")
                .join("minecraft")
                .join("world")
                .join("item")
                .join("Items.java"),
            r#"public class Items {
                public static final Item TEST_COMBO = registerItem("test_combo");
            }"#,
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}-{id}"))
    }
}
