use super::*;

#[test]
fn tracks_entity_transient_events() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.last_animation_action, Some(3));
    assert_eq!(entity.last_event_id, Some(35));
    assert_eq!(entity.last_hurt_yaw, Some(45.5));
    assert_eq!(
        store.entities.transient_events(123).unwrap(),
        EntityTransientEvents {
            last_animation_action: Some(3),
            last_event_id: Some(35),
            last_hurt_yaw: Some(45.5),
        }
    );

    assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 4 }));
    assert!(!store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 999,
        event_id: 21,
    }));
    assert!(!store.apply_hurt_animation(ProtocolHurtAnimation { id: 999, yaw: 90.0 }));

    assert_eq!(store.counters().entity_animation_updates_received, 2);
    assert_eq!(store.counters().entity_animation_updates_applied, 1);
    assert_eq!(store.counters().entity_animation_updates_ignored, 1);
    assert_eq!(store.counters().entity_events_received, 2);
    assert_eq!(store.counters().entity_events_applied, 1);
    assert_eq!(store.counters().entity_events_ignored, 1);
    assert_eq!(store.counters().entity_hurt_animations_received, 2);
    assert_eq!(store.counters().entity_hurt_animations_applied, 1);
    assert_eq!(store.counters().entity_hurt_animations_ignored, 1);
}

#[test]
fn sheep_eat_grass_event_drives_client_animation_tick() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let eat_tick = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .sheep_eat_animation_tick
    };

    // Vanilla Sheep.handleEntityEvent: event 10 resets eatAnimationTick to 40.
    assert_eq!(eat_tick(&store, 70), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 70), 40);

    // Vanilla Sheep.aiStep decrements eatAnimationTick once per client tick.
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 39);
    store.advance_entity_client_animations(38);
    assert_eq!(eat_tick(&store, 70), 1);
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 0);
    // It clamps at 0 and does not run negative.
    store.advance_entity_client_animations(5);
    assert_eq!(eat_tick(&store, 70), 0);

    // Only event 10 starts the animation; other sheep events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 6,
    }));
    assert_eq!(eat_tick(&store, 70), 0);

    // Event 10 on a non-sheep entity never starts the sheep eat animation.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 71), 0);
}

#[test]
fn goat_ram_events_drive_the_lower_head_tick_counter() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_GOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let lower_head = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .goat_lower_head_tick
    };

    // Vanilla Goat.handleEntityEvent: event 58 starts lowering the head; the counter then climbs +1 per
    // client tick (aiStep), clamped at 20.
    assert_eq!(lower_head(&store, 72), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 58,
    }));
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 1);
    store.advance_entity_client_animations(19);
    assert_eq!(lower_head(&store, 72), 20);
    // It clamps at the 20 cap.
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 20);

    // Event 59 raises the head; the counter then decays -2 per tick down to 0, after which the state is
    // dropped (a resting goat projects 0).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 59,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 18);
    store.advance_entity_client_animations(9);
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 0);

    // The ram events on a non-goat entity never start the counter.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 73,
        event_id: 58,
    }));
    store.advance_entity_client_animations(3);
    assert_eq!(lower_head(&store, 73), 0);
}

#[test]
fn iron_golem_attack_and_offer_events_drive_client_animation_timers() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla IronGolem.handleEntityEvent: event 4 sets attackAnimationTick to 10; the projection lerps
    // it with the partial tick (attackTicksRemaining = tick - partial).
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 4,
    }));
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        10.0
    );
    assert_eq!(
        source(&store, 74, 0.5).iron_golem_attack_ticks_remaining,
        9.5
    );
    store.advance_entity_client_animations(10);
    // After 10 ticks the attack timer has run out.
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );

    // Event 11 sets offerFlowerTick to 400; event 34 clears it.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 11,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 400);
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 397);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 34,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 0);

    // The same events on a non-golem never start the golem timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 75,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 75, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
}

#[test]
fn ravager_attack_stun_and_roar_timers_advance_together() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla Ravager.handleEntityEvent: event 4 sets attackTick to 10 (partial-lerped projection).
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 4,
    }));
    assert_eq!(source(&store, 76, 0.5).ravager_attack_ticks_remaining, 9.5);
    store.advance_entity_client_animations(10);
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);

    // Event 39 sets stunnedTick to 40; when it decays to 0 the aiStep arms the post-stun roar (20).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 39,
    }));
    assert_eq!(
        source(&store, 76, 0.0).ravager_stunned_ticks_remaining,
        40.0
    );
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(40);
    // Stun has ended; the roar is now armed at 20 and the roarAnimation ramp begins (0 at tick 20).
    assert_eq!(source(&store, 76, 0.0).ravager_stunned_ticks_remaining, 0.0);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(5);
    // After 5 roar ticks: roarTick = 15, roarAnimation = (20 - 15)/20 = 0.25.
    assert!((source(&store, 76, 0.0).ravager_roar_animation - 0.25).abs() < 1.0e-6);
    store.advance_entity_client_animations(15);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);

    // The ravager events on a non-ravager never start the timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 77,
        event_id: 39,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 77, 0.0).ravager_stunned_ticks_remaining, 0.0);
}

