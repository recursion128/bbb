use super::{
    entity_model_cutout_z_offset_shader, entity_model_glint_shader,
    entity_model_instances_have_camera_billboards, entity_model_portal_shader,
    entity_model_solid_shader, entity_model_solid_z_offset_forward_shader,
    entity_model_tiled_entity_shader, update_entity_model_textured_mesh_gpu_from_mesh,
    win_screen_end_portal_shader, DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY,
    ENTITY_MODEL_ADDITIVE_BLEND, ENTITY_MODEL_ARMOR_CULL_MODE, ENTITY_MODEL_ARMOR_CUTOUT_BLEND,
    ENTITY_MODEL_ARMOR_DECAL_DEPTH_COMPARE, ENTITY_MODEL_ARMOR_DECAL_DEPTH_WRITE_ENABLED,
    ENTITY_MODEL_ARMOR_DEPTH_WRITE_ENABLED, ENTITY_MODEL_ARMOR_SHADER,
    ENTITY_MODEL_ARMOR_TRANSLUCENT_BLEND, ENTITY_MODEL_BEACON_BEAM_CULL_MODE,
    ENTITY_MODEL_BEACON_BEAM_SHADER, ENTITY_MODEL_BEAM_DEPTH_COMPARE,
    ENTITY_MODEL_BEAM_OPAQUE_BLEND, ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED,
    ENTITY_MODEL_BEAM_TRANSLUCENT_BLEND, ENTITY_MODEL_BEAM_TRANSLUCENT_DEPTH_WRITE_ENABLED,
    ENTITY_MODEL_DISSOLVE_SHADER, ENTITY_MODEL_DRAGON_RAYS_BLEND,
    ENTITY_MODEL_DRAGON_RAYS_CULL_MODE, ENTITY_MODEL_DRAGON_RAYS_DEPTH_COMPARE,
    ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_COLOR_WRITE_MASK,
    ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_WRITE_ENABLED,
    ENTITY_MODEL_DRAGON_RAYS_DEPTH_WRITE_ENABLED, ENTITY_MODEL_END_GATEWAY_LAYERS,
    ENTITY_MODEL_END_PORTAL_LAYERS, ENTITY_MODEL_EYES_BLEND, ENTITY_MODEL_EYES_CULL_MODE,
    ENTITY_MODEL_EYES_DEPTH_WRITE_ENABLED, ENTITY_MODEL_GLINT_BLEND,
    ENTITY_MODEL_GLINT_DEPTH_COMPARE, ENTITY_MODEL_GLINT_DEPTH_WRITE_ENABLED,
    ENTITY_MODEL_OUTLINE_BLEND, ENTITY_MODEL_OUTLINE_CULL_MODE, ENTITY_MODEL_OUTLINE_NO_CULL_MODE,
    ENTITY_MODEL_OUTLINE_SHADER, ENTITY_MODEL_PORTAL_CULL_MODE, ENTITY_MODEL_PORTAL_DEPTH_COMPARE,
    ENTITY_MODEL_PORTAL_DEPTH_WRITE_ENABLED, ENTITY_MODEL_POSITION_COLOR_SHADER,
    ENTITY_MODEL_SCROLL_BLEND, ENTITY_MODEL_SCROLL_CULL_MODE, ENTITY_MODEL_SCROLL_DEPTH_COMPARE,
    ENTITY_MODEL_SCROLL_DEPTH_WRITE_ENABLED, ENTITY_MODEL_SCROLL_EMISSIVE_SHADER,
    ENTITY_MODEL_SCROLL_SHADER, ENTITY_MODEL_SURFACE_CULL_MODE,
    ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED, ENTITY_MODEL_SURFACE_NO_CULL_MODE,
    ENTITY_MODEL_SURFACE_OPAQUE_BLEND, ENTITY_MODEL_SURFACE_TRANSLUCENT_BLEND,
    ENTITY_MODEL_TEXTURED_DEPTH_COMPARE, ENTITY_MODEL_TEXTURED_SHADER,
    ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER, ENTITY_MODEL_WATER_MASK_BLEND,
    ENTITY_MODEL_WATER_MASK_COLOR_WRITE_MASK, ENTITY_MODEL_WATER_MASK_CULL_MODE,
    ENTITY_MODEL_WATER_MASK_DEPTH_COMPARE, ENTITY_MODEL_WATER_MASK_DEPTH_WRITE_ENABLED,
    ENTITY_MODEL_WATER_MASK_SHADER,
};
use crate::entity_models::geometry::{EntityModelTexturedMesh, EntityModelTexturedVertex};
use crate::{EntityModelInstance, EntityModelKind};
use bytemuck::Zeroable;

