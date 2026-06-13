use anyhow::Result;
use bbb_protocol::packets::{
    self, CommandSuggestionRequest, InteractionHand, PickItemFromBlock, PlayerAction,
    PlayerCommand, PlayerHealth, PlayerInput, PlayerPositionState, UseItem, UseItemOn,
};

use crate::{
    connection::RawConnection,
    types::{PlayerMoveCommand, VehicleMoveCommand},
};

pub(super) async fn send_player_move_command(
    conn: &mut RawConnection,
    command: PlayerMoveCommand,
    player_position_state: &mut PlayerPositionState,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await?;
    *player_position_state = command.state;
    Ok(())
}

pub(super) async fn send_vehicle_move_command(
    conn: &mut RawConnection,
    command: VehicleMoveCommand,
) -> Result<()> {
    let (id, payload) = command.encode_packet();
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_action(
    conn: &mut RawConnection,
    action: PlayerAction,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_action(action);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_command(
    conn: &mut RawConnection,
    command: PlayerCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_command(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_player_input_command(
    conn: &mut RawConnection,
    input: PlayerInput,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_input(input);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_held_slot_command(conn: &mut RawConnection, slot: u8) -> Result<()> {
    let (id, payload) = packets::encode_play_set_carried_item(i16::from(slot.min(8)));
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_swing_command(
    conn: &mut RawConnection,
    hand: InteractionHand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_swing(hand);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item_on(conn: &mut RawConnection, packet: UseItemOn) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item_on(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item(conn: &mut RawConnection, packet: UseItem) -> Result<()> {
    let (id, payload) = packets::encode_play_use_item(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_pick_item_from_block(
    conn: &mut RawConnection,
    packet: PickItemFromBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_pick_item_from_block(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_command_suggestion_request(
    conn: &mut RawConnection,
    request: CommandSuggestionRequest,
) -> Result<()> {
    let (id, payload) = packets::encode_play_command_suggestion_request(request);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn maybe_send_perform_respawn(
    conn: &mut RawConnection,
    health: PlayerHealth,
    player_was_dead: &mut bool,
) -> Result<()> {
    let is_dead = health.health <= 0.0;
    if is_dead && !*player_was_dead {
        let (id, payload) = packets::encode_play_perform_respawn();
        conn.send_packet(id, &payload).await?;
    }
    *player_was_dead = is_dead;
    Ok(())
}

#[cfg(test)]
mod tests;
