//! The per-type retained state this base owns and the catalog predicates
//! that select it.

use bbb_protocol::entity_types;

use super::*;

// HappyGhast.java:71-74,407-432,525-532: only the still timer is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct HappyGhastState {
    pub(super) time: i32,
    pub(super) is_leash_holder: bool,
    pub(crate) still_timeout: i32,
    pub(crate) stays_still: bool,
    pub(crate) requires_precise_position: bool,
    pub(crate) panic_until: Option<i64>,
}

impl HappyGhastState {
    pub(crate) fn set_still_timeout(&mut self, timeout: i32) -> bool {
        let changed = self.stays_still != (timeout > 0);
        self.still_timeout = timeout;
        self.stays_still = timeout > 0;
        changed
    }
}

pub(crate) const DEFAULT_MOB_EQUIPMENT_DROP_CHANCE: f32 = 0.085;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MobRetainedState {
    pub(super) flags: i8,
    pub(super) can_pick_up_loot: bool,
    pub(super) persistence_required: bool,
    pub(super) drop_chances: [f32; VANILLA_EQUIPMENT_SLOTS.len()],
    pub(crate) leash: Option<crate::entity::leash::LeashData>,
    pub(crate) home_position: BlockPos,
    pub(crate) home_radius: i32,
    /// `Mob.lootTable` (`Mob.java:137`), the override `Mob.getLootTable`
    /// prefers over the entity type's default (`:413-415`) and
    /// `Mob.dropFromLootTable` clears once it has rolled (`:407-410`).
    pub(super) death_loot_table: Option<String>,
    /// `Mob.lootTableSeed` (`Mob.java:138`), which `getLootTableSeed`
    /// (`:418-420`) answers in place of the zero `LivingEntity` answers
    /// (`LivingEntity.java:1507-1509`).
    pub(super) death_loot_table_seed: i64,
}

// `Vex` retains its bound, optional owner, limited lifetime, and tracked flag
// byte independently (`Vex.java:45-81`, `Vex.java:293-355`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VexRetainedState {
    pub(super) flags: i8,
    owner: Option<Uuid>,
    bound_origin: Option<BlockPos>,
    limited_life_ticks: Option<i32>,
}

impl VexRetainedState {
    pub(crate) const fn from_saved(
        owner: Option<Uuid>,
        bound_origin: Option<BlockPos>,
        limited_life_ticks: Option<i32>,
    ) -> Self {
        Self {
            flags: 0,
            owner,
            bound_origin,
            limited_life_ticks,
        }
    }

    pub(crate) const fn flags(&self) -> i8 {
        self.flags
    }

    pub(crate) const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub(crate) const fn bound_origin(&self) -> Option<BlockPos> {
        self.bound_origin
    }

    pub(crate) const fn limited_life_ticks(&self) -> Option<i32> {
        self.limited_life_ticks
    }

    /// `setOwner`, `setBoundOrigin` and `setLimitedLife` of an evoker's
    /// summon (`Evoker.java:254-256`).
    pub(crate) fn set_summoned(&mut self, owner: Uuid, bound_origin: BlockPos, life_ticks: i32) {
        self.owner = Some(owner);
        self.bound_origin = Some(bound_origin);
        self.limited_life_ticks = Some(life_ticks);
    }

    pub(super) fn tick_limited_life(&mut self) -> bool {
        let Some(ticks) = &mut self.limited_life_ticks else {
            return false;
        };
        *ticks = ticks.wrapping_sub(1);
        if *ticks <= 0 {
            *ticks = 20;
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    pub(crate) fn set_flags(&mut self, flags: i8) {
        self.flags = flags;
    }
}

// `TamableAnimal`'s synched, save and restore state (`TamableAnimal.java:34-83`,
// `TamableAnimal.java:115-165`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TamableRetainedState {
    flags: i8,
    owner: Option<Uuid>,
    ordered_to_sit: bool,
    pub(in crate::entity) death_combat:
        Option<Box<crate::entity::taming::death_message::PetCombat>>,
}

impl TamableRetainedState {
    pub(super) const fn new(entity_type_id: i32) -> Option<Self> {
        if is_tamable_entity_type(entity_type_id) {
            Some(Self {
                flags: 0,
                owner: None,
                ordered_to_sit: false,
                death_combat: None,
            })
        } else {
            None
        }
    }

