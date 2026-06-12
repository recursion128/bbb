use std::{mem, path::Path, sync::mpsc};

use anyhow::{anyhow, bail, Context, Result};
use bbb_control::RendererCounters;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

pub mod terrain {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TerrainMaterialClass {
        Empty,
        Opaque,
        Cutout,
        Fluid,
        Translucent,
    }

    impl TerrainMaterialClass {
        fn is_meshed_opaque(self) -> bool {
            matches!(self, Self::Opaque)
        }

        fn is_meshed_cutout(self) -> bool {
            matches!(self, Self::Cutout)
        }

        fn is_meshed_translucent(self) -> bool {
            matches!(self, Self::Fluid | Self::Translucent)
        }

        fn is_meshed_terrain(self) -> bool {
            self.is_meshed_opaque() || self.is_meshed_cutout() || self.is_meshed_translucent()
        }

        fn occludes_opaque(self) -> bool {
            matches!(self, Self::Opaque)
        }

        fn occludes_terrain(self) -> bool {
            matches!(self, Self::Opaque)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TerrainRenderShape {
        Cube,
        Cross,
        Box {
            from: [u8; 3],
            to: [u8; 3],
            face_present: [bool; 6],
            face_uvs: [[u8; 4]; 6],
            face_cull: [bool; 6],
        },
        Boxes(Vec<TerrainBox>),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerrainBox {
        pub from: [u8; 3],
        pub to: [u8; 3],
        pub face_present: [bool; 6],
        pub face_uvs: [[u8; 4]; 6],
        pub face_cull: [bool; 6],
        pub texture_indices: [u32; 6],
        pub tint: [TerrainTint; 6],
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerrainCell {
        pub block_state_id: i32,
        pub material: TerrainMaterialClass,
        pub texture_indices: [u32; 6],
        pub render_shape: TerrainRenderShape,
        pub light: TerrainLight,
        pub tint: [TerrainTint; 6],
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerrainLight {
        pub sky: u8,
        pub block: u8,
    }

    impl TerrainLight {
        pub const FULL_BRIGHT: Self = Self { sky: 15, block: 0 };

        fn as_shader_light(self) -> [f32; 2] {
            [
                self.block.min(15) as f32 / 15.0,
                self.sky.min(15) as f32 / 15.0,
            ]
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerrainTint {
        pub r: u8,
        pub g: u8,
        pub b: u8,
    }

    impl TerrainTint {
        pub const WHITE: Self = Self {
            r: 255,
            g: 255,
            b: 255,
        };

        pub fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
            Self { r, g, b }
        }

        fn as_shader_tint(self) -> [f32; 3] {
            [
                self.r as f32 / 255.0,
                self.g as f32 / 255.0,
                self.b as f32 / 255.0,
            ]
        }
    }

    impl TerrainCell {
        pub const EMPTY: Self = Self {
            block_state_id: 0,
            material: TerrainMaterialClass::Empty,
            texture_indices: [0; 6],
            render_shape: TerrainRenderShape::Cube,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
        };

        pub fn with_texture(
            block_state_id: i32,
            material: TerrainMaterialClass,
            texture_index: u32,
        ) -> Self {
            Self {
                block_state_id,
                material,
                texture_indices: [texture_index; 6],
                render_shape: TerrainRenderShape::Cube,
                light: TerrainLight::FULL_BRIGHT,
                tint: [TerrainTint::WHITE; 6],
            }
        }

        pub fn with_shape(
            block_state_id: i32,
            material: TerrainMaterialClass,
            texture_index: u32,
            render_shape: TerrainRenderShape,
        ) -> Self {
            Self {
                block_state_id,
                material,
                texture_indices: [texture_index; 6],
                render_shape,
                light: TerrainLight::FULL_BRIGHT,
                tint: [TerrainTint::WHITE; 6],
            }
        }

        pub fn with_light(mut self, light: TerrainLight) -> Self {
            self.light = light;
            self
        }

        pub fn with_tint(mut self, tint: [TerrainTint; 6]) -> Self {
            self.tint = tint;
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerrainChunkSnapshot {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub min_y: i32,
        pub height: usize,
        pub cells: Vec<TerrainCell>,
    }

    impl TerrainChunkSnapshot {
        pub fn new(
            chunk_x: i32,
            chunk_z: i32,
            min_y: i32,
            height: usize,
            cells: Vec<TerrainCell>,
        ) -> Self {
            assert_eq!(cells.len(), 16 * height * 16);
            Self {
                chunk_x,
                chunk_z,
                min_y,
                height,
                cells,
            }
        }

        pub fn cell(&self, x: i32, y: i32, z: i32) -> Option<&TerrainCell> {
            if !(0..16).contains(&x)
                || !(0..self.height as i32).contains(&y)
                || !(0..16).contains(&z)
            {
                return None;
            }
            self.cells
                .get(cell_index(x as usize, y as usize, z as usize, self.height))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TerrainFace {
        Down,
        Up,
        North,
        South,
        West,
        East,
    }

    impl TerrainFace {
        fn index(self) -> usize {
            match self {
                Self::Down => 0,
                Self::Up => 1,
                Self::North => 2,
                Self::South => 3,
                Self::West => 4,
                Self::East => 5,
            }
        }
    }

    #[repr(C)]
    #[derive(
        Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
    )]
    pub struct TerrainVertex {
        pub position: [f32; 3],
        pub normal: [f32; 3],
        pub uv: [f32; 2],
        pub light: [f32; 2],
        pub tint: [f32; 3],
        pub block_state_id: i32,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct TerrainMesh {
        pub vertices: Vec<TerrainVertex>,
        pub indices: Vec<u32>,
        pub source_sections: usize,
        pub opaque_faces: usize,
        pub cutout_faces: usize,
        pub translucent_faces: usize,
        pub culled_faces: usize,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct TerrainMeshLayers {
        pub opaque: Vec<TerrainMesh>,
        pub cutout: Vec<TerrainMesh>,
        pub translucent: Vec<TerrainMesh>,
        pub source_sections: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct TerrainUvRect {
        pub min: [f32; 2],
        pub max: [f32; 2],
    }

    impl TerrainUvRect {
        pub const UNIT: Self = Self {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        };

        fn map(self, uv: [f32; 2]) -> [f32; 2] {
            [
                self.min[0] + (self.max[0] - self.min[0]) * uv[0],
                self.min[1] + (self.max[1] - self.min[1]) * uv[1],
            ]
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct TerrainTextureAtlas {
        pub rects: Vec<TerrainUvRect>,
        pub fallback_index: u32,
    }

    impl TerrainTextureAtlas {
        pub fn unit() -> Self {
            Self {
                rects: vec![TerrainUvRect::UNIT],
                fallback_index: 0,
            }
        }

        fn rect(&self, texture_index: u32) -> TerrainUvRect {
            self.rects
                .get(texture_index as usize)
                .copied()
                .or_else(|| self.rects.get(self.fallback_index as usize).copied())
                .unwrap_or(TerrainUvRect::UNIT)
        }
    }

    pub fn build_opaque_chunk_mesh(snapshot: &TerrainChunkSnapshot) -> TerrainMesh {
        build_opaque_terrain_meshes(std::slice::from_ref(snapshot))
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    pub fn build_opaque_terrain_meshes(snapshots: &[TerrainChunkSnapshot]) -> Vec<TerrainMesh> {
        let atlas = TerrainTextureAtlas::unit();
        build_opaque_terrain_meshes_with_atlas(snapshots, &atlas)
    }

    pub fn build_opaque_terrain_meshes_with_atlas(
        snapshots: &[TerrainChunkSnapshot],
        atlas: &TerrainTextureAtlas,
    ) -> Vec<TerrainMesh> {
        let lookup = TerrainChunkLookup::new(snapshots);
        snapshots
            .iter()
            .map(|snapshot| {
                build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::OpaqueOnly)
            })
            .collect()
    }

    pub fn build_terrain_meshes_with_atlas(
        snapshots: &[TerrainChunkSnapshot],
        atlas: &TerrainTextureAtlas,
    ) -> Vec<TerrainMesh> {
        let lookup = TerrainChunkLookup::new(snapshots);
        snapshots
            .iter()
            .map(|snapshot| {
                build_chunk_mesh_with_lookup(
                    snapshot,
                    &lookup,
                    atlas,
                    TerrainMeshMode::OpaqueCutout,
                )
            })
            .collect()
    }

    pub fn build_terrain_mesh_layers_with_atlas(
        snapshots: &[TerrainChunkSnapshot],
        atlas: &TerrainTextureAtlas,
    ) -> TerrainMeshLayers {
        let lookup = TerrainChunkLookup::new(snapshots);
        let source_sections = snapshots
            .iter()
            .map(|snapshot| snapshot.height.div_ceil(16))
            .sum();
        let opaque = snapshots
            .iter()
            .map(|snapshot| {
                build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::OpaqueOnly)
            })
            .collect();
        let cutout = snapshots
            .iter()
            .map(|snapshot| {
                build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::CutoutOnly)
            })
            .collect();
        let translucent = snapshots
            .iter()
            .map(|snapshot| {
                build_chunk_mesh_with_lookup(
                    snapshot,
                    &lookup,
                    atlas,
                    TerrainMeshMode::TranslucentOnly,
                )
            })
            .collect();

        TerrainMeshLayers {
            opaque,
            cutout,
            translucent,
            source_sections,
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum TerrainMeshMode {
        OpaqueOnly,
        CutoutOnly,
        TranslucentOnly,
        OpaqueCutout,
    }

    impl TerrainMeshMode {
        fn is_meshed(self, material: TerrainMaterialClass) -> bool {
            match self {
                Self::OpaqueOnly => material.is_meshed_opaque(),
                Self::CutoutOnly => material.is_meshed_cutout(),
                Self::TranslucentOnly => material.is_meshed_translucent(),
                Self::OpaqueCutout => material.is_meshed_terrain(),
            }
        }

        fn is_occluded_by(self, material: TerrainMaterialClass) -> bool {
            match self {
                Self::OpaqueOnly => material.occludes_opaque(),
                Self::CutoutOnly => material.occludes_terrain(),
                Self::TranslucentOnly => material.occludes_terrain(),
                Self::OpaqueCutout => material.occludes_terrain(),
            }
        }

        fn culls_face_between(
            self,
            current: TerrainMaterialClass,
            neighbor: TerrainMaterialClass,
        ) -> bool {
            self.is_occluded_by(neighbor)
                || (matches!(current, TerrainMaterialClass::Fluid)
                    && matches!(neighbor, TerrainMaterialClass::Fluid))
        }
    }

    fn build_chunk_mesh_with_lookup(
        snapshot: &TerrainChunkSnapshot,
        lookup: &TerrainChunkLookup<'_>,
        atlas: &TerrainTextureAtlas,
        mode: TerrainMeshMode,
    ) -> TerrainMesh {
        let mut mesh = TerrainMesh {
            source_sections: snapshot.height.div_ceil(16),
            ..TerrainMesh::default()
        };
        for y in 0..snapshot.height as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let cell = snapshot
                        .cell(x, y, z)
                        .expect("in-bounds terrain cell exists");
                    if !mode.is_meshed(cell.material) {
                        continue;
                    }

                    let world_x = snapshot.chunk_x * 16 + x;
                    let world_y = snapshot.min_y + y;
                    let world_z = snapshot.chunk_z * 16 + z;
                    match &cell.render_shape {
                        TerrainRenderShape::Cross => {
                            emit_cross(
                                &mut mesh,
                                world_x,
                                world_y,
                                world_z,
                                cell.block_state_id,
                                cell.material,
                                cell.light,
                                cell.tint,
                                cell.texture_indices,
                                atlas,
                            );
                            continue;
                        }
                        TerrainRenderShape::Box {
                            from,
                            to,
                            face_present,
                            face_uvs,
                            face_cull,
                        } => {
                            emit_box(
                                &mut mesh,
                                world_x,
                                world_y,
                                world_z,
                                cell.block_state_id,
                                cell.material,
                                cell.light,
                                cell.tint,
                                cell.texture_indices,
                                atlas,
                                *from,
                                *to,
                                *face_present,
                                *face_uvs,
                                *face_cull,
                                lookup,
                                mode,
                            );
                            continue;
                        }
                        TerrainRenderShape::Boxes(model_boxes) => {
                            for model_box in model_boxes {
                                emit_box(
                                    &mut mesh,
                                    world_x,
                                    world_y,
                                    world_z,
                                    cell.block_state_id,
                                    cell.material,
                                    cell.light,
                                    model_box.tint,
                                    model_box.texture_indices,
                                    atlas,
                                    model_box.from,
                                    model_box.to,
                                    model_box.face_present,
                                    model_box.face_uvs,
                                    model_box.face_cull,
                                    lookup,
                                    mode,
                                );
                            }
                            continue;
                        }
                        TerrainRenderShape::Cube => {}
                    }

                    for face in FACES {
                        let neighbor =
                            lookup.cell(world_x + face.dx, world_y + face.dy, world_z + face.dz);
                        if neighbor
                            .map(|neighbor| {
                                mode.culls_face_between(cell.material, neighbor.material)
                            })
                            .unwrap_or(false)
                        {
                            mesh.culled_faces += 1;
                            continue;
                        }
                        emit_face(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.light,
                            cell.tint[face.face.index()],
                            atlas.rect(cell.texture_indices[face.face.index()]),
                            face,
                        );
                    }
                }
            }
        }
        mesh
    }

    struct TerrainChunkLookup<'a> {
        chunks: HashMap<(i32, i32), &'a TerrainChunkSnapshot>,
    }

    impl<'a> TerrainChunkLookup<'a> {
        fn new(snapshots: &'a [TerrainChunkSnapshot]) -> Self {
            Self {
                chunks: snapshots
                    .iter()
                    .map(|snapshot| ((snapshot.chunk_x, snapshot.chunk_z), snapshot))
                    .collect(),
            }
        }

        fn cell(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<&TerrainCell> {
            let chunk_x = world_x.div_euclid(16);
            let chunk_z = world_z.div_euclid(16);
            let snapshot = self.chunks.get(&(chunk_x, chunk_z))?;
            snapshot.cell(
                world_x.rem_euclid(16),
                world_y - snapshot.min_y,
                world_z.rem_euclid(16),
            )
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct FaceDef {
        face: TerrainFace,
        normal: [f32; 3],
        dx: i32,
        dy: i32,
        dz: i32,
        corners: [[f32; 3]; 4],
    }

    const FACES: [FaceDef; 6] = [
        FaceDef {
            face: TerrainFace::Down,
            normal: [0.0, -1.0, 0.0],
            dx: 0,
            dy: -1,
            dz: 0,
            corners: [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        },
        FaceDef {
            face: TerrainFace::Up,
            normal: [0.0, 1.0, 0.0],
            dx: 0,
            dy: 1,
            dz: 0,
            corners: [
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
        },
        FaceDef {
            face: TerrainFace::North,
            normal: [0.0, 0.0, -1.0],
            dx: 0,
            dy: 0,
            dz: -1,
            corners: [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        },
        FaceDef {
            face: TerrainFace::South,
            normal: [0.0, 0.0, 1.0],
            dx: 0,
            dy: 0,
            dz: 1,
            corners: [
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
        },
        FaceDef {
            face: TerrainFace::West,
            normal: [-1.0, 0.0, 0.0],
            dx: -1,
            dy: 0,
            dz: 0,
            corners: [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
        },
        FaceDef {
            face: TerrainFace::East,
            normal: [1.0, 0.0, 0.0],
            dx: 1,
            dy: 0,
            dz: 0,
            corners: [
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
        },
    ];

    fn emit_face(
        mesh: &mut TerrainMesh,
        x: i32,
        y: i32,
        z: i32,
        block_state_id: i32,
        material: TerrainMaterialClass,
        light: TerrainLight,
        tint: TerrainTint,
        uv_rect: TerrainUvRect,
        face: FaceDef,
    ) {
        let base = mesh.vertices.len() as u32;
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for (corner, uv) in face.corners.into_iter().zip(uvs) {
            mesh.vertices.push(TerrainVertex {
                position: [
                    x as f32 + corner[0],
                    y as f32 + corner[1],
                    z as f32 + corner[2],
                ],
                normal: face.normal,
                uv: uv_rect.map(uv),
                light: light.as_shader_light(),
                tint: tint.as_shader_tint(),
                block_state_id,
            });
        }
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        match material {
            TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
            TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
            TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
                mesh.translucent_faces += 1
            }
            _ => {}
        }
    }

    fn emit_cross(
        mesh: &mut TerrainMesh,
        x: i32,
        y: i32,
        z: i32,
        block_state_id: i32,
        material: TerrainMaterialClass,
        light: TerrainLight,
        tint: [TerrainTint; 6],
        texture_indices: [u32; 6],
        atlas: &TerrainTextureAtlas,
    ) {
        const CROSS_FACES: [(TerrainFace, [f32; 3], [[f32; 3]; 4]); 4] = [
            (
                TerrainFace::North,
                [-0.70710677, 0.0, 0.70710677],
                [
                    [0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 1.0, 1.0],
                    [1.0, 0.0, 1.0],
                ],
            ),
            (
                TerrainFace::South,
                [0.70710677, 0.0, -0.70710677],
                [
                    [1.0, 0.0, 1.0],
                    [1.0, 1.0, 1.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0],
                ],
            ),
            (
                TerrainFace::West,
                [-0.70710677, 0.0, -0.70710677],
                [
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.0, 1.0, 1.0],
                    [0.0, 0.0, 1.0],
                ],
            ),
            (
                TerrainFace::East,
                [0.70710677, 0.0, 0.70710677],
                [
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0, 1.0],
                    [1.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                ],
            ),
        ];

        for (face, normal, corners) in CROSS_FACES {
            emit_custom_quad(
                mesh,
                x,
                y,
                z,
                block_state_id,
                material,
                light,
                tint[face.index()],
                atlas.rect(texture_indices[face.index()]),
                normal,
                corners,
            );
        }
    }

    fn emit_box(
        mesh: &mut TerrainMesh,
        x: i32,
        y: i32,
        z: i32,
        block_state_id: i32,
        material: TerrainMaterialClass,
        light: TerrainLight,
        tint: [TerrainTint; 6],
        texture_indices: [u32; 6],
        atlas: &TerrainTextureAtlas,
        from: [u8; 3],
        to: [u8; 3],
        face_present: [bool; 6],
        face_uvs: [[u8; 4]; 6],
        face_cull: [bool; 6],
        lookup: &TerrainChunkLookup<'_>,
        mode: TerrainMeshMode,
    ) {
        let min = [
            from[0] as f32 / 16.0,
            from[1] as f32 / 16.0,
            from[2] as f32 / 16.0,
        ];
        let mut max = [
            to[0] as f32 / 16.0,
            to[1] as f32 / 16.0,
            to[2] as f32 / 16.0,
        ];
        if matches!(material, TerrainMaterialClass::Fluid)
            && lookup
                .cell(x, y + 1, z)
                .is_some_and(|neighbor| matches!(neighbor.material, TerrainMaterialClass::Fluid))
        {
            max[1] = 1.0;
        }

        for face in FACES {
            let face_index = face.face.index();
            if !face_present[face_index] {
                continue;
            }
            if face_cull[face_index] {
                let neighbor = lookup.cell(x + face.dx, y + face.dy, z + face.dz);
                if neighbor
                    .map(|neighbor| mode.culls_face_between(material, neighbor.material))
                    .unwrap_or(false)
                {
                    mesh.culled_faces += 1;
                    continue;
                }
            }

            emit_custom_quad_with_uvs(
                mesh,
                x,
                y,
                z,
                block_state_id,
                material,
                light,
                tint[face_index],
                atlas.rect(texture_indices[face_index]),
                face.normal,
                box_face_corners(face.face, min, max),
                face_uvs_from_crop(face_uvs[face_index]),
            );
        }
    }

    fn box_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
        match face {
            TerrainFace::Down => [
                [min[0], min[1], max[2]],
                [max[0], min[1], max[2]],
                [max[0], min[1], min[2]],
                [min[0], min[1], min[2]],
            ],
            TerrainFace::Up => [
                [min[0], max[1], min[2]],
                [max[0], max[1], min[2]],
                [max[0], max[1], max[2]],
                [min[0], max[1], max[2]],
            ],
            TerrainFace::North => [
                [max[0], min[1], min[2]],
                [max[0], max[1], min[2]],
                [min[0], max[1], min[2]],
                [min[0], min[1], min[2]],
            ],
            TerrainFace::South => [
                [min[0], min[1], max[2]],
                [min[0], max[1], max[2]],
                [max[0], max[1], max[2]],
                [max[0], min[1], max[2]],
            ],
            TerrainFace::West => [
                [min[0], min[1], min[2]],
                [min[0], max[1], min[2]],
                [min[0], max[1], max[2]],
                [min[0], min[1], max[2]],
            ],
            TerrainFace::East => [
                [max[0], min[1], max[2]],
                [max[0], max[1], max[2]],
                [max[0], max[1], min[2]],
                [max[0], min[1], min[2]],
            ],
        }
    }

    fn face_uvs_from_crop(uv: [u8; 4]) -> [[f32; 2]; 4] {
        let min_u = uv[0] as f32 / 16.0;
        let min_v = uv[1] as f32 / 16.0;
        let max_u = uv[2] as f32 / 16.0;
        let max_v = uv[3] as f32 / 16.0;
        [
            [min_u, min_v],
            [max_u, min_v],
            [max_u, max_v],
            [min_u, max_v],
        ]
    }

    fn emit_custom_quad(
        mesh: &mut TerrainMesh,
        x: i32,
        y: i32,
        z: i32,
        block_state_id: i32,
        material: TerrainMaterialClass,
        light: TerrainLight,
        tint: TerrainTint,
        uv_rect: TerrainUvRect,
        normal: [f32; 3],
        corners: [[f32; 3]; 4],
    ) {
        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            material,
            light,
            tint,
            uv_rect,
            normal,
            corners,
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        );
    }

    fn emit_custom_quad_with_uvs(
        mesh: &mut TerrainMesh,
        x: i32,
        y: i32,
        z: i32,
        block_state_id: i32,
        material: TerrainMaterialClass,
        light: TerrainLight,
        tint: TerrainTint,
        uv_rect: TerrainUvRect,
        normal: [f32; 3],
        corners: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
    ) {
        let base = mesh.vertices.len() as u32;
        for (corner, uv) in corners.into_iter().zip(uvs) {
            mesh.vertices.push(TerrainVertex {
                position: [
                    x as f32 + corner[0],
                    y as f32 + corner[1],
                    z as f32 + corner[2],
                ],
                normal,
                uv: uv_rect.map(uv),
                light: light.as_shader_light(),
                tint: tint.as_shader_tint(),
                block_state_id,
            });
        }
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        match material {
            TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
            TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
            TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
                mesh.translucent_faces += 1
            }
            _ => {}
        }
    }

    fn cell_index(x: usize, y: usize, z: usize, height: usize) -> usize {
        debug_assert!(x < 16);
        debug_assert!(y < height);
        debug_assert!(z < 16);
        ((y * 16) + z) * 16 + x
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn meshes_single_opaque_block_as_six_faces() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            let snapshot = TerrainChunkSnapshot::new(2, -3, 0, 1, cells);

            let mesh = build_opaque_chunk_mesh(&snapshot);
            assert_eq!(mesh.source_sections, 1);
            assert_eq!(mesh.opaque_faces, 6);
            assert_eq!(mesh.culled_faces, 0);
            assert_eq!(mesh.vertices.len(), 24);
            assert_eq!(mesh.indices.len(), 36);
            assert!(mesh
                .vertices
                .iter()
                .all(|vertex| vertex.block_state_id == 1));
            assert!(mesh
                .vertices
                .iter()
                .any(|vertex| vertex.position[0] >= 33.0 && vertex.position[2] <= -45.0));
        }

        #[test]
        fn culls_internal_faces_between_opaque_blocks() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            cells[cell_index(2, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh = build_opaque_chunk_mesh(&snapshot);
            assert_eq!(mesh.source_sections, 1);
            assert_eq!(mesh.opaque_faces, 10);
            assert_eq!(mesh.culled_faces, 2);
            assert_eq!(mesh.vertices.len(), 40);
            assert_eq!(mesh.indices.len(), 60);
        }

        #[test]
        fn culls_internal_faces_between_fluid_blocks() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
            cells[cell_index(2, 0, 2, 1)] =
                TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let layers =
                build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
            assert_eq!(layers.translucent.len(), 1);
            assert_eq!(layers.translucent[0].translucent_faces, 10);
            assert_eq!(layers.translucent[0].culled_faces, 2);
            assert_eq!(layers.translucent[0].vertices.len(), 40);
            assert_eq!(layers.translucent[0].indices.len(), 60);
        }

        #[test]
        fn fluid_box_extends_to_above_fluid_without_internal_top_face() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
            cells[cell_index(1, 0, 2, 2)] =
                TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
            cells[cell_index(1, 1, 2, 2)] =
                TerrainCell::with_shape(87, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

            let layers =
                build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
            let mesh = &layers.translucent[0];

            assert_eq!(mesh.translucent_faces, 10);
            assert_eq!(mesh.culled_faces, 2);
            assert!(mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.block_state_id == 86)
                .any(|vertex| vertex.position[1] == 1.0));
            assert!(mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.block_state_id == 87)
                .any(|vertex| vertex.position[1] == 1.875));
        }

        #[test]
        fn culls_faces_between_adjacent_chunk_snapshots() {
            let left = single_block_snapshot(0, 0, 15, 0, 2);
            let right = single_block_snapshot(1, 0, 0, 0, 2);

            let meshes = build_opaque_terrain_meshes(&[left, right]);
            assert_eq!(meshes.len(), 2);
            assert_eq!(
                meshes.iter().map(|mesh| mesh.opaque_faces).sum::<usize>(),
                10
            );
            assert_eq!(
                meshes.iter().map(|mesh| mesh.culled_faces).sum::<usize>(),
                2
            );
            assert_eq!(
                meshes.iter().map(|mesh| mesh.vertices.len()).sum::<usize>(),
                40
            );
            assert_eq!(
                meshes.iter().map(|mesh| mesh.indices.len()).sum::<usize>(),
                60
            );
        }

        #[test]
        fn culls_fluid_faces_between_adjacent_chunk_snapshots() {
            let left = single_fluid_snapshot(0, 0, 15, 0, 2);
            let right = single_fluid_snapshot(1, 0, 0, 0, 2);

            let layers =
                build_terrain_mesh_layers_with_atlas(&[left, right], &TerrainTextureAtlas::unit());
            assert_eq!(layers.translucent.len(), 2);
            assert_eq!(
                layers
                    .translucent
                    .iter()
                    .map(|mesh| mesh.translucent_faces)
                    .sum::<usize>(),
                10
            );
            assert_eq!(
                layers
                    .translucent
                    .iter()
                    .map(|mesh| mesh.culled_faces)
                    .sum::<usize>(),
                2
            );
        }

        #[test]
        fn maps_face_uvs_into_texture_atlas_rect() {
            let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
            snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1; 6];
            let atlas = TerrainTextureAtlas {
                rects: vec![
                    TerrainUvRect::UNIT,
                    TerrainUvRect {
                        min: [0.25, 0.5],
                        max: [0.5, 0.75],
                    },
                ],
                fallback_index: 0,
            };

            let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
                .into_iter()
                .next()
                .unwrap();

            assert_eq!(mesh.vertices[0].uv, [0.25, 0.5]);
            assert_eq!(mesh.vertices[1].uv, [0.5, 0.5]);
            assert_eq!(mesh.vertices[2].uv, [0.5, 0.75]);
            assert_eq!(mesh.vertices[3].uv, [0.25, 0.75]);
        }

        #[test]
        fn maps_different_faces_to_different_texture_rects() {
            let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
            snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1, 2, 0, 0, 0, 0];
            let atlas = TerrainTextureAtlas {
                rects: vec![
                    TerrainUvRect::UNIT,
                    TerrainUvRect {
                        min: [0.0, 0.0],
                        max: [0.25, 0.25],
                    },
                    TerrainUvRect {
                        min: [0.5, 0.5],
                        max: [0.75, 0.75],
                    },
                ],
                fallback_index: 0,
            };

            let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
                .into_iter()
                .next()
                .unwrap();

            assert_eq!(mesh.vertices[0].uv, [0.0, 0.0]);
            assert_eq!(mesh.vertices[4].uv, [0.5, 0.5]);
        }

        #[test]
        fn combined_terrain_mesh_includes_cutout_faces() {
            let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
            snapshot.cells[cell_index(1, 0, 2, 1)].material = TerrainMaterialClass::Cutout;

            let opaque = build_opaque_chunk_mesh(&snapshot);
            assert_eq!(opaque.opaque_faces, 0);
            assert_eq!(opaque.cutout_faces, 0);

            let combined =
                build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                    .into_iter()
                    .next()
                    .unwrap();
            assert_eq!(combined.opaque_faces, 0);
            assert_eq!(combined.cutout_faces, 6);
            assert_eq!(combined.vertices.len(), 24);
            assert_eq!(combined.indices.len(), 36);
        }

        #[test]
        fn cross_cutout_mesh_emits_vanilla_cross_quads() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
                2,
                TerrainMaterialClass::Cutout,
                0,
                TerrainRenderShape::Cross,
            );
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                .into_iter()
                .next()
                .unwrap();

            assert_eq!(mesh.opaque_faces, 0);
            assert_eq!(mesh.cutout_faces, 4);
            assert_eq!(mesh.culled_faces, 0);
            assert_eq!(mesh.vertices.len(), 16);
            assert_eq!(mesh.indices.len(), 24);
            assert!(mesh
                .vertices
                .iter()
                .any(|vertex| vertex.position == [1.0, 0.0, 2.0]));
            assert!(mesh
                .vertices
                .iter()
                .any(|vertex| vertex.position == [2.0, 1.0, 3.0]));
        }

        #[test]
        fn cross_cutout_faces_are_not_neighbor_culled() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
                2,
                TerrainMaterialClass::Cutout,
                0,
                TerrainRenderShape::Cross,
            );
            cells[cell_index(2, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let layers =
                build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

            assert_eq!(layers.opaque[0].opaque_faces, 6);
            assert_eq!(layers.cutout[0].cutout_faces, 4);
            assert_eq!(layers.cutout[0].culled_faces, 0);
        }

        #[test]
        fn box_model_mesh_uses_bounds_and_face_uv_crop() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 1, slab_box_shape());
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);
            let atlas = TerrainTextureAtlas {
                rects: vec![
                    TerrainUvRect::UNIT,
                    TerrainUvRect {
                        min: [0.25, 0.25],
                        max: [0.75, 0.75],
                    },
                ],
                fallback_index: 0,
            };

            let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
                .into_iter()
                .next()
                .unwrap();

            assert_eq!(mesh.opaque_faces, 6);
            assert_eq!(mesh.vertices.len(), 24);
            assert!(mesh.vertices.iter().any(|vertex| vertex.position[1] == 0.5));
            assert!(!mesh.vertices.iter().any(|vertex| vertex.position[1] == 1.0));
            assert!(mesh.vertices.iter().any(|vertex| vertex.uv == [0.25, 0.5]));
        }

        #[test]
        fn box_model_culls_only_faces_marked_by_cullface() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 0, slab_box_shape());
            cells[cell_index(1, 0, 1, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            cells[cell_index(1, 0, 3, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh =
                build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                    .into_iter()
                    .next()
                    .unwrap();

            assert_eq!(mesh.culled_faces, 4);
            assert_eq!(mesh.opaque_faces, 14);
        }

        #[test]
        fn multi_box_model_skips_absent_faces() {
            let mut upper = TerrainBox {
                from: [8, 8, 0],
                to: [16, 16, 16],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_cull: [false; 6],
                texture_indices: [0; 6],
                tint: [TerrainTint::WHITE; 6],
            };
            upper.face_present[TerrainFace::Down.index()] = false;
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] = TerrainCell {
                block_state_id: 4,
                material: TerrainMaterialClass::Opaque,
                texture_indices: [0; 6],
                light: TerrainLight::FULL_BRIGHT,
                tint: [TerrainTint::WHITE; 6],
                render_shape: TerrainRenderShape::Boxes(vec![
                    TerrainBox {
                        from: [0, 0, 0],
                        to: [16, 8, 16],
                        face_present: [true; 6],
                        face_uvs: [[0, 0, 16, 16]; 6],
                        face_cull: [false; 6],
                        texture_indices: [0; 6],
                        tint: [TerrainTint::WHITE; 6],
                    },
                    upper,
                ]),
            };
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh =
                build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                    .into_iter()
                    .next()
                    .unwrap();

            assert_eq!(mesh.opaque_faces, 11);
            assert_eq!(mesh.vertices.len(), 44);
            assert_eq!(mesh.indices.len(), 66);
        }

        #[test]
        fn boxes_use_per_box_texture_and_tint() {
            let grass_tint = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
            let foliage_tint = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] = TerrainCell {
                block_state_id: 4,
                material: TerrainMaterialClass::Opaque,
                texture_indices: [0; 6],
                light: TerrainLight::FULL_BRIGHT,
                tint: [TerrainTint::WHITE; 6],
                render_shape: TerrainRenderShape::Boxes(vec![
                    TerrainBox {
                        from: [0, 0, 0],
                        to: [16, 8, 16],
                        face_present: [true; 6],
                        face_uvs: [[0, 0, 16, 16]; 6],
                        face_cull: [false; 6],
                        texture_indices: [1; 6],
                        tint: [grass_tint; 6],
                    },
                    TerrainBox {
                        from: [0, 8, 0],
                        to: [16, 16, 16],
                        face_present: [true; 6],
                        face_uvs: [[0, 0, 16, 16]; 6],
                        face_cull: [false; 6],
                        texture_indices: [2; 6],
                        tint: [foliage_tint; 6],
                    },
                ]),
            };
            let atlas = TerrainTextureAtlas {
                rects: vec![
                    TerrainUvRect::UNIT,
                    TerrainUvRect {
                        min: [0.25, 0.25],
                        max: [0.5, 0.5],
                    },
                    TerrainUvRect {
                        min: [0.5, 0.5],
                        max: [0.75, 0.75],
                    },
                ],
                fallback_index: 0,
            };
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
                .into_iter()
                .next()
                .unwrap();

            assert_eq!(mesh.vertices[0].uv, [0.25, 0.25]);
            assert_eq!(mesh.vertices[0].tint, grass_tint.as_shader_tint());
            assert_eq!(mesh.vertices[24].uv, [0.5, 0.5]);
            assert_eq!(mesh.vertices[24].tint, foliage_tint.as_shader_tint());
        }

        #[test]
        fn mesh_vertices_carry_terrain_light() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0)
                    .with_light(TerrainLight { sky: 9, block: 6 });
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh = build_opaque_chunk_mesh(&snapshot);

            assert_eq!(mesh.vertices[0].light, [6.0 / 15.0, 9.0 / 15.0]);
        }

        #[test]
        fn mesh_vertices_carry_face_tint() {
            let mut face_tints = [TerrainTint::WHITE; 6];
            face_tints[TerrainFace::Down.index()] = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
            face_tints[TerrainFace::Up.index()] = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0).with_tint(face_tints);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let mesh = build_opaque_chunk_mesh(&snapshot);

            assert_eq!(
                mesh.vertices[0].tint,
                TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59).as_shader_tint()
            );
            assert_eq!(
                mesh.vertices[4].tint,
                TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18).as_shader_tint()
            );
        }

        #[test]
        fn layer_builder_splits_opaque_and_cutout_meshes() {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(1, 0, 2, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            cells[cell_index(3, 0, 2, 1)] =
                TerrainCell::with_texture(2, TerrainMaterialClass::Cutout, 0);
            cells[cell_index(5, 0, 2, 1)] =
                TerrainCell::with_texture(3, TerrainMaterialClass::Translucent, 0);
            cells[cell_index(7, 0, 2, 1)] =
                TerrainCell::with_texture(4, TerrainMaterialClass::Fluid, 0);
            let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

            let layers =
                build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
            assert_eq!(layers.source_sections, 1);
            assert_eq!(layers.opaque.len(), 1);
            assert_eq!(layers.cutout.len(), 1);
            assert_eq!(layers.translucent.len(), 1);
            assert_eq!(layers.opaque[0].opaque_faces, 6);
            assert_eq!(layers.opaque[0].cutout_faces, 0);
            assert_eq!(layers.opaque[0].translucent_faces, 0);
            assert_eq!(layers.cutout[0].opaque_faces, 0);
            assert_eq!(layers.cutout[0].cutout_faces, 6);
            assert_eq!(layers.cutout[0].translucent_faces, 0);
            assert_eq!(layers.translucent[0].opaque_faces, 0);
            assert_eq!(layers.translucent[0].cutout_faces, 0);
            assert_eq!(layers.translucent[0].translucent_faces, 12);
        }

        fn single_block_snapshot(
            chunk_x: i32,
            chunk_z: i32,
            x: usize,
            y: usize,
            z: usize,
        ) -> TerrainChunkSnapshot {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(x, y, z, 1)] =
                TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
        }

        fn single_fluid_snapshot(
            chunk_x: i32,
            chunk_z: i32,
            x: usize,
            y: usize,
            z: usize,
        ) -> TerrainChunkSnapshot {
            let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
            cells[cell_index(x, y, z, 1)] =
                TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
            TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
        }

        fn slab_box_shape() -> TerrainRenderShape {
            let mut face_uvs = [[0, 0, 16, 16]; 6];
            face_uvs[TerrainFace::North.index()] = [0, 8, 16, 16];
            face_uvs[TerrainFace::South.index()] = [0, 8, 16, 16];
            face_uvs[TerrainFace::West.index()] = [0, 8, 16, 16];
            face_uvs[TerrainFace::East.index()] = [0, 8, 16, 16];
            let mut face_cull = [true; 6];
            face_cull[TerrainFace::Up.index()] = false;
            TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, 8, 16],
                face_present: [true; 6],
                face_uvs,
                face_cull,
            }
        }

        fn fluid_box_shape(height: u8) -> TerrainRenderShape {
            let mut face_uvs = [[0, 0, 16, 16]; 6];
            let side_v0 = 16 - height;
            face_uvs[TerrainFace::North.index()] = [0, side_v0, 16, 16];
            face_uvs[TerrainFace::South.index()] = [0, side_v0, 16, 16];
            face_uvs[TerrainFace::West.index()] = [0, side_v0, 16, 16];
            face_uvs[TerrainFace::East.index()] = [0, side_v0, 16, 16];
            TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, height, 16],
                face_present: [true; 6],
                face_uvs,
                face_cull: [true; 6],
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for ClearColor {
    fn default() -> Self {
        Self {
            r: 0.04,
            g: 0.07,
            b: 0.10,
            a: 1.0,
        }
    }
}

