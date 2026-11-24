//! Warden suspect anger and vibration cooldowns.

use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_WARDEN_ANGER_LEVEL_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use rustc_hash::FxHashMap;
use uuid::Uuid;

use crate::entity::ai::mob::target::{conditions, TargetRef, TargetView};
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::game_event_listener::{EntityGameEventListenerState, ReceivedVibration};
use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_field, optional_int, optional_uuid, uuid_int_array};
use crate::entity::{Entity, EntityData, EntityStore, StoredEntityDamageSource};
use crate::world::simulation_level::LevelCtx;

const RECENT_PROJECTILE_TICKS: i64 = 100;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WardenState {
    pub(crate) listener: EntityGameEventListenerState,
    pub(crate) sonic: super::sonic_boom::SonicBoomState,
    pub(crate) awareness: super::awareness::AwarenessState,
    pub(crate) lifecycle: super::lifecycle::LifecycleState,
    suspects: FxHashMap<Uuid, i32>,
    vibration_cooldown: Option<i64>,
    recent_projectile: Option<i64>,
    touch_cooldown: Option<i64>,
    roaring_target: Option<Uuid>,
    client_anger: i32,
}

impl WardenState {
    // AngerLevel.ANGRY.
    pub(crate) const ANGRY_THRESHOLD: i32 = 80;

    pub(crate) fn new() -> Self {
        Self {
            listener: EntityGameEventListenerState::new(VANILLA_ENTITY_TYPE_WARDEN_ID)
                .expect("warden listener"),
            sonic: super::sonic_boom::SonicBoomState::default(),
            awareness: super::awareness::AwarenessState::default(),
            lifecycle: super::lifecycle::LifecycleState::default(),
            suspects: FxHashMap::default(),
            vibration_cooldown: None,
            recent_projectile: None,
            touch_cooldown: None,
            roaring_target: None,
            client_anger: 0,
        }
    }

    // AngerManagement.java:38-40: each saved suspect carries a UUID and nonnegative anger.
    pub(crate) fn read_save(&mut self, entries: &[NbtSummaryEntry]) {
        self.listener.read_save_data(entries);
        self.suspects.clear();
        let loaded = (|| {
            if let Some(value) = optional_field(entries, "anger") {
                let NbtSummaryValue::Compound(fields) = value else {
                    return None;
                };
                if let Some(value) = optional_field(fields, "suspects") {
                    let NbtSummaryValue::List(suspects) = value else {
                        return None;
                    };
                    for suspect in suspects {
                        let NbtSummaryValue::Compound(fields) = suspect else {
                            return None;
                        };
                        let uuid = optional_uuid(fields, "uuid")?;
                        let anger = optional_int(fields, "anger").filter(|anger| *anger >= 0)?;
                        self.suspects.insert(uuid, anger);
                    }
                }
            }
            Some(())
        })();
        if loaded.is_none() {
            // Invalid anger must not discard the independent vibration listener.
            self.suspects = FxHashMap::default();
        }
        self.client_anger = 0;
    }

