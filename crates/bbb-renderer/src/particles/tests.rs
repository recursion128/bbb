use super::descriptors::VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT;
use super::*;

#[test]
fn particle_spawn_batch_empty_tracks_diagnostics() {
    assert!(ParticleSpawnBatch::default().is_empty());
    assert!(!ParticleSpawnBatch {
        unknown_particle_type_count: 1,
        ..ParticleSpawnBatch::default()
    }
    .is_empty());
    assert!(!ParticleSpawnBatch {
        missing_sprite_count: 1,
        ..ParticleSpawnBatch::default()
    }
    .is_empty());
    let sound_only = ParticleSpawnBatch {
        sound_events: vec![ParticleSoundEvent {
            sound_event_id: "minecraft:test.sound".to_string(),
            source: "ambient".to_string(),
            position: [1.0, 2.0, 3.0],
            volume: 4.0,
            pitch: 0.75,
            seed: 123,
            distance_delay: true,
        }],
        ..ParticleSpawnBatch::default()
    };
    assert_eq!(sound_only.len(), 0);
    assert!(!sound_only.is_empty());
    let scheduled_only = ParticleSpawnBatch {
        scheduled_sound_events: vec![ParticleScheduledSoundEvent {
            event: ParticleSoundEvent {
                sound_event_id: "minecraft:test.delayed".to_string(),
                source: "ambient".to_string(),
                position: [1.0, 2.0, 3.0],
                volume: 4.0,
                pitch: 0.75,
                seed: 123,
                distance_delay: true,
            },
            delay_ticks: 2,
            far_sound_event_id: None,
            far_distance_squared: None,
        }],
        ..ParticleSpawnBatch::default()
    };
    assert_eq!(scheduled_only.len(), 0);
    assert!(!scheduled_only.is_empty());
}

#[test]
fn particle_runtime_submit_batch_queues_batch_sound_events() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let sound = ParticleSoundEvent {
        sound_event_id: "minecraft:entity.firework_rocket.blast".to_string(),
        source: "ambient".to_string(),
        position: [10.0, 64.0, -3.0],
        volume: 20.0,
        pitch: 1.0,
        seed: 123,
        distance_delay: true,
    };

    let summary = particles.submit_batch(ParticleSpawnBatch {
        sound_events: vec![sound.clone()],
        ..ParticleSpawnBatch::default()
    });

    assert_eq!(summary.requested_spawns, 0);
    assert_eq!(summary.queued_spawns, 0);
    assert_eq!(particles.drain_sound_events(), vec![sound]);
}

#[test]
fn particle_runtime_scheduled_sound_events_release_with_current_camera_variant() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let near_sound = ParticleSoundEvent {
        sound_event_id: "minecraft:entity.firework_rocket.twinkle".to_string(),
        source: "ambient".to_string(),
        position: [10.0, 64.0, -3.0],
        volume: 20.0,
        pitch: 0.95,
        seed: 456,
        distance_delay: true,
    };

    let summary = particles.submit_batch(ParticleSpawnBatch {
        scheduled_sound_events: vec![ParticleScheduledSoundEvent {
            event: near_sound,
            delay_ticks: 2,
            far_sound_event_id: Some("minecraft:entity.firework_rocket.twinkle_far".to_string()),
            far_distance_squared: Some(256.0),
        }],
        ..ParticleSpawnBatch::default()
    });
    assert_eq!(summary.requested_spawns, 0);
    assert!(particles.drain_sound_events().is_empty());

    particles.advance_with_world_and_particle_contexts_and_sound_camera(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[],
        Some([10.0, 64.0, 0.0]),
    );
    assert!(particles.drain_sound_events().is_empty());

    particles.advance_with_world_and_particle_contexts_and_sound_camera(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[],
        Some([10.0, 64.0, 14.0]),
    );
    let sounds = particles.drain_sound_events();
    assert_eq!(sounds.len(), 1);
    assert_eq!(
        sounds[0].sound_event_id,
        "minecraft:entity.firework_rocket.twinkle_far"
    );
    assert_eq!(sounds[0].position, [10.0, 64.0, -3.0]);
    assert!(sounds[0].distance_delay);
}

#[test]
fn particle_descriptor_falls_back_without_blocking_unknown_particles() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:unknown_test_particle", 1.0)],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(0);

    assert_eq!(summary.intaken_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.provider, "Particle");
    assert!(instance.lifetime_ticks > 0);
    assert_eq!(instance.current_sprite_index, Some(0));
    assert_eq!(
        instance.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_close_f32(instance.friction, 0.98);
    assert_close_f32(instance.gravity, 0.0);
    assert!(instance.has_physics);
}

#[test]
fn particle_runtime_queues_spawns_and_keeps_newest_on_overflow() {
    let mut particles = ParticleRuntimeState::with_capacity(2);

    let summary = particles.submit_batch(ParticleSpawnBatch {
        commands: vec![
            spawn_command("minecraft:cloud", 1.0),
            spawn_command("minecraft:flame", 2.0),
            spawn_command("minecraft:smoke", 3.0),
        ],
        ..ParticleSpawnBatch::default()
    });

    assert_eq!(summary.requested_spawns, 3);
    assert_eq!(summary.queued_spawns, 3);
    assert_eq!(summary.dropped_spawns, 1);
    assert_eq!(summary.pending_spawns, 2);
    assert_eq!(summary.total_dropped_spawns, 1);
    let ids: Vec<_> = particles
        .pending_spawns()
        .iter()
        .map(|command| command.particle_id.as_str())
        .collect();
    assert_eq!(ids, vec!["minecraft:flame", "minecraft:smoke"]);
}

#[test]
fn particle_runtime_zero_capacity_counts_drops_without_queueing() {
    let mut particles = ParticleRuntimeState::with_capacity(0);

    let summary = particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:cloud", 1.0)],
        missing_definition_count: 2,
        missing_sprite_count: 3,
        ..ParticleSpawnBatch::default()
    });

    assert_eq!(summary.requested_spawns, 1);
    assert_eq!(summary.queued_spawns, 0);
    assert_eq!(summary.dropped_spawns, 1);
    assert_eq!(summary.missing_definition_count, 2);
    assert_eq!(summary.missing_sprite_count, 3);
    assert_eq!(summary.pending_spawns, 0);
    assert!(particles.pending_spawns().is_empty());
}

#[test]
fn particle_runtime_advances_pending_spawns_into_active_instances() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![
            spawn_command("minecraft:cloud", 1.0),
            spawn_command("minecraft:flame", 2.0),
        ],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(0);

    assert_eq!(summary.ticks, 0);
    assert_eq!(summary.intaken_instances, 2);
    assert_eq!(summary.dropped_active_instances, 0);
    assert_eq!(summary.pending_spawns, 0);
    assert_eq!(summary.active_instances, 2);
    assert!(particles.pending_spawns().is_empty());
    assert_eq!(
        particles.active_instances()[0].particle_id,
        "minecraft:cloud"
    );
    assert_eq!(particles.active_instances()[0].position, [1.0, 0.0, 0.0]);
    assert_eq!(particles.active_instances()[0].age_ticks, 0);
}

#[test]
fn particle_runtime_ages_active_instances_on_client_ticks() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:cloud", 1.0)],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    let summary = particles.advance(3);

    assert_eq!(summary.ticks, 3);
    assert_eq!(summary.intaken_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert_eq!(particles.active_instances()[0].age_ticks, 3);
}

#[test]
fn particle_runtime_item_pickup_tracks_target_midpoint_and_expires_on_third_tick() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![item_pickup_spawn_command()],
        ..ParticleSpawnBatch::default()
    });
    let intake = particles.advance(0);
    assert_eq!(intake.intaken_instances, 1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.particle_id, ITEM_PICKUP_PARTICLE_ID);
    assert_eq!(instance.render_group, ParticleRenderGroup::ItemPickup);
    assert_eq!(
        instance.tick_motion,
        ParticleTickMotionDescriptor::ItemPickup
    );
    assert_eq!(instance.lifetime_ticks, ITEM_PICKUP_PARTICLE_LIFETIME_TICKS);
    assert_eq!(instance.start_position, [1.0, 64.0, -2.0]);
    assert_close3(
        instance.item_pickup_previous_target.unwrap(),
        [4.0, 70.8, 8.0],
    );
    assert_close3(instance.item_pickup_target.unwrap(), [4.0, 70.8, 8.0]);
    assert_eq!(
        instance.option_item,
        Some(ParticleItemOptionState {
            item_id: 42,
            count: 5,
            component_patch_len: 0,
        })
    );

    particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[ParticleEntityTargetContext {
            entity_id: 20,
            position: [6.0, 71.0, -4.0],
        }],
    );
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(
        instance.item_pickup_previous_target.unwrap(),
        [4.0, 70.8, 8.0],
    );
    assert_close3(
        instance.item_pickup_target.unwrap(),
        [6.0, 71.0 + f64::from(0.8_f32), -4.0],
    );
    let target = [
        lerp_f64(0.5, 4.0, 6.0),
        lerp_f64(0.5, 70.8, 71.0 + f64::from(0.8_f32)),
        lerp_f64(0.5, 8.0, -4.0),
    ];
    let time = ((1.0_f64 + 0.5) / 3.0).powi(2);
    assert_close3(
        instance
            .item_pickup_position_at_partial_tick(0.5)
            .expect("item pickup particle has an extract position"),
        [
            lerp_f64(time, 1.0, target[0]),
            lerp_f64(time, 64.0, target[1]),
            lerp_f64(time, -2.0, target[2]),
        ],
    );
    let render_states = item_pickup_particle_render_states(particles.active_instances().iter());
    assert_eq!(render_states.len(), 1);
    assert_eq!(render_states[0].source_entity_id, 10);
    assert_eq!(render_states[0].item.item_id, 42);
    assert_eq!(render_states[0].item.count, 5);
    assert_eq!(render_states[0].age_ticks, 12.0);
    assert_eq!(render_states[0].light, [0.4, 0.8]);
    assert_close3_f32(
        render_states[0].position,
        [
            lerp_f64(time, 1.0, target[0]) as f32,
            lerp_f64(time, 64.0, target[1]) as f32,
            lerp_f64(time, -2.0, target[2]) as f32,
        ],
    );

    particles.advance(1);
    assert_eq!(particles.active_instances()[0].age_ticks, 2);
    let expired = particles.advance(1);
    assert_eq!(expired.expired_instances, 1);
    assert_eq!(expired.active_instances, 0);
}

#[test]
fn particle_runtime_item_pickup_round_trips_opaque_component_patch() {
    // The pickup channel carries the picked-up stack's serialized
    // DataComponentPatchSummary as an opaque blob so the native bake can rebuild
    // the component-rich stack after the renderer owns the target
    // interpolation. The renderer must round-trip the payload byte-for-byte
    // through command -> instance -> render state without inspecting it.
    let patch_bytes: Vec<u8> = vec![7, 42, 255, 0, 13];
    let mut command = item_pickup_spawn_command();
    command.option_item_pickup_component_patch = Some(patch_bytes.clone());

    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    let instance = &particles.active_instances()[0];
    assert_eq!(
        instance.option_item_pickup_component_patch,
        Some(patch_bytes.clone())
    );

    particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[ParticleEntityTargetContext {
            entity_id: 20,
            position: [6.0, 71.0, -4.0],
        }],
    );

    let render_states = item_pickup_particle_render_states(particles.active_instances().iter());
    assert_eq!(render_states.len(), 1);
    assert_eq!(render_states[0].component_patch, Some(patch_bytes));
}

#[test]
fn particle_runtime_item_pickup_extracts_projectile_model_render_state() {
    // Vanilla `ItemPickupParticleGroup.ParticleInstance.fromParticle`: the
    // carried entity model renders at the quadratic-interpolated extract
    // position; `ArrowRenderer` / `ThrownTridentRenderer` then orient the model
    // with the extracted yRot/xRot.
    let model = ParticleItemPickupProjectileModel {
        kind: ParticleItemPickupProjectileKind::Trident { foil: true },
        y_rot: 35.0,
        x_rot: -12.0,
    };
    let mut command = item_pickup_spawn_command();
    command.option_item = None;
    command.option_item_pickup_projectile_model = Some(model);
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);
    particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[ParticleEntityTargetContext {
            entity_id: 20,
            position: [6.0, 71.0, -4.0],
        }],
    );

    assert!(item_pickup_particle_render_states(particles.active_instances().iter()).is_empty());
    assert!(
        experience_orb_pickup_particle_render_states(particles.active_instances().iter())
            .is_empty()
    );
    let render_states =
        projectile_pickup_particle_render_states(particles.active_instances().iter());
    assert_eq!(render_states.len(), 1);
    assert_eq!(render_states[0].model, model);
    assert_eq!(render_states[0].light, [0.4, 0.8]);
    let expected_position = particles.active_instances()[0]
        .item_pickup_position_at_partial_tick(0.5)
        .expect("projectile pickup particle has an extract position");
    assert_close3_f32(
        render_states[0].position,
        [
            expected_position[0] as f32,
            expected_position[1] as f32,
            expected_position[2] as f32,
        ],
    );

    // The baked instance transform poses the model at that interpolated
    // position with the vanilla trident orientation (`Ry(yRot - 90)` then
    // `Rz(xRot + 90)`).
    let instances =
        projectile_pickup_particle_render_instances(particles.active_instances().iter());
    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].kind,
        ParticleItemPickupProjectileKind::Trident { foil: true }
    );
    assert_eq!(instances[0].light, [0.4, 0.8]);
    let expected_transform = Mat4::from_translation(Vec3::from_array(render_states[0].position))
        * Mat4::from_rotation_y((35.0_f32 - 90.0).to_radians())
        * Mat4::from_rotation_z((-12.0_f32 + 90.0).to_radians());
    assert_eq!(instances[0].transform, expected_transform);

    // Arrows carry the trailing vanilla `ArrowModel` bake scale (0.9) and the
    // plain `Rz(xRot)` orientation.
    let arrow_model = ParticleItemPickupProjectileModel {
        kind: ParticleItemPickupProjectileKind::TippedArrow,
        y_rot: 35.0,
        x_rot: -12.0,
    };
    let arrow_transform =
        projectile_pickup_particle_model_transform(arrow_model, render_states[0].position);
    assert_eq!(
        arrow_transform,
        Mat4::from_translation(Vec3::from_array(render_states[0].position))
            * Mat4::from_rotation_y((35.0_f32 - 90.0).to_radians())
            * Mat4::from_rotation_z((-12.0_f32).to_radians())
            * Mat4::from_scale(Vec3::splat(0.9))
    );
}

#[test]
fn particle_runtime_item_pickup_extracts_experience_orb_render_state() {
    let mut command = item_pickup_spawn_command();
    command.option_item = None;
    command.option_item_pickup_experience_orb_icon = Some(5);
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);
    particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[ParticleEntityTargetContext {
            entity_id: 20,
            position: [6.0, 71.0, -4.0],
        }],
    );

    assert!(item_pickup_particle_render_states(particles.active_instances().iter()).is_empty());
    let render_states =
        experience_orb_pickup_particle_render_states(particles.active_instances().iter());

    assert_eq!(render_states.len(), 1);
    assert_eq!(render_states[0].source_entity_id, 10);
    assert_eq!(render_states[0].icon, 5);
    assert_eq!(render_states[0].age_ticks, 12.0);
    assert_eq!(render_states[0].light, [0.4, 0.8]);
    let target = [
        lerp_f64(0.5, 4.0, 6.0),
        lerp_f64(0.5, 70.8, 71.0 + f64::from(0.8_f32)),
        lerp_f64(0.5, 8.0, -4.0),
    ];
    let time = ((1.0_f64 + 0.5) / 3.0).powi(2);
    assert_close3_f32(
        render_states[0].position,
        [
            lerp_f64(time, 1.0, target[0]) as f32,
            lerp_f64(time, 64.0, target[1]) as f32,
            lerp_f64(time, -2.0, target[2]) as f32,
        ],
    );
}

#[test]
fn particle_runtime_delays_shriek_tick_until_vanilla_delay_clears() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut command = spawn_command("minecraft:shriek", 1.0);
    command.initial_delay_ticks = 2;
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.delay_ticks, 2);
    assert_eq!(instance.age_ticks, 0);
    assert_eq!(instance.position, [1.0, 0.0, 0.0]);
    assert_eq!(instance.velocity, [0.0, 0.1, 0.0]);

    particles.advance(1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.delay_ticks, 1);
    assert_eq!(instance.age_ticks, 0);
    assert_eq!(instance.position, [1.0, 0.0, 0.0]);

    particles.advance(1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.delay_ticks, 0);
    assert_eq!(instance.age_ticks, 0);
    assert_eq!(instance.position, [1.0, 0.0, 0.0]);

    particles.advance(1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.position, [1.0, 0.1, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.098, 0.0]);
    assert_close_f32(instance.color[3], 1.0 - 1.0 / 30.0);
}

#[test]
fn particle_runtime_advances_motion_with_gravity_before_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:smoke", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.5, 0.25, -0.5];
    instance.gravity = 0.5;
    instance.friction = 0.8;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.5, 2.23, 2.5]);
    assert_close3(instance.velocity, [0.4, 0.184, -0.4]);
}

#[test]
fn particle_runtime_dust_plume_decays_gravity_and_friction_before_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:dust_plume", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, 0.3, -0.4];
    instance.gravity = 0.5;
    instance.friction = 0.96;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close_f32(instance.gravity, 0.44);
    assert_close_f32(instance.friction, 0.8832);
    assert_close3(instance.position, [1.2, 2.2824, 2.6]);
    assert_close3(instance.velocity, [0.17664, 0.249_415_68, -0.35328]);
}

#[test]
fn particle_runtime_falling_dust_rotates_and_clamps_downward_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_dust", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.139, -0.4];
    instance.roll = 0.3;
    instance.previous_roll = 0.2;
    instance.roll_speed = 0.02;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.2, 1.861, 2.6]);
    assert_close3(instance.velocity, [0.2, -0.14, -0.4]);
    assert_close_f32(instance.previous_roll, 0.3);
    assert_close_f32(instance.roll, 0.3 + std::f32::consts::PI * 0.02 * 2.0);
}

#[test]
fn particle_runtime_default_tick_applies_collision_ground_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:dust", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    instance.friction = 0.5;
    instance.gravity = 0.0;
    instance.speed_up_when_y_motion_is_blocked = false;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        assert_close_f64(query.half_width, 0.1);
        assert_close_f64(query.height, 0.2);
        let mut movement = query.movement;
        movement[1] = 0.0;
        movement
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert!(instance.on_ground);
    assert_close3(instance.position, [1.2, 2.0, 2.4]);
    assert_close3(instance.velocity, [0.07, -0.2, -0.21]);
}

#[test]
fn particle_runtime_base_ash_smoke_speeds_up_when_y_motion_is_blocked() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:smoke", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    instance.friction = 0.5;
    instance.gravity = 0.0;
    assert!(instance.speed_up_when_y_motion_is_blocked);
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        movement[1] = 0.0;
        movement
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(instance.on_ground);
    assert_close3(instance.position, [1.2, 2.0, 2.4]);
    assert_close3(instance.velocity, [0.077, -0.2, -0.231]);
}

#[test]
fn particle_runtime_dragon_breath_uses_vanilla_hit_ground_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut hovering = test_instance_with_lifetime("minecraft:dragon_breath", 20);
    hovering.position = [1.0, 2.0, 3.0];
    hovering.previous_position = hovering.position;
    hovering.velocity = [0.5, 0.0, -0.25];
    hovering.friction = 0.96;
    particles.active_instances.push_back(hovering);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let hovering = &particles.active_instances()[0];
    assert_eq!(
        hovering.tick_motion,
        ParticleTickMotionDescriptor::DragonBreath
    );
    assert!(!hovering.hit_ground);
    assert_close3(hovering.position, [1.5, 2.0, 2.75]);
    assert_close3(hovering.velocity, [0.528, 0.0, -0.264]);

    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut rising = test_instance_with_lifetime("minecraft:dragon_breath", 20);
    rising.position = [1.0, 2.0, 3.0];
    rising.previous_position = rising.position;
    rising.velocity = [0.5, 0.25, -0.25];
    rising.friction = 0.96;
    particles.active_instances.push_back(rising);

    particles.advance(1);

    let rising = &particles.active_instances()[0];
    assert_close3(rising.position, [1.5, 2.25, 2.75]);
    assert_close3(rising.velocity, [0.48, 0.25, -0.24]);

    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut grounded = test_instance_with_lifetime("minecraft:dragon_breath", 20);
    grounded.position = [1.0, 2.0, 3.0];
    grounded.previous_position = grounded.position;
    grounded.velocity = [0.5, -0.3, -0.25];
    grounded.friction = 0.96;
    grounded.on_ground = true;
    particles.active_instances.push_back(grounded);

    particles.advance(1);

    let grounded = &particles.active_instances()[0];
    assert!(grounded.hit_ground);
    assert_close3(grounded.position, [1.5, 2.002, 2.75]);
    assert_close3(grounded.velocity, [0.48, 0.00192, -0.24]);
}

#[test]
fn particle_runtime_spell_alpha_tracks_scoping_player_context() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 61);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:infested", 1.0)],
        ..ParticleSpawnBatch::default()
    });
    let near_scoping = Some(ParticleLocalPlayerScopeContext {
        eye_position: [0.0, 0.0, 0.0],
        first_person: true,
        scoping: true,
    });
    let far_scoping = Some(ParticleLocalPlayerScopeContext {
        eye_position: [100.0, 0.0, 0.0],
        first_person: true,
        scoping: true,
    });

    let intake = particles.advance_with_world_and_scope_context(
        0,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        near_scoping,
    );
    assert_eq!(intake.intaken_instances, 1);
    assert_eq!(particles.active_instances()[0].color[3], 0.0);
    assert_eq!(particles.active_instances()[0].original_alpha, 1.0);

    particles.advance_with_world_and_scope_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        far_scoping,
    );
    assert_close_f32(particles.active_instances()[0].color[3], 0.05);

    particles.advance_with_world_and_scope_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        near_scoping,
    );
    assert_eq!(particles.active_instances()[0].color[3], 0.0);
}

#[test]
fn particle_runtime_player_cloud_tracks_nearby_local_player_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
    instance.position = [0.0, 3.0, 0.0];
    instance.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[ParticlePlayerMotionContext {
            position: [0.0, 2.0, 0.0],
            delta_movement: [0.0, -0.2, 0.0],
        }],
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.0, 3.12, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.2672, 0.0]);
}

#[test]
fn particle_runtime_player_cloud_ignores_far_or_lower_local_player() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut far = test_instance_with_lifetime("minecraft:cloud", 20);
    far.position = [3.0, 3.0, 0.0];
    far.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.push_back(far);

    particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[ParticlePlayerMotionContext {
            position: [0.0, 2.0, 0.0],
            delta_movement: [0.0, -0.2, 0.0],
        }],
    );

    let far = &particles.active_instances()[0];
    assert_close3(far.position, [3.0, 3.4, 0.0]);
    assert_close3(far.velocity, [0.0, 0.384, 0.0]);

    let mut lower = test_instance_with_lifetime("minecraft:cloud", 20);
    lower.position = [0.0, 1.0, 0.0];
    lower.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.clear();
    particles.active_instances.push_back(lower);

    particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[ParticlePlayerMotionContext {
            position: [0.0, 2.0, 0.0],
            delta_movement: [0.0, -0.2, 0.0],
        }],
    );

    let lower = &particles.active_instances()[0];
    assert_close3(lower.position, [0.0, 1.4, 0.0]);
    assert_close3(lower.velocity, [0.0, 0.384, 0.0]);
}

// Vanilla `PlayerCloudParticle.tick` pulls toward
// `level.getNearestPlayer(x, y, z, 2.0, false)` (PlayerCloudParticle.java:51,
// EntityGetter.java:74-88): the strictly nearest candidate within 2.0 wins,
// regardless of slice order (native pushes the local player first).
#[test]
fn particle_runtime_player_cloud_pulls_toward_nearest_player_candidate() {
    // Local (first) candidate nearer: post-move particle sits at [0, 3.4, 0];
    // local dist^2 = 0.81 beats remote dist^2 = 1.64.
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
    instance.position = [0.0, 3.0, 0.0];
    instance.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.push_back(instance);

    particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[
            ParticlePlayerMotionContext {
                position: [0.0, 2.5, 0.0],
                delta_movement: [0.0, -0.5, 0.0],
            },
            ParticlePlayerMotionContext {
                position: [1.0, 2.6, 0.0],
                delta_movement: [0.0, 0.3, 0.0],
            },
        ],
    );

    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.0, 3.22, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.2072, 0.0]);

    // Remote (second) candidate nearer: remote dist^2 = 1.0 beats local
    // dist^2 = 1.64, so the pull reads the remote y and delta movement.
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:sneeze", 20);
    instance.position = [0.0, 3.0, 0.0];
    instance.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.push_back(instance);

    particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[
            ParticlePlayerMotionContext {
                position: [1.0, 2.6, 0.0],
                delta_movement: [0.0, -0.5, 0.0],
            },
            ParticlePlayerMotionContext {
                position: [0.0, 2.4, 0.0],
                delta_movement: [0.0, 0.3, 0.0],
            },
        ],
    );

    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.0, 3.2, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.3672, 0.0]);

    // Every candidate at or beyond 2.0: dist^2 = 4.0 exactly (vanilla keeps
    // only `dist < range * range`, EntityGetter.java:81) and dist^2 = 9.0 —
    // no pull, plain friction tick.
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
    instance.position = [0.0, 3.0, 0.0];
    instance.velocity = [0.0, 0.4, 0.0];
    particles.active_instances.push_back(instance);

    particles.advance_with_world_and_player_context(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[
            ParticlePlayerMotionContext {
                position: [0.0, 1.4, 0.0],
                delta_movement: [0.0, -0.5, 0.0],
            },
            ParticlePlayerMotionContext {
                position: [3.0, 3.4, 0.0],
                delta_movement: [0.0, 0.3, 0.0],
            },
        ],
    );

    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.0, 3.4, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.384, 0.0]);
}

#[test]
fn particle_runtime_falling_dust_resets_roll_after_ground_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_dust", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.roll = 0.3;
    instance.previous_roll = 0.2;
    instance.roll_speed = 0.02;
    particles.active_instances.push_back(instance);

    let floor_collision = |query: ParticleCollisionQuery| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    };

    let first_summary = particles.advance_with_collision(1, floor_collision);

    assert_eq!(first_summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert!(instance.on_ground);
    assert_close3(instance.position, [0.0, 0.0, 0.0]);
    assert_close_f32(instance.previous_roll, 0.3);
    assert_close_f32(instance.roll, 0.3 + std::f32::consts::PI * 0.02 * 2.0);

    let second_summary = particles.advance_with_collision(1, floor_collision);

    assert_eq!(second_summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert!(instance.on_ground);
    assert!(instance.stopped_by_collision);
    assert_close3(instance.position, [0.0, 0.0, 0.0]);
    assert_close_f32(instance.previous_roll, 0.0);
    assert_close_f32(instance.roll, 0.0);
}

#[test]
fn particle_runtime_water_drop_uses_direct_gravity_and_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:rain", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, 0.3, -0.4];
    instance.gravity = 0.06;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.2, 2.24, 2.6]);
    assert_close3(instance.velocity, [0.196, 0.2352, -0.392]);
}

#[test]
fn particle_runtime_water_drop_removes_on_ground_when_random_passes() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 4096);
    let mut instance = test_instance_with_lifetime("minecraft:rain", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_water_drop_removes_inside_block_or_fluid_surface() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:rain", 20);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, 0.0, 0.0];
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |query| {
            assert_close3(query.position, [0.5, 0.25, 0.5]);
            ParticleBlockFluidSurfaceSample {
                block_collision_height: 0.5,
                fluid_height: 0.0,
                fluid_kind: None,
                block_is_air: false,
            }
        },
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_drip_falling_removes_on_ground_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_nectar", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_drip_fall_and_land_removes_on_ground_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_honey", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_drip_hang_expiration_spawns_falling_child() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:dripping_honey", 1);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.01, -0.02, 0.03];
    instance.age_ticks = 1;
    instance.child_spawn_templates = vec![
        ParticleChildSpawnTemplate {
            particle_type_id: 80,
            particle_id: "minecraft:falling_honey".to_string(),
            sprite_ids: vec!["minecraft:drip_0".to_string()],
        },
        ParticleChildSpawnTemplate {
            particle_type_id: 81,
            particle_id: "minecraft:landing_honey".to_string(),
            sprite_ids: vec!["minecraft:drip_1".to_string()],
        },
    ];

    let commands = instance.removal_child_spawn_commands(ParticleRemovalReason::LifetimeExpired);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].particle_type_id, 80);
    assert_eq!(commands[0].particle_id, "minecraft:falling_honey");
    assert_close3(commands[0].position, [1.0, 2.0, 3.0]);
    assert_close3(commands[0].velocity, [0.01, -0.02, 0.03]);
    assert_eq!(commands[0].child_spawn_templates.len(), 1);
    assert_eq!(
        commands[0].child_spawn_templates[0].particle_id,
        "minecraft:landing_honey"
    );

    particles.active_instances.push_back(instance);
    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 1);
    let child = &particles.active_instances()[0];
    assert_eq!(child.particle_type_id, 80);
    assert_eq!(child.particle_id, "minecraft:falling_honey");
    assert_eq!(child.provider, "DripParticle.HoneyFallProvider");
    assert_eq!(child.current_sprite_id.as_deref(), Some("minecraft:drip_0"));
    assert_close3(child.position, [1.0, 2.0, 3.0]);
    assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(child.child_spawn_templates.len(), 1);
    assert_eq!(
        child.child_spawn_templates[0].particle_id,
        "minecraft:landing_honey"
    );
}

