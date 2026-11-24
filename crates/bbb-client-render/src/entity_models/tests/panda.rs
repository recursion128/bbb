use super::*;
use crate::entity_models::model_layers::panda_model;

#[test]
fn panda_gene_variant_combination_matches_vanilla() {
    // Vanilla `Panda.Gene.byId` is `ByIdMap.continuous` with `OutOfBoundsStrategy.ZERO`.
    assert_eq!(PandaModelVariant::from_id(0), PandaModelVariant::Normal);
    assert_eq!(PandaModelVariant::from_id(1), PandaModelVariant::Lazy);
    assert_eq!(PandaModelVariant::from_id(2), PandaModelVariant::Worried);
    assert_eq!(PandaModelVariant::from_id(3), PandaModelVariant::Playful);
    assert_eq!(PandaModelVariant::from_id(4), PandaModelVariant::Brown);
    assert_eq!(PandaModelVariant::from_id(5), PandaModelVariant::Weak);
    assert_eq!(PandaModelVariant::from_id(6), PandaModelVariant::Aggressive);
    assert_eq!(PandaModelVariant::from_id(7), PandaModelVariant::Normal);
    assert_eq!(PandaModelVariant::from_id(-1), PandaModelVariant::Normal);

    // Vanilla `Panda.Gene.getVariantFromGenes`: a dominant main gene always shows.
    assert_eq!(
        PandaModelVariant::from_genes(1, 4),
        PandaModelVariant::Lazy,
        "dominant LAZY shows regardless of the hidden gene"
    );
    // A recessive main gene (BROWN=4, WEAK=5) shows only when both genes match, else NORMAL.
    assert_eq!(
        PandaModelVariant::from_genes(4, 4),
        PandaModelVariant::Brown,
        "matching recessive BROWN shows"
    );
    assert_eq!(
        PandaModelVariant::from_genes(4, 1),
        PandaModelVariant::Normal,
        "an unmatched recessive BROWN main falls back to NORMAL"
    );
    assert_eq!(
        PandaModelVariant::from_genes(5, 5),
        PandaModelVariant::Weak,
        "matching recessive WEAK shows"
    );
    assert_eq!(
        PandaModelVariant::from_genes(5, 0),
        PandaModelVariant::Normal,
        "an unmatched recessive WEAK main falls back to NORMAL"
    );
}

#[test]
fn panda_textured_render_matches_vanilla_renderer() {
    // The seven genes × age form a 14-texture matrix; `PandaRenderer.getTextureLocation` keys it off
    // the displayed gene and `isBaby`.
    for (variant, adult, baby) in [
        (
            PandaModelVariant::Normal,
            PANDA_NORMAL_TEXTURE_REF,
            PANDA_NORMAL_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Lazy,
            PANDA_LAZY_TEXTURE_REF,
            PANDA_LAZY_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Worried,
            PANDA_WORRIED_TEXTURE_REF,
            PANDA_WORRIED_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Playful,
            PANDA_PLAYFUL_TEXTURE_REF,
            PANDA_PLAYFUL_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Brown,
            PANDA_BROWN_TEXTURE_REF,
            PANDA_BROWN_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Weak,
            PANDA_WEAK_TEXTURE_REF,
            PANDA_WEAK_BABY_TEXTURE_REF,
        ),
        (
            PandaModelVariant::Aggressive,
            PANDA_AGGRESSIVE_TEXTURE_REF,
            PANDA_AGGRESSIVE_BABY_TEXTURE_REF,
        ),
    ] {
        for (is_baby, texture) in [(false, adult), (true, baby)] {
            let passes = panda_textured_layer_passes(variant, is_baby);
            assert_eq!(passes.len(), 1);
            assert_eq!(passes[0].texture, texture);
            assert_eq!(
                passes[0].render_type,
                EntityModelLayerRenderType::EntityCutout
            );
            assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
            assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
            assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(passes[0].order, 0);
        }
        assert!(entity_model_texture_refs().contains(&adult));
        assert!(entity_model_texture_refs().contains(&baby));
    }

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/panda/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let instance =
            EntityModelInstance::panda(900, [0.0, 64.0, 0.0], 0.0, baby, PandaModelVariant::Brown)
                .with(|s| {
                    s.light_coords = (7_u32 << 4) | (11_u32 << 20);
                    s.white_overlay_progress = 0.8;
                    s.has_red_overlay = true;
                });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
    }
}

