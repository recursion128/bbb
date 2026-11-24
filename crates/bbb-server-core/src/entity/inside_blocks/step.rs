use super::{Entity, LivingInsideBlockCallback};
use crate::block_behavior::ground_contact::{self, GroundBody, StepOn};
use crate::entity::level::EntityBlockContext;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
};

pub(super) fn apply<F>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    mut store: Option<&mut crate::entity::EntityStore>,
    callback: &mut F,
) -> Option<()>
where
    F: FnMut(
        &mut Entity,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::EntityStore>,
        LivingInsideBlockCallback,
    ) -> bool,
{
    let pos = entity.on_pos_legacy_in(level);
    let state = level.block_state(pos);
    let body = ground_body(entity, level);
    let request = match ground_contact::step_on_with_registry(
        state,
        body,
        entity.delta_movement(),
        level.level_random(),
        level.content_registry(),
    )? {
        StepOn::None => return Some(()),
        StepOn::Movement(movement) => {
            entity.set_delta_movement(movement);
            return Some(());
        }
        StepOn::DecreaseTurtleEggs => {
            return level.decrease_turtle_eggs(
                EntityBlockContext::new(store.as_deref_mut(), Some(entity)),
                state,
                pos,
            )
        }
        StepOn::InteractRedstoneOre => {
            return level.interact_redstone_ore(
                EntityBlockContext::new(store.as_deref_mut(), Some(entity)),
                state,
                pos,
            )
        }
        StepOn::Hurt(damage_type, amount) => LivingInsideBlockCallback::Damage {
            damage_type,
            amount,
        },
        StepOn::SculkSensor => LivingInsideBlockCallback::SculkSensorStep { state, pos },
        StepOn::SculkShrieker => LivingInsideBlockCallback::SculkShriekerStep { pos },
    };
    callback(entity, level, store, request).then_some(())
}

/// `Entity.isSteppingCarefully` (`Entity.java:2615-2617`), which cats and
/// ocelots widen to their crouch (`Cat.java:484-486`, `Ocelot.java:265-267`).
pub(crate) fn is_stepping_carefully(entity: &Entity) -> bool {
    entity.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
        || (entity.is_crouching()
            && matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_CAT_ID | VANILLA_ENTITY_TYPE_OCELOT_ID
            ))
}

pub(crate) fn ground_body(entity: &Entity, level: &LevelCtx<'_, '_>) -> GroundBody {
    GroundBody {
        living: entity.living().is_some(),
        player: entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID,
        zombie: crate::entity::zombie::is_zombie_hierarchy(entity.entity_type_id()),
        turtle_or_bat: matches!(
            entity.entity_type_id(),
            VANILLA_ENTITY_TYPE_TURTLE_ID | VANILLA_ENTITY_TYPE_BAT_ID
        ),
        careful: is_stepping_carefully(entity),
        mob_griefing: Some(level.mob_griefing()),
        width: entity.bb_width(),
        height: entity.bb_height(),
    }
}

#[cfg(test)]
mod tests;
