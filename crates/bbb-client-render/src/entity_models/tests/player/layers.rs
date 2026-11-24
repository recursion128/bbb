use super::*;

#[test]
fn player_textured_layer_passes_match_vanilla_avatar_renderer_model_layers() {
    let wide = player_textured_layer_passes(false, PLAYER_MODEL_PARTS_ALL_VISIBLE);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    // The unified `PlayerModel` tree drives the geometry, so the layer-pass parts are vestigial; the
    // part visibility still rides the pass to the renderer.
    assert_eq!(
        wide[0].visibility,
        EntityModelLayerVisibility::PlayerParts(PLAYER_MODEL_PARTS_ALL_VISIBLE)
    );
    assert_eq!(wide[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(wide[0].order, 0);

    let slim_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    let slim = player_textured_layer_passes(true, slim_parts);
    assert_eq!(slim.len(), 1);
    assert_eq!(slim[0].texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(
        slim[0].visibility,
        EntityModelLayerVisibility::PlayerParts(slim_parts)
    );
    assert_eq!(slim[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(slim[0].order, 0);
}

#[test]
fn entity_texture_atlas_stitches_official_player_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();

    assert_eq!(layout.entries.len(), 18);
    assert!(rgba.len() >= 18 * 64 * 64 * 4);
}

#[test]
fn player_deadmau5_ears_layer_emits_vanilla_geometry() {
    // Vanilla `Deadmau5EarsLayer`: when `showExtraEars && !isInvisible`, submits
    // `PlayerEarsModel` with the player's body skin, `entitySolid`, `getOverlayCoords(state, 0.0)`,
    // and default order 0 before the cape layer.
    let ears_pass = player_extra_ears_layer_pass_with_texture(PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(
        ears_pass.render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(ears_pass.render_type.vanilla_name(), "entitySolid");
    assert_eq!(ears_pass.texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(ears_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(ears_pass.order, 0);

    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        56,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.kind.player_mut().show_extra_ears = true)
    .with(|s| (s.head_yaw, s.head_pitch) = (25.0, -15.0))
    .with(|s| {
        s.is_crouching = true;
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(meshes.solid.vertices.len(), 48);
    let ears_vertices = &meshes.solid.vertices;
    assert_eq!(ears_vertices.len(), 48);
    assert!(ears_vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]
    }));
}

#[test]
fn ready_dynamic_player_skin_deadmau5_ears_use_dynamic_skin_atlas() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 7702,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        standalone_sampling: None,
        rgba: vec![0xdd; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        57,
        [1.0, 65.0, -3.0],
        15.0,
        EntityPlayerSkin::Dynamic(dynamic_skin),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.kind.player_mut().show_extra_ears = true;
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.has_red_overlay = true;
    });

    let meshes =
        entity_model_textured_meshes_with_dynamic_skins(&[instance], &atlas, Some(&dynamic_atlas));

    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_cutout.vertices.len(), 288);
    assert_eq!(meshes.dynamic_player_skin_solid.vertices.len(), 48);
    assert!(meshes
        .dynamic_player_skin_solid
        .vertices
        .iter()
        .all(|vertex| {
            vertex.tint == [1.0; 4]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]
        }));
}

#[test]
fn player_deadmau5_ears_layer_is_suppressed_when_player_is_invisible() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        58,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.kind.player_mut().show_extra_ears = true)
    .with_invisible(true)
    .with(|s| s.invisible_to_player = false);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.solid.vertices.is_empty());
}

#[test]
fn player_spin_attack_effect_emits_vanilla_geometry() {
    // Vanilla `AvatarRenderer` registers `SpinAttackEffectLayer` after WingsLayer/ParrotOnShoulder:
    // while `isAutoSpinAttack` is set it submits `SpinAttackEffectModel` with the riptide trident
    // texture, default `EntityModel` render type (`entityCutout`), no overlay, and default order 0.
    let pass = player_spin_attack_effect_layer_pass();
    assert_eq!(pass.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(pass.texture, TRIDENT_RIPTIDE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_riptide_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        54,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.age_in_ticks = 2.0;
        s.auto_spin_age_ticks = Some(2.0);
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let riptide_entry = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == TRIDENT_RIPTIDE_TEXTURE_REF)
        .expect("riptide atlas entry");
    let riptide_vertices: Vec<_> = meshes
        .cutout
        .vertices
        .iter()
        .filter(|vertex| {
            vertex.uv[0] >= riptide_entry.uv.min[0]
                && vertex.uv[0] <= riptide_entry.uv.max[0]
                && vertex.uv[1] >= riptide_entry.uv.min[1]
                && vertex.uv[1] <= riptide_entry.uv.max[1]
        })
        .collect();
    assert_eq!(riptide_vertices.len(), 48);
    assert!(riptide_vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
    assert_eq!(meshes.cutout.vertices.len(), 336);
}

#[test]
fn player_spin_attack_missing_texture_atlas_entry_suppresses_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == TRIDENT_RIPTIDE_TEXTURE_REF));
    let instance = EntityModelInstance::player_with_skin(
        55,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.age_in_ticks = 2.0;
        s.auto_spin_age_ticks = Some(2.0);
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == instance.render_state.overlay_coords()));
}

