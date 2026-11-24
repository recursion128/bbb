//! Shared navigation for `PanicGoal` and `AnimalPanic` registrations.

use crate::block_behavior::weathering::within_manhattan;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_fluid_state;

use crate::entity::level::fluid_state_tag_membership;

use super::random_pos::{air_and_water_random_pos, default_random_pos, RandomPosMob};
use super::stroll::view_vector;
use super::target::TargetView;
use super::{Goal, MobAiContext};

/// `PanicGoal.WATER_CHECK_DISTANCE_VERTICAL` (`PanicGoal.java:18`) and the
/// horizontal reach `canUse` passes (`PanicGoal.java:49`).
const WATER_CHECK_DISTANCE_VERTICAL: i32 = 1;
const WATER_CHECK_DISTANCE_HORIZONTAL: i32 = 5;

/// Damage trigger and water destination semantics for a registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanicKind {
    /// `DamageTypeTags.PANIC_CAUSES`.
    Plain,
    /// `TurtlePanicGoal.canUse`.
    Turtle,
    TamablePlain,
    TamableEnvironmental,
    /// `AnimalPanic`.
    Animal,
    /// `HappyGhastAi.initCoreActivity` uses a flying `AnimalPanic`.
    HappyGhast,
    /// `ArmadilloAi.ArmadilloPanic`.
    AnimalEnvironmental,
    /// `CamelAi.CamelPanic`.
    Camel,
    Panda,
    /// Only a baby panics at every cause.
    PolarBear,
    /// `MountPanicGoal.shouldPanic`.
    Mount,
    /// `FoxPanicGoal.shouldPanic`.
    Fox,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PanicGoal {
    speed_modifier: f64,
    kind: PanicKind,
    wanted: Option<Vec3d>,
}

impl PanicGoal {
    pub(super) const fn new(speed_modifier: f64, kind: PanicKind) -> Self {
        Self {
            speed_modifier,
            kind,
            wanted: None,
        }
    }
}

impl Goal for PanicGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !should_panic(self.kind, targets) {
            return false;
        }
        if self.kind == PanicKind::HappyGhast {
            return true;
        }
        let body = targets.body_entity();
        let level = targets.level();
        if body.is_on_fire() || self.kind == PanicKind::Turtle {
            let animal = matches!(
                self.kind,
                PanicKind::Animal | PanicKind::AnimalEnvironmental | PanicKind::Camel
            );
            match look_for_water(
                level,
                body.block_position(),
                if self.kind == PanicKind::Turtle {
                    7
                } else {
                    WATER_CHECK_DISTANCE_HORIZONTAL
                },
                WATER_CHECK_DISTANCE_VERTICAL,
                animal && body.bb_width().ceil() == 2.0,
            ) {
                Some(Some(pos)) => {
                    let center = if animal { 0.5 } else { 0.0 };
                    self.wanted = Some(Vec3d {
                        x: f64::from(pos.x) + center,
                        y: f64::from(pos.y),
                        z: f64::from(pos.z) + center,
                    });
                    return true;
                }
                Some(None) => {}
                None => return false,
            }
        }
        let mob = RandomPosMob::of(body, context.nav.malus);
        self.wanted = default_random_pos(level, &context.random, mob, 5, 4).flatten();
        self.wanted.is_some()
    }

    // `TamableAnimalPanicGoal.tick`.
    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.kind == PanicKind::HappyGhast && context.nav.navigation_done {
            if let Some(wanted) = happy_ghast_panic_position(context, targets) {
                context.try_navigate_to_position(wanted, 0, self.speed_modifier);
            }
        }
        if matches!(
            self.kind,
            PanicKind::TamablePlain | PanicKind::TamableEnvironmental
        ) {
            super::follow_owner::request_teleport_to_owner(context, targets);
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.kind == PanicKind::Panda
            && matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.panda_retained().is_some_and(|panda| panda.is_sitting()))
        {
            context.stop_navigation();
            return false;
        }
        if self.kind == PanicKind::HappyGhast {
            crate::entity::happy_ghast::is_panicking(targets.body_entity())
        } else {
            !context.nav.navigation_done
        }
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if self.kind == PanicKind::HappyGhast {
            context.stop_navigation();
            return;
        }
        if let Some(wanted) = self.wanted {
            if self.kind == PanicKind::Turtle {
                // Enter the water block instead of stopping on an adjacent bank.
                context.try_navigate_to_position(wanted, 0, self.speed_modifier);
            } else {
                context.navigate_to_position(wanted, self.speed_modifier);
            }
        }
    }
}

