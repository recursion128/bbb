//! Following a llama caravan's immediate head.

use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::llama::{can_join, first_is_leashed, CaravanChange, CaravanState};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{leash, Entity, EntityData};

use super::target::{TargetRef, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct LlamaCaravanGoal {
    speed_modifier: f64,
    dist_check_counter: i32,
}

impl LlamaCaravanGoal {
    pub(super) const fn new() -> Self {
        Self {
            speed_modifier: 2.1,
            dist_check_counter: 0,
        }
    }
}

fn caravan(body: &Entity) -> Option<CaravanState> {
    match body.data() {
        EntityData::GeneralLiving(data) => data.llama_caravan(),
        _ => None,
    }
}

fn target_ref(body: &Entity) -> TargetRef {
    TargetRef {
        entity_id: body.id(),
        uuid: body.uuid(),
    }
}

fn head<'a>(bodies: EntityBodyReadView<'a>, body: &Entity) -> Option<&'a Entity> {
    let target = caravan(body)?.head?;
    bodies
        .get(target.entity_id)
        .filter(|head| head.uuid() == target.uuid)
}

impl Goal for LlamaCaravanGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        let Some(state) = caravan(body) else {
            return false;
        };
        if leash::is_leashed(body) || state.head.is_some() {
            return false;
        }
        let bodies = EntityBodyReadView::new(
            targets.store(),
            Some(body),
            targets.level().detached_bodies(),
        );
        let bounds = body.bounding_box().inflate(9.0, 4.0, 9.0);
        let mut caravan_tail: Option<(TargetRef, f64)> = None;
        let mut leashed_tail: Option<(TargetRef, f64)> = None;
        for id in bodies.ids_in_box_sections(bounds) {
            let Some(candidate) = bodies.get(id) else {
                continue;
            };
            if candidate.id() == body.id() || !candidate.bounding_box().intersects(bounds) {
                continue;
            }
            let Some(state) = caravan(candidate) else {
                continue;
            };
            if state.tail.is_some() {
                continue;
            }
            let distance = body.position().distance_to_sqr(candidate.position());
            let selected = if state.head.is_some() {
                &mut caravan_tail
            } else if leash::is_leashed(candidate) {
                &mut leashed_tail
            } else {
                continue;
            };
            if selected.is_none_or(|(_, nearest)| distance <= nearest) {
                *selected = Some((target_ref(candidate), distance));
            }
        }
        let Some((selected, distance)) = caravan_tail.or(leashed_tail) else {
            return false;
        };
        // Qualification follows nearest selection, so an invalid closest tail blocks joining.
        if distance < 4.0 || !can_join(bodies, target_ref(body), selected) {
            return false;
        }
        context.species.caravan_change = Some(CaravanChange::Join(selected));
        true
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        let bodies = EntityBodyReadView::new(
            targets.store(),
            Some(body),
            targets.level().detached_bodies(),
        );
        let Some(head) = head(bodies, body) else {
            return false;
        };
        if !head.is_alive() || !first_is_leashed(bodies, target_ref(body), 0) {
            return false;
        }
        if body.position().distance_to_sqr(head.position()) > 676.0 {
            if self.speed_modifier <= 3.0 {
                self.speed_modifier *= 1.2;
                self.dist_check_counter = reduced_tick_delay(40);
                return true;
            }
            if self.dist_check_counter == 0 {
                return false;
            }
        }
        if self.dist_check_counter > 0 {
            self.dist_check_counter -= 1;
        }
        true
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.species.caravan_change = Some(CaravanChange::Leave);
        self.speed_modifier = 2.1;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let body = targets.body_entity();
        let bodies = EntityBodyReadView::new(
            targets.store(),
            Some(body),
            targets.level().detached_bodies(),
        );
        if leash::leash_holder(body)
            .and_then(|id| bodies.get(id))
            .is_some_and(|holder| matches!(holder.data(), EntityData::LeashKnot(_)))
        {
            return;
        }
        let Some(head) = head(bodies, body) else {
            return;
        };
        let position = body.position();
        let delta = Vec3d {
            x: head.position().x - position.x,
            y: head.position().y - position.y,
            z: head.position().z - position.z,
        };
        let distance = position.distance_to_sqr(head.position()).sqrt();
        let wanted = position.plus(delta.normalize().scale((distance - 2.0).max(0.0)));
        context.navigate_to_position(wanted, self.speed_modifier);
    }
}
