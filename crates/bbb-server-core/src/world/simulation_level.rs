//! Mutable gameplay context for one dimension.

use bbb_server_types::block_queries::block_state_has_block_entity;
mod analog_output;
mod beehive;
mod berry_harvest;
pub(super) mod block_entity_loot;
mod blocks;
pub(super) mod command_feedback;
pub(crate) mod container_access;
pub(in crate::world) mod enchanted_damage;
mod entity;
#[cfg(test)]
mod entity_tick_tests;
mod equipment_drop_chance;
mod falling_block_data;
mod falling_clip;
pub(super) mod mob_death_loot;
mod occupancy;
pub(super) mod pending_spawns;
mod sculk_step;
mod shrieker_step;
pub(in crate::world) mod structure_block;
mod vehicle_container;
pub(in crate::world) mod worldgen_level;

pub(in crate::world) use command_feedback::{command_feedback_channel, CommandFeedbackChannel};

use super::chunk_lifecycle::full_preparation::{FullChunkPreparation, FullChunkPreparationInputs};
use super::natural_spawn::chunk_generation::OriginalMobInputs;
use crate::random::GameRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_fluid_randomly_ticking,
    vanilla_block_state_identity, vanilla_block_state_light_face_occlusion_index,
    vanilla_block_state_name, vanilla_block_state_randomly_ticking,
    vanilla_face_shape_union_occludes, VanillaBlockSupportDirection,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBlockAnalogOutputOverride,
    VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_NETHER_PORTAL_ID, VANILLA_BLOCK_PISTON_HEAD_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_rules::{vanilla_game_rule, GameRuleValue};
use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary, SoundEvent, Vec3d};
use bbb_registry_codec::item_stack_summary_is_empty;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::block_behavior::affect_neighbors_after_removal as block_affect_neighbors_after_removal;
use crate::block_behavior::bubble_column::schedule_liquid_bubble_column_tick;
use crate::block_behavior::fire::{self, FireFacts};
use crate::block_behavior::lectern as lectern_block;
use crate::block_behavior::level::BlockEntityQuery;
use crate::block_behavior::neighbor_changed as block_neighbor_changed;
use crate::block_behavior::on_place as block_on_place;
use crate::block_behavior::piston::{
    moving_piston_state, piston_head_state, source_positions, PistonMovementPlan,
};
use crate::block_behavior::redstone_torch::RedstoneTorchToggle;
use crate::block_behavior::sculk_sensor as sculk_sensor_block;
use crate::block_behavior::trigger_event_with_level as block_trigger_event;
use crate::block_behavior::update_indirect_neighbour_shapes as block_update_indirect_neighbour_shapes;
use crate::block_behavior::{
    random_tick_block, update_block_shape, BlockMobSpawnRequest, FallingBlockFallRequest,
    GolemSpawnRequest, LightLayer, SET_BLOCK_UPDATE_CLIENTS,
};
use crate::block_entity::{
    block_change_unpacks_container_loot, chiseled_bookshelf, command_block, crafter,
    create_block_entity_on_block_change, new_block_entity, remove_block_entity_on_block_change,
    BlockEntity,
};
use crate::block_events::{BlockEventData, BlockEventQueue};
use crate::chunk::ChunkStore;
use crate::chunk_sender::ChunkKey;
use crate::commands::{CommandFeedback, CommandFeedbackKind};
use crate::entity::block_drops::{add_fresh_item_entity, BlockDropTables};
use crate::entity::{
    fallen_block_replacement, CrossLevelEntities, Entity, EntityData, EntityRegistryTagView,
    EntityStore, NearbyPlayer,
};
use crate::fluid::{
    above, below, liquid_block_family, liquid_block_neighbor_changed, liquid_block_on_place,
    liquid_block_update_shape, SET_BLOCK_UPDATE_ALL,
};
use crate::level_event::{LevelEventSink, VanillaGameEvent};
use crate::level_ticks::{chunk_of_block, LevelTicks};
use crate::loot_runtime::{LootEntity, LootTool};
use crate::neighbor_updater::{
    BlockNeighborUpdate, NeighborUpdates, ShapeNeighborUpdate, UPDATE_LIMIT,
};
use crate::redstone;
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, ScheduledTick, TickPriority};
use bbb_server_types::fluid::{block_state_is_air, FluidFamily};
use bbb_server_types::geometry::{static_block_collision_box_iter, Aabb};

use super::{
    is_in_world_bounds, loot_context::LootLevelFacts, removal_affects_neighbors,
    stored_block_state, LoadedBlocks, MAX_LIGHT_LEVEL, SET_BLOCK_UPDATE_KNOWN_SHAPE,
    SET_BLOCK_UPDATE_MOVE_BY_PISTON, SET_BLOCK_UPDATE_NEIGHBORS, SET_BLOCK_UPDATE_SKIP_ON_PLACE,
    SET_BLOCK_UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE, SET_BLOCK_UPDATE_SUPPRESS_DROPS,
};
use block_entity_loot::{DropBlockEntity, DropTable};
use pending_spawns::PendingEntitySpawns;

// ---------------------------------------------------------------------------
// The world-simulation seam
// ---------------------------------------------------------------------------

/// The game rules `ServerLevel`'s own tick reads, projected out of
/// [`AdminState`].
///
/// Vanilla keeps one `GameRules` on the level data and both the tick loop and
/// the management service read that one object. This server keeps the table in
/// [`AdminState`], because that is the surface that writes a rule; this is the
/// projection of the handful the simulation consumes, refreshed at the single
/// place a rule is written ([`ServerWorld::apply_admin`]) and at restore, so
/// the value still has exactly one writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SimulationGameRules {
    pub(super) random_tick_speed: i32,
    water_source_conversion: bool,
    lava_source_conversion: bool,
    entity_drops: bool,
    mob_drops: bool,
    pub(super) block_drops: bool,
    mob_griefing: bool,
    projectiles_can_break_blocks: bool,
    spread_vines: bool,
    pub(super) fire_spread_radius_around_player: i32,
    pub(super) tnt_explodes: bool,
    pub(super) spawner_blocks_work: bool,
    pub(super) command_blocks_work: bool,
    spawn_mobs: bool,
    spawn_monsters: bool,
    pub(super) raids: bool,
    pub(super) spawn_wardens: bool,
    pub(super) max_entity_cramming: i32,
    pub(super) max_snow_accumulation_height: i32,
}

/// `minecraft:random_tick_speed` (`GameRules.java:74`).
pub(super) const RULE_RANDOM_TICK_SPEED: &str = "minecraft:random_tick_speed";

/// `minecraft:water_source_conversion` (`GameRules.java:92`).
pub(super) const RULE_WATER_SOURCE_CONVERSION: &str = "minecraft:water_source_conversion";

/// `minecraft:lava_source_conversion` (`GameRules.java:47`).
pub(super) const RULE_LAVA_SOURCE_CONVERSION: &str = "minecraft:lava_source_conversion";