#[test]
fn scene_frame_renders_billboard_with_final_camera_and_entity_inputs() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(96, 96) else {
        return;
    };
    let texture = super::super::model_layers::DRAGON_FIREBALL_TEXTURE_REF;
    renderer
        .upload_entity_model_textures(&[crate::EntityModelTextureImage::new(
            texture,
            [255, 32, 64, 255].repeat((texture.size[0] * texture.size[1]) as usize),
        )])
        .unwrap();
    let camera = crate::RendererCameraFrame {
        pose: Some(crate::CameraPose {
            position: [0.0, 64.0, -4.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
        ..Default::default()
    };
    let instance =
        EntityModelInstance::new(1, EntityModelKind::DragonFireball, [0.0, 64.0, 2.0], 0.0);
    let displaced_camera = crate::RendererCameraFrame {
        pose: camera.pose.map(|pose| crate::CameraPose {
            y_rot: 90.0,
            ..pose
        }),
        ..camera
    };
    let mut displaced_instance = instance;
    displaced_instance.position[0] = 6.0;
    renderer.replace_scene_frame(displaced_camera, vec![displaced_instance]);
    let recycled = renderer.replace_scene_frame(camera, vec![instance]);
    assert!(recycled.is_empty());
    let submitted = renderer.render_offscreen_frame().unwrap();
    assert!(submitted.rgba.as_chunks::<4>().0.iter().any(|pixel| {
        pixel[0] > 100
            && u16::from(pixel[0]) > u16::from(pixel[1]) * 2
            && u16::from(pixel[0]) > u16::from(pixel[2]) * 2
    }));

    renderer.set_entity_model_instances(Vec::new());
    renderer.set_camera_frame(displaced_camera);
    renderer.set_entity_model_instances(vec![instance]);
    renderer.set_camera_frame(camera);
    let standalone = renderer.render_offscreen_frame().unwrap();
    assert_eq!(submitted.rgba, standalone.rgba);

    renderer.replace_scene_frame(camera, Vec::new());
    let empty = renderer.render_offscreen_frame().unwrap();
    assert_ne!(submitted.rgba, empty.rgba);

    use super::super::catalog::{BeaconBeamRenderState, EndPortalModelFaces, EndPortalModelKind};
    use super::super::model_layers::{
        BEACON_BEAM_TEXTURE_REF, BREEZE_WIND_TEXTURE_REF, END_PORTAL_TEXTURE_REF,
        END_SKY_TEXTURE_REF,
    };
    renderer
        .upload_entity_model_textures(
            &[
                BREEZE_WIND_TEXTURE_REF,
                END_SKY_TEXTURE_REF,
                END_PORTAL_TEXTURE_REF,
                BEACON_BEAM_TEXTURE_REF,
            ]
            .map(|texture| {
                crate::EntityModelTextureImage::new(
                    texture,
                    vec![255; (texture.size[0] * texture.size[1] * 4) as usize],
                )
            }),
        )
        .unwrap();
    let breeze = EntityModelInstance::breeze(2, [-1.0, 64.0, 2.0], 0.0).with(|state| {
        state.leash_ropes = bbb_render_types::EntityLeashRopesRenderState::from_ropes(&[
            bbb_render_types::LeashRopeRenderState {
                offset: [0.0, 1.0, 0.0],
                delta: [2.0, 0.5, 0.0],
                start_block_light: 15,
                start_sky_light: 15,
                end_block_light: 15,
                end_sky_light: 15,
                slack: true,
            },
        ]);
    });
    let portal = EntityModelInstance::new(
        3,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndPortal,
            faces: EndPortalModelFaces::Y_AXIS,
        },
        [0.0, 64.0, 3.0],
        0.0,
    );
    let beacon = EntityModelInstance::beacon_beam(4, [1.0, 64.0, 4.0]).with_block_entity(|state| {
        state.beacon_beam = Some(BeaconBeamRenderState {
            beam_start: 0,
            height: 4,
            color_argb: 0xFFFFFFFF,
            radius_scale: 1.0,
            animation_time: 0.0,
        })
    });
    renderer.replace_scene_frame(camera, vec![breeze, portal, beacon]);
    let mixed = renderer.render_offscreen_frame().unwrap();
    assert_eq!(renderer.pack.entity_meshes.opaque_beam_draws.len(), 1);
    assert_eq!(
        renderer
            .pack
            .entity_meshes
            .sorted_main_translucent_draws
            .len(),
        4
    );
    assert_eq!(renderer.counters.entity_scene_draw_calls, 5);
    assert_ne!(mixed.rgba, empty.rgba);
    eprintln!("mixed entity geometry frame: 5 indexed draws completed");

    let rotated_camera = crate::RendererCameraFrame {
        pose: camera.pose.map(|pose| crate::CameraPose {
            y_rot: 12.0,
            x_rot: 4.0,
            ..pose
        }),
        ..camera
    };
    renderer.replace_scene_frame(rotated_camera, vec![breeze, portal, beacon]);
    let rotated = renderer.render_offscreen_frame().unwrap();
    renderer.rebuild_entity_model_meshes();
    let rebuilt = renderer.render_offscreen_frame().unwrap();
    assert_eq!(rotated.rgba, rebuilt.rgba);
    assert_ne!(rotated.rgba, mixed.rgba);

    renderer.set_camera_pose(rotated_camera.pose.map(|pose| crate::CameraPose {
        eye_height: 0.5,
        ..pose
    }));
    let raised = renderer.render_offscreen_frame().unwrap();
    renderer.rebuild_entity_model_meshes();
    let rebuilt = renderer.render_offscreen_frame().unwrap();
    assert_eq!(raised.rgba, rebuilt.rgba);
    assert_ne!(raised.rgba, rotated.rgba);
}

#[test]
fn only_camera_billboard_entity_kinds_force_camera_mesh_rebuilds() {
    let armor_stand = EntityModelInstance::armor_stand(
        1,
        [0.0; 3],
        0.0,
        false,
        true,
        true,
        crate::DEFAULT_ARMOR_STAND_MODEL_POSE,
    );
    assert!(!entity_model_instances_have_camera_billboards(&[
        armor_stand
    ]));
    for kind in [
        EntityModelKind::ExperienceOrb { icon: 0 },
        EntityModelKind::DragonFireball,
        EntityModelKind::FishingHook,
    ] {
        assert!(entity_model_instances_have_camera_billboards(&[
            EntityModelInstance::new(2, kind, [0.0; 3], 0.0)
        ]));
    }
}

#[test]
fn dynamic_entity_lane_reuses_sufficient_gpu_buffer_capacity() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };
    let mesh = || EntityModelTexturedMesh {
        vertices: vec![EntityModelTexturedVertex::zeroed(); 4],
        indices: vec![0, 1, 2, 2, 3, 0],
        cutout_faces: 1,
        ..EntityModelTexturedMesh::new()
    };
    let mut resident = None;

    assert!(update_entity_model_textured_mesh_gpu_from_mesh(
        &renderer.device,
        &mut renderer.frame_uploads,
        &mut resident,
        mesh(),
        "bbb-test-dynamic-entity-lane",
    ));
    let first_capacities = resident
        .as_ref()
        .map(|resident| (resident.vertex_capacity, resident.index_capacity))
        .unwrap();
    assert!(!update_entity_model_textured_mesh_gpu_from_mesh(
        &renderer.device,
        &mut renderer.frame_uploads,
        &mut resident,
        mesh(),
        "bbb-test-dynamic-entity-lane",
    ));
    assert_eq!(
        resident
            .as_ref()
            .map(|resident| (resident.vertex_capacity, resident.index_capacity))
            .unwrap(),
        first_capacities
    );
}

