use std::{collections::BTreeMap, sync::Arc};

use bbb_renderer::{ItemFrameMapTexture, ItemFrameMapTexturePatch};
use bbb_world::MapItemState;

pub(crate) const MAP_SIZE: usize = 128;

include!("map_textures/generated_26_1.rs");

const MAX_CACHED_MAP_TEXTURES: usize = 1_024;

#[derive(Debug)]
struct CachedMapTexture {
    owner_generation: u64,
    content_revision: u64,
    rgba: Arc<[u8]>,
    last_used_frame: u64,
}

#[derive(Debug, Default)]
pub(crate) struct MapTextureExtractionCache {
    entries: BTreeMap<i32, CachedMapTexture>,
    frame_requests: BTreeMap<i32, ItemFrameMapTexture>,
    frame_index: u64,
}

impl MapTextureExtractionCache {
    pub(crate) fn begin_frame(&mut self) {
        self.frame_index = self
            .frame_index
            .checked_add(1)
            .expect("map texture extraction frame overflow");
        self.frame_requests.clear();
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.frame_requests.clear();
    }

    pub(crate) fn texture(&mut self, map: &MapItemState) -> ItemFrameMapTexture {
        if let Some(request) = self.frame_requests.get(&map.id) {
            return request.clone();
        }
        let owner_generation = map.colors.owner_generation();
        let content_revision = map.colors.content_revision();
        if let Some(cached) = self.entries.get_mut(&map.id) {
            if cached.owner_generation == owner_generation
                && cached.content_revision == content_revision
            {
                cached.last_used_frame = self.frame_index;
                let request = ItemFrameMapTexture {
                    map_id: map.id,
                    owner_generation,
                    content_revision,
                    rgba: Arc::clone(&cached.rgba),
                    patch: None,
                };
                self.frame_requests.insert(map.id, request.clone());
                return request;
            }
        }

        let patch = self.entries.get_mut(&map.id).and_then(|cached| {
            let patch = map.last_color_patch?;
            if cached.owner_generation != owner_generation
                || cached.content_revision.checked_add(1) != Some(content_revision)
            {
                return None;
            }
            let patch_rgba = apply_rgba_patch(Arc::make_mut(&mut cached.rgba), map, patch)?;
            cached.content_revision = content_revision;
            cached.last_used_frame = self.frame_index;
            Some(ItemFrameMapTexturePatch {
                start_x: patch.start_x,
                start_y: patch.start_y,
                width: patch.width,
                height: patch.height,
                rgba: patch_rgba.into(),
            })
        });

        if patch.is_none() {
            let rgba: Arc<[u8]> = decode_map_rgba(map).into();
            self.entries.insert(
                map.id,
                CachedMapTexture {
                    owner_generation,
                    content_revision,
                    rgba,
                    last_used_frame: self.frame_index,
                },
            );
        }
        self.evict_unused();
        let cached = self
            .entries
            .get(&map.id)
            .expect("map texture inserted or patched");
        let request = ItemFrameMapTexture {
            map_id: map.id,
            owner_generation,
            content_revision,
            rgba: Arc::clone(&cached.rgba),
            patch,
        };
        self.frame_requests.insert(map.id, request.clone());
        request
    }

    fn evict_unused(&mut self) {
        while self.entries.len() > MAX_CACHED_MAP_TEXTURES {
            let Some(map_id) = self
                .entries
                .iter()
                .filter(|(_, entry)| entry.last_used_frame != self.frame_index)
                .min_by_key(|(map_id, entry)| (entry.last_used_frame, **map_id))
                .map(|(map_id, _)| *map_id)
            else {
                break;
            };
            self.entries.remove(&map_id);
        }
    }
}

fn decode_map_rgba(map: &MapItemState) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(MAP_SIZE * MAP_SIZE * 4);
    for index in 0..MAP_SIZE * MAP_SIZE {
        rgba.extend_from_slice(&map_color_rgba8(
            map.colors.get(index).copied().unwrap_or(0),
        ));
    }
    rgba
}

