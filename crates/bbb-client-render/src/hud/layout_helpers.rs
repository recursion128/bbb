use super::*;

/// StatsScreen's three TabNavigationBar buttons share their draw and hit rectangles.
pub fn hud_stats_tab_rects(size: PhysicalSize<u32>) -> [(i32, i32, i32, i32); 3] {
    let width = size.width as i32;
    let tabs_width = width.min(400) - 28;
    let tab_width = ((tabs_width / 3 + 1) / 2) * 2;
    let left = (((width - tabs_width) / 2 + 1) / 2) * 2;
    std::array::from_fn(|index| {
        (
            left + index as i32 * tab_width,
            0,
            tab_width,
            HUD_STATS_LIST_TOP,
        )
    })
}

/// Returns whether vanilla extracts the air row for this frame.
pub fn hud_air_supply_visible(state: HudAirSupply) -> bool {
    hud_air_bubbles_visible(state.air, state.max_air, state.eye_in_water)
}

/// Returns the popping bubble and its sound inputs from the same formulas used
/// by the air-bubble renderer.
pub fn hud_air_supply_pop(state: HudAirSupply) -> Option<HudAirBubblePop> {
    air_bubble_pop(state.air, state.max_air, state.eye_in_water).map(|pop| HudAirBubblePop {
        bubble: pop.bubble,
        empty_bubbles: pop.empty_bubbles,
    })
}

pub(super) fn hud_mob_effect_natural_cmp(a: &HudMobEffect, b: &HudMobEffect) -> std::cmp::Ordering {
    let uses_duration_order =
        (a.duration_ticks <= 32_147 || b.duration_ticks <= 32_147) && (!a.ambient || !b.ambient);
    if uses_duration_order {
        a.ambient
            .cmp(&b.ambient)
            .then_with(|| (a.duration_ticks == -1).cmp(&(b.duration_ticks == -1)))
            .then_with(|| a.duration_ticks.cmp(&b.duration_ticks))
            .then_with(|| a.color.cmp(&b.color))
    } else {
        a.ambient
            .cmp(&b.ambient)
            .then_with(|| a.color.cmp(&b.color))
    }
}

pub(super) fn hud_mob_effect_icon_alpha(effect: &HudMobEffect) -> f32 {
    if effect.ambient || effect.duration_ticks == -1 || effect.duration_ticks > 200 {
        return 1.0;
    }
    let remaining = effect.duration_ticks;
    let used_seconds = 10 - remaining / 20;
    let steady = (remaining as f32 / 10.0 / 5.0 * 0.5).clamp(0.0, 0.5);
    let pulse = (remaining as f32 * std::f32::consts::PI / 5.0).cos()
        * (used_seconds as f32 / 10.0 * 0.25).clamp(0.0, 0.25);
    (steady + pulse).clamp(0.0, 1.0)
}

pub(super) fn hud_sleep_overlay_tint(alpha: u8) -> [f32; 4] {
    [
        16.0 / 255.0,
        16.0 / 255.0,
        32.0 / 255.0,
        f32::from(alpha) / 255.0,
    ]
}

pub(super) fn hud_banner_pattern_atlas_uv(
    renderer: &Renderer,
    texture: BannerPatternTexture,
) -> Option<(HudBannerPatternAtlas, EntityModelUvRect)> {
    match texture {
        BannerPatternTexture::Builtin(pattern) => {
            let texture = banner_pattern_texture_ref(pattern);
            let uv = renderer
                .pack
                .entity_model_texture_atlas
                .as_ref()?
                .layout
                .entries
                .iter()
                .find(|entry| entry.texture == texture)?
                .uv;
            Some((HudBannerPatternAtlas::StaticEntity, uv))
        }
        BannerPatternTexture::Runtime(texture) => {
            let uv = renderer
                .entity_dynamic_player_texture_atlas
                .as_ref()?
                .layout
                .entries
                .iter()
                .find(|entry| entry.handle == texture.handle)?
                .uv;
            Some((HudBannerPatternAtlas::DynamicEntity, uv))
        }
    }
}

pub(super) fn hud_rich_tooltip_lines(
    tooltip: &HudRichTooltip,
    advanced_item_tooltips: bool,
) -> &[HudRichTooltipLine] {
    if advanced_item_tooltips {
        tooltip.advanced_lines.as_deref().unwrap_or(&tooltip.lines)
    } else {
        &tooltip.lines
    }
}

