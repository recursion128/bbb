use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use bbb_render_types::{
    DynamicPlayerSkinImage, EntityDefaultPlayerSkin, EntityDynamicPlayerSkinStatus,
    EntityPlayerSkin,
};
use winit::dpi::PhysicalSize;

use crate::entity_models::EntityModelTextureImage;
use crate::Renderer;

use super::{
    absolute_hud_rect, hotbar_hud_rect, hotbar_item_hud_rect, hotbar_selection_hud_rect,
    hud_font_runs_width, push_hud_draw, push_hud_draw_with_uv_and_tint, push_hud_item_icon,
    push_hud_screen_text_draw, HudDrawCommand, HudScreenTextDraw, HudSpriteGpu, HudStyledTextRun,
    HudUvRect, HudVertex, HUD_HOTBAR_SLOTS,
};

const SPECTATOR_ICON_SIZE: u32 = 16;
const SPECTATOR_FACE_SIZE: u32 = 8;
const SPECTATOR_FACE_DRAW_SIZE: u32 = 12;
const SPECTATOR_HOTBAR_WIDTH: u32 = 182;
const SPECTATOR_HOTBAR_HEIGHT: u32 = 22;
const SPECTATOR_SELECTION_WIDTH: u32 = 24;
const SPECTATOR_SELECTION_HEIGHT: u32 = 23;
const PLAYER_SKIN_LOGICAL_SIZE: u32 = 64;

pub(crate) struct HudPlayerFaceGpu {
    without_hat: HudSpriteGpu,
    with_hat: HudSpriteGpu,
}

