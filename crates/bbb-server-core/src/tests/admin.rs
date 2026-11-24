//! The administration envelope: ban, allowlist, op, execute, save, stop and
//! datapack reload, each applied on a tick and answered by operation id.

use super::*;

#[test]
fn an_admin_command_applies_on_the_tick_and_answers_by_operation_id() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let sequence = enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(7, "diagnostics", AdminScope::Read),
            request: AdminRequest::Diagnostics,
        },
    )
    .unwrap();
    assert!(sequence > 0);
    let commands = receiver.drain();

    let tick = world.tick(commands);
    let outcome = admin_outcome(&tick);
    assert_eq!(outcome.operation_id, 7);
    assert_eq!(outcome.action, "server.diagnostics");
    assert_eq!(outcome.tick, tick.tick);
    let Ok(AdminResult::Diagnostics(report)) = &outcome.result else {
        panic!("expected diagnostics");
    };
    assert_eq!(report.tick, tick.tick);
    assert_eq!(report.online_players, 0);
    // Nobody has joined and nothing has touched a block, so the generator has
    // not been asked for a single column yet.
    assert_eq!(report.loaded_chunks, 0);
}

#[test]
fn banning_a_player_disconnects_the_session_and_refuses_the_next_join() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext {
                remote_address: Some("203.0.113.7".to_string()),
                now: 1_755_000_000_000,
                ..Default::default()
            },
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "ban", AdminScope::Manage),
            request: AdminRequest::BansAdd(vec![UserBanRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                reason: Some("griefing".to_string()),
                source: None,
                expires: None,
            }]),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let closed = closed_reasons(&tick);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].0, ConnectionId(1));
    let WorldCloseReason::Administrative(reason) = &closed[0].1 else {
        panic!("a ban carries its own refusal component");
    };
    // `BanlistService.add` (BanlistService.java:48) disconnects the session
    // with the bare key; only the *login* refusal carries the stored reason.
    let rendered = format!("{:?}", reason.component());
    assert!(
        rendered.contains("multiplayer.disconnect.banned"),
        "{rendered}"
    );
    assert!(!rendered.contains("griefing"), "{rendered}");
    // The player list is empty again, so a stale route cannot be reused.
    assert!(tick.snapshot.players.is_empty());

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext {
                remote_address: Some("203.0.113.7".to_string()),
                now: 1_755_000_000_000,
                ..Default::default()
            },
            connection_id: ConnectionId(2),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let closed = closed_reasons(&tick);
    assert_eq!(closed.len(), 1);
    let WorldCloseReason::Administrative(refusal) = &closed[0].1 else {
        panic!("a banned login carries its own refusal component");
    };
    // `PlayerList.canPlayerLogin` (PlayerList.java:336-342) wraps the stored
    // reason in the translatable the client renders.
    let rendered = format!("{:?}", refusal.component());
    assert!(
        rendered.contains("multiplayer.disconnect.banned.reason"),
        "{rendered}"
    );
    assert!(rendered.contains("griefing"), "{rendered}");
    assert!(tick.snapshot.players.is_empty());
}

#[test]
fn the_allowlist_gates_joins_only_while_it_is_in_use() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "use", AdminScope::Manage),
            request: AdminRequest::SetSetting(AdminSetting::UseAllowlist(true)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert_eq!(closed_reasons(&tick).len(), 1);

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(2, "allow", AdminScope::Manage),
            request: AdminRequest::AllowlistAdd(vec![PlayerRef {
                id: Some(Uuid::from_u128(1)),
                name: Some("alice".into()),
            }]),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(closed_reasons(&tick).is_empty());
    assert_eq!(tick.snapshot.players.len(), 1);
}

