//! `EnderDragonFight` (`EnderDragonFight.java`): the End's boss bar, the
//! dragon it (re)spawns, the exit portal, the gateways, the egg and the
//! four-crystal respawn (`DragonRespawnStage.java`).

use std::collections::BTreeSet;

use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID, VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID,
};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_DRAGON_EGG_ID, VANILLA_BLOCK_END_PORTAL_ID,
    VANILLA_BLOCK_END_STONE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
};
use bbb_protocol::packets::{BlockPos, BossBarColor, BossBarOverlay, BossEventFlags, Vec3d};
use bbb_protocol::RichTextComponent;
use bbb_server_datapack::worldgen::{ConfiguredFeature, EndSpike, EndSpikeConfig};
use bbb_server_persistence::{Java26DragonRespawnStage as Stage, Java26EnderDragonFight};
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use uuid::Uuid;

use super::boss_events::{BossPackets, ServerBossEvent};
use super::simulation_level::{LevelCtx, CHUNK_FULL_LEVEL};
use super::{LevelId, ServerWorld, WorldEvent};
use crate::entity::{end_crystal, ender_dragon, EntityStore, RemovalReason};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::PendingHurtEntity;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::random::GameRandom;

/// `EnderDragonFight.DRAGON_SPAWN_Y` over the fight origin `BlockPos.ZERO`.
const CENTER: BlockPos = BlockPos { x: 0, y: 128, z: 0 };
const GATEWAY_COUNT: i32 = 20;
/// `DimensionType.MIN_Y`/`MAX_Y`, the vertical span of a spike's top box.
const SPIKE_BOX_Y: (f64, f64) = (-2032.0, 2031.0);

pub(super) struct DragonFight {
    saved: Java26EnderDragonFight,
    boss: ServerBossEvent,
    ticks_since_dragon_seen: i32,
    alive_crystals: i32,
    ticks_since_crystals_scanned: i32,
    ticks_since_last_player_scan: i32,
}

impl DragonFight {
    /// `EnderDragonFight.init` (`EnderDragonFight.java:149-163`).
    pub(super) fn new(mut saved: Java26EnderDragonFight, seed: i64) -> Self {
        if saved.gateways.is_empty() {
            saved.gateways = (0..GATEWAY_COUNT).collect();
            let random = GameRandom::with_seed(seed);
            for i in (1..saved.gateways.len()).rev() {
                let j = random.next_int(i as i32 + 1) as usize;
                saved.gateways.swap(i, j);
            }
        }
        let name = RichTextComponent::try_from_component_nbt(crate::commands::translatable(
            "entity.minecraft.ender_dragon",
            Vec::new(),
        ))
        .expect("a translatable entity name is a valid component");
        Self {
            saved,
            boss: ServerBossEvent::new(
                name,
                BossBarColor::Pink,
                BossBarOverlay::Progress,
                BossEventFlags {
                    play_music: true,
                    create_world_fog: true,
                    ..BossEventFlags::default()
                },
            ),
            ticks_since_dragon_seen: 0,
            alive_crystals: 0,
            ticks_since_crystals_scanned: 0,
            ticks_since_last_player_scan: 21,
        }
    }

    pub(super) fn saved(&self) -> &Java26EnderDragonFight {
        &self.saved
    }

    pub(super) fn forget_player(&mut self, connection: crate::connection::ConnectionId) {
        self.boss.forget_player(connection);
    }

    // EnderDragonFight.java:178-179; TicketStorage.java:138-140; TicketType.java:19.
    pub(super) fn chunk_ticket(&self) -> Option<(crate::chunk_sender::ChunkKey, i32)> {
        (!self.boss.players().is_empty()).then_some(((0, 0), CHUNK_FULL_LEVEL - 9))
    }

    fn set_stage(&mut self, stage: Option<Stage>) {
        self.saved.respawn_time = 0;
        self.saved.respawn_stage = stage;
    }
}

fn default_state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).expect("a built-in block has a default state")
}

fn is_block(state: i32, block: i32) -> bool {
    vanilla_block_state_block_type_id(state) == Some(block)
}

fn offset(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x + x,
        y: pos.y + y,
        z: pos.z + z,
    }
}

/// `exitPortalPattern` (`EnderDragonFight.java:84-91`) laid out downwards with
/// `origin` at `getBlock(3, 3, 3)`: a three-block pillar over the bedrock ring
/// and the five-wide disc under it. The pattern is symmetric, so every
/// horizontal facing matches the same blocks.
const RING: [&str; 7] = [
    "  ###  ", " #   # ", "#     #", "#  #  #", "#     #", " #   # ", "  ###  ",
];
const DISC: [&str; 7] = [
    "       ", "  ###  ", " ##### ", " ##### ", " ##### ", "  ###  ", "       ",
];

