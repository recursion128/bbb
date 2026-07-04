use super::*;

#[test]
fn entity_model_sources_project_chicken_wing_flap() {
    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
    };
    // Drives the chicken's synced ground flag (vanilla `Chicken.aiStep` reads
    // `onGround()`); the position stays put so only the flap state evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 80,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // An unticked chicken is frozen at the bind pose (wings held).
    let resting = source(&store, 1.0);
    assert_eq!(resting.chicken_flap, 0.0);
    assert_eq!(resting.chicken_flap_speed, 0.0);

    // Airborne: vanilla `flapSpeed += 4.0 * 0.3 = 1.2` (clamped to 1) jumps straight
    // to the clamp in a single tick, and `flap += flapping * 2` advances each tick.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(1);
    let air_one = source(&store, 1.0);
    assert!(
        (air_one.chicken_flap_speed - 1.0).abs() < 1.0e-6,
        "an airborne chicken saturates flap speed at 1 in one tick: {}",
        air_one.chicken_flap_speed
    );
    assert!(
        air_one.chicken_flap > 0.0,
        "an airborne chicken advances its flap phase: {}",
        air_one.chicken_flap
    );

    store.advance_entity_client_animations(1);
    let air_two = source(&store, 1.0);
    assert!(
        (air_two.chicken_flap_speed - 1.0).abs() < 1.0e-6,
        "flap speed holds at the clamp while airborne: {}",
        air_two.chicken_flap_speed
    );
    assert!(
        air_two.chicken_flap > air_one.chicken_flap,
        "the flap phase keeps advancing across ticks"
    );

    // The flap speed is sitting at 1; land and let vanilla `flapSpeed += -1.0 * 0.3`
    // pull it back toward 0 on the ground.
    let airborne_peak = air_two.chicken_flap_speed;
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(1);
    let grounded = source(&store, 1.0);
    assert!(
        grounded.chicken_flap_speed < airborne_peak,
        "landing drops the flap speed toward 0: {} -> {}",
        airborne_peak,
        grounded.chicken_flap_speed
    );

    // The lerped getters track the partial tick between the previous and current
    // flap endpoints (vanilla `ChickenRenderer.extractRenderState`).
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(3);
    let at_zero = source(&store, 0.0).chicken_flap;
    let at_half = source(&store, 0.5).chicken_flap;
    let at_one = source(&store, 1.0).chicken_flap;
    assert!(
        at_zero < at_half && at_half < at_one,
        "partial tick lerps the flap phase: {at_zero} < {at_half} < {at_one}"
    );
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints"
    );
}

#[test]
fn entity_model_sources_project_slime_squish_from_ground_transitions() {
    let squish = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
            .slime_squish
    };
    // Drives the slime's synced ground flag (vanilla `Slime.tick` reads `onGround()`
    // for the squish target); the position stays put so only the squish evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 81,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_SLIME_ID,
    ));

    // An unticked slime holds its undeformed cube (squish 0).
    assert_eq!(squish(&store, 1.0), 0.0);

    // Land from rest: vanilla seeds `targetSquish = -0.5` on the takeoff→ground
    // transition (then decays it by `0.6`), and the next tick eases `squish` toward
    // that negative target — the landing flatten/splat.
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(2);
    let landed = squish(&store, 1.0);
    assert!(
        landed < 0.0,
        "landing flattens the slime (negative squish): {landed}"
    );

    // Take off: vanilla seeds `targetSquish = 1.0` on the ground→air transition, and
    // the squish eases up through zero into the positive vertical stretch.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(2);
    let airborne = squish(&store, 1.0);
    assert!(
        airborne > 0.0,
        "a jumping slime stretches vertically (positive squish): {airborne}"
    );
    assert!(
        airborne > landed,
        "takeoff lifts the squish above the landing splat: {landed} -> {airborne}"
    );

    // The lerped getter tracks the partial tick between the previous and current
    // squish endpoints (vanilla `SlimeRenderer.extractRenderState`).
    let at_zero = squish(&store, 0.0);
    let at_one = squish(&store, 1.0);
    assert_ne!(
        at_zero, at_one,
        "the squish is still evolving across this tick"
    );
    let at_half = squish(&store, 0.5);
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints: {at_zero} .. {at_half} .. {at_one}"
    );
}

#[test]
fn entity_model_sources_project_parrot_wing_flap() {
    let flap_angle = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .parrot_flap_angle
    };
    // Drives the parrot's synced ground flag (vanilla `Parrot.calculateFlapping` reads
    // `onGround()`); the position stays put so only the flap state evolves.
    let set_on_ground = |store: &mut WorldStore, on_ground: bool| {
        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 90,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground,
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_PARROT_ID,
    ));

    // An unticked parrot is frozen at the bind pose (wings held): `flapAngle == 0`.
    assert_eq!(flap_angle(&store, 1.0), 0.0);

    // Airborne: vanilla `flapSpeed += 4.0 * 0.3 = 1.2` (clamped to 1) saturates in one tick, and
    // `flap += flapping * 2` advances the phase, so `flapAngle = (sin(flap) + 1) * flapSpeed > 0`.
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(1);
    let air_one = flap_angle(&store, 1.0);
    assert!(
        air_one > 0.0,
        "an airborne parrot develops a non-zero flap angle: {air_one}"
    );

    store.advance_entity_client_animations(1);
    let air_two = flap_angle(&store, 1.0);
    assert!(
        air_two > 0.0,
        "the flap angle stays live across airborne ticks: {air_two}"
    );

    // Land: vanilla `flapSpeed += -1.0 * 0.3` pulls the speed back toward 0 on the ground, and after
    // it bleeds to 0 the flap angle collapses to 0 (wings settle).
    set_on_ground(&mut store, true);
    store.advance_entity_client_animations(20);
    assert_eq!(
        flap_angle(&store, 1.0),
        0.0,
        "a grounded parrot settles its wings (flapSpeed -> 0)"
    );

    // The lerped getter tracks the partial tick between the previous and current flap angle
    // endpoints (vanilla `ParrotRenderer.extractRenderState` lerps flap+flapSpeed, then combines).
    set_on_ground(&mut store, false);
    store.advance_entity_client_animations(3);
    let at_zero = flap_angle(&store, 0.0);
    let at_one = flap_angle(&store, 1.0);
    assert_ne!(
        at_zero, at_one,
        "the projected flap angle changes across the partial tick: {at_zero} vs {at_one}"
    );
}

