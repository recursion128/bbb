//! Typed item prototype defaults and derived runtime lookup indexes.

use std::collections::{BTreeMap, BTreeSet};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as DAMAGE_COMPONENT_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID as MAX_DAMAGE_COMPONENT_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID as PIERCING_COMPONENT_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID as UNBREAKABLE_COMPONENT_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE;
use bbb_protocol::packets::data_components::fields::copy_component_value;
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemStackSummary as ProtocolItemStackSummary,
};

use crate::entities::EntityEquipmentAssetId;
use crate::{PublicationShared, RegistrySet};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_protocol::equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemProfiles {
    /// Persistent codec inputs retained by resource id/tag until a frozen
    /// registry generation can resolve every item prototype.
    #[serde(default, deserialize_with = "publication_i32_map::deserialize")]
    pub(crate) default_component_codec_values:
        PublicationShared<FxHashMap<i32, FxHashMap<i32, serde_json::Value>>>,
    /// Static prototype values are available before registry publication;
    /// holder bindings follow the committed frozen registry revision.
    #[serde(
        default,
        alias = "registry_dependent_item_defaults",
        deserialize_with = "publication_i32_map::deserialize"
    )]
    pub(crate) default_item_component_patches:
        PublicationShared<FxHashMap<i32, DataComponentPatchSummary>>,
    /// Derived acceleration index; omitted snapshots use canonical membership.
    #[serde(default)]
    default_piercing_weapon_item_ids: Option<PublicationShared<FxHashSet<i32>>>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders: PublicationShared<BTreeMap<i32, i32>>,
    #[serde(default)]
    pub(crate) default_item_crafting_remainders_known: bool,
    #[serde(default, deserialize_with = "publication_i32_map::deserialize")]
    equipment: PublicationShared<FxHashMap<i32, DefaultEquipment>>,
    #[serde(default)]
    pub(crate) equipment_asset_generation: u64,
    /// Normalized equipment asset names indexed by
    /// [`EntityEquipmentAssetId`](crate::entities::EntityEquipmentAssetId).
    #[serde(default)]
    pub(crate) equipment_asset_names: PublicationShared<Vec<String>>,
    #[serde(skip)]
    equipment_asset_index: EquipmentAssetNameIndex,
    #[serde(default)]
    pub(crate) furnace_fuel_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) furnace_fuel_rules: PublicationShared<Vec<FurnaceFuelRule>>,
    #[serde(default)]
    pub(crate) brewing_potion_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) brewing_ingredient_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) enchantment_lapis_lazuli_item_ids: PublicationShared<BTreeSet<i32>>,
    /// Exact Vanilla cartography additional-slot identities. The kind is
    /// retained because screen prediction must distinguish paper, empty maps,
    /// and glass panes without consulting a renderer-side item runtime.
    #[serde(default, with = "publication_i32_map")]
    pub(crate) cartography_additional_items:
        PublicationShared<BTreeMap<i32, CartographyAdditionalItemKind>>,
    #[serde(default)]
    pub(crate) freeze_immune_wearable_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(default)]
    pub(crate) powder_snow_walkable_foot_item_ids: PublicationShared<BTreeSet<i32>>,
    #[serde(skip)]
    content_registry: crate::registries::state::SessionContentRegistry,
}

#[derive(Debug, Clone, Default)]
struct EquipmentAssetNameIndex(PublicationShared<FxHashMap<String, u32>>);