fn exit_portal_matches(level: &LevelCtx<'_, '_>, origin: BlockPos) -> bool {
    let bedrock = |pos: BlockPos| is_block(level.block_state(pos), VANILLA_BLOCK_BEDROCK_ID);
    let layer = |rows: &[&str; 7], dy: i32| {
        rows.iter().enumerate().all(|(dz, row)| {
            row.bytes().enumerate().all(|(dx, cell)| {
                cell != b'#' || bedrock(offset(origin, dx as i32 - 3, dy, dz as i32 - 3))
            })
        })
    };
    (1..=3).all(|dy| bedrock(offset(origin, 0, dy, 0))) && layer(&RING, 0) && layer(&DISC, -1)
}

/// The end portal and gateway block entities (`TheEndPortalBlockEntity` and
/// its subclass) in the 17x17 chunks around the fight origin.
fn exit_portal_block_entities<'a>(
    level: &'a LevelCtx<'_, '_>,
) -> impl Iterator<Item = BlockPos> + 'a {
    let block_entities = level.chunks.block_entities();
    (-8..=8)
        .flat_map(|x| (-8..=8).map(move |z| (x, z)))
        .flat_map(move |chunk| block_entities.chunk(chunk))
        .filter(|body| {
            matches!(
                body.type_id(),
                VANILLA_BLOCK_ENTITY_TYPE_END_PORTAL_ID | VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID
            )
        })
        .map(|body| body.block_pos())
}

/// `EnderDragonFight.findExitPortal` (`EnderDragonFight.java:307-346`): the
/// pattern around any portal block entity of the arena, then the column scan
/// down from the podium's `MOTION_BLOCKING` height. Only upright matches are
/// tried; the pattern is horizontally symmetric.
fn find_exit_portal(level: &LevelCtx<'_, '_>) -> Option<BlockPos> {
    exit_portal_block_entities(level)
        .find_map(|pos| {
            (-1..=3).find_map(|dy| {
                (-3..=3).find_map(|dz| {
                    (-3..=3)
                        .map(|dx| offset(pos, -dx, -dy, -dz))
                        .find(|&origin| exit_portal_matches(level, origin))
                })
            })
        })
        .or_else(|| {
            let top = level
                .motion_blocking_height(0, 0)
                .unwrap_or_else(|| level.min_y());
            (level.min_y()..=top)
                .rev()
                .map(|y| BlockPos { x: 0, y, z: 0 })
                .find(|&origin| exit_portal_matches(level, origin))
        })
}

/// `hasActiveExitPortal` (`EnderDragonFight.java:291-305`).
fn has_active_exit_portal(level: &LevelCtx<'_, '_>) -> bool {
    exit_portal_block_entities(level).next().is_some()
}

/// `spawnExitPortal` (`EnderDragonFight.java:445-462`).
fn spawn_exit_portal(
    saved: &mut Java26EnderDragonFight,
    level: &mut LevelCtx<'_, '_>,
    active: bool,
) {
    let location = *saved.exit_portal_location.get_or_insert_with(|| {
        let mut y = level
            .motion_blocking_no_leaves_height(0, 0)
            .unwrap_or_else(|| level.min_y())
            - 1;
        while is_block(
            level.block_state(BlockPos { x: 0, y, z: 0 }),
            VANILLA_BLOCK_BEDROCK_ID,
        ) && y > 63
        {
            y -= 1;
        }
        BlockPos {
            x: 0,
            y: y.max(level.min_y() + 1),
            z: 0,
        }
    });
    bbb_server_worldgen::place_end_podium(level, location, active);
}

/// `EnderDragonFight.findExitPortal` with its `exitPortalLocation` write.
fn locate_exit_portal(saved: &mut Java26EnderDragonFight, level: &LevelCtx<'_, '_>) -> bool {
    let Some(found) = find_exit_portal(level) else {
        return false;
    };
    saved.exit_portal_location.get_or_insert(found);
    true
}

fn crystals_in(store: &EntityStore, area: Aabb) -> Vec<i32> {
    store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|&id| {
            store.get(id).is_some_and(|crystal| {
                crystal.entity_type_id() == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
                    && crystal.is_alive()
                    && crystal.bounding_box().intersects(area)
            })
        })
        .collect()
}

fn spike_top_box(spike: &EndSpike) -> Aabb {
    let (x, z, r) = (
        f64::from(spike.center_x),
        f64::from(spike.center_z),
        f64::from(spike.radius),
    );
    Aabb {
        min_x: x - r,
        min_y: SPIKE_BOX_Y.0,
        min_z: z - r,
        max_x: x + r,
        max_y: SPIKE_BOX_Y.1,
        max_z: z + r,
    }
}

