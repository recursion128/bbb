use super::*;

#[test]
fn head_part_indices_match_vanilla_body_layers() {
    // The cow body layer lists the head first for both ages; the pig adult and
    // zombie adult layers list the head first, their baby layers list the body
    // first (head second). The per-family part tests assert that the part at this
    // index is the head.
    assert_eq!(cow_head_part_index(false), 0);
    assert_eq!(cow_head_part_index(true), 0);
    assert_eq!(pig_head_part_index(false), 0);
    assert_eq!(pig_head_part_index(true), 1);
    assert_eq!(zombie_head_part_index(false), 0);
    assert_eq!(zombie_head_part_index(true), 1);
    assert_eq!(piglin_head_part_index(false), 0);
    assert_eq!(piglin_head_part_index(true), 1);
    assert_eq!(
        ADULT_PIGLIN_PARTS[piglin_head_part_index(false)].cubes,
        ADULT_PIGLIN_HEAD.as_slice()
    );
    assert_eq!(
        BABY_PIGLIN_PARTS[piglin_head_part_index(true)].cubes,
        BABY_PIGLIN_HEAD.as_slice()
    );

    // Skeleton/stray/wither/bogged list the head first; parched lists the body
    // first (head second). Tie the indices to the actual head parts so the
    // surprising parched ordering can't silently regress.
    assert_eq!(skeleton_head_part_index(), 0);
    assert_eq!(parched_head_part_index(), 1);
    assert_eq!(
        SKELETON_PARTS[skeleton_head_part_index()].cubes,
        SKELETON_HEAD.as_slice()
    );
    assert_eq!(
        PARCHED_PARTS[parched_head_part_index()].cubes,
        PARCHED_HEAD.as_slice()
    );
    assert_eq!(
        BOGGED_PARTS[skeleton_head_part_index()].cubes,
        BOGGED_HEAD.as_slice()
    );

    // Adult villager/wandering-trader/witch/illager list the head first; the
    // baby villager layout lists arms/legs first, with the head at index 3.
    assert_eq!(villager_head_part_index(false), 0);
    assert_eq!(villager_head_part_index(true), 3);
    assert_eq!(
        ADULT_VILLAGER_PARTS[villager_head_part_index(false)].cubes,
        ADULT_VILLAGER_HEAD.as_slice()
    );
    assert_eq!(
        BABY_VILLAGER_PARTS[villager_head_part_index(true)].cubes,
        BABY_VILLAGER_HEAD.as_slice()
    );
    assert_eq!(
        WITCH_PARTS[villager_head_part_index(false)].cubes,
        WITCH_HEAD.as_slice()
    );
    assert_eq!(
        ILLAGER_SHARED_CROSSED_PARTS[villager_head_part_index(false)].cubes,
        ILLAGER_HEAD.as_slice()
    );

    // The wide and slim player layers both list the head first.
    assert_eq!(player_head_part_index(), 0);
    assert_eq!(
        PLAYER_WIDE_PARTS[player_head_part_index()].cubes,
        PLAYER_HEAD.as_slice()
    );
    assert_eq!(
        PLAYER_SLIM_PARTS[player_head_part_index()].cubes,
        PLAYER_HEAD.as_slice()
    );
}

#[test]
fn head_look_pose_matches_vanilla_setup_anim() {
    // QuadrupedModel/HumanoidModel.setupAnim: head.xRot = xRot*PI/180,
    // head.yRot = yRot*PI/180.
    let base = PartPose {
        offset: [0.0, 4.0, -8.0],
        rotation: [0.0, 0.0, 0.3],
    };
    assert!(head_look_at_rest(0.0, 0.0));
    assert!(!head_look_at_rest(10.0, 0.0));
    assert!(!head_look_at_rest(0.0, 10.0));

    let posed = head_look_pose(base, 40.0, -18.0);
    // The pivot offset is untouched; the look angles set the head rotation.
    assert_eq!(posed.offset, base.offset);
    assert!((posed.rotation[0] - (-18.0_f32).to_radians()).abs() < 1e-6);
    assert!((posed.rotation[1] - 40.0_f32.to_radians()).abs() < 1e-6);
    // The base zRot is preserved (vanilla setupAnim only assigns xRot/yRot).
    assert_eq!(posed.rotation[2], 0.3);

    // No look turn returns the base pose unchanged.
    assert_eq!(head_look_pose(base, 0.0, 0.0), base);
}

