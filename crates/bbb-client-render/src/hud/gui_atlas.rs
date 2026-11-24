//! The `gui` sprite atlas (Vanilla `GuiSpriteManager`) and the keyed cache of
//! standalone GUI textures (`TextureManager` entries such as container
//! backgrounds).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Result};
use bbb_client_assets::{AtlasPacker, AtlasRect, SpriteGuiScaling, SpriteImage, SpriteSource};
use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_TYPES;
use bbb_render_types::{SpriteAnimation, StandaloneTextureSampling};

use super::{
    create_hud_sprite_gpu, create_standalone_hud_sprite_gpu, HudGuiSpriteScaling,
    HudNineSliceScaling, HudSpriteGpu, HudUvRect,
};
use crate::Renderer;

const GUI_ATLAS_MIN_WIDTH: u32 = 1024;

/// Standalone textures the HUD draws by path; resource load uploads each one
/// with [`Renderer::upload_gui_texture`].
pub const GUI_TEXTURES: &[&str] = &[
    "textures/gui/advancements/backgrounds/adventure.png",
    "textures/gui/advancements/backgrounds/end.png",
    "textures/gui/advancements/backgrounds/husbandry.png",
    "textures/gui/advancements/backgrounds/nether.png",
    "textures/gui/advancements/backgrounds/stone.png",
    "textures/gui/advancements/window.png",
    "textures/gui/book.png",
    "textures/gui/container/anvil.png",
    "textures/gui/container/beacon.png",
    "textures/gui/container/blast_furnace.png",
    "textures/gui/container/brewing_stand.png",
    "textures/gui/container/cartography_table.png",
    "textures/gui/container/crafter.png",
    "textures/gui/container/crafting_table.png",
    "textures/gui/container/creative_inventory/tab_inventory.png",
    "textures/gui/container/creative_inventory/tab_item_search.png",
    "textures/gui/container/creative_inventory/tab_items.png",
    "textures/gui/container/dispenser.png",
    "textures/gui/container/enchanting_table.png",
    "textures/gui/container/furnace.png",
    "textures/gui/container/gamemode_switcher.png",
    "textures/gui/container/generic_54.png",
    "textures/gui/container/grindstone.png",
    "textures/gui/container/hopper.png",
    "textures/gui/container/horse.png",
    "textures/gui/container/inventory.png",
    "textures/gui/container/loom.png",
    "textures/gui/container/nautilus.png",
    "textures/gui/container/shulker_box.png",
    "textures/gui/container/smithing.png",
    "textures/gui/container/smoker.png",
    "textures/gui/container/stonecutter.png",
    "textures/gui/container/villager.png",
    "textures/gui/hanging_signs/acacia.png",
    "textures/gui/hanging_signs/bamboo.png",
    "textures/gui/hanging_signs/birch.png",
    "textures/gui/hanging_signs/cherry.png",
    "textures/gui/hanging_signs/crimson.png",
    "textures/gui/hanging_signs/dark_oak.png",
    "textures/gui/hanging_signs/jungle.png",
    "textures/gui/hanging_signs/mangrove.png",
    "textures/gui/hanging_signs/oak.png",
    "textures/gui/hanging_signs/pale_oak.png",
    "textures/gui/hanging_signs/spruce.png",
    "textures/gui/hanging_signs/warped.png",
    "textures/gui/inworld_menu_background.png",
    "textures/gui/header_separator.png",
    "textures/gui/footer_separator.png",
    "textures/gui/menu_background.png",
    "textures/gui/recipe_book.png",
    "textures/gui/title/edition.png",
    "textures/gui/title/minceraft.png",
    "textures/gui/title/minecraft.png",
    "textures/misc/nausea.png",
    "textures/misc/powder_snow_outline.png",
    "textures/misc/pumpkinblur.png",
    "textures/misc/spyglass_scope.png",
];
const MISSING_SPRITE: &str = "missingno";

/// Sprites stitched into one texture and addressed by resource location; the
/// `minecraft:` namespace may be omitted.
#[derive(Default)]
pub(crate) struct GuiSprites {
    ids: HashMap<Box<str>, usize>,
    sprites: Vec<GuiSprite>,
}

