use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use bbb_protocol::{
    content_registry::ContentRegistry,
    data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as DAMAGE_COMPONENT_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID as MAX_DAMAGE_COMPONENT_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID as UNBREAKABLE_COMPONENT_ID,
    },
    item_types::VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
    packets::{DataComponentPatchSummary, ItemStackSummary},
};
use rustc_hash::FxHashMap;

fn component_source<'a>(
    defaults: impl FnOnce() -> Option<&'a DataComponentPatchSummary>,
    patch: &'a DataComponentPatchSummary,
    component_id: i32,
) -> &'a DataComponentPatchSummary {
    if patch.added_type_ids.contains(&component_id)
        || patch.removed_type_ids.contains(&component_id)
    {
        return patch;
    }
    defaults().unwrap_or(patch)
}

/// Immutable item component prototype defaults shared between client world state
/// and renderer item-model evaluation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemComponentDefaults {
    pub default_component_codec_values: Arc<FxHashMap<i32, FxHashMap<i32, serde_json::Value>>>,
    pub default_item_component_patches: Arc<FxHashMap<i32, DataComponentPatchSummary>>,
}

impl ItemComponentDefaults {
    pub fn from_maps(
        codecs: FxHashMap<i32, FxHashMap<i32, serde_json::Value>>,
        patches: FxHashMap<i32, DataComponentPatchSummary>,
    ) -> Self {
        Self {
            default_component_codec_values: Arc::new(codecs),
            default_item_component_patches: Arc::new(patches),
        }
    }

    pub fn from_btree_maps(
        codecs: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
        patches: BTreeMap<i32, DataComponentPatchSummary>,
    ) -> Self {
        Self {
            default_component_codec_values: Arc::new(
                codecs
                    .into_iter()
                    .filter(|(item_id, values)| *item_id >= 0 && !values.is_empty())
                    .map(|(id, values)| (id, values.into_iter().collect()))
                    .collect(),
            ),
            default_item_component_patches: Arc::new(patches.into_iter().collect()),
        }
    }

    pub fn from_hash_maps(
        codecs: HashMap<i32, BTreeMap<i32, serde_json::Value>>,
        patches: HashMap<i32, DataComponentPatchSummary>,
    ) -> Self {
        Self {
            default_component_codec_values: Arc::new(
                codecs
                    .into_iter()
                    .filter(|(item_id, values)| *item_id >= 0 && !values.is_empty())
                    .map(|(id, values)| (id, values.into_iter().collect()))
                    .collect(),
            ),
            default_item_component_patches: Arc::new(patches.into_iter().collect()),
        }
    }

    pub fn as_view(&self) -> ItemComponentDefaultsView<'_> {
        ItemComponentDefaultsView {
            default_component_codec_values: &self.default_component_codec_values,
            default_item_component_patches: &self.default_item_component_patches,
        }
    }

    pub fn component_source<'a>(
        &'a self,
        item_id: Option<i32>,
        patch: &'a DataComponentPatchSummary,
        component_id: i32,
    ) -> &'a DataComponentPatchSummary {
        self.as_view()
            .component_source(item_id, patch, component_id)
    }

    pub fn item_stack_max_stack_size(&self, stack: &ItemStackSummary) -> i32 {
        self.as_view().item_stack_max_stack_size(stack)
    }

    pub fn item_stack_max_stack_size_with_registry(
        &self,
        stack: &ItemStackSummary,
        content_registry: &ContentRegistry,
    ) -> i32 {
        self.as_view()
            .item_stack_max_stack_size_with_registry(stack, content_registry)
    }

    pub fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        self.as_view().item_max_stack_size_for_protocol_id(item_id)
    }

    pub fn item_max_stack_size_for_protocol_id_with_registry(
        &self,
        item_id: i32,
        content_registry: &ContentRegistry,
    ) -> i32 {
        self.as_view()
            .item_max_stack_size_for_protocol_id_with_registry(item_id, content_registry)
    }

    pub fn item_max_damage_for_protocol_id(&self, item_id: i32) -> Option<i32> {
        self.as_view().item_max_damage_for_protocol_id(item_id)
    }

    pub fn damage_state(
        defaults: Option<&Self>,
        item_id: Option<i32>,
        patch: &DataComponentPatchSummary,
    ) -> Option<(i32, i32)> {
        ItemComponentDefaultsView::damage_state(defaults.map(Self::as_view), item_id, patch)
    }
}

/// Borrowed view over [`ItemComponentDefaults`] or world item prototype maps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemComponentDefaultsView<'a> {
    pub default_component_codec_values: &'a FxHashMap<i32, FxHashMap<i32, serde_json::Value>>,
    pub default_item_component_patches: &'a FxHashMap<i32, DataComponentPatchSummary>,
}

