use super::*;

pub(super) fn find_interaction_replay_plan(world: &WorldStore) -> Result<InteractionReplayPlan> {
    let position = world
        .local_player_pose()
        .context("interaction replay requires the canonical player pose")?
        .position;
    let base_x = position.x.floor() as i32;
    let base_y = position.y.floor() as i32;
    let base_z = position.z.floor() as i32;
    let mut surfaces = Vec::new();
    let offsets = [
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
        (1, -1),
    ];
    for (dx, dz) in offsets {
        for depth in 0..=6 {
            let surface = BlockPos {
                x: base_x + dx,
                y: base_y - depth,
                z: base_z + dz,
            };
            let Some(above_y) = surface.y.checked_add(1) else {
                continue;
            };
            let above = BlockPos {
                y: above_y,
                ..surface
            };
            let Some(air) = world.probe_block(above) else {
                continue;
            };
            // Surface decorations may be non-air but replaceable, which makes
            // a top-face placement replace the clicked cell instead of using
            // `above`. A full collision block
            // is the terrain-independent support every replay leg assumes.
            if world.sample_block_collision_shape_is_full_block(surface) == Some(true)
                && air.block_name.as_deref().is_some_and(block_name_is_air)
            {
                // Whether the note block leg fits above this surface. It needs
                // two more free cells than the place does: one for the note
                // block itself, which goes on top of the stone the replay
                // places, and one above that — `NoteBlock.playNote`
                // refuses to queue the event for a
                // base-block instrument unless the cell directly above the
                // note block is air, and every instrument a block underneath
                // can supply is a base-block one
                // (`NoteBlockInstrument.worksAboveNoteBlock`).
                let headroom = [2, 3].into_iter().all(|offset| {
                    surface
                        .y
                        .checked_add(offset)
                        .and_then(|y| world.probe_block(BlockPos { y, ..surface }))
                        .is_some_and(|block| {
                            block.block_name.as_deref().is_some_and(block_name_is_air)
                        })
                });
                surfaces.push((surface, above, headroom));
                break;
            }
        }
    }
    let (broken_block, _, _) = surfaces
        .first()
        .copied()
        .context("interaction replay found no nearby loaded buildable surface")?;
    let (clicked_block, placed_block, _) = surfaces
        .iter()
        .copied()
        .find(|(surface, _, headroom)| *headroom && *surface != broken_block)
        .context(
            "interaction replay requires a second nearby loaded surface block with three \
             air blocks above it",
        )?;
    let note_block = BlockPos {
        y: placed_block
            .y
            .checked_add(1)
            .context("interaction replay found no room above the block it places")?,
        ..placed_block
    };
    let drop_base = find_drop_surface(&surfaces, broken_block, clicked_block)
        .context("interaction replay requires a third nearby loaded surface block to drop from")?;
    let (redstone_clicked_block, redstone_wire_base, _) = surfaces
        .iter()
        .copied()
        .find(|(surface, _, headroom)| {
            *headroom
                && *surface != broken_block
                && *surface != clicked_block
                && *surface != drop_base
        })
        .context(
            "interaction replay requires a fourth nearby loaded surface with room for a wire base",
        )?;
    Ok(InteractionReplayPlan {
        broken_block,
        clicked_block,
        placed_block,
        note_block,
        drop_base,
        redstone_clicked_block,
        redstone_wire_base,
    })
}

/// The surface the drop leg stands its carpet on and then breaks.
///
/// It must be a third surface, because this leg destroys it — the broken block
/// is already air by the time the leg runs and has nothing to stand anything
/// on, and the clicked block is carrying the stone and the note block whose
/// state two later observations read.
///
/// It also needs the headroom the note-block leg's surface needs, for a
/// different reason: the session stands on the carpet to take the drop back,
/// and a player is taller than the cell it puts its feet in. Headroom asks for
/// one more cell than the body fills, which costs nothing here — every column
/// this gate's flat world offers has it.
pub(super) fn find_drop_surface(
    surfaces: &[(BlockPos, BlockPos, bool)],
    broken_block: BlockPos,
    clicked_block: BlockPos,
) -> Option<BlockPos> {
    surfaces
        .iter()
        .filter(|(_, _, headroom)| *headroom)
        .map(|(surface, _, _)| *surface)
        .find(|surface| *surface != broken_block && *surface != clicked_block)
}

/// The top of a carpet, measured from the bottom of the cell it is in.
///
/// `CarpetBlock`'s shape is `Block.column(16.0, 0.0, 1.0)`
/// whose arguments `Block.column`
/// hands to `Block.box` — which
/// reads them in sixteenths of a block.
pub(super) const CARPET_TOP: f64 = 1.0 / 16.0;

/// Where the session stands while the drop leg takes the carpet's support
/// away: the middle of the cell the carpet is in, standing on the carpet.
pub(super) fn drop_pickup_stand(drop_base: BlockPos) -> packets::Vec3d {
    packets::Vec3d {
        x: f64::from(drop_base.x) + 0.5,
        y: f64::from(drop_base.y + INTERACTION_DROP_CELL_OFFSET[1]) + CARPET_TOP,
        z: f64::from(drop_base.z) + 0.5,
    }
}

