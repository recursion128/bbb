mod block_entity_effects;

use std::collections::BTreeMap;

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    heightmap_types::{
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
    },
    packets::{SoundSource, Vec3d as ProtocolVec3d},
};
use bbb_world::{
    block_name_has_falling_dust_animate_tick, block_name_is_air, block_name_is_candle,
    block_name_is_candle_cake, block_name_is_dead_bush_ambient, block_name_is_dry_grass_ambient,
    block_name_is_lightning_rod, block_name_is_pale_oak_log, block_name_is_terracotta,
    block_name_triggers_ambient_desert_dry_vegetation_block_sounds,
    block_name_triggers_ambient_desert_sand_block_sounds, leaf_animate_particle_profile,
    torch_animate_particle_profile, BlockPos, ClientSoundEventHolder, EnvironmentBiomeSampling,
    LeafAnimateParticleProfile, LeafParticleTint, LevelEventSoundRandomState, LocalPlayerPoseState,
    SoundEntityEventState, SoundEventState, TerrainFluidKind, TerrainFluidState,
    TorchAnimateParticleProfile, WorldStore,
};

use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        block_client_particle_options, candle_wick_offsets, color_client_particle_options,
        dust_client_particle_options, dust_color_transition_client_particle_options,
        particle_utils_axis_sample, pointed_dripstone_drip_context,
        pointed_dripstone_particle_position, redstone_ore_particle_positions,
        redstone_ore_solid_render_faces, simple_client_particle_options,
        EnvironmentParticleColorSource, EnvironmentParticleSpawn, LevelParticleSpawnContext,
        ParticleBiomeSampler, ParticleEventSink, ELECTRIC_SPARK_AXIS_RADIUS,
        REDSTONE_EFFECT_DIRECTION_STEPS, REDSTONE_PARTICLE_COLOR, SCULK_PARTICLE_COLOR,
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
const PARTICLE_DRIPPING_HONEY: &str = "minecraft:dripping_honey";
const PARTICLE_FALLING_SPORE_BLOSSOM: &str = "minecraft:falling_spore_blossom";
const PARTICLE_SPORE_BLOSSOM_AIR: &str = "minecraft:spore_blossom_air";
const PARTICLE_ENCHANT: &str = "minecraft:enchant";
const PARTICLE_END_ROD: &str = "minecraft:end_rod";
const PARTICLE_DRIPPING_OBSIDIAN_TEAR: &str = "minecraft:dripping_obsidian_tear";
const PARTICLE_SMALL_FLAME: &str = "minecraft:small_flame";
const PARTICLE_ELECTRIC_SPARK: &str = "minecraft:electric_spark";
const PARTICLE_FALLING_DUST: &str = "minecraft:falling_dust";
const PARTICLE_BLOCK_MARKER: &str = "minecraft:block_marker";
const PARTICLE_MYCELIUM: &str = "minecraft:mycelium";
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
        self.synchronize_level_identity(world, connection_generation);

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
        let marker_target = world.local_marker_particle_target_block();
        let mut animate_random =
            LevelEventSoundRandomState::with_seed(self.animate_seed_random.next_long());
        let mut spawns = Vec::new();
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            self.do_animate_tick(
                world,
                center,
                NEAR_SAMPLE_RADIUS,
                sky_darken,
                marker_target,
                &mut animate_random,
                audio,
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
        marker_target: Option<&str>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) -> bool {
        let pos = sample_animate_position(&mut self.level_random, center, radius);
        let Some(block) = world.probe_block(pos) else {
            return false;
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return false;
        };

        let mut handled = true;
        match block_name {
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
                append_lever_animate_effect(pos, &block.block_properties, animate_random, spawns);
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
            "minecraft:bee_nest" | "minecraft:beehive" => {
                append_beehive_animate_effect(
                    world,
                    pos,
                    &block.block_properties,
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
                append_end_rod_animate_effect(pos, &block.block_properties, animate_random, spawns);
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
                    &block.block_properties,
                    RedstoneTorchAnimateKind::Floor,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:redstone_wall_torch" => {
                append_redstone_torch_animate_effect(
                    pos,
                    &block.block_properties,
                    RedstoneTorchAnimateKind::Wall,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:repeater" => {
                append_repeater_animate_effect(
                    pos,
                    &block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:redstone_wire" => {
                append_redstone_wire_animate_effects(
                    pos,
                    &block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
                append_redstone_ore_animate_effects(
                    world,
                    pos,
                    &block.block_properties,
                    &mut self.level_random,
                    spawns,
                );
            }
            "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor" => {
                append_sculk_sensor_animate_effect(
                    pos,
                    &block.block_properties,
                    animate_random,
                    spawns,
                );
            }
            name if block_name_is_candle(name) => {
                self.animate_candle(
                    world,
                    pos,
                    &block.block_properties,
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
                    &block.block_properties,
                    true,
                    animate_random,
                    audio,
                    spawns,
                );
            }
            name if block_name_is_lightning_rod(name) => append_lightning_rod_animate_effects(
                world,
                pos,
                &block.block_properties,
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
                    append_torch_animate_effects(pos, &block.block_properties, profile, spawns);
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

        if world.sample_block_collision_shape_is_full_block(pos) != Some(false) {
            return handled;
        }
        let particles = world
            .sampled_client_ambient_particles_at(block_position(pos))
            .unwrap_or_default();
        append_ambient_particle_spawns(&mut self.level_random, pos, particles, spawns);
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

fn append_brewing_stand_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_SMOKE),
        position: [
            f64::from(pos.x) + 0.4 + f64::from(random.next_float() * 0.2),
            f64::from(pos.y) + 0.7 + f64::from(random.next_float() * 0.3),
            f64::from(pos.z) + 0.4 + f64::from(random.next_float() * 0.2),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_torch_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    profile: TorchAnimateParticleProfile,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let mut offset = [0.0, 0.0, 0.0];
    if profile.wall {
        let Some([x, 0, z]) = properties
            .get("facing")
            .and_then(|facing| block_state_direction(facing))
        else {
            return;
        };
        offset = [-0.27 * f64::from(x), 0.22, -0.27 * f64::from(z)];
    }
    let position = [
        f64::from(pos.x) + 0.5 + offset[0],
        f64::from(pos.y) + 0.7 + offset[1],
        f64::from(pos.z) + 0.5 + offset[2],
    ];
    for particle in [PARTICLE_SMOKE, profile.particle_resource_id] {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(particle),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

fn append_lever_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("powered").map(String::as_str) != Some("true") || random.next_float() >= 0.25
    {
        return;
    }
    let Some(facing) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let connected_opposite = match properties.get("face").map(String::as_str) {
        Some("floor") => [0, -1, 0],
        Some("ceiling") => [0, 1, 0],
        Some("wall") => [-facing[0], -facing[1], -facing[2]],
        _ => return,
    };
    let facing_opposite = [-facing[0], -facing[1], -facing[2]];
    spawns.push(EnvironmentParticleSpawn {
        particle: dust_client_particle_options(REDSTONE_PARTICLE_COLOR, 0.5),
        position: [
            f64::from(pos.x)
                + 0.5
                + 0.1 * f64::from(facing_opposite[0])
                + 0.2 * f64::from(connected_opposite[0]),
            f64::from(pos.y)
                + 0.5
                + 0.1 * f64::from(facing_opposite[1])
                + 0.2 * f64::from(connected_opposite[1]),
            f64::from(pos.z)
                + 0.5
                + 0.1 * f64::from(facing_opposite[2])
                + 0.2 * f64::from(connected_opposite[2]),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_mycelium_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(10) != 0 {
        return;
    }
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_MYCELIUM),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 1.1,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_wither_rose_animate_effects(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..3 {
        if random.next_boolean() {
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_SMOKE),
                position: [
                    f64::from(pos.x) + 0.5 + random.next_double() / 5.0,
                    f64::from(pos.y) + 0.5 - random.next_double(),
                    f64::from(pos.z) + 0.5 + random.next_double() / 5.0,
                ],
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
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
            color_source: None,
        });
        SOUND_EVENT_BUBBLE_COLUMN_WHIRLPOOL_AMBIENT
    } else {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_BUBBLE_COLUMN_UP),
            position: [x + 0.5, y, z + 0.5],
            velocity: [0.0, 0.04, 0.0],
            always_show: true,
            color_source: None,
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
            color_source: None,
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
            color_source: None,
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
            color_source: None,
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
        fluid_block.full_face_sturdy[0].then_some((
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
        color_source: None,
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
            color_source: None,
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
        color_source: None,
    });
    if kind == FurnaceAnimateKind::Furnace {
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_FLAME),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
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
                color_source: None,
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
    let below_has_sturdy_top = below_sample.is_some_and(|sample| sample.full_face_sturdy[1]);
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
                color_source: None,
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
                color_source: None,
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
        color_source: None,
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
            color_source: None,
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
                color_source: None,
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
            color_source: None,
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
        color_source: None,
    });
}

fn append_beehive_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    animate_random: &mut LevelEventSoundRandomState,
    level_random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties
        .get("honey_level")
        .and_then(|value| value.parse::<u8>().ok())
        .is_none_or(|honey_level| honey_level < 5)
    {
        return;
    }

    let count = animate_random.next_int_bound(1) + 1;
    for _ in 0..count {
        let Some(sample) = world.environment_animate_block_sample(pos) else {
            continue;
        };
        if sample.fluid.is_some() || level_random.next_float() < 0.3 {
            continue;
        }
        let Some(bounds) = sample.collision_bounds else {
            continue;
        };
        if bounds.max_y < 1.0 || sample.impermeable {
            continue;
        }

        let height = if bounds.min_y > 0.0 {
            f64::from(pos.y) + bounds.min_y - 0.05
        } else {
            let below = BlockPos {
                y: pos.y.saturating_sub(1),
                ..pos
            };
            let Some(below_sample) = world.environment_animate_block_sample(below) else {
                continue;
            };
            let below_top = below_sample
                .collision_bounds
                .map_or(0.0, |bounds| bounds.max_y);
            let Some(below_is_full_block) = world.sample_block_collision_shape_is_full_block(below)
            else {
                continue;
            };
            if (below_top >= 1.0 && below_is_full_block) || below_sample.fluid.is_some() {
                continue;
            }
            f64::from(pos.y) - 0.05
        };

        let x1 = f64::from(pos.x) + bounds.min_x;
        let x2 = f64::from(pos.x) + bounds.max_x;
        let z1 = f64::from(pos.z) + bounds.min_z;
        let z2 = f64::from(pos.z) + bounds.max_z;
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_DRIPPING_HONEY),
            position: [
                x1 + level_random.next_double() * (x2 - x1),
                height,
                z1 + level_random.next_double() * (z2 - z1),
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

fn append_spore_blossom_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_FALLING_SPORE_BLOSSOM),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 0.7,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });

    for _ in 0..14 {
        let ambient_pos = BlockPos {
            x: pos.x + random.next_int_bound(21) - 10,
            y: pos.y - random.next_int_bound(10),
            z: pos.z + random.next_int_bound(21) - 10,
        };
        if world.sample_block_collision_shape_is_full_block(ambient_pos) != Some(false) {
            continue;
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_SPORE_BLOSSOM_AIR),
            position: [
                f64::from(ambient_pos.x) + random.next_double(),
                f64::from(ambient_pos.y) + random.next_double(),
                f64::from(ambient_pos.z) + random.next_double(),
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

fn append_enchanting_table_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for (offset_x, offset_y, offset_z) in enchanting_table_bookshelf_offsets() {
        if random.next_int_bound(16) != 0 {
            continue;
        }
        let provider_pos = BlockPos {
            x: pos.x + offset_x,
            y: pos.y + offset_y,
            z: pos.z + offset_z,
        };
        if !world
            .environment_animate_block_sample(provider_pos)
            .is_some_and(|sample| sample.enchantment_power_provider)
        {
            continue;
        }
        let transmitter_pos = BlockPos {
            x: pos.x + offset_x / 2,
            y: pos.y + offset_y,
            z: pos.z + offset_z / 2,
        };
        if !world
            .environment_animate_block_sample(transmitter_pos)
            .is_some_and(|sample| sample.enchantment_power_transmitter)
        {
            continue;
        }

        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_ENCHANT),
            position: [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 2.0,
                f64::from(pos.z) + 0.5,
            ],
            velocity: [
                f64::from(offset_x as f32 + random.next_float()) - 0.5,
                f64::from(offset_y as f32 - random.next_float() - 1.0),
                f64::from(offset_z as f32 + random.next_float()) - 0.5,
            ],
            always_show: false,
            color_source: None,
        });
    }
}

fn enchanting_table_bookshelf_offsets() -> impl Iterator<Item = (i32, i32, i32)> {
    (-2_i32..=2).flat_map(|z| {
        (0_i32..=1).flat_map(move |y| {
            (-2_i32..=2).filter_map(move |x| (x.abs() == 2 || z.abs() == 2).then_some((x, y, z)))
        })
    })
}

fn append_end_rod_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(direction) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let x = f64::from(pos.x) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let y = f64::from(pos.y) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let z = f64::from(pos.z) + 0.55 - f64::from(random.next_float() * 0.1_f32);
    let r = f64::from(0.4_f32 - (random.next_float() + random.next_float()) * 0.4_f32);
    if random.next_int_bound(5) != 0 {
        return;
    }

    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_END_ROD),
        position: [
            x + f64::from(direction[0]) * r,
            y + f64::from(direction[1]) * r,
            z + f64::from(direction[2]) * r,
        ],
        velocity: [
            random.next_gaussian() * 0.005,
            random.next_gaussian() * 0.005,
            random.next_gaussian() * 0.005,
        ],
        always_show: false,
        color_source: None,
    });
}

fn append_candle_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    on_cake: bool,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) -> Vec<CandleAnimateSound> {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return Vec::new();
    }
    let candles = if on_cake {
        1
    } else {
        let Some(candles) = properties
            .get("candles")
            .and_then(|candles| candles.parse::<u8>().ok())
            .filter(|candles| (1..=4).contains(candles))
        else {
            return Vec::new();
        };
        candles
    };

    let mut sounds = Vec::new();
    for offset in candle_wick_offsets(candles, on_cake) {
        let position = [
            f64::from(pos.x) + offset[0] / 16.0,
            f64::from(pos.y) + offset[1] / 16.0,
            f64::from(pos.z) + offset[2] / 16.0,
        ];
        let chance = random.next_float();
        if chance < 0.3 {
            spawns.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(PARTICLE_SMOKE),
                position,
                velocity: [0.0; 3],
                always_show: false,
                color_source: None,
            });
            if chance < 0.17 {
                sounds.push((
                    position.map(|coordinate| coordinate + 0.5),
                    1.0_f32 + random.next_float(),
                    random.next_float() * 0.7_f32 + 0.3_f32,
                ));
            }
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_SMALL_FLAME),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
    sounds
}

fn append_lightning_rod_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if !world.is_thundering() {
        return;
    }
    let game_time = world.world_time().map_or(0, |time| time.game_time);
    if i64::from(random.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE))
        > game_time % i64::from(LIGHTNING_ROD_SPARK_CYCLE)
        || world.sample_heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, pos.x, pos.z)
            != pos.y.checked_add(1)
    {
        return;
    }
    let Some([step_x, step_y, _]) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let axis = if step_x != 0 {
        0
    } else if step_y != 0 {
        1
    } else {
        2
    };
    let particle_count = random.next_int_bound(2) + 1;
    for _ in 0..particle_count {
        let (position, velocity) =
            particle_utils_axis_sample(pos, axis, ELECTRIC_SPARK_AXIS_RADIUS, random);
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_ELECTRIC_SPARK),
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            always_show: false,
            color_source: None,
        });
    }
}

fn append_ender_chest_animate_effects(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..3 {
        let flip_x = random.next_int_bound(2) * 2 - 1;
        let flip_z = random.next_int_bound(2) * 2 - 1;
        let position = [
            f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip_x),
            f64::from(pos.y) + f64::from(random.next_float()),
            f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip_z),
        ];
        let velocity = [
            f64::from(random.next_float() * flip_x as f32),
            f64::from((random.next_float() - 0.5_f32) * 0.125_f32),
            f64::from(random.next_float() * flip_z as f32),
        ];
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_PORTAL),
            position,
            velocity,
            always_show: false,
            color_source: None,
        });
    }
}

