use super::{BlockPos, SessionContact};
use crate::block_behavior::honey;
use crate::connection::ClientboundPlayPacket;
use bbb_protocol::packets::EntityEvent;
use bbb_server_types::interaction::player_pose_dimensions;

/// `HoneyBlock.entityInside` (`HoneyBlock.java:64-74`) for a session player.
pub(super) fn apply(contact: &mut SessionContact<'_>, pos: BlockPos) -> Option<()> {
    let player = contact.world.levels.player(contact.id)?;
    let [width, _] = player_pose_dimensions(player.pose_id, player.pose_scale()).ok()?;
    let dimension = player.dimension;
    if !honey::is_sliding_down(
        pos,
        player.position(),
        width,
        player.on_ground(),
        player.delta_movement(),
    ) {
        return Some(());
    }
    // HoneyBlock.java:103-106: re-read the state before awarding, not the contact snapshot.
    if contact.world.game_time() % 20 == 0 {
        let state = contact.block_state(pos);
        contact
            .world
            .trigger_slide_down_block(contact.id, state, contact.events);
    }
    let player = contact.world.levels.player_mut(contact.id).unwrap();
    player.set_delta_movement(honey::slide_movement(player.delta_movement()));
    let entity_id = player.entity_id();
    contact
        .world
        .reset_player_fall_distance(contact.id, contact.events);
    let random = &mut contact.world.level_mut_by_id(&dimension).random;
    let (sound, event) = honey::slide_effects(|| random.next_int(5));
    if sound {
        contact.world.play_player_landing_sound(
            contact.id,
            "minecraft:block.honey_block.slide",
            1.0,
            1.0,
            contact.events,
        );
    }
    if event {
        contact.world.send_to_player_tracking_and_self_in_level(
            contact.id,
            entity_id,
            &dimension,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: 53,
            }),
            contact.events,
        );
    }
    Some(())
}

#[cfg(test)]
mod tests;
