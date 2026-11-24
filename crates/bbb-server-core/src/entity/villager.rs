//! `Villager`'s trading career: trade experience and level-ups, restocks and
//! the reputation discount (`Villager.java`).

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
use bbb_protocol::packets::{EntityEvent, SoundSource, Vec3d};
use uuid::Uuid;

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::{MemoryModuleType, MemoryValue};
use crate::entity::living::ActiveMobEffect;
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, GeneralLivingData};
use crate::world::simulation_level::LevelCtx;

/// `VillagerData.NEXT_LEVEL_XP_THRESHOLDS` (`VillagerData.java:17`).
const NEXT_LEVEL_XP_THRESHOLDS: [i32; 5] = [0, 10, 70, 150, 250];
/// `Mob.getAmbientSoundInterval` (`Mob.java:269-271`).
const AMBIENT_SOUND_INTERVAL: i32 = 80;
/// `Villager.rewardTradeXp`'s level-up delay (`Villager.java:573`).
const LEVEL_UP_DELAY: i32 = 40;
// Villager.java:106.
pub(crate) const TIME_SINCE_SLEEPING: i64 = 24_000;
/// `EntityEvent.VILLAGER_HAPPY`, the happy-villager particles.
const HAPPY_EVENT: i8 = 14;
/// `Villager.handleEntityEvent`'s splash of sweat (`Villager.java:272-277`).
const RAID_SWEAT_EVENT: i8 = 42;

pub(crate) const VILLAGER_NO: &str = "minecraft:entity.villager.no";

/// The world-level work a villager step leaves behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingVillagerAction {
    /// `AbstractVillager.getOffers` after a profession change dropped them
    /// (`AbstractVillager.java:112-123`), done at once so the saved record
    /// always carries the profession's offers.
    GenerateOffers { entity_id: i32 },
    /// `Villager.increaseMerchantCareer` (`Villager.java:696-699`).
    IncreaseCareer { entity_id: i32 },
    /// `Villager.resendOffersToTradingPlayer` (`Villager.java:386-394`).
    ResendOffers { entity_id: i32 },
    /// `Villager.spawnGolemIfNeeded(level, timestamp, needed)`.
    SpawnGolemIfNeeded { entity_id: i32, needed: usize },
    ReleasePoi {
        dimension: String,
        pos: [i32; 3],
        memory: MemoryModuleType,
        profession: String,
    },
}

// Villager.java:850-852; Villager.java:896-899.
pub(crate) fn wants_to_spawn_golem(store: &EntityStore, id: i32, game_time: i64) -> bool {
    store.mob_ai(id).is_some_and(|runtime| {
        matches!(runtime.brain_memory(MemoryModuleType::LastSlept),
            Some(MemoryValue::Long(slept)) if game_time - slept < TIME_SINCE_SLEEPING)
            && runtime
                .brain_memory(MemoryModuleType::GolemDetectedRecently)
                .is_none()
    })
}

pub(crate) const HOME_POI: &str = "minecraft:home";
pub(crate) const MEETING_POI: &str = "minecraft:meeting";

/// `Villager.POI_MEMORIES` (`Villager.java:171-180`): the POI types a memory
/// may hold, which `Villager.releasePoi` gives a ticket back for.
pub(crate) fn memory_holds_poi(
    memory: MemoryModuleType,
    profession: &str,
    poi_type: &str,
    acquirable_job_site: impl Fn(&str) -> bool,
) -> bool {
    match memory {
        MemoryModuleType::Home => poi_type == HOME_POI,
        MemoryModuleType::MeetingPoint => poi_type == MEETING_POI,
        MemoryModuleType::JobSite => {
            !matches!(profession, "minecraft:none" | "minecraft:nitwit") && profession == poi_type
        }
        MemoryModuleType::PotentialJobSite => acquirable_job_site(poi_type),
        _ => false,
    }
}

