use crate::world::simulation_level::LevelCtx;
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_FOX_VARIANT_RED_ID, VANILLA_FOX_VARIANT_SNOW_ID,
};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_FOX_FLAGS_DATA_ID, VANILLA_FOX_FLAG_DEFENDING, VANILLA_FOX_TRUSTED_DATA_ID_0,
    VANILLA_FOX_TRUSTED_DATA_ID_1, VANILLA_FOX_TYPE_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::nbt_summary::uuid_nbt as uuid_value;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

pub(crate) mod breeding;

use crate::entity::environment::FluidContact;
use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_field};
use crate::entity::{Entity, EntityData};

// Fox.java:659-682.
pub(crate) fn ambient_sound(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
) -> Option<(&'static str, f32)> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    if data.fox_retained()?.is_sleeping() {
        return Some(("minecraft:entity.fox.sleep", 1.0));
    }
    if !level.is_bright_outside()?
        && level.level_random().next_float() < 0.1
        && !level.players().iter().any(|player| {
            !player.spectator
                && player
                    .bounding_box()
                    .intersects(entity.bounding_box().inflate(16.0, 16.0, 16.0))
        })
    {
        return Some(("minecraft:entity.fox.screech", 2.0));
    }
    Some(("minecraft:entity.fox.ambient", 1.0))
}

