use anyhow::{anyhow, bail, Result};
use glam::Vec3;

use crate::{
    camera::TerrainBounds,
    frame_buffers::{
        retire_gpu_textures, write_changed_staged, FrameDataBuffer, StagedCommands, UploadBelt,
    },
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    player_skin::{DynamicEntityTextureImage, DynamicPlayerSkinImage, DynamicPlayerTextureImage},
    texture_sampling::validate_rgba,
    Renderer,
};

use super::{
    catalog::{
        EntityCustomHeadSkull, EntityDynamicPlayerSkinAtlasEntry,
        EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerSkinStatus,
        EntityDynamicPlayerTextureAtlasEntry, EntityDynamicPlayerTextureAtlasLayout,
        EntityModelKind, EntityModelOwnedTextureAtlasEntry, EntityModelOwnedTextureImage,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureImage,
        EntityModelUvRect, EntityPlayerSkin, FirstPersonPlayerArm,
    },
    entity_model_water_mask_mesh,
    equipment::{EntityEquipmentAssetCatalog, ResolvedEntityEquipmentAssetCatalog},
    first_person_player_arm_textured_meshes,
    geometry::{
        EntityModelDissolveMesh, EntityModelDissolveVertex, EntityModelMesh, EntityModelPortalMesh,
        EntityModelPortalVertex, EntityModelScrollMesh, EntityModelScrollVertex,
        EntityModelTexturedMesh, EntityModelTexturedVertex, EntityModelVertex,
    },
    instances::EntityModelInstance,
    textured::{
        elder_guardian_particle_textured_meshes, experience_orb_pickup_particle_textured_mesh,
        projectile_pickup_particle_textured_meshes, ElderGuardianParticleRenderInstance,
        ExperienceOrbPickupParticleRenderInstance, ProjectilePickupParticleRenderInstance,
    },
};

mod meshes;
mod pipelines;
mod shaders;
mod textures;
use meshes::*;
pub(crate) use meshes::{
    upload_elder_guardian_particle_textured_mesh,
    upload_experience_orb_pickup_particle_textured_mesh,
    upload_projectile_pickup_particle_textured_mesh,
};
pub(crate) use pipelines::*;
pub(in crate::entity_models) use shaders::*;
pub(crate) use textures::*;

pub(in crate::entity_models) fn entity_model_instances_have_camera_billboards(
    instances: &[EntityModelInstance],
) -> bool {
    instances.iter().any(|instance| {
        matches!(
            instance.kind,
            EntityModelKind::ExperienceOrb { .. }
                | EntityModelKind::DragonFireball
                | EntityModelKind::FishingHook
        )
    })
}

pub(crate) struct EntityModelMeshGpu {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    vertex_bounds: Option<TerrainBounds>,
    vertex_capacity: u64,
    index_capacity: u64,
    vertex_bytes: Vec<u8>,
    indices: Vec<u32>,
    parts: Vec<super::retained::ModelPartDraw>,
    part_bindings: Option<(u64, wgpu::BindGroup)>,
    pub(crate) vertex_pose: Option<super::retained::EntityPoseBinding>,
}

impl EntityModelMeshGpu {
    pub(crate) fn collect_retire(self, buffers: &mut Vec<wgpu::Buffer>) {
        buffers.push(self.vertex_buffer);
        buffers.push(self.index_buffer);
        if let Some(pose) = self.vertex_pose {
            buffers.push(pose.into_buffer());
        }
    }

    pub(crate) fn camera_bounds(
        &self,
        geometry: &super::retained::EntityGeometryResidency,
    ) -> impl Iterator<Item = TerrainBounds> {
        let corners = self
            .parts
            .iter()
            .flat_map(|part| geometry.part_corners(part));
        TerrainBounds::from_points(corners)
            .into_iter()
            .chain(self.vertex_bounds)
    }
}

pub(crate) struct EntityModelTextureAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
    pub(crate) hud_bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityModelTextureAtlasLayout,
    pub(crate) owned_entries: Vec<EntityModelOwnedTextureAtlasEntry>,
    pub(crate) equipment_assets: ResolvedEntityEquipmentAssetCatalog,
    pub(super) static_index_by_key: hashbrown::HashMap<(&'static str, [u32; 2]), usize>,
    pub(super) static_index_by_path: hashbrown::HashMap<&'static str, usize>,
    pub(super) owned_index_by_ref: hashbrown::HashMap<crate::EntityModelOwnedTextureRef, usize>,
    pub(super) owned_index_by_path: hashbrown::HashMap<std::sync::Arc<str>, usize>,
}

impl EntityModelTextureAtlasGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._texture
    }

    /// `O(1)` lookup of a static entity texture UV rect by exact `(path, logical_size)`.
    pub(crate) fn static_uv(
        &self,
        texture: &crate::EntityModelTextureRef,
    ) -> Option<EntityModelUvRect> {
        let &index = self
            .static_index_by_key
            .get(&(texture.path, texture.size))?;
        self.layout.entries.get(index).map(|entry| entry.uv)
    }

    /// `O(1)` lookup of a pack-owned entity texture UV rect: prefers an exact owned
    /// `(path, logical_size)` match, then an owned path match, then a static path match.
    pub(crate) fn pack_uv(
        &self,
        texture: &crate::EntityModelOwnedTextureRef,
    ) -> Option<EntityModelUvRect> {
        if let Some(&index) = self.owned_index_by_ref.get(texture) {
            return self.owned_entries.get(index).map(|entry| entry.uv);
        }
        if let Some(&index) = self.owned_index_by_path.get(texture.path.as_ref()) {
            return self.owned_entries.get(index).map(|entry| entry.uv);
        }
        if let Some(&index) = self.static_index_by_path.get(texture.path.as_ref()) {
            return self.layout.entries.get(index).map(|entry| entry.uv);
        }
        None
    }
}

impl EntityModelTextureAtlasGpu {
    /// The atlas texture view + sampler, for bind groups that pair the atlas with a camera other
    /// than the world camera (the first-person HUD-3D and GUI entity PIP targets).
    pub(in crate::entity_models) fn view_and_sampler(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self._view, &self._sampler)
    }

    fn end_portal_texture_rects(&self) -> Option<(EntityModelUvRect, EntityModelUvRect)> {
        let find = |path: &str| {
            self.layout
                .entries
                .iter()
                .find(|entry| entry.texture.path == path)
                .map(|entry| entry.uv)
        };
        Some((
            find("textures/environment/end_sky.png")?,
            find("textures/entity/end_portal/end_portal.png")?,
        ))
    }
}

pub(crate) struct EntityDynamicPlayerSkinAtlasGpu {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityDynamicPlayerSkinAtlasLayout,
    pub(super) slot_capacity: usize,
    pub(super) rows_per_col: usize,
    pub(super) next_unused_slot: usize,
    pub(super) free_slots: Vec<usize>,
    pub(super) slot_last_used: Vec<u64>,
    pub(super) use_clock: u64,
}

impl EntityDynamicPlayerSkinAtlasGpu {
    pub(in crate::entity_models) fn view_and_sampler(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self.view, &self.sampler)
    }

    pub(super) fn slot_of_origin(&self, origin: [u32; 2]) -> usize {
        let [skin_w, skin_h] = DynamicPlayerSkinImage::SIZE;
        (origin[0] / skin_w) as usize * self.rows_per_col + (origin[1] / skin_h) as usize
    }

    pub(super) fn origin_of_slot(&self, slot: usize) -> [u32; 2] {
        let [skin_w, skin_h] = DynamicPlayerSkinImage::SIZE;
        [
            (slot / self.rows_per_col) as u32 * skin_w,
            (slot % self.rows_per_col) as u32 * skin_h,
        ]
    }

    pub(super) fn uv_of_origin(&self, origin: [u32; 2]) -> EntityModelUvRect {
        let [skin_w, skin_h] = DynamicPlayerSkinImage::SIZE;
        EntityModelUvRect {
            min: [
                origin[0] as f32 / self.layout.width as f32,
                origin[1] as f32 / self.layout.height as f32,
            ],
            max: [
                (origin[0] + skin_w) as f32 / self.layout.width as f32,
                (origin[1] + skin_h) as f32 / self.layout.height as f32,
            ],
        }
    }

    pub(crate) fn uv_for_handle(&self, handle: u64) -> Option<EntityModelUvRect> {
        let idx = self
            .layout
            .entries
            .binary_search_by_key(&handle, |entry| entry.handle)
            .ok()?;
        Some(self.layout.entries[idx].uv)
    }

    pub(super) fn last_used_for_handle(&self, handle: u64) -> Option<u64> {
        let idx = self
            .layout
            .entries
            .binary_search_by_key(&handle, |entry| entry.handle)
            .ok()?;
        let slot = self.slot_of_origin(self.layout.entries[idx].pixel_origin);
        self.slot_last_used.get(slot).copied()
    }

    fn claim_free_slot(&mut self) -> Option<usize> {
        if let Some(slot) = self.free_slots.pop() {
            return Some(slot);
        }
        if self.next_unused_slot < self.slot_capacity {
            let slot = self.next_unused_slot;
            self.next_unused_slot += 1;
            return Some(slot);
        }
        None
    }
}

