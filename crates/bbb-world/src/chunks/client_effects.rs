use uuid::Uuid;

use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::CampfireBlockEntityData,
};

const CAMPFIRE_SLOT_COUNT: usize = 4;

pub(crate) fn decode_campfire_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<CampfireBlockEntityData>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::List(items)) = find_entry(&entries, "Items") else {
        return Ok(None);
    };

    let mut occupied_slots = [false; CAMPFIRE_SLOT_COUNT];
    for item in items {
        let NbtValue::Compound(item) = item else {
            continue;
        };
        let Some(slot) = nbt_i32(find_entry(item, "Slot"))
            .and_then(|slot| usize::try_from(slot).ok())
            .filter(|slot| *slot < CAMPFIRE_SLOT_COUNT)
        else {
            continue;
        };
        if nbt_item_stack_is_nonempty(item) {
            occupied_slots[slot] = true;
        }
    }

    Ok(Some(CampfireBlockEntityData { occupied_slots }))
}

pub(super) fn nbt_item_stack_is_nonempty(entries: &[(String, NbtValue)]) -> bool {
    let Some(NbtValue::String(item_id)) = find_entry(entries, "id") else {
        return false;
    };
    let count = match find_entry(entries, "count") {
        Some(value) => nbt_i32(Some(value)),
        None => Some(1),
    };
    item_id != "minecraft:air"
        && vanilla_item_type_for_resource_id(item_id).is_some()
        && count.is_some_and(|count| (1..=99).contains(&count))
}

pub(crate) fn decode_conduit_destroy_target(raw_nbt: &[u8]) -> Result<Option<Uuid>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::IntArray(values)) = find_entry(&entries, "Target") else {
        return Ok(None);
    };
    Ok(uuid_from_int_array(values))
}

fn uuid_from_int_array(values: &[i32]) -> Option<Uuid> {
    if values.len() != 4 {
        return None;
    }
    let high = u64::from(values[0] as u32) << 32 | u64::from(values[1] as u32);
    let low = u64::from(values[2] as u32) << 32 | u64::from(values[3] as u32);
    Some(Uuid::from_u128((u128::from(high) << 64) | u128::from(low)))
}

fn nbt_i32(value: Option<&NbtValue>) -> Option<i32> {
    match value? {
        NbtValue::Byte(value) => Some(i32::from(*value)),
        NbtValue::Short(value) => Some(i32::from(*value)),
        NbtValue::Int(value) => Some(*value),
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

        assert_eq!(
            decode_campfire_block_entity_data(&nbt).unwrap(),
            Some(CampfireBlockEntityData {
                occupied_slots: [true, false, false, false],
            })
        );
    }

    #[test]
    fn conduit_target_requires_a_four_int_uuid() {
        let target = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
        let mut nbt = Vec::new();
        nbt.push(10);
        write_tag_name(&mut nbt, 11, "Target");
        nbt.extend_from_slice(&4_i32.to_be_bytes());
        for part in [0x00112233_u32, 0x44556677, 0x8899aabb, 0xccddeeff] {
            nbt.extend_from_slice(&(part as i32).to_be_bytes());
        }
        nbt.push(0);

        assert_eq!(decode_conduit_destroy_target(&nbt).unwrap(), Some(target));
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
