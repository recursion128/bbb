//! Piglin anger, hunting, gold admiration, bartering, and food pickup.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, InteractionHand, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::ai::mob::target::conditions::TargetingConditions;
use crate::entity::ai::mob::target::{Target, TargetRef, TargetView};
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageSource};
use crate::entity::item_slots::{set_living_item, set_living_item_in_tick, SlotWriteEffects};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

mod celebration;
mod combat;
mod hunting;
mod retreat;
mod riding;

// PiglinAi.java:83-88, 814-815.
const ADMIRE_DURATION: i64 = 119;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CelebrationState {
    pub(crate) location: BlockPos,
    pub(crate) ticks: u16,
    pub(crate) dancing: bool,
}

pub(crate) fn celebration(entity: &Entity) -> Option<CelebrationState> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.piglin_celebration(),
        _ => None,
    }
}

pub(crate) fn before_ai(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if is_piglin(entity) {
        celebration::before_ai(entity, store, level);
        riding::before_ai(entity, store, level);
    }
}

#[derive(Clone)]
pub(crate) struct PiglinRuntime {
    food_cooldown: u16,
    sensor_delay: u8,
    nearest_item: Option<i32>,
    walking_to_item: bool,
    reach_ticks: u16,
    avoid_target: Option<TargetRef>,
    retreat_ticks: u16,
    outnumbered: bool,
    nearest_zombified: Option<TargetRef>,
    nearest_repellent: Option<BlockPos>,
    riding: riding::RideState,
    activity: Activity,
}

/// The non-core activity `PiglinAi.updateActivity` selects (`PiglinAi.java:306-311`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Activity {
    AdmireItem,
    Fight,
    Avoid,
    Celebrate,
    Ride,
    #[default]
    Idle,
}

impl PiglinRuntime {
    pub(crate) fn new() -> Self {
        Self {
            food_cooldown: 0,
            sensor_delay: 0,
            nearest_item: None,
            walking_to_item: false,
            reach_ticks: 0,
            avoid_target: None,
            retreat_ticks: 0,
            outnumbered: false,
            nearest_zombified: None,
            nearest_repellent: None,
            riding: riding::RideState::default(),
            activity: Activity::Idle,
        }
    }

    pub(crate) fn sensing_due(&self) -> bool {
        self.sensor_delay <= 1
    }

    pub(crate) fn retreat_target(&self) -> Option<TargetRef> {
        self.avoid_target
    }

    pub(crate) fn nearest_zombified(&self) -> Option<TargetRef> {
        self.nearest_zombified
    }

    pub(crate) fn nearest_repellent(&self) -> Option<BlockPos> {
        self.nearest_repellent
    }

    pub(crate) fn ride_target(&self) -> Option<TargetRef> {
        self.riding.target()
    }

    pub(crate) fn has_eaten_recently(&self) -> bool {
        self.food_cooldown > 0
    }

    pub(crate) fn eat(&mut self) {
        // PiglinAi.java:786-787. ATE_RECENTLY has no persistence codec.
        self.food_cooldown = 200;
    }

    pub(crate) fn tick(&mut self) {
        self.food_cooldown = self.food_cooldown.saturating_sub(1);
        self.sensor_delay = self.sensor_delay.saturating_sub(1);
    }
}

fn activity(entity: &Entity, store: &EntityStore) -> Option<Activity> {
    let runtime = store.mob_ai(entity.id())?;
    let piglin = runtime.piglin()?;
    Some(if is_admiring(entity) {
        Activity::AdmireItem
    } else if runtime.has_target() {
        Activity::Fight
    } else if piglin.retreat_target().is_some() {
        Activity::Avoid
    } else if celebration(entity).is_some() {
        Activity::Celebrate
    } else if piglin.ride_target().is_some() {
        Activity::Ride
    } else {
        Activity::Idle
    })
}

// AbstractPiglin.java:128-132; PiglinAi.java:799-800.
pub(crate) fn ambient_sound(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<&'static str> {
    // PiglinBrute.java:118-120.
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID {
        return (!store.mob_ai(entity.id())?.has_target())
            .then_some("minecraft:entity.piglin_brute.ambient");
    }
    match activity(entity, store)? {
        Activity::Idle => activity_sound(entity, store, level, Activity::Idle),
        _ => None,
    }
}

// PiglinAi.java:615-635.
fn activity_sound(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    activity: Activity,
) -> Option<&'static str> {
    if activity == Activity::Fight {
        return Some("minecraft:entity.piglin.angry");
    }
    let runtime = store.mob_ai(entity.id())?;
    let piglin = runtime.piglin()?;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    if data
        .zombification()?
        .is_converting(data.is_no_ai(), level.piglins_zombify(entity.position())?)
    {
        return Some("minecraft:entity.piglin.retreat");
    }
    let targets = runtime.target_view(entity, store, level);
    if activity == Activity::Avoid
        && piglin
            .retreat_target()
            .and_then(|target| targets.facts_of(target))
            .is_some_and(|target| distance_squared(entity.position(), target.position()) < 144.0)
    {
        return Some("minecraft:entity.piglin.retreat");
    }
    match activity {
        Activity::AdmireItem => return Some("minecraft:entity.piglin.admiring_item"),
        Activity::Celebrate => return Some("minecraft:entity.piglin.celebrate"),
        _ => {}
    }
    if targets
        .local_players()
        .any(|player| player_holds_loved_item(&targets, player) && visible(&targets, player))
    {
        Some("minecraft:entity.piglin.jealous")
    } else if piglin.nearest_repellent().is_some() {
        Some("minecraft:entity.piglin.retreat")
    } else {
        Some("minecraft:entity.piglin.ambient")
    }
}

