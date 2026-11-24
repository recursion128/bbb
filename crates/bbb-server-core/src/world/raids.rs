//! Live raids, wave membership, boss bars and hero awards for one level.

use crate::world::spawner_minecarts::raid_events::RaidLevelRuntime;
use crate::world::LevelId;
use std::collections::{BTreeMap, BTreeSet};

use crate::chunk::poi::PoiOccupancy;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::raider::{PendingRaiderRecruit, RaiderRaidFacts, RaiderWaveFacts};
use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
    VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, BossBarColor, BossBarOverlay, BossEventFlags, NbtSummaryValue, SoundEvent,
    SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::{component_literal_nbt, component_with_siblings_nbt, RichTextComponent};
use bbb_server_persistence::world_storage::saved_data::{
    Java26Raid, Java26RaidStatus, Java26Raids,
};
use bbb_server_types::geometry::{mth_cos, mth_sin};
use bbb_server_types::stats::custom_stat;
use uuid::Uuid;

use super::boss_events::{BossPackets, ServerBossEvent};
use super::WorldEvent;

/// `Raid.RAID_TIMEOUT_TICKS` (`Raid.java:87`).
const RAID_TIMEOUT_TICKS: i64 = 48000;

/// `Raid.POST_RAID_TICK_LIMIT` (`Raid.java:92`).
const POST_RAID_TICK_LIMIT: i32 = 40;

/// `Raid.DEFAULT_PRE_RAID_TICKS` (`Raid.java:93`).
const DEFAULT_PRE_RAID_TICKS: i32 = 300;

/// `Raid.ALLOW_SPAWNING_WITHIN_VILLAGE_SECONDS_THRESHOLD` (`Raid.java:84`).
const ALLOW_SPAWNING_WITHIN_VILLAGE_SECONDS_THRESHOLD: i32 = 7;

/// `Raid.VILLAGE_SEARCH_RADIUS` (`Raid.java:86`), which
/// `findRandomSpawnPos` scales its ring by .
const VILLAGE_SEARCH_RADIUS: f32 = 32.0;

/// `Raid.VALID_RAID_RADIUS` (`Raid.java:103`), which also bounds how far a
/// candidate surface may sit from the centre height .
const VALID_RAID_RADIUS: u32 = 96;

/// `Raid.MAX_CELEBRATION_TICKS` (`Raid.java:95`).
const MAX_CELEBRATION_TICKS: i32 = 600;

/// `Raid.HERO_OF_THE_VILLAGE_DURATION` (`Raid.java:102`).
const HERO_OF_THE_VILLAGE_DURATION: i32 = 48000;

/// `Raid.getMaxRaidOmenLevel` (`Raid.java:234-236`).
pub(super) const MAX_RAID_OMEN_LEVEL: i32 = 5;

/// The block radius `createOrExtendRaid` collects village records in
/// (`Raids.java:116`).
const RAID_TRIGGER_POI_RADIUS: i32 = 64;

/// `MobEffects.RAID_OMEN`, the effect whose last tick is the only caller of
/// `createOrExtendRaid` outside `/raid` (`RaidOmenMobEffect.java`).
pub(super) const RAID_OMEN_EFFECT: &str = "minecraft:raid_omen";

/// `MobEffects.HERO_OF_THE_VILLAGE` as `Raid.java` names it.
const HERO_OF_THE_VILLAGE_EFFECT: &str = "minecraft:hero_of_the_village";

/// `Raid.SECTION_RADIUS_FOR_FINDING_NEW_VILLAGE_CENTER` (`Raid.java:85`).
const SECTION_RADIUS_FOR_FINDING_NEW_VILLAGE_CENTER: i32 = 2;

/// `Raid.LOW_MOB_THRESHOLD` (`Raid.java:98`).
const LOW_MOB_THRESHOLD: usize = 2;

/// `Raid.VALID_RAID_RADIUS_SQR` (`Raid.java:104`), the bound
/// `ServerLevel.getRaidAt` hands `Raids.getNearbyRaid`
/// (`ServerLevel.java`).
const VALID_RAID_RADIUS_SQR: f64 = 9216.0;

/// `Raid.RAID_REMOVAL_THRESHOLD_SQR` (`Raid.java:105`).
const RAID_REMOVAL_THRESHOLD_SQR: f64 = 12544.0;

/// `Raid.MAX_NO_ACTION_TIME` (`Raid.java:94`).
const MAX_NO_ACTION_TIME: i32 = 2400;

/// `Raid.OUTSIDE_RAID_BOUNDS_TIMEOUT` (`Raid.java:96`).
const OUTSIDE_RAID_BOUNDS_TIMEOUT: i32 = 30;

/// These fields have no raid SavedData representation.
#[derive(Debug, Clone, PartialEq)]
struct RaidRuntime {
    /// Each wave's raiders with the health last seen on them, which is what an
    /// unloaded raider takes off the raid's total (`Raid.java:612-626`).
    raiders: BTreeMap<i32, BTreeMap<Uuid, f32>>,
    leaders: BTreeMap<i32, Uuid>,
    celebration_ticks: i32,
    boss: ServerBossEvent,
    wave_spawn_pos: Option<BlockPos>,
    /// `Raider.ticksOutsideRaid` (`Raider.java`), which only `Raid` reads
    /// or writes (`Raid.java`) and no codec saves, so the raid
    /// this runtime keeps it under is as durable as the raider it belongs to.
    ticks_outside_raid: BTreeMap<Uuid, i32>,
}

impl Default for RaidRuntime {
    fn default() -> Self {
        Self {
            raiders: BTreeMap::new(),
            leaders: BTreeMap::new(),
            celebration_ticks: 0,
            boss: raid_boss_event(),
            wave_spawn_pos: None,
            ticks_outside_raid: BTreeMap::new(),
        }
    }
}

fn raid_boss_event() -> ServerBossEvent {
    ServerBossEvent::new(
        raid_name(),
        BossBarColor::Red,
        BossBarOverlay::Notched10,
        BossEventFlags::default(),
    )
}

/// `Raid.RAID_NAME_COMPONENT` (`Raid.java:99`).
fn raid_name() -> RichTextComponent {
    raid_component(crate::commands::translatable(
        "event.minecraft.raid",
        Vec::new(),
    ))
}

/// `Raid.java`: the same name with the count of raiders still alive.
fn raiders_remaining_name(alive: i32) -> RichTextComponent {
    raid_component(
        component_with_siblings_nbt(
            crate::commands::translatable("event.minecraft.raid", Vec::new()),
            vec![
                component_literal_nbt(" - "),
                crate::commands::translatable(
                    "event.minecraft.raid.raiders_remaining",
                    vec![NbtSummaryValue::Int(alive)],
                ),
            ],
        )
        .expect("a translatable component takes siblings"),
    )
}

/// `Raid.RAID_BAR_VICTORY_COMPONENT` and `RAID_BAR_DEFEAT_COMPONENT`
/// (`Raid.java:100-101`).
fn over_name(victory: bool) -> RichTextComponent {
    let key = if victory {
        "event.minecraft.raid.victory.full"
    } else {
        "event.minecraft.raid.defeat.full"
    };
    raid_component(crate::commands::translatable(key, Vec::new()))
}

fn raid_component(value: NbtSummaryValue) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(value)
        .expect("the raid bar names are built from validated vanilla shapes")
}

/// One raider `spawnGroup` asks its level to build (`Raid.java`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RaidSpawn {
    pub(super) entity_type_id: i32,
    /// The block `joinRaid` places it half a block into and one above
    /// (`Raid.java`).
    pub(super) position: BlockPos,
    pub(super) raid_id: i32,
    pub(super) wave: i32,
    /// `Raid.java`: this raider takes `setPatrolLeader` and the
    /// ominous banner `setLeader` equips .
    pub(super) leader: bool,
    /// `Raid.getEnchantOdds` (`Raid.java`) and the provider its wave
    /// selects, which `applyRaidBuffs` draws against.
    pub(super) enchant_odds: f32,
    pub(super) enchant_provider: Option<&'static str>,
    /// The ravager this rider mounts once both are published
    /// (`Raid.java`).
    pub(super) rides: Option<Uuid>,
}