/// `minecraft:entity_drops` (`GameRules.java:36`).
pub(super) const RULE_ENTITY_DROPS: &str = "minecraft:entity_drops";

/// `minecraft:mob_drops` (`GameRules.java:59`).
pub(super) const RULE_MOB_DROPS: &str = "minecraft:mob_drops";

/// `minecraft:block_drops` (`gamerules/GameRules.java:29`).
pub(super) const RULE_BLOCK_DROPS: &str = "minecraft:block_drops";

pub(super) const RULE_MOB_GRIEFING: &str = "minecraft:mob_griefing";

/// `minecraft:projectiles_can_break_blocks` (`gamerules/GameRules.java:71`).
pub(super) const RULE_PROJECTILES_CAN_BREAK_BLOCKS: &str = "minecraft:projectiles_can_break_blocks";

/// `minecraft:spread_vines` (`GameRules.java:88`).
pub(super) const RULE_SPREAD_VINES: &str = "minecraft:spread_vines";

pub(super) const RULE_FIRE_SPREAD_RADIUS_AROUND_PLAYER: &str =
    "minecraft:fire_spread_radius_around_player";

pub(super) const RULE_TNT_EXPLODES: &str = "minecraft:tnt_explodes";

pub(super) const RULE_SPAWNER_BLOCKS_WORK: &str = "minecraft:spawner_blocks_work";

pub(super) const RULE_COMMAND_BLOCKS_WORK: &str = "minecraft:command_blocks_work";

/// `minecraft:spawn_wardens` (`gamerules/GameRules.java:83`).
pub(super) const RULE_SPAWN_WARDENS: &str = "minecraft:spawn_wardens";

pub(super) const RULE_SPAWN_MOBS: &str = "minecraft:spawn_mobs";

pub(super) const RULE_SPAWN_MONSTERS: &str = "minecraft:spawn_monsters";

/// `GameRules.RAIDS` (`gamerules/GameRules.java:73`), which `Raids.tick` reads
/// to stop every raid it is holding (`Raids.java:81-83`).
pub(super) const RULE_RAIDS: &str = "minecraft:raids";

/// `minecraft:max_entity_cramming` (`gamerules/GameRules.java:54`).
pub(super) const RULE_MAX_ENTITY_CRAMMING: &str = "minecraft:max_entity_cramming";

/// `minecraft:max_snow_accumulation_height` (`gamerules/GameRules.java:58`).
pub(super) const RULE_MAX_SNOW_ACCUMULATION_HEIGHT: &str = "minecraft:max_snow_accumulation_height";

/// `minecraft:send_command_feedback` (`GameRules.java:77`). Not a
/// [`SimulationGameRules`] member: nothing in the world simulation reads it,
/// only `CommandSourceStack` does.
pub(crate) const RULE_SEND_COMMAND_FEEDBACK: &str = "minecraft:send_command_feedback";

/// `GameRules.SHOW_DEATH_MESSAGES` (`gamerules/GameRules.java:79`), which
/// `ServerPlayer.die` (ServerPlayer.java:871) reads.
pub(super) const RULE_SHOW_DEATH_MESSAGES: &str = "minecraft:show_death_messages";

/// `ClientboundDamageEventPacket.readOptionalEntityId`
/// (ClientboundDamageEventPacket.java:36-39): the wire writes `id + 1` and a
/// zero means no entity, so the decoded struct carries `-1` for absent.
pub(super) const NO_SOURCE_ENTITY_ID: i32 = -1;

/// `EntityEvent.DEATH` (EntityEvent.java:6), which `ServerPlayer.die`
/// broadcasts (ServerPlayer.java:919) to start the death animation.
pub(super) const ENTITY_EVENT_DEATH: i8 = 3;

/// `EntityEvent.POOF` (EntityEvent.java:55), which `LivingEntity.tickDeath`
/// (LivingEntity.java:552) broadcasts on the tick the body is removed.
pub(super) const ENTITY_EVENT_POOF: i8 = 60;

/// `ServerGamePacketListenerImpl.restartClientLoadTimerAfterRespawn` (:2218),
/// which is also what the listener's constructor opens the window with (`:280`).
pub(super) const CLIENT_LOADED_TIMEOUT_TICKS: i32 = 60;

/// `GameRules.KEEP_INVENTORY` (`gamerules/GameRules.java:46`), which
/// `ServerPlayer.restoreFrom` reads to decide whether a respawn carries the old
/// inventory over (ServerPlayer.java:1605-1607).
pub(super) const RULE_KEEP_INVENTORY: &str = "minecraft:keep_inventory";

/// `GameRules.NATURAL_HEALTH_REGENERATION` (`gamerules/GameRules.java:62`),
/// which `FoodData.tick` reads (`FoodData.java:44`). Not a
/// [`SimulationGameRules`] member: the level tick never reads it, only the
/// player tick does.
pub(super) const RULE_NATURAL_HEALTH_REGENERATION: &str = "minecraft:natural_health_regeneration";

/// `minecraft:spectators_generate_chunks` (`GameRules.java:87`). Not a
/// [`SimulationGameRules`] member either: it is read where the player tickets
/// are filed (`ChunkMap.skipPlayer`), not inside the level tick.
pub(super) const RULE_SPECTATORS_GENERATE_CHUNKS: &str = "minecraft:spectators_generate_chunks";

impl SimulationGameRules {
    /// Every rule at the default its own `register*` call declares, read off
    /// the generated 26.1 catalog rather than repeated here.
    pub(super) fn vanilla_defaults() -> Self {
        let mut rules = Self {
            random_tick_speed: 0,
            water_source_conversion: false,
            lava_source_conversion: false,
            entity_drops: false,
            mob_drops: false,
            block_drops: false,
            mob_griefing: false,
            projectiles_can_break_blocks: false,
            spread_vines: false,
            fire_spread_radius_around_player: 0,
            tnt_explodes: false,
            spawner_blocks_work: false,
            command_blocks_work: false,
            spawn_mobs: false,
            spawn_monsters: false,
            raids: false,
            spawn_wardens: false,
            max_entity_cramming: 0,
            max_snow_accumulation_height: 0,
        };
        for key in [
            RULE_RANDOM_TICK_SPEED,
            RULE_WATER_SOURCE_CONVERSION,
            RULE_LAVA_SOURCE_CONVERSION,
            RULE_ENTITY_DROPS,
            RULE_MOB_DROPS,
            RULE_BLOCK_DROPS,
            RULE_MOB_GRIEFING,
            RULE_PROJECTILES_CAN_BREAK_BLOCKS,
            RULE_SPREAD_VINES,
            RULE_FIRE_SPREAD_RADIUS_AROUND_PLAYER,
            RULE_TNT_EXPLODES,
            RULE_SPAWNER_BLOCKS_WORK,
            RULE_COMMAND_BLOCKS_WORK,
            RULE_SPAWN_MOBS,
            RULE_SPAWN_MONSTERS,
            RULE_RAIDS,
            RULE_SPAWN_WARDENS,
            RULE_MAX_ENTITY_CRAMMING,
            RULE_MAX_SNOW_ACCUMULATION_HEIGHT,
        ] {
            if let Some(entry) = vanilla_game_rule(key) {
                rules.set(key, entry.default_value);
            }
        }
        rules
    }