impl From<ClearColor> for wgpu::Color {
    fn from(value: ClearColor) -> Self {
        Self {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraPose {
    pub position: [f32; 3],
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

impl CameraPose {
    pub const STANDING_EYE_HEIGHT: f32 = 1.62;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionOutline {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const HUD_HOTBAR_WIDTH: u32 = 182;
const HUD_HOTBAR_HEIGHT: u32 = 22;
const HUD_EXPERIENCE_BAR_WIDTH: u32 = 182;
const HUD_EXPERIENCE_BAR_HEIGHT: u32 = 5;
const HUD_EXPERIENCE_MARGIN_BOTTOM: f32 = 24.0;
const HUD_HEARTS_PER_ROW: u32 = 10;
const HUD_HEART_SPACING: f32 = 8.0;
const HUD_FOOD_ICONS_PER_ROW: u32 = 10;
const HUD_FOOD_SPACING: f32 = 8.0;

const TERRAIN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32x3, 5 => Sint32];
const SELECTION_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 1] =
    wgpu::vertex_attr_array![0 => Float32x3];
const HUD_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

const TERRAIN_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var terrain_atlas: texture_2d<f32>;

@group(0) @binding(2)
var terrain_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light: vec2<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) block_state_id: i32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) light: vec2<f32>,
    @location(3) tint: vec3<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.normal = input.normal;
    out.uv = input.uv;
    out.light = input.light;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.35, 0.82, 0.45));
    let normal = normalize(input.normal);
    let direct = max(dot(normal, light_dir), 0.0);
    let texel = textureSample(terrain_atlas, terrain_sampler, input.uv);
    if texel.a <= 0.01 {
        discard;
    }
    let base = texel.rgb * input.tint;
    let block_light = input.light.x;
    let sky_light = input.light.y;
    let light_level = max(block_light, sky_light * 0.95);
    let shade = (0.16 + light_level * 0.84) * (0.86 + direct * 0.14);
    return vec4<f32>(base * shade, texel.a);
}
"#;

