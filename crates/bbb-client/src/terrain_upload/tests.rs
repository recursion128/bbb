use super::*;
use bbb_client_render::terrain::{TerrainMesh, TerrainSectionMesh};
use bbb_client_world::{
    ChunkColumn, ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain,
    PaletteKind, PalettedContainerData, WorldDimension,
};
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, BlockUpdate, SectionBlocksUpdate, SetChunkCacheCenter, Vec3d,
};

#[test]
fn texture_animation_tick_advances_at_client_tick_interval() {
    let mut upload = TerrainUploadState::default();
    let start = Instant::now();

    assert_eq!(advance_texture_animation_tick(&mut upload, start), None);
    assert_eq!(
        advance_texture_animation_tick(&mut upload, start + Duration::from_millis(49)),
        None
    );
    assert_eq!(
        advance_texture_animation_tick(&mut upload, start + Duration::from_millis(50)),
        Some((0, 1))
    );
    assert_eq!(
        advance_texture_animation_tick(&mut upload, start + Duration::from_millis(149)),
        Some((1, 2))
    );
    assert_eq!(
        advance_texture_animation_tick(&mut upload, start + Duration::from_millis(250)),
        Some((2, 5))
    );
    assert_eq!(
        advance_texture_animation_tick(&mut upload, start + Duration::from_millis(299)),
        None
    );
}

#[test]
fn occlusion_rebuild_key_matches_vanilla_eight_block_invalidation() {
    let world = WorldStore::new();
    let mut upload = TerrainUploadState {
        next_resident_revision: 7,
        ..Default::default()
    };
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [0.25, 64.0, 7.75],
            y_rot: 10.0,
            x_rot: -5.0,
            eye_height: 1.5,
        }),
        [1280, 720],
        70.0,
        12,
        true,
    );
    let key = terrain_occlusion_rebuild_key(&world, &upload, context, [0.25, 65.5, 7.75]).unwrap();

    let rotated_and_resized = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [7.99, 64.25, 0.01],
            y_rot: 179.0,
            x_rot: 45.0,
            eye_height: 1.25,
        }),
        [720, 1280],
        110.0,
        12,
        true,
    );
    assert_eq!(
        terrain_occlusion_rebuild_key(&world, &upload, rotated_and_resized, [7.99, 65.5, 0.01],),
        Some(key),
        "rotation, FOV and surface changes only require per-frame frustum traversal",
    );

    let crossed_cell =
        terrain_occlusion_rebuild_key(&world, &upload, context, [8.0, 65.5, 7.75]).unwrap();
    assert_ne!(crossed_cell, key);

    upload.next_resident_revision += 1;
    assert_eq!(
        terrain_occlusion_rebuild_key(&world, &upload, context, [0.25, 65.5, 7.75]),
        Some(key),
        "Vanilla schedules propagation from recompiled sections instead of invalidating the graph",
    );
    let different_view = TerrainVisibilityContext {
        render_distance_chunks: 13,
        ..context
    };
    assert_ne!(
        terrain_occlusion_rebuild_key(
            &world,
            &TerrainUploadState {
                next_resident_revision: 7,
                ..Default::default()
            },
            different_view,
            [0.25, 65.5, 7.75],
        ),
        Some(key),
    );
    assert!(
        terrain_occlusion_rebuild_key(&world, &upload, context, [f64::NAN, 65.5, 7.75],).is_none()
    );
}

#[test]
fn upload_tail_reuses_visibility_already_published_for_the_frame() {
    let world = WorldStore::new();
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [4.0, 64.0, 4.0],
            y_rot: 35.0,
            x_rot: -10.0,
            eye_height: 1.62,
        }),
        [1280, 720],
        70.0,
        12,
        true,
    );
    let mut upload = TerrainUploadState::default();
    let eye = [4.0, 65.62, 4.0];
    upload.occlusion_rebuild_key = terrain_occlusion_rebuild_key(&world, &upload, context, eye);
    upload.occlusion_graph_input_key = Some(terrain_occlusion_input_key(&world, &upload));

    assert!(!refresh_terrain_visibility(&world, &mut upload, context));

    upload.occlusion_rebuild_key = None;
    assert!(refresh_terrain_visibility(&world, &mut upload, context));
    assert!(upload.pending_occlusion_rebuild_key.is_some());

    while upload.occlusion_rebuild_key.is_none() {
        refresh_terrain_visibility(&world, &mut upload, context);
        std::thread::yield_now();
    }
    assert!(upload.pending_occlusion_rebuild_key.is_none());
    assert!(!refresh_terrain_visibility(&world, &mut upload, context));
}

#[test]
fn level_replacement_drains_stale_occlusion_job_before_submitting_replacement() {
    let mut world = WorldStore::new();
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [4.0, 64.0, 4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        [1280, 720],
        70.0,
        12,
        true,
    );
    let mut upload = TerrainUploadState::default();

    assert!(refresh_terrain_visibility(&world, &mut upload, context));
    let stale_key = upload
        .pending_occlusion_rebuild_key
        .expect("initial occlusion job must be in flight");
    assert_eq!(stale_key.level_generation, 0);

    world.clear_client_level();
    synchronize_terrain_level_generation(&mut upload, world.client_level_generation());
    assert_eq!(
        upload.observed_level_generation,
        world.client_level_generation()
    );
    assert_eq!(
        upload.pending_occlusion_rebuild_key,
        Some(stale_key),
        "level replacement must retain ownership of the actual worker job"
    );
    assert_eq!(upload.pending_occlusion_input_key, None);

    while upload.pending_occlusion_rebuild_key == Some(stale_key) {
        refresh_terrain_visibility(&world, &mut upload, context);
        std::thread::yield_now();
    }
    let replacement_key = upload
        .pending_occlusion_rebuild_key
        .expect("draining the stale result must submit the replacement job");
    assert_eq!(
        replacement_key.level_generation,
        world.client_level_generation()
    );
    assert_eq!(upload.occlusion_rebuild_key, None);

    while upload.pending_occlusion_rebuild_key.is_some() {
        refresh_terrain_visibility(&world, &mut upload, context);
        std::thread::yield_now();
    }
    assert_eq!(upload.occlusion_rebuild_key, Some(replacement_key));
}

#[test]
fn same_key_reload_discards_invalidated_occlusion_result() {
    let world = WorldStore::new();
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [4.0, 64.0, 4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        [1280, 720],
        70.0,
        12,
        true,
    );
    let mut upload = TerrainUploadState::default();

    assert!(refresh_terrain_visibility(&world, &mut upload, context));
    let invalidated_key = upload
        .pending_occlusion_rebuild_key
        .expect("initial occlusion job must be in flight");
    clear_terrain_occlusion_inputs(&mut upload);
    assert_eq!(upload.pending_occlusion_rebuild_key, Some(invalidated_key));
    assert_eq!(upload.pending_occlusion_input_key, None);

    while upload.pending_occlusion_input_key.is_none() {
        refresh_terrain_visibility(&world, &mut upload, context);
        std::thread::yield_now();
    }
    assert_eq!(
        upload.pending_occlusion_rebuild_key,
        Some(invalidated_key),
        "same-key invalidation must replace, rather than publish, the stale job"
    );
    assert_eq!(upload.occlusion_rebuild_key, None);

    while upload.pending_occlusion_rebuild_key.is_some() {
        refresh_terrain_visibility(&world, &mut upload, context);
        std::thread::yield_now();
    }
    assert_eq!(upload.occlusion_rebuild_key, Some(invalidated_key));
}

#[test]
fn full_graph_does_not_restore_an_empty_visit_after_section_reinstallation() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let mut upload = TerrainUploadState::default();
    let camera = [8.0; 3];
    let context = |yaw| {
        TerrainVisibilityContext::new(
            Some(CameraPose {
                position: camera,
                x_rot: 0.0,
                y_rot: yaw,
                eye_height: 0.0,
            }),
            [1280, 720],
            70.0,
            5,
            true,
        )
    };
    for x in -1..=1 {
        for z in -3..=1 {
            world.insert_decoded_chunk(test_chunk(ChunkPos { x, z }));
        }
    }
    let empty = TerrainSectionVisibility::all_visible(0, false, true);
    let now = Instant::now();
    for z in -2..=0 {
        install_resident_section(&mut upload, ChunkPos { x: 0, z }, empty, camera, now);
    }
    refresh_terrain_visibility(&world, &mut upload, context(0.0));
    assert!(upload.pending_occlusion_input_key.is_some());
    let far = SectionPosition { x: 0, y: 0, z: -2 };
    let actor = bbb_client_world::EntityVec3 {
        x: 8.0,
        y: 8.0,
        z: -24.0,
    };
    remove_resident_section(&mut upload, far);
    install_resident_section(&mut upload, far.chunk(), empty, camera, now);
    install_resident_section(
        &mut upload,
        far.chunk(),
        TerrainSectionVisibility::all_visible(0, true, false),
        camera,
        now,
    );
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor));

    while upload.pending_occlusion_rebuild_key.is_some() {
        refresh_terrain_visibility(&world, &mut upload, context(0.0));
        std::thread::yield_now();
    }
    assert!(upload
        .occlusion_graph
        .visited_empty(SectionPosition { z: -1, ..far }));
    assert!(!upload.occlusion_graph.visited_empty(far));
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor));
    upload.refresh_frame_visibility(context(180.0));
    assert!(upload.is_entity_section_compiled_and_visible(&world, actor));
}

