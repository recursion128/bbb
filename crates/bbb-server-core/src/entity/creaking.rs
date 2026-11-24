//! `Creaking`'s heart binding (`Creaking.java:95-105`, `:243-256`, `:384-395`):
//! the `HOME_POS` synched data, the transient stuck counter, and the save
//! field both share with `Mob.home_pos`.
//!
//! The freeze semantics `aiStep` owns live in [`freeze`].

mod freeze;

pub(crate) use freeze::before_ai_step;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_CREAKING_CAN_MOVE_DATA_ID, VANILLA_CREAKING_HOME_POS_DATA_ID,
    VANILLA_CREAKING_IS_ACTIVE_DATA_ID, VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::NbtSummaryEntry;

use crate::entity::living::SynchedValue;

/// `Creaking.HOME_POS` (`Creaking.java:59`) and `playerStuckCounter`
/// (`:80`). The position is saved (`:384-395`) and the counter is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreakingRetainedState {
    home_pos: Option<BlockPos>,
    player_stuck_counter: i32,
    /// `Creaking.invulnerabilityAnimationRemainingTicks` (`:78`), transient.
    invulnerability_animation_remaining_ticks: i32,
    /// `Creaking.attackAnimationRemainingTicks` (`:76`), transient: `aiStep`
    /// decrements it (`:219-220`) and `doHurtTarget` sets it to fifteen
    /// (`:146`), which is `ATTACK_ANIMATION_DURATION` (`:63`).
    attack_animation_remaining_ticks: i32,
    /// `Creaking.CAN_MOVE` (`Creaking.java:56`, defined `true` at `:121`),
    /// which `checkCanMove` writes each tick.
    can_move: bool,
    /// `Creaking.IS_ACTIVE` (`:57`, defined `false` at `:122`), which
    /// `setIsActive` writes.
    is_active: bool,
    is_tearing_down: bool,
}

impl CreakingRetainedState {
    pub(in crate::entity) fn from_saved(home_pos: Option<BlockPos>) -> Self {
        Self {
            home_pos,
            player_stuck_counter: 0,
            invulnerability_animation_remaining_ticks: 0,
            attack_animation_remaining_ticks: 0,
            can_move: true,
            is_active: false,
            is_tearing_down: false,
        }
    }

    /// `Creaking.canMove` (`:136-138`).
    pub(crate) const fn can_move(&self) -> bool {
        self.can_move
    }

    /// `Creaking.isActive` (`:509-511`).
    pub(crate) const fn is_active(&self) -> bool {
        self.is_active
    }

    /// `this.entityData.set(CAN_MOVE, nowCanMove)` (`:236`).
    pub(crate) fn set_can_move(&mut self, can_move: bool) -> bool {
        if self.can_move == can_move {
            return false;
        }
        self.can_move = can_move;
        true
    }

    /// `Creaking.setIsActive` (`:505-507`).
    pub(crate) fn set_is_active(&mut self, is_active: bool) -> bool {
        if self.is_active == is_active {
            return false;
        }
        self.is_active = is_active;
        true
    }

    pub(crate) const fn is_tearing_down(&self) -> bool {
        self.is_tearing_down
    }

    pub(crate) fn set_tearing_down(&mut self) -> bool {
        if self.is_tearing_down {
            return false;
        }
        self.is_tearing_down = true;
        true
    }

    /// `Creaking.getHomePos` (`:393-395`).
    pub(crate) const fn home_pos(&self) -> Option<BlockPos> {
        self.home_pos
    }

    /// `Creaking.setTransient` (`:95-102`). The five `setPathfindingMalus`
    /// calls it makes land on the mobile's navigation table through
    /// [`apply_transient_pathfinding_malus`].
    pub(crate) fn set_transient(&mut self, pos: BlockPos) {
        self.home_pos = Some(pos);
    }

    /// `Creaking.aiStep`'s decrement (`:214-216`).
    pub(crate) fn tick_invulnerability_animation(&mut self) {
        if self.invulnerability_animation_remaining_ticks > 0 {
            self.invulnerability_animation_remaining_ticks -= 1;
        }
    }

