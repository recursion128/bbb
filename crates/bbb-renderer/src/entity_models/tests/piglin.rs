use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn piglin_model_parts_match_vanilla_26_1_body_layers() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. The piglin builds a named-children tree (`head` -> ears, `body`,
    // the arms/legs with their sleeve/pants overlays), so the head look resolves the `head` child by
    // name; the geometry is asserted on the per-part cube consts directly.
    assert_eq!(
        ADULT_PIGLIN_HEAD,
        [
            ModelCube::new(
                [-5.0, -8.0, -4.0],
                [10.0, 8.0, 8.0],
                PIGLIN_SKIN,
                [10.0, 8.0, 8.0],
                [0.0, 0.0],
                false,
            ),
            ModelCube::new(
                [-2.0, -4.0, -5.0],
                [4.0, 4.0, 1.0],
                PIGLIN_SKIN,
                [4.0, 4.0, 1.0],
                [31.0, 1.0],
                false,
            ),
            ModelCube::new(
                [2.0, -2.0, -5.0],
                [1.0, 2.0, 1.0],
                PIGLIN_SKIN,
                [1.0, 2.0, 1.0],
                [2.0, 4.0],
                false,
            ),
            ModelCube::new(
                [-3.0, -2.0, -5.0],
                [1.0, 2.0, 1.0],
                PIGLIN_SKIN,
                [1.0, 2.0, 1.0],
                [2.0, 0.0],
                false,
            ),
        ]
    );
    // The adult ears, body, and the inflated sleeve/pants overlays (which keep the base box as
    // `uv_size`).
    assert_eq!(ADULT_PIGLIN_LEFT_EAR[0].size, [1.0, 5.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_RIGHT_EAR[0].size, [1.0, 5.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_BODY[0].size, [8.0, 12.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_RIGHT_ARM[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_RIGHT_SLEEVE[0].size, [4.5, 12.5, 4.5]);
    assert_eq!(ADULT_PIGLIN_RIGHT_SLEEVE[0].uv_size, [4.0, 12.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert_eq!(ADULT_PIGLIN_RIGHT_PANTS[0].size, [4.5, 12.5, 4.5]);
    assert_eq!(ADULT_PIGLIN_RIGHT_PANTS[0].uv_size, [4.0, 12.0, 4.0]);

    // The baby layout's smaller head/snout, ears, body, and the un-sleeved arms/legs.
    assert_eq!(BABY_PIGLIN_HEAD[1].size, [9.0, 6.0, 7.0]);
    assert_eq!(BABY_PIGLIN_LEFT_EAR[0].size, [1.0, 6.0, 4.0]);
    assert_eq!(BABY_PIGLIN_RIGHT_EAR[0].size, [1.0, 6.0, 4.0]);
    assert_eq!(BABY_PIGLIN_BODY[0].size, [6.0, 5.0, 3.0]);
    assert_eq!(BABY_PIGLIN_LEFT_ARM[0].size, [2.0, 5.0, 3.0]);
    assert_eq!(BABY_PIGLIN_RIGHT_LEG[0].size, [3.0, 4.0, 3.0]);
}

#[test]
fn piglin_meshes_use_vanilla_body_layer_geometry() {
    let piglin = entity_model_mesh(&[EntityModelInstance::piglin(
        101,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        false,
    )]);
    assert_eq!(piglin.opaque_faces, 90);
    assert_eq!(piglin.vertices.len(), 360);
    assert_eq!(piglin.indices.len(), 540);
    assert!(piglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
    // The always-on HumanoidModel idle arm bob rolls the resting arms (zRot ±0.1 at
    // ageInTicks 0), widening the adult X extent from ±0.515625 to ±0.578566. The baby's
    // wider head/body still bounds its X, so the baby extents below are unchanged.
    let (piglin_min, piglin_max) = mesh_extents(&piglin);
    assert_close3(piglin_min, [-0.578566, 63.985374, -0.25000003]);
    assert_close3(piglin_max, [0.578566, 66.001, 0.31250003]);

    let baby_piglin = entity_model_mesh(&[EntityModelInstance::piglin(
        101,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        true,
    )]);
    assert_eq!(baby_piglin.opaque_faces, 54);
    assert_eq!(baby_piglin.vertices.len(), 216);
    assert_eq!(baby_piglin.indices.len(), 324);
    assert!(baby_piglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_SKIN, 0.78)));
    // Vanilla runs `setupAnim` every frame, so the ears always carry the flap baseline
    // (`±default ∓ cos(freq) * 0.08`, here `freq = 0` at rest). On the small baby body the
    // flapped ear holders reach slightly past the body half-width, widening the X extent
    // from the un-flapped layer box; the larger adult body still encloses its ears.
    let (baby_piglin_min, baby_piglin_max) = mesh_extents(&baby_piglin);
    assert_close3(baby_piglin_min, [-0.4962139, 64.001, -0.21875003]);
    assert_close3(baby_piglin_max, [0.4962139, 64.9385, 0.28125]);

    let brute = entity_model_mesh(&[EntityModelInstance::piglin(
        102,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::PiglinBrute,
        false,
    )]);
    assert_same_geometry(&brute, &piglin);
    assert!(brute
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIGLIN_BRUTE_SKIN, 0.78)));

    let zombified = entity_model_mesh(&[EntityModelInstance::piglin(
        154,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        false,
    )]);
    // The zombified piglin reuses the regular piglin's body-layer model (same cubes, faces,
    // and indices), but vanilla overrides its arms with the held-out `animateZombieArms`
    // pose, which we defer to the bind rest — so, unlike the regular piglin and the brute, it
    // does not apply the always-on idle arm bob, and its rendered arm vertices differ.
    assert_same_structure(&zombified, &piglin);
    assert_ne!(
        zombified.vertices, piglin.vertices,
        "the zombified piglin's deferred arms stay at rest while the regular piglin's bob"
    );
    assert!(zombified
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));

    let baby_zombified = entity_model_mesh(&[EntityModelInstance::piglin(
        154,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        true,
    )]);
    // The baby zombified piglin likewise shares the baby piglin's model but defers its arm
    // pose, so it does not bob while the baby piglin does.
    assert_same_structure(&baby_zombified, &baby_piglin);
    assert_ne!(
        baby_zombified.vertices, baby_piglin.vertices,
        "the baby zombified piglin's deferred arms stay at rest while the baby piglin's bob"
    );
    assert!(baby_zombified
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOMBIFIED_PIGLIN_SKIN, 0.78)));
}

