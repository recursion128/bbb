//! `FlyNodeEvaluator` searched by `PathFinder`: neighbours share one node map,
//! so a node reached again keeps its larger malus and a `WALKABLE` one pays
//! one more on every visit.

use crate::world::simulation_level::LevelCtx;

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, DebugPathNode, DebugPathType};

use super::path_finder::{
    aggregate_path_type_for_mob, build_path, debug_node, debug_path_nodes, distance, key,
    manhattan, PooledSearchArena, NO_INDEX,
};
use super::{
    is_water_block, neighbor_path_type, GroundPath, GroundPathDebugData, GroundPathType,
    MobPathingSnapshot,
};

pub(super) struct FlyPathSearch<'a, 'l, 'w> {
    level: &'a LevelCtx<'l, 'w>,
    mob: MobPathingSnapshot,
    arena: PooledSearchArena,
}

impl<'a, 'l, 'w> FlyPathSearch<'a, 'l, 'w> {
    pub(super) fn new(level: &'a LevelCtx<'l, 'w>, mob: MobPathingSnapshot) -> Self {
        Self {
            level,
            mob,
            arena: PooledSearchArena::take(),
        }
    }

    pub(super) fn find_path(
        mut self,
        random: &GameRandom,
        target: BlockPos,
        max_path_length: f32,
        reach_range: i32,
        max_visited_nodes: usize,
        capture_debug: bool,
    ) -> Option<GroundPath> {
        let start_idx = self.start(random)?;
        let start = self.arena.nodes[start_idx as usize].public.pos;
        let start_h = distance(start, target);
        {
            let node = &mut self.arena.nodes[start_idx as usize];
            node.g = 0.0;
            node.h = start_h;
            node.f = start_h;
        }
        let mut best_idx = start_idx;
        let mut best_h = start_h;
        self.arena.heap_insert(start_idx);
        let mut reached = false;
        let mut count = 0usize;
        while !self.arena.open.is_empty() {
            count = count.saturating_add(1);
            if count >= max_visited_nodes {
                break;
            }
            let current_idx = self.arena.heap_pop()?;
            let (current, current_g, current_walked) = {
                let node = &mut self.arena.nodes[current_idx as usize];
                node.closed = true;
                (node.public.pos, node.g, node.walked_distance)
            };
            if manhattan(current, target) <= reach_range {
                reached = true;
                break;
            }
            if capture_debug {
                self.arena.closed.push(current_idx);
            }
            if distance(current, start) >= max_path_length {
                continue;
            }
            self.fill_neighbors(current);
            let neighbor_count = self.arena.fly_neighbors.len();
            for index in 0..neighbor_count {
                let neighbor_idx = self.arena.fly_neighbors[index];
                let neighbor = self.arena.nodes[neighbor_idx as usize].public.pos;
                let step = distance(current, neighbor);
                let (was_open, old_f) = {
                    let node = &mut self.arena.nodes[neighbor_idx as usize];
                    node.walked_distance = current_walked + step;
                    let tentative_g = current_g + step + node.public.cost_malus;
                    if !(node.walked_distance < max_path_length
                        && (node.heap_index == NO_INDEX || tentative_g < node.g))
                    {
                        continue;
                    }
                    node.came_from = current_idx;
                    node.g = tentative_g;
                    let h = distance(neighbor, target);
                    if h < best_h {
                        best_h = h;
                        best_idx = neighbor_idx;
                    }
                    node.h = h * 1.5;
                    let old_f = node.f;
                    node.f = node.g + node.h;
                    (node.heap_index != NO_INDEX, old_f)
                };
                if was_open {
                    self.arena.heap_change_cost(neighbor_idx, old_f);
                } else {
                    self.arena.heap_insert(neighbor_idx);
                }
            }
        }
        let debug_data = capture_debug.then(|| GroundPathDebugData {
            nodes: debug_path_nodes(&self.arena.nodes, best_idx),
            target_nodes: vec![DebugPathNode {
                x: target.x,
                y: target.y,
                z: target.z,
                walked_distance: 0.0,
                cost_malus: 0.0,
                closed: false,
                path_type: DebugPathType::Blocked,
                f: 0.0,
            }],
            open_set: self
                .arena
                .open
                .iter()
                .map(|&idx| debug_node(&self.arena.nodes[idx as usize]))
                .collect(),
            closed_set: self
                .arena
                .closed
                .iter()
                .map(|&idx| debug_node(&self.arena.nodes[idx as usize]))
                .collect(),
        });
        build_path(&self.arena.nodes, best_idx, target, reached, debug_data)
    }

