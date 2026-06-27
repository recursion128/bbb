use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn iron_golem_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. The body's chest plate keeps the base 9×5×6 UV box against its
    // 10×6×7 geometry.
    assert_eq!(
        IRON_GOLEM_HEAD,
        [
            ModelCube::new(
                [-4.0, -12.0, -5.5],
                [8.0, 10.0, 8.0],
                IRON_GOLEM_STONE,
                [8.0, 10.0, 8.0],
                [0.0, 0.0],
                false,
            ),
            ModelCube::new(
                [-1.0, -5.0, -7.5],
                [2.0, 4.0, 2.0],
                IRON_GOLEM_STONE,
                [2.0, 4.0, 2.0],
                [24.0, 0.0],
                false,
            ),
        ]
    );
    assert_eq!(IRON_GOLEM_BODY[0].size, [18.0, 12.0, 11.0]);
    assert_eq!(IRON_GOLEM_BODY[1].size, [10.0, 6.0, 7.0]);
    assert_eq!(IRON_GOLEM_BODY[1].uv_size, [9.0, 5.0, 6.0]);
    assert_eq!(IRON_GOLEM_RIGHT_ARM[0].min, [-13.0, -2.5, -3.0]);
    assert_eq!(IRON_GOLEM_LEFT_ARM[0].min, [9.0, -2.5, -3.0]);
    assert_eq!(IRON_GOLEM_RIGHT_LEG[0].size, [6.0, 16.0, 5.0]);
    assert!(!IRON_GOLEM_RIGHT_LEG[0].mirror);
    // The two legs share geometry but the left leg mirrors and draws from a different UV slot.
    assert_eq!(IRON_GOLEM_LEFT_LEG[0].size, IRON_GOLEM_RIGHT_LEG[0].size);
    assert!(IRON_GOLEM_LEFT_LEG[0].mirror);

    // Part poses (vanilla `IronGolemModel.createBodyLayer`): the arms share the body pose, the legs
    // pivot at the hips.
    assert_eq!(IRON_GOLEM_HEAD_POSE.offset, [0.0, -7.0, -2.0]);
    assert_eq!(IRON_GOLEM_BODY_POSE.offset, [0.0, -7.0, 0.0]);
    assert_eq!(IRON_GOLEM_ARM_POSE.offset, [0.0, -7.0, 0.0]);
    assert_eq!(IRON_GOLEM_RIGHT_LEG_POSE.offset, [-4.0, 11.0, 0.0]);
    assert_eq!(IRON_GOLEM_LEFT_LEG_POSE.offset, [5.0, 11.0, 0.0]);
}

#[test]
fn iron_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.8125, 64.001, -0.3125]);
    assert_close3(max, [0.8125, 66.6885, 0.59375]);
}

#[test]
fn iron_golem_texture_ref_matches_vanilla_renderer() {
    let golem = EntityModelKind::IronGolem {
        crackiness: IronGolemCrackiness::None,
    };
    assert_eq!(golem.model_key(), "iron_golem");
    assert_eq!(
        golem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/iron_golem/iron_golem.png",
            size: [128, 128],
        })
    );
    // The base texture (and the model_key) is the same for every crackiness tier; the cracks are a
    // separate overlay layer.
    assert_eq!(
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::High,
        }
        .vanilla_texture_ref(),
        golem.vanilla_texture_ref()
    );
}

#[test]
fn iron_golem_crackiness_from_health_fraction_matches_vanilla() {
    // Vanilla `Crackiness.GOLEM` = (0.75, 0.5, 0.25): below 0.25 high, below 0.5 medium, below 0.75
    // low, otherwise none. Full health (100/100 = 1.0) is uncracked.
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(1.0),
        IronGolemCrackiness::None
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.75),
        IronGolemCrackiness::None
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.74),
        IronGolemCrackiness::Low
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.5),
        IronGolemCrackiness::Low
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.49),
        IronGolemCrackiness::Medium
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.25),
        IronGolemCrackiness::Medium
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.24),
        IronGolemCrackiness::High
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.0),
        IronGolemCrackiness::High
    );
}

