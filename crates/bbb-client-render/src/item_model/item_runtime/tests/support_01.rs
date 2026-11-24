use super::*;

pub(super) struct ComponentConditionHarness {
    root: PathBuf,
    pub(super) runtime: NativeItemRuntime,
    pub(super) defaults: bbb_render_types::ItemComponentDefaults,
    pub(super) trim_keys: [String; 2],
    pub(super) trim_pattern_keys: [String; 2],
    pub(super) enchantment_keys: [String; 2],
    pub(super) attribute_keys: Vec<String>,
}

impl ComponentConditionHarness {
    pub(super) fn new() -> Self {
        let root = unique_temp_dir("item-runtime-component-condition");
        write_component_condition_fixture(&root);
        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let defaults = stack_component_defaults(&runtime);
        let attribute_keys = bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES
            .iter()
            .map(|attribute| attribute.resource_id.to_string())
            .collect();
        Self {
            root,
            runtime,
            defaults,
            trim_keys: [
                "minecraft:quartz".to_string(),
                "minecraft:diamond".to_string(),
            ],
            trim_pattern_keys: ["minecraft:sentry".to_string(), "minecraft:dune".to_string()],
            enchantment_keys: [
                "minecraft:sharpness".to_string(),
                "minecraft:mending".to_string(),
            ],
            attribute_keys,
        }
    }

    pub(super) fn base_context(&self) -> ItemModelContext<'_> {
        ItemModelContext {
            item_profiles: Some(self.defaults.as_view()),
            ..ItemModelContext::new(BlockModelDisplayContext::Gui)
        }
    }

    pub(super) fn uv(&self, model_id: &str) -> ItemAtlasUvRect {
        self.runtime
            .textures
            .texture_uv_rect(
                self.runtime
                    .texture_index(&format!("minecraft:item/{model_id}")),
            )
            .unwrap()
    }

    pub(super) fn resolve_with_context(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
        context: &ItemModelContext<'_>,
    ) -> ItemAtlasUvRect {
        self.runtime
            .resolve_item_model(
                &ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: component_patch.into(),
                },
                context,
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
            .layers[0]
            .uv
    }

    pub(super) fn selected(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        self.resolve_with_context(item_id, component_patch, &self.base_context())
    }

    pub(super) fn selected_with_registry_trim_keys(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
        material_keys: &[String],
        pattern_keys: &[String],
    ) -> ItemAtlasUvRect {
        let context = ItemModelContext {
            registry_keys: ItemModelRegistryKeys {
                trim_materials: Some(material_keys),
                trim_patterns: Some(pattern_keys),
                ..Default::default()
            },
            ..self.base_context()
        };
        self.resolve_with_context(item_id, component_patch, &context)
    }

    pub(super) fn selected_with_trim_keys(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        self.selected_with_registry_trim_keys(
            item_id,
            component_patch,
            &self.trim_keys,
            &self.trim_pattern_keys,
        )
    }

    pub(super) fn selected_with_enchantment_keys(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        let context = ItemModelContext {
            registry_keys: ItemModelRegistryKeys {
                enchantments: Some(&self.enchantment_keys),
                ..Default::default()
            },
            ..self.base_context()
        };
        self.resolve_with_context(item_id, component_patch, &context)
    }

    pub(super) fn selected_with_attribute_keys(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        let context = ItemModelContext {
            registry_keys: ItemModelRegistryKeys {
                attributes: Some(&self.attribute_keys),
                ..Default::default()
            },
            ..self.base_context()
        };
        self.resolve_with_context(item_id, component_patch, &context)
    }

    pub(super) fn assert_cases(
        &self,
        item_id: i32,
        model_prefix: &str,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, bool)>,
    ) {
        let present = self.uv(&format!("{model_prefix}_present"));
        let absent = self.uv(&format!("{model_prefix}_absent"));
        for (idx, (patch, is_present)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.selected(item_id, patch),
                if is_present { present } else { absent },
                "{model_prefix} case #{idx}"
            );
        }
    }

    pub(super) fn assert_trim_cases(
        &self,
        item_id: i32,
        model_prefix: &str,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, bool)>,
    ) {
        let present = self.uv(&format!("{model_prefix}_present"));
        let absent = self.uv(&format!("{model_prefix}_absent"));
        for (idx, (patch, is_present)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.selected_with_trim_keys(item_id, patch),
                if is_present { present } else { absent },
                "{model_prefix} trim case #{idx}"
            );
        }
    }

    pub(super) fn assert_enchant_cases(
        &self,
        item_id: i32,
        model_prefix: &str,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, bool)>,
    ) {
        let present = self.uv(&format!("{model_prefix}_present"));
        let absent = self.uv(&format!("{model_prefix}_absent"));
        for (idx, (patch, is_present)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.selected_with_enchantment_keys(item_id, patch),
                if is_present { present } else { absent },
                "{model_prefix} enchant case #{idx}"
            );
        }
    }

    pub(super) fn assert_attr_cases(
        &self,
        item_id: i32,
        model_prefix: &str,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, bool)>,
    ) {
        let present = self.uv(&format!("{model_prefix}_present"));
        let absent = self.uv(&format!("{model_prefix}_absent"));
        for (idx, (patch, is_present)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.selected_with_attribute_keys(item_id, patch),
                if is_present { present } else { absent },
                "{model_prefix} attr case #{idx}"
            );
        }
    }
}

