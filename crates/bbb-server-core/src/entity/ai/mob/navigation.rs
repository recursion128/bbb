use crate::world::simulation_level::LevelCtx;
mod amphibious;
mod base;
mod factory;
mod fly_node_evaluator;
mod flying;
pub(crate) mod malus;
mod path_finder;
mod smooth_swimming;
mod tuning;
mod water;

use std::cell::Cell;

use malus::PathfindingMalus;
pub(crate) use tuning::GroundTuning;

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
};
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_path_facts,
    vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{
    self as block_types, vanilla_block_type, VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::{BlockPos, DebugPathNode, DebugPathType, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily, FluidState};
use bbb_server_types::geometry::Aabb;

use crate::entity::Entity;

pub(super) use self::base::PathNavigationBase;
pub(super) use self::factory::MobNavigation;
pub(crate) use self::factory::{
    navigation_factory, NavigationFactory, ALLAY_NAVIGATION, BEE_NAVIGATION, FLYING_NAVIGATION,
};
pub(crate) use self::flying::FlyingMoveControlSpec;

#[cfg(test)]
use self::path_finder::find_path;
use self::path_finder::find_path_with_start_status;

// Ground destinations need solid render support; amphibious destinations
// need a non-air floor, and Striders also accept lava.
pub(super) fn random_pos_stable_destination(
    level: &LevelCtx<'_, '_>,
    entity_type_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let factory = navigation_factory(entity_type_id, false);
    if let Some(NavigationFactory::Flying(spec)) = factory {
        return flying::stable_destination(level, spec.stable_destination, pos);
    }
    if matches!(factory, Some(NavigationFactory::Water { .. })) {
        return water::stable_destination(level, pos);
    }
    if factory == Some(NavigationFactory::Strider) {
        if !level.has_ticking_column_at(pos) {
            return None;
        }
        if vanilla_block_state_block_type_id(level.block_state(pos))? == VANILLA_BLOCK_LAVA_ID {
            return Some(true);
        }
    }
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if !level.has_ticking_column_at(below) {
        return None;
    }
    let state = level.block_state(below);
    if matches!(
        factory,
        Some(NavigationFactory::Frog | NavigationFactory::Amphibious)
    ) {
        if let Some(facts) = vanilla_block_state_path_facts(state) {
            Some(!facts.is_air())
        } else {
            level.content_registry().block_by_state_id(state)?;
            Some(true)
        }
    } else if let Some(facts) = vanilla_block_state_support_facts(state) {
        Some(facts.solid_render())
    } else {
        Some(
            level
                .content_registry()
                .block_by_state_id(state)?
                .0
                .full_cube_collision,
        )
    }
}

// `PathType.java`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroundPathType {
    Blocked = 0,
    Open = 1,
    Walkable = 2,
    WalkableDoor = 3,
    Trapdoor = 4,
    PowderSnow = 5,
    OnTopOfPowderSnow = 6,
    Fence = 7,
    Lava = 8,
    Water = 9,
    WaterBorder = 10,
    Rail = 11,
    UnpassableRail = 12,
    FireInNeighbor = 13,
    Fire = 14,
    DamagingInNeighbor = 15,
    Damaging = 16,
    DoorOpen = 17,
    DoorWoodClosed = 18,
    DoorIronClosed = 19,
    Breach = 20,
    Leaves = 21,
    StickyHoney = 22,
    Cocoa = 23,
    DamageCautious = 24,
    OnTopOfTrapdoor = 25,
    BigMobsCloseToDanger = 26,
}

impl GroundPathType {
    const ALL: [Self; 27] = [
        Self::Blocked,
        Self::Open,
        Self::Walkable,
        Self::WalkableDoor,
        Self::Trapdoor,
        Self::PowderSnow,
        Self::OnTopOfPowderSnow,
        Self::Fence,
        Self::Lava,
        Self::Water,
        Self::WaterBorder,
        Self::Rail,
        Self::UnpassableRail,
        Self::FireInNeighbor,
        Self::Fire,
        Self::DamagingInNeighbor,
        Self::Damaging,
        Self::DoorOpen,
        Self::DoorWoodClosed,
        Self::DoorIronClosed,
        Self::Breach,
        Self::Leaves,
        Self::StickyHoney,
        Self::Cocoa,
        Self::DamageCautious,
        Self::OnTopOfTrapdoor,
        Self::BigMobsCloseToDanger,
    ];

    #[inline]
    const fn from_ordinal(ordinal: usize) -> Self {
        Self::ALL[ordinal]
    }

    pub const fn default_malus(self) -> f32 {
        match self {
            Self::Blocked
            | Self::PowderSnow
            | Self::Fence
            | Self::Lava
            | Self::UnpassableRail
            | Self::Damaging
            | Self::DoorWoodClosed
            | Self::DoorIronClosed
            | Self::Leaves => -1.0,
            Self::Water
            | Self::WaterBorder
            | Self::FireInNeighbor
            | Self::DamagingInNeighbor
            | Self::StickyHoney => 8.0,
            Self::Fire => 16.0,
            Self::Breach | Self::BigMobsCloseToDanger => 4.0,
            Self::Open
            | Self::Walkable
            | Self::WalkableDoor
            | Self::Trapdoor
            | Self::OnTopOfPowderSnow
            | Self::Rail
            | Self::DoorOpen
            | Self::Cocoa
            | Self::DamageCautious
            | Self::OnTopOfTrapdoor => 0.0,
        }
    }

