use super::*;

use crate::entity_models::colored::decorated_pot_model_root_transform;
use crate::entity_models::geometry::MODEL_CUBE_FACE_NORTH;
use crate::entity_models::model::EntityModel;
use crate::entity_models::model_layers::{
    DecoratedPotModel, DECORATED_POT_BACK_POSE, DECORATED_POT_BASE_TEXTURE_REF,
    DECORATED_POT_BOTTOM_POSE, DECORATED_POT_FRONT_POSE, DECORATED_POT_LEFT_POSE,
    DECORATED_POT_NECK_OUTER_CUBE, DECORATED_POT_NECK_POSE, DECORATED_POT_NECK_TOP_CUBE,
    DECORATED_POT_RIGHT_POSE, DECORATED_POT_SIDE_CUBE, DECORATED_POT_SIDE_TEXTURE_REF,
    DECORATED_POT_TOP_BOTTOM_CUBE, DECORATED_POT_TOP_POSE, MODEL_LAYER_DECORATED_POT_BASE,
    MODEL_LAYER_DECORATED_POT_SIDES,
};
use glam::{Mat4, Vec3};
use std::f32::consts::{FRAC_PI_2, PI};

fn pot_instance(position: [f32; 3], y_rot: f32) -> EntityModelInstance {
    EntityModelInstance::decorated_pot(-1, position, y_rot, None, None, None, None)
}

#[test]
fn decorated_pot_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla `DecoratedPotRenderer.createBaseLayer` (atlas 32×32): the neck's
    // `addBox(4,17,4, 8,3,8, deflate(-0.1))` shrinks to min (4.1,17.1,4.1) size (7.8,2.8,7.8)
    // while the UV box keeps 8×3×8; its `addBox(5,20,5, 6,1,6, inflate(0.2))` grows to
    // min (4.8,19.8,4.8) size (6.4,1.4,6.4) with the 6×1×6 UV box.
    assert_eq!(DECORATED_POT_NECK_OUTER_CUBE.min, [4.1, 17.1, 4.1]);
    assert_eq!(DECORATED_POT_NECK_OUTER_CUBE.size, [7.8, 2.8, 7.8]);
    assert_eq!(DECORATED_POT_NECK_OUTER_CUBE.uv_size, [8.0, 3.0, 8.0]);
    assert_eq!(DECORATED_POT_NECK_OUTER_CUBE.tex, [0.0, 0.0]);
    assert_eq!(DECORATED_POT_NECK_TOP_CUBE.min, [4.8, 19.8, 4.8]);
    assert_eq!(DECORATED_POT_NECK_TOP_CUBE.size, [6.4, 1.4, 6.4]);
    assert_eq!(DECORATED_POT_NECK_TOP_CUBE.uv_size, [6.0, 1.0, 6.0]);
    assert_eq!(DECORATED_POT_NECK_TOP_CUBE.tex, [0.0, 5.0]);
    // The neck flips upside down onto the pot: offsetAndRotation(0, 37, 16, π, 0, 0).
    assert_eq!(DECORATED_POT_NECK_POSE.offset, [0.0, 37.0, 16.0]);
    assert_eq!(DECORATED_POT_NECK_POSE.rotation, [PI, 0.0, 0.0]);
    // top/bottom share one 14×0×14 plane at texOffs(-14, 13); the negative U offset lands the
    // plane's down/up faces at u = 0..28 on the 32×32 sheet.
    assert_eq!(DECORATED_POT_TOP_BOTTOM_CUBE.min, [0.0, 0.0, 0.0]);
    assert_eq!(DECORATED_POT_TOP_BOTTOM_CUBE.size, [14.0, 0.0, 14.0]);
    assert_eq!(DECORATED_POT_TOP_BOTTOM_CUBE.tex, [-14.0, 13.0]);
    assert_eq!(DECORATED_POT_TOP_POSE.offset, [1.0, 16.0, 1.0]);
    assert_eq!(DECORATED_POT_BOTTOM_POSE.offset, [1.0, 0.0, 1.0]);
    // `createSidesLayer` (atlas 16×16): one 14×16×0 plane at texOffs(1, 0) with only the NORTH
    // face baked (`EnumSet.of(Direction.NORTH)`), posed per side.
    assert_eq!(DECORATED_POT_SIDE_CUBE.min, [0.0, 0.0, 0.0]);
    assert_eq!(DECORATED_POT_SIDE_CUBE.size, [14.0, 16.0, 0.0]);
    assert_eq!(DECORATED_POT_SIDE_CUBE.tex, [1.0, 0.0]);
    assert_eq!(DECORATED_POT_SIDE_CUBE.visible_faces, MODEL_CUBE_FACE_NORTH);
    assert_eq!(DECORATED_POT_BACK_POSE.offset, [15.0, 16.0, 1.0]);
    assert_eq!(DECORATED_POT_BACK_POSE.rotation, [0.0, 0.0, PI]);
    assert_eq!(DECORATED_POT_LEFT_POSE.offset, [1.0, 16.0, 1.0]);
    assert_eq!(DECORATED_POT_LEFT_POSE.rotation, [0.0, -FRAC_PI_2, PI]);
    assert_eq!(DECORATED_POT_RIGHT_POSE.offset, [15.0, 16.0, 15.0]);
    assert_eq!(DECORATED_POT_RIGHT_POSE.rotation, [0.0, FRAC_PI_2, PI]);
    assert_eq!(DECORATED_POT_FRONT_POSE.offset, [1.0, 16.0, 15.0]);
    assert_eq!(DECORATED_POT_FRONT_POSE.rotation, [PI, 0.0, 0.0]);
    // One tree carries all seven parts.
    let model = DecoratedPotModel::new();
    for name in ["neck", "top", "bottom", "back", "left", "right", "front"] {
        assert!(model.root().try_child(name).is_some(), "part {name}");
    }
}

