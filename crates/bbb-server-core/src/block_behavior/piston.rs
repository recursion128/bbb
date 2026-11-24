//! Piston signal, block-event, and structure-movement owner.

use super::LevelCtx;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::{
    vanilla_block_state_support_facts, VanillaPistonPushReaction,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_CRYING_OBSIDIAN_ID, VANILLA_BLOCK_HONEY_BLOCK_ID,
    VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_OBSIDIAN_ID, VANILLA_BLOCK_PISTON_HEAD_ID,
    VANILLA_BLOCK_PISTON_ID, VANILLA_BLOCK_REINFORCED_DEEPSLATE_ID,
    VANILLA_BLOCK_RESPAWN_ANCHOR_ID, VANILLA_BLOCK_SLIME_BLOCK_ID, VANILLA_BLOCK_STICKY_PISTON_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::block_queries::block_state_has_block_entity;
use bbb_server_types::directions::{opposite, relative};

use crate::block_entity::piston::PistonMovingSnapshot;

use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, state_is_block, with_property, BlockTriggerOutcome,
    DIRECTION_VALUES,
};

const FACING: &str = "facing";
const EXTENDED: &str = "extended";
const TYPE: &str = "type";

const TRIGGER_EXTEND: i32 = 0;
const TRIGGER_CONTRACT: i32 = 1;
const TRIGGER_DROP: i32 = 2;
const MAX_PUSH_DEPTH: usize = 12;

/// One resolved piston movement.
#[derive(Debug, Clone, PartialEq)]
pub struct PistonMovementPlan {
    pub piston_pos: BlockPos,
    pub piston_state: i32,
    pub direction: Direction,
    pub extending: bool,
    pub sticky: bool,
    pub event: i32,
    pub clear_arm: bool,
    pub source_base_state: Option<i32>,
    pub moved: Vec<(BlockPos, i32)>,
    pub broken: Vec<(BlockPos, i32)>,
    pub finalize: Vec<(BlockPos, PistonMovingSnapshot)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Resolution {
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}

#[must_use]
pub fn is_piston_base(block_state_id: i32) -> bool {
    matches!(
        block_type_of(block_state_id),
        Some(VANILLA_BLOCK_PISTON_ID | VANILLA_BLOCK_STICKY_PISTON_ID)
    )
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, old_state: i32) {
    if block_type_of(old_state) != block_type_of(state) {
        check_if_extend(level, pos, state);
    }
}

pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    check_if_extend(level, pos, state);
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    check_if_extend(level, pos, state);
}

fn check_if_extend(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let Some(direction) = direction_property(state, FACING) else {
        return;
    };
    let powered = neighbor_signal(level, pos, direction);
    let Some(extended) = bool_property(state, EXTENDED) else {
        return;
    };
    let Some(block_type_id) = block_type_of(state) else {
        return;
    };
    if powered && !extended {
        if StructureResolver::new(&*level, pos, direction, true)
            .resolve()
            .is_some()
        {
            level.block_event(
                pos,
                block_type_id,
                TRIGGER_EXTEND,
                direction_3d_id(direction),
            );
        }
    } else if !powered && extended {
        let two_pos = relative(relative(pos, direction), direction);
        let mut event = TRIGGER_CONTRACT;
        let two_state = level.block_state(two_pos);
        if bbb_server_types::block_queries::block_type_of_with_registry(
            two_state,
            level.content_registry(),
        )
        .is_none()
        {
            return;
        }
        if state_is_block(two_state, VANILLA_BLOCK_MOVING_PISTON_ID)
            && direction_property(two_state, FACING) == Some(direction)
        {
            if let Some(moving) = level.piston_moving_block_entity(two_pos) {
                // `PistonBaseBlock.java:119`: `getProgress(0.0F)` is `progressO` (`last_progress`).
                if moving.extending
                    && (moving.last_progress < 0.5
                        || level.game_time() == moving.last_ticked
                        || level.piston_is_handling_tick())
                {
                    event = TRIGGER_DROP;
                }
            }
        }
        level.block_event(pos, block_type_id, event, direction_3d_id(direction));
    }
}