struct GuiSprite {
    frames: Box<[HudSpriteGpu]>,
    animation: Option<SpriteAnimation>,
}

impl GuiSprites {
    pub(crate) fn collect_retire(self, textures: &mut Vec<wgpu::Texture>) {
        // Every frame shares an `Arc<HudSpriteGpuInner>` to the packed atlas texture(s);
        // `try_unwrap` succeeds exactly once when the final reference drops.
        let arcs: Vec<_> = self
            .sprites
            .into_iter()
            .flat_map(|sprite| sprite.frames.into_vec())
            .map(|frame| frame.gpu)
            .collect();
        for gpu in arcs {
            if let Ok(inner) = std::sync::Arc::try_unwrap(gpu) {
                textures.push(inner.texture);
            }
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&HudSpriteGpu> {
        self.sprites[*self.ids.get(sprite_key(id))?].frames.first()
    }

    /// The frame an animated sprite shows at `tick` (Vanilla ticks the GUI
    /// atlas with the client tick); a static sprite ignores it.
    pub(crate) fn get_at_tick(&self, id: &str, tick: u64) -> Option<&HudSpriteGpu> {
        let sprite = &self.sprites[*self.ids.get(sprite_key(id))?];
        let index = sprite
            .animation
            .as_ref()
            .and_then(|animation| animation.frame_index_at_tick(tick))
            .unwrap_or(0);
        sprite.frames.get(usize::try_from(index).ok()?)
    }

    /// Vanilla `TooltipRenderUtil` style sprites: `<style ns>:tooltip/<style path>_background`
    /// and `_frame`.
    pub(crate) fn tooltip_style(&self, style: &str) -> Option<(&HudSpriteGpu, &HudSpriteGpu)> {
        let (namespace, path) = sprite_key(style)
            .split_once(':')
            .map_or(("", sprite_key(style)), |(namespace, path)| {
                (namespace, path)
            });
        let separator = if namespace.is_empty() { "" } else { ":" };
        Some((
            self.get(&format!("{namespace}{separator}tooltip/{path}_background"))?,
            self.get(&format!("{namespace}{separator}tooltip/{path}_frame"))?,
        ))
    }

    /// Vanilla `Gui.getMobEffectSprite`: `mob_effect/<effect path>`.
    pub(crate) fn mob_effect(&self, effect_id: i32) -> Option<&HudSpriteGpu> {
        let effect = VANILLA_MOB_EFFECT_TYPES.get(usize::try_from(effect_id).ok()?)?;
        self.get(&format!("mob_effect/{}", sprite_key(effect.resource_id)))
    }

    pub(crate) fn identity_ptr(&self) -> usize {
        self.sprites.as_ptr() as usize
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn atlas_texture(&self) -> Option<&super::gpu::HudTexture> {
        self.sprites
            .first()
            .and_then(|sprite| sprite.frames.first())
            .map(|frame| frame.gpu.as_ref())
    }
}

fn sprite_key(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

pub(crate) fn hud_gui_sprite_scaling(scaling: SpriteGuiScaling) -> HudGuiSpriteScaling {
    match scaling {
        SpriteGuiScaling::Stretch => HudGuiSpriteScaling::Stretch,
        SpriteGuiScaling::Tile { width, height } => HudGuiSpriteScaling::Tile { width, height },
        SpriteGuiScaling::NineSlice {
            width,
            height,
            border,
            stretch_inner,
        } => HudGuiSpriteScaling::NineSlice(HudNineSliceScaling {
            sprite_width: width,
            sprite_height: height,
            border_left: border.left,
            border_top: border.top,
            border_right: border.right,
            border_bottom: border.bottom,
            stretch_inner,
        }),
    }
}

pub(crate) fn missing_texture_rgba(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for y in 0..height {
        for x in 0..width {
            if (y < height / 2) ^ (x < width / 2) {
                rgba.extend_from_slice(&[0xf8, 0x00, 0xf8, 0xff]);
            } else {
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0xff]);
            }
        }
    }
    rgba
}

/// One stitched rectangle: an unanimated sprite or one frame of an animated one.
#[cfg(test)]
fn frame_image(id: &str, width: u32, height: u32, rgba: Vec<u8>) -> SpriteImage {
    SpriteImage {
        id: id.to_string(),
        width,
        height,
        transparency: Default::default(),
        animation: None,
        texture_metadata: Default::default(),
        gui_metadata: Default::default(),
        animation_frames_rgba: Vec::new(),
        rgba,
    }
}

struct BorrowedGuiFrame<'a> {
    id: &'a str,
    width: u32,
    height: u32,
    rgba: &'a [u8],
    owner: usize,
}

fn copy_gui_sprite_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    content: AtlasRect,
    padded: AtlasRect,
    id: &str,
    src_rgba: &[u8],
) -> Result<()> {
    let expected_len = (content.width as usize)
        .checked_mul(content.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("GUI sprite {id} RGBA length overflow"))?;
    if src_rgba.len() != expected_len {
        bail!(
            "GUI sprite {id} RGBA byte length {} does not match {}x{}",
            src_rgba.len(),
            content.width,
            content.height
        );
    }
    let atlas_row_bytes = (atlas_width as usize) * 4;
    let src_row_bytes = (content.width as usize) * 4;
    let pad_left = content.x - padded.x;
    let pad_right = (padded.x + padded.width) - (content.x + content.width);
    let pad_top = content.y - padded.y;
    let pad_bottom = (padded.y + padded.height) - (content.y + content.height);

    for row in 0..content.height as usize {
        let src_start = row * src_row_bytes;
        let src_row = &src_rgba[src_start..src_start + src_row_bytes];
        let dst_y = (content.y as usize) + row;
        let dst_row_start = dst_y * atlas_row_bytes;
        let content_start = dst_row_start + (content.x as usize) * 4;
        atlas[content_start..content_start + src_row_bytes].copy_from_slice(src_row);
        let left_px = &src_row[..4];
        for gx in 0..pad_left as usize {
            let offset = dst_row_start + ((padded.x as usize) + gx) * 4;
            atlas[offset..offset + 4].copy_from_slice(left_px);
        }
        let right_px = &src_row[src_row_bytes - 4..];
        let right_base = content_start + src_row_bytes;
        for gx in 0..pad_right as usize {
            let offset = right_base + gx * 4;
            atlas[offset..offset + 4].copy_from_slice(right_px);
        }
    }

    let padded_row_bytes = (padded.width as usize) * 4;
    let padded_x_bytes = (padded.x as usize) * 4;
    let top_src_offset = (content.y as usize) * atlas_row_bytes + padded_x_bytes;
    for gy in 0..pad_top as usize {
        let dst_offset = ((padded.y as usize) + gy) * atlas_row_bytes + padded_x_bytes;
        atlas.copy_within(
            top_src_offset..top_src_offset + padded_row_bytes,
            dst_offset,
        );
    }
    let bottom_src_offset =
        ((content.y + content.height - 1) as usize) * atlas_row_bytes + padded_x_bytes;
    let bottom_base_y = (content.y + content.height) as usize;
    for gy in 0..pad_bottom as usize {
        let dst_offset = (bottom_base_y + gy) * atlas_row_bytes + padded_x_bytes;
        atlas.copy_within(
            bottom_src_offset..bottom_src_offset + padded_row_bytes,
            dst_offset,
        );
    }
    Ok(())
}