#[test]
fn entity_model_sources_project_parrot_party_from_playing_jukebox() {
    let jukebox = ProtocolBlockPos { x: 1, y: 64, z: -2 };
    let parrot_party = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .parrot_party
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_PARROT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_COD_ID,
    ));

    assert!(!parrot_party(&store, 91));

    // Vanilla `LevelEventHandler.playJukeboxSong` notifies nearby living entities, and
    // `Parrot.aiStep` keeps PARTY while `BlockPos.closerToCenterThan(entity.position(), 3.46)`.
    store.apply_level_event(ProtocolLevelEvent {
        event_type: 1010,
        pos: jukebox,
        data: 12,
        global: false,
    });
    assert!(parrot_party(&store, 91));
    assert!(
        !parrot_party(&store, 92),
        "the same active jukebox does not mark non-parrot sources as PARTY"
    );

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 91,
            position: ProtocolVec3d {
                x: 6.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    assert!(
        !parrot_party(&store, 91),
        "moving outside the vanilla 3.46 block-center radius clears PARTY"
    );

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 91,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    assert!(parrot_party(&store, 91));

    store.apply_level_event(ProtocolLevelEvent {
        event_type: 1011,
        pos: jukebox,
        data: 0,
        global: false,
    });
    assert!(
        !parrot_party(&store, 91),
        "the stop event removes the active jukebox song and clears PARTY"
    );
}

#[test]
fn parrot_passenger_holds_its_wings() {
    // Vanilla `Parrot.calculateFlapping` gates the airborne flap build-up on `!onGround() &&
    // !isPassenger()`. A parrot riding a vehicle (its `vehicle_id` set) is a passenger, so even
    // airborne its `flapSpeed` decays toward 0 and `flapAngle` stays at 0 (wings settled).

    let flap_angle = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
            .parrot_flap_angle
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_PARROT_ID,
    ));
    // Mark the parrot airborne — without the passenger gate this would flap.
    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 91,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }));
    // Seat the parrot on the boat so it becomes a passenger.
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 92,
        passenger_ids: vec![91],
    }));

    store.advance_entity_client_animations(5);
    assert_eq!(
        flap_angle(&store),
        0.0,
        "an airborne passenger parrot keeps its wings settled"
    );
}

#[test]
fn entity_model_sources_project_bee_roll_amount() {
    // Vanilla `Bee.DATA_FLAGS_ID` is synced data id 18; `FLAG_ROLL` is mask 2 within that byte.
    let bee_flags = |raw: i8| ProtocolEntityDataValue {
        data_id: 18,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw),
    };
    let roll = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
            .bee_roll_amount
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_BEE_ID,
    ));

    // An upright (un-rolling) bee projects `0.0`.
    assert_eq!(roll(&store, 1.0), 0.0);

    // Setting `FLAG_ROLL` makes vanilla `Bee.updateRollAmount` climb `rollAmount` by `0.2`/tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![bee_flags(2)],
    }));
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.4).abs() < 1.0e-6);

    // It saturates at `1.0` (vanilla `Math.min(1.0, …)`): three more ticks reach 1.0 and hold.
    store.advance_entity_client_animations(5);
    assert!((roll(&store, 1.0) - 1.0).abs() < 1.0e-6);

    // Clearing the flag decays it by `0.24`/tick (vanilla `Math.max(0.0, …)`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![bee_flags(0)],
    }));
    store.advance_entity_client_animations(1);
    assert!((roll(&store, 1.0) - 0.76).abs() < 1.0e-6);

    // The projected getter lerps across the partial tick (vanilla `Bee.getRollAmount`).
    let at_zero = roll(&store, 0.0);
    let at_half = roll(&store, 0.5);
    let at_one = roll(&store, 1.0);
    assert!(
        at_one < at_zero,
        "the decaying roll falls from previous to current"
    );
    assert!((at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-6);
}

#[test]
fn entity_model_sources_project_panda_sit_lie_and_roll_amounts() {
    const AGEABLE_BABY_DATA_ID: u8 = 16;
    // Vanilla `Panda.DATA_ID_FLAGS` id 23: roll=4, sitting=8, onBack=16.
    const PANDA_FLAGS_DATA_ID: u8 = 23;
    const PANDA_FLAG_ROLLING: i8 = 0x04;
    const PANDA_FLAG_SITTING: i8 = 0x08;
    const PANDA_FLAG_ON_BACK: i8 = 0x10;
    let panda_flags = |raw: i8| ProtocolEntityDataValue {
        data_id: PANDA_FLAGS_DATA_ID,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw),
    };
    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_PANDA_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_PANDA_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        84,
        VANILLA_ENTITY_TYPE_PANDA_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));

    let rest = source(&store, 82, 1.0);
    assert_eq!(rest.panda_sit_amount, 0.0);
    assert_eq!(rest.panda_lie_on_back_amount, 0.0);
    assert_eq!(rest.panda_roll_amount, 0.0);
    assert_eq!(rest.panda_roll_time, 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![panda_flags(
            PANDA_FLAG_SITTING | PANDA_FLAG_ON_BACK | PANDA_FLAG_ROLLING
        )],
    }));
    store.advance_entity_client_animations(1);
    let adult_half = source(&store, 82, 0.5);
    assert!((adult_half.panda_sit_amount - 0.075).abs() < 1.0e-6);
    assert!((adult_half.panda_lie_on_back_amount - 0.075).abs() < 1.0e-6);
    assert!((adult_half.panda_roll_amount - 0.075).abs() < 1.0e-6);
    assert!((adult_half.panda_roll_time - 1.5).abs() < 1.0e-6);
    store.advance_entity_client_animations(1);
    let adult = source(&store, 82, 1.0);
    assert!((adult.panda_sit_amount - 0.30).abs() < 1.0e-6);
    assert!((adult.panda_lie_on_back_amount - 0.30).abs() < 1.0e-6);
    assert!((adult.panda_roll_amount - 0.30).abs() < 1.0e-6);
    assert!((adult.panda_roll_time - 3.0).abs() < 1.0e-6);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![panda_flags(0)],
    }));
    store.advance_entity_client_animations(1);
    let adult_decay_start = source(&store, 82, 0.0);
    let adult_decay_end = source(&store, 82, 1.0);
    assert!((adult_decay_start.panda_sit_amount - 0.30).abs() < 1.0e-6);
    assert!((adult_decay_end.panda_sit_amount - 0.11).abs() < 1.0e-6);
    assert_eq!(
        adult_decay_end.panda_roll_time, 0.0,
        "clearing the roll flag resets rollCounter on the next client tick"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![panda_flags(PANDA_FLAG_ROLLING)],
    }));
    store.advance_entity_client_animations(1);
    let baby = source(&store, 83, 0.5);
    assert_eq!(
        baby.panda_roll_amount, 0.0,
        "PandaRenderer.extractRenderState forces baby rollAmount to 0"
    );
    assert!(
        (baby.panda_roll_time - 1.5).abs() < 1.0e-6,
        "baby pandas still tumble via rollTime"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        values: vec![panda_flags(PANDA_FLAG_ROLLING)],
    }));
    store.advance_entity_client_animations(33);
    assert!(
        source(&store, 84, 0.5).panda_roll_time > 32.0,
        "the local roll clear happens after the tick-33 render sample"
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        // Unrelated data sync must not resurrect the old stored flags byte after
        // vanilla `handleRoll` has locally called `roll(false)`.
        values: vec![protocol_int_data(18, 1)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 84, 0.5).panda_roll_time, 0.0);
}