/// Resolves one tooltip line the way vanilla draws it: `GuiGraphicsExtractor`
/// turns every tooltip Component into `getVisualOrderText()`, which iterates the
/// part contents through `SubStringSource`, so `§` codes are consumed
/// before the line is measured or drawn. An unwrapped tooltip has no other legacy
/// pass; the wrapped chat-style lines resolve while splitting instead.
pub(super) fn hud_resolved_tooltip_runs(runs: &[HudComponentTextRun]) -> Vec<HudComponentTextRun> {
    if runs.iter().any(|run| run.visual.text.contains('§')) {
        hud_resolve_formatted_component_text(runs)
    } else {
        runs.to_vec()
    }
}

/// Vanilla `Gui.extractOverlayMessage` fade:
/// `alpha = (int)(t * 255 / 20)` capped at 255; the draw is dropped when the
/// result is not `> 0`. `t` is `overlayMessageTime - partialTick`.
pub(super) fn hud_overlay_message_alpha(t: f32) -> i32 {
    let alpha = (t * 255.0 / 20.0) as i32;
    alpha.min(255)
}

/// Vanilla jukebox now-playing rainbow:
/// `Mth.hsvToArgb(t / 50, 0.7, 0.6, alpha)`. The hue derives from the
/// remaining display time, so the colour cycle is deterministic per frame
/// state (no wall clock). `Mth.hsvToArgb` keeps its Java quirk verbatim: `h`
/// is wrapped mod 6 but `f` is taken against the wrapped `h`, and the final
/// channels are clamped to 0..255.
pub(super) fn hud_overlay_message_rainbow_rgb(t: f32) -> [f32; 3] {
    const SATURATION: f32 = 0.7;
    const VALUE: f32 = 0.6;
    let hue = t / 50.0;
    let h = ((hue * 6.0) as i32) % 6;
    let f = hue * 6.0 - h as f32;
    let p = VALUE * (1.0 - SATURATION);
    let q = VALUE * (1.0 - f * SATURATION);
    let s = VALUE * (1.0 - (1.0 - f) * SATURATION);
    // `t > 0` whenever the alpha gate passes, so `h` is in 0..6 here.
    let (red, green, blue) = match h {
        1 => (q, VALUE, p),
        2 => (p, VALUE, s),
        3 => (p, q, VALUE),
        4 => (s, p, VALUE),
        5 => (VALUE, p, q),
        _ => (VALUE, s, p),
    };
    [red, green, blue].map(|channel| ((channel * 255.0) as i32).clamp(0, 255) as f32 / 255.0)
}

/// Resolves the held-item name line, mirroring `Gui.extractSelectedItemName`
/// drawn while `toolHighlightTimer > 0` at
/// `((guiWidth - width) / 2, guiHeight - 59)` (plus 14 without a health row),
/// white at `min(toolHighlightTimer * 256 / 10, 255)` and skipped at alpha 0.
pub(super) fn hud_selected_item_name_draw<'a>(
    state: &'a HudSelectedItemName,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    backdrop_opacity: f32,
) -> Option<HudComponentScreenTextDraw<'a>> {
    if state.highlight_ticks <= 0 {
        return None;
    }
    let alpha = ((state.highlight_ticks as f32 * 256.0 / 10.0) as i32).min(255);
    if alpha <= 0 {
        return None;
    }
    let width = hud_component_runs_width(&state.runs, fonts).unwrap_or(0);
    Some(HudComponentScreenTextDraw {
        runs: &state.runs,
        origin: hud_selected_item_name_origin(surface_size, width, state.raised),
        scale: 1.0,
        tint: [1.0, 1.0, 1.0, alpha as f32 / 255.0],
        backdrop_opacity,
    })
}

/// Resolves the stacked boss bars, mirroring
/// `BossHealthOverlay.extractRenderState`:
/// bars walk top-down from y=12 stepping 10+9, dropping the remainder once
/// the accumulated offset reaches `guiHeight / 3`; each bar submits its
/// sprite layers and then its name, centered at `(guiWidth/2 - width/2,
/// y - 9)` in opaque white at scale 1.
pub(super) fn hud_boss_bar_draws<'a>(
    bars: &'a [HudBossBar],
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> Vec<HudBossBarDraw<'a>> {
    hud_boss_bar_rows(surface_size, bars.len())
        .into_iter()
        .zip(bars)
        .map(|(y, bar)| {
            let name_width = hud_font_runs_width(&bar.name_runs, glyphs).unwrap_or(0);
            HudBossBarDraw {
                y,
                layers: hud_boss_bar_layers(bar),
                name: HudScreenTextDraw {
                    runs: &bar.name_runs,
                    origin: hud_boss_bar_name_origin(surface_size, y, name_width),
                    scale: 1.0,
                    tint: HUD_TINT_WHITE,
                    backdrop_opacity: 0.0,
                },
            }
        })
        .collect()
}