    pub(crate) fn from_saved(owner: Option<Uuid>, ordered_to_sit: bool) -> Self {
        let mut state = Self {
            flags: 0,
            owner,
            ordered_to_sit,
            death_combat: None,
        };
        state.set_flag(VANILLA_TAMABLE_ANIMAL_FLAG_TAME, state.owner.is_some());
        state.set_flag(VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, ordered_to_sit);
        state
    }

    fn set_flag(&mut self, mask: i8, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }

    pub(crate) const fn flags(&self) -> i8 {
        self.flags
    }

    pub(crate) const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub(crate) const fn ordered_to_sit(&self) -> bool {
        self.ordered_to_sit
    }

    pub(crate) const fn is_tame(&self) -> bool {
        self.flags & VANILLA_TAMABLE_ANIMAL_FLAG_TAME != 0
    }

    /// `TamableAnimal.isInSittingPose` (`TamableAnimal.java:142-144`), which
    /// `Wolf.canMate` reads off the partner (`Wolf.java:618`).
    pub(crate) const fn is_in_sitting_pose(&self) -> bool {
        self.flags & VANILLA_TAMABLE_ANIMAL_FLAG_SITTING != 0
    }

    pub(crate) fn set_in_sitting_pose(&mut self, sitting: bool) {
        self.set_flag(VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, sitting);
    }

    /// `TamableAnimal.setTame`'s flag half (`TamableAnimal.java:119-125`); the
    /// `includeSideEffects` half is the subtype's `applyTamingSideEffects`
    /// (`:132-133`) and belongs to its own module.
    pub(crate) fn set_tame(&mut self, tame: bool) {
        self.set_flag(VANILLA_TAMABLE_ANIMAL_FLAG_TAME, tame);
    }

    /// `TamableAnimal.setOwner` (`TamableAnimal.java:160-162`) as the reference
    /// this crate keeps: the UUID, which is what `EntityReference.of` stores
    /// and what the `Owner` save field round-trips.
    pub(crate) fn set_owner(&mut self, owner: Option<Uuid>) {
        self.owner = owner;
    }

    /// `TamableAnimal.setOrderedToSit` (`TamableAnimal.java:237-239`) together
    /// with `setInSittingPose` (`:145-153`).
    ///
    /// Vanilla writes the pose bit from `SitWhenOrderedToGoal.start`/`stop`
    /// (`SitWhenOrderedToGoal.java:40-43`); coupling it to the order is the
    /// coupling `readAdditionalSaveData` already applies
    /// (`TamableAnimal.java:81-82`), and the goal only holds the movement.
    pub(crate) fn set_ordered_to_sit(&mut self, ordered_to_sit: bool) {
        self.ordered_to_sit = ordered_to_sit;
        self.set_flag(VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, ordered_to_sit);
    }
}

pub(crate) const fn is_tamable_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAT_ID
            | VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_PARROT_ID
            | VANILLA_ENTITY_TYPE_WOLF_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

impl GeneralLivingData {
    /// `Ocelot.setTrusting` (`Ocelot.java:72-75`); false for a non-ocelot.
    pub(crate) fn set_ocelot_trusting(&mut self, trusting: bool) -> bool {
        match &mut self.kind {
            MobVariantState::Ocelot { trusting: value } => {
                *value = trusting;
                true
            }
            _ => false,
        }
    }
}

// `EnderMan` retains and tracks its carried block state
// (`EnderMan.java:141-146`, `EnderMan.java:192-206`, `EnderMan.java:343-349`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndermanRetainedState {
    carried_block_state_id: Option<i32>,
    /// `DATA_CREEPY` and `DATA_STARED_AT`, which no save field carries.
    creepy: bool,
    stared_at: bool,
}

