//! Commands typed into chat, and the management surface that applies the same
//! effects: difficulty, game rules, op, ban, kick, whitelist, game mode.

use super::*;

#[test]
fn long_wire_commands_reach_inventory_and_signed_message_dispatch() {
    use bbb_protocol::packets::{
        decode_play_serverbound, encode_play_chat_command, encode_play_chat_command_signed,
        ChatCommand,
    };

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let probe = "😀".repeat(300);
    for (signed, count) in [(false, 2), (true, 3)] {
        let command = format!("item replace entity alice hotbar.0 with minecraft:stone[minecraft:custom_data={{probe:\"{probe}\"}}] {count}");
        let (id, payload) = if signed {
            encode_play_chat_command_signed(&ChatCommandSigned::unsigned_arguments(command, 0, 0))
        } else {
            encode_play_chat_command(&ChatCommand { command })
        };
        let packet = decode_play_serverbound(id, &payload).unwrap();
        let result = play(&sender, &receiver, &mut world, 1, packet);
        assert_eq!(result.snapshot.players.len(), 1);
        let item = native_stack(result.snapshot.players[0].inventory.slots[0].clone());
        assert_eq!(item.count(), count);
        let resolver = world.item_stack_resolver();
        let data = item
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                &resolver,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            crate::loot_runtime::nbt_field(&data, "probe"),
            Some(&NbtSummaryValue::string(probe.clone()))
        );
    }

    let command = format!(
        "execute {}run say signed-marker",
        "positioned ~ ~ ~ ".repeat(20)
    );
    let packet = ChatCommandSigned {
        command,
        timestamp_millis: 0,
        salt: 0,
        argument_signatures: ArgumentSignatures {
            entries: vec![ArgumentSignature {
                name: "message".into(),
                signature: MessageSignature {
                    bytes: vec![7; 256],
                },
            }],
        },
        last_seen_messages: LastSeenMessagesUpdate::default(),
    };
    let (id, payload) = encode_play_chat_command_signed(&packet);
    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        decode_play_serverbound(id, &payload).unwrap(),
    );
    assert_eq!(result.snapshot.players.len(), 1);
    // `collectSignedArguments` decodes the argument as an unsigned message
    // from alice, so the say reaches her as player chat under her UUID.
    let say_command = world
        .synchronized_registries()
        .registry_element_index("minecraft:chat_type", "minecraft:say_command");
    let announcement = result
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerChat(chat),
            } => Some(chat),
            _ => None,
        })
        .expect("the signed say broadcasts player chat");
    assert_eq!(announcement.sender, uuid::Uuid::from_u128(1));
    assert_eq!(announcement.content, "signed-marker");
    assert_eq!(Some(announcement.chat_type.chat_type_id), say_command);
    assert_eq!(announcement.chat_type.name.text(), "alice");
}

#[test]
fn chat_commands_run_through_the_single_dispatcher_and_reply_to_the_sender() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    settle_joined_bodies(&mut world);

    // `/list` is LEVEL_ALL and reports the live player list, so it doubles as
    // a check that the dispatcher sees the same players the tracker does.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "list".into(),
            },
        },
    )
    .unwrap();
    let listed = world.tick(receiver.drain());
    // `CommandSourceStack.sendSuccess` reaches the source only: bob sees
    // nothing.
    assert_eq!(listed.events.len(), 1);
    let WorldEvent::Send {
        connection_id,
        packet: ClientboundPlayPacket::SystemChat(update),
    } = &listed.events[0]
    else {
        panic!("expected a system chat reply to the sender");
    };
    assert_eq!(*connection_id, ConnectionId(1));
    // The reply is the vanilla translatable component, not rendered English:
    // the client owns the language, so the flattened text is the key plus the
    // component-valued arguments.
    assert_eq!(
        update.content.text(),
        "commands.list.players alice, bob",
        "{:?}",
        update.content.component()
    );

    // An unknown command still replies to the sender rather than closing the
    // connection or going silent. `Commands.finishParsing` sends the error and
    // the cursor context as two separate messages.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "nosuchcommand".into(),
            },
        },
    )
    .unwrap();
    let unknown = world.tick(receiver.drain());
    assert_eq!(unknown.events.len(), 2);
    let texts = unknown
        .events
        .iter()
        .map(|event| {
            let WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(update),
            } = event
            else {
                panic!("expected a system chat reply to the sender");
            };
            update.content.text().to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(texts[0], "command.unknown.command");
    assert_eq!(texts[1], "nosuchcommandcommand.context.here");
    assert_eq!(unknown.snapshot.players.len(), 2);
}

