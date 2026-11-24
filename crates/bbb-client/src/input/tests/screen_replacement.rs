use super::*;

#[test]
fn debug_screens_replacing_a_screen_run_its_removed() {
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_sign_text(
        BlockPos { x: 3, y: 64, z: 3 },
        ["", "", "", ""],
        ["", "", "", ""],
    );
    world.apply_open_sign_editor(OpenSignEditor {
        pos: ProtocolBlockPos { x: 3, y: 64, z: 3 },
        is_front_text: true,
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "kept");

    input.open_debug_options_screen(&mut counters, &commands);
    assert!(input.debug_options_screen_is_open());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::SignUpdate(SignUpdate { lines, .. })) if lines[0] == "kept"
    ));

    open_test_advancements_screen(&mut input);
    input.open_debug_pause_screen_with_menu(&world, &mut counters, &commands);
    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn inventory_opened_and_closed_by_keys_leaves_no_request_to_close_a_later_chat() {
    let (tx, _rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_local_player_id(77);
    world.take_screen_requests();

    for key in [KeyCode::KeyE, KeyCode::Escape, KeyCode::KeyT] {
        for state in [ElementState::Pressed, ElementState::Released] {
            handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(key),
                state,
            );
        }
    }
    assert!(!world.local_inventory_is_open());
    assert!(input.screen.chat_entry().is_some());

    // The next packet drains whatever the batch queued.
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input.screen.chat_entry().is_some());
}

#[test]
fn server_container_close_and_level_load_close_non_container_screens() {
    let mut world = world_with_local_player_id(77);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    open_test_book_screen(&mut input, &mut world, vec!["First"]);
    let book = input.screen.current().unwrap().clone();
    let screens = [
        book,
        Screen::CommandEntry(ChatEntryState::default()),
        Screen::Stats(Default::default()),
        Screen::Pause(Default::default()),
    ];

    for screen in screens {
        input.set_screen_for_test(screen.clone());
        world.apply_container_close(bbb_protocol::packets::ContainerClose { container_id: 99 });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(!input.screen.is_open());

        input.set_screen_for_test(screen);
        world.apply_respawn(&bbb_protocol::packets::Respawn {
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:the_nether".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            data_to_keep: 0,
        });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(!input.screen.is_open());
    }
}