#[test]
fn player_parrot_on_shoulder_layer_emits_vanilla_geometry() {
    // Vanilla `AvatarRenderer` registers `ParrotOnShoulderLayer` after `WingsLayer` and before
    // `SpinAttackEffectLayer`. The layer submits left then right shoulder parrots with
    // `ParrotRenderer.getVariantTexture`, `entityCutout`, player light/outline, and NO_OVERLAY.
    let left_pass = player_parrot_on_shoulder_layer_pass(ParrotModelVariant::Blue);
    assert_eq!(
        left_pass.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(left_pass.render_type.vanilla_name(), "entityCutout");
    assert_eq!(
        left_pass.texture,
        parrot_texture_ref(ParrotModelVariant::Blue)
    );
    assert_eq!(left_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(left_pass.order, 0);

    let right_pass = player_parrot_on_shoulder_layer_pass(ParrotModelVariant::Gray);
    assert_eq!(
        right_pass.texture,
        parrot_texture_ref(ParrotModelVariant::Gray)
    );
    assert_eq!(right_pass.order, 0);

    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_parrot_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        56,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (18.0, -12.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 0.7))
    .with(|s| {
        s.age_in_ticks = 3.5;
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.player_mut().left_shoulder_parrot = Some(ParrotModelVariant::Blue);
        s.kind.player_mut().right_shoulder_parrot = Some(ParrotModelVariant::Gray);
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    for variant in [ParrotModelVariant::Blue, ParrotModelVariant::Gray] {
        let entry = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == parrot_texture_ref(variant))
            .expect("parrot atlas entry");
        let parrot_vertices: Vec<_> = meshes
            .cutout
            .vertices
            .iter()
            .filter(|vertex| {
                vertex.uv[0] >= entry.uv.min[0]
                    && vertex.uv[0] <= entry.uv.max[0]
                    && vertex.uv[1] >= entry.uv.min[1]
                    && vertex.uv[1] <= entry.uv.max[1]
            })
            .collect();
        assert_eq!(parrot_vertices.len(), 264);
        assert!(parrot_vertices.iter().all(|vertex| {
            vertex.tint == [1.0; 4]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        }));
    }
    assert_eq!(meshes.cutout.vertices.len(), 288 + 264 * 2);
}

#[test]
fn player_parrot_on_shoulder_layer_uses_crouching_y_offset() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_and_parrot_texture_images()).unwrap();
    let standing = EntityModelInstance::player(57, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| s.kind.player_mut().left_shoulder_parrot = Some(ParrotModelVariant::Green));
    let crouching = standing.with(|s| s.is_crouching = true);
    let standing_meshes = entity_model_textured_meshes(&[standing], &atlas);
    let crouching_meshes = entity_model_textured_meshes(&[crouching], &atlas);
    assert_eq!(standing_meshes.cutout.vertices.len(), 288 + 264);
    assert_eq!(crouching_meshes.cutout.vertices.len(), 288 + 264);
    for (standing_vertex, crouching_vertex) in standing_meshes.cutout.vertices[288..]
        .iter()
        .zip(&crouching_meshes.cutout.vertices[288..])
    {
        let standing_local = player_model_root_transform(&standing)
            .inverse()
            .transform_point3(Vec3::from_array(standing_vertex.position));
        let crouching_local = player_model_root_transform(&crouching)
            .inverse()
            .transform_point3(Vec3::from_array(crouching_vertex.position));
        assert_close3(
            crouching_local.to_array(),
            (standing_local + Vec3::new(0.0, 0.2, 0.0)).to_array(),
        );
    }
}

#[test]
fn player_shoulder_parrot_missing_texture_atlas_entry_suppresses_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!atlas
        .entries
        .iter()
        .any(|entry| entry.texture == parrot_texture_ref(ParrotModelVariant::YellowBlue)));
    let instance = EntityModelInstance::player_with_skin(
        58,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.kind.player_mut().left_shoulder_parrot = Some(ParrotModelVariant::YellowBlue));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 288);
}

