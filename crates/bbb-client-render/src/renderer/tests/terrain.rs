use super::*;
use crate::gpu::TerrainGpuVertex;
use crate::renderer::{
    translucency_point_of_view, TerrainChunkMeshOwner, TerrainLayer, TerrainSectionMeshOwner,
    TerrainSectionRenderState,
};
use crate::terrain::{TerrainMesh, TerrainSectionMesh, TerrainSectionMeshLayers, TerrainTint};

fn terrain_vertex(position: [f32; 3]) -> TerrainGpuVertex {
    TerrainGpuVertex::pack(position, [0.0; 2], 0, TerrainTint::WHITE, 1.0, 1.0)
}

fn chunk(chunk_x: i32) -> TerrainChunkMeshOwner {
    TerrainChunkMeshOwner {
        level_generation: 7,
        chunk_x,
        chunk_z: 0,
    }
}

fn section(chunk_x: i32, section_y: i32) -> TerrainSectionMeshOwner {
    TerrainSectionMeshOwner {
        chunk: chunk(chunk_x),
        section_y,
    }
}

fn visible(owners: &[TerrainSectionMeshOwner], visibility: f32) -> Vec<TerrainSectionRenderState> {
    owners
        .iter()
        .map(|&owner| TerrainSectionRenderState {
            owner,
            visibility,
            nearby: true,
        })
        .collect()
}

fn triangle_layers(section_y: i32) -> TerrainSectionMeshLayers {
    TerrainSectionMeshLayers {
        opaque: vec![TerrainSectionMesh {
            section_y,
            mesh: TerrainMesh {
                vertices: vec![
                    terrain_vertex([0.0, 0.0, 0.0]),
                    terrain_vertex([1.0, 0.0, 0.0]),
                    terrain_vertex([0.0, 1.0, 0.0]),
                ],
                indices: vec![0, 1, 2],
                source_sections: 1,
                opaque_faces: 1,
                ..TerrainMesh::default()
            },
        }],
        source_sections: 1,
        ..TerrainSectionMeshLayers::default()
    }
}

fn translucent_layers(section_ys: impl IntoIterator<Item = i32>) -> TerrainSectionMeshLayers {
    let quad = |z: f32| {
        [
            terrain_vertex([0.0, 0.0, z]),
            terrain_vertex([1.0, 0.0, z]),
            terrain_vertex([1.0, 1.0, z]),
            terrain_vertex([0.0, 1.0, z]),
        ]
    };
    TerrainSectionMeshLayers {
        translucent: section_ys
            .into_iter()
            .map(|section_y| TerrainSectionMesh {
                section_y,
                mesh: TerrainMesh {
                    vertices: [quad(0.0), quad(1.0)].concat(),
                    indices: vec![4, 5, 6, 6, 7, 4, 0, 1, 2, 2, 3, 0],
                    source_sections: 1,
                    translucent_faces: 2,
                    ..TerrainMesh::default()
                },
            })
            .collect(),
        source_sections: 1,
        ..TerrainSectionMeshLayers::default()
    }
}

fn prepare_test_draws(renderer: &mut Renderer) {
    let (encoder, staging) = renderer.frame_uploads.record(&renderer.device);
    renderer
        .terrain
        .prepare_draws(&renderer.device, encoder, staging);
    renderer.frame_uploads.submit(&renderer.queue);
}

#[test]
fn terrain_visibility_filter_limits_resident_section_draws_and_counter() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let layers = triangle_layers(4);
    let sync = |renderer: &mut Renderer, west_revision| {
        renderer.synchronize_terrain_section_mesh_layer_refs(
            3,
            [
                (chunk(-1), &[(4, west_revision)][..], &layers, [0.0; 3]),
                (chunk(0), &[(4, 1)][..], &layers, [0.0; 3]),
                (chunk(1), &[(4, 1)][..], &layers, [0.0; 3]),
            ],
        );
    };
    sync(&mut renderer, 1);
    renderer.set_terrain_section_states(1, &visible(&[section(0, 4)], 1.0));
    renderer.set_terrain_section_states(2, &visible(&[section(-1, 4)], 1.0));
    renderer.set_terrain_section_states(3, &visible(&[section(0, 4)], 1.0));

    assert_eq!(renderer.counters().uploaded_sections, 3);
    assert_eq!(renderer.counters().visible_sections, 1);
    prepare_test_draws(&mut renderer);
    assert_eq!(
        renderer.terrain.layer_draws(TerrainLayer::Solid),
        [section(0, 4)]
    );

    sync(&mut renderer, 1);
    assert_eq!(
        renderer.counters().visible_sections,
        1,
        "unchanged residency keeps the current graph result"
    );
    sync(&mut renderer, 2);
    assert_eq!(renderer.counters().visible_sections, 1);
    prepare_test_draws(&mut renderer);
    assert_eq!(
        renderer.terrain.layer_draws(TerrainLayer::Solid),
        [section(0, 4)],
        "recompiling a culled owner cannot bypass the current native projection"
    );
}