    /// Takes one rule's value, ignoring every rule the simulation does not
    /// read.
    pub(super) fn set(&mut self, resource_id: &str, value: GameRuleValue) {
        match (resource_id, value) {
            (RULE_RANDOM_TICK_SPEED, GameRuleValue::Integer(value)) => {
                self.random_tick_speed = value;
            }
            (RULE_WATER_SOURCE_CONVERSION, GameRuleValue::Boolean(value)) => {
                self.water_source_conversion = value;
            }
            (RULE_LAVA_SOURCE_CONVERSION, GameRuleValue::Boolean(value)) => {
                self.lava_source_conversion = value;
            }
            (RULE_ENTITY_DROPS, GameRuleValue::Boolean(value)) => self.entity_drops = value,
            (RULE_MOB_DROPS, GameRuleValue::Boolean(value)) => self.mob_drops = value,
            (RULE_BLOCK_DROPS, GameRuleValue::Boolean(value)) => self.block_drops = value,
            (RULE_MOB_GRIEFING, GameRuleValue::Boolean(value)) => self.mob_griefing = value,
            (RULE_PROJECTILES_CAN_BREAK_BLOCKS, GameRuleValue::Boolean(value)) => {
                self.projectiles_can_break_blocks = value;
            }
            (RULE_SPREAD_VINES, GameRuleValue::Boolean(value)) => self.spread_vines = value,
            (RULE_FIRE_SPREAD_RADIUS_AROUND_PLAYER, GameRuleValue::Integer(value)) => {
                self.fire_spread_radius_around_player = value;
            }
            (RULE_TNT_EXPLODES, GameRuleValue::Boolean(value)) => self.tnt_explodes = value,
            (RULE_SPAWNER_BLOCKS_WORK, GameRuleValue::Boolean(value)) => {
                self.spawner_blocks_work = value;
            }
            (RULE_COMMAND_BLOCKS_WORK, GameRuleValue::Boolean(value)) => {
                self.command_blocks_work = value;
            }
            (RULE_SPAWN_MOBS, GameRuleValue::Boolean(value)) => self.spawn_mobs = value,
            (RULE_SPAWN_MONSTERS, GameRuleValue::Boolean(value)) => self.spawn_monsters = value,
            (RULE_RAIDS, GameRuleValue::Boolean(value)) => self.raids = value,
            (RULE_SPAWN_WARDENS, GameRuleValue::Boolean(value)) => {
                self.spawn_wardens = value;
            }
            (RULE_MAX_ENTITY_CRAMMING, GameRuleValue::Integer(value)) => {
                self.max_entity_cramming = value;
            }
            (RULE_MAX_SNOW_ACCUMULATION_HEIGHT, GameRuleValue::Integer(value)) => {
                self.max_snow_accumulation_height = value;
            }
            _ => {}
        }
    }

    /// `ServerLevel.isSpawningMonsters` (`ServerLevel.java:1768-1772`).
    pub(super) const fn spawning_monsters(&self, difficulty: Difficulty) -> bool {
        !matches!(difficulty, Difficulty::Peaceful) && self.spawn_mobs && self.spawn_monsters
    }

    /// `GameRules.SPAWN_MOBS` itself, which `tickThunder`'s trap arm reads
    /// without the monster half beside it (`ServerLevel.java:538`).
    pub(super) const fn spawn_mobs(&self) -> bool {
        self.spawn_mobs
    }
}

/// `ChunkLevel.FULL_CHUNK_LEVEL` (`ChunkLevel.java:10`), which is also
/// `SimulationChunkTracker.MAX_LEVEL` (`SimulationChunkTracker.java:9`): the
/// first level the simulation tracker drops outright (`:36-42`).
pub(super) const CHUNK_FULL_LEVEL: i32 = 33;

/// `ChunkLevel.BLOCK_TICKING_LEVEL` (`ChunkLevel.java:11`), the bound
/// `ChunkLevel.isBlockTicking` tests (`:61-63`).
pub(super) const CHUNK_BLOCK_TICKING_LEVEL: i32 = 32;

/// `ChunkLevel.ENTITY_TICKING_LEVEL` (`ChunkLevel.java:12`), the bound
/// `ChunkLevel.isEntityTicking` tests (`:57-59`).
pub(super) const CHUNK_ENTITY_TICKING_LEVEL: i32 = 31;

/// One live ticket out of `chunk_tickets.dat`, as `TicketStorage` holds it.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SavedChunkTicket {
    pub(super) chunk: ChunkKey,
    pub(super) level: i32,
    pub(super) ticks_left: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SimulationChunkLevels {
    sources: Vec<(ChunkKey, i32)>,
    levels: rustc_hash::FxHashMap<ChunkKey, i32>,
    ordered: Vec<(ChunkKey, i32)>,
}

impl SimulationChunkLevels {
    /// `ChunkTracker.runAllUpdates` over the whole live ticket set.
    ///
    pub(super) fn rebuild(&mut self, sources: impl IntoIterator<Item = (ChunkKey, i32)>) {
        let mut count = 0;
        let mut changed = false;
        for source in sources {
            if self.sources.get(count) != Some(&source) {
                self.sources.truncate(count);
                self.sources.push(source);
                changed = true;
            }
            count += 1;
        }
        if self.sources.len() != count {
            self.sources.truncate(count);
            changed = true;
        }
        if !changed {
            return;
        }
        self.levels.clear();
        for &((source_x, source_z), level) in &self.sources {
            // The furthest a source reaches is where its level is still
            // tracked, `level + distance <= CHUNK_FULL_LEVEL - 1`.
            let reach = CHUNK_FULL_LEVEL - 1 - level;
            if reach < 0 {
                continue;
            }
            // The bounds saturate so the walk stays inside `i32` for any
            // source at all. No source can actually reach the edge: a ticket
            // read out of a save is refused outside
            // `world_storage::JAVA_26_1_TICKET_CHUNK_LIMIT`, and a player
            // position is clamped to `HORIZONTAL_LIMIT` blocks before it is
            // ever shifted into a column. Saturating here is what makes the
            // absence of an overflow a property of this function rather than
            // an assumption about its callers.
            for column_z in source_z.saturating_sub(reach)..=source_z.saturating_add(reach) {
                for column_x in source_x.saturating_sub(reach)..=source_x.saturating_add(reach) {
                    // `ChunkTracker.checkNeighborsAfterUpdate` walks the eight
                    // surrounding columns, so the graph distance is the
                    // Chebyshev distance.
                    let distance = (column_x - source_x).abs().max((column_z - source_z).abs());
                    let slot = self
                        .levels
                        .entry((column_x, column_z))
                        .or_insert(CHUNK_FULL_LEVEL);
                    *slot = (*slot).min(level + distance);
                }
            }
        }
        self.ordered.clear();
        self.ordered
            .extend(self.levels.iter().map(|(&chunk, &level)| (chunk, level)));
        self.ordered.sort_unstable_by_key(|&(chunk, _)| chunk);
    }

