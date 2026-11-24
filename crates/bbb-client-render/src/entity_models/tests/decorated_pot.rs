use super::*;

use crate::entity_models::model_layers::{
    DECORATED_POT_BASE_TEXTURE_REF, DECORATED_POT_SIDE_TEXTURE_REF,
};
use crate::entity_models::transforms::decorated_pot_model_root_transform;
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

fn pot_instance(position: [f32; 3], y_rot: f32) -> EntityModelInstance {
    EntityModelInstance::decorated_pot(-1, position, y_rot, None, None, None, None)
}

#[test]
fn decorated_pot_transform_matches_vanilla_facing_rotation() {
    // Vanilla `createModelTransformation`: rotateAround(Ry(180 - toYRot), 0.5, 0.5, 0.5).
    // NORTH (toYRot 180) is the identity yaw.
    let north = decorated_pot_model_root_transform(&pot_instance([2.0, 3.0, 4.0], 0.0));
    assert!((north.transform_point3(Vec3::ZERO) - Vec3::new(2.0, 3.0, 4.0)).length() < 1e-5);
    // SOUTH (toYRot 0 -> 180°): (x, y, z) -> (1-x, y, 1-z).
    let south = decorated_pot_model_root_transform(&pot_instance([0.0, 0.0, 0.0], 180.0));
    assert!(
        (south.transform_point3(Vec3::new(0.2, 0.5, 0.1)) - Vec3::new(0.8, 0.5, 0.9)).length()
            < 1e-5
    );
    // WEST (toYRot 90 -> 90°): (x, y, z) -> (z, y, 1-x).
    let west = decorated_pot_model_root_transform(&pot_instance([0.0, 0.0, 0.0], 90.0));
    assert!(
        (west.transform_point3(Vec3::new(0.2, 0.5, 0.1)) - Vec3::new(0.1, 0.5, 0.8)).length()
            < 1e-5
    );
    // EAST (toYRot 270 -> -90°): (x, y, z) -> (1-z, y, x).
    let east = decorated_pot_model_root_transform(&pot_instance([0.0, 0.0, 0.0], -90.0));
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
        &pot_instance([0.0, 0.0, 0.0], 0.0).with_block_entity(|s| {
            s.decorated_pot_wobble = Some(DecoratedPotWobble {
                positive: true,
                progress: 0.25,
            })
        }),
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
        &pot_instance([0.0, 0.0, 0.0], 0.0).with_block_entity(|s| {
            s.decorated_pot_wobble = Some(DecoratedPotWobble {
                positive: false,
                progress: 0.5,
            })
        }),
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
        &pot_instance([0.0, 0.0, 0.0], 0.0).with_block_entity(|s| {
            s.decorated_pot_wobble = Some(DecoratedPotWobble {
                positive: false,
                progress: 1.2,
            })
        }),
    );
    assert_eq!(
        expired,
        decorated_pot_model_root_transform(&pot_instance([0.0, 0.0, 0.0], 0.0))
    );
}

#[test]
fn decorated_pot_texture_refs_match_vanilla_selection() {
    assert_eq!(
        DECORATED_POT_BASE_TEXTURE_REF.path,
        "textures/entity/decorated_pot/decorated_pot_base.png"
    );
    assert_eq!(DECORATED_POT_BASE_TEXTURE_REF.size, [32, 32]);
    assert_eq!(
        DECORATED_POT_SIDE_TEXTURE_REF.path,
        "textures/entity/decorated_pot/decorated_pot_side.png"
    );
    // base + side + 23 patterns, all registered into the shared entity atlas.
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
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(base.render_type.vanilla_name(), "entitySolid");
    assert_eq!(base.texture, DECORATED_POT_BASE_TEXTURE_REF);
    assert_eq!(
        base.visibility,
        EntityModelLayerVisibility::RetainedParts(&["neck", "top", "bottom"])
    );
    assert_eq!(base.order, 0);
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
        assert_eq!(pass.render_type, EntityModelLayerRenderType::EntitySolid);
        assert_eq!(pass.texture.path, *texture_path);
        assert_eq!(
            pass.visibility,
            EntityModelLayerVisibility::RetainedParts(retained)
        );
        assert_eq!(pass.order, 0);
    }
}

#[test]
fn decorated_pot_textured_mesh_bakes_base_and_four_sides() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/decorated_pot/"))
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
    .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    // Five layers: the base sheet plus the four per-side sheets.

    // Base pass: neck (2 boxes) + top + bottom planes = 4 cubes × 6 faces = 24 faces; the four
    // side passes bake one NORTH face each (the vanilla `EnumSet.of(Direction.NORTH)` mask) —
    // 28 faces / 112 vertices / 168 indices, all in the backface-culled solid bucket.
    assert_eq!(meshes.solid.cutout_faces, 28);
    assert_eq!(meshes.solid.vertices.len(), 112);
    assert_eq!(meshes.solid.indices.len(), 168);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    let light = instance.render_state.shader_light();
    assert!(meshes
        .solid
        .vertices
        .iter()
        .all(|vertex| vertex.light == light));
}