#[test]
fn decorated_pot_transform_matches_vanilla_facing_rotation() {
    // Vanilla `createModelTransformation`: rotateAround(Ry(180 - toYRot), 0.5, 0.5, 0.5).
    // NORTH (toYRot 180) is the identity yaw.
    let north = decorated_pot_model_root_transform(pot_instance([2.0, 3.0, 4.0], 0.0));
    assert!((north.transform_point3(Vec3::ZERO) - Vec3::new(2.0, 3.0, 4.0)).length() < 1e-5);
    // SOUTH (toYRot 0 -> 180°): (x, y, z) -> (1-x, y, 1-z).
    let south = decorated_pot_model_root_transform(pot_instance([0.0, 0.0, 0.0], 180.0));
    assert!(
        (south.transform_point3(Vec3::new(0.2, 0.5, 0.1)) - Vec3::new(0.8, 0.5, 0.9)).length()
            < 1e-5
    );
    // WEST (toYRot 90 -> 90°): (x, y, z) -> (z, y, 1-x).
    let west = decorated_pot_model_root_transform(pot_instance([0.0, 0.0, 0.0], 90.0));
    assert!(
        (west.transform_point3(Vec3::new(0.2, 0.5, 0.1)) - Vec3::new(0.1, 0.5, 0.8)).length()
            < 1e-5
    );
    // EAST (toYRot 270 -> -90°): (x, y, z) -> (1-z, y, x).
    let east = decorated_pot_model_root_transform(pot_instance([0.0, 0.0, 0.0], -90.0));
    assert!(
        (east.transform_point3(Vec3::new(0.2, 0.5, 0.1)) - Vec3::new(0.9, 0.5, 0.2)).length()
            < 1e-5
    );
    // No entity flip: orientation-preserving.
    assert!(south.determinant() > 0.0);
}

fn rotate_around(rotation: Mat4, pivot: Vec3) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}

