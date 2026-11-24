use super::*;
use crate::control::{ControlWriteRequest, InputKeyControl, InputMouseButtonControl};
use bbb_client_render::{HudAsciiGlyph, HudChatInputView};

struct Driver {
    runtime: SimulationInputRuntime,
    net_rx: mpsc::Receiver<NetCommand>,
}

#[tokio::test]
async fn native_key_text_edits_chat_without_shortcut_or_release_duplicates() {
    with_context(|driver, context| {
        driver.runtime.set_cursor_captured(true);
        for (key, text) in [
            (KeyCode::KeyT, "t"),
            (KeyCode::KeyN, "n"),
            (KeyCode::KeyY, "Y"),
        ] {
            driver.key(context, key, ElementState::Pressed);
            driver.send(
                context,
                SimulationCommand::native_key_text(ElementState::Pressed, Some(text)).unwrap(),
            );
            driver.key(context, key, ElementState::Released);
            if let Some(text) =
                SimulationCommand::native_key_text(ElementState::Released, Some(text))
            {
                driver.send(context, text);
            }
        }
        assert_eq!(String::from_utf16_lossy(&driver.view(context).value), "nY");
        driver.key(context, KeyCode::ControlLeft, ElementState::Pressed);
        driver.send(
            context,
            SimulationCommand::Key {
                physical_key: PhysicalKey::Code(KeyCode::KeyV),
                state: ElementState::Pressed,
                clipboard_text: Some("你好".into()),
            },
        );
        if let Some(text) =
            SimulationCommand::native_key_text(ElementState::Pressed, Some("\u{16}"))
        {
            driver.send(context, text);
        }
        driver.key(context, KeyCode::KeyV, ElementState::Released);
        driver.key(context, KeyCode::ControlLeft, ElementState::Released);
        assert_eq!(
            String::from_utf16_lossy(&driver.view(context).value),
            "nY你好"
        );
        driver.key(context, KeyCode::Enter, ElementState::Pressed);
        if let Some(text) = SimulationCommand::native_key_text(ElementState::Pressed, Some("\r")) {
            driver.send(context, text);
        }
        assert!(!context.input.chat_entry_is_active());
        assert_eq!(context.net_counters.chat_message_commands_queued, 1);
    });
}

impl Driver {
    fn send(&mut self, context: &mut SimulationInputContext<'_, '_>, command: SimulationCommand) {
        self.runtime.apply_commands(vec![command], context, false);
    }

    fn controls(
        &mut self,
        context: &mut SimulationInputContext<'_, '_>,
        requests: impl IntoIterator<Item = InputControlRequest>,
    ) {
        self.runtime.apply_commands(
            requests
                .into_iter()
                .map(|request| SimulationCommand::Control(ControlWriteRequest::Input(request)))
                .collect(),
            context,
            false,
        );
    }

    fn key(
        &mut self,
        context: &mut SimulationInputContext<'_, '_>,
        key: KeyCode,
        state: ElementState,
    ) {
        self.send(
            context,
            SimulationCommand::Key {
                physical_key: PhysicalKey::Code(key),
                state,
                clipboard_text: None,
            },
        );
    }

    fn resize(&mut self, context: &mut SimulationInputContext<'_, '_>, width: u32, height: u32) {
        self.send(
            context,
            SimulationCommand::SurfaceResized {
                size: PhysicalSize::new(width, height),
            },
        );
    }

    fn open(&mut self, context: &mut SimulationInputContext<'_, '_>, text: &str) {
        self.runtime.set_cursor_captured(true);
        self.key(context, KeyCode::KeyT, ElementState::Pressed);
        self.key(context, KeyCode::KeyT, ElementState::Released);
        self.send(context, SimulationCommand::ImeCommit("t".to_string()));
        self.send(context, SimulationCommand::ImeCommit(text.to_string()));
        assert!(context.input.chat_entry_is_active());
    }

    fn view(&self, context: &mut SimulationInputContext<'_, '_>) -> HudChatInputView {
        self.hud(context).input_view.unwrap()
    }

    fn hud(
        &self,
        context: &mut SimulationInputContext<'_, '_>,
    ) -> crate::input::CommandSuggestionHudState {
        context
            .input
            .command_suggestion_hud_state(
                context.world,
                &self.runtime.text_measurements,
                self.runtime.gui_surface_size(),
            )
            .unwrap()
    }

    fn mouse(
        &mut self,
        context: &mut SimulationInputContext<'_, '_>,
        state: ElementState,
        x: f64,
        y: f64,
        target: Option<HudChatHitTarget>,
    ) {
        self.send(
            context,
            SimulationCommand::MouseButton {
                button: MouseButton::Left,
                state,
                position: Some(PhysicalPosition::new(x, y)),
                chat_target: target,
                book_target: None,
                disconnected_screen_target: None,
                command_confirmation_target: None,
                death_screen_target: None,
                game_event_target: None,
                social_interactions_target: None,
            },
        );
    }

    fn moved(&mut self, context: &mut SimulationInputContext<'_, '_>, x: f64, y: f64) {
        self.send(
            context,
            SimulationCommand::CursorMoved {
                position: PhysicalPosition::new(x, y),
                disconnected_screen_target: None,
                death_screen_target: None,
                game_event_target: None,
                social_interactions_target: None,
            },
        );
    }
}

fn measurements(advance: f32) -> HudFontGlyphMap {
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.set_missing_glyph(HudAsciiGlyph::empty(advance));
    glyphs.insert_selected_codepoint_first_wins(
        'N' as u32,
        HudAsciiGlyph::empty(-10.0),
        HudAsciiGlyph::empty(1.25),
        -10.0,
    );
    glyphs.insert_selected_codepoint_first_wins(
        '😀' as u32,
        HudAsciiGlyph::empty(12.0),
        HudAsciiGlyph::empty(12.0),
        12.0,
    );
    glyphs
}

