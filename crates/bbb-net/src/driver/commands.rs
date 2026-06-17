use anyhow::Result;
use bbb_protocol::packets::{
    self, AttackEntity, ChatCommand, CommandSuggestionRequest, ContainerButtonClick,
    ContainerClick, ContainerCloseRequest, ContainerSlotStateChanged, InteractEntity,
    InteractionHand, PaddleBoat, PickItemFromBlock, PickItemFromEntity, PlaceRecipeCommand,
    PlayerAbilitiesCommand, PlayerAction, PlayerCommand, PlayerHealth, PlayerInput,
    PlayerPositionState, RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RenameItem,
    SeenAdvancements, SelectBundleItem, SelectTradeCommand, SignUpdate, UseItem, UseItemOn,
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
    let (id, payload) = command.encode_packet_from(*player_position_state);
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

pub(crate) async fn send_chat_command(conn: &mut RawConnection, packet: ChatCommand) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_command(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_attack_entity(
    conn: &mut RawConnection,
    packet: AttackEntity,
) -> Result<()> {
    let (id, payload) = packets::encode_play_attack_entity(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_interact_entity(
    conn: &mut RawConnection,
    packet: InteractEntity,
) -> Result<()> {
    let (id, payload) = packets::encode_play_interact_entity(packet);
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

pub(crate) async fn send_player_abilities_command(
    conn: &mut RawConnection,
    command: PlayerAbilitiesCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_player_abilities(command);
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

pub(crate) async fn send_pick_item_from_entity(
    conn: &mut RawConnection,
    packet: PickItemFromEntity,
) -> Result<()> {
    let (id, payload) = packets::encode_play_pick_item_from_entity(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_paddle_boat(conn: &mut RawConnection, packet: PaddleBoat) -> Result<()> {
    let (id, payload) = packets::encode_play_paddle_boat(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_ping_request(conn: &mut RawConnection, time: i64) -> Result<()> {
    let (id, payload) = packets::encode_play_ping_request(time);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_place_recipe(
    conn: &mut RawConnection,
    command: PlaceRecipeCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_place_recipe(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_recipe_book_change_settings(
    conn: &mut RawConnection,
    command: RecipeBookChangeSettingsCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_recipe_book_change_settings(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_recipe_book_seen_recipe(
    conn: &mut RawConnection,
    command: RecipeBookSeenRecipeCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_recipe_book_seen_recipe(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_select_trade(
    conn: &mut RawConnection,
    command: SelectTradeCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_select_trade(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_rename_item(conn: &mut RawConnection, packet: RenameItem) -> Result<()> {
    let (id, payload) = packets::encode_play_rename_item(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_seen_advancements(
    conn: &mut RawConnection,
    packet: SeenAdvancements,
) -> Result<()> {
    let (id, payload) = packets::encode_play_seen_advancements(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_sign_update(conn: &mut RawConnection, packet: SignUpdate) -> Result<()> {
    let (id, payload) = packets::encode_play_sign_update(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_select_bundle_item(
    conn: &mut RawConnection,
    packet: SelectBundleItem,
) -> Result<()> {
    let (id, payload) = packets::encode_play_select_bundle_item(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_container_button_click(
    conn: &mut RawConnection,
    packet: ContainerButtonClick,
) -> Result<()> {
    let (id, payload) = packets::encode_play_container_button_click(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_container_click(
    conn: &mut RawConnection,
    packet: ContainerClick,
) -> Result<()> {
    let (id, payload) = packets::encode_play_container_click(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_container_close(
    conn: &mut RawConnection,
    packet: ContainerCloseRequest,
) -> Result<()> {
    let (id, payload) = packets::encode_play_container_close(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_container_slot_state_changed(
    conn: &mut RawConnection,
    packet: ContainerSlotStateChanged,
) -> Result<()> {
    let (id, payload) = packets::encode_play_container_slot_state_changed(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_command_suggestion_request(
    conn: &mut RawConnection,
    request: CommandSuggestionRequest,
) -> Result<()> {
    let (id, payload) = packets::encode_play_command_suggestion_request(request);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_accept_code_of_conduct(conn: &mut RawConnection) -> Result<()> {
    let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
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
