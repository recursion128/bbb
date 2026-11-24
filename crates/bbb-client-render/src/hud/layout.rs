use bbb_render_types::{HudGlyphQuad, HudObfuscatedRandom, HudTextStyle};
use glam::{Mat4, Vec3};
use winit::dpi::PhysicalSize;

use super::{
    HudDigitGlyph, HudHeartKind, HudNineSliceScaling, HudPlayerHealth, HudUvRect, HudVertex,
    HUD_HOTBAR_SLOTS,
};

const HUD_HOTBAR_WIDTH: u32 = 182;
const HUD_HOTBAR_HEIGHT: u32 = 22;
const HUD_HOTBAR_SLOT_SPACING: f32 = 20.0;
const HUD_HOTBAR_ITEM_SIZE: u32 = 16;
const HUD_HOTBAR_OFFHAND_WIDTH: u32 = 29;
const HUD_HOTBAR_ATTACK_INDICATOR_SIZE: u32 = 18;
pub(super) const HUD_EXPERIENCE_BAR_WIDTH: u32 = 182;
const HUD_EXPERIENCE_BAR_HEIGHT: u32 = 5;
const HUD_EXPERIENCE_MARGIN_BOTTOM: f32 = 24.0;
const HUD_LOCATOR_MARKER_WIDTH: u32 = 9;
const HUD_LOCATOR_MARKER_HEIGHT: u32 = 9;
const HUD_LOCATOR_ARROW_WIDTH: u32 = 7;
const HUD_LOCATOR_ARROW_HEIGHT: u32 = 5;
const HUD_LOCATOR_ANGLE_PIXEL_SPAN: f32 = 173.0;
const HUD_LOCATOR_VISIBLE_DEGREES: f32 = 60.0;
const HUD_HEART_SPACING: f32 = 8.0;
pub(super) const HUD_FOOD_ICONS_PER_ROW: u32 = 10;
const HUD_FOOD_SPACING: f32 = 8.0;
pub(super) const HUD_ARMOR_ICONS_PER_ROW: u32 = 10;
const HUD_ARMOR_SPACING: f32 = 8.0;
/// Vanilla `Gui.extractArmor` seats the armor row at
/// `yLineArmor = yLineBase - (numHealthRows - 1) * healthRowHeight - 10`
/// (Gui.java:801): this is the fixed 10px term above the `yLineBase` heart
/// baseline (`surface_height - 39`); the `(numHealthRows - 1) * healthRowHeight`
/// term is added by `armor_hud_rect` from the projected health-row geometry.
const HUD_ARMOR_ROW_Y_OFFSET: f32 = 10.0;
/// `healthRowHeight` for the single-row default (`max(10 - (1 - 2), 3) = 11`),
/// used for the armor row when no player health is projected. The armor
/// `(numHealthRows - 1) * healthRowHeight` term is zero for one row, so this
/// only documents the single-row case.
pub(super) const HUD_SINGLE_HEALTH_ROW_HEIGHT: i32 = 11;
/// Vanilla `Gui.NUM_AIR_BUBBLES` (Gui.java:126): one row of 10 bubbles.
pub(super) const HUD_AIR_BUBBLES_PER_ROW: u32 = 10;
/// Vanilla `Gui.AIR_BUBBLE_SEPERATION` (Gui.java:128): the 8px bubble stride
/// (`airBubbleXPos = xRight - (airBubble - 1) * 8 - 9`, Gui.java:903).
const HUD_AIR_BUBBLE_SPACING: f32 = 8.0;
/// Vanilla vehicle-heart stride (`xo = xRight - i * 8 - 9`, Gui.java:990) and
/// row count cap (`getVehicleMaxHearts` caps at 30 hearts → 3 rows,
/// Gui.java:729-731).
const HUD_VEHICLE_HEART_SPACING: f32 = 8.0;
pub(super) const HUD_VEHICLE_HEARTS_PER_ROW: u32 = 10;
const HUD_VEHICLE_MAX_HEARTS: i32 = 30;
pub(super) const HUD_INVENTORY_ITEM_SIZE: u32 = 16;
const HUD_INVENTORY_SLOT_HIGHLIGHT_SIZE: u32 = 24;
const HUD_INVENTORY_SLOT_HIGHLIGHT_OFFSET: f32 = -4.0;
const HUD_ITEM_DURABILITY_BAR_X_OFFSET: f32 = 2.0;
const HUD_ITEM_DURABILITY_BAR_Y_OFFSET: f32 = 13.0;
const HUD_TOOLTIP_MOUSE_X_OFFSET: f32 = 12.0;
const HUD_TOOLTIP_MOUSE_Y_OFFSET: f32 = -12.0;
const HUD_TOOLTIP_RIGHT_FALLBACK_OFFSET: f32 = 24.0;
const HUD_TOOLTIP_RIGHT_MARGIN: f32 = 4.0;
const HUD_TOOLTIP_BOTTOM_PADDING: f32 = 3.0;
const HUD_TOOLTIP_BACKGROUND_INSET: f32 = 12.0;
const HUD_TOOLTIP_BACKGROUND_PADDING: u32 = 24;
const HUD_TOOLTIP_LINE_HEIGHT: u32 = 10;
const HUD_TOOLTIP_FIRST_LINE_EXTRA_GAP: u32 = 2;
/// Vanilla `Gui.extractOverlayMessage` pose: `translate(guiWidth / 2,
/// guiHeight - 68)` (Gui.java:321), text at `(-width / 2, -4)` (:330).
const HUD_OVERLAY_MESSAGE_BOTTOM_OFFSET: i32 = 68;
const HUD_OVERLAY_MESSAGE_TEXT_Y: i32 = -4;
/// Vanilla `Gui.extractSelectedItemName` (Gui.java:1090-1101): `guiHeight - 59`,
/// pushed down 14 more when `gameMode.canHurtPlayer()` is false.
const HUD_SELECTED_ITEM_NAME_BOTTOM_OFFSET: i32 = 59;
const HUD_SELECTED_ITEM_NAME_NO_HEALTH_OFFSET: i32 = 14;
/// Vanilla `Gui.extractTitle` pose: `translate(guiWidth / 2, guiHeight / 2)`
/// title `scale(4, 4)` at `(-width / 2, -10)`,
/// subtitle `scale(2, 2)` at `(-width / 2, 5)`.
pub(super) const HUD_TITLE_TEXT_SCALE: f32 = 4.0;
const HUD_TITLE_TEXT_Y: i32 = -10;
pub(super) const HUD_SUBTITLE_TEXT_SCALE: f32 = 2.0;
const HUD_SUBTITLE_TEXT_Y: i32 = 5;
/// Vanilla `BossHealthOverlay` geometry: 182x5 sheets
/// (`BAR_WIDTH`/`BAR_HEIGHT`, BossHealthOverlay.java:18-19) at
/// `x = guiWidth / 2 - 91` (:66), stacked from `y = 12` stepping
/// `10 + 9` (bar gap + font line height) per drawn bar (:63,74), and the
/// name line at `y - 9` (:72).
pub(super) const HUD_BOSS_BAR_WIDTH: u32 = 182;
const HUD_BOSS_BAR_HEIGHT: u32 = 5;
const HUD_BOSS_BAR_HALF_WIDTH: i32 = 91;
const HUD_BOSS_BAR_TOP_Y: i32 = 12;
const HUD_BOSS_BAR_ROW_ADVANCE: i32 = 10 + 9;
const HUD_BOSS_BAR_NAME_Y_OFFSET: i32 = 9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudRect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudIconFill {
    Empty,
    Half,
    Full,
}

pub(super) fn centered_hud_rect(
    surface_size: PhysicalSize<u32>,
    width: u32,
    height: u32,
) -> HudRect {
    HudRect {
        x: (surface_size.width.max(1) as f32 - width as f32) * 0.5,
        y: (surface_size.height.max(1) as f32 - height as f32) * 0.5,
        width,
        height,
    }
}