#[test]
fn piglin_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/piglin_brute.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/zombified_piglin.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/piglin/zombified_piglin_baby.png",
            size: [64, 64],
        })
    );
}

#[test]
fn piglin_family_swings_its_arms_when_walking() {
    // `AbstractPiglinModel extends HumanoidModel`, so `super.setupAnim` gives the default
    // arms the inherited counter-swing `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`
    // (the arms are overridden only by `PiglinModel`'s deferred dance/attack/crossbow/
    // admire poses). The zombified piglin instead overwrites the arms via
    // `AnimationUtils.animateZombieArms` (the held-out zombie pose, deferred), so its arms
    // must stay at rest. In the adult layer (15 cubes) the head/snout/ears and body fill
    // 24-vertex blocks [0, 7); the two arms (each a cube plus its sleeve child) fill blocks
    // [7, 11) = vertices [168, 264); the legs fill [11, 15). The baby layer's arms (no
    // sleeve children) fill blocks [5, 7) = vertices [120, 168).
    let z_extent = |verts: &[EntityModelVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let arm_slice = |baby: bool| -> std::ops::Range<usize> {
        if baby {
            120..168
        } else {
            168..264
        }
    };
    // Families whose default arms swing: adult and baby piglin, and the brute (which
    // reuses the adult piglin body layer).
    let swings: [(&str, PiglinModelFamily, bool); 3] = [
        ("piglin", PiglinModelFamily::Piglin, false),
        ("piglin_baby", PiglinModelFamily::Piglin, true),
        ("piglin_brute", PiglinModelFamily::PiglinBrute, false),
    ];
    for (name, family, baby) in swings {
        let base = EntityModelInstance::piglin(95, [0.0, 64.0, 0.0], 0.0, family, baby);
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let arms = arm_slice(baby);
        assert_ne!(
            rest.vertices[arms.clone()],
            walking.vertices[arms.clone()],
            "{name}: arms swing when walking"
        );
        let rest_arm_z = z_extent(&rest.vertices[arms.clone()]);
        let walk_arm_z = z_extent(&walking.vertices[arms.clone()]);
        assert!(
            walk_arm_z > rest_arm_z + 0.1,
            "{name}: a forward/back arm swing deepens the arm Z footprint: {rest_arm_z} -> {walk_arm_z}"
        );
    }
    // The zombified piglin overwrites its arms with the deferred zombie pose, so the arm
    // region is byte-identical between standing and walking — only its legs swing.
    for (name, baby) in [("zombified_piglin", false), ("zombified_piglin_baby", true)] {
        let base = EntityModelInstance::piglin(
            96,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::ZombifiedPiglin,
            baby,
        );
        let rest = entity_model_mesh(&[base]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        let arms = arm_slice(baby);
        assert_eq!(
            rest.vertices[arms.clone()],
            walking.vertices[arms.clone()],
            "{name}: the deferred zombie arm pose keeps the arms at rest"
        );
        assert_ne!(
            rest.vertices, walking.vertices,
            "{name}: the legs still swing"
        );
    }
}

#[test]
fn piglin_family_swings_its_legs_when_walking() {
    // `AbstractPiglinModel extends HumanoidModel`: its `setupAnim` runs
    // `super.setupAnim` (the inherited leg swing) then sways only the ears, so the
    // piglin family inherits the `HumanoidModel` legs unchanged (the default arm swing
    // is covered by `piglin_family_swings_its_arms_when_walking`). A standing piglin is
    // inert; a walking one lifts its feet (a shorter model) and splays along Z, for
    // every family and the baby layout. The ear sway and override arm poses are deferred.
    let instances: [(&str, EntityModelInstance); 5] = [
        (
            "piglin",
            EntityModelInstance::piglin(
                90,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::Piglin,
                false,
            ),
        ),
        (
            "piglin_baby",
            EntityModelInstance::piglin(91, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, true),
        ),
        (
            "piglin_brute",
            EntityModelInstance::piglin(
                92,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::PiglinBrute,
                false,
            ),
        ),
        (
            "zombified_piglin",
            EntityModelInstance::piglin(
                93,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                false,
            ),
        ),
        (
            "zombified_piglin_baby",
            EntityModelInstance::piglin(
                94,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                true,
            ),
        ),
    ];
    for (name, base) in instances {
        let rest = entity_model_mesh(&[base]);
        // A standing piglin keeps its legs at rest at speed 0 regardless of `pos`. (Its ears
        // always flap — see `piglin_family_flaps_its_ears` — so the full mesh is not
        // byte-identical at speed 0; the ear flap is a `zRot` sway that never touches the Z
        // extent, so the legs' Z splay isolates the gait.)
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (still_min, still_max) = mesh_extents(&still);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            ((still_max[2] - still_min[2]) - (rest_max[2] - rest_min[2])).abs() < 1e-4,
            "{name}: a standing piglin keeps its legs unsplayed at speed 0"
        );
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{name}: a walking piglin's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{name}: a walking piglin's legs should splay along Z"
        );
    }
}

