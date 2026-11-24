//! Exact mesh output on a mixed fixture neighborhood, pinned to the hashes the
//! per-layer, 64-byte-vertex mesher produced before the single-pass rewrite.

use crate::terrain::{
    fixture::{TerrainCell, TerrainChunkFixture, TerrainFixtureWorld},
    TerrainCardinalLighting, TerrainFace, TerrainFluid, TerrainFluidKind, TerrainLight,
    TerrainMaterialClass, TerrainQuad, TerrainRenderShape, TerrainSectionMeshLayers,
    TerrainSkipRendering, TerrainTextureAtlas, TerrainTint, TerrainTransparency, TerrainUvRect,
};

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
}

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn fluid_box(height: u8) -> TerrainRenderShape {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    for face in [2, 3, 4, 5] {
        face_uvs[face] = [0, 16 - height, 16, 16];
    }
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: TerrainFace::ALL.map(Some),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn slab() -> TerrainRenderShape {
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 8, 16],
        face_present: [true; 6],
        face_uvs: [
            [0, 0, 16, 16],
            [0, 0, 16, 16],
            [0, 8, 16, 16],
            [0, 8, 16, 16],
            [0, 8, 16, 16],
            [0, 8, 16, 16],
        ],
        face_uv_rotations: [0, 1, 0, 2, 0, 3],
        face_shade: [true; 6],
        face_light_emission: [0, 0, 0, 0, 0, 4],
        face_cull: [
            Some(TerrainFace::Down),
            None,
            Some(TerrainFace::North),
            Some(TerrainFace::South),
            Some(TerrainFace::West),
            Some(TerrainFace::East),
        ],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn tints() -> [TerrainTint; 6] {
    [
        TerrainTint::from_rgb_u8(255, 255, 255),
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59),
        TerrainTint::from_rgb_u8(10, 200, 30),
        TerrainTint::from_rgb_u8(1, 2, 3),
        TerrainTint::from_rgb_u8(254, 128, 7),
        TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4),
    ]
}

fn water(amount: u8, falling: bool) -> TerrainCell {
    let height = ((amount as u16 * 16 + 4) / 9).clamp(1, 16) as u8;
    TerrainCell::with_shape(TerrainMaterialClass::Fluid, 6, fluid_box(height))
        .with_fluid_render_data(
            TerrainFluid::new(TerrainFluidKind::Water, amount, falling),
            [6, 6, 7, 7, 7, 7],
            [TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4); 6],
        )
        .with_fluid_overlay_texture(5)
        .with_face_occlusion_shape_indices([0; 6])
}