#[test]
fn entity_model_sources_project_frog_croak_seconds() {
    // Vanilla `Pose.CROAKING(8, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated` starts
    // `croakAnimationState` when the pose becomes CROAKING and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_CROAKING_ID: i32 = 8;
    let croak = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_croak_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.CROAKING` projects the `-1.0` stopped sentinel (pouch hidden).
    assert_eq!(croak(&store, 1.0), -1.0);

    // Entering `Pose.CROAKING` starts the timer at the current age, so the elapsed seconds begin at
    // `0` (plus the partial tick): vanilla `((ageInTicks - startTick)) / 20`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_CROAKING_ID)],
    }));
    assert!((croak(&store, 0.0) - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((croak(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05` (the age climbs, the start
    // tick is fixed).
    store.advance_entity_client_animations(1);
    assert!((croak(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((croak(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.CROAKING` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(croak(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_frog_tongue_seconds() {
    // Vanilla `Pose.USING_TONGUE(9, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated`
    // starts `tongueAnimationState` when the pose becomes USING_TONGUE and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_USING_TONGUE_ID: i32 = 9;
    let tongue = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_tongue_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.USING_TONGUE` projects the `-1.0` stopped sentinel (no lash).
    assert_eq!(tongue(&store, 1.0), -1.0);

    // Entering `Pose.USING_TONGUE` starts the timer at the current age: vanilla `(ageInTicks -
    // startTick) / 20`, so the elapsed seconds begin at `0` (plus the partial tick).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_USING_TONGUE_ID)],
    }));
    assert!((tongue(&store, 0.0) - 0.0).abs() < 1.0e-6);
    assert!((tongue(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05`.
    store.advance_entity_client_animations(1);
    assert!((tongue(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((tongue(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.USING_TONGUE` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(tongue(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_frog_jump_seconds() {
    // Vanilla `Pose.LONG_JUMPING(6, …)` synced via `DATA_POSE` (id 6); `Frog.onSyncedDataUpdated`
    // starts `jumpAnimationState` when the pose becomes LONG_JUMPING and stops it otherwise.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;
    let jump = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 55)
            .unwrap()
            .frog_jump_seconds
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));

    // A frog not in `Pose.LONG_JUMPING` projects the `-1.0` stopped sentinel (no keyframe applied).
    assert_eq!(jump(&store, 1.0), -1.0);

    // Entering `Pose.LONG_JUMPING` starts the timer at the current age, so the elapsed seconds begin
    // at `0` (plus the partial tick): vanilla `((ageInTicks - startTick)) / 20`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_LONG_JUMPING_ID)],
    }));
    assert!((jump(&store, 0.0) - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((jump(&store, 0.5) - 0.025).abs() < 1.0e-6);

    // Each client tick advances the elapsed seconds by `1 / 20 = 0.05` (the age climbs, the start
    // tick is fixed).
    store.advance_entity_client_animations(1);
    assert!((jump(&store, 0.0) - 0.05).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((jump(&store, 0.0) - 0.25).abs() < 1.0e-6);

    // Leaving `Pose.LONG_JUMPING` stops the animation, returning the `-1.0` sentinel.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    assert_eq!(jump(&store, 1.0), -1.0);
}

#[test]
fn entity_model_sources_project_sniffer_state_animation() {
    // Vanilla `Sniffer.DATA_STATE` (id 18), the `Sniffer.State` ordinal VarInt;
    // `Sniffer.onSyncedDataUpdated` `resetAnimations()` then starts the matching one-shot.
    const SNIFFER_STATE_DATA_ID: u8 = 18;
    const SNIFFER_STATE_IDLING_ID: i32 = 0;
    const SNIFFER_STATE_SNIFFING_ID: i32 = 3;
    const SNIFFER_STATE_SEARCHING_ID: i32 = 4;
    const SNIFFER_STATE_DIGGING_ID: i32 = 5;
    let animation = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 119)
            .unwrap();
        (
            source.sniffer_animation_id,
            source.sniffer_animation_seconds,
        )
    };
    let is_searching = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 119)
            .unwrap()
            .sniffer_is_searching
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        119,
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    ));

    // An idling sniffer projects the `(-1, -1.0)` no-animation sentinel and is not searching.
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(!is_searching(&store));

    // Entering `DIGGING` starts the dig one-shot at the current age: the id is the `DIGGING` ordinal
    // and the elapsed seconds begin at `0` (plus the partial tick), advancing `1 / 20` per tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_DIGGING_ID
        )],
    }));
    let (id, seconds) = animation(&store, 0.0);
    assert_eq!(id, SNIFFER_STATE_DIGGING_ID);
    assert!((seconds - 0.0).abs() < 1.0e-6);
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((animation(&store, 0.5).1 - 0.025).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert_eq!(animation(&store, 0.0), (SNIFFER_STATE_DIGGING_ID, 0.2));

    // Changing to a different animated state restarts the timer from `0` (vanilla `resetAnimations()`
    // + `startIfStopped` on the transition) and switches the id to the new state.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_SNIFFING_ID
        )],
    }));
    assert_eq!(animation(&store, 0.0), (SNIFFER_STATE_SNIFFING_ID, 0.0));

    // `SEARCHING` carries no one-shot (it drives the looping search-walk), so it clears to the
    // no-animation sentinel — but `sniffer_is_searching` flips true to swap in the search-walk.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_SEARCHING_ID
        )],
    }));
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(is_searching(&store));

    // Returning to `IDLING` likewise stays cleared and is no longer searching.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 119,
        values: vec![protocol_enum_data(
            SNIFFER_STATE_DATA_ID,
            EntityDataEnumSerializer::SnifferState,
            SNIFFER_STATE_IDLING_ID
        )],
    }));
    assert_eq!(animation(&store, 1.0), (-1, -1.0));
    assert!(!is_searching(&store));
}

