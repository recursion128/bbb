use std::collections::BTreeMap;

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    packets::{SoundSource, Vec3d as ProtocolVec3d},
};
use bbb_world::{
    block_name_is_pale_oak_log, BlockPos, ClientSoundEventHolder, EnvironmentBiomeSampling,
    LevelEventSoundRandomState, LocalPlayerPoseState, SoundEventState, TerrainFluidKind,
    TerrainFluidState, WorldStore,
};

use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        pointed_dripstone_drip_context, pointed_dripstone_particle_position,
        simple_client_particle_options, EnvironmentParticleSpawn, LevelParticleSpawnContext,
        ParticleBiomeSampler, ParticleEventSink,
    },
    simulation::effects::ParticleBatchSink,
};

const ANIMATE_TICK_ITERATIONS: usize = 667;
const NEAR_SAMPLE_RADIUS: i32 = 16;
const FAR_SAMPLE_RADIUS: i32 = 32;
const FIREFLY_SOUND_CHANCE_BOUND: i32 = 30;
const FIREFLY_SPAWN_MAX_BRIGHTNESS: u8 = 13;
const FIREFLY_SPAWN_CHANCE: f64 = 0.7;
const FIREFLY_HORIZONTAL_RANGE: f64 = 10.0;
const FIREFLY_VERTICAL_RANGE: f64 = 5.0;
const CREAKING_SOUND_CHANCE_BOUND: i32 = 16;
const BUBBLE_COLUMN_SOUND_CHANCE_BOUND: i32 = 200;
const SOUND_EVENT_FIREFLY_BUSH_IDLE: &str = "minecraft:block.firefly_bush.idle";
const SOUND_EVENT_CREAKING_HEART_IDLE: &str = "minecraft:block.creaking_heart.idle";
const SOUND_EVENT_BUBBLE_COLUMN_UPWARDS_AMBIENT: &str =
    "minecraft:block.bubble_column.upwards_ambient";
const SOUND_EVENT_BUBBLE_COLUMN_WHIRLPOOL_AMBIENT: &str =
    "minecraft:block.bubble_column.whirlpool_ambient";
const SOUND_EVENT_WATER_AMBIENT: &str = "minecraft:block.water.ambient";
const SOUND_EVENT_LAVA_POP: &str = "minecraft:block.lava.pop";
const SOUND_EVENT_LAVA_AMBIENT: &str = "minecraft:block.lava.ambient";
const SOUND_EVENT_FURNACE_FIRE_CRACKLE: &str = "minecraft:block.furnace.fire_crackle";
const SOUND_EVENT_BLAST_FURNACE_FIRE_CRACKLE: &str = "minecraft:block.blastfurnace.fire_crackle";
const SOUND_EVENT_SMOKER_SMOKE: &str = "minecraft:block.smoker.smoke";
const SOUND_EVENT_CAMPFIRE_CRACKLE: &str = "minecraft:block.campfire.crackle";
const SOUND_EVENT_FIRE_AMBIENT: &str = "minecraft:block.fire.ambient";
const SOUND_EVENT_RESPAWN_ANCHOR_AMBIENT: &str = "minecraft:block.respawn_anchor.ambient";
const SOUND_EVENT_PORTAL_AMBIENT: &str = "minecraft:block.portal.ambient";
const SOUND_EVENT_DRIED_GHAST_AMBIENT: &str = "minecraft:block.dried_ghast.ambient";
const SOUND_EVENT_DRIED_GHAST_AMBIENT_WATER: &str = "minecraft:block.dried_ghast.ambient_water";
const PARTICLE_FIREFLY: &str = "minecraft:firefly";
const PARTICLE_CURRENT_DOWN: &str = "minecraft:current_down";
const PARTICLE_BUBBLE_COLUMN_UP: &str = "minecraft:bubble_column_up";
const PARTICLE_UNDERWATER: &str = "minecraft:underwater";
const PARTICLE_LAVA: &str = "minecraft:lava";
const PARTICLE_DRIPPING_WATER: &str = "minecraft:dripping_water";
const PARTICLE_DRIPPING_LAVA: &str = "minecraft:dripping_lava";
const PARTICLE_SMOKE: &str = "minecraft:smoke";
const PARTICLE_FLAME: &str = "minecraft:flame";
const PARTICLE_LARGE_SMOKE: &str = "minecraft:large_smoke";
const PARTICLE_REVERSE_PORTAL: &str = "minecraft:reverse_portal";
const PARTICLE_PORTAL: &str = "minecraft:portal";
const PARTICLE_WHITE_SMOKE: &str = "minecraft:white_smoke";
const PARTICLE_HAPPY_VILLAGER: &str = "minecraft:happy_villager";

type FluidAnimateSound = (&'static str, [f64; 3], f32, f32);
type FluidAnimateSounds = [Option<FluidAnimateSound>; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FurnaceAnimateKind {
    Furnace,
    BlastFurnace,
    Smoker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FireSmokeFace {
    West,
    East,
    North,
    South,
    Up,
}

/// Non-serializable owner of Vanilla `ClientLevel.animateTick` random cursors.
#[derive(Debug)]
pub(crate) struct EnvironmentAnimateRuntime {
    level_identity: Option<(u64, u64)>,
    level_random: LevelEventSoundRandomState,
    animate_seed_random: LevelEventSoundRandomState,
}

impl Default for EnvironmentAnimateRuntime {
    fn default() -> Self {
        Self {
            level_identity: None,
            level_random: LevelEventSoundRandomState::with_seed(0),
            animate_seed_random: LevelEventSoundRandomState::with_seed(1),
        }
    }
}

impl EnvironmentAnimateRuntime {
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        connection_generation: u64,
        running_level_ticks: u32,
        mut particle_events: Option<&mut dyn ParticleEventSink>,
        particle_batches: &mut dyn ParticleBatchSink,
        audio: &mut Option<&mut dyn AudioEventSink>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        let level_identity = (connection_generation, world.client_level_generation());
        if self.level_identity != Some(level_identity) {
            self.level_identity = Some(level_identity);
            self.level_random = LevelEventSoundRandomState::with_seed(0);
            self.animate_seed_random = LevelEventSoundRandomState::with_seed(1);
        }

        for _ in 0..running_level_ticks {
            let Some(player) = world.local_player_pose() else {
                continue;
            };
            let spawns = self.tick(world, player, audio);
            if spawns.is_empty() {
                continue;
            }
            let Some(particle_events) = particle_events.as_deref_mut() else {
                continue;
            };
            let biome_sampler = EnvironmentWorldParticleBiomeSampler { world };
            let batch = particle_events.spawn_environment_particles(
                &spawns,
                LevelParticleSpawnContext {
                    camera_position: Some([
                        player.position.x,
                        player.position.y + player.eye_height(),
                        player.position.z,
                    ]),
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

    fn tick(
        &mut self,
        world: &mut WorldStore,
        player: LocalPlayerPoseState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) -> Vec<EnvironmentParticleSpawn> {
        let center = BlockPos {
            x: containing_block_coordinate(player.position.x),
            y: containing_block_coordinate(player.position.y),
            z: containing_block_coordinate(player.position.z),
        };
        let sky_darken = crate::runtime::entity_shadow_sky_darken(world);
        let mut animate_random =
            LevelEventSoundRandomState::with_seed(self.animate_seed_random.next_long());
        let mut spawns = Vec::new();
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            self.do_animate_tick(
                world,
                center,
                NEAR_SAMPLE_RADIUS,
                sky_darken,
                &mut animate_random,
                audio,
                &mut spawns,
            );
            self.do_animate_tick(
                world,
                center,
                FAR_SAMPLE_RADIUS,
                sky_darken,
                &mut animate_random,
                audio,
                &mut spawns,
            );
        }
        spawns
    }

    fn do_animate_tick(
        &mut self,
        world: &mut WorldStore,
        center: BlockPos,
        radius: i32,
        sky_darken: u8,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let pos = sample_animate_position(&mut self.level_random, center, radius);
        let Some(block) = world.probe_block(pos) else {
            return;
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return;
        };

        match block_name {
            "minecraft:firefly_bush" => {
                self.animate_firefly_bush(world, pos, sky_darken, animate_random, audio, spawns);
            }
            "minecraft:creaking_heart" => {
                self.animate_creaking_heart(
                    world,
                    pos,
                    block
                        .block_properties
                        .get("creaking_heart_state")
                        .map(String::as_str),
                    animate_random,
                    audio,
                );
            }
            "minecraft:bubble_column" => {
                self.animate_bubble_column(
                    world,
                    pos,
                    block
                        .block_properties
                        .get("drag")
                        .map(String::as_str)
                        .is_none_or(|value| value == "true"),
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:furnace" => {
                self.animate_furnace_block(
                    world,
                    pos,
                    FurnaceAnimateKind::Furnace,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:blast_furnace" => {
                self.animate_furnace_block(
                    world,
                    pos,
                    FurnaceAnimateKind::BlastFurnace,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:smoker" => {
                self.animate_furnace_block(
                    world,
                    pos,
                    FurnaceAnimateKind::Smoker,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:campfire" => {
                self.animate_campfire(
                    world,
                    pos,
                    true,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:soul_campfire" => {
                self.animate_campfire(
                    world,
                    pos,
                    false,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:fire" => {
                self.animate_fire(world, pos, false, animate_random, audio, spawns);
            }
            "minecraft:soul_fire" => {
                self.animate_fire(world, pos, true, animate_random, audio, spawns);
            }
            "minecraft:respawn_anchor" => {
                self.animate_respawn_anchor(
                    world,
                    pos,
                    &block.block_properties,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:nether_portal" => {
                self.animate_nether_portal(world, pos, animate_random, audio, spawns);
            }
            "minecraft:dried_ghast" => {
                self.animate_dried_ghast(
                    world,
                    pos,
                    block
                        .block_properties
                        .get("waterlogged")
                        .is_some_and(|value| value == "true"),
                    animate_random,
                    audio,
                    spawns,
                );
            }
            "minecraft:pointed_dripstone" => {
                append_pointed_dripstone_animate_effect(
                    world,
                    pos,
                    &block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            _ => {}
        }

        if let Some(fluid) = block.fluid {
            self.animate_fluid(world, pos, fluid, animate_random, audio, spawns);
            if self.level_random.next_int_bound(10) == 0 {
                append_fluid_drip_particle(world, pos, fluid.kind, &mut self.level_random, spawns);
            }
        }

        if world.sample_block_collision_shape_is_full_block(pos) != Some(false) {
            return;
        }
        let particles = world
            .sampled_client_environment_attributes_at(
                block_position(pos),
                EnvironmentBiomeSampling::Direct,
            )
            .ok()
            .and_then(|attributes| attributes.ambient_particles)
            .unwrap_or_default();
        append_ambient_particle_spawns(&mut self.level_random, pos, particles, spawns);
    }

    fn animate_firefly_bush(
        &mut self,
        world: &mut WorldStore,
        pos: BlockPos,
        sky_darken: u8,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        if animate_random.next_int_bound(FIREFLY_SOUND_CHANCE_BOUND) == 0
            && direct_environment_bool(world, pos, |attributes| attributes.firefly_bush_sounds)
            && firefly_sound_height_allows(
                world.sample_heightmap_first_available(
                    VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
                    pos.x,
                    pos.z,
                ),
                pos.y,
            )
        {
            let seed = self.level_random.next_long();
            if let Some(audio) = audio.as_deref_mut() {
                audio.play_positioned_sound(&local_block_sound(
                    world,
                    SOUND_EVENT_FIREFLY_BUSH_IDLE,
                    SoundSource::Ambient,
                    [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ],
                    1.0,
                    1.0,
                    seed,
                ));
            }
        }

        append_firefly_spawn(
            animate_random,
            pos,
            max_local_raw_brightness(world, pos, sky_darken),
            spawns,
        );
    }

    fn animate_creaking_heart(
        &mut self,
        world: &mut WorldStore,
        pos: BlockPos,
        state: Option<&str>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if !direct_environment_bool(world, pos, |attributes| attributes.creaking_active)
            || state.is_none_or(|state| state == "uprooted")
            || animate_random.next_int_bound(CREAKING_SOUND_CHANCE_BOUND) != 0
            || !creaking_heart_is_surrounded_by_logs(world, pos)
        {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_CREAKING_HEART_IDLE,
                SoundSource::Blocks,
                [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)],
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_bubble_column(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        drag_down: bool,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let Some((sound_id, volume, pitch)) =
            append_bubble_column_animate_effects(pos, drag_down, animate_random, spawns)
        else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                sound_id,
                SoundSource::Blocks,
                block_position(pos),
                volume,
                pitch,
                seed,
            ));
        }
    }

    fn animate_fluid(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        fluid: TerrainFluidState,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let sound = match fluid.kind {
            TerrainFluidKind::Water => {
                append_water_fluid_animate_effects(pos, fluid, animate_random, spawns)
            }
            TerrainFluidKind::Lava => {
                let above = BlockPos {
                    y: pos.y.saturating_add(1),
                    ..pos
                };
                let exposed = world
                    .environment_animate_block_sample(above)
                    .is_some_and(|sample| sample.block_is_air && !sample.solid_render);
                append_lava_fluid_animate_effects(pos, exposed, animate_random, spawns)
            }
        };
        for (sound_id, position, volume, pitch) in sound.into_iter().flatten() {
            let seed = self.level_random.next_long();
            if let Some(audio) = audio.as_deref_mut() {
                audio.play_positioned_sound(&local_block_sound(
                    world,
                    sound_id,
                    SoundSource::Ambient,
                    position,
                    volume,
                    pitch,
                    seed,
                ));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn animate_furnace_block(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        kind: FurnaceAnimateKind,
        properties: &std::collections::BTreeMap<String, String>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let Some(sound_id) =
            append_furnace_animate_effects(pos, kind, properties, animate_random, spawns)
        else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                sound_id,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y),
                    f64::from(pos.z) + 0.5,
                ],
                1.0,
                1.0,
                seed,
            ));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn animate_campfire(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        spawn_particles: bool,
        properties: &std::collections::BTreeMap<String, String>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let Some((volume, pitch)) = append_campfire_animate_effects(
            pos,
            spawn_particles,
            properties,
            animate_random,
            spawns,
        ) else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_CAMPFIRE_CRACKLE,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                volume,
                pitch,
                seed,
            ));
        }
    }

    fn animate_fire(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        burns_all_blocks: bool,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let sound =
            append_fire_animate_effects(world, pos, burns_all_blocks, animate_random, spawns);
        let Some((volume, pitch)) = sound else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_FIRE_AMBIENT,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                volume,
                pitch,
                seed,
            ));
        }
    }

    fn animate_respawn_anchor(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        properties: &std::collections::BTreeMap<String, String>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        if !append_respawn_anchor_animate_effects(pos, properties, animate_random, spawns) {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_RESPAWN_ANCHOR_AMBIENT,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_nether_portal(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let Some(pitch) = append_nether_portal_animate_effects(world, pos, animate_random, spawns)
        else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_PORTAL_AMBIENT,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                0.5,
                pitch,
                seed,
            ));
        }
    }

    fn animate_dried_ghast(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        waterlogged: bool,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let Some(sound_id) =
            append_dried_ghast_animate_effects(world, pos, waterlogged, animate_random, spawns)
        else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                sound_id,
                SoundSource::Blocks,
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                1.0,
                1.0,
                seed,
            ));
        }
    }
}

fn append_bubble_column_animate_effects(
    pos: BlockPos,
    drag_down: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(&'static str, f32, f32)> {
    let x = f64::from(pos.x);
    let y = f64::from(pos.y);
    let z = f64::from(pos.z);
    let sound_id = if drag_down {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_CURRENT_DOWN),
            position: [x + 0.5, y + 0.8, z],
            velocity: [0.0; 3],
            always_show: true,
        });
        SOUND_EVENT_BUBBLE_COLUMN_WHIRLPOOL_AMBIENT
    } else {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_BUBBLE_COLUMN_UP),
            position: [x + 0.5, y, z + 0.5],
            velocity: [0.0, 0.04, 0.0],
            always_show: true,
        });
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_BUBBLE_COLUMN_UP),
            position: [
                x + f64::from(random.next_float()),
                y + f64::from(random.next_float()),
                z + f64::from(random.next_float()),
            ],
            velocity: [0.0, 0.04, 0.0],
            always_show: true,
        });
        SOUND_EVENT_BUBBLE_COLUMN_UPWARDS_AMBIENT
    };

    if random.next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND) != 0 {
        return None;
    }
    Some((
        sound_id,
        0.2 + random.next_float() * 0.2,
        0.9 + random.next_float() * 0.15,
    ))
}