const SELECTION_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(input.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.65);
}
"#;

const HUD_SHADER: &str = r#"
@group(0) @binding(0)
var hud_texture: texture_2d<f32>;
@group(0) @binding(1)
var hud_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(hud_texture, hud_sampler, input.uv);
    if texel.a <= 0.01 {
        discard;
    }
    return texel;
}
"#;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear: ClearColor,
    counters: RendererCounters,
    depth: DepthTarget,
    terrain_pipeline: wgpu::RenderPipeline,
    terrain_translucent_pipeline: wgpu::RenderPipeline,
    selection_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    hud_bind_group_layout: wgpu::BindGroupLayout,
    terrain_bind_group_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    terrain_atlas: TerrainAtlasGpu,
    terrain_bind_group: wgpu::BindGroup,
    terrain_opaque: Vec<ResidentTerrainMesh>,
    terrain_cutout: Vec<ResidentTerrainMesh>,
    terrain_translucent: Vec<ResidentTerrainMesh>,
    terrain_source_sections: usize,
    terrain_bounds: Option<TerrainBounds>,
    camera_pose: Option<CameraPose>,
    selection_outline: Option<SelectionOutlineGpu>,
    hud_crosshair: Option<HudSpriteGpu>,
    hud_hotbar: Option<HudSpriteGpu>,
    hud_hotbar_selection: Option<HudSpriteGpu>,
    hud_experience_background: Option<HudSpriteGpu>,
    hud_experience_progress: Option<HudSpriteGpu>,
    hud_heart_container: Option<HudSpriteGpu>,
    hud_heart_full: Option<HudSpriteGpu>,
    hud_heart_half: Option<HudSpriteGpu>,
    hud_food_empty: Option<HudSpriteGpu>,
    hud_food_full: Option<HudSpriteGpu>,
    hud_food_half: Option<HudSpriteGpu>,
    hud_health: Option<f32>,
    hud_food: Option<i32>,
    hud_experience_progress_value: Option<f32>,
    hud_selected_slot: u8,
}