// `Fox` packs these states into its one tracked flag byte (`Fox.java:106-123`).
const FLAG_SITTING: i8 = 1;
const FLAG_CROUCHING: i8 = 4;
const FLAG_INTERESTED: i8 = 8;
const FLAG_POUNCING: i8 = 16;
const FLAG_SLEEPING: i8 = 32;
const FLAG_FACEPLANTED: i8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoxHuntAction {
    Sit,
    Stand,
    Stalk,
    Charge,
    ClearCharge,
    Pounce,
    StopPounce,
    Faceplant,
    Recover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FoxVariant {
    Red,
    Snow,
}

impl FoxVariant {
    const fn id(self) -> i32 {
        match self {
            Self::Red => VANILLA_FOX_VARIANT_RED_ID,
            Self::Snow => VANILLA_FOX_VARIANT_SNOW_ID,
        }
    }

    pub(crate) const fn serialized_name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Snow => "snow",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "red" => Self::Red,
            "snow" => Self::Snow,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoxRetainedState {
    variant: FoxVariant,
    flags: i8,
    trusted: [Option<Uuid>; 2],
    ticks_since_eaten: i32,
    interested_angle: f32,
    interested_angle_o: f32,
    crouch_amount: f32,
    crouch_amount_o: f32,
}

impl FoxRetainedState {
    pub(crate) const fn variant(&self) -> FoxVariant {
        self.variant
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        let persisted = FLAG_SITTING | FLAG_CROUCHING | FLAG_SLEEPING;
        self.flags = (self.flags & persisted) | (previous.flags & !persisted);
        self.ticks_since_eaten = previous.ticks_since_eaten;
        self.interested_angle = previous.interested_angle;
        self.interested_angle_o = previous.interested_angle_o;
        self.crouch_amount = previous.crouch_amount;
        self.crouch_amount_o = previous.crouch_amount_o;
    }

    fn from_saved(
        variant: FoxVariant,
        trusted: [Option<Uuid>; 2],
        sleeping: bool,
        sitting: bool,
        crouching: bool,
    ) -> Self {
        let mut state = Self {
            variant,
            trusted,
            ..Self::default()
        };
        state.set_flag(FLAG_SLEEPING, sleeping);
        state.set_flag(FLAG_SITTING, sitting);
        state.set_flag(FLAG_CROUCHING, crouching);
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

    pub(crate) fn trusted_iter(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.trusted.iter().flatten().copied()
    }

    pub(crate) fn trusts_uuid(&self, uuid: Uuid) -> bool {
        self.trusted.contains(&Some(uuid))
    }

    pub(crate) fn blocks_melee(&self) -> bool {
        self.has_flag(FLAG_SITTING | FLAG_SLEEPING | FLAG_CROUCHING | FLAG_FACEPLANTED)
    }

    pub(crate) fn is_sleeping(&self) -> bool {
        self.has_flag(FLAG_SLEEPING)
    }

    pub(crate) fn is_crouching(&self) -> bool {
        self.has_flag(FLAG_CROUCHING)
    }

    pub(crate) fn is_interested(&self) -> bool {
        self.has_flag(FLAG_INTERESTED)
    }

    pub(crate) fn is_fully_crouched(&self) -> bool {
        self.crouch_amount == 5.0
    }

    pub(crate) fn is_pouncing(&self) -> bool {
        self.has_flag(FLAG_POUNCING)
    }

    pub(crate) fn is_faceplanted(&self) -> bool {
        self.has_flag(FLAG_FACEPLANTED)
    }

    pub(crate) fn resets_pitch(&self) -> bool {
        !self.has_flag(FLAG_POUNCING | FLAG_CROUCHING | FLAG_INTERESTED | FLAG_FACEPLANTED)
    }

    pub(crate) fn is_sitting(&self) -> bool {
        self.has_flag(FLAG_SITTING)
    }

    pub(crate) fn can_move(&self) -> bool {
        !self.has_flag(FLAG_SLEEPING | FLAG_SITTING | FLAG_FACEPLANTED)
    }

    pub(crate) fn is_defending(&self) -> bool {
        self.has_flag(VANILLA_FOX_FLAG_DEFENDING)
    }

    fn before_ai_step(&mut self, no_ai: bool, live_target: bool) -> (bool, bool) {
        let mut dirty = false;
        if !no_ai {
            self.ticks_since_eaten = self.ticks_since_eaten.wrapping_add(1);
            if !live_target {
                dirty |= self.set_flag(FLAG_CROUCHING, false);
                dirty |= self.set_flag(FLAG_INTERESTED, false);
            }
        }
        (self.has_flag(FLAG_SLEEPING), dirty)
    }

    fn after_super(
        &mut self,
        no_ai: bool,
        water: FluidContact,
        has_target: bool,
        thundering: bool,
    ) -> bool {
        let mut dirty = false;
        if !no_ai {
            if has_target || thundering {
                dirty |= self.set_flag(FLAG_SLEEPING, false);
            }
            match water {
                FluidContact::Known(true) => {
                    dirty |= self.set_flag(FLAG_SLEEPING, false);
                    dirty |= self.set_flag(FLAG_SITTING, false);
                }
                FluidContact::Known(false) if self.has_flag(FLAG_SLEEPING) => {
                    dirty |= self.set_flag(FLAG_SITTING, false);
                }
                FluidContact::Known(false) | FluidContact::Unknown => {}
            }
        }

        self.interested_angle_o = self.interested_angle;
        let interested = if self.has_flag(FLAG_INTERESTED) {
            1.0
        } else {
            0.0
        };
        self.interested_angle += (interested - self.interested_angle) * 0.4;
        self.crouch_amount_o = self.crouch_amount;
        if self.has_flag(FLAG_CROUCHING) {
            self.crouch_amount = (self.crouch_amount + 0.2).min(5.0);
        } else {
            self.crouch_amount = 0.0;
        }
        dirty
    }

    #[cfg(test)]
    fn set_test_flags(&mut self, sleeping: bool, sitting: bool, crouching: bool, interested: bool) {
        self.set_flag(FLAG_SLEEPING, sleeping);
        self.set_flag(FLAG_SITTING, sitting);
        self.set_flag(FLAG_CROUCHING, crouching);
        self.set_flag(FLAG_INTERESTED, interested);
    }
}

impl Default for FoxRetainedState {
    fn default() -> Self {
        Self {
            variant: FoxVariant::Red,
            flags: 0,
            trusted: [None; 2],
            ticks_since_eaten: 0,
            interested_angle: 0.0,
            interested_angle_o: 0.0,
            crouch_amount: 0.0,
            crouch_amount_o: 0.0,
        }
    }
}

pub(crate) fn set_spawn_variant(entity: &mut Entity, snow: bool) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.fox_retained_mut() else {
        return false;
    };
    let variant = if snow {
        FoxVariant::Snow
    } else {
        FoxVariant::Red
    };
    if state.variant != variant {
        state.variant = variant;
        entity.mark_synched_dirty(VANILLA_FOX_TYPE_DATA_ID);
    }
    true
}

pub(crate) fn before_ai_step(entity: &mut Entity, live_target: bool) {
    let no_ai = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
    let (sleeping, dirty) = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.fox_retained_mut() else {
                return;
            };
            state.before_ai_step(no_ai, live_target)
        }
        _ => return,
    };
    if sleeping {
        let living = entity.living_mut().expect("Fox is living");
        living.jumping = false;
        living.xxa = 0.0;
        living.zza = 0.0;
    }
    if dirty {
        entity.mark_synched_dirty(VANILLA_FOX_FLAGS_DATA_ID);
    }
}

