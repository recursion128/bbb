//! Conduit frame activation, ambient timing, and persistent target reference.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::{codec_uuid, entry, unique_field, uuid_int_array};

const TAG_TARGET: &str = "Target";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConduitBlockRead {
    pub water: bool,
    pub valid_frame: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConduitRefresh {
    pub refreshed: bool,
    pub active_change: Option<bool>,
    pub active: bool,
    pub hunting: bool,
    pub effect_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConduitSound {
    Ambient,
    AmbientShort,
}

/// `ConduitBlockEntity` (`:35-53`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConduitBlockEntity {
    destroy_target: Option<NbtSummaryValue>,
    tick_count: i32,
    active: bool,
    hunting: bool,
    effect_blocks: Vec<BlockPos>,
    next_ambient_sound_activation: i64,
}

impl ConduitBlockEntity {
    // ConduitBlockEntity.java:92-109; the world runs activation and effect actions.
    #[must_use]
    pub fn tick_shape(
        &mut self,
        pos: BlockPos,
        game_time: i64,
        mut read: impl FnMut(BlockPos) -> ConduitBlockRead,
    ) -> ConduitRefresh {
        self.tick_count = self.tick_count.wrapping_add(1);
        let refreshed = game_time % 40 == 0;
        let mut active_change = None;
        if refreshed {
            self.effect_blocks.clear();
            let active = self.update_shape(pos, &mut read);
            active_change = (active != self.active).then_some(active);
            self.active = active;
            self.hunting = self.effect_blocks.len() >= 42;
        }
        ConduitRefresh {
            refreshed,
            active_change,
            active: self.active,
            hunting: self.hunting,
            effect_size: self.effect_blocks.len(),
        }
    }

    // ConduitBlockEntity.java:127-162.
    fn update_shape(
        &mut self,
        pos: BlockPos,
        read: &mut impl FnMut(BlockPos) -> ConduitBlockRead,
    ) -> bool {
        for ox in -1..=1 {
            for oy in -1..=1 {
                for oz in -1..=1 {
                    if !read(offset(pos, ox, oy, oz)).water {
                        return false;
                    }
                }
            }
        }
        for ox in -2_i32..=2 {
            for oy in -2_i32..=2 {
                for oz in -2_i32..=2 {
                    let (ax, ay, az) = (ox.abs(), oy.abs(), oz.abs());
                    if (ax > 1 || ay > 1 || az > 1)
                        && (ox == 0 && (ay == 2 || az == 2)
                            || oy == 0 && (ax == 2 || az == 2)
                            || oz == 0 && (ax == 2 || ay == 2))
                    {
                        let check = offset(pos, ox, oy, oz);
                        if read(check).valid_frame {
                            self.effect_blocks.push(check);
                        }
                    }
                }
            }
        }
        self.effect_blocks.len() >= 16
    }

    /// Fast-path the 39/40 non-refresh ticks (`game_time % 40 != 0`) in place
    /// without taking or cloning the block entity (`ConduitBlockEntity.java:92-120`).
    #[must_use]
    pub(crate) fn tick_non_refresh(
        &mut self,
        game_time: i64,
        random: impl FnMut(i32) -> i32,
    ) -> [Option<ConduitSound>; 2] {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.ambient_tick_array(game_time, random)
    }

    // ConduitBlockEntity.java:111-120; ambient precedes the optional short sound.
    #[must_use]
    pub fn ambient_tick(
        &mut self,
        game_time: i64,
        random: impl FnMut(i32) -> i32,
    ) -> Vec<ConduitSound> {
        self.ambient_tick_array(game_time, random)
            .into_iter()
            .flatten()
            .collect()
    }

    fn ambient_tick_array(
        &mut self,
        game_time: i64,
        mut random: impl FnMut(i32) -> i32,
    ) -> [Option<ConduitSound>; 2] {
        let mut sounds = [None, None];
        if self.active {
            if game_time % 80 == 0 {
                sounds[0] = Some(ConduitSound::Ambient);
            }
            if game_time > self.next_ambient_sound_activation {
                self.next_ambient_sound_activation = game_time
                    .wrapping_add(60)
                    .wrapping_add(i64::from(random(40)));
                sounds[1] = Some(ConduitSound::AmbientShort);
            }
        }
        sounds
    }

    #[must_use]
    pub const fn tick_count(&self) -> i32 {
        self.tick_count
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub const fn is_hunting(&self) -> bool {
        self.hunting
    }

    #[must_use]
    pub fn effect_blocks(&self) -> &[BlockPos] {
        &self.effect_blocks
    }

    #[must_use]
    pub fn destroy_target_uuid(&self) -> Option<Uuid> {
        self.destroy_target.as_ref().and_then(codec_uuid)
    }

    // EntityReference.java:134-136: equality uses the UUID.
    pub fn set_destroy_target(&mut self, target: Option<Uuid>) -> bool {
        if target == self.destroy_target_uuid() {
            return false;
        }
        self.destroy_target = target.map(|uuid| NbtSummaryValue::IntArray(uuid_int_array(uuid)));
        true
    }

    #[must_use]
    pub fn destroy_target(&self) -> Option<&NbtSummaryValue> {
        self.destroy_target.as_ref()
    }

    /// `ConduitBlockEntity.saveAdditional` (`:62-65`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        match &self.destroy_target {
            Some(target) => vec![entry(TAG_TARGET, target.clone())],
            None => Vec::new(),
        }
    }

    /// `ConduitBlockEntity.loadAdditional` (`:56-59`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.destroy_target = unique_field(record, TAG_TARGET)
            .and_then(codec_uuid)
            .map(|uuid| NbtSummaryValue::IntArray(uuid_int_array(uuid)));
    }
}

