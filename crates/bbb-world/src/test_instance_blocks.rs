use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
    codec::ProtocolError,
    decode_nbt_summary, normalize_resource_location,
    packets::{TestInstanceBlockRotation, TestInstanceBlockStatus, Vec3i},
    NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockSizeState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl From<Vec3i> for TestInstanceBlockSizeState {
    fn from(value: Vec3i) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestInstanceBlockRunStatus {
    #[default]
    Cleared,
    Running,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockEntityDataState {
    pub test: Option<String>,
    pub size: TestInstanceBlockSizeState,
    pub rotation: TestInstanceBlockRotation,
    pub ignore_entities: bool,
    pub status: TestInstanceBlockRunStatus,
    pub error_message: Option<RichTextComponent>,
}

impl Default for TestInstanceBlockEntityDataState {
    fn default() -> Self {
        Self {
            test: None,
            size: TestInstanceBlockSizeState::default(),
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
            status: TestInstanceBlockRunStatus::Cleared,
            error_message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockEditorState {
    pub pos: BlockPos,
    pub data: TestInstanceBlockEntityDataState,
    pub response_status: Option<RichTextComponent>,
    #[serde(default)]
    pub response_size: Option<TestInstanceBlockSizeState>,
    #[serde(default)]
    pub authoritative_revision: usize,
    #[serde(default)]
    pub response_revision: usize,
}

pub(crate) fn decode_test_instance_block_entity_data(
    raw_nbt: &[u8],
) -> Result<TestInstanceBlockEntityDataState> {
    let Some(root) = decode_nbt_summary(raw_nbt)? else {
        return Ok(TestInstanceBlockEntityDataState::default());
    };
    let NbtSummaryValue::Compound(root) = root else {
        return invalid("test-instance block entity root is not a compound");
    };
    let Some(data) = entry(&root, "data") else {
        return Ok(TestInstanceBlockEntityDataState::default());
    };
    let NbtSummaryValue::Compound(data) = data else {
        return invalid("test-instance block entity data is not a compound");
    };

    let test = match entry(data, "test") {
        Some(NbtSummaryValue::String(test)) => {
            Some(normalize_resource_location(test.clone()).map_err(crate::WorldDecodeError::from)?)
        }
        Some(_) => return invalid("test-instance block entity test is not a string"),
        None => None,
    };
    let size = match entry(data, "size") {
        Some(NbtSummaryValue::IntArray(size)) if size.len() == 3 => TestInstanceBlockSizeState {
            x: size[0],
            y: size[1],
            z: size[2],
        },
        Some(_) => return invalid("test-instance block entity size is not a three-int array"),
        None => return invalid("test-instance block entity data is missing size"),
    };
    let rotation = match entry(data, "rotation") {
        Some(NbtSummaryValue::String(rotation)) => match rotation.as_str() {
            "none" => TestInstanceBlockRotation::None,
            "clockwise_90" => TestInstanceBlockRotation::Clockwise90,
            "180" => TestInstanceBlockRotation::Clockwise180,
            "counterclockwise_90" => TestInstanceBlockRotation::Counterclockwise90,
            other => return invalid(format!("invalid test-instance block rotation `{other}`")),
        },
        Some(_) => return invalid("test-instance block entity rotation is not a string"),
        None => return invalid("test-instance block entity data is missing rotation"),
    };
    let ignore_entities = match entry(data, "ignore_entities") {
        Some(NbtSummaryValue::Byte(value)) => *value != 0,
        Some(_) => return invalid("test-instance block entity ignore_entities is not a byte"),
        None => return invalid("test-instance block entity data is missing ignore_entities"),
    };
    let status = match entry(data, "status") {
        Some(NbtSummaryValue::String(status)) => match status.as_str() {
            "cleared" => TestInstanceBlockRunStatus::Cleared,
            "running" => TestInstanceBlockRunStatus::Running,
            "finished" => TestInstanceBlockRunStatus::Finished,
            other => return invalid(format!("invalid test-instance block status `{other}`")),
        },
        Some(_) => return invalid("test-instance block entity status is not a string"),
        None => return invalid("test-instance block entity data is missing status"),
    };
    let error_message = entry(data, "error_message")
        .cloned()
        .map(RichTextComponent::try_from_component_nbt)
        .transpose()
        .map_err(|reason| {
            crate::WorldDecodeError::from(ProtocolError::InvalidData(format!(
                "invalid test-instance block error_message: {reason}"
            )))
        })?;

    Ok(TestInstanceBlockEntityDataState {
        test,
        size,
        rotation,
        ignore_entities,
        status,
        error_message,
    })
}

impl WorldStore {
    pub fn test_instance_block_data_at(
        &self,
        pos: BlockPos,
    ) -> Option<&TestInstanceBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID)
            .then_some(record.test_instance_block.as_ref())
            .flatten()
    }

    pub fn test_instance_block_editor(&self) -> Option<&TestInstanceBlockEditorState> {
        self.client_ui.current_test_instance_block_editor.as_ref()
    }

    pub fn test_instance_block_editor_open_count(&self) -> usize {
        self.client_ui.test_instance_block_editor_open_count
    }

    pub fn try_open_test_instance_block_editor(&mut self, pos: BlockPos) -> bool {
        if !self.local_player_has_gamemaster_permission() || !self.test_instance_block_is_valid(pos)
        {
            return false;
        }
        let Some(data) = self.test_instance_block_data_at(pos).cloned() else {
            return false;
        };
        self.client_ui.replace_screen();
        self.client_ui.test_instance_block_editor_open_count += 1;
        self.client_ui.current_test_instance_block_editor = Some(TestInstanceBlockEditorState {
            pos,
            data,
            response_status: None,
            response_size: None,
            authoritative_revision: 0,
            response_revision: 0,
        });
        true
    }

    pub fn close_test_instance_block_editor(&mut self) -> bool {
        self.client_ui
            .current_test_instance_block_editor
            .take()
            .is_some()
    }

    pub(crate) fn apply_test_instance_block_status_to_editor(
        &mut self,
        packet: &TestInstanceBlockStatus,
    ) {
        let Some(editor) = &mut self.client_ui.current_test_instance_block_editor else {
            return;
        };
        editor.response_status = Some(packet.status.clone());
        editor.response_size = packet.size.map(Into::into);
        if let Some(size) = editor.response_size {
            editor.data.size = size;
        }
        editor.response_revision = editor.response_revision.saturating_add(1);
    }

    pub(crate) fn refresh_test_instance_block_editor_from_authoritative_data(
        &mut self,
        pos: BlockPos,
    ) {
        if self
            .client_ui
            .current_test_instance_block_editor
            .as_ref()
            .is_none_or(|editor| editor.pos != pos)
        {
            return;
        }
        if !self.test_instance_block_is_valid(pos) {
            self.close_test_instance_block_editor();
            return;
        }
        let Some(data) = self.test_instance_block_data_at(pos).cloned() else {
            self.close_test_instance_block_editor();
            return;
        };
        if let Some(editor) = &mut self.client_ui.current_test_instance_block_editor {
            editor.data = data;
            editor.authoritative_revision = editor.authoritative_revision.saturating_add(1);
        }
    }

    pub(crate) fn reconcile_test_instance_block_editor_after_block_change(
        &mut self,
        pos: BlockPos,
    ) {
        if self
            .client_ui
            .current_test_instance_block_editor
            .as_ref()
            .is_some_and(|editor| editor.pos == pos)
            && (!self.test_instance_block_is_valid(pos)
                || self.test_instance_block_data_at(pos).is_none())
        {
            self.close_test_instance_block_editor();
        }
    }

    pub(crate) fn close_test_instance_block_editor_in_chunk(&mut self, chunk: crate::ChunkPos) {
        if self
            .client_ui
            .current_test_instance_block_editor
            .as_ref()
            .is_some_and(|editor| {
                editor.pos.x.div_euclid(16) == chunk.x && editor.pos.z.div_euclid(16) == chunk.z
            })
        {
            self.close_test_instance_block_editor();
        }
    }

    fn test_instance_block_is_valid(&self, pos: BlockPos) -> bool {
        self.probe_block(pos)
            .and_then(|probe| probe.block_name)
            .is_some_and(|name| name == "minecraft:test_instance_block")
    }
}

fn entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(ProtocolError::InvalidData(reason.into()).into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        packets::{
            BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate, EntityEvent,
            TestInstanceBlockStatus, Vec3i,
        },
    };

    use super::*;
    use crate::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn decodes_nested_test_instance_data_defaults_and_rejects_malformed_fields() {
        let decoded = decode_test_instance_block_entity_data(&test_instance_nbt(true)).unwrap();
        assert_eq!(decoded.test.as_deref(), Some("example:rotation_test"));
        assert_eq!(
            decoded.size,
            TestInstanceBlockSizeState { x: 3, y: 4, z: 5 }
        );
        assert_eq!(
            decoded.rotation,
            TestInstanceBlockRotation::Counterclockwise90
        );
        assert!(decoded.ignore_entities);
        assert_eq!(decoded.status, TestInstanceBlockRunStatus::Finished);
        assert_eq!(decoded.error_message.as_ref().unwrap().text(), "failed");

        assert_eq!(
            decode_test_instance_block_entity_data(&[0]).unwrap(),
            TestInstanceBlockEntityDataState::default()
        );
        assert!(
            decode_test_instance_block_entity_data(&test_instance_nbt(false))
                .unwrap_err()
                .to_string()
                .contains("missing status")
        );
    }

    #[test]
    fn editor_requires_permission_scopes_rich_status_and_rejects_bad_updates_atomically() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = test_instance_world(pos);
        assert!(!store.try_open_test_instance_block_editor(pos));

        store.local_player_id = Some(9);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 9,
            event_id: 26,
        }));
        assert!(store.try_open_test_instance_block_editor(pos));
        assert_eq!(store.test_instance_block_editor_open_count(), 1);
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
                raw_nbt: test_instance_nbt(true),
            })
            .unwrap());
        assert_eq!(
            store
                .test_instance_block_editor()
                .unwrap()
                .authoritative_revision,
            1
        );

        let status = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Ready".to_string()),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("green".to_string()),
            },
        ]))
        .unwrap();
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: status.clone(),
            size: Some(Vec3i { x: 8, y: 9, z: 10 }),
        });
        let editor = store.test_instance_block_editor().unwrap();
        assert_eq!(editor.response_status.as_ref(), Some(&status));
        assert_eq!(
            editor.response_size,
            Some(TestInstanceBlockSizeState { x: 8, y: 9, z: 10 })
        );
        assert_eq!(
            editor.data.size,
            TestInstanceBlockSizeState { x: 8, y: 9, z: 10 }
        );
        assert_eq!(editor.response_revision, 1);
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: RichTextComponent::plain("Still ready"),
            size: None,
        });
        let editor = store.test_instance_block_editor().unwrap();
        assert_eq!(
            editor.data.size,
            TestInstanceBlockSizeState { x: 8, y: 9, z: 10 }
        );
        assert_eq!(editor.response_revision, 2);
        assert_eq!(editor.response_size, None);

        let original = store.test_instance_block_data_at(pos).cloned().unwrap();
        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
                raw_nbt: test_instance_nbt(false),
            })
            .is_err());
        assert_eq!(store.test_instance_block_data_at(pos), Some(&original));

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.test_instance_block_editor().is_none());

        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: RichTextComponent::plain("late"),
            size: None,
        });
        assert!(store.test_instance_block_editor().is_none());
        assert_eq!(
            store
                .last_test_instance_block_status()
                .unwrap()
                .status
                .text(),
            "late"
        );
    }

    #[test]
    fn forgetting_test_instance_editor_chunk_closes_editor() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = test_instance_world(pos);
        store.local_player_id = Some(9);
        store.apply_entity_event(EntityEvent {
            entity_id: 9,
            event_id: 26,
        });
        assert!(store.try_open_test_instance_block_editor(pos));
        assert!(store.forget_chunk(ChunkPos { x: 0, z: 0 }));
        assert!(store.test_instance_block_editor().is_none());
    }

    fn test_instance_world(pos: BlockPos) -> WorldStore {
        let block_state_id =
            crate::vanilla_block_state_id("minecraft:test_instance_block", &BTreeMap::new())
                .unwrap();
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        );
        record.raw_nbt = test_instance_nbt(true);
        record.test_instance_block = Some(
            decode_test_instance_block_entity_data(&record.raw_nbt)
                .expect("valid test-instance fixture"),
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

    fn test_instance_nbt(include_status: bool) -> Vec<u8> {
        let mut out = vec![10];
        push_tag_name(&mut out, 10, "data");
        push_string(&mut out, "test", "example:rotation_test");
        push_tag_name(&mut out, 11, "size");
        out.extend_from_slice(&3_i32.to_be_bytes());
        for value in [3_i32, 4, 5] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        push_string(&mut out, "rotation", "counterclockwise_90");
        push_byte(&mut out, "ignore_entities", 1);
        if include_status {
            push_string(&mut out, "status", "finished");
        }
        push_string(&mut out, "error_message", "failed");
        out.push(0);
        out.push(0);
        out
    }

    fn push_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        push_tag_name(out, 1, name);
        out.push(value as u8);
    }

    fn push_string(out: &mut Vec<u8>, name: &str, value: &str) {
        push_tag_name(out, 8, name);
        push_nbt_string(out, value);
    }

    fn push_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        push_nbt_string(out, name);
    }

    fn push_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
