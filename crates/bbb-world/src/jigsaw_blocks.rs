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
    pub final_state: String,
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
            final_state: DEFAULT_JIGSAW_FINAL_STATE.to_string(),
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
    #[serde(default)]
    pub authoritative_revision: usize,
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
            .unwrap_or_else(|| DEFAULT_JIGSAW_FINAL_STATE.to_string()),
        joint: match nbt_string(&entries, "joint").as_deref() {
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
            .then_some(record.jigsaw_block.as_ref())
            .flatten()
    }

    pub fn jigsaw_block_editor(&self) -> Option<&JigsawBlockEditorState> {
        self.client_ui.current_jigsaw_block_editor.as_ref()
    }

    pub fn jigsaw_block_editor_open_count(&self) -> usize {
        self.client_ui.jigsaw_block_editor_open_count
    }

    pub fn try_open_jigsaw_block_editor(&mut self, pos: BlockPos) -> bool {
        if !self.local_player_can_use_gamemaster_blocks() || !self.jigsaw_block_is_present_at(pos) {
            return false;
        }
        let Some(data) = self.jigsaw_block_data_at(pos).cloned() else {
            return false;
        };
        let joint_editable = self.jigsaw_joint_editable_at(pos);
        self.client_ui.replace_screen();
        self.client_ui.jigsaw_block_editor_open_count += 1;
        self.client_ui.current_jigsaw_block_editor = Some(JigsawBlockEditorState {
            pos,
            data,
            joint_editable,
            authoritative_revision: 0,
        });
        true
    }

    pub fn close_jigsaw_block_editor(&mut self) -> bool {
        self.client_ui.current_jigsaw_block_editor.take().is_some()
    }

    pub(crate) fn refresh_jigsaw_block_editor_from_authoritative_data(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_jigsaw_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        if !self.jigsaw_block_is_present_at(pos) {
            self.close_jigsaw_block_editor();
            return;
        }
        let Some(data) = self.jigsaw_block_data_at(pos).cloned() else {
            self.close_jigsaw_block_editor();
            return;
        };
        let joint_editable = self.jigsaw_joint_editable_at(pos);
        if let Some(editor) = &mut self.client_ui.current_jigsaw_block_editor {
            editor.data = data;
            editor.joint_editable = joint_editable;
            editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
        }
    }

    pub(crate) fn reconcile_jigsaw_block_editor_after_block_change(&mut self, pos: BlockPos) {
        if self
            .client_ui
            .current_jigsaw_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        if !self.jigsaw_block_is_present_at(pos) || self.jigsaw_block_data_at(pos).is_none() {
            self.close_jigsaw_block_editor();
            return;
        }
        let joint_editable = self.jigsaw_joint_editable_at(pos);
        if let Some(editor) = &mut self.client_ui.current_jigsaw_block_editor {
            editor.joint_editable = joint_editable;
        }
    }

    pub(crate) fn close_jigsaw_block_editor_in_chunk(&mut self, chunk: crate::ChunkPos) {
        if self
            .client_ui
            .current_jigsaw_block_editor
            .as_ref()
            .is_some_and(|editor| {
                editor.pos.x.div_euclid(16) == chunk.x && editor.pos.z.div_euclid(16) == chunk.z
            })
        {
            self.close_jigsaw_block_editor();
        }
    }

    pub(crate) fn jigsaw_default_joint_at(&self, pos: BlockPos) -> JigsawJoint {
        let orientation = self
            .probe_block(pos)
            .and_then(|probe| probe.block_properties.get("orientation").cloned());
        jigsaw_default_joint_for_orientation(orientation.as_deref())
    }

    fn jigsaw_joint_editable_at(&self, pos: BlockPos) -> bool {
        self.probe_block(pos)
            .and_then(|probe| probe.block_properties.get("orientation").cloned())
            .as_deref()
            .is_some_and(|orientation| jigsaw_joint_editable_for_orientation(Some(orientation)))
    }

    fn jigsaw_block_is_present_at(&self, pos: BlockPos) -> bool {
        self.probe_block(pos)
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

fn nbt_string(entries: &[NbtSummaryEntry], key: &str) -> Option<String> {
    let NbtSummaryValue::String(value) = entry(entries, key)? else {
        return None;
    };
    Some(value.clone())
}

fn nbt_resource(entries: &[NbtSummaryEntry], key: &str) -> String {
    nbt_string(entries, key)
        .and_then(|value| normalize_resource_location(value).ok())
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
    fn decodes_jigsaw_fields_codec_fallbacks_and_orientation_default_joint() {
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
        let fallback = decode_jigsaw_block_entity_data(&wrong_types, JigsawJoint::Aligned).unwrap();
        assert_eq!(
            fallback,
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
    fn editor_requires_permission_refreshes_orientation_and_rejects_bad_root_atomically() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = jigsaw_world(pos, "up_north", Some("rollable"));
        assert!(!store.try_open_jigsaw_block_editor(pos));
        grant_permission(&mut store);
        assert!(store.try_open_jigsaw_block_editor(pos));
        let editor = store.jigsaw_block_editor().unwrap();
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
        let editor = store.jigsaw_block_editor().unwrap();
        assert_eq!(editor.authoritative_revision, 1);
        assert_eq!(editor.data.joint, JigsawJoint::Aligned);

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
        assert!(!store.jigsaw_block_editor().unwrap().joint_editable);

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.jigsaw_block_editor().is_none());
    }

    #[test]
    fn editor_closes_on_chunk_forget_and_level_clear() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = jigsaw_world(pos, "north_up", None);
        assert_eq!(
            store.jigsaw_block_data_at(pos).unwrap().joint,
            JigsawJoint::Aligned
        );
        grant_permission(&mut store);
        assert!(store.try_open_jigsaw_block_editor(pos));
        assert!(store.forget_chunk(ChunkPos { x: 0, z: 0 }));
        assert!(store.jigsaw_block_editor().is_none());

        let mut store = jigsaw_world(pos, "up_north", None);
        grant_permission(&mut store);
        assert!(store.try_open_jigsaw_block_editor(pos));
        store.clear_client_level();
        assert!(store.jigsaw_block_editor().is_none());
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
        record.jigsaw_block = Some(
            decode_jigsaw_block_entity_data(&record.raw_nbt, default_joint)
                .expect("valid jigsaw fixture"),
        );

        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        store.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(
                    PaletteDomain::BlockStates,
                    block_state_id,
                    4096,
                ),
                biomes: single_value_palette(PaletteDomain::Biomes, 0, 64),
            }],
            block_entities: vec![record],
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
