use super::*;
use crate::entity_models::retained;

impl Renderer {
    pub(crate) fn refresh_entity_model_meshes(&mut self) {
        use crate::renderer::EntityMeshLane as Lane;

        let camera = self.camera_sort_position();
        if let Some(atlas) = &self.pack.entity_model_texture_atlas {
            let dynamic_player_skin_atlas = self
                .entity_dynamic_player_skin_atlas
                .as_ref()
                .map(|atlas| &atlas.layout);
            let dynamic_player_texture_atlas = self
                .entity_dynamic_player_texture_atlas
                .as_ref()
                .map(|atlas| &atlas.layout);
            let (meshes, water) = self.pack.entity_extract.build(
                &self.entity_model_instances,
                &atlas.layout,
                &atlas.owned_entries,
                &atlas.equipment_assets,
                dynamic_player_skin_atlas,
                dynamic_player_texture_atlas,
                &self.data_driven_rigs,
                camera,
                self.camera_pose,
            );
            update_entity_model_mesh_gpu_from_mesh(
                &self.device,
                &mut self.frame_uploads,
                &mut self.pack.entity_meshes[Lane::WaterMask],
                water,
                Lane::WaterMask.label(),
            );
            let crumbling_vertices_uploaded = self
                .frame_block_entity_crumbling_vertices
                .upload_staged_commands(
                    &self.device,
                    &mut self.frame_uploads,
                    bytemuck::cast_slice(&meshes.crumbling.vertices),
                );
            let crumbling_indices_uploaded = self
                .frame_block_entity_crumbling_indices
                .upload_staged_commands(
                    &self.device,
                    &mut self.frame_uploads,
                    bytemuck::cast_slice(&meshes.crumbling.indices),
                );
            self.block_entity_crumbling_index_count =
                if crumbling_vertices_uploaded && crumbling_indices_uploaded {
                    u32::try_from(meshes.crumbling.indices.len())
                        .expect("block-entity crumbling index count fits in u32")
                } else {
                    0
                };
            self.pack.entity_meshes.opaque_beam_draws = meshes.opaque_beam_draws;
            self.pack.entity_meshes.sorted_main_translucent_draws =
                meshes.sorted_main_translucent_draws;
            self.pack.entity_meshes.sorted_item_entity_draws = meshes.sorted_item_entity_draws;
            let pose_pipelines = self
                .entity_pose_pipelines
                .as_ref()
                .filter(|_| self.pipelines.shaders.uses_embedded_entity());
            let [solid_scratch, cutout_scratch, cutout_cull_scratch] = [
                (Lane::Solid, meshes.solid),
                (Lane::Textured, meshes.cutout),
                (Lane::TexturedCull, meshes.cutout_cull),
            ]
            .map(|(lane, mesh)| {
                update_surface_entity_mesh(
                    &self.device,
                    &mut self.frame_uploads,
                    &mut self.pack.entity_meshes[lane],
                    mesh,
                    lane.label(),
                    &mut self.pack.entity_geometry,
                    pose_pipelines,
                )
            });
            self.pack.entity_extract.recycle_surface_meshes(
                solid_scratch,
                cutout_scratch,
                cutout_cull_scratch,
            );
            update_entity_model_dissolve_mesh_gpu_from_mesh(
                &self.device,
                &mut self.frame_uploads,
                &mut self.pack.entity_meshes[Lane::Dissolve],
                meshes.dissolve,
                Lane::Dissolve.label(),
            );
            for (lane, mesh) in [
                (Lane::SolidZOffsetForward, meshes.solid_z_offset_forward),
                (Lane::CutoutZOffset, meshes.cutout_z_offset),
                (Lane::ArmorCutout, meshes.armor_cutout),
                (Lane::ArmorTrim, meshes.armor_trim),
                (Lane::ArmorTrimDecal, meshes.armor_trim_decal),
                (Lane::Eyes, meshes.eyes),
                (Lane::Outline, meshes.outline),
                (Lane::OutlineCull, meshes.outline_cull),
                (Lane::Translucent, meshes.translucent),
                (Lane::ArmorTranslucent, meshes.armor_translucent),
                (Lane::TranslucentEmissive, meshes.translucent_emissive),
                (Lane::ItemEntityTranslucent, meshes.item_entity_translucent),
                (
                    Lane::ItemEntityTranslucentCull,
                    meshes.item_entity_translucent_cull,
                ),
                (Lane::SkinCutout, meshes.dynamic_player_skin_cutout),
                (Lane::SkinSolid, meshes.dynamic_player_skin_solid),
                (Lane::SkinCutoutCull, meshes.dynamic_player_skin_cutout_cull),
                (
                    Lane::SkinSolidZOffsetForward,
                    meshes.dynamic_player_skin_solid_z_offset_forward,
                ),
                (
                    Lane::SkinCutoutZOffset,
                    meshes.dynamic_player_skin_cutout_z_offset,
                ),
                (
                    Lane::SkinTranslucent,
                    meshes.dynamic_player_skin_translucent,
                ),
                (
                    Lane::SkinItemEntityTranslucent,
                    meshes.dynamic_player_skin_item_entity_translucent,
                ),
                (
                    Lane::SkinItemEntityTranslucentCull,
                    meshes.dynamic_player_skin_item_entity_translucent_cull,
                ),
                (Lane::SkinOutline, meshes.dynamic_player_skin_outline),
                (
                    Lane::SkinOutlineCull,
                    meshes.dynamic_player_skin_outline_cull,
                ),
                (
                    Lane::PlayerTextureCutout,
                    meshes.dynamic_player_texture_cutout,
                ),
                (
                    Lane::PlayerTextureSolid,
                    meshes.dynamic_player_texture_solid,
                ),
                (
                    Lane::PlayerTextureCutoutCull,
                    meshes.dynamic_player_texture_cutout_cull,
                ),
                (
                    Lane::PlayerTextureSolidZOffsetForward,
                    meshes.dynamic_player_texture_solid_z_offset_forward,
                ),
                (
                    Lane::PlayerTextureCutoutZOffset,
                    meshes.dynamic_player_texture_cutout_z_offset,
                ),
                (
                    Lane::PlayerTextureArmorCutout,
                    meshes.dynamic_player_texture_armor_cutout,
                ),
                (
                    Lane::PlayerTextureArmorTrim,
                    meshes.dynamic_player_texture_armor_trim,
                ),
                (
                    Lane::PlayerTextureArmorTrimDecal,
                    meshes.dynamic_player_texture_armor_trim_decal,
                ),
                (
                    Lane::PlayerTextureTranslucent,
                    meshes.dynamic_player_texture_translucent,
                ),
                (
                    Lane::PlayerTextureItemEntityTranslucent,
                    meshes.dynamic_player_texture_item_entity_translucent,
                ),
                (
                    Lane::PlayerTextureItemEntityTranslucentCull,
                    meshes.dynamic_player_texture_item_entity_translucent_cull,
                ),
                (
                    Lane::PlayerTextureOutline,
                    meshes.dynamic_player_texture_outline,
                ),
                (
                    Lane::PlayerTextureOutlineCull,
                    meshes.dynamic_player_texture_outline_cull,
                ),
            ] {
                update_retained_entity_mesh(
                    &self.device,
                    &mut self.frame_uploads,
                    &mut self.pack.entity_meshes[lane],
                    mesh,
                    lane.label(),
                    &mut self.pack.entity_geometry,
                );
            }
            if let Some(pose_pipelines) = pose_pipelines {
                for lane in [Lane::Solid, Lane::Textured, Lane::TexturedCull] {
                    if let Some(binding) = self.pack.entity_meshes[lane]
                        .as_mut()
                        .and_then(|gpu| gpu.vertex_pose.as_mut())
                    {
                        self.pack.entity_geometry.ensure_pose_binding_current(
                            &self.device,
                            &pose_pipelines.layout,
                            binding,
                        );
                    }
                }
            }
            for (lane, mesh) in [
                (Lane::DragonRays, meshes.dragon_rays),
                (Lane::DragonRaysDepth, meshes.dragon_rays_depth),
                (Lane::Leash, meshes.leash),
            ] {
                update_entity_model_mesh_gpu_from_mesh(
                    &self.device,
                    &mut self.frame_uploads,
                    &mut self.pack.entity_meshes[lane],
                    mesh,
                    lane.label(),
                );
            }
            for (lane, mesh) in [
                (Lane::EndPortal, meshes.end_portal),
                (Lane::EndGateway, meshes.end_gateway),
            ] {
                update_entity_model_portal_mesh_gpu_from_mesh(
                    &self.device,
                    &mut self.frame_uploads,
                    &mut self.pack.entity_meshes[lane],
                    mesh,
                    lane.label(),
                );
            }
            for (lane, mesh) in [
                (Lane::Scroll, meshes.scroll),
                (Lane::ScrollAdditive, meshes.scroll_additive),
                (Lane::GuardianBeam, meshes.guardian_beam),
                (Lane::EndCrystalBeam, meshes.end_crystal_beam),
                (Lane::BeaconBeamOpaque, meshes.beacon_beam_opaque),
                (Lane::BeaconBeamTranslucent, meshes.beacon_beam_translucent),
                (Lane::EntityGlint, meshes.entity_glint),
                (Lane::ArmorEntityGlint, meshes.armor_entity_glint),
            ] {
                update_entity_model_scroll_mesh_gpu_from_mesh(
                    &self.device,
                    &mut self.frame_uploads,
                    &mut self.pack.entity_meshes[lane],
                    mesh,
                    lane.label(),
                );
            }
            self.entity_meshes_staged_unsubmitted = true;
        } else {
            self.block_entity_crumbling_index_count = 0;
            update_entity_model_mesh_gpu_from_mesh(
                &self.device,
                &mut self.frame_uploads,
                &mut self.pack.entity_meshes[Lane::WaterMask],
                entity_model_water_mask_mesh(&self.entity_model_instances),
                Lane::WaterMask.label(),
            );
            let water_mask = self.pack.entity_meshes[Lane::WaterMask].take();
            self.pack.entity_meshes = crate::renderer::EntityMeshes::default();
            self.pack.entity_meshes[Lane::WaterMask] = water_mask;
            self.entity_meshes_staged_unsubmitted = true;
        }
    }
}