fn with_context(run: impl FnOnce(&mut Driver, &mut SimulationInputContext<'_, '_>)) {
    use crate::control::NetCounters;
    use tokio::runtime::Handle;
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let (sender, net_rx) = mpsc::channel(128);
    let commands = Some(sender);
    let mut hands = crate::item_models::FirstPersonHandPresentation::default();
    let cache_dir = TestServerPackCacheDir::new("chat");
    let mut packs = ServerResourcePackRuntime::new(
        Handle::current(),
        cache_dir.path(),
        "chat-simulation-tests",
        uuid::Uuid::nil(),
        None,
    )
    .unwrap();
    let mut items = None;
    let mut win = std::sync::Arc::new(crate::win_screen_resources::WinScreenResources {
        lines: Vec::new(),
        narration_components: Vec::new(),
        total_scroll_length: 0,
    });
    let mut random = bbb_client_world::LevelEventSoundRandomState::default();
    let mut audio: Option<&mut dyn AudioEventSink> = None;
    let mut platform = crate::simulation::effects::PlatformEffects::default();
    let mut context = SimulationInputContext {
        input: &mut input,
        now: Instant::now(),
        first_person_hand_presentation: &mut hands,
        world: &mut world,
        net_counters: &mut counters,
        net_commands: &commands,
        server_resource_packs: &mut packs,
        item_runtime: &mut items,
        code_of_conduct: None,
        win_screen_resources: &mut win,
        entity_partial_tick: 0.0,
        level_random: &mut random,
        audio_events: &mut audio,
        platform: &mut platform,
    };
    let mut driver = Driver {
        runtime: SimulationInputRuntime::new(
            PhysicalSize::new(320, 240),
            false,
            ClientInformation::default(),
        ),
        net_rx,
    };
    driver.runtime.set_gui_scale_options(1, false);
    driver.send(
        &mut context,
        SimulationCommand::TextMeasurements(measurements(6.0)),
    );
    run(&mut driver, &mut context);
}

#[tokio::test]
async fn control_input_opens_local_inventory_through_the_real_key_handler() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::Key {
                    key: InputKeyControl::Inventory,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Inventory,
                    pressed: false,
                },
            ],
        );
        assert!(context.world.inventory().local_inventory_open);
    });
}

#[tokio::test]
async fn container_keyboard_close_recaptures_cursor_before_chat() {
    for close_key in [KeyCode::Escape, KeyCode::KeyE] {
        with_context(|driver, context| {
            context
                .world
                .apply_open_screen(bbb_protocol::packets::OpenScreen {
                    container_id: 7,
                    menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_SHULKER_BOX_ID,
                    title: bbb_protocol::RichTextComponent::plain("Shulker Box"),
                });
            context.input.apply_screen_requests(
                context.world,
                context.net_counters,
                context.net_commands,
            );
            driver.runtime.apply_commands(Vec::new(), context, false);
            assert!(context.input.inventory_screen_is_open());
            assert!(!driver.runtime.cursor_captured());

            driver.key(context, close_key, ElementState::Pressed);
            driver.key(context, close_key, ElementState::Released);
            assert!(context.input.current_screen().is_none());
            assert!(context.world.open_container_id().is_none());
            assert!(driver.runtime.cursor_captured());
            assert!(matches!(driver.net_rx.try_recv(),
                Ok(NetCommand::ContainerClose(request)) if request.container_id == 7
            ));

            driver.key(context, KeyCode::KeyT, ElementState::Pressed);
            driver.key(context, KeyCode::KeyT, ElementState::Released);
            assert!(context.input.chat_entry_is_active());
            assert!(!driver.runtime.cursor_captured());
        });
    }
}

#[tokio::test]
async fn control_chat_text_and_history_use_the_real_screen_handlers() {
    with_context(|driver, context| {
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::Key {
                    key: InputKeyControl::Chat,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Chat,
                    pressed: false,
                },
                InputControlRequest::Text {
                    text: "history message".to_string(),
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Enter,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Enter,
                    pressed: false,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Chat,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Chat,
                    pressed: false,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Up,
                    pressed: true,
                },
            ],
        );
        let status = context
            .input
            .chat_screen_status(context.world.chat_presentation_config().visibility)
            .expect("reopened chat screen");
        assert_eq!(status.screen, crate::control::ChatScreenControlKind::Chat);
        assert_eq!(status.input_text, "history message");
        assert_eq!(status.input_utf16_units, 15);
        assert_eq!(status.history_position, 0);
        assert_eq!(status.recent_count, 1);
        assert!(!status.draft);
    });
}

#[tokio::test]
async fn control_command_key_opens_slash_entry_through_the_real_handler() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::Key {
                    key: InputKeyControl::Command,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Command,
                    pressed: false,
                },
                InputControlRequest::Text {
                    text: "me proof".to_string(),
                },
            ],
        );
        let status = context
            .input
            .chat_screen_status(context.world.chat_presentation_config().visibility)
            .expect("slash command entry");
        assert_eq!(status.method, crate::control::ChatMethodControl::Command);
        assert_eq!(status.input_text, "/me proof");
    });
}

#[tokio::test]
async fn control_input_selects_hotbar_slot_through_the_real_key_handler() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::Key {
                    key: InputKeyControl::Hotbar7,
                    pressed: true,
                },
                InputControlRequest::Key {
                    key: InputKeyControl::Hotbar7,
                    pressed: false,
                },
            ],
        );
        assert_eq!(context.world.client_local_player().selected_hotbar_slot, 6);
    });
}

#[tokio::test]
async fn control_input_replays_keys_through_the_real_key_handler() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::Key {
                    key: InputKeyControl::Forward,
                    pressed: true,
                },
                InputControlRequest::LookDelta { dx: 24.0, dy: -8.0 },
            ],
        );
        let projected = context.input.local_player_input(context.world);
        assert!(projected.forward);
        assert_eq!(projected.mouse_delta_x, 24.0);
        assert_eq!(projected.mouse_delta_y, -8.0);
        driver.controls(
            context,
            [InputControlRequest::Key {
                key: InputKeyControl::Forward,
                pressed: false,
            }],
        );
        assert!(!context.input.local_player_input(context.world).forward);
    });
}