    pub(super) fn level(&self, chunk: ChunkKey) -> i32 {
        self.levels.get(&chunk).copied().unwrap_or(CHUNK_FULL_LEVEL)
    }

    pub(super) fn is_entity_ticking(&self, chunk: ChunkKey) -> bool {
        self.level(chunk) <= CHUNK_ENTITY_TICKING_LEVEL
    }

    pub(super) fn is_block_ticking(&self, chunk: ChunkKey) -> bool {
        self.level(chunk) <= CHUNK_BLOCK_TICKING_LEVEL
    }

    /// Every tracked column at or below `bound`, in chunk-coordinate order.
    pub(super) fn chunks_at_most(&self, bound: i32) -> impl Iterator<Item = ChunkKey> + '_ {
        self.ordered
            .iter()
            .filter(move |&&(_, level)| level <= bound)
            .map(|&(chunk, _)| chunk)
    }
}

#[derive(Clone, Copy)]
pub(super) struct SculkWarningRuntime {
    pub(super) difficulty: Difficulty,
}

/// Experimental feature flags queried by gameplay simulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SimulationFeatureFlags {
    pub(crate) redstone_experiments: bool,
    pub(crate) minecart_improvements: bool,
}

impl SimulationFeatureFlags {
    pub(crate) fn from_feature_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> Self {
        let mut flags = Self::default();
        for id in ids {
            match id {
                "minecraft:redstone_experiments" => flags.redstone_experiments = true,
                "minecraft:minecart_improvements" => flags.minecart_improvements = true,
                _ => {}
            }
        }
        flags
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.redstone_experiments |= other.redstone_experiments;
        self.minecart_improvements |= other.minecart_improvements;
    }
}

pub struct LevelCtx<'a, 'w> {
    pub(super) dimension: &'a bbb_server_types::save::Java26LevelKey,
    pub(super) structure_services: Option<&'a structure_block::StructureBlockServices<'w>>,
    pub(super) world_border: bbb_server_types::interaction::WorldBorder,
    pub(super) block_interaction_policy: &'a super::block_interaction::BlockInteractionPolicy,
    pub(super) sculk_warning: Option<SculkWarningRuntime>,
    pub(super) chunks: &'a mut ChunkStore,
    pub(super) full_preparation: FullChunkPreparation<'a>,
    pub(super) sky_darkening: i32,
    pub(super) ambient_light: f32,
    pub(super) logical_height: i32,
    pub(super) weather: &'a super::weather::LevelWeather,
    pub(super) nether_portal_dimension: bool,
    pub(super) game_time: i64,
    pub(super) sub_tick_count: &'a mut i64,
    pub(super) random: &'a GameRandom,
    pub(super) block_ticks: &'a mut LevelTicks<BlockTickType>,
    pub(super) fluid_ticks: &'a mut LevelTicks<FluidTickType>,
    pub(super) block_events: &'a mut BlockEventQueue,
    pub(super) changed_block_broadcasts: &'a mut BTreeSet<(i32, i32, i32)>,
    pub(super) game_rules: SimulationGameRules,
    pub(super) feature_flags: SimulationFeatureFlags,
    pub(super) cross_level_entities: &'a CrossLevelEntities,
    pub(super) simulation_levels: &'a SimulationChunkLevels,
    pub(super) drop_tables: BlockDropTables<'a>,
    pub(super) loot_level_facts: std::rc::Rc<LootLevelFacts<'w>>,
    pub(super) event_sink: &'a mut LevelEventSink,
    pub(super) item_spawns: &'a mut PendingEntitySpawns,
    pub(super) players: &'a [NearbyPlayer],
    pub(super) recent_torch_toggles: &'a mut Vec<RedstoneTorchToggle>,

    pub(super) portal_context: Option<(bool, i32, bool)>,
    pub(super) portal_request: Option<crate::entity::portal::PortalTransitionRequest>,

    pub(super) detached_entities: Option<&'a crate::entity::store::DetachedEntityBodies<'a>>,
    pub(super) structure_entities: Option<structure_block::StructureEntityContext<'a>>,
    pub(super) neighbours: &'a mut NeighborUpdates,
}

fn block_update_requires_navigation(old_state: i32, new_state: i32) -> bool {
    match (
        static_block_collision_box_iter(old_state),
        static_block_collision_box_iter(new_state),
    ) {
        (Some(old_shape), Some(new_shape)) => !old_shape.eq(new_shape),
        _ => true,
    }
}

impl<'w> LevelCtx<'_, 'w> {
    pub(in crate::world) fn reset_entity_tick_scratch(&mut self) {
        self.portal_context = None;
        self.portal_request = None;
    }

    pub(in crate::world) fn has_pending_spawns(&self) -> bool {
        self.item_spawns.has_pending()
    }

    pub(crate) const fn redstone_experiments_enabled(&self) -> bool {
        self.feature_flags.redstone_experiments
    }

    pub(crate) const fn minecart_improvements_enabled(&self) -> bool {
        self.feature_flags.minecart_improvements
    }

    pub(crate) fn is_in_world_bounds(&self, pos: BlockPos) -> bool {
        is_in_world_bounds(self.chunks, pos)
    }

    pub(crate) fn has_chunk_at(&self, pos: BlockPos) -> bool {
        self.is_in_world_bounds(pos) && self.has_ticking_column_at(pos)
    }

    pub(crate) fn has_ticking_column_at(&self, pos: BlockPos) -> bool {
        self.chunks.is_ticking_full(pos.x >> 4, pos.z >> 4)
    }

    pub(crate) fn block_state(&self, pos: BlockPos) -> i32 {
        stored_block_state(self.chunks, pos)
    }

    /// The store write shared by every writer. `None` outside the world or
    /// in a noise column that is not FULL yet.
    pub(super) fn write_block(&mut self, pos: BlockPos, block_state_id: i32) -> Option<i32> {
        if !self.is_in_world_bounds(pos)
            || !self.full_preparation.live_write_loaded(
                self.chunks,
                (pos.x >> 4, pos.z >> 4),
                self.game_time,
                self.item_spawns,
            )
        {
            return None;
        }
        self.chunks
            .set_live_block_state(pos.x, pos.y, pos.z, block_state_id)
            .ok()
            .flatten()
    }

    pub(super) fn next_sub_tick_count(&mut self) -> i64 {
        let value = *self.sub_tick_count;
        *self.sub_tick_count += 1;
        value
    }

    pub(crate) fn schedule_block_tick(
        &mut self,
        pos: BlockPos,
        block: BlockTickType,
        delay: i32,
        priority: TickPriority,
    ) {
        if !self.ensure_tick_containers(pos) {
            return;
        }
        let tick = ScheduledTick::with_priority(
            block,
            pos,
            self.game_time + i64::from(delay),
            priority,
            self.next_sub_tick_count(),
        );
        self.chunks.schedule_tick(tick);
    }

    pub(crate) fn schedule_fluid_tick(
        &mut self,
        pos: BlockPos,
        fluid: FluidTickType,
        delay: i32,
        priority: TickPriority,
    ) {
        if !self.ensure_tick_containers(pos) {
            return;
        }
        let tick = ScheduledTick::with_priority(
            fluid,
            pos,
            self.game_time + i64::from(delay),
            priority,
            self.next_sub_tick_count(),
        );
        self.chunks.schedule_tick(tick);
    }

    fn ensure_tick_containers(&mut self, pos: BlockPos) -> bool {
        if !self.is_in_world_bounds(pos) {
            return false;
        }
        self.chunks
            .register_ticks(chunk_of_block(pos), self.game_time);
        true
    }

    pub(super) fn send_block_updated(&mut self, pos: BlockPos, old_state: i32, new_state: i32) {
        self.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        self.chunks.invalidate_path_type(pos);
        if block_update_requires_navigation(old_state, new_state) {
            self.chunks.queue_navigation_update(pos);
        }
    }
}

