use std::{borrow::Cow, collections::BTreeMap};

use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID, VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    },
    packets::{validate_data_component_patch_summary, DataComponentPatchSummary, ItemStackSummary},
};

use crate::WorldStore;

use super::{
    item_component_values::{materialize_patch_named_holder_sets, read_persistent_component_into},
    FrozenRegistryAccess, RegistryValidationError,
};

const STATIC_COMPONENT_TYPES: [i32; 13] = [
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
];

pub(super) fn build_item_component_defaults<'a, V: 'a>(
    codec_values: impl IntoIterator<Item = (&'a i32, &'a V)>,
    access: Option<FrozenRegistryAccess<'_>>,
) -> Result<BTreeMap<i32, DataComponentPatchSummary>, RegistryValidationError>
where
    &'a V: IntoIterator<Item = (&'a i32, &'a serde_json::Value)>,
{
    use bbb_protocol::packets::data_components::fields::copy_component_value;
    use std::collections::hash_map::Entry;

    let mut items = BTreeMap::new();
    let mut shared_inputs = rustc_hash::FxHashMap::default();
    let mut shared_fields = DataComponentPatchSummary::default();
    for (item_id, values) in codec_values {
        let mut defaults = DataComponentPatchSummary::default();
        for (type_id, value) in values {
            if access.is_none() && !STATIC_COMPONENT_TYPES.contains(type_id) {
                continue;
            }
            let entry = shared_inputs.entry(*type_id);
            if matches!(&entry, Entry::Occupied(previous) if previous.get() == &value) {
                copy_component_value(*type_id, &shared_fields, &mut defaults)
                    .expect("previously decoded prototype component");
                defaults.added_type_ids.push(*type_id);
                continue;
            }
            let nbt = bbb_protocol::json_value_to_nbt_summary(value)
                .map_err(|error| prototype_codec_error(*item_id, *type_id, error.to_string()))?;
            read_persistent_component_into(*type_id, &nbt, access, &mut defaults)
                .map_err(|message| prototype_codec_error(*item_id, *type_id, message))?;
            if let Entry::Vacant(first) = entry {
                // These common prototype inputs compare exactly without walking nested JSON
                // or equating different floating-point bit patterns.
                let reusable = match value {
                    serde_json::Value::Number(n) => n.is_i64() || n.is_u64(),
                    serde_json::Value::String(_) => true,
                    serde_json::Value::Array(a) => a.is_empty(),
                    serde_json::Value::Object(o) => o.is_empty(),
                    _ => false,
                };
                if reusable {
                    copy_component_value(*type_id, &defaults, &mut shared_fields)
                        .expect("decoded prototype component");
                    first.insert(value);
                }
            }
        }
        if !defaults.added_type_ids.is_empty() {
            // Bootstrap defaults retain named tags until the session publishes its registries.
            if access.is_some() {
                materialize_patch_named_holder_sets(&mut defaults, access)
                    .map_err(|message| prototype_codec_error(*item_id, -1, message))?;
            }
            validate_data_component_patch_summary(&defaults)
                .map_err(|error| prototype_codec_error(*item_id, -1, error.to_string()))?;
            items.insert(*item_id, defaults);
        }
    }
    Ok(items)
}

pub(super) fn static_item_defaults(
    defaults: &rustc_hash::FxHashMap<i32, DataComponentPatchSummary>,
) -> BTreeMap<i32, DataComponentPatchSummary> {
    let mut items = BTreeMap::new();
    for (item_id, patch) in defaults {
        let mut retained = DataComponentPatchSummary::default();
        for type_id in STATIC_COMPONENT_TYPES {
            if patch.added_type_ids.contains(&type_id) {
                bbb_protocol::packets::data_components::fields::copy_component_value(
                    type_id,
                    patch,
                    &mut retained,
                )
                .expect("validated static item component fields");
                retained.added_type_ids.push(type_id);
            }
        }
        for effect in &mut retained.consumable_effects {
            if let bbb_protocol::packets::ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids,
                effect_tag: Some(_),
            } = effect
            {
                effect_registry_ids.clear();
            }
        }
        if !retained.added_type_ids.is_empty() {
            items.insert(*item_id, retained);
        }
    }
    items
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

