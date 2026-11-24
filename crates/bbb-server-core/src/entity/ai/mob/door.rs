use super::{GroundNavigation, MobAiContext, MobAiRuntime};
use crate::block_behavior::door;
use crate::entity::ai::Goal;
use crate::entity::raider::RaiderRaid;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_property;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::{BlockPos, Vec3d};

#[derive(Debug, Clone, Copy)]
pub(super) struct BreakDoorState {
    pos: BlockPos,
    has_door: bool,
    break_time: i32,
    last_progress: i32,
}
impl Default for BreakDoorState {
    fn default() -> Self {
        Self {
            pos: BlockPos { x: 0, y: 0, z: 0 },
            has_door: false,
            break_time: 0,
            last_progress: -1,
        }
    }
}

/// A zombie's `setCanBreakDoors` permission and its `BreakDoorGoal` progress.
#[derive(Debug, Clone, Copy)]
pub(super) struct ZombieDoors {
    pub(super) can_break: bool,
    pub(super) state: BreakDoorState,
    /// Drowned replace the ground navigation the goal reads.
    pub(super) navigation_owned: bool,
}

impl BreakDoorState {
    fn advance(&mut self) -> Option<i32> {
        self.break_time += 1;
        let progress = self.break_time * 10 / 240;
        (progress != self.last_progress).then(|| {
            self.last_progress = progress;
            progress
        })
    }

    pub(super) fn stop_effect(&self) -> DoorEffect {
        DoorEffect::Progress(self.pos, -1)
    }
}

#[derive(Debug, Clone)]
pub(super) enum DoorEffect {
    Hit(BlockPos),
    Progress(BlockPos, i32),
    Break(BlockPos),
}

#[derive(Debug, Clone)]
pub(super) struct DoorContext {
    pub(super) state: BreakDoorState,
    pub(super) effects: Vec<DoorEffect>,
    pub(super) unavailable: bool,
    horizontal_collision: bool,
    navigation_owned: bool,
    path_candidates: Option<Vec<BlockPos>>,
    states: Vec<(BlockPos, Option<i32>)>,
    mob_griefing: bool,
}
impl DoorContext {
    pub(super) fn new(state: BreakDoorState, horizontal_collision: bool) -> Self {
        Self {
            state,
            effects: Vec::new(),
            unavailable: false,
            horizontal_collision,
            navigation_owned: true,
            path_candidates: None,
            states: Vec::new(),
            mob_griefing: false,
        }
    }

    pub(super) fn refresh(
        &mut self,
        level: &LevelCtx<'_, '_>,
        navigation: &GroundNavigation,
        position: Vec3d,
        navigation_owned: bool,
    ) {
        self.navigation_owned = navigation_owned;
        self.mob_griefing = level.mob_griefing();
        // DoorInteractGoal.java: scan the prefix, then the block above the mob.
        self.path_candidates = navigation
            .path()
            .filter(|path| !path.is_done())
            .map(|path| {
                path.nodes()
                    .iter()
                    .take(path.next_node_index().saturating_add(2))
                    .map(|node| BlockPos {
                        y: node.pos.y + 1,
                        ..node.pos
                    })
                    .collect()
            });
        self.states.clear();
        let mob_above = above_mob(position);
        for pos in self
            .path_candidates
            .iter()
            .flatten()
            .copied()
            .chain([self.state.pos, mob_above])
        {
            if !self.states.iter().any(|(stored, _)| *stored == pos) {
                self.states.push((
                    pos,
                    level
                        .has_ticking_column_at(pos)
                        .then(|| level.block_state(pos)),
                ));
            }
        }
    }

    fn block(&mut self, pos: BlockPos) -> Option<i32> {
        let state = self
            .states
            .iter()
            .find(|(stored, _)| *stored == pos)
            .and_then(|(_, state)| *state);
        self.unavailable |= state.is_none();
        state
    }

    fn is_open(&mut self) -> bool {
        if !self.state.has_door {
            return false;
        }
        let Some(state) = self.block(self.state.pos) else {
            return false;
        };
        if !door::is_door(state) {
            self.state.has_door = false;
            return false;
        }
        vanilla_block_state_property(state, "open") == Some("true")
    }