impl LevelCtx<'_, '_> {
    pub(crate) fn value_effect_level(&self) -> super::ValueEffectLevel<'_> {
        super::ValueEffectLevel::for_simulation(
            self.weather.loot_weather(),
            self.chunks,
            self.loot_level_facts.clocks,
            self.loot_level_facts.environment_attributes,
        )
    }

    pub(super) fn update_neighbor_shapes(
        &mut self,
        pos: BlockPos,
        state: i32,
        update_flags: i32,
        update_limit: i32,
    ) {
        self.dispatch_neighbors(|neighbours| {
            neighbours.update_neighbor_shapes(pos, state, update_flags, update_limit)
        });
    }

    pub(super) fn drop_resources_at(&mut self, pos: BlockPos, block_state_id: i32) {
        self.drop_resources_with_tool_at(
            pos,
            block_state_id,
            &LootTool::empty(),
            LootEntity::Absent,
        );
    }

    pub(super) fn drop_resources_with_tool_at(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        tool: &LootTool,
        breaker: LootEntity<'_>,
    ) {
        self.drop_block_loot_at(
            pos,
            block_state_id,
            DropTable::Block,
            DropBlockEntity::Live,
            tool,
            breaker,
        );
    }

    pub(crate) fn set_block(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        flags: i32,
        update_limit: i32,
    ) -> bool {
        if !self.is_in_world_bounds(pos) {
            return false;
        }
        if self.chunks.is_debug() {
            return false;
        }
        let Some(old_state) = self.write_block(pos, block_state_id) else {
            return false;
        };
        if old_state == block_state_id {
            return false;
        }
        let moved_by_piston = flags & SET_BLOCK_UPDATE_MOVE_BY_PISTON != 0;
        if block_change_unpacks_container_loot(
            self.chunks.block_entities(),
            pos,
            old_state,
            block_state_id,
            flags,
            self.tags(),
        ) {
            if let Err(error) = self.unpack_container_loot(pos, None) {
                // Refused removal must keep the old block paired with its retained entity.
                // Loot's cleared table and consumed random prefix remain committed.
                self.write_block(pos, old_state);
                self.record_item_failure(pos, error);
                return false;
            }
        }
        let retiring_beacon = self
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::beacon)
            .is_some()
            && vanilla_block_state_block_type_id(old_state)
                != vanilla_block_state_block_type_id(block_state_id);
        let (block_entities, tags) = self.chunks.block_entities_mut_and_tags();
        let removal_drops = remove_block_entity_on_block_change(
            block_entities,
            pos,
            old_state,
            block_state_id,
            flags,
            tags,
        );
        self.queue_block_entity_removal_drops(pos, removal_drops);
        if retiring_beacon && self.chunks.block_entities().get(pos).is_none() {
            self.event_sink
                .play_block_sound(pos, "minecraft:block.beacon.deactivate");
        }
        if removal_affects_neighbors(
            old_state,
            block_state_id,
            flags,
            self.chunks.content_registry(),
        ) {
            block_affect_neighbors_after_removal(self, pos, old_state, moved_by_piston);
        }

        if vanilla_block_state_block_type_id(self.block_state(pos))
            != vanilla_block_state_block_type_id(block_state_id)
        {
            return false;
        }

        if flags & SET_BLOCK_UPDATE_SKIP_ON_PLACE == 0 {
            self.on_place(pos, block_state_id, old_state, moved_by_piston);
        }

        create_block_entity_on_block_change(self.chunks.block_entities_mut(), pos, block_state_id);

        if self.block_state(pos) != block_state_id {
            return true;
        }

        if flags & SET_BLOCK_UPDATE_CLIENTS != 0 {
            self.send_block_updated(pos, old_state, block_state_id);
        }

        if flags & SET_BLOCK_UPDATE_NEIGHBORS != 0 {
            if let Some(source_block) = vanilla_block_state_block_type_id(old_state) {
                let orientation =
                    crate::block_behavior::redstone_wire::experimental::initial_orientation(
                        self, None, None,
                    );
                self.dispatch_neighbors(|neighbours| {
                    neighbours.update_neighbors_at_with_orientation(
                        pos,
                        source_block,
                        None,
                        orientation,
                    )
                });
            }
            // `:247-249`. The `!isClientSide` half is always true here.
            if redstone::has_analog_output_signal(block_state_id) {
                if let Some(new_block) = vanilla_block_state_block_type_id(block_state_id) {
                    self.update_neighbour_for_output_signal(pos, new_block);
                }
            }
        }

        if flags & SET_BLOCK_UPDATE_KNOWN_SHAPE == 0 && update_limit > 0 {
            let neighbour_flags = flags & -34;
            // `:254`, the state that left, before the fan-out.
            block_update_indirect_neighbour_shapes(
                self,
                old_state,
                pos,
                neighbour_flags,
                update_limit - 1,
            );
            // `:255`.
            self.dispatch_neighbors(|neighbours| {
                neighbours.update_neighbor_shapes(
                    pos,
                    block_state_id,
                    neighbour_flags,
                    update_limit - 1,
                )
            });
            // `:256`, the state that arrived, after it. Same method, other
            // state, other side of the fan-out -- the order is the whole
            // difference between the two calls.
            block_update_indirect_neighbour_shapes(
                self,
                block_state_id,
                pos,
                neighbour_flags,
                update_limit - 1,
            );
        }
        self.chunks
            .update_poi_on_block_change([pos.x, pos.y, pos.z], old_state, block_state_id);
        true
    }

    pub(super) fn on_place(
        &mut self,
        pos: BlockPos,
        block_state_id: i32,
        old_state: i32,
        moved_by_piston: bool,
    ) {
        if liquid_block_family(block_state_id).is_some() {
            liquid_block_on_place(self, pos, block_state_id);
            let below_state = LevelCtx::block_state(self, below(pos));
            schedule_liquid_bubble_column_tick(self, pos, block_state_id, below_state);
            return;
        }
        block_on_place(self, pos, block_state_id, old_state, moved_by_piston);
    }

    pub(super) fn update_or_destroy(
        &mut self,
        old_state: i32,
        new_state: i32,
        pos: BlockPos,
        flags: i32,
        update_limit: i32,
    ) {
        if new_state == old_state {
            return;
        }
        if block_state_is_air(new_state) {
            // The cascade's remaining depth goes through, which is what keeps
            // a destroy made *by* a shape update from handing the next one a
            // fresh budget.
            LevelCtx::destroy_block_with_limit(
                self,
                pos,
                flags & SET_BLOCK_UPDATE_SUPPRESS_DROPS == 0,
                update_limit,
            );
        } else {
            self.set_block(pos, new_state, flags & -33, update_limit);
        }
    }
}