impl EndermanRetainedState {
    pub(crate) const fn from_saved(carried_block_state_id: Option<i32>) -> Self {
        Self {
            carried_block_state_id,
            creepy: false,
            stared_at: false,
        }
    }

    pub(crate) const fn carried_block_state_id(self) -> Option<i32> {
        self.carried_block_state_id
    }

    pub(crate) fn set_carried_block_state_id(&mut self, state: Option<i32>) -> bool {
        std::mem::replace(&mut self.carried_block_state_id, state) != state
    }

    pub(crate) const fn creepy(self) -> bool {
        self.creepy
    }

    pub(crate) const fn stared_at(self) -> bool {
        self.stared_at
    }
}

/// `Mob.setAggressive` (`Mob.java:1292-1295`), publishing only a change.
pub(crate) fn set_aggressive(entity: &mut Entity, aggressive: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data
        .mob_retained_mut()
        .is_some_and(|mob| mob.set_aggressive(aggressive))
    {
        entity.mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_MOB_FLAGS_DATA_ID);
    }
}

/// `PolarBear.setStanding` (`PolarBear.java:223-225`).
pub(crate) fn set_polar_bear_standing(entity: &mut Entity, value: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if let MobVariantState::PolarBear { standing } = &mut data.kind {
        if std::mem::replace(standing, value) != value {
            entity.mark_synched_dirty(
                bbb_protocol::entity_synched_data::VANILLA_POLAR_BEAR_STANDING_DATA_ID,
            );
        }
    }
}

/// `Vex.setIsCharging` (`Vex.java:165-167`).
pub(crate) fn set_vex_charging(entity: &mut Entity, charging: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(vex) = data.vex_retained_mut() else {
        return;
    };
    let flags = if charging {
        vex.flags | 1
    } else {
        vex.flags & !1
    };
    if std::mem::replace(&mut vex.flags, flags) != flags {
        entity.mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_VEX_FLAGS_DATA_ID);
    }
}

/// `entityData.set(DATA_CREEPY, ..)` and `set(DATA_STARED_AT, ..)`, which
/// publish only a changed value.
pub(crate) fn set_enderman_stare_flags(
    entity: &mut Entity,
    creepy: Option<bool>,
    stared_at: Option<bool>,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.enderman_retained_mut() else {
        return;
    };
    let mut dirty = Vec::new();
    if let Some(creepy) = creepy.filter(|creepy| *creepy != state.creepy) {
        state.creepy = creepy;
        dirty.push(bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_CREEPY_DATA_ID);
    }
    if let Some(stared_at) = stared_at.filter(|stared_at| *stared_at != state.stared_at) {
        state.stared_at = stared_at;
        dirty.push(bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_STARED_AT_DATA_ID);
    }
    for data_id in dirty {
        entity.mark_synched_dirty(data_id);
    }
}

// `AgeableMob`'s server state, persistence and synced rows
// (`AgeableMob.java:28-40`, `:104-120`, `:149-175`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgeableMobRetainedState {
    age: i32,
    forced_age: i32,
    age_locked: bool,
    /// `AgeableMob.ageLockParticleTimer` (`AgeableMob.java:40`), which no save
    /// field carries (`:162-176`).
    age_lock_particle_timer: i32,
}

impl AgeableMobRetainedState {
    pub(super) fn new(entity_type_id: i32, is_animal: bool) -> Option<Self> {
        (is_animal
            || matches!(
                entity_type_id,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
            ))
        .then(|| Self::from_saved(0, 0, false))
    }