pub(super) fn absolute_hud_rect(x: f32, y: f32, width: u32, height: u32) -> HudRect {
    HudRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudCrosshairAttackIndicatorLayout {
    pub(super) full: HudRect,
    pub(super) background: HudRect,
    pub(super) progress: HudRect,
    pub(super) progress_uv: HudUvRect,
}

/// Pixel placement from Vanilla `Gui.extractCrosshair`.
pub(super) fn hud_crosshair_attack_indicator_layout(
    surface_size: PhysicalSize<u32>,
    strength_scale: f32,
) -> HudCrosshairAttackIndicatorLayout {
    let x = (surface_size.width.max(1) / 2) as f32 - 8.0;
    let y = (surface_size.height.max(1) / 2) as f32 + 9.0;
    let strength_scale = if strength_scale.is_finite() {
        strength_scale.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let progress_width = ((strength_scale * 17.0) as u32).min(16);
    HudCrosshairAttackIndicatorLayout {
        full: absolute_hud_rect(x, y, 16, 16),
        background: absolute_hud_rect(x, y, 16, 4),
        progress: absolute_hud_rect(x, y, progress_width, 4),
        progress_uv: HudUvRect {
            min: [0.0, 0.0],
            max: [progress_width as f32 / 16.0, 1.0],
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudSpyglassScopeLayout {
    pub(super) texture: HudRect,
    /// Vanilla submission order: bottom, top, left, right.
    pub(super) masks: [HudRect; 4],
}

pub(super) fn hud_spyglass_scope_layout(
    surface_size: PhysicalSize<u32>,
    scale: f32,
) -> HudSpyglassScopeLayout {
    let gui_width = surface_size.width.max(1);
    let gui_height = surface_size.height.max(1);
    let source_size = gui_width.min(gui_height);
    let scaled_size = ((f64::from(source_size) * f64::from(scale))
        .floor()
        .clamp(0.0, f64::from(u32::MAX))) as u32;
    let left = (i64::from(gui_width) - i64::from(scaled_size)) / 2;
    let top = (i64::from(gui_height) - i64::from(scaled_size)) / 2;
    let right = left + i64::from(scaled_size);
    let bottom = top + i64::from(scaled_size);

    HudSpyglassScopeLayout {
        texture: absolute_hud_rect(left as f32, top as f32, scaled_size, scaled_size),
        masks: [
            vanilla_fill_hud_rect(0, bottom, i64::from(gui_width), i64::from(gui_height)),
            vanilla_fill_hud_rect(0, 0, i64::from(gui_width), top),
            vanilla_fill_hud_rect(0, top, left, bottom),
            vanilla_fill_hud_rect(right, top, i64::from(gui_width), bottom),
        ],
    }
}

fn vanilla_fill_hud_rect(x0: i64, y0: i64, x1: i64, y1: i64) -> HudRect {
    absolute_hud_rect(
        x0.min(x1) as f32,
        y0.min(y1) as f32,
        u32::try_from(x0.abs_diff(x1)).unwrap_or(u32::MAX),
        u32::try_from(y0.abs_diff(y1)).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
pub(super) fn hud_rect_bounds(rect: HudRect) -> (f32, f32, f32, f32) {
    (
        rect.x,
        rect.y,
        rect.x + rect.width as f32,
        rect.y + rect.height as f32,
    )
}

pub(super) fn hud_rect_intersection_uv_span(
    rect: HudRect,
    scissor: HudRect,
) -> Option<(HudRect, [f32; 2], [f32; 2])> {
    let left = rect.x.max(scissor.x);
    let top = rect.y.max(scissor.y);
    let right = (rect.x + rect.width as f32).min(scissor.x + scissor.width as f32);
    let bottom = (rect.y + rect.height as f32).min(scissor.y + scissor.height as f32);
    if left >= right || top >= bottom {
        return None;
    }

    let visible_width = (right - left).round() as u32;
    let visible_height = (bottom - top).round() as u32;
    if visible_width == 0 || visible_height == 0 {
        return None;
    }

    let source_width = rect.width.max(1) as f32;
    let source_height = rect.height.max(1) as f32;
    Some((
        HudRect {
            x: left,
            y: top,
            width: visible_width,
            height: visible_height,
        },
        [
            (left - rect.x) / source_width,
            (top - rect.y) / source_height,
        ],
        [
            (right - rect.x) / source_width,
            (bottom - rect.y) / source_height,
        ],
    ))
}

pub(super) fn hotbar_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_HOTBAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_HOTBAR_HEIGHT as f32,
        width,
        height,
    }
}

pub(super) fn experience_bar_hud_rect(
    surface_size: PhysicalSize<u32>,
    width: u32,
    height: u32,
) -> HudRect {
    let surface_height = surface_size.height.max(1) as f32;
    let left = (i64::from(surface_size.width.max(1)) - i64::from(HUD_EXPERIENCE_BAR_WIDTH)) / 2;
    HudRect {
        x: left as f32,
        y: surface_height - HUD_EXPERIENCE_MARGIN_BOTTOM - HUD_EXPERIENCE_BAR_HEIGHT as f32,
        width,
        height,
    }
}

pub(super) fn hud_contextual_bar_progress_width(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 183.0).floor() as u32).min(HUD_EXPERIENCE_BAR_WIDTH)
}

/// Vanilla `LocatorBarRenderer` marker placement: the center reference is
/// `ceil((guiWidth - 9) / 2)`, the accepted ±60° interval spans 173 pixels,
/// and the 9x9 dot starts two pixels above the 182x5 contextual bar.
pub(super) fn hud_locator_marker_rect(
    surface_size: PhysicalSize<u32>,
    angle_degrees: f32,
) -> HudRect {
    let bar = experience_bar_hud_rect(
        surface_size,
        HUD_EXPERIENCE_BAR_WIDTH,
        HUD_EXPERIENCE_BAR_HEIGHT,
    );
    let screen_middle =
        ((surface_size.width.max(1) as f32 - HUD_LOCATOR_MARKER_WIDTH as f32) * 0.5).ceil();
    let pixel_offset = (angle_degrees * HUD_LOCATOR_ANGLE_PIXEL_SPAN
        / (2.0 * HUD_LOCATOR_VISIBLE_DEGREES))
        .floor();
    HudRect {
        x: screen_middle + pixel_offset,
        y: bar.y - 2.0,
        width: HUD_LOCATOR_MARKER_WIDTH,
        height: HUD_LOCATOR_MARKER_HEIGHT,
    }
}

/// Vanilla pitch cue placement relative to a locator dot. `up == true`
/// seats the 7x5 arrow six pixels above the contextual bar; down seats it six
/// pixels below.
pub(super) fn hud_locator_arrow_rect(
    surface_size: PhysicalSize<u32>,
    angle_degrees: f32,
    up: bool,
) -> HudRect {
    let marker = hud_locator_marker_rect(surface_size, angle_degrees);
    let bar = experience_bar_hud_rect(
        surface_size,
        HUD_EXPERIENCE_BAR_WIDTH,
        HUD_EXPERIENCE_BAR_HEIGHT,
    );
    HudRect {
        x: marker.x + 1.0,
        y: if up { bar.y - 6.0 } else { bar.y + 6.0 },
        width: HUD_LOCATOR_ARROW_WIDTH,
        height: HUD_LOCATOR_ARROW_HEIGHT,
    }
}

/// One boss-bar sprite row: `x = guiWidth / 2 - 91` (Java int division,
/// BossHealthOverlay), `width` is the full 182px sheet for
/// backgrounds or the discrete fill width for progress layers, height 5.
pub(super) fn boss_bar_hud_rect(surface_size: PhysicalSize<u32>, y: i32, width: u32) -> HudRect {
    let center_x = (surface_size.width.max(1) / 2) as i32;
    HudRect {
        x: (center_x - HUD_BOSS_BAR_HALF_WIDTH) as f32,
        y: y as f32,
        width,
        height: HUD_BOSS_BAR_HEIGHT,
    }
}

/// Bar-row y offsets that survive vanilla's stacking cutoff
/// (`BossHealthOverlay.extractRenderState`):
/// rows start at 12 and advance `10 + 9` after each drawn bar; the loop
/// draws first and checks after, so the first bar always renders and the
/// remainder is dropped once the accumulated offset reaches `guiHeight / 3`
/// (Java int division).
pub(super) fn hud_boss_bar_rows(surface_size: PhysicalSize<u32>, bar_count: usize) -> Vec<i32> {
    let cutoff = (surface_size.height.max(1) / 3) as i32;
    let mut rows = Vec::new();
    let mut y = HUD_BOSS_BAR_TOP_Y;
    for _ in 0..bar_count {
        rows.push(y);
        y += HUD_BOSS_BAR_ROW_ADVANCE;
        if y >= cutoff {
            break;
        }
    }
    rows
}

/// Vanilla `Mth.lerpDiscrete(progress, 0, 182)` (BossHealthOverlay.java:86,
/// Mth): `floor(progress * 181) + (progress > 0 ? 1: 0)`, so
/// any positive progress fills at least one pixel and 1.0 fills all 182.
/// The clamp keeps direct out-of-range inputs inside the sheet (the setter
/// sanitizes upstream; vanilla trusts the packet float verbatim).
pub(super) fn hud_boss_bar_progress_width(progress: f32) -> u32 {
    let width =
        (progress * (HUD_BOSS_BAR_WIDTH - 1) as f32).floor() as i32 + i32::from(progress > 0.0);
    width.clamp(0, HUD_BOSS_BAR_WIDTH as i32) as u32
}

/// Left-anchored crop UV of a boss-bar layer drawn at `width` of the nominal
/// 182px sheet: vanilla `blitSprite(sprite, 182, 5, 0, 0, x, y, width, 5)`
/// samples the `(0..width, 0..5)` sub-rect.
pub(super) fn hud_boss_bar_fill_uv(width: u32) -> HudUvRect {
    HudUvRect {
        min: [0.0, 0.0],
        max: [
            (width as f32 / HUD_BOSS_BAR_WIDTH as f32).clamp(0.0, 1.0),
            1.0,
        ],
    }
}

/// Pen origin of a bar's centered name line: vanilla draws it at
/// `(guiWidth / 2 - font.width(name) / 2, barY - 9)`
/// in opaque white with the default drop
/// shadow (`graphics.text(..., -1)`, GuiGraphicsExtractor);
/// Java int divisions throughout.
pub(super) fn hud_boss_bar_name_origin(
    surface_size: PhysicalSize<u32>,
    bar_y: i32,
    text_width: i32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as i32;
    (
        (center_x - text_width / 2) as f32,
        (bar_y - HUD_BOSS_BAR_NAME_Y_OFFSET) as f32,
    )
}

pub(super) fn hotbar_selection_hud_rect(
    surface_size: PhysicalSize<u32>,
    selected_slot: u8,
    width: u32,
    height: u32,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: hotbar.x - 1.0
            + f32::from(selected_slot.min((HUD_HOTBAR_SLOTS - 1) as u8)) * HUD_HOTBAR_SLOT_SPACING,
        y: hotbar.y - 1.0,
        width,
        height,
    }
}

pub(super) fn hotbar_item_hud_rect(surface_size: PhysicalSize<u32>, slot: usize) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: hotbar.x + 3.0 + slot.min(HUD_HOTBAR_SLOTS - 1) as f32 * HUD_HOTBAR_SLOT_SPACING,
        y: hotbar.y + 3.0,
        width: HUD_HOTBAR_ITEM_SIZE,
        height: HUD_HOTBAR_ITEM_SIZE,
    }
}

/// Vanilla `Gui.extractItemHotbar` offhand background: the
/// `hud/hotbar_offhand_left` sprite sits at `screenCenter - 91 - 29`, the right
/// one at `screenCenter + 91`, both one pixel above the hotbar strip at
/// `guiHeight - 23` and both 29x24.
/// Vanilla `Gui.extractConfusionOverlay`'s pose: the
/// full-screen quad is scaled uniformly about the screen centre, so a `scale`
/// above one pushes the quad past every edge.
pub(super) fn hud_confusion_overlay_corners(
    surface_size: PhysicalSize<u32>,
    scale: f32,
) -> [[f32; 2]; 4] {
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let scale = if scale.is_finite() {
        scale.max(0.0)
    } else {
        1.0
    };
    let half_width = width * 0.5 * scale;
    let half_height = height * 0.5 * scale;
    let (center_x, center_y) = (width * 0.5, height * 0.5);
    let (x0, x1) = (center_x - half_width, center_x + half_width);
    let (y0, y1) = (center_y - half_height, center_y + half_height);
    [[x0, y0], [x0, y1], [x1, y1], [x1, y0]]
}

/// `Gui.extractTextureOverlay` blits the whole GUI surface.
pub(super) fn full_screen_hud_rect(surface_size: PhysicalSize<u32>) -> HudRect {
    HudRect {
        x: 0.0,
        y: 0.0,
        width: surface_size.width.max(1),
        height: surface_size.height.max(1),
    }
}

pub(super) fn hotbar_offhand_hud_rect(
    surface_size: PhysicalSize<u32>,
    on_left: bool,
    width: u32,
    height: u32,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: if on_left {
            hotbar.x - HUD_HOTBAR_OFFHAND_WIDTH as f32
        } else {
            hotbar.x + HUD_HOTBAR_WIDTH as f32
        },
        y: hotbar.y - 1.0,
        width,
        height,
    }
}