#[test]
fn entity_model_sources_project_armadillo_state_animation() {
    // Vanilla `Armadillo.ARMADILLO_STATE` (id 18), the `ArmadilloState` id VarInt (serializer 36).
    // `Armadillo.setupAnimationStates` `.startIfStopped`s rollUp into ROLLING / rollOut into
    // UNROLLING, and `shouldHideInShell(inStateTicks)` gates the shell-ball swap.
    const ARMADILLO_STATE_DATA_ID: u8 = 18;
    const ARMADILLO_STATE_ROLLING_ID: i32 = 1;
    const ARMADILLO_STATE_SCARED_ID: i32 = 2;
    const ARMADILLO_STATE_UNROLLING_ID: i32 = 3;
    const ARMADILLO_PEEK_EVENT_ID: i8 = 64;
    let project = |store: &WorldStore| {
        let source = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 4)
            .unwrap();
        (
            source.armadillo_is_hiding_in_shell,
            source.armadillo_roll_up_seconds,
            source.armadillo_roll_out_seconds,
            source.armadillo_peek_seconds,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        4,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    ));

    // An IDLE armadillo (no state synced) is unrolled with no transition timers.
    assert_eq!(project(&store), (false, -1.0, -1.0, -1.0));

    // Entering ROLLING starts the roll-up timer at the current age (elapsed `0`) and does NOT yet
    // hide: vanilla `ROLLING.shouldHideInShell(inStateTicks) = inStateTicks > 5`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 4,
        values: vec![protocol_enum_data(
            ARMADILLO_STATE_DATA_ID,
            EntityDataEnumSerializer::ArmadilloState,
            ARMADILLO_STATE_ROLLING_ID,
        )],
    }));
    let (hiding, roll_up, roll_out, peek) = project(&store);
    assert!(!hiding, "rolling does not hide until inStateTicks > 5");
    assert!((roll_up - 0.0).abs() < 1.0e-6, "roll-up starts at 0s");
    assert_eq!((roll_out, peek), (-1.0, -1.0));

    // The roll-up elapsed seconds advance `1 / 20` per client tick.
    store.advance_entity_client_animations(5);
    assert!((project(&store).1 - 0.25).abs() < 1.0e-6);
    // At inStateTicks == 5 it still does not hide (`> 5` is strict); the next tick flips it true.
    assert!(!project(&store).0, "inStateTicks == 5 is not yet hiding");
    store.advance_entity_client_animations(1);
    assert!(
        project(&store).0,
        "inStateTicks == 6 hides the body in the shell"
    );
    // The roll-up keeps advancing past the hide (vanilla applies it regardless of hiding).
    assert!((project(&store).1 - 0.3).abs() < 1.0e-6);

    // Entering SCARED starts `peekAnimationState` and immediately fast-forwards it by the state's
    // 50-tick animation duration. The shell stays hidden for the whole SCARED state.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 4,
        values: vec![protocol_enum_data(
            ARMADILLO_STATE_DATA_ID,
            EntityDataEnumSerializer::ArmadilloState,
            ARMADILLO_STATE_SCARED_ID,
        )],
    }));
    let (hiding, roll_up, roll_out, peek) = project(&store);
    assert!(hiding, "SCARED always hides in the shell");
    assert_eq!((roll_up, roll_out), (-1.0, -1.0));
    assert!(
        (peek - 2.5).abs() < 1.0e-6,
        "first SCARED setup fast-forwards peek by 50 ticks"
    );
    store.advance_entity_client_animations(1);
    assert!(
        (project(&store).3 - 2.55).abs() < 1.0e-6,
        "peek keeps advancing while SCARED"
    );

    // Vanilla entity event 64 sets `peekReceivedClient`; the next setup tick stops the old
    // fast-forwarded peek and immediately restarts it from 0.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 4,
        event_id: ARMADILLO_PEEK_EVENT_ID,
    }));
    assert!(
        (project(&store).3 - 2.55).abs() < 1.0e-6,
        "the event is consumed by the next setup tick, not synchronously"
    );
    store.advance_entity_client_animations(1);
    assert!(
        (project(&store).3 - 0.0).abs() < 1.0e-6,
        "event 64 restarts peek on the next tick"
    );
    store.advance_entity_client_animations(1);
    assert!(
        (project(&store).3 - 0.05).abs() < 1.0e-6,
        "restarted peek advances from the new tick"
    );

    // Entering UNROLLING restarts: the roll-out timer starts at 0, the roll-up stops, and the body
    // stays hidden while `inStateTicks < 26` (`UNROLLING.shouldHideInShell`); peek stops.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 4,
        values: vec![protocol_enum_data(
            ARMADILLO_STATE_DATA_ID,
            EntityDataEnumSerializer::ArmadilloState,
            ARMADILLO_STATE_UNROLLING_ID,
        )],
    }));
    let (hiding, roll_up, roll_out, peek) = project(&store);
    assert!(
        hiding,
        "unrolling keeps the ball until inStateTicks reaches 26"
    );
    assert_eq!(roll_up, -1.0, "the roll-up timer stops on the transition");
    assert!((roll_out - 0.0).abs() < 1.0e-6, "roll-out starts at 0s");
    assert_eq!(peek, -1.0, "the peek timer stops on the transition");

    // The body stays hidden through inStateTicks 25, then un-hides at 26.
    store.advance_entity_client_animations(25);
    assert!(project(&store).0, "inStateTicks == 25 is still hiding");
    assert!(
        (project(&store).2 - 1.25).abs() < 1.0e-6,
        "roll-out advanced 25 ticks"
    );
    store.advance_entity_client_animations(1);
    assert!(!project(&store).0, "inStateTicks == 26 un-hides the body");
}

