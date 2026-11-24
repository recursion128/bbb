pub use bbb_protocol::packets::Vec3i as TestInstanceBlockSizeState;
use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID, codec::ProtocolError,
    decode_nbt_summary, normalize_resource_location, packets::TestInstanceBlockRotation,
    NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use bbb_registry_codec::registry_codec::game_tests::{TestInstanceView, TestRotation};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

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
        Some(NbtSummaryValue::String(test)) => Some(
            test.try_to_string()
                .map_err(|_| {
                    bbb_protocol::codec::ProtocolError::InvalidData(
                        "invalid test identifier UTF-16".to_string(),
                    )
                })
                .and_then(normalize_resource_location)
                .map_err(crate::WorldDecodeError::from)?,
        ),
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
        Some(NbtSummaryValue::String(rotation)) => {
            match rotation.try_to_string().ok().as_deref().unwrap_or("") {
                "none" => TestInstanceBlockRotation::None,
                "clockwise_90" => TestInstanceBlockRotation::Clockwise90,
                "180" => TestInstanceBlockRotation::Clockwise180,
                "counterclockwise_90" => TestInstanceBlockRotation::Counterclockwise90,
                other => return invalid(format!("invalid test-instance block rotation `{other}`")),
            }
        }
        Some(_) => return invalid("test-instance block entity rotation is not a string"),
        None => return invalid("test-instance block entity data is missing rotation"),
    };
    let ignore_entities = match entry(data, "ignore_entities") {
        Some(NbtSummaryValue::Byte(value)) => *value != 0,
        Some(_) => return invalid("test-instance block entity ignore_entities is not a byte"),
        None => return invalid("test-instance block entity data is missing ignore_entities"),
    };
    let status = match entry(data, "status") {
        Some(NbtSummaryValue::String(status)) => {
            match status.try_to_string().ok().as_deref().unwrap_or("") {
                "cleared" => TestInstanceBlockRunStatus::Cleared,
                "running" => TestInstanceBlockRunStatus::Running,
                "finished" => TestInstanceBlockRunStatus::Finished,
                other => return invalid(format!("invalid test-instance block status `{other}`")),
            }
        }
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
        self.test_instance_definition(data)
            .map(|definition| definition.padding)
            .unwrap_or(0)
            .clamp(0, 128)
    }

    /// `TestInstanceBlockEntity.getRotation`: the definition's rotation composed
    /// with the block entity's own, defaulting to `Rotation.NONE`.
    pub(crate) fn test_instance_rotation(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> TestInstanceBlockRotation {
        let definition = match self
            .test_instance_definition(data)
            .map(|definition| definition.rotation)
        {
            Some(TestRotation::Clockwise90) => TestInstanceBlockRotation::Clockwise90,
            Some(TestRotation::Clockwise180) => TestInstanceBlockRotation::Clockwise180,
            Some(TestRotation::Counterclockwise90) => TestInstanceBlockRotation::Counterclockwise90,
            Some(TestRotation::None) | None => TestInstanceBlockRotation::None,
        };
        rotated_by(definition, data.rotation)
    }

    /// `TestInstanceRenderState.errorMarkers` for every loaded test-instance
    /// block, in block order.
    pub fn test_instance_error_marker_source_states(
        &self,
    ) -> Vec<(BlockPos, &TestInstanceErrorMarkerState)> {
        let mut markers = Vec::new();
        for (pos, record) in self
            .chunks
            .block_entities_of_type(VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID)
        {
            let Some(data) = record.test_instance_block() else {
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
        for (pos, record) in self
            .chunks
            .block_entities_of_type(VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID)
        {
            let Some(data) = record.test_instance_block() else {
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
    /// missing value both default to `true`.
    pub(crate) fn test_instance_is_required(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> bool {
        self.test_instance_definition(data)
            .is_none_or(|definition| definition.required)
    }

    fn test_instance_definition(
        &self,
        data: &TestInstanceBlockEntityDataState,
    ) -> Option<TestInstanceView<'_>> {
        let test = data.test.as_deref()?;
        self.frozen_registry_access().typed_test_instance(test)
    }

    pub fn test_instance_block_data_at(
        &self,
        pos: BlockPos,
    ) -> Option<&TestInstanceBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID)
            .then_some(record.test_instance_block())
            .flatten()
    }

    /// `TestInstanceBlockEditScreen`'s block, when the local player may edit it.
    pub fn test_instance_block_editor_at(
        &self,
        pos: BlockPos,
    ) -> Option<TestInstanceBlockEditorState> {
        if !self.local_player_can_use_gamemaster_blocks() || !self.test_instance_block_is_valid(pos)
        {
            return None;
        }
        Some(TestInstanceBlockEditorState {
            pos,
            data: self.test_instance_block_data_at(pos)?.clone(),
        })
    }

    fn test_instance_block_is_valid(&self, pos: BlockPos) -> bool {
        self.block_view(pos)
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
    fn editor_requires_permission_queues_rich_status_and_rejects_bad_updates_atomically() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = test_instance_world(pos);
        assert!(store.test_instance_block_editor_at(pos).is_none());

        grant_gamemaster_block_access(&mut store);
        assert!(store.test_instance_block_editor_at(pos).is_some());

        let status = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("Ready"),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("green"),
            },
        ]))
        .unwrap();
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: status.clone(),
            size: Some(Vec3i { x: 8, y: 9, z: 10 }),
        });
        let Some(crate::ScreenRequest::TestInstanceBlockStatus(requested)) =
            store.screen_requests().last()
        else {
            panic!("the status goes to the open screen");
        };
        assert_eq!(requested.status, status);
        assert_eq!(
            requested.size,
            Some(crate::DebugVec3iState { x: 8, y: 9, z: 10 })
        );

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
        assert!(store.test_instance_block_editor_at(pos).is_none());
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
        std::sync::Arc::make_mut(&mut world.chunks[0].block_entities)[0].payload =
            crate::BlockEntityPayload::TestInstanceBlock(Box::new(data));
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
        record.payload = crate::BlockEntityPayload::TestInstanceBlock(Box::new(
            decode_test_instance_block_entity_data(&record.raw_nbt)
                .expect("valid test-instance fixture"),
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