#[test]
fn recipe_commands_mutate_live_books_and_send_one_packet_per_target() {
    let (sender, receiver) = mailbox();
    let startup = Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    );
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::clone(&startup));
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let recipe_id = "minecraft:flint_and_steel";

    let given = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("recipe give bob {recipe_id}"),
    );
    let add = given
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::RecipeBookAdd(add),
            } => Some(add),
            _ => None,
        })
        .expect("the target receives one recipe-book add");
    assert!(!add.replace);
    assert_eq!(add.entries.len(), 1);
    assert_eq!(
        add.entries[0].contents.id,
        startup.recipe_displays.displays_for_recipe(recipe_id)[0].id
    );
    assert!(add.entries[0].notification);
    assert!(add.entries[0].highlight);
    assert!(world.player_knows_recipe_for_test(ConnectionId(2), recipe_id));
    assert!(system_chats(&given)[0]
        .1
        .starts_with("commands.recipe.give.success.single"));

    let duplicate = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("recipe give bob {recipe_id}"),
    );
    assert!(!duplicate.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::RecipeBookAdd(_),
            ..
        }
    )));
    assert_eq!(system_chats(&duplicate)[0].1, "commands.recipe.give.failed");

    let taken = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("recipe take bob {recipe_id}"),
    );
    let removed = taken.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::RecipeBookRemove(remove),
        } => Some(remove),
        _ => None,
    });
    assert_eq!(
        removed.map(|packet| packet.recipe_ids.as_slice()),
        Some(&[startup.recipe_displays.displays_for_recipe(recipe_id)[0].id][..])
    );
    assert!(!world.player_knows_recipe_for_test(ConnectionId(2), recipe_id));
    assert!(system_chats(&taken)[0]
        .1
        .starts_with("commands.recipe.take.success.single"));
}

/// `CommandSourceStack.sendSuccess` sends only when `source.acceptsSuccess()`
/// (`CommandSourceStack.java:464-466`), which for a player is
/// `GameRules.SEND_COMMAND_FEEDBACK` (`ServerPlayer.java:341-343`).
/// `sendFailure` (`:495-499`) carries no such gate, because
/// `ServerPlayer.acceptsFailure` is unconditionally true (`:345-347`).
///
/// The rule is read where vanilla reads it -- after the command's own change
/// landed. `GameRuleCommand.setRule` writes the rule and only then reports
/// (`GameRuleCommand.java:38-39`), so `/gamerule send_command_feedback false`
/// swallows its own confirmation and the `true` that undoes it announces
/// itself.
#[test]
fn send_command_feedback_gates_success_and_the_rule_write_lands_before_the_report() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let queried = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule send_command_feedback",
    );
    assert_eq!(
        system_chats(&queried)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.gamerule.query send_command_feedbacktrue"]
    );

    let disabled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule send_command_feedback false",
    );
    assert_eq!(
        system_chats(&disabled),
        Vec::new(),
        "the write lands first, so the command that turns the rule off is the \
         first one it silences"
    );

    let listed = chat_command(&sender, &receiver, &mut world, 1, "list");
    assert_eq!(system_chats(&listed), Vec::new());

    // A refusal still arrives: only the success channel is gated.
    let refused = chat_command(&sender, &receiver, &mut world, 1, "list nonsense");
    assert!(
        !system_chats(&refused).is_empty(),
        "sendFailure has no rule in front of it"
    );

    let enabled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule send_command_feedback true",
    );
    assert_eq!(
        system_chats(&enabled)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.gamerule.set send_command_feedbacktrue"]
    );
}

/// `MsgCommand.sendMessage` reports the outgoing copy with
/// `source.sendChatMessage` (`MsgCommand.java:40`), and
/// `CommandSourceStack.sendChatMessage` (`CommandSourceStack.java:442-451`)
/// reads `this.silent` alone -- `acceptsSuccess` is the gate on `sendSuccess`
/// (`:465`) and on nothing else. So `send_command_feedback false` silences
/// `/seed`, which reports through `sendSuccess` (`SeedCommand.java:15`), and
/// leaves the whisper the sender sees alone.
#[test]
fn send_command_feedback_silences_a_success_line_but_not_the_msg_the_sender_sees() {
    let (sender, receiver) = mailbox();
    // `/seed` and `/gamerule` are both `LEVEL_GAMEMASTERS`
    // (`SeedCommand.java:12`, `GameRuleCommand.java:16`), so the sender has to
    // be an operator to reach either node at all.
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let before = chat_command(&sender, &receiver, &mut world, 1, "seed");
    assert_eq!(system_chats(&before).len(), 1, "{before:?}");

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule send_command_feedback false",
    );

    let silenced = chat_command(&sender, &receiver, &mut world, 1, "seed");
    assert_eq!(
        system_chats(&silenced),
        Vec::new(),
        "`/seed` goes through sendSuccess: {silenced:?}"
    );

    let whispered = chat_command(&sender, &receiver, &mut world, 1, "msg bob hello");
    let chats = chat_lines(&world, &whispered);
    assert_eq!(chats.len(), 2, "{chats:?}");
    assert_eq!(chats[0].0, ConnectionId(1));
    assert!(
        chats[0].1.starts_with("commands.message.display.outgoing"),
        "the sender's own copy is a chat message, not a success line: {chats:?}"
    );
    assert_eq!(chats[1].0, ConnectionId(2));
    assert!(
        chats[1].1.starts_with("commands.message.display.incoming"),
        "{chats:?}"
    );
}

/// Asks one session to log in and returns the refusal, if the world produced
/// one.
fn refused_join(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
    name: &str,
    uuid: u128,
    address: Option<&str>,
) -> Option<WorldCloseReason> {
    enqueue(
        sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext {
                remote_address: address.map(str::to_string),
                now: 1_755_000_000_000,
                ..Default::default()
            },
            connection_id: ConnectionId(connection_id),
            profile: profile(name, uuid),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    closed_reasons(&tick)
        .into_iter()
        .next()
        .map(|(_, reason)| reason)
}

#[test]
fn stop_from_chat_halts_the_world_rather_than_only_saying_so() {
    // `StopCommand` (StopCommand.java:10-14) answers and then calls
    // `halt(false)`. The answer alone is what this server used to do, so the
    // test names both halves.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let tick = chat_command(&sender, &receiver, &mut world, 1, "stop");
    assert_eq!(
        system_chats(&tick),
        vec![(ConnectionId(1), "commands.stop.stopping".to_string())]
    );
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::StopRequested)));

    // A stopping world takes no new session, which is the state the halt left
    // behind rather than anything the runtime did with the event.
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 2, "bob", 2, None),
        Some(WorldCloseReason::ServerShutdown)
    );
}

