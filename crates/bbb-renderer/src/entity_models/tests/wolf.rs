use super::*;

use crate::entity_models::model::EntityModel;
use crate::entity_models::model::ModelCube;

// The adult wolf tail bind pose, mirrored from the model file so the tail pose-math tests can run
// without the deleted `ADULT_WOLF_PARTS` const tree. The layer rests the tail at the π/5 wild droop.
const ADULT_WOLF_TAIL_POSE: PartPose = PartPose {
    offset: [-1.0, 12.0, 8.0],
    rotation: [0.62831855, 0.0, 0.0],
};

#[test]
fn wolf_textured_mesh_uses_vanilla_uvs_and_collar_tint() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            305,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            false,
            Some(EntityDyeColor::Blue),
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [10.0 / 64.0, 32.0 / 256.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[144].uv, [4.0 / 64.0, 52.0 / 256.0]);
    assert_close2(mesh.vertices[264].uv, [10.0 / 64.0, 192.0 / 256.0]);
    assert_eq!(
        mesh.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );

    let untamed_with_collar_metadata = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            false,
            Some(EntityDyeColor::Red),
        )],
        &atlas,
    );
    assert_eq!(untamed_with_collar_metadata.cutout_faces, 66);
    assert!(untamed_with_collar_metadata
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let invisible_tame = entity_model_textured_mesh(
        &[EntityModelInstance::wolf_state(
            307,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            true,
            Some(EntityDyeColor::Blue),
        )],
        &atlas,
    );
    // An invisible wolf renders nothing: the unified `render_state.invisible` skips the whole
    // model in both paths, so no body and no collar layer is emitted.
    assert_eq!(invisible_tame.cutout_faces, 0);
    assert!(invisible_tame.vertices.is_empty());
}

