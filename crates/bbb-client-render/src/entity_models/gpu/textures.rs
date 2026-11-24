use std::{borrow::Borrow, collections::HashSet};

use super::*;
use crate::texture_sampling::{
    copy_rgba_image, create_camera_texture_bind_group, create_nearest_clamp_sampler,
    create_texture_bind_group, rgba_offset, upload_rgba_texture, validate_rgba,
};

pub(crate) const MAX_ENTITY_MODEL_ATLAS_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_DYNAMIC_ENTITY_ATLAS_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_ENTITY_MODEL_OWNED_TEXTURES: usize = 8_192;

#[allow(clippy::too_many_arguments)]
pub(super) fn create_entity_model_texture_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    hud_bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    images: &[EntityModelTextureImage],
    owned_images: &[EntityModelOwnedTextureImage],
    equipment_assets: &EntityEquipmentAssetCatalog,
) -> Result<EntityModelTextureAtlasGpu> {
    let max_dimension = device.limits().max_texture_dimension_2d;
    let (layout, owned_entries, rgba) = build_entity_model_texture_atlas_with_owned_limits(
        images,
        owned_images,
        [max_dimension, max_dimension],
        MAX_ENTITY_MODEL_ATLAS_BYTES,
    )?;
    let equipment_assets = equipment_assets.resolve(&owned_entries)?;
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [layout.width, layout.height],
        &rgba,
        "bbb-entity-model-texture-atlas",
    )?;
    let sampler = create_nearest_clamp_sampler(device, "bbb-entity-model-texture-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-entity-model-texture-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-entity-model-texture-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-entity-model-texture-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    let hud_bind_group = create_texture_bind_group(
        device,
        hud_bind_group_layout,
        "bbb-entity-model-texture-hud-bind-group",
        &view,
        &sampler,
    );

    let mut static_index_by_key = hashbrown::HashMap::with_capacity(layout.entries.len());
    let mut static_index_by_path = hashbrown::HashMap::with_capacity(layout.entries.len());
    for (index, entry) in layout.entries.iter().enumerate() {
        static_index_by_key
            .entry((entry.texture.path, entry.texture.size))
            .or_insert(index);
        static_index_by_path
            .entry(entry.texture.path)
            .or_insert(index);
    }
    let mut owned_index_by_ref = hashbrown::HashMap::with_capacity(owned_entries.len());
    let mut owned_index_by_path = hashbrown::HashMap::with_capacity(owned_entries.len());
    for (index, entry) in owned_entries.iter().enumerate() {
        owned_index_by_ref
            .entry(entry.texture.clone())
            .or_insert(index);
        owned_index_by_path
            .entry(std::sync::Arc::clone(&entry.texture.path))
            .or_insert(index);
    }

    Ok(EntityModelTextureAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        hud_bind_group,
        layout,
        owned_entries,
        equipment_assets,
        static_index_by_key,
        static_index_by_path,
        owned_index_by_ref,
        owned_index_by_path,
    })
}

pub(super) const DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY: usize = 16;
pub(super) const DYNAMIC_ENTITY_TEXTURE_MIN_DIMENSION: u32 = 256;

fn create_empty_rgba_atlas_texture(
    device: &wgpu::Device,
    [width, height]: [u32; 2],
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
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
    (texture, view)
}

pub(super) fn write_rgba_subregion(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    [x, y]: [u32; 2],
    [width, height]: [u32; 2],
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
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
}

