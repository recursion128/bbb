use super::*;
use crate::entity_models::model::EntityModel;

#[test]
fn ender_dragon_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EnderDragonModel.createBodyLayer` (atlas 256×256): head (+jaw), five neck and twelve
    // tail spine segments, and the body (+wings +legs) — 19 root parts hung off the mesh root. The
    // tree is hand-built by `EnderDragonModel::new()`; the structural assertions walk it by the
    // index child names `StaticModel` used.
    let mut model = EnderDragonModel::new();

    // `head` (offset (0, 20, -62)): six cubes, parenting the jaw at (0, 4, -8).
    let head = model.root_mut().child_mut("0");
    assert_eq!(head.pose.offset, [0.0, 20.0, -62.0]);
    assert_eq!(DRAGON_HEAD_CUBES.len(), 6);
    assert_eq!(DRAGON_HEAD_CUBES[1].size, [16.0, 16.0, 16.0]);
    let jaw = head.child_mut("0");
    assert_eq!(jaw.pose.offset, [0.0, 4.0, -8.0]);
    assert_eq!(DRAGON_JAW_CUBES[0].size, [12.0, 4.0, 16.0]);

    // The five neck segments at `offset(0, 20, -12 - i·10)`, each the 2-cube spine.
    for i in 0..5 {
        let neck = model.root_mut().child_mut(&(1 + i).to_string());
        assert_eq!(neck.pose.offset, [0.0, 20.0, -12.0 - i as f32 * 10.0]);
    }
    assert_eq!(DRAGON_SPINE_CUBES.len(), 2);
    assert_eq!(DRAGON_SPINE_CUBES[0].size, [10.0, 10.0, 10.0]);

    // The twelve tail segments at `offset(0, 10, 60 + i·10)`, each the 2-cube spine.
    for i in 0..12 {
        let tail = model.root_mut().child_mut(&(6 + i).to_string());
        assert_eq!(tail.pose.offset, [0.0, 10.0, 60.0 + i as f32 * 10.0]);
    }

    // `body` (offset (0, 3, 8)): four cubes, parenting two wings and four legs.
    let body = model.root_mut().child_mut("18");
    assert_eq!(body.pose.offset, [0.0, 3.0, 8.0]);
    assert_eq!(DRAGON_BODY_CUBES[0].size, [24.0, 24.0, 64.0]);

    // `left_wing` (offset (12, 2, -6)): the bone plus the membrane, parenting the wing tip.
    let left_wing = body.child_mut("0");
    assert_eq!(left_wing.pose.offset, [12.0, 2.0, -6.0]);
    assert_eq!(DRAGON_LEFT_WING_CUBES[0].size, [56.0, 8.0, 8.0]);
    assert_eq!(DRAGON_LEFT_WING_CUBES[1].size, [56.0, 0.0, 56.0]);
    assert_eq!(left_wing.child_mut("0").pose.offset, [56.0, 0.0, 0.0]);

    // A front leg is a three-segment chain (leg → tip → foot) with the vanilla bind rotations.
    let left_front_leg = model.root_mut().child_mut("18").child_mut("1");
    assert_eq!(left_front_leg.pose.offset, [12.0, 17.0, -6.0]);
    assert_eq!(left_front_leg.pose.rotation, [1.3, 0.0, 0.0]);
    let leg_tip = left_front_leg.child_mut("0");
    assert_eq!(leg_tip.pose.rotation, [-0.5, 0.0, 0.0]);
    let foot = leg_tip.child_mut("0");
    assert_eq!(foot.pose.rotation, [0.75, 0.0, 0.0]);
    assert_eq!(DRAGON_FRONT_FOOT_CUBES[0].size, [8.0, 4.0, 16.0]);

    // The right wing extends -X (vanilla's mirror is true geometry).
    let right_wing = model.root_mut().child_mut("18").child_mut("3");
    assert_eq!(right_wing.pose.offset, [-12.0, 2.0, -6.0]);
    assert_eq!(DRAGON_RIGHT_WING_CUBES[0].min, [-56.0, -4.0, -4.0]);
}

#[test]
fn ender_dragon_mesh_uses_vanilla_body_layer_geometry() {
    // 65 cubes → 390 faces / 1560 vertices / 2340 indices; the body is dark and the wing membranes
    // carry their lighter tint.
    let dragon = entity_model_mesh(&[EntityModelInstance::ender_dragon(
        430,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(dragon.opaque_faces, 390);
    assert_eq!(dragon.vertices.len(), 1560);
    assert_eq!(dragon.indices.len(), 2340);
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_BODY, 1.0)));
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_MEMBRANE, 1.0)));
}

#[test]
fn ender_dragon_textured_render_matches_vanilla_renderer() {
    let passes = ender_dragon_textured_layer_passes();
    // The cutout base body plus the always-on emissive `dragon_eyes.png` eyes overlay.
    assert_eq!(passes.len(), 2);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(
        EntityModelKind::EnderDragon.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/enderdragon/dragon.png",
            size: [256, 256],
        })
    );
    assert!(entity_model_texture_refs().contains(&ENDER_DRAGON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ENDER_DRAGON_EYES_TEXTURE_REF));
    assert_eq!(
        ender_dragon_entity_texture_refs(),
        &[ENDER_DRAGON_TEXTURE_REF, ENDER_DRAGON_EYES_TEXTURE_REF]
    );

    let images: Vec<EntityModelTextureImage> = ender_dragon_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::ender_dragon(
            900,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );
    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}
