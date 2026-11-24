//! `PathNavigation`'s own state and the path-following half every navigation
//! variant inherits; a variant supplies its path finder, temp position,
//! `canUpdatePath`, `canCutCorner` and `canMoveDirectly`.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::{BlockPos, DebugPath, DebugPathInfo, Vec3d};

use crate::entity::level::block_state_tag_membership;

use super::{
    distance_sqr, dot, length_sqr, node_entity_position, normalize, subtract, GroundPath,
    GroundPathType, MobPathingSnapshot,
};

// `PathNavigation.java`.
#[derive(Debug, Clone)]
pub(in crate::entity::ai::mob) struct PathNavigationBase {
    pub(in crate::entity::ai::mob) path: Option<GroundPath>,
    pub(super) speed_modifier: f64,
    pub(super) tick: i32,
    pub(super) last_stuck_check: i32,
    pub(super) last_stuck_check_pos: Vec3d,
    pub(super) timeout_cached_node: BlockPos,
    pub(super) timeout_timer: i64,
    pub(super) last_timeout_check: i64,
    pub(super) timeout_limit: f64,
    pub(super) max_distance_to_waypoint: f32,
    pub(super) delayed_recomputation: bool,
    pub(super) time_last_recompute: i64,
    pub(super) target_pos: Option<BlockPos>,
    pub(super) reach_range: i32,
    pub(super) is_stuck: bool,
    pub(in crate::entity::ai::mob) can_float: bool,
    pub(in crate::entity::ai::mob) can_open_doors: bool,
    pub(in crate::entity::ai::mob) can_pass_doors: bool,
    pub(super) capture_debug: bool,
}

impl Default for PathNavigationBase {
    fn default() -> Self {
        Self {
            path: None,
            speed_modifier: 0.0,
            tick: 0,
            last_stuck_check: 0,
            last_stuck_check_pos: Vec3d::default(),
            timeout_cached_node: BlockPos { x: 0, y: 0, z: 0 },
            timeout_timer: 0,
            last_timeout_check: 0,
            timeout_limit: 0.0,
            max_distance_to_waypoint: 0.5,
            delayed_recomputation: false,
            time_last_recompute: 0,
            target_pos: None,
            reach_range: 0,
            is_stuck: false,
            can_float: false,
            // `NodeEvaluator.java`.
            can_open_doors: false,
            can_pass_doors: true,
            capture_debug: false,
        }
    }
}

impl PathNavigationBase {
    pub(super) fn configure_mob(&self, mut mob: MobPathingSnapshot) -> MobPathingSnapshot {
        mob.can_float = self.can_float;
        mob.can_open_doors = self.can_open_doors;
        mob.can_pass_doors = self.can_pass_doors;
        mob
    }

    pub(in crate::entity::ai::mob) fn set_debug_capture(&mut self, enabled: bool) {
        self.capture_debug = enabled;
    }

    pub(in crate::entity::ai::mob) fn path(&self) -> Option<&GroundPath> {
        self.path.as_ref()
    }

    // `PathNavigation.isDone` (`PathNavigation.java`).
    pub(in crate::entity::ai::mob) fn is_done(&self) -> bool {
        self.path.as_ref().is_none_or(GroundPath::is_done)
    }

    pub(in crate::entity::ai::mob) fn stop(&mut self) {
        self.path = None;
    }

    // `PathNavigation.isStuck()` (`PathNavigation.java`).
    pub(in crate::entity::ai::mob) fn is_stuck(&self) -> bool {
        self.is_stuck
    }

    // `PathNavigation.shouldRecomputePath` (`PathNavigation.java`).
    pub(in crate::entity::ai::mob) fn should_recompute(
        &self,
        positions: &[BlockPos],
        mob_position: Vec3d,
    ) -> bool {
        if self.delayed_recomputation {
            return false;
        }
        let Some(path) = self
            .path
            .as_ref()
            .filter(|path| !path.is_done() && !path.nodes.is_empty())
        else {
            return false;
        };
        let end = path.nodes.last().expect("nonempty path").pos;
        let middle = Vec3d {
            x: (f64::from(end.x) + mob_position.x) / 2.0,
            y: (f64::from(end.y) + mob_position.y) / 2.0,
            z: (f64::from(end.z) + mob_position.z) / 2.0,
        };
        let remaining = path.nodes.len().saturating_sub(path.next_node_index) as f64;
        positions.iter().any(|pos| {
            let dx = f64::from(pos.x) + 0.5 - middle.x;
            let dy = f64::from(pos.y) + 0.5 - middle.y;
            let dz = f64::from(pos.z) + 0.5 - middle.z;
            dx * dx + dy * dy + dz * dz < remaining * remaining
        })
    }