#[tokio::test]
async fn control_input_drops_input_behind_the_code_of_conduct_overlay() {
    with_context(|driver, context| {
        driver.runtime.set_cursor_captured(true);
        context
            .world
            .apply_code_of_conduct("Keep chat friendly.".to_string());
        driver.controls(
            context,
            [InputControlRequest::Key {
                key: InputKeyControl::Jump,
                pressed: true,
            }],
        );
        assert!(!context.input.local_player_input(context.world).jump);
    });
}

#[tokio::test]
async fn control_input_replays_clicks_through_the_real_mouse_handler() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::MouseButton {
                    button: InputMouseButtonControl::Left,
                    pressed: true,
                },
            ],
        );
        assert_eq!(context.net_counters.swing_commands_queued, 1);
        assert!(matches!(driver.net_rx.try_recv(), Ok(NetCommand::Swing(_))));
    });
}

#[tokio::test]
async fn control_input_replays_atomic_mouse_tap_before_tick_advancement() {
    with_context(|driver, context| {
        *context.input = crate::input::ClientInputState::new(false);
        driver.runtime.set_cursor_captured(true);
        driver.controls(
            context,
            [
                InputControlRequest::Focus { focused: true },
                InputControlRequest::MouseTap {
                    button: InputMouseButtonControl::Left,
                },
            ],
        );
        assert_eq!(context.net_counters.swing_commands_queued, 1);
        assert!(matches!(driver.net_rx.try_recv(), Ok(NetCommand::Swing(_))));
        assert!(!context
            .input
            .item_model_keybind_context()
            .keybind_down("key.attack"));
    });
}

#[tokio::test]
async fn filtered_mouse_shift_drag_release_and_focus_use_command_dispatch() {
    with_context(|driver, context| {
        driver.open(context, "NNNN");
        driver.key(context, KeyCode::ShiftLeft, ElementState::Pressed);
        driver.mouse(context, ElementState::Pressed, 6.99, 230.0, None);
        let view = driver.view(context);
        assert_eq!((view.cursor, view.selection), (1, 4));
        driver.moved(context, -10.0, 0.0);
        let view = driver.view(context);
        assert_eq!((view.cursor, view.selection), (0, 4));
        driver.mouse(context, ElementState::Released, -10.0, 0.0, None);
        driver.moved(context, 300.0, 230.0);
        assert_eq!(driver.view(context), view);
        driver.key(context, KeyCode::ShiftLeft, ElementState::Released);
        driver.mouse(context, ElementState::Pressed, 6.99, 230.0, None);
        driver.send(context, SimulationCommand::FocusChanged(false));
        let stopped = driver.view(context);
        driver.moved(context, 300.0, 230.0);
        assert_eq!(driver.view(context), stopped);
        driver.send(context, SimulationCommand::FocusChanged(true));
        driver.moved(context, 4.0, 230.0);
        assert_eq!(driver.view(context), stopped);
    });
}

#[tokio::test]
async fn chat_wheel_and_page_keys_clamp_to_the_rendered_page_at_scroll_time() {
    with_context(|driver, context| {
        let wheel = |driver: &mut Driver, context: &mut SimulationInputContext<'_, '_>, y| {
            driver.send(
                context,
                SimulationCommand::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(0.0, y),
                    position: None,
                },
            );
            context.input.chat_scroll().position
        };
        let page = |lines| {
            SimulationCommand::HudChatScrollPage(bbb_client_render::HudChatScrollPage {
                lines,
                lines_per_page: 6,
            })
        };
        driver.open(context, "x");
        // No rendered page yet: `scrollChat` clamps against zero lines.
        assert_eq!(wheel(driver, context, 1.0), 0);
        driver.send(context, page(30));
        // ChatScreen: seven lines a notch, clamped to 30 - 6.
        assert_eq!(wheel(driver, context, 1.0), 7);
        assert_eq!(wheel(driver, context, 3.0), 14);
        assert_eq!(wheel(driver, context, 1.0), 21);
        assert_eq!(wheel(driver, context, 1.0), 24);
        assert_eq!(wheel(driver, context, -1.0), 17);
        // ChatScreen: a page key moves `linesPerPage - 1`.
        driver.key(context, KeyCode::PageDown, ElementState::Pressed);
        assert_eq!(context.input.chat_scroll().position, 12);
        driver.key(context, KeyCode::PageUp, ElementState::Pressed);
        driver.key(context, KeyCode::PageUp, ElementState::Pressed);
        assert_eq!(context.input.chat_scroll().position, 22);
        driver.key(context, KeyCode::PageUp, ElementState::Pressed);
        assert_eq!(context.input.chat_scroll().position, 24);
        // A smaller page applies at the next scroll, from the unclamped position.
        driver.send(context, page(10));
        assert_eq!(context.input.chat_scroll().position, 24);
        assert_eq!(wheel(driver, context, -1.0), 4);
        driver.key(context, KeyCode::ShiftLeft, ElementState::Pressed);
        assert_eq!(wheel(driver, context, -1.0), 3);
    });
}

