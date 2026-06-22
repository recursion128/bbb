use super::*;

#[test]
fn salmon_model_parts_match_vanilla_26_1_body_layer() {
    // Vanilla `SalmonModel.createBodyLayer` (atlas 32×32): body front (top fin child),
    // body back (tail fin + top fin children), head, then the two side fins (zRot ±π/4).
    assert_eq!(SALMON_PARTS.len(), 5);

    assert_part_tree(
        &SALMON_PARTS[0],
        [0.0, 20.0, -7.2],
        [0.0, 0.0, 0.0],
        SALMON_BODY_FRONT.as_slice(),
        &SALMON_BODY_FRONT_CHILDREN,
    );
    assert_eq!(
        SALMON_BODY_FRONT[0],
        ModelCubeDesc {
            min: [-1.5, -2.5, 0.0],
            size: [3.0, 5.0, 8.0],
            color: SALMON_RED,
        }
    );

    // The back body segment is index `SALMON_BODY_BACK_PART_INDEX`; it sits 0.8 forward and
    // carries the swaying tail + rear top fin.
    assert_eq!(SALMON_BODY_BACK_PART_INDEX, 1);
    assert_part_tree(
        &SALMON_PARTS[1],
        [0.0, 20.0, 0.8],
        [0.0, 0.0, 0.0],
        SALMON_BODY_BACK.as_slice(),
        &SALMON_BODY_BACK_CHILDREN,
    );
    assert_eq!(SALMON_PARTS[1].children.len(), 2);
    // back_fin (zero-thickness plane) at +8 Z, then the rear top fin at -4.5 Y / -1 Z.
    assert_part(
        &SALMON_PARTS[1].children[0],
        [0.0, 0.0, 8.0],
        [0.0, 0.0, 0.0],
        SALMON_BACK_FIN.as_slice(),
    );
    assert_eq!(SALMON_BACK_FIN[0].size, [0.0, 5.0, 6.0]);
    assert_part(
        &SALMON_PARTS[1].children[1],
        [0.0, -4.5, -1.0],
        [0.0, 0.0, 0.0],
        SALMON_TOP_BACK_FIN.as_slice(),
    );
    assert_eq!(SALMON_TOP_BACK_FIN[0].size, [0.0, 2.0, 4.0]);

    // The front body carries the forward top fin.
    assert_eq!(SALMON_PARTS[0].children.len(), 1);
    assert_part(
        &SALMON_PARTS[0].children[0],
        [0.0, -4.5, 5.0],
        [0.0, 0.0, 0.0],
        SALMON_TOP_FRONT_FIN.as_slice(),
    );
    assert_eq!(SALMON_TOP_FRONT_FIN[0].size, [0.0, 2.0, 3.0]);

    assert_part(
        &SALMON_PARTS[2],
        [0.0, 20.0, -7.2],
        [0.0, 0.0, 0.0],
        SALMON_HEAD.as_slice(),
    );
    assert_eq!(SALMON_HEAD[0].size, [2.0, 4.0, 3.0]);

    // Side fins: zero-height planes rotated ±π/4 about Z.
    assert_part(
        &SALMON_PARTS[3],
        [-1.5, 21.5, -7.2],
        [0.0, 0.0, -std::f32::consts::FRAC_PI_4],
        SALMON_RIGHT_FIN.as_slice(),
    );
    assert_part(
        &SALMON_PARTS[4],
        [1.5, 21.5, -7.2],
        [0.0, 0.0, std::f32::consts::FRAC_PI_4],
        SALMON_LEFT_FIN.as_slice(),
    );
    assert_eq!(SALMON_RIGHT_FIN[0].size, [2.0, 0.0, 2.0]);
    assert_eq!(SALMON_LEFT_FIN[0].size, [2.0, 0.0, 2.0]);
}

#[test]
fn salmon_swim_multipliers_match_vanilla() {
    // `SalmonModel.setupAnim` / `SalmonRenderer.setupRotations`: a swimming salmon uses
    // `(1.0, 1.0)`; a beached salmon thrashes harder and faster `(1.3, 1.7)`.
    assert_eq!(salmon_swim_multipliers(true), (1.0, 1.0));
    assert_eq!(salmon_swim_multipliers(false), (1.3, 1.7));
}