impl WorldStore {
    pub fn replace_item_component_defaults(
        &mut self,
        codec_values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
        asset_generation: u64,
        asset_names: Vec<String>,
    ) -> Result<(), RegistryValidationError> {
        let revision = self.registries.revision;
        let access = (self.registries.pending.is_none()
            && (revision.contents != 0 || revision.tags != 0))
            .then(|| self.registries.frozen_access());
        let defaults = build_item_component_defaults(&codec_values, access)?;
        self.items.replace_component_defaults(
            codec_values,
            defaults,
            asset_generation,
            asset_names,
        );
        Ok(())
    }

    pub fn set_default_item_component_codec_values(
        &mut self,
        values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        self.items.set_default_component_codec_values(values);
    }

    pub fn rebuild_item_component_defaults(&mut self) -> Result<(), RegistryValidationError> {
        match build_item_component_defaults(
            &*self.items.default_component_codec_values,
            Some(self.registries.frozen_access()),
        ) {
            Ok(defaults) => {
                self.items.commit_item_component_defaults(defaults);
                Ok(())
            }
            Err(error) => {
                let defaults = build_item_component_defaults(
                    &*self.items.default_component_codec_values,
                    None,
                )?;
                self.items.commit_item_component_defaults(defaults);
                Err(error)
            }
        }
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
        Cow::Owned(ItemStackSummary {
            item_id: stack.item_id,
            count: stack.count,
            component_patch: component_patch.into(),
        })
    }

    pub fn item_stack_has_effective_component(
        &self,
        stack: &ItemStackSummary,
        component_id: i32,
    ) -> bool {
        self.items
            .item_has_effective_component(stack.item_id, &stack.component_patch, component_id)
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

    use bbb_client_assets::ItemRegistryCatalog;
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
    };
    use bbb_protocol::packets::{BlocksAttacksSummary, ItemDamageFunctionSummary};

    use super::*;
    use crate::{RegistryContentState, RegistryPacketEntry, RegistryTagState};

    const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
    const BANNER_PATTERN_REGISTRY: &str = "minecraft:banner_pattern";
    const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";
    const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";
    const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
    const BYPASSES_SHIELD_TAG: &str = "minecraft:bypasses_shield";