macro_rules! entity_mesh_uploads {
    ($($upload:ident: $mesh:ty),* $(,)?) => {$(
        pub(super) fn $upload(
            device: &wgpu::Device,
            uploads: &mut StagedCommands,
            target: &mut Option<EntityModelMeshGpu>,
            mesh: $mesh,
            label_prefix: &str,
        ) -> bool {
            let bounds = TerrainBounds::from_points(
                mesh.vertices.iter().map(|vertex| Vec3::from_array(vertex.position)),
            );
            update_entity_model_mesh_gpu(
                device, uploads, target, bytemuck::cast_slice(&mesh.vertices),
                &mesh.indices, bounds, label_prefix,
            )
        }
    )*};
}

entity_mesh_uploads! {
    update_entity_model_mesh_gpu_from_mesh: EntityModelMesh,
    update_entity_model_textured_mesh_gpu_from_mesh: EntityModelTexturedMesh,
    update_entity_model_scroll_mesh_gpu_from_mesh: EntityModelScrollMesh,
    update_entity_model_dissolve_mesh_gpu_from_mesh: EntityModelDissolveMesh,
    update_entity_model_portal_mesh_gpu_from_mesh: EntityModelPortalMesh,
}

pub(super) fn update_entity_model_mesh_gpu(
    device: &wgpu::Device,
    uploads: &mut StagedCommands,
    target: &mut Option<EntityModelMeshGpu>,
    vertex_bytes: &[u8],
    indices: &[u32],
    bounds: Option<TerrainBounds>,
    label_prefix: &str,
) -> bool {
    if vertex_bytes.is_empty() || indices.is_empty() {
        if let Some(old) = target.take() {
            uploads.belt().retire_buffer(old.vertex_buffer);
            uploads.belt().retire_buffer(old.index_buffer);
        }
        return false;
    }
    let index_bytes = bytemuck::cast_slice(indices);
    let vertex_len = vertex_bytes.len() as u64;
    let index_len = index_bytes.len() as u64;
    if let Some(resident) = target.as_mut().filter(|resident| {
        resident.vertex_capacity >= vertex_len && resident.index_capacity >= index_len
    }) {
        write_changed_staged(
            device,
            uploads,
            &resident.vertex_buffer,
            0,
            vertex_bytes,
            &resident.vertex_bytes,
        );
        write_changed_staged(
            device,
            uploads,
            &resident.index_buffer,
            0,
            index_bytes,
            bytemuck::cast_slice(&resident.indices),
        );
        resident.vertex_bytes.clear();
        resident.vertex_bytes.extend_from_slice(vertex_bytes);
        resident.indices.clear();
        resident.indices.extend_from_slice(indices);
        resident.index_count = indices.len() as u32;
        resident.vertex_bounds = bounds;
        return false;
    }

    let vertex_capacity = vertex_len.next_power_of_two();
    let index_capacity = index_len.next_power_of_two();
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{label_prefix}-vertices")),
        size: vertex_capacity,
        usage: wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{label_prefix}-indices")),
        size: index_capacity,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    uploads.write(device, &vertex_buffer, 0, vertex_bytes);
    uploads.write(device, &index_buffer, 0, index_bytes);
    if let Some(old) = target.replace(EntityModelMeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
        vertex_bounds: bounds,
        vertex_capacity,
        index_capacity,
        vertex_bytes: vertex_bytes.to_vec(),
        indices: indices.to_vec(),
        parts: Vec::new(),
        part_bindings: None,
        vertex_pose: None,
    }) {
        uploads.belt().retire_buffer(old.vertex_buffer);
        uploads.belt().retire_buffer(old.index_buffer);
    }
    true
}