fn make_sound(entity: &Entity, store: &mut EntityStore, sound: &str) {
    if entity.is_silent() {
        return;
    }
    let pitch = crate::entity::living::voice_pitch(store, baby(entity));
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position: entity.position(),
        resource_id: sound.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}

/// `PiglinAi.updateActivity`'s entry sound (`PiglinAi.java:313-315`). Avoid and
/// celebrate entries play theirs where those activities are installed.
fn update_activity_sound(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(activity) = activity(entity, store) else {
        return;
    };
    let Some(piglin) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.piglin_mut())
    else {
        return;
    };
    if std::mem::replace(&mut piglin.activity, activity) == activity
        || matches!(activity, Activity::Avoid | Activity::Celebrate)
    {
        return;
    }
    if let Some(sound) = activity_sound(entity, store, level, activity) {
        make_sound(entity, store, sound);
    }
}

/// `PiglinBruteAi.updateActivity`'s fight entry sound (`PiglinBruteAi.java:113-120`)
/// and `maybePlayActivitySound` (`:150-162`).
fn brute_activity_sound(entity: &Entity, store: &mut EntityStore) {
    use crate::entity::ai::mob::CombatActivity;
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    let fighting = runtime.has_target();
    let entered = runtime.swap_combat_activity(if fighting {
        CombatActivity::Fight
    } else {
        CombatActivity::Idle
    }) != CombatActivity::Fight;
    if fighting && entered {
        make_sound(entity, store, "minecraft:entity.piglin_brute.angry");
    }
    if fighting && store.random().next_float() < 0.0125 {
        // PiglinBrute.java:137-139.
        make_sound(entity, store, "minecraft:entity.piglin_brute.angry");
    }
}

pub(crate) fn visible<'w>(targets: &TargetView<'_, '_, 'w>, target: Target<'_, '_, 'w>) -> bool {
    let Some(living) = targets.body_entity().living() else {
        return false;
    };
    target.is_local()
        && target.alive()
        && target.entity_id() != targets.body_entity().id()
        && TargetingConditions::for_non_combat()
            .range(
                living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
                ),
            )
            .test(targets, &target, |_| true)
}

// PiglinAi.java:835-836.
pub(crate) fn player_holds_loved_item(
    targets: &TargetView<'_, '_, '_>,
    target: Target<'_, '_, '_>,
) -> bool {
    target.player().is_some_and(|player| {
        [player.main_hand_item_id, player.off_hand_item_id]
            .into_iter()
            .flatten()
            .filter_map(bbb_protocol::item_types::vanilla_item_type)
            .any(|item| {
                targets.level().registry_tag_membership(
                    "minecraft:item",
                    "minecraft:piglin_loved",
                    item.resource_id,
                )
            })
    })
}

fn has_eaten_recently(entity: &Entity, store: &EntityStore) -> bool {
    store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.piglin())
        .is_some_and(PiglinRuntime::has_eaten_recently)
}

fn is_piglin(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID
}

fn is_piglin_family(entity: &Entity) -> bool {
    matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
    )
}

fn can_hunt(entity: &Entity) -> bool {
    is_piglin(entity)
        && matches!(
            entity.data(),
            EntityData::GeneralLiving(data) if data.piglin_cannot_hunt() == Some(false)
        )
}

fn can_be_hunted(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID
        && !baby(entity)
        && matches!(
            entity.data(),
            EntityData::GeneralLiving(data) if data.hoglin_cannot_be_hunted() == Some(false)
        )
}

// PiglinAi.java:92; PiglinAi.java:782-784.
pub(crate) fn dont_hunt(entity: &mut Entity, random: &GameRandom) {
    crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::HuntedRecently,
            value: MemoryValue::Bool(true),
            time_to_live: Some(i64::from(random.next_int_between_inclusive(600, 2400))),
        },
    );
}

fn baby(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby())
}

fn is_avoiding(entity: &Entity, store: &EntityStore) -> bool {
    store.mob_ai(entity.id()).is_some_and(|runtime| {
        runtime.target_ref().is_none()
            && runtime
                .piglin()
                .is_some_and(|piglin| piglin.avoid_target.is_some())
    })
}

// PiglinSpecificSensor.java:63-80; NearestLivingEntitySensor.java:17-24.
fn visit_visible_adults(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    mut visit: impl FnMut(&Entity, f64),
) {
    visit_visible_living(entity, store, level, |target, distance| {
        if !baby(target)
            && matches!(
                target.entity_type_id(),
                VANILLA_ENTITY_TYPE_HOGLIN_ID
                    | VANILLA_ENTITY_TYPE_PIGLIN_ID
                    | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            )
        {
            visit(target, distance);
        }
    });
}

