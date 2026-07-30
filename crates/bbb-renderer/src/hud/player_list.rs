use std::collections::{BTreeMap, BTreeSet};

use winit::dpi::PhysicalSize;

use super::layout::absolute_hud_rect;
use super::{
    hud_font_runs_width, hud_plain_text_width, hud_wrap_chat_runs, push_hud_debug_tinted_rect,
    push_hud_draw, push_hud_draw_with_uv, push_hud_screen_text_draw, HudDrawCommand,
    HudFontGlyphMap, HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu, HudStyledTextRun,
    HudUvRect, HudVertex, HUD_TINT_WHITE,
};
use crate::{EntityPlayerSkin, Renderer};

/// One frame's multiplayer player-list overlay. Entries arrive in vanilla
/// comparator order; the renderer owns font measurement, wrapping, columns,
/// and the cross-frame heart blink/display state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudPlayerList {
    pub header: Option<Vec<HudStyledTextRun>>,
    pub footer: Option<Vec<HudStyledTextRun>>,
    pub entries: Vec<HudPlayerListEntry>,
    /// Remote native clients show player faces only while the play transport
    /// is encrypted (`PlayerTabOverlay.showHead`).
    pub show_heads: bool,
    /// Vanilla `Gui.getGuiTicks()` at extraction time.
    pub tick_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudPlayerListEntry {
    /// Stable vanilla profile UUID, represented as a `u128` to keep the
    /// renderer independent of the protocol crate.
    pub profile_id: u128,
    pub name_runs: Vec<HudStyledTextRun>,
    pub skin: EntityPlayerSkin,
    pub show_hat: bool,
    pub upside_down: bool,
    pub latency: i32,
    pub spectator: bool,
    pub score: Option<HudPlayerListScore>,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPlayerListPingSprite {
    Unknown,
    One,
    Two,
    Three,
    Four,
    Five,
}

impl HudPlayerListPingSprite {
    pub const ALL: [Self; 6] = [
        Self::Unknown,
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
    ];

    pub const fn sprite_path(self) -> &'static str {
        match self {
            Self::Unknown => "icon/ping_unknown",
            Self::One => "icon/ping_1",
            Self::Two => "icon/ping_2",
            Self::Three => "icon/ping_3",
            Self::Four => "icon/ping_4",
            Self::Five => "icon/ping_5",
        }
    }

    pub const fn from_latency(latency: i32) -> Self {
        match latency {
            latency if latency < 0 => Self::Unknown,
            latency if latency < 150 => Self::Five,
            latency if latency < 300 => Self::Four,
            latency if latency < 600 => Self::Three,
            latency if latency < 1000 => Self::Two,
            _ => Self::One,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HudPlayerListScore {
    Integer(Vec<HudStyledTextRun>),
    Hearts {
        value: i32,
        /// Localized `%shp` component resolved by native.
        hp_text: String,
        /// Vanilla `Float.toString(score / 2.0F)` fallback.
        compact_text: String,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct HudPlayerListHeartSprites<'a> {
    pub(super) container: Option<&'a HudSpriteGpu>,
    pub(super) container_blinking: Option<&'a HudSpriteGpu>,
    pub(super) full: Option<&'a HudSpriteGpu>,
    pub(super) half: Option<&'a HudSpriteGpu>,
    pub(super) full_blinking: Option<&'a HudSpriteGpu>,
    pub(super) half_blinking: Option<&'a HudSpriteGpu>,
    pub(super) absorbing_full_blinking: Option<&'a HudSpriteGpu>,
    pub(super) absorbing_half_blinking: Option<&'a HudSpriteGpu>,
}

impl HudPlayerListHeartSprites<'_> {
    fn complete(self) -> bool {
        self.container.is_some()
            && self.container_blinking.is_some()
            && self.full.is_some()
            && self.half.is_some()
            && self.full_blinking.is_some()
            && self.half_blinking.is_some()
            && self.absorbing_full_blinking.is_some()
            && self.absorbing_half_blinking.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudPlayerListHealthState {
    last_value: i32,
    displayed_value: i32,
    last_update_tick: u64,
    blink_until_tick: u64,
}

impl HudPlayerListHealthState {
    fn new(value: i32) -> Self {
        Self {
            last_value: value,
            displayed_value: value,
            last_update_tick: 0,
            blink_until_tick: 0,
        }
    }

    /// Vanilla `PlayerTabOverlay.HealthState.update`: damage blinks for 20
    /// ticks, healing for 10, and the lagging displayed value catches up only
    /// after strictly more than 20 ticks.
    fn update(&mut self, value: i32, tick: u64) {
        if value != self.last_value {
            let blink_duration = if value < self.last_value { 20 } else { 10 };
            self.blink_until_tick = tick.saturating_add(blink_duration);
            self.last_value = value;
            self.last_update_tick = tick;
        }
        if tick.saturating_sub(self.last_update_tick) > 20 {
            self.displayed_value = value;
        }
    }

    fn is_blinking(self, tick: u64) -> bool {
        self.blink_until_tick > tick && (self.blink_until_tick - tick) % 6 >= 3
    }
}

/// Renderer-owned equivalent of `PlayerTabOverlay.visible` plus
/// `healthStates`. Visibility transitions clear the history, while each
/// visible extraction prunes UUIDs no longer in the list.
#[derive(Debug, Default)]
pub(crate) struct HudPlayerListRuntime {
    visible: bool,
    health_states: BTreeMap<u128, HudPlayerListHealthState>,
}

impl HudPlayerListRuntime {
    pub(super) fn synchronize(&mut self, list: Option<&HudPlayerList>) {
        let visible = list.is_some();
        if visible != self.visible {
            self.health_states.clear();
            self.visible = visible;
        }
        let Some(list) = list else {
            return;
        };

        let profile_ids = list
            .entries
            .iter()
            .map(|entry| entry.profile_id)
            .collect::<BTreeSet<_>>();
        self.health_states
            .retain(|profile_id, _| profile_ids.contains(profile_id));
        for entry in &list.entries {
            let Some(HudPlayerListScore::Hearts { value, .. }) = entry.score else {
                continue;
            };
            if entry.spectator {
                continue;
            }
            self.health_states
                .entry(entry.profile_id)
                .or_insert_with(|| HudPlayerListHealthState::new(value))
                .update(value, list.tick_count);
        }
    }

    fn health_state(&self, profile_id: u128) -> Option<HudPlayerListHealthState> {
        self.health_states.get(&profile_id).copied()
    }
}

pub(super) fn sanitize_hud_player_list(mut list: HudPlayerList) -> HudPlayerList {
    list.entries.truncate(80);
    list
}

impl Renderer {
    pub fn upload_hud_player_list_ping_sprite(
        &mut self,
        kind: HudPlayerListPingSprite,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        let sprite = self.upload_hud_sprite(width, height, rgba)?;
        self.hud_player_list_ping_sprites[kind as usize] = Some(sprite);
        Ok(())
    }

    fn hud_player_list_ping_sprite(&self, latency: i32) -> Option<&HudSpriteGpu> {
        self.hud_player_list_ping_sprites[HudPlayerListPingSprite::from_latency(latency) as usize]
            .as_ref()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_player_list<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    renderer: &'a Renderer,
    list: &HudPlayerList,
    runtime: &HudPlayerListRuntime,
    heart_sprites: HudPlayerListHeartSprites<'a>,
) {
    let entries = list.entries.as_slice();
    let slots = entries.len();
    let (columns, rows) = hud_player_list_grid(slots);
    let max_name_width = entries
        .iter()
        .map(|entry| hud_font_runs_width(&entry.name_runs, glyphs).unwrap_or(0))
        .max()
        .unwrap_or(0);
    let width_for_score = hud_player_list_score_width(entries, glyphs);
    let screen_width = surface_size.width as i32;
    let available = screen_width.saturating_sub(50).max(1) as u32;
    let slot_width = hud_player_list_slot_width(
        columns,
        max_name_width,
        width_for_score,
        available,
        list.show_heads,
    );
    let table_width = slot_width
        .saturating_mul(columns as u32)
        .saturating_add(5 * columns.saturating_sub(1) as u32);
    let header_lines = hud_player_list_component_lines(list.header.as_deref(), available, glyphs);
    let footer_lines = hud_player_list_component_lines(list.footer.as_deref(), available, glyphs);
    let max_line_width = header_lines
        .iter()
        .chain(&footer_lines)
        .map(|line| hud_font_runs_width(line, glyphs).unwrap_or(0))
        .fold(table_width, u32::max);
    let left = screen_width / 2 - max_line_width as i32 / 2;
    let mut y = 10i32;
    let draw_text = |vertices: &mut Vec<HudVertex>,
                     commands: &mut Vec<HudDrawCommand<'a>>,
                     runs: &[HudStyledTextRun],
                     x: i32,
                     y: i32,
                     tint: [f32; 4]| {
        push_hud_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &HudScreenTextDraw {
                runs,
                origin: (x as f32, y as f32),
                scale: 1.0,
                tint,
                backdrop_opacity: 0.0,
            },
        );
    };

    if !header_lines.is_empty() {
        push_hud_debug_tinted_rect(
            vertices,
            commands,
            white_pixel,
            surface_size,
            left - 1,
            y - 1,
            max_line_width + 2,
            header_lines.len() as u32 * 9 + 1,
            [0.0, 0.0, 0.0, 0.5],
        );
        for line in &header_lines {
            let width = hud_font_runs_width(line, glyphs).unwrap_or(0) as i32;
            draw_text(
                vertices,
                commands,
                line,
                screen_width / 2 - width / 2,
                y,
                HUD_TINT_WHITE,
            );
            y += 9;
        }
        y += 1;
    }

    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        left - 1,
        y - 1,
        max_line_width + 2,
        rows as u32 * 9 + 1,
        [0.0, 0.0, 0.0, 0.5],
    );
    let table_left = screen_width / 2 - table_width as i32 / 2;
    for (index, entry) in entries.iter().enumerate() {
        let column = index / rows;
        let row = index % rows;
        let x = table_left + column as i32 * (slot_width as i32 + 5);
        let row_y = y + row as i32 * 9;
        push_hud_debug_tinted_rect(
            vertices,
            commands,
            white_pixel,
            surface_size,
            x,
            row_y,
            slot_width,
            8,
            [1.0, 1.0, 1.0, 32.0 / 255.0],
        );
        if list.show_heads {
            if let Some(sprite) = renderer.hud_player_face_with_hat(entry.skin, entry.show_hat) {
                push_hud_draw_with_uv(
                    vertices,
                    commands,
                    sprite,
                    surface_size,
                    absolute_hud_rect(x as f32, row_y as f32, 8, 8),
                    hud_player_list_face_uv(entry.upside_down),
                );
            }
        }
        let name_x = hud_player_list_name_x(x, list.show_heads);
        draw_text(
            vertices,
            commands,
            &entry.name_runs,
            name_x,
            row_y,
            if entry.spectator {
                [1.0, 1.0, 1.0, 144.0 / 255.0]
            } else {
                HUD_TINT_WHITE
            },
        );

        if !entry.spectator {
            let score_left = name_x + max_name_width as i32 + 1;
            let score_right = score_left + width_for_score as i32;
            if score_right - score_left > 5 {
                match entry.score.as_ref() {
                    Some(HudPlayerListScore::Integer(score_runs)) => {
                        let score_width =
                            hud_font_runs_width(score_runs, glyphs).unwrap_or(0) as i32;
                        draw_text(
                            vertices,
                            commands,
                            score_runs,
                            score_right - score_width,
                            row_y,
                            HUD_TINT_WHITE,
                        );
                    }
                    Some(HudPlayerListScore::Hearts {
                        value,
                        hp_text,
                        compact_text,
                    }) => {
                        push_hud_player_list_hearts(
                            vertices,
                            commands,
                            white_pixel,
                            font_atlas,
                            glyphs,
                            obfuscated_pool,
                            frame_index,
                            surface_size,
                            score_left,
                            score_right,
                            row_y,
                            *value,
                            hp_text,
                            compact_text,
                            runtime
                                .health_state(entry.profile_id)
                                .unwrap_or_else(|| HudPlayerListHealthState::new(*value)),
                            list.tick_count,
                            heart_sprites,
                        );
                    }
                    None => {}
                }
            }
        }

        if let Some(sprite) = renderer.hud_player_list_ping_sprite(entry.latency) {
            push_hud_draw(
                vertices,
                commands,
                sprite,
                surface_size,
                absolute_hud_rect(
                    hud_player_list_ping_x(x, slot_width) as f32,
                    row_y as f32,
                    10,
                    8,
                ),
            );
        }
    }

    if !footer_lines.is_empty() {
        y += rows as i32 * 9 + 1;
        push_hud_debug_tinted_rect(
            vertices,
            commands,
            white_pixel,
            surface_size,
            left - 1,
            y - 1,
            max_line_width + 2,
            footer_lines.len() as u32 * 9 + 1,
            [0.0, 0.0, 0.0, 0.5],
        );
        for line in &footer_lines {
            let width = hud_font_runs_width(line, glyphs).unwrap_or(0) as i32;
            draw_text(
                vertices,
                commands,
                line,
                screen_width / 2 - width / 2,
                y,
                HUD_TINT_WHITE,
            );
            y += 9;
        }
    }
}

fn hud_player_list_component_lines(
    runs: Option<&[HudStyledTextRun]>,
    available: u32,
    glyphs: &HudFontGlyphMap,
) -> Vec<Vec<HudStyledTextRun>> {
    runs.map(|runs| hud_wrap_chat_runs(runs, available, glyphs))
        .unwrap_or_default()
}

fn hud_player_list_score_width(entries: &[HudPlayerListEntry], glyphs: &HudFontGlyphMap) -> u32 {
    if entries
        .iter()
        .any(|entry| matches!(entry.score, Some(HudPlayerListScore::Hearts { .. })))
    {
        return 90;
    }
    let spacer_width = hud_plain_text_width(" ", glyphs);
    entries
        .iter()
        .filter_map(|entry| match entry.score.as_ref() {
            Some(HudPlayerListScore::Integer(runs)) => hud_font_runs_width(runs, glyphs),
            _ => None,
        })
        .map(|width| spacer_width.saturating_add(width))
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudPlayerListHeartLayout {
    full_hearts: usize,
    hearts_to_render: usize,
    width_per_heart: i32,
    compressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudPlayerListHeartLayer {
    Container { blinking: bool },
    DisplayedFull,
    DisplayedHalf,
    ActualFull { absorbing: bool },
    ActualHalf { absorbing: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudPlayerListHeartDraw {
    heart: usize,
    layer: HudPlayerListHeartLayer,
}

fn hud_player_list_heart_layout(
    score: i32,
    displayed_score: i32,
    left: i32,
    right: i32,
) -> Option<HudPlayerListHeartLayout> {
    let greatest_score = score.max(displayed_score);
    if greatest_score <= 0 {
        return None;
    }
    let full_hearts = usize::try_from((i64::from(greatest_score) + 1) / 2).ok()?;
    let hearts_to_render =
        usize::try_from(i64::from(score.max(displayed_score).max(20)) / 2).ok()?;
    let width_per_heart =
        (((right - left - 4) as f32 / hearts_to_render as f32).min(9.0)).floor() as i32;
    Some(HudPlayerListHeartLayout {
        full_hearts,
        hearts_to_render,
        width_per_heart,
        compressed: width_per_heart <= 3,
    })
}

fn hud_player_list_heart_draws(
    layout: HudPlayerListHeartLayout,
    score: i32,
    displayed_score: i32,
    blinking: bool,
) -> Vec<HudPlayerListHeartDraw> {
    let mut draws = Vec::new();
    for heart in layout.full_hearts..layout.hearts_to_render {
        draws.push(HudPlayerListHeartDraw {
            heart,
            layer: HudPlayerListHeartLayer::Container { blinking },
        });
    }
    for heart in 0..layout.full_hearts {
        draws.push(HudPlayerListHeartDraw {
            heart,
            layer: HudPlayerListHeartLayer::Container { blinking },
        });
        if blinking && heart as i64 * 2 + 1 < i64::from(displayed_score) {
            draws.push(HudPlayerListHeartDraw {
                heart,
                layer: HudPlayerListHeartLayer::DisplayedFull,
            });
        }
        if blinking && heart as i64 * 2 + 1 == i64::from(displayed_score) {
            draws.push(HudPlayerListHeartDraw {
                heart,
                layer: HudPlayerListHeartLayer::DisplayedHalf,
            });
        }
        if heart as i64 * 2 + 1 < i64::from(score) {
            draws.push(HudPlayerListHeartDraw {
                heart,
                layer: HudPlayerListHeartLayer::ActualFull {
                    absorbing: heart >= 10,
                },
            });
        }
        if heart as i64 * 2 + 1 == i64::from(score) {
            draws.push(HudPlayerListHeartDraw {
                heart,
                layer: HudPlayerListHeartLayer::ActualHalf {
                    absorbing: heart >= 10,
                },
            });
        }
    }
    draws
}

#[allow(clippy::too_many_arguments)]
fn push_hud_player_list_hearts<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    left: i32,
    right: i32,
    y: i32,
    score: i32,
    hp_text: &str,
    compact_text: &str,
    health: HudPlayerListHealthState,
    tick: u64,
    sprites: HudPlayerListHeartSprites<'a>,
) {
    let Some(layout) = hud_player_list_heart_layout(score, health.displayed_value, left, right)
    else {
        return;
    };
    if layout.compressed || !sprites.complete() {
        let pct = (score as f32 / 20.0).clamp(0.0, 1.0);
        let red = ((1.0 - pct) * 255.0) as u8;
        let green = (pct * 255.0) as u8;
        let tint = [f32::from(red) / 255.0, f32::from(green) / 255.0, 0.0, 1.0];
        let hp_runs = [HudStyledTextRun::plain(hp_text)];
        let compact_runs = [HudStyledTextRun::plain(compact_text)];
        let (runs, width) = {
            let hp_width = hud_font_runs_width(&hp_runs, glyphs).unwrap_or(0) as i32;
            if right - hp_width >= left {
                (&hp_runs[..], hp_width)
            } else {
                let width = hud_font_runs_width(&compact_runs, glyphs).unwrap_or(0) as i32;
                (&compact_runs[..], width)
            }
        };
        push_hud_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &HudScreenTextDraw {
                runs,
                origin: (((right + left - width) / 2) as f32, y as f32),
                scale: 1.0,
                tint,
                backdrop_opacity: 0.0,
            },
        );
        return;
    }

    let blinking = health.is_blinking(tick);
    for draw in hud_player_list_heart_draws(layout, score, health.displayed_value, blinking) {
        let rect = absolute_hud_rect(
            (left + draw.heart as i32 * layout.width_per_heart) as f32,
            y as f32,
            9,
            9,
        );
        let sprite = match draw.layer {
            HudPlayerListHeartLayer::Container { blinking: true } => {
                sprites.container_blinking.expect("complete sprite set")
            }
            HudPlayerListHeartLayer::Container { blinking: false } => {
                sprites.container.expect("complete sprite set")
            }
            HudPlayerListHeartLayer::DisplayedFull => {
                sprites.full_blinking.expect("complete sprite set")
            }
            HudPlayerListHeartLayer::DisplayedHalf => {
                sprites.half_blinking.expect("complete sprite set")
            }
            HudPlayerListHeartLayer::ActualFull { absorbing: true } => sprites
                .absorbing_full_blinking
                .expect("complete sprite set"),
            HudPlayerListHeartLayer::ActualHalf { absorbing: true } => sprites
                .absorbing_half_blinking
                .expect("complete sprite set"),
            HudPlayerListHeartLayer::ActualFull { absorbing: false } => {
                sprites.full.expect("complete sprite set")
            }
            HudPlayerListHeartLayer::ActualHalf { absorbing: false } => {
                sprites.half.expect("complete sprite set")
            }
        };
        push_hud_draw(vertices, commands, sprite, surface_size, rect);
    }
}

fn hud_player_list_grid(slots: usize) -> (usize, usize) {
    let mut columns = 1usize;
    let mut rows = slots;
    while rows > 20 {
        columns += 1;
        rows = slots.div_ceil(columns);
    }
    (columns, rows)
}

fn hud_player_list_slot_width(
    columns: usize,
    max_name_width: u32,
    width_for_score: u32,
    available: u32,
    show_heads: bool,
) -> u32 {
    let head_width = if show_heads { 9 } else { 0 };
    ((columns as u32)
        .saturating_mul(
            max_name_width
                .saturating_add(head_width)
                .saturating_add(width_for_score)
                .saturating_add(13),
        )
        .min(available)
        / columns as u32)
        .max(13 + head_width)
}

fn hud_player_list_name_x(slot_x: i32, show_heads: bool) -> i32 {
    slot_x + if show_heads { 9 } else { 0 }
}

fn hud_player_list_face_uv(upside_down: bool) -> HudUvRect {
    if upside_down {
        HudUvRect {
            min: [0.0, 1.0],
            max: [1.0, 0.0],
        }
    } else {
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }
    }
}

fn hud_player_list_ping_x(slot_x: i32, slot_width: u32) -> i32 {
    slot_x + slot_width as i32 - 11
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{HudAsciiGlyph, HudTextStyle};
    use crate::EntityDefaultPlayerSkin;

    fn test_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in " abcdefghijklmnopqrstuvwxyz0123456789".chars() {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance: 6,
                    width: 5,
                    height: 8,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        glyphs
    }

    fn hearts_entry(profile_id: u128, value: i32) -> HudPlayerListEntry {
        HudPlayerListEntry {
            profile_id,
            name_runs: vec![HudStyledTextRun::plain("player")],
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            show_hat: true,
            upside_down: false,
            latency: 0,
            spectator: false,
            score: Some(HudPlayerListScore::Hearts {
                value,
                hp_text: format!("{}hp", value as f32 / 2.0),
                compact_text: (value as f32 / 2.0).to_string(),
            }),
        }
    }

    #[test]
    fn grid_and_ping_thresholds_match_vanilla() {
        assert_eq!(hud_player_list_grid(1), (1, 1));
        assert_eq!(hud_player_list_grid(20), (1, 20));
        assert_eq!(hud_player_list_grid(21), (2, 11));
        assert_eq!(hud_player_list_grid(80), (4, 20));
        assert_eq!(
            [-1, 0, 149, 150, 299, 300, 599, 600, 999, 1000]
                .map(HudPlayerListPingSprite::from_latency),
            [
                HudPlayerListPingSprite::Unknown,
                HudPlayerListPingSprite::Five,
                HudPlayerListPingSprite::Five,
                HudPlayerListPingSprite::Four,
                HudPlayerListPingSprite::Four,
                HudPlayerListPingSprite::Three,
                HudPlayerListPingSprite::Three,
                HudPlayerListPingSprite::Two,
                HudPlayerListPingSprite::Two,
                HudPlayerListPingSprite::One,
            ]
        );
        assert_eq!(hud_player_list_ping_x(17, 52), 58);
        assert_eq!(
            HudPlayerListPingSprite::ALL.map(HudPlayerListPingSprite::sprite_path),
            [
                "icon/ping_unknown",
                "icon/ping_1",
                "icon/ping_2",
                "icon/ping_3",
                "icon/ping_4",
                "icon/ping_5",
            ]
        );
    }

    #[test]
    fn encrypted_player_head_layout_reserves_nine_pixels_and_flips_v_only() {
        assert_eq!(hud_player_list_slot_width(1, 30, 0, 270, false), 43);
        assert_eq!(hud_player_list_slot_width(1, 30, 0, 270, true), 52);
        assert_eq!(hud_player_list_name_x(17, false), 17);
        assert_eq!(hud_player_list_name_x(17, true), 26);
        assert_eq!(
            hud_player_list_face_uv(false),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            }
        );
        assert_eq!(
            hud_player_list_face_uv(true),
            HudUvRect {
                min: [0.0, 1.0],
                max: [1.0, 0.0],
            }
        );
    }

    #[test]
    fn score_width_reserves_spacer_or_ninety_heart_pixels() {
        let glyphs = test_glyphs();
        let mut entry = HudPlayerListEntry {
            profile_id: 1,
            name_runs: Vec::new(),
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            show_hat: true,
            upside_down: false,
            latency: 0,
            spectator: false,
            score: Some(HudPlayerListScore::Integer(vec![HudStyledTextRun::plain(
                "12",
            )])),
        };
        assert_eq!(hud_player_list_score_width(&[entry.clone()], &glyphs), 18);
        entry.score = Some(HudPlayerListScore::Integer(vec![HudStyledTextRun {
            text: String::new(),
            style: HudTextStyle::default(),
            color: None,
        }]));
        assert_eq!(hud_player_list_score_width(&[entry.clone()], &glyphs), 0);
        entry.score = Some(HudPlayerListScore::Hearts {
            value: 20,
            hp_text: "10.0hp".to_string(),
            compact_text: "10.0".to_string(),
        });
        assert_eq!(hud_player_list_score_width(&[entry], &glyphs), 90);
    }

    #[test]
    fn header_footer_wrapping_preserves_style_across_vanilla_width_breaks() {
        let glyphs = test_glyphs();
        let runs = vec![
            HudStyledTextRun {
                text: "bold ".to_string(),
                style: HudTextStyle {
                    bold: true,
                    ..HudTextStyle::default()
                },
                color: Some(0xFF_55_55),
            },
            HudStyledTextRun {
                text: "plain".to_string(),
                style: HudTextStyle::default(),
                color: Some(0x55_FF_55),
            },
        ];

        let lines = hud_player_list_component_lines(Some(&runs), 36, &glyphs);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][0].text, "bold");
        assert!(lines[0][0].style.bold);
        assert_eq!(lines[0][0].color, Some(0xFF_55_55));
        assert_eq!(lines[1][0].text, "plain");
        assert!(!lines[1][0].style.bold);
        assert_eq!(lines[1][0].color, Some(0x55_FF_55));
        assert!(hud_player_list_component_lines(None, 36, &glyphs).is_empty());
    }

    #[test]
    fn heart_layout_uses_lagging_max_and_compresses_dense_scores() {
        assert_eq!(
            hud_player_list_heart_layout(18, 20, 0, 90),
            Some(HudPlayerListHeartLayout {
                full_hearts: 10,
                hearts_to_render: 10,
                width_per_heart: 8,
                compressed: false,
            })
        );
        assert_eq!(
            hud_player_list_heart_layout(100, 100, 0, 90),
            Some(HudPlayerListHeartLayout {
                full_hearts: 50,
                hearts_to_render: 50,
                width_per_heart: 1,
                compressed: true,
            })
        );
        assert_eq!(hud_player_list_heart_layout(0, 0, 0, 90), None);
    }

    #[test]
    fn heart_draw_order_layers_blink_history_and_absorption_like_vanilla() {
        let damage_layout = hud_player_list_heart_layout(16, 20, 0, 90).expect("damage layout");
        let damage_draws = hud_player_list_heart_draws(damage_layout, 16, 20, true);
        assert_eq!(
            &damage_draws[..3],
            [
                HudPlayerListHeartDraw {
                    heart: 0,
                    layer: HudPlayerListHeartLayer::Container { blinking: true },
                },
                HudPlayerListHeartDraw {
                    heart: 0,
                    layer: HudPlayerListHeartLayer::DisplayedFull,
                },
                HudPlayerListHeartDraw {
                    heart: 0,
                    layer: HudPlayerListHeartLayer::ActualFull { absorbing: false },
                },
            ]
        );
        assert_eq!(
            damage_draws
                .iter()
                .filter(|draw| matches!(draw.layer, HudPlayerListHeartLayer::DisplayedFull))
                .count(),
            10
        );
        assert_eq!(
            damage_draws
                .iter()
                .filter(|draw| matches!(draw.layer, HudPlayerListHeartLayer::ActualFull { .. }))
                .count(),
            8
        );

        let absorption_layout =
            hud_player_list_heart_layout(23, 23, 0, 90).expect("absorption layout");
        let absorption_draws = hud_player_list_heart_draws(absorption_layout, 23, 23, false);
        assert!(absorption_draws.contains(&HudPlayerListHeartDraw {
            heart: 10,
            layer: HudPlayerListHeartLayer::ActualFull { absorbing: true },
        }));
        assert_eq!(
            absorption_draws.last(),
            Some(&HudPlayerListHeartDraw {
                heart: 11,
                layer: HudPlayerListHeartLayer::ActualHalf { absorbing: true },
            })
        );
    }

    #[test]
    fn health_state_matches_damage_heal_blink_and_delayed_catchup() {
        let mut health = HudPlayerListHealthState::new(20);
        health.update(16, 100);
        assert_eq!(health.displayed_value, 20);
        assert_eq!(health.blink_until_tick, 120);
        assert!(!health.is_blinking(100));
        assert!(health.is_blinking(103));
        health.update(16, 120);
        assert_eq!(health.displayed_value, 20);
        health.update(16, 121);
        assert_eq!(health.displayed_value, 16);

        health.update(18, 130);
        assert_eq!(health.blink_until_tick, 140);
        assert_eq!(health.displayed_value, 16);
    }

    #[test]
    fn runtime_clears_on_visibility_edges_and_prunes_absent_profiles() {
        let mut runtime = HudPlayerListRuntime::default();
        let list = HudPlayerList {
            entries: vec![hearts_entry(1, 20), hearts_entry(2, 18)],
            tick_count: 10,
            ..HudPlayerList::default()
        };
        runtime.synchronize(Some(&list));
        assert_eq!(runtime.health_states.len(), 2);

        let shorter = HudPlayerList {
            entries: vec![hearts_entry(2, 16)],
            tick_count: 11,
            ..HudPlayerList::default()
        };
        runtime.synchronize(Some(&shorter));
        assert_eq!(runtime.health_states.len(), 1);
        assert_eq!(runtime.health_state(2).unwrap().displayed_value, 18);

        let unchanged_payload_later_tick = HudPlayerList {
            tick_count: 32,
            ..shorter.clone()
        };
        runtime.synchronize(Some(&unchanged_payload_later_tick));
        assert_eq!(runtime.health_state(2).unwrap().displayed_value, 16);

        runtime.synchronize(None);
        assert!(runtime.health_states.is_empty());
        runtime.synchronize(Some(&shorter));
        assert_eq!(runtime.health_state(2).unwrap().displayed_value, 16);
    }
}