impl LevelCtx<'_, '_> {
    pub(crate) fn fluid_block_state(&self, pos: BlockPos) -> i32 {
        self.block_state(pos)
    }

    pub(crate) fn fluid_can_convert_to_source(&self, family: FluidFamily) -> bool {
        match family {
            FluidFamily::Water => self.game_rules.water_source_conversion,
            FluidFamily::Lava => self.game_rules.lava_source_conversion,
        }
    }

    pub(crate) fn fluid_fast_lava(&self) -> bool {
        self.structure_services
            .as_ref()
            .and_then(|services| services.fast_lava)
            .unwrap_or(false)
    }

    pub(crate) fn fluid_drop_resources(&mut self, pos: BlockPos, block_state_id: i32) {
        self.drop_resources_at(pos, block_state_id);
    }

    pub(crate) fn fluid_level_event(&mut self, event_id: i32, pos: BlockPos, data: i32) {
        self.event_sink.level_event(event_id, pos, data);
    }

    pub(crate) fn fluid_state(&self, pos: BlockPos) -> bbb_server_types::fluid::FluidState {
        bbb_server_types::fluid::block_state_fluid_state(self.block_state(pos))
    }
}

fn vanilla_light_block_into(
    from_state: i32,
    to_state: i32,
    direction: Direction,
    simple_opacity: i32,
) -> i32 {
    let (from_face, to_face) = match direction {
        Direction::Down => (
            VanillaBlockSupportDirection::Down,
            VanillaBlockSupportDirection::Up,
        ),
        Direction::Up => (
            VanillaBlockSupportDirection::Up,
            VanillaBlockSupportDirection::Down,
        ),
        Direction::North => (
            VanillaBlockSupportDirection::North,
            VanillaBlockSupportDirection::South,
        ),
        Direction::South => (
            VanillaBlockSupportDirection::South,
            VanillaBlockSupportDirection::North,
        ),
        Direction::West => (
            VanillaBlockSupportDirection::West,
            VanillaBlockSupportDirection::East,
        ),
        Direction::East => (
            VanillaBlockSupportDirection::East,
            VanillaBlockSupportDirection::West,
        ),
    };
    let occludes = vanilla_block_state_light_face_occlusion_index(from_state, from_face)
        .zip(vanilla_block_state_light_face_occlusion_index(
            to_state, to_face,
        ))
        .and_then(|(from, to)| vanilla_face_shape_union_occludes(from, to));
    if occludes.unwrap_or(true) {
        MAX_LIGHT_LEVEL + 1
    } else {
        simple_opacity
    }
}

pub(super) fn update_from_neighbour_shapes(
    level: &mut LevelCtx<'_, '_>,
    mut state: i32,
    pos: BlockPos,
) -> i32 {
    for direction in [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Down,
        Direction::Up,
    ] {
        let neighbour_pos = bbb_server_types::directions::relative(pos, direction);
        let neighbour = level.block_state(neighbour_pos);
        state = update_block_shape(
            &mut crate::block_behavior::shape_context::BlockShapeCtx::Live(level),
            state,
            pos,
            direction,
            neighbour_pos,
            neighbour,
        );
    }
    state
}

fn simulation_brightness(chunks: &ChunkStore, layer: LightLayer, pos: BlockPos) -> i32 {
    i32::from(match layer {
        LightLayer::Block => chunks.block_light_level(pos.x, pos.y, pos.z),
        LightLayer::Sky => chunks.sky_light_level(pos.x, pos.y, pos.z),
    })
}

pub(super) fn placement_raw_brightness(
    chunks: &ChunkStore,
    pos: BlockPos,
    sky_darkening: i32,
) -> i32 {
    simulation_brightness(chunks, LightLayer::Block, pos)
        .max(simulation_brightness(chunks, LightLayer::Sky, pos) - sky_darkening)
}

pub(super) fn max_local_raw_brightness(
    chunks: &ChunkStore,
    pos: BlockPos,
    sky_darkening: i32,
) -> i32 {
    if pos.x >= -30_000_000 && pos.z >= -30_000_000 && pos.x < 30_000_000 && pos.z < 30_000_000 {
        placement_raw_brightness(chunks, pos, sky_darkening)
    } else {
        15
    }
}

pub(super) fn hopper_facing(block_state_id: i32) -> Direction {
    match vanilla_block_state_identity(block_state_id)
        .and_then(|identity| identity.properties.get("facing").map(String::as_str))
    {
        Some("north") => Direction::North,
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        _ => Direction::Down,
    }
}

pub(super) fn run_live_random_tick_sample(
    level: &mut LevelCtx<'_, '_>,
    environment: &FireFacts<'_>,
    pos: BlockPos,
    state: i32,
) {
    let block_type = vanilla_block_state_block_type_id(state);
    let is_lava = block_type == Some(VANILLA_BLOCK_LAVA_ID);
    if vanilla_block_state_randomly_ticking(state).unwrap_or(false) {
        if is_lava {
            let _ = fire::lava_random_tick(level, environment, pos);
        } else if block_type == Some(VANILLA_BLOCK_NETHER_PORTAL_ID) {
            // `NetherPortalBlock.randomTick` reads the spawn environment, which
            // the `block_behavior_owners!` `random_tick:` column cannot carry.
            let _ = fire::portal_random_tick(level, environment, pos);
        } else {
            random_tick_block(level, pos, state);
        }
    }

    if vanilla_block_state_fluid_randomly_ticking(state).unwrap_or(false) && is_lava {
        let _ = fire::lava_random_tick(level, environment, pos);
    }
}