#[test]
fn piglin_ear_flap_pose_matches_vanilla_formula() {
    // Vanilla `AbstractPiglinModel.setupAnim`:
    //   freq = ageInTicks * 0.1 + pos * 0.5;  amp = 0.08 + speed * 0.4;
    //   leftEar.zRot  = -default - cos(freq * 1.2) * amp;
    //   rightEar.zRot =  default + cos(freq)       * amp.
    // The default ear angle is 30° (adult) or 5° (baby), in radians.
    assert!((PIGLIN_ADULT_EAR_ANGLE - std::f32::consts::FRAC_PI_6).abs() < 1e-6);
    assert!((PIGLIN_BABY_EAR_ANGLE - 5.0 * std::f32::consts::PI / 180.0).abs() < 1e-9);

    let default = PIGLIN_ADULT_EAR_ANGLE;
    let left_base = PartPose {
        offset: [4.5, -6.0, 0.0],
        rotation: [0.0, 0.0, -default],
    };
    let right_base = PartPose {
        offset: [-4.5, -6.0, 0.0],
        rotation: [0.0, 0.0, default],
    };

    // Standing (age 0, pos 0, speed 0): freq 0, amp 0.08, so the ears carry the ±0.08
    // baseline flap on top of the default angle.
    let left = piglin_ear_flap_pose(left_base, true, default, 0.0, 0.0, 0.0);
    assert!(
        (left.rotation[2] - (-default - 0.08)).abs() < 1e-6,
        "{}",
        left.rotation[2]
    );
    let right = piglin_ear_flap_pose(right_base, false, default, 0.0, 0.0, 0.0);
    assert!(
        (right.rotation[2] - (default + 0.08)).abs() < 1e-6,
        "{}",
        right.rotation[2]
    );

    // A general (age, pos, speed) reproduces the formula, including the left ear's ×1.2
    // frequency and the speed-scaled amplitude.
    let (age, pos, speed) = (40.0_f32, 1.5_f32, 0.6_f32);
    let freq = age * 0.1 + pos * 0.5;
    let amp = 0.08 + speed * 0.4;
    let left = piglin_ear_flap_pose(left_base, true, default, age, pos, speed);
    assert!((left.rotation[2] - (-default - (freq * 1.2).cos() * amp)).abs() < 1e-6);
    let right = piglin_ear_flap_pose(right_base, false, default, age, pos, speed);
    assert!((right.rotation[2] - (default + freq.cos() * amp)).abs() < 1e-6);

    // The offset and the untouched xRot/yRot are preserved.
    assert_eq!(left.offset, left_base.offset);
    assert_eq!(left.rotation[0], left_base.rotation[0]);
    assert_eq!(left.rotation[1], left_base.rotation[1]);

    // The baby ear uses the 5° default angle.
    let baby_left = piglin_ear_flap_pose(left_base, true, PIGLIN_BABY_EAR_ANGLE, 0.0, 0.0, 0.0);
    assert!((baby_left.rotation[2] - (-PIGLIN_BABY_EAR_ANGLE - 0.08)).abs() < 1e-6);
}

