use super::*;
use bbb_client_render::{
    hud_component_text_width, hud_logical_component_text_width_f32,
    hud_wrap_formatted_component_text,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdvancementTextLine {
    pub(super) text: String,
    pub(super) runs: Vec<HudComponentTextRun>,
    width: i32,
}

pub(super) fn advancement_wrap_text_lines(
    text: &RichTextComponent,
    max_width: u32,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<AdvancementTextLine> {
    let runs = localized_component_runs(text, &ComponentStyle::default(), item_runtime)
        .into_iter()
        .map(component_hover::hud_component_run)
        .collect::<Vec<_>>();
    hud_wrap_formatted_component_text(&runs, max_width, fonts)
        .into_iter()
        .map(|runs| {
            let width = hud_component_text_width(&runs, fonts);
            let text = runs.iter().map(|run| run.visual.text.as_str()).collect();
            AdvancementTextLine { text, runs, width }
        })
        .collect()
}

// Vanilla AdvancementWidget: try margins in order, keep the first tie.
fn advancement_description_lines(
    text: &RichTextComponent,
    preferred_width: u32,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<AdvancementTextLine> {
    let mut best = Vec::new();
    let mut best_distance = f32::MAX;
    for margin in [0, 10, -10, 25, -25] {
        let width = (i64::from(preferred_width) - margin).clamp(1, i64::from(u32::MAX)) as u32;
        let lines = advancement_wrap_text_lines(text, width, fonts, item_runtime);
        let max_width = lines
            .iter()
            .map(|line| hud_logical_component_text_width_f32(&line.runs, fonts))
            .reduce(f32::max)
            .unwrap_or(0.0);
        let distance = (max_width - preferred_width as f32).abs();
        if distance <= 10.0 {
            return lines;
        }
        if distance < best_distance {
            best_distance = distance;
            best = lines;
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_advancement_hover_boxes_and_text(
    background_layers: &mut Vec<HudInventoryBackgroundLayer>,
    text_labels: &mut Vec<HudInventoryTextLabel>,
    widget: &bbb_client_world::AdvancementWidgetSummary,
    scroll_x: i32,
    scroll_y: i32,
    window_x: i32,
    inside_x: i32,
    inside_y: i32,
    screen_width: u32,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let title_lines = advancement_wrap_text_lines(
        &widget.title,
        ADVANCEMENTS_HOVER_TITLE_MAX_WIDTH,
        fonts,
        item_runtime,
    );
    let progress_runs = (widget.progress_total > 1).then(|| {
        advancement_progress_runs(
            widget.progress_done.min(widget.progress_total),
            widget.progress_total,
            item_runtime,
        )
    });
    let progress_width = progress_runs
        .as_ref()
        .map(|runs| hud_component_text_width(runs, fonts))
        .unwrap_or_default();
    let title_width = title_lines
        .iter()
        .map(|line| line.width.max(0) as u32)
        .max()
        .unwrap_or_default()
        .max(ADVANCEMENTS_HOVER_TITLE_MIN_WIDTH);
    let max_progress_width = advancement_max_progress_width(widget, fonts, item_runtime);
    let preferred_description_width = ADVANCEMENTS_HOVER_PROGRESS_PREFIX_WIDTH
        .saturating_add(title_width)
        .saturating_add(max_progress_width);
    let description_lines = advancement_description_lines(
        &widget.description,
        preferred_description_width,
        fonts,
        item_runtime,
    );
    let tooltip_width = description_lines
        .iter()
        .map(|line| line.width.max(0) as u32)
        .fold(preferred_description_width, u32::max)
        .saturating_add(
            ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT + ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT,
        );
    let Ok(tooltip_width_i32) = i32::try_from(tooltip_width) else {
        return;
    };
    let title_bar_height = i32::try_from(title_lines.len())
        .unwrap_or_default()
        .saturating_mul(ADVANCEMENTS_HOVER_LINE_HEIGHT)
        .saturating_add(ADVANCEMENTS_HOVER_TITLE_PADDING_TOP)
        .saturating_add(ADVANCEMENTS_HOVER_TITLE_PADDING_BOTTOM);
    let description_text_height = i32::try_from(description_lines.len())
        .unwrap_or_default()
        .saturating_mul(ADVANCEMENTS_HOVER_LINE_HEIGHT);
    let description_height = ADVANCEMENTS_HOVER_DESCRIPTION_PADDING_TOP + description_text_height;
    let title_top =
        scroll_y + widget.y + (ADVANCEMENTS_HOVER_FRAME_SIZE as i32 - title_bar_height) / 2;
    let title_bar_bottom = title_top + title_bar_height;
    let left_side =
        window_x + scroll_x + widget.x + tooltip_width_i32 + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            >= i32::try_from(screen_width).unwrap_or(i32::MAX);
    let top_side =
        title_bar_bottom + description_height >= ADVANCEMENTS_WINDOW_INSIDE_HEIGHT as i32;
    let title_left = if left_side {
        scroll_x + widget.x - tooltip_width_i32
            + ADVANCEMENTS_HOVER_FRAME_SIZE as i32
            + ADVANCEMENTS_HOVER_RIGHT_MARGIN
    } else {
        scroll_x + widget.x
    };
    let title_bar_height_u32 = u32::try_from(title_bar_height.max(1)).unwrap_or_default();

    if !description_lines.is_empty() {
        let background_height = title_bar_height + description_height;
        let background_y = if top_side {
            title_bar_bottom - background_height
        } else {
            title_top
        };
        background_layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(
                HudAdvancementHoverBoxSprite::Title.sprite_path(),
            ),
            inside_x + title_left,
            inside_y + background_y,
            tooltip_width,
            u32::try_from(background_height.max(1)).unwrap_or_default(),
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    }

    let (first_box, second_box, first_half_width, frame_obtained) =
        advancement_hover_progress_box_state(widget, tooltip_width);
    let second_half_width = tooltip_width.saturating_sub(first_half_width);
    if first_box == second_box {
        background_layers.push(hud_inventory_background_layer(
            HudInventoryBackgroundTexture::Sprite(first_box.sprite_path()),
            inside_x + title_left,
            inside_y + title_top,
            tooltip_width,
            title_bar_height_u32,
            [0.0, 0.0],
            [1.0, 1.0],
        ));
    } else {
        push_advancement_hover_progress_box_segment(
            background_layers,
            first_box,
            inside_x + title_left,
            inside_y + title_top,
            first_half_width,
            title_bar_height_u32,
            0,
            first_half_width,
        );
        push_advancement_hover_progress_box_segment(
            background_layers,
            second_box,
            inside_x + title_left + i32::try_from(first_half_width).unwrap_or_default(),
            inside_y + title_top,
            second_half_width,
            title_bar_height_u32,
            ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH.saturating_sub(second_half_width),
            ADVANCEMENTS_HOVER_BOX_SOURCE_WIDTH,
        );
    }
    background_layers.push(hud_inventory_background_layer(
        HudInventoryBackgroundTexture::Sprite(
            advancement_widget_frame_sprite(widget.frame_type, frame_obtained).sprite_path(),
        ),
        inside_x + scroll_x + widget.x + ADVANCEMENTS_WIDGET_FRAME_OFFSET_X,
        inside_y + scroll_y + widget.y,
        ADVANCEMENTS_WIDGET_FRAME_WIDTH,
        ADVANCEMENTS_WIDGET_FRAME_HEIGHT,
        [0.0, 0.0],
        [1.0, 1.0],
    ));

    let title_y = inside_y + title_top + ADVANCEMENTS_HOVER_TITLE_PADDING_TOP;
    let title_x = if left_side {
        inside_x + title_left + ADVANCEMENTS_HOVER_TITLE_PADDING_LEFT as i32 + 2
    } else {
        inside_x + scroll_x + widget.x + ADVANCEMENTS_HOVER_TITLE_X
    };
    push_advancement_hover_text_lines(
        text_labels,
        &title_lines,
        title_x,
        title_y,
        ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR,
    );
    if let Some(progress_runs) = progress_runs {
        let progress_text = progress_runs
            .iter()
            .map(|run| run.visual.text.as_str())
            .collect();
        let progress_x = if left_side {
            inside_x + scroll_x + widget.x - progress_width
        } else {
            inside_x + scroll_x + widget.x + tooltip_width_i32
                - progress_width
                - ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT as i32
        };
        push_advancement_hover_text_label(
            text_labels,
            progress_text,
            progress_x,
            title_y,
            progress_width.max(0) as u32,
            ADVANCEMENTS_HOVER_TITLE_TEXT_COLOR,
            progress_runs,
        );
    }

    let description_y = if top_side {
        inside_y + title_top - description_text_height + 1
    } else {
        inside_y + title_bar_bottom
    };
    push_advancement_hover_text_lines(
        text_labels,
        &description_lines,
        inside_x + title_left + ADVANCEMENTS_HOVER_TITLE_PADDING_RIGHT as i32,
        description_y,
        advancement_hover_description_text_color(widget.frame_type),
    );
}

pub(super) fn advancement_label_runs(
    key: &str,
    fallback: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    let template = localized_gui_text(item_runtime, key, fallback);
    let mut raw = Vec::new();
    visit_localized_template(&template, 0, |part| {
        if let LocalizedTemplatePart::Literal(text) = part {
            raw.push(HudComponentTextRun::plain(text));
        }
    });
    bbb_client_render::hud_resolve_formatted_component_text(&raw)
}

pub(super) fn advancement_progress_runs(
    done: usize,
    total: usize,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    let template = localized_gui_text(item_runtime, "advancements.progress", "%s/%s");
    let arguments = [done.to_string(), total.to_string()];
    let mut raw = Vec::new();
    visit_localized_template(&template, arguments.len(), |part| {
        let text = match part {
            LocalizedTemplatePart::Literal(text) => text,
            LocalizedTemplatePart::Argument { index, .. } => &arguments[index],
        };
        raw.push(HudComponentTextRun::plain(text));
    });
    bbb_client_render::hud_resolve_formatted_component_text(&raw)
}

fn advancement_max_progress_width(
    widget: &bbb_client_world::AdvancementWidgetSummary,
    fonts: &HudFontGlyphBook,
    item_runtime: Option<&NativeItemRuntime>,
) -> u32 {
    if widget.progress_total <= 1 {
        return 0;
    }
    let runs =
        advancement_progress_runs(widget.progress_total, widget.progress_total, item_runtime);
    (hud_component_text_width(&runs, fonts).max(0) as u32)
        .saturating_add(ADVANCEMENTS_HOVER_PROGRESS_PADDING)
}

fn push_advancement_hover_text_lines(
    labels: &mut Vec<HudInventoryTextLabel>,
    lines: &[AdvancementTextLine],
    x: i32,
    y: i32,
    tint: [f32; 4],
) {
    for (line_index, line) in lines.iter().enumerate() {
        push_advancement_hover_text_label(
            labels,
            line.text.clone(),
            x,
            y + i32::try_from(line_index).unwrap_or_default() * ADVANCEMENTS_HOVER_LINE_HEIGHT,
            line.width.max(0) as u32,
            tint,
            line.runs.clone(),
        );
    }
}

fn push_advancement_hover_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: String,
    x: i32,
    y: i32,
    width: u32,
    tint: [f32; 4],
    runs: Vec<HudComponentTextRun>,
) {
    labels.push(HudInventoryTextLabel {
        x,
        y,
        width,
        text,
        tint,
        background: None,
        input: None,
        scrolling: None,
        shadow: false,
        font: None,
        component_runs: runs,
        runs: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advancement_formatted_title_preserves_color_when_wrapping() {
        let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
        glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(1.0));
        let fonts = HudFontGlyphBook::from(glyphs);
        let lines = advancement_wrap_text_lines(&RichTextComponent::plain("§aAB"), 1, &fonts, None);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert!(lines
            .iter()
            .all(|line| line.width == 1 && line.runs[0].visual.color == Some(0x55ff55)));
    }

    #[test]
    fn advancement_description_tries_wider_margin_before_accepting_best_split() {
        let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
        glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(1.0));
        let fonts = HudFontGlyphBook::from(glyphs);
        let text = RichTextComponent::plain(format!(
            "{} {} {}",
            "a".repeat(18),
            "b".repeat(18),
            "c".repeat(18)
        ));
        let lines = advancement_description_lines(&text, 30, &fonts, None);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            [
                "aaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbb",
                "cccccccccccccccccc"
            ]
        );
        assert_eq!(lines[0].width, 37);
        assert!(
            advancement_description_lines(&RichTextComponent::plain(""), 30, &fonts, None)
                .is_empty()
        );
    }
}