/// What the level answered: the raider it published and the health
/// `addWaveMob` adds for it (`Raid.java`), or the refusal it counted.
pub(super) enum RaidSpawnOutcome {
    Spawned { uuid: Uuid, health: f32 },
    Refused(String),
}

/// One raider as `updateRaiders` reads it (`Raid.java`). A raider this
/// level does not hold answers `None`, which is `isRemoved` and the
/// foreign-dimension test at once; the `getEntity` null check nested inside the
/// tick-count arm  is unreachable behind the same answer.
pub(super) struct RaidRaider {
    pub(super) position: BlockPos,
    pub(super) removed: bool,
    pub(super) tick_count: i32,
    pub(super) no_action_time: i32,
    pub(super) health: f32,
}

/// One session as `Raid.validPlayer` (`Raid.java`) reads it: the block
/// it stands on, and whether `LivingEntity.isAlive` holds. `playSound` reads
/// the same session's undivided position instead (`Raid.java`).
pub(super) struct RaidPlayer {
    pub(super) connection: ConnectionId,
    pub(super) position: BlockPos,
    pub(super) precise: Vec3d,
    pub(super) alive: bool,
}

/// What one `Raids.tick` pass leaves for its caller to apply.
#[derive(Debug, Default, PartialEq)]
pub(super) struct RaidTickOutcome {
    /// The raiders whose raid this pass dropped; vanilla's raiders hold a
    /// pointer the removal drops for them.
    pub(super) unbound: BTreeSet<Uuid>,
    /// Raid ids whose centre decision the level could not answer.
    pub(super) unanswered_village: Vec<i32>,
    /// Raid ids whose wave-spawn search the level could not answer.
    pub(super) unanswered_wave_spawn: Vec<i32>,
    /// What the raid boss events published this pass, in emission order.
    pub(super) boss_packets: BossPackets,
    /// Whom the victories this pass reached owe their awards.
    pub(super) heroes: Vec<RaidHero>,
    /// The raiders a wave asked for and could not get, one line each.
    pub(super) refused_raiders: Vec<(i32, String)>,
    /// What the horns this pass blew addressed, in emission order.
    pub(super) sounds: Vec<(ConnectionId, ClientboundPlayPacket)>,
}

/// What `Raids.createOrExtendRaid` reads before it touches the raid map
/// (`Raids.java`), beside the omen `absorbRaidOmen` then takes off the
/// same session (`Raid.java`). A fact the level could not answer is `None`
/// and refuses in vanilla's read order rather than standing in for it.
pub(super) struct RaidTrigger {
    /// `ServerPlayer.isSpectator` (`Raids.java`).
    pub(super) spectator: bool,
    /// `GameRules.RAIDS` (`Raids.java`).
    pub(super) raids_rule: bool,
    /// `EnvironmentAttributes.CAN_START_RAID` at the trigger position
    /// (`Raids.java`).
    pub(super) can_start_raid: Option<bool>,
    /// The positions of the occupied `PoiTypeTags.VILLAGE` records within
    /// [`RAID_TRIGGER_POI_RADIUS`] (`Raids.java:116`).
    pub(super) village_poi: Option<Vec<BlockPos>>,
    /// `ServerLevel.getDifficulty` (`Raids.java`).
    pub(super) difficulty: Difficulty,
    /// The amplifier of the session's `MobEffects.RAID_OMEN`, `None` for a
    /// session holding none (`Raid.java`).
    pub(super) raid_omen_amplifier: Option<i32>,
}

/// What one `createOrExtendRaid` left for its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RaidTriggerOutcome {
    /// A gate refused, which is the `null` vanilla returns.
    NoRaid,
    /// A fact this level could not answer, named for the console.
    Unanswered(&'static str),
    /// The raid this trigger created or extended, and whether `absorbRaidOmen`
    /// reached its pre-first-wave award arm (`Raid.java`).
    Raid { id: i32, award: bool },
}

