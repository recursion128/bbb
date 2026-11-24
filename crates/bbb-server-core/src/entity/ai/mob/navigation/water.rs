//! Water navigation and body-sized swimming nodes.

use rustc_hash::FxHashMap;

use super::path_finder::{key, search_from_start, PositionKey, SearchNode};
use super::{
    can_move_directly, containing_block, fluid_is_water, node_entity_position, GroundPath,
    GroundPathType, MobPathingSnapshot, NavigationMove, PathNavigationBase,
};
use crate::entity::level::block_state_water_pathfindable;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_path_facts, vanilla_block_state_support_facts,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_fluid_state;

#[derive(Debug, Clone)]
pub(in crate::entity::ai::mob) struct WaterNavigation {
    base: PathNavigationBase,
    allow_breaching: bool,
}

impl std::ops::Deref for WaterNavigation {
    type Target = PathNavigationBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for WaterNavigation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl WaterNavigation {
    pub(super) fn new(allow_breaching: bool) -> Self {
        let base = PathNavigationBase {
            can_pass_doors: false,
            ..PathNavigationBase::default()
        };
        Self {
            base,
            allow_breaching,
        }
    }

    fn can_update_path(&self, mob: MobPathingSnapshot) -> bool {
        self.allow_breaching || mob.in_liquid
    }

    pub(super) fn create_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
    ) -> Option<GroundPath> {
        let reused = self
            .base
            .path
            .as_ref()
            .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target));
        let path = self.create_path_to(
            level,
            mob,
            target,
            reach_range,
            None,
            self.base.capture_debug,
        )?;
        if !reused {
            self.base.accept_created_path(&path, reach_range);
        }
        Some(path)
    }

    pub(super) fn create_path_to(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        max_path_length: Option<f32>,
        capture_debug: bool,
    ) -> Option<GroundPath> {
        let mob = self.base.configure_mob(mob);
        if mob.position.y < f64::from(level.min_y()) || !self.can_update_path(mob) {
            return None;
        }
        if self
            .base
            .path
            .as_ref()
            .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target))
        {
            return self.base.path.clone();
        }
        let max_path_length = max_path_length.unwrap_or(mob.follow_range as f32);
        let mut evaluator = SwimNodes {
            level,
            mob,
            allow_breaching: self.allow_breaching,
            path_types: FxHashMap::default(),
        };
        let start = containing_block(Vec3d {
            x: mob.bounding_box.min_x,
            y: mob.bounding_box.min_y + 0.5,
            z: mob.bounding_box.min_z,
        });
        let start = SearchNode::new(start, GroundPathType::Blocked, 0.0);
        let path = search_from_start(
            level,
            mob,
            std::slice::from_ref(&target),
            super::path_finder::SearchBudget {
                max_path_length,
                reach_range,
                max_visited_nodes: (max_path_length * 16.0).floor().max(0.0) as usize,
                capture_debug,
            },
            start,
            |pos, _, out| evaluator.neighbors_into(pos, out),
        )?;
        Some(path)
    }

    pub(super) fn start_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        path: GroundPath,
        mob: MobPathingSnapshot,
        speed_modifier: f64,
    ) -> bool {
        self.base
            .move_to_path(level, Some(path), temp_position(mob), speed_modifier)
    }

    pub(super) fn move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
        retain_on_failure: bool,
    ) -> bool {
        let path = self.create_path(level, mob, target, reach_range);
        if retain_on_failure && path.is_none() {
            return false;
        }
        self.base
            .move_to_path(level, path, temp_position(mob), speed_modifier)
    }

    pub(super) fn recompute(&mut self, level: &LevelCtx<'_, '_>, mob: MobPathingSnapshot) {
        let Some((target, reach_range)) = self
            .base
            .recompute_target(level.game_time(), self.can_update_path(mob))
        else {
            return;
        };
        self.base.path = None;
        let path = self.create_path(level, mob, target, reach_range);
        self.base.finish_recompute(level.game_time(), path);
    }

    pub(super) fn tick(
        &mut self,
        level: &LevelCtx<'_, '_>,
        context: &mut super::super::MobAiContext,
    ) -> Option<NavigationMove> {
        self.base.tick = self.base.tick.wrapping_add(1);
        let mob = self
            .base
            .configure_mob(MobPathingSnapshot::from_context(context));
        if self.base.delayed_recomputation {
            self.recompute(level, mob);
        }
        if self.base.is_done() {
            return None;
        }
        if self.can_update_path(mob) {
            self.base.follow_the_path(
                mob,
                temp_position(mob),
                level.game_time(),
                GroundPathType::can_cut_corner,
                |start, stop| can_move_directly(level, mob, false, start, stop),
            );
        }
        let next = self.base.path.as_ref().and_then(GroundPath::next_node)?;
        Some(NavigationMove {
            wanted: node_entity_position(mob, next.pos),
            speed_modifier: self.base.speed_modifier,
        })
    }
}

