//! `BannerBlockEntity` (`BannerBlockEntity.java:21-72`).
//!
//! Patterns and CustomName round-trip as opaque extra NBT. Color is on the block.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field};

const TAG_PATTERNS: &str = "patterns";
const TAG_CUSTOM_NAME: &str = "CustomName";

/// `BannerBlockEntity` (`:21-27`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BannerBlockEntity {
    patterns: Vec<NbtSummaryValue>,
    custom_name: Option<NbtSummaryValue>,
}

impl BannerBlockEntity {
    // BannerBlockEntity.java:89-93.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        let patterns = components.read("minecraft:banner_patterns")?;
        self.patterns = match patterns {
            Some(NbtSummaryValue::List(patterns)) => patterns,
            _ => Vec::new(),
        };
        self.custom_name = components.read("minecraft:custom_name")?;
        Ok(())
    }

    // BannerBlockEntity.java:44-46.
    pub(crate) fn custom_name(&self) -> Option<&NbtSummaryValue> {
        self.custom_name.as_ref()
    }

    /// `BannerBlockEntity.collectImplicitComponents` (`:96-100`).
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set(
            "minecraft:banner_patterns",
            Some(NbtSummaryValue::List(self.patterns.clone())),
        );
        components.set("minecraft:custom_name", self.custom_name.clone());
    }

    /// `BannerBlockEntity.saveAdditional` (`:49-56`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if !self.patterns.is_empty() {
            entries.push(entry(
                TAG_PATTERNS,
                NbtSummaryValue::List(self.patterns.clone()),
            ));
        }
        if let Some(name) = &self.custom_name {
            entries.push(entry(TAG_CUSTOM_NAME, name.clone()));
        }
        entries
    }

    /// `BannerBlockEntity.loadAdditional` (`:59-63`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.patterns = match unique_field(record, TAG_PATTERNS) {
            Some(NbtSummaryValue::List(values)) => values.clone(),
            _ => Vec::new(),
        };
        self.custom_name = unique_field(record, TAG_CUSTOM_NAME).cloned();
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_WHITE_BANNER_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use super::{entry, TAG_CUSTOM_NAME, TAG_PATTERNS};
    use crate::block_entity::{new_block_entity, unique_field};

    fn pos() -> BlockPos {
        BlockPos { x: 1, y: 2, z: 3 }
    }

    fn white_banner() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_WHITE_BANNER_ID)
            .expect("minecraft:white_banner has a default state")
    }

    fn patterns() -> NbtSummaryValue {
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry(
                "pattern",
                NbtSummaryValue::string("minecraft:stripe_bottom"),
            ),
            entry("color", NbtSummaryValue::string("red")),
        ])])
    }

    fn custom_name() -> NbtSummaryValue {
        NbtSummaryValue::string("named banner")
    }

    #[test]
    fn banner_save_load_round_trips_patterns_and_custom_name() {
        let state = white_banner();
        let mut record = new_block_entity(pos(), state)
            .expect("a white banner builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.push(entry(TAG_PATTERNS, patterns()));
        record.push(entry(TAG_CUSTOM_NAME, custom_name()));
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("banner loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_PATTERNS), Some(&patterns()));
        assert_eq!(unique_field(&saved, TAG_CUSTOM_NAME), Some(&custom_name()));
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_PATTERNS
            ),
            Some(&patterns())
        );
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_CUSTOM_NAME
            ),
            Some(&custom_name())
        );
    }
}