/// One entry of `Raid.heroesOfTheVillage` (`Raid.java`) at the moment its
/// raid is won, carrying the omen level the effect it is owed reads .
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RaidHero {
    pub(super) uuid: Uuid,
    pub(super) raid_omen_level: i32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct LevelRaids {
    saved: Java26Raids,
    runtime: BTreeMap<i32, RaidRuntime>,
}

impl LevelRaids {
    pub(super) fn from_saved(saved: Java26Raids) -> Self {
        Self {
            runtime: saved
                .raids
                .keys()
                .map(|&id| (id, RaidRuntime::default()))
                .collect(),
            saved,
        }
    }

    pub(super) fn to_saved(&self) -> Java26Raids {
        self.saved.clone()
    }

    /// `Raids.getRaidCentersInChunk` (`Raids.java`): every raid whose
    /// centre this column contains, raided or not.
    pub(super) fn raid_centers_in_chunk(&self, chunk: (i32, i32)) -> Vec<BlockPos> {
        self.saved
            .raids
            .values()
            .map(|raid| raid.center)
            // `ChunkPos.contains` (`ChunkPos.java`).
            .filter(|center| (center.x >> 4, center.z >> 4) == chunk)
            .collect()
    }

    /// `Raids.get` (`Raids.java`) as the load-time predicate
    /// `Raider.readAdditionalSaveData` resolves `RaidId` with
    /// (`Raider.java`).
    pub(super) fn contains(&self, id: i32) -> bool {
        self.saved.raids.contains_key(&id)
    }

    /// `Raid.addWaveMob(level, wave, raider, updateHealth)` (`Raid.java`)
    /// for a raider that already exists, so no health is added
    /// (`Raider.java`). The set is keyed by UUID, which is what vanilla's
    /// `existingCopy` scan makes it.
    pub(super) fn add_wave_mob(&mut self, id: i32, wave: i32, raider: Uuid) -> bool {
        let Some(runtime) = self.runtime.get_mut(&id) else {
            return false;
        };
        runtime
            .raiders
            .entry(wave)
            .or_default()
            .entry(raider)
            .or_insert(0.0);
        true
    }

    /// `Raid.setLeader` (`Raid.java`); the ominous banner it also
    /// equips belongs to the raider owner, not to this map.
    pub(super) fn set_leader(&mut self, id: i32, wave: i32, raider: Uuid) -> bool {
        let Some(runtime) = self.runtime.get_mut(&id) else {
            return false;
        };
        runtime.leaders.insert(wave, raider);
        true
    }

    /// `Raid.getLeader` (`Raid.java`).
    pub(super) fn leader(&self, id: i32, wave: i32) -> Option<Uuid> {
        self.runtime
            .get(&id)
            .and_then(|runtime| runtime.leaders.get(&wave))
            .copied()
    }

    /// `Raid.addHeroOfTheVillage` (Raid.java:807-809).
    pub(super) fn add_hero(&mut self, id: i32, hero: Uuid) {
        if let Some(raid) = self.saved.raids.get_mut(&id) {
            raid.heroes_of_the_village.insert(hero);
        }
    }

    // Raid.java:164-165; Raid.java:184-186; Raid.java:781-782.
    pub(super) fn raider_raid(&self, id: i32) -> Option<RaiderRaidFacts> {
        let raid = self.saved.raids.get(&id)?;
        Some(RaiderRaidFacts {
            center: raid.center,
            active: raid.active,
            is_over: matches!(
                raid.status,
                Java26RaidStatus::Victory | Java26RaidStatus::Loss
            ),
            is_loss: raid.status == Java26RaidStatus::Loss,
        })
    }

    pub(super) fn raider_wave(&self, id: i32, wave: i32) -> Option<RaiderWaveFacts> {
        let raid = self.raider_raid(id)?;
        Some(RaiderWaveFacts {
            active: raid.active,
            is_over: raid.is_over,
            leader: self.leader(id, wave),
        })
    }

    /// `Raider.hurtServer` refreshes an active raid's bar (`Raider.java:252-257`).
    pub(super) fn raider_hurt(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
    ) -> RaidTickOutcome {
        let mut outcome = RaidTickOutcome::default();
        if self.saved.raids.get(&id).is_some_and(|raid| raid.active)
            && self.runtime.contains_key(&id)
        {
            self.update_bossbar(id, level, &mut outcome.boss_packets);
        }
        outcome
    }

    /// `Raid.joinRaid` for an existing body (`Raid.java`).
    pub(super) fn recruit_raider(
        &mut self,
        pending: PendingRaiderRecruit,
        level: &mut RaidLevelRuntime<'_>,
    ) -> RaidTickOutcome {
        let mut outcome = RaidTickOutcome::default();
        let id = pending.raid_id;
        let Some(raid) = self.saved.raids.get(&id).filter(|raid| raid.active) else {
            return outcome;
        };
        if !self.runtime.contains_key(&id) {
            return outcome;
        }
        let wave = raid.groups_spawned;
        let Some((uuid, health)) = level.recruit_raider(&pending, wave, self) else {
            return outcome;
        };
        let runtime = self.runtime.get_mut(&id).expect("validated raid runtime");
        let added = runtime
            .raiders
            .entry(wave)
            .or_default()
            .insert(uuid, health)
            .is_none();
        runtime.ticks_outside_raid.remove(&uuid);
        if added {
            self.saved
                .raids
                .get_mut(&id)
                .expect("validated raid row")
                .total_health += health;
        }
        self.update_bossbar(id, level, &mut outcome.boss_packets);
        outcome
    }

    /// `ServerLevel.getRaidAt` (`ServerLevel.java`) narrowed to the
    /// omen level `BadOmenMobEffect` compares against the maximum
    /// (`BadOmenMobEffect.java`).
    pub(super) fn raid_omen_level_at(&self, pos: BlockPos) -> Option<i32> {
        nearby_raid(&self.saved, pos).map(|id| self.saved.raids[&id].raid_omen_level)
    }

    /// `ServerLevel.getRaidAt(BlockPos) != null` (`ServerLevel.java`):
    /// whether an active raid's radius holds this position, which is the second
    /// disjunct of `Raider.hasRaid` (`Raider.java`).
    pub(super) fn raid_at(&self, pos: BlockPos) -> bool {
        self.raid_id_at(pos).is_some()
    }

    pub(crate) fn raid_id_at(&self, pos: BlockPos) -> Option<i32> {
        nearby_raid(&self.saved, pos)
    }

    pub(super) fn villager_activity_at(
        &self,
        pos: BlockPos,
    ) -> Option<crate::entity::ai::Activity> {
        use crate::entity::ai::Activity;
        let id = nearby_raid(&self.saved, pos)?;
        let raid = &self.saved.raids[&id];
        if raid.status == Java26RaidStatus::Victory {
            return Some(Activity::CELEBRATE);
        }
        if raid.status != Java26RaidStatus::Ongoing {
            return None;
        }
        // `SetRaidStatus.create` (`SetRaidStatus.java:17-25`).
        let gathering = raid.groups_spawned <= 0
            || (raid.cooldown_ticks > 0 && self.total_raiders_alive(id) == 0);
        Some(if gathering {
            Activity::PRE_RAID
        } else {
            Activity::RAID
        })
    }

    /// `Raid.getTotalRaidersAlive` (`Raid.java`).
    pub(super) fn total_raiders_alive(&self, id: i32) -> usize {
        self.runtime.get(&id).map_or(0, |runtime| {
            runtime.raiders.values().map(BTreeMap::len).sum()
        })
    }

    #[cfg(test)]
    pub(super) fn row(&self, id: i32) -> Option<&Java26Raid> {
        self.saved.raids.get(&id)
    }

    /// `Raids.createOrExtendRaid` (`Raids.java`) over `getOrCreateRaid`
    ///  and `Raid.absorbRaidOmen` (`Raid.java`), in the
    /// order vanilla reads its gates.
    pub(super) fn create_or_extend_raid(
        &mut self,
        position: BlockPos,
        trigger: &RaidTrigger,
    ) -> RaidTriggerOutcome {
        if trigger.spectator || !trigger.raids_rule {
            return RaidTriggerOutcome::NoRaid;
        }
        match trigger.can_start_raid {
            Some(true) => {}
            Some(false) => return RaidTriggerOutcome::NoRaid,
            None => return RaidTriggerOutcome::Unanswered("minecraft:gameplay/can_start_raid"),
        }
        let Some(village) = trigger.village_poi.as_deref() else {
            return RaidTriggerOutcome::Unanswered("occupied minecraft:village points of interest");
        };
        let center = village_center(village).unwrap_or(position);
        let id = self.get_or_create_raid(center, trigger.difficulty);
        let raid = &self.saved.raids[&id];
        let award = if !raid.started || raid.raid_omen_level < MAX_RAID_OMEN_LEVEL {
            self.absorb_raid_omen(id, trigger.raid_omen_amplifier)
        } else {
            false
        };
        RaidTriggerOutcome::Raid { id, award }
    }

    /// `Raids.getOrCreateRaid` (`Raids.java`) over
    /// `ServerLevel.getRaidAt` (`ServerLevel.java`), carrying the
    /// insert its caller guards on not-started and not-already-present
    /// (`Raids.java`): a raid the map already holds fails the second
    /// test, and a fresh one passes both.
    fn get_or_create_raid(&mut self, center: BlockPos, difficulty: Difficulty) -> i32 {
        if let Some(id) = nearby_raid(&self.saved, center) {
            return id;
        }
        // `Raids.getUniqueId` (`Raids.java`) pre-increments, so the
        // first raid of a fresh manager is id two.
        self.saved.next_id = self.saved.next_id.wrapping_add(1);
        let id = self.saved.next_id;
        self.saved.raids.insert(id, new_raid(center, difficulty));
        self.runtime.insert(
            id,
            RaidRuntime {
                // `Raid.java`, which the codec constructor does not run.
                boss: {
                    let mut boss = raid_boss_event();
                    boss.set_progress(0.0, &mut Vec::new());
                    boss
                },
                ..RaidRuntime::default()
            },
        );
        id
    }

    /// `Raid.absorbRaidOmen` (`Raid.java`). The answer is its
    /// pre-first-wave arm  rather than the boolean
    /// `createOrExtendRaid` discards; a session carrying no raid omen absorbs
    /// nothing and is owed nothing.
    fn absorb_raid_omen(&mut self, id: i32, amplifier: Option<i32>) -> bool {
        let Some(amplifier) = amplifier else {
            return false;
        };
        let raid = self.saved.raids.get_mut(&id).expect("raid row");
        raid.raid_omen_level = raid
            .raid_omen_level
            .wrapping_add(amplifier)
            .wrapping_add(1)
            .clamp(0, MAX_RAID_OMEN_LEVEL);
        // `Raid.hasFirstWaveSpawned` (`Raid.java`).
        raid.groups_spawned == 0
    }

    /// `Raid.stop` (`Raid.java`).
    fn stop(&mut self, id: i32, packets: &mut BossPackets) {
        if let Some(raid) = self.saved.raids.get_mut(&id) {
            raid.active = false;
            raid.status = Java26RaidStatus::Stopped;
        }
        if let Some(runtime) = self.runtime.get_mut(&id) {
            runtime.boss.remove_all_players(packets);
        }
    }

    fn boss(&mut self, id: i32) -> &mut ServerBossEvent {
        &mut self.runtime.entry(id).or_default().boss
    }

    /// `Raid.updatePlayers` (`Raid.java`) over `validPlayer`
    /// , whose `ServerLevel.getRaidAt` is the nearest active raid
    /// within `VALID_RAID_RADIUS_SQR` (`ServerLevel.java`).
    fn update_players(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        packets: &mut BossPackets,
    ) {
        let Self { saved, runtime } = self;
        let Some(runtime) = runtime.get_mut(&id) else {
            return;
        };
        let joined: BTreeSet<ConnectionId> = level
            .players()
            .iter()
            .filter(|player| player.alive && nearby_raid(saved, player.position) == Some(id))
            .map(|player| player.connection)
            .collect();
        runtime.boss.set_players(&joined, packets);
    }

    /// `Raid.updateRaiders` (`Raid.java`): every raider that left this
    /// level, the raid's removal radius or its patience is dropped from the
    /// wave it joined, including its matching wave leader slot.
    /// `false` when the level could not answer a village query the scan
    /// reads, which refuses before the scan writes anything.
    fn update_raiders(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) -> bool {
        let center = self.saved.raids[&id].center;
        let Some(runtime) = self.runtime.get_mut(&id) else {
            return true;
        };
        let waves: Vec<(i32, Uuid)> = runtime
            .raiders
            .iter()
            .flat_map(|(&wave, raiders)| raiders.keys().map(move |&uuid| (wave, uuid)))
            .collect();
        let mut outside = Vec::new();
        let mut removals = Vec::new();
        for (wave, uuid) in waves {
            let Some(raider) = level.raider(uuid) else {
                removals.push((wave, uuid));
                continue;
            };
            if raider.removed || dist_sqr(center, raider.position) >= RAID_REMOVAL_THRESHOLD_SQR {
                removals.push((wave, uuid));
                continue;
            }
            // `Raid.java`: only a raider past its first six hundred ticks
            // is asked to leave for standing outside the village.
            if raider.tick_count <= 600 {
                continue;
            }
            let mut ticks = runtime
                .ticks_outside_raid
                .get(&uuid)
                .copied()
                .unwrap_or_default();
            let Some(village) = level.is_village(raider.position) else {
                return false;
            };
            if !village && raider.no_action_time > MAX_NO_ACTION_TIME {
                ticks += 1;
                outside.push((uuid, ticks));
            }
            if ticks >= OUTSIDE_RAID_BOUNDS_TIMEOUT {
                removals.push((wave, uuid));
            }
        }
        for (uuid, ticks) in outside {
            runtime.ticks_outside_raid.insert(uuid, ticks);
        }
        for (wave, uuid) in removals {
            self.remove_from_raid(id, wave, uuid, true, level, outcome);
        }
        true
    }

    /// `Raider.die` (`Raider.java:115-133`) leaves the raid at once and keeps
    /// the raid's total health; every other raider refreshes the health an
    /// unload would take off it.
    fn drop_dead_raiders(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) {
        let Some(runtime) = self.runtime.get_mut(&id) else {
            return;
        };
        let mut dead = Vec::new();
        for (&wave, raiders) in &mut runtime.raiders {
            for (&uuid, health) in raiders.iter_mut() {
                match level.raider(uuid) {
                    Some(raider) if raider.health <= 0.0 => dead.push((wave, uuid)),
                    Some(raider) => *health = raider.health,
                    None => {}
                }
            }
        }
        for (wave, uuid) in dead {
            self.remove_from_raid(id, wave, uuid, false, level, outcome);
        }
    }

    /// `Raid.removeFromRaid` (`Raid.java:612-626`), whose
    /// `setCurrentRaid(null)` the caller applies for the raider.
    fn remove_from_raid(
        &mut self,
        id: i32,
        wave: i32,
        raider: Uuid,
        remove_from_total_health: bool,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) {
        let Some(runtime) = self.runtime.get_mut(&id) else {
            return;
        };
        let Some(last_health) = runtime
            .raiders
            .get_mut(&wave)
            .and_then(|raiders| raiders.remove(&raider))
        else {
            return;
        };
        // Raider.java:115-128. A removed captain must relinquish its wave's leader slot.
        if runtime.leaders.get(&wave) == Some(&raider) {
            runtime.leaders.remove(&wave);
        }
        runtime.ticks_outside_raid.remove(&raider);
        if remove_from_total_health {
            let health = level
                .raider(raider)
                .map_or(last_health, |raider| raider.health);
            let raid = self.saved.raids.get_mut(&id).expect("raid row");
            raid.total_health -= health;
        }
        outcome.unbound.insert(raider);
        self.update_bossbar(id, level, &mut outcome.boss_packets);
    }

    /// `Raid.updateBossbar` (`Raid.java`) over
    /// `getHealthOfLivingRaiders` .
    fn update_bossbar(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        packets: &mut BossPackets,
    ) {
        let health: f32 = self.runtime.get(&id).map_or(0.0, |runtime| {
            runtime
                .raiders
                .values()
                .flatten()
                .map(|(&uuid, &last)| level.raider(uuid).map_or(last, |raider| raider.health))
                .sum()
        });
        let total = self.saved.raids[&id].total_health;
        let progress = clamp_unit(health / total);
        self.boss(id).set_progress(progress, packets);
    }

    /// `Raids.tick` (`Raids.java`): the counter, the game-rule stop, the
    /// removal of an already stopped raid, and then each surviving raid's own
    /// tick. A raid that stops inside its own tick is removed by the next pass,
    /// which is where vanilla's iterator finds it too.
    pub(super) fn tick(
        &mut self,
        raids_rule: bool,
        level: &mut RaidLevelRuntime<'_>,
    ) -> RaidTickOutcome {
        self.saved.tick = self.saved.tick.wrapping_add(1);
        let mut outcome = RaidTickOutcome::default();
        for id in self.saved.raids.keys().copied().collect::<Vec<_>>() {
            if !raids_rule {
                self.stop(id, &mut outcome.boss_packets);
            }
            if self.saved.raids[&id].status == Java26RaidStatus::Stopped {
                self.saved.raids.remove(&id);
                if let Some(runtime) = self.runtime.remove(&id) {
                    outcome
                        .unbound
                        .extend(runtime.raiders.values().flat_map(BTreeMap::keys).copied());
                }
                continue;
            }
            self.tick_raid(id, level, &mut outcome);
        }
        outcome
    }

    /// `Raid.tick` (`Raid.java`).
    fn tick_raid(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) {
        self.drop_dead_raiders(id, level, outcome);
        let raid = &self.saved.raids[&id];
        match raid.status {
            Java26RaidStatus::Ongoing => {
                let center = raid.center;
                let was_active = raid.active;
                let active = level.has_chunk_at(center);
                self.saved.raids.get_mut(&id).expect("raid row").active = active;
                if level.difficulty() == Difficulty::Peaceful {
                    self.stop(id, &mut outcome.boss_packets);
                    return;
                }
                if was_active != active {
                    self.boss(id).set_visible(active, &mut outcome.boss_packets);
                }
                if !active {
                    return;
                }
                match self.settle_center(id, level) {
                    Some(true) => {}
                    Some(false) => {
                        // `Raid.java`: a raid that has already spawned a
                        // wave is lost, one that has not simply stops.
                        let raid = self.saved.raids.get_mut(&id).expect("raid row");
                        if raid.groups_spawned > 0 {
                            raid.status = Java26RaidStatus::Loss;
                        } else {
                            self.stop(id, &mut outcome.boss_packets);
                        }
                    }
                    None => {
                        outcome.unanswered_village.push(id);
                        return;
                    }
                }
                let raid = self.saved.raids.get_mut(&id).expect("raid row");
                raid.ticks_active = raid.ticks_active.saturating_add(1);
                let ticks_active = raid.ticks_active;
                if ticks_active >= RAID_TIMEOUT_TICKS {
                    self.stop(id, &mut outcome.boss_packets);
                    return;
                }
                // `Raid.java`, taken before the between-wave cooldown arm
                // and kept by every arm below.
                let alive = self.total_raiders_alive(id);
                if alive == 0 && has_more_waves(&self.saved.raids[&id]) {
                    let cooldown = self.saved.raids[&id].cooldown_ticks;
                    if cooldown <= 0 {
                        // `Raid.java`: the wave that just died opens the
                        // next cooldown and gives the bar its plain name back.
                        if cooldown == 0 && self.saved.raids[&id].groups_spawned > 0 {
                            self.saved
                                .raids
                                .get_mut(&id)
                                .expect("raid row")
                                .cooldown_ticks = DEFAULT_PRE_RAID_TICKS;
                            self.boss(id)
                                .set_name(raid_name(), &mut outcome.boss_packets);
                            return;
                        }
                    } else {
                        if !self.settle_wave_spawn_pos(id, level, cooldown) {
                            outcome.unanswered_wave_spawn.push(id);
                            return;
                        }
                        if cooldown == DEFAULT_PRE_RAID_TICKS || cooldown % 20 == 0 {
                            self.update_players(id, level, &mut outcome.boss_packets);
                        }
                        let raid = self.saved.raids.get_mut(&id).expect("raid row");
                        raid.cooldown_ticks -= 1;
                        let elapsed = DEFAULT_PRE_RAID_TICKS - raid.cooldown_ticks;
                        let progress = clamp_unit(elapsed as f32 / DEFAULT_PRE_RAID_TICKS as f32);
                        self.boss(id)
                            .set_progress(progress, &mut outcome.boss_packets);
                    }
                }
                if ticks_active % 20 == 0 {
                    // `Raid.java`, whose raider count was taken at
                    // and is not retaken after `updateRaiders`.
                    self.update_players(id, level, &mut outcome.boss_packets);
                    if !self.update_raiders(id, level, outcome) {
                        outcome.unanswered_village.push(id);
                        return;
                    }
                    let name = if (1..=LOW_MOB_THRESHOLD).contains(&alive) {
                        raiders_remaining_name(i32::try_from(alive).unwrap_or(i32::MAX))
                    } else {
                        raid_name()
                    };
                    self.boss(id).set_name(name, &mut outcome.boss_packets);
                }
                if !self.spawn_waves(id, level, outcome) {
                    outcome.unanswered_wave_spawn.push(id);
                    return;
                }
                self.tick_victory(id, alive, outcome);
            }
            // `Raid.java`: an over raid celebrates, then stops.
            Java26RaidStatus::Victory | Java26RaidStatus::Loss => {
                let runtime = self.runtime.entry(id).or_default();
                runtime.celebration_ticks = runtime.celebration_ticks.saturating_add(1);
                let celebration_ticks = runtime.celebration_ticks;
                if celebration_ticks >= MAX_CELEBRATION_TICKS {
                    self.stop(id, &mut outcome.boss_packets);
                    return;
                }
                if celebration_ticks % 20 != 0 {
                    return;
                }
                self.update_players(id, level, &mut outcome.boss_packets);
                let victory = self.saved.raids[&id].status == Java26RaidStatus::Victory;
                let boss = self.boss(id);
                boss.set_visible(true, &mut outcome.boss_packets);
                if victory {
                    boss.set_progress(0.0, &mut outcome.boss_packets);
                }
                boss.set_name(over_name(victory), &mut outcome.boss_packets);
            }
            Java26RaidStatus::Stopped => {}
        }
    }

    /// `Raid.java`: every group whose turn has come is spawned from the
    /// cached wave spawn position or a fresh twenty-try search, the first one
    /// blowing the horn, and a sixth search that answers nothing stops the
    /// raid. `false` when the level could not answer a query the search reads.
    fn spawn_waves(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) -> bool {
        let mut sound_played = false;
        let mut attempt = 0;
        while self.should_spawn_group(id) {
            let spawn_pos = match self.runtime.entry(id).or_default().wave_spawn_pos {
                Some(pos) => Some(pos),
                None => match self.find_random_spawn_pos(id, level, 20) {
                    Some(found) => found,
                    None => return false,
                },
            };
            if let Some(pos) = spawn_pos {
                self.saved.raids.get_mut(&id).expect("raid row").started = true;
                self.spawn_group(id, pos, level, outcome);
                if !sound_played {
                    self.play_horn(id, pos, level, outcome);
                    sound_played = true;
                }
            } else {
                attempt += 1;
            }
            if attempt > 5 {
                self.stop(id, &mut outcome.boss_packets);
                break;
            }
        }
        true
    }

    /// `Raid.shouldSpawnGroup` (`Raid.java`).
    fn should_spawn_group(&self, id: i32) -> bool {
        let raid = &self.saved.raids[&id];
        raid.cooldown_ticks == 0
            && (raid.groups_spawned < raid.group_count || self.should_spawn_bonus_group(id))
            && self.total_raiders_alive(id) == 0
    }

    /// `Raid.shouldSpawnBonusGroup` (`Raid.java`) over `isFinalWave`
    ///  and `hasBonusWave` .
    fn should_spawn_bonus_group(&self, id: i32) -> bool {
        let raid = &self.saved.raids[&id];
        raid.groups_spawned == raid.group_count
            && self.total_raiders_alive(id) == 0
            && raid.raid_omen_level > 1
    }

    /// `Raid.spawnGroup` (`Raid.java`): each wave type in turn, the
    /// first leader-capable raider taking the wave's leader slot, and every
    /// ravager carrying the rider its wave number selects.
    fn spawn_group(
        &mut self,
        id: i32,
        pos: BlockPos,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) {
        let mut leader_set = false;
        let (group_number, group_count) = {
            let raid = self.saved.raids.get_mut(&id).expect("raid row");
            raid.total_health = 0.0;
            (raid.groups_spawned + 1, raid.group_count)
        };
        let difficulty = level.difficulty();
        let bonus_group = self.should_spawn_bonus_group(id);
        for raider_type in RAIDER_TYPES {
            let Some(default) =
                default_num_spawns(&raider_type, group_count, group_number, bonus_group)
            else {
                outcome.refused_raiders.push((
                    id,
                    format!("wave {group_number} of {group_count} is outside the group table"),
                ));
                continue;
            };
            let bonus = potential_bonus_spawns(
                raider_type.entity_type_id,
                level.level_random,
                group_number,
                difficulty,
                bonus_group,
            );
            let mut ravagers_spawned = 0;
            for _ in 0..(default + bonus) {
                let leader = !leader_set && raider_type.can_be_leader;
                let Some(raider) = self.join_raid(
                    id,
                    group_number,
                    raider_type.entity_type_id,
                    pos,
                    leader,
                    None,
                    level,
                    outcome,
                ) else {
                    // `Raid.java`: a raider the level would not build
                    // ends this type's run the way a null `create` does.
                    break;
                };
                leader_set |= leader;
                if raider_type.entity_type_id != VANILLA_ENTITY_TYPE_RAVAGER_ID {
                    continue;
                }
                let rider = if group_number == num_groups(Difficulty::Normal) {
                    Some(VANILLA_ENTITY_TYPE_PILLAGER_ID)
                } else if group_number >= num_groups(Difficulty::Hard) {
                    Some(if ravagers_spawned == 0 {
                        VANILLA_ENTITY_TYPE_EVOKER_ID
                    } else {
                        VANILLA_ENTITY_TYPE_VINDICATOR_ID
                    })
                } else {
                    None
                };
                ravagers_spawned += 1;
                if let Some(rider) = rider {
                    self.join_raid(
                        id,
                        group_number,
                        rider,
                        pos,
                        false,
                        Some(raider),
                        level,
                        outcome,
                    );
                }
            }
        }
        self.runtime.entry(id).or_default().wave_spawn_pos = None;
        self.saved
            .raids
            .get_mut(&id)
            .expect("raid row")
            .groups_spawned += 1;
        self.update_bossbar(id, level, &mut outcome.boss_packets);
    }

    /// `Raid.joinRaid(level, wave, raider, pos, false)` (`Raid.java`)
    /// for a raider this wave is building, over `addWaveMob`'s health
    ///  and `setLeader`'s map . Only a raider the
    /// level published reaches either.
    #[allow(clippy::too_many_arguments)]
    fn join_raid(
        &mut self,
        id: i32,
        wave: i32,
        entity_type_id: i32,
        pos: BlockPos,
        leader: bool,
        rides: Option<Uuid>,
        level: &mut RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) -> Option<Uuid> {
        let enchant_odds = enchant_odds(self.saved.raids[&id].raid_omen_level);
        let spawned = level.spawn_raider(RaidSpawn {
            entity_type_id,
            position: pos,
            raid_id: id,
            wave,
            leader,
            enchant_odds,
            enchant_provider: raid_enchant_provider(entity_type_id, wave),
            rides,
        });
        let (uuid, health) = match spawned {
            RaidSpawnOutcome::Spawned { uuid, health } => (uuid, health),
            RaidSpawnOutcome::Refused(detail) => {
                outcome.refused_raiders.push((id, detail));
                return None;
            }
        };
        let runtime = self.runtime.entry(id).or_default();
        runtime
            .raiders
            .entry(wave)
            .or_default()
            .insert(uuid, health);
        runtime.ticks_outside_raid.remove(&uuid);
        if leader {
            runtime.leaders.insert(wave, uuid);
        }
        self.saved
            .raids
            .get_mut(&id)
            .expect("raid row")
            .total_health += health;
        self.update_bossbar(id, level, &mut outcome.boss_packets);
        Some(uuid)
    }

    /// `Raid.playSound` (`Raid.java`): one seed off the raid's own
    /// random, then one horn for each session within sixty-four blocks or
    /// already on the bar, aimed thirteen blocks from the listener toward the
    /// raid.
    fn play_horn(
        &mut self,
        id: i32,
        origin: BlockPos,
        level: &RaidLevelRuntime<'_>,
        outcome: &mut RaidTickOutcome,
    ) {
        let Some(registry_id) = vanilla_sound_event_registry_id(RAID_HORN_SOUND) else {
            outcome
                .refused_raiders
                .push((id, format!("no registry id for {RAID_HORN_SOUND}")));
            return;
        };
        let runtime = self.runtime.entry(id).or_default();
        let seed = level.level_random.next_long();
        let members = runtime.boss.players().clone();
        // `Vec3.atCenterOf` (`Vec3.java`).
        let raid_x = f64::from(origin.x) + 0.5;
        let raid_z = f64::from(origin.z) + 0.5;
        for player in level.players() {
            let dx = raid_x - player.precise.x;
            let dz = raid_z - player.precise.z;
            let distance = (dx * dx + dz * dz).sqrt();
            if distance > 64.0 && !members.contains(&player.connection) {
                continue;
            }
            let (sound_x, sound_z) = if distance > 0.0 {
                (
                    player.precise.x + 13.0 / distance * dx,
                    player.precise.z + 13.0 / distance * dz,
                )
            } else {
                (player.precise.x, player.precise.z)
            };
            outcome.sounds.push((
                player.connection,
                ClientboundPlayPacket::Sound(SoundEvent {
                    sound: SoundEventHolder::Reference { registry_id },
                    source: SoundSource::Neutral,
                    position: Vec3d {
                        x: sound_x,
                        y: player.precise.y,
                        z: sound_z,
                    },
                    volume: 64.0,
                    pitch: 1.0,
                    seed,
                }),
            ));
        }
    }

    /// `Raid.java`: a started raid whose last wave is spawned and dead
    /// counts forty ticks and is then won, which is where every hero it
    /// recorded becomes owed its awards. The status a stop or a lost centre
    /// left this tick is no guard, so a raid can still be won out of them.
    fn tick_victory(&mut self, id: i32, alive: usize, outcome: &mut RaidTickOutcome) {
        let raid = self.saved.raids.get_mut(&id).expect("raid row");
        if !raid.started || has_more_waves(raid) || alive != 0 {
            return;
        }
        if raid.post_raid_ticks < POST_RAID_TICK_LIMIT {
            raid.post_raid_ticks = raid.post_raid_ticks.saturating_add(1);
            return;
        }
        raid.status = Java26RaidStatus::Victory;
        let raid_omen_level = raid.raid_omen_level;
        outcome
            .heroes
            .extend(raid.heroes_of_the_village.iter().map(|&uuid| RaidHero {
                uuid,
                raid_omen_level,
            }));
    }

    /// `Raid.java`: the cached wave spawn position, searched again as
    /// soon as it leaves the entity-ticking window and otherwise every fifth
    /// cooldown tick until one answers. `false` when the level could not
    /// answer a query the search reads.
    fn settle_wave_spawn_pos(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        cooldown: i32,
    ) -> bool {
        let cached = self.runtime.entry(id).or_default().wave_spawn_pos;
        let stale = cached.is_some_and(|pos| !level.is_position_entity_ticking(pos));
        if !(stale || (cached.is_none() && cooldown % 5 == 0)) {
            return true;
        }
        // `Raid.getValidSpawnPos` (`Raid.java`).
        let Some(found) = self.find_random_spawn_pos(id, level, 8) else {
            return false;
        };
        self.runtime.entry(id).or_default().wave_spawn_pos = found;
        true
    }

    /// `Raid.findRandomSpawnPos` (`Raid.java`): a ring of candidates
    /// widening with the cooldown left, each taken at the world surface.
    /// `None` when the level could not answer a height or village query.
    fn find_random_spawn_pos(
        &mut self,
        id: i32,
        level: &mut RaidLevelRuntime<'_>,
        max_tries: i32,
    ) -> Option<Option<BlockPos>> {
        let (center, seconds_remaining) = {
            let raid = &self.saved.raids[&id];
            (raid.center, raid.cooldown_ticks / 20)
        };
        let how_far = 0.22 * seconds_remaining as f32 - 0.24;
        let step = mth_floor(how_far);
        let random = level.level_random;
        let start_angle = random.next_float() * std::f32::consts::TAU;
        for attempt in 0..max_tries {
            let angle = start_angle + std::f32::consts::PI * attempt as f32 / 8.0;
            let spawn_x = center
                .x
                .wrapping_add(mth_floor(
                    mth_cos(f64::from(angle)) * VILLAGE_SEARCH_RADIUS * how_far,
                ))
                .wrapping_add(random.next_int(3).wrapping_mul(step));
            let spawn_z = center
                .z
                .wrapping_add(mth_floor(
                    mth_sin(f64::from(angle)) * VILLAGE_SEARCH_RADIUS * how_far,
                ))
                .wrapping_add(random.next_int(3).wrapping_mul(step));
            let spawn_y = level.world_surface_height(spawn_x, spawn_z)?;
            if spawn_y.abs_diff(center.y) > VALID_RAID_RADIUS {
                continue;
            }
            let pos = BlockPos {
                x: spawn_x,
                y: spawn_y,
                z: spawn_z,
            };
            if level.is_village(pos)?
                && seconds_remaining > ALLOW_SPAWNING_WITHIN_VILLAGE_SECONDS_THRESHOLD
            {
                continue;
            }
            let delta = 10;
            if has_chunks_at(
                level,
                pos.x - delta,
                pos.z - delta,
                pos.x + delta,
                pos.z + delta,
            ) && level.is_position_entity_ticking(pos)
                && level.wave_spawn_block_ok(pos)
            {
                return Some(Some(pos));
            }
        }
        Some(None)
    }

    /// `Raid.java` and `moveRaidCenterToNearbyVillageSection`
    /// (`Raid.java`): a centre that is not a village moves to the
    /// nearest village section centre within two sections. Answers whether the
    /// settled centre is a village, or `None` when the level could not tell.
    fn settle_center(&mut self, id: i32, level: &mut RaidLevelRuntime<'_>) -> Option<bool> {
        let center = self.saved.raids[&id].center;
        if level.is_village(center)? {
            return Some(true);
        }
        // `SectionPos.cube` walks x fastest, then y, then z (`Cursor3D.java`),
        // and `Stream.min` keeps the first of equal distances.
        let section = [center.x >> 4, center.y >> 4, center.z >> 4];
        let radius = SECTION_RADIUS_FOR_FINDING_NEW_VILLAGE_CENTER;
        let mut best: Option<(f64, BlockPos)> = None;
        for section_z in (section[2] - radius)..=(section[2] + radius) {
            for section_y in (section[1] - radius)..=(section[1] + radius) {
                for section_x in (section[0] - radius)..=(section[0] + radius) {
                    let candidate = BlockPos {
                        x: (section_x << 4) + 8,
                        y: (section_y << 4) + 8,
                        z: (section_z << 4) + 8,
                    };
                    if !level.is_village(candidate)? {
                        continue;
                    }
                    let distance = dist_sqr(candidate, center);
                    if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                        best = Some((distance, candidate));
                    }
                }
            }
        }
        let Some((_, moved)) = best else {
            return Some(false);
        };
        self.saved.raids.get_mut(&id).expect("raid row").center = moved;
        level.is_village(moved)
    }
}