#[test]
fn save_off_holds_the_unattended_save_back_and_save_on_lets_it_through() {
    // `MinecraftServer.setAutoSave` (MinecraftServer.java:2069-2079) clears
    // `ServerLevel.noSave`, which `saveAllChunks` hands on to every unforced
    // save (:595). `/save-all` forces its own (SaveAllCommand.java:26), so it
    // is unaffected.
    //
    // The commands run through `command.execute` rather than through a chat
    // session, because reaching the next unattended save takes longer than the
    // keep-alive window a session survives.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();

    let saved = management_command(&sender, &receiver, &mut world, 1, "save-all");
    assert!(saved.events.iter().any(|event| matches!(
        event,
        WorldEvent::SaveRequested {
            flush: false,
            trigger: SaveTrigger::Management,
        }
    )));

    management_command(&sender, &receiver, &mut world, 2, "save-off");
    assert!(
        !autosave_fires_on_the_next_interval(&mut world),
        "save-off must hold the unattended save back"
    );

    // The forced save still goes through while saving is off.
    let forced = management_command(&sender, &receiver, &mut world, 3, "save-all flush");
    assert!(forced.events.iter().any(|event| matches!(
        event,
        WorldEvent::SaveRequested {
            flush: true,
            trigger: SaveTrigger::Management,
        }
    )));

    management_command(&sender, &receiver, &mut world, 4, "save-on");
    assert!(
        autosave_fires_on_the_next_interval(&mut world),
        "save-on must let the unattended save through again"
    );
}

/// Runs one command through the management surface, which needs no session.
fn management_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Admin {
            envelope: admin_envelope(operation_id, command, AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// Ticks the world to the next unattended-save boundary and answers whether it
/// asked for one.
fn autosave_fires_on_the_next_interval(world: &mut ServerWorld) -> bool {
    loop {
        let last = world.ticks_until_autosave() <= 1;
        let tick = world.tick(Vec::new());
        if last {
            return tick.events.iter().any(|event| {
                matches!(
                    event,
                    WorldEvent::SaveRequested {
                        trigger: SaveTrigger::Autosave,
                        ..
                    }
                )
            });
        }
    }
}

#[test]
fn difficulty_from_chat_writes_the_setting_and_tells_every_client() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    assert_eq!(world.admin_settings().difficulty, Difficulty::Easy);

    let tick = chat_command(&sender, &receiver, &mut world, 1, "difficulty peaceful");
    assert_eq!(world.admin_settings().difficulty, Difficulty::Peaceful);
    // `MinecraftServer.sendDifficultyUpdate` (MinecraftServer.java:1311-1314)
    // reaches every session, not only the one that asked.
    let told = tick
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::ChangeDifficulty(update) => {
                Some((connection_id, update.difficulty))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        told,
        vec![
            (ConnectionId(1), Difficulty::Peaceful),
            (ConnectionId(2), Difficulty::Peaceful),
        ]
    );

    // A difficulty already in force changes nothing and reports the refusal.
    let again = chat_command(&sender, &receiver, &mut world, 1, "difficulty peaceful");
    assert_eq!(
        system_chats(&again),
        vec![(
            ConnectionId(1),
            "commands.difficulty.failure peaceful".to_string()
        )]
    );
}

#[test]
fn gamerule_from_chat_changes_what_the_next_query_reports() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let queried = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed",
    );
    assert_eq!(
        system_chats(&queried),
        vec![(
            ConnectionId(1),
            "commands.gamerule.query random_tick_speed3".to_string()
        )]
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed 7",
    );
    let after = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed",
    );
    assert_eq!(
        system_chats(&after),
        vec![(
            ConnectionId(1),
            "commands.gamerule.query random_tick_speed7".to_string()
        )]
    );
}

#[test]
fn op_from_chat_grants_the_level_the_next_command_is_gated_on() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    // Before the grant, `/stop` is not even a command bob's parse can see.
    let unpermitted = chat_command(&sender, &receiver, &mut world, 2, "stop");
    assert_eq!(
        system_chats(&unpermitted)[0].1,
        "command.unknown.command".to_string()
    );

    let granted = chat_command(&sender, &receiver, &mut world, 1, "op bob");
    // `PlayerList.op` (PlayerList.java:537-548) ends in
    // `sendPlayerPermissionLevel`, which announces the level and republishes
    // the tree to that session alone.
    assert!(granted.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityEvent(update),
        } if update.event_id == CommandPermission::Owner.entity_event_id()
    )));
    assert!(granted.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::Commands(_),
        }
    )));
    // The list the runtime writes carries the entry, which is the state that
    // survives a restart.
    let document = String::from_utf8(world.admin_documents().operators).unwrap();
    assert!(document.contains("bob"), "{document}");

    // The grant is what the next parse reads, so `/stop` now reaches its
    // executor.
    let permitted = chat_command(&sender, &receiver, &mut world, 2, "stop");
    assert!(permitted
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::StopRequested)));
}