#[test]
fn evoker_fangs_attack_event_drives_the_bite_progress_ramp() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let progress = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .evoker_fangs_bite_progress
    };

    // An un-attacked fang is hidden underground (biteProgress 0).
    assert_eq!(progress(&store, 78, 1.0), 0.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 0.0);

    // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
    // and `lifeTicks` (22) begins counting down; `getAnimationProgress` at `lifeTicks`
    // 22 is `1 - (20 - partial)/20`, i.e. just above 0 and climbing.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 78,
        event_id: 4,
    }));
    let started = progress(&store, 78, 1.0);
    assert!(started > 0.0, "the attack ramp starts climbing: {started}");

    store.advance_entity_client_animations(1);
    let after_one = progress(&store, 78, 1.0);
    assert!(
        after_one > started,
        "the bite ramp keeps climbing: {started} -> {after_one}"
    );

    // After 20 ticks `lifeTicks` has reached 2, so `getAnimationProgress` saturates at
    // 1.0 (the fang has fully snapped shut and vanished) and holds there.
    store.advance_entity_client_animations(20);
    assert_eq!(progress(&store, 78, 1.0), 1.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 1.0);

    // The fang event on a non-fang never starts a ramp.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 79,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(progress(&store, 79, 1.0), 0.0);
}

#[test]
fn ravager_stun_tick_particles_follow_the_vanilla_head_anchor() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ));

    assert!(store.take_ravager_stun_particle_states().is_empty());
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 39,
    }));
    store.advance_entity_client_animations(40);

    let particles = store.take_ravager_stun_particle_states();
    assert!(
        !particles.is_empty(),
        "the deterministic client RNG should emit at least one stun particle"
    );
    assert!(particles.len() <= 40);
    let y_body_rot = 20.0_f64.to_radians();
    let anchor_x = 1.0 - 1.95 * y_body_rot.sin();
    let anchor_z = -2.0 + 1.95 * y_body_rot.cos();
    for particle in particles {
        assert_eq!(particle.entity_id, 76);
        assert!((particle.position.y - 65.9).abs() < 1.0e-6);
        assert!(
            (particle.position.x - anchor_x).abs() <= 0.3,
            "x jitter stays inside vanilla +/-0.3 around the head anchor: {:?}",
            particle
        );
        assert!(
            (particle.position.z - anchor_z).abs() <= 0.3,
            "z jitter stays inside vanilla +/-0.3 around the head anchor: {:?}",
            particle
        );
    }
    assert!(store.take_ravager_stun_particle_states().is_empty());
}

#[test]
fn evoker_fangs_attack_tick_emits_twelve_crit_particles_once() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
    ));

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 78,
        event_id: 4,
    }));
    store.advance_entity_client_animations(7);
    assert!(store.take_evoker_fangs_crit_particle_states().is_empty());

    store.advance_entity_client_animations(1);
    let particles = store.take_evoker_fangs_crit_particle_states();
    assert_eq!(particles.len(), 12);
    for particle in particles {
        assert_eq!(particle.entity_id, 78);
        assert!((0.75..=1.25).contains(&particle.position.x));
        assert!((65.05..66.05).contains(&particle.position.y));
        assert!((-2.25..=-1.75).contains(&particle.position.z));
        assert!((-0.3..=0.3).contains(&particle.velocity.x));
        assert!((0.3..=0.6).contains(&particle.velocity.y));
        assert!((-0.3..=0.3).contains(&particle.velocity.z));
    }

    store.advance_entity_client_animations(8);
    assert!(store.take_evoker_fangs_crit_particle_states().is_empty());
}

