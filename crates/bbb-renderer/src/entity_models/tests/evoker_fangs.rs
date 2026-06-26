use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn evoker_fangs_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `EvokerFangsModel.createBodyLayer` (atlas 64×32): the base block (texOffs 0,0) and the
    // shared jaw box (texOffs 40,0). Each unified cube carries the colored tint and the textured UV.
    assert_eq!(
        EVOKER_FANGS_BASE_CUBE,
        ModelCube::new(
            [0.0, 0.0, 0.0],
            [10.0, 12.0, 10.0],
            EVOKER_FANGS_BASE,
            [10.0, 12.0, 10.0],
            [0.0, 0.0],
            false
        )
    );
    // Both jaws share this one box, differing only by pivot and rotation.
    assert_eq!(
        EVOKER_FANGS_JAW_CUBE,
        ModelCube::new(
            [0.0, 0.0, 0.0],
            [4.0, 14.0, 8.0],
            EVOKER_FANGS_JAW,
            [4.0, 14.0, 8.0],
            [40.0, 0.0],
            false
        )
    );
    assert_eq!(EVOKER_FANGS_TEXTURE_REF.size, [64, 32]);
}

#[test]
fn evoker_fangs_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the base and jaws carry their tints. The fang
    // must be mid-attack (`biteProgress > 0`) to be drawn at all — vanilla hides it underground while
    // `biteProgress == 0`; the face/vertex counts are pose-invariant so any visible progress works.
    let fangs = entity_model_mesh(&[
        EntityModelInstance::evoker_fangs(470, [0.0, 64.0, 0.0], 0.0)
            .with_evoker_fangs_bite_progress(0.5),
    ]);
    assert_eq!(fangs.opaque_faces, 18);
    assert_eq!(fangs.vertices.len(), 72);
    assert_eq!(fangs.indices.len(), 108);
    assert!(fangs
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(EVOKER_FANGS_BASE, 1.0)));
    assert!(fangs
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(EVOKER_FANGS_JAW, 1.0)));
}

#[test]
fn evoker_fangs_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = evoker_fangs_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, EVOKER_FANGS_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);

    assert_eq!(
        EntityModelKind::EvokerFangs.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/illager/evoker_fangs.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&EVOKER_FANGS_TEXTURE_REF));
    assert_eq!(
        evoker_fangs_entity_texture_refs(),
        &[EVOKER_FANGS_TEXTURE_REF]
    );
}

#[test]
fn evoker_fangs_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = evoker_fangs_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // A mid-attack fang (`biteProgress > 0`); a resting fang is hidden underground (vanilla render gate).
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::evoker_fangs(470, [0.0, 64.0, 0.0], 0.0)
                .with_evoker_fangs_bite_progress(0.5),
        ],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 18);
    assert_eq!(mesh.vertices.len(), 72);
    assert_eq!(mesh.indices.len(), 108);
}

#[test]
fn evoker_fangs_bite_progress_snaps_the_jaws_and_vanishes() {
    use crate::entity_models::model::EntityModel;
    use std::f32::consts::PI;

    let base = EntityModelInstance::evoker_fangs(470, [0.0, 64.0, 0.0], 0.0);

    // biteProgress 0: vanilla `EvokerFangsRenderer` skips the render entirely (the fang
    // is still underground), so the whole model is hidden.
    let mut hidden = EvokerFangsModel::new();
    hidden.prepare(&base);
    assert!(!hidden.root().visible, "an un-attacked fang is hidden");

    // biteProgress 0.5: the cubic ease-out `biteAmount` has reached 0, so both jaws have
    // snapped fully shut (zRot = π); the fang has risen out of the ground (base.y below
    // its bind 24); and the model is at full size (preScale = 1).
    let mut biting = EvokerFangsModel::new();
    biting.prepare(&base.with_evoker_fangs_bite_progress(0.5));
    assert!(biting.root().visible, "an attacking fang is shown");
    assert_eq!(biting.root().scale, [1.0; 3], "full size mid-bite");
    let base_part = biting.root_mut().child_mut("base");
    assert!(
        (base_part.child_mut("upper_jaw").pose.rotation[2] - PI).abs() < 1.0e-6,
        "the upper jaw snaps shut to zRot π"
    );
    assert!(
        (base_part.child_mut("lower_jaw").pose.rotation[2] - PI).abs() < 1.0e-6,
        "the lower jaw snaps shut to zRot π"
    );
    assert!(
        base_part.pose.offset[1] < 24.0,
        "the fang rises out of the ground (base.y below its bind 24): {}",
        base_part.pose.offset[1]
    );

    // biteProgress just opening (small positive): the jaws are still spread (biteAmount
    // ≈ 1 → upper zRot ≈ 0.65π), i.e. open wider than the fully-shut π.
    let mut opening = EvokerFangsModel::new();
    opening.prepare(&base.with_evoker_fangs_bite_progress(0.05));
    let upper_open = opening
        .root_mut()
        .child_mut("base")
        .child_mut("upper_jaw")
        .pose
        .rotation[2];
    assert!(
        upper_open < PI,
        "the jaws start open before snapping shut: {upper_open}"
    );

    // biteProgress 1.0: the final vanish — `preScale = (1 - 1)/0.1 = 0` shrinks the
    // whole model to nothing.
    let mut vanished = EvokerFangsModel::new();
    vanished.prepare(&base.with_evoker_fangs_bite_progress(1.0));
    assert_eq!(
        vanished.root().scale,
        [0.0; 3],
        "the fang vanishes at the end of the bite"
    );
}