#[test]
fn entity_model_sources_project_warden_combat_animations() {
    // Vanilla `Pose.ROARING(11)` / `Pose.SNIFFING(12)` synced via `DATA_POSE` (id 6);
    // `Warden.onSyncedDataUpdated` `.start()`s the matching one-shot when the pose CHANGES to it.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_ROARING_ID: i32 = 11;
    const VANILLA_POSE_SNIFFING_ID: i32 = 12;
    // Vanilla `Warden.handleEntityEvent`: id 4 starts the attack (and stops the roar); id 62 starts
    // the sonic boom.
    const WARDEN_ATTACK_EVENT_ID: i8 = 4;
    const WARDEN_SONIC_BOOM_EVENT_ID: i8 = 62;
    let combat = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 142)
            .unwrap();
        (
            source.warden_roar_seconds,
            source.warden_sniff_seconds,
            source.warden_attack_seconds,
            source.warden_sonic_boom_seconds,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        142,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));

    // A warden in no triggered pose with no event projects all `-1.0` stopped sentinels.
    assert_eq!(combat(&store, 1.0), (-1.0, -1.0, -1.0, -1.0));

    // Entering `Pose.ROARING` starts the roar timer at the current age: the elapsed seconds begin at
    // `0` (plus the partial tick), advancing `1 / 20` per tick. Only the roar activates.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_ROARING_ID)],
    }));
    let (roar, sniff, attack, sonic) = combat(&store, 0.0);
    assert!((roar - 0.0).abs() < 1.0e-6);
    assert_eq!((sniff, attack, sonic), (-1.0, -1.0, -1.0));
    // The partial tick folds into the live age (`(0 + 0.5) / 20`).
    assert!((combat(&store, 0.5).0 - 0.025).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((combat(&store, 0.0).0 - 0.2).abs() < 1.0e-6);

    // Leaving `Pose.ROARING` does NOT stop the roar (vanilla never auto-stops on pose leave); the
    // non-looping keyframe just holds its final frame, so the timer keeps advancing.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    store.advance_entity_client_animations(1);
    assert!((combat(&store, 0.0).0 - 0.25).abs() < 1.0e-6);

    // Event 4 starts the attack AND stops the roar (vanilla `roarAnimationState.stop()` +
    // `attackAnimationState.start()`).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 142,
        event_id: WARDEN_ATTACK_EVENT_ID,
    }));
    let (roar, _, attack, _) = combat(&store, 0.0);
    assert_eq!(roar, -1.0, "the attack event stops the roar");
    assert!((attack - 0.0).abs() < 1.0e-6, "the attack starts at 0");
    store.advance_entity_client_animations(2);
    assert!((combat(&store, 0.0).2 - 0.1).abs() < 1.0e-6);

    // Event 62 starts the sonic boom independently (the attack keeps holding its final frame).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 142,
        event_id: WARDEN_SONIC_BOOM_EVENT_ID,
    }));
    let (_, _, attack, sonic) = combat(&store, 0.0);
    assert!((sonic - 0.0).abs() < 1.0e-6, "the sonic boom starts at 0");
    assert!((attack - 0.1).abs() < 1.0e-6, "the attack still holds");

    // Entering `Pose.SNIFFING` starts the sniff timer; the other three keep their running timers.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_SNIFFING_ID)],
    }));
    let (_, sniff, _, _) = combat(&store, 0.0);
    assert!((sniff - 0.0).abs() < 1.0e-6, "the sniff starts at 0");
}

#[test]
fn entity_model_sources_project_warden_emerge_and_dig() {
    // Vanilla `Pose.EMERGING(13)` / `Pose.DIGGING(14)` synced via `DATA_POSE` (id 6); like the
    // roar/sniff poses, `Warden.onSyncedDataUpdated` `.start()`s the spawn/despawn one-shot when the
    // pose CHANGES to it. These are the 6.68s `WARDEN_EMERGE` and 5.0s `WARDEN_DIG` keyframes.
    const VANILLA_POSE_STANDING_ID: i32 = 0;
    const VANILLA_POSE_EMERGING_ID: i32 = 13;
    const VANILLA_POSE_DIGGING_ID: i32 = 14;
    let spawn = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 142)
            .unwrap();
        (source.warden_emerge_seconds, source.warden_dig_seconds)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        142,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));

    // A warden in no triggered pose projects the `-1.0` stopped sentinels.
    assert_eq!(spawn(&store, 1.0), (-1.0, -1.0));

    // Entering `Pose.EMERGING` starts the emerge timer at the current age (elapsed begins at `0`,
    // plus the partial tick, advancing `1 / 20` per tick). The dig stays stopped.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_EMERGING_ID)],
    }));
    let (emerge, dig) = spawn(&store, 0.0);
    assert!((emerge - 0.0).abs() < 1.0e-6, "the emerge starts at 0");
    assert_eq!(dig, -1.0, "the dig is still stopped");
    assert!(
        (spawn(&store, 0.5).0 - 0.025).abs() < 1.0e-6,
        "partial folds in"
    );
    store.advance_entity_client_animations(4);
    assert!((spawn(&store, 0.0).0 - 0.2).abs() < 1.0e-6);

    // Leaving `Pose.EMERGING` does NOT stop the emerge (vanilla never auto-stops on pose leave); the
    // non-looping keyframe just holds its final frame, so the timer keeps advancing.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_STANDING_ID)],
    }));
    store.advance_entity_client_animations(1);
    assert!((spawn(&store, 0.0).0 - 0.25).abs() < 1.0e-6);

    // Entering `Pose.DIGGING` starts the dig timer; the emerge keeps holding its running timer.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 142,
        values: vec![protocol_pose_data(6, VANILLA_POSE_DIGGING_ID)],
    }));
    let (emerge, dig) = spawn(&store, 0.0);
    assert!((dig - 0.0).abs() < 1.0e-6, "the dig starts at 0");
    assert!((emerge - 0.25).abs() < 1.0e-6, "the emerge still holds");
}

