//! Shared block-seeking goal (`MoveToBlockGoal`) parameterized by mob behavior kind.

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_path_facts,
};
use bbb_protocol::block_types::VANILLA_BLOCK_LAVA_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use super::body_rotation::base_max_head_x_rot;
use super::cat::{self, CatRestKind};
use super::drowned;
use super::fox_berries;
use super::rabbit::carrots as rabbit_carrots;
use super::target::TargetView;
use super::turtle;
use super::zombie;
use super::{reduced_tick_delay, Goal, MobAiContext};
use crate::entity::fox::FoxHuntAction;
use crate::entity::EntityData;
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveToBlockKind {
    CatLieOnBed,
    CatSitOnBlock,
    StriderGoToLava,
    TurtleGoToWater,
    DrownedGoToBeach,
    ZombieAttackTurtleEgg,
    RabbitRaidGarden,
    FoxEatBerries,
}

impl MoveToBlockKind {
    pub(super) const fn debug_name(self) -> &'static str {
        match self {
            Self::CatLieOnBed | Self::CatSitOnBlock => "CatRestOnBlockGoal",
            Self::StriderGoToLava => "StriderGoToLavaGoal",
            Self::TurtleGoToWater => "TurtleGoToWaterGoal",
            Self::DrownedGoToBeach => "DrownedGoToBeachGoal",
            Self::ZombieAttackTurtleEgg => "ZombieAttackTurtleEggGoal",
            Self::RabbitRaidGarden => "RabbitRaidGardenGoal",
            Self::FoxEatBerries => "FoxEatBerriesGoal",
        }
    }

    const fn search(self) -> &'static MoveToBlockSearch {
        match self {
            Self::CatLieOnBed => &MoveToBlockSearch {
                range: 7,
                heights: &[-3, 2, -4, 3, -5, 4, -6, 5, -7],
                y_outer: true,
                check_home_below: false,
                search_body_entity_pos: true,
                search_y_offset: 1.0,
                dest_y_offset: 1.0,
                path_interval: 40,
            },
            Self::CatSitOnBlock => &MoveToBlockSearch {
                range: 7,
                heights: &[-1, 0, -2],
                y_outer: true,
                check_home_below: false,
                search_body_entity_pos: true,
                search_y_offset: 1.0,
                dest_y_offset: 1.0,
                path_interval: 40,
            },
            Self::StriderGoToLava => &MoveToBlockSearch {
                range: 7,
                heights: &[-3, -2, -1, 0, 1],
                y_outer: false,
                check_home_below: false,
                search_body_entity_pos: true,
                search_y_offset: 0.0,
                dest_y_offset: 0.0,
                path_interval: 20,
            },
            Self::TurtleGoToWater => &MoveToBlockSearch {
                range: 23,
                heights: &[-2],
                y_outer: false,
                check_home_below: false,
                search_body_entity_pos: true,
                search_y_offset: 1.0,
                dest_y_offset: 1.0,
                path_interval: 160,
            },
            Self::DrownedGoToBeach => &MoveToBlockSearch {
                range: 7,
                heights: &[-3, -2, -1, 0, 1],
                y_outer: false,
                check_home_below: false,
                search_body_entity_pos: false,
                search_y_offset: 0.5,
                dest_y_offset: 1.5,
                path_interval: 40,
            },
            Self::ZombieAttackTurtleEgg => &MoveToBlockSearch {
                range: 23,
                heights: &[-4, -3, -2, -1, 0, 1, 2],
                y_outer: false,
                check_home_below: false,
                search_body_entity_pos: false,
                search_y_offset: 1.0,
                dest_y_offset: 1.0,
                path_interval: 40,
            },
            Self::RabbitRaidGarden => &MoveToBlockSearch {
                range: 15,
                heights: &[0, 1, -1],
                y_outer: false,
                check_home_below: true,
                search_body_entity_pos: false,
                search_y_offset: 0.0,
                dest_y_offset: 0.0,
                path_interval: 40,
            },
            Self::FoxEatBerries => &MoveToBlockSearch {
                range: 11,
                heights: &[-1, 0, -2],
                y_outer: true,
                check_home_below: false,
                search_body_entity_pos: false,
                search_y_offset: 1.0,
                dest_y_offset: 1.0,
                path_interval: 100,
            },
        }
    }
}

struct MoveToBlockSearch {
    range: i32,
    heights: &'static [i32],
    y_outer: bool,
    check_home_below: bool,
    search_body_entity_pos: bool,
    search_y_offset: f64,
    dest_y_offset: f64,
    path_interval: i64,
}

