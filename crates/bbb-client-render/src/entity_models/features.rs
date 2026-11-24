//! Dispatcher-owned generic entity features shared by every concrete renderer.
//!
//! Flame surfaces use the resident blocks atlas and are rendered in the solid
//! feature phase after entity models. Shadow surfaces use their dedicated
//! texture and lead the translucent feature phase. Both are submitted by
//! `EntityRenderDispatcher`, not by a concrete entity model.

use std::{borrow::Borrow, collections::HashMap, sync::OnceLock};

use anyhow::Result;
use glam::{Mat4, Vec3};

use crate::item_models::{ItemModelMesh, ItemModelQuad, ItemModelVertex};
use crate::item_render::item_models::display_text::WorldTextBuffers;
use crate::terrain::TerrainUvRect;
use crate::{
    texture_sampling::{
        create_camera_texture_bind_group, standalone_sampler_descriptor, upload_rgba_texture,
        validate_rgba,
    },
    CameraPose, Renderer, StandaloneTextureSampling,
};

use super::{entity_dispatch_render_offset, EntityModelInstance};

pub const ENTITY_FLAME_FIRE_0_SPRITE: &str = "minecraft:block/fire_0";
pub const ENTITY_FLAME_FIRE_1_SPRITE: &str = "minecraft:block/fire_1";
pub const ENTITY_SHADOW_TEXTURE_PATH: &str = "textures/misc/shadow.png";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityShadowPiece {
    pub relative_position: [f32; 3],
    pub shape_min: [f32; 3],
    pub shape_max: [f32; 3],
    pub alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityShadowSubmission {
    pub entity_id: i32,
    pub root_position: [f32; 3],
    pub radius: f32,
    pub piece_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityShadowSurface {
    pub submission: EntityShadowSubmission,
    mesh: ItemModelMesh,
}

impl EntityShadowSurface {
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.index_count()
    }

    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }
}

pub(crate) struct EntityShadowTextureGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl EntityShadowTextureGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._texture
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityFlameSubmission {
    pub entity_id: i32,
    pub root_position: [f32; 3],
    pub dimensions: [f32; 2],
    pub layer_count: u32,
    camera_y_rot: f32,
    light: [f32; 2],
    sprites: [TerrainUvRect; 2],
}

#[derive(Debug, Clone)]
pub struct EntityFlameSurface {
    pub submission: EntityFlameSubmission,
    mesh: OnceLock<ItemModelMesh>,
}

impl PartialEq for EntityFlameSurface {
    fn eq(&self, other: &Self) -> bool {
        self.submission == other.submission
    }
}

impl EntityFlameSurface {
    pub fn vertex_count(&self) -> usize {
        self.submission.layer_count as usize * 4
    }

    pub fn index_count(&self) -> usize {
        self.submission.layer_count as usize * 6
    }

    pub fn is_empty(&self) -> bool {
        self.submission.layer_count == 0
    }

    fn mesh(&self) -> &ItemModelMesh {
        self.mesh
            .get_or_init(|| mesh_entity_flame(&self.submission))
    }
}

const MAX_ENTITY_FLAME_LAYERS: u32 = 64;
const MAX_ENTITY_FLAME_HEIGHT_RATIO: f32 = 10_000.0;

/// Bakes Vanilla 26.1 `FlameFeatureRenderer.renderFlame` for one already
/// visible entity. The dispatcher render offset is part of the captured pose;
/// the feature then keeps only the camera orientation's Y-axis rotation.
pub fn bake_entity_flame_surface(
    instance: EntityModelInstance,
    camera: CameraPose,
    fire_0: TerrainUvRect,
    fire_1: TerrainUvRect,
) -> Option<EntityFlameSurface> {
    let [width, height] = instance.render_state.flame_dimensions?;
    let scale = width * 1.4;
    if !width.is_finite() || !height.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let remaining_height = height / scale;
    if !remaining_height.is_finite()
        || remaining_height <= 0.0
        || remaining_height > MAX_ENTITY_FLAME_HEIGHT_RATIO
    {
        return None;
    }
    let layer_count = ((remaining_height / 0.45).ceil() as u32).clamp(1, MAX_ENTITY_FLAME_LAYERS);
    let offset = entity_dispatch_render_offset(&instance);
    Some(EntityFlameSurface {
        submission: EntityFlameSubmission {
            entity_id: instance.entity_id,
            root_position: [
                instance.position[0] + offset[0],
                instance.position[1] + offset[1],
                instance.position[2] + offset[2],
            ],
            dimensions: [width, height],
            layer_count,
            camera_y_rot: camera.y_rot,
            light: flame_shader_light(instance.render_state.light_coords),
            sprites: [fire_0, fire_1],
        },
        mesh: OnceLock::new(),
    })
}

