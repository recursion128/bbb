use super::*;
use crate::entity_models::colored::pufferfish_model_root_transform;
use std::f32::consts::FRAC_PI_4;

// (offset, rotation, cube min, cube size, texOffs)
type Row = ([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 2]);

fn assert_parts(parts: &[PufferfishPart], expected: &[Row]) {
    assert_eq!(parts.len(), expected.len());
    for (index, (offset, rotation, min, size, tex)) in expected.iter().enumerate() {
        let part = &parts[index];
        assert_eq!(part.offset, *offset, "part {index} offset");
        assert_eq!(part.rotation, *rotation, "part {index} rotation");
        assert_eq!(part.min, *min, "part {index} min");
        assert_eq!(part.size, *size, "part {index} size");
        assert_eq!(part.tex, *tex, "part {index} texOffs");
    }
}

#[test]
fn pufferfish_small_parts_match_vanilla_26_1_body_layer() {
    let z = [0.0, 0.0, 0.0];
    assert_parts(
        &PUFFERFISH_SMALL_PARTS,
        &[
            (
                [0.0, 23.0, 0.0],
                z,
                [-1.5, -2.0, -1.5],
                [3.0, 2.0, 3.0],
                [0.0, 27.0],
            ),
            (
                [0.0, 20.0, 0.0],
                z,
                [-1.5, 0.0, -1.5],
                [1.0, 1.0, 1.0],
                [24.0, 6.0],
            ),
            (
                [0.0, 20.0, 0.0],
                z,
                [0.5, 0.0, -1.5],
                [1.0, 1.0, 1.0],
                [28.0, 6.0],
            ),
            (
                [0.0, 22.0, 1.5],
                z,
                [-1.5, 0.0, 0.0],
                [3.0, 0.0, 3.0],
                [-3.0, 0.0],
            ),
            (
                [-1.5, 22.0, -1.5],
                z,
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 2.0],
                [25.0, 0.0],
            ),
            (
                [1.5, 22.0, -1.5],
                z,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 2.0],
                [25.0, 0.0],
            ),
        ],
    );
    // The pectoral fins are the fifth and sixth children (`right_fin`/`left_fin`), the two that
    // `setup_anim` wiggles by name.
    assert_eq!(PUFFERFISH_SMALL_NAMES[4..], ["right_fin", "left_fin"]);
    assert_eq!(PUFFERFISH_SMALL_FIN_NAMES, ["right_fin", "left_fin"]);
}

#[test]
fn pufferfish_mid_parts_match_vanilla_26_1_body_layer() {
    let p = FRAC_PI_4;
    let n = -FRAC_PI_4;
    assert_parts(
        &PUFFERFISH_MID_PARTS,
        &[
            (
                [0.0, 22.0, 0.0],
                [0.0, 0.0, 0.0],
                [-2.5, -5.0, -2.5],
                [5.0, 5.0, 5.0],
                [12.0, 22.0],
            ),
            (
                [-2.5, 18.0, -1.5],
                [0.0, 0.0, 0.0],
                [-2.0, 0.0, 0.0],
                [2.0, 0.0, 2.0],
                [24.0, 0.0],
            ),
            (
                [2.5, 18.0, -1.5],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 2.0],
                [24.0, 3.0],
            ),
            (
                [0.0, 17.0, -2.5],
                [p, 0.0, 0.0],
                [-2.5, -1.0, 0.0],
                [5.0, 1.0, 0.0],
                [19.0, 17.0],
            ),
            (
                [0.0, 17.0, 2.5],
                [n, 0.0, 0.0],
                [-2.5, -1.0, 0.0],
                [5.0, 1.0, 0.0],
                [11.0, 17.0],
            ),
            (
                [-2.5, 22.0, -2.5],
                [0.0, n, 0.0],
                [-1.0, -5.0, 0.0],
                [1.0, 5.0, 0.0],
                [5.0, 17.0],
            ),
            (
                [-2.5, 22.0, 2.5],
                [0.0, p, 0.0],
                [-1.0, -5.0, 0.0],
                [1.0, 5.0, 0.0],
                [9.0, 17.0],
            ),
            (
                [2.5, 22.0, 2.5],
                [0.0, n, 0.0],
                [0.0, -5.0, 0.0],
                [1.0, 5.0, 0.0],
                [1.0, 17.0],
            ),
            (
                [2.5, 22.0, -2.5],
                [0.0, p, 0.0],
                [0.0, -5.0, 0.0],
                [1.0, 5.0, 0.0],
                [1.0, 17.0],
            ),
            (
                [-2.5, 22.0, 2.5],
                [p, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [5.0, 1.0, 0.0],
                [18.0, 20.0],
            ),
            (
                [0.0, 22.0, -2.5],
                [n, 0.0, 0.0],
                [-2.5, 0.0, 0.0],
                [5.0, 1.0, 1.0],
                [17.0, 19.0],
            ),
        ],
    );
    // The blue fins are the second and third children (`right_blue_fin`/`left_blue_fin`), the two
    // that `setup_anim` wiggles by name.
    assert_eq!(
        PUFFERFISH_MID_NAMES[1..3],
        ["right_blue_fin", "left_blue_fin"]
    );
    assert_eq!(
        PUFFERFISH_MID_FIN_NAMES,
        ["right_blue_fin", "left_blue_fin"]
    );
}