/// Vanilla `Gui.extractItemHotbar` offhand slot: the item
/// draws at `screenCenter - 91 - 26` / `screenCenter + 91 + 10`, sharing the
/// hotbar slots' `guiHeight - 16 - 3` baseline.
pub(super) fn hotbar_offhand_item_hud_rect(
    surface_size: PhysicalSize<u32>,
    on_left: bool,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: if on_left {
            hotbar.x - 26.0
        } else {
            hotbar.x + HUD_HOTBAR_WIDTH as f32 + 10.0
        },
        y: hotbar.y + 3.0,
        width: HUD_HOTBAR_ITEM_SIZE,
        height: HUD_HOTBAR_ITEM_SIZE,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudHotbarAttackIndicatorLayout {
    pub(super) background: HudRect,
    pub(super) progress: HudRect,
    pub(super) progress_uv: HudUvRect,
}

/// Vanilla `Gui.extractItemHotbar`'s hotbar attack indicator progress bar
/// `(int)(scale * 19)` pixels of the 18x18 progress sprite
/// fill upward from the bottom edge, so both the source and the destination
/// start at `18 - progress`.
pub(super) fn hud_hotbar_attack_indicator_layout(
    surface_size: PhysicalSize<u32>,
    strength_scale: f32,
    main_arm_left: bool,
) -> HudHotbarAttackIndicatorLayout {
    let background = hotbar_attack_indicator_hud_rect(surface_size, main_arm_left);
    let strength_scale = if strength_scale.is_finite() {
        strength_scale.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = ((strength_scale * 19.0) as u32).min(HUD_HOTBAR_ATTACK_INDICATOR_SIZE);
    let empty = HUD_HOTBAR_ATTACK_INDICATOR_SIZE - filled;
    HudHotbarAttackIndicatorLayout {
        background,
        progress: HudRect {
            x: background.x,
            y: background.y + empty as f32,
            width: HUD_HOTBAR_ATTACK_INDICATOR_SIZE,
            height: filled,
        },
        progress_uv: HudUvRect {
            min: [0.0, empty as f32 / HUD_HOTBAR_ATTACK_INDICATOR_SIZE as f32],
            max: [1.0, 1.0],
        },
    }
}

/// Vanilla `Gui.extractItemHotbar` hotbar attack indicator
/// drawn only for `AttackIndicatorStatus.HOTBAR` while the
/// attack strength is still charging, outside the hotbar on the main-arm side
/// at `guiHeight - 20`, 18x18.
fn hotbar_attack_indicator_hud_rect(
    surface_size: PhysicalSize<u32>,
    main_arm_left: bool,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: if main_arm_left {
            hotbar.x - 22.0
        } else {
            hotbar.x + HUD_HOTBAR_WIDTH as f32 + 6.0
        },
        y: hotbar.y + 2.0,
        width: HUD_HOTBAR_ATTACK_INDICATOR_SIZE,
        height: HUD_HOTBAR_ATTACK_INDICATOR_SIZE,
    }
}

/// Vanilla `Gui.extractSlot`'s pop pose: while
/// `ItemStack.getPopTime()` is still ahead of the partial tick, the item quad
/// is squeezed about `(x + 8, y + 12)` by `1 / squeeze` horizontally and
/// `(squeeze + 1) / 2` vertically, where `squeeze = 1 + pop / 5`. Only the item
/// itself is posed — `graphics.itemDecorations` runs after `popMatrix`.
///
/// Returns the quad corners in `hud_styled_quad_vertices` order
/// (`[top_left, bottom_left, bottom_right, top_right]`), which for a
/// non-positive `pop` are exactly `rect`'s own corners.
pub(super) fn hud_item_pop_corners(rect: HudRect, pop: f32) -> [[f32; 2]; 4] {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width as f32;
    let y1 = rect.y + rect.height as f32;
    if pop <= 0.0 || !pop.is_finite() {
        return [[x0, y0], [x0, y1], [x1, y1], [x1, y0]];
    }
    let squeeze = 1.0 + pop / 5.0;
    let pivot_x = rect.x + 8.0;
    let pivot_y = rect.y + 12.0;
    let scale_x = 1.0 / squeeze;
    let scale_y = (squeeze + 1.0) / 2.0;
    let posed_x = |x: f32| pivot_x + (x - pivot_x) * scale_x;
    let posed_y = |y: f32| pivot_y + (y - pivot_y) * scale_y;
    let (px0, px1) = (posed_x(x0), posed_x(x1));
    let (py0, py1) = (posed_y(y0), posed_y(y1));
    [[px0, py0], [px0, py1], [px1, py1], [px1, py0]]
}

/// The same pop pose as [`hud_item_pop_corners`], as the model-space transform
/// a 3D block item's GUI placement composes with.
pub(super) fn hud_item_pop_placement(rect: HudRect, pop: f32) -> Mat4 {
    if pop <= 0.0 || !pop.is_finite() {
        return Mat4::IDENTITY;
    }
    let squeeze = 1.0 + pop / 5.0;
    let pivot = Vec3::new(rect.x + 8.0, rect.y + 12.0, 0.0);
    Mat4::from_translation(pivot)
        * Mat4::from_scale(Vec3::new(1.0 / squeeze, (squeeze + 1.0) / 2.0, 1.0))
        * Mat4::from_translation(-pivot)
}

/// The model→GUI-pixel placement for a 3D inventory item rendered in `rect`. Composed with the
/// item's GUI display transform and projected by the GUI ortho camera, it seats a `0..1`
/// (post-display) model in the slot's pixel rect, flipping Y to GUI space.
pub(super) fn gui_item_slot_placement(rect: HudRect) -> Mat4 {
    let width = rect.width as f32;
    let height = rect.height as f32;
    let center_x = rect.x + width / 2.0;
    let center_y = rect.y + height / 2.0;
    Mat4::from_translation(Vec3::new(center_x, center_y, 0.0))
        * Mat4::from_scale(Vec3::new(width, -height, width))
}

/// One heart sprite to draw (vanilla `Gui.extractHeart`), produced by
/// [`hud_player_heart_instances`]. `x`/`y` are the top-left HUD pixel; hearts
/// blit at the sprite's own 9x9 size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudHeartInstance {
    pub kind: HudHeartKind,
    pub half: bool,
    pub blinking: bool,
    pub x: f32,
    pub y: f32,
}

impl HudHeartInstance {
    /// This heart's draw rect at the given sprite size (9x9 for every heart).
    pub(super) fn rect(self, width: u32, height: u32) -> HudRect {
        HudRect {
            x: self.x,
            y: self.y,
            width,
            height,
        }
    }
}

/// Vanilla `Gui.extractPlayerHealth` health-row geometry from the raw player
/// state: resolves `currentHealth = ceil(health)`
/// and `maxHealth = max(attr, displayHealth, currentHealth)`, then
/// Upper bound on emitted HUD heart containers per frame (409.6 rows of 10 hearts = 8,192 HP),
/// protecting against untrusted server `SetHealth`/`SetEntityData` OOM and `i32` overflow.
pub(super) const MAX_HUD_HEART_CONTAINERS: i32 = 4_096;
const MAX_HUD_HEALTH_POINTS: f32 = (MAX_HUD_HEART_CONTAINERS * 2) as f32;

pub(super) fn clamp_hud_player_health(mut health: HudPlayerHealth) -> HudPlayerHealth {
    health.health = health.health.clamp(0.0, MAX_HUD_HEALTH_POINTS);
    health.max_health = health.max_health.clamp(0.0, MAX_HUD_HEALTH_POINTS);
    health.absorption = health.absorption.clamp(0.0, MAX_HUD_HEALTH_POINTS);
    health.display_health = health
        .display_health
        .clamp(0, MAX_HUD_HEART_CONTAINERS.saturating_mul(2));
    health
}

/// `numHealthRows = ceil((maxHealth + ceil(absorption)) / 2 / 10)` and
/// `healthRowHeight = max(10 - (numHealthRows - 2), 3)`.
pub(super) fn hud_health_row_geometry(health: HudPlayerHealth) -> (u32, i32) {
    let health = clamp_hud_player_health(health);
    let current_health = health.health.ceil().max(0.0);
    let display_health = health.display_health.max(0) as f32;
    let max_health = health.max_health.max(display_health).max(current_health);
    hud_health_rows(max_health, health.absorption)
}

/// Core of [`hud_health_row_geometry`]: `numHealthRows` /`healthRowHeight` from
/// an already-resolved `maxHealth` float and the raw absorption amount.
/// `absorption` is ceiled to `totalAbsorption` first.
pub(super) fn hud_health_rows(max_health: f32, absorption: f32) -> (u32, i32) {
    let max_health = if max_health.is_finite() {
        max_health.clamp(0.0, MAX_HUD_HEALTH_POINTS)
    } else {
        0.0
    };
    let absorption = if absorption.is_finite() {
        absorption.clamp(0.0, MAX_HUD_HEALTH_POINTS)
    } else {
        0.0
    };
    let total_absorption = absorption.ceil() as i32;
    let num_rows = ((max_health + total_absorption as f32) / 2.0 / 10.0)
        .ceil()
        .clamp(0.0, MAX_HUD_HEART_CONTAINERS as f32) as i32;
    // Vanilla `Math.max(10 - (numHealthRows - 2), 3)`.
    let row_height = 10_i32.saturating_sub(num_rows.saturating_sub(2)).max(3);
    (num_rows.max(0) as u32, row_height)
}

