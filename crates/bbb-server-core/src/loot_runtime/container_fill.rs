//! `LootTable.fill` (`LootTable.java:146-165`): a container loot table drawn and
//! scattered over a container's empty slots, which is what
//! `RandomizableContainer.unpackLootTable` (`RandomizableContainer.java:72-90`)
//! runs the first time a packed container is read.

use crate::random::GameRandom;
#[cfg(test)]
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_datapack::loot::LootRegistries;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{loot_table_drops, LootParams, Result};

/// `Util.shuffle` (`Util.java:1026-1033`).
fn shuffle<T>(random: &GameRandom, list: &mut [T]) {
    for size in (2..=list.len()).rev() {
        list.swap(size - 1, random.next_int(size as i32) as usize);
    }
}

/// `LootTable.fill(container, params, seed)` (`LootTable.java:146-165`) over
/// the caller's available slots, answering owned slot writes in call order.
///
/// The stacks are drawn first, the empty slots shuffled, the stacks split and
/// shuffled across them, and each written into the last remaining slot. A
/// draw with more stacks than slots stops where vanilla logs its over-fill
/// warning.
pub fn fill_container(
    tables: &LootRegistries,
    table_id: &str,
    params: &LootParams<'_>,
    random: &GameRandom,
    available: Vec<usize>,
) -> Result<Vec<(usize, RuntimeItemStack)>> {
    let stacks = loot_table_drops(tables, table_id, params, random)?;
    let stacks = stacks
        .into_iter()
        .map(RuntimeItemStack::from_summary)
        .collect();
    Ok(scatter(stacks, random, available))
}

/// `LootTable.java:150-164`: everything `fill` does after the draw.
fn scatter(
    mut stacks: Vec<RuntimeItemStack>,
    random: &GameRandom,
    mut available: Vec<usize>,
) -> Vec<(usize, RuntimeItemStack)> {
    shuffle(random, &mut available);
    shuffle_and_split_items(&mut stacks, available.len(), random);
    available.into_iter().rev().zip(stacks).collect()
}

/// `LootTable.shuffleAndSplitItems` (`LootTable.java:167-200`).
fn shuffle_and_split_items(
    result: &mut Vec<RuntimeItemStack>,
    available_slots: usize,
    random: &GameRandom,
) {
    let mut splittable: Vec<_> = result
        .extract_if(.., |stack| stack.count() != 1)
        .filter(|stack| stack.count() > 1)
        .collect();
    while available_slots as i64 - result.len() as i64 - splittable.len() as i64 > 0
        && !splittable.is_empty()
    {
        let index = random.next_int_between_inclusive(0, splittable.len() as i32 - 1) as usize;
        let mut stack: RuntimeItemStack = splittable.remove(index);
        let remove = random.next_int_between_inclusive(1, stack.count() / 2);
        // `ItemStack.split` (`ItemStack.java:328-333`).
        let copy = stack.split(remove);
        if stack.count() > 1 && random.next_boolean() {
            splittable.push(stack);
        } else {
            result.push(stack);
        }
        if copy.count() > 1 && random.next_boolean() {
            splittable.push(copy);
        } else {
            result.push(copy);
        }
    }
    result.append(&mut splittable);
    shuffle(random, result);
}

#[cfg(test)]
mod tests {
    use bbb_protocol::item_types::{vanilla_item_type, vanilla_item_type_for_resource_id};

    use super::*;

    fn stack(name: &str, count: i32) -> RuntimeItemStack {
        ItemStackSummary {
            item_id: Some(
                vanilla_item_type_for_resource_id(&format!("minecraft:{name}"))
                    .expect("a registered item")
                    .id,
            ),
            count,
            ..ItemStackSummary::empty()
        }
        .into()
    }

    #[test]
    fn the_scatter_places_drawn_items_only_in_empty_slots() {
        for seed in [42_i64, -7, 8_675_309] {
            let mut slots = vec![RuntimeItemStack::empty(); 27];
            for slot in &mut slots[1..=24] {
                *slot = stack("bedrock", 1);
            }
            let drawn = vec![
                stack("stone", 10),
                stack("dirt", 3),
                RuntimeItemStack::empty(),
                stack("diamond", 1),
                stack("gold_ingot", 5),
                stack("apple", 1),
            ];
            let available = slots
                .iter()
                .enumerate()
                .filter(|(_, stack)| stack.is_empty())
                .map(|(slot, _)| slot)
                .collect();
            for (slot, stack) in scatter(drawn, &GameRandom::with_seed(seed), available) {
                slots[slot] = stack;
            }
            assert!(slots[1..=24]
                .iter()
                .all(|slot| slot.item_id() == stack("bedrock", 1).item_id()));
            assert!([0, 25, 26].iter().all(|&slot| slots[slot].count() > 0));
        }

        // With room to spare every drawn item lands, split or not.
        for seed in [42_i64, -7, 8_675_309] {
            let mut slots = vec![RuntimeItemStack::empty(); 27];
            let drawn = vec![
                stack("stone", 10),
                stack("dirt", 3),
                stack("diamond", 1),
                stack("gold_ingot", 5),
                stack("apple", 1),
            ];
            let available = slots
                .iter()
                .enumerate()
                .filter(|(_, stack)| stack.is_empty())
                .map(|(slot, _)| slot)
                .collect();
            for (slot, stack) in scatter(drawn, &GameRandom::with_seed(seed), available) {
                slots[slot] = stack;
            }
            let mut totals = std::collections::BTreeMap::new();
            for slot in &slots {
                if let Some(id) = slot.item_id() {
                    let name = vanilla_item_type(id).unwrap().resource_id;
                    *totals.entry(name).or_insert(0) += slot.count();
                }
            }
            assert_eq!(
                totals,
                std::collections::BTreeMap::from([
                    ("minecraft:apple", 1),
                    ("minecraft:diamond", 1),
                    ("minecraft:dirt", 3),
                    ("minecraft:gold_ingot", 5),
                    ("minecraft:stone", 10),
                ]),
                "seed={seed}"
            );
        }
    }
}