#[test]
fn iron_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    // An uncracked golem is a single base pass.
    let passes = iron_golem_textured_layer_passes(IronGolemCrackiness::None);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::IronGolemBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_IRON_GOLEM);
    assert_eq!(passes[0].texture, IRON_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));

    // A cracked golem appends the matching crack overlay (Cutout, white, same model layer) — vanilla
    // `IronGolemCrackinessLayer.renderColoredCutoutModel`.
    for (crackiness, texture) in [
        (
            IronGolemCrackiness::Low,
            IRON_GOLEM_CRACKINESS_LOW_TEXTURE_REF,
        ),
        (
            IronGolemCrackiness::Medium,
            IRON_GOLEM_CRACKINESS_MEDIUM_TEXTURE_REF,
        ),
        (
            IronGolemCrackiness::High,
            IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF,
        ),
    ] {
        let passes = iron_golem_textured_layer_passes(crackiness);
        assert_eq!(passes.len(), 2);
        assert_eq!(passes[1].kind, EntityModelLayerKind::IronGolemCrackiness);
        assert_eq!(
            passes[1].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[1].model_layer, MODEL_LAYER_IRON_GOLEM);
        assert_eq!(passes[1].texture, texture);
        assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((passes[1].order, passes[1].submit_sequence), (1, 1));
    }
}

#[test]
fn iron_golem_renderer_body_wobble_matches_vanilla_setup_rotations() {
    // Vanilla `IronGolemRenderer.setupRotations` appends the renderer-level Z wobble after
    // `super.setupRotations` and before the model flip. At walk pos 0:
    //   triangleWave(0 + 6, 13) = (|6 - 6.5| - 3.25) / 3.25 = -11/13.
    let position = [1.0, 64.0, -2.0];
    let base = EntityModelInstance::iron_golem(75, position, 0.0);

    let still = base.with_walk_animation(3.0, 0.009);
    assert_eq!(
        iron_golem_model_root_transform(still),
        entity_model_root_transform(still),
        "speed below 0.01 skips the renderer wobble"
    );

    let walking = base.with_walk_animation(0.0, 1.0);
    let triangle_wave = ((6.0_f32 % 13.0 - 6.5).abs() - 3.25) / 3.25;
    let expected = Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_rotation_z((6.5 * triangle_wave).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    let actual = iron_golem_model_root_transform(walking);

    assert_ne!(
        actual,
        entity_model_root_transform(walking),
        "walking golems use a renderer root distinct from generic living entities"
    );
    assert_close_transform(actual, expected);
}

#[test]
fn iron_golem_textured_submissions_apply_body_wobble_to_base_and_cracks() {
    let (atlas, _) = build_entity_model_texture_atlas(&iron_golem_submission_texture_images())
        .expect("iron golem submission atlas");
    let walking = EntityModelInstance::new(
        76,
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::High,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20))
    .with_white_overlay_progress(0.8)
    .with_has_red_overlay(true)
    .with_walk_animation(0.0, 1.0);

    let meshes = entity_model_textured_meshes(&[walking], &atlas);
    assert_golem_submissions_match_vanilla(&meshes, walking);
    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(meshes.cutout.vertices.len(), 384);

    let expected_transform = iron_golem_model_root_transform(walking);
    assert_ne!(
        expected_transform,
        entity_model_root_transform(walking),
        "the submission transform includes the renderer body wobble"
    );

    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, IRON_GOLEM_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.transform, expected_transform);
    assert_eq!(base.light, walking.render_state.shader_light());
    assert_eq!(base.overlay, walking.render_state.overlay_coords());
    assert_ne!(base.overlay, [0.0, 10.0]);

    let cracks = meshes.submissions[1];
    assert_eq!(cracks.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(cracks.render_type.vanilla_name(), "entityCutout");
    assert_eq!(cracks.texture, IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF);
    assert_eq!(cracks.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((cracks.order, cracks.submit_sequence), (1, 1));
    assert_eq!(cracks.transform, expected_transform);
    assert_eq!(cracks.light, walking.render_state.shader_light());
    assert_eq!(
        cracks.overlay,
        [0.0, walking.render_state.overlay_coords()[1]]
    );
    assert_ne!(cracks.overlay, base.overlay);
    assert_ne!(cracks.overlay, [0.0, 10.0]);

    assert!(meshes.cutout.vertices[..192]
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));
    assert!(meshes.cutout.vertices[192..]
        .iter()
        .all(|vertex| vertex.light == cracks.light && vertex.overlay == cracks.overlay));
}

