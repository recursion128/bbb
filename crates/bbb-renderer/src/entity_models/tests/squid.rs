use super::*;

#[test]
fn squid_model_parts_match_vanilla_26_1_body_layer() {
    // Body cube carries `CubeDeformation(0.02)`: box(-6, -8, -6, 12×16×12) inflated 0.02.
    assert_eq!(
        SQUID_BODY[0],
        ModelCubeDesc {
            min: [-6.02, -8.02, -6.02],
            size: [12.04, 16.04, 12.04],
            color: SQUID_BLUE,
        }
    );
    // Tentacle cube: `texOffs(48, 0)` box(-1, 0, -1, 2×18×2), no deformation.
    assert_eq!(
        SQUID_TENTACLE[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 18.0, 2.0],
            color: SQUID_BLUE,
        }
    );

    // Nine parts: the body followed by the eight tentacle ring. The body sits at
    // `offset(0, 8, 0)`.
    let parts = squid_model_parts(0.0);
    assert_eq!(parts.len(), 9);
    assert_part(
        &parts[0],
        [0.0, 8.0, 0.0],
        [0.0, 0.0, 0.0],
        SQUID_BODY.as_slice(),
    );

    // The tentacle ring: tentacle `i` at `(cos(i·2π/8)·5, 15, sin(i·2π/8)·5)`, yawed
    // `-i·2π/8 + π/2`. Spot-check the four cardinal tentacles (indices 0/2/4/6 → parts
    // 1/3/5/7) and confirm each uses the shared tentacle cube.
    let half_pi = std::f32::consts::FRAC_PI_2;
    for (part_index, offset, y_rot) in [
        (1usize, [5.0, 15.0, 0.0], half_pi),
        (3, [0.0, 15.0, 5.0], 0.0),
        (5, [-5.0, 15.0, 0.0], -half_pi),
        (7, [0.0, 15.0, -5.0], -std::f32::consts::PI),
    ] {
        assert_close3(parts[part_index].pose.offset, offset);
        assert_close3(parts[part_index].pose.rotation, [0.0, y_rot, 0.0]);
        assert_eq!(parts[part_index].cubes, SQUID_TENTACLE.as_slice());
    }
}

#[test]
fn squid_tentacle_sweep_applies_tentacle_angle_to_every_tentacle() {
    // `SquidModel.setupAnim` sets `tentacle.xRot = tentacleAngle` on all eight tentacles
    // and leaves the body untouched.
    let parts = squid_model_parts(0.65);
    assert_eq!(parts[0].pose.rotation, [0.0, 0.0, 0.0], "body is static");
    for tentacle in &parts[1..] {
        assert_eq!(
            tentacle.pose.rotation[0], 0.65,
            "every tentacle sweeps by tentacleAngle"
        );
    }
}