    pub(crate) const fn from_saved(age: i32, forced_age: i32, age_locked: bool) -> Self {
        Self {
            age,
            forced_age,
            age_locked,
            age_lock_particle_timer: 0,
        }
    }

    pub(crate) const fn age(self) -> i32 {
        self.age
    }

    pub(crate) const fn forced_age(self) -> i32 {
        self.forced_age
    }

    pub(crate) const fn age_locked(self) -> bool {
        self.age_locked
    }

    pub(crate) const fn age_lock_particle_timer(self) -> i32 {
        self.age_lock_particle_timer
    }

    /// `AgeableMob.setAgeLockedData` (`AgeableMob.java:82-86`): the flip, the
    /// baby-start age and the `AGE_LOCK_COOLDOWN_TICKS` press (`:32`).
    pub(crate) fn toggle_age_lock(&mut self, baby_start_age: i32) -> bool {
        self.age_locked = !self.age_locked;
        self.age_lock_particle_timer = 40;
        self.set_age(baby_start_age)
    }

    /// `AgeableMob.makeAgeLockedParticle`'s counter (`AgeableMob.java:214-228`),
    /// run from `aiStep` (`:207`); the particle itself is the client's own
    /// (`:216`).
    pub(crate) fn tick_age_lock_particle_timer(&mut self) {
        if self.age_lock_particle_timer > 0 {
            self.age_lock_particle_timer -= 1;
        }
    }

    pub(crate) const fn is_baby(self) -> bool {
        self.age < 0
    }

    /// `AgeableMob.canAgeUp()` (`AgeableMob.java:210-212`).
    pub(crate) const fn can_age_up(self) -> bool {
        self.is_baby() && !self.age_locked
    }

    pub(crate) fn set_age(&mut self, age: i32) -> bool {
        let was_baby = self.is_baby();
        self.age = age;
        was_baby != self.is_baby()
    }

    pub(super) fn tick(&mut self, can_age_up: bool) -> bool {
        let was_baby = self.is_baby();
        if self.age < 0 && can_age_up && !self.age_locked {
            self.age += 1;
        } else if self.age > 0 {
            self.age -= 1;
        }
        was_baby != self.is_baby()
    }

    pub(crate) fn age_up(&mut self, seconds: i32, forced: bool) -> bool {
        // `AgeableMob.ageUp` (`AgeableMob.java:123-143`), whose clamp precedes
        // the forced-age bookkeeping and the reapply at `:140-142`.
        let old_age = self.age;
        let mut age = old_age.wrapping_add(seconds.wrapping_mul(20));
        if age > 0 {
            age = 0;
        }
        let delta = age.wrapping_sub(old_age);
        let was_baby = self.is_baby();
        self.age = age;
        if forced {
            // `forcedAgeTimer = 40` (`:135-137`) only drives a client particle
            // this server never sends; the counter it arms is not synched data.
            self.forced_age = self.forced_age.wrapping_add(delta);
        }
        if self.age == 0 {
            self.age = self.forced_age;
        }
        was_baby != self.is_baby()
    }
}

// `Animal` retains love state across ticks and saves (`Animal.java:42-108`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimalRetainedState {
    in_love: i32,
    love_cause: Option<Uuid>,
}

impl AnimalRetainedState {
    pub(super) fn new(entity_type_id: i32) -> Option<Self> {
        is_base_animal_entity_type(entity_type_id).then(|| Self::from_saved(0, None))
    }

    pub(crate) const fn from_saved(in_love: i32, love_cause: Option<Uuid>) -> Self {
        Self {
            in_love,
            love_cause,
        }
    }

    pub(crate) const fn in_love(self) -> i32 {
        self.in_love
    }

    pub(crate) const fn love_cause(self) -> Option<Uuid> {
        self.love_cause
    }

    /// `Animal.isInLove()` (`Animal.java:193-195`).
    pub(crate) const fn is_in_love(self) -> bool {
        self.in_love > 0
    }

