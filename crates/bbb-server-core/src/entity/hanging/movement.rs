use super::{
    apply_hanging_post_actions_in_level, post_actions, prepare_drop_contents, HangingPostAction,
};
use crate::entity::{Entity, EntityData, EntityStore, RemovalReason};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

pub(crate) fn move_by_piston(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    delta: Vec3d,
) -> Option<bool> {
    if !breaks_on_movement(entity, delta)? {
        return Some(true);
    }
    Some(break_on_movement(entity, store, level).is_some())
}

impl EntityStore {
    pub(crate) fn push_hanging_entity(
        &mut self,
        level: &mut LevelCtx<'_, '_>,
        id: i32,
        impulse: Vec3d,
    ) -> Option<()> {
        let Some(entity) = self.get(id) else {
            return Some(());
        };
        let breaks = breaks_on_movement(entity, impulse)?;
        if !impulse.x.is_finite() || !impulse.y.is_finite() || !impulse.z.is_finite() || !breaks {
            return Some(());
        }
        let mut entity = self.detach(id)?;
        let result = break_on_movement(&mut entity, self, level);
        self.attach(entity);
        result
    }
}

fn breaks_on_movement(entity: &Entity, delta: Vec3d) -> Option<bool> {
    let fixed = match entity.data() {
        EntityData::Painting(_) | EntityData::LeashKnot(_) => false,
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => frame.fixed(),
        _ => return None,
    };
    Some(!fixed && entity.removal_reason().is_none() && delta.length_sqr() > 0.0)
}

fn break_on_movement(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    entity.set_removed(RemovalReason::Killed);
    let mut actions = prepare_drop_contents(entity, level.entity_drops(), true, true, false);
    if matches!(
        entity.data(),
        EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
    ) {
        actions.push(HangingPostAction::BlockChange(entity.block_position()));
    }
    let actions = post_actions::resolve(store, actions)?;
    store.publish_detached_hanging_removal(entity)?;
    crate::entity::kill::raise_entity_die(entity, level);
    apply_hanging_post_actions_in_level(actions, store, level, None);
    Some(())
}

#[cfg(test)]
mod tests;