#[test]
fn published_partial_graph_filters_sections_without_resident_meshes() {
    let resident_pos = ChunkPos { x: 0, z: 0 };
    let mut upload = TerrainUploadState {
        observed_level_generation: 4,
        selected_chunks: vec![resident_pos],
        ..Default::default()
    };
    upload.meshes.insert(
        resident_pos,
        CachedTerrainMesh {
            revision: 1,
            section_mesh_revisions: vec![(0, 1)],
            resident_revisions: vec![(0, 1)],
            camera_position: [0.0; 3],
            layers: TerrainSectionMeshLayers::default(),
        },
    );
    upload.section_residency.insert(
        SectionPosition { x: 0, y: 0, z: 0 },
        TerrainSectionResidency {
            uploaded_at: Instant::now(),
            fade_duration: Duration::ZERO,
            compiled_visibility: TerrainSectionVisibility::all_visible(0, true, false),
        },
    );
    let loaded = (-2..=2)
        .flat_map(|z| (-2..=2).map(move |x| ChunkPos { x, z }))
        .collect::<TerrainCoordinateSet<_>>();
    upload.occlusion_graph.rebuild(
        [
            ResidentSection {
                position: SectionPosition { x: 0, y: 0, z: 0 },
                visibility: TerrainSectionVisibility::all_visible(0, true, false),
            },
            ResidentSection {
                position: SectionPosition { x: 0, y: 0, z: 1 },
                visibility: TerrainSectionVisibility::all_visible(0, true, false),
            },
        ],
        &loaded,
        [8.0, 8.0, 8.0],
        2,
        0,
        1,
        false,
    );
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [8.0, 0.0, 8.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 8.0,
        }),
        [1280, 720],
        120.0,
        2,
        false,
    );

    publish_terrain_frustum_visibility(&mut upload, context);

    assert_eq!(
        upload.renderer_sections.states,
        [TerrainSectionRenderState {
            owner: TerrainSectionMeshOwner {
                chunk: TerrainChunkMeshOwner {
                    level_generation: 4,
                    chunk_x: 0,
                    chunk_z: 0,
                },
                section_y: 0,
            },
            visibility: 1.0,
            nearby: true,
        }]
    );
}

#[test]
fn frame_visibility_matches_vanilla_two_degree_rotation_gate() {
    let mut upload = TerrainUploadState::default();
    let context = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [4.0, 64.0, 4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 1.62,
        }),
        [1280, 720],
        70.0,
        12,
        true,
    );
    assert!(upload.refresh_frame_visibility(context));
    assert!(!upload.refresh_frame_visibility(context));

    let within_camera_cell = TerrainVisibilityContext::new(
        Some(CameraPose {
            position: [7.5, 64.0, 7.5],
            y_rot: 1.99,
            ..context.camera_pose.unwrap()
        }),
        context.surface_size,
        context.camera_fov_degrees,
        context.render_distance_chunks,
        context.smart_cull,
    );
    assert!(!upload.refresh_frame_visibility(within_camera_cell));

    let next_rotation_bucket = TerrainVisibilityContext::new(
        Some(CameraPose {
            y_rot: 2.0,
            ..within_camera_cell.camera_pose.unwrap()
        }),
        within_camera_cell.surface_size,
        within_camera_cell.camera_fov_degrees,
        within_camera_cell.render_distance_chunks,
        within_camera_cell.smart_cull,
    );
    assert!(upload.refresh_frame_visibility(next_rotation_bucket));

    let resized = TerrainVisibilityContext::new(
        next_rotation_bucket.camera_pose,
        [1920, 1080],
        next_rotation_bucket.camera_fov_degrees,
        next_rotation_bucket.render_distance_chunks,
        next_rotation_bucket.smart_cull,
    );
    assert!(upload.refresh_frame_visibility(resized));
}

#[test]
fn terrain_reload_request_marks_all_cached_chunks_for_rebuild() {
    let mut upload = TerrainUploadState::default();
    assert!(!upload.reload_all_chunks_requested());
    upload.request_reload_all_chunks();
    assert!(upload.reload_all_chunks_requested());
}

#[test]
fn ready_unchanged_terrain_reuses_resident_selection() {
    let mut world = WorldStore::new();
    let selected = ChunkPos { x: 2, z: -3 };
    world.insert_decoded_chunk(test_chunk_with_sections(selected, 1));
    let mut upload = TerrainUploadState {
        observed_world_revision: world.terrain_revision(),
        selection_key: TerrainSelectionKey::of(&world),
        selected_chunks: vec![selected],
        cache_ready: true,
        ..Default::default()
    };

    assert!(terrain_cache_selection_is_stable(&world, &upload, false));
    assert!(!terrain_cache_selection_needs_refresh(
        &world, &upload, false
    ));
    assert!(!terrain_cache_selection_is_stable(&world, &upload, true));
    assert!(terrain_cache_selection_needs_refresh(&world, &upload, true));
    upload.reload_all_chunks_requested = true;
    assert!(!terrain_cache_selection_is_stable(&world, &upload, false));
    assert!(terrain_cache_selection_needs_refresh(
        &world, &upload, false
    ));
    upload.reload_all_chunks_requested = false;
    upload.cache_ready = false;
    assert!(!terrain_cache_selection_is_stable(&world, &upload, false));
    assert!(!terrain_cache_selection_needs_refresh(
        &world, &upload, false
    ));
    upload.selected_chunks.clear();
    upload.selection_key = None;
    assert!(terrain_cache_selection_needs_refresh(
        &world, &upload, false
    ));
}

