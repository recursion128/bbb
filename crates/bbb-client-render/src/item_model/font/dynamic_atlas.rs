//! Lazy on-demand dynamic font atlas backing Unihex and TrueType providers.
//!
//! Bitmap pages and the missing glyph live in the small static region at the top
//! of the atlas (`y < static_height`); Unihex and TrueType glyphs are shelf-packed
//! and rasterized individually on first draw so startup avoids allocating a
//! monolithic ~148 MiB RGBA8 image.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use bbb_render_types::{
    HudDynamicAtlasSnapshot, HudDynamicAtlasUpdate, HudDynamicFontAtlas, HudDynamicGlyphUpload,
    HudUvRect,
};

use super::{ttf::TtfGlyphSource, unihex::UnihexGlyphSource};

pub(crate) const DEFAULT_MAX_DYNAMIC_ATLAS_DIMENSION: u32 = 4_096;
pub(crate) const MAX_DYNAMIC_FONT_ATLAS_BYTES: u64 = 32 * 1024 * 1024;
const DYNAMIC_GLYPH_GAP: u32 = 1;

#[derive(Debug, Clone)]
pub(super) enum LazyProviderSource {
    Unihex(Arc<UnihexGlyphSource>),
    TrueType(Arc<TtfGlyphSource>),
}

