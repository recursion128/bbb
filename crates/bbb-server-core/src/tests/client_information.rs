use super::*;
use bbb_protocol::packets::ClientChatVisibility;

fn visibility(sender: &WorldCommandSender, id: u64, value: ClientChatVisibility) {
    enqueue(
        sender,
        WorldCommand::Play {
            connection_id: ConnectionId(id),
            packet: PlayServerbound::ClientInformation(ClientInformation {
                chat_visibility: value,
                ..Default::default()
            }),
        },
    )
    .unwrap();
}

fn recipients(tick: &TickResult) -> Vec<u64> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } if chat_content(packet).is_some() => Some(connection_id.0),
            _ => None,
        })
        .collect()
}

#[test]
fn client_information_chat_channels_and_hidden_sender_refusal() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    for (id, name) in [(1, "alice"), (2, "bob"), (3, "carol")] {
        join_loaded(&sender, &receiver, &mut world, id, name, id.into());
    }
    visibility(&sender, 2, ClientChatVisibility::System);
    visibility(&sender, 3, ClientChatVisibility::Hidden);
    world.tick(receiver.drain());
    for command in ["say hello", "me hello"] {
        assert_eq!(
            recipients(&chat_command(&sender, &receiver, &mut world, 1, command)),
            [1]
        );
    }
    assert_eq!(
        recipients(&chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "tellraw @a \"system\""
        )),
        [1, 2]
    );
    assert_eq!(
        recipients(&chat_command(&sender, &receiver, &mut world, 2, "list")),
        [2]
    );
    assert!(recipients(&chat_command(&sender, &receiver, &mut world, 3, "list")).is_empty());
    assert!(recipients(&chat_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "invalid_command"
    ))
    .is_empty());
    assert_eq!(
        recipients(&chat_command(
            &sender,
            &receiver,
            &mut world,
            2,
            "msg alice hello"
        )),
        [1]
    );
    assert_eq!(
        recipients(&chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "msg bob hello"
        )),
        [1]
    );
    assert!(recipients(&chat_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "msg bob hello"
    ))
    .is_empty());
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(50, "chat-command", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "say managed".to_owned(),
            },
        },
    )
    .unwrap();
    assert_eq!(recipients(&world.tick(receiver.drain())), [1]);
    let chat = |message: &str| PlayServerbound::Chat(ChatMessage::unsigned(message, 0, 0));
    let system_sender = play(&sender, &receiver, &mut world, 2, chat("hello"));
    assert_eq!(
        chat_lines(&world, &system_sender),
        [(ConnectionId(1), "chat.type.text bobhello".to_owned())]
    );
    let hidden_sender = play(&sender, &receiver, &mut world, 3, chat("rejected"));
    assert_eq!(
        system_chats(&hidden_sender),
        [(ConnectionId(3), "chat.disabled.options".to_owned())]
    );
    let mut signed = ChatMessage::unsigned("also rejected", 0, 0);
    signed.signature = Some(bbb_protocol::packets::MessageSignature {
        bytes: vec![0; 256],
    });
    let hidden_signed = play(
        &sender,
        &receiver,
        &mut world,
        3,
        PlayServerbound::Chat(signed),
    );
    assert_eq!(
        system_chats(&hidden_signed),
        [(ConnectionId(3), "chat.disabled.options".to_owned())]
    );
    assert_eq!(hidden_signed.snapshot.players.len(), 3);
}

#[test]
fn client_information_hidden_self_kick_does_not_gain_feedback_after_removal() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    visibility(&sender, 1, ClientChatVisibility::Hidden);
    world.tick(receiver.drain());
    let kicked = chat_command(&sender, &receiver, &mut world, 1, "kick alice");
    assert!(kicked.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            ..
        }
    )));
    assert!(system_chats(&kicked).is_empty());
}