#[test]
fn pig_colored_mesh_applies_head_look_to_head_only() {
    let base =
        EntityModelInstance::pig(90, [0.0, 64.0, 0.0], 0.0, PigModelVariant::Temperate, false);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(60.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -30.0)]);

    // Adult pig head part is index 0: its two cubes are the first 48 vertices.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices[0..48], yawed.vertices[0..48]);
    assert_eq!(resting.vertices[48..], yawed.vertices[48..]);
    assert_ne!(resting.vertices[0..48], pitched.vertices[0..48]);
    assert_eq!(resting.vertices[48..], pitched.vertices[48..]);
    // Yaw and pitch are distinct head rotations.
    assert_ne!(yawed.vertices[0..48], pitched.vertices[0..48]);
}

#[test]
fn baby_pig_colored_mesh_turns_the_head_part_not_the_body() {
    let base =
        EntityModelInstance::pig(91, [0.0, 64.0, 0.0], 0.0, PigModelVariant::Temperate, true);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(60.0, -20.0)]);

    // Baby pig lists the body first (index 0, one cube = first 24 vertices) and
    // the head second (index 1). Head look must leave the body untouched.
    assert_ne!(resting.vertices, looking.vertices);
    assert_eq!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn cow_colored_mesh_applies_head_look_to_head_only() {
    let base = EntityModelInstance::cow(601, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(45.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -25.0)]);

    // Adult cow head part is index 0: its four cubes are the first 96 vertices.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices[0..96], yawed.vertices[0..96]);
    assert_eq!(resting.vertices[96..], yawed.vertices[96..]);
    assert_ne!(resting.vertices[0..96], pitched.vertices[0..96]);
    assert_eq!(resting.vertices[96..], pitched.vertices[96..]);
    assert_ne!(yawed.vertices[0..96], pitched.vertices[0..96]);
}

#[test]
fn zombie_colored_mesh_applies_head_look_to_head_only() {
    let base = EntityModelInstance::zombie(700, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(50.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -20.0)]);

    // The zombie head (with its hat children) is part 0, emitted first; the last
    // part is a leg, which head look must leave untouched.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], yawed.vertices[n - 24..]);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn baby_zombie_colored_mesh_turns_head_part_not_body() {
    let base = EntityModelInstance::zombie(701, [0.0, 64.0, 0.0], 0.0, true);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Baby zombie lists the body first (index 0, one cube = first 24 vertices)
    // and the head second (index 1). Head look must leave the body untouched.
    assert_ne!(resting.vertices, looking.vertices);
    assert_eq!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn zombie_villager_variant_colored_mesh_applies_head_look() {
    let base = EntityModelInstance::zombie_variant(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        false,
    );
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -15.0)]);

    // The variant emitter routes through the same head-look helper; the last
    // part (a leg) stays put while the head turns.
    assert_eq!(resting.vertices.len(), looking.vertices.len());
    assert_ne!(resting.vertices, looking.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], looking.vertices[n - 24..]);
}

#[test]
fn piglin_colored_mesh_applies_head_look_to_head_only() {
    let base =
        EntityModelInstance::piglin(720, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(50.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -20.0)]);

    // Adult piglin head (with its ear children) is part 0, emitted first; the
    // last part is a leg, which head look must leave untouched.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], yawed.vertices[n - 24..]);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn baby_piglin_colored_mesh_turns_head_part_not_body() {
    let base =
        EntityModelInstance::piglin(721, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, true);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Baby piglin lists the body first (index 0); the head is index 1. Head look
    // must leave the leading body cube untouched.
    assert_ne!(resting.vertices, looking.vertices);
    assert_eq!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn baby_piglin_brute_colored_mesh_uses_adult_head_index() {
    // A baby piglin brute renders the adult layout (head at index 0), so head
    // look must turn the leading head cube, unlike a baby piglin.
    let base = EntityModelInstance::piglin(
        722,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::PiglinBrute,
        true,
    );
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    assert_ne!(resting.vertices, looking.vertices);
    assert_ne!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn skeleton_colored_mesh_applies_head_look_to_head_only() {
    let base = EntityModelInstance::skeleton(710, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(50.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -20.0)]);

    // Skeleton head (with its hat children) is part 0, emitted first; the last
    // part is a leg, which head look must leave untouched.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], yawed.vertices[n - 24..]);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn parched_skeleton_colored_mesh_turns_head_part_not_body() {
    let base = EntityModelInstance::skeleton_variant(
        711,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    );
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Parched lists the body first (index 0); the head is index 1. Head look
    // must leave the leading body cube untouched.
    assert_ne!(resting.vertices, looking.vertices);
    assert_eq!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn wither_skeleton_colored_mesh_applies_head_look_with_scaled_transform() {
    let base = EntityModelInstance::skeleton_variant(
        713,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::WitherSkeleton,
    );
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Wither skeleton uses the scaled model root transform; head look still
    // turns the head (part 0) and leaves the trailing leg untouched.
    assert_eq!(resting.vertices.len(), looking.vertices.len());
    assert_ne!(resting.vertices, looking.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], looking.vertices[n - 24..]);
}

