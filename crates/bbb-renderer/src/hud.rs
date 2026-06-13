use anyhow::Result;
use winit::dpi::PhysicalSize;

use crate::Renderer;

mod gpu;
mod layout;

use self::gpu::create_hud_sprite_gpu;
pub(super) use self::gpu::{create_hud_bind_group_layout, create_hud_pipeline, HudSpriteGpu};
use self::layout::{
    centered_hud_rect, experience_bar_hud_rect, food_hud_rect, heart_hud_rect, hotbar_hud_rect,
    hotbar_selection_hud_rect, hud_experience_progress_width, hud_food_fill, hud_heart_fill,
    hud_quad_vertices, HudIconFill, HudRect, HUD_FOOD_ICONS_PER_ROW, HUD_HEARTS_PER_ROW,
};

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
        self.hud_selected_slot = slot.min(8);
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
                        progress_width as f32 / progress_sprite.width.max(1) as f32,
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
    push_hud_draw_with_uv(vertices, commands, sprite, surface_size, rect, 1.0);
}

fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv_max_x: f32,
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        rect,
        uv_max_x.clamp(0.0, 1.0),
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand { sprite, start, end });
}