#[derive(Clone)]
pub(crate) struct MoveToBlockGoal {
    kind: MoveToBlockKind,
    target: Option<BlockPos>,
    next_search: i64,
    next_path: i64,
    try_ticks: i32,
    max_stay_ticks: i32,
    extra_ticks: u16,
    waited: u16,
}

impl MoveToBlockGoal {
    pub(crate) const fn new(kind: MoveToBlockKind) -> Self {
        Self {
            kind,
            target: None,
            next_search: 0,
            next_path: 0,
            try_ticks: 0,
            max_stay_ticks: 0,
            extra_ticks: 0,
            waited: 0,
        }
    }

    pub(crate) const fn kind(&self) -> MoveToBlockKind {
        self.kind
    }

    pub(super) fn block_point(pos: BlockPos, y_offset: f64) -> Vec3d {
        Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + y_offset,
            z: f64::from(pos.z) + 0.5,
        }
    }

    fn valid_target(&self, targets: &TargetView<'_, '_, '_>, pos: BlockPos) -> bool {
        let level = targets.level();
        match self.kind {
            MoveToBlockKind::CatLieOnBed => cat::valid_target(level, pos, CatRestKind::LieOnBed),
            MoveToBlockKind::CatSitOnBlock => {
                cat::valid_target(level, pos, CatRestKind::SitOnBlock)
            }
            MoveToBlockKind::StriderGoToLava => valid_lava_target(level, pos),
            MoveToBlockKind::TurtleGoToWater => turtle::valid_water_target(level, pos),
            MoveToBlockKind::DrownedGoToBeach => drowned::is_beach(targets, pos),
            MoveToBlockKind::ZombieAttackTurtleEgg => zombie::valid_egg_target(level, pos),
            MoveToBlockKind::RabbitRaidGarden => rabbit_carrots::ripe_carrot(level, pos),
            MoveToBlockKind::FoxEatBerries => fox_berries::ripe(level, pos),
        }
    }

    fn can_enter_before_cooldown(
        &self,
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) -> bool {
        match self.kind {
            MoveToBlockKind::CatLieOnBed => {
                cat::can_move(targets)
                    && !matches!(targets.body_entity().data(), EntityData::GeneralLiving(data) if data.cat_lying())
            }
            MoveToBlockKind::CatSitOnBlock => cat::can_move(targets),
            MoveToBlockKind::StriderGoToLava => !targets.body_entity().is_in_lava(),
            MoveToBlockKind::TurtleGoToWater => turtle::can_seek_water(targets),
            MoveToBlockKind::DrownedGoToBeach => {
                targets.bright_outside() == Some(false)
                    && context.body.in_water
                    && targets
                        .level()
                        .sea_level()
                        .is_some_and(|sea| context.body.position.y >= f64::from(sea - 3))
            }
            MoveToBlockKind::ZombieAttackTurtleEgg | MoveToBlockKind::RabbitRaidGarden => {
                targets.level().mob_griefing()
            }
            MoveToBlockKind::FoxEatBerries => fox_berries::awake(targets.body_entity()),
        }
    }

    fn navigate(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
        let Some(pos) = self.target else {
            return;
        };
        let search = self.kind.search();
        let wanted = Self::block_point(pos, search.dest_y_offset);
        match self.kind {
            MoveToBlockKind::CatLieOnBed => context.navigate_to_position(wanted, 1.1),
            MoveToBlockKind::CatSitOnBlock => context.navigate_to_position(wanted, 0.8),
            MoveToBlockKind::FoxEatBerries => context.navigate_to_position(wanted, 1.2),
            MoveToBlockKind::TurtleGoToWater => {
                let speed = if targets.body().baby() { 2.0 } else { 1.0 };
                context.try_navigate_to_position(wanted, 0, speed);
            }
            MoveToBlockKind::RabbitRaidGarden => context.try_navigate_to_position(wanted, 0, 0.7),
            MoveToBlockKind::StriderGoToLava
            | MoveToBlockKind::DrownedGoToBeach
            | MoveToBlockKind::ZombieAttackTurtleEgg => {
                context.try_navigate_to_position(wanted, 0, 1.0);
            }
        }
        self.next_path = targets.game_time() + search.path_interval;
    }

    fn set_cat_pose(&self, context: &mut MobAiContext, reached: bool) {
        match self.kind {
            MoveToBlockKind::CatLieOnBed => context.species.cat_lying = Some(reached),
            MoveToBlockKind::CatSitOnBlock => context.species.cat_sitting = Some(reached),
            _ => {}
        }
    }
}

