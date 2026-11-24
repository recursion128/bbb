use super::*;
use crate::entity_models::model_layers::copper_golem_model;

use crate::entity_models::textured::copper_golem_statue_block_layer_passes;
use crate::entity_models::transforms::copper_golem_statue_block_root_transform;
use crate::item_models::SpecialItemCopperGolemPose;

const RAD: f32 = std::f32::consts::PI / 180.0;

#[test]
fn copper_golem_weathering_from_vanilla_id_clamps_like_weather_state_codec() {
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(-1),
        CopperGolemWeathering::Unaffected
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(0),
        CopperGolemWeathering::Unaffected
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(1),
        CopperGolemWeathering::Exposed
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(2),
        CopperGolemWeathering::Weathered
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(3),
        CopperGolemWeathering::Oxidized
    );
    assert_eq!(
        CopperGolemWeathering::from_vanilla_id(4),
        CopperGolemWeathering::Oxidized
    );
}

#[test]
fn copper_golem_walk_keyframes_use_vanilla_apply_walk() {
    // Vanilla `CopperGolemModel.setupAnim`: empty rendered hands select
    // `CopperGolemAnimation.COPPER_GOLEM_WALK.applyWalk(pos, speed, 2, 2.5)`.

    let walking = EntityModelInstance::new(
        912,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut model = copper_golem_model(&walking);

    let body = model.child_mut("body");
    assert_close3(body.pose.rotation, degree_vec(10.0, 15.0, 0.0));
    assert_close3(
        body.child_mut("head").pose.rotation,
        [(-5.0 - 10.0) * RAD, 20.0 * RAD, 0.0],
    );
    assert_close3(
        body.child_mut("right_arm").pose.rotation,
        degree_vec(70.0, 0.0, 0.0),
    );
    assert_close3(
        body.child_mut("left_arm").pose.rotation,
        degree_vec(-80.0, 0.0, 0.0),
    );
    assert_close3(
        model.child_mut("right_leg").pose.rotation,
        degree_vec(-60.0, 0.0, 0.0),
    );
    assert_close3(
        model.child_mut("left_leg").pose.rotation,
        degree_vec(60.0, 0.0, 0.0),
    );

    let still = walking.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 0.0));
    let mut still_model = copper_golem_model(&still);
    let still_body = still_model.child_mut("body");
    assert_close3(still_body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_close3(
        still_body.child_mut("head").pose.rotation,
        [-5.0 * RAD, 20.0 * RAD, 0.0],
    );
    assert_close3(
        still_model.child_mut("right_leg").pose.rotation,
        [0.0, 0.0, 0.0],
    );
}

#[test]
fn copper_golem_walk_with_item_uses_vanilla_item_cycle_and_clamp() {
    // Vanilla switches to `COPPER_GOLEM_WALK_ITEM` as soon as either rendered hand is non-empty,
    // then runs `poseHeldItemArmsIfStill` to clamp both arms into the carried-item rest pose.

    let instance = EntityModelInstance::new(
        913,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| s.kind.copper_golem_mut().holding_item = true)
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.rotation, degree_vec(10.0, 7.5, 0.0));
    assert_close3(
        body.child_mut("head").pose.rotation,
        [(-5.0 - 10.0) * RAD, 20.0 * RAD, 0.0],
    );
    assert_close3(
        body.child_mut("right_arm").pose.rotation,
        [
            -59.78638 * RAD,
            (-6.49053 * RAD).min(-0.1134464),
            (-3.76613 * RAD).min(-0.064577185),
        ],
    );
    let left_arm = body.child_mut("left_arm");
    assert_close3(
        left_arm.pose.rotation,
        [
            -59.78638 * RAD,
            (6.49053 * RAD).max(0.1134464),
            (3.76613 * RAD).max(0.064577185),
        ],
    );
    assert_close3(left_arm.pose.offset, [3.78871, -5.9788, -0.07004]);
    assert_close3(
        model.child_mut("right_leg").pose.rotation,
        degree_vec(-30.0, 0.0, 0.0),
    );
    assert_close3(
        model.child_mut("left_leg").pose.rotation,
        degree_vec(30.0, 0.0, 0.0),
    );
}

