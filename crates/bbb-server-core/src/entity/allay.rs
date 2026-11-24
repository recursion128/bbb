//! Allay item interest and delivery memories on the retained brain.

pub(crate) mod delivery;

use bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::ai::mob::target::TargetView;
use crate::entity::ai::{MemoryGlobalPos, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::game_event_listener::AllayJukeboxState;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DepositTarget {
    pub(crate) position: Vec3d,
    pub(crate) block_pos: BlockPos,
    pub(crate) liked_player: Option<Uuid>,
}

fn state(entity: &Entity) -> Option<&AllayJukeboxState> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.game_event_listener_state()?.allay.as_ref()
}

pub(crate) fn is_dancing(entity: &Entity) -> bool {
    state(entity).is_some_and(|state| state.dancing)
}

pub(crate) fn can_duplicate(entity: &Entity) -> bool {
    state(entity).is_some_and(AllayJukeboxState::can_duplicate)
}

// Allay.java:453-456.
fn set_duplication_cooldown(entity: &mut Entity, cooldown: i64) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data
        .game_event_listener_state_mut()
        .and_then(|listener| listener.allay.as_mut())
    else {
        return;
    };
    let before = state.can_duplicate();
    state.duplication_cooldown = cooldown;
    if state.can_duplicate() != before {
        entity.mark_synched_dirty(VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID);
    }
}

// Allay.java:469-471.
pub(crate) fn reset_duplication_cooldown(entity: &mut Entity) {
    set_duplication_cooldown(entity, 6000);
}

// Allay.java:447-451.
pub(super) fn tick_duplication_cooldown(entity: &mut Entity) {
    if let Some(cooldown) = state(entity)
        .map(|state| state.duplication_cooldown)
        .filter(|cooldown| *cooldown > 0)
    {
        set_duplication_cooldown(entity, cooldown - 1);
    }
}

fn memory(entity: &Entity, kind: MemoryModuleType) -> Option<&MemoryValue> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID {
        return None;
    }
    Some(&crate::entity::general_living::spawner_brain_memory(entity, kind)?.value)
}

fn set_memory(entity: &mut Entity, kind: MemoryModuleType, value: Option<MemoryValue>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let memories = &mut data.retained_brain_mut().memories;
    if let Some(value) = value {
        if let Some(memory) = memories
            .iter_mut()
            .find(|memory| memory.memory_type == kind)
        {
            memory.value = value;
            memory.time_to_live = None;
        } else {
            memories.push(PackedMemory {
                memory_type: kind,
                value,
                time_to_live: None,
            });
        }
    } else {
        memories.retain(|memory| memory.memory_type != kind);
    }
}

pub(crate) fn liked_player(entity: &Entity) -> Option<Uuid> {
    match memory(entity, MemoryModuleType::LikedPlayer)? {
        MemoryValue::Uuid(uuid) => Some(*uuid),
        _ => None,
    }
}

pub(crate) fn remember_player(entity: &mut Entity, player: Option<Uuid>) {
    set_memory(
        entity,
        MemoryModuleType::LikedPlayer,
        player.map(MemoryValue::Uuid),
    );
}

pub(crate) fn pickup_on_cooldown(entity: &Entity) -> bool {
    memory(entity, MemoryModuleType::ItemPickupCooldownTicks).is_some()
}

// GoAndGiveItemsToTarget.java:69-75.
pub(crate) fn on_item_thrown(entity: &mut Entity) {
    set_memory(
        entity,
        MemoryModuleType::ItemPickupCooldownTicks,
        Some(MemoryValue::Int(60)),
    );
}

// AllayAi.java:96-105.
pub(crate) fn hear_noteblock(entity: &mut Entity, level: &LevelCtx<'_, '_>, pos: BlockPos) {
    let matches = match memory(entity, MemoryModuleType::LikedNoteblockPosition) {
        None => {
            set_memory(
                entity,
                MemoryModuleType::LikedNoteblockPosition,
                Some(MemoryValue::GlobalPos(MemoryGlobalPos {
                    dimension: level.dimension_key().resource_id().into(),
                    pos,
                })),
            );
            true
        }
        Some(MemoryValue::GlobalPos(liked)) => {
            liked.dimension == level.dimension_key().resource_id() && liked.pos == pos
        }
        Some(_) => false,
    };
    if matches {
        set_memory(
            entity,
            MemoryModuleType::LikedNoteblockCooldownTicks,
            Some(MemoryValue::Int(600)),
        );
    }
}

