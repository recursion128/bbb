use super::*;

#[test]
fn visibility_first_culling_applies_distance_frustum_and_leash_union() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let front = render_candidate(1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, 10.0]);
    let behind = render_candidate(2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, -10.0]);
    assert!(entity_renderer_should_render(
        front,
        &[front],
        &HashMap::from([(front.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    assert!(!entity_renderer_should_render(
        behind,
        &[behind],
        &HashMap::from([(behind.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    let holder = render_candidate(3, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, 10.0]);
    let mut leashed = render_candidate(4, VANILLA_ENTITY_TYPE_COW_ID, [30.0, 0.0, 10.0]);
    leashed.leash_holder_id = Some(holder.entity_id);
    let candidates = [leashed, holder];
    assert!(entity_renderer_should_render(
        leashed,
        &candidates,
        &HashMap::from([(leashed.entity_id, 0), (holder.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));

    let local_player = render_candidate(5, VANILLA_ENTITY_TYPE_PLAYER_ID, [0.0, 0.0, 0.0]);
    let mut first_vehicle = render_candidate(6, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, -10.0]);
    let root_vehicle = render_candidate(7, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, -10.0]);
    let mut riding_player = local_player;
    riding_player.vehicle_id = Some(first_vehicle.entity_id);
    first_vehicle.vehicle_id = Some(root_vehicle.entity_id);
    let candidates = [riding_player, first_vehicle, root_vehicle];
    let index = HashMap::from([
        (riding_player.entity_id, 0),
        (first_vehicle.entity_id, 1),
        (root_vehicle.entity_id, 2),
    ]);
    assert!(candidate_has_local_player_indirect_passenger(
        root_vehicle.entity_id,
        Some(riding_player.entity_id),
        &candidates,
        &index,
    ));
    assert!(!candidate_has_local_player_indirect_passenger(
        99,
        Some(riding_player.entity_id),
        &candidates,
        &index,
    ));

    let mut fishing_hook =
        render_candidate(8, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, [0.0, 0.0, 10.0]);
    assert!(!entity_renderer_should_render(
        fishing_hook,
        &[fishing_hook],
        &HashMap::from([(fishing_hook.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    fishing_hook.fishing_hook_owner_id = Some(local_player.entity_id);
    let candidates = [fishing_hook, local_player];
    assert!(entity_renderer_should_render(
        fishing_hook,
        &candidates,
        &HashMap::from([(fishing_hook.entity_id, 0), (local_player.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));
    fishing_hook.position.z = -10.0;
    let candidates = [fishing_hook, local_player];
    assert!(!entity_renderer_should_render(
        fishing_hook,
        &candidates,
        &HashMap::from([(fishing_hook.entity_id, 0), (local_player.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));
}

#[test]
fn entity_distance_scaling_multiplies_the_shared_vanilla_view_scale() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let candidate = render_candidate(9, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, 80.0]);

    assert!(!entity_passes_vanilla_render_distance(
        candidate, camera, 8, 0.75,
    ));
    assert!(entity_passes_vanilla_render_distance(
        candidate, camera, 8, 1.0,
    ));
    assert!(entity_passes_vanilla_render_distance(
        candidate, camera, 8, 1.25,
    ));
}

#[test]
fn specialized_culling_uses_exact_guardian_crystal_and_shulker_fallbacks() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);

    let mut guardian = render_candidate(20, VANILLA_ENTITY_TYPE_GUARDIAN_ID, [0.0, 0.0, -10.0]);
    guardian.special_culling = EntityRendererSpecialCullingState::Guardian {
        attack_target_id: 21,
        eye_height: 1.5,
    };
    let target = render_candidate(21, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, 10.0]);
    let candidates = [guardian, target];
    let index = HashMap::from([(guardian.entity_id, 0), (target.entity_id, 1)]);
    assert!(entity_renderer_should_render(
        guardian,
        &candidates,
        &index,
        camera,
        frustum,
        12,
    ));

    let non_living_target = render_candidate(21, VANILLA_ENTITY_TYPE_ITEM_ID, [0.0, 0.0, 10.0]);
    let candidates = [guardian, non_living_target];
    assert!(!entity_renderer_should_render(
        guardian,
        &candidates,
        &index,
        camera,
        frustum,
        12,
    ));

    let mut crystal = render_candidate(
        30,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [0.0, 0.0, -10_000.0],
    );
    crystal.special_culling = EntityRendererSpecialCullingState::EndCrystal {
        has_beam_target: false,
    };
    assert!(!entity_renderer_should_render(
        crystal,
        &[crystal],
        &HashMap::from([(crystal.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    crystal.special_culling = EntityRendererSpecialCullingState::EndCrystal {
        has_beam_target: true,
    };
    assert!(entity_renderer_should_render(
        crystal,
        &[crystal],
        &HashMap::from([(crystal.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    let mut shulker = render_candidate(40, VANILLA_ENTITY_TYPE_SHULKER_ID, [0.0, 0.0, -10.0]);
    shulker.special_culling = EntityRendererSpecialCullingState::Shulker {
        render_offset_at_zero: Some(EntityVec3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        }),
        half_width: 0.5,
        half_height: 0.5,
    };
    assert!(entity_renderer_should_render(
        shulker,
        &[shulker],
        &HashMap::from([(shulker.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    shulker.special_culling = EntityRendererSpecialCullingState::Shulker {
        render_offset_at_zero: None,
        half_width: 0.5,
        half_height: 0.5,
    };
    assert!(!entity_renderer_should_render(
        shulker,
        &[shulker],
        &HashMap::from([(shulker.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
}

#[test]
fn specialized_culling_bounds_use_current_tick_vanilla_endpoints() {
    let mut guardian = render_candidate(50, VANILLA_ENTITY_TYPE_GUARDIAN_ID, [2.0, 3.0, -10.0]);
    guardian.special_culling = EntityRendererSpecialCullingState::Guardian {
        attack_target_id: 51,
        eye_height: 1.5,
    };
    let target = render_candidate(51, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [-4.0, 5.0, 10.0]);
    assert_eq!(
        guardian_renderer_culling_bounds(
            guardian,
            &[guardian, target],
            &HashMap::from([(guardian.entity_id, 0), (target.entity_id, 1)]),
        ),
        Some(([-4.0, 4.5, -10.0], [2.0, 6.0, 10.0]))
    );
    guardian.special_culling = EntityRendererSpecialCullingState::Guardian {
        attack_target_id: 0,
        eye_height: 1.5,
    };
    assert_eq!(
        guardian_renderer_culling_bounds(
            guardian,
            &[guardian, target],
            &HashMap::from([(guardian.entity_id, 0), (target.entity_id, 1)]),
        ),
        None
    );
    guardian.special_culling = EntityRendererSpecialCullingState::Guardian {
        attack_target_id: 999,
        eye_height: 1.5,
    };
    assert_eq!(
        guardian_renderer_culling_bounds(
            guardian,
            &[guardian, target],
            &HashMap::from([(guardian.entity_id, 0), (target.entity_id, 1)]),
        ),
        None
    );

    let mut shulker = render_candidate(60, VANILLA_ENTITY_TYPE_SHULKER_ID, [10.2, 64.9, -3.8]);
    shulker.special_culling = EntityRendererSpecialCullingState::Shulker {
        render_offset_at_zero: Some(EntityVec3 {
            x: -2.0,
            y: 1.0,
            z: 4.0,
        }),
        half_width: 0.5,
        half_height: 0.5,
    };
    assert_eq!(
        shulker_renderer_culling_bounds(shulker),
        Some(([-2.5, 1.0, -4.0], [11.0, 65.0, 4.5]))
    );
}

#[test]
fn cached_entity_candidates_follow_moving_mounts_and_world_revisions() {
    let mut world = WorldStore::new();
    insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: 0 });
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_COW_ID,
        [1.0, 64.0, 1.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [8.0, 64.0, 1.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![11],
    }));
    assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
        id: 10,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);

    let mut scratch = EntityRenderExtractionScratch::default();
    scratch.update_candidates(&world, 1, 0.0);
    let start = scratch.candidates.clone();
    scratch.update_candidates(&world, 1, 0.5);
    assert_eq!(
        scratch.candidates,
        world.entity_render_candidates_at_partial_tick(0.5)
    );
    for id in [10, 11] {
        let index = scratch.candidate_index[&id];
        assert!(scratch.candidates[index].position.x > start[index].position.x);
        assert_eq!(
            scratch.candidates[index].current_position,
            start[index].current_position
        );
    }

    world.apply_remove_entities(bbb_protocol::packets::RemoveEntities {
        entity_ids: vec![10],
    });
    scratch.update_candidates(&world, 2, 0.5);
    assert_eq!(
        scratch.candidates,
        world.entity_render_candidates_at_partial_tick(0.5)
    );
    assert_eq!(scratch.candidate_index.len(), 1);
    assert_eq!(scratch.candidate_index.get(&11), Some(&0));
}

#[test]
fn visible_entity_render_state_count_reads_the_current_survivor_owner() {
    let mut scratch = EntityRenderExtractionScratch::default();
    scratch.survivors.extend([
        render_candidate(1, VANILLA_ENTITY_TYPE_PLAYER_ID, [0.0, 64.0, 0.0]),
        render_candidate(2, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, [0.0, 64.0, -2.0]),
    ]);
    scratch.observation.survivors = 99;

    assert_eq!(scratch.visible_entity_render_state_count(), 2);
}

#[test]
fn visibility_first_culling_keeps_catalog_exceptions_and_custom_bounds() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let lightning = render_candidate(1, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, [0.0, 0.0, -10.0]);
    assert!(entity_renderer_should_render(
        lightning,
        &[lightning],
        &HashMap::from([(lightning.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    let sniffer = render_candidate(2, VANILLA_ENTITY_TYPE_SNIFFER_ID, [0.0, 4.0, 10.0]);
    let (sniffer_min, sniffer_max) = entity_renderer_culling_bounds(sniffer, true);
    for (actual, expected) in sniffer_min.into_iter().zip([-1.6, 2.9, 8.4]) {
        assert!((actual - expected).abs() < 1.0e-5);
    }
    for (actual, expected) in sniffer_max.into_iter().zip([1.6, 7.1, 11.6]) {
        assert!((actual - expected).abs() < 1.0e-5);
    }

    let mut zombie = render_candidate(3, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 4.0, 10.0]);
    zombie.wears_dragon_head = true;
    let (dragon_head_min, dragon_head_max) = entity_renderer_culling_bounds(zombie, true);
    assert_eq!(dragon_head_min, [-1.5, 3.0, 8.5]);
    assert_eq!(dragon_head_max, [1.5, 7.0, 11.5]);
}

#[test]
fn display_visibility_uses_view_range_and_metadata_owned_culling_box() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let mut display = render_candidate(10, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, [0.0, 0.0, -10.0]);
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 0.0,
        height: 0.0,
    });
    assert!(entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 4.0,
        height: 6.0,
    });
    assert!(!entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.position = EntityVec3 {
        x: 0.0,
        y: 0.0,
        z: 100.0,
    };
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 0.0,
        height: 0.0,
    });
    assert!(!entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.position = EntityVec3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 2.0,
        width: 4.0,
        height: 6.0,
    });
    assert_eq!(
        entity_renderer_culling_bounds(display, true),
        ([-1.5, 1.5, 0.5], [3.5, 8.5, 5.5])
    );
}

#[test]
fn entity_scene_outline_is_none_without_visible_entity_targets() {
    assert_eq!(
        entity_scene_outline_from_world_at_partial_tick(&WorldStore::new(), 1.0),
        None
    );
}

#[test]
fn entity_scene_outline_projects_missing_server_entity_labels_when_debug_flag_is_enabled() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));

    let default_outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected default client-side hitbox outline");
    assert!(default_outline.text_labels.is_empty());

    let outline =
        entity_scene_outline_from_world_at_partial_tick_with_server_details(&world, 1.0, true)
            .expect("expected local-server debug entity outline");

    assert_eq!(outline.text_labels.len(), 1);
    assert_eq!(
        outline.text_labels[0].text,
        ENTITY_MISSING_SERVER_LABEL_TEXT
    );
    assert_eq!(
        outline.text_labels[0].color,
        ENTITY_MISSING_SERVER_LABEL_COLOR
    );
    assert!(outline.text_labels[0].centered);
    assert_selection_box_close(outline.text_labels[0].position, [0.0, 3.2, 3.0]);
}

#[test]
fn entity_scene_outline_projects_pick_bounds_for_all_visible_targets() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [2.0, 1.0, 3.0],
    ));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.5)
        .expect("expected entity scene outline");

    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 2);
    assert_eq!(outline.points.len(), 2);
    assert_selection_box_close(outline.colored_boxes[0].min, [-0.49, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[0].max, [0.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_selection_box_close(outline.colored_boxes[1].min, [1.51, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[1].max, [2.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert!((outline.lines[0].to[2] - outline.lines[0].from[2] - 2.0).abs() < 1.0e-5);
    assert_selection_box_close(outline.points[0].position, [0.0, 1.0, 3.0]);
    assert_eq!(outline.points[0].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.points[0].size, ENTITY_POSITION_POINT_SIZE);
    assert_selection_box_close(outline.points[1].position, [2.0, 1.0, 3.0]);
}

#[test]
fn entity_scene_outline_projects_passenger_vehicle_slab() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 90,
        passenger_ids: vec![91],
    }));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected passenger vehicle debug outline");

    assert_eq!(
        outline.colored_boxes[2].color,
        ENTITY_PASSENGER_VEHICLE_COLOR
    );
    assert_selection_box_close(outline.colored_boxes[2].min, [0.8, 64.1875, -2.2]);
    assert_selection_box_close(outline.colored_boxes[2].max, [1.2, 64.25, -1.8]);
}

#[test]
fn entity_scene_outline_uses_bounds_without_pick_radius_inflation() {
    let outline_box = entity_pick_target_box(EntityPickTargetState {
        entity_id: 7,
        position: EntityVec3 {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        },
        bounds: EntityPickBoundsState::from_centered_size(2.0, 4.0, 6.0, 1.5),
    });

    assert_selection_box_close(outline_box.min, [9.0, 18.0, 27.0]);
    assert_selection_box_close(outline_box.max, [11.0, 22.0, 33.0]);
}

#[test]
fn entity_scene_outline_filters_local_player_and_camera_entity() {
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(10));
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [2.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [4.0, 1.0, 3.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected non-camera entity scene outline");

    assert_eq!(outline.colored_boxes.len(), 1);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert_selection_box_close(outline.colored_boxes[0].min, [3.51, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[0].max, [4.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_selection_box_close(outline.points[0].position, [4.0, 1.0, 3.0]);
}

#[test]
fn entity_scene_outline_projects_living_eye_height_box_and_view_vector() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        20,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [0.0, 1.0, 3.0],
        90.0,
        -30.0,
        90.0,
    ));
    let eye_height = world.probe_entity_camera_pose(20).unwrap().eye_height;

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected living entity debug outline");

    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_EYE_HEIGHT_COLOR);
    assert_selection_box_close(
        outline.colored_boxes[1].min,
        [-0.3, 1.0 + eye_height - 0.01, 2.7],
    );
    assert_selection_box_close(
        outline.colored_boxes[1].max,
        [0.3, 1.0 + eye_height + 0.01, 3.3],
    );
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert_selection_box_close(outline.lines[0].from, [0.0, 1.0 + eye_height, 3.0]);
    assert_selection_box_close(outline.lines[0].to, [-1.7320508, 2.0 + eye_height, 3.0]);
    assert_selection_box_close(outline.points[0].position, [0.0, 1.0, 3.0]);
    assert_eq!(outline.points[0].size, ENTITY_POSITION_POINT_SIZE);
}

#[test]
fn entity_scene_outline_projects_ender_dragon_parent_and_part_hitboxes() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [0.0, 64.0, 0.0],
    ));
    world.advance_entity_client_animations(1);

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected dragon debug outline");

    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 10);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert!(
        (outline.colored_boxes[0].max[0] - outline.colored_boxes[0].min[0] - 16.0).abs() < 1e-5
    );
    assert!((outline.colored_boxes[0].max[1] - outline.colored_boxes[0].min[1] - 8.0).abs() < 1e-5);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_EYE_HEIGHT_COLOR);
    assert_selection_box_close(outline.colored_boxes[1].min, [-8.0, 70.79, -8.0]);
    assert_selection_box_close(outline.colored_boxes[1].max, [8.0, 70.81, 8.0]);
    assert_eq!(outline.points.len(), 1);
    assert_eq!(outline.points[0].position, [0.0, 64.0, 0.0]);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert_selection_box_close(outline.lines[0].from, [0.0, 70.8, 0.0]);
    assert_selection_box_close(outline.lines[0].to, [0.0, 70.8, 2.0]);
    for part_box in &outline.colored_boxes[2..] {
        assert_eq!(part_box.color, ENTITY_DRAGON_PART_HITBOX_COLOR);
    }
    assert!((outline.colored_boxes[2].max[0] - outline.colored_boxes[2].min[0] - 1.0).abs() < 1e-5);
    assert!((outline.colored_boxes[2].max[1] - outline.colored_boxes[2].min[1] - 1.0).abs() < 1e-5);
    assert!((outline.colored_boxes[8].max[0] - outline.colored_boxes[8].min[0] - 4.0).abs() < 1e-5);
    assert!((outline.colored_boxes[8].max[1] - outline.colored_boxes[8].min[1] - 2.0).abs() < 1e-5);
}

#[test]
fn solid_wall_culls_occluded_mob_models_while_preserving_count_glowing_and_players() {
    use crate::occlusion_culling::tests::{populate_test_world_blocks, stone_state_id};

    let stone = stone_state_id();
    let mut blocks = Vec::new();
    for y in 63..=67 {
        for x in -2..=2 {
            blocks.push((x, y, 5, stone));
        }
    }
    let mut world = WorldStore::new();
    populate_test_world_blocks(&mut world, &blocks);
    world.apply_add_entity(protocol_add_entity(
        1,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [0.5, 64.0, 8.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        2,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [0.5, 64.0, 8.0],
    ));
    let glowing_mask = entity_metadata_accessors().glowing_mask;
    world.apply_set_entity_data(SetEntityData {
        id: 2,
        values: vec![entity_shared_flags_data(glowing_mask)],
    });
    world.apply_add_entity(protocol_add_entity(
        3,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.5, 64.0, 8.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        4,
        VANILLA_ENTITY_TYPE_COW_ID,
        [0.5, 64.0, 3.0],
    ));

    let mut upload = TerrainUploadState::default();
    upload.mark_section_occluder_ready_for_test(&world, 0, 3, 0, 1.0);
    upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 1.0);
    upload.mark_section_occluder_ready_for_test(&world, -1, 3, 0, 1.0);
    upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);

    let camera = CameraPose {
        position: [0.5, 65.5, 1.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let textures = NativeEntityVariantTextureRuntime::new(
        None,
        crate::entity_assets::PreparedEntityTextures::default(),
    );
    let mut scratch = EntityRenderExtractionScratch::default();
    let visibility = EntityRenderVisibilityContext {
        world_revision: 1,
        camera_pose: Some(camera),
        camera_detached: false,
        surface_size: [1280, 720],
        camera_fov_degrees: 70.0,
        base_camera_fov_degrees: 70.0,
        render_distance_chunks: 8,
        entity_distance_scaling: 1.0,
        terrain_upload: &upload,
        entity_shadows: true,
        shadow_sky_darken: 0,
    };
    let instances = visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(
        &world,
        None,
        &textures,
        1.0,
        visibility,
        &mut scratch,
    );

    assert_eq!(scratch.visible_entity_render_state_count(), 4);
    assert_eq!(
        scratch
            .occluded_candidates()
            .iter()
            .map(|c| c.entity_id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    let rendered_ids = instances.iter().map(|i| i.entity_id).collect::<Vec<_>>();
    assert!(!rendered_ids.contains(&1));
    assert!(rendered_ids.contains(&2));
    assert!(rendered_ids.contains(&3));
    assert!(rendered_ids.contains(&4));
    world.apply_set_entity_link(bbb_protocol::packets::SetEntityLink {
        source_id: 4,
        dest_id: 1,
    });
    let leashed = visible_entity_model_instances_from_world_with_variant_textures_at_partial_tick(
        &world,
        None,
        &textures,
        1.0,
        EntityRenderVisibilityContext {
            world_revision: 2,
            ..visibility
        },
        &mut scratch,
    );
    assert!(leashed.iter().any(|instance| instance.entity_id == 1));
}