#[test]
fn uploaded_cpu_meshes_can_retire_without_changing_full_sync_or_translucent_resort() {
    use bbb_client_render::{render_core::gpu::TerrainGpuVertex, terrain::TerrainTint};

    let Some(mut renderer) = bbb_client_render::Renderer::new_offscreen(64, 32) else {
        return;
    };
    renderer.set_clear_color(bbb_client_render::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .unwrap();
    let camera = |z, yaw| CameraPose {
        position: [0.0, 0.0, z],
        y_rot: yaw,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let quad = |x: f32, z: f32, tint| {
        let vertices: Vec<_> = [
            [x - 0.75, -0.75, z],
            [x + 0.75, -0.75, z],
            [x + 0.75, 0.75, z],
            [x - 0.75, 0.75, z],
        ]
        .into_iter()
        .cycle()
        .take(8)
        .map(|position| TerrainGpuVertex::pack(position, [0.5; 2], 0x00F0_00F0, tint, 1.0, 1.0))
        .collect();
        TerrainMesh {
            vertices,
            indices: vec![0, 1, 2, 2, 3, 0, 4, 6, 5, 6, 4, 7],
            source_sections: 1,
            ..TerrainMesh::default()
        }
    };
    let red = TerrainTint { r: 255, g: 0, b: 0 };
    let green = TerrainTint { r: 0, g: 255, b: 0 };
    let mut translucent = quad(0.0, -0.25, red);
    let far = quad(0.0, 0.25, green);
    translucent.vertices.extend(far.vertices);
    translucent.indices = far
        .indices
        .into_iter()
        .map(|index| index + 8)
        .chain(translucent.indices)
        .collect();
    translucent.translucent_faces = 4;
    let visibility = TerrainSectionVisibility::all_visible(0, true, false);
    let layers = TerrainSectionMeshLayers {
        opaque: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: quad(-2.0, 0.0, red),
        }],
        cutout: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: quad(2.0, 0.0, green),
        }],
        translucent: vec![TerrainSectionMesh {
            section_y: 0,
            mesh: translucent,
        }],
        visibility: vec![visibility],
        source_sections: 1,
    };
    let pos = ChunkPos { x: 0, z: 0 };
    let chunk = TerrainChunkMeshOwner {
        level_generation: 7,
        chunk_x: 0,
        chunk_z: 0,
    };
    let owner = TerrainSectionMeshOwner {
        chunk,
        section_y: 0,
    };
    let mut upload = TerrainUploadState {
        observed_level_generation: 7,
        selected_chunks: vec![pos],
        meshes: FxHashMap::from_iter([(
            pos,
            CachedTerrainMesh {
                revision: 1,
                section_mesh_revisions: vec![(0, 1)],
                resident_revisions: vec![(0, 1)],
                camera_position: [0.0, 0.0, -4.0],
                layers,
            },
        )]),
        ..Default::default()
    };
    install_resident_section(
        &mut upload,
        pos,
        visibility,
        [0.0, 0.0, -4.0],
        Instant::now(),
    );
    upload
        .meshes
        .get_mut(&pos)
        .unwrap()
        .layers
        .visibility
        .clear();
    renderer.set_camera_pose(Some(camera(-4.0, 0.0)));
    renderer.synchronize_terrain_section_mesh_layer_refs(
        1,
        [(
            chunk,
            &[(0, 1)][..],
            &upload.meshes[&pos].layers,
            [0.0, 0.0, -4.0],
        )],
    );
    renderer.set_terrain_section_states(
        1,
        &[TerrainSectionRenderState {
            owner,
            visibility: 1.0,
            nearby: true,
        }],
    );
    let front = renderer.render_offscreen_frame().unwrap();
    assert!(front.pixel(32, 16)[0] > front.pixel(32, 16)[1]);
    renderer.set_camera_pose(Some(camera(4.0, 180.0)));
    let back = renderer.render_offscreen_frame().unwrap();
    assert!(back.pixel(32, 16)[1] > back.pixel(32, 16)[0]);

    renderer.set_camera_pose(Some(camera(-4.0, 0.0)));
    upload_cached_terrain(&mut renderer, &mut upload, 1, true);
    let cached = &upload.meshes[&pos];
    assert!(cached.layers.opaque.is_empty());
    assert!(cached.layers.cutout.is_empty());
    assert!(cached.layers.translucent.is_empty());
    assert!(cached.layers.visibility.is_empty());
    assert_eq!(
        upload.section_residency[&SectionPosition {
            x: pos.x,
            y: 0,
            z: pos.z
        }]
            .compiled_visibility,
        visibility
    );
    assert_eq!(cached.resident_revisions, [(0, 1)]);
    let front_after = renderer.render_offscreen_frame().unwrap();
    assert!(front_after.pixel(32, 16)[0] > front_after.pixel(32, 16)[1]);
    renderer.set_camera_pose(Some(camera(4.0, 180.0)));
    let back_after = renderer.render_offscreen_frame().unwrap();
    assert!(back_after.pixel(32, 16)[1] > back_after.pixel(32, 16)[0]);
    upload_cached_terrain(&mut renderer, &mut upload, 1, true);
    let restored_back = renderer.render_offscreen_frame().unwrap().pixel(32, 16);
    assert!(restored_back[1] > restored_back[0]);

    let mut replacement = quad(0.0, 0.0, red);
    replacement.opaque_faces = 2;
    let cached = upload.meshes.get_mut(&pos).unwrap();
    cached.layers.opaque.push(TerrainSectionMesh {
        section_y: 0,
        mesh: replacement,
    });
    cached.resident_revisions[0].1 = 2;
    upload
        .mesh_section_updates
        .insert(SectionPosition { x: 0, y: 0, z: 0 });
    upload_cached_terrain(&mut renderer, &mut upload, 1, false);
    assert!(upload.meshes[&pos].layers.opaque.is_empty());
    let changed = renderer.render_offscreen_frame().unwrap().pixel(32, 16);
    assert!(changed[0] > changed[1]);
}

#[test]
fn empty_world_transition_clears_incremental_caches_once() {
    let pos = ChunkPos { x: 2, z: -3 };
    let mut upload = TerrainUploadState {
        observed_world_revision: 4,
        selected_chunks: vec![pos],
        meshes: FxHashMap::from_iter([(
            pos,
            CachedTerrainMesh {
                revision: 3,
                section_mesh_revisions: Vec::new(),
                resident_revisions: Vec::new(),
                camera_position: [0.0; 3],
                layers: TerrainSectionMeshLayers::default(),
            },
        )]),
        reload_all_chunks_requested: true,
        ..Default::default()
    };

    assert!(synchronize_empty_terrain_upload(&mut upload, 9));
    assert_eq!(upload.observed_world_revision, 9);
    assert!(upload.selected_chunks.is_empty());
    assert!(upload.meshes.is_empty());
    assert!(!upload.reload_all_chunks_requested);

    assert!(!synchronize_empty_terrain_upload(&mut upload, 9));
}

#[test]
fn resource_reload_without_loaded_chunks_rebuilds_the_block_table_for_the_next_chunk() {
    let pos = ChunkPos { x: 0, z: 0 };
    let mut world = WorldStore::new();
    world.insert_decoded_chunk(test_chunk_with_sections(pos, 1));
    let mut upload = TerrainUploadState {
        selected_chunks: vec![pos],
        ..Default::default()
    };
    let mesh_chunk = |upload: &mut TerrainUploadState, textures: &TerrainTextureState| {
        let work = terrain_mesh_work(&world, upload, pos);
        queue_terrain_chunk_mesh(
            &world,
            upload,
            textures,
            work,
            TerrainPrioritizeChunkUpdates::None,
            [0.0; 3],
            true,
        );
        Arc::clone(
            upload
                .block_table
                .as_ref()
                .expect("meshing builds a block table"),
        )
    };
    let before_reload = TerrainTextureState::default();
    let first = mesh_chunk(&mut upload, &before_reload);
    assert!(Arc::ptr_eq(
        &first,
        &mesh_chunk(&mut upload, &before_reload)
    ));

    // The reload lands while the level has no chunks, so it never reaches the
    // reload-all path that drops the table.
    upload.request_reload_all_chunks();
    synchronize_empty_terrain_upload(&mut upload, world.terrain_revision());
    upload.selected_chunks = vec![pos];
    let after_reload = TerrainTextureState::default();

    assert!(!Arc::ptr_eq(
        &first,
        &mesh_chunk(&mut upload, &after_reload)
    ));
}

#[test]
fn level_generation_change_drops_old_residency_without_reusing_revision_ids() {
    let pos = ChunkPos { x: 2, z: -3 };
    let mut upload = TerrainUploadState {
        observed_level_generation: 4,
        selected_chunks: vec![pos],
        meshes: FxHashMap::from_iter([(
            pos,
            CachedTerrainMesh {
                revision: 3,
                section_mesh_revisions: Vec::new(),
                resident_revisions: Vec::new(),
                camera_position: [0.0; 3],
                layers: TerrainSectionMeshLayers::default(),
            },
        )]),
        next_resident_revision: 8,
        ..Default::default()
    };
    upload.section_residency.insert(
        SectionPosition {
            x: pos.x,
            y: 4,
            z: pos.z,
        },
        TerrainSectionResidency {
            uploaded_at: Instant::now(),
            fade_duration: Duration::ZERO,
            compiled_visibility: TerrainSectionVisibility::all_visible(0, true, false),
        },
    );

    assert!(!synchronize_terrain_level_generation(&mut upload, 4));
    assert!(synchronize_terrain_level_generation(&mut upload, 5));
    assert!(upload.selected_chunks.is_empty());
    assert!(upload.meshes.is_empty());
    assert!(upload.section_residency.is_empty());
    assert_eq!(upload.next_resident_revision, 8);
    assert!(!synchronize_terrain_level_generation(&mut upload, 5));
}

#[test]
fn selected_resident_sections_publish_exact_vertical_mesh_membership() {
    let selected = ChunkPos { x: -1, z: 2 };
    let unselected = ChunkPos { x: 8, z: 9 };
    let mesh = |resident_revisions| CachedTerrainMesh {
        revision: 1,
        section_mesh_revisions: Vec::new(),
        resident_revisions,
        camera_position: [0.0; 3],
        layers: TerrainSectionMeshLayers::default(),
    };
    let upload = TerrainUploadState {
        selected_chunks: vec![selected],
        meshes: FxHashMap::from_iter([
            (selected, mesh(vec![(-4, 10), (3, 11)])),
            (unselected, mesh(vec![(7, 12)])),
        ]),
        ..Default::default()
    };

    assert_eq!(
        selected_resident_sections(
            &upload,
            TerrainVisibilityContext::new(None, [1280, 720], 70.0, 12, true),
        )
        .into_iter()
        .map(|visible| visible.position)
        .collect::<Vec<_>>(),
        vec![
            SectionPosition { x: -1, y: -4, z: 2 },
            SectionPosition { x: -1, y: 3, z: 2 },
        ]
    );
}