fn offset(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x.wrapping_add(x),
        y: pos.y.wrapping_add(y),
        z: pos.z.wrapping_add(z),
    }
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_CONDUIT_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::NbtSummaryValue;

    use crate::block_entity::{entry, new_block_entity, unique_field, BlockEntityPayload};

    use super::*;

    fn pos() -> BlockPos {
        BlockPos { x: 2, y: 3, z: 4 }
    }

    fn default_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_CONDUIT_ID)
            .expect("minecraft:conduit has a default state")
    }

    fn target() -> NbtSummaryValue {
        NbtSummaryValue::IntArray(vec![0x0011_2233, 0x4455_6677, 0x7799_aabb_u32 as i32, -1])
    }

    #[test]
    fn conduit_save_load_normalizes_and_round_trips_target_uuid() {
        let mut record = new_block_entity(pos(), default_state())
            .expect("a conduit builds an object")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.push(entry(TAG_TARGET, target()));
        let loaded = crate::block_entity::load_test_fixture(pos(), default_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a conduit");
        let BlockEntityPayload::Conduit(conduit) = loaded.payload() else {
            panic!("payload is a conduit");
        };
        assert_eq!(conduit.destroy_target(), Some(&target()));
        assert_eq!(conduit.destroy_target_uuid(), codec_uuid(&target()));
        assert_eq!(conduit.tick_count(), 0);
        assert!(!conduit.is_active());
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_TARGET), Some(&target()));
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_TARGET
            ),
            Some(&target())
        );
        let again = crate::block_entity::load_test_fixture(pos(), default_state(), &saved)
            .expect("fixture codec succeeds")
            .expect("the round-trip still names a conduit");
        let BlockEntityPayload::Conduit(again) = again.payload() else {
            panic!("payload is a conduit");
        };
        assert_eq!(again.destroy_target(), Some(&target()));
    }

    #[test]
    fn conduit_refreshes_frame_and_preserves_live_target_and_ambient_clock() {
        let pos = pos();
        let read = |_: BlockPos| ConduitBlockRead {
            water: true,
            valid_frame: true,
        };
        let mut conduit = ConduitBlockEntity::default();
        assert_eq!(
            conduit.tick_shape(pos, 39, |_| panic!("not a refresh tick")),
            ConduitRefresh {
                refreshed: false,
                active_change: None,
                active: false,
                hunting: false,
                effect_size: 0,
            }
        );
        assert_eq!(conduit.tick_count(), 1);
        assert!(conduit
            .ambient_tick(39, |_| panic!("inactive RNG"))
            .is_empty());
        assert_eq!(
            conduit.tick_shape(pos, 40, read),
            ConduitRefresh {
                refreshed: true,
                active_change: Some(true),
                active: true,
                hunting: true,
                effect_size: 42,
            }
        );
        assert_eq!(conduit.tick_count(), 2);
        assert_eq!(
            conduit.ambient_tick(40, |bound| {
                assert_eq!(bound, 40);
                20
            }),
            vec![ConduitSound::AmbientShort]
        );
        assert_eq!(
            conduit.ambient_tick(80, |_| panic!("short sound not due")),
            vec![ConduitSound::Ambient]
        );
        assert!(conduit
            .ambient_tick(120, |_| panic!("strict greater-than"))
            .is_empty());
        assert_eq!(
            conduit.ambient_tick(160, |_| 0),
            vec![ConduitSound::Ambient, ConduitSound::AmbientShort]
        );

        assert_eq!(
            conduit.tick_shape(pos, 200, |at| ConduitBlockRead {
                water: true,
                valid_frame: at.y == pos.y,
            }),
            ConduitRefresh {
                refreshed: true,
                active_change: None,
                active: true,
                hunting: false,
                effect_size: 16,
            }
        );
        assert_eq!(
            conduit.tick_shape(pos, 240, |_| ConduitBlockRead {
                water: false,
                valid_frame: true,
            }),
            ConduitRefresh {
                refreshed: true,
                active_change: Some(false),
                active: false,
                hunting: false,
                effect_size: 0,
            }
        );
        assert!(conduit.effect_blocks().is_empty());
        assert!(conduit
            .ambient_tick(240, |_| panic!("deactivated RNG"))
            .is_empty());
    }
}