#[test]
fn opaque_draws_follow_and_translucent_draws_reverse_the_visible_traversal() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let layers = TerrainSectionMeshLayers {
        cutout: triangle_layers(0).opaque,
        translucent: translucent_layers([0]).translucent,
        ..triangle_layers(0)
    };
    let (near, middle, far, hidden) = (section(0, 0), section(1, 0), section(2, 0), section(3, 0));
    // Upload order is deliberately unrelated to the visible traversal.
    for owner in [middle, hidden, far, near] {
        renderer.update_terrain_section_mesh_layer_refs(4, [(owner, 1, &layers, [0.0; 3])], []);
    }
    renderer.set_terrain_section_states(1, &visible(&[near, middle, far], 1.0));
    prepare_test_draws(&mut renderer);
    assert_eq!(
        renderer.terrain.layer_draws(TerrainLayer::Translucent),
        [far, middle, near]
    );
    for layer in [TerrainLayer::Solid, TerrainLayer::Cutout] {
        assert_eq!(renderer.terrain.layer_draws(layer), [near, middle, far]);
    }

    renderer.set_terrain_section_states(2, &visible(&[far, middle, near], 1.0));
    prepare_test_draws(&mut renderer);
    assert_eq!(
        renderer.terrain.layer_draws(TerrainLayer::Translucent),
        [near, middle, far]
    );
    for layer in [TerrainLayer::Solid, TerrainLayer::Cutout] {
        assert_eq!(renderer.terrain.layer_draws(layer), [far, middle, near]);
    }
    renderer.update_terrain_section_mesh_layer_refs(4, [(near, 2, &layers, [0.0; 3])], []);
    prepare_test_draws(&mut renderer);
    for layer in [TerrainLayer::Solid, TerrainLayer::Cutout] {
        assert_eq!(renderer.terrain.layer_draws(layer), [far, middle, near]);
    }
}

#[test]
fn terrain_visibility_filter_changes_deterministic_frame_pixels() {
    for layer in [
        TerrainLayer::Solid,
        TerrainLayer::Cutout,
        TerrainLayer::Translucent,
    ] {
        let Some(mut indirect) = crate::Renderer::new_offscreen(64, 64) else {
            return;
        };
        let Some(mut direct) = crate::Renderer::new_offscreen_with_indirect_draws(64, 64, false)
        else {
            return;
        };
        assert_eq!(
            terrain_visibility_frames(&mut indirect, layer),
            terrain_visibility_frames(&mut direct, layer),
            "{layer:?} indirect and direct submission must produce identical frames",
        );
    }
}