fn visit_visible_living(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    mut visit: impl FnMut(&Entity, f64),
) {
    let Some(living) = entity.living() else {
        return;
    };
    let range =
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let bounds = entity.bounding_box().inflate(range, range, range);
    let conditions = TargetingConditions::for_non_combat().range(range);
    let current = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.target_ref());
    let sensing = Default::default();
    let view = TargetView::new(entity, store, level, &sensing);
    for target in view.local_stored() {
        let conditions = if current == Some(target.target_ref()) {
            conditions.ignore_invisibility_testing()
        } else {
            conditions
        };
        if !target.bounding_box().intersects(bounds) || !conditions.test(&view, &target, |_| true) {
            continue;
        }
        if let Some(body) = store.get(target.entity_id()) {
            visit(body, distance_squared(entity.position(), target.position()));
        }
    }
}

// PiglinSpecificSensor.java:63-95.
fn sense_environment(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let mut piglins = 1;
    let mut hoglins = 0;
    let mut zombified = None;
    let mut nemesis = None;
    let mut huntable = None;
    let mut baby_hoglin = None;
    let mut recent_hunt = false;
    visit_visible_living(entity, store, level, |target, distance| {
        let nearest = match target.entity_type_id() {
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
                &mut zombified
            }
            VANILLA_ENTITY_TYPE_WITHER_ID | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => &mut nemesis,
            VANILLA_ENTITY_TYPE_HOGLIN_ID if baby(target) => &mut baby_hoglin,
            VANILLA_ENTITY_TYPE_HOGLIN_ID if !baby(target) => {
                hoglins += 1;
                if !can_be_hunted(target) {
                    return;
                }
                &mut huntable
            }
            VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
                if !baby(target) =>
            {
                piglins += 1;
                recent_hunt |= memory(target, MemoryModuleType::HuntedRecently);
                return;
            }
            _ => return,
        };
        if nearest.is_none_or(|(_, best)| distance < best) {
            *nearest = Some((
                TargetRef {
                    entity_id: target.id(),
                    uuid: target.uuid(),
                },
                distance,
            ));
        }
    });
    let repellent = find_repellent(entity, level);
    if let Some(piglin) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.piglin_mut())
    {
        piglin.outnumbered = hoglins > piglins;
        piglin.nearest_zombified = zombified.map(|(target, _)| target);
        piglin.nearest_repellent = repellent;
    }
    retreat::sense(entity, store, level, nemesis.map(|(target, _)| target));
    riding::sense(entity, store, level, baby_hoglin.map(|(target, _)| target));
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    if runtime
        .piglin()
        .is_some_and(|piglin| piglin.avoid_target.is_none())
    {
        let current = runtime.target_ref();
        hunting::try_start(
            entity,
            store,
            level,
            current,
            huntable.map(|(target, _)| target),
            recent_hunt,
        );
    }
}

// PiglinSpecificSensor.java:111-118; BlockPos.java:349-359.
fn find_repellent(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<BlockPos> {
    let center = entity.block_position();
    for distance in 0..=20_i32 {
        let min_x = (-distance).max(-8);
        let max_x = distance.min(8);
        for x in min_x..=max_x {
            let rem_x = distance - x.abs();
            let min_y = (-rem_x).max(-4);
            let max_y = rem_x.min(4);
            for y in min_y..=max_y {
                let abs_z = rem_x - y.abs();
                if abs_z > 8 {
                    continue;
                }
                let zs: &[i32] = if abs_z == 0 { &[0] } else { &[-abs_z, abs_z] };
                for &z in zs {
                    let pos = BlockPos {
                        x: center.x + x,
                        y: center.y + y,
                        z: center.z + z,
                    };
                    let state = level.block_state(pos);
                    if bbb_server_types::fluid::block_state_is_air(state) {
                        continue;
                    }
                    if level.block_state_in_tag(state, "minecraft:piglin_repellents")
                        && (bbb_protocol::block_state_types::vanilla_block_state_name(state)
                            != Some("minecraft:soul_campfire")
                            || crate::block_behavior::campfire::is_lit(state))
                    {
                        return Some(pos);
                    }
                }
            }
        }
    }
    None
}

fn memory(entity: &Entity, memory_type: MemoryModuleType) -> bool {
    crate::entity::brain_packed(entity).is_some_and(|brain| {
        brain
            .memories
            .iter()
            .any(|entry| entry.memory_type == memory_type)
    })
}

fn erase(entity: &mut Entity, memory_type: MemoryModuleType) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.retained_brain_mut()
            .memories
            .retain(|entry| entry.memory_type != memory_type);
    }
}

fn attack_conditions(entity: &Entity) -> TargetingConditions {
    let range = entity.living().map_or(16.0, |living| {
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
    });
    TargetingConditions::for_combat().range(range)
}

