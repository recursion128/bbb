use bbb_protocol::environment_audio::SoundEventHolderValue as ClientSoundEventHolder;
mod block_entity_effects;

use std::collections::BTreeMap;

mod ambient_effects;
mod block_effects;
mod redstone_effects;
mod surface_effects;
use ambient_effects::*;
use block_effects::*;
use redstone_effects::*;
use surface_effects::*;

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::{
    block_name_has_falling_dust_animate_tick, block_name_is_air, block_name_is_candle,
    block_name_is_candle_cake, block_name_is_dead_bush_ambient, block_name_is_dry_grass_ambient,
    block_name_is_lightning_rod, block_name_is_pale_oak_log, block_name_is_terracotta,
    block_name_triggers_ambient_desert_dry_vegetation_block_sounds,
    block_name_triggers_ambient_desert_sand_block_sounds, leaf_animate_particle_profile,
    torch_animate_particle_profile, BlockPos, EnvironmentBiomeSampling, LeafAnimateParticleProfile,
    LeafParticleTint, LevelEventSoundRandomState, LocalPlayerPoseState, SoundEntityEventState,
    SoundEventState, TerrainFluidKind, TerrainFluidState, TorchAnimateParticleProfile, WorldStore,
};
use bbb_protocol::{
    environment_particles::AmbientParticle,
    heightmap_types::{
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
    },
    packets::{SoundSource, Vec3d as ProtocolVec3d},
};

use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        candle_wick_offsets, particle_utils_axis_sample, pointed_dripstone_drip_context,
        pointed_dripstone_particle_position, redstone_ore_particle_positions,
        redstone_ore_solid_render_faces, ClientParticle, EnvironmentParticleColorSource,
        EnvironmentParticleSpawn, ParticleBiomeSampler, ParticleTarget, ELECTRIC_SPARK_AXIS_RADIUS,
        REDSTONE_EFFECT_DIRECTION_STEPS, REDSTONE_PARTICLE_COLOR, SCULK_PARTICLE_COLOR,
    },
};
use bbb_client_world::particles::ParticleKind;

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
const FALLING_DUST_CHANCE_BOUND: i32 = 16;
const DESERT_SAND_SOUND_CHANCE_BOUND: i32 = 2100;
const DESERT_DRY_GRASS_SOUND_CHANCE_BOUND: i32 = 200;
const DESERT_DEAD_BUSH_SOUND_CHANCE_BOUND: i32 = 130;
const DESERT_DEAD_BUSH_BADLANDS_DECREASED_CHANCE_BOUND: i32 = 3;
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
const SOUND_EVENT_CANDLE_AMBIENT: &str = "minecraft:block.candle.ambient";
const SOUND_EVENT_SAND_IDLE: &str = "minecraft:block.sand.idle";
const SOUND_EVENT_DRY_GRASS_AMBIENT: &str = "minecraft:block.dry_grass.ambient";
const SOUND_EVENT_DEAD_BUSH_IDLE: &str = "minecraft:block.deadbush.idle";
const SOUND_EVENT_EYEBLOSSOM_IDLE: &str = "minecraft:block.eyeblossom.idle";
const SOUND_EVENT_PALE_HANGING_MOSS_IDLE: &str = "minecraft:block.pale_hanging_moss.idle";
const LIGHTNING_ROD_SPARK_CYCLE: i32 = 200;

const OPPOSITE_EFFECT_DIRECTION_INDICES: [usize; 6] = [1, 0, 3, 2, 5, 4];

