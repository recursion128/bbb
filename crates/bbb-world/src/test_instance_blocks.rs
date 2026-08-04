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
    /// `TestInstanceBlockEntity.errorMarkers`, saved beside `data` rather than
    /// inside it. `TestInstanceRenderer` draws one marker per entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_markers: Vec<TestInstanceErrorMarkerState>,
}

/// One `TestInstanceBlockEntity.ErrorMarker`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceErrorMarkerState {
    pub pos: BlockPos,
    pub text: RichTextComponent,
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
            error_markers: Vec::new(),
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

/// `TestInstanceBlockEntity.BEAM_RUNNING`.
static BEAM_RUNNING: crate::BeaconBeamSection = beam_section(0xFF80_8080);
/// `TestInstanceBlockEntity.BEAM_SUCCESS`.
static BEAM_SUCCESS: crate::BeaconBeamSection = beam_section(0xFF00_FF00);
/// `TestInstanceBlockEntity.BEAM_REQUIRED_FAILED`.
static BEAM_REQUIRED_FAILED: crate::BeaconBeamSection = beam_section(0xFFFF_0000);
/// `TestInstanceBlockEntity.BEAM_OPTIONAL_FAILED`.
static BEAM_OPTIONAL_FAILED: crate::BeaconBeamSection = beam_section(0xFFFF_8000);

/// `new BeaconBeamOwner.Section(color)` starts one block tall.
const fn beam_section(color_argb: u32) -> crate::BeaconBeamSection {
    crate::BeaconBeamSection {
        color_argb,
        height: 1,
    }
}

/// `Rotation.getRotated`: both rotations compose as quarter turns clockwise.
fn rotated_by(
    rotation: TestInstanceBlockRotation,
    by: TestInstanceBlockRotation,
) -> TestInstanceBlockRotation {
    const ORDER: [TestInstanceBlockRotation; 4] = [
        TestInstanceBlockRotation::None,
        TestInstanceBlockRotation::Clockwise90,
        TestInstanceBlockRotation::Clockwise180,
        TestInstanceBlockRotation::Counterclockwise90,
    ];
    let quarter = |value: TestInstanceBlockRotation| {
        ORDER
            .iter()
            .position(|candidate| *candidate == value)
            .unwrap_or(0)
    };
    ORDER[(quarter(rotation) + quarter(by)) % ORDER.len()]
}

fn definition_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        _ => None,
    }
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
    let error_markers = decode_error_markers(&root)?;
    let Some(data) = entry(&root, "data") else {
        return Ok(TestInstanceBlockEntityDataState {
            error_markers,
            ..TestInstanceBlockEntityDataState::default()
        });
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
        error_markers,
    })
}

/// `ErrorMarker.LIST_CODEC`: a list of `{pos: BlockPos, text: Component}`.
/// A malformed entry is dropped rather than failing the whole block entity —
/// vanilla's `input.read(...).orElse(List.of())` treats the field as optional.
fn decode_error_markers(root: &[NbtSummaryEntry]) -> Result<Vec<TestInstanceErrorMarkerState>> {
    let Some(NbtSummaryValue::List(markers)) = entry(root, "errors") else {
        return Ok(Vec::new());
    };
    let mut decoded = Vec::new();
    for marker in markers {
        let NbtSummaryValue::Compound(marker) = marker else {
            continue;
        };
        let Some(NbtSummaryValue::IntArray(pos)) = entry(marker, "pos") else {
            continue;
        };
        if pos.len() != 3 {
            continue;
        }
        let Some(text) = entry(marker, "text") else {
            continue;
        };
        let text = RichTextComponent::try_from_component_nbt(text.clone()).map_err(|reason| {
            crate::WorldDecodeError::from(ProtocolError::InvalidData(format!(
                "invalid test-instance block error marker text: {reason}"
            )))
        })?;
        decoded.push(TestInstanceErrorMarkerState {
            pos: BlockPos {
                x: pos[0],
                y: pos[1],
                z: pos[2],
            },
            text,
        });
    }
    Ok(decoded)
}

