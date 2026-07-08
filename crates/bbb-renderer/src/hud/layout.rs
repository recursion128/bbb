use bbb_render_types::HudObfuscatedRandom;
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
const HUD_EXPERIENCE_BAR_WIDTH: u32 = 182;
const HUD_EXPERIENCE_BAR_HEIGHT: u32 = 5;
const HUD_EXPERIENCE_MARGIN_BOTTOM: f32 = 24.0;
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
/// Vanilla `Gui.extractTitle` pose: `translate(guiWidth / 2, guiHeight / 2)`
/// (Gui.java:357), title `scale(4, 4)` at `(-width / 2, -10)` (:359-362),
/// subtitle `scale(2, 2)` at `(-width / 2, 5)` (:366-368).
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
    x: f32,
    y: f32,
    width: u32,
    height: u32,
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
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_EXPERIENCE_BAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_EXPERIENCE_MARGIN_BOTTOM - HUD_EXPERIENCE_BAR_HEIGHT as f32,
        width,
        height,
    }
}

pub(super) fn hud_contextual_bar_progress_width(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 183.0).floor() as u32).min(HUD_EXPERIENCE_BAR_WIDTH)
}

/// One boss-bar sprite row: `x = guiWidth / 2 - 91` (Java int division,
/// BossHealthOverlay.java:66), `width` is the full 182px sheet for
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
/// (`BossHealthOverlay.extractRenderState`, BossHealthOverlay.java:63-77):
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
/// Mth.java:527-531): `floor(progress * 181) + (progress > 0 ? 1 : 0)`, so
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
/// samples the `(0..width, 0..5)` sub-rect (BossHealthOverlay.java:101-103).
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
/// (BossHealthOverlay.java:71-73) in opaque white with the default drop
/// shadow (`graphics.text(..., -1)`, GuiGraphicsExtractor.java:261-263);
/// Java int divisions throughout.
pub(super) fn hud_boss_bar_name_origin(
    surface_size: PhysicalSize<u32>,
    bar_y: i32,
    text_width: u32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as i32;
    (
        (center_x - text_width as i32 / 2) as f32,
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
/// state (Gui.java:746,768,770-771): resolves `currentHealth = ceil(health)`
/// and `maxHealth = max(attr, currentHealth)` (blink deferred, so `oldHealth ==
/// currentHealth`), then `numHealthRows = ceil((maxHealth + ceil(absorption)) /
/// 2 / 10)` and `healthRowHeight = max(10 - (numHealthRows - 2), 3)`.
pub(super) fn hud_health_row_geometry(health: HudPlayerHealth) -> (u32, i32) {
    let current_health = health.health.ceil().max(0.0);
    let max_health = health.max_health.max(current_health);
    hud_health_rows(max_health, health.absorption)
}

/// Core of [`hud_health_row_geometry`]: `numHealthRows` /`healthRowHeight` from
/// an already-resolved `maxHealth` float and the raw absorption amount
/// (Gui.java:769-771). `absorption` is ceiled to `totalAbsorption` first.
pub(super) fn hud_health_rows(max_health: f32, absorption: f32) -> (u32, i32) {
    let total_absorption = absorption.ceil().max(0.0) as i32;
    let num_rows = ((max_health + total_absorption as f32) / 2.0 / 10.0)
        .ceil()
        .max(0.0) as i32;
    // Vanilla `Math.max(10 - (numHealthRows - 2), 3)`.
    let row_height = (10 - (num_rows - 2)).max(3);
    (num_rows.max(0) as u32, row_height)
}

/// Vanilla `Gui.extractHearts` (Gui.java:820-873) with blink omitted (blink is
/// deferred: it needs the untracked `invulnerableTime` and wall-clock
/// `displayHealth`). Walks containers from `healthContainerCount +
/// absorptionContainerCount - 1` down to `0` (the vanilla descending loop),
/// emitting for each: the `Container` background, then the absorbing overlay
/// (`Withered` keeps its own sprite, else `Absorbing`) while its half is inside
/// the absorption amount, then the base fill while its half is inside
/// `currentHealth`. `xLeft = guiWidth/2 - 91`, `yLineBase = guiHeight - 39`;
/// rows stack up by `healthRowHeight`. The Regeneration wave lifts one health
/// heart (`containerIndex == tickCount % ceil(maxHealth + 5)`) by 2px, and at
/// `currentHealth + absorption <= 4` every heart shakes by `nextInt(2)` from a
/// `tickCount * 312871`-seeded LCG (vanilla reseeds the identical
/// `LegacyRandomSource` at Gui.java:764, so this reproduces its sequence
/// exactly — unlike the food/air shakes whose seed is wall-clock).
pub(super) fn hud_player_heart_instances(
    surface_size: PhysicalSize<u32>,
    health: HudPlayerHealth,
) -> Vec<HudHeartInstance> {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    let x_left = surface_width * 0.5 - 91.0;
    let y_line_base = surface_height - 39.0;

    let current_health = health.health.ceil().max(0.0) as i32;
    let absorption = health.absorption.ceil().max(0.0) as i32;
    let max_health = health.max_health.max(current_health as f32);
    let (_, health_row_height) = hud_health_rows(max_health, health.absorption);

    let health_container_count = (max_health / 2.0).ceil().max(0.0) as i32;
    let absorption_container_count = (absorption as f32 / 2.0).ceil().max(0.0) as i32;
    let max_health_halves = health_container_count * 2;

    // Vanilla `heartOffsetIndex = tickCount % ceil(maxHealth + 5)` under
    // Regeneration, else `-1` (never matches a container index).
    let heart_offset_index = if health.regen {
        let period = (max_health + 5.0).ceil().max(1.0) as i64;
        (health.tick_count as i64 % period) as i32
    } else {
        -1
    };

    // Low-health shake: one `nextInt(2)` per container consumed in the
    // descending order, from a per-tick reseed (Gui.java:764,844-846).
    let mut shake = ((current_health + absorption) <= 4).then(|| {
        let seed = (health.tick_count as i32).wrapping_mul(312871) as i64 as u64;
        HudObfuscatedRandom::with_seed(seed)
    });

    let total_containers = health_container_count + absorption_container_count;
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
            x: xo,
            y: yo,
        });

        let halves = container_index * 2;
        if container_index >= health_container_count {
            let absorption_halves = halves - max_health_halves;
            if absorption_halves < absorption {
                let kind = if health.heart_type == HudHeartKind::Withered {
                    HudHeartKind::Withered
                } else {
                    HudHeartKind::Absorbing
                };
                instances.push(HudHeartInstance {
                    kind,
                    half: absorption_halves + 1 == absorption,
                    x: xo,
                    y: yo,
                });
            }
        }

        if halves < current_health {
            instances.push(HudHeartInstance {
                kind: health.heart_type,
                half: halves + 1 == current_health,
                x: xo,
                y: yo,
            });
        }
    }
    instances
}