#[test]
fn entity_model_sources_project_fox_head_roll_and_crouch() {
    // Vanilla `Fox.DATA_FLAGS_ID` is synced data id 19; `FLAG_CROUCHING` is mask 4 and
    // `FLAG_INTERESTED` is mask 8 within that byte.
    let fox_flags = |raw: i8| ProtocolEntityDataValue {
        data_id: 19,
        serializer_id: 0,
        value: EntityDataValueKind::Byte(raw),
    };
    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
    };
    let head_roll = |store: &WorldStore, partial: f32| source(store, partial).fox_head_roll_angle;
    let crouch = |store: &WorldStore, partial: f32| source(store, partial).fox_crouch_amount;
    const HEAD_ROLL_SCALE: f32 = 0.11 * std::f32::consts::PI;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_FOX_ID,
    ));

    // A resting fox projects level head and no crouch, and no flag bools.
    assert_eq!(head_roll(&store, 1.0), 0.0);
    assert_eq!(crouch(&store, 1.0), 0.0);
    assert!(!source(&store, 1.0).fox_is_crouching);
    assert!(!source(&store, 1.0).fox_is_sleeping);

    // Setting `FLAG_INTERESTED` eases `interestedAngle` toward 1 by `* 0.4`/tick. After one tick the
    // angle is `0.4`, so the head roll is `0.4 * 0.11 * π`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(8)],
    }));
    store.advance_entity_client_animations(1);
    assert!((head_roll(&store, 1.0) - 0.4 * HEAD_ROLL_SCALE).abs() < 1.0e-6);
    // A second tick: `0.4 + (1 - 0.4) * 0.4 = 0.64`.
    store.advance_entity_client_animations(1);
    assert!((head_roll(&store, 1.0) - 0.64 * HEAD_ROLL_SCALE).abs() < 1.0e-6);

    // Setting `FLAG_CROUCHING` (and clearing interest) climbs `crouchAmount` by `0.2`/tick and instantly
    // resets the interest ease toward 0.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(4)],
    }));
    assert!(source(&store, 1.0).fox_is_crouching);
    store.advance_entity_client_animations(1);
    assert!((crouch(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(1);
    assert!((crouch(&store, 1.0) - 0.4).abs() < 1.0e-6);

    // `crouchAmount` saturates at `5.0` (vanilla `MAX_CROUCH_AMOUNT`).
    store.advance_entity_client_animations(30);
    assert!((crouch(&store, 1.0) - 5.0).abs() < 1.0e-6);

    // The crouch getter lerps across the partial tick (vanilla `Fox.getCrouchAmount`); clearing the
    // flag resets `crouchAmount` to `0` INSTANTLY (vanilla's non-crouching branch is an assignment).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(0)],
    }));
    assert!(!source(&store, 1.0).fox_is_crouching);
    store.advance_entity_client_animations(1);
    assert_eq!(crouch(&store, 1.0), 0.0);
    // Mid-tick the lerp still shows the drop from `5.0` to `0.0`.
    let at_zero = crouch(&store, 0.0);
    let at_half = crouch(&store, 0.5);
    assert!((at_zero - 5.0).abs() < 1.0e-6);
    assert!((at_half - 2.5).abs() < 1.0e-6);

    // The plain sleep/sit/pounce/faceplant bools project straight off the synced byte.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![fox_flags(32 | 1 | 16 | 64)],
    }));
    let posed = source(&store, 1.0);
    assert!(posed.fox_is_sleeping);
    assert!(posed.fox_is_sitting);
    assert!(posed.fox_is_pouncing);
    assert!(posed.fox_is_faceplanted);
    assert!(!posed.fox_is_crouching);
}

#[test]
fn entity_model_sources_project_cat_lie_down_and_relax_amounts() {
    const CAT_IS_LYING_DATA_ID: u8 = 21;
    const CAT_RELAX_STATE_ONE_DATA_ID: u8 = 22;

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };
    let amounts = |store: &WorldStore, id: i32, partial: f32| {
        let source = source(store, id, partial);
        (
            source.feline_lie_down_amount,
            source.feline_lie_down_amount_tail,
            source.feline_relax_state_one_amount,
        )
    };
    let assert_close = |actual: f32, expected: f32| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    };
    let assert_amounts = |actual: (f32, f32, f32), expected: (f32, f32, f32)| {
        assert_close(actual.0, expected.0);
        assert_close(actual.1, expected.1);
        assert_close(actual.2, expected.2);
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_CAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        93,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
    ));

    assert_eq!(amounts(&store, 92, 1.0), (0.0, 0.0, 0.0));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 92,
        values: vec![
            protocol_bool_data(CAT_IS_LYING_DATA_ID, true),
            protocol_bool_data(CAT_RELAX_STATE_ONE_DATA_ID, true),
        ],
    }));
    store.advance_entity_client_animations(2);
    assert_amounts(amounts(&store, 92, 1.0), (0.3, 0.16, 0.2));
    assert_amounts(amounts(&store, 92, 0.5), (0.225, 0.12, 0.15));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 92,
        values: vec![
            protocol_bool_data(CAT_IS_LYING_DATA_ID, false),
            protocol_bool_data(CAT_RELAX_STATE_ONE_DATA_ID, false),
        ],
    }));
    store.advance_entity_client_animations(1);
    assert_amounts(amounts(&store, 92, 0.0), (0.3, 0.16, 0.2));
    assert_amounts(amounts(&store, 92, 1.0), (0.08, 0.03, 0.07));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 93,
        values: vec![
            protocol_bool_data(CAT_IS_LYING_DATA_ID, true),
            protocol_bool_data(CAT_RELAX_STATE_ONE_DATA_ID, true),
        ],
    }));
    store.advance_entity_client_animations(2);
    assert_eq!(
        amounts(&store, 93, 1.0),
        (0.0, 0.0, 0.0),
        "ocelots do not consume the cat-only lie/relax metadata slots"
    );
}