/// `SoundEvents.RAID_HORN` (`SoundEvents.java:1328`).
const RAID_HORN_SOUND: &str = "minecraft:event.raid.horn";

/// One row of `Raid.RaiderType` (`Raid.java:830-845`).
struct RaiderType {
    entity_type_id: i32,
    spawns_per_wave_before_bonus: [i32; 8],
    /// `PatrollingMonster.canBeLeader` (`PatrollingMonster.java`) and the
    /// two overrides that refuse it (`Witch.java`,
    /// `Ravager.java`).
    can_be_leader: bool,
}

/// `Raid.RaiderType.VALUES` in the order `spawnGroup` walks it
/// (`Raid.java`).
const RAIDER_TYPES: [RaiderType; 5] = [
    RaiderType {
        entity_type_id: VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        spawns_per_wave_before_bonus: [0, 0, 2, 0, 1, 4, 2, 5],
        can_be_leader: true,
    },
    RaiderType {
        entity_type_id: VANILLA_ENTITY_TYPE_EVOKER_ID,
        spawns_per_wave_before_bonus: [0, 0, 0, 0, 0, 1, 1, 2],
        can_be_leader: true,
    },
    RaiderType {
        entity_type_id: VANILLA_ENTITY_TYPE_PILLAGER_ID,
        spawns_per_wave_before_bonus: [0, 4, 3, 3, 4, 4, 4, 2],
        can_be_leader: true,
    },
    RaiderType {
        entity_type_id: VANILLA_ENTITY_TYPE_WITCH_ID,
        spawns_per_wave_before_bonus: [0, 0, 0, 0, 3, 0, 0, 1],
        can_be_leader: false,
    },
    RaiderType {
        entity_type_id: VANILLA_ENTITY_TYPE_RAVAGER_ID,
        spawns_per_wave_before_bonus: [0, 0, 0, 1, 0, 1, 0, 2],
        can_be_leader: false,
    },
];