    #[test]
    fn equippable_publication_rejects_invalid_reload_without_losing_equipment() {
        use bbb_protocol::{
            data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID as EQUIPPABLE,
            equipment_slots::EquipmentSlot,
            packets::{RegistryTags, UpdateTags},
        };

        let mut world = WorldStore::new();
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let values = BTreeMap::from([(
            7,
            BTreeMap::from([(EQUIPPABLE, serde_json::json!({"slot": "head"}))]),
        )]);
        world
            .replace_item_component_defaults(values, 0, Vec::new())
            .unwrap();
        assert_eq!(world.item_stack_equipment(&stack), None);
        world.begin_registry_configuration();
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:entity_type".to_string(),
                tags: Vec::new(),
            }],
        });
        world.finish_registry_configuration().unwrap();
        assert_eq!(
            world.item_stack_equipment(&stack),
            Some((EquipmentSlot::Head, None))
        );

        let invalid = BTreeMap::from([(
            7,
            BTreeMap::from([(EQUIPPABLE, serde_json::json!({"asset_id": "example:hat"}))]),
        )]);
        assert!(world
            .replace_item_component_defaults(invalid, 1, vec!["example:hat".to_string()],)
            .is_err());
        assert_eq!(
            world.item_stack_equipment(&stack),
            Some((EquipmentSlot::Head, None))
        );
        world.reset_registry_session();
        assert_eq!(world.item_stack_equipment(&stack), None);
    }

    #[test]
    fn static_defaults_remain_usable_before_registry_publication_and_after_session_reset() {
        let mut world = WorldStore::new();
        world.set_default_item_component_codec_values(BTreeMap::from([(
            7,
            BTreeMap::from([
                (
                    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
                    serde_json::json!({"max_reach": 4.0}),
                ),
                (
                    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
                    serde_json::json!(0.5),
                ),
                (VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red")),
                (VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID, serde_json::json!(0.125)),
                (VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID, serde_json::json!({"seconds": 0.5, "cooldown_group": "example:shared"})),
                (VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, serde_json::json!(1)),
                (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, serde_json::json!(100)),
                (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, serde_json::json!(5)),
                (VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, serde_json::json!({})),
                (
                    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                    serde_json::json!({"type": "stab", "duration": 13}),
                ),
                (
                    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                    serde_json::json!({"speed_multiplier": 0.75}),
                ),
                (
                    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
                    serde_json::json!({"nutrition": 3, "saturation": 1.5}),
                ),
                (
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    serde_json::json!({
                        "consume_seconds": 0.25, "animation": "drink",
                        "on_consume_effects": [{"type": "minecraft:remove_effects", "effects": "#example:unpublished"}]
                    }),
                ),
                (
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                    serde_json::json!("example:unpublished"),
                ),
            ]),
        )]));
        assert!(world.rebuild_item_component_defaults().is_err());
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let speed = bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID;
        let bound_effects = [speed];
        for phase in 0..3 {
            let effective = world.effective_item_stack(&stack);
            let patch = &effective.component_patch;
            assert_eq!(patch.attack_range.unwrap().max_reach, 4.0);
            assert_eq!(patch.minimum_attack_charge_bits, Some(0.5_f32.to_bits()));
            assert_eq!(
                patch.dye_color_id,
                Some(bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID)
            );
            assert_eq!(patch.potion_duration_scale_bits, Some(0.125_f32.to_bits()));
            assert_eq!(patch.use_cooldown_ticks(), Some(10));
            assert_eq!(patch.use_cooldown_group.as_deref(), Some("example:shared"));
            assert_eq!(patch.max_stack_size, Some(1));
            assert_eq!(patch.max_damage, Some(100));
            assert_eq!(patch.damage, Some(5));
            assert!(patch.unbreakable);
            assert_eq!(world.item_stack_swing_animation(&stack).duration, 13);
            assert_eq!(patch.use_effects.unwrap().speed_multiplier, 0.75);
            assert_eq!(patch.food_nutrition, Some(3));
            assert_eq!(patch.food_saturation_bits, Some(1.5_f32.to_bits()));
            assert_eq!(patch.consumable.unwrap().consume_seconds, 0.25);
            assert_eq!(patch.damage_type_id.is_some(), phase == 1);
            let bbb_protocol::packets::ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids,
                effect_tag,
            } = &patch.consumable_effects[0]
            else {
                panic!("remove effects");
            };
            assert_eq!(effect_tag.as_deref(), Some("example:unpublished"));
            assert_eq!(
                effect_registry_ids.as_slice(),
                if phase == 1 { &bound_effects[..] } else { &[] }
            );
            if phase == 0 {
                insert_registry(&mut world, DAMAGE_TYPE_REGISTRY, &["example:unpublished"]);
                insert_tag(
                    &mut world,
                    "minecraft:mob_effect",
                    "example:unpublished",
                    vec![speed],
                );
                world.rebuild_item_component_defaults().unwrap();
            } else if phase == 1 {
                world.reset_registry_session();
            }
        }
    }

    #[test]
    fn tooltip_style_query_honors_component_membership_and_removal() {
        let world = WorldStore::new();
        let mut stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary {
                tooltip_style: Some("example:rare".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        };
        assert_eq!(world.item_stack_tooltip_style(&stack), None);
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
        let catalog = ItemRegistryCatalog::vanilla_26_1().unwrap();
        world.set_default_item_component_codec_values(
            [
                (
                    1,
                    "netherite_ingot",
                    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
                ),
                (
                    2,
                    "music_disc_13",
                    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                ),
                (3, "shield", VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID),
                (
                    4,
                    "flower_banner_pattern",
                    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                ),
                (5, "goat_horn", VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID),
                (
                    6,
                    "diamond",
                    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
                ),
                (
                    7,
                    "wooden_spear",
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                ),
            ]
            .into_iter()
            .map(|(id, item, kind)| {
                (
                    id,
                    BTreeMap::from([(
                        kind,
                        catalog.default_component_codec_values(item).unwrap()[&kind].clone(),
                    )]),
                )
            })
            .collect(),
        );

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
        assert_eq!(defaults[&5].instrument.registry_id, Some(1));
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID],
                jukebox_song_id: Some(42),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        };

        let effective = world.effective_item_stack(&stack);
        assert_eq!(effective.component_patch.jukebox_song_id, Some(42));
        assert!(effective.component_patch.blocks_attacks.is_some());
        assert_eq!(effective.component_patch.added_type_ids.len(), 2);
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
        world.set_default_item_component_codec_values(BTreeMap::from([(
            7,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                serde_json::json!("minecraft:spear"),
            )]),
        )]));
        world.rebuild_item_component_defaults().unwrap();

        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective = world.effective_item_stack(&stack);
        assert_eq!(effective.component_patch.damage_type_id, Some(1));
        assert_eq!(
            effective.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID]
        );

        let explicit = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
                damage_type_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            }
            .into(),
            ..stack
        };
        assert_eq!(
            world
                .effective_item_stack(&removed)
                .component_patch
                .damage_type_id,
            None
        );
    }

    #[test]
    fn effective_stack_applies_potion_duration_scale_default_with_patch_precedence() {
        let mut world = WorldStore::new();
        world.set_default_item_component_codec_values(BTreeMap::from([(
            7,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                serde_json::json!(0.125),
            )]),
        )]));
        world.rebuild_item_component_defaults().unwrap();
        let stack = ItemStackSummary {
            item_id: Some(7),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
                potion_duration_scale_bits: Some(0.5_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            }
            .into(),
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
        let codec_values = catalog.default_component_codec_values_by_protocol_id();
        let mut world = WorldStore::new();
        insert_referenced_dynamic_prototype_entries(&mut world, &codec_values);
        insert_registry(&mut world, BANNER_PATTERN_REGISTRY, &["minecraft:base"]);
        insert_referenced_prototype_tags(&mut world, &codec_values);
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
                world.items.default_component_codec_values[item_id]
                    .keys()
                    .copied()
                    .collect::<BTreeSet<_>>(),
                "item {item_id} decoded prototype membership diverged"
            );
            validate_data_component_patch_summary(patch).unwrap_or_else(|error| {
                panic!("item {item_id} prototype is not encodable: {error}")
            });
        }

        let stick_id = catalog.protocol_id("minecraft:stick").unwrap();
        let stick = ItemStackSummary {
            item_id: Some(stick_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let effective = world.effective_item_stack(&stick);
        assert_eq!(effective.component_patch.added_type_ids.len(), 12);
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID],
                max_stack_size: Some(16),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            RegistryContentState::from_entries(
                registry,
                entries.into_iter().map(RegistryPacketEntry::stub).collect(),
            ),
        );
    }

    fn insert_tag(world: &mut WorldStore, registry: &str, tag: &str, entries: Vec<i32>) {
        let entry = world
            .registries
            .tags
            .entry(registry.to_string())
            .or_insert_with(|| {
                RegistryTagState::from_materialized(registry.to_string(), BTreeMap::new())
            });
        entry.tags.insert(tag.to_string(), entries);
        *entry = RegistryTagState::from_materialized(
            registry.to_string(),
            std::mem::take(&mut entry.tags),
        );
    }
}
