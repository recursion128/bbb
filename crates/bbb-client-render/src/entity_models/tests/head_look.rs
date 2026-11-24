use super::*;

#[test]
fn head_part_indices_match_vanilla_body_layers() {
    // The zombie adult layer lists the head first, its baby layer the body first
    // (head second). The per-family part tests assert that the part at this index is
    // the head. (The cow and pig now build named-children trees, so their head look
    // resolves the `head` child by name rather than an index.)
    // (The zombie family and piglin now build named-children trees, so their head look resolves
    // the `head` child by name rather than an index — the adult layers list the head first, the
    // baby layers list the body first, but both name the head `head`.)

    // (Every skeleton family — skeleton/stray/wither/bogged and the parched dual body layer — now
    // builds a named tree and resolves the head by name; the textured-only `SkeletonClothingModel`
    // stray frost / bogged mushroom overlay does too, so the skeleton head-part indices are gone.)

    // (The player now builds a named-children tree, so its head look resolves the `head` child by
    // name rather than an index; the wide and slim layouts both list the head first.)

    // Standalone head-first models list the head as part 0. (The creeper, enderman, spider, and now
    // the iron/snow golem build named-children trees, so their head look resolves the `head` child by
    // name rather than this index; the constant is retained for the remaining index-addressed
    // head-first layouts.)
    assert_eq!(head_first_part_index(), 0);

    // (The hoglin now builds a named-children tree, so its head look resolves the `head`
    // child by name rather than an index — the adult layer lists the body first, the baby
    // layer lists the head first, but both name the head `head`.)

    // (The ravager now builds a named-children tree, so its head look resolves the nested head
    // as `neck.child_mut("head")` by name rather than two positional indices.)
}

#[test]
fn head_look_yaw_pose_matches_vanilla_hoglin_setup_anim() {
    // HoglinModel.setupAnim sets head.yRot = yRot*PI/180 but keeps head.xRot at the
    // headbutt-rest tilt baked into the base pose, so yaw-only look preserves xRot
    // and zRot.
    let base = PartPose {
        offset: [0.0, 2.0, -12.0],
        rotation: [HOGLIN_HEAD_X_ROT, 0.0, 0.2],
    };
    assert!(head_yaw_at_rest(0.0));
    assert!(!head_yaw_at_rest(10.0));

    let posed = head_look_yaw_pose(base, 35.0);
    assert_eq!(posed.offset, base.offset);
    // Only the yaw is set; the base headbutt-rest pitch and zRot are untouched.
    assert_eq!(posed.rotation[0], HOGLIN_HEAD_X_ROT);
    assert!((posed.rotation[1] - 35.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(posed.rotation[2], 0.2);

    // No yaw turn returns the base pose unchanged.
    assert_eq!(head_look_yaw_pose(base, 0.0), base);
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