#[test]
fn iron_golem_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_IRON_GOLEM, "minecraft:iron_golem#main");
    assert_eq!(IRON_GOLEM_TEXTURE_REF.size, [128, 128]);
    // The unified cubes carry the textured UV sources (`uv_size`/`texOffs`/`mirror`) merged into the
    // colored geometry.
    assert_eq!(IRON_GOLEM_HEAD[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(IRON_GOLEM_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(IRON_GOLEM_BODY[1].uv_size, [9.0, 5.0, 6.0]);
    assert_eq!(IRON_GOLEM_BODY[1].tex, [0.0, 70.0]);
    assert_eq!(IRON_GOLEM_LEFT_LEG[0].uv_size, [6.0, 16.0, 5.0]);
    assert_eq!(IRON_GOLEM_LEFT_LEG[0].tex, [60.0, 0.0]);
    assert!(IRON_GOLEM_LEFT_LEG[0].mirror);
    assert_eq!(IRON_GOLEM_RIGHT_ARM[0].tex, [60.0, 21.0]);
    assert_eq!(IRON_GOLEM_LEFT_ARM[0].tex, [60.0, 58.0]);
}

#[test]
fn snow_golem_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. Each snow ball / stick arm keeps a base UV box one unit larger than
    // its colored geometry.
    assert_eq!(
        SNOW_GOLEM_HEAD[0],
        ModelCube::new(
            [-3.5, -7.5, -3.5],
            [7.0, 7.0, 7.0],
            SNOW_GOLEM_WHITE,
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(SNOW_GOLEM_ARM[0].size, [11.0, 1.0, 1.0]);
    assert_eq!(SNOW_GOLEM_ARM[0].uv_size, [12.0, 2.0, 2.0]);
    assert_eq!(SNOW_GOLEM_UPPER_BODY[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(SNOW_GOLEM_LOWER_BODY[0].size, [11.0, 11.0, 11.0]);

    // Part poses (vanilla `SnowGolemModel.createBodyLayer`): head, left arm, right arm (the two stick
    // arms droop ±1 rad, the right arm yawed π), upper body, lower body.
    assert_eq!(SNOW_GOLEM_HEAD_POSE.offset, [0.0, 4.0, 0.0]);
    assert_eq!(SNOW_GOLEM_LEFT_ARM_POSE.offset, [5.0, 6.0, 1.0]);
    assert_eq!(SNOW_GOLEM_LEFT_ARM_POSE.rotation, [0.0, 0.0, 1.0]);
    assert_eq!(SNOW_GOLEM_RIGHT_ARM_POSE.offset, [-5.0, 6.0, -1.0]);
    assert_eq!(
        SNOW_GOLEM_RIGHT_ARM_POSE.rotation,
        [0.0, std::f32::consts::PI, -1.0]
    );
    assert_eq!(SNOW_GOLEM_UPPER_BODY_POSE.offset, [0.0, 13.0, 0.0]);
    assert_eq!(SNOW_GOLEM_LOWER_BODY_POSE.offset, [0.0, 24.0, 0.0]);
}

#[test]
fn snow_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(
        EntityModelKind::SnowGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/snow_golem/snow_golem.png",
            size: [64, 64],
        })
    );
}

#[test]
fn snow_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = snow_golem_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::SnowGolemBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_SNOW_GOLEM);
    assert_eq!(passes[0].texture, SNOW_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
}

#[test]
fn snow_golem_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SNOW_GOLEM, "minecraft:snow_golem#main");
    assert_eq!(SNOW_GOLEM_TEXTURE_REF.size, [64, 64]);
    // The unified cubes carry the textured UV sources (`uv_size`/`texOffs`/`mirror`) merged into the
    // colored geometry; each snow ball / arm keeps a base UV box one unit larger.
    assert_eq!(SNOW_GOLEM_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(SNOW_GOLEM_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(SNOW_GOLEM_ARM[0].uv_size, [12.0, 2.0, 2.0]);
    assert_eq!(SNOW_GOLEM_ARM[0].tex, [32.0, 0.0]);
    assert_eq!(SNOW_GOLEM_UPPER_BODY[0].uv_size, [10.0, 10.0, 10.0]);
    assert_eq!(SNOW_GOLEM_UPPER_BODY[0].tex, [0.0, 16.0]);
    assert_eq!(SNOW_GOLEM_LOWER_BODY[0].uv_size, [12.0, 12.0, 12.0]);
    assert_eq!(SNOW_GOLEM_LOWER_BODY[0].tex, [0.0, 36.0]);
}

#[test]
fn golem_texture_atlas_stitches_official_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/iron_golem/iron_golem.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 128.0 / 192.0]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/snow_golem/snow_golem.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 128.0 / 192.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let snow_start = rgba_offset(layout.width, 128, 0, "snow golem atlas row").unwrap();
    assert_eq!(&rgba[snow_start..snow_start + 4], &[1; 4]);
}