#[test]
fn partial_worker_result_preserves_unchanged_section_mesh_and_residency() {
    let pos = ChunkPos { x: 0, z: 0 };
    let mut world = WorldStore::new();
    world.insert_decoded_chunk(test_chunk(pos));
    let current_sections = terrain_section_mesh_revisions(&world, pos);
    assert!(current_sections.len() >= 2);
    let dirty_y = current_sections[0].0;
    let stable_y = current_sections[1].0;
    let mut cached_sections = current_sections.clone();
    cached_sections[0].1 = cached_sections[0].1.saturating_add(1);
    let cached_layers = TerrainSectionMeshLayers {
        opaque: vec![
            TerrainSectionMesh {
                section_y: dirty_y,
                mesh: TerrainMesh {
                    source_sections: 11,
                    ..TerrainMesh::default()
                },
            },
            TerrainSectionMesh {
                section_y: stable_y,
                mesh: TerrainMesh {
                    source_sections: 22,
                    ..TerrainMesh::default()
                },
            },
        ],
        source_sections: current_sections.len(),
        ..TerrainSectionMeshLayers::default()
    };
    let mut upload = TerrainUploadState {
        observed_level_generation: world.client_level_generation(),
        mesh_generation: 3,
        selected_chunks: vec![pos],
        meshes: FxHashMap::from_iter([(
            pos,
            CachedTerrainMesh {
                revision: world.terrain_chunk_mesh_revision(pos).saturating_sub(1),
                section_mesh_revisions: cached_sections,
                resident_revisions: current_sections
                    .iter()
                    .map(|&(section_y, _)| (section_y, 10))
                    .collect(),
                camera_position: [0.0; 3],
                layers: cached_layers,
            },
        )]),
        next_resident_revision: 10,
        ..Default::default()
    };
    let key = terrain_mesh_job_key(&world, &upload, pos);
    upload.pending_mesh_jobs.insert(pos, key);

    assert!(accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key,
            camera_position: [4.0, 5.0, 6.0],
            build_nanos: 23,
            sections: vec![(dirty_y, current_sections[0].1)],
            layers: TerrainSectionMeshLayers {
                opaque: vec![TerrainSectionMesh {
                    section_y: dirty_y,
                    mesh: TerrainMesh {
                        source_sections: 33,
                        ..TerrainMesh::default()
                    },
                }],
                visibility: vec![TerrainSectionVisibility::all_visible(dirty_y, false, false)],
                source_sections: 1,
                ..TerrainSectionMeshLayers::default()
            },
        },
    ));

    let cached = upload.meshes.get(&pos).expect("partial result installed");
    assert_eq!(cached.section_mesh_revisions, current_sections);
    assert_eq!(cached.layers.source_sections, current_sections.len());
    assert_eq!(cached.layers.opaque.len(), 2);
    assert_eq!(cached.layers.opaque[0].section_y, dirty_y);
    assert_eq!(cached.layers.opaque[0].mesh.source_sections, 33);
    assert_eq!(cached.layers.opaque[1].section_y, stable_y);
    assert_eq!(cached.layers.opaque[1].mesh.source_sections, 22);
    assert_eq!(
        cached
            .resident_revisions
            .iter()
            .find(|(section_y, _)| *section_y == dirty_y),
        Some(&(dirty_y, 11))
    );
    assert_eq!(
        cached
            .resident_revisions
            .iter()
            .find(|(section_y, _)| *section_y == stable_y),
        Some(&(stable_y, 10))
    );
    assert_eq!(upload.mesh_worker_counters.completed_sections, 1);
    assert_eq!(upload.mesh_worker_counters.recent_build_sections, 1);
    assert_eq!(upload.mesh_worker_counters.peak_build_sections, 1);
    assert_eq!(upload.occlusion_section_updates.len(), 1);
    assert_eq!(
        upload
            .occlusion_section_updates
            .get(&SectionPosition {
                x: pos.x,
                y: dirty_y,
                z: pos.z,
            })
            .and_then(|update| update.visibility),
        Some(TerrainSectionVisibility::all_visible(dirty_y, false, false))
    );
}

#[test]
fn block_update_packets_rebuild_dirty_sections_and_reuse_stable_layers() {
    let pos = ChunkPos { x: 0, z: 0 };
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 48,
    });
    world.insert_decoded_chunk(test_chunk_with_sections(pos, 3));
    let stone = propertyless_block_state_id("minecraft:stone");
    let dirt = propertyless_block_state_id("minecraft:dirt");
    let initial_sections = terrain_section_mesh_revisions(&world, pos);
    assert_eq!(
        initial_sections
            .iter()
            .map(|&(section_y, _)| section_y)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let initial_layers = TerrainSectionMeshLayers {
        opaque: initial_sections
            .iter()
            .enumerate()
            .map(|(index, &(section_y, _))| TerrainSectionMesh {
                section_y,
                mesh: TerrainMesh {
                    source_sections: (index + 1) * 10,
                    ..TerrainMesh::default()
                },
            })
            .collect(),
        source_sections: initial_sections.len(),
        ..TerrainSectionMeshLayers::default()
    };
    let mut initial_mesh = cached_mesh(&world, pos, 100);
    initial_mesh.layers = initial_layers;
    let mut upload = TerrainUploadState {
        observed_level_generation: world.client_level_generation(),
        selected_chunks: vec![pos],
        meshes: FxHashMap::from_iter([(pos, initial_mesh)]),
        next_resident_revision: 100,
        ..Default::default()
    };

    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 5, y: 5, z: 7 },
        block_state_id: stone,
    }));
    let single_dirty = dirty_terrain_sections(&world, &upload, pos);
    assert_eq!(
        single_dirty
            .iter()
            .map(|&(section_y, _)| section_y)
            .collect::<Vec<_>>(),
        vec![0]
    );
    let single_key = terrain_mesh_job_key(&world, &upload, pos);
    upload.pending_mesh_jobs.insert(pos, single_key);
    assert!(accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key: single_key,
            camera_position: [0.0; 3],
            build_nanos: 1,
            sections: single_dirty,
            layers: TerrainSectionMeshLayers {
                opaque: vec![TerrainSectionMesh {
                    section_y: 0,
                    mesh: TerrainMesh {
                        source_sections: 11,
                        ..TerrainMesh::default()
                    },
                }],
                source_sections: 1,
                ..TerrainSectionMeshLayers::default()
            },
        },
    ));

    let cached = upload.meshes.get(&pos).expect("single update installed");
    assert_eq!(
        cached
            .layers
            .opaque
            .iter()
            .map(|section| section.mesh.source_sections)
            .collect::<Vec<_>>(),
        vec![11, 20, 30]
    );
    assert_eq!(
        cached.resident_revisions,
        vec![(0, 101), (1, 100), (2, 100)]
    );

    assert_eq!(
        world.apply_section_blocks_update(SectionBlocksUpdate {
            section_x: 0,
            section_y: 2,
            section_z: 0,
            updates: vec![BlockUpdate {
                pos: ProtocolBlockPos { x: 6, y: 37, z: 8 },
                block_state_id: dirt,
            }],
        }),
        1
    );
    let section_dirty = dirty_terrain_sections(&world, &upload, pos);
    assert_eq!(
        section_dirty
            .iter()
            .map(|&(section_y, _)| section_y)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let section_key = terrain_mesh_job_key(&world, &upload, pos);
    upload.pending_mesh_jobs.insert(pos, section_key);
    assert!(accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key: section_key,
            camera_position: [0.0; 3],
            build_nanos: 1,
            sections: section_dirty,
            layers: TerrainSectionMeshLayers {
                opaque: vec![TerrainSectionMesh {
                    section_y: 2,
                    mesh: TerrainMesh {
                        source_sections: 31,
                        ..TerrainMesh::default()
                    },
                }],
                source_sections: 1,
                ..TerrainSectionMeshLayers::default()
            },
        },
    ));

    let cached = upload.meshes.get(&pos).expect("section update installed");
    assert_eq!(
        cached
            .layers
            .opaque
            .iter()
            .map(|section| section.mesh.source_sections)
            .collect::<Vec<_>>(),
        vec![11, 20, 31]
    );
    assert_eq!(
        cached.resident_revisions,
        vec![(0, 101), (1, 100), (2, 102)]
    );
}