#[test]
fn ready_dynamic_player_skin_body_uses_dynamic_cutout_atlas() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 7701,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let instance = EntityModelInstance::player_with_skin(
        904,
        [1.0, 65.0, -3.0],
        15.0,
        EntityPlayerSkin::Dynamic(dynamic_skin),
        PlayerModelPartVisibility::from_vanilla_mask(
            PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
        ),
    );
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let meshes =
        entity_model_textured_meshes_with_dynamic_skins(&[instance], &atlas, Some(&dynamic_atlas));

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_cutout.cutout_faces, 48);
    assert_eq!(meshes.dynamic_player_skin_cutout.vertices.len(), 192);
}

#[test]
fn first_person_player_arm_submits_selected_arm_as_entity_translucent() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let light = [5.0 / 15.0, 11.0 / 15.0];
    let bare_arm = FirstPersonPlayerArm {
        left: false,
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        sleeve_visible: false,
        transform,
        light,
    };
    let sleeved_arm = FirstPersonPlayerArm {
        sleeve_visible: true,
        ..bare_arm
    };

    let bare = first_person_player_arm_textured_meshes(&[bare_arm], &atlas, None);
    let sleeved = first_person_player_arm_textured_meshes(&[sleeved_arm], &atlas, None);

    assert!(bare.cutout.vertices.is_empty());
    assert_eq!(bare.translucent.vertices.len(), 24);
    assert_eq!(bare.translucent.indices.len(), 36);
    assert_eq!(sleeved.translucent.vertices.len(), 48);
    assert_eq!(sleeved.translucent.indices.len(), 72);
    assert_ne!(bare.translucent.vertices, sleeved.translucent.vertices);
}