/// `Raid.getDefaultNumSpawns` (`Raid.java`). `None` for a group count
/// or wave number the table cannot be indexed by, which vanilla reaches only
/// by throwing.
fn default_num_spawns(
    raider_type: &RaiderType,
    group_count: i32,
    wave: i32,
    bonus_wave: bool,
) -> Option<i32> {
    let index = if bonus_wave { group_count } else { wave };
    usize::try_from(index)
        .ok()
        .and_then(|index| raider_type.spawns_per_wave_before_bonus.get(index))
        .copied()
}

/// `Raid.getPotentialBonusSpawns` (`Raid.java`), whose difficulty is
/// the level's own (`DifficultyInstance.java`).
fn potential_bonus_spawns(
    entity_type_id: i32,
    random: &GameRandom,
    wave: i32,
    difficulty: Difficulty,
    bonus_wave: bool,
) -> i32 {
    let is_easy = difficulty == Difficulty::Easy;
    let bonus_spawns = match entity_type_id {
        VANILLA_ENTITY_TYPE_VINDICATOR_ID | VANILLA_ENTITY_TYPE_PILLAGER_ID => {
            if is_easy {
                random.next_int(2)
            } else if difficulty == Difficulty::Normal {
                1
            } else {
                2
            }
        }
        VANILLA_ENTITY_TYPE_WITCH_ID => {
            if is_easy || wave <= 2 || wave == 4 {
                return 0;
            }
            1
        }
        VANILLA_ENTITY_TYPE_RAVAGER_ID => i32::from(!is_easy && bonus_wave),
        // `Raid.java`: the evoker arm returns before the draw.
        _ => return 0,
    };
    if bonus_spawns > 0 {
        random.next_int(bonus_spawns + 1)
    } else {
        0
    }
}

