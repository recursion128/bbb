use bbb_protocol::{
    packets::ItemStackSummary,
    stat_types::{VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_ITEM_CRAFTED_ID},
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, WorldStore};

const COMPLETES_FIND_TREE_TUTORIAL_TAG: &str = "minecraft:completes_find_tree_tutorial";
const LOGS_TAG: &str = "minecraft:logs";
const PLANKS_TAG: &str = "minecraft:planks";
const TUTORIAL_PROGRESS_SCALE: u32 = 1_000_000;

const MOVEMENT_REQUIRED_TICKS: u32 = 40;
const MOVEMENT_HINT_DELAY_TICKS: u32 = 100;
const LOOK_HINT_DELAY_TICKS: u32 = 20;
const FIND_TREE_HINT_DELAY_TICKS: u32 = 6_000;
const PUNCH_TREE_HINT_DELAY_TICKS: u32 = 600;
const OPEN_INVENTORY_HINT_DELAY_TICKS: u32 = 600;
const CRAFT_PLANKS_HINT_DELAY_TICKS: u32 = 1_200;

/// Vanilla's persisted `TutorialSteps` option. The native client accepts the
/// initial value at startup instead of exposing an in-game options screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TutorialStep {
    #[default]
    Movement,
    FindTree,
    PunchTree,
    OpenInventory,
    CraftPlanks,
    None,
}

/// Presentation identity for one Vanilla `TutorialToast`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TutorialToastKind {
    Movement,
    Look,
    FindTree,
    PunchTree,
    OpenInventory,
    CraftPlanks,
    SocialInteractions,
}

impl TutorialToastKind {
    pub fn progressable(self) -> bool {
        matches!(
            self,
            Self::Movement | Self::Look | Self::PunchTree | Self::SocialInteractions
        )
    }
}

/// Canonical desired state for the tutorial toast currently owned by the
/// active step. Queue placement and smoothing remain native runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTutorialToastState {
    pub id: u64,
    pub kind: TutorialToastKind,
    pub progress_millionths: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum TutorialStepInstance {
    Movement {
        time_waiting: u32,
        time_moved: u32,
        time_looked: u32,
        pending_moved_ticks: u32,
        turned: bool,
        move_completed_at: Option<u32>,
        look_completed_at: Option<u32>,
    },
    FindTree {
        time_waiting: u32,
    },
    PunchTree {
        time_waiting: u32,
        reset_count: u32,
    },
    OpenInventory {
        time_waiting: u32,
    },
    CraftPlanks {
        time_waiting: u32,
    },
    None,
}

impl TutorialStep {
    fn create_instance(self) -> TutorialStepInstance {
        match self {
            Self::Movement => TutorialStepInstance::Movement {
                time_waiting: 0,
                time_moved: 0,
                time_looked: 0,
                pending_moved_ticks: 0,
                turned: false,
                move_completed_at: None,
                look_completed_at: None,
            },
            Self::FindTree => TutorialStepInstance::FindTree { time_waiting: 0 },
            Self::PunchTree => TutorialStepInstance::PunchTree {
                time_waiting: 0,
                reset_count: 0,
            },
            Self::OpenInventory => TutorialStepInstance::OpenInventory { time_waiting: 0 },
            Self::CraftPlanks => TutorialStepInstance::CraftPlanks { time_waiting: 0 },
            Self::None => TutorialStepInstance::None,
        }
    }
}

/// Deterministic canonical owner for Vanilla's `Tutorial` and step instances.
///
/// Tick counters and desired toast progress are serialized; wall-clock slide
/// animation and progress smoothing stay in the native toast runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTutorialState {
    #[serde(default)]
    step: TutorialStep,
    #[serde(default)]
    instance: Option<TutorialStepInstance>,
    #[serde(default)]
    active_toast: Option<ClientTutorialToastState>,
    #[serde(default)]
    next_toast_id: u64,
}