fn mesh_entity_flame(submission: &EntityFlameSubmission) -> ItemModelMesh {
    let [width, height] = submission.dimensions;
    let scale = width * 1.4;
    let relative_height = (height / scale).min(MAX_ENTITY_FLAME_LAYERS as f32 * 0.45);
    let transform = Mat4::from_translation(Vec3::from_array(submission.root_position))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_rotation_y((180.0 - submission.camera_y_rot).to_radians())
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.0,
            0.3 - (relative_height as i32) as f32 * 0.02,
        ));
    let mut mesh = ItemModelMesh::new();
    let mut radius = 0.5;
    let mut y_offset = 0.0;
    let mut z_offset = 0.0;
    for layer in 0..submission.layer_count {
        let sprite = submission.sprites[(layer % 2) as usize];
        let (mut u0, mut u1) = (sprite.min[0], sprite.max[0]);
        if layer / 2 % 2 == 0 {
            std::mem::swap(&mut u0, &mut u1);
        }
        let (v0, v1) = (sprite.min[1], sprite.max[1]);
        let quad = ItemModelQuad {
            corners: [
                flame_corner(-radius, -y_offset, z_offset),
                flame_corner(radius, -y_offset, z_offset),
                flame_corner(radius, 1.4 - y_offset, z_offset),
                flame_corner(-radius, 1.4 - y_offset, z_offset),
            ],
            uvs: [[u1, v1], [u0, v1], [u0, v0], [u1, v0]],
            tint: [1.0; 4],
            normal: [0.0, 1.0, 0.0],
            shade: 1.0,
            translucent: false,
        };
        mesh.append_quads_with_light(std::slice::from_ref(&quad), transform, submission.light);
        y_offset -= 0.45;
        radius *= 0.9;
        z_offset -= 0.03;
    }
    mesh
}

pub(crate) struct EntityFeatureMesh {
    dirty: bool,
    geometry: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) buffers: WorldTextBuffers,
}

impl Default for EntityFeatureMesh {
    fn default() -> Self {
        Self {
            dirty: true,
            geometry: (Vec::new(), Vec::new()),
            buffers: WorldTextBuffers::default(),
        }
    }
}

impl EntityFeatureMesh {
    pub(crate) fn restore_unsubmitted(&mut self) {
        self.geometry = (Vec::new(), Vec::new());
        self.dirty = true;
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        geometry: (Vec<ItemModelVertex>, Vec<u32>),
    ) {
        self.buffers.vertices.upload_changed_staged(
            device,
            uploads,
            bytemuck::cast_slice(&geometry.0),
            bytemuck::cast_slice(&self.geometry.0),
        );
        if self.geometry.1 != geometry.1 || self.buffers.index_count != geometry.1.len() as u32 {
            self.buffers.indices.upload_staged_commands(
                device,
                uploads,
                bytemuck::cast_slice(&geometry.1),
            );
            self.buffers.index_count = geometry.1.len() as u32;
        }
        self.geometry = geometry;
        self.dirty = false;
    }
}