    pub(crate) fn reset_love(&mut self) {
        self.in_love = 0;
    }

    /// `Animal.setInLove`'s two writes (`Animal.java:172-179`): the timer, and
    /// the cause only when the player is a `ServerPlayer` (`:174-176` leaves an
    /// older cause in place otherwise).
    pub(crate) fn set_in_love(&mut self, in_love: i32, cause: Option<Uuid>) {
        self.in_love = in_love;
        if cause.is_some() {
            self.love_cause = cause;
        }
    }

    pub(super) fn ai_step(&mut self, age: i32) -> bool {
        if age != 0 {
            self.reset_love();
            return false;
        }
        if self.in_love <= 0 {
            return false;
        }
        self.in_love -= 1;
        self.in_love % 10 == 0
    }
}

pub(crate) const fn is_base_animal_entity_type(entity_type_id: i32) -> bool {
    is_abstract_horse_entity_type(entity_type_id)
        || matches!(
            entity_type_id,
            entity_types::VANILLA_ENTITY_TYPE_BEE_ID
                | entity_types::VANILLA_ENTITY_TYPE_FOX_ID
                | entity_types::VANILLA_ENTITY_TYPE_GOAT_ID
                | entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID
                | entity_types::VANILLA_ENTITY_TYPE_COW_ID
                | entity_types::VANILLA_ENTITY_TYPE_FROG_ID
                | entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
                | entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID
                | entity_types::VANILLA_ENTITY_TYPE_PIG_ID
                | entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID
                | entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID
                | entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID
                | entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID
                | entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID
                | entity_types::VANILLA_ENTITY_TYPE_CAT_ID
                | entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
                | entity_types::VANILLA_ENTITY_TYPE_PARROT_ID
                | entity_types::VANILLA_ENTITY_TYPE_WOLF_ID
                | entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
                | entity_types::VANILLA_ENTITY_TYPE_OCELOT_ID
                | entity_types::VANILLA_ENTITY_TYPE_PANDA_ID
                | entity_types::VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
                | entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID
                | entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID
                | entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID
        )
}

// `AbstractHorse` owns one tracked flag byte plus retained temper/owner and
// autonomous counters (`AbstractHorse.java:96-125`, `AbstractHorse.java:154-170`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AbstractHorseRetainedState {
    pub(super) flags: i8,
    temper: i32,
    owner: Option<Uuid>,
    pub(super) eating_counter: i32,
    pub(super) mouth_counter: i32,
    pub(super) stand_counter: i32,
    pub(super) tail_counter: i32,
    rearing_amount: f32,
}

impl AbstractHorseRetainedState {
    pub(super) fn new(entity_type_id: i32) -> Option<Self> {
        is_abstract_horse_entity_type(entity_type_id)
            .then(|| Self::from_saved(false, false, 0, false, None))
    }

