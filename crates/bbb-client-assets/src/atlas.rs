use anyhow::{bail, Result};
use bbb_render_types::{
    AtlasGpuAnimationFrame, AtlasGpuAnimationPlan, AtlasGpuAnimationSprite, AtlasMipPatch,
    AtlasRect, SpriteAnimation, SpriteMipLevel,
};
use serde::{Deserialize, Serialize};

use crate::{
    image::MAX_ATLAS_RGBA_BYTES, mipmap::generate_sprite_rgba_mip_levels,
    resources::PackResourceStack, rgba_len, rgba_offset, sprites::MAX_SPRITE_ANIMATION_FRAMES,
    SpriteGuiMetadata, SpriteImage, SpriteSource, SpriteTextureMetadata, SpriteTransparency,
};

pub const DEFAULT_MAX_ATLAS_HEIGHT: u32 = 16_384;
pub(crate) const MAX_ATLAS_ANIMATION_FRAMES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasSprite {
    pub id: String,
    pub source_width: u32,
    pub source_height: u32,
    pub transparency: SpriteTransparency,
    #[serde(default)]
    pub animation: Option<SpriteAnimation>,
    #[serde(default)]
    pub texture_metadata: SpriteTextureMetadata,
    #[serde(default)]
    pub gui_metadata: SpriteGuiMetadata,
    pub content: AtlasRect,
    pub padded: AtlasRect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasLayout {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub sprites: Vec<AtlasSprite>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasImage {
    pub layout: AtlasLayout,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasMipLevel {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Precomputed source-frame mip chains for an atlas' animated sprites.
///
/// Vanilla 26.1 builds one GPU texture containing every mip for each unique
/// animation frame when the atlas is uploaded. Runtime animation then blits a
/// frame, or interpolates two already-mipped frames, into each atlas mip. Keep
/// the same ordering here so a client tick never regenerates mipmaps from the
/// interpolated base level.
#[derive(Debug, Clone)]
pub struct AtlasAnimationMipCache {
    sprites: Vec<Option<AtlasSpriteAnimationMipCache>>,
    mip_level: u32,
}

#[derive(Debug, Clone)]
struct AtlasSpriteAnimationMipCache {
    id: String,
    animation: SpriteAnimation,
    frames: Vec<Option<Vec<SpriteMipLevel>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasMipImage {
    pub layout: AtlasLayout,
    pub levels: Vec<AtlasMipLevel>,
}

impl AtlasMipImage {
    pub fn base_rgba(&self) -> &[u8] {
        self.levels
            .first()
            .map_or(&[], |level| level.rgba.as_slice())
    }

    pub fn rgba_slices(&self) -> Vec<&[u8]> {
        self.levels
            .iter()
            .map(|level| level.rgba.as_slice())
            .collect()
    }

    pub fn mip_level(&self) -> u32 {
        self.levels.len().saturating_sub(1) as u32
    }

    /// Releases the CPU RGBA mip buffers after GPU upload while retaining
    /// layout and per-level dimension metadata.
    pub fn release_rgba(&mut self) {
        for level in &mut self.levels {
            level.rgba = Vec::new();
        }
    }

    pub fn has_cpu_rgba(&self) -> bool {
        self.levels.iter().all(|level| !level.rgba.is_empty())
    }

    /// Applies dirty atlas regions to the CPU mirror and returns an exact-pixel
    /// upload plan with horizontally compatible regions coalesced.
    ///
    /// The coalesced rectangles may include unchanged texels between dirty
    /// regions, but those bytes are read back from the updated CPU mirror. The
    /// resulting atlas is therefore byte-identical to applying every input
    /// patch in order. `max_overdraw` bounds the uploaded area of each merged
    /// rectangle relative to the dirty regions it contains.
    pub fn apply_and_coalesce_patches(
        &mut self,
        patches: &[AtlasMipPatch],
        max_overdraw: u32,
    ) -> Result<Vec<AtlasMipPatch>> {
        if max_overdraw == 0 {
            bail!("atlas patch max overdraw must be greater than zero");
        }
        for patch in patches {
            validate_mip_patch(self, patch)?;
        }
        for patch in patches {
            apply_mip_patch(self, patch);
        }

        let mut dirty = patches
            .iter()
            .map(|patch| (patch.level, patch.rect))
            .collect::<Vec<_>>();
        dirty.sort_by_key(|(level, rect)| (*level, rect.y, rect.height, rect.x, rect.width));

        let mut merged = Vec::<(u32, AtlasRect, u64)>::new();
        for (level, rect) in dirty {
            let dirty_area = u64::from(rect.width) * u64::from(rect.height);
            let Some((merged_level, merged_rect, merged_dirty_area)) = merged.last_mut() else {
                merged.push((level, rect, dirty_area));
                continue;
            };
            let compatible_row = *merged_level == level
                && merged_rect.y == rect.y
                && merged_rect.height == rect.height;
            let merged_right = u64::from(merged_rect.x) + u64::from(merged_rect.width);
            let rect_right = u64::from(rect.x) + u64::from(rect.width);
            let union_left = u64::from(merged_rect.x.min(rect.x));
            let union_right = merged_right.max(rect_right);
            let union_area = union_right
                .saturating_sub(union_left)
                .saturating_mul(u64::from(rect.height));
            let combined_dirty_area = merged_dirty_area.saturating_add(dirty_area);
            if compatible_row
                && union_area <= combined_dirty_area.saturating_mul(u64::from(max_overdraw))
            {
                merged_rect.x = u32::try_from(union_left).unwrap_or(u32::MAX);
                merged_rect.width = u32::try_from(union_right - union_left).unwrap_or(u32::MAX);
                *merged_dirty_area = combined_dirty_area;
            } else {
                merged.push((level, rect, dirty_area));
            }
        }

        merged
            .into_iter()
            .map(|(level, rect, _)| mip_patch_from_current_level(self, level, rect))
            .collect()
    }
}

fn validate_mip_patch(atlas: &AtlasMipImage, patch: &AtlasMipPatch) -> Result<()> {
    let Some(level) = atlas.levels.iter().find(|level| level.level == patch.level) else {
        bail!("atlas patch references missing mip level {}", patch.level);
    };
    let expected_level_len = rgba_len(level.width, level.height)?;
    if level.rgba.len() != expected_level_len {
        bail!(
            "atlas mip level {} CPU rgba length {} does not match {}x{} dimensions",
            patch.level,
            level.rgba.len(),
            level.width,
            level.height
        );
    }
    if patch.rect.width == 0 || patch.rect.height == 0 {
        bail!("atlas patch dimensions must be greater than zero");
    }
    let right = patch
        .rect
        .x
        .checked_add(patch.rect.width)
        .ok_or_else(|| anyhow::anyhow!("atlas patch width overflow"))?;
    let bottom = patch
        .rect
        .y
        .checked_add(patch.rect.height)
        .ok_or_else(|| anyhow::anyhow!("atlas patch height overflow"))?;
    if right > level.width || bottom > level.height {
        bail!(
            "atlas patch {}x{} at {},{} exceeds mip {} dimensions {}x{}",
            patch.rect.width,
            patch.rect.height,
            patch.rect.x,
            patch.rect.y,
            patch.level,
            level.width,
            level.height
        );
    }
    let expected_len = rgba_len(patch.rect.width, patch.rect.height)?;
    if patch.rgba.len() != expected_len {
        bail!(
            "atlas patch rgba length {} does not match {}x{} dimensions",
            patch.rgba.len(),
            patch.rect.width,
            patch.rect.height
        );
    }
    Ok(())
}

fn apply_mip_patch(atlas: &mut AtlasMipImage, patch: &AtlasMipPatch) {
    let level = atlas
        .levels
        .iter_mut()
        .find(|level| level.level == patch.level)
        .expect("validated atlas mip patch level");
    for row in 0..patch.rect.height {
        let source_start = (row * patch.rect.width * 4) as usize;
        let source_end = source_start + (patch.rect.width * 4) as usize;
        let target_start = (((patch.rect.y + row) * level.width + patch.rect.x) * 4) as usize;
        let target_end = target_start + (patch.rect.width * 4) as usize;
        level.rgba[target_start..target_end].copy_from_slice(&patch.rgba[source_start..source_end]);
    }
}

fn mip_patch_from_current_level(
    atlas: &AtlasMipImage,
    level_index: u32,
    rect: AtlasRect,
) -> Result<AtlasMipPatch> {
    let level = atlas
        .levels
        .iter()
        .find(|level| level.level == level_index)
        .ok_or_else(|| anyhow::anyhow!("missing atlas mip level {level_index}"))?;
    let mut rgba = Vec::with_capacity(rgba_len(rect.width, rect.height)?);
    for row in 0..rect.height {
        let start = (((rect.y + row) * level.width + rect.x) * 4) as usize;
        let end = start + (rect.width * 4) as usize;
        rgba.extend_from_slice(&level.rgba[start..end]);
    }
    Ok(AtlasMipPatch {
        level: level_index,
        rect,
        rgba,
    })
}

impl AtlasLayout {
    pub fn prepare_gpu_animation_plan(
        &self,
        images: &[SpriteImage],
        mip_level: u32,
    ) -> Result<Option<AtlasGpuAnimationPlan>> {
        let cache = self.prepare_animation_mip_cache(images, mip_level)?;
        cache.into_gpu_plan(self)
    }

    pub fn prepare_gpu_animation_plan_with_stack(
        &self,
        images: &[SpriteImage],
        mip_level: u32,
        stack: &PackResourceStack,
    ) -> Result<Option<AtlasGpuAnimationPlan>> {
        let cache = self.prepare_animation_mip_cache_with_stack(images, mip_level, stack)?;
        cache.into_gpu_plan(self)
    }

    pub fn prepare_animation_mip_cache(
        &self,
        images: &[SpriteImage],
        mip_level: u32,
    ) -> Result<AtlasAnimationMipCache> {
        self.prepare_animation_mip_cache_borrowed(images, mip_level, None)
    }

    pub fn prepare_animation_mip_cache_with_stack(
        &self,
        images: &[SpriteImage],
        mip_level: u32,
        stack: &PackResourceStack,
    ) -> Result<AtlasAnimationMipCache> {
        self.prepare_animation_mip_cache_borrowed(images, mip_level, Some(stack))
    }

    pub fn prepare_animation_mip_cache_refs(
        &self,
        images: &[&SpriteImage],
        mip_level: u32,
    ) -> Result<AtlasAnimationMipCache> {
        self.prepare_animation_mip_cache_borrowed(images, mip_level, None)
    }

    fn prepare_animation_mip_cache_borrowed<T: std::borrow::Borrow<SpriteImage> + Sync>(
        &self,
        images: &[T],
        mip_level: u32,
        stack: Option<&PackResourceStack>,
    ) -> Result<AtlasAnimationMipCache> {
        if images.len() != self.sprites.len() {
            bail!(
                "atlas animation has {} images for {} sprites",
                images.len(),
                self.sprites.len()
            );
        }
        let mut sprites = Vec::with_capacity(images.len());
        let mut tasks = Vec::new();
        let mut total_mip_bytes = 0_u64;
        for (sprite_index, (sprite, image_ref)) in self.sprites.iter().zip(images).enumerate() {
            let image: &SpriteImage = image_ref.borrow();
            if sprite.id != image.id {
                bail!(
                    "atlas animation sprite order mismatch: layout {}, image {}",
                    sprite.id,
                    image.id
                );
            }
            let Some(animation) = image.animation.as_ref() else {
                sprites.push(None);
                continue;
            };
            if animation.frame_count > MAX_SPRITE_ANIMATION_FRAMES {
                bail!(
                    "animated sprite {} has {} frames, exceeding the {MAX_SPRITE_ANIMATION_FRAMES} frame limit",
                    image.id,
                    animation.frame_count
                );
            }
            if animation.frames.len() > MAX_SPRITE_ANIMATION_FRAMES as usize {
                bail!(
                    "animated sprite {} has {} frame entries, exceeding the {MAX_SPRITE_ANIMATION_FRAMES} frame limit",
                    image.id,
                    animation.frames.len()
                );
            }
            let frame_count = usize::try_from(animation.frame_count).map_err(|_| {
                anyhow::anyhow!("animated sprite {} frame count overflow", image.id)
            })?;
            let frame_mip_bytes =
                sprite_mip_chain_rgba_bytes(image.width, image.height, mip_level)?;
            let mut seen = vec![false; frame_count];
            for frame in &animation.frames {
                let frame_index = usize::try_from(frame.index).map_err(|_| {
                    anyhow::anyhow!("animated sprite {} frame index overflow", image.id)
                })?;
                let Some(flag) = seen.get_mut(frame_index) else {
                    bail!(
                        "animated sprite {} frame {} exceeds declared frame count {}",
                        image.id,
                        frame.index,
                        animation.frame_count
                    );
                };
                if *flag {
                    continue;
                }
                if image.frame_rgba(frame.index).is_none() {
                    bail!("animated sprite {} missing frame {}", image.id, frame.index);
                }
                *flag = true;
                if tasks.len() >= MAX_ATLAS_ANIMATION_FRAMES {
                    bail!(
                        "atlas animation cache exceeds the {MAX_ATLAS_ANIMATION_FRAMES} total frame limit"
                    );
                }
                total_mip_bytes = total_mip_bytes
                    .checked_add(frame_mip_bytes)
                    .filter(|&bytes| bytes <= MAX_ATLAS_RGBA_BYTES)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "atlas animation mip cache would exceed the {MAX_ATLAS_RGBA_BYTES} RGBA byte limit"
                        )
                    })?;
                tasks.push((sprite_index, frame_index));
            }
            sprites.push(Some(AtlasSpriteAnimationMipCache {
                id: image.id.clone(),
                animation: animation.clone(),
                frames: vec![None; frame_count],
            }));
        }
        if let Some(stack) = stack {
            stack.reserve_image_bytes(total_mip_bytes, "atlas animation mip cache")?;
        }
        let generated = {
            let build_frame = |&(sprite_index, frame_index): &(usize, usize)| {
                let image: &SpriteImage = images[sprite_index].borrow();
                let source_rgba = image
                    .frame_rgba(frame_index as u32)
                    .expect("validated animated sprite frame");
                let mips = generate_sprite_rgba_mip_levels(
                    &image.id,
                    image.width,
                    image.height,
                    source_rgba,
                    image.texture_metadata,
                    image.transparency,
                    mip_level,
                )?;
                Ok((sprite_index, frame_index, mips))
            };
            if crate::can_use_rayon() {
                use rayon::prelude::*;
                tasks
                    .par_iter()
                    .map(build_frame)
                    .collect::<Vec<Result<_>>>()
            } else {
                tasks.iter().map(build_frame).collect::<Vec<Result<_>>>()
            }
        };
        for item in generated {
            let (sprite_index, frame_index, mips) = item?;
            sprites[sprite_index]
                .as_mut()
                .expect("animated sprite entry")
                .frames[frame_index] = Some(mips);
        }
        Ok(AtlasAnimationMipCache { sprites, mip_level })
    }

    /// Builds only the animated sprite regions whose visual contents changed.
    ///
    /// Vanilla 26.1's `TextureAtlas.cycleAnimationFrames` draws dirty (or
    /// interpolated) `AnimationState`s into each atlas mip instead of
    /// rebuilding the complete atlas. The returned rectangles include the
    /// mip-aligned replicated gutter used by this packer, so applying every
    /// patch produces the same bytes as a full animated-frame stitch.
    #[cfg(test)]
    pub fn animation_frame_mip_patches(
        &self,
        images: &[SpriteImage],
        previous_tick: u64,
        tick: u64,
        mip_level: u32,
    ) -> Result<Vec<AtlasMipPatch>> {
        if images.len() != self.sprites.len() {
            bail!(
                "atlas animation has {} images for {} sprites",
                images.len(),
                self.sprites.len()
            );
        }
        let mut patches = Vec::new();
        for (sprite, image) in self.sprites.iter().zip(images) {
            if sprite.id != image.id {
                bail!(
                    "atlas animation sprite order mismatch: layout {}, image {}",
                    sprite.id,
                    image.id
                );
            }
            let Some(animation) = image.animation.as_ref() else {
                continue;
            };
            let previous_frame = animation.frame_index_at_tick(previous_tick);
            let current_frame = animation.frame_index_at_tick(tick);
            if !animation.interpolate && previous_frame == current_frame {
                continue;
            }
            let source_rgba = image.frame_rgba_at_tick(tick)?;
            let mips = generate_sprite_rgba_mip_levels(
                &image.id,
                image.width,
                image.height,
                source_rgba.as_ref(),
                image.texture_metadata,
                image.transparency,
                mip_level,
            )?;
            for (level, mip) in mips.iter().enumerate() {
                patches.push(sprite_mip_patch(sprite, level as u32, mip)?);
            }
        }
        Ok(patches)
    }
}

impl AtlasAnimationMipCache {
    fn into_gpu_plan(self, layout: &AtlasLayout) -> Result<Option<AtlasGpuAnimationPlan>> {
        if self.sprites.len() != layout.sprites.len() {
            bail!(
                "atlas animation cache has {} sprites for {} layout sprites",
                self.sprites.len(),
                layout.sprites.len()
            );
        }
        let mut sprites = Vec::new();
        for (sprite, cached) in layout.sprites.iter().zip(self.sprites) {
            let Some(cached) = cached else {
                continue;
            };
            if sprite.id != cached.id {
                bail!(
                    "atlas animation cache order mismatch: layout {}, cache {}",
                    sprite.id,
                    cached.id
                );
            }
            let frames = cached
                .frames
                .into_iter()
                .enumerate()
                .filter_map(|(index, mips)| {
                    mips.map(|mips| AtlasGpuAnimationFrame {
                        index: index as u32,
                        mips,
                    })
                })
                .collect::<Vec<_>>();
            sprites.push(AtlasGpuAnimationSprite {
                id: cached.id,
                destination: sprite.padded,
                content_width: sprite.content.width,
                content_height: sprite.content.height,
                animation: cached.animation,
                frames,
            });
        }
        if sprites.is_empty() {
            return Ok(None);
        }
        Ok(Some(AtlasGpuAnimationPlan {
            atlas_width: layout.width,
            atlas_height: layout.height,
            padding: layout.padding,
            mip_level: self.mip_level,
            sprites,
        }))
    }

    pub fn frame_mip_patches(
        &self,
        layout: &AtlasLayout,
        previous_tick: u64,
        tick: u64,
    ) -> Result<Vec<AtlasMipPatch>> {
        if self.sprites.len() != layout.sprites.len() {
            bail!(
                "atlas animation cache has {} sprites for {} layout sprites",
                self.sprites.len(),
                layout.sprites.len()
            );
        }
        let mut patches = Vec::new();
        for (sprite, cached) in layout.sprites.iter().zip(&self.sprites) {
            let Some(cached) = cached else {
                continue;
            };
            if sprite.id != cached.id {
                bail!(
                    "atlas animation cache order mismatch: layout {}, cache {}",
                    sprite.id,
                    cached.id
                );
            }
            let previous_frame = cached.animation.frame_index_at_tick(previous_tick);
            let current = cached
                .animation
                .frame_at_tick(tick)
                .ok_or_else(|| anyhow::anyhow!("animated sprite {} has no frames", cached.id))?;
            if !cached.animation.interpolate && previous_frame == Some(current.frame_index) {
                continue;
            }
            let current_mips = cached.frame_mips(current.frame_index)?;
            if cached.animation.interpolate {
                let next_mips = cached.frame_mips(current.next_frame_index)?;
                let progress = vanilla_animation_progress(current.frame_progress);
                for (level, (current_mip, next_mip)) in
                    current_mips.iter().zip(next_mips).enumerate()
                {
                    let mip = interpolate_sprite_mip(current_mip, next_mip, progress)?;
                    patches.push(sprite_mip_patch(sprite, level as u32, &mip)?);
                }
            } else {
                for (level, mip) in current_mips.iter().enumerate() {
                    patches.push(sprite_mip_patch(sprite, level as u32, mip)?);
                }
            }
        }
        Ok(patches)
    }

    pub fn mip_level(&self) -> u32 {
        self.mip_level
    }
}

impl AtlasSpriteAnimationMipCache {
    fn frame_mips(&self, frame_index: u32) -> Result<&[SpriteMipLevel]> {
        self.frames
            .get(frame_index as usize)
            .and_then(Option::as_deref)
            .ok_or_else(|| {
                anyhow::anyhow!("animated sprite {} missing frame {}", self.id, frame_index)
            })
    }
}

fn vanilla_animation_progress(progress: f32) -> f32 {
    // `AnimationState.drawToAtlas` truncates progress to an integer in
    // thousandths and encodes it in `gl_VertexID` before the shader mix.
    ((progress.clamp(0.0, 1.0) * 1000.0) as u32) as f32 / 1000.0
}

fn interpolate_sprite_mip(
    current: &SpriteMipLevel,
    next: &SpriteMipLevel,
    progress: f32,
) -> Result<SpriteMipLevel> {
    if (current.width, current.height, current.rgba.len())
        != (next.width, next.height, next.rgba.len())
    {
        bail!(
            "animated sprite mip mismatch: {}x{} ({} bytes) vs {}x{} ({} bytes)",
            current.width,
            current.height,
            current.rgba.len(),
            next.width,
            next.height,
            next.rgba.len()
        );
    }
    let rgba = current
        .rgba
        .iter()
        .zip(&next.rgba)
        .map(|(current, next)| {
            let current = f32::from(*current);
            let next = f32::from(*next);
            // The interpolated channel is nonnegative and progress has thousandth precision.
            (current + (next - current) * progress + 0.5) as u8
        })
        .collect();
    Ok(SpriteMipLevel {
        width: current.width,
        height: current.height,
        rgba,
    })
}

// Vanilla `Stitcher.java:24-29, 79-88` bounds atlas growth by both `maxWidth` and `maxHeight`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasPacker {
    max_width: u32,
    max_height: u32,
    padding: u32,
}

impl AtlasPacker {
    pub fn new(max_width: u32, padding: u32) -> Result<Self> {
        Self::new_bounded(max_width, DEFAULT_MAX_ATLAS_HEIGHT, padding)
    }

    pub fn new_bounded(max_width: u32, max_height: u32, padding: u32) -> Result<Self> {
        if max_width == 0 {
            bail!("atlas max width must be greater than zero");
        }
        if max_height == 0 {
            bail!("atlas max height must be greater than zero");
        }
        Ok(Self {
            max_width,
            max_height,
            padding,
        })
    }

    pub fn with_max_height(mut self, max_height: u32) -> Result<Self> {
        if max_height == 0 {
            bail!("atlas max height must be greater than zero");
        }
        self.max_height = max_height;
        Ok(self)
    }

    pub fn max_height(&self) -> u32 {
        self.max_height
    }

    pub fn pack(&self, sources: &[SpriteSource]) -> Result<AtlasLayout> {
        self.pack_aligned(sources, self.padding, 1, false)
    }

    fn pack_for_mip(&self, sources: &[SpriteSource], mip_level: u32) -> Result<AtlasLayout> {
        let alignment = 1_u32
            .checked_shl(mip_level)
            .ok_or_else(|| anyhow::anyhow!("atlas mip alignment overflow"))?;
        let padding = self
            .padding
            .checked_mul(alignment)
            .ok_or_else(|| anyhow::anyhow!("atlas mip padding overflow"))?;
        self.pack_aligned(sources, padding, alignment, true)
    }

    fn pack_aligned(
        &self,
        sources: &[SpriteSource],
        padding: u32,
        alignment: u32,
        power_of_two_dimensions: bool,
    ) -> Result<AtlasLayout> {
        let mut prepared = Vec::with_capacity(sources.len());
        let gutter = if sources.is_empty() {
            0
        } else {
            match padding.checked_mul(2) {
                Some(gutter) => gutter,
                None => bail!("atlas padding overflow"),
            }
        };

        for (index, source) in sources.iter().enumerate() {
            if source.width == 0 || source.height == 0 {
                bail!("sprite {} has zero-sized dimensions", source.id);
            }

            let raw_padded_width = match source.width.checked_add(gutter) {
                Some(width) => width,
                None => bail!("sprite {} padded width overflow", source.id),
            };
            let raw_padded_height = match source.height.checked_add(gutter) {
                Some(height) => height,
                None => bail!("sprite {} padded height overflow", source.id),
            };
            let padded_width = align_up(raw_padded_width, alignment)
                .ok_or_else(|| anyhow::anyhow!("sprite {} padded width overflow", source.id))?;
            let padded_height = align_up(raw_padded_height, alignment)
                .ok_or_else(|| anyhow::anyhow!("sprite {} padded height overflow", source.id))?;
            if padded_width > self.max_width {
                bail!(
                    "sprite {} padded width {} exceeds atlas max width {}",
                    source.id,
                    padded_width,
                    self.max_width
                );
            }
            if padded_height > self.max_height {
                bail!(
                    "sprite {} padded height {} exceeds atlas max height {}",
                    source.id,
                    padded_height,
                    self.max_height
                );
            }
            prepared.push((index, padded_width, padded_height));
        }

        // Pack shelves by descending padded height (tie-breaking on descending
        // width, then original index) to minimize wasted shelf height while
        // preserving `AtlasLayout::sprites` in caller input order.
        prepared.sort_unstable_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.0.cmp(&right.0))
        });

        let zero_rect = AtlasRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        let mut placements = vec![(zero_rect, zero_rect); sources.len()];
        let mut cursor_x: u32 = 0;
        let mut cursor_y: u32 = 0;
        let mut row_height: u32 = 0;
        let mut used_width: u32 = 0;

        for (index, padded_width, padded_height) in prepared {
            let source = &sources[index];
            let row_width = match cursor_x.checked_add(padded_width) {
                Some(width) => width,
                None => bail!("atlas row width overflow"),
            };
            if cursor_x > 0 && row_width > self.max_width {
                cursor_y = match cursor_y.checked_add(row_height) {
                    Some(y) => y,
                    None => bail!("atlas height overflow"),
                };
                cursor_x = 0;
                row_height = 0;
            }

            let padded = AtlasRect {
                x: cursor_x,
                y: cursor_y,
                width: padded_width,
                height: padded_height,
            };
            let content = AtlasRect {
                x: cursor_x + padding,
                y: cursor_y + padding,
                width: source.width,
                height: source.height,
            };
            placements[index] = (content, padded);

            cursor_x = match cursor_x.checked_add(padded_width) {
                Some(x) => x,
                None => bail!("atlas row width overflow"),
            };
            row_height = row_height.max(padded_height);
            used_width = used_width.max(cursor_x);
        }

        let sprites = sources
            .iter()
            .zip(placements)
            .map(|(source, (content, padded))| AtlasSprite {
                id: source.id.clone(),
                source_width: source.width,
                source_height: source.height,
                transparency: SpriteTransparency::default(),
                animation: source.animation.clone(),
                texture_metadata: source.texture_metadata,
                gui_metadata: source.gui_metadata,
                content,
                padded,
            })
            .collect();

        let used_height = match cursor_y.checked_add(row_height) {
            Some(height) => height,
            None => bail!("atlas height overflow"),
        };
        let (width, height) = if power_of_two_dimensions {
            (
                checked_next_power_of_two(used_width, "width")?,
                checked_next_power_of_two(used_height, "height")?,
            )
        } else {
            (used_width, used_height)
        };
        if width > self.max_width {
            bail!(
                "power-of-two atlas width {} exceeds atlas max width {}",
                width,
                self.max_width
            );
        }
        if height > self.max_height {
            bail!(
                "atlas height {} exceeds atlas max height {}",
                height,
                self.max_height
            );
        }
        Ok(AtlasLayout {
            width,
            height,
            padding,
            sprites,
        })
    }

    pub fn stitch(&self, images: &[SpriteImage]) -> Result<AtlasImage> {
        self.stitch_with_animation_tick_borrowed(images, None, None)
    }

    pub fn stitch_with_stack(
        &self,
        images: &[SpriteImage],
        stack: &PackResourceStack,
    ) -> Result<AtlasImage> {
        self.stitch_with_animation_tick_borrowed(images, None, Some(stack))
    }

    pub fn stitch_refs(&self, images: &[&SpriteImage]) -> Result<AtlasImage> {
        self.stitch_with_animation_tick_borrowed(images, None, None)
    }

    pub fn stitch_refs_with_stack(
        &self,
        images: &[&SpriteImage],
        stack: &PackResourceStack,
    ) -> Result<AtlasImage> {
        self.stitch_with_animation_tick_borrowed(images, None, Some(stack))
    }

    pub fn stitch_animation_frame(&self, images: &[SpriteImage], tick: u64) -> Result<AtlasImage> {
        self.stitch_with_animation_tick_borrowed(images, Some(tick), None)
    }

    pub fn stitch_mips(&self, images: &[SpriteImage], mip_level: u32) -> Result<AtlasMipImage> {
        self.stitch_with_animation_tick_mips(images, None, mip_level, None)
    }

    pub fn stitch_mips_with_stack(
        &self,
        images: &[SpriteImage],
        mip_level: u32,
        stack: &PackResourceStack,
    ) -> Result<AtlasMipImage> {
        self.stitch_with_animation_tick_mips(images, None, mip_level, Some(stack))
    }

    pub fn stitch_mips_with_max_level(
        &self,
        images: &[SpriteImage],
        max_mipmap_levels: u32,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mip_level = vanilla_mip_level(&sources, max_mipmap_levels);
        self.stitch_with_animation_tick_mips(images, None, mip_level, None)
    }

    pub fn stitch_mips_with_max_level_and_stack(
        &self,
        images: &[SpriteImage],
        max_mipmap_levels: u32,
        stack: &PackResourceStack,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mip_level = vanilla_mip_level(&sources, max_mipmap_levels);
        self.stitch_with_animation_tick_mips(images, None, mip_level, Some(stack))
    }

    pub fn stitch_animation_frame_mips(
        &self,
        images: &[SpriteImage],
        tick: u64,
        mip_level: u32,
    ) -> Result<AtlasMipImage> {
        self.stitch_with_animation_tick_mips(images, Some(tick), mip_level, None)
    }

    pub fn stitch_animation_frame_mips_with_max_level(
        &self,
        images: &[SpriteImage],
        tick: u64,
        max_mipmap_levels: u32,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mip_level = vanilla_mip_level(&sources, max_mipmap_levels);
        self.stitch_with_animation_tick_mips(images, Some(tick), mip_level, None)
    }

    fn stitch_with_animation_tick_borrowed<T: std::borrow::Borrow<SpriteImage>>(
        &self,
        images: &[T],
        tick: Option<u64>,
        stack: Option<&PackResourceStack>,
    ) -> Result<AtlasImage> {
        let sources = images
            .iter()
            .map(|image| image.borrow().source())
            .collect::<Vec<_>>();
        let mut layout = self.pack(&sources)?;
        for (sprite, image_ref) in layout.sprites.iter_mut().zip(images) {
            let image = image_ref.borrow();
            sprite.transparency = image.transparency;
            sprite.animation = image.animation.clone();
            sprite.texture_metadata = image.texture_metadata;
            sprite.gui_metadata = image.gui_metadata;
        }
        let rgba_bytes_usize = rgba_len(layout.width, layout.height)?;
        let rgba_bytes = u64::try_from(rgba_bytes_usize)
            .map_err(|_| anyhow::anyhow!("stitched atlas RGBA size overflow"))?;
        if rgba_bytes > MAX_ATLAS_RGBA_BYTES {
            bail!(
                "stitched atlas {}x{} would use {rgba_bytes} RGBA bytes, exceeding the {MAX_ATLAS_RGBA_BYTES} byte limit",
                layout.width,
                layout.height
            );
        }
        if let Some(stack) = stack {
            stack.reserve_image_bytes(rgba_bytes, "stitched atlas")?;
        }
        let mut rgba = vec![0; rgba_bytes_usize];

        for (sprite, image_ref) in layout.sprites.iter().zip(images) {
            copy_sprite_with_gutter(&mut rgba, layout.width, sprite, image_ref.borrow(), tick)?;
        }

        Ok(AtlasImage { layout, rgba })
    }

    fn stitch_with_animation_tick_mips(
        &self,
        images: &[SpriteImage],
        tick: Option<u64>,
        mip_level: u32,
        stack: Option<&PackResourceStack>,
    ) -> Result<AtlasMipImage> {
        let sources = images.iter().map(SpriteImage::source).collect::<Vec<_>>();
        let mut layout = self.pack_for_mip(&sources, mip_level)?;
        for (sprite, image) in layout.sprites.iter_mut().zip(images) {
            sprite.transparency = image.transparency;
            sprite.animation = image.animation.clone();
            sprite.texture_metadata = image.texture_metadata;
            sprite.gui_metadata = image.gui_metadata;
        }

        let mut level_specs = Vec::with_capacity(mip_level as usize + 1);
        let mut total_mip_bytes = 0_u64;
        for level in 0..=mip_level {
            let width = layout.width.checked_shr(level).unwrap_or(0);
            let height = layout.height.checked_shr(level).unwrap_or(0);
            if width == 0 || height == 0 {
                bail!(
                    "atlas mip level {} has zero-sized dimensions from {}x{}",
                    level,
                    layout.width,
                    layout.height
                );
            }
            let level_bytes_usize = rgba_len(width, height)?;
            let level_bytes = u64::try_from(level_bytes_usize)
                .map_err(|_| anyhow::anyhow!("atlas mip level RGBA size overflow"))?;
            total_mip_bytes = total_mip_bytes
                .checked_add(level_bytes)
                .filter(|&bytes| bytes <= MAX_ATLAS_RGBA_BYTES)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "stitched atlas mip chain ({}x{}, {} levels) would exceed the {MAX_ATLAS_RGBA_BYTES} RGBA byte limit",
                        layout.width,
                        layout.height,
                        mip_level + 1
                    )
                })?;
            level_specs.push((level, width, height, level_bytes_usize));
        }
        if let Some(stack) = stack {
            stack.reserve_image_bytes(total_mip_bytes, "stitched atlas mips")?;
        }

        let sprite_mips = {
            let build_mips = |image: &SpriteImage| {
                let source_rgba = match tick {
                    Some(tick) => image.frame_rgba_at_tick(tick)?,
                    None => std::borrow::Cow::Borrowed(image.rgba.as_slice()),
                };
                generate_sprite_rgba_mip_levels(
                    &image.id,
                    image.width,
                    image.height,
                    source_rgba.as_ref(),
                    image.texture_metadata,
                    image.transparency,
                    mip_level,
                )
            };
            if crate::can_use_rayon() {
                use rayon::prelude::*;
                images
                    .par_iter()
                    .map(build_mips)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            } else {
                images.iter().map(build_mips).collect::<Result<Vec<_>>>()?
            }
        };

        let mut levels = Vec::with_capacity(level_specs.len());
        for (level, width, height, level_bytes_usize) in level_specs {
            let mut rgba = vec![0; level_bytes_usize];
            for (sprite, mips) in layout.sprites.iter().zip(&sprite_mips) {
                let mip = &mips[level as usize];
                copy_sprite_mip_with_gutter(&mut rgba, width, sprite, level, mip)?;
            }
            levels.push(AtlasMipLevel {
                level,
                width,
                height,
                rgba,
            });
        }

        Ok(AtlasMipImage { layout, levels })
    }
}

