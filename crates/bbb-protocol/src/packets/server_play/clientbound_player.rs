use super::*;

/// `ClientboundTickingStatePacket.write`
/// (`ClientboundTickingStatePacket.java:22-25`): the tick rate as a float and the
/// frozen flag as a boolean, read back in the same order (`:14-16`).
///
/// `from` copies both fields straight out of a `TickRateManager` (`:18-20`), so
/// what this carries is `tickrate()` and `isFrozen()`
/// (`TickRateManager.java:20-22`, `:52-54`). The senders are
/// `ServerTickRateManager.updateStateToClients` (`ServerTickRateManager.java:33-35`),
/// reached from the `setFrozen` override (`:28-31`, behind `/tick freeze` and
/// `/tick unfreeze`, `TickCommand.java:109-129`) and the `setTickRate` override
/// (`ServerTickRateManager.java:115-119`, behind `/tick rate`,
/// `TickCommand.java:61-66`), plus the join-time
/// resend in `updateJoiningPlayer` (`ServerTickRateManager.java:121-124`).
///
/// There is no rejectable input here, so this encoder has no failure mode and
/// returns the payload directly: `writeFloat` and `writeBoolean` carry every `f32`
/// and every `bool`, and the rate's floor is a clamp rather than a wire limit.
/// `TickRateManager.setTickRate` applies `Math.max(rate, 1.0F)` against
/// `MIN_TICKRATE` (`TickRateManager.java:8`, `:15-18`), and
/// `ClientPacketListener.handleTickingState` pushes the received rate through that
/// same setter (`ClientPacketListener.java:703-710`), so an under-1 rate arrives
/// clamped instead of refused -- the 1.0..=10000.0 band is only the `/tick rate`
/// argument's range (`TickCommand.java:28`).
pub fn encode_play_clientbound_ticking_state(update: TickingState) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_f32(update.tick_rate);
    encoder.write_bool(update.frozen);
    (ids::play::CLIENTBOUND_TICKING_STATE, encoder.into_inner())
}

/// `ClientboundTickingStepPacket.write`
/// (`ClientboundTickingStepPacket.java:22-24`): the step count as a single var
/// int, read back the same way (`:14-16`).
///
/// `from` reads `TickRateManager.frozenTicksToRun` (`:18-20`,
/// `TickRateManager.java:44-46`), the counter that `tick` drains by one per tick
/// while it is positive (`TickRateManager.java:56-61`) and that the client stores
/// through `setFrozenTicksToRun` (`TickRateManager.java:40-42`,
/// `ClientPacketListener.java:713-719`). Both senders are step transitions on
/// `ServerTickRateManager.updateStepTicks` (`ServerTickRateManager.java:37-39`):
/// `stepGameIfPaused` arms the counter (`:41-49`, behind `/tick step`,
/// `TickCommand.java:131-141`) and `stopStepping` clears it to zero (`:51-59`,
/// behind `/tick step stop`, `TickCommand.java:143-153`); joining players get the
/// current value as well (`ServerTickRateManager.java:121-124`).
///
/// There is no rejectable input here either, so this encoder has no failure mode:
/// a var int carries every `i32` (negatives as the five byte form), so no step
/// count is unrepresentable. `/tick step` never asks for one below 1 because its
/// argument is `TimeArgument.time(1)` (`TickCommand.java:38`) and `stopStepping`
/// sends exactly 0, but that is the command surface's range, not the packet's.
pub fn encode_play_clientbound_ticking_step(update: TickingStep) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.tick_steps);
    (ids::play::CLIENTBOUND_TICKING_STEP, encoder.into_inner())
}

/// `ClientboundAwardStatsPacket` (ClientboundAwardStatsPacket.java:12-18), the
/// answer `ServerStatsCounter.sendStats` (:126-134) gives a client that asked
/// for its statistics.
pub fn encode_play_clientbound_award_stats(update: &AwardStats) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(
        i32::try_from(update.stats.len())
            .map_err(|_| ProtocolError::InvalidData("too many statistics to award".to_string()))?,
    );
    for stat in &update.stats {
        encoder.write_var_i32(stat.stat_type_id);
        encoder.write_var_i32(stat.value_id);
        encoder.write_var_i32(stat.amount);
    }
    Ok((ids::play::CLIENTBOUND_AWARD_STATS, encoder.into_inner()))
}