#[test]
fn camel_dash_flag_drives_the_dash_animation_timer() {
    const CAMEL_DASH_DATA_ID: u8 = 19;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));

    let dash_seconds = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_dash_seconds
    };
    let jump_cooldown = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_jump_cooldown
    };
    let idle_seconds = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_idle_seconds
    };
    let set_dashing = |store: &mut WorldStore, dashing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 90,
            values: vec![ProtocolEntityDataValue {
                data_id: CAMEL_DASH_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dashing),
            }],
        }));
    };

    // A non-dashing camel projects the stopped-animation sentinel.
    assert_eq!(dash_seconds(&store, 1.0), -1.0);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
    assert_eq!(idle_seconds(&store, 1.0), -1.0);
    store.advance_entity_client_animations(3);
    assert_eq!(dash_seconds(&store, 1.0), -1.0);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
    assert!(
        (idle_seconds(&store, 1.0) - 0.15).abs() < 1.0e-6,
        "Camel.setupAnimationStates starts CAMEL_IDLE on the first ticking client frame"
    );

    // Vanilla `Camel.setupAnimationStates`: the synced DASH rising edge starts `dashAnimationState`,
    // and the elapsed seconds climb from there (1 tick = 0.05 s). Vanilla
    // `Camel.onSyncedDataUpdated(DASH)` also seeds `dashCooldown = 55`; the following tick decrements
    // it before `CamelRenderer.getJumpCooldown` subtracts `partialTicks`.
    set_dashing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = dash_seconds(&store, 1.0);
    assert!(
        after_one >= 0.0,
        "a dashing camel starts the dash timer: {after_one}"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 53.0);
    store.advance_entity_client_animations(2);
    let after_three = dash_seconds(&store, 1.0);
    assert!(
        after_three > after_one,
        "the dash timer keeps climbing: {after_one} -> {after_three}"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 51.0);

    // Clearing the DASH flag stops the animation (back to the sentinel) but the cooldown keeps
    // counting down, just like the client-side `Camel.tick` field.
    set_dashing(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(
        dash_seconds(&store, 1.0),
        -1.0,
        "clearing DASH stops the dash animation"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 50.0);
    store.advance_entity_client_animations(50);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
}

#[test]
fn copper_golem_idle_state_drives_delayed_idle_animation_timer() {
    const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
    const COPPER_GOLEM_STATE_IDLE_ID: i32 = 0;
    const COPPER_GOLEM_STATE_GETTING_ITEM_ID: i32 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let idle_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .copper_golem_idle_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 92,
            values: vec![protocol_enum_data(
                COPPER_GOLEM_STATE_DATA_ID,
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    // Vanilla `CopperGolem.setupAnimationStates` first schedules a delayed `random.nextInt(200, 240)`
    // start while the synced state is IDLE; it does not start the head spin on the first client tick.
    assert_eq!(idle_seconds(&store), -1.0);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);

    // bbb uses a deterministic Java-LCG-shaped client seed, but preserves vanilla's 200..239 tick
    // timeout range, so by 240 client ticks the first idle keyframe has started.
    store.advance_entity_client_animations(239);
    let after_timeout = idle_seconds(&store);
    assert!(
        after_timeout >= 0.0,
        "the delayed copper golem idle timer starts by the vanilla timeout window: {after_timeout}"
    );

    // Any non-IDLE interaction state stops the idle animation and clears the timer; returning to IDLE
    // schedules a later restart instead of immediately resuming the head spin.
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_ITEM_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);
}