pub(super) async fn send_interaction_replay(
    conn: &mut RawConnection,
    world: &mut WorldStore,
    username: &str,
    profile_id: Uuid,
    client_information: &packets::ClientInformation,
    plan: InteractionReplayPlan,
) -> Result<()> {
    predict_interaction_replay_state(world)?;
    send_encoded(conn, packets::encode_play_set_carried_item(1)).await?;
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: 37,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: INTERACTION_HELD_STACK_COUNT,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    // A second stocked slot, which the container click at the end of the
    // replay acts on. It is not the selected slot, so the block interactions
    // below still run with the stack they expect.
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: INTERACTION_CLICKED_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: INTERACTION_CLICKED_STACK_COUNT,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_chat_message(&packets::ChatMessage::unsigned(
            INTERACTION_CHAT_MESSAGE,
            chrono::Utc::now().timestamp_millis(),
            0,
        )),
    )
    .await?;
    // Chat commands are their own inbound mechanism: the reply proves the
    // server parsed the command, reported through the vanilla translation key
    // and agrees with the other side about who is online.
    send_encoded(
        conn,
        packets::encode_play_chat_command(&packets::ChatCommand {
            command: INTERACTION_LIST_COMMAND.to_string(),
        }),
    )
    .await?;
    // The same command as a signed request, twice. `performSignedChatCommand`
    // (:1535-1550) matches the signatures against `SignableCommand.of` before
    // the dispatcher ever runs: `/list` reads no signable argument, so the
    // request that carries none runs the command a second time, and the one
    // that signs an argument the parse never read is refused whole with
    // `INVALID_COMMAND_SIGNATURE` (:237) and answers nothing else.
    send_encoded(
        conn,
        packets::encode_play_chat_command_signed(&packets::ChatCommandSigned::unsigned_arguments(
            INTERACTION_LIST_COMMAND,
            INTERACTION_SIGNED_COMMAND_TIMESTAMP_MILLIS,
            INTERACTION_SIGNED_COMMAND_SALT,
        )),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_chat_command_signed(&packets::ChatCommandSigned {
            argument_signatures: packets::ArgumentSignatures {
                entries: vec![packets::ArgumentSignature {
                    name: INTERACTION_SIGNED_COMMAND_ARGUMENT.to_string(),
                    signature: packets::MessageSignature {
                        bytes: vec![
                            INTERACTION_SIGNED_COMMAND_SIGNATURE_BYTE;
                            packets::MESSAGE_SIGNATURE_BYTES
                        ],
                    },
                }],
            },
            ..packets::ChatCommandSigned::unsigned_arguments(
                INTERACTION_LIST_COMMAND,
                INTERACTION_SIGNED_COMMAND_TIMESTAMP_MILLIS,
                INTERACTION_SIGNED_COMMAND_SALT,
            )
        }),
    )
    .await?;
    // The first of the two completion requests. `handleCustomCommandSuggestions`
    // (:588-601) answers every one of them, and what it answers is the
    // dispatcher's own `getCompletionSuggestions` — so the reply names the
    // commands the session may run and the range of the input they replace.
    let request = world.next_command_suggestion_request(INTERACTION_ROOT_SUGGESTION_COMMAND);
    send_encoded(
        conn,
        packets::encode_play_command_suggestion_request(request)?,
    )
    .await?;
    // The session-housekeeping surface (`ServerCommonPacketListenerImpl` plus
    // `handleClientInformation`): the ping is echoed, the rest are vanilla
    // no-ops whose evidence is that the session survives them. Resending the
    // configuration-state options keeps the model-part flag unchanged, so
    // neither server broadcasts a tab-list update for it.
    send_encoded(
        conn,
        packets::encode_play_ping_request(INTERACTION_PING_TIME),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_client_information(client_information)?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_custom_payload(&packets::ServerboundCustomPayload::Brand {
            brand: INTERACTION_CLIENT_BRAND.to_string(),
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_resource_pack_response(
            INTERACTION_RESOURCE_PACK_ID,
            ResourcePackResponseAction::Declined,
        ),
    )
    .await?;
    // The other two silent members of that surface. `handlePong`
    // is empty, and
    // `handleCustomClickAction` forwards to a server method whose
    // whole body is a debug log — neither server defines a built-in action —
    // so the evidence for both is that the session survives them.
    send_encoded(conn, packets::encode_play_pong(INTERACTION_PONG_ID)).await?;
    send_encoded(
        conn,
        packets::encode_play_custom_click_action(&packets::CustomClickAction {
            id: INTERACTION_CUSTOM_CLICK_ACTION.to_string(),
            payload: None,
        })?,
    )
    .await?;
    // The session-authority requests. All three are things a client may ask for
    // and the server owns: the two difficulty ones are gamemaster-gated
    // (`handleChangeDifficulty`/`handleLockDifficulty`) and this probe is never
    // an operator, and `handlePlayerAbilities` never echoes. So the visible
    // result of all three is that nothing changes — which the block-place
    // acknowledgement below orders after them.
    send_encoded(
        conn,
        packets::encode_play_change_difficulty(packets::ChangeDifficultyCommand {
            difficulty: INTERACTION_REQUESTED_DIFFICULTY,
        }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_lock_difficulty(packets::LockDifficultyCommand { locked: true }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_player_abilities(packets::PlayerAbilitiesCommand { flying: true }),
    )
    .await?;
    send_encoded(conn, packets::encode_play_swing(InteractionHand::MainHand)).await?;
    send_encoded(
        conn,
        packets::encode_play_player_action(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: plan.broken_block,
            direction: Direction::Up,
            sequence: 1,
        }),
    )
    .await?;
    send_encoded(conn, packets::encode_play_swing(InteractionHand::MainHand)).await?;
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.clicked_block,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 2,
        }),
    )
    .await?;
    // Natural spawn may land on leaves, whose collision top is spawn-safe but
    // not sturdy enough for dust. Give the wire its own stone support.
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.redstone_clicked_block,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: INTERACTION_REDSTONE_BASE_PLACE_SEQUENCE,
        }),
    )
    .await?;
    // The block-event leg. It runs here, after the place above and before the
    // picks below, and both halves of that are load-bearing. It needs the
    // stone: the note block goes on top of it, and `NoteBlock.setInstrument`
    // reads that block to pick the instrument, so
    // standing the note block on something the replay placed itself is what
    // makes the instrument — and the sound the event plays — the same on two
    // servers whose worlds are not. And it must come before the picks, which
    // end by moving the selection to the slot holding stone: anything that
    // changes the selection afterwards would make the pick's own result
    // unreadable. The slot it stocks is one no later observation reads.
    //
    // The item is looked up by the resource id the placed block is compared
    // against rather than spelled as a number.
    let note_block_item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(INTERACTION_NOTE_BLOCK_NAME)
            .context("the generated item catalog names no note block")?;
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: INTERACTION_NOTE_BLOCK_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(note_block_item.id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_carried_item(i16::from(INTERACTION_NOTE_BLOCK_INVENTORY_SLOT)),
    )
    .await?;
    if !world.set_local_selected_hotbar_slot(INTERACTION_NOTE_BLOCK_INVENTORY_SLOT) {
        anyhow::bail!("interaction replay could not select the note block hotbar slot");
    }
    // The place, aimed at the top face of the stone. `BlockItem.useOn` builds
    // the state through `NoteBlock.getStateForPlacement`,
    // which reads the instrument off the block underneath — the stone, whose
    // `Properties.instrument` is the same on both servers.
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.placed_block,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: INTERACTION_NOTE_BLOCK_PLACE_SEQUENCE,
        }),
    )
    .await?;
    // The right-click on the note block itself, still holding a note block.
    // `NoteBlock.useItemOn` only passes for an item
    // in `ItemTags.NOTE_BLOCK_TOP_INSTRUMENTS` — the seven heads
    // which a note block is not — so
    // it falls through to `BlockBehaviour.useItemOn`
    // and answers `TRY_WITH_EMPTY_HAND`.
    // `ServerPlayerGameMode.useItemOn` then runs `useWithoutItem`
    // which cycles the note, writes the state back
    // and queues the block event. The replay never sneaks, so the
    // `suppressUsingBlock` gate is open on both servers.
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.note_block,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: INTERACTION_NOTE_BLOCK_TUNE_SEQUENCE,
        }),
    )
    .await?;
    // Redstone-wire leg: runs while surrounding cells are clear so connection state reflects a lone cross.
    let redstone_wire_item = bbb_protocol::item_types::vanilla_item_type_for_resource_id(
        INTERACTION_REDSTONE_WIRE_ITEM_NAME,
    )
    .context("the generated item catalog names no redstone dust")?;
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: INTERACTION_REDSTONE_WIRE_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(redstone_wire_item.id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_carried_item(i16::from(INTERACTION_REDSTONE_WIRE_INVENTORY_SLOT)),
    )
    .await?;
    if !world.set_local_selected_hotbar_slot(INTERACTION_REDSTONE_WIRE_INVENTORY_SLOT) {
        anyhow::bail!("interaction replay could not select the redstone wire hotbar slot");
    }
    // The place, aimed at the top face of the fourth surface, which puts the
    // dust in the cell above it.
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.redstone_wire_base,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: INTERACTION_REDSTONE_WIRE_PLACE_SEQUENCE,
        }),
    )
    .await?;
    // Drop leg: breaking the support block drops the carpet via neighbor shape update even in creative mode.
    let drop_item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(INTERACTION_DROP_BLOCK_NAME)
            .context("the generated item catalog names no drop-leg block")?;
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: INTERACTION_DROP_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(drop_item.id),
                count: INTERACTION_DROP_ITEM_COUNT,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_carried_item(i16::from(INTERACTION_DROP_INVENTORY_SLOT)),
    )
    .await?;
    if !world.set_local_selected_hotbar_slot(INTERACTION_DROP_INVENTORY_SLOT) {
        anyhow::bail!("interaction replay could not select the drop-leg hotbar slot");
    }
    // The place, aimed at the top face of the base, which puts the carpet in
    // the cell above it. `CarpetBlock.canSurvive` is
    // satisfied there because the base is not empty.
    send_encoded(
        conn,
        packets::encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: plan.drop_base,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: INTERACTION_DROP_PLACE_SEQUENCE,
        }),
    )
    .await?;
    // Step onto the carpet cell so any dropped item stays inside the 1-block horizontal pickup box through the 10-tick delay.
    let stand = drop_pickup_stand(plan.drop_base);
    let pose = world
        .local_player_pose()
        .context("the drop leg needs the session's own pose to keep its facing")?;
    send_encoded(
        conn,
        packets::encode_play_move_player_pos_rot(
            stand.x, stand.y, stand.z, pose.y_rot, pose.x_rot, true, false,
        ),
    )
    .await?;
    // The server never answers an accepted move, so the client's own model is
    // the only record that it happened. Later legs read it: the peer session is
    // sent the primary's position to walk to, and a stale one would aim it at
    // the cell the session left.
    world.set_local_player_pose(LocalPlayerPoseState {
        position: stand,
        on_ground: true,
        ..pose
    });

    // And the break that takes the support away. A creative session breaks in
    // one packet — `handleBlockBreakAction` destroys and acknowledges on
    // `START_DESTROY_BLOCK` — so this is the whole of it, and the shape update
    // it triggers runs before the handler returns
    // (`CollectingNeighborUpdater.addAndRun`; `CollectingNeighborUpdater`
    // drains its own stack when nothing else is already draining it).
    send_encoded(
        conn,
        packets::encode_play_player_action(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: plan.drop_base,
            direction: Direction::Up,
            sequence: INTERACTION_DROP_BREAK_SEQUENCE,
        }),
    )
    .await?;
    // The picks, which read the inventory `findSlotMatchingItem` scans. Both
    // servers stock slots 1 and 2 from the replay's own creative writes, but
    // what a fresh session starts with below them is each server's own
    // business — so the replay clears the slot in front of them and owns the
    // whole answer.
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: FIRST_HOTBAR_MENU_SLOT,
            item: ItemStackSummary::empty(),
        })?,
    )
    .await?;
    // Parking on an empty slot is what makes the pick readable: the answer is
    // the selection moving off it, and a server that ignored the packet would
    // leave it here.
    send_encoded(
        conn,
        packets::encode_play_set_carried_item(i16::from(INTERACTION_PICK_PARKED_SLOT)),
    )
    .await?;
    if !world.set_local_selected_hotbar_slot(INTERACTION_PICK_PARKED_SLOT) {
        anyhow::bail!("interaction replay could not park on the pick hotbar slot");
    }
    // Two picks that must answer nothing. The broken block is air, whose
    // `getCloneItemStack` is empty (`Item.BY_BLOCK` has no air entry), and
    // `Entity.getPickResult` is null for every entity this world has — the
    // probe aims at itself so the target is in reach by construction and no
    // second session is needed. `includeData` is set on both so the
    // block-entity and profile-print branches are the ones declining.
    send_encoded(
        conn,
        packets::encode_play_pick_item_from_block(packets::PickItemFromBlock {
            pos: plan.broken_block,
            include_data: true,
        }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_pick_item_from_entity(packets::PickItemFromEntity {
            entity_id: world
                .local_player_id()
                .context("the pick replay needs the session's own entity id")?,
            include_data: true,
        }),
    )
    .await?;
    // And the pick that must: the stone just placed resolves to the stone item
    // through `Item.BY_BLOCK`, which `findSlotMatchingItem` finds in the
    // hotbar slot the replay stocked.
    send_encoded(
        conn,
        packets::encode_play_pick_item_from_block(packets::PickItemFromBlock {
            pos: plan.placed_block,
            include_data: false,
        }),
    )
    .await?;
    // The right-click that hits no block. `handleUseItem` acknowledges the
    // sequence before it looks at the stack, and the stone in hand answers
    // `PASS`, so the acknowledgement below is the whole of it — and it is the
    // last one the replay sends, which is what makes it readable.
    send_encoded(
        conn,
        packets::encode_play_use_item(packets::UseItem {
            hand: InteractionHand::MainHand,
            sequence: INTERACTION_USE_ITEM_SEQUENCE,
            // `Mth.wrapDegrees` folds this back into (-180, 180], so a server
            // that stored it verbatim would be aiming somewhere else.
            y_rot: INTERACTION_USE_ITEM_UNWRAPPED_Y_ROT,
            x_rot: 0.0,
        }),
    )
    .await?;
    // Acknowledging nothing. The window starts full and this replay is never
    // sent a player chat message on either server, so zero is the only offset
    // both of them can afford — anything else ends the session.
    send_encoded(
        conn,
        packets::encode_play_chat_acknowledgement(packets::ChatAcknowledgement { offset: 0 }),
    )
    .await?;
    // Asking for things about this session and about the world. The respawn
    // is a no-op for a living player, the statistics answer is what a server
    // that counts nothing has to say, and the game rule readout and the two
    // writes below are gamemaster-gated like the block editors are.
    for (id, payload) in [
        packets::encode_play_perform_respawn(),
        packets::encode_play_client_command(packets::ClientCommandAction::RequestStats),
        packets::encode_play_client_command(packets::ClientCommandAction::RequestGameRuleValues),
        packets::encode_play_change_game_mode(packets::ChangeGameModeCommand {
            game_mode: packets::GameType::Survival,
        }),
    ] {
        send_encoded(conn, (id, payload)).await?;
    }
    send_encoded(
        conn,
        packets::encode_play_set_game_rule(&packets::SetGameRule {
            entries: vec![packets::SetGameRuleEntry {
                rule: PERMISSION_REFUSED_GAME_RULE.to_string(),
                value: "false".to_string(),
            }],
        })?,
    )
    .await?;
    // Everything the session is not allowed to do. `handleSetCommandBlock`
    // and `handleSetCommandMinecart` answer the refusal with
    // `advMode.notAllowed`; the other ten are silent, either because their
    // gate returns without a word or because the block entity behind it does
    // not exist. Every one of them is aimed at something real — a loaded
    // block, this session's own avatar — so a server that passed the gate
    // would have something to act on and the silence would break.
    let own_entity_id = world
        .local_player_id()
        .context("the permission replay needs the session's own entity id")?;
    // `handleTeleportToEntityPacket` names its target by profile id, which is
    // the one this session logged in with.
    for (id, payload) in [
        packets::encode_play_block_entity_tag_query(packets::BlockEntityTagQuery {
            transaction_id: PERMISSION_QUERY_TRANSACTION_ID,
            pos: plan.clicked_block,
        }),
        packets::encode_play_entity_tag_query(packets::EntityTagQuery {
            transaction_id: PERMISSION_QUERY_TRANSACTION_ID + 1,
            entity_id: own_entity_id,
        }),
        packets::encode_play_jigsaw_generate(packets::JigsawGenerate {
            pos: plan.clicked_block,
            levels: 1,
            keep_jigsaws: false,
        }),
        packets::encode_play_spectate_entity(packets::SpectateEntity {
            entity_id: own_entity_id,
        }),
        packets::encode_play_teleport_to_entity(packets::TeleportToEntity { uuid: profile_id }),
    ] {
        send_encoded(conn, (id, payload)).await?;
    }
    send_encoded(
        conn,
        packets::encode_play_set_command_block(&packets::SetCommandBlock {
            pos: plan.clicked_block,
            command: PERMISSION_REFUSED_COMMAND.into(),
            mode: packets::CommandBlockMode::Redstone,
            track_output: true,
            conditional: false,
            automatic: false,
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_command_minecart(&packets::SetCommandMinecart {
            entity_id: own_entity_id,
            command: PERMISSION_REFUSED_COMMAND.into(),
            track_output: true,
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_jigsaw_block(&packets::SetJigsawBlock {
            pos: plan.clicked_block,
            name: "minecraft:bottom".to_string(),
            target: "minecraft:top".to_string(),
            pool: "minecraft:empty".to_string(),
            final_state: "minecraft:air".into(),
            joint: packets::JigsawJoint::Aligned,
            selection_priority: 0,
            placement_priority: 0,
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_structure_block(&packets::SetStructureBlock {
            pos: plan.clicked_block,
            update: packets::StructureBlockUpdateKind::SaveArea,
            mode: packets::StructureBlockMode::Save,
            name: PERMISSION_REFUSED_STRUCTURE.to_string(),
            offset: packets::BlockPos { x: 0, y: 1, z: 0 },
            size: packets::Vec3i { x: 2, y: 2, z: 2 },
            mirror: packets::StructureBlockMirror::None,
            rotation: packets::StructureBlockRotation::None,
            data: Default::default(),
            ignore_entities: true,
            strict: false,
            show_air: false,
            show_bounding_box: true,
            integrity: 1.0,
            seed: 0,
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_test_block(&packets::SetTestBlock {
            pos: plan.clicked_block,
            mode: packets::TestBlockMode::Accept,
            message: PERMISSION_REFUSED_COMMAND.into(),
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_test_instance_block_action(&packets::TestInstanceBlockAction {
            pos: plan.clicked_block,
            action: packets::TestInstanceBlockActionKind::Run,
            test: None,
            size: packets::Vec3i { x: 1, y: 1, z: 1 },
            rotation: packets::TestInstanceBlockRotation::None,
            ignore_entities: false,
        })?,
    )
    .await?;
    // `handleDebugSubscriptionRequest` keeps the set whatever the session may
    // do with it, so an empty request is the one that stays a no-op on a
    // server that honours the permission behind reading it.
    send_encoded(conn, packets::encode_play_debug_subscription_request(&[])?).await?;
    // The requests whose vanilla handler finds nothing to act on. Each names
    // something real — a mob effect the beacon screen offers, an offer index, a
    // finite vehicle pose, an oar — so a server that had the menu, the vehicle
    // or the recipe display behind it would act and the silence would break.
    // `handleRecipeBookSeenRecipePacket` is the one aimed at a value both
    // servers must reject the same way: `RecipeManager.getRecipeFromDisplay`
    // (:179-182) bounds-checks the index, and a negative one is out of bounds
    // of any display list, including the one the official server publishes.
    for (id, payload) in [
        packets::encode_play_rename_item(&packets::RenameItem {
            name: INTERACTION_ANVIL_NAME.to_string(),
        }),
        packets::encode_play_select_trade(packets::SelectTradeCommand { item: 0 }),
        packets::encode_play_move_vehicle(packets::MoveVehicleCommand {
            position: packets::Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: true,
        }),
        packets::encode_play_paddle_boat(packets::PaddleBoat {
            left: true,
            right: true,
        }),
        packets::encode_play_recipe_book_change_settings(
            packets::RecipeBookChangeSettingsCommand {
                book_type: packets::RecipeBookType::Smoker,
                open: true,
                filtering: true,
            },
        ),
        packets::encode_play_recipe_book_seen_recipe(packets::RecipeBookSeenRecipeCommand {
            recipe: packets::RecipeDisplayId {
                index: INTERACTION_UNRESOLVABLE_RECIPE_DISPLAY,
            },
        }),
        // The same unresolvable display index, this time aimed at the open
        // menu: `handlePlaceRecipe` (:1954-1962) resolves it before it reaches
        // the menu, so both servers stop at the same place.
        packets::encode_play_place_recipe(packets::PlaceRecipeCommand {
            container_id: bbb_client_world::INVENTORY_MENU_CONTAINER_ID,
            recipe_index: INTERACTION_UNRESOLVABLE_RECIPE_DISPLAY,
            use_max_items: true,
        }),
        // A bundle selection aimed at the stocked slot. The stack there is
        // stone rather than a bundle, so `toggleSelectedItem` finds no
        // `BUNDLE_CONTENTS` to write — and even on a bundle the selection is
        // one `BundleContents.STREAM_CODEC` (:29-31) drops.
        packets::encode_play_select_bundle_item(packets::SelectBundleItem {
            slot_id: i32::from(INTERACTION_CLICKED_MENU_SLOT),
            selected_item_index: 0,
        }),
    ] {
        send_encoded(conn, (id, payload)).await?;
    }
    // The writable book the second wave edits, stocked here so the stack is in
    // place a whole tick before the first edit arrives. See
    // [`send_interaction_book_edits`] for why that separation is required.
    send_encoded(
        conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: INTERACTION_BOOK_MENU_SLOT,
            item: ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        })?,
    )
    .await?;
    // The sign editor, aimed at the block the replay placed. Neither server
    // has a block entity there — `updateSignText` (:2033-2044) needs a
    // `SignBlockEntity` — so the lines go nowhere.
    send_encoded(
        conn,
        packets::encode_play_sign_update(&packets::SignUpdate {
            pos: plan.placed_block,
            is_front_text: true,
            lines: [
                INTERACTION_SIGN_LINE.to_string(),
                String::new(),
                String::new(),
                String::new(),
            ],
        }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_set_beacon(packets::SetBeacon {
            primary_effect: Some(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID),
            secondary_effect: None,
        })?,
    )
    .await?;
    // The advancement tabs, which are the one request in this block that is
    // not silent. `setSelectedTab` answers
    // whenever the selection moved: the closed screen and the id that names no
    // advancement must move nothing, the root with a display must be taken, and
    // the loaded advancement that is not a root must clear it again.
    for (id, payload) in [
        packets::encode_play_seen_advancements(&packets::SeenAdvancements::ClosedScreen),
        packets::encode_play_seen_advancements(&packets::SeenAdvancements::OpenedTab {
            tab: INTERACTION_UNKNOWN_ADVANCEMENT_TAB.to_string(),
        }),
        packets::encode_play_seen_advancements(&packets::SeenAdvancements::OpenedTab {
            tab: INTERACTION_ROOT_ADVANCEMENT_TAB.to_string(),
        }),
        packets::encode_play_seen_advancements(&packets::SeenAdvancements::OpenedTab {
            tab: INTERACTION_CHILD_ADVANCEMENT_TAB.to_string(),
        }),
    ] {
        send_encoded(conn, (id, payload)).await?;
    }
    // One click against the always-open player inventory, last so nothing
    // after it depends on what the cursor now holds. The replay predicts
    // nothing — no changed slots and an empty cursor — so both servers find
    // their remote copy stale and answer with what the click actually
    // produced, which is exactly the authority the differential compares.
    send_encoded(
        conn,
        packets::encode_play_container_click(packets::ContainerClick {
            container_id: bbb_client_world::INVENTORY_MENU_CONTAINER_ID,
            state_id: world.inventory().inventory_menu_state_id(),
            slot_num: INTERACTION_CLICKED_MENU_SLOT,
            button_num: 0,
            input: packets::ContainerInput::Pickup,
            changed_slots: std::collections::BTreeMap::new(),
            carried_item: packets::HashedStack::empty(),
        })?,
    )
    .await?;
    // The rest of the container session. The button and the crafter slot
    // toggle only reach a menu that declares them — `clickMenuButton` is false
    // for every menu that does not override it and
    // `handleContainerSlotStateChanged` needs a `CrafterMenu` — so on the one
    // menu this session has, both are silent. The close is not: it runs
    // `InventoryMenu.removed`, which hands the stack the click left on the
    // cursor back to the inventory, and that is a result the differential can
    // read straight out of the held slot.
    send_encoded(
        conn,
        packets::encode_play_container_button_click(packets::ContainerButtonClick {
            container_id: bbb_client_world::INVENTORY_MENU_CONTAINER_ID,
            button_id: 0,
        }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_container_slot_state_changed(packets::ContainerSlotStateChanged {
            slot_id: INTERACTION_CLICKED_MENU_SLOT.into(),
            container_id: bbb_client_world::INVENTORY_MENU_CONTAINER_ID,
            new_state: true,
        }),
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_container_close(packets::ContainerCloseRequest {
            container_id: bbb_client_world::INVENTORY_MENU_CONTAINER_ID,
        }),
    )
    .await?;
    tracing::debug!(%username, ?plan, "sent normalized interaction replay");
    Ok(())
}

pub(super) fn predict_interaction_replay_state(world: &mut WorldStore) -> Result<()> {
    if !world.set_local_selected_hotbar_slot(1) {
        anyhow::bail!("interaction replay could not select canonical hotbar slot 1");
    }
    let mut hotbar = world.creative_hotbar_preset_snapshot().map_err(|error| {
        anyhow::anyhow!(
            "interaction replay could not snapshot the canonical creative hotbar: {error:?}"
        )
    })?;
    *hotbar
        .get_mut(1)
        .context("interaction replay canonical creative hotbar is missing slot 1")? =
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: INTERACTION_HELD_STACK_COUNT,
            component_patch: DataComponentPatchSummary::default().into(),
        };
    world
        .load_local_creative_hotbar_preset(&hotbar)
        .map_err(|error| {
            anyhow::anyhow!(
                "interaction replay could not update the canonical creative hotbar: {error:?}"
            )
        })?;
    Ok(())
}

/// Sends book save and sign edits after wave 1 completes so creative slot stock is already committed.
pub(crate) async fn send_interaction_book_edits(
    conn: &mut RawConnection,
    world: &mut WorldStore,
) -> Result<()> {
    // The second completion request rides this wave because the client keeps
    // only one outstanding one: asking past a node the first request could not
    // reach is what shows the answer follows the parse rather than the root.
    let request = world.next_command_suggestion_request(INTERACTION_CHILD_SUGGESTION_COMMAND);
    send_encoded(
        conn,
        packets::encode_play_command_suggestion_request(request)?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_edit_book(&packets::EditBook {
            slot: i32::from(INTERACTION_BOOK_INVENTORY_SLOT),
            pages: vec![INTERACTION_BOOK_PAGE.to_string()],
            title: None,
        })?,
    )
    .await?;
    send_encoded(
        conn,
        packets::encode_play_edit_book(&packets::EditBook {
            slot: i32::from(INTERACTION_BOOK_INVENTORY_SLOT),
            pages: vec![INTERACTION_BOOK_PAGE.to_string()],
            title: Some(INTERACTION_BOOK_TITLE.to_string()),
        })?,
    )
    .await
}

pub(super) async fn send_encoded(conn: &mut RawConnection, packet: (i32, Vec<u8>)) -> Result<()> {
    conn.send_packet(packet.0, &packet.1).await
}

/// The input a tick barrier asks the server to complete.
///
/// Every node it can reach is a literal, so the dispatcher's future is already
/// complete when `handleCustomCommandSuggestions` chains onto it and the answer
/// is sent from the packet phase itself rather than from a later one.
pub(super) const TICK_BARRIER_COMMAND: &str = "tick ";

/// The completion request one tick barrier is answered by.
///
/// A leg that closes its rounds on a chat command closes them on nothing: a
/// chat packet is never handed to the packet processor at all
/// (`handleChatCommand`), it goes
/// straight onto the server's own task queue (`tryHandleChat`),
/// and `waitUntilNextTick` drains that queue for the whole gap between two
/// ticks. So such a fence can be answered ahead
/// of a packet the leg sent before it and ahead of every tick. A completion
/// request instead waits for the packet phase
/// (`handleCustomCommandSuggestions`, ServerGamePacketListenerImpl.java:
/// 589-601, via `PacketUtils.ensureRunningOnSameThread`,
/// PacketUtils, which the phase drains from
/// MinecraftServer), which runs the leg's earlier packets first
/// and cannot run before the next tick has started.
pub(super) use packets::encode_play_command_suggestion_request as tick_barrier_request;

/// Records the latest completed command suggestion when a new request has arrived.
pub(super) fn record_command_suggestion(
    world: &mut WorldStore,
    observed: &mut Vec<CommandSuggestionObservation>,
) {
    let Some(result) = world.last_command_suggestions() else {
        return;
    };
    let Some(request) = world.command_suggestions().last_request.as_ref() else {
        return;
    };
    if request.id != result.id || observed.iter().any(|seen| seen.command == request.command) {
        return;
    }
    observed.push(CommandSuggestionObservation {
        command: request.command.clone(),
        start: result.start,
        length: result.length,
        suggestions: result
            .suggestions
            .iter()
            .map(|suggestion| suggestion.text.clone())
            .collect(),
    });
}

pub(super) fn observe_interaction_replay(
    world: &WorldStore,
    plan: InteractionReplayPlan,
    command_suggestions: &[CommandSuggestionObservation],
    positioned_sounds: &[SoundEventState],
    block_event_sounds_before: &[usize],
    spawned_entities: &BTreeMap<i32, SpawnedEntitySnapshot>,
    taken_item_entities: &BTreeSet<i32>,
) -> InteractionReplayReport {
    let chat_observed = world
        .client_chat()
        .messages
        .iter()
        .any(|message| message.content.contains(INTERACTION_CHAT_MESSAGE));
    let list_command_replies = world
        .client_chat()
        .messages
        .iter()
        .filter(|message| message.content.contains(LIST_COMMAND_TRANSLATION_KEY))
        .collect::<Vec<_>>();
    let list_command_response = list_command_replies
        .first()
        .map(|message| message.content.clone());
    let invalid_command_signature_refusals = world
        .client_chat()
        .messages
        .iter()
        .filter(|message| {
            message
                .content
                .contains(INVALID_COMMAND_SIGNATURE_TRANSLATION_KEY)
        })
        .count();
    let hotbar = world.inventory().hotbar_items();
    let hotbar_slot = world.local_player().selected_hotbar_slot;
    let item = &hotbar[usize::from(hotbar_slot)];
    let cursor = world.inventory().cursor_item.clone();
    let clicked = world
        .inventory()
        .inventory_menu_slot_item(INTERACTION_CLICKED_MENU_SLOT)
        .cloned()
        .unwrap_or_else(ItemStackSummary::empty);
    let parked = &hotbar[usize::from(INTERACTION_PICK_PARKED_SLOT)];
    let book = world
        .inventory()
        .inventory_menu_slot_item(INTERACTION_BOOK_MENU_SLOT)
        .cloned()
        .unwrap_or_else(ItemStackSummary::empty);
    let book_content = book.component_patch.written_book.as_ref();
    let permission_refusals = world
        .client_chat()
        .messages
        .iter()
        .filter(|message| message.content.contains(COMMAND_BLOCK_NOT_ALLOWED_KEY))
        .count();
    let note_block = world.probe_block(plan.note_block);
    // The cell the wire leg placed into: directly above its replay-owned stone.
    let redstone_wire = plan.redstone_wire_base.y.checked_add(1).and_then(|y| {
        world.probe_block(BlockPos {
            y,
            ..plan.redstone_wire_base
        })
    });
    let block_events = world
        .block_events()
        .iter()
        .enumerate()
        .map(|(index, event)| BlockEventObservation {
            block_name: world
                .registries()
                .block_name_by_registry_id(event.block_id)
                .map(str::to_string),
            param_a: event.b0,
            param_b: event.b1,
            at_note_block: event.pos == plan.note_block,
            // An event the caller never stamped is one that arrived outside
            // the session loop this observation is taken in, so every sound
            // the loop saw came first.
            positioned_sounds_before: block_event_sounds_before
                .get(index)
                .copied()
                .unwrap_or(positioned_sounds.len()),
        })
        .collect();
    // `Level.playSeededSound` is handed the block's centre
    // (`NoteBlock.triggerEvent`, NoteBlock: `pos.getX() + 0.5`
    // and the same for the other two axes), which is what "came from the note
    // block" has to mean.
    let note_block_center = [
        f64::from(plan.note_block.x) + 0.5,
        f64::from(plan.note_block.y) + 0.5,
        f64::from(plan.note_block.z) + 0.5,
    ];
    let sounds_heard = positioned_sounds
        .iter()
        .map(|sound| note_block_sound_observation(sound, note_block_center))
        .collect();
    let observed_spawns = spawned_entities
        .iter()
        .map(|(id, snapshot)| spawned_entity_observation(*id, snapshot, plan.drop_base))
        .collect::<Vec<_>>();
    let drop_slot = world
        .inventory()
        .inventory_menu_slot_item(INTERACTION_DROP_MENU_SLOT)
        .cloned()
        .unwrap_or_else(ItemStackSummary::empty);
    InteractionReplayReport {
        chat_content: if chat_observed {
            INTERACTION_CHAT_MESSAGE
        } else {
            Default::default()
        }
        .to_string(),
        chat_observed,
        broken_block: plan.broken_block,
        broken_block_name: world
            .probe_block(plan.broken_block)
            .and_then(|block| block.block_name),
        placed_block: plan.placed_block,
        placed_block_name: world
            .probe_block(plan.placed_block)
            .and_then(|block| block.block_name),
        hotbar_slot,
        hotbar_item_id: item.item_id,
        hotbar_item_count: item.count,
        acknowledged_sequence: world.last_block_changed_ack().map(|ack| ack.sequence),
        list_command_response,
        list_command_responses: list_command_replies.len(),
        invalid_command_signature_refusals,
        command_suggestions: command_suggestions.to_vec(),
        ping_response_time: world.last_pong_response().map(|response| response.time),
        container_click_cursor_item_id: cursor.item_id,
        container_click_cursor_item_count: cursor.count,
        container_click_slot_item_id: clicked.item_id,
        container_click_slot_item_count: clicked.count,
        pick_parked_slot_item_id: parked.item_id,
        pick_parked_slot_item_count: parked.count,
        award_stats_packets: world.counters().award_stats_packets,
        awarded_statistics: world.counters().award_stats_entries_received,
        permission_refusals,
        permission_level: format!("{:?}", world.local_player_permission_level()),
        advancement_tab_packets: world.counters().select_advancements_tab_packets,
        advancement_tab_selected: world.advancements().selected_tab.clone(),
        book_item_id: book.item_id,
        book_title: book_content.map(|content| content.title.to_display_string()),
        book_author: book_content.map(|content| content.author.to_display_string()),
        book_generation: book_content.map(|content| content.generation),
        book_resolved: book_content.map(|content| content.resolved),
        book_pages: book_content
            .map(|content| content.pages.clone())
            .unwrap_or_default(),
        note_block_name: note_block
            .as_ref()
            .and_then(|block| block.block_name.clone()),
        note_block_properties: note_block
            .map(|block| block.block_properties)
            .unwrap_or_default(),
        redstone_wire_name: redstone_wire
            .as_ref()
            .and_then(|block| block.block_name.clone()),
        redstone_wire_properties: redstone_wire
            .map(|block| block.block_properties)
            .unwrap_or_default(),
        block_events,
        sounds_heard,
        spawned_entities: observed_spawns.clone(),
        spawned_entity_count: observed_spawns.len(),
        drop_observed: observed_spawns
            .iter()
            .any(SpawnedEntityObservation::is_the_leg_drop),
        // Only the leg's own drop counts. Another item entity being taken
        // says nothing about whether this one was, and the whole point of the
        // wait above is that this one is not left to chance.
        drop_picked_up: observed_spawns
            .iter()
            .filter(|spawn| spawn.is_the_leg_drop())
            .any(|spawn| taken_item_entities.contains(&spawn.entity_id)),
        drop_slot_item_id: drop_slot.item_id,
        drop_slot_item_count: drop_slot.count,
    }
}

/// Snapshots the server-owned session authority, or `None` until the server has
/// announced both halves of it.
pub(super) fn observe_session_authority(world: &WorldStore) -> Option<SessionAuthorityReport> {
    let counters = world.counters();
    // The client derives abilities from the login game mode as well, so the
    // packet count — not the presence of a value — is what proves the server
    // announced them.
    if counters.change_difficulty_packets == 0 || counters.player_abilities_packets == 0 {
        return None;
    }
    let difficulty = world.difficulty();
    Some(SessionAuthorityReport {
        difficulty: difficulty.difficulty.clone(),
        difficulty_locked: difficulty.difficulty_locked,
        difficulty_updates: counters.change_difficulty_packets,
        abilities: world.local_player().abilities?,
        ability_updates: counters.player_abilities_packets,
    })
}

/// The rule [`ProbeContext::drop_pickup_settled`] runs, split out so it can be
/// asked without a connection: the whole state is one optional deadline and
/// the two facts it is measured against.
///
/// The first call arms the deadline and answers no, so the wait always gets at
/// least one look at the world after it started.
pub(super) fn drop_pickup_wait_ended(
    deadline: &mut Option<i64>,
    drop_finished: bool,
    game_time: Option<i64>,
) -> bool {
    if drop_finished {
        return true;
    }
    // A session that was never told the server's tick count has no clock to
    // bound the wait on, and waiting on the wall instead would put a real
    // difference behind however busy each machine was.
    let Some(now) = game_time else {
        return true;
    };
    match *deadline {
        Some(at) => now >= at,
        None => {
            *deadline = Some(now.saturating_add(DROP_PICKUP_TICK_BUDGET));
            false
        }
    }
}

/// The stack an item entity is carrying, or `None` for anything that is not
/// one.
///
/// The accessor is resolved through the generated metadata catalog rather than
/// by a spelled index, which is also what decides whether the entity is an
/// item entity at all: the catalog answers only for the type whose `DATA_ITEM`
/// is `ItemEntity`'s own.
pub(super) fn item_entity_stack(state: &bbb_client_world::EntityState) -> Option<ItemStackSummary> {
    let data_id =
        bbb_client_world::vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            state.entity_type_id,
        )?;
    state.data_values.iter().find_map(|value| {
        if value.data_id != data_id {
            return None;
        }
        match &value.value {
            packets::EntityDataValueKind::ItemStack(stack) => Some(stack.clone()),
            _ => None,
        }
    })
}

/// Projects one recorded spawn into what the differential compares.
///
/// The cell is taken with a floor rather than a truncation, because the drop
/// leg's base can sit either side of the origin and `Mth.floor` is what
/// vanilla addresses blocks with.
pub(super) fn spawned_entity_observation(
    id: i32,
    snapshot: &SpawnedEntitySnapshot,
    drop_base: BlockPos,
) -> SpawnedEntityObservation {
    let item = snapshot.item.as_ref();
    SpawnedEntityObservation {
        entity_type: bbb_protocol::entity_types::vanilla_entity_type(snapshot.entity_type_id)
            .map(|entry| entry.resource_id.to_string()),
        cell_offset: [
            (snapshot.position[0].floor() as i32) - drop_base.x,
            (snapshot.position[1].floor() as i32) - drop_base.y,
            (snapshot.position[2].floor() as i32) - drop_base.z,
        ],
        item_id: item
            .and_then(|stack| stack.item_id)
            .and_then(bbb_protocol::item_types::vanilla_item_type)
            .map(|entry| entry.resource_id.to_string()),
        item_count: item.map_or(0, |stack| stack.count),
        entity_id: id,
        uuid: snapshot.uuid.to_string(),
        position: snapshot.position,
        delta_movement: snapshot.delta_movement,
    }
}