#[test]
fn prioritized_block_changes_install_without_waiting_for_an_older_mesh() {
    for policy in [
        TerrainPrioritizeChunkUpdates::Nearby,
        TerrainPrioritizeChunkUpdates::PlayerAffected,
    ] {
        let pos = ChunkPos { x: 0, z: 0 };
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 48,
        });
        world.insert_decoded_chunk(test_chunk_with_sections(pos, 3));
        let stone = propertyless_block_state_id("minecraft:stone");
        for y in [5, 21, 37] {
            assert!(world.apply_block_update(BlockUpdate {
                pos: ProtocolBlockPos { x: 5, y, z: 7 },
                block_state_id: stone,
            }));
        }
        let textures = TerrainTextureState::default();
        let mut upload = TerrainUploadState {
            observed_level_generation: world.client_level_generation(),
            selected_chunks: vec![pos],
            ..Default::default()
        };
        let work = terrain_mesh_work(&world, &upload, pos);
        queue_terrain_chunk_mesh(
            &world,
            &mut upload,
            &textures,
            work,
            TerrainPrioritizeChunkUpdates::Nearby,
            [8.0, 24.0, 8.0],
            true,
        );
        let opaque_index_counts = |cached: &CachedTerrainMesh| {
            [0, 1, 2].map(|y| {
                cached
                    .layers
                    .opaque
                    .iter()
                    .find(|section| section.section_y == y)
                    .map_or(0, |section| section.mesh.index_count())
            })
        };
        assert_eq!(opaque_index_counts(&upload.meshes[&pos]), [36, 36, 36]);
        let stable = upload.meshes[&pos].layers.opaque[1].clone();
        let stable_revision = upload.meshes[&pos].resident_revisions[1];

        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 5, y: 5, z: 7 },
            block_state_id: propertyless_block_state_id("minecraft:dirt"),
        }));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 5, y: 37, z: 7 },
            block_state_id: bbb_client_world::vanilla_air_block_state_id(),
        }));
        let key = terrain_mesh_job_key(&world, &upload, pos);
        let old_result = build_terrain_mesh_job(TerrainMeshJob {
            key,
            target: snapshot_chunk_from_world(&world, pos, terrain_cardinal_lighting(&world))
                .unwrap(),
            neighborhood: Vec::new(),
            table: Arc::clone(upload.block_table.as_ref().unwrap()),
            camera_position: [8.0; 3],
            sections: dirty_terrain_sections(&world, &upload, pos),
            shared_quad_indices: true,
        });
        if policy == TerrainPrioritizeChunkUpdates::PlayerAffected {
            let block = bbb_client_world::BlockPos { x: 5, y: 5, z: 7 };
            assert!(world.predict_local_destroy_block(block, 1));
        } else {
            assert!(world.apply_block_update(BlockUpdate {
                pos: ProtocolBlockPos { x: 5, y: 5, z: 7 },
                block_state_id: bbb_client_world::vanilla_air_block_state_id(),
            }));
        }
        let (pool, pending) = TerrainMeshPool::with_pending_result();
        upload.mesh_worker = Some(pool);
        upload.pending_mesh_jobs.insert(pos, key);
        assert!(
            matches!(next_terrain_cache_work(&world, &upload, &mut 0), TerrainCacheWork::QueueMesh(work) if work.key.pos == pos)
        );
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let delayed = std::thread::spawn(move || {
            // Bound a regression's wait while keeping the successful path event-driven.
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
            pending(old_result);
        });
        let work = terrain_mesh_work(&world, &upload, pos);
        let outcome =
            queue_terrain_chunk_mesh(&world, &mut upload, &textures, work, policy, [8.0; 3], true);
        let installed_before_release = release_tx.send(()).is_ok();
        delayed.join().unwrap();
        assert!(
            installed_before_release,
            "priority update waited for the older mesh"
        );
        assert!(matches!(
            outcome,
            TerrainMeshQueueOutcome::SynchronouslyCompiled {
                renderer_dirty: true
            }
        ));
        let cached = &upload.meshes[&pos];
        assert_eq!(opaque_index_counts(cached), [0, 36, 36]);
        assert_eq!(cached.resident_revisions[1], stable_revision);
        assert!(matches!(
            dirty_terrain_sections(&world, &upload, pos).as_slice(),
            [(2, _)]
        ));

        assert!(!drain_terrain_mesh_results(
            &world,
            &mut upload,
            Instant::now()
        ));
        assert_eq!(upload.mesh_worker_counters.stale_jobs, 1);
        assert_eq!(upload.meshes[&pos].layers.opaque[1], stable);
        assert_eq!(upload.meshes[&pos].resident_revisions[1], stable_revision);
        let work = terrain_mesh_work(&world, &upload, pos);
        assert!(matches!(
            queue_terrain_chunk_mesh(&world, &mut upload, &textures, work, policy, [8.0; 3], true),
            TerrainMeshQueueOutcome::Async
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        while upload.meshes[&pos].revision != world.terrain_chunk_mesh_revision(pos) {
            drain_terrain_mesh_results(&world, &mut upload, Instant::now());
            assert!(
                Instant::now() < deadline,
                "remaining section never installed"
            );
            std::thread::yield_now();
        }
        assert_eq!(opaque_index_counts(&upload.meshes[&pos]), [0, 36, 0]);
        assert_eq!(upload.meshes[&pos].resident_revisions[1], stable_revision);
        upload.meshes.clear();
        assert!(dirty_terrain_section_states(&world, &upload, pos)
            .iter()
            .all(|section| !section.dirty_from_player));
    }
}

#[test]
fn stale_worker_result_cannot_replace_current_chunk_revision() {
    let pos = ChunkPos { x: 0, z: 0 };
    let mut world = WorldStore::new();
    world.insert_decoded_chunk(test_chunk(pos));
    let mut upload = TerrainUploadState {
        observed_level_generation: world.client_level_generation(),
        mesh_generation: 3,
        selected_chunks: vec![pos],
        ..Default::default()
    };
    let current = terrain_mesh_job_key(&world, &upload, pos);
    let current_sections = dirty_terrain_sections(&world, &upload, pos);
    let stale = TerrainMeshJobKey {
        mesh_generation: current.mesh_generation - 1,
        ..current
    };
    upload.pending_mesh_jobs.insert(pos, stale);

    assert!(!accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key: stale,
            camera_position: [1.0, 2.0, 3.0],
            build_nanos: 17,
            sections: current_sections.clone(),
            layers: TerrainSectionMeshLayers::default(),
        },
    ));
    assert!(upload.meshes.is_empty());
    assert_eq!(upload.mesh_worker_counters.completed_jobs, 1);
    assert_eq!(upload.mesh_worker_counters.stale_jobs, 1);

    upload.pending_mesh_jobs.insert(pos, current);
    assert!(accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key: current,
            camera_position: [4.0, 5.0, 6.0],
            build_nanos: 23,
            sections: current_sections,
            layers: TerrainSectionMeshLayers::default(),
        },
    ));
    let cached = upload.meshes.get(&pos).expect("current result installed");
    assert_eq!(cached.revision, current.mesh_revision);
    assert_eq!(cached.camera_position, [4.0, 5.0, 6.0]);
    assert_eq!(upload.mesh_worker_counters.completed_jobs, 2);
    assert_eq!(upload.mesh_worker_counters.stale_jobs, 1);
    assert_eq!(upload.mesh_worker_counters.recent_build_nanos, 23);
    assert_eq!(upload.mesh_worker_counters.peak_build_nanos, 23);
}

#[test]
fn cache_scheduler_refreshes_one_neighbor_at_a_time_before_one_mesh() {
    use bbb_client_assets::{
        BiomeColorCatalog, BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier,
    };

    let center = ChunkPos { x: 0, z: 0 };
    let east = ChunkPos { x: 1, z: 0 };
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(test_chunk_with_sections(center, 1));
    let mut east_chunk = test_chunk_with_sections(east, 1);
    world.insert_decoded_chunk(east_chunk.clone());
    let grass = bbb_client_world::vanilla_block_state_id(
        "minecraft:grass_block",
        &std::collections::BTreeMap::from([("snowy".into(), "false".into())]),
    )
    .unwrap();
    assert!(world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 15, y: 5, z: 8 },
        block_state_id: grass,
    }));
    let profiles = [[255, 0, 0], [0, 255, 0]]
        .into_iter()
        .enumerate()
        .map(|(id, color)| BiomeColorProfile {
            id: id as i32,
            name: format!("test:biome_{id}"),
            temperature: 0.5,
            downfall: 0.5,
            has_precipitation: true,
            temperature_modifier: BiomeTemperatureModifier::None,
            grass_color: Some(color),
            foliage_color: None,
            dry_foliage_color: None,
            water_color: None,
            grass_color_modifier: GrassColorModifier::None,
        });
    let textures =
        TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new(profiles));
    let compile_next = |world: &WorldStore, upload: &mut TerrainUploadState, expected| {
        let TerrainCacheWork::QueueMesh(work) = next_terrain_cache_work(world, upload, &mut 0)
        else {
            panic!("expected a ready column mesh")
        };
        assert_eq!(work.key.pos, expected);
        assert!(matches!(
            queue_terrain_chunk_mesh(
                world,
                upload,
                &textures,
                work,
                TerrainPrioritizeChunkUpdates::Nearby,
                [15.0, 5.0, 8.0],
                true
            ),
            TerrainMeshQueueOutcome::SynchronouslyCompiled {
                renderer_dirty: true
            }
        ));
    };
    let top_color = |upload: &TerrainUploadState| {
        upload.meshes[&center].layers.opaque[0]
            .mesh
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .find(|face| face.iter().all(|v| v.local_position()[1] == 6.0))
            .expect("grass top face")[0]
            .color
    };
    let mut selected_chunks = world.chunk_positions();
    sort_terrain_chunks(&mut selected_chunks, center);
    let mut upload = TerrainUploadState {
        selected_chunks,
        ..Default::default()
    };

    compile_next(&world, &mut upload, center);
    compile_next(&world, &mut upload, east);
    assert_eq!(
        next_terrain_cache_work(&world, &upload, &mut 0),
        TerrainCacheWork::Idle
    );

    let before = top_color(&upload);
    Arc::make_mut(&mut east_chunk.sections[0]).biomes =
        single_value_container(PaletteDomain::Biomes, 64, 1);
    world.insert_decoded_chunk(east_chunk);
    compile_next(&world, &mut upload, center);
    let after = top_color(&upload);
    assert!(
        after[0] < before[0] && after[1] > before[1],
        "grass top {before:?} -> {after:?}"
    );
}