    pub(super) fn start(&mut self, random: &GameRandom) -> Option<u32> {
        let x = self.mob.position.x.floor() as i32;
        let z = self.mob.position.z.floor() as i32;
        let start_y = if self.mob.can_float && self.mob.in_water {
            let mut y = self.mob.block_position.y;
            while is_water_block(self.level, BlockPos { x, y, z })? {
                y += 1;
            }
            y
        } else {
            (self.mob.position.y + 0.5).floor() as i32
        };
        let start = BlockPos { x, y: start_y, z };
        if !self.can_start_at(start)? {
            let bounds = self.mob.bounding_box;
            let x_size = bounds.max_x - bounds.min_x;
            let y_size = bounds.max_y - bounds.min_y;
            let z_size = bounds.max_z - bounds.min_z;
            if (x_size + y_size + z_size) / 3.0 < 1.0 {
                let padding = |size: f64| (f64::from(1.1_f32) - size).max(0.0);
                let inflated = bounds.inflate(padding(x_size), padding(y_size), padding(z_size));
                let min_x = inflated.min_x.floor() as i32;
                let min_y = inflated.min_y.floor() as i32;
                let min_z = inflated.min_z.floor() as i32;
                let width = inflated.max_x.floor() as i32 - min_x + 1;
                let height = inflated.max_y.floor() as i32 - min_y + 1;
                let depth = inflated.max_z.floor() as i32 - min_z + 1;
                for _ in 0..10 {
                    let candidate = BlockPos {
                        x: min_x + random.next_int(width),
                        y: min_y + random.next_int(height),
                        z: min_z + random.next_int(depth),
                    };
                    if self.can_start_at(candidate)? {
                        return self.start_node(candidate);
                    }
                }
            } else {
                for (corner_x, corner_z) in [
                    (bounds.min_x, bounds.min_z),
                    (bounds.min_x, bounds.max_z),
                    (bounds.max_x, bounds.min_z),
                    (bounds.max_x, bounds.max_z),
                ] {
                    let candidate = BlockPos {
                        x: corner_x.floor() as i32,
                        y: self.mob.block_position.y,
                        z: corner_z.floor() as i32,
                    };
                    if self.can_start_at(candidate)? {
                        return self.start_node(candidate);
                    }
                }
            }
        }
        self.start_node(start)
    }

    fn can_start_at(&mut self, pos: BlockPos) -> Option<bool> {
        let path_type = self.cached_path_type(pos)?;
        Some(self.mob.pathfinding_malus(path_type) >= 0.0)
    }

    pub(super) fn start_node(&mut self, pos: BlockPos) -> Option<u32> {
        let path_type = self.cached_path_type(pos)?;
        let malus = self.mob.pathfinding_malus(path_type);
        let idx = self.arena.get_or_insert_blocked(pos);
        let node = &mut self.arena.nodes[idx as usize];
        node.public.path_type = path_type;
        node.public.cost_malus = malus;
        Some(idx)
    }

    fn fill_neighbors(&mut self, pos: BlockPos) {
        let BlockPos { x, y, z } = pos;
        self.arena.fly_neighbors.clear();
        let south = self.find_accepted_node(x, y, z + 1);
        self.push_if(south, true);
        let west = self.find_accepted_node(x - 1, y, z);
        self.push_if(west, true);
        let east = self.find_accepted_node(x + 1, y, z);
        self.push_if(east, true);
        let north = self.find_accepted_node(x, y, z - 1);
        self.push_if(north, true);
        let up = self.find_accepted_node(x, y + 1, z);
        self.push_if(up, true);
        let down = self.find_accepted_node(x, y - 1, z);
        self.push_if(down, true);

        let south_up = self.find_accepted_node(x, y + 1, z + 1);
        let ok = self.has_malus(south) && self.has_malus(up);
        self.push_if(south_up, ok);
        let west_up = self.find_accepted_node(x - 1, y + 1, z);
        let ok = self.has_malus(west) && self.has_malus(up);
        self.push_if(west_up, ok);
        let east_up = self.find_accepted_node(x + 1, y + 1, z);
        let ok = self.has_malus(east) && self.has_malus(up);
        self.push_if(east_up, ok);
        let north_up = self.find_accepted_node(x, y + 1, z - 1);
        let ok = self.has_malus(north) && self.has_malus(up);
        self.push_if(north_up, ok);
        let south_down = self.find_accepted_node(x, y - 1, z + 1);
        let ok = self.has_malus(south) && self.has_malus(down);
        self.push_if(south_down, ok);
        let west_down = self.find_accepted_node(x - 1, y - 1, z);
        let ok = self.has_malus(west) && self.has_malus(down);
        self.push_if(west_down, ok);
        let east_down = self.find_accepted_node(x + 1, y - 1, z);
        let ok = self.has_malus(east) && self.has_malus(down);
        self.push_if(east_down, ok);
        let north_down = self.find_accepted_node(x, y - 1, z - 1);
        let ok = self.has_malus(north) && self.has_malus(down);
        self.push_if(north_down, ok);

        let north_east = self.find_accepted_node(x + 1, y, z - 1);
        let ok = self.has_malus(north) && self.has_malus(east);
        self.push_if(north_east, ok);
        let south_east = self.find_accepted_node(x + 1, y, z + 1);
        let ok = self.has_malus(south) && self.has_malus(east);
        self.push_if(south_east, ok);
        let north_west = self.find_accepted_node(x - 1, y, z - 1);
        let ok = self.has_malus(north) && self.has_malus(west);
        self.push_if(north_west, ok);
        let south_west = self.find_accepted_node(x - 1, y, z + 1);
        let ok = self.has_malus(south) && self.has_malus(west);
        self.push_if(south_west, ok);

        #[rustfmt::skip]
        let corners = [
            (1, 1, -1, north_east, north, east, up, north_up, east_up),
            (1, 1, 1, south_east, south, east, up, south_up, east_up),
            (-1, 1, -1, north_west, north, west, up, north_up, west_up),
            (-1, 1, 1, south_west, south, west, up, south_up, west_up),
            (1, -1, -1, north_east, north, east, down, north_down, east_down),
            (1, -1, 1, south_east, south, east, down, south_down, east_down),
            (-1, -1, -1, north_west, north, west, down, north_down, west_down),
            (-1, -1, 1, south_west, south, west, down, south_down, west_down),
        ];
        for (dx, dy, dz, diagonal, first, second, vertical, first_vertical, second_vertical) in
            corners
        {
            let corner = self.find_accepted_node(x + dx, y + dy, z + dz);
            let ok = self.has_malus(diagonal)
                && self.has_malus(first)
                && self.has_malus(second)
                && self.has_malus(vertical)
                && self.has_malus(first_vertical)
                && self.has_malus(second_vertical);
            self.push_if(corner, ok);
        }
    }

