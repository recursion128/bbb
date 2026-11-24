use super::*;
use crate::content_registry::ContentRegistry;

pub fn decode_play_serverbound(packet_id: i32, payload: &[u8]) -> Result<PlayServerbound> {
    decode_play_serverbound_with_content_registry(packet_id, payload, ContentRegistry::vanilla())
}

pub fn decode_play_serverbound_with_content_registry(
    packet_id: i32,
    payload: &[u8],
    registry: &ContentRegistry,
) -> Result<PlayServerbound> {
    let mut decoder = Decoder::new(payload);
    let packet = match packet_id {
        ids::play::SERVERBOUND_ACCEPT_TELEPORTATION => PlayServerbound::AcceptTeleportation {
            id: decoder.read_var_i32()?,
        },
        // `Difficulty.STREAM_CODEC` (`Difficulty.java:19-20`) is an id mapper,
        // so the request carries the difficulty id as a var int. The wrapping
        // `BY_ID` strategy is what `Difficulty::from_id` already implements.
        ids::play::SERVERBOUND_CHANGE_DIFFICULTY => PlayServerbound::ChangeDifficulty {
            difficulty: Difficulty::from_id(decoder.read_var_i32()?),
        },
        ids::play::SERVERBOUND_CHAT => PlayServerbound::Chat(decode_chat(&mut decoder)?),
        ids::play::SERVERBOUND_COMMAND_SUGGESTION => PlayServerbound::CommandSuggestion(
            super::super::command_suggestions::decode_command_suggestion_request(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED => {
            PlayServerbound::ChatCommandSigned(decode_chat_command_signed(&mut decoder)?)
        }
        // ServerboundChatCommandPacket uses the default readUtf limit, unlike chat messages.
        ids::play::SERVERBOUND_CHAT_COMMAND => PlayServerbound::ChatCommand {
            command: decoder.read_string(super::super::serverbound::MAX_CHAT_COMMAND_CHARS)?,
        },
        ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED => PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: decoder.read_f32()?,
        },
        // `ServerGamePacketListenerImpl.handleClientInformation`
        // (ServerGamePacketListenerImpl.java:2053-2060): the same payload the
        // configuration state carries, resent whenever the client's options
        // change mid-session.
        ids::play::SERVERBOUND_CLIENT_INFORMATION => PlayServerbound::ClientInformation(
            super::super::connection::decode_client_information_payload(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_CLIENT_TICK_END => PlayServerbound::ClientTickEnd,
        ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED => {
            PlayServerbound::ConfigurationAcknowledged
        }
        ids::play::SERVERBOUND_COOKIE_RESPONSE => {
            let (key, payload) =
                super::super::connection::decode_cookie_response_payload(&mut decoder)?;
            PlayServerbound::CookieResponse { key, payload }
        }
        ids::play::SERVERBOUND_CUSTOM_CLICK_ACTION => PlayServerbound::CustomClickAction(
            super::super::client_common::decode_custom_click_action_payload(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_CUSTOM_PAYLOAD => {
            PlayServerbound::CustomPayload(super::super::client_common::decode_custom_payload(
                &mut decoder,
                super::super::client_common::MAX_SERVERBOUND_CUSTOM_PAYLOAD,
            )?)
        }
        ids::play::SERVERBOUND_KEEP_ALIVE => PlayServerbound::KeepAlive {
            id: decoder.read_i64()?,
        },
        ids::play::SERVERBOUND_LOCK_DIFFICULTY => PlayServerbound::LockDifficulty {
            locked: decoder.read_bool()?,
        },
        // `ServerboundPlayerAbilitiesPacket.FLAG_FLYING`
        // (`ServerboundPlayerAbilitiesPacket.java:13`): every other bit the
        // client might set is ignored, exactly as vanilla's reader does.
        ids::play::SERVERBOUND_PLAYER_ABILITIES => PlayServerbound::PlayerAbilities {
            flying: decoder.read_u8()? & PLAYER_ABILITIES_FLAG_FLYING != 0,
        },
        // `ServerGamePacketListenerImpl.handlePingRequest`
        // (ServerGamePacketListenerImpl.java:1779-1781).
        ids::play::SERVERBOUND_PING_REQUEST => PlayServerbound::PingRequest {
            time: decoder.read_i64()?,
        },
        ids::play::SERVERBOUND_PONG => PlayServerbound::Pong {
            id: decoder.read_i32()?,
        },
        ids::play::SERVERBOUND_RESOURCE_PACK => {
            let id = decoder.read_uuid()?;
            let ordinal = decoder.read_var_i32()?;
            PlayServerbound::ResourcePack {
                id,
                action: ResourcePackResponseAction::from_ordinal(ordinal).ok_or_else(|| {
                    ProtocolError::InvalidData(format!(
                        "invalid resource pack response ordinal {ordinal}"
                    ))
                })?,
            }
        }
        ids::play::SERVERBOUND_MOVE_PLAYER_POS => {
            let position = decode_position(&mut decoder)?;
            let (on_ground, horizontal_collision) = decode_move_flags(&mut decoder)?;
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: None,
                on_ground,
                horizontal_collision,
            })
        }
        ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT => {
            let position = decode_position(&mut decoder)?;
            let rotation = decode_rotation(&mut decoder)?;
            let (on_ground, horizontal_collision) = decode_move_flags(&mut decoder)?;
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: Some(rotation),
                on_ground,
                horizontal_collision,
            })
        }
        ids::play::SERVERBOUND_MOVE_PLAYER_ROT => {
            let rotation = decode_rotation(&mut decoder)?;
            let (on_ground, horizontal_collision) = decode_move_flags(&mut decoder)?;
            PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: Some(rotation),
                on_ground,
                horizontal_collision,
            })
        }
        ids::play::SERVERBOUND_MOVE_PLAYER_STATUS_ONLY => {
            let (on_ground, horizontal_collision) = decode_move_flags(&mut decoder)?;
            PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: None,
                on_ground,
                horizontal_collision,
            })
        }
        // `ServerboundPlayerCommandPacket` writes the entity id, the action
        // ordinal and the action's data argument.
        ids::play::SERVERBOUND_PLAYER_COMMAND => {
            let entity_id = decoder.read_var_i32()?;
            let ordinal = decoder.read_var_i32()?;
            let action = crate::player_command_actions::VANILLA_PLAYER_COMMAND_ACTIONS
                .iter()
                .find(|entry| entry.ordinal == ordinal)
                .map(|entry| entry.action)
                .ok_or_else(|| {
                    ProtocolError::InvalidData(format!("invalid player command ordinal {ordinal}"))
                })?;
            PlayServerbound::PlayerCommand(PlayerCommand {
                entity_id,
                action,
                data: decoder.read_var_i32()?,
            })
        }
        // `Input.STREAM_CODEC` (`Input.java:14-37`) packs the seven movement
        // keys into one byte; bits above them are not part of the record.
        ids::play::SERVERBOUND_PLAYER_INPUT => {
            let flags = decoder.read_u8()?;
            PlayServerbound::PlayerInput(PlayerInput {
                forward: flags & super::super::serverbound::PLAYER_INPUT_FORWARD != 0,
                backward: flags & super::super::serverbound::PLAYER_INPUT_BACKWARD != 0,
                left: flags & super::super::serverbound::PLAYER_INPUT_LEFT != 0,
                right: flags & super::super::serverbound::PLAYER_INPUT_RIGHT != 0,
                jump: flags & super::super::serverbound::PLAYER_INPUT_JUMP != 0,
                shift: flags & super::super::serverbound::PLAYER_INPUT_SHIFT != 0,
                sprint: flags & super::super::serverbound::PLAYER_INPUT_SPRINT != 0,
            })
        }
        // `ServerboundAttackPacket.STREAM_CODEC`
        // (`ServerboundAttackPacket.java:10-12`) is a bare var-int entity id.
        ids::play::SERVERBOUND_ATTACK => PlayServerbound::Attack(AttackEntity {
            entity_id: decoder.read_var_i32()?,
        }),
        ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK => {
            PlayServerbound::ContainerButtonClick(ContainerButtonClick {
                container_id: decoder.read_var_i32()?,
                button_id: decoder.read_var_i32()?,
            })
        }
        ids::play::SERVERBOUND_CONTAINER_CLICK => {
            PlayServerbound::ContainerClick(decode_container_click(&mut decoder, registry)?)
        }
        ids::play::SERVERBOUND_CONTAINER_CLOSE => {
            PlayServerbound::ContainerClose(ContainerCloseRequest {
                container_id: decoder.read_var_i32()?,
            })
        }
        // `ServerboundInteractPacket.STREAM_CODEC`
        // (`ServerboundInteractPacket.java:23-31`): the entity id, the hand,
        // the aimed-at location and the secondary-action flag. The client
        // encoder writes the same order.
        ids::play::SERVERBOUND_INTERACT => PlayServerbound::Interact(InteractEntity {
            entity_id: decoder.read_var_i32()?,
            hand: decode_hand(&mut decoder)?,
            location: super::super::entities::decode_lp_vec3d(&mut decoder)?,
            using_secondary_action: decoder.read_bool()?,
        }),
        ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED => {
            PlayServerbound::ContainerSlotStateChanged(ContainerSlotStateChanged {
                slot_id: decoder.read_var_i32()?,
                container_id: decoder.read_var_i32()?,
                new_state: decoder.read_bool()?,
            })
        }
        // `ServerboundPickItemFromBlockPacket.STREAM_CODEC`
        // (`ServerboundPickItemFromBlockPacket.java:11-16`): the packed block
        // position and the data flag.
        ids::play::SERVERBOUND_CHANGE_GAME_MODE => {
            PlayServerbound::ChangeGameMode(serverbound::decode_change_game_mode(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SET_GAME_RULE => {
            PlayServerbound::SetGameRule(serverbound::decode_set_game_rule(&mut decoder)?)
        }
        ids::play::SERVERBOUND_CLIENT_COMMAND => {
            PlayServerbound::ClientCommand(serverbound::decode_client_command(&mut decoder)?)
        }
        // `ServerboundChatAckPacket.STREAM_CODEC`: a single var int.
        ids::play::SERVERBOUND_CHAT_ACK => PlayServerbound::ChatAck {
            offset: decoder.read_var_i32()?,
        },
        // `ServerboundChatSessionUpdatePacket.STREAM_CODEC`
        // (`ServerboundChatSessionUpdatePacket.java:10-20`).
        ids::play::SERVERBOUND_CHAT_SESSION_UPDATE => PlayServerbound::ChatSessionUpdate(
            super::super::player_info::decode_player_info_chat_session(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY => PlayServerbound::BlockEntityTagQuery(
            serverbound::decode_block_entity_tag_query(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_ENTITY_TAG_QUERY => {
            PlayServerbound::EntityTagQuery(serverbound::decode_entity_tag_query(&mut decoder)?)
        }
        ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST => {
            PlayServerbound::DebugSubscriptionRequest {
                subscription_ids: serverbound::decode_debug_subscription_request(&mut decoder)?,
            }
        }
        ids::play::SERVERBOUND_JIGSAW_GENERATE => {
            PlayServerbound::JigsawGenerate(jigsaw_blocks::decode_jigsaw_generate(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SET_COMMAND_BLOCK => PlayServerbound::SetCommandBlock(
            command_blocks::decode_set_command_block(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_SET_COMMAND_MINECART => PlayServerbound::SetCommandMinecart(
            command_blocks::decode_set_command_minecart(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_SET_JIGSAW_BLOCK => {
            PlayServerbound::SetJigsawBlock(jigsaw_blocks::decode_set_jigsaw_block(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SET_STRUCTURE_BLOCK => PlayServerbound::SetStructureBlock(
            structure_blocks::decode_set_structure_block(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_SET_TEST_BLOCK => {
            PlayServerbound::SetTestBlock(test_blocks::decode_set_test_block(&mut decoder)?)
        }
        ids::play::SERVERBOUND_TEST_INSTANCE_BLOCK_ACTION => {
            PlayServerbound::TestInstanceBlockAction(
                test_instance_blocks::decode_test_instance_block_action(&mut decoder)?,
            )
        }
        ids::play::SERVERBOUND_SPECTATE_ENTITY => {
            PlayServerbound::SpectateEntity(serverbound::decode_spectate_entity(&mut decoder)?)
        }
        ids::play::SERVERBOUND_TELEPORT_TO_ENTITY => {
            PlayServerbound::TeleportToEntity(serverbound::decode_teleport_to_entity(&mut decoder)?)
        }
        ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK => {
            PlayServerbound::PickItemFromBlock(PickItemFromBlock {
                pos: chunks::decode_block_pos(decoder.read_i64()?),
                include_data: decoder.read_bool()?,
            })
        }
        ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY => {
            PlayServerbound::PickItemFromEntity(PickItemFromEntity {
                entity_id: decoder.read_var_i32()?,
                include_data: decoder.read_bool()?,
            })
        }
        ids::play::SERVERBOUND_PLAYER_LOADED => PlayServerbound::PlayerLoaded,
        ids::play::SERVERBOUND_PLAYER_ACTION => {
            PlayServerbound::PlayerAction(decode_player_action(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SET_CARRIED_ITEM => PlayServerbound::SetCarriedItem {
            slot: decoder.read_i16()?,
        },
        ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT => {
            let slot_num = decoder.read_i16()?;
            let item =
                super::super::inventory::decode_item_stack_summary_with_framing_and_registry(
                    &mut decoder,
                    super::super::data_components::ComponentValueFraming::LengthPrefixed,
                    registry,
                )?;
            if item.count > 0
                && item
                    .item_id
                    .is_some_and(|item_id| item_id != crate::item_types::VANILLA_ITEM_AIR_ID)
            {
                serverbound::validate_creative_item_component_ranges(
                    &item.component_patch,
                    registry,
                )?;
            }
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot { slot_num, item })
        }
        ids::play::SERVERBOUND_SWING => PlayServerbound::Swing {
            hand: decode_hand(&mut decoder)?,
        },
        ids::play::SERVERBOUND_EDIT_BOOK => {
            PlayServerbound::EditBook(serverbound::decode_edit_book(&mut decoder)?)
        }
        ids::play::SERVERBOUND_PLACE_RECIPE => {
            PlayServerbound::PlaceRecipe(serverbound::decode_place_recipe(&mut decoder)?)
        }
        ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED => {
            PlayServerbound::SelectBundleItem(serverbound::decode_select_bundle_item(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SIGN_UPDATE => {
            PlayServerbound::SignUpdate(serverbound::decode_sign_update(&mut decoder)?)
        }
        ids::play::SERVERBOUND_MOVE_VEHICLE => {
            PlayServerbound::MoveVehicle(serverbound::decode_move_vehicle_command(&mut decoder)?)
        }
        ids::play::SERVERBOUND_PADDLE_BOAT => {
            PlayServerbound::PaddleBoat(serverbound::decode_paddle_boat(&mut decoder)?)
        }
        ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS => {
            PlayServerbound::RecipeBookChangeSettings(
                serverbound::decode_recipe_book_change_settings(&mut decoder)?,
            )
        }
        ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE => PlayServerbound::RecipeBookSeenRecipe(
            serverbound::decode_recipe_book_seen_recipe(&mut decoder)?,
        ),
        ids::play::SERVERBOUND_RENAME_ITEM => {
            PlayServerbound::RenameItem(serverbound::decode_rename_item(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SEEN_ADVANCEMENTS => {
            PlayServerbound::SeenAdvancements(serverbound::decode_seen_advancements(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SELECT_TRADE => {
            PlayServerbound::SelectTrade(serverbound::decode_select_trade(&mut decoder)?)
        }
        ids::play::SERVERBOUND_SET_BEACON => {
            PlayServerbound::SetBeacon(serverbound::decode_set_beacon(&mut decoder)?)
        }
        // `ServerboundUseItemPacket(FriendlyByteBuf)` (:25-30).
        ids::play::SERVERBOUND_USE_ITEM => PlayServerbound::UseItem(UseItem {
            hand: decode_hand(&mut decoder)?,
            sequence: decoder.read_var_i32()?,
            y_rot: decoder.read_f32()?,
            x_rot: decoder.read_f32()?,
        }),
        ids::play::SERVERBOUND_USE_ITEM_ON => {
            PlayServerbound::UseItemOn(decode_use_item_on(&mut decoder)?)
        }
        id => {
            return Err(ProtocolError::UnknownPacket {
                state: "play serverbound",
                id,
            })
        }
    };
    ensure_finished(&decoder, "play serverbound")?;
    Ok(packet)
}

/// `ServerboundChatCommandSignedPacket.STREAM_CODEC`
/// (ServerboundChatCommandSignedPacket.java:14-28).
fn decode_chat_command_signed(decoder: &mut Decoder<'_>) -> Result<ChatCommandSigned> {
    let command = decoder.read_string(super::super::serverbound::MAX_CHAT_COMMAND_CHARS)?;
    let timestamp_millis = decoder.read_i64()?;
    let salt = decoder.read_i64()?;
    let argument_signatures = decode_argument_signatures(decoder)?;
    let last_seen_messages = decode_last_seen_messages_update(decoder)?;
    Ok(ChatCommandSigned {
        command,
        timestamp_millis,
        salt,
        argument_signatures,
        last_seen_messages,
    })
}

/// `ArgumentSignatures.STREAM_CODEC` (ArgumentSignatures.java:13-41): at most
/// `MAX_ARGUMENT_COUNT` entries, each a name of at most `MAX_ARGUMENT_NAME_LENGTH`
/// characters and a fixed-width signature.
fn decode_argument_signatures(decoder: &mut Decoder<'_>) -> Result<ArgumentSignatures> {
    let count = decoder.read_len()?;
    if count > super::super::serverbound::MAX_ARGUMENT_SIGNATURE_COUNT {
        return Err(ProtocolError::PacketTooLarge(
            count,
            super::super::serverbound::MAX_ARGUMENT_SIGNATURE_COUNT,
        ));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let name =
            decoder.read_string(super::super::serverbound::MAX_ARGUMENT_SIGNATURE_NAME_CHARS)?;
        let bytes = decoder
            .read_exact(
                super::super::serverbound::MESSAGE_SIGNATURE_BYTES,
                "argument signature",
            )?
            .to_vec();
        entries.push(ArgumentSignature {
            name,
            signature: MessageSignature { bytes },
        });
    }
    Ok(ArgumentSignatures { entries })
}

/// `LastSeenMessages.Update.STREAM_CODEC` (LastSeenMessages.java:52-58).
fn decode_last_seen_messages_update(decoder: &mut Decoder<'_>) -> Result<LastSeenMessagesUpdate> {
    let offset = decoder.read_var_i32()?;
    let bytes = decoder.read_exact(
        super::super::serverbound::LAST_SEEN_MESSAGES_BITSET_BYTES,
        "last seen messages bitset",
    )?;
    let acknowledged = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
    let checksum = decoder.read_u8()?;
    Ok(LastSeenMessagesUpdate {
        offset,
        acknowledged,
        checksum,
    })
}

fn decode_chat(decoder: &mut Decoder<'_>) -> Result<ChatMessage> {
    let message = decoder.read_string(super::super::serverbound::MAX_CHAT_MESSAGE_CHARS)?;
    let timestamp_millis = decoder.read_i64()?;
    let salt = decoder.read_i64()?;
    let signature = decoder
        .read_bool()?
        .then(|| {
            decoder
                .read_exact(
                    super::super::serverbound::MESSAGE_SIGNATURE_BYTES,
                    "chat message signature",
                )
                .map(|bytes| MessageSignature {
                    bytes: bytes.to_vec(),
                })
        })
        .transpose()?;
    let last_seen_messages = decode_last_seen_messages_update(decoder)?;
    Ok(ChatMessage {
        message,
        timestamp_millis,
        salt,
        signature,
        last_seen_messages,
    })
}

/// `ServerboundContainerClickPacket.STREAM_CODEC`
/// (ServerboundContainerClickPacket.java:21-41).
///
/// The changed slots and the carried item are `HashedStack`s: the client's own
/// prediction of what the click produced, sent so a server that agrees can
/// stay silent. `ContainerInput.BY_ID` is `ByIdMap.continuous(..., ZERO)`, so
/// an id outside the enum decodes as `PICKUP` rather than failing the packet.
fn decode_container_click(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ContainerClick> {
    let container_id = decoder.read_var_i32()?;
    let state_id = decoder.read_var_i32()?;
    let slot_num = decoder.read_i16()?;
    let button_num = decoder.read_i8()?;
    let input = ContainerInput::from_id(decoder.read_var_i32()?);
    let count = decoder.read_var_i32()?;
    let count = usize::try_from(count)
        .ok()
        .filter(|count| *count <= super::super::serverbound::MAX_CONTAINER_CLICK_CHANGED_SLOTS)
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "container click changed slot count {count} exceeds vanilla maximum {}",
                super::super::serverbound::MAX_CONTAINER_CLICK_CHANGED_SLOTS
            ))
        })?;
    let mut changed_slots = BTreeMap::new();
    for _ in 0..count {
        let slot = decoder.read_i16()?;
        changed_slots.insert(slot, decode_hashed_stack(decoder, registry)?);
    }
    let carried_item = decode_hashed_stack(decoder, registry)?;
    Ok(ContainerClick {
        container_id,
        state_id,
        slot_num,
        button_num,
        input,
        changed_slots,
        carried_item,
    })
}

/// `HashedStack.STREAM_CODEC` (HashedStack.java:22-27), an optional
/// `ActualItem` that decodes to `EMPTY` when absent.
fn decode_hashed_stack(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<HashedStack> {
    if !decoder.read_bool()? {
        return Ok(HashedStack::Empty);
    }
    let item_id = super::super::inventory::decode_item_holder_id_with_registry(decoder, registry)?;
    let count = decoder.read_var_i32()?;
    Ok(HashedStack::Item(HashedItemStack {
        item_id,
        count,
        components: decode_hashed_component_patch(decoder)?,
    }))
}

/// `HashedPatchMap.STREAM_CODEC` (HashedPatchMap.java:17-23): the added
/// components carry a hash each, the removed ones carry only their type.
fn decode_hashed_component_patch(decoder: &mut Decoder<'_>) -> Result<HashedComponentPatch> {
    let mut patch = HashedComponentPatch::default();
    let added = decoder.read_var_i32()?;
    let added = hashed_patch_entry_count(added, "added")?;
    for _ in 0..added {
        let type_id = decoder.read_var_i32()?;
        patch.added_components.insert(type_id, decoder.read_i32()?);
    }
    let removed = decoder.read_var_i32()?;
    let removed = hashed_patch_entry_count(removed, "removed")?;
    for _ in 0..removed {
        patch.removed_components.insert(decoder.read_var_i32()?);
    }
    Ok(patch)
}

fn hashed_patch_entry_count(count: i32, label: &str) -> Result<usize> {
    usize::try_from(count)
        .ok()
        .filter(|count| *count <= super::super::serverbound::MAX_HASHED_COMPONENT_PATCH_ENTRIES)
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "hashed component {label} count {count} exceeds vanilla maximum {}",
                super::super::serverbound::MAX_HASHED_COMPONENT_PATCH_ENTRIES
            ))
        })
}

fn decode_player_action(decoder: &mut Decoder<'_>) -> Result<PlayerAction> {
    let ordinal = decoder.read_var_i32()?;
    let action = VANILLA_PLAYER_ACTION_KINDS
        .get(usize::try_from(ordinal).unwrap_or(usize::MAX))
        .map(|entry| entry.kind)
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!("invalid player action ordinal {ordinal}"))
        })?;
    Ok(PlayerAction {
        action,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
        direction: decode_direction_byte(decoder.read_u8()?)?,
        sequence: decoder.read_var_i32()?,
    })
}

fn decode_use_item_on(decoder: &mut Decoder<'_>) -> Result<UseItemOn> {
    Ok(UseItemOn {
        hand: decode_hand(decoder)?,
        hit: BlockHitResult {
            pos: chunks::decode_block_pos(decoder.read_i64()?),
            direction: decode_direction(decoder)?,
            cursor_x: decoder.read_f32()?,
            cursor_y: decoder.read_f32()?,
            cursor_z: decoder.read_f32()?,
            inside: decoder.read_bool()?,
            world_border_hit: decoder.read_bool()?,
        },
        sequence: decoder.read_var_i32()?,
    })
}

fn decode_hand(decoder: &mut Decoder<'_>) -> Result<InteractionHand> {
    let ordinal = decoder.read_var_i32()?;
    InteractionHand::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid interaction hand ordinal {ordinal}"))
    })
}

fn decode_direction(decoder: &mut Decoder<'_>) -> Result<Direction> {
    let id = decoder.read_var_i32()?;
    let id = u8::try_from(id)
        .map_err(|_| ProtocolError::InvalidData(format!("invalid direction id {id}")))?;
    decode_direction_byte(id)
}

fn decode_direction_byte(id: u8) -> Result<Direction> {
    match id {
        0 => Ok(Direction::Down),
        1 => Ok(Direction::Up),
        2 => Ok(Direction::North),
        3 => Ok(Direction::South),
        4 => Ok(Direction::West),
        5 => Ok(Direction::East),
        _ => Err(ProtocolError::InvalidData(format!(
            "invalid direction id {id}"
        ))),
    }
}

fn decode_position(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    })
}

fn decode_rotation(decoder: &mut Decoder<'_>) -> Result<PlayerRotation> {
    Ok(PlayerRotation {
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
    })
}

fn decode_move_flags(decoder: &mut Decoder<'_>) -> Result<(bool, bool)> {
    let flags = decoder.read_u8()?;
    Ok((flags & 1 != 0, flags & 2 != 0))
}