/// `Villager.releaseAllPois` (`Villager.java:607-612`). Memories are erased so
/// death and conversion cannot return the same ticket twice.
pub(crate) fn release_all_pois(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if !is_villager(entity) {
        return;
    }
    let profession = general(entity)
        .and_then(|data| data.villager_profession_and_level())
        .map_or("minecraft:none", |(profession, _)| profession);
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    if !runtime.ensure_brain(entity) {
        return;
    }
    let mut remote_pois = Vec::new();
    for memory in [
        MemoryModuleType::Home,
        MemoryModuleType::JobSite,
        MemoryModuleType::PotentialJobSite,
        MemoryModuleType::MeetingPoint,
    ] {
        let Some(MemoryValue::GlobalPos(site)) = runtime.brain_memory(memory).cloned() else {
            continue;
        };
        runtime.erase_brain_memory(memory);
        let pos = [site.pos.x, site.pos.y, site.pos.z];
        if site.dimension == level.dimension_key().as_str() {
            release_poi(memory, profession, pos, level);
        } else {
            remote_pois.push(PendingVillagerAction::ReleasePoi {
                dimension: site.dimension,
                pos,
                memory,
                profession: profession.to_owned(),
            });
        }
    }
    for action in remote_pois {
        store.queue_command(EntityCommand::Villager(action));
    }
}

pub(crate) fn release_poi(
    memory: MemoryModuleType,
    profession: &str,
    pos: [i32; 3],
    level: &mut LevelCtx<'_, '_>,
) {
    let Some(poi_type) = level.poi_mut().record(pos).map(|record| record.type_id) else {
        return;
    };
    let holds = memory_holds_poi(memory, profession, poi_type, |poi_type| {
        level.registry_tag_membership(
            "minecraft:point_of_interest_type",
            "minecraft:acquirable_job_site",
            poi_type,
        )
    });
    if holds {
        level.poi_mut().release_ticket(pos);
    }
}

/// The villager's own per-tick work that runs dead or alive.
pub(crate) fn tick(entity: &mut Entity, level: &mut LevelCtx<'_, '_>) {
    if !is_villager(entity) {
        return;
    }
    crate::entity::villager_life::tick(entity, level.game_time());
}

pub(crate) fn is_villager(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
}

/// `VillagerData.getMinXpPerLevel`/`getMaxXpPerLevel` gate
/// (`VillagerData.java:68-78`).
fn should_increase_level(level: i32, xp: i32) -> bool {
    (1..5).contains(&level) && xp >= NEXT_LEVEL_XP_THRESHOLDS[level as usize]
}

