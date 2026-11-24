pub(crate) mod foraging;

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID, VANILLA_BEE_FLAGS_DATA_ID, VANILLA_BEE_FLAG_HAS_NECTAR,
    VANILLA_BEE_FLAG_HAS_STUNG, VANILLA_BEE_FLAG_ROLL,
};
use bbb_protocol::nbt_summary::uuid_nbt as uuid_value;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::general_living::hurt_from_environment;
use crate::entity::living::SynchedValue;
use crate::entity::save::{
    optional_block_pos, optional_bool, optional_int, optional_long, optional_uuid,
};
use crate::entity::{Entity, EntityData, EntityStore};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BeeRetainedState {
    flags: i8,
    anger_end_time: i64,
    anger_target: Option<Uuid>,
    hive_pos: Option<BlockPos>,
    flower_pos: Option<BlockPos>,
    ticks_without_nectar: i32,
    stay_out_of_hive_countdown: i32,
    crops_grown_since_pollination: i32,
}

struct BeeSavedFields {
    hive_pos: Option<BlockPos>,
    flower_pos: Option<BlockPos>,
    has_nectar: bool,
    has_stung: bool,
    ticks_without_nectar: i32,
    stay_out_of_hive_countdown: i32,
    crops_grown_since_pollination: i32,
    anger_end_time: i64,
    anger_target: Option<Uuid>,
}

impl BeeRetainedState {
    pub(crate) const fn persistent_anger(&self) -> (i64, Option<Uuid>) {
        (self.anger_end_time, self.anger_target)
    }

    pub(crate) fn set_persistent_anger(&mut self, end_time: i64, target: Option<Uuid>) -> bool {
        let changed = self.anger_end_time != end_time;
        self.anger_end_time = end_time;
        self.anger_target = target;
        changed
    }

    pub(crate) fn has_stung(&self) -> bool {
        self.has_flag(VANILLA_BEE_FLAG_HAS_STUNG)
    }

    /// `Bee.setHasStung(true)` (`Bee.java:508-510`).
    pub(crate) fn set_has_stung(&mut self) -> bool {
        self.set_flag(VANILLA_BEE_FLAG_HAS_STUNG, true)
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        let persisted = VANILLA_BEE_FLAG_HAS_NECTAR | VANILLA_BEE_FLAG_HAS_STUNG;
        self.flags = (self.flags & persisted) | (previous.flags & !persisted);
    }

    fn from_saved(saved: BeeSavedFields) -> Self {
        let mut state = Self {
            anger_end_time: saved.anger_end_time,
            anger_target: saved.anger_target,
            hive_pos: saved.hive_pos,
            flower_pos: saved.flower_pos,
            ticks_without_nectar: saved.ticks_without_nectar,
            stay_out_of_hive_countdown: saved.stay_out_of_hive_countdown,
            crops_grown_since_pollination: saved.crops_grown_since_pollination,
            ..Self::default()
        };
        state.set_flag(VANILLA_BEE_FLAG_HAS_NECTAR, saved.has_nectar);
        state.set_flag(VANILLA_BEE_FLAG_HAS_STUNG, saved.has_stung);
        state
    }

    fn has_flag(&self, flag: i8) -> bool {
        self.flags & flag != 0
    }

    fn set_flag(&mut self, flag: i8, value: bool) -> bool {
        let next = if value {
            self.flags | flag
        } else {
            self.flags & !flag
        };
        let changed = self.flags != next;
        self.flags = next;
        changed
    }

    fn custom_server_ai_step(&mut self) {
        if !self.has_flag(VANILLA_BEE_FLAG_HAS_NECTAR) {
            self.ticks_without_nectar = self.ticks_without_nectar.wrapping_add(1);
        }
    }

    fn after_ai_step(&mut self, rolling: bool) -> bool {
        if self.stay_out_of_hive_countdown > 0 {
            self.stay_out_of_hive_countdown -= 1;
        }
        self.set_flag(VANILLA_BEE_FLAG_ROLL, rolling)
    }

    #[cfg(test)]
    fn set_test_flags(&mut self, has_nectar: bool, has_stung: bool, rolling: bool) {
        self.set_flag(VANILLA_BEE_FLAG_HAS_NECTAR, has_nectar);
        self.set_flag(VANILLA_BEE_FLAG_HAS_STUNG, has_stung);
        self.set_flag(VANILLA_BEE_FLAG_ROLL, rolling);
    }
}

