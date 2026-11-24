use super::*;

#[test]
fn input_latency_state_is_monotonic_bounded_and_reports_software_stages() {
    let start = Instant::now();
    let stamp = |sequence, superseded_events| RendererInputLatencyStamp {
        sequence,
        enqueued_at: start,
        simulation_applied_at: start + Duration::from_millis(2),
        renderer_extracted_at: start + Duration::from_millis(3),
        superseded_events,
    };
    let mut state = RendererInputLatencyState::default();
    state.push(stamp(10, 1));
    state.push(stamp(10, 99));
    state.push(stamp(9, 0));
    state.push(stamp(12, 2));

    let profile = state
        .complete(
            start + Duration::from_millis(5),
            start + Duration::from_millis(6),
        )
        .expect("newest input completes once");
    assert_eq!(profile.sequence, 12);
    // The simulation publication owner already accounts for any input
    // receipt replaced before present; renderer association must not
    // double-count that loss.
    assert_eq!(profile.superseded_events, 2);
    assert_eq!(profile.out_of_order_events, 1);
    assert_eq!(
        profile.enqueue_to_simulation_apply,
        Duration::from_millis(2)
    );
    assert_eq!(
        profile.simulation_apply_to_renderer_extract,
        Duration::from_millis(1)
    );
    assert_eq!(
        profile.renderer_extract_to_queue_submit,
        Duration::from_millis(2)
    );
    assert_eq!(
        profile.queue_submit_to_present_call_return,
        Duration::from_millis(1)
    );
    assert_eq!(
        profile.enqueue_to_present_call_return,
        Duration::from_millis(6)
    );
    assert!(state
        .complete(
            start + Duration::from_millis(7),
            start + Duration::from_millis(8),
        )
        .is_none());
}

#[test]
fn camera_frame_publishes_all_uniform_inputs_once() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(1920, 1080) else {
        return;
    };
    let baseline = renderer.camera_uniform_writes.get();
    let pose = CameraPose {
        position: [8.0, 72.0, 8.0],
        y_rot: 45.0,
        x_rot: -12.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let lightmap_environment = LightmapEnvironment {
        sky_factor: 0.5,
        ..LightmapEnvironment::default()
    };

    renderer.set_camera_frame(RendererCameraFrame {
        lightmap_environment,
        fog_environment: crate::FogEnvironment::world([0.1, 0.2, 0.3, 1.0], 0.0, 96.0, 12),
        sky_environment: crate::SkyEnvironment::from_rgb([0.2, 0.3, 0.4]),
        pose: Some(pose),
        hurt_effect: Some(crate::CameraHurtEffect {
            hurt_time: 4.0,
            hurt_duration: 10.0,
            hurt_dir: 15.0,
            death_time: 0.0,
            is_dead_or_dying: false,
            damage_tilt_strength: 1.0,
        }),
        view_bob_effect: Some(crate::CameraViewBobEffect {
            backwards_interpolated_walk_distance: 2.0,
            bob: 0.25,
        }),
        spinning_effect: Some(crate::CameraSpinningEffect {
            intensity: 0.2,
            angle_degrees: 30.0,
        }),
        fov_degrees: 90.0,
        hud_fov_degrees: 80.0,
        shader_game_time_ticks: 6000.5,
    });

    assert_eq!(renderer.camera_uniform_writes.get(), baseline + 1);
    assert_eq!(renderer.camera_pose, Some(pose));
    assert_eq!(renderer.lightmap_environment, lightmap_environment);
    assert_eq!(renderer.camera_fov_degrees, 90.0);
    assert_eq!(renderer.camera_hud_fov_degrees, 80.0);
    assert_eq!(renderer.shader_game_time_ticks, 6000.5);
    assert!(renderer.lightmap_dirty);

    // The frame boundary still refreshes real-time glint state while the
    // simulation is paused, but never more than once per publication.
    renderer.set_camera_frame(RendererCameraFrame {
        lightmap_environment,
        fog_environment: renderer.fog_environment,
        sky_environment: renderer.sky.environment,
        pose: Some(pose),
        hurt_effect: renderer.camera_hurt_effect,
        view_bob_effect: renderer.camera_view_bob_effect,
        spinning_effect: renderer.camera_spinning_effect,
        fov_degrees: 90.0,
        hud_fov_degrees: 80.0,
        shader_game_time_ticks: 6000.5,
    });
    assert_eq!(renderer.camera_uniform_writes.get(), baseline + 2);
}

