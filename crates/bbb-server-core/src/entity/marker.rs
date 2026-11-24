//! `net/minecraft/world/entity/Marker.java`: an invisible point that never
//! ticks, is never sent to a client and cannot be hurt. Its only payload is
//! `Entity.customData`, the `data` compound of `Entity.saveWithoutId`
//! (`Entity.java:2021-2023`), which is kept here verbatim because no other
//! stored type reads it.
//!
//! `noPhysics` (`Marker.java:17`), `getPistonPushReaction` = `IGNORE`
//! (`:56-59`) and `isIgnoringBlockTriggers` (`:61-64`) have no consumer in
//! this crate: a marker never moves, and pistons and block triggers do not
//! reach stored entities here.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{parse_snbt_compound_utf16, NbtSummaryEntry, NbtSummaryValue};

use crate::entity::display::save_fields::{entry, optional_field};
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `Entity.isPickable()` (`Entity.java:1923-1925`), not overridden.
pub const IS_PICKABLE: bool = false;
/// `Entity.canBeHitByProjectile()` (`Entity.java:1919-1921`) over
/// [`IS_PICKABLE`].
pub const CAN_BE_HIT_BY_PROJECTILE: bool = false;

/// `Entity.customData` (`Entity.java:301`) as the marker carries it: the
/// entries of the `data` compound, empty for `CustomData.EMPTY`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarkerData {
    data: Vec<NbtSummaryEntry>,
}

impl MarkerData {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn with_data(data: Vec<NbtSummaryEntry>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &[NbtSummaryEntry] {
        &self.data
    }
}

pub fn marker(entity: &Entity) -> Option<&MarkerData> {
    match entity.data() {
        EntityData::Marker(data) => Some(data),
        _ => None,
    }
}

/// `new Marker(EntityType.MARKER, level)` (`Marker.java:15-18`) added to the
/// level: the UUID draw of the `Entity` constructor (`Entity.java:278`) and
/// nothing else.
pub fn spawn_marker(store: &mut EntityStore, position: Vec3d, data: MarkerData) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_MARKER_ID,
        position,
        EntityData::Marker(data),
    )
}

/// `Marker.tick()` (`Marker.java:21-22`): empty, so neither `baseTick` nor
/// `checkBelowWorld` runs and `firstTick` is never cleared.
pub fn tick(_entity: &mut Entity) {}

/// `Marker.defineSynchedData` (`Marker.java:25-26`) adds nothing to the
/// `Entity` rows.
pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    crate::entity::living::entity_synched_values(entity)
}

/// `Entity.saveWithoutId`'s `data` field (`Entity.java:2021-2023`): written
/// only when the compound is non-empty. `Marker.addAdditionalSaveData`
/// (`Marker.java:33-34`) adds nothing.
pub fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, data: &MarkerData) {
    if !data.data.is_empty() {
        entries.push(entry("data", NbtSummaryValue::Compound(data.data.clone())));
    }
}

/// `Entity.load`'s `data` read (`Entity.java:2088`) over `CustomData.CODEC`
/// (`CustomData.java:16-17`): a compound verbatim, or the flattened SNBT
/// string alternative parsed into one. A decode failure leaves the default
/// (`orElse(CustomData.EMPTY)`). `Marker.readAdditionalSaveData`
/// (`Marker.java:29-30`) adds nothing.
pub fn read_save_data(entries: &[NbtSummaryEntry]) -> MarkerData {
    let data = match optional_field(entries, "data") {
        None => Vec::new(),
        Some(NbtSummaryValue::Compound(data)) => data.clone(),
        Some(NbtSummaryValue::String(snbt)) => match parse_snbt_compound_utf16(snbt.units()) {
            Ok(NbtSummaryValue::Compound(data)) => data,
            _ => Vec::new(),
        },
        Some(_) => Vec::new(),
    };
    MarkerData { data }
}
