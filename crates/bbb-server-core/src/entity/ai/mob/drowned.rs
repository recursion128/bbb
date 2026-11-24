//! Drowned seek water by day and the surface and shore at night.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::block_queries::collision_face_full;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::Vec3Ops;

use super::random_pos::{default_random_pos_towards, RandomPosMob};
use super::spec::MobGoal;
use super::target::TargetView;
use super::{Goal, MobAiContext, MobAiRuntime};
use crate::entity::{Entity, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(super) fn held_trident(
    entity: &Entity,
) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
    entity
        .living()?
        .equipment()
        .get(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        .filter(|stack| !stack.is_empty() && stack.item_resource_id() == Some("minecraft:trident"))
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SwimIntent {
    pub(crate) wants_to_swim: bool,
    pub(crate) rise: bool,
}

impl MobAiRuntime {
    pub(crate) fn drowned_swim_intent(
        &self,
        body: &Entity,
        store: &EntityStore,
        level: &LevelCtx<'_, '_>,
    ) -> SwimIntent {
        let searching = self
            .goal_selector
            .is_running(|goal| matches!(goal, MobGoal::DrownedSwimUp(_)));
        let target = self.water_target_position(body, store, level);
        SwimIntent {
            wants_to_swim: searching || target.is_some(),
            rise: searching || target.is_some_and(|target| target.y > body.position().y),
        }
    }
}

fn center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    }
}

#[derive(Clone, Default)]
pub(super) struct DrownedGoToWaterGoal {
    wanted: Option<Vec3d>,
    next_search: i64,
}

impl Goal for DrownedGoToWaterGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if targets.bright_outside() != Some(true)
            || context.body.in_water
            || targets.game_time() < self.next_search
        {
            return false;
        }
        self.next_search = targets.game_time() + 20;
        let origin = targets.body_entity().block_position();
        self.wanted = (0..10).find_map(|_| {
            let pos = BlockPos {
                x: origin.x + context.next_int(20) - 10,
                y: origin.y + 2 - context.next_int(8),
                z: origin.z + context.next_int(20) - 10,
            };
            (targets.level().has_ticking_column_at(pos)
                && vanilla_block_state_block_type_id(targets.level().block_state(pos))
                    == Some(VANILLA_BLOCK_WATER_ID))
            .then(|| center(pos))
        });
        self.wanted.is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.nav.navigation_done
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if let Some(wanted) = self.wanted {
            context.try_navigate_to_position(wanted, 0, 1.0);
        }
    }
}

pub(super) fn is_beach(targets: &TargetView<'_, '_, '_>, pos: BlockPos) -> bool {
    let level = targets.level();
    level.has_ticking_column_at(pos)
        && [1, 2].into_iter().all(|dy| {
            block_state_is_air(level.block_state(BlockPos {
                y: pos.y + dy,
                ..pos
            }))
        })
        && collision_face_full(level.block_state(pos), Direction::Up)
            .or_else(|| {
                let boxes = level.block_collision_boxes_for_entity(targets.body_entity(), pos)?;
                bbb_server_types::geometry::collision_face_full(&boxes, Direction::Up)
            })
            .unwrap_or(false)
}

#[derive(Clone, Default)]
pub(super) struct DrownedSwimUpGoal {
    stuck: bool,
}

fn can_swim_up(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    targets.bright_outside() == Some(false)
        && context.body.in_water
        && targets
            .level()
            .sea_level()
            .is_some_and(|sea| context.body.position.y < f64::from(sea - 2))
}

impl Goal for DrownedSwimUpGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        can_swim_up(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !self.stuck && can_swim_up(context, targets)
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.stuck = false;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let close = context.nav.navigation_target.is_some_and(|pos| {
            context.body.position.distance_to_sqr(Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            }) < 4.0
        });
        if !context.nav.navigation_done && !close {
            return;
        }
        let Some(sea) = targets.level().sea_level() else {
            return;
        };
        // Drowned: reuse the ordinary home/malus-aware sampler.
        let wanted = default_random_pos_towards(
            targets.level(),
            &context.random,
            RandomPosMob::of(targets.body_entity(), context.nav.malus),
            4,
            8,
            Vec3d {
                y: f64::from(sea - 1),
                ..context.body.position
            },
            f64::from(std::f32::consts::FRAC_PI_2),
        )
        .flatten();
        if let Some(wanted) = wanted {
            context.try_navigate_to_position(wanted, 0, 1.0);
        } else {
            self.stuck = true;
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