impl HudPlayerFaceGpu {
    fn sprite(&self, show_hat: bool) -> &HudSpriteGpu {
        if show_hat {
            &self.with_hat
        } else {
            &self.without_hat
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum HudHotbarMode {
    #[default]
    Ordinary,
    Spectator(Option<HudSpectatorGui>),
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSpectatorGui {
    pub alpha: f32,
    pub selected_slot: Option<u8>,
    pub prompt_runs: Vec<HudStyledTextRun>,
    pub slots: [Option<HudSpectatorGuiSlot>; HUD_HOTBAR_SLOTS],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSpectatorGuiSlot {
    pub icon: HudSpectatorIcon,
    pub enabled: bool,
    pub name_runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSpectatorIcon {
    Close,
    PreviousPage,
    NextPage,
    TeleportToPlayer,
    TeleportToTeam,
    Player {
        skin: EntityPlayerSkin,
    },
    Team {
        skin: EntityPlayerSkin,
        color_rgb: Option<u32>,
    },
}

pub(super) fn sanitize_hud_hotbar_mode(mode: HudHotbarMode) -> HudHotbarMode {
    let HudHotbarMode::Spectator(Some(mut menu)) = mode else {
        return mode;
    };
    if !menu.alpha.is_finite() {
        return HudHotbarMode::Spectator(None);
    }
    menu.alpha = menu.alpha.clamp(0.0, 1.0);
    if menu.alpha <= 0.0 {
        return HudHotbarMode::Spectator(None);
    }
    menu.selected_slot = menu
        .selected_slot
        .filter(|slot| usize::from(*slot) < HUD_HOTBAR_SLOTS);
    menu.prompt_runs = sanitize_spectator_runs(menu.prompt_runs);
    for slot in menu.slots.iter_mut().flatten() {
        slot.name_runs = sanitize_spectator_runs(std::mem::take(&mut slot.name_runs));
        if let HudSpectatorIcon::Team { color_rgb, .. } = &mut slot.icon {
            *color_rgb = color_rgb.map(|color| color & 0xFF_FF_FF);
        }
    }
    HudHotbarMode::Spectator(Some(menu))
}

fn sanitize_spectator_runs(runs: Vec<HudStyledTextRun>) -> Vec<HudStyledTextRun> {
    let fallback = runs
        .iter()
        .flat_map(|run| run.text.chars())
        .filter(|ch| !ch.is_control())
        .take(256)
        .collect::<String>();
    super::sanitize_hud_styled_runs(runs, &fallback)
}

impl Renderer {
    pub(super) fn push_hud_hotbar_draws<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
    ) {
        match &self.hud_hotbar_mode {
            HudHotbarMode::Hidden | HudHotbarMode::Spectator(None) => {}
            HudHotbarMode::Ordinary => self.push_hud_ordinary_hotbar_draws(
                vertices,
                commands,
                post_gui_item_commands,
                surface_size,
            ),
            HudHotbarMode::Spectator(Some(menu)) => {
                self.push_hud_spectator_menu_draws(vertices, commands, surface_size, menu);
            }
        }
    }

    fn push_hud_ordinary_hotbar_draws<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        post_gui_item_commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
    ) {
        if let Some(hotbar) = &self.hud_hotbar {
            push_hud_draw(
                vertices,
                commands,
                hotbar,
                surface_size,
                hotbar_hud_rect(surface_size, hotbar.width, hotbar.height),
            );
        }
        if let Some(selection) = &self.hud_hotbar_selection {
            push_hud_draw(
                vertices,
                commands,
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
                let Some(icon) = icon else {
                    continue;
                };
                let item_rect = hotbar_item_hud_rect(surface_size, slot);
                let renders_as_3d_block = self
                    .hud_hotbar_block_item_models
                    .get(slot)
                    .is_some_and(Option::is_some);
                push_hud_item_icon(
                    vertices,
                    commands,
                    atlas,
                    &self.hud_white_pixel,
                    self.hud_digit_atlas.as_ref(),
                    &self.hud_digit_glyphs,
                    surface_size,
                    item_rect,
                    icon,
                    !renders_as_3d_block,
                    !renders_as_3d_block,
                );
                if renders_as_3d_block {
                    push_hud_item_icon(
                        vertices,
                        post_gui_item_commands,
                        atlas,
                        &self.hud_white_pixel,
                        self.hud_digit_atlas.as_ref(),
                        &self.hud_digit_glyphs,
                        surface_size,
                        item_rect,
                        icon,
                        false,
                        true,
                    );
                }
            }
        }
    }

    fn push_hud_spectator_menu_draws<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
        menu: &HudSpectatorGui,
    ) {
        let (Some(hotbar), Some(selection)) = (&self.hud_hotbar, &self.hud_hotbar_selection) else {
            return;
        };
        let center = (surface_size.width.max(1) / 2) as f32;
        let y = (surface_size.height.max(1) as f32 - SPECTATOR_HOTBAR_HEIGHT as f32 * menu.alpha)
            .floor();
        let alpha_tint = [1.0, 1.0, 1.0, menu.alpha];
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            hotbar,
            surface_size,
            absolute_hud_rect(
                center - 91.0,
                y,
                SPECTATOR_HOTBAR_WIDTH,
                SPECTATOR_HOTBAR_HEIGHT,
            ),
            full_sprite_uv(),
            alpha_tint,
        );
        if let Some(slot) = menu.selected_slot {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                selection,
                surface_size,
                absolute_hud_rect(
                    center - 92.0 + f32::from(slot) * 20.0,
                    y - 1.0,
                    SPECTATOR_SELECTION_WIDTH,
                    SPECTATOR_SELECTION_HEIGHT,
                ),
                full_sprite_uv(),
                alpha_tint,
            );
        }

        for (slot_index, slot) in menu.slots.iter().enumerate() {
            let Some(slot) = slot else {
                continue;
            };
            let x = center - 88.0 + slot_index as f32 * 20.0;
            let icon_y = y + 3.0;
            let brightness = if slot.enabled { 1.0 } else { 0.25 };
            self.push_hud_spectator_icon(
                vertices,
                commands,
                surface_size,
                slot.icon,
                x,
                icon_y,
                brightness,
                menu.alpha,
            );
            if slot.enabled {
                self.push_hud_spectator_key_label(
                    vertices,
                    commands,
                    surface_size,
                    slot_index,
                    x,
                    icon_y + 9.0,
                    menu.alpha,
                );
            }
        }
    }

    pub(super) fn push_hud_spectator_action_draws<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
    ) {
        let HudHotbarMode::Spectator(Some(menu)) = &self.hud_hotbar_mode else {
            return;
        };
        let action_runs = menu
            .selected_slot
            .and_then(|slot| menu.slots.get(usize::from(slot)))
            .and_then(Option::as_ref)
            .map_or(menu.prompt_runs.as_slice(), |slot| {
                slot.name_runs.as_slice()
            });
        self.push_hud_spectator_action(vertices, commands, surface_size, action_runs, menu.alpha);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_hud_spectator_icon<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
        icon: HudSpectatorIcon,
        x: f32,
        y: f32,
        brightness: f32,
        alpha: f32,
    ) {
        let tint = [brightness, brightness, brightness, alpha];
        match icon {
            HudSpectatorIcon::Close
            | HudSpectatorIcon::PreviousPage
            | HudSpectatorIcon::NextPage
            | HudSpectatorIcon::TeleportToPlayer
            | HudSpectatorIcon::TeleportToTeam => {
                let sprite = match icon {
                    HudSpectatorIcon::Close => self.hud_spectator_close.as_ref(),
                    HudSpectatorIcon::PreviousPage => self.hud_spectator_scroll_left.as_ref(),
                    HudSpectatorIcon::NextPage => self.hud_spectator_scroll_right.as_ref(),
                    HudSpectatorIcon::TeleportToPlayer => {
                        self.hud_spectator_teleport_to_player.as_ref()
                    }
                    HudSpectatorIcon::TeleportToTeam => {
                        self.hud_spectator_teleport_to_team.as_ref()
                    }
                    HudSpectatorIcon::Player { .. } | HudSpectatorIcon::Team { .. } => None,
                };
                if let Some(sprite) = sprite {
                    push_hud_draw_with_uv_and_tint(
                        vertices,
                        commands,
                        sprite,
                        surface_size,
                        absolute_hud_rect(x, y, SPECTATOR_ICON_SIZE, SPECTATOR_ICON_SIZE),
                        full_sprite_uv(),
                        tint,
                    );
                }
            }
            HudSpectatorIcon::Player { skin } => {
                self.push_hud_spectator_face(vertices, commands, surface_size, skin, x, y, tint);
            }
            HudSpectatorIcon::Team { skin, color_rgb } => {
                if let Some(color) = color_rgb {
                    push_hud_draw_with_uv_and_tint(
                        vertices,
                        commands,
                        &self.hud_white_pixel,
                        surface_size,
                        absolute_hud_rect(x + 1.0, y + 1.0, 14, 14),
                        full_sprite_uv(),
                        [
                            ((color >> 16) & 0xFF) as f32 / 255.0 * brightness,
                            ((color >> 8) & 0xFF) as f32 / 255.0 * brightness,
                            (color & 0xFF) as f32 / 255.0 * brightness,
                            alpha,
                        ],
                    );
                }
                self.push_hud_spectator_face(vertices, commands, surface_size, skin, x, y, tint);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_hud_spectator_face<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
        skin: EntityPlayerSkin,
        x: f32,
        y: f32,
        tint: [f32; 4],
    ) {
        let Some(sprite) = self.hud_player_face(skin) else {
            return;
        };
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            sprite,
            surface_size,
            absolute_hud_rect(
                x + 2.0,
                y + 2.0,
                SPECTATOR_FACE_DRAW_SIZE,
                SPECTATOR_FACE_DRAW_SIZE,
            ),
            full_sprite_uv(),
            tint,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_hud_spectator_key_label<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
        slot: usize,
        x: f32,
        y: f32,
        alpha: f32,
    ) {
        let Some(font_atlas) = self.hud_font_atlas.as_ref() else {
            return;
        };
        let runs = [HudStyledTextRun::plain((slot + 1).to_string())];
        let width = hud_font_runs_width(&runs, &self.hud_font_glyphs).unwrap_or_default();
        let draw = HudScreenTextDraw {
            runs: &runs,
            origin: (x + 17.0 - width as f32, y),
            scale: 1.0,
            tint: [1.0, 1.0, 1.0, alpha],
            backdrop_opacity: 0.0,
        };
        push_hud_screen_text_draw(
            vertices,
            commands,
            &self.hud_white_pixel,
            font_atlas,
            &self.hud_font_glyphs,
            &self.hud_obfuscated_glyph_pool,
            self.counters.frame_index,
            surface_size,
            &draw,
        );
    }

    fn push_hud_spectator_action<'a>(
        &'a self,
        vertices: &mut Vec<HudVertex>,
        commands: &mut Vec<HudDrawCommand<'a>>,
        surface_size: PhysicalSize<u32>,
        runs: &[HudStyledTextRun],
        alpha: f32,
    ) {
        let Some(font_atlas) = self.hud_font_atlas.as_ref() else {
            return;
        };
        let width = hud_font_runs_width(runs, &self.hud_font_glyphs).unwrap_or_default();
        let action_x = (i64::from(surface_size.width) - i64::from(width)) / 2;
        let draw = HudScreenTextDraw {
            runs,
            origin: (
                action_x as f32,
                (i64::from(surface_size.height) - 35) as f32,
            ),
            scale: 1.0,
            tint: [1.0, 1.0, 1.0, alpha],
            backdrop_opacity: if self.hud_background_for_chat_only {
                0.0
            } else {
                self.hud_text_background_opacity
            },
        };
        push_hud_screen_text_draw(
            vertices,
            commands,
            &self.hud_white_pixel,
            font_atlas,
            &self.hud_font_glyphs,
            &self.hud_obfuscated_glyph_pool,
            self.counters.frame_index,
            surface_size,
            &draw,
        );
    }

    pub(super) fn hud_player_face(&self, skin: EntityPlayerSkin) -> Option<&HudSpriteGpu> {
        self.hud_player_face_with_hat(skin, true)
    }

    pub(super) fn hud_player_face_with_hat(
        &self,
        skin: EntityPlayerSkin,
        show_hat: bool,
    ) -> Option<&HudSpriteGpu> {
        let default = skin.fallback();
        let face = match skin {
            EntityPlayerSkin::Dynamic(dynamic)
                if dynamic.status == EntityDynamicPlayerSkinStatus::Ready =>
            {
                self.hud_dynamic_player_faces
                    .get(&dynamic.handle)
                    .or_else(|| self.hud_default_player_faces.get(&default))
            }
            EntityPlayerSkin::Default(_)
            | EntityPlayerSkin::ProfiledDefault(_)
            | EntityPlayerSkin::Dynamic(_) => self.hud_default_player_faces.get(&default),
        }?;
        Some(face.sprite(show_hat))
    }

    pub(crate) fn build_hud_player_face_owners(
        &self,
        static_images: &[EntityModelTextureImage],
        dynamic_images: &[DynamicPlayerSkinImage],
    ) -> Result<(
        HashMap<EntityDefaultPlayerSkin, HudPlayerFaceGpu>,
        HashMap<u64, HudPlayerFaceGpu>,
    )> {
        let mut defaults = HashMap::new();
        for skin in EntityDefaultPlayerSkin::ALL {
            let Some(image) = static_images
                .iter()
                .find(|image| image.texture.path == skin.texture_path())
            else {
                continue;
            };
            let face = self.build_hud_player_face(image.size, &image.rgba)?;
            defaults.insert(skin, face);
        }
        let mut dynamics = HashMap::new();
        for image in dynamic_images {
            let face = self.build_hud_player_face(DynamicPlayerSkinImage::SIZE, &image.rgba)?;
            dynamics.insert(image.handle, face);
        }
        Ok((defaults, dynamics))
    }

    pub(crate) fn build_hud_dynamic_player_face(
        &self,
        image: &DynamicPlayerSkinImage,
    ) -> Result<HudPlayerFaceGpu> {
        self.build_hud_player_face(DynamicPlayerSkinImage::SIZE, &image.rgba)
    }

    fn build_hud_player_face(&self, size: [u32; 2], rgba: &[u8]) -> Result<HudPlayerFaceGpu> {
        let without_hat = player_face_rgba(size, rgba, false)?;
        let with_hat = player_face_rgba(size, rgba, true)?;
        Ok(HudPlayerFaceGpu {
            without_hat: self.upload_hud_sprite(
                SPECTATOR_FACE_SIZE,
                SPECTATOR_FACE_SIZE,
                &without_hat,
            )?,
            with_hat: self.upload_hud_sprite(
                SPECTATOR_FACE_SIZE,
                SPECTATOR_FACE_SIZE,
                &with_hat,
            )?,
        })
    }
}

fn full_sprite_uv() -> HudUvRect {
    HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    }
}

