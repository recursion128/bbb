use super::*;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn vec3(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}

/// `UUIDUtil.uuidFromIntArray` over the four ints `Codec.INT_STREAM` reads.
fn saved_uuid() -> Uuid {
    Uuid::from_u128((1_u128 << 96) | (2_u128 << 64) | (3_u128 << 32) | 4_u128)
}

#[test]
fn the_codec_reads_and_writes_both_xor_sides() {
    let uuid_form = vec![entry(
        LEASH_TAG,
        NbtSummaryValue::Compound(vec![entry(
            "UUID",
            NbtSummaryValue::IntArray(vec![1, 2, 3, 4]),
        )]),
    )];
    let data = read_leash_data(&uuid_form).expect("a UUID holder decodes");
    assert_eq!(
        data.delayed_leash_info(),
        Some(LeashAttachment::Holder(saved_uuid()))
    );
    assert_eq!(data.leash_holder(), None);
    let mut written = Vec::new();
    write_leash_data(&mut written, Some(&data)).expect("a delayed holder re-encodes");
    assert_eq!(written, uuid_form);

    let knot_form = vec![entry(LEASH_TAG, NbtSummaryValue::IntArray(vec![7, -8, 9]))];
    let data = read_leash_data(&knot_form).expect("a knot position decodes");
    assert_eq!(
        data.delayed_leash_info(),
        Some(LeashAttachment::Knot(BlockPos { x: 7, y: -8, z: 9 }))
    );
    let mut written = Vec::new();
    write_leash_data(&mut written, Some(&data)).expect("a delayed knot re-encodes");
    assert_eq!(written, knot_form);
}

/// `storeNullable` writes nothing for a null `LeashData` (`Leashable.java:86`).
#[test]
fn an_absent_leash_field_reads_and_writes_as_nothing() {
    assert_eq!(read_leash_data(&[]), None);
    let mut written = Vec::new();
    write_leash_data(&mut written, None).expect("nothing to write");
    assert!(written.is_empty());
}

/// `Leashable.computeDampenedSpringInteraction` (`:253-269`): nothing inside
/// the slack, and outside it a displacement toward the pivot whose torque is
/// taken before the same-direction dampening scales it.
#[test]
fn the_spring_is_slack_below_its_length_and_dampens_along_the_motion() {
    assert_eq!(
        compute_dampened_spring_interaction(
            vec3(0.0, 0.0, 5.0),
            Vec3d::default(),
            LEASH_ELASTIC_DISTANCE,
            Vec3d::default(),
            vec3(1.0, 0.0, 0.0),
        ),
        None
    );

    // Ten out along +z with six of slack leaves a four-long displacement; the
    // lever arm is one along +x, so the torque is `1 * 4` with `:408`'s sign.
    let pulled = compute_dampened_spring_interaction(
        vec3(0.0, 0.0, 10.0),
        Vec3d::default(),
        LEASH_ELASTIC_DISTANCE,
        vec3(0.0, 0.0, -1.0),
        vec3(1.0, 0.0, 0.0),
    )
    .expect("ten is outside the slack");
    assert_eq!(pulled.torque, -4.0);
    assert_eq!(pulled.force, vec3(0.0, 0.0, 4.0));

    // The same pull with the object already moving that way keeps the torque
    // and scales only the force, by the float literal at `:265`.
    let dampened = compute_dampened_spring_interaction(
        vec3(0.0, 0.0, 10.0),
        Vec3d::default(),
        LEASH_ELASTIC_DISTANCE,
        vec3(0.0, 0.0, 1.0),
        vec3(1.0, 0.0, 0.0),
    )
    .expect("ten is outside the slack");
    assert_eq!(dampened.torque, -4.0);
    assert_eq!(dampened.force, vec3(0.0, 0.0, 4.0 * f64::from(0.3_f32)));
}

/// `Leashable.Wrench` (`:404-435`).
#[test]
fn wrenches_accumulate_componentwise_and_scale_together() {
    let accumulated = Wrench::accumulate(&[
        Wrench {
            force: vec3(1.0, 2.0, 3.0),
            torque: 0.5,
        },
        Wrench {
            force: vec3(-0.5, 0.0, 1.0),
            torque: -0.25,
        },
    ]);
    assert_eq!(accumulated.force, vec3(0.5, 2.0, 4.0));
    assert_eq!(accumulated.torque, 0.25);
    let quartered = accumulated.scale(0.25);
    assert_eq!(quartered.force, vec3(0.125, 0.5, 1.0));
    assert_eq!(quartered.torque, 0.0625);
    assert_eq!(
        Wrench::accumulate(&[]),
        Wrench {
            force: Vec3d::default(),
            torque: 0.0,
        }
    );
}

/// `Vec3.yRot` reads the `Mth` sine table, not `f64::sin`
/// (`Vec3.java:237-244`, `Mth.java:49-55`).
#[test]
fn the_attachment_rotation_uses_the_sine_table() {
    let quarter = y_rot(vec3(1.0, 5.0, 0.0), -std::f32::consts::FRAC_PI_2);
    assert_eq!(quarter.y, 5.0);
    assert_eq!(
        quarter.x,
        f64::from(bbb_server_types::geometry::mth_cos(f64::from(
            -std::f32::consts::FRAC_PI_2
        )))
    );
    assert_eq!(
        quarter.z,
        -f64::from(bbb_server_types::geometry::mth_sin(f64::from(
            -std::f32::consts::FRAC_PI_2
        )))
    );
}

mod player_pose;