#[test]
fn copper_golem_getting_item_state_drives_interaction_timer() {
    const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
    const COPPER_GOLEM_STATE_IDLE_ID: i32 = 0;
    const COPPER_GOLEM_STATE_GETTING_ITEM_ID: i32 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        93,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let get_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 93)
            .unwrap()
            .copper_golem_get_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 93,
            values: vec![protocol_enum_data(
                COPPER_GOLEM_STATE_DATA_ID,
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(get_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = get_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "GETTING_ITEM starts CopperGolem.interactionGetItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = get_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the GETTING_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(get_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_getting_no_item_state_drives_interaction_timer() {
    const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
    const COPPER_GOLEM_STATE_IDLE_ID: i32 = 0;
    const COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID: i32 = 2;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        94,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let get_no_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 94)
            .unwrap()
            .copper_golem_get_no_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 94,
            values: vec![protocol_enum_data(
                COPPER_GOLEM_STATE_DATA_ID,
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(get_no_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = get_no_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "GETTING_NO_ITEM starts CopperGolem.interactionGetNoItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = get_no_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the GETTING_NO_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(get_no_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_dropping_item_state_drives_interaction_timer() {
    const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
    const COPPER_GOLEM_STATE_IDLE_ID: i32 = 0;
    const COPPER_GOLEM_STATE_DROPPING_ITEM_ID: i32 = 3;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        95,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let drop_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 95)
            .unwrap()
            .copper_golem_drop_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 95,
            values: vec![protocol_enum_data(
                COPPER_GOLEM_STATE_DATA_ID,
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(drop_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_DROPPING_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = drop_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "DROPPING_ITEM starts CopperGolem.interactionDropItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = drop_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the DROPPING_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(drop_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_dropping_no_item_state_drives_interaction_timer() {
    const COPPER_GOLEM_STATE_DATA_ID: u8 = 17;
    const COPPER_GOLEM_STATE_IDLE_ID: i32 = 0;
    const COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID: i32 = 4;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        96,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let drop_no_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 96)
            .unwrap()
            .copper_golem_drop_no_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 96,
            values: vec![protocol_enum_data(
                COPPER_GOLEM_STATE_DATA_ID,
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(drop_no_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = drop_no_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "DROPPING_NO_ITEM starts CopperGolem.interactionDropNoItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = drop_no_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the DROPPING_NO_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(drop_no_item_seconds(&store), -1.0);
}

#[test]
fn allay_dancing_flag_drives_the_dance_spin_state() {
    const ALLAY_DANCING_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
    };
    let set_dancing = |store: &mut WorldStore, dancing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 91,
            values: vec![ProtocolEntityDataValue {
                data_id: ALLAY_DANCING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dancing),
            }],
        }));
    };

    // A non-dancing allay projects the inert dance state.
    let resting = source(&store, 1.0);
    assert!(!resting.allay_dancing);
    assert!(!resting.allay_spinning);
    assert_eq!(resting.allay_spinning_progress, 0.0);
    store.advance_entity_client_animations(3);
    assert!(!source(&store, 1.0).allay_dancing);

    // Vanilla `Allay.tick` (client): while DATA_DANCING is set, `dancingAnimationTicks` climbs and
    // the first 15 ticks of each 55-tick loop are the spin sub-window (`spinningAnimationTicks`
    // ramping 0->15, `spinningProgress` 0->1).
    set_dancing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = source(&store, 1.0);
    assert!(
        after_one.allay_dancing,
        "the synced flag marks the allay dancing"
    );
    assert!(
        after_one.allay_spinning,
        "the dance opens in the spin sub-window"
    );
    assert!(after_one.allay_spinning_progress > 0.0);

    // Ten ticks into the spin window the progress has climbed further.
    store.advance_entity_client_animations(9);
    let after_ten = source(&store, 1.0);
    assert!(
        after_ten.allay_spinning_progress > after_one.allay_spinning_progress,
        "the spin ramp climbs: {} -> {}",
        after_one.allay_spinning_progress,
        after_ten.allay_spinning_progress
    );

    // Past the 15-tick spin window the allay is still dancing but no longer spinning, and the spin
    // progress unwinds back toward 0 (`spinningAnimationTicks` decrements once `isSpinning` is false).
    store.advance_entity_client_animations(10);
    let after_twenty = source(&store, 1.0);
    assert!(after_twenty.allay_dancing);
    assert!(
        !after_twenty.allay_spinning,
        "the spin sub-window has closed"
    );
    assert!(
        after_twenty.allay_spinning_progress < after_ten.allay_spinning_progress,
        "the spin ramp unwinds once spinning stops"
    );

    // Clearing the flag resets the dance entirely.
    set_dancing(&mut store, false);
    store.advance_entity_client_animations(1);
    let stopped = source(&store, 1.0);
    assert!(!stopped.allay_dancing);
    assert!(!stopped.allay_spinning);
    assert_eq!(stopped.allay_spinning_progress, 0.0);
}

#[test]
fn allay_main_hand_equipment_drives_holding_item_progress() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    ));

    let holding_progress = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .allay_holding_item_progress
    };
    let set_main_hand = |store: &mut WorldStore, item: ItemStackSummary| {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 92,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item,
            }],
        }));
    };

    // Vanilla `Allay.tick`: empty-handed allays keep `holdingItemAnimationTicks`
    // settled at 0, so the projected render-state progress is inert.
    assert_eq!(holding_progress(&store, 1.0), 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(holding_progress(&store, 1.0), 0.0);

    // A non-empty main hand raises the held-item counter by 1 per client tick and
    // `getHoldingItemAnimationProgress(partialTick)` divides the lerp by 5.
    set_main_hand(&mut store, item_stack(42, 1));
    store.advance_entity_client_animations(1);
    assert!((holding_progress(&store, 0.5) - 0.1).abs() < 1.0e-6);
    assert!((holding_progress(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((holding_progress(&store, 1.0) - 1.0).abs() < 1.0e-6);

    // Clearing the main hand eases the arms back down before the idle state is dropped.
    set_main_hand(&mut store, ItemStackSummary::empty());
    store.advance_entity_client_animations(1);
    assert!((holding_progress(&store, 1.0) - 0.8).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert_eq!(holding_progress(&store, 1.0), 0.0);
}

#[test]
fn pillager_charging_crossbow_flag_drives_the_use_item_tick_counter() {
    const PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 17;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 77)
            .unwrap()
    };
    let set_charging = |store: &mut WorldStore, charging: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 77,
            values: vec![ProtocolEntityDataValue {
                data_id: PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(charging),
            }],
        }));
    };

    // A pillager that has never drawn its crossbow projects zero use-item ticks.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    // Vanilla `LivingEntity` reconstructs `getTicksUsingItem()` as a per-tick count that rises while the
    // crossbow draw (use-item) is active; the renderer reads `getTicksUsingItem(partialTicks)`.
    set_charging(&mut store, true);
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        1.0,
        "one tick of charging counts as one use-item tick"
    );
    // The partial tick adds the in-between fraction, matching `getTicksUsingItem(partialTicks)`.
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 1.5);

    store.advance_entity_client_animations(9);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        10.0,
        "the counter keeps climbing while the draw is held"
    );

    // Clearing the charging flag resets the counter to zero (the draw stopped).
    set_charging(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 1.0).crossbow_charge_ticks,
        0.0,
        "releasing the crossbow resets the use-item counter"
    );
}