pub(crate) fn after_super(
    entity: &mut Entity,
    water: FluidContact,
    has_target: bool,
    thundering: bool,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_FOX_ID {
        return;
    }
    let no_ai = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
    let dirty = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .fox_retained_mut()
            .is_some_and(|state| state.after_super(no_ai, water, has_target, thundering)),
        _ => false,
    };
    if dirty {
        entity.mark_synched_dirty(VANILLA_FOX_FLAGS_DATA_ID);
    }
}

pub(crate) fn tick_faceplant_effect(
    entity: &Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let no_ai = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
    // Fox.java:560-563.
    if !no_ai
        && matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.fox_retained().is_some_and(FoxRetainedState::is_faceplanted))
        && store.random().next_float() < 0.2
    {
        let pos = entity.block_position();
        level.level_event(2001, pos, level.block_state(pos));
    }
}

pub(crate) fn ticks_since_eaten(entity: &Entity) -> Option<i32> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.fox_retained().map(|state| state.ticks_since_eaten),
        _ => None,
    }
}

pub(crate) fn reset_ticks_since_eaten(entity: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.fox_retained_mut() {
            state.ticks_since_eaten = 0;
        }
    }
}

pub(crate) fn apply_hunt_action(entity: &mut Entity, action: FoxHuntAction) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.fox_retained_mut() else {
        return;
    };
    let mut dirty = false;
    match action {
        FoxHuntAction::Sit | FoxHuntAction::Stand => {
            dirty |= state.set_flag(FLAG_SITTING, action == FoxHuntAction::Sit);
        }
        FoxHuntAction::Stalk => {
            dirty |= state.set_flag(FLAG_SITTING | FLAG_FACEPLANTED, false);
        }
        FoxHuntAction::Charge | FoxHuntAction::ClearCharge => {
            dirty |= state.set_flag(
                FLAG_CROUCHING | FLAG_INTERESTED,
                action == FoxHuntAction::Charge,
            );
        }
        FoxHuntAction::Pounce => {
            dirty |= state.set_flag(FLAG_POUNCING, true);
            dirty |= state.set_flag(FLAG_INTERESTED, false);
        }
        FoxHuntAction::StopPounce => {
            dirty |= state.set_flag(FLAG_CROUCHING | FLAG_INTERESTED | FLAG_POUNCING, false);
            state.crouch_amount = 0.0;
            state.crouch_amount_o = 0.0;
        }
        FoxHuntAction::Faceplant | FoxHuntAction::Recover => {
            dirty |= state.set_flag(FLAG_FACEPLANTED, action == FoxHuntAction::Faceplant);
        }
    }
    if dirty {
        entity.mark_synched_dirty(VANILLA_FOX_FLAGS_DATA_ID);
    }
}

pub(crate) fn set_defending(entity: &mut Entity, defending: bool) {
    set_flag_published(entity, VANILLA_FOX_FLAG_DEFENDING, defending);
}

// Fox.java:1419-1433.
pub(crate) fn set_sleeping(entity: &mut Entity, sleeping: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.fox_retained_mut() else {
        return;
    };
    let clear = FLAG_SITTING | FLAG_CROUCHING | FLAG_INTERESTED;
    let clear = if sleeping {
        clear
    } else {
        clear | VANILLA_FOX_FLAG_DEFENDING | FLAG_FACEPLANTED
    };
    let next = (state.flags & !clear & !FLAG_SLEEPING) | if sleeping { FLAG_SLEEPING } else { 0 };
    if state.flags != next {
        state.flags = next;
        entity.mark_synched_dirty(VANILLA_FOX_FLAGS_DATA_ID);
    }
    if sleeping {
        entity.living_mut().expect("Fox is living").jumping = false;
    }
}

pub(crate) fn wake_up(entity: &mut Entity) {
    set_flag_published(entity, FLAG_SLEEPING, false);
}

fn set_flag_published(entity: &mut Entity, flag: i8, value: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let changed = data
        .fox_retained_mut()
        .is_some_and(|state| state.set_flag(flag, value));
    if changed {
        entity.mark_synched_dirty(VANILLA_FOX_FLAGS_DATA_ID);
    }
}

