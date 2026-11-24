use super::*;

const SCOREBOARD_LINE_HEIGHT: i32 = 9;
const SCOREBOARD_RIGHT_PADDING: i32 = 3;
const SCOREBOARD_SPACER: &str = ": ";
const SCOREBOARD_BODY_ALPHA: f32 = 0.3;
const SCOREBOARD_HEADER_ALPHA: f32 = 0.4;

/// One right-side scoreboard entry after native Component localization.
#[derive(Debug, Clone, PartialEq)]
pub struct HudScoreboardSidebarEntry {
    pub name_runs: Vec<HudStyledTextRun>,
    pub score_runs: Vec<HudStyledTextRun>,
}

/// Vanilla `Gui.displayScoreboardSidebar` source data. Native owns Component
/// localization; renderer owns font measurement and final right-edge layout.
#[derive(Debug, Clone, PartialEq)]
pub struct HudScoreboardSidebar {
    pub title_runs: Vec<HudStyledTextRun>,
    pub entries: Vec<HudScoreboardSidebarEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudScoreboardEntryLayout {
    score_width: i32,
    y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HudScoreboardLayout {
    width: u32,
    left: i32,
    right: i32,
    header_y: i32,
    bottom: i32,
    title_x: i32,
    entries: Vec<HudScoreboardEntryLayout>,
}

pub(super) fn sanitize_hud_scoreboard_sidebar(
    mut sidebar: HudScoreboardSidebar,
) -> HudScoreboardSidebar {
    sidebar.title_runs = sanitize_hud_styled_runs(sidebar.title_runs, "");
    sidebar.entries.truncate(15);
    for entry in &mut sidebar.entries {
        entry.name_runs = sanitize_hud_styled_runs(std::mem::take(&mut entry.name_runs), "");
        entry.score_runs = sanitize_hud_styled_runs(std::mem::take(&mut entry.score_runs), "");
    }
    sidebar
}

fn hud_scoreboard_layout(
    surface_size: PhysicalSize<u32>,
    title_width: i32,
    entry_widths: &[(i32, i32)],
    spacer_width: i32,
) -> HudScoreboardLayout {
    let width = entry_widths
        .iter()
        .map(|(name_width, score_width)| {
            name_width.saturating_add(if *score_width > 0 {
                spacer_width.saturating_add(*score_width)
            } else {
                0
            })
        })
        .fold(title_width, i32::max);
    let screen_width = surface_size.width as i32;
    let screen_height = surface_size.height as i32;
    let height = i32::try_from(entry_widths.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(SCOREBOARD_LINE_HEIGHT);
    let bottom = screen_height / 2 + height / 3;
    let left = screen_width
        .saturating_sub(width)
        .saturating_sub(SCOREBOARD_RIGHT_PADDING);
    let right = screen_width
        .saturating_sub(SCOREBOARD_RIGHT_PADDING)
        .saturating_add(2);
    let header_y = bottom.saturating_sub(height);
    let entries = entry_widths
        .iter()
        .enumerate()
        .map(
            |(index, (_name_width, score_width))| HudScoreboardEntryLayout {
                score_width: *score_width,
                y: bottom.saturating_sub(
                    i32::try_from(entry_widths.len() - index)
                        .unwrap_or(i32::MAX)
                        .saturating_mul(SCOREBOARD_LINE_HEIGHT),
                ),
            },
        )
        .collect();
    HudScoreboardLayout {
        width: width.max(0) as u32,
        left,
        right,
        header_y,
        bottom,
        title_x: left + width / 2 - title_width / 2,
        entries,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_scoreboard_sidebar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    sidebar: &HudScoreboardSidebar,
) {
    let title_width = hud_font_runs_width(&sidebar.title_runs, glyphs).unwrap_or(0);
    let entry_widths = sidebar
        .entries
        .iter()
        .map(|entry| {
            (
                hud_font_runs_width(&entry.name_runs, glyphs).unwrap_or(0),
                hud_font_runs_width(&entry.score_runs, glyphs).unwrap_or(0),
            )
        })
        .collect::<Vec<_>>();
    let spacer_width = hud_raw_string_width(SCOREBOARD_SPACER, glyphs);
    let layout = hud_scoreboard_layout(surface_size, title_width, &entry_widths, spacer_width);
    let rect_width = layout.width.saturating_add(4);
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        layout.left - 2,
        layout.header_y - 10,
        rect_width,
        9,
        [0.0, 0.0, 0.0, SCOREBOARD_HEADER_ALPHA],
    );
    push_hud_fill_rect(
        vertices,
        commands,
        surface_size,
        layout.left - 2,
        layout.header_y - 1,
        rect_width,
        layout
            .bottom
            .saturating_sub(layout.header_y.saturating_sub(1))
            .max(0) as u32,
        [0.0, 0.0, 0.0, SCOREBOARD_BODY_ALPHA],
    );
    push_scoreboard_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &sidebar.title_runs,
        layout.title_x,
        layout.header_y - 9,
    );

    for (entry, entry_layout) in sidebar.entries.iter().zip(&layout.entries) {
        push_scoreboard_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &entry.name_runs,
            layout.left,
            entry_layout.y,
        );
        push_scoreboard_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &entry.score_runs,
            layout.right - entry_layout.score_width,
            entry_layout.y,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_scoreboard_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    x: i32,
    y: i32,
) {
    let geometry = hud_styled_text_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (x as f32, y as f32),
        false,
        HUD_TINT_WHITE,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoreboard_layout_matches_vanilla_right_edge_and_vertical_formula() {
        let layout = hud_scoreboard_layout(
            PhysicalSize::new(320, 240),
            42,
            &[(30, 12), (50, 0), (24, 18)],
            8,
        );
        assert_eq!(layout.width, 50);
        assert_eq!(layout.left, 267);
        assert_eq!(layout.right, 319);
        assert_eq!(layout.bottom, 129);
        assert_eq!(layout.header_y, 102);
        assert_eq!(layout.title_x, 271);
        assert_eq!(
            layout
                .entries
                .iter()
                .map(|entry| entry.y)
                .collect::<Vec<_>>(),
            [102, 111, 120]
        );
    }

    #[test]
    fn scoreboard_layout_counts_spacer_only_for_visible_scores() {
        let layout = hud_scoreboard_layout(PhysicalSize::new(200, 100), 1, &[(10, 0), (10, 1)], 4);
        assert_eq!(layout.width, 15);
    }

    #[test]
    fn scoreboard_layout_reads_the_resolved_runs_the_glyph_pass_draws() {
        // `bbb-client`'s `scoreboard_sidebar_runs_consume_legacy_codes_with_the_inherited_style_as_reset`
        // pins the projection of `K§cills§r!` for the title and of the team
        // prefix plus `A§cce§r!` for the entry name; this is the right-edge
        // layout the same list produces.
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 6.0), ('b', 6.0), ('?', 6.0), (':', 6.0), (' ', 4.0)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let bold = HudTextStyle {
            bold: true,
            ..HudTextStyle::default()
        };
        let run = |text: &str, style: HudTextStyle, color: Option<u32>| HudStyledTextRun {
            text: text.to_string(),
            style,
            color,
            ..HudStyledTextRun::plain("")
        };
        let title_runs = vec![
            run("a", bold, None),
            run("b", HudTextStyle::default(), Some(0xFF_55_55)),
            run("a", bold, None),
        ];
        let name_runs = vec![
            run("ab", bold, Some(0xFF5555)),
            run("a", HudTextStyle::default(), Some(0xFF5555)),
            run("b", bold, Some(0xFF5555)),
            run("a", HudTextStyle::default(), Some(0xFF5555)),
        ];
        // Three glyphs in the title and four in the entry name: the consumed
        // `§` codes and their `r`/`c` operands added no advance, and an
        // unconsumed `§` would have measured the `?` fallback instead.
        let title_width = hud_font_runs_width(&title_runs, &glyphs).unwrap_or(0);
        assert_eq!(title_width, 20);
        let entry_widths = vec![(
            hud_font_runs_width(&name_runs, &glyphs).unwrap_or(0),
            hud_font_runs_width(&[run("a", bold, None)], &glyphs).unwrap_or(0),
        )];
        assert_eq!(entry_widths, [(33, 7)]);
        assert_eq!(hud_raw_string_width(SCOREBOARD_SPACER, &glyphs), 10);
        let layout = hud_scoreboard_layout(
            PhysicalSize::new(320, 240),
            title_width,
            &entry_widths,
            hud_raw_string_width(SCOREBOARD_SPACER, &glyphs),
        );
        assert_eq!(layout.width, 50);
        assert_eq!(layout.left, 267);
        assert_eq!(layout.right, 319);
        assert_eq!(layout.title_x, 282);
    }

    #[test]
    fn scoreboard_sanitizer_limits_entries_and_preserves_blank_score_width() {
        let sidebar = sanitize_hud_scoreboard_sidebar(HudScoreboardSidebar {
            title_runs: vec![HudStyledTextRun::plain("Title")],
            entries: (0..20)
                .map(|index| HudScoreboardSidebarEntry {
                    name_runs: vec![HudStyledTextRun::plain(format!("P{index}"))],
                    score_runs: Vec::new(),
                })
                .collect(),
        });
        assert_eq!(sidebar.entries.len(), 15);
        assert_eq!(sidebar.entries[0].score_runs, [HudStyledTextRun::plain("")]);
    }
}