impl Default for ClientTutorialState {
    fn default() -> Self {
        Self {
            step: TutorialStep::Movement,
            instance: None,
            active_toast: None,
            next_toast_id: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TutorialTickFacts {
    is_survival: bool,
    has_find_tree_item: bool,
    has_log_item: bool,
    has_planks_item: bool,
    has_mined_find_tree_block: bool,
    has_crafted_planks: bool,
}

impl ClientTutorialState {
    pub fn step(&self) -> TutorialStep {
        self.step
    }

    pub fn active_toast(&self) -> Option<ClientTutorialToastState> {
        self.active_toast
    }

    fn configure_step(&mut self, step: TutorialStep) {
        self.step = step;
        self.active_toast = None;
        if self.instance.is_some() {
            self.instance = Some(step.create_instance());
        }
    }

    fn stop(&mut self) {
        self.active_toast = None;
        self.instance = None;
    }

    fn set_step(&mut self, step: TutorialStep) {
        self.active_toast = None;
        self.step = step;
        self.instance = Some(step.create_instance());
    }

    fn show_toast(&mut self, kind: TutorialToastKind) -> (u64, TutorialToastKind) {
        let id = self.next_toast_id;
        self.next_toast_id = self
            .next_toast_id
            .checked_add(1)
            .expect("tutorial toast id overflow");
        self.active_toast = Some(ClientTutorialToastState {
            id,
            kind,
            progress_millionths: kind.progressable().then_some(0),
        });
        (id, kind)
    }

    fn active_kind(&self) -> Option<TutorialToastKind> {
        self.active_toast.map(|toast| toast.kind)
    }

    fn clear_active_kind(&mut self, kind: TutorialToastKind) {
        if self.active_kind() == Some(kind) {
            self.active_toast = None;
        }
    }

    fn update_progress(&mut self, kind: TutorialToastKind, progress_millionths: u32) {
        if let Some(toast) = self
            .active_toast
            .as_mut()
            .filter(|toast| toast.kind == kind)
        {
            toast.progress_millionths = Some(progress_millionths.min(TUTORIAL_PROGRESS_SCALE));
        }
    }

    fn tick(&mut self, facts: TutorialTickFacts) -> Option<(u64, TutorialToastKind)> {
        let active_kind = self.active_kind();
        let Some(instance) = self.instance.as_mut() else {
            self.instance = Some(self.step.create_instance());
            return None;
        };

        enum Action {
            None,
            Show(TutorialToastKind),
            Transition(TutorialStep),
        }

        let mut clear_kind = None;
        let mut progress = None;
        let action = match instance {
            TutorialStepInstance::Movement {
                time_waiting,
                time_moved,
                time_looked,
                pending_moved_ticks,
                turned,
                move_completed_at,
                look_completed_at,
            } => {
                *time_waiting = time_waiting.saturating_add(1);
                if *pending_moved_ticks > 0 {
                    *time_moved = time_moved.saturating_add(1);
                    *pending_moved_ticks -= 1;
                }
                if *turned {
                    *time_looked = time_looked.saturating_add(1);
                    *turned = false;
                }

                if move_completed_at.is_none() && *time_moved > MOVEMENT_REQUIRED_TICKS {
                    clear_kind = Some(TutorialToastKind::Movement);
                    *move_completed_at = Some(*time_waiting);
                }
                if look_completed_at.is_none() && *time_looked > MOVEMENT_REQUIRED_TICKS {
                    clear_kind = Some(TutorialToastKind::Look);
                    *look_completed_at = Some(*time_waiting);
                }

                if move_completed_at.is_some() && look_completed_at.is_some() {
                    Action::Transition(if facts.is_survival {
                        TutorialStep::FindTree
                    } else {
                        TutorialStep::None
                    })
                } else {
                    progress = match active_kind {
                        Some(TutorialToastKind::Movement) => {
                            Some((TutorialToastKind::Movement, movement_progress(*time_moved)))
                        }
                        Some(TutorialToastKind::Look) => {
                            Some((TutorialToastKind::Look, movement_progress(*time_looked)))
                        }
                        _ => None,
                    };
                    if *time_waiting >= MOVEMENT_HINT_DELAY_TICKS {
                        if move_completed_at.is_none()
                            && active_kind != Some(TutorialToastKind::Movement)
                        {
                            Action::Show(TutorialToastKind::Movement)
                        } else if let Some(move_completed_at) = *move_completed_at {
                            if time_waiting.saturating_sub(move_completed_at)
                                >= LOOK_HINT_DELAY_TICKS
                                && look_completed_at.is_none()
                                && active_kind != Some(TutorialToastKind::Look)
                            {
                                Action::Show(TutorialToastKind::Look)
                            } else {
                                Action::None
                            }
                        } else {
                            Action::None
                        }
                    } else {
                        Action::None
                    }
                }
            }
            TutorialStepInstance::FindTree { time_waiting } => {
                *time_waiting = time_waiting.saturating_add(1);
                if !facts.is_survival {
                    Action::Transition(TutorialStep::None)
                } else if *time_waiting == 1
                    && (facts.has_find_tree_item || facts.has_mined_find_tree_block)
                {
                    Action::Transition(TutorialStep::CraftPlanks)
                } else if *time_waiting >= FIND_TREE_HINT_DELAY_TICKS
                    && active_kind != Some(TutorialToastKind::FindTree)
                {
                    Action::Show(TutorialToastKind::FindTree)
                } else {
                    Action::None
                }
            }
            TutorialStepInstance::PunchTree {
                time_waiting,
                reset_count,
            } => {
                *time_waiting = time_waiting.saturating_add(1);
                if !facts.is_survival {
                    Action::Transition(TutorialStep::None)
                } else if *time_waiting == 1
                    && (facts.has_log_item || facts.has_mined_find_tree_block)
                {
                    Action::Transition(TutorialStep::CraftPlanks)
                } else if (*time_waiting >= PUNCH_TREE_HINT_DELAY_TICKS || *reset_count > 3)
                    && active_kind != Some(TutorialToastKind::PunchTree)
                {
                    Action::Show(TutorialToastKind::PunchTree)
                } else {
                    Action::None
                }
            }
            TutorialStepInstance::OpenInventory { time_waiting } => {
                *time_waiting = time_waiting.saturating_add(1);
                if !facts.is_survival {
                    Action::Transition(TutorialStep::None)
                } else if *time_waiting >= OPEN_INVENTORY_HINT_DELAY_TICKS
                    && active_kind != Some(TutorialToastKind::OpenInventory)
                {
                    Action::Show(TutorialToastKind::OpenInventory)
                } else {
                    Action::None
                }
            }
            TutorialStepInstance::CraftPlanks { time_waiting } => {
                *time_waiting = time_waiting.saturating_add(1);
                if !facts.is_survival {
                    Action::Transition(TutorialStep::None)
                } else if *time_waiting == 1 && (facts.has_planks_item || facts.has_crafted_planks)
                {
                    Action::Transition(TutorialStep::None)
                } else if *time_waiting >= CRAFT_PLANKS_HINT_DELAY_TICKS
                    && active_kind != Some(TutorialToastKind::CraftPlanks)
                {
                    Action::Show(TutorialToastKind::CraftPlanks)
                } else {
                    Action::None
                }
            }
            TutorialStepInstance::None => Action::None,
        };

        if let Some(kind) = clear_kind {
            self.clear_active_kind(kind);
        }
        if let Some((kind, progress)) = progress {
            self.update_progress(kind, progress);
        }
        match action {
            Action::None => None,
            Action::Show(kind) => Some(self.show_toast(kind)),
            Action::Transition(step) => {
                self.set_step(step);
                None
            }
        }
    }

    fn on_input(&mut self, moving: bool) {
        if moving {
            if let Some(TutorialStepInstance::Movement {
                pending_moved_ticks,
                ..
            }) = self.instance.as_mut()
            {
                *pending_moved_ticks = pending_moved_ticks.saturating_add(1);
            }
        }
    }

    fn on_mouse(&mut self, delta_x: f64, delta_y: f64) {
        if let Some(TutorialStepInstance::Movement { turned, .. }) = self.instance.as_mut() {
            *turned |= delta_x.abs() > 0.01 || delta_y.abs() > 0.01;
        }
    }

    fn on_look_at_find_tree_block(&mut self, matches: bool) {
        if matches!(self.instance, Some(TutorialStepInstance::FindTree { .. })) && matches {
            self.set_step(TutorialStep::PunchTree);
        }
    }

    fn on_destroy_log_block(&mut self, is_log: bool, progress_millionths: i32) {
        if !matches!(self.instance, Some(TutorialStepInstance::PunchTree { .. })) {
            return;
        }
        if is_log && progress_millionths > 0 {
            let progress = u32::try_from(progress_millionths)
                .unwrap_or(TUTORIAL_PROGRESS_SCALE)
                .min(TUTORIAL_PROGRESS_SCALE);
            self.update_progress(TutorialToastKind::PunchTree, progress);
            if progress >= TUTORIAL_PROGRESS_SCALE {
                self.set_step(TutorialStep::OpenInventory);
            }
        } else if self.active_kind() == Some(TutorialToastKind::PunchTree) {
            self.update_progress(TutorialToastKind::PunchTree, 0);
        } else if is_log {
            if let Some(TutorialStepInstance::PunchTree { reset_count, .. }) =
                self.instance.as_mut()
            {
                *reset_count = reset_count.saturating_add(1);
            }
        }
    }

    fn on_open_inventory(&mut self) {
        if matches!(
            self.instance,
            Some(TutorialStepInstance::OpenInventory { .. })
        ) {
            self.set_step(TutorialStep::CraftPlanks);
        }
    }

    fn on_get_item(&mut self, completes_find_tree: bool, is_log: bool, is_planks: bool) {
        match self.instance {
            Some(TutorialStepInstance::FindTree { .. }) if completes_find_tree => {
                self.set_step(TutorialStep::CraftPlanks);
            }
            Some(TutorialStepInstance::PunchTree { .. }) if is_log => {
                self.set_step(TutorialStep::CraftPlanks);
            }
            Some(TutorialStepInstance::CraftPlanks { .. }) if is_planks => {
                self.set_step(TutorialStep::None);
            }
            _ => {}
        }
    }
}

impl WorldStore {
    pub fn client_tutorial(&self) -> &ClientTutorialState {
        &self.client_tutorial
    }

    pub fn tutorial_step(&self) -> TutorialStep {
        self.client_tutorial.step()
    }

    pub fn active_tutorial_toast(&self) -> Option<ClientTutorialToastState> {
        self.client_tutorial.active_toast()
    }

    pub fn set_tutorial_step(&mut self, step: TutorialStep) {
        self.client_tutorial.configure_step(step);
    }

    pub fn advance_tutorial_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            if self.level.is_none() {
                self.client_tutorial.stop();
                continue;
            }
            let facts = self.tutorial_tick_facts();
            if let Some((toast_id, kind)) = self.client_tutorial.tick(facts) {
                self.push_tutorial_toast_event(toast_id, kind);
            }
        }
    }

    pub fn tutorial_on_input(
        &mut self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        jump: bool,
    ) {
        self.client_tutorial
            .on_input(forward || backward || left || right || jump);
    }

    pub fn tutorial_on_mouse(&mut self, delta_x: f64, delta_y: f64) {
        self.client_tutorial.on_mouse(delta_x, delta_y);
    }

    pub fn tutorial_on_look_at_block(&mut self, pos: BlockPos) {
        let matches = self.probe_block(pos).is_some_and(|block| {
            block.block_name.as_deref().is_some_and(|block_name| {
                self.registry_holder_id_by_resource_id("minecraft:block", block_name)
                    .is_some_and(|holder_id| {
                        self.registry_holder_is_in_tag(
                            "minecraft:block",
                            COMPLETES_FIND_TREE_TUTORIAL_TAG,
                            holder_id,
                        )
                    })
            })
        });
        self.client_tutorial.on_look_at_find_tree_block(matches);
    }

    pub fn tutorial_on_destroy_block(&mut self, pos: BlockPos, progress_millionths: i32) {
        let is_log = self.probe_block(pos).is_some_and(|block| {
            block.block_name.as_deref().is_some_and(|block_name| {
                self.registry_holder_id_by_resource_id("minecraft:block", block_name)
                    .is_some_and(|holder_id| {
                        self.registry_holder_is_in_tag("minecraft:block", LOGS_TAG, holder_id)
                    })
            })
        });
        self.client_tutorial
            .on_destroy_log_block(is_log, progress_millionths);
    }

    pub fn tutorial_on_open_inventory(&mut self) {
        self.client_tutorial.on_open_inventory();
    }

    pub(crate) fn tutorial_on_get_item(&mut self, item: &ItemStackSummary) {
        let Some(item_id) = item.item_id.filter(|_| item.count > 0) else {
            return;
        };
        let completes_find_tree = self.registry_holder_is_in_tag(
            "minecraft:item",
            COMPLETES_FIND_TREE_TUTORIAL_TAG,
            item_id,
        );
        let is_log = self.registry_holder_is_in_tag("minecraft:item", LOGS_TAG, item_id);
        let is_planks = self.registry_holder_is_in_tag("minecraft:item", PLANKS_TAG, item_id);
        self.client_tutorial
            .on_get_item(completes_find_tree, is_log, is_planks);
    }

    pub(crate) fn stop_tutorial_for_missing_level(&mut self) {
        self.client_tutorial.stop();
    }

    fn tutorial_tick_facts(&self) -> TutorialTickFacts {
        TutorialTickFacts {
            is_survival: self.local_player_is_survival(),
            has_find_tree_item: self.local_player_has_item_in_tag(COMPLETES_FIND_TREE_TUTORIAL_TAG)
                == Some(true),
            has_log_item: self.local_player_has_item_in_tag(LOGS_TAG) == Some(true),
            has_planks_item: self.local_player_has_item_in_tag(PLANKS_TAG) == Some(true),
            has_mined_find_tree_block: self.has_positive_stat_in_tag(
                VANILLA_STAT_TYPE_BLOCK_MINED_ID,
                "minecraft:block",
                COMPLETES_FIND_TREE_TUTORIAL_TAG,
            ),
            has_crafted_planks: self.has_positive_stat_in_tag(
                VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                "minecraft:item",
                PLANKS_TAG,
            ),
        }
    }

    fn has_positive_stat_in_tag(&self, stat_type_id: i32, registry: &str, tag: &str) -> bool {
        let Some(holder_ids) = self.registry_holder_ids_for_tag(registry, tag) else {
            return false;
        };
        holder_ids.iter().any(|holder_id| {
            self.stat_value(stat_type_id, *holder_id)
                .is_some_and(|value| value > 0)
        })
    }
}

fn movement_progress(ticks: u32) -> u32 {
    ticks
        .saturating_mul(TUTORIAL_PROGRESS_SCALE)
        .checked_div(MOVEMENT_REQUIRED_TICKS)
        .unwrap_or_default()
        .min(TUTORIAL_PROGRESS_SCALE)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommonPlayerSpawnInfo, ContainerSetSlot, PlayLogin, Respawn, SetCursorItem,
        SetPlayerInventory,
    };

