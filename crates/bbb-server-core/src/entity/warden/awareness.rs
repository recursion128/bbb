//! Warden sniff memories and temporary disturbance locations.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SNIFFING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::packets::BlockPos;

use crate::entity::ai::mob::target::{conditions, TargetRef, TargetView};
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AwarenessState {
    sniffing: Option<i64>,
    sniff_cooldown: Option<i64>,
    disturbance: Option<(BlockPos, i64)>,
}

impl AwarenessState {
    pub(crate) fn is_sniffing(&self) -> bool {
        self.sniffing.is_some()
    }

    pub(crate) fn can_sniff(&self) -> bool {
        self.sniff_cooldown.is_none()
    }

    pub(crate) fn disturbance(&self) -> Option<BlockPos> {
        self.disturbance.map(|(pos, _)| pos)
    }

    pub(super) fn tick(&mut self) {
        super::anger::tick_cooldown(&mut self.sniffing);
        super::anger::tick_cooldown(&mut self.sniff_cooldown);
        if let Some((_, ttl)) = &mut self.disturbance {
            *ttl -= 1;
            if *ttl <= 0 {
                self.disturbance = None;
            }
        }
    }

    pub(super) fn restore_command_transients(&mut self, previous: &Self) {
        self.disturbance = previous.disturbance;
    }

    pub(super) fn take_memories(&mut self, brain: &mut BrainPacked) {
        self.sniffing = None;
        self.sniff_cooldown = None;
        let mut index = 0;
        while index < brain.memories.len() {
            let cooldown = match brain.memories[index].memory_type {
                MemoryModuleType::IsSniffing => &mut self.sniffing,
                MemoryModuleType::SniffCooldown => &mut self.sniff_cooldown,
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
            (MemoryModuleType::IsSniffing, self.sniffing),
            (MemoryModuleType::SniffCooldown, self.sniff_cooldown),
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
}

// WardenEntitySensor.java:24-40; NearestLivingEntitySensor.java:18-21.
pub(crate) fn nearest_attackable(targets: &TargetView<'_, '_, '_>) -> Option<TargetRef> {
    let body = targets.body();
    let range = targets
        .body_entity()
        .living()?
        .attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let bounds = body.bounding_box().inflate(range, range, range);
    targets
        .local_living()
        .filter(|target| {
            target.alive()
                && target.bounding_box().intersects(bounds)
                && conditions::is_valid_target(targets, target)
        })
        .min_by(|a, b| {
            b.is_player().cmp(&a.is_player()).then_with(|| {
                body.distance_to_facts_sqr(a)
                    .total_cmp(&body.distance_to_facts_sqr(b))
            })
        })
        .map(|target| target.target_ref())
}

pub(crate) fn begin_sniff(entity: &mut Entity, cooldown: i64) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    state.awareness.sniffing = Some(i64::MAX);
    state.awareness.sniff_cooldown = Some(cooldown);
}

// Sniffing.java:47-62.
pub(crate) fn end_sniff(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let no_ai = data.is_no_ai();
    let Some(state) = data.warden_state_mut() else {
        return;
    };
    state.awareness.sniffing = None;
    if entity.pose_id() == VANILLA_ENTITY_POSE_SNIFFING_ID {
        entity.set_pose_id(VANILLA_ENTITY_POSE_STANDING_ID);
    }
    if no_ai {
        return;
    }
    let Some((target, pos, nearby)) = store.mob_ai(entity.id()).and_then(|runtime| {
        let targets = runtime.target_view(entity, store, level);
        let target = targets.facts_of(nearest_attackable(&targets)?)?;
        let source = entity.position();
        let position = target.position();
        let dx = source.x - position.x;
        let dy = source.y - position.y;
        let dz = source.z - position.z;
        Some((
            target.target_ref(),
            target.block_position(),
            dx * dx + dz * dz < 6.0 * 6.0 && dy * dy < 20.0 * 20.0,
        ))
    }) else {
        return;
    };
    let has_disturbance = matches!(entity.data(), EntityData::GeneralLiving(data) if data.warden_state().is_some_and(|state| state.awareness.disturbance().is_some()));
    if nearby {
        let anger = super::anger::increase_anger_at(entity, store, level, target, 35);
        super::anger::play_listening_sound(entity, store, anger);
    }
    if !has_disturbance {
        set_disturbance(entity, store, level, pos);
    }
}

// WardenAi.java:171-180.
pub(super) fn set_disturbance(
    entity: &mut Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    position: BlockPos,
) {
    if !level
        .world_border()
        .is_some_and(|border| border.contains_block_at(position, level.game_time()))
    {
        return;
    }
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    if runtime.target_ref().is_some() {
        return;
    }
    let targets = runtime.target_view(entity, store, level);
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if data
        .warden_state()
        .is_none_or(|state| state.angry_target(&targets).is_some())
    {
        return;
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        let state = data.warden_state_mut().expect("warden state");
        state.lifecycle.refresh_dig_cooldown();
        let awareness = &mut state.awareness;
        awareness.sniff_cooldown = Some(100);
        awareness.disturbance = Some((position, 100));
    }
}
