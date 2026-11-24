use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID, decode_nbt_summary,
    normalize_resource_location, packets::JigsawJoint, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

pub const DEFAULT_JIGSAW_RESOURCE_ID: &str = "minecraft:empty";
pub const DEFAULT_JIGSAW_FINAL_STATE: &str = "minecraft:air";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JigsawBlockEntityDataState {
    pub name: String,
    pub target: String,
    pub pool: String,
    pub final_state: bbb_protocol::JavaString,
    pub joint: JigsawJoint,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

impl Default for JigsawBlockEntityDataState {
    fn default() -> Self {
        Self::with_default_joint(JigsawJoint::Rollable)
    }
}

impl JigsawBlockEntityDataState {
    fn with_default_joint(joint: JigsawJoint) -> Self {
        Self {
            name: DEFAULT_JIGSAW_RESOURCE_ID.to_string(),
            target: DEFAULT_JIGSAW_RESOURCE_ID.to_string(),
            pool: DEFAULT_JIGSAW_RESOURCE_ID.to_string(),
            final_state: DEFAULT_JIGSAW_FINAL_STATE.into(),
            joint,
            selection_priority: 0,
            placement_priority: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JigsawBlockEditorState {
    pub pos: BlockPos,
    pub data: JigsawBlockEntityDataState,
    pub joint_editable: bool,
}

pub(crate) fn decode_jigsaw_block_entity_data(
    raw_nbt: &[u8],
    default_joint: JigsawJoint,
) -> Result<JigsawBlockEntityDataState> {
    let Some(root) = decode_nbt_summary(raw_nbt)? else {
        return Ok(JigsawBlockEntityDataState::with_default_joint(
            default_joint,
        ));
    };
    let NbtSummaryValue::Compound(entries) = root else {
        return Err(bbb_protocol::codec::ProtocolError::InvalidData(
            "jigsaw block entity root is not a compound".to_string(),
        )
        .into());
    };

    Ok(JigsawBlockEntityDataState {
        name: nbt_resource(&entries, "name"),
        target: nbt_resource(&entries, "target"),
        pool: nbt_resource(&entries, "pool"),
        final_state: nbt_string(&entries, "final_state")
            .unwrap_or_else(|| DEFAULT_JIGSAW_FINAL_STATE.into()),
        joint: match nbt_string(&entries, "joint")
            .and_then(|value| value.try_to_string().ok())
            .as_deref()
        {
            Some("rollable") => JigsawJoint::Rollable,
            Some("aligned") => JigsawJoint::Aligned,
            _ => default_joint,
        },
        selection_priority: nbt_int(&entries, "selection_priority").unwrap_or(0),
        placement_priority: nbt_int(&entries, "placement_priority").unwrap_or(0),
    })
}

pub(crate) fn jigsaw_joint_editable_for_orientation(orientation: Option<&str>) -> bool {
    orientation.is_some_and(|orientation| {
        orientation.starts_with("up_") || orientation.starts_with("down_")
    })
}

pub(crate) fn jigsaw_default_joint_for_orientation(orientation: Option<&str>) -> JigsawJoint {
    if jigsaw_joint_editable_for_orientation(orientation) {
        JigsawJoint::Rollable
    } else if orientation.is_some() {
        JigsawJoint::Aligned
    } else {
        JigsawJoint::Rollable
    }
}

impl WorldStore {
    pub fn jigsaw_block_data_at(&self, pos: BlockPos) -> Option<&JigsawBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID)
            .then_some(record.jigsaw_block())
            .flatten()
    }

    /// `JigsawBlockEditScreen`'s block, when the local player may edit it.
    pub fn jigsaw_block_editor_at(&self, pos: BlockPos) -> Option<JigsawBlockEditorState> {
        if !self.local_player_can_use_gamemaster_blocks() || !self.jigsaw_block_is_present_at(pos) {
            return None;
        }
        Some(JigsawBlockEditorState {
            pos,
            data: self.jigsaw_block_data_at(pos)?.clone(),
            joint_editable: self.jigsaw_joint_editable_at(pos),
        })
    }

    pub(crate) fn jigsaw_default_joint_at(&self, pos: BlockPos) -> JigsawJoint {
        let orientation = self
            .block_view(pos)
            .and_then(|probe| probe.block_properties.get("orientation").cloned());
        jigsaw_default_joint_for_orientation(orientation.as_deref())
    }

    fn jigsaw_joint_editable_at(&self, pos: BlockPos) -> bool {
        self.block_view(pos)
            .and_then(|probe| probe.block_properties.get("orientation").cloned())
            .as_deref()
            .is_some_and(|orientation| jigsaw_joint_editable_for_orientation(Some(orientation)))
    }

    fn jigsaw_block_is_present_at(&self, pos: BlockPos) -> bool {
        self.block_view(pos)
            .and_then(|probe| probe.block_name)
            .is_some_and(|name| name == "minecraft:jigsaw")
    }
}

fn entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn nbt_string(entries: &[NbtSummaryEntry], key: &str) -> Option<bbb_protocol::JavaString> {
    let NbtSummaryValue::String(value) = entry(entries, key)? else {
        return None;
    };
    Some(value.clone())
}

fn nbt_resource(entries: &[NbtSummaryEntry], key: &str) -> String {
    nbt_string(entries, key)
        .and_then(|value| {
            value
                .try_to_string()
                .ok()
                .and_then(|value| normalize_resource_location(value).ok())
        })
        .unwrap_or_else(|| DEFAULT_JIGSAW_RESOURCE_ID.to_string())
}

fn nbt_int(entries: &[NbtSummaryEntry], key: &str) -> Option<i32> {
    let NbtSummaryValue::Int(value) = entry(entries, key)? else {
        return None;
    };
    Some(*value)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{
        BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, EntityEvent,
    };

    use super::*;
    use crate::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn decodes_jigsaw_fields_codec_defaults_and_orientation_default_joint() {
        let decoded =
            decode_jigsaw_block_entity_data(&jigsaw_nbt(Some("aligned")), JigsawJoint::Rollable)
                .unwrap();
        assert_eq!(decoded.name, "example:entry");
        assert_eq!(decoded.target, "example:target");
        assert_eq!(decoded.pool, "example:village/houses");
        assert_eq!(decoded.final_state, "minecraft:oak_planks[axis=y]");
        assert_eq!(decoded.joint, JigsawJoint::Aligned);
        assert_eq!(decoded.selection_priority, i32::MIN);
        assert_eq!(decoded.placement_priority, i32::MAX);

        let wrong_types = nbt_compound(vec![
            nbt_int("name", 1),
            nbt_string("target", "Bad Target"),
            nbt_int("pool", 2),
            nbt_int("final_state", 3),
            nbt_string("joint", "unknown"),
            nbt_string("selection_priority", "wrong"),
        ]);
        let defaulted =
            decode_jigsaw_block_entity_data(&wrong_types, JigsawJoint::Aligned).unwrap();
        assert_eq!(
            defaulted,
            JigsawBlockEntityDataState {
                joint: JigsawJoint::Aligned,
                ..JigsawBlockEntityDataState::with_default_joint(JigsawJoint::Aligned)
            }
        );
        assert_eq!(
            decode_jigsaw_block_entity_data(&[0], JigsawJoint::Rollable).unwrap(),
            JigsawBlockEntityDataState::default()
        );
        assert!(decode_jigsaw_block_entity_data(&[3, 0, 0, 0, 1], JigsawJoint::Rollable).is_err());
        assert_eq!(
            jigsaw_default_joint_for_orientation(Some("north_up")),
            JigsawJoint::Aligned
        );
        assert_eq!(
            jigsaw_default_joint_for_orientation(Some("up_north")),
            JigsawJoint::Rollable
        );
        assert!(!jigsaw_joint_editable_for_orientation(Some("east_up")));
        assert!(jigsaw_joint_editable_for_orientation(Some("down_west")));
    }

    #[test]
    fn editor_requires_permission_reads_orientation_and_bad_root_keeps_data() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = jigsaw_world(pos, "up_north", Some("rollable"));
        assert!(store.jigsaw_block_editor_at(pos).is_none());
        grant_permission(&mut store);
        let editor = store.jigsaw_block_editor_at(pos).unwrap();
        assert!(editor.joint_editable);
        assert_eq!(editor.data.name, "example:entry");

        let updated = jigsaw_nbt(Some("aligned"));
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
                raw_nbt: updated,
            })
            .unwrap());
        assert_eq!(
            store.jigsaw_block_data_at(pos).unwrap().joint,
            JigsawJoint::Aligned
        );

        let original = store.jigsaw_block_data_at(pos).cloned().unwrap();
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
                raw_nbt: vec![3, 0, 0, 0, 1],
            })
            .is_err());
        assert_eq!(store.jigsaw_block_data_at(pos), Some(&original));

        let horizontal = crate::vanilla_block_state_id(
            "minecraft:jigsaw",
            &BTreeMap::from([("orientation".to_string(), "north_up".to_string())]),
        )
        .unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: horizontal,
        }));
        assert!(!store.jigsaw_block_editor_at(pos).unwrap().joint_editable);

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.jigsaw_block_editor_at(pos).is_none());
    }

    #[test]
    fn horizontal_jigsaw_defaults_to_an_aligned_joint() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let store = jigsaw_world(pos, "north_up", None);
        assert_eq!(
            store.jigsaw_block_data_at(pos).unwrap().joint,
            JigsawJoint::Aligned
        );
    }

    fn grant_permission(store: &mut WorldStore) {
        store.local_player_id = Some(11);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 11,
            event_id: 26,
        }));
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
    }

    fn jigsaw_world(pos: BlockPos, orientation: &str, joint: Option<&str>) -> WorldStore {
        let block_state_id = crate::vanilla_block_state_id(
            "minecraft:jigsaw",
            &BTreeMap::from([("orientation".to_string(), orientation.to_string())]),
        )
        .unwrap();
        let default_joint = jigsaw_default_joint_for_orientation(Some(orientation));
        let raw_nbt = jigsaw_nbt(joint);
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
        );
        record.raw_nbt = raw_nbt;
        record.payload = crate::BlockEntityPayload::JigsawBlock(Box::new(
            decode_jigsaw_block_entity_data(&record.raw_nbt, default_joint)
                .expect("valid jigsaw fixture"),
        ));

        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(
                    PaletteDomain::BlockStates,
                    block_state_id,
                    4096,
                ),
                biomes: single_value_palette(PaletteDomain::Biomes, 0, 64),
            })],
            block_entities: std::sync::Arc::new(vec![record]),
            light: LightData::default(),
        });
        store
    }

    fn single_value_palette(
        domain: PaletteDomain,
        global_id: i32,
        entry_count: usize,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn jigsaw_nbt(joint: Option<&str>) -> Vec<u8> {
        let mut entries = vec![
            nbt_string("name", "example:entry"),
            nbt_string("target", "example:target"),
            nbt_string("pool", "example:village/houses"),
            nbt_string("final_state", "minecraft:oak_planks[axis=y]"),
            nbt_int("selection_priority", i32::MIN),
            nbt_int("placement_priority", i32::MAX),
        ];
        if let Some(joint) = joint {
            entries.push(nbt_string("joint", joint));
        }
        nbt_compound(entries)
    }

    fn nbt_compound(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![10];
        for entry in entries {
            out.extend(entry);
        }
        out.push(0);
        out
    }

    fn nbt_string(name: &str, value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_nbt_text(&mut out, name);
        write_nbt_text(&mut out, value);
        out
    }

    fn nbt_int(name: &str, value: i32) -> Vec<u8> {
        let mut out = vec![3];
        write_nbt_text(&mut out, name);
        out.extend_from_slice(&value.to_be_bytes());
        out
    }

    fn write_nbt_text(out: &mut Vec<u8>, text: &str) {
        out.extend_from_slice(&(text.len() as u16).to_be_bytes());
        out.extend_from_slice(text.as_bytes());
    }
}
