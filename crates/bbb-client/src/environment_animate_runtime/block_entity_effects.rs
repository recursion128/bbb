//! Vanilla client-ticked block-entity particles and local sounds.
//!
//! These effects share `EnvironmentAnimateRuntime::level_random` with
//! `ClientLevel.animateTick`; keeping the algorithms here makes the random
//! cursor and the world-to-backend boundary explicit without growing the
//! environment animate facade further.

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::{
    BlockEntityClientEffectSource, BlockPos, CampfireClientEffectSource, ConduitClientEffectSource,
    LevelEventSoundRandomState, SoundEventState, TrialSpawnerClientEffectSource,
    TrialSpawnerClientState, VaultClientEffectSource, WorldStore,
};
use bbb_protocol::packets::SoundSource;

use super::{local_block_sound, EnvironmentAnimateRuntime, EnvironmentWorldParticleBiomeSampler};
use crate::{
    audio_runtime::AudioEventSink,
    particle_runtime::{ClientParticle, EnvironmentParticleSpawn, ParticleTarget},
};
use bbb_client_world::particles::ParticleKind;

const SOUND_TRIAL_SPAWNER_AMBIENT: &str = "minecraft:block.trial_spawner.ambient";
const SOUND_TRIAL_SPAWNER_AMBIENT_OMINOUS: &str = "minecraft:block.trial_spawner.ambient_ominous";
const SOUND_VAULT_AMBIENT: &str = "minecraft:block.vault.ambient";

#[derive(Default)]
struct BlockEntityClientEffects {
    particles: Vec<EnvironmentParticleSpawn>,
    sounds: Vec<SoundEventState>,
}

impl EnvironmentAnimateRuntime {
    pub(crate) fn advance_block_entity_effects_for_ticks(
        &mut self,
        world: &WorldStore,
        connection_generation: u64,
        running_level_ticks: u32,
        mut particles: Option<ParticleTarget<'_>>,
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
        let mut effects = BlockEntityClientEffects::default();
        for tick_index in 0..running_level_ticks {
            let game_time = start_game_time.saturating_add(i64::from(tick_index));
            let conduit_ticks_after_this = running_level_ticks
                .saturating_sub(tick_index)
                .saturating_sub(1);
            effects.particles.clear();
            effects.sounds.clear();
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
                for sound in &effects.sounds {
                    audio.play_positioned_sound(sound);
                }
            }
            if effects.particles.is_empty() {
                continue;
            }
            let Some(ParticleTarget { runtime, engine }) = particles.as_mut() else {
                continue;
            };
            let camera_position = world.local_player_pose().map(|player| {
                [
                    player.position.x,
                    player.position.y + world.local_player_eye_height(player),
                    player.position.z,
                ]
            });
            let biome_sampler = EnvironmentWorldParticleBiomeSampler { world };
            runtime.spawn_environment_particles(
                engine,
                &effects.particles,
                camera_position,
                Some(&biome_sampler),
                item_runtime,
            );
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
                    ParticleKind::CAMPFIRE_SIGNAL_SMOKE
                } else {
                    ParticleKind::CAMPFIRE_COSY_SMOKE
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
        let direction = (slot + usize::from(source.facing_2d)) % 4;
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
                ParticleKind::SMOKE,
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
    for particle in [ParticleKind::SMOKE, ParticleKind::FLAME] {
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
                ParticleKind::SMOKE,
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
                    ParticleKind::SMOKE,
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
                        ParticleKind::SMOKE,
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
            .sounds
            .push(randomized_block_sound(world, source.pos, sound, random));
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
        push_particle(
            particles,
            ParticleKind::NAUTILUS,
            particle_end,
            velocity,
            false,
        );
    }
    if let Some(target) = source.destroy_target {
        let velocity = [
            f64::from((-0.5 + random.next_float()) * (3.0 + target.width)),
            f64::from(-1.0 + random.next_float() * target.height),
            f64::from((-0.5 + random.next_float()) * (3.0 + target.width)),
        ];
        push_particle(
            particles,
            ParticleKind::NAUTILUS,
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
        if let Some(connection) = world.vault_connection_particle_state(source.pos) {
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
                        ParticleKind::VAULT_CONNECTION,
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
            ParticleKind::SMOKE,
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
        effects.sounds.push(randomized_block_sound(
            world,
            source.pos,
            SOUND_VAULT_AMBIENT,
            random,
        ));
    }
}

fn randomized_block_sound(
    world: &WorldStore,
    pos: BlockPos,
    sound_id: &str,
    random: &mut LevelEventSoundRandomState,
) -> SoundEventState {
    let volume = random.next_float() * 0.25 + 0.75;
    let pitch = random.next_float() + 0.5;
    let seed = random.next_long();
    local_block_sound(
        world,
        sound_id,
        SoundSource::Blocks,
        block_center(pos),
        volume,
        pitch,
        seed,
    )
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

fn flame_particle(ominous: bool, small: bool) -> ParticleKind {
    if ominous {
        ParticleKind::SOUL_FIRE_FLAME
    } else if small {
        ParticleKind::SMALL_FLAME
    } else {
        ParticleKind::FLAME
    }
}

fn push_particle(
    particles: &mut Vec<EnvironmentParticleSpawn>,
    particle: ParticleKind,
    position: [f64; 3],
    velocity: [f64; 3],
    always_show: bool,
) {
    particles.push(EnvironmentParticleSpawn {
        particle: ClientParticle::simple(particle),
        position,
        velocity,
        always_show,
        color_source: None,
    });
}