// PiglinAi.java:687-698.
fn set_anger_target(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) -> bool {
    let sensing = Default::default();
    let view = TargetView::new(entity, store, level, &sensing)
        .with_detached(level.detached_entity(target.entity_id));
    let Some(facts) = view.facts_of(target) else {
        return false;
    };
    let mut conditions = attack_conditions(entity).ignore_line_of_sight();
    if store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.target_ref())
        == Some(target)
    {
        conditions = conditions.ignore_invisibility_testing();
    }
    if !conditions.test(&view, &facts, |_| true) {
        return false;
    }
    let hoglin = facts.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID;
    let universal = facts.is_player() && view.universal_anger();
    erase(entity, MemoryModuleType::CantReachWalkTargetSince);
    crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::AngryAt,
            value: MemoryValue::Uuid(target.uuid),
            time_to_live: Some(600),
        },
    );
    if hoglin && can_hunt(entity) {
        dont_hunt(entity, store.random());
    }
    if universal {
        crate::entity::set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::UniversalAnger,
                value: MemoryValue::Bool(true),
                time_to_live: Some(600),
            },
        );
    }
    true
}

fn nearest_visible_player(
    targets: &TargetView<'_, '_, '_>,
    current: Option<TargetRef>,
    eligible: impl Fn(Target<'_, '_, '_>) -> bool,
) -> Option<TargetRef> {
    let conditions = attack_conditions(targets.body_entity());
    targets
        .local_players()
        .filter(|target| {
            let conditions = if current == Some(target.target_ref()) {
                conditions.ignore_invisibility_testing()
            } else {
                conditions
            };
            eligible(*target) && conditions.test(targets, target, |_| true)
        })
        .min_by(|a, b| {
            targets
                .body()
                .distance_to_facts_sqr(a)
                .total_cmp(&targets.body().distance_to_facts_sqr(b))
        })
        .map(|target| target.target_ref())
}

pub(crate) fn anger_nearby(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    player_id: i32,
    only_if_visible: bool,
) {
    let Some(player) = level
        .players()
        .iter()
        .find(|player| player.entity_id == player_id)
    else {
        return;
    };
    // PiglinAi.java:538-547.
    let area = player.bounding_box().inflate(16.0, 16.0, 16.0);
    for id in store.ids_in_box_sections(area) {
        let Some(entity) = store.get(id) else {
            continue;
        };
        if !is_piglin(entity)
            || !entity.is_alive_with_health()
            || !entity.bounding_box().intersects(area)
            || is_admiring(entity)
            || is_avoiding(entity, store)
            || celebration(entity).is_some()
            || store.mob_ai(id).is_some_and(|runtime| runtime.has_target())
        {
            continue;
        }
        let sensing = Default::default();
        let targets = TargetView::new(entity, store, level, &sensing);
        let Some(trigger) = targets.facts(player_id) else {
            continue;
        };
        if only_if_visible && !trigger.line_of_sight() {
            continue;
        }
        let universal = targets.universal_anger();
        let target = if universal {
            nearest_visible_player(&targets, None, |_| true).unwrap_or_else(|| trigger.target_ref())
        } else {
            trigger.target_ref()
        };
        let _ = store.with_entity_mut(id, |entity, store| {
            set_anger_target(entity, store, level, target)
        });
    }
}

fn anger_uuid(entity: &Entity) -> Option<uuid::Uuid> {
    crate::entity::brain_packed(entity)?
        .memories
        .iter()
        .find_map(|entry| match (&entry.memory_type, &entry.value) {
            (MemoryModuleType::AngryAt, MemoryValue::Uuid(uuid)) => Some(*uuid),
            _ => None,
        })
}

fn player_without_safe_armor(targets: &TargetView<'_, '_, '_>, target: Target<'_, '_, '_>) -> bool {
    // PiglinAi.java:650-658. Only equipped armor participates in this item tag.
    targets
        .store()
        .player(target.entity_id())
        .is_some_and(|player| {
            !player.armor_items().any(|stack| {
                stack.item_resource_id().is_some_and(|item| {
                    targets.level().registry_tag_membership(
                        "minecraft:item",
                        "minecraft:piglin_safe_armor",
                        item,
                    )
                })
            })
        })
}

pub(crate) fn attack_target(
    targets: &TargetView<'_, '_, '_>,
    current: Option<TargetRef>,
    sensing_due: bool,
    avoid: Option<TargetRef>,
    zombified: Option<TargetRef>,
) -> Option<TargetRef> {
    let entity = targets.body_entity();
    let brute = entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID;
    // PiglinAi.java:501-519.
    if !brute
        && zombified
            .and_then(|target| targets.facts_of(target))
            .is_some_and(|target| {
                target.is_local()
                    && target.alive()
                    && distance_squared(entity.position(), target.position()) < 36.0
            })
    {
        return None;
    }
    // PiglinAi.java:155-169, 511-525. Gold armor does not exempt an anger target.
    if baby(entity) || (!brute && (is_admiring(entity) || (avoid.is_some() && current.is_none()))) {
        return None;
    }
    let conditions = attack_conditions(entity).ignore_line_of_sight();
    let angry = anger_uuid(entity)
        .and_then(|uuid| targets.by_uuid(uuid))
        .filter(|target| {
            // Sensor.java:80-85. A locked target keeps its full detection range when invisible.
            let conditions = if current == Some(target.target_ref()) {
                conditions.ignore_invisibility_testing()
            } else {
                conditions
            };
            conditions.test(targets, target, |_| true)
        })
        .map(|target| target.target_ref());
    if angry.is_some() {
        return angry;
    }
    let universal = memory(entity, MemoryModuleType::UniversalAnger);
    if !sensing_due {
        let retained = current
            .and_then(|target| targets.facts_of(target))
            .filter(|target| {
                attack_conditions(entity)
                    .ignore_invisibility_testing()
                    .test(targets, target, |_| true)
            });
        return retained
            .filter(|target| {
                is_nemesis(*target)
                    || (target.is_player()
                        && (brute || universal || player_without_safe_armor(targets, *target)))
            })
            .map(|target| target.target_ref());
    }
    // PiglinBruteAi.java:129-136: players take priority over a visible nemesis.
    if universal || brute {
        let player = nearest_visible_player(targets, current, |_| true);
        if player.is_some() {
            return player;
        }
    }
    let nemesis = targets
        .local_stored()
        .filter(|target| {
            let conditions = attack_conditions(entity);
            let conditions = if current == Some(target.target_ref()) {
                conditions.ignore_invisibility_testing()
            } else {
                conditions
            };
            is_nemesis(*target) && conditions.test(targets, target, |_| true)
        })
        .min_by(|a, b| {
            targets
                .body()
                .distance_to_facts_sqr(a)
                .total_cmp(&targets.body().distance_to_facts_sqr(b))
        })
        .map(|target| target.target_ref());
    if brute {
        nemesis
    } else {
        nemesis.or_else(|| {
            nearest_visible_player(targets, current, |target| {
                player_without_safe_armor(targets, target)
            })
        })
    }
}

// PiglinSpecificSensor.java:89-94; PiglinAi.java:529-535.
fn is_nemesis(target: Target<'_, '_, '_>) -> bool {
    matches!(
        target.entity_type_id(),
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID | VANILLA_ENTITY_TYPE_WITHER_ID
    )
}

pub(crate) fn is_admiring(entity: &Entity) -> bool {
    is_piglin(entity) && memory(entity, MemoryModuleType::AdmiringItem)
}

pub(crate) fn is_admiring_held_item(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    is_admiring(entity)
        && entity
            .living()
            .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
            .and_then(RuntimeItemStack::item_resource_id)
            .is_some_and(|item| {
                level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item)
            })
}