fn neighbor_signal(level: &mut LevelCtx<'_, '_>, pos: BlockPos, push_direction: Direction) -> bool {
    let above = relative(pos, Direction::Up);
    DIRECTION_VALUES.into_iter().any(|direction| {
        direction != push_direction && level.has_signal(relative(pos, direction), direction)
    }) || level.has_signal(pos, Direction::Down)
        || DIRECTION_VALUES.into_iter().any(|direction| {
            direction != Direction::Down && level.has_signal(relative(above, direction), direction)
        })
}

pub(super) fn trigger_event(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    event: i32,
    event_direction: i32,
) -> BlockTriggerOutcome {
    let Some(direction) = direction_property(state, FACING) else {
        return BlockTriggerOutcome::not_triggered();
    };
    let Some(extended_state) = with_property(state, EXTENDED, "true") else {
        return BlockTriggerOutcome::not_triggered();
    };
    let powered = neighbor_signal(level, pos, direction);
    if powered && matches!(event, TRIGGER_CONTRACT | TRIGGER_DROP) {
        level.set_block(
            pos,
            extended_state,
            2,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        return BlockTriggerOutcome::not_triggered();
    }
    if !powered && event == TRIGGER_EXTEND {
        return BlockTriggerOutcome::not_triggered();
    }

    let Some(sticky) = sticky_base(state) else {
        return BlockTriggerOutcome::not_triggered();
    };
    let plan = if event == TRIGGER_EXTEND {
        movement_plan(level, pos, state, direction, sticky, true, event)
    } else if matches!(event, TRIGGER_CONTRACT | TRIGGER_DROP) {
        Some(retract_plan(
            level,
            pos,
            state,
            direction,
            sticky,
            event,
            direction_from_3d_id(event_direction & 7),
        ))
    } else {
        return BlockTriggerOutcome::not_triggered();
    };
    let Some(plan) = plan else {
        return BlockTriggerOutcome::not_triggered();
    };
    level.piston_apply_movement(&plan);

    let (sound, pitch, game_event, affected_state) = if event == TRIGGER_EXTEND {
        (
            "minecraft:block.piston.extend",
            level.level_random().next_float() * 0.25 + 0.6,
            VanillaGameEvent::BLOCK_ACTIVATE,
            extended_state,
        )
    } else {
        (
            "minecraft:block.piston.contract",
            level.level_random().next_float() * 0.15 + 0.6,
            VanillaGameEvent::BLOCK_DEACTIVATE,
            moving_piston_state(direction, sticky),
        )
    };
    level.play_block_sound_with_volume_pitch(pos, sound, 0.5, pitch);
    level.block_game_event(game_event, pos, Some(affected_state));
    BlockTriggerOutcome {
        triggered: true,
        sound: None,
    }
}

fn movement_plan(
    level: &LevelCtx<'_, '_>,
    piston_pos: BlockPos,
    piston_state: i32,
    direction: Direction,
    sticky: bool,
    extending: bool,
    event: i32,
) -> Option<PistonMovementPlan> {
    let (moved, broken) = StructureResolver::new(level, piston_pos, direction, extending)
        .resolve()?
        .with_states(level);
    Some(PistonMovementPlan {
        piston_pos,
        piston_state,
        direction,
        extending,
        sticky,
        event,
        clear_arm: false,
        source_base_state: None,
        moved,
        broken,
        finalize: Vec::new(),
    })
}

fn retract_plan(
    level: &LevelCtx<'_, '_>,
    piston_pos: BlockPos,
    piston_state: i32,
    direction: Direction,
    sticky: bool,
    event: i32,
    event_direction: Direction,
) -> PistonMovementPlan {
    let arm = relative(piston_pos, direction);
    let mut finalize = Vec::new();
    if state_is_block(level.block_state(arm), VANILLA_BLOCK_MOVING_PISTON_ID) {
        if let Some(moving) = level.piston_moving_block_entity(arm) {
            finalize.push((arm, moving));
        }
    }

    let mut moved = Vec::new();
    let mut broken = Vec::new();
    let mut clear_arm = !sticky;
    if sticky {
        let two = relative(arm, direction);
        let state = level.block_state(two);
        let moving_piece = match level.piston_moving_block_entity(two) {
            Some(moving)
                if state_is_block(state, VANILLA_BLOCK_MOVING_PISTON_ID)
                    && moving.facing == direction
                    && moving.extending =>
            {
                finalize.push((two, moving));
                true
            }
            _ => false,
        };
        if !moving_piece
            && event == TRIGGER_CONTRACT
            && !state_is_air(state)
            && is_pushable(level, state, two, opposite(direction), false, direction)
            && (vanilla_block_state_support_facts(state).map(|facts| facts.piston_push_reaction())
                == Some(VanillaPistonPushReaction::Normal)
                || is_piston_base(state))
        {
            // `moveBlocks` clears the head first; an unresolved pull moves nothing.
            if let Some(resolution) =
                StructureResolver::new(level, piston_pos, direction, false).resolve()
            {
                (moved, broken) = resolution.with_states(level);
            }
        }
        clear_arm = !moving_piece && moved.is_empty();
    }
    PistonMovementPlan {
        piston_pos,
        piston_state,
        direction,
        extending: false,
        sticky,
        event,
        clear_arm,
        source_base_state: contracted_base_state(piston_state, event_direction),
        moved,
        broken,
        finalize,
    }
}

type PistonStateLists = (Vec<(BlockPos, i32)>, Vec<(BlockPos, i32)>);

impl Resolution {
    fn with_states(self, level: &LevelCtx<'_, '_>) -> PistonStateLists {
        let states = |positions: Vec<BlockPos>| {
            positions
                .into_iter()
                .map(|pos| (pos, level.block_state(pos)))
                .collect()
        };
        (states(self.to_push), states(self.to_destroy))
    }
}

struct StructureResolver<'a, 'b, 'w> {
    level: &'a LevelCtx<'b, 'w>,
    piston_pos: BlockPos,
    extending: bool,
    start_pos: BlockPos,
    push_direction: Direction,
    piston_direction: Direction,
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}

