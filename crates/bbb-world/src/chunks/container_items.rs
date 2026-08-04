//! Persistent block-entity item payloads sent to the client.
//!
//! Several block entities publish their contents through
//! `ContainerHelper.saveAllItems`, i.e. an `Items` list of `{Slot, <ItemStack>}`
//! compounds. Their renderers resolve those stacks against the session
//! registries, so decoding stops at the persistent NBT and this module owns the
//! single shape/validity rule shared by every such block entity.

use bbb_protocol::{
    decode_nbt_summary, item_types::vanilla_item_type_for_resource_id, NbtSummaryEntry,
    NbtSummaryValue,
};

use crate::{Result, WorldDecodeError};

use super::state::{CampfireBlockEntityData, ShelfBlockEntityData};

/// `CampfireBlockEntity` holds four cooking slots.
const CAMPFIRE_SLOT_COUNT: usize = 4;
/// `ShelfBlockEntity` holds three display slots.
const SHELF_SLOT_COUNT: usize = 3;

/// Decodes `CampfireBlockEntity.getUpdateTag`.
pub(crate) fn decode_campfire_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<CampfireBlockEntityData>> {
    let Some(NbtSummaryValue::Compound(entries)) = summary_root(raw_nbt)? else {
        return Ok(None);
    };
    let Some(items) = decode_container_slots::<CAMPFIRE_SLOT_COUNT>(&entries) else {
        return Ok(None);
    };
    Ok(Some(CampfireBlockEntityData { items }))
}

/// Decodes `ShelfBlockEntity.getUpdateTag`: the same container list plus the
/// renderer's `align_items_to_bottom` flag.
pub(crate) fn decode_shelf_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<ShelfBlockEntityData>> {
    let Some(NbtSummaryValue::Compound(entries)) = summary_root(raw_nbt)? else {
        return Ok(None);
    };
    let Some(items) = decode_container_slots::<SHELF_SLOT_COUNT>(&entries) else {
        return Ok(None);
    };
    Ok(Some(ShelfBlockEntityData {
        items,
        // `ValueInput.getBooleanOr("align_items_to_bottom", false)`.
        align_items_to_bottom: matches!(
            summary_field(&entries, "align_items_to_bottom"),
            Some(NbtSummaryValue::Byte(value)) if *value != 0
        ),
    }))
}

/// Reduces one `ContainerHelper.saveAllItems` list to its fixed slot array,
/// keeping only entries that are renderable stacks in a legal slot.
fn decode_container_slots<const N: usize>(
    entries: &[NbtSummaryEntry],
) -> Option<[Option<NbtSummaryValue>; N]> {
    let NbtSummaryValue::List(items) = summary_field(entries, "Items")? else {
        return None;
    };
    let mut slots: [Option<NbtSummaryValue>; N] = std::array::from_fn(|_| None);
    for item in items {
        let NbtSummaryValue::Compound(item_entries) = item else {
            continue;
        };
        let Some(slot) = summary_i32(summary_field(item_entries, "Slot"))
            .and_then(|slot| usize::try_from(slot).ok())
            .filter(|slot| *slot < N)
        else {
            continue;
        };
        if summary_item_stack_is_nonempty(item_entries) {
            slots[slot] = Some(item.clone());
        }
    }
    Some(slots)
}

pub(super) fn summary_root(raw_nbt: &[u8]) -> Result<Option<NbtSummaryValue>> {
    decode_nbt_summary(raw_nbt).map_err(|error| WorldDecodeError::Protocol(error).into())
}

pub(super) fn summary_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

/// Vanilla treats a slot as filled when it holds a registered, non-air stack
/// with a legal count; `ItemStack.CODEC` rejects everything else.
pub(super) fn summary_item_stack_is_nonempty(entries: &[NbtSummaryEntry]) -> bool {
    let Some(NbtSummaryValue::String(item_id)) = summary_field(entries, "id") else {
        return false;
    };
    let count = match summary_field(entries, "count") {
        Some(value) => summary_i32(Some(value)),
        None => Some(1),
    };
    item_id != "minecraft:air"
        && vanilla_item_type_for_resource_id(item_id).is_some()
        && count.is_some_and(|count| (1..=99).contains(&count))
}

fn summary_i32(value: Option<&NbtSummaryValue>) -> Option<i32> {
    match value? {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campfire_items_decode_only_valid_non_empty_slots() {
        let mut nbt = Vec::new();
        nbt.push(10);
        write_tag_name(&mut nbt, 9, "Items");
        nbt.push(10);
        nbt.extend_from_slice(&4_i32.to_be_bytes());
        write_item(&mut nbt, 0, "minecraft:cod", 1);
        write_item(&mut nbt, 2, "minecraft:air", 1);
        write_item(&mut nbt, 3, "minecraft:potato", 0);
        write_item(&mut nbt, 8, "minecraft:beef", 1);
        nbt.push(0);

        let decoded = decode_campfire_block_entity_data(&nbt)
            .unwrap()
            .expect("campfire data");
        assert_eq!(decoded.occupied_slots(), [true, false, false, false]);
        // The occupied slot keeps its persistent stack NBT for the renderer.
        let NbtSummaryValue::Compound(stack) = decoded.items[0].as_ref().expect("cod stack") else {
            panic!("cooked slot keeps a compound stack");
        };
        assert_eq!(
            summary_field(stack, "id"),
            Some(&NbtSummaryValue::String("minecraft:cod".to_string()))
        );
        assert!(decoded.items[1..].iter().all(Option::is_none));
    }

    #[test]
    fn shelf_items_decode_with_the_bottom_alignment_flag() {
        let mut nbt = Vec::new();
        nbt.push(10);
        write_tag_name(&mut nbt, 9, "Items");
        nbt.push(10);
        nbt.extend_from_slice(&2_i32.to_be_bytes());
        write_item(&mut nbt, 1, "minecraft:apple", 3);
        // Slot 3 is outside the three-slot shelf container.
        write_item(&mut nbt, 3, "minecraft:apple", 1);
        write_tag_name(&mut nbt, 1, "align_items_to_bottom");
        nbt.push(1);
        nbt.push(0);

        let decoded = decode_shelf_block_entity_data(&nbt)
            .unwrap()
            .expect("shelf data");
        assert!(decoded.align_items_to_bottom);
        assert!(decoded.items[0].is_none());
        assert!(decoded.items[1].is_some());
        assert!(decoded.items[2].is_none());
    }

    #[test]
    fn shelves_without_the_flag_align_to_the_slot_centre() {
        let mut nbt = Vec::new();
        nbt.push(10);
        write_tag_name(&mut nbt, 9, "Items");
        nbt.push(10);
        nbt.extend_from_slice(&0_i32.to_be_bytes());
        nbt.push(0);

        let decoded = decode_shelf_block_entity_data(&nbt)
            .unwrap()
            .expect("shelf data");
        assert!(!decoded.align_items_to_bottom);
        assert!(decoded.items.iter().all(Option::is_none));
    }

    fn write_item(out: &mut Vec<u8>, slot: i8, item_id: &str, count: i32) {
        write_tag_name(out, 1, "Slot");
        out.push(slot as u8);
        write_tag_name(out, 8, "id");
        write_string(out, item_id);
        write_tag_name(out, 3, "count");
        out.extend_from_slice(&count.to_be_bytes());
        out.push(0);
    }

    fn write_tag_name(out: &mut Vec<u8>, tag_id: u8, name: &str) {
        out.push(tag_id);
        write_string(out, name);
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
