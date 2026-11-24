//! `GrindstoneMenu`: disenchanting and durability merging.

use crate::world::maps::ServerMapStore;
use bbb_protocol::item_types::{VANILLA_ITEM_BOOK_ID, VANILLA_ITEM_ENCHANTED_BOOK_ID};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

use super::item_edits::{
    crafting_enchantments, damage_value, has_any_enchantments, increased_repair_cost,
    is_damageable, max_damage, transmute, upgrade, with_crafting_enchantments, with_damage,
    with_max_damage, with_repair_cost,
};
use super::{PlayerInventoryMenu, UtilityMenuEvent, UtilityMenuFacts};

const INPUT: usize = 0;
const ADDITIONAL: usize = 1;
const RESULT: usize = 2;

/// Both input slots' `mayPlace` (`GrindstoneMenu.java:48-59`).
pub(super) fn may_place(item: &RuntimeItemStack) -> Result<bool> {
    Ok(is_damageable(item)? || has_any_enchantments(item))
}

impl PlayerInventoryMenu {
    /// `GrindstoneMenu.createResult` (`GrindstoneMenu.java:117-120`).
    pub(super) fn grindstone_create_result(&mut self) -> Result<()> {
        let input = self.container[INPUT].clone();
        let additional = self.container[ADDITIONAL].clone();
        let result = match self.utility().facts.clone() {
            Some(facts) => compute_result(&facts, &input, &additional)?,
            None => RuntimeItemStack::empty(),
        };
        self.container[RESULT] = result;
        Ok(())
    }

    /// The result slot's `onTake` (`GrindstoneMenu.java:67-77`).
    pub(super) fn grindstone_on_take(&mut self) -> Result<()> {
        let experience = match self.utility().facts.clone() {
            Some(facts) => {
                experience_from(&facts, &self.container[INPUT])
                    + experience_from(&facts, &self.container[ADDITIONAL])
            }
            None => 0,
        };
        self.container[INPUT] = RuntimeItemStack::empty();
        self.container[ADDITIONAL] = RuntimeItemStack::empty();
        self.grindstone_create_result()?;
        self.utility_mut()
            .events
            .push(UtilityMenuEvent::GrindstoneUsed { experience });
        Ok(())
    }

    /// `GrindstoneMenu.quickMoveStack` (`GrindstoneMenu.java:212-256`).
    pub(super) fn grindstone_quick_move_stack(
        &mut self,
        resolver: &dyn PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        slot: i16,
    ) -> Result<RuntimeItemStack> {
        let clicked = self.slot_item(slot).into_owned();
        let mut stack = clicked.clone();
        let both = !self.container[INPUT].is_empty() && !self.container[ADDITIONAL].is_empty();
        let moved = if slot == RESULT as i16 {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 39, true)?
        } else if slot != 0 && slot != 1 {
            if both {
                if (3..30).contains(&slot) {
                    self.move_item_stack_to(
                        resolver,
                        maps.as_deref_mut(),
                        &mut stack,
                        30,
                        39,
                        false,
                    )?
                } else if (30..39).contains(&slot) {
                    self.move_item_stack_to(
                        resolver,
                        maps.as_deref_mut(),
                        &mut stack,
                        3,
                        30,
                        false,
                    )?
                } else {
                    true
                }
            } else {
                self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 0, 2, false)?
            }
        } else {
            self.move_item_stack_to(resolver, maps.as_deref_mut(), &mut stack, 3, 39, false)?
        };
        if !moved {
            return Ok(RuntimeItemStack::empty());
        }
        self.finish_utility_move(resolver, maps, slot, &clicked, stack)
    }
}

/// `GrindstoneMenu.computeResult` (`GrindstoneMenu.java:122-139`).
fn compute_result(
    facts: &UtilityMenuFacts,
    input: &RuntimeItemStack,
    additional: &RuntimeItemStack,
) -> Result<RuntimeItemStack> {
    if input.is_empty() && additional.is_empty() {
        return Ok(RuntimeItemStack::empty());
    }
    if input.count() > 1 || additional.count() > 1 {
        return Ok(RuntimeItemStack::empty());
    }
    if input.is_empty() || additional.is_empty() {
        let item = if input.is_empty() { additional } else { input };
        if !has_any_enchantments(item) {
            return Ok(RuntimeItemStack::empty());
        }
        return remove_non_curses(facts, item.clone());
    }
    merge_items(facts, input, additional)
}

/// `GrindstoneMenu.mergeItems` (`GrindstoneMenu.java:141-167`).
fn merge_items(
    facts: &UtilityMenuFacts,
    input: &RuntimeItemStack,
    additional: &RuntimeItemStack,
) -> Result<RuntimeItemStack> {
    if input.item_id() != additional.item_id() {
        return Ok(RuntimeItemStack::empty());
    }
    let durability = max_damage(input)?.max(max_damage(additional)?);
    let remaining1 = max_damage(input)? - damage_value(input)?;
    let remaining2 = max_damage(additional)? - damage_value(additional)?;
    let remaining = remaining1 + remaining2 + durability * 5 / 100;
    let mut count = 1;
    if !is_damageable(input)? {
        if input.max_stack_size()? < 2 || input != additional {
            return Ok(RuntimeItemStack::empty());
        }
        count = 2;
    }
    let resolver = &facts.resolver;
    let mut item = input.with_count(count);
    if is_damageable(&item)? {
        item = with_max_damage(&item, durability, resolver)?;
        item = with_damage(&item, (durability - remaining).max(0), resolver)?;
    }
    // `mergeEnchantsFrom` (`GrindstoneMenu.java:169-180`).
    let mut enchantments = crafting_enchantments(&item);
    for (holder, level) in crafting_enchantments(additional) {
        let curse = facts
            .enchantments
            .get(holder)
            .is_some_and(|definition| definition.curse);
        if !curse || enchantments.iter().all(|(id, _)| *id != holder) {
            upgrade(&mut enchantments, holder, level);
        }
    }
    let item = with_crafting_enchantments(&item, &enchantments, resolver)?;
    remove_non_curses(facts, item)
}

/// `GrindstoneMenu.removeNonCursesFrom` (`GrindstoneMenu.java:182-198`).
fn remove_non_curses(facts: &UtilityMenuFacts, item: RuntimeItemStack) -> Result<RuntimeItemStack> {
    let mut enchantments = crafting_enchantments(&item);
    enchantments.retain(|(holder, _)| {
        facts
            .enchantments
            .get(*holder)
            .is_some_and(|definition| definition.curse)
    });
    let mut item = with_crafting_enchantments(&item, &enchantments, &facts.resolver)?;
    if item.item_id() == Some(VANILLA_ITEM_ENCHANTED_BOOK_ID) && enchantments.is_empty() {
        item = transmute(&item, VANILLA_ITEM_BOOK_ID, &facts.resolver)?;
    }
    let repair_cost = (0..enchantments.len()).fold(0, |cost, _| increased_repair_cost(cost));
    with_repair_cost(&item, repair_cost, &facts.resolver)
}

/// `getExperienceFromItem` (`GrindstoneMenu.java:91-104`).
fn experience_from(facts: &UtilityMenuFacts, item: &RuntimeItemStack) -> i32 {
    crafting_enchantments(item)
        .into_iter()
        .filter_map(|(holder, level)| {
            let definition = facts.enchantments.get(holder)?;
            (!definition.curse).then(|| definition.min_cost(level))
        })
        .sum()
}