#[test]
fn piglin_family_flaps_its_ears() {
    // Vanilla runs `AbstractPiglinModel.setupAnim` every frame (every subclass calls
    // `super.setupAnim`), so the ears flap continuously — driven by `ageInTicks` even when
    // the piglin stands still. Advancing `ageInTicks` re-poses only the ears, so the mesh
    // changes while the (age-independent) legs hold still. Covers every family and the baby
    // layout, in the colored render path (the textured path's ear flap is covered separately).
    for (name, base) in [
        (
            "piglin",
            EntityModelInstance::piglin(
                201,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::Piglin,
                false,
            ),
        ),
        (
            "baby_piglin",
            EntityModelInstance::piglin(
                202,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::Piglin,
                true,
            ),
        ),
        (
            "brute",
            EntityModelInstance::piglin(
                203,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::PiglinBrute,
                false,
            ),
        ),
        (
            "zombified",
            EntityModelInstance::piglin(
                204,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                false,
            ),
        ),
        (
            "baby_zombified",
            EntityModelInstance::piglin(
                205,
                [0.0, 64.0, 0.0],
                0.0,
                PiglinModelFamily::ZombifiedPiglin,
                true,
            ),
        ),
    ] {
        let early = entity_model_mesh(&[base]);
        let later = entity_model_mesh(&[base.with_age_in_ticks(31.4)]);
        assert_eq!(early.vertices.len(), later.vertices.len(), "{name}");
        assert_ne!(
            early.vertices, later.vertices,
            "{name}: the ears flap as ageInTicks advances"
        );
        // The legs carry no age term, so the final leg cube is byte-identical.
        let leg_tail = early.vertices.len() - 24;
        assert_eq!(
            early.vertices[leg_tail..],
            later.vertices[leg_tail..],
            "{name}: the legs do not depend on ageInTicks"
        );
    }
}