impl PartialEq for EquipmentAssetNameIndex {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DefaultEquipment {
    slot: EquipmentSlot,
    asset: Option<EntityEquipmentAssetId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnaceFuelRule {
    AddItem(i32),
    AddTag(String),
    RemoveTag(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CartographyAdditionalItemKind {
    Paper,
    EmptyMap,
    GlassPane,
}

mod publication_i32_map {
    use std::collections::BTreeMap;

    use serde::{
        de::{DeserializeOwned, Error},
        Deserialize, Deserializer, Serialize, Serializer,
    };

    use crate::PublicationShared;

    pub(super) fn serialize<S, V>(
        values: &PublicationShared<BTreeMap<i32, V>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        serializer.collect_map(
            values
                .iter()
                .map(|(item_id, value)| (item_id.to_string(), value)),
        )
    }

    pub(super) fn deserialize<'de, D, V, M>(
        deserializer: D,
    ) -> Result<PublicationShared<M>, D::Error>
    where
        D: Deserializer<'de>,
        V: DeserializeOwned,
        M: FromIterator<(i32, V)>,
    {
        BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?
            .into_iter()
            .map(|(item_id, value)| {
                let value = serde_json::from_value(value).map_err(D::Error::custom)?;
                item_id
                    .parse::<i32>()
                    .map(|item_id| (item_id, value))
                    .map_err(D::Error::custom)
            })
            .collect::<Result<M, _>>()
            .map(Into::into)
    }
}

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
    // Canonical defaults contain only the fields owned by their declared components.
    defaults().unwrap_or(patch)
}

impl ItemProfiles {
    pub fn component_defaults(&self) -> bbb_render_types::ItemComponentDefaults {
        bbb_render_types::ItemComponentDefaults {
            default_component_codec_values: self.default_component_codec_values.clone_arc(),
            default_item_component_patches: self.default_item_component_patches.clone_arc(),
        }
    }

    pub fn component_defaults_view(&self) -> bbb_render_types::ItemComponentDefaultsView<'_> {
        bbb_render_types::ItemComponentDefaultsView {
            default_component_codec_values: &self.default_component_codec_values,
            default_item_component_patches: &self.default_item_component_patches,
        }
    }

    pub(crate) fn refresh_tool_block_tags(&mut self, registries: &RegistrySet) {
        for tool in self
            .default_item_component_patches
            .values_mut()
            .filter_map(|patch| patch.tool.as_mut())
        {
            for rule in &mut tool.rules {
                if let Some(tag) = &rule.block_tag {
                    rule.block_registry_ids.clear();
                    rule.block_registry_ids.extend_from_slice(
                        registries
                            .holder_ids_for_tag("minecraft:block", tag)
                            .unwrap_or(&[]),
                    );
                }
            }
        }
    }

