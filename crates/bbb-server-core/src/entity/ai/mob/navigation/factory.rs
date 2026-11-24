//! The navigation each mob's navigation factory builds and the variant
//! dispatch over it.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::{BlockPos, DebugPathInfo, Vec3d};

use crate::entity::ai::mob::spec::mob_spec;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID;

use super::flying::{
    FlyingMoveControlSpec, FlyingNavigation, FlyingNavigationSpec, FlyingStableDestination,
    FlyingTickGate,
};
use super::water::WaterNavigation;
use super::{
    AmphibiousPathPrepare, GroundNavigation, GroundPath, MobPathingSnapshot, NavigationMove,
    PathNavigationBase,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NavigationFactory {
    Ground,
    WallClimber,
    Strider,
    Frog,
    Amphibious,
    Water { allow_breaching: bool },
    Flying(FlyingNavigationSpec),
}

const FLYING: FlyingNavigationSpec = FlyingNavigationSpec {
    can_float: true,
    required_path_length: None,
    stable_destination: FlyingStableDestination::StandOnBlock,
    tick_gate: FlyingTickGate::Always,
    fluids_block_direct_move: true,
    move_control: FlyingMoveControlSpec {
        max_turn: 10,
        hovers_in_place: false,
    },
};

pub(crate) const FLYING_NAVIGATION: FlyingNavigationSpec = FLYING;

pub(crate) const ALLAY_NAVIGATION: FlyingNavigationSpec = FlyingNavigationSpec {
    required_path_length: Some(48.0),
    move_control: FlyingMoveControlSpec {
        max_turn: 20,
        hovers_in_place: true,
    },
    ..FLYING
};

pub(crate) const BEE_NAVIGATION: FlyingNavigationSpec = FlyingNavigationSpec {
    can_float: false,
    required_path_length: Some(48.0),
    stable_destination: FlyingStableDestination::BelowNotAir,
    tick_gate: FlyingTickGate::UnlessPollinating,
    move_control: FlyingMoveControlSpec {
        max_turn: 20,
        hovers_in_place: true,
    },
    ..FLYING
};

/// `HappyGhast.createBabyNavigation`.
const HAPPY_GHAST_BABY_NAVIGATION: FlyingNavigationSpec = FlyingNavigationSpec {
    required_path_length: Some(48.0),
    fluids_block_direct_move: false,
    move_control: FlyingMoveControlSpec {
        max_turn: 180,
        hovers_in_place: true,
    },
    ..FLYING
};

pub(crate) fn navigation_factory(entity_type_id: i32, baby: bool) -> Option<NavigationFactory> {
    if baby && entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
        return Some(NavigationFactory::Flying(HAPPY_GHAST_BABY_NAVIGATION));
    }
    mob_spec(entity_type_id)?.navigation
}

/// The navigation a mob holds, one variant per factory family.
#[derive(Debug, Clone)]
pub(in crate::entity::ai::mob) enum MobNavigation {
    Ground(GroundNavigation),
    Flying(FlyingNavigation),
    Water(WaterNavigation),
}

impl Default for MobNavigation {
    fn default() -> Self {
        Self::Ground(GroundNavigation::default())
    }
}

impl MobNavigation {
    pub(in crate::entity::ai::mob) fn from_factory(
        factory: NavigationFactory,
        entity_type_id: i32,
        base_follow_range: f64,
    ) -> Self {
        let mut navigation = match factory {
            NavigationFactory::Ground => Self::Ground(GroundNavigation::default()),
            NavigationFactory::Strider => Self::Ground(GroundNavigation {
                can_stand_on_lava: true,
                ..GroundNavigation::default()
            }),
            NavigationFactory::WallClimber => Self::Ground(GroundNavigation {
                wall_climber: true,
                ..GroundNavigation::default()
            }),
            NavigationFactory::Frog => {
                let mut navigation = GroundNavigation::default();
                navigation.set_frog();
                Self::Ground(navigation)
            }
            NavigationFactory::Amphibious => {
                let mut navigation = GroundNavigation::default();
                navigation.set_amphibious();
                Self::Ground(navigation)
            }
            NavigationFactory::Flying(spec) => {
                Self::Flying(FlyingNavigation::new(spec, base_follow_range))
            }
            NavigationFactory::Water { allow_breaching } => {
                Self::Water(WaterNavigation::new(allow_breaching))
            }
        };
        if let Self::Ground(ground) = &mut navigation {
            ground.apply_tuning(entity_type_id);
        }
        navigation
    }

