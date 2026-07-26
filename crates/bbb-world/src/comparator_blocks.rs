use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID, decode_nbt_summary,
    NbtSummaryValue,
};

use crate::{BlockPos, Result, WorldStore};

pub(crate) fn decode_comparator_output(raw_nbt: &[u8]) -> Result<i32> {
    if raw_nbt.is_empty() {
        return Ok(0);
    }
    let Some(NbtSummaryValue::Compound(entries)) = decode_nbt_summary(raw_nbt)? else {
        return Ok(0);
    };
    Ok(entries
        .iter()
        .find(|entry| entry.name == "OutputSignal")
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(0))
}

impl WorldStore {
    pub(crate) fn comparator_output_at(&self, pos: BlockPos) -> Option<i32> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID)
            .then_some(record.comparator_output)
            .flatten()
    }

    pub(crate) fn set_comparator_output_at(&mut self, pos: BlockPos, output: i32) -> bool {
        let chunk = self.chunks.iter_mut().find(|chunk| {
            chunk.pos.x == pos.x.div_euclid(16) && chunk.pos.z == pos.z.div_euclid(16)
        });
        let Some(chunk) = chunk else {
            return false;
        };
        let Ok(y) = i16::try_from(pos.y) else {
            return false;
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let Some(record) = chunk
            .block_entities
            .iter_mut()
            .find(|record| record.local_x == local_x && record.y == y && record.local_z == local_z)
        else {
            return false;
        };
        if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID {
            return false;
        }
        record.comparator_output = Some(output);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparator_output_decodes_saved_int_and_defaults_missing_data() {
        assert_eq!(decode_comparator_output(&[]).unwrap(), 0);
        assert_eq!(
            decode_comparator_output(&int_nbt("OutputSignal", 13)).unwrap(),
            13
        );
        assert_eq!(decode_comparator_output(&int_nbt("Other", 7)).unwrap(), 0);
    }

    fn int_nbt(name: &str, value: i32) -> Vec<u8> {
        let mut out = vec![10, 3];
        out.extend_from_slice(&(name.len() as u16).to_be_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&value.to_be_bytes());
        out.push(0);
        out
    }
}