/// `Raid.getNumGroups` (`Raid.java`).
const fn num_groups(difficulty: Difficulty) -> i32 {
    match difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 3,
        Difficulty::Normal => 5,
        Difficulty::Hard => 7,
    }
}

/// `Raid.getEnchantOdds` (`Raid.java`).
fn enchant_odds(raid_omen_level: i32) -> f32 {
    match raid_omen_level {
        2 => 0.1,
        3 => 0.25,
        4 => 0.5,
        5 => 0.75,
        _ => 0.0,
    }
}

/// The `EnchantmentProvider` one wave's `applyRaidBuffs` selects
/// (`Pillager.java`, `Vindicator.java`); every other raider
/// type selects none, and so does a pillager below wave four.
fn raid_enchant_provider(entity_type_id: i32, wave: i32) -> Option<&'static str> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_PILLAGER_ID => {
            if wave > num_groups(Difficulty::Normal) {
                Some("minecraft:raid/pillager_post_wave_5")
            } else if wave > num_groups(Difficulty::Easy) {
                Some("minecraft:raid/pillager_post_wave_3")
            } else {
                None
            }
        }
        VANILLA_ENTITY_TYPE_VINDICATOR_ID => Some(if wave > num_groups(Difficulty::Normal) {
            "minecraft:raid/vindicator_post_wave_5"
        } else {
            "minecraft:raid/vindicator"
        }),
        _ => None,
    }
}