#[test]
fn special_item_owned_entity_atlas_keeps_owned_identity_and_logical_size() {
    let static_texture = crate::EntityModelTextureRef {
        path: "textures/entity/static_special_test.png",
        size: [32, 16],
    };
    let owned_texture =
        crate::EntityModelOwnedTextureRef::new("textures/entity/pack/special_test.png", [64, 32]);
    let static_image =
        crate::EntityModelTextureImage::with_physical_size(static_texture, [1, 2], vec![17; 8]);
    let owned_image = crate::EntityModelOwnedTextureImage::with_physical_size(
        owned_texture.clone(),
        [2, 1],
        vec![29; 8],
    );
    let (layout, owned_entries, rgba) =
        super::build_entity_model_texture_atlas_with_owned_height_limit(
            &[static_image],
            &[owned_image],
            2,
        )
        .unwrap();
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(owned_entries.len(), 1);
    assert_eq!(owned_entries[0].texture, owned_texture);
    assert_eq!(owned_entries[0].texture.size, [64, 32]);
    assert_eq!([layout.width, layout.height], [4, 2]);
    assert_eq!(rgba.len(), 4 * 2 * 4);
}

#[test]
fn dynamic_entity_texture_generation_reuses_exact_owner_and_invalidates_edges() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };

    assert!(renderer.synchronize_dynamic_entity_texture_generation(7, 3, true));
    renderer
        .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
            handle: 91,
            standalone_sampling: None,
            rgba: vec![0xff; 64 * 64 * 4],
        })
        .unwrap();
    renderer
        .upload_dynamic_player_texture(bbb_render_types::DynamicPlayerTextureImage {
            handle: 92,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x55; 64 * 32 * 4],
        })
        .unwrap();
    let width = renderer.device.limits().max_texture_dimension_2d + 1;
    let mut rejected = vec![bbb_render_types::DynamicPlayerTextureImage {
        handle: 94,
        size: [width, 1],
        standalone_sampling: None,
        rgba: vec![0xff; width as usize * 4],
    }];
    assert!(renderer
        .install_dynamic_entity_textures(&mut rejected)
        .is_err());
    assert_eq!(rejected.len(), 1);
    assert_eq!(renderer.entity_dynamic_player_texture_images.len(), 1);
    renderer
        .upload_dynamic_player_texture(bbb_render_types::DynamicPlayerTextureImage {
            handle: 93,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x66; 64 * 32 * 4],
        })
        .unwrap();
    assert_eq!(renderer.entity_dynamic_player_skin_images.len(), 1);
    assert!(renderer.entity_dynamic_player_skin_atlas.is_some());
    assert!(renderer.pack.hud_dynamic_player_faces.contains_key(&91));
    assert_eq!(renderer.entity_dynamic_player_texture_images.len(), 2);
    assert!(renderer.entity_dynamic_player_texture_atlas.is_some());

    assert!(!renderer.synchronize_dynamic_entity_texture_generation(7, 3, true));
    assert_eq!(renderer.entity_dynamic_player_skin_images.len(), 1);

    assert!(renderer.synchronize_dynamic_entity_texture_generation(7, 3, false));
    assert!(renderer.entity_dynamic_player_skin_images.is_empty());
    assert!(renderer.entity_dynamic_player_skin_atlas.is_none());
    assert!(renderer.pack.hud_dynamic_player_faces.is_empty());
    assert!(renderer.entity_dynamic_player_texture_images.is_empty());
    assert!(renderer.entity_dynamic_player_texture_atlas.is_none());

    renderer
        .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
            handle: 91,
            standalone_sampling: None,
            rgba: vec![0x44; 64 * 64 * 4],
        })
        .unwrap();
    assert!(renderer.synchronize_dynamic_entity_texture_generation(8, 4, true));
    assert!(renderer.entity_dynamic_player_skin_images.is_empty());
    assert!(renderer.entity_dynamic_player_skin_atlas.is_none());
    assert!(renderer.pack.hud_dynamic_player_faces.is_empty());

    // Re-uploading the same handle replaces the cached HUD player face without leaking stale entries.
    for byte in [0x11u8, 0x22u8] {
        renderer
            .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
                handle: 91,
                standalone_sampling: None,
                rgba: vec![byte; 64 * 64 * 4],
            })
            .unwrap();
        assert_eq!(renderer.pack.hud_dynamic_player_faces.len(), 1);
        assert!(renderer.pack.hud_dynamic_player_faces.contains_key(&91));
    }
}

#[test]
fn slotted_dynamic_skin_and_texture_atlases_reuse_gpu_capacity_and_grow_2x() {
    let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
        eprintln!("skipping offscreen renderer test without a compatible adapter");
        return;
    };

    for handle in 100..116u64 {
        renderer
            .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
                handle,
                standalone_sampling: None,
                rgba: vec![handle as u8; 64 * 64 * 4],
            })
            .unwrap();
    }
    let first_skin_atlas = renderer.entity_dynamic_player_skin_atlas.as_ref().unwrap();
    let initial_skin_texture = first_skin_atlas.texture.clone();
    let initial_uv_100 = first_skin_atlas
        .layout
        .entries
        .iter()
        .find(|entry| entry.handle == 100)
        .unwrap()
        .uv;
    assert_eq!(
        first_skin_atlas.slot_capacity,
        DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY
    );
    assert_eq!(first_skin_atlas.next_unused_slot, 16);

    // Overwriting an existing skin handle updates in-place without reallocating or shifting UVs.
    renderer
        .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
            handle: 100,
            standalone_sampling: None,
            rgba: vec![0xee; 64 * 64 * 4],
        })
        .unwrap();
    let same_skin_atlas = renderer.entity_dynamic_player_skin_atlas.as_ref().unwrap();
    assert_eq!(same_skin_atlas.texture, initial_skin_texture);
    assert_eq!(
        same_skin_atlas.slot_capacity,
        DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY
    );
    assert_eq!(
        same_skin_atlas
            .layout
            .entries
            .iter()
            .find(|entry| entry.handle == 100)
            .unwrap()
            .uv,
        initial_uv_100
    );

    // Filling past the 16th slot doubles capacity to 32.
    renderer
        .upload_dynamic_player_skin(bbb_render_types::DynamicPlayerSkinImage {
            handle: 116,
            standalone_sampling: None,
            rgba: vec![0xab; 64 * 64 * 4],
        })
        .unwrap();
    let grown_skin_atlas = renderer.entity_dynamic_player_skin_atlas.as_ref().unwrap();
    assert_ne!(grown_skin_atlas.texture, initial_skin_texture);
    assert_eq!(
        grown_skin_atlas.slot_capacity,
        DYNAMIC_PLAYER_SKIN_INITIAL_SLOT_CAPACITY * 2
    );
    assert_eq!(grown_skin_atlas.layout.entries.len(), 17);

    // Incremental dynamic entity texture uploads reuse the pre-allocated 256x256 GPU texture.
    renderer
        .upload_dynamic_player_texture(bbb_render_types::DynamicPlayerTextureImage {
            handle: 500,
            size: [64, 32],
            standalone_sampling: None,
            rgba: vec![0x11; 64 * 32 * 4],
        })
        .unwrap();
    let initial_dyn_texture = renderer
        .entity_dynamic_player_texture_atlas
        .as_ref()
        .unwrap()
        .texture
        .clone();
    for handle in 501..510u64 {
        renderer
            .upload_dynamic_player_texture(bbb_render_types::DynamicPlayerTextureImage {
                handle,
                size: [64, 32],
                standalone_sampling: None,
                rgba: vec![handle as u8; 64 * 32 * 4],
            })
            .unwrap();
    }
    let reused_dyn_atlas = renderer
        .entity_dynamic_player_texture_atlas
        .as_ref()
        .unwrap();
    assert_eq!(reused_dyn_atlas.texture, initial_dyn_texture);
    assert_eq!(reused_dyn_atlas.layout.entries.len(), 10);
}

