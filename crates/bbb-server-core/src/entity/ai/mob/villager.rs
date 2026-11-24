//! `Villager`'s brain as its provider builds it,
//! with the job-site, work and trading-player behaviours this runtime has.

use crate::entity::ai::brain::{
    acquire_poi, assign_profession_from_job_site, do_nothing, go_to_closest_village,
    go_to_potential_job_site, harvest_farmland, inside_brownian_walk, interact_with_cat,
    interact_with_villager, jump_on_bed, locate_hiding_place, look_and_follow_trading_player,
    look_at_target_sink, move_to_sky_seeing_spot, move_to_target_sink, play_tag_with_other_kids,
    poi_competitor_scan, reset_profession, run_one, set_closest_home_as_walk_target,
    set_hidden_state, set_look_and_interact_player, set_walk_target_away_from,
    set_walk_target_from_block_memory, set_walk_target_from_look_target, show_trades_to_player,
    sleep_in_bed, socialize_at_bell, stroll_around_poi, stroll_to_poi, stroll_to_poi_list, swim,
    trade_with_villager, use_bonemeal, validate_nearby_poi, village_bound_random_stroll,
    village_bound_random_stroll_within, villager_calm_down, villager_full_look, villager_make_love,
    villager_minimal_look, villager_panic_trigger, villager_play_fallback, wake_up, work_at_poi,
    yield_job_site,
};
use crate::entity::ai::{Activity, Brain, BrainPacked, MemoryModuleType, MemoryStatus, SensorType};
use crate::entity::{Entity, EntityData, EntityStore};

use super::brain_runtime::{BrainBodyFacts, VillagerBrainFacts};

pub(crate) mod hero;
pub(crate) mod raid;
pub(super) mod wanted_item;

/// `BuiltInRegistries.VILLAGER_PROFESSION` in bootstrap order
/// indexed by registry id.
pub(crate) const PROFESSIONS: [&str; 15] = [
    "minecraft:none",
    "minecraft:armorer",
    "minecraft:butcher",
    "minecraft:cartographer",
    "minecraft:cleric",
    "minecraft:farmer",
    "minecraft:fisherman",
    "minecraft:fletcher",
    "minecraft:leatherworker",
    "minecraft:librarian",
    "minecraft:mason",
    "minecraft:nitwit",
    "minecraft:shepherd",
    "minecraft:toolsmith",
    "minecraft:weaponsmith",
];

/// `Villager.SPEED_MODIFIER` (`Villager.java:108`).
const SPEED_MODIFIER: f32 = 0.5;
/// `VillagerGoalPackages.STROLL_SPEED_MODIFIER` (`VillagerGoalPackages.java:27`).
const STROLL_SPEED_MODIFIER: f32 = 0.4;

/// Every memory the provider registers that carries a codec, so a saved
/// villager's brain survives the round trip, plus the ones the behaviours
/// here read.
const VILLAGER_MEMORY_MODULES: &[MemoryModuleType] = &[
    MemoryModuleType::Home,
    MemoryModuleType::JobSite,
    MemoryModuleType::PotentialJobSite,
    MemoryModuleType::MeetingPoint,
    MemoryModuleType::GolemDetectedRecently,
    MemoryModuleType::LastSlept,
    MemoryModuleType::LastWoken,
    MemoryModuleType::LastWorkedAtPoi,
    MemoryModuleType::NearestLivingEntities,
    MemoryModuleType::NearestVisibleLivingEntities,
    MemoryModuleType::WalkTarget,
    MemoryModuleType::NearestVisibleWantedItem,
    MemoryModuleType::ItemPickupCooldownTicks,
    MemoryModuleType::LookTarget,
    MemoryModuleType::Path,
    MemoryModuleType::CantReachWalkTargetSince,
    MemoryModuleType::HidingPlace,
    MemoryModuleType::HeardBellTime,
    MemoryModuleType::BreedTarget,
    MemoryModuleType::InteractionTarget,
];