#[test]
fn copper_golem_idle_keyframes_use_vanilla_head_spin() {
    // Vanilla `CopperGolemModel.setupAnim` applies `COPPER_GOLEM_IDLE` after the walk branch.

    let instance = EntityModelInstance::new(
        915,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0))
    .with(|s| s.kind.copper_golem_mut().idle_seconds = 1.75);
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.rotation, degree_vec(0.0, 35.0, 0.0));
    assert_close3(
        body.child_mut("head").pose.rotation,
        [(-5.0 - 25.0) * RAD, (20.0 + 300.0) * RAD, 0.0],
    );

    let stopped = instance.with(|s| s.kind.copper_golem_mut().idle_seconds = -1.0);
    let mut stopped_model = copper_golem_model(&stopped);
    let stopped_body = stopped_model.child_mut("body");
    assert_close3(stopped_body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_close3(
        stopped_body.child_mut("head").pose.rotation,
        [-5.0 * RAD, 20.0 * RAD, 0.0],
    );
}

#[test]
fn copper_golem_get_item_keyframes_use_vanilla_chest_interaction() {
    // Vanilla `CopperGolemModel.setupAnim` applies `COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET`
    // after idle while `CopperGolemState.GETTING_ITEM` is active.

    let instance = EntityModelInstance::new(
        917,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0))
    .with(|s| s.kind.copper_golem_mut().get_item_seconds = 0.4167);
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.offset, [0.0, -5.5, 0.0]);
    assert_close3(body.pose.rotation, degree_vec(12.5, 0.0, 0.0));
    assert_close3(
        body.child_mut("head").pose.rotation,
        [-5.0 * RAD, 20.0 * RAD, 0.0],
    );
    assert_close3(
        body.child_mut("right_arm").pose.rotation,
        degree_vec(-34.55418, 11.73507, 36.8361),
    );
    assert_close3(
        body.child_mut("left_arm").pose.rotation,
        degree_vec(-93.27, -1.79, -1.15),
    );
    assert_close3(
        model.child_mut("right_leg").pose.rotation,
        degree_vec(7.5, 0.0, 0.0),
    );
    assert_close3(
        model.child_mut("left_leg").pose.rotation,
        degree_vec(-10.0, 0.0, 0.0),
    );

    let stopped = instance.with(|s| s.kind.copper_golem_mut().get_item_seconds = -1.0);
    let mut stopped_model = copper_golem_model(&stopped);
    let stopped_body = stopped_model.child_mut("body");
    assert_close3(stopped_body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(stopped_body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_close3(
        stopped_body.child_mut("right_arm").pose.rotation,
        [0.0, 0.0, 0.0],
    );
}

#[test]
fn copper_golem_get_no_item_keyframes_use_vanilla_chest_interaction() {
    // Vanilla `CopperGolemModel.setupAnim` applies `COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET`
    // after the get-item interaction while `CopperGolemState.GETTING_NO_ITEM` is active.

    let instance = EntityModelInstance::new(
        919,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0))
    .with(|s| s.kind.copper_golem_mut().get_no_item_seconds = 2.5417);
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(body.pose.rotation, [0.0, 0.0, 0.0]);
    assert_close3(
        body.child_mut("head").pose.rotation,
        [-5.0 * RAD, 20.0 * RAD, 0.0],
    );
    let right_arm = body.child_mut("right_arm");
    assert_close3(right_arm.pose.offset, [-4.46, -5.12, -0.3]);
    assert_close3(right_arm.pose.rotation, degree_vec(3.9, -4.38, 3.36));
    let left_arm = body.child_mut("left_arm");
    assert_close3(left_arm.pose.offset, [4.03, -4.72, -0.07]);
    assert_close3(left_arm.pose.rotation, degree_vec(2.41, -0.65, -5.01));

    let stopped = instance.with(|s| s.kind.copper_golem_mut().get_no_item_seconds = -1.0);
    let mut stopped_model = copper_golem_model(&stopped);
    let stopped_body = stopped_model.child_mut("body");
    assert_close3(stopped_body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(
        stopped_body.child_mut("left_arm").pose.offset,
        [4.0, -6.0, 0.0],
    );
}

#[test]
fn copper_golem_drop_item_keyframes_use_vanilla_chest_interaction() {
    // Vanilla `CopperGolemModel.setupAnim` applies `COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP`
    // after the get/no-get interactions while `CopperGolemState.DROPPING_ITEM` is active.

    let instance = EntityModelInstance::new(
        921,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0))
    .with(|s| s.kind.copper_golem_mut().drop_item_seconds = 2.4583);
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.offset, [0.0, -5.0, -1.0]);
    assert_close3(body.pose.rotation, degree_vec(-10.0, 0.0, 0.0));
    assert_close3(
        body.child_mut("head").pose.rotation,
        [(-5.0 - 10.0) * RAD, 20.0 * RAD, 0.0],
    );
    let right_arm = body.child_mut("right_arm");
    assert_close3(right_arm.pose.offset, [-4.51, -5.62, 0.8]);
    assert_close3(right_arm.pose.rotation, degree_vec(-113.76, 21.39, 15.48));
    let left_arm = body.child_mut("left_arm");
    assert_close3(left_arm.pose.offset, [4.0512, -5.23824, 3.12882]);
    assert_close3(left_arm.pose.rotation, degree_vec(-88.59, -17.1, 11.77));
    let right_leg = model.child_mut("right_leg");
    assert_close3(right_leg.pose.offset, [0.0, -5.09, -0.11]);
    assert_close3(right_leg.pose.rotation, degree_vec(5.0, 0.0, 0.0));

    let stopped = instance.with(|s| s.kind.copper_golem_mut().drop_item_seconds = -1.0);
    let mut stopped_model = copper_golem_model(&stopped);
    let stopped_body = stopped_model.child_mut("body");
    assert_close3(stopped_body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(stopped_body.pose.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn copper_golem_drop_no_item_keyframes_use_vanilla_chest_interaction() {
    // Vanilla `CopperGolemModel.setupAnim` applies `COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP`
    // after the item-drop interaction while `CopperGolemState.DROPPING_NO_ITEM` is active.

    let instance = EntityModelInstance::new(
        923,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -5.0))
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0))
    .with(|s| s.kind.copper_golem_mut().drop_no_item_seconds = 2.3333);
    let mut model = copper_golem_model(&instance);

    let body = model.child_mut("body");
    assert_close3(body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(body.pose.rotation, degree_vec(-2.5, 0.0, 0.0));
    let head = body.child_mut("head");
    assert_close3(head.pose.offset, [0.0, -5.99, -0.03]);
    assert_close3(head.pose.rotation, [(-5.0 + 2.5) * RAD, 20.0 * RAD, 0.0]);
    let right_arm = body.child_mut("right_arm");
    assert_close3(right_arm.pose.offset, [-4.46, -6.1159, -0.30086]);
    assert_close3(
        right_arm.pose.rotation,
        degree_vec(3.50393, -4.70737, 8.3608),
    );
    let left_arm = body.child_mut("left_arm");
    assert_close3(left_arm.pose.offset, [4.03, -5.71771, -0.07133]);
    assert_close3(
        left_arm.pose.rotation,
        degree_vec(2.47864, -0.32621, -12.50706),
    );
    assert_close3(model.child_mut("left_leg").scale, [1.0, 1.0, 1.0]);

    let stopped = instance.with(|s| s.kind.copper_golem_mut().drop_no_item_seconds = -1.0);
    let mut stopped_model = copper_golem_model(&stopped);
    let stopped_body = stopped_model.child_mut("body");
    assert_close3(stopped_body.pose.offset, [0.0, -5.0, 0.0]);
    assert_close3(stopped_body.pose.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn copper_golem_textured_layer_passes_match_vanilla_renderer() {
    let passes = copper_golem_textured_layer_passes(CopperGolemWeathering::Weathered);

    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, COPPER_GOLEM_WEATHERED_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].order, 0);

    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].texture, COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF);
    assert_eq!(passes[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[1].order, 1);
}