#[test]
fn opping_a_player_announces_the_level_and_opens_the_gated_commands() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    // `PlayerList.sendPlayerPermissionLevel` announces the baseline at join.
    assert!(joined.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(EntityEvent { event_id, .. }),
            ..
        } if *event_id == CommandPermission::All.entity_event_id()
    )));

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "op", AdminScope::Manage),
            request: AdminRequest::OperatorsAdd(vec![OperatorRequest {
                player: PlayerRef::by_id(Uuid::from_u128(1)),
                permission_level: Some(CommandPermission::GameMaster),
                bypasses_player_limit: None,
            }]),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(EntityEvent { event_id, .. }),
            ..
        } if *event_id == CommandPermission::GameMaster.entity_event_id()
    )));
    // `sendPlayerPermissionLevel` (PlayerList.java:559-572) republishes the
    // tree with the level, so the client's completions gain the newly usable
    // commands without reconnecting.
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Commands(tree),
            ..
        } if *tree == crate::commands::tree::usable_command_tree(CommandPermission::GameMaster)
            && tree.nodes.iter().any(|node| node.name.as_deref() == Some("say"))
    )));

    // `/say` is gamemaster-gated, so the new level is what makes it run.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::ChatCommand {
                command: "say hello".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let chats = chat_lines(&world, &tick);
    assert!(
        chats.iter().any(|(_, text)| text.contains("hello")),
        "{chats:?}"
    );
}

#[test]
fn a_management_command_execute_reaches_the_one_dispatcher_and_broadcasts() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(9, "say", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "say from the agent".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("expected a command answer");
    };
    assert_eq!(report.result, Some(1));
    // The console is no player, so the message is disguised.
    let broadcast = tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::DisguisedChat(chat),
            } if chat.message.text() == "from the agent"
        )
    });
    assert!(broadcast);
}

#[test]
fn save_and_stop_reach_the_runtime_as_lifecycle_events() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "save", AdminScope::Manage),
            request: AdminRequest::Save { flush: true },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::SaveRequested {
            flush: true,
            trigger: SaveTrigger::Management,
        }
    )));

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(2, "stop", AdminScope::Manage),
            request: AdminRequest::Stop,
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::StopRequested)));
    assert!(tick.snapshot.stopping);

    // A join that lands after the stop is refused rather than half-served.
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert_eq!(
        closed_reasons(&tick),
        vec![(ConnectionId(1), WorldCloseReason::ServerShutdown)]
    );
}

#[test]
fn the_admin_mailbox_partition_bounds_a_management_burst() {
    let limits = WorldMailboxLimits {
        admin: crate::mailbox::MailboxClassLimits {
            entries: 2,
            bytes: 64 * 1024,
        },
        ..WorldMailboxLimits::default()
    };
    let (sender, receiver) = world_command_mailbox(limits).unwrap();
    for index in 0..2 {
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(index, "burst", AdminScope::Read),
                request: AdminRequest::Players,
            },
        )
        .unwrap();
    }
    assert!(matches!(
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(3, "burst", AdminScope::Read),
                request: AdminRequest::Players,
            },
        ),
        Err(WorldCommandSendError::Full(_))
    ));
    // A network command still fits: the two classes have independent quotas.
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    assert_eq!(receiver.drain().len(), 3);
}

