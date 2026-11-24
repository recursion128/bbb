//! Sonic charge, sound delay and recovery on the warden's real state.

use bbb_protocol::packets::{EntityEvent, SoundSource, Vec3d};

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore};

// SonicBoom.java:22-25.
pub(crate) const DURATION: i64 = 60;
const CHARGE_DELAY: i64 = 34;
const RECOVERY: i64 = 40;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SonicBoomState {
    cooldown: Option<i64>,
    sound_delay: Option<i64>,
    sound_cooldown: Option<i64>,
}

impl SonicBoomState {
    pub(crate) fn ready(&self) -> bool {
        self.cooldown.is_none()
    }

    pub(crate) fn waiting_for_boom(&self) -> bool {
        self.sound_delay.is_none() && self.sound_cooldown.is_none()
    }

    pub(crate) fn tick(&mut self) {
        super::anger::tick_cooldown(&mut self.cooldown);
        super::anger::tick_cooldown(&mut self.sound_delay);
        super::anger::tick_cooldown(&mut self.sound_cooldown);
    }

    pub(crate) fn take_memories(&mut self, brain: &mut BrainPacked) {
        *self = Self::default();
        let mut index = 0;
        while index < brain.memories.len() {
            let cooldown = match brain.memories[index].memory_type {
                MemoryModuleType::SonicBoomCooldown => &mut self.cooldown,
                MemoryModuleType::SonicBoomSoundDelay => &mut self.sound_delay,
                MemoryModuleType::SonicBoomSoundCooldown => &mut self.sound_cooldown,
                _ => {
                    index += 1;
                    continue;
                }
            };
            let memory = brain.memories.remove(index);
            *cooldown = Some(memory.time_to_live.unwrap_or(i64::MAX));
        }
    }

    pub(crate) fn write_memories(&self, brain: &mut BrainPacked) {
        for (memory_type, cooldown) in [
            (MemoryModuleType::SonicBoomCooldown, self.cooldown),
            (MemoryModuleType::SonicBoomSoundDelay, self.sound_delay),
            (
                MemoryModuleType::SonicBoomSoundCooldown,
                self.sound_cooldown,
            ),
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

    // Warden.java:504-509.
    pub(crate) fn target_set(&mut self) {
        self.cooldown = Some(200);
    }

    pub(crate) fn charge(&mut self) {
        self.sound_delay = Some(CHARGE_DELAY);
    }

    pub(crate) fn fired(&mut self) {
        self.sound_cooldown = Some(DURATION - CHARGE_DELAY);
    }

    pub(crate) fn recover(&mut self) {
        self.cooldown = Some(RECOVERY);
    }
}

// SonicBoom.java:19-20; Entity.java:450-454.
pub(crate) fn within_range(source: Vec3d, target: Vec3d) -> bool {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let dz = target.z - source.z;
    dx * dx + dz * dz < 15.0 * 15.0 && dy * dy < 20.0 * 20.0
}

fn state_mut(entity: &mut Entity) -> Option<&mut SonicBoomState> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    Some(&mut data.warden_state_mut()?.sonic)
}

pub(crate) fn target_set(entity: &mut Entity) {
    if let Some(state) = state_mut(entity) {
        state.target_set();
    }
}

// SonicBoom.java:51-56.
pub(crate) fn charge(entity: &mut Entity, store: &mut EntityStore) {
    let Some(state) = state_mut(entity) else {
        return;
    };
    state.charge();
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: 62,
        }),
    );
    if !entity.is_silent() {
        store.play_entity_sound(crate::entity::PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.warden.sonic_charge".to_owned(),
            source: SoundSource::Hostile,
            volume: 3.0,
            pitch: 1.0,
        });
    }
}

pub(crate) fn fire(entity: &mut Entity, store: &mut EntityStore, target_id: i32) {
    let Some(state) = state_mut(entity) else {
        return;
    };
    state.fired();
    store.queue_command(EntityCommand::WardenSonicBoom {
        source_entity_id: entity.id(),
        target_entity_id: target_id,
    });
}

pub(crate) fn recover(entity: &mut Entity) {
    if let Some(state) = state_mut(entity) {
        state.recover();
    }
}