/// Sprite layers in vanilla submission order (`BossHealthOverlay.extractBar`,
/// BossHealthOverlay): the full-width colored background, the
/// notched background on top, then — only when `Mth.lerpDiscrete` yields a
/// positive width — the colored and notched progress layers cropped to that
/// width.
pub(super) fn hud_boss_bar_layers(bar: &HudBossBar) -> Vec<HudBossBarLayer> {
    let mut layers = vec![HudBossBarLayer {
        sheet: HudBossBarSheet::ColorBackground(bar.color),
        width: HUD_BOSS_BAR_WIDTH,
    }];
    if bar.overlay != HudBossBarOverlay::Progress {
        layers.push(HudBossBarLayer {
            sheet: HudBossBarSheet::NotchedBackground(bar.overlay),
            width: HUD_BOSS_BAR_WIDTH,
        });
    }
    let progress_width = hud_boss_bar_progress_width(bar.progress);
    if progress_width > 0 {
        layers.push(HudBossBarLayer {
            sheet: HudBossBarSheet::ColorProgress(bar.color),
            width: progress_width,
        });
        if bar.overlay != HudBossBarOverlay::Progress {
            layers.push(HudBossBarLayer {
                sheet: HudBossBarSheet::NotchedProgress(bar.overlay),
                width: progress_width,
            });
        }
    }
    layers
}

/// Vanilla `Gui` gate: the experience level number renders only when
/// `experienceLevel > 0`, so a zero (or negative) projection clears the text.
pub(super) fn hud_experience_level_projection(level: Option<i32>) -> Option<i32> {
    level.filter(|&level| level > 0)
}

pub(super) fn hud_argb_alpha_channel(alpha: f32) -> u32 {
    if !alpha.is_finite() || alpha <= 0.0 {
        0
    } else {
        (alpha.clamp(0.0, 1.0) * 255.0).floor() as u32
    }
}

pub(super) fn hud_argb_black_alpha(alpha: f32) -> f32 {
    hud_argb_alpha_channel(alpha) as f32 / 255.0
}

pub(super) fn hud_argb_lerp(alpha: f64, start: u32, end: u32) -> u32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let a = hud_lerp_channel(alpha, (start >> 24) & 0xFF, (end >> 24) & 0xFF);
    let r = hud_lerp_channel(alpha, (start >> 16) & 0xFF, (end >> 16) & 0xFF);
    let g = hud_lerp_channel(alpha, (start >> 8) & 0xFF, (end >> 8) & 0xFF);
    let b = hud_lerp_channel(alpha, start & 0xFF, end & 0xFF);
    (a << 24) | (r << 16) | (g << 8) | b
}

fn hud_lerp_channel(alpha: f64, start: u32, end: u32) -> u32 {
    let value = start as i32 + (alpha * f64::from(end as i32 - start as i32)).floor() as i32;
    value.clamp(0, 255) as u32
}

pub(super) fn hud_argb_multiply(lhs: u32, rhs: u32) -> u32 {
    if lhs == 0xFFFF_FFFF {
        return rhs;
    }
    if rhs == 0xFFFF_FFFF {
        return lhs;
    }
    let a = ((lhs >> 24) & 0xFF) * ((rhs >> 24) & 0xFF) / 255;
    let r = ((lhs >> 16) & 0xFF) * ((rhs >> 16) & 0xFF) / 255;
    let g = ((lhs >> 8) & 0xFF) * ((rhs >> 8) & 0xFF) / 255;
    let b = (lhs & 0xFF) * (rhs & 0xFF) / 255;
    (a << 24) | (r << 16) | (g << 8) | b
}

pub(super) fn hud_argb_to_tint(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}

/// The PIP-texture sub-rect (as `0..1` UV fractions of the preview rect) a scissored blit
/// samples: `visible == rect ∩ scissor` maps to the matching texture region, identity UVs when
/// no scissor applies. Vanilla scissors the full-rect blit instead; for an axis-aligned scissor
/// the two are pixel-equivalent. wgpu's row-0-top texture origin means the PIP render is already
/// GUI-oriented — vanilla's `v0=1, v1=0` vertical flip is a GL framebuffer-origin artifact and
/// has no bbb counterpart.
pub(super) fn hud_entity_preview_blit_uv(
    rect: HudEntityPreviewRect,
    visible: HudEntityPreviewRect,
) -> HudUvRect {
    let width = rect.width.max(1) as f32;
    let height = rect.height.max(1) as f32;
    HudUvRect {
        min: [
            (visible.x - rect.x) as f32 / width,
            (visible.y - rect.y) as f32 / height,
        ],
        max: [
            (visible.right() - i64::from(rect.x)) as f32 / width,
            (visible.bottom() - i64::from(rect.y)) as f32 / height,
        ],
    }
}