fn append_water_fluid_animate_effects(
    pos: BlockPos,
    fluid: TerrainFluidState,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> FluidAnimateSounds {
    if fluid.amount < 8 && !fluid.falling {
        if random.next_int_bound(64) == 0 {
            return [
                Some((
                    SOUND_EVENT_WATER_AMBIENT,
                    [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ],
                    random.next_float() * 0.25 + 0.75,
                    random.next_float() + 0.5,
                )),
                None,
            ];
        }
    } else if random.next_int_bound(10) == 0 {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_UNDERWATER),
            position: [
                f64::from(pos.x) + random.next_double(),
                f64::from(pos.y) + random.next_double(),
                f64::from(pos.z) + random.next_double(),
            ],
            velocity: [0.0; 3],
            always_show: false,
        });
    }
    [None; 2]
}

fn append_lava_fluid_animate_effects(
    pos: BlockPos,
    exposed: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> FluidAnimateSounds {
    if !exposed {
        return [None; 2];
    }
    let mut sounds = [None; 2];
    let mut next_sound = 0;
    if random.next_int_bound(100) == 0 {
        let position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 1.0,
            f64::from(pos.z) + random.next_double(),
        ];
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_LAVA),
            position,
            velocity: [0.0; 3],
            always_show: false,
        });
        sounds[next_sound] = Some((
            SOUND_EVENT_LAVA_POP,
            position,
            0.2 + random.next_float() * 0.2,
            0.9 + random.next_float() * 0.15,
        ));
        next_sound += 1;
    }
    if random.next_int_bound(200) == 0 {
        sounds[next_sound] = Some((
            SOUND_EVENT_LAVA_AMBIENT,
            block_position(pos),
            0.2 + random.next_float() * 0.2,
            0.9 + random.next_float() * 0.15,
        ));
    }
    sounds
}

fn append_fluid_drip_particle(
    world: &WorldStore,
    fluid_pos: BlockPos,
    fluid_kind: TerrainFluidKind,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(fluid_block) = world.environment_animate_block_sample(fluid_pos) else {
        return;
    };
    let below_pos = BlockPos {
        y: fluid_pos.y.saturating_sub(1),
        ..fluid_pos
    };
    let Some(below) = world.environment_animate_block_sample(below_pos) else {
        return;
    };
    if below.fluid.is_some() {
        return;
    }

    let below_top = below
        .collision_bounds
        .map_or(f64::NEG_INFINITY, |bounds| bounds.max_y);
    let range_and_y = if below_top < 1.0 {
        fluid_block.down_face_sturdy.then_some((
            [f64::from(below_pos.x), f64::from(below_pos.x) + 1.0],
            [f64::from(below_pos.z), f64::from(below_pos.z) + 1.0],
            f64::from(below_pos.y) + 0.95,
        ))
    } else if below.impermeable {
        None
    } else {
        let Some(bounds) = below.collision_bounds else {
            return;
        };
        if bounds.min_y > 0.0 {
            Some((
                [
                    f64::from(below_pos.x) + bounds.min_x,
                    f64::from(below_pos.x) + bounds.max_x,
                ],
                [
                    f64::from(below_pos.z) + bounds.min_z,
                    f64::from(below_pos.z) + bounds.max_z,
                ],
                f64::from(below_pos.y) + bounds.min_y - 0.05,
            ))
        } else {
            let two_below_pos = BlockPos {
                y: below_pos.y.saturating_sub(1),
                ..below_pos
            };
            let Some(two_below) = world.environment_animate_block_sample(two_below_pos) else {
                return;
            };
            let two_below_top = two_below
                .collision_bounds
                .map_or(f64::NEG_INFINITY, |bounds| bounds.max_y);
            (two_below_top < 1.0 && two_below.fluid.is_none()).then_some((
                [
                    f64::from(below_pos.x) + bounds.min_x,
                    f64::from(below_pos.x) + bounds.max_x,
                ],
                [
                    f64::from(below_pos.z) + bounds.min_z,
                    f64::from(below_pos.z) + bounds.max_z,
                ],
                f64::from(below_pos.y) - 0.05,
            ))
        }
    };
    let Some((x_range, z_range, y)) = range_and_y else {
        return;
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(match fluid_kind {
            TerrainFluidKind::Water => PARTICLE_DRIPPING_WATER,
            TerrainFluidKind::Lava => PARTICLE_DRIPPING_LAVA,
        }),
        position: [
            x_range[0] + random.next_double() * (x_range[1] - x_range[0]),
            y,
            z_range[0] + random.next_double() * (z_range[1] - z_range[0]),
        ],
        velocity: [0.0; 3],
        always_show: false,
    });
}