pub(super) fn update_surface_entity_mesh(
    device: &wgpu::Device,
    uploads: &mut StagedCommands,
    target: &mut Option<EntityModelMeshGpu>,
    mut mesh: EntityModelTexturedMesh,
    label: &str,
    geometry: &mut retained::EntityGeometryResidency,
    pipelines: Option<&retained::EntityPosePipelines>,
) -> EntityModelTexturedMesh {
    let previous = target.as_ref();
    // Changing visibility or a new mesh already needs an upload. Start skinning once a
    // stable layout actually animates; a stationary cuboid keeps its ordinary vertex buffer.
    if let Some(pipelines) = pipelines.filter(|p| {
        mesh.vertices.is_empty()
            && retained::uses_direct_vertex_upload(mesh.deferred_vertices)
            && !mesh.parts.is_empty()
            && retained::EntityPoseBinding::fits(mesh.parts.len(), p.storage_limit)
            && previous.is_some_and(|gpu| {
                gpu.parts.len() == mesh.parts.len()
                    && gpu
                        .parts
                        .iter()
                        .zip(&mesh.parts)
                        .all(|(old, new)| old.asset_key() == new.asset_key())
            })
    }) {
        let gpu = previous.unwrap();
        if gpu.vertex_pose.is_none() && gpu.parts != mesh.parts {
            if let Some((keys, binding)) = geometry.vertex_pose(
                device,
                uploads,
                &pipelines.layout,
                &mesh.parts,
                pipelines.storage_limit,
            ) {
                if let Some(old) = target.take() {
                    retire_entity_mesh_gpu(uploads.belt(), old);
                }
                update_entity_model_mesh_gpu(
                    device,
                    uploads,
                    target,
                    bytemuck::cast_slice(&keys),
                    &mesh.indices,
                    None,
                    label,
                );
                let gpu = target.as_mut().unwrap();
                gpu.vertex_bytes = Vec::new();
                gpu.vertex_pose = Some(binding);
            }
        }
        let gpu = target.as_mut().unwrap();
        if let Some(binding) = &mut gpu.vertex_pose {
            binding.update(device, uploads, &mesh.parts, &gpu.parts);
            debug_assert_eq!(mesh.indices, gpu.indices);
            std::mem::swap(&mut gpu.parts, &mut mesh.parts);
            return mesh;
        }
    }
    if target.as_ref().is_some_and(|gpu| gpu.vertex_pose.is_some()) {
        if let Some(old) = target.take() {
            retire_entity_mesh_gpu(uploads.belt(), old);
        }
    }
    update_retained_entity_mesh(device, uploads, target, mesh, label, geometry)
}