/// Vanilla `Gui.extractHearts`. Walks containers from
/// `healthContainerCount + absorptionContainerCount - 1` down to `0` (the
/// vanilla descending loop), emitting for each: the container with the frame
/// blink bit, a non-blinking absorption overlay, the blinking displayed-health
/// fill when active, then the non-blinking current-health fill.
/// `xLeft = guiWidth/2 - 91`, `yLineBase = guiHeight - 39`; rows stack up by
/// `healthRowHeight`. The Regeneration wave lifts one health heart
/// (`containerIndex == tickCount % ceil(maxHealth + 5)`) by 2px, and at
/// `currentHealth + absorption <= 4` every heart shakes by `nextInt(2)` from a
/// `tickCount * 312871`-seeded LCG (vanilla reseeds the identical
/// `LegacyRandomSource` at Gui, so this reproduces its sequence
/// exactly — unlike the food/air shakes whose seed is wall-clock).
pub(super) fn hud_player_heart_instances(
    surface_size: PhysicalSize<u32>,
    health: HudPlayerHealth,
) -> Vec<HudHeartInstance> {
    if !health.health.is_finite()
        || !health.max_health.is_finite()
        || !health.absorption.is_finite()
    {
        return Vec::new();
    }
    let health = clamp_hud_player_health(health);
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    let x_left = surface_width * 0.5 - 91.0;
    let y_line_base = surface_height - 39.0;

    let current_health = health.health.ceil() as i32;
    let display_health = health.display_health.max(0);
    let absorption = health.absorption.ceil() as i32;
    let max_health = health
        .max_health
        .max(display_health as f32)
        .max(current_health as f32);
    let (_, health_row_height) = hud_health_rows(max_health, health.absorption);

    let health_container_count = ((max_health / 2.0).ceil() as i32).min(MAX_HUD_HEART_CONTAINERS);
    let absorption_container_count = ((absorption as f32 / 2.0).ceil() as i32)
        .min(MAX_HUD_HEART_CONTAINERS.saturating_sub(health_container_count));
    let max_health_halves = health_container_count.saturating_mul(2);

    // Vanilla `heartOffsetIndex = tickCount % ceil(maxHealth + 5)` under
    // Regeneration, else `-1` (never matches a container index).
    let heart_offset_index = if health.regen {
        let period = (max_health + 5.0).ceil().max(1.0) as i64;
        (health.tick_count as i64 % period) as i32
    } else {
        -1
    };

    // Low-health shake: one `nextInt(2)` per container consumed in the
    // descending order, from a per-tick reseed (844-846).
    let mut shake = (current_health.saturating_add(absorption) <= 4).then(|| {
        let seed = (health.tick_count as i32).wrapping_mul(312871) as i64 as u64;
        HudObfuscatedRandom::with_seed(seed)
    });

    let max_visible_rows = (((y_line_base + 16.0).max(0.0)) / health_row_height.max(1) as f32)
        .ceil()
        .clamp(1.0, (MAX_HUD_HEART_CONTAINERS / 10) as f32) as i32
        + 1;
    let max_visible_containers = max_visible_rows
        .saturating_mul(10)
        .clamp(10, MAX_HUD_HEART_CONTAINERS);
    let total_containers = health_container_count
        .saturating_add(absorption_container_count)
        .min(max_visible_containers);
    let mut instances = Vec::new();
    for container_index in (0..total_containers).rev() {
        let row = container_index / 10;
        let column = container_index % 10;
        let xo = x_left + column as f32 * HUD_HEART_SPACING;
        let mut yo = y_line_base - row as f32 * health_row_height as f32;
        if let Some(random) = shake.as_mut() {
            yo += random.next_int_bound(2) as f32;
        }
        if container_index < health_container_count && container_index == heart_offset_index {
            yo -= 2.0;
        }

        instances.push(HudHeartInstance {
            kind: HudHeartKind::Container,
            half: false,
            blinking: health.blinking,
            x: xo,
            y: yo,
        });

        let halves = container_index.saturating_mul(2);
        if container_index >= health_container_count {
            let absorption_halves = halves.saturating_sub(max_health_halves);
            if absorption_halves < absorption {
                let kind = if health.heart_type == HudHeartKind::Withered {
                    HudHeartKind::Withered
                } else {
                    HudHeartKind::Absorbing
                };
                instances.push(HudHeartInstance {
                    kind,
                    half: absorption_halves.saturating_add(1) == absorption,
                    blinking: false,
                    x: xo,
                    y: yo,
                });
            }
        }

        if health.blinking && halves < display_health {
            instances.push(HudHeartInstance {
                kind: health.heart_type,
                half: halves.saturating_add(1) == display_health,
                blinking: true,
                x: xo,
                y: yo,
            });
        }

        if halves < current_health {
            instances.push(HudHeartInstance {
                kind: health.heart_type,
                half: halves.saturating_add(1) == current_health,
                blinking: false,
                x: xo,
                y: yo,
            });
        }
    }
    instances
}

/// One food icon's rect. `y_offset` is vanilla `Gui.extractFood`'s per-icon
/// starvation shake (`yo += random.nextInt(3) - 1`, Gui), applied
/// identically to the empty background and the half/full fill of that index.
pub(super) fn food_hud_rect(
    surface_size: PhysicalSize<u32>,
    index: u32,
    width: u32,
    height: u32,
    y_offset: i32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_FOOD_SPACING - width as f32,
        y: surface_height - 39.0 + y_offset as f32,
        width,
        height,
    }
}

/// One air bubble's rect. Vanilla `Gui.extractAirBubbles` walks
/// `airBubbleXPos = xRight - (airBubble - 1) * 8 - 9` (Gui, `airBubble`
/// is 1-based; `index` here is 0-based) from the same `xRight = guiWidth/2 + 91`
/// right edge as the food row. The y line replays `Gui.extractPlayerHealth` +
/// `getAirBubbleYLine`:
/// `yLineAir = (guiHeight - 39) - 10`, minus another 10 when no vehicle hearts
/// replace the food row (`vehicleHearts == 0`), then minus
/// `(getVisibleVehicleHeartRows(vehicleHearts) - 1) * 10` — which for 0 hearts
/// is `-1` rows and *adds* 10 back, seating the row at `guiHeight - 49` both
/// on foot and on a 1-row-heart vehicle. `y_offset` is the all-empty drowning
/// wobble ([`hud_air_bubble_wobble_offsets`]).
pub(super) fn air_bubble_hud_rect(
    surface_size: PhysicalSize<u32>,
    index: u32,
    vehicle_hearts: u32,
    width: u32,
    height: u32,
    y_offset: i32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    let mut y_line_air = (surface_height - 39.0) - 10.0;
    if vehicle_hearts == 0 {
        y_line_air -= 10.0;
    }
    // Vanilla `getVisibleVehicleHeartRows` = ceil(hearts / 10.0); 0 hearts → 0
    // rows → a -1 row offset.
    let vehicle_heart_rows = vehicle_hearts.div_ceil(HUD_VEHICLE_HEARTS_PER_ROW) as i32;
    y_line_air -= (vehicle_heart_rows - 1) as f32 * 10.0;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_AIR_BUBBLE_SPACING - width as f32,
        y: y_line_air + y_offset as f32,
        width,
        height,
    }
}

/// One vehicle heart's rect. Vanilla `Gui.extractVehicleHealth` walks
/// `xo = xRight - i * 8 - 9` from `xRight = guiWidth / 2 + 91` (the food row's
/// edge) and stacks rows upward from `yLine1 = guiHeight - 39` in 10px steps
pub(super) fn vehicle_heart_hud_rect(
    surface_size: PhysicalSize<u32>,
    row: u32,
    index: u32,
    width: u32,
    height: u32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_VEHICLE_HEART_SPACING - width as f32,
        y: surface_height - 39.0 - row as f32 * 10.0,
        width,
        height,
    }
}

/// One armor icon's rect. Vanilla `Gui.extractArmor` walks `xo = xLeft + i * 8`
/// along the same `xLeft = guiWidth / 2 - 91` left edge as the
/// hearts, at `yLineArmor = yLineBase - (numHealthRows - 1) * healthRowHeight -
/// 10`: a fixed `HUD_ARMOR_ROW_Y_OFFSET` (10px) above the heart
/// baseline plus one `healthRowHeight` step per extra health row, so multi-row
/// health pushes the armor bar up with the hearts.
pub(super) fn armor_hud_rect(
    surface_size: PhysicalSize<u32>,
    index: u32,
    num_health_rows: u32,
    health_row_height: i32,
    width: u32,
    height: u32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    let extra_rows = num_health_rows.saturating_sub(1) as f32;
    HudRect {
        x: surface_width * 0.5 - 91.0 + index as f32 * HUD_ARMOR_SPACING,
        y: surface_height - 39.0 - extra_rows * health_row_height as f32 - HUD_ARMOR_ROW_Y_OFFSET,
        width,
        height,
    }
}

pub(super) fn inventory_background_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> HudRect {
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    HudRect {
        x: origin_x + x as f32,
        y: origin_y + y as f32,
        width,
        height,
    }
}

pub(super) fn inventory_slot_item_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    slot_x: i32,
    slot_y: i32,
) -> HudRect {
    let (x, y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    HudRect {
        x: x + slot_x as f32,
        y: y + slot_y as f32,
        width: HUD_INVENTORY_ITEM_SIZE,
        height: HUD_INVENTORY_ITEM_SIZE,
    }
}

pub(super) fn inventory_slot_highlight_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    slot_x: i32,
    slot_y: i32,
) -> HudRect {
    let (x, y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    HudRect {
        x: x + slot_x as f32 + HUD_INVENTORY_SLOT_HIGHLIGHT_OFFSET,
        y: y + slot_y as f32 + HUD_INVENTORY_SLOT_HIGHLIGHT_OFFSET,
        width: HUD_INVENTORY_SLOT_HIGHLIGHT_SIZE,
        height: HUD_INVENTORY_SLOT_HIGHLIGHT_SIZE,
    }
}

pub(super) fn hud_item_count_digit_quad(
    item_rect: HudRect,
    text_width: i32,
    pen_x: f32,
    shadow: bool,
    glyph: HudDigitGlyph,
) -> HudGlyphQuad {
    let offset = if shadow { glyph.shadow_offset } else { 0.0 };
    glyph.styled_quads(
        item_rect.x + 17.0 - text_width as f32 + pen_x + offset,
        item_rect.y + 9.0 + offset,
        HudTextStyle::default(),
        false,
    )[0]
}

pub(super) fn hud_inventory_tooltip_text_height(line_count: usize) -> Option<u32> {
    match line_count {
        0 => None,
        1 => Some(HUD_TOOLTIP_LINE_HEIGHT - HUD_TOOLTIP_FIRST_LINE_EXTRA_GAP),
        line_count => u32::try_from(line_count)
            .ok()
            .and_then(|line_count| line_count.checked_mul(HUD_TOOLTIP_LINE_HEIGHT)),
    }
}

pub(super) fn hud_inventory_tooltip_background_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    anchor_x: i32,
    anchor_y: i32,
    text_width: i32,
    text_height: u32,
) -> HudRect {
    let (x, y) = inventory_tooltip_text_origin(
        surface_size,
        screen_width,
        screen_height,
        anchor_x,
        anchor_y,
        text_width,
        text_height,
    );
    HudRect {
        x: x - HUD_TOOLTIP_BACKGROUND_INSET,
        y: y - HUD_TOOLTIP_BACKGROUND_INSET,
        width: text_width
            .saturating_add(HUD_TOOLTIP_BACKGROUND_PADDING as i32)
            .max(0) as u32,
        height: text_height + HUD_TOOLTIP_BACKGROUND_PADDING,
    }
}

/// Pen origin (line top-left) of a tooltip text line in HUD pixels. Glyph
/// geometry from this origin — including the `7 - ascent` baseline offset
/// (vanilla `GlyphBitmap.getTop()`) and every style pass — comes from
/// `HudDigitGlyph::styled_quads` / `styled_effect_rects`.
#[allow(clippy::too_many_arguments)]
pub(super) fn hud_inventory_tooltip_line_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    anchor_x: i32,
    anchor_y: i32,
    text_width: i32,
    text_height: u32,
    line_index: usize,
) -> (f32, f32) {
    let (x, y) = inventory_tooltip_text_origin(
        surface_size,
        screen_width,
        screen_height,
        anchor_x,
        anchor_y,
        text_width,
        text_height,
    );
    (x, y + tooltip_line_y(line_index) as f32)
}

/// Pen origin (line top-left) of an inventory-screen text label in HUD
/// pixels; glyph geometry is produced by `HudDigitGlyph::styled_quads` /
/// `styled_effect_rects` from this origin.
pub(super) fn hud_inventory_text_label_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    label_x: i32,
    label_y: i32,
) -> (f32, f32) {
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    (origin_x + label_x as f32, origin_y + label_y as f32)
}

/// Pen origin (line top-left, HUD pixels) of the centered action-bar overlay
/// message: vanilla `Gui.extractOverlayMessage` translates to
/// `(guiWidth / 2, guiHeight - 68)` and draws at `(-width / 2, -4)`
/// (330); all divisions are Java int truncations.
pub(super) fn hud_overlay_message_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: i32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as i32;
    let base_y = surface_size.height.max(1) as i32 - HUD_OVERLAY_MESSAGE_BOTTOM_OFFSET;
    (
        (center_x - text_width / 2) as f32,
        (base_y + HUD_OVERLAY_MESSAGE_TEXT_Y) as f32,
    )
}