#[test]
fn piglin_charging_crossbow_flag_drives_the_shared_use_item_tick_counter() {
    const PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 18;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 78)
            .unwrap()
    };
    let set_charging = |store: &mut WorldStore, charging: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 78,
            values: vec![ProtocolEntityDataValue {
                data_id: PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(charging),
            }],
        }));
    };

    // The regular piglin draws its crossbow with the SAME `animateCrossbowCharge`, so it shares the
    // pillager's `getTicksUsingItem` counter — only its synced flag's id (18 vs 17) differs.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    set_charging(&mut store, true);
    store.advance_entity_client_animations(4);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        4.0,
        "four ticks of charging count as four use-item ticks"
    );
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 4.5);

    // Releasing the crossbow resets the shared counter.
    set_charging(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
}

#[test]
fn player_using_item_flag_drives_the_shared_use_item_tick_counter() {
    const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
    const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 79)
            .unwrap()
    };
    let set_using = |store: &mut WorldStore, using: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 79,
            values: vec![ProtocolEntityDataValue {
                data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(if using {
                    LIVING_ENTITY_FLAG_IS_USING
                } else {
                    0
                }),
            }],
        }));
    };

    // Vanilla `getTicksUsingItem()` is item-agnostic, so the player drives the SAME draw counter off its
    // `isUsingItem` bit (`DATA_LIVING_ENTITY_FLAGS & 1`); the native layer applies the crossbow pose only
    // when the using item is an uncharged crossbow.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    set_using(&mut store, true);
    store.advance_entity_client_animations(3);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        3.0,
        "three ticks of using an item count as three use-item ticks"
    );
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 3.5);

    // Stopping the use resets the shared counter (the draw stopped).
    set_using(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
}

#[test]
fn axolotl_playing_dead_flag_drives_the_eased_factor() {
    const AXOLOTL_PLAYING_DEAD_DATA_ID: u8 = 19;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    ));

    let factor = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .axolotl_playing_dead_factor
    };
    let set_playing_dead = |store: &mut WorldStore, dead: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 92,
            values: vec![ProtocolEntityDataValue {
                data_id: AXOLOTL_PLAYING_DEAD_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(dead),
            }],
        }));
    };

    // An awake axolotl projects no play-dead blend.
    assert_eq!(factor(&store, 1.0), 0.0);

    // Vanilla `Axolotl.playingDeadAnimator` (`BinaryAnimator(10, IN_OUT_SINE)`): the synced
    // `DATA_PLAYING_DEAD` flag eases the factor from 0 to a full 1.0 over the animator's 10 ticks.
    set_playing_dead(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = factor(&store, 1.0);
    assert!(
        after_one > 0.0 && after_one < 1.0,
        "the play-dead factor eases up: {after_one}"
    );
    store.advance_entity_client_animations(9);
    assert!(
        (factor(&store, 1.0) - 1.0).abs() < 1.0e-6,
        "the factor saturates at 1.0 after the 10-tick animator length"
    );

    // Clearing the flag eases the factor back down to 0 over the next 10 ticks.
    set_playing_dead(&mut store, false);
    store.advance_entity_client_animations(1);
    let easing_down = factor(&store, 1.0);
    assert!(
        easing_down < 1.0,
        "the factor eases back down once awake: {easing_down}"
    );
    store.advance_entity_client_animations(10);
    assert_eq!(factor(&store, 1.0), 0.0, "fully awake again");
}

#[test]
fn hoglin_and_zoglin_attack_event_drives_the_headbutt_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let attack_tick = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .hoglin_attack_animation_tick
    };

    // Vanilla Hoglin/Zoglin.handleEntityEvent: event 4 sets attackAnimationRemainingTicks to 10 (the
    // RAW int, decremented each tick — no partial lerp). Both the hoglin and the zoglin headbutt.
    for id in [78, 79] {
        assert_eq!(attack_tick(&store, id), 0);
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: id,
            event_id: 4,
        }));
        assert_eq!(attack_tick(&store, id), 10);
    }
    store.advance_entity_client_animations(1);
    assert_eq!(attack_tick(&store, 78), 9);
    assert_eq!(attack_tick(&store, 79), 9);
    store.advance_entity_client_animations(9);
    assert_eq!(attack_tick(&store, 78), 0);
    assert_eq!(attack_tick(&store, 79), 0);

    // The attack event on a non-hoglin never starts the headbutt timer.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(attack_tick(&store, 80), 0);
}

