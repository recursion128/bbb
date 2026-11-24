use super::*;
use crate::entity_models::model_layers::evoker_fangs_model;

#[test]
fn evoker_fangs_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = evoker_fangs_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, EVOKER_FANGS_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    assert!(entity_model_texture_refs().contains(&EVOKER_FANGS_TEXTURE_REF));
}

#[test]
fn evoker_fangs_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = [EVOKER_FANGS_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // A mid-attack fang (`biteProgress > 0`); a resting fang is hidden underground (vanilla render gate).
    let instance = EntityModelInstance::evoker_fangs(470, [1.0, 64.0, -2.0], 35.0).with(|s| {
        s.kind.evoker_fangs_mut().bite_progress = 0.5;
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    assert_eq!(meshes.cutout.cutout_faces, 18);
    assert_eq!(meshes.cutout.vertices.len(), 72);
    assert_eq!(meshes.cutout.indices.len(), 108);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

#[test]
fn evoker_fangs_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::evoker_fangs(471, [1.0, 64.0, -2.0], 35.0).with(|s| {
        s.kind.evoker_fangs_mut().bite_progress = 0.5;
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn evoker_fangs_bite_progress_snaps_the_jaws_and_vanishes() {
    use std::f32::consts::PI;

    let base = EntityModelInstance::evoker_fangs(470, [0.0, 64.0, 0.0], 0.0);

    // biteProgress 0: vanilla `EvokerFangsRenderer` skips the render entirely (the fang
    // is still underground), so the whole model is hidden.
    let hidden = evoker_fangs_model(&base);
    assert!(!hidden.visible, "an un-attacked fang is hidden");

    // biteProgress 0.5: the cubic ease-out `biteAmount` has reached 0, so both jaws have
    // snapped fully shut (zRot = π); the fang has risen out of the ground (base.y below
    // its bind 24); and the model is at full size (preScale = 1).
    let mut biting =
        evoker_fangs_model(&base.with(|s| s.kind.evoker_fangs_mut().bite_progress = 0.5));
    assert!(biting.visible, "an attacking fang is shown");
    assert_eq!(biting.scale, [1.0; 3], "full size mid-bite");
    let base_part = biting.child_mut("base");
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
    let mut opening =
        evoker_fangs_model(&base.with(|s| s.kind.evoker_fangs_mut().bite_progress = 0.05));
    let upper_open = opening
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
    let vanished =
        evoker_fangs_model(&base.with(|s| s.kind.evoker_fangs_mut().bite_progress = 1.0));
    assert_eq!(
        vanished.scale, [0.0; 3],
        "the fang vanishes at the end of the bite"
    );
}
