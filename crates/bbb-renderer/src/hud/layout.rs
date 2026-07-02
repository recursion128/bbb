use glam::{Mat4, Vec3};
use winit::dpi::PhysicalSize;

use super::{HudAsciiGlyph, HudDigitGlyph, HudUvRect, HudVertex, HUD_HOTBAR_SLOTS};

const HUD_HOTBAR_WIDTH: u32 = 182;
const HUD_HOTBAR_HEIGHT: u32 = 22;
const HUD_HOTBAR_SLOT_SPACING: f32 = 20.0;
const HUD_HOTBAR_ITEM_SIZE: u32 = 16;
const HUD_EXPERIENCE_BAR_WIDTH: u32 = 182;
const HUD_EXPERIENCE_BAR_HEIGHT: u32 = 5;
const HUD_EXPERIENCE_MARGIN_BOTTOM: f32 = 24.0;
pub(super) const HUD_HEARTS_PER_ROW: u32 = 10;
const HUD_HEART_SPACING: f32 = 8.0;
pub(super) const HUD_FOOD_ICONS_PER_ROW: u32 = 10;
const HUD_FOOD_SPACING: f32 = 8.0;
const HUD_INVENTORY_ITEM_SIZE: u32 = 16;
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

#[derive(Debug, Clone, Copy)]
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

pub(super) fn hud_experience_progress_width(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 183.0).floor() as u32).min(HUD_EXPERIENCE_BAR_WIDTH)
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

/// The model→GUI-pixel placement for a 3D inventory item rendered in `rect` (vanilla `GuiItemAtlas`:
/// `translate(slot_center, 0) · scale(slot_px, -slot_px, slot_px)`). Composed with the item's GUI
/// display transform and projected by the GUI ortho camera, it seats a `0..1` (post-display) model in the
/// slot's pixel rect, flipping Y to GUI space.
pub(super) fn gui_item_slot_placement(rect: HudRect) -> Mat4 {
    let size = rect.width as f32;
    let center_x = rect.x + size / 2.0;
    let center_y = rect.y + rect.height as f32 / 2.0;
    Mat4::from_translation(Vec3::new(center_x, center_y, 0.0))
        * Mat4::from_scale(Vec3::new(size, -size, size))
}

pub(super) fn heart_hud_rect(
    surface_size: PhysicalSize<u32>,
    index: u32,
    width: u32,
    height: u32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 - 91.0 + index as f32 * HUD_HEART_SPACING,
        y: surface_height - 39.0,
        width,
        height,
    }
}

pub(super) fn food_hud_rect(
    surface_size: PhysicalSize<u32>,
    index: u32,
    width: u32,
    height: u32,
) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_FOOD_SPACING - width as f32,
        y: surface_height - 39.0,
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

pub(super) fn hud_inventory_tooltip_text_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    anchor_x: i32,
    anchor_y: i32,
    text_width: u32,
    text_height: u32,
    line_index: usize,
    pen_x: u32,
    shadow_offset: f32,
    glyph: HudAsciiGlyph,
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
        x: x + pen_x as f32 + shadow_offset,
        y: y + tooltip_line_y(line_index) as f32 + shadow_offset,
        width: glyph.width,
        height: glyph.height,
    }
}

pub(super) fn hud_inventory_text_label_glyph_hud_rect(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    label_x: i32,
    label_y: i32,
    pen_x: u32,
    shadow_offset: f32,
    glyph: HudAsciiGlyph,
) -> HudRect {
    let (origin_x, origin_y) = inventory_screen_origin(surface_size, screen_width, screen_height);
    HudRect {
        x: origin_x + label_x as f32 + pen_x as f32 + shadow_offset,
        y: origin_y + label_y as f32 + shadow_offset,
        width: glyph.width,
        height: glyph.height,
    }
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

pub(super) fn hud_heart_fill(health: f32, index: u32) -> HudIconFill {
    let current_halves = health.ceil().clamp(0.0, (HUD_HEARTS_PER_ROW * 2) as f32) as u32;
    let start_half = index * 2;
    if start_half >= current_halves {
        HudIconFill::Empty
    } else if start_half + 1 == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Full
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
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let left = x0 / width * 2.0 - 1.0;
    let right = x1 / width * 2.0 - 1.0;
    let top = 1.0 - y0 / height * 2.0;
    let bottom = 1.0 - y1 / height * 2.0;
    [
        HudVertex {
            position: [left, top],
            uv: uv.min,
            tint,
            local_uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, top],
            uv: [uv.max[0], uv.min[1]],
            tint,
            local_uv: [1.0, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: uv.max,
            tint,
            local_uv: [1.0, 1.0],
        },
        HudVertex {
            position: [left, top],
            uv: uv.min,
            tint,
            local_uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: uv.max,
            tint,
            local_uv: [1.0, 1.0],
        },
        HudVertex {
            position: [left, bottom],
            uv: [uv.min[0], uv.max[1]],
            tint,
            local_uv: [0.0, 1.0],
        },
    ]
}

#[cfg(test)]
mod tests {
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
    fn hud_layout_matches_vanilla_experience_bar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let bar = experience_bar_hud_rect(surface_size, 182, 5);
        assert_eq!(bar.x, 549.0);
        assert_eq!(bar.y, 691.0);
        assert_eq!(bar.width, 182);
        assert_eq!(bar.height, 5);

        assert_eq!(hud_experience_progress_width(0.0), 0);
        assert_eq!(hud_experience_progress_width(0.5), 91);
        assert_eq!(hud_experience_progress_width(1.0), 182);
    }

    #[test]
    fn hud_layout_places_player_hearts_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = heart_hud_rect(surface_size, 0, 9, 9);
        let last = heart_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 549.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 621.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_layout_places_food_icons_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = food_hud_rect(surface_size, 0, 9, 9);
        let last = food_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 722.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(last.y, 681.0);
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
        let text = hud_inventory_tooltip_text_hud_rect(
            surface_size,
            176,
            166,
            8,
            84,
            36,
            8,
            1,
            6,
            1.0,
            glyph,
        );
        assert_eq!(text.x, 99.0);
        assert_eq!(text.y, 122.0);
        assert_eq!(text.width, 8);
        assert_eq!(text.height, 8);
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

    #[test]
    fn hud_inventory_text_label_glyph_rect_uses_inventory_origin() {
        let glyph = HudAsciiGlyph {
            width: 8,
            height: 8,
            advance: 6,
            ..HudAsciiGlyph::default()
        };
        let rect = hud_inventory_text_label_glyph_hud_rect(
            PhysicalSize::new(320, 240),
            176,
            166,
            62,
            24,
            12,
            1.0,
            glyph,
        );

        assert_eq!(rect.x, 147.0);
        assert_eq!(rect.y, 62.0);
        assert_eq!(rect.width, 8);
        assert_eq!(rect.height, 8);
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
    fn hud_heart_fill_uses_ceiled_health_halves() {
        assert_eq!(hud_heart_fill(0.0, 0), HudIconFill::Empty);
        assert_eq!(hud_heart_fill(5.0, 0), HudIconFill::Full);
        assert_eq!(hud_heart_fill(5.0, 2), HudIconFill::Half);
        assert_eq!(hud_heart_fill(5.5, 2), HudIconFill::Full);
        assert_eq!(hud_heart_fill(20.0, 9), HudIconFill::Full);
        assert_eq!(hud_heart_fill(25.0, 9), HudIconFill::Full);
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
