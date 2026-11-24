use super::*;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::horse_phenotype::HorsePhenotypeState;
use crate::entity::living_internal_inventory::LivingInternalInventory;

pub(super) fn write_plain_inventory(
    entries: &mut Vec<NbtSummaryEntry>,
    entity_type_id: i32,
    data: &crate::entity::GeneralLivingData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    if crate::entity::living_internal_inventory::plain_inventory_capacity(entity_type_id).is_none()
    {
        return;
    }
    let items = data
        .internal_inventory()
        .expect("InventoryCarrier owns an internal inventory")
        .original_items()
        .filter(|stack| !stack.is_empty())
        .map(|stack| crate::entity::item::write_saved_stack(stack, items))
        .collect();
    entries.push(NbtSummaryEntry {
        name: "Inventory".into(),
        value: NbtSummaryValue::List(items),
    });
}

pub(super) fn read_save_data(
    inventory: &mut LivingInternalInventory,
    entries: &[NbtSummaryEntry],
    horse: Option<HorsePhenotypeState>,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    if let Some(horse) = horse {
        inventory.resize_for_horse(horse);
    }
    if inventory.slots().is_empty() {
        return Ok(());
    }
    let plain = horse.is_none();
    let field = if plain { "Inventory" } else { "Items" };
    let Some(value) = optional_field(entries, field) else {
        return Ok(());
    };
    let NbtSummaryValue::List(saved) = value else {
        return Ok(());
    };
    for raw in saved {
        let Some(stack) = RuntimeItemStack::try_read(raw, items)
            .ok()
            .filter(|stack| !stack.is_empty())
        else {
            continue;
        };
        if plain {
            insert_plain(inventory, stack, items)?;
        } else {
            let NbtSummaryValue::Compound(fields) = raw else {
                continue;
            };
            // ItemStackWithSlot.java:9-18; ExtraCodecs.java:143-147: decode before capacity.
            let slot = optional_field(fields, "Slot")
                .and_then(bbb_protocol::nbt_summary::codec_int_value)
                .unwrap_or(0) as u8 as usize;
            if slot < inventory.slots().len() {
                let max = item::stack_max_size(&stack)
                    .ok_or_else(|| field_error(field, "stack size cannot be resolved"))?
                    .min(99);
                let stack = stack.with_count(item::stack_count(&stack).min(max));
                inventory.set_item(slot, Some(stack));
            }
        }
    }
    Ok(())
}

