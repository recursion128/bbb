use super::*;

use crate::entity_models::colored::cod_model_root_transform;
use crate::entity_models::model::ModelCube;

#[test]
fn cod_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `CodModel.createBodyLayer` (atlas 32×32): seven cubes — body, head, nose, the two
    // side fins, tail fin, top fin. Each unified cube carries the colored tint (`COD_TAN`) and the
    // textured UV (`CubeDeformation.NONE`, so `uv_size == size`).
    assert_eq!(
        COD_BODY[0],
        ModelCube::new(
            [-1.0, -2.0, 0.0],
            [2.0, 4.0, 7.0],
            COD_TAN,
            [2.0, 4.0, 7.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(COD_HEAD[0].tex, [11.0, 0.0]);
    assert_eq!(COD_NOSE[0].size, [2.0, 3.0, 1.0]);
    // The fins are zero-thickness planes.
    assert_eq!(COD_RIGHT_FIN[0].size, [2.0, 0.0, 2.0]);
    assert_eq!(COD_TAIL_FIN[0].size, [0.0, 4.0, 4.0]);
    assert_eq!(COD_TOP_FIN[0].size, [0.0, 1.0, 6.0]);
    // Every cube wears the cod tint on the colored path.
    for cube in [
        COD_BODY[0],
        COD_HEAD[0],
        COD_NOSE[0],
        COD_RIGHT_FIN[0],
        COD_LEFT_FIN[0],
        COD_TAIL_FIN[0],
        COD_TOP_FIN[0],
    ] {
        assert_eq!(cube.color, COD_TAN);
    }
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
fn cod_textured_cubes_match_vanilla_renderer() {
    // The unified cubes carry the vanilla `texOffs` for the single cutout pass; the top fin keeps
    // its negative `texOffs(20, -6)` V origin. The cod texture ref is the vanilla atlas path.
    assert_eq!(COD_TOP_FIN[0].tex, [20.0, -6.0]);
    assert_eq!(COD_RIGHT_FIN[0].tex, [22.0, 1.0]);
    assert_eq!(COD_LEFT_FIN[0].tex, [22.0, 4.0]);
    assert_eq!(COD_TAIL_FIN[0].tex, [22.0, 3.0]);
    assert_eq!(
        COD_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/fish/cod.png",
            size: [32, 32],
        }
    );
}

#[test]
fn cod_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&cod_texture_images()).unwrap();
    // Vanilla `CodModel` calls `EntityModel(root)`, so the base submit uses the default
    // `entityCutout` render type. The backend folds it into the cutout mesh, but the submission
    // keeps the vanilla texture, render type, tint, transform, and default collector order.
    let base = EntityModelInstance::cod(910, [0.0, 64.0, 0.0], 0.0).with_in_water(true);
    let still_meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_cod_base_submission(&still_meshes, base, true);

    // Seven cubes → 168 textured vertices on the cutout pass.
    assert!(still_meshes.translucent.vertices.is_empty());
    assert!(still_meshes.eyes.vertices.is_empty());
    assert_eq!(still_meshes.cutout.vertices.len(), 168);

    // The tail sway / body wiggle reorient the mesh as the age advances.
    let swimming = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_cod_base_submission(&swimming, base.with_age_in_ticks(7.0), true);
    assert_eq!(
        still_meshes.cutout.vertices.len(),
        swimming.cutout.vertices.len()
    );
    assert_ne!(still_meshes.cutout.vertices, swimming.cutout.vertices);

    // A beached cod flops onto its side.
    let beached_instance = base.with_in_water(false);
    let beached = entity_model_textured_meshes(&[beached_instance], &atlas);
    assert_cod_base_submission(&beached, beached_instance, false);
    assert_ne!(still_meshes.cutout.vertices, beached.cutout.vertices);
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

fn assert_cod_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    in_water: bool,
) {
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, COD_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        submit.transform,
        cod_model_root_transform(instance, in_water)
    );
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}
