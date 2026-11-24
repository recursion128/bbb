use super::*;

fn styled_run(text: &str, style: ComponentStyle) -> StyledTextRun {
    StyledTextRun {
        text: text.to_string(),
        style,
    }
}

fn copied_event(value: &str) -> ComponentClickEvent {
    ComponentClickEvent::CopyToClipboard {
        value: value.to_string().into(),
    }
}

fn message_with_component(
    text: &str,
    component: NbtSummaryValue,
    width: i32,
) -> DialogPlainMessageState {
    DialogPlainMessageState {
        contents: DialogTextState {
            text: text.to_string(),
            runs: Vec::new(),
            component,
        },
        width,
    }
}

fn line_texts(block: &DialogTextBlock) -> Vec<&str> {
    block.lines.iter().map(|line| line.text.as_str()).collect()
}

#[test]
fn dialog_body_consumes_codes_before_lines_runs_and_click_spans() {
    let event = copied_event("copied");
    let message = message_with_component(
        "§aAB§rCD§?E§",
        NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("§aAB§rCD§?E§")),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("copy_to_clipboard")),
                    entry("value", NbtSummaryValue::string("copied")),
                ]),
            ),
        ]),
        20,
    );
    let block = dialog_text_block(&message).unwrap();

    // The unknown code and the trailing `§` never reach a line, and neither
    // consumes a glyph in the break decisions.
    assert_eq!(line_texts(&block), ["AB", "CD", "E"]);
    // `§a` colours both glyphs; `§r` returns to the part's inherited style.
    assert_eq!(block.lines[0].runs[0].color, Some(0x55_ff_55));
    assert_eq!(block.lines[1].runs[0].color, None);
    assert_eq!(
        block.lines[..2]
            .iter()
            .map(|line| line
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>())
            .collect::<Vec<_>>(),
        ["AB", "CD"]
    );
    // Click spans cover the drawn glyph advances of the resolved line.
    let span = |x: usize, width: usize| DialogTextClickSpan {
        x: x as u32,
        width: width as u32,
        event: event.clone(),
    };
    assert_eq!(block.lines[0].click_spans, [span(0, 12)]);
    assert_eq!(block.lines[1].click_spans, [span(0, 12)]);
    assert_eq!(block.lines[2].click_spans, [span(0, 6)]);

    // The hit index reads the same three drawn lines: the centred line starts
    // at `width / 2 - line_width / 2` and the glyphs end at their own advance.
    let rectangle = DialogFocusRectangle {
        x: 0,
        y: 0,
        width: block.width as i32,
        height: block.height,
    };
    let hit = |x: f64, y: f64| {
        dialog_text_click_event_at(
            &message,
            rectangle,
            Some(PhysicalPosition::new(x, y)),
            PhysicalSize::new(320, 240),
        )
    };
    assert_eq!(hit(5.0, 4.0), Some(event.clone()));
    assert_eq!(hit(5.0, 13.0), Some(event.clone()));
    assert_eq!(hit(7.0, 22.0), Some(event.clone()));
    assert_eq!(hit(21.0, 4.0), None);
    assert_eq!(hit(5.0, 40.0), None);
}

#[test]
fn dialog_body_reset_after_a_colour_returns_to_the_part_style() {
    let block = dialog_text_block(&message_with_component(
        "§cX§rY",
        NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("§cX§rY")),
            entry("bold", NbtSummaryValue::Byte(1)),
            entry("color", NbtSummaryValue::string("#ffff55")),
            entry(
                "shadow_color",
                NbtSummaryValue::Int(0x80_12_34_56u32 as i32),
            ),
        ]),
        32,
    ))
    .unwrap();

    assert_eq!(line_texts(&block), ["XY"]);
    // Style: a colour clears the five decorations and keeps the
    // font, events and shadow; `§r` then restores the inherited part style.
    assert_eq!(
        block.lines[0].runs[0],
        HudStyledTextRun {
            shadow_color: Some(0x80_12_34_56),
            text: "X".to_string(),
            style: HudTextStyle::default(),
            color: Some(0xff_55_55),
        }
    );
    assert_eq!(
        block.lines[0].runs[1],
        HudStyledTextRun {
            shadow_color: Some(0x80_12_34_56),
            text: "Y".to_string(),
            style: HudTextStyle {
                bold: true,
                ..HudTextStyle::default()
            },
            color: Some(0xff_ff_55),
        }
    );
}

#[test]
fn dialog_widget_labels_consume_codes_before_width_runs_and_centering() {
    let style = ComponentStyle {
        bold: Some(true),
        ..ComponentStyle::default()
    };
    let text = DialogTextState {
        text: "§cAB§rCD§?E§".to_string(),
        runs: vec![styled_run("§cAB§rCD§?E§", style)],
        component: NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("§cAB§rCD§?E§")),
            entry("bold", NbtSummaryValue::Byte(1)),
        ]),
    };

    // `StringWidget.visitLines` measures with `font.width(Component)` and draws
    // `getVisualOrderText()`, so the label width and
    // the drawn glyphs come from this one list.
    let runs = dialog_text_runs(&text);
    // The unknown code and the trailing `§` are consumed; `CD` and `E` merge
    // because `§r` returned them to the same inherited style.
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["AB", "CDE"]
    );
    assert_eq!(runs[0].color, Some(0xff_55_55));
    assert!(!runs[0].style.bold, "a colour clears the five decorations");
    // `§r` returns to the part's inherited bold and colour, not to the default.
    assert_eq!(runs[1].color, None);
    assert!(runs[1].style.bold);

    let mut labels = Vec::new();
    push_centered_text_label(
        &mut labels,
        &text,
        DialogTextContext::approximate(None),
        100,
        7,
    );
    let label = &labels[0];
    assert_eq!(label.text, "ABCDE");
    assert_eq!(label.runs, runs);
    // The label box is the five drawn characters (6px plain, 7px bold for the
    // `§r`-restored part style); the four code units contribute nothing, and
    // the box is centred on that resolved width.
    let width = runs_approx_width(&label.runs);
    assert_eq!(width, 33);
    assert_eq!(label.width, 33);
    assert_eq!(label.x, 100 - 16);

    let mut plain = Vec::new();
    push_text_label(
        &mut plain,
        &text,
        DialogTextContext::approximate(None),
        3,
        11,
    );
    assert_eq!(plain[0].runs, label.runs);
    assert_eq!(plain[0].text, "ABCDE");
    assert_eq!(plain[0].width, 33);
    assert_eq!((plain[0].x, plain[0].y), (3, 11));
}