pub(crate) struct EntityDynamicPlayerTextureAtlasGpu {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) hud3d_bind_group: wgpu::BindGroup,
    pub(crate) gui_bind_group: wgpu::BindGroup,
    pub(crate) hud_bind_group: wgpu::BindGroup,
    pub(crate) layout: EntityDynamicPlayerTextureAtlasLayout,
    pub(super) cursor_x: u32,
    pub(super) cursor_y: u32,
    pub(super) column_width: u32,
}

impl EntityDynamicPlayerTextureAtlasGpu {
    pub(crate) fn uv_for_handle(&self, handle: u64) -> Option<EntityModelUvRect> {
        let idx = self
            .layout
            .entries
            .binary_search_by_key(&handle, |entry| entry.handle)
            .ok()?;
        Some(self.layout.entries[idx].uv)
    }
}

impl EntityDynamicPlayerTextureAtlasGpu {
    pub(in crate::entity_models) fn view_and_sampler(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self.view, &self.sampler)
    }

    fn uv_of_rect(&self, origin: [u32; 2], size: [u32; 2]) -> EntityModelUvRect {
        EntityModelUvRect {
            min: [
                origin[0] as f32 / self.layout.width as f32,
                origin[1] as f32 / self.layout.height as f32,
            ],
            max: [
                (origin[0] + size[0]) as f32 / self.layout.width as f32,
                (origin[1] + size[1]) as f32 / self.layout.height as f32,
            ],
        }
    }
}

fn skin_is_ready_dynamic(skin: EntityPlayerSkin, target_handle: Option<u64>) -> bool {
    matches!(
        skin,
        EntityPlayerSkin::Dynamic(dynamic)
            if dynamic.status == EntityDynamicPlayerSkinStatus::Ready
                && target_handle.is_none_or(|handle| dynamic.handle == handle)
    )
}

fn instance_uses_ready_dynamic_player_skin(
    instance: &EntityModelInstance,
    target_handle: Option<u64>,
) -> bool {
    let kind_uses = match instance.kind {
        EntityModelKind::Player { skin, .. } => skin_is_ready_dynamic(skin, target_handle),
        EntityModelKind::SkullBlock {
            skull: EntityCustomHeadSkull::Player(skin),
            ..
        } => skin_is_ready_dynamic(skin, target_handle),
        _ => false,
    };
    kind_uses
        || matches!(
            instance.render_state.custom_head_skull,
            Some(EntityCustomHeadSkull::Player(skin)) if skin_is_ready_dynamic(skin, target_handle)
        )
}

fn upsert_sorted_skin(images: &mut Vec<DynamicPlayerSkinImage>, image: DynamicPlayerSkinImage) {
    match images.binary_search_by_key(&image.handle, |skin| skin.handle) {
        Ok(idx) => images[idx] = image,
        Err(idx) => images.insert(idx, image),
    }
}

fn upsert_sorted_dynamic_texture(
    images: &mut Vec<DynamicEntityTextureImage>,
    image: DynamicEntityTextureImage,
) {
    match images.binary_search_by_key(&image.handle, |entry| entry.handle) {
        Ok(idx) => images[idx] = image,
        Err(idx) => images.insert(idx, image),
    }
}

pub(super) const ENTITY_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2, 3 => Float32x2, 4 => Float32x3];

/// Vanilla `RenderPipelines.DRAGON_RAYS_DEPTH`: default depth write plus zero colour writes.
pub(super) const ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_WRITE_ENABLED: bool = true;

impl Renderer {
    pub(crate) fn entity_model_meshes_have_camera_billboards(&self) -> bool {
        entity_model_instances_have_camera_billboards(&self.entity_model_instances)
    }

    fn rebuild_meshes_for_dynamic_skin_filter(&mut self, target_handle: Option<u64>) {
        if self
            .entity_model_instances
            .iter()
            .any(|instance| instance_uses_ready_dynamic_player_skin(instance, target_handle))
        {
            self.rebuild_entity_model_meshes();
        }
        if self
            .first_person_player_arms
            .iter()
            .any(|arm| skin_is_ready_dynamic(arm.skin, target_handle))
        {
            self.rebuild_first_person_player_arm_meshes();
        }
    }