/// End-to-end GPU proof of the player-arm content of [`crate::render`]'s `first_person_item_pass`:
/// bakes the real first-person right-arm translucent mesh (`first_person_player_arm_textured_meshes`,
/// the exact geometry the pass draws) in front of a fixed perspective first-person camera, renders it
/// through the real [`create_entity_model_translucent_pipeline`] + entity texture atlas + lightmap (the
/// pass's `entity_model_translucent_pipeline` path), reads the framebuffer back, and asserts the arm's
/// projected centroid shows the arm's texture colour while a far corner stays background — the
/// remaining first-person sentinel state (2). Skips (no assertion) when no GPU adapter is available.
#[test]
fn first_person_player_arm_renders_visible_pixels() {
    use wgpu::util::DeviceExt;

    use crate::camera::{CameraPose, CameraUniform, LightmapEnvironment};
    use crate::entity_models::gpu::create_entity_model_translucent_pipeline;
    use crate::gpu::{create_camera_buffer, create_depth_target, create_terrain_bind_group_layout};
    use crate::lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu,
        create_lightmap_sample_bind_group_layout,
    };

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;
    // Non-sRGB target so the readback bytes are the shader's linear output verbatim. `320 * 4 = 1280`
    // is a multiple of 256, so the texture-to-buffer copy needs no row padding.
    const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: crate::renderer::renderer_instance_backends(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    })) else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bbb-first-person-arm-test-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        ..Default::default()
    })) else {
        return;
    };

    // Fixed first-person camera looking straight down +Z (yaw = pitch = 0); `CameraUniform::from_pose`
    // is the exact perspective `view_proj` the first-person pass uploads, and `camera_world =
    // view.inverse()` mirrors `first_person_camera_world_transform` in bbb-client.
    let aspect = WIDTH as f32 / HEIGHT as f32;
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let camera_uniform = CameraUniform::from_pose(pose, aspect);
    let view_proj = camera_uniform.view_proj();
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    let camera_world = glam::camera::rh::view::look_at_mat4(eye, eye + Vec3::Z, Vec3::Y).inverse();

    // Entity texture atlas filled with opaque green in the WideSteve skin slot, so the arm's textured
    // faces sample an unambiguous, non-background colour (the shipped `steve` test images are a
    // transparent 0-fill, which would render nothing). The atlas layout drives the real arm mesh baker.
    let green = |texture: EntityModelTextureRef| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        let mut rgba = vec![0u8; len];
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[0, 255, 0, 255]);
        }
        EntityModelTextureImage::new(texture, rgba)
    };
    let images = [
        green(PLAYER_WIDE_STEVE_TEXTURE_REF),
        green(PLAYER_SLIM_STEVE_TEXTURE_REF),
    ];
    let (atlas_layout, atlas_rgba) =
        build_entity_model_texture_atlas(&images).expect("entity atlas");

    // Probe the real right-arm mesh at identity to find its model-space centroid, then seat that
    // centroid a fixed offset in front of the camera (`camera_world * T(view_offset)`, scaled down): the
    // arm anchor is derived from the mesh, not guessed. `first_person_player_arm_textured_meshes` is the
    // exact production baker the renderer feeds into the first-person pass.
    let arm = FirstPersonPlayerArm {
        left: false,
        skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        sleeve_visible: false,
        transform: Mat4::IDENTITY,
        light: [5.25 / 15.0, 11.5 / 15.0],
    };
    let probe = first_person_player_arm_textured_meshes(&[arm], &atlas_layout, None);
    assert!(
        !probe.translucent.vertices.is_empty(),
        "arm mesh has geometry"
    );
    let mut centroid = Vec3::ZERO;
    for vertex in &probe.translucent.vertices {
        centroid += Vec3::from_array(vertex.position);
    }
    centroid /= probe.translucent.vertices.len() as f32;

    // Seat the centroid at `VIEW_OFFSET` (right + slightly up + forward) in camera-local space, scaled
    // so the ~12-unit arm becomes a fraction of a metre — small enough that a far corner stays clear.
    const SCALE: f32 = 0.03;
    let view_offset = Vec3::new(0.15, -0.10, -0.75);
    let placed_transform = camera_world
        * Mat4::from_translation(view_offset)
        * Mat4::from_scale(Vec3::splat(SCALE))
        * Mat4::from_translation(-centroid);
    let placed_arm = FirstPersonPlayerArm {
        transform: placed_transform,
        ..arm
    };
    let mesh =
        first_person_player_arm_textured_meshes(&[placed_arm], &atlas_layout, None).translucent;

    // Anchor: the centroid maps to `camera_world * VIEW_OFFSET`; project it through the same `view_proj`
    // the shader uses, then apply wgpu's viewport transform (origin top-left, NDC y up so y is flipped).
    let clip = view_proj * (placed_transform * centroid.extend(1.0));
    let ndc = clip.truncate() / clip.w;
    let anchor_px = ((ndc.x * 0.5 + 0.5) * WIDTH as f32).round() as u32;
    let anchor_py = ((0.5 - ndc.y * 0.5) * HEIGHT as f32).round() as u32;
    assert!(
        anchor_px < WIDTH && anchor_py < HEIGHT,
        "arm centroid projects on-screen, got ({anchor_px},{anchor_py})"
    );

    // Upload the green atlas as the entity texture atlas bound at group 0 (camera @0, texture @1,
    // sampler @2 — the terrain-style layout the entity pipeline shares).
    let bind_group_layout = create_terrain_bind_group_layout(&device);
    let camera_buffer = create_camera_buffer(&device);
    queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fp-arm-test-atlas"),
        size: wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.width * 4),
            rows_per_image: Some(atlas_layout.height),
        },
        wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("fp-arm-test-sampler"),
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fp-arm-test-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas_sampler),
            },
        ],
    });
    let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
    let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
    let lightmap = create_lightmap_gpu(
        &device,
        &queue,
        &lightmap_bind_group_layout,
        &lightmap_sample_bind_group_layout,
        LightmapEnvironment::default(),
    );
    let pipeline = create_entity_model_translucent_pipeline(
        &device,
        COLOR_FORMAT,
        &bind_group_layout,
        &lightmap_sample_bind_group_layout,
        None,
    )
    .expect("embedded entity pipeline");

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("fp-arm-test-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("fp-arm-test-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fp-arm-test-color"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = create_depth_target(&device, WIDTH, HEIGHT);
    let bytes_per_row = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fp-arm-test-readback"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let render_pixels = || {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fp-arm-test-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Blue background — distinct from the green arm.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let data = slice.get_mapped_range().expect("mapped GPU readback");
        let pixel = |x: u32, y: u32| -> [u8; 4] {
            let o = (y * bytes_per_row + x * 4) as usize;
            [data[o], data[o + 1], data[o + 2], data[o + 3]]
        };
        let anchor_pixel = pixel(anchor_px, anchor_py);
        let corner_pixel = pixel(0, 0);
        drop(data);
        readback.unmap();
        (anchor_pixel, corner_pixel)
    };
    let (anchor_pixel, corner_pixel) = render_pixels();

    // (2) arm visible: the arm centroid shows the green arm texture (green dominant, not the blue
    // background); the top-left corner stays blue background.
    assert!(
        anchor_pixel[1] > 64
            && anchor_pixel[1] > anchor_pixel[2]
            && anchor_pixel[0] < anchor_pixel[1],
        "arm centroid should show the green player arm, got {anchor_pixel:?}"
    );
    assert!(
        corner_pixel[2] > 128 && corner_pixel[1] < 128,
        "corner should stay background, got {corner_pixel:?}"
    );

    // Fractional block/sky light coordinates must retain bilinear lightmap sampling.
    let lightmap_rgba = (0..16)
        .flat_map(|sky| (0..16).flat_map(move |block| [0, 20 + block * 8 + sky * 4, 0, 255]))
        .collect::<Vec<u8>>();
    queue.write_texture(
        lightmap._texture.as_image_copy(),
        &lightmap_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16 * 4),
            rows_per_image: Some(16),
        },
        wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
    );
    let (lit_anchor, lit_corner) = render_pixels();
    let expected_green = (f32::from(anchor_pixel[1]) * 108.0 / 255.0).round() as u8;
    assert!(
        lit_anchor[1].abs_diff(expected_green) <= 1,
        "fractional lightmap sample must scale the arm's green channel: white={anchor_pixel:?}, lit={lit_anchor:?}, expected={expected_green}"
    );
    assert_eq!(lit_corner, corner_pixel);

    queue.write_texture(
        lightmap._texture.as_image_copy(),
        &[255; 16 * 16 * 4],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16 * 4),
            rows_per_image: Some(16),
        },
        wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
    );
    let mut vertices = mesh.vertices.clone();
    for (overlay, color, alpha) in [
        ([0.0, 3.0], [255.0, 0.0, 0.0], 178.0 / 255.0),
        ([15.0, 10.0], [255.0; 3], 63.0 / 255.0),
    ] {
        for vertex in &mut vertices {
            vertex.overlay = overlay;
        }
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let (overlay_anchor, overlay_corner) = render_pixels();
        for channel in 0..3 {
            let expected = (color[channel] * (1.0 - alpha)
                + f32::from(anchor_pixel[channel]) * alpha)
                .round() as u8;
            assert!(
                overlay_anchor[channel].abs_diff(expected) <= 1,
                "overlay mixes over directional lighting: overlay={overlay:?}, base={anchor_pixel:?}, actual={overlay_anchor:?}, channel={channel}, expected={expected}"
            );
        }
        assert_eq!(overlay_corner, corner_pixel);
    }
}

