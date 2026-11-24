use std::{borrow::Cow, collections::BTreeMap, sync::OnceLock};

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID;
use bbb_protocol::packets::{DataComponentPatchSummary, ItemEnchantmentSummary, ItemStackSummary};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::{ItemStackPersistentProjectionError, PersistentItemStackHolderResolver};

/// A live item stack. The empty stack is always `ItemStackSummary::empty()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemStack(ItemStackSummary);

impl Default for RuntimeItemStack {
    fn default() -> Self {
        Self::empty()
    }
}

impl RuntimeItemStack {
    pub fn empty() -> Self {
        Self(ItemStackSummary::empty())
    }

    pub fn from_summary(summary: ItemStackSummary) -> Self {
        if bbb_registry_codec::item_stack_summary_is_empty(&summary) {
            return Self::empty();
        }
        Self(summary)
    }

    /// `ItemStack.OPTIONAL_CODEC` read: an unreadable stack loads as empty.
    pub fn read(value: &NbtSummaryValue, resolver: &dyn PersistentItemStackHolderResolver) -> Self {
        Self::try_read(value, resolver).unwrap_or_default()
    }

    pub fn try_read(
        value: &NbtSummaryValue,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<Self, ItemStackPersistentProjectionError> {
        bbb_registry_codec::read_item_stack(value, resolver).map(Self::from_summary)
    }

    /// `ItemStack.CODEC` write; `None` for the empty stack.
    pub fn write(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<Option<NbtSummaryValue>, ItemStackPersistentProjectionError> {
        if self.is_empty() {
            return Ok(None);
        }
        bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver(&self.0, resolver)
            .map(Some)
    }

    /// `ItemStack.OPTIONAL_CODEC` write: the empty stack is `{}`.
    pub fn write_optional(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<NbtSummaryValue, ItemStackPersistentProjectionError> {
        self.write(resolver)
            .map(|value| value.unwrap_or(NbtSummaryValue::Compound(Vec::new())))
    }

    pub fn summary(&self) -> &ItemStackSummary {
        &self.0
    }

    pub fn into_summary(self) -> ItemStackSummary {
        self.0
    }

    /// Mutable access for component edits; an edit that empties the stack
    /// must go through [`Self::set_count`].
    pub fn summary_mut(&mut self) -> &mut ItemStackSummary {
        &mut self.0
    }

    pub fn item_id(&self) -> Option<i32> {
        self.0.item_id
    }

    pub fn item_resource_id(&self) -> Option<&'static str> {
        self.item_id()
            .and_then(bbb_protocol::item_types::vanilla_item_type)
            .map(|item| item.resource_id)
    }

    pub fn item_resource_id_with_registry<'a>(
        &self,
        registry: &'a bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<&'a str> {
        let item_id = self.item_id()?;
        if let Some(item) = bbb_protocol::item_types::vanilla_item_type(item_id) {
            return Some(item.resource_id);
        }
        registry
            .item_by_id(item_id)
            .map(bbb_protocol::content_registry::ModItemSpec::resource_id)
    }

    pub fn count(&self) -> i32 {
        self.0.count
    }

    pub fn is_empty(&self) -> bool {
        bbb_registry_codec::item_stack_summary_is_empty(&self.0)
    }

    pub fn with_count(&self, count: i32) -> Self {
        if count <= 0 || self.is_empty() {
            return Self::empty();
        }
        let mut summary = self.0.clone();
        summary.count = count;
        Self(summary)
    }

    pub fn set_count(&mut self, count: i32) {
        if count <= 0 {
            *self = Self::empty();
        } else if !self.is_empty() {
            self.0.count = count;
        }
    }

    pub fn grow(&mut self, amount: i32) {
        self.set_count(self.count() + amount);
    }

    pub fn shrink(&mut self, amount: i32) {
        self.set_count(self.count() - amount);
    }

    pub fn split(&mut self, amount: i32) -> Self {
        let taken = amount.min(self.count()).max(0);
        let result = self.with_count(taken);
        self.shrink(taken);
        result
    }

    pub fn same_item_same_components(&self, other: &Self) -> bool {
        // ItemStack.java:639-643: empty stacks are AIR.
        if self.is_empty() || other.is_empty() {
            return self.is_empty() && other.is_empty();
        }
        super::rules::is_same_item_same_components(&self.0, &other.0)
    }

    /// Component equality against the stack's actual registry publication,
    /// including prototype defaults and excluding unused patch fields.
    pub fn same_item_same_effective_components(
        &self,
        other: &Self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<bool> {
        if self.same_item_same_components(other) {
            return Ok(true);
        }
        if self.item_id() != other.item_id() || self.is_empty() || other.is_empty() {
            return Ok(false);
        }
        let left = &self.0.component_patch;
        let right = &other.0.component_patch;
        let changed = left
            .added_type_ids
            .iter()
            .chain(&left.removed_type_ids)
            .chain(&right.added_type_ids)
            .chain(&right.removed_type_ids)
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for type_id in changed {
            match (
                self.effective_component(type_id, resolver)?,
                other.effective_component(type_id, resolver)?,
            ) {
                (None, None) => {}
                (Some(left), Some(right)) => {
                    if !bbb_protocol::packets::data_components::fields::component_values_equal(
                        type_id, &left, &right,
                    )
                    .map_err(component_error)?
                    {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.count() == other.count() && self.same_item_same_components(other)
    }

    pub fn max_stack_size(&self) -> crate::Result<i32> {
        if self.is_empty() {
            return Ok(1);
        }
        super::rules::max_stack_size(&self.0)
    }

    pub fn map_id(&self) -> Option<i32> {
        self.0.component_patch.map_id
    }

    pub fn equippable_slot(
        &self,
    ) -> crate::Result<Option<bbb_protocol::equipment_slots::EquipmentSlot>> {
        let Some(patch) =
            static_item_component(&self.0, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)?
        else {
            return Ok(None);
        };
        patch
            .equippable
            .as_ref()
            .and_then(|equippable| {
                bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
            })
            .map(|entry| Some(entry.slot))
            .ok_or_else(|| {
                crate::ServerWorldError::InvalidConfiguration(
                    "patched equippable has no readable slot".into(),
                )
            })
    }

    pub fn has_component(&self, type_id: i32) -> crate::Result<bool> {
        if self.is_empty() {
            return Ok(false);
        }
        super::rules::has_component(&self.0, type_id)
    }

    pub fn additional_trade_cost(&self) -> Option<i32> {
        if self.is_empty() {
            return None;
        }
        self.0.component_patch.additional_trade_cost
    }

    // VillagerTrade.java:177-180 removes this live-only component before publishing the offer.
    pub fn with_additional_trade_cost(mut self, cost: Option<i32>) -> Self {
        take_trade_cost(&mut self.0);
        if let Some(cost) = cost {
            self.0.component_patch.additional_trade_cost = Some(cost);
            super::rules::add_patched_component(
                &mut self.0,
                VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
            );
        }
        self
    }

    /// The effective persistent value of one component (patch, else prototype).
    pub fn component_nbt(
        &self,
        type_id: i32,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Option<NbtSummaryValue>> {
        bbb_registry_codec::item_component_nbt(&self.0, type_id, resolver).map_err(component_error)
    }

    /// `ItemStack.set` / `ItemStack.remove` with a persistent component value.
    pub fn with_component_nbt(
        mut self,
        type_id: i32,
        value: Option<&NbtSummaryValue>,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Self> {
        if self.is_empty() {
            return Ok(Self::empty());
        }
        if type_id == VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID {
            let cost = match value {
                Some(NbtSummaryValue::Int(cost)) => Some(*cost),
                None => None,
                Some(_) => return Err(component_error("additional trade cost must be an integer")),
            };
            return Ok(self.with_additional_trade_cost(cost));
        }
        match value {
            Some(value) => {
                let patch = bbb_registry_codec::read_item_component(type_id, value, resolver)
                    .map_err(component_error)?;
                self.set_component_from(type_id, &patch, resolver)?;
            }
            None => self.remove_component(type_id)?,
        }
        Ok(self)
    }

    /// `PatchedDataComponentMap.applyPatch` (`PatchedDataComponentMap.java:139-148`).
    pub fn with_component_patch(
        mut self,
        patch: &DataComponentPatchSummary,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Self> {
        if self.is_empty() {
            return Ok(Self::empty());
        }
        for &type_id in &patch.added_type_ids {
            self.set_component_from(type_id, patch, resolver)?;
        }
        for &type_id in &patch.removed_type_ids {
            self.remove_component(type_id)?;
        }
        Ok(self)
    }

    fn enchantment_component_type(&self) -> i32 {
        if self.item_id() == Some(VANILLA_ITEM_ENCHANTED_BOOK_ID) {
            VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
        } else {
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
        }
    }

    pub fn enchantments(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Option<Cow<'_, [ItemEnchantmentSummary]>>> {
        let type_id = self.enchantment_component_type();
        let stored = type_id == VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID;
        Ok(self
            .effective_component(type_id, resolver)?
            .map(|patch| match patch {
                Cow::Borrowed(patch) => Cow::Borrowed(if stored {
                    patch.stored_enchantments.as_slice()
                } else {
                    patch.enchantments.as_slice()
                }),
                Cow::Owned(patch) => Cow::Owned(if stored {
                    patch.stored_enchantments
                } else {
                    patch.enchantments
                }),
            }))
    }

    pub fn with_enchantments(
        self,
        mut values: Vec<ItemEnchantmentSummary>,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Self> {
        let type_id = self.enchantment_component_type();
        let mut patch = DataComponentPatchSummary {
            added_type_ids: vec![type_id],
            ..Default::default()
        };
        values.sort_unstable_by_key(|entry| entry.holder_id);
        if type_id == VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID {
            patch.stored_enchantments = values;
        } else {
            patch.enchantments = values;
        }
        self.with_component_patch(&patch, resolver)
    }

    /// Copies one component value out of `source`; a value equal to the
    /// prototype's leaves no patch entry (`PatchedDataComponentMap.java:150-160`).
    fn set_component_from(
        &mut self,
        type_id: i32,
        source: &DataComponentPatchSummary,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<()> {
        use bbb_protocol::packets::data_components::fields::{
            component_values_equal, copy_component_value,
        };
        if type_id == VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID {
            take_trade_cost(&mut self.0);
            self.0.component_patch.additional_trade_cost = source.additional_trade_cost;
            super::rules::add_patched_component(&mut self.0, type_id);
            return Ok(());
        }
        let matches_prototype = match self.prototype_component(type_id, resolver)? {
            Some(prototype) => {
                component_values_equal(type_id, &prototype, source).map_err(component_error)?
            }
            None => false,
        };
        if matches_prototype {
            bbb_protocol::packets::data_components::fields::clear_component_value(
                type_id,
                &mut self.0.component_patch,
            )
            .map_err(component_error)?;
        } else {
            copy_component_value(type_id, source, &mut self.0.component_patch)
                .map_err(component_error)?;
        }
        let patch = &mut self.0.component_patch;
        patch.removed_type_ids.retain(|id| *id != type_id);
        if matches_prototype {
            super::rules::remove_patched_component(&mut self.0, type_id);
        } else {
            super::rules::add_patched_component(&mut self.0, type_id);
            self.0.component_patch.added_type_ids.sort_unstable();
        }
        Ok(())
    }

    /// One component's effective value as a patch carrying it: the stack's
    /// own patch when it mentions the type, else the prototype's.
    pub fn effective_component(
        &self,
        type_id: i32,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Option<Cow<'_, DataComponentPatchSummary>>> {
        match component_source(&self.0, type_id) {
            None => Ok(None),
            Some(ComponentSource::Patched(patch)) => Ok(Some(Cow::Borrowed(patch))),
            Some(ComponentSource::Prototype(_)) => self.prototype_component(type_id, resolver),
        }
    }

    fn prototype_component(
        &self,
        type_id: i32,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Option<Cow<'_, DataComponentPatchSummary>>> {
        let Some(item_id) = self.0.item_id else {
            return Ok(None);
        };
        if STATIC_COMPONENT_TYPES.contains(&type_id) {
            return static_component_prototype(item_id, type_id)
                .map(|value| value.map(Cow::Borrowed));
        }
        let Some(value) = super::rules::prototype_components(item_id)?
            .and_then(|components| components.get(&type_id))
        else {
            return Ok(None);
        };
        Ok(decode_prototype(type_id, value, resolver)
            .ok()
            .map(Cow::Owned))
    }

    // PatchedDataComponentMap.java:86-97: a prototype component is masked, a
    // patch-only one disappears.
    fn remove_component(&mut self, type_id: i32) -> crate::Result<()> {
        if type_id == VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID {
            take_trade_cost(&mut self.0);
            return Ok(());
        }
        bbb_protocol::packets::data_components::fields::clear_component_value(
            type_id,
            &mut self.0.component_patch,
        )
        .map_err(component_error)?;
        super::rules::remove_patched_component(&mut self.0, type_id);
        let in_prototype = match self.0.item_id {
            Some(item_id) => super::rules::prototype_components(item_id)?
                .is_some_and(|components| components.contains_key(&type_id)),
            None => false,
        };
        let patch = &mut self.0.component_patch;
        patch.removed_type_ids.retain(|id| *id != type_id);
        if in_prototype {
            patch.removed_type_ids.push(type_id);
            patch.removed_type_ids.sort_unstable();
        }
        Ok(())
    }

    /// `ItemContainerContents` of `CONTAINER`, one entry per slot up to the
    /// highest occupied one; `None` when the stack has no such component.
    pub fn container_contents(&self) -> Option<Vec<RuntimeItemStack>> {
        let patch = &self.0.component_patch;
        let type_id = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID;
        if !self.has_component(type_id).unwrap_or(false) {
            return None;
        }
        let mut items = vec![Self::empty(); patch.container.slot_count.unwrap_or(0)];
        for slot in &patch.container.slots {
            if slot.slot >= items.len() {
                items.resize(slot.slot + 1, Self::empty());
            }
            items[slot.slot] = Self::from_template(&slot.item);
        }
        Some(items)
    }

    /// `components.set(CONTAINER, ItemContainerContents.fromItems(items))`.
    pub fn with_container_contents(
        &self,
        items: &[RuntimeItemStack],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> crate::Result<Self> {
        self.clone()
            .with_component_patch(&container_patch(items), resolver)
    }

    pub fn from_template(template: &bbb_protocol::packets::ItemStackTemplateSummary) -> Self {
        Self::from_summary(ItemStackSummary {
            item_id: Some(template.item_id),
            count: template.count,
            component_patch: template.component_patch.clone(),
        })
    }

    pub fn to_template(&self) -> Option<bbb_protocol::packets::ItemStackTemplateSummary> {
        (!self.is_empty()).then(|| bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: self.0.item_id.expect("non-empty stack has an item"),
            count: self.0.count,
            component_patch: self.0.component_patch.clone(),
        })
    }

    pub fn item_identity_summary(&self) -> ItemStackSummary {
        if self.is_empty() {
            return ItemStackSummary::empty();
        }
        ItemStackSummary {
            item_id: self.item_id(),
            count: self.count(),
            component_patch: Default::default(),
        }
    }
}

/// A single-component patch holding `ItemContainerContents.fromItems(items)`
/// the way the reader projects it.
pub fn container_patch(items: &[RuntimeItemStack]) -> DataComponentPatchSummary {
    let type_id = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID;
    let mut patch = DataComponentPatchSummary {
        added_type_ids: vec![type_id],
        ..Default::default()
    };
    let mut slot_count = 0;
    for (slot, item) in items.iter().enumerate() {
        let Some(template) = item.to_template() else {
            continue;
        };
        slot_count = slot + 1;
        patch
            .container
            .slots
            .push(bbb_protocol::packets::ItemContainerSlotSummary {
                slot,
                item: template,
            });
    }
    patch.container.slot_count = Some(slot_count);
    patch
}

fn component_error(error: impl std::fmt::Display) -> crate::ServerWorldError {
    crate::ServerWorldError::InvalidConfiguration(format!("item component: {error}"))
}

pub fn take_trade_cost(summary: &mut ItemStackSummary) -> Option<i32> {
    let id = VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID;
    let cost = summary.component_patch.additional_trade_cost.take();
    summary.component_patch.codec_backed_nbt.remove(&id);
    summary
        .component_patch
        .removed_type_ids
        .retain(|value| *value != id);
    super::rules::remove_patched_component(summary, id);
    cost
}

impl From<ItemStackSummary> for RuntimeItemStack {
    fn from(value: ItemStackSummary) -> Self {
        Self::from_summary(value)
    }
}

enum ComponentSource<'a> {
    Patched(&'a DataComponentPatchSummary),
    Prototype(i32),
}

#[inline]
fn component_source(item: &ItemStackSummary, type_id: i32) -> Option<ComponentSource<'_>> {
    let patch = &item.component_patch;
    if bbb_registry_codec::item_stack_summary_is_empty(item)
        || patch.removed_type_ids.contains(&type_id)
    {
        return None;
    }
    if patch.added_type_ids.contains(&type_id) {
        return Some(ComponentSource::Patched(patch));
    }
    Some(ComponentSource::Prototype(item.item_id.unwrap()))
}

/// Borrows components whose built-in defaults need no dynamic holder IDs.
#[inline]
pub fn static_item_component(
    item: &ItemStackSummary,
    type_id: i32,
) -> crate::Result<Option<&DataComponentPatchSummary>> {
    match component_source(item, type_id) {
        None => Ok(None),
        Some(ComponentSource::Patched(patch)) => Ok(Some(patch)),
        Some(ComponentSource::Prototype(item_id)) => static_component_prototype(item_id, type_id),
    }
}

// Tags remain named; these prototypes need no dynamic holder IDs.
const STATIC_COMPONENT_TYPES: [i32; 19] = [
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
];

struct StaticComponentPrototype {
    codec: &'static serde_json::Value,
    typed: OnceLock<Result<Box<DataComponentPatchSummary>, ItemStackPersistentProjectionError>>,
}

#[inline]
fn static_component_prototype(
    item_id: i32,
    type_id: i32,
) -> crate::Result<Option<&'static DataComponentPatchSummary>> {
    static PROTOTYPES: [OnceLock<crate::Result<BTreeMap<i32, StaticComponentPrototype>>>;
        bbb_protocol::item_types::VANILLA_ITEM_TYPES.len()] =
        [const { OnceLock::new() }; bbb_protocol::item_types::VANILLA_ITEM_TYPES.len()];
    let Some(prototypes) = PROTOTYPES.get(item_id as usize) else {
        return Ok(None);
    };
    let prototypes = prototypes
        .get_or_init(|| {
            let values = super::rules::prototype_components(item_id)?;
            Ok(STATIC_COMPONENT_TYPES
                .iter()
                .filter_map(|&type_id| {
                    Some((
                        type_id,
                        StaticComponentPrototype {
                            codec: values?.get(&type_id)?,
                            typed: OnceLock::new(),
                        },
                    ))
                })
                .collect())
        })
        .as_ref()
        .map_err(component_error)?;
    let Some(prototype) = prototypes.get(&type_id) else {
        return Ok(None);
    };
    prototype
        .typed
        .get_or_init(|| {
            decode_prototype(
                type_id,
                prototype.codec,
                &bbb_registry_codec::UnavailableHolderResolver,
            )
            .map(Box::new)
        })
        .as_ref()
        .map(|patch| Some(patch.as_ref()))
        .map_err(component_error)
}

fn decode_prototype(
    type_id: i32,
    value: &serde_json::Value,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<DataComponentPatchSummary, ItemStackPersistentProjectionError> {
    let value = bbb_protocol::json_value_to_nbt_summary(value).map_err(|error| {
        ItemStackPersistentProjectionError::InvalidWireProjection {
            detail: error.to_string(),
        }
    })?;
    bbb_registry_codec::read_item_component(type_id, &value, resolver)
}
