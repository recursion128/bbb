use crate::world::simulation_level::LevelCtx;
use std::cell::Cell;

use bbb_protocol::packets::BlockPos;
use rustc_hash::FxHashMap;

use super::{
    debug_path_type, evaluator_floor_level, is_water_start_state, walk_path_type, GroundPath,
    GroundPathDebugData, GroundPathNode, GroundPathType, MobPathingSnapshot,
};
use bbb_protocol::packets::{DebugPathNode, DebugPathType};

pub(super) const NO_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub(super) struct SearchNode {
    pub(super) public: GroundPathNode,
    pub(super) came_from: u32,
    pub(super) g: f32,
    pub(super) h: f32,
    pub(super) f: f32,
    pub(super) walked_distance: f32,
    pub(super) closed: bool,
    pub(super) heap_index: u32,
}

// Packed `(x, z, y)` matching `BlockPos.asLong`.
pub(super) type PositionKey = i64;

#[inline]
pub(super) const fn key(pos: BlockPos) -> PositionKey {
    (((pos.x as i64) & 0x3ff_ffff) << 38)
        | (((pos.z as i64) & 0x3ff_ffff) << 12)
        | ((pos.y as i64) & 0xfff)
}

impl SearchNode {
    pub(super) fn new(pos: BlockPos, path_type: GroundPathType, cost_malus: f32) -> Self {
        Self {
            public: GroundPathNode {
                pos,
                path_type,
                cost_malus,
            },
            came_from: NO_INDEX,
            g: 0.0,
            h: 0.0,
            f: 0.0,
            walked_distance: 0.0,
            closed: false,
            heap_index: NO_INDEX,
        }
    }
}

#[derive(Default)]
pub(super) struct SearchArena {
    pub(super) nodes: Vec<SearchNode>,
    pub(super) by_pos: FxHashMap<PositionKey, u32>,
    pub(super) open: Vec<u32>,
    pub(super) closed: Vec<u32>,
    pub(super) neighbors: Vec<SearchNode>,
    pub(super) fly_neighbors: Vec<u32>,
    pub(super) fly_path_types: FxHashMap<PositionKey, Option<GroundPathType>>,
}

impl SearchArena {
    fn clear(&mut self) {
        self.nodes.clear();
        self.by_pos.clear();
        self.open.clear();
        self.closed.clear();
        self.neighbors.clear();
        self.fly_neighbors.clear();
        self.fly_path_types.clear();
    }

    #[inline]
    pub(super) fn insert_node(&mut self, pos_key: PositionKey, node: SearchNode) -> u32 {
        let idx = self.nodes.len() as u32;
        self.nodes.push(node);
        self.by_pos.insert(pos_key, idx);
        idx
    }

    #[inline]
    pub(super) fn get_or_insert_blocked(&mut self, pos: BlockPos) -> u32 {
        let pos_key = key(pos);
        if let Some(&idx) = self.by_pos.get(&pos_key) {
            idx
        } else {
            self.insert_node(pos_key, SearchNode::new(pos, GroundPathType::Blocked, 0.0))
        }
    }

    pub(super) fn heap_insert(&mut self, node_idx: u32) {
        let index = self.open.len();
        self.open.push(node_idx);
        self.nodes[node_idx as usize].heap_index = index as u32;
        self.heap_up(index);
    }

    pub(super) fn heap_pop(&mut self) -> Option<u32> {
        let popped = *self.open.first()?;
        let last = self.open.pop()?;
        if !self.open.is_empty() {
            self.open[0] = last;
            self.nodes[last as usize].heap_index = 0;
            self.heap_down(0);
        }
        self.nodes[popped as usize].heap_index = NO_INDEX;
        Some(popped)
    }

    pub(super) fn heap_change_cost(&mut self, node_idx: u32, old_f: f32) {
        let index = self.nodes[node_idx as usize].heap_index;
        if index == NO_INDEX {
            return;
        }
        if self.nodes[node_idx as usize].f < old_f {
            self.heap_up(index as usize);
        } else {
            self.heap_down(index as usize);
        }
    }

    fn heap_up(&mut self, mut index: usize) {
        let node_idx = self.open[index];
        let cost = self.nodes[node_idx as usize].f;
        while index > 0 {
            let parent_index = (index - 1) >> 1;
            let parent = self.open[parent_index];
            if cost >= self.nodes[parent as usize].f {
                break;
            }
            self.open[index] = parent;
            self.nodes[parent as usize].heap_index = index as u32;
            index = parent_index;
        }
        self.open[index] = node_idx;
        self.nodes[node_idx as usize].heap_index = index as u32;
    }