fn alive_dragons(store: &EntityStore) -> Vec<(i32, Uuid)> {
    store
        .dragon_ids()
        .iter()
        .filter_map(|&id| store.get(id))
        .filter(|entity| {
            entity.entity_type_id() == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID && entity.is_alive()
        })
        .map(|entity| (entity.id(), entity.uuid()))
        .collect()
}

/// `createNewDragon` (`EnderDragonFight.java:464-478`).
fn create_new_dragon(store: &mut EntityStore, level: &LevelCtx<'_, '_>) -> Option<Uuid> {
    let position = Vec3d {
        x: f64::from(CENTER.x),
        y: f64::from(CENTER.y),
        z: f64::from(CENTER.z),
    };
    let y_rot = level.level_random().next_float() * 360.0;
    let id = store.spawn_general_living(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, position)?;
    let dragon = store.get_mut(id)?;
    dragon.set_rot(y_rot, 0.0);
    ender_dragon::begin_holding_pattern(dragon);
    Some(dragon.uuid())
}

fn set_beams(store: &mut EntityStore, crystals: &[i32], target: Option<BlockPos>) {
    for &id in crystals {
        if let Some(crystal) = store.get_mut(id) {
            end_crystal::set_beam_target(crystal, target);
        }
    }
}

/// `resetSpikeCrystals` (`EnderDragonFight.java:574-581`).
fn reset_spike_crystals(store: &mut EntityStore, seed: i64) {
    for spike in bbb_server_worldgen::end_spike::spikes_for_seed(seed) {
        for id in crystals_in(store, spike_top_box(&spike)) {
            if let Some(crystal) = store.get_mut(id) {
                crystal.set_invulnerable(false);
                end_crystal::set_beam_target(crystal, None);
            }
        }
    }
}

impl ServerWorld {
    pub(super) fn prepare_dragon_fight(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let seed = self.cold.seed;
        let level = self.level_mut_by_id(dimension);
        if !level.spawn_facts.has_ender_dragon_fight {
            return;
        }
        let mut fight = level
            .dragon_fight
            .take()
            .unwrap_or_else(|| DragonFight::new(Java26EnderDragonFight::default(), seed));
        let mut packets = BossPackets::new();
        fight
            .boss
            .set_visible(!fight.saved.dragon_killed, &mut packets);
        fight.ticks_since_last_player_scan += 1;
        if fight.ticks_since_last_player_scan >= 20 {
            let players = self.dragon_fight_players(dimension);
            fight.boss.set_players(&players, &mut packets);
            fight.ticks_since_last_player_scan = 0;
        }
        self.level_mut_by_id(dimension).dragon_fight = Some(fight);
        send(packets, events);
    }

    /// `EnderDragonFight.tick` (`EnderDragonFight.java:171-219`), run from
    /// `ServerLevel.tick` ahead of the entity loop.
    pub(super) fn tick_dragon_fight(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        let Some(mut fight) = self.level_mut_by_id(dimension).dragon_fight.take() else {
            return;
        };
        let mut packets = BossPackets::new();
        self.advance_dragon_fight(dimension, &mut fight, &mut packets, events);
        self.level_mut_by_id(dimension).dragon_fight = Some(fight);
        send(packets, events);
        self.keep_crystal_fires(dimension);
    }

    /// `EndCrystal.tick` (`EndCrystal.java:53-62`): with a fight in the level,
    /// each crystal relights the air in its block.
    fn keep_crystal_fires(&mut self, dimension: &LevelId) {
        if self
            .level_by_id(dimension)
            .entities
            .end_crystal_ids()
            .is_empty()
        {
            return;
        }
        self.with_level_and_entities(*dimension, |level, store| {
            let crystals: Vec<BlockPos> = store
                .end_crystal_ids()
                .iter()
                .filter_map(|&id| store.get(id))
                .filter(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
                        && entity.removal_reason().is_none()
                })
                .map(|entity| entity.block_position())
                .collect();
            for pos in crystals {
                if let Some(fire) = crate::block_behavior::fire::surviving_fire_state(level, pos) {
                    level.set_block(pos, fire, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                }
            }
        });
    }