pub(super) fn hud_selected_item_name_origin(
    surface_size: PhysicalSize<u32>,
    text_width: i32,
    raised: bool,
) -> (f32, f32) {
    let gui_width = surface_size.width.max(1) as i32;
    let gui_height = surface_size.height.max(1) as i32;
    let y = gui_height - HUD_SELECTED_ITEM_NAME_BOTTOM_OFFSET
        + if raised {
            HUD_SELECTED_ITEM_NAME_NO_HEALTH_OFFSET
        } else {
            0
        };
    ((gui_width.wrapping_sub(text_width) / 2) as f32, y as f32)
}

/// Pen origin of the 4x-scaled title line: vanilla `Gui.extractTitle`
/// translates to the screen center `(guiWidth / 2, guiHeight / 2)`
/// then draws at `(-width / 2, -10)` under `scale(4, 4)`,
/// so the font-pixel offset is multiplied by the pose scale.
pub(super) fn hud_title_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: i32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as f32;
    let center_y = (surface_size.height.max(1) / 2) as f32;
    (
        center_x + HUD_TITLE_TEXT_SCALE * (-(text_width / 2)) as f32,
        center_y + HUD_TITLE_TEXT_SCALE * HUD_TITLE_TEXT_Y as f32,
    )
}

/// Pen origin of the 2x-scaled subtitle line: same screen-center pose as the
/// title, drawn at `(-width / 2, 5)` under `scale(2, 2)`.
pub(super) fn hud_subtitle_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: i32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as f32;
    let center_y = (surface_size.height.max(1) / 2) as f32;
    (
        center_x + HUD_SUBTITLE_TEXT_SCALE * (-(text_width / 2)) as f32,
        center_y + HUD_SUBTITLE_TEXT_SCALE * HUD_SUBTITLE_TEXT_Y as f32,
    )
}

pub(super) fn hud_item_durability_bar_rect(item_rect: HudRect, width: u32, height: u32) -> HudRect {
    HudRect {
        x: item_rect.x + HUD_ITEM_DURABILITY_BAR_X_OFFSET,
        y: item_rect.y + HUD_ITEM_DURABILITY_BAR_Y_OFFSET,
        width,
        height,
    }
}

pub(super) fn hud_item_cooldown_rect(item_rect: HudRect, progress: f32) -> Option<HudRect> {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return None;
    }
    let top_offset = (HUD_INVENTORY_ITEM_SIZE as f32 * (1.0 - progress)).floor();
    let height = (HUD_INVENTORY_ITEM_SIZE as f32 * progress).ceil() as u32;
    (height > 0).then_some(HudRect {
        x: item_rect.x,
        y: item_rect.y + top_offset,
        width: HUD_INVENTORY_ITEM_SIZE,
        height,
    })
}

fn inventory_screen_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> (f32, f32) {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    (
        (surface_width - screen_width as f32) * 0.5,
        (surface_height - screen_height as f32) * 0.5,
    )
}

fn inventory_tooltip_text_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    anchor_x: i32,
    anchor_y: i32,
    text_width: i32,
    text_height: u32,
) -> (f32, f32) {
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    let text_width = text_width as f32;
    let text_height = text_height as f32;
    let mut x = origin_x + anchor_x as f32 + HUD_TOOLTIP_MOUSE_X_OFFSET;
    let mut y = origin_y + anchor_y as f32 + HUD_TOOLTIP_MOUSE_Y_OFFSET;

    if x + text_width > surface_width {
        x = (x - HUD_TOOLTIP_RIGHT_FALLBACK_OFFSET - text_width).max(HUD_TOOLTIP_RIGHT_MARGIN);
    }

    let padded_height = text_height + HUD_TOOLTIP_BOTTOM_PADDING;
    if y + padded_height > surface_height {
        y = surface_height - padded_height;
    }

    (x, y)
}

fn tooltip_line_y(line_index: usize) -> u32 {
    if line_index == 0 {
        0
    } else {
        u32::try_from(line_index)
            .ok()
            .and_then(|line_index| line_index.checked_mul(HUD_TOOLTIP_LINE_HEIGHT))
            .and_then(|line_y| line_y.checked_add(HUD_TOOLTIP_FIRST_LINE_EXTRA_GAP))
            .unwrap_or(u32::MAX)
    }
}

pub(super) fn hud_food_fill(food: i32, index: u32) -> HudIconFill {
    let current_halves = food.clamp(0, (HUD_FOOD_ICONS_PER_ROW * 2) as i32) as u32;
    let center_half = index * 2 + 1;
    if center_half < current_halves {
        HudIconFill::Full
    } else if center_half == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Empty
    }
}

/// Which armor icon to draw at `index`, mirroring vanilla `Gui.extractArmor`'s
/// per-slot branches on `i * 2 + 1` versus the armor value:
/// `i*2+1 < armor` → full, `== armor` → half, `> armor` → empty. The overall
/// `armor > 0` visibility gate is applied by the caller (vanilla).
pub(super) fn hud_armor_fill(armor: i32, index: u32) -> HudIconFill {
    let center_half = index as i32 * 2 + 1;
    if center_half < armor {
        HudIconFill::Full
    } else if center_half == armor {
        HudIconFill::Half
    } else {
        HudIconFill::Empty
    }
}

/// What one air-bubble slot draws, mirroring the vanilla per-bubble branches
/// (`Gui.extractAirBubbles`): a full bubble (`hud/air`), the
/// one-tick popping frame (`hud/air_bursting`), the empty shell
/// (`hud/air_empty`), or nothing at all — the burst-delay gap between the
/// popping position and the delayed empty count draws no sprite for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudAirBubbleIcon {
    Full,
    Popping,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HudAirBubblePop {
    pub(crate) bubble: i32,
    pub(crate) empty_bubbles: i32,
}

/// Vanilla `Gui.extractAirBubbles`'s visibility gate:
/// the row draws only while the eye is in water or the clamped air supply is
/// below the max (`isUnderWater || currentAirSupplyTicks < maxAirSupplyTicks`).
pub(crate) fn hud_air_bubbles_visible(air: i32, max_air: i32, eye_in_water: bool) -> bool {
    eye_in_water || air.clamp(0, max_air) < max_air
}

/// Vanilla `Gui.getCurrentAirSupplyBubble`:
/// `Mth.ceil((cur + offset) * 10 / (float) max)` — float division, then the
/// float-ceil `Mth.ceil` (negative intermediates round up to 0).
fn current_air_supply_bubble(current_air: i32, max_air: i32, tick_offset: i32) -> i32 {
    (((current_air + tick_offset) * 10) as f32 / max_air as f32).ceil() as i32
}

#[derive(Debug, Clone, Copy)]
struct HudAirBubbleLayout {
    full_bubbles: i32,
    popping_bubble: i32,
    empty_bubbles: i32,
}

fn hud_air_bubble_layout(air: i32, max_air: i32, eye_in_water: bool) -> HudAirBubbleLayout {
    let current_air = air.clamp(0, max_air);
    let empty_delay = if current_air != 0 && eye_in_water {
        1
    } else {
        0
    };
    HudAirBubbleLayout {
        full_bubbles: current_air_supply_bubble(current_air, max_air, -2),
        popping_bubble: current_air_supply_bubble(current_air, max_air, 0),
        empty_bubbles: HUD_AIR_BUBBLES_PER_ROW as i32
            - current_air_supply_bubble(current_air, max_air, empty_delay),
    }
}

/// The one popping bubble extracted by vanilla for this air-supply frame,
/// together with the empty-bubble count used by `playAirBubblePoppedSound`.
pub(crate) fn hud_air_bubble_pop(
    air: i32,
    max_air: i32,
    eye_in_water: bool,
) -> Option<HudAirBubblePop> {
    if !eye_in_water {
        return None;
    }
    let layout = hud_air_bubble_layout(air, max_air, eye_in_water);
    (layout.full_bubbles != layout.popping_bubble).then_some(HudAirBubblePop {
        bubble: layout.popping_bubble,
        empty_bubbles: layout.empty_bubbles,
    })
}

/// The 10 air-bubble slots for one frame, mirroring `Gui.extractAirBubbles`.
/// Index 0 is the rightmost bubble (vanilla's 1-based
/// `airBubble` counter walks right-to-left):
/// - `fullAirBubbles = getCurrentAirSupplyBubble(cur, max, -2)`,
/// - `poppingAirBubblePosition = getCurrentAirSupplyBubble(cur, max, 0)`; a
///   bubble pops only while it differs from the full count *and* the eye is
///   under water (the popping frame is suppressed on land),
/// - `emptyAirBubbles = 10 - getCurrentAirSupplyBubble(cur, max, delay)` with
///   the one-tick refill delay `delay = (cur != 0 && underwater) ? 1: 0`
///   (`getEmptyBubbleDelayDuration`); slots between the full
///   count and the delayed empty tail draw nothing (`None`).
pub(super) fn hud_air_bubble_icons(
    air: i32,
    max_air: i32,
    eye_in_water: bool,
) -> [Option<HudAirBubbleIcon>; HUD_AIR_BUBBLES_PER_ROW as usize] {
    let layout = hud_air_bubble_layout(air, max_air, eye_in_water);
    let is_popping = layout.full_bubbles != layout.popping_bubble;

    let mut icons = [None; HUD_AIR_BUBBLES_PER_ROW as usize];
    for (index, icon) in icons.iter_mut().enumerate() {
        let bubble = index as i32 + 1;
        *icon = if bubble <= layout.full_bubbles {
            Some(HudAirBubbleIcon::Full)
        } else if is_popping && bubble == layout.popping_bubble && eye_in_water {
            Some(HudAirBubbleIcon::Popping)
        } else if bubble > HUD_AIR_BUBBLES_PER_ROW as i32 - layout.empty_bubbles {
            Some(HudAirBubbleIcon::Empty)
        } else {
            None
        };
    }
    icons
}

/// Per-bubble y wobble for the empty shells, mirroring vanilla
/// `Gui.extractAirBubbles`: while *all* 10 bubbles are empty
/// (out of air) and on even ticks, each empty shell shifts down by
/// `random.nextInt(2)` (∈ {0, 1}). Like the food starvation shake, vanilla's
/// wall-clock-seeded `this.random` sequence is unreproducible, so bbb reseeds
/// the identical `nextInt` LCG ([`HudObfuscatedRandom`]) per frame from `seed`
/// (the render frame counter) while gating on the real client `tick_count`.
pub(super) fn hud_air_bubble_wobble_offsets(
    all_bubbles_empty: bool,
    tick_count: u64,
    seed: u64,
) -> [i32; HUD_AIR_BUBBLES_PER_ROW as usize] {
    let mut offsets = [0i32; HUD_AIR_BUBBLES_PER_ROW as usize];
    if all_bubbles_empty && tick_count.is_multiple_of(2) {
        super::mark_hud_frame_varying_used();
        let mut random = HudObfuscatedRandom::with_seed(seed);
        for offset in &mut offsets {
            *offset = random.next_int_bound(2) as i32;
        }
    }
    offsets
}