#[test]
fn first_person_ready_dynamic_player_arm_uses_dynamic_skin_translucent_bucket() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 8812,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Slim,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_atlas = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: dynamic_skin.handle,
        standalone_sampling: Some(StandaloneTextureSampling::new(true, true)),
        rgba: vec![0x7f; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let arm = FirstPersonPlayerArm {
        left: true,
        skin: EntityPlayerSkin::Dynamic(dynamic_skin),
        sleeve_visible: true,
        transform: Mat4::IDENTITY,
        light: [1.0, 1.0],
    };

    let meshes = first_person_player_arm_textured_meshes(&[arm], &atlas, Some(&dynamic_atlas));

    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.dynamic_player_skin_translucent.vertices.len(), 48);
    assert_eq!(meshes.dynamic_player_skin_translucent.indices.len(), 72);
    assert!(meshes
        .dynamic_player_skin_translucent
        .vertices
        .iter()
        .all(|vertex| {
            vertex.standalone_local_uv == vertex.uv
                && vertex.standalone_pixel_origin == [0.0, 0.0]
                && vertex.standalone_size_flags == [64.0, 64.0, 1.0, 1.0]
        }));
}

#[test]
fn dynamic_player_texture_atlas_stitches_variable_profile_textures() {
    let mut cape = dynamic_player_texture_image(20, [4, 2], 10);
    cape.standalone_sampling = Some(StandaloneTextureSampling::new(true, false));
    let elytra = dynamic_player_texture_image(10, [2, 3], 40);

    let (layout, rgba) = build_dynamic_player_texture_atlas(&[cape.clone(), elytra.clone()])
        .expect("dynamic player texture atlas");

    assert_eq!(layout.width, 4);
    assert_eq!(layout.height, 5);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(layout.entries[0].handle, cape.handle);
    assert_eq!(layout.entries[0].pixel_origin, [0, 0]);
    assert_eq!(layout.entries[0].size, cape.size);
    assert_eq!(
        layout.entries[0].standalone_sampling,
        cape.standalone_sampling
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 2.0 / 5.0]);
    assert_eq!(layout.entries[1].handle, elytra.handle);
    assert_eq!(layout.entries[1].pixel_origin, [0, 2]);
    assert_eq!(layout.entries[1].size, elytra.size);
    assert_eq!(layout.entries[1].standalone_sampling, None);
    assert_close2(layout.entries[1].uv.min, [0.0, 2.0 / 5.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);

    assert_eq!(atlas_pixel(&rgba, layout.width, 3, 1), [13, 3, 1, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 1, 2), [41, 1, 0, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 2, 2), [0, 0, 0, 0]);
}