fn retire_entity_mesh_gpu(belt: &mut crate::frame_buffers::UploadBelt, old: EntityModelMeshGpu) {
    belt.retire_buffer(old.vertex_buffer);
    belt.retire_buffer(old.index_buffer);
    if let Some(pose) = old.vertex_pose {
        belt.retire_buffer(pose.into_buffer());
    }
}

pub(super) fn update_retained_entity_mesh(
    device: &wgpu::Device,
    uploads: &mut StagedCommands,
    target: &mut Option<EntityModelMeshGpu>,
    mut mesh: EntityModelTexturedMesh,
    label: &str,
    geometry: &mut retained::EntityGeometryResidency,
) -> EntityModelTexturedMesh {
    if mesh.indices.is_empty() {
        if let Some(old) = target.take() {
            retire_entity_mesh_gpu(uploads.belt(), old);
        }
        return mesh;
    }
    let stride = std::mem::size_of::<EntityModelTexturedVertex>();
    let vertex_len = (mesh.vertex_count() * stride) as u64;
    let index_bytes = bytemuck::cast_slice(&mesh.indices);
    let recreate = target.as_ref().is_none_or(|gpu| {
        gpu.vertex_capacity < vertex_len || gpu.index_capacity < index_bytes.len() as u64
    });
    if recreate {
        let vertex_capacity = vertex_len.next_power_of_two();
        let index_capacity = (index_bytes.len() as u64).next_power_of_two();
        if let Some(old) = target.replace(EntityModelMeshGpu {
            vertex_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: vertex_capacity,
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            index_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            index_count: 0,
            vertex_bounds: None,
            vertex_capacity,
            index_capacity,
            vertex_bytes: Vec::new(),
            indices: Vec::new(),
            parts: Vec::new(),
            part_bindings: None,
            vertex_pose: None,
        }) {
            uploads.belt().retire_buffer(old.vertex_buffer);
            uploads.belt().retire_buffer(old.index_buffer);
        }
    }
    let gpu = target.as_mut().unwrap();
    let same_layout = gpu.parts.len() == mesh.parts.len()
        && gpu
            .parts
            .iter()
            .zip(&mesh.parts)
            .all(|(a, b)| a.vertex_start == b.vertex_start && a.vertex_count == b.vertex_count);
    let bytes = bytemuck::cast_slice(&mesh.vertices);
    let previous = if same_layout {
        gpu.vertex_bytes.as_slice()
    } else {
        &[]
    };
    let mut output_cursor = 0;
    let mut source_cursor = 0;
    for part in &mesh.parts {
        let count = (part.vertex_start - output_cursor) * stride;
        write_changed_staged(
            device,
            uploads,
            &gpu.vertex_buffer,
            (output_cursor * stride) as u64,
            &bytes[source_cursor..source_cursor + count],
            previous
                .get(source_cursor..source_cursor + count)
                .unwrap_or(&[]),
        );
        source_cursor += count;
        output_cursor = part.vertex_start + part.vertex_count;
    }
    write_changed_staged(
        device,
        uploads,
        &gpu.vertex_buffer,
        (output_cursor * stride) as u64,
        &bytes[source_cursor..],
        previous.get(source_cursor..).unwrap_or(&[]),
    );
    write_changed_staged(
        device,
        uploads,
        &gpu.index_buffer,
        0,
        index_bytes,
        bytemuck::cast_slice(&gpu.indices),
    );
    geometry.expand(
        device,
        uploads,
        &gpu.vertex_buffer,
        &mut gpu.part_bindings,
        &mesh.parts,
        &gpu.parts,
    );
    gpu.vertex_bounds =
        TerrainBounds::from_points(mesh.vertices.iter().map(|v| Vec3::from_array(v.position)));
    gpu.index_count = mesh.indices.len() as u32;
    gpu.vertex_bytes.clear();
    gpu.vertex_bytes.extend_from_slice(bytes);
    std::mem::swap(&mut gpu.indices, &mut mesh.indices);
    std::mem::swap(&mut gpu.parts, &mut mesh.parts);
    mesh
}

