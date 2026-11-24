//! `FuelValues` as the server builds it (`FuelValues.java:19-151`): the ordered
//! item -> burn duration table `vanillaBurnTimes` (`:42-110`) fills from the
//! loaded item tags, and the divisor each furnace block entity applies to it.
//! The fuel slot, the lit timer and the burn tick stay outside.

use std::collections::BTreeMap;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::ItemStackSummary;
pub use bbb_server_types::fuel_burn_times::{
    BurnTime, FuelRow, VANILLA_BURN_TIME_ROWS, VANILLA_FUEL_BASE_UNIT,
};

use super::ingredient::{stack_is_empty, ItemTagIndex};

/// A `vanillaBurnTimes` row this build cannot turn into a burn duration, named
/// rather than silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuelRefusal {
    pub resource_id: &'static str,
    pub reason: FuelRefusalReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelRefusalReason {
    /// The item registry this build carries answers no such id, so `asItem`
    /// has no key to put under.
    UnknownItem,
}

impl FuelRefusalReason {
    pub fn reason(self) -> &'static str {
        match self {
            Self::UnknownItem => "the item registry answers no such id",
        }
    }
}

/// The furnace block entities that burn fuel. `CampfireBlockEntity` has no fuel
/// slot and never asks `FuelValues` (`CampfireBlockEntity.java:53-83`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceFuelKind {
    Furnace,
    BlastFurnace,
    Smoker,
}

impl FurnaceFuelKind {
    /// `AbstractFurnaceBlockEntity.getBurnDuration`
    /// (`AbstractFurnaceBlockEntity.java:247-249`), halved by both
    /// `BlastFurnaceBlockEntity` (`BlastFurnaceBlockEntity.java:25-27`) and
    /// `SmokerBlockEntity` (`SmokerBlockEntity.java:25-27`).
    fn divisor(self) -> i32 {
        match self {
            Self::Furnace => 1,
            Self::BlastFurnace | Self::Smoker => 2,
        }
    }
}

/// `FuelValues` (`FuelValues.java:19-36`) over its
/// `Object2IntLinkedOpenHashMap` (`:115`): a re-put keeps a key's first
/// position and only replaces its value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FuelValues {
    values: Vec<(i32, i32)>,
    positions: BTreeMap<i32, usize>,
}

impl FuelValues {
    /// `FuelValues.vanillaBurnTimes` (`FuelValues.java:38-40`).
    pub fn vanilla_burn_times(tags: &ItemTagIndex) -> FuelTable {
        Self::vanilla_burn_times_with_content_registry(tags, ContentRegistry::vanilla())
    }

    /// Content-registry-aware form of [`Self::vanilla_burn_times`].
    pub fn vanilla_burn_times_with_content_registry(
        tags: &ItemTagIndex,
        content_registry: &ContentRegistry,
    ) -> FuelTable {
        Self::build_with_content_registry(
            VANILLA_BURN_TIME_ROWS,
            VANILLA_FUEL_BASE_UNIT,
            tags,
            content_registry,
        )
    }

    /// `FuelValues.Builder` (`FuelValues.java:112-151`) over an arbitrary row
    /// list and `baseUnit` (`:42`).
    pub fn build(rows: &[FuelRow], base_unit: i32, tags: &ItemTagIndex) -> FuelTable {
        Self::build_with_content_registry(rows, base_unit, tags, ContentRegistry::vanilla())
    }

    /// Content-registry-aware form of [`Self::build`].
    pub fn build_with_content_registry(
        rows: &[FuelRow],
        base_unit: i32,
        tags: &ItemTagIndex,
        content_registry: &ContentRegistry,
    ) -> FuelTable {
        let mut values = Self::default();
        let mut refusals = Vec::new();
        for row in rows {
            match row {
                FuelRow::Item(resource_id, time) => match item_id(resource_id, content_registry) {
                    Some(item) => values.put(item, time.ticks(base_unit)),
                    None => refusals.push(FuelRefusal {
                        resource_id,
                        reason: FuelRefusalReason::UnknownItem,
                    }),
                },
                FuelRow::Tag(tag, time) => {
                    let ticks = time.ticks(base_unit);
                    for item in tags.members(tag) {
                        values.put(item, ticks);
                    }
                }
                FuelRow::RemoveTag(tag) => values.remove_tag(tag, tags),
            }
        }
        FuelTable { values, refusals }
    }

    /// `FuelValues.isFuel` (`FuelValues.java:26-28`): an empty stack reads as
    /// air, which is never a key.
    pub fn is_fuel(&self, stack: &ItemStackSummary) -> bool {
        self.position(stack).is_some()
    }

    /// `FuelValues.burnDuration` (`FuelValues.java:34-36`); an item the map has
    /// no entry for reads as the map's zero default.
    pub fn burn_duration(&self, stack: &ItemStackSummary) -> i32 {
        match self.position(stack) {
            Some(index) => self.values[index].1,
            None => 0,
        }
    }

    /// The duration one furnace block entity gets out of that table.
    pub fn burn_duration_for(&self, kind: FurnaceFuelKind, stack: &ItemStackSummary) -> i32 {
        self.burn_duration(stack) / kind.divisor()
    }

    /// `FuelValues.fuelItems` (`FuelValues.java:30-32`), in insertion order.
    pub fn fuel_items(&self) -> impl Iterator<Item = i32> + '_ {
        self.values.iter().map(|(item, _)| *item)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn position(&self, stack: &ItemStackSummary) -> Option<usize> {
        if stack_is_empty(stack) {
            return None;
        }
        self.positions.get(&stack.item_id?).copied()
    }

    fn put(&mut self, item_id: i32, ticks: i32) {
        match self.positions.get(&item_id) {
            Some(index) => self.values[*index].1 = ticks,
            None => {
                self.positions.insert(item_id, self.values.len());
                self.values.push((item_id, ticks));
            }
        }
    }

    fn remove_tag(&mut self, tag: &str, tags: &ItemTagIndex) {
        self.values.retain(|(item, _)| !tags.contains(tag, *item));
        self.positions = self
            .values
            .iter()
            .enumerate()
            .map(|(index, (item, _))| (*item, index))
            .collect();
    }
}

/// One built fuel table with the rows it could not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuelTable {
    pub values: FuelValues,
    pub refusals: Vec<FuelRefusal>,
}

fn item_id(resource_id: &str, content_registry: &ContentRegistry) -> Option<i32> {
    vanilla_item_type_for_resource_id(resource_id)
        .map(|entry| entry.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(resource_id)
                .map(|spec| spec.id)
        })
}