#[tokio::test]
async fn new_chat_lines_follow_a_scrolled_focused_page_until_it_returns_to_the_bottom() {
    with_context(|driver, context| {
        let wheel = |driver: &mut Driver, context: &mut SimulationInputContext<'_, '_>, y| {
            driver.send(
                context,
                SimulationCommand::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(0.0, y),
                    position: None,
                },
            );
            context.input.chat_scroll()
        };
        let appended = |prior_lines, message_lines: &[usize]| {
            SimulationCommand::HudChatAppendedLines(bbb_client_render::HudChatAppendedLines {
                prior_lines,
                message_lines: message_lines.to_vec(),
            })
        };
        let scroll = |position, new_message_since_scroll| bbb_client_render::HudChatScroll {
            position,
            new_message_since_scroll,
        };
        driver.send(
            context,
            SimulationCommand::HudChatScrollPage(bbb_client_render::HudChatScrollPage {
                lines: 30,
                lines_per_page: 6,
            }),
        );
        driver.send(context, appended(30, &[2]));
        assert_eq!(context.input.chat_scroll(), scroll(0, false));
        driver.open(context, "x");
        // At the bottom a new line only pushes the page up.
        driver.send(context, appended(30, &[2]));
        assert_eq!(context.input.chat_scroll(), scroll(0, false));

        // ChatComponent: every line scrolls once against the lines
        // before it, here 32 - 6 then 33 - 6 and 34 - 6.
        assert_eq!(wheel(driver, context, 1.0), scroll(7, false));
        driver.send(context, appended(32, &[1, 2]));
        assert_eq!(context.input.chat_scroll(), scroll(10, true));
        // Only the scroll that reaches the bottom clears the colour.
        assert_eq!(wheel(driver, context, -1.0), scroll(3, true));
        assert_eq!(wheel(driver, context, -1.0), scroll(0, false));

        // A refresh re-adds every line to an empty list: the first clamps to 0.
        assert_eq!(wheel(driver, context, 1.0), scroll(7, false));
        driver.send(context, appended(0, &[1; 5]));
        assert_eq!(context.input.chat_scroll(), scroll(0, false));

        // `ChatScreen.removed` resets it when another screen replaces the chat
        // and when the chat closes.
        assert_eq!(wheel(driver, context, 1.0), scroll(7, false));
        driver.send(context, appended(35, &[1]));
        assert_eq!(context.input.chat_scroll(), scroll(8, true));
        assert!(context.input.open_chat_restrictions_screen());
        assert_eq!(context.input.chat_scroll(), scroll(0, false));
        assert!(context.input.close_chat_restrictions_screen());
        assert!(context.input.chat_entry_is_active());
        assert_eq!(wheel(driver, context, 1.0), scroll(7, false));
        driver.send(context, appended(35, &[1]));
        assert_eq!(context.input.chat_scroll(), scroll(8, true));
        driver.key(context, KeyCode::Escape, ElementState::Pressed);
        assert!(!context.input.chat_entry_is_active());
        assert_eq!(context.input.chat_scroll(), scroll(0, false));
    });
}

#[tokio::test]
async fn measurements_do_not_scroll_and_resize_rebuilds_saved_value_at_end() {
    with_context(|driver, context| {
        driver.resize(context, 22, 100);
        driver.open(context, "😀AB");
        assert_eq!(driver.view(context).display_start, 1);
        driver.mouse(context, ElementState::Pressed, 4.0, 90.0, None);
        driver.mouse(context, ElementState::Released, 4.0, 90.0, None);
        let split = driver.view(context);
        assert_eq!((split.cursor, split.selection), (1, 1));
        driver.send(
            context,
            SimulationCommand::TextMeasurements(measurements(10.0)),
        );
        assert_eq!(driver.view(context), split);
        driver.resize(context, 23, 100);
        let rebuilt = driver.view(context);
        assert_eq!(rebuilt.value, split.value);
        assert_eq!(
            (rebuilt.cursor, rebuilt.selection, rebuilt.display_start),
            (4, 4, 3)
        );
        driver.moved(context, 4.0, 90.0);
        assert_eq!(driver.view(context), rebuilt);
        let status = context
            .input
            .chat_screen_status(bbb_client_world::ChatVisibilityState::Full)
            .unwrap();
        assert_eq!(status.history_position, status.recent_count);
    });
}

fn target(
    context: &SimulationInputContext<'_, '_>,
    kind: HudChatHitTargetKind,
) -> HudChatHitTarget {
    HudChatHitTarget {
        level_generation: context.world.client_level_generation(),
        delivery_revision: context.world.client_chat().delivery_revision,
        chat_screen_revision: context.input.chat_screen_revision(),
        resource_generation: 0,
        resources_loaded: false,
        kind,
    }
}

fn clickable_message(
    context: &mut SimulationInputContext<'_, '_>,
    command: &str,
) -> HudChatHitTarget {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    let field = |name: &str, value| Entry {
        name: name.into(),
        value,
    };
    let component = bbb_protocol::RichTextComponent::try_from_component_nbt(Value::Compound(vec![
        field("text", Value::String("click".into())),
        field(
            "click_event",
            Value::Compound(vec![
                field("action", Value::String("suggest_command".into())),
                field("command", Value::String(command.into())),
            ]),
        ),
    ]))
    .unwrap();
    context
        .world
        .apply_system_chat(bbb_protocol::packets::SystemChat {
            content: Box::new(component),
            overlay: false,
        });
    target(
        context,
        HudChatHitTargetKind::Message {
            display_sequence: context
                .world
                .client_chat()
                .messages
                .last()
                .unwrap()
                .display_sequence,
            target_index: 0,
            click_target_index: Some(0),
        },
    )
}

/// One styled run carrying only `Style.insertion`, as the click finder sees it
/// with `includeInsertions(false)`: no click-only target at all.
fn insertion_only_message(
    context: &mut SimulationInputContext<'_, '_>,
    insertion: bbb_protocol::NbtSummaryValue,
) -> HudChatHitTarget {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    let field = |name: &str, value| Entry {
        name: name.into(),
        value,
    };
    let component = bbb_protocol::RichTextComponent::try_from_component_nbt(Value::Compound(vec![
        field("text", Value::String("click".into())),
        field("insertion", insertion),
    ]))
    .unwrap();
    context
        .world
        .apply_system_chat(bbb_protocol::packets::SystemChat {
            content: Box::new(component),
            overlay: false,
        });
    target(
        context,
        HudChatHitTargetKind::Message {
            display_sequence: context
                .world
                .client_chat()
                .messages
                .last()
                .unwrap()
                .display_sequence,
            target_index: 0,
            click_target_index: None,
        },
    )
}