/// `ClientboundEntityEventPacket` (ClientboundEntityEventPacket.java:14-24),
/// whose only server-side use here is
/// `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:559-572).
pub fn encode_play_clientbound_entity_event(update: EntityEvent) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i32(update.entity_id);
    encoder.write_i8(update.event_id);
    (ids::play::CLIENTBOUND_ENTITY_EVENT, encoder.into_inner())
}

/// `ClientboundHurtAnimationPacket.write` (`ClientboundHurtAnimationPacket.java:22-25`).
pub fn encode_play_clientbound_hurt_animation(
    update: super::super::HurtAnimation,
) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_f32(update.yaw);
    (ids::play::CLIENTBOUND_HURT_ANIMATION, encoder.into_inner())
}

/// `ClientboundGameRuleValuesPacket.STREAM_CODEC`
/// (`ClientboundGameRuleValuesPacket.java:15-18`): `ByteBufCodecs.map`'s count,
/// then each rule's identifier and serialized value, both `STRING_UTF8`
/// (`ByteBufCodecs.java:168`).
pub fn encode_play_clientbound_game_rule_values(update: &GameRuleValues) -> Result<(i32, Vec<u8>)> {
    let count = i32::try_from(update.values.len())
        .map_err(|_| ProtocolError::InvalidData("game rule value count exceeds i32".to_string()))?;
    let mut encoder = Encoder::new();
    encoder.write_var_i32(count);
    for entry in &update.values {
        if entry.rule.encode_utf16().count() > GAME_RULE_VALUES_STRING_MAX_CHARS
            || entry.value.encode_utf16().count() > GAME_RULE_VALUES_STRING_MAX_CHARS
        {
            return Err(ProtocolError::InvalidData(format!(
                "game rule value for {} exceeds {GAME_RULE_VALUES_STRING_MAX_CHARS} chars",
                entry.rule
            )));
        }
        encoder.write_string(&normalize_resource_location(entry.rule.clone())?);
        encoder.write_string(&entry.value);
    }
    Ok((
        ids::play::CLIENTBOUND_GAME_RULE_VALUES,
        encoder.into_inner(),
    ))
}

/// `ClientboundTagQueryPacket.write` (`ClientboundTagQueryPacket.java:27-30`):
/// the transaction id, then the tag exactly as [`TagQuery::from_tag`] wrote it.
pub fn encode_play_clientbound_tag_query(update: &TagQuery) -> Result<(i32, Vec<u8>)> {
    if super::super::client_features::validate_tag_query_nbt(&update.raw_nbt)? != update.tag_present
    {
        return Err(ProtocolError::InvalidData(
            "tag query presence disagrees with its nbt root".to_string(),
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.transaction_id);
    encoder.write_bytes(&update.raw_nbt);
    Ok((ids::play::CLIENTBOUND_TAG_QUERY, encoder.into_inner()))
}

/// `ClientboundAnimatePacket.write` (ClientboundAnimatePacket.java:31-34): the
/// entity id as a var int and the action as one byte.
///
/// The action is one of the five the class names (`:13-17`); the constructor
/// takes the id off the entity it was built against (`:21-24`). The producer
/// this server has is `LivingEntity.swing` (LivingEntity.java:1993-2008), which
/// picks `SWING_MAIN_HAND` or `SWING_OFF_HAND` from the hand it was handed
/// (`:1999`).
pub fn encode_play_clientbound_animate(update: EntityAnimation) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.id);
    encoder.write_u8(update.action);
    (ids::play::CLIENTBOUND_ANIMATE, encoder.into_inner())
}

/// `ClientboundSetCameraPacket.write` (ClientboundSetCameraPacket.java:25-27):
/// the camera entity's id as a var int, taken off the entity the packet was
/// built against (`:17-19`).
///
/// `ServerPlayer.setCamera` (ServerPlayer.java:1916-1931) is the only sender,
/// and it never sends a null camera: the field it reads is `this.camera`, which
/// the line above set to the player itself when the argument was null (`:1918`).
pub fn encode_play_clientbound_set_camera(update: SetCamera) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.camera_id);
    (ids::play::CLIENTBOUND_SET_CAMERA, encoder.into_inner())
}

/// `ClientboundPlayerRotationPacket.STREAM_CODEC`
/// (ClientboundPlayerRotationPacket.java:10-20): the yaw, whether it is
/// relative, the pitch, and whether that is relative, in that order.
///
/// `ServerPlayer.forceSetRotation` (ServerPlayer.java:1138-1141) is the only
/// sender, and it forwards the four values it was called with rather than the
/// absolute rotation `Entity.forceSetRotation` (Entity.java:3091-3100) just
/// computed -- so the client applies the same relative arithmetic itself.
pub fn encode_play_clientbound_player_rotation(update: PlayerRotationUpdate) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_f32(update.y_rot);
    encoder.write_bool(update.relative_y);
    encoder.write_f32(update.x_rot);
    encoder.write_bool(update.relative_x);
    (ids::play::CLIENTBOUND_PLAYER_ROTATION, encoder.into_inner())
}