type FluidAnimateSound = (&'static str, [f64; 3], f32, f32);
type FluidAnimateSounds = [Option<FluidAnimateSound>; 2];
type CandleAnimateSound = ([f64; 3], f32, f32);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedstoneWireSide {
    Up,
    Side,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedstoneTorchAnimateKind {
    Floor,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RedstoneWireDirection {
    property: &'static str,
    step: [i32; 3],
}

const REDSTONE_WIRE_DIRECTIONS: [RedstoneWireDirection; 4] = [
    RedstoneWireDirection {
        property: "north",
        step: [0, 0, -1],
    },
    RedstoneWireDirection {
        property: "east",
        step: [1, 0, 0],
    },
    RedstoneWireDirection {
        property: "south",
        step: [0, 0, 1],
    },
    RedstoneWireDirection {
        property: "west",
        step: [-1, 0, 0],
    },
];

/// Non-serializable owner of Vanilla `ClientLevel.animateTick` random cursors.
#[derive(Debug)]
pub(crate) struct EnvironmentAnimateRuntime {
    level_identity: Option<(u64, u64)>,
    level_random: LevelEventSoundRandomState,
    animate_seed_random: LevelEventSoundRandomState,
    /// Reused per-tick spawn list.
    spawns: Vec<EnvironmentParticleSpawn>,
}

impl Default for EnvironmentAnimateRuntime {
    fn default() -> Self {
        Self {
            level_identity: None,
            level_random: LevelEventSoundRandomState::with_seed(0),
            animate_seed_random: LevelEventSoundRandomState::with_seed(1),
            spawns: Vec::new(),
        }
    }
}

impl EnvironmentAnimateRuntime {
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &WorldStore,
        connection_generation: u64,
        running_level_ticks: u32,
        mut particles: Option<ParticleTarget<'_>>,
        audio: &mut Option<&mut dyn AudioEventSink>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        self.synchronize_level_identity(world, connection_generation);

        for _ in 0..running_level_ticks {
            let Some(player) = world.local_player_pose() else {
                continue;
            };
            self.tick(world, player, audio);
            if self.spawns.is_empty() {
                continue;
            }
            let Some(ParticleTarget { runtime, engine }) = particles.as_mut() else {
                continue;
            };
            let biome_sampler = EnvironmentWorldParticleBiomeSampler { world };
            runtime.spawn_environment_particles(
                engine,
                &self.spawns,
                Some([
                    player.position.x,
                    player.position.y + world.local_player_eye_height(player),
                    player.position.z,
                ]),
                Some(&biome_sampler),
                item_runtime,
            );
        }
    }

    fn synchronize_level_identity(&mut self, world: &WorldStore, connection_generation: u64) {
        let level_identity = (connection_generation, world.client_level_generation());
        if self.level_identity == Some(level_identity) {
            return;
        }
        self.level_identity = Some(level_identity);
        self.level_random = LevelEventSoundRandomState::with_seed(0);
        self.animate_seed_random = LevelEventSoundRandomState::with_seed(1);
    }

    fn tick(
        &mut self,
        world: &WorldStore,
        player: LocalPlayerPoseState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        let center = BlockPos {
            x: containing_block_coordinate(player.position.x),
            y: containing_block_coordinate(player.position.y),
            z: containing_block_coordinate(player.position.z),
        };
        let sky_darken = crate::runtime::entity_shadow_sky_darken(world);
        let marker_target = world.local_marker_particle_target_block();
        let mut animate_random =
            LevelEventSoundRandomState::with_seed(self.animate_seed_random.next_long());
        let mut spawns = std::mem::take(&mut self.spawns);
        spawns.clear();
        let mut ambient_particles = BTreeMap::new();
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            self.do_animate_tick(
                world,
                center,
                NEAR_SAMPLE_RADIUS,
                sky_darken,
                marker_target,
                &mut animate_random,
                audio,
                &mut ambient_particles,
                &mut spawns,
            );
            self.do_animate_tick(
                world,
                center,
                FAR_SAMPLE_RADIUS,
                sky_darken,
                marker_target,
                &mut animate_random,
                audio,
                &mut ambient_particles,
                &mut spawns,
            );
        }
        self.spawns = spawns;
    }

    #[allow(clippy::too_many_arguments)]
    fn do_animate_tick(
        &mut self,
        world: &WorldStore,
        center: BlockPos,
        radius: i32,
        sky_darken: u8,
        marker_target: Option<&str>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        ambient_particles: &mut BTreeMap<Option<i32>, Vec<AmbientParticle>>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) -> bool {
        let pos = sample_animate_position(&mut self.level_random, center, radius);
        let Some(block) = world.block_view(pos) else {
            return false;
        };
        let Some(block_name) = block.block_name else {
            return false;
        };

        let mut handled = true;
        match block_name {
            _ if !block.has_animate_tick => handled = false,
            "minecraft:brewing_stand" => {
                append_brewing_stand_animate_effect(pos, animate_random, spawns);
            }
            "minecraft:open_eyeblossom" => {
                self.animate_eyeblossom(world, pos, animate_random, audio);
            }
            "minecraft:closed_eyeblossom" => {}
            "minecraft:pale_hanging_moss" => {
                self.animate_hanging_moss(world, pos, animate_random, audio);
            }
            "minecraft:lever" => {
                append_lever_animate_effect(pos, block.block_properties, animate_random, spawns);
            }
            "minecraft:mycelium" => {
                append_mycelium_animate_effect(pos, animate_random, spawns);
            }
            "minecraft:wither_rose" => {
                append_wither_rose_animate_effects(pos, animate_random, spawns);
            }
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
                    block.block_properties,
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
                    block.block_properties,
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
                    block.block_properties,
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
                    block.block_properties,
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
                    block.block_properties,
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
                    block.block_properties,
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
                    block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:bee_nest" | "minecraft:beehive" => {
                append_beehive_animate_effect(
                    world,
                    pos,
                    block.block_properties,
                    animate_random,
                    &mut self.level_random,
                    spawns,
                );
            }
            "minecraft:spore_blossom" => {
                append_spore_blossom_animate_effects(world, pos, animate_random, spawns);
            }
            "minecraft:enchanting_table" => {
                append_enchanting_table_animate_effects(world, pos, animate_random, spawns);
            }
            "minecraft:end_rod" => {
                append_end_rod_animate_effect(pos, block.block_properties, animate_random, spawns);
            }
            "minecraft:wet_sponge" => {
                append_wet_sponge_animate_effect(world, pos, animate_random, spawns);
            }
            "minecraft:crying_obsidian" => {
                append_crying_obsidian_animate_effect(world, pos, animate_random, spawns);
            }
            "minecraft:redstone_torch" => {
                append_redstone_torch_animate_effect(
                    pos,
                    block.block_properties,
                    RedstoneTorchAnimateKind::Floor,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:redstone_wall_torch" => {
                append_redstone_torch_animate_effect(
                    pos,
                    block.block_properties,
                    RedstoneTorchAnimateKind::Wall,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:repeater" => {
                append_repeater_animate_effect(pos, block.block_properties, animate_random, spawns);
            }
            "minecraft:redstone_wire" => {
                append_redstone_wire_animate_effects(
                    pos,
                    block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
                append_redstone_ore_animate_effects(
                    world,
                    pos,
                    block.block_properties,
                    &mut self.level_random,
                    spawns,
                );
            }
            "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor" => {
                append_sculk_sensor_animate_effect(
                    pos,
                    block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            name if block_name_is_candle(name) => {
                self.animate_candle(
                    world,
                    pos,
                    block.block_properties,
                    false,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            name if block_name_is_candle_cake(name) => {
                self.animate_candle(
                    world,
                    pos,
                    block.block_properties,
                    true,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            name if block_name_is_lightning_rod(name) => append_lightning_rod_animate_effects(
                world,
                pos,
                block.block_properties,
                &mut self.level_random,
                spawns,
            ),
            "minecraft:ender_chest" => {
                append_ender_chest_animate_effects(pos, animate_random, spawns);
            }
            "minecraft:end_portal" => {
                append_end_portal_animate_effect(pos, animate_random, spawns);
            }
            "minecraft:end_gateway" => {
                if let Some(particle_count) = world.end_gateway_particle_amount(pos) {
                    append_end_gateway_animate_effects(pos, particle_count, animate_random, spawns);
                }
            }
            name if block_name_triggers_ambient_desert_sand_block_sounds(name) => {
                append_falling_dust_animate_effect(
                    world,
                    pos,
                    block.block_state_id,
                    animate_random,
                    spawns,
                );
                self.animate_desert_sand(world, pos, animate_random, audio);
            }
            name if block_name_is_dry_grass_ambient(name) => {
                self.animate_desert_dry_grass(world, pos, animate_random, audio);
            }
            name if block_name_is_dead_bush_ambient(name) => {
                self.animate_desert_dead_bush(world, pos, animate_random, audio);
            }
            name if block_name_has_falling_dust_animate_tick(name) => {
                append_falling_dust_animate_effect(
                    world,
                    pos,
                    block.block_state_id,
                    animate_random,
                    spawns,
                );
            }
            _ => {
                if let Some(profile) = torch_animate_particle_profile(block_name) {
                    append_torch_animate_effects(pos, block.block_properties, profile, spawns);
                } else if let Some(profile) = leaf_animate_particle_profile(block_name) {
                    append_leaves_animate_effects(world, pos, profile, animate_random, spawns);
                } else {
                    handled = false;
                }
            }
        }

        if let Some(fluid) = block.fluid {
            self.animate_fluid(world, pos, fluid, animate_random, audio, spawns);
            if self.level_random.next_int_bound(10) == 0 {
                append_fluid_drip_particle(world, pos, fluid.kind, &mut self.level_random, spawns);
            }
        }

        if marker_target == Some(block_name) {
            append_block_marker_animate_effect(pos, block.block_state_id, spawns);
        }

        if block.collision_shape_is_full_block() {
            return handled;
        }
        // Loaded blocks use the same direct quart biome as ambient attribute sampling.
        let particles = ambient_particles
            .entry(world.biome_id_at(pos))
            .or_insert_with(|| {
                world
                    .sampled_client_ambient_particles_at(block_position(pos))
                    .unwrap_or_default()
            });
        append_ambient_particle_spawns(
            &mut self.level_random,
            pos,
            particles,
            world.content_registry(),
            spawns,
        );
        handled
    }

    fn animate_desert_sand(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if !block_name_at(
            world,
            BlockPos {
                y: pos.y.saturating_add(1),
                ..pos
            },
        )
        .is_some_and(|name| name == "minecraft:air")
            || random.next_int_bound(DESERT_SAND_SOUND_CHANCE_BOUND) != 0
            || !should_play_ambient_desert_sand_sound(world, pos)
        {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_SAND_IDLE,
                SoundSource::Ambient,
                block_position(pos),
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_desert_dry_grass(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if random.next_int_bound(DESERT_DRY_GRASS_SOUND_CHANCE_BOUND) != 0
            || !supports_ambient_desert_dry_vegetation_sound(world, pos)
        {
            return;
        }
        let Some(entity_id) = world.local_player_id() else {
            return;
        };
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_entity_sound(
                &SoundEntityEventState {
                    sound: sound_holder_state(
                        world,
                        &ClientSoundEventHolder::Reference {
                            resource_id: SOUND_EVENT_DRY_GRASS_AMBIENT.to_string(),
                        },
                    ),
                    source: SoundSource::Ambient.as_str().to_string(),
                    entity_id,
                    volume: 1.0,
                    pitch: 1.0,
                    seed,
                    looping: false,
                    linear_attenuation: true,
                    tickable: false,
                },
                world
                    .local_player_pose()
                    .map(|pose| [pose.position.x, pose.position.y, pose.position.z]),
            );
        }
    }

    fn animate_desert_dead_bush(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if random.next_int_bound(DESERT_DEAD_BUSH_SOUND_CHANCE_BOUND) != 0 {
            return;
        }
        let below = BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        };
        let badlands_substrate = block_name_at(world, below)
            .is_some_and(|name| name == "minecraft:red_sand" || block_name_is_terracotta(&name));
        if badlands_substrate
            && random.next_int_bound(DESERT_DEAD_BUSH_BADLANDS_DECREASED_CHANCE_BOUND) != 0
        {
            return;
        }
        if !supports_ambient_desert_dry_vegetation_sound(world, pos) {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_DEAD_BUSH_IDLE,
                SoundSource::Ambient,
                block_position(pos),
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_eyeblossom(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if random.next_int_bound(700) != 0
            || block_name_at(
                world,
                BlockPos {
                    y: pos.y.saturating_sub(1),
                    ..pos
                },
            )
            .as_deref()
                != Some("minecraft:pale_moss_block")
        {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_EYEBLOSSOM_IDLE,
                SoundSource::Ambient,
                block_position(pos),
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_hanging_moss(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if random.next_int_bound(500) != 0
            || !block_name_at(
                world,
                BlockPos {
                    y: pos.y.saturating_add(1),
                    ..pos
                },
            )
            .as_deref()
            .is_some_and(|name| {
                name == "minecraft:pale_oak_leaves" || block_name_is_pale_oak_log(name)
            })
        {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_PALE_HANGING_MOSS_IDLE,
                SoundSource::Ambient,
                block_position(pos),
                1.0,
                1.0,
                seed,
            ));
        }
    }

    fn animate_firefly_bush(
        &mut self,
        world: &WorldStore,
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

    #[allow(clippy::too_many_arguments)]
    fn animate_candle(
        &mut self,
        world: &WorldStore,
        pos: BlockPos,
        properties: &BTreeMap<String, String>,
        on_cake: bool,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        for (position, volume, pitch) in
            append_candle_animate_effects(pos, properties, on_cake, animate_random, spawns)
        {
            let seed = self.level_random.next_long();
            if let Some(audio) = audio.as_deref_mut() {
                audio.play_positioned_sound(&local_block_sound(
                    world,
                    SOUND_EVENT_CANDLE_AMBIENT,
                    SoundSource::Blocks,
                    position,
                    volume,
                    pitch,
                    seed,
                ));
            }
        }
    }

    fn animate_creaking_heart(
        &mut self,
        world: &WorldStore,
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

struct EnvironmentWorldParticleBiomeSampler<'a> {
    world: &'a WorldStore,
}

impl ParticleBiomeSampler for EnvironmentWorldParticleBiomeSampler<'_> {
    fn biome_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.world.biome_id_at(pos)
    }
}