impl<'a, 'b, 'w> StructureResolver<'a, 'b, 'w> {
    fn new(
        level: &'a LevelCtx<'b, 'w>,
        piston_pos: BlockPos,
        direction: Direction,
        extending: bool,
    ) -> Self {
        Self {
            level,
            piston_pos,
            extending,
            start_pos: if extending {
                relative(piston_pos, direction)
            } else {
                relative(relative(piston_pos, direction), direction)
            },
            push_direction: if extending {
                direction
            } else {
                opposite(direction)
            },
            piston_direction: direction,
            to_push: Vec::new(),
            to_destroy: Vec::new(),
        }
    }

    fn state(&self, pos: BlockPos) -> i32 {
        let state = self.level.block_state(pos);
        if !self.extending
            && pos == relative(self.piston_pos, self.piston_direction)
            && (state_is_block(state, VANILLA_BLOCK_PISTON_HEAD_ID)
                || state_is_block(state, VANILLA_BLOCK_MOVING_PISTON_ID))
        {
            default_block_state_id(VANILLA_BLOCK_AIR_ID)
        } else {
            state
        }
    }

    fn resolve(mut self) -> Option<Resolution> {
        let state = self.state(self.start_pos);
        if !is_pushable(
            self.level,
            state,
            self.start_pos,
            self.push_direction,
            false,
            self.piston_direction,
        ) {
            if self.extending
                && vanilla_block_state_support_facts(state)
                    .map(|facts| facts.piston_push_reaction())
                    == Some(VanillaPistonPushReaction::Destroy)
            {
                self.to_destroy.push(self.start_pos);
            } else {
                return None;
            }
        } else {
            if !self.add_block_line(self.start_pos, self.push_direction) {
                return None;
            }
            let mut index = 0;
            while index < self.to_push.len() {
                let pos = self.to_push[index];
                if is_sticky(self.state(pos)) && !self.add_branching_blocks(pos) {
                    return None;
                }
                index += 1;
            }
        }
        Some(Resolution {
            to_push: self.to_push,
            to_destroy: self.to_destroy,
        })
    }