    pub(crate) fn write_save(&self, entries: &mut Vec<NbtSummaryEntry>) {
        crate::entity::game_event_listener::write_save_data(entries, &self.listener);
        entries.push(NbtSummaryEntry {
            name: "anger".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "suspects".into(),
                value: NbtSummaryValue::List(
                    self.suspects
                        .iter()
                        .map(|(uuid, anger)| {
                            NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "uuid".into(),
                                    value: uuid_int_array(*uuid),
                                },
                                NbtSummaryEntry {
                                    name: "anger".into(),
                                    value: NbtSummaryValue::Int(*anger),
                                },
                            ])
                        })
                        .collect(),
                ),
            }]),
        });
    }

    pub(crate) fn synched_value(&self) -> SynchedValue {
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_WARDEN_ANGER_LEVEL_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(self.client_anger),
            },
            is_default: self.client_anger == 0,
        }
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.listener.restore_command_transients(&previous.listener);
        self.roaring_target = previous.roaring_target;
        self.client_anger = previous.client_anger;
        self.awareness
            .restore_command_transients(&previous.awareness);
    }

    pub(crate) fn take_cooldowns(&mut self, brain: &mut BrainPacked) {
        self.sonic.take_memories(brain);
        self.awareness.take_memories(brain);
        self.lifecycle.take_memories(brain);
        self.vibration_cooldown = None;
        self.recent_projectile = None;
        self.touch_cooldown = None;
        let mut index = 0;
        while index < brain.memories.len() {
            let cooldown = match brain.memories[index].memory_type {
                MemoryModuleType::VibrationCooldown => &mut self.vibration_cooldown,
                MemoryModuleType::RecentProjectile => &mut self.recent_projectile,
                MemoryModuleType::TouchCooldown => &mut self.touch_cooldown,
                _ => {
                    index += 1;
                    continue;
                }
            };
            let memory = brain.memories.remove(index);
            *cooldown = Some(memory.time_to_live.unwrap_or(i64::MAX));
        }
    }

    pub(crate) fn write_cooldowns(&self, brain: &mut BrainPacked) {
        self.sonic.write_memories(brain);
        self.awareness.write_memories(brain);
        self.lifecycle.write_memories(brain);
        for (memory_type, cooldown) in [
            (MemoryModuleType::VibrationCooldown, self.vibration_cooldown),
            (MemoryModuleType::RecentProjectile, self.recent_projectile),
            (MemoryModuleType::TouchCooldown, self.touch_cooldown),
        ] {
            if let Some(ttl) = cooldown {
                brain.memories.push(PackedMemory {
                    memory_type,
                    value: MemoryValue::Unit,
                    time_to_live: (ttl != i64::MAX).then_some(ttl),
                });
            }
        }
    }

    pub(crate) fn vibration_cooldown(&self) -> bool {
        self.vibration_cooldown.is_some()
    }

    pub(crate) fn anger(&self, uuid: Uuid) -> i32 {
        self.suspects.get(&uuid).copied().unwrap_or(0)
    }

    // AngerManagement.Sorter: angry suspects, then players, then greatest anger.
    fn active_suspect(&self, targets: &TargetView<'_, '_, '_>) -> Option<TargetRef> {
        self.suspects
            .iter()
            .filter_map(|(uuid, anger)| {
                let target = targets.by_uuid(*uuid)?;
                conditions::is_valid_target(targets, &target).then_some((
                    (*anger >= Self::ANGRY_THRESHOLD, target.is_player(), *anger),
                    target.target_ref(),
                ))
            })
            .max_by_key(|(priority, _)| *priority)
            .map(|(_, target)| target)
    }

    pub(crate) fn angry_target(&self, targets: &TargetView<'_, '_, '_>) -> Option<TargetRef> {
        self.active_suspect(targets)
            .filter(|target| self.anger(target.uuid) >= Self::ANGRY_THRESHOLD)
    }

    // AngerManagement.java:143-145: new anger is capped at 150.
    fn increase(&mut self, uuid: Uuid, amount: i32) {
        self.lifecycle.refresh_dig_cooldown();
        let anger = self.suspects.entry(uuid).or_default();
        *anger = anger.saturating_add(amount).min(150);
    }

    fn tick(&mut self, targets: &TargetView<'_, '_, '_>, current_target: Option<TargetRef>) {
        self.sonic.tick();
        self.awareness.tick();
        tick_cooldown(&mut self.vibration_cooldown);
        tick_cooldown(&mut self.recent_projectile);
        tick_cooldown(&mut self.touch_cooldown);
        if targets.body_entity().tick_count() % 20 == 0 {
            self.suspects.retain(|uuid, anger| {
                *anger -= i32::from(*anger > 0);
                *anger > 0
                    && targets.by_uuid(*uuid).is_none_or(|target| {
                        !target.is_local() || conditions::is_valid_target(targets, &target)
                    })
            });
        }
        let roaring_target = (targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_ROARING_ID)
            .then_some(current_target)
            .flatten();
        if roaring_target.map(|target| target.uuid) != self.roaring_target {
            if let Some(target) = roaring_target.and_then(|target| targets.facts_of(target)) {
                if conditions::is_valid_target(targets, &target) {
                    self.increase(target.uuid(), 20);
                }
            }
        }
        self.roaring_target = roaring_target.map(|target| target.uuid);
        // WardenAi.java:135-149: fighting keeps an existing dig cooldown alive.
        if targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID
            && current_target.is_some_and(|target| {
                self.anger(target.uuid) >= Self::ANGRY_THRESHOLD
                    && targets
                        .facts_of(target)
                        .is_some_and(|target| conditions::is_valid_target(targets, &target))
            })
        {
            self.lifecycle.refresh_dig_cooldown();
        }
        // Warden.java:246-251: a persistent warden keeps refreshing its dig cooldown.
        let body = targets.body_entity();
        if matches!(body.data(), crate::entity::EntityData::GeneralLiving(data)
            if data.mob_retained().is_some_and(|mob| mob.persistence_required()))
            || targets.store().vehicle(body.id()).is_some()
            || crate::entity::leash::is_leashed(body)
        {
            self.lifecycle.refresh_dig_cooldown();
        }
        self.lifecycle.tick();
    }

    fn active_anger(&self, targets: &TargetView<'_, '_, '_>, target: Option<TargetRef>) -> i32 {
        target.map_or_else(
            || {
                self.suspects
                    .iter()
                    .filter(|(uuid, _)| {
                        targets
                            .by_uuid(**uuid)
                            .is_some_and(|target| conditions::is_valid_target(targets, &target))
                    })
                    .map(|(_, anger)| *anger)
                    .max()
                    .unwrap_or(0)
            },
            |target| self.anger(target.uuid),
        )
    }
}