impl<'a> ItemComponentDefaultsView<'a> {
    pub fn component_source(
        self,
        item_id: Option<i32>,
        patch: &'a DataComponentPatchSummary,
        component_id: i32,
    ) -> &'a DataComponentPatchSummary {
        component_source(
            || item_id.and_then(|item_id| self.default_item_component_patches.get(&item_id)),
            patch,
            component_id,
        )
    }

    pub fn item_stack_max_stack_size(self, stack: &ItemStackSummary) -> i32 {
        self.item_stack_max_stack_size_inner(stack, None)
    }

    pub fn item_stack_max_stack_size_with_registry(
        self,
        stack: &ItemStackSummary,
        content_registry: &ContentRegistry,
    ) -> i32 {
        self.item_stack_max_stack_size_inner(stack, Some(content_registry))
    }

    fn item_stack_max_stack_size_inner(
        self,
        stack: &ItemStackSummary,
        content_registry: Option<&ContentRegistry>,
    ) -> i32 {
        if stack.item_id.is_none() || stack.count <= 0 {
            return 0;
        }
        if stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID)
        {
            return 1;
        }
        self.component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        )
        .max_stack_size
        .or_else(|| {
            let item_id = stack.item_id?;
            let spec = content_registry?.item_by_id(item_id)?;
            // Vanilla `DataComponents.MAX_STACK_SIZE` range (`DataComponents.java:109-111`).
            Some(i32::from(spec.max_stack_size).clamp(1, 99))
        })
        .unwrap_or(VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE)
    }

    pub fn item_max_stack_size_for_protocol_id(self, item_id: i32) -> i32 {
        self.item_max_stack_size_for_protocol_id_inner(item_id, None)
    }

    pub fn item_max_stack_size_for_protocol_id_with_registry(
        self,
        item_id: i32,
        content_registry: &ContentRegistry,
    ) -> i32 {
        self.item_max_stack_size_for_protocol_id_inner(item_id, Some(content_registry))
    }

    fn item_max_stack_size_for_protocol_id_inner(
        self,
        item_id: i32,
        content_registry: Option<&ContentRegistry>,
    ) -> i32 {
        self.default_item_component_patches
            .get(&item_id)
            .and_then(|patch| patch.max_stack_size)
            .or_else(|| {
                let spec = content_registry?.item_by_id(item_id)?;
                // Vanilla `DataComponents.MAX_STACK_SIZE` range (`DataComponents.java:109-111`).
                Some(i32::from(spec.max_stack_size).clamp(1, 99))
            })
            .unwrap_or(VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE)
    }

    pub fn item_max_damage_for_protocol_id(self, item_id: i32) -> Option<i32> {
        self.default_item_component_patches
            .get(&item_id)
            .and_then(|patch| patch.max_damage)
    }

    pub fn damage_state(
        defaults: Option<Self>,
        item_id: Option<i32>,
        patch: &DataComponentPatchSummary,
    ) -> Option<(i32, i32)> {
        if patch.removed_type_ids.contains(&MAX_DAMAGE_COMPONENT_ID)
            || patch.removed_type_ids.contains(&DAMAGE_COMPONENT_ID)
            || (patch.added_type_ids.contains(&UNBREAKABLE_COMPONENT_ID)
                && !patch.removed_type_ids.contains(&UNBREAKABLE_COMPONENT_ID))
        {
            return None;
        }
        let default_patch = defaults.and_then(|defaults| {
            item_id.and_then(|item_id| defaults.default_item_component_patches.get(&item_id))
        });
        let max_damage =
            component_source(|| default_patch, patch, MAX_DAMAGE_COMPONENT_ID).max_damage?;
        let damage = component_source(|| default_patch, patch, DAMAGE_COMPONENT_ID).damage;
        let default_values = defaults.and_then(|defaults| {
            item_id.and_then(|item_id| defaults.default_component_codec_values.get(&item_id))
        });
        let has_component = |component_id| {
            !patch.removed_type_ids.contains(&component_id)
                && (patch.added_type_ids.contains(&component_id)
                    || default_patch
                        .is_some_and(|defaults| defaults.added_type_ids.contains(&component_id))
                    || default_values.is_some_and(|defaults| defaults.contains_key(&component_id)))
        };
        if has_component(UNBREAKABLE_COMPONENT_ID)
            || (damage.is_none() && !has_component(DAMAGE_COMPONENT_ID))
        {
            return None;
        }
        // Wire `DataComponents.MAX_DAMAGE` is a signed VarInt (`DataComponents.java:112-114`),
        // whereas Vanilla's `Mth.clamp` (`ItemStack.java:425-431`, `Mth.java:93-95`) never panics
        // on non-positive upper bounds. Clamping `max_damage` to >= 0 guarantees
        // `0 <= damage <= max_damage` and prevents downstream `max_damage - 1` underflow or
        // division by zero/negative when `damage > 0`.
        let max_damage = max_damage.max(0);
        Some((damage.unwrap_or(0).clamp(0, max_damage), max_damage))
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModItemRegistration};

    use super::*;

    fn stack_with_patch(
        item_id: Option<i32>,
        count: i32,
        patch: DataComponentPatchSummary,
    ) -> ItemStackSummary {
        ItemStackSummary {
            count,
            item_id,
            component_patch: Box::new(patch).into(),
        }
    }

    #[test]
    fn damage_state_clamps_negative_and_zero_max_damage_without_panicking() {
        for (raw_max_damage, raw_damage, expected) in [
            (-5, 3, (0, 0)),
            (i32::MIN, 10, (0, 0)),
            (0, 5, (0, 0)),
            (100, -10, (0, 100)),
            (100, 25, (25, 100)),
            (100, 250, (100, 100)),
        ] {
            let patch = DataComponentPatchSummary {
                added_type_ids: vec![MAX_DAMAGE_COMPONENT_ID, DAMAGE_COMPONENT_ID],
                max_damage: Some(raw_max_damage),
                damage: Some(raw_damage),
                ..Default::default()
            };
            let result = ItemComponentDefaults::damage_state(None, Some(1), &patch);
            assert_eq!(result, Some(expected));
            let (damage, max_damage) = result.unwrap();
            assert!(damage >= 0 && damage <= max_damage);
            // Downstream `Broken` checks compute `max_damage - 1`; verify no underflow.
            let _ = max_damage - 1;
        }

        let mut default_patches = FxHashMap::default();
        default_patches.insert(
            42,
            DataComponentPatchSummary {
                added_type_ids: vec![MAX_DAMAGE_COMPONENT_ID, DAMAGE_COMPONENT_ID],
                max_damage: Some(-12),
                damage: Some(0),
                ..Default::default()
            },
        );
        let defaults = ItemComponentDefaults::from_maps(FxHashMap::default(), default_patches);
        let patch = DataComponentPatchSummary {
            added_type_ids: vec![DAMAGE_COMPONENT_ID],
            damage: Some(4),
            ..Default::default()
        };
        assert_eq!(
            ItemComponentDefaults::damage_state(Some(&defaults), Some(42), &patch),
            Some((0, 0))
        );
    }

    #[test]
    fn registry_aware_max_stack_size_honors_mod_item_spec_and_component_patches() {
        let mut builder = ContentRegistryBuilder::new();
        builder
            .register_item(ModItemRegistration {
                resource_id: "example:relic_blade".to_string(),
                max_stack_size: 1,
                is_block_item: false,
            })
            .register_item(ModItemRegistration {
                resource_id: "example:frost_orb".to_string(),
                max_stack_size: 16,
                is_block_item: false,
            })
            .register_item(ModItemRegistration {
                resource_id: "example:compressed_gem".to_string(),
                max_stack_size: 99,
                is_block_item: false,
            });
        let registry = builder.freeze_for_session().expect("valid mod registry");

        let blade_id = registry
            .item_by_resource_id("example:relic_blade")
            .unwrap()
            .id;
        let orb_id = registry
            .item_by_resource_id("example:frost_orb")
            .unwrap()
            .id;
        let gem_id = registry
            .item_by_resource_id("example:compressed_gem")
            .unwrap()
            .id;

        let mut default_patches = FxHashMap::default();
        default_patches.insert(
            7,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(16),
                ..Default::default()
            },
        );
        let defaults = ItemComponentDefaults::from_maps(FxHashMap::default(), default_patches);

        // Protocol-id resolution returns ModItemSpec::max_stack_size for mod items,
        // prototype default for vanilla items, and 64 for unknown ids.
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(blade_id, &registry),
            1
        );
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(orb_id, &registry),
            16
        );
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(gem_id, &registry),
            99
        );
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(7, &registry),
            16
        );
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(1, &registry),
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE
        );
        assert_eq!(
            defaults.item_max_stack_size_for_protocol_id_with_registry(gem_id + 100, &registry),
            VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE
        );

        // Unpatched mod stacks use ModItemSpec::max_stack_size.
        let orb_stack = stack_with_patch(Some(orb_id), 1, DataComponentPatchSummary::default());
        assert_eq!(
            defaults.item_stack_max_stack_size_with_registry(&orb_stack, &registry),
            16
        );

        // Explicit patch override takes precedence over ModItemSpec::max_stack_size.
        let overridden_orb = stack_with_patch(
            Some(orb_id),
            1,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(32),
                ..Default::default()
            },
        );
        assert_eq!(
            defaults.item_stack_max_stack_size_with_registry(&overridden_orb, &registry),
            32
        );

        // Removed max_stack_size component collapses to 1 just like Vanilla.
        let unstackable_orb = stack_with_patch(
            Some(orb_id),
            1,
            DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                ..Default::default()
            },
        );
        assert_eq!(
            defaults.item_stack_max_stack_size_with_registry(&unstackable_orb, &registry),
            1
        );

        // Empty or zero-count stacks still report 0.
        let empty_mod = stack_with_patch(Some(orb_id), 0, DataComponentPatchSummary::default());
        assert_eq!(
            defaults.item_stack_max_stack_size_with_registry(&empty_mod, &registry),
            0
        );
    }
}
