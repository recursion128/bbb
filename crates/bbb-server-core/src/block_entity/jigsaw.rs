//! `JigsawBlockEntity` (`JigsawBlockEntity.java:25-166`).

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::{JigsawJoint, SetJigsawBlock};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::value_input::int_or;
use super::{entry, unique_field};

const TAG_NAME: &str = "name";
const TAG_TARGET: &str = "target";
const TAG_POOL: &str = "pool";
const TAG_FINAL_STATE: &str = "final_state";
const TAG_JOINT: &str = "joint";
const TAG_PLACEMENT_PRIORITY: &str = "placement_priority";
const TAG_SELECTION_PRIORITY: &str = "selection_priority";

/// `JigsawBlockEntity.EMPTY_ID` / `Pools.EMPTY` (`JigsawBlockEntity.java:27`,
/// `Pools.java:12`).
const EMPTY_ID: &str = "minecraft:empty";
/// `JigsawBlockEntity.DEFAULT_FINAL_STATE` (`JigsawBlockEntity.java:37`).
const DEFAULT_FINAL_STATE: &str = "minecraft:air";

/// `JigsawBlockEntity` (`JigsawBlockEntity.java:38-44`).
#[derive(Debug, Clone, PartialEq)]
pub struct JigsawBlockEntity {
    name: String,
    target: String,
    pool: String,
    joint: JigsawJoint,
    final_state: bbb_protocol::JavaString,
    placement_priority: i32,
    selection_priority: i32,
}

impl Default for JigsawBlockEntity {
    /// Field defaults (`JigsawBlockEntity.java:38-44`).
    fn default() -> Self {
        Self {
            name: EMPTY_ID.to_string(),
            target: EMPTY_ID.to_string(),
            pool: EMPTY_ID.to_string(),
            joint: JigsawJoint::Rollable,
            final_state: DEFAULT_FINAL_STATE.into(),
            placement_priority: 0,
            selection_priority: 0,
        }
    }
}

impl JigsawBlockEntity {
    /// `ServerGamePacketListenerImpl.handleSetJigsawBlock`
    /// (`ServerGamePacketListenerImpl.java:937-943`).
    pub fn apply_edit(&mut self, packet: &SetJigsawBlock) {
        self.name = normalize_resource_location(packet.name.clone())
            .unwrap_or_else(|_| EMPTY_ID.to_string());
        self.target = normalize_resource_location(packet.target.clone())
            .unwrap_or_else(|_| EMPTY_ID.to_string());
        self.pool = normalize_resource_location(packet.pool.clone())
            .unwrap_or_else(|_| EMPTY_ID.to_string());
        self.final_state = packet.final_state.clone();
        self.joint = packet.joint;
        self.placement_priority = packet.placement_priority;
        self.selection_priority = packet.selection_priority;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn pool(&self) -> &str {
        &self.pool
    }

    pub fn final_state(&self) -> &bbb_protocol::JavaString {
        &self.final_state
    }

    pub const fn joint(&self) -> JigsawJoint {
        self.joint
    }

    pub const fn placement_priority(&self) -> i32 {
        self.placement_priority
    }

    pub const fn selection_priority(&self) -> i32 {
        self.selection_priority
    }

    /// `JigsawBlockEntity.saveAdditional` (`JigsawBlockEntity.java:107-116`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry(TAG_NAME, NbtSummaryValue::string(self.name.clone())),
            entry(TAG_TARGET, NbtSummaryValue::string(self.target.clone())),
            entry(TAG_POOL, NbtSummaryValue::string(self.pool.clone())),
            entry(
                TAG_FINAL_STATE,
                NbtSummaryValue::string(self.final_state.clone()),
            ),
            entry(
                TAG_JOINT,
                NbtSummaryValue::string(self.joint.serialized_name()),
            ),
            entry(
                TAG_PLACEMENT_PRIORITY,
                NbtSummaryValue::Int(self.placement_priority),
            ),
            entry(
                TAG_SELECTION_PRIORITY,
                NbtSummaryValue::Int(self.selection_priority),
            ),
        ]
    }

    /// `StructureTemplate.getDefaultJointType` (`StructureTemplate.java:779-781`).
    pub fn load_additional_for_state(&mut self, record: &[NbtSummaryEntry], state: i32) {
        let horizontal = matches!(
            crate::block_behavior::string_property(state, "orientation").as_deref(),
            Some("north_up" | "south_up" | "west_up" | "east_up")
        );
        self.load_additional_with_joint(
            record,
            if horizontal {
                JigsawJoint::Aligned
            } else {
                JigsawJoint::Rollable
            },
        );
    }

    fn load_additional_with_joint(
        &mut self,
        record: &[NbtSummaryEntry],
        default_joint: JigsawJoint,
    ) {
        self.name = nbt_identifier(record, TAG_NAME);
        self.target = nbt_identifier(record, TAG_TARGET);
        self.pool = nbt_identifier(record, TAG_POOL);
        self.final_state = nbt_string(record, TAG_FINAL_STATE)
            .cloned()
            .unwrap_or_else(|| DEFAULT_FINAL_STATE.into());
        self.joint = match unique_field(record, TAG_JOINT) {
            Some(NbtSummaryValue::String(value)) => {
                match value.try_to_string().as_deref().unwrap_or("") {
                    "rollable" => JigsawJoint::Rollable,
                    "aligned" => JigsawJoint::Aligned,
                    _ => default_joint,
                }
            }
            _ => default_joint,
        };
        self.placement_priority = int_or(record, TAG_PLACEMENT_PRIORITY, 0);
        self.selection_priority = int_or(record, TAG_SELECTION_PRIORITY, 0);
    }
}