    pub(in crate::entity::ai::mob) fn base(&self) -> &PathNavigationBase {
        match self {
            Self::Ground(navigation) => navigation,
            Self::Flying(navigation) => navigation,
            Self::Water(navigation) => navigation,
        }
    }

    fn base_mut(&mut self) -> &mut PathNavigationBase {
        match self {
            Self::Ground(navigation) => navigation,
            Self::Flying(navigation) => navigation,
            Self::Water(navigation) => navigation,
        }
    }

    pub(in crate::entity::ai::mob) fn ground(&self) -> Option<&GroundNavigation> {
        match self {
            Self::Ground(navigation) => Some(navigation),
            Self::Flying(_) | Self::Water(_) => None,
        }
    }

    pub(in crate::entity::ai::mob) fn ground_mut(&mut self) -> Option<&mut GroundNavigation> {
        match self {
            Self::Ground(navigation) => Some(navigation),
            Self::Flying(_) | Self::Water(_) => None,
        }
    }

    pub(in crate::entity::ai::mob) fn path(&self) -> Option<&GroundPath> {
        self.base().path()
    }

    pub(in crate::entity::ai::mob) fn is_done(&self) -> bool {
        self.base().is_done()
    }

    pub(in crate::entity::ai::mob) fn stop(&mut self) {
        match self {
            Self::Ground(navigation) => navigation.stop(),
            Self::Flying(navigation) => navigation.stop(),
            Self::Water(navigation) => navigation.stop(),
        }
    }

    pub(in crate::entity::ai::mob) fn set_debug_capture(&mut self, enabled: bool) {
        self.base_mut().set_debug_capture(enabled);
    }

    pub(in crate::entity::ai::mob) fn debug_info(&self) -> Option<DebugPathInfo> {
        self.base().debug_info()
    }

    pub(in crate::entity::ai::mob) fn should_recompute(
        &self,
        positions: &[BlockPos],
        mob_position: Vec3d,
    ) -> bool {
        self.base().should_recompute(positions, mob_position)
    }

    /// Ground paths read the live follow range for every search instead.
    pub(in crate::entity::ai::mob) fn update_pathfinder_max_visited_nodes(
        &mut self,
        follow_range: f64,
    ) {
        if let Self::Flying(navigation) = self {
            navigation.update_pathfinder_max_visited_nodes(follow_range);
        }
    }

    pub(in crate::entity::ai::mob) fn take_amphibious_prepare(
        &self,
    ) -> Option<AmphibiousPathPrepare> {
        self.ground()
            .and_then(GroundNavigation::take_amphibious_prepare)
    }

    // `PathNavigation.setMaxVisitedNodesMultiplier`.
    pub(in crate::entity::ai::mob) fn set_max_visited_nodes_multiplier(&mut self, multiplier: f32) {
        if let Self::Flying(navigation) = self {
            navigation.max_visited_nodes_multiplier = multiplier;
        }
    }

    pub(in crate::entity::ai::mob) fn set_can_float(&mut self, can_float: bool) {
        match self {
            Self::Ground(navigation) => navigation.set_can_float(can_float),
            Self::Flying(navigation) => navigation.set_can_float(can_float),
            Self::Water(_) => {}
        }
    }

    pub(in crate::entity::ai::mob) fn set_can_open_doors(&mut self, can_open_doors: bool) {
        match self {
            Self::Ground(navigation) => navigation.set_can_open_doors(can_open_doors),
            Self::Flying(navigation) => navigation.set_can_open_doors(can_open_doors),
            Self::Water(navigation) => navigation.can_open_doors = can_open_doors,
        }
    }

    pub(in crate::entity::ai::mob) fn set_can_path_to_targets_below_surface(
        &mut self,
        value: bool,
    ) {
        if let Self::Ground(navigation) = self {
            navigation.set_can_path_to_targets_below_surface(value);
        }
    }