fn golden_cell(kind: u64, light: TerrainLight) -> TerrainCell {
    let cell = match kind % 20 {
        0..=5 => TerrainCell::EMPTY,
        6 => TerrainCell::with_texture(TerrainMaterialClass::Opaque, 1)
            .with_ambient_light_facts(true, 15, 51)
            .with_tint(tints()),
        7 => TerrainCell::with_texture(TerrainMaterialClass::Opaque, 2)
            .with_ambient_occlusion(false)
            .with_ambient_light_facts(true, 15, 255),
        8 => TerrainCell::with_texture(TerrainMaterialClass::Cutout, 3)
            .with_face_occlusion_shape_indices([0; 6])
            .with_skip_rendering(TerrainSkipRendering {
                culls_with_leaves: true,
                ..TerrainSkipRendering::NONE
            }),
        9 => TerrainCell::with_texture(TerrainMaterialClass::Translucent, 4)
            .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6])
            .with_face_occlusion_shape_indices([0; 6])
            .with_skip_rendering(TerrainSkipRendering {
                same_block_key: 9,
                same_block_culls_all_faces: true,
                ..TerrainSkipRendering::NONE
            }),
        10 => water(8, false),
        11 => water(3, false),
        12 => water(5, true),
        13 => TerrainCell::with_shape(TerrainMaterialClass::Opaque, 5, slab())
            .with_face_occlusion_shape_indices([1, 0, 0, 0, 0, 0])
            .with_ambient_light_facts(false, 0, 255),
        14 => TerrainCell::with_shape(
            TerrainMaterialClass::Cutout,
            3,
            TerrainRenderShape::Cross {
                shade: false,
                light_emission: 0,
            },
        )
        .with_face_occlusion_shape_indices([0; 6])
        .with_tint([TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59); 6]),
        15 => TerrainCell::with_shape(
            TerrainMaterialClass::Cutout,
            3,
            TerrainRenderShape::Quads(std::sync::Arc::from(vec![
                TerrainQuad {
                    corners: [
                        [0.0, 0.0, 8.0],
                        [0.0, 16.0, 8.0],
                        [16.0, 16.0, 8.0],
                        [16.0, 0.0, 8.0],
                    ],
                    normal: [0.0, 0.0, -1.0],
                    uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                    cull: None,
                    texture_index: 2,
                    tint: TerrainTint::from_rgb_u8(200, 100, 50),
                    transparency: TerrainTransparency {
                        has_transparent: true,
                        has_translucent: false,
                    },
                    shade: true,
                    light_emission: 0,
                },
                TerrainQuad {
                    corners: [
                        [0.0, 0.0, 0.0],
                        [16.0, 0.0, 0.0],
                        [16.0, 0.0, 16.0],
                        [0.0, 0.0, 16.0],
                    ],
                    normal: [0.0, -1.0, 0.0],
                    uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    cull: Some(TerrainFace::Down),
                    texture_index: 1,
                    tint: TerrainTint::WHITE,
                    transparency: TerrainTransparency::OPAQUE,
                    shade: true,
                    light_emission: 0,
                },
            ])),
        )
        .with_face_occlusion_shape_indices([0; 6]),
        16 => TerrainCell::with_shape(TerrainMaterialClass::Opaque, 5, slab())
            .with_face_occlusion_shape_indices([1, 0, 0, 0, 0, 0])
            .with_fluid(TerrainFluid::new(TerrainFluidKind::Water, 8, false))
            .with_fluid_render_data(
                TerrainFluid::new(TerrainFluidKind::Water, 8, false),
                [6, 6, 7, 7, 7, 7],
                [TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4); 6],
            )
            .with_fluid_overlay_texture(5),
        17 => TerrainCell::with_texture(TerrainMaterialClass::Opaque, 2)
            .with_state_light(9, true)
            .with_fluid_overlay_neighbor(),
        18 => TerrainCell::with_shape(TerrainMaterialClass::Fluid, 6, fluid_box(14))
            .with_fluid_render_data(
                TerrainFluid::new(TerrainFluidKind::Lava, 7, false),
                [6, 6, 7, 7, 7, 7],
                [TerrainTint::WHITE; 6],
            )
            .with_face_occlusion_shape_indices([0; 6])
            .with_state_light(15, false),
        _ => TerrainCell::with_texture(TerrainMaterialClass::Cutout, 3).with_face_transparency([
            TerrainTransparency::OPAQUE,
            TerrainTransparency::TRANSLUCENT,
            TerrainTransparency {
                has_transparent: true,
                has_translucent: false,
            },
            TerrainTransparency::OPAQUE,
            TerrainTransparency::OPAQUE,
            TerrainTransparency::OPAQUE,
        ]),
    };
    cell.with_light(light)
}

fn golden_chunks() -> Vec<TerrainChunkFixture> {
    let mut chunks = Vec::new();
    for chunk_z in -1..=1 {
        for chunk_x in -1..=1 {
            let mut seed = 0x9e37_79b9_7f4a_7c15_u64
                ^ ((chunk_x as u64) << 32)
                ^ (chunk_z as u64).wrapping_mul(0x51);
            let height = 32;
            let mut cells = Vec::with_capacity(16 * 16 * height);
            for y in 0..height {
                for _z in 0..16 {
                    for _x in 0..16 {
                        let roll = next(&mut seed);
                        let kind = if y < 6 {
                            6 + roll % 14
                        } else if y < 14 {
                            roll % 20
                        } else {
                            roll % 40
                        };
                        let light = TerrainLight {
                            sky: ((roll >> 20) % 16) as u8,
                            block: ((roll >> 28) % 16) as u8,
                        };
                        cells.push(golden_cell(kind, light));
                    }
                }
            }
            chunks.push(
                TerrainChunkFixture::new(chunk_x, chunk_z, -16, height, cells)
                    .with_cardinal_lighting(if chunk_x == 1 {
                        TerrainCardinalLighting::Nether
                    } else {
                        TerrainCardinalLighting::Default
                    }),
            );
        }
    }
    chunks
}

fn golden_atlas() -> TerrainTextureAtlas {
    TerrainTextureAtlas {
        rects: (0..8)
            .map(|index| {
                let x = (index % 4) as f32 * 0.25;
                let y = (index / 4) as f32 * 0.5;
                TerrainUvRect {
                    min: [x + 0.01, y + 0.02],
                    max: [x + 0.2, y + 0.43],
                }
            })
            .collect(),
        fallback_index: 2,
    }
}