fn terrain_visibility_frames(renderer: &mut Renderer, layer: TerrainLayer) -> Vec<Vec<u8>> {
    renderer.terrain.set_page_size(
        if layer == TerrainLayer::Translucent {
            8
        } else {
            16
        },
        24,
    );
    renderer.config.present_mode = wgpu::PresentMode::Immediate;
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white terrain atlas");
    let vertex = |position| {
        TerrainGpuVertex::pack(
            position,
            [0.5; 2],
            0x00F0_00F0,
            TerrainTint { r: 255, g: 0, b: 0 },
            1.0,
            1.0,
        )
    };
    let mut layers = TerrainSectionMeshLayers {
        opaque: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: TerrainMesh {
                vertices: vec![
                    vertex([-1.0, -1.0, 2.0]),
                    vertex([1.0, -1.0, 2.0]),
                    vertex([1.0, 1.0, 2.0]),
                    vertex([-1.0, 1.0, 2.0]),
                ],
                // Both windings keep this readback independent of the
                // backend's viewport-Y front-face convention.
                indices: vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0],
                source_sections: 1,
                opaque_faces: 2,
                ..TerrainMesh::default()
            },
        }],
        source_sections: 1,
        ..TerrainSectionMeshLayers::default()
    };
    match layer {
        TerrainLayer::Solid => {}
        TerrainLayer::Cutout => {
            layers.cutout = std::mem::take(&mut layers.opaque);
            layers.cutout[0].mesh.opaque_faces = 0;
            layers.cutout[0].mesh.cutout_faces = 2;
        }
        TerrainLayer::Translucent => {
            layers.translucent = std::mem::take(&mut layers.opaque);
            layers.translucent[0].mesh.opaque_faces = 0;
            layers.translucent[0].mesh.translucent_faces = 2;
        }
    }
    let owner = section(0, 0);
    // Reserve slot zero so the visible section's fade requires nonzero first_instance.
    renderer.synchronize_terrain_section_mesh_layer_refs(
        2,
        [
            (chunk(-1), &[(0, 1)][..], &layers, [0.0; 3]),
            (chunk(0), &[(0, 1)][..], &layers, [0.0; 3]),
        ],
    );
    renderer.set_terrain_section_states(1, &visible(&[owner], 1.0));
    renderer.set_terrain_section_states(1, &[]);
    assert!(renderer.terrain.is_visible(&owner));
    let visible_frame = renderer
        .render_offscreen_frame()
        .expect("visible terrain frame");

    renderer.set_terrain_section_states(2, &visible(&[owner], 0.0));
    let faded = renderer
        .render_offscreen_frame()
        .expect("zero-visibility terrain frame");

    renderer.set_terrain_section_states(3, &[]);
    assert_eq!(renderer.terrain.fade(owner), Some(1.0));
    let hidden = renderer
        .render_offscreen_frame()
        .expect("culled terrain frame");

    let visible_center = visible_frame.pixel(32, 32);
    let faded_center = faded.pixel(32, 32);
    let hidden_center = hidden.pixel(32, 32);
    assert!(
        visible_center[0] > visible_center[2],
        "visible owner should draw the red terrain quad, got {visible_center:?}"
    );
    assert!(
        faded_center[0] < visible_center[0],
        "section visibility should mix terrain toward fog color: {faded_center:?} vs {visible_center:?}"
    );
    assert!(
        hidden_center[2] > hidden_center[0],
        "culled owner should expose the blue clear color, got {hidden_center:?}"
    );
    assert_ne!(visible_frame.rgba, hidden.rgba);

    let overlapping = section(-1, -1);
    let adjacent = section(0, -1);
    let first = section(-1, 0);
    let green_for = |target: TerrainSectionMeshOwner| {
        let local_dx = (-target.chunk.chunk_x * 16) as f32;
        let local_dy = (-target.section_y * 16) as f32;
        let green_vertex = |position: [f32; 3]| {
            TerrainGpuVertex::pack(
                [position[0] + local_dx, position[1] + local_dy, position[2]],
                [0.5; 2],
                0x00F0_00F0,
                TerrainTint { r: 0, g: 255, b: 0 },
                1.0,
                1.0,
            )
        };
        let mut vertices = vec![
            green_vertex([-1.0, -1.0, 2.0]),
            green_vertex([1.0, -1.0, 2.0]),
            green_vertex([1.0, 1.0, 2.0]),
            green_vertex([-1.0, 1.0, 2.0]),
        ];
        let mut indices = vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0];
        if layer != TerrainLayer::Translucent {
            vertices.extend([vertices[0], vertices[3], vertices[2], vertices[1]]);
            indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
        }
        let section_mesh = TerrainSectionMesh {
            section_y: target.section_y,
            mesh: TerrainMesh {
                vertices,
                indices,
                source_sections: 1,
                opaque_faces: usize::from(layer == TerrainLayer::Solid) * 2,
                cutout_faces: usize::from(layer == TerrainLayer::Cutout) * 2,
                translucent_faces: usize::from(layer == TerrainLayer::Translucent) * 2,
                ..TerrainMesh::default()
            },
        };
        let mut green = TerrainSectionMeshLayers {
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        match layer {
            TerrainLayer::Solid => green.opaque.push(section_mesh),
            TerrainLayer::Cutout => green.cutout.push(section_mesh),
            TerrainLayer::Translucent => green.translucent.push(section_mesh),
        }
        green
    };
    let green_meshes = [overlapping, adjacent, first].map(|target| (target, green_for(target)));
    renderer.update_terrain_section_mesh_layer_refs(
        4,
        green_meshes
            .iter()
            .map(|(target, green)| (*target, 2, green, [0.0; 3])),
        [],
    );
    let order = if layer == TerrainLayer::Translucent {
        [owner, overlapping, adjacent, first]
    } else {
        [first, adjacent, overlapping, owner]
    };
    renderer.set_terrain_section_states(4, &visible(&order, 1.0));
    let before_rebuild = renderer.render_offscreen_frame().unwrap();
    let center = before_rebuild.pixel(32, 32);
    assert!(
        center[0] > center[1],
        "last visible coplanar quad must be red: {center:?}"
    );
    renderer.update_terrain_section_mesh_layer_refs(2, [(owner, 2, &layers, [0.0; 3])], []);
    let after_rebuild = renderer.render_offscreen_frame().unwrap();
    assert_eq!(before_rebuild.rgba, after_rebuild.rgba);
    renderer.set_terrain_section_states(5, &visible(&[overlapping], 1.0));
    let singleton = renderer.render_offscreen_frame().unwrap();
    let center = singleton.pixel(32, 32);
    assert!(
        center[1] > center[0],
        "single visible owner must draw the green quad: {center:?}"
    );
    renderer.set_terrain_section_states(6, &visible(&order, 1.0));
    let restored = renderer.render_offscreen_frame().unwrap();
    assert_eq!(after_rebuild.rgba, restored.rgba);
    renderer.synchronize_terrain_section_mesh_layer_refs(0, []);
    assert_eq!(renderer.counters().resident_bytes, 0);
    vec![
        visible_frame.rgba,
        faded.rgba,
        hidden.rgba,
        before_rebuild.rgba,
        after_rebuild.rgba,
        singleton.rgba,
        restored.rgba,
    ]
}