fn sprite_mip_chain_rgba_bytes(width: u32, height: u32, mip_level: u32) -> Result<u64> {
    let mut total = 0_u64;
    for level in 0..=mip_level {
        let mip_width = (width >> level).max(1);
        let mip_height = (height >> level).max(1);
        let bytes = u64::from(mip_width)
            .checked_mul(u64::from(mip_height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("sprite mip chain RGBA size overflow"))?;
        total = total
            .checked_add(bytes)
            .ok_or_else(|| anyhow::anyhow!("sprite mip chain RGBA size overflow"))?;
    }
    Ok(total)
}

fn align_up(value: u32, alignment: u32) -> Option<u32> {
    debug_assert!(alignment.is_power_of_two());
    value
        .checked_add(alignment.checked_sub(1)?)
        .map(|value| value & !(alignment - 1))
}

fn checked_next_power_of_two(value: u32, dimension: &str) -> Result<u32> {
    if value == 0 {
        bail!("atlas {dimension} must be greater than zero");
    }
    value
        .checked_next_power_of_two()
        .ok_or_else(|| anyhow::anyhow!("atlas {dimension} power-of-two overflow"))
}

fn copy_sprite_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    image: &SpriteImage,
    tick: Option<u64>,
) -> Result<()> {
    let source_rgba = match tick {
        Some(tick) => image.frame_rgba_at_tick(tick)?,
        None => std::borrow::Cow::Borrowed(image.rgba.as_slice()),
    };
    copy_sprite_rgba_with_gutter(
        atlas,
        atlas_width,
        sprite,
        image.width,
        image.height,
        source_rgba.as_ref(),
    )
}