// Warden.java:199-205; AngerLevel.java:9-11.
pub(crate) fn ambient_sound(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<&'static str> {
    if matches!(
        entity.pose_id(),
        VANILLA_ENTITY_POSE_ROARING_ID
            | VANILLA_ENTITY_POSE_DIGGING_ID
            | VANILLA_ENTITY_POSE_EMERGING_ID
    ) {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let runtime = store.mob_ai(entity.id())?;
    let targets = runtime.target_view(entity, store, level);
    let anger = data
        .warden_state()?
        .active_anger(&targets, runtime.target_ref());
    Some(if anger >= WardenState::ANGRY_THRESHOLD {
        "minecraft:entity.warden.angry"
    } else if anger >= 40 {
        "minecraft:entity.warden.agitated"
    } else {
        "minecraft:entity.warden.ambient"
    })
}

pub(super) fn tick_cooldown(cooldown: &mut Option<i64>) {
    if let Some(ttl) = *cooldown {
        if ttl != i64::MAX {
            *cooldown = (ttl > 1).then(|| ttl - 1);
        }
    }
}

pub(crate) fn before_ai(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data.warden_state().is_none() {
        return;
    }
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    let mut state = std::mem::replace(state, WardenState::new());
    let previous_anger = state.client_anger;
    if let Some(runtime) = store.mob_ai(entity.id()) {
        let targets = runtime.target_view(entity, store, level);
        let current_target = runtime.target_ref();
        state.tick(&targets, current_target);
        state.client_anger = state.active_anger(&targets, current_target);
    }
    let client_anger = state.client_anger;
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        *data.warden_state_mut().expect("warden state") = state;
    }
    if client_anger != previous_anger {
        entity.mark_synched_dirty(VANILLA_WARDEN_ANGER_LEVEL_DATA_ID);
    }
}

// Warden.java:608-636: arrival updates memories and disturbance even with AI disabled.
pub(crate) fn on_vibration(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    received: ReceivedVibration,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let no_ai = data.is_no_ai();
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    state.vibration_cooldown = Some(40);
    let recent_projectile = state.recent_projectile.is_some();
    if received.projectile_owner.is_some() {
        state.recent_projectile = Some(RECENT_PROJECTILE_TICKS);
    }
    let mut suspicious_pos = received.pos;
    let attributed = store.mob_ai(entity.id()).and_then(|runtime| {
        let targets = runtime.target_view(entity, store, level);
        let (target, amount) = if let Some(owner) = received.projectile_owner {
            let target = targets.by_uuid(owner)?;
            if targets.body().distance_to_facts_sqr(&target) >= 30.0 * 30.0 {
                return None;
            }
            if recent_projectile && conditions::is_valid_target(&targets, &target) {
                suspicious_pos = target.block_position();
            }
            (target, if recent_projectile { 35 } else { 10 })
        } else {
            (targets.by_uuid(received.source?)?, 35)
        };
        conditions::is_valid_target(&targets, &target).then_some((target.target_ref(), amount))
    });
    if let Some((target, amount)) = attributed.filter(|_| !no_ai) {
        let anger = increase_anger_at(entity, store, level, target, amount);
        play_listening_sound(entity, store, anger);
    }
    let investigate = store.mob_ai(entity.id()).is_some_and(|runtime| {
        let targets = runtime.target_view(entity, store, level);
        let EntityData::GeneralLiving(data) = entity.data() else {
            return false;
        };
        let state = data.warden_state().expect("warden state");
        state.active_anger(&targets, runtime.target_ref()) < WardenState::ANGRY_THRESHOLD
            && (received.projectile_owner.is_some()
                || state
                    .active_suspect(&targets)
                    .is_none_or(|target| Some(target.uuid) == received.source))
    });
    if investigate {
        super::awareness::set_disturbance(entity, store, level, suspicious_pos);
    }
}

