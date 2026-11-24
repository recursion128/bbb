mod advancement_rewards;

use super::*;

fn command_data(world: &ServerWorld, id: i32) -> &crate::entity::CommandBlockMinecartData {
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .expect("command minecart is live");
    let crate::entity::EntityData::CommandBlockMinecart(data) = entity.data() else {
        panic!("expected command minecart")
    };
    data
}

fn spawn_command_minecart(world: &mut ServerWorld) -> i32 {
    world.spawn_command_minecart_for_test(
        Java26BuiltinDimension::Overworld,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    )
}

#[test]
fn set_command_minecart_respects_permission_enabled_projection_and_feedback() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let id = spawn_command_minecart(&mut world);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work false",
    );
    let disabled = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCommandMinecart(SetCommandMinecart {
            entity_id: id,
            command: "seed".into(),
            track_output: false,
        }),
    );
    assert!(system_chats(&disabled)
        .iter()
        .any(|(_, text)| text.contains("advMode.setCommand.disabled")));
    let data = command_data(&world, id);
    assert_eq!(data.command(), "seed");
    assert_eq!(data.synched_command(), "");
    assert!(!data.track_output());

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work true",
    );
    let enabled = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCommandMinecart(SetCommandMinecart {
            entity_id: id,
            command: "seed".into(),
            track_output: true,
        }),
    );
    assert!(system_chats(&enabled)
        .iter()
        .any(|(_, text)| text.contains("advMode.setCommand.success")));
    let data = command_data(&world, id);
    assert_eq!(data.synched_command(), "seed");
    assert!(data.track_output());
}

#[test]
fn pending_execution_runs_as_stored_entity_and_commits_output_state() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let id = spawn_command_minecart(&mut world);
    let game_time = world.game_time();
    world.edit_command_minecart_for_test(Java26BuiltinDimension::Overworld, id, "seed", true);
    world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, id);
    let mut events = Vec::new();
    world.run_pending_command_minecart_executions_for_test(
        Java26BuiltinDimension::Overworld,
        &mut events,
    );

    let data = command_data(&world, id);
    assert_eq!(data.success_count(), 1);
    assert_eq!(data.last_execution(), game_time);
    assert!(data
        .last_output()
        .is_some_and(|output| output.text().contains("commands.seed.success")));
    assert_eq!(data.synched_last_output(), data.last_output().unwrap());
}

#[test]
fn stored_self_selector_reaches_the_live_command_minecart() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let id = spawn_command_minecart(&mut world);
    world.edit_command_minecart_for_test(Java26BuiltinDimension::Overworld, id, "kill @s", false);
    world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, id);
    world.run_pending_command_minecart_executions_for_test(
        Java26BuiltinDimension::Overworld,
        &mut Vec::new(),
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .is_none());
}

#[test]
fn client_information_command_minecart_chat_respects_recipient_options() {
    use bbb_protocol::packets::ClientChatVisibility;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let id = spawn_command_minecart(&mut world);
    for (visibility, delivered) in [
        (ClientChatVisibility::Full, true),
        (ClientChatVisibility::System, false),
        (ClientChatVisibility::Hidden, false),
    ] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientInformation(ClientInformation {
                chat_visibility: visibility,
                ..Default::default()
            }),
        );
        world.edit_command_minecart_for_test(
            Java26BuiltinDimension::Overworld,
            id,
            "say MARK-cart",
            true,
        );
        world.queue_command_minecart_execution_for_test(Java26BuiltinDimension::Overworld, id);
        let mut events = Vec::new();
        world.run_pending_command_minecart_executions_for_test(
            Java26BuiltinDimension::Overworld,
            &mut events,
        );
        assert_eq!(
            events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::DisguisedChat(chat), .. }
            if chat.message.text().contains("MARK-cart"))),
            delivered
        );
    }
}

#[test]
fn custom_source_names_survive_command_bans_and_saved_json() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for (index, units) in [
        vec![0xd800],
        vec![0xd801],
        vec![0xfffd],
        vec![0xd800; crate::admin::MAX_ADMIN_TEXT_CHARS + 1],
    ]
    .into_iter()
    .enumerate()
    {
        let source = bbb_protocol::JavaString::from(units);
        let target = format!("target{index}");
        let address = format!("203.0.113.{}", index + 1);
        for command in [format!("ban {target}"), format!("ban-ip {address}")] {
            let id = spawn_command_minecart(&mut world);
            let bbb_protocol::NbtSummaryValue::Compound(mut saved) = world
                .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
            else {
                panic!("entity saves as a compound");
            };
            saved.push(bbb_protocol::NbtSummaryEntry {
                name: "CustomName".into(),
                value: bbb_protocol::NbtSummaryValue::String(source.clone()),
            });
            world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &saved);
            let uuid = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .uuid();
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!("execute as {uuid} run {command}"),
            );
        }
        let user = crate::admin::NameAndId::offline(&target);
        assert_eq!(world.admin_lists().player_bans[&user.id].ban.source, source);
        assert_eq!(world.admin_lists().ip_bans[&address].source, source);
        let documents = world.admin_documents();
        let lists = crate::admin::StoredUserLists::parse(&documents).unwrap();
        assert_eq!(lists.player_bans[&user.id].ban.source, source);
        assert_eq!(lists.ip_bans[&address].source, source);
        let restored = ServerWorld::new(ServerWorldConfig {
            admin_lists: lists,
            ..creative_fixture_config()
        })
        .unwrap();
        assert_eq!(
            restored.admin_lists().player_bans[&user.id].ban.source,
            source
        );
        assert_eq!(restored.admin_lists().ip_bans[&address].source, source);
    }
}
