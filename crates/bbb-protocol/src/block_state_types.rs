//! Fixed Vanilla 26.1 block-state registry wire identities.

/// The checked-in Vanilla 26.1 global block-state registry report.
///
/// Protocol decode needs only the contiguous id range. World and pack
/// consumers reuse the same report for names, properties, and model coverage
/// instead of embedding a second registry snapshot.
pub const VANILLA_BLOCK_STATES_26_1_JSON: &str = include_str!("../data/block_states_26_1.json");

/// `Block.BLOCK_STATE_REGISTRY.size()` for Vanilla 26.1.
pub const VANILLA_BLOCK_STATE_COUNT: i32 = 29_873;

pub fn vanilla_block_state_id(id: i32) -> Option<i32> {
    (0..VANILLA_BLOCK_STATE_COUNT).contains(&id).then_some(id)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize)]
    struct BlockStateReport {
        version: String,
        states: Vec<BlockStateEntry>,
    }

    #[derive(Debug, Deserialize)]
    struct BlockStateEntry {
        id: i32,
    }

    #[test]
    fn block_state_registry_report_is_contiguous_and_matches_wire_range() {
        let report: BlockStateReport =
            serde_json::from_str(VANILLA_BLOCK_STATES_26_1_JSON).unwrap();
        assert_eq!(report.version, crate::MC_VERSION);
        assert_eq!(report.states.len(), VANILLA_BLOCK_STATE_COUNT as usize);
        for (id, state) in report.states.iter().enumerate() {
            assert_eq!(state.id, id as i32);
        }
        assert_eq!(vanilla_block_state_id(-1), None);
        assert_eq!(vanilla_block_state_id(0), Some(0));
        assert_eq!(
            vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT - 1),
            Some(VANILLA_BLOCK_STATE_COUNT - 1)
        );
        assert_eq!(vanilla_block_state_id(VANILLA_BLOCK_STATE_COUNT), None);
    }
}