fn general(entity: &Entity) -> Option<&crate::entity::GeneralLivingData> {
    match entity.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn general_mut(entity: &mut Entity) -> Option<&mut crate::entity::GeneralLivingData> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn is_baby(entity: &Entity) -> bool {
    general(entity).is_some_and(|data| data.ageable_retained().is_some_and(|age| age.is_baby()))
}

/// `Villager.getVillagerXp`.
pub fn villager_xp(entity: &Entity) -> Option<i32> {
    Some(general(entity)?.villager_work()?.xp)
}

/// `Villager.getPlayerReputation` (`Villager.java:674-676`) over
/// `GossipContainer.getReputation` (`GossipContainer.java:108-111`) and the
/// `GossipType` weights (`GossipType.java:7-11`).
pub(crate) fn player_reputation(entity: &Entity, player: Uuid) -> i32 {
    let Some(gossips) = general(entity).and_then(GeneralLivingData::villager_gossips) else {
        return 0;
    };
    crate::entity::general_living::gossip_reputation(gossips, player)
}

/// `Villager.updateSpecialPrices` (`Villager.java:451-469`).
pub(crate) fn update_special_prices(
    entity: &mut Entity,
    reputation: i32,
    hero_amplifier: Option<i32>,
) {
    let Some(offers) = entity.merchant_offers_mut() else {
        return;
    };
    for offer in offers.offers_mut() {
        if reputation != 0 {
            let diff = (reputation as f32 * offer.price_multiplier()).floor() as i32;
            offer.add_to_special_price_diff(diff.wrapping_neg());
        }
        if let Some(amplifier) = hero_amplifier {
            let modifier = 0.3 + 0.0625 * f64::from(amplifier);
            let reduction = (modifier * f64::from(offer.base_cost_a().count())).floor() as i32;
            offer.add_to_special_price_diff(-reduction.max(1));
        }
    }
}

/// `Villager.resetSpecialPrices` (`Villager.java:356-362`), which
/// `stopTrading` runs when the trading player leaves.
pub(crate) fn reset_special_prices(entity: &mut Entity) {
    if !is_villager(entity) {
        return;
    }
    if let Some(offers) = entity.merchant_offers_mut() {
        offers
            .offers_mut()
            .iter_mut()
            .for_each(crate::entity::MerchantOffer::reset_special_price_diff);
    }
}

/// `Villager.shouldRestock` and `restock` (`Villager.java:374-428`), run when
/// `WorkAtPoi` starts at the job site. A new overworld day counts as half a
/// day passed. Answers whether the offers changed.
pub(crate) fn restock_if_due(entity: &mut Entity, game_time: i64, current_day: i64) -> bool {
    let Some(work) = general(entity)
        .and_then(|data| data.villager_work())
        .cloned()
    else {
        return false;
    };
    let mut changed = false;
    let mut work = work;
    let new_day = game_time > work.last_restock + 12_000
        || (work.last_restock_check_day > 0 && current_day > work.last_restock_check_day);
    work.last_restock_check_day = current_day;
    if new_day {
        work.last_restock = game_time;
        // `resetNumberOfRestocks` over `catchUpDemand` (`Villager.java:430-443`).
        let missed = 2 - work.restocks_today;
        if let Some(offers) = entity.merchant_offers_mut() {
            for offer in offers.offers_mut() {
                if missed > 0 {
                    offer.reset_uses();
                }
                for _ in 0..missed.max(0) {
                    offer.update_demand();
                }
            }
            changed = missed > 0;
        }
        work.restocks_today = 0;
    }
    let allowed = work.restocks_today == 0
        || (work.restocks_today < 2 && game_time > work.last_restock + 2_400);
    let needs = entity.merchant_offers().is_some_and(|offers| {
        offers
            .offers()
            .iter()
            .any(crate::entity::MerchantOffer::needs_restock)
    });
    if allowed && needs {
        if let Some(offers) = entity.merchant_offers_mut() {
            for offer in offers.offers_mut() {
                offer.update_demand();
                offer.reset_uses();
            }
        }
        work.last_restock = game_time;
        work.restocks_today += 1;
        changed = true;
    }
    if let Some(slot) = general_mut(entity).and_then(|data| data.villager_work_mut()) {
        *slot = work;
    }
    changed
}

/// The `customServerAiStep` half after the brain (`Villager.java:245-284`):
/// the delayed level-up and the trade's reputation event.
pub(crate) fn after_ai_step(entity: &mut Entity, store: &mut EntityStore) {
    if !is_villager(entity) {
        return;
    }
    let mut trading = entity.merchant_trading_player().is_some();
    let id = entity.id();
    // `Villager.customServerAiStep` (`Villager.java:279-281`): a villager that
    // lost its profession closes the merchant menu.
    if trading
        && general(entity)
            .and_then(|data| data.villager_profession_and_level())
            .is_some_and(|(profession, _)| profession == "minecraft:none")
    {
        entity.set_merchant_trading_player(None);
        reset_special_prices(entity);
        trading = false;
    }
    // `Villager.java:272-277`.
    if !general(entity).is_some_and(crate::entity::GeneralLivingData::is_no_ai)
        && store.random().next_int(100) == 0
        && store
            .mob_ai(id)
            .is_some_and(crate::entity::ai::mob::MobAiRuntime::villager_in_ongoing_raid)
    {
        store.broadcast_to_tracking(
            id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: id,
                event_id: RAID_SWEAT_EVENT,
            }),
        );
    }
    let Some(work) = general_mut(entity).and_then(|data| data.villager_work_mut()) else {
        return;
    };
    let mut level_up = false;
    let mut regenerate = false;
    if !trading && work.update_merchant_timer > 0 {
        work.update_merchant_timer -= 1;
        if work.update_merchant_timer <= 0 {
            level_up = std::mem::take(&mut work.increase_level_on_update);
            regenerate = true;
        }
    }
    let traded_with = work.last_traded_player.take();
    if level_up {
        store.queue_command(EntityCommand::Villager(
            PendingVillagerAction::IncreaseCareer { entity_id: id },
        ));
    }
    if regenerate {
        if let (Some(effect), Some(living)) = (
            ActiveMobEffect::new("minecraft:regeneration", 200, 0),
            entity.living_mut(),
        ) {
            living.add_active_effect(effect);
        }
    }
    if let Some(player) = traded_with {
        // `Villager.onReputationEventFrom(TRADE)` (`Villager.java:859-860`).
        crate::entity::zombie::villager::add_gossip(entity, player, "trading", 2, 25);
        store.broadcast_to_tracking(
            id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: id,
                event_id: HAPPY_EVENT,
            }),
        );
    }
}