#[test]
fn deop_from_chat_takes_the_level_away_again() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1), ("bob", 2)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let removed = chat_command(&sender, &receiver, &mut world, 1, "deop bob");
    assert!(removed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityEvent(update),
        } if update.event_id == CommandPermission::All.entity_event_id()
    )));
    let document = String::from_utf8(world.admin_documents().operators).unwrap();
    assert!(!document.contains("bob"), "{document}");

    let unpermitted = chat_command(&sender, &receiver, &mut world, 2, "stop");
    assert_eq!(
        system_chats(&unpermitted)[0].1,
        "command.unknown.command".to_string()
    );
}

#[test]
fn ban_from_chat_drops_the_session_and_refuses_the_next_login() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let name = format!("Stand {}", "x".repeat(130));
    for x in [0.5, 1.5] {
        let stand = world
            .spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                Vec3d { x, y: 64.0, z: 0.5 },
                crate::entity::EntityData::ArmorStand(
                    crate::entity::ArmorStandData::new().unwrap(),
                ),
            )
            .1;
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!(r#"data merge entity {stand} {{CustomName:{{text:"{name}\ud800"}}}}"#),
        );
    }
    let banned = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "ban bob griefing @e[type=armor_stand]",
    );
    // `BanPlayerCommands.banPlayers` (BanPlayerCommands.java:46-49)
    // disconnects the session with the bare key.
    let closed = closed_reasons(&banned);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].0, ConnectionId(2));
    let WorldCloseReason::Administrative(reason) = &closed[0].1 else {
        panic!("a ban carries its own refusal component, got {closed:?}");
    };
    assert_eq!(reason.text(), "multiplayer.disconnect.banned");
    assert_eq!(banned.snapshot.players.len(), 1);

    // `UserBanListEntry` records the reason and `source.getTextName()`
    // (BanPlayerCommands.java:42), which is the session that ran the command.
    let document = String::from_utf8(world.admin_documents().player_bans).unwrap();
    assert!(document.contains("alice"), "{document}");
    let lists = StoredUserLists::parse(&world.admin_documents()).unwrap();
    let expected = bbb_protocol::JavaString::from(
        "griefing "
            .encode_utf16()
            .chain(name.encode_utf16())
            .chain([0xd800])
            .chain(", ".encode_utf16())
            .chain(name.encode_utf16())
            .chain([0xd800])
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        lists.player_bans[&Uuid::from_u128(2)].ban.reason.as_ref(),
        Some(&expected)
    );
    let mut restored = ServerWorld::new(ServerWorldConfig {
        admin_lists: lists,
        ..creative_fixture_config()
    })
    .unwrap();
    let (restored_sender, restored_receiver) = mailbox();
    join_loaded(
        &restored_sender,
        &restored_receiver,
        &mut restored,
        1,
        "alice",
        1,
    );
    let listing = chat_command(
        &restored_sender,
        &restored_receiver,
        &mut restored,
        1,
        "banlist players",
    );
    assert!(listing.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SystemChat(chat), .. }
        if bbb_server_datapack::language::component_plain_text(chat.content.component())
            .windows(expected.units().len()).any(|units| units == expected.units()))));
    let cold_refusal = refused_join(
        &restored_sender,
        &restored_receiver,
        &mut restored,
        2,
        "bob",
        2,
        None,
    )
    .unwrap();
    let WorldCloseReason::Administrative(cold_refusal) = cold_refusal else {
        panic!("restored ban refusal");
    };
    assert!(
        bbb_server_datapack::language::component_plain_text(cold_refusal.component())
            .windows(expected.units().len())
            .any(|units| units == expected.units())
    );

    // `PlayerList.canPlayerLogin` (PlayerList.java:336-342) refuses the next
    // login from the list the command wrote, wrapping the stored reason.
    let refusal = refused_join(&sender, &receiver, &mut world, 3, "bob", 2, None)
        .expect("a banned login is refused");
    let WorldCloseReason::Administrative(refusal) = refusal else {
        panic!("a banned login carries its own refusal component");
    };
    assert!(
        refusal
            .text()
            .contains("multiplayer.disconnect.banned.reason"),
        "{}",
        refusal.text()
    );
    assert!(refusal.text().contains("griefing"), "{}", refusal.text());

    // `/pardon` takes the entry back out, and the same login is accepted.
    chat_command(&sender, &receiver, &mut world, 1, "pardon bob");
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 4, "bob", 2, None),
        None
    );
}

