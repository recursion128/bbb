use anyhow::Result;

use super::*;
use crate::texture_sampling::{
    copy_rgba_image, create_camera_texture_bind_group, create_nearest_clamp_sampler,
    upload_rgba_texture,
};

pub(super) const MAX_MAP_AUX_ATLAS_BYTES: usize = 32 * 1024 * 1024;
const DEFAULT_MAX_MAP_AUX_ATLAS_DIMENSION: u32 = 16_384;

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_first_person_map_background_atlas(
    textures: &[FirstPersonMapBackgroundTexture],
) -> Option<(FirstPersonMapBackgroundAtlasLayout, Vec<u8>)> {
    build_first_person_map_background_atlas_with_limits(
        textures,
        DEFAULT_MAX_MAP_AUX_ATLAS_DIMENSION,
        MAX_MAP_AUX_ATLAS_BYTES,
    )
    .ok()
    .flatten()
}

pub(super) fn build_first_person_map_background_atlas_with_limits(
    textures: &[FirstPersonMapBackgroundTexture],
    max_dimension: u32,
    max_bytes: usize,
) -> Result<Option<(FirstPersonMapBackgroundAtlasLayout, Vec<u8>)>> {
    let mut by_kind: BTreeMap<FirstPersonMapBackgroundKind, &FirstPersonMapBackgroundTexture> =
        BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_kind.insert(texture.kind, texture);
        }
    }
    if by_kind.is_empty() {
        return Ok(None);
    }
    let width = by_kind
        .values()
        .map(|texture| texture.width)
        .max()
        .unwrap_or(0);
    let height = by_kind
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))
        .ok_or_else(|| anyhow::anyhow!("first-person map background atlas height overflow"))?;
    if width > max_dimension || height > max_dimension {
        anyhow::bail!(
            "first-person map background atlas dimensions {width}x{height} exceed limit {max_dimension}"
        );
    }
    let atlas_len = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("first-person map background atlas size overflow"))?;
    if atlas_len > max_bytes {
        anyhow::bail!(
            "first-person map background atlas size {atlas_len} bytes exceeds {max_bytes} byte budget"
        );
    }
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (kind, texture) in by_kind {
        copy_rgba_image(
            &mut atlas_rgba,
            width,
            0,
            y_offset,
            texture.width,
            &texture.rgba,
        )?;
        rects.insert(
            kind,
            ItemFrameMapUvRect {
                min: [0.0, y_offset as f32 / height as f32],
                max: [
                    texture.width as f32 / width as f32,
                    (y_offset + texture.height) as f32 / height as f32,
                ],
            },
        );
        y_offset += texture.height;
    }
    Ok(Some((
        FirstPersonMapBackgroundAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    )))
}

pub(super) fn create_first_person_map_background_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    layout: FirstPersonMapBackgroundAtlasLayout,
    rgba: &[u8],
) -> Result<FirstPersonMapBackgroundAtlasGpu> {
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [layout.width, layout.height],
        rgba,
        "bbb-first-person-map-background-atlas-texture",
    )?;
    let sampler =
        create_nearest_clamp_sampler(device, "bbb-first-person-map-background-atlas-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-first-person-map-background-atlas-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    Ok(FirstPersonMapBackgroundAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    })
}

