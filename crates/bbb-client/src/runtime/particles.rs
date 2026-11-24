use super::*;

/// The resolver and engine pair spawn sources write to; `None` without
/// particle resources.
pub(super) fn particle_target<'a>(
    particle_runtime: &'a mut Option<NativeParticleRuntime>,
    engine: &'a mut ParticleEngine,
) -> Option<ParticleTarget<'a>> {
    particle_runtime
        .as_mut()
        .map(|runtime| ParticleTarget { runtime, engine })
}

pub(super) fn emit_particle_sounds(
    audio_events: &mut Option<&mut dyn AudioEventSink>,
    engine: &mut ParticleEngine,
) {
    let sounds = engine.drain_sounds();
    let Some(audio_events) = audio_events.as_deref_mut() else {
        sounds.for_each(drop);
        return;
    };
    for sound in sounds {
        if let Some(state) = particle_sound_event_state(sound) {
            audio_events.play_positioned_sound(&state);
        }
    }
}

pub(super) fn submit_local_player_action_particles(
    world: &mut WorldStore,
    input: &mut ClientInputState,
    particles: Option<ParticleTarget<'_>>,
    item_runtime: Option<&NativeItemRuntime>,
) {
    let actions = input
        .take_pending_local_player_action_particles()
        .into_iter()
        .chain(
            world
                .take_pending_local_interaction_particle_actions()
                .into_iter()
                .map(LocalPlayerActionParticleState::Interaction),
        );
    let Some(ParticleTarget { runtime, engine }) = particles else {
        return;
    };
    for state in actions {
        match state {
            LocalPlayerActionParticleState::BlockAttack(effect) => {
                runtime.spawn_local_block_attack_particles(engine, world, effect)
            }
            LocalPlayerActionParticleState::DestroyHit(state) => {
                runtime.spawn_local_destroy_hit_particles(engine, *state)
            }
            LocalPlayerActionParticleState::Interaction(action) => {
                runtime.spawn_local_interaction_particles(engine, world, action, item_runtime)
            }
        }
    }
}

/// Vanilla `Entity.tick`: one `spawnSprintParticle` per client tick while
/// `canSpawnSprintParticle` holds for the local player.
pub(super) fn submit_local_player_sprint_particles(
    world: &WorldStore,
    particles: Option<ParticleTarget<'_>>,
    ticks: u32,
) {
    let Some(ParticleTarget { runtime, engine }) = particles else {
        return;
    };
    let Some(state) = world.local_player_sprint_particle_state() else {
        return;
    };
    for _ in 0..ticks {
        runtime.spawn_local_sprint_particles(engine, state);
    }
}

/// `PrimedTnt.tick`: one smoke particle half a block above each fuse per tick.
pub(super) fn submit_primed_tnt_smoke_particles(
    engine: &mut ParticleEngine,
    world: &WorldStore,
    ticks: u32,
) {
    if ticks == 0 {
        return;
    }
    let states = world.primed_tnt_smoke_particle_states();
    for _ in 0..ticks {
        for state in &states {
            engine.spawn(ParticleSpawn::new(
                ParticleKind::SMOKE,
                [state.position.x, state.position.y + 0.5, state.position.z],
                [0.0; 3],
            ));
        }
    }
}

pub(super) fn submit_entity_client_tick_effects(
    engine: &mut ParticleEngine,
    world: &mut WorldStore,
    mut particle_runtime: Option<&mut NativeParticleRuntime>,
    audio_events: &mut Option<&mut dyn AudioEventSink>,
) {
    for state in world.take_ravager_stun_particle_states() {
        engine.spawn(
            ParticleSpawn::new(
                ParticleKind::ENTITY_EFFECT,
                [state.position.x, state.position.y, state.position.z],
                [0.0; 3],
            )
            .with_params(ParticleParams::Color([
                0.498_039_22,
                0.513_725_5,
                0.572_549_05,
                1.0,
            ])),
        );
    }
    for state in world.take_evoker_fangs_crit_particle_states() {
        engine.spawn(ParticleSpawn::new(
            ParticleKind::CRIT,
            [state.position.x, state.position.y, state.position.z],
            [state.velocity.x, state.velocity.y, state.velocity.z],
        ));
    }

    let mut effects = world.take_entity_client_movement_effect_states();
    for effect in effects.drain(..) {
        match effect {
            EntityClientMovementEffectState::Particle(state) => {
                if let Some(runtime) = particle_runtime.as_deref_mut() {
                    runtime.spawn_entity_client_movement_particles(engine, state);
                }
            }
            EntityClientMovementEffectState::Sound(state) => {
                let sound = world.record_entity_client_movement_sound(state);
                if let Some(audio_events) = audio_events.as_deref_mut() {
                    audio_events.play_positioned_sound(&sound);
                }
            }
        }
    }
    world.recycle_entity_client_movement_effect_states(effects);
}

pub(super) fn submit_ominous_item_spawner_particles(
    world: &mut WorldStore,
    particles: Option<ParticleTarget<'_>>,
) {
    let states = world.take_ominous_item_spawner_particle_states();
    let Some(ParticleTarget { runtime, engine }) = particles else {
        return;
    };
    for state in states {
        runtime.spawn_ominous_item_spawner_particles(engine, state);
    }
}

fn particle_sound_event_state(sound: ParticleSound) -> Option<SoundEventState> {
    let location = bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(sound.sound)?;
    Some(SoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(location.to_string()),
            fixed_range: None,
        },
        source: sound.source.as_str().to_string(),
        position: Vec3d {
            x: sound.position[0],
            y: sound.position[1],
            z: sound.position[2],
        },
        volume: sound.volume,
        pitch: sound.pitch,
        seed: sound.seed,
        distance_delay: sound.distance_delay,
    })
}

pub(super) fn particle_light_block_pos(position: [f64; 3]) -> BlockPos {
    BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    }
}

pub(super) fn particle_local_player_scope_context(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    first_person: bool,
) -> Option<ParticleLocalPlayerScopeContext> {
    let scoping = world.local_player_is_scoping();
    if !scoping {
        return None;
    }
    let camera = camera_pose?;
    let eye_position = camera_eye_position(camera).map(f64::from);
    Some(ParticleLocalPlayerScopeContext {
        eye_position,
        first_person,
        scoping,
    })
}

/// Candidate players for the PlayerCloud / Sneeze per-particle
/// `level.getNearestPlayer(x, y, z, 2.0, false)` resolution
/// the local player plus every remote player
/// entity, minus spectators (`EntitySelector.NO_SPECTATORS` — creative
/// players stay in, EntityGetter). The nearest-candidate pick
/// itself happens in the particle tick, which knows each particle's position.
pub(super) fn particle_player_motion_contexts(
    world: &WorldStore,
) -> Vec<ParticlePlayerMotionContext> {
    let mut contexts = Vec::new();
    if !world.local_player_is_spectator() {
        if let Some(pose) = world.local_player_pose() {
            contexts.push(ParticlePlayerMotionContext {
                position: [pose.position.x, pose.position.y, pose.position.z],
                delta_movement: [
                    pose.delta_movement.x,
                    pose.delta_movement.y,
                    pose.delta_movement.z,
                ],
            });
        }
    }
    for transform in world.player_entity_transforms() {
        if world
            .player_info_entry(transform.uuid)
            .is_some_and(|info| info.is_spectator())
        {
            continue;
        }
        contexts.push(ParticlePlayerMotionContext {
            position: [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            delta_movement: [
                transform.delta_movement.x,
                transform.delta_movement.y,
                transform.delta_movement.z,
            ],
        });
    }
    contexts
}