fn append_end_portal_animate_effect(
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_SMOKE),
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + 0.8,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_end_gateway_animate_effects(
    pos: BlockPos,
    particle_count: usize,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for _ in 0..particle_count {
        let mut position = [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) + random.next_double(),
            f64::from(pos.z) + random.next_double(),
        ];
        let mut velocity = [
            (random.next_double() - 0.5) * 0.5,
            (random.next_double() - 0.5) * 0.5,
            (random.next_double() - 0.5) * 0.5,
        ];
        let flip = random.next_int_bound(2) * 2 - 1;
        if random.next_boolean() {
            position[2] = f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip);
            velocity[2] = f64::from(random.next_float() * 2.0 * flip as f32);
        } else {
            position[0] = f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip);
            velocity[0] = f64::from(random.next_float() * 2.0 * flip as f32);
        }
        spawns.push(EnvironmentParticleSpawn {
            particle: simple_client_particle_options(PARTICLE_PORTAL),
            position,
            velocity,
            always_show: false,
            color_source: None,
        });
    }
}

/// Vanilla `ClientLevel.doAnimateTick` marker branch: a matching sampled block
/// spawns one zero-velocity BLOCK_MARKER particle carrying the sampled state at
/// the block center.
fn append_block_marker_animate_effect(
    pos: BlockPos,
    block_state_id: i32,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle: block_client_particle_options(PARTICLE_BLOCK_MARKER, block_state_id),
        position: [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_falling_dust_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    block_state_id: i32,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(FALLING_DUST_CHANCE_BOUND) != 0
        || world.falling_block_is_free(BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        }) != Some(true)
    {
        return;
    }
    append_particle_below(
        pos,
        block_client_particle_options(PARTICLE_FALLING_DUST, block_state_id),
        None,
        random,
        spawns,
    );
}

fn should_play_ambient_desert_sand_sound(world: &WorldStore, pos: BlockPos) -> bool {
    const DIRECTIONS: [[i32; 2]; 4] = [[0, -8], [8, 0], [0, 8], [-8, 0]];

    let mut matching_blocks_found = 0;
    let mut sides_checked = 0;
    for [dx, dz] in DIRECTIONS {
        let column_pos = BlockPos {
            x: pos.x.saturating_add(dx),
            y: pos.y,
            z: pos.z.saturating_add(dz),
        };
        if column_contains_ambient_desert_sand_trigger(world, column_pos) {
            let previous_matches = matching_blocks_found;
            matching_blocks_found += 1;
            if previous_matches >= 3 {
                return true;
            }
        }

        sides_checked += 1;
        let remaining_sides = 4 - sides_checked;
        if remaining_sides + matching_blocks_found < 3 {
            return false;
        }
    }
    false
}

fn column_contains_ambient_desert_sand_trigger(world: &WorldStore, pos: BlockPos) -> bool {
    let Some(surface_y) = world
        .sample_heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, pos.x, pos.z)
        .and_then(|first_available| first_available.checked_sub(1))
    else {
        return false;
    };

    if surface_y.abs_diff(pos.y) > 5 {
        let mut above_name = block_name_at(
            world,
            BlockPos {
                y: pos.y.saturating_add(6),
                ..pos
            },
        );
        for offset in (pos.y.saturating_sub(4)..=pos.y.saturating_add(5)).rev() {
            let current_name = block_name_at(world, BlockPos { y: offset, ..pos });
            if above_name.as_deref().is_some_and(block_name_is_air)
                && current_name
                    .as_deref()
                    .is_some_and(block_name_triggers_ambient_desert_sand_block_sounds)
            {
                return true;
            }
            above_name = current_name;
        }
        return false;
    }

    block_name_at(
        world,
        BlockPos {
            y: surface_y.saturating_add(1),
            ..pos
        },
    )
    .as_deref()
    .is_some_and(block_name_is_air)
        && block_name_at(
            world,
            BlockPos {
                y: surface_y,
                ..pos
            },
        )
        .as_deref()
        .is_some_and(block_name_triggers_ambient_desert_sand_block_sounds)
}

fn supports_ambient_desert_dry_vegetation_sound(world: &WorldStore, pos: BlockPos) -> bool {
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_below = BlockPos {
        y: below.y.saturating_sub(1),
        ..below
    };
    [below, below_below].into_iter().all(|support| {
        block_name_at(world, support)
            .as_deref()
            .is_some_and(block_name_triggers_ambient_desert_dry_vegetation_block_sounds)
    })
}

fn block_name_at(world: &WorldStore, pos: BlockPos) -> Option<String> {
    world.probe_block(pos)?.block_name
}

fn append_leaves_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    profile: LeafAnimateParticleProfile,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    let below_sample = world.environment_animate_block_sample(below);

    if world.is_raining_at(BlockPos {
        y: pos.y.saturating_add(1),
        ..pos
    }) && random.next_int_bound(15) == 1
        && below_sample.is_some_and(|sample| !sample.can_occlude || !sample.full_face_sturdy[1])
    {
        append_particle_below(
            pos,
            simple_client_particle_options(PARTICLE_DRIPPING_WATER),
            None,
            random,
            spawns,
        );
    }

    if random.next_float() >= profile.leaf_particle_chance()
        || below_sample.is_none_or(|sample| sample.collision_full_face[1])
    {
        return;
    }
    let (particle, color_source) = match profile.tint {
        LeafParticleTint::None => (
            simple_client_particle_options(profile.particle_resource_id),
            None,
        ),
        LeafParticleTint::FixedArgb(argb) => (
            color_client_particle_options(profile.particle_resource_id, argb),
            None,
        ),
        LeafParticleTint::ClientLeaf => (
            color_client_particle_options(profile.particle_resource_id, u32::MAX),
            Some(EnvironmentParticleColorSource::ClientLeaf {
                block_pos: pos,
                block_resource_id: profile.block_resource_id,
            }),
        ),
    };
    append_particle_below(pos, particle, color_source, random, spawns);
}

