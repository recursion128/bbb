//! Dispatcher-owned generic entity features shared by every concrete renderer.
//!
//! Flame surfaces use the resident blocks atlas and are rendered in the solid
//! feature phase after entity models. Shadow surfaces use their dedicated
//! texture and lead the translucent feature phase. Both are submitted by
//! `EntityRenderDispatcher`, not by a concrete entity model.

use anyhow::{ensure, Result};
use glam::{Mat4, Vec3};

use crate::item_models::{ItemModelMesh, ItemModelQuad, ItemModelVertex};
use crate::terrain::TerrainUvRect;
use crate::{
    texture_sampling::standalone_sampler_descriptor, CameraPose, Renderer,
    StandaloneTextureSampling,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityFlameSubmission {
    pub entity_id: i32,
    pub root_position: [f32; 3],
    pub dimensions: [f32; 2],
    pub layer_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityFlameSurface {
    pub submission: EntityFlameSubmission,
    mesh: ItemModelMesh,
}

impl EntityFlameSurface {
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
    let mut remaining_height = height / scale;
    if !remaining_height.is_finite() || remaining_height <= 0.0 {
        return None;
    }

    let offset = entity_dispatch_render_offset(instance);
    let root_position = [
        instance.position[0] + offset[0],
        instance.position[1] + offset[1],
        instance.position[2] + offset[2],
    ];
    let transform = Mat4::from_translation(Vec3::from_array(root_position))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_rotation_y((180.0 - camera.y_rot).to_radians())
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.0,
            0.3 - (remaining_height as i32) as f32 * 0.02,
        ));

    let mut mesh = ItemModelMesh::new();
    let mut radius = 0.5;
    let mut y_offset = 0.0;
    let mut z_offset = 0.0;
    let mut layer = 0_u32;
    let light = flame_shader_light(instance.render_state.light_coords);
    while remaining_height > 0.0 {
        let sprite = if layer % 2 == 0 { fire_0 } else { fire_1 };
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
        mesh.append_quads_with_light(std::slice::from_ref(&quad), transform, light);
        remaining_height -= 0.45;
        y_offset -= 0.45;
        radius *= 0.9;
        z_offset -= 0.03;
        layer += 1;
    }

    Some(EntityFlameSurface {
        submission: EntityFlameSubmission {
            entity_id: instance.entity_id,
            root_position,
            dimensions: [width, height],
            layer_count: layer,
        },
        mesh,
    })
}

/// Bakes one vanilla `ShadowSubmit`. Avatar render offsets are canceled before shadow submission;
/// other concrete renderer offsets remain captured in the dispatcher pose.
pub fn bake_entity_shadow_surface(
    instance: EntityModelInstance,
    origin: [f32; 3],
    radius: f32,
    pieces: &[EntityShadowPiece],
) -> Option<EntityShadowSurface> {
    if !radius.is_finite() || radius <= 0.0 || pieces.is_empty() {
        return None;
    }
    let root_position = if matches!(instance.kind, super::EntityModelKind::Player { .. }) {
        origin
    } else {
        let offset = entity_dispatch_render_offset(instance);
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
        vertices.extend_from_slice(&surface.mesh.vertices);
        indices.extend(surface.mesh.indices.iter().map(|index| base + index));
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

pub(crate) fn create_entity_shadow_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<EntityShadowTextureGpu> {
    ensure!(width > 0 && height > 0, "entity shadow texture is empty");
    ensure!(
        rgba.len() == width as usize * height as usize * 4,
        "entity shadow texture RGBA length mismatch"
    );
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-entity-shadow-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&standalone_sampler_descriptor(
        "bbb-entity-shadow-sampler",
        sampling,
    ));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-entity-shadow-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    Ok(EntityShadowTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

impl Renderer {
    pub fn set_entity_flame_surfaces(&mut self, surfaces: Vec<EntityFlameSurface>) {
        self.entity_flame_surfaces = surfaces
            .into_iter()
            .filter(|surface| !surface.is_empty())
            .collect();
    }

    pub(crate) fn collect_entity_flame_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_entity_flame_surfaces(&self.entity_flame_surfaces)
    }

    pub fn set_entity_shadow_surfaces(&mut self, surfaces: Vec<EntityShadowSurface>) {
        self.entity_shadow_surfaces = surfaces
            .into_iter()
            .filter(|surface| !surface.is_empty())
            .collect();
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
        self.entity_shadow_texture = Some(create_entity_shadow_texture_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
            sampling,
        )?);
        Ok(())
    }

    pub(crate) fn collect_entity_shadow_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_entity_shadow_surfaces(&self.entity_shadow_surfaces)
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
            .with_light_coords(4 << 20)
            .with_flame_dimensions(Some([1.0, 2.0]));
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
        assert_eq!(surface.mesh.vertices[0].uv, [fire_0.min[0], fire_0.max[1]]);
        assert_eq!(surface.mesh.vertices[4].uv, [fire_1.min[0], fire_1.max[1]]);
        assert_eq!(surface.mesh.vertices[8].uv, [fire_0.max[0], fire_0.max[1]]);
        assert_eq!(surface.mesh.vertices[0].light, [1.0, 4.0 / 15.0]);
        assert_eq!(surface.mesh.vertices[0].overlay, [0.0, 10.0]);
    }

    #[test]
    fn flame_uses_dispatcher_root_and_camera_y_only_rotation() {
        let instance = EntityModelInstance::new(8, EntityModelKind::NoRender, [3.0, 5.0, 7.0], 0.0)
            .with_light_coords(ENTITY_FULL_BRIGHT_LIGHT_COORDS)
            .with_flame_dimensions(Some([0.5, 0.5]));
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
        assert_eq!(tilted.mesh, level.mesh, "camera pitch must not tilt flames");
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
            base.with_flame_dimensions(Some([0.0, 1.0])),
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
            &[EntityShadowPiece {
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
        let instance =
            EntityModelInstance::player(11, [9.0, 8.0, 7.0], 0.0, false).with_is_crouching(true);
        let surface = bake_entity_shadow_surface(
            instance,
            [1.0, 2.0, 3.0],
            0.5,
            &[EntityShadowPiece {
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
    fn main_pass_orders_flame_between_entity_models_and_item_features() {
        let source = include_str!("../render.rs");
        let model = source
            .find("&self.entity_model_entity_glint_mesh")
            .expect("last entity-model feature");
        let flame = source
            .find("self.collect_entity_flame_geometry()")
            .expect("flame feature");
        let item = source
            .find("self.collect_block_item_model_geometry()")
            .expect("block item feature");
        assert!(model < flame && flame < item);
    }

    #[test]
    fn translucent_feature_phase_draws_shadow_before_entity_models() {
        let source = include_str!("../render.rs");
        let phase = source
            .find("fn entity_translucent_feature_pass")
            .expect("translucent feature pass");
        let shadow = source[phase..]
            .find("self.collect_entity_shadow_geometry()")
            .expect("shadow feature");
        let model = source[phase..]
            .find("self.draw_entity_translucent_features(")
            .expect("translucent entity models");
        assert!(shadow < model);
    }
}