fn hash_layers(layers: &TerrainSectionMeshLayers) -> [u64; 4] {
    let mut out = [0xcbf2_9ce4_8422_2325_u64; 4];
    for (slot, layer) in [&layers.opaque, &layers.cutout, &layers.translucent]
        .into_iter()
        .enumerate()
    {
        for section in layer {
            let hash = &mut out[slot];
            fnv(hash, &section.section_y.to_le_bytes());
            fnv(hash, bytemuck::cast_slice(&section.mesh.vertices));
            fnv(hash, bytemuck::cast_slice(&section.mesh.indices));
            for count in [
                section.mesh.opaque_faces,
                section.mesh.cutout_faces,
                section.mesh.translucent_faces,
                section.mesh.culled_faces,
            ] {
                fnv(hash, &(count as u64).to_le_bytes());
            }
        }
    }
    for visibility in &layers.visibility {
        fnv(&mut out[3], &visibility.section_y.to_le_bytes());
        for from in TerrainFace::ALL {
            for to in TerrainFace::ALL {
                fnv(
                    &mut out[3],
                    &[u8::from(visibility.faces_can_see_each_other(from, to))],
                );
            }
        }
        fnv(
            &mut out[3],
            &[
                u8::from(visibility.has_renderable_layers),
                u8::from(visibility.loaded_empty),
            ],
        );
    }
    out
}

#[test]
fn shared_indices_preserve_oriented_textured_triangles() {
    let world = TerrainFixtureWorld::new(&golden_chunks(), golden_atlas());
    let sections = &[-1, 0, 1, 5];
    let camera = [-3.0, 40.0, 9.0];
    let source = world.build(0, camera, sections);
    let shared = crate::terrain::build_terrain_section_meshes_with_index_order(
        &world.snapshots[0],
        world.snapshots[1..].iter(),
        &world.table,
        camera,
        sections,
        true,
    );
    assert_eq!(source.translucent, shared.translucent);
    assert_eq!(source.visibility, shared.visibility);
    for (before, after) in [
        (&source.opaque, &shared.opaque),
        (&source.cutout, &shared.cutout),
    ] {
        assert_eq!(before.len(), after.len());
        assert!(before
            .iter()
            .any(|section| !section.mesh.indices.is_empty()));
        for (before, after) in before.iter().zip(after) {
            assert_eq!(before.section_y, after.section_y);
            let (before, after) = (&before.mesh, &after.mesh);
            assert!(after.shared_quad_indices && after.indices.is_empty());
            assert_eq!(before.indices.len(), after.index_count());
            let indices: Vec<_> = (0..after.vertices.len() as u32 / 4)
                .flat_map(crate::terrain::sequential_quad_indices)
                .collect();
            for (a, b) in before
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .zip(indices.as_chunks::<3>().0)
            {
                assert!(
                    (0..3).any(|offset| (0..3).all(|i| before.vertices[a[i] as usize]
                        == after.vertices[b[(i + offset) % 3] as usize])),
                    "triangle corners or winding changed"
                );
            }
        }
    }
}

#[test]
fn section_meshes_match_the_pre_rewrite_output_exactly() {
    let world = TerrainFixtureWorld::new(&golden_chunks(), golden_atlas());
    let center = world.build(4, [3.5, 2.0, -7.25], &[-1, 0, 1, 5]);
    assert_eq!(
        [&center.opaque, &center.cutout, &center.translucent].map(|layer| layer
            .iter()
            .map(|section| section.mesh.vertices.len())
            .sum::<usize>()),
        [45128, 25628, 74468]
    );
    assert_eq!(
        hash_layers(&center),
        [
            0x8df83aa33eb537aa,
            0x0b43077399437956,
            0xbb657e6244902884,
            0x8908f668195b36a9,
        ]
    );
    assert_eq!(
        hash_layers(&world.build(4, [-20.0, 40.0, 9.0], &[-1, 0, 1, 5])),
        [
            0x8df83aa33eb537aa,
            0x0b43077399437956,
            0x251698e3a546e1ac,
            0x8908f668195b36a9,
        ]
    );
    let edge_chunks = golden_chunks();
    let edge = TerrainFixtureWorld::new(
        &[
            edge_chunks[0].clone(),
            edge_chunks[1].clone(),
            edge_chunks[3].clone(),
        ],
        golden_atlas(),
    );
    assert_eq!(
        hash_layers(&edge.build(0, [0.0; 3], &[-1, 0])),
        [
            0x222b8e91b845a132,
            0xa2139958e83b3036,
            0x52d50f996a6b5e18,
            0x8908f668195b36a9,
        ]
    );
}