#[test]
fn reload_asks_the_runtime_to_rebuild_and_the_answer_lands_at_a_tick_barrier() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert!(world.datapacks().is_none());

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "reload", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "reload".into(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    // The world owns no I/O, so all it can do is ask. The command itself
    // already answered, the way `ReloadCommand` answers before the reload runs.
    assert!(tick
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::ReloadRequested { selected: None })));
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("expected a command report, got {:?}", outcome.result);
    };
    assert_eq!(report.result, Some(0));

    // A rebuild that failed leaves the generation in place and reports the
    // vanilla failure key, because by now the command has long since answered.
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Failed("the pack directory is unreadable".to_string()),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(world.datapacks().is_none());
    let broadcast_keys = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(chat),
                ..
            } => Some(chat.content.component().clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    // Nobody is connected, so the failure reaches nobody; what matters is that
    // the generation survived a failed rebuild.
    assert!(broadcast_keys.is_empty());

    // A rebuild whose registry entries differ is refused: Vanilla reloads
    // tags/resources against its existing registry access and never moves the
    // synchronized holder catalog.
    let mut changed_catalog = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .expect("the built-in pack always loads");
    changed_catalog
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .swap(0, 1);
    let changed_catalog = Arc::new(changed_catalog);
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::clone(&changed_catalog)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert!(world.datapacks().is_none());
    assert_eq!(
        world
            .synchronized_registries()
            .dimension_type_index("minecraft:overworld"),
        Some(0)
    );

    // A generation with the same registry catalog but a new tag report is
    // installed atomically and announces Play UpdateTags at the barrier.
    let mut tags_only = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .expect("the built-in pack always loads");
    let damage_type_id = tags_only
        .registries
        .registry("minecraft:damage_type")
        .and_then(|registry| registry.elements.first())
        .map(|element| element.id.clone())
        .expect("the built-in damage type registry is non-empty");
    tags_only
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:damage_type")
        .expect("the built-in damage type tags are present")
        .tags
        .insert(
            "minecraft:bbb_reload_test".to_string(),
            vec![damage_type_id],
        );
    assert_eq!(
        crate::registry_sync::SynchronizedRegistries::load(Some(&tags_only))
            .unwrap()
            .published(),
        world.synchronized_registries().published()
    );
    enqueue(
        &sender,
        WorldCommand::Join {
            connection_id: ConnectionId(7),
            profile: profile("reload-player", 7),
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(tags_only)),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(world.datapacks().is_some());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::UpdateTags(update),
            ..
        } if update
            .registries
            .iter()
            .any(|registry| registry.registry == "minecraft:damage_type"
                && registry.tags.iter().any(|tag| tag.tag == "minecraft:bbb_reload_test"))
    )));
    let packet_order = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(7),
                packet: ClientboundPlayPacket::UpdateTags(_),
            } => Some("tags"),
            WorldEvent::Send {
                connection_id: ConnectionId(7),
                packet: ClientboundPlayPacket::UpdateRecipes(_),
            } => Some("recipes"),
            WorldEvent::Send {
                connection_id: ConnectionId(7),
                packet: ClientboundPlayPacket::RecipeBookSettings(_),
            } => Some("settings"),
            WorldEvent::Send {
                connection_id: ConnectionId(7),
                packet: ClientboundPlayPacket::RecipeBookAdd(add),
            } if add.replace => Some("book"),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(packet_order, ["tags", "recipes", "settings", "book"]);
}

#[test]
fn datapack_reload_keeps_the_old_publication_if_a_live_level_type_is_missing() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let mut startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .expect("the built-in pack always loads");
    startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .retain(|element| element.id != "minecraft:the_nether");

    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(startup)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    assert!(world.datapacks().is_none());
    assert_eq!(
        world
            .synchronized_registries()
            .dimension_type_index("minecraft:the_nether"),
        Some(3)
    );
}