#[test]
fn rabbit_jump_event_drives_the_hop_window() {
    const RABBIT_JUMP_EVENT_ID: i8 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let hop = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .rabbit_hop_seconds
    };

    // A resting rabbit projects the `-1.0` stopped sentinel.
    assert_eq!(hop(&store, 50, 1.0), -1.0);

    // Vanilla `Rabbit.handleEntityEvent(1)` seeds `jumpDuration = 15; jumpTicks = 0`. The hop is NOT
    // started yet — vanilla's `setupAnimationStates` (the hop branch) runs BEFORE `aiStep` lifts
    // `jumpTicks` past `0`, so the seed tick still reads the stopped sentinel.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // First tick: `jumpTicks` climbs to 1, but the hop only `startIfStopped`s on the NEXT tick's
    // `setupAnimationStates`, so it is still stopped here.
    store.advance_entity_client_animations(1);
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // Second tick: `jumpTicks > 0`, so the hop starts at the current age (elapsed begins at 0).
    store.advance_entity_client_animations(1);
    assert!((hop(&store, 50, 0.0) - 0.0).abs() < 1.0e-6);
    // The hop advances `1 / 20` per tick while the window runs.
    store.advance_entity_client_animations(5);
    assert!((hop(&store, 50, 0.0) - 0.25).abs() < 1.0e-6);
    // The partial tick folds into the live age.
    assert!((hop(&store, 50, 0.5) - 0.275).abs() < 1.0e-6);

    // The window is 15 ticks; the hop holds through its end (`jumpTicks` reaches `jumpDuration` and
    // resets), then stops on the following tick (`jumpTicks` back to 0).
    store.advance_entity_client_animations(9);
    assert!(
        (hop(&store, 50, 0.0) - 0.7).abs() < 1.0e-6,
        "still hopping at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        hop(&store, 50, 0.0),
        -1.0,
        "the hop stops when the jump window closes"
    );

    // The jump event on a non-rabbit never starts a hop.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 51,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    assert_eq!(hop(&store, 51, 0.0), -1.0);
}

#[test]
fn entity_model_sources_project_arrow_impact_shake() {
    const ABSTRACT_ARROW_IN_GROUND_DATA_ID: u8 = 10;

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        62,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    ));

    // Vanilla `AbstractArrow.onSyncedDataUpdated(IN_GROUND)` ignores the entity's first tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, true)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 60, 0.0).arrow_shake,
        0.0,
        "the first-tick metadata update must not be replayed from stored state"
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, false)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    // Past the first client tick, an `IN_GROUND` update to true starts `shakeTime = 7`;
    // `ArrowRenderer.extractRenderState` projects `shakeTime - partialTick`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, true)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 7.0);
    assert_eq!(source(&store, 60, 0.5).arrow_shake, 6.5);

    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 6.0);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, true)],
    }));
    assert_eq!(
        source(&store, 60, 0.0).arrow_shake,
        6.0,
        "vanilla only restarts when the current shake has settled"
    );
    store.advance_entity_client_animations(6);
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, true)],
    }));
    assert_eq!(
        source(&store, 61, 0.25).arrow_shake,
        6.75,
        "spectral arrows share AbstractArrow.shakeTime"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![protocol_bool_data(ABSTRACT_ARROW_IN_GROUND_DATA_ID, true)],
    }));
    assert_eq!(
        source(&store, 62, 0.0).arrow_shake,
        0.0,
        "thrown tridents use their own renderer state and do not consume ArrowRenderState.shake"
    );
}

