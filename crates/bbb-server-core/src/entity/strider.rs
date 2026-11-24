use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_fluid_state;

use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::environment::{fluid_contact, FluidContact, FluidTag};
use crate::entity::level::{block_state_tag_membership, fluid_state_tag_membership};
use crate::entity::{Entity, EntityData};

const SUFFOCATING_MODIFIER_ID: &str = "minecraft:suffocating";

// Strider.java:340-342.
pub(super) fn ambient_sound(runtime: &super::ai::mob::MobAiRuntime) -> Option<&'static str> {
    (!runtime.is_panicking() && !runtime.tempt_goal_running())
        .then_some("minecraft:entity.strider.ambient")
}

// Strider.java:288-293.
pub(crate) fn tick_sounds(entity: &Entity, store: &mut super::EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_STRIDER_ID {
        return;
    }
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    let (tempted, panicking) = (runtime.tempt_goal_running(), runtime.is_panicking());
    let sound = if tempted && store.random().next_int(140) == 0 {
        "minecraft:entity.strider.happy"
    } else if panicking && store.random().next_int(60) == 0 {
        "minecraft:entity.strider.retreat"
    } else {
        return;
    };
    if entity.is_silent() {
        return;
    }
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = super::living::voice_pitch(store, baby);
    store.play_entity_sound(super::store::PendingEntitySound {
        position: entity.position(),
        resource_id: sound.to_owned(),
        source: super::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StriderState {
    pub(crate) steering: crate::entity::item_steering::ItemSteering,
    suffocating: bool,
}

impl StriderState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        *self = *previous;
    }

    pub(crate) const fn new() -> Self {
        Self {
            steering: crate::entity::item_steering::ItemSteering::new(),
            suffocating: false,
        }
    }

    pub(crate) const fn boost_time(self) -> i32 {
        self.steering.boost_time_total()
    }

    pub(crate) const fn suffocating(self) -> bool {
        self.suffocating
    }

    fn set_suffocating(&mut self, suffocating: bool) -> bool {
        let changed = self.suffocating != suffocating;
        self.suffocating = suffocating;
        changed
    }

    #[cfg(test)]
    pub(crate) fn set_suffocating_for_test(&mut self, suffocating: bool) {
        self.suffocating = suffocating;
    }
}

fn is_warm(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<bool> {
    for pos in [entity.block_position(), entity.on_pos_legacy()] {
        if block_state_tag_membership(
            level,
            level.block_state(pos),
            "minecraft:strider_warm_blocks",
        ) {
            return Some(true);
        }
    }
    fluid_contact(entity, level, FluidTag::Lava).known()
}

pub(crate) fn tick_environment(entity: &mut Entity, level: &LevelCtx<'_, '_>) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return true;
    };
    if data.strider_state().is_none() || data.is_no_ai() {
        return true;
    }
    let Some(warm) = is_warm(entity, level) else {
        return false;
    };
    if entity.living().is_none_or(|living| {
        !living
            .attributes()
            .has_attribute(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    }) {
        return true;
    }

    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .strider_state_mut()
            .expect("classified Strider state")
            .set_suffocating(!warm),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_STRIDER_SUFFOCATING_DATA_ID);
    }
    let instance = entity
        .living_mut()
        .expect("classified Strider living state")
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .expect("Strider declares movement speed");
    if warm {
        instance.remove_modifier(SUFFOCATING_MODIFIER_ID);
    } else {
        instance.add_or_update_transient_modifier(AttributeModifier {
            id: SUFFOCATING_MODIFIER_ID.to_string(),
            amount: f64::from(-0.34_f32),
            operation: AttributeOperation::AddMultipliedBase,
        });
    }
    true
}

pub(crate) fn float_strider(entity: &mut Entity, level: &LevelCtx<'_, '_>) -> bool {
    if !matches!(entity.data(), EntityData::GeneralLiving(data) if data.strider_state().is_some()) {
        return true;
    }
    match fluid_contact(entity, level, FluidTag::Lava) {
        FluidContact::Known(true) => {}
        FluidContact::Known(false) => return true,
        FluidContact::Unknown => return false,
    }
    let block_pos = entity.block_position();
    let above_surface = entity.position().y > f64::from(block_pos.y) + 0.5 - f64::from(1.0e-5_f32);
    let surface = if above_surface {
        let above_id = level.block_state(BlockPos {
            y: block_pos.y + 1,
            ..block_pos
        });
        if vanilla_block_state_block_type_id(above_id).is_none() {
            return false;
        }
        !fluid_state_tag_membership(level, block_state_fluid_state(above_id), "minecraft:lava")
    } else {
        false
    };
    if surface {
        entity.on_ground = true;
    } else {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            x: movement.x * 0.5,
            y: movement.y * 0.5 + 0.05,
            z: movement.z * 0.5,
        });
    }
    true
}

#[cfg(test)]
mod tests;
