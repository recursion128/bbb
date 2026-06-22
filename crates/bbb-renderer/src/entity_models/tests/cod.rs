use super::*;

#[test]
fn cod_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(COD_PARTS.len(), 7);
    // body, head, nose, then the two side fins (zRot ±π/4), tail fin, top fin.
    assert_part(
        &COD_PARTS[0],
        [0.0, 22.0, 0.0],
        [0.0, 0.0, 0.0],
        COD_BODY.as_slice(),
    );
    assert_eq!(
        COD_BODY[0],
        ModelCubeDesc {
            min: [-1.0, -2.0, 0.0],
            size: [2.0, 4.0, 7.0],
            color: COD_TAN,
        }
    );
    assert_part(
        &COD_PARTS[2],
        [0.0, 22.0, -3.0],
        [0.0, 0.0, 0.0],
        COD_NOSE.as_slice(),
    );
    assert_part(
        &COD_PARTS[3],
        [-1.0, 23.0, 0.0],
        [0.0, 0.0, -std::f32::consts::FRAC_PI_4],
        COD_RIGHT_FIN.as_slice(),
    );
    assert_part(
        &COD_PARTS[4],
        [1.0, 23.0, 0.0],
        [0.0, 0.0, std::f32::consts::FRAC_PI_4],
        COD_LEFT_FIN.as_slice(),
    );
    // The tail fin is index 5 and the top fin index 6; both are zero-thickness planes.
    assert_eq!(COD_TAIL_FIN_PART_INDEX, 5);
    assert_eq!(COD_TAIL_FIN[0].size, [0.0, 4.0, 4.0]);
    assert_eq!(COD_TOP_FIN[0].size, [0.0, 1.0, 6.0]);
    assert_eq!(COD_RIGHT_FIN[0].size, [2.0, 0.0, 2.0]);
}

#[test]
fn cod_tail_fin_sway_matches_vanilla_setup_anim() {
    // `tailFin.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, amplitude 1.0 in water
    // / 1.5 out. At age 0 the sway is zero regardless of amplitude.
    assert_eq!(cod_tail_fin_yrot(0.0, true), 0.0);
    assert_eq!(cod_tail_fin_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.6 * age).sin();
    assert!((cod_tail_fin_yrot(age, true) - (-1.0 * 0.45 * s)).abs() < 1.0e-6);
    assert!((cod_tail_fin_yrot(age, false) - (-1.5 * 0.45 * s)).abs() < 1.0e-6);
    // The beached cod thrashes harder (1.5x amplitude).
    assert!(cod_tail_fin_yrot(age, false).abs() > cod_tail_fin_yrot(age, true).abs());
}

#[test]
fn cod_mesh_uses_vanilla_body_layer_geometry() {
    // Seven cubes (body, head, nose, two fins, tail, top fin) → 42 faces / 168 vertices.
    let cod = entity_model_mesh(&[
        EntityModelInstance::cod(900, [0.0, 64.0, 0.0], 0.0).with_in_water(true)
    ]);
    assert_eq!(cod.opaque_faces, 42);
    assert_eq!(cod.vertices.len(), 168);
    assert_eq!(cod.indices.len(), 252);
    assert!(cod
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COD_TAN, 1.0)));
}

#[test]
fn cod_flops_when_out_of_water() {
    // `CodRenderer.setupRotations` lays a beached cod on its side (`RotZ(90)` + offset).
    // At age 0 the swim wiggle and tail sway are both zero, so the only difference between
    // an in-water and a beached cod is the flop, which reorients the body.
    let base = EntityModelInstance::cod(901, [0.0, 64.0, 0.0], 0.0);
    let swimming = entity_model_mesh(&[base.with_in_water(true)]);
    let beached = entity_model_mesh(&[base.with_in_water(false)]);
    assert_eq!(swimming.vertices.len(), beached.vertices.len());
    assert_ne!(swimming.vertices, beached.vertices, "the beached cod flops");

    // The upright cod is taller (Y) than wide (X); the 90° flop swaps those extents.
    let (swim_min, swim_max) = mesh_extents(&swimming);
    let (beach_min, beach_max) = mesh_extents(&beached);
    assert!(
        (swim_max[1] - swim_min[1]) > (swim_max[0] - swim_min[0]),
        "an upright cod is taller than it is wide"
    );
    assert!(
        (beach_max[0] - beach_min[0]) > (beach_max[1] - beach_min[1]),
        "a beached cod is wider than it is tall"
    );
}

#[test]
fn cod_swims_its_tail_with_age() {
    // A still cod (age 0) is inert; advancing the age sways the tail and wiggles the body.
    let base = EntityModelInstance::cod(902, [0.0, 64.0, 0.0], 0.0).with_in_water(true);
    let still = entity_model_mesh(&[base]);
    let swimming = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), swimming.vertices.len());
    assert_ne!(still.vertices, swimming.vertices, "the tail sways with age");
}

#[test]
fn cod_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Cod;
    assert_eq!(kind.model_key(), "cod");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/fish/cod.png",
            size: [32, 32],
        })
    );
}

#[test]
fn cod_textured_layer_passes_match_vanilla_renderer() {
    let passes = cod_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CodBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_COD);
    assert_eq!(passes[0].texture, COD_TEXTURE_REF);
    assert_eq!(passes[0].parts, COD_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);

    // The textured parts mirror the colored poses (the top fin keeps its negative
    // `texOffs(20, -6)` V origin).
    assert_eq!(MODEL_LAYER_COD, "minecraft:cod#main");
    assert_eq!(COD_TEXTURED_TOP_FIN[0].tex, [20.0, -6.0]);
    for (colored, textured) in COD_PARTS.iter().zip(COD_TEXTURED_PARTS.iter()) {
        assert_eq!(colored.pose, textured.pose);
    }
}

#[test]
fn cod_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&cod_texture_images()).unwrap();
    // Seven cubes → 168 textured vertices on the cutout pass.
    let base = EntityModelInstance::cod(910, [0.0, 64.0, 0.0], 0.0).with_in_water(true);
    let still = entity_model_textured_mesh(&[base], &atlas);
    assert_eq!(still.vertices.len(), 168);

    // The tail sway / body wiggle reorient the mesh as the age advances.
    let swimming = entity_model_textured_mesh(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_eq!(still.vertices.len(), swimming.vertices.len());
    assert_ne!(still.vertices, swimming.vertices);

    // A beached cod flops onto its side.
    let beached = entity_model_textured_mesh(&[base.with_in_water(false)], &atlas);
    assert_ne!(still.vertices, beached.vertices);
}

fn cod_texture_images() -> Vec<EntityModelTextureImage> {
    cod_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