#[test]
fn salmon_body_back_sway_matches_vanilla_setup_anim() {
    // `bodyBack.yRot = -amplitude * 0.25 * sin(angle * 0.6 * ageInTicks)`. At age 0 the
    // sway is zero regardless of amplitude.
    assert_eq!(salmon_body_back_yrot(0.0, true), 0.0);
    assert_eq!(salmon_body_back_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (1.0 * 0.6 * age).sin();
    assert!((salmon_body_back_yrot(age, true) - (-1.0 * 0.25 * s)).abs() < 1.0e-6);
    let s_out = (1.7 * 0.6 * age).sin();
    assert!((salmon_body_back_yrot(age, false) - (-1.3 * 0.25 * s_out)).abs() < 1.0e-6);
}

#[test]
fn salmon_mesh_uses_vanilla_body_layer_geometry() {
    // Eight cubes (body front + top fin, body back + tail + top fin, head, two side fins)
    // → 48 faces / 192 vertices.
    let salmon = entity_model_mesh(&[EntityModelInstance::salmon(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        SalmonModelSize::Medium,
    )
    .with_in_water(true)]);
    assert_eq!(salmon.opaque_faces, 48);
    assert_eq!(salmon.vertices.len(), 192);
    assert_eq!(salmon.indices.len(), 288);
    assert!(salmon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SALMON_RED, 1.0)));
}

#[test]
fn salmon_flops_when_out_of_water() {
    // `SalmonRenderer.setupRotations` lays a beached salmon on its side (`RotZ(90)` +
    // offset). At age 0 the swim wiggle and body sway are both zero, so the only difference
    // between an in-water and a beached salmon is the flop.
    let base = EntityModelInstance::salmon(701, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Medium);
    let swimming = entity_model_mesh(&[base.with_in_water(true)]);
    let beached = entity_model_mesh(&[base.with_in_water(false)]);
    assert_eq!(swimming.vertices.len(), beached.vertices.len());
    assert_ne!(
        swimming.vertices, beached.vertices,
        "the beached salmon flops"
    );

    // The upright salmon is taller (Y) than wide (X); the 90° flop swaps those extents.
    let (swim_min, swim_max) = mesh_extents(&swimming);
    let (beach_min, beach_max) = mesh_extents(&beached);
    assert!(
        (swim_max[1] - swim_min[1]) > (swim_max[0] - swim_min[0]),
        "an upright salmon is taller than it is wide"
    );
    assert!(
        (beach_max[0] - beach_min[0]) > (beach_max[1] - beach_min[1]),
        "a beached salmon is wider than it is tall"
    );
}

#[test]
fn salmon_sways_its_back_with_age() {
    // A still salmon (age 0) is inert; advancing the age sways the back body segment (and
    // its tail/top-fin children) plus the renderer body wiggle.
    let base = EntityModelInstance::salmon(702, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Medium)
        .with_in_water(true);
    let still = entity_model_mesh(&[base]);
    let swimming = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), swimming.vertices.len());
    assert_ne!(still.vertices, swimming.vertices, "the back sways with age");
}

#[test]
fn salmon_size_variants_scale_the_mesh() {
    // `SalmonRenderer` swaps between small/medium/large `SalmonModel` layers, which differ
    // only by a `MeshTransformer.scaling` factor of 0.5 / 1.0 / 1.5.
    assert_eq!(SalmonModelSize::Small.scale(), 0.5);
    assert_eq!(SalmonModelSize::Medium.scale(), 1.0);
    assert_eq!(SalmonModelSize::Large.scale(), 1.5);

    let position = [0.0, 64.0, 0.0];
    let small = entity_model_mesh(&[EntityModelInstance::salmon(
        710,
        position,
        0.0,
        SalmonModelSize::Small,
    )
    .with_in_water(true)]);
    let medium = entity_model_mesh(&[EntityModelInstance::salmon(
        711,
        position,
        0.0,
        SalmonModelSize::Medium,
    )
    .with_in_water(true)]);
    let large = entity_model_mesh(&[EntityModelInstance::salmon(
        712,
        position,
        0.0,
        SalmonModelSize::Large,
    )
    .with_in_water(true)]);

    // All three carry the same cube count; only the spatial extents differ.
    assert_eq!(small.vertices.len(), medium.vertices.len());
    assert_eq!(large.vertices.len(), medium.vertices.len());

    let extent = |mesh: &EntityModelMesh| {
        let (min, max) = mesh_extents(mesh);
        [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
    };
    let s = extent(&small);
    let m = extent(&medium);
    let l = extent(&large);
    for axis in 0..3 {
        assert!(
            s[axis] < m[axis],
            "the small salmon is smaller on axis {axis}"
        );
        assert!(
            l[axis] > m[axis],
            "the large salmon is bigger on axis {axis}"
        );
    }
}

#[test]
fn salmon_from_vanilla_variant_id_clamps() {
    // `Salmon.Variant`: SMALL(0) / MEDIUM(1, the default) / LARGE(2, any other id).
    assert_eq!(SalmonModelSize::from_vanilla_id(0), SalmonModelSize::Small);
    assert_eq!(SalmonModelSize::from_vanilla_id(1), SalmonModelSize::Medium);
    assert_eq!(SalmonModelSize::from_vanilla_id(2), SalmonModelSize::Large);
    assert_eq!(SalmonModelSize::from_vanilla_id(99), SalmonModelSize::Large);
}

#[test]
fn salmon_texture_ref_matches_vanilla_renderer() {
    // `SalmonRenderer` keys every size on `ModelLayers.SALMON*`; all share one texture.
    assert_eq!(
        EntityModelKind::Salmon {
            size: SalmonModelSize::Small
        }
        .model_key(),
        "salmon_small"
    );
    assert_eq!(
        EntityModelKind::Salmon {
            size: SalmonModelSize::Medium
        }
        .model_key(),
        "salmon"
    );
    assert_eq!(
        EntityModelKind::Salmon {
            size: SalmonModelSize::Large
        }
        .model_key(),
        "salmon_large"
    );
    assert_eq!(
        EntityModelKind::Salmon {
            size: SalmonModelSize::Medium
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/fish/salmon.png",
            size: [32, 32],
        })
    );
}