    fn heap_down(&mut self, mut index: usize) {
        let node_idx = self.open[index];
        let cost = self.nodes[node_idx as usize].f;
        let len = self.open.len();
        loop {
            let left_index = 1 + (index << 1);
            if left_index >= len {
                break;
            }
            let right_index = left_index + 1;
            let left = self.open[left_index];
            let (child, child_index) = if right_index < len {
                let right = self.open[right_index];
                if self.nodes[left as usize].f >= self.nodes[right as usize].f {
                    (right, right_index)
                } else {
                    (left, left_index)
                }
            } else {
                (left, left_index)
            };
            if self.nodes[child as usize].f >= cost {
                break;
            }
            self.open[index] = child;
            self.nodes[child as usize].heap_index = index as u32;
            index = child_index;
        }
        self.open[index] = node_idx;
        self.nodes[node_idx as usize].heap_index = index as u32;
    }
}

#[derive(Default)]
struct WalkPathCache {
    mob_types: FxHashMap<PositionKey, Option<GroundPathType>>,
    sample_types: FxHashMap<PositionKey, Option<GroundPathType>>,
    ground_types: FxHashMap<PositionKey, Option<GroundPathType>>,
}

impl WalkPathCache {
    fn clear(&mut self) {
        self.mob_types.clear();
        self.sample_types.clear();
        self.ground_types.clear();
    }
}

thread_local! {
    static SEARCH_ARENA_POOL: Cell<Option<Box<SearchArena>>> = const { Cell::new(None) };
    static WALK_PATH_CACHE_POOL: Cell<Option<Box<WalkPathCache>>> = const { Cell::new(None) };
}

struct PooledWalkPathCache(Option<Box<WalkPathCache>>);

impl PooledWalkPathCache {
    fn take() -> Self {
        let mut cache = WALK_PATH_CACHE_POOL
            .with(|slot| slot.take())
            .unwrap_or_default();
        cache.clear();
        Self(Some(cache))
    }
}

impl std::ops::Deref for PooledWalkPathCache {
    type Target = WalkPathCache;

    fn deref(&self) -> &Self::Target {
        self.0.as_deref().expect("pooled walk path cache active")
    }
}

impl std::ops::DerefMut for PooledWalkPathCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_deref_mut()
            .expect("pooled walk path cache active")
    }
}

impl Drop for PooledWalkPathCache {
    fn drop(&mut self) {
        if let Some(mut cache) = self.0.take() {
            if cache.mob_types.capacity() <= 8192
                && cache.sample_types.capacity() <= 8192
                && cache.ground_types.capacity() <= 8192
            {
                cache.clear();
                WALK_PATH_CACHE_POOL.with(|slot| slot.set(Some(cache)));
            }
        }
    }
}

pub(super) struct PooledSearchArena(Option<Box<SearchArena>>);

impl PooledSearchArena {
    pub(super) fn take() -> Self {
        let mut arena = SEARCH_ARENA_POOL
            .with(|slot| slot.take())
            .unwrap_or_default();
        arena.clear();
        Self(Some(arena))
    }
}

impl std::ops::Deref for PooledSearchArena {
    type Target = SearchArena;

    fn deref(&self) -> &Self::Target {
        self.0.as_deref().expect("pooled arena active")
    }
}

impl std::ops::DerefMut for PooledSearchArena {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_deref_mut().expect("pooled arena active")
    }
}

impl Drop for PooledSearchArena {
    fn drop(&mut self) {
        if let Some(mut arena) = self.0.take() {
            if arena.nodes.capacity() <= 8192 && arena.by_pos.capacity() <= 8192 {
                arena.clear();
                SEARCH_ARENA_POOL.with(|slot| slot.set(Some(arena)));
            }
        }
    }
}

#[cfg(test)]
pub(super) fn find_path(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    target: BlockPos,
    max_path_length: f32,
    reach_range: i32,
    max_visited_nodes: usize,
) -> Option<GroundPath> {
    find_path_with_debug(
        level,
        mob,
        target,
        max_path_length,
        reach_range,
        max_visited_nodes,
        false,
    )
}

#[cfg(test)]
pub(super) fn find_path_with_debug(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    target: BlockPos,
    max_path_length: f32,
    reach_range: i32,
    max_visited_nodes: usize,
    capture_debug: bool,
) -> Option<GroundPath> {
    find_path_with_start_status(
        level,
        mob,
        std::slice::from_ref(&target),
        max_path_length,
        reach_range,
        max_visited_nodes,
        capture_debug,
    )
    .0
}

pub(super) fn find_path_with_start_status(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    targets: &[BlockPos],
    max_path_length: f32,
    reach_range: i32,
    max_visited_nodes: usize,
    capture_debug: bool,
) -> (Option<GroundPath>, bool) {
    if targets.is_empty() {
        return (None, false);
    }
    let mut path_cache = PooledWalkPathCache::take();
    let Some(start) = start_node(level, mob, &mut path_cache) else {
        return (None, false);
    };
    let path = search_from_start(
        level,
        mob,
        targets,
        SearchBudget {
            max_path_length,
            reach_range,
            max_visited_nodes,
            capture_debug,
        },
        start,
        |pos, cost, out| neighbors_into(level, mob, pos, cost, out, &mut path_cache),
    );
    let completed = path.is_some();
    (path, completed)
}