    use super::*;

    #[test]
    fn movement_thresholds_and_hints_follow_vanilla_tick_boundaries() {
        let mut world = tutorial_world();
        world.advance_tutorial_ticks(1);
        for _ in 0..MOVEMENT_REQUIRED_TICKS {
            world.tutorial_on_input(true, false, false, false, false);
            world.tutorial_on_mouse(1.0, 0.0);
            world.advance_tutorial_ticks(1);
        }
        assert_eq!(world.tutorial_step(), TutorialStep::Movement);
        assert_eq!(world.active_tutorial_toast(), None);

        world.tutorial_on_input(true, false, false, false, false);
        world.tutorial_on_mouse(1.0, 0.0);
        world.advance_tutorial_ticks(1);
        assert_eq!(world.tutorial_step(), TutorialStep::FindTree);

        let mut hints = tutorial_world();
        hints.advance_tutorial_ticks(1);
        hints.advance_tutorial_ticks(MOVEMENT_HINT_DELAY_TICKS);
        assert_eq!(
            hints.active_tutorial_toast().map(|toast| toast.kind),
            Some(TutorialToastKind::Movement)
        );
        assert_eq!(
            hints.active_tutorial_toast().unwrap().progress_millionths,
            Some(0)
        );

        let mut early_move = tutorial_world();
        early_move.advance_tutorial_ticks(1);
        for _ in 0..=MOVEMENT_REQUIRED_TICKS {
            early_move.tutorial_on_input(true, false, false, false, false);
            early_move.advance_tutorial_ticks(1);
        }
        early_move.advance_tutorial_ticks(MOVEMENT_HINT_DELAY_TICKS - MOVEMENT_REQUIRED_TICKS - 2);
        assert_eq!(early_move.active_tutorial_toast(), None);
        early_move.advance_tutorial_ticks(1);
        assert_eq!(
            early_move.active_tutorial_toast().map(|toast| toast.kind),
            Some(TutorialToastKind::Look),
            "Vanilla's outer 100-tick hint delay still gates an early movement completion"
        );
    }