    fn push_if(&mut self, node: Option<u32>, condition: bool) {
        if let Some(idx) = node {
            if !self.arena.nodes[idx as usize].closed && condition {
                self.arena.fly_neighbors.push(idx);
            }
        }
    }

    fn has_malus(&self, node: Option<u32>) -> bool {
        node.is_some_and(|idx| self.arena.nodes[idx as usize].public.cost_malus >= 0.0)
    }

    pub(super) fn find_accepted_node(&mut self, x: i32, y: i32, z: i32) -> Option<u32> {
        let pos = BlockPos { x, y, z };
        let path_type = self.cached_path_type(pos)?;
        let cost = self.mob.pathfinding_malus(path_type);
        if cost < 0.0 {
            return None;
        }
        let idx = self.arena.get_or_insert_blocked(pos);
        let node = &mut self.arena.nodes[idx as usize];
        node.public.path_type = path_type;
        node.public.cost_malus = node.public.cost_malus.max(cost);
        if path_type == GroundPathType::Walkable {
            node.public.cost_malus += 1.0;
        }
        Some(idx)
    }

    fn cached_path_type(&mut self, pos: BlockPos) -> Option<GroundPathType> {
        let pos_key = key(pos);
        if let Some(&cached) = self.arena.fly_path_types.get(&pos_key) {
            return cached;
        }
        let level = self.level;
        let mob = self.mob;
        let resolved = aggregate_path_type_for_mob(
            mob,
            pos,
            |sample| fly_path_type(level, mob.block_position, sample),
            |sample| fly_path_type(level, mob.block_position, sample),
        );
        self.arena.fly_path_types.insert(pos_key, resolved);
        resolved
    }
}

pub(super) fn fly_path_type(
    level: &LevelCtx<'_, '_>,
    mob_block_position: BlockPos,
    pos: BlockPos,
) -> Option<GroundPathType> {
    let mut path_type = level.ground_path_type_from_state(pos)?;
    if path_type == GroundPathType::Open && pos.y > level.min_y() {
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        path_type = match level.ground_path_type_from_state(below)? {
            GroundPathType::Fire | GroundPathType::Lava => GroundPathType::Fire,
            GroundPathType::Damaging => GroundPathType::Damaging,
            GroundPathType::Cocoa => GroundPathType::Cocoa,
            GroundPathType::Fence if below != mob_block_position => GroundPathType::Fence,
            GroundPathType::Fence => path_type,
            GroundPathType::Walkable | GroundPathType::Open | GroundPathType::Water => {
                GroundPathType::Open
            }
            _ => GroundPathType::Walkable,
        };
    }
    if matches!(path_type, GroundPathType::Walkable | GroundPathType::Open) {
        path_type = neighbor_path_type(level, pos, path_type)?;
    }
    Some(path_type)
}
