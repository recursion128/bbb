//! End gateway persistent state and server cooldown lifecycle.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::value_input::{boolean_or, long_or};
use super::{entry, unique_field};

// TheEndGatewayBlockEntity.java:31-34; source-locked by the lifecycle test.
const SPAWN_TIME: i64 = 200;
const COOLDOWN_TIME: i32 = 40;
const ATTENTION_INTERVAL: i64 = 2400;
const EVENT_COOLDOWN: i32 = 1;

const TAG_AGE: &str = "Age";
const TAG_EXIT_PORTAL: &str = "exit_portal";
const TAG_EXACT_TELEPORT: &str = "ExactTeleport";

/// `TheEndGatewayBlockEntity` (`:29-45`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TheEndGatewayBlockEntity {
    age: i64,
    exit_portal: Option<BlockPos>,
    exact_teleport: bool,
    teleport_cooldown: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GatewayTickEffects {
    pub changed: bool,
    pub block_event: bool,
}

impl TheEndGatewayBlockEntity {
    #[must_use]
    pub const fn age(&self) -> i64 {
        self.age
    }

    #[must_use]
    pub const fn exit_portal(&self) -> Option<BlockPos> {
        self.exit_portal
    }

    #[must_use]
    pub const fn exact_teleport(&self) -> bool {
        self.exact_teleport
    }

    #[must_use]
    pub const fn is_spawning(&self) -> bool {
        self.age < SPAWN_TIME
    }

    #[must_use]
    pub const fn is_cooling_down(&self) -> bool {
        self.teleport_cooldown > 0
    }

    #[must_use]
    pub const fn teleport_cooldown(&self) -> i32 {
        self.teleport_cooldown
    }

    pub fn set_exit_position(&mut self, pos: BlockPos, exact: bool) {
        self.exit_portal = Some(pos);
        self.exact_teleport = exact;
    }

    pub fn trigger_cooldown(&mut self) {
        self.teleport_cooldown = COOLDOWN_TIME;
    }

    // TheEndGatewayBlockEntity.java:121-127; event data is ignored.
    #[must_use]
    pub fn trigger_event(&mut self, event: i32, _data: i32) -> bool {
        if event == EVENT_COOLDOWN {
            self.trigger_cooldown();
            true
        } else {
            false
        }
    }

    // TheEndGatewayBlockEntity.java:72-84; the world applies changed/event effects.
    #[must_use]
    pub fn tick(&mut self) -> GatewayTickEffects {
        let spawning = self.is_spawning();
        let cooling_down = self.is_cooling_down();
        self.age = self.age.wrapping_add(1);
        let mut block_event = false;
        if cooling_down {
            self.teleport_cooldown -= 1;
        } else if self.age % ATTENTION_INTERVAL == 0 {
            self.trigger_cooldown();
            block_event = true;
        }
        GatewayTickEffects {
            changed: spawning != self.is_spawning() || cooling_down != self.is_cooling_down(),
            block_event,
        }
    }

    /// `TheEndGatewayBlockEntity.saveAdditional` (`:48-55`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = vec![entry(TAG_AGE, NbtSummaryValue::Long(self.age))];
        if let Some(exit) = self.exit_portal {
            entries.push(entry(TAG_EXIT_PORTAL, save_block_pos(exit)));
        }
        if self.exact_teleport {
            entries.push(entry(TAG_EXACT_TELEPORT, NbtSummaryValue::Byte(1)));
        }
        entries
    }

    /// `TheEndGatewayBlockEntity.loadAdditional` (`:58-63`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.age = long_or(record, TAG_AGE, 0);
        self.exit_portal = unique_field(record, TAG_EXIT_PORTAL)
            .and_then(load_block_pos)
            .filter(is_in_spawnable_bounds);
        self.exact_teleport = boolean_or(record, TAG_EXACT_TELEPORT, false);
    }
}

/// `BlockPos.CODEC` (`BlockPos.java:34-38`): an int array of x, y, z.
fn save_block_pos(pos: BlockPos) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z])
}

fn load_block_pos(value: &NbtSummaryValue) -> Option<BlockPos> {
    match value {
        NbtSummaryValue::IntArray(values) if values.len() == 3 => Some(BlockPos {
            x: values[0],
            y: values[1],
            z: values[2],
        }),
        _ => None,
    }
}