fn append_furnace_animate_effects(
    pos: BlockPos,
    kind: FurnaceAnimateKind,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<&'static str> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return None;
    }

    let sound = (random.next_double() < 0.1).then_some(match kind {
        FurnaceAnimateKind::Furnace => SOUND_EVENT_FURNACE_FIRE_CRACKLE,
        FurnaceAnimateKind::BlastFurnace => SOUND_EVENT_BLAST_FURNACE_FIRE_CRACKLE,
        FurnaceAnimateKind::Smoker => SOUND_EVENT_SMOKER_SMOKE,
    });
    let center = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    if kind == FurnaceAnimateKind::Smoker {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_SMOKE),
            position: [center[0], center[1] + 1.1, center[2]],
            velocity: [0.0; 3],
            always_show: false,
        });
        return sound;
    }

    let facing = properties.get("facing").map(String::as_str);
    let side_offset = random.next_double() * 0.6 - 0.3;
    let vertical_range = if kind == FurnaceAnimateKind::Furnace {
        6.0 / 16.0
    } else {
        9.0 / 16.0
    };
    let vertical_offset = random.next_double() * vertical_range;
    let (dx, dz) = match facing {
        Some("east") => (0.52, side_offset),
        Some("west") => (-0.52, side_offset),
        Some("south") => (side_offset, 0.52),
        _ => (side_offset, -0.52),
    };
    let position = [center[0] + dx, center[1] + vertical_offset, center[2] + dz];
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_SMOKE),
        position,
        velocity: [0.0; 3],
        always_show: false,
    });
    if kind == FurnaceAnimateKind::Furnace {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_FLAME),
            position,
            velocity: [0.0; 3],
            always_show: false,
        });
    }
    sound
}

fn append_campfire_animate_effects(
    pos: BlockPos,
    spawn_particles: bool,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(f32, f32)> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return None;
    }

    let sound = (random.next_int_bound(10) == 0)
        .then(|| (0.5 + random.next_float(), random.next_float() * 0.7 + 0.6));
    if spawn_particles && random.next_int_bound(5) == 0 {
        let particle_count = random.next_int_bound(1) + 1;
        for _ in 0..particle_count {
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_LAVA),
                position: [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ],
                velocity: [
                    f64::from(random.next_float() / 2.0),
                    5.0e-5,
                    f64::from(random.next_float() / 2.0),
                ],
                always_show: false,
            });
        }
    }
    sound
}

fn append_fire_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    burns_all_blocks: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<(f32, f32)> {
    let sound = (random.next_int_bound(24) == 0)
        .then(|| (1.0 + random.next_float(), random.next_float() * 0.7 + 0.3));
    let can_burn = |target| {
        burns_all_blocks
            || world
                .environment_animate_block_sample(target)
                .is_some_and(|sample| sample.fire_flammable)
    };
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_sample = world.environment_animate_block_sample(below);
    let below_burns = burns_all_blocks || below_sample.is_some_and(|sample| sample.fire_flammable);
    let below_has_sturdy_top = below_sample.is_some_and(|sample| sample.up_face_sturdy);
    if below_burns || below_has_sturdy_top {
        for _ in 0..3 {
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_LARGE_SMOKE),
                position: [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double() * 0.5 + 0.5,
                    f64::from(pos.z) + random.next_double(),
                ],
                velocity: [0.0; 3],
                always_show: false,
            });
        }
        return sound;
    }

    for (target, side) in [
        (
            BlockPos {
                x: pos.x.saturating_sub(1),
                ..pos
            },
            FireSmokeFace::West,
        ),
        (
            BlockPos {
                x: pos.x.saturating_add(1),
                ..pos
            },
            FireSmokeFace::East,
        ),
        (
            BlockPos {
                z: pos.z.saturating_sub(1),
                ..pos
            },
            FireSmokeFace::North,
        ),
        (
            BlockPos {
                z: pos.z.saturating_add(1),
                ..pos
            },
            FireSmokeFace::South,
        ),
        (
            BlockPos {
                y: pos.y.saturating_add(1),
                ..pos
            },
            FireSmokeFace::Up,
        ),
    ] {
        if !can_burn(target) {
            continue;
        }
        for _ in 0..2 {
            let position = match side {
                FireSmokeFace::West => [
                    f64::from(pos.x) + random.next_double() * 0.1,
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
                FireSmokeFace::East => [
                    f64::from(pos.x) + 1.0 - random.next_double() * 0.1,
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
                FireSmokeFace::North => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double() * 0.1,
                ],
                FireSmokeFace::South => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + 1.0 - random.next_double() * 0.1,
                ],
                FireSmokeFace::Up => [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + 1.0 - random.next_double() * 0.1,
                    f64::from(pos.z) + random.next_double(),
                ],
            };
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_LARGE_SMOKE),
                position,
                velocity: [0.0; 3],
                always_show: false,
            });
        }
    }
    sound
}

fn append_respawn_anchor_animate_effects(
    pos: BlockPos,
    properties: &std::collections::BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> bool {
    if !properties
        .get("charges")
        .is_some_and(|charges| charges != "0")
    {
        return false;
    }
    let play_sound = random.next_int_bound(100) == 0;
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_REVERSE_PORTAL),
        position: [
            f64::from(pos.x) + 0.5 + (0.5 - random.next_double()),
            f64::from(pos.y) + 1.0,
            f64::from(pos.z) + 0.5 + (0.5 - random.next_double()),
        ],
        velocity: [0.0, f64::from(random.next_float()) * 0.04, 0.0],
        always_show: false,
    });
    play_sound
}

fn append_nether_portal_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<f32> {
    let pitch = (random.next_int_bound(100) == 0).then(|| random.next_float() * 0.4 + 0.8);
    let portal_at = |target| {
        world
            .probe_block(target)
            .is_some_and(|block| block.block_name.as_deref() == Some("minecraft:nether_portal"))
    };
    let pin_x = !portal_at(BlockPos {
        x: pos.x.saturating_sub(1),
        ..pos
    }) && !portal_at(BlockPos {
        x: pos.x.saturating_add(1),
        ..pos
    });
    for _ in 0..4 {
        let mut position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + random.next_double(),
            f64::from(pos.z) + random.next_double(),
        ];
        let mut velocity = [
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
        ];
        let flip = random.next_int_bound(2) * 2 - 1;
        if pin_x {
            position[0] = f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip);
            velocity[0] = f64::from(random.next_float() * 2.0 * flip as f32);
        } else {
            position[2] = f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip);
            velocity[2] = f64::from(random.next_float() * 2.0 * flip as f32);
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_PORTAL),
            position,
            velocity,
            always_show: false,
        });
    }
    pitch
}

fn append_dried_ghast_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    waterlogged: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Option<&'static str> {
    let center = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ];
    if waterlogged {
        let sound =
            (random.next_int_bound(40) == 0).then_some(SOUND_EVENT_DRIED_GHAST_AMBIENT_WATER);
        if random.next_int_bound(6) == 0 {
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_HAPPY_VILLAGER),
                position: [
                    center[0] + f64::from((random.next_float() * 2.0 - 1.0) / 3.0),
                    center[1] + 0.4,
                    center[2] + f64::from((random.next_float() * 2.0 - 1.0) / 3.0),
                ],
                velocity: [0.0, f64::from(random.next_float()), 0.0],
                always_show: false,
            });
        }
        return sound;
    }

    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let sound = (random.next_int_bound(40) == 0
        && world
            .environment_animate_block_sample(below)
            .is_some_and(|sample| sample.triggers_dried_ghast_ambient_sound))
    .then_some(SOUND_EVENT_DRIED_GHAST_AMBIENT);
    if random.next_int_bound(6) == 0 {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_WHITE_SMOKE),
            position: center,
            velocity: [0.0, 0.02, 0.0],
            always_show: false,
        });
    }
    sound
}

fn append_pointed_dripstone_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("vertical_direction").map(String::as_str) != Some("down")
        || properties.get("thickness").map(String::as_str) != Some("tip")
        || properties.get("waterlogged").map(String::as_str) != Some("false")
    {
        return;
    }

    let random_value = random.next_float();
    if random_value > 0.12 {
        return;
    }
    let Some(context) = pointed_dripstone_drip_context(world, pos) else {
        return;
    };
    if random_value >= 0.02 && context.fluid_kind.is_none() {
        return;
    }

    spawns.push(EnvironmentParticleSpawn {
        particle: context.particle,
        position: pointed_dripstone_particle_position(pos),
        velocity: [0.0; 3],
        always_show: false,
    });
}

fn append_ambient_particle_spawns(
    random: &mut LevelEventSoundRandomState,
    pos: BlockPos,
    particles: Vec<bbb_world::ClientAmbientParticle>,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for particle in particles {
        if random.next_float() <= f32::from_bits(particle.probability_bits) {
            spawns.push(EnvironmentParticleSpawn {
                particle: particle.particle,
                position: [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
                velocity: [0.0; 3],
                always_show: false,
            });
        }
    }
}

fn append_firefly_spawn(
    random: &mut LevelEventSoundRandomState,
    pos: BlockPos,
    brightness: Option<u8>,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(brightness) = brightness else {
        return;
    };
    if brightness <= FIREFLY_SPAWN_MAX_BRIGHTNESS && random.next_double() <= FIREFLY_SPAWN_CHANCE {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_FIREFLY),
            position: [
                f64::from(pos.x) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
                f64::from(pos.y) + random.next_double() * FIREFLY_VERTICAL_RANGE,
                f64::from(pos.z) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
            ],
            velocity: [0.0; 3],
            always_show: false,
        });
    }
}

fn sample_animate_position(
    random: &mut LevelEventSoundRandomState,
    center: BlockPos,
    radius: i32,
) -> BlockPos {
    BlockPos {
        x: center.x + random.next_int_bound(radius) - random.next_int_bound(radius),
        y: center.y + random.next_int_bound(radius) - random.next_int_bound(radius),
        z: center.z + random.next_int_bound(radius) - random.next_int_bound(radius),
    }
}

fn direct_environment_bool(
    world: &WorldStore,
    pos: BlockPos,
    value: impl FnOnce(bbb_world::ClientEnvironmentAttributes) -> Option<bool>,
) -> bool {
    world
        .sampled_client_environment_attributes_at(
            block_position(pos),
            EnvironmentBiomeSampling::Direct,
        )
        .ok()
        .and_then(value)
        .unwrap_or(false)
}

fn max_local_raw_brightness(world: &WorldStore, pos: BlockPos, sky_darken: u8) -> Option<u8> {
    if pos.x < -30_000_000 || pos.z < -30_000_000 || pos.x >= 30_000_000 || pos.z >= 30_000_000 {
        return Some(15);
    }
    world
        .sample_block_light(pos)
        .map(|light| light.block.max(light.sky.saturating_sub(sky_darken)))
}

fn firefly_sound_height_allows(height: Option<i32>, block_y: i32) -> bool {
    height.is_some_and(|height| height <= block_y)
}

fn creaking_heart_is_surrounded_by_logs(world: &WorldStore, pos: BlockPos) -> bool {
    [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ]
    .into_iter()
    .all(|(dx, dy, dz)| {
        world
            .probe_block(BlockPos {
                x: pos.x + dx,
                y: pos.y + dy,
                z: pos.z + dz,
            })
            .and_then(|block| block.block_name)
            .is_some_and(|name| block_name_is_pale_oak_log(&name))
    })
}

