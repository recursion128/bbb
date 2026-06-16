use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::Renderer;

mod gpu;
mod layout;

use self::gpu::create_hud_sprite_gpu;
pub(super) use self::gpu::{create_hud_bind_group_layout, create_hud_pipeline, HudSpriteGpu};
use self::layout::{
    centered_hud_rect, experience_bar_hud_rect, food_hud_rect, heart_hud_rect, hotbar_hud_rect,
    hotbar_item_hud_rect, hotbar_selection_hud_rect, hud_experience_progress_width, hud_food_fill,
    hud_heart_fill, hud_quad_vertices, HudIconFill, HudRect, HUD_FOOD_ICONS_PER_ROW,
    HUD_HEARTS_PER_ROW,
};

pub const HUD_HOTBAR_SLOTS: usize = 9;
const HUD_TINT_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudIconLayer {
    pub uv: HudUvRect,
    pub tint: [f32; 4],
}

impl HudIconLayer {
    pub fn new(uv: HudUvRect, tint: [f32; 4]) -> Self {
        Self { uv, tint }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudItemIcon {
    pub layers: Vec<HudIconLayer>,
}

impl HudItemIcon {
    pub fn single(uv: HudUvRect) -> Self {
        Self {
            layers: vec![HudIconLayer::new(uv, HUD_TINT_WHITE)],
        }
    }
}

pub(super) struct HudDrawCommand<'a> {
    pub(super) sprite: &'a HudSpriteGpu,
    pub(super) start: u32,
    pub(super) end: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct HudVertex {
    position: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
}

impl Renderer {
    pub fn upload_hud_crosshair(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        let crosshair = self.upload_hud_sprite(width, height, rgba)?;
        self.counters.hud_crosshair_width = width;
        self.counters.hud_crosshair_height = height;
        self.hud_crosshair = Some(crosshair);
        Ok(())
    }

    pub fn upload_hud_hotbar(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_hotbar = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hotbar_selection(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hotbar_selection = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_item_atlas(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_item_atlas = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_container(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_container = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_empty(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn set_hud_health(&mut self, health: Option<f32>) {
        self.hud_health = health.filter(|health| health.is_finite());
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        self.hud_food = food;
    }

    pub fn set_hud_experience_progress(&mut self, progress: Option<f32>) {
        self.hud_experience_progress_value = progress
            .filter(|progress| progress.is_finite())
            .map(|progress| progress.clamp(0.0, 1.0));
    }

    pub fn set_hud_selected_slot(&mut self, slot: u8) {
        self.hud_selected_slot = slot.min((HUD_HOTBAR_SLOTS - 1) as u8);
    }

    pub fn set_hud_hotbar_item_uvs(&mut self, uvs: [Option<HudUvRect>; HUD_HOTBAR_SLOTS]) {
        self.hud_hotbar_item_icons = uvs.map(|uv| uv.and_then(sanitize_hud_hotbar_item_uv));
    }

    pub fn set_hud_hotbar_item_icons(&mut self, icons: [Option<HudItemIcon>; HUD_HOTBAR_SLOTS]) {
        self.hud_hotbar_item_icons = icons.map(|icon| icon.and_then(sanitize_hud_item_icon));
    }

    fn upload_hud_sprite(&self, width: u32, height: u32, rgba: &[u8]) -> Result<HudSpriteGpu> {
        create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )
    }

    pub(super) fn collect_hud_draws(&self) -> (Vec<HudVertex>, Vec<HudDrawCommand<'_>>) {
        let mut vertices = Vec::new();
        let mut commands = Vec::new();
        let surface_size = self.surface_size();

        if let Some(crosshair) = &self.hud_crosshair {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                crosshair,
                surface_size,
                centered_hud_rect(surface_size, crosshair.width, crosshair.height),
            );
        }

        if let Some(hotbar) = &self.hud_hotbar {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                hotbar,
                surface_size,
                hotbar_hud_rect(surface_size, hotbar.width, hotbar.height),
            );
        }
        if let Some(selection) = &self.hud_hotbar_selection {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                selection,
                surface_size,
                hotbar_selection_hud_rect(
                    surface_size,
                    self.hud_selected_slot,
                    selection.width,
                    selection.height,
                ),
            );
        }

        if let Some(atlas) = &self.hud_item_atlas {
            for (slot, icon) in self.hud_hotbar_item_icons.iter().enumerate() {
                if let Some(icon) = icon {
                    for layer in &icon.layers {
                        push_hud_draw_with_uv_and_tint(
                            &mut vertices,
                            &mut commands,
                            atlas,
                            surface_size,
                            hotbar_item_hud_rect(surface_size, slot),
                            layer.uv,
                            layer.tint,
                        );
                    }
                }
            }
        }

        if let (Some(progress), Some(background)) = (
            self.hud_experience_progress_value,
            &self.hud_experience_background,
        ) {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                background,
                surface_size,
                experience_bar_hud_rect(surface_size, background.width, background.height),
            );

            let progress_width = hud_experience_progress_width(progress);
            if progress_width > 0 {
                if let Some(progress_sprite) = &self.hud_experience_progress {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        progress_sprite,
                        surface_size,
                        experience_bar_hud_rect(
                            surface_size,
                            progress_width,
                            progress_sprite.height,
                        ),
                        HudUvRect {
                            min: [0.0, 0.0],
                            max: [
                                (progress_width as f32 / progress_sprite.width.max(1) as f32)
                                    .clamp(0.0, 1.0),
                                1.0,
                            ],
                        },
                    );
                }
            }
        }

        if let (Some(health), Some(container)) = (self.hud_health, &self.hud_heart_container) {
            for index in 0..HUD_HEARTS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    container,
                    surface_size,
                    heart_hud_rect(surface_size, index, container.width, container.height),
                );
            }

            for index in 0..HUD_HEARTS_PER_ROW {
                let sprite = match hud_heart_fill(health, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_heart_half.as_ref(),
                    HudIconFill::Full => self.hud_heart_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        heart_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        if let (Some(food), Some(empty)) = (self.hud_food, &self.hud_food_empty) {
            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    empty,
                    surface_size,
                    food_hud_rect(surface_size, index, empty.width, empty.height),
                );
            }

            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                let sprite = match hud_food_fill(food, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_food_half.as_ref(),
                    HudIconFill::Full => self.hud_food_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        food_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        (vertices, commands)
    }
}

fn push_hud_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TINT_WHITE,
    );
}

fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect,
        uv,
        HUD_TINT_WHITE,
    );
}

fn push_hud_draw_with_uv_and_tint<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv: HudUvRect,
    tint: [f32; 4],
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(surface_size, rect, uv, tint));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand { sprite, start, end });
}

fn sanitize_hud_uv_rect(rect: HudUvRect) -> Option<HudUvRect> {
    let components = [rect.min[0], rect.min[1], rect.max[0], rect.max[1]];
    if !components.iter().all(|component| component.is_finite()) {
        return None;
    }

    let min_x = rect.min[0].clamp(0.0, 1.0);
    let min_y = rect.min[1].clamp(0.0, 1.0);
    let max_x = rect.max[0].clamp(0.0, 1.0);
    let max_y = rect.max[1].clamp(0.0, 1.0);
    Some(HudUvRect {
        min: [min_x.min(max_x), min_y.min(max_y)],
        max: [min_x.max(max_x), min_y.max(max_y)],
    })
}

fn sanitize_hud_hotbar_item_uv(uv: HudUvRect) -> Option<HudItemIcon> {
    sanitize_hud_item_icon(HudItemIcon::single(uv))
}

fn sanitize_hud_item_icon(icon: HudItemIcon) -> Option<HudItemIcon> {
    let layers = icon
        .layers
        .into_iter()
        .filter_map(sanitize_hud_icon_layer)
        .collect::<Vec<_>>();
    (!layers.is_empty()).then_some(HudItemIcon { layers })
}