#[test]
fn ban_ip_from_chat_drops_every_session_on_that_address() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext {
                remote_address: Some("203.0.113.9".to_string()),
                now: 1_755_000_000_000,
                ..Default::default()
            },
            connection_id: ConnectionId(2),
            profile: profile("bob", 2),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    // `BanIpCommands.banIpOrName` (BanIpCommands.java:42-44) resolves a name to
    // the address that player is connected from.
    let banned = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "ban-ip bob blocked @a[name=alice]",
    );
    let closed = closed_reasons(&banned);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].0, ConnectionId(2));
    let document = String::from_utf8(world.admin_documents().ip_bans).unwrap();
    assert!(document.contains("203.0.113.9"), "{document}");
    let lists = StoredUserLists::parse(&world.admin_documents()).unwrap();
    assert_eq!(
        lists.ip_bans["203.0.113.9"].reason,
        Some("blocked alice".into())
    );
    let mut world = ServerWorld::new(ServerWorldConfig {
        admin_lists: lists,
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let refusal = refused_join(
        &sender,
        &receiver,
        &mut world,
        3,
        "carol",
        3,
        Some("203.0.113.9"),
    )
    .expect("a banned address is refused whoever is behind it");
    let WorldCloseReason::Administrative(refusal) = refusal else {
        panic!("a banned address carries its own refusal component");
    };
    assert!(
        refusal.text().contains("multiplayer.disconnect.banned_ip"),
        "{}",
        refusal.text()
    );

    chat_command(&sender, &receiver, &mut world, 1, "pardon-ip 203.0.113.9");
    assert_eq!(
        refused_join(
            &sender,
            &receiver,
            &mut world,
            4,
            "carol",
            3,
            Some("203.0.113.9")
        ),
        None
    );
}

#[test]
fn kick_from_chat_closes_the_named_session() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let kicked = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "kick bob rude @a[name=alice] @z",
    );
    let closed = closed_reasons(&kicked);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].0, ConnectionId(2));
    let WorldCloseReason::Administrative(reason) = &closed[0].1 else {
        panic!("a kick carries the reason it was given, got {closed:?}");
    };
    assert_eq!(reason.text(), "rude alice @z");
    assert!(reason.styled().iter().any(|run| run.text == "alice"
        && run.style.click_event.is_some()
        && run.style.hover_event.is_some()));
    assert_eq!(kicked.snapshot.players.len(), 1);
    // A kick is not a ban: the same login comes straight back.
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 3, "bob", 2, None),
        None
    );
}

/// With `enforce-whitelist` set at startup, `/whitelist on` kicks the online
/// players the list does not name (`MinecraftServer.java:1656-1667`), and the
/// change is what the runtime saves for the next start.
#[test]
fn an_enforced_whitelist_kicks_unlisted_players_and_is_saved() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        enforce_allowlist: true,
        player_idle_timeout: 3,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(&sender, &receiver, &mut world, 1, "whitelist add alice");

    let enabled = chat_command(&sender, &receiver, &mut world, 1, "whitelist on");

    let closed = closed_reasons(&enabled);
    assert_eq!(closed.len(), 1, "{closed:?}");
    assert_eq!(closed[0].0, ConnectionId(2));
    assert!(
        format!("{:?}", closed[0].1).contains("multiplayer.disconnect.not_whitelisted"),
        "{closed:?}"
    );
    chat_command(&sender, &receiver, &mut world, 1, "setidletimeout 9");
    let saved = world.persisted_settings();
    assert_eq!(
        (
            saved.white_list,
            saved.enforce_whitelist,
            saved.player_idle_timeout
        ),
        (Some(true), Some(true), Some(9))
    );
}

#[test]
fn whitelist_from_chat_gates_the_next_login() {
    // The names here are the identities a real login carries, because
    // `/whitelist add <name>` resolves a name the server has never seen to its
    // offline derivation and the allowlist is keyed by id.
    let (sender, receiver) = mailbox();
    let bob = offline_uuid("bob");
    let carol = offline_uuid("carol");
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(!world.admin_settings().use_allowlist);

    chat_command(&sender, &receiver, &mut world, 1, "whitelist on");
    assert!(world.admin_settings().use_allowlist);
    // `MinecraftServer.kickUnlistedPlayers` (MinecraftServer.java:1657) needs
    // `enforce-whitelist` as well, which is off by default, so the session that
    // asked stays on.
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 2, "bob", bob, None)
            .map(|reason| matches!(reason, WorldCloseReason::Administrative(_))),
        Some(true)
    );

    chat_command(&sender, &receiver, &mut world, 1, "whitelist add bob");
    let document = String::from_utf8(world.admin_documents().allowlist).unwrap();
    assert!(document.contains("bob"), "{document}");
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 3, "bob", bob, None),
        None
    );

    chat_command(&sender, &receiver, &mut world, 1, "whitelist remove bob");
    let document = String::from_utf8(world.admin_documents().allowlist).unwrap();
    assert!(!document.contains("bob"), "{document}");
    // The gate is still on for anyone the list never named.
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 4, "carol", carol, None)
            .map(|reason| matches!(reason, WorldCloseReason::Administrative(_))),
        Some(true)
    );

    chat_command(&sender, &receiver, &mut world, 1, "whitelist off");
    assert!(!world.admin_settings().use_allowlist);
    assert_eq!(
        refused_join(&sender, &receiver, &mut world, 5, "carol", carol, None),
        None
    );
}

/// The id a login carries in offline mode
/// (`bbb_protocol::codec::offline_player_uuid`), which is also what a command
/// resolves a bare name to when the server has never seen it.
fn offline_uuid(name: &str) -> u128 {
    bbb_protocol::codec::offline_player_uuid(name).as_u128()
}

#[test]
fn reload_from_chat_asks_the_runtime_to_rebuild_the_datapacks() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let tick = chat_command(&sender, &receiver, &mut world, 1, "reload");
    assert_eq!(
        system_chats(&tick),
        vec![(ConnectionId(1), "commands.reload.success".to_string())]
    );
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::ReloadRequested { selected: None })));
}