#[test]
fn golem_textured_meshes_use_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    let iron_instance = EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0);
    let iron_meshes = entity_model_textured_meshes(&[iron_instance], &atlas);
    assert_golem_submissions_match_vanilla(&iron_meshes, iron_instance);
    let iron = &iron_meshes.cutout;
    assert_eq!(iron.cutout_faces, 48);
    assert_eq!(iron.vertices.len(), 192);
    assert_eq!(iron.indices.len(), 288);
    assert_close2(iron.vertices[0].uv, [16.0 / 128.0, 0.0]);
    assert!(iron
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (iron_min, iron_max) = textured_mesh_extents(iron);
    assert_close3(iron_min, [-0.8125, 64.001, -0.3125]);
    assert_close3(iron_max, [0.8125, 66.6885, 0.59375]);

    let snow_instance = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0);
    let snow_meshes = entity_model_textured_meshes(&[snow_instance], &atlas);
    assert_golem_submissions_match_vanilla(&snow_meshes, snow_instance);
    let snow = &snow_meshes.cutout;
    assert_eq!(snow.cutout_faces, 30);
    assert_eq!(snow.vertices.len(), 120);
    assert_eq!(snow.indices.len(), 180);
    assert_close2(snow.vertices[0].uv, [16.0 / 128.0, 128.0 / 192.0]);
    assert!(snow
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (snow_min, snow_max) = textured_mesh_extents(snow);
    assert_close3(snow_min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(snow_max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_twists_upper_body_and_orbits_arms_with_head_yaw() {
    // Vanilla SnowGolemModel.setupAnim twists the middle snow ball by a quarter of the
    // head yaw and orbits the two stick arms around that twist. Turning the head changes
    // the head (verts 0..24), the arms (24..72), and the upper body (72..96) but leaves
    // the lower body (96..120) fixed. The colored mesh emits the five parts in body-layer
    // order, one cube each.
    let base = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(60.0, 0.0)]);
    assert_eq!(rest.vertices.len(), 120);
    assert_eq!(rest.vertices.len(), yawed.vertices.len());

    assert_ne!(rest.vertices[0..24], yawed.vertices[0..24], "head turns");
    assert_ne!(rest.vertices[24..72], yawed.vertices[24..72], "arms orbit");
    assert_ne!(
        rest.vertices[72..96],
        yawed.vertices[72..96],
        "upper body twists"
    );
    assert_eq!(
        rest.vertices[96..120],
        yawed.vertices[96..120],
        "lower body stays put"
    );

    // A pure pitch (no yaw) twists neither the body nor the arms: only the head pitches.
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);
    assert_ne!(
        rest.vertices[0..24],
        pitched.vertices[0..24],
        "head pitches"
    );
    assert_eq!(
        rest.vertices[24..96],
        pitched.vertices[24..96],
        "no yaw means no body twist or arm orbit"
    );
}