/// `ClientboundTransferPacket.write` (ClientboundTransferPacket.java:17-20):
/// the host as a string and the port as a var int.
///
/// `writeUtf` bounds the string at 32767 characters
/// (FriendlyByteBuf.java:572-574), which `Utf8String.write` refuses anything
/// above (Utf8String.java:35-37). A host longer than that is refused here for
/// the same reason, rather than being truncated onto the wire.
pub fn encode_play_clientbound_transfer(update: &Transfer) -> Result<(i32, Vec<u8>)> {
    Ok((
        ids::play::CLIENTBOUND_TRANSFER,
        super::super::connection::encode_transfer_payload(update)?,
    ))
}

/// `ClientboundSelectAdvancementsTabPacket.STREAM_CODEC`
/// (`ClientboundSelectAdvancementsTabPacket.java:14-16`): an optional
/// advancement id, absent when the selection was cleared.
pub fn encode_play_clientbound_select_advancements_tab(
    update: &SelectAdvancementsTab,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    match &update.tab {
        Some(tab) => {
            encoder.write_bool(true);
            encoder.write_string(&normalize_resource_location(tab.clone())?);
        }
        None => encoder.write_bool(false),
    }
    Ok((
        ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB,
        encoder.into_inner(),
    ))
}

pub fn encode_play_clientbound_set_held_slot(update: SetHeldSlot) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.slot);
    (ids::play::CLIENTBOUND_SET_HELD_SLOT, encoder.into_inner())
}

/// `ClientboundCooldownPacket.STREAM_CODEC` (ClientboundCooldownPacket.java:11-17):
/// the cooldown group as an `Identifier`, then the duration as a var int.
pub fn encode_play_clientbound_cooldown(update: &Cooldown) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_string(&update.cooldown_group);
    encoder.write_var_i32(update.duration);
    (ids::play::CLIENTBOUND_COOLDOWN, encoder.into_inner())
}

/// `ClientboundUpdateMobEffectPacket.write`
/// (`ClientboundUpdateMobEffectPacket.java:58-64`). The effect id is validated
/// on the way out for the same reason the decode validates it on the way in: a
/// holder the registry does not know would put a packet on the wire no client
/// can read.
pub fn encode_play_clientbound_update_mob_effect(
    update: &UpdateMobEffect,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    encoder.write_var_i32(super::super::client_state::validate_mob_effect_holder_id(
        update.effect_id,
    )?);
    encoder.write_var_i32(update.amplifier);
    encoder.write_var_i32(update.duration_ticks);
    encoder.write_u8(update.flags.to_bits());
    Ok((
        ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT,
        encoder.into_inner(),
    ))
}

/// `ClientboundRemoveMobEffectPacket.STREAM_CODEC`
/// (`ClientboundRemoveMobEffectPacket.java:15-21`).
pub fn encode_play_clientbound_remove_mob_effect(
    update: &RemoveMobEffect,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    encoder.write_var_i32(super::super::client_state::validate_mob_effect_holder_id(
        update.effect_id,
    )?);
    Ok((
        ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT,
        encoder.into_inner(),
    ))
}

/// `ClientboundSetPlayerInventoryPacket`, whose one vanilla owner is
/// `Inventory.placeItemBackInInventory` (Inventory.java:307-328): it reports
/// the single inventory slot a returning stack landed in, outside any menu
/// numbering.
pub fn encode_play_clientbound_set_player_inventory(
    update: &SetPlayerInventory,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_set_player_inventory_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_set_player_inventory_with_content_registry(
    update: &SetPlayerInventory,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.slot);
    super::super::serverbound::encode_item_stack_summary_with_content_registry(
        &mut encoder,
        &update.item,
        super::super::data_components::ComponentValueFraming::Direct,
        registry,
    )?;
    Ok((
        ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
        encoder.into_inner(),
    ))
}