#[cfg(test)]
pub(super) fn build_item_frame_map_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapTexture>,
) -> Option<(ItemFrameMapAtlasLayout, Vec<u8>)> {
    let mut by_id: BTreeMap<i32, &[u8]> = BTreeMap::new();
    for texture in textures {
        if texture.rgba.len() == ITEM_FRAME_MAP_RGBA_LEN {
            by_id.insert(texture.map_id, &texture.rgba);
        }
    }
    let map_count = u32::try_from(by_id.len()).ok()?;
    if map_count == 0 {
        return None;
    }
    let width = ITEM_FRAME_MAP_SIZE;
    let height = ITEM_FRAME_MAP_SIZE.checked_mul(map_count)?;
    let mut atlas_rgba = vec![0; width as usize * height as usize * 4];
    let mut rects = BTreeMap::new();
    let per_map_len = ITEM_FRAME_MAP_RGBA_LEN;
    for (slot, (map_id, rgba)) in by_id.into_iter().enumerate() {
        let dst = slot * per_map_len;
        atlas_rgba[dst..dst + per_map_len].copy_from_slice(rgba);
        let v0 = slot as f32 / map_count as f32;
        let v1 = (slot as f32 + 1.0) / map_count as f32;
        rects.insert(
            map_id,
            ItemFrameMapUvRect {
                min: [0.0, v0],
                max: [1.0, v1],
            },
        );
    }
    Some((
        ItemFrameMapAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn map_texture_slot_origin(slot: u32, side_slots: u32) -> wgpu::Origin3d {
    wgpu::Origin3d {
        x: (slot % side_slots) * ITEM_FRAME_MAP_SIZE,
        y: (slot / side_slots) * ITEM_FRAME_MAP_SIZE,
        z: 0,
    }
}

pub(super) fn map_texture_slot_rect(slot: u32, side_slots: u32) -> ItemFrameMapUvRect {
    let atlas_size = (side_slots * ITEM_FRAME_MAP_SIZE) as f32;
    let origin = map_texture_slot_origin(slot, side_slots);
    ItemFrameMapUvRect {
        min: [
            (origin.x as f32 + 0.5) / atlas_size,
            (origin.y as f32 + 0.5) / atlas_size,
        ],
        max: [
            (origin.x as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
            (origin.y as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
        ],
    }
}

pub(super) fn create_item_frame_map_atlas_gpu() -> ItemFrameMapAtlasGpu {
    let atlas_size = MAP_ATLAS_PAGE_SIDE_SLOTS * ITEM_FRAME_MAP_SIZE;
    ItemFrameMapAtlasGpu {
        pages: Vec::new(),
        overflow_pages: Vec::new(),
        overflow_locations: BTreeMap::new(),
        layout: ItemFrameMapAtlasLayout {
            width: atlas_size,
            height: atlas_size,
            rects: BTreeMap::new(),
        },
        owners: BTreeMap::new(),
        frame_index: 0,
        max_resident_textures: MAX_RESIDENT_MAP_TEXTURES,
        max_overflow_pages: MAX_OVERFLOW_MAP_PAGES,
    }
}

pub(super) fn create_item_frame_map_atlas_page_gpu(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
) -> ItemFrameMapAtlasPageGpu {
    let atlas_size = MAP_ATLAS_PAGE_SIDE_SLOTS * ITEM_FRAME_MAP_SIZE;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-atlas-texture"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = create_nearest_clamp_sampler(device, "bbb-item-frame-map-atlas-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-atlas-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-atlas-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-atlas-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    ItemFrameMapAtlasPageGpu {
        texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        free_slots: (0..MAP_ATLAS_PAGE_SIDE_SLOTS * MAP_ATLAS_PAGE_SIDE_SLOTS)
            .rev()
            .collect(),
    }
}

pub(super) fn allocate_resident_map_slot(
    atlas: &mut ItemFrameMapAtlasGpu,
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
) -> Option<(usize, u32)> {
    if let Some((page_index, slot)) = atlas
        .pages
        .iter_mut()
        .enumerate()
        .find_map(|(page, owner)| owner.free_slots.pop().map(|slot| (page, slot)))
    {
        return Some((page_index, slot));
    }
    if atlas.pages.len() >= MAX_RESIDENT_MAP_PAGES {
        return None;
    }
    atlas.pages.push(create_item_frame_map_atlas_page_gpu(
        device,
        bind_group_layout,
        camera_buffer,
        hud3d_camera_buffer,
        gui_camera_buffer,
    ));
    let page_index = atlas.pages.len() - 1;
    let slot = atlas.pages[page_index]
        .free_slots
        .pop()
        .expect("new resident map page has a free slot");
    Some((page_index, slot))
}

pub(super) fn write_full_map_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    owner: &ResidentMapTexture,
) {
    write_full_map_pixels(queue, texture, owner.slot, &owner.rgba);
}

pub(super) fn write_full_map_pixels(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    slot: u32,
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: map_texture_slot_origin(slot, MAP_ATLAS_PAGE_SIDE_SLOTS),
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(ITEM_FRAME_MAP_SIZE * 4),
            rows_per_image: Some(ITEM_FRAME_MAP_SIZE),
        },
        wgpu::Extent3d {
            width: ITEM_FRAME_MAP_SIZE,
            height: ITEM_FRAME_MAP_SIZE,
            depth_or_array_layers: 1,
        },
    );
}

pub(super) fn write_map_texture_patch(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    side_slots: u32,
    slot: u32,
    patch: &ItemFrameMapTexturePatch,
) -> bool {
    let width = u32::from(patch.width);
    let height = u32::from(patch.height);
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok());
    if width == 0 || height == 0 || expected != Some(patch.rgba.len()) {
        return false;
    }
    let mut origin = map_texture_slot_origin(slot, side_slots);
    origin.x += u32::from(patch.start_x);
    origin.y += u32::from(patch.start_y);
    if u32::from(patch.start_x) + width > ITEM_FRAME_MAP_SIZE
        || u32::from(patch.start_y) + height > ITEM_FRAME_MAP_SIZE
    {
        return false;
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        &patch.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    true
}

pub(super) fn insert_resident_map_texture(
    atlas: &mut ItemFrameMapAtlasGpu,
    owner: ResidentMapTexture,
) {
    let map_id = owner.map_id;
    atlas.layout.rects.insert(
        map_id,
        map_texture_slot_rect(owner.slot, MAP_ATLAS_PAGE_SIDE_SLOTS),
    );
    atlas.owners.insert(map_id, owner);
}

pub(super) fn remove_resident_map_texture(atlas: &mut ItemFrameMapAtlasGpu, map_id: i32) {
    if let Some(owner) = atlas.owners.remove(&map_id) {
        atlas.pages[owner.page].free_slots.push(owner.slot);
        atlas.layout.rects.remove(&map_id);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_item_frame_map_decoration_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapDecorationTexture>,
) -> Option<(ItemFrameMapDecorationAtlasLayout, Vec<u8>)> {
    build_item_frame_map_decoration_atlas_with_limits(
        textures,
        DEFAULT_MAX_MAP_AUX_ATLAS_DIMENSION,
        MAX_MAP_AUX_ATLAS_BYTES,
    )
    .ok()
    .flatten()
}

pub(super) fn build_item_frame_map_decoration_atlas_with_limits<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapDecorationTexture>,
    max_dimension: u32,
    max_bytes: usize,
) -> Result<Option<(ItemFrameMapDecorationAtlasLayout, Vec<u8>)>> {
    let mut by_id: BTreeMap<&str, &ItemFrameMapDecorationTexture> = BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_id.insert(texture.sprite_id.as_str(), texture);
        }
    }
    if by_id.is_empty() {
        return Ok(None);
    }
    let width = by_id
        .values()
        .map(|texture| texture.width)
        .max()
        .unwrap_or(0);
    let height = by_id
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))
        .ok_or_else(|| anyhow::anyhow!("map decoration atlas height overflow"))?;
    if width > max_dimension || height > max_dimension {
        anyhow::bail!(
            "map decoration atlas dimensions {width}x{height} exceed limit {max_dimension}"
        );
    }
    let atlas_len = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("map decoration atlas size overflow"))?;
    if atlas_len > max_bytes {
        anyhow::bail!(
            "map decoration atlas size {atlas_len} bytes exceeds {max_bytes} byte budget"
        );
    }
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (sprite_id, texture) in by_id {
        copy_rgba_image(
            &mut atlas_rgba,
            width,
            0,
            y_offset,
            texture.width,
            &texture.rgba,
        )?;
        rects.insert(
            sprite_id.to_string(),
            ItemFrameMapUvRect {
                min: [0.0, y_offset as f32 / height as f32],
                max: [
                    texture.width as f32 / width as f32,
                    (y_offset + texture.height) as f32 / height as f32,
                ],
            },
        );
        y_offset += texture.height;
    }
    Ok(Some((
        ItemFrameMapDecorationAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    )))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_item_frame_map_decoration_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    layout: ItemFrameMapDecorationAtlasLayout,
    rgba: &[u8],
) -> Result<ItemFrameMapDecorationAtlasGpu> {
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [layout.width, layout.height],
        rgba,
        "bbb-item-frame-map-decoration-atlas-texture",
    )?;
    let sampler =
        create_nearest_clamp_sampler(device, "bbb-item-frame-map-decoration-atlas-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-decoration-atlas-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-decoration-atlas-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-decoration-atlas-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    Ok(ItemFrameMapDecorationAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        layout,
    })
}

pub(super) fn create_item_frame_map_text_font_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    font_texture: &wgpu::Texture,
) -> ItemFrameMapTextFontAtlasGpu {
    let view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler =
        create_nearest_clamp_sampler(device, "bbb-item-frame-map-text-font-atlas-sampler");
    // Vanilla's font provider exposes a dedicated opaque white `effect()`
    // glyph for underline/strikethrough rectangles. Keep the equivalent 1x1
    // material beside the shared font atlas instead of assuming a resource
    // glyph contains an opaque white texel.
    let (effect_texture, effect_view) = upload_rgba_texture(
        device,
        queue,
        [1, 1],
        &[255, 255, 255, 255],
        "bbb-item-frame-map-text-effect-texture",
    )
    .expect("1x1 map text effect texture fits device limits");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-text-font-atlas-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let effect_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-text-effect-bind-group",
        camera_buffer,
        &effect_view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-text-font-atlas-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-item-frame-map-text-font-atlas-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    ItemFrameMapTextFontAtlasGpu {
        _view: view,
        _effect_texture: effect_texture,
        _effect_view: effect_view,
        _sampler: sampler,
        bind_group,
        effect_bind_group,
        hud3d_bind_group,
        gui_bind_group,
    }
}