#[tokio::test]
async fn message_click_precedes_edit_and_rejects_previous_screen_revision() {
    with_context(|driver, context| {
        driver.open(context, "typed");
        let message = clickable_message(context, "replacement");
        driver.mouse(context, ElementState::Pressed, 4.0, 230.0, Some(message));
        let view = driver.view(context);
        assert_eq!(String::from_utf16_lossy(&view.value), "replacement");
        assert_eq!((view.cursor, view.selection), (11, 11));
        let initial = "replacement".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            context.input.chat_input_initial_for_test(),
            Some(initial.as_slice())
        );
        driver.moved(context, 4.0, 230.0);
        assert_eq!(driver.view(context), view);
        driver.key(context, KeyCode::Escape, ElementState::Pressed);
        driver.key(context, KeyCode::Escape, ElementState::Released);
        driver.open(context, "fresh");
        let before = driver.view(context).value;
        assert_ne!(
            context.input.chat_screen_revision(),
            message.chat_screen_revision
        );
        driver.mouse(context, ElementState::Pressed, 4.0, 230.0, Some(message));
        let after = driver.view(context);
        assert_eq!(after.value, before);
        assert_eq!((after.cursor, after.selection), (0, 0));
    });
}

#[tokio::test]
async fn shift_insertion_target_inserts_and_a_plain_click_falls_through() {
    with_context(|driver, context| {
        driver.open(context, "typed");
        let message =
            insertion_only_message(context, bbb_protocol::NbtSummaryValue::String("INS".into()));
        let initial = context
            .input
            .chat_input_initial_for_test()
            .unwrap()
            .to_vec();
        // `includeInsertions(false)`: the click-only scan found nothing, so the
        // click belongs to the edit box and the insertion stays untouched.
        driver.mouse(context, ElementState::Pressed, 4.0, 230.0, Some(message));
        assert_eq!(
            String::from_utf16_lossy(&driver.view(context).value),
            "typed"
        );
        // Shift selects the insertion scan; `ChatScreen.handleComponentClicked`
        // then inserts the payload at the cursor instead of running a click
        // event.
        driver.key(context, KeyCode::ShiftLeft, ElementState::Pressed);
        driver.mouse(context, ElementState::Pressed, 4.0, 230.0, Some(message));
        assert_eq!(
            String::from_utf16_lossy(&driver.view(context).value),
            "INStyped"
        );
        let view = driver.view(context);
        assert_eq!((view.cursor, view.selection), (8, 0));
        assert_eq!(
            context.input.chat_input_initial_for_test(),
            Some(initial.as_slice())
        );
        driver.moved(context, -10.0, 0.0);
        let dragged = driver.view(context);
        assert_eq!((dragged.cursor, dragged.selection), (0, 0));
        driver.mouse(context, ElementState::Released, -10.0, 0.0, None);
        driver.moved(context, 28.0, 230.0);
        assert_eq!(driver.view(context), dragged);
        driver.key(context, KeyCode::ShiftLeft, ElementState::Released);
    });
}

#[tokio::test]
async fn suggestion_popup_consumes_before_message_and_edit() {
    with_context(|driver, context| {
        use bbb_protocol::packets::{CommandNode, CommandNodeType, Commands};
        let node = |node_type, name, children| CommandNode {
            node_type,
            flags: 0,
            children,
            redirect: None,
            name,
            parser: None,
            suggestions: None,
            executable: false,
            restricted: false,
        };
        context.world.apply_commands(Commands {
            root_index: 0,
            nodes: vec![
                node(CommandNodeType::Root, None, vec![1, 2]),
                node(CommandNodeType::Literal, Some("foo".into()), vec![]),
                node(CommandNodeType::Literal, Some("food".into()), vec![]),
            ],
        });
        driver.open(context, "/fo");
        let popup = driver.hud(context);
        assert_eq!(popup.entries.len(), 2);
        let message = clickable_message(context, "message won");
        driver.mouse(
            context,
            ElementState::Pressed,
            f64::from(popup.rect.x + 1),
            f64::from(popup.rect.y + 1),
            Some(message),
        );
        let view = driver.view(context);
        assert_eq!(String::from_utf16_lossy(&view.value), "/foo");
        assert_eq!((view.cursor, view.selection), (4, 4));
        driver.moved(context, 4.0, 230.0);
        assert_eq!(driver.view(context), view);
    });
}

#[tokio::test]
async fn reload_ack_rebuilds_initial_only_for_current_generation() {
    with_context(|driver, context| {
        driver.open(context, "saved");
        driver.resize(context, 321, 240);
        driver.send(context, SimulationCommand::ImeCommit(" edited".into()));
        driver.key(context, KeyCode::ArrowLeft, ElementState::Pressed);
        let edited = driver.view(context);
        context.server_resource_packs.request_reload();
        let reload = context.server_resource_packs.take_reload().unwrap();
        let ack = |generation| {
            SimulationCommand::ResourceReloadAck(Box::new(
                crate::resource_reload::ResourceReloadAck {
                    generation,
                    diagnostics: Default::default(),
                    result: Err("fixture resource load failure".into()),
                },
            ))
        };
        driver.send(context, ack(reload.generation + 1));
        assert_eq!(driver.view(context), edited);
        driver.send(context, ack(reload.generation));
        let rebuilt = driver.view(context);
        assert_eq!(String::from_utf16_lossy(&rebuilt.value), "saved");
        assert_eq!(
            (rebuilt.cursor, rebuilt.selection, rebuilt.display_start),
            (5, 5, 0)
        );
    });
}