// AllayAi.java:128-133; GlobalPos.java:31-33.
fn candidate_noteblock(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<BlockPos> {
    let MemoryValue::GlobalPos(liked) = memory(entity, MemoryModuleType::LikedNoteblockPosition)?
    else {
        return None;
    };
    let here = entity.block_position();
    let nearby = [
        i64::from(liked.pos.x) - i64::from(here.x),
        i64::from(liked.pos.y) - i64::from(here.y),
        i64::from(liked.pos.z) - i64::from(here.z),
    ]
    .into_iter()
    .all(|delta| delta.abs() <= 1024);
    (liked.dimension == level.dimension_key().resource_id()
        && nearby
        && memory(entity, MemoryModuleType::LikedNoteblockCooldownTicks).is_some())
    .then_some(liked.pos)
}

fn valid_noteblock(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<BlockPos> {
    candidate_noteblock(entity, level).filter(|pos| {
        level
            .block_state_checked(*pos)
            .and_then(bbb_server_types::block_queries::block_type_of)
            == Some(VANILLA_BLOCK_NOTE_BLOCK_ID)
    })
}

pub(crate) fn before_ai(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    data.retained_brain_mut().memories.retain_mut(|memory| {
        if !matches!(
            memory.memory_type,
            MemoryModuleType::LikedPlayer
                | MemoryModuleType::LikedNoteblockPosition
                | MemoryModuleType::LikedNoteblockCooldownTicks
                | MemoryModuleType::ItemPickupCooldownTicks
        ) {
            return true;
        }
        if let Some(ttl) = &mut memory.time_to_live {
            if *ttl != i64::MAX {
                *ttl = ttl.saturating_sub(1);
                if *ttl <= 0 {
                    return false;
                }
            }
        }
        // CountDownCooldownTicks.java:34-41: integer counters are separate from memory expiry.
        if matches!(
            memory.memory_type,
            MemoryModuleType::LikedNoteblockCooldownTicks
                | MemoryModuleType::ItemPickupCooldownTicks
        ) {
            if let MemoryValue::Int(ticks) = &mut memory.value {
                *ticks = ticks.saturating_sub(1);
                return *ticks > 0;
            }
        }
        true
    });
    if memory(entity, MemoryModuleType::LikedNoteblockPosition).is_some() {
        let invalid = candidate_noteblock(entity, level).is_none_or(|pos| {
            level.block_state_checked(pos).is_some_and(|state| {
                bbb_server_types::block_queries::block_type_of(state)
                    != Some(VANILLA_BLOCK_NOTE_BLOCK_ID)
            })
        });
        if invalid {
            set_memory(entity, MemoryModuleType::LikedNoteblockPosition, None);
        }
    }
}

// Allay.java:340-356.
pub(crate) fn wants_to_pick_up(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID
        || stack.is_empty()
        || !level.mob_griefing()
    {
        return false;
    }
    let Some(sample) = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .filter(|sample| !sample.is_empty())
    else {
        return false;
    };
    if sample.item_id() != stack.item_id() {
        return false;
    }
    let resolver = store.item_resolver();
    let potions_equal = match (
        sample.effective_component(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, resolver),
        stack.effective_component(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, resolver),
    ) {
        (Ok(None), Ok(None)) => true,
        (Ok(Some(left)), Ok(Some(right))) => {
            bbb_protocol::packets::data_components::fields::component_values_equal(
                VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                &left,
                &right,
            )
            .unwrap_or(false)
        }
        _ => false,
    };
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    potions_equal
        && data
            .internal_inventory()
            .is_some_and(|inventory| inventory.can_add_item(resolver, stack).unwrap_or(false))
}

// AllayAi.java:108-147; GameType.java:90-91.
pub(crate) fn deposit_target(targets: &TargetView<'_, '_, '_>) -> Option<DepositTarget> {
    let body = targets.body_entity();
    let player = liked_player(body)
        .and_then(|uuid| targets.player_by_uuid(uuid))
        .filter(|player| {
            player.is_local()
                && !player.spectator()
                && targets.body().distance_to_facts_sqr(player) < 64.0 * 64.0
        });
    if let Some(pos) = valid_noteblock(body, targets.level()) {
        let block_pos = BlockPos {
            y: pos.y + 1,
            ..pos
        };
        return Some(DepositTarget {
            position: Vec3d {
                x: f64::from(block_pos.x) + 0.5,
                y: f64::from(block_pos.y) + 0.5,
                z: f64::from(block_pos.z) + 0.5,
            },
            block_pos,
            liked_player: player.map(|player| player.uuid()),
        });
    }
    let player = player?;
    Some(DepositTarget {
        position: Vec3d {
            y: player.eye_y(),
            ..player.position()
        },
        block_pos: player.block_position(),
        liked_player: Some(player.uuid()),
    })
}