/// `Level.isInSpawnableBounds` (`Level.java:175-177`, `:189-191`).
fn is_in_spawnable_bounds(pos: &BlockPos) -> bool {
    pos.y >= -20_000_000
        && pos.y < 20_000_000
        && pos.x >= -30_000_000
        && pos.z >= -30_000_000
        && pos.x < 30_000_000
        && pos.z < 30_000_000
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::block_entity::{new_block_entity, unique_field, BlockEntityPayload};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 8, y: 9, z: 10 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_END_GATEWAY_ID)
            .expect("minecraft:end_gateway has a default state")
    }

    fn upsert(record: &mut Vec<NbtSummaryEntry>, extra: Vec<NbtSummaryEntry>) {
        for field in extra {
            if let Some(existing) = record.iter_mut().find(|entry| entry.name == field.name) {
                existing.value = field.value;
            } else {
                record.push(field);
            }
        }
    }

    #[test]
    fn end_gateway_save_load_round_trips_age_exit_and_exact() {
        let gateway = TheEndGatewayBlockEntity {
            age: 2400,
            exit_portal: Some(BlockPos {
                x: 100,
                y: 50,
                z: -20,
            }),
            exact_teleport: true,
            teleport_cooldown: COOLDOWN_TIME,
        };
        let mut record = new_block_entity(pos(), default_state())
            .expect("an end gateway builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        upsert(&mut record, gateway.save_additional());
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names an end gateway");
        let BlockEntityPayload::EndGateway(loaded) = loaded.payload() else {
            panic!("payload is an end gateway");
        };
        assert_eq!(loaded.age(), 2400);
        assert_eq!(loaded.teleport_cooldown(), 0);
        assert_eq!(
            loaded.exit_portal(),
            Some(BlockPos {
                x: 100,
                y: 50,
                z: -20,
            })
        );
        assert!(loaded.exact_teleport());
        let saved = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("reload")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(
            unique_field(&saved, TAG_AGE),
            Some(&NbtSummaryValue::Long(2400))
        );
        assert_eq!(
            unique_field(&saved, TAG_EXIT_PORTAL),
            Some(&NbtSummaryValue::IntArray(vec![100, 50, -20]))
        );
        assert_eq!(
            unique_field(&saved, TAG_EXACT_TELEPORT),
            Some(&NbtSummaryValue::Byte(1))
        );
        let again = crate::block_entity::load_test_fixture(pos(), default_state(), &saved)
            .expect("fixture codec succeeds")
            .expect("the round-trip still names an end gateway");
        let BlockEntityPayload::EndGateway(again) = again.payload() else {
            panic!("payload is an end gateway");
        };
        assert_eq!(again.age(), 2400);
        assert!(!again.is_cooling_down());
        assert!(again.exact_teleport());
    }

    #[test]
    fn end_gateway_lifecycle_ends_spawning_and_runs_periodic_cooldown() {
        let mut gateway = TheEndGatewayBlockEntity {
            age: SPAWN_TIME - 1,
            ..Default::default()
        };
        assert!(gateway.is_spawning());
        assert_eq!(
            gateway.tick(),
            GatewayTickEffects {
                changed: true,
                block_event: false
            }
        );
        assert!(!gateway.is_spawning());
        assert_eq!(gateway.tick(), GatewayTickEffects::default());

        gateway.age = ATTENTION_INTERVAL - 1;
        assert_eq!(
            gateway.tick(),
            GatewayTickEffects {
                changed: true,
                block_event: true
            }
        );
        assert_eq!(gateway.teleport_cooldown(), COOLDOWN_TIME);
        assert!(gateway.is_cooling_down());
        for _ in 1..COOLDOWN_TIME {
            assert_eq!(gateway.tick(), GatewayTickEffects::default());
        }
        assert_eq!(gateway.teleport_cooldown(), 1);
        assert_eq!(
            gateway.tick(),
            GatewayTickEffects {
                changed: true,
                block_event: false
            }
        );
        assert_eq!(gateway.teleport_cooldown(), 0);
        assert!(!gateway.is_cooling_down());
    }
}