#[test]
fn salmon_textured_layer_passes_match_vanilla_renderer() {
    // `SalmonRenderer` renders a single cutout base layer; only the bound model layer
    // differs per size (`ModelLayers.SALMON` / `SALMON_SMALL` / `SALMON_LARGE`).
    for (size, layer) in [
        (SalmonModelSize::Small, "minecraft:salmon_small#main"),
        (SalmonModelSize::Medium, "minecraft:salmon#main"),
        (SalmonModelSize::Large, "minecraft:salmon_large#main"),
    ] {
        let passes = salmon_textured_layer_passes(size);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::SalmonBase);
        assert_eq!(passes[0].model_layer, layer);
        assert_eq!(passes[0].texture, SALMON_TEXTURE_REF);
        assert_eq!(passes[0].parts, SALMON_TEXTURED_PARTS.as_slice());
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    }

    // The textured parts mirror the colored poses, including the nested fin children, so
    // the `setupAnim` body-back sway re-poses the same subtree in both paths.
    for (colored, textured) in SALMON_PARTS.iter().zip(SALMON_TEXTURED_PARTS.iter()) {
        assert_eq!(colored.pose, textured.pose);
    }
    assert_eq!(
        SALMON_TEXTURED_PARTS[SALMON_BODY_BACK_PART_INDEX]
            .children
            .len(),
        2
    );
    assert_eq!(
        SALMON_TEXTURED_PARTS[SALMON_BODY_BACK_PART_INDEX].children[0].pose,
        SALMON_BODY_BACK_CHILDREN[0].pose
    );
    assert_eq!(
        SALMON_TEXTURED_PARTS[0].children[0].pose,
        SALMON_BODY_FRONT_CHILDREN[0].pose
    );
    // No cube mirrors and `uv_size` equals the geometry size (`CubeDeformation.NONE`); the
    // right fin keeps its negative `texOffs(-4, 0)` U origin.
    assert!(!SALMON_TEXTURED_BODY_FRONT[0].mirror);
    assert_eq!(
        SALMON_TEXTURED_BODY_FRONT[0].uv_size,
        SALMON_TEXTURED_BODY_FRONT[0].size
    );
    assert_eq!(SALMON_TEXTURED_RIGHT_FIN[0].tex, [-4.0, 0.0]);
    assert_eq!(SALMON_TEXTURED_BODY_BACK[0].tex, [0.0, 13.0]);
}

#[test]
fn salmon_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&salmon_texture_images()).unwrap();
    // Eight cubes → 192 textured vertices on the cutout pass.
    let base = EntityModelInstance::salmon(720, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Medium)
        .with_in_water(true);
    let still = entity_model_textured_mesh(&[base], &atlas);
    assert_eq!(still.vertices.len(), 192);

    // The back-body sway / body wiggle reorient the mesh as the age advances.
    let swimming = entity_model_textured_mesh(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_eq!(still.vertices.len(), swimming.vertices.len());
    assert_ne!(still.vertices, swimming.vertices);

    // A beached salmon flops onto its side.
    let beached = entity_model_textured_mesh(&[base.with_in_water(false)], &atlas);
    assert_ne!(still.vertices, beached.vertices);

    // The size variants scale the textured mesh exactly like the colored path.
    let small = entity_model_textured_mesh(
        &[
            EntityModelInstance::salmon(721, [0.0, 64.0, 0.0], 0.0, SalmonModelSize::Small)
                .with_in_water(true),
        ],
        &atlas,
    );
    assert_eq!(small.vertices.len(), still.vertices.len());
    assert_ne!(small.vertices, still.vertices, "the small salmon is scaled");
}

fn salmon_texture_images() -> Vec<EntityModelTextureImage> {
    salmon_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