#[test]
fn reload_succeeds_when_dynamic_registry_json_on_disk_is_modified_and_updates_tags_recipes_and_functions(
) {
    let dir = std::env::temp_dir().join(format!("bbb-reload-frozen-{}", Uuid::new_v4()));
    let datapacks_dir = dir.join("datapacks");
    let pack_dir = datapacks_dir.join("testpack");
    std::fs::create_dir_all(pack_dir.join("data/testpack/damage_type")).unwrap();
    std::fs::create_dir_all(pack_dir.join("data/testpack/tags/damage_type")).unwrap();
    std::fs::create_dir_all(pack_dir.join("data/testpack/tags/item")).unwrap();
    std::fs::create_dir_all(pack_dir.join("data/testpack/recipe")).unwrap();
    std::fs::create_dir_all(pack_dir.join("data/testpack/function")).unwrap();
    std::fs::write(
        pack_dir.join("pack.mcmeta"),
        r#"{"pack":{"description":"testpack","min_format":101,"max_format":101}}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/damage_type/custom.json"),
        r#"{"message_id":"custom","scaling":"never","exhaustion":0.1}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/tags/damage_type/custom_group.json"),
        r#"{"values":["testpack:custom"]}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/tags/item/reloadable.json"),
        r#"{"values":["minecraft:stick"]}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/recipe/reload_craft.json"),
        r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:stick"],"result":{"id":"minecraft:apple","count":1}}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/function/ping.mcfunction"),
        "say before_reload\n",
    )
    .unwrap();

    let boot_options = bbb_server_datapack::startup::DatapackStartupOptions {
        datapacks_dir: Some(datapacks_dir.clone()),
        saved_data_packs: Some(bbb_server_types::save::Java26DataPacks::new(
            vec!["vanilla".to_string(), "file/testpack".to_string()],
            Vec::new(),
        )),
        safe_mode: false,
        limits: Default::default(),
    };
    let startup = Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(&boot_options)
            .expect("boot datapack startup succeeds"),
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let published_before = world.synchronized_registries().published_shared();
    let (_, _) = super::join_loaded_ticks(&sender, &receiver, &mut world, 1, "reloader", 1);
    world.op_player(
        crate::admin::NameAndId {
            id: profile("reloader", 1).uuid,
            name: "reloader".into(),
        },
        &mut Vec::new(),
    );
    let before_fn =
        super::chat_command(&sender, &receiver, &mut world, 1, "function testpack:ping");
    assert!(
        before_fn.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::DisguisedChat(chat),
            } if chat.message.text().contains("before_reload")
        )),
        "{before_fn:?}"
    );

    // Mutate and corrupt dynamic-registry JSON files on disk after world boot,
    // while updating reloadable tags, recipes, and functions.
    std::fs::write(
        pack_dir.join("data/testpack/damage_type/custom.json"),
        r#"{"not_valid_damage_type": true"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/damage_type/added_after_boot.json"),
        r#"{"message_id":"added","scaling":"always","exhaustion":0.5}"#,
    )
    .unwrap();
    std::fs::remove_file(pack_dir.join("data/testpack/tags/damage_type/custom_group.json"))
        .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/tags/item/reloadable.json"),
        r#"{"values":["minecraft:blaze_rod"]}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/recipe/reload_craft.json"),
        r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:stick"],"result":{"id":"minecraft:golden_apple","count":1}}"#,
    )
    .unwrap();
    std::fs::write(
        pack_dir.join("data/testpack/function/ping.mcfunction"),
        "say after_reload\n",
    )
    .unwrap();

    let reload_options = bbb_server_datapack::startup::DatapackStartupOptions {
        datapacks_dir: Some(datapacks_dir),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    };
    let reloaded = bbb_server_datapack::startup::DatapackStartup::reload(
        &reload_options,
        world.datapacks().expect("world holds startup datapacks"),
    )
    .expect("/reload succeeds against frozen startup registries despite modified dynamic registry JSON on disk");
    assert_eq!(
        reloaded.selection.ordered_ids,
        vec!["vanilla".to_string(), "file/testpack".to_string()]
    );

    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::new(reloaded)),
        },
    )
    .unwrap();
    let reload_tick = world.tick(receiver.drain());

    // Frozen synchronized registries pointer is preserved untouched.
    assert!(Arc::ptr_eq(
        &published_before,
        &world.synchronized_registries().published_shared()
    ));
    let blaze_rod_id = bbb_facts::item_types::VANILLA_ITEM_BLAZE_ROD_ID;
    assert!(reload_tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::UpdateTags(update),
        } if update
            .registries
            .iter()
            .any(|registry| registry.registry == "minecraft:item"
                && registry
                    .tags
                    .iter()
                    .any(|tag| tag.tag == "testpack:reloadable" && tag.entries == [blaze_rod_id]))
    )));
    assert!(reload_tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::UpdateRecipes(_),
        }
    )));
    assert!(world
        .datapacks()
        .unwrap()
        .recipes
        .recipes
        .iter()
        .any(|recipe| recipe.id == "testpack:reload_craft"));

    let after_fn = super::chat_command(&sender, &receiver, &mut world, 1, "function testpack:ping");
    assert!(
        after_fn.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::DisguisedChat(chat),
            } if chat.message.text().contains("after_reload")
        )),
        "{after_fn:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
