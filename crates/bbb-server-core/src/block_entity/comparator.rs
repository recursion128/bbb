//! `ComparatorBlockEntity` (`ComparatorBlockEntity.java:8-34`).
//!
//! The object holds one integer: the last `calculateOutputSignal` the block
//! wrote. `DiodeBlock.getSignal` reads it through
//! `ComparatorBlock.getOutputSignal` (`ComparatorBlock.java:67-70`).

use bbb_protocol::NbtSummaryEntry;
use bbb_protocol::NbtSummaryValue;

use super::entry;
use super::value_input::int_or;

/// `ComparatorBlockEntity.DEFAULT_OUTPUT` (`ComparatorBlockEntity.java:9`).
const DEFAULT_OUTPUT: i32 = 0;
/// The save field `saveAdditional` / `loadAdditional` use (`:19`, `:25`).
const TAG_OUTPUT_SIGNAL: &str = "OutputSignal";

/// `ComparatorBlockEntity` (`ComparatorBlockEntity.java:8-10`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComparatorBlockEntity {
    output: i32,
}

impl Default for ComparatorBlockEntity {
    /// `new ComparatorBlockEntity(pos, state)` (`:12-14`): output starts at
    /// [`DEFAULT_OUTPUT`].
    fn default() -> Self {
        Self {
            output: DEFAULT_OUTPUT,
        }
    }
}

impl ComparatorBlockEntity {
    /// `ComparatorBlockEntity.getOutputSignal` (`:28-30`).
    #[must_use]
    pub const fn output_signal(&self) -> i32 {
        self.output
    }

    /// `ComparatorBlockEntity.setOutputSignal` (`:32-34`).
    pub fn set_output_signal(&mut self, value: i32) {
        self.output = value;
    }

    /// `ComparatorBlockEntity.saveAdditional` (`:17-20`).
    #[must_use]
    pub fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![entry(TAG_OUTPUT_SIGNAL, NbtSummaryValue::Int(self.output))]
    }

    /// `ComparatorBlockEntity.loadAdditional` (`:22-26`).
    pub fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.output = int_or(record, TAG_OUTPUT_SIGNAL, DEFAULT_OUTPUT);
    }
}