#[test]
fn terrain_arena_reuses_freed_ranges_and_keeps_geometry_across_growth() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(CameraPose {
        position: [0.0, 0.0, -3.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white terrain atlas");
    let vertex = |position| {
        TerrainGpuVertex::pack(
            position,
            [0.5; 2],
            0x00F0_00F0,
            TerrainTint { r: 255, g: 0, b: 0 },
            1.0,
            1.0,
        )
    };
    let quad = TerrainSectionMeshLayers {
        opaque: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: TerrainMesh {
                vertices: vec![
                    vertex([-0.75, -0.75, 0.0]),
                    vertex([0.75, -0.75, 0.0]),
                    vertex([0.75, 0.75, 0.0]),
                    vertex([-0.75, 0.75, 0.0]),
                ],
                indices: vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0],
                source_sections: 1,
                opaque_faces: 2,
                ..TerrainMesh::default()
            },
        }],
        source_sections: 1,
        ..TerrainSectionMeshLayers::default()
    };
    let (red, small) = (section(0, 0), section(1, 0));
    renderer.update_terrain_section_mesh_layer_refs(1, [(red, 1, &quad, [0.0; 3])], []);
    renderer.update_terrain_section_mesh_layer_refs(
        2,
        [(small, 1, &triangle_layers(0), [0.0; 3])],
        [],
    );
    let small_range = renderer.terrain.mesh_range(small, TerrainLayer::Solid);
    renderer.update_terrain_section_mesh_layer_refs(1, [], [small]);
    renderer.update_terrain_section_mesh_layer_refs(
        2,
        [(small, 2, &triangle_layers(0), [0.0; 3])],
        [],
    );
    assert_eq!(
        renderer.terrain.mesh_range(small, TerrainLayer::Solid),
        small_range,
        "a freed range is reused first-fit"
    );

    let (buffers_before, indices_before) = renderer
        .terrain
        .layer_buffers(TerrainLayer::Solid, red)
        .expect("resident solid arena");
    // Degenerate geometry far behind the camera; large enough to force both u16 arenas on `red`'s page to grow.
    let mut big_owners = Vec::new();
    for x in 2..6 {
        let owner = section(x, 0);
        let chunk_layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 0,
                mesh: TerrainMesh {
                    vertices: vec![vertex([0.0, 0.0, -8.0]); 50_000],
                    indices: vec![0; 75_000],
                    source_sections: 1,
                    opaque_faces: 1,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        renderer.update_terrain_section_mesh_layer_refs(
            3 + (x - 2) as usize,
            [(owner, 1, &chunk_layers, [0.0; 3])],
            [],
        );
        big_owners.push(owner);
    }
    let (buffers_after, indices_after) = renderer
        .terrain
        .layer_buffers(TerrainLayer::Solid, red)
        .expect("resident solid arena");
    assert_ne!(buffers_before, buffers_after, "the vertex arena grew");
    assert_ne!(indices_before, indices_after, "the index arena grew");
    let mut visible_owners = vec![red];
    visible_owners.extend_from_slice(&big_owners);
    renderer.set_terrain_section_states(1, &visible(&visible_owners, 1.0));
    let pixel = renderer
        .render_offscreen_frame()
        .expect("terrain frame")
        .pixel(16, 16);
    assert!(
        pixel[0] > pixel[2],
        "geometry uploaded before growth must survive the copy, got {pixel:?}"
    );
}