impl Drop for ComponentConditionHarness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub(super) struct FixtureRuntimeHarness {
    pub(super) root: PathBuf,
    pub(super) runtime: NativeItemRuntime,
    pub(super) defaults: bbb_render_types::ItemComponentDefaults,
}

impl FixtureRuntimeHarness {
    pub(super) fn load(label: &str, write_fixture: impl FnOnce(&Path)) -> Self {
        let root = unique_temp_dir(label);
        write_fixture(&root);
        let runtime = NativeItemRuntime::load(&PackRoots::from_root(&root).unwrap()).unwrap();
        let defaults = stack_component_defaults(&runtime);
        Self {
            root,
            runtime,
            defaults,
        }
    }

    pub(super) fn base_context(&self) -> ItemModelContext<'_> {
        ItemModelContext {
            item_profiles: Some(self.defaults.as_view()),
            ..ItemModelContext::new(BlockModelDisplayContext::Gui)
        }
    }

    pub(super) fn uv(&self, model_id: &str) -> ItemAtlasUvRect {
        self.runtime
            .textures
            .texture_uv_rect(
                self.runtime
                    .texture_index(&format!("minecraft:item/{model_id}")),
            )
            .unwrap()
    }

    pub(super) fn flat_icon_with_context(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
        context: &ItemModelContext<'_>,
    ) -> ItemAtlasIcon {
        self.runtime
            .resolve_item_model(
                &ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: component_patch.into(),
                },
                context,
            )
            .and_then(|model| model.flat_icon())
            .unwrap()
    }

    pub(super) fn profiled_selected(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        self.flat_icon_with_context(item_id, component_patch, &self.base_context())
            .layers[0]
            .uv
    }

    pub(super) fn gui_selected(
        &self,
        item_id: i32,
        component_patch: impl Into<bbb_protocol::packets::SparseDataComponentPatchSummary>,
    ) -> ItemAtlasUvRect {
        self.flat_icon_with_context(
            item_id,
            component_patch,
            &ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .layers[0]
            .uv
    }

    pub(super) fn assert_profiled_models<'a>(
        &self,
        item_id: i32,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, &'a str)>,
    ) {
        for (idx, (patch, expected_model)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.profiled_selected(item_id, patch),
                self.uv(expected_model),
                "item {item_id} profiled model case #{idx} -> {expected_model}"
            );
        }
    }

    pub(super) fn assert_gui_models<'a>(
        &self,
        item_id: i32,
        cases: impl IntoIterator<Item = (DataComponentPatchSummary, &'a str)>,
    ) {
        for (idx, (patch, expected_model)) in cases.into_iter().enumerate() {
            assert_eq!(
                self.gui_selected(item_id, patch),
                self.uv(expected_model),
                "item {item_id} gui model case #{idx} -> {expected_model}"
            );
        }
    }
}