#[test]
fn dynamic_player_texture_atlas_wraps_columns_at_the_height_limit() {
    let first = dynamic_player_texture_image(20, [4, 2], 10);
    let second = dynamic_player_texture_image(30, [4, 2], 20);
    let third = dynamic_player_texture_image(40, [2, 2], 30);

    let (layout, rgba) = build_dynamic_player_texture_atlas_with_limits(
        &[first.clone(), second.clone(), third.clone()],
        [6, 5],
    )
    .expect("dynamic player texture atlas");

    assert_eq!(layout.width, 6);
    assert_eq!(layout.height, 4);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(layout.entries[0].pixel_origin, [0, 0]);
    assert_eq!(layout.entries[1].pixel_origin, [0, 2]);
    assert_eq!(layout.entries[2].pixel_origin, [4, 0]);
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [4.0 / 6.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [4.0 / 6.0, 1.0]);
    assert_close2(layout.entries[2].uv.min, [4.0 / 6.0, 0.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 0.5]);

    // Row 0 pixel 0 of each image lands at its column origin.
    assert_eq!(atlas_pixel(&rgba, layout.width, 0, 0), [10, 0, 0, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 0, 2), [20, 0, 0, 255]);
    assert_eq!(atlas_pixel(&rgba, layout.width, 4, 0), [30, 0, 0, 255]);
    // The second column below the third image stays empty.
    assert_eq!(atlas_pixel(&rgba, layout.width, 4, 2), [0, 0, 0, 0]);
}

#[test]
fn dynamic_player_texture_atlas_rejects_one_image_taller_than_the_height_limit() {
    let err = build_dynamic_player_texture_atlas_with_limits(
        &[dynamic_player_texture_image(20, [2, 6], 10)],
        [8, 5],
    )
    .unwrap_err();

    assert!(err.to_string().contains("exceeds atlas column limit 5"));
}

#[test]
fn dynamic_player_texture_atlas_rejects_bad_profile_texture_dimensions() {
    let err = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: 88,
        size: [2, 2],
        standalone_sampling: None,
        rgba: vec![0xff; 15],
    }])
    .unwrap_err();

    assert!(err.to_string().contains("expected 16 for 2x2"));
}

#[test]
fn dynamic_player_texture_atlas_rejects_duplicate_profile_texture_handles() {
    let err = build_dynamic_player_texture_atlas(&[
        dynamic_player_texture_image(88, [2, 2], 10),
        dynamic_player_texture_image(88, [1, 3], 20),
    ])
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("duplicate dynamic player texture handle 88"));
}

#[test]
fn dynamic_player_texture_atlas_rejects_zero_sized_profile_textures() {
    assert!(
        build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
            handle: 89,
            size: [0, 2],
            standalone_sampling: None,
            rgba: Vec::new(),
        }])
        .is_err()
    );
}

#[test]
fn ready_dynamic_player_texture_submission_uses_dynamic_texture_atlas_bucket() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_texture = EntityDynamicPlayerTexture {
        handle: 7001,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: dynamic_texture.handle,
        size: [64, 64],
        standalone_sampling: Some(StandaloneTextureSampling::new(false, true)),
        rgba: vec![0xaa; 64 * 64 * 4],
    }])
    .unwrap()
    .0;

    let meshes = dynamic_player_texture_test_meshes(
        EntityModelLayerRenderType::EntityCutout,
        dynamic_texture,
        &static_atlas,
        Some(&dynamic_atlas),
    );

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert_eq!(meshes.dynamic_player_texture_cutout.cutout_faces, 72);
    assert_eq!(meshes.dynamic_player_texture_cutout.vertices.len(), 288);
    assert!(meshes
        .dynamic_player_texture_cutout
        .vertices
        .iter()
        .all(|vertex| {
            vertex.standalone_local_uv == vertex.uv
                && vertex.standalone_pixel_origin == [0.0, 0.0]
                && vertex.standalone_size_flags == [64.0, 64.0, 0.0, 1.0]
        }));
}

#[test]
fn dynamic_player_texture_submission_falls_back_to_static_atlas_when_not_uploaded() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let dynamic_texture = EntityDynamicPlayerTexture {
        handle: 7002,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };

    let meshes = dynamic_player_texture_test_meshes(
        EntityModelLayerRenderType::EntityTranslucent,
        dynamic_texture,
        &static_atlas,
        None,
    );

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 72);
    assert_eq!(meshes.translucent.vertices.len(), 288);
}