    /// The gate of `PathNavigation.recomputePath` (`PathNavigation.java`):
    /// the target to path to again, or `None` after deferring or with no target.
    pub(super) fn recompute_target(
        &mut self,
        game_time: i64,
        can_update_path: bool,
    ) -> Option<(BlockPos, i32)> {
        if game_time - self.time_last_recompute <= 20 || !can_update_path {
            self.delayed_recomputation = true;
            return None;
        }
        self.target_pos.map(|target| (target, self.reach_range))
    }

    pub(super) fn finish_recompute(&mut self, game_time: i64, path: Option<GroundPath>) {
        self.path = path;
        self.time_last_recompute = game_time;
        self.delayed_recomputation = false;
    }

    pub(super) fn accept_created_path(&mut self, path: &GroundPath, reach_range: i32) {
        self.target_pos = Some(path.target());
        self.reach_range = reach_range;
        self.reset_stuck_timeout();
    }

    /// `PathNavigation.moveTo(Path, double)` (`PathNavigation.java`)
    /// with the cauldron trim of `trimPath` (`:354-368`).
    pub(super) fn move_to_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        new_path: Option<GroundPath>,
        temp_position: Vec3d,
        speed_modifier: f64,
    ) -> bool {
        let Some(new_path) = new_path else {
            self.path = None;
            return false;
        };
        if !self
            .path
            .as_ref()
            .is_some_and(|path| path.nodes_match(&new_path))
        {
            self.path = Some(new_path);
        }
        if self.is_done() {
            return false;
        }
        self.trim_path(level);
        if self.path.as_ref().is_none_or(|path| path.nodes.is_empty()) {
            return false;
        }
        self.speed_modifier = speed_modifier;
        self.last_stuck_check = self.tick;
        self.last_stuck_check_pos = temp_position;
        true
    }

    fn trim_path(&mut self, level: &LevelCtx<'_, '_>) {
        let Some(path) = self.path.as_mut() else {
            return;
        };
        for index in 0..path.nodes.len() {
            let node = path.nodes[index];
            let is_cauldron = block_state_tag_membership(
                level,
                level.block_state(node.pos),
                "minecraft:cauldrons",
            );
            if !is_cauldron {
                continue;
            }
            path.nodes[index].pos.y = node.pos.y + 1;
            if let Some(next) = path.nodes.get(index + 1).copied() {
                if node.pos.y >= next.pos.y {
                    path.nodes[index + 1] = super::GroundPathNode {
                        pos: BlockPos {
                            x: next.pos.x,
                            y: node.pos.y + 1,
                            z: next.pos.z,
                        },
                        ..node
                    };
                }
            }
        }
    }

    /// `PathNavigation.followThePath` (`PathNavigation.java`) over
    /// `shouldTargetNextNodeInDirection` (`:262-290`).
    pub(super) fn follow_the_path(
        &mut self,
        mob: MobPathingSnapshot,
        temp_position: Vec3d,
        game_time: i64,
        can_cut_corner: impl Fn(GroundPathType) -> bool,
        mut can_move_directly: impl FnMut(Vec3d, Vec3d) -> bool,
    ) {
        self.max_distance_to_waypoint = if mob.width > 0.75 {
            mob.width / 2.0
        } else {
            0.75 - mob.width / 2.0
        };
        let max = f64::from(self.max_distance_to_waypoint);
        let Some(path) = self.path.as_mut() else {
            return;
        };
        let Some(next) = path.next_node() else {
            return;
        };
        let dx = (mob.position.x - (f64::from(next.pos.x) + 0.5)).abs();
        let dy = (mob.position.y - f64::from(next.pos.y)).abs();
        let dz = (mob.position.z - (f64::from(next.pos.z) + 0.5)).abs();
        if dx < max && dz < max && dy < 1.0
            || can_cut_corner(next.path_type)
                && should_target_next_node_in_direction(
                    path,
                    mob,
                    temp_position,
                    &mut can_move_directly,
                )
        {
            path.advance();
        }
        self.detect_stuck(game_time, temp_position, f64::from(mob.speed));
    }

    pub(in crate::entity::ai::mob) fn debug_info(&self) -> Option<DebugPathInfo> {
        let path = self.path.as_ref()?;
        let debug = path.debug_data.as_ref()?;
        Some(DebugPathInfo {
            path: DebugPath {
                reached: path.reached,
                next_node_index: path.next_node_index as i32,
                target: path.target,
                nodes: debug.nodes.clone(),
                target_nodes: debug.target_nodes.clone(),
                open_set: debug.open_set.clone(),
                closed_set: debug.closed_set.clone(),
            },
            max_node_distance: self.max_distance_to_waypoint,
        })
    }

    // `PathNavigation.doStuckDetection` (`PathNavigation.java`).
    pub(super) fn detect_stuck(&mut self, game_time: i64, mob_pos: Vec3d, speed: f64) {
        if self.tick.wrapping_sub(self.last_stuck_check) > 100 {
            let effective = if speed >= 1.0 { speed } else { speed * speed };
            let threshold = effective * 25.0;
            if distance_sqr(mob_pos, self.last_stuck_check_pos) < threshold * threshold {
                self.is_stuck = true;
                self.stop();
            } else {
                self.is_stuck = false;
            }
            self.last_stuck_check = self.tick;
            self.last_stuck_check_pos = mob_pos;
        }
        let Some(node) = self.path.as_ref().and_then(GroundPath::next_node) else {
            return;
        };
        if node.pos == self.timeout_cached_node {
            self.timeout_timer = self
                .timeout_timer
                .wrapping_add(game_time.wrapping_sub(self.last_timeout_check));
        } else {
            self.timeout_cached_node = node.pos;
            let node_center = Vec3d {
                x: f64::from(node.pos.x) + 0.5,
                y: f64::from(node.pos.y),
                z: f64::from(node.pos.z) + 0.5,
            };
            self.timeout_limit = if speed > 0.0 {
                distance_sqr(mob_pos, node_center).sqrt() / speed * 20.0
            } else {
                0.0
            };
        }
        if self.timeout_limit > 0.0 && self.timeout_timer as f64 > self.timeout_limit * 3.0 {
            self.reset_stuck_timeout();
            self.stop();
        }
        self.last_timeout_check = game_time;
    }

    pub(super) fn reset_stuck_timeout(&mut self) {
        self.timeout_cached_node = BlockPos { x: 0, y: 0, z: 0 };
        self.timeout_timer = 0;
        self.timeout_limit = 0.0;
        self.is_stuck = false;
    }
}