    const fn can_cut_corner(self) -> bool {
        !matches!(
            self,
            Self::FireInNeighbor | Self::DamagingInNeighbor | Self::WalkableDoor
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundPathNode {
    pub pos: BlockPos,
    pub path_type: GroundPathType,
    pub cost_malus: f32,
}

// `Path.java`.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundPath {
    nodes: Vec<GroundPathNode>,
    next_node_index: usize,
    target: BlockPos,
    distance_to_target: f32,
    reached: bool,
    debug_data: Option<GroundPathDebugData>,
}

#[derive(Debug, Clone, PartialEq)]
struct GroundPathDebugData {
    nodes: Vec<DebugPathNode>,
    target_nodes: Vec<DebugPathNode>,
    open_set: Vec<DebugPathNode>,
    closed_set: Vec<DebugPathNode>,
}

impl GroundPath {
    pub fn nodes(&self) -> &[GroundPathNode] {
        &self.nodes
    }

    pub const fn next_node_index(&self) -> usize {
        self.next_node_index
    }

    pub const fn target(&self) -> BlockPos {
        self.target
    }

    pub const fn reached(&self) -> bool {
        self.reached
    }

    pub const fn distance_to_target(&self) -> f32 {
        self.distance_to_target
    }

    pub fn is_done(&self) -> bool {
        self.next_node_index >= self.nodes.len()
    }

    // `Path.sameAs` (`Path.java`): nodes compare by position.
    fn nodes_match(&self, other: &Self) -> bool {
        self.nodes.len() == other.nodes.len()
            && self
                .nodes
                .iter()
                .zip(&other.nodes)
                .all(|(left, right)| left.pos == right.pos)
    }

    fn next_node(&self) -> Option<GroundPathNode> {
        self.nodes.get(self.next_node_index).copied()
    }

    fn advance(&mut self) {
        self.next_node_index = self.next_node_index.saturating_add(1);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MobPathingSnapshot {
    position: Vec3d,
    block_position: BlockPos,
    creaking_home_pos: Option<BlockPos>,
    bounding_box: Aabb,
    width: f32,
    height: f32,
    on_ground: bool,
    in_water: bool,
    in_liquid: bool,
    on_fire: bool,
    can_float: bool,
    can_walk_over_fences: bool,
    can_stand_on_lava: bool,
    can_open_doors: bool,
    can_pass_doors: bool,
    is_passenger: bool,
    speed: f32,
    /// `getHealth()` while `getTarget()` is set.
    target_health: Option<f32>,
    follow_range: f64,
    step_height: f64,
    max_fall_distance: i32,
    malus: PathfindingMalus,
    amphibious: bool,
    frog: bool,
    inherits_malus: bool,
}

impl MobPathingSnapshot {
    fn can_stand_on_fluid(self, level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
        if !self.can_stand_on_lava {
            return Some(false);
        }
        if !level.has_ticking_column_at(pos) {
            return None;
        }
        vanilla_block_state_block_type_id(level.block_state(pos))?;
        Some(crate::entity::level::fluid_state_tag_membership(
            level,
            block_state_fluid_state(level.block_state(pos)),
            "minecraft:lava",
        ))
    }

    pub(super) fn malus(self) -> PathfindingMalus {
        self.malus
    }
    pub(super) fn with_malus(mut self, malus: PathfindingMalus) -> Self {
        self.malus = malus;
        self
    }
    pub(crate) fn with_inherited_malus(mut self, inherited: bool) -> Self {
        self.inherits_malus = inherited;
        self
    }
    pub(super) fn pathfinding_malus(self, path_type: GroundPathType) -> f32 {
        self.malus.get(path_type)
    }

    pub(crate) fn from_entity(
        entity: &Entity,
        has_target: bool,
        is_passenger: bool,
        difficulty: Option<bbb_protocol::difficulties::Difficulty>,
        malus: PathfindingMalus,
    ) -> Option<Self> {
        let living = entity.living()?;
        let bounding_box = entity.bounding_box();
        Some(Self {
            malus,
            amphibious: false,
            frog: false,
            inherits_malus: false,
            position: entity.position(),
            block_position: entity.block_position(),
            creaking_home_pos: entity.creaking_home_pos(),
            bounding_box,
            width: (bounding_box.max_x - bounding_box.min_x) as f32,
            height: (bounding_box.max_y - bounding_box.min_y) as f32,
            on_ground: entity.on_ground(),
            in_water: entity.is_in_water(),
            in_liquid: entity.is_in_water() || entity.is_in_lava(),
            on_fire: entity.is_on_fire(),
            can_float: false,
            can_walk_over_fences: false,
            can_stand_on_lava: false,
            can_open_doors: false,
            can_pass_doors: true,
            is_passenger,
            speed: living.speed(),
            target_health: has_target.then(|| living.health()),
            follow_range: living.attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID),
            step_height: living.attribute_value(VANILLA_ATTRIBUTE_STEP_HEIGHT_ID),
            max_fall_distance: max_fall_distance(
                living.health(),
                living.max_health(),
                has_target,
                difficulty,
            ),
        })
    }

    pub(super) fn from_context(context: &super::MobAiContext) -> Self {
        Self {
            malus: context.nav.malus,
            amphibious: false,
            frog: false,
            inherits_malus: context.nav.inherits_malus,
            position: context.body.position,
            block_position: containing_block(context.body.position),
            creaking_home_pos: context.body.creaking_home_pos,
            bounding_box: context.body.bounding_box,
            width: (context.body.bounding_box.max_x - context.body.bounding_box.min_x) as f32,
            height: (context.body.bounding_box.max_y - context.body.bounding_box.min_y) as f32,
            on_ground: context.body.on_ground,
            in_water: context.body.water_fluid_height > 0.0,
            in_liquid: context.body.in_liquid,
            on_fire: context.body.on_fire,
            can_float: false,
            can_walk_over_fences: false,
            can_stand_on_lava: false,
            can_open_doors: false,
            can_pass_doors: true,
            is_passenger: context.body.is_passenger,
            speed: context.controls.speed,
            target_health: context
                .target
                .target
                .is_some()
                .then_some(context.body.health),
            follow_range: context.body.follow_range,
            step_height: context.body.step_height,
            max_fall_distance: max_fall_distance(
                context.body.health,
                context.body.max_health,
                context.target.target.is_some(),
                context.difficulty,
            ),
        }
    }
}

// `Mob.getMaxFallDistance` (`Mob.java`).
fn max_fall_distance(
    health: f32,
    max_health: f32,
    has_target: bool,
    difficulty: Option<bbb_protocol::difficulties::Difficulty>,
) -> i32 {
    if !has_target {
        return 3;
    }
    let mut sacrifice = (health - max_health * 0.33) as i32;
    sacrifice -= (3 - difficulty.map_or(2, |value| value.id())) * 4;
    sacrifice.max(0) + 3
}

// `GroundPathNavigation` over the shared `PathNavigation` state
// (`GroundPathNavigation.java`; `AmphibiousPathNavigation.java`).
#[derive(Debug, Clone, Default)]
pub(super) struct GroundNavigation {
    base: PathNavigationBase,
    amphibious: bool,
    frog: bool,
    wall_climber: bool,
    can_stand_on_lava: bool,
    wall_target: Option<BlockPos>,
    avoid_sun: bool,
    can_path_to_targets_below_surface: bool,
    tuning: GroundTuning,
    last_prepare: Cell<Option<AmphibiousPathPrepare>>,
}

impl std::ops::Deref for GroundNavigation {
    type Target = PathNavigationBase;

    fn deref(&self) -> &PathNavigationBase {
        &self.base
    }
}

impl std::ops::DerefMut for GroundNavigation {
    fn deref_mut(&mut self) -> &mut PathNavigationBase {
        &mut self.base
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct AmphibiousPathPrepare {
    walkable_before: f32,
    border_before: f32,
}

impl AmphibiousPathPrepare {
    pub(super) fn apply(self, malus: &mut PathfindingMalus) {
        malus.set(GroundPathType::Water, 0.0);
        malus.set(GroundPathType::Walkable, self.walkable_before);
        malus.set(GroundPathType::WaterBorder, self.border_before);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct NavigationMove {
    pub(super) wanted: Vec3d,
    pub(super) speed_modifier: f64,
}

impl GroundNavigation {
    pub(super) fn stop(&mut self) {
        self.base.stop();
        self.wall_target = None;
    }

    // `WallClimberNavigation.tick`.
    fn wall_climber_move(&mut self, mob: MobPathingSnapshot) -> Option<NavigationMove> {
        let target = self.wall_target?;
        let center = Vec3d {
            x: f64::from(target.x) + 0.5,
            y: f64::from(target.y) + 0.5,
            z: f64::from(target.z) + 0.5,
        };
        let width_sqr = f64::from(mob.width).powi(2);
        let above_center = Vec3d {
            y: mob.position.y.floor() + 0.5,
            ..center
        };
        if distance_sqr(mob.position, center) < width_sqr
            || mob.position.y > f64::from(target.y)
                && distance_sqr(mob.position, above_center) < width_sqr
        {
            self.wall_target = None;
            return None;
        }
        Some(NavigationMove {
            wanted: Vec3d {
                x: f64::from(target.x),
                y: f64::from(target.y),
                z: f64::from(target.z),
            },
            speed_modifier: self.base.speed_modifier,
        })
    }

    pub(super) fn set_frog(&mut self) {
        self.amphibious = true;
        self.frog = true;
    }

    pub(super) fn set_amphibious(&mut self) {
        self.amphibious = true;
    }

    pub(super) fn frog_enabled(&self) -> bool {
        self.frog
    }

    pub(super) fn take_amphibious_prepare(&self) -> Option<AmphibiousPathPrepare> {
        self.last_prepare.take()
    }

    pub(super) fn set_can_float(&mut self, can_float: bool) {
        if !self.amphibious {
            self.base.can_float = can_float;
        }
    }

    pub(super) fn apply_tuning(&mut self, entity_type_id: i32) {
        self.tuning = super::spec::mob_spec(entity_type_id)
            .map_or(GroundTuning::DEFAULT, |spec| spec.ground_tuning);
        if self.tuning.can_float {
            self.set_can_float(true);
        }
    }

    // `PathNavigation.getMaxPathLength`.
    fn max_path_length(&self, mob: MobPathingSnapshot) -> f32 {
        (mob.follow_range as f32).max(self.tuning.required_path_length.unwrap_or(16.0))
    }

    /// The malus the search reads, after the evaluator's `prepare` hooks.
    fn search_mob(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
    ) -> Option<MobPathingSnapshot> {
        let mut search_mob = mob;
        if self.amphibious && !mob.inherits_malus {
            level.sea_level()?;
            search_mob.malus.set(GroundPathType::Water, 0.0);
            search_mob.malus.set(GroundPathType::Walkable, 6.0);
            search_mob.malus.set(GroundPathType::WaterBorder, 4.0);
        }
        if self.frog && mob.inherits_malus {
            level.sea_level()?;
        }
        if self.tuning.water_while_burning_or_wet && (mob.on_fire || mob.in_water) {
            search_mob.malus.set(GroundPathType::Water, 0.0);
        }
        Some(search_mob)
    }

    // `GroundPathNavigation.setAvoidSun`.
    pub(super) fn set_avoid_sun(&mut self, avoid_sun: bool) {
        self.avoid_sun = avoid_sun;
    }

    // `GroundPathNavigation.setCanPathToTargetsBelowSurface` (`GroundPathNavigation.java:155-157`).
    pub(super) fn set_can_path_to_targets_below_surface(&mut self, value: bool) {
        self.can_path_to_targets_below_surface = value;
    }

    /// `moveTo` against a freshly created path, leaving the current one
    /// alone when no path is found.
    pub(super) fn try_move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        match self.create_path(level, mob, target, reach_range) {
            Some(path) => self.start_path(level, path, mob, speed_modifier),
            None => false,
        }
    }

    // `PathNavigation.java`.
    pub(super) fn set_can_open_doors(&mut self, can_open_doors: bool) {
        self.base.can_open_doors = can_open_doors;
    }

    /// `MoveThroughVillageGoal.canUse`'s path:
    /// it ends at the first wooden door, and the navigation opens doors after.
    pub(super) fn move_through_village(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        speed_modifier: f64,
        can_break_doors: bool,
    ) {
        self.set_can_open_doors(can_break_doors);
        let mut path = self.create_path(level, mob, target, 0);
        let door = path.as_ref().and_then(|path| {
            path.nodes.iter().map(|node| node.pos).find(|pos| {
                super::door::is_wooden_door(level.block_state(BlockPos {
                    y: pos.y + 1,
                    ..*pos
                }))
            })
        });
        if let Some(door) = door {
            path = self.create_path(level, mob, door, 0);
        }
        if let Some(path) = path {
            self.start_path(level, path, mob, speed_modifier);
        }
        self.set_can_open_doors(true);
    }

    fn configure_mob(&self, mob: MobPathingSnapshot) -> MobPathingSnapshot {
        let mut mob = self.base.configure_mob(mob);
        mob.amphibious = self.amphibious;
        mob.frog = self.frog;
        mob.can_stand_on_lava = self.can_stand_on_lava;
        mob.can_walk_over_fences = self.tuning.can_walk_over_fences;
        if self.tuning.creeper_fall {
            mob.max_fall_distance = mob
                .target_health
                .map_or(3, |health| (health - 1.0 + 3.0).floor() as i32);
        }
        mob
    }

    fn can_update_path(mob: MobPathingSnapshot) -> bool {
        mob.amphibious || mob.on_ground || mob.in_liquid || mob.is_passenger
    }

    // `PathNavigation.moveTo` and path reuse (`PathNavigation.java`).
    pub(super) fn move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let Some(path) = self.create_path(level, mob, target, reach_range) else {
            self.base.path = None;
            return false;
        };
        self.start_path(level, path, mob, speed_modifier)
    }

    pub(super) fn move_to_entity(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let Some(path) = self.create_path(level, mob, target, reach_range) else {
            if self.wall_climber {
                self.base.speed_modifier = speed_modifier;
                return true;
            }
            return false;
        };
        self.start_path(level, path, mob, speed_modifier)
    }

    pub(super) fn start_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        path: GroundPath,
        mob: MobPathingSnapshot,
        speed_modifier: f64,
    ) -> bool {
        let mob = self.configure_mob(mob);
        let temp_position = ground_temp_position(level, mob);
        if !self
            .base
            .move_to_path(level, Some(path), temp_position, speed_modifier)
        {
            return false;
        }
        if self.avoid_sun {
            self.trim_sunlit(level, mob.position);
            if self.base.is_done() {
                return false;
            }
        }
        true
    }

    // `GroundPathNavigation.trimPath`.
    fn trim_sunlit(&mut self, level: &LevelCtx<'_, '_>, position: Vec3d) {
        let standing = containing_block(Vec3d {
            y: position.y + 0.5,
            ..position
        });
        let Some(path) = self.base.path.as_mut() else {
            return;
        };
        if level.can_see_sky(standing) {
            return;
        }
        if let Some(index) = path
            .nodes
            .iter()
            .position(|node| level.can_see_sky(node.pos))
        {
            path.nodes.truncate(index);
        }
    }

    pub(super) fn create_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
    ) -> Option<GroundPath> {
        if self.wall_climber {
            self.wall_target = Some(target);
        }
        self.last_prepare.set(None);
        let mob = self.configure_mob(mob);
        if mob.position.y < f64::from(level.min_y())
            || !Self::can_update_path(mob)
            || !level.has_ticking_column_at(target)
        {
            return None;
        }
        let target = if self.amphibious || self.can_path_to_targets_below_surface {
            target
        } else {
            surface_target(level, target)?
        };
        if self
            .base
            .path
            .as_ref()
            .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target))
        {
            return self.base.path.clone();
        }
        let max_path_length = self.max_path_length(mob);
        let max_visited = (max_path_length * 16.0).floor() as usize;
        let search_mob = self.search_mob(level, mob)?;
        let (path, search_completed) = find_path_with_start_status(
            level,
            search_mob,
            std::slice::from_ref(&target),
            max_path_length,
            reach_range,
            max_visited,
            self.base.capture_debug,
        );
        if self.amphibious && search_completed {
            self.last_prepare.set(Some(AmphibiousPathPrepare {
                walkable_before: mob.pathfinding_malus(GroundPathType::Walkable),
                border_before: mob.pathfinding_malus(GroundPathType::WaterBorder),
            }));
        }
        let path = path?;
        self.base.accept_created_path(&path, reach_range);
        Some(path)
    }

    // `PathNavigation.java`.
    pub(super) fn tick(
        &mut self,
        level: &LevelCtx<'_, '_>,
        context: &mut super::MobAiContext,
    ) -> Option<NavigationMove> {
        self.base.tick = self.base.tick.wrapping_add(1);
        let mob = self.configure_mob(MobPathingSnapshot::from_context(context));
        if self.base.delayed_recomputation {
            self.recompute(level, mob);
        }
        if self.base.is_done() {
            return self.wall_climber_move(mob);
        }
        if Self::can_update_path(mob) {
            let temp_position = ground_temp_position(level, mob);
            // `FrogPathNavigation.canCutCorner` (`Frog.java`).
            let frog = mob.frog;
            self.base.follow_the_path(
                mob,
                temp_position,
                level.game_time(),
                |path_type| {
                    path_type.can_cut_corner()
                        && !(frog && path_type == GroundPathType::WaterBorder)
                },
                |start, stop| {
                    mob.amphibious
                        && mob.in_liquid
                        && can_move_directly(level, mob, false, start, stop)
                },
            );
        } else {
            let temp_position = ground_temp_position(level, mob);
            let should_advance = self
                .base
                .path
                .as_ref()
                .and_then(GroundPath::next_node)
                .is_some_and(|next| {
                    let target = node_entity_position(mob, next.pos);
                    temp_position.y > target.y
                        && !mob.on_ground
                        && temp_position.x.floor() == target.x.floor()
                        && temp_position.z.floor() == target.z.floor()
                });
            if should_advance {
                self.base.path.as_mut().expect("checked path").advance();
            }
        }
        if let Some(next) = self.base.path.as_ref().and_then(GroundPath::next_node) {
            let Some(y) = (if mob.amphibious {
                Some(f64::from(next.pos.y))
            } else {
                ground_y(level, next.pos)
            }) else {
                self.base.stop();
                return None;
            };
            let mut target = node_entity_position(mob, next.pos);
            target.y = y;
            return Some(NavigationMove {
                wanted: target,
                speed_modifier: self.base.speed_modifier,
            });
        }
        None
    }

    // `PathNavigation.java`.
    pub(super) fn recompute(&mut self, level: &LevelCtx<'_, '_>, mob: MobPathingSnapshot) {
        let mob = self.configure_mob(mob);
        let Some((target, reach_range)) = self
            .base
            .recompute_target(level.game_time(), Self::can_update_path(mob))
        else {
            return;
        };
        self.base.path = None;
        let path = self.create_path(level, mob, target, reach_range);
        self.base.finish_recompute(level.game_time(), path);
    }

    /// `PathNavigation.createPath(Entity, int)` (`PathNavigation.java`)
    /// over `createPath(BlockPos, int)` (`:124-126`): a path for the target
    /// block, computed without starting one, so a caller that only asks
    /// `Path.getDistToTarget` leaves the navigation's own route alone
    /// (`ShootTongue.canPathfindToTarget`, `ShootTongue.java`).
    ///
    /// `max_path_length` is Vanilla's `maxPathLength` argument
    /// (`PathNavigation.java`); `None` is `getMaxPathLength()` (`:79-81`).
    pub(super) fn create_path_to(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        max_path_length: Option<f32>,
    ) -> Option<GroundPath> {
        self.last_prepare.set(None);
        let mob = self.configure_mob(mob);
        if mob.position.y < f64::from(level.min_y())
            || !Self::can_update_path(mob)
            || !level.has_ticking_column_at(target)
        {
            return None;
        }
        let target = if self.amphibious || self.can_path_to_targets_below_surface {
            target
        } else {
            surface_target(level, target)?
        };
        if self.frog
            && self
                .base
                .path
                .as_ref()
                .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target))
        {
            return self.base.path.clone();
        }
        self.search_path_to_targets(
            level,
            mob,
            std::slice::from_ref(&target),
            reach_range,
            max_path_length,
        )
    }

    // PathNavigation: the Set overload keeps the original target positions.
    pub(super) fn create_path_to_targets(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        targets: &[BlockPos],
        reach_range: i32,
        max_path_length: Option<f32>,
    ) -> Option<GroundPath> {
        self.last_prepare.set(None);
        let mob = self.configure_mob(mob);
        if mob.position.y < f64::from(level.min_y()) || !Self::can_update_path(mob) {
            return None;
        }
        self.search_path_to_targets(level, mob, targets, reach_range, max_path_length)
    }

    fn search_path_to_targets(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        targets: &[BlockPos],
        reach_range: i32,
        max_path_length: Option<f32>,
    ) -> Option<GroundPath> {
        let max_path_length = max_path_length.unwrap_or_else(|| self.max_path_length(mob));
        let max_visited = (max_path_length * 16.0).floor() as usize;
        let search_mob = self.search_mob(level, mob)?;
        let (path, search_completed) = find_path_with_start_status(
            level,
            search_mob,
            targets,
            max_path_length,
            reach_range,
            max_visited,
            false,
        );
        if self.amphibious && search_completed {
            self.last_prepare.set(Some(AmphibiousPathPrepare {
                walkable_before: mob.pathfinding_malus(GroundPathType::Walkable),
                border_before: mob.pathfinding_malus(GroundPathType::WaterBorder),
            }));
        }
        path
    }
}

