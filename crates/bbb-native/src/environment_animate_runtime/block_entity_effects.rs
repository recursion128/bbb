//! Vanilla client-ticked block-entity particles and local sounds.
//!
//! These effects share `EnvironmentAnimateRuntime::level_random` with
//! `ClientLevel.animateTick`; keeping the algorithms here makes the random
//! cursor and the world-to-backend boundary explicit without growing the
//! environment animate facade further.

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::packets::SoundSource;
use bbb_world::{
    BlockEntityClientEffectSource, BlockPos, CampfireClientEffectSource, ClientSoundEventHolder,
    ConduitClientEffectSource, LevelEventSoundRandomState, LocalSoundEventState,
    TrialSpawnerClientEffectSource, TrialSpawnerClientState, VaultClientEffectSource, WorldStore,
};

use super::{EnvironmentAnimateRuntime, EnvironmentWorldParticleBiomeSampler};
use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        simple_client_particle_options, EnvironmentParticleSpawn, LevelParticleSpawnContext,
        ParticleEventSink,
    },
    simulation::effects::ParticleBatchSink,
};

const PARTICLE_CAMPFIRE_COSY_SMOKE: &str = "minecraft:campfire_cosy_smoke";
const PARTICLE_CAMPFIRE_SIGNAL_SMOKE: &str = "minecraft:campfire_signal_smoke";
const PARTICLE_FLAME: &str = "minecraft:flame";
const PARTICLE_NAUTILUS: &str = "minecraft:nautilus";
const PARTICLE_SMALL_FLAME: &str = "minecraft:small_flame";
const PARTICLE_SMOKE: &str = "minecraft:smoke";
const PARTICLE_SOUL_FIRE_FLAME: &str = "minecraft:soul_fire_flame";
const PARTICLE_VAULT_CONNECTION: &str = "minecraft:vault_connection";

const SOUND_TRIAL_SPAWNER_AMBIENT: &str = "minecraft:block.trial_spawner.ambient";
const SOUND_TRIAL_SPAWNER_AMBIENT_OMINOUS: &str = "minecraft:block.trial_spawner.ambient_ominous";
const SOUND_VAULT_AMBIENT: &str = "minecraft:block.vault.ambient";

#[derive(Default)]
struct BlockEntityClientEffects {
    particles: Vec<EnvironmentParticleSpawn>,
    local_sounds: Vec<LocalSoundEventState>,
}

impl EnvironmentAnimateRuntime {
    pub(crate) fn advance_block_entity_effects_for_ticks(
        &mut self,
        world: &WorldStore,
        connection_generation: u64,
        running_level_ticks: u32,
        mut particle_events: Option<&mut dyn ParticleEventSink>,
        particle_batches: &mut dyn ParticleBatchSink,
        audio: &mut Option<&mut dyn AudioEventSink>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        self.synchronize_level_identity(world, connection_generation);
        if running_level_ticks == 0 {
            return;
        }

        let end_game_time = world.world_time().map_or(0, |time| time.game_time);
        let start_game_time = end_game_time
            .saturating_sub(i64::from(running_level_ticks))
            .saturating_add(1);
        for tick_index in 0..running_level_ticks {
            let game_time = start_game_time.saturating_add(i64::from(tick_index));
            let conduit_ticks_after_this = running_level_ticks
                .saturating_sub(tick_index)
                .saturating_sub(1);
            let mut effects = BlockEntityClientEffects::default();
            for source in world.block_entity_client_effect_sources() {
                append_source_effects(
                    world,
                    &source,
                    game_time,
                    conduit_ticks_after_this,
                    &mut self.level_random,
                    &mut effects,
                );
            }

            if let Some(audio) = audio.as_deref_mut() {
                for sound in &effects.local_sounds {
                    audio.play_local_sound(sound);
                }
            }
            if effects.particles.is_empty() {
                continue;
            }
            let Some(particle_events) = particle_events.as_deref_mut() else {
                continue;
            };
            let camera_position = world.local_player_pose().map(|player| {
                [
                    player.position.x,
                    player.position.y + player.eye_height(),
                    player.position.z,
                ]
            });
            let biome_sampler = EnvironmentWorldParticleBiomeSampler { world };
            let batch = particle_events.spawn_environment_particles(
                &effects.particles,
                LevelParticleSpawnContext {
                    camera_position,
                    vibration_entity_position: None,
                },
                Some(&biome_sampler),
                item_runtime,
            );
            if !batch.is_empty() {
                particle_batches.submit_particle_batch(batch);
            }
        }
    }
}

