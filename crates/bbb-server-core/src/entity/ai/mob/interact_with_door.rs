//! `InteractWithDoor`: villagers, piglins, piglin brutes and copper golems
//! open doors on their path and close them after passing through.

use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::geometry::{Aabb, Vec3Ops};

use crate::block_behavior::door;
use crate::entity::{Entity, EntityData, EntityStore, GroundPath};
use crate::world::simulation_level::LevelCtx;

#[derive(Clone, Default)]
pub(super) struct DoorRuntime {
    pending: Vec<BlockPos>,
}

pub(super) fn opens_doors(entity_type_id: i32) -> bool {
    use bbb_protocol::entity_types::*;
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_VILLAGER_ID
            | VANILLA_ENTITY_TYPE_PIGLIN_ID
            | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            | VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
    )
}

pub(crate) fn tick(entity: &Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if !entity.is_alive_with_health()
        || !matches!(entity.data(), EntityData::GeneralLiving(data) if !data.is_no_ai())
    {
        return;
    }
    let Some(mut doors) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.doors.take())
    else {
        return;
    };
    doors.tick(entity, store, level);
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.doors = Some(doors);
    }
}

impl DoorRuntime {
    // InteractWithDoor: live tagged doors on the path,
    // with nearby same-type path users keeping a passed door open; a sleeping
    // villager closes what it remembers.
    fn tick(&mut self, entity: &Entity, store: &EntityStore, level: &mut LevelCtx<'_, '_>) {
        let nodes = if entity.sleeping_pos().is_some() {
            None
        } else {
            store
                .mob_ai(entity.id())
                .and_then(|runtime| runtime.ground_path())
                .and_then(path_nodes)
        };
        if let Some([previous, next]) = nodes {
            for (pos, remember_open) in [(previous, true), (next, false)] {
                let Some(state) = interactable_door(level, pos) else {
                    continue;
                };
                let opened = door::set_open(level, pos, state, Some(entity.id()), true);
                if (remember_open || opened) && !self.pending.contains(&pos) {
                    self.pending.push(pos);
                }
            }
        }
        self.pending.retain(|&pos| {
            let Some(state) = interactable_door(level, pos) else {
                return false;
            };
            if entity.position().distance_to_sqr(center(pos)) >= 9.0
                || bool_property(state, "open") != Some(true)
            {
                return false;
            }
            if nodes.is_some_and(|nodes| nodes.contains(&pos)) {
                return true;
            }
            if !another_mob_coming_through(entity, store, pos) {
                door::set_open(level, pos, state, Some(entity.id()), false);
            }
            false
        });
    }
}

fn interactable_door(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<i32> {
    let state = level.block_state_checked(pos)?;
    (door::is_door(state) && level.block_state_in_tag(state, "minecraft:mob_interactable_doors"))
        .then_some(state)
}

fn path_nodes(path: &GroundPath) -> Option<[BlockPos; 2]> {
    if path.is_done() {
        return None;
    }
    let next = path.next_node_index();
    Some([
        path.nodes().get(next.checked_sub(1)?)?.pos,
        path.nodes().get(next)?.pos,
    ])
}

fn another_mob_coming_through(entity: &Entity, store: &EntityStore, pos: BlockPos) -> bool {
    let center = center(pos);
    let bounds = Aabb {
        min_x: center.x - 2.0,
        min_y: center.y - 2.0,
        min_z: center.z - 2.0,
        max_x: center.x + 2.0,
        max_y: center.y + 2.0,
        max_z: center.z + 2.0,
    };
    store.ids_in_box_sections(bounds).into_iter().any(|id| {
        let Some(other) = store.get(id) else {
            return false;
        };
        id != entity.id()
            && other.entity_type_id() == entity.entity_type_id()
            && other.is_alive_with_health()
            && other.position().distance_to_sqr(center) < 4.0
            && store
                .mob_ai(id)
                .and_then(|runtime| runtime.ground_path())
                .and_then(path_nodes)
                .is_some_and(|nodes| nodes.contains(&pos))
    })
}

fn center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}
