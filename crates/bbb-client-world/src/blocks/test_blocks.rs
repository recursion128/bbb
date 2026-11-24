use bbb_protocol::{
    block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID, decode_nbt_summary,
    packets::TestBlockMode, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, Result, WorldStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestBlockEntityDataState {
    pub mode: TestBlockMode,
    pub message: bbb_protocol::JavaString,
    pub powered: bool,
}

impl Default for TestBlockEntityDataState {
    fn default() -> Self {
        Self {
            mode: TestBlockMode::Fail,
            message: Default::default(),
            powered: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestBlockEditorState {
    pub pos: BlockPos,
    pub data: TestBlockEntityDataState,
}

pub(crate) fn decode_test_block_entity_data(raw_nbt: &[u8]) -> Result<TestBlockEntityDataState> {
    let Some(root) = decode_nbt_summary(raw_nbt)? else {
        return Ok(TestBlockEntityDataState::default());
    };
    let NbtSummaryValue::Compound(entries) = root else {
        return Err(bbb_protocol::codec::ProtocolError::InvalidData(
            "test block entity root is not a compound".to_string(),
        )
        .into());
    };

    Ok(TestBlockEntityDataState {
        mode: match string(&entries, "mode")
            .and_then(|value| value.try_to_string().ok())
            .as_deref()
        {
            Some("start") => TestBlockMode::Start,
            Some("log") => TestBlockMode::Log,
            Some("accept") => TestBlockMode::Accept,
            _ => TestBlockMode::Fail,
        },
        message: string(&entries, "message").unwrap_or_default(),
        powered: byte(&entries, "powered").is_some_and(|value| value != 0),
    })
}

impl WorldStore {
    pub fn test_block_data_at(&self, pos: BlockPos) -> Option<&TestBlockEntityDataState> {
        let record = self.block_entity_record_at(pos)?;
        (record.type_id == VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID)
            .then_some(record.test_block())
            .flatten()
    }

    /// `TestBlockEditScreen`'s block, when the local player may edit it.
    pub fn test_block_editor_at(&self, pos: BlockPos) -> Option<TestBlockEditorState> {
        if !self.local_player_can_use_gamemaster_blocks() || !self.test_block_is_present_at(pos) {
            return None;
        }
        Some(TestBlockEditorState {
            pos,
            data: self.test_block_data_at(pos)?.clone(),
        })
    }

    fn test_block_is_present_at(&self, pos: BlockPos) -> bool {
        self.block_view(pos)
            .and_then(|probe| probe.block_name)
            .is_some_and(|name| name == "minecraft:test_block")
    }
}

fn entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn string(entries: &[NbtSummaryEntry], key: &str) -> Option<bbb_protocol::JavaString> {
    let NbtSummaryValue::String(value) = entry(entries, key)? else {
        return None;
    };
    Some(value.clone())
}

fn byte(entries: &[NbtSummaryEntry], key: &str) -> Option<i8> {
    let NbtSummaryValue::Byte(value) = entry(entries, key)? else {
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
    fn decodes_modes_defaults_and_malformed_fields() {
        for (name, expected) in [
            ("start", TestBlockMode::Start),
            ("log", TestBlockMode::Log),
            ("fail", TestBlockMode::Fail),
            ("accept", TestBlockMode::Accept),
        ] {
            assert_eq!(
                decode_test_block_entity_data(&test_block_nbt(name, "message", true)).unwrap(),
                TestBlockEntityDataState {
                    mode: expected,
                    message: "message".into(),
                    powered: true,
                }
            );
        }

        assert_eq!(
            decode_test_block_entity_data(&[0]).unwrap(),
            TestBlockEntityDataState::default()
        );
        assert_eq!(
            decode_test_block_entity_data(&[10, 0]).unwrap(),
            TestBlockEntityDataState::default()
        );
        assert_eq!(
            decode_test_block_entity_data(&test_block_nbt("unknown", "retained", false)).unwrap(),
            TestBlockEntityDataState {
                mode: TestBlockMode::Fail,
                message: "retained".into(),
                powered: false,
            }
        );
        assert_eq!(
            decode_test_block_entity_data(&wrong_typed_test_block_nbt()).unwrap(),
            TestBlockEntityDataState::default()
        );
        assert!(decode_test_block_entity_data(&[8, 0, 1, b'x'])
            .unwrap_err()
            .to_string()
            .contains("root is not a compound"));
    }

    #[test]
    fn editor_requires_permission_and_world_tracks_authoritative_data() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut store = test_block_world(pos, TestBlockMode::Log, "initial", false);
        assert!(store.test_block_editor_at(pos).is_none());

        grant_gamemaster_block_access(&mut store);
        assert_eq!(
            store.test_block_editor_at(pos).unwrap().data.mode,
            TestBlockMode::Log
        );

        assert!(store
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
                raw_nbt: test_block_nbt("accept", "server ack", true),
            })
            .unwrap());
        assert_eq!(
            store.test_block_data_at(pos),
            Some(&TestBlockEntityDataState {
                mode: TestBlockMode::Accept,
                message: "server ack".into(),
                powered: true,
            })
        );
        let accept_state = crate::vanilla_block_state_id(
            "minecraft:test_block",
            &BTreeMap::from([("mode".to_string(), "accept".to_string())]),
        )
        .unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: accept_state,
        }));
        assert_eq!(
            store
                .probe_block(pos)
                .unwrap()
                .block_properties
                .get("mode")
                .map(String::as_str),
            Some("accept")
        );

        let stone = crate::vanilla_block_state_id("minecraft:stone", &BTreeMap::new()).unwrap();
        assert!(store.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert!(store.test_block_data_at(pos).is_none());
        assert!(store.test_block_editor_at(pos).is_none());
    }

    fn test_block_world(
        pos: BlockPos,
        mode: TestBlockMode,
        message: &str,
        powered: bool,
    ) -> WorldStore {
        let block_state_id = crate::vanilla_block_state_id(
            "minecraft:test_block",
            &BTreeMap::from([("mode".to_string(), mode.serialized_name().to_string())]),
        )
        .unwrap();
        let mut record = BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        );
        record.raw_nbt = test_block_nbt(mode.serialized_name(), message, powered);
        record.payload = crate::BlockEntityPayload::TestBlock(Box::new(
            decode_test_block_entity_data(&record.raw_nbt).expect("valid test block fixture"),
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

    fn test_block_nbt(mode: &str, message: &str, powered: bool) -> Vec<u8> {
        let mut out = vec![10];
        push_string(&mut out, "mode", mode);
        push_string(&mut out, "message", message);
        push_byte(&mut out, "powered", i8::from(powered));
        out.push(0);
        out
    }

    fn wrong_typed_test_block_nbt() -> Vec<u8> {
        let mut out = vec![10];
        push_tag_name(&mut out, 3, "mode");
        out.extend_from_slice(&1_i32.to_be_bytes());
        push_byte(&mut out, "message", 1);
        push_string(&mut out, "powered", "true");
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