#[test]
fn piglin_textured_parts_match_vanilla_body_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PIGLIN, "minecraft:piglin#main");
    assert_eq!(MODEL_LAYER_PIGLIN_BABY, "minecraft:piglin_baby#main");
    assert_eq!(MODEL_LAYER_PIGLIN_BRUTE, "minecraft:piglin_brute#main");
    assert_eq!(
        MODEL_LAYER_ZOMBIFIED_PIGLIN,
        "minecraft:zombified_piglin#main"
    );
    assert_eq!(
        MODEL_LAYER_ZOMBIFIED_PIGLIN_BABY,
        "minecraft:zombified_piglin_baby#main"
    );

    // Adult: `AbstractPiglinModel.addHead` head/snout/nostril UVs + ears, the `texOffs(16, 16)`
    // body (no jacket), and the shared `PlayerModel` wide arm/sleeve/leg/pants UVs. The unified
    // cubes carry the `texOffs` on the `.tex` field.
    assert_eq!(ADULT_PIGLIN_HEAD[0].tex, [0.0, 0.0]); // head
    assert_eq!(ADULT_PIGLIN_HEAD[0].uv_size, [10.0, 8.0, 8.0]);
    assert_eq!(ADULT_PIGLIN_HEAD[1].tex, [31.0, 1.0]); // snout
    assert_eq!(ADULT_PIGLIN_HEAD[2].tex, [2.0, 4.0]); // nostril
    assert_eq!(ADULT_PIGLIN_HEAD[3].tex, [2.0, 0.0]); // nostril
    assert_eq!(ADULT_PIGLIN_LEFT_EAR[0].tex, [51.0, 6.0]); // left ear
    assert_eq!(ADULT_PIGLIN_RIGHT_EAR[0].tex, [39.0, 6.0]); // right ear
    assert_eq!(ADULT_PIGLIN_BODY[0].tex, [16.0, 16.0]); // body (no jacket)
    assert_eq!(ADULT_PIGLIN_RIGHT_ARM[0].tex, [40.0, 16.0]); // right arm
    assert_eq!(ADULT_PIGLIN_RIGHT_SLEEVE[0].tex, [40.0, 32.0]); // right sleeve
    assert_eq!(ADULT_PIGLIN_LEFT_ARM[0].tex, [32.0, 48.0]); // left arm
    assert_eq!(ADULT_PIGLIN_LEFT_SLEEVE[0].tex, [48.0, 48.0]); // left sleeve
    assert_eq!(ADULT_PIGLIN_RIGHT_LEG[0].tex, [0.0, 16.0]); // right leg
    assert_eq!(ADULT_PIGLIN_RIGHT_PANTS[0].tex, [0.0, 32.0]); // right pants
    assert_eq!(ADULT_PIGLIN_LEFT_LEG[0].tex, [16.0, 48.0]); // left leg
    assert_eq!(ADULT_PIGLIN_LEFT_PANTS[0].tex, [0.0, 48.0]); // left pants

    // Baby: `BabyPiglinModel.createBodyLayer`. The smaller body, the snout + head, the two flapping
    // ears, and the un-sleeved arms/legs.
    assert_eq!(BABY_PIGLIN_BODY[0].tex, [0.0, 13.0]); // body
    assert_eq!(BABY_PIGLIN_HEAD[0].tex, [21.0, 30.0]); // snout
    assert_eq!(BABY_PIGLIN_HEAD[1].tex, [0.0, 0.0]); // head
    assert_eq!(BABY_PIGLIN_LEFT_EAR[0].tex, [0.0, 21.0]); // left ear
    assert_eq!(BABY_PIGLIN_RIGHT_EAR[0].tex, [18.0, 13.0]); // right ear
    assert_eq!(BABY_PIGLIN_LEFT_ARM[0].tex, [28.0, 13.0]); // left arm
    assert_eq!(BABY_PIGLIN_RIGHT_ARM[0].tex, [10.0, 30.0]); // right arm
    assert_eq!(BABY_PIGLIN_RIGHT_LEG[0].tex, [22.0, 23.0]); // right leg
    assert_eq!(BABY_PIGLIN_LEFT_LEG[0].tex, [10.0, 23.0]); // left leg
}