#[test]
fn a_fragmented_arena_at_its_size_limit_compacts_and_a_failed_upload_retries() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(CameraPose {
        position: [0.0, 0.0, -3.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white terrain atlas");
    renderer.terrain.limit_arenas(32, u32::MAX);
    let red = |position| {
        TerrainGpuVertex::pack(
            position,
            [0.5; 2],
            0x00F0_00F0,
            TerrainTint { r: 255, g: 0, b: 0 },
            1.0,
            1.0,
        )
    };
    // `count` vertices; only a red quad in front of the camera when `quad` is set.
    let layers = |count: usize, quad: bool| {
        let mut vertices = vec![terrain_vertex([0.0, 0.0, -8.0]); count];
        let mut indices = vec![0, 0, 0];
        if quad {
            vertices[..4].copy_from_slice(&[
                red([-0.75, -0.75, 0.0]),
                red([0.75, -0.75, 0.0]),
                red([0.75, 0.75, 0.0]),
                red([-0.75, 0.75, 0.0]),
            ]);
            indices = vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0];
        }
        TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y: 0,
                mesh: TerrainMesh {
                    vertices,
                    indices,
                    source_sections: 1,
                    opaque_faces: 2,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        }
    };
    let [a, b, c, d, e, f] = [-1, 0, 1, 2, 3, 4].map(|x| section(x, 0));
    let quarter = layers(8, false);
    let quad = layers(8, true);
    for (owner, layers) in [(a, &quarter), (b, &quad), (c, &quarter), (d, &quarter)] {
        renderer.update_terrain_section_mesh_layer_refs(1, [(owner, 1, layers, [0.0; 3])], []);
    }
    assert_eq!(
        renderer.terrain.mesh_range(d, TerrainLayer::Solid),
        Some((24, 18))
    );
    renderer.update_terrain_section_mesh_layer_refs(1, [], [a, c]);

    // 12 of 32 vertices fit only after the two freed quarters are packed together.
    let three_eighths = layers(12, false);
    renderer.update_terrain_section_mesh_layer_refs(1, [(e, 1, &three_eighths, [0.0; 3])], []);
    assert_eq!(
        renderer.terrain.mesh_range(b, TerrainLayer::Solid),
        Some((0, 3))
    );
    assert_eq!(
        renderer.terrain.mesh_range(d, TerrainLayer::Solid),
        Some((8, 18))
    );
    assert_eq!(
        renderer.terrain.mesh_range(e, TerrainLayer::Solid),
        Some((16, 0))
    );
    assert_eq!(renderer.counters().terrain_failed_owners, 0);
    renderer.set_terrain_section_states(1, &visible(&[b, d, e], 1.0));
    let pixel = renderer
        .render_offscreen_frame()
        .expect("terrain frame")
        .pixel(16, 16);
    assert!(
        pixel[0] > pixel[2],
        "geometry moved by the compaction must still draw, got {pixel:?}"
    );

    // 16 vertices do not fit the 4 left: the owner stays unrecorded and retries after a release.
    let large = layers(16, false);
    renderer.update_terrain_section_mesh_layer_refs(1, [(f, 1, &large, [0.0; 3])], []);
    assert_eq!(renderer.terrain.mesh_range(f, TerrainLayer::Solid), None);
    assert_eq!(renderer.counters().terrain_failed_owners, 1);
    assert!(renderer.terrain_retry_owners().is_empty());
    renderer.update_terrain_section_mesh_layer_refs(1, [], [e]);
    assert_eq!(renderer.terrain_retry_owners(), vec![f]);
    renderer.update_terrain_section_mesh_layer_refs(1, [(f, 1, &large, [0.0; 3])], []);
    assert_eq!(
        renderer.terrain.mesh_range(f, TerrainLayer::Solid),
        Some((16, 0))
    );
    assert_eq!(renderer.counters().terrain_failed_owners, 0);
    assert!(renderer.terrain_retry_owners().is_empty());
}

#[test]
fn incremental_terrain_residency_replaces_only_changed_section_owners() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let layers = triangle_layers(4);
    renderer.synchronize_terrain_section_mesh_layer_refs(
        2,
        [
            (chunk(-1), &[(4, 1)][..], &layers, [0.0; 3]),
            (chunk(0), &[(4, 1)][..], &layers, [0.0; 3]),
        ],
    );
    let one_owner_bytes = renderer.counters().upload_bytes / 2;
    assert_eq!(renderer.counters().terrain_changed_owners, 2);
    assert_eq!(renderer.counters().terrain_uploaded_owners, 2);
    assert_eq!(renderer.counters().terrain_vertices, 6);
    let east_range = renderer
        .terrain
        .mesh_range(section(0, 4), TerrainLayer::Solid);

    renderer.synchronize_terrain_section_mesh_layer_refs(
        2,
        [
            (chunk(-1), &[(4, 1)][..], &layers, [0.0; 3]),
            (chunk(0), &[(4, 1)][..], &layers, [0.0; 3]),
        ],
    );
    assert_eq!(renderer.counters().terrain_changed_owners, 0);
    assert_eq!(renderer.counters().terrain_uploaded_owners, 0);
    assert_eq!(renderer.counters().terrain_removed_owners, 0);
    assert_eq!(renderer.counters().upload_bytes, 0);

    renderer.synchronize_terrain_section_mesh_layer_refs(
        2,
        [
            (chunk(-1), &[(4, 2)][..], &layers, [0.0; 3]),
            (chunk(0), &[(4, 1)][..], &layers, [0.0; 3]),
        ],
    );
    assert_eq!(renderer.counters().terrain_changed_owners, 1);
    assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
    assert_eq!(renderer.counters().terrain_removed_owners, 1);
    assert_eq!(renderer.counters().upload_bytes, one_owner_bytes);
    assert_eq!(renderer.counters().terrain_vertices, 6);
    assert_eq!(
        renderer
            .terrain
            .mesh_range(section(0, 4), TerrainLayer::Solid),
        east_range,
        "an unchanged sibling keeps its arena range"
    );

    renderer.synchronize_terrain_section_mesh_layer_refs(
        1,
        [(chunk(0), &[(4, 1)][..], &layers, [0.0; 3])],
    );
    assert_eq!(renderer.counters().terrain_changed_owners, 0);
    assert_eq!(renderer.counters().terrain_removed_owners, 1);
    assert_eq!(renderer.counters().terrain_vertices, 3);
    assert_eq!(renderer.terrain.revision(&section(-1, 4)), None);
    assert_eq!(renderer.terrain.revision(&section(0, 4)), Some(1));

    let next_level = TerrainChunkMeshOwner {
        level_generation: 8,
        ..chunk(0)
    };
    renderer.synchronize_terrain_section_mesh_layer_refs(
        1,
        [(next_level, &[(4, 1)][..], &layers, [0.0; 3])],
    );
    assert_eq!(renderer.counters().terrain_changed_owners, 1);
    assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
    assert_eq!(renderer.counters().terrain_removed_owners, 1);
}

