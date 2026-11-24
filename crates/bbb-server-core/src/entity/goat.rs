use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_GOAT_LEFT_HORN_DATA_ID, VANILLA_GOAT_RIGHT_HORN_DATA_ID, VANILLA_GOAT_SCREAMING_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::general_living::{set_spawner_brain_memory, spawner_brain_memory};
use crate::entity::living::SynchedValue;
use crate::entity::save::optional_bool;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;

pub(crate) mod ram;

pub(crate) fn cooldown_active(entity: &Entity, memory_type: MemoryModuleType) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_GOAT_ID
        && spawner_brain_memory(entity, memory_type).is_some_and(|memory| {
            matches!(memory.value, MemoryValue::Int(ticks) if ticks > 0)
                && memory.time_to_live.is_none_or(|ttl| ttl > 0)
        })
}

// GoatAi.java:70-82.
pub(crate) fn before_ai(entity: &mut Entity) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_GOAT_ID {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    data.retained_brain_mut().memories.retain_mut(|memory| {
        if !matches!(
            memory.memory_type,
            MemoryModuleType::RamCooldownTicks | MemoryModuleType::LongJumpCooldownTicks
        ) {
            return true;
        }
        if let Some(ttl) = &mut memory.time_to_live {
            *ttl = ttl.saturating_sub(1);
            if *ttl <= 0 {
                return false;
            }
        }
        if let MemoryValue::Int(ticks) = &mut memory.value {
            *ticks = ticks.saturating_sub(1);
            *ticks > 0
        } else {
            true
        }
    });
}

// GoatAi.java:44; LongJumpToRandomPos.java:93-110; LongJumpMidJump.java:33-42.
pub(crate) fn reset_long_jump_cooldown(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    failed: bool,
) {
    let ticks = 600 + level.level_random().next_int(601);
    set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(if failed { ticks / 2 } else { ticks }),
            time_to_live: None,
        },
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GoatRetainedState {
    screaming: bool,
    left_horn: bool,
    right_horn: bool,
    is_lowering_head: bool,
    lower_head_tick: i32,
}

impl GoatRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.is_lowering_head = previous.is_lowering_head;
        self.lower_head_tick = previous.lower_head_tick;
    }

    const fn from_saved(screaming: bool, left_horn: bool, right_horn: bool) -> Self {
        Self {
            screaming,
            left_horn,
            right_horn,
            ..Self::DEFAULT
        }
    }

    const DEFAULT: Self = Self {
        screaming: false,
        left_horn: true,
        right_horn: true,
        is_lowering_head: false,
        lower_head_tick: 0,
    };

    pub(crate) const fn is_screaming(self) -> bool {
        self.screaming
    }

    fn ai_step(&mut self) {
        // `Goat.aiStep` advances this before the inherited step
        // (`Goat.java:277-285`).
        self.lower_head_tick =
            (self.lower_head_tick + if self.is_lowering_head { 1 } else { -2 }).clamp(0, 20);
    }

    #[cfg(test)]
    fn set_lowering_head(&mut self, value: bool) {
        self.is_lowering_head = value;
    }
}

impl Default for GoatRetainedState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpawnHornRemoval {
    None,
    Left,
    Right,
}

pub(crate) fn apply_spawn_traits(
    entity: &mut Entity,
    screaming: bool,
    horn_removal: SpawnHornRemoval,
) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.goat_retained_mut() else {
        return false;
    };
    let old = *state;
    state.screaming = screaming;
    match horn_removal {
        SpawnHornRemoval::None => {}
        SpawnHornRemoval::Left => state.left_horn = false,
        SpawnHornRemoval::Right => state.right_horn = false,
    }
    let dirty = [
        (old.screaming != state.screaming).then_some(VANILLA_GOAT_SCREAMING_DATA_ID),
        (old.left_horn != state.left_horn).then_some(VANILLA_GOAT_LEFT_HORN_DATA_ID),
        (old.right_horn != state.right_horn).then_some(VANILLA_GOAT_RIGHT_HORN_DATA_ID),
    ];
    for data_id in dirty.into_iter().flatten() {
        entity.mark_synched_dirty(data_id);
    }
    age_boundary_reached(entity);
    true
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &GoatRetainedState) {
    // `Goat.addAdditionalSaveData` (`Goat.java:250-255`).
    entries.extend([
        boolean_entry("IsScreamingGoat", state.screaming),
        boolean_entry("HasLeftHorn", state.left_horn),
        boolean_entry("HasRightHorn", state.right_horn),
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> GoatRetainedState {
    // `Goat.readAdditionalSaveData` defaults the three booleans independently
    // (`Goat.java:258-263`).
    GoatRetainedState::from_saved(
        optional_bool(entries, "IsScreamingGoat").unwrap_or(false),
        optional_bool(entries, "HasLeftHorn").unwrap_or(true),
        optional_bool(entries, "HasRightHorn").unwrap_or(true),
    )
}

pub(crate) fn synched_values(state: &GoatRetainedState) -> [SynchedValue; 3] {
    // `Goat.defineSynchedData` (`Goat.java:289-294`).
    [
        boolean_value(VANILLA_GOAT_SCREAMING_DATA_ID, state.screaming, false),
        boolean_value(VANILLA_GOAT_LEFT_HORN_DATA_ID, state.left_horn, true),
        boolean_value(VANILLA_GOAT_RIGHT_HORN_DATA_ID, state.right_horn, true),
    ]
}

pub(crate) fn before_ai_step(entity: &mut Entity) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if let Some(state) = data.goat_retained_mut() {
        state.ai_step();
    }
}

pub(crate) fn set_lowering_head(entity: &mut Entity, lowering: bool) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.goat_retained_mut() {
            state.is_lowering_head = lowering;
        }
    }
}

pub(crate) fn age_boundary_reached(entity: &mut Entity) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data.goat_retained().is_none() {
        return;
    }
    let base_value = if data.ageable_retained().is_some_and(|state| state.is_baby()) {
        1.0
    } else {
        2.0
    };
    // `Goat.ageBoundaryReached` (`Goat.java:112-114`).
    if let Some(instance) = data
        .living_mut()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
    {
        instance.set_base_value(base_value);
    }
}

pub(crate) fn finish_load(entity: &mut Entity) {
    age_boundary_reached(entity);
}

pub(crate) fn adjust_fall_damage(entity: &Entity, damage: i32) -> i32 {
    // `Goat.calculateFallDamage` subtracts ten from the inherited result
    // (`Goat.java:117-119`).
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_GOAT_ID {
        damage - 10
    } else {
        damage
    }
}

fn boolean_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

fn boolean_value(data_id: u8, value: bool, default: bool) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        },
        is_default: value == default,
    }
}

#[cfg(test)]
mod tests;