fn sanitize_hud_icon_layer(layer: HudIconLayer) -> Option<HudIconLayer> {
    if !layer.tint.iter().all(|component| component.is_finite()) {
        return None;
    }
    Some(HudIconLayer {
        uv: sanitize_hud_uv_rect(layer.uv)?,
        tint: layer.tint.map(|component| component.clamp(0.0, 1.0)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_hud_uv_rect_discards_non_finite_values() {
        let rect = HudUvRect {
            min: [0.0, f32::NAN],
            max: [1.0, 1.0],
        };
        assert_eq!(sanitize_hud_uv_rect(rect), None);

        let rect = HudUvRect {
            min: [0.0, 0.0],
            max: [f32::INFINITY, 1.0],
        };
        assert_eq!(sanitize_hud_uv_rect(rect), None);
    }

    #[test]
    fn sanitize_hud_uv_rect_clamps_and_orders_bounds() {
        let rect = HudUvRect {
            min: [1.25, 0.75],
            max: [-0.5, 0.25],
        };
        assert_eq!(
            sanitize_hud_uv_rect(rect),
            Some(HudUvRect {
                min: [0.0, 0.25],
                max: [1.0, 0.75],
            })
        );
    }

    #[test]
    fn sanitize_hud_hotbar_item_uv_wraps_legacy_api_as_single_white_layer() {
        let icon = sanitize_hud_hotbar_item_uv(HudUvRect {
            min: [1.25, 0.75],
            max: [-0.5, 0.25],
        })
        .expect("clamped legacy UV should remain");

        assert_eq!(
            icon,
            HudItemIcon {
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.25],
                        max: [1.0, 0.75],
                    },
                    HUD_TINT_WHITE,
                )],
            }
        );

        assert_eq!(
            sanitize_hud_hotbar_item_uv(HudUvRect {
                min: [0.0, f32::NAN],
                max: [1.0, 1.0],
            }),
            None
        );
    }

    #[test]
    fn sanitize_hud_item_icon_preserves_layer_order_and_clamps_tint() {
        let first = HudIconLayer::new(
            HudUvRect {
                min: [0.0, 0.0],
                max: [0.25, 0.25],
            },
            [-1.0, 0.25, 1.5, 1.0],
        );
        let second = HudIconLayer::new(
            HudUvRect {
                min: [0.25, 0.25],
                max: [0.5, 0.5],
            },
            [0.75, 0.5, 0.25, 0.0],
        );
        let icon = sanitize_hud_item_icon(HudItemIcon {
            layers: vec![first, second],
        })
        .expect("valid icon layers should remain");

        assert_eq!(icon.layers.len(), 2);
        assert_eq!(icon.layers[0].uv.min, [0.0, 0.0]);
        assert_eq!(icon.layers[0].uv.max, [0.25, 0.25]);
        assert_eq!(icon.layers[0].tint, [0.0, 0.25, 1.0, 1.0]);
        assert_eq!(icon.layers[1].uv.min, [0.25, 0.25]);
        assert_eq!(icon.layers[1].uv.max, [0.5, 0.5]);
        assert_eq!(icon.layers[1].tint, [0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn sanitize_hud_item_icon_discards_invalid_layers() {
        let icon = sanitize_hud_item_icon(HudItemIcon {
            layers: vec![
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, f32::NAN],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                ),
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, f32::INFINITY, 1.0, 1.0],
                ),
                HudIconLayer::new(
                    HudUvRect {
                        min: [0.25, 0.25],
                        max: [0.75, 0.75],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                ),
            ],
        })
        .expect("one valid layer should remain");

        assert_eq!(icon.layers.len(), 1);
        assert_eq!(icon.layers[0].uv.min, [0.25, 0.25]);
        assert_eq!(icon.layers[0].uv.max, [0.75, 0.75]);

        assert_eq!(
            sanitize_hud_item_icon(HudItemIcon {
                layers: vec![HudIconLayer::new(
                    HudUvRect {
                        min: [f32::NAN, 0.0],
                        max: [1.0, 1.0],
                    },
                    [1.0, 1.0, 1.0, 1.0],
                )],
            }),
            None
        );
    }
}