#[test]
fn unhappy_panda_shakes_its_head_and_paddles_its_front_legs() {
    // Vanilla `PandaModel.setupAnim` `isUnhappy`: the head shakes (`yRot = zRot = 0.35·sin(0.6·age)`,
    // overwriting the look yaw) and the front legs paddle (`xRot = ∓0.75·sin(0.3·age)`, overwriting the
    // walk swing). All driven by `ageInTicks`; the hind legs and body are untouched.
    let age = 7.0_f32;
    let base =
        EntityModelInstance::panda(620, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| (s.head_yaw, s.head_pitch) = (40.0, 0.0))
            .with(|s| s.age_in_ticks = age);
    let mut model = panda_model(false, &base.with(|s| s.kind.panda_mut().unhappy = true));

    let shake = 0.35 * (0.6 * age).sin();
    let head = model.child_mut("head").pose.rotation;
    assert!(
        (head[1] - shake).abs() < 1.0e-6,
        "the head shake overwrites the look yaw: {}",
        head[1]
    );
    assert!(
        (head[2] - shake).abs() < 1.0e-6,
        "the head rolls: {}",
        head[2]
    );

    let paddle = 0.75 * (0.3 * age).sin();
    let right_front = model.child_mut("right_front_leg").pose.rotation[0];
    let left_front = model.child_mut("left_front_leg").pose.rotation[0];
    assert!(
        (right_front + paddle).abs() < 1.0e-6,
        "right front paddles: {right_front}"
    );
    assert!(
        (left_front - paddle).abs() < 1.0e-6,
        "left front mirrors: {left_front}"
    );

    // A content panda keeps the look yaw (no shake) and a flat head roll.
    let mut content = panda_model(false, &base);
    let content_head = content.child_mut("head").pose.rotation;
    assert_eq!(
        content_head[2], 0.0,
        "a content panda does not roll its head"
    );
    assert!(
        (content_head[1] - 40.0_f32.to_radians()).abs() < 1.0e-6,
        "a content panda tracks the look yaw: {}",
        content_head[1]
    );

    // The shake animates with ageInTicks (mesh changes frame to frame).
    let later = entity_model_mesh(&[base.with(|s| {
        s.kind.panda_mut().unhappy = true;
        s.age_in_ticks = age + 3.0;
    })]);
    assert_ne!(
        entity_model_mesh(&[base.with(|s| s.kind.panda_mut().unhappy = true)]).vertices,
        later.vertices,
        "the unhappy shake keeps moving"
    );
}

#[test]
fn sneezing_panda_dips_its_head() {
    use std::f32::consts::PI;

    // Vanilla `PandaModel.setupAnim` `isSneezing`: the head dips, `xRot = -π/4·sneezeTime/14` over ticks
    // 0..14, then holds at `-π/4` for 15..19 (vanilla's `(sneezeTime-15)/5` integer division is 0). The
    // sneeze SETs the head pitch, overwriting the look pitch.
    let base =
        EntityModelInstance::panda(621, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -30.0))
            .with(|s| s.kind.panda_mut().sneezing = true);

    // Mid-ramp (tick 7): the head has dipped halfway to -π/4.
    let mut ramp = panda_model(false, &base.with(|s| s.kind.panda_mut().sneeze_time = 7));
    let ramp_pitch = ramp.child_mut("head").pose.rotation[0];
    assert!(
        (ramp_pitch - (-PI / 4.0 * 7.0 / 14.0)).abs() < 1.0e-6,
        "the head dips partway: {ramp_pitch}"
    );

    // Peak hold (tick 17): the head holds at -π/4.
    let mut hold = panda_model(false, &base.with(|s| s.kind.panda_mut().sneeze_time = 17));
    let hold_pitch = hold.child_mut("head").pose.rotation[0];
    assert!(
        (hold_pitch + PI / 4.0).abs() < 1.0e-6,
        "the head holds at -π/4: {hold_pitch}"
    );

    // A panda not sneezing keeps its look pitch (sneeze flag off → the dip never applies, even with a
    // stale counter).
    let mut quiet = panda_model(
        false,
        &EntityModelInstance::panda(621, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -30.0))
            .with(|s| s.kind.panda_mut().sneeze_time = 7),
    );
    let quiet_pitch = quiet.child_mut("head").pose.rotation[0];
    assert!(
        (quiet_pitch - (-30.0_f32).to_radians()).abs() < 1.0e-6,
        "a non-sneezing panda keeps its look pitch: {quiet_pitch}"
    );
}