pub(super) fn dynamic_player_skin_max_slots(max_dimension: u32) -> usize {
    let [skin_w, skin_h] = DynamicPlayerSkinImage::SIZE;
    ((max_dimension / skin_w) as usize).saturating_mul((max_dimension / skin_h) as usize)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_slotted_dynamic_player_skin_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    requested_slots: usize,
    images: &[DynamicPlayerSkinImage],
    previous: Option<&EntityDynamicPlayerSkinAtlasGpu>,
) -> Result<EntityDynamicPlayerSkinAtlasGpu> {
    let [skin_w, skin_h] = DynamicPlayerSkinImage::SIZE;
    let max_dimension = device.limits().max_texture_dimension_2d;
    let max_cols = (max_dimension / skin_w) as usize;
    let max_rows = (max_dimension / skin_h) as usize;
    let max_slots = max_cols.saturating_mul(max_rows);
    if max_slots == 0 || images.len() > max_slots {
        bail!(
            "dynamic player skin count {} exceeds device 2D texture limit {}",
            images.len(),
            max_dimension
        );
    }

    let target_slots = requested_slots
        .max(images.len())
        .max(DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY)
        .min(max_slots);
    let rows_per_col = target_slots.min(max_rows).max(1);
    let cols = target_slots.div_ceil(rows_per_col).max(1);
    let slot_capacity = cols * rows_per_col;
    let width = skin_w
        .checked_mul(u32::try_from(cols).map_err(|_| anyhow!("skin atlas cols overflow"))?)
        .ok_or_else(|| anyhow!("dynamic player skin atlas width overflow"))?;
    let height = skin_h
        .checked_mul(u32::try_from(rows_per_col).map_err(|_| anyhow!("skin atlas rows overflow"))?)
        .ok_or_else(|| anyhow!("dynamic player skin atlas height overflow"))?;

    let (texture, view) =
        create_empty_rgba_atlas_texture(device, [width, height], "bbb-dynamic-player-skin-atlas");
    let sampler = create_nearest_clamp_sampler(device, "bbb-dynamic-player-skin-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-skin-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-skin-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-skin-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );

    let mut entries = Vec::with_capacity(images.len());
    let mut slot_last_used = vec![0u64; slot_capacity];
    let mut use_clock = previous.map_or(0, |prev| prev.use_clock);
    for (slot, image) in images.iter().enumerate() {
        validate_rgba(skin_w, skin_h, &image.rgba)?;
        let origin = [
            (slot / rows_per_col) as u32 * skin_w,
            (slot % rows_per_col) as u32 * skin_h,
        ];
        write_rgba_subregion(queue, &texture, origin, [skin_w, skin_h], &image.rgba);
        let stamp = previous
            .and_then(|prev| prev.last_used_for_handle(image.handle))
            .unwrap_or_else(|| {
                use_clock += 1;
                use_clock
            });
        slot_last_used[slot] = stamp;
        use_clock = use_clock.max(stamp);
        entries.push(EntityDynamicPlayerSkinAtlasEntry {
            handle: image.handle,
            pixel_origin: origin,
            size: [skin_w, skin_h],
            uv: EntityModelUvRect {
                min: [
                    origin[0] as f32 / width as f32,
                    origin[1] as f32 / height as f32,
                ],
                max: [
                    (origin[0] + skin_w) as f32 / width as f32,
                    (origin[1] + skin_h) as f32 / height as f32,
                ],
            },
            standalone_sampling: image.standalone_sampling,
        });
    }

    entries.sort_unstable_by_key(|entry| entry.handle);
    Ok(EntityDynamicPlayerSkinAtlasGpu {
        texture,
        view,
        sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        layout: EntityDynamicPlayerSkinAtlasLayout {
            width,
            height,
            entries,
        },
        slot_capacity,
        rows_per_col,
        next_unused_slot: images.len(),
        free_slots: Vec::new(),
        slot_last_used,
        use_clock,
    })
}

struct PackedDynamicTextureColumns {
    origins: Vec<[u32; 2]>,
    packed_width: u32,
    packed_height: u32,
    cursor_x: u32,
    cursor_y: u32,
    column_width: u32,
}

fn pack_dynamic_texture_columns(
    images: &[&DynamicPlayerTextureImage],
    max_column_height: u32,
) -> Result<PackedDynamicTextureColumns> {
    let mut origins = Vec::with_capacity(images.len());
    let mut cursor_x = 0u32;
    let mut cursor_y = 0u32;
    let mut column_width = 0u32;
    let mut packed_width = 0u32;
    let mut packed_height = 0u32;
    for image in images {
        if cursor_y > 0
            && cursor_y
                .checked_add(image.size[1])
                .is_none_or(|fill| fill > max_column_height)
        {
            cursor_x = packed_width;
            cursor_y = 0;
            column_width = 0;
        }
        origins.push([cursor_x, cursor_y]);
        let right = cursor_x
            .checked_add(image.size[0])
            .ok_or_else(|| anyhow!("dynamic player texture atlas width overflow"))?;
        column_width = column_width.max(image.size[0]);
        packed_width = packed_width.max(right);
        cursor_y += image.size[1];
        packed_height = packed_height.max(cursor_y);
    }
    Ok(PackedDynamicTextureColumns {
        origins,
        packed_width,
        packed_height,
        cursor_x,
        cursor_y,
        column_width,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_slotted_dynamic_player_texture_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    hud_bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    min_dimensions: [u32; 2],
    images: &[&DynamicPlayerTextureImage],
) -> Result<EntityDynamicPlayerTextureAtlasGpu> {
    if images.is_empty() {
        bail!("dynamic player texture atlas requires at least one image");
    }
    let max_dimension = device.limits().max_texture_dimension_2d;
    let mut max_image_height = 0u32;
    for (idx, image) in images.iter().enumerate() {
        validate_rgba(image.size[0], image.size[1], &image.rgba)?;
        if idx > 0 && images[idx - 1].handle >= image.handle {
            bail!(
                "duplicate or unsorted dynamic player texture handle {}",
                image.handle
            );
        }
        if image.size[0] == 0 || image.size[1] == 0 {
            bail!(
                "dynamic player texture {} dimensions must be non-zero",
                image.handle
            );
        }
        if image.size[0] > max_dimension {
            bail!(
                "dynamic player texture {} width {} exceeds device limit {}",
                image.handle,
                image.size[0],
                max_dimension
            );
        }
        if image.size[1] > max_dimension {
            bail!(
                "dynamic player texture {} height {} exceeds atlas column limit {}",
                image.handle,
                image.size[1],
                max_dimension
            );
        }
        max_image_height = max_image_height.max(image.size[1]);
    }

    let initial_col_limit = min_dimensions[1]
        .max(DYNAMIC_ENTITY_TEXTURE_MIN_DIMENSION.min(max_dimension))
        .max(max_image_height.next_power_of_two().min(max_dimension))
        .min(max_dimension);
    let mut packed = pack_dynamic_texture_columns(images, initial_col_limit)?;
    let mut col_limit = initial_col_limit;
    if packed.packed_width > max_dimension && col_limit < max_dimension {
        col_limit = max_dimension;
        packed = pack_dynamic_texture_columns(images, col_limit)?;
    }
    if packed.packed_width > max_dimension {
        bail!(
            "dynamic player texture atlas width {} exceeds device limit {}",
            packed.packed_width,
            max_dimension
        );
    }

    let mut width = packed
        .packed_width
        .next_power_of_two()
        .max(min_dimensions[0].min(max_dimension))
        .max(DYNAMIC_ENTITY_TEXTURE_MIN_DIMENSION.min(max_dimension))
        .min(max_dimension)
        .max(packed.packed_width);
    let mut height = packed
        .packed_height
        .next_power_of_two()
        .max(col_limit)
        .min(max_dimension)
        .max(packed.packed_height);
    let mut atlas_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("dynamic player texture atlas size overflow"))?;
    if atlas_bytes > MAX_DYNAMIC_ENTITY_ATLAS_BYTES as u64 {
        // Retry with tight power-of-two bounds around the packed extent before rejecting so
        // speculative headroom does not prematurely trigger the VRAM budget cap.
        width = packed
            .packed_width
            .next_power_of_two()
            .min(max_dimension)
            .max(packed.packed_width);
        height = packed
            .packed_height
            .next_power_of_two()
            .min(max_dimension)
            .max(packed.packed_height);
        atlas_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow!("dynamic player texture atlas size overflow"))?;
        if atlas_bytes > MAX_DYNAMIC_ENTITY_ATLAS_BYTES as u64 {
            bail!(
                "dynamic player texture atlas {width}x{height} ({atlas_bytes} bytes) exceeds {MAX_DYNAMIC_ENTITY_ATLAS_BYTES} byte budget"
            );
        }
    }

    let (texture, view) = create_empty_rgba_atlas_texture(
        device,
        [width, height],
        "bbb-dynamic-player-texture-atlas",
    );
    let sampler = create_nearest_clamp_sampler(device, "bbb-dynamic-player-texture-sampler");
    let bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-texture-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-texture-gui-bind-group",
        gui_camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        bind_group_layout,
        "bbb-dynamic-player-texture-hud3d-bind-group",
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    let hud_bind_group = create_texture_bind_group(
        device,
        hud_bind_group_layout,
        "bbb-dynamic-player-texture-hud-bind-group",
        &view,
        &sampler,
    );

    let mut entries = Vec::with_capacity(images.len());
    for (image, origin) in images.iter().zip(packed.origins) {
        write_rgba_subregion(queue, &texture, origin, image.size, &image.rgba);
        entries.push(EntityDynamicPlayerTextureAtlasEntry {
            handle: image.handle,
            pixel_origin: origin,
            size: image.size,
            uv: EntityModelUvRect {
                min: [
                    origin[0] as f32 / width as f32,
                    origin[1] as f32 / height as f32,
                ],
                max: [
                    (origin[0] + image.size[0]) as f32 / width as f32,
                    (origin[1] + image.size[1]) as f32 / height as f32,
                ],
            },
            standalone_sampling: image.standalone_sampling,
        });
    }

    entries.sort_unstable_by_key(|entry| entry.handle);
    Ok(EntityDynamicPlayerTextureAtlasGpu {
        texture,
        view,
        sampler,
        bind_group,
        hud3d_bind_group,
        gui_bind_group,
        hud_bind_group,
        layout: EntityDynamicPlayerTextureAtlasLayout {
            width,
            height,
            entries,
        },
        cursor_x: packed.cursor_x,
        cursor_y: packed.cursor_y,
        column_width: packed.column_width,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_dynamic_player_skin_atlas(
    images: &[DynamicPlayerSkinImage],
) -> Result<(EntityDynamicPlayerSkinAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("dynamic player skin atlas requires at least one image");
    }

    let width = DynamicPlayerSkinImage::SIZE[0];
    let skin_height = DynamicPlayerSkinImage::SIZE[1];
    let height = skin_height
        .checked_mul(u32::try_from(images.len()).map_err(|_| {
            anyhow!(
                "dynamic player skin atlas image count {} overflows u32",
                images.len()
            )
        })?)
        .ok_or_else(|| anyhow!("dynamic player skin atlas height overflow"))?;
    let atlas_len = rgba_len(width, height, "dynamic player skin atlas")?;
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());

    let mut y = 0u32;
    for image in images {
        validate_rgba(width, skin_height, &image.rgba)?;
        let start = rgba_offset(width, y, 0, "dynamic player skin atlas")?;
        rgba[start..start + image.rgba.len()].copy_from_slice(&image.rgba);
        entries.push(EntityDynamicPlayerSkinAtlasEntry {
            handle: image.handle,
            pixel_origin: [0, y],
            size: DynamicPlayerSkinImage::SIZE,
            uv: EntityModelUvRect {
                min: [0.0, y as f32 / height as f32],
                max: [1.0, (y + skin_height) as f32 / height as f32],
            },
            standalone_sampling: image.standalone_sampling,
        });
        y += skin_height;
    }

    Ok((
        EntityDynamicPlayerSkinAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_dynamic_player_texture_atlas(
    images: &[DynamicPlayerTextureImage],
) -> Result<(EntityDynamicPlayerTextureAtlasLayout, Vec<u8>)> {
    build_dynamic_player_texture_atlas_with_limits(images, [u32::MAX; 2])
}

/// Packs images into vertical columns so the atlas never exceeds
/// `max_column_height`; hardware like llvmpipe caps 2D textures at 16384.
pub(crate) fn build_dynamic_player_texture_atlas_with_limits(
    images: &[impl Borrow<DynamicPlayerTextureImage>],
    [max_width, max_column_height]: [u32; 2],
) -> Result<(EntityDynamicPlayerTextureAtlasLayout, Vec<u8>)> {
    if images.is_empty() {
        bail!("dynamic player texture atlas requires at least one image");
    }

    let mut seen = HashSet::with_capacity(images.len());
    let mut origins = Vec::with_capacity(images.len());
    let mut x = 0u32;
    let mut y = 0u32;
    let mut width = 0u32;
    let mut height = 0u32;
    for image in images {
        let image = image.borrow();
        validate_rgba(image.size[0], image.size[1], &image.rgba)?;
        if !seen.insert(image.handle) {
            bail!("duplicate dynamic player texture handle {}", image.handle);
        }
        if image.size[1] > max_column_height {
            bail!(
                "dynamic player texture {} height {} exceeds atlas column limit {}",
                image.handle,
                image.size[1],
                max_column_height
            );
        }
        if y.checked_add(image.size[1])
            .is_none_or(|fill| fill > max_column_height)
        {
            x = width;
            y = 0;
        }
        origins.push([x, y]);
        let right = x
            .checked_add(image.size[0])
            .ok_or_else(|| anyhow!("dynamic player texture atlas width overflow"))?;
        width = width.max(right);
        y += image.size[1];
        height = height.max(y);
    }
    if width == 0 || height == 0 {
        bail!("dynamic player texture atlas dimensions must be non-zero");
    }
    if width > max_width {
        bail!("dynamic player texture atlas width {width} exceeds device limit {max_width}");
    }
    let atlas_len = rgba_len(width, height, "dynamic player texture atlas")?;
    if atlas_len > MAX_DYNAMIC_ENTITY_ATLAS_BYTES {
        bail!(
            "dynamic player texture atlas {width}x{height} ({atlas_len} bytes) exceeds {MAX_DYNAMIC_ENTITY_ATLAS_BYTES} byte budget"
        );
    }
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());

    for (image, [x, y]) in images.iter().zip(origins) {
        let image = image.borrow();
        let image_width = image.size[0];
        let image_height = image.size[1];
        copy_rgba_image(&mut rgba, width, x, y, image_width, &image.rgba)?;
        entries.push(EntityDynamicPlayerTextureAtlasEntry {
            handle: image.handle,
            pixel_origin: [x, y],
            size: image.size,
            uv: EntityModelUvRect {
                min: [x as f32 / width as f32, y as f32 / height as f32],
                max: [
                    (x + image_width) as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
            standalone_sampling: image.standalone_sampling,
        });
    }

    Ok((
        EntityDynamicPlayerTextureAtlasLayout {
            width,
            height,
            entries,
        },
        rgba,
    ))
}

#[cfg(test)]
pub(crate) fn build_entity_model_texture_atlas(
    images: &[EntityModelTextureImage],
) -> Result<(EntityModelTextureAtlasLayout, Vec<u8>)> {
    build_entity_model_texture_atlas_with_height_limit(images, u32::MAX)
}

/// Packs static pack-backed entity images into vertical columns bounded by
/// the active device's 2D texture limit. Physical pack resolutions are not
/// reduced; model UVs continue to use each texture ref's logical extent.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_entity_model_texture_atlas_with_height_limit(
    images: &[EntityModelTextureImage],
    max_column_height: u32,
) -> Result<(EntityModelTextureAtlasLayout, Vec<u8>)> {
    let (layout, owned_entries, rgba) =
        build_entity_model_texture_atlas_with_owned_height_limit(images, &[], max_column_height)?;
    debug_assert!(owned_entries.is_empty());
    Ok((layout, rgba))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_entity_model_texture_atlas_with_owned_height_limit(
    images: &[EntityModelTextureImage],
    owned_images: &[EntityModelOwnedTextureImage],
    max_column_height: u32,
) -> Result<(
    EntityModelTextureAtlasLayout,
    Vec<EntityModelOwnedTextureAtlasEntry>,
    Vec<u8>,
)> {
    build_entity_model_texture_atlas_with_owned_limits(
        images,
        owned_images,
        [u32::MAX, max_column_height],
        MAX_ENTITY_MODEL_ATLAS_BYTES,
    )
}

pub(crate) fn build_entity_model_texture_atlas_with_owned_limits(
    images: &[EntityModelTextureImage],
    owned_images: &[EntityModelOwnedTextureImage],
    [max_width, max_column_height]: [u32; 2],
    max_bytes: usize,
) -> Result<(
    EntityModelTextureAtlasLayout,
    Vec<EntityModelOwnedTextureAtlasEntry>,
    Vec<u8>,
)> {
    if images.is_empty() && owned_images.is_empty() {
        bail!("entity model texture atlas requires at least one image");
    }
    if owned_images.len() > MAX_ENTITY_MODEL_OWNED_TEXTURES {
        bail!(
            "entity model owned texture count {} exceeds limit {MAX_ENTITY_MODEL_OWNED_TEXTURES}",
            owned_images.len()
        );
    }
    let mut seen = HashSet::with_capacity(images.len());
    let mut column_width = 0u32;
    for image in images {
        validate_rgba(image.size[0], image.size[1], &image.rgba)?;
        if !seen.insert(image.texture.path) {
            bail!("duplicate entity model texture {}", image.texture.path);
        }
        if image.size[1] > max_column_height {
            bail!(
                "entity model texture {} height {} exceeds atlas column limit {}",
                image.texture.path,
                image.size[1],
                max_column_height
            );
        }
        column_width = column_width.max(image.size[0]);
    }
    for image in owned_images {
        validate_rgba(image.size[0], image.size[1], &image.rgba)?;
        if image.texture.size.contains(&0) {
            bail!(
                "entity model texture {} has zero-sized logical dimensions",
                image.texture.path
            );
        }
        if !seen.insert(image.texture.path.as_ref()) {
            bail!("duplicate entity model texture {}", image.texture.path);
        }
        if image.size[1] > max_column_height {
            bail!(
                "entity model texture {} height {} exceeds atlas column limit {}",
                image.texture.path,
                image.size[1],
                max_column_height
            );
        }
        column_width = column_width.max(image.size[0]);
    }
    let mut columns = 1u32;
    let mut column_y = 0u32;
    let mut height = 0u32;
    for image in images {
        let image_height = image.size[1];
        if column_y
            .checked_add(image_height)
            .is_none_or(|fill| fill > max_column_height)
        {
            columns += 1;
            column_y = 0;
        }
        column_y += image_height;
        height = height.max(column_y);
    }
    for image in owned_images {
        let image_height = image.size[1];
        if column_y
            .checked_add(image_height)
            .is_none_or(|fill| fill > max_column_height)
        {
            columns += 1;
            column_y = 0;
        }
        column_y += image_height;
        height = height.max(column_y);
    }
    let width = column_width
        .checked_mul(columns)
        .ok_or_else(|| anyhow!("entity model texture atlas width overflow"))?;
    if width == 0 || height == 0 {
        bail!("entity model texture atlas dimensions must be non-zero");
    }
    if width > max_width {
        bail!("entity model texture atlas width {width} exceeds device limit {max_width}");
    }
    if height > max_column_height {
        bail!(
            "entity model texture atlas height {height} exceeds device limit {max_column_height}"
        );
    }
    let atlas_len = rgba_len(width, height, "entity model texture atlas")?;
    if atlas_len > max_bytes {
        bail!(
            "entity model texture atlas {width}x{height} ({atlas_len} bytes) exceeds {max_bytes} byte budget"
        );
    }
    let mut rgba = vec![0u8; atlas_len];
    let mut entries = Vec::with_capacity(images.len());
    let mut owned_entries = Vec::with_capacity(owned_images.len());
    let mut x = 0u32;
    let mut y = 0u32;
    for image in images {
        let image_width = image.size[0];
        let image_height = image.size[1];
        if y.checked_add(image_height)
            .is_none_or(|fill| fill > max_column_height)
        {
            x += column_width;
            y = 0;
        }
        copy_rgba_image(&mut rgba, width, x, y, image_width, &image.rgba)?;
        entries.push(EntityModelTextureAtlasEntry {
            texture: image.texture,
            uv: EntityModelUvRect {
                min: [x as f32 / width as f32, y as f32 / height as f32],
                max: [
                    (x + image_width) as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
        });
        y += image_height;
    }
    for image in owned_images {
        let image_width = image.size[0];
        let image_height = image.size[1];
        if y.checked_add(image_height)
            .is_none_or(|fill| fill > max_column_height)
        {
            x += column_width;
            y = 0;
        }
        copy_rgba_image(&mut rgba, width, x, y, image_width, &image.rgba)?;
        owned_entries.push(EntityModelOwnedTextureAtlasEntry {
            texture: image.texture.clone(),
            uv: EntityModelUvRect {
                min: [x as f32 / width as f32, y as f32 / height as f32],
                max: [
                    (x + image_width) as f32 / width as f32,
                    (y + image_height) as f32 / height as f32,
                ],
            },
        });
        y += image_height;
    }

    Ok((
        EntityModelTextureAtlasLayout {
            width,
            height,
            entries,
        },
        owned_entries,
        rgba,
    ))
}

fn rgba_len(width: u32, height: u32, label: &str) -> Result<usize> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA size overflow"))
}