#[tokio::test]
async fn command_confirmation_return_keeps_hidden_drag_inert_after_mouse_release() {
    use bbb_protocol::packets::{DialogHolder, ShowDialog};
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    with_context(|driver, context| {
        let entry = |name: &str, value| Entry {
            name: name.into(),
            value,
        };
        let dialog = ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: Value::Compound(vec![
                    entry("type", Value::String("minecraft:notice".into())),
                    entry("title", Value::String("Slider".into())),
                    entry(
                        "inputs",
                        Value::List(vec![Value::Compound(vec![
                            entry("type", Value::String("minecraft:number_range".into())),
                            entry("key", Value::String("value".into())),
                            entry("label", Value::String("Value".into())),
                            entry("start", Value::Float(0.0f32.to_bits())),
                            entry("end", Value::Float(1.0f32.to_bits())),
                            entry("initial", Value::Float(0.25f32.to_bits())),
                        ])]),
                    ),
                ]),
            },
        };
        context.world.apply_show_dialog(dialog.clone());
        context.input.apply_screen_requests(
            context.world,
            context.net_counters,
            context.net_commands,
        );
        assert!(context.input.begin_dialog_number_range_drag(0));
        let values = context.input.current_dialog_input_values().to_vec();
        context.input.start_dialog_unattended_command(
            &"/seed".into(),
            "none",
            context.world,
            context.net_counters,
            context.net_commands,
        );
        let original = context.input.command_confirmation().unwrap().clone();
        context.world.apply_show_dialog(dialog);
        context.input.apply_screen_requests(
            context.world,
            context.net_counters,
            context.net_commands,
        );
        driver.mouse(context, ElementState::Released, 100.0, 80.0, None);
        assert!(context.input.close_dialog());
        let restored = context.input.command_confirmation().unwrap();
        assert_eq!(restored.generation, original.generation);
        assert!(restored.presentation > original.presentation);
        driver.key(context, KeyCode::Escape, ElementState::Pressed);
        driver.key(context, KeyCode::Escape, ElementState::Released);
        assert!(context.input.command_confirmation().is_none());
        assert_eq!(context.input.dialog_dragging_number_range_input(), Some(0));
        driver.moved(context, 300.0, 80.0);
        assert_eq!(context.input.current_dialog_input_values(), values);
        assert_eq!(context.input.dialog_dragging_number_range_input(), Some(0));
    });
}

#[tokio::test]
async fn command_confirmation_modal_drains_narrator_toggle_and_initialization_requests() {
    with_context(|driver, context| {
        context.input.set_narrator_available(true);
        context.input.set_narrator_hotkey_enabled(true);
        context
            .input
            .set_narrator_status(crate::narrator_runtime::NarratorStatus::Off);
        context.input.start_unattended_command(
            &"/seed".into(),
            context.world,
            context.net_counters,
            context.net_commands,
        );
        driver.key(context, KeyCode::ControlLeft, ElementState::Pressed);
        driver.key(context, KeyCode::KeyB, ElementState::Pressed);
        assert!(context.input.command_confirmation().is_some());
        assert!(context.input.take_narrator_status_requests().is_empty());
        assert!(context.input.take_debug_client_option_writes().is_empty());
        assert!(context
            .input
            .take_command_confirmation_narrations()
            .is_empty());
        driver.key(context, KeyCode::Escape, ElementState::Pressed);
        assert!(context.input.command_confirmation().is_none());
    });
}

#[tokio::test]
async fn copied_location_does_not_start_the_manual_crash_countdown() {
    with_context(|driver, context| {
        context
            .world
            .apply_login(&bbb_protocol::packets::PlayLogin {
                player_id: 42,
                hardcore: false,
                levels: vec!["minecraft:overworld".to_string()],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 6,
                reduced_debug_info: false,
                show_death_screen: true,
                do_limited_crafting: false,
                common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
                    dimension_type_id: 0,
                    dimension: "minecraft:overworld".to_string(),
                    seed: 0,
                    game_type: 0,
                    previous_game_type: -1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                enforces_secure_chat: false,
            });
        context
            .world
            .apply_game_event(bbb_protocol::packets::GameEvent {
                event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
                param: 0.0,
            });
        assert!(context.world.complete_client_level_load(false));
        context
            .world
            .set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
                position: bbb_protocol::packets::Vec3d {
                    x: 10.25,
                    y: 64.0,
                    z: -5.75,
                },
                y_rot: 90.5,
                x_rot: -15.25,
                ..Default::default()
            });
        driver.runtime.set_cursor_captured(true);
        driver.key(context, KeyCode::F3, ElementState::Pressed);
        driver.key(context, KeyCode::KeyC, ElementState::Pressed);
        assert!(context
            .platform
            .take()
            .iter().any(|effect| matches!(effect, PlatformEffect::ClipboardWrite(text) if text == &bbb_protocol::JavaString::from(
                "/execute in minecraft:overworld run tp @s 10.25 64.00 -5.75 90.50 -15.25"
            ))));
        for seconds in [0, 11] {
            crate::input::advance_player_input_for_client_ticks(
                context.input,
                context.world,
                context.net_counters,
                context.net_commands,
                context.now + Duration::from_secs(seconds),
                0,
            );
        }
        let messages = &context.world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "[Debug]: Copied location to clipboard");
    });
}

#[tokio::test]
async fn chat_preedit_keeps_draft_until_commit_and_tracks_focus_and_close() {
    with_context(|driver, context| {
        driver.open(context, "/msg ");
        let before = driver.view(context);
        let requests = context.net_counters.command_suggestion_commands_queued;
        driver.send(
            context,
            SimulationCommand::ImePreedit {
                text: "nihao".into(),
                cursor_range: Some((2, 4)),
            },
        );
        assert_eq!(driver.view(context), before);
        let overlay = context.input.chat_ime_preedit_hud_state().unwrap();
        assert_eq!(
            (
                overlay.text.as_str(),
                overlay.focused_start,
                overlay.focused_end,
                overlay.caret
            ),
            ("nihao", 2, 4, Some(4))
        );
        assert_eq!(
            context.net_counters.command_suggestion_commands_queued,
            requests
        );
        driver.send(context, SimulationCommand::FocusChanged(false));
        assert!(context.input.chat_ime_preedit_hud_state().is_none());
        driver.send(context, SimulationCommand::FocusChanged(true));
        assert_eq!(
            context.input.chat_ime_preedit_hud_state().unwrap().text,
            "nihao"
        );
        driver.send(context, SimulationCommand::ImeCommit("你好".into()));
        driver.send(
            context,
            SimulationCommand::ImePreedit {
                text: String::new(),
                cursor_range: None,
            },
        );
        assert_eq!(
            String::from_utf16_lossy(&driver.view(context).value),
            "/msg 你好"
        );
        assert!(context.input.chat_ime_preedit_hud_state().is_none());
        driver.send(
            context,
            SimulationCommand::ImePreedit {
                text: "pending".into(),
                cursor_range: Some((7, 7)),
            },
        );
        driver.key(context, KeyCode::Escape, ElementState::Pressed);
        assert!(!context.input.chat_entry_is_active());
        assert!(context.input.chat_ime_preedit_hud_state().is_none());
        driver.send(context, SimulationCommand::ImeDisabled);
        driver.open(context, "");
        assert!(context.input.chat_ime_preedit_hud_state().is_none());
        assert!(!String::from_utf16_lossy(&driver.view(context).value).contains("pending"));
        assert_eq!(context.net_counters.chat_message_commands_queued, 0);
    });
}