#[test]
fn particle_runtime_drip_fall_and_land_ground_hit_spawns_landing_child() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:falling_honey", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    instance.child_spawn_templates = vec![ParticleChildSpawnTemplate {
        particle_type_id: 81,
        particle_id: "minecraft:landing_honey".to_string(),
        sprite_ids: vec!["minecraft:drip_1".to_string()],
    }];
    instance.on_ground = true;
    let commands = instance.removal_child_spawn_commands(ParticleRemovalReason::RemovedDuringTick);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].particle_type_id, 81);
    assert_eq!(commands[0].particle_id, "minecraft:landing_honey");
    assert_close3(commands[0].position, [0.0, 0.05, 0.0]);
    assert_eq!(commands[0].velocity, [0.0, 0.0, 0.0]);
    instance.on_ground = false;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 1);
    let child = &particles.active_instances()[0];
    assert_eq!(child.particle_type_id, 81);
    assert_eq!(child.particle_id, "minecraft:landing_honey");
    assert_eq!(child.provider, "DripParticle.HoneyLandProvider");
    assert_eq!(child.current_sprite_id.as_deref(), Some("minecraft:drip_1"));
    assert_close3(child.position, [0.0, 0.0, 0.0]);
    assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
    assert!(child.child_spawn_templates.is_empty());

    let sounds = particles.drain_sound_events();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_event_id, "minecraft:block.beehive.drip");
    assert_eq!(sounds[0].source, "block");
    assert_close3(sounds[0].position, [0.0, 0.0, 0.0]);
    assert!((0.3..=1.0).contains(&sounds[0].volume));
    assert_close_f32(sounds[0].pitch, 1.0);
    assert!(!sounds[0].distance_delay);
}

#[test]
fn particle_runtime_dripstone_ground_hit_emits_local_sound() {
    for (particle_id, expected_sound) in [
        (
            "minecraft:falling_dripstone_lava",
            "minecraft:block.pointed_dripstone.drip_lava",
        ),
        (
            "minecraft:falling_dripstone_water",
            "minecraft:block.pointed_dripstone.drip_water",
        ),
    ] {
        let mut particles = ParticleRuntimeState::with_capacities(4, 4);
        let mut instance = test_instance_with_lifetime(particle_id, 20);
        instance.position = [1.0, 2.05, 3.0];
        instance.previous_position = instance.position;
        instance.velocity = [0.0, -0.1, 0.0];
        instance.gravity = 0.0;
        particles.active_instances.push_back(instance);

        let summary = particles.advance_with_collision(1, |query| {
            let mut movement = query.movement;
            if movement[1] < 0.0 && query.position[1] + movement[1] < 2.0 {
                movement[1] = 2.0 - query.position[1];
            }
            movement
        });

        assert_eq!(summary.expired_instances, 1, "{particle_id}");
        let sounds = particles.drain_sound_events();
        assert_eq!(sounds.len(), 1, "{particle_id}");
        assert_eq!(sounds[0].sound_event_id, expected_sound);
        assert_eq!(sounds[0].source, "block");
        assert_close3(sounds[0].position, [1.0, 2.0, 3.0]);
        assert!((0.3..=1.0).contains(&sounds[0].volume));
        assert_close_f32(sounds[0].pitch, 1.0);
        assert!(!sounds[0].distance_delay);
    }
}

#[test]
fn particle_runtime_drip_land_uses_collision_without_ground_removal() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 4096);
    let mut instance = test_instance_with_lifetime("minecraft:landing_honey", 20);
    instance.position = [0.0, 0.05, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        if movement[1] < 0.0 && query.position[1] + movement[1] < 0.0 {
            movement[1] = -query.position[1];
        }
        movement
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(instance.on_ground);
    assert!(!instance.removed);
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.position, [0.0, 0.0, 0.0]);
}

#[test]
fn particle_runtime_drip_hang_applies_post_move_damping_before_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:dripping_honey", 100);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.0, -0.2];
    instance.gravity = 0.000_012;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.1, 1.999_988, 2.8]);
    assert_close3(instance.velocity, [0.001_96, -0.000_000_235_2, -0.003_92]);
}

#[test]
fn particle_runtime_lava_drip_hang_updates_cooling_color_before_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:dripping_lava", 40);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.0, -0.2];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_eq!(instance.color, [1.0, 1.0, 0.5, 1.0]);
    assert_close3(instance.position, [1.1, 1.9988, 2.8]);
    assert_close3(instance.velocity, [0.001_96, -0.000_023_52, -0.003_92]);
}

#[test]
fn particle_runtime_drip_water_removes_inside_matching_fluid() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_water", 40);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample {
            block_collision_height: 0.0,
            fluid_height: 8.0 / 9.0,
            fluid_kind: Some(ParticleFluidKind::Water),
            block_is_air: false,
        },
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_drip_water_ignores_non_matching_fluid() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:falling_water", 40);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample {
            block_collision_height: 0.0,
            fluid_height: 8.0 / 9.0,
            fluid_kind: Some(ParticleFluidKind::Lava),
            block_is_air: false,
        },
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert!(!particles.active_instances()[0].removed);
}

#[test]
fn particle_runtime_drip_lava_land_removes_inside_matching_fluid() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:landing_lava", 40);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample {
            block_collision_height: 0.0,
            fluid_height: 8.0 / 9.0,
            fluid_kind: Some(ParticleFluidKind::Lava),
            block_is_air: false,
        },
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_drip_empty_fluid_provider_ignores_fluid_sample() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:landing_honey", 40);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample {
            block_collision_height: 0.0,
            fluid_height: 8.0 / 9.0,
            fluid_kind: Some(ParticleFluidKind::Water),
            block_is_air: false,
        },
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert!(!particles.active_instances()[0].removed);
}

#[test]
fn particle_runtime_wake_uses_command_motion_and_vanilla_sprite_cycle() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:fishing", 39);
    instance.sprite_ids = vec![
        "minecraft:wake_0".to_string(),
        "minecraft:wake_1".to_string(),
        "minecraft:wake_2".to_string(),
        "minecraft:wake_3".to_string(),
        "minecraft:wake_4".to_string(),
    ];
    instance.current_sprite_index = Some(0);
    instance.current_sprite_id = Some("minecraft:wake_0".to_string());
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, 0.3, -0.4];
    instance.gravity = 0.0;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.2, 2.3, 2.6]);
    assert_close3(instance.velocity, [0.196, 0.294, -0.392]);
    assert_eq!(instance.current_sprite_index, Some(1));
    assert_eq!(
        instance.current_sprite_id.as_deref(),
        Some("minecraft:wake_1")
    );
}

#[test]
fn particle_runtime_wake_uses_collision_backed_move() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:fishing", 39);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.3, -0.4];
    instance.gravity = 0.0;
    instance.friction = 0.98;
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |query| {
        collision_queries += 1;
        assert_close3(query.position, [1.0, 2.0, 3.0]);
        assert_close3(query.movement, [0.2, -0.3, -0.4]);
        assert_close_f64(query.half_width, 0.005);
        assert_close_f64(query.height, 0.01);
        [query.movement[0], 0.0, query.movement[2]]
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(collision_queries, 1);
    let instance = &particles.active_instances()[0];
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.2, 2.0, 2.6]);
    assert_close3(instance.velocity, [0.196, -0.294, -0.392]);
    assert!(instance.on_ground);
    assert!(instance.stopped_by_collision);
}

#[test]
fn particle_runtime_wake_grows_collision_size_each_tick() {
    // WakeParticle.java:46-47: `float size = life * 0.001F; this.setSize(size,
    // size);` with `life = 60 - this.lifetime`, applied every tick. In bbb
    // `life = 60 - (lifetime_ticks - age_ticks)`, evaluated with the pre-increment
    // age, so with lifetime 40 the first tick sees life 20 and each later tick
    // adds one more 0.001 step.
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:fishing", 40);
    instance.velocity = [0.0, 0.0, 0.0];
    instance.gravity = 0.0;
    // Constructor `setSize(0.01F, 0.01F)` is the initial box (WakeParticle.java:20).
    assert_eq!(instance.collision_width, 0.01);
    assert_eq!(instance.collision_height, 0.01);
    particles.active_instances.push_back(instance);

    for age_before in 0..5u32 {
        particles.advance(1);
        let instance = &particles.active_instances()[0];
        let life = 60 - (40 - age_before);
        let expected = life as f32 * 0.001;
        assert_close_f32(instance.collision_width, expected);
        assert_close_f32(instance.collision_height, expected);
    }
}

#[test]
fn particle_runtime_wake_move_uses_previous_tick_grown_size() {
    // The per-tick `setSize` trails `move` (WakeParticle.java:44 move, :46 setSize),
    // so tick N's move must consume the box grown at the end of tick N-1: tick 1
    // uses the constructor 0.01 box, tick 2 uses life(20) * 0.001 = 0.020.
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:fishing", 40);
    instance.position = [0.0, 5.0, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.1, 0.0];
    instance.gravity = 0.0;
    instance.friction = 1.0;
    particles.active_instances.push_back(instance);

    let mut half_widths = Vec::new();
    for _ in 0..2 {
        particles.advance_with_collision(1, |query| {
            half_widths.push(query.half_width);
            query.movement
        });
    }
    assert_close_f64(half_widths[0], 0.005);
    assert_close_f64(half_widths[1], 0.010);
}

#[test]
fn particle_runtime_campfire_smoke_drifts_up_and_fades_near_lifetime_end() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:campfire_cosy_smoke", 100);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, 0.002, 0.0];
    instance.age_ticks = 39;
    instance.color = [1.0, 1.0, 1.0, 0.9];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 40);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_range_f64(instance.velocity[0].abs(), 0.0, 0.0002);
    assert_range_f64(instance.velocity[2].abs(), 0.0, 0.0002);
    assert_close_f64(instance.velocity[1], 0.002 - 3.0E-6);
    assert_close_f64(instance.position[1], 2.0 + 0.002 - 3.0E-6);
    assert_close_f32(instance.color[3], 0.885);
}

#[test]
fn particle_runtime_campfire_smoke_uses_collision_backed_move() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:campfire_cosy_smoke", 100);
    instance.position = [1.0, 0.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.05, 0.0];
    instance.gravity = 0.0;
    instance.color = [1.0, 1.0, 1.0, 0.9];
    particles.active_instances.push_back(instance);

    let mut queries = Vec::new();
    let summary = particles.advance_with_collision(1, |query| {
        queries.push(query);
        [query.movement[0], 0.0, query.movement[2]]
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert_eq!(queries.len(), 1);
    assert_close_f64(queries[0].half_width, 0.125);
    assert_close_f64(queries[0].height, 0.25);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close_f64(instance.position[1], 0.0);
    assert_close_f64(instance.velocity[1], -0.05);
    assert!(instance.on_ground);
    assert!(instance.stopped_by_collision);
}

#[test]
fn particle_runtime_campfire_smoke_alpha_zero_removes_before_motion() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:campfire_cosy_smoke", 100);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.2, 0.3];
    instance.color = [1.0, 1.0, 1.0, 0.0];
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |query| {
        collision_queries += 1;
        query.movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert_eq!(collision_queries, 0);
}

#[test]
fn particle_runtime_lava_emits_child_smoke_after_tick_when_vanilla_odds_pass() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(8, 8, 0);
    let mut lava = test_instance_with_lifetime("minecraft:lava", 20);
    lava.position = [1.0, 2.0, 3.0];
    lava.previous_position = lava.position;
    lava.velocity = [0.1, 0.2, 0.3];
    lava.child_spawn_templates = vec![ParticleChildSpawnTemplate {
        particle_type_id: 62,
        particle_id: "minecraft:smoke".to_string(),
        sprite_ids: vec!["minecraft:generic_7".to_string()],
    }];
    particles.active_instances.push_back(lava);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 2);
    let lava = &particles.active_instances()[0];
    assert_eq!(lava.age_ticks, 1);
    assert_close3(lava.position, [1.1, 2.17, 3.3]);
    assert_close3(lava.velocity, [0.0999, 0.16983, 0.2997]);

    let smoke = &particles.active_instances()[1];
    assert_eq!(smoke.particle_type_id, 62);
    assert_eq!(smoke.particle_id, "minecraft:smoke");
    assert_eq!(smoke.provider, "SmokeParticle.Provider");
    assert_eq!(
        smoke.current_sprite_id.as_deref(),
        Some("minecraft:generic_7")
    );
    assert_close3(smoke.position, lava.position);
    // The lava particle spawns a `minecraft:smoke` child with its post-tick
    // velocity as the command velocity. Vanilla `SmokeParticle` (via
    // `BaseAshSmokeParticle` -> the base `Particle` 6-arg constructor) then
    // adds the constructor-random spread scaled by `0.1` on intake, so the
    // child velocity is the lava velocity plus a small deterministic spread
    // rather than an exact copy of it.
    assert_close3(
        smoke.velocity,
        [0.10881595316538636, 0.17285028287621526, 0.3025607498781397],
    );
    assert!(smoke.velocity[0] > lava.velocity[0]);
    assert!(smoke.velocity[1] > lava.velocity[1]);
    assert!(smoke.velocity[2] > lava.velocity[2]);
    assert!(smoke.child_spawn_templates.is_empty());
}

#[test]
fn particle_runtime_smoke_intake_applies_vanilla_base_particle_spread() {
    // Vanilla `SmokeParticle` (via `BaseAshSmokeParticle` -> the base
    // `Particle` 6-arg constructor) seeds a constructor-random velocity,
    // scales it by `0.1`, then adds the command velocity, matching the
    // player-cloud velocity model. The intake path must therefore offset
    // the command velocity by the deterministic base spread instead of
    // copying the command velocity verbatim.
    let command_velocity = [0.3, 0.4, 0.5];
    for particle_id in [
        "minecraft:smoke",
        "minecraft:large_smoke",
        "minecraft:white_smoke",
    ] {
        let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
        let mut command = spawn_command(particle_id, 1.0);
        command.velocity = command_velocity;
        particles.submit_batch(ParticleSpawnBatch {
            commands: vec![command],
            ..ParticleSpawnBatch::default()
        });
        particles.advance(0);

        let instance = &particles.active_instances()[0];
        assert_eq!(instance.particle_id, particle_id);
        let expected =
            descriptors::ParticleInitialVelocityDescriptor::ParticleConstructorZeroScaledPlusCommand {
                scale: 0.1,
            }
            .sample(command_velocity, &mut ParticleRandom::new(0));
        assert_close3(instance.velocity, expected);
        assert_ne!(instance.velocity, command_velocity, "{particle_id}");
    }
}

#[test]
fn particle_runtime_huge_explosion_seed_emits_vanilla_child_explosions() {
    let explosion_template = ParticleChildSpawnTemplate {
        particle_type_id: 23,
        particle_id: "minecraft:explosion".to_string(),
        sprite_ids: vec!["minecraft:explosion_0".to_string()],
    };
    let mut seed_for_command = test_instance_with_lifetime("minecraft:explosion_emitter", 8);
    seed_for_command.position = [1.0, 0.0, 0.0];
    seed_for_command.age_ticks = 3;
    seed_for_command.child_spawn_templates = vec![explosion_template.clone()];
    let commands = seed_for_command.child_spawn_commands(&mut ParticleRandom::new(0));
    assert_eq!(commands.len(), 6);
    for command in &commands {
        assert_eq!(command.particle_type_id, 23);
        assert_eq!(command.particle_id, "minecraft:explosion");
        assert_close3(command.velocity, [2.0 / 8.0, 0.0, 0.0]);
        assert_range_f64(command.position[0], -3.0, 5.0);
        assert_range_f64(command.position[1], -4.0, 4.0);
        assert_range_f64(command.position[2], -4.0, 4.0);
    }

    let mut particles = ParticleRuntimeState::with_capacities_and_seed(32, 32, 0);
    let mut seed = spawn_command("minecraft:explosion_emitter", 1.0);
    seed.child_spawn_templates = vec![explosion_template];
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![seed],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    let seed = &particles.active_instances()[0];
    assert_eq!(seed.provider, "HugeExplosionSeedParticle.Provider");
    assert_eq!(seed.render_group, ParticleRenderGroup::NoRender);
    assert_eq!(seed.lifetime_ticks, 8);

    let summary = particles.advance(1);

    assert_eq!(summary.intaken_instances, 6);
    assert_eq!(summary.active_instances, 7);
    let seed = &particles.active_instances()[0];
    assert_eq!(seed.age_ticks, 1);
    for child in particles.active_instances().iter().skip(1) {
        assert_eq!(child.particle_type_id, 23);
        assert_eq!(child.particle_id, "minecraft:explosion");
        assert_eq!(child.provider, "HugeExplosionParticle.Provider");
        assert_eq!(
            child.current_sprite_id.as_deref(),
            Some("minecraft:explosion_0")
        );
        assert_close_f32(child.base_quad_size, 2.0);
        assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
        assert!(child.child_spawn_templates.is_empty());
    }

    let summary = particles.advance(1);

    assert_eq!(summary.intaken_instances, 6);
    assert_eq!(summary.active_instances, 13);
    for child in particles.active_instances().iter().skip(7) {
        assert_close_f32(child.base_quad_size, 1.875);
    }
}

#[test]
fn particle_runtime_gust_seed_emits_vanilla_child_gusts() {
    let gust_template = ParticleChildSpawnTemplate {
        particle_type_id: 24,
        particle_id: "minecraft:gust".to_string(),
        sprite_ids: vec!["minecraft:gust_0".to_string()],
    };
    let mut seed_for_command = test_instance_with_lifetime("minecraft:gust_emitter_large", 8);
    seed_for_command.position = [1.0, 0.0, 0.0];
    seed_for_command.age_ticks = 2;
    seed_for_command.child_spawn_templates = vec![gust_template.clone()];
    let commands = seed_for_command.child_spawn_commands(&mut ParticleRandom::new(0));
    assert_eq!(commands.len(), 3);
    for command in &commands {
        assert_eq!(command.particle_type_id, 24);
        assert_eq!(command.particle_id, "minecraft:gust");
        assert_close3(command.velocity, [1.0 / 7.0, 0.0, 0.0]);
    }

    let mut particles = ParticleRuntimeState::with_capacities_and_seed(16, 16, 0);
    let mut large = spawn_command("minecraft:gust_emitter_large", 1.0);
    large.child_spawn_templates = vec![gust_template.clone()];
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![large],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    let seed = &particles.active_instances()[0];
    assert_eq!(seed.provider, "GustSeedParticle.Provider(3.0,7,0)");
    assert_eq!(seed.render_group, ParticleRenderGroup::NoRender);
    assert_eq!(seed.lifetime_ticks, 8);

    let summary = particles.advance(1);

    assert_eq!(summary.intaken_instances, 3);
    assert_eq!(summary.active_instances, 4);
    let seed = &particles.active_instances()[0];
    assert_eq!(seed.age_ticks, 1);
    for child in particles.active_instances().iter().skip(1) {
        assert_eq!(child.particle_type_id, 24);
        assert_eq!(child.particle_id, "minecraft:gust");
        assert_eq!(child.provider, "GustParticle.Provider");
        assert_eq!(child.current_sprite_id.as_deref(), Some("minecraft:gust_0"));
        assert_range_f64(child.position[0], -2.0, 4.0);
        assert_range_f64(child.position[1], -3.0, 3.0);
        assert_range_f64(child.position[2], -3.0, 3.0);
        assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
        assert!(child.child_spawn_templates.is_empty());
    }

    let summary = particles.advance(1);

    assert_eq!(summary.intaken_instances, 3);
    assert_eq!(summary.active_instances, 7);
    for child in particles.active_instances().iter().skip(4) {
        assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
    }

    let mut particles = ParticleRuntimeState::with_capacities_and_seed(16, 16, 0);
    let mut small = spawn_command("minecraft:gust_emitter_small", 1.0);
    small.child_spawn_templates = vec![gust_template];
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![small],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);

    assert_eq!(particles.advance(1).intaken_instances, 3);
    assert_eq!(particles.advance(1).intaken_instances, 0);
    assert_eq!(particles.advance(1).intaken_instances, 0);
    assert_eq!(particles.advance(1).intaken_instances, 3);
}

#[test]
fn particle_runtime_negative_gravity_increases_y_velocity_before_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:poof", 20);
    instance.velocity = [0.0, 0.0, 0.0];
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.0, 0.004, 0.0]);
    assert_close3(instance.velocity, [0.0, 0.0036, 0.0]);
}

#[test]
fn particle_runtime_bubble_pop_subtracts_full_gravity_without_friction() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:bubble_pop", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.5, 0.25, -0.5];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.5, 2.242, 2.5]);
    assert_close3(instance.velocity, [0.5, 0.242, -0.5]);
}

#[test]
fn particle_runtime_firefly_first_tick_rerolls_speed_and_fades_alpha() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:firefly", 100);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.2, -0.3];
    instance.color[3] = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.1, 2.2, 2.7]);
    assert_range_f64(instance.velocity[0], -0.05, 0.05);
    assert_range_f64(instance.velocity[1], -0.05, 0.05);
    assert_range_f64(instance.velocity[2], -0.05, 0.05);
    assert_close_f32(instance.color[3], firefly_fade_amount(0.01, 0.3, 0.5));
}

#[test]
fn particle_runtime_firefly_removes_inside_non_air_block() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:firefly", 100);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |query| {
            assert_close3(query.position, [1.0, 2.0, 3.0]);
            ParticleBlockFluidSurfaceSample {
                block_is_air: false,
                ..ParticleBlockFluidSurfaceSample::default()
            }
        },
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
}

#[test]
fn particle_runtime_firefly_uses_collision_backed_default_move() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut instance = test_instance_with_lifetime("minecraft:firefly", 100);
    instance.age_ticks = 1;
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    instance.friction = 0.5;
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| {
            let mut movement = query.movement;
            movement[1] = 0.0;
            movement
        },
        |_query| ParticleBlockFluidSurfaceSample::default(),
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(instance.on_ground);
    assert!(instance.stopped_by_collision);
    assert_close3(instance.position, [1.2, 2.0, 2.4]);
    assert_close3(instance.velocity, [0.077, -0.2, -0.231]);
}

#[test]
fn particle_runtime_no_motion_tick_preserves_attack_sweep_position_and_velocity() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:sweep_attack", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = [0.0, 0.0, 0.0];
    instance.velocity = [0.5, 0.25, -0.5];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.0, 2.0, 3.0]);
    assert_close3(instance.velocity, [0.5, 0.25, -0.5]);
}

#[test]
fn particle_runtime_current_down_tick_uses_vanilla_swirl_motion() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:current_down", 30);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.05, 0.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| water_fluid_surface_sample(),
    );

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close3(instance.position, [1.042, 1.95, 3.0]);
    assert_close3(instance.velocity, [0.042, -0.05, 0.0]);
    assert_close_f32(instance.tick_angle, 0.08);
}

#[test]
fn particle_runtime_bubble_removes_outside_water() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:bubble", 30);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, 0.0, 0.0];
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample::default(),
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_bubble_column_stays_in_water() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:bubble_column_up", 30);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, 0.0, 0.0];
    instance.gravity = 0.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| water_fluid_surface_sample(),
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert!(!particles.active_instances()[0].removed);
}

#[test]
fn particle_runtime_current_down_removes_outside_water() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:current_down", 30);
    instance.position = [0.5, 0.25, 0.5];
    instance.previous_position = instance.position;
    instance.velocity = [0.0, -0.05, 0.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample::default(),
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_fly_towards_position_tick_uses_vanilla_curve() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:enchant", 40);
    instance.start_position = [10.0, 64.0, -2.0];
    instance.position = [6.0, 65.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [-4.0, 1.0, 5.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    let pos = 1.0_f64 - 1.0 / 40.0;
    let pp = (1.0 - pos).powi(4);
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [6.0, 65.0, 3.0]);
    assert_close3(
        instance.position,
        [
            10.0 - 4.0 * pos,
            64.0 + 1.0 * pos - pp * 1.2,
            -2.0 + 5.0 * pos,
        ],
    );
    assert_close3(instance.velocity, [-4.0, 1.0, 5.0]);
}

#[test]
fn particle_runtime_fly_straight_towards_uses_vanilla_curve_and_srgb_lerp() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:ominous_spawning", 25);
    instance.start_position = [1.0, 2.0, 3.0];
    instance.position = [1.25, 2.5, 2.25];
    instance.previous_position = instance.position;
    instance.velocity = [0.25, 0.5, -0.75];
    instance.color = [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0, 1.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [1.25, 2.5, 2.25]);
    assert_close3(instance.position, [1.24, 2.48, 2.28]);
    assert_close3(instance.velocity, [0.25, 0.5, -0.75]);
    assert_close_f32(instance.color[0], 76.0 / 255.0);
    assert_close_f32(instance.color[1], 177.0 / 255.0);
    assert_close_f32(instance.color[2], 254.0 / 255.0);
    assert_close_f32(instance.color[3], 1.0);
}

#[test]
fn particle_runtime_trail_tick_interpolates_toward_option_target() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:trail", 5);
    instance.position = [0.0, 0.0, 0.0];
    instance.previous_position = instance.position;
    instance.option_target = Some([4.0, 8.0, 12.0]);
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [0.0, 0.0, 0.0]);
    assert_close3(instance.position, [1.0, 2.0, 3.0]);

    let mut terminal = test_instance_with_lifetime("minecraft:trail", 5);
    terminal.age_ticks = 4;
    terminal.position = [3.0, 6.0, 9.0];
    terminal.previous_position = terminal.position;
    terminal.option_target = Some([4.0, 8.0, 12.0]);
    terminal.tick_motion_without_collision(&mut ParticleRandom::new(0));

    assert_close3(terminal.previous_position, [3.0, 6.0, 9.0]);
    assert_close3(terminal.position, [4.0, 8.0, 12.0]);
    assert!(terminal
        .position
        .iter()
        .all(|coordinate| coordinate.is_finite()));
}

#[test]
fn particle_runtime_vibration_tick_interpolates_toward_option_target_and_rotation() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:vibration", 5);
    instance.position = [0.0, 0.0, 0.0];
    instance.previous_position = instance.position;
    instance.option_target = Some([4.0, 8.0, 12.0]);
    instance.previous_yaw = 8.0;
    instance.yaw = 9.0;
    instance.previous_pitch = 6.0;
    instance.pitch = 7.0;
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    let (yaw, pitch) = vibration_particle_angles([1.0, 2.0, 3.0], [4.0, 8.0, 12.0]);
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [0.0, 0.0, 0.0]);
    assert_close3(instance.position, [1.0, 2.0, 3.0]);
    assert_close_f32(instance.previous_yaw, 9.0);
    assert_close_f32(instance.yaw, yaw);
    assert_close_f32(instance.previous_pitch, 7.0);
    assert_close_f32(instance.pitch, pitch);

    let mut terminal = test_instance_with_lifetime("minecraft:vibration", 5);
    terminal.age_ticks = 4;
    terminal.position = [3.0, 6.0, 9.0];
    terminal.previous_position = terminal.position;
    terminal.option_target = Some([4.0, 8.0, 12.0]);
    terminal.tick_motion_without_collision(&mut ParticleRandom::new(0));

    assert_close3(terminal.previous_position, [3.0, 6.0, 9.0]);
    assert_close3(terminal.position, [4.0, 8.0, 12.0]);
    assert!(terminal
        .position
        .iter()
        .all(|coordinate| coordinate.is_finite()));
}

#[test]
fn particle_runtime_vibration_refreshes_entity_target_each_tick() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:vibration", 5);
    instance.position = [0.0, 0.0, 0.0];
    instance.previous_position = instance.position;
    instance.option_target = Some([4.0, 8.0, 12.0]);
    instance.option_entity_target_source = Some(ParticleEntityTargetSource {
        entity_id: 77,
        y_offset: 0.5,
    });
    particles.active_instances.push_back(instance);
    let entity_targets = [ParticleEntityTargetContext {
        entity_id: 77,
        position: [8.0, 10.0, 12.0],
    }];

    let summary = particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &entity_targets,
    );

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    let target = [8.0, 10.5, 12.0];
    let (yaw, pitch) = vibration_particle_angles([2.0, 2.625, 3.0], target);
    assert_eq!(instance.option_target, Some(target));
    assert_close3(instance.previous_position, [0.0, 0.0, 0.0]);
    assert_close3(instance.position, [2.0, 2.625, 3.0]);
    assert_close_f32(instance.yaw, yaw);
    assert_close_f32(instance.pitch, pitch);
}

