//! Allays retrieve matching ground items and approach their current deposit target.

use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use super::target::TargetView;
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::entity::{allay, Entity, EntityData};

// Sensor.java:14.
const SCAN_INTERVAL: i64 = 20;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Idle,
    Collecting,
    Delivering,
    Following,
}

struct Action {
    mode: Mode,
    position: Vec3d,
}

#[derive(Debug, Clone, Default)]
pub(super) struct AllayCollectGoal {
    wanted_item: Option<i32>,
    mode: Mode,
    next_scan: i64,
    next_path: i64,
}

fn carrying_items(body: &Entity) -> bool {
    matches!(body.data(), EntityData::GeneralLiving(data)
        if data.internal_inventory().and_then(|inventory| inventory.item(0)).is_some())
}

fn eye_position(body: &Entity) -> Vec3d {
    Vec3d {
        y: body.position().y + f64::from(body.eye_height()),
        ..body.position()
    }
}

impl AllayCollectGoal {
    fn wanted_position(&self, targets: &TargetView<'_, '_, '_>) -> Option<Vec3d> {
        let item = targets
            .store()
            .get(self.wanted_item?)
            .filter(|item| item.is_alive())?;
        let EntityData::Item(data) = item.data() else {
            return None;
        };
        (targets
            .body_entity()
            .position()
            .distance_to_sqr(item.position())
            < 32.0 * 32.0
            && allay::wants_to_pick_up(
                targets.body_entity(),
                targets.store(),
                targets.level(),
                data.item(),
            )
            && targets.level().world_border().is_some_and(|border| {
                border.contains_block_at(item.block_position(), targets.game_time())
            }))
        .then_some(item.position())
    }

    fn scan(&mut self, targets: &TargetView<'_, '_, '_>) {
        if targets.game_time() < self.next_scan {
            return;
        }
        self.next_scan = targets.game_time() + SCAN_INTERVAL;
        self.wanted_item = None;
        let body = targets.body_entity();
        let area = body.bounding_box().inflate(32.0, 16.0, 32.0);
        let mut nearest_distance = 32.0 * 32.0;
        for id in targets.store().ids_in_box_sections(area) {
            let Some(item) = targets.store().get(id).filter(|item| item.is_alive()) else {
                continue;
            };
            let EntityData::Item(data) = item.data() else {
                continue;
            };
            let distance = body.position().distance_to_sqr(item.position());
            if distance >= nearest_distance
                || !item.bounding_box().intersects(area)
                || !allay::wants_to_pick_up(body, targets.store(), targets.level(), data.item())
            {
                continue;
            }
            if crate::entity::projectile_hit::collision_clip_misses(
                targets.level(),
                eye_position(body),
                eye_position(item),
            ) {
                self.wanted_item = Some(id);
                nearest_distance = distance;
            }
        }
    }

    fn action(&mut self, targets: &TargetView<'_, '_, '_>) -> Option<Action> {
        let deposit = allay::deposit_target(targets);
        if carrying_items(targets.body_entity()) {
            if let Some(deposit) = &deposit {
                return Some(Action {
                    mode: Mode::Delivering,
                    position: deposit.position,
                });
            }
        }
        if !allay::pickup_on_cooldown(targets.body_entity()) {
            self.scan(targets);
            if let Some(position) = self.wanted_position(targets) {
                return Some(Action {
                    mode: Mode::Collecting,
                    position,
                });
            }
        }
        let deposit = deposit?;
        // AllayAi.
        let distance = if self.mode == Mode::Following {
            4.0
        } else {
            16.0
        };
        (targets
            .body_entity()
            .position()
            .distance_to_sqr(deposit.position)
            > distance * distance)
            .then_some(Action {
                mode: Mode::Following,
                position: deposit.position,
            })
    }
}

impl Goal for AllayCollectGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.action(targets).is_some()
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.action(targets).is_some()
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        self.mode = Mode::Idle;
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        self.mode = Mode::Idle;
        self.wanted_item = None;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(action) = self.action(targets) else {
            context.stop_navigation();
            return;
        };
        context.set_look_at(action.position.x, action.position.y, action.position.z);
        if action.mode == Mode::Delivering
            && eye_position(targets.body_entity()).distance_to_sqr(action.position) < 9.0
        {
            context.stop_navigation();
            context.post_step.push(MobPostStepAction::AllayDeliverItem);
            self.mode = action.mode;
            return;
        }
        if self.mode != action.mode {
            context.stop_navigation();
            self.mode = action.mode;
            self.next_path = targets.game_time() + 1;
        } else if targets.game_time() >= self.next_path {
            let (reach, speed) = match action.mode {
                Mode::Collecting => (0, 1.75),
                Mode::Delivering => (1, 2.25),
                Mode::Following => (4, 2.25),
                Mode::Idle => return,
            };
            context.try_navigate_to_position(action.position, reach, speed);
            self.next_path = targets.game_time() + SCAN_INTERVAL;
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