/// Vanilla `Gui.getVehicleMaxHearts`: a living vehicle's
/// heart count is `(int) (maxHealth + 0.5F) / 2` (Java float add, then int
/// truncation, then int division), capped at 30 hearts; the whole vehicle row
/// (and the food-row replacement) is skipped at 0 hearts.
pub(super) fn hud_vehicle_max_hearts(max_health: f32) -> u32 {
    let hearts = ((max_health + 0.5) as i32 / 2).min(HUD_VEHICLE_MAX_HEARTS);
    hearts.max(0) as u32
}

/// Which overlay one vehicle heart draws, mirroring `Gui.extractVehicleHealth`'s
/// per-heart branches: with `currentHealth =
/// ceil(vehicle.getHealth())` and each row spanning 20 half-hearts
/// (`baseHealth += 20`), heart `i` of a row draws the full overlay while
/// `i * 2 + 1 + baseHealth < currentHealth`, the half overlay at equality, and
/// only the container otherwise (`Empty`).
pub(super) fn hud_vehicle_heart_fill(health: f32, row: u32, index: u32) -> HudIconFill {
    let current_health = health.ceil() as i32;
    let center_half = index as i32 * 2 + 1 + row as i32 * 20;
    if center_half < current_health {
        HudIconFill::Full
    } else if center_half == current_health {
        HudIconFill::Half
    } else {
        HudIconFill::Empty
    }
}

/// Per-icon starvation-shake y offsets for the food row, mirroring vanilla
/// `Gui.extractFood`: while `saturation <= 0` and
/// `tickCount % (foodLevel*3+1) == 0`, every icon shifts by
/// `random.nextInt(3) - 1` (∈ {-1, 0, 1}); otherwise all icons stay at 0.
///
/// Vanilla's `this.random` is a wall-clock-seeded `LegacyRandomSource` advanced
/// across the whole GUI, so its exact sequence is unreproducible; bbb keeps the
/// identical `nextInt(3)` LCG ([`HudObfuscatedRandom`]) but reseeds it per frame
/// from `seed` (the render frame counter, the same deterministic source the
/// obfuscated-glyph jitter uses) so the shake flickers frame-to-frame yet stays
/// reproducible. The tick modulo still gates on the real client `tick_count`
/// (vanilla `Gui.tickCount`) so the shake keeps its per-tick cadence.
pub(super) fn hud_food_jitter_offsets(
    food: i32,
    saturation_empty: bool,
    tick_count: u64,
    seed: u64,
) -> [i32; HUD_FOOD_ICONS_PER_ROW as usize] {
    let mut offsets = [0i32; HUD_FOOD_ICONS_PER_ROW as usize];
    // `foodLevel * 3 + 1` is always >= 1 for the vanilla 0..=20 food range; the
    // `max(0)` only guards a malformed negative projection from a modulo panic.
    let divisor = (food.max(0) as u64) * 3 + 1;
    if saturation_empty && tick_count.is_multiple_of(divisor) {
        super::mark_hud_frame_varying_used();
        let mut random = HudObfuscatedRandom::with_seed(seed);
        for offset in &mut offsets {
            *offset = random.next_int_bound(3) as i32 - 1;
        }
    }
    offsets
}

/// Pen origin (line top-left, HUD pixels) of the centered experience-level
/// number, mirroring `ContextualBarRenderer.extractExperienceLevel`
/// `x = (guiWidth - font.width(str)) / 2`
/// (Java int division), `y = guiHeight - 24 - 9 - 2`. The caller offsets this
/// origin by `(±1, 0)/(0, ±1)` for the four black outline copies.
pub(super) fn hud_experience_level_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: i32,
) -> (f32, f32) {
    let gui_width = surface_size.width.max(1) as i32;
    let gui_height = surface_size.height.max(1) as i32;
    let x = gui_width.wrapping_sub(text_width) / 2;
    let y = gui_height - 24 - 9 - 2;
    (x as f32, y as f32)
}

pub(super) fn hud_quad_vertices(
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
    tint: [f32; 4],
) -> [HudVertex; 6] {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width as f32;
    let y1 = rect.y + rect.height as f32;
    hud_styled_quad_vertices(
        surface_size,
        [[x0, y0], [x0, y1], [x1, y1], [x1, y0]],
        uv,
        tint,
    )
}

/// Corner-based variant of [`hud_quad_vertices`] for styled glyph quads:
/// `corners` are in HUD pixels using the `HudDigitGlyph::styled_quads` /
/// `HudGlyphQuad` order `[top_left, bottom_left, bottom_right, top_right]`
/// (vanilla `BakedSheetGlyph.render` winding). For an axis-aligned quad this
/// emits exactly the vertices of [`hud_quad_vertices`].
pub(super) fn hud_styled_quad_vertices(
    surface_size: PhysicalSize<u32>,
    corners: [[f32; 2]; 4],
    uv: HudUvRect,
    tint: [f32; 4],
) -> [HudVertex; 6] {
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let [top_left, bottom_left, bottom_right, top_right] =
        corners.map(|[x, y]| [x / width * 2.0 - 1.0, 1.0 - y / height * 2.0]);
    let uv_top_left = uv.min;
    let uv_top_right = [uv.max[0], uv.min[1]];
    let uv_bottom_right = uv.max;
    let uv_bottom_left = [uv.min[0], uv.max[1]];
    [
        HudVertex {
            position: top_left,
            uv: uv_top_left,
            tint,
            local_uv: [0.0, 0.0],
        },
        HudVertex {
            position: top_right,
            uv: uv_top_right,
            tint,
            local_uv: [1.0, 0.0],
        },
        HudVertex {
            position: bottom_right,
            uv: uv_bottom_right,
            tint,
            local_uv: [1.0, 1.0],
        },
        HudVertex {
            position: top_left,
            uv: uv_top_left,
            tint,
            local_uv: [0.0, 0.0],
        },
        HudVertex {
            position: bottom_right,
            uv: uv_bottom_right,
            tint,
            local_uv: [1.0, 1.0],
        },
        HudVertex {
            position: bottom_left,
            uv: uv_bottom_left,
            tint,
            local_uv: [0.0, 1.0],
        },
    ]
}

/// One textured quad of a nine-slice blit: `rect` is the on-screen placement and `uv` the
/// sub-rectangle of the (standalone) sprite texture, in the same conventions as
/// [`hud_quad_vertices`]. bbb uploads each nine-slice sprite as its own texture, so UVs are direct
/// fractions of the sprite (vanilla `TextureAtlasSprite.getU(x / spriteWidth)` with `u0 == 0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct NineSliceSegment {
    pub(super) rect: HudRect,
    pub(super) uv: HudUvRect,
}

/// Hard cap on tiled quads emitted for a single GUI sprite or nine-slice region so a
/// resource pack cannot force unbounded CPU/GPU expansion via 1x1 tile metadata.
pub(super) const MAX_TILED_SPRITE_SEGMENTS: usize = 4_096;

/// Clips a HUD background rect (e.g. tooltip or toast) against the viewport with a small
/// margin so untrusted wide strings cannot produce gigantic offscreen sprite rects.
pub(super) fn clip_hud_rect_to_viewport(rect: HudRect, surface_size: PhysicalSize<u32>) -> HudRect {
    let max_w = surface_size.width.max(1).saturating_add(64);
    let max_h = surface_size.height.max(1).saturating_add(64);
    if rect.width <= max_w && rect.height <= max_h {
        return rect;
    }
    let min_x = rect.x.max(-32.0);
    let min_y = rect.y.max(-32.0);
    let max_x = (rect.x + rect.width as f32).min(surface_size.width as f32 + 32.0);
    let max_y = (rect.y + rect.height as f32).min(surface_size.height as f32 + 32.0);
    let width = (max_x - min_x).max(0.0).min(max_w as f32) as u32;
    let height = (max_y - min_y).max(0.0).min(max_h as f32) as u32;
    HudRect {
        x: min_x,
        y: min_y,
        width,
        height,
    }
}

/// Tiles a whole GUI sprite over `target` at the declared tile size, clipping the final row and
/// column like vanilla `GuiGraphicsExtractor.blitTiledSprite`.
pub(super) fn tiled_sprite_segments(
    target: HudRect,
    tile_width: u32,
    tile_height: u32,
) -> Vec<NineSliceSegment> {
    let mut segments = Vec::new();
    if target.width == 0 || target.height == 0 || tile_width == 0 || tile_height == 0 {
        return segments;
    }

    let cols = u64::from(target.width.div_ceil(tile_width));
    let rows = u64::from(target.height.div_ceil(tile_height));
    if cols.saturating_mul(rows) > MAX_TILED_SPRITE_SEGMENTS as u64 {
        // Fall back to a single stretched quad when tiling would exceed the per-sprite segment cap.
        segments.push(nine_slice_region_segment(
            target,
            tile_width,
            tile_height,
            0,
            0,
            target.width,
            target.height,
            0,
            0,
            tile_width,
            tile_height,
        ));
        return segments;
    }

    let mut oy = 0u32;
    while oy < target.height {
        let height = tile_height.min(target.height - oy);
        let mut ox = 0u32;
        while ox < target.width {
            let width = tile_width.min(target.width - ox);
            segments.push(nine_slice_region_segment(
                target,
                tile_width,
                tile_height,
                ox,
                oy,
                width,
                height,
                0,
                0,
                width,
                height,
            ));
            let Some(next_ox) = ox.checked_add(tile_width) else {
                break;
            };
            ox = next_ox;
        }
        let Some(next_oy) = oy.checked_add(tile_height) else {
            break;
        };
        oy = next_oy;
    }

    segments
}

