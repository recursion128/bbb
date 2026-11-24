//! `BoneMealItem.useOn` (`BoneMealItem.java:35-62`).

use super::ItemUseOnContext;
use crate::block_behavior::bonemeal::{grow_crop, grow_water_plant};
use crate::level_event::{PendingLevelEvent, VanillaGameEvent};
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::block_state_types::VanillaBlockSupportType;
use bbb_server_types::block_queries::face_sturdy_at;
use bbb_server_types::directions::relative;

/// `LevelEvent.PARTICLES_AND_SOUND_PLANT_GROWTH`.
const PLANT_GROWTH: i32 = 1505;

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let clicked = context.pos();
    let face = context.update.hit.direction;
    let Some((pos, server_swing)) = world.with_level(context.dimension, |level| {
        if grow_crop(level, clicked) {
            return Some((clicked, true));
        }
        let adjacent = relative(clicked, face);
        (face_sturdy_at(
            level.block_state(clicked),
            clicked,
            face,
            VanillaBlockSupportType::Full,
        ) && grow_water_plant(level, adjacent, Some(face)))
        .then_some((adjacent, false))
    }) else {
        return false;
    };
    world.shrink_used_stack(context);
    world.cause_use_vibration(
        context.connection_id,
        context.stack,
        VanillaGameEvent::ITEM_INTERACT_FINISH,
    );
    world.broadcast_level_event_by_key(
        &context.dimension,
        None,
        PendingLevelEvent {
            event_type: PLANT_GROWTH,
            pos,
            data: 15,
        },
        events,
    );
    // `SUCCESS_SERVER`: the client passed, so the server swings for it.
    if server_swing {
        world.mark_use_item_on_server_swing();
    }
    true
}