pub(crate) fn upload_elder_guardian_particle_textured_mesh(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging: &mut UploadBelt,
    vertex_buffer: &mut FrameDataBuffer,
    index_buffer: &mut FrameDataBuffer,
    instances: &[ElderGuardianParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> Option<u32> {
    let mut meshes = elder_guardian_particle_textured_meshes(instances, atlas);
    let mesh = std::mem::take(&mut meshes.translucent);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let index_count = u32::try_from(mesh.indices.len()).expect("particle index count fits in u32");
    let vertices_uploaded = vertex_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.vertices),
    );
    let indices_uploaded = index_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.indices),
    );
    (vertices_uploaded && indices_uploaded).then_some(index_count)
}

pub(crate) fn upload_projectile_pickup_particle_textured_mesh(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging: &mut UploadBelt,
    vertex_buffer: &mut FrameDataBuffer,
    index_buffer: &mut FrameDataBuffer,
    instances: &[ProjectilePickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> Option<u32> {
    let mut meshes = projectile_pickup_particle_textured_meshes(instances, atlas);
    let mesh = std::mem::take(&mut meshes.translucent);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let index_count = u32::try_from(mesh.indices.len()).expect("particle index count fits in u32");
    let vertices_uploaded = vertex_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.vertices),
    );
    let indices_uploaded = index_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.indices),
    );
    (vertices_uploaded && indices_uploaded).then_some(index_count)
}

pub(crate) fn upload_experience_orb_pickup_particle_textured_mesh(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging: &mut UploadBelt,
    vertex_buffer: &mut FrameDataBuffer,
    index_buffer: &mut FrameDataBuffer,
    instances: &[ExperienceOrbPickupParticleRenderInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> Option<u32> {
    let mesh = experience_orb_pickup_particle_textured_mesh(instances, atlas);
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }
    let index_count = u32::try_from(mesh.indices.len()).expect("particle index count fits in u32");
    let vertices_uploaded = vertex_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.vertices),
    );
    let indices_uploaded = index_buffer.upload_staged(
        device,
        encoder,
        staging,
        bytemuck::cast_slice(&mesh.indices),
    );
    (vertices_uploaded && indices_uploaded).then_some(index_count)
}