fn temp_position(mob: MobPathingSnapshot) -> Vec3d {
    Vec3d {
        y: mob.position.y + f64::from(mob.height) * 0.5,
        ..mob.position
    }
}

pub(super) fn stable_destination(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    level.has_ticking_column_at(pos).then_some(())?;
    let state = level.block_state(pos);
    if let Some(facts) = vanilla_block_state_support_facts(state) {
        Some(!facts.solid_render())
    } else {
        Some(
            !level
                .content_registry()
                .block_by_state_id(state)?
                .0
                .full_cube_collision,
        )
    }
}

struct SwimNodes<'a, 'l, 'w> {
    level: &'a LevelCtx<'l, 'w>,
    mob: MobPathingSnapshot,
    allow_breaching: bool,
    path_types: FxHashMap<PositionKey, Option<GroundPathType>>,
}

impl SwimNodes<'_, '_, '_> {
    fn neighbors_into(&mut self, pos: BlockPos, neighbors: &mut Vec<SearchNode>) {
        let mut horizontal = [false; 4];
        for (index, (x, y, z)) in [
            (0, 0, -1),
            (1, 0, 0),
            (0, 0, 1),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(node) = self.accepted_node(BlockPos {
                x: pos.x + x,
                y: pos.y + y,
                z: pos.z + z,
            }) {
                if index < horizontal.len() {
                    horizontal[index] = true;
                }
                neighbors.push(node);
            }
        }
        for (index, (x, z)) in [(1, -1), (1, 1), (-1, 1), (-1, -1)].into_iter().enumerate() {
            if horizontal[index] && horizontal[(index + 1) % 4] {
                if let Some(node) = self.accepted_node(BlockPos {
                    x: pos.x + x,
                    y: pos.y,
                    z: pos.z + z,
                }) {
                    neighbors.push(node);
                }
            }
        }
    }

    fn accepted_node(&mut self, pos: BlockPos) -> Option<SearchNode> {
        let path_type = match self.path_types.entry(key(pos)) {
            std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                *entry.insert(path_type(self.level, self.mob, pos))
            }
        }?;
        if path_type != GroundPathType::Water && !self.allow_breaching {
            return None;
        }
        let mut malus = self.mob.pathfinding_malus(path_type);
        if malus < 0.0 {
            return None;
        }
        if block_state_fluid_state(self.level.block_state(pos)).is_empty() {
            malus += 8.0;
        }
        Some(SearchNode::new(pos, path_type, malus))
    }
}

fn path_type(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
) -> Option<GroundPathType> {
    let width = (mob.width + 1.0).floor() as i32;
    let height = (mob.height + 1.0).floor() as i32;
    let mut breach = false;
    for x in pos.x..pos.x + width {
        for y in pos.y..pos.y + height {
            for z in pos.z..pos.z + width {
                let at = BlockPos { x, y, z };
                level.has_ticking_column_at(at).then_some(())?;
                let state = level.block_state(at);
                let is_air = if let Some(facts) = vanilla_block_state_path_facts(state) {
                    facts.is_air()
                } else {
                    level.content_registry().block_by_state_id(state)?;
                    false
                };
                let fluid = block_state_fluid_state(state);
                if fluid.is_empty() && is_air {
                    let below = BlockPos { y: y - 1, ..at };
                    level.has_ticking_column_at(below).then_some(())?;
                    if block_state_water_pathfindable(level, level.block_state(below))? {
                        breach = true;
                        continue;
                    }
                }
                if !fluid_is_water(level, at)? || !block_state_water_pathfindable(level, state)? {
                    return None;
                }
            }
        }
    }
    Some(if breach {
        GroundPathType::Breach
    } else {
        GroundPathType::Water
    })
}