impl Drop for FixtureRuntimeHarness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub(super) fn patch_with(
    added_type_ids: &[i32],
    build: impl FnOnce(&mut DataComponentPatchSummary),
) -> DataComponentPatchSummary {
    let mut patch = DataComponentPatchSummary {
        added_type_ids: added_type_ids.to_vec(),
        ..DataComponentPatchSummary::default()
    };
    build(&mut patch);
    patch
}

pub(super) fn patch_removed(removed_type_ids: &[i32]) -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        removed_type_ids: removed_type_ids.to_vec(),
        ..DataComponentPatchSummary::default()
    }
}

pub(super) fn template_stack(
    item_id: i32,
    count: i32,
    patch: DataComponentPatchSummary,
) -> ItemStackTemplateSummary {
    ItemStackTemplateSummary {
        item_id,
        count,
        component_patch: patch.into(),
    }
}

pub(super) fn plain_template_stack(item_id: i32, count: i32) -> ItemStackTemplateSummary {
    template_stack(item_id, count, DataComponentPatchSummary::default())
}

pub(super) fn bundle_patch(items: Vec<ItemStackTemplateSummary>) -> DataComponentPatchSummary {
    patch_with(&[50], |p| {
        p.bundle_contents_items = items;
    })
}

pub(super) fn container_patch(
    items: Vec<ItemStackTemplateSummary>,
    slot_count: Option<usize>,
) -> DataComponentPatchSummary {
    patch_with(&[75], |p| {
        p.container = container_from_items(items, slot_count);
    })
}

pub(super) fn bee_occupants(count: usize) -> Vec<bbb_protocol::packets::BeeOccupantSummary> {
    vec![
        bbb_protocol::packets::BeeOccupantSummary {
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
            entity_data_tag: None,
            ticks_in_hive: 0,
            min_ticks_in_hive: 0,
        };
        count
    ]
}

pub(super) fn container_from_items(
    items: Vec<ItemStackTemplateSummary>,
    slot_count: Option<usize>,
) -> bbb_protocol::packets::data_components::ContainerContentsSummary {
    let slots = items
        .into_iter()
        .enumerate()
        .map(|(slot, item)| bbb_protocol::packets::ItemContainerSlotSummary { slot, item })
        .collect::<Vec<_>>();
    bbb_protocol::packets::data_components::ContainerContentsSummary {
        slot_count: slot_count.or(Some(slots.len())),
        slots,
    }
}

pub(super) static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

pub(super) fn assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft")
}

pub(super) fn write_item_atlases(assets: &Path) {
    for (file, prefix, source) in [
        ("items.json", "item/", "item"),
        ("blocks.json", "block/", "block"),
    ] {
        write_json(
            &assets.join("atlases").join(file),
            &serde_json::json!({
                "sources": [{ "type": "minecraft:directory", "prefix": prefix, "source": source }]
            })
            .to_string(),
        );
    }
}

pub(super) fn write_elytra_damage_condition_fixture(
    root: &Path,
    property: &str,
    true_model: &str,
    true_color: &[u8],
) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "elytra");
    write_item_registry_override_with_profiles(root, &["elytra"], &[("elytra", "elytra")]);
    write_json(
        &assets.join("items").join("elytra.json"),
        &serde_json::json!({
            "model": {
                "type": "minecraft:condition",
                "property": property,
                "on_false": { "type": "minecraft:model", "model": "minecraft:item/elytra" },
                "on_true": { "type": "minecraft:model", "model": format!("minecraft:item/{true_model}") }
            }
        })
        .to_string(),
    );
    write_flat_item_model_and_texture(&assets, "elytra", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, true_model, true_color);
}