/// Splits `target` into vanilla nine-slice quads for `scaling`, mirroring
/// `GuiGraphicsExtractor.blitNineSlicedSprite`: borders are clamped to `target / 2` (integer
/// division), corners blit 1:1, and the four edges plus center either stretch (`stretch_inner`) or
/// tile (matching `blitTiledSprite`, with the last row/column tile clipped) across the remaining
/// span. Degenerate spans (`0` after clamping) are dropped, so a target smaller than twice its
/// borders collapses to just the four corner quads.
pub(super) fn nine_slice_segments(
    target: HudRect,
    scaling: HudNineSliceScaling,
) -> Vec<NineSliceSegment> {
    let mut segments = Vec::new();
    let target_width = target.width;
    let target_height = target.height;
    if target_width == 0 || target_height == 0 {
        return segments;
    }
    let sprite_width = scaling.sprite_width.max(1);
    let sprite_height = scaling.sprite_height.max(1);
    let border_left = scaling.border_left.min(target_width / 2);
    let border_right = scaling.border_right.min(target_width / 2);
    let border_top = scaling.border_top.min(target_height / 2);
    let border_bottom = scaling.border_bottom.min(target_height / 2);
    let inner_width = target_width.saturating_sub(border_left + border_right);
    let inner_height = target_height.saturating_sub(border_top + border_bottom);
    let tex_inner_width = sprite_width.saturating_sub(border_left + border_right);
    let tex_inner_height = sprite_height.saturating_sub(border_top + border_bottom);

    // Column/row spans as (screen offset, screen size, texture offset, texture size). The clamped
    // borders are used for both the screen rect and the texture rect, matching vanilla's use of the
    // clamped `borderLeft`/`borderRight`/... when it computes texture coordinates.
    let columns = [
        (0, border_left, 0, border_left),
        (border_left, inner_width, border_left, tex_inner_width),
        (
            target_width - border_right,
            border_right,
            sprite_width - border_right,
            border_right,
        ),
    ];
    let rows = [
        (0, border_top, 0, border_top),
        (border_top, inner_height, border_top, tex_inner_height),
        (
            target_height - border_bottom,
            border_bottom,
            sprite_height - border_bottom,
            border_bottom,
        ),
    ];

    for (row_index, &(row_offset, row_size, tex_row_offset, tex_row_size)) in
        rows.iter().enumerate()
    {
        for (col_index, &(col_offset, col_size, tex_col_offset, tex_col_size)) in
            columns.iter().enumerate()
        {
            if col_size == 0 || row_size == 0 {
                continue;
            }
            // The center (1, 1) and the four edges (exactly one index == 1) are the "inner" slices
            // that stretch or tile; the four corners (both indices != 1) always blit 1:1.
            let is_inner = row_index == 1 || col_index == 1;
            let tile = is_inner && !scaling.stretch_inner;
            push_nine_slice_region(
                &mut segments,
                target,
                sprite_width,
                sprite_height,
                col_offset,
                row_offset,
                col_size,
                row_size,
                tex_col_offset,
                tex_row_offset,
                tex_col_size,
                tex_row_size,
                tile,
            );
        }
    }

    segments
}

#[allow(clippy::too_many_arguments)]
fn push_nine_slice_region(
    segments: &mut Vec<NineSliceSegment>,
    target: HudRect,
    sprite_width: u32,
    sprite_height: u32,
    col_offset: u32,
    row_offset: u32,
    col_size: u32,
    row_size: u32,
    tex_col_offset: u32,
    tex_row_offset: u32,
    tex_col_size: u32,
    tex_row_size: u32,
    tile: bool,
) {
    let tile_count = if tile && tex_col_size > 0 && tex_row_size > 0 {
        u64::from(col_size.div_ceil(tex_col_size))
            .saturating_mul(u64::from(row_size.div_ceil(tex_row_size)))
    } else {
        0
    };
    let remaining_budget = MAX_TILED_SPRITE_SEGMENTS.saturating_sub(segments.len()) as u64;
    if !tile || tex_col_size == 0 || tex_row_size == 0 || tile_count > remaining_budget {
        // Single stretched quad (also the graceful fallback when a texture span is zero or when
        // tiling would exceed the per-sprite segment cap).
        segments.push(nine_slice_region_segment(
            target,
            sprite_width,
            sprite_height,
            col_offset,
            row_offset,
            col_size,
            row_size,
            tex_col_offset,
            tex_row_offset,
            tex_col_size,
            tex_row_size,
        ));
        return;
    }
    // Tile the texture span (`tex_col_size` x `tex_row_size`) across the screen span at 1:1 scale,
    // clipping the final row/column tile like vanilla's `blitTiledSprite`.
    let mut oy = 0u32;
    while oy < row_size {
        let tile_height = tex_row_size.min(row_size - oy);
        let Some(screen_y) = row_offset.checked_add(oy) else {
            break;
        };
        let mut ox = 0u32;
        while ox < col_size {
            let tile_width = tex_col_size.min(col_size - ox);
            let Some(screen_x) = col_offset.checked_add(ox) else {
                break;
            };
            segments.push(nine_slice_region_segment(
                target,
                sprite_width,
                sprite_height,
                screen_x,
                screen_y,
                tile_width,
                tile_height,
                tex_col_offset,
                tex_row_offset,
                tile_width,
                tile_height,
            ));
            let Some(next_ox) = ox.checked_add(tex_col_size) else {
                break;
            };
            ox = next_ox;
        }
        let Some(next_oy) = oy.checked_add(tex_row_size) else {
            break;
        };
        oy = next_oy;
    }
}

#[allow(clippy::too_many_arguments)]
fn nine_slice_region_segment(
    target: HudRect,
    sprite_width: u32,
    sprite_height: u32,
    col_offset: u32,
    row_offset: u32,
    col_size: u32,
    row_size: u32,
    tex_col_offset: u32,
    tex_row_offset: u32,
    tex_col_size: u32,
    tex_row_size: u32,
) -> NineSliceSegment {
    let sprite_width = sprite_width as f32;
    let sprite_height = sprite_height as f32;
    NineSliceSegment {
        rect: HudRect {
            x: target.x + col_offset as f32,
            y: target.y + row_offset as f32,
            width: col_size,
            height: row_size,
        },
        uv: HudUvRect {
            min: [
                tex_col_offset as f32 / sprite_width,
                tex_row_offset as f32 / sprite_height,
            ],
            max: [
                (tex_col_offset + tex_col_size) as f32 / sprite_width,
                (tex_row_offset + tex_row_size) as f32 / sprite_height,
            ],
        },
    }
}

/// Plans the tooltip's two nine-slice layers over `target` in vanilla source order: every
/// `background` segment, then every `frame` segment (`TooltipRenderUtil.extractTooltipBackground`
/// blits `tooltip/background` before `tooltip/frame` over the same rect).
#[cfg(test)]
mod tests {
    use super::super::{HudAsciiGlyph, HudTextStyle};
    use super::*;

    mod geometry;

    #[test]
    fn hud_layout_matches_vanilla_hotbar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let hotbar = hotbar_hud_rect(surface_size, 182, 22);
        assert_eq!(hotbar.x, 549.0);
        assert_eq!(hotbar.y, 698.0);
        assert_eq!(hotbar.width, 182);
        assert_eq!(hotbar.height, 22);

        let selection = hotbar_selection_hud_rect(surface_size, 0, 24, 23);
        assert_eq!(selection.x, 548.0);
        assert_eq!(selection.y, 697.0);
        assert_eq!(selection.width, 24);
        assert_eq!(selection.height, 23);