pub(crate) fn after_ai_step(entity: &Entity, store: &mut crate::entity::store::EntityStore) {
    if matches!(entity.data(), EntityData::GeneralLiving(data)
        if !data.is_no_ai()
            && data.fox_retained().is_some_and(|fox| !fox.is_sleeping() && fox.ticks_since_eaten > 560))
        && entity.on_ground()
        && !store.mob_ai(entity.id()).is_some_and(|ai| ai.has_target())
        && entity
            .living()
            .and_then(|living| {
                living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            })
            .is_some_and(|stack| {
                crate::entity::item_components::is_consumable_food(stack) == Ok(true)
            })
    {
        store.queue_command(crate::entity::EntityCommand::FoxMouthFood {
            entity_id: entity.id(),
            uuid: entity.uuid(),
        });
    }
    let defending = matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.fox_retained().is_some_and(FoxRetainedState::is_defending));
    if !defending {
        return;
    }
    if store.random().next_float() < 0.05 {
        crate::entity::ai::mob::target::play_target_sound(
            entity,
            store,
            crate::entity::ai::mob::target::TargetSound {
                resource_id: FOX_AGGRO_SOUND,
                volume: 1.0,
                pitch: 1.0,
            },
            entity.position(),
        );
    }
}

pub(crate) const FOX_AGGRO_SOUND: &str = "minecraft:entity.fox.aggro";

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &FoxRetainedState) {
    entries.extend([
        NbtSummaryEntry {
            name: "Trusted".into(),
            value: NbtSummaryValue::List(
                state
                    .trusted
                    .iter()
                    .flatten()
                    .copied()
                    .map(uuid_value)
                    .collect(),
            ),
        },
        boolean_entry("Sleeping", state.has_flag(FLAG_SLEEPING)),
        NbtSummaryEntry {
            name: "Type".into(),
            value: NbtSummaryValue::string(state.variant.serialized_name()),
        },
        boolean_entry("Sitting", state.has_flag(FLAG_SITTING)),
        boolean_entry("Crouching", state.has_flag(FLAG_CROUCHING)),
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> FoxRetainedState {
    let variant = match optional_field(entries, "Type") {
        Some(NbtSummaryValue::String(name)) => name
            .with_utf8(FoxVariant::parse)
            .flatten()
            .unwrap_or(FoxVariant::Red),
        _ => FoxVariant::Red,
    };
    FoxRetainedState::from_saved(
        variant,
        read_trusted(entries),
        optional_bool(entries, "Sleeping").unwrap_or(false),
        optional_bool(entries, "Sitting").unwrap_or(false),
        optional_bool(entries, "Crouching").unwrap_or(false),
    )
}

pub(crate) fn synched_values(state: &FoxRetainedState) -> [SynchedValue; 4] {
    [
        synched_value(
            VANILLA_FOX_TYPE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(state.variant.id()),
            state.variant == FoxVariant::Red,
        ),
        synched_value(
            VANILLA_FOX_FLAGS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(state.flags),
            state.flags == 0,
        ),
        trusted_value(VANILLA_FOX_TRUSTED_DATA_ID_0, state.trusted[0]),
        trusted_value(VANILLA_FOX_TRUSTED_DATA_ID_1, state.trusted[1]),
    ]
}

fn read_trusted(entries: &[NbtSummaryEntry]) -> [Option<Uuid>; 2] {
    let mut trusted = [None; 2];
    let Some(NbtSummaryValue::List(values)) = optional_field(entries, "Trusted") else {
        return trusted;
    };
    let uuids = values
        .iter()
        .filter_map(crate::entity::save::read_uuid_codec_input);
    for (count, uuid) in uuids.enumerate() {
        trusted[count.min(1)] = Some(uuid);
    }
    trusted
}

fn boolean_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

fn trusted_value(data_id: u8, value: Option<Uuid>) -> SynchedValue {
    synched_value(
        data_id,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
        EntityDataValueKind::OptionalLivingEntityReference(value),
        value.is_none(),
    )
}

fn synched_value(
    data_id: u8,
    serializer_id: i32,
    value: EntityDataValueKind,
    is_default: bool,
) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id,
            value,
        },
        is_default,
    }
}

#[cfg(test)]
mod tests;