#[test]
fn terrain_selection_keeps_every_server_loaded_chunk_in_distance_order() {
    let center = ChunkPos { x: 0, z: 0 };
    let mut world = WorldStore::new();
    for x in -4..=4 {
        for z in -4..=4 {
            world.insert_decoded_chunk(test_chunk(ChunkPos { x, z }));
        }
    }

    world.apply_set_chunk_cache_center(SetChunkCacheCenter {
        chunk_x: 0,
        chunk_z: 0,
    });
    let mut upload = TerrainUploadState::default();
    refresh_terrain_selection(&world, &mut upload);
    let selected = &upload.selected_chunks;

    assert_eq!(selected.len(), 81);
    assert_eq!(selected[0], center);
    assert_eq!(selected.iter().copied().collect::<FxHashSet<_>>().len(), 81);
    assert!(selected.windows(2).all(|pair| {
        chunk_distance_key(pair[0], center) <= chunk_distance_key(pair[1], center)
    }));

    upload.meshes.insert(center, cached_mesh(&world, center, 1));
    world.apply_set_chunk_cache_center(SetChunkCacheCenter {
        chunk_x: 1,
        chunk_z: 0,
    });
    assert!(!refresh_terrain_selection(&world, &mut upload));
    assert_eq!(upload.selected_chunks[0], ChunkPos { x: 1, z: 0 });
    assert!(world.forget_chunk(center));
    assert!(refresh_terrain_selection(&world, &mut upload));
    assert!(!upload.meshes.contains_key(&center));
    assert_eq!(upload.selected_chunks.len(), 80);
}

#[test]
fn terrain_residency_counters_require_every_selected_section_and_no_pending_job() {
    let center = ChunkPos { x: 0, z: 0 };
    let east = ChunkPos { x: 1, z: 0 };
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 48,
    });
    let mut center_chunk = test_chunk_with_sections(center, 3);
    let section = Arc::make_mut(&mut center_chunk.sections[0]);
    section.block_states = single_value_container(
        PaletteDomain::BlockStates,
        4096,
        propertyless_block_state_id("minecraft:stone"),
    );
    section.non_empty_block_count = 4096;
    world.insert_decoded_chunk(center_chunk);
    world.insert_decoded_chunk(test_chunk_with_sections(east, 3));
    let mut upload = TerrainUploadState {
        selected_chunks: vec![center, east],
        meshes: FxHashMap::from_iter([(center, cached_mesh(&world, center, 1))]),
        ..Default::default()
    };

    let incomplete = terrain_residency_counters(&world, &mut upload);
    assert_eq!(incomplete.selected_chunks, 2);
    assert_eq!(incomplete.resident_chunks, 1);
    assert_eq!(incomplete.selected_sections, 6);
    assert_eq!(incomplete.resident_sections, 3);
    assert!(!incomplete.cache_ready);

    let key = terrain_mesh_job_key(&world, &upload, east);
    assert!(accept_terrain_mesh_result(
        &world,
        &mut upload,
        TerrainMeshResult {
            key,
            camera_position: [0.0; 3],
            build_nanos: 0,
            sections: terrain_section_mesh_revisions(&world, east),
            layers: TerrainSectionMeshLayers::default(),
        },
    ));
    upload.pending_mesh_jobs.insert(east, key);
    let pending = terrain_residency_counters(&world, &mut upload);
    assert_eq!(pending.resident_chunks, 2);
    assert_eq!(pending.resident_sections, 6);
    assert_eq!(pending.pending_jobs, 1);
    assert!(!pending.cache_ready);

    upload.pending_mesh_jobs.clear();
    assert!(terrain_residency_counters(&world, &mut upload).cache_ready);

    upload.selected_cursor = upload.selected_chunks.len();
    world.forget_chunk(east);
    refresh_terrain_selection(&world, &mut upload);
    let invalidated = terrain_residency_counters(&world, &mut upload);
    assert_eq!(invalidated.selected_chunks, 1);
    assert_eq!(invalidated.resident_sections, 0);
    assert!(!invalidated.cache_ready);
    let mut cursor = upload.selected_cursor;
    assert!(matches!(
        next_terrain_cache_work(&world, &upload, &mut cursor),
        TerrainCacheWork::QueueMesh(work) if work.key.pos == center
    ));
}

#[test]
fn entity_section_visits_survive_turns_without_changing_terrain_or_player_readiness() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: 8.0,
            y: 8.0,
            z: -24.0,
        },
        ..Default::default()
    });
    let mut upload = TerrainUploadState::default();
    let camera = [8.0; 3];
    let position = |z| SectionPosition { x: 0, y: 0, z };
    let far = position(-2);
    let actor = |section: SectionPosition| bbb_client_world::EntityVec3 {
        x: 8.0,
        y: 8.0,
        z: f64::from(section.z * 16 + 8),
    };
    let context = |yaw| {
        TerrainVisibilityContext::new(
            Some(CameraPose {
                position: camera,
                y_rot: yaw,
                x_rot: 0.0,
                eye_height: 0.0,
            }),
            [1280, 720],
            70.0,
            5,
            true,
        )
    };
    let old = Instant::now() - Duration::from_secs(5);
    let sections = (-3..=0)
        .map(|z| {
            let empty = z % 2 != 0;
            let visibility = TerrainSectionVisibility::all_visible(0, !empty, empty);
            install_resident_section(&mut upload, position(z).chunk(), visibility, camera, old);
            ResidentSection {
                position: position(z),
                visibility,
            }
        })
        .collect::<Vec<_>>();
    upload.selected_chunks.push(far.chunk());
    upload.meshes.insert(
        far.chunk(),
        CachedTerrainMesh {
            revision: 1,
            section_mesh_revisions: vec![(0, 1)],
            resident_revisions: vec![(0, 1)],
            camera_position: camera,
            layers: TerrainSectionMeshLayers::default(),
        },
    );
    upload.refresh_frame_visibility(context(0.0));
    assert!(upload.renderer_sections.consumer_visible(far));
    assert!(upload.is_player_section_compiled_and_visible(&world));
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor(far)));
    let loaded = (-4..=1)
        .flat_map(|z| (-1..=1).map(move |x| ChunkPos { x, z }))
        .collect();
    upload
        .occlusion_graph
        .rebuild(sections, &loaded, camera.map(f64::from), 5, 0, 1, true);
    upload.refresh_frame_visibility(context(0.0));
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor(far)));
    assert!(upload.is_entity_section_compiled_and_visible(&world, actor(position(-3))));

    let mut moved = context(180.0);
    moved.camera_pose.as_mut().unwrap().position[2] = -24.0;
    upload.refresh_frame_visibility(moved);
    assert!(upload.renderer_sections.consumer_visible(far));
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor(far)));

    let before = bbb_client_clock::now();
    upload.refresh_frame_visibility(context(180.0));
    assert!(upload.renderer_sections.consumer_visible(far));
    let EntitySectionVisit::VisibleAt(at) = upload.entity_visits[&far] else {
        panic!("visible section did not admit entities");
    };
    assert!(at >= before + Duration::from_millis(225));
    upload
        .entity_visits
        .insert(far, EntitySectionVisit::VisibleAt(old));
    upload.refresh_frame_visibility(context(0.0));
    assert!(!upload.renderer_sections.consumer_visible(far));
    assert!(upload.is_entity_section_compiled_and_visible(&world, actor(far)));
    install_resident_section(
        &mut upload,
        position(-3).chunk(),
        TerrainSectionVisibility::none_visible(0, true, false),
        camera,
        old,
    );
    assert!(upload.is_entity_section_compiled_and_visible(&world, actor(position(-3))));
    remove_resident_section(&mut upload, position(-3));
    assert!(!upload.occlusion_graph.visited_empty(position(-3)));
    synchronize_terrain_level_generation(&mut upload, world.client_level_generation() + 1);
    assert!(!upload.is_entity_section_compiled_and_visible(&world, actor(far)));
}