fn append_source_effects(
    world: &WorldStore,
    source: &BlockEntityClientEffectSource,
    game_time: i64,
    conduit_ticks_after_this: u32,
    random: &mut LevelEventSoundRandomState,
    effects: &mut BlockEntityClientEffects,
) {
    match source {
        BlockEntityClientEffectSource::Campfire(source) => {
            append_campfire_effects(*source, random, &mut effects.particles);
        }
        BlockEntityClientEffectSource::Spawner { pos } => {
            append_spawner_effects(*pos, random, &mut effects.particles);
        }
        BlockEntityClientEffectSource::TrialSpawner(source) => {
            append_trial_spawner_effects(world, *source, game_time, random, effects);
        }
        BlockEntityClientEffectSource::Conduit(source) => {
            append_conduit_effects(
                source,
                conduit_ticks_after_this,
                random,
                &mut effects.particles,
            );
        }
        BlockEntityClientEffectSource::Vault(source) => {
            append_vault_effects(world, source, game_time, random, effects);
        }
    }
}

fn append_campfire_effects(
    source: CampfireClientEffectSource,
    random: &mut LevelEventSoundRandomState,
    particles: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_float() < 0.11 {
        let smoke_count = random.next_int_bound(2) + 2;
        for _ in 0..smoke_count {
            let x_offset = random.next_double() / 3.0;
            let x_sign = if random.next_boolean() { 1.0 } else { -1.0 };
            let x = f64::from(source.pos.x) + 0.5 + x_offset * x_sign;
            let y = f64::from(source.pos.y) + random.next_double() + random.next_double();
            let z_offset = random.next_double() / 3.0;
            let z_sign = if random.next_boolean() { 1.0 } else { -1.0 };
            let z = f64::from(source.pos.z) + 0.5 + z_offset * z_sign;
            push_particle(
                particles,
                if source.signal_fire {
                    PARTICLE_CAMPFIRE_SIGNAL_SMOKE
                } else {
                    PARTICLE_CAMPFIRE_COSY_SMOKE
                },
                [x, y, z],
                [0.0, 0.07, 0.0],
                true,
            );
        }
    }

    for (slot, occupied) in source.occupied_slots.into_iter().enumerate() {
        if !occupied || random.next_float() >= 0.2 {
            continue;
        }
        let direction = ((slot + usize::from(source.facing_2d)) % 4) as usize;
        let step = DIRECTION_STEPS[direction];
        let clockwise = DIRECTION_STEPS[(direction + 1) % 4];
        let position = [
            f64::from(source.pos.x) + 0.5 - f64::from(step[0]) * 0.3125
                + f64::from(clockwise[0]) * 0.3125,
            f64::from(source.pos.y) + 0.5,
            f64::from(source.pos.z) + 0.5 - f64::from(step[1]) * 0.3125
                + f64::from(clockwise[1]) * 0.3125,
        ];
        for _ in 0..4 {
            push_particle(
                particles,
                PARTICLE_SMOKE,
                position,
                [0.0, 0.0005, 0.0],
                false,
            );
        }
    }
}

const DIRECTION_STEPS: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];

fn append_spawner_effects(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    particles: &mut Vec<EnvironmentParticleSpawn>,
) {
    let position = [
        f64::from(pos.x) + random.next_double(),
        f64::from(pos.y) + random.next_double(),
        f64::from(pos.z) + random.next_double(),
    ];
    for particle in [PARTICLE_SMOKE, PARTICLE_FLAME] {
        push_particle(particles, particle, position, [0.0; 3], false);
    }
}