fn copy_sprite_rgba_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    source_width: u32,
    source_height: u32,
    source_rgba: &[u8],
) -> Result<()> {
    copy_sprite_rect_rgba_with_gutter(
        atlas,
        atlas_width,
        &sprite.id,
        sprite.content,
        sprite.padded,
        RgbaCopySource {
            width: source_width,
            height: source_height,
            rgba: source_rgba,
        },
    )
}

fn copy_sprite_mip_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite: &AtlasSprite,
    level: u32,
    mip: &SpriteMipLevel,
) -> Result<()> {
    let content = AtlasRect {
        x: sprite.content.x.checked_shr(level).unwrap_or(0),
        y: sprite.content.y.checked_shr(level).unwrap_or(0),
        width: mip.width,
        height: mip.height,
    };
    let mut padded = mip_rect(sprite.padded, level)?;
    let content_right = content
        .x
        .checked_add(content.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip content width overflow"))?;
    let content_bottom = content
        .y
        .checked_add(content.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip content height overflow"))?;
    if content.x < padded.x || content.y < padded.y {
        bail!(
            "sprite {} mip level {} content is outside padding",
            sprite.id,
            level
        );
    }
    let padded_right = padded
        .x
        .checked_add(padded.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip padded width overflow"))?;
    let padded_bottom = padded
        .y
        .checked_add(padded.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip padded height overflow"))?;
    if content_right > padded_right {
        padded.width = content_right - padded.x;
    }
    if content_bottom > padded_bottom {
        padded.height = content_bottom - padded.y;
    }
    copy_sprite_rect_rgba_with_gutter(
        atlas,
        atlas_width,
        &sprite.id,
        content,
        padded,
        RgbaCopySource {
            width: mip.width,
            height: mip.height,
            rgba: &mip.rgba,
        },
    )
}

fn sprite_mip_patch(
    sprite: &AtlasSprite,
    level: u32,
    mip: &SpriteMipLevel,
) -> Result<AtlasMipPatch> {
    let content = AtlasRect {
        x: sprite.content.x.checked_shr(level).unwrap_or(0),
        y: sprite.content.y.checked_shr(level).unwrap_or(0),
        width: mip.width,
        height: mip.height,
    };
    let mut padded = mip_rect(sprite.padded, level)?;
    let content_right = content
        .x
        .checked_add(content.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip patch content width overflow"))?;
    let content_bottom = content
        .y
        .checked_add(content.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip patch content height overflow"))?;
    if content.x < padded.x || content.y < padded.y {
        bail!(
            "sprite {} mip level {} patch content is outside padding",
            sprite.id,
            level
        );
    }
    let padded_right = padded
        .x
        .checked_add(padded.width)
        .ok_or_else(|| anyhow::anyhow!("atlas mip patch padded width overflow"))?;
    let padded_bottom = padded
        .y
        .checked_add(padded.height)
        .ok_or_else(|| anyhow::anyhow!("atlas mip patch padded height overflow"))?;
    if content_right > padded_right {
        padded.width = content_right - padded.x;
    }
    if content_bottom > padded_bottom {
        padded.height = content_bottom - padded.y;
    }
    let patch_content = AtlasRect {
        x: content.x - padded.x,
        y: content.y - padded.y,
        width: content.width,
        height: content.height,
    };
    let patch_bounds = AtlasRect {
        x: 0,
        y: 0,
        width: padded.width,
        height: padded.height,
    };
    let mut rgba = vec![0; rgba_len(padded.width, padded.height)?];
    copy_sprite_rect_rgba_with_gutter(
        &mut rgba,
        padded.width,
        &sprite.id,
        patch_content,
        patch_bounds,
        RgbaCopySource {
            width: mip.width,
            height: mip.height,
            rgba: &mip.rgba,
        },
    )?;
    Ok(AtlasMipPatch {
        level,
        rect: padded,
        rgba,
    })
}

struct RgbaCopySource<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

fn copy_sprite_rect_rgba_with_gutter(
    atlas: &mut [u8],
    atlas_width: u32,
    sprite_id: &str,
    content: AtlasRect,
    padded: AtlasRect,
    source: RgbaCopySource<'_>,
) -> Result<()> {
    let expected_len = rgba_len(source.width, source.height)?;
    if source.rgba.len() != expected_len {
        bail!(
            "sprite {} frame has {} RGBA bytes, expected {}",
            sprite_id,
            source.rgba.len(),
            expected_len
        );
    }
    let content_offset_x = content
        .x
        .checked_sub(padded.x)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} content starts before padding"))?;
    let content_offset_y = content
        .y
        .checked_sub(padded.y)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} content starts before padding"))?;
    if padded.width == 0 || padded.height == 0 {
        return Ok(());
    }
    if source.width == 0 || source.height == 0 {
        bail!("sprite {sprite_id} has zero-sized source dimensions");
    }
    let padded_right = padded
        .x
        .checked_add(padded.width)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} padded x overflow"))?;
    let padded_bottom = padded
        .y
        .checked_add(padded.height)
        .ok_or_else(|| anyhow::anyhow!("sprite {sprite_id} padded y overflow"))?;
    let last_offset = rgba_offset(atlas_width, padded_right - 1, padded_bottom - 1)?;
    if last_offset + 4 > atlas.len() {
        bail!("sprite {sprite_id} padded destination exceeds atlas buffer");
    }

    let atlas_row_bytes = (atlas_width as usize) * 4;
    let src_row_bytes = (source.width as usize) * 4;
    let padded_x_bytes = (padded.x as usize) * 4;
    let padded_row_bytes = (padded.width as usize) * 4;
    let left_len = content_offset_x.min(padded.width) as usize;
    let copy_px = source
        .width
        .min(padded.width.saturating_sub(content_offset_x)) as usize;
    let right_start = left_len + copy_px;
    let right_len = (padded.width as usize) - right_start;

    let write_row = |atlas: &mut [u8], dst_y: u32, source_y: u32| {
        let src_start = (source_y as usize) * src_row_bytes;
        let src_row = &source.rgba[src_start..src_start + src_row_bytes];
        let dst_row_start = (dst_y as usize) * atlas_row_bytes + padded_x_bytes;
        let dst_row = &mut atlas[dst_row_start..dst_row_start + padded_row_bytes];
        if left_len > 0 {
            let left_px = &src_row[..4];
            for px in dst_row[..left_len * 4].as_chunks_mut::<4>().0 {
                px.copy_from_slice(left_px);
            }
        }
        if copy_px > 0 {
            let content_start = left_len * 4;
            let content_bytes = copy_px * 4;
            dst_row[content_start..content_start + content_bytes]
                .copy_from_slice(&src_row[..content_bytes]);
        }
        if right_len > 0 {
            let right_px = &src_row[src_row_bytes - 4..];
            for px in dst_row[right_start * 4..].as_chunks_mut::<4>().0 {
                px.copy_from_slice(right_px);
            }
        }
    };

    let top_rows = content_offset_y.min(padded.height);
    let content_rows = source.height.min(padded.height.saturating_sub(top_rows));
    if content_rows > 0 {
        for row in 0..content_rows {
            write_row(atlas, padded.y + top_rows + row, row);
        }
        let first_content_offset =
            ((padded.y + top_rows) as usize) * atlas_row_bytes + padded_x_bytes;
        for gy in 0..top_rows {
            let dst_offset = ((padded.y + gy) as usize) * atlas_row_bytes + padded_x_bytes;
            atlas.copy_within(
                first_content_offset..first_content_offset + padded_row_bytes,
                dst_offset,
            );
        }
        let last_content_offset =
            ((padded.y + top_rows + content_rows - 1) as usize) * atlas_row_bytes + padded_x_bytes;
        for gy in (top_rows + content_rows)..padded.height {
            let dst_offset = ((padded.y + gy) as usize) * atlas_row_bytes + padded_x_bytes;
            atlas.copy_within(
                last_content_offset..last_content_offset + padded_row_bytes,
                dst_offset,
            );
        }
    } else {
        write_row(atlas, padded.y, 0);
        let first_offset = (padded.y as usize) * atlas_row_bytes + padded_x_bytes;
        for gy in 1..padded.height {
            let dst_offset = ((padded.y + gy) as usize) * atlas_row_bytes + padded_x_bytes;
            atlas.copy_within(first_offset..first_offset + padded_row_bytes, dst_offset);
        }
    }
    Ok(())
}

fn mip_rect(rect: AtlasRect, level: u32) -> Result<AtlasRect> {
    let x = rect.x.checked_shr(level).unwrap_or(0);
    let y = rect.y.checked_shr(level).unwrap_or(0);
    let right = rect
        .x
        .checked_add(rect.width)
        .ok_or_else(|| anyhow::anyhow!("atlas rect width overflow"))?
        .checked_shr(level)
        .unwrap_or(0);
    let bottom = rect
        .y
        .checked_add(rect.height)
        .ok_or_else(|| anyhow::anyhow!("atlas rect height overflow"))?
        .checked_shr(level)
        .unwrap_or(0);
    Ok(AtlasRect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    })
}

fn vanilla_mip_level(sources: &[SpriteSource], max_mipmap_levels: u32) -> u32 {
    let Some(first) = sources.first() else {
        return 0;
    };
    let mut min_texel_size = first.width.min(first.height);
    let mut lowest_bit = 1u32.checked_shl(max_mipmap_levels).unwrap_or(u32::MAX);
    for source in sources {
        min_texel_size = min_texel_size.min(source.width.min(source.height));
        let lowest_texture_bit = lowest_one_bit(source.width).min(lowest_one_bit(source.height));
        lowest_bit = lowest_bit.min(lowest_texture_bit);
    }

    let min_size = min_texel_size.min(lowest_bit).max(1);
    floor_log2(min_size).min(max_mipmap_levels)
}

fn lowest_one_bit(value: u32) -> u32 {
    value.isolate_lowest_one()
}

fn floor_log2(value: u32) -> u32 {
    u32::BITS - 1 - value.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::{AtlasLayout, AtlasMipImage, AtlasMipLevel, AtlasMipPatch, AtlasPacker, AtlasRect};
    use crate::{
        SpriteAnimation, SpriteAnimationFrame, SpriteGuiMetadata, SpriteGuiScaling, SpriteImage,
        SpriteMipmapStrategy, SpriteNineSliceBorder, SpriteSource, SpriteTextureMetadata,
        SpriteTransparency,
    };

    #[test]
    fn atlas_rects_preserve_content_dimensions_inside_padding() {
        let layout = AtlasPacker::new(128, 2)
            .unwrap()
            .pack(&[
                SpriteSource::new("minecraft:block/stone", 16, 16),
                SpriteSource::new("pack:block/hd_overlay", 64, 32),
            ])
            .unwrap();

        assert_eq!(layout.width, 88);
        assert_eq!(layout.height, 36);
        assert_eq!(layout.padding, 2);

        let stone = &layout.sprites[0];
        assert_eq!(stone.id, "minecraft:block/stone");
        assert_eq!(stone.source_width, 16);
        assert_eq!(stone.source_height, 16);
        assert_eq!(
            stone.padded,
            AtlasRect {
                x: 68,
                y: 0,
                width: 20,
                height: 20
            }
        );
        assert_eq!(
            stone.content,
            AtlasRect {
                x: 70,
                y: 2,
                width: 16,
                height: 16
            }
        );

        let overlay = &layout.sprites[1];
        assert_eq!(overlay.id, "pack:block/hd_overlay");
        assert_eq!(
            overlay.padded,
            AtlasRect {
                x: 0,
                y: 0,
                width: 68,
                height: 36
            }
        );
        assert_eq!(
            overlay.content,
            AtlasRect {
                x: 2,
                y: 2,
                width: 64,
                height: 32
            }
        );
    }

    #[test]
    fn atlas_packer_sorts_shelf_placement_by_descending_height_while_preserving_sprite_order() {
        let layout = AtlasPacker::new(36, 1)
            .unwrap()
            .pack(&[
                SpriteSource::new("test:short_a", 16, 16),
                SpriteSource::new("test:tall_a", 16, 64),
                SpriteSource::new("test:short_b", 16, 16),
                SpriteSource::new("test:tall_b", 16, 64),
            ])
            .unwrap();

        // Both 64-tall sprites share shelf 0 (height 66) and both 16-tall sprites
        // share shelf 1 (height 18), saving 48px vs unsorted placement (132px).
        assert_eq!(layout.width, 36);
        assert_eq!(layout.height, 84);
        assert_eq!(
            layout
                .sprites
                .iter()
                .map(|sprite| sprite.id.as_str())
                .collect::<Vec<_>>(),
            vec!["test:short_a", "test:tall_a", "test:short_b", "test:tall_b"]
        );
        assert_eq!(layout.sprites[1].padded.y, 0);
        assert_eq!(layout.sprites[3].padded.y, 0);
        assert_eq!(layout.sprites[0].padded.y, 66);
        assert_eq!(layout.sprites[2].padded.y, 66);
    }

    #[test]
    fn atlas_packer_wraps_rows_for_mixed_resolution_sprites() {
        let layout = AtlasPacker::new(300, 1)
            .unwrap()
            .pack(&[
                SpriteSource::new("pack:block/large", 256, 256),
                SpriteSource::new("pack:block/medium", 64, 64),
                SpriteSource::new("minecraft:block/small", 16, 16),
            ])
            .unwrap();

        assert_eq!(layout.width, 258);
        assert_eq!(layout.height, 324);
        assert_eq!(
            layout.sprites[0].content,
            AtlasRect {
                x: 1,
                y: 1,
                width: 256,
                height: 256
            }
        );
        assert_eq!(
            layout.sprites[1].content,
            AtlasRect {
                x: 1,
                y: 259,
                width: 64,
                height: 64
            }
        );
        assert_eq!(
            layout.sprites[2].content,
            AtlasRect {
                x: 67,
                y: 259,
                width: 16,
                height: 16
            }
        );
    }

    #[test]
    fn atlas_packer_rejects_invalid_sprite_dimensions() {
        let zero = AtlasPacker::new(128, 1)
            .unwrap()
            .pack(&[SpriteSource::new("bad", 0, 16)]);
        assert!(zero.is_err());

        let too_wide = AtlasPacker::new(16, 1)
            .unwrap()
            .pack(&[SpriteSource::new("wide", 16, 16)]);
        assert!(too_wide.is_err());
    }

    #[test]
    fn atlas_layout_preserves_sprite_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 4,
            interpolate: true,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 4 },
                SpriteAnimationFrame { index: 1, time: 8 },
            ],
        };
        let texture_metadata = SpriteTextureMetadata {
            blur: false,
            clamp: false,
            mipmap_strategy: SpriteMipmapStrategy::StrictCutout,
            alpha_cutoff_bias: 0.125,
        };
        let gui_metadata = SpriteGuiMetadata {
            scaling: SpriteGuiScaling::NineSlice {
                width: 16,
                height: 16,
                border: SpriteNineSliceBorder::uniform(2),
                stretch_inner: false,
            },
        };
        let source = SpriteSource {
            id: "minecraft:block/water_still".to_string(),
            width: 16,
            height: 16,
            animation: Some(animation.clone()),
            texture_metadata,
            gui_metadata,
        };

        let layout = AtlasPacker::new(64, 1).unwrap().pack(&[source]).unwrap();

        assert_eq!(layout.sprites[0].animation, Some(animation));
        assert_eq!(layout.sprites[0].texture_metadata, texture_metadata);
        assert_eq!(layout.sprites[0].gui_metadata, gui_metadata);
    }

    #[test]
    fn atlas_stitcher_extends_sprite_edges_into_padding() {
        let image = SpriteImage::new(
            "test:quad",
            2,
            2,
            vec![10, 0, 0, 255, 20, 0, 0, 0, 30, 0, 0, 255, 40, 0, 0, 255],
        )
        .unwrap();
        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.width, 4);
        assert_eq!(atlas.layout.height, 4);
        assert!(atlas.layout.sprites[0].transparency.has_transparent);
        assert!(!atlas.layout.sprites[0].transparency.has_translucent);
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 0),
            [10, 0, 0, 255]
        );
        assert_eq!(pixel(&atlas.rgba, atlas.layout.width, 3, 0), [20, 0, 0, 0]);
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 0, 3),
            [30, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 3, 3),
            [40, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&atlas.rgba, atlas.layout.width, 2, 2),
            [40, 0, 0, 255]
        );
    }

    #[test]
    fn atlas_stitcher_preserves_image_metadata() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 2,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 2 },
            ],
        };
        let texture_metadata = SpriteTextureMetadata {
            blur: true,
            clamp: true,
            mipmap_strategy: SpriteMipmapStrategy::Mean,
            alpha_cutoff_bias: 0.0,
        };
        let gui_metadata = SpriteGuiMetadata {
            scaling: SpriteGuiScaling::Tile {
                width: 4,
                height: 4,
            },
        };
        let image = SpriteImage {
            id: "minecraft:block/campfire_fire".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(animation.clone()),
            texture_metadata,
            gui_metadata,
            animation_frames_rgba: vec![vec![255, 255, 255, 255]; 2],
            rgba: vec![255, 255, 255, 255],
        };

        let atlas = AtlasPacker::new(8, 1).unwrap().stitch(&[image]).unwrap();

        assert_eq!(atlas.layout.sprites[0].animation, Some(animation));
        assert_eq!(atlas.layout.sprites[0].texture_metadata, texture_metadata);
        assert_eq!(atlas.layout.sprites[0].gui_metadata, gui_metadata);
    }

    #[test]
    fn atlas_stitcher_can_render_animation_frame_for_tick() {
        let animation = SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 1 },
            ],
        };
        let image = SpriteImage {
            id: "minecraft:block/water_still".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(animation),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![vec![10, 0, 0, 255], vec![20, 0, 0, 255]],
            rgba: vec![10, 0, 0, 255],
        };

        let initial = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch(std::slice::from_ref(&image))
            .unwrap();
        let tick_two = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(std::slice::from_ref(&image), 2)
            .unwrap();
        let tick_three = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(&[image], 3)
            .unwrap();

        assert_eq!(
            pixel(&initial.rgba, initial.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            pixel(&tick_two.rgba, tick_two.layout.width, 1, 1),
            [20, 0, 0, 255]
        );
        assert_eq!(
            pixel(&tick_three.rgba, tick_three.layout.width, 1, 1),
            [10, 0, 0, 255]
        );
    }

    #[test]
    fn atlas_stitcher_interpolates_animation_frame_for_tick() {
        let image = SpriteImage {
            id: "minecraft:block/sculk".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 4 },
                    SpriteAnimationFrame { index: 1, time: 4 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![vec![0, 0, 100, 255], vec![100, 40, 0, 127]],
            rgba: vec![0, 0, 100, 255],
        };

        let tick_two = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame(&[image], 2)
            .unwrap();

        assert_eq!(
            pixel(&tick_two.rgba, tick_two.layout.width, 1, 1),
            [50, 20, 50, 191]
        );
    }

    #[test]
    fn animated_mip_patches_match_full_atlas_rebuild_bytes() {
        let static_image =
            SpriteImage::new("minecraft:block/stone", 2, 2, [90, 90, 90, 255].repeat(4)).unwrap();
        let animated_image = SpriteImage {
            id: "minecraft:block/water_still".to_string(),
            width: 2,
            height: 2,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: false,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 2 },
                    SpriteAnimationFrame { index: 1, time: 1 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![
                [10, 20, 30, 255].repeat(4),
                [110, 120, 130, 255].repeat(4),
            ],
            rgba: [10, 20, 30, 255].repeat(4),
        };
        let images = vec![static_image, animated_image];
        let packer = AtlasPacker::new(32, 1).unwrap();
        let mut patched = packer.stitch_animation_frame_mips(&images, 0, 1).unwrap();

        assert!(patched
            .layout
            .animation_frame_mip_patches(&images, 0, 1, 1)
            .unwrap()
            .is_empty());
        let patches = patched
            .layout
            .animation_frame_mip_patches(&images, 0, 2, 1)
            .unwrap();
        assert_eq!(patches.len(), 2);
        for patch in &patches {
            apply_mip_patch(&mut patched.levels[patch.level as usize], patch);
        }
        let rebuilt = packer.stitch_animation_frame_mips(&images, 2, 1).unwrap();
        assert_eq!(patched.levels, rebuilt.levels);
    }

    #[test]
    fn interpolated_animation_emits_patch_without_frame_index_change() {
        let image = SpriteImage {
            id: "minecraft:block/sculk".to_string(),
            width: 1,
            height: 1,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 4 },
                    SpriteAnimationFrame { index: 1, time: 4 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![vec![0, 0, 100, 255], vec![100, 40, 0, 127]],
            rgba: vec![0, 0, 100, 255],
        };
        let images = [image];
        let atlas = AtlasPacker::new(8, 1)
            .unwrap()
            .stitch_animation_frame_mips(&images, 0, 0)
            .unwrap();

        let patches = atlas
            .layout
            .animation_frame_mip_patches(&images, 0, 1, 0)
            .unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(
            pixel(&patches[0].rgba, patches[0].rect.width, 1, 1),
            [25, 10, 75, 223]
        );
    }

    #[test]
    fn prepared_animation_cache_interpolates_precomputed_mips_like_vanilla() {
        let frame_zero = vec![0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
        let frame_one = vec![
            255, 255, 255, 255, 0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 0, 255,
        ];
        let image = SpriteImage {
            id: "minecraft:block/interpolated_mips".to_string(),
            width: 2,
            height: 2,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 2,
                default_frame_time: 1,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 3 },
                    SpriteAnimationFrame { index: 1, time: 3 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata {
                mipmap_strategy: SpriteMipmapStrategy::Mean,
                ..SpriteTextureMetadata::default()
            },
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![frame_zero.clone(), frame_one.clone()],
            rgba: frame_zero.clone(),
        };
        let images = [image.clone()];
        let atlas = AtlasPacker::new(16, 1)
            .unwrap()
            .stitch_animation_frame_mips(&images, 0, 1)
            .unwrap();
        let cache = atlas
            .layout
            .prepare_animation_mip_cache(&images, 1)
            .unwrap();
        let patches = cache.frame_mip_patches(&atlas.layout, 0, 1).unwrap();
        let mip_one_patch = patches.iter().find(|patch| patch.level == 1).unwrap();

        let current_mips = super::generate_sprite_rgba_mip_levels(
            &image.id,
            image.width,
            image.height,
            &frame_zero,
            image.texture_metadata,
            image.transparency,
            1,
        )
        .unwrap();
        let next_mips = super::generate_sprite_rgba_mip_levels(
            &image.id,
            image.width,
            image.height,
            &frame_one,
            image.texture_metadata,
            image.transparency,
            1,
        )
        .unwrap();
        let progress = super::vanilla_animation_progress(1.0 / 3.0);
        assert_eq!(progress, 0.333);
        let expected = std::array::from_fn(|channel| {
            let current = f32::from(current_mips[1].rgba[channel]);
            let next = f32::from(next_mips[1].rgba[channel]);
            (current + (next - current) * progress).round() as u8
        });
        let sprite = &atlas.layout.sprites[0];
        let local_x = (sprite.content.x >> 1) - mip_one_patch.rect.x;
        let local_y = (sprite.content.y >> 1) - mip_one_patch.rect.y;
        assert_eq!(
            pixel(
                &mip_one_patch.rgba,
                mip_one_patch.rect.width,
                local_x,
                local_y
            ),
            expected
        );
    }

    #[test]
    fn gpu_animation_plan_preserves_unique_frame_mips_and_atlas_destination() {
        let image = SpriteImage {
            id: "minecraft:block/water_still".to_string(),
            width: 2,
            height: 2,
            transparency: SpriteTransparency::default(),
            animation: Some(SpriteAnimation {
                frame_count: 3,
                default_frame_time: 1,
                interpolate: true,
                frames: vec![
                    SpriteAnimationFrame { index: 0, time: 2 },
                    SpriteAnimationFrame { index: 2, time: 3 },
                    SpriteAnimationFrame { index: 0, time: 1 },
                ],
            }),
            texture_metadata: SpriteTextureMetadata::default(),
            gui_metadata: SpriteGuiMetadata::default(),
            animation_frames_rgba: vec![
                [10, 20, 30, 255].repeat(4),
                [40, 50, 60, 255].repeat(4),
                [70, 80, 90, 255].repeat(4),
            ],
            rgba: [10, 20, 30, 255].repeat(4),
        };
        let images = [image];
        let atlas = AtlasPacker::new(32, 1)
            .unwrap()
            .stitch_animation_frame_mips(&images, 0, 1)
            .unwrap();
        let plan = atlas
            .layout
            .prepare_gpu_animation_plan(&images, 1)
            .unwrap()
            .unwrap();

        assert_eq!(plan.atlas_width, atlas.layout.width);
        assert_eq!(plan.atlas_height, atlas.layout.height);
        assert_eq!(plan.padding, atlas.layout.padding);
        assert_eq!(plan.mip_level, 1);
        assert_eq!(plan.sprites.len(), 1);
        let sprite = &plan.sprites[0];
        assert_eq!(sprite.destination, atlas.layout.sprites[0].padded);
        assert_eq!((sprite.content_width, sprite.content_height), (2, 2));
        assert!(sprite.animation.interpolate);
        assert_eq!(
            sprite
                .frames
                .iter()
                .map(|frame| frame.index)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert!(sprite.frames.iter().all(|frame| frame.mips.len() == 2));
    }

    #[test]
    fn coalesced_mip_patches_preserve_exact_sequential_result() {
        let initial_rgba = (0_u8..=255).cycle().take(16 * 4 * 4).collect::<Vec<_>>();
        let mut atlas = AtlasMipImage {
            layout: AtlasLayout {
                width: 16,
                height: 4,
                padding: 0,
                sprites: Vec::new(),
            },
            levels: vec![AtlasMipLevel {
                level: 0,
                width: 16,
                height: 4,
                rgba: initial_rgba.clone(),
            }],
        };
        let patches = vec![
            AtlasMipPatch {
                level: 0,
                rect: AtlasRect {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                rgba: [10, 20, 30, 255].repeat(4),
            },
            AtlasMipPatch {
                level: 0,
                rect: AtlasRect {
                    x: 4,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                rgba: [110, 120, 130, 255].repeat(4),
            },
            AtlasMipPatch {
                level: 0,
                rect: AtlasRect {
                    x: 8,
                    y: 3,
                    width: 1,
                    height: 1,
                },
                rgba: vec![210, 220, 230, 255],
            },
        ];

        let coalesced = atlas.apply_and_coalesce_patches(&patches, 2).unwrap();
        assert_eq!(coalesced.len(), 2);
        assert_eq!(
            coalesced[0].rect,
            AtlasRect {
                x: 0,
                y: 0,
                width: 6,
                height: 2,
            }
        );

        let mut sequential = AtlasMipLevel {
            level: 0,
            width: 16,
            height: 4,
            rgba: initial_rgba.clone(),
        };
        for patch in &patches {
            apply_mip_patch(&mut sequential, patch);
        }
        let mut batched = AtlasMipLevel {
            level: 0,
            width: 16,
            height: 4,
            rgba: initial_rgba,
        };
        for patch in &coalesced {
            apply_mip_patch(&mut batched, patch);
        }
        assert_eq!(batched, sequential);
        assert_eq!(atlas.levels[0], sequential);
    }

    #[test]
    fn coalesced_mip_patches_respect_overdraw_bound() {
        let mut atlas = AtlasMipImage {
            layout: AtlasLayout {
                width: 16,
                height: 2,
                padding: 0,
                sprites: Vec::new(),
            },
            levels: vec![AtlasMipLevel {
                level: 0,
                width: 16,
                height: 2,
                rgba: vec![0; 16 * 2 * 4],
            }],
        };
        let patches = [
            AtlasMipPatch {
                level: 0,
                rect: AtlasRect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                rgba: vec![255; 4],
            },
            AtlasMipPatch {
                level: 0,
                rect: AtlasRect {
                    x: 3,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                rgba: vec![255; 4],
            },
        ];

        assert_eq!(
            atlas.apply_and_coalesce_patches(&patches, 1).unwrap().len(),
            2
        );
    }

    #[test]
    fn atlas_mips_are_generated_per_sprite_without_cross_sprite_blending() {
        let red = SpriteImage::new(
            "minecraft:block/red",
            2,
            2,
            vec![
                200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255,
            ],
        )
        .unwrap();
        let blue = SpriteImage::new(
            "minecraft:block/blue",
            2,
            2,
            vec![
                0, 0, 200, 255, 0, 0, 200, 255, 0, 0, 200, 255, 0, 0, 200, 255,
            ],
        )
        .unwrap();

        let atlas = AtlasPacker::new(16, 1)
            .unwrap()
            .stitch_mips(&[red, blue], 1)
            .unwrap();

        assert_eq!(atlas.levels.len(), 2);
        assert_eq!(atlas.layout.padding, 2);
        assert_eq!((atlas.layout.width, atlas.layout.height), (16, 8));
        assert_eq!((atlas.levels[1].width, atlas.levels[1].height), (8, 4));
        let red = &atlas.layout.sprites[0];
        let blue = &atlas.layout.sprites[1];
        assert_eq!(
            pixel(
                &atlas.levels[1].rgba,
                atlas.levels[1].width,
                red.content.x >> 1,
                red.content.y >> 1,
            ),
            [200, 0, 0, 255]
        );
        assert_eq!(
            pixel(
                &atlas.levels[1].rgba,
                atlas.levels[1].width,
                blue.content.x >> 1,
                blue.content.y >> 1,
            ),
            [0, 0, 200, 255]
        );
    }

    #[test]
    fn atlas_mip_layout_keeps_base_uvs_inside_the_same_sprite_at_level_four() {
        let red = SpriteImage::new(
            "minecraft:block/red",
            16,
            16,
            [200, 0, 0, 255].repeat(16 * 16),
        )
        .unwrap();
        let blue = SpriteImage::new(
            "minecraft:block/blue",
            16,
            16,
            [0, 0, 200, 255].repeat(16 * 16),
        )
        .unwrap();

        let atlas = AtlasPacker::new(128, 1)
            .unwrap()
            .stitch_mips(&[red, blue], 4)
            .unwrap();

        assert_eq!(atlas.layout.padding, 16);
        assert!(atlas.layout.width.is_power_of_two());
        assert!(atlas.layout.height.is_power_of_two());
        let mip = &atlas.levels[4];
        for (sprite, expected) in atlas
            .layout
            .sprites
            .iter()
            .zip([[200, 0, 0, 255], [0, 0, 200, 255]])
        {
            assert_eq!(sprite.content.x % 16, 0);
            assert_eq!(sprite.content.y % 16, 0);
            let base_center_u = (sprite.content.x as f32 + 8.0) / atlas.layout.width as f32;
            let base_center_v = (sprite.content.y as f32 + 8.0) / atlas.layout.height as f32;
            let mip_x = (base_center_u * mip.width as f32).floor() as u32;
            let mip_y = (base_center_v * mip.height as f32).floor() as u32;
            assert_eq!(mip_x, sprite.content.x >> 4);
            assert_eq!(mip_y, sprite.content.y >> 4);
            assert_eq!(pixel(&mip.rgba, mip.width, mip_x, mip_y), expected);
        }
    }

    #[test]
    fn atlas_mips_with_max_level_follow_vanilla_lowest_one_bit_limit() {
        let image = SpriteImage::new(
            "minecraft:block/non_power_of_two",
            12,
            12,
            vec![80; 12 * 12 * 4],
        )
        .unwrap();

        let atlas = AtlasPacker::new(32, 1)
            .unwrap()
            .stitch_mips_with_max_level(&[image], 4)
            .unwrap();

        assert_eq!(atlas.mip_level(), 2);
        assert_eq!(atlas.layout.padding, 4);
        assert_eq!(
            atlas
                .levels
                .iter()
                .map(|level| (level.width, level.height))
                .collect::<Vec<_>>(),
            vec![(32, 32), (16, 16), (8, 8)]
        );
    }

    #[test]
    fn release_rgba_drops_cpu_mip_buffers_while_preserving_level_metadata() {
        let image =
            SpriteImage::new("minecraft:block/stone", 16, 16, vec![255; 16 * 16 * 4]).unwrap();
        let mut atlas = AtlasPacker::new(64, 1)
            .unwrap()
            .stitch_mips_with_max_level(&[image], 4)
            .unwrap();
        assert!(atlas.has_cpu_rgba());
        assert_eq!(atlas.mip_level(), 4);

        atlas.release_rgba();
        assert!(!atlas.has_cpu_rgba());
        assert_eq!(atlas.mip_level(), 4);
        assert!(atlas.levels.iter().all(|level| level.rgba.is_empty()));
        assert!(atlas
            .apply_and_coalesce_patches(
                &[AtlasMipPatch {
                    level: 0,
                    rect: AtlasRect {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    },
                    rgba: vec![255; 4],
                }],
                1,
            )
            .is_err());
    }

    #[test]
    fn atlas_packer_enforces_max_height_and_reserves_stitched_bytes_before_allocating() {
        // Single sprite taller than max_height fails.
        let err = AtlasPacker::new_bounded(64, 32, 1)
            .unwrap()
            .pack(&[SpriteSource::new("tall", 16, 32)])
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("exceeds atlas max height"),
            "{err:#}"
        );

        // Multiple wrapped rows exceeding max_height fail.
        let err = AtlasPacker::new_bounded(32, 32, 1)
            .unwrap()
            .pack(&[
                SpriteSource::new("a", 16, 16),
                SpriteSource::new("b", 16, 16),
            ])
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("exceeds atlas max height"),
            "{err:#}"
        );

        // Tight generation image budget rejects stitched RGBA/mips before allocating.
        let root = std::env::temp_dir().join(format!(
            "bbb-atlas-budget-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("sources/26.1/assets/minecraft")).unwrap();
        let base = crate::PackRoots::from_root(&root).unwrap();
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&base, []).unwrap();
        let tight = base
            .with_client_resource_generation_image_limit(1, selected, 64)
            .unwrap();
        let stack = tight.resource_stack();
        let sprite = SpriteImage::new("a", 4, 4, vec![255u8; 4 * 4 * 4]).unwrap();
        // Padded 6x6 = 144 RGBA bytes > 64 byte limit.
        let err = AtlasPacker::new(16, 1)
            .unwrap()
            .stitch_with_stack(std::slice::from_ref(&sprite), &stack)
            .unwrap_err();
        assert!(format!("{err:#}").contains("stitched atlas"), "{err:#}");
        let err = AtlasPacker::new(16, 1)
            .unwrap()
            .stitch_mips_with_stack(&[sprite], 1, &stack)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("stitched atlas mips"),
            "{err:#}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn animation_mip_cache_rejects_excessive_atlas_frame_totals() {
        let mut images = Vec::new();
        // 5 sprites * 4000 frames = 20,000 unique frames (> MAX_ATLAS_ANIMATION_FRAMES = 16,384).
        for idx in 0..5 {
            let mut image =
                SpriteImage::new(format!("anim_{idx}"), 1, 1, vec![255, 0, 0, 255]).unwrap();
            let frames = (0..4000)
                .map(|frame_idx| bbb_render_types::SpriteAnimationFrame {
                    index: frame_idx,
                    time: 1,
                })
                .collect();
            image.animation = Some(bbb_render_types::SpriteAnimation {
                frame_count: 4000,
                default_frame_time: 1,
                interpolate: false,
                frames,
            });
            image.animation_frames_rgba = vec![vec![255, 0, 0, 255]; 4000];
            images.push(image);
        }
        let sources: Vec<SpriteSource> = images.iter().map(SpriteImage::source).collect();
        let layout = AtlasPacker::new(64, 1).unwrap().pack(&sources).unwrap();
        let err = layout.prepare_animation_mip_cache(&images, 0).unwrap_err();
        assert!(format!("{err:#}").contains("total frame limit"), "{err:#}");
    }

    fn apply_mip_patch(level: &mut AtlasMipLevel, patch: &AtlasMipPatch) {
        for row in 0..patch.rect.height {
            let source_start = (row * patch.rect.width * 4) as usize;
            let source_end = source_start + (patch.rect.width * 4) as usize;
            let target_start = (((patch.rect.y + row) * level.width + patch.rect.x) * 4) as usize;
            let target_end = target_start + (patch.rect.width * 4) as usize;
            level.rgba[target_start..target_end]
                .copy_from_slice(&patch.rgba[source_start..source_end]);
        }
    }

    fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        rgba[offset..offset + 4].try_into().unwrap()
    }
}