#[test]
fn entity_model_dissolve_shader_matches_vanilla_entity_fsh_branch_order() {
    // Vanilla `entity.fsh` (DISSOLVE + ALPHA_CUTOUT + PER_FACE_LIGHTING): the base ALPHA_CUTOUT
    // discard runs first, then the DISSOLVE mask compare discards `if (faceVertexColor.a <
    // texture(DissolveMaskSampler, texCoord0).a)`, and surviving fragments force the vertex-colour
    // alpha to 1.0 ("the dissolve effect entirely replaces translucency") before the tint multiply.
    let cutout_discard = ENTITY_MODEL_DISSOLVE_SHADER
        .find("if sample.a < 0.1 {")
        .expect("dissolve shader keeps the base ALPHA_CUTOUT 0.1 discard");
    let mask_sample = ENTITY_MODEL_DISSOLVE_SHADER
        .find("textureSample(entity_texture_atlas, entity_sampler, input.mask_uv)")
        .expect("dissolve shader samples the mask from the shared atlas at mask_uv");
    let dissolve_discard = ENTITY_MODEL_DISSOLVE_SHADER
        .find("if input.tint.a < mask.a {")
        .expect("dissolve shader discards where tint alpha is below the mask alpha");
    let force_opaque = ENTITY_MODEL_DISSOLVE_SHADER
        .find("let tint = vec4<f32>(input.tint.rgb, 1.0);")
        .expect("dissolve shader forces surviving fragments fully opaque");
    assert!(
        cutout_discard < mask_sample
            && mask_sample < dissolve_discard
            && dissolve_discard < force_opaque,
        "vanilla entity.fsh order: ALPHA_CUTOUT discard, then DISSOLVE mask compare, then force alpha 1.0"
    );
}