fn append_trial_spawner_effects(
    world: &WorldStore,
    source: TrialSpawnerClientEffectSource,
    game_time: i64,
    random: &mut LevelEventSoundRandomState,
    effects: &mut BlockEntityClientEffects,
) {
    let center = block_center(source.pos);
    match source.state {
        TrialSpawnerClientState::Inactive => {}
        TrialSpawnerClientState::WaitingForPlayers
        | TrialSpawnerClientState::WaitingForRewardEjection
        | TrialSpawnerClientState::EjectingReward => {
            if random.next_int_bound(2) == 0 {
                let position = offset_random(center, random, 0.9);
                push_particle(
                    &mut effects.particles,
                    flame_particle(source.ominous, true),
                    position,
                    [0.0; 3],
                    false,
                );
            }
        }
        TrialSpawnerClientState::Active => {
            let position = offset_random(center, random, 1.0);
            push_particle(
                &mut effects.particles,
                PARTICLE_SMOKE,
                position,
                [0.0; 3],
                false,
            );
            push_particle(
                &mut effects.particles,
                flame_particle(source.ominous, false),
                position,
                [0.0; 3],
                false,
            );
        }
        TrialSpawnerClientState::Cooldown => {
            let position = offset_random(center, random, 0.9);
            if random.next_int_bound(3) == 0 {
                push_particle(
                    &mut effects.particles,
                    PARTICLE_SMOKE,
                    position,
                    [0.0; 3],
                    false,
                );
            }
            if game_time.rem_euclid(20) == 0 {
                let top_position = [center[0], center[1] + 0.5, center[2]];
                let smoke_count = random.next_int_bound(4) + 20;
                for _ in 0..smoke_count {
                    push_particle(
                        &mut effects.particles,
                        PARTICLE_SMOKE,
                        top_position,
                        [0.0; 3],
                        false,
                    );
                }
            }
        }
    }

    if source.state.is_capable_of_spawning() && random.next_float() <= 0.02 {
        let sound = if source.ominous {
            SOUND_TRIAL_SPAWNER_AMBIENT_OMINOUS
        } else {
            SOUND_TRIAL_SPAWNER_AMBIENT
        };
        effects
            .local_sounds
            .push(randomized_block_sound(world, sound, random));
    }
}

fn append_conduit_effects(
    source: &ConduitClientEffectSource,
    ticks_after_this: u32,
    random: &mut LevelEventSoundRandomState,
    particles: &mut Vec<EnvironmentParticleSpawn>,
) {
    let tick_count = source
        .tick_count
        .saturating_sub(i32::try_from(ticks_after_this).unwrap_or(i32::MAX));
    let phase = (tick_count.saturating_add(35) as f32) * 0.1;
    let mut height = vanilla_mth_sin(phase) / 2.0 + 0.5;
    height = (height * height + height) * 0.3;
    let particle_end = [
        f64::from(source.pos.x) + 0.5,
        f64::from(source.pos.y) + 1.5 + f64::from(height),
        f64::from(source.pos.z) + 0.5,
    ];
    for frame_pos in &source.effect_blocks {
        if random.next_int_bound(50) != 0 {
            continue;
        }
        let velocity = [
            f64::from(
                -0.5_f32 + random.next_float() + (frame_pos.x.saturating_sub(source.pos.x) as f32),
            ),
            f64::from(
                -2.0_f32 + random.next_float() + (frame_pos.y.saturating_sub(source.pos.y) as f32),
            ),
            f64::from(
                -0.5_f32 + random.next_float() + (frame_pos.z.saturating_sub(source.pos.z) as f32),
            ),
        ];
        push_particle(particles, PARTICLE_NAUTILUS, particle_end, velocity, false);
    }
    if let Some(target) = source.destroy_target {
        let velocity = [
            f64::from((-0.5 + random.next_float()) * (3.0 + target.width)),
            f64::from(-1.0 + random.next_float() * target.height),
            f64::from((-0.5 + random.next_float()) * (3.0 + target.width)),
        ];
        push_particle(
            particles,
            PARTICLE_NAUTILUS,
            [target.position[0], target.eye_y, target.position[2]],
            velocity,
            false,
        );
    }
}