impl WorldStore {
    /// `TestInstanceBlockEntity.getPadding`: the `GameTestInstance`'s padding,
    /// or `0` when the test key resolves to nothing on this client.
    pub(crate) fn test_instance_padding(&self, data: &TestInstanceBlockEntityDataState) -> i32 {
        self.test_instance_definition_field(data, "padding")
            .and_then(definition_i32)
            .unwrap_or(0)
            .clamp(0, 128)
    }

    /// `TestInstanceBlockEntity.getRotation`: the definition's rotation composed
    /// with the block entity's own, defaulting to `Rotation.NONE`.
    pub(crate) fn test_instance_rotation(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> TestInstanceBlockRotation {
        let definition = match self.test_instance_definition_field(data, "rotation") {
            Some(NbtSummaryValue::String(rotation)) => match rotation.as_str() {
                "clockwise_90" => TestInstanceBlockRotation::Clockwise90,
                "180" => TestInstanceBlockRotation::Clockwise180,
                "counterclockwise_90" => TestInstanceBlockRotation::Counterclockwise90,
                _ => TestInstanceBlockRotation::None,
            },
            _ => TestInstanceBlockRotation::None,
        };
        rotated_by(definition, data.rotation)
    }

    /// `TestInstanceRenderState.errorMarkers` for every loaded test-instance
    /// block, in block order.
    pub fn test_instance_error_marker_source_states(
        &self,
    ) -> Vec<(BlockPos, &TestInstanceErrorMarkerState)> {
        let mut markers = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID {
                continue;
            }
            let Some(data) = record.test_instance_block.as_ref() else {
                continue;
            };
            markers.extend(data.error_markers.iter().map(|marker| (pos, marker)));
        }
        markers.sort_by_key(|(pos, marker)| (*pos, marker.pos));
        markers
    }

    /// `TestInstanceBlockEntity.getBeamSections` for every loaded test-instance
    /// block. Vanilla publishes at most one section per block.
    pub(crate) fn test_instance_beam_sections(
        &self,
    ) -> Vec<(BlockPos, &'static crate::BeaconBeamSection)> {
        let mut sections = Vec::new();
        for (pos, record) in self.loaded_block_entity_records() {
            if record.type_id != VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID {
                continue;
            }
            let Some(data) = record.test_instance_block.as_ref() else {
                continue;
            };
            let section = match data.status {
                TestInstanceBlockRunStatus::Cleared => continue,
                TestInstanceBlockRunStatus::Running => &BEAM_RUNNING,
                TestInstanceBlockRunStatus::Finished if data.error_message.is_none() => {
                    &BEAM_SUCCESS
                }
                TestInstanceBlockRunStatus::Finished if self.test_instance_is_required(data) => {
                    &BEAM_REQUIRED_FAILED
                }
                TestInstanceBlockRunStatus::Finished => &BEAM_OPTIONAL_FAILED,
            };
            sections.push((pos, section));
        }
        sections.sort_by_key(|(pos, _)| *pos);
        sections
    }