#[test]
fn entity_model_dissolve_pipeline_state_matches_vanilla_entity_cutout_dissolve() {
    // `RenderPipelines.ENTITY_CUTOUT_DISSOLVE` is `ENTITY_SNIPPET` + ALPHA_CUTOUT + PER_FACE_LIGHTING
    // + DISSOLVE + withCull(false), with no colour-target blend override: opaque surface state
    // identical to the plain entity cutout pipeline, cull off.
    assert_eq!(
        ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
        Some(wgpu::BlendState::REPLACE),
        "dissolve draws opaque like the entity cutout surface (no translucent blend)"
    );
    const {
        assert!(ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_SURFACE_NO_CULL_MODE, None,
        "vanilla ENTITY_CUTOUT_DISSOLVE.withCull(false)"
    );
    assert_eq!(
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
}

#[test]
fn entity_model_outline_shader_matches_vanilla_rendertype_outline_shape() {
    assert!(
        ENTITY_MODEL_OUTLINE_SHADER.contains("if (texel.a == 0.0)"),
        "vanilla rendertype_outline uses texture alpha only as a zero-alpha discard mask"
    );
    assert!(
        ENTITY_MODEL_OUTLINE_SHADER.contains("return vec4<f32>(input.tint.rgb, 1.0)"),
        "outline target output should be the submitted outline colour with default ColorModulator alpha"
    );
    assert!(
        !ENTITY_MODEL_OUTLINE_SHADER.contains("lightmap")
            && !ENTITY_MODEL_OUTLINE_SHADER.contains("overlay")
            && !ENTITY_MODEL_OUTLINE_SHADER.contains("apply_fog"),
        "vanilla rendertype_outline does not apply lightmap, overlay, or fog"
    );
}

#[test]
fn entity_model_outline_pipeline_uses_replace_blend() {
    assert_eq!(
        ENTITY_MODEL_OUTLINE_BLEND, None,
        "vanilla OUTLINE_SNIPPET declares no color-target blend state"
    );
}

#[test]
fn entity_model_translucent_emissive_shader_matches_vanilla_pipeline_state() {
    assert!(
        ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("if sample.a < 0.1")
            && ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("let texel = sample * input.tint")
            && ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("discard"),
        "vanilla entity_translucent_emissive cuts out sampled texture alpha before tint"
    );
    assert!(
        ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("input.overlay")
            && ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("per_face_diffuse_light"),
        "vanilla entity_translucent_emissive keeps overlay and PER_FACE_LIGHTING"
    );
    assert!(
        !ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("lightmap_texture")
            && !ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER.contains("sample_lightmap"),
        "vanilla ENTITY_EMISSIVE snippet omits LightTexture sampling"
    );
}

#[test]
fn entity_model_textured_shaders_cut_out_sampled_texture_alpha_before_tint() {
    for (label, shader, sampling) in [
        (
            "entity textured",
            ENTITY_MODEL_TEXTURED_SHADER,
            "let sample = sample_entity_texture(",
        ),
        (
            "breezeWind scroll",
            ENTITY_MODEL_SCROLL_SHADER,
            "let sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv)",
        ),
    ] {
        assert!(
            shader.contains(sampling),
            "{label} shader should sample texture before tint"
        );
        assert!(
            shader.contains("if sample.a < 0.1") && shader.contains("discard"),
            "{label} shader should use vanilla ALPHA_CUTOUT 0.1 comparison"
        );
        assert!(
            shader.contains("let texel = sample * input.tint"),
            "{label} shader should apply tint after alpha cutoff"
        );
        assert!(
            !shader.contains("if texel.a <= 0.1"),
            "{label} shader must not cut out post-tint alpha"
        );
    }
}

#[test]
fn embedded_entity_textured_pipelines_have_one_shader_source() {
    assert!(
        ENTITY_MODEL_TEXTURED_SHADER.contains("@builtin(front_facing) front_facing: bool")
            && ENTITY_MODEL_TEXTURED_SHADER
                .contains("per_face_diffuse_light(input.normal, front_facing)"),
        "the shared source must preserve Vanilla PER_FACE_LIGHTING for no-cull surfaces"
    );
    assert_eq!(ENTITY_MODEL_SURFACE_CULL_MODE, Some(wgpu::Face::Back));
    assert_eq!(ENTITY_MODEL_SURFACE_NO_CULL_MODE, None);
}

#[test]
fn entity_model_armor_shader_matches_vanilla_no_overlay_entity_pipeline() {
    assert!(
        ENTITY_MODEL_ARMOR_SHADER
            .contains("let sample = textureSample(entity_texture_atlas, entity_sampler, input.uv)")
            && ENTITY_MODEL_ARMOR_SHADER.contains("if sample.a < 0.1")
            && ENTITY_MODEL_ARMOR_SHADER.contains("let texel = sample * input.tint")
            && ENTITY_MODEL_ARMOR_SHADER.contains("discard"),
        "vanilla armor pipelines cut out sampled texture alpha before tint"
    );
    assert!(
        ENTITY_MODEL_ARMOR_SHADER.contains("sample_lightmap")
            && ENTITY_MODEL_ARMOR_SHADER.contains("per_face_diffuse_light"),
        "vanilla armor pipelines keep LightTexture and PER_FACE_LIGHTING"
    );
    assert!(
        !ENTITY_MODEL_ARMOR_SHADER.contains("overlay"),
        "vanilla armor pipelines define NO_OVERLAY"
    );
    assert!(
        ENTITY_MODEL_ARMOR_SHADER.contains("view_proj_view_offset_z: mat4x4<f32>")
            && ENTITY_MODEL_ARMOR_SHADER
                .contains("camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0)"),
        "vanilla armorCutoutNoCull / armorTranslucent apply VIEW_OFFSET_Z_LAYERING"
    );
}

#[test]
fn entity_model_solid_z_offset_forward_shader_matches_vanilla_entity_solid() {
    let shader = entity_model_solid_z_offset_forward_shader();
    assert!(
        shader.contains("view_proj_view_offset_z_forward: mat4x4<f32>")
            && shader.contains(
                "camera.view_proj_view_offset_z_forward * vec4<f32>(input.position, 1.0)"
            ),
        "vanilla entitySolidZOffsetForward applies VIEW_OFFSET_Z_LAYERING_FORWARD"
    );
    assert!(
        shader.contains("sample_lightmap(input.light)")
            && shader.contains("input.overlay")
            && shader.contains("per_face_diffuse_light(input.normal, front_facing)"),
        "entitySolid derives from the shared textured source; its cull descriptor admits only front-facing lighting"
    );
    assert!(
        !shader.contains("if sample.a < 0.1") && !shader.contains("discard"),
        "vanilla ENTITY_SOLID does not define ALPHA_CUTOUT"
    );
}

#[test]
fn entity_model_solid_shader_preserves_low_alpha_texels_without_cutout() {
    let shader = entity_model_solid_shader();
    assert!(
        shader.contains("let sample = sample_entity_texture(")
            && shader.contains("let texel = sample * input.tint"),
        "entitySolid still samples and tints the entity texture"
    );
    assert!(
        !shader.contains("if sample.a < 0.1") && !shader.contains("discard"),
        "vanilla ENTITY_SOLID has no ALPHA_CUTOUT and must preserve low-alpha texels"
    );
    assert!(
        shader.contains("sample_lightmap(input.light)")
            && shader.contains("input.overlay")
            && shader.contains("per_face_diffuse_light(input.normal, front_facing)"),
        "entitySolid keeps lightmap/overlay while the cull descriptor makes the shared front/back decision one-sided"
    );
}

#[test]
fn entity_model_cutout_z_offset_shader_matches_vanilla_cutout_shape() {
    let shader = entity_model_cutout_z_offset_shader();
    assert!(
        shader.contains("let sample = sample_entity_texture(")
            && shader.contains("if sample.a < 0.1")
            && shader.contains("let texel = sample * input.tint")
            && shader.contains("discard"),
        "vanilla ENTITY_CUTOUT_Z_OFFSET cuts out sampled texture alpha before tint"
    );
    assert!(
        shader.contains("sample_lightmap(input.light)")
            && shader.contains("input.overlay")
            && shader.contains("per_face_diffuse_light"),
        "vanilla ENTITY_CUTOUT_Z_OFFSET uses LightTexture, overlay, and PER_FACE_LIGHTING"
    );
    assert!(
        shader.contains("view_proj_view_offset_z: mat4x4<f32>")
            && shader.contains("camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0)"),
        "vanilla ENTITY_CUTOUT_Z_OFFSET applies VIEW_OFFSET_Z_LAYERING before projection"
    );
}

#[test]
fn entity_model_armor_pipeline_state_matches_vanilla_armor_pipelines() {
    assert!(
        ENTITY_MODEL_ARMOR_CUTOUT_BLEND.is_none(),
        "armorCutoutNoCull does not enable blending"
    );
    let blend = ENTITY_MODEL_ARMOR_TRANSLUCENT_BLEND
        .expect("armorTranslucent uses TRANSLUCENT blend state");
    assert_eq!(
        blend.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.SRC_ALPHA for colour"
    );
    assert_eq!(
        blend.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for colour"
    );
    assert_eq!(
        blend.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        blend.alpha.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for alpha"
    );
    const {
        assert!(ENTITY_MODEL_ARMOR_DEPTH_WRITE_ENABLED);
        assert!(!ENTITY_MODEL_ARMOR_DECAL_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_ARMOR_DECAL_DEPTH_COMPARE,
        wgpu::CompareFunction::Equal
    );
    assert_eq!(ENTITY_MODEL_ARMOR_CULL_MODE, None);
}

#[test]
fn entity_model_glint_pipeline_state_matches_vanilla_glint() {
    assert_eq!(
        ENTITY_MODEL_GLINT_BLEND.color.src_factor,
        wgpu::BlendFactor::Src
    );
    assert_eq!(
        ENTITY_MODEL_GLINT_BLEND.color.dst_factor,
        wgpu::BlendFactor::One
    );
    assert_eq!(
        ENTITY_MODEL_GLINT_BLEND.alpha.src_factor,
        wgpu::BlendFactor::Zero
    );
    assert_eq!(
        ENTITY_MODEL_GLINT_BLEND.alpha.dst_factor,
        wgpu::BlendFactor::One
    );
    const {
        assert!(!ENTITY_MODEL_GLINT_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_GLINT_DEPTH_COMPARE,
        wgpu::CompareFunction::Equal
    );
}

#[test]
fn entity_model_glint_shader_uses_vanilla_texture_transform_shape() {
    let entity_glint = entity_model_glint_shader("0.5", false);
    let armor_glint = entity_model_glint_shader("0.16", true);
    assert!(entity_glint.contains("const GLINT_UV_SCALE: f32 = 0.5"));
    assert!(armor_glint.contains("const GLINT_UV_SCALE: f32 = 0.16"));
    assert!(entity_glint.contains("const GLINT_ALPHA: f32 = 0.75"));
    assert!(entity_glint.contains("const GLINT_ANGLE: f32 = 0.1745329252"));
    assert!(entity_glint.contains("glint_offsets: vec4<f32>"));
    assert!(entity_glint.contains("view_proj_view_offset_z: mat4x4<f32>"));
    assert!(entity_glint.contains("rotated + camera.glint_offsets.xy"));
    assert!(entity_glint.contains("camera.view_proj * vec4<f32>(input.position, 1.0)"));
    assert!(armor_glint.contains("camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0)"));
    assert!(
        entity_glint.contains("textureSample(glint_texture, glint_sampler, fract(input.local_uv))")
    );
    assert!(!entity_glint.contains("atlas_uv"));
    assert!(entity_glint.contains("if color.a < 0.1"));
    assert!(!entity_glint.contains("lightmap_texture"));
    assert!(!entity_glint.contains("sample_lightmap"));
}

#[test]
fn entity_model_energy_swirl_pipeline_state_matches_vanilla_additive() {
    assert_eq!(
        ENTITY_MODEL_ADDITIVE_BLEND.color.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.ADDITIVE uses SourceFactor.ONE for colour"
    );
    assert_eq!(
        ENTITY_MODEL_ADDITIVE_BLEND.color.dst_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.ADDITIVE uses DestFactor.ONE for colour"
    );
    assert_eq!(
        ENTITY_MODEL_ADDITIVE_BLEND.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.ADDITIVE uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        ENTITY_MODEL_ADDITIVE_BLEND.alpha.dst_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.ADDITIVE uses DestFactor.ONE for alpha"
    );
}

#[test]
fn entity_model_energy_swirl_shader_matches_vanilla_emissive_scroll_shape() {
    assert!(
        ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("fract(input.local_uv)"),
        "vanilla energySwirl applies the texture matrix offset and wraps atlas-local UVs"
    );
    assert!(
        ENTITY_MODEL_SCROLL_EMISSIVE_SHADER
            .contains("let sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv)")
            && ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("if sample.a < 0.1")
            && ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("let texel = sample * input.tint"),
        "vanilla ENERGY_SWIRL cuts out sampled texture alpha before tint"
    );
    assert!(
        !ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("sample_lightmap")
            && !ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("lightmap_texture")
            && !ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("input.overlay"),
        "vanilla ENERGY_SWIRL defines EMISSIVE and NO_OVERLAY"
    );
}

#[test]
fn entity_model_beam_pipeline_states_match_vanilla() {
    assert_eq!(ENTITY_MODEL_BEAM_OPAQUE_BLEND, None);
    const {
        assert!(ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED);
    }
    let translucent =
        ENTITY_MODEL_BEAM_TRANSLUCENT_BLEND.expect("beacon glow uses translucent blending");
    assert_eq!(translucent.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(
        translucent.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha
    );
    const {
        assert!(!ENTITY_MODEL_BEAM_TRANSLUCENT_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_BEAM_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(ENTITY_MODEL_SCROLL_CULL_MODE, None);
    assert_eq!(ENTITY_MODEL_BEACON_BEAM_CULL_MODE, Some(wgpu::Face::Back));
}

#[test]
fn guardian_and_end_crystal_beam_shaders_keep_distinct_entity_state() {
    let guardian = entity_model_tiled_entity_shader(true, true);
    assert!(guardian.contains("fract(input.local_uv)"));
    assert!(guardian.contains("if sample.a < 0.1"));
    assert!(guardian.contains("input.overlay.y < 8.0"));
    assert!(guardian.contains("let diffuse = per_face_diffuse_light(input.normal, front_facing)"));
    assert!(guardian.contains("sample_lightmap(input.light)"));

    let crystal = entity_model_tiled_entity_shader(false, false);
    assert!(crystal.contains("fract(input.local_uv)"));
    assert!(crystal.contains("if sample.a < 0.1"));
    assert!(!crystal.contains("input.overlay.y < 8.0"));
    assert!(crystal.contains("let diffuse = diffuse_light(input.normal)"));
    assert!(crystal.contains("sample_lightmap(input.light)"));
}

#[test]
fn beacon_beam_shader_omits_entity_only_stages() {
    assert!(ENTITY_MODEL_BEACON_BEAM_SHADER.contains("fract(input.local_uv)"));
    assert!(!ENTITY_MODEL_BEACON_BEAM_SHADER.contains("if sample.a < 0.1"));
    assert!(!ENTITY_MODEL_BEACON_BEAM_SHADER.contains("lightmap_texture"));
    assert!(!ENTITY_MODEL_BEACON_BEAM_SHADER.contains("input.overlay"));
    assert!(!ENTITY_MODEL_BEACON_BEAM_SHADER.contains("diffuse_light"));
}

#[test]
fn entity_model_eyes_pipeline_state_matches_vanilla_eyes() {
    assert_eq!(
        ENTITY_MODEL_EYES_BLEND.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.SRC_ALPHA for colour"
    );
    assert_eq!(
        ENTITY_MODEL_EYES_BLEND.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for colour"
    );
    assert_eq!(
        ENTITY_MODEL_EYES_BLEND.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        ENTITY_MODEL_EYES_BLEND.alpha.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for alpha"
    );
    const {
        assert!(!ENTITY_MODEL_EYES_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(ENTITY_MODEL_EYES_CULL_MODE, None);
}

#[test]
fn entity_model_dragon_rays_pipeline_state_matches_vanilla_rays() {
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_BLEND.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.LIGHTNING uses SRC_ALPHA for colour"
    );
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_BLEND.color.dst_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.LIGHTNING adds to destination colour"
    );
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_BLEND.alpha.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.LIGHTNING uses SRC_ALPHA for alpha"
    );
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_BLEND.alpha.dst_factor,
        wgpu::BlendFactor::One
    );
    const {
        assert!(!ENTITY_MODEL_DRAGON_RAYS_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(ENTITY_MODEL_DRAGON_RAYS_CULL_MODE, None);
    assert!(ENTITY_MODEL_POSITION_COLOR_SHADER.contains("@location(1) color: vec4<f32>"));
    assert!(!ENTITY_MODEL_POSITION_COLOR_SHADER.contains("textureSample"));
    assert!(!ENTITY_MODEL_POSITION_COLOR_SHADER.contains("lightmap_texture"));
}

#[test]
fn entity_model_dragon_rays_depth_pipeline_state_matches_vanilla_depth_replay() {
    const {
        assert!(ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_COLOR_WRITE_MASK,
        wgpu::ColorWrites::empty(),
        "vanilla DRAGON_RAYS_DEPTH uses ColorTargetState(Optional.empty(), 0)"
    );
}

#[test]
fn entity_model_portal_shader_ports_vanilla_end_portal_layers() {
    let portal = entity_model_portal_shader(ENTITY_MODEL_END_PORTAL_LAYERS);
    let gateway = entity_model_portal_shader(ENTITY_MODEL_END_GATEWAY_LAYERS);
    assert!(portal.contains("const PORTAL_LAYERS: u32 = 15u;"));
    assert!(gateway.contains("const PORTAL_LAYERS: u32 = 16u;"));
    assert!(portal.contains("end_portal_layer_uv(input.tex_proj, 15.0)"));
    assert!(!portal.contains("end_portal_layer_uv(input.tex_proj, 16.0)"));
    assert!(gateway.contains("end_portal_layer_uv(input.tex_proj, 16.0)"));
    assert!(portal.contains("projection_from_position"));
    assert!(portal.contains("17.0 / layer"));
    assert!(portal.contains("(tex_proj.x * c - tex_proj.y * s) * scale"));
    assert!(portal.contains("(tex_proj.x * s + tex_proj.y * c) * scale"));
    assert!(portal.contains("(2.0 + layer / 1.5) * (camera.shader_game_time.x * 1.5)"));
    assert!(portal.contains("COLORS: array<vec3<f32>, 16>"));
    assert!(portal.contains("atlas_repeat_sample("));
    assert!(portal.contains("input.sky_rect_min"));
    assert!(portal.contains("input.portal_rect_min"));
}

#[test]
fn hud_win_screen_portal_reuses_fifteen_layers_without_world_fog() {
    let shader = win_screen_end_portal_shader();
    assert!(shader.contains("end_portal_layer_uv(input.tex_proj, 15.0)"));
    assert!(!shader.contains("end_portal_layer_uv(input.tex_proj, 16.0)"));
    assert!(shader
        .contains("out.tex_proj = vec4<f32>(input.position.xy * 0.5 + vec2<f32>(0.5), 0.0, 1.0)"));
    assert!(shader.contains("return vec4<f32>(color, 1.0);"));
    assert!(!shader.contains("return apply_fog(vec4<f32>(color, 1.0), input.spherical_distance"));
}

#[test]
fn entity_model_portal_pipeline_state_matches_vanilla_end_portal() {
    const {
        assert!(ENTITY_MODEL_PORTAL_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_PORTAL_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual,
        "vanilla DepthStencilState.DEFAULT uses LESS_EQUAL"
    );
    assert_eq!(
        ENTITY_MODEL_PORTAL_CULL_MODE,
        Some(wgpu::Face::Back),
        "RenderPipeline default cull is enabled for END_PORTAL"
    );
}

#[test]
fn entity_model_water_mask_pipeline_state_matches_vanilla_water_mask() {
    let blend = ENTITY_MODEL_WATER_MASK_BLEND.expect("waterMask keeps TRANSLUCENT blend state");
    assert_eq!(
        blend.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.SRC_ALPHA for colour"
    );
    assert_eq!(
        blend.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for colour"
    );
    assert_eq!(
        blend.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        blend.alpha.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for alpha"
    );
    assert!(ENTITY_MODEL_WATER_MASK_COLOR_WRITE_MASK.is_empty());
    const {
        assert!(ENTITY_MODEL_WATER_MASK_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_WATER_MASK_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(
        ENTITY_MODEL_WATER_MASK_CULL_MODE,
        Some(wgpu::Face::Back),
        "RenderPipeline.Builder defaults cull=true for WATER_MASK"
    );
}

#[test]
fn entity_model_water_mask_shader_matches_vanilla_depth_only_shape() {
    assert!(
        ENTITY_MODEL_WATER_MASK_SHADER.contains("@location(0) position: vec3<f32>"),
        "vanilla rendertype_water_mask.vsh consumes only Position"
    );
    assert!(
        !ENTITY_MODEL_WATER_MASK_SHADER.contains("textureSample")
            && !ENTITY_MODEL_WATER_MASK_SHADER.contains("lightmap_texture")
            && !ENTITY_MODEL_WATER_MASK_SHADER.contains("overlay"),
        "vanilla rendertype_water_mask has no texture, lightmap, or overlay sampling"
    );
    assert!(
        ENTITY_MODEL_WATER_MASK_SHADER.contains("return vec4<f32>(1.0, 1.0, 1.0, 1.0)"),
        "fragment color is ignored because WATER_MASK has color write mask 0"
    );
}

#[test]
fn entity_model_breeze_wind_pipeline_state_matches_vanilla_breeze_wind() {
    assert_eq!(
        ENTITY_MODEL_SCROLL_BLEND.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.SRC_ALPHA for colour"
    );
    assert_eq!(
        ENTITY_MODEL_SCROLL_BLEND.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for colour"
    );
    assert_eq!(
        ENTITY_MODEL_SCROLL_BLEND.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        ENTITY_MODEL_SCROLL_BLEND.alpha.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for alpha"
    );
    const {
        assert!(ENTITY_MODEL_SCROLL_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_SCROLL_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(ENTITY_MODEL_SCROLL_CULL_MODE, None);
}

#[test]
fn entity_model_outline_pipelines_represent_vanilla_cull_modes() {
    assert_eq!(ENTITY_MODEL_OUTLINE_NO_CULL_MODE, None);
    assert_eq!(ENTITY_MODEL_OUTLINE_CULL_MODE, Some(wgpu::Face::Back));
}

#[test]
fn entity_model_surface_pipelines_represent_vanilla_core_state() {
    assert_eq!(
        ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
        Some(wgpu::BlendState::REPLACE),
        "vanilla entitySolid/entityCutout/entityCutoutCull/entityCutoutZOffset use replacement color writes"
    );
    let translucent = ENTITY_MODEL_SURFACE_TRANSLUCENT_BLEND
        .expect("entity translucent surfaces use BlendFunction.TRANSLUCENT");
    assert_eq!(
        translucent.color.src_factor,
        wgpu::BlendFactor::SrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.SRC_ALPHA for colour"
    );
    assert_eq!(
        translucent.color.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for colour"
    );
    assert_eq!(
        translucent.alpha.src_factor,
        wgpu::BlendFactor::One,
        "vanilla BlendFunction.TRANSLUCENT uses SourceFactor.ONE for alpha"
    );
    assert_eq!(
        translucent.alpha.dst_factor,
        wgpu::BlendFactor::OneMinusSrcAlpha,
        "vanilla BlendFunction.TRANSLUCENT uses DestFactor.ONE_MINUS_SRC_ALPHA for alpha"
    );
    const {
        assert!(ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(ENTITY_MODEL_SURFACE_NO_CULL_MODE, None);
    assert_eq!(ENTITY_MODEL_SURFACE_CULL_MODE, Some(wgpu::Face::Back));
}

#[test]
fn beacon_beam_wgsl_computes_fog_distance_per_fragment() {
    assert!(ENTITY_MODEL_BEACON_BEAM_SHADER.contains("@location(4) fog_pos: vec3<f32>,"));
    assert!(ENTITY_MODEL_BEACON_BEAM_SHADER
        .contains("out.fog_pos = input.position - camera.camera_position.xyz;"));
    assert!(
        ENTITY_MODEL_BEACON_BEAM_SHADER.contains("let spherical_distance = length(input.fog_pos);")
    );
}

#[test]
fn entity_and_dynamic_atlases_reject_over_budget_dimensions_before_alloc() {
    // R11-01 / R12-02: tall + wide slivers would create a multi-GiB atlas; must fail before alloc.
    let tall = crate::EntityModelOwnedTextureImage::with_physical_size(
        crate::EntityModelOwnedTextureRef::new("textures/entity/pack/tall.png", [1, 8_192]),
        [1, 8_192],
        vec![0xff; 8_192 * 4],
    );
    let wide = crate::EntityModelOwnedTextureImage::with_physical_size(
        crate::EntityModelOwnedTextureRef::new("textures/entity/pack/wide.png", [8_192, 1]),
        [8_192, 1],
        vec![0xff; 8_192 * 4],
    );
    let err = super::textures::build_entity_model_texture_atlas_with_owned_limits(
        &[],
        &[tall, wide],
        [16_384, 8_192],
        super::textures::MAX_ENTITY_MODEL_ATLAS_BYTES,
    )
    .unwrap_err();
    assert!(err.to_string().contains("budget") || err.to_string().contains("limit"));

    let dyn_tall = bbb_render_types::DynamicPlayerTextureImage {
        handle: 1,
        size: [1, 8_192],
        standalone_sampling: None,
        rgba: vec![0xff; 8_192 * 4],
    };
    let dyn_wide = bbb_render_types::DynamicPlayerTextureImage {
        handle: 2,
        size: [8_192, 1],
        standalone_sampling: None,
        rgba: vec![0xff; 8_192 * 4],
    };
    let dyn_err = super::textures::build_dynamic_player_texture_atlas_with_limits(
        &[dyn_tall, dyn_wide],
        [16_384, 8_192],
    )
    .unwrap_err();
    assert!(dyn_err.to_string().contains("budget"));

    // R21-02: exceeding `MAX_ENTITY_MODEL_OWNED_TEXTURES` fails before scanning or allocating RGBA.
    let too_many: Vec<crate::EntityModelOwnedTextureImage> = (0
        ..=super::textures::MAX_ENTITY_MODEL_OWNED_TEXTURES)
        .map(|i| {
            crate::EntityModelOwnedTextureImage::with_physical_size(
                crate::EntityModelOwnedTextureRef::new(
                    format!("textures/entity/pack/special_{i}.png"),
                    [1, 1],
                ),
                [1, 1],
                vec![0xff; 4],
            )
        })
        .collect();
    let count_err = super::textures::build_entity_model_texture_atlas_with_owned_limits(
        &[],
        &too_many,
        [16_384, 16_384],
        super::textures::MAX_ENTITY_MODEL_ATLAS_BYTES,
    )
    .unwrap_err();
    assert!(count_err.to_string().contains("exceeds limit"));
}

#[test]
fn humanoid_rot_lerp_rad_handles_extreme_and_non_finite_angles_in_o1() {
    use crate::entity_models::humanoid_rot_lerp_rad;
    assert_eq!(humanoid_rot_lerp_rad(0.5, 0.0, f32::INFINITY), 0.0);
    assert_eq!(humanoid_rot_lerp_rad(0.5, 0.0, f32::NAN), 0.0);
    let huge = humanoid_rot_lerp_rad(0.5, 0.0, 1e20);
    assert!(huge.is_finite());
    let normal = humanoid_rot_lerp_rad(0.5, 0.0, std::f32::consts::FRAC_PI_2);
    assert!((normal - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
}