    fn add_block_line(&mut self, start: BlockPos, direction: Direction) -> bool {
        let mut next_state = self.state(start);
        if state_is_air(next_state)
            || !is_pushable(
                self.level,
                next_state,
                start,
                self.push_direction,
                false,
                direction,
            )
            || start == self.piston_pos
            || self.to_push.contains(&start)
        {
            return true;
        }
        let mut block_count = 1;
        if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
            return false;
        }
        while is_sticky(next_state) {
            let pos = relative_n(start, opposite(self.push_direction), block_count as i32);
            let previous = next_state;
            next_state = self.state(pos);
            if state_is_air(next_state)
                || !can_stick(previous, next_state)
                || !is_pushable(
                    self.level,
                    next_state,
                    pos,
                    self.push_direction,
                    false,
                    opposite(self.push_direction),
                )
                || pos == self.piston_pos
            {
                break;
            }
            block_count += 1;
            if block_count + self.to_push.len() > MAX_PUSH_DEPTH {
                return false;
            }
        }
        let mut blocks_added = 0;
        for offset in (0..block_count).rev() {
            self.to_push.push(relative_n(
                start,
                opposite(self.push_direction),
                offset as i32,
            ));
            blocks_added += 1;
        }
        let mut distance = 1;
        loop {
            let pos = relative_n(start, self.push_direction, distance);
            if let Some(collision_pos) = self.to_push.iter().position(|candidate| *candidate == pos)
            {
                self.reorder_at_collision(blocks_added, collision_pos);
                for index in 0..=collision_pos + blocks_added {
                    let branch = self.to_push[index];
                    if is_sticky(self.state(branch)) && !self.add_branching_blocks(branch) {
                        return false;
                    }
                }
                return true;
            }
            next_state = self.state(pos);
            if state_is_air(next_state) {
                return true;
            }
            if !is_pushable(
                self.level,
                next_state,
                pos,
                self.push_direction,
                true,
                self.push_direction,
            ) || pos == self.piston_pos
            {
                return false;
            }
            if vanilla_block_state_support_facts(next_state)
                .map(|facts| facts.piston_push_reaction())
                == Some(VanillaPistonPushReaction::Destroy)
            {
                self.to_destroy.push(pos);
                return true;
            }
            if self.to_push.len() >= MAX_PUSH_DEPTH {
                return false;
            }
            self.to_push.push(pos);
            blocks_added += 1;
            distance += 1;
        }
    }

    fn reorder_at_collision(&mut self, blocks_added: usize, collision_pos: usize) {
        let last = self.to_push.len() - blocks_added;
        let mut reordered = self.to_push[..collision_pos].to_vec();
        reordered.extend_from_slice(&self.to_push[last..]);
        reordered.extend_from_slice(&self.to_push[collision_pos..last]);
        self.to_push = reordered;
    }

    fn add_branching_blocks(&mut self, from: BlockPos) -> bool {
        let from_state = self.state(from);
        for direction in DIRECTION_VALUES {
            if same_axis(direction, self.push_direction) {
                continue;
            }
            let neighbour = relative(from, direction);
            let neighbour_state = self.state(neighbour);
            if can_stick(neighbour_state, from_state) && !self.add_block_line(neighbour, direction)
            {
                return false;
            }
        }
        true
    }
}

fn is_pushable(
    level: &LevelCtx<'_, '_>,
    state: i32,
    pos: BlockPos,
    direction: Direction,
    allow_destroyable: bool,
    connection_direction: Direction,
) -> bool {
    if pos.y < level.get_min_y()
        || pos.y > level.get_max_y()
        || !level
            .world_border()
            .is_some_and(|border| border.contains_block_at(pos, level.game_time()))
    {
        return false;
    }
    if state_is_air(state) {
        return true;
    }
    let (Some(block), Some(facts)) = (
        block_type_of(state),
        vanilla_block_state_support_facts(state),
    ) else {
        return false;
    };
    if matches!(
        block,
        VANILLA_BLOCK_OBSIDIAN_ID
            | VANILLA_BLOCK_CRYING_OBSIDIAN_ID
            | VANILLA_BLOCK_RESPAWN_ANCHOR_ID
            | VANILLA_BLOCK_REINFORCED_DEEPSLATE_ID
    ) || (direction == Direction::Down && pos.y == level.get_min_y())
        || (direction == Direction::Up && pos.y == level.get_max_y())
    {
        return false;
    }
    if matches!(
        block,
        VANILLA_BLOCK_PISTON_ID | VANILLA_BLOCK_STICKY_PISTON_ID
    ) {
        if bool_property(state, EXTENDED) != Some(false) {
            return false;
        }
    } else {
        if facts.piston_unbreakable() {
            return false;
        }
        match facts.piston_push_reaction() {
            VanillaPistonPushReaction::Block => return false,
            VanillaPistonPushReaction::Destroy => return allow_destroyable,
            VanillaPistonPushReaction::PushOnly => return direction == connection_direction,
            VanillaPistonPushReaction::Normal | VanillaPistonPushReaction::Ignore => {}
        }
    }
    !block_state_has_block_entity(state)
}

