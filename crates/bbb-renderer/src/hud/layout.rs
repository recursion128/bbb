use winit::dpi::PhysicalSize;

use super::{HudDigitGlyph, HudUvRect, HudVertex, HUD_HOTBAR_SLOTS};

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
const HUD_INVENTORY_IMAGE_WIDTH: u32 = 176;
const HUD_INVENTORY_IMAGE_HEIGHT: u32 = 166;
const HUD_INVENTORY_ITEM_SIZE: u32 = 16;
const HUD_INVENTORY_SLOT_HIGHLIGHT_SIZE: u32 = 24;
const HUD_INVENTORY_SLOT_HIGHLIGHT_OFFSET: f32 = -4.0;

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
    width: u32,
    height: u32,
) -> HudRect {
    let (x, y) = inventory_screen_origin(surface_size);
    HudRect {
        x,
        y,
        width,
        height,
    }
}

pub(super) fn inventory_slot_item_hud_rect(
    surface_size: PhysicalSize<u32>,
    slot_x: i32,
    slot_y: i32,
) -> HudRect {
    let (x, y) = inventory_screen_origin(surface_size);
    HudRect {
        x: x + slot_x as f32,
        y: y + slot_y as f32,
        width: HUD_INVENTORY_ITEM_SIZE,
        height: HUD_INVENTORY_ITEM_SIZE,
    }
}

pub(super) fn inventory_slot_highlight_hud_rect(
    surface_size: PhysicalSize<u32>,
    slot_x: i32,
    slot_y: i32,
) -> HudRect {
    let (x, y) = inventory_screen_origin(surface_size);
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

fn inventory_screen_origin(surface_size: PhysicalSize<u32>) -> (f32, f32) {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    (
        (surface_width - HUD_INVENTORY_IMAGE_WIDTH as f32) * 0.5,
        (surface_height - HUD_INVENTORY_IMAGE_HEIGHT as f32) * 0.5,
    )
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
        },
        HudVertex {
            position: [right, top],
            uv: [uv.max[0], uv.min[1]],
            tint,
        },
        HudVertex {
            position: [right, bottom],
            uv: uv.max,
            tint,
        },
        HudVertex {
            position: [left, top],
            uv: uv.min,
            tint,
        },
        HudVertex {
            position: [right, bottom],
            uv: uv.max,
            tint,
        },
        HudVertex {
            position: [left, bottom],
            uv: [uv.min[0], uv.max[1]],
            tint,
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
        let background = inventory_background_hud_rect(surface_size, 176, 166);
        assert_eq!(background.x, 552.0);
        assert_eq!(background.y, 277.0);
        assert_eq!(background.width, 176);
        assert_eq!(background.height, 166);

        let oversized_background = inventory_background_hud_rect(surface_size, 200, 180);
        assert_eq!(oversized_background.x, 552.0);
        assert_eq!(oversized_background.y, 277.0);
        assert_eq!(oversized_background.width, 200);
        assert_eq!(oversized_background.height, 180);
    }

    #[test]
    fn hud_layout_places_inventory_slot_icons_relative_to_screen_origin() {
        let surface_size = PhysicalSize::new(1280, 720);
        let item = inventory_slot_item_hud_rect(surface_size, 8, 84);
        assert_eq!(item.x, 560.0);
        assert_eq!(item.y, 361.0);
        assert_eq!(item.width, 16);
        assert_eq!(item.height, 16);

        let highlight = inventory_slot_highlight_hud_rect(surface_size, 8, 84);
        assert_eq!(highlight.x, 556.0);
        assert_eq!(highlight.y, 357.0);
        assert_eq!(highlight.width, 24);
        assert_eq!(highlight.height, 24);
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

    fn full_uv_rect() -> HudUvRect {
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        }
    }
}