#[test]
fn entity_model_sources_project_cat_lying_on_sleeping_player() {
    const CAT_IS_LYING_DATA_ID: u8 = 21;
    const POSE_STANDING: i32 = 0;
    const POSE_SLEEPING: i32 = 2;

    let add = |id, entity_type_id, position: [f64; 3]| ProtocolAddEntity {
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        ..protocol_add_entity_with_type(id, entity_type_id)
    };
    let lying_on_player = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .feline_is_lying_on_top_of_sleeping_player
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(add(94, VANILLA_ENTITY_TYPE_CAT_ID, [1.0, 64.0, -2.0]));
    store.apply_add_entity(add(95, VANILLA_ENTITY_TYPE_PLAYER_ID, [2.0, 64.0, -1.0]));
    store.apply_add_entity(add(96, VANILLA_ENTITY_TYPE_PLAYER_ID, [20.0, 64.0, 20.0]));
    store.apply_add_entity(add(97, VANILLA_ENTITY_TYPE_OCELOT_ID, [1.0, 64.0, -2.0]));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 95,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            POSE_SLEEPING,
        )],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 96,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            POSE_SLEEPING,
        )],
    }));
    assert!(
        !lying_on_player(&store, 94),
        "a nearby sleeping player does not matter until Cat.isLying is true"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 94,
        values: vec![protocol_bool_data(CAT_IS_LYING_DATA_ID, true)],
    }));
    assert!(
        lying_on_player(&store, 94),
        "lying cats detect sleeping players in new AABB(cat.blockPosition()).inflate(2)"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 95,
        values: vec![protocol_pose_data(
            super::dimensions::ENTITY_DATA_POSE_ID,
            POSE_STANDING,
        )],
    }));
    assert!(
        !lying_on_player(&store, 94),
        "awake nearby players and far sleeping players do not set the cat renderer flag"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 97,
        values: vec![protocol_bool_data(CAT_IS_LYING_DATA_ID, true)],
    }));
    assert!(
        !lying_on_player(&store, 97),
        "ocelots do not project CatRenderer.isLyingOnTopOfSleepingPlayer"
    );
}

#[test]
fn chicken_flap_state_initializes_flapping_to_one() {
    // Vanilla `Chicken` field initializer `public float flapping = 1.0F;`; every
    // other flap field defaults to 0.
    let state = super::animations::ChickenFlapAnimationState::default();
    assert_eq!(state.flapping, 1.0);
    assert_eq!(state.flap, 0.0);
    assert_eq!(state.o_flap, 0.0);
    assert_eq!(state.flap_speed, 0.0);
    assert_eq!(state.o_flap_speed, 0.0);
}

#[test]
fn entity_model_sources_project_walk_animation_limb_swing() {
    // partial tick 1.0 → WalkAnimationState.position/speed return the current
    // (un-lerped) accumulator values.
    let walk = |store: &WorldStore, partial: f32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync_position = |store: &mut WorldStore, x: f64, z: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 60,
                position: ProtocolVec3d { x, y: 64.0, z },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    // Establish a known baseline feet position, then take the first tick: it only
    // records the position (vanilla `xo == getX()`), so the swing stays at rest.
    sync_position(&mut store, 0.0, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(walk(&store, 1.0), (0.0, 0.0));

    // Move 0.5 blocks along X, then tick: vanilla distance = 0.5, targetSpeed =
    // min(0.5 * 4, 1) = 1.0, speed = 0 + (1 - 0) * 0.4 = 0.4, position = 0 + 0.4.
    sync_position(&mut store, 0.5, 0.0);
    store.advance_entity_client_animations(1);
    let (pos1, speed1) = walk(&store, 1.0);
    assert!(
        (speed1 - 0.4).abs() < 1e-5,
        "speed after one step: {speed1}"
    );
    assert!((pos1 - 0.4).abs() < 1e-5, "position after one step: {pos1}");

    // Move another 0.5 along X and tick: targetSpeed = 1.0 again, speed = 0.4 + (1
    // - 0.4) * 0.4 = 0.64, position = 0.4 + 0.64 = 1.04.
    sync_position(&mut store, 1.0, 0.0);
    store.advance_entity_client_animations(1);
    let (pos2, speed2) = walk(&store, 1.0);
    assert!(
        (speed2 - 0.64).abs() < 1e-5,
        "speed after two steps: {speed2}"
    );
    assert!(
        (pos2 - 1.04).abs() < 1e-5,
        "position after two steps: {pos2}"
    );

    // Vanilla `WalkAnimationState.position/speed(partialTicks)` lerp the projection:
    // speed(0.5) = lerp(0.5, 0.4, 0.64) = 0.52; position(0.5) = 1.04 - 0.64 * 0.5.
    let (pos_mid, speed_mid) = walk(&store, 0.5);
    assert!(
        (speed_mid - 0.52).abs() < 1e-5,
        "mid-tick speed: {speed_mid}"
    );
    assert!(
        (pos_mid - 0.72).abs() < 1e-5,
        "mid-tick position: {pos_mid}"
    );

    // Standing still (no position change) for a tick: distance = 0, targetSpeed =
    // 0, speed = 0.64 + (0 - 0.64) * 0.4 = 0.384; the position keeps integrating.
    store.advance_entity_client_animations(1);
    let (pos3, speed3) = walk(&store, 1.0);
    assert!(
        (speed3 - 0.384).abs() < 1e-5,
        "speed decays toward zero: {speed3}"
    );
    assert!(
        (pos3 - (1.04 + 0.384)).abs() < 1e-5,
        "position keeps integrating: {pos3}"
    );
}

#[test]
fn entity_model_sources_walk_animation_scales_position_for_babies() {
    const AGEABLE_MOB_BABY_DATA_ID: u8 = 16;
    const BOOLEAN_SERIALIZER_ID: i32 = 8;

    let walk = |store: &WorldStore| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 61)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    // Vanilla `updateWalkAnimation` passes `isBaby() ? 3.0F : 1.0F` as the position
    // scale, so a baby's limb-swing position is tripled (the speed is unscaled).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![ProtocolEntityDataValue {
            data_id: AGEABLE_MOB_BABY_DATA_ID,
            serializer_id: BOOLEAN_SERIALIZER_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 61,
            position: ProtocolVec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    store.advance_entity_client_animations(1);
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 61,
            position: ProtocolVec3d {
                x: 0.5,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    store.advance_entity_client_animations(1);
    let (position, speed) = walk(&store);
    // speed = 0.4 (unscaled); position = 0.4 * 3 = 1.2.
    assert!(
        (speed - 0.4).abs() < 1e-5,
        "baby speed is unscaled: {speed}"
    );
    assert!(
        (position - 1.2).abs() < 1e-5,
        "baby position is tripled: {position}"
    );
}

#[test]
fn entity_model_sources_walk_animation_stops_for_passengers_and_the_dead() {
    const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
    const FLOAT_SERIALIZER_ID: i32 = 3;

    let walk = |store: &WorldStore, id: i32| -> (bool, f32, f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            source.is_passenger,
            source.walk_animation_position,
            source.walk_animation_speed,
            source.worn_head_animation_position,
        )
    };
    let move_one_step = |store: &mut WorldStore, id: i32, x0: f64, x1: f64| {
        for x in [x0, x1] {
            assert!(
                store.apply_entity_position_sync(ProtocolEntityPositionSync {
                    id,
                    position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                    delta_movement: ProtocolVec3d {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    on_ground: true,
                })
            );
            store.advance_entity_client_animations(1);
        }
    };

    // A cow riding a boat is a passenger: vanilla `calculateEntityAnimation` calls
    // `walkAnimation.stop()` so its limb swing stays at rest however it is moved.
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 70,
        passenger_ids: vec![71],
    }));
    move_one_step(&mut store, 71, 0.0, 0.5);
    assert_eq!(walk(&store, 71), (true, 0.0, 0.0, 0.0));

    // A cow riding a living entity still stops its own limb swing, but vanilla
    // `LivingEntityRenderer.extractRenderState` drives worn skull animation from the
    // living vehicle's walk animation position.
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 73,
        passenger_ids: vec![74],
    }));
    move_one_step(&mut store, 73, 0.0, 0.5);
    let (is_passenger, passenger_pos, passenger_speed, worn_head_pos) = walk(&store, 74);
    assert!(is_passenger);
    assert_eq!((passenger_pos, passenger_speed), (0.0, 0.0));
    assert!(
        (worn_head_pos - 0.4).abs() < 1e-5,
        "worn head animation follows living vehicle walk: {worn_head_pos}"
    );

    // A dead cow (`isAlive()` false once health <= 0) also stops its limb swing.
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ENTITY_HEALTH_DATA_ID,
            serializer_id: FLOAT_SERIALIZER_ID,
            value: EntityDataValueKind::Float(0.0),
        }],
    }));
    move_one_step(&mut store, 72, 0.0, 0.5);
    assert_eq!(walk(&store, 72), (false, 0.0, 0.0, 0.0));
}

