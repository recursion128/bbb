//! Live hive queries and admission.

use super::LevelCtx;
use crate::block_entity::BlockEntity;
use crate::chunk::poi::PoiOccupancy;
use crate::entity::{Entity, EntityStore};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID;
use bbb_protocol::packets::BlockPos;

impl LevelCtx<'_, '_> {
    pub(crate) fn bee_hive_valid(&self, pos: BlockPos) -> bool {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::beehive)
            .is_some()
    }

    pub(crate) fn bee_hive_has_space(&self, pos: BlockPos) -> bool {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::beehive)
            .is_some_and(|hive| hive.has_space())
    }

    /// `BeehiveBlockEntity.isFireNearby` (`BeehiveBlockEntity.java:103-114`).
    pub(crate) fn bee_hive_near_fire(&self, pos: BlockPos) -> bool {
        (-1..=1).any(|x| {
            (-1..=1).any(|y| {
                (-1..=1).any(|z| {
                    vanilla_block_state_block_type_id(self.block_state(BlockPos {
                        x: pos.x + x,
                        y: pos.y + y,
                        z: pos.z + z,
                    })) == Some(VANILLA_BLOCK_FIRE_ID)
                })
            })
        })
    }

    pub(crate) fn bee_hives_near(&mut self, pos: BlockPos) -> Vec<BlockPos> {
        let mut candidates = self.chunks.poi_mut().records_in_range(
            [pos.x, pos.y, pos.z],
            20,
            PoiOccupancy::Any,
            |_| true,
        );
        candidates.retain(|record| {
            self.registry_tag_membership(
                "minecraft:point_of_interest_type",
                "minecraft:bee_home",
                record.type_id,
            )
        });
        candidates.sort_by_key(|record| {
            record
                .pos
                .iter()
                .zip([pos.x, pos.y, pos.z])
                .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
                .sum::<i64>()
        });
        candidates
            .into_iter()
            .map(|record| BlockPos {
                x: record.pos[0],
                y: record.pos[1],
                z: record.pos[2],
            })
            .filter(|pos| self.bee_hive_has_space(*pos) && !self.bee_hive_near_fire(*pos))
            .collect()
    }

    /// `BeehiveBlockEntity.addOccupant` (`BeehiveBlockEntity.java:167-186`).
    pub(crate) fn bee_enter_hive(
        &mut self,
        entity: &mut Entity,
        store: &mut EntityStore,
        pos: BlockPos,
        nectar: bool,
        flower: Option<BlockPos>,
    ) -> bool {
        if !self.bee_hive_has_space(pos) || self.bee_hive_near_fire(pos) {
            return false;
        }
        store.stop_invalid_riding(entity.id());
        store.eject_body_passengers_reverse(entity);
        if !crate::entity::leash::drop_leash(entity, store, self, true, true) {
            return false;
        }
        let Ok(record) = crate::entity::save::beehive::hive_occupant_record(entity, store) else {
            return false;
        };
        let replace_flower = self.level_random().next_boolean();
        let Some(hive) = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::beehive_mut)
        else {
            return false;
        };
        if !hive.add_occupant(record, nectar, flower, replace_flower) {
            return false;
        }
        self.chunks.block_entities_mut().set_changed(pos);
        self.play_block_sound(pos, "minecraft:block.beehive.enter");
        self.game_event_from_entity(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(entity.id()),
            Some(self.block_state(pos)),
        );
        entity.discard();
        true
    }
}