/// Bakes one vanilla `ShadowSubmit`. Avatar render offsets are canceled before shadow submission;
/// other concrete renderer offsets remain captured in the dispatcher pose.
pub fn bake_entity_shadow_surface(
    instance: EntityModelInstance,
    origin: [f32; 3],
    radius: f32,
    pieces: impl IntoIterator<Item = impl Borrow<EntityShadowPiece>>,
) -> Option<EntityShadowSurface> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let root_position = if matches!(instance.kind, super::EntityModelKind::Player { .. }) {
        origin
    } else {
        let offset = entity_dispatch_render_offset(&instance);
        [
            instance.position[0] + offset[0],
            instance.position[1] + offset[1],
            instance.position[2] + offset[2],
        ]
    };
    let transform = Mat4::from_translation(Vec3::from_array(root_position));
    let mut mesh = ItemModelMesh::new();
    let mut piece_count = 0_u32;
    for piece in pieces {
        let piece = piece.borrow();
        if !piece.alpha.is_finite() || piece.alpha <= 0.0 {
            continue;
        }
        let x0 = piece.relative_position[0] + piece.shape_min[0];
        let x1 = piece.relative_position[0] + piece.shape_max[0];
        let y = piece.relative_position[1] + piece.shape_min[1];
        let z0 = piece.relative_position[2] + piece.shape_min[2];
        let z1 = piece.relative_position[2] + piece.shape_max[2];
        let u0 = -x0 / (2.0 * radius) + 0.5;
        let u1 = -x1 / (2.0 * radius) + 0.5;
        let v0 = -z0 / (2.0 * radius) + 0.5;
        let v1 = -z1 / (2.0 * radius) + 0.5;
        let quad = ItemModelQuad {
            corners: [
                [x0 * 16.0, y * 16.0, z0 * 16.0],
                [x0 * 16.0, y * 16.0, z1 * 16.0],
                [x1 * 16.0, y * 16.0, z1 * 16.0],
                [x1 * 16.0, y * 16.0, z0 * 16.0],
            ],
            uvs: [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
            tint: [1.0, 1.0, 1.0, vanilla_argb_channel(piece.alpha)],
            normal: [0.0, 1.0, 0.0],
            shade: 1.0,
            translucent: true,
        };
        mesh.append_quads(&[quad], transform);
        piece_count += 1;
    }
    (!mesh.is_empty()).then_some(EntityShadowSurface {
        submission: EntityShadowSubmission {
            entity_id: instance.entity_id,
            root_position,
            radius,
            piece_count,
        },
        mesh,
    })
}

fn vanilla_argb_channel(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) * 255.0).floor() / 255.0
}

fn flame_corner(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x * 16.0, y * 16.0, z * 16.0]
}

fn flame_shader_light(light_coords: u32) -> [f32; 2] {
    [1.0, ((light_coords >> 20) & 0xF) as f32 / 15.0]
}

pub(crate) fn merge_entity_flame_surfaces(
    surfaces: &[EntityFlameSurface],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let vertex_count = surfaces.iter().map(EntityFlameSurface::vertex_count).sum();
    let index_count = surfaces.iter().map(EntityFlameSurface::index_count).sum();
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(index_count);
    for surface in surfaces {
        let base = u32::try_from(vertices.len()).expect("entity flame vertex count fits in u32");
        let mesh = surface.mesh();
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|index| base + index));
    }
    (vertices, indices)
}