#[test]
fn snow_golem_textured_mesh_twists_upper_body_and_orbits_arms() {
    // The real (texture-backed) snow golem render path applies the same twist and orbit.
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    let base = EntityModelInstance::snow_golem(123, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with_head_look(60.0, 0.0);
    let rest = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    assert_golem_submissions_match_vanilla(&rest, base);
    assert_golem_submissions_match_vanilla(&yawed, yawed_instance);
    assert_eq!(rest.cutout.vertices.len(), 120);
    assert_eq!(rest.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(
        rest.cutout.vertices[24..72],
        yawed.cutout.vertices[24..72],
        "arms orbit"
    );
    assert_ne!(
        rest.cutout.vertices[72..96],
        yawed.cutout.vertices[72..96],
        "upper body twists"
    );
    assert_eq!(
        rest.cutout.vertices[96..120],
        yawed.cutout.vertices[96..120],
        "lower body stays put"
    );
}

#[test]
fn snow_golem_arm_pose_matches_vanilla_orbit_formula() {
    // Vanilla SnowGolemModel.setupAnim: upperBody.yRot = headYaw * π/180 * 0.25; then
    //   leftArm.yRot = upperBodyYRot;  leftArm.x = cos(upperBodyYRot)*5;  leftArm.z =
    //     -sin(upperBodyYRot)*5;
    //   rightArm.yRot = upperBodyYRot + π;  rightArm.x = -cos*5;  rightArm.z = sin*5.
    // The arm y offset and the drooping zRot (±1.0) are preserved; x/z are overwritten
    // even at rest, so a forward-facing snow golem pulls both arms to z = 0.
    let left_base = SNOW_GOLEM_LEFT_ARM_POSE;
    let right_base = SNOW_GOLEM_RIGHT_ARM_POSE;

    // Rest (yaw 0): the orbit collapses the arms to z = 0 but keeps their droop.
    assert!((snow_golem_upper_body_yrot(0.0)).abs() < 1e-6);
    let left_rest = snow_golem_arm_pose(left_base, 0.0, false);
    let right_rest = snow_golem_arm_pose(right_base, 0.0, true);
    assert_close3(left_rest.offset, [5.0, 6.0, 0.0]);
    assert_close3(right_rest.offset, [-5.0, 6.0, 0.0]);
    assert_eq!(left_rest.rotation[1], 0.0);
    assert!((right_rest.rotation[1] - std::f32::consts::PI).abs() < 1e-6);
    assert_eq!(left_rest.rotation[2], left_base.rotation[2]); // 1.0 droop preserved
    assert_eq!(right_rest.rotation[2], right_base.rotation[2]); // -1.0 droop preserved

    // A general head yaw: upper body twists by a quarter, arms orbit by cos/sin.
    let head_yaw_deg = 80.0_f32;
    let upper = snow_golem_upper_body_yrot(head_yaw_deg);
    assert!((upper - head_yaw_deg.to_radians() * 0.25).abs() < 1e-6);
    let (sin, cos) = upper.sin_cos();
    let left = snow_golem_arm_pose(left_base, upper, false);
    let right = snow_golem_arm_pose(right_base, upper, true);
    assert_close3(left.offset, [cos * 5.0, 6.0, -sin * 5.0]);
    assert!((left.rotation[1] - upper).abs() < 1e-6);
    assert_close3(right.offset, [-cos * 5.0, 6.0, sin * 5.0]);
    assert!((right.rotation[1] - (upper + std::f32::consts::PI)).abs() < 1e-6);

    // The upper-body twist sets only yRot; offset, xRot, zRot are preserved.
    let upper_base = SNOW_GOLEM_UPPER_BODY_POSE;
    let twisted = snow_golem_upper_body_pose(upper_base, upper);
    assert_eq!(twisted.offset, upper_base.offset);
    assert_eq!(twisted.rotation[0], upper_base.rotation[0]);
    assert!((twisted.rotation[1] - upper).abs() < 1e-6);
    assert_eq!(twisted.rotation[2], upper_base.rotation[2]);
}

#[test]
fn golem_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    for base in [
        EntityModelInstance::iron_golem(71, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::snow_golem(122, [0.0, 64.0, 0.0], 0.0),
    ] {
        let yawed_instance = base.with_head_look(45.0, 0.0);
        let pitched_instance = base.with_head_look(0.0, -20.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_golem_submissions_match_vanilla(&resting, base);
        assert_golem_submissions_match_vanilla(&yawed, yawed_instance);
        assert_golem_submissions_match_vanilla(&pitched, pitched_instance);
        assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
        assert_ne!(
            resting.cutout.vertices, yawed.cutout.vertices,
            "{:?}",
            base.kind
        );
        assert_ne!(
            yawed.cutout.vertices, pitched.cutout.vertices,
            "{:?}",
            base.kind
        );
    }
}

#[test]
fn iron_golem_walk_pose_matches_vanilla_triangle_wave() {
    // Vanilla IronGolemModel.setupAnim drives the limbs by Mth.triangleWave(pos, 13).
    // At pos = 0: triangleWave = (|0 - 6.5| - 3.25) / 3.25 = 1.0. With speed = 1:
    // rightLeg = -1.5, leftLeg = 1.5, rightArm = (-0.2 + 1.5) = 1.3, leftArm =
    // (-0.2 - 1.5) = -1.7.
    let right_leg = iron_golem_walk_pose(
        IRON_GOLEM_RIGHT_LEG_POSE,
        0.0,
        1.0,
        IronGolemWalkPart::RightLeg,
    );
    let left_leg = iron_golem_walk_pose(
        IRON_GOLEM_LEFT_LEG_POSE,
        0.0,
        1.0,
        IronGolemWalkPart::LeftLeg,
    );
    let right_arm =
        iron_golem_walk_pose(IRON_GOLEM_ARM_POSE, 0.0, 1.0, IronGolemWalkPart::RightArm);
    let left_arm = iron_golem_walk_pose(IRON_GOLEM_ARM_POSE, 0.0, 1.0, IronGolemWalkPart::LeftArm);
    assert!(
        (right_leg.rotation[0] + 1.5).abs() < 1e-6,
        "{}",
        right_leg.rotation[0]
    );
    assert!(
        (left_leg.rotation[0] - 1.5).abs() < 1e-6,
        "{}",
        left_leg.rotation[0]
    );
    assert!(
        (right_arm.rotation[0] - 1.3).abs() < 1e-6,
        "{}",
        right_arm.rotation[0]
    );
    assert!(
        (left_arm.rotation[0] + 1.7).abs() < 1e-6,
        "{}",
        left_arm.rotation[0]
    );

    // At pos = 6.5: triangleWave = (|6.5 - 6.5| - 3.25) / 3.25 = -1.0, scaled by speed.
    let right_leg = iron_golem_walk_pose(
        IRON_GOLEM_RIGHT_LEG_POSE,
        6.5,
        0.5,
        IronGolemWalkPart::RightLeg,
    );
    assert!(
        (right_leg.rotation[0] - (-1.5 * -1.0 * 0.5)).abs() < 1e-6,
        "{}",
        right_leg.rotation[0]
    );
}

#[test]
fn iron_golem_swings_its_limbs_when_walking() {
    // `IronGolemModel.setupAnim` swings the legs and (default branch) the arms by
    // `Mth.triangleWave(pos, 13) * speed`. A standing golem is inert; a walking one
    // moves its arms and legs and lifts its feet. The attack swing and offer-flower
    // arm pose are deferred. Colored path here, textured below.
    let base = EntityModelInstance::iron_golem(270, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_ne!(rest.vertices, walking.vertices, "walking differs");

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking iron golem's feet should lift off the ground"
    );
    assert!(
        (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
        "a walking iron golem's limbs should splay along Z"
    );
}

#[test]
fn iron_golem_textured_mesh_swings_its_limbs_when_walking() {
    // The real iron golem render path (texture-backed) swings the same limbs. A
    // standing golem is byte-identical however far the swing has advanced; a walking
    // one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    let base = EntityModelInstance::iron_golem(271, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with_walk_animation(2.5, 0.0);
    let walking_instance = base.with_walk_animation(0.0, 1.0);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_golem_submissions_match_vanilla(&resting, base);
    assert_golem_submissions_match_vanilla(&still, still_instance);
    assert_golem_submissions_match_vanilla(&walking, walking_instance);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing iron golem is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "limb swing keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices, walking.cutout.vertices,
        "a walking iron golem differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
    let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking iron golem's feet should lift off the ground"
    );
}

fn assert_golem_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_golem_folded_meshes_are_cutout_only(meshes);
    let (passes, transform) = match instance.kind {
        EntityModelKind::IronGolem { crackiness } => (
            iron_golem_textured_layer_passes(crackiness),
            iron_golem_model_root_transform(instance),
        ),
        EntityModelKind::SnowGolem => (
            snow_golem_textured_layer_passes(),
            entity_model_root_transform(instance),
        ),
        _ => panic!("expected golem instance"),
    };
    assert_eq!(meshes.submissions.len(), passes.len());
    for (submit, pass) in meshes.submissions.iter().copied().zip(passes) {
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type, pass.render_type);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, pass.texture);
        assert_eq!(submit.tint, pass.tint);
        assert_eq!(submit.transform, transform);
        assert_eq!(submit.light, instance.render_state.shader_light());
        let expected_overlay = match pass.kind {
            EntityModelLayerKind::IronGolemCrackiness => {
                [0.0, instance.render_state.overlay_coords()[1]]
            }
            _ => instance.render_state.overlay_coords(),
        };
        assert_eq!(submit.overlay, expected_overlay);
        assert_eq!(
            (submit.order, submit.submit_sequence),
            (pass.order, pass.submit_sequence)
        );
    }
}