    fn advance_dragon_fight(
        &mut self,
        dimension: &LevelId,
        fight: &mut DragonFight,
        packets: &mut BossPackets,
        events: &mut Vec<WorldEvent>,
    ) {
        self.link_dragon(dimension, fight, packets);
        if (!fight.saved.needs_state_scanning
            && fight.saved.respawn_stage.is_none()
            && fight.saved.dragon_killed)
            || fight.boss.players().is_empty()
            || !self.dragon_arena_loaded(dimension)
        {
            return;
        }
        if fight.saved.needs_state_scanning {
            self.scan_dragon_fight_state(dimension, fight);
            fight.saved.needs_state_scanning = false;
        }
        if let Some(stage) = fight.saved.respawn_stage {
            let crystals = self.respawn_crystals(dimension, fight);
            if crystals.is_empty() {
                self.abort_dragon_respawn(dimension, fight);
                return;
            }
            let time = fight.saved.respawn_time;
            fight.saved.respawn_time += 1;
            self.tick_respawn_stage(dimension, fight, stage, &crystals, time, events);
        }
        if !fight.saved.dragon_killed {
            if fight.saved.dragon_uuid.is_none() || {
                fight.ticks_since_dragon_seen += 1;
                fight.ticks_since_dragon_seen >= 1200
            } {
                self.find_or_create_dragon(dimension, fight);
                fight.ticks_since_dragon_seen = 0;
            }
            fight.ticks_since_crystals_scanned += 1;
            if fight.ticks_since_crystals_scanned >= 100 {
                self.update_crystal_count(dimension, fight);
            }
        }
    }

    /// `EnderDragon.dragonFight` and `EnderDragonFight.updateDragon`
    /// (`EnderDragon.java:165-170`, `EnderDragonFight.java:480-488`): only the
    /// dragon the fight names belongs to it.
    fn link_dragon(
        &mut self,
        dimension: &LevelId,
        fight: &mut DragonFight,
        packets: &mut BossPackets,
    ) {
        let link = ender_dragon::FightLink {
            alive_crystals: fight.alive_crystals,
            previously_killed: fight.saved.previously_killed,
        };
        let scoreboard = self.scoreboard.board();
        let store = &mut self
            .levels
            .get_mut(dimension)
            .expect("every reachable dimension is a level this world runs")
            .entities;
        if store.dragon_ids().is_empty() {
            return;
        }
        let ids: Vec<i32> = store.dragon_ids().iter().copied().collect();
        for id in ids {
            let Some(dragon) = store.get_mut(id) else {
                continue;
            };
            if ender_dragon::state(dragon).is_none() {
                continue;
            }
            let ours = Some(dragon.uuid()) == fight.saved.dragon_uuid;
            ender_dragon::set_fight(dragon, ours.then_some(link));
            if !ours {
                continue;
            }
            if let Some(living) = dragon.living() {
                fight
                    .boss
                    .set_progress(living.health() / living.max_health(), packets);
            }
            fight.ticks_since_dragon_seen = 0;
            if dragon.custom_name().is_some() {
                fight.boss.set_name(
                    super::boss_events::boss_display_name(
                        dragon,
                        scoreboard,
                        self.cold.config.plugins.content_registry(),
                    ),
                    packets,
                );
            }
        }
    }

    /// `updatePlayers` over `validPlayer` (`EnderDragonFight.java:156`,
    /// `:373-387`): living players within 192 of the spawn point.
    fn dragon_fight_players(
        &self,
        dimension: &LevelId,
    ) -> BTreeSet<crate::connection::ConnectionId> {
        let center = Vec3d {
            x: f64::from(CENTER.x),
            y: f64::from(CENTER.y),
            z: f64::from(CENTER.z),
        };
        self.levels[dimension]
            .player_order
            .iter()
            .filter(|&&connection| {
                self.levels.player(connection).is_some_and(|player| {
                    !player.avatar_removed
                        && !player.damage.is_dead_or_dying()
                        && player.position().distance_to_sqr(center) <= 192.0 * 192.0
                })
            })
            .copied()
            .collect()
    }

    /// `Level.getChunk` loads what the fight writes to: a live write needs
    /// the column FULL.
    fn load_fight_chunks(&mut self, dimension: &LevelId, from: BlockPos, to: BlockPos) {
        let chunks: Vec<_> = (from.x >> 4..=to.x >> 4)
            .flat_map(|x| (from.z >> 4..=to.z >> 4).map(move |z| (x, z)))
            .collect();
        if let Err(error) = self.install_full_chunks_by_key(dimension, &chunks) {
            tracing::warn!("dragon fight chunk load: {error}");
        }
    }

    fn load_podium(&mut self, dimension: &LevelId) {
        let reach = BlockPos { x: 4, y: 0, z: 4 };
        self.load_fight_chunks(dimension, offset(reach, -8, 0, -8), reach);
    }

    fn dragon_arena_loaded(&self, dimension: &LevelId) -> bool {
        let level = self.level_by_id(dimension);
        // EnderDragonFight.java:349-370: block-ticking arena and central entities.
        (-8..=8).all(|x| {
            (-8..=8).all(|z| {
                level.chunk_lifecycle.status((x, z))
                    >= super::chunk_lifecycle::FullChunkStatus::BlockTicking
            })
        }) && (-1..=1).all(|x| (-1..=1).all(|z| level.chunk_lifecycle.entities_loaded((x, z))))
    }