const fn debug_path_type(path_type: GroundPathType) -> DebugPathType {
    match path_type {
        GroundPathType::Blocked => DebugPathType::Blocked,
        GroundPathType::Open => DebugPathType::Open,
        GroundPathType::Walkable => DebugPathType::Walkable,
        GroundPathType::WalkableDoor => DebugPathType::WalkableDoor,
        GroundPathType::Trapdoor => DebugPathType::Trapdoor,
        GroundPathType::PowderSnow => DebugPathType::PowderSnow,
        GroundPathType::OnTopOfPowderSnow => DebugPathType::OnTopOfPowderSnow,
        GroundPathType::Fence => DebugPathType::Fence,
        GroundPathType::Lava => DebugPathType::Lava,
        GroundPathType::Water => DebugPathType::Water,
        GroundPathType::WaterBorder => DebugPathType::WaterBorder,
        GroundPathType::Rail => DebugPathType::Rail,
        GroundPathType::UnpassableRail => DebugPathType::UnpassableRail,
        GroundPathType::FireInNeighbor => DebugPathType::FireInNeighbor,
        GroundPathType::Fire => DebugPathType::Fire,
        GroundPathType::DamagingInNeighbor => DebugPathType::DamagingInNeighbor,
        GroundPathType::Damaging => DebugPathType::Damaging,
        GroundPathType::DoorOpen => DebugPathType::DoorOpen,
        GroundPathType::DoorWoodClosed => DebugPathType::DoorWoodClosed,
        GroundPathType::DoorIronClosed => DebugPathType::DoorIronClosed,
        GroundPathType::Breach => DebugPathType::Breach,
        GroundPathType::Leaves => DebugPathType::Leaves,
        GroundPathType::StickyHoney => DebugPathType::StickyHoney,
        GroundPathType::Cocoa => DebugPathType::Cocoa,
        GroundPathType::DamageCautious => DebugPathType::DamageCautious,
        GroundPathType::OnTopOfTrapdoor => DebugPathType::OnTopOfTrapdoor,
        GroundPathType::BigMobsCloseToDanger => DebugPathType::BigMobsCloseToDanger,
    }
}