#[test]
fn command_completion_answers_with_live_names_and_component_ranges() {
    // `handleCustomCommandSuggestions` asks the dispatcher over the live world,
    // which is the only way a player-name argument can answer at all.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::CommandSuggestion(CommandSuggestionRequest {
            id: 11,
            command: "msg b".into(),
        }),
    );
    let answer = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::CommandSuggestions(update),
                ..
            } => Some(update.clone()),
            _ => None,
        })
        .expect("the handler always answers");
    assert_eq!(answer.id, 11);
    assert_eq!(
        answer
            .suggestions
            .into_iter()
            .map(|suggestion| suggestion.text)
            .collect::<Vec<_>>(),
        vec!["bob".to_string()]
    );

    for (id, command, expected) in [
        (
            12,
            "/give alice diamond[custom_data={text:'😀'},max_d",
            "minecraft:max_damage=",
        ),
        (
            13,
            "/execute if items entity @s armor.* *[custom_data={text:'😀'},max_d",
            "minecraft:max_damage",
        ),
    ] {
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::CommandSuggestion(CommandSuggestionRequest {
                id,
                command: command.to_string(),
            }),
        );
        let answer = tick
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::CommandSuggestions(update),
                } => Some(update),
                _ => None,
            })
            .expect("the handler answers the component request");
        assert_eq!(answer.id, id);
        let start = command.rfind("max_d").unwrap();
        assert_eq!(
            answer.start as usize,
            command[..start].encode_utf16().count()
        );
        assert_eq!(answer.length, 5);
        assert!(answer
            .suggestions
            .iter()
            .any(|suggestion| suggestion.text == expected));
    }
}

#[test]
fn seed_reports_the_seed_the_restored_save_declares() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.world_gen_settings.seed = 6_478_291;
    let mut world = ServerWorld::restore(operator_config(&[("alice", 1)]), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let tick = chat_command(&sender, &receiver, &mut world, 1, "seed");
    let chats = system_chats(&tick);
    assert_eq!(chats.len(), 1, "{chats:?}");
    assert!(chats[0].1.contains("6478291"), "{chats:?}");
}

/// `/defaultgamemode` reports the count `enforceGameTypeForPlayers` produced
/// (`DefaultGameModeCommands.java:23-25`), and the management surface is the
/// only place a `CommandRun`'s numeric result is answered with.
#[test]
fn the_forced_game_mode_command_reports_the_sessions_it_moved() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        force_game_mode: true,
        ..creative_fixture_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    let run = |world: &mut ServerWorld, request_id: &str, command: &str| {
        enqueue(
            &sender,
            WorldCommand::Admin {
                // The request id is the management surface's idempotency key, so
                // a repeat of the same command needs its own.
                envelope: admin_envelope(3, request_id, AdminScope::Command),
                request: AdminRequest::ExecuteCommand {
                    command: command.to_string(),
                },
            },
        )
        .unwrap();
        let tick = world.tick(receiver.drain());
        let outcome = admin_outcome(&tick);
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            panic!("expected a command answer, got {:?}", outcome.result);
        };
        report.result
    };

    // Both sessions joined on the forced default, which the fixture makes
    // creative, so moving the default to survival moves both.
    assert_eq!(
        run(&mut world, "first", "defaultgamemode survival"),
        Some(2)
    );
    // And the same command again moves nobody: `changeGameModeForPlayer`
    // returns false for a session already in the mode
    // (`ServerPlayerGameMode.java:56-58`).
    assert_eq!(
        run(&mut world, "second", "defaultgamemode survival"),
        Some(0)
    );
}

#[test]
fn the_management_command_surface_applies_the_same_effects_as_chat() {
    // `command.execute` runs the one dispatcher at owner level and hands its
    // effects to the same applier, so a command that changes the world changes
    // it whichever surface asked.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(3, "difficulty", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "difficulty hard".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("expected a command answer, got {:?}", outcome.result);
    };
    assert_eq!(report.result, Some(0));
    assert_eq!(world.admin_settings().difficulty, Difficulty::Hard);
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
        } if update.difficulty == Difficulty::Hard
    )));

    // A ban asked for through the same surface reaches the same list.
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(4, "ban", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "ban alice".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert_eq!(closed_reasons(&tick).len(), 1);
    let document = String::from_utf8(world.admin_documents().player_bans).unwrap();
    assert!(document.contains("alice"), "{document}");
}

