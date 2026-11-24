//! Sniffer searching, digging and gift loot.

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::navigation::MobPathingSnapshot;
use crate::entity::ai::mob::random_pos::{land_random_pos, RandomPosMob};
use crate::entity::ai::{MemoryGlobalPos, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::level::block_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
};
use bbb_protocol::entity_synched_data::*;
use bbb_protocol::packets::{
    BlockPos, EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind, EntityEvent,
    SoundSource, Vec3d,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Scenting,
    Sniffing,
    Searching,
    Digging,
    Rising,
    Happy,
}

impl Phase {
    fn id(self) -> i32 {
        match self {
            Self::Idle => VANILLA_SNIFFER_STATE_IDLING_ID,
            Self::Scenting => VANILLA_SNIFFER_STATE_SCENTING_ID,
            Self::Sniffing => VANILLA_SNIFFER_STATE_SNIFFING_ID,
            Self::Searching => VANILLA_SNIFFER_STATE_SEARCHING_ID,
            Self::Digging => VANILLA_SNIFFER_STATE_DIGGING_ID,
            Self::Rising => VANILLA_SNIFFER_STATE_RISING_ID,
            Self::Happy => VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnifferState {
    phase: Phase,
    ticks: u16,
    drop_seed_at_tick: i32,
}

impl SnifferState {
    pub(crate) fn new() -> Self {
        Self {
            phase: Phase::Idle,
            ticks: 20,
            drop_seed_at_tick: 0,
        }
    }

    pub(crate) fn is_digging(self) -> bool {
        self.phase == Phase::Digging
    }

    // Sniffer.java:402-404.
    pub(crate) fn ambient_sound(self) -> Option<&'static str> {
        (!matches!(self.phase, Phase::Digging | Phase::Searching))
            .then_some("minecraft:entity.sniffer.idle")
    }

    pub(crate) fn synched_values(self) -> [SynchedValue; 2] {
        [
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_SNIFFER_STATE_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
                    value: EntityDataValueKind::EnumId {
                        serializer: EntityDataEnumSerializer::SnifferState,
                        id: self.phase.id(),
                    },
                },
                is_default: self.phase == Phase::Idle,
            },
            SynchedValue {
                value: EntityDataValue {
                    data_id: VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(self.drop_seed_at_tick),
                },
                is_default: self.drop_seed_at_tick == 0,
            },
        ]
    }
}

fn state(entity: &Entity) -> Option<SnifferState> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.sniffer_state(),
        _ => None,
    }
}

/// The states `Sniffer.canMate` admits (`Sniffer.java:427-434`).
pub(crate) fn can_mate_in_state(entity: &Entity) -> bool {
    state(entity)
        .is_some_and(|state| matches!(state.phase, Phase::Idle | Phase::Scenting | Phase::Happy))
}

#[cfg(test)]
pub(crate) fn set_digging_for_test(entity: &mut Entity) {
    state_mut(entity).phase = Phase::Digging;
}

pub(crate) fn allows_idle_stroll(entity: &Entity) -> bool {
    state(entity).is_none_or(|state| state.phase == Phase::Idle)
}

fn state_mut(entity: &mut Entity) -> &mut SnifferState {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!("a sniffer is living")
    };
    data.sniffer_state_mut().expect("sniffer state")
}

fn sound(entity: &Entity, store: &mut EntityStore, name: &str, pitch: f32) {
    if !entity.is_silent() {
        store.play_sound_from_entity_at(
            entity.id(),
            entity.position(),
            name,
            SoundSource::Neutral,
            1.0,
            pitch,
        );
    }
}

fn transition(entity: &mut Entity, store: &mut EntityStore, phase: Phase, ticks: u16) {
    let old = state(entity).expect("sniffer state").phase;
    let drop_seed_at_tick = entity.tick_count().wrapping_add(120);
    let next = state_mut(entity);
    next.phase = phase;
    next.ticks = ticks;
    if phase == Phase::Digging {
        // Sniffer.java:243-245: the synchronized deadline drives the seed drop.
        next.drop_seed_at_tick = drop_seed_at_tick;
        entity.mark_synched_dirty(VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID);
        store.broadcast_to_tracking(
            entity.id(),
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: entity.id(),
                event_id: VANILLA_SNIFFER_DIGGING_SOUND_EVENT_ID,
            }),
        );
    }
    if old != phase {
        entity.mark_synched_dirty(VANILLA_SNIFFER_STATE_DATA_ID);
        crate::entity::living::refresh_dimensions(entity);
    }
    match phase {
        Phase::Scenting => sound(
            entity,
            store,
            "minecraft:entity.sniffer.scenting",
            if is_baby(entity) { 1.3 } else { 1.0 },
        ),
        Phase::Sniffing => sound(entity, store, "minecraft:entity.sniffer.sniffing", 1.0),
        Phase::Rising => sound(entity, store, "minecraft:entity.sniffer.digging_stop", 1.0),
        Phase::Happy => sound(entity, store, "minecraft:entity.sniffer.happy", 1.0),
        _ => {}
    }
}

