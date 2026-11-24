use std::sync::Arc;

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
            .then_some(record.comparator_output())
            .flatten()
    }

    pub(crate) fn set_comparator_output_at(&mut self, pos: BlockPos, output: i32) -> bool {
        let Some(index) = self.chunks.block_entity_index(pos) else {
            return false;
        };
        let Some(existing) = self
            .chunks
            .get(pos.chunk())
            .and_then(|chunk| chunk.block_entities.get(index))
        else {
            return false;
        };
        if existing.type_id != VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID {
            return false;
        }
        if existing.comparator_output() == Some(output) {
            return true;
        }
        let Some(record) = self
            .chunks
            .get_mut(pos.chunk())
            .and_then(|chunk| Arc::make_mut(&mut chunk.block_entities).get_mut(index))
        else {
            return false;
        };
        record.payload = crate::BlockEntityPayload::ComparatorOutput(output);
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
