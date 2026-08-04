use anyhow::Result;
use bbb_protocol::packets::{
    self, BlockEntityTagQuery, ChangeDifficultyCommand, ChangeGameModeCommand, ChatAcknowledgement,
    ChatCommand, ChatCommandSigned, ChatMessage, ChatSessionUpdate, ClientCommandAction,
    CommandSuggestionRequest, ContainerButtonClick, ContainerClick, ContainerCloseRequest,
    ContainerSlotStateChanged, CustomClickAction, EditBook, EntityTagQuery, InteractionHand,
    JigsawGenerate, LockDifficultyCommand, PaddleBoat, PickItemFromBlock, PickItemFromEntity,
    PlaceRecipeCommand, PlayerAbilitiesCommand, PlayerAction, PlayerCommand, PlayerInput,
    PlayerPositionState, RecipeBookChangeSettingsCommand, RecipeBookSeenRecipeCommand, RenameItem,
    ResourcePackResponseAction, SeenAdvancements, SelectBundleItem, SelectTradeCommand,
    ServerboundCustomPayload, SetBeacon, SetCommandBlock, SetCommandMinecart, SetCreativeModeSlot,
    SetGameRule, SetJigsawBlock, SetStructureBlock, SetTestBlock, SignUpdate, SpectateEntity,
    TeleportToEntity, TestInstanceBlockAction,
};
use uuid::Uuid;

use crate::{
    connection::RawConnection,
    types::{
        AttackEntityCommand, ConnectionState, InteractEntityCommand, PlayerMoveCommand,
        UseItemCommand, UseItemOnCommand, VehicleMoveCommand,
    },
};

pub(super) async fn send_resource_pack_response(
    conn: &mut RawConnection,
    state: ConnectionState,
    id: Uuid,
    action: ResourcePackResponseAction,
) -> Result<()> {
    let packet = match state {
        ConnectionState::Configuration => Some(
            packets::encode_configuration_resource_pack_response(id, action),
        ),
        ConnectionState::Play => Some(packets::encode_play_resource_pack_response(id, action)),
        ConnectionState::Handshake | ConnectionState::Status | ConnectionState::Login => None,
    };
    if let Some((packet_id, payload)) = packet {
        conn.send_packet(packet_id, &payload).await?;
    }
    Ok(())
}

pub(super) async fn send_client_tick_end(conn: &mut RawConnection) -> Result<()> {
    let (packet_id, payload) = packets::encode_play_client_tick_end();
    conn.send_packet(packet_id, &payload).await
}

pub(super) async fn send_player_loaded(conn: &mut RawConnection) -> Result<()> {
    let (packet_id, payload) = packets::encode_play_player_loaded();
    conn.send_packet(packet_id, &payload).await
}