/// `Raid.hasMoreWaves` (`Raid.java`): the omen level buys a bonus wave
///  that outlives the numbered ones ().
fn has_more_waves(raid: &Java26Raid) -> bool {
    if raid.raid_omen_level > 1 {
        raid.groups_spawned <= raid.group_count
    } else {
        raid.groups_spawned != raid.group_count
    }
}

/// `LevelReader.hasChunksAt(x0, z0, x1, z1)` (`LevelReader.java`) over
/// the same `hasChunk` the centre reads; `hasChunkAt` takes only the two
/// horizontal coordinates (`LevelReader.java`).
fn has_chunks_at(level: &RaidLevelRuntime<'_>, x0: i32, z0: i32, x1: i32, z1: i32) -> bool {
    ((x0 >> 4)..=(x1 >> 4)).all(|chunk_x| {
        ((z0 >> 4)..=(z1 >> 4)).all(|chunk_z| {
            level.has_chunk_at(BlockPos {
                x: chunk_x << 4,
                y: 0,
                z: chunk_z << 4,
            })
        })
    })
}

/// `Mth.floor(float)` (`Mth.java`).
fn mth_floor(value: f32) -> i32 {
    value.floor() as i32
}

/// `Mth.clamp(value, 0.0F, 1.0F)` (`Mth.java`), whose `Math.min` keeps
/// the NaN an empty raid's health ratio produces where `f32::min` drops it.
fn clamp_unit(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else if value.is_nan() {
        value
    } else {
        value.min(1.0)
    }
}

/// `Raids.getNearbyRaid` (`Raids.java`). Vanilla keeps the first of two
/// equal distances, which its map order decides and this one takes in id order.
fn nearby_raid(saved: &Java26Raids, pos: BlockPos) -> Option<i32> {
    let mut closest = VALID_RAID_RADIUS_SQR;
    let mut nearest = None;
    for (&id, raid) in &saved.raids {
        let distance = dist_sqr(raid.center, pos);
        if raid.active && distance < closest {
            closest = distance;
            nearest = Some(id);
        }
    }
    nearest
}