pub(crate) const MAX_GUI_ATLAS_BYTES: usize = 128 * 1024 * 1024;

fn pack_gui_atlas_layout(
    sorted_sources: &[SpriteSource],
    widest_padded: u32,
    limit: u32,
) -> Result<(bbb_client_assets::AtlasLayout, u32, u32)> {
    pack_gui_atlas_layout_with_budget(sorted_sources, widest_padded, limit, MAX_GUI_ATLAS_BYTES)
}

fn pack_gui_atlas_layout_with_budget(
    sorted_sources: &[SpriteSource],
    widest_padded: u32,
    limit: u32,
    max_bytes: usize,
) -> Result<(bbb_client_assets::AtlasLayout, u32, u32)> {
    let mut atlas_width = widest_padded
        .next_power_of_two()
        .clamp(GUI_ATLAS_MIN_WIDTH.min(limit), limit);
    let mut last_oversized: Option<(u32, u32, u64)> = None;
    loop {
        let packed = AtlasPacker::new_bounded(atlas_width, limit, 1)?.pack(sorted_sources);
        if let Ok(layout) = packed {
            let width = layout
                .width
                .max(1)
                .checked_next_power_of_two()
                .ok_or_else(|| anyhow::anyhow!("GUI atlas width overflow"))?;
            let height = layout
                .height
                .max(1)
                .checked_next_power_of_two()
                .ok_or_else(|| anyhow::anyhow!("GUI atlas height overflow"))?;
            if height <= limit {
                if width > limit {
                    bail!("GUI atlas width {width} exceeds device texture limit {limit}");
                }
                let bytes = u64::from(width)
                    .checked_mul(u64::from(height))
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| anyhow::anyhow!("GUI atlas byte length overflow"))?;
                if bytes <= max_bytes as u64 {
                    return Ok((layout, width, height));
                }
                last_oversized = Some((width, height, bytes));
            }
        }
        if atlas_width >= limit {
            if let Some((width, height, bytes)) = last_oversized {
                bail!("GUI atlas {width}x{height} ({bytes} bytes) exceeds {max_bytes} byte budget");
            }
            bail!("GUI atlas height exceeds device texture limit {limit}");
        }
        atlas_width = atlas_width
            .checked_mul(2)
            .map_or(limit, |doubled| doubled.min(limit));
    }
}