#[test]
fn creaking_combat_events_and_tearing_down_drive_the_keyframes() {
    const CREAKING_ATTACK_EVENT_ID: i8 = 4;
    const CREAKING_INVULNERABLE_EVENT_ID: i8 = 66;
    const CREAKING_CAN_MOVE_DATA_ID: u8 = 16;
    const CREAKING_IS_TEARING_DOWN_DATA_ID: u8 = 18;

    let creaking_bool = |data_id: u8, value: bool| ProtocolEntityDataValue {
        data_id,
        serializer_id: 8,
        value: EntityDataValueKind::Boolean(value),
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
        70,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting creaking can move (the `CAN_MOVE` default) and projects the `-1.0` stopped sentinels.
    let rest = source(&store, 70, 1.0);
    assert!(rest.creaking_can_move, "default canMove is true");
    assert_eq!(rest.creaking_attack_seconds, -1.0);
    assert_eq!(rest.creaking_invulnerable_seconds, -1.0);
    assert_eq!(rest.creaking_death_seconds, -1.0);

    // The synced `CAN_MOVE = false` freezes the walk (a creaking observed mid-step turns to a statue).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_CAN_MOVE_DATA_ID, false)],
    }));
    assert!(!source(&store, 70, 1.0).creaking_can_move);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_CAN_MOVE_DATA_ID, true)],
    }));

    // Vanilla `Creaking.handleEntityEvent(4)`: `attackAnimationRemainingTicks = 15`. The one-shot is
    // NOT started yet — it only `animateWhen`s on the NEXT tick's `setupAnimationStates`, after
    // `aiStep` has decremented the counter (still positive). So the seed tick reads the stopped value.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    assert_eq!(source(&store, 70, 0.0).creaking_attack_seconds, -1.0);
    // First tick: vanilla decrements `15 -> 14` BEFORE `setupAnimationStates`, so the attack starts at
    // the current age this very tick (elapsed begins at 0), unlike the rabbit (which is animate-first).
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.0).abs() < 1.0e-6);
    // It advances `1 / 20` per tick, with the partial folded into the live age.
    store.advance_entity_client_animations(5);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.25).abs() < 1.0e-6);
    assert!((source(&store, 70, 0.5).creaking_attack_seconds - 0.275).abs() < 1.0e-6);
    // The window is 15 ticks (`attackTicks` 15 -> 0); the attack animates while it stays positive, so
    // it holds through tick 14 then stops when the counter hits 0 on tick 15.
    store.advance_entity_client_animations(8);
    assert!(
        (source(&store, 70, 0.0).creaking_attack_seconds - 0.65).abs() < 1.0e-6,
        "still attacking at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_attack_seconds,
        -1.0,
        "the attack stops when the 15-tick window closes"
    );

    // Vanilla `Creaking.handleEntityEvent(66)`: `invulnerabilityAnimationRemainingTicks = 8`, the same
    // decrement-first window (8 ticks).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_INVULNERABLE_EVENT_ID,
    }));
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_invulnerable_seconds - 0.0).abs() < 1.0e-6);
    store.advance_entity_client_animations(7);
    assert_eq!(
        source(&store, 70, 0.0).creaking_invulnerable_seconds,
        -1.0,
        "the stagger stops when the 8-tick window closes"
    );

    // Vanilla `deathAnimationState.animateWhen(isTearingDown(), tickCount)`: the synced
    // `IS_TEARING_DOWN` boolean drives the collapse directly (no counter). Setting it spins up the
    // death one-shot on the next tick; clearing it stops the timer.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_IS_TEARING_DOWN_DATA_ID, true)],
    }));
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_death_seconds - 0.0).abs() < 1.0e-6);
    store.advance_entity_client_animations(5);
    assert!((source(&store, 70, 0.0).creaking_death_seconds - 0.25).abs() < 1.0e-6);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(CREAKING_IS_TEARING_DOWN_DATA_ID, false)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_death_seconds,
        -1.0,
        "clearing isTearingDown stops the collapse"
    );

    // A non-creaking never gets a creaking state: its combat seconds stay stopped, and `canMove`
    // projects the gated `true` regardless of the event/metadata.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    let chicken = source(&store, 71, 0.0);
    assert!(chicken.creaking_can_move);
    assert_eq!(chicken.creaking_attack_seconds, -1.0);
    assert_eq!(chicken.creaking_death_seconds, -1.0);
}