/// One food icon's rect. `y_offset` is vanilla `Gui.extractFood`'s per-icon
/// starvation shake (`yo += random.nextInt(3) - 1`, Gui.java:958-960), applied
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
/// `airBubbleXPos = xRight - (airBubble - 1) * 8 - 9` (Gui.java:903, `airBubble`
/// is 1-based; `index` here is 0-based) from the same `xRight = guiWidth/2 + 91`
/// right edge as the food row. The y line replays `Gui.extractPlayerHealth` +
/// `getAirBubbleYLine` exactly (Gui.java:772,784-792,917-920):
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
    // rows → a -1 row offset (Gui.java:917-920).
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
/// (Gui.java:981-1001).
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
/// (Gui.java:804) along the same `xLeft = guiWidth / 2 - 91` left edge as the
/// hearts, at `yLineArmor = yLineBase - (numHealthRows - 1) * healthRowHeight -
/// 10` (Gui.java:801): a fixed `HUD_ARMOR_ROW_Y_OFFSET` (10px) above the heart
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

pub(super) fn hud_item_count_digit_hud_rect(
    item_rect: HudRect,
    text_width: u32,
    pen_x: u32,
    shadow_offset: f32,
    glyph: HudDigitGlyph,
) -> HudRect {
    HudRect {
        x: item_rect.x + 17.0 - text_width as f32 + pen_x as f32 + shadow_offset,
        y: item_rect.y + 9.0 + shadow_offset,
        width: glyph.width,
        height: glyph.height,
    }
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
    text_width: u32,
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
        width: text_width + HUD_TOOLTIP_BACKGROUND_PADDING,
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
    text_width: u32,
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
/// (Gui.java:321,330); all divisions are Java int truncations.
pub(super) fn hud_overlay_message_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: u32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as i32;
    let base_y = surface_size.height.max(1) as i32 - HUD_OVERLAY_MESSAGE_BOTTOM_OFFSET;
    (
        (center_x - text_width as i32 / 2) as f32,
        (base_y + HUD_OVERLAY_MESSAGE_TEXT_Y) as f32,
    )
}

/// Pen origin of the 4x-scaled title line: vanilla `Gui.extractTitle`
/// translates to the screen center `(guiWidth / 2, guiHeight / 2)`
/// (Gui.java:357), then draws at `(-width / 2, -10)` under `scale(4, 4)`
/// (:359-362), so the font-pixel offset is multiplied by the pose scale.
pub(super) fn hud_title_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: u32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as f32;
    let center_y = (surface_size.height.max(1) / 2) as f32;
    (
        center_x + HUD_TITLE_TEXT_SCALE * (-(text_width as i32 / 2)) as f32,
        center_y + HUD_TITLE_TEXT_SCALE * HUD_TITLE_TEXT_Y as f32,
    )
}

/// Pen origin of the 2x-scaled subtitle line: same screen-center pose as the
/// title, drawn at `(-width / 2, 5)` under `scale(2, 2)` (Gui.java:366-368).
pub(super) fn hud_subtitle_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: u32,
) -> (f32, f32) {
    let center_x = (surface_size.width.max(1) / 2) as f32;
    let center_y = (surface_size.height.max(1) / 2) as f32;
    (
        center_x + HUD_SUBTITLE_TEXT_SCALE * (-(text_width as i32 / 2)) as f32,
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
    text_width: u32,
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
/// per-slot branches on `i * 2 + 1` versus the armor value (Gui.java:805-814):
/// `i*2+1 < armor` → full, `== armor` → half, `> armor` → empty. The overall
/// `armor > 0` visibility gate is applied by the caller (vanilla Gui.java:800).
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
/// (`Gui.extractAirBubbles`, Gui.java:902-913): a full bubble (`hud/air`), the
/// one-tick popping frame (`hud/air_bursting`), the empty shell
/// (`hud/air_empty`), or nothing at all — the burst-delay gap between the
/// popping position and the delayed empty count draws no sprite for a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudAirBubbleIcon {
    Full,
    Popping,
    Empty,
}

/// Vanilla `Gui.extractAirBubbles`'s visibility gate (Gui.java:888-891):
/// the row draws only while the eye is in water or the clamped air supply is
/// below the max (`isUnderWater || currentAirSupplyTicks < maxAirSupplyTicks`).
pub(super) fn hud_air_bubbles_visible(air: i32, max_air: i32, eye_in_water: bool) -> bool {
    eye_in_water || air.clamp(0, max_air) < max_air
}

/// Vanilla `Gui.getCurrentAirSupplyBubble` (Gui.java:922-924):
/// `Mth.ceil((cur + offset) * 10 / (float) max)` — float division, then the
/// float-ceil `Mth.ceil` (negative intermediates round up to 0).
fn current_air_supply_bubble(current_air: i32, max_air: i32, tick_offset: i32) -> i32 {
    (((current_air + tick_offset) * 10) as f32 / max_air as f32).ceil() as i32
}

/// The 10 air-bubble slots for one frame, mirroring `Gui.extractAirBubbles`
/// (Gui.java:887-913). Index 0 is the rightmost bubble (vanilla's 1-based
/// `airBubble` counter walks right-to-left):
/// - `fullAirBubbles = getCurrentAirSupplyBubble(cur, max, -2)`,
/// - `poppingAirBubblePosition = getCurrentAirSupplyBubble(cur, max, 0)`; a
///   bubble pops only while it differs from the full count *and* the eye is
///   under water (the popping frame is suppressed on land, Gui.java:906),
/// - `emptyAirBubbles = 10 - getCurrentAirSupplyBubble(cur, max, delay)` with
///   the one-tick refill delay `delay = (cur != 0 && underwater) ? 1 : 0`
///   (`getEmptyBubbleDelayDuration`, Gui.java:926-928); slots between the full
///   count and the delayed empty tail draw nothing (`None`).
pub(super) fn hud_air_bubble_icons(
    air: i32,
    max_air: i32,
    eye_in_water: bool,
) -> [Option<HudAirBubbleIcon>; HUD_AIR_BUBBLES_PER_ROW as usize] {
    let current_air = air.clamp(0, max_air);
    let full_bubbles = current_air_supply_bubble(current_air, max_air, -2);
    let popping_position = current_air_supply_bubble(current_air, max_air, 0);
    let empty_delay = if current_air != 0 && eye_in_water {
        1
    } else {
        0
    };
    let empty_bubbles = HUD_AIR_BUBBLES_PER_ROW as i32
        - current_air_supply_bubble(current_air, max_air, empty_delay);
    let is_popping = full_bubbles != popping_position;

    let mut icons = [None; HUD_AIR_BUBBLES_PER_ROW as usize];
    for (index, icon) in icons.iter_mut().enumerate() {
        let bubble = index as i32 + 1;
        *icon = if bubble <= full_bubbles {
            Some(HudAirBubbleIcon::Full)
        } else if is_popping && bubble == popping_position && eye_in_water {
            Some(HudAirBubbleIcon::Popping)
        } else if bubble > HUD_AIR_BUBBLES_PER_ROW as i32 - empty_bubbles {
            Some(HudAirBubbleIcon::Empty)
        } else {
            None
        };
    }
    icons
}

/// Per-bubble y wobble for the empty shells, mirroring vanilla
/// `Gui.extractAirBubbles` (Gui.java:910): while *all* 10 bubbles are empty
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
    if all_bubbles_empty && tick_count % 2 == 0 {
        let mut random = HudObfuscatedRandom::with_seed(seed);
        for offset in &mut offsets {
            *offset = random.next_int_bound(2) as i32;
        }
    }
    offsets
}