fn assert_golem_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn golem_texture_images() -> Vec<EntityModelTextureImage> {
    golem_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

// Vanilla `Mth.triangleWave(index, period)` (replicated for the test expectations).
fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

fn iron_golem_submission_texture_images() -> Vec<EntityModelTextureImage> {
    [
        IRON_GOLEM_TEXTURE_REF,
        IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for vector in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z] {
        assert_close3(
            actual.transform_point3(vector).to_array(),
            expected.transform_point3(vector).to_array(),
        );
    }
}

#[test]
fn attacking_iron_golem_raises_both_fists() {
    // Vanilla `IronGolemModel.setupAnim`: while `attackTicksRemaining > 0` both arms raise into the
    // smash (`xRot = -2 + 1.5·triangleWave(tick, 10)`), overriding the walk swing; the legs keep it.
    let attack = 8.0_f32;
    let walking =
        EntityModelInstance::iron_golem(73, [0.0, 64.0, 0.0], 0.0).with_walk_animation(3.0, 1.0);
    let attacking = walking.with_iron_golem_attack_ticks_remaining(attack);

    let mut model = IronGolemModel::new();
    model.prepare(&attacking);
    let expected = -2.0 + 1.5 * triangle_wave(attack, 10.0);
    let right_arm = model.root_mut().child_mut("right_arm").pose.rotation[0];
    let left_arm = model.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (right_arm - expected).abs() < 1.0e-6,
        "right fist raises: {right_arm}"
    );
    assert!(
        (left_arm - expected).abs() < 1.0e-6,
        "left fist raises: {left_arm}"
    );

    // The legs still walk-swing (the smash only overrides the arms): they differ from a standing golem.
    let standing = {
        let mut m = IronGolemModel::new();
        m.prepare(&EntityModelInstance::iron_golem(73, [0.0, 64.0, 0.0], 0.0));
        m.root_mut().child_mut("right_leg").pose.rotation[0]
    };
    let attacking_leg = model.root_mut().child_mut("right_leg").pose.rotation[0];
    assert_ne!(
        attacking_leg, standing,
        "the legs keep walking during the smash"
    );
}

