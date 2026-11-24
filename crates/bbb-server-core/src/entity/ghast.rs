use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FLYING_SPEED_ID;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_synched_data::VANILLA_GHAST_CHARGING_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::{travel_flying, FlyingContact};
use crate::entity::save::optional_byte;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GhastRetainedState {
    explosion_power: i8,
    charging: bool,
}

impl GhastRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.charging = previous.charging;
    }

    pub(crate) const fn from_saved(explosion_power: i8) -> Self {
        Self {
            explosion_power,
            charging: false,
        }
    }

    pub(crate) const fn explosion_power(self) -> i8 {
        self.explosion_power
    }

    pub(crate) const fn charging(self) -> bool {
        self.charging
    }

    fn set_charging(&mut self, charging: bool) -> bool {
        let changed = self.charging != charging;
        self.charging = charging;
        changed
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: GhastRetainedState) {
    entries.push(NbtSummaryEntry {
        name: "ExplosionPower".into(),
        value: NbtSummaryValue::Byte(state.explosion_power()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> GhastRetainedState {
    GhastRetainedState::from_saved(optional_byte(entries, "ExplosionPower").unwrap_or(1))
}

pub(crate) fn set_charging(entity: &mut Entity, charging: bool) -> bool {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .ghast_retained_mut()
            .is_some_and(|state| state.set_charging(charging)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_GHAST_CHARGING_DATA_ID);
    }
    changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GhastFluidContact {
    Water,
    Lava,
    Dry,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FluidContact {
    Present,
    Absent,
    Unknown,
}

fn fluid_contact(entity: &Entity, level: &LevelCtx<'_, '_>, family: FluidFamily) -> FluidContact {
    let tag_id = match family {
        FluidFamily::Water => "minecraft:water",
        FluidFamily::Lava => "minecraft:lava",
    };
    let bb = entity.bounding_box();
    let min_x = (bb.min_x + 0.001).floor() as i32;
    let min_y = (bb.min_y + 0.001).floor() as i32;
    let min_z = (bb.min_z + 0.001).floor() as i32;
    let max_x = (bb.max_x - 0.001).ceil() as i32 - 1;
    let max_y = (bb.max_y - 0.001).ceil() as i32 - 1;
    let max_z = (bb.max_z - 0.001).ceil() as i32 - 1;
    let mut unknown = false;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let pos = BlockPos { x, y, z };
                let state_id = level.block_state(pos);
                if vanilla_block_state_block_type_id(state_id).is_none() {
                    if level
                        .content_registry()
                        .block_by_state_id(state_id)
                        .is_none()
                    {
                        unknown = true;
                    }
                    continue;
                }
                let fluid = block_state_fluid_state(state_id);
                if !fluid_state_tag_membership(level, fluid, tag_id) {
                    continue;
                }
                let above_id = level.block_state(BlockPos { y: y + 1, ..pos });
                if vanilla_block_state_block_type_id(above_id).is_none()
                    && level
                        .content_registry()
                        .block_by_state_id(above_id)
                        .is_none()
                {
                    unknown = true;
                    continue;
                }
                let height = if block_state_fluid_state(above_id).family() == fluid.family() {
                    1.0
                } else {
                    f64::from(fluid.own_height())
                };
                if f64::from(y) + height >= bb.min_y + 0.001 {
                    return FluidContact::Present;
                }
            }
        }
    }
    if unknown {
        FluidContact::Unknown
    } else {
        FluidContact::Absent
    }
}

pub(crate) fn contact(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<GhastFluidContact> {
    if !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_GHAST_ID | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
    ) {
        return None;
    }
    match fluid_contact(entity, level, FluidFamily::Water) {
        FluidContact::Present => return Some(GhastFluidContact::Water),
        FluidContact::Unknown => return Some(GhastFluidContact::Unknown),
        FluidContact::Absent => {}
    }
    Some(match fluid_contact(entity, level, FluidFamily::Lava) {
        FluidContact::Present => GhastFluidContact::Lava,
        FluidContact::Absent => GhastFluidContact::Dry,
        FluidContact::Unknown => GhastFluidContact::Unknown,
    })
}

pub(crate) fn travel(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    contact: GhastFluidContact,
) -> bool {
    let Some(living) = entity.living() else {
        return true;
    };
    let input = Vec3d {
        x: f64::from(living.xxa),
        y: f64::from(living.yya),
        z: f64::from(living.zza),
    };
    let contact = match contact {
        GhastFluidContact::Water => FlyingContact::Water,
        GhastFluidContact::Lava => FlyingContact::Lava,
        GhastFluidContact::Dry => FlyingContact::Air,
        GhastFluidContact::Unknown => FlyingContact::Unknown,
    };
    let (air_speed, fluid_speed) = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
    {
        // HappyGhast.java:172-175: every medium uses the same float-rounded speed.
        let speed = living.attribute_value(VANILLA_ATTRIBUTE_FLYING_SPEED_ID) as f32 * 5.0 / 3.0;
        (speed, speed)
    } else {
        (0.02, 0.02)
    };
    travel_flying(entity, store, level, input, contact, air_speed, fluid_speed).is_ok()
}

pub(crate) fn tick_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_GHAST_ID {
        return;
    }
    let Some(pitch) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.tick_ghast_ambient_sound(entity.is_alive()))
    else {
        return;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.ghast.ambient".to_string(),
            source: SoundSource::Hostile,
            volume: 5.0,
            pitch,
        });
    }
}

#[cfg(test)]
mod tests;