/// Vanilla `Gui.getVehicleMaxHearts` (Gui.java:725-737): a living vehicle's
/// heart count is `(int) (maxHealth + 0.5F) / 2` (Java float add, then int
/// truncation, then int division), capped at 30 hearts; the whole vehicle row
/// (and the food-row replacement) is skipped at 0 hearts.
pub(super) fn hud_vehicle_max_hearts(max_health: f32) -> u32 {
    let hearts = ((max_health + 0.5) as i32 / 2).min(HUD_VEHICLE_MAX_HEARTS);
    hearts.max(0) as u32
}

/// Which overlay one vehicle heart draws, mirroring `Gui.extractVehicleHealth`'s
/// per-heart branches (Gui.java:985-999): with `currentHealth =
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
/// `Gui.extractFood` (Gui.java:958-960): while `saturation <= 0` and
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
    if saturation_empty && tick_count % divisor == 0 {
        let mut random = HudObfuscatedRandom::with_seed(seed);
        for offset in &mut offsets {
            *offset = random.next_int_bound(3) as i32 - 1;
        }
    }
    offsets
}

/// Pen origin (line top-left, HUD pixels) of the centered experience-level
/// number, mirroring `ContextualBarRenderer.extractExperienceLevel`
/// (ContextualBarRenderer.java:37-38): `x = (guiWidth - font.width(str)) / 2`
/// (Java int division), `y = guiHeight - 24 - 9 - 2`. The caller offsets this
/// origin by `(±1, 0)/(0, ±1)` for the four black outline copies.
pub(super) fn hud_experience_level_text_origin(
    surface_size: PhysicalSize<u32>,
    text_width: u32,
) -> (f32, f32) {
    let gui_width = surface_size.width.max(1) as i32;
    let gui_height = surface_size.height.max(1) as i32;
    let x = (gui_width - text_width as i32) / 2;
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

/// Which tooltip sprite a planned segment belongs to. Vanilla draws the whole background sprite
/// first and the whole frame sprite second (`TooltipRenderUtil.extractTooltipBackground` blits
/// `tooltip/background` then `tooltip/frame`), so the planner preserves that source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudTooltipSpriteLayer {
    Background,
    Frame,
}