#[tokio::test]
async fn ime_preedit_routes_to_all_non_chat_text_input_screens() {
    use bbb_client_world::{
        BlockPos, BookEditScreenState, BookEditorText, BookScreenState, BookViewScreenState,
        JigsawBlockEditorState, StructureBlockEditorState, TestBlockEditorState,
    };
    use bbb_protocol::menu_types::VANILLA_MENU_TYPE_ANVIL_ID;
    use bbb_protocol::packets::{
        ContainerClose, ContainerSetContent, InteractionHand, ItemStackSummary, OpenScreen,
        RecipeBookSettings, RecipeBookTypeSettings, TestBlockMode,
    };
    use bbb_protocol::RichTextComponent;

    with_context(|driver, context| {
        let preedit = |text: &str| SimulationCommand::ImePreedit {
            text: text.into(),
            cursor_range: Some((0, text.len())),
        };

        // 1. Anvil rename (with an item in slot 0 so rename input is editable)
        context.world.apply_open_screen(OpenScreen {
            container_id: 3,
            menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
            title: RichTextComponent::plain("Anvil"),
        });
        let mut items = vec![ItemStackSummary::empty(); 39];
        items[0] = ItemStackSummary {
            count: 1,
            item_id: Some(1),
            ..ItemStackSummary::empty()
        };
        context
            .world
            .apply_container_set_content(ContainerSetContent {
                container_id: 3,
                state_id: 1,
                items,
                carried_item: ItemStackSummary::empty(),
            });
        context.input.apply_screen_requests(
            context.world,
            context.net_counters,
            context.net_commands,
        );
        driver.send(context, preedit("jin"));
        assert!(context.input.text_input_is_active(context.world));
        assert_eq!(
            context
                .input
                .anvil_rename_field()
                .and_then(|field| field.ime_preedit_hud_state())
                .map(|state| state.text),
            Some("jin".to_string())
        );
        driver.send(context, SimulationCommand::ImeCommit("金".into()));
        assert!(context.input.anvil_rename_text().contains('金'));
        assert!(context
            .input
            .anvil_rename_field()
            .and_then(|field| field.ime_preedit_hud_state())
            .is_none());
        context
            .world
            .apply_container_close(ContainerClose { container_id: 3 });
        context.input.apply_screen_requests(
            context.world,
            context.net_counters,
            context.net_commands,
        );

        // 2. Book & Quill editor vs read-only View book
        context
            .input
            .set_screen_for_test(Screen::Book(crate::input::BookScreen::new(
                BookScreenState::Edit(BookEditScreenState {
                    generation: 1,
                    hand: InteractionHand::MainHand,
                    slot: 0,
                    pages: vec![BookEditorText::default()],
                    current_page: 0,
                    sign_title: BookEditorText::default(),
                }),
            )));
        driver.send(context, preedit("shu"));
        assert!(context.input.text_input_is_active(context.world));
        assert_eq!(
            context
                .input
                .book_ime_preedit_hud_state()
                .map(|state| state.text),
            Some("shu".to_string())
        );
        driver.send(context, SimulationCommand::ImeCommit("书".into()));
        assert!(context.input.book_ime_preedit_hud_state().is_none());
        // Read-only book view does not accept IME preedit or text input.
        context
            .input
            .set_screen_for_test(Screen::Book(crate::input::BookScreen::new(
                BookScreenState::View(BookViewScreenState {
                    generation: 2,
                    hand: InteractionHand::MainHand,
                    pages: Vec::new(),
                    current_page: 0,
                }),
            )));
        driver.send(context, preedit("ignored"));
        assert!(!context.input.text_input_is_active(context.world));
        assert!(context.input.book_ime_preedit_hud_state().is_none());

        // 3. Structure Block editor
        context.input.open_structure_block_editor(
            context.world,
            context.net_counters,
            context.net_commands,
            StructureBlockEditorState {
                pos: BlockPos { x: 1, y: 2, z: 3 },
                data: Default::default(),
            },
        );
        driver.send(context, preedit("jie"));
        assert!(context.input.text_input_is_active(context.world));
        let structure_hud = context.input.structure_block_editor_hud_state().unwrap();
        assert_eq!(
            structure_hud.fields[structure_hud.focus_index]
                .ime_preedit_hud_state()
                .map(|state| state.text),
            Some("jie".to_string())
        );

        // 4. Jigsaw Block editor
        context.input.open_jigsaw_block_editor(
            context.world,
            context.net_counters,
            context.net_commands,
            JigsawBlockEditorState {
                pos: BlockPos { x: 4, y: 5, z: 6 },
                data: Default::default(),
                joint_editable: true,
            },
        );
        driver.send(context, preedit("pin"));
        let jigsaw_hud = context.input.jigsaw_block_editor_hud_state().unwrap();
        assert_eq!(
            jigsaw_hud.fields[jigsaw_hud.focus_index]
                .ime_preedit_hud_state()
                .map(|state| state.text),
            Some("pin".to_string())
        );

        // 5. Test Block editor (Log mode editable vs Start mode non-editable)
        context.input.open_test_block_editor(
            context.world,
            context.net_counters,
            context.net_commands,
            TestBlockEditorState {
                pos: BlockPos { x: 7, y: 8, z: 9 },
                data: bbb_client_world::TestBlockEntityDataState {
                    mode: TestBlockMode::Log,
                    message: Default::default(),
                    powered: false,
                },
            },
        );
        driver.send(context, preedit("ce"));
        assert!(context.input.text_input_is_active(context.world));
        assert_eq!(
            context
                .input
                .test_block_editor_hud_state()
                .unwrap()
                .message
                .ime_preedit_hud_state()
                .map(|state| state.text),
            Some("ce".to_string())
        );
        context.input.open_test_block_editor(
            context.world,
            context.net_counters,
            context.net_commands,
            TestBlockEditorState {
                pos: BlockPos { x: 7, y: 8, z: 9 },
                data: bbb_client_world::TestBlockEntityDataState {
                    mode: TestBlockMode::Start,
                    message: Default::default(),
                    powered: false,
                },
            },
        );
        driver.send(context, preedit("no"));
        assert!(!context.input.text_input_is_active(context.world));
        assert!(context
            .input
            .test_block_editor_hud_state()
            .unwrap()
            .message
            .ime_preedit_hud_state()
            .is_none());

        // 6. Social Interactions search
        context.input.open_social_interactions_screen(
            context.world,
            context.net_counters,
            context.net_commands,
        );
        driver.send(context, preedit("peng"));
        assert!(context.input.text_input_is_active(context.world));
        assert_eq!(
            context
                .input
                .social_interactions_runtime()
                .unwrap()
                .search()
                .ime_preedit_hud_state()
                .map(|state| state.text),
            Some("peng".to_string())
        );

        // 7. Recipe Book search (local inventory with crafting book open + focused)
        context.world.open_local_inventory();
        context.input.apply_screen_requests(
            context.world,
            context.net_counters,
            context.net_commands,
        );
        context
            .world
            .apply_recipe_book_settings(RecipeBookSettings {
                crafting: RecipeBookTypeSettings {
                    open: true,
                    filtering: false,
                },
                furnace: RecipeBookTypeSettings::default(),
                blast_furnace: RecipeBookTypeSettings::default(),
                smoker: RecipeBookTypeSettings::default(),
            });
        let mut container = crate::input::ContainerInputState::default();
        container.recipe_book_search_focused = true;
        context
            .input
            .set_screen_for_test(Screen::Container(container));
        driver.send(context, preedit("pei"));
        assert!(context.input.text_input_is_active(context.world));
        assert_eq!(
            context
                .input
                .recipe_book_search_hud_state()
                .edit
                .ime_preedit_hud_state()
                .map(|state| state.text),
            Some("pei".to_string())
        );
    });
}

