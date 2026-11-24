//! `ServerPlayer.die` (`ServerPlayer.java:869-929`) for a session:
//! vanishing-cursed stacks are destroyed, the rest of the inventory drops and
//! the experience reward becomes orbs, all unless `keep_inventory`; the menus
//! drop their stacks with the body; the death message follows the team's
//! `deathMessageVisibility`; and the death location reaches the respawn packet
//! and the save.

use super::*;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn survival_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

fn ground_items(world: &ServerWorld) -> Vec<(String, i32)> {
    let mut items: Vec<(String, i32)> = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(DIM, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((stack.item_resource_id()?.to_string(), stack.count()))
        })
        .collect();
    items.sort();
    items
}

fn orb_experience(world: &ServerWorld) -> i32 {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| match world.stored_entity_for_test(DIM, id)?.data() {
            crate::entity::EntityData::ExperienceOrb(orb) => Some(orb.value() * orb.count()),
            _ => None,
        })
        .sum()
}

fn arm(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    for command in [
        "give alice minecraft:stone 10",
        "give alice minecraft:diamond_sword[enchantments={\"minecraft:vanishing_curse\":1}]",
        "item replace entity alice armor.head with minecraft:iron_helmet",
        "experience set alice 10 levels",
    ] {
        chat_command(sender, receiver, world, 1, command);
    }
}

#[test]
fn a_dying_session_drops_its_inventory_and_experience_but_not_cursed_items() {
    let (sender, receiver, mut world) = survival_world();
    arm(&sender, &receiver, &mut world);
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    world.tick(receiver.drain());

    assert_eq!(
        ground_items(&world),
        [
            ("minecraft:iron_helmet".to_string(), 1),
            ("minecraft:stone".to_string(), 10),
        ]
    );
    // `min(experienceLevel * 7, 100)` (`Player.java:1604-1606`).
    assert_eq!(orb_experience(&world), 70);
}

#[test]
fn player_death_runs_its_entity_loot_table_under_mob_drops() {
    let root = std::env::temp_dir().join(format!("bbb-player-death-loot-{}", Uuid::new_v4()));
    let pack = root.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/entities")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"player death loot","pack_format":1000,"supported_formats":[0,10000]}}"#,
    ).unwrap();
    std::fs::write(
        pack.join("data/minecraft/loot_table/entities/player.json"),
        serde_json::to_vec(&serde_json::json!({
            "type":"minecraft:entity",
            "pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}]
        }))
        .unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let startup = Arc::new(startup);
    std::fs::remove_dir_all(root).unwrap();
    for mob_drops in [true, false] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            game_type: GameType::Survival,
            datapacks: Some(startup.clone()),
            ..operator_config(&[("alice", 1)])
        });
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        for command in [
            format!("gamerule mob_drops {mob_drops}"),
            "give alice minecraft:stone 10".into(),
            "kill alice".into(),
        ] {
            chat_command(&sender, &receiver, &mut world, 1, &command);
        }
        let mut expected = vec![("minecraft:stone".to_string(), 10)];
        if mob_drops {
            expected.insert(0, ("minecraft:diamond".to_string(), 1));
        }
        assert_eq!(ground_items(&world), expected, "mob_drops={mob_drops}");
    }
}

#[test]
fn keep_inventory_keeps_everything_and_drops_no_experience() {
    let (sender, receiver, mut world) = survival_world();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule keep_inventory true",
    );
    arm(&sender, &receiver, &mut world);
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    world.tick(receiver.drain());

    assert!(ground_items(&world).is_empty());
    assert_eq!(orb_experience(&world), 0);
}

fn tick_until_body_removed(receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    // `LivingEntity.tickDeath` removes the body twenty ticks after death.
    for _ in 0..21 {
        world.tick(receiver.drain());
    }
}

#[test]
fn the_crafting_grid_drops_with_the_body_even_under_keep_inventory() {
    let (sender, receiver, mut world) = survival_world();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule keep_inventory true",
    );
    let stack = |item_id, count| ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    };
    // Slots 1-4 of `InventoryMenu` are the 2x2 grid.
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        1,
        stack(bbb_protocol::item_types::VANILLA_ITEM_OAK_PLANKS_ID, 3),
    );
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        4,
        stack(bbb_protocol::item_types::VANILLA_ITEM_STICK_ID, 2),
    );
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    tick_until_body_removed(&receiver, &mut world);

    assert_eq!(
        ground_items(&world),
        [
            ("minecraft:oak_planks".to_string(), 3),
            ("minecraft:stick".to_string(), 2),
        ]
    );
}

