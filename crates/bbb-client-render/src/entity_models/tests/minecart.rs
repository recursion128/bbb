use super::*;

#[test]
fn minecart_layer_passes_match_vanilla_renderer() {
    let passes = minecart_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, MINECART_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn minecart_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&MINECART_TEXTURE_REF));
}

#[test]
fn minecart_jitter_matches_vanilla_offset_seed_formula() {
    assert_close3(
        minecart_render_jitter_offset(1),
        [0.00175, 0.00075, 0.00175],
    );
    assert_close3(
        minecart_render_jitter_offset(2),
        [0.00175, -0.00125, 0.00025],
    );
    assert_close3(
        minecart_render_jitter_offset(41),
        [-0.00175, 0.00175, 0.00025],
    );

    for offset in [-1, 0, 1, 2, 41, i32::MAX]
        .into_iter()
        .map(minecart_render_jitter_offset)
    {
        assert!(offset.iter().all(|component| component.abs() <= 0.00175));
    }
}

#[test]
fn minecart_root_transform_matches_vanilla_old_render_without_rail() {
    let instance = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0));
    let expected = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - instance.render_state.body_rot).to_radians())
        * Mat4::from_rotation_z((-instance.render_state.head_pitch).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));

    assert_close_transform(minecart_model_root_transform(&instance), expected);
    assert_ne!(
        minecart_model_root_transform(&instance),
        entity_model_root_transform(&instance)
    );
}

#[test]
fn minecart_old_render_root_transform_follows_rail_points() {
    let instance = EntityModelInstance::minecart(41, [2.5, 1.0, 3.5], 20.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        .with(|s| {
            s.kind.minecart_mut().pos_on_rail = Some([2.5, 1.5625, 3.5]);
            s.kind.minecart_mut().front_pos = Some([2.8, 1.8625, 3.5]);
            s.kind.minecart_mut().back_pos = Some([2.2, 1.2625, 3.5]);
        });
    let front_pos = Vec3::from_array(instance.render_state.kind.minecart().front_pos.unwrap());
    let back_pos = Vec3::from_array(instance.render_state.kind.minecart().back_pos.unwrap());
    let direction = (back_pos - front_pos).normalize();
    let rotation = direction.z.atan2(direction.x).to_degrees();
    let x_rot = direction.y.atan() * 73.0;
    let expected = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.5625, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - rotation).to_radians())
        * Mat4::from_rotation_z((-x_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));

    assert_close_transform(minecart_model_root_transform(&instance), expected);
    assert_ne!(
        minecart_model_root_transform(&instance),
        minecart_model_root_transform(
            &EntityModelInstance::minecart(41, [2.5, 1.0, 3.5], 20.0)
                .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        )
    );
}

#[test]
fn minecart_new_render_root_transform_uses_vanilla_order() {
    let old_render = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0));
    let new_render = old_render.with(|s| s.kind.minecart_mut().new_render = true);
    let expected = Mat4::from_translation(Vec3::from_array(new_render.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_rotation_y(new_render.render_state.body_rot.to_radians())
        * Mat4::from_rotation_z((-new_render.render_state.head_pitch).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));

    assert_close_transform(minecart_model_root_transform(&new_render), expected);
    assert_ne!(
        minecart_model_root_transform(&new_render),
        minecart_model_root_transform(&old_render)
    );
}