fn node_entity_position(mob: MobPathingSnapshot, pos: BlockPos) -> Vec3d {
    let offset = f64::from((mob.width + 1.0).floor()) * 0.5;
    Vec3d {
        x: f64::from(pos.x) + offset,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + offset,
    }
}

fn can_move_directly(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    fluids: bool,
    start: Vec3d,
    stop: Vec3d,
) -> bool {
    let to = Vec3d {
        y: stop.y + f64::from(mob.height) * 0.5,
        ..stop
    };
    if fluids {
        crate::entity::projectile_hit::collision_and_fluid_clip_misses(level, start, to)
    } else {
        crate::entity::projectile_hit::collision_clip_misses(level, start, to)
    }
}

fn ground_temp_position(level: &LevelCtx<'_, '_>, mob: MobPathingSnapshot) -> Vec3d {
    if mob.amphibious {
        return Vec3d {
            x: mob.position.x,
            y: mob.position.y + f64::from(mob.height) * 0.5,
            z: mob.position.z,
        };
    }
    // `GroundPathNavigation.java`.
    let y = if mob.in_water && mob.can_float {
        let mut surface = mob.block_position.y;
        let mut steps = 0;
        while steps <= 16 {
            let pos = BlockPos {
                y: surface,
                ..mob.block_position
            };
            let Some(is_water) = is_water_block(level, pos) else {
                return Vec3d {
                    x: mob.position.x,
                    y: f64::from(mob.block_position.y),
                    z: mob.position.z,
                };
            };
            if !is_water {
                break;
            }
            surface += 1;
            steps += 1;
        }
        if steps > 16 {
            f64::from(mob.block_position.y)
        } else {
            f64::from(surface)
        }
    } else {
        (mob.position.y + 0.5).floor()
    };
    Vec3d {
        x: mob.position.x,
        y,
        z: mob.position.z,
    }
}