#[test]
fn camel_walk_animation_uses_vanilla_update_override_and_gates() {
    // Vanilla `Camel.updateWalkAnimation`: while standing and not dashing,
    // `targetSpeed = min(distance * 6, 1)` and `WalkAnimationState.update(..., factor = 0.2)`.
    // Non-standing or dashing camels target zero. This differs from the base cow mapping
    // (`min(distance * 4, 1)`, factor 0.4) for the same movement.
    const CAMEL_DASH_DATA_ID: u8 = 19;
    const POSE_STANDING: i32 = 0;
    const POSE_SITTING: i32 = 10;

    let walk = |store: &WorldStore, id: i32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let move_one_step = |store: &mut WorldStore, id: i32| {
        for x in [0.0, 0.5] {
            assert!(
                store.apply_entity_position_sync(ProtocolEntityPositionSync {
                    id,
                    position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                    delta_movement: ProtocolVec3d {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    y_rot: 0.0,
                    x_rot: 0.0,
                    on_ground: true,
                })
            );
            store.advance_entity_client_animations(1);
        }
    };

    let run_case = |entity_type_id: i32, values: Vec<ProtocolEntityDataValue>| {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(80, entity_type_id));
        if !values.is_empty() {
            assert!(store.apply_set_entity_data(ProtocolSetEntityData { id: 80, values }));
        }
        move_one_step(&mut store, 80);
        walk(&store, 80)
    };

    assert_eq!(
        run_case(VANILLA_ENTITY_TYPE_COW_ID, Vec::new()),
        (0.4, 0.4),
        "the cow uses the base LivingEntity updateWalkAnimation mapping"
    );
    assert_eq!(
        run_case(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            vec![protocol_pose_data(6, POSE_STANDING)]
        ),
        (0.2, 0.2),
        "the standing camel uses the vanilla camel factor 0.2 override"
    );
    assert_eq!(
        run_case(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            vec![protocol_pose_data(6, POSE_SITTING)]
        ),
        (0.0, 0.0),
        "a sitting camel targets zero walk speed"
    );
    assert_eq!(
        run_case(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            vec![ProtocolEntityDataValue {
                data_id: CAMEL_DASH_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            }]
        ),
        (0.0, 0.0),
        "a dashing camel targets zero walk speed"
    );
}

#[test]
fn creaking_walk_uses_the_vanilla_distance_to_speed_override() {
    // Vanilla `Creaking.updateWalkAnimation`: `targetSpeed = min(distance · 25, 3); walkAnimation
    // .update(targetSpeed, 0.4, 1)`. After one 0.5-block step the target saturates at `3.0`, so
    // `speed = 0 + (3 - 0) · 0.4 = 1.2` and `position = 1.2` — but `speed(partial)` clamps to `1.0`.
    // A cow with the base `min(distance · 4, 1)` mapping reaches only `position = speed = 0.4` from
    // the same movement, so the creaking ramps ~3× faster.

    let walk = |store: &WorldStore, id: i32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync = |store: &mut WorldStore, id: i32, x: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id,
                position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    // Sync both to the baseline, then take the first shared tick: it only records the feet position
    // (vanilla `xo == getX()`), so the swing stays at rest. (Both entities are advanced together each
    // tick, so neither integrates an extra non-moving tick from the other's update.)
    sync(&mut store, 82, 0.0);
    sync(&mut store, 83, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(walk(&store, 82), (0.0, 0.0));

    // One 0.5-block step: the creaking's `min(0.5 · 25, 3) = 3.0` target gives `position = 1.2` and a
    // clamped `speed = 1.0`; the cow's `min(0.5 · 4, 1) = 1.0` target gives `position = speed = 0.4`.
    sync(&mut store, 82, 0.5);
    sync(&mut store, 83, 0.5);
    store.advance_entity_client_animations(1);
    let (creaking_pos, creaking_speed) = walk(&store, 82);
    assert!(
        (creaking_pos - 1.2).abs() < 1e-5,
        "creaking position ramps with the ·25→3 mapping: {creaking_pos}"
    );
    assert!(
        (creaking_speed - 1.0).abs() < 1e-5,
        "the projected walk speed clamps to 1.0: {creaking_speed}"
    );
    let (cow_pos, _) = walk(&store, 83);
    assert!(
        (cow_pos - 0.4).abs() < 1e-5,
        "cow position uses the base ·4→1 mapping: {cow_pos}"
    );
    assert!(
        creaking_pos > cow_pos,
        "the creaking ramps faster than the base mapping"
    );
}