impl Renderer {
    /// Stitches the pack's `gui` atlas sources and replaces the GUI sprite set.
    pub fn upload_gui_sprites(&mut self, images: &[SpriteImage]) -> Result<()> {
        let limit = self.device.limits().max_texture_dimension_2d;
        let missing = !images
            .iter()
            .any(|image| sprite_key(&image.id) == MISSING_SPRITE);
        let missing_rgba = missing.then(|| missing_texture_rgba(16, 16));
        let mut frames = Vec::with_capacity(images.len() + usize::from(missing));
        for (index, image) in images.iter().enumerate() {
            if image.width.saturating_add(2) > limit || image.height.saturating_add(2) > limit {
                tracing::warn!(
                    "skipping GUI sprite {} ({}x{}): larger than the {limit} px texture limit",
                    image.id,
                    image.width,
                    image.height
                );
                continue;
            }
            if image.animation.is_some() && !image.animation_frames_rgba.is_empty() {
                for rgba in &image.animation_frames_rgba {
                    frames.push(BorrowedGuiFrame {
                        id: &image.id,
                        width: image.width,
                        height: image.height,
                        rgba: rgba.as_slice(),
                        owner: index,
                    });
                }
            } else {
                frames.push(BorrowedGuiFrame {
                    id: &image.id,
                    width: image.width,
                    height: image.height,
                    rgba: image.rgba.as_slice(),
                    owner: index,
                });
            }
        }
        if let Some(rgba) = missing_rgba.as_deref() {
            frames.push(BorrowedGuiFrame {
                id: MISSING_SPRITE,
                width: 16,
                height: 16,
                rgba,
                owner: images.len(),
            });
        }
        // Shelf packing wastes least when rows hold sprites of similar height.
        let mut order = (0..frames.len()).collect::<Vec<_>>();
        order.sort_by_key(|&index| std::cmp::Reverse((frames[index].height, frames[index].width)));
        let sorted_sources = order
            .iter()
            .map(|&index| {
                let frame = &frames[index];
                SpriteSource {
                    id: frame.id.to_string(),
                    width: frame.width,
                    height: frame.height,
                    animation: None,
                    texture_metadata: Default::default(),
                    gui_metadata: Default::default(),
                }
            })
            .collect::<Vec<_>>();
        // One texel of edge-replicated gutter keeps nearest sampling at a
        // sprite edge identical to a clamped standalone texture.
        let widest = frames
            .iter()
            .map(|frame| frame.width + 2)
            .max()
            .unwrap_or(0);
        let (layout, width, height) = pack_gui_atlas_layout(&sorted_sources, widest, limit)?;
        let atlas_byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("GUI atlas byte length overflow"))?;
        if atlas_byte_len > MAX_GUI_ATLAS_BYTES {
            bail!(
                "GUI atlas {width}x{height} ({atlas_byte_len} bytes) exceeds {MAX_GUI_ATLAS_BYTES} byte budget"
            );
        }
        let mut atlas_rgba = vec![0u8; atlas_byte_len];
        let mut rects = vec![None; frames.len()];
        for (slot, (&index, placed)) in order.iter().zip(&layout.sprites).enumerate() {
            let _ = slot;
            let frame = &frames[index];
            copy_gui_sprite_with_gutter(
                &mut atlas_rgba,
                width,
                placed.content,
                placed.padded,
                frame.id,
                frame.rgba,
            )?;
            rects[index] = Some(placed.content);
        }
        let texture = create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            &atlas_rgba,
        )?
        .gpu;

        let mut sprites = GuiSprites::default();
        let mut start = 0;
        while start < frames.len() {
            let owner = frames[start].owner;
            let mut end = start;
            while end < frames.len() && frames[end].owner == owner {
                end += 1;
            }
            let (id, scaling, animation) = match images.get(owner) {
                Some(image) => (
                    image.id.as_str(),
                    hud_gui_sprite_scaling(image.gui_metadata.scaling),
                    image.animation.clone(),
                ),
                None => (MISSING_SPRITE, HudGuiSpriteScaling::Stretch, None),
            };
            if let HudGuiSpriteScaling::Tile { width, height } = scaling {
                if width == 0 || height == 0 {
                    bail!("GUI sprite {id} tile size must be positive, got {width}x{height}");
                }
            }
            let frames = rects[start..end]
                .iter()
                .map(|rect| {
                    let rect = rect.expect("every stitched frame has a rect");
                    HudSpriteGpu {
                        gpu: Arc::clone(&texture),
                        uv: HudUvRect {
                            min: [rect.x as f32 / width as f32, rect.y as f32 / height as f32],
                            max: [
                                (rect.x + rect.width) as f32 / width as f32,
                                (rect.y + rect.height) as f32 / height as f32,
                            ],
                        },
                        width: rect.width,
                        height: rect.height,
                        scaling,
                    }
                })
                .collect::<Box<[_]>>();
            sprites
                .ids
                .insert(sprite_key(id).into(), sprites.sprites.len());
            sprites.sprites.push(GuiSprite { frames, animation });
            start = end;
        }
        let crosshair = sprites
            .get("hud/crosshair")
            .map_or((0, 0), |sprite| (sprite.width, sprite.height));
        self.counters.hud_crosshair_width = crosshair.0;
        self.counters.hud_crosshair_height = crosshair.1;
        self.pack.gui_sprites = sprites;
        self.mark_hud_dirty();
        Ok(())
    }

    pub(crate) fn gui_sprite(&self, id: &str) -> Option<&HudSpriteGpu> {
        self.pack.gui_sprites.get(id)
    }

    pub(crate) fn gui_sprite_at_tick(&self, id: &str, tick: u64) -> Option<&HudSpriteGpu> {
        self.pack.gui_sprites.get_at_tick(id, tick)
    }

    /// Adds or replaces one standalone GUI texture keyed by its resource path
    /// (`textures/gui/container/loom.png`).
    pub fn upload_gui_texture(
        &mut self,
        path: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        let texture = create_standalone_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
            sampling,
        )?;
        self.pack
            .gui_textures
            .insert(sprite_key(path).into(), texture);
        self.mark_hud_dirty();
        Ok(())
    }

    pub(crate) fn gui_texture(&self, path: &str) -> Option<&HudSpriteGpu> {
        self.pack.gui_textures.get(sprite_key(path))
    }
}