/// `ClientboundContainerSetContentPacket`, which the container synchronizer
/// sends from `AbstractContainerMenu.sendAllDataToRemote`
/// (AbstractContainerMenu.java:157-177). This is how the vanilla server
/// publishes a whole menu, the always-open player inventory included.
pub fn encode_play_clientbound_container_set_content(
    update: &ContainerSetContent,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_container_set_content_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_container_set_content_with_content_registry(
    update: &ContainerSetContent,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.container_id);
    encoder.write_var_i32(update.state_id);
    encoder.write_var_i32(i32::try_from(update.items.len()).map_err(|_| {
        ProtocolError::InvalidData(format!("container holds {} slots", update.items.len()))
    })?);
    for item in &update.items {
        super::super::serverbound::encode_item_stack_summary_with_content_registry(
            &mut encoder,
            item,
            super::super::data_components::ComponentValueFraming::Direct,
            registry,
        )?;
    }
    super::super::serverbound::encode_item_stack_summary_with_content_registry(
        &mut encoder,
        &update.carried_item,
        super::super::data_components::ComponentValueFraming::Direct,
        registry,
    )?;
    Ok((
        ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
        encoder.into_inner(),
    ))
}

/// `ClientboundContainerSetDataPacket`, which
/// `ServerPlayer.containerSynchronizer.sendDataChange` (ServerPlayer.java:309-311)
/// sends for one `ContainerData` field.
pub fn encode_play_clientbound_container_set_data(update: ContainerSetData) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.container_id);
    encoder.write_i16(update.id);
    encoder.write_i16(update.value);
    (
        ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
        encoder.into_inner(),
    )
}

/// `ClientboundContainerSetSlotPacket`, which
/// `ServerPlayer.containerSynchronizer.sendSlotChange` (ServerPlayer.java:299-301)
/// sends for one slot whose contents the client's own copy no longer matches.
pub fn encode_play_clientbound_container_set_slot(
    update: &ContainerSetSlot,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_container_set_slot_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_container_set_slot_with_content_registry(
    update: &ContainerSetSlot,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.container_id);
    encoder.write_var_i32(update.state_id);
    encoder.write_i16(update.slot);
    super::super::serverbound::encode_item_stack_summary_with_content_registry(
        &mut encoder,
        &update.item,
        super::super::data_components::ComponentValueFraming::Direct,
        registry,
    )?;
    Ok((
        ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
        encoder.into_inner(),
    ))
}

/// `ClientboundOpenScreenPacket.STREAM_CODEC`
/// (`ClientboundOpenScreenPacket.java:14-22`): the container id, the
/// `Registries.MENU` id of the menu type, and the title over the trusted
/// context-free component codec.
///
/// `ServerPlayer.openMenu` (`ServerPlayer.java:1337`) sends this *before*
/// `initMenu`, so the screen the client opens is always the one the
/// `ClientboundContainerSetContentPacket` that follows fills in.
///
/// `ByteBufCodecs.CONTAINER_ID` (`ByteBufCodecs.java:200-208`) is
/// `FriendlyByteBuf.writeContainerId`, a var int, and
/// `ByteBufCodecs.registry(Registries.MENU)` (`:580-582`) writes the raw
/// registry id the generated [`crate::menu_types`] catalog carries.
pub fn encode_play_clientbound_open_screen(update: &OpenScreen) -> Result<(i32, Vec<u8>)> {
    if crate::menu_types::vanilla_menu_type(update.menu_type_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid vanilla menu type id {}",
            update.menu_type_id
        )));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.container_id);
    encoder.write_var_i32(update.menu_type_id);
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.title.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((ids::play::CLIENTBOUND_OPEN_SCREEN, encoder.into_inner()))
}

/// `ClientboundContainerClosePacket`, which `ServerPlayer.closeContainer`
/// (`ServerPlayer.java:1399-1402`) sends before `doCloseContainer` — the
/// server-initiated half of a close. The client-initiated half
/// (`ServerboundContainerClosePacket`) reaches `doCloseContainer` directly and
/// sends nothing back.
pub fn encode_play_clientbound_container_close(update: ContainerClose) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.container_id);
    (ids::play::CLIENTBOUND_CONTAINER_CLOSE, encoder.into_inner())
}

