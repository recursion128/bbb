use super::text_field::{TextField, TextFieldLayout};
use super::*;

impl ClientInputState {
    #[cfg(test)]
    pub(crate) fn chat_input_initial_for_test(&self) -> Option<&[u16]> {
        let CommandEntryKind::Chat(screen) = &self.screen.chat_entry()?.owner else {
            return None;
        };
        Some(&screen.initial)
    }

    pub(crate) fn chat_input_screen_revision(&self) -> Option<u64> {
        let CommandEntryKind::Chat(screen) = &self.screen.chat_entry()?.owner else {
            return None;
        };
        Some(screen.revision)
    }

    pub(crate) fn note_chat_input_click_at(
        &mut self,
        revision: u64,
        button: MouseButton,
        now: Instant,
    ) {
        if let Some((_, screen)) = self
            .chat_mouse_screens_mut()
            .find(|(_, screen)| screen.revision == revision)
        {
            screen.last_click = Some((button, now));
        }
    }

    pub(crate) fn save_chat_input_initial(&mut self, revision: u64) {
        if let Some((edit, screen)) = self
            .chat_mouse_screens_mut()
            .find(|(_, screen)| screen.revision == revision)
        {
            screen.initial = edit.value().to_vec();
        }
    }

    pub(crate) fn handle_chat_input_mouse_button_at(
        &mut self,
        button: MouseButton,
        state: ElementState,
        position: Option<PhysicalPosition<f64>>,
        surface: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
        now: Instant,
    ) -> bool {
        let extend = self.shift_down();
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        let CommandEntryKind::Chat(screen) = &mut entry.owner else {
            return false;
        };
        if button != MouseButton::Left {
            return false;
        }
        if state == ElementState::Released {
            return std::mem::take(&mut screen.dragging);
        }
        let Some((x, y)) = position.and_then(physical_position_floor_i32) else {
            return false;
        };
        let width = i32::try_from(surface.width).unwrap_or(i32::MAX);
        let height = i32::try_from(surface.height).unwrap_or(i32::MAX);
        if x < 4 || x >= width || y < height.saturating_sub(12) || y >= height {
            return false;
        }
        let layout = chat_mouse_layout(surface, measurer);
        let clicked = chat_mouse_caret(&entry.edit, x, &layout);
        let double_click = screen
            .last_click
            .is_some_and(|(previous_button, previous_time)| {
                previous_button == button
                    && now
                        .checked_duration_since(previous_time)
                        .is_some_and(|elapsed| elapsed < Duration::from_millis(250))
            });
        // Vanilla EditBox computes both word bounds before either scroll.
        if double_click {
            let start = entry.edit.word_position(clicked, -1);
            let end = entry.edit.word_position(clicked, 1);
            entry.edit.move_cursor(start, false, Some(&layout));
            entry.edit.move_cursor(end, true, Some(&layout));
        } else {
            entry.edit.move_cursor(clicked, extend, Some(&layout));
        }
        screen.dragging = true;
        let revision = screen.revision;
        self.note_chat_input_click_at(revision, button, now);
        true
    }

    pub(crate) fn handle_chat_input_cursor_moved(
        &mut self,
        position: Option<PhysicalPosition<f64>>,
        surface: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let Some(entry) = self.screen.chat_entry_mut() else {
            return false;
        };
        let CommandEntryKind::Chat(screen) = &mut entry.owner else {
            return false;
        };
        if !screen.dragging {
            return false;
        }
        let Some((x, _)) = position.and_then(physical_position_floor_i32) else {
            return false;
        };
        let layout = chat_mouse_layout(surface, measurer);
        let clicked = chat_mouse_caret(&entry.edit, x, &layout);
        entry.edit.move_cursor(clicked, true, Some(&layout));
        true
    }

    pub(crate) fn cancel_chat_input_drag(&mut self) {
        for (_, screen) in self.chat_mouse_screens_mut() {
            screen.dragging = false;
        }
    }

    fn chat_mouse_screens_mut(
        &mut self,
    ) -> impl Iterator<Item = (&mut TextField, &mut ChatScreenEntryState)> {
        self.screen
            .iter_mut()
            .filter_map(|screen| match screen {
                Screen::CommandEntry(entry) => Some(entry),
                _ => None,
            })
            .filter_map(|entry| match &mut entry.owner {
                CommandEntryKind::Chat(screen) => Some((&mut entry.edit, screen)),
                _ => None,
            })
    }
}

fn chat_mouse_layout(
    surface: PhysicalSize<u32>,
    measurer: &impl CommandSuggestionTextMeasurer,
) -> TextFieldLayout<'_> {
    TextFieldLayout {
        glyphs: measurer.chat_input_glyphs(),
        inner_width: i32::try_from(surface.width)
            .unwrap_or(i32::MAX)
            .saturating_sub(4),
    }
}