    /// `scanState` (`EnderDragonFight.java:221-255`).
    fn scan_dragon_fight_state(&mut self, dimension: &LevelId, fight: &mut DragonFight) {
        self.load_podium(dimension);
        let saved = &mut fight.saved;
        self.with_stored_hurt_level(dimension, |store, level| {
            let active = has_active_exit_portal(level);
            saved.previously_killed = active;
            if !active && !locate_exit_portal(saved, level) {
                spawn_exit_portal(saved, level, false);
            }
            match alive_dragons(store).first() {
                None => saved.dragon_killed = true,
                Some(&(id, uuid)) => {
                    saved.dragon_uuid = Some(uuid);
                    saved.dragon_killed = false;
                    if !active {
                        store.remove(id, RemovalReason::Discarded);
                        saved.dragon_uuid = None;
                    }
                }
            }
            if !saved.previously_killed && saved.dragon_killed {
                saved.dragon_killed = false;
            }
        });
    }

    /// `findOrCreateDragon` (`EnderDragonFight.java:257-267`).
    fn find_or_create_dragon(&mut self, dimension: &LevelId, fight: &mut DragonFight) {
        let saved = &mut fight.saved;
        self.with_stored_hurt_level(dimension, |store, level| {
            saved.dragon_uuid = match alive_dragons(store).first() {
                Some(&(_, uuid)) => Some(uuid),
                None => create_new_dragon(store, level),
            };
        });
    }

    /// `updateCrystalCount` (`EnderDragonFight.java:389-398`).
    fn update_crystal_count(&self, dimension: &LevelId, fight: &mut DragonFight) {
        let store = &self.level_by_id(dimension).entities;
        fight.ticks_since_crystals_scanned = 0;
        fight.alive_crystals = bbb_server_worldgen::end_spike::spikes_for_seed(self.cold.seed)
            .iter()
            .map(|spike| crystals_in(store, spike_top_box(spike)).len() as i32)
            .sum();
    }

    /// The `respawnCrystals` still in the level.
    fn respawn_crystals(&self, dimension: &LevelId, fight: &DragonFight) -> Vec<i32> {
        let store = &self.level_by_id(dimension).entities;
        fight
            .saved
            .respawn_crystals
            .iter()
            .filter_map(|&uuid| store.entity_id_for_uuid(uuid))
            .filter(|&id| store.get(id).is_some_and(|crystal| crystal.is_alive()))
            .collect()
    }

    /// `abortRespawnSequence` (`EnderDragonFight.java:505-512`).
    fn abort_dragon_respawn(&mut self, dimension: &LevelId, fight: &mut DragonFight) {
        fight.set_stage(None);
        self.load_podium(dimension);
        let seed = self.cold.seed;
        let saved = &mut fight.saved;
        self.with_stored_hurt_level(dimension, |store, level| {
            reset_spike_crystals(store, seed);
            spawn_exit_portal(saved, level, true);
        });
    }

    /// `DragonRespawnStage.tick` (`DragonRespawnStage.java`).
    fn tick_respawn_stage(
        &mut self,
        dimension: &LevelId,
        fight: &mut DragonFight,
        stage: Stage,
        crystals: &[i32],
        time: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let summon_event = |world: &mut Self| {
            world.with_level(*dimension, |level| level.level_event(3001, CENTER, 0));
        };
        match stage {
            Stage::Start => {
                set_beams(
                    &mut self.level_mut_by_id(dimension).entities,
                    crystals,
                    Some(CENTER),
                );
                fight.set_stage(Some(Stage::PreparingToSummonPillars));
            }
            Stage::PreparingToSummonPillars => {
                if time >= 100 {
                    fight.set_stage(Some(Stage::SummoningPillars));
                } else if matches!(time, 0 | 50 | 51 | 52) || time >= 95 {
                    summon_event(self);
                }
            }
            Stage::SummoningPillars => {
                let (start, end) = (time % 40 == 0, time % 40 == 39);
                if !start && !end {
                    return;
                }
                let spikes = bbb_server_worldgen::end_spike::spikes_for_seed(self.cold.seed);
                match spikes.get((time / 40) as usize) {
                    Some(spike) if start => {
                        let top = BlockPos {
                            x: spike.center_x,
                            y: spike.height + 1,
                            z: spike.center_z,
                        };
                        set_beams(
                            &mut self.level_mut_by_id(dimension).entities,
                            crystals,
                            Some(top),
                        );
                    }
                    Some(spike) => self.rebuild_spike(dimension, *spike, events),
                    None if start => fight.set_stage(Some(Stage::SummoningDragon)),
                    None => {}
                }
            }
            Stage::SummoningDragon => {
                if time >= 100 {
                    self.finish_dragon_respawn(dimension, fight, crystals, events);
                } else if time >= 80 {
                    summon_event(self);
                } else if time == 0 {
                    set_beams(
                        &mut self.level_mut_by_id(dimension).entities,
                        crystals,
                        Some(CENTER),
                    );
                } else if time < 5 {
                    summon_event(self);
                }
            }
            Stage::End => {}
        }
    }