fn append_particle_below(
    pos: BlockPos,
    particle: bbb_world::ClientParticleOptions,
    color_source: Option<EnvironmentParticleColorSource>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    spawns.push(EnvironmentParticleSpawn {
        particle,
        position: [
            f64::from(pos.x) + random.next_double(),
            f64::from(pos.y) - 0.05,
            f64::from(pos.z) + random.next_double(),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source,
    });
}

fn block_state_direction(facing: &str) -> Option<[i32; 3]> {
    match facing {
        "down" => Some([0, -1, 0]),
        "up" => Some([0, 1, 0]),
        "north" => Some([0, 0, -1]),
        "south" => Some([0, 0, 1]),
        "west" => Some([-1, 0, 0]),
        "east" => Some([1, 0, 0]),
        _ => None,
    }
}

fn append_wet_sponge_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let direction_index = random.next_int_bound(6) as usize;
    if direction_index == 1 || neighbor_has_sturdy_opposite_face(world, pos, direction_index) {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    let mut x = f64::from(pos.x);
    let mut y = f64::from(pos.y);
    let mut z = f64::from(pos.z);
    if direction_index == 0 {
        y -= 0.05;
        x += random.next_double();
        z += random.next_double();
    } else {
        y += random.next_double() * 0.8;
        if direction[0] != 0 {
            z += random.next_double();
            x += if direction[0] > 0 { 1.1 } else { 0.05 };
        } else {
            x += random.next_double();
            z += if direction[2] > 0 { 1.1 } else { 0.05 };
        }
    }

    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_DRIPPING_WATER),
        position: [x, y, z],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_crying_obsidian_animate_effect(
    world: &WorldStore,
    pos: BlockPos,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if random.next_int_bound(5) != 0 {
        return;
    }
    let direction_index = random.next_int_bound(6) as usize;
    if direction_index == 1 || neighbor_has_sturdy_opposite_face(world, pos, direction_index) {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    spawns.push(EnvironmentParticleSpawn {
        particle: simple_client_particle_options(PARTICLE_DRIPPING_OBSIDIAN_TEAR),
        position: [
            f64::from(pos.x) + crying_obsidian_offset(direction[0], random),
            f64::from(pos.y) + crying_obsidian_offset(direction[1], random),
            f64::from(pos.z) + crying_obsidian_offset(direction[2], random),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn crying_obsidian_offset(step: i32, random: &mut LevelEventSoundRandomState) -> f64 {
    if step == 0 {
        random.next_double()
    } else {
        0.5 + f64::from(step) * 0.6
    }
}

fn neighbor_has_sturdy_opposite_face(
    world: &WorldStore,
    pos: BlockPos,
    direction_index: usize,
) -> bool {
    let step = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
    world
        .environment_animate_block_sample(BlockPos {
            x: pos.x + step[0],
            y: pos.y + step[1],
            z: pos.z + step[2],
        })
        .is_some_and(|sample| {
            sample.full_face_sturdy[OPPOSITE_EFFECT_DIRECTION_INDICES[direction_index]]
        })
}

fn append_redstone_torch_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    kind: RedstoneTorchAnimateKind,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return;
    }
    let direction = match kind {
        RedstoneTorchAnimateKind::Floor => [0, 0, 0],
        RedstoneTorchAnimateKind::Wall => {
            let Some([x, 0, z]) = properties
                .get("facing")
                .and_then(|facing| block_state_direction(facing))
            else {
                return;
            };
            [-x, 0, -z]
        }
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: dust_client_particle_options(REDSTONE_PARTICLE_COLOR, 1.0),
        position: [
            f64::from(pos.x)
                + 0.5
                + (random.next_double() - 0.5) * 0.2
                + 0.27 * f64::from(direction[0]),
            f64::from(pos.y)
                + 0.7
                + (random.next_double() - 0.5) * 0.2
                + if kind == RedstoneTorchAnimateKind::Wall {
                    0.22
                } else {
                    0.0
                },
            f64::from(pos.z)
                + 0.5
                + (random.next_double() - 0.5) * 0.2
                + 0.27 * f64::from(direction[2]),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_repeater_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("powered").map(String::as_str) != Some("true") {
        return;
    }
    let Some([step_x, 0, step_z]) = properties
        .get("facing")
        .and_then(|facing| block_state_direction(facing))
    else {
        return;
    };
    let Some(delay) = properties
        .get("delay")
        .and_then(|delay| delay.parse::<i32>().ok())
        .filter(|delay| (1..=4).contains(delay))
    else {
        return;
    };

    let x = f64::from(pos.x) + 0.5 + (random.next_double() - 0.5) * 0.2;
    let y = f64::from(pos.y) + 0.4 + (random.next_double() - 0.5) * 0.2;
    let z = f64::from(pos.z) + 0.5 + (random.next_double() - 0.5) * 0.2;
    let mut offset = -5.0_f32;
    if random.next_boolean() {
        offset = (delay * 2 - 1) as f32;
    }
    offset /= 16.0;

    spawns.push(EnvironmentParticleSpawn {
        particle: dust_client_particle_options(REDSTONE_PARTICLE_COLOR, 1.0),
        position: [
            x + f64::from(offset * step_x as f32),
            y,
            z + f64::from(offset * step_z as f32),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn append_redstone_wire_animate_effects(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(power) = properties
        .get("power")
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|power| *power <= 15)
    else {
        return;
    };
    if power == 0 {
        return;
    }
    let [Some(north), Some(east), Some(south), Some(west)] =
        REDSTONE_WIRE_DIRECTIONS.map(|direction| {
            redstone_wire_side(properties.get(direction.property).map(String::as_str))
        })
    else {
        return;
    };
    let sides = [north, east, south, west];

    let color = redstone_wire_color(power);
    for (direction, side) in REDSTONE_WIRE_DIRECTIONS.into_iter().zip(sides) {
        if side == RedstoneWireSide::Up {
            append_redstone_wire_particle_along_line(
                pos,
                color,
                direction.step,
                [0, 1, 0],
                -0.5,
                0.5,
                random,
                spawns,
            );
        }
        let (from, to) = if side == RedstoneWireSide::None {
            (0.0, 0.3)
        } else {
            (0.0, 0.5)
        };
        append_redstone_wire_particle_along_line(
            pos,
            color,
            [0, -1, 0],
            direction.step,
            from,
            to,
            random,
            spawns,
        );
    }
}

fn redstone_wire_side(value: Option<&str>) -> Option<RedstoneWireSide> {
    match value {
        Some("up") => Some(RedstoneWireSide::Up),
        Some("side") => Some(RedstoneWireSide::Side),
        Some("none") => Some(RedstoneWireSide::None),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_redstone_wire_particle_along_line(
    pos: BlockPos,
    color: u32,
    side: [i32; 3],
    along: [i32; 3],
    from: f32,
    to: f32,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let span = to - from;
    if random.next_float() >= 0.2 * span {
        return;
    }
    let position_on_line = from + span * random.next_float();
    let coordinate = |axis: usize| {
        0.5 + f64::from(0.4375_f32 * side[axis] as f32)
            + f64::from(position_on_line * along[axis] as f32)
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: dust_client_particle_options(color, 1.0),
        position: [
            f64::from(pos.x) + coordinate(0),
            f64::from(pos.y) + coordinate(1),
            f64::from(pos.z) + coordinate(2),
        ],
        velocity: [0.0; 3],
        always_show: false,
        color_source: None,
    });
}

fn redstone_wire_color(power: u8) -> u32 {
    let power = f32::from(power) / 15.0;
    let red = power * 0.6 + if power > 0.0 { 0.4 } else { 0.3 };
    let green = (power * power * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (power * power * 0.6 - 0.7).clamp(0.0, 1.0);
    let channel = |value: f32| (value * 255.0).floor() as u32;
    (channel(1.0) << 24) | (channel(red) << 16) | (channel(green) << 8) | channel(blue)
}

fn append_redstone_ore_animate_effects(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    level_random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("lit").map(String::as_str) != Some("true") {
        return;
    }
    let solid_render_faces = redstone_ore_solid_render_faces(world, pos);
    for position in
        redstone_ore_particle_positions(pos, solid_render_faces, || level_random.next_float())
            .into_iter()
            .flatten()
    {
        spawns.push(EnvironmentParticleSpawn {
            particle: dust_client_particle_options(REDSTONE_PARTICLE_COLOR, 1.0),
            position,
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

fn append_sculk_sensor_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("sculk_sensor_phase").map(String::as_str) != Some("active") {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[random.next_int_bound(6) as usize];
    if direction[1] != 0 {
        return;
    }
    let x_offset = if direction[0] == 0 {
        0.5 - random.next_double()
    } else {
        f64::from(direction[0]) * 0.6
    };
    let z_offset = if direction[2] == 0 {
        0.5 - random.next_double()
    } else {
        f64::from(direction[2]) * 0.6
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: dust_color_transition_client_particle_options(
            SCULK_PARTICLE_COLOR,
            REDSTONE_PARTICLE_COLOR,
            1.0,
        ),
        position: [
            f64::from(pos.x) + 0.5 + x_offset,
            f64::from(pos.y) + 0.25,
            f64::from(pos.z) + 0.5 + z_offset,
        ],
        velocity: [0.0, f64::from(random.next_float()) * 0.04, 0.0],
        always_show: false,
        color_source: None,
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
                color_source: None,
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
            color_source: None,
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

    use bbb_audio::TickEntitySoundPositionsCommand;
    use bbb_control::AudioCounters;
    use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
    use bbb_protocol::game_event_types::{
        VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
        VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
    };
    use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;
    use bbb_protocol::packets::{
        BlockUpdate, ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes,
        CommonPlayerSpawnInfo, GameEvent, PlayLogin, PlayTime,
    };
    use bbb_world::{
        AmbientLoopSoundState, ChunkColumn, ChunkPos, ChunkSection, ChunkState,
        ClientAmbientParticle, HeightmapData, JukeboxLevelEventState, LightData,
        LocalSoundEventState, PaletteDomain, PaletteKind, PalettedContainerData,
        RegistryPacketEntry, SoundEntityEventState, StopSoundEventState, TickableEntitySoundState,
        WorldDimension,
    };

    use super::*;

    #[derive(Default)]
    struct RecordingEnvironmentAudio {
        positioned: Vec<SoundEventState>,
        entity: Vec<(SoundEntityEventState, Option<[f64; 3]>)>,
    }

    impl AudioEventSink for RecordingEnvironmentAudio {
        fn counters(&self) -> AudioCounters {
            AudioCounters::default()
        }

        fn set_sound_event_registry(&mut self, _registry: SoundEventRegistry) {}

        fn set_jukebox_song_registry(&mut self, _registry: JukeboxSongRegistry) {}

        fn play_local_sound(&mut self, _state: &LocalSoundEventState) {}

        fn play_positioned_sound(&mut self, state: &SoundEventState) {
            self.positioned.push(state.clone());
        }

        fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
            self.entity.push((state.clone(), position));
        }

        fn play_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_sound(&mut self, _state: &StopSoundEventState) {}

        fn set_music_gain(&mut self, _gain: f32) {}

        fn sync_ambient_loops(&mut self, _loops: &[AmbientLoopSoundState]) {}

        fn tick_entity_sound_positions(
            &mut self,
            _command: TickEntitySoundPositionsCommand,
            _tickable_sounds: &[TickableEntitySoundState],
        ) {
        }
    }

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
    fn brewing_stand_and_torches_preserve_exact_vanilla_particle_positions() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let mut actual = LevelEventSoundRandomState::with_seed(91);
        let mut expected = LevelEventSoundRandomState::with_seed(91);
        let mut spawns = Vec::new();
        append_brewing_stand_animate_effect(pos, &mut actual, &mut spawns);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_SMOKE);
        assert_eq!(
            spawns[0].position,
            [
                -4.0 + 0.4 + f64::from(expected.next_float() * 0.2),
                12.0 + 0.7 + f64::from(expected.next_float() * 0.3),
                30.0 + 0.4 + f64::from(expected.next_float() * 0.2),
            ]
        );
        assert_eq!(actual.next_long(), expected.next_long());

        spawns.clear();
        append_torch_animate_effects(
            pos,
            &BTreeMap::new(),
            torch_animate_particle_profile("minecraft:copper_torch").unwrap(),
            &mut spawns,
        );
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.particle.resource_id.as_str())
                .collect::<Vec<_>>(),
            [PARTICLE_SMOKE, "minecraft:copper_fire_flame"]
        );
        assert!(spawns
            .iter()
            .all(|spawn| spawn.position == [-3.5, 12.7, 30.5]));

        spawns.clear();
        append_torch_animate_effects(
            pos,
            &BTreeMap::from([("facing".to_string(), "east".to_string())]),
            torch_animate_particle_profile("minecraft:soul_wall_torch").unwrap(),
            &mut spawns,
        );
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.particle.resource_id.as_str())
                .collect::<Vec<_>>(),
            [PARTICLE_SMOKE, "minecraft:soul_fire_flame"]
        );
        assert!(spawns
            .iter()
            .all(|spawn| spawn.position == [-3.77, 12.92, 30.5]));
    }

    #[test]
    fn lever_mycelium_and_wither_rose_preserve_random_draw_order() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let lever_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.25)
            .expect("deterministic lever particle seed");
        let mut actual = LevelEventSoundRandomState::with_seed(lever_seed);
        let mut expected = LevelEventSoundRandomState::with_seed(lever_seed);
        let mut spawns = Vec::new();
        append_lever_animate_effect(
            pos,
            &BTreeMap::from([
                ("powered".to_string(), "true".to_string()),
                ("facing".to_string(), "north".to_string()),
                ("face".to_string(), "ceiling".to_string()),
            ]),
            &mut actual,
            &mut spawns,
        );
        assert!(expected.next_float() < 0.25);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].position, [-3.5, 12.7, 30.6]);
        assert_eq!(
            spawns[0].particle.option,
            bbb_world::ClientParticleOptionValue::Dust {
                color: REDSTONE_PARTICLE_COLOR,
                scale_bits: 0.5_f32.to_bits(),
            }
        );
        assert_eq!(actual.next_long(), expected.next_long());

        let mycelium_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(10) == 0)
            .expect("deterministic mycelium particle seed");
        actual = LevelEventSoundRandomState::with_seed(mycelium_seed);
        expected = LevelEventSoundRandomState::with_seed(mycelium_seed);
        spawns.clear();
        append_mycelium_animate_effect(pos, &mut actual, &mut spawns);
        assert_eq!(expected.next_int_bound(10), 0);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_MYCELIUM);
        assert_eq!(
            spawns[0].position,
            [
                -4.0 + expected.next_double(),
                13.1,
                30.0 + expected.next_double(),
            ]
        );
        assert_eq!(actual.next_long(), expected.next_long());

        let wither_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                let mut emitted = 0;
                for _ in 0..3 {
                    if random.next_boolean() {
                        emitted += 1;
                        for _ in 0..3 {
                            let _ = random.next_double();
                        }
                    }
                }
                emitted == 2
            })
            .expect("deterministic mixed wither rose particle seed");
        actual = LevelEventSoundRandomState::with_seed(wither_seed);
        expected = LevelEventSoundRandomState::with_seed(wither_seed);
        spawns.clear();
        append_wither_rose_animate_effects(pos, &mut actual, &mut spawns);
        let mut expected_positions = Vec::new();
        for _ in 0..3 {
            if expected.next_boolean() {
                expected_positions.push([
                    -3.5 + expected.next_double() / 5.0,
                    12.5 - expected.next_double(),
                    30.5 + expected.next_double() / 5.0,
                ]);
            }
        }
        assert_eq!(spawns.len(), 2);
        assert!(spawns
            .iter()
            .all(|spawn| spawn.particle.resource_id == PARTICLE_SMOKE));
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
    fn eyeblossom_and_hanging_moss_idle_sounds_require_exact_supports() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let below = BlockPos { y: 31, ..pos };
        let above = BlockPos { y: 33, ..pos };
        let mut world = world_with_ambient_particles();
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut recording = RecordingEnvironmentAudio::default();
        let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut recording);

        set_block(
            &mut world,
            below,
            block_state_id("minecraft:pale_moss_block", &[]),
        );
        let eyeblossom_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(700) == 0)
            .expect("deterministic eyeblossom sound seed");
        let mut eyeblossom_random = LevelEventSoundRandomState::with_seed(eyeblossom_seed);
        runtime.animate_eyeblossom(&world, pos, &mut eyeblossom_random, &mut audio);

        set_block(
            &mut world,
            above,
            block_state_id("minecraft:stripped_pale_oak_log", &[("axis", "y")]),
        );
        let moss_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(500) == 0)
            .expect("deterministic hanging moss sound seed");
        let mut moss_random = LevelEventSoundRandomState::with_seed(moss_seed);
        runtime.animate_hanging_moss(&world, pos, &mut moss_random, &mut audio);

        assert_eq!(recording.positioned.len(), 2);
        let expected_eyeblossom_id = world
            .resolved_registry_entry_by_resource_id(
                "minecraft:sound_event",
                SOUND_EVENT_EYEBLOSSOM_IDLE,
            )
            .map(|entry| entry.holder_id);
        let expected_hanging_moss_id = world
            .resolved_registry_entry_by_resource_id(
                "minecraft:sound_event",
                SOUND_EVENT_PALE_HANGING_MOSS_IDLE,
            )
            .map(|entry| entry.holder_id);
        assert!(expected_eyeblossom_id.is_some());
        assert!(expected_hanging_moss_id.is_some());
        assert_eq!(
            recording.positioned[0].sound.registry_id,
            expected_eyeblossom_id
        );
        assert_eq!(
            recording.positioned[1].sound.registry_id,
            expected_hanging_moss_id
        );
        assert!(recording.positioned.iter().all(|sound| {
            sound.source == "ambient"
                && sound.position
                    == ProtocolVec3d {
                        x: 8.0,
                        y: 32.0,
                        z: 8.0,
                    }
                && sound.volume == 1.0
                && sound.pitch == 1.0
                && !sound.distance_delay
        }));
        let mut expected_level = LevelEventSoundRandomState::with_seed(0);
        assert_eq!(recording.positioned[0].seed, expected_level.next_long());
        assert_eq!(recording.positioned[1].seed, expected_level.next_long());
        assert_eq!(runtime.level_random.next_long(), expected_level.next_long());
    }

    #[test]
    fn environment_dispatches_remaining_block_animate_tick_particles() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::default();
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let lever_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.25)
            .unwrap();
        let mycelium_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(10) == 0)
            .unwrap();
        let wither_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                (0..3)
                    .filter(|_| {
                        let emitted = random.next_boolean();
                        if emitted {
                            for _ in 0..3 {
                                let _ = random.next_double();
                            }
                        }
                        emitted
                    })
                    .count()
                    == 2
            })
            .unwrap();
        for (block_name, properties, seed, particle, expected_count) in [
            (
                "minecraft:brewing_stand",
                vec![
                    ("has_bottle_0", "false"),
                    ("has_bottle_1", "false"),
                    ("has_bottle_2", "false"),
                ],
                1,
                PARTICLE_SMOKE,
                1,
            ),
            (
                "minecraft:soul_wall_torch",
                vec![("facing", "north")],
                1,
                "minecraft:soul_fire_flame",
                1,
            ),
            (
                "minecraft:lever",
                vec![("face", "wall"), ("facing", "north"), ("powered", "true")],
                lever_seed,
                "minecraft:dust",
                1,
            ),
            (
                "minecraft:mycelium",
                vec![("snowy", "false")],
                mycelium_seed,
                PARTICLE_MYCELIUM,
                1,
            ),
            (
                "minecraft:wither_rose",
                Vec::new(),
                wither_seed,
                PARTICLE_SMOKE,
                2,
            ),
        ] {
            let mut world = world_with_ambient_particles();
            set_block(&mut world, pos, block_state_id(block_name, &properties));
            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut random = LevelEventSoundRandomState::with_seed(seed);
            let mut audio = None;
            let mut spawns = Vec::new();
            runtime.do_animate_tick(
                &mut world,
                center,
                NEAR_SAMPLE_RADIUS,
                0,
                None,
                &mut random,
                &mut audio,
                &mut spawns,
            );
            assert_eq!(
                spawns
                    .iter()
                    .filter(|spawn| spawn.particle.resource_id == particle)
                    .count(),
                expected_count,
                "{block_name}"
            );
        }
    }

    #[test]
    fn generated_animate_tick_catalog_is_fully_covered_by_runtime_dispatch() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::default();
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();

        for &block_name in bbb_world::vanilla_block_animate_tick_names() {
            let block_type =
                bbb_protocol::block_types::vanilla_block_type_for_resource_id(block_name)
                    .unwrap_or_else(|| panic!("missing generated block type {block_name}"));
            let block_state_id =
                bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
                    .unwrap_or_else(|| panic!("missing default block state {block_name}"));
            set_block(&mut world, pos, block_state_id);

            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut random = LevelEventSoundRandomState::with_seed(42);
            let mut audio = None;
            let mut spawns = Vec::new();
            assert!(
                runtime.do_animate_tick(
                    &mut world,
                    center,
                    NEAR_SAMPLE_RADIUS,
                    0,
                    None,
                    &mut random,
                    &mut audio,
                    &mut spawns,
                ),
                "generated animateTick block lacks native dispatch: {block_name}"
            );
        }
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
        let mut stone_actual = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut stone_expected = LevelEventSoundRandomState::with_seed(effect_seed);
        let mut stone_spawns = Vec::new();
        assert_eq!(
            append_dried_ghast_animate_effects(
                &world,
                pos,
                false,
                &mut stone_actual,
                &mut stone_spawns,
            ),
            None
        );
        assert_eq!(stone_expected.next_int_bound(40), 0);
        assert_eq!(stone_expected.next_int_bound(6), 0);
        assert_eq!(stone_spawns.len(), 1);
        assert_eq!(stone_spawns[0].particle.resource_id, PARTICLE_WHITE_SMOKE);
        assert_eq!(stone_actual.next_long(), stone_expected.next_long());

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
    fn beehive_animate_effects_preserve_honey_collision_and_two_random_streams() {
        let mut world = world_with_ambient_particles();
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let below = BlockPos { y: 31, ..pos };
        let full_properties = BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("honey_level".to_string(), "5".to_string()),
        ]);
        let partial_properties = BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("honey_level".to_string(), "4".to_string()),
        ]);
        let full_beehive_id = block_state_id(
            "minecraft:beehive",
            &[("facing", "north"), ("honey_level", "5")],
        );
        let partial_beehive_id = block_state_id(
            "minecraft:beehive",
            &[("facing", "north"), ("honey_level", "4")],
        );
        let spawn_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() >= 0.3)
            .expect("deterministic honey drip spawn seed");
        let skip_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.3)
            .expect("deterministic honey drip skip seed");

        set_block(&mut world, pos, full_beehive_id);
        set_block(&mut world, below, 0);
        let mut animate_actual = LevelEventSoundRandomState::with_seed(11);
        let mut animate_expected = LevelEventSoundRandomState::with_seed(11);
        let mut level_actual = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut level_expected = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &full_properties,
            &mut animate_actual,
            &mut level_actual,
            &mut spawns,
        );
        assert_eq!(animate_expected.next_int_bound(1), 0);
        assert!(level_expected.next_float() >= 0.3);
        let expected_position = [
            8.0 + level_expected.next_double(),
            31.95,
            8.0 + level_expected.next_double(),
        ];
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_DRIPPING_HONEY);
        assert_eq!(spawns[0].position, expected_position);
        assert_eq!(spawns[0].velocity, [0.0; 3]);
        assert!(!spawns[0].always_show);
        assert_eq!(animate_actual.next_long(), animate_expected.next_long());
        assert_eq!(level_actual.next_long(), level_expected.next_long());

        let mut skip_animate_actual = LevelEventSoundRandomState::with_seed(11);
        let mut skip_animate_expected = LevelEventSoundRandomState::with_seed(11);
        let mut skip_level_actual = LevelEventSoundRandomState::with_seed(skip_seed);
        let mut skip_level_expected = LevelEventSoundRandomState::with_seed(skip_seed);
        let mut skip_spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &full_properties,
            &mut skip_animate_actual,
            &mut skip_level_actual,
            &mut skip_spawns,
        );
        assert_eq!(skip_animate_expected.next_int_bound(1), 0);
        assert!(skip_level_expected.next_float() < 0.3);
        assert!(skip_spawns.is_empty());
        assert_eq!(
            skip_animate_actual.next_long(),
            skip_animate_expected.next_long()
        );
        assert_eq!(
            skip_level_actual.next_long(),
            skip_level_expected.next_long()
        );

        set_block(&mut world, pos, partial_beehive_id);
        let mut partial_animate_actual = LevelEventSoundRandomState::with_seed(11);
        let mut partial_animate_expected = LevelEventSoundRandomState::with_seed(11);
        let mut partial_level_actual = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut partial_level_expected = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut partial_spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &partial_properties,
            &mut partial_animate_actual,
            &mut partial_level_actual,
            &mut partial_spawns,
        );
        assert!(partial_spawns.is_empty());
        assert_eq!(
            partial_animate_actual.next_long(),
            partial_animate_expected.next_long(),
            "honey levels below five consume neither random stream"
        );
        assert_eq!(
            partial_level_actual.next_long(),
            partial_level_expected.next_long()
        );

        set_block(&mut world, pos, full_beehive_id);
        set_block(&mut world, below, block_state_id("minecraft:stone", &[]));
        let mut stone_animate = LevelEventSoundRandomState::with_seed(11);
        let mut stone_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut stone_expected_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut stone_spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &full_properties,
            &mut stone_animate,
            &mut stone_level,
            &mut stone_spawns,
        );
        let _ = stone_expected_level.next_float();
        assert!(stone_spawns.is_empty());
        assert_eq!(stone_level.next_long(), stone_expected_level.next_long());

        set_block(
            &mut world,
            below,
            block_state_id(
                "minecraft:oak_slab",
                &[("type", "bottom"), ("waterlogged", "false")],
            ),
        );
        let mut slab_animate = LevelEventSoundRandomState::with_seed(11);
        let mut slab_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut slab_expected_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut slab_spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &full_properties,
            &mut slab_animate,
            &mut slab_level,
            &mut slab_spawns,
        );
        let _ = slab_expected_level.next_float();
        let expected_slab_position = [
            8.0 + slab_expected_level.next_double(),
            31.95,
            8.0 + slab_expected_level.next_double(),
        ];
        assert_eq!(slab_spawns.len(), 1);
        assert_eq!(slab_spawns[0].position, expected_slab_position);
        assert_eq!(slab_level.next_long(), slab_expected_level.next_long());

        set_block(
            &mut world,
            below,
            block_state_id("minecraft:water", &[("level", "0")]),
        );
        let mut water_animate = LevelEventSoundRandomState::with_seed(11);
        let mut water_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut water_expected_level = LevelEventSoundRandomState::with_seed(spawn_seed);
        let mut water_spawns = Vec::new();
        append_beehive_animate_effect(
            &world,
            pos,
            &full_properties,
            &mut water_animate,
            &mut water_level,
            &mut water_spawns,
        );
        let _ = water_expected_level.next_float();
        assert!(water_spawns.is_empty());
        assert_eq!(water_level.next_long(), water_expected_level.next_long());
    }

    #[test]
    fn spore_blossom_animate_effects_preserve_fourteen_collision_samples_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                let _ = random.next_double();
                let _ = random.next_double();
                let first = BlockPos {
                    x: pos.x + random.next_int_bound(21) - 10,
                    y: pos.y - random.next_int_bound(10),
                    z: pos.z + random.next_int_bound(21) - 10,
                };
                (0..16).contains(&first.x) && (0..16).contains(&first.z) && first != pos
            })
            .expect("deterministic loaded first spore-blossom ambient sample");
        let mut first_random = LevelEventSoundRandomState::with_seed(seed);
        let _ = first_random.next_double();
        let _ = first_random.next_double();
        let first_ambient = BlockPos {
            x: pos.x + first_random.next_int_bound(21) - 10,
            y: pos.y - first_random.next_int_bound(10),
            z: pos.z + first_random.next_int_bound(21) - 10,
        };

        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id("minecraft:spore_blossom", &[]),
        );
        set_block(
            &mut world,
            first_ambient,
            block_state_id("minecraft:stone", &[]),
        );
        let mut actual = LevelEventSoundRandomState::with_seed(seed);
        let mut spawns = Vec::new();
        append_spore_blossom_animate_effects(&world, pos, &mut actual, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(seed);
        let mut expected_particles = vec![PARTICLE_FALLING_SPORE_BLOSSOM];
        let mut expected_positions = vec![[
            8.0 + expected.next_double(),
            32.7,
            8.0 + expected.next_double(),
        ]];
        for _ in 0..14 {
            let ambient_pos = BlockPos {
                x: pos.x + expected.next_int_bound(21) - 10,
                y: pos.y - expected.next_int_bound(10),
                z: pos.z + expected.next_int_bound(21) - 10,
            };
            if world.sample_block_collision_shape_is_full_block(ambient_pos) != Some(false) {
                continue;
            }
            expected_particles.push(PARTICLE_SPORE_BLOSSOM_AIR);
            expected_positions.push([
                f64::from(ambient_pos.x) + expected.next_double(),
                f64::from(ambient_pos.y) + expected.next_double(),
                f64::from(ambient_pos.z) + expected.next_double(),
            ]);
        }
        assert!(expected_particles.len() > 1);
        assert!(expected_particles.len() < 15);
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.particle.resource_id.as_str())
                .collect::<Vec<_>>(),
            expected_particles
        );
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_positions
        );
        assert!(spawns
            .iter()
            .all(|spawn| spawn.velocity == [0.0; 3] && !spawn.always_show));
        assert_eq!(actual.next_long(), expected.next_long());

        let stone_id = block_state_id("minecraft:stone", &[]);
        let mut blocked_world = world_with_ambient_particles_and_block_state(stone_id);
        set_block(
            &mut blocked_world,
            pos,
            block_state_id("minecraft:spore_blossom", &[]),
        );
        let mut blocked_actual = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_expected = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_spawns = Vec::new();
        append_spore_blossom_animate_effects(
            &blocked_world,
            pos,
            &mut blocked_actual,
            &mut blocked_spawns,
        );
        let _ = blocked_expected.next_double();
        let _ = blocked_expected.next_double();
        for _ in 0..14 {
            let _ = blocked_expected.next_int_bound(21);
            let _ = blocked_expected.next_int_bound(10);
            let _ = blocked_expected.next_int_bound(21);
        }
        assert_eq!(blocked_spawns.len(), 1);
        assert_eq!(
            blocked_spawns[0].particle.resource_id,
            PARTICLE_FALLING_SPORE_BLOSSOM
        );
        assert_eq!(blocked_actual.next_long(), blocked_expected.next_long());
    }

    #[test]
    fn enchanting_table_animate_effects_preserve_offset_tag_short_circuit_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let offsets = enchanting_table_bookshelf_offsets().collect::<Vec<_>>();
        assert_eq!(offsets.len(), 32);
        assert_eq!(
            &offsets[..6],
            &[
                (-2, 0, -2),
                (-1, 0, -2),
                (0, 0, -2),
                (1, 0, -2),
                (2, 0, -2),
                (-2, 1, -2),
            ]
        );
        assert_eq!(offsets.last(), Some(&(2, 1, 2)));

        let seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                offsets.iter().any(|_| {
                    let succeeds = random.next_int_bound(16) == 0;
                    if succeeds {
                        let _ = random.next_float();
                        let _ = random.next_float();
                        let _ = random.next_float();
                    }
                    succeeds
                })
            })
            .expect("deterministic enchanting-table particle seed");
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id("minecraft:enchanting_table", &[]),
        );
        let bookshelf_id = block_state_id("minecraft:bookshelf", &[]);
        for &(offset_x, offset_y, offset_z) in &offsets {
            set_block(
                &mut world,
                BlockPos {
                    x: pos.x + offset_x,
                    y: pos.y + offset_y,
                    z: pos.z + offset_z,
                },
                bookshelf_id,
            );
        }

        let mut actual = LevelEventSoundRandomState::with_seed(seed);
        let mut spawns = Vec::new();
        append_enchanting_table_animate_effects(&world, pos, &mut actual, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(seed);
        let mut expected_velocities = Vec::new();
        for &(offset_x, offset_y, offset_z) in &offsets {
            if expected.next_int_bound(16) != 0 {
                continue;
            }
            expected_velocities.push([
                f64::from(offset_x as f32 + expected.next_float()) - 0.5,
                f64::from(offset_y as f32 - expected.next_float() - 1.0),
                f64::from(offset_z as f32 + expected.next_float()) - 0.5,
            ]);
        }
        assert!(!expected_velocities.is_empty());
        assert_eq!(spawns.len(), expected_velocities.len());
        assert!(spawns.iter().all(|spawn| {
            spawn.particle.resource_id == PARTICLE_ENCHANT
                && spawn.position == [8.5, 34.0, 8.5]
                && spawn.velocity != [0.0; 3]
                && !spawn.always_show
        }));
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.velocity)
                .collect::<Vec<_>>(),
            expected_velocities
        );
        assert_eq!(actual.next_long(), expected.next_long());

        let first_offset = offsets[0];
        let first_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(16) == 0)
            .expect("deterministic first bookshelf chance seed");
        let mut blocked_world = world_with_ambient_particles();
        set_block(
            &mut blocked_world,
            pos,
            block_state_id("minecraft:enchanting_table", &[]),
        );
        set_block(
            &mut blocked_world,
            BlockPos {
                x: pos.x + first_offset.0,
                y: pos.y + first_offset.1,
                z: pos.z + first_offset.2,
            },
            bookshelf_id,
        );
        let transmitter_pos = BlockPos {
            x: pos.x + first_offset.0 / 2,
            y: pos.y + first_offset.1,
            z: pos.z + first_offset.2 / 2,
        };
        set_block(
            &mut blocked_world,
            transmitter_pos,
            block_state_id("minecraft:stone", &[]),
        );
        let mut blocked_actual = LevelEventSoundRandomState::with_seed(first_seed);
        let mut blocked_expected = LevelEventSoundRandomState::with_seed(first_seed);
        let mut blocked_spawns = Vec::new();
        append_enchanting_table_animate_effects(
            &blocked_world,
            pos,
            &mut blocked_actual,
            &mut blocked_spawns,
        );
        for _ in 0..32 {
            let _ = blocked_expected.next_int_bound(16);
        }
        assert!(blocked_spawns.is_empty());
        assert_eq!(blocked_actual.next_long(), blocked_expected.next_long());

        set_block(&mut blocked_world, transmitter_pos, 0);
        let mut open_actual = LevelEventSoundRandomState::with_seed(first_seed);
        let mut open_expected = LevelEventSoundRandomState::with_seed(first_seed);
        let mut open_spawns = Vec::new();
        append_enchanting_table_animate_effects(
            &blocked_world,
            pos,
            &mut open_actual,
            &mut open_spawns,
        );
        assert_eq!(open_expected.next_int_bound(16), 0);
        let expected_first_velocity = [
            f64::from(first_offset.0 as f32 + open_expected.next_float()) - 0.5,
            f64::from(first_offset.1 as f32 - open_expected.next_float() - 1.0),
            f64::from(first_offset.2 as f32 + open_expected.next_float()) - 0.5,
        ];
        for _ in 1..32 {
            let _ = open_expected.next_int_bound(16);
        }
        assert_eq!(open_spawns.len(), 1);
        assert_eq!(open_spawns[0].velocity, expected_first_velocity);
        assert_eq!(open_actual.next_long(), open_expected.next_long());
    }

    #[test]
    fn end_rod_animate_effect_preserves_float_math_directions_and_gaussian_cache() {
        let pos = BlockPos { x: 8, y: 32, z: -4 };
        let seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                for _ in 0..5 {
                    let _ = random.next_float();
                }
                random.next_int_bound(5) == 0
            })
            .expect("deterministic end-rod particle seed");
        let facings = [
            ("down", [0, -1, 0]),
            ("up", [0, 1, 0]),
            ("north", [0, 0, -1]),
            ("south", [0, 0, 1]),
            ("west", [-1, 0, 0]),
            ("east", [1, 0, 0]),
        ];

        for (facing, direction) in facings {
            let properties = BTreeMap::from([("facing".to_string(), facing.to_string())]);
            let mut actual = LevelEventSoundRandomState::with_seed(seed);
            let mut spawns = Vec::new();
            append_end_rod_animate_effect(pos, &properties, &mut actual, &mut spawns);

            let mut expected = LevelEventSoundRandomState::with_seed(seed);
            let x = 8.55 - f64::from(expected.next_float() * 0.1_f32);
            let y = 32.55 - f64::from(expected.next_float() * 0.1_f32);
            let z = -3.45 - f64::from(expected.next_float() * 0.1_f32);
            let r = f64::from(0.4_f32 - (expected.next_float() + expected.next_float()) * 0.4_f32);
            assert_eq!(expected.next_int_bound(5), 0);
            let expected_position = [
                x + f64::from(direction[0]) * r,
                y + f64::from(direction[1]) * r,
                z + f64::from(direction[2]) * r,
            ];
            let expected_velocity = [
                expected.next_gaussian() * 0.005,
                expected.next_gaussian() * 0.005,
                expected.next_gaussian() * 0.005,
            ];

            let [spawn] = spawns.as_slice() else {
                panic!("{facing} end rod should emit one particle");
            };
            assert_eq!(spawn.particle.resource_id, PARTICLE_END_ROD);
            assert_eq!(spawn.position, expected_position);
            assert_eq!(spawn.velocity, expected_velocity);
            assert!(!spawn.always_show);
            assert_eq!(
                actual.next_gaussian(),
                expected.next_gaussian(),
                "third Gaussian draw must leave the same cached pair value"
            );
            assert_eq!(actual.next_long(), expected.next_long());
        }

        let miss_seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                for _ in 0..5 {
                    let _ = random.next_float();
                }
                random.next_int_bound(5) != 0
            })
            .expect("deterministic end-rod miss seed");
        let properties = BTreeMap::from([("facing".to_string(), "up".to_string())]);
        let mut miss_actual = LevelEventSoundRandomState::with_seed(miss_seed);
        let mut miss_expected = LevelEventSoundRandomState::with_seed(miss_seed);
        let mut miss_spawns = Vec::new();
        append_end_rod_animate_effect(pos, &properties, &mut miss_actual, &mut miss_spawns);
        for _ in 0..5 {
            let _ = miss_expected.next_float();
        }
        assert_ne!(miss_expected.next_int_bound(5), 0);
        assert!(miss_spawns.is_empty());
        assert_eq!(miss_actual.next_gaussian(), miss_expected.next_gaussian());
        assert_eq!(miss_actual.next_long(), miss_expected.next_long());

        let invalid = BTreeMap::from([("facing".to_string(), "invalid".to_string())]);
        let mut invalid_actual = LevelEventSoundRandomState::with_seed(seed);
        let mut invalid_expected = LevelEventSoundRandomState::with_seed(seed);
        let mut invalid_spawns = Vec::new();
        append_end_rod_animate_effect(pos, &invalid, &mut invalid_actual, &mut invalid_spawns);
        assert!(invalid_spawns.is_empty());
        assert_eq!(invalid_actual.next_long(), invalid_expected.next_long());
    }

    #[test]
    fn environment_dispatches_end_rod_animate_effect() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id("minecraft:end_rod", &[("facing", "east")]),
        );
        let seed = (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                for _ in 0..5 {
                    let _ = random.next_float();
                }
                random.next_int_bound(5) == 0
            })
            .expect("deterministic end-rod particle seed");
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(seed);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == PARTICLE_END_ROD)
                .count(),
            1
        );
    }

    #[test]
    fn marker_target_spawns_centered_block_marker_even_for_a_full_collision_block() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let barrier_id = block_state_id("minecraft:barrier", &[("waterlogged", "false")]);
        let mut world = world_with_ambient_particles();
        set_block(&mut world, pos, barrier_id);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(3);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            Some("minecraft:barrier"),
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        let [spawn] = spawns.as_slice() else {
            panic!("matching barrier should emit exactly the marker particle");
        };
        assert_eq!(spawn.particle.resource_id, PARTICLE_BLOCK_MARKER);
        assert_eq!(
            spawn.particle.option,
            bbb_world::ClientParticleOptionValue::Block {
                block_state_id: barrier_id
            }
        );
        assert_eq!(
            spawn.position,
            [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 0.5,
                f64::from(pos.z) + 0.5,
            ]
        );
        assert_eq!(spawn.velocity, [0.0; 3]);
        assert!(!spawn.always_show);
        assert!(spawn.color_source.is_none());
    }

    #[test]
    fn marker_target_carries_the_sampled_state_and_ignores_non_matching_blocks() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let lit_light_id = block_state_id(
            "minecraft:light",
            &[("level", "7"), ("waterlogged", "false")],
        );
        let mut world = world_with_ambient_particles();
        set_block(&mut world, pos, lit_light_id);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(3);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            Some("minecraft:light"),
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        let markers = spawns
            .iter()
            .filter(|spawn| spawn.particle.resource_id == PARTICLE_BLOCK_MARKER)
            .collect::<Vec<_>>();
        let [marker] = markers.as_slice() else {
            panic!("matching light should emit exactly one marker particle");
        };
        assert_eq!(
            marker.particle.option,
            bbb_world::ClientParticleOptionValue::Block {
                block_state_id: lit_light_id
            }
        );

        let mut mismatch_runtime = EnvironmentAnimateRuntime::default();
        let mut mismatch_random = LevelEventSoundRandomState::with_seed(3);
        let mut mismatch_spawns = Vec::new();
        mismatch_runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            Some("minecraft:barrier"),
            &mut mismatch_random,
            &mut audio,
            &mut mismatch_spawns,
        );
        assert!(!mismatch_spawns
            .iter()
            .any(|spawn| spawn.particle.resource_id == PARTICLE_BLOCK_MARKER));

        let mut disabled_runtime = EnvironmentAnimateRuntime::default();
        let mut disabled_random = LevelEventSoundRandomState::with_seed(3);
        let mut disabled_spawns = Vec::new();
        disabled_runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut disabled_random,
            &mut audio,
            &mut disabled_spawns,
        );
        assert!(!disabled_spawns
            .iter()
            .any(|spawn| spawn.particle.resource_id == PARTICLE_BLOCK_MARKER));
    }

    #[test]
    fn falling_dust_preserves_free_gate_block_option_coordinates_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let sand_id = block_state_id("minecraft:sand", &[]);
        let chance_seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(FALLING_DUST_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic falling-dust chance seed");
        let mut world = world_with_ambient_particles();
        let mut actual = LevelEventSoundRandomState::with_seed(chance_seed);
        let mut expected = LevelEventSoundRandomState::with_seed(chance_seed);
        let mut spawns = Vec::new();

        append_falling_dust_animate_effect(&world, pos, sand_id, &mut actual, &mut spawns);

        assert_eq!(expected.next_int_bound(FALLING_DUST_CHANCE_BOUND), 0);
        let expected_position = [
            f64::from(pos.x) + expected.next_double(),
            f64::from(pos.y) - 0.05,
            f64::from(pos.z) + expected.next_double(),
        ];
        let [spawn] = spawns.as_slice() else {
            panic!("free block below should emit one falling-dust particle");
        };
        assert_eq!(spawn.particle.resource_id, PARTICLE_FALLING_DUST);
        assert_eq!(
            spawn.particle.option,
            bbb_world::ClientParticleOptionValue::Block {
                block_state_id: sand_id
            }
        );
        assert_eq!(spawn.position, expected_position);
        assert_eq!(spawn.velocity, [0.0; 3]);
        assert!(!spawn.always_show);
        assert_eq!(actual.next_long(), expected.next_long());

        set_block(
            &mut world,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:stone", &[]),
        );
        let mut blocked_actual = LevelEventSoundRandomState::with_seed(chance_seed);
        let mut blocked_expected = LevelEventSoundRandomState::with_seed(chance_seed);
        let mut blocked_spawns = Vec::new();
        append_falling_dust_animate_effect(
            &world,
            pos,
            sand_id,
            &mut blocked_actual,
            &mut blocked_spawns,
        );
        assert_eq!(
            blocked_expected.next_int_bound(FALLING_DUST_CHANCE_BOUND),
            0
        );
        assert!(blocked_spawns.is_empty());
        assert_eq!(blocked_actual.next_long(), blocked_expected.next_long());
    }

    #[test]
    fn falling_block_dispatch_emits_typed_dust_from_the_animate_random_stream() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let gravel_id = block_state_id("minecraft:gravel", &[]);
        let chance_seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(FALLING_DUST_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic falling-dust dispatch seed");
        let mut world = world_with_ambient_particles();
        set_block(&mut world, pos, gravel_id);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(chance_seed);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        assert!(spawns.iter().any(|spawn| {
            spawn.particle.resource_id == PARTICLE_FALLING_DUST
                && spawn.particle.option
                    == bbb_world::ClientParticleOptionValue::Block {
                        block_state_id: gravel_id,
                    }
        }));
    }

    #[test]
    fn desert_sand_columns_require_all_four_vanilla_direction_matches() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let mut world = world_with_desert_sand_ring(pos);
        assert!(should_play_ambient_desert_sand_sound(&world, pos));

        set_block(
            &mut world,
            BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z - 8,
            },
            block_state_id("minecraft:stone", &[]),
        );
        assert!(!should_play_ambient_desert_sand_sound(&world, pos));

        let far_column = BlockPos { x: 8, y: 32, z: 8 };
        let mut far_world = world_with_surface(far_column, 60, 0);
        set_block(
            &mut far_world,
            BlockPos {
                y: far_column.y + 3,
                ..far_column
            },
            block_state_id("minecraft:red_sand", &[]),
        );
        assert!(column_contains_ambient_desert_sand_trigger(
            &far_world, far_column
        ));
        set_block(
            &mut far_world,
            BlockPos {
                y: far_column.y + 4,
                ..far_column
            },
            block_state_id("minecraft:stone", &[]),
        );
        assert!(!column_contains_ambient_desert_sand_trigger(
            &far_world, far_column
        ));
    }

    #[test]
    fn desert_ambient_sounds_preserve_position_entity_binding_and_level_seeds() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let sand_chance_seed = (0..1_000_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(DESERT_SAND_SOUND_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic ambient sand sound seed");
        let dry_grass_chance_seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(DESERT_DRY_GRASS_SOUND_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic ambient dry-grass sound seed");
        let dead_bush_chance_seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(DESERT_DEAD_BUSH_SOUND_CHANCE_BOUND)
                    == 0
            })
            .expect("deterministic ambient dead-bush sound seed");

        let mut sand_world = world_with_desert_sand_ring(pos);
        let mut sand_runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(91),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut sand_random = LevelEventSoundRandomState::with_seed(sand_chance_seed);
        let mut sand_recording = RecordingEnvironmentAudio::default();
        {
            let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut sand_recording);
            sand_runtime.animate_desert_sand(&sand_world, pos, &mut sand_random, &mut audio);
        }
        let mut sand_level_expected = LevelEventSoundRandomState::with_seed(91);
        let [sand_sound] = sand_recording.positioned.as_slice() else {
            panic!("four matching sand columns should emit the positioned idle sound");
        };
        assert_eq!(sand_sound.source, SoundSource::Ambient.as_str());
        assert_eq!(
            [
                sand_sound.position.x,
                sand_sound.position.y,
                sand_sound.position.z,
            ],
            block_position(pos)
        );
        assert_eq!(sand_sound.seed, sand_level_expected.next_long());
        assert!(sand_recording.entity.is_empty());
        assert_eq!(
            sand_runtime.level_random.next_long(),
            sand_level_expected.next_long()
        );

        for y in [pos.y - 1, pos.y - 2] {
            set_block(
                &mut sand_world,
                BlockPos { y, ..pos },
                block_state_id("minecraft:sand", &[]),
            );
        }
        let mut dry_runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(37),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut dry_random = LevelEventSoundRandomState::with_seed(dry_grass_chance_seed);
        let mut dry_recording = RecordingEnvironmentAudio::default();
        {
            let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut dry_recording);
            dry_runtime.animate_desert_dry_grass(&sand_world, pos, &mut dry_random, &mut audio);
        }
        let mut dry_level_expected = LevelEventSoundRandomState::with_seed(37);
        let [(dry_sound, dry_position)] = dry_recording.entity.as_slice() else {
            panic!("supported dry grass should emit one player-bound sound");
        };
        assert_eq!(dry_sound.source, SoundSource::Ambient.as_str());
        assert_eq!(dry_sound.entity_id, 42);
        assert_eq!(dry_sound.seed, dry_level_expected.next_long());
        assert_eq!(*dry_position, Some([1.0, 32.0, 1.0]));
        assert!(dry_recording.positioned.is_empty());
        assert_eq!(
            dry_runtime.level_random.next_long(),
            dry_level_expected.next_long()
        );

        let mut dead_runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(73),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut dead_random = LevelEventSoundRandomState::with_seed(dead_bush_chance_seed);
        let mut dead_recording = RecordingEnvironmentAudio::default();
        {
            let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut dead_recording);
            dead_runtime.animate_desert_dead_bush(&sand_world, pos, &mut dead_random, &mut audio);
        }
        let mut dead_level_expected = LevelEventSoundRandomState::with_seed(73);
        let [dead_sound] = dead_recording.positioned.as_slice() else {
            panic!("supported dead bush should emit one positioned idle sound");
        };
        assert_eq!(dead_sound.source, SoundSource::Ambient.as_str());
        assert_eq!(
            [
                dead_sound.position.x,
                dead_sound.position.y,
                dead_sound.position.z,
            ],
            block_position(pos)
        );
        assert_eq!(dead_sound.seed, dead_level_expected.next_long());
        assert!(dead_recording.entity.is_empty());
        assert_eq!(
            dead_runtime.level_random.next_long(),
            dead_level_expected.next_long()
        );

        for y in [pos.y - 1, pos.y - 2] {
            set_block(
                &mut sand_world,
                BlockPos { y, ..pos },
                block_state_id("minecraft:red_sand", &[]),
            );
        }
        let badlands_seed = (0..1_000_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                random.next_int_bound(DESERT_DEAD_BUSH_SOUND_CHANCE_BOUND) == 0
                    && random.next_int_bound(DESERT_DEAD_BUSH_BADLANDS_DECREASED_CHANCE_BOUND) != 0
            })
            .expect("deterministic badlands dead-bush suppression seed");
        let mut badlands_runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(19),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut badlands_actual = LevelEventSoundRandomState::with_seed(badlands_seed);
        let mut badlands_expected = LevelEventSoundRandomState::with_seed(badlands_seed);
        let mut badlands_recording = RecordingEnvironmentAudio::default();
        {
            let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut badlands_recording);
            badlands_runtime.animate_desert_dead_bush(
                &sand_world,
                pos,
                &mut badlands_actual,
                &mut audio,
            );
        }
        assert_eq!(
            badlands_expected.next_int_bound(DESERT_DEAD_BUSH_SOUND_CHANCE_BOUND),
            0
        );
        assert_ne!(
            badlands_expected.next_int_bound(DESERT_DEAD_BUSH_BADLANDS_DECREASED_CHANCE_BOUND),
            0
        );
        assert_eq!(badlands_actual.next_long(), badlands_expected.next_long());
        assert!(badlands_recording.positioned.is_empty());
        assert!(badlands_recording.entity.is_empty());
        let mut badlands_level_expected = LevelEventSoundRandomState::with_seed(19);
        assert_eq!(
            badlands_runtime.level_random.next_long(),
            badlands_level_expected.next_long()
        );
    }

    #[test]
    fn candle_animate_effects_preserve_wick_order_chance_branches_and_random_order() {
        assert_eq!(candle_wick_offsets(1, false), &[[8.0, 8.0, 8.0]]);
        assert_eq!(
            candle_wick_offsets(2, false),
            &[[6.0, 7.0, 8.0], [10.0, 8.0, 7.0]]
        );
        assert_eq!(
            candle_wick_offsets(3, false),
            &[[8.0, 5.0, 10.0], [6.0, 7.0, 8.0], [9.0, 8.0, 7.0]]
        );
        assert_eq!(
            candle_wick_offsets(4, false),
            &[
                [7.0, 5.0, 9.0],
                [10.0, 7.0, 9.0],
                [6.0, 7.0, 6.0],
                [9.0, 8.0, 6.0],
            ]
        );
        assert_eq!(candle_wick_offsets(1, true), &[[8.0, 16.0, 8.0]]);

        let seed = (0..1_000_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                let mut saw_sound = false;
                let mut saw_smoke_only = false;
                let mut saw_flame_only = false;
                for _ in 0..4 {
                    let chance = random.next_float();
                    if chance < 0.17 {
                        saw_sound = true;
                        let _ = random.next_float();
                        let _ = random.next_float();
                    } else if chance < 0.3 {
                        saw_smoke_only = true;
                    } else {
                        saw_flame_only = true;
                    }
                }
                saw_sound && saw_smoke_only && saw_flame_only
            })
            .expect("deterministic candle branch coverage seed");
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let properties = BTreeMap::from([
            ("candles".to_string(), "4".to_string()),
            ("lit".to_string(), "true".to_string()),
        ]);
        let mut actual = LevelEventSoundRandomState::with_seed(seed);
        let mut spawns = Vec::new();
        let sounds =
            append_candle_animate_effects(pos, &properties, false, &mut actual, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(seed);
        let mut expected_spawns = Vec::new();
        let mut expected_sounds = Vec::new();
        for offset in candle_wick_offsets(4, false) {
            let position = [
                8.0 + offset[0] / 16.0,
                32.0 + offset[1] / 16.0,
                8.0 + offset[2] / 16.0,
            ];
            let chance = expected.next_float();
            if chance < 0.3 {
                expected_spawns.push((PARTICLE_SMOKE, position));
                if chance < 0.17 {
                    expected_sounds.push((
                        position.map(|coordinate| coordinate + 0.5),
                        1.0_f32 + expected.next_float(),
                        expected.next_float() * 0.7_f32 + 0.3_f32,
                    ));
                }
            }
            expected_spawns.push((PARTICLE_SMALL_FLAME, position));
        }
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| (spawn.particle.resource_id.as_str(), spawn.position))
                .collect::<Vec<_>>(),
            expected_spawns
        );
        assert!(spawns
            .iter()
            .all(|spawn| spawn.velocity == [0.0; 3] && !spawn.always_show));
        assert_eq!(sounds, expected_sounds);
        assert_eq!(actual.next_long(), expected.next_long());

        let cake_properties = BTreeMap::from([("lit".to_string(), "true".to_string())]);
        let mut cake_random = LevelEventSoundRandomState::with_seed(seed);
        let mut cake_spawns = Vec::new();
        let _ = append_candle_animate_effects(
            pos,
            &cake_properties,
            true,
            &mut cake_random,
            &mut cake_spawns,
        );
        assert_eq!(
            cake_spawns.last().map(|spawn| spawn.position),
            Some([8.5, 33.0, 8.5])
        );

        let unlit = BTreeMap::from([
            ("candles".to_string(), "4".to_string()),
            ("lit".to_string(), "false".to_string()),
        ]);
        let mut unlit_actual = LevelEventSoundRandomState::with_seed(seed);
        let mut unlit_expected = LevelEventSoundRandomState::with_seed(seed);
        let mut unlit_spawns = Vec::new();
        assert!(append_candle_animate_effects(
            pos,
            &unlit,
            false,
            &mut unlit_actual,
            &mut unlit_spawns,
        )
        .is_empty());
        assert!(unlit_spawns.is_empty());
        assert_eq!(unlit_actual.next_long(), unlit_expected.next_long());
    }

    #[test]
    fn candle_animate_sound_reaches_audio_sink_with_shared_level_seed() {
        let animate_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.17)
            .expect("deterministic candle sound seed");
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let properties = BTreeMap::from([
            ("candles".to_string(), "1".to_string()),
            ("lit".to_string(), "true".to_string()),
        ]);
        let world = WorldStore::default();
        let expected_sound_registry_id = world
            .resolved_registry_entry_by_resource_id(
                "minecraft:sound_event",
                SOUND_EVENT_CANDLE_AMBIENT,
            )
            .map(|entry| entry.holder_id);
        let mut runtime = EnvironmentAnimateRuntime {
            level_random: LevelEventSoundRandomState::with_seed(91),
            ..EnvironmentAnimateRuntime::default()
        };
        let mut animate_actual = LevelEventSoundRandomState::with_seed(animate_seed);
        let mut animate_expected = LevelEventSoundRandomState::with_seed(animate_seed);
        let mut recording = RecordingEnvironmentAudio::default();
        let mut audio: Option<&mut dyn AudioEventSink> = Some(&mut recording);
        let mut spawns = Vec::new();

        runtime.animate_candle(
            &world,
            pos,
            &properties,
            false,
            &mut animate_actual,
            &mut audio,
            &mut spawns,
        );

        assert!(animate_expected.next_float() < 0.17);
        let expected_volume = 1.0_f32 + animate_expected.next_float();
        let expected_pitch = animate_expected.next_float() * 0.7_f32 + 0.3_f32;
        assert_eq!(animate_actual.next_long(), animate_expected.next_long());
        let mut level_expected = LevelEventSoundRandomState::with_seed(91);
        let expected_seed = level_expected.next_long();
        let [sound] = recording.positioned.as_slice() else {
            panic!("candle sound should reach the positioned audio sink");
        };
        assert_eq!(sound.sound.kind, "reference");
        assert_eq!(sound.sound.registry_id, expected_sound_registry_id);
        assert_eq!(sound.source, SoundSource::Blocks.as_str());
        assert_eq!(
            [sound.position.x, sound.position.y, sound.position.z],
            [9.0, 33.0, 9.0]
        );
        assert_eq!(sound.volume, expected_volume);
        assert_eq!(sound.pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(runtime.level_random.next_long(), level_expected.next_long());
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.particle.resource_id.as_str())
                .collect::<Vec<_>>(),
            vec![PARTICLE_SMOKE, PARTICLE_SMALL_FLAME]
        );
    }

    #[test]
    fn environment_dispatches_normal_and_cake_candle_wicks() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        for (name, properties, expected_flames) in [
            (
                "minecraft:blue_candle",
                vec![("candles", "4"), ("lit", "true"), ("waterlogged", "false")],
                4,
            ),
            ("minecraft:black_candle_cake", vec![("lit", "true")], 1),
        ] {
            let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
            let pos =
                sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
            let mut world = world_with_ambient_particles();
            set_block(&mut world, pos, block_state_id(name, &properties));
            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut animate_random = LevelEventSoundRandomState::with_seed(42);
            let mut audio = None;
            let mut spawns = Vec::new();

            runtime.do_animate_tick(
                &mut world,
                center,
                NEAR_SAMPLE_RADIUS,
                0,
                None,
                &mut animate_random,
                &mut audio,
                &mut spawns,
            );

            assert_eq!(
                spawns
                    .iter()
                    .filter(|spawn| spawn.particle.resource_id == PARTICLE_SMALL_FLAME)
                    .count(),
                expected_flames,
                "{name}"
            );
        }
    }

    #[test]
    fn ender_chest_animate_effects_preserve_float_math_signs_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let mut actual = LevelEventSoundRandomState::with_seed(73);
        let mut spawns = Vec::new();
        append_ender_chest_animate_effects(pos, &mut actual, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(73);
        let expected_spawns = (0..3)
            .map(|_| {
                let flip_x = expected.next_int_bound(2) * 2 - 1;
                let flip_z = expected.next_int_bound(2) * 2 - 1;
                (
                    [
                        8.5 + 0.25 * f64::from(flip_x),
                        32.0 + f64::from(expected.next_float()),
                        8.5 + 0.25 * f64::from(flip_z),
                    ],
                    [
                        f64::from(expected.next_float() * flip_x as f32),
                        f64::from((expected.next_float() - 0.5_f32) * 0.125_f32),
                        f64::from(expected.next_float() * flip_z as f32),
                    ],
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| (spawn.position, spawn.velocity))
                .collect::<Vec<_>>(),
            expected_spawns
        );
        assert!(spawns
            .iter()
            .all(|spawn| { spawn.particle.resource_id == PARTICLE_PORTAL && !spawn.always_show }));
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn environment_dispatches_ender_chest_portal_particles_from_animate_random() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::default();
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id(
                "minecraft:ender_chest",
                &[("facing", "north"), ("waterlogged", "false")],
            ),
        );
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut actual = LevelEventSoundRandomState::with_seed(91);
        let mut expected = LevelEventSoundRandomState::with_seed(91);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut actual,
            &mut audio,
            &mut spawns,
        );
        let mut expected_portals = Vec::new();
        append_ender_chest_animate_effects(pos, &mut expected, &mut expected_portals);

        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == PARTICLE_PORTAL)
                .count(),
            3
        );
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn end_portal_and_gateway_animate_effects_preserve_vanilla_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let mut portal_actual = LevelEventSoundRandomState::with_seed(73);
        let mut portal_expected = LevelEventSoundRandomState::with_seed(73);
        let mut spawns = Vec::new();
        append_end_portal_animate_effect(pos, &mut portal_actual, &mut spawns);
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_SMOKE);
        assert_eq!(
            spawns[0].position,
            [
                f64::from(pos.x) + portal_expected.next_double(),
                f64::from(pos.y) + 0.8,
                f64::from(pos.z) + portal_expected.next_double(),
            ]
        );
        assert_eq!(spawns[0].velocity, [0.0; 3]);
        assert!(!spawns[0].always_show);
        assert_eq!(portal_actual.next_long(), portal_expected.next_long());

        let mut gateway_actual = LevelEventSoundRandomState::with_seed(91);
        let mut gateway_expected = LevelEventSoundRandomState::with_seed(91);
        spawns.clear();
        append_end_gateway_animate_effects(pos, 3, &mut gateway_actual, &mut spawns);
        let expected_spawns = (0..3)
            .map(|_| {
                let mut position = [
                    f64::from(pos.x) + gateway_expected.next_double(),
                    f64::from(pos.y) + gateway_expected.next_double(),
                    f64::from(pos.z) + gateway_expected.next_double(),
                ];
                let mut velocity = [
                    (gateway_expected.next_double() - 0.5) * 0.5,
                    (gateway_expected.next_double() - 0.5) * 0.5,
                    (gateway_expected.next_double() - 0.5) * 0.5,
                ];
                let flip = gateway_expected.next_int_bound(2) * 2 - 1;
                if gateway_expected.next_boolean() {
                    position[2] = f64::from(pos.z) + 0.5 + 0.25 * f64::from(flip);
                    velocity[2] = f64::from(gateway_expected.next_float() * 2.0 * flip as f32);
                } else {
                    position[0] = f64::from(pos.x) + 0.5 + 0.25 * f64::from(flip);
                    velocity[0] = f64::from(gateway_expected.next_float() * 2.0 * flip as f32);
                }
                (position, velocity)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| (spawn.position, spawn.velocity))
                .collect::<Vec<_>>(),
            expected_spawns
        );
        assert!(spawns
            .iter()
            .all(|spawn| spawn.particle.resource_id == PARTICLE_PORTAL && !spawn.always_show));
        assert_eq!(gateway_actual.next_long(), gateway_expected.next_long());
    }

    #[test]
    fn environment_dispatches_end_portal_family_particles_from_animate_random() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        for (block_name, expected_particle, expected_count) in [
            ("minecraft:end_portal", PARTICLE_SMOKE, 1),
            ("minecraft:end_gateway", PARTICLE_PORTAL, 6),
        ] {
            let mut sampled_level_random = LevelEventSoundRandomState::default();
            let pos =
                sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
            let mut world = world_with_ambient_particles();
            set_block(&mut world, pos, block_state_id(block_name, &[]));
            world.advance_end_gateway_ticks(0);
            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut actual = LevelEventSoundRandomState::with_seed(107);
            let mut expected = LevelEventSoundRandomState::with_seed(107);
            let mut audio = None;
            let mut spawns = Vec::new();

            runtime.do_animate_tick(
                &mut world,
                center,
                NEAR_SAMPLE_RADIUS,
                0,
                None,
                &mut actual,
                &mut audio,
                &mut spawns,
            );
            if block_name == "minecraft:end_portal" {
                let _ = expected.next_double();
                let _ = expected.next_double();
            } else {
                for _ in 0..expected_count {
                    for _ in 0..6 {
                        let _ = expected.next_double();
                    }
                    let _ = expected.next_int_bound(2);
                    let _ = expected.next_boolean();
                    let _ = expected.next_float();
                }
            }

            assert_eq!(
                spawns
                    .iter()
                    .filter(|spawn| spawn.particle.resource_id == expected_particle)
                    .count(),
                expected_count,
                "{block_name}"
            );
            assert_eq!(actual.next_long(), expected.next_long(), "{block_name}");
        }
    }

    #[test]
    fn leaves_animate_effects_preserve_rain_leaf_gates_options_and_random_order() {
        let pos = BlockPos {
            x: 8,
            y: WorldDimension::default().min_y + WorldDimension::default().height - 1,
            z: 8,
        };
        let mut world = raining_world_for_leaf(pos);
        let success_seed = (0..1_000_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                random.next_int_bound(15) == 1 && {
                    let _ = random.next_double();
                    let _ = random.next_double();
                    random.next_float() < 0.01
                }
            })
            .expect("deterministic rain and oak-leaf success seed");
        let oak = leaf_animate_particle_profile("minecraft:oak_leaves").unwrap();
        let mut actual = LevelEventSoundRandomState::with_seed(success_seed);
        let mut expected = LevelEventSoundRandomState::with_seed(success_seed);
        let mut spawns = Vec::new();

        append_leaves_animate_effects(&world, pos, oak, &mut actual, &mut spawns);

        assert_eq!(expected.next_int_bound(15), 1);
        let expected_water_position = [
            f64::from(pos.x) + expected.next_double(),
            f64::from(pos.y) - 0.05,
            f64::from(pos.z) + expected.next_double(),
        ];
        assert!(expected.next_float() < oak.leaf_particle_chance());
        let expected_leaf_position = [
            f64::from(pos.x) + expected.next_double(),
            f64::from(pos.y) - 0.05,
            f64::from(pos.z) + expected.next_double(),
        ];
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_DRIPPING_WATER);
        assert_eq!(spawns[0].position, expected_water_position);
        assert_eq!(spawns[0].color_source, None);
        assert_eq!(spawns[1].particle.resource_id, "minecraft:tinted_leaves");
        assert_eq!(
            spawns[1].particle.option,
            bbb_world::ClientParticleOptionValue::Color { argb: u32::MAX }
        );
        assert_eq!(spawns[1].position, expected_leaf_position);
        assert_eq!(
            spawns[1].color_source,
            Some(EnvironmentParticleColorSource::ClientLeaf {
                block_pos: pos,
                block_resource_id: "minecraft:oak_leaves",
            })
        );
        assert_eq!(actual.next_long(), expected.next_long());

        set_block(
            &mut world,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:glass", &[]),
        );
        let mut glass_actual = LevelEventSoundRandomState::with_seed(success_seed);
        let mut glass_expected = LevelEventSoundRandomState::with_seed(success_seed);
        spawns.clear();
        append_leaves_animate_effects(&world, pos, oak, &mut glass_actual, &mut spawns);
        assert_eq!(glass_expected.next_int_bound(15), 1);
        let _ = glass_expected.next_double();
        let _ = glass_expected.next_double();
        let _ = glass_expected.next_float();
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].particle.resource_id, PARTICLE_DRIPPING_WATER);
        assert_eq!(glass_actual.next_long(), glass_expected.next_long());

        set_block(
            &mut world,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:stone", &[]),
        );
        let mut stone_actual = LevelEventSoundRandomState::with_seed(success_seed);
        let mut stone_expected = LevelEventSoundRandomState::with_seed(success_seed);
        spawns.clear();
        append_leaves_animate_effects(&world, pos, oak, &mut stone_actual, &mut spawns);
        assert_eq!(stone_expected.next_int_bound(15), 1);
        let _ = stone_expected.next_float();
        assert!(spawns.is_empty());
        assert_eq!(stone_actual.next_long(), stone_expected.next_long());
    }

    #[test]
    fn leaf_particles_preserve_untinted_and_fixed_color_profiles() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let world = world_with_ambient_particles();
        let success_seed = (0..10_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.01)
            .expect("deterministic leaf chance seed");

        for (block_name, resource_id, option) in [
            (
                "minecraft:cherry_leaves",
                "minecraft:cherry_leaves",
                bbb_world::ClientParticleOptionValue::Empty,
            ),
            (
                "minecraft:azalea_leaves",
                "minecraft:tinted_leaves",
                bbb_world::ClientParticleOptionValue::Color { argb: 0xff70_922d },
            ),
        ] {
            let profile = leaf_animate_particle_profile(block_name).unwrap();
            let mut actual = LevelEventSoundRandomState::with_seed(success_seed);
            let mut expected = LevelEventSoundRandomState::with_seed(success_seed);
            let mut spawns = Vec::new();

            append_leaves_animate_effects(&world, pos, profile, &mut actual, &mut spawns);

            assert!(expected.next_float() < profile.leaf_particle_chance());
            let expected_position = [
                f64::from(pos.x) + expected.next_double(),
                f64::from(pos.y) - 0.05,
                f64::from(pos.z) + expected.next_double(),
            ];
            let [spawn] = spawns.as_slice() else {
                panic!("{block_name} should emit one leaf particle");
            };
            assert_eq!(spawn.particle.resource_id, resource_id);
            assert_eq!(spawn.particle.option, option);
            assert_eq!(spawn.position, expected_position);
            assert_eq!(spawn.color_source, None);
            assert_eq!(actual.next_long(), expected.next_long());
        }
    }

    #[test]
    fn environment_dispatches_leaf_particles_from_animate_random() {
        let mut sampled_level_random = LevelEventSoundRandomState::default();
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id(
                "minecraft:cherry_leaves",
                &[
                    ("distance", "7"),
                    ("persistent", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        );
        let success_seed = (0..10_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_float() < 0.1)
            .expect("deterministic cherry leaf chance seed");
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut actual = LevelEventSoundRandomState::with_seed(success_seed);
        let mut expected = LevelEventSoundRandomState::with_seed(success_seed);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut actual,
            &mut audio,
            &mut spawns,
        );

        assert!(expected.next_float() < 0.1);
        let _ = expected.next_double();
        let _ = expected.next_double();
        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == "minecraft:cherry_leaves")
                .count(),
            1
        );
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn lightning_rod_animate_effects_preserve_weather_surface_axis_and_level_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let world = thundering_world_with_surface(pos, pos.y + 1, 199);

        for (facing, axis) in [("east", 0), ("up", 1), ("north", 2)] {
            let properties = BTreeMap::from([("facing".to_string(), facing.to_string())]);
            let mut actual = LevelEventSoundRandomState::with_seed(73);
            let mut spawns = Vec::new();
            append_lightning_rod_animate_effects(
                &world,
                pos,
                &properties,
                &mut actual,
                &mut spawns,
            );

            let mut expected = LevelEventSoundRandomState::with_seed(73);
            assert!(expected.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE) <= 199);
            let particle_count = expected.next_int_bound(2) + 1;
            let expected_spawns = (0..particle_count)
                .map(|_| expected_axis_particle(pos, axis, &mut expected))
                .collect::<Vec<_>>();
            assert_eq!(spawns.len(), particle_count as usize, "{facing}");
            assert_eq!(
                spawns
                    .iter()
                    .map(|spawn| (spawn.position, spawn.velocity))
                    .collect::<Vec<_>>(),
                expected_spawns,
                "{facing}"
            );
            assert!(spawns.iter().all(|spawn| {
                spawn.particle.resource_id == PARTICLE_ELECTRIC_SPARK && !spawn.always_show
            }));
            assert_eq!(actual.next_long(), expected.next_long(), "{facing}");
        }
    }

    #[test]
    fn lightning_rod_animate_gates_short_circuit_before_later_level_random_draws() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let properties = BTreeMap::from([("facing".to_string(), "up".to_string())]);

        let clear_world = world_with_surface(pos, pos.y + 1, 199);
        let mut clear_actual = LevelEventSoundRandomState::with_seed(91);
        let mut clear_expected = LevelEventSoundRandomState::with_seed(91);
        let mut spawns = Vec::new();
        append_lightning_rod_animate_effects(
            &clear_world,
            pos,
            &properties,
            &mut clear_actual,
            &mut spawns,
        );
        assert!(spawns.is_empty());
        assert_eq!(clear_actual.next_long(), clear_expected.next_long());

        let threshold_seed = (0..1_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed)
                    .next_int_bound(LIGHTNING_ROD_SPARK_CYCLE)
                    > 0
            })
            .expect("deterministic lightning rod threshold rejection seed");
        let threshold_world = thundering_world_with_surface(pos, pos.y + 1, 0);
        let mut threshold_actual = LevelEventSoundRandomState::with_seed(threshold_seed);
        let mut threshold_expected = LevelEventSoundRandomState::with_seed(threshold_seed);
        spawns.clear();
        append_lightning_rod_animate_effects(
            &threshold_world,
            pos,
            &properties,
            &mut threshold_actual,
            &mut spawns,
        );
        assert!(threshold_expected.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE) > 0);
        assert!(spawns.is_empty());
        assert_eq!(threshold_actual.next_long(), threshold_expected.next_long());

        let covered_world = thundering_world_with_surface(pos, pos.y + 2, 199);
        let mut covered_actual = LevelEventSoundRandomState::with_seed(91);
        let mut covered_expected = LevelEventSoundRandomState::with_seed(91);
        spawns.clear();
        append_lightning_rod_animate_effects(
            &covered_world,
            pos,
            &properties,
            &mut covered_actual,
            &mut spawns,
        );
        let _ = covered_expected.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE);
        assert!(spawns.is_empty());
        assert_eq!(covered_actual.next_long(), covered_expected.next_long());
    }

    #[test]
    fn environment_dispatches_lightning_rod_sparks_from_the_level_random_stream() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut expected_level_random = LevelEventSoundRandomState::default();
        let pos = sample_animate_position(&mut expected_level_random, center, NEAR_SAMPLE_RADIUS);
        assert_eq!(
            (pos.x.div_euclid(16), pos.z.div_euclid(16)),
            (0, 0),
            "fixture must sample its loaded chunk"
        );
        let mut world = thundering_world_with_surface(pos, pos.y + 1, 199);
        set_block(
            &mut world,
            pos,
            block_state_id(
                "minecraft:waxed_oxidized_lightning_rod",
                &[
                    ("facing", "west"),
                    ("powered", "false"),
                    ("waterlogged", "false"),
                ],
            ),
        );
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(42);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        assert!(expected_level_random.next_int_bound(LIGHTNING_ROD_SPARK_CYCLE) <= 199);
        let expected_count = expected_level_random.next_int_bound(2) + 1;
        for _ in 0..expected_count {
            let _ = expected_axis_particle(pos, 0, &mut expected_level_random);
        }
        let ambient_particles = world
            .sampled_client_environment_attributes_at(
                block_position(pos),
                EnvironmentBiomeSampling::Direct,
            )
            .unwrap()
            .ambient_particles
            .unwrap_or_default();
        let mut expected_ambient_spawns = Vec::new();
        append_ambient_particle_spawns(
            &mut expected_level_random,
            pos,
            ambient_particles,
            &mut expected_ambient_spawns,
        );
        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == PARTICLE_ELECTRIC_SPARK)
                .count(),
            expected_count as usize
        );
        assert_eq!(
            runtime.level_random.next_long(),
            expected_level_random.next_long()
        );
    }

    #[test]
    fn wet_sponge_animate_effect_preserves_directions_sturdy_gate_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let world = world_with_ambient_particles();
        for direction_index in 0..6 {
            let seed = animate_direction_seed(None, direction_index);
            let mut actual = LevelEventSoundRandomState::with_seed(seed);
            let mut expected = LevelEventSoundRandomState::with_seed(seed);
            let mut spawns = Vec::new();
            append_wet_sponge_animate_effect(&world, pos, &mut actual, &mut spawns);

            assert_eq!(expected.next_int_bound(6) as usize, direction_index);
            if direction_index == 1 {
                assert!(spawns.is_empty(), "up direction short-circuits");
            } else {
                let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
                let mut expected_position = [8.0, 32.0, 8.0];
                if direction_index == 0 {
                    expected_position[1] -= 0.05;
                    expected_position[0] += expected.next_double();
                    expected_position[2] += expected.next_double();
                } else {
                    expected_position[1] += expected.next_double() * 0.8;
                    if direction[0] != 0 {
                        expected_position[2] += expected.next_double();
                        expected_position[0] += if direction[0] > 0 { 1.1 } else { 0.05 };
                    } else {
                        expected_position[0] += expected.next_double();
                        expected_position[2] += if direction[2] > 0 { 1.1 } else { 0.05 };
                    }
                }
                let [spawn] = spawns.as_slice() else {
                    panic!("direction {direction_index} should emit one drip");
                };
                assert_eq!(spawn.particle.resource_id, PARTICLE_DRIPPING_WATER);
                assert_eq!(spawn.position, expected_position);
                assert_eq!(spawn.velocity, [0.0; 3]);
                assert!(!spawn.always_show);
            }
            assert_eq!(actual.next_long(), expected.next_long());
        }

        let mut blocked_world = world_with_ambient_particles();
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        set_block(
            &mut blocked_world,
            below,
            block_state_id(
                "minecraft:oak_slab",
                &[("type", "top"), ("waterlogged", "false")],
            ),
        );
        let seed = animate_direction_seed(None, 0);
        let mut blocked_actual = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_expected = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_spawns = Vec::new();
        append_wet_sponge_animate_effect(
            &blocked_world,
            pos,
            &mut blocked_actual,
            &mut blocked_spawns,
        );
        assert_eq!(blocked_expected.next_int_bound(6), 0);
        assert!(blocked_spawns.is_empty());
        assert_eq!(blocked_actual.next_long(), blocked_expected.next_long());
    }

    #[test]
    fn crying_obsidian_animate_effect_preserves_chance_offsets_and_sturdy_gate() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let world = world_with_ambient_particles();
        for direction_index in 0..6 {
            let seed = animate_direction_seed(Some(5), direction_index);
            let mut actual = LevelEventSoundRandomState::with_seed(seed);
            let mut expected = LevelEventSoundRandomState::with_seed(seed);
            let mut spawns = Vec::new();
            append_crying_obsidian_animate_effect(&world, pos, &mut actual, &mut spawns);

            assert_eq!(expected.next_int_bound(5), 0);
            assert_eq!(expected.next_int_bound(6) as usize, direction_index);
            if direction_index == 1 {
                assert!(spawns.is_empty(), "up direction short-circuits");
            } else {
                let direction = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
                let expected_position = std::array::from_fn(|axis| {
                    [8.0, 32.0, 8.0][axis]
                        + if direction[axis] == 0 {
                            expected.next_double()
                        } else {
                            0.5 + f64::from(direction[axis]) * 0.6
                        }
                });
                let [spawn] = spawns.as_slice() else {
                    panic!("direction {direction_index} should emit one tear");
                };
                assert_eq!(spawn.particle.resource_id, PARTICLE_DRIPPING_OBSIDIAN_TEAR);
                assert_eq!(spawn.position, expected_position);
                assert_eq!(spawn.velocity, [0.0; 3]);
                assert!(!spawn.always_show);
            }
            assert_eq!(actual.next_long(), expected.next_long());
        }

        let miss_seed = (0..100_000)
            .find(|seed| LevelEventSoundRandomState::with_seed(*seed).next_int_bound(5) != 0)
            .expect("deterministic crying-obsidian chance miss");
        let mut miss_actual = LevelEventSoundRandomState::with_seed(miss_seed);
        let mut miss_expected = LevelEventSoundRandomState::with_seed(miss_seed);
        let mut miss_spawns = Vec::new();
        append_crying_obsidian_animate_effect(&world, pos, &mut miss_actual, &mut miss_spawns);
        assert_ne!(miss_expected.next_int_bound(5), 0);
        assert!(miss_spawns.is_empty());
        assert_eq!(miss_actual.next_long(), miss_expected.next_long());

        let mut blocked_world = world_with_ambient_particles();
        let direction_index = 2;
        let step = REDSTONE_EFFECT_DIRECTION_STEPS[direction_index];
        set_block(
            &mut blocked_world,
            BlockPos {
                x: pos.x + step[0],
                y: pos.y + step[1],
                z: pos.z + step[2],
            },
            block_state_id("minecraft:stone", &[]),
        );
        let seed = animate_direction_seed(Some(5), direction_index);
        let mut blocked_actual = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_expected = LevelEventSoundRandomState::with_seed(seed);
        let mut blocked_spawns = Vec::new();
        append_crying_obsidian_animate_effect(
            &blocked_world,
            pos,
            &mut blocked_actual,
            &mut blocked_spawns,
        );
        assert_eq!(blocked_expected.next_int_bound(5), 0);
        assert_eq!(blocked_expected.next_int_bound(6) as usize, direction_index);
        assert!(blocked_spawns.is_empty());
        assert_eq!(blocked_actual.next_long(), blocked_expected.next_long());
    }

    #[test]
    fn environment_dispatches_sponge_and_obsidian_drips() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        for (name, seed, particle) in [
            (
                "minecraft:wet_sponge",
                animate_direction_seed(None, 5),
                PARTICLE_DRIPPING_WATER,
            ),
            (
                "minecraft:crying_obsidian",
                animate_direction_seed(Some(5), 5),
                PARTICLE_DRIPPING_OBSIDIAN_TEAR,
            ),
        ] {
            let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
            let pos =
                sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
            let mut world = world_with_ambient_particles();
            set_block(&mut world, pos, block_state_id(name, &[]));
            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut animate_random = LevelEventSoundRandomState::with_seed(seed);
            let mut audio = None;
            let mut spawns = Vec::new();

            runtime.do_animate_tick(
                &mut world,
                center,
                NEAR_SAMPLE_RADIUS,
                0,
                None,
                &mut animate_random,
                &mut audio,
                &mut spawns,
            );

            assert_eq!(
                spawns
                    .iter()
                    .filter(|spawn| spawn.particle.resource_id == particle)
                    .count(),
                1,
                "{name}"
            );
        }
    }

    fn animate_direction_seed(chance_bound: Option<i32>, direction_index: usize) -> i64 {
        (0..100_000)
            .find(|seed| {
                let mut random = LevelEventSoundRandomState::with_seed(*seed);
                chance_bound.is_none_or(|bound| random.next_int_bound(bound) == 0)
                    && random.next_int_bound(6) as usize == direction_index
            })
            .expect("deterministic animate direction seed")
    }

    #[test]
    fn redstone_torch_animate_effects_preserve_lit_direction_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: -4 };
        let lit = BTreeMap::from([("lit".to_string(), "true".to_string())]);
        let mut floor_actual = LevelEventSoundRandomState::with_seed(42);
        let mut floor_expected = LevelEventSoundRandomState::with_seed(42);
        let mut floor_spawns = Vec::new();

        append_redstone_torch_animate_effect(
            pos,
            &lit,
            RedstoneTorchAnimateKind::Floor,
            &mut floor_actual,
            &mut floor_spawns,
        );

        let floor_position = [
            8.5 + (floor_expected.next_double() - 0.5) * 0.2,
            32.7 + (floor_expected.next_double() - 0.5) * 0.2,
            -3.5 + (floor_expected.next_double() - 0.5) * 0.2,
        ];
        let [floor_spawn] = floor_spawns.as_slice() else {
            panic!("lit floor redstone torch should emit one particle");
        };
        assert_eq!(floor_spawn.position, floor_position);
        assert_eq!(
            floor_spawn.particle.option,
            bbb_world::ClientParticleOptionValue::Dust {
                color: REDSTONE_PARTICLE_COLOR,
                scale_bits: 1.0_f32.to_bits(),
            }
        );
        assert_eq!(floor_spawn.velocity, [0.0; 3]);
        assert!(!floor_spawn.always_show);
        assert_eq!(floor_actual.next_long(), floor_expected.next_long());

        for (facing, opposite) in [
            ("north", [0, 0, 1]),
            ("south", [0, 0, -1]),
            ("west", [1, 0, 0]),
            ("east", [-1, 0, 0]),
        ] {
            let properties = BTreeMap::from([
                ("facing".to_string(), facing.to_string()),
                ("lit".to_string(), "true".to_string()),
            ]);
            let mut actual = LevelEventSoundRandomState::with_seed(42);
            let mut expected = LevelEventSoundRandomState::with_seed(42);
            let mut spawns = Vec::new();
            append_redstone_torch_animate_effect(
                pos,
                &properties,
                RedstoneTorchAnimateKind::Wall,
                &mut actual,
                &mut spawns,
            );
            let expected_position = [
                8.5 + (expected.next_double() - 0.5) * 0.2 + 0.27 * f64::from(opposite[0]),
                32.7 + (expected.next_double() - 0.5) * 0.2 + 0.22,
                -3.5 + (expected.next_double() - 0.5) * 0.2 + 0.27 * f64::from(opposite[2]),
            ];
            assert_eq!(spawns.len(), 1);
            assert_eq!(spawns[0].position, expected_position, "{facing}");
            assert_eq!(actual.next_long(), expected.next_long(), "{facing}");
        }

        let unlit = BTreeMap::from([("lit".to_string(), "false".to_string())]);
        let mut unlit_actual = LevelEventSoundRandomState::with_seed(42);
        let mut unlit_expected = LevelEventSoundRandomState::with_seed(42);
        let mut unlit_spawns = Vec::new();
        append_redstone_torch_animate_effect(
            pos,
            &unlit,
            RedstoneTorchAnimateKind::Floor,
            &mut unlit_actual,
            &mut unlit_spawns,
        );
        assert!(unlit_spawns.is_empty());
        assert_eq!(unlit_actual.next_long(), unlit_expected.next_long());
    }

    #[test]
    fn repeater_animate_effect_preserves_delay_boolean_branch_and_float_offset() {
        let pos = BlockPos { x: 8, y: 32, z: -4 };
        let branch_seeds = [false, true].map(|expected_branch| {
            (
                expected_branch,
                (0..100_000)
                    .find(|seed| {
                        let mut random = LevelEventSoundRandomState::with_seed(*seed);
                        for _ in 0..3 {
                            let _ = random.next_double();
                        }
                        random.next_boolean() == expected_branch
                    })
                    .expect("deterministic repeater boolean branch seed"),
            )
        });

        for (expected_branch, seed) in branch_seeds {
            for (facing, delay, direction) in [
                ("north", 1, [0, 0, -1]),
                ("east", 2, [1, 0, 0]),
                ("south", 3, [0, 0, 1]),
                ("west", 4, [-1, 0, 0]),
            ] {
                let properties = BTreeMap::from([
                    ("delay".to_string(), delay.to_string()),
                    ("facing".to_string(), facing.to_string()),
                    ("powered".to_string(), "true".to_string()),
                ]);
                let mut actual = LevelEventSoundRandomState::with_seed(seed);
                let mut expected = LevelEventSoundRandomState::with_seed(seed);
                let mut spawns = Vec::new();
                append_repeater_animate_effect(pos, &properties, &mut actual, &mut spawns);

                let x = 8.5 + (expected.next_double() - 0.5) * 0.2;
                let y = 32.4 + (expected.next_double() - 0.5) * 0.2;
                let z = -3.5 + (expected.next_double() - 0.5) * 0.2;
                assert_eq!(expected.next_boolean(), expected_branch);
                let offset = if expected_branch {
                    (delay * 2 - 1) as f32
                } else {
                    -5.0
                } / 16.0;
                let expected_position = [
                    x + f64::from(offset * direction[0] as f32),
                    y,
                    z + f64::from(offset * direction[2] as f32),
                ];
                assert_eq!(spawns.len(), 1);
                assert_eq!(
                    spawns[0].position, expected_position,
                    "{facing} delay {delay} branch {expected_branch}"
                );
                assert_eq!(actual.next_long(), expected.next_long());
            }
        }

        let unpowered = BTreeMap::from([
            ("delay".to_string(), "4".to_string()),
            ("facing".to_string(), "north".to_string()),
            ("powered".to_string(), "false".to_string()),
        ]);
        let mut unpowered_actual = LevelEventSoundRandomState::with_seed(42);
        let mut unpowered_expected = LevelEventSoundRandomState::with_seed(42);
        let mut unpowered_spawns = Vec::new();
        append_repeater_animate_effect(
            pos,
            &unpowered,
            &mut unpowered_actual,
            &mut unpowered_spawns,
        );
        assert!(unpowered_spawns.is_empty());
        assert_eq!(unpowered_actual.next_long(), unpowered_expected.next_long());
    }

    #[test]
    fn environment_dispatches_active_redstone_component_animate_effects() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        for (name, properties) in [
            ("minecraft:redstone_torch", vec![("lit", "true")]),
            (
                "minecraft:redstone_wall_torch",
                vec![("facing", "north"), ("lit", "true")],
            ),
            (
                "minecraft:repeater",
                vec![
                    ("delay", "4"),
                    ("facing", "east"),
                    ("locked", "false"),
                    ("powered", "true"),
                ],
            ),
        ] {
            let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
            let pos =
                sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
            let mut world = world_with_ambient_particles();
            set_block(&mut world, pos, block_state_id(name, &properties));
            let mut runtime = EnvironmentAnimateRuntime::default();
            let mut animate_random = LevelEventSoundRandomState::with_seed(42);
            let mut audio = None;
            let mut spawns = Vec::new();

            runtime.do_animate_tick(
                &mut world,
                center,
                NEAR_SAMPLE_RADIUS,
                0,
                None,
                &mut animate_random,
                &mut audio,
                &mut spawns,
            );

            assert_eq!(
                spawns
                    .iter()
                    .filter(|spawn| {
                        spawn.particle.option
                            == bbb_world::ClientParticleOptionValue::Dust {
                                color: REDSTONE_PARTICLE_COLOR,
                                scale_bits: 1.0_f32.to_bits(),
                            }
                    })
                    .count(),
                1,
                "{name}"
            );
        }
    }

    #[test]
    fn redstone_wire_colors_match_vanilla_power_table() {
        assert_eq!(
            (0..=15).map(redstone_wire_color).collect::<Vec<_>>(),
            vec![
                0xff4c_0000,
                0xff70_0000,
                0xff7a_0000,
                0xff84_0000,
                0xff8e_0000,
                0xff99_0000,
                0xffa3_0000,
                0xffad_0000,
                0xffb7_0000,
                0xffc1_0000,
                0xffcc_0000,
                0xffd6_0000,
                0xffe0_0000,
                0xffea_0600,
                0xfff4_1b00,
                0xffff_3200,
            ]
        );
    }

    #[test]
    fn redstone_wire_animate_effects_preserve_direction_fallthrough_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let properties = [
            ("power", "15"),
            ("north", "up"),
            ("east", "side"),
            ("south", "none"),
            ("west", "up"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<BTreeMap<_, _>>();
        let mut actual = LevelEventSoundRandomState::with_seed(245_131);
        let mut spawns = Vec::new();

        append_redstone_wire_animate_effects(pos, &properties, &mut actual, &mut spawns);

        assert_eq!(spawns.len(), 6);
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            vec![
                [8.5, 32.634_743_452_072_144, 8.0625],
                [8.5, 32.0625, 8.389_978_110_790_253],
                [8.556_022_971_868_515, 32.0625, 8.5],
                [8.5, 32.0625, 8.795_673_221_349_716],
                [8.0625, 32.041_953_563_690_186, 8.5],
                [8.402_953_326_702_118, 32.0625, 8.5],
            ]
        );
        assert!(spawns.iter().all(|spawn| {
            spawn.particle.resource_id == "minecraft:dust"
                && spawn.particle.option
                    == bbb_world::ClientParticleOptionValue::Dust {
                        color: 0xffff_3200,
                        scale_bits: 1.0_f32.to_bits(),
                    }
                && spawn.velocity == [0.0; 3]
                && !spawn.always_show
        }));
        assert_eq!(
            spawns[0].particle.fields,
            vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "type".to_string(),
                    value: bbb_protocol::NbtSummaryValue::String("minecraft:dust".to_string()),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "color".to_string(),
                    value: bbb_protocol::NbtSummaryValue::Int(0xffff_3200_u32 as i32),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "scale".to_string(),
                    value: bbb_protocol::NbtSummaryValue::Float(1.0_f32.to_bits()),
                },
            ]
        );
        let mut expected = LevelEventSoundRandomState::with_seed(245_131);
        for _ in 0..12 {
            let _ = expected.next_float();
        }
        assert_eq!(actual.next_long(), expected.next_long());

        let mut unpowered = properties;
        unpowered.insert("power".to_string(), "0".to_string());
        let mut unpowered_actual = LevelEventSoundRandomState::with_seed(245_131);
        let mut unpowered_expected = LevelEventSoundRandomState::with_seed(245_131);
        let mut unpowered_spawns = Vec::new();
        append_redstone_wire_animate_effects(
            pos,
            &unpowered,
            &mut unpowered_actual,
            &mut unpowered_spawns,
        );
        assert!(unpowered_spawns.is_empty());
        assert_eq!(unpowered_actual.next_long(), unpowered_expected.next_long());
    }

    #[test]
    fn environment_dispatches_powered_redstone_wire_animate_effects() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id(
                "minecraft:redstone_wire",
                &[
                    ("east", "side"),
                    ("north", "up"),
                    ("power", "15"),
                    ("south", "none"),
                    ("west", "up"),
                ],
            ),
        );
        let probe = world
            .probe_block(pos)
            .expect("redstone wire test block should be loaded");
        assert_eq!(probe.block_name.as_deref(), Some("minecraft:redstone_wire"));
        assert_eq!(
            probe.block_properties.get("power").map(String::as_str),
            Some("15")
        );
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_random = LevelEventSoundRandomState::with_seed(245_131);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_random,
            &mut audio,
            &mut spawns,
        );

        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == "minecraft:dust")
                .count(),
            6
        );
    }

    #[test]
    fn redstone_ore_animate_effects_share_face_sampling_and_level_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            BlockPos { y: 31, ..pos },
            block_state_id("minecraft:stone", &[]),
        );
        let lit = BTreeMap::from([("lit".to_string(), "true".to_string())]);
        let mut actual = LevelEventSoundRandomState::with_seed(0);
        let mut expected = LevelEventSoundRandomState::with_seed(0);
        let mut spawns = Vec::new();

        append_redstone_ore_animate_effects(&world, pos, &lit, &mut actual, &mut spawns);

        assert_eq!(spawns.len(), 5, "the solid-render face below is skipped");
        let expected_positions = [
            [
                8.0 + f64::from(expected.next_float()),
                33.0625,
                8.0 + f64::from(expected.next_float()),
            ],
            [
                8.0 + f64::from(expected.next_float()),
                32.0 + f64::from(expected.next_float()),
                7.9375,
            ],
            [
                8.0 + f64::from(expected.next_float()),
                32.0 + f64::from(expected.next_float()),
                9.0625,
            ],
            [
                7.9375,
                32.0 + f64::from(expected.next_float()),
                8.0 + f64::from(expected.next_float()),
            ],
            [
                9.0625,
                32.0 + f64::from(expected.next_float()),
                8.0 + f64::from(expected.next_float()),
            ],
        ];
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_positions
        );
        assert!(spawns.iter().all(|spawn| {
            spawn.particle.option
                == bbb_world::ClientParticleOptionValue::Dust {
                    color: 0x00ff_0000,
                    scale_bits: 1.0_f32.to_bits(),
                }
                && spawn.velocity == [0.0; 3]
                && !spawn.always_show
        }));
        assert_eq!(actual.next_long(), expected.next_long());

        let unlit = BTreeMap::from([("lit".to_string(), "false".to_string())]);
        let mut unlit_actual = LevelEventSoundRandomState::with_seed(0);
        let mut unlit_expected = LevelEventSoundRandomState::with_seed(0);
        let mut unlit_spawns = Vec::new();
        append_redstone_ore_animate_effects(
            &world,
            pos,
            &unlit,
            &mut unlit_actual,
            &mut unlit_spawns,
        );
        assert!(unlit_spawns.is_empty());
        assert_eq!(unlit_actual.next_long(), unlit_expected.next_long());
    }

    #[test]
    fn environment_dispatches_lit_redstone_ore_without_consuming_animate_random() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id("minecraft:deepslate_redstone_ore", &[("lit", "true")]),
        );
        set_block(
            &mut world,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            block_state_id("minecraft:stone", &[]),
        );
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_actual = LevelEventSoundRandomState::with_seed(91);
        let mut animate_expected = LevelEventSoundRandomState::with_seed(91);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_actual,
            &mut audio,
            &mut spawns,
        );

        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn.particle.resource_id == "minecraft:dust")
                .count(),
            5
        );
        assert_eq!(animate_actual.next_long(), animate_expected.next_long());
    }

    #[test]
    fn sculk_sensor_animate_effect_preserves_phase_direction_and_random_order() {
        let pos = BlockPos { x: 8, y: 32, z: 8 };
        let active = BTreeMap::from([("sculk_sensor_phase".to_string(), "active".to_string())]);
        let mut actual = LevelEventSoundRandomState::with_seed(3);
        let mut expected = LevelEventSoundRandomState::with_seed(3);
        let mut spawns = Vec::new();

        append_sculk_sensor_animate_effect(pos, &active, &mut actual, &mut spawns);

        assert_eq!(expected.next_int_bound(6), 2, "seed selects north");
        let expected_position = [9.0 - expected.next_double(), 32.25, 7.9];
        let expected_velocity = [0.0, f64::from(expected.next_float()) * 0.04, 0.0];
        let [spawn] = spawns.as_slice() else {
            panic!("active horizontal sculk sensor should emit one particle");
        };
        assert_eq!(spawn.position, expected_position);
        assert_eq!(spawn.velocity, expected_velocity);
        assert_eq!(
            spawn.particle.option,
            bbb_world::ClientParticleOptionValue::DustColorTransition {
                from_color: 0x0039_d6e0,
                to_color: 0x00ff_0000,
                scale_bits: 1.0_f32.to_bits(),
            }
        );
        assert_eq!(
            spawn.particle.fields,
            vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "type".to_string(),
                    value: bbb_protocol::NbtSummaryValue::String(
                        "minecraft:dust_color_transition".to_string(),
                    ),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "from_color".to_string(),
                    value: bbb_protocol::NbtSummaryValue::Int(0x0039_d6e0),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "to_color".to_string(),
                    value: bbb_protocol::NbtSummaryValue::Int(0x00ff_0000),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "scale".to_string(),
                    value: bbb_protocol::NbtSummaryValue::Float(1.0_f32.to_bits()),
                },
            ]
        );
        assert!(!spawn.always_show);
        assert_eq!(actual.next_long(), expected.next_long());

        let mut vertical_actual = LevelEventSoundRandomState::with_seed(0);
        let mut vertical_expected = LevelEventSoundRandomState::with_seed(0);
        let mut vertical_spawns = Vec::new();
        append_sculk_sensor_animate_effect(
            pos,
            &active,
            &mut vertical_actual,
            &mut vertical_spawns,
        );
        assert_eq!(vertical_expected.next_int_bound(6), 0, "seed selects down");
        assert!(vertical_spawns.is_empty());
        assert_eq!(vertical_actual.next_long(), vertical_expected.next_long());

        let cooldown = BTreeMap::from([("sculk_sensor_phase".to_string(), "cooldown".to_string())]);
        let mut cooldown_actual = LevelEventSoundRandomState::with_seed(3);
        let mut cooldown_expected = LevelEventSoundRandomState::with_seed(3);
        let mut cooldown_spawns = Vec::new();
        append_sculk_sensor_animate_effect(
            pos,
            &cooldown,
            &mut cooldown_actual,
            &mut cooldown_spawns,
        );
        assert!(cooldown_spawns.is_empty());
        assert_eq!(cooldown_actual.next_long(), cooldown_expected.next_long());
    }

    #[test]
    fn environment_dispatches_calibrated_sculk_sensor_animate_effect() {
        let center = BlockPos { x: 8, y: 32, z: 8 };
        let mut sampled_level_random = LevelEventSoundRandomState::with_seed(0);
        let pos = sample_animate_position(&mut sampled_level_random, center, NEAR_SAMPLE_RADIUS);
        let mut world = world_with_ambient_particles();
        set_block(
            &mut world,
            pos,
            block_state_id(
                "minecraft:calibrated_sculk_sensor",
                &[
                    ("facing", "north"),
                    ("power", "0"),
                    ("sculk_sensor_phase", "active"),
                    ("waterlogged", "false"),
                ],
            ),
        );
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut animate_actual = LevelEventSoundRandomState::with_seed(3);
        let mut animate_expected = LevelEventSoundRandomState::with_seed(3);
        let mut audio = None;
        let mut spawns = Vec::new();

        runtime.do_animate_tick(
            &mut world,
            center,
            NEAR_SAMPLE_RADIUS,
            0,
            None,
            &mut animate_actual,
            &mut audio,
            &mut spawns,
        );

        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| { spawn.particle.resource_id == "minecraft:dust_color_transition" })
                .count(),
            1
        );
        assert_eq!(animate_expected.next_int_bound(6), 2);
        let _ = animate_expected.next_double();
        let _ = animate_expected.next_float();
        assert_eq!(animate_actual.next_long(), animate_expected.next_long());
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

    fn world_with_surface(pos: BlockPos, first_available: i32, game_time: i64) -> WorldStore {
        let mut world = world_with_ambient_particles();
        let mut chunk = chunk_with_biome(world.dimension(), 0, 0);
        chunk.heightmaps.push(heightmap_with_first_available(
            world.dimension(),
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            pos.x.rem_euclid(16) as u8,
            pos.z.rem_euclid(16) as u8,
            first_available,
        ));
        world.insert_decoded_chunk(chunk);
        world.apply_world_time(PlayTime {
            game_time,
            clock_updates: Vec::new(),
        });
        world
    }

    fn world_with_desert_sand_ring(pos: BlockPos) -> WorldStore {
        let mut world = world_with_ambient_particles();
        let dimension = world.dimension();
        let mut columns_by_chunk = BTreeMap::<(i32, i32), Vec<(u8, u8, i32)>>::new();
        let targets = [
            BlockPos {
                z: pos.z - 8,
                ..pos
            },
            BlockPos {
                x: pos.x + 8,
                ..pos
            },
            BlockPos {
                z: pos.z + 8,
                ..pos
            },
            BlockPos {
                x: pos.x - 8,
                ..pos
            },
        ];
        for target in targets {
            columns_by_chunk
                .entry((target.x.div_euclid(16), target.z.div_euclid(16)))
                .or_default()
                .push((
                    target.x.rem_euclid(16) as u8,
                    target.z.rem_euclid(16) as u8,
                    target.y + 1,
                ));
        }
        for ((chunk_x, chunk_z), columns) in columns_by_chunk {
            let mut chunk = chunk_with_biome(dimension, 0, 0);
            chunk.pos = ChunkPos {
                x: chunk_x,
                z: chunk_z,
            };
            chunk
                .heightmaps
                .push(heightmap_with_first_available_columns(
                    dimension,
                    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                    &columns,
                ));
            world.insert_decoded_chunk(chunk);
        }
        for target in targets {
            set_block(&mut world, target, block_state_id("minecraft:sand", &[]));
        }
        world
    }

    fn thundering_world_with_surface(
        pos: BlockPos,
        first_available: i32,
        game_time: i64,
    ) -> WorldStore {
        let mut world = world_with_surface(pos, first_available, game_time);
        world.apply_game_event(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
            param: 1.0,
        });
        world.apply_game_event(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
            param: 1.0,
        });
        assert!(world.is_thundering());
        world
    }

    fn raining_world_for_leaf(pos: BlockPos) -> WorldStore {
        let mut world = world_with_ambient_particles();
        let mut chunk = chunk_with_biome(world.dimension(), 0, 0);
        chunk.heightmaps.push(heightmap_with_first_available(
            world.dimension(),
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            pos.x.rem_euclid(16) as u8,
            pos.z.rem_euclid(16) as u8,
            pos.y + 1,
        ));
        world.insert_decoded_chunk(chunk);
        world.apply_game_event(GameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
            param: 1.0,
        });
        assert!(world.is_raining_at(BlockPos {
            y: pos.y + 1,
            ..pos
        }));
        world
    }

    fn heightmap_with_first_available(
        dimension: WorldDimension,
        kind_id: i32,
        local_x: u8,
        local_z: u8,
        first_available: i32,
    ) -> HeightmapData {
        heightmap_with_first_available_columns(
            dimension,
            kind_id,
            &[(local_x, local_z, first_available)],
        )
    }

    fn heightmap_with_first_available_columns(
        dimension: WorldDimension,
        kind_id: i32,
        columns: &[(u8, u8, i32)],
    ) -> HeightmapData {
        let value_range = u64::try_from(dimension.height).unwrap() + 1;
        let bits = (u64::BITS - (value_range - 1).leading_zeros()).max(1) as usize;
        let values_per_long = 64 / bits;
        let mut data = vec![0_i64; (16_usize * 16).div_ceil(values_per_long)];
        for &(local_x, local_z, first_available) in columns {
            let index = usize::from(local_x) + usize::from(local_z) * 16;
            let cell_index = index / values_per_long;
            let bit_index = (index - cell_index * values_per_long) * bits;
            let raw = u64::try_from(first_available - dimension.min_y).unwrap();
            data[cell_index] |= (raw << bit_index) as i64;
        }
        HeightmapData { kind_id, data }
    }

    fn expected_axis_particle(
        pos: BlockPos,
        axis: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ([f64; 3], [f64; 3]) {
        let offsets = [
            random.next_double() * 2.0 + -1.0,
            random.next_double() * 2.0 + -1.0,
            random.next_double() * 2.0 + -1.0,
        ];
        let axis_index = usize::try_from(axis).unwrap();
        let position = std::array::from_fn(|index| {
            f64::from([pos.x, pos.y, pos.z][index])
                + 0.5
                + offsets[index]
                    * if index == axis_index {
                        0.5
                    } else {
                        ELECTRIC_SPARK_AXIS_RADIUS
                    }
        });
        let mut velocity = [0.0; 3];
        velocity[axis_index] = random.next_double() * 2.0 + -1.0;
        (position, velocity)
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
        // Scene setup tolerates the vanilla identical-state no-op; the block
        // must hold the requested state either way.
        world.apply_block_update(BlockUpdate {
            pos: bbb_protocol::packets::BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        });
        assert_eq!(
            world.probe_block(pos).map(|block| block.block_state_id),
            Some(block_state_id)
        );
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