struct ResidentTerrainMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_count: usize,
    index_count: usize,
    opaque_faces: usize,
    cutout_faces: usize,
    translucent_faces: usize,
    culled_faces: usize,
    resident_bytes: u64,
}

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct TerrainAtlasGpu {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

struct SelectionOutlineGpu {
    outline: SelectionOutline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

struct HudSpriteGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

struct HudDrawCommand<'a> {
    sprite: &'a HudSpriteGpu,
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, Copy)]
struct HudRect {
    x: f32,
    y: f32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudIconFill {
    Empty,
    Half,
    Full,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct HudVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SelectionVertex {
    position: [f32; 3],
}

impl CameraUniform {
    fn identity() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn from_bounds(bounds: TerrainBounds, aspect: f32) -> Self {
        let center = bounds.center();
        let extent = bounds.extent();
        let radius = extent.length().max(48.0);
        let eye = center + Vec3::new(radius * 0.55, radius * 0.42, radius * 0.78);
        let view = Mat4::look_at_rh(eye, center, Vec3::Y);

        let half_height = (extent.y * 0.65 + extent.x.max(extent.z) * 0.45).max(40.0);
        let half_width = half_height * aspect.max(0.1);
        let far = radius * 5.0 + 512.0;
        let projection =
            Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, 0.1, far);

        Self {
            view_proj: (projection * view).to_cols_array_2d(),
        }
    }

    fn from_pose(pose: CameraPose, aspect: f32) -> Self {
        let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
        let yaw = pose.y_rot.to_radians();
        let pitch = pose.x_rot.to_radians();
        let cos_pitch = pitch.cos();
        let forward = Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch)
            .normalize_or_zero();
        let target = eye
            + if forward.length_squared() > 0.0 {
                forward
            } else {
                Vec3::Z
            };
        let projection = Mat4::perspective_rh(70.0_f32.to_radians(), aspect.max(0.1), 0.05, 2048.0);
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);