#[test]
fn recompiled_and_removed_owners_keep_the_native_fade_projection() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let layers = triangle_layers(4);
    let (west, east) = (section(-1, 4), section(0, 4));
    renderer.synchronize_terrain_section_mesh_layer_refs(
        2,
        [
            (chunk(-1), &[(4, 1)][..], &layers, [0.0; 3]),
            (chunk(0), &[(4, 1)][..], &layers, [0.0; 3]),
        ],
    );
    assert_eq!(
        renderer.counters().visible_sections,
        0,
        "new residency waits for the native membership/fade projection"
    );
    renderer.set_terrain_section_states(1, &visible(&[east], 0.4));

    renderer.update_terrain_section_mesh_layer_refs(2, [(west, 2, &layers, [0.0; 3])], []);
    assert_eq!(renderer.counters().terrain_changed_owners, 1);
    assert_eq!(renderer.counters().terrain_uploaded_owners, 1);
    assert_eq!(renderer.counters().terrain_removed_owners, 1);
    assert_eq!(renderer.counters().visible_sections, 1);
    assert_eq!(renderer.terrain.fade(east), Some(0.4));
    assert_eq!(renderer.terrain.revision(&east), Some(1));

    renderer.update_terrain_section_mesh_layer_refs(2, [(east, 2, &layers, [0.0; 3])], []);
    assert_eq!(renderer.counters().visible_sections, 1);
    assert_eq!(renderer.terrain.fade(east), Some(0.4));

    renderer.update_terrain_section_mesh_layer_refs(1, [], [east]);
    assert_eq!(renderer.counters().terrain_changed_owners, 0);
    assert_eq!(renderer.counters().terrain_removed_owners, 1);
    assert_eq!(renderer.counters().visible_sections, 0);
    renderer.update_terrain_section_mesh_layer_refs(2, [(east, 3, &layers, [0.0; 3])], []);
    assert_eq!(
        renderer.terrain.fade(east),
        Some(1.0),
        "a re-admitted owner starts unfaded and culled"
    );

    renderer.set_terrain_section_states(3, &visible(&[east], 0.4));
    let replacement = section(1, 4);
    renderer.update_terrain_section_mesh_layer_refs(
        2,
        [(replacement, 1, &layers, [0.0; 3])],
        [east],
    );
    prepare_test_draws(&mut renderer);
    assert!(renderer.terrain.layer_draws(TerrainLayer::Solid).is_empty());
    assert_eq!(renderer.terrain.fade(replacement), Some(1.0));

    renderer.set_terrain_section_states(4, &visible(&[west], 0.4));
    type SynchronizedSectionLayerRef<'a> = (
        TerrainChunkMeshOwner,
        &'a [(i32, u64)],
        &'a TerrainSectionMeshLayers,
        [f32; 3],
    );
    let no_chunks: [SynchronizedSectionLayerRef<'_>; 0] = [];
    renderer.synchronize_terrain_section_mesh_layer_refs(0, no_chunks);
    assert_eq!(renderer.counters().visible_sections, 0);
    assert_eq!(renderer.counters().uploaded_sections, 0);
    assert_eq!(renderer.counters().resident_bytes, 0);
}