fn append_vault_effects(
    world: &WorldStore,
    source: &VaultClientEffectSource,
    game_time: i64,
    random: &mut LevelEventSoundRandomState,
    effects: &mut BlockEntityClientEffects,
) {
    if game_time.rem_euclid(20) == 0 {
        if let Some(connection) = &source.connection_particles {
            for target in &connection.targets {
                let direction = [
                    target.target_position[0] - connection.origin[0],
                    target.target_position[1] - connection.origin[1],
                    target.target_position[2] - connection.origin[2],
                ];
                let particle_count = random.next_int_bound(4) + 2;
                for _ in 0..particle_count {
                    let velocity = offset_random(direction, random, 1.0);
                    push_particle(
                        &mut effects.particles,
                        PARTICLE_VAULT_CONNECTION,
                        connection.origin,
                        velocity,
                        false,
                    );
                }
            }
        }
    }

    if random.next_float() <= 0.5 {
        let position = [
            f64::from(source.pos.x) + random_between(random, 0.1, 0.9),
            f64::from(source.pos.y) + random_between(random, 0.25, 0.75),
            f64::from(source.pos.z) + random_between(random, 0.1, 0.9),
        ];
        push_particle(
            &mut effects.particles,
            PARTICLE_SMOKE,
            position,
            [0.0; 3],
            false,
        );
        if source.has_display_item {
            push_particle(
                &mut effects.particles,
                flame_particle(source.ominous, true),
                position,
                [0.0; 3],
                false,
            );
        }
    }
    if source.has_display_item && random.next_float() <= 0.02 {
        effects
            .local_sounds
            .push(randomized_block_sound(world, SOUND_VAULT_AMBIENT, random));
    }
}

fn randomized_block_sound(
    world: &WorldStore,
    sound_id: &str,
    random: &mut LevelEventSoundRandomState,
) -> LocalSoundEventState {
    let volume = random.next_float() * 0.25 + 0.75;
    let pitch = random.next_float() + 0.5;
    let seed = random.next_long();
    LocalSoundEventState {
        sound: sound_holder_state(
            world,
            &ClientSoundEventHolder::Reference {
                resource_id: sound_id.to_string(),
            },
        ),
        source: SoundSource::Blocks.as_str().to_string(),
        volume,
        pitch,
        seed,
    }
}

fn block_center(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ]
}

fn offset_random(
    position: [f64; 3],
    random: &mut LevelEventSoundRandomState,
    offset: f32,
) -> [f64; 3] {
    [
        position[0] + f64::from((random.next_float() - 0.5) * offset),
        position[1] + f64::from((random.next_float() - 0.5) * offset),
        position[2] + f64::from((random.next_float() - 0.5) * offset),
    ]
}

fn random_between(random: &mut LevelEventSoundRandomState, min: f64, max: f64) -> f64 {
    min + random.next_double() * (max - min)
}

fn vanilla_mth_sin(angle: f32) -> f32 {
    let index = ((angle * 10_430.378_f32) as i32) & 65_535;
    (f64::from(index) * std::f64::consts::TAU / 65_536.0).sin() as f32
}

fn flame_particle(ominous: bool, small: bool) -> &'static str {
    if ominous {
        PARTICLE_SOUL_FIRE_FLAME
    } else if small {
        PARTICLE_SMALL_FLAME
    } else {
        PARTICLE_FLAME
    }
}