#[test]
fn sitting_panda_applies_vanilla_adult_and_baby_pose() {
    use std::f32::consts::{FRAC_PI_2, PI};

    let adult =
        EntityModelInstance::panda(622, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| s.kind.panda_mut().sit_amount = 1.0);
    let mut adult_model = panda_model(false, &adult);
    assert_close(adult_model.child_mut("body").pose.rotation[0], 1.7407963);
    assert_close(adult_model.child_mut("head").pose.rotation[0], FRAC_PI_2);
    assert_close(
        adult_model.child_mut("right_front_leg").pose.rotation[2],
        -0.27079642,
    );
    assert_close(
        adult_model.child_mut("left_hind_leg").pose.rotation[2],
        -0.5707964,
    );

    let baby =
        EntityModelInstance::panda(623, [0.0, 64.0, 0.0], 0.0, true, PandaModelVariant::Normal)
            .with(|s| s.kind.panda_mut().sit_amount = 1.0);
    let mut baby_model = panda_model(true, &baby);
    let body = baby_model.child_mut("body");
    assert_close(body.pose.rotation[0], PI / 18.0);
    assert_close(body.pose.offset[2], -1.5);
    let head = baby_model.child_mut("head");
    assert_close(head.pose.offset[1], 17.5);
    assert_close(head.pose.offset[2], -11.5);
    assert_close(baby_model.child_mut("right_front_leg").pose.offset[2], -5.0);
    assert_close(baby_model.child_mut("left_hind_leg").pose.offset[2], 3.0);
}

#[test]
fn sitting_panda_applies_eating_and_scared_overrides() {
    use std::f32::consts::FRAC_PI_2;

    let age = 5.0_f32;
    let wave = (age * 0.6).sin();
    let base =
        EntityModelInstance::panda(624, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| {
                s.kind.panda_mut().sit_amount = 1.0;
                s.age_in_ticks = age;
            });
    let mut eating = panda_model(false, &base.with(|s| s.kind.panda_mut().eating = true));
    assert_close(
        eating.child_mut("head").pose.rotation[0],
        FRAC_PI_2 + 0.2 * wave,
    );
    assert_close(
        eating.child_mut("right_front_leg").pose.rotation[0],
        -0.4 - 0.2 * wave,
    );
    assert_close(
        eating.child_mut("left_front_leg").pose.rotation[0],
        -0.4 - 0.2 * wave,
    );
    let mut scared = panda_model(
        false,
        &base.with(|s| {
            s.kind.panda_mut().eating = true;
            s.kind.panda_mut().scared = true;
        }),
    );
    assert_close(scared.child_mut("head").pose.rotation[0], 2.1707964);
    assert_close(scared.child_mut("right_front_leg").pose.rotation[0], -0.9);
    assert_close(scared.child_mut("left_front_leg").pose.rotation[0], -0.9);
}

#[test]
fn panda_lie_on_back_and_roll_amount_pose_limbs_and_head() {
    use std::f32::consts::FRAC_PI_2;

    let age = 4.0_f32;
    let base =
        EntityModelInstance::panda(625, [0.0, 64.0, 0.0], 0.0, false, PandaModelVariant::Normal)
            .with(|s| s.age_in_ticks = age);
    let mut lying = panda_model(
        false,
        &base.with(|s| s.kind.panda_mut().lie_on_back_amount = 1.0),
    );
    assert_close(lying.child_mut("head").pose.rotation[0], FRAC_PI_2);
    assert_close(
        lying.child_mut("right_hind_leg").pose.rotation[0],
        -0.6 * (age * 0.15).sin(),
    );
    assert_close(
        lying.child_mut("left_front_leg").pose.rotation[0],
        -0.3 * (age * 0.25).sin(),
    );
    let mut rolling = panda_model(false, &base.with(|s| s.kind.panda_mut().roll_amount = 1.0));
    assert_close(rolling.child_mut("head").pose.rotation[0], 2.0561945);
    assert_close(
        rolling.child_mut("right_front_leg").pose.rotation[0],
        0.5 * (age * 0.5).sin(),
    );
    assert_close(
        rolling.child_mut("left_hind_leg").pose.rotation[0],
        0.5 * (age * 0.5).sin(),
    );
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}