/// `/defaultgamemode` moves the level's own game type, which is what the next
/// join reads and what the next `level.dat` write records -- and what the next
/// startup writes over.
///
/// `DefaultGameModeCommands.setMode` (DefaultGameModeCommands.java:20-26) is
/// `worldData.setGameType` plus `enforceGameTypeForPlayers(getForcedGameType())`,
/// and this world is built without `force-gamemode`, so the second call takes
/// the null branch (DedicatedServer.java:792-794) and returns zero before it
/// reaches a player (MinecraftServer.java:1389-1392). A session already placed
/// therefore keeps the mode `ServerPlayer.calculateGameModeForNewPlayer`
/// (`ServerPlayer.java:2029-2036`) gave it, and only the next join sees the new
/// default. `force_game_mode_outranks_saved_modes_and_moves_the_sessions_already_placed`
/// is the other half, where the same command moves everybody.
///
/// The restart is the other half and it goes the other way: a dedicated server
/// runs `this.worldData.setGameType(properties.gameMode.get())`
/// (DedicatedServer.java:237) after the level data is loaded, so the saved
/// `GameType` never reaches a joining session. What does reach one is the
/// player's own saved `playerGameType`, which `calculateGameModeForNewPlayer`
/// prefers over the level default (`ServerPlayer.java:2034`).
#[test]
fn defaultgamemode_moves_the_next_joins_mode_and_the_level_it_is_saved_in() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-default-game-mode-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-default-game-mode");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(candidate.level.game_type(), GameType::Creative.id());

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    assert_eq!(
        login_game_type(&sender, &receiver, &mut world, 1, "op", 1),
        GameType::Creative.id() as i8
    );

    let ran = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode adventure",
    );
    assert_eq!(
        system_chats(&ran)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.defaultgamemode.success gameMode.adventure"]
    );

    // The next join takes the new default; the one already placed does not move.
    assert_eq!(
        login_game_type(&sender, &receiver, &mut world, 2, "newcomer", 2),
        GameType::Adventure.id() as i8
    );

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.level_state.game_type, GameType::Adventure.id());
    let mut saved = request
        .player_patches
        .iter()
        .map(|patch| patch.created.game_mode.id())
        .collect::<Vec<_>>();
    saved.sort_unstable();
    assert_eq!(
        saved,
        vec![GameType::Creative.id(), GameType::Adventure.id()],
        "the session placed before the command keeps creative, the one after it \
         is written as adventure"
    );
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    // A restart reads the level back and then writes the `gamemode` property
    // over it, so what the file records and what the next session runs at are
    // two different answers.
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(reloaded.level.game_type(), GameType::Adventure.id());
    let mut restarted = ServerWorld::restore(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..creative_fixture_config()
        },
        &reloaded,
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    // A profile the save holds no document for falls through to the level
    // default, which the property just replaced.
    assert_eq!(
        login_game_type(&sender, &receiver, &mut restarted, 3, "stranger", 3),
        GameType::Survival.id() as i8,
        "the gamemode property outranks the saved GameType"
    );
    // The two saved sessions each get their own `playerGameType` back, not the
    // level default and not each other's.
    assert_eq!(
        login_game_type(&sender, &receiver, &mut restarted, 1, "op", 1),
        GameType::Creative.id() as i8,
        "op was saved as creative"
    );
    assert_eq!(
        login_game_type(&sender, &receiver, &mut restarted, 2, "newcomer", 2),
        GameType::Adventure.id() as i8,
        "newcomer was saved as adventure"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `force-gamemode` is the branch `ServerPlayer.calculateGameModeForNewPlayer`
/// (`ServerPlayer.java:2029-2036`) takes before it ever looks at a saved
/// `playerGameType`, and it is also what turns `/defaultgamemode` into a
/// command that moves the sessions already placed.
///
/// Both halves hang off one answer: `getForcedGameType`
/// (`DedicatedServer.java:792-794`) reports the level's own game type instead
/// of null, so `calculateGameModeForNewPlayer` returns at `:2032` and
/// `enforceGameTypeForPlayers` walks the player list instead of returning zero
/// at `MinecraftServer.java:1390-1391`. The official server this repository
/// replays against declares the property
/// (`scripts/data/official-replay-server.properties`), which is why the
/// unforced shape alone is not enough.
#[test]
fn force_game_mode_outranks_saved_modes_and_moves_the_sessions_already_placed() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-force-game-mode-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-force-game-mode");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();

    // A save recording two different `playerGameType`s, so neither of them can
    // be confused with the forced answer below.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            game_type: GameType::Adventure,
            ..operator_config(&[("op", 1)])
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(
        login_game_type(&sender, &receiver, &mut world, 1, "op", 1),
        GameType::Adventure.id() as i8
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode creative",
    );
    assert_eq!(
        login_game_type(&sender, &receiver, &mut world, 2, "builder", 2),
        GameType::Creative.id() as i8
    );
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();

    // Without the property the saved mode wins for the two profiles the save
    // has a document for, and the level default only answers for the third --
    // the `loadedGameType != null ? loadedGameType : getDefaultGameType()` arm
    // (`ServerPlayer.java:2034`).
    let (sender, receiver) = mailbox();
    let mut unforced = ServerWorld::restore(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..operator_config(&[("op", 1)])
        },
        &candidate,
    )
    .unwrap();
    for (connection_id, name, uuid, expected) in [
        (1u64, "op", 1u128, GameType::Adventure),
        (2, "builder", 2, GameType::Creative),
        (3, "stranger", 3, GameType::Survival),
    ] {
        assert_eq!(
            login_game_type(&sender, &receiver, &mut unforced, connection_id, name, uuid),
            expected.id() as i8,
            "{name} took the unforced branch"
        );
    }

    // With it, the level default is the only answer any of the three gets.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            game_type: GameType::Survival,
            force_game_mode: true,
            ..operator_config(&[("op", 1)])
        },
        &candidate,
    )
    .unwrap();
    for (connection_id, name, uuid) in [(1u64, "op", 1u128), (2, "builder", 2), (3, "stranger", 3)]
    {
        assert_eq!(
            login_game_type(&sender, &receiver, &mut world, connection_id, name, uuid),
            GameType::Survival.id() as i8,
            "{name} joined at the forced type, not at what the save held for it"
        );
    }

    // `/defaultgamemode` writes the value the next line reads back as the
    // forced type, so every session online is moved onto it.
    let moved = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode spectator",
    );
    let announced = moved
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::PlayerInfoUpdate(update)
                if update.actions == vec![PlayerInfoAction::UpdateGameMode] =>
            {
                Some((connection_id, update.entries[0].game_mode))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        announced.len(),
        9,
        "each of the three moves is broadcast to all three sessions \
         (ServerPlayerGameMode.java:67-70)"
    );
    assert!(announced
        .iter()
        .all(|(_, game_mode)| *game_mode == GameType::Spectator));

    // The same command again moves nobody: `changeGameModeForPlayer` returns
    // false for a session already in the mode (ServerPlayerGameMode.java:56-58),
    // which is what makes the vanilla count a count of moves.
    let repeated = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode spectator",
    );
    assert!(
        !repeated.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(
                ClientboundPlayPacket::PlayerAbilities(_)
                    | ClientboundPlayPacket::PlayerInfoUpdate(_)
            )
        )),
        "a repeat of the same forced type is not a move"
    );

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let saved = request
        .player_patches
        .iter()
        .map(|patch| patch.created.game_mode.id())
        .collect::<Vec<_>>();
    assert_eq!(
        saved,
        vec![GameType::Spectator.id(); 3],
        "`storeGameTypes` (ServerPlayer.java:2038-2042) writes the mode the \
         session was moved to"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

type BoundChatEntry = (u64, Option<u128>, i32, i32, Option<String>);

/// What one tick's chat-type-bound packets said: recipient, sender (`None` for
/// disguised chat), global index, chat type id and target name.
fn bound_chats(tick: &TickResult) -> Vec<BoundChatEntry> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerChat(chat),
            } => Some((
                connection_id.0,
                Some(chat.sender.as_u128()),
                chat.global_index,
                chat.chat_type.chat_type_id,
                chat.chat_type
                    .target_name
                    .as_ref()
                    .map(|name| name.text().to_owned()),
            )),
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DisguisedChat(chat),
            } => Some((
                connection_id.0,
                None,
                -1,
                chat.chat_type.chat_type_id,
                chat.chat_type
                    .target_name
                    .as_ref()
                    .map(|name| name.text().to_owned()),
            )),
            _ => None,
        })
        .collect()
}