pub(crate) fn merge_entity_shadow_surfaces(
    surfaces: &[EntityShadowSurface],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let vertex_count = surfaces.iter().map(EntityShadowSurface::vertex_count).sum();
    let index_count = surfaces.iter().map(EntityShadowSurface::index_count).sum();
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(index_count);
    for surface in surfaces {
        let base = u32::try_from(vertices.len()).expect("entity shadow vertex count fits in u32");
        vertices.extend_from_slice(&surface.mesh.vertices);
        indices.extend(surface.mesh.indices.iter().map(|index| base + index));
    }
    (vertices, indices)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_entity_shadow_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::BufferBinding<'_>,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<EntityShadowTextureGpu> {
    validate_rgba(width, height, rgba)?;
    let (texture, view) = upload_rgba_texture(
        device,
        queue,
        [width, height],
        rgba,
        "bbb-entity-shadow-texture",
    )?;
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-entity-shadow-sampler",
        sampling,
    ));
    let bind_group = create_camera_texture_bind_group(
        device,
        layout,
        "bbb-entity-shadow-bind-group",
        camera_buffer,
        &view,
        &sampler,
    );
    Ok(EntityShadowTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

impl Renderer {
    pub fn set_entity_flame_surfaces(&mut self, surfaces: Vec<EntityFlameSurface>) {
        let mut surfaces: Vec<_> = surfaces
            .into_iter()
            .filter(|surface| !surface.is_empty())
            .collect();
        if self.entity_flame_surfaces == surfaces {
            return;
        }
        let mut previous = std::mem::take(&mut self.entity_flame_surfaces)
            .into_iter()
            .map(|surface| (surface.submission.entity_id, surface))
            .collect::<HashMap<_, _>>();
        for surface in &mut surfaces {
            if let Some(previous) = previous.get_mut(&surface.submission.entity_id) {
                if surface.submission == previous.submission {
                    surface.mesh = std::mem::take(&mut previous.mesh);
                }
            }
        }
        self.entity_flame_surfaces = surfaces;
        self.pack.entity_flames.dirty = true;
    }

    pub(crate) fn prepare_entity_flames(&mut self) {
        let state = &mut self.pack.entity_flames;
        if !state.dirty {
            return;
        }
        let geometry = merge_entity_flame_surfaces(&self.entity_flame_surfaces);
        state.upload(&self.device, &mut self.frame_uploads, geometry);
    }

    pub fn set_entity_shadow_surfaces(&mut self, surfaces: Vec<EntityShadowSurface>) {
        if self.entity_shadow_surfaces != surfaces {
            self.entity_shadow_surfaces = surfaces;
            self.pack.entity_shadows.dirty = true;
        }
    }

    pub fn upload_entity_shadow_texture(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.upload_entity_shadow_texture_with_sampling(
            width,
            height,
            rgba,
            StandaloneTextureSampling::new(false, true),
        )
    }

    pub fn upload_entity_shadow_texture_with_sampling(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        sampling: StandaloneTextureSampling,
    ) -> Result<()> {
        self.pack.entity_shadow_texture = Some(create_entity_shadow_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffers.world(),
            width,
            height,
            rgba,
            sampling,
        )?);
        Ok(())
    }

    pub(crate) fn prepare_entity_shadows(&mut self) {
        let state = &mut self.pack.entity_shadows;
        if state.dirty && self.pack.entity_shadow_texture.is_some() {
            let geometry = merge_entity_shadow_surfaces(&self.entity_shadow_surfaces);
            state.upload(&self.device, &mut self.frame_uploads, geometry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntityModelKind, ENTITY_FULL_BRIGHT_LIGHT_COORDS};

    fn camera() -> CameraPose {
        CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 35.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        }
    }

    #[test]
    fn flame_layers_match_vanilla_width_height_shrink_and_uv_alternation() {
        let instance = EntityModelInstance::new(7, EntityModelKind::NoRender, [2.0, 4.0, 6.0], 0.0)
            .with(|s| {
                s.light_coords = 4 << 20;
                s.flame_dimensions = Some([1.0, 2.0]);
            });
        let fire_0 = TerrainUvRect {
            min: [0.1, 0.2],
            max: [0.3, 0.4],
        };
        let fire_1 = TerrainUvRect {
            min: [0.5, 0.6],
            max: [0.7, 0.8],
        };
        let surface = bake_entity_flame_surface(instance, camera(), fire_0, fire_1).unwrap();

        // h=2/1.4 => layers at 1.428, .978, .528, .078.
        assert_eq!(surface.submission.layer_count, 4);
        assert_eq!(surface.vertex_count(), 16);
        assert_eq!(surface.index_count(), 24);
        assert_eq!(
            surface.mesh().vertices[0].uv,
            [fire_0.min[0], fire_0.max[1]]
        );
        assert_eq!(
            surface.mesh().vertices[4].uv,
            [fire_1.min[0], fire_1.max[1]]
        );
        assert_eq!(
            surface.mesh().vertices[8].uv,
            [fire_0.max[0], fire_0.max[1]]
        );
        assert_eq!(surface.mesh().vertices[0].light, [1.0, 4.0 / 15.0]);
        assert_eq!(surface.mesh().vertices[0].overlay, [0.0, 10.0]);
    }

    #[test]
    fn flame_uses_dispatcher_root_and_camera_y_only_rotation() {
        let instance = EntityModelInstance::new(8, EntityModelKind::NoRender, [3.0, 5.0, 7.0], 0.0)
            .with(|s| {
                s.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
                s.flame_dimensions = Some([0.5, 0.5]);
            });
        let tilted =
            bake_entity_flame_surface(instance, camera(), TerrainUvRect::UNIT, TerrainUvRect::UNIT)
                .unwrap();
        let mut level_camera = camera();
        level_camera.x_rot = 0.0;
        let level = bake_entity_flame_surface(
            instance,
            level_camera,
            TerrainUvRect::UNIT,
            TerrainUvRect::UNIT,
        )
        .unwrap();

        assert_eq!(tilted.submission.root_position, [3.0, 5.0, 7.0]);
        assert_eq!(
            tilted.mesh(),
            level.mesh(),
            "camera pitch must not tilt flames"
        );
    }

    #[test]
    fn flame_rejects_absent_or_degenerate_dimensions() {
        let base = EntityModelInstance::new(9, EntityModelKind::NoRender, [0.0; 3], 0.0);
        assert!(bake_entity_flame_surface(
            base,
            camera(),
            TerrainUvRect::UNIT,
            TerrainUvRect::UNIT
        )
        .is_none());
        assert!(bake_entity_flame_surface(
            base.with(|s| s.flame_dimensions = Some([0.0, 1.0])),
            camera(),
            TerrainUvRect::UNIT,
            TerrainUvRect::UNIT,
        )
        .is_none());
    }

    #[test]
    fn shadow_quad_matches_vanilla_uv_alpha_and_dispatch_root() {
        let instance =
            EntityModelInstance::new(10, EntityModelKind::NoRender, [3.0, 5.0, 7.0], 0.0);
        let surface = bake_entity_shadow_surface(
            instance,
            [2.0, 4.0, 6.0],
            2.0,
            [EntityShadowPiece {
                relative_position: [-1.0, -0.25, 0.0],
                shape_min: [0.0; 3],
                shape_max: [1.0; 3],
                alpha: 0.5,
            }],
        )
        .unwrap();

        assert_eq!(surface.submission.root_position, [3.0, 5.0, 7.0]);
        assert_eq!(surface.submission.piece_count, 1);
        assert_eq!(surface.mesh.vertices[0].position, [2.0, 4.75, 7.0]);
        assert_eq!(surface.mesh.vertices[0].uv, [0.75, 0.5]);
        assert_eq!(surface.mesh.vertices[2].uv, [0.5, 0.25]);
        assert_eq!(surface.mesh.vertices[0].color[3], 127.0 / 255.0);
    }

    #[test]
    fn avatar_shadow_cancels_crouch_dispatch_offset() {
        let instance = EntityModelInstance::player(11, [9.0, 8.0, 7.0], 0.0, false)
            .with(|s| s.is_crouching = true);
        let surface = bake_entity_shadow_surface(
            instance,
            [1.0, 2.0, 3.0],
            0.5,
            [EntityShadowPiece {
                relative_position: [0.0; 3],
                shape_min: [0.0; 3],
                shape_max: [1.0; 3],
                alpha: 1.0,
            }],
        )
        .unwrap();

        assert_eq!(surface.submission.root_position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn entity_flame_rejects_degenerate_aspect_and_caps_layer_count() {
        let uv = TerrainUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        };
        let camera = camera();
        // R8-03: tiny width / huge height ratio previously caused IEEE-754 infinite loop or OOM.
        let degenerate = EntityModelInstance::new(12, EntityModelKind::NoRender, [0.0; 3], 0.0)
            .with(|s| s.flame_dimensions = Some([1e-30, 100.0]));
        assert!(bake_entity_flame_surface(degenerate, camera, uv, uv).is_none());

        let tall = EntityModelInstance::new(13, EntityModelKind::NoRender, [0.0; 3], 0.0)
            .with(|s| s.flame_dimensions = Some([0.05, 100.0]));
        let surface = bake_entity_flame_surface(tall, camera, uv, uv).expect("capped flame");
        assert_eq!(surface.submission.layer_count, MAX_ENTITY_FLAME_LAYERS);
        assert!(!surface.mesh().vertices.is_empty());
    }
}