pub(super) fn play_listening_sound(entity: &Entity, store: &mut EntityStore, client_anger: i32) {
    if entity.pose_id() != VANILLA_ENTITY_POSE_ROARING_ID {
        let pitch = store
            .mob_ai_mut(entity.id())
            .map(|runtime| runtime.voice_pitch(false));
        if let Some(pitch) = pitch.filter(|_| !entity.is_silent()) {
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position: entity.position(),
                resource_id: if client_anger >= 40 {
                    "minecraft:entity.warden.listening_angry"
                } else {
                    "minecraft:entity.warden.listening"
                }
                .into(),
                source: crate::entity::living::living_sound_source(entity),
                volume: 10.0,
                pitch,
            });
        }
    }
}

// Warden.java:489-501: anger follows the hurt attempt even when ordinary damage was refused.
pub(crate) fn after_hurt(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WARDEN_ID
        || matches!(
            entity.pose_id(),
            VANILLA_ENTITY_POSE_DIGGING_ID | VANILLA_ENTITY_POSE_EMERGING_ID
        )
        || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return;
    }
    let Some(attacker) = source.causing_entity else {
        return;
    };
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    let targets = runtime
        .target_view(entity, store, level)
        .with_detached(level.detached_entity(attacker.entity_id()));
    let Some(attacker) = targets
        .facts(attacker.entity_id())
        .filter(|target| conditions::is_valid_target(&targets, target))
    else {
        return;
    };
    let target = attacker.target_ref();
    let direct_or_near = source.direct_entity_id == Some(target.entity_id)
        || targets.body().distance_to_facts_sqr(&attacker) < 5.0 * 5.0;
    increase_anger_at(
        entity,
        store,
        level,
        target,
        WardenState::ANGRY_THRESHOLD + 20,
    );
    let direct_attack = direct_or_near
        && (entity.pose_id() == VANILLA_ENTITY_POSE_ROARING_ID
            || store
                .mob_ai(entity.id())
                .is_some_and(|runtime| runtime.target_ref().is_none()));
    if direct_attack {
        if let Some(runtime) = store.mob_ai_mut(entity.id()) {
            runtime.target_state_mut().target = Some(target);
        }
        super::sonic_boom::target_set(entity);
        entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
        sync_client_anger(entity, store, level);
    }
}

// Warden.java:523-531: contact always starts its cooldown, including an invalid suspect.
pub(crate) fn on_push(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    other_id: i32,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data.is_no_ai() {
        return;
    }
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    if state.touch_cooldown.is_some() {
        return;
    }
    state.touch_cooldown = Some(20);
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    let targets = runtime
        .target_view(entity, store, level)
        .with_detached(level.detached_entity(other_id));
    let Some(target) = targets
        .facts(other_id)
        .filter(|target| conditions::is_valid_target(&targets, target))
    else {
        return;
    };
    let target = target.target_ref();
    let client_anger = increase_anger_at(entity, store, level, target, 35);
    play_listening_sound(entity, store, client_anger);
}

// Warden.java:445-452: newly angry players can replace a non-player attack target.
pub(super) fn increase_anger_at(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
    amount: i32,
) -> i32 {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return 0;
    };
    let state = data.warden_state_mut().expect("warden state");
    state.increase(target.uuid, amount);
    if state.anger(target.uuid) >= WardenState::ANGRY_THRESHOLD {
        let claims_target = store.mob_ai(entity.id()).is_some_and(|runtime| {
            let targets = runtime
                .target_view(entity, store, level)
                .with_detached(level.detached_entity(target.entity_id));
            targets
                .facts_of(target)
                .is_some_and(|target| target.is_player())
                && runtime
                    .target_ref()
                    .and_then(|target| targets.facts_of(target))
                    .is_none_or(|target| !target.is_player())
        });
        if claims_target {
            if let Some(runtime) = store.mob_ai_mut(entity.id()) {
                runtime.target_state_mut().target = None;
            }
        }
    }
    sync_client_anger(entity, store, level)
}

fn sync_client_anger(entity: &mut Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> i32 {
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return 0;
    };
    let targets = runtime.target_view(entity, store, level);
    let EntityData::GeneralLiving(data) = entity.data() else {
        return 0;
    };
    let state = data.warden_state().expect("warden state");
    let anger = state.active_anger(&targets, runtime.target_ref());
    let changed = state.client_anger != anger;
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.warden_state_mut().expect("warden state").client_anger = anger;
    }
    if changed {
        entity.mark_synched_dirty(VANILLA_WARDEN_ANGER_LEVEL_DATA_ID);
    }
    anger
}