#[cfg(test)]
impl super::ServerWorld {
    pub(crate) fn entity_level_light_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        pos: BlockPos,
    ) -> (Option<i32>, Option<f32>) {
        self.with_level(dimension, |level| {
            (
                LevelCtx::max_local_raw_brightness(level, pos),
                LevelCtx::dimension_ambient_light(level),
            )
        })
    }

    pub(crate) fn ground_path_type_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<crate::entity::GroundPathType> {
        self.with_level(dimension, |level| {
            LevelCtx::ground_path_type_from_state(level, pos)
        })
    }

    pub(crate) fn cache_ground_path_type_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        pos: BlockPos,
        path_type: crate::entity::GroundPathType,
    ) {
        self.level_mut(dimension)
            .chunks
            .cache_path_type(pos, entity::ground_path_type_to_cache(path_type));
    }

    pub(crate) fn cached_ground_path_type_for_test(
        &self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<crate::entity::GroundPathType> {
        self.level(dimension)
            .chunks
            .cached_path_type(pos)
            .and_then(entity::ground_path_type_from_cache)
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    fn default_block_state(resource_id: &str) -> i32 {
        let block_type = vanilla_block_type_for_resource_id(resource_id).unwrap();
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id).unwrap()
    }

    #[test]
    fn ground_navigation_block_update_shape_comparison_is_fail_closed() {
        let stone = default_block_state("minecraft:stone");
        let dirt = default_block_state("minecraft:dirt");
        let air = default_block_state("minecraft:air");
        let moving_piston = default_block_state("minecraft:moving_piston");

        assert!(!block_update_requires_navigation(stone, dirt));
        assert!(block_update_requires_navigation(stone, air));
        assert!(static_block_collision_box_iter(moving_piston).is_none());
        assert!(block_update_requires_navigation(stone, moving_piston));
    }
}

pub(super) fn execute_neighbor_update(level: &mut LevelCtx<'_, '_>, update: &BlockNeighborUpdate) {
    let state = update
        .state
        .unwrap_or_else(|| level.block_state(update.pos));
    if vanilla_block_state_name(state) == Some("minecraft:structure_block") {
        structure_block::neighbor_changed(level, update.pos);
    } else if liquid_block_family(state).is_some() {
        liquid_block_neighbor_changed(level, update.pos, state);
        let below_state = LevelCtx::block_state(level, below(update.pos));
        schedule_liquid_bubble_column_tick(level, update.pos, state, below_state);
    } else {
        block_neighbor_changed(
            level,
            update.pos,
            state,
            update.source_block,
            update.orientation,
            update.moved_by_piston,
        );
    }
}

pub(super) fn execute_shape_update(level: &mut LevelCtx<'_, '_>, update: &ShapeNeighborUpdate) {
    let current = level.block_state(update.pos);
    // `:46`: `UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE` swallows the whole update on
    // redstone wire.
    if update.update_flags & SET_BLOCK_UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE != 0
        && vanilla_block_state_block_type_id(current) == redstone_wire_block_type_id()
    {
        return;
    }
    let new_state = if liquid_block_family(current).is_some() {
        liquid_block_update_shape(level, update.pos, current, update.neighbor_state);
        if update.direction == Direction::Down {
            schedule_liquid_bubble_column_tick(level, update.pos, current, update.neighbor_state);
        }
        current
    } else {
        update_block_shape(
            &mut crate::block_behavior::shape_context::BlockShapeCtx::Live(level),
            current,
            update.pos,
            update.direction,
            update.neighbor_pos,
            update.neighbor_state,
        )
    };
    level.update_or_destroy(
        current,
        new_state,
        update.pos,
        update.update_flags,
        update.update_limit,
    );
}