#[test]
fn copper_golem_textured_walk_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        914,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.35;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 0.0));
    let walking = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        walking_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        walking_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, walking_meshes.cutout.vertices,
        "walk keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, walking_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_idle_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        916,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Exposed,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (7_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.45;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| s.kind.copper_golem_mut().idle_seconds = -1.0);
    let idle = base.with(|s| s.kind.copper_golem_mut().idle_seconds = 1.75);

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let idle_meshes = entity_model_textured_meshes(&[idle], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        idle_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        idle_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, idle_meshes.cutout.vertices,
        "idle keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, idle_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_get_item_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        918,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Oxidized,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.25;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| s.kind.copper_golem_mut().get_item_seconds = -1.0);
    let getting_item = base.with(|s| s.kind.copper_golem_mut().get_item_seconds = 0.4167);

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let get_meshes = entity_model_textured_meshes(&[getting_item], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        get_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        get_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, get_meshes.cutout.vertices,
        "GETTING_ITEM keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, get_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_get_no_item_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        920,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (3_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.2;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| s.kind.copper_golem_mut().get_no_item_seconds = -1.0);
    let getting_no_item = base.with(|s| s.kind.copper_golem_mut().get_no_item_seconds = 2.5417);

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let get_meshes = entity_model_textured_meshes(&[getting_no_item], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        get_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        get_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, get_meshes.cutout.vertices,
        "GETTING_NO_ITEM keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, get_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_drop_item_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        922,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Exposed,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.3;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| s.kind.copper_golem_mut().drop_item_seconds = -1.0);
    let dropping_item = base.with(|s| s.kind.copper_golem_mut().drop_item_seconds = 2.4583);

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let drop_meshes = entity_model_textured_meshes(&[dropping_item], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        drop_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        drop_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, drop_meshes.cutout.vertices,
        "DROPPING_ITEM keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, drop_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_drop_no_item_preserves_submission_metadata() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::new(
        924,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.35;
        s.has_red_overlay = true;
    });
    let still = base.with(|s| s.kind.copper_golem_mut().drop_no_item_seconds = -1.0);
    let dropping_no_item = base.with(|s| s.kind.copper_golem_mut().drop_no_item_seconds = 2.3333);

    let still_meshes = entity_model_textured_meshes(&[still], &atlas);
    let drop_meshes = entity_model_textured_meshes(&[dropping_no_item], &atlas);

    assert_eq!(
        still_meshes.cutout.indices.len(),
        drop_meshes.cutout.indices.len()
    );
    assert_eq!(
        still_meshes.eyes.indices.len(),
        drop_meshes.eyes.indices.len()
    );
    assert_ne!(
        still_meshes.cutout.vertices, drop_meshes.cutout.vertices,
        "DROPPING_NO_ITEM keyframes move the base layer without changing geometry counts"
    );
    assert_ne!(
        still_meshes.eyes.vertices, drop_meshes.eyes.vertices,
        "the emissive eyes layer uses the same posed tree as the base layer"
    );
}

#[test]
fn copper_golem_textured_mesh_uses_living_emissive_vertex_state() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::new(
        910,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());

    assert!(!meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.eyes.vertices.len(), meshes.cutout.vertices.len());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]));
}