fn insert_plain(
    inventory: &mut LivingInternalInventory,
    incoming: RuntimeItemStack,
    items: crate::entity::item::ItemResolver<'_>,
) -> Result<(), EntitySaveError> {
    // SimpleContainer.java:71-83, 160-198: merge first, then copyAndClear into one empty slot.
    let incoming_facts = item::merge_facts(&incoming)
        .ok_or_else(|| field_error("Inventory", "incoming stack merge facts cannot be resolved"))?;
    let mut remaining = incoming_facts.count();
    for slot in 0..inventory.slots().len() {
        let Some(current) = inventory.item(slot) else {
            continue;
        };
        let current_facts = item::merge_facts(current).ok_or_else(|| {
            field_error("Inventory", "stored stack merge facts cannot be resolved")
        })?;
        if current
            .same_item_same_effective_components(&incoming, items)
            .map_err(|error| field_error("Inventory", error.to_string()))?
        {
            let moved = remaining
                .min(current_facts.max_stack_size().min(99) - current_facts.count())
                .max(0);
            if moved > 0 {
                inventory.update_count(slot, current_facts.count() + moved);
                remaining -= moved;
                if remaining == 0 {
                    return Ok(());
                }
            }
        }
    }
    if let Some(slot) = (0..inventory.slots().len()).find(|slot| inventory.item(*slot).is_none()) {
        let count = remaining.min(incoming_facts.max_stack_size().min(99));
        let stack = incoming.with_count(count);
        inventory.set_item(slot, Some(stack));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_inventory(
        entity_type_id: i32,
        entries: &[NbtSummaryEntry],
        horse: Option<HorsePhenotypeState>,
        items: crate::entity::item::ItemResolver<'_>,
    ) -> LivingInternalInventory {
        let mut inventory = LivingInternalInventory::new(entity_type_id).unwrap();
        read_save_data(&mut inventory, entries, horse, items).unwrap();
        inventory
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn stack(item: &str, count: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(item)),
            entry("count", NbtSummaryValue::Int(count)),
        ])
    }

    fn stone_with_max(count: i32, max: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:stone")),
            entry("count", NbtSummaryValue::Int(count)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:max_stack_size",
                    NbtSummaryValue::Int(max),
                )]),
            ),
        ])
    }

    fn slotted(slot: Option<i32>, item: &str, count: i32) -> NbtSummaryValue {
        let mut fields = Vec::new();
        if let Some(slot) = slot {
            fields.push(entry("Slot", NbtSummaryValue::Int(slot)));
        }
        if let NbtSummaryValue::Compound(stack) = stack(item, count) {
            fields.extend(stack);
        }
        NbtSummaryValue::Compound(fields)
    }

    fn saved_count(stack: &bbb_server_types::item_stack::RuntimeItemStack) -> NbtSummaryValue {
        let NbtSummaryValue::Compound(fields) = stack
            .write(&bbb_registry_codec::UnavailableHolderResolver)
            .unwrap()
            .unwrap()
        else {
            panic!("saved item stack must be a compound");
        };
        optional_field(&fields, "count").unwrap().clone()
    }

    #[test]
    fn allay_plain_list_merges_to_default_limit_and_discards_extra_item() {
        let entity_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID;
        let items = crate::entity::ItemPublication::builtin().resolver();
        for (incoming_count, explicit_default, expected) in [(40, false, 64), (20, true, 60)] {
            let mut incoming = stack("minecraft:stone", incoming_count);
            if explicit_default {
                let NbtSummaryValue::Compound(fields) = &mut incoming else {
                    unreachable!();
                };
                fields.push(entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:rarity",
                        NbtSummaryValue::String("common".into()),
                    )]),
                ));
            }
            let expected_stack = RuntimeItemStack::try_read(&incoming, items).unwrap();
            let entries = vec![entry(
                "Inventory",
                NbtSummaryValue::List(vec![
                    stack("minecraft:stone", 40),
                    incoming,
                    stack("minecraft:cake", 1),
                ]),
            )];
            let inventory = read_inventory(entity_type, &entries, None, items);
            let retained = inventory.item(0).unwrap();
            assert_eq!(retained.item_resource_id().unwrap(), "minecraft:stone");
            assert_eq!(retained.count(), expected);

            let mut data = crate::entity::GeneralLivingData::new(entity_type).unwrap();
            data.internal_inventory = Some(inventory);
            let mut saved = Vec::new();
            write_plain_inventory(&mut saved, entity_type, &data, items);
            let restored = read_inventory(entity_type, &saved, None, items);
            let restored = restored.item(0).unwrap();
            assert_eq!(restored.count(), expected);
            assert!(restored
                .same_item_same_effective_components(&expected_stack, items)
                .unwrap());
        }
    }

    #[test]
    fn allay_full_slot_does_not_merge_a_different_component_patch() {
        let changed = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:stone")),
            entry("count", NbtSummaryValue::Int(20)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:custom_data",
                    NbtSummaryValue::Compound(vec![entry("token", NbtSummaryValue::Int(1))]),
                )]),
            ),
        ]);
        let entries = vec![entry(
            "Inventory",
            NbtSummaryValue::List(vec![stack("minecraft:stone", 10), changed]),
        )];
        let inventory = read_inventory(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID,
            &entries,
            None,
            &bbb_registry_codec::UnavailableHolderResolver,
        );
        assert_eq!(inventory.slots()[0].as_ref().unwrap().count(), 10);
    }

    #[test]
    fn allay_uses_effective_stack_limit_above_the_default_sixty_four() {
        for (max, each, expected) in [(99, 40, 80), (16, 10, 16)] {
            let entries = vec![entry(
                "Inventory",
                NbtSummaryValue::List(vec![stone_with_max(each, max), stone_with_max(each, max)]),
            )];
            let inventory = read_inventory(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID,
                &entries,
                None,
                &bbb_registry_codec::UnavailableHolderResolver,
            );
            assert_eq!(inventory.slots()[0].as_ref().unwrap().count(), expected);
        }
    }

    #[test]
    fn allay_canonicalizes_java_count_defaults_and_numeric_tags_for_save() {
        for (count_field, expected) in [
            (None, 1),
            (Some(NbtSummaryValue::Byte(3)), 3),
            (Some(NbtSummaryValue::Int(100)), 1),
        ] {
            let mut fields = vec![entry("id", NbtSummaryValue::string("stone"))];
            if let Some(count) = count_field {
                fields.push(entry("count", count));
            }
            let entries = vec![entry(
                "Inventory",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(fields)]),
            )];
            let inventory = read_inventory(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID,
                &entries,
                None,
                &bbb_registry_codec::UnavailableHolderResolver,
            );
            let stack = inventory.item(0).unwrap();
            assert_eq!(stack.item_resource_id().unwrap(), "minecraft:stone");
            assert_eq!(saved_count(stack), NbtSummaryValue::Int(expected));
        }
    }

    #[test]
    fn horse_set_item_clamps_per_item_and_canonicalizes_saved_count() {
        let mut patched = vec![entry("Slot", NbtSummaryValue::Int(2))];
        let NbtSummaryValue::Compound(patched_stack) = stone_with_max(99, 99) else {
            unreachable!();
        };
        patched.extend(patched_stack);
        let entries = vec![entry(
            "Items",
            NbtSummaryValue::List(vec![
                slotted(Some(0), "minecraft:cake", 99),
                slotted(Some(1), "minecraft:stone", 99),
                NbtSummaryValue::Compound(patched),
                NbtSummaryValue::Compound(vec![
                    entry("Slot", NbtSummaryValue::Int(3)),
                    entry("id", NbtSummaryValue::string("minecraft:stone")),
                    entry("count", NbtSummaryValue::Byte(3)),
                ]),
                NbtSummaryValue::Compound(vec![
                    entry("Slot", NbtSummaryValue::Int(4)),
                    entry("id", NbtSummaryValue::string("minecraft:stone")),
                ]),
            ]),
        )];
        let inventory = read_inventory(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID,
            &entries,
            Some(HorsePhenotypeState::chested(true)),
            &bbb_registry_codec::UnavailableHolderResolver,
        );
        for (slot, expected) in [(0, 1), (1, 64), (2, 99), (3, 3), (4, 1)] {
            let stack = inventory.item(slot).unwrap();
            assert_eq!(stack.count(), expected);
            assert_eq!(saved_count(stack), NbtSummaryValue::Int(expected));
        }
    }

    #[test]
    fn horse_item_list_defaults_missing_slot_overwrites_duplicates_and_skips_outside_capacity() {
        let entries = vec![entry(
            "Items",
            NbtSummaryValue::List(vec![
                slotted(None, "minecraft:stone", 1),
                slotted(Some(256), "minecraft:cake", 1),
                slotted(Some(-256), "minecraft:stick", 1),
                slotted(Some(0), "minecraft:apple", 3),
                slotted(Some(14), "minecraft:cake", 1),
                slotted(Some(255), "minecraft:stick", 1),
                NbtSummaryValue::Compound(vec![
                    entry("Slot", NbtSummaryValue::Byte(-1)),
                    entry("id", NbtSummaryValue::string("minecraft:stick")),
                    entry("count", NbtSummaryValue::Int(1)),
                ]),
            ]),
        )];
        let inventory = read_inventory(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID,
            &entries,
            Some(HorsePhenotypeState::chested(true)),
            &bbb_registry_codec::UnavailableHolderResolver,
        );
        assert_eq!(inventory.slots().len(), 15);
        assert_eq!(
            inventory.slots()[0]
                .as_ref()
                .unwrap()
                .item_resource_id()
                .unwrap(),
            "minecraft:apple"
        );
        assert_eq!(
            inventory.slots()[14]
                .as_ref()
                .unwrap()
                .item_resource_id()
                .unwrap(),
            "minecraft:cake"
        );
        assert_eq!(inventory.slots().iter().flatten().count(), 2);
    }
}