pub(super) async fn send_player_move_command(
    conn: &mut RawConnection,
    command: PlayerMoveCommand,
    player_position_state: &mut PlayerPositionState,
) -> Result<()> {
    let previous = *player_position_state;
    let (id, payload) = command.encode_packet_from(*player_position_state);
    conn.send_packet(id, &payload).await?;
    *player_position_state = command.apply_to_position_state(previous);
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

pub(crate) async fn send_chat_command_signed(
    conn: &mut RawConnection,
    packet: ChatCommandSigned,
) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_command_signed(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_chat_message(conn: &mut RawConnection, packet: ChatMessage) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_message(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_chat_session_update(
    conn: &mut RawConnection,
    packet: ChatSessionUpdate,
) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_session_update(&packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_chat_acknowledgement(
    conn: &mut RawConnection,
    command: ChatAcknowledgement,
) -> Result<()> {
    let (id, payload) = packets::encode_play_chat_acknowledgement(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_custom_payload(
    conn: &mut RawConnection,
    packet: ServerboundCustomPayload,
) -> Result<()> {
    let (id, payload) = packets::encode_play_custom_payload(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_custom_click_action(
    conn: &mut RawConnection,
    state: ConnectionState,
    packet: CustomClickAction,
) -> Result<()> {
    let encoded = match state {
        ConnectionState::Configuration => {
            Some(packets::encode_configuration_custom_click_action(&packet)?)
        }
        ConnectionState::Play => Some(packets::encode_play_custom_click_action(&packet)?),
        ConnectionState::Handshake | ConnectionState::Status | ConnectionState::Login => None,
    };
    if let Some((id, payload)) = encoded {
        conn.send_packet(id, &payload).await?;
    }
    Ok(())
}

pub(crate) async fn send_debug_subscription_request(
    conn: &mut RawConnection,
    subscription_ids: &[i32],
) -> Result<()> {
    let (id, payload) = packets::encode_play_debug_subscription_request(subscription_ids)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_attack_entity(
    conn: &mut RawConnection,
    command: AttackEntityCommand,
) -> Result<()> {
    send_set_held_slot_command(conn, command.selected_hotbar_slot).await?;
    let (id, payload) = packets::encode_play_attack_entity(command.packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_interact_entity(
    conn: &mut RawConnection,
    command: InteractEntityCommand,
) -> Result<()> {
    send_set_held_slot_command(conn, command.selected_hotbar_slot).await?;
    let (id, payload) = packets::encode_play_interact_entity(command.packet);
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
    let slot = slot.min(8);
    if conn.serverbound_carried_hotbar_slot() == Some(slot) {
        return Ok(());
    }
    let (id, payload) = packets::encode_play_set_carried_item(i16::from(slot));
    conn.send_packet(id, &payload).await?;
    conn.remember_serverbound_carried_hotbar_slot(slot);
    Ok(())
}

pub(crate) async fn send_swing_command(
    conn: &mut RawConnection,
    hand: InteractionHand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_swing(hand);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item_on(
    conn: &mut RawConnection,
    command: UseItemOnCommand,
) -> Result<()> {
    send_set_held_slot_command(conn, command.selected_hotbar_slot).await?;
    let (id, payload) = packets::encode_play_use_item_on(command.packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_use_item(conn: &mut RawConnection, command: UseItemCommand) -> Result<()> {
    send_set_held_slot_command(conn, command.selected_hotbar_slot).await?;
    let (id, payload) = packets::encode_play_use_item(command.packet);
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

/// `ClientCommonPacketListenerImpl.broadcastClientInformation` from the play
/// phase.
pub(crate) async fn send_play_client_information(
    conn: &mut RawConnection,
    information: &packets::ClientInformation,
) -> Result<()> {
    let (id, payload) = packets::encode_play_client_information(information)?;
    conn.send_packet(id, &payload).await
}

/// The same broadcast while the connection is still in configuration, where the
/// packet keeps its configuration-phase id.
pub(crate) async fn send_configuration_client_information(
    conn: &mut RawConnection,
    information: &packets::ClientInformation,
) -> Result<()> {
    let (id, payload) = packets::encode_client_information(information)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_block_entity_tag_query(
    conn: &mut RawConnection,
    packet: BlockEntityTagQuery,
) -> Result<()> {
    let (id, payload) = packets::encode_play_block_entity_tag_query(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_entity_tag_query(
    conn: &mut RawConnection,
    packet: EntityTagQuery,
) -> Result<()> {
    let (id, payload) = packets::encode_play_entity_tag_query(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_change_difficulty(
    conn: &mut RawConnection,
    command: ChangeDifficultyCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_change_difficulty(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_change_game_mode(
    conn: &mut RawConnection,
    command: ChangeGameModeCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_change_game_mode(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_lock_difficulty(
    conn: &mut RawConnection,
    command: LockDifficultyCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_lock_difficulty(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_game_rule(
    conn: &mut RawConnection,
    packet: SetGameRule,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_game_rule(&packet)?;
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

pub(crate) async fn send_edit_book(conn: &mut RawConnection, packet: EditBook) -> Result<()> {
    let (id, payload) = packets::encode_play_edit_book(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_select_trade(
    conn: &mut RawConnection,
    command: SelectTradeCommand,
) -> Result<()> {
    let (id, payload) = packets::encode_play_select_trade(command);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_beacon(conn: &mut RawConnection, packet: SetBeacon) -> Result<()> {
    let (id, payload) = packets::encode_play_set_beacon(packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_command_block(
    conn: &mut RawConnection,
    packet: SetCommandBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_command_block(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_command_minecart(
    conn: &mut RawConnection,
    packet: SetCommandMinecart,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_command_minecart(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_jigsaw_block(
    conn: &mut RawConnection,
    packet: SetJigsawBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_jigsaw_block(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_jigsaw_generate(
    conn: &mut RawConnection,
    packet: JigsawGenerate,
) -> Result<()> {
    let (id, payload) = packets::encode_play_jigsaw_generate(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_structure_block(
    conn: &mut RawConnection,
    packet: SetStructureBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_structure_block(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_test_block(
    conn: &mut RawConnection,
    packet: SetTestBlock,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_test_block(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_test_instance_block_action(
    conn: &mut RawConnection,
    packet: TestInstanceBlockAction,
) -> Result<()> {
    let (id, payload) = packets::encode_play_test_instance_block_action(&packet)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_set_creative_mode_slot(
    conn: &mut RawConnection,
    packet: SetCreativeModeSlot,
) -> Result<()> {
    let (id, payload) = packets::encode_play_set_creative_mode_slot(&packet)?;
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

pub(crate) async fn send_spectate_entity(
    conn: &mut RawConnection,
    packet: SpectateEntity,
) -> Result<()> {
    let (id, payload) = packets::encode_play_spectate_entity(packet);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_teleport_to_entity(
    conn: &mut RawConnection,
    packet: TeleportToEntity,
) -> Result<()> {
    let (id, payload) = packets::encode_play_teleport_to_entity(packet);
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
    let (id, payload) = packets::encode_play_container_click(packet)?;
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
    let (id, payload) = packets::encode_play_command_suggestion_request(request)?;
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_client_command(
    conn: &mut RawConnection,
    action: ClientCommandAction,
) -> Result<()> {
    let (id, payload) = packets::encode_play_client_command(action);
    conn.send_packet(id, &payload).await
}

pub(crate) async fn send_accept_code_of_conduct(conn: &mut RawConnection) -> Result<()> {
    let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
    conn.send_packet(id, &payload).await
}

#[cfg(test)]
mod tests;