/// `PiglinAi.stopWalking` (`PiglinAi.java:660-663`).
pub(crate) fn stop_walking(entity: &mut Entity, store: &mut EntityStore) {
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.stop_navigation();
    }
}

fn gold(stack: &RuntimeItemStack) -> bool {
    !stack.is_empty() && stack.item_resource_id() == Some("minecraft:gold_ingot")
}

fn hold_admired_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    stack: RuntimeItemStack,
) -> Result<SlotWriteEffects, ()> {
    // Piglin.java:361-368; Mob.java:557-561.
    let makes_persistent = !gold(&stack);
    let old = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
        .cloned();
    let effects = set_living_item(entity, EquipmentSlot::OffHand, Some(stack)).map_err(|_| ())?;
    if let Some(old) = old.filter(|stack| !stack.is_empty()) {
        crate::entity::item::spawn_dropped_item_entity(store, entity.position(), old).ok_or(())?;
    }
    crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::AdmiringItem,
            value: MemoryValue::Bool(true),
            time_to_live: Some(ADMIRE_DURATION),
        },
    );
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = &mut data.mob {
            mob.set_guaranteed_drop(EquipmentSlot::OffHand);
            if makes_persistent {
                mob.set_persistence_required(true);
            }
        }
    }
    erase(entity, MemoryModuleType::TimeTryingToReachAdmireItem);
    if let Some(runtime) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.piglin_mut())
    {
        runtime.walking_to_item = false;
        runtime.reach_ticks = 0;
    }
    stop_walking(entity, store);
    Ok(effects)
}

/// PiglinAi.java:549-564.
pub(crate) fn interact(
    store: &mut EntityStore,
    entity_id: i32,
    stack: &RuntimeItemStack,
) -> Result<Option<SlotWriteEffects>, ()> {
    let entity = store.get(entity_id).ok_or(())?;
    if !is_piglin(entity)
        || !entity.is_alive_with_health()
        || baby(entity)
        || !gold(stack)
        || is_admiring(entity)
        || memory(entity, MemoryModuleType::AdmiringDisabled)
    {
        return Ok(None);
    }
    store
        .with_entity_mut(entity_id, |entity, store| {
            hold_admired_item(entity, store, stack.with_count(1))
        })
        .ok_or(())?
        .map(Some)
}