#[test]
fn decorated_pot_wobble_matches_vanilla_formulas() {
    // POSITIVE at progress 0.25 (`DecoratedPotRenderer.submit`): dt = 0.25·2π = π/2;
    // tiltX = -1.5·(cos(dt)+0.5)·sin(dt/2) = -1.5·0.5·sin(π/4) = -0.5303301;
    // rotate X by tiltX·0.015625 about (0.5, 0, 0.5), then Z by sin(dt)·0.015625 = 0.015625.
    let positive = decorated_pot_model_root_transform(
        pot_instance([0.0, 0.0, 0.0], 0.0).with_decorated_pot_wobble(Some(DecoratedPotWobble {
            positive: true,
            progress: 0.25,
        })),
    );
    let pivot = Vec3::new(0.5, 0.0, 0.5);
    let tilt_x = -1.5_f32 * (FRAC_PI_2.cos() + 0.5) * (FRAC_PI_2 / 2.0).sin();
    let expected_positive = rotate_around(Mat4::from_rotation_x(tilt_x * 0.015625), pivot)
        * rotate_around(Mat4::from_rotation_z(0.015625), pivot);
    assert!(
        (positive.transform_point3(Vec3::new(1.0, 1.0, 1.0))
            - expected_positive.transform_point3(Vec3::new(1.0, 1.0, 1.0)))
        .length()
            < 1e-6
    );
    // NEGATIVE at progress 0.5: turn = sin(-0.5·3π)·0.125 = 0.125 (sin(-3π/2) = 1), decayed by
    // (1 - 0.5) -> a 0.0625 rad yaw about the pot bottom centre.
    let negative = decorated_pot_model_root_transform(
        pot_instance([0.0, 0.0, 0.0], 0.0).with_decorated_pot_wobble(Some(DecoratedPotWobble {
            positive: false,
            progress: 0.5,
        })),
    );
    let expected_negative = rotate_around(Mat4::from_rotation_y(0.0625), pivot);
    assert!(
        (negative.transform_point3(Vec3::new(1.0, 0.0, 0.0))
            - expected_negative.transform_point3(Vec3::new(1.0, 0.0, 0.0)))
        .length()
            < 1e-6
    );
    // Past the duration (progress > 1) the wobble is gated off, like vanilla's
    // `wobbleProgress >= 0 && <= 1` render window.
    let expired = decorated_pot_model_root_transform(
        pot_instance([0.0, 0.0, 0.0], 0.0).with_decorated_pot_wobble(Some(DecoratedPotWobble {
            positive: false,
            progress: 1.2,
        })),
    );
    assert_eq!(
        expired,
        decorated_pot_model_root_transform(pot_instance([0.0, 0.0, 0.0], 0.0))
    );
}

#[test]
fn decorated_pot_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = EntityModelKind::DecoratedPot {
        back: Some(DecoratedPotPattern::Angler),
        left: None,
        right: None,
        front: None,
    };
    assert_eq!(kind.model_key(), "decorated_pot");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(DECORATED_POT_BASE_TEXTURE_REF)
    );
    assert_eq!(
        DECORATED_POT_BASE_TEXTURE_REF.path,
        "textures/entity/decorated_pot/decorated_pot_base.png"
    );
    assert_eq!(DECORATED_POT_BASE_TEXTURE_REF.size, [32, 32]);
    assert_eq!(
        DECORATED_POT_SIDE_TEXTURE_REF.path,
        "textures/entity/decorated_pot/decorated_pot_side.png"
    );
    assert_eq!(DECORATED_POT_SIDE_TEXTURE_REF.size, [16, 16]);
    // base + side + 23 patterns, all registered into the shared entity atlas.
    assert_eq!(decorated_pot_entity_texture_refs().len(), 25);
    for texture in decorated_pot_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}