impl LazyProviderSource {
    fn rasterize_rgba(&self, glyph_index: usize) -> Vec<u8> {
        match self {
            Self::Unihex(source) => source.rasterize_glyph_rgba(glyph_index),
            Self::TrueType(source) => source.rasterize_glyph_rgba(glyph_index),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LazyGlyphDescriptor {
    pub source_index: u32,
    pub glyph_index: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
struct DynamicGlyphPlacement {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    upload_rev: u64,
}

#[derive(Debug)]
struct DynamicAtlasState {
    width: u32,
    height: u32,
    band_min_x: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    layout_generation: u64,
    upload_revision: u64,
    placements: Vec<Option<DynamicGlyphPlacement>>,
    realized_order: Vec<u32>,
}

#[derive(Debug)]
pub(super) struct DynamicFontAtlas {
    static_width: u32,
    static_height: u32,
    static_rgba: Arc<[u8]>,
    initial_width: u32,
    initial_height: u32,
    max_dimension: u32,
    missing_uv: HudUvRect,
    sources: Vec<LazyProviderSource>,
    entries: Vec<LazyGlyphDescriptor>,
    current_dims: AtomicU64,
    layout_generation: AtomicU64,
    upload_revision: AtomicU64,
    state: Mutex<DynamicAtlasState>,
}

#[inline]
fn pack_dims(width: u32, height: u32) -> u64 {
    (u64::from(width) << 32) | u64::from(height)
}

#[inline]
fn unpack_dims(packed: u64) -> (u32, u32) {
    ((packed >> 32) as u32, packed as u32)
}

#[inline]
fn uv_for_rect(x: u32, y: u32, width: u32, height: u32, atlas_w: u32, atlas_h: u32) -> HudUvRect {
    let aw = atlas_w as f32;
    let ah = atlas_h as f32;
    HudUvRect {
        min: [(x as f32 + 0.01) / aw, (y as f32 + 0.01) / ah],
        max: [
            (x as f32 + width as f32 - 0.01) / aw,
            (y as f32 + height as f32 - 0.01) / ah,
        ],
    }
}

impl DynamicFontAtlas {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        static_width: u32,
        static_height: u32,
        static_rgba: Arc<[u8]>,
        initial_width: u32,
        initial_height: u32,
        max_dimension: u32,
        missing_uv: HudUvRect,
        sources: Vec<LazyProviderSource>,
        entries: Vec<LazyGlyphDescriptor>,
    ) -> Self {
        let count = entries.len();
        let max_dimension = max_dimension.clamp(
            initial_width.max(initial_height).max(1),
            DEFAULT_MAX_DYNAMIC_ATLAS_DIMENSION,
        );
        Self {
            static_width,
            static_height,
            static_rgba,
            initial_width,
            initial_height,
            max_dimension,
            missing_uv,
            sources,
            entries,
            current_dims: AtomicU64::new(pack_dims(initial_width, initial_height)),
            layout_generation: AtomicU64::new(1),
            upload_revision: AtomicU64::new(0),
            state: Mutex::new(DynamicAtlasState {
                width: initial_width,
                height: initial_height,
                band_min_x: 0,
                cursor_x: 0,
                cursor_y: static_height,
                row_height: 0,
                layout_generation: 1,
                upload_revision: 0,
                placements: vec![None; count],
                realized_order: Vec::new(),
            }),
        }
    }

    fn rasterize_entry(&self, lazy_id: u32) -> Vec<u8> {
        let desc = self.entries[lazy_id as usize];
        self.sources[desc.source_index as usize].rasterize_rgba(desc.glyph_index as usize)
    }

    fn within_byte_budget(width: u32, height: u32) -> bool {
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .is_some_and(|bytes| bytes <= MAX_DYNAMIC_FONT_ATLAS_BYTES)
    }

    fn allocate_slot(
        &self,
        state: &mut DynamicAtlasState,
        glyph_w: u32,
        glyph_h: u32,
    ) -> Option<(u32, u32)> {
        if glyph_w == 0
            || glyph_h == 0
            || glyph_w > self.max_dimension
            || glyph_h > self.max_dimension
        {
            return None;
        }
        loop {
            if state.cursor_x > state.band_min_x
                && state.cursor_x.saturating_add(glyph_w) > state.width
            {
                state.cursor_x = state.band_min_x;
                state.cursor_y = state
                    .cursor_y
                    .saturating_add(state.row_height.saturating_add(DYNAMIC_GLYPH_GAP));
                state.row_height = 0;
            }
            if state.cursor_x.saturating_add(glyph_w) <= state.width
                && state.cursor_y.saturating_add(glyph_h) <= state.height
            {
                let x = state.cursor_x;
                let y = state.cursor_y;
                state.cursor_x = state
                    .cursor_x
                    .saturating_add(glyph_w.saturating_add(DYNAMIC_GLYPH_GAP));
                state.row_height = state.row_height.max(glyph_h);
                return Some((x, y));
            }
            // Grow vertically first while preserving every existing (x, y) placement.
            if state.height < self.max_dimension {
                let next_height = (state.height.saturating_mul(2))
                    .max(state.cursor_y.saturating_add(glyph_h))
                    .next_power_of_two()
                    .min(self.max_dimension);
                if next_height > state.height && Self::within_byte_budget(state.width, next_height)
                {
                    state.height = next_height;
                    state.layout_generation = state.layout_generation.wrapping_add(1);
                    self.current_dims
                        .store(pack_dims(state.width, state.height), Ordering::Release);
                    self.layout_generation
                        .store(state.layout_generation, Ordering::Release);
                    continue;
                }
            }
            // Once height reaches the texture or byte limit, expand horizontally into a new band.
            if state.width < self.max_dimension {
                let old_width = state.width;
                let next_width = (state.width.saturating_mul(2))
                    .max(old_width.saturating_add(glyph_w))
                    .next_power_of_two()
                    .min(self.max_dimension);
                if next_width > state.width && Self::within_byte_budget(next_width, state.height) {
                    state.width = next_width;
                    state.band_min_x = old_width;
                    state.cursor_x = old_width;
                    state.cursor_y = 0;
                    state.row_height = 0;
                    state.layout_generation = state.layout_generation.wrapping_add(1);
                    self.current_dims
                        .store(pack_dims(state.width, state.height), Ordering::Release);
                    self.layout_generation
                        .store(state.layout_generation, Ordering::Release);
                    continue;
                }
            }
            return None;
        }
    }
}

impl HudDynamicFontAtlas for DynamicFontAtlas {
    fn atlas_dimensions(&self) -> (u32, u32) {
        unpack_dims(self.current_dims.load(Ordering::Acquire))
    }

    fn static_uv_scale(&self) -> [f32; 2] {
        let (width, height) = self.atlas_dimensions();
        if width == self.initial_width && height == self.initial_height {
            return [1.0, 1.0];
        }
        [
            self.initial_width as f32 / width as f32,
            self.initial_height as f32 / height as f32,
        ]
    }

    fn layout_generation(&self) -> u64 {
        self.layout_generation.load(Ordering::Acquire)
    }

    fn upload_revision(&self) -> u64 {
        self.upload_revision.load(Ordering::Acquire)
    }

    fn ensure_glyph_uv(&self, lazy_id: u32) -> HudUvRect {
        let index = lazy_id as usize;
        let Some(&desc) = self.entries.get(index) else {
            return self.missing_uv;
        };
        let mut state = self.state.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(placement) = state.placements[index] {
            return uv_for_rect(
                placement.x,
                placement.y,
                placement.width,
                placement.height,
                state.width,
                state.height,
            );
        }
        let Some((x, y)) = self.allocate_slot(&mut state, desc.width, desc.height) else {
            let scale = [
                self.initial_width as f32 / state.width as f32,
                self.initial_height as f32 / state.height as f32,
            ];
            return HudUvRect {
                min: [
                    self.missing_uv.min[0] * scale[0],
                    self.missing_uv.min[1] * scale[1],
                ],
                max: [
                    self.missing_uv.max[0] * scale[0],
                    self.missing_uv.max[1] * scale[1],
                ],
            };
        };
        state.upload_revision = state.upload_revision.wrapping_add(1);
        let upload_rev = state.upload_revision;
        state.placements[index] = Some(DynamicGlyphPlacement {
            x,
            y,
            width: desc.width,
            height: desc.height,
            upload_rev,
        });
        state.realized_order.push(lazy_id);
        self.upload_revision.store(upload_rev, Ordering::Release);
        uv_for_rect(x, y, desc.width, desc.height, state.width, state.height)
    }

    fn snapshot(&self) -> HudDynamicAtlasSnapshot {
        let (width, height, layout_generation, upload_revision, placed) = {
            let state = self.state.lock().unwrap_or_else(|err| err.into_inner());
            let placed = state
                .realized_order
                .iter()
                .filter_map(|&lazy_id| {
                    state.placements[lazy_id as usize].map(|placement| (lazy_id, placement))
                })
                .collect::<Vec<_>>();
            (
                state.width,
                state.height,
                state.layout_generation,
                state.upload_revision,
                placed,
            )
        };
        let glyphs = placed
            .into_iter()
            .map(|(lazy_id, placement)| HudDynamicGlyphUpload {
                x: placement.x,
                y: placement.y,
                width: placement.width,
                height: placement.height,
                rgba: self.rasterize_entry(lazy_id),
            })
            .collect();
        HudDynamicAtlasSnapshot {
            width,
            height,
            static_width: self.static_width,
            static_height: self.static_height,
            static_rgba: Arc::clone(&self.static_rgba),
            glyphs,
            layout_generation,
            upload_revision,
        }
    }

    fn poll_updates(
        &self,
        synced_layout_gen: u64,
        synced_upload_rev: u64,
    ) -> HudDynamicAtlasUpdate {
        let cur_layout = self.layout_generation.load(Ordering::Acquire);
        let cur_upload = self.upload_revision.load(Ordering::Acquire);
        if cur_layout == synced_layout_gen && cur_upload == synced_upload_rev {
            return HudDynamicAtlasUpdate::Unchanged;
        }
        if cur_layout != synced_layout_gen {
            return HudDynamicAtlasUpdate::Resized(self.snapshot());
        }
        let (latest_upload_rev, pending, resized) = {
            let state = self.state.lock().unwrap_or_else(|err| err.into_inner());
            if state.layout_generation != synced_layout_gen {
                (state.upload_revision, Vec::new(), true)
            } else {
                let start = state.realized_order.partition_point(|&lazy_id| {
                    state.placements[lazy_id as usize]
                        .is_some_and(|placement| placement.upload_rev <= synced_upload_rev)
                });
                let pending = state.realized_order[start..]
                    .iter()
                    .filter_map(|&lazy_id| {
                        state.placements[lazy_id as usize].map(|placement| (lazy_id, placement))
                    })
                    .collect::<Vec<_>>();
                (state.upload_revision, pending, false)
            }
        };
        if resized {
            return HudDynamicAtlasUpdate::Resized(self.snapshot());
        }
        if pending.is_empty() {
            return HudDynamicAtlasUpdate::Unchanged;
        }
        let uploads = pending
            .into_iter()
            .map(|(lazy_id, placement)| HudDynamicGlyphUpload {
                x: placement.x,
                y: placement.y,
                width: placement.width,
                height: placement.height,
                rgba: self.rasterize_entry(lazy_id),
            })
            .collect();
        HudDynamicAtlasUpdate::Uploads {
            upload_revision: latest_upload_rev,
            uploads,
        }
    }

    fn pixel_rgba(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.static_width && y < self.static_height {
            let offset = ((y * self.static_width + x) * 4) as usize;
            if let Some(pixel) = self.static_rgba.get(offset..offset + 4) {
                return [pixel[0], pixel[1], pixel[2], pixel[3]];
            }
            return [0; 4];
        }
        let hit = {
            let state = self.state.lock().unwrap_or_else(|err| err.into_inner());
            state.realized_order.iter().find_map(|&lazy_id| {
                let placement = state.placements[lazy_id as usize]?;
                if x >= placement.x
                    && x < placement.x + placement.width
                    && y >= placement.y
                    && y < placement.y + placement.height
                {
                    Some((lazy_id, placement))
                } else {
                    None
                }
            })
        };
        let Some((lazy_id, placement)) = hit else {
            return [0; 4];
        };
        let rgba = self.rasterize_entry(lazy_id);
        let lx = x - placement.x;
        let ly = y - placement.y;
        let offset = ((ly * placement.width + lx) * 4) as usize;
        rgba.get(offset..offset + 4)
            .map_or([0; 4], |px| [px[0], px[1], px[2], px[3]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_font_atlas_enforces_dimension_and_byte_budget() {
        let missing_uv = HudUvRect {
            min: [0.0, 0.0],
            max: [0.1, 0.1],
        };
        let entries = (0..600)
            .map(|i| LazyGlyphDescriptor {
                source_index: 0,
                glyph_index: i,
                width: 256,
                height: 256,
            })
            .collect();
        let atlas = DynamicFontAtlas::new(
            64,
            64,
            Arc::from(vec![0u8; 64 * 64 * 4]),
            512,
            512,
            2_048,
            missing_uv,
            Vec::new(),
            entries,
        );
        for id in 0..600 {
            let _ = atlas.ensure_glyph_uv(id);
        }
        let (width, height) = atlas.atlas_dimensions();
        assert!(width <= 2_048 && height <= 2_048);
        assert!(u64::from(width) * u64::from(height) * 4 <= MAX_DYNAMIC_FONT_ATLAS_BYTES);
    }
}
