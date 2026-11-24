use winit::dpi::PhysicalSize;

use super::death_screen::{
    confirmation_geometry, push_button, push_menu_background, ConfirmationGeometry,
};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCommandConfirmationButton {
    Accept,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCommandConfirmationHitTarget {
    pub generation: u64,
    pub button: HudCommandConfirmationButton,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudCommandConfirmation {
    pub generation: u64,
    pub title: String,
    pub message_runs: Vec<HudComponentTextRun>,
    pub accept_label: String,
    pub cancel_label: String,
    pub focused: Option<HudCommandConfirmationButton>,
    pub hovered: Option<HudCommandConfirmationButton>,
}

fn layout(
    screen: &HudCommandConfirmation,
    fonts: &HudFontGlyphBook,
    size: PhysicalSize<u32>,
) -> (ConfirmationGeometry, Vec<Vec<HudComponentTextRun>>) {
    let mut lines: Vec<_> = hud_wrap_component_runs_with_interactions(
        &screen.message_runs,
        &[],
        size.width.saturating_sub(50).max(1),
        fonts,
    )
    .into_iter()
    .map(|line| line.runs)
    .collect();
    if lines.len() > 15 {
        lines.truncate(15);
        let last = lines.last_mut().unwrap();
        let mut remaining = (hud_component_runs_width(last, fonts).unwrap_or(0)
            - hud_raw_string_width("...", fonts.glyphs(None))) as f32;
        let mut prefix = Vec::new();
        for run in last.iter() {
            let mut text = String::new();
            let mut stopped = false;
            for ch in run.visual.text.chars() {
                remaining -= hud_font_glyph(ch, fonts.glyphs(run.font.as_deref()))
                    .styled_advance_f32(run.visual.style);
                if remaining < 0.0 {
                    stopped = true;
                    break;
                }
                text.push(ch);
            }
            if !text.is_empty() {
                let mut kept = run.clone();
                kept.visual.text = text;
                prefix.push(kept);
            }
            if stopped {
                break;
            }
        }
        prefix.push(HudComponentTextRun::plain("..."));
        *last = prefix;
    }
    let geometry = confirmation_geometry(
        hud_coded_string_width(&hud_coded_string_runs(&screen.title), fonts.glyphs(None)),
        lines
            .iter()
            .map(|line| hud_component_runs_width(line, fonts).unwrap_or(0))
            .max()
            .unwrap_or(0),
        lines.len() as i32 * 9,
        150,
        2,
        size,
    );
    (geometry, lines)
}

pub(super) fn hit_target_at(
    screen: &HudCommandConfirmation,
    fonts: &HudFontGlyphBook,
    size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudCommandConfirmationHitTarget> {
    let (layout, _) = layout(screen, fonts, size);
    [
        HudCommandConfirmationButton::Accept,
        HudCommandConfirmationButton::Cancel,
    ]
    .into_iter()
    .zip(layout.buttons)
    .find_map(|(button, rect)| {
        rect.contains(position)
            .then_some(HudCommandConfirmationHitTarget {
                generation: screen.generation,
                button,
            })
    })
}

pub(super) fn refresh_hover(
    screen: &mut HudCommandConfirmation,
    fonts: &HudFontGlyphBook,
    size: PhysicalSize<u32>,
    position: Option<(f64, f64)>,
) {
    screen.hovered = position
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .and_then(|(x, y)| hit_target_at(screen, fonts, size, (x.trunc(), y.trunc())))
        .map(|target| target.button);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    fonts: &HudFontGlyphBook,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    size: PhysicalSize<u32>,
    screen: &HudCommandConfirmation,
) {
    push_menu_background(vertices, commands, white_pixel, background, size);
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let (layout, lines) = layout(screen, fonts, size);
    // `ClientPacketListener` builds this title as a Component; the
    // centering width above already read this same resolved run list.
    let title = hud_coded_string_runs(&screen.title)
        .into_iter()
        .map(HudComponentTextRun::from)
        .collect::<Vec<_>>();
    for (runs, x, y) in std::iter::once((title.as_slice(), layout.title_x, layout.content_y)).chain(
        lines.iter().enumerate().map(|(index, line)| {
            (
                line.as_slice(),
                layout.message_center_x - hud_component_runs_width(line, fonts).unwrap_or(0) / 2,
                layout.content_y + 17 + index as i32 * 9,
            )
        }),
    ) {
        push_hud_component_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            size,
            &HudComponentScreenTextDraw {
                runs,
                origin: (x as f32, y as f32),
                scale: 1.0,
                tint: HUD_TINT_WHITE,
                backdrop_opacity: 0.0,
            },
        );
    }
    for ((button, label), rect) in [
        (
            HudCommandConfirmationButton::Accept,
            hud_coded_string_runs(&screen.accept_label),
        ),
        (
            HudCommandConfirmationButton::Cancel,
            hud_coded_string_runs(&screen.cancel_label),
        ),
    ]
    .into_iter()
    .zip(layout.buttons)
    {
        push_button(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            fonts.glyphs(None),
            obfuscated_pool,
            frame_index,
            size,
            rect,
            &label,
            true,
            screen.hovered == Some(button) || screen.focused == Some(button),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in "A.?".chars() {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: 6.0,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        HudFontGlyphBook::from(glyphs)
    }
    fn screen() -> HudCommandConfirmation {
        HudCommandConfirmation {
            generation: 123,
            title: "A".repeat(101),
            message_runs: vec![HudComponentTextRun::plain("AAA")],
            accept_label: "yes".into(),
            cancel_label: "no".into(),
            focused: Some(HudCommandConfirmationButton::Accept),
            hovered: None,
        }
    }
    #[test]
    fn command_confirmation_stationary_hover_refreshes_for_open_return_resize_and_leave() {
        let mut screen = screen();
        let fonts = fonts();
        let size = PhysicalSize::new(401, 301);
        let pointer = Some((50.9, 165.9));
        refresh_hover(&mut screen, &fonts, size, pointer);
        assert_eq!(screen.hovered, Some(HudCommandConfirmationButton::Accept));
        refresh_hover(&mut screen, &fonts, PhysicalSize::new(501, 301), pointer);
        assert_eq!(screen.hovered, None);
        screen.generation += 1;
        refresh_hover(&mut screen, &fonts, size, pointer);
        assert_eq!(screen.hovered, Some(HudCommandConfirmationButton::Accept));
        refresh_hover(&mut screen, &fonts, size, Some((199.0, 165.0)));
        assert_eq!(screen.hovered, None);
        refresh_hover(&mut screen, &fonts, size, Some((203.0, 165.0)));
        assert_eq!(screen.hovered, Some(HudCommandConfirmationButton::Cancel));
        refresh_hover(&mut screen, &fonts, size, None);
        assert_eq!(screen.hovered, None);
    }

    #[test]
    fn command_confirmation_uses_shared_geometry_and_generation_tagged_half_open_buttons() {
        let screen = screen();
        let fonts = fonts();
        let size = PhysicalSize::new(401, 301);
        let (geometry, _) = layout(&screen, &fonts, size);
        assert_eq!(geometry.title_x, -102);
        assert_eq!(geometry.content_y, 115);
        for (x, expected) in [
            (49.0, Some(HudCommandConfirmationButton::Accept)),
            (198.0, Some(HudCommandConfirmationButton::Accept)),
            (199.0, None),
            (202.0, None),
            (203.0, Some(HudCommandConfirmationButton::Cancel)),
            (353.0, None),
        ] {
            assert_eq!(
                hit_target_at(&screen, &fonts, size, (x, 165.0)),
                expected.map(|button| HudCommandConfirmationHitTarget {
                    generation: 123,
                    button
                })
            );
        }
        assert_eq!(hit_target_at(&screen, &fonts, size, (50.0, 185.0)), None);
    }
    #[test]
    fn command_confirmation_replaces_underlying_screen_draws_and_gates_ingame_ui() {
        let Some(mut renderer) = Renderer::new_offscreen(401, 301) else {
            return;
        };
        renderer.set_hud_command_confirmation(Some(screen()));
        renderer.set_hud_screen_presence(true, false);
        assert!(renderer.hud_has_any_screen());
        assert!(!renderer.hud_has_ingame_ui_screen());
        let count = renderer.collect_hud_draws().commands.len();
        renderer.set_hud_death_screen(Some(HudDeathScreen::QuitConfirmation(
            HudDeathQuitConfirmationScreen {
                generation: 9,
                title: "quit".into(),
                title_screen_label: "quit".into(),
                respawn_label: "respawn".into(),
                buttons_active: true,
                focused: HudDeathScreenButton::QuitTitleScreen,
                hovered: None,
            },
        )));
        assert_eq!(renderer.collect_hud_draws().commands.len(), count);
        renderer.set_hud_command_confirmation(None);
        assert_eq!(
            renderer.hud_command_confirmation_hit_target_at((50.0, 165.0)),
            None
        );
    }

    #[test]
    fn command_confirmation_wraps_using_each_runs_font() {
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'A',
            HudAsciiGlyph {
                advance: 60.0,
                ..HudAsciiGlyph::default()
            },
        );
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("minecraft:default".into(), HudFontGlyphMap::new()),
                ("example:wide".into(), glyphs),
            ]),
        );
        let mut screen = screen();
        screen.message_runs[0].font = Some("example:wide".into());
        let (_, lines) = layout(&screen, &fonts, PhysicalSize::new(170, 301));
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][0].visual.text, "AA");
        assert_eq!(lines[1][0].font.as_deref(), Some("example:wide"));
    }

    #[test]
    fn command_confirmation_preserves_rich_runs_and_caps_message_only() {
        let mut screen = screen();
        let mut run = HudComponentTextRun::plain(
            std::iter::repeat_n("AAAAA", 20)
                .collect::<Vec<_>>()
                .join("\n"),
        );
        run.visual.style.bold = true;
        run.visual.color = Some(0x334466);
        screen.message_runs = vec![run];
        let fonts = fonts();
        let (geometry, lines) = layout(&screen, &fonts, PhysicalSize::new(401, 301));
        assert_eq!(lines.len(), 15);
        assert!(lines[0][0].visual.style.bold);
        assert_eq!(lines[0][0].visual.color, Some(0x334466));
        assert_eq!(lines[14].last().unwrap().visual.text, "...");
        assert_eq!(geometry.title_x, -102);
        assert_eq!(screen.title.len(), 101);
    }
}