#[test]
fn particle_runtime_vibration_entity_target_missing_removes_particle() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:vibration", 5);
    instance.option_target = Some([4.0, 8.0, 12.0]);
    instance.option_entity_target_source = Some(ParticleEntityTargetSource {
        entity_id: 77,
        y_offset: 0.5,
    });
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world_and_particle_contexts(
        1,
        |query| query.movement,
        |_| ParticleBlockFluidSurfaceSample::default(),
        None,
        &[],
        &[],
    );

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_portal_tick_uses_vanilla_start_position_curve() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:portal", 40);
    instance.start_position = [10.0, 64.0, -2.0];
    instance.position = instance.start_position;
    instance.previous_position = instance.position;
    instance.velocity = [-5.0, 0.5, 5.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    let progress = 1.0_f64 / 40.0;
    let position_scale = 1.0 - (-progress + progress * progress * 2.0);
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [10.0, 64.0, -2.0]);
    assert_close3(
        instance.position,
        [
            10.0 - 5.0 * position_scale,
            64.0 + 0.5 * position_scale + (1.0 - progress),
            -2.0 + 5.0 * position_scale,
        ],
    );
    assert_close3(instance.velocity, [-5.0, 0.5, 5.0]);
}

#[test]
fn particle_runtime_reverse_portal_tick_uses_incremental_age_scaled_velocity() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:reverse_portal", 60);
    instance.position = [10.0, 64.0, -2.0];
    instance.previous_position = instance.position;
    instance.velocity = [-6.0, 0.6, 6.0];
    particles.active_instances.push_back(instance);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    let instance = &particles.active_instances()[0];
    let speed_multiplier = 1.0_f64 / 60.0;
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.previous_position, [10.0, 64.0, -2.0]);
    assert_close3(
        instance.position,
        [
            10.0 - 6.0 * speed_multiplier,
            64.0 + 0.6 * speed_multiplier,
            -2.0 + 6.0 * speed_multiplier,
        ],
    );
    assert_close3(instance.velocity, [-6.0, 0.6, 6.0]);
}

#[test]
fn particle_runtime_moves_particles_even_when_physics_is_disabled() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:angry_villager", 20);
    assert!(!instance.has_physics);
    instance.velocity = [0.25, 0.5, 0.75];
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [0.25, 0.5, 0.75]);
    assert_close3(instance.velocity, [0.215, 0.43, 0.645]);
}

#[test]
fn particle_runtime_expires_after_vanilla_post_increment_lifetime_boundary() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles
        .active_instances
        .push_back(test_instance_with_lifetime("minecraft:poof", 2));

    let summary = particles.advance(2);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    assert_eq!(particles.active_instances()[0].age_ticks, 2);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.total_instances_expired, 1);
    assert_eq!(summary.active_instances, 0);
    assert!(particles.active_instances().is_empty());
}

#[test]
fn particle_runtime_ticks_existing_active_before_intaking_pending_spawns() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:cloud", 1.0)],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:flame", 2.0)],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(1);

    assert_eq!(summary.ticks, 1);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 2);
    assert_eq!(
        particles.active_instances()[0].particle_id,
        "minecraft:cloud"
    );
    assert_eq!(particles.active_instances()[0].age_ticks, 1);
    assert_eq!(
        particles.active_instances()[1].particle_id,
        "minecraft:flame"
    );
    assert_eq!(particles.active_instances()[1].age_ticks, 0);
    assert_eq!(particles.active_instances()[1].position, [2.0, 0.0, 0.0]);
}

#[test]
fn particle_runtime_updates_age_based_sprite_frames_after_tick() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:smoke", 4);
    instance.sprite_ids = vec![
        "minecraft:smoke_0".to_string(),
        "minecraft:smoke_1".to_string(),
        "minecraft:smoke_2".to_string(),
    ];
    instance.current_sprite_index = Some(0);
    instance.current_sprite_id = Some("minecraft:smoke_0".to_string());
    particles.active_instances.push_back(instance);

    particles.advance(2);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 2);
    assert_eq!(instance.current_sprite_index, Some(1));
    assert_eq!(
        instance.current_sprite_id.as_deref(),
        Some("minecraft:smoke_1")
    );
}

#[test]
fn particle_runtime_age_based_sprite_reaches_last_frame_at_lifetime_boundary() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:poof", 2);
    instance.sprite_ids = vec![
        "minecraft:poof_0".to_string(),
        "minecraft:poof_1".to_string(),
        "minecraft:poof_2".to_string(),
    ];
    instance.current_sprite_index = Some(0);
    instance.current_sprite_id = Some("minecraft:poof_0".to_string());
    particles.active_instances.push_back(instance);

    particles.advance(2);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 2);
    assert_eq!(instance.current_sprite_index, Some(2));
    assert_eq!(
        instance.current_sprite_id.as_deref(),
        Some("minecraft:poof_2")
    );
}

#[test]
fn particle_runtime_keeps_random_sprite_selection_stable_after_tick() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 4, 0);
    let mut command = spawn_command("minecraft:flame", 1.0);
    command.sprite_ids = vec![
        "minecraft:flame_0".to_string(),
        "minecraft:flame_1".to_string(),
        "minecraft:flame_2".to_string(),
    ];
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });
    particles.advance(0);
    let initial_sprite = particles.active_instances()[0].current_sprite_id.clone();
    assert!(initial_sprite.is_some());

    particles.advance(3);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(instance.current_sprite_id, initial_sprite);
}

#[test]
fn particle_runtime_simple_animated_alpha_fades_after_half_lifetime() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:totem_of_undying", 60);
    instance.age_ticks = 30;
    instance.color[3] = 1.0;
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 31);
    assert_close_f32(instance.color[3], 1.0 - 1.0 / 60.0);
}

#[test]
fn particle_runtime_squid_ink_alpha_fades_after_half_lifetime() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:squid_ink", 20);
    instance.age_ticks = 10;
    instance.color[3] = 1.0;
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 11);
    assert_close_f32(instance.color[3], 0.95);
}

#[test]
fn particle_runtime_squid_ink_drifts_downward_in_air_after_default_tick() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:squid_ink", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.2, -0.3];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample::default(),
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert_close3(instance.position, [1.1, 2.2, 2.7]);
    assert_close3(
        instance.velocity,
        [
            0.092,
            0.184 - descriptors::SQUID_INK_AIR_DOWNWARD_ACCELERATION,
            -0.276,
        ],
    );
}

#[test]
fn particle_runtime_squid_ink_keeps_velocity_inside_non_air_block() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:glow_squid_ink", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.2, -0.3];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_world(
        1,
        |query| query.movement,
        |_query| ParticleBlockFluidSurfaceSample {
            block_is_air: false,
            ..ParticleBlockFluidSurfaceSample::default()
        },
    );

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_close3(instance.position, [1.1, 2.2, 2.7]);
    assert_close3(instance.velocity, [0.092, 0.184, -0.276]);
}

#[test]
fn particle_runtime_end_rod_alpha_and_rgb_fade_after_half_lifetime() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:end_rod", 60);
    instance.age_ticks = 30;
    instance.color = [1.0, 1.0, 1.0, 1.0];
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 31);
    let fade = descriptors::END_ROD_FADE_COLOR;
    assert_close_f32(instance.color[0], 1.0 + (fade[0] - 1.0) * 0.2);
    assert_close_f32(instance.color[1], 1.0 + (fade[1] - 1.0) * 0.2);
    assert_close_f32(instance.color[2], 1.0 + (fade[2] - 1.0) * 0.2);
    assert_close_f32(instance.color[3], 1.0 - 1.0 / 60.0);
}

#[test]
fn particle_runtime_end_rod_move_ignores_collision_callback() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:end_rod", 60);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    assert!(instance.has_physics);
    assert!(instance.moves_without_collision);
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |_query| {
        collision_queries += 1;
        [0.0, 0.0, 0.0]
    });

    assert_eq!(collision_queries, 0);
    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(!instance.on_ground);
    assert!(!instance.stopped_by_collision);
    assert_close3(instance.position, [1.2, 1.5995, 2.4]);
    assert_close3(instance.velocity, [0.182, -0.364455, -0.546]);
}

#[test]
fn particle_runtime_flame_move_ignores_collision_callback() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:flame", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    assert!(instance.has_physics);
    assert!(instance.moves_without_collision);
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |_query| {
        collision_queries += 1;
        [0.0, 0.0, 0.0]
    });

    assert_eq!(collision_queries, 0);
    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(!instance.on_ground);
    assert!(!instance.stopped_by_collision);
    assert_close3(instance.position, [1.2, 1.6, 2.4]);
    assert_close3(instance.velocity, [0.192, -0.384, -0.576]);
}

#[test]
fn particle_runtime_suspended_town_move_ignores_collision_callback() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:happy_villager", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    assert!(instance.has_physics);
    assert!(instance.moves_without_collision);
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |_query| {
        collision_queries += 1;
        [0.0, 0.0, 0.0]
    });

    assert_eq!(collision_queries, 0);
    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(!instance.on_ground);
    assert!(!instance.stopped_by_collision);
    assert_close3(instance.position, [1.2, 1.6, 2.4]);
    assert_close3(instance.velocity, [0.198, -0.396, -0.594]);
}

#[test]
fn particle_runtime_crit_constructor_tick_advances_spawn_state() {
    let mut random = ParticleRandom::new(56);
    let mut command = spawn_command("minecraft:crit", 1.0);
    command.velocity = [0.5, 0.25, -0.5];

    let crit = ParticleInstance::from_spawn_command(command, &mut random);

    assert_eq!(crit.provider, "CritParticle.Provider");
    assert_eq!(crit.age_ticks, 1);
    assert_close3(crit.previous_position, [1.0, 0.0, 0.0]);
    assert_range_f64(crit.position[0], 1.19, 1.21);
    assert_range_f64(crit.position[1], 0.08, 0.10);
    assert_range_f64(crit.position[2], -0.21, -0.19);
    assert_range_f64(crit.velocity[0], 0.133, 0.147);
    assert_range_f64(crit.velocity[1], 0.056, 0.070);
    assert_range_f64(crit.velocity[2], -0.147, -0.133);
    assert!(!crit.on_ground);
    assert!(!crit.stopped_by_collision);
}

#[test]
fn particle_runtime_vault_connection_alpha_follows_vanilla_lifetime_window() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:vault_connection", 40);
    instance.age_ticks = 20;
    instance.color[3] = 0.0;
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 21);
    assert_close_f32(instance.color[3], 0.22);
}

#[test]
fn particle_runtime_sets_initial_sprite_from_spawn_command_sprites() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:smoke", 1.0)],
        ..ParticleSpawnBatch::default()
    });

    particles.advance(0);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.current_sprite_index, Some(0));
    assert_eq!(
        instance.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
}

#[test]
fn particle_runtime_handles_empty_sprite_sets_without_blocking_spawn() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut command = spawn_command("minecraft:smoke", 1.0);
    command.sprite_ids.clear();
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![command],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(0);

    assert_eq!(summary.intaken_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.current_sprite_index, None);
    assert_eq!(instance.current_sprite_id, None);
}

#[test]
fn particle_runtime_uses_age_selection_for_ash_family_particles() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![
            spawn_command("minecraft:ash", 1.0),
            spawn_command("minecraft:white_ash", 2.0),
            spawn_command("minecraft:white_smoke", 3.0),
        ],
        ..ParticleSpawnBatch::default()
    });

    particles.advance(0);

    let selections: Vec<_> = particles
        .active_instances()
        .iter()
        .map(|instance| (instance.particle_id.as_str(), instance.sprite_selection))
        .collect();
    assert_eq!(
        selections,
        vec![
            ("minecraft:ash", ParticleSpriteSelection::Age),
            ("minecraft:white_ash", ParticleSpriteSelection::Age),
            ("minecraft:white_smoke", ParticleSpriteSelection::Age),
        ]
    );
}

// Independent witness for `BaseAshSmokeParticle`: `Particle.java` 7-arg base
// spread (super called with xa=ya=za=0), reconstructed straight from the
// vanilla source lines so it does not lean on the descriptor under test.
fn base_ash_smoke_base_spread(seed: i64) -> [f64; 3] {
    let mut random = ParticleRandom::new(seed);
    let x = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let y = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let z = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
    let length = (x * x + y * y + z * z).sqrt();
    [
        x / length * speed * 0.4,
        y / length * speed * 0.4 + 0.1,
        z / length * speed * 0.4,
    ]
}

// Full `BaseAshSmokeParticle` velocity: base spread times per-axis `dir`
// (`xd *= dirX; yd *= dirY; zd *= dirZ`) plus the provider velocity
// (`xd += xa; yd += ya; zd += za`). `white_ash` draws the same negative-biased
// xa/ya/za as `WhiteAshParticle.Provider`; `ash` adds `(0, 0, 0)`.
fn expected_base_ash_smoke_velocity(seed: i64, dir: [f64; 3], white_ash: bool) -> [f64; 3] {
    let mut random = ParticleRandom::new(seed);
    let x = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let y = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let z = (f64::from(random.next_f32()) * 2.0 - 1.0) * 0.4;
    let speed = (f64::from(random.next_f32()) + f64::from(random.next_f32()) + 1.0) * 0.15;
    let length = (x * x + y * y + z * z).sqrt();
    let base = [
        x / length * speed * 0.4,
        y / length * speed * 0.4 + 0.1,
        z / length * speed * 0.4,
    ];
    let offset = if white_ash {
        [
            f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
            f64::from(random.next_f32()) * -0.5 * f64::from(random.next_f32()) * 0.1 * 5.0,
            f64::from(random.next_f32()) * -1.9 * f64::from(random.next_f32()) * 0.1,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };
    [
        base[0] * dir[0] + offset[0],
        base[1] * dir[1] + offset[1],
        base[2] * dir[2] + offset[2],
    ]
}

#[test]
fn ash_provider_applies_per_axis_dir_to_base_spread_and_ignores_command_velocity() {
    // AshParticle.Provider.createParticle forces provider velocity (0, 0, 0);
    // the incoming command velocity must be dropped.
    let mut command = spawn_command("minecraft:ash", 5.0);
    command.velocity = [3.0, 4.0, 5.0];
    let mut random = ParticleRandom::new(0);

    let ash = ParticleInstance::from_spawn_command(command, &mut random);

    let dir = [0.1, -0.1, 0.1];
    let expected = expected_base_ash_smoke_velocity(0, dir, false);
    assert_close_f64(ash.velocity[0], expected[0]);
    assert_close_f64(ash.velocity[1], expected[1]);
    assert_close_f64(ash.velocity[2], expected[2]);

    // Per-axis dir: x/z scaled by 0.1, y negated and damped by 0.1.
    let base = base_ash_smoke_base_spread(0);
    assert_close_f64(ash.velocity[0], base[0] * 0.1);
    assert_close_f64(ash.velocity[1], base[1] * -0.1);
    assert_close_f64(ash.velocity[2], base[2] * 0.1);

    // Command velocity is fully ignored: nowhere near [3, 4, 5].
    assert_ne!(ash.velocity, [3.0, 4.0, 5.0]);
    assert!(ash.velocity[0].abs() < 0.02);
    assert!(ash.velocity[1].abs() < 0.03);
    assert!(ash.velocity[2].abs() < 0.02);
}

#[test]
fn white_ash_provider_adds_negative_biased_offset_to_per_axis_base_spread() {
    // WhiteAshParticle.Provider.createParticle ignores the command velocity and
    // adds its own negative-biased xa/ya/za on top of the dir-scaled spread.
    let mut command = spawn_command("minecraft:white_ash", 5.0);
    command.velocity = [3.0, 4.0, 5.0];
    let mut random = ParticleRandom::new(0);

    let white_ash = ParticleInstance::from_spawn_command(command, &mut random);

    let dir = [0.1, -0.1, 0.1];
    let expected = expected_base_ash_smoke_velocity(0, dir, true);
    assert_close_f64(white_ash.velocity[0], expected[0]);
    assert_close_f64(white_ash.velocity[1], expected[1]);
    assert_close_f64(white_ash.velocity[2], expected[2]);

    // Command velocity is ignored.
    assert_ne!(white_ash.velocity, [3.0, 4.0, 5.0]);

    // The provider offset makes white_ash diverge from the ash zero-offset
    // branch at the same seed. Because `ya = rand*-0.5*rand*0.1*5.0 <= 0`, the
    // extra provider velocity always biases the y component downward.
    let ash_only = expected_base_ash_smoke_velocity(0, dir, false);
    assert_ne!(white_ash.velocity, ash_only);
    assert!(white_ash.velocity[1] < ash_only[1]);
}

#[test]
fn particle_runtime_ash_no_physics_ignores_collision_callback() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:ash", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.2, -0.4, -0.6];
    instance.friction = 0.5;
    instance.gravity = 0.0;
    assert!(!instance.has_physics);
    assert!(instance.speed_up_when_y_motion_is_blocked);
    particles.active_instances.push_back(instance);

    let mut collision_queries = 0;
    let summary = particles.advance_with_collision(1, |_query| {
        collision_queries += 1;
        [0.0, 0.0, 0.0]
    });

    assert_eq!(collision_queries, 0);
    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert!(!instance.on_ground);
    assert_close3(instance.position, [1.2, 1.6, 2.4]);
    assert_close3(instance.velocity, [0.1, -0.2, -0.3]);
}

#[test]
fn dust_plume_provider_adds_command_velocity_and_y_offset_to_per_axis_base_spread() {
    // DustPlumeParticle.Provider passes the command velocity xAux/yAux/zAux as
    // xa/ya/za; DustPlumeParticle calls
    // super(..., 0.7F, 0.6F, 0.7F, xa, ya + 0.15F, za, ...), so the Particle
    // base spread is scaled per axis by (0.7, 0.6, 0.7) and the command
    // velocity (with +0.15 on y) is added on top.
    let mut command = spawn_command("minecraft:dust_plume", 1.0);
    command.velocity = [0.25, 0.5, -0.75];
    let mut random = ParticleRandom::new(86);

    let dust_plume = ParticleInstance::from_spawn_command(command, &mut random);

    let dir = [0.7, 0.6, 0.7];
    let spread = expected_base_ash_smoke_velocity(86, dir, false);
    assert_close_f64(dust_plume.velocity[0], spread[0] + 0.25);
    assert_close_f64(dust_plume.velocity[1], spread[1] + 0.5 + 0.15);
    assert_close_f64(dust_plume.velocity[2], spread[2] - 0.75);

    // Unlike the old CommandWithYOffset path, the per-axis base spread is now
    // applied, so the result is not exactly command velocity + 0.15 on y.
    assert_ne!(dust_plume.velocity, [0.25, 0.65, -0.75]);
}

#[test]
fn firework_spark_provider_uses_vanilla_simple_animated_state() {
    let mut random = ParticleRandom::new(71);
    let mut command = spawn_command("minecraft:firework", 1.0);
    command.sprite_ids = vec![
        "minecraft:firework_0".to_string(),
        "minecraft:firework_1".to_string(),
        "minecraft:firework_2".to_string(),
    ];
    command.velocity = [0.1, 0.2, 0.3];
    command.option_color = Some([
        0x11 as f32 / 255.0,
        0x22 as f32 / 255.0,
        0x33 as f32 / 255.0,
        0.99,
    ]);
    command.option_color_to = Some([
        0x77 as f32 / 255.0,
        0x88 as f32 / 255.0,
        0x99 as f32 / 255.0,
        1.0,
    ]);
    command.option_firework_trail = true;
    command.option_firework_twinkle = true;

    let firework = ParticleInstance::from_spawn_command(command, &mut random);

    assert_eq!(firework.provider, "FireworkParticles.SparkProvider");
    assert_eq!(firework.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(
        firework.current_sprite_id.as_deref(),
        Some("minecraft:firework_0")
    );
    assert_range_f32(firework.base_quad_size, 0.075, 0.15);
    assert!((48..=59).contains(&firework.lifetime_ticks));
    assert_eq!(
        firework.color,
        [
            0x11 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x33 as f32 / 255.0,
            0.99,
        ]
    );
    assert_eq!(
        firework.color_fade_target,
        Some([
            0x77 as f32 / 255.0,
            0x88 as f32 / 255.0,
            0x99 as f32 / 255.0,
        ])
    );
    assert_eq!(firework.velocity, [0.1, 0.2, 0.3]);
    assert!(firework.firework_trail);
    assert!(firework.firework_twinkle);
    assert_eq!(firework.friction, 0.91);
    assert_eq!(firework.gravity, 0.1);
    assert!(firework.has_physics);
    assert_eq!(firework.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(
        firework.light_emission,
        ParticleLightEmissionDescriptor::FullBright
    );
}

#[test]
fn particle_runtime_firework_spark_alpha_preserves_initial_then_fades() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:firework", 48);
    instance.color[3] = 0.99;
    instance.age_ticks = 23;
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &mut particles.active_instances[0];
    assert_eq!(instance.age_ticks, 24);
    assert_close_f32(instance.color[3], 0.99);
    instance.age_ticks = 24;
    instance.color[3] = 0.99;

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 25);
    assert_close_f32(instance.color[3], 1.0 - 1.0 / 48.0);
}

#[test]
fn particle_runtime_firework_trail_spawns_half_lifetime_twinkle_child() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(4, 8, 99);
    let mut parent = test_instance_with_lifetime("minecraft:firework", 48);
    parent.particle_type_id = 30;
    parent.sprite_ids = vec!["minecraft:firework_0".to_string()];
    parent.current_sprite_id = Some("minecraft:firework_0".to_string());
    parent.position = [1.0, 2.0, 3.0];
    parent.previous_position = parent.position;
    parent.velocity = [0.0, 0.0, 0.0];
    parent.age_ticks = 21;
    parent.color = [0.2, 0.4, 0.6, 0.99];
    parent.color_fade_target = Some([0.8, 0.7, 0.6]);
    parent.firework_trail = true;
    parent.firework_twinkle = true;
    particles.active_instances.push_back(parent);

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 2);
    let parent = &particles.active_instances()[0];
    assert_eq!(parent.age_ticks, 22);
    assert!(parent.firework_trail);
    let child = &particles.active_instances()[1];
    assert_eq!(child.particle_id, "minecraft:firework");
    assert!(!child.firework_trail);
    assert!(child.firework_twinkle);
    assert_eq!(child.age_ticks, child.lifetime_ticks / 2);
    assert_eq!(child.color, [0.2, 0.4, 0.6, 0.99]);
    assert_eq!(child.color_fade_target, Some([0.8, 0.7, 0.6]));
    assert_eq!(child.velocity, [0.0, 0.0, 0.0]);
}

#[test]
fn particle_render_color_hides_firework_twinkle_frames() {
    let mut twinkle = test_instance_with_lifetime("minecraft:firework", 48);
    twinkle.firework_twinkle = true;
    twinkle.color = [1.0, 1.0, 1.0, 0.99];

    twinkle.age_ticks = 15;
    assert_close_f32(particle_render_color(&twinkle)[3], 0.99);
    twinkle.age_ticks = 16;
    assert_eq!(particle_render_color(&twinkle)[3], 0.0);
    twinkle.age_ticks = 18;
    assert_close_f32(particle_render_color(&twinkle)[3], 0.99);
}

#[test]
fn particle_runtime_mirrors_falling_leaves_provider_state_and_motion() {
    let mut particles = ParticleRuntimeState::with_capacities_and_seed(8, 8, 7);
    let mut cherry = spawn_command("minecraft:cherry_leaves", 1.0);
    cherry.sprite_ids = vec![
        "minecraft:cherry_leaf_0".to_string(),
        "minecraft:cherry_leaf_1".to_string(),
    ];
    let mut pale_oak = spawn_command("minecraft:pale_oak_leaves", 1.0);
    pale_oak.sprite_ids = vec![
        "minecraft:pale_oak_leaf_0".to_string(),
        "minecraft:pale_oak_leaf_1".to_string(),
    ];
    let mut tinted = spawn_command("minecraft:tinted_leaves", 1.0);
    tinted.sprite_ids = vec![
        "minecraft:tinted_leaf_0".to_string(),
        "minecraft:tinted_leaf_1".to_string(),
    ];
    tinted.option_color = Some([0.25, 0.5, 0.75, 0.25]);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![cherry, pale_oak, tinted],
        ..ParticleSpawnBatch::default()
    });

    particles.advance(0);

    let initial_sprites: Vec<_> = particles
        .active_instances()
        .iter()
        .map(|instance| instance.current_sprite_id.clone())
        .collect();
    let cherry = &particles.active_instances()[0];
    assert_eq!(cherry.provider, "FallingLeavesParticle.CherryProvider");
    assert_eq!(cherry.sprite_selection, ParticleSpriteSelection::Random);
    assert!(matches!(
        cherry.current_sprite_id.as_deref(),
        Some("minecraft:cherry_leaf_0" | "minecraft:cherry_leaf_1")
    ));
    assert_eq!(cherry.lifetime_ticks, 300);
    assert_range_f32(cherry.base_quad_size, 0.05, 0.075);
    assert_eq!(cherry.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(cherry.velocity, [0.0, -0.0, 0.0]);
    assert_eq!(cherry.friction, 1.0);
    assert_close_f32(cherry.gravity, 0.00075);
    assert_eq!(
        cherry.tick_motion,
        ParticleTickMotionDescriptor::FallingLeaves
    );
    assert_eq!(cherry.render_layer, ParticleRenderLayer::Opaque);
    assert!(cherry.has_physics);
    let cherry_motion = cherry.falling_leaves_motion.expect("falling leaves motion");
    assert!(cherry_motion.flow_away);
    assert!(!cherry_motion.swirl);
    assert_close_f32(cherry_motion.wind_big, 2.0);

    let pale_oak = &particles.active_instances()[1];
    assert_eq!(pale_oak.provider, "FallingLeavesParticle.PaleOakProvider");
    assert!(matches!(
        pale_oak.current_sprite_id.as_deref(),
        Some("minecraft:pale_oak_leaf_0" | "minecraft:pale_oak_leaf_1")
    ));
    assert_range_f32(pale_oak.base_quad_size, 0.1, 0.15);
    assert_eq!(pale_oak.velocity, [0.0, -0.021, 0.0]);
    assert_close_f32(pale_oak.gravity, 0.00021);
    let pale_motion = pale_oak
        .falling_leaves_motion
        .expect("falling leaves motion");
    assert!(!pale_motion.flow_away);
    assert!(pale_motion.swirl);
    assert_close_f32(pale_motion.wind_big, 10.0);

    let tinted = &particles.active_instances()[2];
    assert_eq!(
        tinted.provider,
        "FallingLeavesParticle.TintedLeavesProvider"
    );
    assert!(matches!(
        tinted.current_sprite_id.as_deref(),
        Some("minecraft:tinted_leaf_0" | "minecraft:tinted_leaf_1")
    ));
    assert_range_f32(tinted.base_quad_size, 0.1, 0.15);
    assert_eq!(tinted.color, [0.25, 0.5, 0.75, 1.0]);
    assert_eq!(tinted.velocity, [0.0, -0.021, 0.0]);
    assert_close_f32(tinted.gravity, 0.00021);
    assert_eq!(tinted.render_layer, ParticleRenderLayer::Opaque);

    particles.advance(1);

    for (instance, initial_sprite) in particles.active_instances().iter().zip(initial_sprites) {
        assert_eq!(instance.age_ticks, 1);
        assert_eq!(instance.current_sprite_id, initial_sprite);
        assert_ne!(instance.roll, 0.0);
        assert_eq!(instance.previous_roll, 0.0);
    }
    let cherry = &particles.active_instances()[0];
    assert_close_f64(cherry.position[1], -0.00075);
    assert_close_f64(cherry.velocity[1], -0.00075);
    assert!(cherry.position[0] != 0.0 || cherry.position[2] != 0.0);
    let pale_oak = &particles.active_instances()[1];
    assert_close_f64(pale_oak.position[1], -0.02121);
    assert_close_f64(pale_oak.velocity[1], -0.02121);
    assert!(pale_oak.position[0] != 0.0 || pale_oak.position[2] != 0.0);
    let tinted = &particles.active_instances()[2];
    assert_close_f64(tinted.position[1], -0.02121);
    assert_close_f64(tinted.velocity[1], -0.02121);
    assert!(tinted.position[0] != 0.0 || tinted.position[2] != 0.0);
}

#[test]
fn particle_runtime_falling_leaves_removes_on_ground_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = falling_leaves_instance("minecraft:cherry_leaves", 11);
    instance.position = [0.0, 1.0, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, -0.2, 0.1];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        movement[1] = 0.0;
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
}

#[test]
fn particle_runtime_falling_leaves_keeps_first_tick_horizontal_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = falling_leaves_instance("minecraft:cherry_leaves", 13);
    instance.position = [0.0, 1.0, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.0, 0.1];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        movement[0] = 0.0;
        movement
    });

    assert_eq!(summary.expired_instances, 0);
    assert_eq!(summary.active_instances, 1);
    let instance = &particles.active_instances()[0];
    assert_eq!(instance.age_ticks, 1);
    assert!(!instance.on_ground);
    assert_eq!(instance.velocity[0], 0.0);
}

#[test]
fn particle_runtime_falling_leaves_removes_on_later_horizontal_collision() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = falling_leaves_instance("minecraft:cherry_leaves", 17);
    instance.age_ticks = 1;
    instance.position = [0.0, 1.0, 0.0];
    instance.previous_position = instance.position;
    instance.velocity = [0.1, 0.0, 0.1];
    particles.active_instances.push_back(instance);

    let summary = particles.advance_with_collision(1, |query| {
        let mut movement = query.movement;
        movement[0] = 0.0;
        movement
    });

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.active_instances, 0);
}