// Villager.java:530-536.
pub(crate) fn ambient_sound(entity: &Entity) -> Option<&'static str> {
    if entity.sleeping_pos().is_some() {
        None
    } else if entity.merchant_trading_player().is_some() {
        Some("minecraft:entity.villager.trade")
    } else {
        Some("minecraft:entity.villager.ambient")
    }
}

pub(crate) fn tick_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if !is_villager(entity) {
        return;
    }
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    if !runtime.tick_mob_ambient_sound(entity.is_alive()) || entity.is_silent() {
        return;
    }
    if let Some(sound) = ambient_sound(entity) {
        play_body_sound(entity, store, sound);
    }
}

pub(crate) fn play_body_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str) {
    if !entity.is_silent() {
        store.play_villager_sound_at(entity.id(), entity.position(), is_baby(entity), resource_id);
    }
}

impl EntityStore {
    /// `Entity.makeSound` for a villager.
    pub(crate) fn play_villager_sound(&mut self, entity_id: i32, resource_id: &str) {
        let Some(entity) = self.get(entity_id) else {
            return;
        };
        if entity.is_silent() {
            return;
        }
        let position = entity.position();
        let baby = is_baby(entity);
        self.play_villager_sound_at(entity_id, position, baby, resource_id);
    }

    fn play_villager_sound_at(
        &mut self,
        entity_id: i32,
        position: bbb_protocol::packets::Vec3d,
        baby: bool,
        resource_id: &str,
    ) {
        let Some(runtime) = self.mob_ai_mut(entity_id) else {
            return;
        };
        let pitch = runtime.voice_pitch(baby);
        self.play_entity_sound(crate::entity::store::PendingEntitySound {
            position,
            resource_id: resource_id.into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }

    /// `AbstractVillager.notifyTrade` (`AbstractVillager.java:134-141`) with
    /// `Villager.rewardTradeXp` (`Villager.java:568-581`). The `TRADE`
    /// criterion belongs to the caller.
    pub fn villager_notify_trade(
        &mut self,
        entity_id: i32,
        offer: usize,
        player: Option<Uuid>,
    ) -> bool {
        let Some(entity) = self.get_mut(entity_id) else {
            return false;
        };
        if !is_villager(entity) {
            return false;
        }
        let position = entity.position();
        let Some(offer) = entity
            .merchant_offers_mut()
            .and_then(|offers| offers.offers_mut().get_mut(offer))
        else {
            return false;
        };
        offer.increase_uses();
        let (reward_exp, offer_xp) = (offer.reward_exp(), offer.xp());
        let level = general(entity)
            .and_then(|data| data.villager_profession_and_level())
            .map_or(1, |(_, level)| level);
        let Some(work) = general_mut(entity).and_then(|data| data.villager_work_mut()) else {
            return false;
        };
        work.xp = work.xp.wrapping_add(offer_xp);
        work.last_traded_player = player;
        let level_up = should_increase_level(level, work.xp);
        if level_up {
            work.update_merchant_timer = LEVEL_UP_DELAY;
            work.increase_level_on_update = true;
        }
        let Some(runtime) = self.mob_ai_mut(entity_id) else {
            return false;
        };
        runtime.set_ambient_sound_time(-AMBIENT_SOUND_INTERVAL);
        let mut xp = runtime.merchant_trade_xp();
        if level_up {
            xp += 5;
        }
        if reward_exp {
            crate::entity::experience_orb::spawn_experience_orb(
                self,
                Vec3d {
                    x: position.x,
                    y: position.y + 0.5,
                    z: position.z,
                },
                xp,
            );
        }
        true
    }

    /// `AbstractVillager.notifyTradeUpdated` (`AbstractVillager.java:151-156`).
    pub fn villager_notify_trade_updated(&mut self, entity_id: i32, valid: bool) {
        let Some(runtime) = self.mob_ai_mut(entity_id) else {
            return;
        };
        if runtime.ambient_sound_time() <= -AMBIENT_SOUND_INTERVAL + 20 {
            return;
        }
        runtime.set_ambient_sound_time(-AMBIENT_SOUND_INTERVAL);
        self.play_villager_sound(
            entity_id,
            if valid {
                "minecraft:entity.villager.yes"
            } else {
                VILLAGER_NO
            },
        );
    }
}