fn player_face_rgba(size: [u32; 2], rgba: &[u8], show_hat: bool) -> Result<Vec<u8>> {
    let [width, height] = size;
    if width == 0 || height == 0 {
        bail!("player skin face source dimensions must be non-zero");
    }
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .context("player skin face source byte length overflow")?;
    if rgba.len() != expected {
        bail!(
            "player skin face source byte length mismatch: expected {expected}, got {}",
            rgba.len()
        );
    }
    let mut face = vec![0; (SPECTATOR_FACE_SIZE * SPECTATOR_FACE_SIZE * 4) as usize];
    for y in 0..SPECTATOR_FACE_SIZE {
        for x in 0..SPECTATOR_FACE_SIZE {
            let base = sample_logical_skin_pixel(size, rgba, x + 8, y + 8);
            let pixel = if show_hat {
                let hat = sample_logical_skin_pixel(size, rgba, x + 40, y + 8);
                alpha_over(base, hat)
            } else {
                base
            };
            let offset = ((y * SPECTATOR_FACE_SIZE + x) * 4) as usize;
            face[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
    Ok(face)
}

fn sample_logical_skin_pixel(
    [width, height]: [u32; 2],
    rgba: &[u8],
    logical_x: u32,
    logical_y: u32,
) -> [u8; 4] {
    let x = ((u64::from(logical_x) * u64::from(width)) / u64::from(PLAYER_SKIN_LOGICAL_SIZE))
        .min(u64::from(width - 1)) as u32;
    let y = ((u64::from(logical_y) * u64::from(height)) / u64::from(PLAYER_SKIN_LOGICAL_SIZE))
        .min(u64::from(height - 1)) as u32;
    let offset = ((u64::from(y) * u64::from(width) + u64::from(x)) * 4) as usize;
    rgba[offset..offset + 4]
        .try_into()
        .expect("validated RGBA offset")
}

fn alpha_over(base: [u8; 4], overlay: [u8; 4]) -> [u8; 4] {
    let source_alpha = u32::from(overlay[3]);
    let inverse = 255 - source_alpha;
    let output_alpha = source_alpha + (u32::from(base[3]) * inverse + 127) / 255;
    if output_alpha == 0 {
        return [0; 4];
    }
    let mut output = [0; 4];
    for channel in 0..3 {
        let premultiplied = u32::from(overlay[channel]) * source_alpha
            + (u32::from(base[channel]) * u32::from(base[3]) * inverse + 127) / 255;
        output[channel] = ((premultiplied + output_alpha / 2) / output_alpha).min(255) as u8;
    }
    output[3] = output_alpha.min(255) as u8;
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_player_face_hat_variants_use_logical_skin_coordinates() {
        let mut rgba = vec![0; 64 * 64 * 4];
        set_pixel(&mut rgba, 64, 8, 8, [20, 40, 60, 255]);
        set_pixel(&mut rgba, 64, 40, 8, [220, 20, 10, 128]);

        let face = player_face_rgba([64, 64], &rgba, true).unwrap();
        let face_without_hat = player_face_rgba([64, 64], &rgba, false).unwrap();

        assert_eq!(&face[..4], &[120, 30, 35, 255]);
        assert_eq!(&face_without_hat[..4], &[20, 40, 60, 255]);
        assert!(face[4..].iter().all(|channel| *channel == 0));
    }

    #[test]
    fn hud_spectator_default_face_sources_cover_the_generated_player_skin_catalog() {
        let textures = crate::entity_models::entity_model_texture_refs();
        assert_eq!(EntityDefaultPlayerSkin::ALL.len(), 18);
        for skin in EntityDefaultPlayerSkin::ALL {
            assert!(
                textures
                    .iter()
                    .any(|texture| texture.path == skin.texture_path()),
                "missing default HUD face source {}",
                skin.texture_path()
            );
        }
    }

    #[test]
    fn hud_spectator_geometry_matches_vanilla_hotbar_and_slot_origins() {
        let size = PhysicalSize::new(320, 180);
        let alpha = 0.5;
        let center = (size.width / 2) as f32;
        let y = (size.height as f32 - 22.0 * alpha).floor();

        assert_eq!((center - 91.0, y), (69.0, 169.0));
        assert_eq!((center - 92.0 + 8.0 * 20.0, y - 1.0), (228.0, 168.0));
        assert_eq!((center - 88.0, y + 3.0), (72.0, 172.0));
        assert_eq!((center - 88.0 + 8.0 * 20.0, y + 3.0), (232.0, 172.0));
        assert_eq!((321u32 / 2) as f32 - 91.0, 69.0);
    }

    #[test]
    fn hud_spectator_draws_faded_hotbar_selection_and_icon_in_vanilla_order() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 180) else {
            eprintln!("skipping offscreen renderer test without a compatible adapter");
            return;
        };
        renderer
            .upload_hud_hotbar(182, 22, &[255, 0, 0, 255].repeat(182 * 22))
            .unwrap();
        renderer
            .upload_hud_hotbar_selection(24, 23, &[0, 255, 0, 255].repeat(24 * 23))
            .unwrap();
        renderer
            .upload_hud_spectator_close(16, 16, &[0, 0, 255, 255].repeat(16 * 16))
            .unwrap();
        let mut slots = std::array::from_fn(|_| None);
        slots[8] = Some(HudSpectatorGuiSlot {
            icon: HudSpectatorIcon::Close,
            enabled: true,
            name_runs: vec![HudStyledTextRun::plain("Close Menu")],
        });
        renderer.set_hud_hotbar_mode(HudHotbarMode::Spectator(Some(HudSpectatorGui {
            alpha: 0.5,
            selected_slot: Some(8),
            prompt_runs: vec![HudStyledTextRun::plain("Prompt")],
            slots,
        })));

        let pixels = renderer.render_offscreen_frame().unwrap();
        let hotbar = pixels.pixel(70, 170);
        assert!(
            hotbar[0] > hotbar[1] && hotbar[0] > hotbar[2],
            "hotbar should remain the red bottom layer, got {hotbar:?}"
        );
        let selection = pixels.pixel(229, 169);
        assert!(
            selection[1] > selection[0] && selection[1] > selection[2],
            "selection should overlay the hotbar in green, got {selection:?}"
        );
        let icon = pixels.pixel(233, 173);
        assert!(
            icon[2] > icon[0] && icon[2] > icon[1],
            "close icon should overlay the selection in blue, got {icon:?}"
        );
    }

    #[test]
    fn hud_spectator_action_uses_accessibility_text_backdrop() {
        let Some(mut renderer) = Renderer::new_offscreen(320, 180) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        let mut glyphs = bbb_render_types::HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'a',
            bbb_render_types::HudAsciiGlyph {
                advance: 6,
                width: 5,
                height: 8,
                ..bbb_render_types::HudAsciiGlyph::default()
            },
        );
        renderer
            .upload_hud_font_atlas(
                1,
                1,
                &[255, 255, 255, 255],
                bbb_render_types::HudFontGlyphBook::from(glyphs),
            )
            .expect("font atlas");
        renderer.set_hud_hotbar_mode(HudHotbarMode::Spectator(Some(HudSpectatorGui {
            alpha: 1.0,
            selected_slot: None,
            prompt_runs: vec![HudStyledTextRun::plain("a")],
            slots: std::array::from_fn(|_| None),
        })));
        renderer.set_hud_text_background_options(0.5, false);

        let pixels = renderer.render_offscreen_frame().expect("spectator frame");
        assert!(
            pixels.pixel(155, 143)[2] < 230,
            "spectator action padding must receive the global text backdrop"
        );
        assert!(
            pixels.pixel(157, 145)[0] > 230,
            "spectator action glyph must draw after its backdrop"
        );
    }

    fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, pixel: [u8; 4]) {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].copy_from_slice(&pixel);
    }
}
