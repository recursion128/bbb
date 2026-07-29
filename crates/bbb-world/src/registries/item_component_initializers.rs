use std::{borrow::Cow, collections::BTreeMap};

use bbb_pack::RegistryDependentItemComponentInitializer;
use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
    },
    packets::{
        validate_data_component_patch_summary, BlocksAttacksSummary, DamageReductionSummary,
        DamageTypeHolderSetSummary, DataComponentPatchSummary, ItemDamageFunctionSummary,
        ItemStackSummary, SoundEventSummary,
    },
};

use crate::WorldStore;

use super::{FrozenRegistryAccess, RegistryResolutionError, RegistryValidationError};

const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";
const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";
const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";
const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";
const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
const BYPASSES_SHIELD_TAG: &str = "minecraft:bypasses_shield";

pub(super) fn build_item_component_defaults(
    codec_values: &BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    registry_dependent_rules: &BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>,
    access: FrozenRegistryAccess<'_>,
) -> Result<BTreeMap<i32, DataComponentPatchSummary>, RegistryValidationError> {
    if codec_values.is_empty() {
        return build_registry_dependent_item_defaults(registry_dependent_rules, access);
    }
    codec_values
        .iter()
        .map(|(item_id, values)| {
            let mut defaults = DataComponentPatchSummary::default();
            for (type_id, value) in values {
                let decoded = crate::decode_persistent_component_select_value_with_registry_access(
                    *type_id,
                    value,
                    Some(access),
                )
                .map_err(|message| prototype_codec_error(*item_id, *type_id, message))?
                .ok_or_else(|| {
                    prototype_codec_error(
                        *item_id,
                        *type_id,
                        "persistent component has no typed projection".to_string(),
                    )
                })?;
                crate::item_profiles::copy_persistent_component_value_summary(
                    *type_id,
                    &decoded,
                    &mut defaults,
                )
                .map_err(|error| prototype_codec_error(*item_id, *type_id, error.to_string()))?;
                defaults.added_type_ids.push(*type_id);
            }
            defaults.added = defaults.added_type_ids.len();
            validate_data_component_patch_summary(&crate::component_select_codec_projection(
                &defaults,
            ))
            .map_err(|error| prototype_codec_error(*item_id, -1, error.to_string()))?;
            Ok((*item_id, defaults))
        })
        .collect()
}

fn prototype_codec_error(
    item_id: i32,
    component_type_id: i32,
    message: String,
) -> RegistryValidationError {
    RegistryValidationError {
        registry: "minecraft:item".to_string(),
        validator: "DataComponentInitializers.persistentCodec".to_string(),
        message: format!(
            "item {item_id} component {component_type_id} prototype decode failed: {message}"
        ),
    }
}

pub(super) fn build_registry_dependent_item_defaults(
    rules: &BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>,
    access: FrozenRegistryAccess<'_>,
) -> Result<BTreeMap<i32, DataComponentPatchSummary>, RegistryValidationError> {
    rules
        .iter()
        .map(|(item_id, initializers)| {
            let mut defaults = DataComponentPatchSummary::default();
            for initializer in initializers {
                apply_initializer(&mut defaults, initializer, access)?;
            }
            defaults.added = defaults.added_type_ids.len();
            Ok((*item_id, defaults))
        })
        .collect()
}