fn apply_rgba_patch(
    full_rgba: &mut [u8],
    map: &MapItemState,
    patch: bbb_world::MapColorPatchState,
) -> Option<Vec<u8>> {
    let width = usize::from(patch.width);
    let height = usize::from(patch.height);
    let start_x = usize::from(patch.start_x);
    let start_y = usize::from(patch.start_y);
    if full_rgba.len() != MAP_SIZE * MAP_SIZE * 4
        || start_x.checked_add(width)? > MAP_SIZE
        || start_y.checked_add(height)? > MAP_SIZE
    {
        return None;
    }
    let mut patch_rgba = Vec::with_capacity(width.checked_mul(height)?.checked_mul(4)?);
    for y in 0..height {
        for x in 0..width {
            let map_index = start_x + x + (start_y + y) * MAP_SIZE;
            let rgba = map_color_rgba8(map.colors.get(map_index).copied().unwrap_or(0));
            patch_rgba.extend_from_slice(&rgba);
            let rgba_index = map_index * 4;
            full_rgba[rgba_index..rgba_index + 4].copy_from_slice(&rgba);
        }
    }
    Some(patch_rgba)
}

pub(crate) fn map_color_rgba8(packed: u8) -> [u8; 4] {
    let material_id = usize::from(packed >> 2);
    let base = MAP_MATERIAL_COLORS.get(material_id).copied().unwrap_or(0);
    if base == 0 {
        return [0, 0, 0, 0];
    }
    let modifier = MAP_BRIGHTNESS_MODIFIERS[usize::from(packed & 3)];
    let r = ((base >> 16) & 0xFF) * modifier / 255;
    let g = ((base >> 8) & 0xFF) * modifier / 255;
    let b = (base & 0xFF) * modifier / 255;
    [r as u8, g as u8, b as u8, 255]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{MapColorPatch, MapItemData};
    use bbb_world::WorldStore;

    #[test]
    fn map_color_rgba_matches_generated_full_packed_domain() {
        for packed in u8::MIN..=u8::MAX {
            let base = MAP_MATERIAL_COLORS[usize::from(packed >> 2)];
            let modifier = MAP_BRIGHTNESS_MODIFIERS[usize::from(packed & 3)];
            let expected = if base == 0 {
                [0, 0, 0, 0]
            } else {
                [
                    ((((base >> 16) & 0xFF) * modifier) / 255) as u8,
                    ((((base >> 8) & 0xFF) * modifier) / 255) as u8,
                    (((base & 0xFF) * modifier) / 255) as u8,
                    255,
                ]
            };
            assert_eq!(map_color_rgba8(packed), expected, "packed id {packed}");
        }
    }

    #[test]
    fn extraction_cache_reuses_full_owner_and_emits_only_consecutive_patch_pixels() {
        let mut world = WorldStore::new();
        world.apply_map_item_data(MapItemData {
            map_id: 4,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: None,
        });
        let mut cache = MapTextureExtractionCache::default();
        cache.begin_frame();
        let first = cache.texture(world.map_item(4).unwrap());
        assert!(first.patch.is_none());
        cache.begin_frame();
        let unchanged = cache.texture(world.map_item(4).unwrap());
        assert!(Arc::ptr_eq(&first.rgba, &unchanged.rgba));
        assert!(unchanged.patch.is_none());

        world.apply_map_item_data(MapItemData {
            map_id: 4,
            scale: 0,
            locked: false,
            decorations: None,
            color_patch: Some(MapColorPatch {
                start_x: 2,
                start_y: 3,
                width: 1,
                height: 1,
                colors: vec![5],
            }),
        });
        cache.begin_frame();
        let updated = cache.texture(world.map_item(4).unwrap());
        let patch = updated.patch.as_ref().unwrap();
        assert_eq!(
            (patch.start_x, patch.start_y, patch.width, patch.height),
            (2, 3, 1, 1)
        );
        assert_eq!(patch.rgba.as_ref(), &map_color_rgba8(5));
        assert_eq!(
            &updated.rgba[(2 + 3 * MAP_SIZE) * 4..(2 + 3 * MAP_SIZE) * 4 + 4],
            &map_color_rgba8(5)
        );
        let repeated = cache.texture(world.map_item(4).unwrap());
        assert_eq!(repeated.patch, updated.patch);
        assert!(Arc::ptr_eq(&repeated.rgba, &updated.rgba));
    }
}