fn chat_mouse_caret(edit: &TextField, x: i32, layout: &TextFieldLayout<'_>) -> usize {
    let display = edit.display_start();
    // Vanilla EditBox retains negative widths and raw UTF-16 display offsets.
    let width = x.saturating_sub(4).min(layout.inner_width);
    display
        + layout
            .glyphs
            .plain_utf16_head_len(&edit.value()[display..], width)
            .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{HudAsciiGlyph, HudFontGlyphMap};

    struct Measurer(HudFontGlyphMap);

    impl CommandSuggestionTextMeasurer for Measurer {
        fn command_suggestion_glyphs(&self) -> &HudFontGlyphMap {
            &self.0
        }

        fn chat_input_glyphs(&self) -> &HudFontGlyphMap {
            self.0.non_fishy()
        }
    }

    fn measurer() -> Measurer {
        let mut map = HudFontGlyphMap::new();
        map.set_missing_glyph(HudAsciiGlyph::empty(6.0));
        for (cp, any, filtered) in [('😀', 12.0, 12.0), ('N', -10.0, 1.25)] {
            map.insert_selected_codepoint_first_wins(
                cp as u32,
                HudAsciiGlyph::empty(any),
                HudAsciiGlyph::empty(filtered),
                any,
            );
        }
        Measurer(map)
    }

    fn fixture(text: &str, width: u32) -> (ClientInputState, Measurer, PhysicalSize<u32>) {
        let measurer = measurer();
        let surface = PhysicalSize::new(width, 100);
        let mut input = ClientInputState::new(true);
        input.set_chat_input_surface_size(surface);
        let mut entry = ChatEntryState::default();
        let CommandEntryKind::Chat(screen) = &mut entry.owner else {
            unreachable!()
        };
        screen.revision = 7;
        entry.edit.set_value(
            &text.encode_utf16().collect::<Vec<_>>(),
            Some(&chat_mouse_layout(surface, &measurer)),
        );
        input.set_screen_for_test(Screen::CommandEntry(entry));
        (input, measurer, surface)
    }

    fn edit(input: &ClientInputState) -> &TextField {
        &input.screen.chat_entry().unwrap().edit
    }

    fn screen(input: &ClientInputState) -> &ChatScreenEntryState {
        let CommandEntryKind::Chat(screen) = &input.screen.chat_entry().unwrap().owner else {
            panic!("expected Chat")
        };
        screen
    }

    fn press(
        input: &mut ClientInputState,
        measurer: &Measurer,
        surface: PhysicalSize<u32>,
        x: f64,
        now: Instant,
    ) -> bool {
        input.handle_chat_input_mouse_button_at(
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(x, 90.0)),
            surface,
            measurer,
            now,
        )
    }

    #[test]
    fn chat_mouse_caret_keeps_raw_surrogate_and_scrolled_viewport() {
        let (mut input, measurer, surface) = fixture("😀AB", 22);
        assert_eq!(edit(&input).display_start(), 1);
        assert!(press(&mut input, &measurer, surface, 4.0, Instant::now()));
        assert_eq!((edit(&input).cursor(), edit(&input).selection()), (1, 1));
        let (mut input, measurer, surface) = fixture("AAAAAA", 22);
        assert_eq!(edit(&input).display_start(), 3);
        assert!(press(&mut input, &measurer, surface, 10.0, Instant::now()));
        assert_eq!(edit(&input).cursor(), 4);
    }

    #[test]
    fn chat_mouse_fractional_filtered_width_uses_floor_before_head_measurement() {
        for (x, expected) in [(5.99, 0), (6.99, 1), (7.0, 2)] {
            let (mut input, measurer, surface) = fixture("NNN", 100);
            assert!(press(&mut input, &measurer, surface, x, Instant::now()));
            assert_eq!(edit(&input).cursor(), expected);
        }
    }

    #[test]
    fn chat_mouse_shift_and_press_move_release_move_keep_selection_anchor() {
        let (mut input, measurer, surface) = fixture("AAAA", 100);
        input.shift_left_down = true;
        assert!(press(&mut input, &measurer, surface, 10.0, Instant::now()));
        assert_eq!((edit(&input).cursor(), edit(&input).selection()), (1, 4));
        assert!(input.handle_chat_input_cursor_moved(
            Some(PhysicalPosition::new(-25.5, -100.0)),
            surface,
            &measurer
        ));
        assert_eq!((edit(&input).cursor(), edit(&input).selection()), (0, 4));
        assert!(!input.handle_chat_input_cursor_moved(None, surface, &measurer));
        assert!(input.handle_chat_input_mouse_button_at(
            MouseButton::Left,
            ElementState::Released,
            None,
            surface,
            &measurer,
            Instant::now()
        ));
        assert!(!input.handle_chat_input_cursor_moved(
            Some(PhysicalPosition::new(90.0, 90.0)),
            surface,
            &measurer
        ));
        assert_eq!(edit(&input).cursor(), 0);
        assert!(!input.handle_chat_input_mouse_button_at(
            MouseButton::Left,
            ElementState::Released,
            None,
            surface,
            &measurer,
            Instant::now()
        ));
    }

    #[test]
    fn chat_mouse_double_word_requires_strict_time_button_and_revision() {
        let now = Instant::now();
        for (delay, button, revision, expected) in [
            (249, MouseButton::Left, 7, (6, 3)),
            (250, MouseButton::Left, 7, (4, 4)),
            (249, MouseButton::Right, 7, (4, 4)),
            (249, MouseButton::Left, 8, (4, 4)),
        ] {
            let (mut input, measurer, surface) = fixture("aa bb cc", 100);
            input.note_chat_input_click_at(revision, button, now);
            assert!(press(
                &mut input,
                &measurer,
                surface,
                28.0,
                now + Duration::from_millis(delay)
            ));
            assert_eq!((edit(&input).cursor(), edit(&input).selection()), expected);
            assert_eq!(
                screen(&input).last_click,
                Some((MouseButton::Left, now + Duration::from_millis(delay)))
            );
        }
    }

    #[test]
    fn chat_mouse_own_click_records_double_word_and_cancel_stops_drag() {
        let (mut input, measurer, surface) = fixture("aa bb cc", 100);
        let now = Instant::now();
        assert!(press(&mut input, &measurer, surface, 28.0, now));
        assert!(press(
            &mut input,
            &measurer,
            surface,
            28.0,
            now + Duration::from_millis(20)
        ));
        assert_eq!((edit(&input).cursor(), edit(&input).selection()), (6, 3));
        input.cancel_chat_input_drag();
        assert!(!screen(&input).dragging);
        assert!(!input.mouse_left_down);
        assert!(!input.handle_chat_input_cursor_moved(
            Some(PhysicalPosition::new(4.0, 90.0)),
            surface,
            &measurer
        ));
    }

    #[test]
    fn chat_mouse_records_original_screen_after_restriction_or_dialog() {
        let now = Instant::now();
        for dialog in [false, true] {
            let (mut input, measurer, surface) = fixture("text", 100);
            let revision = input.chat_input_screen_revision().unwrap();
            assert!(press(&mut input, &measurer, surface, 4.0, now));
            if dialog {
                let mut world = bbb_client_world::WorldStore::new();
                world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
                    dialog: bbb_protocol::packets::DialogHolder::Reference { registry_id: 0 },
                });
                input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
            } else {
                assert!(input.open_chat_restrictions_screen());
            }
            assert_eq!(input.chat_input_screen_revision(), None);
            assert!(!press(&mut input, &measurer, surface, 10.0, now));
            input.note_chat_input_click_at(revision, MouseButton::Right, now);
            input.cancel_chat_input_drag();
            let previous = input.screen.parent();
            let Some(Screen::CommandEntry(previous)) = previous else {
                unreachable!()
            };
            let CommandEntryKind::Chat(screen) = &previous.owner else {
                unreachable!()
            };
            assert_eq!(screen.last_click, Some((MouseButton::Right, now)));
            assert!(!screen.dragging);
        }
    }

    #[test]
    fn chat_mouse_hitbox_is_half_open_and_rejects_unavailable_positions() {
        let (mut input, measurer, surface) = fixture("text", 100);
        for position in [
            None,
            Some(PhysicalPosition::new(f64::NAN, 90.0)),
            Some(PhysicalPosition::new(3.99, 90.0)),
            Some(PhysicalPosition::new(100.0, 90.0)),
            Some(PhysicalPosition::new(4.0, 87.99)),
            Some(PhysicalPosition::new(4.0, 100.0)),
        ] {
            assert!(!input.handle_chat_input_mouse_button_at(
                MouseButton::Left,
                ElementState::Pressed,
                position,
                surface,
                &measurer,
                Instant::now()
            ));
        }
        assert!(input.handle_chat_input_mouse_button_at(
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(4.0, 88.0)),
            surface,
            &measurer,
            Instant::now()
        ));
        assert!(!input.handle_chat_input_mouse_button_at(
            MouseButton::Right,
            ElementState::Released,
            None,
            surface,
            &measurer,
            Instant::now()
        ));
        assert!(screen(&input).dragging);
    }

    #[test]
    fn chat_mouse_isolates_other_text_editors() {
        let (mut input, measurer, surface) = fixture("command", 100);
        input.screen.chat_entry_mut().unwrap().owner =
            CommandEntryKind::CommandBlock(CommandBlockEditorInputState {
                pos: BlockPos { x: 0, y: 0, z: 0 },
                mode: CommandBlockMode::Redstone,
                conditional: false,
                automatic: false,
                track_output: false,
                previous_output: String::new(),
                controls_enabled: true,
            });
        let previous = input.screen.chat_entry().cloned();
        assert_eq!(input.chat_input_screen_revision(), None);
        input.note_chat_input_click_at(7, MouseButton::Left, Instant::now());
        assert!(!press(&mut input, &measurer, surface, 4.0, Instant::now()));
        assert!(!input.handle_chat_input_cursor_moved(
            Some(PhysicalPosition::new(10.0, 90.0)),
            surface,
            &measurer
        ));
        input.cancel_chat_input_drag();
        assert_eq!(input.screen.chat_entry().cloned(), previous);
    }
}