/// `ClientboundSetCursorItemPacket`, which
/// `ServerPlayer.containerSynchronizer.sendCarriedChange` (ServerPlayer.java:304-306)
/// sends for the stack the click left on the cursor. It carries no container
/// id or state id: the cursor belongs to the player, not to a menu.
pub fn encode_play_clientbound_set_cursor_item(update: &SetCursorItem) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_set_cursor_item_with_content_registry(
        update,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_set_cursor_item_with_content_registry(
    update: &SetCursorItem,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    super::super::serverbound::encode_item_stack_summary_with_content_registry(
        &mut encoder,
        &update.item,
        super::super::data_components::ComponentValueFraming::Direct,
        registry,
    )?;
    Ok((ids::play::CLIENTBOUND_SET_CURSOR_ITEM, encoder.into_inner()))
}

/// `ClientboundPongResponsePacket`: the echo
/// `ServerGamePacketListenerImpl.handlePingRequest` sends straight back on the
/// connection, carrying the client's own timestamp.
pub fn encode_play_clientbound_pong_response(response: PongResponse) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(response.time);
    (ids::play::CLIENTBOUND_PONG_RESPONSE, encoder.into_inner())
}

pub fn encode_play_clientbound_system_chat(update: &SystemChat) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.content.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    encoder.write_bool(update.overlay);
    Ok((ids::play::CLIENTBOUND_SYSTEM_CHAT, encoder.into_inner()))
}

/// `ChatType.Bound` as the server binds it (ChatType.java:75-89): a registry
/// holder, the sender name and the optional target name.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundChatType {
    pub chat_type_id: i32,
    pub name: RichTextComponent,
    pub target_name: Option<RichTextComponent>,
}

/// `ClientboundPlayerChatPacket` for an unsigned message
/// (`PlayerChatMessage.unsigned`, PlayerChatMessage.java:46-50): link index 0,
/// no signature, salt 0, no last-seen entries, `FilterMask.PASS_THROUGH`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsignedPlayerChat {
    pub global_index: i32,
    pub sender: Uuid,
    pub content: String,
    pub timestamp_millis: i64,
    pub unsigned_content: Option<RichTextComponent>,
    pub chat_type: BoundChatType,
}

/// `ClientboundDisguisedChatPacket` (ClientboundDisguisedChatPacket.java:9-14).
#[derive(Debug, Clone, PartialEq)]
pub struct DisguisedChatMessage {
    pub message: RichTextComponent,
    pub chat_type: BoundChatType,
}

fn write_component(encoder: &mut Encoder, component: &RichTextComponent) -> Result<()> {
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(component.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok(())
}

fn write_bound_chat_type(encoder: &mut Encoder, bound: &BoundChatType) -> Result<()> {
    encoder.write_var_i32(bound.chat_type_id + 1);
    write_component(encoder, &bound.name)?;
    encoder.write_bool(bound.target_name.is_some());
    if let Some(target) = &bound.target_name {
        write_component(encoder, target)?;
    }
    Ok(())
}

/// `ClientboundPlayerChatPacket.write` (ClientboundPlayerChatPacket.java:44-53).
pub fn encode_play_clientbound_player_chat(chat: &UnsignedPlayerChat) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(chat.global_index);
    encoder.write_uuid(chat.sender);
    encoder.write_var_i32(0);
    encoder.write_bool(false);
    encoder.write_string(&chat.content);
    encoder.write_i64(chat.timestamp_millis);
    encoder.write_i64(0);
    encoder.write_var_i32(0);
    encoder.write_bool(chat.unsigned_content.is_some());
    if let Some(content) = &chat.unsigned_content {
        write_component(&mut encoder, content)?;
    }
    encoder.write_var_i32(0);
    write_bound_chat_type(&mut encoder, &chat.chat_type)?;
    Ok((ids::play::CLIENTBOUND_PLAYER_CHAT, encoder.into_inner()))
}

pub fn encode_play_clientbound_disguised_chat(
    chat: &DisguisedChatMessage,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    write_component(&mut encoder, &chat.message)?;
    write_bound_chat_type(&mut encoder, &chat.chat_type)?;
    Ok((ids::play::CLIENTBOUND_DISGUISED_CHAT, encoder.into_inner()))
}

/// `ClientboundDisconnectPacket.STREAM_CODEC`
/// (`ClientboundDisconnectPacket.java:11-12`): the reason alone, over the
/// trusted context-free component codec — so the whole payload is the NBT
/// component.
pub fn encode_play_clientbound_disconnect(reason: &RichTextComponent) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(reason.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((ids::play::CLIENTBOUND_DISCONNECT, encoder.into_inner()))
}