fn should_target_next_node_in_direction(
    path: &GroundPath,
    mob: MobPathingSnapshot,
    position: Vec3d,
    can_move_directly: &mut impl FnMut(Vec3d, Vec3d) -> bool,
) -> bool {
    if path.next_node_index + 1 >= path.nodes.len() {
        return false;
    }
    let current = path.nodes[path.next_node_index].pos;
    let next = path.nodes[path.next_node_index + 1].pos;
    let current_center = Vec3d {
        x: f64::from(current.x) + 0.5,
        y: f64::from(current.y),
        z: f64::from(current.z) + 0.5,
    };
    if distance_sqr(position, current_center) >= 4.0 {
        return false;
    }
    if can_move_directly(position, node_entity_position(mob, current)) {
        return true;
    }
    let next_center = Vec3d {
        x: f64::from(next.x) + 0.5,
        y: f64::from(next.y),
        z: f64::from(next.z) + 0.5,
    };
    let current_delta = subtract(current_center, position);
    let next_delta = subtract(next_center, position);
    let current_sqr = length_sqr(current_delta);
    let next_sqr = length_sqr(next_delta);
    if next_sqr >= current_sqr && current_sqr >= 0.5 {
        return false;
    }
    dot(normalize(current_delta), normalize(next_delta)) < 0.0
}