#[test]
fn particle_runtime_falling_leaves_collision_size_matches_per_spawn_quad_size() {
    // FallingLeavesParticle.java:41-43: `float size = scale * (nextBoolean ? 0.05F
    // : 0.075F); this.quadSize = size; this.setSize(size, size);`. bbb reuses the
    // sampled `base_quad_size` for the collision box, so the two must stay in
    // lockstep bit-for-bit and land on one of the two vanilla per-spawn choices.
    // `scale` = 1.0 (Cherry) / 2.0 (PaleOak & Tinted).
    for (particle_id, provider, scale) in [
        (
            "minecraft:cherry_leaves",
            "FallingLeavesParticle.CherryProvider",
            1.0_f32,
        ),
        (
            "minecraft:pale_oak_leaves",
            "FallingLeavesParticle.PaleOakProvider",
            2.0,
        ),
        (
            "minecraft:tinted_leaves",
            "FallingLeavesParticle.TintedLeavesProvider",
            2.0,
        ),
    ] {
        let small = scale * 0.05;
        let large = scale * 0.075;
        let mut saw_small = false;
        let mut saw_large = false;
        for seed in 0..24 {
            let instance = falling_leaves_instance(particle_id, seed);
            assert_eq!(instance.provider, provider);
            // Collision box tracks the sampled quad size exactly.
            assert_eq!(instance.collision_width, instance.base_quad_size);
            assert_eq!(instance.collision_height, instance.base_quad_size);
            // And equals one of the two vanilla per-spawn `size` choices.
            if instance.collision_width == small {
                saw_small = true;
            } else if instance.collision_width == large {
                saw_large = true;
            } else {
                panic!(
                    "{particle_id} seed {seed}: collision {} not in {{{small}, {large}}}",
                    instance.collision_width
                );
            }
        }
        // The `nextBoolean` branch is genuinely per-spawn random: both sizes appear.
        assert!(
            saw_small && saw_large,
            "{particle_id}: expected both per-spawn sizes"
        );
    }
}