    /// `Creaking.aiStep`'s second decrement (`:219-220`), which runs right
    /// after the invulnerability one.
    pub(crate) fn tick_attack_animation(&mut self) {
        if self.attack_animation_remaining_ticks > 0 {
            self.attack_animation_remaining_ticks -= 1;
        }
    }

    /// `this.attackAnimationRemainingTicks = 15` (`:146`), which
    /// `ATTACK_ANIMATION_DURATION` (`:63`) and `handleEntityEvent`'s fourth
    /// event (`:335`) both carry.
    pub(crate) fn begin_attack_animation(&mut self) {
        self.attack_animation_remaining_ticks = ATTACK_ANIMATION_DURATION;
    }

    /// `Creaking.attackAnimationRemainingTicks`, for the debug projection and
    /// the tests that pin the tick it was set on.
    pub(crate) const fn attack_animation_remaining_ticks(&self) -> i32 {
        self.attack_animation_remaining_ticks
    }

    /// `Creaking.hurtServer`'s absorption window (`:158`).
    pub(crate) const fn absorbing_hurt(&self) -> bool {
        self.invulnerability_animation_remaining_ticks <= 0
    }

    /// `this.invulnerabilityAnimationRemainingTicks = 8` (`:165`).
    pub(crate) fn begin_hurt_animation(&mut self) {
        self.invulnerability_animation_remaining_ticks = 8;
    }

    /// `Creaking.playerIsStuckInYou` (`:357-375`): one counter, incremented
    /// while a player's eye is inside the creaking's own box and reset the
    /// moment none is; the fifth consecutive tick answers true.
    pub(crate) fn note_player_eye_inside(&mut self, eye_inside: bool) -> bool {
        if !eye_inside {
            self.player_stuck_counter = 0;
            return false;
        }
        self.player_stuck_counter += 1;
        self.player_stuck_counter > 4
    }
}

/// `Creaking.ATTACK_ANIMATION_DURATION` (`Creaking.java:63`).
pub(crate) const ATTACK_ANIMATION_DURATION: i32 = 15;

/// `Creaking.setTransient`'s five `setPathfindingMalus` calls
/// (`Creaking.java:96-101`) over the path table the navigation reads.
///
/// Vanilla runs them where `setTransient` does: on the protector the heart has
/// just spawned (`CreakingHeartBlockEntity.java:211`) and again on load, where
/// `readAdditionalSaveData` reads `home_pos` back through the same method
/// (`Creaking.java:378-381`). `HOME_POS` is the only state that method writes
/// besides these, so a bound creaking's table is its type's plus these five and
/// an unbound one's is untouched -- the answer both call sites produce.
pub fn apply_transient_pathfinding_malus(
    malus: &mut crate::entity::ai::mob::navigation::malus::PathfindingMalus,
    entity: &crate::entity::Entity,
) {
    if !entity.is_heart_bound() {
        return;
    }
    use crate::entity::GroundPathType;
    malus.set(GroundPathType::Damaging, 8.0);
    malus.set(GroundPathType::PowderSnow, 8.0);
    malus.set(GroundPathType::Lava, 8.0);
    malus.set(GroundPathType::Fire, 0.0);
    malus.set(GroundPathType::FireInNeighbor, 0.0);
}