    pub(crate) fn from_saved(
        eating: bool,
        bred: bool,
        temper: i32,
        tame: bool,
        owner: Option<Uuid>,
    ) -> Self {
        let mut state = Self {
            flags: 0,
            temper,
            owner,
            eating_counter: 0,
            mouth_counter: 0,
            stand_counter: 0,
            tail_counter: 0,
            rearing_amount: 0.0,
        };
        state.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_EATING, eating);
        state.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_BRED, bred);
        state.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_TAME, tame);
        state
    }

    pub(crate) const fn flags(&self) -> i8 {
        self.flags
    }

    pub(crate) const fn rearing_amount(&self) -> f32 {
        self.rearing_amount
    }

    pub(crate) const fn temper(&self) -> i32 {
        self.temper
    }

    pub(crate) const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub(crate) const fn is_eating(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_HORSE_FLAG_EATING != 0
    }

    pub(crate) const fn is_bred(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_HORSE_FLAG_BRED != 0
    }

    pub(crate) const fn is_tame(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_HORSE_FLAG_TAME != 0
    }

    pub(crate) const fn is_standing(&self) -> bool {
        self.flags & VANILLA_ABSTRACT_HORSE_FLAG_STANDING != 0
    }

    fn set_flag(&mut self, mask: i8, value: bool) -> bool {
        let flags = if value {
            self.flags | mask
        } else {
            self.flags & !mask
        };
        let changed = flags != self.flags;
        self.flags = flags;
        changed
    }

    // `AbstractHorse.modifyTemper` (`AbstractHorse.java:247-250`) over
    // `setTemper` (`:243-245`); the clamp reads the subtype's `getMaxTemper`.
    pub(crate) fn modify_temper(&mut self, amount: i32, max_temper: i32) -> i32 {
        self.temper = (self.temper.saturating_add(amount)).clamp(0, max_temper);
        self.temper
    }

    /// `AbstractHorse.setTamed` (`AbstractHorse.java:185-187`).
    pub(crate) fn set_tame(&mut self, tame: bool) -> bool {
        self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_TAME, tame)
    }

    /// `AbstractHorse.setOwner` (`AbstractHorse.java:181-183`); the field is
    /// not synched, only saved as `Owner` (`:794`).
    pub(crate) fn set_owner(&mut self, owner: Option<Uuid>) {
        self.owner = owner;
    }

    /// `AbstractHorse.setEating` (`AbstractHorse.java:677-679`).
    pub(crate) fn set_eating(&mut self, eating: bool) -> bool {
        self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_EATING, eating)
    }

    /// `AbstractHorse.openMouth` (`AbstractHorse.java:670-675`).
    pub(crate) fn open_mouth(&mut self) -> bool {
        self.mouth_counter = 1;
        self.set_flag(
            bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
            true,
        )
    }

    /// `AbstractHorse.clearStanding` (`AbstractHorse.java:687-690`).
    pub(crate) fn clear_standing(&mut self) -> bool {
        let changed = self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_STANDING, false);
        self.stand_counter = 0;
        changed
    }

    pub(super) fn move_tail(&mut self) {
        self.tail_counter = 1;
    }

    pub(super) fn set_standing(&mut self, ticks: i32) -> bool {
        let eating_changed = self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_EATING, false);
        let standing_changed = self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_STANDING, true);
        self.stand_counter = ticks;
        eating_changed || standing_changed
    }

    pub(super) fn post_ai_step(&mut self, can_eat_grass: bool, start_eating: bool) -> bool {
        if !can_eat_grass {
            return false;
        }
        let mut changed = start_eating && self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_EATING, true);
        if self.is_eating() {
            self.eating_counter += 1;
            if self.eating_counter > 50 {
                self.eating_counter = 0;
                changed |= self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_EATING, false);
            }
        }
        changed
    }

    pub(super) fn tick_counters(&mut self) -> bool {
        let mut changed = false;
        // `AbstractHorse.tick` (`AbstractHorse.java:577-582`).
        if self.mouth_counter > 0 {
            self.mouth_counter += 1;
            if self.mouth_counter > 30 {
                self.mouth_counter = 0;
                changed |= self.set_flag(
                    bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH,
                    false,
                );
            }
        }
        if self.stand_counter > 0 {
            self.stand_counter -= 1;
            if self.stand_counter <= 0 {
                changed |= self.set_flag(VANILLA_ABSTRACT_HORSE_FLAG_STANDING, false);
                self.stand_counter = 0;
            }
        }
        if self.tail_counter > 0 {
            self.tail_counter += 1;
            if self.tail_counter > 8 {
                self.tail_counter = 0;
            }
        }
        // AbstractHorse.java:612-625: rearing also moves the passenger attachment.
        let amount = self.rearing_amount;
        self.rearing_amount = if self.is_standing() {
            (amount + (1.0 - amount) * 0.4 + 0.05).min(1.0)
        } else {
            (amount + (0.8 * amount * amount * amount - amount) * 0.6 - 0.05).max(0.0)
        };
        changed
    }
}