// State: `MoveControl.java`; tick: `MoveControl.java`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum MoveOperation {
    Wait,
    MoveTo,
    MoveToDirect,
    Strafe,
    Jumping,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum MoveControlKind {
    #[default]
    Ground,
    SmoothSwimming,
    NautilusSwimming,
    AxolotlSwimming,
    Fish,
    Guardian,
    Drowned,
    Flying(FlyingMoveControlSpec),
}

#[derive(Debug, Clone)]
pub(super) struct MoveControl {
    wanted: Vec3d,
    speed_modifier: f64,
    strafe_forwards: f32,
    strafe_right: f32,
    operation: MoveOperation,
    kind: MoveControlKind,
}

impl Default for MoveControl {
    fn default() -> Self {
        Self {
            wanted: Vec3d::default(),
            speed_modifier: 0.0,
            strafe_forwards: 0.0,
            strafe_right: 0.0,
            operation: MoveOperation::Wait,
            kind: MoveControlKind::Ground,
        }
    }
}

impl MoveControl {
    pub(super) fn wanted_movement(&self) -> NavigationMove {
        NavigationMove {
            wanted: self.wanted,
            speed_modifier: self.speed_modifier,
        }
    }

    pub(super) fn tick_rabbit(
        &mut self,
        level: &LevelCtx<'_, '_>,
        context: &mut super::MobAiContext,
        wants_jump: bool,
    ) {
        if context.body.in_water && self.has_move_to() {
            self.speed_modifier = 1.5;
        }
        let next_jump_speed = self.speed_modifier;
        if context.body.on_ground
            && !context.controls.jumping
            && !wants_jump
            && !context.controls.jump_requested
        {
            self.speed_modifier = 0.0;
        }
        self.tick(level, context);
        self.speed_modifier = next_jump_speed;
    }

    pub(super) fn has_move_to(&self) -> bool {
        self.operation == MoveOperation::MoveTo
    }

