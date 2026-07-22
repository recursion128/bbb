use super::*;

use crate::entity_models::colored::LegacyRandom;
use crate::entity_models::model::{EntityModel, ModelCube};

fn player_stuck_atlas() -> EntityModelTextureAtlasLayout {
    let images = [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        ARROW_TEXTURE_REF,
        BEE_STINGER_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn wide_player(entity_id: i32) -> EntityModelInstance {
    EntityModelInstance::player_with_skin(
        entity_id,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
}

#[test]
fn player_stuck_bee_stinger_model_and_texture_match_vanilla_layer() {
    assert_eq!(BEE_STINGER_TEXTURE_REF.size, [16, 16]);
    assert_eq!(
        BEE_STINGER_TEXTURE_REF.path,
        "textures/entity/bee/bee_stinger.png"
    );
    assert!(entity_model_texture_refs().contains(&BEE_STINGER_TEXTURE_REF));
    assert_eq!(
        BEE_STINGER_CROSS_CUBE,
        ModelCube::new(
            [-1.0, -0.5, 0.0],
            [2.0, 1.0, 0.0],
            [1.0; 4],
            [2.0, 1.0, 0.0],
            [0.0, 0.0],
            false,
        )
    );
    let model = BeeStingerModel::new();
    assert_eq!(
        model.root().try_descendant_attach_transform(&["cross_1"]),
        Some(Mat4::from_rotation_x(std::f32::consts::FRAC_PI_4))
    );
    assert_eq!(
        model.root().try_descendant_attach_transform(&["cross_2"]),
        Some(Mat4::from_rotation_x(3.0 * std::f32::consts::FRAC_PI_4))
    );
}

#[test]
fn player_stuck_java_random_matches_single_threaded_random_source() {
    // Fixed Java 26.1 `SingleThreadedRandomSource(3)` sample: body part, one-cube selection,
    // x/y/z fractions, then the ON_SURFACE plane selection.
    let mut random = LegacyRandom::new(3);
    assert_eq!(random.next_int_bound(6), 2);
    assert_eq!(random.next_int_bound(1), 0);
    for (actual, expected) in [
        (random.next_float(), 0.070_991_99),
        (random.next_float(), 0.805_250_6),
        (random.next_float(), 0.067_119_956),
    ] {
        assert!((actual - expected).abs() < 1.0e-7, "{actual} != {expected}");
    }
    assert_eq!(random.next_int_bound(3), 0);
}

#[test]
fn player_stuck_layers_submit_deterministic_posed_arrow_and_surface_stinger() {
    let atlas = player_stuck_atlas();
    let instance = wide_player(3)
        .with_walk_animation(2.25, 0.7)
        .with_age_in_ticks(4.5)
        .with_player_arrow_count(2)
        .with_player_stinger_count(1)
        .with_light_coords((7_u32 << 4) | (9_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_outline_color(0xff12_3456);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 4);
    let arrow_submits = meshes
        .submissions
        .iter()
        .filter(|submit| submit.texture == ARROW_TEXTURE_REF)
        .collect::<Vec<_>>();
    let stinger_submit = meshes
        .submissions
        .iter()
        .find(|submit| submit.texture == BEE_STINGER_TEXTURE_REF)
        .expect("bee stinger submission");
    assert_eq!(arrow_submits.len(), 2);
    assert_ne!(arrow_submits[0].transform, arrow_submits[1].transform);
    for submit in arrow_submits
        .iter()
        .copied()
        .chain(std::iter::once(stinger_submit))
    {
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::EntityCutoutCull
        );
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, ENTITY_VERTEX_NO_OVERLAY);
        assert_eq!(submit.outline_color, 0xff12_3456);
    }
    assert!(arrow_submits
        .iter()
        .all(|submit| (submit.order, submit.submit_sequence) == (0, 1)));
    assert_eq!(
        (stinger_submit.order, stinger_submit.submit_sequence),
        (0, 9)
    );

    let repeated = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions, repeated.submissions);
    assert_eq!(meshes.cutout_cull.vertices, repeated.cutout_cull.vertices);
}

#[test]
fn player_stuck_arrow_follows_the_selected_posed_left_arm() {
    // Entity id 3 selects bodyParts[2] = leftArm on its first Java nextInt(6).
    let atlas = player_stuck_atlas();
    let idle = wide_player(3).with_player_arrow_count(1);
    let walking = idle.with_walk_animation(2.25, 0.8).with_age_in_ticks(4.5);
    let transform = |instance| {
        entity_model_textured_meshes(&[instance], &atlas)
            .submissions
            .into_iter()
            .find(|submit| submit.texture == ARROW_TEXTURE_REF)
            .expect("stuck arrow submission")
            .transform
    };
    assert_ne!(transform(idle), transform(walking));
}

#[test]
fn player_stuck_layers_bound_untrusted_counts_and_ignore_non_positive_values() {
    let atlas = player_stuck_atlas();
    let hostile = entity_model_textured_meshes(
        &[wide_player(42)
            .with_player_arrow_count(1_000)
            .with_player_stinger_count(1_000)],
        &atlas,
    );
    assert_eq!(
        hostile
            .submissions
            .iter()
            .filter(|submit| submit.texture == ARROW_TEXTURE_REF)
            .count(),
        64
    );
    assert_eq!(
        hostile
            .submissions
            .iter()
            .filter(|submit| submit.texture == BEE_STINGER_TEXTURE_REF)
            .count(),
        64
    );

    let non_positive = entity_model_textured_meshes(
        &[wide_player(42)
            .with_player_arrow_count(-1)
            .with_player_stinger_count(0)],
        &atlas,
    );
    assert_eq!(non_positive.submissions.len(), 1);
    assert_eq!(
        non_positive.submissions[0].texture,
        PLAYER_WIDE_STEVE_TEXTURE_REF
    );
}

#[test]
fn player_stuck_layers_record_submissions_before_missing_atlas_suppresses_geometry() {
    let images = [PLAYER_WIDE_STEVE_TEXTURE_REF]
        .into_iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![0; len])
        })
        .collect::<Vec<_>>();
    let atlas = build_entity_model_texture_atlas(&images).unwrap().0;
    let instance = wide_player(5)
        .with_player_arrow_count(1)
        .with_player_stinger_count(1);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 3);
    assert!(meshes
        .submissions
        .iter()
        .any(|submit| submit.texture == ARROW_TEXTURE_REF));
    assert!(meshes
        .submissions
        .iter()
        .any(|submit| submit.texture == BEE_STINGER_TEXTURE_REF));
    assert!(meshes.cutout_cull.vertices.is_empty());
}

#[test]
fn player_stuck_layers_remain_visible_when_the_base_body_has_no_render_type() {
    let atlas = player_stuck_atlas();
    let hidden = wide_player(11)
        .with_player_arrow_count(1)
        .with_player_stinger_count(1)
        .with_invisible(true)
        .with_invisible_to_player(true);
    let meshes = entity_model_textured_meshes(&[hidden], &atlas);
    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(meshes.submissions[0].texture, ARROW_TEXTURE_REF);
    assert_eq!(meshes.submissions[1].texture, BEE_STINGER_TEXTURE_REF);
    assert!(meshes
        .submissions
        .iter()
        .all(|submit| submit.render_type == EntityModelLayerRenderType::EntityCutoutCull));
}
