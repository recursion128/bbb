//! Enderman block moves against the authoritative level and carried state.

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::VANILLA_BLOCK_BEDROCK_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_CARRY_STATE_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{Aabb, BoxShape};

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::level::block_state_tag_membership;
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, PartialEq)]
pub struct PendingBlockMove {
    pub(crate) enderman: TargetRef,
    pub(crate) action: BlockAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockAction {
    Take(BlockPos),
    Place(BlockPos),
}

pub(crate) fn apply(
    level: &mut LevelCtx<'_, '_>,
    store: &mut EntityStore,
    request: PendingBlockMove,
) {
    if !level.mob_griefing() {
        return;
    }
    let _ = store.with_entity_mut(request.enderman.entity_id, |body, store| {
        if body.uuid() != request.enderman.uuid
            || body.entity_type_id() != VANILLA_ENTITY_TYPE_ENDERMAN_ID
            || !body.is_alive_with_health()
        {
            return;
        }
        match request.action {
            BlockAction::Take(pos) => take(level, body, pos),
            BlockAction::Place(pos) => place(level, store, body, pos),
        }
    });
}

fn carried(body: &Entity) -> Option<i32> {
    match body.data() {
        EntityData::GeneralLiving(data) => data.enderman_retained()?.carried_block_state_id(),
        _ => None,
    }
}

// EnderMan.java:343-349.
fn set_carried(body: &mut Entity, state: Option<i32>) {
    let EntityData::GeneralLiving(data) = &mut body.data else {
        return;
    };
    if data
        .enderman_retained_mut()
        .is_some_and(|retained| retained.set_carried_block_state_id(state))
    {
        body.mark_synched_dirty(VANILLA_ENDERMAN_CARRY_STATE_DATA_ID);
    }
}

// EnderMan.java:596-612.
fn take(level: &mut LevelCtx<'_, '_>, body: &mut Entity, pos: BlockPos) {
    if carried(body).is_some() {
        return;
    }
    let Some(state) = level.block_state_checked(pos) else {
        return;
    };
    if !block_state_tag_membership(level, state, "minecraft:enderman_holdable") {
        return;
    }
    let origin = body.block_position();
    let from = Vec3d {
        x: f64::from(origin.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(origin.z) + 0.5,
    };
    let to = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    // A completed miss carries the endpoint cell in Vanilla's BlockHitResult.
    if !level
        .clip_outline_for_entity(body, from, to)
        .is_some_and(|hit| hit.is_none_or(|hit| hit == pos))
    {
        return;
    }
    let Some(default_state) =
        vanilla_block_state_block_type_id(state).and_then(vanilla_block_default_state_id)
    else {
        return;
    };
    level.remove_block(pos);
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_DESTROY,
        pos,
        Some(body.id()),
        Some(state),
    );
    set_carried(body, Some(default_state));
}

// EnderMan.java:451-481.
fn place(level: &mut LevelCtx<'_, '_>, store: &EntityStore, body: &mut Entity, pos: BlockPos) {
    let Some(state) = carried(body) else {
        return;
    };
    let state = level.placement_state_from_neighbors(state, pos);
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    let Some(target_state) = level.block_state_checked(pos) else {
        return;
    };
    let Some(below_state) = level.block_state_checked(below) else {
        return;
    };
    if !block_state_is_air(target_state)
        || block_state_is_air(below_state)
        || vanilla_block_state_block_type_id(below_state) == Some(VANILLA_BLOCK_BEDROCK_ID)
        || support_is_full_cube(level, below) != Some(true)
        || !crate::block_behavior::can_survive_at(state, pos, level)
        || occupied(level, store, body, Aabb::of_block(pos))
    {
        return;
    }
    level.set_block(
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(
        VanillaGameEvent::BLOCK_PLACE,
        pos,
        Some(body.id()),
        Some(state),
    );
    set_carried(body, None);
}

fn support_is_full_cube(level: &LevelCtx<'_, '_>, below: BlockPos) -> Option<bool> {
    let boxes = level.block_collision_boxes_empty(below)?;
    if boxes.len() == 1 && boxes[0].is_unit_cube() {
        return Some(true);
    }
    let shape = BoxShape::from_boxes(&boxes)?;
    let cube = BoxShape::from_box(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }))?;
    Some(cube.only_first(&shape)?.is_empty() && shape.only_first(&cube)?.is_empty())
}

// EntityGetter.java:29-30; EntitySelector.java:17.
fn occupied(level: &LevelCtx<'_, '_>, store: &EntityStore, body: &Entity, area: Aabb) -> bool {
    let bodies = EntityBodyReadView::new(store, Some(body), level.detached_bodies());
    bodies.ids_in_box_sections(area).into_iter().any(|id| {
        id != body.id()
            && bodies
                .get(id)
                .is_some_and(|other| other.bounding_box().intersects(area))
    }) || level
        .players_in_box(area)
        .into_iter()
        .any(|player| player.entity_id != body.id() && !player.spectator)
}