fn apply_initializer(
    defaults: &mut DataComponentPatchSummary,
    initializer: &RegistryDependentItemComponentInitializer,
    access: FrozenRegistryAccess<'_>,
) -> Result<(), RegistryValidationError> {
    match initializer {
        RegistryDependentItemComponentInitializer::DamageResistant { tag } => {
            insert_component(defaults, VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID)?;
            let holder_set = access
                .holder_set_by_tag(DAMAGE_TYPE_REGISTRY, tag)
                .map_err(initializer_error)?;
            defaults.damage_resistant_type_registry_ids = holder_set
                .entries
                .iter()
                .map(|entry| entry.holder_id)
                .collect();
            defaults.damage_resistant_type_tag = Some(tag.clone());
        }
        RegistryDependentItemComponentInitializer::DamageType { damage_type } => {
            insert_component(defaults, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID)?;
            defaults.damage_type_id = Some(
                access
                    .require_entry_by_resource_id(DAMAGE_TYPE_REGISTRY, damage_type)
                    .map_err(initializer_error)?
                    .holder_id,
            );
        }
        RegistryDependentItemComponentInitializer::JukeboxPlayable { song } => {
            insert_component(defaults, VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)?;
            defaults.jukebox_song_id = Some(
                access
                    .require_entry_by_resource_id(JUKEBOX_SONG_REGISTRY, song)
                    .map_err(initializer_error)?
                    .holder_id,
            );
        }
        RegistryDependentItemComponentInitializer::BlocksAttacks => {
            insert_component(defaults, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)?;
            let bypassed_by = access
                .holder_set_by_tag(DAMAGE_TYPE_REGISTRY, BYPASSES_SHIELD_TAG)
                .map_err(initializer_error)?;
            defaults.blocks_attacks = Some(BlocksAttacksSummary {
                block_delay_seconds_bits: 0.25_f32.to_bits(),
                disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                damage_reductions: vec![DamageReductionSummary {
                    horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                    damage_types: None,
                    base_bits: 0.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                }],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: 3.0_f32.to_bits(),
                    base_bits: 1.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                },
                bypassed_by: Some(DamageTypeHolderSetSummary {
                    registry_ids: bypassed_by
                        .entries
                        .iter()
                        .map(|entry| entry.holder_id)
                        .collect(),
                    tag: Some(BYPASSES_SHIELD_TAG.to_string()),
                }),
                block_sound: Some(sound_event(access, "minecraft:item.shield.block")?),
                disable_sound: Some(sound_event(access, "minecraft:item.shield.break")?),
            });
        }
        RegistryDependentItemComponentInitializer::ProvidesBannerPatterns { tag } => {
            insert_component(
                defaults,
                VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            )?;
            let holder_set = access
                .holder_set_by_tag(BANNER_PATTERN_REGISTRY, tag)
                .map_err(initializer_error)?;
            defaults.provides_banner_pattern_registry_ids = holder_set
                .entries
                .iter()
                .map(|entry| entry.holder_id)
                .collect();
            defaults.provides_banner_pattern_tag = Some(tag.clone());
        }
        RegistryDependentItemComponentInitializer::ProvidesTrimMaterial { material } => {
            insert_component(
                defaults,
                VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            )?;
            defaults.provides_trim_material_id = Some(
                access
                    .require_entry_by_resource_id(TRIM_MATERIAL_REGISTRY, material)
                    .map_err(initializer_error)?
                    .holder_id,
            );
        }
        RegistryDependentItemComponentInitializer::Instrument { instrument } => {
            insert_component(defaults, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID)?;
            defaults.instrument_id = Some(
                access
                    .require_entry_by_resource_id(INSTRUMENT_REGISTRY, instrument)
                    .map_err(initializer_error)?
                    .holder_id,
            );
        }
    }
    Ok(())
}

fn insert_component(
    defaults: &mut DataComponentPatchSummary,
    component_id: i32,
) -> Result<(), RegistryValidationError> {
    if defaults.added_type_ids.contains(&component_id) {
        return Err(RegistryValidationError {
            registry: "minecraft:item".to_string(),
            validator: "DataComponentInitializers.apply".to_string(),
            message: format!("duplicate registry-dependent item component {component_id}"),
        });
    }
    defaults.added_type_ids.push(component_id);
    Ok(())
}

fn sound_event(
    access: FrozenRegistryAccess<'_>,
    resource_id: &str,
) -> Result<SoundEventSummary, RegistryValidationError> {
    Ok(SoundEventSummary {
        registry_id: Some(
            access
                .require_entry_by_resource_id(SOUND_EVENT_REGISTRY, resource_id)
                .map_err(initializer_error)?
                .holder_id,
        ),
        sound_id: None,
        fixed_range_bits: None,
    })
}

fn initializer_error(error: RegistryResolutionError) -> RegistryValidationError {
    RegistryValidationError {
        registry: "minecraft:item".to_string(),
        validator: "DataComponentInitializers.apply".to_string(),
        message: error.to_string(),
    }
}

impl WorldStore {
    pub fn set_default_item_component_type_ids(
        &mut self,
        defaults: BTreeMap<i32, std::collections::BTreeSet<i32>>,
    ) {
        self.items.set_default_component_type_ids(defaults);
    }