// `PanicGoal.shouldPanic` and its overrides.
pub(crate) fn should_panic(kind: PanicKind, targets: &TargetView<'_, '_, '_>) -> bool {
    let body = targets.body_entity();
    if kind == PanicKind::HappyGhast {
        return crate::entity::happy_ghast::is_panicking(body);
    }
    let data = match body.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    };
    let refused = match kind {
        PanicKind::Mount => matches!(
            targets.store().controlling_mob_passenger(body, targets.level()),
            Some(Some(_))
        ),
        PanicKind::Camel => {
            body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
                && targets.store().passengers(body.id()).first()
                    .and_then(|id| targets.store().get(*id))
                    .is_some_and(|passenger| {
                        matches!(passenger.data(), EntityData::GeneralLiving(data) if data.is_mob())
                    })
        }
        PanicKind::Fox => data
            .and_then(|data| data.fox_retained())
            .is_some_and(|fox| fox.is_defending()),
        PanicKind::Plain
        | PanicKind::Turtle
        | PanicKind::TamablePlain
        | PanicKind::TamableEnvironmental
        | PanicKind::Animal
        | PanicKind::HappyGhast
        | PanicKind::AnimalEnvironmental
        | PanicKind::Panda
        | PanicKind::PolarBear => false,
    };
    if refused {
        return false;
    }
    let environmental = match kind {
        PanicKind::Panda | PanicKind::TamableEnvironmental | PanicKind::AnimalEnvironmental => true,
        PanicKind::PolarBear => !data.is_some_and(|data| data.is_baby()),
        PanicKind::Plain
        | PanicKind::Turtle
        | PanicKind::TamablePlain
        | PanicKind::Animal
        | PanicKind::HappyGhast
        | PanicKind::Camel
        | PanicKind::Mount
        | PanicKind::Fox => false,
    };
    damage_causes_panic_with_tag(body, targets.level(), environmental)
}

pub(crate) fn damage_causes_panic(body: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    damage_causes_panic_with_tag(body, level, false)
}

fn damage_causes_panic_with_tag(
    body: &Entity,
    level: &LevelCtx<'_, '_>,
    environmental: bool,
) -> bool {
    let Some(damage_type) = body
        .living()
        .and_then(|living| living.last_damage_type.as_deref())
    else {
        return false;
    };
    let tag = if environmental {
        "minecraft:panic_environmental_causes"
    } else {
        "minecraft:panic_causes"
    };
    level.registry_tag_membership("minecraft:damage_type", tag, damage_type)
}

// `AnimalPanic.getPanicPos` and its flying constructor.
fn happy_ghast_panic_position(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
) -> Option<Vec3d> {
    let body = targets.body_entity();
    if body.is_on_fire() {
        if let Some(pos) = look_for_water(
            targets.level(),
            body.block_position(),
            WATER_CHECK_DISTANCE_HORIZONTAL,
            WATER_CHECK_DISTANCE_VERTICAL,
            body.bb_width().ceil() == 2.0,
        )? {
            return Some(Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            });
        }
    }
    let direction = view_vector(
        body.x_rot(),
        body.living()
            .map_or(body.y_rot(), |living| living.y_head_rot_o),
    );
    air_and_water_random_pos(
        targets.level(),
        &context.random,
        RandomPosMob::of(body, context.nav.malus),
        5,
        4,
        0,
        direction.x,
        direction.z,
        f64::from(std::f32::consts::FRAC_PI_2),
    )
    .flatten()
}

pub(super) fn look_for_water(
    level: &LevelCtx<'_, '_>,
    block_position: BlockPos,
    horizontal_distance: i32,
    vertical_distance: i32,
    wide: bool,
) -> Option<Option<BlockPos>> {
    if !level.has_ticking_column_at(block_position) {
        return None;
    }
    let empty = bbb_server_types::geometry::static_block_collision_box_iter(
        level.block_state(block_position),
    )
    .map_or_else(
        || level.block_collision_boxes(block_position).is_empty(),
        |boxes| boxes.len() == 0,
    );
    if !empty {
        return Some(None);
    }
    for pos in within_manhattan(
        block_position,
        horizontal_distance,
        vertical_distance,
        horizontal_distance,
    ) {
        if !level.has_ticking_column_at(pos) {
            return None;
        }
        // `AnimalPanic.lookForWater`.
        let mut water = true;
        for (dx, dz) in if wide {
            &[(0, 0), (1, 0), (0, 1), (1, 1)][..]
        } else {
            &[(0, 0)][..]
        } {
            let sample = BlockPos {
                x: pos.x + dx,
                y: pos.y,
                z: pos.z + dz,
            };
            if !level.has_ticking_column_at(sample) {
                return None;
            }
            let fluid = block_state_fluid_state(level.block_state(sample));
            water &= fluid_state_tag_membership(level, fluid, "minecraft:water");
        }
        if water {
            return Some(Some(pos));
        }
    }
    Some(None)
}