#[test]
fn piglin_textured_layer_passes_match_vanilla_renderer() {
    let cases = [
        (
            PiglinModelFamily::Piglin,
            false,
            "minecraft:piglin#main",
            PIGLIN_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::Piglin,
            true,
            "minecraft:piglin_baby#main",
            PIGLIN_BABY_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::PiglinBrute,
            false,
            "minecraft:piglin_brute#main",
            PIGLIN_BRUTE_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::ZombifiedPiglin,
            false,
            "minecraft:zombified_piglin#main",
            ZOMBIFIED_PIGLIN_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::ZombifiedPiglin,
            true,
            "minecraft:zombified_piglin_baby#main",
            ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
        ),
    ];
    for (family, baby, model_layer, texture) in cases {
        let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
        let passes = piglin_textured_layer_passes(family, baby_layout);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::PiglinBase);
        assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert!(entity_model_texture_refs().contains(&texture));
        // The unified `PiglinModel` tree drives the geometry, so the layer-pass parts are vestigial.
    }
    // The brute is never baby: its baby flag still selects the adult layer + brute texture.
    let brute_baby = piglin_textured_layer_passes(PiglinModelFamily::PiglinBrute, false);
    assert_eq!(brute_baby[0].texture, PIGLIN_BRUTE_TEXTURE_REF);
    assert_eq!(
        piglin_entity_texture_refs(),
        &[
            PIGLIN_TEXTURE_REF,
            PIGLIN_BABY_TEXTURE_REF,
            PIGLIN_BRUTE_TEXTURE_REF,
            ZOMBIFIED_PIGLIN_TEXTURE_REF,
            ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
        ]
    );
}

#[test]
fn piglin_textured_mesh_matches_colored_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&piglin_texture_images()).unwrap();
    let cases = [
        (PiglinModelFamily::Piglin, false),
        (PiglinModelFamily::Piglin, true),
        (PiglinModelFamily::PiglinBrute, false),
        (PiglinModelFamily::ZombifiedPiglin, false),
        (PiglinModelFamily::ZombifiedPiglin, true),
    ];
    for (family, baby) in cases {
        let instances = [EntityModelInstance::piglin(
            90,
            [0.0, 64.0, 0.0],
            0.0,
            family,
            baby,
        )];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured piglin shares the colored geometry exactly (same flapped ears at age 0).
        assert_eq!(
            textured.cutout_faces, colored.opaque_faces,
            "{family:?} {baby}"
        );
        assert_eq!(
            textured.vertices.len(),
            colored.vertices.len(),
            "{family:?}"
        );
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        let (cmin, cmax) = mesh_extents(&colored);
        let (tmin, tmax) = textured_mesh_extents(&textured);
        assert_close3(tmin, cmin);
        assert_close3(tmax, cmax);

        // The ears flap as ageInTicks advances on the textured path too.
        let aged = [instances[0].with_age_in_ticks(31.4)];
        let textured_aged = entity_model_textured_mesh(&aged, &atlas);
        assert_ne!(textured.vertices, textured_aged.vertices, "{family:?} ears");
    }

    // Non-zombified piglins swing their arms when walking; the zombified piglin holds them out.
    let piglin =
        EntityModelInstance::piglin(90, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false);
    let piglin_rest = entity_model_textured_mesh(&[piglin], &atlas);
    let piglin_walk = entity_model_textured_mesh(&[piglin.with_walk_animation(0.0, 1.0)], &atlas);
    assert_ne!(
        piglin_rest.vertices, piglin_walk.vertices,
        "the textured piglin swings its arms/legs when walking"
    );
}

