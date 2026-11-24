use bbb_protocol::entity_synched_data::{
    VANILLA_ARMADILLO_STATE_DATA_ID, VANILLA_ARMADILLO_STATE_IDLE_ID,
    VANILLA_ARMADILLO_STATE_ROLLING_ID, VANILLA_ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;
use bbb_protocol::packets::{
    EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind, ItemStackSummary, SoundSource,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::living::SynchedValue;
use crate::entity::shearing_loot::ShearingLoot;
use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) mod scare;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmadilloPose {
    Idle,
    Rolling,
    Scared,
    Unrolling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArmadilloState {
    pub(crate) pose: ArmadilloPose,
    pub(crate) scute_time: i32,
    pub(crate) state_since: i32,
    pub(crate) scan_delay: u8,
    pub(crate) next_peek: u16,
    pub(crate) danger_was_around: bool,
}

impl ArmadilloState {
    pub(crate) fn new(scute_time: i32) -> Self {
        Self {
            pose: ArmadilloPose::Idle,
            scute_time,
            state_since: 0,
            scan_delay: 0,
            next_peek: 0,
            danger_was_around: false,
        }
    }

    /// `Armadillo.readAdditionalSaveData` (`Armadillo.java:257-260`).
    pub(crate) fn read_save(&mut self, entries: &[NbtSummaryEntry]) {
        self.pose = match crate::entity::save::optional_field(entries, "state") {
            Some(NbtSummaryValue::String(value)) => match value {
                value if value == "rolling" => ArmadilloPose::Rolling,
                value if value == "scared" => ArmadilloPose::Scared,
                value if value == "unrolling" => ArmadilloPose::Unrolling,
                _ => ArmadilloPose::Idle,
            },
            _ => ArmadilloPose::Idle,
        };
        if let Some(time) = crate::entity::save::optional_int(entries, "scute_time") {
            self.scute_time = time;
        }
    }

    /// `Armadillo.addAdditionalSaveData` (`Armadillo.java:250-253`).
    pub(crate) fn write_save(&self, entries: &mut Vec<NbtSummaryEntry>) {
        let name = match self.pose {
            ArmadilloPose::Idle => "idle",
            ArmadilloPose::Rolling => "rolling",
            ArmadilloPose::Scared => "scared",
            ArmadilloPose::Unrolling => "unrolling",
        };
        entries.extend([
            NbtSummaryEntry {
                name: "state".into(),
                value: NbtSummaryValue::string(name),
            },
            NbtSummaryEntry {
                name: "scute_time".into(),
                value: NbtSummaryValue::Int(self.scute_time),
            },
        ]);
    }

    pub(crate) fn synched_value(&self) -> SynchedValue {
        let id = match self.pose {
            ArmadilloPose::Idle => VANILLA_ARMADILLO_STATE_IDLE_ID,
            ArmadilloPose::Rolling => VANILLA_ARMADILLO_STATE_ROLLING_ID,
            ArmadilloPose::Scared => VANILLA_ARMADILLO_STATE_SCARED_ID,
            ArmadilloPose::Unrolling => VANILLA_ARMADILLO_STATE_UNROLLING_ID,
        };
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ARMADILLO_STATE_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID,
                value: EntityDataValueKind::EnumId {
                    serializer: EntityDataEnumSerializer::ArmadilloState,
                    id,
                },
            },
            is_default: self.pose == ArmadilloPose::Idle,
        }
    }
}

/// `Armadillo.isScared` (`Armadillo.java:97-99`).
pub(crate) fn is_scared(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.armadillo_state().is_some_and(|state| state.pose != ArmadilloPose::Idle))
}

/// `Armadillo.mobInteract` (`Armadillo.java:306-310`).
pub(crate) const BRUSH_DAMAGE: i32 = 16;

/// `Armadillo.brushOffScute` (`Armadillo.java:316-328`).
pub(crate) fn brush(
    store: &mut EntityStore,
    target_id: i32,
    hand: &ItemStackSummary,
    loot: Option<&mut ShearingLoot<'_>>,
) -> bool {
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_BRUSH_ID) || hand.count <= 0 {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    if target.entity_type_id() != VANILLA_ENTITY_TYPE_ARMADILLO_ID {
        return false;
    }
    let EntityData::GeneralLiving(data) = target.data() else {
        return false;
    };
    if data
        .ageable_retained()
        .is_some_and(|ageable| ageable.is_baby())
    {
        return false;
    }
    let Some(loot) = loot else {
        return false;
    };
    let position = target.position();
    let silent = target.is_silent();
    let drops = crate::entity::shearing_loot::draw_entity_interact_loot(
        "minecraft:brush/armadillo",
        store,
        target,
        hand,
        loot,
    )
    .unwrap_or_default();
    for stack in drops {
        crate::entity::spawn_dropped_item_entity(store, position, stack);
    }
    if !silent {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position,
            resource_id: "minecraft:entity.armadillo.brush".into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    store.queue_command(crate::entity::EntityCommand::GameEvent {
        kind: crate::level_event::VanillaGameEvent::ENTITY_INTERACT,
        position,
        source_entity_id: Some(target_id),
    });
    true
}
