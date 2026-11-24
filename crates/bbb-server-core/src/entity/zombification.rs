//! Environment driven conversion of piglins and hoglins.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::SoundSource;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::living::ActiveMobEffect;
use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityData, EntityStore, GeneralLivingData};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ZombificationState {
    pub(crate) immune: bool,
    time: i32,
}

impl ZombificationState {
    // Hoglin.java:283-287; AbstractPiglin.java:98-101.
    pub(crate) fn is_converting(&self, no_ai: bool, piglins_zombify: bool) -> bool {
        !self.immune && !no_ai && piglins_zombify
    }

    pub(crate) fn read_save(entries: &[NbtSummaryEntry]) -> Self {
        Self {
            immune: crate::entity::save::optional_bool(entries, "IsImmuneToZombification")
                .unwrap_or(false),
            time: crate::entity::save::optional_int(entries, "TimeInOverworld").unwrap_or(0),
        }
    }

    pub(crate) fn write_save(self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.extend([
            NbtSummaryEntry {
                name: "IsImmuneToZombification".into(),
                value: NbtSummaryValue::Byte(i8::from(self.immune)),
            },
            NbtSummaryEntry {
                name: "TimeInOverworld".into(),
                value: NbtSummaryValue::Int(self.time),
            },
        ]);
    }

    pub(crate) fn owns_save_field(name: &str) -> bool {
        matches!(name, "IsImmuneToZombification" | "TimeInOverworld")
    }
}

pub(crate) fn tick(
    source: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let (target_type, preserve_pickup, sound) = match source.entity_type_id() {
        VANILLA_ENTITY_TYPE_HOGLIN_ID => (
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            false,
            "minecraft:entity.hoglin.converted_to_zombified",
        ),
        VANILLA_ENTITY_TYPE_PIGLIN_ID => (
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            true,
            "minecraft:entity.piglin.converted_to_zombified",
        ),
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => (
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            true,
            "minecraft:entity.piglin_brute.converted_to_zombified",
        ),
        _ => return true,
    };
    let Some(zombifies) = level.piglins_zombify(source.position()) else {
        return false;
    };
    let EntityData::GeneralLiving(data) = &mut source.data else {
        return true;
    };
    let no_ai = data.is_no_ai();
    let state = data.zombification_mut().expect("zombifying mob state");
    // AbstractPiglin.java:79-91; Hoglin.java:135-149.
    state.time = if state.is_converting(no_ai, zombifies) {
        state.time.wrapping_add(1)
    } else {
        0
    };
    if state.time <= 300 {
        return true;
    }
    crate::entity::piglin::prepare_zombification(source, store);
    let snapshot = SingleMobConversionSnapshot::capture_with(source, true, preserve_pickup);
    let data = GeneralLivingData::new(target_type).expect("zombification target is living");
    let uuid = store.next_uuid();
    let Some(target_id) = store.restore_initialized(
        uuid,
        target_type,
        snapshot.position(),
        EntityData::GeneralLiving(data),
        |target| {
            snapshot.apply(target);
            target
                .living_mut()
                .unwrap()
                .add_active_effect(ActiveMobEffect::new("minecraft:nausea", 200, 0).unwrap());
        },
    ) else {
        return false;
    };
    source.living_mut().unwrap().equipment_mut().clear();
    crate::entity::conversion::transfer_single_relations_from(source, store, target_id);
    if !source.is_silent() {
        let baby = matches!(source.data(), EntityData::GeneralLiving(data) if data.is_baby());
        let pitch = store
            .mob_ai_mut(source.id())
            .map_or(1.0, |runtime| runtime.voice_pitch(baby));
        store.play_entity_sound(PendingEntitySound {
            position: source.position(),
            resource_id: sound.into(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
    source.discard();
    true
}