#[cfg(test)]
impl Renderer {
    /// Adds one stretch-scaled test sprite and restitches the GUI atlas.
    pub(crate) fn add_test_gui_sprite(
        &mut self,
        id: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.add_test_gui_sprite_scaled(id, width, height, rgba, HudGuiSpriteScaling::Stretch)
    }

    pub(crate) fn add_test_gui_sprite_scaled(
        &mut self,
        id: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
        scaling: HudGuiSpriteScaling,
    ) -> Result<()> {
        use bbb_client_assets::SpriteNineSliceBorder;
        let mut image = frame_image(id, width, height, rgba.to_vec());
        image.gui_metadata.scaling = match scaling {
            HudGuiSpriteScaling::Stretch => SpriteGuiScaling::Stretch,
            HudGuiSpriteScaling::Tile { width, height } => SpriteGuiScaling::Tile { width, height },
            HudGuiSpriteScaling::NineSlice(nine) => SpriteGuiScaling::NineSlice {
                width: nine.sprite_width,
                height: nine.sprite_height,
                border: SpriteNineSliceBorder {
                    left: nine.border_left,
                    top: nine.border_top,
                    right: nine.border_right,
                    bottom: nine.border_bottom,
                },
                stretch_inner: nine.stretch_inner,
            },
        };
        let key = sprite_key(id);
        self.test_gui_sprite_images
            .retain(|image| sprite_key(&image.id) != key);
        self.test_gui_sprite_images.push(image);
        let images = std::mem::take(&mut self.test_gui_sprite_images);
        let result = self.upload_gui_sprites(&images);
        self.test_gui_sprite_images = images;
        result
    }