pub(super) fn write_wolf_armor_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "wolf_armor");
    write_json(
        &assets.join("items").join("wolf_armor.json"),
        r#"{"model":{"type":"minecraft:condition","property":"minecraft:has_component","component":"minecraft:dyed_color","on_false":{"type":"minecraft:model","model":"minecraft:item/wolf_armor"},"on_true":{"type":"minecraft:model","model":"minecraft:item/wolf_armor_dyed","tints":[{"type":"minecraft:dye","default":0}]}}}"#,
    );
    write_flat_item_model_and_texture(&assets, "wolf_armor", &[40, 80, 120, 255]);
    write_flat_item_model_and_texture(&assets, "wolf_armor_dyed", &[120, 80, 40, 255]);
}

pub(super) fn write_compass_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_single_item_registry_fixture(root, "compass");
    write_json(
        &assets.join("items").join("compass.json"),
        r#"{"model":{"type":"minecraft:condition","property":"minecraft:has_component","component":"minecraft:lodestone_tracker","on_true":{"type":"minecraft:range_dispatch","property":"minecraft:compass","target":"lodestone","scale":32.0,"entries":[{"threshold":0.0,"model":{"type":"minecraft:model","model":"minecraft:item/compass_lodestone"}}]},"on_false":{"type":"minecraft:range_dispatch","property":"minecraft:compass","target":"spawn","scale":32.0,"entries":[{"threshold":0.0,"model":{"type":"minecraft:model","model":"minecraft:item/compass"}}]}}}"#,
    );
    write_flat_item_model_and_texture(&assets, "compass", &[40, 120, 80, 255]);
    write_flat_item_model_and_texture(&assets, "compass_lodestone", &[120, 40, 80, 255]);
}

pub(super) fn write_default_has_component_fixture(root: &Path) {
    let assets = assets_dir(root);
    write_item_atlases(&assets);
    write_item_registry_fixture(
        root,
        &[
            "has_max_stack",
            "has_max_stack_ignore_default",
            "has_rarity",
            "has_enchantments",
            "enchanted_book",
        ],
    );
    for (item_id, component, ignore_default, true_model, false_model, true_rgba, false_rgba) in [
        (
            "has_max_stack",
            "minecraft:max_stack_size",
            false,
            "has_max_stack_present",
            "has_max_stack_absent",
            [40, 140, 80, 255],
            [80, 40, 40, 255],
        ),
        (
            "has_max_stack_ignore_default",
            "minecraft:max_stack_size",
            true,
            "has_max_stack_patched",
            "has_max_stack_unpatched",
            [40, 80, 180, 255],
            [40, 40, 80, 255],
        ),
        (
            "has_rarity",
            "minecraft:rarity",
            false,
            "has_rarity_present",
            "has_rarity_absent",
            [180, 80, 220, 255],
            [60, 40, 80, 255],
        ),
        (
            "has_enchantments",
            "minecraft:enchantments",
            false,
            "has_enchantments_present",
            "has_enchantments_absent",
            [220, 200, 80, 255],
            [80, 70, 30, 255],
        ),
        (
            "enchanted_book",
            "minecraft:stored_enchantments",
            false,
            "has_stored_enchantments_present",
            "has_stored_enchantments_absent",
            [230, 210, 100, 255],
            [90, 80, 40, 255],
        ),
    ] {
        let mut model = serde_json::json!({
            "type": "minecraft:condition",
            "property": "minecraft:has_component",
            "component": component,
            "on_true": { "type": "minecraft:model", "model": format!("minecraft:item/{true_model}") },
            "on_false": { "type": "minecraft:model", "model": format!("minecraft:item/{false_model}") },
        });
        if ignore_default {
            model["ignore_default"] = serde_json::json!(true);
        }
        write_json(
            &assets.join("items").join(format!("{item_id}.json")),
            &serde_json::json!({ "model": model }).to_string(),
        );
        write_flat_item_model_and_texture(&assets, true_model, &true_rgba);
        write_flat_item_model_and_texture(&assets, false_model, &false_rgba);
    }
}