#[tokio::test]
async fn focus_capture_look_at_and_timed_holds_advance_and_complete_canonically() {
    with_context(|driver, context| {
        context
            .world
            .set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
                position: bbb_protocol::packets::Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                ..bbb_client_world::LocalPlayerPoseState::default()
            });

        // Without cursor capture, input.look fails explicitly via completion.
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        driver.send(
            context,
            SimulationCommand::WithCompletion {
                command: Box::new(SimulationCommand::Control(ControlWriteRequest::Input(
                    InputControlRequest::Look {
                        yaw: 45.0,
                        pitch: 10.0,
                    },
                ))),
                completion: tx,
            },
        );
        assert!(matches!(rx.try_recv(), Ok(Err(_))));

        // Focus + capture enables synthetic cursor capture and allows input.look_at.
        driver.controls(
            context,
            [
                InputControlRequest::FocusCapture {
                    focused: true,
                    capture_cursor: true,
                },
                InputControlRequest::LookAt {
                    target: crate::control::LookAtTargetControl::Position {
                        x: 0.5,
                        y: 64.0 + 1.62,
                        z: 4.5,
                    },
                },
            ],
        );
        assert!(driver.runtime.cursor_captured());
        assert!(driver.runtime.synthetic_cursor_capture());
        let pose = context.world.local_player_pose().unwrap();
        assert!(
            pose.y_rot.abs() < 0.01,
            "expected +Z yaw 0, got {}",
            pose.y_rot
        );
        assert!(
            pose.x_rot.abs() < 0.01,
            "expected level pitch 0, got {}",
            pose.x_rot
        );

        // Timed key hold presses immediately, stays held for 2 ticks, then auto-releases and completes.
        let (hold_tx, mut hold_rx) = tokio::sync::oneshot::channel();
        driver.send(
            context,
            SimulationCommand::WithCompletion {
                command: Box::new(SimulationCommand::Control(ControlWriteRequest::Input(
                    InputControlRequest::KeyHold {
                        key: InputKeyControl::Forward,
                        hold_ticks: 3,
                    },
                ))),
                completion: hold_tx,
            },
        );
        assert!(context.input.local_player_input(context.world).forward);
        assert!(!driver.runtime.has_pending_control_completions());
        driver.runtime.advance_scheduled_control_timers(2, context);
        assert!(context.input.local_player_input(context.world).forward);
        assert!(!driver.runtime.has_pending_control_completions());
        driver.runtime.advance_scheduled_control_timers(1, context);
        assert!(!context.input.local_player_input(context.world).forward);
        assert!(driver.runtime.has_pending_control_completions());
        driver.runtime.complete_control_requests(99);
        assert!(matches!(
            hold_rx.try_recv(),
            Ok(Ok(ControlCompletion::Processed { generation: 99 }))
        ));

        // WaitTicks waits until exact tick count then completes.
        let (wait_tx, mut wait_rx) = tokio::sync::oneshot::channel();
        driver.send(
            context,
            SimulationCommand::WithCompletion {
                command: Box::new(SimulationCommand::Control(ControlWriteRequest::WaitTicks {
                    ticks: 2,
                })),
                completion: wait_tx,
            },
        );
        driver.runtime.advance_scheduled_control_timers(1, context);
        assert!(!driver.runtime.has_pending_control_completions());
        driver.runtime.advance_scheduled_control_timers(1, context);
        driver.runtime.complete_control_requests(100);
        assert!(matches!(
            wait_rx.try_recv(),
            Ok(Ok(ControlCompletion::Processed { generation: 100 }))
        ));
    });
}