    #[test]
    fn non_survival_movement_completion_ends_tutorial() {
        let mut world = tutorial_world();
        world.gameplay.game_type = bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID;
        world.advance_tutorial_ticks(1);
        for _ in 0..=MOVEMENT_REQUIRED_TICKS {
            world.tutorial_on_input(true, false, false, false, false);
            world.tutorial_on_mouse(1.0, 0.0);
            world.advance_tutorial_ticks(1);
        }
        assert_eq!(world.tutorial_step(), TutorialStep::None);
    }

    #[test]
    fn all_three_inventory_packets_notify_the_active_tutorial_before_apply() {
        for packet_kind in 0..3 {
            let mut world = tutorial_world();
            set_item_tag(&mut world, COMPLETES_FIND_TREE_TUTORIAL_TAG, vec![42]);
            world.set_tutorial_step(TutorialStep::FindTree);
            world.advance_tutorial_ticks(1);
            let stack = ItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: Default::default(),
            };
            match packet_kind {
                0 => {
                    world.apply_container_set_slot(ContainerSetSlot {
                        container_id: 99,
                        state_id: 0,
                        slot: -300,
                        item: stack,
                    });
                }
                1 => world.apply_set_cursor_item(SetCursorItem { item: stack }),
                2 => {
                    world.apply_set_player_inventory(SetPlayerInventory {
                        slot: i32::MAX,
                        item: stack,
                    });
                }
                _ => unreachable!(),
            }
            assert_eq!(
                world.tutorial_step(),
                TutorialStep::CraftPlanks,
                "packet kind {packet_kind} must notify even when canonical apply ignores the slot"
            );
        }
    }

    #[test]
    fn find_punch_inventory_and_craft_steps_follow_vanilla_callbacks_and_delays() {
        let facts = TutorialTickFacts {
            is_survival: true,
            has_find_tree_item: false,
            has_log_item: false,
            has_planks_item: false,
            has_mined_find_tree_block: false,
            has_crafted_planks: false,
        };
        let mut tutorial = ClientTutorialState::default();
        tutorial.configure_step(TutorialStep::FindTree);
        assert_eq!(tutorial.tick(facts), None);
        for _ in 0..FIND_TREE_HINT_DELAY_TICKS {
            tutorial.tick(facts);
        }
        assert_eq!(tutorial.active_kind(), Some(TutorialToastKind::FindTree));
        tutorial.on_look_at_find_tree_block(true);
        assert_eq!(tutorial.step(), TutorialStep::PunchTree);
        assert_eq!(tutorial.active_kind(), None);

        tutorial.tick(facts);
        for _ in 0..4 {
            tutorial.on_destroy_log_block(true, 0);
        }
        tutorial.tick(facts);
        assert_eq!(tutorial.active_kind(), Some(TutorialToastKind::PunchTree));
        tutorial.on_destroy_log_block(true, 500_000);
        assert_eq!(
            tutorial.active_toast().unwrap().progress_millionths,
            Some(500_000)
        );
        tutorial.on_destroy_log_block(true, 1_000_000);
        assert_eq!(tutorial.step(), TutorialStep::OpenInventory);
        assert_eq!(tutorial.active_kind(), None);

        for _ in 0..OPEN_INVENTORY_HINT_DELAY_TICKS {
            tutorial.tick(facts);
        }
        assert_eq!(
            tutorial.active_kind(),
            Some(TutorialToastKind::OpenInventory)
        );
        tutorial.on_open_inventory();
        assert_eq!(tutorial.step(), TutorialStep::CraftPlanks);

        for _ in 0..CRAFT_PLANKS_HINT_DELAY_TICKS {
            tutorial.tick(facts);
        }
        assert_eq!(tutorial.active_kind(), Some(TutorialToastKind::CraftPlanks));
        tutorial.on_get_item(false, false, true);
        assert_eq!(tutorial.step(), TutorialStep::None);
        assert_eq!(tutorial.active_kind(), None);
    }

    #[test]
    fn first_step_tick_skips_completed_find_punch_and_craft_work() {
        for (step, facts, expected) in [
            (
                TutorialStep::FindTree,
                TutorialTickFacts {
                    has_find_tree_item: true,
                    ..survival_facts()
                },
                TutorialStep::CraftPlanks,
            ),
            (
                TutorialStep::PunchTree,
                TutorialTickFacts {
                    has_log_item: true,
                    ..survival_facts()
                },
                TutorialStep::CraftPlanks,
            ),
            (
                TutorialStep::CraftPlanks,
                TutorialTickFacts {
                    has_crafted_planks: true,
                    ..survival_facts()
                },
                TutorialStep::None,
            ),
        ] {
            let mut tutorial = ClientTutorialState::default();
            tutorial.configure_step(step);
            tutorial.tick(facts);
            tutorial.tick(facts);
            assert_eq!(tutorial.step(), expected, "{step:?}");
        }
    }

    #[test]
    fn tutorial_toast_enqueue_shares_the_global_sequence() {
        let mut world = tutorial_world();
        world.advance_tutorial_ticks(1);
        world.push_recipe_toast_event(7, ItemStackSummary::empty(), ItemStackSummary::empty());
        world.advance_tutorial_ticks(MOVEMENT_HINT_DELAY_TICKS);
        assert_eq!(
            world
                .client_toast_events()
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(matches!(
            world.client_toast_events()[0].system(),
            Some(crate::ClientSystemToastEvent::InsecureServerWarning)
        ));
        assert_eq!(
            world.client_toast_events()[2].tutorial(),
            world.active_tutorial_toast().map(|toast| {
                crate::ClientTutorialToastEvent::Show {
                    token: crate::ClientTutorialToastToken::Step(toast.id),
                    kind: toast.kind,
                    time_to_display_ms: 0,
                }
            })
        );
    }

    #[test]
    fn first_multiplayer_tick_owns_one_timed_social_toast_and_persists_across_disconnect() {
        let mut no_level = WorldStore::new();
        assert!(!no_level.show_social_interactions_tutorial_if_first_join());
        assert!(no_level.client_toast_events().is_empty());

        let mut world = tutorial_world();
        assert!(world.show_social_interactions_tutorial_if_first_join());
        assert_eq!(
            world
                .client_toast_events()
                .back()
                .and_then(|event| event.tutorial()),
            Some(crate::ClientTutorialToastEvent::Show {
                token: crate::ClientTutorialToastToken::SocialInteractions,
                kind: TutorialToastKind::SocialInteractions,
                time_to_display_ms: 8_000,
            })
        );
        assert!(!world.show_social_interactions_tutorial_if_first_join());
        assert_eq!(world.client_toast_events().len(), 2);

        world.clear_client_level();
        assert!(world.client_toasts().joined_first_server);
        assert!(world.client_toasts().social_interactions_tutorial_owned);
        assert!(world.hide_social_interactions_tutorial());
        assert_eq!(
            world
                .client_toast_events()
                .back()
                .and_then(|event| event.tutorial()),
            Some(crate::ClientTutorialToastEvent::Hide {
                token: crate::ClientTutorialToastToken::SocialInteractions,
            })
        );
        assert!(!world.hide_social_interactions_tutorial());
    }

    #[test]
    fn respawn_retains_active_tutorial_toast_and_disconnect_stops_the_instance() {
        let mut world = tutorial_world();
        world.advance_tutorial_ticks(1);
        world.advance_tutorial_ticks(MOVEMENT_HINT_DELAY_TICKS);
        let active = world.active_tutorial_toast().unwrap();
        let retained_events = world.client_toast_events().clone();

        world.apply_respawn(&Respawn {
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 2,
                game_type: 0,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 0,
        });
        assert_eq!(world.active_tutorial_toast(), Some(active));
        assert_eq!(world.client_toast_events(), &retained_events);

        world.clear_client_level();
        assert_eq!(world.active_tutorial_toast(), None);
        assert_eq!(world.tutorial_step(), TutorialStep::Movement);
        world.advance_tutorial_ticks(1);
        assert_eq!(world.active_tutorial_toast(), None);
    }

    fn tutorial_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 1,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world
    }

    fn survival_facts() -> TutorialTickFacts {
        TutorialTickFacts {
            is_survival: true,
            has_find_tree_item: false,
            has_log_item: false,
            has_planks_item: false,
            has_mined_find_tree_block: false,
            has_crafted_planks: false,
        }
    }

    fn set_item_tag(world: &mut WorldStore, tag: &str, entries: Vec<i32>) {
        world.registries.tags.insert(
            "minecraft:item".to_string(),
            crate::RegistryTagState {
                registry: "minecraft:item".to_string(),
                tags: std::collections::BTreeMap::from([(tag.to_string(), entries)]),
            },
        );
    }
}