#[test]
fn pufferfish_big_parts_match_vanilla_26_1_body_layer() {
    let p = FRAC_PI_4;
    let n = -FRAC_PI_4;
    assert_parts(
        &PUFFERFISH_BIG_PARTS,
        &[
            (
                [0.0, 22.0, 0.0],
                [0.0, 0.0, 0.0],
                [-4.0, -8.0, -4.0],
                [8.0, 8.0, 8.0],
                [0.0, 0.0],
            ),
            (
                [-4.0, 15.0, -2.0],
                [0.0, 0.0, 0.0],
                [-2.0, 0.0, -1.0],
                [2.0, 1.0, 2.0],
                [24.0, 0.0],
            ),
            (
                [4.0, 15.0, -2.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
                [2.0, 1.0, 2.0],
                [24.0, 3.0],
            ),
            (
                [0.0, 14.0, -4.0],
                [p, 0.0, 0.0],
                [-4.0, -1.0, 0.0],
                [8.0, 1.0, 0.0],
                [15.0, 17.0],
            ),
            (
                [0.0, 14.0, 0.0],
                [0.0, 0.0, 0.0],
                [-4.0, -1.0, 0.0],
                [8.0, 1.0, 1.0],
                [14.0, 16.0],
            ),
            (
                [0.0, 14.0, 4.0],
                [n, 0.0, 0.0],
                [-4.0, -1.0, 0.0],
                [8.0, 1.0, 0.0],
                [23.0, 18.0],
            ),
            (
                [-4.0, 22.0, -4.0],
                [0.0, n, 0.0],
                [-1.0, -8.0, 0.0],
                [1.0, 8.0, 0.0],
                [5.0, 17.0],
            ),
            (
                [4.0, 22.0, -4.0],
                [0.0, p, 0.0],
                [0.0, -8.0, 0.0],
                [1.0, 8.0, 0.0],
                [1.0, 17.0],
            ),
            (
                [0.0, 22.0, -4.0],
                [n, 0.0, 0.0],
                [-4.0, 0.0, 0.0],
                [8.0, 1.0, 0.0],
                [15.0, 20.0],
            ),
            (
                [0.0, 22.0, 0.0],
                [0.0, 0.0, 0.0],
                [-4.0, 0.0, 0.0],
                [8.0, 1.0, 0.0],
                [15.0, 20.0],
            ),
            (
                [0.0, 22.0, 4.0],
                [p, 0.0, 0.0],
                [-4.0, 0.0, 0.0],
                [8.0, 1.0, 0.0],
                [15.0, 20.0],
            ),
            (
                [-4.0, 22.0, 4.0],
                [0.0, p, 0.0],
                [-1.0, -8.0, 0.0],
                [1.0, 8.0, 0.0],
                [9.0, 17.0],
            ),
            (
                [4.0, 22.0, 4.0],
                [0.0, n, 0.0],
                [0.0, -8.0, 0.0],
                [1.0, 8.0, 0.0],
                [9.0, 17.0],
            ),
        ],
    );
    // The blue fins are the second and third children (`right_blue_fin`/`left_blue_fin`), the two
    // that `setup_anim` wiggles by name.
    assert_eq!(
        PUFFERFISH_BIG_NAMES[1..3],
        ["right_blue_fin", "left_blue_fin"]
    );
    assert_eq!(
        PUFFERFISH_BIG_FIN_NAMES,
        ["right_blue_fin", "left_blue_fin"]
    );
}

#[test]
fn pufferfish_parts_select_model_by_puff_state() {
    assert_eq!(pufferfish_parts(0).0.len(), 6);
    assert_eq!(pufferfish_parts(1).0.len(), 11);
    assert_eq!(pufferfish_parts(2).0.len(), 13);
    // Vanilla `PufferfishRenderer.submit` default branch: any state >= 2 renders the big model.
    assert_eq!(pufferfish_parts(7).0.len(), 13);
}

#[test]
fn pufferfish_texture_ref_matches_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Pufferfish { puff_state: 0 }.model_key(),
        "pufferfish_small"
    );
    assert_eq!(
        EntityModelKind::Pufferfish { puff_state: 1 }.model_key(),
        "pufferfish_mid"
    );
    assert_eq!(
        EntityModelKind::Pufferfish { puff_state: 2 }.model_key(),
        "pufferfish_big"
    );
    assert_eq!(
        EntityModelKind::Pufferfish { puff_state: 0 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/fish/pufferfish.png",
            size: [32, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&PUFFERFISH_TEXTURE_REF));
    assert_eq!(pufferfish_entity_texture_refs(), &[PUFFERFISH_TEXTURE_REF]);
}