    pub fn set_default_item_component_codec_values(
        &mut self,
        values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        self.items.set_default_component_codec_values(values);
    }

    pub fn set_default_item_dye_color_ids(&mut self, defaults: BTreeMap<i32, i32>) {
        self.items.set_default_dye_color_ids(defaults);
    }

    pub fn set_default_item_potion_duration_scale_bits(&mut self, defaults: BTreeMap<i32, u32>) {
        self.items.set_default_potion_duration_scale_bits(defaults);
    }

    pub fn set_default_item_use_cooldown_seconds_bits(&mut self, defaults: BTreeMap<i32, u32>) {
        self.items.set_default_use_cooldown_seconds_bits(defaults);
    }

    pub fn set_registry_dependent_item_component_initializers(
        &mut self,
        rules: BTreeMap<i32, Vec<RegistryDependentItemComponentInitializer>>,
    ) {
        self.items
            .set_registry_dependent_component_initializer_rules(rules);
    }

    pub fn rebuild_item_component_defaults(&mut self) -> Result<(), RegistryValidationError> {
        let defaults = build_item_component_defaults(
            &self.items.default_component_codec_values,
            &self.items.registry_dependent_component_initializer_rules,
            self.registries.frozen_access(),
        )?;
        self.items.commit_item_component_defaults(defaults);
        Ok(())
    }

