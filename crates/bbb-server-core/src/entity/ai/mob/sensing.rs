use crate::world::simulation_level::LevelCtx;
use std::cell::RefCell;

#[derive(Debug, Clone, Default)]
pub(crate) struct Sensing {
    seen: RefCell<Vec<i32>>,
    unseen: RefCell<Vec<i32>>,
}

pub(in crate::entity::ai::mob) fn body_line_of_sight(
    level: &LevelCtx<'_, '_>,
    body: &crate::entity::Entity,
    from: bbb_protocol::packets::Vec3d,
    to: bbb_protocol::packets::Vec3d,
) -> Option<bool> {
    use bbb_server_types::geometry::Vec3Ops;
    if from.distance_to_sqr(to).sqrt() > 128.0 {
        return Some(false);
    }
    Some(
        bbb_server_types::geometry::clip::clip_with(from, to, |pos| {
            level.block_state_checked(pos)?;
            Some((level.block_collision_boxes_for_entity(body, pos)?, None))
        })?
        .is_none(),
    )
}

impl Sensing {
    pub(super) fn tick(&mut self) {
        self.seen.get_mut().clear();
        self.unseen.get_mut().clear();
    }

    pub(super) fn has_line_of_sight(
        &self,
        target_id: i32,
        calculate: impl FnOnce() -> bool,
    ) -> bool {
        self.has_line_of_sight_checked(target_id, || Some(calculate()))
            .expect("known sight")
    }

    pub(super) fn has_line_of_sight_checked(
        &self,
        target_id: i32,
        calculate: impl FnOnce() -> Option<bool>,
    ) -> Option<bool> {
        if self.seen.borrow().contains(&target_id) {
            return Some(true);
        }
        if self.unseen.borrow().contains(&target_id) {
            return Some(false);
        }
        let visible = calculate()?;
        if visible {
            self.seen.borrow_mut().push(target_id);
        } else {
            self.unseen.borrow_mut().push(target_id);
        }
        Some(visible)
    }
}