pub(crate) fn wants_ground_item(
    entity: &Entity,
    stack: &RuntimeItemStack,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    let Some(item) = stack.item_resource_id() else {
        return false;
    };
    // PiglinAi.java:462-480.
    if (baby(entity)
        && level.registry_tag_membership(
            "minecraft:item",
            "minecraft:ignored_by_piglin_babies",
            item,
        ))
        || level.registry_tag_membership("minecraft:item", "minecraft:piglin_repellents", item)
    {
        return false;
    }
    if memory(entity, MemoryModuleType::AdmiringDisabled)
        && (memory(entity, MemoryModuleType::AttackTarget)
            || store
                .mob_ai(entity.id())
                .is_some_and(|runtime| runtime.has_target()))
    {
        return false;
    }
    let offhand_loved = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
        .and_then(RuntimeItemStack::item_resource_id)
        .is_some_and(|item| {
            level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item)
        });
    if gold(stack) {
        return !offhand_loved;
    }
    let has_space = matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.internal_inventory().is_some_and(|inventory| inventory.can_add_item(level.item_stack_resolver(), stack).unwrap_or(false)));
    if item == "minecraft:gold_nugget" {
        return has_space;
    }
    if level.registry_tag_membership("minecraft:item", "minecraft:piglin_food", item) {
        return !has_eaten_recently(entity, store) && has_space;
    }
    if level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item) {
        return !offhand_loved && has_space;
    }
    crate::entity::mob_pickup::can_replace_piglin_item(entity, store, level, stack)
}

pub(crate) fn pick_up_ground_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stack: RuntimeItemStack,
) -> Result<(), ()> {
    stop_walking(entity, store);
    // PiglinAi.java:338-360.
    if stack.item_resource_id().is_some_and(|item| {
        level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item)
    }) {
        let effects = hold_admired_item(entity, store, stack)?;
        if let Some(sound) = effects.sound {
            store.play_entity_sound(sound);
        }
        if let Some(event) = effects.equip_event {
            level.game_event_at(event, entity.position(), Some(entity.id()));
        }
    } else if stack.item_resource_id().is_some_and(|item| {
        level.registry_tag_membership("minecraft:item", "minecraft:piglin_food", item)
    }) && !has_eaten_recently(entity, store)
    {
        if let Some(runtime) = store
            .mob_ai_mut(entity.id())
            .and_then(|runtime| runtime.piglin_mut())
        {
            runtime.eat();
        }
    } else if !crate::entity::mob_pickup::equip_piglin_item(entity, store, level, &stack) {
        inventory_or_throw(entity, store, level, stack);
    }
    Ok(())
}

pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if !entity.is_alive_with_health()
        || !is_piglin_family(entity)
        || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return;
    }
    if let Some(runtime) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.piglin_mut())
    {
        runtime.tick();
    }
    celebration::tick(entity);
    if store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.piglin())
        .is_some_and(|piglin| piglin.sensor_delay == 0)
    {
        sense_environment(entity, store, level);
    }
    retreat::tick(entity, store, level);
    riding::tick(entity, store, level);
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.retained_brain_mut().memories.retain_mut(|entry| {
            if !matches!(
                entry.memory_type,
                MemoryModuleType::AngryAt
                    | MemoryModuleType::UniversalAnger
                    | MemoryModuleType::HuntedRecently
                    | MemoryModuleType::AdmiringItem
                    | MemoryModuleType::AdmiringDisabled
                    | MemoryModuleType::DisableWalkToAdmireItem
            ) {
                return true;
            }
            if let Some(ttl) = &mut entry.time_to_live {
                *ttl = ttl.saturating_sub(1);
                return *ttl > 0;
            }
            true
        });
    }
    if let Some(uuid) = anger_uuid(entity) {
        let forgive_players = store.target_level_facts().forgive_dead_players;
        // StopBeingAngryIfTargetDead.java:16-22.
        let dead = store
            .visible_entity_id_for_uuid(uuid)
            .and_then(|id| store.get(id))
            .is_some_and(|target| {
                target
                    .living()
                    .is_some_and(|living| living.is_dead_or_dying())
                    && (target.entity_type_id()
                        != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
                        || forgive_players)
            })
            || (forgive_players
                && level
                    .players()
                    .iter()
                    .any(|player| player.uuid == uuid && player.dead_or_dying));
        if dead {
            erase(entity, MemoryModuleType::AngryAt);
        }
    }
    if !is_piglin(entity) {
        brute_activity_sound(entity, store);
        return;
    }
    update_activity_sound(entity, store, level);
    tick_wanted_item(entity, store, level);
    // StopHoldingItemIfNoLongerAdmiring.java:10-18.
    if !is_admiring(entity) && entity.living().and_then(|living| living.item_by_slot(EquipmentSlot::OffHand)).is_some_and(|stack|
        stack.has_component(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID).is_ok_and(|has| !has)) {
        stop_holding(entity, store, level, true);
    }
}

fn distance_squared(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}

