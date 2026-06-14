use bbb_protocol::packets::{
    DebugBlockValue as ProtocolDebugBlockValue, DebugChunkValue as ProtocolDebugChunkValue,
    DebugEntityValue as ProtocolDebugEntityValue, DebugEvent as ProtocolDebugEvent,
    DebugSample as ProtocolDebugSample, GameRuleValues as ProtocolGameRuleValues,
    GameTestHighlightPos as ProtocolGameTestHighlightPos,
    TestInstanceBlockStatus as ProtocolTestInstanceBlockStatus, Vec3i as ProtocolVec3i,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, ChunkPos, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDebugGameState {
    #[serde(default)]
    pub last_debug_block_value: Option<DebugBlockValueState>,
    #[serde(default)]
    pub last_debug_chunk_value: Option<DebugChunkValueState>,
    #[serde(default)]
    pub last_debug_entity_value: Option<DebugEntityValueState>,
    #[serde(default)]
    pub last_debug_event: Option<DebugEventState>,
    #[serde(default)]
    pub last_debug_sample: Option<DebugSampleState>,
    #[serde(default)]
    pub last_game_rule_values: Option<GameRuleValuesState>,
    #[serde(default)]
    pub last_game_test_highlight_pos: Option<GameTestHighlightPosState>,
    #[serde(default)]
    pub last_test_instance_block_status: Option<TestInstanceBlockStatusState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBlockValueState {
    pub pos: BlockPos,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugChunkValueState {
    pub pos: ChunkPos,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEntityValueState {
    pub entity_id: i32,
    pub raw_update_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEventState {
    pub raw_event_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSampleState {
    pub sample_len: usize,
    pub sample_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValuesState {
    #[serde(default)]
    pub values: Vec<GameRuleValueState>,
}

impl GameRuleValuesState {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRuleValueState {
    pub rule: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightPosState {
    pub absolute_pos: BlockPos,
    pub relative_pos: BlockPos,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockStatusState {
    pub status: String,
    pub size: Option<DebugVec3iState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugVec3iState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl WorldStore {
    pub fn apply_debug_block_value(&mut self, packet: ProtocolDebugBlockValue) {
        self.counters.debug_block_value_packets += 1;
        self.client_debug_game.last_debug_block_value = Some(DebugBlockValueState {
            pos: protocol_block_pos(packet.pos),
            raw_update_payload_len: packet.raw_update_payload.len(),
        });
    }

    pub fn apply_debug_chunk_value(&mut self, packet: ProtocolDebugChunkValue) {
        self.counters.debug_chunk_value_packets += 1;
        self.client_debug_game.last_debug_chunk_value = Some(DebugChunkValueState {
            pos: ChunkPos {
                x: packet.pos.x,
                z: packet.pos.z,
            },
            raw_update_payload_len: packet.raw_update_payload.len(),
        });
    }

    pub fn apply_debug_entity_value(&mut self, packet: ProtocolDebugEntityValue) {
        self.counters.debug_entity_value_packets += 1;
        self.client_debug_game.last_debug_entity_value = Some(DebugEntityValueState {
            entity_id: packet.entity_id,
            raw_update_payload_len: packet.raw_update_payload.len(),
        });
    }

    pub fn apply_debug_event(&mut self, packet: ProtocolDebugEvent) {
        self.counters.debug_event_packets += 1;
        self.client_debug_game.last_debug_event = Some(DebugEventState {
            raw_event_payload_len: packet.raw_event_payload.len(),
        });
    }

    pub fn apply_debug_sample(&mut self, packet: ProtocolDebugSample) {
        self.counters.debug_sample_packets += 1;
        self.client_debug_game.last_debug_sample = Some(DebugSampleState {
            sample_len: packet.sample.len(),
            sample_type: packet.sample_type.as_str().to_string(),
        });
    }

    pub fn apply_game_rule_values(&mut self, packet: ProtocolGameRuleValues) {
        self.counters.game_rule_value_packets += 1;
        self.client_debug_game.last_game_rule_values = Some(GameRuleValuesState {
            values: packet
                .values
                .into_iter()
                .map(|value| GameRuleValueState {
                    rule: value.rule,
                    value: value.value,
                })
                .collect(),
        });
    }

    pub fn apply_game_test_highlight_pos(&mut self, packet: ProtocolGameTestHighlightPos) {
        self.counters.game_test_highlight_pos_packets += 1;
        self.client_debug_game.last_game_test_highlight_pos = Some(GameTestHighlightPosState {
            absolute_pos: protocol_block_pos(packet.absolute_pos),
            relative_pos: protocol_block_pos(packet.relative_pos),
        });
    }

    pub fn apply_test_instance_block_status(&mut self, packet: ProtocolTestInstanceBlockStatus) {
        self.counters.test_instance_block_status_packets += 1;
        self.client_debug_game.last_test_instance_block_status =
            Some(TestInstanceBlockStatusState {
                status: packet.status,
                size: packet.size.map(debug_vec3i_state),
            });
    }

    pub fn client_debug_game(&self) -> &ClientDebugGameState {
        &self.client_debug_game
    }

    pub fn last_debug_block_value(&self) -> Option<&DebugBlockValueState> {
        self.client_debug_game.last_debug_block_value.as_ref()
    }

    pub fn last_debug_chunk_value(&self) -> Option<&DebugChunkValueState> {
        self.client_debug_game.last_debug_chunk_value.as_ref()
    }

    pub fn last_debug_entity_value(&self) -> Option<&DebugEntityValueState> {
        self.client_debug_game.last_debug_entity_value.as_ref()
    }

    pub fn last_debug_event(&self) -> Option<&DebugEventState> {
        self.client_debug_game.last_debug_event.as_ref()
    }

    pub fn last_debug_sample(&self) -> Option<&DebugSampleState> {
        self.client_debug_game.last_debug_sample.as_ref()
    }

    pub fn last_game_rule_values(&self) -> Option<&GameRuleValuesState> {
        self.client_debug_game.last_game_rule_values.as_ref()
    }

    pub fn last_game_test_highlight_pos(&self) -> Option<&GameTestHighlightPosState> {
        self.client_debug_game.last_game_test_highlight_pos.as_ref()
    }

    pub fn last_test_instance_block_status(&self) -> Option<&TestInstanceBlockStatusState> {
        self.client_debug_game
            .last_test_instance_block_status
            .as_ref()
    }
}

fn debug_vec3i_state(pos: ProtocolVec3i) -> DebugVec3iState {
    DebugVec3iState {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, ChunkPos as ProtocolChunkPos, DebugBlockValue,
        DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample, GameRuleValue, GameRuleValues,
        GameTestHighlightPos, RemoteDebugSampleType, TestInstanceBlockStatus,
    };

    #[test]
    fn tracks_debug_game_events_and_counters() {
        let mut store = WorldStore::new();

        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        });
        store.apply_debug_chunk_value(DebugChunkValue {
            pos: ProtocolChunkPos { x: 3, z: -4 },
            raw_update_payload: vec![7, 0],
        });
        store.apply_debug_entity_value(DebugEntityValue {
            entity_id: 123,
            raw_update_payload: vec![9, 1, 0xbb],
        });
        store.apply_debug_event(DebugEvent {
            raw_event_payload: vec![4, 0xcc],
        });
        store.apply_debug_sample(DebugSample {
            sample: vec![100, -50],
            sample_type: RemoteDebugSampleType::TickTime,
        });
        store.apply_game_rule_values(GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        });
        store.apply_game_test_highlight_pos(GameTestHighlightPos {
            absolute_pos: ProtocolBlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        });
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        });

        assert_eq!(
            store.last_debug_block_value(),
            Some(&DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload_len: 3,
            })
        );
        assert_eq!(
            store.last_debug_chunk_value(),
            Some(&DebugChunkValueState {
                pos: ChunkPos { x: 3, z: -4 },
                raw_update_payload_len: 2,
            })
        );
        assert_eq!(
            store.last_debug_entity_value(),
            Some(&DebugEntityValueState {
                entity_id: 123,
                raw_update_payload_len: 3,
            })
        );
        assert_eq!(
            store.last_debug_event(),
            Some(&DebugEventState {
                raw_event_payload_len: 2,
            })
        );
        assert_eq!(
            store.last_debug_sample(),
            Some(&DebugSampleState {
                sample_len: 2,
                sample_type: "tick_time".to_string(),
            })
        );
        assert_eq!(
            store.last_game_rule_values(),
            Some(&GameRuleValuesState {
                values: vec![
                    GameRuleValueState {
                        rule: "minecraft:do_daylight_cycle".to_string(),
                        value: "false".to_string(),
                    },
                    GameRuleValueState {
                        rule: "minecraft:random_tick_speed".to_string(),
                        value: "3".to_string(),
                    },
                ],
            })
        );
        assert_eq!(store.last_game_rule_values().unwrap().len(), 2);
        assert_eq!(
            store.last_game_test_highlight_pos(),
            Some(&GameTestHighlightPosState {
                absolute_pos: BlockPos {
                    x: -10,
                    y: 70,
                    z: 22,
                },
                relative_pos: BlockPos { x: 1, y: 2, z: 3 },
            })
        );
        assert_eq!(
            store.last_test_instance_block_status(),
            Some(&TestInstanceBlockStatusState {
                status: "Ready".to_string(),
                size: Some(DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );

        let counters = store.counters();
        assert_eq!(counters.debug_block_value_packets, 1);
        assert_eq!(counters.debug_chunk_value_packets, 1);
        assert_eq!(counters.debug_entity_value_packets, 1);
        assert_eq!(counters.debug_event_packets, 1);
        assert_eq!(counters.debug_sample_packets, 1);
        assert_eq!(counters.game_rule_value_packets, 1);
        assert_eq!(counters.game_test_highlight_pos_packets, 1);
        assert_eq!(counters.test_instance_block_status_packets, 1);
    }

    #[test]
    fn debug_game_state_survives_world_store_json_round_trip() {
        let mut store = WorldStore::new();

        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        });
        store.apply_game_rule_values(GameRuleValues {
            values: vec![GameRuleValue {
                rule: "minecraft:random_tick_speed".to_string(),
                value: "3".to_string(),
            }],
        });
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        });

        let json = serde_json::to_string(&store).unwrap();
        let restored: WorldStore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.counters().debug_block_value_packets, 1);
        assert_eq!(restored.counters().game_rule_value_packets, 1);
        assert_eq!(restored.counters().test_instance_block_status_packets, 1);
        assert_eq!(
            restored.last_debug_block_value(),
            Some(&DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload_len: 3,
            })
        );
        assert_eq!(
            restored.last_game_rule_values(),
            Some(&GameRuleValuesState {
                values: vec![GameRuleValueState {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                }],
            })
        );
        assert_eq!(
            restored.last_test_instance_block_status(),
            Some(&TestInstanceBlockStatusState {
                status: "Ready".to_string(),
                size: Some(DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );
    }
}