    pub(in crate::entity::ai::mob) fn create_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
    ) -> Option<GroundPath> {
        match self {
            Self::Ground(navigation) => navigation.create_path(level, mob, target, reach_range),
            Self::Flying(navigation) => {
                navigation.create_path(level, mob, random, target, reach_range)
            }
            Self::Water(navigation) => navigation.create_path(level, mob, target, reach_range),
        }
    }

    pub(in crate::entity::ai::mob) fn create_path_to(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        max_path_length: Option<f32>,
    ) -> Option<GroundPath> {
        match self {
            Self::Ground(navigation) => {
                navigation.create_path_to(level, mob, target, reach_range, max_path_length)
            }
            Self::Flying(navigation) => navigation.create_path_to(
                level,
                mob,
                random,
                target,
                (reach_range, max_path_length, false),
            ),
            Self::Water(navigation) => {
                navigation.create_path_to(level, mob, target, reach_range, max_path_length, false)
            }
        }
    }

    /// `createPath` remembers its target for `recomputePath` before the
    /// path starts (`PathNavigation.java`).
    pub(in crate::entity::ai::mob) fn start_created_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        path: GroundPath,
        mob: MobPathingSnapshot,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        self.base_mut().accept_created_path(&path, reach_range);
        self.start_path(level, path, mob, speed_modifier)
    }

    pub(in crate::entity::ai::mob) fn start_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        path: GroundPath,
        mob: MobPathingSnapshot,
        speed_modifier: f64,
    ) -> bool {
        match self {
            Self::Ground(navigation) => navigation.start_path(level, path, mob, speed_modifier),
            Self::Flying(navigation) => navigation.start_path(level, path, mob, speed_modifier),
            Self::Water(navigation) => navigation.start_path(level, path, mob, speed_modifier),
        }
    }

    pub(in crate::entity::ai::mob) fn is_stuck(&self) -> bool {
        self.base().is_stuck()
    }

    pub(in crate::entity::ai::mob) fn frog_enabled(&self) -> bool {
        self.ground().is_some_and(GroundNavigation::frog_enabled)
    }

    pub(in crate::entity::ai::mob) fn move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        match self {
            Self::Ground(navigation) => {
                navigation.move_to(level, mob, target, reach_range, speed_modifier)
            }
            Self::Flying(navigation) => {
                navigation.move_to(level, mob, random, target, reach_range, speed_modifier)
            }
            Self::Water(navigation) => {
                navigation.move_to(level, mob, target, reach_range, speed_modifier, false)
            }
        }
    }

    pub(in crate::entity::ai::mob) fn try_move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        match self {
            Self::Ground(navigation) => {
                navigation.try_move_to(level, mob, target, reach_range, speed_modifier)
            }
            Self::Flying(navigation) => {
                navigation.move_to(level, mob, random, target, reach_range, speed_modifier)
            }
            Self::Water(navigation) => {
                navigation.move_to(level, mob, target, reach_range, speed_modifier, true)
            }
        }
    }

    pub(in crate::entity::ai::mob) fn set_speed_modifier(&mut self, speed_modifier: f64) {
        self.base_mut().speed_modifier = speed_modifier;
    }

    pub(in crate::entity::ai::mob) fn move_to_entity(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        match self {
            Self::Ground(navigation) => {
                navigation.move_to_entity(level, mob, target, reach_range, speed_modifier)
            }
            Self::Flying(navigation) => {
                navigation.move_to_entity(level, mob, random, target, reach_range, speed_modifier)
            }
            Self::Water(navigation) => {
                navigation.move_to(level, mob, target, reach_range, speed_modifier, true)
            }
        }
    }

    pub(in crate::entity::ai::mob) fn recompute(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
    ) {
        match self {
            Self::Ground(navigation) => navigation.recompute(level, mob),
            Self::Flying(navigation) => navigation.recompute(level, mob, random),
            Self::Water(navigation) => navigation.recompute(level, mob),
        }
    }

    /// `PathNavigation.tick`; `pollinating` is what a gated override reads.
    pub(in crate::entity::ai::mob) fn tick(
        &mut self,
        level: &LevelCtx<'_, '_>,
        context: &mut crate::entity::ai::mob::MobAiContext,
        pollinating: bool,
    ) -> Option<NavigationMove> {
        match self {
            Self::Ground(navigation) => navigation.tick(level, context),
            Self::Water(navigation) => navigation.tick(level, context),
            Self::Flying(navigation) => {
                let gate_open =
                    navigation.spec().tick_gate == FlyingTickGate::Always || !pollinating;
                navigation.tick(level, context, gate_open)
            }
        }
    }
}