fn tick_wanted_item(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(runtime) = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.piglin())
    else {
        return;
    };
    let sensor_due = runtime.sensor_delay == 0;
    if sensor_due {
        // NearestItemSensor.java:26-33. Sense wanted items before testing the loved tag.
        let area = entity.bounding_box().inflate(32.0, 16.0, 32.0);
        let from = Vec3d {
            y: entity.position().y + f64::from(entity.eye_height()),
            ..entity.position()
        };
        let mut nearest = None;
        let mut nearest_distance = 32.0 * 32.0;
        for id in store.ids_in_box_sections(area) {
            let Some(item) = store.get(id).filter(|item| item.is_alive()) else {
                continue;
            };
            let EntityData::Item(data) = item.data() else {
                continue;
            };
            let distance = distance_squared(entity.position(), item.position());
            if distance >= nearest_distance
                || !item.bounding_box().intersects(area)
                || !wants_ground_item(entity, data.item(), store, level)
            {
                continue;
            }
            let to = Vec3d {
                y: item.position().y + f64::from(item.eye_height()),
                ..item.position()
            };
            if crate::entity::projectile_hit::collision_clip_misses(level, from, to) {
                nearest = Some(id);
                nearest_distance = distance;
            }
        }
        let runtime = store
            .mob_ai_mut(entity.id())
            .and_then(|runtime| runtime.piglin_mut())
            .unwrap();
        runtime.nearest_item = nearest;
        runtime.sensor_delay = 20;
    }
    let runtime = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.piglin())
        .unwrap();
    let was_walking = runtime.walking_to_item;
    let target = runtime
        .nearest_item
        .and_then(|id| store.get(id))
        .filter(|item| {
            item.is_alive()
                && matches!(item.data(), EntityData::Item(data)
            if wants_ground_item(entity, data.item(), store, level))
        });
    let loved = target.is_some_and(|item| {
        matches!(item.data(), EntityData::Item(data)
        if data.item().item_resource_id().is_some_and(|item|
            level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item)))
    });
    let has_target = target.is_some();
    let position = target.map(Entity::block_position);
    let close =
        target.is_some_and(|item| distance_squared(entity.position(), item.position()) < 9.0 * 9.0);
    // StartAdmiringItemIfSeen.java:13-25; PiglinAi.java:215-225.
    if loved
        && !is_admiring(entity)
        && !memory(entity, MemoryModuleType::AdmiringDisabled)
        && !memory(entity, MemoryModuleType::DisableWalkToAdmireItem)
    {
        crate::entity::set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::AdmiringItem,
                value: MemoryValue::Bool(true),
                time_to_live: Some(ADMIRE_DURATION),
            },
        );
    }
    let offhand_empty = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
        .is_none_or(RuntimeItemStack::is_empty);
    // StopAdmiringIfItemTooFarAway.java:16-26.
    if is_admiring(entity) && offhand_empty && !close {
        erase(entity, MemoryModuleType::AdmiringItem);
    }
    let tired = {
        let runtime = store
            .mob_ai_mut(entity.id())
            .and_then(|runtime| runtime.piglin_mut())
            .unwrap();
        if is_admiring(entity) && offhand_empty && has_target {
            runtime.reach_ticks = runtime.reach_ticks.saturating_add(1);
        }
        runtime.reach_ticks > 200
    };
    // StopAdmiringIfTiredOfTryingToReachItem.java:24-37.
    if tired {
        erase(entity, MemoryModuleType::AdmiringItem);
        crate::entity::set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::DisableWalkToAdmireItem,
                value: MemoryValue::Bool(true),
                time_to_live: Some(200),
            },
        );
        store
            .mob_ai_mut(entity.id())
            .and_then(|runtime| runtime.piglin_mut())
            .unwrap()
            .reach_ticks = 0;
    }
    let can_walk = is_admiring(entity)
        && !is_admiring_held_item(entity, level)
        && loved
        && close
        && !memory(entity, MemoryModuleType::ItemPickupCooldownTicks)
        && matches!(entity.data(), EntityData::GeneralLiving(data) if data.mob.as_ref().is_some_and(|mob| mob.can_pick_up_loot()))
        && position.is_some_and(|pos| {
            level
                .world_border()
                .is_some_and(|border| border.contains_block_at(pos, level.game_time()))
        });
    if can_walk && (!was_walking || sensor_due) {
        let runtime = store.mob_ai(entity.id()).unwrap();
        if let Some(mob) = crate::entity::ai::mob::navigation::MobPathingSnapshot::from_entity(
            entity,
            runtime.has_target(),
            store.vehicle(entity.id()).is_some(),
            store.difficulty(),
            runtime.pathfinding_malus(),
        ) {
            store.mob_ai_mut(entity.id()).unwrap().navigate_to(
                level,
                mob,
                position.unwrap(),
                0,
                1.0,
            );
        }
    } else if !can_walk && was_walking {
        stop_walking(entity, store);
    }
    store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.piglin_mut())
        .unwrap()
        .walking_to_item = can_walk;
}