fn death_message_recipients(tick: &TickResult) -> Vec<ConnectionId> {
    let mut recipients: Vec<_> = system_chats(tick)
        .into_iter()
        .filter(|(_, text)| text.contains("death.attack.genericKill"))
        .map(|(connection_id, _)| connection_id)
        .collect();
    recipients.sort();
    recipients
}

/// `Team.Visibility` (`ServerPlayer.java:890-897`; `PlayerList.java:489-513`).
#[test]
fn the_death_message_reaches_the_players_the_team_visibility_allows() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    for command in ["team add red", "team join red alice", "team join red bob"] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    let cases = [
        ("always", vec![1, 2, 3]),
        ("hideForOtherTeams", vec![2]),
        ("hideForOwnTeam", vec![3]),
        ("never", vec![]),
    ];
    for (visibility, expected) in cases {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("team modify red deathMessageVisibility {visibility}"),
        );
        let died = chat_command(&sender, &receiver, &mut world, 1, "kill alice");
        assert_eq!(
            death_message_recipients(&died),
            expected.into_iter().map(ConnectionId).collect::<Vec<_>>(),
            "{visibility}"
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerLoaded,
        );
    }
}

/// `ServerPlayer.die` records `lastDeathLocation` (`:927`), which the respawn
/// packet carries (`createCommonSpawnInfo`, `:2160`) and the save writes and
/// reads back (`Player.java:642`, `:660`) for the next login.
#[test]
fn the_death_location_reaches_the_respawn_packet_the_save_and_the_next_login() {
    let config = || ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config(), &restore_candidate(0, None, vec![])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: -2.5,
        },
    );
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    let expected = Some(bbb_protocol::packets::GlobalPos {
        dimension: "minecraft:overworld".to_string(),
        pos: BlockPos { x: 3, y: 64, z: -3 },
    });
    let spawn_info = |tick: &TickResult| {
        tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Respawn(respawn),
                ..
            } => Some(respawn.common_spawn_info.last_death_location.clone()),
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login.common_spawn_info.last_death_location.clone()),
            _ => None,
        })
    };
    assert_eq!(spawn_info(&respawned), Some(expected.clone()));

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert_eq!(parsed.last_death_location(), expected.as_ref());

    let mut restored = ServerWorld::restore(
        config(),
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut restored, 1, "alice", 1);
    assert_eq!(spawn_info(&joined), Some(expected));
}

/// `CombatTracker.getFallMessage` (CombatTracker.java:62-80): a fall of more
/// than five blocks is the most significant one, with no climbable or water
/// behind it.
#[test]
fn a_lethal_fall_names_the_accident() {
    let (sender, receiver, mut world) = survival_world();
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let mut step = |y: f64, on_ground: bool| {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MovePlayer(bbb_protocol::packets::MovePlayer {
                position: Some(Vec3d { x: 0.5, y, z: 0.5 }),
                rotation: None,
                on_ground,
                horizontal_collision: false,
            }),
        )
    };
    for y in [72.0, 80.0, 88.0, 96.0, 104.0, 96.0, 88.0, 80.0, 72.0] {
        step(y, false);
    }
    let landed = step(64.0, true);
    let messages: Vec<_> = system_chats(&landed)
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert!(
        messages
            .iter()
            .any(|text| text.contains("death.fell.accident.generic")),
        "{messages:?}"
    );
}

/// `ServerPlayer.restoreFrom` keeps the experience and score of a spectator's
/// death even without `keep_inventory` (`ServerPlayer.java:1605-1607`).
#[test]
fn a_spectator_death_respawns_with_its_experience() {
    let (sender, receiver, mut world) = survival_world();
    arm(&sender, &receiver, &mut world);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator");
    chat_command(&sender, &receiver, &mut world, 1, "kill alice");
    world.tick(receiver.drain());
    assert_eq!(orb_experience(&world), 0);
    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert!(respawned.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Respawn(_),
            ..
        }
    )));
    assert_eq!(world.experience_level_for_test(ConnectionId(1)), 10);
}