#[test]
fn villager_colored_mesh_applies_head_look_to_head_only() {
    let base = EntityModelInstance::villager(730, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_mesh(&[base]);
    let yawed = entity_model_mesh(&[base.with_head_look(50.0, 0.0)]);
    let pitched = entity_model_mesh(&[base.with_head_look(0.0, -20.0)]);

    // Adult villager head (with its nose child) is part 0, emitted first; the
    // last part is a leg, which head look must leave untouched.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], yawed.vertices[n - 24..]);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn baby_villager_colored_mesh_turns_head_part_not_legs() {
    let base = EntityModelInstance::villager(731, [0.0, 64.0, 0.0], 0.0, true);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Baby villager lists an (empty) arms container then legs first; the head is
    // index 3. The first emitted cubes are a leg, which head look must not move.
    assert_ne!(resting.vertices, looking.vertices);
    assert_eq!(resting.vertices[0..24], looking.vertices[0..24]);
}

#[test]
fn wandering_trader_colored_mesh_applies_head_look() {
    let base = EntityModelInstance::wandering_trader(732, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    assert_eq!(resting.vertices.len(), looking.vertices.len());
    assert_ne!(resting.vertices, looking.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], looking.vertices[n - 24..]);
}

#[test]
fn witch_colored_mesh_applies_head_look() {
    let base = EntityModelInstance::witch(733, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

    // Witch head (with nose and wart children) is part 0; the last part is a leg.
    assert_eq!(resting.vertices.len(), looking.vertices.len());
    assert_ne!(resting.vertices, looking.vertices);
    let n = resting.vertices.len();
    assert_eq!(resting.vertices[n - 24..], looking.vertices[n - 24..]);
}

#[test]
fn player_colored_mesh_applies_head_look_to_head_only() {
    for slim in [false, true] {
        let base = EntityModelInstance::player(740, [0.0, 64.0, 0.0], 0.0, slim);
        let resting = entity_model_mesh(&[base]);
        let yawed = entity_model_mesh(&[base.with_head_look(50.0, 0.0)]);
        let pitched = entity_model_mesh(&[base.with_head_look(0.0, -20.0)]);

        // Player head (with its hat child) is part 0, emitted first; the last
        // part is a leg, which head look must leave untouched.
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "slim={slim} head turns");
        let n = resting.vertices.len();
        assert_eq!(
            resting.vertices[n - 24..],
            yawed.vertices[n - 24..],
            "slim={slim} leg stays"
        );
        assert_ne!(yawed.vertices, pitched.vertices, "slim={slim} yaw != pitch");
    }
}

#[test]
fn illager_colored_mesh_applies_head_look_across_families() {
    for family in [
        IllagerModelFamily::Evoker,
        IllagerModelFamily::Illusioner,
        IllagerModelFamily::Pillager,
        IllagerModelFamily::Vindicator,
    ] {
        let base = EntityModelInstance::illager(734, [0.0, 64.0, 0.0], 0.0, family);
        let resting = entity_model_mesh(&[base]);
        let looking = entity_model_mesh(&[base.with_head_look(50.0, -20.0)]);

        // Illager head is part 0; head look turns it and leaves the trailing leg.
        assert_eq!(resting.vertices.len(), looking.vertices.len());
        assert_ne!(resting.vertices, looking.vertices, "{family:?} head turns");
        let n = resting.vertices.len();
        assert_eq!(
            resting.vertices[n - 24..],
            looking.vertices[n - 24..],
            "{family:?} legs stay"
        );
    }
}