#[test]
fn offering_iron_golem_holds_out_a_poppy() {
    // Vanilla `IronGolemModel.setupAnim`: while `offerFlowerTick > 0` (and not attacking) the right arm
    // holds the poppy out (`xRot = -0.8 + 0.025·triangleWave(tick, 70)`) and the left arm drops flat.
    let offer = 200;
    let base = EntityModelInstance::iron_golem(74, [0.0, 64.0, 0.0], 0.0)
        .with_iron_golem_offer_flower_tick(offer);
    let mut model = IronGolemModel::new();
    model.prepare(&base);

    let right_arm = model.root_mut().child_mut("right_arm").pose.rotation[0];
    let left_arm = model.root_mut().child_mut("left_arm").pose.rotation[0];
    assert!(
        (right_arm - (-0.8 + 0.025 * triangle_wave(offer as f32, 70.0))).abs() < 1.0e-6,
        "the right arm holds out the poppy: {right_arm}"
    );
    assert_eq!(left_arm, 0.0, "the left arm drops flat");

    // Attack takes priority over the offer pose.
    let mut both = IronGolemModel::new();
    both.prepare(&base.with_iron_golem_attack_ticks_remaining(5.0));
    let both_left = both.root_mut().child_mut("left_arm").pose.rotation[0];
    let both_right = both.root_mut().child_mut("right_arm").pose.rotation[0];
    assert!(
        (both_left - both_right).abs() < 1.0e-6,
        "the attack raises both arms equally, overriding the offer"
    );

    // A resting golem with neither timer keeps its bind arms (no override).
    let mut resting = IronGolemModel::new();
    resting.prepare(&EntityModelInstance::iron_golem(74, [0.0, 64.0, 0.0], 0.0));
    assert_eq!(
        resting.root_mut().child_mut("right_arm").pose.rotation[0],
        0.0
    );
}
