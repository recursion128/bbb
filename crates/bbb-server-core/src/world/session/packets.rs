//! Serverbound Play packet dispatch for an active player session.

use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::packets::{
    AwardStats, ChatMessage, ClientCommandAction, ClientInformation, PlayServerbound,
    PlayerCommand, PlayerInfoUpdate, PongResponse, RecipeBookSeenRecipeCommand, SeenAdvancements,
    SelectAdvancementsTab, StatUpdate, SystemChat, Vec3d,
};
use bbb_protocol::player_command_actions::PlayerCommandAction;
use bbb_protocol::player_info_actions::PlayerInfoAction;
use bbb_protocol::RichTextComponent;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use crate::commands;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::inventory::{PlayerInventoryMenu, SELECTION_SIZE};
use crate::world::{
    initializing_player_info_entry, send, ChatKind, OutgoingChat, PendingBookEdit, ServerWorld,
    WorldCloseReason, WorldEvent,
};

impl ServerWorld {
    pub(in crate::world) fn apply_play(
        &mut self,
        connection_id: ConnectionId,
        packet: PlayServerbound,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.levels.contains_player(connection_id) {
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::UnknownConnection,
            });
            return;
        }
        let close = match packet {
            PlayServerbound::AcceptTeleportation { id } => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                if player.awaiting_teleport == Some(id) {
                    player.awaiting_teleport = None;
                    player.last_good = player.position();
                    // `ServerGamePacketListenerImpl.java:544`.
                    player.is_changing_dimension = false;
                    // The confirming position packet needs terrain from the new player ticket.
                    let _ = self.move_session_chunk_tracking(connection_id, events);
                }
                None
            }
            PlayServerbound::ChangeDifficulty { difficulty } => {
                self.apply_change_difficulty(connection_id, difficulty, events);
                None
            }
            PlayServerbound::ChangeGameMode(packet) => {
                self.apply_change_game_mode_request(connection_id, packet.game_mode, events)
            }
            PlayServerbound::SetGameRule(packet) => {
                self.apply_set_game_rule_request(connection_id, packet, events);
                None
            }
            PlayServerbound::ClientCommand(action) => {
                self.apply_client_command(connection_id, action, events)
            }
            PlayServerbound::ChatAck { offset } => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                match player.apply_last_seen_offset(offset) {
                    true => None,
                    false => Some(WorldCloseReason::ChatValidationFailed),
                }
            }
            PlayServerbound::ChatSessionUpdate(_) => None,
            PlayServerbound::Chat(chat) => self.apply_chat(connection_id, chat, events),
            PlayServerbound::ChatCommand { command } => illegal_chat(&command).or_else(|| {
                self.apply_chat_command(connection_id, &command, events)
                    .or_else(|| self.detect_rate_spam(connection_id))
            }),
            PlayServerbound::ChatCommandSigned(packet) => {
                if !self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked")
                    .apply_last_seen_update(packet.last_seen_messages)
                {
                    Some(WorldCloseReason::ChatValidationFailed)
                } else {
                    illegal_chat(&packet.command).or_else(|| {
                        self.apply_signed_chat_command(connection_id, &packet, events)
                            .or_else(|| self.detect_rate_spam(connection_id))
                    })
                }
            }
            PlayServerbound::CommandSuggestion(request) => {
                self.apply_command_suggestion(connection_id, &request, events)
            }
            PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick,
            } => {
                let accepted = self
                    .sessions
                    .get_mut(&connection_id)
                    .expect("connection presence was checked")
                    .chunk_sender
                    .on_chunk_batch_received_by_client(desired_chunks_per_tick);
                if accepted {
                    None
                } else {
                    Some(WorldCloseReason::PacketOutsideActiveContract)
                }
            }
            PlayServerbound::ClientInformation(information) => {
                self.apply_client_information(connection_id, information, events)
            }
            PlayServerbound::ClientTickEnd => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                if !player.received_movement_this_tick {
                    player.known_movement = Vec3d::default();
                }
                player.received_movement_this_tick = false;
                None
            }
            PlayServerbound::ContainerButtonClick(click) => {
                self.apply_container_button_click(connection_id, click, events);
                None
            }
            PlayServerbound::ContainerClick(click) => {
                self.apply_container_click(connection_id, click, events);
                None
            }
            PlayServerbound::ContainerClose(_) => {
                self.apply_container_close(connection_id, events);
                None
            }
            PlayServerbound::ContainerSlotStateChanged(change) => {
                self.apply_container_slot_state_changed(connection_id, change);
                None
            }
            PlayServerbound::ConfigurationAcknowledged => {
                Some(WorldCloseReason::PacketOutsideActiveContract)
            }
            PlayServerbound::CookieResponse { .. } => Some(WorldCloseReason::UnexpectedQuery),
            PlayServerbound::CustomClickAction(action) => {
                self.handle_custom_click_action(connection_id, action);
                None
            }
            PlayServerbound::CustomPayload(_) => None,
            PlayServerbound::KeepAlive { id } => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                if player.cold.pending_keep_alive == Some(id) {
                    player.cold.pending_keep_alive = None;
                    let time = (crate::world::idle_timeout::monotonic_millis()
                        - player.cold.keep_alive_sent_millis) as i32;
                    player.cold.latency = (player.cold.latency * 3 + time) / 4;
                    None
                } else {
                    Some(WorldCloseReason::KeepAliveMismatch)
                }
            }
            PlayServerbound::LockDifficulty { locked } => {
                self.apply_lock_difficulty(connection_id, locked, events);
                None
            }
            PlayServerbound::MovePlayer(movement) => {
                self.apply_move_player(connection_id, movement, events)
            }
            PlayServerbound::PingRequest { time } => {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::PongResponse(PongResponse { time }),
                ));
                None
            }
            PlayServerbound::PlayerAbilities { flying } => {
                let abilities = &mut self.levels[connection_id].abilities;
                abilities.flying = flying && abilities.can_fly;
                None
            }
            PlayServerbound::Attack(packet) => self.apply_attack(connection_id, packet, events),
            PlayServerbound::Interact(packet) => {
                if !self.apply_external_player_boat_interact(connection_id, packet, events) {
                    self.apply_interact(connection_id, packet, events);
                }
                None
            }
            PlayServerbound::BlockEntityTagQuery(packet) => {
                self.answer_block_entity_tag_query(connection_id, packet, events);
                None
            }
            PlayServerbound::EntityTagQuery(packet) => {
                self.answer_entity_tag_query(connection_id, packet, events);
                None
            }
            PlayServerbound::DebugSubscriptionRequest { subscription_ids } => {
                self.levels[connection_id]
                    .cold
                    .requested_debug_subscriptions
                    .replace(subscription_ids);
                None
            }
            PlayServerbound::SetCommandBlock(packet) => {
                self.apply_set_command_block(connection_id, packet, events);
                None
            }
            PlayServerbound::SetCommandMinecart(packet) => {
                self.apply_set_command_minecart(connection_id, packet, events);
                None
            }
            PlayServerbound::SetJigsawBlock(packet) => {
                self.apply_set_jigsaw_block(connection_id, packet);
                None
            }
            PlayServerbound::JigsawGenerate(packet) => {
                self.apply_jigsaw_generate(connection_id, packet);
                None
            }
            PlayServerbound::SetStructureBlock(packet) => {
                self.apply_set_structure_block(connection_id, packet, events);
                None
            }
            PlayServerbound::SetTestBlock(_) | PlayServerbound::TestInstanceBlockAction(_) => None,
            PlayServerbound::SpectateEntity(packet) => {
                self.apply_spectate_entity(connection_id, packet.entity_id, events);
                None
            }
            PlayServerbound::TeleportToEntity(packet) => {
                self.apply_teleport_to_entity(connection_id, packet.uuid, events);
                None
            }
            PlayServerbound::PickItemFromBlock(packet) => {
                self.apply_pick_item_from_block(connection_id, packet, events);
                None
            }
            PlayServerbound::PickItemFromEntity(packet) => {
                self.apply_pick_item_from_entity(connection_id, packet, events);
                None
            }
            PlayServerbound::PlayerAction(action) => {
                self.apply_player_action(connection_id, action, events)
            }
            PlayServerbound::PlayerCommand(command) => {
                self.apply_player_command(connection_id, command, events);
                None
            }
            PlayServerbound::PlayerInput(input) => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                player.last_client_input = input;
                if player.client_loaded() {
                    player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, input.shift);
                }
                None
            }
            PlayServerbound::PlayerLoaded => {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                player.mark_client_loaded();
                None
            }
            PlayServerbound::Pong { .. } => None,
            PlayServerbound::ResourcePack { action, .. } => {
                use bbb_protocol::resource_pack_response_actions::ResourcePackResponseAction;
                (self.cold.config.resource_pack_required
                    && matches!(
                        action,
                        ResourcePackResponseAction::Declined
                            | ResourcePackResponseAction::FailedDownload
                            | ResourcePackResponseAction::InvalidUrl
                            | ResourcePackResponseAction::FailedReload
                            | ResourcePackResponseAction::Discarded
                    ))
                .then_some(WorldCloseReason::RequiredResourcePackDeclined)
            }
            PlayServerbound::SetCarriedItem { slot } => {
                if let Ok(slot) = u8::try_from(slot) {
                    if usize::from(slot) < SELECTION_SIZE {
                        let player = self.levels.player_at(connection_id);
                        if player.inventory.selected_slot() != slot
                            && player.using_item.used_item_hand()
                                == bbb_protocol::packets::InteractionHand::MainHand
                        {
                            self.stop_using_item(connection_id);
                        }
                        self.levels[connection_id].inventory.set_selected_slot(slot);
                    }
                }
                None
            }
            PlayServerbound::SetCreativeModeSlot(update) => {
                self.apply_creative_slot(connection_id, update, events);
                None
            }
            PlayServerbound::Swing { hand } => {
                self.player_swing(connection_id, hand, false, events);
                self.levels[connection_id].reset_attack_strength_ticker();
                None
            }
            PlayServerbound::MoveVehicle(update) => {
                self.apply_external_player_boat_move(connection_id, update, events)
            }
            PlayServerbound::PaddleBoat(update) => {
                self.apply_external_player_boat_paddles(connection_id, update);
                None
            }
            PlayServerbound::RecipeBookChangeSettings(command) => {
                self.levels[connection_id].set_recipe_book_setting(command);
                None
            }
            PlayServerbound::EditBook(packet) => {
                if let Some(slot) = PlayerInventoryMenu::edit_book_slot(packet.slot) {
                    self.levels[connection_id]
                        .cold
                        .pending_book_edits
                        .push(PendingBookEdit { slot, packet });
                }
                None
            }
            PlayServerbound::SignUpdate(update) => {
                self.apply_sign_update(connection_id, update, events);
                None
            }
            PlayServerbound::PlaceRecipe(request) => {
                self.apply_place_recipe(connection_id, request, events);
                None
            }
            PlayServerbound::SelectBundleItem(select) => {
                if let (Some(player), Ok(slot)) = (
                    self.levels.player_mut(connection_id),
                    i16::try_from(select.slot_id),
                ) {
                    player
                        .inventory
                        .select_bundle_item(slot, select.selected_item_index);
                }
                None
            }
            PlayServerbound::RecipeBookSeenRecipe(command) => {
                self.apply_seen_recipe(connection_id, command);
                None
            }
            PlayServerbound::SelectTrade(command) => {
                self.apply_select_trade(connection_id, command.item, events);
                None
            }
            PlayServerbound::RenameItem(update) => {
                self.apply_rename_item(connection_id, &update.name, events);
                None
            }
            PlayServerbound::SetBeacon(update) => {
                self.apply_set_beacon(connection_id, &update, events);
                None
            }
            PlayServerbound::SeenAdvancements(update) => {
                self.apply_seen_advancements(connection_id, &update, events);
                None
            }
            PlayServerbound::UseItem(update) => self.apply_use_item(connection_id, update, events),
            PlayServerbound::UseItemOn(update) => {
                self.apply_use_item_on(connection_id, update, events)
            }
        };
        if let Some(reason) = close {
            self.remove_player(connection_id, events);
            events.push(WorldEvent::Close {
                connection_id,
                reason,
            });
        }
    }

    pub(in crate::world) fn apply_chat(
        &mut self,
        connection_id: ConnectionId,
        chat: ChatMessage,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if !self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked")
            .apply_last_seen_update(chat.last_seen_messages)
        {
            return Some(WorldCloseReason::ChatValidationFailed);
        }
        if let Some(illegal) = illegal_chat(&chat.message) {
            return Some(illegal);
        }
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        // ServerGamePacketListenerImpl.java:1613-1622.
        if player.cold.client_information.chat_visibility
            == bbb_protocol::packets::ClientChatVisibility::Hidden
        {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                    RichTextComponent::try_from_component_nbt(
                        bbb_protocol::component_translatable_nbt(
                            "chat.disabled.options",
                            Vec::new(),
                            &bbb_protocol::ComponentStyle::with_chat_formatting_color(
                                bbb_protocol::chat_formattings::ChatFormatting::Red,
                            )
                            .expect("red is a color"),
                        )
                        .expect("valid chat refusal"),
                    )
                    .expect("valid chat refusal"),
                    false,
                )),
            ));
            return None;
        }
        let view = self.command_player_view(connection_id);
        let name = commands::team_player_display_name_nbt(&view, self.scoreboard());
        let message = OutgoingChat::new_with_registry(
            Some(view.uuid),
            chat.message.as_str(),
            bbb_protocol::component_literal_nbt(chat.message.as_str()),
            ChatKind::Chat,
            name,
            std::sync::Arc::clone(self.registries.content_registry_arc()),
        );
        self.broadcast_chat(&message, events);
        self.detect_rate_spam(connection_id)
    }

    /// `ServerGamePacketListenerImpl.detectRateSpam` (:1660-1667).
    fn detect_rate_spam(&mut self, connection_id: ConnectionId) -> Option<WorldCloseReason> {
        let player = self.levels.player_mut(connection_id)?;
        player.cold.chat_spam = player.cold.chat_spam.saturating_add(20);
        (player.cold.chat_spam >= 200 && !self.admin.is_operator(player.profile.uuid)).then(|| {
            WorldCloseReason::Administrative(commands::disconnect_reason("disconnect.spam"))
        })
    }

    // ServerPlayer.java:1834-1844; ServerGamePacketListenerImpl.java:2053-2060.
    pub(in crate::world) fn apply_client_information(
        &mut self,
        connection_id: ConnectionId,
        information: ClientInformation,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let was_hat_shown = player.cold.client_information.model_parts().hat;
        let listing_changed =
            player.cold.client_information.allows_listing != information.allows_listing;
        player.cold.main_hand_dirty |=
            player.cold.client_information.main_hand != information.main_hand;
        player.cold.model_customization_dirty |=
            player.cold.client_information.displayed_skin_parts != information.displayed_skin_parts;
        player.cold.client_information = information;
        let update =
            (player.cold.client_information.model_parts().hat != was_hat_shown).then(|| {
                PlayerInfoUpdate {
                    actions: vec![PlayerInfoAction::UpdateHat],
                    entries: vec![initializing_player_info_entry(player)],
                }
            });
        if listing_changed {
            self.levels.bump_player_list_revision();
        }
        if let Some(update) = update {
            self.broadcast(ClientboundPlayPacket::PlayerInfoUpdate(update), events);
        }
        None
    }

    // ServerGamePacketListenerImpl.java:1689-1731: all actions require hasClientLoaded.
    pub(in crate::world) fn apply_player_command(
        &mut self,
        connection_id: ConnectionId,
        command: PlayerCommand,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if !player.client_loaded() {
            return;
        }
        match command.action {
            PlayerCommandAction::StartSprinting => player.set_sprinting(true),
            PlayerCommandAction::StopSprinting => player.set_sprinting(false),
            PlayerCommandAction::StartFallFlying => self.start_player_fall_flying(connection_id),
            PlayerCommandAction::StopSleeping => {
                if player.cold.sleeping_pos.is_some() {
                    self.stop_player_sleep(connection_id, false, true, events);
                }
            }
            PlayerCommandAction::StartRidingJump => {
                self.start_vehicle_riding_jump(connection_id, command.data, events);
            }
            PlayerCommandAction::StopRidingJump => {}
            PlayerCommandAction::OpenInventory => {
                self.open_vehicle_inventory(connection_id, events)
            }
        }
    }

    /// `ServerGamePacketListenerImpl.handleClientCommand` (:1859-1891).
    pub(in crate::world) fn apply_client_command(
        &mut self,
        connection_id: ConnectionId,
        action: ClientCommandAction,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        match action {
            ClientCommandAction::PerformRespawn if self.levels[connection_id].cold.won_game => {
                self.levels[connection_id].cold.won_game = false;
                if self
                    .perform_respawn_checked(connection_id, true, events)
                    .is_some()
                {
                    self.trigger_changed_dimension(
                        connection_id,
                        &Java26LevelKey::from_builtin(Java26BuiltinDimension::End),
                        &Java26LevelKey::from_builtin(Java26BuiltinDimension::Overworld),
                        events,
                    );
                }
            }
            ClientCommandAction::PerformRespawn => {
                let dead = self.levels[connection_id].damage.is_dead_or_dying();
                self.perform_respawn(connection_id, events);
                let respawned = !self.levels[connection_id].damage.is_dead_or_dying();
                if self.cold.hardcore && dead && respawned {
                    self.hardcore_respawn(connection_id, events);
                }
            }
            ClientCommandAction::RequestStats => {
                let stats = self
                    .levels
                    .player_mut(connection_id)
                    .map(|player| player.cold.stats.take_dirty())
                    .unwrap_or_default();
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::AwardStats(AwardStats {
                        stats: stats
                            .into_iter()
                            .map(|(stat, amount)| StatUpdate {
                                stat_type_id: stat.stat_type_id,
                                value_id: stat.value_id,
                                amount,
                            })
                            .collect(),
                    }),
                ));
            }
            ClientCommandAction::RequestGameRuleValues => {
                self.send_game_rule_values(connection_id, events);
            }
        }
        None
    }

    /// `handleRecipeBookSeenRecipePacket` (`ServerGamePacketListenerImpl.java:556-562`).
    fn apply_seen_recipe(
        &mut self,
        connection_id: ConnectionId,
        command: RecipeBookSeenRecipeCommand,
    ) {
        let Some(recipe_id) = self
            .cold
            .config
            .datapacks
            .as_deref()
            .and_then(|startup| startup.recipe_displays.parent_recipe_id(command.recipe))
            .map(str::to_owned)
        else {
            return;
        };
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        player
            .inventory
            .recipe_book_mut()
            .remove_highlight(&recipe_id);
    }

    /// `ServerGamePacketListenerImpl.handleSeenAdvancements` (:577-586).
    pub(in crate::world) fn apply_seen_advancements(
        &mut self,
        connection_id: ConnectionId,
        update: &SeenAdvancements,
        events: &mut Vec<WorldEvent>,
    ) {
        let SeenAdvancements::OpenedTab { tab } = update else {
            return;
        };
        let Some(index) = self.cold.advancement_tree.index_of(tab) else {
            return;
        };
        let tree = &self.cold.advancement_tree;
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let Some(selected) = player.cold.advancements.set_selected_tab(tree, Some(index)) else {
            return;
        };
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SelectAdvancementsTab(SelectAdvancementsTab { tab: selected }),
        ));
    }

    /// `ServerCommonPacketListenerImpl.handleCustomClickAction`
    /// (`ServerCommonPacketListenerImpl.java:99-102`) ->
    /// `MinecraftServer.handleCustomClickAction` (`MinecraftServer.java:2061-2063`).
    fn handle_custom_click_action(
        &self,
        connection_id: ConnectionId,
        action: bbb_protocol::packets::CustomClickAction,
    ) {
        tracing::debug!(
            event = "play.custom_click_action",
            connection_id = connection_id.0,
            action_id = %action.id,
            has_payload = action.payload.is_some(),
        );
    }
}

/// `isChatMessageIllegal` (ServerGamePacketListenerImpl.java:1640-1648) over
/// `StringUtil.isAllowedChatCharacter` (StringUtil.java:62-64).
fn illegal_chat(message: &str) -> Option<WorldCloseReason> {
    message
        .encode_utf16()
        .any(|unit| unit == 167 || unit < 32 || unit == 127)
        .then(|| {
            WorldCloseReason::Administrative(commands::disconnect_reason(
                "multiplayer.disconnect.illegal_characters",
            ))
        })
}
