//! Dispatcher-owned generic entity features shared by every concrete renderer.
//!
//! Flame surfaces use the resident blocks atlas and are rendered in the solid
//! feature phase after entity models. Shadow surfaces join this owner in the
//! following stage because both are submitted by `EntityRenderDispatcher`, not
//! by a concrete entity model.

use glam::{Mat4, Vec3};

use crate::item_models::{ItemModelMesh, ItemModelQuad, ItemModelVertex};
use crate::terrain::TerrainUvRect;
use crate::{CameraPose, Renderer};

use super::{entity_dispatch_render_offset, EntityModelInstance};

pub const ENTITY_FLAME_FIRE_0_SPRITE: &str = "minecraft:block/fire_0";
pub const ENTITY_FLAME_FIRE_1_SPRITE: &str = "minecraft:block/fire_1";

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
}