#[test]
fn pufferfish_fin_wiggle_matches_vanilla_setup_anim() {
    // Vanilla: right fin zRot = -0.2 + 0.4 * sin(ageInTicks * 0.2); left fin is the negation.
    for age in [0.0f32, 7.5, 31.4, 100.0] {
        let want = -0.2 + 0.4 * (age * 0.2).sin();
        assert!(
            (pufferfish_right_fin_z_rot(age) - want).abs() < 1e-7,
            "age {age}"
        );
    }
    // The fin pose sets zRot absolutely over the zeroed rest pose.
    let base = PUFFERFISH_SMALL_PARTS[4].pose();
    let posed = pufferfish_fin_pose(base, 0.37);
    assert_eq!(posed.offset, base.offset);
    assert_eq!(posed.rotation, [0.0, 0.0, 0.37]);
}

#[test]
fn pufferfish_meshes_use_vanilla_body_layer_geometry() {
    // small 6 cubes, mid 11, big 13 => 24 verts / 6 faces per cube.
    for (puff_state, cubes) in [(0, 6usize), (1, 11), (2, 13)] {
        let mesh = entity_model_mesh(&[EntityModelInstance::pufferfish(
            107,
            [0.0, 64.0, 0.0],
            0.0,
            puff_state,
        )]);
        assert_eq!(mesh.opaque_faces, cubes * 6, "puff {puff_state} faces");
        assert_eq!(mesh.vertices.len(), cubes * 24, "puff {puff_state} verts");
        assert_eq!(mesh.indices.len(), cubes * 36, "puff {puff_state} indices");
    }
    // The big model's widest reach is the four corner fins (offset +-4px, rotated 45deg in y,
    // so they extend to ~+-5.96px), past the 8px body.
    let big = entity_model_mesh(&[EntityModelInstance::pufferfish(
        107,
        [0.0, 64.0, 0.0],
        0.0,
        2,
    )]);
    let (min, max) = mesh_extents(&big);
    assert_close3(min, [-0.37250832, 64.1435, -0.29419422]);
    assert_close3(max, [0.37250835, 64.7685, 0.29419422]);
}

#[test]
fn pufferfish_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&pufferfish_texture_images()).unwrap();
    let instance = EntityModelInstance::pufferfish(107, [0.0, 64.0, 0.0], 0.0, 2);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, PUFFERFISH_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, pufferfish_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 78);
    assert_eq!(mesh.vertices.len(), 312);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // Same geometry and transform as the colored path, so the extents match exactly.
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.37250832, 64.1435, -0.29419422]);
    assert_close3(max, [0.37250835, 64.7685, 0.29419422]);
}

#[test]
fn pufferfish_fins_wiggle_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: the two fins wiggle by ageInTicks. Compare two ages
    // whose vertical bob is equal (cos(age*0.05) matched) so only the fin wiggle differs.
    let base = EntityModelInstance::pufferfish(107, [0.0, 64.0, 0.0], 0.0, 0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(15.0)]);
    assert_ne!(
        early.vertices, later.vertices,
        "the fins wiggle as ageInTicks advances"
    );
}

#[test]
fn pufferfish_body_bobs_vertically_with_age() {
    // Vanilla PufferfishRenderer.setupRotations bobs the whole model:
    // translate(0, cos(ageInTicks * 0.05) * 0.08, 0). The body (part 0, verts [0, 24)) only
    // bobs, so its y shifts by the bob delta between two ages while x/z hold.
    let base = EntityModelInstance::pufferfish(107, [0.0, 64.0, 0.0], 0.0, 2);
    let a = entity_model_mesh(&[base]);
    let b = entity_model_mesh(&[base.with_age_in_ticks(std::f32::consts::PI / 0.05)]);
    // bob(0) = 0.08, bob(pi/0.05) = cos(pi) * 0.08 = -0.08, so |delta| = 0.16.
    let dy = b.vertices[0].position[1] - a.vertices[0].position[1];
    assert!((dy.abs() - 0.16).abs() < 1e-4, "body bobs by 0.16: {dy}");
    assert!((b.vertices[0].position[0] - a.vertices[0].position[0]).abs() < 1e-6);
    assert!((b.vertices[0].position[2] - a.vertices[0].position[2]).abs() < 1e-6);
}

fn pufferfish_texture_images() -> Vec<EntityModelTextureImage> {
    pufferfish_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