fn push_particle(
    particles: &mut Vec<EnvironmentParticleSpawn>,
    particle: &str,
    position: [f64; 3],
    velocity: [f64; 3],
    always_show: bool,
) {
    particles.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(particle),
        position,
        velocity,
        always_show,
        color_source: None,
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::BlockUpdate;
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, ConduitDestroyTargetState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, VaultConnectionParticleState,
        VaultConnectionParticleTargetState, WorldDimension,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn block_entity_client_effect_campfire_and_spawner_follow_java_random_order() {
        let mut random = LevelEventSoundRandomState::with_seed(4096);
        let mut expected_random = random;
        let mut particles = Vec::new();
        let source = CampfireClientEffectSource {
            pos: BlockPos { x: 2, y: 3, z: 4 },
            facing_2d: 2,
            signal_fire: true,
            occupied_slots: [true; 4],
        };

        append_campfire_effects(source, &mut random, &mut particles);

        assert!(expected_random.next_float() < 0.11);
        let smoke_count = expected_random.next_int_bound(2) + 2;
        for particle in particles.iter().take(smoke_count as usize) {
            let x_offset = expected_random.next_double() / 3.0;
            let x_sign = if expected_random.next_boolean() {
                1.0
            } else {
                -1.0
            };
            let y = f64::from(source.pos.y)
                + expected_random.next_double()
                + expected_random.next_double();
            let z_offset = expected_random.next_double() / 3.0;
            let z_sign = if expected_random.next_boolean() {
                1.0
            } else {
                -1.0
            };
            assert_eq!(
                particle.particle.resource_id,
                PARTICLE_CAMPFIRE_SIGNAL_SMOKE
            );
            assert_eq!(
                particle.position,
                [
                    f64::from(source.pos.x) + 0.5 + x_offset * x_sign,
                    y,
                    f64::from(source.pos.z) + 0.5 + z_offset * z_sign,
                ]
            );
            assert_eq!(particle.velocity, [0.0, 0.07, 0.0]);
            assert!(particle.always_show);
        }
        let mut occupied_slot_smoke_count = 0;
        for _ in source.occupied_slots {
            if expected_random.next_float() < 0.2 {
                occupied_slot_smoke_count += 4;
            }
        }
        assert_eq!(
            particles.len(),
            smoke_count as usize + occupied_slot_smoke_count
        );
        assert_eq!(random, expected_random);

        let spawner_start = particles.len();
        append_spawner_effects(
            BlockPos {
                x: -7,
                y: 11,
                z: 13,
            },
            &mut random,
            &mut particles,
        );
        let expected_position = [
            -7.0 + expected_random.next_double(),
            11.0 + expected_random.next_double(),
            13.0 + expected_random.next_double(),
        ];
        assert_eq!(particles[spawner_start].position, expected_position);
        assert_eq!(particles[spawner_start + 1].position, expected_position);
        assert_eq!(
            particles[spawner_start].particle.resource_id,
            PARTICLE_SMOKE
        );
        assert_eq!(
            particles[spawner_start + 1].particle.resource_id,
            PARTICLE_FLAME
        );
        assert_eq!(random, expected_random);
    }

    #[test]
    fn block_entity_client_effect_trial_spawner_preserves_particle_sound_random_order() {
        let world = WorldStore::new();
        let source = TrialSpawnerClientEffectSource {
            pos: BlockPos { x: 3, y: 5, z: 7 },
            state: TrialSpawnerClientState::Active,
            ominous: true,
        };
        let mut random = LevelEventSoundRandomState::with_seed(2);
        let mut expected_random = random;
        let mut effects = BlockEntityClientEffects::default();

        append_trial_spawner_effects(&world, source, 1, &mut random, &mut effects);

        let expected_position = offset_random(block_center(source.pos), &mut expected_random, 1.0);
        assert!(expected_random.next_float() <= 0.02);
        let expected_volume = expected_random.next_float() * 0.25 + 0.75;
        let expected_pitch = expected_random.next_float() + 0.5;
        let expected_seed = expected_random.next_long();
        assert_eq!(effects.particles.len(), 2);
        assert_eq!(effects.particles[0].particle.resource_id, PARTICLE_SMOKE);
        assert_eq!(
            effects.particles[1].particle.resource_id,
            PARTICLE_SOUL_FIRE_FLAME
        );
        assert_eq!(effects.particles[0].position, expected_position);
        assert_eq!(effects.particles[1].position, expected_position);
        assert_eq!(effects.local_sounds.len(), 1);
        assert_eq!(
            effects.local_sounds[0].sound,
            sound_holder_state(
                &world,
                &ClientSoundEventHolder::Reference {
                    resource_id: SOUND_TRIAL_SPAWNER_AMBIENT_OMINOUS.to_string(),
                }
            )
        );
        assert_eq!(effects.local_sounds[0].source, "block");
        assert_eq!(effects.local_sounds[0].volume, expected_volume);
        assert_eq!(effects.local_sounds[0].pitch, expected_pitch);
        assert_eq!(effects.local_sounds[0].seed, expected_seed);
        assert_eq!(random, expected_random);
    }

    #[test]
    fn block_entity_client_effect_conduit_and_vault_keep_every_conditional_draw() {
        let conduit = ConduitClientEffectSource {
            pos: BlockPos { x: 4, y: 8, z: 12 },
            tick_count: 19,
            effect_blocks: vec![BlockPos { x: 6, y: 9, z: 11 }],
            destroy_target: Some(ConduitDestroyTargetState {
                entity_id: 17,
                position: [20.0, 21.0, 22.0],
                eye_y: 22.625,
                width: 0.6,
                height: 1.95,
            }),
        };
        let mut random = LevelEventSoundRandomState::with_seed(18);
        let mut expected_random = random;
        let mut particles = Vec::new();

        append_conduit_effects(&conduit, 0, &mut random, &mut particles);

        assert_eq!(expected_random.next_int_bound(50), 0);
        let expected_frame_velocity = [
            f64::from(-0.5 + expected_random.next_float() + 2.0),
            f64::from(-2.0 + expected_random.next_float() + 1.0),
            f64::from(-0.5 + expected_random.next_float() - 1.0),
        ];
        let target = conduit.destroy_target.unwrap();
        let expected_target_velocity = [
            f64::from((-0.5 + expected_random.next_float()) * (3.0 + target.width)),
            f64::from(-1.0 + expected_random.next_float() * target.height),
            f64::from((-0.5 + expected_random.next_float()) * (3.0 + target.width)),
        ];
        assert_eq!(particles.len(), 2);
        assert_eq!(particles[0].particle.resource_id, PARTICLE_NAUTILUS);
        assert_eq!(particles[0].velocity, expected_frame_velocity);
        assert_eq!(
            particles[1].position,
            [target.position[0], target.eye_y, target.position[2]]
        );
        assert_eq!(particles[1].velocity, expected_target_velocity);
        assert_eq!(random, expected_random);

        let world = WorldStore::new();
        let vault = VaultClientEffectSource {
            pos: BlockPos { x: 1, y: 2, z: 3 },
            ominous: false,
            has_display_item: false,
            previous_spin: 0.0,
            current_spin: 10.0,
            connection_particles: Some(VaultConnectionParticleState {
                origin: [1.0, 2.0, 3.0],
                targets: vec![VaultConnectionParticleTargetState {
                    entity_id: 4,
                    uuid: Uuid::nil(),
                    target_position: [5.0, 7.0, 11.0],
                }],
            }),
        };
        let mut effects = BlockEntityClientEffects::default();
        let vault_start = particles.len();
        append_vault_effects(&world, &vault, 20, &mut random, &mut effects);

        let connection_count = expected_random.next_int_bound(4) + 2;
        let direction = [4.0, 5.0, 8.0];
        for particle in effects.particles.iter().take(connection_count as usize) {
            assert_eq!(particle.particle.resource_id, PARTICLE_VAULT_CONNECTION);
            assert_eq!(particle.position, [1.0, 2.0, 3.0]);
            assert_eq!(
                particle.velocity,
                offset_random(direction, &mut expected_random, 1.0)
            );
        }
        let idle_particle = expected_random.next_float() <= 0.5;
        if idle_particle {
            let expected_position = [
                1.0 + random_between(&mut expected_random, 0.1, 0.9),
                2.0 + random_between(&mut expected_random, 0.25, 0.75),
                3.0 + random_between(&mut expected_random, 0.1, 0.9),
            ];
            assert_eq!(
                effects.particles[connection_count as usize]
                    .particle
                    .resource_id,
                PARTICLE_SMOKE
            );
            assert_eq!(
                effects.particles[connection_count as usize].position,
                expected_position
            );
        }
        assert_eq!(
            effects.particles.len(),
            connection_count as usize + usize::from(idle_particle)
        );
        assert!(effects.local_sounds.is_empty());
        assert_eq!(random, expected_random);
        assert_eq!(particles.len(), vault_start);
    }

    #[test]
    fn block_entity_client_effect_missing_sinks_still_consume_level_random() {
        let world = world_with_active_trial_spawner();
        let sources = world.block_entity_client_effect_sources();
        assert_eq!(sources.len(), 1);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_effects = BlockEntityClientEffects::default();
        append_source_effects(
            &world,
            &sources[0],
            0,
            0,
            &mut expected_random,
            &mut expected_effects,
        );
        assert!(!expected_effects.particles.is_empty());

        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut batches = DiscardParticleBatches;
        let mut audio = None;
        runtime.advance_block_entity_effects_for_ticks(
            &world,
            0,
            1,
            None,
            &mut batches,
            &mut audio,
            None,
        );

        assert_eq!(runtime.level_random, expected_random);
    }

    struct DiscardParticleBatches;

    impl ParticleBatchSink for DiscardParticleBatches {
        fn submit_particle_batch(&mut self, _batch: bbb_renderer::ParticleSpawnBatch) {}
    }

    fn world_with_active_trial_spawner() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![bbb_world::vanilla_air_block_state_id()],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        let properties = [
            ("ominous".to_string(), "false".to_string()),
            ("trial_spawner_state".to_string(), "active".to_string()),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let block_state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:trial_spawner", &properties)
            .expect("active trial-spawner state");
        assert!(world.apply_block_update(BlockUpdate {
            pos: bbb_protocol::packets::BlockPos { x: 1, y: 2, z: 3 },
            block_state_id,
        }));
        world
    }
}