fn state_is_air(state: i32) -> bool {
    bbb_server_types::fluid::block_state_is_air(state)
}

fn sticky_base(state: i32) -> Option<bool> {
    match block_type_of(state)? {
        VANILLA_BLOCK_PISTON_ID => Some(false),
        VANILLA_BLOCK_STICKY_PISTON_ID => Some(true),
        _ => None,
    }
}

fn is_sticky(state: i32) -> bool {
    matches!(
        block_type_of(state),
        Some(VANILLA_BLOCK_SLIME_BLOCK_ID | VANILLA_BLOCK_HONEY_BLOCK_ID)
    )
}

fn can_stick(first: i32, second: i32) -> bool {
    let first = block_type_of(first);
    let second = block_type_of(second);
    if matches!(
        (first, second),
        (
            Some(VANILLA_BLOCK_HONEY_BLOCK_ID),
            Some(VANILLA_BLOCK_SLIME_BLOCK_ID)
        ) | (
            Some(VANILLA_BLOCK_SLIME_BLOCK_ID),
            Some(VANILLA_BLOCK_HONEY_BLOCK_ID)
        )
    ) {
        false
    } else {
        first.is_some_and(|block| {
            matches!(
                block,
                VANILLA_BLOCK_SLIME_BLOCK_ID | VANILLA_BLOCK_HONEY_BLOCK_ID
            )
        }) || second.is_some_and(|block| {
            matches!(
                block,
                VANILLA_BLOCK_SLIME_BLOCK_ID | VANILLA_BLOCK_HONEY_BLOCK_ID
            )
        })
    }
}

pub fn moving_piston_state(direction: Direction, sticky: bool) -> i32 {
    oriented_state(VANILLA_BLOCK_MOVING_PISTON_ID, direction, sticky)
}

pub fn piston_head_state(direction: Direction, sticky: bool) -> i32 {
    oriented_state(VANILLA_BLOCK_PISTON_HEAD_ID, direction, sticky)
}

fn oriented_state(block: i32, direction: Direction, sticky: bool) -> i32 {
    let state = default_block_state_id(block);
    with_property(state, FACING, direction_serialized_name(direction))
        .and_then(|state| with_property(state, TYPE, if sticky { "sticky" } else { "normal" }))
        .unwrap_or(state)
}

pub fn contracted_base_state(base_state: i32, event_direction: Direction) -> Option<i32> {
    let block = block_type_of(base_state)?;
    if !matches!(
        block,
        VANILLA_BLOCK_PISTON_ID | VANILLA_BLOCK_STICKY_PISTON_ID
    ) {
        return None;
    }
    let state = default_block_state_id(block);
    with_property(state, FACING, direction_serialized_name(event_direction))
}

fn relative_n(mut pos: BlockPos, direction: Direction, distance: i32) -> BlockPos {
    for _ in 0..distance {
        pos = relative(pos, direction);
    }
    pos
}

fn same_axis(left: Direction, right: Direction) -> bool {
    matches!(
        (left, right),
        (
            Direction::Down | Direction::Up,
            Direction::Down | Direction::Up
        ) | (
            Direction::North | Direction::South,
            Direction::North | Direction::South
        ) | (
            Direction::West | Direction::East,
            Direction::West | Direction::East
        )
    )
}

fn direction_3d_id(direction: Direction) -> i32 {
    match direction {
        Direction::Down => 0,
        Direction::Up => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::West => 4,
        Direction::East => 5,
    }
}

fn direction_from_3d_id(id: i32) -> Direction {
    match id.rem_euclid(6) {
        0 => Direction::Down,
        1 => Direction::Up,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::West,
        _ => Direction::East,
    }
}

pub fn source_positions(plan: &PistonMovementPlan) -> Vec<BlockPos> {
    plan.moved.iter().map(|(pos, _)| *pos).collect()
}