    fn can_interact(&mut self, position: Vec3d) -> bool {
        if !self.horizontal_collision {
            return false;
        }
        if !self.navigation_owned {
            self.unavailable = true;
            return false;
        }
        let Some(candidates) = self.path_candidates.clone() else {
            return false;
        };
        let selected = find_door(position, candidates, |pos| self.block(pos));
        self.state.has_door = selected.is_some();
        if let Some(pos) = selected {
            self.state.pos = pos;
        }
        self.state.has_door
    }
}
fn above_mob(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32 + 1,
        z: position.z.floor() as i32,
    }
}

#[derive(Clone)]
pub(crate) struct BreakDoorGoal;
impl Goal for BreakDoorGoal {
    // BreakDoorGoal.java.
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(door) = context.nav.door.as_mut() else {
            return false;
        };
        if !door.can_interact(context.body.position) || !door.mob_griefing {
            return false;
        }
        door.unavailable |= context.difficulty.is_none();
        context.difficulty == Some(Difficulty::Hard) && !door.is_open()
    }
    // BreakDoorGoal.java: mobGriefing is only an admission condition.
    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(door) = context.nav.door.as_mut() else {
            return false;
        };
        if door.state.break_time > 240 || door.is_open() {
            return false;
        }
        let pos = door.state.pos;
        let distance = center_distance_squared(context.body.position, pos);
        if distance >= 4.0 {
            return false;
        }
        door.unavailable |= context.difficulty.is_none();
        context.difficulty == Some(Difficulty::Hard)
    }
    // DoorInteractGoal.java; BreakDoorGoal.java.
    fn start(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let door = context.nav.door.as_mut().unwrap();
        door.state.break_time = 0;
    }
    fn stop(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        if let Some(door) = context.nav.door.as_mut() {
            door.effects.push(door.state.stop_effect());
        }
    }
    // BreakDoorGoal.java; DoorInteractGoal.java.
    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let hit = context.next_int(20) == 0;
        let door = context.nav.door.as_mut().unwrap();
        let pos = door.state.pos;
        if hit {
            door.effects.push(DoorEffect::Hit(pos));
        }
        if let Some(progress) = door.state.advance() {
            door.effects.push(DoorEffect::Progress(pos, progress));
        }
        if door.state.break_time == 240 {
            door.unavailable |= context.difficulty.is_none();
        }
        if door.state.break_time == 240 && context.difficulty == Some(Difficulty::Hard) {
            door.effects.push(DoorEffect::Break(pos));
        }
    }
    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn find_door(
    position: Vec3d,
    candidates: impl IntoIterator<Item = BlockPos>,
    mut block: impl FnMut(BlockPos) -> Option<i32>,
) -> Option<BlockPos> {
    for pos in candidates {
        if (position.x - f64::from(pos.x)).powi(2) + (position.z - f64::from(pos.z)).powi(2) > 2.25
        {
            continue;
        }
        let state = block(pos)?;
        if is_wooden_door(state) {
            return Some(pos);
        }
    }
    let pos = above_mob(position);
    is_wooden_door(block(pos)?).then_some(pos)
}

pub(super) fn is_wooden_door(state: i32) -> bool {
    door::is_door(state) && door::can_open_by_hand(state) == Some(true)
}

fn center_distance_squared(position: Vec3d, pos: BlockPos) -> f64 {
    (position.x - (f64::from(pos.x) + 0.5)).powi(2)
        + (position.y - (f64::from(pos.y) + 0.5)).powi(2)
        + (position.z - (f64::from(pos.z) + 0.5)).powi(2)
}