    /// The `SUMMONING_PILLARS` end-of-beam rebuild (`DragonRespawnStage.java:56-76`):
    /// the cube around the spike top is cleared, blown up and replaced by a
    /// fresh spike whose invulnerable crystal beams at the dragon spawn.
    fn rebuild_spike(
        &mut self,
        dimension: &LevelId,
        spike: EndSpike,
        events: &mut Vec<WorldEvent>,
    ) {
        let (x, h, z) = (spike.center_x, spike.height, spike.center_z);
        let center = BlockPos { x, y: h, z };
        self.load_fight_chunks(
            dimension,
            offset(center, -10, 0, -10),
            offset(center, 10, 0, 10),
        );
        self.with_level(*dimension, |level| {
            for dy in -10..=10 {
                for dz in -10..=10 {
                    for dx in -10..=10 {
                        level.remove_block(BlockPos {
                            x: x + dx,
                            y: h + dy,
                            z: z + dz,
                        });
                    }
                }
            }
        });
        let _ = self.explode(
            *dimension,
            PendingExplosion {
                source: None,
                center: Vec3d {
                    x: f64::from(x as f32 + 0.5),
                    y: f64::from(h),
                    z: f64::from(z as f32 + 0.5),
                },
                radius: 5.0,
                fire: false,
                interaction: ExplosionInteraction::Block,
                damage_source: None,
                damage_profile: ExplosionDamageProfile::Default,
                presentation: ExplosionPresentation::Default,
            },
            events,
        );
        let feature = ConfiguredFeature::EndSpike(EndSpikeConfig {
            crystal_invulnerable: true,
            spikes: vec![spike],
            crystal_beam_target: Some(CENTER),
        });
        self.with_stored_hurt_level(dimension, |store, level| {
            let mut random = level.level_random();
            let _ = bbb_server_worldgen::place_configured_feature(
                &feature,
                level,
                BlockPos { x, y: 45, z },
                &mut random,
            );
            // `EndSpikeFeature.placeSpike`'s crystal (`EndSpikeFeature.java:110-115`):
            // the live level does not add worldgen entities itself.
            let position = Vec3d {
                x: f64::from(x) + 0.5,
                y: f64::from(h + 1),
                z: f64::from(z) + 0.5,
            };
            let y_rot = level.level_random().next_float() * 360.0;
            if let Some(crystal) = end_crystal::spawn_end_crystal(store, position, true)
                .and_then(|id| store.get_mut(id))
            {
                crystal.set_rot(y_rot, 0.0);
                crystal.set_invulnerable(true);
                end_crystal::set_beam_target(crystal, Some(CENTER));
            }
        });
    }

    /// `SUMMONING_DRAGON` at 100 ticks (`DragonRespawnStage.java:86-95`) with
    /// `setRespawnStage(END)` (`EnderDragonFight.java:269-289`).
    fn finish_dragon_respawn(
        &mut self,
        dimension: &LevelId,
        fight: &mut DragonFight,
        crystals: &[i32],
        events: &mut Vec<WorldEvent>,
    ) {
        fight.set_stage(None);
        fight.saved.dragon_killed = false;
        let seed = self.cold.seed;
        let mut explosions = Vec::new();
        let saved = &mut fight.saved;
        self.with_stored_hurt_level(dimension, |store, level| {
            saved.dragon_uuid = create_new_dragon(store, level);
            reset_spike_crystals(store, seed);
            for &id in crystals {
                let Some(crystal) = store.get_mut(id) else {
                    continue;
                };
                end_crystal::set_beam_target(crystal, None);
                explosions.push(PendingExplosion {
                    source: Some(ExplosionSource {
                        direct: crate::entity::damage::explosion_source_entity(crystal),
                        indirect: None,
                        causing: None,
                        in_water: false,
                        wind_charge: false,
                    }),
                    center: crystal.position(),
                    radius: 6.0,
                    fire: false,
                    interaction: ExplosionInteraction::None,
                    damage_source: None,
                    damage_profile: ExplosionDamageProfile::Default,
                    presentation: ExplosionPresentation::Default,
                });
                store.remove(id, RemovalReason::Discarded);
            }
        });
        // EnderDragonFight.java:275-284 grants only the boss bar's players.
        if let Some(dragon_id) = fight.saved.dragon_uuid.and_then(|uuid| {
            self.level_by_id(dimension)
                .entities
                .entity_id_for_uuid(uuid)
        }) {
            for &connection_id in fight.boss.players() {
                self.trigger_summoned_entity(connection_id, *dimension, dragon_id, events);
            }
        }
        for explosion in explosions {
            let _ = self.explode(*dimension, explosion, events);
        }
    }