#[test]
fn section_ready_distinguishes_vertical_meshes_and_height_policy() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: -64,
        height: 384,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: -0.25,
            y: 64.0,
            z: 16.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut upload = TerrainUploadState::default();

    assert!(!upload.is_player_section_compiled_and_visible(&world));
    let position = SectionPosition { x: -1, y: 4, z: 1 };
    upload.section_residency.insert(
        position,
        TerrainSectionResidency {
            uploaded_at: Instant::now(),
            fade_duration: Duration::ZERO,
            compiled_visibility: TerrainSectionVisibility::all_visible(0, true, false),
        },
    );
    upload
        .renderer_sections
        .replace(0, [position], &upload.section_residency, Instant::now());
    upload
        .entity_visits
        .insert(position, EntitySectionVisit::VisibleAt(Instant::now()));
    assert!(upload.is_player_section_compiled_and_visible(&world));
    assert!(upload.is_entity_section_compiled_and_visible(
        &world,
        bbb_client_world::EntityVec3 {
            x: -0.25,
            y: 64.0,
            z: 16.0,
        }
    ));
    assert!(upload.is_block_section_compiled_and_visible(
        &world,
        bbb_client_world::BlockPos {
            x: -1,
            y: 64,
            z: 16,
        }
    ));

    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: -0.25,
            y: 80.0,
            z: 16.0,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(
        !upload.is_player_section_compiled_and_visible(&world),
        "another vertical section in the same chunk is not compiled"
    );
    assert!(!upload.is_entity_section_compiled_and_visible(
        &world,
        bbb_client_world::EntityVec3 {
            x: -0.25,
            y: 80.0,
            z: 16.0,
        }
    ));
    assert!(!upload.is_block_section_compiled_and_visible(
        &world,
        bbb_client_world::BlockPos {
            x: -1,
            y: 80,
            z: 16,
        }
    ));

    world.set_local_player_pose(LocalPlayerPoseState {
        position: Vec3d {
            x: -0.25,
            y: 400.0,
            z: 16.0,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(!upload.is_player_section_compiled_and_visible(&world));
    assert!(upload.is_entity_section_compiled_and_visible(
        &world,
        bbb_client_world::EntityVec3 {
            x: -0.25,
            y: 400.0,
            z: 16.0,
        }
    ));
    assert!(upload.is_block_section_compiled_and_visible(
        &world,
        bbb_client_world::BlockPos {
            x: -1,
            y: 400,
            z: 16,
        }
    ));
}

#[test]
fn terrain_upload_center_is_derived_from_world_chunk_state() {
    let mut world = WorldStore::new();
    assert_eq!(terrain_upload_center(&world), ChunkPos { x: 0, z: 0 });

    world.insert_decoded_chunk(test_chunk(ChunkPos { x: 5, z: 6 }));
    assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

    world.insert_decoded_chunk(test_chunk(ChunkPos { x: -1, z: 2 }));
    assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

    world.apply_set_chunk_cache_center(SetChunkCacheCenter {
        chunk_x: -4,
        chunk_z: 7,
    });
    assert_eq!(terrain_upload_center(&world), ChunkPos { x: -4, z: 7 });
}

fn test_chunk(pos: ChunkPos) -> ChunkColumn {
    ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: Arc::default(),
        light: LightData::default(),
    }
}

fn test_chunk_with_sections(pos: ChunkPos, section_count: usize) -> ChunkColumn {
    ChunkColumn {
        sections: (0..section_count)
            .map(|_| {
                Arc::new(ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_client_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        ..test_chunk(pos)
    }
}

fn single_value_container(
    domain: PaletteDomain,
    entry_count: usize,
    global_id: i32,
) -> PalettedContainerData {
    PalettedContainerData {
        domain,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![global_id],
        packed_data: Vec::new(),
        entry_count,
    }
}

fn propertyless_block_state_id(name: &str) -> i32 {
    bbb_client_world::vanilla_block_state_id(name, &std::collections::BTreeMap::new())
        .unwrap_or_else(|| panic!("missing propertyless vanilla block state {name}"))
}

#[test]
fn ready_terrain_rebuilds_retired_inputs_after_block_light_chunk_and_resource_changes() {
    use bbb_protocol::packets::{LightUpdate, LightUpdateData};

    let Some(mut renderer) = bbb_client_render::Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer
        .upload_terrain_texture_atlas(1, 1, &[255; 4])
        .unwrap();
    renderer.set_clear_color(bbb_client_render::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    let camera = CameraPose {
        position: [8.5, 8.5, 3.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    renderer.set_camera_pose(Some(camera));
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 48,
    });
    let pos = ChunkPos { x: 0, z: 0 };
    let mut dark_chunk = test_chunk_with_sections(pos, 3);
    for section_index in 0..5 {
        dark_chunk.light.set_sky_empty(section_index);
        dark_chunk.light.set_block_empty(section_index);
    }
    world.insert_decoded_chunk(dark_chunk.clone());
    let block = ProtocolBlockPos { x: 8, y: 8, z: 8 };
    let light_sample = bbb_client_world::BlockPos { x: 8, y: 8, z: 7 };
    let stone = propertyless_block_state_id("minecraft:stone");
    assert!(world.apply_block_update(BlockUpdate {
        pos: block,
        block_state_id: stone
    }));
    let mut upload = TerrainUploadState::default();
    let textures = TerrainTextureState::default();
    let frame = |world: &WorldStore, upload: &mut TerrainUploadState, renderer: &mut Renderer| {
        loop {
            maybe_upload_decoded_terrain(
                world,
                renderer,
                upload,
                &textures,
                TerrainVisibilityContext::new(Some(camera), [32, 32], 70.0, 8, false),
                TerrainPrioritizeChunkUpdates::Nearby,
                camera.position,
            );
            if upload.cache_ready {
                break;
            }
            std::thread::yield_now();
        }
        let cached = &upload.meshes[&pos];
        assert!(cached.layers.opaque.is_empty());
        assert!(cached.layers.cutout.is_empty());
        assert!(cached.layers.translucent.is_empty());
        renderer.set_terrain_section_states(
            upload.renderer_sections_revision(),
            upload.renderer_sections(),
        );
        renderer.render_offscreen_frame().unwrap().rgba
    };
    assert_eq!(world.sample_block_light(light_sample).unwrap().block, 0);
    world.run_light_updates().unwrap();
    let initial = frame(&world, &mut upload, &mut renderer);
    assert!(world.apply_block_update(BlockUpdate {
        pos: block,
        block_state_id: bbb_client_world::vanilla_air_block_state_id(),
    }));
    world.run_light_updates().unwrap();
    let empty = frame(&world, &mut upload, &mut renderer);
    assert_ne!(initial, empty);
    assert!(world.apply_block_update(BlockUpdate {
        pos: block,
        block_state_id: stone
    }));
    world.run_light_updates().unwrap();
    assert_eq!(frame(&world, &mut upload, &mut renderer), initial);

    let torch_pos = ProtocolBlockPos { x: 10, y: 8, z: 7 };
    assert!(world.apply_block_update(BlockUpdate {
        pos: torch_pos,
        block_state_id: propertyless_block_state_id("minecraft:torch"),
    }));
    world.run_light_updates().unwrap();
    assert_eq!(world.sample_block_light(light_sample).unwrap().block, 12);
    let torch_lit = frame(&world, &mut upload, &mut renderer);
    let center_red = (16 * 32 + 16) * 4;
    assert!(
        torch_lit[center_red] > initial[center_red],
        "the stone face must receive propagated torch light"
    );
    assert!(world.apply_block_update(BlockUpdate {
        pos: torch_pos,
        block_state_id: bbb_client_world::vanilla_air_block_state_id(),
    }));
    world.run_light_updates().unwrap();
    assert_eq!(world.sample_block_light(light_sample).unwrap().block, 0);
    assert_eq!(frame(&world, &mut upload, &mut renderer), initial);

    world
        .queue_light_update(LightUpdate {
            chunk_x: 0,
            chunk_z: 0,
            light_data: LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: vec![0b10],
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: vec![vec![0xEE; 2048]],
            },
        })
        .unwrap();
    let pending_publication = world.publication_snapshot();
    assert_eq!(world.poll_queued_light_updates(), 1);
    assert_eq!(world.sample_block_light(light_sample).unwrap().block, 14);
    assert_eq!(
        pending_publication
            .sample_block_light(light_sample)
            .unwrap()
            .block,
        0,
    );
    let lit = frame(&world, &mut upload, &mut renderer);
    assert_ne!(
        lit, initial,
        "propagated block light must brighten retired terrain"
    );
    upload.request_reload_all_chunks();
    assert_eq!(frame(&world, &mut upload, &mut renderer), lit);

    world.insert_decoded_chunk(dark_chunk);
    assert_eq!(frame(&world, &mut upload, &mut renderer), empty);
    assert!(world.apply_block_update(BlockUpdate {
        pos: block,
        block_state_id: stone
    }));
    world.run_light_updates().unwrap();
    assert_eq!(frame(&world, &mut upload, &mut renderer), initial);
}

fn cached_mesh(world: &WorldStore, pos: ChunkPos, resident_revision: u64) -> CachedTerrainMesh {
    let section_mesh_revisions = terrain_section_mesh_revisions(world, pos);
    CachedTerrainMesh {
        revision: world.terrain_chunk_mesh_revision(pos),
        resident_revisions: section_mesh_revisions
            .iter()
            .map(|&(section_y, _)| (section_y, resident_revision))
            .collect(),
        section_mesh_revisions,
        camera_position: [0.0; 3],
        layers: TerrainSectionMeshLayers::default(),
    }
}

fn network_biome_entry(id: &str) -> bbb_client_world::RegistryPacketEntry {
    bbb_client_world::RegistryPacketEntry::with_raw_data(id, synchronized_biome_nbt(true))
}

fn blend_biome_profile(id: i32, water: [u8; 3], modifier: GrassColorModifier) -> BiomeColorProfile {
    BiomeColorProfile {
        id,
        name: format!("minecraft:blend_biome_{id}"),
        temperature: 0.5,
        temperature_modifier: BiomeTemperatureModifier::None,
        downfall: 0.5,
        has_precipitation: true,
        grass_color: Some([100, 100, 100]),
        foliage_color: Some([40, 50, 60]),
        dry_foliage_color: Some([70, 80, 90]),
        water_color: Some(water),
        grass_color_modifier: modifier,
    }
}

#[test]
fn biome_colors_follow_active_registry_resource_order_after_finish() {
    let mut plains = blend_biome_profile(10, [1, 2, 3], GrassColorModifier::None);
    plains.name = "minecraft:plains".to_string();
    plains.temperature = 0.2;
    let mut desert = blend_biome_profile(20, [4, 5, 6], GrassColorModifier::None);
    desert.name = "minecraft:desert".to_string();
    desert.temperature = 0.8;
    let mut textures = TerrainTextureState::with_biome_colors_for_tests(
        bbb_client_assets::BiomeColorCatalog::new([plains, desert]),
    );
    let mut reloaded = textures.clone();
    let mut world = WorldStore::new();

    assert_eq!(textures.biome_temperature(Some(10)), Some(0.2));
    world.begin_registry_configuration();
    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![
            network_biome_entry("example:custom"),
            network_biome_entry("minecraft:desert"),
            network_biome_entry("minecraft:plains"),
        ],
    );
    for data in bbb_client_render::scene::synchronized_registry_test_fixture::minimal_non_empty_synchronized_registry_data() {
        world.record_registry_data(data);
    }
    assert!(!synchronize_terrain_biome_registry(&mut textures, &world));

    world.finish_registry_configuration().unwrap();
    assert!(synchronize_terrain_biome_registry(&mut textures, &world));
    assert!(!reloaded
        .clone()
        .with_cutout_leaves(false)
        .preserve_mesh_inputs_from(&mut textures));
    assert!(reloaded.preserve_mesh_inputs_from(&mut textures));
    let mut textures = reloaded;
    assert_eq!(textures.biome_temperature(Some(0)), Some(0.35));
    assert_eq!(textures.biome_temperature(Some(1)), Some(0.35));
    assert_eq!(textures.biome_temperature(Some(2)), Some(0.35));
    let water_state_id =
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:water",
            &std::collections::BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .unwrap();
    let expected_water_particle = [
        0x11 as f32 / 255.0 * 0.6,
        0x22 as f32 / 255.0 * 0.6,
        0x33 as f32 / 255.0 * 0.6,
        1.0,
    ];
    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state_at(water_state_id, Some(1), None),
        Some(expected_water_particle)
    );
    let particle_tints = textures.particle_tint_catalog();
    assert_eq!(
        particle_tints.terrain_particle_tint_color_for_block_state(
            water_state_id,
            Some(1),
            None,
            None,
        ),
        Some(expected_water_particle)
    );
    assert!(!synchronize_terrain_biome_registry(&mut textures, &world));

    assert!(synchronize_terrain_biome_registry(
        &mut textures,
        &WorldStore::new()
    ));
    assert_eq!(textures.biome_temperature(Some(10)), Some(0.2));
}

#[test]
fn synchronized_biome_data_overrides_pack_fallback_and_rejects_malformed_configuration() {
    let mut plains = blend_biome_profile(10, [1, 2, 3], GrassColorModifier::None);
    plains.name = "minecraft:plains".to_string();
    let mut desert = blend_biome_profile(20, [4, 5, 6], GrassColorModifier::None);
    desert.name = "minecraft:desert".to_string();
    let mut textures = TerrainTextureState::with_biome_colors_for_tests(
        bbb_client_assets::BiomeColorCatalog::new([plains, desert]),
    );
    let mut world = WorldStore::new();

    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::with_raw_data(
                "minecraft:plains",
                synchronized_biome_nbt(true),
            ),
            bbb_client_world::RegistryPacketEntry::stub("minecraft:desert"),
        ],
    );

    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    assert!(synchronize_terrain_biome_registry(&mut textures, &world));
    assert_eq!(textures.biome_temperature(Some(0)), Some(0.35));
    assert_eq!(textures.biome_has_precipitation(Some(0)), Some(true));
    assert_eq!(textures.biome_temperature(Some(1)), Some(0.5));
    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:malformed",
            synchronized_biome_nbt(false),
        )],
    );
    world.begin_registry_configuration();
    assert!(world.finish_registry_configuration().is_err());
}