#[test]
fn decorated_pot_layer_passes_match_vanilla_renderer() {
    // Vanilla `DecoratedPotRenderer.submit`: five `entitySolid` part submissions — the base
    // sheet for neck/top/bottom, then each side's sherd pattern (or the plain side).
    let passes = decorated_pot_textured_layer_passes(
        Some(DecoratedPotPattern::Angler),
        None,
        Some(DecoratedPotPattern::Skull),
        None,
    );
    assert_eq!(passes.len(), 5);
    let base = &passes[0];
    assert_eq!(base.kind, EntityModelLayerKind::DecoratedPotBase);
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(base.render_type.vanilla_name(), "entitySolid");
    assert_eq!(base.model_layer, MODEL_LAYER_DECORATED_POT_BASE);
    assert_eq!(base.texture, DECORATED_POT_BASE_TEXTURE_REF);
    assert_eq!(
        base.visibility,
        EntityModelLayerVisibility::RetainedParts(&["neck", "top", "bottom"])
    );
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    let side_expectations: [(&'static [&'static str], &str); 4] = [
        (
            &["back"],
            "textures/entity/decorated_pot/angler_pottery_pattern.png",
        ),
        (
            &["left"],
            "textures/entity/decorated_pot/decorated_pot_side.png",
        ),
        (
            &["right"],
            "textures/entity/decorated_pot/skull_pottery_pattern.png",
        ),
        (
            &["front"],
            "textures/entity/decorated_pot/decorated_pot_side.png",
        ),
    ];
    for (index, (retained, texture_path)) in side_expectations.iter().enumerate() {
        let pass = &passes[index + 1];
        assert_eq!(pass.kind, EntityModelLayerKind::DecoratedPotSide);
        assert_eq!(pass.render_type, EntityModelLayerRenderType::EntitySolid);
        assert_eq!(pass.model_layer, MODEL_LAYER_DECORATED_POT_SIDES);
        assert_eq!(pass.texture.path, *texture_path);
        assert_eq!(
            pass.visibility,
            EntityModelLayerVisibility::RetainedParts(retained)
        );
        assert_eq!((pass.order, pass.submit_sequence), (0, (index + 1) as u32));
    }
}

#[test]
fn decorated_pot_textured_mesh_bakes_base_and_four_sides() {
    let images: Vec<EntityModelTextureImage> = decorated_pot_entity_texture_refs()
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::decorated_pot(
        -1,
        [3.0, 4.0, 5.0],
        0.0,
        Some(DecoratedPotPattern::Angler),
        None,
        None,
        None,
    )
    .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    // Five submissions: the base sheet plus the four per-side sheets.
    assert_eq!(meshes.submissions.len(), 5);
    assert_eq!(
        meshes.submissions[0].texture,
        DECORATED_POT_BASE_TEXTURE_REF
    );
    assert_eq!(
        meshes.submissions[1].texture.path,
        "textures/entity/decorated_pot/angler_pottery_pattern.png"
    );
    for submission in &meshes.submissions[2..] {
        assert_eq!(submission.texture, DECORATED_POT_SIDE_TEXTURE_REF);
    }
    for submission in &meshes.submissions {
        assert_eq!(
            submission.render_type,
            EntityModelLayerRenderType::EntitySolid
        );
        assert_eq!(
            submission.transform,
            decorated_pot_model_root_transform(instance)
        );
        assert_eq!(submission.light, instance.render_state.shader_light());
    }
    // Base pass: neck (2 boxes) + top + bottom planes = 4 cubes × 6 faces = 24 faces; the four
    // side passes bake one NORTH face each (the vanilla `EnumSet.of(Direction.NORTH)` mask) —
    // 28 faces / 112 vertices / 168 indices, all in the backface-culled cutout bucket
    // (`entitySolid`).
    assert_eq!(meshes.cutout_cull.cutout_faces, 28);
    assert_eq!(meshes.cutout_cull.vertices.len(), 112);
    assert_eq!(meshes.cutout_cull.indices.len(), 168);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    let light = meshes.submissions[0].light;
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.light == light));
}

#[test]
fn decorated_pot_model_has_no_animation() {
    let mut model = DecoratedPotModel::new();
    let posed_before = model.root().try_child("neck").unwrap().pose;
    model.prepare(&pot_instance([0.0, 0.0, 0.0], 0.0));
    assert_eq!(model.root().try_child("neck").unwrap().pose, posed_before);
}