#[test]
fn wolf_cubes_match_vanilla_26_1_body_layers() {
    // Vanilla `AdultWolfModel.createBodyLayer` (atlas 64×32). Each unified cube carries the colored
    // tint (`WOLF_GRAY`) and the textured UV; the right legs reuse the left leg's `texOffs(0, 18)`
    // mirrored.
    assert_eq!(
        ADULT_WOLF_REAL_HEAD[0],
        ModelCube::new(
            [-2.0, -3.0, -2.0],
            [6.0, 6.0, 4.0],
            WOLF_GRAY,
            [6.0, 6.0, 4.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_WOLF_REAL_HEAD.len(), 4);
    assert_eq!(ADULT_WOLF_BODY[0].tex, [18.0, 14.0]);
    assert_eq!(ADULT_WOLF_UPPER_BODY[0].tex, [21.0, 0.0]);
    assert_eq!(ADULT_WOLF_LEFT_LEG[0].tex, [0.0, 18.0]);
    assert!(!ADULT_WOLF_LEFT_LEG[0].mirror);
    assert_eq!(
        ADULT_WOLF_RIGHT_LEG[0],
        ModelCube::new(
            [0.0, 0.0, -1.0],
            [2.0, 8.0, 2.0],
            WOLF_GRAY,
            [2.0, 8.0, 2.0],
            [0.0, 18.0],
            true,
        )
    );
    assert_eq!(ADULT_WOLF_REAL_TAIL[0].tex, [9.0, 18.0]);

    // Vanilla `BabyWolfModel.createBodyLayer` (atlas 32×32): an inflated skull box keeping the base UV.
    assert_eq!(
        BABY_WOLF_HEAD[0],
        ModelCube::new(
            [-3.015, -3.275, -3.025],
            [6.05, 5.05, 5.05],
            WOLF_GRAY,
            [6.0, 5.0, 5.0],
            [0.0, 12.0],
            false,
        )
    );
    assert_eq!(BABY_WOLF_RIGHT_EAR[0].tex, [0.0, 5.0]);
    assert_eq!(BABY_WOLF_LEFT_EAR[0].tex, [20.0, 5.0]);
    assert_eq!(BABY_WOLF_BODY[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_WOLF_RIGHT_HIND_LEG[0].tex, [0.0, 22.0]);
    assert_eq!(BABY_WOLF_LEFT_HIND_LEG[0].tex, [8.0, 22.0]);
    assert_eq!(BABY_WOLF_RIGHT_FRONT_LEG[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_WOLF_LEFT_FRONT_LEG[0].tex, [20.0, 0.0]);
    assert_eq!(BABY_WOLF_TAIL_R1[0].tex, [22.0, 16.0]);
}

#[test]
fn wolf_meshes_use_vanilla_body_layer_geometry() {
    let adult = entity_model_mesh(&[EntityModelInstance::wolf(148, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(adult.opaque_faces, 66);
    assert_eq!(adult.vertices.len(), 264);
    assert_eq!(adult.indices.len(), 396);
    let (adult_min, adult_max) = mesh_extents(&adult);
    assert_close3(adult_min, [-0.25, 64.001, -0.8444562]);
    assert_close3(adult_max, [0.25000006, 64.96975, 0.75]);

    let baby = entity_model_mesh(&[EntityModelInstance::wolf(149, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_eq!(baby.indices.len(), 360);
    let (baby_min, baby_max) = mesh_extents(&baby);
    // The baby tail rests at the `tailAngle` π/5 (the wild `getTailAngle()`), which
    // `WolfModel.setupAnim` writes over the baby layer's −π/6 base pose every frame (vanilla
    // never displays the un-posed layer rest), so the tail points back rather than up: the
    // feet (y 64.001) are the lowest point and the tail reaches further back in −Z.
    assert_close3(baby_min, [-0.1884375, 64.001, -0.44576454]);
    assert_close3(baby_max, [0.18968754, 64.6885, 0.5625]);
}

#[test]
fn wolf_texture_refs_match_vanilla_renderer_pale_variant_assets() {
    let cases = [
        (
            false,
            false,
            false,
            "wolf",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf.png",
                size: [64, 32],
            },
        ),
        (
            false,
            true,
            false,
            "wolf_tame",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame.png",
                size: [64, 32],
            },
        ),
        (
            false,
            false,
            true,
            "wolf_angry",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry.png",
                size: [64, 32],
            },
        ),
        (
            true,
            false,
            false,
            "wolf_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            true,
            false,
            "wolf_tame_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_tame_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            false,
            true,
            "wolf_angry_baby",
            EntityModelTextureRef {
                path: "textures/entity/wolf/wolf_angry_baby.png",
                size: [32, 32],
            },
        ),
    ];
    for (baby, tame, angry, model_key, texture) in cases {
        let kind = EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_COLLAR_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::Wolf {
            baby: true,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
        .vanilla_layer_texture_refs(),
        &[WOLF_BABY_COLLAR_TEXTURE_REF]
    );
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        collar_color: None,
        variant: WolfModelVariant::Pale,
    }
    .vanilla_layer_texture_refs()
    .is_empty());
    assert!(EntityModelKind::Wolf {
        baby: false,
        tame: false,
        angry: false,
        collar_color: Some(EntityDyeColor::Red),
        variant: WolfModelVariant::Pale,
    }
    .vanilla_layer_texture_refs()
    .is_empty());
}

#[test]
fn wolf_texture_refs_match_vanilla_renderer_biome_variants() {
    // Vanilla `Wolf.getTexture` selects `variant.adultInfo()/babyInfo()` then `.tame()/.angry()/
    // .wild()`. Spot-check each of the eight non-pale variants across the wild/tame/angry × baby
    // matrix, mirroring the `WolfVariants.register` file-name scheme.
    let texture = |baby, tame, angry, variant| {
        EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color: None,
            variant,
        }
        .vanilla_texture_ref()
        .unwrap()
        .path
    };
    assert_eq!(
        texture(false, false, false, WolfModelVariant::Spotted),
        "textures/entity/wolf/wolf_spotted.png"
    );
    assert_eq!(
        texture(false, true, false, WolfModelVariant::Snowy),
        "textures/entity/wolf/wolf_snowy_tame.png"
    );
    assert_eq!(
        texture(false, false, true, WolfModelVariant::Black),
        "textures/entity/wolf/wolf_black_angry.png"
    );
    assert_eq!(
        texture(true, false, false, WolfModelVariant::Ashen),
        "textures/entity/wolf/wolf_ashen_baby.png"
    );
    assert_eq!(
        texture(true, true, false, WolfModelVariant::Rusty),
        "textures/entity/wolf/wolf_rusty_tame_baby.png"
    );
    assert_eq!(
        texture(true, false, true, WolfModelVariant::Woods),
        "textures/entity/wolf/wolf_woods_angry_baby.png"
    );
    assert_eq!(
        texture(false, false, false, WolfModelVariant::Chestnut),
        "textures/entity/wolf/wolf_chestnut.png"
    );
    assert_eq!(
        texture(false, false, true, WolfModelVariant::Striped),
        "textures/entity/wolf/wolf_striped_angry.png"
    );
    // A tamed wolf shows the tame face regardless of anger (vanilla checks `isTame()` first).
    assert_eq!(
        texture(false, true, true, WolfModelVariant::Spotted),
        "textures/entity/wolf/wolf_spotted_tame.png"
    );

    // The model_key (mesh geometry) is variant-agnostic: all coats share one `WolfModel`.
    for variant in [WolfModelVariant::Spotted, WolfModelVariant::Striped] {
        assert_eq!(
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                collar_color: None,
                variant,
            }
            .model_key(),
            "wolf"
        );
    }

    // Every biome face joins the global entity atlas, so a variant wolf resolves at runtime.
    for path in [
        "textures/entity/wolf/wolf_spotted.png",
        "textures/entity/wolf/wolf_striped_angry_baby.png",
        "textures/entity/wolf/wolf_chestnut_tame.png",
    ] {
        assert!(
            entity_model_texture_refs()
                .iter()
                .any(|texture| texture.path == path),
            "missing {path} from the global atlas"
        );
    }
}

#[test]
fn wolf_textured_layer_passes_match_vanilla_renderer_layers() {
    // The vestigial `parts` slices are nulled; every pass reads the unified `WolfModel` tree.
    let wild = wolf_textured_layer_passes(false, false, false, None, WolfModelVariant::Pale, 1.0);
    assert_eq!(
        wild.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![EntityModelLayerKind::WolfBase]
    );
    assert_eq!(wild[0].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(wild[0].texture, WOLF_TEXTURE_REF);
    assert_eq!(wild[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wild[0].collector_order, wild[0].submit_sequence), (0, 0));

    let tame_blue = wolf_textured_layer_passes(
        false,
        true,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(
        tame_blue.iter().map(|pass| pass.kind).collect::<Vec<_>>(),
        vec![
            EntityModelLayerKind::WolfBase,
            EntityModelLayerKind::WolfCollar
        ]
    );
    assert_eq!(tame_blue[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_blue[1].model_layer, MODEL_LAYER_WOLF);
    assert_eq!(tame_blue[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(
        tame_blue[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(
        (tame_blue[1].collector_order, tame_blue[1].submit_sequence),
        (1, 1)
    );

    // An untamed wolf carrying collar metadata still emits only the base layer: the collar pass
    // is gated on `tame`, so a wild wolf renders no collar.
    let untamed_with_collar = wolf_textured_layer_passes(
        false,
        false,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(
        untamed_with_collar
            .iter()
            .map(|pass| pass.kind)
            .collect::<Vec<_>>(),
        vec![EntityModelLayerKind::WolfBase]
    );
    assert_eq!(untamed_with_collar[0].texture, WOLF_TEXTURE_REF);

    let angry = wolf_textured_layer_passes(false, false, true, None, WolfModelVariant::Pale, 1.0);
    assert_eq!(angry[0].texture, WOLF_ANGRY_TEXTURE_REF);
    assert_eq!(angry.len(), 1);

    let tame_angry = wolf_textured_layer_passes(
        false,
        true,
        true,
        Some(EntityDyeColor::Red),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(tame_angry[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_angry.len(), 2);

    let baby_tame = wolf_textured_layer_passes(
        true,
        true,
        false,
        Some(EntityDyeColor::Red),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(baby_tame[0].model_layer, MODEL_LAYER_WOLF_BABY);
    assert_eq!(baby_tame[0].texture, WOLF_TAME_BABY_TEXTURE_REF);
    assert_eq!(baby_tame[1].texture, WOLF_BABY_COLLAR_TEXTURE_REF);

    assert_eq!(MODEL_LAYER_WOLF, "minecraft:wolf#main");
    assert_eq!(MODEL_LAYER_WOLF_BABY, "minecraft:wolf_baby#main");
}

#[test]
fn wet_wolf_textured_base_tints_like_vanilla_model_tint_without_shading_collar() {
    // Vanilla `WolfRenderer.getModelTint`: `wetShade == 1 ? -1 :
    // ARGB.colorFromFloat(1, wetShade, wetShade, wetShade)`. `LivingEntityRenderer.submit`
    // multiplies that tint into the base model submission; `WolfCollarLayer` is a later
    // order-1 layer with its own dye color.
    let wet_shade = 0.75;
    let passes = wolf_textured_layer_passes(
        false,
        true,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        wet_shade,
    );
    assert_eq!(passes.len(), 2);
    assert_eq!(passes[0].kind, EntityModelLayerKind::WolfBase);
    assert_eq!(passes[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(passes[0].tint, [wet_shade, wet_shade, wet_shade, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
    assert_eq!(passes[1].kind, EntityModelLayerKind::WolfCollar);
    assert_eq!(passes[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(passes[1].tint, EntityDyeColor::Blue.texture_diffuse_color());
    assert_eq!(
        (passes[1].collector_order, passes[1].submit_sequence),
        (1, 1)
    );

    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::wolf_state(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            false,
            Some(EntityDyeColor::Blue),
        )
        .with_wolf_wet_shade(wet_shade)],
        &atlas,
    );

    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(meshes.submissions[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(
        meshes.submissions[0].tint,
        [wet_shade, wet_shade, wet_shade, 1.0]
    );
    assert_eq!(meshes.submissions[0].collector_order, 0);
    assert_eq!(meshes.submissions[0].submit_sequence, 0);
    assert_eq!(meshes.submissions[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(
        meshes.submissions[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(meshes.submissions[1].collector_order, 1);
    assert_eq!(meshes.submissions[1].submit_sequence, 1);
    assert_eq!(
        meshes.cutout.vertices[0].tint,
        [wet_shade, wet_shade, wet_shade, 1.0]
    );
    assert_eq!(
        meshes.cutout.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
}

#[test]
fn wolf_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for base in [
        EntityModelInstance::wolf(480, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::wolf(481, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

#[test]
fn wolf_swings_its_legs_when_walking() {
    // Vanilla `WolfModel.setupAnim` (adult and baby) swings the four legs with the
    // `QuadrupedModel` diagonal phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed` in its
    // non-sitting branch. A standing wolf is inert; a walking one splays its legs along
    // Z. The adult (with its tall legs) also lifts its feet; the baby's short legs swing
    // inside the head/body bounding box, so only the Z splay shows. The Z splay is
    // measured over the leg vertex region so the tail's resting `tailAngle` droop (which
    // for the baby overrides the layer pose) does not mask the gait. The water-shake body
    // roll and the sitting pose are deferred. Colored path.
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    for (base, adult_size, legs) in [
        (
            EntityModelInstance::wolf(148, [0.0, 64.0, 0.0], 0.0, false),
            true,
            144..240,
        ),
        (
            EntityModelInstance::wolf(149, [0.0, 64.0, 0.0], 0.0, true),
            false,
            120..216,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        assert!(
            z_extent(&walking.vertices[legs.clone()])
                > z_extent(&rest.vertices[legs.clone()]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
        if adult_size {
            let (rest_min, rest_max) = mesh_extents(&rest);
            let (walk_min, walk_max) = mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "an adult wolf's feet should lift off the ground"
            );
        }
    }
}

#[test]
fn wolf_textured_mesh_swings_its_legs_when_walking() {
    // The real wolf render path (texture-backed) swings the same legs. A standing wolf is
    // byte-identical however far the swing has advanced; a walking one differs, splays
    // along Z (measured over the leg region so the resting tail droop does not mask it),
    // and (for the adult) lifts its feet, while keeping the vertex count.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for (base, adult_size, legs) in [
        (
            EntityModelInstance::wolf(482, [0.0, 64.0, 0.0], 0.0, false),
            true,
            144..240,
        ),
        (
            EntityModelInstance::wolf(483, [0.0, 64.0, 0.0], 0.0, true),
            false,
            120..216,
        ),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(resting.vertices, still.vertices, "{:?} is inert", base.kind);
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        assert!(
            z_extent(&walking.vertices[legs.clone()])
                > z_extent(&resting.vertices[legs.clone()]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "an adult wolf's feet should lift off the ground (textured)"
            );
        }
    }
}

fn wolf_texture_images() -> Vec<EntityModelTextureImage> {
    wolf_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn wolf_tail_swing_pose_wags_with_the_quadruped_amplitude() {
    // Vanilla WolfModel.setupAnim (non-angry branch): tail.yRot = cos(pos * 0.6662) *
    // 1.4 * speed (the same QuadrupedModel amplitude as the legs, no phase offset), then
    // tail.xRot = state.tailAngle. The base tail pose carries the layer's resting xRot
    // droop (0.62831855 = π/5, the untamed tailAngle); the wag sets yRot and xRot and
    // preserves the offset and zRot.
    let base = ADULT_WOLF_TAIL_POSE;
    let wild = std::f32::consts::PI / 5.0;
    assert!(
        (base.rotation[0] - wild).abs() < 1e-6,
        "adult tail rests with the layer xRot droop: {}",
        base.rotation[0]
    );
    let tail = wolf_tail_swing_pose(base, wild, 0.0, 1.0);
    assert!(
        (tail.rotation[1] - 1.4).abs() < 1e-6,
        "tail wags to cos(0) * 1.4 * 1 = 1.4: {}",
        tail.rotation[1]
    );
    assert_eq!(
        tail.rotation[0], wild,
        "untamed tailAngle sets the π/5 droop"
    );
    assert_eq!(tail.rotation[2], base.rotation[2], "zRot preserved");
    assert_eq!(tail.offset, base.offset, "offset preserved");

    // A general (pos, speed) reproduces cos(pos * 0.6662) * 1.4 * speed.
    let phase = 2.0_f32 * 0.6662;
    let tail = wolf_tail_swing_pose(base, wild, 2.0, 0.5);
    assert!((tail.rotation[1] - phase.cos() * 1.4 * 0.5).abs() < 1e-6);

    // At rest, an untamed wolf (tailAngle = π/5 = the layer xRot) is byte-identical to the
    // base pose, so the colored/textured borrow fast paths still apply.
    let tail = wolf_tail_swing_pose(base, wild, 3.0, 0.0);
    assert_eq!(tail.rotation[1], 0.0);
    assert_eq!(
        tail, base,
        "untamed wolf at rest matches the layer pose exactly"
    );

    // A tame wolf's health droop SETS a different xRot even when standing still.
    let droop = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // damageRatio 0.8 (health 8/40)
    let tail = wolf_tail_swing_pose(base, droop, 0.0, 0.0);
    assert_eq!(tail.rotation[0], droop, "tame tailAngle droop sets xRot");
    assert_eq!(tail.rotation[1], 0.0, "no wag at rest");
    assert_ne!(
        tail, base,
        "a drooping tame tail differs from the layer rest pose"
    );
}

#[test]
fn wolf_wags_its_tail_when_walking() {
    // The non-angry wolf wags its tail side to side (a yRot sweep) in step with the gait.
    // In the colored body layer the parts emit the head subtree and body/mane in the
    // leading blocks, then the four legs, then the tail child cube last: for the adult
    // (264 verts) the tail occupies vertices [240, 264) and the legs [144, 240); the baby
    // (240 verts) lists the tail at [216, 240) and the legs [120, 216). A yRot wag sweeps
    // the tail sideways, deepening the tail region's X footprint.
    let x_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[0]);
            hi = hi.max(vertex.position[0]);
        }
        hi - lo
    };
    for (baby, tail) in [(false, 240..264), (true, 216..240)] {
        let base = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, baby);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices[tail.clone()],
            walking.vertices[tail.clone()],
            "baby={baby}: the tail wags when walking"
        );
        let rest_tail_x = x_extent(&rest.vertices[tail.clone()]);
        let walk_tail_x = x_extent(&walking.vertices[tail.clone()]);
        assert!(
            walk_tail_x > rest_tail_x + 0.1,
            "baby={baby}: a yRot tail wag deepens the tail X footprint: {rest_tail_x} -> {walk_tail_x}"
        );
    }
}

#[test]
fn wolf_water_shake_roll_matches_vanilla_body_roll_angle() {
    // Vanilla `WolfRenderState.getBodyRollAngle(offset)` clamps `(shakeAnim + offset) / 1.8` and
    // applies two sine waves. Adult `AdultWolfModel.shakeOffWater` rolls the real head, body,
    // upper-body mane, and real tail; baby `BabyWolfModel.shakeOffWater` rolls head/body/tail.
    let shake_anim = 0.9;
    assert_eq!(wolf_body_roll_angle(0.0, -0.16), 0.0);
    assert!(wolf_body_roll_angle(2.5, 0.0).abs() < 1.0e-6);

    let adult_instance = EntityModelInstance::wolf(170, [0.0, 64.0, 0.0], 0.0, false)
        .with_wolf_shake_anim(shake_anim);
    let mut adult = WolfModel::new(false, false);
    adult.prepare(&adult_instance);
    let adult_root = adult.root_mut();
    assert!(
        (adult_root.child_mut("body").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.16))
            .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root
            .child_mut("head")
            .child_mut("real_head")
            .pose
            .rotation[2]
            - wolf_body_roll_angle(shake_anim, 0.0))
        .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root.child_mut("upper_body").pose.rotation[2]
            - wolf_body_roll_angle(shake_anim, -0.08))
        .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root
            .child_mut("tail")
            .child_mut("real_tail")
            .pose
            .rotation[2]
            - wolf_body_roll_angle(shake_anim, -0.2))
        .abs()
            < 1.0e-6
    );

    let baby_instance = EntityModelInstance::wolf(171, [0.0, 64.0, 0.0], 0.0, true)
        .with_wolf_shake_anim(shake_anim);
    let mut baby = WolfModel::new(true, false);
    baby.prepare(&baby_instance);
    let baby_root = baby.root_mut();
    assert!(
        (baby_root.child_mut("body").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.16))
            .abs()
            < 1.0e-6
    );
    assert!(
        (baby_root.child_mut("head").pose.rotation[2] - wolf_body_roll_angle(shake_anim, 0.0))
            .abs()
            < 1.0e-6
    );
    assert!(
        (baby_root.child_mut("tail").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.2))
            .abs()
            < 1.0e-6
    );
}

#[test]
fn wolf_textured_mesh_applies_water_shake_roll_to_base_and_collar() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let base = EntityModelInstance::wolf_state(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        false,
        Some(EntityDyeColor::Blue),
    );
    let dry = entity_model_textured_meshes(&[base], &atlas);
    let shaking = entity_model_textured_meshes(&[base.with_wolf_shake_anim(0.9)], &atlas);

    assert_eq!(dry.cutout.vertices.len(), shaking.cutout.vertices.len());
    assert_ne!(
        dry.cutout.vertices[..264],
        shaking.cutout.vertices[..264],
        "base wolf pass rolls while shaking water off"
    );
    assert_ne!(
        dry.cutout.vertices[264..],
        shaking.cutout.vertices[264..],
        "collar pass reuses the same rolled wolf pose"
    );

    assert_eq!(shaking.submissions.len(), 2);
    assert_eq!(shaking.submissions[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(shaking.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (
            shaking.submissions[0].collector_order,
            shaking.submissions[0].submit_sequence
        ),
        (0, 0)
    );
    assert_eq!(shaking.submissions[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(
        shaking.submissions[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(
        (
            shaking.submissions[1].collector_order,
            shaking.submissions[1].submit_sequence
        ),
        (1, 1)
    );
}

#[test]
fn wolf_textured_mesh_wags_its_tail_when_walking() {
    // The texture-backed wolf base layer runs the same tail wag, emitting the parts in the
    // same order, so the adult tail occupies textured vertices [240, 264). A standing wolf
    // is byte-identical; a walking one wags its tail sideways.
    let x_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[0]);
            hi = hi.max(vertex.position[0]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let base = EntityModelInstance::wolf(151, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);
    assert_ne!(
        resting.vertices[240..264],
        walking.vertices[240..264],
        "the tail wags when walking"
    );
    let rest_tail_x = x_extent(&resting.vertices[240..264]);
    let walk_tail_x = x_extent(&walking.vertices[240..264]);
    assert!(
        walk_tail_x > rest_tail_x + 0.1,
        "the textured tail wags sideways when walking: {rest_tail_x} -> {walk_tail_x}"
    );
}

#[test]
fn tame_wolf_droops_its_tail_with_damage() {
    // Vanilla `Wolf.getTailAngle()` for a tame wolf: `(0.55 - damageRatio * 0.4) * π`,
    // `damageRatio = (maxHealth - health) / maxHealth` (tame maxHealth = 40). The renderer
    // SETS the non-angry tail `xRot` to this projected `wolf_tail_angle`, so a healthy tame
    // wolf raises its tail off the π/5 wild rest droop and a damaged one bends it further
    // again, while the rest of the body is unchanged. Colored path here; textured below.
    // The colored adult layout lists head/body/mane/legs at [0, 240) and the tail at
    // [240, 264).
    let wild = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, false);
    let full = 0.55 * std::f32::consts::PI; // health 40/40 → damageRatio 0
    let hurt = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // health 8/40 → damageRatio 0.8
    let wild_mesh = entity_model_mesh(&[wild]);
    let healthy_mesh = entity_model_mesh(&[wild.with_wolf_tail_angle(full)]);
    let damaged_mesh = entity_model_mesh(&[wild.with_wolf_tail_angle(hurt)]);
    let tail = 240..264;
    assert_eq!(
        wild_mesh.vertices[..240],
        damaged_mesh.vertices[..240],
        "only the tail bends with the tail angle"
    );
    assert_ne!(
        wild_mesh.vertices[tail.clone()],
        healthy_mesh.vertices[tail.clone()],
        "a healthy tame wolf's tail differs from the π/5 wild rest droop"
    );
    assert_ne!(
        healthy_mesh.vertices[tail.clone()],
        damaged_mesh.vertices[tail.clone()],
        "the tail bends further as health drops"
    );
}

#[test]
fn tame_wolf_droops_its_tail_with_damage_textured() {
    // The texture-backed base layer SETS the same `wolf_tail_angle` droop on the tail.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let wild = EntityModelInstance::wolf(151, [0.0, 64.0, 0.0], 0.0, false);
    let full = 0.55 * std::f32::consts::PI;
    let hurt = (0.55 - 0.8 * 0.4) * std::f32::consts::PI;
    let wild_mesh = entity_model_textured_mesh(&[wild], &atlas);
    let healthy_mesh = entity_model_textured_mesh(&[wild.with_wolf_tail_angle(full)], &atlas);
    let damaged_mesh = entity_model_textured_mesh(&[wild.with_wolf_tail_angle(hurt)], &atlas);
    let tail = 240..264;
    assert_eq!(
        wild_mesh.vertices[..240],
        damaged_mesh.vertices[..240],
        "only the tail bends with the tail angle"
    );
    assert_ne!(
        wild_mesh.vertices[tail.clone()],
        healthy_mesh.vertices[tail.clone()],
        "a healthy tame wolf's tail differs from the π/5 wild rest droop"
    );
    assert_ne!(
        healthy_mesh.vertices[tail.clone()],
        damaged_mesh.vertices[tail.clone()],
        "the tail bends further as health drops"
    );
}

#[test]
fn wolf_sitting_pose_matches_vanilla_set_sitting_pose() {
    // Vanilla `WolfModel.setSittingPose` (ageScale 1.0 adult / 0.5 baby; rotations SET):
    //   body:     y += 4*as,  z -= 2*as,  xRot = π/4  (baby: a further −π/2, → −π/4)
    //   hindLeg:  y += 6.7*as, z -= 5*as, xRot = 3π/2
    //   frontLeg: xRot = 5.811947, x += ±0.01*as (right +, left −), y += 1*as
    //   tail:     y += 9*as,  z -= 2*as  (offset only; xRot/yRot come from the tail pose)
    // The roles are now resolved by child name (the adult and baby trees name the same parts).
    assert_eq!(WOLF_SIT_FRONT_LEG_X_ROT, 5.811947);
    assert_eq!(
        wolf_sitting_part_roles(),
        [
            ("body", WolfSitPart::Body),
            ("right_hind_leg", WolfSitPart::HindLeg),
            ("left_hind_leg", WolfSitPart::HindLeg),
            ("right_front_leg", WolfSitPart::RightFrontLeg),
            ("left_front_leg", WolfSitPart::LeftFrontLeg),
            ("tail", WolfSitPart::Tail),
        ]
    );

    let base = PartPose {
        offset: [1.0, 2.0, 3.0],
        rotation: [0.1, 0.2, 0.3],
    };

    let mut body = base;
    apply_wolf_sitting_pose(&mut body, WolfSitPart::Body, false);
    assert_eq!(body.offset, [1.0, 6.0, 1.0]);
    assert_eq!(body.rotation, [std::f32::consts::FRAC_PI_4, 0.2, 0.3]);

    let mut baby_body = base;
    apply_wolf_sitting_pose(&mut baby_body, WolfSitPart::Body, true);
    assert_eq!(baby_body.offset, [1.0, 4.0, 2.0]);
    assert!(
        (baby_body.rotation[0] - (std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2)).abs()
            < 1e-6
    );

    let mut hind = base;
    apply_wolf_sitting_pose(&mut hind, WolfSitPart::HindLeg, false);
    assert_eq!(hind.offset, [1.0, 8.7, -2.0]);
    assert!((hind.rotation[0] - std::f32::consts::PI * 1.5).abs() < 1e-6);

    let mut right = base;
    apply_wolf_sitting_pose(&mut right, WolfSitPart::RightFrontLeg, false);
    assert!((right.offset[0] - 1.01).abs() < 1e-6);
    assert_eq!(right.offset[1], 3.0);
    assert_eq!(right.rotation[0], WOLF_SIT_FRONT_LEG_X_ROT);
    let mut left = base;
    apply_wolf_sitting_pose(&mut left, WolfSitPart::LeftFrontLeg, false);
    assert!((left.offset[0] - 0.99).abs() < 1e-6);

    // The baby front-leg x nudge scales by ageScale 0.5.
    let mut baby_right = base;
    apply_wolf_sitting_pose(&mut baby_right, WolfSitPart::RightFrontLeg, true);
    assert!((baby_right.offset[0] - 1.005).abs() < 1e-6);

    let mut tail = base;
    apply_wolf_sitting_pose(&mut tail, WolfSitPart::Tail, false);
    assert_eq!(tail.offset, [1.0, 11.0, 1.0]);
    assert_eq!(
        tail.rotation, base.rotation,
        "the tail rotation is left to the tail pose"
    );
}

#[test]
fn wolf_sits_folds_legs_and_tilts_body() {
    // Vanilla `WolfModel.setSittingPose` repositions the body, both hind legs, both front
    // legs, and the tail when `isSitting`; the head still follows the look (unchanged here).
    // The adult head subtree occupies vertices [0, 96) (the empty head part plus the
    // four-cube real head); the baby head is [0, 72) (one head cube plus two ears).
    // Colored path here; textured below.
    for (baby, head_end) in [(false, 96), (true, 72)] {
        let standing = EntityModelInstance::wolf(160, [0.0, 64.0, 0.0], 0.0, baby);
        let sitting = standing.with_wolf_sitting(true);
        let stand_mesh = entity_model_mesh(&[standing]);
        let sit_mesh = entity_model_mesh(&[sitting]);
        assert_eq!(
            stand_mesh.vertices.len(),
            sit_mesh.vertices.len(),
            "baby={baby}"
        );
        assert_eq!(
            stand_mesh.vertices[..head_end],
            sit_mesh.vertices[..head_end],
            "baby={baby}: the head is unchanged by sitting"
        );
        assert_ne!(
            stand_mesh.vertices[head_end..],
            sit_mesh.vertices[head_end..],
            "baby={baby}: the body, legs, and tail fold when sitting"
        );
    }
}

#[test]
fn wolf_textured_mesh_sits_folds_legs_and_tilts_body() {
    // The texture-backed render path folds into the same sitting pose.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for (baby, head_end) in [(false, 96), (true, 72)] {
        let standing = EntityModelInstance::wolf(161, [0.0, 64.0, 0.0], 0.0, baby);
        let sitting = standing.with_wolf_sitting(true);
        let stand_mesh = entity_model_textured_mesh(&[standing], &atlas);
        let sit_mesh = entity_model_textured_mesh(&[sitting], &atlas);
        assert_eq!(
            stand_mesh.vertices.len(),
            sit_mesh.vertices.len(),
            "baby={baby}"
        );
        assert_eq!(
            stand_mesh.vertices[..head_end],
            sit_mesh.vertices[..head_end],
            "baby={baby}: the head is unchanged by sitting"
        );
        assert_ne!(
            stand_mesh.vertices[head_end..],
            sit_mesh.vertices[head_end..],
            "baby={baby}: the body, legs, and tail fold when sitting"
        );
    }
}

#[test]
fn wolf_angry_tail_pose_matches_vanilla_get_tail_angle() {
    // Vanilla `WolfModel.setupAnim` for an angry wolf: `tail.yRot = 0` (no wag) and
    // `tail.xRot = getTailAngle() = 1.5393804` (the angry constant), overriding the layer's
    // π/5 wild rest droop. The offset and zRot are preserved.
    let base = ADULT_WOLF_TAIL_POSE;
    assert!(
        (base.rotation[0] - 0.62831855).abs() < 1e-6,
        "adult tail rests at the π/5 wild droop: {}",
        base.rotation[0]
    );
    assert_eq!(WOLF_ANGRY_TAIL_X_ROT, 1.5393804);

    let angry = wolf_angry_tail_pose(base);
    assert!(
        (angry.rotation[0] - 1.5393804).abs() < 1e-6,
        "angry tail raises to 1.5393804: {}",
        angry.rotation[0]
    );
    assert_eq!(angry.rotation[1], 0.0, "angry tail does not wag");
    assert_eq!(angry.rotation[2], base.rotation[2], "zRot preserved");
    assert_eq!(angry.offset, base.offset, "offset preserved");
}

#[test]
fn angry_wolf_raises_and_holds_its_tail_still() {
    // An angry wolf raises its tail (xRot 1.5393804, vs the π/5 wild rest droop) and holds
    // it straight: it does not wag when walking (`tail.yRot = 0`), unlike the non-angry
    // wolf, while the legs still swing. The colored adult layout lists head/body/mane at
    // vertices [0, 144), the four legs at [144, 240), and the tail child cube at [240, 264).
    let calm = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, false);
    let angry = EntityModelInstance::wolf_state(
        151,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        false,
        None,
    );
    let tail = 240..264;

    let calm_rest = entity_model_mesh(&[calm]);
    let angry_rest = entity_model_mesh(&[angry]);
    // The colored path tints both wolves with the same uniform colors, so only the raised
    // tail differs at rest; head/body/mane/legs are byte-identical.
    assert_eq!(
        calm_rest.vertices[..240],
        angry_rest.vertices[..240],
        "only the tail differs between a calm and an angry standing wolf"
    );
    assert_ne!(
        calm_rest.vertices[tail.clone()],
        angry_rest.vertices[tail.clone()],
        "the angry wolf raises its tail"
    );

    // Walking swings the legs but leaves the angry tail untouched (held straight, no wag).
    let angry_walking = entity_model_mesh(&[angry.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        angry_rest.vertices[tail.clone()],
        angry_walking.vertices[tail.clone()],
        "the angry tail is held still when walking (no wag)"
    );
    assert_ne!(
        angry_rest.vertices[144..240],
        angry_walking.vertices[144..240],
        "the angry wolf still swings its legs"
    );
}

#[test]
fn angry_wolf_textured_mesh_raises_and_holds_its_tail_still() {
    // The texture-backed angry wolf runs the same tail branch: the tail is raised and held
    // still (no wag) while the legs swing. Positions ignore the differing angry-texture UVs.
    let tail_positions = |verts: &[EntityModelTexturedVertex]| -> Vec<[f32; 3]> {
        verts[240..264].iter().map(|v| v.position).collect()
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let calm = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, false);
    let angry = EntityModelInstance::wolf_state(
        151,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        false,
        None,
    );

    let calm_rest = entity_model_textured_mesh(&[calm], &atlas);
    let angry_rest = entity_model_textured_mesh(&[angry], &atlas);
    assert_ne!(
        tail_positions(&calm_rest.vertices),
        tail_positions(&angry_rest.vertices),
        "the angry wolf raises its textured tail"
    );

    let angry_walking = entity_model_textured_mesh(&[angry.with_walk_animation(0.0, 1.0)], &atlas);
    assert_eq!(
        angry_rest.vertices[240..264],
        angry_walking.vertices[240..264],
        "the angry textured tail is held still when walking (no wag)"
    );
    assert_ne!(
        angry_rest.vertices[144..240],
        angry_walking.vertices[144..240],
        "the angry wolf still swings its legs in the textured path"
    );
}
