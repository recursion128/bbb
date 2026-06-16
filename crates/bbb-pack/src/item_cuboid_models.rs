use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    block_models::{
        load_cuboid_model_resources, normalize_cuboid_model_id, resolve_cuboid_model,
        BlockFaceTextures, BlockModelDisplayTransforms, BlockModelGuiLight, BlockModelShape,
        RawBlockModel,
    },
    item_models::{ClientItemDefinition, ItemModelCatalog},
    resources::{PackResourceStack, ResourceLocation},
    PackRoots,
};

#[derive(Debug, Clone)]
pub struct ItemCuboidModelCatalog {
    models: HashMap<String, RawBlockModel>,
    item_model_count: usize,
}

impl ItemCuboidModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut models = load_cuboid_model_resources(stack, "models/block", "block model")?;
        let item_models = load_cuboid_model_resources(stack, "models/item", "item model")?;
        let item_model_count = item_models.len();
        models.extend(item_models);
        Ok(Self {
            models,
            item_model_count,
        })
    }

    pub fn model(&self, model_id: &str) -> Option<ItemCuboidModel> {
        let model_id = normalize_item_model_query_id(model_id)?;
        let raw = self.models.get(&model_id)?;
        let resolved = resolve_cuboid_model(&self.models, &model_id)?;
        Some(ItemCuboidModel {
            id: model_id,
            parent: raw.parent.as_deref().map(normalize_cuboid_model_id),
            use_ambient_occlusion: resolved.use_ambient_occlusion(),
            gui_light: resolved.gui_light(),
            display_transforms: resolved.display_transforms(),
            texture_slots: resolved
                .texture_slots()
                .into_iter()
                .map(|(slot, (id, force_translucent))| {
                    (
                        slot,
                        ItemCuboidTexture {
                            id,
                            force_translucent,
                        },
                    )
                })
                .collect(),
            face_textures: resolved.face_textures(),
            shape: resolved.shape,
        })
    }

    pub fn models_for_item(
        &self,
        item_models: &ItemModelCatalog,
        item_id: &str,
    ) -> Option<ItemCuboidModelSet> {
        item_models
            .definition(item_id)
            .map(|definition| self.models_for_definition(definition))
    }

    pub fn models_for_definition(&self, definition: &ClientItemDefinition) -> ItemCuboidModelSet {
        self.models_for_references(definition.model_references())
    }

    pub fn models_for_references(
        &self,
        model_ids: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> ItemCuboidModelSet {
        let mut models = Vec::new();
        let mut missing_model_ids = Vec::new();
        let mut seen = BTreeSet::new();
        for model_id in model_ids {
            let model_id = model_id.as_ref();
            let Some(normalized_id) = normalize_item_model_query_id(model_id) else {
                missing_model_ids.push(model_id.to_string());
                continue;
            };
            if !seen.insert(normalized_id.clone()) {
                continue;
            }
            match self.model(&normalized_id) {
                Some(model) => models.push(model),
                None => missing_model_ids.push(normalized_id),
            }
        }
        ItemCuboidModelSet {
            models,
            missing_model_ids,
        }
    }

    pub fn contains_model(&self, model_id: &str) -> bool {
        normalize_item_model_query_id(model_id)
            .is_some_and(|model_id| self.models.contains_key(&model_id))
    }

    pub fn len(&self) -> usize {
        self.item_model_count
    }

    pub fn loaded_model_count(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.item_model_count == 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemCuboidModelSet {
    pub models: Vec<ItemCuboidModel>,
    pub missing_model_ids: Vec<String>,
}

impl ItemCuboidModelSet {
    pub fn all_models_resolved(&self) -> bool {
        self.missing_model_ids.is_empty()
    }

    pub fn texture_ids(&self) -> Vec<String> {
        let mut texture_ids = BTreeSet::new();
        for model in &self.models {
            texture_ids.extend(model.texture_ids());
        }
        texture_ids.into_iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty() && self.missing_model_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemCuboidModel {
    pub id: String,
    pub parent: Option<String>,
    pub use_ambient_occlusion: bool,
    pub gui_light: BlockModelGuiLight,
    pub display_transforms: BlockModelDisplayTransforms,
    pub texture_slots: BTreeMap<String, ItemCuboidTexture>,
    pub face_textures: Option<BlockFaceTextures>,
    pub shape: BlockModelShape,
}

impl ItemCuboidModel {
    pub fn texture_ids(&self) -> Vec<String> {
        let mut texture_ids = BTreeSet::new();
        texture_ids.extend(
            self.texture_slots
                .values()
                .map(|texture| texture.id.clone()),
        );
        if let Some(face_textures) = &self.face_textures {
            texture_ids.extend(face_textures.textures.iter().cloned());
        }
        texture_ids.into_iter().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCuboidTexture {
    pub id: String,
    pub force_translucent: bool,
}

fn normalize_item_model_query_id(model_id: &str) -> Option<String> {
    if model_id.contains(':') || model_id.contains('/') {
        ResourceLocation::parse(model_id).ok().map(|id| id.id())
    } else {
        ResourceLocation::new("minecraft", format!("item/{model_id}"))
            .ok()
            .map(|id| id.id())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::block_models::{BlockModelDisplayContext, BlockModelFace};
    use crate::MC_VERSION;

    #[test]
    fn item_cuboid_catalog_resolves_item_and_block_parent_models() {
        let root = unique_temp_dir("item-cuboid-model-catalog");
        let assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_json(
            &assets.join("models").join("item").join("generated.json"),
            r#"{
                "parent": "builtin/generated",
                "gui_light": "front",
                "display": {
                    "ground": {
                        "translation": [0, 2, 0],
                        "scale": [0.5, 0.5, 0.5]
                    }
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("handheld.json"),
            r#"{
                "parent": "minecraft:item/generated",
                "display": {
                    "thirdperson_righthand": {
                        "rotation": [0, -90, 55],
                        "translation": [0, 4, 0.5],
                        "scale": [0.85, 0.85, 0.85]
                    }
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r#"{
                "parent": "minecraft:item/handheld",
                "display": {
                    "gui": {
                        "scale": [2, 2, 2]
                    }
                }
            }"#,
        );
        write_full_cube_model(
            &assets.join("models").join("block").join("small_top.json"),
            "minecraft:block/small_top",
        );
        write_json(
            &assets.join("models").join("item").join("block_item.json"),
            r#"{
                "parent": "minecraft:block/small_top"
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_cuboid_model_catalog()
            .unwrap();
        let sword = catalog.model("test_sword").unwrap();
        let block_item = catalog.model("minecraft:item/block_item").unwrap();

        assert_eq!(catalog.len(), 4);
        assert_eq!(catalog.loaded_model_count(), 5);
        assert!(catalog.contains_model("item/test_sword"));
        assert_eq!(sword.parent.as_deref(), Some("minecraft:item/handheld"));
        assert!(sword.use_ambient_occlusion);
        assert_eq!(sword.gui_light, BlockModelGuiLight::Front);
        assert_eq!(
            sword
                .display_transforms
                .get(BlockModelDisplayContext::Ground)
                .translation,
            [0.0, 0.125, 0.0]
        );
        assert_eq!(
            sword
                .display_transforms
                .get(BlockModelDisplayContext::ThirdPersonLeftHand),
            sword
                .display_transforms
                .get(BlockModelDisplayContext::ThirdPersonRightHand)
        );
        assert_eq!(
            sword
                .display_transforms
                .get(BlockModelDisplayContext::Gui)
                .scale,
            [2.0, 2.0, 2.0]
        );
        assert_eq!(
            block_item
                .face_textures
                .as_ref()
                .unwrap()
                .get(BlockModelFace::North),
            "minecraft:block/small_top"
        );
        assert_eq!(block_item.shape, BlockModelShape::Cube);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_cuboid_catalog_uses_resource_pack_model_precedence() {
        let root = unique_temp_dir("item-cuboid-resource-pack");
        let base_assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        let pack = root.join("resource_pack");
        let pack_assets = pack.join("assets").join("minecraft");

        write_json(
            &base_assets
                .join("models")
                .join("item")
                .join("test_item.json"),
            r#"{
                "gui_light": "side",
                "display": {
                    "gui": {
                        "scale": [1, 1, 1]
                    }
                }
            }"#,
        );
        write_json(
            &pack_assets
                .join("models")
                .join("item")
                .join("test_item.json"),
            r#"{
                "gui_light": "front",
                "display": {
                    "gui": {
                        "scale": [3, 3, 3]
                    }
                }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack])
            .load_item_cuboid_model_catalog()
            .unwrap();
        let model = catalog.model("test_item").unwrap();

        assert_eq!(model.gui_light, BlockModelGuiLight::Front);
        assert_eq!(
            model
                .display_transforms
                .get(BlockModelDisplayContext::Gui)
                .scale,
            [3.0, 3.0, 3.0]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_cuboid_catalog_resolves_models_for_item_definitions() {
        let root = unique_temp_dir("item-cuboid-definition-resolution");
        let assets = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
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
                            "model": "minecraft:block/test_block"
                        },
                        {
                            "type": "minecraft:model",
                            "model": "minecraft:item/missing_model"
                        },
                        {
                            "type": "minecraft:special",
                            "base": "minecraft:item/test_sword",
                            "model": {
                                "type": "minecraft:chest"
                            }
                        }
                    ]
                }
            }"#,
        );
        write_json(
            &assets.join("models").join("item").join("test_sword.json"),
            r##"{
                "gui_light": "front",
                "textures": {
                    "layer0": "minecraft:item/test_sword",
                    "layer1": "#overlay",
                    "overlay": {
                        "sprite": "custom:item/test_overlay",
                        "force_translucent": true
                    }
                },
                "display": {
                    "gui": {
                        "scale": [2, 2, 2]
                    }
                }
            }"##,
        );
        write_full_cube_model(
            &assets.join("models").join("block").join("test_block.json"),
            "minecraft:block/test_block",
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let item_models = roots.load_item_model_catalog().unwrap();
        let cuboid_models = roots.load_item_cuboid_model_catalog().unwrap();
        let resolved = cuboid_models
            .models_for_item(&item_models, "test_combo")
            .unwrap();

        assert!(!resolved.all_models_resolved());
        assert_eq!(
            resolved
                .models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["minecraft:block/test_block", "minecraft:item/test_sword"]
        );
        assert_eq!(
            resolved.missing_model_ids,
            vec!["minecraft:item/missing_model".to_string()]
        );
        assert_eq!(
            resolved.models[0]
                .face_textures
                .as_ref()
                .unwrap()
                .get(BlockModelFace::North),
            "minecraft:block/test_block"
        );
        assert_eq!(
            resolved.models[1]
                .display_transforms
                .get(BlockModelDisplayContext::Gui)
                .scale,
            [2.0, 2.0, 2.0]
        );
        assert_eq!(
            resolved.models[1].texture_slots["layer0"].id,
            "minecraft:item/test_sword"
        );
        assert_eq!(
            resolved.models[1].texture_slots["layer1"].id,
            "custom:item/test_overlay"
        );
        assert!(resolved.models[1].texture_slots["layer1"].force_translucent);
        assert_eq!(
            resolved.texture_ids(),
            vec![
                "custom:item/test_overlay",
                "minecraft:block/test_block",
                "minecraft:item/test_sword",
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore]
    fn loads_local_vanilla_item_cuboid_model_catalog() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_item_cuboid_model_catalog()
            .unwrap();

        assert_eq!(catalog.len(), 1284);
        assert!(catalog.loaded_model_count() > catalog.len());

        let generated = catalog.model("minecraft:item/generated").unwrap();
        assert_eq!(
            generated.parent.as_deref(),
            Some("minecraft:builtin/generated")
        );
        assert_eq!(generated.gui_light, BlockModelGuiLight::Front);
        assert_eq!(
            generated
                .display_transforms
                .get(BlockModelDisplayContext::Ground)
                .translation,
            [0.0, 0.125, 0.0]
        );

        let apple = catalog.model("minecraft:item/apple").unwrap();
        assert_eq!(apple.texture_slots["layer0"].id, "minecraft:item/apple");
        assert_eq!(apple.texture_ids(), vec!["minecraft:item/apple"]);

        let tipped_arrow = catalog.model("minecraft:item/tipped_arrow").unwrap();
        assert_eq!(
            tipped_arrow.texture_slots["layer0"].id,
            "minecraft:item/tipped_arrow_head"
        );
        assert_eq!(
            tipped_arrow.texture_slots["layer1"].id,
            "minecraft:item/tipped_arrow_base"
        );
        assert_eq!(
            tipped_arrow.texture_ids(),
            vec![
                "minecraft:item/tipped_arrow_base",
                "minecraft:item/tipped_arrow_head"
            ]
        );

        let spyglass = catalog.model("minecraft:item/spyglass_in_hand").unwrap();
        assert_eq!(
            spyglass.texture_slots["spyglass"].id,
            "minecraft:item/spyglass_model"
        );
        assert_eq!(
            spyglass.texture_slots["particle"].id,
            "minecraft:item/spyglass_model"
        );
        assert_eq!(
            spyglass
                .face_textures
                .as_ref()
                .unwrap()
                .get(BlockModelFace::North),
            "minecraft:item/spyglass_model"
        );

        let stone_sword = catalog.model("stone_sword").unwrap();
        assert_eq!(
            stone_sword.parent.as_deref(),
            Some("minecraft:item/handheld")
        );
        assert_eq!(stone_sword.gui_light, BlockModelGuiLight::Front);
        assert_eq!(
            stone_sword
                .display_transforms
                .get(BlockModelDisplayContext::ThirdPersonRightHand)
                .rotation,
            [0.0, -90.0, 55.0]
        );

        let small_dripleaf = catalog.model("minecraft:item/small_dripleaf").unwrap();
        assert_eq!(
            small_dripleaf.parent.as_deref(),
            Some("minecraft:block/small_dripleaf_top")
        );
        assert!(small_dripleaf.face_textures.is_some());
    }

    #[test]
    #[ignore]
    fn resolves_local_vanilla_item_definition_cuboid_models() {
        let roots = PackRoots::discover().unwrap();
        let item_models = roots.load_item_model_catalog().unwrap();
        let cuboid_models = roots.load_item_cuboid_model_catalog().unwrap();
        let mut missing_model_ids = BTreeSet::new();
        for item_id in item_models.definitions().keys() {
            let resolved = cuboid_models
                .models_for_item(&item_models, item_id)
                .unwrap();
            missing_model_ids.extend(resolved.missing_model_ids);
        }

        assert!(
            missing_model_ids.is_empty(),
            "missing item cuboid models: {missing_model_ids:?}"
        );

        let beehive = cuboid_models
            .models_for_item(&item_models, "minecraft:beehive")
            .unwrap();
        assert_eq!(
            beehive
                .models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "minecraft:block/beehive_empty",
                "minecraft:block/beehive_honey"
            ]
        );
        assert!(beehive.all_models_resolved());

        let glass = cuboid_models
            .models_for_item(&item_models, "minecraft:glass")
            .unwrap();
        assert_eq!(
            glass
                .models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["minecraft:block/glass"]
        );
        assert_eq!(
            glass.models[0].texture_slots["all"].id,
            "minecraft:block/glass"
        );
        assert!(glass.models[0].texture_slots["all"].force_translucent);
        assert_eq!(glass.texture_ids(), vec!["minecraft:block/glass"]);

        let air = cuboid_models
            .models_for_item(&item_models, "minecraft:air")
            .unwrap();
        assert_eq!(
            air.models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["minecraft:item/air"]
        );
        assert!(air.all_models_resolved());
        assert_eq!(
            air.models[0]
                .face_textures
                .as_ref()
                .unwrap()
                .get(BlockModelFace::North),
            "minecraft:missingno"
        );
        assert_eq!(air.models[0].texture_ids(), vec!["minecraft:missingno"]);
        assert_eq!(air.texture_ids(), vec!["minecraft:missingno"]);
    }

    fn write_full_cube_model(path: &Path, texture: &str) {
        write_json(
            path,
            &format!(
                r##"{{
                    "textures": {{ "all": "{texture}" }},
                    "elements": [{{
                        "from": [0, 0, 0],
                        "to": [16, 16, 16],
                        "faces": {{
                            "down": {{ "texture": "#all" }},
                            "up": {{ "texture": "#all" }},
                            "north": {{ "texture": "#all" }},
                            "south": {{ "texture": "#all" }},
                            "west": {{ "texture": "#all" }},
                            "east": {{ "texture": "#all" }}
                        }}
                    }}]
                }}"##
            ),
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{name}-{}-{nonce}", std::process::id()))
    }
}