    pub(super) fn set_kind(&mut self, kind: MoveControlKind) {
        self.kind = kind;
    }

    pub(super) fn guardian_moving(&self, navigation_done: bool) -> Option<bool> {
        matches!(self.kind, MoveControlKind::Guardian)
            .then_some(self.operation == MoveOperation::MoveTo && !navigation_done)
    }

    pub(super) fn set_direct_position(&mut self, wanted: Vec3d, speed_modifier: f64) {
        self.set_wanted_position(wanted, speed_modifier);
        self.operation = MoveOperation::MoveToDirect;
    }

    pub(super) fn set_wanted_position(&mut self, wanted: Vec3d, speed_modifier: f64) {
        self.wanted = wanted;
        self.speed_modifier = speed_modifier;
        if self.operation != MoveOperation::Jumping {
            self.operation = MoveOperation::MoveTo;
        }
    }

    pub(super) fn strafe(&mut self, forwards: f32, right: f32) {
        self.operation = MoveOperation::Strafe;
        self.strafe_forwards = forwards;
        self.strafe_right = right;
        self.speed_modifier = 0.25;
    }

    pub(super) fn tick(&mut self, level: &LevelCtx<'_, '_>, context: &mut super::MobAiContext) {
        match self.kind {
            MoveControlKind::Fish => return self.tick_fish(context),
            MoveControlKind::SmoothSwimming
            | MoveControlKind::NautilusSwimming
            | MoveControlKind::AxolotlSwimming => {
                return self.tick_smooth_swimming(context);
            }
            MoveControlKind::Guardian => return self.tick_guardian(context),
            MoveControlKind::Flying(spec) => return self.tick_flying(spec, context),
            MoveControlKind::Drowned => {
                if self.tick_drowned(context) {
                    return;
                }
            }
            MoveControlKind::Ground => {}
        }
        match self.operation {
            MoveOperation::Strafe => self.tick_strafe(level, context),
            MoveOperation::MoveTo | MoveOperation::MoveToDirect => {
                self.tick_move_to(level, context)
            }
            MoveOperation::Jumping => {
                context.controls.speed = (self.speed_modifier * context.body.movement_speed) as f32;
                context.controls.zza = context.controls.speed;
                if context.body.on_ground || context.body.in_liquid {
                    self.operation = MoveOperation::Wait;
                }
            }
            MoveOperation::Wait => context.controls.zza = 0.0,
        }
    }

    fn tick_drowned(&mut self, context: &mut super::MobAiContext) -> bool {
        if !context.body.in_water || !context.species.drowned_swim.wants_to_swim {
            if !context.body.on_ground {
                context.controls.swimming_impulse.y -= 0.008;
            }
            return false;
        }
        if context.species.drowned_swim.rise {
            context.controls.swimming_impulse.y += 0.002;
        }
        if self.operation != MoveOperation::MoveTo || context.nav.navigation_done {
            context.controls.speed = 0.0;
            context.controls.zza = 0.0;
            return true;
        }
        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        let distance = (xd * xd + yd * yd + zd * zd).sqrt();
        if distance < 1.0e-6 {
            return true;
        }
        let yaw = (crate::entity::projectile_shoot::mth_atan2(zd, xd) * f64::from(180.0_f32)
            / f64::from(std::f32::consts::PI)) as f32
            - 90.0;
        context.body.y_rot = rotlerp(context.body.y_rot, yaw, 90.0);
        context.body.y_body_rot = context.body.y_rot;
        let target_speed = (self.speed_modifier * context.body.movement_speed) as f32;
        context.controls.speed += (target_speed - context.controls.speed) * 0.125;
        context.controls.zza = context.controls.speed;
        context.controls.swimming_impulse.x += f64::from(context.controls.speed) * xd * 0.005;
        context.controls.swimming_impulse.y +=
            f64::from(context.controls.speed) * yd / distance * 0.1;
        context.controls.swimming_impulse.z += f64::from(context.controls.speed) * zd * 0.005;
        true
    }

    fn tick_fish(&mut self, context: &mut super::MobAiContext) {
        if context.body.eye_in_water {
            context.controls.swimming_impulse.y += 0.005;
        }
        if self.operation != MoveOperation::MoveTo || context.nav.navigation_done {
            context.controls.speed = 0.0;
            context.controls.zza = 0.0;
            return;
        }
        let target_speed = (self.speed_modifier * context.body.movement_speed) as f32;
        context.controls.speed += (target_speed - context.controls.speed) * 0.125;
        context.controls.zza = context.controls.speed;
        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        if yd != 0.0 {
            let distance = (xd * xd + yd * yd + zd * zd).sqrt();
            context.controls.swimming_impulse.y +=
                f64::from(context.controls.speed) * yd / distance * 0.1;
        }
        if xd != 0.0 || zd != 0.0 {
            let angle = (crate::entity::projectile_shoot::mth_atan2(zd, xd) * f64::from(180.0_f32)
                / f64::from(std::f32::consts::PI)) as f32
                - 90.0;
            context.body.y_rot = rotlerp(context.body.y_rot, angle, 90.0);
            context.body.y_body_rot = context.body.y_rot;
        }
    }

    fn tick_guardian(&mut self, context: &mut super::MobAiContext) {
        if self.operation != MoveOperation::MoveTo || context.nav.navigation_done {
            context.controls.speed = 0.0;
            context.controls.zza = 0.0;
            return;
        }
        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        let distance = (xd * xd + yd * yd + zd * zd).sqrt();
        if distance < 1.0e-6 {
            return;
        }
        let yaw = (crate::entity::projectile_shoot::mth_atan2(zd, xd) * f64::from(180.0_f32)
            / f64::from(std::f32::consts::PI)) as f32
            - 90.0;
        context.body.y_rot = rotlerp(context.body.y_rot, yaw, 90.0);
        context.body.y_body_rot = context.body.y_rot;
        let target_speed = (self.speed_modifier * context.body.movement_speed) as f32;
        context.controls.speed += (target_speed - context.controls.speed) * 0.125;
        context.controls.zza = context.controls.speed;
        let phase = f64::from(context.tick_count);
        let push = (phase * 0.5).sin() * 0.05;
        let radians = f64::from(context.body.y_rot * (std::f32::consts::PI / 180.0));
        let (sin, cos) = radians.sin_cos();
        let y_push = (phase * 0.75).sin() * 0.05;
        context.controls.swimming_impulse.x += push * cos;
        context.controls.swimming_impulse.y +=
            y_push * (sin + cos) * 0.25 + f64::from(context.controls.speed) * yd / distance * 0.1;
        context.controls.swimming_impulse.z += push * sin;
        context.set_look_at_with_limits(
            context.body.position.x + xd / distance * 2.0,
            context.body.eye_y + yd / distance / distance,
            context.body.position.z + zd / distance * 2.0,
            10.0,
            40.0,
        );
    }