#[test]
fn minecart_hurt_roll_matches_vanilla_vehicle_damage_formula() {
    let damaged = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        .with(|s| {
            s.kind.minecart_mut().hurt.time = 7.5;
            s.kind.minecart_mut().hurt.dir = -1;
            s.kind.minecart_mut().hurt.damage_time = 17.5;
        });
    let roll = -(7.5_f32.sin() * 7.5 * 17.5 / 10.0);
    assert!((minecart_damage_roll_degrees(&damaged) - roll).abs() < 1.0e-6);

    let expected = Mat4::from_translation(Vec3::from_array(damaged.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - damaged.render_state.body_rot).to_radians())
        * Mat4::from_rotation_z((-damaged.render_state.head_pitch).to_radians())
        * Mat4::from_rotation_x(roll.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    assert_close_transform(minecart_model_root_transform(&damaged), expected);

    let settled = damaged.with(|s| s.kind.minecart_mut().hurt.time = 0.0);
    assert_eq!(minecart_damage_roll_degrees(&settled), 0.0);
}

#[test]
fn minecart_display_block_transform_uses_vanilla_content_pose_before_body_flip() {
    let display_offset = 10;
    let minecart = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        .with(|s| {
            s.kind.minecart_mut().hurt.time = 7.5;
            s.kind.minecart_mut().hurt.dir = -1;
            s.kind.minecart_mut().hurt.damage_time = 17.5;
        });
    let roll = minecart_damage_roll_degrees(&minecart);
    let expected = Mat4::from_translation(Vec3::from_array(minecart.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - minecart.render_state.body_rot).to_radians())
        * Mat4::from_rotation_z((-minecart.render_state.head_pitch).to_radians())
        * Mat4::from_rotation_x(roll.to_radians())
        * Mat4::from_scale(Vec3::splat(0.75))
        * Mat4::from_translation(Vec3::new(-0.5, (display_offset as f32 - 8.0) / 16.0, 0.5))
        * Mat4::from_rotation_y(90.0_f32.to_radians());

    let transform = minecart_display_block_transform(&minecart, display_offset).unwrap();
    assert_close_transform(transform, expected);
    assert_ne!(transform, minecart_model_root_transform(&minecart));

    let creeper = EntityModelInstance::new(100, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    assert!(minecart_display_block_transform(&creeper, display_offset).is_none());
    assert!(
        minecart_display_block_transform(&minecart.with_invisible(true), display_offset).is_none()
    );
    assert!(minecart_display_block_transform(
        &minecart.with_invisible(true).with_appears_glowing(true),
        display_offset
    )
    .is_some());
}

#[test]
fn tnt_minecart_display_block_transform_applies_vanilla_fuse_scale() {
    let display_offset = 6;
    let fuse = 4.5_f32;
    let minecart = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = fuse);
    let g = (1.0 - fuse / 10.0).clamp(0.0, 1.0);
    let scale = 1.0 + g * g * g * g * 0.3;
    let expected = minecart_display_block_transform(&minecart, display_offset).unwrap()
        * Mat4::from_scale(Vec3::splat(scale));

    assert_close_transform(
        minecart_tnt_display_block_transform(&minecart, display_offset).unwrap(),
        expected,
    );
    assert_close_transform(
        minecart_tnt_display_block_transform(
            &minecart.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = 10.0),
            display_offset,
        )
        .unwrap(),
        minecart_display_block_transform(
            &minecart.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = 10.0),
            display_offset,
        )
        .unwrap(),
    );
    assert_close_transform(
        minecart_tnt_display_block_transform(
            &minecart.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = -1.0),
            display_offset,
        )
        .unwrap(),
        minecart_display_block_transform(
            &minecart.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = -1.0),
            display_offset,
        )
        .unwrap(),
    );
}

#[test]
fn primed_tnt_block_transform_uses_vanilla_submit_pose_and_fuse_scale() {
    let fuse = 4.5_f32;
    let tnt = EntityModelInstance::no_render(133, [2.0, 64.0, -3.0], 0.0);
    let g = (1.0 - fuse / 10.0).clamp(0.0, 1.0);
    let scale = 1.0 + g * g * g * g * 0.3;
    let expected = Mat4::from_translation(Vec3::from_array(tnt.position))
        * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_rotation_y(-90.0_f32.to_radians())
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.5))
        * Mat4::from_rotation_y(90.0_f32.to_radians());

    assert_close_transform(primed_tnt_block_transform(&tnt, fuse).unwrap(), expected);
    assert_close_transform(
        primed_tnt_block_transform(&tnt, 10.0).unwrap(),
        Mat4::from_translation(Vec3::from_array(tnt.position))
            * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0))
            * Mat4::from_rotation_y(-90.0_f32.to_radians())
            * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.5))
            * Mat4::from_rotation_y(90.0_f32.to_radians()),
    );

    let creeper = EntityModelInstance::new(100, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    assert!(primed_tnt_block_transform(&creeper, fuse).is_none());
    assert!(primed_tnt_block_transform(&tnt.with_invisible(true), fuse).is_none());
    assert!(
        primed_tnt_block_transform(&tnt.with_invisible(true).with_appears_glowing(true), fuse)
            .is_some()
    );
}

#[test]
fn minecart_textured_mesh_uses_vanilla_render_type_and_overlay() {
    let (atlas, _) = build_entity_model_texture_atlas(&minecart_texture_images()).unwrap();
    let instance = EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let textured = &meshes.cutout;
    assert_eq!(textured.cutout_faces, 30);
    assert_eq!(textured.vertices.len(), 120);
    assert_eq!(textured.indices.len(), 180);
    assert!(textured
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(textured.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

fn minecart_texture_images() -> Vec<EntityModelTextureImage> {
    [MINECART_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for (actual, expected) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}
