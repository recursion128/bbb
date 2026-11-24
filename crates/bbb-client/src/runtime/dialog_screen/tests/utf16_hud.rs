use super::*;
use crate::input::dialog::DialogInputValueState;

#[test]
fn raw_dialog_hud_ime_empty_and_clipped_insert_use_widget_cursor_offset() {
    let mut input = crate::input::ClientInputState::new(true);
    let surface = PhysicalSize::new(320, 240);
    for (text, width, max_length, offset) in [("", 100, 32, 0), ("A", 1, 1, -1)] {
        let world = world_with_notice(
            &mut input,
            vec![entry(
                "inputs",
                NbtSummaryValue::List(vec![text_input(
                    "text", "Text", width, false, text, max_length,
                )]),
            )],
        );
        assert!(input.focus_dialog_text_input(0, 0, false));
        let preedit = DialogImePreeditHudState {
            text: "X".into(),
            caret: Some(1),
            focused_start: 0,
            focused_end: 1,
            screen_revision: input.dialog_screen_revision(),
            caret_visible: false,
        };
        let local = DialogHudLocalState {
            dialog_ime_preedit: Some(preedit),
            ..Default::default()
        };
        let screen = hud_dialog_screen(
            &input,
            &world,
            input.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &local,
            surface,
            0.0,
        )
        .unwrap();
        let label = screen
            .scroll_area
            .as_ref()
            .unwrap()
            .text_labels
            .iter()
            .find(|label| label.input.is_some())
            .unwrap();
        assert_eq!(screen.preedit_overlay.unwrap().text_x, label.x + offset);
    }
}

#[test]
fn raw_dialog_hud_producer_retains_units_and_ime_pair_prefix() {
    let mut input = crate::input::ClientInputState::new(true);
    let surface = PhysicalSize::new(320, 240);
    for multiline in [false, true] {
        let raw = bbb_protocol::JavaString::from(vec![65, 0xd83d, 0xde00, 66]);
        let definition = if multiline {
            multiline_text_input("text", "Text", 100, false, raw.clone(), 10000, None, None)
        } else {
            text_input("text", "Text", 100, false, raw.clone(), 10000)
        };
        let world = world_with_notice(
            &mut input,
            vec![entry("inputs", NbtSummaryValue::List(vec![definition]))],
        );
        assert!(input.focus_dialog_text_input(0, 2, false));
        assert!(input.set_dialog_text_input_raw_value_and_selection(0, raw.clone(), 2, 3));
        let preedit = DialogImePreeditHudState {
            text: "X".into(),
            caret: Some(1),
            focused_start: 0,
            focused_end: 1,
            screen_revision: input.dialog_screen_revision(),
            caret_visible: false,
        };
        let local = DialogHudLocalState {
            dialog_ime_preedit: Some(preedit.clone()),
            ..Default::default()
        };
        let screen = hud_dialog_screen(
            &input,
            &world,
            input.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &local,
            surface,
            0.0,
        )
        .unwrap();
        let controls = dialog_controls(
            input.current_dialog().unwrap().data.as_ref().unwrap(),
            world.server_links(),
        )
        .unwrap();
        let layout = dialog_input_layouts(&controls, surface).unwrap()[0];
        let text_x = if multiline {
            let area = screen
                .scroll_area
                .as_ref()
                .unwrap()
                .text_areas
                .first()
                .unwrap();
            assert_eq!(area.text, "A😀B");
            assert_eq!(area.max_length, 10000);
            assert_eq!(
                (area.cursor, area.selection, area.needs_cursor_position),
                (2, 3, true)
            );
            dialog_multiline_text_box_layout(layout, false).x + TEXT_INPUT_INSET
        } else {
            let label = screen
                .scroll_area
                .as_ref()
                .unwrap()
                .text_labels
                .iter()
                .find(|label| label.input.is_some())
                .unwrap();
            assert_eq!(label.text, "A😀B");
            let input = label.input.unwrap();
            assert_eq!(input.max_length, 10000);
            assert_eq!((input.cursor, input.selection, input.scroll_to), (2, 3, 3));
            dialog_single_line_text_box_layout(layout, false).x + TEXT_INPUT_INSET
        };
        assert_eq!(
            screen.preedit_overlay.unwrap().text_x,
            text_x + ApproximateDialogTextMeasurer.width("A�")
        );
        assert!(dialog_ime_cursor_area(
            &input,
            &world,
            Some(&preedit),
            surface,
            &ApproximateDialogTextMeasurer
        )
        .is_some());
        if multiline {
            let mut stale = local;
            stale.dialog_ime_preedit.as_mut().unwrap().screen_revision += 1;
            let screen = hud_dialog_screen(
                &input,
                &world,
                input.current_dialog().unwrap(),
                None,
                &TerrainTextureState::default(),
                &stale,
                surface,
                0.0,
            )
            .unwrap();
            assert!(!screen.scroll_area.as_ref().unwrap().text_areas[0].needs_cursor_position);
            assert!(screen.preedit_overlay.is_none());
        }
        assert_eq!(
            input.current_dialog_input_values(),
            [Some(DialogInputValueState::Text { value: raw.clone() })]
        );
        assert_eq!(
            bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
            raw.to_nbt()
        );
    }
}

#[test]
fn raw_dialog_hud_producer_keeps_long_controls_and_isolated_units() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut units = vec![0xd800, 0, 9, 65, 0xdc00];
    units.extend(std::iter::repeat_n(66, 300));
    let raw = bbb_protocol::JavaString::from(units);
    for multiline in [false, true] {
        let definition = if multiline {
            multiline_text_input("text", "Text", 100, false, raw.clone(), 10000, None, None)
        } else {
            text_input("text", "Text", 100, false, raw.clone(), 10000)
        };
        let world = world_with_notice(
            &mut input,
            vec![entry("inputs", NbtSummaryValue::List(vec![definition]))],
        );
        assert!(input.focus_dialog_text_input(0, 5, false));
        let screen = hud_dialog_screen(
            &input,
            &world,
            input.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &DialogHudLocalState::default(),
            PhysicalSize::new(320, 240),
            0.0,
        )
        .unwrap();
        let display = if multiline {
            &screen.scroll_area.as_ref().unwrap().text_areas[0].text
        } else {
            &screen
                .scroll_area
                .as_ref()
                .unwrap()
                .text_labels
                .iter()
                .find(|label| label.input.is_some())
                .unwrap()
                .text
        };
        assert_eq!(display, &raw.to_display_string());
        assert_eq!(display.encode_utf16().count(), raw.units().len());
        assert_eq!(
            bound_dialog_input_values(&input, &world).unwrap()["text"].tag,
            raw.to_nbt()
        );
    }
}
