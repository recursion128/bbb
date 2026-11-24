//! Warden emergence memories and departure actions.

use bbb_protocol::entity_poses::{VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID};
use bbb_protocol::packets::SoundSource;

use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityData, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;

// WardenAi.java:48-52.
const EMERGE_DURATION: i64 = 134;
const DIG_COOLDOWN: i64 = 1200;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LifecycleState {
    dig_cooldown: Option<i64>,
    emerging: Option<i64>,
}

impl LifecycleState {
    pub(crate) fn is_emerging(&self) -> bool {
        self.emerging.is_some()
    }

    pub(crate) fn can_dig(&self) -> bool {
        self.dig_cooldown.is_none()
    }

    pub(super) fn tick(&mut self) {
        super::anger::tick_cooldown(&mut self.dig_cooldown);
        super::anger::tick_cooldown(&mut self.emerging);
    }

    pub(super) fn take_memories(&mut self, brain: &mut BrainPacked) {
        *self = Self::default();
        let mut index = 0;
        while index < brain.memories.len() {
            let cooldown = match brain.memories[index].memory_type {
                MemoryModuleType::DigCooldown => &mut self.dig_cooldown,
                MemoryModuleType::IsEmerging => &mut self.emerging,
                _ => {
                    index += 1;
                    continue;
                }
            };
            let memory = brain.memories.remove(index);
            *cooldown = Some(memory.time_to_live.unwrap_or(i64::MAX));
        }
    }

    pub(super) fn write_memories(&self, brain: &mut BrainPacked) {
        for (memory_type, cooldown) in [
            (MemoryModuleType::DigCooldown, self.dig_cooldown),
            (MemoryModuleType::IsEmerging, self.emerging),
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

    // WardenAi.java:165-168.
    pub(super) fn refresh_dig_cooldown(&mut self) {
        if self.dig_cooldown.is_some() {
            self.dig_cooldown = Some(DIG_COOLDOWN);
        }
    }
}

// Warden.java:475-485.
pub(crate) fn finalize_spawn(entity: &mut Entity, store: &mut EntityStore, triggered: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    state.lifecycle.dig_cooldown = Some(DIG_COOLDOWN);
    if triggered {
        state.lifecycle.emerging = Some(EMERGE_DURATION);
        entity.set_pose_id(VANILLA_ENTITY_POSE_EMERGING_ID);
        play_sound(entity, store, "minecraft:entity.warden.agitated");
    }
}

// Digging.java:26-40.
pub(crate) fn begin_dig(
    entity: &mut Entity,
    store: &mut EntityStore,
    _level: &mut LevelCtx<'_, '_>,
) -> bool {
    if entity.on_ground() {
        entity.set_pose_id(VANILLA_ENTITY_POSE_DIGGING_ID);
        play_sound(entity, store, "minecraft:entity.warden.dig");
    } else {
        play_sound(entity, store, "minecraft:entity.warden.agitated");
        entity.discard();
    }
    entity.is_alive()
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str) {
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.into(),
            source: SoundSource::Hostile,
            volume: 5.0,
            pitch: 1.0,
        });
    }
}