    fn tick_strafe(&mut self, level: &LevelCtx<'_, '_>, context: &mut super::MobAiContext) {
        let speed = (self.speed_modifier * context.body.movement_speed) as f32;
        let mut xa = self.strafe_forwards;
        let mut za = self.strafe_right;
        let distance = (xa * xa + za * za).sqrt().max(1.0);
        xa *= speed / distance;
        za *= speed / distance;
        let radians = context.body.y_rot.to_radians();
        let dx = xa * radians.cos() - za * radians.sin();
        let dz = za * radians.cos() + xa * radians.sin();
        let ahead = containing_block(Vec3d {
            x: context.body.position.x + f64::from(dx),
            y: context.body.position.y,
            z: context.body.position.z + f64::from(dz),
        });
        if walk_path_type(level, ahead) != Some(GroundPathType::Walkable) {
            self.strafe_forwards = 1.0;
            self.strafe_right = 0.0;
        }
        context.controls.speed = speed;
        context.controls.zza = self.strafe_forwards;
        context.controls.xxa = self.strafe_right;
        self.operation = MoveOperation::Wait;
    }

    fn tick_move_to(&mut self, level: &LevelCtx<'_, '_>, context: &mut super::MobAiContext) {
        self.operation = MoveOperation::Wait;
        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        if xd * xd + yd * yd + zd * zd < 2.500_000_3e-7 {
            context.controls.zza = 0.0;
            return;
        }
        let wanted_rot = (zd.atan2(xd).to_degrees() as f32) - 90.0;
        context.body.y_rot = rotlerp(context.body.y_rot, wanted_rot, 90.0);
        context.controls.speed = (self.speed_modifier * context.body.movement_speed) as f32;
        // `Mob.setSpeed` also writes `zza` (`Mob.java`).
        context.controls.zza = context.controls.speed;
        let pos = containing_block(context.body.position);
        let state = level.block_state(pos);
        let shape_top = if let Some(boxes) =
            bbb_server_types::geometry::static_block_collision_box_iter(state)
        {
            boxes
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap_or(0.0)
        } else {
            level
                .block_collision_boxes(pos)
                .iter()
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap_or(0.0)
        };
        let block = vanilla_block_state_block_type_id(state).and_then(vanilla_block_type);
        let tagged = |tag| {
            block.is_some_and(|block| {
                level.registry_tag_membership("minecraft:block", tag, block.resource_id)
            })
        };
        if yd > context.body.step_height
            && xd * xd + zd * zd < f64::from(context.body.width.max(1.0))
            || shape_top > 0.0
                && context.body.position.y < f64::from(pos.y) + shape_top
                && !tagged("minecraft:doors")
                && !tagged("minecraft:fences")
        {
            context.request_jump();
            self.operation = MoveOperation::Jumping;
        }
    }
}

pub(crate) fn ground_path_type_from_state(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<GroundPathType> {
    // `WalkNodeEvaluator.java`.
    if !level.has_ticking_column_at(pos) {
        return None;
    }
    let state_id = level.block_state(pos);
    let Some(facts) = bbb_protocol::block_state_types::vanilla_block_state_path_facts(state_id)
    else {
        let (block, _) = level.content_registry().block_by_state_id(state_id)?;
        let tagged =
            |tag| level.registry_tag_membership("minecraft:block", tag, block.resource_id());
        if tagged("minecraft:trapdoors") {
            return Some(GroundPathType::Trapdoor);
        }
        if tagged("minecraft:fire") {
            return Some(GroundPathType::Fire);
        }
        if tagged("minecraft:fences") || tagged("minecraft:walls") {
            return Some(GroundPathType::Fence);
        }
        return Some(if block.full_cube_collision {
            GroundPathType::Blocked
        } else {
            GroundPathType::Open
        });
    };
    if facts.is_air() {
        return Some(GroundPathType::Open);
    }
    let block_type_id = vanilla_block_state_block_type_id(state_id)?;
    let block = vanilla_block_type(block_type_id)?;
    let tagged = |tag| level.registry_tag_membership("minecraft:block", tag, block.resource_id);
    if tagged("minecraft:trapdoors")
        || matches!(
            block_type_id,
            block_types::VANILLA_BLOCK_LILY_PAD_ID | block_types::VANILLA_BLOCK_BIG_DRIPLEAF_ID
        )
    {
        return Some(GroundPathType::Trapdoor);
    }
    match block_type_id {
        block_types::VANILLA_BLOCK_POWDER_SNOW_ID => return Some(GroundPathType::PowderSnow),
        block_types::VANILLA_BLOCK_CACTUS_ID | block_types::VANILLA_BLOCK_SWEET_BERRY_BUSH_ID => {
            return Some(GroundPathType::Damaging)
        }
        block_types::VANILLA_BLOCK_HONEY_BLOCK_ID => return Some(GroundPathType::StickyHoney),
        block_types::VANILLA_BLOCK_COCOA_ID => return Some(GroundPathType::Cocoa),
        block_types::VANILLA_BLOCK_WITHER_ROSE_ID
        | block_types::VANILLA_BLOCK_POINTED_DRIPSTONE_ID => {
            return Some(GroundPathType::DamageCautious);
        }
        _ => {}
    }
    let fluid = bbb_server_types::fluid::block_state_fluid_state(state_id);
    let fluid_tagged =
        |tag| level.registry_tag_membership("minecraft:fluid", tag, fluid.identity().resource_id());
    if fluid_tagged("minecraft:lava") {
        return Some(GroundPathType::Lava);
    }
    if tagged("minecraft:fire")
        || matches!(
            block_type_id,
            block_types::VANILLA_BLOCK_LAVA_ID
                | block_types::VANILLA_BLOCK_MAGMA_BLOCK_ID
                | block_types::VANILLA_BLOCK_LAVA_CAULDRON_ID
        )
        || facts.is_lit_campfire()
    {
        return Some(GroundPathType::Fire);
    }
    if facts.is_door() {
        return Some(if facts.door_open() {
            GroundPathType::DoorOpen
        } else if facts.door_can_open_by_hand() {
            GroundPathType::DoorWoodClosed
        } else {
            GroundPathType::DoorIronClosed
        });
    }
    let fence = tagged("minecraft:fences");
    let wall = tagged("minecraft:walls");
    let water = fluid_tagged("minecraft:water");
    if facts.is_rail() {
        return Some(GroundPathType::Rail);
    }
    if facts.is_leaves() {
        return Some(GroundPathType::Leaves);
    }
    if fence || wall || facts.is_fence_gate() && !facts.fence_gate_open() {
        return Some(GroundPathType::Fence);
    }
    Some(if facts.land_pathfindable() {
        if water {
            GroundPathType::Water
        } else {
            GroundPathType::Open
        }
    } else {
        GroundPathType::Blocked
    })
}

/// `WalkNodeEvaluator.getPathTypeStatic(context, pos)`
/// (`WalkNodeEvaluator.java`) over
/// `PathfindingContext.getPathTypeFromState` (`PathfindingContext.java`).
pub(crate) fn walk_path_type(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<GroundPathType> {
    let direct = level.ground_path_type_from_state(pos)?;
    if direct != GroundPathType::Open || pos.y < level.min_y() + 1 {
        return Some(direct);
    }
    let below = level.ground_path_type_from_state(BlockPos {
        y: pos.y - 1,
        ..pos
    })?;
    Some(match below {
        GroundPathType::Open
        | GroundPathType::Water
        | GroundPathType::Lava
        | GroundPathType::Walkable => GroundPathType::Open,
        GroundPathType::Fire => GroundPathType::Fire,
        GroundPathType::Damaging => GroundPathType::Damaging,
        GroundPathType::StickyHoney => GroundPathType::StickyHoney,
        GroundPathType::PowderSnow => GroundPathType::OnTopOfPowderSnow,
        GroundPathType::DamageCautious => GroundPathType::DamageCautious,
        GroundPathType::Trapdoor => GroundPathType::OnTopOfTrapdoor,
        _ => neighbor_path_type(level, pos, GroundPathType::Walkable)?,
    })
}

fn neighbor_path_type(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    base_type: GroundPathType,
) -> Option<GroundPathType> {
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let value = level.ground_path_type_from_state(BlockPos {
                    x: pos.x + dx,
                    y: pos.y + dy,
                    z: pos.z + dz,
                })?;
                match value {
                    GroundPathType::Damaging => return Some(GroundPathType::DamagingInNeighbor),
                    GroundPathType::Fire | GroundPathType::Lava => {
                        return Some(GroundPathType::FireInNeighbor);
                    }
                    GroundPathType::Water => return Some(GroundPathType::WaterBorder),
                    GroundPathType::DamageCautious => return Some(GroundPathType::DamageCautious),
                    _ => {}
                }
            }
        }
    }
    Some(base_type)
}