fn local_block_sound(
    world: &WorldStore,
    resource_id: &str,
    source: SoundSource,
    position: [f64; 3],
    volume: f32,
    pitch: f32,
    seed: i64,
) -> SoundEventState {
    SoundEventState {
        sound: sound_holder_state(
            world,
            &ClientSoundEventHolder::Reference {
                resource_id: resource_id.to_string(),
            },
        ),
        source: source.as_str().to_string(),
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        volume,
        pitch,
        seed,
        distance_delay: false,
    }
}

fn block_position(pos: BlockPos) -> [f64; 3] {
    [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)]
}

fn containing_block_coordinate(value: f64) -> i32 {
    value.floor() as i32
}

struct EnvironmentWorldParticleBiomeSampler<'a> {
    world: &'a WorldStore,
}

impl ParticleBiomeSampler for EnvironmentWorldParticleBiomeSampler<'_> {
    fn biome_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.world.probe_block(pos).and_then(|probe| probe.biome_id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{
        BlockUpdate, ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes,
        CommonPlayerSpawnInfo, PlayLogin,
    };
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, ClientAmbientParticle, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, RegistryPacketEntry, WorldDimension,
    };

    use super::*;

    #[test]
    fn animate_position_consumes_near_then_far_coordinate_draws() {
        let center = BlockPos {
            x: 10,
            y: 20,
            z: 30,
        };
        let mut actual = LevelEventSoundRandomState::with_seed(42);
        let near = sample_animate_position(&mut actual, center, NEAR_SAMPLE_RADIUS);
        let far = sample_animate_position(&mut actual, center, FAR_SAMPLE_RADIUS);

        let mut expected = LevelEventSoundRandomState::with_seed(42);
        let expected_near = BlockPos {
            x: center.x + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
            y: center.y + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
            z: center.z + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
        };
        let expected_far = BlockPos {
            x: center.x + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
            y: center.y + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
            z: center.z + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
        };

        assert_eq!(near, expected_near);
        assert_eq!(far, expected_far);
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn simple_particle_options_preserve_the_canonical_type_field() {
        let options = simple_client_particle_options(PARTICLE_FIREFLY);
        assert_eq!(options.resource_id, PARTICLE_FIREFLY);
        assert_eq!(
            options.fields,
            vec![bbb_protocol::NbtSummaryEntry {
                name: "type".to_string(),
                value: bbb_protocol::NbtSummaryValue::String(PARTICLE_FIREFLY.to_string()),
            }]
        );
    }

    #[test]
    fn local_block_sounds_preserve_position_source_and_shared_level_seed() {
        let world = WorldStore::default();
        let firefly = local_block_sound(
            &world,
            SOUND_EVENT_FIREFLY_BUSH_IDLE,
            SoundSource::Ambient,
            [1.5, 2.5, 3.5],
            1.0,
            1.0,
            42,
        );
        let creaking = local_block_sound(
            &world,
            SOUND_EVENT_CREAKING_HEART_IDLE,
            SoundSource::Blocks,
            [1.0, 2.0, 3.0],
            1.0,
            1.0,
            84,
        );
        let bubble = local_block_sound(
            &world,
            SOUND_EVENT_BUBBLE_COLUMN_UPWARDS_AMBIENT,
            SoundSource::Blocks,
            [4.0, 5.0, 6.0],
            0.3,
            0.95,
            96,
        );

        assert_eq!(
            firefly.position,
            ProtocolVec3d {
                x: 1.5,
                y: 2.5,
                z: 3.5
            }
        );
        assert_eq!(firefly.source, "ambient");
        assert_eq!(firefly.seed, 42);
        assert_eq!(
            creaking.position,
            ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
        assert_eq!(creaking.source, "block");
        assert_eq!(creaking.seed, 84);
        assert_eq!(bubble.source, "block");
        assert_eq!(bubble.volume, 0.3);
        assert_eq!(bubble.pitch, 0.95);
        assert_eq!(bubble.seed, 96);
    }

    #[test]
    fn animate_tick_runs_exactly_667_near_far_pairs() {
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut world = WorldStore::default();
        let mut audio = None;
        let player = player_pose([0.0, 64.0, 0.0]);

        assert!(runtime.tick(&mut world, player, &mut audio).is_empty());

        let mut expected = LevelEventSoundRandomState::with_seed(0);
        let center = BlockPos { x: 0, y: 64, z: 0 };
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            let _ = sample_animate_position(&mut expected, center, NEAR_SAMPLE_RADIUS);
            let _ = sample_animate_position(&mut expected, center, FAR_SAMPLE_RADIUS);
        }
        assert_eq!(runtime.level_random.next_long(), expected.next_long());

        let mut expected_animate_seeds = LevelEventSoundRandomState::with_seed(1);
        let _ = expected_animate_seeds.next_long();
        assert_eq!(
            runtime.animate_seed_random.next_long(),
            expected_animate_seeds.next_long()
        );
    }

    #[test]
    fn ambient_particles_test_in_order_and_draw_positions_only_on_success() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let particles = vec![
            ambient_particle("minecraft:ash", 1.0),
            ambient_particle("minecraft:white_ash", -1.0),
            ambient_particle("minecraft:cloud", 1.0),
        ];
        let mut actual = LevelEventSoundRandomState::with_seed(91);
        let mut spawns = Vec::new();

        append_ambient_particle_spawns(&mut actual, pos, particles, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(91);
        let mut expected_positions = Vec::new();
        for succeeds in [true, false, true] {
            let _ = expected.next_float();
            if succeeds {
                expected_positions.push([
                    f64::from(pos.x) + expected.next_double(),
                    f64::from(pos.y) + expected.next_double(),
                    f64::from(pos.z) + expected.next_double(),
                ]);
            }
        }
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[0].particle.resource_id, "minecraft:ash");
        assert_eq!(spawns[1].particle.resource_id, "minecraft:cloud");
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_positions
        );
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn bubble_column_animate_effects_preserve_vanilla_particles_and_random_order() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let drag_seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic whirlpool ambient sound seed");
        let mut drag_random = LevelEventSoundRandomState::with_seed(drag_seed);
        let mut drag_spawns = Vec::new();
        let drag_sound =
            append_bubble_column_animate_effects(pos, true, &mut drag_random, &mut drag_spawns)
                .unwrap();
        let mut expected_drag = LevelEventSoundRandomState::with_seed(drag_seed);
        assert_eq!(
            expected_drag.next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND),
            0
        );
        let expected_drag_volume = 0.2 + expected_drag.next_float() * 0.2;
        let expected_drag_pitch = 0.9 + expected_drag.next_float() * 0.15;
        assert_eq!(
            drag_sound,
            (
                SOUND_EVENT_BUBBLE_COLUMN_WHIRLPOOL_AMBIENT,
                expected_drag_volume,
                expected_drag_pitch,
            )
        );
        assert_eq!(drag_spawns.len(), 1);
        assert_eq!(drag_spawns[0].particle.resource_id, PARTICLE_CURRENT_DOWN);
        assert_eq!(drag_spawns[0].position, [-3.5, 12.8, 30.0]);
        assert_eq!(drag_spawns[0].velocity, [0.0; 3]);
        assert!(drag_spawns[0].always_show);
        assert_eq!(drag_random.next_long(), expected_drag.next_long());

        let upward_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                for _ in 0..3 {
                    let _ = random.next_float();
                }
                random.next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND) == 0
            })
            .expect("deterministic upward ambient sound seed");
        let mut upward_random = LevelEventSoundRandomState::with_seed(upward_seed);
        let mut upward_spawns = Vec::new();
        let upward_sound = append_bubble_column_animate_effects(
            pos,
            false,
            &mut upward_random,
            &mut upward_spawns,
        )
        .unwrap();
        let mut expected_upward = LevelEventSoundRandomState::with_seed(upward_seed);
        let random_position = [
            -4.0 + f64::from(expected_upward.next_float()),
            12.0 + f64::from(expected_upward.next_float()),
            30.0 + f64::from(expected_upward.next_float()),
        ];
        assert_eq!(
            expected_upward.next_int_bound(BUBBLE_COLUMN_SOUND_CHANCE_BOUND),
            0
        );
        let expected_upward_volume = 0.2 + expected_upward.next_float() * 0.2;
        let expected_upward_pitch = 0.9 + expected_upward.next_float() * 0.15;
        assert_eq!(
            upward_sound,
            (
                SOUND_EVENT_BUBBLE_COLUMN_UPWARDS_AMBIENT,
                expected_upward_volume,
                expected_upward_pitch,
            )
        );
        assert_eq!(upward_spawns.len(), 2);
        assert_eq!(upward_spawns[0].position, [-3.5, 12.0, 30.5],);
        assert_eq!(upward_spawns[1].position, random_position);
        assert!(upward_spawns.iter().all(|spawn| {
            spawn.particle.resource_id == PARTICLE_BUBBLE_COLUMN_UP
                && spawn.velocity == [0.0, 0.04, 0.0]
                && spawn.always_show
        }));
        assert_eq!(upward_random.next_long(), expected_upward.next_long());

        let water_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(10) == 0)
            .expect("deterministic underwater particle seed");
        let mut water_random = LevelEventSoundRandomState::with_seed(water_seed);
        let mut water_spawns = Vec::new();
        let _ = append_water_fluid_animate_effects(
            pos,
            TerrainFluidState::new(TerrainFluidKind::Water, 8, false),
            &mut water_random,
            &mut water_spawns,
        );
        let mut expected_water = LevelEventSoundRandomState::with_seed(water_seed);
        assert_eq!(expected_water.next_int_bound(10), 0);
        assert_eq!(water_spawns.len(), 1);
        assert_eq!(water_spawns[0].particle.resource_id, "minecraft:underwater");
        assert_eq!(
            water_spawns[0].position,
            [
                -4.0 + expected_water.next_double(),
                12.0 + expected_water.next_double(),
                30.0 + expected_water.next_double(),
            ]
        );
        assert_eq!(water_spawns[0].velocity, [0.0; 3]);
        assert!(!water_spawns[0].always_show);
        assert_eq!(water_random.next_long(), expected_water.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut sound_random = LevelEventSoundRandomState::with_seed(drag_seed);
        let mut audio = None;
        let mut spawns = Vec::new();
        runtime.animate_bubble_column(
            &WorldStore::default(),
            pos,
            true,
            &mut sound_random,
            &mut audio,
            &mut spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "playLocalSound consumes the shared level seed even without an audio sink"
        );
    }

    #[test]
    fn fluid_animate_effects_preserve_vanilla_sound_particle_and_random_order() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let water_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(64) == 0)
            .expect("deterministic flowing-water sound seed");
        let mut water_random = LevelEventSoundRandomState::with_seed(water_seed);
        let mut water_spawns = Vec::new();
        let water_sounds = append_water_fluid_animate_effects(
            pos,
            TerrainFluidState::new(TerrainFluidKind::Water, 7, false),
            &mut water_random,
            &mut water_spawns,
        );
        let mut expected_water = LevelEventSoundRandomState::with_seed(water_seed);
        assert_eq!(expected_water.next_int_bound(64), 0);
        assert_eq!(
            water_sounds,
            [
                Some((
                    SOUND_EVENT_WATER_AMBIENT,
                    [-3.5, 12.5, 30.5],
                    expected_water.next_float() * 0.25 + 0.75,
                    expected_water.next_float() + 0.5,
                )),
                None
            ]
        );
        assert!(water_spawns.is_empty());
        assert_eq!(water_random.next_long(), expected_water.next_long());

        let lava_pop_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                if random.next_int_bound(100) != 0 {
                    return false;
                }
                let _ = random.next_double();
                let _ = random.next_double();
                let _ = random.next_float();
                let _ = random.next_float();
                random.next_int_bound(200) != 0
            })
            .expect("deterministic lava pop-only seed");
        let mut lava_pop_random = LevelEventSoundRandomState::with_seed(lava_pop_seed);
        let mut lava_pop_spawns = Vec::new();
        let lava_pop_sounds = append_lava_fluid_animate_effects(
            pos,
            true,
            &mut lava_pop_random,
            &mut lava_pop_spawns,
        );
        let mut expected_lava_pop = LevelEventSoundRandomState::with_seed(lava_pop_seed);
        assert_eq!(expected_lava_pop.next_int_bound(100), 0);
        let expected_pop_position = [
            -4.0 + expected_lava_pop.next_double(),
            13.0,
            30.0 + expected_lava_pop.next_double(),
        ];
        assert_eq!(
            lava_pop_sounds,
            [
                Some((
                    SOUND_EVENT_LAVA_POP,
                    expected_pop_position,
                    0.2 + expected_lava_pop.next_float() * 0.2,
                    0.9 + expected_lava_pop.next_float() * 0.15,
                )),
                None
            ]
        );
        assert_ne!(expected_lava_pop.next_int_bound(200), 0);
        assert_eq!(lava_pop_spawns.len(), 1);
        assert_eq!(lava_pop_spawns[0].particle.resource_id, PARTICLE_LAVA);
        assert_eq!(lava_pop_spawns[0].position, expected_pop_position);
        assert_eq!(lava_pop_spawns[0].velocity, [0.0; 3]);
        assert!(!lava_pop_spawns[0].always_show);
        assert_eq!(lava_pop_random.next_long(), expected_lava_pop.next_long());

        let lava_ambient_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                random.next_int_bound(100) != 0 && random.next_int_bound(200) == 0
            })
            .expect("deterministic lava ambient-only seed");
        let mut lava_ambient_random = LevelEventSoundRandomState::with_seed(lava_ambient_seed);
        let mut lava_ambient_spawns = Vec::new();
        let lava_ambient_sounds = append_lava_fluid_animate_effects(
            pos,
            true,
            &mut lava_ambient_random,
            &mut lava_ambient_spawns,
        );
        let mut expected_lava_ambient = LevelEventSoundRandomState::with_seed(lava_ambient_seed);
        assert_ne!(expected_lava_ambient.next_int_bound(100), 0);
        assert_eq!(expected_lava_ambient.next_int_bound(200), 0);
        assert_eq!(
            lava_ambient_sounds,
            [
                Some((
                    SOUND_EVENT_LAVA_AMBIENT,
                    [-4.0, 12.0, 30.0],
                    0.2 + expected_lava_ambient.next_float() * 0.2,
                    0.9 + expected_lava_ambient.next_float() * 0.15,
                )),
                None
            ]
        );
        assert!(lava_ambient_spawns.is_empty());
        assert_eq!(
            lava_ambient_random.next_long(),
            expected_lava_ambient.next_long()
        );

        let mut covered_random = LevelEventSoundRandomState::with_seed(lava_pop_seed);
        let mut expected_covered = LevelEventSoundRandomState::with_seed(lava_pop_seed);
        let mut covered_spawns = Vec::new();
        assert!(append_lava_fluid_animate_effects(
            pos,
            false,
            &mut covered_random,
            &mut covered_spawns,
        )
        .iter()
        .all(Option::is_none));
        assert!(covered_spawns.is_empty());
        assert_eq!(covered_random.next_long(), expected_covered.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut sound_random = LevelEventSoundRandomState::with_seed(water_seed);
        let mut audio = None;
        let mut spawns = Vec::new();
        runtime.animate_fluid(
            &WorldStore::default(),
            pos,
            TerrainFluidState::new(TerrainFluidKind::Water, 7, false),
            &mut sound_random,
            &mut audio,
            &mut spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "playLocalSound consumes the shared level seed even without an audio sink"
        );
    }

    #[test]
    fn furnace_family_animate_effects_preserve_lit_direction_and_random_order() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let sound_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_double() < 0.1)
            .expect("deterministic furnace sound seed");

        for (kind, facing, sound_id, vertical_range, particle_ids) in [
            (
                FurnaceAnimateKind::Furnace,
                "east",
                SOUND_EVENT_FURNACE_FIRE_CRACKLE,
                6.0 / 16.0,
                &[PARTICLE_SMOKE, PARTICLE_FLAME][..],
            ),
            (
                FurnaceAnimateKind::BlastFurnace,
                "south",
                SOUND_EVENT_BLAST_FURNACE_FIRE_CRACKLE,
                9.0 / 16.0,
                &[PARTICLE_SMOKE][..],
            ),
            (
                FurnaceAnimateKind::Smoker,
                "north",
                SOUND_EVENT_SMOKER_SMOKE,
                0.0,
                &[PARTICLE_SMOKE][..],
            ),
        ] {
            let properties = BTreeMap::from([
                ("facing".to_string(), facing.to_string()),
                ("lit".to_string(), "true".to_string()),
            ]);
            let mut actual = LevelEventSoundRandomState::with_seed(sound_seed);
            let mut expected = LevelEventSoundRandomState::with_seed(sound_seed);
            let mut spawns = Vec::new();

            let sound =
                append_furnace_animate_effects(pos, kind, &properties, &mut actual, &mut spawns);

            assert!(expected.next_double() < 0.1);
            assert_eq!(sound, Some(sound_id));
            let expected_position = if kind == FurnaceAnimateKind::Smoker {
                [-3.5, 13.1, 30.5]
            } else {
                let side_offset = expected.next_double() * 0.6 - 0.3;
                let vertical_offset = expected.next_double() * vertical_range;
                match facing {
                    "east" => [-3.5 + 0.52, 12.0 + vertical_offset, 30.5 + side_offset],
                    "south" => [-3.5 + side_offset, 12.0 + vertical_offset, 30.5 + 0.52],
                    _ => unreachable!("test uses horizontal furnace facings"),
                }
            };
            assert_eq!(
                spawns
                    .iter()
                    .map(|spawn| spawn.particle.resource_id.as_str())
                    .collect::<Vec<_>>(),
                particle_ids
            );
            assert!(spawns.iter().all(|spawn| {
                spawn.position == expected_position
                    && spawn.velocity == [0.0; 3]
                    && !spawn.always_show
            }));
            assert_eq!(actual.next_long(), expected.next_long());
        }

        for (facing, expected_fixed_axis) in [
            ("north", ("z", 30.5 - 0.52)),
            ("south", ("z", 30.5 + 0.52)),
            ("west", ("x", -3.5 - 0.52)),
            ("east", ("x", -3.5 + 0.52)),
        ] {
            let properties = BTreeMap::from([
                ("facing".to_string(), facing.to_string()),
                ("lit".to_string(), "true".to_string()),
            ]);
            let mut random = LevelEventSoundRandomState::with_seed(82);
            let mut spawns = Vec::new();
            let _ = append_furnace_animate_effects(
                pos,
                FurnaceAnimateKind::Furnace,
                &properties,
                &mut random,
                &mut spawns,
            );
            let position = spawns[0].position;
            match expected_fixed_axis.0 {
                "x" => assert_eq!(position[0], expected_fixed_axis.1),
                "z" => assert_eq!(position[2], expected_fixed_axis.1),
                _ => unreachable!("test uses x/z axes"),
            }
        }

        let unlit = BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("lit".to_string(), "false".to_string()),
        ]);
        let mut actual_unlit = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut expected_unlit = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut unlit_spawns = Vec::new();
        assert_eq!(
            append_furnace_animate_effects(
                pos,
                FurnaceAnimateKind::Furnace,
                &unlit,
                &mut actual_unlit,
                &mut unlit_spawns,
            ),
            None
        );
        assert!(unlit_spawns.is_empty());
        assert_eq!(actual_unlit.next_long(), expected_unlit.next_long());

        let lit = BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("lit".to_string(), "true".to_string()),
        ]);
        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut animate_random = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut audio = None;
        let mut spawns = Vec::new();
        runtime.animate_furnace_block(
            &WorldStore::default(),
            pos,
            FurnaceAnimateKind::Furnace,
            &lit,
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "playLocalSound consumes the shared level seed even without an audio sink"
        );
    }

    #[test]
    fn campfire_animate_effects_preserve_variant_particle_and_random_order() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let effect_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                if random.next_int_bound(10) != 0 {
                    return false;
                }
                let _ = random.next_float();
                let _ = random.next_float();
                random.next_int_bound(5) == 0
            })
            .expect("deterministic campfire sound and particle seed");
        let lit = BTreeMap::from([("lit".to_string(), "true".to_string())]);
        let mut actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut spawns = Vec::new();

        let sound = append_campfire_animate_effects(pos, true, &lit, &mut actual, &mut spawns);

        assert_eq!(expected.next_int_bound(10), 0);
        let expected_sound = (
            0.5 + expected.next_float(),
            expected.next_float() * 0.7 + 0.6,
        );
        assert_eq!(sound, Some(expected_sound));
        assert_eq!(expected.next_int_bound(5), 0);
        assert_eq!(expected.next_int_bound(1), 0);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_LAVA);
        assert_eq!(spawns[0].position, [-3.5, 12.5, 30.5]);
        assert_eq!(
            spawns[0].velocity,
            [
                f64::from(expected.next_float() / 2.0),
                5.0e-5,
                f64::from(expected.next_float() / 2.0),
            ]
        );
        assert!(!spawns[0].always_show);
        assert_eq!(actual.next_long(), expected.next_long());

        let mut soul_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut soul_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut soul_spawns = Vec::new();
        let soul_sound =
            append_campfire_animate_effects(pos, false, &lit, &mut soul_actual, &mut soul_spawns);
        assert_eq!(soul_expected.next_int_bound(10), 0);
        assert_eq!(
            soul_sound,
            Some((
                0.5 + soul_expected.next_float(),
                soul_expected.next_float() * 0.7 + 0.6,
            ))
        );
        assert!(soul_spawns.is_empty());
        assert_eq!(
            soul_actual.next_long(),
            soul_expected.next_long(),
            "soul campfire skips the spawnParticles random branch"
        );

        let unlit = BTreeMap::from([("lit".to_string(), "false".to_string())]);
        let mut unlit_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut unlit_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut unlit_spawns = Vec::new();
        assert_eq!(
            append_campfire_animate_effects(
                pos,
                true,
                &unlit,
                &mut unlit_actual,
                &mut unlit_spawns,
            ),
            None
        );
        assert!(unlit_spawns.is_empty());
        assert_eq!(unlit_actual.next_long(), unlit_expected.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut animate_random = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut audio = None;
        let mut runtime_spawns = Vec::new();
        runtime.animate_campfire(
            &WorldStore::default(),
            pos,
            true,
            &lit,
            &mut animate_random,
            &mut audio,
            &mut runtime_spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "campfire playLocalSound consumes the shared level seed without an audio sink"
        );
    }

    #[test]
    fn fire_animate_effects_preserve_support_neighbor_variant_and_random_order() {
        let mut world = world_with_ambient_particles();
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let below = BlockPos { y: 31, ..pos };
        let west = BlockPos { x: 7, ..pos };
        let above = BlockPos { y: 33, ..pos };
        let sound_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(24) == 0)
            .expect("deterministic fire ambient sound seed");

        set_block(&mut world, below, block_state_id("minecraft:stone", &[]));
        let mut supported_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut supported_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut supported_spawns = Vec::new();
        let supported_sound = append_fire_animate_effects(
            &world,
            pos,
            false,
            &mut supported_actual,
            &mut supported_spawns,
        );
        assert_eq!(supported_expected.next_int_bound(24), 0);
        assert_eq!(
            supported_sound,
            Some((
                1.0 + supported_expected.next_float(),
                supported_expected.next_float() * 0.7 + 0.3,
            ))
        );
        let expected_supported_positions = (0..3)
            .map(|_| {
                [
                    8.0 + supported_expected.next_double(),
                    32.0 + supported_expected.next_double() * 0.5 + 0.5,
                    8.0 + supported_expected.next_double(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(
            supported_spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_supported_positions
        );
        assert!(supported_spawns.iter().all(|spawn| {
            spawn.particle.resource_id == PARTICLE_LARGE_SMOKE
                && spawn.velocity == [0.0; 3]
                && !spawn.always_show
        }));
        assert_eq!(supported_actual.next_long(), supported_expected.next_long());

        set_block(&mut world, below, 0);
        let oak_planks = block_state_id("minecraft:oak_planks", &[]);
        set_block(&mut world, west, oak_planks);
        set_block(&mut world, above, oak_planks);
        let mut neighbor_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut neighbor_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut neighbor_spawns = Vec::new();
        let neighbor_sound = append_fire_animate_effects(
            &world,
            pos,
            false,
            &mut neighbor_actual,
            &mut neighbor_spawns,
        );
        assert_eq!(neighbor_expected.next_int_bound(24), 0);
        assert_eq!(
            neighbor_sound,
            Some((
                1.0 + neighbor_expected.next_float(),
                neighbor_expected.next_float() * 0.7 + 0.3,
            ))
        );
        let mut expected_neighbor_positions = Vec::new();
        for _ in 0..2 {
            expected_neighbor_positions.push([
                8.0 + neighbor_expected.next_double() * 0.1,
                32.0 + neighbor_expected.next_double(),
                8.0 + neighbor_expected.next_double(),
            ]);
        }
        for _ in 0..2 {
            expected_neighbor_positions.push([
                8.0 + neighbor_expected.next_double(),
                33.0 - neighbor_expected.next_double() * 0.1,
                8.0 + neighbor_expected.next_double(),
            ]);
        }
        assert_eq!(
            neighbor_spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_neighbor_positions
        );
        assert_eq!(neighbor_actual.next_long(), neighbor_expected.next_long());

        let mut soul_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut soul_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut soul_spawns = Vec::new();
        let soul_sound =
            append_fire_animate_effects(&world, pos, true, &mut soul_actual, &mut soul_spawns);
        assert_eq!(soul_expected.next_int_bound(24), 0);
        assert_eq!(
            soul_sound,
            Some((
                1.0 + soul_expected.next_float(),
                soul_expected.next_float() * 0.7 + 0.3,
            ))
        );
        let expected_soul_positions = (0..3)
            .map(|_| {
                [
                    8.0 + soul_expected.next_double(),
                    32.0 + soul_expected.next_double() * 0.5 + 0.5,
                    8.0 + soul_expected.next_double(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(
            soul_spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_soul_positions
        );
        assert_eq!(soul_actual.next_long(), soul_expected.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut animate_random = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut audio = None;
        let mut runtime_spawns = Vec::new();
        runtime.animate_fire(
            &world,
            pos,
            false,
            &mut animate_random,
            &mut audio,
            &mut runtime_spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "fire playLocalSound consumes the shared level seed without an audio sink"
        );
    }

    #[test]
    fn portal_block_animate_effects_preserve_state_axis_and_random_order() {
        let mut world = world_with_ambient_particles();
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let sound_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(100) == 0)
            .expect("deterministic portal ambient sound seed");

        let charged = BTreeMap::from([("charges".to_string(), "1".to_string())]);
        let mut anchor_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut anchor_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut anchor_spawns = Vec::new();
        assert!(append_respawn_anchor_animate_effects(
            pos,
            &charged,
            &mut anchor_actual,
            &mut anchor_spawns,
        ));
        assert_eq!(anchor_expected.next_int_bound(100), 0);
        assert_eq!(anchor_spawns.len(), 1);
        assert_eq!(
            anchor_spawns[0].particle.resource_id,
            PARTICLE_REVERSE_PORTAL
        );
        assert_eq!(
            anchor_spawns[0].position,
            [
                8.5 + (0.5 - anchor_expected.next_double()),
                33.0,
                8.5 + (0.5 - anchor_expected.next_double()),
            ]
        );
        assert_eq!(
            anchor_spawns[0].velocity,
            [0.0, f64::from(anchor_expected.next_float()) * 0.04, 0.0]
        );
        assert_eq!(anchor_actual.next_long(), anchor_expected.next_long());

        let uncharged = BTreeMap::from([("charges".to_string(), "0".to_string())]);
        let mut uncharged_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut uncharged_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut uncharged_spawns = Vec::new();
        assert!(!append_respawn_anchor_animate_effects(
            pos,
            &uncharged,
            &mut uncharged_actual,
            &mut uncharged_spawns,
        ));
        assert!(uncharged_spawns.is_empty());
        assert_eq!(uncharged_actual.next_long(), uncharged_expected.next_long());

        set_block(
            &mut world,
            pos,
            block_state_id("minecraft:nether_portal", &[("axis", "x")]),
        );
        let expected_portal_spawns = |random: &mut LevelEventSoundRandomState, pin_x: bool| {
            (0..4)
                .map(|_| {
                    let mut position = [
                        8.0 + random.next_double(),
                        32.0 + random.next_double(),
                        8.0 + random.next_double(),
                    ];
                    let mut velocity = [
                        (f64::from(random.next_float()) - 0.5) * 0.5,
                        (f64::from(random.next_float()) - 0.5) * 0.5,
                        (f64::from(random.next_float()) - 0.5) * 0.5,
                    ];
                    let flip = random.next_int_bound(2) * 2 - 1;
                    if pin_x {
                        position[0] = 8.5 + 0.25 * f64::from(flip);
                        velocity[0] = f64::from(random.next_float() * 2.0 * flip as f32);
                    } else {
                        position[2] = 8.5 + 0.25 * f64::from(flip);
                        velocity[2] = f64::from(random.next_float() * 2.0 * flip as f32);
                    }
                    (position, velocity)
                })
                .collect::<Vec<_>>()
        };

        let mut portal_x_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut portal_x_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut portal_x_spawns = Vec::new();
        let portal_x_pitch = append_nether_portal_animate_effects(
            &world,
            pos,
            &mut portal_x_actual,
            &mut portal_x_spawns,
        );
        assert_eq!(portal_x_expected.next_int_bound(100), 0);
        assert_eq!(
            portal_x_pitch,
            Some(portal_x_expected.next_float() * 0.4 + 0.8)
        );
        let expected_x = expected_portal_spawns(&mut portal_x_expected, true);
        assert_eq!(
            portal_x_spawns
                .iter()
                .map(|spawn| (spawn.position, spawn.velocity))
                .collect::<Vec<_>>(),
            expected_x
        );
        assert!(portal_x_spawns
            .iter()
            .all(|spawn| { spawn.particle.resource_id == PARTICLE_PORTAL && !spawn.always_show }));
        assert_eq!(portal_x_actual.next_long(), portal_x_expected.next_long());

        set_block(
            &mut world,
            BlockPos { x: 7, ..pos },
            block_state_id("minecraft:nether_portal", &[("axis", "x")]),
        );
        let mut portal_z_actual = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut portal_z_expected = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut portal_z_spawns = Vec::new();
        let portal_z_pitch = append_nether_portal_animate_effects(
            &world,
            pos,
            &mut portal_z_actual,
            &mut portal_z_spawns,
        );
        assert_eq!(portal_z_expected.next_int_bound(100), 0);
        assert_eq!(
            portal_z_pitch,
            Some(portal_z_expected.next_float() * 0.4 + 0.8)
        );
        let expected_z = expected_portal_spawns(&mut portal_z_expected, false);
        assert_eq!(
            portal_z_spawns
                .iter()
                .map(|spawn| (spawn.position, spawn.velocity))
                .collect::<Vec<_>>(),
            expected_z
        );
        assert_eq!(portal_z_actual.next_long(), portal_z_expected.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut animate_random = LevelEventSoundRandomState::with_seed(sound_seed);
        let mut audio = None;
        let mut runtime_spawns = Vec::new();
        runtime.animate_respawn_anchor(
            &world,
            pos,
            &charged,
            &mut animate_random,
            &mut audio,
            &mut runtime_spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "respawn-anchor playLocalSound consumes the shared level seed without an audio sink"
        );
    }

    #[test]
    fn dried_ghast_animate_effects_preserve_waterlogging_support_and_random_order() {
        let mut world = world_with_ambient_particles();
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let below = BlockPos { y: 31, ..pos };
        let effect_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                random.next_int_bound(40) == 0 && random.next_int_bound(6) == 0
            })
            .expect("deterministic dried-ghast sound and particle seed");

        set_block(
            &mut world,
            below,
            block_state_id("minecraft:soul_sand", &[]),
        );
        let mut dry_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut dry_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut dry_spawns = Vec::new();
        assert_eq!(
            append_dried_ghast_animate_effects(
                &world,
                pos,
                false,
                &mut dry_actual,
                &mut dry_spawns,
            ),
            Some(SOUND_EVENT_DRIED_GHAST_AMBIENT)
        );
        assert_eq!(dry_expected.next_int_bound(40), 0);
        assert_eq!(dry_expected.next_int_bound(6), 0);
        assert_eq!(dry_spawns.len(), 1);
        assert_eq!(dry_spawns[0].particle.resource_id, PARTICLE_WHITE_SMOKE);
        assert_eq!(dry_spawns[0].position, [8.5, 32.5, 8.5]);
        assert_eq!(dry_spawns[0].velocity, [0.0, 0.02, 0.0]);
        assert!(!dry_spawns[0].always_show);
        assert_eq!(dry_actual.next_long(), dry_expected.next_long());

        set_block(&mut world, below, block_state_id("minecraft:stone", &[]));
        let mut unsupported_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut unsupported_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut unsupported_spawns = Vec::new();
        assert_eq!(
            append_dried_ghast_animate_effects(
                &world,
                pos,
                false,
                &mut unsupported_actual,
                &mut unsupported_spawns,
            ),
            None
        );
        assert_eq!(unsupported_expected.next_int_bound(40), 0);
        assert_eq!(unsupported_expected.next_int_bound(6), 0);
        assert_eq!(unsupported_spawns.len(), 1);
        assert_eq!(
            unsupported_spawns[0].particle.resource_id,
            PARTICLE_WHITE_SMOKE
        );
        assert_eq!(
            unsupported_actual.next_long(),
            unsupported_expected.next_long()
        );

        let mut water_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut water_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut water_spawns = Vec::new();
        assert_eq!(
            append_dried_ghast_animate_effects(
                &world,
                pos,
                true,
                &mut water_actual,
                &mut water_spawns,
            ),
            Some(SOUND_EVENT_DRIED_GHAST_AMBIENT_WATER)
        );
        assert_eq!(water_expected.next_int_bound(40), 0);
        assert_eq!(water_expected.next_int_bound(6), 0);
        let expected_position = [
            8.5 + f64::from((water_expected.next_float() * 2.0 - 1.0) / 3.0),
            32.9,
            8.5 + f64::from((water_expected.next_float() * 2.0 - 1.0) / 3.0),
        ];
        let expected_velocity = [0.0, f64::from(water_expected.next_float()), 0.0];
        assert_eq!(water_spawns.len(), 1);
        assert_eq!(
            water_spawns[0].particle.resource_id,
            PARTICLE_HAPPY_VILLAGER
        );
        assert_eq!(water_spawns[0].position, expected_position);
        assert_eq!(water_spawns[0].velocity, expected_velocity);
        assert!(!water_spawns[0].always_show);
        assert_eq!(water_actual.next_long(), water_expected.next_long());

        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(42),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut expected_level_random = LevelEventSoundRandomState::with_seed(42);
        let _ = expected_level_random.next_long();
        let mut animate_random = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut audio = None;
        let mut runtime_spawns = Vec::new();
        runtime.animate_dried_ghast(
            &world,
            pos,
            true,
            &mut animate_random,
            &mut audio,
            &mut runtime_spawns,
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long(),
            "dried-ghast playLocalSound consumes the shared level seed without an audio sink"
        );
    }

    #[test]
    fn pointed_dripstone_animate_effects_preserve_fluid_scan_threshold_and_random_order() {
        let mut world = world_with_ambient_particles();
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let tip_properties = BTreeMap::from([
            ("thickness".to_string(), "tip".to_string()),
            ("vertical_direction".to_string(), "down".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]);
        let tip_id = block_state_id(
            "minecraft:pointed_dripstone",
            &[
                ("thickness", "tip"),
                ("vertical_direction", "down"),
                ("waterlogged", "false"),
            ],
        );
        let body_id = block_state_id(
            "minecraft:pointed_dripstone",
            &[
                ("thickness", "middle"),
                ("vertical_direction", "down"),
                ("waterlogged", "false"),
            ],
        );
        let waterlogged_tip_id = block_state_id(
            "minecraft:pointed_dripstone",
            &[
                ("thickness", "tip"),
                ("vertical_direction", "down"),
                ("waterlogged", "true"),
            ],
        );
        let stone_id = block_state_id("minecraft:stone", &[]);
        let water_id = block_state_id("minecraft:water", &[("level", "0")]);
        let lava_id = block_state_id("minecraft:lava", &[("level", "0")]);
        let mud_id = block_state_id("minecraft:mud", &[]);
        let fluid_seed = (0..100_000)
            .find(|seed| {
                let value = LevelEventSoundRandomState::with_seed(*seed).next_float();
                (0.02..=0.12).contains(&value)
            })
            .expect("deterministic fluid-backed dripstone particle seed");
        let rare_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.02)
            .expect("deterministic empty-fluid dripstone particle seed");
        let no_effect_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() > 0.12)
            .expect("deterministic dripstone no-effect seed");

        set_block(&mut world, pos, tip_id);
        set_block(&mut world, BlockPos { y: 33, ..pos }, body_id);
        set_block(&mut world, BlockPos { y: 34, ..pos }, stone_id);
        set_block(&mut world, BlockPos { y: 35, ..pos }, water_id);
        let mut water_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut water_expected = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut water_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut water_actual,
            &mut water_spawns,
        );
        assert!((0.02..=0.12).contains(&water_expected.next_float()));
        assert_eq!(water_spawns.len(), 1);
        assert_eq!(
            water_spawns[0].particle.resource_id,
            "minecraft:dripping_dripstone_water"
        );
        assert_eq!(
            water_spawns[0].position,
            pointed_dripstone_particle_position(pos)
        );
        assert_eq!(water_spawns[0].velocity, [0.0; 3]);
        assert!(!water_spawns[0].always_show);
        assert_eq!(water_actual.next_long(), water_expected.next_long());

        set_block(&mut world, BlockPos { y: 35, ..pos }, lava_id);
        let mut lava_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut lava_expected = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut lava_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut lava_actual,
            &mut lava_spawns,
        );
        let _ = lava_expected.next_float();
        assert_eq!(lava_spawns.len(), 1);
        assert_eq!(
            lava_spawns[0].particle.resource_id,
            "minecraft:dripping_dripstone_lava"
        );
        assert_eq!(lava_actual.next_long(), lava_expected.next_long());

        set_block(&mut world, BlockPos { y: 35, ..pos }, mud_id);
        let mut mud_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut mud_expected = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut mud_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut mud_actual,
            &mut mud_spawns,
        );
        let _ = mud_expected.next_float();
        assert_eq!(mud_spawns.len(), 1);
        assert_eq!(
            mud_spawns[0].particle.resource_id,
            "minecraft:dripping_dripstone_water"
        );
        assert_eq!(mud_actual.next_long(), mud_expected.next_long());

        set_block(&mut world, BlockPos { y: 35, ..pos }, 0);
        let mut common_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut common_expected = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut common_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut common_actual,
            &mut common_spawns,
        );
        let common_value = common_expected.next_float();
        assert!((0.02..=0.12).contains(&common_value));
        assert!(common_spawns.is_empty());
        assert_eq!(common_actual.next_long(), common_expected.next_long());

        let mut rare_actual = LevelEventSoundRandomState::with_seed(rare_seed);
        let mut rare_expected = LevelEventSoundRandomState::with_seed(rare_seed);
        let mut rare_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut rare_actual,
            &mut rare_spawns,
        );
        assert!(rare_expected.next_float() < 0.02);
        assert_eq!(rare_spawns.len(), 1);
        assert_eq!(
            rare_spawns[0].particle.resource_id,
            "minecraft:dripping_dripstone_water"
        );
        assert_eq!(rare_actual.next_long(), rare_expected.next_long());

        let mut no_effect_actual = LevelEventSoundRandomState::with_seed(no_effect_seed);
        let mut no_effect_expected = LevelEventSoundRandomState::with_seed(no_effect_seed);
        let mut no_effect_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut no_effect_actual,
            &mut no_effect_spawns,
        );
        assert!(no_effect_expected.next_float() > 0.12);
        assert!(no_effect_spawns.is_empty());
        assert_eq!(
            no_effect_actual.next_long(),
            no_effect_expected.next_long(),
            "eligible states always consume exactly one threshold draw"
        );

        let waterlogged_properties = BTreeMap::from([
            ("thickness".to_string(), "tip".to_string()),
            ("vertical_direction".to_string(), "down".to_string()),
            ("waterlogged".to_string(), "true".to_string()),
        ]);
        set_block(&mut world, pos, waterlogged_tip_id);
        let mut waterlogged_actual = LevelEventSoundRandomState::with_seed(rare_seed);
        let mut waterlogged_expected = LevelEventSoundRandomState::with_seed(rare_seed);
        let mut waterlogged_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &waterlogged_properties,
            &mut waterlogged_actual,
            &mut waterlogged_spawns,
        );
        assert!(waterlogged_spawns.is_empty());
        assert_eq!(
            waterlogged_actual.next_long(),
            waterlogged_expected.next_long(),
            "ineligible states consume no animate random"
        );

        set_block(&mut world, pos, tip_id);
        for y in 33..42 {
            set_block(&mut world, BlockPos { y, ..pos }, body_id);
        }
        set_block(&mut world, BlockPos { y: 42, ..pos }, stone_id);
        set_block(&mut world, BlockPos { y: 43, ..pos }, water_id);
        let mut boundary_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut boundary_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut boundary_actual,
            &mut boundary_spawns,
        );
        assert_eq!(
            boundary_spawns.len(),
            1,
            "the first non-dripstone root at search step ten is included"
        );

        set_block(&mut world, BlockPos { y: 42, ..pos }, body_id);
        set_block(&mut world, BlockPos { y: 43, ..pos }, stone_id);
        set_block(&mut world, BlockPos { y: 44, ..pos }, water_id);
        let mut beyond_actual = LevelEventSoundRandomState::with_seed(fluid_seed);
        let mut beyond_spawns = Vec::new();
        append_pointed_dripstone_animate_effect(
            &world,
            pos,
            &tip_properties,
            &mut beyond_actual,
            &mut beyond_spawns,
        );
        assert!(
            beyond_spawns.is_empty(),
            "a root beyond Vanilla's exclusive maxSteps bound is ignored"
        );
    }

    #[test]
    fn fluid_drips_use_vanilla_collision_branches_tags_and_level_random_order() {
        let mut world = world_with_ambient_particles();
        let fluid_pos = BlockPos { x: 8, y: 32, z: 8 };
        let below_pos = BlockPos { y: 31, ..fluid_pos };
        let two_below_pos = BlockPos { y: 30, ..fluid_pos };
        let water = block_state_id("minecraft:water", &[("level", "0")]);
        let bottom_slab = block_state_id(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "false")],
        );
        let top_slab = block_state_id(
            "minecraft:oak_slab",
            &[("type", "top"), ("waterlogged", "false")],
        );

        set_block(
            &mut world,
            fluid_pos,
            block_state_id("minecraft:mangrove_roots", &[("waterlogged", "true")]),
        );
        set_block(&mut world, below_pos, bottom_slab);
        let mut watertight_random = LevelEventSoundRandomState::with_seed(11);
        let mut expected_watertight = LevelEventSoundRandomState::with_seed(11);
        let mut spawns = Vec::new();
        append_fluid_drip_particle(
            &world,
            fluid_pos,
            TerrainFluidKind::Water,
            &mut watertight_random,
            &mut spawns,
        );
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_DRIPPING_WATER);
        assert_eq!(
            spawns[0].position,
            [
                8.0 + expected_watertight.next_double(),
                31.95,
                8.0 + expected_watertight.next_double(),
            ]
        );
        assert_eq!(
            watertight_random.next_long(),
            expected_watertight.next_long()
        );

        set_block(&mut world, fluid_pos, water);
        set_block(&mut world, below_pos, top_slab);
        let mut raised_bottom_random = LevelEventSoundRandomState::with_seed(12);
        let mut expected_raised_bottom = LevelEventSoundRandomState::with_seed(12);
        spawns.clear();
        append_fluid_drip_particle(
            &world,
            fluid_pos,
            TerrainFluidKind::Lava,
            &mut raised_bottom_random,
            &mut spawns,
        );
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_DRIPPING_LAVA);
        assert_eq!(
            spawns[0].position,
            [
                8.0 + expected_raised_bottom.next_double(),
                31.45,
                8.0 + expected_raised_bottom.next_double(),
            ]
        );
        assert_eq!(
            raised_bottom_random.next_long(),
            expected_raised_bottom.next_long()
        );

        set_block(
            &mut world,
            below_pos,
            block_state_id("minecraft:glass", &[]),
        );
        let mut impermeable_random = LevelEventSoundRandomState::with_seed(13);
        let mut expected_impermeable = LevelEventSoundRandomState::with_seed(13);
        spawns.clear();
        append_fluid_drip_particle(
            &world,
            fluid_pos,
            TerrainFluidKind::Water,
            &mut impermeable_random,
            &mut spawns,
        );
        assert!(spawns.is_empty());
        assert_eq!(
            impermeable_random.next_long(),
            expected_impermeable.next_long()
        );

        set_block(
            &mut world,
            below_pos,
            block_state_id("minecraft:stone", &[]),
        );
        set_block(&mut world, two_below_pos, 0);
        let mut underside_random = LevelEventSoundRandomState::with_seed(14);
        let mut expected_underside = LevelEventSoundRandomState::with_seed(14);
        spawns.clear();
        append_fluid_drip_particle(
            &world,
            fluid_pos,
            TerrainFluidKind::Water,
            &mut underside_random,
            &mut spawns,
        );
        assert_eq!(spawns.len(), 1);
        assert_eq!(
            spawns[0].position,
            [
                8.0 + expected_underside.next_double(),
                30.95,
                8.0 + expected_underside.next_double(),
            ]
        );
        assert_eq!(underside_random.next_long(), expected_underside.next_long());

        set_block(&mut world, below_pos, water);
        let mut submerged_random = LevelEventSoundRandomState::with_seed(15);
        let mut expected_submerged = LevelEventSoundRandomState::with_seed(15);
        spawns.clear();
        append_fluid_drip_particle(
            &world,
            fluid_pos,
            TerrainFluidKind::Water,
            &mut submerged_random,
            &mut spawns,
        );
        assert!(spawns.is_empty());
        assert_eq!(submerged_random.next_long(), expected_submerged.next_long());
    }

    #[test]
    fn firefly_light_gate_and_success_preserve_vanilla_random_draws() {
        let pos = BlockPos {
            x: 10,
            y: 20,
            z: -3,
        };
        let seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed).next_double() <= FIREFLY_SPAWN_CHANCE
            })
            .expect("successful deterministic firefly seed");
        let mut actual = LevelEventSoundRandomState::with_seed(seed);
        let mut spawns = Vec::new();

        append_firefly_spawn(&mut actual, pos, Some(13), &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(seed);
        assert!(expected.next_double() <= FIREFLY_SPAWN_CHANCE);
        let expected_position = [
            f64::from(pos.x) + expected.next_double() * FIREFLY_HORIZONTAL_RANGE
                - FIREFLY_HORIZONTAL_RANGE / 2.0,
            f64::from(pos.y) + expected.next_double() * FIREFLY_VERTICAL_RANGE,
            f64::from(pos.z) + expected.next_double() * FIREFLY_HORIZONTAL_RANGE
                - FIREFLY_HORIZONTAL_RANGE / 2.0,
        ];
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].position, expected_position);
        assert_eq!(actual.next_long(), expected.next_long());

        for brightness in [None, Some(14)] {
            let mut actual = LevelEventSoundRandomState::with_seed(seed);
            let mut untouched = LevelEventSoundRandomState::with_seed(seed);
            let mut spawns = Vec::new();
            append_firefly_spawn(&mut actual, pos, brightness, &mut spawns);
            assert!(spawns.is_empty());
            assert_eq!(actual.next_long(), untouched.next_long());
        }
    }

    #[test]
    fn collision_and_six_side_pale_log_queries_use_loaded_block_shapes_and_tag_membership() {
        let mut world = world_with_ambient_particles();
        let center = BlockPos { x: 8, y: 32, z: 8 };
        assert_eq!(
            world.sample_block_collision_shape_is_full_block(center),
            Some(false)
        );
        set_block(&mut world, center, block_state_id("minecraft:stone", &[]));
        assert_eq!(
            world.sample_block_collision_shape_is_full_block(center),
            Some(true)
        );
        assert!(!creaking_heart_is_surrounded_by_logs(&world, center));

        let pale_log = block_state_id("minecraft:pale_oak_log", &[("axis", "y")]);
        for (dx, dy, dz) in [
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
            (-1, 0, 0),
            (1, 0, 0),
        ] {
            set_block(
                &mut world,
                BlockPos {
                    x: center.x + dx,
                    y: center.y + dy,
                    z: center.z + dz,
                },
                pale_log,
            );
        }
        assert!(creaking_heart_is_surrounded_by_logs(&world, center));

        set_block(
            &mut world,
            BlockPos {
                x: center.x + 1,
                ..center
            },
            block_state_id("minecraft:oak_log", &[("axis", "y")]),
        );
        assert!(!creaking_heart_is_surrounded_by_logs(&world, center));
    }

    #[test]
    fn ambient_particles_observe_chunks_biomes_replacement_on_the_next_tick() {
        let mut world = world_with_ambient_particles();
        let player = player_pose([1.0, 32.0, 1.0]);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut audio = None;

        let initial = runtime.tick(&mut world, player, &mut audio);
        assert!(!initial.is_empty());
        assert!(initial
            .iter()
            .all(|spawn| spawn.particle.resource_id == "minecraft:ash"));

        world
            .apply_biome_update(ChunksBiomes {
                chunks: vec![ChunkBiomeData {
                    pos: ProtocolChunkPos { x: 0, z: 0 },
                    raw_biomes: [0, 1].repeat(world.dimension().section_count()),
                }],
            })
            .unwrap();
        let replaced = runtime.tick(&mut world, player, &mut audio);

        assert!(!replaced.is_empty());
        assert!(replaced
            .iter()
            .all(|spawn| spawn.particle.resource_id == "minecraft:white_ash"));
    }

    #[test]
    fn full_block_collision_suppresses_ambient_particles() {
        let stone = block_state_id("minecraft:stone", &[]);
        let mut world = world_with_ambient_particles_and_block_state(stone);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut audio = None;

        let spawns = runtime.tick(&mut world, player_pose([1.0, 32.0, 1.0]), &mut audio);

        assert!(spawns.is_empty());
    }

    #[test]
    fn environment_animate_ticks_after_movement_and_before_music_and_particle_intake() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let movement = advance
            .find("advance_player_input_for_client_ticks(")
            .expect("local player movement");
        let animate = advance
            .find("environment_animate.advance_for_ticks(")
            .expect("environment animate tick");
        let music = advance
            .find("music.advance_for_ticks(")
            .expect("music tick");
        let particle_intake = advance
            .find("submit_entity_client_tick_effects(")
            .expect("later client particle intake");

        assert!(movement < animate);
        assert!(animate < music);
        assert!(animate < particle_intake);
    }

    #[test]
    fn firefly_light_and_height_gates_match_vanilla_boundaries() {
        let light = bbb_world::TerrainLight { block: 6, sky: 12 };
        assert_eq!(light.block.max(light.sky.saturating_sub(4)), 8);
        assert_eq!(light.block.max(light.sky.saturating_sub(14)), 6);
        assert!(firefly_sound_height_allows(Some(20), 20));
        assert!(firefly_sound_height_allows(Some(19), 20));
        assert!(!firefly_sound_height_allows(Some(21), 20));
        assert!(!firefly_sound_height_allows(None, 20));
    }

    fn ambient_particle(resource_id: &str, probability: f32) -> ClientAmbientParticle {
        ClientAmbientParticle {
            particle: simple_client_particle_options(resource_id),
            probability_bits: probability.to_bits(),
        }
    }

    fn world_with_ambient_particles() -> WorldStore {
        world_with_ambient_particles_and_block_state(0)
    }

    fn world_with_ambient_particles_and_block_state(block_state_id: i32) -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension::default());
        world.begin_registry_configuration();
        world.record_registry_entries(
            "minecraft:worldgen/biome",
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:biome_a",
                    biome_with_ambient_particle_nbt("minecraft:ash"),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:biome_b",
                    biome_with_ambient_particle_nbt("minecraft:white_ash"),
                ),
            ],
        );
        for data in crate::runtime::minimal_non_empty_synchronized_registry_data() {
            if data.registry != "minecraft:worldgen/biome" {
                world.record_registry_data(data);
            }
        }
        world.finish_registry_configuration().unwrap();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["example:level".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "example:level".to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(player_pose([1.0, 32.0, 1.0]));
        world.insert_decoded_chunk(chunk_with_biome(world.dimension(), 0, block_state_id));
        world
    }

    fn biome_with_ambient_particle_nbt(resource_id: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_byte(&mut out, "has_precipitation", 1);
        write_nbt_float(&mut out, "temperature", 0.8);
        write_nbt_float(&mut out, "downfall", 0.4);
        out.push(10);
        write_nbt_utf(&mut out, "effects");
        write_nbt_string(&mut out, "water_color", "#3f76e4");
        out.push(0);
        out.push(10);
        write_nbt_utf(&mut out, "attributes");
        out.push(9);
        write_nbt_utf(&mut out, "minecraft:visual/ambient_particles");
        out.push(10);
        out.extend_from_slice(&1_i32.to_be_bytes());
        out.push(10);
        write_nbt_utf(&mut out, "particle");
        write_nbt_string(&mut out, "type", resource_id);
        out.push(0);
        write_nbt_float(&mut out, "probability", 1.0);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn chunk_with_biome(
        dimension: WorldDimension,
        biome_id: i32,
        block_state_id: i32,
    ) -> ChunkColumn {
        ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..dimension.section_count())
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        block_state_id,
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, biome_id),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: bbb_protocol::packets::BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        bbb_world::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("missing test block state {name} {properties:?}"))
    }

    fn player_pose(position: [f64; 3]) -> LocalPlayerPoseState {
        LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            ..LocalPlayerPoseState::default()
        }
    }

    fn write_nbt_utf(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn write_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        out.push(1);
        write_nbt_utf(out, name);
        out.push(value as u8);
    }

    fn write_nbt_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_nbt_utf(out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    fn write_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_utf(out, name);
        write_nbt_utf(out, value);
    }
}