#[test]
fn translucent_camera_resort_uploads_indices_without_replacing_vertices() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let owner = section(0, 0);
    renderer.synchronize_terrain_section_mesh_layer_refs(
        1,
        [(
            chunk(0),
            &[(0, 1)][..],
            &translucent_layers([0]),
            [0.5, 0.5, -4.0],
        )],
    );
    renderer.set_terrain_section_states(1, &visible(&[owner], 1.0));
    let vertex_upload_bytes = renderer.counters().terrain_vertex_upload_bytes;

    renderer.set_camera_pose(Some(CameraPose {
        position: [0.5, 0.5, 4.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));

    assert_eq!(renderer.counters().terrain_translucent_resort_owners, 1);
    assert_eq!(
        renderer.counters().terrain_translucent_resort_index_bytes,
        12 * size_of::<u16>() as u64
    );
    assert_eq!(
        renderer.counters().terrain_vertex_upload_bytes,
        vertex_upload_bytes
    );
    assert_eq!(renderer.counters().terrain_changed_owners, 1);
}

#[test]
fn translucent_camera_resort_drains_block_boundary_work_over_updates() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    let revisions = [(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)];
    renderer.synchronize_terrain_section_mesh_layer_refs(
        5,
        [(
            chunk(0),
            &revisions[..],
            &translucent_layers(0..5),
            [0.5, 32.5, -4.0],
        )],
    );
    let owners: Vec<_> = (0..5).map(|section_y| section(0, section_y)).collect();
    renderer.set_terrain_section_states(1, &visible(&owners, 1.0));

    let mut completed = 0;
    for x in [0.5, 0.6, 0.7, 0.8, 0.9, 0.95] {
        renderer.set_camera_pose(Some(CameraPose {
            position: [x, 32.5, 4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        let resorts = renderer.counters().terrain_translucent_resort_owners;
        assert!(resorts <= 1, "one resort per camera update");
        completed += resorts;
    }
    assert_eq!(completed, 5, "every scheduled section drains exactly once");
}

#[test]
fn translucent_point_of_view_matches_section_coordinates() {
    let owner = TerrainSectionMeshOwner {
        chunk: TerrainChunkMeshOwner {
            level_generation: 1,
            chunk_x: -2,
            chunk_z: 4,
        },
        section_y: 7,
    };
    assert_eq!(
        translucency_point_of_view([-31.5, 127.9, 64.0], owner),
        [0, 0, 0]
    );
    assert_eq!(
        translucency_point_of_view([100.0, 0.0, 0.0], owner),
        [1, -1, -1]
    );
}

#[test]
fn terrain_renders_identically_at_30m_world_border_coordinates() {
    let render_at = |chunk_x: i32, section_y: i32, chunk_z: i32| -> Option<Vec<u8>> {
        let mut renderer = crate::Renderer::new_offscreen(48, 48)?;
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(CameraPose {
            position: [
                (chunk_x * 16) as f32,
                (section_y * 16) as f32,
                (chunk_z * 16) as f32,
            ],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer
            .upload_terrain_texture_atlas(1, 1, &[255; 4])
            .expect("white terrain atlas");
        let vertex = |position| {
            TerrainGpuVertex::pack(
                position,
                [0.5; 2],
                0x00F0_00F0,
                TerrainTint {
                    r: 220,
                    g: 96,
                    b: 32,
                },
                1.0,
                1.0,
            )
        };
        let owner = TerrainSectionMeshOwner {
            chunk: TerrainChunkMeshOwner {
                level_generation: 1,
                chunk_x,
                chunk_z,
            },
            section_y,
        };
        let layers = TerrainSectionMeshLayers {
            opaque: vec![TerrainSectionMesh {
                section_y,
                mesh: TerrainMesh {
                    // Sub-block coordinates (`-0.375..0.375`) collapse to zero area under f32
                    // world-space positions at 30,000,000 (`1 ULP == 2.0`), so matching origin
                    // pixels proves exact integer section-origin subtraction in the vertex shader.
                    vertices: vec![
                        vertex([-0.375, -0.375, 2.0]),
                        vertex([0.375, -0.375, 2.0]),
                        vertex([0.375, 0.375, 2.0]),
                        vertex([-0.375, 0.375, 2.0]),
                    ],
                    indices: vec![0, 1, 2, 2, 3, 0, 0, 3, 2, 2, 1, 0],
                    source_sections: 1,
                    opaque_faces: 2,
                    ..TerrainMesh::default()
                },
            }],
            source_sections: 1,
            ..TerrainSectionMeshLayers::default()
        };
        renderer.update_terrain_section_mesh_layer_refs(1, [(owner, 1, &layers, [0.0; 3])], []);
        renderer.set_terrain_section_states(1, &visible(&[owner], 1.0));
        let frame = renderer.render_offscreen_frame().expect("terrain frame");
        assert!(frame.pixel(24, 24)[0] > 180);
        Some(frame.rgba)
    };
    let Some(origin_rgba) = render_at(0, 0, 0) else {
        return;
    };
    let border_rgba = render_at(1_875_000, 20, -1_875_000).expect("world-border terrain frame");
    assert_eq!(origin_rgba, border_rgba);
}

#[test]
fn large_sections_over_65536_vertices_fall_back_to_u32_terrain_indices() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer.set_camera_pose(Some(CameraPose {
        position: [0.0, 0.0, -3.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    }));
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .expect("white terrain atlas");
    let red = |position| {
        TerrainGpuVertex::pack(
            position,
            [0.5; 2],
            0x00F0_00F0,
            TerrainTint { r: 255, g: 0, b: 0 },
            1.0,
            1.0,
        )
    };
    // 65,540 vertices > u16::MAX+1: the final visible quad uses vertex indices 65,536..=65,539.
    let mut vertices = vec![terrain_vertex([0.0, 0.0, -8.0]); 65_540];
    vertices[65_536..].copy_from_slice(&[
        red([-0.75, -0.75, 0.0]),
        red([0.75, -0.75, 0.0]),
        red([0.75, 0.75, 0.0]),
        red([-0.75, 0.75, 0.0]),
    ]);
    let owner = section(0, 0);
    let layers = TerrainSectionMeshLayers {
        translucent: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: TerrainMesh {
                vertices,
                indices: vec![
                    65_536, 65_537, 65_538, 65_538, 65_539, 65_536, 65_536, 65_539, 65_538, 65_538,
                    65_537, 65_536,
                ],
                source_sections: 1,
                translucent_faces: 1,
                ..TerrainMesh::default()
            },
        }],
        source_sections: 1,
        ..TerrainSectionMeshLayers::default()
    };
    renderer.update_terrain_section_mesh_layer_refs(1, [(owner, 1, &layers, [0.0; 3])], []);
    assert_eq!(
        renderer.counters().terrain_index_upload_bytes,
        12 * size_of::<u32>() as u64
    );
    renderer.set_terrain_section_states(1, &visible(&[owner], 1.0));
    let pixel = renderer
        .render_offscreen_frame()
        .expect("terrain frame")
        .pixel(16, 16);
    assert!(
        pixel[0] > pixel[2],
        "quad at indices 65,536..=65,539 must render via u32 fallback, got {pixel:?}"
    );
}