        Self {
            view_proj: (projection * view).to_cols_array_2d(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TerrainBounds {
    min: Vec3,
    max: Vec3,
}

impl TerrainBounds {
    fn from_vertices(vertices: &[terrain::TerrainVertex]) -> Option<Self> {
        let mut vertices = vertices.iter();
        let first = vertices.next()?;
        let mut bounds = Self {
            min: Vec3::from_array(first.position),
            max: Vec3::from_array(first.position),
        };
        for vertex in vertices {
            bounds.include_point(Vec3::from_array(vertex.position));
        }
        Some(bounds)
    }

    fn include_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    fn include_bounds(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn extent(self) -> Vec3 {
        (self.max - self.min).max(Vec3::splat(1.0))
    }
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window)?)?
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("bbb-native-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = choose_format(&caps.formats)?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .first()
                .copied()
                .ok_or_else(|| anyhow!("surface has no present modes"))?
        };
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let depth = create_depth_target(&device, config.width, config.height);
        let terrain_bind_group_layout = create_terrain_bind_group_layout(&device);
        let hud_bind_group_layout = create_hud_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        let terrain_atlas = create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 255, 255, 255])?;
        let terrain_bind_group = create_terrain_bind_group(
            &device,
            &terrain_bind_group_layout,
            &camera_buffer,
            &terrain_atlas,
        );
        let terrain_pipeline = create_terrain_pipeline(&device, format, &terrain_bind_group_layout);
        let terrain_translucent_pipeline =
            create_terrain_translucent_pipeline(&device, format, &terrain_bind_group_layout);
        let selection_pipeline =
            create_selection_pipeline(&device, format, &terrain_bind_group_layout);
        let hud_pipeline = create_hud_pipeline(&device, format, &hud_bind_group_layout);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            clear: ClearColor::default(),
            counters: RendererCounters {
                width: size.width,
                height: size.height,
                ..RendererCounters::default()
            },
            depth,
            terrain_pipeline,
            terrain_translucent_pipeline,
            selection_pipeline,
            hud_pipeline,
            hud_bind_group_layout,
            terrain_bind_group_layout,
            camera_buffer,
            terrain_atlas,
            terrain_bind_group,
            terrain_opaque: Vec::new(),
            terrain_cutout: Vec::new(),
            terrain_translucent: Vec::new(),
            terrain_source_sections: 0,
            terrain_bounds: None,
            camera_pose: None,
            selection_outline: None,
            hud_crosshair: None,
            hud_hotbar: None,
            hud_hotbar_selection: None,
            hud_experience_background: None,
            hud_experience_progress: None,
            hud_heart_container: None,
            hud_heart_full: None,
            hud_heart_half: None,
            hud_food_empty: None,
            hud_food_full: None,
            hud_food_half: None,
            hud_health: None,
            hud_food: None,
            hud_experience_progress_value: None,
            hud_selected_slot: 0,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth = create_depth_target(&self.device, self.config.width, self.config.height);
        self.update_camera();
        self.counters.width = size.width;
        self.counters.height = size.height;
    }

    pub fn render(&mut self, screenshot: Option<&Path>) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-native-clear"),
            });

        let mut opaque_draw_calls = 0;
        let mut cutout_draw_calls = 0;
        let mut translucent_draw_calls = 0;
        let mut selection_draw_calls = 0;
        let mut hud_draw_calls = 0;
        let mut pipeline_switches = 0;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-opaque-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !self.terrain_opaque.is_empty() {
                pass.set_pipeline(&self.terrain_pipeline);
                pipeline_switches += 1;
                pass.set_bind_group(0, &self.terrain_bind_group, &[]);
                for mesh in &self.terrain_opaque {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                    opaque_draw_calls += 1;
                }
            }
        }

        if !self.terrain_cutout.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-cutout-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.terrain_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for mesh in &self.terrain_cutout {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                cutout_draw_calls += 1;
            }
        }

        if !self.terrain_translucent.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-terrain-translucent-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.terrain_translucent_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            for mesh in &self.terrain_translucent {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                translucent_draw_calls += 1;
            }
        }

        if let Some(outline) = &self.selection_outline {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-native-selection-outline-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.selection_pipeline);
            pipeline_switches += 1;
            pass.set_bind_group(0, &self.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, outline.vertex_buffer.slice(..));
            pass.draw(0..outline.vertex_count, 0..1);
            selection_draw_calls += 1;
        }

        {
            let (hud_vertices, hud_commands) = self.collect_hud_draws();
            if !hud_commands.is_empty() {
                let hud_vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("bbb-hud-frame-vertices"),
                            contents: bytemuck::cast_slice(&hud_vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-native-hud-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.hud_pipeline);
                pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
                pipeline_switches += 1;
                for command in &hud_commands {
                    pass.set_bind_group(0, &command.sprite.bind_group, &[]);
                    pass.draw(command.start..command.end, 0..1);
                }
                hud_draw_calls = hud_commands.len() as u64;
            }
        }

        let readback = if let Some(path) = screenshot {
            Some(self.prepare_screenshot_copy(&mut encoder, &frame.texture, path)?)
        } else {
            None
        };

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        if let Some(readback) = readback {
            self.finish_screenshot(readback)?;
            self.counters.screenshots_written += 1;
        }

        self.counters.frame_index += 1;
        self.counters.opaque_draw_calls = opaque_draw_calls;
        self.counters.cutout_draw_calls = cutout_draw_calls;
        self.counters.translucent_draw_calls = translucent_draw_calls;
        self.counters.selection_draw_calls = selection_draw_calls;
        self.counters.hud_draw_calls = hud_draw_calls;
        self.counters.draw_calls = opaque_draw_calls
            + cutout_draw_calls
            + translucent_draw_calls
            + selection_draw_calls
            + hud_draw_calls;
        self.counters.pipeline_switches = pipeline_switches;
        Ok(())
    }

    pub fn counters(&self) -> RendererCounters {
        self.counters.clone()
    }

    pub fn set_camera_pose(&mut self, pose: Option<CameraPose>) {
        if self.camera_pose == pose {
            return;
        }
        self.camera_pose = pose;
        self.update_camera();
    }

    pub fn set_clear_color(&mut self, clear: ClearColor) {
        self.clear = clear;
    }

    pub fn set_selection_outline(&mut self, outline: Option<SelectionOutline>) {
        if self
            .selection_outline
            .as_ref()
            .map(|selection| selection.outline)
            == outline
        {
            return;
        }
        self.selection_outline =
            outline.map(|outline| create_selection_outline_gpu(&self.device, outline));
    }

    pub fn upload_hud_crosshair(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        let crosshair = self.upload_hud_sprite(width, height, rgba)?;
        self.counters.hud_crosshair_width = width;
        self.counters.hud_crosshair_height = height;
        self.hud_crosshair = Some(crosshair);
        Ok(())
    }

    pub fn upload_hud_hotbar(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_hotbar = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_hotbar_selection(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_hotbar_selection = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_background = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_experience_progress(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_experience_progress = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_container(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.hud_heart_container = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_heart_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_heart_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_empty(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_empty = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_full(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_full = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_food_half(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        self.hud_food_half = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn set_hud_health(&mut self, health: Option<f32>) {
        self.hud_health = health.filter(|health| health.is_finite());
    }

    pub fn set_hud_food(&mut self, food: Option<i32>) {
        self.hud_food = food;
    }

    pub fn set_hud_experience_progress(&mut self, progress: Option<f32>) {
        self.hud_experience_progress_value = progress
            .filter(|progress| progress.is_finite())
            .map(|progress| progress.clamp(0.0, 1.0));
    }

    pub fn set_hud_selected_slot(&mut self, slot: u8) {
        self.hud_selected_slot = slot.min(8);
    }

    pub fn upload_terrain_meshes(&mut self, meshes: Vec<terrain::TerrainMesh>) {
        let source_sections = meshes.iter().map(|mesh| mesh.source_sections).sum();
        self.upload_terrain_mesh_layers(terrain::TerrainMeshLayers {
            opaque: meshes,
            cutout: Vec::new(),
            translucent: Vec::new(),
            source_sections,
        });
    }

    pub fn upload_terrain_mesh_layers(&mut self, layers: terrain::TerrainMeshLayers) {
        self.terrain_source_sections = layers.source_sections;
        self.counters.queued_sections = self.terrain_source_sections;
        self.counters.meshed_sections = self.terrain_source_sections;
        self.counters.upload_bytes = 0;
        self.terrain_opaque.clear();
        self.terrain_cutout.clear();
        self.terrain_translucent.clear();
        self.terrain_bounds = None;

        for mesh in layers.opaque {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_opaque.push(resident);
            }
        }
        for mesh in layers.cutout {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_cutout.push(resident);
            }
        }
        for mesh in layers.translucent {
            if let Some(resident) = self.create_resident_terrain_mesh(mesh) {
                self.terrain_translucent.push(resident);
            }
        }

        self.update_camera();
        self.refresh_terrain_counters();
    }

    pub fn upload_terrain_texture_atlas(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        self.terrain_atlas =
            create_terrain_atlas_gpu(&self.device, &self.queue, width, height, rgba)?;
        self.terrain_bind_group = create_terrain_bind_group(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            &self.terrain_atlas,
        );
        self.counters.atlas_pages = 1;
        self.counters.atlas_reallocations += 1;
        self.counters.atlas_width = width;
        self.counters.atlas_height = height;
        Ok(())
    }

    fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.config.width.max(1), self.config.height.max(1))
    }

    fn upload_hud_sprite(&self, width: u32, height: u32, rgba: &[u8]) -> Result<HudSpriteGpu> {
        create_hud_sprite_gpu(
            &self.device,
            &self.queue,
            &self.hud_bind_group_layout,
            width,
            height,
            rgba,
        )
    }

    fn collect_hud_draws(&self) -> (Vec<HudVertex>, Vec<HudDrawCommand<'_>>) {
        let mut vertices = Vec::new();
        let mut commands = Vec::new();
        let surface_size = self.surface_size();

        if let Some(crosshair) = &self.hud_crosshair {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                crosshair,
                surface_size,
                centered_hud_rect(surface_size, crosshair.width, crosshair.height),
            );
        }

        if let Some(hotbar) = &self.hud_hotbar {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                hotbar,
                surface_size,
                hotbar_hud_rect(surface_size, hotbar.width, hotbar.height),
            );
        }
        if let Some(selection) = &self.hud_hotbar_selection {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                selection,
                surface_size,
                hotbar_selection_hud_rect(
                    surface_size,
                    self.hud_selected_slot,
                    selection.width,
                    selection.height,
                ),
            );
        }

        if let (Some(progress), Some(background)) = (
            self.hud_experience_progress_value,
            &self.hud_experience_background,
        ) {
            push_hud_draw(
                &mut vertices,
                &mut commands,
                background,
                surface_size,
                experience_bar_hud_rect(surface_size, background.width, background.height),
            );

            let progress_width = hud_experience_progress_width(progress);
            if progress_width > 0 {
                if let Some(progress_sprite) = &self.hud_experience_progress {
                    push_hud_draw_with_uv(
                        &mut vertices,
                        &mut commands,
                        progress_sprite,
                        surface_size,
                        experience_bar_hud_rect(
                            surface_size,
                            progress_width,
                            progress_sprite.height,
                        ),
                        progress_width as f32 / progress_sprite.width.max(1) as f32,
                    );
                }
            }
        }

        if let (Some(health), Some(container)) = (self.hud_health, &self.hud_heart_container) {
            for index in 0..HUD_HEARTS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    container,
                    surface_size,
                    heart_hud_rect(surface_size, index, container.width, container.height),
                );
            }

            for index in 0..HUD_HEARTS_PER_ROW {
                let sprite = match hud_heart_fill(health, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_heart_half.as_ref(),
                    HudIconFill::Full => self.hud_heart_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        heart_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        if let (Some(food), Some(empty)) = (self.hud_food, &self.hud_food_empty) {
            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                push_hud_draw(
                    &mut vertices,
                    &mut commands,
                    empty,
                    surface_size,
                    food_hud_rect(surface_size, index, empty.width, empty.height),
                );
            }

            for index in 0..HUD_FOOD_ICONS_PER_ROW {
                let sprite = match hud_food_fill(food, index) {
                    HudIconFill::Empty => None,
                    HudIconFill::Half => self.hud_food_half.as_ref(),
                    HudIconFill::Full => self.hud_food_full.as_ref(),
                };
                if let Some(sprite) = sprite {
                    push_hud_draw(
                        &mut vertices,
                        &mut commands,
                        sprite,
                        surface_size,
                        food_hud_rect(surface_size, index, sprite.width, sprite.height),
                    );
                }
            }
        }

        (vertices, commands)
    }

    fn refresh_terrain_counters(&mut self) {
        self.counters.uploaded_sections = if self.terrain_opaque.is_empty()
            && self.terrain_cutout.is_empty()
            && self.terrain_translucent.is_empty()
        {
            0
        } else {
            self.terrain_source_sections
        };
        self.counters.visible_sections = self.counters.uploaded_sections;
        self.counters.resident_bytes = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.resident_bytes)
            .sum();
        self.counters.terrain_vertices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.vertex_count)
            .sum();
        self.counters.terrain_indices = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.index_count)
            .sum();
        self.counters.opaque_faces = self
            .terrain_opaque
            .iter()
            .map(|mesh| mesh.opaque_faces)
            .sum();
        self.counters.cutout_faces = self
            .terrain_cutout
            .iter()
            .map(|mesh| mesh.cutout_faces)
            .sum();
        self.counters.translucent_faces = self
            .terrain_translucent
            .iter()
            .map(|mesh| mesh.translucent_faces)
            .sum();
        self.counters.culled_faces = self
            .terrain_opaque
            .iter()
            .chain(self.terrain_cutout.iter())
            .chain(self.terrain_translucent.iter())
            .map(|mesh| mesh.culled_faces)
            .sum();
    }

    fn create_resident_terrain_mesh(
        &mut self,
        mesh: terrain::TerrainMesh,
    ) -> Option<ResidentTerrainMesh> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }

        let bounds = TerrainBounds::from_vertices(&mesh.vertices);
        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-vertex-buffer"),
                contents: vertex_bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bbb-terrain-index-buffer"),
                contents: index_bytes,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
        let resident_bytes = (vertex_bytes.len() + index_bytes.len()) as u64;
        self.counters.upload_bytes += resident_bytes;
        if let Some(bounds) = bounds {
            if let Some(existing) = &mut self.terrain_bounds {
                existing.include_bounds(bounds);
            } else {
                self.terrain_bounds = Some(bounds);
            }
        }
        Some(ResidentTerrainMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len(),
            opaque_faces: mesh.opaque_faces,
            cutout_faces: mesh.cutout_faces,
            translucent_faces: mesh.translucent_faces,
            culled_faces: mesh.culled_faces,
            resident_bytes,
        })
    }

    fn update_camera(&self) {
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let uniform = if let Some(pose) = self.camera_pose {
            CameraUniform::from_pose(pose, aspect)
        } else {
            self.terrain_bounds
                .map(|bounds| CameraUniform::from_bounds(bounds, aspect))
                .unwrap_or_else(CameraUniform::identity)
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn prepare_screenshot_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        path: &Path,
    ) -> Result<PendingScreenshot> {
        let width = self.config.width;
        let height = self.config.height;
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-native-screenshot-buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(PendingScreenshot {
            buffer,
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            format: self.config.format,
            path: path.to_path_buf(),
        })
    }

    fn finish_screenshot(&self, pending: PendingScreenshot) -> Result<()> {
        let slice = pending.buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .context("screenshot map callback dropped")?
            .map_err(|err| anyhow!("map screenshot buffer: {err}"))?;

        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((pending.width * pending.height * 4) as usize);
        for row in mapped
            .chunks(pending.padded_bytes_per_row as usize)
            .take(pending.height as usize)
        {
            let row = &row[..pending.unpadded_bytes_per_row as usize];
            match pending.format {
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                    for px in row.chunks_exact(4) {
                        rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                    }
                }
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
                    rgba.extend_from_slice(row);
                }
                other => bail!("unsupported screenshot surface format {other:?}"),
            }
        }
        drop(mapped);
        pending.buffer.unmap();

        if let Some(parent) = pending.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        image::save_buffer(
            &pending.path,
            &rgba,
            pending.width,
            pending.height,
            image::ColorType::Rgba8,
        )?;
        Ok(())
    }
}