struct SearchTarget {
    pos: BlockPos,
    best_idx: u32,
    distance: f32,
    reached: bool,
}

impl SearchTarget {
    fn new(pos: BlockPos, start: BlockPos, start_idx: u32) -> Self {
        Self {
            pos,
            best_idx: start_idx,
            distance: distance(start, pos),
            reached: false,
        }
    }
}

fn best_target_distance(node_idx: u32, pos: BlockPos, targets: &mut [SearchTarget]) -> f32 {
    let mut best = f32::MAX;
    for target in targets {
        let heuristic = distance(pos, target.pos);
        if heuristic < target.distance {
            target.best_idx = node_idx;
            target.distance = heuristic;
        }
        best = best.min(heuristic);
    }
    best
}

#[derive(Clone, Copy)]
pub(super) struct SearchBudget {
    pub(super) max_path_length: f32,
    pub(super) reach_range: i32,
    pub(super) max_visited_nodes: usize,
    pub(super) capture_debug: bool,
}

pub(super) fn search_from_start(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    targets: &[BlockPos],
    budget: SearchBudget,
    start: SearchNode,
    mut neighbors_at: impl FnMut(BlockPos, f32, &mut Vec<SearchNode>),
) -> Option<GroundPath> {
    let SearchBudget {
        max_path_length,
        reach_range,
        max_visited_nodes,
        capture_debug,
    } = budget;
    let start_pos = start.public.pos;
    let (&first, _) = targets.split_first()?;
    let mut arena = PooledSearchArena::take();
    let start_idx = arena.insert_node(key(start_pos), start);
    let mut single = [SearchTarget::new(first, start_pos, start_idx)];
    // Keep single-target searches free of target-list allocations.
    let mut multiple = if targets.len() > 1 {
        targets
            .iter()
            .map(|&pos| SearchTarget::new(pos, start_pos, start_idx))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let targets = if targets.len() == 1 {
        &mut single[..]
    } else {
        multiple.as_mut_slice()
    };
    let start_h = best_target_distance(start_idx, start_pos, targets);
    {
        let start_node = &mut arena.nodes[start_idx as usize];
        start_node.h = start_h;
        start_node.f = start_h;
    }
    arena.heap_insert(start_idx);
    let sea_cutoff = if mob.frog {
        Some(
            level
                .sea_level()
                .expect("frog sea level checked before search")
                - 10,
        )
    } else {
        None
    };
    let mut reached = false;
    let mut count = 0usize;
    while !arena.open.is_empty() {
        count = count.saturating_add(1);
        if count >= max_visited_nodes {
            break;
        }
        let current_idx = arena.heap_pop()?;
        let (current_pos, current_g, current_walked, current_cost) = {
            let current = &mut arena.nodes[current_idx as usize];
            current.closed = true;
            (
                current.public.pos,
                current.g,
                current.walked_distance,
                current.public.cost_malus,
            )
        };
        for target in targets.iter_mut() {
            if manhattan(current_pos, target.pos) <= reach_range {
                target.reached = true;
                reached = true;
            }
        }
        if reached {
            break;
        }
        if capture_debug {
            arena.closed.push(current_idx);
        }
        if distance(current_pos, start_pos) >= max_path_length {
            continue;
        }
        arena.neighbors.clear();
        neighbors_at(current_pos, current_cost, &mut arena.neighbors);
        let neighbor_count = arena.neighbors.len();
        for index in 0..neighbor_count {
            let mut neighbor = arena.neighbors[index].clone();
            let neighbor_key = key(neighbor.public.pos);
            let existing = arena.by_pos.get(&neighbor_key).copied();
            if mob.amphibious && existing.is_some_and(|idx| arena.nodes[idx as usize].closed) {
                continue;
            }
            let deep_water = sea_cutoff.is_some_and(|cutoff| {
                neighbor.public.path_type == GroundPathType::Water && neighbor.public.pos.y < cutoff
            });
            if deep_water {
                neighbor.public.cost_malus = existing.map_or(neighbor.public.cost_malus, |idx| {
                    arena.nodes[idx as usize].public.cost_malus
                }) + 1.0;
            }
            let step_distance = distance(current_pos, neighbor.public.pos);
            let walked = current_walked + step_distance;
            let tentative_g = current_g + step_distance + neighbor.public.cost_malus;
            let deep_cost = neighbor.public.cost_malus;
            let neighbor_idx = match existing {
                Some(idx) => idx,
                None => arena.insert_node(neighbor_key, neighbor),
            };
            let entry = &mut arena.nodes[neighbor_idx as usize];
            if deep_water {
                entry.public.cost_malus = deep_cost;
            }
            if entry.closed {
                continue;
            }
            entry.walked_distance = walked;
            if walked >= max_path_length {
                continue;
            }
            if entry.heap_index != NO_INDEX && tentative_g >= entry.g {
                continue;
            }
            let was_open = entry.heap_index != NO_INDEX;
            let old_f = entry.f;
            let neighbor_pos = entry.public.pos;
            entry.came_from = current_idx;
            entry.g = tentative_g;
            entry.h = best_target_distance(neighbor_idx, neighbor_pos, targets) * 1.5;
            entry.f = entry.g + entry.h;
            if was_open {
                arena.heap_change_cost(neighbor_idx, old_f);
            } else {
                arena.heap_insert(neighbor_idx);
            }
        }
    }
    let mut path = targets
        .iter()
        .filter(|target| !reached || target.reached)
        .filter_map(|target| build_path(&arena.nodes, target.best_idx, target.pos, reached, None))
        .min_by(|left, right| {
            if reached {
                left.nodes.len().cmp(&right.nodes.len())
            } else {
                left.distance_to_target
                    .total_cmp(&right.distance_to_target)
                    .then_with(|| left.nodes.len().cmp(&right.nodes.len()))
            }
        })?;
    if capture_debug {
        let end_idx = *arena.by_pos.get(&key(path.nodes.last()?.pos))?;
        path.debug_data = Some(GroundPathDebugData {
            nodes: debug_path_nodes(&arena.nodes, end_idx),
            target_nodes: targets
                .iter()
                .map(|target| DebugPathNode {
                    x: target.pos.x,
                    y: target.pos.y,
                    z: target.pos.z,
                    walked_distance: 0.0,
                    cost_malus: 0.0,
                    closed: false,
                    path_type: DebugPathType::Blocked,
                    f: 0.0,
                })
                .collect(),
            open_set: arena
                .open
                .iter()
                .map(|&idx| debug_node(&arena.nodes[idx as usize]))
                .collect(),
            closed_set: arena
                .closed
                .iter()
                .map(|&idx| debug_node(&arena.nodes[idx as usize]))
                .collect(),
        });
    }
    Some(path)
}

pub(super) fn debug_node(node: &SearchNode) -> DebugPathNode {
    DebugPathNode {
        x: node.public.pos.x,
        y: node.public.pos.y,
        z: node.public.pos.z,
        walked_distance: node.walked_distance,
        cost_malus: node.public.cost_malus,
        closed: node.closed,
        path_type: debug_path_type(node.public.path_type),
        f: node.f,
    }
}

pub(super) fn debug_path_nodes(nodes: &[SearchNode], mut current: u32) -> Vec<DebugPathNode> {
    let mut path = Vec::new();
    while let Some(node) = nodes.get(current as usize) {
        path.push(debug_node(node));
        if node.came_from == NO_INDEX {
            break;
        }
        current = node.came_from;
    }
    path.reverse();
    path
}

fn start_node(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    if mob.amphibious && mob.in_water {
        return node_at(
            level,
            mob,
            BlockPos {
                x: mob.bounding_box.min_x.floor() as i32,
                y: mob.bounding_box.min_y.floor() as i32,
                z: mob.bounding_box.min_z.floor() as i32,
            },
            cache,
        );
    }
    let y = if mob.can_stand_on_fluid(level, mob.block_position)? {
        let mut cursor_y = mob.block_position.y;
        while cursor_y <= level.max_y()
            && mob.can_stand_on_fluid(
                level,
                BlockPos {
                    y: cursor_y,
                    ..mob.block_position
                },
            )?
        {
            cursor_y += 1;
        }
        cursor_y - 1
    } else if mob.can_float && mob.in_water {
        let mut cursor_y = mob.block_position.y;
        while cursor_y <= level.max_y()
            && is_water_start_state(
                level,
                BlockPos {
                    y: cursor_y,
                    ..mob.block_position
                },
            )?
        {
            cursor_y += 1;
        }
        cursor_y - 1
    } else if mob.on_ground {
        (mob.position.y + 0.5).floor() as i32
    } else {
        let mut cursor_y = (mob.position.y + 1.0).floor() as i32;
        let mut start_y = cursor_y;
        while cursor_y > level.min_y() {
            start_y = cursor_y;
            cursor_y -= 1;
            let below = BlockPos {
                y: cursor_y,
                ..mob.block_position
            };
            if !level.has_ticking_column_at(below) {
                return None;
            }
            let below_state = level.block_state(below);
            let is_open = if let Some(facts) =
                bbb_protocol::block_state_types::vanilla_block_state_path_facts(below_state)
            {
                facts.is_air() || facts.land_pathfindable()
            } else {
                !level
                    .content_registry()
                    .block_by_state_id(below_state)?
                    .0
                    .full_cube_collision
            };
            if !is_open {
                break;
            }
        }
        start_y
    };
    let primary = BlockPos {
        y,
        ..mob.block_position
    };
    if let Some(node) = accepted_start(level, mob, primary, cache) {
        return Some(node);
    }
    for (x, z) in [
        (mob.bounding_box.min_x, mob.bounding_box.min_z),
        (mob.bounding_box.min_x, mob.bounding_box.max_z),
        (mob.bounding_box.max_x, mob.bounding_box.min_z),
        (mob.bounding_box.max_x, mob.bounding_box.max_z),
    ] {
        if let Some(node) = accepted_start(
            level,
            mob,
            BlockPos {
                x: x.floor() as i32,
                y,
                z: z.floor() as i32,
            },
            cache,
        ) {
            return Some(node);
        }
    }
    node_at(level, mob, primary, cache)
}

fn accepted_start(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    let node = node_at(level, mob, pos, cache)?;
    (node.public.path_type != GroundPathType::Open && node.public.cost_malus >= 0.0).then_some(node)
}

#[cfg(test)]
fn neighbors(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    current: BlockPos,
    current_cost: f32,
) -> Vec<SearchNode> {
    let mut result = Vec::with_capacity(8);
    let mut cache = WalkPathCache::default();
    neighbors_into(level, mob, current, current_cost, &mut result, &mut cache);
    result
}

fn neighbors_into(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    current: BlockPos,
    current_cost: f32,
    result: &mut Vec<SearchNode>,
    cache: &mut WalkPathCache,
) {
    let directions = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let current_type = node_at(level, mob, current, cache)
        .map(|node| node.public.path_type)
        .unwrap_or(GroundPathType::Blocked);
    let above = path_type_for_mob(
        level,
        mob,
        BlockPos {
            y: current.y + 1,
            ..current
        },
        cache,
    );
    let jump_size = if above.is_some_and(|value| mob.pathfinding_malus(value) >= 0.0)
        && current_type != GroundPathType::StickyHoney
    {
        mob.step_height.max(1.0).floor() as i32
    } else {
        0
    };
    let Some(current_floor) = evaluator_floor_level(level, mob, current) else {
        return;
    };
    let mut cardinal: [Option<SearchNode>; 4] = [None, None, None, None];
    for (index, (dx, dz)) in directions.into_iter().enumerate() {
        let candidate = accepted_node(
            level,
            mob,
            BlockPos {
                x: current.x + dx,
                y: current.y,
                z: current.z + dz,
            },
            jump_size,
            current_floor,
            current_type,
            (dx, dz),
            cache,
        );
        if candidate
            .as_ref()
            .is_some_and(|node| neighbor_is_valid(node, current_cost))
        {
            result.push(candidate.as_ref().expect("checked candidate").clone());
        }
        cardinal[index] = candidate;
    }
    for index in 0..4 {
        let next = (index + 1) % 4;
        let (Some(first), Some(second)) = (&cardinal[index], &cardinal[next]) else {
            continue;
        };
        if first.public.pos.y > current.y
            || second.public.pos.y > current.y
            || first.public.path_type == GroundPathType::WalkableDoor
            || second.public.path_type == GroundPathType::WalkableDoor
            || mob.width > 1.0 && (first.public.cost_malus > 0.0 || second.public.cost_malus > 0.0)
        {
            continue;
        }
        let can_pass_between_posts = first.public.path_type == GroundPathType::Fence
            && second.public.path_type == GroundPathType::Fence
            && mob.width < 0.5;
        if !(first.public.pos.y < current.y
            || first.public.cost_malus >= 0.0
            || can_pass_between_posts)
            || !(second.public.pos.y < current.y
                || second.public.cost_malus >= 0.0
                || can_pass_between_posts)
        {
            continue;
        }
        let (dx1, dz1) = directions[index];
        let (dx2, dz2) = directions[next];
        if let Some(diagonal) = accepted_node(
            level,
            mob,
            BlockPos {
                x: current.x + dx1 + dx2,
                y: current.y,
                z: current.z + dz1 + dz2,
            },
            jump_size,
            current_floor,
            current_type,
            (dx1, dz1),
            cache,
        ) {
            if !diagonal.closed
                && diagonal.public.path_type != GroundPathType::WalkableDoor
                && diagonal.public.cost_malus >= 0.0
            {
                result.push(diagonal);
            }
        }
    }
    if mob.amphibious {
        for dy in [1, -1] {
            if let Some(node) = accepted_node(
                level,
                mob,
                BlockPos {
                    y: current.y + dy,
                    ..current
                },
                if dy > 0 {
                    jump_size.saturating_sub(1)
                } else {
                    jump_size
                },
                current_floor,
                current_type,
                (0, 0),
                cache,
            ) {
                if neighbor_is_valid(&node, current_cost)
                    && node.public.path_type == GroundPathType::Water
                    && (dy > 0 || current_type != GroundPathType::Trapdoor)
                {
                    result.push(node);
                }
            }
        }
    }
}

fn neighbor_is_valid(node: &SearchNode, current_cost: f32) -> bool {
    !node.closed && (node.public.cost_malus >= 0.0 || current_cost < 0.0)
}

#[allow(clippy::too_many_arguments)]
fn accepted_node(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    jump_size: i32,
    source_floor: f64,
    current_type: GroundPathType,
    travel_direction: (i32, i32),
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    let floor = evaluator_floor_level(level, mob, pos)?;
    if floor - source_floor > mob.step_height.max(1.125) {
        return None;
    }
    let path_type = path_type_for_mob(level, mob, pos, cache)?;
    let mut node = (mob.pathfinding_malus(path_type) >= 0.0)
        .then(|| SearchNode::new(pos, path_type, mob.pathfinding_malus(path_type)));
    if is_partial_collision(current_type)
        && node
            .as_ref()
            .is_some_and(|node| node.public.cost_malus >= 0.0)
        && !can_reach_without_collision(level, mob, pos)
    {
        node = None;
    }
    if path_type != GroundPathType::Walkable
        && !(mob.amphibious && path_type == GroundPathType::Water)
    {
        if node
            .as_ref()
            .is_none_or(|node| node.public.cost_malus < 0.0)
            && jump_size > 0
            && (path_type != GroundPathType::Fence || mob.can_walk_over_fences)
            && !matches!(
                path_type,
                GroundPathType::UnpassableRail
                    | GroundPathType::Trapdoor
                    | GroundPathType::PowderSnow
            )
        {
            let jumped = accepted_node(
                level,
                mob,
                BlockPos {
                    y: pos.y + 1,
                    ..pos
                },
                jump_size - 1,
                source_floor,
                current_type,
                travel_direction,
                cache,
            );
            node = jumped.filter(|node| {
                mob.width >= 1.0
                    || !matches!(
                        node.public.path_type,
                        GroundPathType::Open | GroundPathType::Walkable
                    )
                    || jump_corridor_free(level, mob, pos, node.public.pos, travel_direction)
            });
        } else if path_type == GroundPathType::Water && !mob.can_float {
            node = first_non_water_below(level, mob, pos, node, cache);
        } else if path_type == GroundPathType::Open {
            node = ground_below(level, mob, pos, cache);
        } else if matches!(
            path_type,
            GroundPathType::Fence | GroundPathType::DoorWoodClosed | GroundPathType::DoorIronClosed
        ) && node.is_none()
        {
            let mut closed = SearchNode::new(pos, path_type, path_type.default_malus());
            closed.closed = true;
            node = Some(closed);
        }
    }
    node
}

fn first_non_water_below(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    mut best: Option<SearchNode>,
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    let mut y = pos.y - 1;
    while y > level.min_y() {
        let candidate = BlockPos { y, ..pos };
        let path_type = path_type_for_mob(level, mob, candidate, cache)?;
        if path_type != GroundPathType::Water {
            return best;
        }
        best = Some(SearchNode::new(
            candidate,
            path_type,
            mob.pathfinding_malus(path_type),
        ));
        y -= 1;
    }
    best
}

#[cfg(test)]
pub(super) fn accepted_node_pos(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    jump_size: i32,
    source_floor: f64,
    current_type: GroundPathType,
    travel_direction: (i32, i32),
) -> Option<BlockPos> {
    let mut cache = WalkPathCache::default();
    accepted_node(
        level,
        mob,
        pos,
        jump_size,
        source_floor,
        current_type,
        travel_direction,
        &mut cache,
    )
    .map(|node| node.public.pos)
}

fn ground_below(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    origin: BlockPos,
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    for fall in 1.. {
        let pos = BlockPos {
            y: origin.y - fall,
            ..origin
        };
        if fall > mob.max_fall_distance {
            return Some(SearchNode::new(
                pos,
                GroundPathType::Blocked,
                GroundPathType::Blocked.default_malus(),
            ));
        }
        let path_type = path_type_for_mob(level, mob, pos, cache)?;
        if path_type == GroundPathType::Open {
            continue;
        }
        if mob.pathfinding_malus(path_type) >= 0.0 {
            return Some(SearchNode::new(
                pos,
                path_type,
                mob.pathfinding_malus(path_type),
            ));
        }
        return Some(SearchNode::new(
            pos,
            GroundPathType::Blocked,
            GroundPathType::Blocked.default_malus(),
        ));
    }
    None
}

fn node_at(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    cache: &mut WalkPathCache,
) -> Option<SearchNode> {
    let path_type = path_type_for_mob(level, mob, pos, cache)?;
    Some(SearchNode::new(
        pos,
        path_type,
        mob.pathfinding_malus(path_type),
    ))
}

// Creaking.java:593.
const CREAKING_MAX_DISTANCE_TO_HOME_SQ: f64 = 1024.0;

fn path_type_for_mob(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    pos: BlockPos,
    cache: &mut WalkPathCache,
) -> Option<GroundPathType> {
    let pos_key = key(pos);
    if let Some(&cached) = cache.mob_types.get(&pos_key) {
        return cached;
    }
    let eval_ground = |sample: BlockPos| {
        if let Some(home) = mob.creaking_home_pos {
            let distance_to_home = |pos: BlockPos| {
                let x = f64::from(pos.x) - f64::from(home.x);
                let y = f64::from(pos.y) - f64::from(home.y);
                let z = f64::from(pos.z) - f64::from(home.z);
                x * x + y * y + z * z
            };
            let distance = distance_to_home(sample);
            if distance > CREAKING_MAX_DISTANCE_TO_HOME_SQ
                && distance >= distance_to_home(mob.block_position)
            {
                return Some(GroundPathType::Blocked);
            }
        }
        walk_path_type(level, sample)
    };
    let sample_types = &mut cache.sample_types;
    let ground_types = &mut cache.ground_types;
    let resolved = aggregate_path_type_for_mob(
        mob,
        pos,
        |sample| {
            let sample_key = key(sample);
            if let Some(&cached) = sample_types.get(&sample_key) {
                return cached;
            }
            let value = if mob.frog {
                super::amphibious::frog_path_type(level, sample)
            } else if mob.amphibious {
                super::amphibious::path_type(level, sample)
            } else {
                eval_ground(sample)
            };
            sample_types.insert(sample_key, value);
            value
        },
        |sample| {
            let sample_key = key(sample);
            if let Some(&cached) = ground_types.get(&sample_key) {
                return cached;
            }
            let value = eval_ground(sample);
            ground_types.insert(sample_key, value);
            value
        },
    );
    cache.mob_types.insert(pos_key, resolved);
    resolved
}

pub(super) fn aggregate_path_type_for_mob(
    mob: MobPathingSnapshot,
    pos: BlockPos,
    mut sample: impl FnMut(BlockPos) -> Option<GroundPathType>,
    mut node_type: impl FnMut(BlockPos) -> Option<GroundPathType>,
) -> Option<GroundPathType> {
    let width = (mob.width + 1.0).floor() as i32;
    let height = (mob.height + 1.0).floor() as i32;
    let depth = width;
    let mut types_mask = 0u32;
    for dx in 0..width {
        for dy in 0..height {
            for dz in 0..depth {
                let mut value = sample(BlockPos {
                    x: pos.x + dx,
                    y: pos.y + dy,
                    z: pos.z + dz,
                })?;
                if value == GroundPathType::DoorWoodClosed
                    && mob.can_open_doors
                    && mob.can_pass_doors
                {
                    value = GroundPathType::WalkableDoor;
                }
                if value == GroundPathType::DoorOpen && !mob.can_pass_doors {
                    value = GroundPathType::Blocked;
                }
                if value == GroundPathType::Rail {
                    let here = node_type(mob.block_position)?;
                    let below = node_type(BlockPos {
                        y: mob.block_position.y - 1,
                        ..mob.block_position
                    })?;
                    if here != GroundPathType::Rail && below != GroundPathType::Rail {
                        value = GroundPathType::UnpassableRail;
                    }
                }
                types_mask |= 1u32 << (value as u8);
            }
        }
    }
    if types_mask == 0 {
        return None;
    }
    if types_mask.count_ones() == 1 {
        return Some(GroundPathType::from_ordinal(
            types_mask.trailing_zeros() as usize
        ));
    }
    if (types_mask & (1u32 << (GroundPathType::Fence as u8))) != 0 {
        return Some(GroundPathType::Fence);
    }
    if (types_mask & (1u32 << (GroundPathType::UnpassableRail as u8))) != 0 {
        return Some(GroundPathType::UnpassableRail);
    }
    let mut selected = GroundPathType::Blocked;
    let mut highest = mob.pathfinding_malus(selected);
    let mut bits = types_mask;
    while bits != 0 {
        let ordinal = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        let value = GroundPathType::from_ordinal(ordinal);
        let malus = mob.pathfinding_malus(value);
        if malus < 0.0 {
            return Some(value);
        }
        if malus >= highest {
            highest = malus;
            selected = value;
        }
    }
    let current = node_type(pos)?;
    if width > 1 {
        return Some(
            if mob.pathfinding_malus(current) < highest
                && mob.pathfinding_malus(GroundPathType::BigMobsCloseToDanger) < highest
            {
                GroundPathType::BigMobsCloseToDanger
            } else {
                selected
            },
        );
    }
    if current == GroundPathType::Open && selected != GroundPathType::Open && highest == 0.0 {
        Some(GroundPathType::Open)
    } else {
        Some(selected)
    }
}

const fn is_partial_collision(path_type: GroundPathType) -> bool {
    matches!(
        path_type,
        GroundPathType::Fence | GroundPathType::DoorWoodClosed | GroundPathType::DoorIronClosed
    )
}

fn can_reach_without_collision(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    target: BlockPos,
) -> bool {
    let mut body = mob.bounding_box;
    let x_size = body.max_x - body.min_x;
    let y_size = body.max_y - body.min_y;
    let z_size = body.max_z - body.min_z;
    let delta = bbb_protocol::packets::Vec3d {
        x: f64::from(target.x) - mob.position.x + x_size / 2.0,
        y: f64::from(target.y) - mob.position.y + y_size / 2.0,
        z: f64::from(target.z) - mob.position.z + z_size / 2.0,
    };
    let length = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    let average_size = (x_size + y_size + z_size) / 3.0;
    let steps = (length / average_size).ceil().max(1.0) as i32;
    let step = bbb_protocol::packets::Vec3d {
        x: delta.x / f64::from(steps),
        y: delta.y / f64::from(steps),
        z: delta.z / f64::from(steps),
    };
    for _ in 0..steps {
        body = body.moved(step);
        if !aabb_collision_free(level, body) {
            return false;
        }
    }
    true
}

fn jump_corridor_free(
    level: &LevelCtx<'_, '_>,
    mob: MobPathingSnapshot,
    candidate: BlockPos,
    node_above: BlockPos,
    travel_direction: (i32, i32),
) -> bool {
    let half = f64::from(mob.width) / 2.0;
    let center_x = f64::from(candidate.x - travel_direction.0) + 0.5;
    let center_z = f64::from(candidate.z - travel_direction.1) + 0.5;
    let Some(target_floor) = evaluator_floor_level(level, mob, node_above) else {
        return false;
    };
    let Some(source_top) = evaluator_floor_level(
        level,
        mob,
        BlockPos {
            x: candidate.x - travel_direction.0,
            y: candidate.y + 1,
            z: candidate.z - travel_direction.1,
        },
    ) else {
        return false;
    };
    aabb_collision_free(
        level,
        bbb_server_types::geometry::Aabb {
            min_x: center_x - half,
            min_y: source_top + 0.001,
            min_z: center_z - half,
            max_x: center_x + half,
            max_y: target_floor + f64::from(mob.height) - 0.002,
            max_z: center_z + half,
        },
    )
}

fn aabb_collision_free(level: &LevelCtx<'_, '_>, body: bbb_server_types::geometry::Aabb) -> bool {
    let min = BlockPos {
        x: (body.min_x - 1.0e-7).floor() as i32 - 1,
        y: ((body.min_y - 1.0e-7).floor() as i32 - 1).max(level.min_y()),
        z: (body.min_z - 1.0e-7).floor() as i32 - 1,
    };
    let max = BlockPos {
        x: (body.max_x + 1.0e-7).floor() as i32 + 1,
        y: ((body.max_y + 1.0e-7).floor() as i32 + 1).min(level.max_y()),
        z: (body.max_z + 1.0e-7).floor() as i32 + 1,
    };
    let mut scratch = Vec::new();
    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                let block = BlockPos { x, y, z };
                if !level.has_ticking_column_at(block) {
                    return false;
                }
                let offset = bbb_protocol::packets::Vec3d {
                    x: f64::from(x),
                    y: f64::from(y),
                    z: f64::from(z),
                };
                let state = level.block_state(block);
                if let Some(boxes) =
                    bbb_server_types::geometry::static_block_collision_box_iter(state)
                {
                    for shape in boxes {
                        if shape.moved(offset).intersects(body) {
                            return false;
                        }
                    }
                } else {
                    scratch.clear();
                    level.push_block_collision_boxes(block, &mut scratch);
                    for shape in &scratch {
                        if shape.moved(offset).intersects(body) {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

pub(super) fn build_path(
    nodes: &[SearchNode],
    mut current: u32,
    target: BlockPos,
    reached: bool,
    debug_data: Option<GroundPathDebugData>,
) -> Option<GroundPath> {
    let mut path = vec![nodes.get(current as usize)?.public];
    while nodes[current as usize].came_from != NO_INDEX {
        current = nodes[current as usize].came_from;
        path.push(nodes.get(current as usize)?.public);
    }
    path.reverse();
    let distance_to_target = path
        .last()
        .map_or(f32::MAX, |node| manhattan(node.pos, target) as f32);
    Some(GroundPath {
        nodes: path,
        next_node_index: 0,
        target,
        distance_to_target,
        reached,
        debug_data,
    })
}

pub(super) fn distance(left: BlockPos, right: BlockPos) -> f32 {
    let dx = (left.x - right.x) as f32;
    let dy = (left.y - right.y) as f32;
    let dz = (left.z - right.z) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub(super) fn manhattan(left: BlockPos, right: BlockPos) -> i32 {
    left.x.abs_diff(right.x) as i32
        + left.y.abs_diff(right.y) as i32
        + left.z.abs_diff(right.z) as i32
}

#[cfg(test)]
mod malus_tests;