impl Goal for MoveToBlockGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let game_time = targets.game_time();
        if !self.can_enter_before_cooldown(context, targets) || game_time < self.next_search {
            return false;
        }
        self.next_search = game_time
            + match self.kind {
                MoveToBlockKind::CatLieOnBed => 40,
                MoveToBlockKind::DrownedGoToBeach => 200,
                MoveToBlockKind::FoxEatBerries => {
                    i64::from(reduced_tick_delay(200 + context.next_int(200)))
                }
                _ => i64::from(200 + context.next_int(200)),
            };
        if self.kind == MoveToBlockKind::RabbitRaidGarden && !rabbit_carrots::hungry(targets) {
            return false;
        }

        let search = self.kind.search();
        let origin = targets.body_entity().block_position();
        let origin_pos = if search.search_body_entity_pos {
            targets.body_entity().position()
        } else {
            context.body.position
        };
        let mut nearest = None;
        let mut distance = f64::INFINITY;
        let mut consider = |pos: BlockPos| {
            let home_pos = if search.check_home_below {
                BlockPos {
                    y: pos.y - 1,
                    ..pos
                }
            } else {
                pos
            };
            if targets.within_home(home_pos) && self.valid_target(targets, pos) {
                let next =
                    origin_pos.distance_to_sqr(Self::block_point(pos, search.search_y_offset));
                if next < distance {
                    distance = next;
                    nearest = Some(pos);
                    return true;
                }
            }
            false
        };

        if self.kind == MoveToBlockKind::ZombieAttackTurtleEgg {
            // Spiral outward by Chebyshev ring (`MoveToBlockGoal.findNearestBlock`)
            // and exit on the first ring containing a valid turtle egg.
            for r in 0..=search.range {
                let mut found_in_ring = false;
                for dx in -r..=r {
                    for dz in -r..=r {
                        if dx.abs() != r && dz.abs() != r {
                            continue;
                        }
                        for &dy in search.heights {
                            if consider(BlockPos {
                                x: origin.x + dx,
                                y: origin.y + dy,
                                z: origin.z + dz,
                            }) {
                                found_in_ring = true;
                            }
                        }
                    }
                }
                if found_in_ring {
                    break;
                }
            }
        } else if search.y_outer {
            for &dy in search.heights {
                for dx in -search.range..=search.range {
                    for dz in -search.range..=search.range {
                        consider(BlockPos {
                            x: origin.x + dx,
                            y: origin.y + dy,
                            z: origin.z + dz,
                        });
                    }
                }
            }
        } else {
            for dx in -search.range..=search.range {
                for dz in -search.range..=search.range {
                    for &dy in search.heights {
                        consider(BlockPos {
                            x: origin.x + dx,
                            y: origin.y + dy,
                            z: origin.z + dz,
                        });
                    }
                }
            }
        }

        self.target = nearest;
        if self.kind == MoveToBlockKind::ZombieAttackTurtleEgg && nearest.is_some() {
            self.next_search = game_time + 20;
        }
        nearest.is_some()
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let valid_state = match self.kind {
            MoveToBlockKind::CatLieOnBed | MoveToBlockKind::CatSitOnBlock => {
                cat::can_move(targets)
                    && self.try_ticks <= 1200
                    && self.try_ticks >= -self.max_stay_ticks
            }
            MoveToBlockKind::StriderGoToLava => !targets.body_entity().is_in_lava(),
            MoveToBlockKind::TurtleGoToWater => {
                turtle::can_seek_water(targets) && self.try_ticks <= 1200
            }
            MoveToBlockKind::DrownedGoToBeach => self.try_ticks <= 1200,
            MoveToBlockKind::ZombieAttackTurtleEgg => {
                targets.level().mob_griefing() && self.try_ticks <= 1200
            }
            MoveToBlockKind::RabbitRaidGarden => {
                targets.level().mob_griefing()
                    && rabbit_carrots::hungry(targets)
                    && self.try_ticks <= 1200
            }
            MoveToBlockKind::FoxEatBerries => {
                fox_berries::awake(targets.body_entity())
                    && self.try_ticks <= 1200
                    && self.extra_ticks <= 2400
            }
        };
        valid_state
            && self
                .target
                .is_some_and(|pos| self.valid_target(targets, pos))
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.try_ticks = 0;
        self.extra_ticks = 0;
        self.waited = 0;
        match self.kind {
            MoveToBlockKind::CatLieOnBed | MoveToBlockKind::CatSitOnBlock => {
                let stay_range = context.next_int(1200) + 1200;
                self.max_stay_ticks = context.next_int(stay_range) + 1200;
                self.set_cat_pose(context, false);
                self.navigate(context, targets);
                self.next_path = targets.game_time();
            }
            MoveToBlockKind::FoxEatBerries => {
                if let Some((entity, _, _)) = targets.world_mut() {
                    crate::entity::fox::apply_hunt_action(entity, FoxHuntAction::Stand);
                }
                self.navigate(context, targets);
            }
            _ => self.navigate(context, targets),
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(pos) = self.target else {
            return;
        };
        match self.kind {
            MoveToBlockKind::StriderGoToLava
            | MoveToBlockKind::TurtleGoToWater
            | MoveToBlockKind::DrownedGoToBeach => {
                if self.kind != MoveToBlockKind::StriderGoToLava {
                    self.try_ticks += 1;
                }
                if targets.game_time() >= self.next_path {
                    self.navigate(context, targets);
                }
            }
            MoveToBlockKind::CatLieOnBed | MoveToBlockKind::CatSitOnBlock => {
                let reached = context
                    .body
                    .position
                    .distance_to_sqr(Self::block_point(pos, 1.5))
                    < 1.0;
                self.set_cat_pose(context, reached);
                if reached {
                    self.try_ticks -= 1;
                } else {
                    self.try_ticks += 1;
                    if targets.game_time() >= self.next_path {
                        self.navigate(context, targets);
                    }
                }
            }
            MoveToBlockKind::ZombieAttackTurtleEgg => {
                let reached = context
                    .body
                    .position
                    .distance_to_sqr(Self::block_point(pos, 1.5))
                    < 1.14 * 1.14;
                if !reached {
                    self.try_ticks += 1;
                    if targets.game_time() >= self.next_path {
                        self.navigate(context, targets);
                    }
                    return;
                }
                zombie::tick_stomp_egg(context, targets, self.extra_ticks);
                self.extra_ticks = self.extra_ticks.saturating_add(1);
            }
            MoveToBlockKind::RabbitRaidGarden => {
                let wanted = Self::block_point(pos, 0.0);
                context.set_look_at_with_limits(
                    wanted.x,
                    wanted.y,
                    wanted.z,
                    10.0,
                    base_max_head_x_rot(targets.body_entity().entity_type_id()).unwrap_or(40.0),
                );
                if context
                    .body
                    .position
                    .distance_to_sqr(Self::block_point(pos, 0.5))
                    >= 1.0
                {
                    self.try_ticks += 1;
                    if targets.game_time() >= self.next_path {
                        self.navigate(context, targets);
                    }
                    return;
                }
                rabbit_carrots::raid_carrot(targets, pos);
                self.target = None;
                self.next_search = targets.game_time() + 10;
            }
            MoveToBlockKind::FoxEatBerries => {
                if context
                    .body
                    .position
                    .distance_to_sqr(Self::block_point(pos, 1.5))
                    < 4.0
                {
                    self.extra_ticks = self.extra_ticks.saturating_add(1);
                    self.waited = self.waited.saturating_add(1);
                    if self.waited >= 40 && targets.level().mob_griefing() {
                        if let Some((entity, store, level)) = targets.world_mut() {
                            fox_berries::harvest(entity, store, level, pos);
                        }
                    }
                } else {
                    self.try_ticks += 1;
                    if targets.game_time() >= self.next_path {
                        self.navigate(context, targets);
                    }
                    if context.next_float() < 0.05 {
                        let source =
                            crate::entity::living::living_sound_source(targets.body_entity());
                        targets.play_body_sound("minecraft:entity.fox.sniff", source, 1.0, 1.0);
                    }
                }
            }
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        match self.kind {
            MoveToBlockKind::CatLieOnBed | MoveToBlockKind::CatSitOnBlock => {
                self.set_cat_pose(context, false);
            }
            MoveToBlockKind::StriderGoToLava | MoveToBlockKind::TurtleGoToWater => {}
            MoveToBlockKind::DrownedGoToBeach
            | MoveToBlockKind::ZombieAttackTurtleEgg
            | MoveToBlockKind::RabbitRaidGarden
            | MoveToBlockKind::FoxEatBerries => context.stop_navigation(),
        }
        self.target = None;
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn valid_lava_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.has_ticking_column_at(pos)
        && vanilla_block_state_block_type_id(level.block_state(pos)) == Some(VANILLA_BLOCK_LAVA_ID)
        && vanilla_block_state_path_facts(level.block_state(BlockPos {
            y: pos.y + 1,
            ..pos
        }))
        .is_some_and(|facts| facts.land_pathfindable())
}