#[test]
fn player_cape_layer_uses_dynamic_profile_texture_atlas() {
    let pass = player_cape_layer_pass();
    assert_eq!(pass.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(pass.render_type.vanilla_name(), "entitySolid");
    assert_eq!(pass.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7101,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x66; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        45,
        [1.0, 65.0, -2.0],
        30.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.player_mut().cape_texture = Some(cape_texture);
        s.kind.player_mut().cape_flap = 4.0;
        s.kind.player_mut().cape_lean = 10.0;
        s.kind.player_mut().cape_lean2 = -6.0;
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.dynamic_player_texture_solid.cutout_faces, 6);
    assert_eq!(meshes.dynamic_player_texture_solid.vertices.len(), 24);
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
}

#[test]
fn player_cape_layer_waits_for_dynamic_profile_texture_upload() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7102,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let instance = EntityModelInstance::player_with_skin(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.kind.player_mut().cape_texture = Some(cape_texture));

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.dynamic_player_texture_solid.vertices.is_empty());
}

#[test]
fn player_cape_layer_is_suppressed_when_player_is_invisible() {
    // Vanilla `CapeLayer.submit` gates on `!state.isInvisible`, even when the player is self-visible
    // through the force-transparent base-body path.
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7105,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x99; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        49,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| s.kind.player_mut().cape_texture = Some(cape_texture))
    .with_invisible(true)
    .with(|s| s.invisible_to_player = false);

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.dynamic_player_texture_solid.vertices.is_empty());
}

#[test]
fn player_cape_layer_is_suppressed_by_wings_chest_equipment() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7103,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x77; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        47,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.kind.player_mut().cape_texture = Some(cape_texture);
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(meshes.dynamic_player_texture_solid.vertices.is_empty());
    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
}

#[test]
fn player_cape_layer_offsets_for_humanoid_chest_equipment() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let cape_texture = EntityDynamicPlayerTexture {
        handle: 7104,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: cape_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x88; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        48,
        [3.0, 66.0, -4.0],
        25.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.player_mut().cape_texture = Some(cape_texture);
        s.chest_wings_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.dynamic_player_texture_solid.vertices.len(), 24);
    assert!(meshes
        .dynamic_player_texture_solid
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    let bare = entity_model_textured_meshes_with_dynamic_textures(
        &[instance.with(|s| s.chest_wings_equipment_asset = None)],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );
    let root = player_model_root_transform(&instance);
    let offset = root.transform_vector3(Vec3::new(0.0, -0.053125, 0.06875));
    for (bare, armored) in bare
        .dynamic_player_texture_solid
        .vertices
        .iter()
        .zip(&meshes.dynamic_player_texture_solid.vertices)
    {
        assert_close3(
            armored.position,
            (Vec3::from_array(bare.position) + offset).to_array(),
        );
    }
}

#[test]
fn player_wings_layer_uses_static_equipment_texture() {
    let pass = expected_equipment_wings_pass(ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_and_elytra_texture_images()).unwrap();
    let instance = EntityModelInstance::player_with_skin(
        49,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
    });

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert_eq!(meshes.dynamic_player_texture_cutout.vertices.len(), 0);
    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(meshes.armor_cutout.vertices.len(), 48);
}

#[test]
fn player_wings_layer_static_missing_texture_atlas_entry_suppresses_geometry() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    assert!(!static_atlas
        .entries
        .iter()
        .any(|entry| entry.texture == ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF));
    let instance = EntityModelInstance::player_with_skin(
        53,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
    });

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.armor_cutout.vertices.is_empty());
}