#[test]
fn gui_item_camera_viewport_updates_even_when_ceil_layout_is_unchanged() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(801, 603) else {
        return;
    };
    renderer.set_gui_viewport(PhysicalSize::new(401, 302), 2);
    assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(401, 302));
    assert_eq!(
        renderer.physical_surface_size(),
        PhysicalSize::new(801, 603)
    );
    assert_eq!(renderer.gui_projection_extent(), [400.5, 301.5]);
    let writes = renderer.camera_uniform_writes.get();
    renderer.set_gui_viewport(PhysicalSize::new(401, 302), 2);
    assert_eq!(renderer.camera_uniform_writes.get(), writes);
    renderer.resize(PhysicalSize::new(802, 604)).unwrap();
    renderer.set_gui_viewport(PhysicalSize::new(401, 302), 2);
    assert_eq!(renderer.gui_surface_size(), PhysicalSize::new(401, 302));
    assert_eq!(renderer.gui_projection_extent(), [401.0, 302.0]);
    assert!(renderer.camera_uniform_writes.get() > writes);
    let writes = renderer.camera_uniform_writes.get();
    renderer.set_gui_viewport(PhysicalSize::new(401, 302), 1);
    assert_eq!(renderer.gui_projection_extent(), [802.0, 604.0]);
    assert_eq!(
        renderer.physical_surface_size(),
        PhysicalSize::new(802, 604)
    );
    assert!(renderer.camera_uniform_writes.get() > writes);
}

#[test]
fn choose_format_rejects_non_screenshot_readback_formats() {
    let err = choose_format(&[
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Depth24Plus,
    ])
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("surface does not expose a Vanilla-compatible RGBA/BGRA 8-bit UNORM format"));
}

#[test]
fn choose_present_mode_tracks_vsync_preference() {
    assert_eq!(
        choose_present_mode(
            &[
                wgpu::PresentMode::Fifo,
                wgpu::PresentMode::Immediate,
                wgpu::PresentMode::Mailbox,
            ],
            true,
        )
        .unwrap(),
        wgpu::PresentMode::Fifo
    );
    assert_eq!(
        choose_present_mode(
            &[
                wgpu::PresentMode::Fifo,
                wgpu::PresentMode::Immediate,
                wgpu::PresentMode::Mailbox,
            ],
            false,
        )
        .unwrap(),
        wgpu::PresentMode::Immediate
    );
    assert_eq!(
        choose_present_mode(
            &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox],
            false
        )
        .unwrap(),
        wgpu::PresentMode::Mailbox
    );
    assert_eq!(
        choose_present_mode(&[wgpu::PresentMode::Fifo], false).unwrap(),
        wgpu::PresentMode::Fifo
    );
}

#[test]
fn terrain_residency_observation_crosses_the_renderer_counter_boundary() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        return;
    };
    renderer.set_terrain_mesh_worker_counters(crate::TerrainMeshWorkerCounters {
        selected_chunks: 625,
        resident_chunks: 625,
        selected_sections: 15_000,
        resident_sections: 15_000,
        pending_jobs: 0,
        cache_ready: true,
        ..Default::default()
    });

    assert_eq!(renderer.counters().terrain_selected_chunks, 625);
    assert_eq!(renderer.counters().terrain_resident_chunks, 625);
    assert_eq!(renderer.counters().terrain_selected_sections, 15_000);
    assert_eq!(renderer.counters().terrain_resident_sections, 15_000);
    assert_eq!(renderer.counters().terrain_pending_jobs, 0);
    assert!(renderer.counters().terrain_cache_ready);
}