    pub(crate) fn replace_component_defaults(
        &mut self,
        codecs: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
        defaults: BTreeMap<i32, DataComponentPatchSummary>,
        generation: u64,
        names: Vec<String>,
    ) {
        self.default_component_codec_values = codecs
            .into_iter()
            .map(|(id, values)| (id, values.into_iter().collect()))
            .collect::<FxHashMap<_, _>>()
            .into();
        self.equipment_asset_generation = generation;
        self.equipment_asset_index = EquipmentAssetNameIndex(
            names
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index as u32))
                .collect::<FxHashMap<_, _>>()
                .into(),
        );
        self.equipment_asset_names = names.into();
        self.commit_item_component_defaults(defaults);
    }

    fn rebuild_equipment_index(&mut self) {
        let assets = self
            .equipment_asset_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                (
                    name.as_str(),
                    EntityEquipmentAssetId::new(self.equipment_asset_generation, index as u32),
                )
            })
            .collect::<FxHashMap<_, _>>();
        self.equipment = self
            .default_item_component_patches
            .iter()
            .filter_map(|(item_id, patch)| {
                let equippable = patch.equippable.as_ref()?;
                let slot = vanilla_equipment_slot_by_id(equippable.slot_id)
                    .expect("published equippable slot")
                    .slot;
                Some((
                    *item_id,
                    DefaultEquipment {
                        slot,
                        asset: equippable
                            .asset_id
                            .as_deref()
                            .and_then(|name| assets.get(name).copied()),
                    },
                ))
            })
            .collect::<FxHashMap<_, _>>()
            .into();
    }

    pub(crate) fn item_stack_equipment(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<(EquipmentSlot, Option<EntityEquipmentAssetId>)> {
        if stack.count <= 0
            || stack
                .component_patch
                .removed_type_ids
                .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return None;
        }
        if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            let equippable = stack.component_patch.equippable.as_ref()?;
            let slot = vanilla_equipment_slot_by_id(equippable.slot_id)?.slot;
            let asset = equippable
                .asset_id
                .as_deref()
                .and_then(|name| {
                    if self.equipment_asset_index.0.len() == self.equipment_asset_names.len() {
                        self.equipment_asset_index
                            .0
                            .get(name)
                            .copied()
                            .map(|index| index as usize)
                    } else {
                        self.equipment_asset_names
                            .iter()
                            .position(|candidate| candidate == name)
                    }
                })
                .map(|index| {
                    EntityEquipmentAssetId::new(self.equipment_asset_generation, index as u32)
                });
            return Some((slot, asset));
        }
        let equipment = self.equipment.get(&stack.item_id?)?;
        Some((equipment.slot, equipment.asset))
    }

    pub fn item_stack_max_stack_size(&self, stack: &ProtocolItemStackSummary) -> i32 {
        let Some(item_id) = stack.item_id else {
            return 0;
        };
        if stack.count <= 0 {
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
        .unwrap_or_else(|| self.item_max_stack_size_for_protocol_id(item_id))
    }

    pub fn item_stack_spear_kinetic_weapon(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<crate::SpearKineticWeapon> {
        if stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
        {
            return None;
        }
        let kinetic = self
            .component_source(
                stack.item_id,
                &stack.component_patch,
                VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            )
            .kinetic_weapon
            .as_ref()?;
        Some(crate::SpearKineticWeapon {
            delay_ticks: kinetic.delay_ticks as f32,
            dismount_duration_ticks: kinetic
                .dismount_conditions
                .map_or(0, |condition| condition.max_duration_ticks)
                as f32,
            knockback_duration_ticks: kinetic
                .knockback_conditions
                .map_or(0, |condition| condition.max_duration_ticks)
                as f32,
            damage_duration_ticks: kinetic
                .damage_conditions
                .map_or(0, |condition| condition.max_duration_ticks)
                as f32,
            forward_movement: f32::from_bits(kinetic.forward_movement_bits),
        })
    }

    pub(crate) fn set_default_component_codec_values(
        &mut self,
        values: BTreeMap<i32, BTreeMap<i32, serde_json::Value>>,
    ) {
        self.default_component_codec_values = values
            .into_iter()
            .filter(|(item_id, values)| *item_id >= 0 && !values.is_empty())
            .map(|(id, values)| (id, values.into_iter().collect()))
            .collect::<FxHashMap<_, _>>()
            .into();
        self.default_item_component_patches.clear();
        self.equipment.clear();
        self.rebuild_piercing_weapon_index();
    }

    pub(crate) fn commit_item_component_defaults(
        &mut self,
        defaults: BTreeMap<i32, DataComponentPatchSummary>,
    ) {
        self.default_item_component_patches =
            defaults.into_iter().collect::<FxHashMap<_, _>>().into();
        self.rebuild_equipment_index();
        self.rebuild_piercing_weapon_index();
    }

    fn rebuild_piercing_weapon_index(&mut self) {
        let item_ids = self
            .default_component_codec_values
            .iter()
            .filter_map(|(item_id, values)| {
                values
                    .contains_key(&PIERCING_COMPONENT_ID)
                    .then_some(*item_id)
            })
            .chain(
                self.default_item_component_patches
                    .iter()
                    .filter_map(|(item_id, patch)| {
                        patch
                            .added_type_ids
                            .contains(&PIERCING_COMPONENT_ID)
                            .then_some(*item_id)
                    }),
            )
            .collect::<FxHashSet<_>>();
        self.default_piercing_weapon_item_ids = Some(item_ids.into());
    }

    #[inline(always)]
    pub(crate) fn item_has_effective_component(
        &self,
        item_id: Option<i32>,
        patch: &DataComponentPatchSummary,
        component_id: i32,
    ) -> bool {
        if patch.removed_type_ids.contains(&component_id) {
            return false;
        }
        if patch.added_type_ids.contains(&component_id) {
            return true;
        }
        if component_id == PIERCING_COMPONENT_ID {
            if let Some(index) = &self.default_piercing_weapon_item_ids {
                return item_id.is_some_and(|item_id| index.contains(&item_id));
            }
        }
        item_id.is_some_and(|item_id| {
            self.default_component_codec_values
                .get(&item_id)
                .is_some_and(|defaults| defaults.contains_key(&component_id))
                || self
                    .default_item_component_patches
                    .get(&item_id)
                    .is_some_and(|defaults| defaults.added_type_ids.contains(&component_id))
        })
    }

    /// Borrows the patch supplying a prototype-backed component.
    pub fn component_source<'a>(
        &'a self,
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

    pub(crate) fn effective_component_patch(
        &self,
        item_id: i32,
        patch: &DataComponentPatchSummary,
    ) -> Option<DataComponentPatchSummary> {
        let defaults = self.default_item_component_patches.get(&item_id)?;
        let mut effective: Option<DataComponentPatchSummary> = None;
        for component_id in &defaults.added_type_ids {
            if patch.removed_type_ids.contains(component_id)
                || patch.added_type_ids.contains(component_id)
            {
                continue;
            }
            let effective = effective.get_or_insert_with(|| patch.clone());
            copy_component_value(*component_id, defaults, effective)
                .expect("validated item prototype must have known component fields");
            effective.added_type_ids.push(*component_id);
        }
        effective
    }

    pub(crate) fn set_default_item_crafting_remainders(&mut self, remainders: BTreeMap<i32, i32>) {
        self.default_item_crafting_remainders_known = true;
        self.default_item_crafting_remainders = remainders
            .into_iter()
            .filter(|(item_id, remainder_id)| *item_id >= 0 && *remainder_id >= 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_content_registry(
        &mut self,
        registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
    ) {
        self.content_registry = crate::registries::state::SessionContentRegistry(registry);
    }

    pub fn item_max_stack_size_for_protocol_id(&self, item_id: i32) -> i32 {
        if let Some(max_stack) = self
            .default_item_component_patches
            .get(&item_id)
            .and_then(|patch| patch.max_stack_size)
        {
            return max_stack;
        }
        if !self.content_registry.0.is_vanilla_only() {
            if let Some(spec) = self.content_registry.0.item_by_id(item_id) {
                return i32::from(spec.max_stack_size.clamp(1, 99));
            }
        }
        VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE
    }

    pub fn item_max_damage_for_protocol_id(&self, item_id: i32) -> Option<i32> {
        self.default_item_component_patches
            .get(&item_id)
            .and_then(|patch| patch.max_damage)
    }

    pub fn damage_state(
        profiles: Option<&Self>,
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
        let defaults = profiles.and_then(|profiles| {
            item_id.and_then(|item_id| profiles.default_item_component_patches.get(&item_id))
        });
        let max_damage = component_source(|| defaults, patch, MAX_DAMAGE_COMPONENT_ID).max_damage?;
        let damage = component_source(|| defaults, patch, DAMAGE_COMPONENT_ID).damage;
        let default_values = profiles.and_then(|profiles| {
            item_id.and_then(|item_id| profiles.default_component_codec_values.get(&item_id))
        });
        let has_component = |component_id| {
            !patch.removed_type_ids.contains(&component_id)
                && (patch.added_type_ids.contains(&component_id)
                    || defaults
                        .is_some_and(|defaults| defaults.added_type_ids.contains(&component_id))
                    || default_values.is_some_and(|defaults| defaults.contains_key(&component_id)))
        };
        if has_component(UNBREAKABLE_COMPONENT_ID)
            || (damage.is_none() && !has_component(DAMAGE_COMPONENT_ID))
        {
            return None;
        }
        Some((damage.unwrap_or(0).clamp(0, max_damage.max(0)), max_damage))
    }

    pub(crate) fn set_furnace_fuel_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.furnace_fuel_item_ids = filter_item_ids(item_ids).into();
        self.furnace_fuel_rules.clear();
    }

    pub(crate) fn set_furnace_fuel_profile(
        &mut self,
        item_ids: BTreeSet<i32>,
        rules: Vec<FurnaceFuelRule>,
    ) {
        self.furnace_fuel_item_ids = filter_item_ids(item_ids).into();
        self.furnace_fuel_rules = rules
            .into_iter()
            .filter(|rule| !matches!(rule, FurnaceFuelRule::AddItem(item_id) if *item_id < 0))
            .collect::<Vec<_>>()
            .into();
    }

    pub(crate) fn refresh_furnace_fuel_item_ids(&mut self, registries: &crate::RegistrySet) {
        if self.furnace_fuel_rules.is_empty() {
            return;
        }

        let mut item_ids = BTreeSet::new();
        for rule in self.furnace_fuel_rules.iter() {
            match rule {
                FurnaceFuelRule::AddItem(item_id) => {
                    item_ids.insert(*item_id);
                }
                FurnaceFuelRule::AddTag(tag) => {
                    if let Some(tag_item_ids) = registries.holder_ids_for_tag("minecraft:item", tag)
                    {
                        item_ids
                            .extend(tag_item_ids.iter().copied().filter(|item_id| *item_id >= 0));
                    }
                }
                FurnaceFuelRule::RemoveTag(tag) => {
                    if let Some(tag_item_ids) = registries.holder_ids_for_tag("minecraft:item", tag)
                    {
                        for item_id in tag_item_ids {
                            item_ids.remove(item_id);
                        }
                    }
                }
            }
        }
        self.furnace_fuel_item_ids = item_ids.into();
    }

    pub(crate) fn set_brewing_potion_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_potion_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_brewing_ingredient_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.brewing_ingredient_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_enchantment_lapis_lazuli_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.enchantment_lapis_lazuli_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_cartography_additional_items(
        &mut self,
        items: BTreeMap<i32, CartographyAdditionalItemKind>,
    ) {
        self.cartography_additional_items = items
            .into_iter()
            .filter(|(item_id, _)| *item_id >= 0)
            .collect::<BTreeMap<_, _>>()
            .into();
    }

    pub(crate) fn set_freeze_immune_wearable_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.freeze_immune_wearable_item_ids = filter_item_ids(item_ids).into();
    }

    pub(crate) fn set_powder_snow_walkable_foot_item_ids(&mut self, item_ids: BTreeSet<i32>) {
        self.powder_snow_walkable_foot_item_ids = filter_item_ids(item_ids).into();
    }
}

fn filter_item_ids(item_ids: BTreeSet<i32>) -> BTreeSet<i32> {
    item_ids
        .into_iter()
        .filter(|item_id| *item_id >= 0)
        .collect()
}

#[cfg(test)]
pub(crate) fn test_equippable(slot: EquipmentSlot) -> bbb_protocol::packets::EquippableSummary {
    let sound = bbb_protocol::packets::SoundEventSummary {
        registry_id: Some(0),
        sound_id: None,
        fixed_range_bits: None,
    };
    bbb_protocol::packets::EquippableSummary {
        slot_id: bbb_protocol::equipment_slots::vanilla_equipment_slot_by_ordinal(slot.ordinal())
            .unwrap()
            .id,
        equip_sound: sound.clone(),
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: sound,
    }
}

#[cfg(test)]
impl crate::WorldStore {
    pub(crate) fn set_test_equippable_defaults(
        &mut self,
        entries: impl IntoIterator<Item = (i32, bbb_protocol::packets::EquippableSummary)>,
    ) {
        let mut defaults = self
            .items
            .default_item_component_patches
            .iter()
            .map(|(id, patch)| (*id, patch.clone()))
            .collect::<BTreeMap<_, _>>();
        for (item_id, equippable) in entries {
            let patch = defaults.entry(item_id).or_default();
            if !patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
            {
                patch
                    .added_type_ids
                    .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
            }
            patch.equippable = Some(equippable);
        }
        self.items.commit_item_component_defaults(defaults);
    }
}