pub(crate) const fn is_abstract_horse_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

// `AbstractFish.java:35-35`; concrete leaves are `Cod.java:11-11`,
// `Pufferfish.java:30-30`, `Salmon.java:35-35`, `Tadpole.java:44-44`, and
// `TropicalFish.java:47-47`.
pub(crate) const fn is_water_animal_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_COD_ID
            | VANILLA_ENTITY_TYPE_PUFFERFISH_ID
            | VANILLA_ENTITY_TYPE_SALMON_ID
            | VANILLA_ENTITY_TYPE_TADPOLE_ID
            | VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID
    )
}

// `AgeableWaterCreature` has three concrete leaves; Dolphin replaces its air
// hook while Squid and GlowSquid inherit it (`AgeableWaterCreature.java:16-54`,
// `Dolphin.java:107-108`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgeableWaterCreatureRetainedState {
    /// `Dolphin.java:70-78`.
    Dolphin {
        got_fish: bool,
        moistness: i32,
        treasure_pos: Option<BlockPos>,
    },
    Squid,
    GlowSquid {
        dark_ticks_remaining: i32,
    },
}

impl AgeableWaterCreatureRetainedState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_DOLPHIN_ID => Some(Self::Dolphin {
                got_fish: false,
                moistness: 2400,
                treasure_pos: None,
            }),
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => Some(Self::GlowSquid {
                dark_ticks_remaining: 0,
            }),
            VANILLA_ENTITY_TYPE_SQUID_ID => Some(Self::Squid),
            _ => None,
        }
    }

    pub(crate) const fn from_saved_dolphin(got_fish: bool, moistness: i32) -> Self {
        Self::Dolphin {
            got_fish,
            moistness,
            treasure_pos: None,
        }
    }

    pub(crate) const fn from_saved_glow_squid(dark_ticks_remaining: i32) -> Self {
        Self::GlowSquid {
            dark_ticks_remaining,
        }
    }

    pub(crate) const fn dolphin_fields(self) -> Option<(bool, i32)> {
        match self {
            Self::Dolphin {
                got_fish,
                moistness,
                ..
            } => Some((got_fish, moistness)),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn glow_squid_dark_ticks(self) -> Option<i32> {
        match self {
            Self::GlowSquid {
                dark_ticks_remaining,
            } => Some(dark_ticks_remaining),
            _ => None,
        }
    }

    pub(super) fn set_dolphin_moistness(&mut self, value: i32) -> bool {
        let Self::Dolphin { moistness, .. } = self else {
            return false;
        };
        let changed = *moistness != value;
        *moistness = value;
        changed
    }

    pub(super) fn tick_glow_squid(&mut self) -> bool {
        let Self::GlowSquid {
            dark_ticks_remaining,
        } = self
        else {
            return false;
        };
        if *dark_ticks_remaining <= 0 {
            return false;
        }
        *dark_ticks_remaining -= 1;
        true
    }

    pub(super) fn set_glow_squid_dark_ticks(&mut self, value: i32) -> bool {
        let Self::GlowSquid {
            dark_ticks_remaining,
        } = self
        else {
            return false;
        };
        let changed = *dark_ticks_remaining != value;
        *dark_ticks_remaining = value;
        changed
    }
}

pub(super) fn animal_custom_server_ai_step_calls_super(resource_id: &str) -> bool {
    !matches!(
        resource_id,
        // These overrides do not call `Animal.customServerAiStep`.
        // (`Bee.java:364-388`, `Axolotl.java:365-377`, `Cat.java:228-245`,
        // `Ocelot.java:110-127`, `Rabbit.java:205-251`).
        "minecraft:bee"
            | "minecraft:axolotl"
            | "minecraft:cat"
            | "minecraft:ocelot"
            | "minecraft:rabbit"
    )
}