fn create_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-terrain-depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    DepthTarget {
        _texture: texture,
        view,
    }
}

fn create_terrain_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-terrain-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_camera_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-terrain-camera-buffer"),
        contents: bytemuck::bytes_of(&CameraUniform::identity()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

fn create_hud_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bbb-hud-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_terrain_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    atlas: &TerrainAtlasGpu,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-terrain-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas.sampler),
            },
        ],
    })
}

fn create_selection_outline_gpu(
    device: &wgpu::Device,
    outline: SelectionOutline,
) -> SelectionOutlineGpu {
    let vertices = selection_outline_vertices(outline);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-selection-outline-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    SelectionOutlineGpu {
        outline,
        vertex_buffer,
        vertex_count: vertices.len() as u32,
    }
}

fn selection_outline_vertices(outline: SelectionOutline) -> [SelectionVertex; 24] {
    let min = Vec3::from_array(outline.min).min(Vec3::from_array(outline.max)) - Vec3::splat(0.002);
    let max = Vec3::from_array(outline.min).max(Vec3::from_array(outline.max)) + Vec3::splat(0.002);
    let p000 = [min.x, min.y, min.z];
    let p100 = [max.x, min.y, min.z];
    let p010 = [min.x, max.y, min.z];
    let p110 = [max.x, max.y, min.z];
    let p001 = [min.x, min.y, max.z];
    let p101 = [max.x, min.y, max.z];
    let p011 = [min.x, max.y, max.z];
    let p111 = [max.x, max.y, max.z];

    [
        selection_vertex(p000),
        selection_vertex(p100),
        selection_vertex(p100),
        selection_vertex(p101),
        selection_vertex(p101),
        selection_vertex(p001),
        selection_vertex(p001),
        selection_vertex(p000),
        selection_vertex(p010),
        selection_vertex(p110),
        selection_vertex(p110),
        selection_vertex(p111),
        selection_vertex(p111),
        selection_vertex(p011),
        selection_vertex(p011),
        selection_vertex(p010),
        selection_vertex(p000),
        selection_vertex(p010),
        selection_vertex(p100),
        selection_vertex(p110),
        selection_vertex(p101),
        selection_vertex(p111),
        selection_vertex(p001),
        selection_vertex(p011),
    ]
}