    pub(crate) fn add_test_animated_gui_sprite(
        &mut self,
        id: &str,
        width: u32,
        height: u32,
        frames_rgba: Vec<Vec<u8>>,
        frame_time: u32,
    ) -> Result<()> {
        use bbb_render_types::SpriteAnimationFrame;
        let frame_count = u32::try_from(frames_rgba.len()).expect("frame count");
        let mut image = frame_image(
            id,
            width,
            height,
            frames_rgba.first().cloned().unwrap_or_default(),
        );
        image.animation = Some(SpriteAnimation {
            frame_count,
            default_frame_time: frame_time,
            interpolate: false,
            frames: (0..frame_count)
                .map(|index| SpriteAnimationFrame {
                    index,
                    time: frame_time,
                })
                .collect(),
        });
        image.animation_frames_rgba = frames_rgba;
        let key = sprite_key(id);
        self.test_gui_sprite_images
            .retain(|existing| sprite_key(&existing.id) != key);
        self.test_gui_sprite_images.push(image);
        let images = std::mem::take(&mut self.test_gui_sprite_images);
        let result = self.upload_gui_sprites(&images);
        self.test_gui_sprite_images = images;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_gui_atlas_layout_widens_when_initial_width_exceeds_height_limit() {
        let sources = (0..16)
            .map(|i| SpriteSource {
                id: format!("minecraft:gui/tile_{i}"),
                width: 254,
                height: 254,
                animation: None,
                texture_metadata: Default::default(),
                gui_metadata: Default::default(),
            })
            .collect::<Vec<_>>();
        // With widest_padded = 256, initial width is 1024 -> 4x4 grid = 1024x1024.
        // Restricting limit to 2048 and giving 40 tiles of 254x254 overflows height at 1024
        // (10 rows = 2560 > 2048) and must widen to 2048 (5 rows = 1280 <= 2048).
        let many = (0..40)
            .map(|i| SpriteSource {
                id: format!("minecraft:gui/tile_{i}"),
                width: 254,
                height: 254,
                animation: None,
                texture_metadata: Default::default(),
                gui_metadata: Default::default(),
            })
            .collect::<Vec<_>>();
        let (_layout, width, height) = pack_gui_atlas_layout(&many, 256, 2048).unwrap();
        assert_eq!(width, 2048);
        assert!(height <= 2048);

        let err = pack_gui_atlas_layout(&sources, 256, 512).unwrap_err();
        assert!(err.to_string().contains("exceeds device texture limit"));
    }

    #[test]
    fn pack_gui_atlas_layout_rejects_over_budget_sliver_sprites() {
        // R12-01: 3 tiny sliver sprites (1x8192 + 2x 8192x1) would expand a power-of-two atlas to
        // 16384x16384 (1 GiB RGBA); the byte budget must reject them before allocating pixels.
        let slivers = [
            ("minecraft:gui/tall", 1, 8_192),
            ("minecraft:gui/wide_a", 8_192, 1),
            ("minecraft:gui/wide_b", 8_192, 1),
        ]
        .map(|(id, width, height)| SpriteSource {
            id: id.to_string(),
            width,
            height,
            animation: None,
            texture_metadata: Default::default(),
            gui_metadata: Default::default(),
        });
        let err = pack_gui_atlas_layout(&slivers, 8_194, 16_384).unwrap_err();
        assert!(err.to_string().contains("budget"));
    }
}