fn surface_target(level: &LevelCtx<'_, '_>, mut pos: BlockPos) -> Option<BlockPos> {
    let is_air = |pos| {
        let state = level.block_state(pos);
        if let Some(facts) = bbb_protocol::block_state_types::vanilla_block_state_path_facts(state)
        {
            Some(facts.is_air())
        } else {
            level.content_registry().block_by_state_id(state)?;
            Some(false)
        }
    };
    let is_solid = |pos| {
        bbb_server_types::fluid::block_state_is_solid_with_registry(
            level.block_state(pos),
            level.content_registry(),
        )
    };
    if is_air(pos)? {
        let mut below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        while below.y >= level.min_y() && is_air(below)? {
            below.y -= 1;
        }
        if below.y >= level.min_y() {
            return Some(BlockPos {
                y: below.y + 1,
                ..below
            });
        }
        pos.y += 1;
        while pos.y <= level.max_y() && is_air(pos)? {
            pos.y += 1;
        }
    }
    if !is_solid(pos) {
        return Some(pos);
    }
    while pos.y <= level.max_y() && is_solid(pos) {
        pos.y += 1;
    }
    (pos.y <= level.max_y()).then_some(pos)
}

/// `PathNavigation.getGroundY` (`PathNavigation.java:242-245`).
fn ground_y(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<f64> {
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if !level.has_ticking_column_at(below) {
        return None;
    }
    if bbb_server_types::fluid::block_state_is_air(level.block_state(below)) {
        Some(f64::from(pos.y))
    } else {
        floor_level(level, pos)
    }
}

fn floor_level(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<f64> {
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if !level.has_ticking_column_at(below) {
        return None;
    }
    let state = level.block_state(below);
    let top =
        if let Some(boxes) = bbb_server_types::geometry::static_block_collision_box_iter(state) {
            boxes
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap_or(0.0)
        } else {
            level
                .block_collision_boxes(below)
                .iter()
                .map(|shape| shape.max_y)
                .reduce(f64::max)
                .unwrap_or(0.0)
        };
    Some(f64::from(below.y) + top)
}

fn evaluator_floor_level(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
) -> Option<f64> {
    // `WalkNodeEvaluator.java`.
    if (mob.can_float || mob.amphibious) && fluid_is_water(level, pos)? {
        Some(f64::from(pos.y) + 0.5)
    } else {
        floor_level(level, pos)
    }
}

fn is_water_block(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    Some(vanilla_block_state_block_type_id(level.block_state(pos))? == VANILLA_BLOCK_WATER_ID)
}

fn is_water_start_state(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    if is_water_block(level, pos)? {
        return Some(true);
    }
    Some(matches!(
        block_state_fluid_state(level.block_state(pos)),
        FluidState::Source {
            family: FluidFamily::Water,
            falling: false
        }
    ))
}

fn fluid_is_water(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    vanilla_block_state_block_type_id(level.block_state(pos))?;
    let fluid = block_state_fluid_state(level.block_state(pos));
    if fluid.is_empty() {
        return Some(false);
    }
    Some(level.registry_tag_membership(
        "minecraft:fluid",
        "minecraft:water",
        fluid.identity().resource_id(),
    ))
}

pub(super) fn containing_block(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn rotlerp(from: f32, to: f32, max: f32) -> f32 {
    let mut difference = (to - from) % 360.0;
    if difference >= 180.0 {
        difference -= 360.0;
    }
    if difference < -180.0 {
        difference += 360.0;
    }
    let mut result = from + difference.clamp(-max, max);
    if result < 0.0 {
        result += 360.0;
    } else if result > 360.0 {
        result -= 360.0;
    }
    result
}

fn subtract(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn length_sqr(value: Vec3d) -> f64 {
    value.x * value.x + value.y * value.y + value.z * value.z
}

fn normalize(value: Vec3d) -> Vec3d {
    let length = length_sqr(value).sqrt();
    if length < f64::from(1.0e-5_f32) {
        Vec3d::default()
    } else {
        Vec3d {
            x: value.x / length,
            y: value.y / length,
            z: value.z / length,
        }
    }
}

fn dot(left: Vec3d, right: Vec3d) -> f64 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn distance_sqr(left: Vec3d, right: Vec3d) -> f64 {
    length_sqr(subtract(left, right))
}

#[cfg(test)]
mod tests;