#[test]
fn squid_mesh_uses_vanilla_body_layer_geometry_and_glow_variant() {
    // Nine cubes (body + eight tentacles) → 54 faces / 216 vertices.
    let squid = entity_model_mesh(&[EntityModelInstance::squid(
        800,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    assert_eq!(squid.opaque_faces, 54);
    assert_eq!(squid.vertices.len(), 216);
    assert_eq!(squid.indices.len(), 324);
    assert!(squid
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SQUID_BLUE, 1.0)));

    // The glow squid reuses the geometry with the glow tint.
    let glow = entity_model_mesh(&[EntityModelInstance::squid(
        801,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )]);
    assert_same_geometry(&glow, &squid);
    assert!(glow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GLOW_SQUID_TEAL, 1.0)));

    // The baby uses the `BABY_TRANSFORMER` 0.5-scaled body layer: same topology, smaller.
    let baby = entity_model_mesh(&[EntityModelInstance::squid(
        802,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    assert_eq!(baby.vertices.len(), 216);
    let (adult_min, adult_max) = mesh_extents(&squid);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_height = adult_max[1] - adult_min[1];
    let baby_height = baby_max[1] - baby_min[1];
    assert!(
        (baby_height - adult_height * 0.5).abs() < 1.0e-3,
        "baby squid is half the adult height ({baby_height} vs {adult_height})"
    );
}

#[test]
fn squid_swims_its_tentacles_when_animated() {
    // A resting squid (tentacleAngle 0) is inert; sweeping the tentacles bends them
    // without changing the vertex count.
    let base = EntityModelInstance::squid(803, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_mesh(&[base]);
    let swept = entity_model_mesh(&[base.with_squid_tentacle_angle(0.8)]);

    assert_eq!(rest.vertices.len(), swept.vertices.len());
    assert_ne!(rest.vertices, swept.vertices, "the tentacles bend");
}

#[test]
fn squid_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            false,
            "squid",
            EntityModelTextureRef {
                path: "textures/entity/squid/squid.png",
                size: [64, 32],
            },
        ),
        (
            false,
            true,
            "squid_baby",
            EntityModelTextureRef {
                path: "textures/entity/squid/squid_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            false,
            "glow_squid",
            EntityModelTextureRef {
                path: "textures/entity/squid/glow_squid.png",
                size: [64, 32],
            },
        ),
        (
            true,
            true,
            "glow_squid_baby",
            EntityModelTextureRef {
                path: "textures/entity/squid/glow_squid_baby.png",
                size: [32, 32],
            },
        ),
    ];

    for (glow, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Squid { glow, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn squid_textured_model_parts_match_colored_layout() {
    // The textured ring mirrors the colored one (same poses); only the cubes carry UVs.
    // The body's `CubeDeformation(0.02)` inflates the geometry but keeps the base UV box.
    assert_eq!(
        SQUID_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-6.02, -8.02, -6.02],
            size: [12.04, 16.04, 12.04],
            uv_size: [12.0, 16.0, 12.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(SQUID_TEXTURED_TENTACLE[0].tex, [48.0, 0.0]);
    assert_eq!(SQUID_TEXTURED_TENTACLE[0].uv_size, [2.0, 18.0, 2.0]);

    let colored = squid_model_parts(0.37);
    let textured = squid_textured_model_parts(0.37);
    assert_eq!(textured.len(), colored.len());
    for (c, t) in colored.iter().zip(textured.iter()) {
        assert_eq!(c.pose, t.pose);
    }
}

#[test]
fn squid_textured_mesh_uses_vanilla_geometry_and_variant_texture() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    // Nine cubes (body + eight tentacles) → 216 textured vertices, all on the cutout pass.
    let squid = entity_model_textured_mesh(
        &[EntityModelInstance::squid(
            810,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
        )],
        &atlas,
    );
    assert_eq!(squid.vertices.len(), 216);

    // The glow squid reuses the geometry at the same positions but samples a different
    // texture (glow_squid.png), so the vertex UVs differ from the plain squid.
    let glow = entity_model_textured_mesh(
        &[EntityModelInstance::squid(
            811,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
        )],
        &atlas,
    );
    assert_eq!(glow.vertices.len(), squid.vertices.len());
    assert_eq!(
        glow.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        squid
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        glow.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
        squid.vertices.iter().map(|v| v.uv).collect::<Vec<_>>()
    );

    // The baby uses the 0.5-scaled body layer.
    let baby = entity_model_textured_mesh(
        &[EntityModelInstance::squid(
            812,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
        )],
        &atlas,
    );
    assert_eq!(baby.vertices.len(), 216);
    let (adult_min, adult_max) = textured_mesh_extents(&squid);
    let (baby_min, baby_max) = textured_mesh_extents(&baby);
    assert!(
        ((baby_max[1] - baby_min[1]) - (adult_max[1] - adult_min[1]) * 0.5).abs() < 1.0e-3,
        "baby squid is half the adult height"
    );
}

#[test]
fn squid_textured_mesh_swims_its_tentacles() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    let base = EntityModelInstance::squid(813, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_textured_mesh(&[base], &atlas);
    let swept = entity_model_textured_mesh(&[base.with_squid_tentacle_angle(0.8)], &atlas);
    assert_eq!(rest.vertices.len(), swept.vertices.len());
    assert_ne!(rest.vertices, swept.vertices);
}

#[test]
fn squid_applies_swim_body_tilt() {
    // `SquidRenderer.setupRotations` pitches the squid by `xBodyRot` (about X) then rolls
    // it by `zBodyRot` (about Y) after the body yaw. A resting squid is upright; a pitched
    // squid is reoriented (its tall body lays toward the horizontal) without changing the
    // vertex count.
    let base = EntityModelInstance::squid(820, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_mesh(&[base]);
    let pitched = entity_model_mesh(&[base.with_squid_body_tilt(-90.0, 0.0)]);
    assert_eq!(rest.vertices.len(), pitched.vertices.len());
    assert_ne!(
        rest.vertices, pitched.vertices,
        "the pitch reorients the squid"
    );

    // A 90° pitch swaps the body's vertical extent into depth: the pitched squid is much
    // shorter in Y and deeper in Z than the upright one.
    let (rest_min, rest_max) = mesh_extents(&rest);
    let (pitch_min, pitch_max) = mesh_extents(&pitched);
    assert!(
        (pitch_max[1] - pitch_min[1]) < (rest_max[1] - rest_min[1]) - 0.3,
        "a pitched squid is shorter in Y"
    );
    assert!(
        (pitch_max[2] - pitch_min[2]) > (rest_max[2] - rest_min[2]) + 0.3,
        "a pitched squid is deeper in Z"
    );

    // The roll about Y also reorients the model.
    let rolled = entity_model_mesh(&[base.with_squid_body_tilt(0.0, 35.0)]);
    assert_ne!(
        rest.vertices, rolled.vertices,
        "the roll reorients the squid"
    );
}

fn squid_texture_images() -> Vec<EntityModelTextureImage> {
    squid_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
