use uuid::Uuid;

use crate::Result;

use super::nbt::{decode_nbt_root, find_entry, NbtValue};

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

#[cfg(test)]
mod tests {
    use super::*;

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

    fn write_tag_name(out: &mut Vec<u8>, tag_id: u8, name: &str) {
        out.push(tag_id);
        write_string(out, name);
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
