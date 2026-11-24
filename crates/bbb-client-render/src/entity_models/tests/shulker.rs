use super::*;

#[test]
fn shulker_lid_pose_matches_vanilla_setup_anim() {
    use std::f32::consts::PI;

    // Closed (peek 0): `bs = 0.5π`, `sin(bs) = 1` → `lid.y = 16 + 8 = 24` (the bind offset), and
    // `peek ≤ 0.3` keeps `lid.yRot = 0`. The closed pose equals the bind pose.
    let (lid_y, lid_yrot) = shulker_lid_pose(0.0, 17.0);
    assert!((lid_y - 24.0).abs() < 1.0e-5);
    assert_eq!(lid_yrot, 0.0);

    // Fully open (peek 1): `bs = 1.5π`, `sin(bs) = -1` → `lid.y = 16 - 8 = 8` (the bob `extra` is
    // `sin(age·0.1)·0.7`, which is `0` at age 0). `q = -1 + (-1) = -2`, so
    // `lid.yRot = (-2)⁴ · π · 0.125 = 16 · π · 0.125 = 2π`.
    let (lid_y, lid_yrot) = shulker_lid_pose(1.0, 0.0);
    assert!((lid_y - 8.0).abs() < 1.0e-5);
    assert!((lid_yrot - 2.0 * PI).abs() < 1.0e-5);

    // The `lid.yRot` twist switches on strictly above `peek = 0.3` (vanilla `peekAmount > 0.3F`).
    assert_eq!(shulker_lid_pose(0.3, 0.0).1, 0.0);
    assert_ne!(shulker_lid_pose(0.4, 0.0).1, 0.0);

    // The open-lid bob `sin(age·0.1)·0.7` only applies past half-open (`bs > π`, i.e. `peek > 0.5`):
    // at `peek = 0.6` the lid Y moves with age; at `peek = 0.4` it does not.
    assert_ne!(shulker_lid_pose(0.6, 0.0).0, shulker_lid_pose(0.6, 15.0).0);
    assert_eq!(shulker_lid_pose(0.4, 0.0).0, shulker_lid_pose(0.4, 15.0).0);
}

#[test]
fn shulker_root_transform_matches_vanilla_attach_face_setup_rotations() {
    // Vanilla `ShulkerRenderer.setupRotations` calls `super.setupRotations(bodyRot + 180)`, so the
    // normal non-sleeping yaw becomes `-bodyRot`, then rotates around `(0, 0.5, 0)` by
    // `attachFace.getOpposite().getRotation()`.
    let position = [1.0, 64.0, -2.0];
    let body_rot = 30.0;
    let base = EntityModelInstance::shulker(1131, position, body_rot, None);

    let floor = shulker_model_root_transform(
        &base.with(|s| s.kind.shulker_mut().attach_face = EntityAttachmentFace::Down),
    );
    let expected_floor = Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_rotation_y((-body_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    assert_close3(
        floor.transform_point3(Vec3::ZERO).to_array(),
        expected_floor.transform_point3(Vec3::ZERO).to_array(),
    );
    assert_close3(
        floor.transform_vector3(Vec3::X).to_array(),
        expected_floor.transform_vector3(Vec3::X).to_array(),
    );
    assert_close3(
        floor.transform_vector3(Vec3::Y).to_array(),
        expected_floor.transform_vector3(Vec3::Y).to_array(),
    );
    assert_close3(
        floor.transform_vector3(Vec3::Z).to_array(),
        expected_floor.transform_vector3(Vec3::Z).to_array(),
    );

    // Attach NORTH uses opposite SOUTH, and `Direction.SOUTH.getRotation()` is `rotationX(π/2)`.
    let north = shulker_model_root_transform(
        &base.with(|s| s.kind.shulker_mut().attach_face = EntityAttachmentFace::North),
    );
    let pivot = Vec3::new(0.0, 0.5, 0.0);
    let expected_north = Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_rotation_y((-body_rot).to_radians())
        * Mat4::from_translation(pivot)
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
        * Mat4::from_translation(-pivot)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    assert_close3(
        north.transform_point3(Vec3::ZERO).to_array(),
        expected_north.transform_point3(Vec3::ZERO).to_array(),
    );
    assert_close3(
        north.transform_vector3(Vec3::X).to_array(),
        expected_north.transform_vector3(Vec3::X).to_array(),
    );
    assert_close3(
        north.transform_vector3(Vec3::Y).to_array(),
        expected_north.transform_vector3(Vec3::Y).to_array(),
    );
    assert_close3(
        north.transform_vector3(Vec3::Z).to_array(),
        expected_north.transform_vector3(Vec3::Z).to_array(),
    );
    assert_ne!(floor, north);
}

#[test]
fn shulker_textured_render_matches_vanilla_renderer() {
    // `ShulkerRenderer.getTextureLocation`: an uncolored shulker (`None`) uses the default texture,
    // and each of the sixteen `DyeColor`s its own `shulker_<color>.png`.
    let colors = [
        EntityDyeColor::White,
        EntityDyeColor::Orange,
        EntityDyeColor::Magenta,
        EntityDyeColor::LightBlue,
        EntityDyeColor::Yellow,
        EntityDyeColor::Lime,
        EntityDyeColor::Pink,
        EntityDyeColor::Gray,
        EntityDyeColor::LightGray,
        EntityDyeColor::Cyan,
        EntityDyeColor::Purple,
        EntityDyeColor::Blue,
        EntityDyeColor::Brown,
        EntityDyeColor::Green,
        EntityDyeColor::Red,
        EntityDyeColor::Black,
    ];
    for color in std::iter::once(None).chain(colors.map(Some)) {
        let texture = shulker_texture_ref(color);
        let passes = shulker_textured_layer_passes(color);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutoutZOffset
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutoutZOffset");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
        assert!(entity_model_texture_refs().contains(&texture));
    }
    assert_eq!(
        shulker_texture_ref(None),
        EntityModelTextureRef {
            path: "textures/entity/shulker/shulker.png",
            size: [64, 64],
        }
    );
    assert_eq!(
        shulker_texture_ref(Some(EntityDyeColor::Red)),
        EntityModelTextureRef {
            path: "textures/entity/shulker/shulker_red.png",
            size: [64, 64],
        }
    );

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/shulker/shulker"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let default = EntityModelInstance::shulker(900, [0.0, 64.0, 0.0], 0.0, None).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[default], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let mesh = &meshes.cutout_z_offset;

    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == default.render_state.shader_light()
            && vertex.overlay == default.render_state.overlay_coords()));

    let north =
        EntityModelInstance::shulker(901, [0.0, 64.0, 0.0], 15.0, Some(EntityDyeColor::Red)).with(
            |s| {
                s.kind.shulker_mut().attach_face = EntityAttachmentFace::North;
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            },
        );
    let meshes = entity_model_textured_meshes(&[north], &atlas);

    assert!(meshes
        .cutout_z_offset
        .vertices
        .iter()
        .all(|vertex| vertex.light == north.render_state.shader_light()
            && vertex.overlay == north.render_state.overlay_coords()));
}

#[test]
fn shulker_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::shulker(901, [0.0, 64.0, 0.0], 15.0, Some(EntityDyeColor::Red)).with(
            |s| {
                s.kind.shulker_mut().attach_face = EntityAttachmentFace::North;
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            },
        );

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout_z_offset.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}