fn chat_type_id(world: &ServerWorld, id: &str) -> i32 {
    world
        .synchronized_registries()
        .registry_element_index("minecraft:chat_type", id)
        .unwrap()
}

/// ServerGamePacketListenerImpl.java:1735-1761: each recipient numbers the
/// player chat it receives itself, and a player-sent message names its sender.
#[test]
fn player_chat_is_unsigned_and_numbered_per_recipient() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let chat = chat_type_id(&world, "minecraft:chat");
    let message = |text: &str| PlayServerbound::Chat(ChatMessage::unsigned(text, 0, 0));
    let first = play(&sender, &receiver, &mut world, 1, message("one"));
    assert_eq!(
        bound_chats(&first),
        [(1, Some(1), 0, chat, None), (2, Some(1), 0, chat, None)]
    );
    let second = play(&sender, &receiver, &mut world, 2, message("two"));
    assert_eq!(
        bound_chats(&second),
        [(1, Some(2), 1, chat, None), (2, Some(2), 1, chat, None)]
    );
}

/// MsgCommand.java:33-49 and MessageArgument.java:42-52: a typed (signed)
/// `/msg` is player chat bound to the incoming type for the target and the
/// outgoing type, with the target's name, for the sender; the same command
/// without a signing context, like the console's `/say`, is disguised.
#[test]
fn msg_binds_the_direct_message_chat_types_and_say_from_the_console_is_disguised() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let incoming = chat_type_id(&world, "minecraft:msg_command_incoming");
    let outgoing = chat_type_id(&world, "minecraft:msg_command_outgoing");
    let say = chat_type_id(&world, "minecraft:say_command");
    let signed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatCommandSigned(ChatCommandSigned::unsigned_arguments(
            "msg bob hello",
            0,
            0,
        )),
    );
    assert_eq!(
        bound_chats(&signed),
        [
            (1, Some(1), 0, outgoing, Some("bob".to_owned())),
            (2, Some(1), 0, incoming, None),
        ]
    );
    let unsigned = chat_command(&sender, &receiver, &mut world, 1, "msg bob hello");
    assert_eq!(
        bound_chats(&unsigned),
        [
            (1, None, -1, outgoing, Some("bob".to_owned())),
            (2, None, -1, incoming, None),
        ]
    );
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(9, "say", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "say from the console".into(),
            },
        },
    )
    .unwrap();
    let console = world.tick(receiver.drain());
    assert_eq!(
        bound_chats(&console),
        [(1, None, -1, say, None), (2, None, -1, say, None)]
    );
}

/// The item list outgrows the 1000 suggestions the handler sends
/// (`ServerGamePacketListenerImpl.java:598`).
#[test]
fn command_completion_sends_at_most_a_thousand_suggestions() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::CommandSuggestion(CommandSuggestionRequest {
            id: 1,
            command: "give alice ".into(),
        }),
    );
    let count = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::CommandSuggestions(update),
                ..
            } => Some(update.suggestions.len()),
            _ => None,
        })
        .expect("the handler always answers");
    assert_eq!(count, 1000);
}