    /// `EnderDragonFight.tryRespawn` (`EnderDragonFight.java:518-549`): four
    /// crystals, one on each side of the exit portal, start the respawn.
    pub(super) fn try_respawn_dragon(&mut self, dimension: &LevelId) {
        let Some(mut fight) = self
            .levels
            .get_mut(dimension)
            .and_then(|level| level.dragon_fight.take())
        else {
            return;
        };
        if fight.saved.dragon_killed && fight.saved.respawn_stage.is_none() {
            self.respawn_from_crystals(dimension, &mut fight);
        }
        self.level_mut_by_id(dimension).dragon_fight = Some(fight);
    }

    fn respawn_from_crystals(&mut self, dimension: &LevelId, fight: &mut DragonFight) {
        self.load_podium(dimension);
        let saved = &mut fight.saved;
        let started = self.with_stored_hurt_level(dimension, |store, level| {
            if saved.exit_portal_location.is_none() && !locate_exit_portal(saved, level) {
                spawn_exit_portal(saved, level, true);
            }
            let center = offset(saved.exit_portal_location?, 0, 1, 0);
            let mut crystals = Vec::new();
            for (dx, dz) in [(0, -3), (3, 0), (0, 3), (-3, 0)] {
                let pos = offset(center, dx, 0, dz);
                let found = crystals_in(store, Aabb::of_block(pos));
                if found.is_empty() {
                    return None;
                }
                crystals.extend(found);
            }
            // `respawnDragon` (`EnderDragonFight.java:551-572`).
            while let Some(origin) = find_exit_portal(level) {
                for dy in -1..=3 {
                    for dz in -3..=3 {
                        for dx in -3..=3 {
                            let pos = offset(origin, dx, dy, dz);
                            let state = level.block_state(pos);
                            if is_block(state, VANILLA_BLOCK_BEDROCK_ID)
                                || is_block(state, VANILLA_BLOCK_END_PORTAL_ID)
                            {
                                level.set_block(
                                    pos,
                                    default_state(VANILLA_BLOCK_END_STONE_ID),
                                    SET_BLOCK_UPDATE_ALL,
                                    UPDATE_LIMIT,
                                );
                            }
                        }
                    }
                }
            }
            spawn_exit_portal(saved, level, false);
            Some(
                crystals
                    .into_iter()
                    .filter_map(|id| store.get(id).map(|crystal| crystal.uuid()))
                    .collect::<Vec<_>>(),
            )
        });
        if let Some(crystals) = started {
            fight.set_stage(Some(Stage::Start));
            fight.saved.respawn_crystals = crystals;
        }
    }

    /// `EnderDragonFight.setDragonKilled` (`EnderDragonFight.java:400-417`).
    pub(super) fn dragon_fight_killed(
        &mut self,
        dimension: &LevelId,
        dragon: Uuid,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(mut fight) = self
            .levels
            .get_mut(dimension)
            .and_then(|level| level.dragon_fight.take())
        else {
            return;
        };
        if fight.saved.dragon_uuid == Some(dragon) {
            let mut packets = BossPackets::new();
            fight.boss.set_progress(0.0, &mut packets);
            fight.boss.set_visible(false, &mut packets);
            send(packets, events);
            // `spawnNewGateway` (`EnderDragonFight.java:426-443`).
            let gateway = fight.saved.gateways.pop().map(|gateway| {
                let angle = 2.0
                    * (-std::f64::consts::PI + std::f64::consts::PI / 20.0 * f64::from(gateway));
                BlockPos {
                    x: (96.0 * angle.cos()).floor() as i32,
                    y: 75,
                    z: (96.0 * angle.sin()).floor() as i32,
                }
            });
            self.load_podium(dimension);
            let saved = &mut fight.saved;
            self.with_level(*dimension, |level| {
                spawn_exit_portal(saved, level, true);
                if let Some(pos) = gateway {
                    level.level_event(3000, pos, 0);
                }
            });
            if let Some(pos) = gateway {
                self.load_fight_chunks(dimension, offset(pos, -1, 0, -1), offset(pos, 1, 0, 1));
                self.place_live_configured_feature(
                    *dimension,
                    "minecraft:end_gateway_delayed",
                    pos,
                );
            }
            if !fight.saved.previously_killed {
                self.with_level(*dimension, |level| {
                    let y = level
                        .motion_blocking_height(0, 0)
                        .unwrap_or_else(|| level.min_y());
                    level.set_block(
                        BlockPos { x: 0, y, z: 0 },
                        default_state(VANILLA_BLOCK_DRAGON_EGG_ID),
                        SET_BLOCK_UPDATE_ALL,
                        UPDATE_LIMIT,
                    );
                });
            }
            fight.saved.previously_killed = true;
            fight.saved.dragon_killed = true;
        }
        self.level_mut_by_id(dimension).dragon_fight = Some(fight);
    }