/// `ClientboundSetTimePacket` (`ClientboundSetTimePacket.java:14-21`): i64 gameTime,
/// then the clock-update map (varint length; per entry the WORLD_CLOCK registry id
/// varint, `ClockNetworkState` var-long totalTicks + f32 partialTick + f32 rate).
pub fn encode_play_clientbound_set_time(update: &PlayTime) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_i64(update.game_time);
    write_len(
        &mut encoder,
        update.clock_updates.len(),
        "set time clock updates",
    )?;
    for clock in &update.clock_updates {
        encoder.write_var_i32(clock.clock_id);
        encoder.write_var_i64(clock.total_ticks);
        encoder.write_f32(clock.partial_tick);
        encoder.write_f32(clock.rate);
    }
    Ok((ids::play::CLIENTBOUND_SET_TIME, encoder.into_inner()))
}

/// `ClientboundSetHealthPacket.write` (`ClientboundSetHealthPacket.java:28-32`):
/// the health as a float, the food level as a varint and the saturation as a
/// second float.
///
/// The one sender is `ServerPlayer.doTick` (`ServerPlayer.java:671-678`), which
/// compares the three against the last ones it sent and only writes the packet
/// when one of them moved -- so the packet carries whatever the caller decided
/// is now current rather than re-deriving anything. Food and saturation travel
/// with it because vanilla has one packet for all three, not because this
/// server models hunger: `FoodData`'s defaults are what the caller passes.
pub fn encode_play_clientbound_set_health(update: PlayerHealth) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_f32(update.health);
    encoder.write_var_i32(update.food);
    encoder.write_f32(update.saturation);
    (ids::play::CLIENTBOUND_SET_HEALTH, encoder.into_inner())
}

/// `ClientboundSetExperiencePacket.write`
/// (`ClientboundSetExperiencePacket.java:28-32`): the progress as a float, then
/// the *level* and only then the total, both as varints.
///
/// The field order on the wire is not the constructor's
/// (`:16-20`, which takes progress, total, level), and the difference is
/// invisible to a reader who trusts the constructor -- so it is transcribed off
/// `write` and pinned by a round trip against the decoder.
///
/// The one sender is `ServerPlayer.doTick` (`ServerPlayer.java:710-713`), which
/// writes the packet only when `totalExperience` moved.
pub fn encode_play_clientbound_set_experience(update: PlayerExperience) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_f32(update.progress);
    encoder.write_var_i32(update.level);
    encoder.write_var_i32(update.total);
    (ids::play::CLIENTBOUND_SET_EXPERIENCE, encoder.into_inner())
}

/// `ClientboundDamageEventPacket.write`
/// (`ClientboundDamageEventPacket.java:50-60`): the hurt entity, the damage type
/// as a registry id, the two entity ids the source names and an optional
/// position.
///
/// The two entity ids go out through `writeOptionalEntityId` (`:42-44`), which
/// writes `id + 1` so that a zero means "no entity"; the decoder here already
/// subtracts the one back off (`client_state::decode_damage_event`), so the
/// struct holds vanilla's own `-1` for absent and this adds the offset back.
///
/// `sourcePosition` is `DamageSource.sourcePositionRaw` (`DamageSource.java:112-114`)
/// rather than `getSourcePosition` (`:104-110`): only a source built with an
/// explicit position carries one, and a source that merely has a direct entity
/// writes nothing and lets the client read the position off that entity.
pub fn encode_play_clientbound_damage_event(update: DamageEvent) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.entity_id);
    encoder.write_var_i32(update.source_type_id);
    encoder.write_var_i32(update.source_cause_id + 1);
    encoder.write_var_i32(update.source_direct_id + 1);
    encoder.write_bool(update.source_position.is_some());
    if let Some(position) = update.source_position {
        encode_position(&mut encoder, position);
    }
    (ids::play::CLIENTBOUND_DAMAGE_EVENT, encoder.into_inner())
}

/// `ClientboundPlayerCombatKillPacket.STREAM_CODEC`
/// (`ClientboundPlayerCombatKillPacket.java:12-18`): the dead player's entity id
/// as a varint, then the death message through `ComponentSerialization.TRUSTED_STREAM_CODEC`.
///
/// The trusted codec is the same context-free NBT component
/// `encode_play_clientbound_system_chat` writes, which is why the message is
/// bounded the same way rather than through the item-bearing registry codec.
///
/// `ServerPlayer.die` (`ServerPlayer.java:869-929`) is the only sender, and it
/// sends the packet on both sides of the `showDeathMessages` game rule: the
/// composed death message when the rule is on (`:876`) and `CommonComponents.EMPTY`
/// when it is off (`:899`). So an empty message is a message, not an absent one.
pub fn encode_play_clientbound_player_combat_kill(
    update: &PlayerCombatKill,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.player_id);
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(update.message.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok((
        ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL,
        encoder.into_inner(),
    ))
}