#[test]
fn dancing_piglin_raises_its_arms_and_bobs() {
    use std::f32::consts::PI;

    // Vanilla `PiglinModel.setupAnim` DANCING (`Piglin.isDancing()` → DATA_IS_DANCING): the head/body
    // bob, both arms raise overhead (`rightArm.zRot = (70 + cos(dancePos·40)·10)°`, the left mirrored)
    // and wag with `ageInTicks`, and the ears sway. `dancePos = ageInTicks / 60`. We compare a dancing
    // model to an idle one at the same age so the bob is read as the delta over the shared bind pose.
    let deg = PI / 180.0;
    let age = 48.0_f32;
    let dance_pos = age / 60.0;
    let bob = (dance_pos * 40.0).sin();

    let base =
        EntityModelInstance::piglin(91, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with_age_in_ticks(age);
    let mut idle = PiglinModel::new(PiglinModelFamily::Piglin, false);
    idle.prepare(&base);
    let mut dancing = PiglinModel::new(PiglinModelFamily::Piglin, false);
    dancing.prepare(&base.with_piglin_dancing(true));

    // Both arms raise overhead, the left mirroring the right's `zRot`.
    let right_zrot = deg * (70.0 + (dance_pos * 40.0).cos() * 10.0);
    let danced_right = dancing.root_mut().child_mut("right_arm").pose;
    assert!(
        (danced_right.rotation[2] - right_zrot).abs() < 1.0e-6,
        "the right arm raises to ~80°: {}",
        danced_right.rotation[2]
    );
    let danced_left_zrot = dancing.root_mut().child_mut("left_arm").pose.rotation[2];
    assert!(
        (danced_left_zrot + right_zrot).abs() < 1.0e-6,
        "the left arm mirrors the right: {danced_left_zrot}"
    );

    // The bob offsets add onto the bind pose (read as the dancing − idle delta).
    let idle_head = idle.root_mut().child_mut("head").pose.offset;
    let danced_head = dancing.root_mut().child_mut("head").pose.offset;
    assert!((danced_head[0] - idle_head[0] - (dance_pos * 10.0).sin()).abs() < 1.0e-6);
    assert!((danced_head[1] - idle_head[1] - (bob + 0.4)).abs() < 1.0e-6);

    let idle_ry = idle.root_mut().child_mut("right_arm").pose.offset[1];
    let danced_ry = dancing.root_mut().child_mut("right_arm").pose.offset[1];
    assert!((danced_ry - idle_ry - (bob * 0.5 - 0.5)).abs() < 1.0e-6);
    let idle_ly = idle.root_mut().child_mut("left_arm").pose.offset[1];
    let danced_ly = dancing.root_mut().child_mut("left_arm").pose.offset[1];
    assert!((danced_ly - idle_ly - (bob * 0.5 + 0.5)).abs() < 1.0e-6);
    let idle_by = idle.root_mut().child_mut("body").pose.offset[1];
    let danced_by = dancing.root_mut().child_mut("body").pose.offset[1];
    assert!((danced_by - idle_by - bob * 0.35).abs() < 1.0e-6);

    // The dance overwrites the idle ear flap with its own sway.
    let danced_right_ear = dancing
        .root_mut()
        .child_mut("head")
        .child_mut("right_ear")
        .pose
        .rotation[2];
    assert!(
        (danced_right_ear - (PI / 6.0 + deg * (dance_pos * 30.0).sin() * 10.0)).abs() < 1.0e-6,
        "the right ear sways with the dance: {danced_right_ear}"
    );

    // The full mesh visibly changes, and advancing the dance keeps animating it.
    let dancing_mesh = entity_model_mesh(&[base.with_piglin_dancing(true)]);
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        dancing_mesh.vertices,
        "a dancing piglin no longer stands idle"
    );
    let later = entity_model_mesh(&[base.with_piglin_dancing(true).with_age_in_ticks(age + 7.0)]);
    assert_ne!(
        dancing_mesh.vertices, later.vertices,
        "the dance keeps wagging as ageInTicks advances"
    );
}

fn piglin_texture_images() -> Vec<EntityModelTextureImage> {
    piglin_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