fn stop_holding(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    barter: bool,
) {
    let Some(held) = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
        .cloned()
    else {
        return;
    };
    if set_living_item_in_tick(entity, store, level, EquipmentSlot::OffHand, None).is_err() {
        return;
    }
    if !baby(entity) {
        if gold(&held) {
            if barter {
                let stacks = level.piglin_barter_loot_drops(
                    EntityBodyReadView::new(store, Some(entity), None),
                    entity,
                );
                let stacks = stacks
                    .into_iter()
                    .map(RuntimeItemStack::from_summary)
                    .collect::<Vec<_>>();
                throw_items(entity, store, level, stacks, false);
            }
        } else if !crate::entity::mob_pickup::equip_piglin_item(entity, store, level, &held) {
            inventory_or_throw(entity, store, level, held);
        }
    } else if !crate::entity::mob_pickup::equip_piglin_item(entity, store, level, &held) {
        if let Some(main) = entity
            .living()
            .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
            .cloned()
        {
            let loved = main.item_resource_id().is_some_and(|item| {
                level.registry_tag_membership("minecraft:item", "minecraft:piglin_loved", item)
            });
            if loved {
                inventory_or_throw(entity, store, level, main);
            } else {
                throw_items(entity, store, level, vec![main], false);
            }
        }
        let _ = set_living_item_in_tick(entity, store, level, EquipmentSlot::MainHand, Some(held));
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            if let Some(mob) = &mut data.mob {
                mob.set_guaranteed_drop(EquipmentSlot::MainHand);
                mob.set_persistence_required(true);
            }
        }
    }
}

fn inventory_or_throw(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stack: RuntimeItemStack,
) {
    let rest = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .internal_inventory_mut()
            .map(|inventory| inventory.add_item(level.item_stack_resolver(), stack)),
        _ => None,
    };
    if let Some(rest) = rest.filter(|stack| !stack.is_empty()) {
        throw_items(entity, store, level, vec![rest], true);
    }
}

fn throw_items(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stacks: Vec<RuntimeItemStack>,
    random_only: bool,
) {
    if stacks.is_empty() {
        return;
    }
    let player = if random_only {
        None
    } else {
        use crate::entity::ai::mob::target::{conditions::TargetingConditions, TargetView};
        let sensing = Default::default();
        let view = TargetView::new(entity, store, level, &sensing);
        let conditions = TargetingConditions::for_non_combat().range(16.0);
        view.local_players()
            .filter(|target| conditions.test(&view, target, |_| true))
            .min_by(|a, b| {
                view.body()
                    .distance_to_facts_sqr(a)
                    .total_cmp(&view.body().distance_to_facts_sqr(b))
            })
            .map(|target| target.position())
    };
    let mut towards = player.unwrap_or_else(|| {
        use crate::entity::ai::mob::random_pos::{land_random_pos, RandomPosMob};
        store
            .effective_pathfinding_malus(entity, level)
            .and_then(|malus| {
                land_random_pos(
                    level,
                    level.level_random(),
                    RandomPosMob::of(entity, malus),
                    4,
                    2,
                )
            })
            .flatten()
            .unwrap_or_else(|| entity.position())
    });
    towards.y += 1.0;
    if entity
        .living_mut()
        .is_some_and(|living| living.swing(InteractionHand::OffHand))
    {
        level.entity_animation(entity.id(), InteractionHand::OffHand);
    }
    for stack in stacks {
        if !stack.is_empty() {
            crate::entity::item::throw_item(entity, store, towards, stack);
        }
    }
}

/// PiglinAi.java:566-578.
pub(crate) fn was_hurt(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    living_attacker: bool,
) {
    if !is_piglin_family(entity) || !living_attacker {
        return;
    }
    let Some(attacker) = source.causing_entity else {
        return;
    };
    if attacker.entity_id() == entity.id() {
        return;
    }
    let target = {
        let sensing = Default::default();
        let view = TargetView::new(entity, store, level, &sensing)
            .with_detached(level.detached_entity(attacker.entity_id()));
        view.facts(attacker.entity_id())
            .map(|target| (target.target_ref(), target.entity_type_id()))
    };
    let Some((target, kind)) = target else {
        return;
    };
    if kind == VANILLA_ENTITY_TYPE_PIGLIN_ID
        || (entity.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
            && kind == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID)
    {
        return;
    }
    if !is_piglin(entity) {
        combat::was_hurt(entity, store, level, target);
        return;
    }
    stop_holding(entity, store, level, false);
    erase(entity, MemoryModuleType::AdmiringItem);
    celebration::clear(entity);
    if matches!(attacker, StoredDamageCausingEntity::Player { .. }) {
        crate::entity::set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::AdmiringDisabled,
                value: MemoryValue::Bool(true),
                time_to_live: Some(400),
            },
        );
    }
    if !retreat::was_hurt(entity, store, level, target) && !baby(entity) {
        combat::was_hurt(entity, store, level, target);
    }
}

pub(crate) fn prepare_zombification(entity: &mut Entity, store: &mut EntityStore) {
    if !is_piglin(entity) {
        return;
    }
    // PiglinAi.java:412-417; Piglin.java:283-286.
    if is_admiring(entity) {
        let held = entity
            .living()
            .and_then(|living| living.item_by_slot(EquipmentSlot::OffHand))
            .cloned();
        if let Some(held) = held {
            let _ = set_living_item(entity, EquipmentSlot::OffHand, None);
            crate::entity::item::spawn_dropped_item_entity(store, entity.position(), held);
        }
    }
    let stacks = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .internal_inventory_mut()
            .map(|inventory| inventory.remove_all_items())
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    for (_, stack) in stacks {
        crate::entity::item::spawn_dropped_item_entity(store, entity.position(), stack);
    }
}
