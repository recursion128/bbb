use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::GroundPathType;

use super::spec::{is_instance_of, runtime_class};
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

const MOB_CLASS: &str = "Mob";

#[derive(Debug, Clone)]
pub(crate) struct FollowMobGoal {
    speed_modifier: f64,
    stop_distance: f32,
    area_size: f32,
    following: Option<i32>,
    time_to_recalc_path: i32,
    old_water_cost: f32,
}

impl FollowMobGoal {
    pub(super) const fn new(speed_modifier: f64, stop_distance: f32, area_size: f32) -> Self {
        Self {
            speed_modifier,
            stop_distance,
            area_size,
            following: None,
            time_to_recalc_path: 0,
            old_water_cost: 0.0,
        }
    }
}

impl Goal for FollowMobGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(class) = runtime_class(targets.body_entity().entity_type_id()) else {
            return false;
        };
        let area = f64::from(self.area_size);
        let search = targets
            .body_entity()
            .bounding_box()
            .inflate(area, area, area);
        let Some(found) = targets.store().first_entity_in_box(search, |target| {
            let type_id = target.entity_type_id();
            is_instance_of(type_id, MOB_CLASS)
                && runtime_class(type_id) != Some(class)
                && !target.is_invisible()
        }) else {
            return false;
        };
        self.following = Some(found.id());
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(following) = self.following.and_then(|id| targets.store().get(id)) else {
            return false;
        };
        let stop = self.stop_distance * self.stop_distance;
        !context.nav.navigation_done
            && context.body.position.distance_to_sqr(following.position()) > f64::from(stop)
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.time_to_recalc_path = 0;
        self.old_water_cost = context.nav.malus.get(GroundPathType::Water);
        context.set_own_pathfinding_malus(GroundPathType::Water, 0.0);
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.following = None;
        context.stop_navigation();
        context.set_own_pathfinding_malus(GroundPathType::Water, self.old_water_cost);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(following) = self.following.and_then(|id| targets.store().get(id)) else {
            return;
        };
        if crate::entity::leash::is_leashed(targets.body_entity()) {
            return;
        }
        let position = following.position();
        context.set_look_at_with_limits(
            position.x,
            position.y + f64::from(following.eye_height()),
            position.z,
            10.0,
            40.0,
        );
        self.time_to_recalc_path -= 1;
        if self.time_to_recalc_path > 0 {
            return;
        }
        self.time_to_recalc_path = reduced_tick_delay(10);
        let dist_sqr = context.body.position.distance_to_sqr(position);
        if !matches!(
            dist_sqr.partial_cmp(&f64::from(self.stop_distance * self.stop_distance)),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ) {
            context.navigate_to(position, self.speed_modifier);
            return;
        }
        context.stop_navigation();
        let look_wanted = match targets.store().mob_ai(following.id()) {
            Some(runtime) => runtime.look_wanted_position(),
            None => Some(Vec3d::default()),
        };
        let looked_at = look_wanted == Some(context.body.position);
        if dist_sqr <= f64::from(self.stop_distance) || looked_at {
            let away = Vec3d {
                x: context.body.position.x - (position.x - context.body.position.x),
                y: context.body.position.y,
                z: context.body.position.z - (position.z - context.body.position.z),
            };
            context.navigate_to_position(away, self.speed_modifier);
        }
    }
}