fn selection_vertex(position: [f32; 3]) -> SelectionVertex {
    SelectionVertex { position }
}

fn create_hud_sprite_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<HudSpriteGpu> {
    if width == 0 || height == 0 {
        bail!("hud sprite dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("hud sprite size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "hud sprite has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-hud-sprite-texture"),
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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-hud-sprite-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-hud-sprite-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok(HudSpriteGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        width,
        height,
    })
}

fn push_hud_draw<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
) {
    push_hud_draw_with_uv(vertices, commands, sprite, surface_size, rect, 1.0);
}

fn push_hud_draw_with_uv<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv_max_x: f32,
) {
    let start = vertices.len() as u32;
    vertices.extend_from_slice(&hud_quad_vertices(
        surface_size,
        rect,
        uv_max_x.clamp(0.0, 1.0),
    ));
    let end = vertices.len() as u32;
    commands.push(HudDrawCommand { sprite, start, end });
}

fn centered_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    HudRect {
        x: (surface_size.width.max(1) as f32 - width as f32) * 0.5,
        y: (surface_size.height.max(1) as f32 - height as f32) * 0.5,
        width,
        height,
    }
}

fn hotbar_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_HOTBAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_HOTBAR_HEIGHT as f32,
        width,
        height,
    }
}

fn experience_bar_hud_rect(surface_size: PhysicalSize<u32>, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: (surface_width - HUD_EXPERIENCE_BAR_WIDTH as f32) * 0.5,
        y: surface_height - HUD_EXPERIENCE_MARGIN_BOTTOM - HUD_EXPERIENCE_BAR_HEIGHT as f32,
        width,
        height,
    }
}