pub(super) fn apply_effect(
    effect: DoorEffect,
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    match effect {
        DoorEffect::Hit(pos) => {
            level.level_event(1019, pos, 0);
            let living = body.living_mut().expect("door interaction requires a mob");
            if !living.is_swinging() {
                let hand = living.used_item_hand();
                if living.swing(hand) {
                    level.entity_animation(body.id(), hand);
                }
            }
        }
        DoorEffect::Progress(pos, stage) => level.destroy_block_progress(body.id(), pos, stage),
        DoorEffect::Break(pos) => {
            let state = level.block_state(pos);
            let fluid = bbb_server_types::fluid::block_state_fluid_state(state);
            level.set_entity_block(
                crate::entity::level::EntityBlockContext::new(Some(store), Some(body)),
                pos,
                fluid.create_legacy_block(),
                3,
            );
            level.level_event(1021, pos, 0);
            level.level_event(2001, pos, state);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct VindicatorDoors {
    breaking: Option<BreakDoorState>,
    opened: Option<BlockPos>,
}

fn live_door(
    body: &Entity,
    navigation: &super::MobNavigation,
    level: &LevelCtx<'_, '_>,
) -> Option<BlockPos> {
    if !body.horizontal_collision() {
        return None;
    }
    let path = navigation.ground()?.path().filter(|path| !path.is_done())?;
    find_door(
        body.position(),
        path.nodes()
            .iter()
            .take(path.next_node_index().saturating_add(2))
            .map(|node| BlockPos {
                y: node.pos.y + 1,
                ..node.pos
            }),
        |pos| {
            level
                .has_ticking_column_at(pos)
                .then(|| level.block_state(pos))
        },
    )
}

impl MobAiRuntime {
    pub(super) fn vindicator_door_step(
        &mut self,
        body: &mut Entity,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) {
        let Some(doors) = self.kind.vindicator_doors_mut() else {
            return;
        };
        let ground = self.navigation.ground_mut();
        if let Some(ground) = ground {
            ground.set_can_open_doors(level.raid_id_at(body.block_position()).is_some());
        }
        let active = match body.data() {
            EntityData::GeneralLiving(data) => data
                .raider_retained()
                .and_then(|raider| match raider.raid() {
                    RaiderRaid::Current(id) => level.raider_raid(id),
                    _ => None,
                })
                .is_some_and(|raid| raid.active),
            _ => false,
        };
        let allowed_difficulty = matches!(
            store.difficulty(),
            Some(Difficulty::Normal | Difficulty::Hard)
        );
        let can_move = self.goal_selector.can_vindicator_break();
        if doors.breaking.is_some_and(|state| {
            !active
                || !allowed_difficulty
                || !can_move
                || state.break_time >= 240
                || center_distance_squared(body.position(), state.pos) >= 4.0
                || !level.has_ticking_column_at(state.pos)
                || {
                    let block = level.block_state(state.pos);
                    !door::is_door(block)
                        || vanilla_block_state_property(block, "open") == Some("true")
                }
        }) {
            let state = doors.breaking.take().expect("breaking door");
            apply_effect(state.stop_effect(), body, store, level);
        }
        let next_open = if active {
            live_door(body, &self.navigation, level)
        } else {
            None
        };
        if doors.opened != next_open {
            if let Some(pos) = doors.opened.take() {
                if level.has_ticking_column_at(pos) {
                    let state = level.block_state(pos);
                    door::set_open(level, pos, state, Some(body.id()), false);
                }
            }
        }
        if doors.breaking.is_none()
            && active
            && can_move
            && allowed_difficulty
            && level.mob_griefing()
            && self.random.next_int(super::reduced_tick_delay(10)) == 0
        {
            if let Some(pos) = next_open {
                if level.has_ticking_column_at(pos)
                    && vanilla_block_state_property(level.block_state(pos), "open") != Some("true")
                {
                    doors.breaking = Some(BreakDoorState {
                        pos,
                        has_door: true,
                        ..Default::default()
                    });
                    body.living_mut()
                        .expect("vindicator living state")
                        .no_action_time = 0;
                    self.navigation.stop();
                }
            }
        }
        if doors.breaking.is_none() {
            if let Some(pos) = next_open {
                if doors.opened != Some(pos) && level.has_ticking_column_at(pos) {
                    let state = level.block_state(pos);
                    if vanilla_block_state_property(state, "open") != Some("true") {
                        door::set_open(level, pos, state, Some(body.id()), true);
                    }
                    doors.opened = Some(pos);
                }
            }
        }
        self.goal_selector
            .set_vindicator_breaking(doors.breaking.is_some());
        if let Some(state) = doors.breaking.as_mut() {
            if self.random.next_int(20) == 0 {
                apply_effect(DoorEffect::Hit(state.pos), body, store, level);
            }
            if let Some(progress) = state.advance() {
                apply_effect(
                    DoorEffect::Progress(state.pos, progress),
                    body,
                    store,
                    level,
                );
            }
            if state.break_time >= 240 {
                let pos = state.pos;
                doors.breaking = None;
                self.goal_selector.set_vindicator_breaking(false);
                apply_effect(DoorEffect::Break(pos), body, store, level);
            }
        }
    }
}