    /// Returns the stack view after applying registry-bound prototype defaults.
    /// Explicit patch additions and removals always win over the prototype.
    pub fn effective_item_stack<'a>(
        &self,
        stack: &'a ItemStackSummary,
    ) -> Cow<'a, ItemStackSummary> {
        let Some(item_id) = stack.item_id else {
            return Cow::Borrowed(stack);
        };
        let Some(component_patch) = self
            .items
            .effective_component_patch(item_id, &stack.component_patch)
        else {
            return Cow::Borrowed(stack);
        };
        let mut effective = stack.clone();
        effective.component_patch = component_patch;
        Cow::Owned(effective)
    }

    pub fn item_stack_has_effective_component(
        &self,
        stack: &ItemStackSummary,
        component_id: i32,
    ) -> bool {
        self.items
            .item_stack_has_effective_component(stack, component_id)
    }

    /// Returns the synchronized tooltip-style resource id only when its patch
    /// component is effective. Explicit removals therefore suppress stale
    /// projection fields rather than leaking them into presentation.
    pub fn item_stack_tooltip_style<'a>(&self, stack: &'a ItemStackSummary) -> Option<&'a str> {
        self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID)
            .then(|| stack.component_patch.tooltip_style.as_deref())
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bbb_pack::{ItemRegistryCatalog, RegistryDependentItemComponentInitializer as Initializer};
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
    };

    use super::*;
    use crate::{RegistryContentState, RegistryPacketEntry, RegistryTagState};

    #[test]
    fn tooltip_style_query_honors_component_membership_and_removal() {
        let world = WorldStore::new();
        let mut stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tooltip_style: Some("example:rare".to_string()),
                ..DataComponentPatchSummary::default()
            },
        };
        assert_eq!(world.item_stack_tooltip_style(&stack), None);

        stack.component_patch.added = 1;
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID);
        assert_eq!(world.item_stack_tooltip_style(&stack), Some("example:rare"));

        stack
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID);
        assert_eq!(world.item_stack_tooltip_style(&stack), None);
    }

    #[test]
    fn resolves_all_initializer_families_through_candidate_holder_order_and_tags() {
        let mut world = WorldStore::new();
        insert_registry(
            &mut world,
            DAMAGE_TYPE_REGISTRY,
            &[
                "example:custom",
                "minecraft:in_fire",
                "minecraft:explosion",
                "minecraft:spear",
            ],
        );
        insert_registry(
            &mut world,
            JUKEBOX_SONG_REGISTRY,
            &["example:custom", "minecraft:13"],
        );
        insert_registry(
            &mut world,
            INSTRUMENT_REGISTRY,
            &["example:custom", "minecraft:ponder_goat_horn"],
        );
        insert_registry(
            &mut world,
            BANNER_PATTERN_REGISTRY,
            &["example:custom", "minecraft:flower"],
        );
        insert_registry(
            &mut world,
            TRIM_MATERIAL_REGISTRY,
            &["example:custom", "minecraft:diamond"],
        );
        insert_tag(
            &mut world,
            DAMAGE_TYPE_REGISTRY,
            "minecraft:is_fire",
            vec![1],
        );
        insert_tag(
            &mut world,
            DAMAGE_TYPE_REGISTRY,
            BYPASSES_SHIELD_TAG,
            vec![0, 2],
        );
        insert_tag(
            &mut world,
            BANNER_PATTERN_REGISTRY,
            "minecraft:pattern_item/flower",
            vec![1],
        );
        world.set_registry_dependent_item_component_initializers(BTreeMap::from([
            (
                1,
                vec![Initializer::DamageResistant {
                    tag: "minecraft:is_fire".to_string(),
                }],
            ),
            (
                2,
                vec![Initializer::JukeboxPlayable {
                    song: "minecraft:13".to_string(),
                }],
            ),
            (3, vec![Initializer::BlocksAttacks]),
            (
                4,
                vec![Initializer::ProvidesBannerPatterns {
                    tag: "minecraft:pattern_item/flower".to_string(),
                }],
            ),
            (
                5,
                vec![Initializer::Instrument {
                    instrument: "minecraft:ponder_goat_horn".to_string(),
                }],
            ),
            (
                6,
                vec![Initializer::ProvidesTrimMaterial {
                    material: "minecraft:diamond".to_string(),
                }],
            ),
            (
                7,
                vec![Initializer::DamageType {
                    damage_type: "minecraft:spear".to_string(),
                }],
            ),
        ]));

        world.rebuild_item_component_defaults().unwrap();

        let defaults = &world.items.default_item_component_patches;
        assert_eq!(defaults[&1].damage_resistant_type_registry_ids, vec![1]);
        assert_eq!(defaults[&2].jukebox_song_id, Some(1));
        assert_eq!(
            defaults[&3]
                .blocks_attacks
                .as_ref()
                .unwrap()
                .bypassed_by
                .as_ref()
                .unwrap()
                .registry_ids,
            vec![0, 2]
        );
        assert_eq!(defaults[&4].provides_banner_pattern_registry_ids, vec![1]);
        assert_eq!(defaults[&5].instrument_id, Some(1));
        assert_eq!(defaults[&6].provides_trim_material_id, Some(1));
        assert_eq!(defaults[&7].damage_type_id, Some(3));
        assert_eq!(
            defaults[&6].added_type_ids,
            vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID]
        );
        assert_eq!(
            defaults[&7].added_type_ids,
            vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID]
        );
    }

    #[test]
    fn effective_stack_applies_defaults_without_overriding_patch_or_remove() {
        let mut world = WorldStore::new();
        world.items.commit_item_component_defaults(BTreeMap::from([(
            7,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                ],
                blocks_attacks: Some(BlocksAttacksSummary {
                    block_delay_seconds_bits: 0,
                    disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                    damage_reductions: Vec::new(),
                    item_damage: ItemDamageFunctionSummary {
                        threshold_bits: 0,
                        base_bits: 0,
                        factor_bits: 0,
                    },
                    bypassed_by: None,
                    block_sound: None,
                    disable_sound: None,
                }),
                jukebox_song_id: Some(9),
                ..DataComponentPatchSummary::default()
            },
        )]));
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID],
                jukebox_song_id: Some(42),
                ..DataComponentPatchSummary::default()
            },
        };

        let effective = world.effective_item_stack(&stack);
        assert_eq!(effective.component_patch.jukebox_song_id, Some(42));
        assert!(effective.component_patch.blocks_attacks.is_some());
        assert_eq!(effective.component_patch.added, 2);
        assert_eq!(stack.component_patch.blocks_attacks, None);

        let mut removed = stack.clone();
        removed
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID);
        let effective = world.effective_item_stack(&removed);
        assert_eq!(effective.component_patch.blocks_attacks, None);
        assert!(!world.item_stack_has_effective_component(
            &removed,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
        ));
    }

    #[test]
    fn damage_type_default_uses_session_order_and_patch_remove_precedence() {
        let mut world = WorldStore::new();
        insert_registry(
            &mut world,
            DAMAGE_TYPE_REGISTRY,
            &["example:custom", "minecraft:spear"],
        );
        world.set_registry_dependent_item_component_initializers(BTreeMap::from([(
            7,
            vec![Initializer::DamageType {
                damage_type: "minecraft:spear".to_string(),
            }],
        )]));
        world.rebuild_item_component_defaults().unwrap();

        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let effective = world.effective_item_stack(&stack);
        assert_eq!(effective.component_patch.damage_type_id, Some(1));
        assert_eq!(
            effective.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID]
        );

        let explicit = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
                damage_type_id: Some(0),
                ..DataComponentPatchSummary::default()
            },
            ..stack.clone()
        };
        assert_eq!(
            world
                .effective_item_stack(&explicit)
                .component_patch
                .damage_type_id,
            Some(0)
        );

        let removed = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
            ..stack
        };
        assert_eq!(
            world
                .effective_item_stack(&removed)
                .component_patch
                .damage_type_id,
            None
        );

        let mut missing = WorldStore::new();
        insert_registry(&mut missing, DAMAGE_TYPE_REGISTRY, &["example:custom"]);
        missing.set_registry_dependent_item_component_initializers(BTreeMap::from([(
            7,
            vec![Initializer::DamageType {
                damage_type: "minecraft:spear".to_string(),
            }],
        )]));
        let error = missing.rebuild_item_component_defaults().unwrap_err();
        assert!(error
            .message
            .contains("missing minecraft:damage_type resource"));
        assert!(missing.items.default_item_component_patches.is_empty());
    }

    #[test]
    fn effective_stack_applies_potion_duration_scale_default_with_patch_precedence() {
        let mut world = WorldStore::new();
        world.set_default_item_component_type_ids(BTreeMap::from([(
            7,
            BTreeSet::from([VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID]),
        )]));
        world.set_default_item_potion_duration_scale_bits(BTreeMap::from([(
            7,
            0.125_f32.to_bits(),
        )]));
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };

        let effective = world.effective_item_stack(&stack);
        assert_eq!(
            effective.component_patch.potion_duration_scale_bits,
            Some(0.125_f32.to_bits())
        );
        assert!(effective
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID));

        let explicit = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
                potion_duration_scale_bits: Some(0.5_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            ..stack.clone()
        };
        let effective = world.effective_item_stack(&explicit);
        assert_eq!(
            effective.component_patch.potion_duration_scale_bits,
            Some(0.5_f32.to_bits())
        );

        let removed = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
            ..stack
        };
        assert!(matches!(
            world.effective_item_stack(&removed),
            Cow::Borrowed(_)
        ));
        assert!(!world.item_stack_has_effective_component(
            &removed,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID
        ));
    }

    #[test]
    fn all_vanilla_item_prototypes_decode_and_patch_precedence_is_exact() {
        let catalog = ItemRegistryCatalog::vanilla_26_1().unwrap();
        let memberships = catalog.default_component_type_ids_by_protocol_id();
        let codec_values = catalog.default_component_codec_values_by_protocol_id();
        let mut world = WorldStore::new();
        insert_referenced_dynamic_prototype_entries(&mut world, &codec_values);
        insert_registry(&mut world, BANNER_PATTERN_REGISTRY, &["minecraft:base"]);
        insert_referenced_prototype_tags(&mut world, &codec_values);
        world.set_default_item_component_type_ids(memberships.clone());
        world.set_default_item_component_codec_values(codec_values);
        world.rebuild_item_component_defaults().unwrap();

        let defaults = &world.items.default_item_component_patches;
        assert_eq!(defaults.len(), 1506);
        assert_eq!(
            defaults
                .values()
                .map(|patch| patch.added_type_ids.len())
                .sum::<usize>(),
            18_981
        );
        for (item_id, patch) in &**defaults {
            assert_eq!(
                patch
                    .added_type_ids
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>(),
                memberships[item_id],
                "item {item_id} decoded prototype membership diverged"
            );
            validate_data_component_patch_summary(&crate::component_select_codec_projection(patch))
                .unwrap_or_else(|error| {
                    panic!("item {item_id} prototype is not encodable: {error}")
                });
        }

        let stick_id = catalog.protocol_id("minecraft:stick").unwrap();
        let stick = ItemStackSummary {
            item_id: Some(stick_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        let effective = world.effective_item_stack(&stick);
        assert_eq!(effective.component_patch.added, 12);
        assert_eq!(effective.component_patch.max_stack_size, Some(64));
        assert_eq!(
            effective.component_patch.item_model.as_deref(),
            Some("minecraft:stick")
        );
        assert_eq!(
            effective.component_patch.item_name.as_deref(),
            Some("item.minecraft.stick")
        );

        let wooden_pickaxe_id = catalog.protocol_id("minecraft:wooden_pickaxe").unwrap();
        let wooden_pickaxe = &defaults[&wooden_pickaxe_id];
        let named_tool_rules = wooden_pickaxe
            .tool
            .as_ref()
            .unwrap()
            .rules
            .iter()
            .filter(|rule| rule.block_tag.is_some())
            .collect::<Vec<_>>();
        assert_eq!(named_tool_rules.len(), 2);
        assert!(named_tool_rules
            .iter()
            .all(|rule| rule.block_registry_ids == [0]));

        let explicit = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
                max_stack_size: Some(16),
                ..DataComponentPatchSummary::default()
            },
            ..stick
        };
        let effective = world.effective_item_stack(&explicit);
        assert_eq!(effective.component_patch.max_stack_size, Some(16));
        assert!(!effective
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID));
        assert!(effective.component_patch.item_name.is_none());
        assert!(world.item_stack_has_effective_component(
            &explicit,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
        ));
        assert!(!world.item_stack_has_effective_component(
            &explicit,
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID
        ));
        validate_data_component_patch_summary(&effective.component_patch).unwrap();
    }

    fn insert_referenced_prototype_tags(
        world: &mut WorldStore,
        codec_values: &BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        let mut tags = BTreeSet::new();
        for value in codec_values.values().flat_map(BTreeMap::values) {
            collect_json_tags(value, &mut tags);
        }
        for registry in [
            "minecraft:block",
            "minecraft:item",
            "minecraft:entity_type",
            DAMAGE_TYPE_REGISTRY,
            BANNER_PATTERN_REGISTRY,
        ] {
            for tag in &tags {
                insert_tag(world, registry, tag, vec![0]);
            }
        }
    }

    fn insert_referenced_dynamic_prototype_entries(
        world: &mut WorldStore,
        codec_values: &BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        for (type_id, registry) in [
            (
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                DAMAGE_TYPE_REGISTRY,
            ),
            (
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                INSTRUMENT_REGISTRY,
            ),
            (
                VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
                TRIM_MATERIAL_REGISTRY,
            ),
            (
                VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                JUKEBOX_SONG_REGISTRY,
            ),
        ] {
            let entries = codec_values
                .values()
                .filter_map(|values| values.get(&type_id))
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<BTreeSet<_>>();
            insert_registry_owned(world, registry, entries);
        }
    }

    fn collect_json_tags(value: &serde_json::Value, tags: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::String(value) => {
                if let Some(tag) = value.strip_prefix('#') {
                    tags.insert(tag.to_string());
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_json_tags(value, tags);
                }
            }
            serde_json::Value::Object(entries) => {
                for value in entries.values() {
                    collect_json_tags(value, tags);
                }
            }
            _ => {}
        }
    }

    fn insert_registry(world: &mut WorldStore, registry: &str, entries: &[&str]) {
        insert_registry_owned(
            world,
            registry,
            entries.iter().map(|entry| (*entry).to_string()),
        );
    }

    fn insert_registry_owned(
        world: &mut WorldStore,
        registry: &str,
        entries: impl IntoIterator<Item = String>,
    ) {
        world.registries.contents.insert(
            registry.to_string(),
            RegistryContentState {
                registry: registry.to_string(),
                packet_count: 1,
                entries: entries.into_iter().map(RegistryPacketEntry::stub).collect(),
                duplicate_entry_ids: BTreeMap::new(),
            },
        );
    }

    fn insert_tag(world: &mut WorldStore, registry: &str, tag: &str, entries: Vec<i32>) {
        world
            .registries
            .tags
            .entry(registry.to_string())
            .or_insert_with(|| RegistryTagState {
                registry: registry.to_string(),
                tags: BTreeMap::new(),
            })
            .tags
            .insert(tag.to_string(), entries);
    }
}
