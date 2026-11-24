//! Guardian state and behavior.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{
    VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID, VANILLA_GUARDIAN_MOVING_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID;

use crate::entity::ai::mob::brain_runtime::hurt_entity_from_body;
use crate::entity::ai::mob::do_hurt_target;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::{PendingHurtEntity, PendingPlayerHurt};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuardianState {
    pub(crate) attack_target: i32,
    pub(crate) moving: bool,
}

impl GuardianState {
    pub(crate) const fn new() -> Self {
        Self {
            attack_target: 0,
            moving: false,
        }
    }
}

pub(crate) fn set_attack_target(entity: &mut Entity, target_id: i32) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.guardian_state_mut() else {
        return;
    };
    if state.attack_target != target_id {
        state.attack_target = target_id;
        entity.mark_synched_dirty(VANILLA_GUARDIAN_ATTACK_TARGET_DATA_ID);
    }
}

// Guardian.java:405-417.
pub(crate) fn attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
) -> bool {
    let amount =
        1.0 + if store.difficulty() == Some(Difficulty::Hard) {
            2.0
        } else {
            0.0
        } + if entity.entity_type_id() == VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID {
            2.0
        } else {
            0.0
        };
    if let Some(player) = level.player_by_entity_id(target_id) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.hurt_player(PendingPlayerHurt {
            player: player.uuid,
            damage_type: "minecraft:indirect_magic",
            amount,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: false,
            impulse: None,
        });
    } else if hurt_entity_from_body(
        entity,
        store,
        level,
        target_id,
        "minecraft:indirect_magic",
        amount,
    )
    .is_none()
    {
        return false;
    }
    do_hurt_target(entity, store, level, target_id)
}

pub(crate) fn set_moving(entity: &mut Entity, moving: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.guardian_state_mut() else {
        return;
    };
    if state.moving != moving {
        state.moving = moving;
        entity.mark_synched_dirty(VANILLA_GUARDIAN_MOVING_DATA_ID);
    }
}

// ElderGuardian.java:74-76.
pub(crate) fn custom_server_ai_step(entity: &mut Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID {
        return;
    }
    // ElderGuardian.java:66-71.
    if entity.tick_count() % 1200 == 0 {
        store.queue_command(crate::entity::EntityCommand::MobSpell(
            crate::entity::ai::mob::ranged::PendingMobSpell::ElderGuardianPulse {
                source: entity.uuid(),
                position: entity.position(),
                silent: entity.is_silent(),
            },
        ));
    }
    if entity.mob_home().is_none() {
        entity.set_mob_home(entity.block_position(), 16);
    }
}

// Guardian.java:260-272.
pub(crate) fn ai_step(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.guardian_state(),
        _ => None,
    };
    let Some(state) = state else {
        return;
    };
    if entity.is_in_water() {
        entity.set_air_supply(300);
    } else if entity.on_ground() {
        let random = level.level_random();
        let movement = entity.delta_movement();
        entity.set_delta_movement(bbb_protocol::packets::Vec3d {
            x: movement.x + f64::from((random.next_float() * 2.0 - 1.0) * 0.4),
            y: movement.y + 0.5,
            z: movement.z + f64::from((random.next_float() * 2.0 - 1.0) * 0.4),
        });
        entity.set_rot(random.next_float() * 360.0, entity.x_rot());
        entity.on_ground = false;
        entity.set_needs_sync(true);
    }
    if state.attack_target != 0 {
        if let Some(living) = entity.living() {
            entity.set_rot(living.y_head_rot, entity.x_rot());
        }
    }
}