/// `Blocks.REDSTONE_WIRE` as a `BuiltInRegistries.BLOCK` id.
pub(super) fn redstone_wire_block_type_id() -> Option<i32> {
    static ID: OnceLock<Option<i32>> = OnceLock::new();
    *ID.get_or_init(|| {
        vanilla_block_type_for_resource_id("minecraft:redstone_wire").map(|entry| entry.id)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum BlockEventBroadcast {
    Sound(SoundEvent),
    Event(BlockEventData),
}

impl LevelCtx<'_, '_> {
    pub(super) fn run_block_events(&mut self, broadcasts: &mut Vec<BlockEventBroadcast>) {
        // `ServerLevel.java:342,390,393`: `handlingTick` stays true through `runBlockEvents`.
        let prev_handling = self.block_events.set_handling_tick(true);
        let mut deferred = Vec::new();
        while let Some(event) = self.block_events.pop() {
            if !self
                .simulation_levels
                .is_block_ticking((event.pos.x >> 4, event.pos.z >> 4))
            {
                deferred.push(event);
                continue;
            }
            if !self.is_in_world_bounds(event.pos) {
                continue;
            }
            let state = self.block_state(event.pos);
            if vanilla_block_state_block_type_id(state) != Some(event.block_type_id) {
                continue;
            }
            if self.trigger_block_event(event, state, broadcasts) {
                broadcasts.push(BlockEventBroadcast::Event(event));
            }
        }
        for event in deferred {
            self.block_events.block_event(event);
        }
        self.block_events.set_handling_tick(prev_handling);
    }

    fn trigger_block_event(
        &mut self,
        event: BlockEventData,
        state: i32,
        broadcasts: &mut Vec<BlockEventBroadcast>,
    ) -> bool {
        let custom_sound = self
            .chunks
            .block_entities()
            .get(above(event.pos))
            .and_then(BlockEntity::skull)
            .and_then(crate::block_entity::skull::SkullBlockEntity::note_block_sound)
            .map(str::to_string);
        let mut outcome = block_trigger_event(
            self,
            event.pos,
            state,
            event.param_a,
            event.param_b,
            custom_sound.as_deref(),
        );
        if !outcome.triggered && block_state_has_block_entity(state) {
            if let Some(block_entity) = self.chunks.block_entities_mut().get_mut(event.pos) {
                outcome.triggered = block_entity.trigger_event(event.param_a, event.param_b);
            }
        }
        if let Some(sound) = outcome.sound {
            broadcasts.push(BlockEventBroadcast::Sound(sound));
        }
        outcome.triggered
    }
}

#[cfg(test)]
mod game_rule_tests {
    use super::*;

    #[test]
    fn spread_vines_uses_the_catalog_default_and_live_boolean_update() {
        let mut rules = SimulationGameRules::vanilla_defaults();
        assert!(rules.spread_vines);

        rules.set(RULE_SPREAD_VINES, GameRuleValue::Boolean(false));
        assert!(!rules.spread_vines);
        rules.set(RULE_SPREAD_VINES, GameRuleValue::Integer(1));
        assert!(!rules.spread_vines);
    }

    #[test]
    fn entity_gamerules_use_catalog_defaults_and_live_boolean_updates() {
        let mut rules = SimulationGameRules::vanilla_defaults();
        assert!(rules.tnt_explodes);
        assert!(rules.spawner_blocks_work);

        rules.set(RULE_TNT_EXPLODES, GameRuleValue::Boolean(false));
        rules.set(RULE_SPAWNER_BLOCKS_WORK, GameRuleValue::Boolean(false));
        assert!(!rules.tnt_explodes);
        assert!(!rules.spawner_blocks_work);
    }
}

#[cfg(test)]
mod light_occlusion_tests {
    use super::*;
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    };

    fn default_state(name: &str) -> i32 {
        let block = vanilla_block_type_for_resource_id(name).unwrap();
        vanilla_block_default_state_id(block.id).unwrap()
    }

    fn slab(kind: &str) -> i32 {
        vanilla_block_state_id_by_name_and_properties(
            "minecraft:oak_slab",
            &[
                ("type".to_string(), kind.to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap()
    }

    #[test]
    fn light_block_into_uses_exact_adjacent_face_union() {
        let grass = default_state("minecraft:grass_block");
        assert_eq!(
            vanilla_light_block_into(grass, slab("bottom"), Direction::Up, 0),
            MAX_LIGHT_LEVEL + 1
        );
        assert_eq!(
            vanilla_light_block_into(grass, slab("top"), Direction::Up, 0),
            0
        );
        assert_eq!(
            vanilla_light_block_into(
                grass,
                default_state("minecraft:oak_leaves"),
                Direction::Up,
                1
            ),
            1
        );
    }
}

/// What one level borrow carries besides the level itself.
pub(super) struct LevelFrame<'a, 'w> {
    pub(super) block_interaction_policy: &'a super::block_interaction::BlockInteractionPolicy,
    pub(super) full_preparation: FullChunkPreparationInputs<'a, 'w>,
    pub(super) game_time: i64,
    pub(super) game_rules: SimulationGameRules,
    pub(super) feature_flags: SimulationFeatureFlags,
    pub(super) drop_tables: BlockDropTables<'w>,
    pub(super) loot_level_facts: LootLevelFacts<'w>,
    pub(super) structure_services: Option<&'a structure_block::StructureBlockServices<'w>>,
    pub(super) players: &'a [NearbyPlayer],
    pub(super) item_spawns: &'a mut PendingEntitySpawns,
}

/// The level fields a [`LevelCtx`] leaves to its caller.
pub(super) struct LevelRest<'w> {
    pub(super) entities: &'w mut EntityStore,
}

impl<'a, 'w: 'a> LevelCtx<'a, 'w> {
    pub(super) fn new(
        level: &'w mut super::ServerLevel,
        mut frame: LevelFrame<'a, 'w>,
    ) -> (Self, LevelRest<'w>) {
        let nether_portal_dimension = matches!(
            level.dimension.builtin(),
            Some(
                bbb_server_types::save::Java26BuiltinDimension::Overworld
                    | bbb_server_types::save::Java26BuiltinDimension::Nether
            )
        );
        let super::ServerLevel {
            dimension,
            world_border,
            raids,
            spawn_facts,
            chunks,
            chunk_lifecycle,
            saved_chunks,
            saved_tick_chunks,
            block_ticks,
            fluid_ticks,
            block_events,
            simulation_levels,
            neighbor_updater,
            entities,
            cross_level_entities,
            sub_tick_count,
            random,
            changed_block_broadcasts,
            event_sink,
            recent_torch_toggles,
            weather,
            sky_darkening,
            ..
        } = level;
        frame
            .loot_level_facts
            .dimension_key
            .get_or_insert_with(|| std::sync::Arc::clone(dimension));
        let ctx = Self {
            dimension: &**dimension,
            structure_services: frame.structure_services,
            world_border: *world_border,
            block_interaction_policy: frame.block_interaction_policy,
            sculk_warning: None,
            chunks,
            full_preparation: FullChunkPreparation {
                originals: OriginalMobInputs::new(
                    frame.full_preparation.originals,
                    dimension,
                    simulation_levels,
                    *world_border,
                    weather,
                    entities.item_publication(),
                ),
                entity_nbt: frame.full_preparation.entity_nbt,
                lifecycle: chunk_lifecycle,
                saved_chunks,
                saved_tick_chunks,
            },
            sky_darkening: sky_darkening.map(i32::from).unwrap_or(0),
            ambient_light: spawn_facts.ambient_light,
            logical_height: spawn_facts.logical_height,
            weather,
            nether_portal_dimension,
            game_time: frame.game_time,
            sub_tick_count,
            random,
            block_ticks,
            fluid_ticks,
            block_events,
            changed_block_broadcasts,
            game_rules: frame.game_rules,
            feature_flags: frame.feature_flags,
            cross_level_entities,
            simulation_levels,
            drop_tables: frame.drop_tables,
            loot_level_facts: std::rc::Rc::new(frame.loot_level_facts.with_raids(raids)),
            event_sink,
            item_spawns: frame.item_spawns,
            players: frame.players,
            recent_torch_toggles,
            portal_context: None,
            portal_request: None,
            detached_entities: None,
            structure_entities: None,
            neighbours: neighbor_updater,
        };
        (ctx, LevelRest { entities })
    }
}

impl<'w> LevelCtx<'_, 'w> {
    pub(super) fn reborrow(&mut self) -> LevelCtx<'_, 'w> {
        LevelCtx {
            portal_context: None,
            portal_request: None,

            detached_entities: self.detached_entities,
            dimension: self.dimension,
            structure_services: self.structure_services,
            world_border: self.world_border,
            block_interaction_policy: self.block_interaction_policy,
            sculk_warning: self.sculk_warning,
            sky_darkening: self.sky_darkening,
            ambient_light: self.ambient_light,
            logical_height: self.logical_height,
            weather: self.weather,
            nether_portal_dimension: self.nether_portal_dimension,
            game_time: self.game_time,
            game_rules: self.game_rules,
            feature_flags: self.feature_flags,
            cross_level_entities: self.cross_level_entities,
            simulation_levels: self.simulation_levels,
            players: self.players,
            chunks: &mut *self.chunks,
            full_preparation: self.full_preparation.reborrow(),
            sub_tick_count: &mut *self.sub_tick_count,
            random: self.random,
            block_ticks: &mut *self.block_ticks,
            fluid_ticks: &mut *self.fluid_ticks,
            block_events: &mut *self.block_events,
            changed_block_broadcasts: &mut *self.changed_block_broadcasts,
            event_sink: &mut *self.event_sink,
            item_spawns: &mut *self.item_spawns,
            recent_torch_toggles: &mut *self.recent_torch_toggles,
            neighbours: &mut *self.neighbours,

            drop_tables: BlockDropTables {
                tables: self.drop_tables.tables,
                recipes: self.drop_tables.recipes,
                damage_type_tags: self.drop_tables.damage_type_tags,
            },
            loot_level_facts: std::rc::Rc::clone(&self.loot_level_facts),
            structure_entities: self
                .structure_entities
                .as_mut()
                .map(|context| context.reborrow()),
        }
    }
}