pub(super) fn brain_facts(entity: &Entity) -> Option<VillagerBrainFacts> {
    if !crate::entity::villager::is_villager(entity) {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let (profession_id, level, assign_profession_when_spawned) = data.villager_brain_state()?;
    Some(VillagerBrainFacts {
        profession: PROFESSIONS
            .get(usize::try_from(profession_id).ok()?)
            .copied()?,
        level,
        xp: data.villager_work()?.xp,
        assign_profession_when_spawned,
        baby: data.ageable_retained().is_some_and(|age| age.is_baby()),
        trading_player: entity.merchant_trading_player(),
        scheduled: None,
        sleeping: entity.sleeping_pos().is_some(),
        can_breed: crate::entity::villager_life::can_breed(entity),
        has_bone_meal: crate::entity::villager_life::has_bone_meal(entity),
        has_farm_seeds: false,
    })
}

/// `Villager.BRAIN_PROVIDER.makeBrain` with `registerBrainGoals`
pub(super) fn villager_brain(packed: &BrainPacked) -> Brain {
    let mut brain = Brain::new();
    for memory_type in VILLAGER_MEMORY_MODULES {
        brain.register_memory(*memory_type);
    }
    brain.register_sensor(SensorType::NearestLivingEntities);
    brain.register_sensor(SensorType::NearestPlayers);
    brain.register_sensor(SensorType::HurtBy);
    brain.register_sensor(SensorType::VillagerHostiles);
    brain.register_sensor(SensorType::SecondaryPois);
    brain.register_sensor(SensorType::GolemDetected);
    // `VillagerGoalPackages.getCorePackage`.
    brain.add_activity(
        Activity::CORE,
        [
            (0, swim(0.8, false)),
            (0, look_at_target_sink()),
            (0, villager_panic_trigger()),
            (0, wake_up()),
            (0, validate_nearby_poi(MemoryModuleType::JobSite)),
            (0, validate_nearby_poi(MemoryModuleType::PotentialJobSite)),
            (1, move_to_target_sink()),
            (2, poi_competitor_scan()),
            (3, look_and_follow_trading_player(SPEED_MODIFIER)),
            (6, acquire_poi(MemoryModuleType::JobSite)),
            (7, go_to_potential_job_site(SPEED_MODIFIER)),
            (8, yield_job_site(SPEED_MODIFIER)),
            (10, acquire_poi(MemoryModuleType::Home)),
            (10, acquire_poi(MemoryModuleType::MeetingPoint)),
            (10, assign_profession_from_job_site()),
            (10, reset_profession()),
        ],
        &[],
        &[],
    );
    // `VillagerGoalPackages.getWorkPackage`.
    brain.add_activity(
        Activity::WORK,
        [
            (
                5,
                run_one(
                    Vec::new(),
                    [
                        (7, work_at_poi()),
                        (
                            2,
                            stroll_around_poi(MemoryModuleType::JobSite, STROLL_SPEED_MODIFIER, 4),
                        ),
                        (
                            5,
                            stroll_to_poi(MemoryModuleType::JobSite, STROLL_SPEED_MODIFIER, 1, 10),
                        ),
                        (
                            5,
                            stroll_to_poi_list(
                                MemoryModuleType::SecondaryJobSite,
                                SPEED_MODIFIER,
                                1,
                                6,
                                MemoryModuleType::JobSite,
                            ),
                        ),
                        (2, harvest_farmland()),
                        (4, use_bonemeal()),
                    ],
                ),
            ),
            (5, villager_minimal_look()),
            (
                2,
                set_walk_target_from_block_memory(
                    MemoryModuleType::JobSite,
                    SPEED_MODIFIER,
                    9,
                    100,
                    1200,
                ),
            ),
            (10, show_trades_to_player()),
            (10, set_look_and_interact_player(4)),
        ],
        &[(MemoryModuleType::JobSite, MemoryStatus::ValuePresent)],
        &[],
    );
    brain.add_activity(
        Activity::PLAY,
        [
            (5, villager_full_look()),
            (5, play_tag_with_other_kids()),
            (5, villager_play_fallback(SPEED_MODIFIER)),
        ],
        &[],
        &[
            MemoryModuleType::InteractionTarget,
            MemoryModuleType::WalkTarget,
            MemoryModuleType::LookTarget,
            MemoryModuleType::Path,
        ],
    );
    // `VillagerGoalPackages.getRestPackage`.
    brain.add_activity(
        Activity::REST,
        [
            (
                2,
                set_walk_target_from_block_memory(
                    MemoryModuleType::Home,
                    SPEED_MODIFIER,
                    1,
                    150,
                    1200,
                ),
            ),
            (3, validate_nearby_poi(MemoryModuleType::Home)),
            (3, sleep_in_bed()),
            (
                5,
                run_one(
                    vec![(MemoryModuleType::Home, MemoryStatus::ValueAbsent)],
                    [
                        (1, set_closest_home_as_walk_target(SPEED_MODIFIER)),
                        (4, inside_brownian_walk(SPEED_MODIFIER)),
                        (2, go_to_closest_village(SPEED_MODIFIER)),
                        (2, do_nothing(20, 40)),
                    ],
                ),
            ),
            (5, villager_minimal_look()),
        ],
        &[],
        &[],
    );
    // `VillagerGoalPackages.getMeetPackage` (`:147-173`), which needs a
    // meeting point.
    brain.add_activity(
        Activity::MEET,
        [
            (
                2,
                run_one(
                    Vec::new(),
                    [
                        (
                            2,
                            stroll_around_poi(
                                MemoryModuleType::MeetingPoint,
                                STROLL_SPEED_MODIFIER,
                                40,
                            ),
                        ),
                        (2, socialize_at_bell()),
                    ],
                ),
            ),
            (
                2,
                set_walk_target_from_block_memory(
                    MemoryModuleType::MeetingPoint,
                    SPEED_MODIFIER,
                    6,
                    100,
                    200,
                ),
            ),
            (10, show_trades_to_player()),
            (10, set_look_and_interact_player(4)),
            (3, validate_nearby_poi(MemoryModuleType::MeetingPoint)),
            (3, trade_with_villager()),
            (5, villager_full_look()),
        ],
        &[(MemoryModuleType::MeetingPoint, MemoryStatus::ValuePresent)],
        &[],
    );
    // `VillagerGoalPackages.getIdlePackage` (`:175-219`).
    brain.add_activity(
        Activity::IDLE,
        [
            (
                2,
                run_one(
                    Vec::new(),
                    [
                        (
                            2,
                            interact_with_villager(MemoryModuleType::InteractionTarget),
                        ),
                        (1, interact_with_villager(MemoryModuleType::BreedTarget)),
                        (1, interact_with_cat()),
                        (1, village_bound_random_stroll(SPEED_MODIFIER)),
                        (1, set_walk_target_from_look_target(SPEED_MODIFIER, 2)),
                        (1, jump_on_bed(SPEED_MODIFIER)),
                        (1, do_nothing(30, 60)),
                    ],
                ),
            ),
            (3, set_look_and_interact_player(4)),
            (3, show_trades_to_player()),
            (3, trade_with_villager()),
            (3, villager_make_love()),
            (5, villager_full_look()),
        ],
        &[],
        &[],
    );
    // `VillagerGoalPackages.getPanicPackage` (`:221-230`).
    let runaway_speed = SPEED_MODIFIER * 1.5;
    brain.add_activity(
        Activity::PANIC,
        [
            (0, villager_calm_down()),
            (
                1,
                set_walk_target_away_from(MemoryModuleType::NearestHostile, runaway_speed, 6),
            ),
            (
                1,
                set_walk_target_away_from(MemoryModuleType::HurtByEntity, runaway_speed, 6),
            ),
            (3, village_bound_random_stroll_within(runaway_speed, 2, 2)),
            (5, villager_minimal_look()),
        ],
        &[],
        &[],
    );
    // `VillagerGoalPackages.getHidePackage` (`:272-277`).
    brain.add_activity(
        Activity::HIDE,
        [
            (0, set_hidden_state(15, 3)),
            (1, locate_hiding_place(32, SPEED_MODIFIER * 1.25, 2)),
            (5, villager_minimal_look()),
        ],
        &[],
        &[],
    );
    // `VillagerGoalPackages.getPreRaidPackage`.
    brain.add_activity(
        Activity::PRE_RAID,
        [
            (
                0,
                run_one(
                    Vec::new(),
                    [
                        (
                            6,
                            set_walk_target_from_block_memory(
                                MemoryModuleType::MeetingPoint,
                                SPEED_MODIFIER * 1.5,
                                2,
                                150,
                                200,
                            ),
                        ),
                        (2, village_bound_random_stroll(SPEED_MODIFIER * 1.5)),
                    ],
                ),
            ),
            (5, villager_minimal_look()),
        ],
        &[],
        &[MemoryModuleType::WalkTarget, MemoryModuleType::Path],
    );
    // `VillagerGoalPackages.getRaidPackage`.
    brain.add_activity(
        Activity::RAID,
        [
            (2, locate_hiding_place(24, SPEED_MODIFIER * 1.4, 1)),
            (5, villager_minimal_look()),
        ],
        &[],
        &[
            MemoryModuleType::WalkTarget,
            MemoryModuleType::Path,
            MemoryModuleType::HidingPlace,
        ],
    );
    // `VillagerGoalPackages.getRaidPackage`.
    brain.add_activity(
        Activity::CELEBRATE,
        [
            (
                0,
                run_one(
                    Vec::new(),
                    [
                        (5, move_to_sky_seeing_spot()),
                        (2, village_bound_random_stroll(SPEED_MODIFIER * 1.1)),
                    ],
                ),
            ),
            (0, raid::celebrate_villagers_survived_raid()),
            (5, villager_minimal_look()),
        ],
        &[],
        &[MemoryModuleType::WalkTarget, MemoryModuleType::Path],
    );
    brain.set_schedule(true);
    brain.apply_packed(packed);
    brain
}

/// Immediate threats keep their escape behavior while a raid overrides the daily schedule.
pub(super) fn update_activity(
    brain: &mut Brain,
    game_time: i64,
    body: &BrainBodyFacts,
    raid_activity: Option<Activity>,
) {
    let was_raid = brain.is_active(Activity::PRE_RAID)
        || brain.is_active(Activity::RAID)
        || brain.is_active(Activity::CELEBRATE);
    if raid_activity.is_some() || was_raid {
        brain.erase_memory(MemoryModuleType::HeardBellTime);
    }
    if brain.has_memory_value(MemoryModuleType::HurtBy)
        || brain.has_memory_value(MemoryModuleType::NearestHostile)
    {
        if !brain.is_active(Activity::PANIC) {
            forget_destination(brain);
        }
        brain.set_active_activity_if_possible(Activity::PANIC);
        return;
    }
    if brain.is_active(Activity::PANIC) && brain.has_memory_value(MemoryModuleType::HurtByEntity) {
        return;
    }
    if let Some(activity) = raid_activity {
        if !brain.is_active(activity) {
            forget_destination(brain);
            brain.set_active_activity_if_possible(activity);
        }
        return;
    }
    if brain.has_memory_value(MemoryModuleType::HeardBellTime) {
        brain.set_active_activity_if_possible(Activity::HIDE);
        return;
    }
    let Some(scheduled) = body.villager.and_then(|villager| villager.scheduled) else {
        return;
    };
    if was_raid
        || (brain.is_active(Activity::PLAY) && !body.villager.is_some_and(|villager| villager.baby))
    {
        brain.set_active_activity_if_possible(scheduled);
    } else {
        brain.update_activity_from_schedule(game_time, scheduled);
    }
}

fn forget_destination(brain: &mut Brain) {
    for memory in [
        MemoryModuleType::Path,
        MemoryModuleType::WalkTarget,
        MemoryModuleType::LookTarget,
        MemoryModuleType::BreedTarget,
        MemoryModuleType::InteractionTarget,
    ] {
        brain.erase_memory(memory);
    }
}

/// `WorkAtPoi.start`'s `playWorkSound` and restock.
pub(crate) fn worked_at_poi(
    entity: &mut Entity,
    store: &mut EntityStore,
    game_time: i64,
    current_day: i64,
) {
    let profession = match entity.data() {
        EntityData::GeneralLiving(data) => data
            .villager_profession_and_level()
            .map(|(profession, _)| profession.to_owned()),
        _ => None,
    };
    if let Some(path) = profession
        .as_deref()
        .and_then(|profession| profession.strip_prefix("minecraft:"))
        .filter(|path| !matches!(*path, "none" | "nitwit"))
    {
        crate::entity::villager::play_body_sound(
            entity,
            store,
            &format!("minecraft:entity.villager.work_{path}"),
        );
    }
    if crate::entity::villager::restock_if_due(entity, game_time, current_day)
        && entity.merchant_trading_player().is_some()
    {
        store.queue_command(crate::entity::EntityCommand::Villager(
            crate::entity::villager::PendingVillagerAction::ResendOffers {
                entity_id: entity.id(),
            },
        ));
    }
}