/// `Creaking.defineSynchedData` (`Creaking.java:119-125`) in declaration order.
/// `CAN_MOVE` and `IS_ACTIVE` carry what `checkCanMove` (`:454-489`) and
/// `setIsActive` (`:505-511`) wrote; `IS_TEARING_DOWN` follows
/// `removeProtector(damageSource)` (`CreakingHeartBlockEntity.java:311-323`).
pub(crate) fn synched_values(state: &CreakingRetainedState) -> [SynchedValue; 4] {
    let row = |data_id: u8, serializer_id: i32, value: EntityDataValueKind, is_default: bool| {
        SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id,
                value,
            },
            is_default,
        }
    };
    [
        row(
            VANILLA_CREAKING_CAN_MOVE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(state.can_move()),
            state.can_move(),
        ),
        row(
            VANILLA_CREAKING_IS_ACTIVE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(state.is_active()),
            !state.is_active(),
        ),
        row(
            VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(state.is_tearing_down()),
            !state.is_tearing_down(),
        ),
        row(
            VANILLA_CREAKING_HOME_POS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            EntityDataValueKind::OptionalBlockPos(state.home_pos()),
            state.home_pos().is_none(),
        ),
    ]
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &CreakingRetainedState) {
    entries.retain(|entry| entry.name != "home_pos");
    if let Some(pos) = state.home_pos {
        entries.push(crate::entity::wandering_trader::block_pos_entry(
            "home_pos", pos,
        ));
    }
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> CreakingRetainedState {
    // `Creaking.readAdditionalSaveData` (`:384`) reads the same key
    // `Mob.readAdditionalSaveData` (`:397`) did.
    CreakingRetainedState::from_saved(crate::entity::save::optional_block_pos(entries, "home_pos"))
}

impl crate::entity::Entity {
    /// `Creaking.getHomePos` (`Creaking.java:393-395`).
    #[must_use]
    pub fn creaking_home_pos(&self) -> Option<BlockPos> {
        match self.data() {
            crate::entity::EntityData::GeneralLiving(data) => data.creaking_retained()?.home_pos(),
            _ => None,
        }
    }

    /// `Creaking.isHeartBound` (`:104-106`): the binding decides fire immunity,
    /// portal use, the hurt sound and the tick that dies without its heart.
    #[must_use]
    pub fn is_heart_bound(&self) -> bool {
        self.creaking_home_pos().is_some()
    }

    /// `Creaking.canMove` (`Creaking.java:136-138`), the `CAN_MOVE` synched
    /// row. Every other type answers `true`, which is what a type that never
    /// defines the accessor reads.
    #[must_use]
    pub fn creaking_can_move(&self) -> bool {
        match self.data() {
            crate::entity::EntityData::GeneralLiving(data) => data
                .creaking_retained()
                .is_none_or(CreakingRetainedState::can_move),
            _ => true,
        }
    }

    /// `this.entityData.set(CAN_MOVE, nowCanMove)` (`Creaking.java:236`),
    /// whose `SynchedEntityData.set` marks the row dirty.
    pub fn set_creaking_can_move(&mut self, can_move: bool) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        if !state.set_can_move(can_move) {
            return false;
        }
        self.mark_synched_dirty(VANILLA_CREAKING_CAN_MOVE_DATA_ID);
        true
    }

    /// `Creaking.isActive` (`Creaking.java:509-511`).
    #[must_use]
    pub fn creaking_is_active(&self) -> bool {
        match self.data() {
            crate::entity::EntityData::GeneralLiving(data) => data
                .creaking_retained()
                .is_some_and(CreakingRetainedState::is_active),
            _ => false,
        }
    }

    /// `Creaking.setIsActive` (`Creaking.java:505-507`).
    pub fn set_creaking_active(&mut self, is_active: bool) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        if !state.set_is_active(is_active) {
            return false;
        }
        self.mark_synched_dirty(VANILLA_CREAKING_IS_ACTIVE_DATA_ID);
        true
    }

    #[must_use]
    pub fn creaking_is_tearing_down(&self) -> bool {
        match self.data() {
            crate::entity::EntityData::GeneralLiving(data) => data
                .creaking_retained()
                .is_some_and(CreakingRetainedState::is_tearing_down),
            _ => false,
        }
    }

    pub fn set_creaking_tearing_down(&mut self) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        if !state.set_tearing_down() {
            return false;
        }
        self.mark_synched_dirty(VANILLA_CREAKING_IS_TEARING_DOWN_DATA_ID);
        true
    }

    /// `Creaking.setTransient` (`:95-102`), which the heart runs on the
    /// protector it just spawned (`CreakingHeartBlockEntity.java:211`).
    pub fn set_creaking_transient(&mut self, pos: BlockPos) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        state.set_transient(pos);
        // `setHomePos` (`:389-391`) writes the `HOME_POS` accessor, whose
        // `SynchedEntityData.set` is what marks the row dirty.
        self.mark_synched_dirty(VANILLA_CREAKING_HOME_POS_DATA_ID);
        true
    }

    /// `Creaking.playerIsStuckInYou` (`:357-375`): the caller answers whether a
    /// player's eye is inside this creaking's box this tick.
    pub fn creaking_note_player_eye_inside(&mut self, eye_inside: bool) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        data.creaking_retained_mut()
            .is_some_and(|state| state.note_player_eye_inside(eye_inside))
    }

    /// `Creaking.aiStep`'s two animation decrements (`:214-216`, `:219-220`),
    /// which run in that order before `super.aiStep()`.
    pub fn tick_creaking_animations(&mut self) {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return;
        };
        if let Some(state) = data.creaking_retained_mut() {
            state.tick_invulnerability_animation();
            state.tick_attack_animation();
        }
    }

    /// `Creaking.attackAnimationRemainingTicks` (`:76`), the state
    /// `doHurtTarget` sets and `aiStep` decrements.
    #[must_use]
    pub fn creaking_attack_animation_ticks(&self) -> i32 {
        match self.data() {
            crate::entity::EntityData::GeneralLiving(data) => data
                .creaking_retained()
                .map_or(0, CreakingRetainedState::attack_animation_remaining_ticks),
            _ => 0,
        }
    }

    /// `this.attackAnimationRemainingTicks = 15` (`:146`).
    pub fn begin_creaking_attack_animation(&mut self) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        state.begin_attack_animation();
        true
    }

    /// `Creaking.hurtServer` (`:152-180`) as a decision. The caller answers the
    /// facts that need the level: the source's invulnerability tags, whether a
    /// living entity or projectile caused the damage directly, and whether a
    /// player is responsible for it.
    pub fn creaking_hurt_decision(
        &self,
        bypasses_invulnerability: bool,
        invulnerable_to_source: bool,
        direct_is_living_or_projectile: bool,
        responsible_player: bool,
    ) -> CreakingHurtDecision {
        let crate::entity::EntityData::GeneralLiving(data) = self.data() else {
            return CreakingHurtDecision::PassThrough;
        };
        let Some(state) = data.creaking_retained() else {
            return CreakingHurtDecision::PassThrough;
        };
        if state.home_pos().is_none() || bypasses_invulnerability {
            return CreakingHurtDecision::PassThrough;
        }
        // `:158`: the branch runs only while the creaking is not invulnerable
        // to the source and neither dead nor dying.
        if invulnerable_to_source
            || !state.absorbing_hurt()
            || !self.is_alive()
            || data.living().is_dead_or_dying()
        {
            return CreakingHurtDecision::Refuse;
        }
        if !direct_is_living_or_projectile && !responsible_player {
            return CreakingHurtDecision::Refuse;
        }
        CreakingHurtDecision::Absorb
    }

    /// `this.invulnerabilityAnimationRemainingTicks = 8` (`:165`).
    pub fn begin_creaking_hurt_animation(&mut self) -> bool {
        let crate::entity::EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(state) = data.creaking_retained_mut() else {
            return false;
        };
        state.begin_hurt_animation();
        true
    }
}

/// `Creaking.hurtServer`'s three outcomes (`Creaking.java:152-180`):
/// `Absorb` is the heart-bound branch that never applies damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreakingHurtDecision {
    /// `homePos == null` or the source bypasses invulnerability: run `super`.
    PassThrough,
    /// The branch answered `false` (`:161-163`, `:177-179`).
    Refuse,
    /// The heart-bound creaking swallows the hit (`:165-176`).
    Absorb,
}
