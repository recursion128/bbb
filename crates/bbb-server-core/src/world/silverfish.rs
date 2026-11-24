//! Silverfish block actions against their current dimension and entity state.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID;
use bbb_protocol::packets::{BlockPos, EntityEvent, ItemStackSummary};
use uuid::Uuid;

use crate::block_behavior::infested::{host_state_by_infested, infested_state_by_host};
use crate::connection::ClientboundPlayPacket;
use crate::entity::{Entity, EntityStore};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::neighbor_updater::UPDATE_LIMIT;

use super::level_blocks::stored_block_state;
use super::{LevelCtx, LevelId, ServerWorld};

impl ServerWorld {
    pub(in crate::world) fn wake_silverfish_friends(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        uuid: Uuid,
    ) {
        let Some(source) = self
            .levels
            .get(dimension)
            .and_then(|level| level.entities.get(entity_id))
            .filter(|body| is_source(body, uuid))
        else {
            return;
        };
        let origin = source.block_position();
        // `SilverfishWakeUpFriendsGoal.tick` (`Silverfish.java:210-212`): scan
        // outward `0, 1, -1, 2, -2, ...` from the hurt silverfish.
        for y in outward_offsets(5) {
            for x in outward_offsets(10) {
                for z in outward_offsets(10) {
                    let pos = BlockPos {
                        x: origin.x + x,
                        y: origin.y + y,
                        z: origin.z + z,
                    };
                    let state = stored_block_state(&self.level_by_id(dimension).chunks, pos);
                    if host_state_by_infested(state).is_none() {
                        continue;
                    }
                    let result = self.with_level_and_entities(*dimension, |level, store| {
                        wake_cell(level, store, entity_id, uuid, pos)
                    });
                    match result {
                        WakeCell::MissingSource => return,
                        WakeCell::Unchanged => continue,
                        WakeCell::Restored => {}
                        WakeCell::Destroyed(state) => self.spawn_after_break(
                            *dimension,
                            pos,
                            state,
                            &ItemStackSummary::empty(),
                            false,
                        ),
                    }
                    if self.level_by_id(dimension).random.next_boolean() {
                        return;
                    }
                }
            }
        }
    }

    pub(in crate::world) fn merge_silverfish(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        uuid: Uuid,
        pos: BlockPos,
    ) {
        if self.levels.get(dimension).is_none() {
            return;
        }
        self.with_level_and_entities(*dimension, |level, store| {
            if !level.mob_griefing() {
                return;
            }
            let _ = store.with_entity_mut(entity_id, |body, store| {
                if !is_source(body, uuid)
                    || !store
                        .mob_ai(entity_id)
                        .is_some_and(|ai| !ai.has_target() && ai.navigation_done())
                {
                    return;
                }
                let position = body.position();
                let origin = BlockPos {
                    x: position.x.floor() as i32,
                    y: (position.y + 0.5).floor() as i32,
                    z: position.z.floor() as i32,
                };
                let distance = i64::from(pos.x).abs_diff(i64::from(origin.x))
                    + i64::from(pos.y).abs_diff(i64::from(origin.y))
                    + i64::from(pos.z).abs_diff(i64::from(origin.z));
                if distance != 1 {
                    return;
                }
                let Some(state) = infested_state_by_host(level.block_state(pos)) else {
                    return;
                };
                if level.set_block(pos, state, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT) {
                    store.broadcast_to_tracking(
                        entity_id,
                        ClientboundPlayPacket::EntityEvent(EntityEvent {
                            entity_id,
                            event_id: 20,
                        }),
                    );
                    body.discard();
                }
            });
        });
    }
}

fn is_source(body: &Entity, uuid: Uuid) -> bool {
    body.uuid() == uuid
        && body.entity_type_id() == VANILLA_ENTITY_TYPE_SILVERFISH_ID
        && body.is_alive_with_health()
}

fn outward_offsets(radius: i32) -> impl Iterator<Item = i32> {
    std::iter::once(0).chain((1..=radius).flat_map(|distance| [distance, -distance]))
}

enum WakeCell {
    MissingSource,
    Unchanged,
    Restored,
    Destroyed(i32),
}

fn wake_cell(
    level: &mut LevelCtx<'_, '_>,
    store: &mut EntityStore,
    entity_id: i32,
    uuid: Uuid,
    pos: BlockPos,
) -> WakeCell {
    let Some(body) = store.get(entity_id).filter(|body| is_source(body, uuid)) else {
        return WakeCell::MissingSource;
    };
    let state = level.block_state(pos);
    let Some(host) = host_state_by_infested(state) else {
        return WakeCell::Unchanged;
    };
    if level.mob_griefing() {
        if level.destroy_block_by_entity(pos, true, body, Some(store)) {
            return WakeCell::Destroyed(state);
        }
    } else if level.set_block(pos, host, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT) {
        return WakeCell::Restored;
    }
    WakeCell::Unchanged
}