/// A tooltip nine-slice quad tagged with its owning sprite so the renderer can dispatch to the
/// background/frame GPU textures while keeping vanilla source order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudTooltipSpriteSegment {
    pub(super) layer: HudTooltipSpriteLayer,
    pub(super) rect: HudRect,
    pub(super) uv: HudUvRect,
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
    if !tile || tex_col_size == 0 || tex_row_size == 0 {
        // Single stretched quad (also the graceful fallback when a texture span is zero and so
        // cannot be tiled).
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
    let mut oy = 0;
    while oy < row_size {
        let tile_height = tex_row_size.min(row_size - oy);
        let mut ox = 0;
        while ox < col_size {
            let tile_width = tex_col_size.min(col_size - ox);
            segments.push(nine_slice_region_segment(
                target,
                sprite_width,
                sprite_height,
                col_offset + ox,
                row_offset + oy,
                tile_width,
                tile_height,
                tex_col_offset,
                tex_row_offset,
                tile_width,
                tile_height,
            ));
            ox += tex_col_size;
        }
        oy += tex_row_size;
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
pub(super) fn hud_inventory_tooltip_sprite_segments(
    target: HudRect,
    background: HudNineSliceScaling,
    frame: HudNineSliceScaling,
) -> Vec<HudTooltipSpriteSegment> {
    let mut segments = Vec::new();
    for (layer, scaling) in [
        (HudTooltipSpriteLayer::Background, background),
        (HudTooltipSpriteLayer::Frame, frame),
    ] {
        for region in nine_slice_segments(target, scaling) {
            segments.push(HudTooltipSpriteSegment {
                layer,
                rect: region.rect,
                uv: region.uv,
            });
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::super::{HudAsciiGlyph, HudTextStyle};
    use super::*;

    #[test]
    fn hud_quad_vertices_center_sprite_in_ndc() {
        let surface_size = PhysicalSize::new(1280, 720);
        let vertices = hud_quad_vertices(
            surface_size,
            centered_hud_rect(surface_size, 16, 8),
            full_uv_rect(),
            [1.0, 1.0, 1.0, 1.0],
        );
        assert_f32_near(vertices[0].position[0], -0.0125);
        assert_f32_near(vertices[0].position[1], 0.011111111);
        assert_f32_near(vertices[2].position[0], 0.0125);
        assert_f32_near(vertices[2].position[1], -0.011111111);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn hud_quad_vertices_maps_full_uv_rect() {
        let surface_size = PhysicalSize::new(1280, 720);
        let vertices = hud_quad_vertices(
            surface_size,
            centered_hud_rect(surface_size, 16, 8),
            HudUvRect {
                min: [0.25, 0.5],
                max: [0.75, 0.875],
            },
            [1.0, 1.0, 1.0, 1.0],
        );
        assert_eq!(vertices[0].uv, [0.25, 0.5]);
        assert_eq!(vertices[1].uv, [0.75, 0.5]);
        assert_eq!(vertices[2].uv, [0.75, 0.875]);
        assert_eq!(vertices[5].uv, [0.25, 0.875]);
        assert_eq!(vertices[0].local_uv, [0.0, 0.0]);
        assert_eq!(vertices[1].local_uv, [1.0, 0.0]);
        assert_eq!(vertices[2].local_uv, [1.0, 1.0]);
        assert_eq!(vertices[5].local_uv, [0.0, 1.0]);
    }

    #[test]
    fn hud_quad_vertices_maps_tint_to_all_vertices() {
        let surface_size = PhysicalSize::new(1280, 720);
        let tint = [0.25, 0.5, 0.75, 1.0];
        let vertices = hud_quad_vertices(
            surface_size,
            centered_hud_rect(surface_size, 16, 8),
            full_uv_rect(),
            tint,
        );

        assert!(vertices.iter().all(|vertex| vertex.tint == tint));
    }

    #[test]
    fn hud_rect_intersection_uv_span_returns_visible_rect_and_source_span() {
        let rect = HudRect {
            x: 100.0,
            y: 50.0,
            width: 16,
            height: 16,
        };
        let scissor = HudRect {
            x: 108.0,
            y: 46.0,
            width: 20,
            height: 12,
        };

        let (visible, uv_min, uv_max) = hud_rect_intersection_uv_span(rect, scissor).unwrap();

        assert_eq!(
            visible,
            HudRect {
                x: 108.0,
                y: 50.0,
                width: 8,
                height: 8,
            }
        );
        assert_eq!(uv_min, [0.5, 0.0]);
        assert_eq!(uv_max, [1.0, 0.5]);
        assert!(hud_rect_intersection_uv_span(
            rect,
            HudRect {
                x: 116.0,
                y: 50.0,
                width: 10,
                height: 10,
            }
        )
        .is_none());
    }

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

    #[test]
    fn gui_item_slot_placement_respects_non_square_item_rect_height() {
        let placement = gui_item_slot_placement(HudRect {
            x: 10.0,
            y: 20.0,
            width: 16,
            height: 24,
        });

        let top_left = placement * glam::Vec4::new(-0.5, 0.5, 0.0, 1.0);
        let bottom_right = placement * glam::Vec4::new(0.5, -0.5, 0.0, 1.0);
        assert_f32_near(top_left.x, 10.0);
        assert_f32_near(top_left.y, 20.0);
        assert_f32_near(bottom_right.x, 26.0);
        assert_f32_near(bottom_right.y, 44.0);
    }

    #[test]
    fn hud_layout_matches_vanilla_experience_bar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let bar = experience_bar_hud_rect(surface_size, 182, 5);
        assert_eq!(bar.x, 549.0);
        assert_eq!(bar.y, 691.0);
        assert_eq!(bar.width, 182);
        assert_eq!(bar.height, 5);

        assert_eq!(hud_contextual_bar_progress_width(0.0), 0);
        assert_eq!(hud_contextual_bar_progress_width(0.5), 91);
        assert_eq!(hud_contextual_bar_progress_width(1.0), 182);
    }

    #[test]
    fn hud_layout_matches_vanilla_boss_bar_positions() {
        let surface_size = PhysicalSize::new(320, 240);
        // x = guiWidth / 2 - 91 = 69, 182x5 sheet.
        let bar = boss_bar_hud_rect(surface_size, 12, 182);
        assert_eq!(bar.x, 69.0);
        assert_eq!(bar.y, 12.0);
        assert_eq!(bar.width, 182);
        assert_eq!(bar.height, 5);

        // Odd width: Java int division 321 / 2 = 160; a cropped fill keeps
        // the bar's left edge.
        let odd = boss_bar_hud_rect(PhysicalSize::new(321, 240), 31, 91);
        assert_eq!(odd.x, 69.0);
        assert_eq!(odd.y, 31.0);
        assert_eq!(odd.width, 91);

        // Name pen: (guiWidth / 2 - width / 2, barY - 9), int truncation.
        assert_eq!(
            hud_boss_bar_name_origin(surface_size, 31, 13),
            (154.0, 22.0)
        );
        assert_eq!(hud_boss_bar_name_origin(surface_size, 12, 12), (154.0, 3.0));
    }

    #[test]
    fn hud_boss_bar_rows_stack_and_truncate_at_a_third_of_the_screen() {
        // guiHeight / 3 = 80: rows 12, 31, 50, 69 fit; after drawing the
        // fourth bar the offset reaches 88 >= 80 and the rest is dropped.
        let surface_size = PhysicalSize::new(320, 240);
        assert_eq!(hud_boss_bar_rows(surface_size, 0), Vec::<i32>::new());
        assert_eq!(hud_boss_bar_rows(surface_size, 2), vec![12, 31]);
        assert_eq!(hud_boss_bar_rows(surface_size, 6), vec![12, 31, 50, 69]);
        // Vanilla draws first and checks after, so one bar always renders
        // even when y=12 already exceeds the cutoff.
        assert_eq!(hud_boss_bar_rows(PhysicalSize::new(320, 30), 3), vec![12]);
    }

    #[test]
    fn hud_boss_bar_progress_width_matches_vanilla_lerp_discrete() {
        // Mth.lerpDiscrete(p, 0, 182) = floor(p * 181) + (p > 0 ? 1 : 0).
        assert_eq!(hud_boss_bar_progress_width(0.0), 0);
        assert_eq!(hud_boss_bar_progress_width(0.001), 1);
        assert_eq!(hud_boss_bar_progress_width(0.5), 91);
        assert_eq!(hud_boss_bar_progress_width(1.0), 182);
        // Out-of-range/non-finite inputs (clamped upstream by the setter)
        // stay inside the sheet.
        assert_eq!(hud_boss_bar_progress_width(-1.0), 0);
        assert_eq!(hud_boss_bar_progress_width(2.0), 182);
        assert_eq!(hud_boss_bar_progress_width(f32::NAN), 0);

        // The crop UV is the left `width / 182` band of the sheet.
        assert_eq!(hud_boss_bar_fill_uv(91).min, [0.0, 0.0]);
        assert_eq!(hud_boss_bar_fill_uv(91).max, [0.5, 1.0]);
        assert_eq!(hud_boss_bar_fill_uv(182).max, [1.0, 1.0]);
        assert_eq!(hud_boss_bar_fill_uv(0).max, [0.0, 1.0]);
    }

    fn player_health(health: f32) -> HudPlayerHealth {
        HudPlayerHealth {
            health,
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
    fn hud_layout_places_player_hearts_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let instances = hud_player_heart_instances(surface_size, player_health(20.0));
        // Full 20 health / 20 max: 10 container backgrounds + 10 Normal fulls,
        // all on the single row baseline (720 - 39 = 681).
        let containers = container_hearts(&instances);
        assert_eq!(containers.len(), 10);
        let left = containers
            .iter()
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        let right = containers
            .iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        assert_eq!((left.x, left.y), (549.0, 681.0));
        assert_eq!((right.x, right.y), (621.0, 681.0));
        assert!(containers.iter().all(|i| i.y == 681.0 && !i.half));

        let fills = fill_hearts(&instances);
        assert_eq!(fills.len(), 10);
        assert!(fills
            .iter()
            .all(|i| i.kind == HudHeartKind::Normal && !i.half));
    }

    #[test]
    fn hud_player_heart_instances_split_half_and_empty() {
        let surface_size = PhysicalSize::new(1280, 720);
        // ceil(15) = 15 half-hearts: 7 full + 1 half (halves 0..=12 full, 14
        // half), the remaining containers draw background only.
        let fills = fill_hearts(&hud_player_heart_instances(
            surface_size,
            player_health(15.0),
        ));
        assert_eq!(fills.iter().filter(|i| !i.half).count(), 7);
        assert_eq!(fills.iter().filter(|i| i.half).count(), 1);
        assert!(fills.iter().all(|i| i.kind == HudHeartKind::Normal));

        // ceil(0.5) = 1 -> a single half heart; ceil rounds fractional health up.
        let fills = fill_hearts(&hud_player_heart_instances(
            surface_size,
            player_health(0.5),
        ));
        assert_eq!(fills.len(), 1);
        assert!(fills[0].half && fills[0].kind == HudHeartKind::Normal);

        // Zero health: only the 10 container backgrounds, no fills.
        let instances = hud_player_heart_instances(surface_size, player_health(0.0));
        assert_eq!(container_hearts(&instances).len(), 10);
        assert!(fill_hearts(&instances).is_empty());
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
    fn hud_player_heart_instances_stack_multiple_rows() {
        let surface_size = PhysicalSize::new(1280, 720);
        let mut health = player_health(40.0);
        health.max_health = 40.0; // 20 hearts -> 2 rows
        health.health = 40.0;
        let instances = hud_player_heart_instances(surface_size, health);
        assert_eq!(container_hearts(&instances).len(), 20);
        // numHealthRows = ceil(40/2/10) = 2 -> healthRowHeight = max(10-0,3) = 10.
        assert_eq!(hud_health_rows(40.0, 0.0), (2, 10));
        // Row 0 sits at 681, row 1 one healthRowHeight (10px) above at 671.
        let ys: std::collections::BTreeSet<i64> = container_hearts(&instances)
            .iter()
            .map(|i| i.y as i64)
            .collect();
        assert_eq!(ys.into_iter().collect::<Vec<_>>(), vec![671, 681]);
    }

    #[test]
    fn hud_player_heart_instances_regeneration_lifts_one_heart() {
        let surface_size = PhysicalSize::new(1280, 720);
        let mut health = player_health(20.0);
        health.regen = true;
        // heartOffsetIndex = tickCount % ceil(maxHealth + 5) = 3 % ceil(25) = 3.
        health.tick_count = 3;
        let instances = hud_player_heart_instances(surface_size, health);
        let lifted: Vec<_> = container_hearts(&instances)
            .into_iter()
            .filter(|i| i.y == 679.0)
            .collect();
        // Container index 3 = column 3 (x = 549 + 3*8 = 573) lifted by 2px.
        assert_eq!(lifted.len(), 1);
        assert_eq!(lifted[0].x, 573.0);

        // No regeneration: no heart is lifted.
        health.regen = false;
        let instances = hud_player_heart_instances(surface_size, health);
        assert!(container_hearts(&instances).iter().all(|i| i.y == 681.0));
    }

    #[test]
    fn hud_player_heart_instances_shake_only_at_low_health() {
        let surface_size = PhysicalSize::new(1280, 720);
        // 2 health (<= 4): every container jitters by nextInt(2) from the
        // tickCount * 312871 seed, drawn in descending container order.
        let mut health = player_health(2.0);
        health.tick_count = 5;
        let shaken = hud_player_heart_instances(surface_size, health);
        let seed = 5i32.wrapping_mul(312871) as i64 as u64;
        let mut rng = HudObfuscatedRandom::with_seed(seed);
        for container in container_hearts(&shaken) {
            assert_eq!(container.y, 681.0 + rng.next_int_bound(2) as f32);
        }
        // Deterministic for a given tick.
        assert_eq!(shaken, hud_player_heart_instances(surface_size, health));

        // Above the 4-half-heart threshold nothing shakes.
        let mut steady = player_health(6.0);
        steady.tick_count = 5;
        assert!(hud_player_heart_instances(surface_size, steady)
            .iter()
            .all(|i| i.y == 681.0));
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
    fn hud_layout_places_armor_row_one_row_above_the_hearts() {
        let surface_size = PhysicalSize::new(1280, 720);
        // Single health row (numHealthRows = 1): the armor row sits the fixed
        // 10px above the heart baseline (720 - 39 - 10 = 671), the same as the
        // pre-multi-row layout, regardless of healthRowHeight.
        let first = armor_hud_rect(surface_size, 0, 1, HUD_SINGLE_HEALTH_ROW_HEIGHT, 9, 9);
        let last = armor_hud_rect(surface_size, 9, 1, HUD_SINGLE_HEALTH_ROW_HEIGHT, 9, 9);
        assert_eq!(first.x, 549.0);
        assert_eq!(last.x, 621.0);
        assert_eq!(first.y, 671.0);
        assert_eq!(last.y, 671.0);
        // The heart baseline (yLineBase) is 720 - 39 = 681, one 10px gap below.
        assert_eq!(681.0 - first.y, 10.0);

        // Multi-row health lifts the armor row by (numHealthRows - 1) *
        // healthRowHeight on top of the 10px gap: 2 rows @ 10px -> 671 - 10 =
        // 661; 3 rows @ 9px -> 671 - 18 = 653.
        assert_eq!(armor_hud_rect(surface_size, 0, 2, 10, 9, 9).y, 661.0);
        assert_eq!(armor_hud_rect(surface_size, 0, 3, 9, 9, 9).y, 653.0);
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
        // Additional rows stack upward in 10px steps (yo -= 10, Gui.java:1001).
        assert_eq!(vehicle_heart_hud_rect(surface_size, 1, 0, 9, 9).y, 671.0);
        assert_eq!(vehicle_heart_hud_rect(surface_size, 2, 0, 9, 9).y, 661.0);
    }

    #[test]
    fn hud_layout_centers_vanilla_inventory_background() {
        let surface_size = PhysicalSize::new(1280, 720);
        let background = inventory_background_hud_rect(surface_size, 176, 166, 0, 0, 176, 166);
        assert_eq!(background.x, 552.0);
        assert_eq!(background.y, 277.0);
        assert_eq!(background.width, 176);
        assert_eq!(background.height, 166);

        let generic_top = inventory_background_hud_rect(surface_size, 176, 222, 0, 0, 176, 125);
        assert_eq!(generic_top.x, 552.0);
        assert_eq!(generic_top.y, 249.0);
        assert_eq!(generic_top.width, 176);
        assert_eq!(generic_top.height, 125);

        let generic_bottom = inventory_background_hud_rect(surface_size, 176, 222, 0, 125, 176, 96);
        assert_eq!(generic_bottom.x, 552.0);
        assert_eq!(generic_bottom.y, 374.0);
        assert_eq!(generic_bottom.width, 176);
        assert_eq!(generic_bottom.height, 96);

        let oversized_background =
            inventory_background_hud_rect(surface_size, 176, 166, 4, 6, 200, 180);
        assert_eq!(oversized_background.x, 556.0);
        assert_eq!(oversized_background.y, 283.0);
        assert_eq!(oversized_background.width, 200);
        assert_eq!(oversized_background.height, 180);
    }

    #[test]
    fn hud_layout_places_inventory_slot_icons_relative_to_screen_origin() {
        let surface_size = PhysicalSize::new(1280, 720);
        let item = inventory_slot_item_hud_rect(surface_size, 176, 166, 8, 84);
        assert_eq!(item.x, 560.0);
        assert_eq!(item.y, 361.0);
        assert_eq!(item.width, 16);
        assert_eq!(item.height, 16);

        let generic_item = inventory_slot_item_hud_rect(surface_size, 176, 222, 8, 197);
        assert_eq!(generic_item.x, 560.0);
        assert_eq!(generic_item.y, 446.0);

        let highlight = inventory_slot_highlight_hud_rect(surface_size, 176, 166, 8, 84);
        assert_eq!(highlight.x, 556.0);
        assert_eq!(highlight.y, 357.0);
        assert_eq!(highlight.width, 24);
        assert_eq!(highlight.height, 24);
    }

    #[test]
    fn hud_inventory_tooltip_layout_uses_vanilla_default_offsets() {
        let surface_size = PhysicalSize::new(320, 240);
        assert_eq!(hud_inventory_tooltip_text_height(1), Some(8));
        assert_eq!(hud_inventory_tooltip_text_height(2), Some(20));

        let background =
            hud_inventory_tooltip_background_hud_rect(surface_size, 176, 166, 8, 84, 36, 8);
        assert_eq!(background.x, 80.0);
        assert_eq!(background.y, 97.0);
        assert_eq!(background.width, 60);
        assert_eq!(background.height, 32);

        let glyph = HudAsciiGlyph {
            width: 8,
            height: 8,
            advance: 6,
            ..HudAsciiGlyph::default()
        };
        // Line 1 pen origin + a pen advance of 6 with the 1px shadow offset:
        // the glyph quad is the axis-aligned 8x8 cell at (99, 122).
        let (line_x, line_y) =
            hud_inventory_tooltip_line_origin(surface_size, 176, 166, 8, 84, 36, 8, 1);
        let quads = glyph.styled_quads(
            line_x + 6.0 + 1.0,
            line_y + 1.0,
            HudTextStyle::default(),
            false,
        );
        assert_eq!(quads.len(), 1);
        assert_eq!(
            quads[0].corners,
            [[99.0, 122.0], [99.0, 130.0], [107.0, 130.0], [107.0, 122.0],]
        );
    }

    #[test]
    fn hud_inventory_tooltip_layout_matches_vanilla_edge_fallbacks() {
        let surface_size = PhysicalSize::new(100, 100);

        let right = hud_inventory_tooltip_background_hud_rect(surface_size, 80, 80, 70, 20, 60, 8);
        assert_eq!(right.x, -4.0);
        assert_eq!(right.y, 6.0);
        assert_eq!(right.width, 84);

        let bottom =
            hud_inventory_tooltip_background_hud_rect(surface_size, 80, 80, 20, 96, 30, 20);
        assert_eq!(bottom.x, 30.0);
        assert_eq!(bottom.y, 65.0);
        assert_eq!(bottom.height, 44);
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

    #[test]
    fn nine_slice_segments_stretch_inner_splits_into_nine_vanilla_regions() {
        let target = HudRect {
            x: 100.0,
            y: 50.0,
            width: 64,
            height: 32,
        };
        let segments = nine_slice_segments(target, uniform_nine_slice(100, 9, true));

        // 100x100 sprite, border 9 (well below target/2), stretched inner => exactly nine quads.
        assert_eq!(segments.len(), 9);

        // Top-left corner: 9x9 at the target origin, sampling the sprite's top-left 9x9 texels.
        assert_eq!(
            segments[0],
            NineSliceSegment {
                rect: HudRect {
                    x: 100.0,
                    y: 50.0,
                    width: 9,
                    height: 9,
                },
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [9.0 / 100.0, 9.0 / 100.0],
                },
            }
        );

        // Center: the 46x14 inner span stretched across the sprite's 82x82 inner texels.
        assert_eq!(
            segments[4],
            NineSliceSegment {
                rect: HudRect {
                    x: 109.0,
                    y: 59.0,
                    width: 46,
                    height: 14,
                },
                uv: HudUvRect {
                    min: [9.0 / 100.0, 9.0 / 100.0],
                    max: [91.0 / 100.0, 91.0 / 100.0],
                },
            }
        );

        // Bottom-right corner anchored to the target's far edge and the sprite's far texels.
        assert_eq!(
            segments[8],
            NineSliceSegment {
                rect: HudRect {
                    x: 155.0,
                    y: 73.0,
                    width: 9,
                    height: 9,
                },
                uv: HudUvRect {
                    min: [91.0 / 100.0, 91.0 / 100.0],
                    max: [1.0, 1.0],
                },
            }
        );
    }

    #[test]
    fn nine_slice_segments_clamp_borders_and_drop_center_when_target_smaller_than_borders() {
        let target = HudRect {
            x: 20.0,
            y: 30.0,
            width: 10,
            height: 10,
        };
        let segments = nine_slice_segments(target, uniform_nine_slice(100, 9, true));

        // Vanilla clamps each border to target/2 (=5 here), collapsing the inner spans to zero, so
        // only the four 5x5 corner quads survive.
        assert_eq!(segments.len(), 4);
        assert_eq!(
            segments[0].rect,
            HudRect {
                x: 20.0,
                y: 30.0,
                width: 5,
                height: 5,
            }
        );
        assert_eq!(segments[0].uv.max, [5.0 / 100.0, 5.0 / 100.0]);
        assert_eq!(
            segments[3].rect,
            HudRect {
                x: 25.0,
                y: 35.0,
                width: 5,
                height: 5,
            }
        );
        assert_eq!(segments[3].uv.min, [95.0 / 100.0, 95.0 / 100.0]);
        assert_eq!(segments[3].uv.max, [1.0, 1.0]);
    }

    #[test]
    fn nine_slice_segments_tile_inner_repeats_and_clips_last_tile() {
        let target = HudRect {
            x: 0.0,
            y: 0.0,
            width: 28,
            height: 12,
        };
        let tiled = nine_slice_segments(target, uniform_nine_slice(20, 4, false));
        let stretched = nine_slice_segments(target, uniform_nine_slice(20, 4, true));

        // Inner span (20 wide) exceeds the sprite's inner texel span (12), so tiling emits an extra
        // clipped quad per horizontal inner slice; the stretched variant stays at nine.
        assert_eq!(stretched.len(), 9);
        assert_eq!(tiled.len(), 12);

        // Ordered row-major, the center row's tiles are indices 5 and 6.
        assert_eq!(
            tiled[5],
            NineSliceSegment {
                rect: HudRect {
                    x: 4.0,
                    y: 4.0,
                    width: 12,
                    height: 4,
                },
                uv: HudUvRect {
                    min: [4.0 / 20.0, 4.0 / 20.0],
                    max: [16.0 / 20.0, 8.0 / 20.0],
                },
            }
        );
        // Final center tile is clipped to the remaining 8px, sampling only 8 of the 12 inner texels.
        assert_eq!(
            tiled[6],
            NineSliceSegment {
                rect: HudRect {
                    x: 16.0,
                    y: 4.0,
                    width: 8,
                    height: 4,
                },
                uv: HudUvRect {
                    min: [4.0 / 20.0, 4.0 / 20.0],
                    max: [12.0 / 20.0, 8.0 / 20.0],
                },
            }
        );
    }

    #[test]
    fn hud_inventory_tooltip_sprite_segments_layer_background_then_frame_in_vanilla_order() {
        // The two real tooltip sprites: background tiles its inner (stretch_inner=false, border 9),
        // frame stretches its inner (stretch_inner=true, border 10). Both are 100x100.
        let background = uniform_nine_slice(100, 9, false);
        let frame = uniform_nine_slice(100, 10, true);
        let target = HudRect {
            x: 100.0,
            y: 50.0,
            width: 64,
            height: 32,
        };

        let segments = hud_inventory_tooltip_sprite_segments(target, background, frame);

        // Both layers' inner spans stay below their sprite inner texels, so each contributes nine
        // quads: eighteen total, all background quads first (vanilla blits background then frame).
        assert_eq!(segments.len(), 18);
        assert!(segments[..9]
            .iter()
            .all(|segment| segment.layer == HudTooltipSpriteLayer::Background));
        assert!(segments[9..]
            .iter()
            .all(|segment| segment.layer == HudTooltipSpriteLayer::Frame));

        // Both layers share the same padded rect origin; border widths differ per sprite (9 vs 10).
        assert_eq!(
            segments[0].rect,
            HudRect {
                x: 100.0,
                y: 50.0,
                width: 9,
                height: 9,
            }
        );
        assert_eq!(
            segments[9].rect,
            HudRect {
                x: 100.0,
                y: 50.0,
                width: 10,
                height: 10,
            }
        );

        // The background center is a single clipped tile (not stretched): its UV covers only the
        // 46x14 inner span, unlike a stretch that would reach the sprite's 82x82 inner texels.
        assert_eq!(
            segments[4].uv,
            HudUvRect {
                min: [9.0 / 100.0, 9.0 / 100.0],
                max: [55.0 / 100.0, 23.0 / 100.0],
            }
        );
    }

    #[test]
    fn hud_inventory_text_label_glyph_quad_uses_inventory_origin() {
        let glyph = HudAsciiGlyph {
            width: 8,
            height: 8,
            advance: 6,
            ..HudAsciiGlyph::default()
        };
        let (label_x, label_y) =
            hud_inventory_text_label_origin(PhysicalSize::new(320, 240), 176, 166, 62, 24);
        let quads = glyph.styled_quads(
            label_x + 12.0 + 1.0,
            label_y + 1.0,
            HudTextStyle::default(),
            false,
        );
        assert_eq!(quads.len(), 1);
        assert_eq!(
            quads[0].corners,
            [[147.0, 62.0], [147.0, 70.0], [155.0, 70.0], [155.0, 62.0],]
        );
    }

    #[test]
    fn glyph_quads_align_pages_on_the_vanilla_baseline() {
        // `GlyphBitmap.getTop()` = 7 - ascent: an accented-page glyph (é,
        // height 12, ascent 10) starts 3px above an ascii-page glyph (e,
        // ascent 7) drawn at the same pen position.
        let surface_size = PhysicalSize::new(320, 240);
        let ascii_e = HudAsciiGlyph {
            width: 8,
            height: 8,
            advance: 6,
            ascent: 7,
            ..HudAsciiGlyph::default()
        };
        let accented_e = HudAsciiGlyph {
            width: 9,
            height: 12,
            advance: 6,
            ascent: 10,
            ..HudAsciiGlyph::default()
        };

        let (label_x, label_y) = hud_inventory_text_label_origin(surface_size, 176, 166, 62, 24);
        let plain = HudTextStyle::default();
        let label_e = glyph_top_left(ascii_e, label_x, label_y, plain);
        let label_e_accent = glyph_top_left(accented_e, label_x, label_y, plain);
        assert_eq!(label_e_accent[1], label_e[1] - 3.0);

        let (tooltip_x, tooltip_y) =
            hud_inventory_tooltip_line_origin(surface_size, 176, 166, 8, 84, 36, 8, 0);
        let tooltip_e = glyph_top_left(ascii_e, tooltip_x, tooltip_y, plain);
        let tooltip_e_accent = glyph_top_left(accented_e, tooltip_x, tooltip_y, plain);
        assert_eq!(tooltip_e_accent[1], tooltip_e[1] - 3.0);
    }

    fn glyph_top_left(glyph: HudAsciiGlyph, x: f32, y: f32, style: HudTextStyle) -> [f32; 2] {
        glyph.styled_quads(x, y, style, false)[0].corners[0]
    }

    #[test]
    fn hud_item_count_digit_rect_uses_vanilla_item_count_position() {
        let surface_size = PhysicalSize::new(1280, 720);
        let item = hotbar_item_hud_rect(surface_size, 0);
        let glyph = HudDigitGlyph {
            width: 8,
            height: 8,
            advance: 6,
            ..HudDigitGlyph::default()
        };

        let digit = hud_item_count_digit_hud_rect(item, 12, 0, 0.0, glyph);
        assert_eq!(digit.x, 557.0);
        assert_eq!(digit.y, 710.0);
        assert_eq!(digit.width, 8);
        assert_eq!(digit.height, 8);

        let shadow = hud_item_count_digit_hud_rect(item, 12, 6, 1.0, glyph);
        assert_eq!(shadow.x, 564.0);
        assert_eq!(shadow.y, 711.0);
    }

    #[test]
    fn hud_health_rows_scale_rows_and_compress_height() {
        // 20 max / no absorption -> 1 row (healthRowHeight is unused for a
        // single row but follows max(10-(1-2),3) = 11).
        assert_eq!(hud_health_rows(20.0, 0.0), (1, 11));
        // Health Boost / absorption push into more rows and shrink the step.
        assert_eq!(hud_health_rows(40.0, 0.0), (2, 10));
        assert_eq!(hud_health_rows(20.0, 20.0), (2, 10));
        assert_eq!(hud_health_rows(60.0, 0.0), (3, 9));
        // The row height floors at 3 for very tall stacks.
        assert_eq!(hud_health_rows(200.0, 0.0).1, 3);
        // Absorption is ceiled before adding: 20 max + ceil(0.5) = 1 -> 21 HP,
        // ceil(21 / 2 / 10) = 2 rows.
        assert_eq!(hud_health_rows(20.0, 0.5), (2, 10));

        // hud_health_row_geometry resolves currentHealth/maxHealth first:
        // maxHealth falls back to currentHealth when the attribute is lower.
        let mut health = player_health(30.0);
        health.max_health = 20.0; // ceil(30) forces maxHealth up to 30
        assert_eq!(hud_health_row_geometry(health), (2, 10));
    }

    #[test]
    fn hud_food_fill_uses_server_food_halves() {
        assert_eq!(hud_food_fill(0, 0), HudIconFill::Empty);
        assert_eq!(hud_food_fill(5, 0), HudIconFill::Full);
        assert_eq!(hud_food_fill(5, 2), HudIconFill::Half);
        assert_eq!(hud_food_fill(6, 2), HudIconFill::Full);
        assert_eq!(hud_food_fill(20, 9), HudIconFill::Full);
        assert_eq!(hud_food_fill(25, 9), HudIconFill::Full);
    }

    #[test]
    fn hud_armor_fill_splits_full_half_empty_on_the_armor_value() {
        // Vanilla `Gui.extractArmor` compares `i * 2 + 1` to the armor value.
        // armor = 7 -> icons 0..3 full (thresholds 1,3,5 < 7), icon 3 half
        // (threshold 7 == 7), icons 4..9 empty (thresholds 9.. > 7): 3 full + 1
        // half + 6 empty.
        let fills: Vec<HudIconFill> = (0..HUD_ARMOR_ICONS_PER_ROW)
            .map(|index| hud_armor_fill(7, index))
            .collect();
        assert_eq!(
            fills,
            vec![
                HudIconFill::Full,
                HudIconFill::Full,
                HudIconFill::Full,
                HudIconFill::Half,
                HudIconFill::Empty,
                HudIconFill::Empty,
                HudIconFill::Empty,
                HudIconFill::Empty,
                HudIconFill::Empty,
                HudIconFill::Empty,
            ]
        );
        assert_eq!(fills.iter().filter(|&&f| f == HudIconFill::Full).count(), 3);
        assert_eq!(fills.iter().filter(|&&f| f == HudIconFill::Half).count(), 1);
        assert_eq!(
            fills.iter().filter(|&&f| f == HudIconFill::Empty).count(),
            6
        );

        // A full 20-point armor bar is 10 full icons, no half.
        assert!((0..HUD_ARMOR_ICONS_PER_ROW)
            .all(|index| hud_armor_fill(20, index) == HudIconFill::Full));
        // An odd armor of 1 is a single half icon in slot 0.
        assert_eq!(hud_armor_fill(1, 0), HudIconFill::Half);
        assert_eq!(hud_armor_fill(1, 1), HudIconFill::Empty);
    }

    #[test]
    fn hud_air_bubbles_visible_only_underwater_or_below_max() {
        // Full supply on land: hidden (vanilla Gui.java:891).
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
        // `isUnderWater`, Gui.java:906) and drops the refill delay (delay = 0
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
    fn hud_air_bubble_wobble_needs_all_empty_and_an_even_tick() {
        // Not all empty: never wobbles.
        assert_eq!(hud_air_bubble_wobble_offsets(false, 4, 7), [0; 10]);
        // All empty on an odd tick: no wobble (vanilla `tickCount % 2 == 0`).
        assert_eq!(hud_air_bubble_wobble_offsets(true, 3, 7), [0; 10]);
        // All empty on an even tick: each shell shifts by a deterministic
        // `nextInt(2)` from the frame seed's LCG.
        let offsets = hud_air_bubble_wobble_offsets(true, 4, 7);
        let mut random = HudObfuscatedRandom::with_seed(7);
        let expected: [i32; 10] = std::array::from_fn(|_| random.next_int_bound(2) as i32);
        assert_eq!(offsets, expected);
        assert!(offsets.iter().all(|&offset| (0..=1).contains(&offset)));
        // Same seed -> same offsets: a redraw of the same frame is stable.
        assert_eq!(
            hud_air_bubble_wobble_offsets(true, 4, 42),
            hud_air_bubble_wobble_offsets(true, 4, 42)
        );
    }

    #[test]
    fn hud_vehicle_max_hearts_rounds_and_caps_like_the_gui() {
        // Vanilla `(int)(maxHealth + 0.5F) / 2`, capped at 30 (Gui.java:725-737).
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

    #[test]
    fn hud_food_jitter_shakes_only_when_saturation_empty_and_the_tick_hits() {
        // Saturation above zero never shakes, whatever the tick.
        assert_eq!(hud_food_jitter_offsets(18, false, 0, 7), [0; 10]);
        assert_eq!(hud_food_jitter_offsets(18, false, 55, 7), [0; 10]);

        // Saturation empty but the tick misses the `food*3+1` modulo (food = 18
        // -> divisor 55; 1 % 55 != 0): still no shake.
        assert_eq!(hud_food_jitter_offsets(18, true, 1, 7), [0; 10]);

        // Saturation empty and the tick hits the modulo: every icon shifts by a
        // deterministic `nextInt(3) - 1` drawn from the frame seed's LCG.
        let offsets = hud_food_jitter_offsets(18, true, 55, 7);
        let mut random = HudObfuscatedRandom::with_seed(7);
        let expected: [i32; 10] = std::array::from_fn(|_| random.next_int_bound(3) as i32 - 1);
        assert_eq!(offsets, expected);
        assert!(offsets.iter().all(|&offset| (-1..=1).contains(&offset)));
        assert!(
            offsets.iter().any(|&offset| offset != 0),
            "seed 7 shakes at least one icon"
        );

        // food = 0 -> divisor 1, so any tick hits while saturation is empty.
        let starving = hud_food_jitter_offsets(0, true, 3, 11);
        assert!(starving.iter().all(|&offset| (-1..=1).contains(&offset)));
        // Same seed -> same offsets, so a redraw of the same frame is stable.
        assert_eq!(
            hud_food_jitter_offsets(0, true, 3, 42),
            hud_food_jitter_offsets(0, true, 3, 42)
        );
    }

    #[test]
    fn hud_food_rect_applies_the_per_icon_shake_offset() {
        let surface_size = PhysicalSize::new(400, 300);
        let base = food_hud_rect(surface_size, 0, 9, 9, 0);
        let shaken_up = food_hud_rect(surface_size, 0, 9, 9, -1);
        let shaken_down = food_hud_rect(surface_size, 0, 9, 9, 1);
        assert_eq!(shaken_up.y, base.y - 1.0);
        assert_eq!(shaken_down.y, base.y + 1.0);
        // Only y moves; x/size are untouched.
        assert_eq!(shaken_up.x, base.x);
        assert_eq!(shaken_up.width, base.width);
    }

    #[test]
    fn hud_experience_level_text_origin_centers_above_the_bar() {
        // Vanilla: x = (guiWidth - width) / 2 (int division), y = guiHeight - 35.
        assert_eq!(
            hud_experience_level_text_origin(PhysicalSize::new(400, 300), 12),
            (194.0, 265.0)
        );
        // Odd numerator truncates toward zero, matching Java int division.
        assert_eq!(
            hud_experience_level_text_origin(PhysicalSize::new(401, 300), 10),
            (195.0, 265.0)
        );
        assert_eq!(
            hud_experience_level_text_origin(PhysicalSize::new(400, 300), 13),
            (193.0, 265.0)
        );
    }

    fn assert_f32_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000001,
            "actual {actual} expected {expected}"
        );
    }

    #[test]
    fn hud_item_durability_bar_rect_uses_vanilla_item_bar_position() {
        let rect = hud_item_durability_bar_rect(
            HudRect {
                x: 10.0,
                y: 20.0,
                width: 16,
                height: 16,
            },
            13,
            2,
        );

        assert_eq!(rect.x, 12.0);
        assert_eq!(rect.y, 33.0);
        assert_eq!(rect.width, 13);
        assert_eq!(rect.height, 2);
    }

    #[test]
    fn hud_item_cooldown_rect_uses_vanilla_fill_position() {
        let item = HudRect {
            x: 10.0,
            y: 20.0,
            width: 16,
            height: 16,
        };

        assert!(hud_item_cooldown_rect(item, 0.0).is_none());
        assert!(hud_item_cooldown_rect(item, -1.0).is_none());
        let partial = hud_item_cooldown_rect(item, 0.5).unwrap();
        assert_eq!(partial.x, 10.0);
        assert_eq!(partial.y, 28.0);
        assert_eq!(partial.width, 16);
        assert_eq!(partial.height, 8);
        let clamped = hud_item_cooldown_rect(item, 2.0).unwrap();
        assert_eq!(clamped.x, 10.0);
        assert_eq!(clamped.y, 20.0);
        assert_eq!(clamped.width, 16);
        assert_eq!(clamped.height, 16);
    }

    fn full_uv_rect() -> HudUvRect {
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }
    }
}