    /// `EndCrystal.onDestroyedBy` into `EnderDragonFight.onCrystalDestroyed`
    /// and `EnderDragon.onCrystalDestroyed` (`EnderDragonFight.java:494-503`,
    /// `EnderDragon.java:802-815`) for crystals killed in this level.
    ///
    /// The breaker is the player behind the damage, else the nearest
    /// combat-targetable player within 64 blocks.
    pub(super) fn dragon_fight_crystals_destroyed(
        &mut self,
        dimension: &LevelId,
        crystals: Vec<(i32, Uuid, Vec3d, Option<Uuid>)>,
        events: &mut Vec<WorldEvent>,
    ) {
        if crystals.is_empty() {
            return;
        }
        let Some(mut fight) = self
            .levels
            .get_mut(dimension)
            .and_then(|level| level.dragon_fight.take())
        else {
            return;
        };
        for (crystal_id, uuid, position, destroyer) in crystals {
            if fight.saved.respawn_stage.is_some() && fight.saved.respawn_crystals.contains(&uuid) {
                self.abort_dragon_respawn(dimension, &mut fight);
                continue;
            }
            self.update_crystal_count(dimension, &mut fight);
            let Some(dragon_id) = fight.saved.dragon_uuid.and_then(|uuid| {
                self.level_by_id(dimension)
                    .entities
                    .entity_id_for_uuid(uuid)
            }) else {
                continue;
            };
            self.dragon_crystal_destroyed(
                dimension, dragon_id, crystal_id, position, destroyer, events,
            );
        }
        self.level_mut_by_id(dimension).dragon_fight = Some(fight);
    }

    fn dragon_crystal_destroyed(
        &mut self,
        dimension: &LevelId,
        dragon_id: i32,
        crystal_id: i32,
        position: Vec3d,
        destroyer: Option<Uuid>,
        events: &mut Vec<WorldEvent>,
    ) {
        let at = Vec3d {
            x: position.x.floor(),
            y: position.y.floor(),
            z: position.z.floor(),
        };
        let player = destroyer.or_else(|| {
            self.with_stored_hurt_level(dimension, |_, level| {
                level
                    .players()
                    .iter()
                    .filter(|player| {
                        !player.spectator
                            && !player.instabuild
                            && !player.invulnerable
                            && !player.dead_or_dying
                            && player.position.distance_to_sqr(at) <= 64.0 * 64.0
                    })
                    .min_by(|a, b| {
                        a.position
                            .distance_to_sqr(at)
                            .total_cmp(&b.position.distance_to_sqr(at))
                    })
                    .map(|player| player.uuid)
            })
        });
        let nearest = self
            .level_by_id(dimension)
            .entities
            .get(dragon_id)
            .and_then(ender_dragon::state)
            .is_some_and(|state| state.nearest_crystal() == Some(crystal_id));
        if nearest {
            if let Some(dragon) = self.level_mut_by_id(dimension).entities.get_mut(dragon_id) {
                ender_dragon::set_hit_part(dragon, Some(ender_dragon::DragonPart::Head));
            }
            let causing = player.and_then(|uuid| {
                self.pending_hurt_source_entity(dimension, PendingHurtEntity::Player { uuid })
            });
            let damage_type = if causing.is_some() {
                "minecraft:player_explosion"
            } else {
                "minecraft:explosion"
            };
            let _ = self.hurt_stored_entity_by_command(
                *dimension,
                dragon_id,
                damage_type,
                10.0,
                causing,
                None,
                None,
                events,
            );
        }
        self.with_stored_hurt_level(dimension, |store, level| {
            let Some(mut dragon) = store.detach(dragon_id) else {
                return;
            };
            ender_dragon::crystal_destroyed(&mut dragon, store, level, player);
            store.attach(dragon);
        });
    }
}

fn send(packets: BossPackets, events: &mut Vec<WorldEvent>) {
    for (connection_id, packet) in packets {
        events.push(WorldEvent::Send {
            connection_id,
            packet,
        });
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn dragon_fight_for_test(
        &self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
    ) -> Option<&Java26EnderDragonFight> {
        self.level_by_id(&dimension.into())
            .dragon_fight
            .as_ref()
            .map(DragonFight::saved)
    }

    pub(crate) fn dragon_part_box_for_test(
        &self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        part_id: i32,
    ) -> Option<Aabb> {
        ender_dragon::part_by_id(&self.level(dimension).entities, part_id).map(|part| part.2)
    }
}