#[test]
fn particle_instances_sample_provider_visual_state() {
    let mut flame_random = ParticleRandom::new(42);
    let flame = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:flame", 1.0),
        &mut flame_random,
    );
    let mut small_flame_random = ParticleRandom::new(42);
    let small_flame = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:small_flame", 1.0),
        &mut small_flame_random,
    );
    assert_close_f32(small_flame.base_quad_size, flame.base_quad_size * 0.5);
    assert_eq!(flame.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(flame.quad_size_curve, ParticleQuadSizeCurve::Flame);
    assert!(flame.has_physics);
    assert!(flame.moves_without_collision);
    assert!(small_flame.has_physics);
    assert!(small_flame.moves_without_collision);

    let mut cosy_random = ParticleRandom::new(46);
    let mut cosy_command = spawn_command("minecraft:campfire_cosy_smoke", 1.0);
    cosy_command.velocity = [0.1, 0.2, 0.3];
    let cosy = ParticleInstance::from_spawn_command(cosy_command, &mut cosy_random);
    assert_eq!(cosy.provider, "CampfireSmokeParticle.CosyProvider");
    assert_eq!(cosy.sprite_selection, ParticleSpriteSelection::Random);
    assert_range_f32(cosy.base_quad_size, 0.3, 0.6);
    assert_eq!(cosy.color, [1.0, 1.0, 1.0, 0.9]);
    assert!((80..=129).contains(&cosy.lifetime_ticks));
    assert_eq!(cosy.velocity[0], 0.1);
    assert_range_f64(cosy.velocity[1], 0.2, 0.202);
    assert_eq!(cosy.velocity[2], 0.3);
    assert_eq!(cosy.gravity, 3.0E-6);
    assert_eq!(
        cosy.tick_motion,
        ParticleTickMotionDescriptor::CampfireSmoke
    );
    assert_eq!(cosy.render_layer, ParticleRenderLayer::Translucent);

    let mut signal_random = ParticleRandom::new(47);
    let signal = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:campfire_signal_smoke", 1.0),
        &mut signal_random,
    );
    assert_eq!(signal.provider, "CampfireSmokeParticle.SignalProvider");
    assert_range_f32(signal.base_quad_size, 0.3, 0.6);
    assert_eq!(signal.color, [1.0, 1.0, 1.0, 0.95]);
    assert!((280..=329).contains(&signal.lifetime_ticks));
    assert_eq!(signal.render_layer, ParticleRenderLayer::Translucent);

    let mut lava_random = ParticleRandom::new(44);
    let lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:lava", 1.0),
        &mut lava_random,
    );
    assert_eq!(lava.provider, "LavaParticle.Provider");
    assert_eq!(lava.sprite_selection, ParticleSpriteSelection::Random);
    assert_range_f32(lava.base_quad_size, 0.02, 0.44);
    assert_eq!(lava.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(lava.quad_size_curve, ParticleQuadSizeCurve::Lava);
    assert!((16..=80).contains(&lava.lifetime_ticks));
    assert_range_f64(lava.velocity[0], -0.15, 0.15);
    assert_range_f64(lava.velocity[1], 0.05, 0.45);
    assert_range_f64(lava.velocity[2], -0.15, 0.15);
    assert_eq!(lava.friction, 0.999);
    assert_eq!(lava.gravity, 0.75);
    assert!(lava.has_physics);
    assert_eq!(
        lava.child_emission,
        Some(ParticleChildEmissionDescriptor::LavaSmoke)
    );

    let mut soul_random = ParticleRandom::new(68);
    let mut soul_command = spawn_command("minecraft:soul", 1.0);
    soul_command.position = [1.0, 2.0, 3.0];
    soul_command.velocity = [1.0, 2.0, 3.0];
    let soul = ParticleInstance::from_spawn_command(soul_command, &mut soul_random);
    assert_eq!(soul.provider, "SoulParticle.Provider");
    assert_eq!(soul.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(soul.base_quad_size, 0.15, 0.3);
    assert_eq!(soul.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(soul.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!((12..=44).contains(&soul.lifetime_ticks));
    assert_eq!(soul.friction, 0.96);
    assert_eq!(soul.gravity, 0.0);
    assert!(soul.has_physics);
    assert!(!soul.speed_up_when_y_motion_is_blocked);
    assert_range_f64(soul.position[0], 0.95, 1.05);
    assert_range_f64(soul.position[1], 1.95, 2.05);
    assert_range_f64(soul.position[2], 2.95, 3.05);
    assert_range_f64(soul.velocity[0], 0.998, 1.002);
    assert_range_f64(soul.velocity[1], 2.0, 2.003);
    assert_range_f64(soul.velocity[2], 2.998, 3.002);

    let mut sculk_soul_random = ParticleRandom::new(69);
    let sculk_soul = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:sculk_soul", 1.0),
        &mut sculk_soul_random,
    );
    assert_eq!(sculk_soul.provider, "SoulParticle.EmissiveProvider");
    assert_eq!(sculk_soul.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(sculk_soul.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((12..=44).contains(&sculk_soul.lifetime_ticks));
    assert!(sculk_soul.has_physics);

    let mut cloud_random = ParticleRandom::new(43);
    let cloud = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:cloud", 1.0),
        &mut cloud_random,
    );
    assert_eq!(cloud.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(cloud.base_quad_size, 0.1875, 0.375);
    assert_range_f32(cloud.color[0], 0.7, 1.0);
    assert_eq!(cloud.color[0], cloud.color[1]);
    assert_eq!(cloud.color[1], cloud.color[2]);
    assert_ne!(cloud.velocity, [0.0, 0.0, 0.0]);

    let mut bubble_random = ParticleRandom::new(59);
    let mut bubble_command = spawn_command("minecraft:bubble", 1.0);
    bubble_command.velocity = [1.0, 2.0, 3.0];
    let bubble = ParticleInstance::from_spawn_command(bubble_command, &mut bubble_random);
    assert_eq!(bubble.provider, "BubbleParticle.Provider");
    assert_eq!(bubble.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(bubble.base_quad_size, 0.02, 0.16);
    assert_eq!(bubble.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((8..=40).contains(&bubble.lifetime_ticks));
    assert_eq!(bubble.friction, 0.85);
    assert_eq!(bubble.gravity, -0.05);
    assert!(bubble.has_physics);
    assert_range_f64(bubble.velocity[0], 0.18, 0.22);
    assert_range_f64(bubble.velocity[1], 0.38, 0.42);
    assert_range_f64(bubble.velocity[2], 0.58, 0.62);
    assert_eq!(bubble.required_fluid, Some(ParticleFluidKind::Water));

    let mut rain_random = ParticleRandom::new(62);
    let rain = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:rain", 1.0),
        &mut rain_random,
    );
    assert_eq!(rain.provider, "WaterDropParticle.Provider");
    assert_eq!(rain.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(rain.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(rain.base_quad_size, 0.1, 0.2);
    assert_eq!(rain.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((8..=40).contains(&rain.lifetime_ticks));
    assert_eq!(rain.friction, 0.98);
    assert_eq!(rain.gravity, 0.06);
    assert!(rain.has_physics);
    assert!(!rain.speed_up_when_y_motion_is_blocked);
    assert_eq!(rain.tick_motion, ParticleTickMotionDescriptor::WaterDrop);
    assert_eq!(rain.render_layer, ParticleRenderLayer::Opaque);
    assert_range_f64(rain.velocity[0], -0.06, 0.06);
    assert_range_f64(rain.velocity[1], 0.1, 0.3);
    assert_range_f64(rain.velocity[2], -0.06, 0.06);

    let mut splash_random = ParticleRandom::new(63);
    let mut splash_command = spawn_command("minecraft:splash", 1.0);
    splash_command.velocity = [0.25, 0.0, -0.75];
    let splash = ParticleInstance::from_spawn_command(splash_command, &mut splash_random);
    assert_eq!(splash.provider, "SplashParticle.Provider");
    assert_eq!(splash.sprite_selection, ParticleSpriteSelection::Random);
    assert_range_f32(splash.base_quad_size, 0.1, 0.2);
    assert!((8..=40).contains(&splash.lifetime_ticks));
    assert_eq!(splash.velocity, [0.25, 0.1, -0.75]);
    assert_eq!(splash.friction, 0.98);
    assert_eq!(splash.gravity, 0.04);
    assert!(splash.has_physics);
    assert_eq!(splash.tick_motion, ParticleTickMotionDescriptor::WaterDrop);
    assert_eq!(splash.render_layer, ParticleRenderLayer::Opaque);

    let mut wake_random = ParticleRandom::new(64);
    let mut wake_command = spawn_command("minecraft:fishing", 1.0);
    wake_command.velocity = [0.25, 0.5, -0.75];
    wake_command.sprite_ids = vec![
        "minecraft:wake_0".to_string(),
        "minecraft:wake_1".to_string(),
        "minecraft:wake_2".to_string(),
        "minecraft:wake_3".to_string(),
        "minecraft:wake_4".to_string(),
    ];
    let wake = ParticleInstance::from_spawn_command(wake_command, &mut wake_random);
    assert_eq!(wake.provider, "WakeParticle.Provider");
    assert_eq!(wake.sprite_selection, ParticleSpriteSelection::First);
    assert_eq!(wake.current_sprite_index, Some(0));
    assert_eq!(wake.current_sprite_id.as_deref(), Some("minecraft:wake_0"));
    assert_eq!(wake.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(wake.base_quad_size, 0.1, 0.2);
    assert!((8..=40).contains(&wake.lifetime_ticks));
    assert_eq!(wake.velocity, [0.25, 0.5, -0.75]);
    assert_eq!(wake.friction, 0.98);
    assert_eq!(wake.gravity, 0.0);
    assert!(wake.has_physics);
    assert_eq!(wake.tick_motion, ParticleTickMotionDescriptor::Wake);
    assert_eq!(wake.render_layer, ParticleRenderLayer::Opaque);

    let mut ominous_spawn_random = ParticleRandom::new(65);
    let mut ominous_spawn_command = spawn_command("minecraft:ominous_spawning", 1.0);
    ominous_spawn_command.position = [1.0, 2.0, 3.0];
    ominous_spawn_command.velocity = [0.25, 0.5, -0.75];
    ominous_spawn_command.sprite_ids = vec![
        "minecraft:ominous_spawn_0".to_string(),
        "minecraft:ominous_spawn_1".to_string(),
    ];
    let ominous_spawn =
        ParticleInstance::from_spawn_command(ominous_spawn_command, &mut ominous_spawn_random);
    assert_eq!(
        ominous_spawn.provider,
        "FlyStraightTowardsParticle.OminousSpawnProvider"
    );
    assert_eq!(
        ominous_spawn.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert!(matches!(ominous_spawn.current_sprite_index, Some(0 | 1)));
    assert!(matches!(
        ominous_spawn.current_sprite_id.as_deref(),
        Some("minecraft:ominous_spawn_0" | "minecraft:ominous_spawn_1")
    ));
    assert_eq!(ominous_spawn.start_position, [1.0, 2.0, 3.0]);
    assert_eq!(ominous_spawn.previous_position, [1.25, 2.5, 2.25]);
    assert_eq!(ominous_spawn.position, [1.25, 2.5, 2.25]);
    assert_eq!(ominous_spawn.velocity, [0.25, 0.5, -0.75]);
    assert_range_f32(ominous_spawn.base_quad_size, 0.06, 0.35);
    assert_eq!(
        ominous_spawn.color,
        [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0, 1.0]
    );
    assert!((25..=29).contains(&ominous_spawn.lifetime_ticks));
    assert_eq!(ominous_spawn.friction, 0.98);
    assert_eq!(ominous_spawn.gravity, 0.0);
    assert!(!ominous_spawn.has_physics);
    assert_eq!(
        ominous_spawn.tick_motion,
        ParticleTickMotionDescriptor::FlyStraightTowards
    );
    assert_eq!(ominous_spawn.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(
        ominous_spawn.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );

    let mut column_bubble_random = ParticleRandom::new(60);
    let mut column_bubble_command = spawn_command("minecraft:bubble_column_up", 1.0);
    column_bubble_command.velocity = [1.0, 2.0, 3.0];
    let column_bubble =
        ParticleInstance::from_spawn_command(column_bubble_command, &mut column_bubble_random);
    assert_eq!(column_bubble.provider, "BubbleColumnUpParticle.Provider");
    assert_eq!(
        column_bubble.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(column_bubble.base_quad_size, 0.02, 0.16);
    assert!((40..=200).contains(&column_bubble.lifetime_ticks));
    assert_eq!(column_bubble.friction, 0.85);
    assert_eq!(column_bubble.gravity, -0.125);
    assert!(column_bubble.has_physics);
    assert_range_f64(column_bubble.velocity[0], 0.18, 0.22);
    assert_range_f64(column_bubble.velocity[1], 0.38, 0.42);
    assert_range_f64(column_bubble.velocity[2], 0.58, 0.62);
    assert_eq!(column_bubble.required_fluid, Some(ParticleFluidKind::Water));

    let mut current_down_random = ParticleRandom::new(82);
    let mut current_down_command = spawn_command("minecraft:current_down", 1.0);
    current_down_command.velocity = [9.0, 9.0, 9.0];
    let current_down =
        ParticleInstance::from_spawn_command(current_down_command, &mut current_down_random);
    assert_eq!(current_down.provider, "WaterCurrentDownParticle.Provider");
    assert_eq!(
        current_down.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        current_down.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(current_down.base_quad_size, 0.02, 0.16);
    assert_eq!(current_down.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((30..=89).contains(&current_down.lifetime_ticks));
    assert_eq!(current_down.velocity, [0.0, -0.05, 0.0]);
    assert_eq!(current_down.friction, 0.98);
    assert_eq!(current_down.gravity, 0.002);
    assert!(!current_down.has_physics);
    assert_eq!(
        current_down.tick_motion,
        ParticleTickMotionDescriptor::CurrentDown
    );
    assert_eq!(current_down.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(current_down.required_fluid, Some(ParticleFluidKind::Water));

    let mut bubble_pop_random = ParticleRandom::new(75);
    let mut bubble_pop_command = spawn_command("minecraft:bubble_pop", 1.0);
    bubble_pop_command.velocity = [1.0, 2.0, 3.0];
    let bubble_pop =
        ParticleInstance::from_spawn_command(bubble_pop_command, &mut bubble_pop_random);
    assert_eq!(bubble_pop.provider, "BubblePopParticle.Provider");
    assert_eq!(bubble_pop.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(bubble_pop.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(bubble_pop.base_quad_size, 0.1, 0.2);
    assert_eq!(bubble_pop.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(bubble_pop.lifetime_ticks, 4);
    assert_eq!(bubble_pop.friction, 0.98);
    assert_eq!(bubble_pop.gravity, 0.008);
    assert!(bubble_pop.has_physics);
    assert_eq!(bubble_pop.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(bubble_pop.required_fluid, None);
    assert_eq!(
        bubble_pop.tick_motion,
        ParticleTickMotionDescriptor::DirectGravityNoFriction
    );

    let mut dust_random = ParticleRandom::new(79);
    let mut dust_command = spawn_command("minecraft:dust", 1.0);
    dust_command.velocity = [1.0, 2.0, 3.0];
    dust_command.option_color = Some([0.25, 0.5, 0.75, 1.0]);
    dust_command.option_scale = Some(2.0);
    let dust = ParticleInstance::from_spawn_command(dust_command, &mut dust_random);
    assert_eq!(dust.provider, "DustParticle.Provider");
    assert_eq!(dust.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(
        dust.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_eq!(dust.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(dust.base_quad_size, 0.15, 0.3);
    assert_range_f32(dust.color[0], 0.25 * 0.48, 0.25);
    assert_range_f32(dust.color[1], 0.5 * 0.48, 0.5);
    assert_range_f32(dust.color[2], 0.75 * 0.48, 0.75);
    assert_eq!(dust.color[3], 1.0);
    assert!((16..=80).contains(&dust.lifetime_ticks));
    assert_eq!(dust.option_scale, Some(2.0));
    assert_eq!(dust.render_layer, ParticleRenderLayer::Opaque);
    assert!(dust.speed_up_when_y_motion_is_blocked);

    let mut transition_random = ParticleRandom::new(80);
    let mut transition_command = spawn_command("minecraft:dust_color_transition", 1.0);
    transition_command.option_color = Some([0.0, 0.0, 1.0, 1.0]);
    transition_command.option_color_to = Some([1.0, 0.0, 0.0, 1.0]);
    transition_command.option_scale = Some(1.0);
    let mut transition =
        ParticleInstance::from_spawn_command(transition_command, &mut transition_random);
    assert_eq!(transition.provider, "DustColorTransitionParticle.Provider");
    assert!(transition.color_transition_target.is_some());
    transition.age_ticks = 10;
    transition.lifetime_ticks = 20;
    let target = transition.color_transition_target.unwrap();
    let tint = particle_render_color(&transition);
    let alpha = 10.5 / 21.0;
    assert_close_f32(tint[0], lerp_f32(alpha, transition.color[0], target[0]));
    assert_close_f32(tint[1], lerp_f32(alpha, transition.color[1], target[1]));
    assert_close_f32(tint[2], lerp_f32(alpha, transition.color[2], target[2]));

    let mut firefly_tint = test_instance_with_lifetime("minecraft:firefly", 100);
    firefly_tint.color = [1.0, 1.0, 1.0, 1.0];
    firefly_tint.age_ticks = 90;
    let tint = particle_render_color(&firefly_tint);
    assert_close_f32(tint[3], firefly_fade_amount(90.5 / 100.0, 0.3, 0.5));

    let mut sweep_random = ParticleRandom::new(76);
    let mut sweep_command = spawn_command("minecraft:sweep_attack", 1.0);
    sweep_command.velocity = [0.5, 0.0, 0.0];
    let sweep = ParticleInstance::from_spawn_command(sweep_command, &mut sweep_random);
    assert_eq!(sweep.provider, "AttackSweepParticle.Provider");
    assert_eq!(sweep.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(sweep.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_close_f32(sweep.base_quad_size, 0.75);
    assert_range_f32(sweep.color[0], 0.4, 1.0);
    assert_eq!(sweep.color[0], sweep.color[1]);
    assert_eq!(sweep.color[1], sweep.color[2]);
    assert_eq!(sweep.color[3], 1.0);
    assert_eq!(sweep.lifetime_ticks, 4);
    assert_eq!(sweep.friction, 0.98);
    assert_eq!(sweep.gravity, 0.0);
    assert!(sweep.has_physics);
    assert_ne!(sweep.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(sweep.tick_motion, ParticleTickMotionDescriptor::NoMotion);

    let mut underwater_random = ParticleRandom::new(77);
    let underwater = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:underwater", 1.0),
        &mut underwater_random,
    );
    assert_eq!(underwater.provider, "SuspendedParticle.UnderwaterProvider");
    assert_eq!(underwater.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(underwater.current_sprite_index, Some(0));
    assert_eq!(
        underwater.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_eq!(underwater.previous_position, [1.0, -0.125, 0.0]);
    assert_eq!(underwater.position, [1.0, -0.125, 0.0]);
    assert_eq!(underwater.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(underwater.base_quad_size, 0.02, 0.16);
    assert_eq!(underwater.color, [0.4, 0.4, 0.7, 1.0]);
    assert!((8..=40).contains(&underwater.lifetime_ticks));
    assert_eq!(underwater.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(underwater.friction, 1.0);
    assert_eq!(underwater.gravity, 0.0);
    assert!(!underwater.has_physics);

    let mut spore_random = ParticleRandom::new(78);
    let spore_blossom_air = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:spore_blossom_air", 1.0),
        &mut spore_random,
    );
    assert_eq!(
        spore_blossom_air.provider,
        "SuspendedParticle.SporeBlossomAirProvider"
    );
    assert_eq!(
        spore_blossom_air.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(spore_blossom_air.previous_position, [1.0, -0.125, 0.0]);
    assert_eq!(spore_blossom_air.position, [1.0, -0.125, 0.0]);
    assert_eq!(
        spore_blossom_air.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(spore_blossom_air.base_quad_size, 0.06, 0.24);
    assert_eq!(spore_blossom_air.color, [0.32, 0.5, 0.22, 1.0]);
    assert!((500..=1000).contains(&spore_blossom_air.lifetime_ticks));
    assert_eq!(spore_blossom_air.velocity, [0.0, -0.8, 0.0]);
    assert_eq!(spore_blossom_air.friction, 1.0);
    assert_eq!(spore_blossom_air.gravity, 0.01);
    assert!(!spore_blossom_air.has_physics);
    assert_eq!(
        spore_blossom_air.particle_limit,
        Some(ParticleLimitDescriptor::SporeBlossom)
    );
    assert_eq!(spore_blossom_air.render_layer, ParticleRenderLayer::Opaque);

    let mut nectar_random = ParticleRandom::new(79);
    let falling_nectar = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_nectar", 1.0),
        &mut nectar_random,
    );
    assert_eq!(falling_nectar.provider, "DripParticle.NectarFallProvider");
    assert_eq!(
        falling_nectar.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        falling_nectar.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(falling_nectar.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_nectar.color, [0.92, 0.782, 0.72, 1.0]);
    assert!((16..=80).contains(&falling_nectar.lifetime_ticks));
    assert_eq!(falling_nectar.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_nectar.friction, 0.98);
    assert_eq!(falling_nectar.gravity, 0.007);
    assert!(falling_nectar.has_physics);
    assert_eq!(
        falling_nectar.tick_motion,
        ParticleTickMotionDescriptor::DripFalling
    );
    assert_eq!(falling_nectar.render_layer, ParticleRenderLayer::Opaque);

    let mut falling_spore_random = ParticleRandom::new(80);
    let falling_spore_blossom = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_spore_blossom", 1.0),
        &mut falling_spore_random,
    );
    assert_eq!(
        falling_spore_blossom.provider,
        "DripParticle.SporeBlossomFallProvider"
    );
    assert_eq!(
        falling_spore_blossom.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        falling_spore_blossom.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(falling_spore_blossom.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_spore_blossom.color, [0.32, 0.5, 0.22, 1.0]);
    assert!((71..=640).contains(&falling_spore_blossom.lifetime_ticks));
    assert_eq!(falling_spore_blossom.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_spore_blossom.friction, 0.98);
    assert_eq!(falling_spore_blossom.gravity, 0.005);
    assert!(falling_spore_blossom.has_physics);
    assert_eq!(
        falling_spore_blossom.tick_motion,
        ParticleTickMotionDescriptor::DripFalling
    );
    assert_eq!(
        falling_spore_blossom.render_layer,
        ParticleRenderLayer::Opaque
    );

    let mut dripping_honey_random = ParticleRandom::new(81);
    let dripping_honey = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_honey", 1.0),
        &mut dripping_honey_random,
    );
    assert_eq!(dripping_honey.provider, "DripParticle.HoneyHangProvider");
    assert_eq!(
        dripping_honey.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        dripping_honey.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(dripping_honey.base_quad_size, 0.1, 0.2);
    assert_eq!(dripping_honey.color, [0.622, 0.508, 0.082, 1.0]);
    assert_eq!(dripping_honey.lifetime_ticks, 100);
    assert_eq!(dripping_honey.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_honey.friction, 0.98);
    assert_eq!(dripping_honey.gravity, 0.000_012);
    assert!(dripping_honey.has_physics);
    assert_eq!(
        dripping_honey.tick_motion,
        ParticleTickMotionDescriptor::DripHang
    );
    assert_eq!(dripping_honey.drip_fluid, None);
    assert_eq!(dripping_honey.render_layer, ParticleRenderLayer::Opaque);

    let mut falling_honey_random = ParticleRandom::new(82);
    let falling_honey = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_honey", 1.0),
        &mut falling_honey_random,
    );
    assert_eq!(falling_honey.provider, "DripParticle.HoneyFallProvider");
    assert_range_f32(falling_honey.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_honey.color, [0.582, 0.448, 0.082, 1.0]);
    assert!((64..=320).contains(&falling_honey.lifetime_ticks));
    assert_eq!(falling_honey.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_honey.friction, 0.98);
    assert_eq!(falling_honey.gravity, 0.01);
    assert_eq!(
        falling_honey.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(falling_honey.render_layer, ParticleRenderLayer::Opaque);

    let mut landing_honey_random = ParticleRandom::new(83);
    let landing_honey = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:landing_honey", 1.0),
        &mut landing_honey_random,
    );
    assert_eq!(landing_honey.provider, "DripParticle.HoneyLandProvider");
    assert_range_f32(landing_honey.base_quad_size, 0.1, 0.2);
    assert_eq!(landing_honey.color, [0.522, 0.408, 0.082, 1.0]);
    assert!((128..=640).contains(&landing_honey.lifetime_ticks));
    assert_eq!(landing_honey.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(landing_honey.friction, 0.98);
    assert_eq!(landing_honey.gravity, 0.06);
    assert_eq!(
        landing_honey.tick_motion,
        ParticleTickMotionDescriptor::DripLand
    );
    assert_eq!(landing_honey.render_layer, ParticleRenderLayer::Opaque);

    let mut dripping_obsidian_random = ParticleRandom::new(84);
    let dripping_obsidian = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_obsidian_tear", 1.0),
        &mut dripping_obsidian_random,
    );
    assert_eq!(
        dripping_obsidian.provider,
        "DripParticle.ObsidianTearHangProvider"
    );
    assert_range_f32(dripping_obsidian.base_quad_size, 0.1, 0.2);
    assert_eq!(
        dripping_obsidian.color,
        [0.511_718_75, 0.031_25, 0.890_625, 1.0]
    );
    assert_eq!(dripping_obsidian.lifetime_ticks, 100);
    assert_eq!(dripping_obsidian.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_obsidian.friction, 0.98);
    assert_eq!(dripping_obsidian.gravity, 0.000_012);
    assert_eq!(
        dripping_obsidian.tick_motion,
        ParticleTickMotionDescriptor::DripHang
    );
    assert_eq!(dripping_obsidian.drip_fluid, None);
    assert_eq!(
        dripping_obsidian.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(dripping_obsidian.render_layer, ParticleRenderLayer::Opaque);

    let mut falling_obsidian_random = ParticleRandom::new(85);
    let falling_obsidian = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_obsidian_tear", 1.0),
        &mut falling_obsidian_random,
    );
    assert_eq!(
        falling_obsidian.provider,
        "DripParticle.ObsidianTearFallProvider"
    );
    assert_range_f32(falling_obsidian.base_quad_size, 0.1, 0.2);
    assert_eq!(
        falling_obsidian.color,
        [0.511_718_75, 0.031_25, 0.890_625, 1.0]
    );
    assert!((64..=320).contains(&falling_obsidian.lifetime_ticks));
    assert_eq!(falling_obsidian.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_obsidian.friction, 0.98);
    assert_eq!(falling_obsidian.gravity, 0.01);
    assert_eq!(
        falling_obsidian.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(
        falling_obsidian.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(falling_obsidian.render_layer, ParticleRenderLayer::Opaque);

    let mut landing_obsidian_random = ParticleRandom::new(86);
    let landing_obsidian = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:landing_obsidian_tear", 1.0),
        &mut landing_obsidian_random,
    );
    assert_eq!(
        landing_obsidian.provider,
        "DripParticle.ObsidianTearLandProvider"
    );
    assert_range_f32(landing_obsidian.base_quad_size, 0.1, 0.2);
    assert_eq!(
        landing_obsidian.color,
        [0.511_718_75, 0.031_25, 0.890_625, 1.0]
    );
    assert!((28..=140).contains(&landing_obsidian.lifetime_ticks));
    assert_eq!(landing_obsidian.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(landing_obsidian.friction, 0.98);
    assert_eq!(landing_obsidian.gravity, 0.06);
    assert_eq!(
        landing_obsidian.tick_motion,
        ParticleTickMotionDescriptor::DripLand
    );
    assert_eq!(
        landing_obsidian.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(landing_obsidian.render_layer, ParticleRenderLayer::Opaque);

    let mut dripping_lava_random = ParticleRandom::new(87);
    let dripping_lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_lava", 1.0),
        &mut dripping_lava_random,
    );
    assert_eq!(dripping_lava.provider, "DripParticle.LavaHangProvider");
    assert_range_f32(dripping_lava.base_quad_size, 0.1, 0.2);
    assert_eq!(dripping_lava.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(dripping_lava.lifetime_ticks, 40);
    assert_eq!(dripping_lava.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_lava.friction, 0.98);
    assert_eq!(dripping_lava.gravity, 0.0012);
    assert_eq!(
        dripping_lava.tick_motion,
        ParticleTickMotionDescriptor::CoolingDripHang
    );
    assert_eq!(dripping_lava.drip_fluid, Some(ParticleFluidKind::Lava));
    assert_eq!(
        dripping_lava.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(dripping_lava.render_layer, ParticleRenderLayer::Opaque);

    let mut falling_lava_random = ParticleRandom::new(88);
    let falling_lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_lava", 1.0),
        &mut falling_lava_random,
    );
    assert_eq!(falling_lava.provider, "DripParticle.LavaFallProvider");
    assert_range_f32(falling_lava.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_lava.color, [1.0, 0.285_714_3, 0.083_333_336, 1.0]);
    assert!((64..=320).contains(&falling_lava.lifetime_ticks));
    assert_eq!(falling_lava.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_lava.friction, 0.98);
    assert_eq!(falling_lava.gravity, 0.06);
    assert_eq!(
        falling_lava.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(falling_lava.drip_fluid, Some(ParticleFluidKind::Lava));
    assert_eq!(
        falling_lava.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(falling_lava.render_layer, ParticleRenderLayer::Opaque);

    let mut landing_lava_random = ParticleRandom::new(89);
    let landing_lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:landing_lava", 1.0),
        &mut landing_lava_random,
    );
    assert_eq!(landing_lava.provider, "DripParticle.LavaLandProvider");
    assert_range_f32(landing_lava.base_quad_size, 0.1, 0.2);
    assert_eq!(landing_lava.color, [1.0, 0.285_714_3, 0.083_333_336, 1.0]);
    assert!((16..=80).contains(&landing_lava.lifetime_ticks));
    assert_eq!(landing_lava.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(landing_lava.friction, 0.98);
    assert_eq!(landing_lava.gravity, 0.06);
    assert_eq!(
        landing_lava.tick_motion,
        ParticleTickMotionDescriptor::DripLand
    );
    assert_eq!(landing_lava.drip_fluid, Some(ParticleFluidKind::Lava));
    assert_eq!(
        landing_lava.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(landing_lava.render_layer, ParticleRenderLayer::Opaque);

    let mut dripping_water_random = ParticleRandom::new(90);
    let dripping_water = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_water", 1.0),
        &mut dripping_water_random,
    );
    assert_eq!(dripping_water.provider, "DripParticle.WaterHangProvider");
    assert_range_f32(dripping_water.base_quad_size, 0.1, 0.2);
    assert_eq!(dripping_water.color, [0.2, 0.3, 1.0, 1.0]);
    assert_eq!(dripping_water.lifetime_ticks, 40);
    assert_eq!(dripping_water.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_water.friction, 0.98);
    assert_eq!(dripping_water.gravity, 0.0012);
    assert_eq!(
        dripping_water.tick_motion,
        ParticleTickMotionDescriptor::DripHang
    );
    assert_eq!(dripping_water.drip_fluid, Some(ParticleFluidKind::Water));
    assert_eq!(
        dripping_water.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(dripping_water.render_layer, ParticleRenderLayer::Opaque);

    let mut falling_water_random = ParticleRandom::new(91);
    let falling_water = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_water", 1.0),
        &mut falling_water_random,
    );
    assert_eq!(falling_water.provider, "DripParticle.WaterFallProvider");
    assert_range_f32(falling_water.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_water.color, [0.2, 0.3, 1.0, 1.0]);
    assert!((64..=320).contains(&falling_water.lifetime_ticks));
    assert_eq!(falling_water.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_water.friction, 0.98);
    assert_eq!(falling_water.gravity, 0.06);
    assert_eq!(
        falling_water.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(falling_water.drip_fluid, Some(ParticleFluidKind::Water));
    assert_eq!(
        falling_water.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(falling_water.render_layer, ParticleRenderLayer::Opaque);

    let mut dripping_dripstone_lava_random = ParticleRandom::new(92);
    let dripping_dripstone_lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_dripstone_lava", 1.0),
        &mut dripping_dripstone_lava_random,
    );
    assert_eq!(
        dripping_dripstone_lava.provider,
        "DripParticle.DripstoneLavaHangProvider"
    );
    assert_range_f32(dripping_dripstone_lava.base_quad_size, 0.1, 0.2);
    assert_eq!(dripping_dripstone_lava.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(dripping_dripstone_lava.lifetime_ticks, 40);
    assert_eq!(dripping_dripstone_lava.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_dripstone_lava.friction, 0.98);
    assert_eq!(dripping_dripstone_lava.gravity, 0.0012);
    assert_eq!(
        dripping_dripstone_lava.tick_motion,
        ParticleTickMotionDescriptor::CoolingDripHang
    );
    assert_eq!(
        dripping_dripstone_lava.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(
        dripping_dripstone_lava.render_layer,
        ParticleRenderLayer::Opaque
    );

    let mut falling_dripstone_lava_random = ParticleRandom::new(93);
    let falling_dripstone_lava = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_dripstone_lava", 1.0),
        &mut falling_dripstone_lava_random,
    );
    assert_eq!(
        falling_dripstone_lava.provider,
        "DripParticle.DripstoneLavaFallProvider"
    );
    assert_range_f32(falling_dripstone_lava.base_quad_size, 0.1, 0.2);
    assert_eq!(
        falling_dripstone_lava.color,
        [1.0, 0.285_714_3, 0.083_333_336, 1.0]
    );
    assert!((64..=320).contains(&falling_dripstone_lava.lifetime_ticks));
    assert_eq!(falling_dripstone_lava.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_dripstone_lava.friction, 0.98);
    assert_eq!(falling_dripstone_lava.gravity, 0.06);
    assert_eq!(
        falling_dripstone_lava.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(
        falling_dripstone_lava.drip_fluid,
        Some(ParticleFluidKind::Lava)
    );
    assert_eq!(
        falling_dripstone_lava.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(
        falling_dripstone_lava.render_layer,
        ParticleRenderLayer::Opaque
    );

    let mut dripping_dripstone_water_random = ParticleRandom::new(94);
    let dripping_dripstone_water = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dripping_dripstone_water", 1.0),
        &mut dripping_dripstone_water_random,
    );
    assert_eq!(
        dripping_dripstone_water.provider,
        "DripParticle.DripstoneWaterHangProvider"
    );
    assert_range_f32(dripping_dripstone_water.base_quad_size, 0.1, 0.2);
    assert_eq!(dripping_dripstone_water.color, [0.2, 0.3, 1.0, 1.0]);
    assert_eq!(dripping_dripstone_water.lifetime_ticks, 40);
    assert_eq!(dripping_dripstone_water.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(dripping_dripstone_water.friction, 0.98);
    assert_eq!(dripping_dripstone_water.gravity, 0.0012);
    assert_eq!(
        dripping_dripstone_water.tick_motion,
        ParticleTickMotionDescriptor::DripHang
    );
    assert_eq!(
        dripping_dripstone_water.drip_fluid,
        Some(ParticleFluidKind::Water)
    );
    assert_eq!(
        dripping_dripstone_water.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(
        dripping_dripstone_water.render_layer,
        ParticleRenderLayer::Opaque
    );

    let mut falling_dripstone_water_random = ParticleRandom::new(95);
    let falling_dripstone_water = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_dripstone_water", 1.0),
        &mut falling_dripstone_water_random,
    );
    assert_eq!(
        falling_dripstone_water.provider,
        "DripParticle.DripstoneWaterFallProvider"
    );
    assert_range_f32(falling_dripstone_water.base_quad_size, 0.1, 0.2);
    assert_eq!(falling_dripstone_water.color, [0.2, 0.3, 1.0, 1.0]);
    assert!((64..=320).contains(&falling_dripstone_water.lifetime_ticks));
    assert_eq!(falling_dripstone_water.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(falling_dripstone_water.friction, 0.98);
    assert_eq!(falling_dripstone_water.gravity, 0.06);
    assert_eq!(
        falling_dripstone_water.tick_motion,
        ParticleTickMotionDescriptor::DripFallAndLand
    );
    assert_eq!(
        falling_dripstone_water.light_emission,
        ParticleLightEmissionDescriptor::World
    );
    assert_eq!(
        falling_dripstone_water.render_layer,
        ParticleRenderLayer::Opaque
    );

    let mut crimson_random = ParticleRandom::new(46);
    let crimson_spore = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:crimson_spore", 1.0),
        &mut crimson_random,
    );
    assert_eq!(
        crimson_spore.provider,
        "SuspendedParticle.CrimsonSporeProvider"
    );
    assert_eq!(
        crimson_spore.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(crimson_spore.previous_position, [1.0, -0.125, 0.0]);
    assert_eq!(crimson_spore.position, [1.0, -0.125, 0.0]);
    assert_range_f32(crimson_spore.base_quad_size, 0.06, 0.24);
    assert_eq!(crimson_spore.color, [0.9, 0.4, 0.5, 1.0]);
    assert!((16..=80).contains(&crimson_spore.lifetime_ticks));
    assert_close_f64(crimson_spore.velocity[0], 1.3558214650566454E-6);
    assert_close_f64(crimson_spore.velocity[1], -0.8270729973920494E-4);
    assert_close_f64(crimson_spore.velocity[2], 1.6065611415614136E-6);
    assert_eq!(crimson_spore.friction, 1.0);
    assert_eq!(crimson_spore.gravity, 0.0);
    assert!(!crimson_spore.has_physics);
    assert_eq!(crimson_spore.render_layer, ParticleRenderLayer::Opaque);

    let mut warped_random = ParticleRandom::new(47);
    let warped_spore = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:warped_spore", 1.0),
        &mut warped_random,
    );
    assert_eq!(
        warped_spore.provider,
        "SuspendedParticle.WarpedSporeProvider"
    );
    assert_eq!(warped_spore.previous_position, [1.0, -0.125, 0.0]);
    assert_eq!(warped_spore.position, [1.0, -0.125, 0.0]);
    assert_range_f32(warped_spore.base_quad_size, 0.06, 0.24);
    assert_eq!(warped_spore.color, [0.1, 0.1, 0.3, 1.0]);
    assert!((16..=80).contains(&warped_spore.lifetime_ticks));
    assert_close_f64(warped_spore.velocity[0], 0.0);
    assert_close_f64(warped_spore.velocity[1], -0.055236806630186874);
    assert_close_f64(warped_spore.velocity[2], 0.0);
    assert_eq!(warped_spore.friction, 1.0);
    assert_eq!(warped_spore.gravity, 0.0);
    assert!(!warped_spore.has_physics);
    assert_eq!(warped_spore.render_layer, ParticleRenderLayer::Opaque);

    let mut glow_random = ParticleRandom::new(67);
    let mut glow_command = spawn_command("minecraft:glow", 1.0);
    glow_command.velocity = [0.0, 1.0, 0.0];
    let glow = ParticleInstance::from_spawn_command(glow_command, &mut glow_random);
    assert_eq!(glow.provider, "GlowParticle.GlowSquidProvider");
    assert_eq!(glow.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(glow.base_quad_size, 0.075, 0.15);
    assert!(glow.color == [0.6, 1.0, 0.8, 1.0] || glow.color == [0.08, 0.4, 0.4, 1.0]);
    assert!((8..=40).contains(&glow.lifetime_ticks));
    assert_eq!(glow.friction, 0.96);
    assert!(!glow.has_physics);
    assert!(glow.speed_up_when_y_motion_is_blocked);
    assert_range_f64(glow.velocity[0].abs(), 0.0, 0.02);
    assert_range_f64(glow.velocity[1], 0.015, 0.08);
    assert_range_f64(glow.velocity[2].abs(), 0.0, 0.02);

    let mut electric_random = ParticleRandom::new(63);
    let mut electric_command = spawn_command("minecraft:electric_spark", 1.0);
    electric_command.velocity = [2.0, 3.0, 4.0];
    let electric = ParticleInstance::from_spawn_command(electric_command, &mut electric_random);
    assert_eq!(electric.provider, "GlowParticle.ElectricSparkProvider");
    assert_eq!(electric.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(electric.base_quad_size, 0.075, 0.15);
    assert_eq!(electric.color, [1.0, 0.9, 1.0, 1.0]);
    assert!((2..=3).contains(&electric.lifetime_ticks));
    assert_eq!(electric.friction, 0.96);
    assert!(!electric.has_physics);
    assert!(electric.speed_up_when_y_motion_is_blocked);
    assert_range_f64(electric.velocity[0], 0.499, 0.501);
    assert_range_f64(electric.velocity[1], 0.749, 0.751);
    assert_range_f64(electric.velocity[2], 0.999, 1.001);

    let mut scrape_random = ParticleRandom::new(64);
    let mut scrape_command = spawn_command("minecraft:scrape", 1.0);
    scrape_command.velocity = [2.0, 3.0, 4.0];
    let scrape = ParticleInstance::from_spawn_command(scrape_command, &mut scrape_random);
    assert_eq!(scrape.provider, "GlowParticle.ScrapeProvider");
    assert!(scrape.color == [0.29, 0.58, 0.51, 1.0] || scrape.color == [0.43, 0.77, 0.62, 1.0]);
    assert!((10..=39).contains(&scrape.lifetime_ticks));
    assert_range_f64(scrape.velocity[0], 0.019, 0.021);
    assert_range_f64(scrape.velocity[1], 0.029, 0.031);
    assert_range_f64(scrape.velocity[2], 0.039, 0.041);

    let mut wax_on_random = ParticleRandom::new(65);
    let mut wax_on_command = spawn_command("minecraft:wax_on", 1.0);
    wax_on_command.velocity = [2.0, 3.0, 4.0];
    let wax_on = ParticleInstance::from_spawn_command(wax_on_command, &mut wax_on_random);
    assert_eq!(wax_on.provider, "GlowParticle.WaxOnProvider");
    assert_eq!(wax_on.color, [0.91, 0.55, 0.08, 1.0]);
    assert!((10..=39).contains(&wax_on.lifetime_ticks));
    assert_range_f64(wax_on.velocity[0], 0.009, 0.011);
    assert_range_f64(wax_on.velocity[1], 0.029, 0.031);
    assert_range_f64(wax_on.velocity[2], 0.019, 0.021);

    let mut wax_off_random = ParticleRandom::new(66);
    let mut wax_off_command = spawn_command("minecraft:wax_off", 1.0);
    wax_off_command.velocity = [2.0, 3.0, 4.0];
    let wax_off = ParticleInstance::from_spawn_command(wax_off_command, &mut wax_off_random);
    assert_eq!(wax_off.provider, "GlowParticle.WaxOffProvider");
    assert_eq!(wax_off.color, [1.0, 0.9, 1.0, 1.0]);
    assert!((10..=39).contains(&wax_off.lifetime_ticks));
    assert_range_f64(wax_off.velocity[0], 0.009, 0.011);
    assert_range_f64(wax_off.velocity[1], 0.029, 0.031);
    assert_range_f64(wax_off.velocity[2], 0.019, 0.021);

    let mut sneeze_random = ParticleRandom::new(55);
    let sneeze = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:sneeze", 1.0),
        &mut sneeze_random,
    );
    assert_eq!(sneeze.provider, "PlayerCloudParticle.SneezeProvider");
    assert_eq!(sneeze.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(sneeze.base_quad_size, 0.1875, 0.375);
    assert_eq!(sneeze.color, [0.22, 1.0, 0.53, 0.4]);
    assert_eq!(sneeze.friction, 0.96);
    assert!(!sneeze.has_physics);
    assert_ne!(sneeze.velocity, [0.0, 0.0, 0.0]);

    let mut snowflake_random = ParticleRandom::new(56);
    let mut snowflake_command = spawn_command("minecraft:snowflake", 1.0);
    snowflake_command.velocity = [1.0, 2.0, 3.0];
    let snowflake = ParticleInstance::from_spawn_command(snowflake_command, &mut snowflake_random);
    assert_eq!(snowflake.provider, "SnowflakeParticle.Provider");
    assert_eq!(snowflake.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(snowflake.base_quad_size, 0.1, 0.2);
    assert_eq!(snowflake.color, [0.923, 0.964, 0.999, 1.0]);
    assert_eq!(snowflake.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!((18..=82).contains(&snowflake.lifetime_ticks));
    assert_range_f64(snowflake.velocity[0], 0.95, 1.05);
    assert_range_f64(snowflake.velocity[1], 1.95, 2.05);
    assert_range_f64(snowflake.velocity[2], 2.95, 3.05);
    assert_eq!(snowflake.friction, 1.0);
    assert_eq!(snowflake.gravity, 0.225);
    assert!(snowflake.has_physics);
    assert_eq!(snowflake.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(
        snowflake.tick_motion,
        ParticleTickMotionDescriptor::Snowflake
    );

    let mut squid_ink_random = ParticleRandom::new(57);
    let mut squid_ink_command = spawn_command("minecraft:squid_ink", 1.0);
    squid_ink_command.velocity = [1.0, 2.0, 3.0];
    let squid_ink = ParticleInstance::from_spawn_command(squid_ink_command, &mut squid_ink_random);
    assert_eq!(squid_ink.provider, "SquidInkParticle.Provider");
    assert_eq!(squid_ink.sprite_selection, ParticleSpriteSelection::Age);
    assert_close_f32(squid_ink.base_quad_size, 0.5);
    assert_eq!(squid_ink.color, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(squid_ink.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!((6..=30).contains(&squid_ink.lifetime_ticks));
    assert_eq!(squid_ink.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(squid_ink.friction, 0.92);
    assert_eq!(squid_ink.gravity, 0.0);
    assert!(!squid_ink.has_physics);
    assert_close_f64(
        squid_ink.air_downward_acceleration,
        descriptors::SQUID_INK_AIR_DOWNWARD_ACCELERATION,
    );
    assert_eq!(squid_ink.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(
        squid_ink.alpha_curve,
        ParticleAlphaCurve::SimpleAnimatedFade
    );

    let mut glow_ink_random = ParticleRandom::new(58);
    let glow_ink = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:glow_squid_ink", 1.0),
        &mut glow_ink_random,
    );
    assert_eq!(glow_ink.provider, "SquidInkParticle.GlowInkProvider");
    assert_close_f32(glow_ink.base_quad_size, 0.5);
    assert_eq!(glow_ink.color, [0.2, 0.8, 0.6, 1.0]);
    assert!((6..=30).contains(&glow_ink.lifetime_ticks));
    assert!(!glow_ink.has_physics);
    assert_close_f64(
        glow_ink.air_downward_acceleration,
        descriptors::SQUID_INK_AIR_DOWNWARD_ACCELERATION,
    );
    assert_eq!(glow_ink.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(glow_ink.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);

    let mut note_random = ParticleRandom::new(54);
    let mut note_command = spawn_command("minecraft:note", 1.0);
    note_command.velocity = [0.0, 0.0, 0.0];
    let note = ParticleInstance::from_spawn_command(note_command, &mut note_random);
    assert_eq!(note.provider, "NoteParticle.Provider");
    assert_eq!(note.lifetime_ticks, 6);
    assert_eq!(note.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(note.base_quad_size, 0.15, 0.3);
    assert_close_f32(note.color[0], 0.35);
    assert_close_f32(note.color[1], 0.912_916_5);
    assert_close_f32(note.color[2], 0.0);
    assert_eq!(note.color[3], 1.0);
    assert_eq!(note.friction, 0.66);
    assert!(note.has_physics);
    assert!(note.speed_up_when_y_motion_is_blocked);
    assert_range_f64(note.velocity[1], 0.198, 0.202);

    let mut flash_random = ParticleRandom::new(66);
    let mut flash_command = spawn_command("minecraft:flash", 1.0);
    flash_command.option_color = Some([0.1, 0.2, 0.3, 0.4]);
    let flash = ParticleInstance::from_spawn_command(flash_command, &mut flash_random);
    assert_eq!(flash.provider, "FireworkParticles.FlashProvider");
    assert_eq!(flash.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(flash.lifetime_ticks, 4);
    assert_eq!(flash.color, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(flash.quad_size_curve, ParticleQuadSizeCurve::FlashOverlay);
    assert_eq!(flash.alpha_curve, ParticleAlphaCurve::FlashOverlayFade);
    assert_eq!(flash.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(flash.render_layer, ParticleRenderLayer::Translucent);

    let mut trail_random = ParticleRandom::new(67);
    let mut trail_command = spawn_command("minecraft:trail", 1.0);
    trail_command.velocity = [0.1, 0.2, 0.3];
    trail_command.option_color = Some([0.2, 0.4, 0.8, 1.0]);
    trail_command.option_target = Some([4.0, 6.0, 8.0]);
    trail_command.option_duration_ticks = Some(12);
    let mut expected_trail_random = ParticleRandom::new(67);
    let _ = select_initial_sprite(
        &trail_command.sprite_ids,
        ParticleSpriteSelection::Random,
        &mut expected_trail_random,
    );
    let _ = expected_trail_random.next_f32();
    let expected_trail_color = [
        0.2 * (0.875 + expected_trail_random.next_f32() * 0.25),
        0.4 * (0.875 + expected_trail_random.next_f32() * 0.25),
        0.8 * (0.875 + expected_trail_random.next_f32() * 0.25),
        1.0,
    ];
    let trail = ParticleInstance::from_spawn_command(trail_command, &mut trail_random);
    assert_eq!(trail.provider, "TrailParticle.Provider");
    assert_eq!(
        trail.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_eq!(trail.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(trail.lifetime_ticks, 12);
    assert_close_f32(trail.base_quad_size, 0.26);
    assert_close3_f32(
        [trail.color[0], trail.color[1], trail.color[2]],
        [
            expected_trail_color[0],
            expected_trail_color[1],
            expected_trail_color[2],
        ],
    );
    assert_eq!(trail.color[3], expected_trail_color[3]);
    assert_eq!(trail.option_target, Some([4.0, 6.0, 8.0]));
    assert_eq!(trail.option_duration_ticks, Some(12));
    assert_eq!(trail.velocity, [0.1, 0.2, 0.3]);
    assert_eq!(trail.tick_motion, ParticleTickMotionDescriptor::TrailTarget);
    assert_eq!(
        trail.light_emission,
        ParticleLightEmissionDescriptor::FullBright
    );
    assert_eq!(particle_light_with_emission(&trail, [0.2, 0.3]), [1.0, 1.0]);
    assert_eq!(trail.render_layer, ParticleRenderLayer::Opaque);

    let mut vibration_random = ParticleRandom::new(68);
    let mut vibration_command = spawn_command("minecraft:vibration", 1.0);
    vibration_command.position = [1.0, 2.0, 3.0];
    vibration_command.velocity = [9.0, 9.0, 9.0];
    vibration_command.option_target = Some([4.0, 6.0, 8.0]);
    vibration_command.option_duration_ticks = Some(20);
    let vibration = ParticleInstance::from_spawn_command(vibration_command, &mut vibration_random);
    let (yaw, pitch) = vibration_particle_angles([1.0, 2.0, 3.0], [4.0, 6.0, 8.0]);
    assert_eq!(vibration.provider, "VibrationSignalParticle.Provider");
    assert_eq!(
        vibration.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_eq!(vibration.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(vibration.lifetime_ticks, 20);
    assert_close_f32(vibration.base_quad_size, 0.3);
    assert_eq!(vibration.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(vibration.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        vibration.tick_motion,
        ParticleTickMotionDescriptor::VibrationSignal
    );
    assert_eq!(vibration.option_target, Some([4.0, 6.0, 8.0]));
    assert_eq!(vibration.option_duration_ticks, Some(20));
    assert_close_f32(vibration.previous_yaw, yaw);
    assert_close_f32(vibration.yaw, yaw);
    assert_close_f32(vibration.previous_pitch, pitch);
    assert_close_f32(vibration.pitch, pitch);
    assert_eq!(
        vibration.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(
        particle_light_with_emission(&vibration, [0.2, 0.3]),
        [1.0, 0.3]
    );
    assert_eq!(vibration.render_layer, ParticleRenderLayer::Translucent);

    let mut spell_random = ParticleRandom::new(61);
    let mut spell_command = spawn_command("minecraft:infested", 1.0);
    spell_command.velocity = [0.0, 1.0, 0.0];
    let spell = ParticleInstance::from_spawn_command(spell_command, &mut spell_random);
    assert_eq!(spell.provider, "SpellParticle.Provider");
    assert_eq!(spell.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(spell.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(spell.base_quad_size, 0.075, 0.15);
    assert_eq!(spell.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((8..=40).contains(&spell.lifetime_ticks));
    assert_eq!(spell.friction, 0.96);
    assert_eq!(spell.gravity, -0.1);
    assert!(!spell.has_physics);
    assert!(spell.speed_up_when_y_motion_is_blocked);
    assert_range_f64(spell.velocity[0].abs(), 0.0, 0.008);
    assert_range_f64(spell.velocity[1], 0.0, 0.06);
    assert_range_f64(spell.velocity[2].abs(), 0.0, 0.008);
    assert_eq!(spell.original_alpha, 1.0);

    let mut scoped_spell_random = ParticleRandom::new(61);
    let scoped_spell = ParticleInstance::from_spawn_command_with_scope_context(
        spawn_command("minecraft:infested", 1.0),
        &mut scoped_spell_random,
        Some(ParticleLocalPlayerScopeContext {
            eye_position: [0.0, 0.0, 0.0],
            first_person: true,
            scoping: true,
        }),
    );
    assert_eq!(scoped_spell.color[3], 0.0);
    assert_eq!(scoped_spell.original_alpha, 1.0);

    let mut base_effect_random = ParticleRandom::new(63);
    let mut base_effect_command = spawn_command("minecraft:effect", 1.0);
    base_effect_command.velocity = [1.0, 1.0, 0.0];
    let base_effect =
        ParticleInstance::from_spawn_command(base_effect_command, &mut base_effect_random);
    let mut powered_effect_random = ParticleRandom::new(63);
    let mut powered_effect_command = spawn_command("minecraft:effect", 1.0);
    powered_effect_command.velocity = [1.0, 1.0, 0.0];
    powered_effect_command.option_color = Some([0.2, 0.4, 0.6, 1.0]);
    powered_effect_command.option_power = Some(0.5);
    let powered_effect =
        ParticleInstance::from_spawn_command(powered_effect_command, &mut powered_effect_random);
    assert_eq!(powered_effect.provider, "SpellParticle.InstantProvider");
    assert_eq!(
        powered_effect.render_layer,
        ParticleRenderLayer::Translucent
    );
    assert_eq!(powered_effect.color, [0.2, 0.4, 0.6, 1.0]);
    assert_eq!(powered_effect.option_power, Some(0.5));
    assert_close_f64(powered_effect.velocity[0], base_effect.velocity[0] * 0.5);
    assert_close_f64(
        powered_effect.velocity[1],
        (base_effect.velocity[1] - 0.1) * 0.5 + 0.1,
    );
    assert_close_f64(powered_effect.velocity[2], base_effect.velocity[2] * 0.5);

    let mut entity_effect_random = ParticleRandom::new(64);
    let mut entity_effect_command = spawn_command("minecraft:entity_effect", 1.0);
    entity_effect_command.option_color = Some([0.1, 0.2, 0.3, 0.4]);
    let entity_effect =
        ParticleInstance::from_spawn_command(entity_effect_command, &mut entity_effect_random);
    assert_eq!(entity_effect.provider, "SpellParticle.MobEffectProvider");
    assert_eq!(entity_effect.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(entity_effect.color, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(entity_effect.original_alpha, 0.4);
    assert_eq!(entity_effect.option_power, None);

    let mut scoped_entity_effect_random = ParticleRandom::new(64);
    let mut scoped_entity_effect_command = spawn_command("minecraft:entity_effect", 1.0);
    scoped_entity_effect_command.option_color = Some([0.1, 0.2, 0.3, 0.4]);
    let scoped_entity_effect = ParticleInstance::from_spawn_command_with_scope_context(
        scoped_entity_effect_command,
        &mut scoped_entity_effect_random,
        Some(ParticleLocalPlayerScopeContext {
            eye_position: [0.0, 0.0, 0.0],
            first_person: true,
            scoping: true,
        }),
    );
    assert_eq!(
        scoped_entity_effect.provider,
        "SpellParticle.MobEffectProvider"
    );
    assert_eq!(scoped_entity_effect.color, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(scoped_entity_effect.original_alpha, 0.4);

    let mut instant_effect_random = ParticleRandom::new(65);
    let mut instant_effect_command = spawn_command("minecraft:instant_effect", 1.0);
    instant_effect_command.option_color = Some([0.9, 0.8, 0.7, 1.0]);
    instant_effect_command.option_power = Some(1.25);
    let instant_effect =
        ParticleInstance::from_spawn_command(instant_effect_command, &mut instant_effect_random);
    assert_eq!(instant_effect.provider, "SpellParticle.InstantProvider");
    assert_eq!(instant_effect.color, [0.9, 0.8, 0.7, 1.0]);

    let mut pause_random = ParticleRandom::new(59);
    let mut pause_command = spawn_command("minecraft:pause_mob_growth", 1.0);
    pause_command.velocity = [1.0, 2.0, 3.0];
    let pause_growth = ParticleInstance::from_spawn_command(pause_command, &mut pause_random);
    assert_eq!(
        pause_growth.provider,
        "SimpleVerticalParticle.PauseMobGrowthProvider"
    );
    assert_eq!(
        pause_growth.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_range_f32(pause_growth.base_quad_size, 0.05, 0.22);
    assert_eq!(pause_growth.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        pause_growth.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_eq!(pause_growth.lifetime_ticks, 8);
    assert_eq!(pause_growth.velocity, [1.0, 1.97, 3.0]);
    assert_eq!(pause_growth.friction, 0.98);
    assert_eq!(pause_growth.gravity, 0.0);
    assert!(pause_growth.has_physics);
    assert_eq!(pause_growth.render_layer, ParticleRenderLayer::Opaque);

    let mut reset_random = ParticleRandom::new(60);
    let mut reset_command = spawn_command("minecraft:reset_mob_growth", 1.0);
    reset_command.velocity = [1.0, 2.0, 3.0];
    let reset_growth = ParticleInstance::from_spawn_command(reset_command, &mut reset_random);
    assert_eq!(
        reset_growth.provider,
        "SimpleVerticalParticle.ResetMobGrowthProvider"
    );
    assert_eq!(reset_growth.lifetime_ticks, 8);
    assert_eq!(reset_growth.velocity, [1.0, 2.03, 3.0]);
    assert!(reset_growth.has_physics);
    assert_eq!(reset_growth.render_layer, ParticleRenderLayer::Opaque);

    let mut witch_random = ParticleRandom::new(62);
    let witch = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:witch", 1.0),
        &mut witch_random,
    );
    assert_eq!(witch.provider, "SpellParticle.WitchProvider");
    assert_eq!(witch.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(witch.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(witch.base_quad_size, 0.075, 0.15);
    assert_range_f32(witch.color[0], 0.35, 0.85);
    assert_eq!(witch.color[1], 0.0);
    assert_eq!(witch.color[2], witch.color[0]);
    assert_eq!(witch.color[3], 1.0);
    assert!((8..=40).contains(&witch.lifetime_ticks));
    assert_eq!(witch.friction, 0.96);
    assert_eq!(witch.gravity, -0.1);
    assert!(!witch.has_physics);
    assert!(witch.speed_up_when_y_motion_is_blocked);

    let mut crit_random = ParticleRandom::new(56);
    let mut crit_command = spawn_command("minecraft:crit", 1.0);
    crit_command.velocity = [0.5, 0.25, -0.5];
    let crit = ParticleInstance::from_spawn_command(crit_command, &mut crit_random);
    assert_eq!(crit.provider, "CritParticle.Provider");
    assert_eq!(crit.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(crit.base_quad_size, 0.075, 0.15);
    assert_range_f32(crit.color[0], 0.6, 0.9);
    assert_close_f32(crit.color[1], crit.color[0] * 0.96);
    assert_close_f32(crit.color[2], crit.color[0] * 0.9);
    assert_eq!(crit.color[3], 1.0);
    assert!((4..=10).contains(&crit.lifetime_ticks));
    assert_eq!(crit.friction, 0.7);
    assert_eq!(crit.gravity, 0.5);
    assert!(!crit.has_physics);
    assert_eq!(crit.age_ticks, 1);
    assert_range_f64(crit.position[0], 1.19, 1.21);
    assert_range_f64(crit.position[1], 0.08, 0.10);
    assert_range_f64(crit.position[2], -0.21, -0.19);
    assert_range_f64(crit.velocity[0], 0.133, 0.147);
    assert_range_f64(crit.velocity[1], 0.056, 0.070);
    assert_range_f64(crit.velocity[2], -0.147, -0.133);

    let mut damage_random = ParticleRandom::new(57);
    let mut damage_command = spawn_command("minecraft:damage_indicator", 1.0);
    damage_command.velocity = [0.0, 0.0, 0.0];
    let damage = ParticleInstance::from_spawn_command(damage_command, &mut damage_random);
    assert_eq!(damage.provider, "CritParticle.DamageIndicatorProvider");
    assert_eq!(damage.lifetime_ticks, 20);
    assert_eq!(damage.age_ticks, 1);
    assert_range_f64(damage.velocity[1], 0.266, 0.287);

    let mut magic_random = ParticleRandom::new(58);
    let magic = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:enchanted_hit", 1.0),
        &mut magic_random,
    );
    assert_eq!(magic.provider, "CritParticle.MagicProvider");
    assert_eq!(magic.age_ticks, 1);
    assert_range_f32(magic.color[0], 0.18, 0.27);
    assert!(magic.color[1] > magic.color[0]);
    assert!(magic.color[2] > magic.color[1]);
    assert!((4..=10).contains(&magic.lifetime_ticks));

    let mut enchant_random = ParticleRandom::new(83);
    let mut enchant_command = spawn_command("minecraft:enchant", 1.0);
    enchant_command.position = [1.0, 2.0, 3.0];
    enchant_command.velocity = [0.5, 1.0, -0.25];
    let enchant = ParticleInstance::from_spawn_command(enchant_command, &mut enchant_random);
    assert_eq!(
        enchant.provider,
        "FlyTowardsPositionParticle.EnchantProvider"
    );
    assert_eq!(enchant.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(enchant.start_position, [1.0, 2.0, 3.0]);
    assert_eq!(enchant.previous_position, [1.5, 3.0, 2.75]);
    assert_eq!(enchant.position, [1.5, 3.0, 2.75]);
    assert_eq!(enchant.velocity, [0.5, 1.0, -0.25]);
    assert_range_f32(enchant.base_quad_size, 0.02, 0.07);
    assert_close_f32(enchant.color[0], enchant.color[2] * 0.9);
    assert_close_f32(enchant.color[1], enchant.color[2] * 0.9);
    assert_eq!(enchant.color[3], 1.0);
    assert!((30..=39).contains(&enchant.lifetime_ticks));
    assert!(!enchant.has_physics);
    assert_eq!(
        enchant.tick_motion,
        ParticleTickMotionDescriptor::FlyTowardsPosition
    );
    assert_eq!(
        enchant.light_emission,
        ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
    );

    let mut nautilus_random = ParticleRandom::new(84);
    let mut nautilus_command = spawn_command("minecraft:nautilus", 1.0);
    nautilus_command.position = [1.0, 2.0, 3.0];
    nautilus_command.velocity = [-0.25, 0.5, 1.25];
    let nautilus = ParticleInstance::from_spawn_command(nautilus_command, &mut nautilus_random);
    assert_eq!(
        nautilus.provider,
        "FlyTowardsPositionParticle.NautilusProvider"
    );
    assert_eq!(nautilus.start_position, [1.0, 2.0, 3.0]);
    assert_eq!(nautilus.previous_position, [0.75, 2.5, 4.25]);
    assert_eq!(nautilus.position, [0.75, 2.5, 4.25]);
    assert_range_f32(nautilus.base_quad_size, 0.02, 0.07);
    assert!((30..=39).contains(&nautilus.lifetime_ticks));

    let mut vault_random = ParticleRandom::new(86);
    let mut vault_command = spawn_command("minecraft:vault_connection", 1.0);
    vault_command.position = [1.0, 2.0, 3.0];
    vault_command.velocity = [0.25, -0.5, 0.75];
    let vault = ParticleInstance::from_spawn_command(vault_command, &mut vault_random);
    assert_eq!(
        vault.provider,
        "FlyTowardsPositionParticle.VaultConnectionProvider"
    );
    assert_eq!(vault.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(vault.start_position, [1.0, 2.0, 3.0]);
    assert_eq!(vault.previous_position, [1.25, 1.5, 3.75]);
    assert_eq!(vault.position, [1.25, 1.5, 3.75]);
    assert_eq!(vault.velocity, [0.25, -0.5, 0.75]);
    assert_range_f32(vault.base_quad_size, 0.03, 0.105);
    assert_close_f32(vault.color[0], vault.color[2] * 0.9);
    assert_close_f32(vault.color[1], vault.color[2] * 0.9);
    assert_eq!(vault.color[3], 0.0);
    assert!((30..=39).contains(&vault.lifetime_ticks));
    assert!(!vault.has_physics);
    assert_eq!(
        vault.tick_motion,
        ParticleTickMotionDescriptor::FlyTowardsPosition
    );
    assert_eq!(vault.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(vault.alpha_curve, ParticleAlphaCurve::VaultConnectionFade);
    assert_eq!(
        vault.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );

    let mut totem_random = ParticleRandom::new(85);
    let mut totem_command = spawn_command("minecraft:totem_of_undying", 1.0);
    totem_command.velocity = [0.25, 0.5, -0.75];
    let totem = ParticleInstance::from_spawn_command(totem_command, &mut totem_random);
    assert_eq!(totem.provider, "TotemParticle.Provider");
    assert_eq!(totem.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(totem.current_sprite_index, Some(0));
    assert_range_f32(totem.base_quad_size, 0.075, 0.15);
    assert!((0.1..=0.3).contains(&totem.color[0]) || (0.6..=0.8).contains(&totem.color[0]));
    assert_range_f32(totem.color[1], 0.4, 0.9);
    assert_range_f32(totem.color[2], 0.0, 0.2);
    assert_eq!(totem.color[3], 1.0);
    assert!((60..=71).contains(&totem.lifetime_ticks));
    assert_eq!(totem.velocity, [0.25, 0.5, -0.75]);
    assert_eq!(totem.friction, 0.6);
    assert_eq!(totem.gravity, 1.25);
    assert!(totem.has_physics);
    assert_eq!(totem.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(
        totem.light_emission,
        ParticleLightEmissionDescriptor::FullBright
    );
    assert_eq!(totem.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);

    let mut angry_villager_random = ParticleRandom::new(52);
    let angry_villager = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:angry_villager", 1.0),
        &mut angry_villager_random,
    );
    assert_eq!(
        angry_villager.provider,
        "HeartParticle.AngryVillagerProvider"
    );
    assert_eq!(angry_villager.previous_position, [1.0, 0.5, 0.0]);
    assert_eq!(angry_villager.position, [1.0, 0.5, 0.0]);
    assert_eq!(angry_villager.lifetime_ticks, 16);
    assert_eq!(
        angry_villager.quad_size_curve,
        ParticleQuadSizeCurve::GrowToBase
    );
    assert_eq!(angry_villager.color, [1.0, 1.0, 1.0, 1.0]);

    let mut heart_random = ParticleRandom::new(51);
    let heart = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:heart", 1.0),
        &mut heart_random,
    );
    assert_eq!(heart.provider, "HeartParticle.Provider");
    assert_eq!(heart.lifetime_ticks, 16);
    assert_eq!(heart.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_range_f32(heart.base_quad_size, 0.15, 0.3);
    assert_eq!(heart.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(heart.friction, 0.86);
    assert!(!heart.has_physics);
    assert!(heart.speed_up_when_y_motion_is_blocked);
    assert_range_f64(heart.velocity[1], 0.098, 0.102);

    let mut dragon_random = ParticleRandom::new(46);
    let dragon_breath = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dragon_breath", 1.0),
        &mut dragon_random,
    );
    assert_eq!(dragon_breath.provider, "DragonBreathParticle.Provider");
    assert_eq!(
        dragon_breath.quad_size_curve,
        ParticleQuadSizeCurve::GrowToBase
    );
    assert_range_f32(dragon_breath.base_quad_size, 0.075, 0.15);
    assert_range_f32(dragon_breath.color[0], 0.717_647_1, 0.874_509_8);
    assert_close_f32(dragon_breath.color[1], 0.0);
    assert_range_f32(dragon_breath.color[2], 0.823_529_4, 0.976_470_6);
    assert_eq!(dragon_breath.friction, 0.96);
    assert_eq!(dragon_breath.gravity, 0.0);
    assert!(!dragon_breath.has_physics);
    assert_eq!(
        dragon_breath.tick_motion,
        ParticleTickMotionDescriptor::DragonBreath
    );

    let mut end_rod_random = ParticleRandom::new(79);
    let mut end_rod_command = spawn_command("minecraft:end_rod", 1.0);
    end_rod_command.velocity = [1.0, 2.0, 3.0];
    let end_rod = ParticleInstance::from_spawn_command(end_rod_command, &mut end_rod_random);
    assert_eq!(end_rod.provider, "EndRodParticle.Provider");
    assert_eq!(end_rod.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(end_rod.base_quad_size, 0.075, 0.15);
    assert_eq!(end_rod.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(end_rod.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!((60..=71).contains(&end_rod.lifetime_ticks));
    assert_eq!(end_rod.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(end_rod.friction, 0.91);
    assert_eq!(end_rod.gravity, 0.0125);
    assert!(end_rod.has_physics);
    assert!(end_rod.moves_without_collision);
    assert_eq!(end_rod.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(end_rod.alpha_curve, ParticleAlphaCurve::SimpleAnimatedFade);
    assert_eq!(
        end_rod.color_fade_target,
        Some(descriptors::END_ROD_FADE_COLOR)
    );

    let mut dolphin_random = ParticleRandom::new(53);
    let dolphin = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dolphin", 1.0),
        &mut dolphin_random,
    );
    assert_eq!(
        dolphin.provider,
        "SuspendedTownParticle.DolphinSpeedProvider"
    );
    assert!((10..=50).contains(&dolphin.lifetime_ticks));
    assert_close_f32(dolphin.color[0], 0.3);
    assert_close_f32(dolphin.color[1], 0.5);
    assert_close_f32(dolphin.color[2], 1.0);
    assert_range_f32(dolphin.color[3], 0.3, 1.0);
    assert_eq!(dolphin.friction, 0.99);
    assert!(dolphin.has_physics);
    assert!(dolphin.moves_without_collision);
    assert_ne!(dolphin.velocity, [0.0, 0.0, 0.0]);

    let mut happy_villager_random = ParticleRandom::new(47);
    let happy_villager = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:happy_villager", 1.0),
        &mut happy_villager_random,
    );
    assert_eq!(
        happy_villager.provider,
        "SuspendedTownParticle.HappyVillagerProvider"
    );
    assert_eq!(
        happy_villager.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        happy_villager.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert_range_f32(happy_villager.base_quad_size, 0.05, 0.22);
    assert_eq!(happy_villager.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(happy_villager.friction, 0.99);
    assert_eq!(happy_villager.gravity, 0.0);
    assert!(happy_villager.has_physics);
    assert!(happy_villager.moves_without_collision);
    assert_ne!(happy_villager.velocity, [0.0, 0.0, 0.0]);

    let mut composter_random = ParticleRandom::new(48);
    let composter = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:composter", 1.0),
        &mut composter_random,
    );
    assert_eq!(
        composter.provider,
        "SuspendedTownParticle.ComposterFillProvider"
    );
    assert!((3..=7).contains(&composter.lifetime_ticks));
    assert_eq!(composter.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_eq!(composter.color, [1.0, 1.0, 1.0, 1.0]);
    assert!(composter.moves_without_collision);
    assert_ne!(composter.velocity, [0.0, 0.0, 0.0]);

    let mut mycelium_random = ParticleRandom::new(49);
    let mycelium = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:mycelium", 1.0),
        &mut mycelium_random,
    );
    assert_eq!(mycelium.provider, "SuspendedTownParticle.Provider");
    assert_range_f32(mycelium.color[0], 0.2, 0.3);
    assert_eq!(mycelium.color[0], mycelium.color[1]);
    assert_eq!(mycelium.color[1], mycelium.color[2]);
    assert_eq!(mycelium.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!(mycelium.moves_without_collision);
    assert_ne!(mycelium.velocity, [0.0, 0.0, 0.0]);

    let mut egg_crack_random = ParticleRandom::new(50);
    let egg_crack = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:egg_crack", 1.0),
        &mut egg_crack_random,
    );
    assert_eq!(egg_crack.provider, "SuspendedTownParticle.EggCrackProvider");
    assert_eq!(egg_crack.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(egg_crack.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert!(egg_crack.moves_without_collision);
    assert_ne!(egg_crack.velocity, [0.0, 0.0, 0.0]);

    let mut smoke_random = ParticleRandom::new(44);
    let smoke = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:white_smoke", 1.0),
        &mut smoke_random,
    );
    assert_eq!(smoke.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_close_f32(smoke.color[0], 186.0 / 255.0);
    assert_close_f32(smoke.color[1], 177.0 / 255.0);
    assert_close_f32(smoke.color[2], 194.0 / 255.0);

    let mut poof_random = ParticleRandom::new(45);
    let poof = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:poof", 1.0),
        &mut poof_random,
    );
    assert_eq!(poof.quad_size_curve, ParticleQuadSizeCurve::Constant);
    assert_range_f32(poof.base_quad_size, 0.1, 0.7);
    assert_range_f32(poof.color[0], 0.7, 1.0);

    let mut portal_random = ParticleRandom::new(80);
    let mut portal_command = spawn_command("minecraft:portal", 1.0);
    portal_command.velocity = [-5.0, 0.0, 5.0];
    let portal = ParticleInstance::from_spawn_command(portal_command, &mut portal_random);
    assert_eq!(portal.provider, "PortalParticle.Provider");
    assert_eq!(portal.sprite_selection, ParticleSpriteSelection::Random);
    assert_eq!(portal.quad_size_curve, ParticleQuadSizeCurve::Portal);
    assert_range_f32(portal.base_quad_size, 0.05, 0.07);
    assert_close_f32(portal.color[0], portal.color[2] * 0.9);
    assert_close_f32(portal.color[1], portal.color[2] * 0.3);
    assert_eq!(portal.color[3], 1.0);
    assert!((40..=49).contains(&portal.lifetime_ticks));
    assert_eq!(portal.velocity, [-5.0, 0.0, 5.0]);
    assert_eq!(portal.friction, 0.98);
    assert_eq!(portal.gravity, 0.0);
    assert!(portal.has_physics);
    assert!(portal.moves_without_collision);
    assert_eq!(portal.tick_motion, ParticleTickMotionDescriptor::Portal);
    assert_eq!(
        portal.light_emission,
        ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
    );

    let mut reverse_portal_random = ParticleRandom::new(81);
    let mut reverse_portal_command = spawn_command("minecraft:reverse_portal", 1.0);
    reverse_portal_command.velocity = [-5.0, 0.0, 5.0];
    let reverse_portal =
        ParticleInstance::from_spawn_command(reverse_portal_command, &mut reverse_portal_random);
    assert_eq!(
        reverse_portal.provider,
        "ReversePortalParticle.ReversePortalProvider"
    );
    assert_eq!(
        reverse_portal.sprite_selection,
        ParticleSpriteSelection::Random
    );
    assert_eq!(
        reverse_portal.quad_size_curve,
        ParticleQuadSizeCurve::ReversePortal
    );
    assert_range_f32(reverse_portal.base_quad_size, 0.075, 0.105);
    assert_close_f32(reverse_portal.color[0], reverse_portal.color[2] * 0.9);
    assert_close_f32(reverse_portal.color[1], reverse_portal.color[2] * 0.3);
    assert_eq!(reverse_portal.color[3], 1.0);
    assert!((60..=61).contains(&reverse_portal.lifetime_ticks));
    assert_eq!(reverse_portal.velocity, [-5.0, 0.0, 5.0]);
    assert!(reverse_portal.has_physics);
    assert!(reverse_portal.moves_without_collision);
    assert_eq!(
        reverse_portal.tick_motion,
        ParticleTickMotionDescriptor::ReversePortal
    );
    assert_eq!(
        reverse_portal.light_emission,
        ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic
    );

    let mut explosion_random = ParticleRandom::new(70);
    let mut explosion_command = spawn_command("minecraft:explosion", 1.0);
    explosion_command.velocity = [0.5, 2.0, 3.0];
    let explosion = ParticleInstance::from_spawn_command(explosion_command, &mut explosion_random);
    assert_eq!(explosion.provider, "HugeExplosionParticle.Provider");
    assert_eq!(explosion.sprite_selection, ParticleSpriteSelection::Age);
    assert_close_f32(explosion.base_quad_size, 1.5);
    assert_range_f32(explosion.color[0], 0.4, 1.0);
    assert_eq!(explosion.color[0], explosion.color[1]);
    assert_eq!(explosion.color[1], explosion.color[2]);
    assert_eq!(explosion.color[3], 1.0);
    assert!((6..=9).contains(&explosion.lifetime_ticks));
    assert_eq!(explosion.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(explosion.friction, 0.98);
    assert!(explosion.has_physics);

    let mut explosion_emitter_command = spawn_command("minecraft:explosion_emitter", 1.0);
    explosion_emitter_command.child_spawn_templates = vec![ParticleChildSpawnTemplate {
        particle_type_id: 23,
        particle_id: "minecraft:explosion".to_string(),
        sprite_ids: vec!["minecraft:explosion_0".to_string()],
    }];
    let explosion_emitter =
        ParticleInstance::from_spawn_command(explosion_emitter_command, &mut explosion_random);
    assert_eq!(
        explosion_emitter.provider,
        "HugeExplosionSeedParticle.Provider"
    );
    assert_eq!(
        explosion_emitter.render_group,
        ParticleRenderGroup::NoRender
    );
    assert_eq!(explosion_emitter.lifetime_ticks, 8);
    assert_eq!(explosion_emitter.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        explosion_emitter.child_emission,
        Some(ParticleChildEmissionDescriptor::HugeExplosionSeed)
    );

    let mut sonic_boom_random = ParticleRandom::new(73);
    let mut sonic_boom_command = spawn_command("minecraft:sonic_boom", 1.0);
    sonic_boom_command.velocity = [1.0, 2.0, 3.0];
    let sonic_boom =
        ParticleInstance::from_spawn_command(sonic_boom_command, &mut sonic_boom_random);
    assert_eq!(sonic_boom.provider, "SonicBoomParticle.Provider");
    assert_eq!(sonic_boom.sprite_selection, ParticleSpriteSelection::Age);
    assert_close_f32(sonic_boom.base_quad_size, 1.5);
    assert_range_f32(sonic_boom.color[0], 0.4, 1.0);
    assert_eq!(sonic_boom.color[0], sonic_boom.color[1]);
    assert_eq!(sonic_boom.color[1], sonic_boom.color[2]);
    assert_eq!(sonic_boom.lifetime_ticks, 16);
    assert_eq!(sonic_boom.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(sonic_boom.friction, 0.98);
    assert!(sonic_boom.has_physics);

    let mut sculk_charge_random = ParticleRandom::new(78);
    let mut sculk_charge_command = spawn_command("minecraft:sculk_charge", 1.0);
    sculk_charge_command.velocity = [1.0, 2.0, 3.0];
    sculk_charge_command.option_roll = Some(0.75);
    let sculk_charge =
        ParticleInstance::from_spawn_command(sculk_charge_command, &mut sculk_charge_random);
    assert_eq!(sculk_charge.provider, "SculkChargeParticle.Provider");
    assert_eq!(sculk_charge.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(sculk_charge.base_quad_size, 0.15, 0.3);
    assert_eq!(sculk_charge.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        sculk_charge.quad_size_curve,
        ParticleQuadSizeCurve::Constant
    );
    assert!((8..=19).contains(&sculk_charge.lifetime_ticks));
    assert_eq!(sculk_charge.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(sculk_charge.friction, 0.96);
    assert_eq!(sculk_charge.gravity, 0.0);
    assert!(!sculk_charge.has_physics);
    assert_eq!(sculk_charge.option_roll, Some(0.75));
    assert_eq!(sculk_charge.previous_roll, 0.75);
    assert_eq!(sculk_charge.roll, 0.75);

    let mut sculk_charge_pop_random = ParticleRandom::new(74);
    let mut sculk_charge_pop_command = spawn_command("minecraft:sculk_charge_pop", 1.0);
    sculk_charge_pop_command.velocity = [1.0, 2.0, 3.0];
    let sculk_charge_pop = ParticleInstance::from_spawn_command(
        sculk_charge_pop_command,
        &mut sculk_charge_pop_random,
    );
    assert_eq!(sculk_charge_pop.provider, "SculkChargePopParticle.Provider");
    assert_eq!(
        sculk_charge_pop.sprite_selection,
        ParticleSpriteSelection::Age
    );
    assert_range_f32(sculk_charge_pop.base_quad_size, 0.1, 0.2);
    assert_eq!(sculk_charge_pop.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((6..=9).contains(&sculk_charge_pop.lifetime_ticks));
    assert_eq!(sculk_charge_pop.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(sculk_charge_pop.friction, 0.96);
    assert!(!sculk_charge_pop.has_physics);

    let mut firefly_random = ParticleRandom::new(75);
    let mut firefly_command = spawn_command("minecraft:firefly", 1.0);
    firefly_command.velocity = [0.0, 0.25, 0.0];
    let firefly = ParticleInstance::from_spawn_command(firefly_command, &mut firefly_random);
    assert_eq!(firefly.provider, "FireflyParticle.FireflyProvider");
    assert_eq!(firefly.sprite_selection, ParticleSpriteSelection::Random);
    assert_range_f32(firefly.base_quad_size, 0.1125, 0.225);
    assert_eq!(firefly.color, [1.0, 1.0, 1.0, 0.0]);
    assert!((200..=300).contains(&firefly.lifetime_ticks));
    assert_range_f64(firefly.velocity[0], -0.15, 0.15);
    assert_range_f64(firefly.velocity[1], -0.07, 0.23);
    assert_range_f64(firefly.velocity[2], -0.15, 0.15);
    assert_eq!(firefly.friction, 0.96);
    assert!(firefly.has_physics);
    assert!(firefly.speed_up_when_y_motion_is_blocked);
    assert_eq!(firefly.tick_motion, ParticleTickMotionDescriptor::Firefly);
    assert_eq!(firefly.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(firefly.alpha_curve, ParticleAlphaCurve::FireflyFade);
    assert_eq!(
        firefly.light_emission,
        ParticleLightEmissionDescriptor::Firefly
    );

    let mut shriek_random = ParticleRandom::new(76);
    let mut shriek_command = spawn_command("minecraft:shriek", 1.0);
    shriek_command.velocity = [1.0, 2.0, 3.0];
    shriek_command.initial_delay_ticks = 15;
    let shriek = ParticleInstance::from_spawn_command(shriek_command, &mut shriek_random);
    assert_eq!(shriek.provider, "ShriekParticle.Provider");
    assert_eq!(shriek.sprite_selection, ParticleSpriteSelection::Random);
    assert_close_f32(shriek.base_quad_size, 0.85);
    assert_eq!(shriek.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(shriek.quad_size_curve, ParticleQuadSizeCurve::Shriek);
    assert_eq!(shriek.alpha_curve, ParticleAlphaCurve::ShriekFade);
    assert_eq!(
        shriek.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(shriek.lifetime_ticks, 30);
    assert_eq!(shriek.velocity, [0.0, 0.1, 0.0]);
    assert_eq!(shriek.friction, 0.98);
    assert!(shriek.has_physics);
    assert_eq!(shriek.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(shriek.delay_ticks, 15);

    let mut detection_random = ParticleRandom::new(82);
    let mut detection_command = spawn_command("minecraft:trial_spawner_detection", 1.0);
    detection_command.velocity = [0.25, 0.5, -0.75];
    let detection = ParticleInstance::from_spawn_command(detection_command, &mut detection_random);
    assert_eq!(detection.provider, "TrialSpawnerDetectionParticle.Provider");
    assert_eq!(detection.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(detection.base_quad_size, 0.1125, 0.225);
    assert_eq!(detection.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(detection.quad_size_curve, ParticleQuadSizeCurve::GrowToBase);
    assert_eq!(
        detection.light_emission,
        ParticleLightEmissionDescriptor::FullBlock
    );
    assert_eq!(
        detection.facing_camera_mode,
        ParticleFacingCameraMode::LookAtY
    );
    assert!((12..=24).contains(&detection.lifetime_ticks));
    // TrialSpawnerDetectionParticle scales the base spread per axis by
    // (0.0, 0.9, 0.0) and threads the command velocity through with no offset,
    // so x/z drop the base spread and pass straight through while y keeps the
    // 0.9-scaled upward drift on top of the command y.
    let detection_spread = expected_base_ash_smoke_velocity(82, [0.0, 0.9, 0.0], false);
    assert_close_f64(detection.velocity[0], detection_spread[0] + 0.25);
    assert_close_f64(detection.velocity[1], detection_spread[1] + 0.5);
    assert_close_f64(detection.velocity[2], detection_spread[2] - 0.75);
    assert_close_f64(detection.velocity[0], 0.25);
    assert_close_f64(detection.velocity[2], -0.75);
    assert_eq!(detection.friction, 0.96);
    assert_eq!(detection.gravity, -0.1);
    assert!(detection.has_physics);
    assert!(detection.speed_up_when_y_motion_is_blocked);
    assert_eq!(detection.render_layer, ParticleRenderLayer::Opaque);

    let mut dust_plume_random = ParticleRandom::new(86);
    let mut dust_plume_command = spawn_command("minecraft:dust_plume", 1.0);
    dust_plume_command.velocity = [0.25, 0.5, -0.75];
    let dust_plume =
        ParticleInstance::from_spawn_command(dust_plume_command, &mut dust_plume_random);
    assert_eq!(dust_plume.provider, "DustPlumeParticle.Provider");
    assert_eq!(dust_plume.sprite_selection, ParticleSpriteSelection::Age);
    assert_range_f32(dust_plume.base_quad_size, 0.075, 0.15);
    assert_range_f32(dust_plume.color[0], 186.0 / 255.0 - 0.2, 186.0 / 255.0);
    assert_eq!(
        dust_plume.quad_size_curve,
        ParticleQuadSizeCurve::GrowToBase
    );
    assert!((7..=35).contains(&dust_plume.lifetime_ticks));
    // DustPlumeParticle scales the base spread per axis by (0.7, 0.6, 0.7) and
    // adds the command velocity with +0.15 on y.
    let dust_plume_spread = expected_base_ash_smoke_velocity(86, [0.7, 0.6, 0.7], false);
    assert_close_f64(dust_plume.velocity[0], dust_plume_spread[0] + 0.25);
    assert_close_f64(dust_plume.velocity[1], dust_plume_spread[1] + 0.65);
    assert_close_f64(dust_plume.velocity[2], dust_plume_spread[2] - 0.75);
    assert_eq!(dust_plume.friction, 0.96);
    assert_eq!(dust_plume.gravity, 0.5);
    assert!(!dust_plume.has_physics);
    assert!(dust_plume.speed_up_when_y_motion_is_blocked);
    assert_eq!(
        dust_plume.tick_motion,
        ParticleTickMotionDescriptor::DustPlume
    );
    assert_eq!(dust_plume.render_layer, ParticleRenderLayer::Opaque);

    let mut gust_random = ParticleRandom::new(71);
    let mut gust_command = spawn_command("minecraft:gust", 1.0);
    gust_command.velocity = [1.0, 2.0, 3.0];
    let gust = ParticleInstance::from_spawn_command(gust_command, &mut gust_random);
    assert_eq!(gust.provider, "GustParticle.Provider");
    assert_eq!(gust.sprite_selection, ParticleSpriteSelection::Age);
    assert_close_f32(gust.base_quad_size, 1.0);
    assert_eq!(gust.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((12..=15).contains(&gust.lifetime_ticks));
    assert_eq!(gust.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(gust.friction, 0.98);
    assert!(gust.has_physics);

    let mut small_gust_random = ParticleRandom::new(72);
    let small_gust = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:small_gust", 1.0),
        &mut small_gust_random,
    );
    assert_eq!(small_gust.provider, "GustParticle.SmallProvider");
    assert_eq!(small_gust.sprite_selection, ParticleSpriteSelection::Age);
    assert_close_f32(small_gust.base_quad_size, 0.15);
    assert_eq!(small_gust.color, [1.0, 1.0, 1.0, 1.0]);
    assert!((12..=15).contains(&small_gust.lifetime_ticks));
    assert_eq!(small_gust.velocity, [0.0, 0.0, 0.0]);
    assert!(small_gust.has_physics);

    let mut gust_emitter_large_command = spawn_command("minecraft:gust_emitter_large", 1.0);
    gust_emitter_large_command.child_spawn_templates = vec![ParticleChildSpawnTemplate {
        particle_type_id: 24,
        particle_id: "minecraft:gust".to_string(),
        sprite_ids: vec!["minecraft:gust_0".to_string()],
    }];
    let gust_emitter_large =
        ParticleInstance::from_spawn_command(gust_emitter_large_command, &mut gust_random);
    assert_eq!(
        gust_emitter_large.provider,
        "GustSeedParticle.Provider(3.0,7,0)"
    );
    assert_eq!(
        gust_emitter_large.render_group,
        ParticleRenderGroup::NoRender
    );
    assert_eq!(gust_emitter_large.lifetime_ticks, 8);
    assert_eq!(gust_emitter_large.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        gust_emitter_large.child_emission,
        Some(ParticleChildEmissionDescriptor::GustSeed {
            scale_tenths: 30,
            vanilla_lifetime: 7,
            tick_delay: 0,
        })
    );

    let gust_emitter_small = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:gust_emitter_small", 1.0),
        &mut small_gust_random,
    );
    assert_eq!(
        gust_emitter_small.provider,
        "GustSeedParticle.Provider(1.0,3,2)"
    );
    assert_eq!(
        gust_emitter_small.render_group,
        ParticleRenderGroup::NoRender
    );
    assert_eq!(gust_emitter_small.lifetime_ticks, 4);
    assert_eq!(
        gust_emitter_small.child_emission,
        Some(ParticleChildEmissionDescriptor::GustSeed {
            scale_tenths: 10,
            vanilla_lifetime: 3,
            tick_delay: 2,
        })
    );
}

#[test]
fn particle_instances_record_vanilla_render_groups_and_layers() {
    let mut random = ParticleRandom::new(0);
    let opaque =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:flame", 1.0), &mut random);
    let cloud =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:cloud", 2.0), &mut random);
    let squid_ink = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:squid_ink", 3.0),
        &mut random,
    );
    let sculk = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:sculk_charge", 4.0),
        &mut random,
    );
    let glow =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:glow", 5.0), &mut random);
    let current_down = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:current_down", 6.0),
        &mut random,
    );
    let enchant =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:enchant", 7.0), &mut random);
    let nautilus =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:nautilus", 8.0), &mut random);
    let totem = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:totem_of_undying", 9.0),
        &mut random,
    );
    let vault = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:vault_connection", 10.0),
        &mut random,
    );
    let ominous_spawn = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:ominous_spawning", 11.0),
        &mut random,
    );
    let mut vibration_command = spawn_command("minecraft:vibration", 12.0);
    vibration_command.option_target = Some([12.0, 1.0, 0.0]);
    vibration_command.option_duration_ticks = Some(20);
    let vibration = ParticleInstance::from_spawn_command(vibration_command, &mut random);
    let unresolved_vibration = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:vibration", 13.0),
        &mut random,
    );
    let elder_guardian = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:elder_guardian", 14.0),
        &mut random,
    );
    let terrain =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:block", 15.0), &mut random);
    let block_marker = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:block_marker", 16.0),
        &mut random,
    );
    let dust_pillar = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dust_pillar", 17.0),
        &mut random,
    );
    let block_crumble = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:block_crumble", 18.0),
        &mut random,
    );
    let item =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:item", 19.0), &mut random);
    let item_slime = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_slime", 20.0),
        &mut random,
    );
    let item_cobweb = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_cobweb", 21.0),
        &mut random,
    );
    let item_snowball = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_snowball", 22.0),
        &mut random,
    );
    let falling_dust = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_dust", 23.0),
        &mut random,
    );

    assert_eq!(opaque.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(cloud.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(squid_ink.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(sculk.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(glow.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(current_down.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(enchant.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(nautilus.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(totem.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(vault.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(ominous_spawn.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(vibration.render_group, ParticleRenderGroup::SingleQuads);
    assert_eq!(
        unresolved_vibration.render_group,
        ParticleRenderGroup::NoRender
    );
    assert_eq!(
        elder_guardian.render_group,
        ParticleRenderGroup::ElderGuardians
    );
    assert_eq!(elder_guardian.provider, "ElderGuardianParticle.Provider");
    assert_eq!(elder_guardian.lifetime_ticks, 30);
    assert_eq!(opaque.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(cloud.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(squid_ink.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(sculk.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(totem.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(glow.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(current_down.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(enchant.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(nautilus.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(vault.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(ominous_spawn.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(vibration.render_layer, ParticleRenderLayer::Translucent);
    assert_eq!(
        elder_guardian.render_layer,
        ParticleRenderLayer::Translucent
    );
    assert_eq!(terrain.render_layer, ParticleRenderLayer::OpaqueTerrain);
    assert_eq!(
        block_marker.render_layer,
        ParticleRenderLayer::OpaqueTerrain
    );
    assert_eq!(dust_pillar.render_layer, ParticleRenderLayer::OpaqueTerrain);
    assert_eq!(
        block_crumble.render_layer,
        ParticleRenderLayer::OpaqueTerrain
    );
    assert_eq!(item.render_layer, ParticleRenderLayer::OpaqueItems);
    assert_eq!(item_slime.render_layer, ParticleRenderLayer::OpaqueItems);
    assert_eq!(item_cobweb.render_layer, ParticleRenderLayer::OpaqueItems);
    assert_eq!(item_snowball.render_layer, ParticleRenderLayer::OpaqueItems);
    assert_eq!(falling_dust.render_layer, ParticleRenderLayer::Opaque);
    for particle in [
        &opaque,
        &cloud,
        &squid_ink,
        &sculk,
        &glow,
        &current_down,
        &enchant,
        &nautilus,
        &totem,
        &vault,
        &ominous_spawn,
        &vibration,
        &unresolved_vibration,
        &elder_guardian,
        &falling_dust,
    ] {
        assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Particles);
    }
    for particle in [&terrain, &block_marker, &dust_pillar, &block_crumble] {
        assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Terrain);
    }
    for particle in [&item, &item_slime, &item_cobweb, &item_snowball] {
        assert_eq!(particle.texture_atlas, ParticleTextureAtlasKind::Items);
    }
}

#[test]
fn elder_guardian_particle_render_instances_use_vanilla_special_group_state() {
    let mut random = ParticleRandom::new(0);
    let mut elder_guardian = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:elder_guardian", 14.0),
        &mut random,
    );
    elder_guardian.age_ticks = 10;
    elder_guardian.position = [999.0, 999.0, 999.0];
    let mut delayed = elder_guardian.clone();
    delayed.delay_ticks = 1;
    let cloud =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:cloud", 2.0), &mut random);
    let pose = crate::CameraPose {
        position: [10.0, 64.0, -3.0],
        y_rot: 45.0,
        x_rot: -15.0,
        eye_height: 1.62,
    };

    let instances =
        elder_guardian_particle_render_instances([&elder_guardian, &delayed, &cloud], pose);

    assert_eq!(instances.len(), 1);
    let age_scale = (10.0 + DEFAULT_PARTICLE_RENDER_PARTIAL_TICK) / 30.0;
    assert_eq!(&instances[0].tint[0..3], &[1.0, 1.0, 1.0]);
    assert_close_f32(
        instances[0].tint[3],
        0.05 + 0.5 * (age_scale * std::f32::consts::PI).sin(),
    );
    assert_mat4_close(
        instances[0].transform,
        elder_guardian_particle_model_transform(pose, age_scale),
    );
}

#[test]
fn particle_instances_preserve_terrain_and_item_option_metadata() {
    let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
    let mut block_command = spawn_command("minecraft:block", 0.0);
    block_command.option_block = Some(ParticleBlockOptionState {
        block_state_id: 321,
    });
    let mut item_command = spawn_command("minecraft:item", 1.0);
    item_command.option_item = Some(ParticleItemOptionState {
        item_id: 42,
        count: 3,
        component_patch_len: 2,
    });

    let block = ParticleInstance::from_spawn_command(block_command, &mut random);
    let item = ParticleInstance::from_spawn_command(item_command, &mut random);

    assert_eq!(block.render_layer, ParticleRenderLayer::OpaqueTerrain);
    assert_eq!(
        block.option_block,
        Some(ParticleBlockOptionState {
            block_state_id: 321
        })
    );
    assert_eq!(block.option_item, None);
    assert_eq!(item.render_layer, ParticleRenderLayer::OpaqueItems);
    assert_eq!(
        item.option_item,
        Some(ParticleItemOptionState {
            item_id: 42,
            count: 3,
            component_patch_len: 2,
        })
    );
    assert_eq!(item.option_block, None);
}

#[test]
fn particle_instances_record_terrain_and_item_atlas_provider_shape_and_sub_rects() {
    let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
    let block =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:block", 0.0), &mut random);
    let block_marker = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:block_marker", 1.0),
        &mut random,
    );
    let dust_pillar = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:dust_pillar", 2.0),
        &mut random,
    );
    let block_crumble = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:block_crumble", 3.0),
        &mut random,
    );
    let item =
        ParticleInstance::from_spawn_command(spawn_command("minecraft:item", 4.0), &mut random);
    let item_slime = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_slime", 5.0),
        &mut random,
    );
    let item_cobweb = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_cobweb", 6.0),
        &mut random,
    );
    let item_snowball = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:item_snowball", 7.0),
        &mut random,
    );
    let falling_dust = ParticleInstance::from_spawn_command(
        spawn_command("minecraft:falling_dust", 8.0),
        &mut random,
    );

    for terrain in [&block, &dust_pillar, &block_crumble] {
        assert_eq!(terrain.render_layer, ParticleRenderLayer::OpaqueTerrain);
        assert_range_f32(terrain.base_quad_size, 0.05, 0.1);
        assert_eq!(terrain.color, [0.6, 0.6, 0.6, 1.0]);
        assert_close_f32(terrain.gravity, 1.0);
        assert!(terrain.has_physics);
        assert_atlas_sub_rect(terrain);
    }
    assert_eq!(block.provider, "TerrainParticle.Provider");
    assert_eq!(dust_pillar.provider, "TerrainParticle.DustPillarProvider");
    assert_range_f32(dust_pillar.lifetime_ticks as f32, 20.0, 39.0);
    assert_eq!(block_crumble.provider, "TerrainParticle.CrumblingProvider");
    assert_range_f32(block_crumble.lifetime_ticks as f32, 1.0, 10.0);

    assert_eq!(block_marker.provider, "BlockMarker.Provider");
    assert_eq!(
        block_marker.render_layer,
        ParticleRenderLayer::OpaqueTerrain
    );
    assert_close_f32(block_marker.base_quad_size, 0.5);
    assert_eq!(block_marker.lifetime_ticks, 80);
    assert_close_f32(block_marker.gravity, 0.0);
    assert!(!block_marker.has_physics);
    assert_eq!(block_marker.atlas_uv_sub_rect, None);

    for item_particle in [&item, &item_slime, &item_cobweb, &item_snowball] {
        assert_eq!(item_particle.render_layer, ParticleRenderLayer::OpaqueItems);
        assert_range_f32(item_particle.base_quad_size, 0.05, 0.1);
        assert_eq!(item_particle.color, [1.0, 1.0, 1.0, 1.0]);
        assert_close_f32(item_particle.gravity, 1.0);
        assert!(item_particle.has_physics);
        assert_atlas_sub_rect(item_particle);
    }
    assert_eq!(item.provider, "BreakingItemParticle.Provider");
    assert_eq!(
        item.current_sprite_id.as_deref(),
        Some("minecraft:generic_0")
    );
    assert_eq!(item_slime.provider, "BreakingItemParticle.SlimeProvider");
    assert_eq!(
        item_slime.current_sprite_id.as_deref(),
        Some("minecraft:item/slime_ball")
    );
    assert_eq!(item_cobweb.provider, "BreakingItemParticle.CobwebProvider");
    assert_eq!(
        item_cobweb.current_sprite_id.as_deref(),
        Some("minecraft:block/cobweb")
    );
    assert_eq!(
        item_snowball.provider,
        "BreakingItemParticle.SnowballProvider"
    );
    assert_eq!(
        item_snowball.current_sprite_id.as_deref(),
        Some("minecraft:item/snowball")
    );

    assert_eq!(falling_dust.provider, "FallingDustParticle.Provider");
    assert_eq!(falling_dust.render_layer, ParticleRenderLayer::Opaque);
    assert_eq!(
        falling_dust.texture_atlas,
        ParticleTextureAtlasKind::Particles
    );
    assert_eq!(falling_dust.sprite_selection, ParticleSpriteSelection::Age);
    assert_eq!(falling_dust.current_sprite_index, Some(0));
    assert_range_f32(falling_dust.lifetime_ticks as f32, 28.0, 144.0);
    assert_range_f32(falling_dust.base_quad_size, 0.067_499_995, 0.135);
    assert_eq!(
        falling_dust.quad_size_curve,
        ParticleQuadSizeCurve::GrowToBase
    );
    assert_eq!(falling_dust.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(falling_dust.velocity, [0.0, 0.0, 0.0]);
    assert_eq!(
        falling_dust.tick_motion,
        ParticleTickMotionDescriptor::FallingDust
    );
    assert_range_f32(falling_dust.roll_speed, -0.05, 0.05);
    assert_range_f32(falling_dust.roll, 0.0, std::f32::consts::PI * 2.0);
    assert_close_f32(falling_dust.previous_roll, falling_dust.roll);
    assert_eq!(falling_dust.atlas_uv_sub_rect, None);
}

#[test]
fn falling_dust_instance_uses_option_color_for_block_dust_tint() {
    let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
    let mut command = spawn_command("minecraft:falling_dust", 0.0);
    command.option_color = Some([0.86, 0.83, 0.63, 1.0]);

    let falling_dust = ParticleInstance::from_spawn_command(command, &mut random);

    assert_eq!(falling_dust.provider, "FallingDustParticle.Provider");
    assert_eq!(falling_dust.color, [0.86, 0.83, 0.63, 1.0]);
    assert_eq!(
        falling_dust.quad_size_curve,
        ParticleQuadSizeCurve::GrowToBase
    );
}

#[test]
fn particle_runtime_snowflake_applies_vanilla_post_tick_damping() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut instance = test_instance_with_lifetime("minecraft:snowflake", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.velocity = [1.0, 2.0, 3.0];
    particles.active_instances.push_back(instance);

    particles.advance(1);

    let instance = &particles.active_instances()[0];
    assert_eq!(instance.previous_position, [1.0, 2.0, 3.0]);
    assert_close_f64(instance.position[0], 2.0);
    assert_close_f64(instance.position[1], 3.991);
    assert_close_f64(instance.position[2], 6.0);
    assert_close_f64(instance.velocity[0], 0.95);
    assert_close_f64(instance.velocity[1], 1.7919);
    assert_close_f64(instance.velocity[2], 2.85);
}

#[test]
fn particle_render_group_and_layer_order_match_vanilla_extract_passes() {
    assert_eq!(ParticleRenderGroup::SingleQuads.vanilla_order(), 0);
    assert_eq!(ParticleRenderGroup::ItemPickup.vanilla_order(), 1);
    assert_eq!(ParticleRenderGroup::ElderGuardians.vanilla_order(), 2);
    assert_eq!(ParticleRenderGroup::NoRender.vanilla_order(), 3);

    assert!(
        ParticleRenderLayer::OpaqueTerrain.vanilla_solid_translucent_order()
            < ParticleRenderLayer::TranslucentTerrain.vanilla_solid_translucent_order()
    );
    assert!(
        ParticleRenderLayer::OpaqueItems.vanilla_solid_translucent_order()
            < ParticleRenderLayer::TranslucentItems.vanilla_solid_translucent_order()
    );
    assert!(
        ParticleRenderLayer::Opaque.vanilla_solid_translucent_order()
            < ParticleRenderLayer::Translucent.vanilla_solid_translucent_order()
    );
}

#[test]
fn particle_billboard_vertices_follow_vanilla_group_and_layer_order() {
    let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
    cloud.position = [10.0, 0.0, 0.0];
    cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut block = test_instance_with_lifetime("minecraft:block", 20);
    block.position = [15.0, 0.0, 0.0];
    block.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
    flame.position = [20.0, 0.0, 0.0];
    flame.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut item = test_instance_with_lifetime("minecraft:item", 20);
    item.position = [25.0, 0.0, 0.0];
    item.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut soul = test_instance_with_lifetime("minecraft:soul", 20);
    soul.position = [30.0, 0.0, 0.0];
    soul.current_sprite_id = Some("minecraft:generic_0".to_string());
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&cloud, &block, &flame, &item, &soul],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        None,
    );

    assert_eq!(vertices.len(), 30);
    assert_close_f32(vertices[0].position[0], 14.9);
    assert_close_f32(vertices[6].position[0], 24.9);
    assert_close_f32(vertices[12].position[0], 19.9);
    assert_close_f32(vertices[18].position[0], 9.9);
    assert_close_f32(vertices[24].position[0], 29.9);
}

#[test]
fn particle_billboard_vertices_skip_non_single_quad_groups() {
    let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
    cloud.position = [10.0, 0.0, 0.0];
    cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut elder_guardian = test_instance_with_lifetime("minecraft:elder_guardian", 30);
    elder_guardian.position = [20.0, 0.0, 0.0];
    elder_guardian.current_sprite_id = Some("minecraft:generic_0".to_string());
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&cloud, &elder_guardian],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        None,
    );

    assert_eq!(vertices.len(), 6);
    assert_close_f32(vertices[0].position[0], 9.9);
}

#[test]
fn particle_billboard_vertices_split_vanilla_opaque_and_translucent_pipelines() {
    let mut cloud = test_instance_with_lifetime("minecraft:cloud", 20);
    cloud.position = [10.0, 0.0, 0.0];
    cloud.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
    flame.position = [20.0, 0.0, 0.0];
    flame.current_sprite_id = Some("minecraft:generic_0".to_string());
    let mut soul = test_instance_with_lifetime("minecraft:soul", 20);
    soul.position = [30.0, 0.0, 0.0];
    soul.current_sprite_id = Some("minecraft:generic_0".to_string());
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);
    let axes = ParticleBillboardAxes {
        right: Vec3::X,
        up: Vec3::Y,
    };

    let opaque_vertices = particle_billboard_vertices(
        [&cloud, &flame, &soul],
        &sprite_uvs,
        axes,
        Some(ParticlePipelineKind::Opaque),
    );
    let translucent_vertices = particle_billboard_vertices(
        [&cloud, &flame, &soul],
        &sprite_uvs,
        axes,
        Some(ParticlePipelineKind::Translucent),
    );

    assert_eq!(opaque_vertices.len(), 6);
    assert_close_f32(opaque_vertices[0].position[0], 19.9);
    assert_eq!(translucent_vertices.len(), 12);
    assert_close_f32(translucent_vertices[0].position[0], 9.9);
    assert_close_f32(translucent_vertices[6].position[0], 29.9);
}

#[test]
fn particle_pipeline_vertex_batches_split_texture_atlases_in_vanilla_layer_order() {
    let mut block = test_instance_with_lifetime("minecraft:block", 20);
    block.position = [20.0, 0.0, 0.0];
    block.current_sprite_id = Some("minecraft:block/oak_planks".to_string());
    let mut item = test_instance_with_lifetime("minecraft:item", 20);
    item.position = [30.0, 0.0, 0.0];
    item.current_sprite_id = Some("minecraft:item/apple".to_string());
    let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
    flame.position = [40.0, 0.0, 0.0];
    flame.current_sprite_id = Some("minecraft:generic_0".to_string());
    let particle_sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);
    let terrain_sprite_uvs = BTreeMap::from([(
        "minecraft:block/oak_planks".to_string(),
        ParticleUvRect {
            min: [0.1, 0.1],
            max: [0.2, 0.2],
        },
    )]);
    let item_sprite_uvs = BTreeMap::from([(
        "minecraft:item/apple".to_string(),
        ParticleUvRect {
            min: [0.3, 0.3],
            max: [0.4, 0.4],
        },
    )]);

    let batch = particle_pipeline_vertex_batch(
        [&flame, &item, &block],
        ParticleAtlasUvSets {
            particles: Some(&particle_sprite_uvs),
            terrain: Some(&terrain_sprite_uvs),
            items: Some(&item_sprite_uvs),
            terrain_translucent_sprites: None,
            item_translucent_sprites: None,
        },
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        ParticlePipelineKind::Opaque,
    );

    assert_eq!(batch.vertices.len(), 18);
    assert_eq!(
        batch.draws,
        vec![
            ParticleAtlasDrawRange {
                texture_atlas: ParticleTextureAtlasKind::Terrain,
                vertex_start: 0,
                vertex_count: 6,
            },
            ParticleAtlasDrawRange {
                texture_atlas: ParticleTextureAtlasKind::Items,
                vertex_start: 6,
                vertex_count: 6,
            },
            ParticleAtlasDrawRange {
                texture_atlas: ParticleTextureAtlasKind::Particles,
                vertex_start: 12,
                vertex_count: 6,
            },
        ]
    );
    assert_close_f32(batch.vertices[0].position[0], 19.9);
    assert_close_f32(batch.vertices[6].position[0], 29.9);
    assert_close_f32(batch.vertices[12].position[0], 39.9);
    assert_eq!(batch.vertices[0].uv, [0.1, 0.2]);
    assert_eq!(batch.vertices[6].uv, [0.3, 0.4]);
    assert_eq!(batch.vertices[12].uv, [0.0, 1.0]);
}

#[test]
fn fixed_item_particles_emit_with_item_atlas_sprite_uvs() {
    let mut random = ParticleRandom::new(DEFAULT_PARTICLE_RANDOM_SEED);
    let mut command = spawn_command("minecraft:item_slime", 0.0);
    command.sprite_ids.clear();
    let mut item_slime = ParticleInstance::from_spawn_command(command, &mut random);
    item_slime.position = [10.0, 0.0, 0.0];
    let item_sprite_uvs = BTreeMap::from([(
        "minecraft:item/slime_ball".to_string(),
        ParticleUvRect {
            min: [0.2, 0.2],
            max: [0.8, 0.8],
        },
    )]);

    let batch = particle_pipeline_vertex_batch(
        [&item_slime],
        ParticleAtlasUvSets {
            particles: None,
            terrain: None,
            items: Some(&item_sprite_uvs),
            terrain_translucent_sprites: None,
            item_translucent_sprites: None,
        },
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        ParticlePipelineKind::Opaque,
    );

    assert_eq!(batch.vertices.len(), 6);
    assert_eq!(
        batch.draws,
        vec![ParticleAtlasDrawRange {
            texture_atlas: ParticleTextureAtlasKind::Items,
            vertex_start: 0,
            vertex_count: 6,
        }]
    );
    assert!(batch.vertices[0].position[0] < 10.0);
}

#[test]
fn terrain_particles_use_current_sprite_translucency_for_pipeline() {
    let mut block = test_instance_with_lifetime("minecraft:block", 20);
    block.position = [10.0, 0.0, 0.0];
    block.current_sprite_id = Some("minecraft:block/tinted_glass".to_string());
    let terrain_sprite_uvs = BTreeMap::from([(
        "minecraft:block/tinted_glass".to_string(),
        ParticleUvRect {
            min: [0.25, 0.25],
            max: [0.75, 0.75],
        },
    )]);
    let terrain_translucent_sprites = BTreeSet::from(["minecraft:block/tinted_glass".to_string()]);
    let atlas_uvs = ParticleAtlasUvSets {
        particles: None,
        terrain: Some(&terrain_sprite_uvs),
        items: None,
        terrain_translucent_sprites: Some(&terrain_translucent_sprites),
        item_translucent_sprites: None,
    };
    let axes = ParticleBillboardAxes {
        right: Vec3::X,
        up: Vec3::Y,
    };

    let opaque =
        particle_pipeline_vertex_batch([&block], atlas_uvs, axes, ParticlePipelineKind::Opaque);
    let translucent = particle_pipeline_vertex_batch(
        [&block],
        atlas_uvs,
        axes,
        ParticlePipelineKind::Translucent,
    );

    assert_eq!(opaque.vertices.len(), 0);
    assert_eq!(translucent.vertices.len(), 6);
    assert_eq!(
        translucent.draws,
        vec![ParticleAtlasDrawRange {
            texture_atlas: ParticleTextureAtlasKind::Terrain,
            vertex_start: 0,
            vertex_count: 6,
        }]
    );
    assert_eq!(translucent.vertices[0].uv, [0.25, 0.75]);
}

#[test]
fn item_particles_use_current_sprite_translucency_for_pipeline() {
    let mut item = test_instance_with_lifetime("minecraft:item", 20);
    item.position = [10.0, 0.0, 0.0];
    item.current_sprite_id = Some("minecraft:item/glass_bottle".to_string());
    let item_sprite_uvs = BTreeMap::from([(
        "minecraft:item/glass_bottle".to_string(),
        ParticleUvRect {
            min: [0.125, 0.125],
            max: [0.625, 0.625],
        },
    )]);
    let item_translucent_sprites = BTreeSet::from(["minecraft:item/glass_bottle".to_string()]);
    let atlas_uvs = ParticleAtlasUvSets {
        particles: None,
        terrain: None,
        items: Some(&item_sprite_uvs),
        terrain_translucent_sprites: None,
        item_translucent_sprites: Some(&item_translucent_sprites),
    };
    let axes = ParticleBillboardAxes {
        right: Vec3::X,
        up: Vec3::Y,
    };

    let opaque =
        particle_pipeline_vertex_batch([&item], atlas_uvs, axes, ParticlePipelineKind::Opaque);
    let translucent =
        particle_pipeline_vertex_batch([&item], atlas_uvs, axes, ParticlePipelineKind::Translucent);

    assert_eq!(opaque.vertices.len(), 0);
    assert_eq!(translucent.vertices.len(), 6);
    assert_eq!(
        translucent.draws,
        vec![ParticleAtlasDrawRange {
            texture_atlas: ParticleTextureAtlasKind::Items,
            vertex_start: 0,
            vertex_count: 6,
        }]
    );
    assert_eq!(translucent.vertices[0].uv, [0.125, 0.625]);
}

#[test]
fn particle_quad_size_curves_follow_vanilla_shapes() {
    let mut constant = test_instance_with_lifetime("minecraft:squid_ink", 20);
    constant.base_quad_size = 0.5;
    constant.quad_size_curve = ParticleQuadSizeCurve::Constant;
    assert_close_f32(constant.quad_size_at_partial_tick(0.0), 0.5);
    constant.age_ticks = 20;
    assert_close_f32(constant.quad_size_at_partial_tick(0.0), 0.5);

    let mut cloud = test_instance_with_lifetime("minecraft:cloud", 64);
    cloud.base_quad_size = 0.4;
    cloud.quad_size_curve = ParticleQuadSizeCurve::GrowToBase;
    assert_close_f32(cloud.quad_size_at_partial_tick(0.0), 0.0);
    assert_close_f32(cloud.quad_size_at_partial_tick(0.5), 0.1);
    cloud.age_ticks = 2;
    assert_close_f32(cloud.quad_size_at_partial_tick(0.0), 0.4);

    let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
    flame.base_quad_size = 0.2;
    flame.quad_size_curve = ParticleQuadSizeCurve::Flame;
    assert_close_f32(flame.quad_size_at_partial_tick(0.0), 0.2);
    flame.age_ticks = 20;
    assert_close_f32(flame.quad_size_at_partial_tick(0.0), 0.1);

    let mut lava = test_instance_with_lifetime("minecraft:lava", 20);
    lava.base_quad_size = 0.2;
    lava.quad_size_curve = ParticleQuadSizeCurve::Lava;
    assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.2);
    lava.age_ticks = 10;
    assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.15);
    lava.age_ticks = 20;
    assert_close_f32(lava.quad_size_at_partial_tick(0.0), 0.0);

    let mut portal = test_instance_with_lifetime("minecraft:portal", 40);
    portal.base_quad_size = 0.06;
    portal.quad_size_curve = ParticleQuadSizeCurve::Portal;
    assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.0);
    portal.age_ticks = 20;
    assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.045);
    portal.age_ticks = 40;
    assert_close_f32(portal.quad_size_at_partial_tick(0.0), 0.06);

    let mut reverse_portal = test_instance_with_lifetime("minecraft:reverse_portal", 60);
    reverse_portal.base_quad_size = 0.09;
    reverse_portal.quad_size_curve = ParticleQuadSizeCurve::ReversePortal;
    assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.09);
    reverse_portal.age_ticks = 30;
    assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.06);
    reverse_portal.age_ticks = 60;
    assert_close_f32(reverse_portal.quad_size_at_partial_tick(0.0), 0.03);

    let mut shriek = test_instance_with_lifetime("minecraft:shriek", 30);
    shriek.base_quad_size = 0.85;
    shriek.quad_size_curve = ParticleQuadSizeCurve::Shriek;
    assert_close_f32(shriek.quad_size_at_partial_tick(0.0), 0.0);
    assert_close_f32(shriek.quad_size_at_partial_tick(0.5), 0.010_625);
    shriek.age_ticks = 30;
    assert_close_f32(shriek.quad_size_at_partial_tick(0.0), 0.637_5);

    let mut flash = test_instance_with_lifetime("minecraft:flash", 4);
    flash.quad_size_curve = ParticleQuadSizeCurve::FlashOverlay;
    flash.age_ticks = 1;
    assert_close_f32(
        flash.quad_size_at_partial_tick(0.5),
        7.1 * (0.5 * 0.25 * std::f32::consts::PI).sin(),
    );
}

#[test]
fn particle_runtime_expires_existing_active_before_intaking_pending_spawns() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    particles
        .active_instances
        .push_back(test_instance_with_lifetime("minecraft:poof", 0));
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:flame", 2.0)],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 1);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.active_instances, 1);
    assert_eq!(
        particles.active_instances()[0].particle_id,
        "minecraft:flame"
    );
    assert_eq!(particles.active_instances()[0].age_ticks, 0);
}

#[test]
fn particle_runtime_limits_active_instances_and_keeps_newest() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 2);
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![
            spawn_command("minecraft:cloud", 1.0),
            spawn_command("minecraft:flame", 2.0),
            spawn_command("minecraft:smoke", 3.0),
        ],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(0);

    assert_eq!(summary.intaken_instances, 3);
    assert_eq!(summary.dropped_active_instances, 1);
    assert_eq!(summary.active_instances, 2);
    assert_eq!(summary.total_instances_created, 3);
    assert_eq!(summary.total_dropped_active_instances, 1);
    let ids: Vec<_> = particles
        .active_instances()
        .iter()
        .map(|instance| instance.particle_id.as_str())
        .collect();
    assert_eq!(ids, vec!["minecraft:flame", "minecraft:smoke"]);
}

#[test]
fn particle_runtime_enforces_vanilla_spore_blossom_particle_limit() {
    let mut particles = ParticleRuntimeState::with_capacities(1105, 1105);
    let commands = (0..=VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT)
        .map(|index| spawn_command("minecraft:spore_blossom_air", index as f64))
        .collect();
    particles.submit_batch(ParticleSpawnBatch {
        commands,
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(0);

    assert_eq!(summary.intaken_instances, 1000);
    assert_eq!(summary.limited_particle_drops, 1);
    assert_eq!(summary.total_limited_particle_drops, 1);
    assert_eq!(summary.dropped_active_instances, 0);
    assert_eq!(summary.active_instances, 1000);
    assert_eq!(
        particles.active_instances()[0].particle_limit,
        Some(ParticleLimitDescriptor::SporeBlossom)
    );
    assert_eq!(
        particles.active_instances()[0].position[0],
        0.0,
        "ParticleEngine.add rejects the over-limit particle instead of evicting accepted ones"
    );
    assert_eq!(particles.active_instances()[999].position[0], 999.0);
}

#[test]
fn particle_runtime_releases_spore_blossom_limit_counts_on_expiry() {
    let mut particles = ParticleRuntimeState::with_capacities(1101, 1101);
    let commands = (0..VANILLA_SPORE_BLOSSOM_PARTICLE_LIMIT)
        .map(|index| spawn_command("minecraft:spore_blossom_air", index as f64))
        .collect();
    particles.submit_batch(ParticleSpawnBatch {
        commands,
        ..ParticleSpawnBatch::default()
    });
    let first_summary = particles.advance(0);
    assert_eq!(first_summary.intaken_instances, 1000);
    for instance in &mut particles.active_instances {
        instance.lifetime_ticks = 0;
    }
    particles.submit_batch(ParticleSpawnBatch {
        commands: vec![spawn_command("minecraft:spore_blossom_air", 1000.0)],
        ..ParticleSpawnBatch::default()
    });

    let summary = particles.advance(1);

    assert_eq!(summary.expired_instances, 1000);
    assert_eq!(summary.intaken_instances, 1);
    assert_eq!(summary.limited_particle_drops, 0);
    assert_eq!(summary.active_instances, 1);
    assert_eq!(particles.active_instances()[0].position[0], 1000.0);
}

#[test]
fn particle_runtime_refreshes_active_lights_from_world_positions() {
    let mut particles = ParticleRuntimeState::with_capacities(4, 4);
    let mut first = test_instance_with_lifetime("minecraft:cloud", 20);
    first.position = [1.25, 2.0, 3.75];
    let mut second = test_instance_with_lifetime("minecraft:smoke", 20);
    second.position = [-2.0, 9.5, 0.25];
    particles.active_instances.push_back(first);
    particles.active_instances.push_back(second);

    particles.refresh_lights(|position| {
        if position[0] < 0.0 {
            [1.25, f32::NAN]
        } else {
            [4.0 / 15.0, 11.0 / 15.0]
        }
    });

    assert_eq!(
        particles.active_instances()[0].light,
        [4.0 / 15.0, 11.0 / 15.0]
    );
    assert_eq!(particles.active_instances()[1].light, [1.0, 1.0]);
}

#[test]
fn particle_runtime_applies_vanilla_particle_light_emission_overrides() {
    let sampled_light = [2.0 / 15.0, 7.0 / 15.0];
    let mut particles = ParticleRuntimeState::with_capacities(17, 17);
    let cloud = test_instance_with_lifetime("minecraft:cloud", 20);
    let mut flame = test_instance_with_lifetime("minecraft:flame", 20);
    flame.age_ticks = 4;
    let mut glow = test_instance_with_lifetime("minecraft:glow", 4);
    glow.age_ticks = 1;
    let lava = test_instance_with_lifetime("minecraft:lava", 20);
    let sculk_soul = test_instance_with_lifetime("minecraft:sculk_soul", 20);
    let sculk_charge_pop = test_instance_with_lifetime("minecraft:sculk_charge_pop", 20);
    let attack_sweep = test_instance_with_lifetime("minecraft:sweep_attack", 4);
    let end_rod = test_instance_with_lifetime("minecraft:end_rod", 60);
    let totem = test_instance_with_lifetime("minecraft:totem_of_undying", 60);
    let mut enchant = test_instance_with_lifetime("minecraft:enchant", 40);
    enchant.age_ticks = 20;
    let mut portal = test_instance_with_lifetime("minecraft:portal", 40);
    portal.age_ticks = 20;
    let mut reverse_portal = test_instance_with_lifetime("minecraft:reverse_portal", 60);
    reverse_portal.age_ticks = 30;
    let shriek = test_instance_with_lifetime("minecraft:shriek", 30);
    let vault_connection = test_instance_with_lifetime("minecraft:vault_connection", 40);
    let vibration = test_instance_with_lifetime("minecraft:vibration", 40);
    let ominous_spawn = test_instance_with_lifetime("minecraft:ominous_spawning", 25);
    let mut firefly = test_instance_with_lifetime("minecraft:firefly", 100);
    firefly.age_ticks = 15;

    particles.active_instances.push_back(cloud);
    particles.active_instances.push_back(flame);
    particles.active_instances.push_back(glow);
    particles.active_instances.push_back(lava);
    particles.active_instances.push_back(sculk_soul);
    particles.active_instances.push_back(sculk_charge_pop);
    particles.active_instances.push_back(attack_sweep);
    particles.active_instances.push_back(end_rod);
    particles.active_instances.push_back(totem);
    particles.active_instances.push_back(enchant);
    particles.active_instances.push_back(portal);
    particles.active_instances.push_back(reverse_portal);
    particles.active_instances.push_back(shriek);
    particles.active_instances.push_back(vault_connection);
    particles.active_instances.push_back(vibration);
    particles.active_instances.push_back(ominous_spawn);
    particles.active_instances.push_back(firefly);

    particles.refresh_lights(|_| sampled_light);

    assert_eq!(particles.active_instances()[0].light, sampled_light);
    assert_close_f32(
        particles.active_instances()[1].light[0],
        sampled_light[0] + 4.5 / 20.0,
    );
    assert_close_f32(particles.active_instances()[1].light[1], sampled_light[1]);
    assert_close_f32(
        particles.active_instances()[2].light[0],
        sampled_light[0] + 1.5 / 4.0,
    );
    assert_close_f32(particles.active_instances()[2].light[1], sampled_light[1]);
    assert_eq!(
        particles.active_instances()[3].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(
        particles.active_instances()[4].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(
        particles.active_instances()[5].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(particles.active_instances()[6].light, [1.0, 1.0]);
    assert_eq!(particles.active_instances()[7].light, [1.0, 1.0]);
    assert_eq!(particles.active_instances()[8].light, [1.0, 1.0]);
    assert_close_f32(
        particles.active_instances()[9].light[0],
        sampled_light[0] + 0.5_f32.powi(4),
    );
    assert_close_f32(particles.active_instances()[9].light[1], sampled_light[1]);
    assert_close_f32(
        particles.active_instances()[10].light[0],
        sampled_light[0] + 0.5_f32.powi(4),
    );
    assert_close_f32(particles.active_instances()[10].light[1], sampled_light[1]);
    assert_close_f32(
        particles.active_instances()[11].light[0],
        sampled_light[0] + 0.5_f32.powi(4),
    );
    assert_close_f32(particles.active_instances()[11].light[1], sampled_light[1]);
    assert_eq!(
        particles.active_instances()[12].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(
        particles.active_instances()[13].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(
        particles.active_instances()[14].light,
        [1.0, sampled_light[1]]
    );
    assert_eq!(
        particles.active_instances()[15].light,
        [1.0, sampled_light[1]]
    );
    assert_close_f32(
        particles.active_instances()[16].light[0],
        firefly_fade_amount(15.5 / 100.0, 0.1, 0.3),
    );
    assert_close_f32(particles.active_instances()[16].light[1], 0.0);
}

#[test]
fn particle_billboard_vertices_emit_camera_facing_textured_quad() {
    let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 0.4;
    instance.color = [0.25, 0.5, 0.75, 0.8];
    instance.light = [0.4, 0.8];
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.25, 0.125],
            max: [0.5, 0.375],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        None,
    );

    assert_eq!(vertices.len(), 6);
    assert_close3_f32(vertices[0].position, [0.8, 1.8, 3.0]);
    assert_eq!(vertices[0].uv, [0.25, 0.375]);
    assert_eq!(vertices[0].color, [0.25, 0.5, 0.75, 0.8]);
    assert_eq!(vertices[0].light, [0.4, 0.8]);
    assert_close3_f32(vertices[2].position, [1.2, 2.2, 3.0]);
    assert_eq!(vertices[2].uv, [0.5, 0.125]);
    assert_eq!(vertices[2].color, [0.25, 0.5, 0.75, 0.8]);
    assert_eq!(vertices[2].light, [0.4, 0.8]);
    assert_close3_f32(vertices[5].position, [0.8, 2.2, 3.0]);
    assert_eq!(vertices[5].uv, [0.25, 0.125]);
    assert_eq!(vertices[5].color, [0.25, 0.5, 0.75, 0.8]);
    assert_eq!(vertices[5].light, [0.4, 0.8]);
}

#[test]
fn particle_billboard_vertices_apply_vanilla_atlas_sub_rect_uvs() {
    let mut instance = test_instance_with_lifetime("minecraft:block", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:block/oak_planks".to_string());
    instance.base_quad_size = 0.4;
    instance.atlas_uv_sub_rect = Some(ParticleAtlasUvSubRect {
        u_offset: 1.0,
        v_offset: 2.0,
    });
    let sprite_uvs = BTreeMap::from([(
        "minecraft:block/oak_planks".to_string(),
        ParticleUvRect {
            min: [0.2, 0.4],
            max: [1.0, 0.8],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Opaque),
    );

    assert_eq!(vertices.len(), 6);
    assert_eq!(vertices[0].uv, [0.6, 0.700_000_05]);
    assert_eq!(vertices[1].uv, [0.4, 0.700_000_05]);
    assert_eq!(vertices[2].uv, [0.4, 0.6]);
    assert_eq!(vertices[5].uv, [0.6, 0.6]);
}

#[test]
fn particle_billboard_vertices_apply_vanilla_lookat_y_facing_mode() {
    let mut instance = test_instance_with_lifetime("minecraft:trial_spawner_detection", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 2.0;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::Z,
            up: Vec3::new(0.0, 0.5, 0.866_025_4),
        },
        Some(ParticlePipelineKind::Opaque),
    );

    assert_eq!(
        instance.facing_camera_mode,
        ParticleFacingCameraMode::LookAtY
    );
    assert_eq!(vertices.len(), 6);
    assert_close3_f32(vertices[0].position, [1.0, 1.0, 2.0]);
    assert_close3_f32(vertices[1].position, [1.0, 1.0, 4.0]);
    assert_close3_f32(vertices[2].position, [1.0, 3.0, 4.0]);
    assert_close3_f32(vertices[5].position, [1.0, 3.0, 2.0]);
}

#[test]
fn particle_billboard_vertices_apply_vanilla_roll_transform() {
    let mut instance = test_instance_with_lifetime("minecraft:sculk_charge", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 2.0;
    instance.previous_roll = std::f32::consts::FRAC_PI_2;
    instance.roll = std::f32::consts::FRAC_PI_2;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    assert_eq!(vertices.len(), 6);
    assert_close3_f32(vertices[0].position, [2.0, 1.0, 3.0]);
    assert_close3_f32(vertices[1].position, [2.0, 3.0, 3.0]);
    assert_close3_f32(vertices[2].position, [0.0, 3.0, 3.0]);
    assert_close3_f32(vertices[5].position, [0.0, 1.0, 3.0]);
}

#[test]
fn particle_billboard_vertices_apply_vault_connection_lifetime_alpha() {
    let mut instance = test_instance_with_lifetime("minecraft:vault_connection", 40);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 0.4;
    instance.color = [0.45, 0.45, 0.5, 0.0];
    instance.age_ticks = 20;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    assert_eq!(vertices.len(), 6);
    assert_eq!(vertices[0].color[0], 0.45);
    assert_eq!(vertices[0].color[1], 0.45);
    assert_eq!(vertices[0].color[2], 0.5);
    assert_close_f32(vertices[0].color[3], 0.21);
}

#[test]
fn particle_billboard_vertices_apply_flash_overlay_alpha_and_size() {
    let mut instance = test_instance_with_lifetime("minecraft:flash", 4);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.quad_size_curve = ParticleQuadSizeCurve::FlashOverlay;
    instance.color = [0.1, 0.2, 0.3, 0.4];
    instance.age_ticks = 1;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    let size = 7.1 * (0.5 * 0.25 * std::f32::consts::PI).sin();
    assert_eq!(vertices.len(), 6);
    assert_close3_f32(
        vertices[0].position,
        [1.0 - size / 2.0, 2.0 - size / 2.0, 3.0],
    );
    assert_close_f32(vertices[0].color[0], 0.1);
    assert_close_f32(vertices[0].color[1], 0.2);
    assert_close_f32(vertices[0].color[2], 0.3);
    assert_close_f32(vertices[0].color[3], flash_overlay_alpha(1, 0.5));
}

#[test]
fn particle_billboard_vertices_use_simple_animated_runtime_alpha() {
    let mut instance = test_instance_with_lifetime("minecraft:squid_ink", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 0.4;
    instance.color = [0.0, 0.0, 0.0, 0.95];
    instance.age_ticks = 11;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    assert_eq!(vertices.len(), 6);
    assert_close_f32(vertices[0].color[3], 0.95);
}

#[test]
fn particle_billboard_vertices_skip_instances_without_uploaded_sprite_uv() {
    let mut instance = test_instance_with_lifetime("minecraft:cloud", 20);
    instance.current_sprite_id = Some("minecraft:missing".to_string());

    let vertices = particle_billboard_vertices(
        [&instance],
        &BTreeMap::new(),
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        None,
    );

    assert!(vertices.is_empty());
}

#[test]
fn particle_billboard_vertices_skip_delayed_shriek_instances() {
    let mut instance = test_instance_with_lifetime("minecraft:shriek", 30);
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.delay_ticks = 1;
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    assert!(vertices.is_empty());
}

#[test]
fn particle_billboard_vertices_emit_vanilla_shriek_rotated_quads() {
    let mut instance = test_instance_with_lifetime("minecraft:shriek", 30);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 0.85;
    instance.quad_size_curve = ParticleQuadSizeCurve::Shriek;
    instance.alpha_curve = ParticleAlphaCurve::ShriekFade;
    instance.light = [1.0, 0.4];
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.25, 0.5],
            max: [0.75, 0.875],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    assert_eq!(vertices.len(), 12);
    assert_close3_f32(
        vertices[0].position,
        [0.994_687_5, 1.997_343_8, 3.004_600_8],
    );
    assert_close3_f32(
        vertices[2].position,
        [1.005_312_5, 2.002_656_2, 2.995_399_2],
    );
    assert_close3_f32(
        vertices[6].position,
        [1.005_312_5, 1.997_343_8, 3.004_600_8],
    );
    assert_close3_f32(
        vertices[8].position,
        [0.994_687_5, 2.002_656_2, 2.995_399_2],
    );
    assert_eq!(vertices[0].uv, [0.25, 0.875]);
    assert_eq!(vertices[2].uv, [0.75, 0.5]);
    assert_eq!(vertices[6].light, [1.0, 0.4]);
    assert_eq!(vertices[6].color[0], 1.0);
    assert_eq!(vertices[6].color[1], 1.0);
    assert_eq!(vertices[6].color[2], 1.0);
    assert_close_f32(vertices[6].color[3], 1.0 - 0.5 / 30.0);
}

#[test]
fn particle_billboard_vertices_emit_vanilla_vibration_rotated_quads() {
    let mut instance = test_instance_with_lifetime("minecraft:vibration", 20);
    instance.position = [1.0, 2.0, 3.0];
    instance.current_sprite_id = Some("minecraft:generic_0".to_string());
    instance.base_quad_size = 0.3;
    instance.previous_pitch = -std::f32::consts::FRAC_PI_2;
    instance.pitch = -std::f32::consts::FRAC_PI_2;
    instance.light = [1.0, 0.4];
    let sprite_uvs = BTreeMap::from([(
        "minecraft:generic_0".to_string(),
        ParticleUvRect {
            min: [0.25, 0.5],
            max: [0.75, 0.875],
        },
    )]);

    let vertices = particle_billboard_vertices(
        [&instance],
        &sprite_uvs,
        ParticleBillboardAxes {
            right: Vec3::X,
            up: Vec3::Y,
        },
        Some(ParticlePipelineKind::Translucent),
    );

    let half_size = 0.15;
    let random_sway = vibration_particle_sway(0, DEFAULT_PARTICLE_RENDER_PARTIAL_TICK);
    let first_rotation = Quat::from_rotation_y(random_sway);
    let second_rotation = Quat::from_rotation_y(-std::f32::consts::PI + random_sway);
    let center = Vec3::new(1.0, 2.0, 3.0);
    let first_bottom_left = center + first_rotation * Vec3::new(-half_size, -half_size, 0.0);
    let second_bottom_left = center + second_rotation * Vec3::new(-half_size, -half_size, 0.0);

    assert_eq!(vertices.len(), 12);
    assert_close3_f32(vertices[0].position, first_bottom_left.to_array());
    assert_close3_f32(vertices[6].position, second_bottom_left.to_array());
    assert_eq!(vertices[0].uv, [0.25, 0.875]);
    assert_eq!(vertices[6].light, [1.0, 0.4]);
    assert_eq!(vertices[6].color, [1.0, 1.0, 1.0, 1.0]);
}

fn test_instance_with_lifetime(particle_id: &str, lifetime_ticks: u32) -> ParticleInstance {
    let descriptor = ParticleDescriptor::for_particle(particle_id);
    let [collision_width, collision_height] = descriptor.collision_size().unwrap_or([
        DEFAULT_PARTICLE_COLLISION_WIDTH,
        DEFAULT_PARTICLE_COLLISION_HEIGHT,
    ]);
    ParticleInstance {
        particle_type_id: 0,
        particle_id: particle_id.to_string(),
        sprite_ids: Vec::new(),
        current_sprite_id: None,
        current_sprite_index: None,
        start_position: [0.0, 0.0, 0.0],
        previous_position: [0.0, 0.0, 0.0],
        position: [0.0, 0.0, 0.0],
        velocity: [0.0, 0.0, 0.0],
        age_ticks: 0,
        lifetime_ticks,
        previous_roll: 0.0,
        roll: 0.0,
        roll_speed: 0.0,
        previous_yaw: 0.0,
        yaw: 0.0,
        previous_pitch: 0.0,
        pitch: 0.0,
        base_quad_size: DEFAULT_PARTICLE_QUAD_SIZE,
        color: [1.0, 1.0, 1.0, 1.0],
        original_alpha: 1.0,
        color_fade_target: descriptor.color_fade_target(),
        color_transition_target: None,
        light: DEFAULT_PARTICLE_LIGHT,
        light_emission: descriptor.light_emission(),
        alpha_curve: descriptor.alpha_curve(),
        quad_size_curve: ParticleQuadSizeCurve::Constant,
        provider: descriptor.provider.to_string(),
        render_group: particle_render_group_for_particle(particle_id),
        render_layer: particle_render_layer_for_particle(particle_id),
        texture_atlas: particle_render_layer_for_particle(particle_id).texture_atlas_kind(),
        facing_camera_mode: descriptor.facing_camera_mode(),
        friction: descriptor.friction,
        gravity: descriptor.gravity,
        has_physics: descriptor.has_physics,
        moves_without_collision: descriptor.moves_without_collision(),
        speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
        collision_width,
        collision_height,
        on_ground: false,
        hit_ground: false,
        stopped_by_collision: false,
        removed: false,
        tick_motion: descriptor.tick_motion(),
        drip_fluid: descriptor.drip_fluid(),
        required_fluid: descriptor.required_fluid(),
        air_downward_acceleration: descriptor.air_downward_acceleration(),
        tick_angle: 0.0,
        particle_limit: particle_limit_for_particle(particle_id),
        child_emission: descriptor.child_emission(),
        child_spawn_templates: Vec::new(),
        falling_leaves_motion: None,
        sprite_selection: descriptor.sprite_selection,
        override_limiter: false,
        always_show: false,
        raw_options_len: 0,
        delay_ticks: 0,
        option_color: None,
        option_color_to: None,
        option_scale: None,
        option_power: None,
        option_target: None,
        option_entity_target_source: None,
        option_duration_ticks: None,
        option_roll: None,
        option_block: None,
        option_item: None,
        option_item_pickup_source_entity_id: None,
        option_item_pickup_age_ticks: None,
        option_item_pickup_light: None,
        option_item_pickup_experience_orb_icon: None,
        option_item_pickup_component_patch: None,
        option_item_pickup_projectile_model: None,
        firework_trail: false,
        firework_twinkle: false,
        item_pickup_previous_target: None,
        item_pickup_target: None,
        atlas_uv_sub_rect: None,
    }
}

fn falling_leaves_instance(particle_id: &str, seed: i64) -> ParticleInstance {
    let mut random = ParticleRandom::new(seed);
    ParticleInstance::from_spawn_command(spawn_command(particle_id, 1.0), &mut random)
}

fn spawn_command(particle_id: &str, x: f64) -> ParticleSpawnCommand {
    ParticleSpawnCommand {
        particle_type_id: 4,
        particle_id: particle_id.to_string(),
        sprite_ids: vec!["minecraft:generic_0".to_string()],
        position: [x, 0.0, 0.0],
        velocity: [0.0, 0.0, 0.0],
        override_limiter: false,
        always_show: false,
        raw_options_len: 0,
        initial_delay_ticks: 0,
        child_spawn_templates: Vec::new(),
        option_color: None,
        option_color_to: None,
        option_scale: None,
        option_power: None,
        option_target: None,
        option_entity_target_source: None,
        option_duration_ticks: None,
        option_roll: None,
        option_block: None,
        option_item: None,
        option_item_pickup_source_entity_id: None,
        option_item_pickup_age_ticks: None,
        option_item_pickup_light: None,
        option_item_pickup_experience_orb_icon: None,
        option_item_pickup_component_patch: None,
        option_item_pickup_projectile_model: None,
        option_firework_trail: false,
        option_firework_twinkle: false,
        option_firework_half_lifetime_age: false,
    }
}

fn item_pickup_spawn_command() -> ParticleSpawnCommand {
    let mut command = spawn_command(ITEM_PICKUP_PARTICLE_ID, 1.0);
    command.particle_type_id = -1;
    command.sprite_ids.clear();
    command.position = [1.0, 64.0, -2.0];
    command.velocity = [0.1, 0.2, 0.3];
    command.override_limiter = true;
    command.option_target = Some([4.0, 70.8, 8.0]);
    command.option_entity_target_source = Some(ParticleEntityTargetSource {
        entity_id: 20,
        y_offset: 0.8,
    });
    command.option_item = Some(ParticleItemOptionState {
        item_id: 42,
        count: 5,
        component_patch_len: 0,
    });
    command.option_item_pickup_source_entity_id = Some(10);
    command.option_item_pickup_age_ticks = Some(12.0);
    command.option_item_pickup_light = Some([0.4, 0.8]);
    command
}

fn water_fluid_surface_sample() -> ParticleBlockFluidSurfaceSample {
    ParticleBlockFluidSurfaceSample {
        block_collision_height: 0.0,
        fluid_height: 8.0 / 9.0,
        fluid_kind: Some(ParticleFluidKind::Water),
        block_is_air: false,
    }
}

fn assert_close_f32(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn assert_close_f64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn assert_mat4_close(actual: Mat4, expected: Mat4) {
    let actual = actual.to_cols_array();
    let expected = expected.to_cols_array();
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "matrix[{index}] expected {expected}, got {actual}"
        );
    }
}

fn assert_range_f32(actual: f32, min: f32, max: f32) {
    assert!(
        actual >= min && actual <= max,
        "expected {actual} to be in {min}..={max}"
    );
}

fn assert_range_f64(actual: f64, min: f64, max: f64) {
    assert!(
        actual >= min && actual <= max,
        "expected {actual} to be in {min}..={max}"
    );
}

fn assert_atlas_sub_rect(instance: &ParticleInstance) {
    let sub_rect = instance
        .atlas_uv_sub_rect
        .expect("terrain/item atlas particle should record a 4x4 sub-rect offset");
    assert_range_f32(sub_rect.u_offset, 0.0, 3.0);
    assert_range_f32(sub_rect.v_offset, 0.0, 3.0);
}

fn assert_close3(actual: [f64; 3], expected: [f64; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }
}

fn assert_close3_f32(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }
}