#[test]
fn breeze_pose_drives_the_action_animations() {
    // Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `tick`: the synced pose starts/stops the
    // shoot/inhale/slide/longJump one-shots (active while their pose holds), and LEAVING `Pose.SLIDING`
    // fires the brief `slideBack`. Each is projected as the elapsed seconds since it started, `-1.0`
    // when stopped. The looping idle is renderer-side and not projected.
    const POSE_STANDING: i32 = 0;
    const POSE_LONG_JUMPING: i32 = 6;
    const POSE_SLIDING: i32 = 15;
    const POSE_SHOOTING: i32 = 16;
    const POSE_INHALING: i32 = 17;

    let actions = |store: &WorldStore, id: i32| {
        let s = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            s.breeze_shoot_seconds,
            s.breeze_slide_seconds,
            s.breeze_slide_back_seconds,
            s.breeze_inhale_seconds,
            s.breeze_long_jump_seconds,
        )
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![protocol_pose_data(6, pose)],
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_BREEZE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting breeze projects the stopped sentinel for every action.
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SHOOTING` starts the shoot at the current age (elapsed begins at 0, advancing 1/20 per
    // tick); the others stay stopped.
    set_pose(&mut store, 60, POSE_SHOOTING);
    assert_eq!(actions(&store, 60), (0.0, -1.0, -1.0, -1.0, -1.0));
    store.advance_entity_client_animations(5);
    assert!((actions(&store, 60).0 - 0.25).abs() < 1.0e-6);
    // Leaving SHOOTING stops the shoot (it is not a SLIDING leave, so no slideBack).
    set_pose(&mut store, 60, POSE_STANDING);
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SLIDING` starts the slide; LEAVING it stops the slide AND fires `slideBack` at the leave.
    set_pose(&mut store, 60, POSE_SLIDING);
    assert_eq!(actions(&store, 60).1, 0.0, "slide starts on SLIDING");
    store.advance_entity_client_animations(2);
    assert!((actions(&store, 60).1 - 0.1).abs() < 1.0e-6);
    set_pose(&mut store, 60, POSE_STANDING);
    let (shoot, slide, slide_back, _, _) = actions(&store, 60);
    assert_eq!(shoot, -1.0);
    assert_eq!(slide, -1.0, "leaving SLIDING stops the slide");
    assert_eq!(
        slide_back, 0.0,
        "leaving SLIDING fires slideBack at the leave"
    );
    store.advance_entity_client_animations(3);
    assert!(
        (actions(&store, 60).2 - 0.15).abs() < 1.0e-6,
        "the slideBack return advances"
    );

    // `Pose.INHALING` starts the inhale; switching to `Pose.LONG_JUMPING` stops it and starts longJump.
    set_pose(&mut store, 60, POSE_INHALING);
    assert_eq!(actions(&store, 60).3, 0.0, "inhale starts on INHALING");
    set_pose(&mut store, 60, POSE_LONG_JUMPING);
    let (_, _, _, inhale, long_jump) = actions(&store, 60);
    assert_eq!(inhale, -1.0, "leaving INHALING stops the inhale");
    assert_eq!(long_jump, 0.0, "LONG_JUMPING starts the jump");

    // A non-breeze never gets a breeze state: every action stays stopped regardless of the pose.
    set_pose(&mut store, 61, POSE_SHOOTING);
    store.advance_entity_client_animations(2);
    assert_eq!(actions(&store, 61), (-1.0, -1.0, -1.0, -1.0, -1.0));
}

#[test]
fn warden_tendril_event_drives_client_animation_pulse() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let tendril = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .tendril_animation
    };

    // A warden at rest reports no tendril pulse.
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Vanilla Warden.handleEntityEvent: event 61 resets tendrilAnimation to 10. Vanilla
    // getTendrilAnimation lerps (tendrilAnimationO, tendrilAnimation) / 10, so right after the
    // event the lerp fades from the previous 0 (partialTick 0) to the new 10 (partialTick 1).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 61,
    }));
    assert!((tendril(&store, 80, 0.0) - 0.0).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 0.5) - 0.5).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 1.0) - 1.0).abs() < 1.0e-6);

    // Vanilla Warden.tick decrements tendrilAnimation once per client tick (lerp endpoint = current).
    store.advance_entity_client_animations(1);
    assert!((tendril(&store, 80, 1.0) - 0.9).abs() < 1.0e-6);
    store.advance_entity_client_animations(9);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);
    // It settles at 0 and stays there.
    store.advance_entity_client_animations(5);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);

    // Only event 61 starts the pulse; other warden events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 4,
    }));
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Event 61 on a non-warden entity never starts the tendril pulse.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 81,
        event_id: 61,
    }));
    assert_eq!(tendril(&store, 81, 1.0), 0.0);
}

#[test]
fn probes_entity_status_from_world_store() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let status = store.probe_entity_status(123).unwrap();

    assert_eq!(status.id, 123);
    assert_eq!(status.entity_type_id, 7);
    assert_eq!(status.last_animation_action, Some(3));
    assert_eq!(status.last_event_id, Some(35));
    assert_eq!(status.last_hurt_yaw, Some(45.5));
    assert!(status.mob_effects.is_empty());
    assert!(status.last_damage.is_none());
    assert!(store.probe_entity_status(999).is_none());
}

#[test]
fn tracks_entity_link_updates() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(10));
    store.apply_add_entity(protocol_add_entity(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(20));
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 999,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(999));
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(999));

    assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 999,
        dest_id: 20,
    }));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 0,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![20],
        }),
        1
    );
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

    assert_eq!(store.counters().entity_link_updates_received, 5);
    assert_eq!(store.counters().entity_link_updates_applied, 4);
    assert_eq!(store.counters().entity_link_updates_ignored, 1);
}

#[test]
fn entity_store_clone_matches_projected_states_component_for_component() {
    let mut store = WorldStore::new();
    // Plain mob, a minecart (conditional lerp component), a hurting
    // projectile (conditional acceleration component), and a re-added id.
    store.apply_add_entity(protocol_add_entity_with_type(1, VANILLA_ENTITY_TYPE_COW_ID));
    store.apply_add_entity(protocol_add_entity_with_type(
        2,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        3,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(1, VANILLA_ENTITY_TYPE_COW_ID));

    let cloned = store.clone();
    assert_eq!(
        serde_json::to_string(&cloned).unwrap(),
        serde_json::to_string(&store).unwrap()
    );
}