fn is_baby(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby())
}

fn head_block(entity: &Entity) -> BlockPos {
    // Sniffer.java:142-148.
    let forward = crate::entity::ai::mob::stroll::view_vector(entity.x_rot(), entity.y_rot());
    let position = entity.position();
    BlockPos {
        x: (position.x + forward.x * 2.25).floor() as i32,
        y: (position.y + f64::from(0.2_f32)).floor() as i32,
        z: (position.z + forward.z * 2.25).floor() as i32,
    }
}

fn can_work(entity: &Entity, store: &EntityStore) -> bool {
    !entity.is_in_water()
        && entity.on_ground()
        && store.vehicle(entity.id()).is_none()
        && !crate::entity::animal_breeding::is_in_love(entity)
        && store
            .mob_ai(entity.id())
            .is_some_and(|runtime| !runtime.is_panicking() && !runtime.tempt_goal_running())
}

fn can_sniff(entity: &Entity, store: &EntityStore) -> bool {
    // Sniffer.java:134-135.
    can_work(entity, store) && !crate::entity::leash::is_leashed(entity)
}

fn valid_soil(entity: &Entity, level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.has_ticking_column_at(pos)
        && level.world_border().is_some_and(|border| border.contains_block_at(pos, level.game_time()))
        && block_state_tag_membership(level, level.block_state(pos), "minecraft:sniffer_diggable_block")
        && !crate::entity::spawner_brain_memory(entity, MemoryModuleType::SnifferExploredPositions)
            .is_some_and(|memory| matches!(&memory.value, MemoryValue::GlobalPosList(positions)
                if positions.iter().any(|prior| prior.dimension == level.dimension_key().as_str() && prior.pos == pos)))
}

fn pathing(entity: &Entity, store: &EntityStore) -> Option<MobPathingSnapshot> {
    MobPathingSnapshot::from_entity(
        entity,
        false,
        store.vehicle(entity.id()).is_some(),
        store.difficulty(),
        store.mob_ai(entity.id())?.pathfinding_malus(),
    )
}

fn reachable(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> bool {
    let Some(mob) = pathing(entity, store) else {
        return false;
    };
    let range = entity.living().map_or(0.0, |living| {
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
    }) as f32;
    store
        .mob_ai(entity.id())
        .is_some_and(|runtime| runtime.ground_path_reaches(level, mob, pos, 1, range) == Some(true))
}

fn find_dig_target(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<BlockPos> {
    let malus = store.effective_pathfinding_malus(entity, level)?;
    // Sniffer.java:257-265.
    for radius in [10, 12, 14, 16, 18] {
        let Some(position) = land_random_pos(
            level,
            level.level_random(),
            RandomPosMob::of(entity, malus),
            radius,
            3,
        )?
        else {
            continue;
        };
        let soil = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32 - 1,
            z: position.z.floor() as i32,
        };
        if valid_soil(entity, level, soil) && reachable(entity, store, level, soil) {
            return Some(soil);
        }
    }
    None
}

fn remember_explored(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    // Sniffer.java:249-252,322-326: keep twenty old positions, then prepend.
    let mut positions = match crate::entity::spawner_brain_memory(
        entity,
        MemoryModuleType::SnifferExploredPositions,
    ) {
        Some(PackedMemory {
            value: MemoryValue::GlobalPosList(positions),
            ..
        }) => positions.iter().take(20).cloned().collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    positions.insert(
        0,
        MemoryGlobalPos {
            dimension: level.dimension_key().as_str().to_owned(),
            pos: entity.get_on_pos(),
        },
    );
    crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::SnifferExploredPositions,
            value: MemoryValue::GlobalPosList(positions),
            time_to_live: None,
        },
    );
}