fn hud_experience_progress_width(progress: f32) -> u32 {
    ((progress.clamp(0.0, 1.0) * 183.0).floor() as u32).min(HUD_EXPERIENCE_BAR_WIDTH)
}

fn hotbar_selection_hud_rect(
    surface_size: PhysicalSize<u32>,
    selected_slot: u8,
    width: u32,
    height: u32,
) -> HudRect {
    let hotbar = hotbar_hud_rect(surface_size, HUD_HOTBAR_WIDTH, HUD_HOTBAR_HEIGHT);
    HudRect {
        x: hotbar.x - 1.0 + f32::from(selected_slot.min(8)) * 20.0,
        y: hotbar.y - 1.0,
        width,
        height,
    }
}

fn heart_hud_rect(surface_size: PhysicalSize<u32>, index: u32, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 - 91.0 + index as f32 * HUD_HEART_SPACING,
        y: surface_height - 39.0,
        width,
        height,
    }
}

fn food_hud_rect(surface_size: PhysicalSize<u32>, index: u32, width: u32, height: u32) -> HudRect {
    let surface_width = surface_size.width.max(1) as f32;
    let surface_height = surface_size.height.max(1) as f32;
    HudRect {
        x: surface_width * 0.5 + 91.0 - index as f32 * HUD_FOOD_SPACING - width as f32,
        y: surface_height - 39.0,
        width,
        height,
    }
}

fn hud_heart_fill(health: f32, index: u32) -> HudIconFill {
    let current_halves = health.ceil().clamp(0.0, (HUD_HEARTS_PER_ROW * 2) as f32) as u32;
    let start_half = index * 2;
    if start_half >= current_halves {
        HudIconFill::Empty
    } else if start_half + 1 == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Full
    }
}

fn hud_food_fill(food: i32, index: u32) -> HudIconFill {
    let current_halves = food.clamp(0, (HUD_FOOD_ICONS_PER_ROW * 2) as i32) as u32;
    let center_half = index * 2 + 1;
    if center_half < current_halves {
        HudIconFill::Full
    } else if center_half == current_halves {
        HudIconFill::Half
    } else {
        HudIconFill::Empty
    }
}

fn hud_quad_vertices(
    surface_size: PhysicalSize<u32>,
    rect: HudRect,
    uv_max_x: f32,
) -> [HudVertex; 6] {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width as f32;
    let y1 = rect.y + rect.height as f32;
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    let left = x0 / width * 2.0 - 1.0;
    let right = x1 / width * 2.0 - 1.0;
    let top = 1.0 - y0 / height * 2.0;
    let bottom = 1.0 - y1 / height * 2.0;
    [
        HudVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, top],
            uv: [uv_max_x, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: [uv_max_x, 1.0],
        },
        HudVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        HudVertex {
            position: [right, bottom],
            uv: [uv_max_x, 1.0],
        },
        HudVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
    ]
}

fn create_terrain_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<TerrainAtlasGpu> {
    if width == 0 || height == 0 {
        bail!("terrain texture atlas dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("terrain texture atlas size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "terrain texture atlas has {} bytes, expected {} for {}x{} RGBA",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-terrain-texture-atlas"),
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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-terrain-texture-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    Ok(TerrainAtlasGpu {
        _texture: texture,
        view,
        sampler,
    })
}

fn create_terrain_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        "bbb-terrain-pipeline",
        true,
        Some(wgpu::BlendState::REPLACE),
    )
}

fn create_selection_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-selection-outline-shader"),
        source: wgpu::ShaderSource::Wgsl(SELECTION_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selection-outline-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-selection-outline-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[selection_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_hud_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-hud-shader"),
        source: wgpu::ShaderSource::Wgsl(HUD_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-hud-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-hud-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[hud_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_terrain_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_terrain_pipeline_with_options(
        device,
        format,
        camera_bind_group_layout,
        "bbb-terrain-translucent-pipeline",
        false,
        Some(wgpu::BlendState::ALPHA_BLENDING),
    )
}

fn create_terrain_pipeline_with_options(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    depth_write_enabled: bool,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-terrain-shader"),
        source: wgpu::ShaderSource::Wgsl(TERRAIN_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-terrain-pipeline-layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[terrain_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn terrain_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<terrain::TerrainVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &TERRAIN_VERTEX_ATTRIBUTES,
    }
}

fn selection_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<SelectionVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &SELECTION_VERTEX_ATTRIBUTES,
    }
}

fn hud_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<HudVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &HUD_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_quad_vertices_center_sprite_in_ndc() {
        let surface_size = PhysicalSize::new(1280, 720);
        let vertices = hud_quad_vertices(surface_size, centered_hud_rect(surface_size, 16, 8), 1.0);
        assert_f32_near(vertices[0].position[0], -0.0125);
        assert_f32_near(vertices[0].position[1], 0.011111111);
        assert_f32_near(vertices[2].position[0], 0.0125);
        assert_f32_near(vertices[2].position[1], -0.011111111);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn selection_outline_vertices_emit_expanded_box_edges() {
        let vertices = selection_outline_vertices(SelectionOutline {
            min: [1.0, 2.0, 3.0],
            max: [2.0, 3.0, 4.0],
        });
        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[0].position, [0.998, 1.998, 2.998]);
        assert_eq!(vertices[1].position, [2.002, 1.998, 2.998]);
        assert_eq!(vertices[22].position, [0.998, 1.998, 4.002]);
        assert_eq!(vertices[23].position, [0.998, 3.002, 4.002]);
    }

    #[test]
    fn hud_layout_matches_vanilla_hotbar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let hotbar = hotbar_hud_rect(surface_size, 182, 22);
        assert_eq!(hotbar.x, 549.0);
        assert_eq!(hotbar.y, 698.0);
        assert_eq!(hotbar.width, 182);
        assert_eq!(hotbar.height, 22);

        let selection = hotbar_selection_hud_rect(surface_size, 0, 24, 23);
        assert_eq!(selection.x, 548.0);
        assert_eq!(selection.y, 697.0);
        assert_eq!(selection.width, 24);
        assert_eq!(selection.height, 23);

        let last_selection = hotbar_selection_hud_rect(surface_size, 8, 24, 23);
        assert_eq!(last_selection.x, 708.0);
        assert_eq!(last_selection.y, 697.0);
    }

    #[test]
    fn hud_layout_matches_vanilla_experience_bar_positions() {
        let surface_size = PhysicalSize::new(1280, 720);
        let bar = experience_bar_hud_rect(surface_size, 182, 5);
        assert_eq!(bar.x, 549.0);
        assert_eq!(bar.y, 691.0);
        assert_eq!(bar.width, 182);
        assert_eq!(bar.height, 5);

        assert_eq!(hud_experience_progress_width(0.0), 0);
        assert_eq!(hud_experience_progress_width(0.5), 91);
        assert_eq!(hud_experience_progress_width(1.0), 182);
    }

    #[test]
    fn hud_layout_places_player_hearts_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = heart_hud_rect(surface_size, 0, 9, 9);
        let last = heart_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 549.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 621.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_layout_places_food_icons_above_hotbar() {
        let surface_size = PhysicalSize::new(1280, 720);
        let first = food_hud_rect(surface_size, 0, 9, 9);
        let last = food_hud_rect(surface_size, 9, 9, 9);
        assert_eq!(first.x, 722.0);
        assert_eq!(first.y, 681.0);
        assert_eq!(last.x, 650.0);
        assert_eq!(last.y, 681.0);
    }

    #[test]
    fn hud_heart_fill_uses_ceiled_health_halves() {
        assert_eq!(hud_heart_fill(0.0, 0), HudIconFill::Empty);
        assert_eq!(hud_heart_fill(5.0, 0), HudIconFill::Full);
        assert_eq!(hud_heart_fill(5.0, 2), HudIconFill::Half);
        assert_eq!(hud_heart_fill(5.5, 2), HudIconFill::Full);
        assert_eq!(hud_heart_fill(20.0, 9), HudIconFill::Full);
        assert_eq!(hud_heart_fill(25.0, 9), HudIconFill::Full);
    }

    #[test]
    fn hud_food_fill_uses_server_food_halves() {
        assert_eq!(hud_food_fill(0, 0), HudIconFill::Empty);
        assert_eq!(hud_food_fill(5, 0), HudIconFill::Full);
        assert_eq!(hud_food_fill(5, 2), HudIconFill::Half);
        assert_eq!(hud_food_fill(6, 2), HudIconFill::Full);
        assert_eq!(hud_food_fill(20, 9), HudIconFill::Full);
        assert_eq!(hud_food_fill(25, 9), HudIconFill::Full);
    }

    fn assert_f32_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000001,
            "actual {actual} expected {expected}"
        );
    }
}

struct PendingScreenshot {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    format: wgpu::TextureFormat,
    path: std::path::PathBuf,
}

fn choose_format(formats: &[wgpu::TextureFormat]) -> Result<wgpu::TextureFormat> {
    formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb
            )
        })
        .or_else(|| {
            formats.iter().copied().find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
        })
        .ok_or_else(|| anyhow!("surface does not expose an RGBA/BGRA 8-bit format"))
}