        let last_selection = hotbar_selection_hud_rect(surface_size, 8, 24, 23);
        assert_eq!(last_selection.x, 708.0);
        assert_eq!(last_selection.y, 697.0);
    }

    #[test]
    fn hud_layout_matches_vanilla_hotbar_item_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = hotbar_item_hud_rect(surface_size, 0);
        assert_eq!(first.x, 552.0);
        assert_eq!(first.y, 701.0);
        assert_eq!(first.width, 16);
        assert_eq!(first.height, 16);

        let last = hotbar_item_hud_rect(surface_size, 8);
        assert_eq!(last.x, 712.0);
        assert_eq!(last.y, 701.0);
    }

    fn player_health(health: f32) -> HudPlayerHealth {
        HudPlayerHealth {
            health,
            display_health: health.ceil() as i32,
            blinking: false,
            max_health: 20.0,
            absorption: 0.0,
            heart_type: HudHeartKind::Normal,
            hardcore: false,
            regen: false,
            tick_count: 0,
        }
    }

    fn container_hearts(instances: &[HudHeartInstance]) -> Vec<HudHeartInstance> {
        instances
            .iter()
            .copied()
            .filter(|i| i.kind == HudHeartKind::Container)
            .collect()
    }

    fn fill_hearts(instances: &[HudHeartInstance]) -> Vec<HudHeartInstance> {
        instances
            .iter()
            .copied()
            .filter(|i| i.kind != HudHeartKind::Container)
            .collect()
    }

    #[test]
    fn hud_player_heart_instances_follow_the_base_heart_type() {
        let surface_size = PhysicalSize::new(1280, 720);
        for kind in [
            HudHeartKind::Normal,
            HudHeartKind::Poisoned,
            HudHeartKind::Withered,
            HudHeartKind::Frozen,
        ] {
            let mut health = player_health(20.0);
            health.heart_type = kind;
            let fills = fill_hearts(&hud_player_heart_instances(surface_size, health));
            assert_eq!(fills.len(), 10);
            assert!(fills.iter().all(|i| i.kind == kind && !i.half));
        }
    }

    #[test]
    fn hud_player_heart_instances_append_absorption_hearts() {
        let surface_size = PhysicalSize::new(1280, 720);
        let mut health = player_health(20.0);
        health.absorption = 6.0; // 3 absorption hearts beyond the 10 health hearts
        let instances = hud_player_heart_instances(surface_size, health);
        // 13 containers (10 health + 3 absorption), 10 Normal fulls, 3 Absorbing.
        assert_eq!(container_hearts(&instances).len(), 13);
        let normals = instances
            .iter()
            .filter(|i| i.kind == HudHeartKind::Normal)
            .count();
        let absorbing = instances
            .iter()
            .filter(|i| i.kind == HudHeartKind::Absorbing)
            .count();
        assert_eq!(normals, 10);
        assert_eq!(absorbing, 3);

        // An odd absorption amount ends in a half absorbing heart.
        health.absorption = 5.0; // ceil -> 5 -> 3 containers, last is half
        let instances = hud_player_heart_instances(surface_size, health);
        let absorbing: Vec<_> = instances
            .iter()
            .filter(|i| i.kind == HudHeartKind::Absorbing)
            .collect();
        assert_eq!(absorbing.len(), 3);
        assert_eq!(absorbing.iter().filter(|i| i.half).count(), 1);

        // Withered players keep their own sprite for the absorption hearts
        // (vanilla `type == WITHERED ? type : ABSORBING`).
        let mut withered = player_health(20.0);
        withered.heart_type = HudHeartKind::Withered;
        withered.absorption = 6.0;
        let instances = hud_player_heart_instances(surface_size, withered);
        assert!(instances.iter().all(|i| i.kind != HudHeartKind::Absorbing));
        assert_eq!(
            instances
                .iter()
                .filter(|i| i.kind == HudHeartKind::Withered)
                .count(),
            13
        );
    }

    #[test]
    fn hud_player_heart_instances_order_damage_and_heal_blink_strata() {
        let surface_size = PhysicalSize::new(1280, 720);
        let x_left = 549.0;

        let mut damaged = player_health(18.0);
        damaged.display_health = 19;
        damaged.blinking = true;
        let instances = hud_player_heart_instances(surface_size, damaged);
        let lost_slot: Vec<_> = instances
            .iter()
            .copied()
            .filter(|instance| instance.x == x_left + 9.0 * HUD_HEART_SPACING)
            .collect();
        assert_eq!(lost_slot.len(), 2);
        assert_eq!(lost_slot[0].kind, HudHeartKind::Container);
        assert!(lost_slot[0].blinking && !lost_slot[0].half);
        assert_eq!(lost_slot[1].kind, HudHeartKind::Normal);
        assert!(lost_slot[1].blinking && lost_slot[1].half);

        let overlapping_slot: Vec<_> = instances
            .iter()
            .copied()
            .filter(|instance| instance.x == x_left + 8.0 * HUD_HEART_SPACING)
            .collect();
        assert_eq!(overlapping_slot.len(), 3);
        assert!(overlapping_slot[0].blinking);
        assert!(overlapping_slot[1].blinking);
        assert!(!overlapping_slot[2].blinking);

        let mut healed = player_health(20.0);
        healed.display_health = 17;
        healed.blinking = true;
        let healed_slot: Vec<_> = hud_player_heart_instances(surface_size, healed)
            .into_iter()
            .filter(|instance| instance.x == x_left + 8.0 * HUD_HEART_SPACING)
            .collect();
        assert_eq!(healed_slot.len(), 3);
        assert_eq!(
            healed_slot
                .iter()
                .map(|instance| (instance.kind, instance.half, instance.blinking))
                .collect::<Vec<_>>(),
            vec![
                (HudHeartKind::Container, false, true),
                (HudHeartKind::Normal, true, true),
                (HudHeartKind::Normal, false, false),
            ]
        );
    }

    #[test]
    fn hud_layout_places_food_icons_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = food_hud_rect(surface_size, 0, 9, 9, 0);
        let last = food_hud_rect(surface_size, 9, 9, 9, 0);
        assert_eq!(first.x, 722.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_layout_places_air_bubbles_one_row_above_the_food_row() {
        let surface_size = PhysicalSize::new(1280, 720);
        // On foot (0 vehicle hearts): same right edge and stride as the food
        // row, one 10px row above it. Vanilla's chain is
        // yLineAir = (720-39) - 10, food shown -> -10 more, then
        // `getAirBubbleYLine(0, ..)` has a -1 row offset and adds 10 back:
        // 720 - 49 = 671.
        let first = air_bubble_hud_rect(surface_size, 0, 0, 9, 9, 0);
        let last = air_bubble_hud_rect(surface_size, 9, 0, 9, 9, 0);
        assert_eq!(first.x, 722.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(first.y, 671.0);
        assert_eq!(last.y, 671.0);
        assert_eq!(food_hud_rect(surface_size, 0, 9, 9, 0).y - first.y, 10.0);

        // Riding a 1-row-heart vehicle (1..=10 hearts): the food -10 is
        // skipped and the row offset is 0, landing on the same 671 line.
        assert_eq!(air_bubble_hud_rect(surface_size, 0, 10, 9, 9, 0).y, 671.0);
        // 2 and 3 vehicle heart rows push the bubbles up 10px per extra row.
        assert_eq!(air_bubble_hud_rect(surface_size, 0, 13, 9, 9, 0).y, 661.0);
        assert_eq!(air_bubble_hud_rect(surface_size, 0, 25, 9, 9, 0).y, 651.0);

        // The all-empty drowning wobble shifts only y.
        let wobbled = air_bubble_hud_rect(surface_size, 0, 0, 9, 9, 1);
        assert_eq!(wobbled.y, first.y + 1.0);
        assert_eq!(wobbled.x, first.x);
    }

    #[test]
    fn hud_layout_places_vehicle_hearts_on_the_food_row() {
        let surface_size = PhysicalSize::new(1280, 720);
        // Row 0 shares the food row baseline (720 - 39 = 681) and the
        // right-anchored 8px stride (xRight = 640 + 91).
        let first = vehicle_heart_hud_rect(surface_size, 0, 0, 9, 9);
        let last = vehicle_heart_hud_rect(surface_size, 0, 9, 9, 9);
        assert_eq!(first.x, 722.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(first.y, food_hud_rect(surface_size, 0, 9, 9, 0).y);
        // Additional rows stack upward in 10px steps (yo -= 10).
        assert_eq!(vehicle_heart_hud_rect(surface_size, 1, 0, 9, 9).y, 671.0);
        assert_eq!(vehicle_heart_hud_rect(surface_size, 2, 0, 9, 9).y, 661.0);
    }

    #[test]
    fn tiled_sprite_segments_repeat_whole_sprite_and_clip_final_row_and_column() {
        let segments = tiled_sprite_segments(
            HudRect {
                x: 10.0,
                y: 20.0,
                width: 5,
                height: 3,
            },
            2,
            2,
        );

        assert_eq!(segments.len(), 6);
        assert_eq!(
            segments[0],
            NineSliceSegment {
                rect: HudRect {
                    x: 10.0,
                    y: 20.0,
                    width: 2,
                    height: 2,
                },
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            }
        );
        assert_eq!(
            segments[5],
            NineSliceSegment {
                rect: HudRect {
                    x: 14.0,
                    y: 22.0,
                    width: 1,
                    height: 1,
                },
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [0.5, 0.5],
                },
            }
        );
    }

    fn uniform_nine_slice(size: u32, border: u32, stretch_inner: bool) -> HudNineSliceScaling {
        HudNineSliceScaling {
            sprite_width: size,
            sprite_height: size,
            border_left: border,
            border_top: border,
            border_right: border,
            border_bottom: border,
            stretch_inner,
        }
    }

    fn glyph_top_left(glyph: HudAsciiGlyph, x: f32, y: f32, style: HudTextStyle) -> [f32; 2] {
        glyph.styled_quads(x, y, style, false)[0].corners[0]
    }

    #[test]
    fn hud_air_bubbles_visible_only_underwater_or_below_max() {
        // Full supply on land: hidden (vanilla).
        assert!(!hud_air_bubbles_visible(300, 300, false));
        // Underwater always shows, even at the full supply.
        assert!(hud_air_bubbles_visible(300, 300, true));
        // Back on land while refilling: still shown until the max.
        assert!(hud_air_bubbles_visible(299, 300, false));
        // Over-max values clamp down to max (hidden on land).
        assert!(!hud_air_bubbles_visible(400, 300, false));
    }

    #[test]
    fn hud_air_bubble_icons_mirror_the_vanilla_bubble_formulas() {
        use HudAirBubbleIcon::{Empty, Full, Popping};

        // Full supply underwater: full = ceil((300-2)*10/300) = ceil(9.93) =
        // 10, popping position = ceil(3000/300) = 10 -> equal, no popping;
        // all 10 bubbles full.
        assert_eq!(hud_air_bubble_icons(300, 300, true), [Some(Full); 10]);

        // air = 150 underwater: full = ceil(148*10/300) = ceil(4.93) = 5;
        // popping = ceil(1500/300) = 5 -> no popping frame; the one-tick
        // refill delay makes empty = 10 - ceil(151*10/300) = 10 - 6 = 4, so
        // bubble 6 (index 5) draws nothing this tick and bubbles 7..10 are
        // empty shells.
        assert_eq!(
            hud_air_bubble_icons(150, 300, true),
            [
                Some(Full),
                Some(Full),
                Some(Full),
                Some(Full),
                Some(Full),
                None,
                Some(Empty),
                Some(Empty),
                Some(Empty),
                Some(Empty),
            ]
        );

        // air = 61 underwater: full = ceil(59*10/300) = 2 but popping position
        // = ceil(610/300) = 3 -> bubble 3 (index 2) shows the bursting frame;
        // empty = 10 - ceil(62*10/300) = 7 -> bubbles 4..10 empty.
        let icons = hud_air_bubble_icons(61, 300, true);
        assert_eq!(icons[0], Some(Full));
        assert_eq!(icons[1], Some(Full));
        assert_eq!(icons[2], Some(Popping));
        assert!(icons[3..].iter().all(|icon| *icon == Some(Empty)));

        // The same air on land suppresses the popping frame (vanilla requires
        // `isUnderWater`, Gui) and drops the refill delay (delay = 0
        // -> empty = 10 - 3 = 7): bubble 3 draws nothing.
        let icons = hud_air_bubble_icons(61, 300, false);
        assert_eq!(icons[2], None);
        assert!(icons[3..].iter().all(|icon| *icon == Some(Empty)));

        // Out of air underwater: full = ceil(-20/300) = 0, popping = 0 (no
        // popping), delay = 0 because cur == 0, so all 10 are empty shells.
        assert_eq!(hud_air_bubble_icons(0, 300, true), [Some(Empty); 10]);
        // Negative (drowning-damage) supplies clamp to 0 first.
        assert_eq!(hud_air_bubble_icons(-10, 300, true), [Some(Empty); 10]);
    }

    #[test]
    fn hud_air_bubble_pop_exposes_the_sound_inputs_from_the_same_formulas() {
        assert_eq!(
            hud_air_bubble_pop(61, 300, true),
            Some(HudAirBubblePop {
                bubble: 3,
                empty_bubbles: 7,
            })
        );
        assert_eq!(hud_air_bubble_pop(60, 300, true), None);
        assert_eq!(hud_air_bubble_pop(61, 300, false), None);
        assert_eq!(hud_air_bubble_pop(-10, 300, true), None);
    }

    #[test]
    fn hud_vehicle_max_hearts_rounds_and_caps_like_the_gui() {
        // Vanilla `(int)(maxHealth + 0.5F) / 2`, capped at 30.
        assert_eq!(hud_vehicle_max_hearts(20.0), 10); // pigs/horses at 20 max
        assert_eq!(hud_vehicle_max_hearts(15.0), 7); // (15.5 -> 15) / 2
        assert_eq!(hud_vehicle_max_hearts(15.5), 8); // (16.0 -> 16) / 2
        assert_eq!(hud_vehicle_max_hearts(100.0), 30); // 50 hearts capped
        assert_eq!(hud_vehicle_max_hearts(1.0), 0); // (1.5 -> 1) / 2 = 0
        assert_eq!(hud_vehicle_max_hearts(0.0), 0);
    }

    #[test]
    fn hud_vehicle_heart_fill_splits_rows_on_the_20_half_heart_base() {
        // ceil(health) = 7 in row 0: hearts 0..2 full (1,3,5 < 7), heart 3
        // half (7 == 7), hearts 4.. container-only.
        assert_eq!(hud_vehicle_heart_fill(6.2, 0, 2), HudIconFill::Full);
        assert_eq!(hud_vehicle_heart_fill(6.2, 0, 3), HudIconFill::Half);
        assert_eq!(hud_vehicle_heart_fill(6.2, 0, 4), HudIconFill::Empty);

        // Row 1 offsets by baseHealth = 20: health 22 fills row-1 heart 0
        // (21 < 22) and leaves heart 1 empty (23 > 22); health 21 puts the
        // half on row-1 heart 0 (21 == 21).
        assert!((0..HUD_VEHICLE_HEARTS_PER_ROW)
            .all(|index| hud_vehicle_heart_fill(22.0, 0, index) == HudIconFill::Full));
        assert_eq!(hud_vehicle_heart_fill(22.0, 1, 0), HudIconFill::Full);
        assert_eq!(hud_vehicle_heart_fill(22.0, 1, 1), HudIconFill::Empty);
        assert_eq!(hud_vehicle_heart_fill(21.0, 1, 0), HudIconFill::Half);
    }

    fn assert_f32_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000001,
            "actual {actual} expected {expected}"
        );
    }
}