#[test]
fn player_wings_layer_prefers_ready_profile_elytra_texture_over_cape() {
    let pass = expected_equipment_wings_pass(PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, PLAYER_PROFILE_ELYTRA_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 7201,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let profile_cape = EntityDynamicPlayerTexture {
        handle: 7202,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicPlayerTextureImage {
            handle: profile_elytra.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x99; 64 * 32 * 4],
        },
        DynamicPlayerTextureImage {
            handle: profile_cape.handle,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x55; 64 * 32 * 4],
        },
    ])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        50,
        [1.0, 64.0, -1.0],
        10.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
        s.kind.player_mut().cape_texture = Some(profile_cape);
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert!(meshes
        .dynamic_player_texture_armor_cutout
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
    let uv = dynamic_atlas
        .entries
        .iter()
        .find(|entry| entry.handle == profile_elytra.handle)
        .unwrap()
        .uv;
    assert!(meshes
        .dynamic_player_texture_armor_cutout
        .vertices
        .iter()
        .all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
}

#[test]
fn player_wings_layer_uses_ready_profile_cape_texture_when_elytra_is_absent() {
    let pass = expected_equipment_wings_pass(PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(
        pass.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(pass.render_type.vanilla_name(), "armorCutoutNoCull");
    assert_eq!(pass.texture, PLAYER_PROFILE_CAPE_TEXTURE_REF);
    assert_eq!(pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(pass.order, 0);

    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let profile_cape = EntityDynamicPlayerTexture {
        handle: 7203,
        kind: EntityDynamicPlayerTextureKind::Cape,
    };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: profile_cape.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0x55; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::player_with_skin(
        51,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.kind.player_mut().cape_texture = Some(profile_cape);
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &static_atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(
        meshes.dynamic_player_texture_armor_cutout.vertices.len(),
        48
    );
    assert!(meshes
        .dynamic_player_texture_armor_cutout
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
}

#[test]
fn player_wings_layer_waits_for_profile_texture_upload() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_and_elytra_texture_images()).unwrap();
    let profile_elytra = EntityDynamicPlayerTexture {
        handle: 7204,
        kind: EntityDynamicPlayerTextureKind::Elytra,
    };
    let instance = EntityModelInstance::player_with_skin(
        52,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.chest_wings_equipment_asset = Some(test_wings_equipment_asset_id());
        s.kind.player_mut().elytra_texture = Some(profile_elytra);
    });

    let meshes =
        entity_model_textured_meshes_with_dynamic_textures(&[instance], &static_atlas, None, None);

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_armor_cutout
        .vertices
        .is_empty());
}

#[test]
fn glowing_dynamic_player_skin_and_texture_emit_outlines_without_panicking_when_invisible() {
    let (static_atlas, _) =
        build_entity_model_texture_atlas(&steve_and_elytra_texture_images()).unwrap();
    let skin_handle = 8101;
    let (skin_atlas, _) = build_dynamic_player_skin_atlas(&[DynamicPlayerSkinImage {
        handle: skin_handle,
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap();
    let tex_handle = 8102;
    let (tex_atlas, _) = build_dynamic_player_texture_atlas(&[DynamicPlayerTextureImage {
        handle: tex_handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap();
    let outline_color = 0xff00_ff80;
    let dynamic_skin = EntityPlayerSkin::Dynamic(EntityDynamicPlayerSkin {
        handle: skin_handle,
        model: EntityPlayerSkinModel::Wide,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        status: EntityDynamicPlayerSkinStatus::Ready,
    });

    // Invisible + glowing dynamic-skin player takes `EntityModelLayerRenderBucket::OutlineOnly`.
    let invisible_player = EntityModelInstance::player_with_skin(
        60,
        [0.0, 64.0, 0.0],
        0.0,
        dynamic_skin,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.invisible = true;
        s.invisible_to_player = true;
        s.appears_glowing = true;
        s.outline_color = outline_color;
    });
    let invisible_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[invisible_player],
        &static_atlas,
        Some(&skin_atlas),
        Some(&tex_atlas),
    );
    assert!(!invisible_meshes
        .dynamic_player_skin_outline
        .vertices
        .is_empty());
    assert!(invisible_meshes
        .dynamic_player_skin_cutout
        .vertices
        .is_empty());

    // Visible + glowing dynamic-skin player + dynamic entity texture emit outline copies too.
    let visible_player = EntityModelInstance::player_with_skin(
        61,
        [2.0, 64.0, 0.0],
        0.0,
        dynamic_skin,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with(|s| {
        s.appears_glowing = true;
        s.outline_color = outline_color;
    });
    let mut invisible_variant_mob =
        EntityModelInstance::new(62, EntityModelKind::Creeper, [4.0, 64.0, 0.0], 0.0).with(|s| {
            s.invisible = true;
            s.invisible_to_player = true;
            s.appears_glowing = true;
            s.outline_color = outline_color;
        });
    invisible_variant_mob.dynamic_texture = Some(EntityDynamicTexture { handle: tex_handle });
    let combined = entity_model_textured_meshes_with_dynamic_textures(
        &[visible_player, invisible_variant_mob],
        &static_atlas,
        Some(&skin_atlas),
        Some(&tex_atlas),
    );
    assert!(
        !combined.dynamic_player_skin_outline.vertices.is_empty()
            || !combined
                .dynamic_player_skin_outline_cull
                .vertices
                .is_empty()
    );
    assert!(!combined.dynamic_player_texture_outline.vertices.is_empty());
}