impl Default for BeeRetainedState {
    fn default() -> Self {
        Self {
            flags: 0,
            anger_end_time: -1,
            anger_target: None,
            hive_pos: None,
            flower_pos: None,
            ticks_without_nectar: 0,
            stay_out_of_hive_countdown: 0,
            crops_grown_since_pollination: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BeeRuntime {
    roll_amount: f32,
    roll_amount_o: f32,
    time_since_sting: i32,
    under_water_ticks: i32,
    remaining_hive_cooldown: i32,
    remaining_flower_cooldown: i32,
    pollination_ticks: i32,
    successful_pollination_ticks: i32,
    travelling_ticks: i32,
    known_flower_ticks: i32,
    hover_pos: Option<bbb_protocol::packets::Vec3d>,
    failed_flower: Option<BlockPos>,
}

impl BeeRuntime {
    /// Ends a pollination in progress; `false` when there was none.
    pub(crate) fn stop_pollinating(&mut self) -> bool {
        if self.pollination_ticks == 0 {
            return false;
        }
        self.pollination_ticks = 0;
        self.successful_pollination_ticks = 0;
        self.remaining_flower_cooldown = 200;
        true
    }

    pub(crate) fn new(random: &GameRandom) -> Self {
        Self {
            roll_amount: 0.0,
            roll_amount_o: 0.0,
            time_since_sting: 0,
            under_water_ticks: 0,
            pollination_ticks: 0,
            successful_pollination_ticks: 0,
            travelling_ticks: 0,
            known_flower_ticks: 0,
            hover_pos: None,
            failed_flower: None,
            remaining_hive_cooldown: 0,
            remaining_flower_cooldown: random.next_int(41) + 20,
        }
    }

    pub(crate) fn custom_server_ai_step(
        &mut self,
        random: &GameRandom,
        has_stung: bool,
        in_water: Option<bool>,
    ) -> BeeCustomStep {
        match in_water {
            Some(true) => self.under_water_ticks = self.under_water_ticks.wrapping_add(1),
            Some(false) => self.under_water_ticks = 0,
            None => {}
        }
        let drown = matches!(in_water, Some(true)) && self.under_water_ticks > 20;
        let mut starve = false;
        if has_stung {
            self.time_since_sting = self.time_since_sting.wrapping_add(1);
            if self.time_since_sting % 5 == 0 {
                let bound = 1200_i32.wrapping_sub(self.time_since_sting).clamp(1, 1200);
                starve = random.next_int(bound) == 0;
            }
        }
        BeeCustomStep { drown, starve }
    }

    pub(crate) fn after_ai_step(&mut self) {
        if self.remaining_hive_cooldown > 0 {
            self.remaining_hive_cooldown -= 1;
        }
        if self.remaining_flower_cooldown > 0 {
            self.remaining_flower_cooldown -= 1;
        }
    }

    pub(crate) fn after_tick(&mut self, rolling: bool) {
        self.roll_amount_o = self.roll_amount;
        self.roll_amount = if rolling {
            (self.roll_amount + 0.2).min(1.0)
        } else {
            (self.roll_amount - 0.24).max(0.0)
        };
    }

    #[cfg(test)]
    const fn counters(&self) -> (i32, i32, i32, i32) {
        (
            self.time_since_sting,
            self.under_water_ticks,
            self.remaining_hive_cooldown,
            self.remaining_flower_cooldown,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BeeCustomStep {
    drown: bool,
    starve: bool,
}

/// `Bee.dropOffNectar` (`Bee.java:630-633`).
pub(crate) fn drop_off_nectar(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(bee) = data.bee_retained_mut() else {
        return false;
    };
    if !bee.has_flag(VANILLA_BEE_FLAG_HAS_NECTAR) {
        return false;
    }
    bee.set_flag(VANILLA_BEE_FLAG_HAS_NECTAR, false);
    bee.crops_grown_since_pollination = 0;
    entity.mark_synched_dirty(VANILLA_BEE_FLAGS_DATA_ID);
    true
}

/// `Bee.setStayOutOfHiveCountdown` (`Bee.java:346-348`).
pub(crate) fn set_stay_out_of_hive_countdown(entity: &mut Entity, ticks: i32) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(bee) = data.bee_retained_mut() {
            bee.stay_out_of_hive_countdown = ticks;
        }
    }
}

/// `BeehiveBlockEntity.Occupant.createEntity` (`:401-429`) updates only bees.
pub(crate) fn prepare_hive_release(
    entity: &mut Entity,
    hive_pos: BlockPos,
    ticks_in_hive: i32,
    saved_flower_pos: Option<BlockPos>,
    world_random: &GameRandom,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(bee) = data.bee_retained_mut() else {
        return;
    };
    bee.hive_pos = Some(hive_pos);
    let baby_changed = data.ageable_retained_mut().is_some_and(|ageable| {
        if ageable.age_locked() {
            return false;
        }
        let age = ageable.age();
        let next = if age < 0 {
            age.wrapping_add(ticks_in_hive).min(0)
        } else if age > 0 {
            age.wrapping_sub(ticks_in_hive).max(0)
        } else {
            0
        };
        ageable.set_age(next)
    });
    if let Some(animal) = data.animal_retained_mut() {
        animal.set_in_love(animal.in_love().wrapping_sub(ticks_in_hive).max(0), None);
    }
    if let Some(flower) = saved_flower_pos {
        let bee = data
            .bee_retained_mut()
            .expect("Bee retained state was present");
        if bee.flower_pos.is_none() && world_random.next_float() < 0.9 {
            bee.flower_pos = Some(flower);
        }
    }
    if baby_changed {
        crate::entity::general_living::publish_baby_change(entity);
    }
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(
        name,
        "hive_pos"
            | "flower_pos"
            | "HasNectar"
            | "HasStung"
            | "TicksSincePollination"
            | "CannotEnterHiveTicks"
            | "CropsGrownSincePollination"
            | "anger_end_time"
            | "angry_at"
            | "AngerTime"
    )
}

pub(crate) fn custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    contact: Option<bool>,
) -> bool {
    let retained = match entity.data() {
        EntityData::GeneralLiving(data) => data.bee_retained(),
        _ => None,
    };
    let Some(retained) = retained else {
        return true;
    };
    let has_stung = retained.has_flag(VANILLA_BEE_FLAG_HAS_STUNG);
    let Some(contact) = contact else {
        return false;
    };
    let contact = Some(contact);
    let Some(step) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.bee_custom_server_ai_step(has_stung, contact))
    else {
        return true;
    };
    if step.drown {
        let _ = hurt_from_environment(entity, store, level, "minecraft:drown", 1.0);
    }
    if step.starve {
        let health = entity.living().map_or(0.0, |living| living.health());
        let _ = hurt_from_environment(entity, store, level, "minecraft:generic", health);
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.bee_retained_mut()
            .expect("classified Bee retained state")
            .custom_server_ai_step();
    }
    true
}

pub(crate) fn after_ai_step(entity: &mut Entity, store: &mut EntityStore, rolling: bool) {
    let entity_id = entity.id();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.bee_retained_mut() else {
        return;
    };
    store
        .mob_ai_mut(entity_id)
        .expect("Bee has a Mob runtime")
        .bee_after_ai_step();
    if state.after_ai_step(rolling) {
        entity.mark_synched_dirty(VANILLA_BEE_FLAGS_DATA_ID);
    }
}

pub(crate) fn after_super(entity: &Entity, store: &mut EntityStore) {
    let retained = match entity.data() {
        EntityData::GeneralLiving(data) => data.bee_retained(),
        _ => None,
    };
    let Some(state) = retained else {
        return;
    };
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.bee_after_tick(state.has_flag(VANILLA_BEE_FLAG_ROLL));
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &BeeRetainedState) {
    if let Some(pos) = state.hive_pos {
        entries.push(block_pos_entry("hive_pos", pos));
    }
    if let Some(pos) = state.flower_pos {
        entries.push(block_pos_entry("flower_pos", pos));
    }
    entries.extend([
        boolean_entry("HasNectar", state.has_flag(VANILLA_BEE_FLAG_HAS_NECTAR)),
        boolean_entry("HasStung", state.has_flag(VANILLA_BEE_FLAG_HAS_STUNG)),
        int_entry("TicksSincePollination", state.ticks_without_nectar),
        int_entry("CannotEnterHiveTicks", state.stay_out_of_hive_countdown),
        int_entry(
            "CropsGrownSincePollination",
            state.crops_grown_since_pollination,
        ),
        NbtSummaryEntry {
            name: "anger_end_time".into(),
            value: NbtSummaryValue::Long(state.anger_end_time),
        },
    ]);
    if let Some(target) = state.anger_target {
        entries.push(NbtSummaryEntry {
            name: "angry_at".into(),
            value: uuid_value(target),
        });
    }
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> BeeRetainedState {
    let nectar = optional_bool(entries, "HasNectar").unwrap_or(false);
    let stung = optional_bool(entries, "HasStung").unwrap_or(false);
    let ticks = optional_int(entries, "TicksSincePollination").unwrap_or(0);
    let cooldown = optional_int(entries, "CannotEnterHiveTicks").unwrap_or(0);
    let crops = optional_int(entries, "CropsGrownSincePollination").unwrap_or(0);
    let hive = optional_block_pos(entries, "hive_pos");
    let flower = optional_block_pos(entries, "flower_pos");
    BeeRetainedState::from_saved(BeeSavedFields {
        hive_pos: hive,
        flower_pos: flower,
        has_nectar: nectar,
        has_stung: stung,
        ticks_without_nectar: ticks,
        stay_out_of_hive_countdown: cooldown,
        crops_grown_since_pollination: crops,
        anger_end_time: optional_long(entries, "anger_end_time").unwrap_or(-1),
        anger_target: optional_uuid(entries, "angry_at"),
    })
}

pub(crate) fn synched_values(state: &BeeRetainedState) -> [SynchedValue; 2] {
    [
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_BEE_FLAGS_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(state.flags),
            },
            is_default: state.flags == 0,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_BEE_ANGER_END_TIME_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(state.anger_end_time),
            },
            is_default: state.anger_end_time == -1,
        },
    ]
}

fn block_pos_entry(name: &str, pos: BlockPos) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
    }
}

fn boolean_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

#[cfg(test)]
mod tests;