#[test]
fn copper_golem_eyes_missing_texture_preserves_other_geometry() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/copper_golem/"))
            .iter()
            .enumerate()
            .filter_map(|(index, texture)| {
                if *texture == COPPER_GOLEM_EYES_WEATHERED_TEXTURE_REF {
                    return None;
                }
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                Some(EntityModelTextureImage::new(
                    *texture,
                    vec![index as u8; len],
                ))
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::new(
        911,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Weathered,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (2_u32 << 4) | (14_u32 << 20);
        s.white_overlay_progress = 0.7;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.eyes.indices.is_empty());
}

#[test]
fn copper_golem_statue_block_passes_drop_the_emissive_eyes_layer() {
    let passes = copper_golem_statue_block_layer_passes(CopperGolemWeathering::Weathered);
    // `CopperGolemStatueModel` is a plain `RenderTypes::entityCutout` model:
    // one pass, no `LivingEntityEmissiveLayer`.
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, COPPER_GOLEM_WEATHERED_TEXTURE_REF);
}

#[test]
fn copper_golem_statue_block_root_transform_faces_away_from_the_block_facing() {
    let instance = EntityModelInstance::copper_golem_statue_block(
        -1,
        [4.0, 5.0, 6.0],
        // `Direction.NORTH.toYRot()`; the renderer rotates by the opposite.
        180.0,
        SpecialItemCopperGolemPose::Standing,
        CopperGolemWeathering::Unaffected,
    );

    let transform = copper_golem_statue_block_root_transform(&instance);
    // `translate(0.5, 0, 0.5)` inside the block, then no net rotation for a
    // north-facing statue (`-(180 + 180)` degrees).
    let origin = transform.transform_point3(glam::Vec3::ZERO);
    assert_close3(origin.to_array(), [4.5, 5.0, 6.5]);
    let forward = transform.transform_vector3(glam::Vec3::Z);
    assert_close3(forward.to_array(), [0.0, 0.0, 1.0]);

    // Facing east (`toYRot() = 270`) turns the model a quarter turn.
    let east = EntityModelInstance::copper_golem_statue_block(
        -1,
        [0.0; 3],
        270.0,
        SpecialItemCopperGolemPose::Standing,
        CopperGolemWeathering::Unaffected,
    );
    let east_forward =
        copper_golem_statue_block_root_transform(&east).transform_vector3(glam::Vec3::Z);
    assert_close3(east_forward.to_array(), [-1.0, 0.0, 0.0]);
}