pub(crate) fn custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let Some(before) = state(entity) else {
        return;
    };
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.retained_brain_mut().memories.retain_mut(|entry| {
            if entry.memory_type == MemoryModuleType::SniffCooldown {
                if let Some(ttl) = &mut entry.time_to_live {
                    *ttl = ttl.saturating_sub(1);
                    return *ttl > 0;
                }
            }
            true
        });
    }
    let interrupted = match before.phase {
        Phase::Rising | Phase::Happy => store
            .mob_ai(entity.id())
            .is_some_and(|runtime| runtime.is_panicking()),
        Phase::Digging => false,
        _ => !can_sniff(entity, store),
    };
    if interrupted {
        if before.phase != Phase::Idle {
            store.stop_mob_navigation(entity.id());
            transition(entity, store, Phase::Idle, 20);
        }
        return;
    }
    let next = state_mut(entity);
    next.ticks = next.ticks.saturating_sub(1);
    let elapsed = next.ticks == 0;
    match before.phase {
        Phase::Idle if elapsed => {
            let ticks = (40 + level.level_random().next_int(41)) as u16;
            let phase = if !is_baby(entity)
                && crate::entity::spawner_brain_memory(entity, MemoryModuleType::SniffCooldown)
                    .is_none()
                && level.level_random().next_boolean()
            {
                Phase::Sniffing
            } else {
                Phase::Scenting
            };
            store.stop_mob_navigation(entity.id());
            transition(entity, store, phase, ticks);
        }
        Phase::Scenting | Phase::Happy if elapsed => transition(
            entity,
            store,
            Phase::Idle,
            (5 + level.level_random().next_int(16)) as u16,
        ),
        Phase::Sniffing if elapsed => {
            let target = find_dig_target(entity, store, level);
            let moved = target
                .zip(pathing(entity, store))
                .is_some_and(|(soil, mob)| {
                    store.mob_ai_mut(entity.id()).unwrap().navigate_to(
                        level,
                        mob,
                        BlockPos {
                            y: soil.y + 1,
                            ..soil
                        },
                        0,
                        1.25,
                    )
                });
            if moved {
                transition(entity, store, Phase::Searching, 600);
            } else {
                transition(entity, store, Phase::Idle, 20);
            }
        }
        Phase::Searching => {
            let stopped = store
                .mob_ai(entity.id())
                .is_none_or(|runtime| runtime.ground_path().is_none_or(|path| path.is_done()));
            if elapsed || stopped {
                let head = head_block(entity);
                let soil = BlockPos {
                    y: head.y - 1,
                    ..head
                };
                store.stop_mob_navigation(entity.id());
                if !is_baby(entity)
                    && valid_soil(entity, level, soil)
                    && reachable(entity, store, level, soil)
                {
                    transition(
                        entity,
                        store,
                        Phase::Digging,
                        (160 + level.level_random().next_int(21)) as u16,
                    );
                } else {
                    transition(entity, store, Phase::Idle, 20);
                }
            }
        }
        Phase::Digging => {
            let head = head_block(entity);
            let soil = BlockPos {
                y: head.y - 1,
                ..head
            };
            if is_baby(entity)
                || !can_work(entity, store)
                || !valid_soil(entity, level, soil)
                || !reachable(entity, store, level, soil)
            {
                transition(entity, store, Phase::Idle, 20);
            } else if elapsed {
                // SnifferAi.java:163-166.
                crate::entity::set_spawner_brain_memory(
                    entity,
                    PackedMemory {
                        memory_type: MemoryModuleType::SniffCooldown,
                        value: MemoryValue::Unit,
                        time_to_live: Some(9600),
                    },
                );
                transition(entity, store, Phase::Rising, 40);
            }
        }
        Phase::Rising if elapsed => {
            remember_explored(entity, level);
            transition(
                entity,
                store,
                Phase::Happy,
                (40 + level.level_random().next_int(61)) as u16,
            );
        }
        _ => {}
    }
    if matches!(
        state(entity).unwrap().phase,
        Phase::Sniffing | Phase::Digging | Phase::Rising | Phase::Scenting | Phase::Happy
    ) {
        store.stop_mob_navigation(entity.id());
        if let Some(living) = entity.living_mut() {
            crate::entity::living::apply_immobile(living);
        }
    }
}

pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let Some(current) = state(entity) else {
        return;
    };
    if !entity.is_alive_with_health() {
        if current.phase != Phase::Idle {
            transition(entity, store, Phase::Idle, 20);
        }
        return;
    }
    if current.phase != Phase::Digging {
        return;
    }
    let head = head_block(entity);
    if entity.tick_count() % 10 == 0 {
        level.game_event(VanillaGameEvent::ENTITY_ACTION, head, Some(entity.id()));
    }
    // Sniffer.java:284-294: this drop is independent of AI and mob_drops.
    if entity.tick_count() == current.drop_seed_at_tick {
        crate::entity::gift_loot::drop_gift(
            entity,
            store,
            level,
            "minecraft:gameplay/sniffer_digging",
            Vec3d {
                x: f64::from(head.x),
                y: f64::from(head.y),
                z: f64::from(head.z),
            },
            true,
        );
        sound(entity, store, "minecraft:entity.sniffer.drop_seed", 1.0);
    }
}