    /// `GameTestInstance.required`, whose codec and vanilla's missing-holder
    /// fallback both default to `true`.
    pub(crate) fn test_instance_is_required(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> bool {
        match self.test_instance_definition_field(data, "required") {
            Some(NbtSummaryValue::Byte(value)) => *value != 0,
            _ => true,
        }
    }

    /// One field of the block's `minecraft:test_instance` registry element.
    /// `GameTestInstance.DIRECT_CODEC` dispatches on `type`, so `TestData`'s
    /// fields sit at the element's top level.
    fn test_instance_definition_field(
        &self,
        data: &TestInstanceBlockEntityDataState,
        name: &str,
    ) -> Option<&NbtSummaryValue> {
        let test = data.test.as_deref()?;
        let entry = self
            .frozen_registry_access()
            .entry_by_resource_id("minecraft:test_instance", test)?;
        let NbtSummaryValue::Compound(entries) = &entry.element_data?.value else {
            return None;
        };
        self::entry(entries, name)
    }

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
        if !self.local_player_can_use_gamemaster_blocks() || !self.test_instance_block_is_valid(pos)
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
        assert_eq!(decoded.error_markers.len(), 1);
        assert_eq!(decoded.error_markers[0].pos, BlockPos { x: 1, y: 2, z: 3 });
        assert_eq!(decoded.error_markers[0].text.text(), "boom");

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

        grant_gamemaster_block_access(&mut store);
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
        grant_gamemaster_block_access(&mut store);
        assert!(store.try_open_test_instance_block_editor(pos));
        assert!(store.forget_chunk(ChunkPos { x: 0, z: 0 }));
        assert!(store.test_instance_block_editor().is_none());
    }

    #[test]
    fn beam_sections_and_boxes_follow_the_run_status_and_definition_defaults() {
        let pos = BlockPos { x: 3, y: 2, z: 5 };
        let world = test_instance_world(pos);
        let data = world
            .test_instance_block_data_at(pos)
            .expect("test instance data")
            .clone();

        // The fixture finished with an error message; without a synced
        // `minecraft:test_instance` entry `required()` falls back to true.
        assert_eq!(data.status, TestInstanceBlockRunStatus::Finished);
        assert!(world.test_instance_is_required(&data));
        assert_eq!(
            world.test_instance_beam_sections(),
            vec![(pos, &BEAM_REQUIRED_FAILED)]
        );
        // One `submitBeaconBeam` per section, drawn to the render limit.
        let beams = world.beacon_beam_source_states(0.0);
        assert_eq!(beams.len(), 1);
        assert_eq!(beams[0].pos, pos);
        assert_eq!(beams[0].beam_start, 0);
        assert_eq!(beams[0].height, crate::BEACON_MAX_RENDER_Y);
        assert_eq!(beams[0].color_argb, 0xFFFF_0000);

        // Without a definition the padding is zero and the block's own rotation
        // is the whole transform, so `counterclockwise_90` swaps X and Z.
        assert_eq!(world.test_instance_padding(&data), 0);
        assert_eq!(
            world.test_instance_rotation(&data),
            TestInstanceBlockRotation::Counterclockwise90
        );
    }

    #[test]
    fn cleared_test_instances_publish_no_beam() {
        let pos = BlockPos { x: 3, y: 2, z: 5 };
        let mut world = test_instance_world(pos);
        let data = TestInstanceBlockEntityDataState {
            status: TestInstanceBlockRunStatus::Cleared,
            error_message: None,
            ..world
                .test_instance_block_data_at(pos)
                .expect("test instance data")
                .clone()
        };
        world.chunks[0].block_entities[0].test_instance_block = Some(data);
        assert!(world.test_instance_beam_sections().is_empty());
        assert!(world.beacon_beam_source_states(0.0).is_empty());
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

    fn grant_gamemaster_block_access(store: &mut WorldStore) {
        store.local_player_id = Some(9);
        assert!(store.apply_entity_event(EntityEvent {
            entity_id: 9,
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
        push_tag_name(&mut out, 9, "errors");
        out.push(10);
        out.extend_from_slice(&2_i32.to_be_bytes());
        push_error_marker(&mut out, [1, 2, 3], "boom");
        // A marker without a position is dropped, not a decode failure.
        push_string(&mut out, "text", "orphan");
        out.push(0);
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

    fn push_error_marker(out: &mut Vec<u8>, pos: [i32; 3], text: &str) {
        push_tag_name(out, 11, "pos");
        out.extend_from_slice(&3_i32.to_be_bytes());
        for value in pos {
            out.extend_from_slice(&value.to_be_bytes());
        }
        push_string(out, "text", text);
        out.push(0);
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