fn synchronized_biome_nbt(include_effects: bool) -> Vec<u8> {
    let mut out = vec![10];
    test_nbt_byte(&mut out, "has_precipitation", 1);
    test_nbt_float(&mut out, "temperature", 0.35);
    test_nbt_string(&mut out, "temperature_modifier", "frozen");
    test_nbt_float(&mut out, "downfall", 0.8);
    if include_effects {
        test_nbt_compound(&mut out, "effects");
        test_nbt_string(&mut out, "water_color", "#112233");
        test_nbt_string(&mut out, "grass_color_modifier", "swamp");
        out.push(0);
        test_nbt_compound(&mut out, "attributes");
        test_nbt_string(&mut out, "minecraft:visual/fog_color", "#446688");
        test_nbt_string(&mut out, "minecraft:visual/sky_color", "#557799");
        test_nbt_int(&mut out, "minecraft:visual/water_fog_color", 0x6688aa);
        test_nbt_float(&mut out, "minecraft:visual/water_fog_end_distance", 48.0);
        out.push(0);
    }
    out.push(0);
    out
}

fn test_nbt_compound(out: &mut Vec<u8>, name: &str) {
    out.push(10);
    test_nbt_utf(out, name);
}

fn test_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
    out.push(1);
    test_nbt_utf(out, name);
    out.push(value as u8);
}

fn test_nbt_int(out: &mut Vec<u8>, name: &str, value: i32) {
    out.push(3);
    test_nbt_utf(out, name);
    out.extend_from_slice(&value.to_be_bytes());
}

fn test_nbt_float(out: &mut Vec<u8>, name: &str, value: f32) {
    out.push(5);
    test_nbt_utf(out, name);
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn test_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    test_nbt_utf(out, name);
    test_nbt_utf(out, value);
}

fn test_nbt_utf(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

#[test]
fn fade_only_projection_refresh_keeps_layout_revision_stable() {
    let mut projection = TerrainSectionFrameProjection::default();
    let mut residency = TerrainCoordinateMap::default();
    let pos = SectionPosition { x: 0, y: 0, z: 0 };
    let start = Instant::now();
    residency.insert(
        pos,
        TerrainSectionResidency {
            uploaded_at: start,
            fade_duration: Duration::from_millis(750),
            compiled_visibility: TerrainSectionVisibility::default(),
        },
    );

    projection.replace(
        1,
        [VisibleSection {
            position: pos,
            nearby: true,
        }],
        &residency,
        start,
    );
    let layout_rev = projection.layout_revision;
    let initial_rev = projection.revision;
    assert_eq!(projection.fade_states.len(), 1);
    assert_eq!(projection.fade_states[0].1, 0.0);

    // Mid-fade refresh advances visibility and revision without bumping layout_revision.
    assert!(projection.refresh(&residency, start + Duration::from_millis(375)));
    assert_eq!(projection.layout_revision, layout_rev);
    assert!(projection.revision > initial_rev);
    assert_eq!(projection.fade_states.len(), 1);
    assert!((projection.fade_states[0].1 - 0.5).abs() < 0.02);

    // Final refresh to full visibility keeps layout_revision unchanged and clears fading_indices.
    assert!(projection.refresh(&residency, start + Duration::from_millis(800)));
    assert_eq!(projection.layout_revision, layout_rev);
    assert!(!projection.has_fading_sections());
    assert_eq!(projection.fade_states[0].1, 1.0);
}