    /// Selects the exact resource/session owner for runtime entity textures.
    /// Any edge invalidates the CPU images and GPU atlases before downloads or
    /// pack-backed variants from the new generation are admitted.
    pub fn synchronize_dynamic_entity_texture_generation(
        &mut self,
        resource_generation: u64,
        connection_generation: u64,
        connected: bool,
    ) -> bool {
        let next = (resource_generation, connection_generation, connected);
        if self.dynamic_entity_texture_generation == Some(next) {
            return false;
        }
        self.dynamic_entity_texture_generation = Some(next);

        let had_dynamic_textures = !self.entity_dynamic_player_skin_images.is_empty()
            || self.entity_dynamic_player_skin_atlas.is_some()
            || !self.pack.hud_dynamic_player_faces.is_empty()
            || !self.entity_dynamic_player_texture_images.is_empty()
            || self.entity_dynamic_player_texture_atlas.is_some();
        self.entity_dynamic_player_skin_images.clear();
        let mut retired = Vec::new();
        if let Some(old_skin) = self.entity_dynamic_player_skin_atlas.take() {
            retired.push(old_skin.texture);
        }
        for (_, face) in self.pack.hud_dynamic_player_faces.drain() {
            face.collect_retire(&mut retired);
        }
        self.entity_dynamic_player_texture_images.clear();
        if let Some(old_tex) = self.entity_dynamic_player_texture_atlas.take() {
            retired.push(old_tex.texture);
        }
        if !retired.is_empty() {
            retire_gpu_textures(&self.queue, retired);
        }
        if had_dynamic_textures {
            self.mark_hud_dirty();
            self.invalidate_item_model_atlas(
                crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
            );
            self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::DynamicEntity);
            self.rebuild_entity_model_meshes();
            self.rebuild_first_person_player_arm_meshes();
        }
        true
    }

    pub fn upload_entity_model_textures(
        &mut self,
        images: &[EntityModelTextureImage],
    ) -> Result<()> {
        self.upload_entity_model_textures_with_owned(images, &[])
    }

    /// Replaces the shared entity atlas with the static catalog plus
    /// resource-pack-owned special item textures in one atomic upload.
    pub fn upload_entity_model_textures_with_owned(
        &mut self,
        images: &[EntityModelTextureImage],
        owned_images: &[EntityModelOwnedTextureImage],
    ) -> Result<()> {
        self.upload_entity_model_textures_with_owned_and_equipment(
            images,
            owned_images,
            &EntityEquipmentAssetCatalog::default(),
        )
    }

    /// Replaces the shared entity atlas and its generation-matched equipment
    /// layer catalog in one atomic renderer mutation.
    pub fn upload_entity_model_textures_with_owned_and_equipment(
        &mut self,
        images: &[EntityModelTextureImage],
        owned_images: &[EntityModelOwnedTextureImage],
        equipment_assets: &EntityEquipmentAssetCatalog,
    ) -> Result<()> {
        let (default_faces, dynamic_faces) =
            self.build_hud_player_face_owners(images, &self.entity_dynamic_player_skin_images)?;
        let atlas = create_entity_model_texture_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.hud_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            images,
            owned_images,
            equipment_assets,
        )?;
        let mut retired = Vec::new();
        if let Some(old_atlas) = self.pack.entity_model_texture_atlas.replace(atlas) {
            retired.push(old_atlas.into_texture());
        }
        self.pack.hud_win_portal_mesh = self
            .pack
            .entity_model_texture_atlas
            .as_ref()
            .and_then(|atlas| create_win_screen_end_portal_mesh(&self.device, &self.queue, atlas));
        let old_default_faces =
            std::mem::replace(&mut self.pack.hud_default_player_faces, default_faces);
        let old_dynamic_faces =
            std::mem::replace(&mut self.pack.hud_dynamic_player_faces, dynamic_faces);
        for face in old_default_faces
            .into_values()
            .chain(old_dynamic_faces.into_values())
        {
            face.collect_retire(&mut retired);
        }
        if !retired.is_empty() {
            retire_gpu_textures(&self.queue, retired);
        }
        self.mark_hud_dirty();
        self.rebuild_entity_model_meshes();
        self.rebuild_first_person_player_arm_meshes();
        self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::Entity);
        Ok(())
    }

    pub fn upload_dynamic_player_skin(&mut self, image: DynamicPlayerSkinImage) -> Result<()> {
        let [width, height] = DynamicPlayerSkinImage::SIZE;
        validate_rgba(width, height, &image.rgba)?;
        let max_dimension = self.device.limits().max_texture_dimension_2d;
        let max_slots = dynamic_player_skin_max_slots(max_dimension);
        if max_slots == 0 {
            bail!(
                "device 2D texture limit {} cannot fit a {}x{} player skin",
                max_dimension,
                width,
                height
            );
        }
        let handle = image.handle;
        let face = self.build_hud_dynamic_player_face(&image)?;

        if let Some(atlas) = self.entity_dynamic_player_skin_atlas.as_mut() {
            match atlas
                .layout
                .entries
                .binary_search_by_key(&handle, |entry| entry.handle)
            {
                Ok(entry_idx) => {
                    let origin = atlas.layout.entries[entry_idx].pixel_origin;
                    let slot = atlas.slot_of_origin(origin);
                    atlas.use_clock += 1;
                    atlas.slot_last_used[slot] = atlas.use_clock;
                    let sampling_changed = atlas.layout.entries[entry_idx].standalone_sampling
                        != image.standalone_sampling;
                    atlas.layout.entries[entry_idx].standalone_sampling = image.standalone_sampling;
                    write_rgba_subregion(
                        &self.queue,
                        &atlas.texture,
                        origin,
                        [width, height],
                        &image.rgba,
                    );
                    upsert_sorted_skin(&mut self.entity_dynamic_player_skin_images, image);
                    self.replace_hud_dynamic_player_face(handle, face);
                    self.mark_hud_dirty();
                    if sampling_changed {
                        self.rebuild_meshes_for_dynamic_skin_filter(Some(handle));
                        self.invalidate_item_model_atlas(
                            crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
                        );
                    }
                    return Ok(());
                }
                Err(insert_idx) => {
                    if let Some(slot) = atlas.claim_free_slot() {
                        let origin = atlas.origin_of_slot(slot);
                        let uv = atlas.uv_of_origin(origin);
                        atlas.use_clock += 1;
                        atlas.slot_last_used[slot] = atlas.use_clock;
                        write_rgba_subregion(
                            &self.queue,
                            &atlas.texture,
                            origin,
                            [width, height],
                            &image.rgba,
                        );
                        atlas.layout.entries.insert(
                            insert_idx,
                            EntityDynamicPlayerSkinAtlasEntry {
                                handle,
                                pixel_origin: origin,
                                size: [width, height],
                                uv,
                                standalone_sampling: image.standalone_sampling,
                            },
                        );
                        upsert_sorted_skin(&mut self.entity_dynamic_player_skin_images, image);
                        self.replace_hud_dynamic_player_face(handle, face);
                        self.mark_hud_dirty();
                        self.rebuild_meshes_for_dynamic_skin_filter(Some(handle));
                        return Ok(());
                    }

                    if atlas.slot_capacity >= max_slots && !atlas.layout.entries.is_empty() {
                        let is_active = |candidate: u64| {
                            self.entity_model_instances.iter().any(|instance| {
                                instance_uses_ready_dynamic_player_skin(instance, Some(candidate))
                            }) || self
                                .first_person_player_arms
                                .iter()
                                .any(|arm| skin_is_ready_dynamic(arm.skin, Some(candidate)))
                        };
                        let victim_idx =
                            atlas
                                .layout
                                .entries
                                .iter()
                                .enumerate()
                                .filter(|(_, entry)| !is_active(entry.handle))
                                .min_by_key(|(_, entry)| {
                                    atlas.slot_last_used[atlas.slot_of_origin(entry.pixel_origin)]
                                })
                                .or_else(|| {
                                    atlas.layout.entries.iter().enumerate().min_by_key(
                                        |(_, entry)| {
                                            atlas.slot_last_used
                                                [atlas.slot_of_origin(entry.pixel_origin)]
                                        },
                                    )
                                })
                                .map(|(idx, _)| idx)
                                .expect("non-empty skin atlas has an LRU victim");
                        let evicted = atlas.layout.entries.remove(victim_idx);
                        let slot = atlas.slot_of_origin(evicted.pixel_origin);
                        let uv = evicted.uv;
                        atlas.use_clock += 1;
                        atlas.slot_last_used[slot] = atlas.use_clock;
                        write_rgba_subregion(
                            &self.queue,
                            &atlas.texture,
                            evicted.pixel_origin,
                            [width, height],
                            &image.rgba,
                        );
                        let new_idx = atlas
                            .layout
                            .entries
                            .binary_search_by_key(&handle, |entry| entry.handle)
                            .expect_err("new skin handle is unique");
                        atlas.layout.entries.insert(
                            new_idx,
                            EntityDynamicPlayerSkinAtlasEntry {
                                handle,
                                pixel_origin: evicted.pixel_origin,
                                size: [width, height],
                                uv,
                                standalone_sampling: image.standalone_sampling,
                            },
                        );
                        if let Ok(img_idx) = self
                            .entity_dynamic_player_skin_images
                            .binary_search_by_key(&evicted.handle, |skin| skin.handle)
                        {
                            self.entity_dynamic_player_skin_images.remove(img_idx);
                        }
                        self.remove_hud_dynamic_player_face(evicted.handle);
                        upsert_sorted_skin(&mut self.entity_dynamic_player_skin_images, image);
                        self.replace_hud_dynamic_player_face(handle, face);
                        self.mark_hud_dirty();
                        self.rebuild_meshes_for_dynamic_skin_filter(None);
                        self.invalidate_item_model_atlas(
                            crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
                        );
                        return Ok(());
                    }
                }
            }
        }

        let had_previous_atlas = self.entity_dynamic_player_skin_atlas.is_some();
        let next_capacity = self
            .entity_dynamic_player_skin_atlas
            .as_ref()
            .map_or(DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY, |atlas| {
                atlas.slot_capacity.saturating_mul(2)
            })
            .min(max_slots);
        upsert_sorted_skin(&mut self.entity_dynamic_player_skin_images, image);
        let atlas = create_slotted_dynamic_player_skin_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            next_capacity,
            &self.entity_dynamic_player_skin_images,
            self.entity_dynamic_player_skin_atlas.as_ref(),
        )?;
        if let Some(old) = self.entity_dynamic_player_skin_atlas.replace(atlas) {
            retire_gpu_textures(&self.queue, [old.texture]);
        }
        self.replace_hud_dynamic_player_face(handle, face);
        self.mark_hud_dirty();
        if had_previous_atlas {
            self.rebuild_meshes_for_dynamic_skin_filter(None);
            self.invalidate_item_model_atlas(
                crate::item_models::ItemModelAtlasKind::DynamicPlayerSkin,
            );
        } else {
            self.rebuild_meshes_for_dynamic_skin_filter(Some(handle));
        }
        Ok(())
    }

    fn replace_hud_dynamic_player_face(&mut self, handle: u64, face: crate::hud::HudPlayerFaceGpu) {
        if let Some(old_face) = self.pack.hud_dynamic_player_faces.insert(handle, face) {
            let mut retired = Vec::with_capacity(2);
            old_face.collect_retire(&mut retired);
            if !retired.is_empty() {
                retire_gpu_textures(&self.queue, retired);
            }
        }
    }

    fn remove_hud_dynamic_player_face(&mut self, handle: u64) {
        if let Some(old_face) = self.pack.hud_dynamic_player_faces.remove(&handle) {
            let mut retired = Vec::with_capacity(2);
            old_face.collect_retire(&mut retired);
            if !retired.is_empty() {
                retire_gpu_textures(&self.queue, retired);
            }
        }
    }

    pub fn upload_dynamic_player_texture(
        &mut self,
        image: DynamicPlayerTextureImage,
    ) -> Result<()> {
        self.upload_dynamic_entity_texture(image)
    }

    /// Uploads or replaces one variable-size entity texture in the shared runtime atlas.
    pub fn upload_dynamic_entity_texture(
        &mut self,
        image: DynamicEntityTextureImage,
    ) -> Result<()> {
        self.upload_dynamic_entity_textures(vec![image])
    }

    /// Uploads or replaces a batch of variable-size entity textures, rebuilding the shared runtime
    /// atlas exactly once. Armor-trim paletted permutations use this path for their generated atlas.
    pub fn upload_dynamic_entity_textures(
        &mut self,
        mut images: Vec<DynamicEntityTextureImage>,
    ) -> Result<()> {
        self.install_dynamic_entity_textures(&mut images)
    }

    /// Consumes the images on success. A rejected batch leaves both the input images and
    /// the resident atlas intact so independent texture groups can still be installed.
    pub fn install_dynamic_entity_textures(
        &mut self,
        images: &mut Vec<DynamicEntityTextureImage>,
    ) -> Result<()> {
        if images.is_empty() {
            return Ok(());
        }
        let max_dimension = self.device.limits().max_texture_dimension_2d;
        for image in images.iter() {
            validate_rgba(image.size[0], image.size[1], &image.rgba)?;
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
        }

        // Keep only the final occurrence per handle in sorted handle order without cloning RGBA buffers.
        let mut winning_indices: Vec<(u64, usize)> = images
            .iter()
            .enumerate()
            .map(|(idx, image)| (image.handle, idx))
            .collect();
        winning_indices.sort_by_key(|&(handle, idx)| (handle, idx));
        let mut deduped = Vec::with_capacity(winning_indices.len());
        for (handle, idx) in winning_indices {
            if let Some(last) = deduped.last_mut().filter(|(h, _)| *h == handle) {
                *last = (handle, idx);
            } else {
                deduped.push((handle, idx));
            }
        }

        if let Some(atlas) = self.entity_dynamic_player_texture_atlas.as_mut() {
            let mut sim_x = atlas.cursor_x;
            let mut sim_y = atlas.cursor_y;
            let mut sim_col_w = atlas.column_width;
            let mut placements = Vec::with_capacity(deduped.len());
            let mut fits_all = true;

            for &(handle, idx) in &deduped {
                let size = images[idx].size;
                if let Ok(entry_idx) = atlas
                    .layout
                    .entries
                    .binary_search_by_key(&handle, |entry| entry.handle)
                {
                    if atlas.layout.entries[entry_idx].size == size {
                        placements.push((idx, atlas.layout.entries[entry_idx].pixel_origin, true));
                        continue;
                    }
                }
                if sim_y
                    .checked_add(size[1])
                    .is_some_and(|end_y| end_y <= atlas.layout.height)
                    && sim_x
                        .checked_add(size[0])
                        .is_some_and(|right_x| right_x <= atlas.layout.width)
                {
                    placements.push((idx, [sim_x, sim_y], false));
                    sim_y += size[1];
                    sim_col_w = sim_col_w.max(size[0]);
                } else {
                    let next_x = sim_x.saturating_add(sim_col_w);
                    if size[1] <= atlas.layout.height
                        && next_x
                            .checked_add(size[0])
                            .is_some_and(|right_x| right_x <= atlas.layout.width)
                    {
                        placements.push((idx, [next_x, 0], false));
                        sim_x = next_x;
                        sim_y = size[1];
                        sim_col_w = size[0];
                    } else {
                        fits_all = false;
                        break;
                    }
                }
            }

            if fits_all {
                atlas.cursor_x = sim_x;
                atlas.cursor_y = sim_y;
                atlas.column_width = sim_col_w;
                let mut layout_changed = false;
                let mut existing_entry_mutated = false;
                let mut drained: Vec<Option<DynamicEntityTextureImage>> =
                    images.drain(..).map(Some).collect();
                for (idx, origin, reused_same_size) in placements {
                    let image = drained[idx].take().expect("unique winning index");
                    write_rgba_subregion(
                        &self.queue,
                        &atlas.texture,
                        origin,
                        image.size,
                        &image.rgba,
                    );
                    let uv = atlas.uv_of_rect(origin, image.size);
                    let new_entry = EntityDynamicPlayerTextureAtlasEntry {
                        handle: image.handle,
                        pixel_origin: origin,
                        size: image.size,
                        uv,
                        standalone_sampling: image.standalone_sampling,
                    };
                    match atlas
                        .layout
                        .entries
                        .binary_search_by_key(&image.handle, |entry| entry.handle)
                    {
                        Ok(entry_idx) => {
                            let changed = atlas.layout.entries[entry_idx] != new_entry;
                            atlas.layout.entries[entry_idx] = new_entry;
                            layout_changed |= changed;
                            existing_entry_mutated |= changed || !reused_same_size;
                        }
                        Err(insert_idx) => {
                            atlas.layout.entries.insert(insert_idx, new_entry);
                            layout_changed = true;
                        }
                    }
                    upsert_sorted_dynamic_texture(
                        &mut self.entity_dynamic_player_texture_images,
                        image,
                    );
                }
                if layout_changed {
                    self.mark_hud_dirty();
                    if !self.entity_model_instances.is_empty() {
                        self.rebuild_entity_model_meshes();
                    }
                }
                if existing_entry_mutated {
                    self.invalidate_item_model_atlas(
                        crate::item_models::ItemModelAtlasKind::DynamicEntity,
                    );
                }
                return Ok(());
            }
        }

        let resident = &self.entity_dynamic_player_texture_images;
        let mut sources = Vec::with_capacity(resident.len() + deduped.len());
        let mut r_idx = 0usize;
        let mut i_idx = 0usize;
        while r_idx < resident.len() && i_idx < deduped.len() {
            let r_handle = resident[r_idx].handle;
            let (i_handle, img_pos) = deduped[i_idx];
            match r_handle.cmp(&i_handle) {
                std::cmp::Ordering::Less => {
                    sources.push(&resident[r_idx]);
                    r_idx += 1;
                }
                std::cmp::Ordering::Equal => {
                    sources.push(&images[img_pos]);
                    r_idx += 1;
                    i_idx += 1;
                }
                std::cmp::Ordering::Greater => {
                    sources.push(&images[img_pos]);
                    i_idx += 1;
                }
            }
        }
        sources.extend(&resident[r_idx..]);
        sources.extend(deduped[i_idx..].iter().map(|&(_, pos)| &images[pos]));

        let min_dimensions = self.entity_dynamic_player_texture_atlas.as_ref().map_or(
            [DYNAMIC_ENTITY_TEXTURE_MIN_DIMENSION.min(max_dimension); 2],
            |prev| {
                // Grow one axis at a time (2x VRAM per step instead of 4x) to avoid premature
                // VRAM exhaustion when streaming dynamic player or entity variant textures.
                let w = prev.layout.width;
                let h = prev.layout.height;
                if w <= h && w < max_dimension {
                    [w.saturating_mul(2).min(max_dimension), h.min(max_dimension)]
                } else if h < max_dimension {
                    [w.min(max_dimension), h.saturating_mul(2).min(max_dimension)]
                } else {
                    [w.saturating_mul(2).min(max_dimension), h.min(max_dimension)]
                }
            },
        );
        let atlas = create_slotted_dynamic_player_texture_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.hud_bind_group_layout,
            &self.camera_buffers.world(),
            &self.camera_buffers.hud3d(),
            &self.camera_buffers.gui(),
            min_dimensions,
            &sources,
        )?;
        let had_previous_atlas = self.entity_dynamic_player_texture_atlas.is_some();
        if let Some(old) = self.entity_dynamic_player_texture_atlas.replace(atlas) {
            retire_gpu_textures(&self.queue, [old.texture]);
        }
        let mut drained: Vec<Option<DynamicEntityTextureImage>> =
            images.drain(..).map(Some).collect();
        for (_, pos) in deduped {
            let image = drained[pos].take().expect("unique winning index");
            upsert_sorted_dynamic_texture(&mut self.entity_dynamic_player_texture_images, image);
        }
        self.mark_hud_dirty();
        if !self.entity_model_instances.is_empty() {
            self.rebuild_entity_model_meshes();
        }
        if had_previous_atlas {
            self.invalidate_item_model_atlas(crate::item_models::ItemModelAtlasKind::DynamicEntity);
        }
        Ok(())
    }

    pub fn set_entity_model_instances(&mut self, instances: Vec<EntityModelInstance>) {
        let _ = self.replace_entity_model_instances(instances);
    }

    /// Replaces entity instances and returns a cleared allocation owned by the
    /// previous or unchanged input so render extraction can reuse its capacity.
    pub fn replace_entity_model_instances(
        &mut self,
        instances: Vec<EntityModelInstance>,
    ) -> Vec<EntityModelInstance> {
        let (previous, changed) = self.replace_entity_model_inputs(instances);
        if changed {
            self.refresh_entity_model_meshes();
            self.update_camera();
        }
        previous
    }

    pub(crate) fn replace_entity_model_inputs(
        &mut self,
        instances: Vec<EntityModelInstance>,
    ) -> (Vec<EntityModelInstance>, bool) {
        let mut instances = sanitize_entity_model_instances(instances);
        if self.entity_model_instances.as_slice() == instances.as_slice() {
            instances.clear();
            return (instances, false);
        }

        let mut previous = std::mem::replace(&mut self.entity_model_instances, instances);
        previous.clear();
        (previous, true)
    }

    pub fn set_first_person_player_arms(&mut self, arms: Vec<FirstPersonPlayerArm>) {
        if self.first_person_player_arms.as_slice() == arms.as_slice() {
            return;
        }

        self.first_person_player_arms = arms;
        self.rebuild_first_person_player_arm_meshes();
    }

    pub(crate) fn rebuild_first_person_player_arm_meshes(&mut self) {
        if let Some(atlas) = &self.pack.entity_model_texture_atlas {
            let dynamic_player_skin_atlas = self
                .entity_dynamic_player_skin_atlas
                .as_ref()
                .map(|atlas| &atlas.layout);
            let meshes = first_person_player_arm_textured_meshes(
                &self.first_person_player_arms,
                &atlas.layout,
                dynamic_player_skin_atlas,
            );
            update_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                &mut self.frame_uploads,
                &mut self.pack.first_person_player_arm_mesh,
                meshes.translucent,
                "bbb-first-person-player-arm",
            );
            update_entity_model_textured_mesh_gpu_from_mesh(
                &self.device,
                &mut self.frame_uploads,
                &mut self.pack.first_person_dynamic_player_arm_mesh,
                meshes.dynamic_player_skin_translucent,
                "bbb-first-person-dynamic-player-arm",
            );
            self.entity_meshes_staged_unsubmitted = true;
        } else {
            self.pack.first_person_player_arm_mesh = None;
            self.pack.first_person_dynamic_player_arm_mesh = None;
        }
    }

    pub(crate) fn rebuild_entity_model_meshes(&mut self) {
        self.pack.entity_extract.clear();
        self.refresh_entity_model_meshes();
        self.update_camera();
    }

    pub(crate) fn resort_entity_model_draws(&mut self) {
        let camera = self.camera_sort_position().unwrap_or([0.0; 3]);
        let meshes = &mut self.pack.entity_meshes;
        super::textured::resort_ranges(
            &mut meshes.opaque_beam_draws,
            &mut meshes.sorted_main_translucent_draws,
            &mut meshes.sorted_item_entity_draws,
            camera,
        );
    }
}

pub(super) fn sanitize_entity_model_instances(
    mut instances: Vec<EntityModelInstance>,
) -> Vec<EntityModelInstance> {
    instances.retain(|instance| {
        instance.render_state.body_rot.is_finite()
            && instance
                .position
                .iter()
                .all(|component| component.is_finite())
    });
    instances
}

#[cfg(test)]
mod tests;