/// `Raid.<init>(center, difficulty)` (`Raid.java`), whose every other
/// field is the Java default the codec constructor would otherwise restore
/// (`Raid.java`).
fn new_raid(center: BlockPos, difficulty: Difficulty) -> Java26Raid {
    Java26Raid {
        started: false,
        active: true,
        ticks_active: 0,
        raid_omen_level: 0,
        groups_spawned: 0,
        cooldown_ticks: DEFAULT_PRE_RAID_TICKS,
        post_raid_ticks: 0,
        total_health: 0.0,
        group_count: num_groups(difficulty),
        status: Java26RaidStatus::Ongoing,
        center,
        heroes_of_the_village: BTreeSet::new(),
    }
}

/// `Raids.java`: the average of the village record positions as
/// `BlockPos.containing` floors it (`BlockPos.java`), or `None` when the
/// query found no record and the trigger position stands instead.
fn village_center(records: &[BlockPos]) -> Option<BlockPos> {
    let count = i32::try_from(records.len())
        .ok()
        .filter(|&count| count > 0)?;
    let total = records
        .iter()
        .fold((0.0_f64, 0.0_f64, 0.0_f64), |acc, pos| {
            (
                acc.0 + f64::from(pos.x),
                acc.1 + f64::from(pos.y),
                acc.2 + f64::from(pos.z),
            )
        });
    let scale = 1.0 / f64::from(count);
    Some(BlockPos {
        x: (total.0 * scale).floor() as i32,
        y: (total.1 * scale).floor() as i32,
        z: (total.2 * scale).floor() as i32,
    })
}

/// `Vec3i.distSqr` (`Vec3i.java`), which is `distToLowCornerSqr`
/// (`Vec3i.java`) between two integer positions.
fn dist_sqr(from: BlockPos, to: BlockPos) -> f64 {
    let dx = f64::from(from.x) - f64::from(to.x);
    let dy = f64::from(from.y) - f64::from(to.y);
    let dz = f64::from(from.z) - f64::from(to.z);
    dx * dx + dy * dy + dz * dz
}

impl super::ServerWorld {
    /// `Raids.createOrExtendRaid` (`Raids.java`) for one session, over
    /// the level facts it reads and the two awards `absorbRaidOmen` hands a
    /// raid before its first wave (`Raid.java`).
    pub(super) fn create_or_extend_raid(
        &mut self,
        connection_id: ConnectionId,
        position: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        let spectator = player.game_type == GameType::Spectator;
        let raid_omen_amplifier = player
            .active_effect(RAID_OMEN_EFFECT)
            .map(ActiveMobEffect::amplifier);
        let trigger = RaidTrigger {
            spectator,
            raids_rule: self.game_rules.raids,
            can_start_raid: self.can_start_raid(&dimension, position),
            village_poi: self.occupied_village_poi(&dimension, position),
            difficulty: self.admin.settings().difficulty,
            raid_omen_amplifier,
        };
        let Some(level) = self.levels.get_mut(&dimension) else {
            return;
        };
        match level.raids.create_or_extend_raid(position, &trigger) {
            RaidTriggerOutcome::NoRaid => {}
            RaidTriggerOutcome::Unanswered(fact) => {
                events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                    format!(
                        "raid trigger at {} {} {} in {}: no {fact} answer",
                        position.x,
                        position.y,
                        position.z,
                        self.level_key(dimension).resource_id()
                    ),
                )));
            }
            RaidTriggerOutcome::Raid { award, .. } => {
                if award {
                    if let Some(player) = self.levels.player_mut(connection_id) {
                        // `Stats.RAID_TRIGGER` (`Stats.java:92`).
                        player.award_stat_once(
                            &mut self.scoreboard,
                            custom_stat("minecraft:raid_trigger"),
                        );
                    }
                    // `CriteriaTriggers.RAID_OMEN` (`CriteriaTriggers.java:88`).
                    self.fire_player_trigger(connection_id, "minecraft:voluntary_exile", events);
                }
            }
        }
    }

    /// The occupied `PoiTypeTags.VILLAGE` records `createOrExtendRaid` averages
    /// (`Raids.java:116`, `PoiTypeTags.java:9`), read through the same POI
    /// query and village tag `isVillage` resolves against.
    fn occupied_village_poi(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> Option<Vec<BlockPos>> {
        let tags = self
            .levels
            .get(dimension)?
            .chunks
            .registry_tags_shared()
            .unwrap_or_default();
        let records = self.poi_in_range_id(
            dimension,
            [position.x, position.y, position.z],
            RAID_TRIGGER_POI_RADIUS,
            PoiOccupancy::IsOccupied,
            |record| {
                tags.contains(
                    "minecraft:point_of_interest_type",
                    "minecraft:village",
                    record.type_id,
                )
            },
        )?;
        Some(
            records
                .into_iter()
                .map(|record| BlockPos {
                    x: record.pos[0],
                    y: record.pos[1],
                    z: record.pos[2],
                })
                .collect(),
        )
    }

    /// `Raid.java:397-406`: `MobEffects.HERO_OF_THE_VILLAGE`, `Stats.RAID_WIN`
    /// (`Stats.java:93`) and `CriteriaTriggers.RAID_WIN`
    /// (`CriteriaTriggers.java:87`) for every hero resolved in the raid's level.
    pub(super) fn award_raid_heroes(
        &mut self,
        dimension: &LevelId,
        heroes: Vec<RaidHero>,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut stored_changed = false;
        for hero in heroes {
            let effect = ActiveMobEffect::with_visibility(
                HERO_OF_THE_VILLAGE_EFFECT,
                HERO_OF_THE_VILLAGE_DURATION,
                hero.raid_omen_level - 1,
                false,
                false,
                true,
            )
            .expect("the hero effect is a vanilla id with a positive duration");
            let session = self.levels[dimension]
                .player_order
                .iter()
                .copied()
                .find(|connection| {
                    self.levels
                        .player(*connection)
                        .is_some_and(|player| player.profile.uuid == hero.uuid)
                });
            let Some(connection_id) = session else {
                let Some(entity_id) = self
                    .level_by_id(dimension)
                    .entities
                    .entity_id_for_uuid(hero.uuid)
                else {
                    continue;
                };
                if self.add_stored_mob_effect(dimension, entity_id, effect) == Some(true) {
                    stored_changed = true;
                }
                continue;
            };
            if self.levels[connection_id].game_type == GameType::Spectator {
                continue;
            }
            let _ = self.add_player_mob_effect(connection_id, effect, events);
            if let Some(player) = self.levels.player_mut(connection_id) {
                player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:raid_win"));
            }
            self.fire_player_trigger(connection_id, "minecraft:hero_of_the_village", events);
        }
        if stored_changed {
            self.apply_level_entity_lifecycle_by_key(dimension, events);
        }
    }
}

/// `Raider.readAdditionalSaveData` (`Raider.java:191-206`): resolves saved `RaidId`
/// across the root entity and every descendant passenger, clearing `Unresolved` to
/// `None` whenever the target raid is absent from this level.
pub(in crate::world) fn resolve_raider_raids_in_tree_parts(
    entities: &mut crate::entity::EntityStore,
    raids: &mut LevelRaids,
    root_id: i32,
) {
    let Some(tree) = entities.entity_tree_ids(root_id) else {
        return;
    };
    for entity_id in tree {
        let Some(entity) = entities.get_mut(entity_id) else {
            continue;
        };
        let uuid = entity.uuid();
        if let Some(join) =
            crate::entity::resolve_raider_raid(entity, |raid_id| raids.contains(raid_id))
        {
            raids.add_wave_mob(join.raid_id, join.wave, uuid);
            if join.patrol_leader {
                raids.set_leader(join.raid_id, join.wave, uuid);
            }
        }
    }
}

impl crate::world::ServerWorld {
    pub(in crate::world) fn resolve_raider_raids_in_tree(
        &mut self,
        dimension: &crate::world::LevelId,
        root_id: i32,
    ) {
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        resolve_raider_raids_in_tree_parts(&mut level.entities, &mut level.raids, root_id);
    }
}

#[cfg(test)]
mod tests;