fn nbt_string<'a>(
    record: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a bbb_protocol::JavaString> {
    match unique_field(record, name) {
        Some(NbtSummaryValue::String(value)) => Some(value),
        _ => None,
    }
}

fn nbt_identifier(record: &[NbtSummaryEntry], name: &str) -> String {
    match unique_field(record, name) {
        Some(NbtSummaryValue::String(value)) => {
            normalize_resource_location(value.try_to_string().unwrap_or_default())
                .unwrap_or_else(|_| EMPTY_ID.to_string())
        }
        _ => EMPTY_ID.to_string(),
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_JIGSAW_ID;
    use bbb_protocol::packets::{BlockPos, JigsawJoint, SetJigsawBlock};
    use bbb_protocol::NbtSummaryValue;

    use super::{
        TAG_FINAL_STATE, TAG_JOINT, TAG_NAME, TAG_PLACEMENT_PRIORITY, TAG_POOL,
        TAG_SELECTION_PRIORITY, TAG_TARGET,
    };
    use crate::block_entity::{new_block_entity, unique_field};

    fn pos() -> BlockPos {
        BlockPos { x: 7, y: 8, z: 9 }
    }

    fn jigsaw() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_JIGSAW_ID)
            .expect("minecraft:jigsaw has a default state")
    }

    fn overlay(
        record: &mut Vec<bbb_protocol::NbtSummaryEntry>,
        fields: Vec<bbb_protocol::NbtSummaryEntry>,
    ) {
        for field in fields {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn jigsaw_save_load_round_trips_owned_fields() {
        let state = jigsaw();
        let mut record = new_block_entity(pos(), state)
            .expect("a jigsaw builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let mut edited = super::JigsawBlockEntity::default();
        edited.apply_edit(&SetJigsawBlock {
            pos: pos(),
            name: "example:entry".to_string(),
            target: "example:target".to_string(),
            pool: "example:village/houses".to_string(),
            final_state: "minecraft:oak_planks[axis=y]".into(),
            joint: JigsawJoint::Aligned,
            placement_priority: 3,
            selection_priority: -2,
        });
        overlay(&mut record, edited.save_additional());
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("jigsaw loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_NAME),
            Some(&NbtSummaryValue::string("example:entry"))
        );
        assert_eq!(
            unique_field(&saved, TAG_TARGET),
            Some(&NbtSummaryValue::string("example:target"))
        );
        assert_eq!(
            unique_field(&saved, TAG_POOL),
            Some(&NbtSummaryValue::string("example:village/houses"))
        );
        assert_eq!(
            unique_field(&saved, TAG_FINAL_STATE),
            Some(&NbtSummaryValue::string("minecraft:oak_planks[axis=y]"))
        );
        assert_eq!(
            unique_field(&saved, TAG_JOINT),
            Some(&NbtSummaryValue::string("aligned"))
        );
        assert_eq!(
            unique_field(&saved, TAG_PLACEMENT_PRIORITY),
            Some(&NbtSummaryValue::Int(3))
        );
        assert_eq!(
            unique_field(&saved, TAG_SELECTION_PRIORITY),
            Some(&NbtSummaryValue::Int(-2))
        );
        let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
            .expect("fixture codec succeeds")
            .expect("second load");
        assert_eq!(
            unique_field(
                &again
                    .save_with_full_metadata_with_resolver(
                        crate::block_entity::test_item_resolver(),
                    )
                    .expect("fixture codec succeeds"),
                TAG_JOINT
            ),
            Some(&NbtSummaryValue::string("aligned"))
        );
    }
}
