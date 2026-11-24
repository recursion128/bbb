use crate::connection::ClientboundPlayPacket;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_GRASS_BLOCK_ID,
};
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::entity_synched_data::{VANILLA_SHEEP_FLAG_SHEARED, VANILLA_SHEEP_WOOL_DATA_ID};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::ai::mob::sheep::GrazingAction;
use crate::entity::level::block_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_byte};
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData, GeneralLivingData};

const COLOR_MASK: i8 = 15;
const UPDATE_CLIENTS: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SheepRetainedState {
    wool: i8,
}

impl SheepRetainedState {
    pub(in crate::entity) const fn from_saved(color: i8, sheared: bool) -> Self {
        Self {
            wool: color
                | if sheared {
                    VANILLA_SHEEP_FLAG_SHEARED
                } else {
                    0
                },
        }
    }

    pub(crate) const fn color(self) -> i8 {
        self.wool & COLOR_MASK
    }

    pub(crate) const fn sheared(self) -> bool {
        self.wool & VANILLA_SHEEP_FLAG_SHEARED != 0
    }

    pub(crate) const fn packed(self) -> i8 {
        self.wool
    }

    pub(crate) fn set_sheared(&mut self, sheared: bool) -> bool {
        let next = if sheared {
            self.wool | VANILLA_SHEEP_FLAG_SHEARED
        } else {
            self.wool & !VANILLA_SHEEP_FLAG_SHEARED
        };
        let changed = next != self.wool;
        self.wool = next;
        changed
    }
}

impl GeneralLivingData {
    #[must_use]
    pub fn sheep_is_sheared(&self) -> Option<bool> {
        self.sheep_retained().map(SheepRetainedState::sheared)
    }
}

pub(crate) fn color_of(entity: &Entity) -> Option<i8> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.sheep_retained().map(SheepRetainedState::color)
}

pub(crate) fn set_spawn_color(entity: &mut Entity, color: i8) -> bool {
    let color = normalize_color(color);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.sheep_retained_mut() else {
        return false;
    };
    if state.color() != color {
        state.wool = color
            | if state.sheared() {
                VANILLA_SHEEP_FLAG_SHEARED
            } else {
                0
            };
        entity.mark_synched_dirty(VANILLA_SHEEP_WOOL_DATA_ID);
    }
    true
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: SheepRetainedState) {
    entries.extend([
        byte_entry("Sheared", i8::from(state.sheared())),
        byte_entry("Color", state.color()),
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> SheepRetainedState {
    let sheared = optional_bool(entries, "Sheared").unwrap_or(false);
    let color = optional_byte(entries, "Color").map_or(0, normalize_color);
    SheepRetainedState::from_saved(color, sheared)
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "Sheared" | "Color")
}

pub(crate) fn synched_values(state: SheepRetainedState) -> [SynchedValue; 1] {
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_SHEEP_WOOL_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(state.packed()),
        },
        is_default: state.packed() == 0,
    }]
}

pub(crate) fn apply_grazing(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    action: GrazingAction,
) -> bool {
    match action {
        GrazingAction::Start => {
            store.broadcast_to_tracking(
                entity.id(),
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id: entity.id(),
                    event_id: 10,
                }),
            );
            true
        }
        GrazingAction::Eat => eat_block(entity, store, level),
    }
}

pub(super) fn can_eat_at(pos: BlockPos, level: &LevelCtx<'_, '_>) -> bool {
    if block_state_tag_membership(level, level.block_state(pos), "minecraft:edible_for_sheep") {
        true
    } else {
        block_type_id(level.block_state(below(pos))) == Some(VANILLA_BLOCK_GRASS_BLOCK_ID)
    }
}

fn eat_block(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    let pos = entity.block_position();
    let state_id = level.block_state(pos);
    if block_state_tag_membership(level, state_id, "minecraft:edible_for_sheep") {
        if level.mob_griefing() {
            level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, state_id);
            let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID)
                .expect("built-in air has a default state");
            if level.set_entity_block(
                crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
                pos,
                air,
                SET_BLOCK_UPDATE_ALL,
            ) {
                level.game_event_with_state(
                    VanillaGameEvent::BLOCK_DESTROY,
                    pos,
                    None,
                    Some(state_id),
                );
            }
        }
        return ate(entity, store, level);
    }
    let below = below(pos);
    let below_state = level.block_state(below);
    if block_type_id(below_state) != Some(VANILLA_BLOCK_GRASS_BLOCK_ID) {
        return true;
    }
    if level.mob_griefing() {
        let grass = vanilla_block_default_state_id(VANILLA_BLOCK_GRASS_BLOCK_ID)
            .expect("built-in grass block has a default state");
        let dirt = vanilla_block_default_state_id(VANILLA_BLOCK_DIRT_ID)
            .expect("built-in dirt has a default state");
        level.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, below, grass);
        let _ = level.set_entity_block(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
            below,
            dirt,
            UPDATE_CLIENTS,
        );
    }
    ate(entity, store, level)
}

fn ate(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    level.game_event(
        VanillaGameEvent::EAT,
        entity.block_position(),
        Some(entity.id()),
    );
    let (wool_changed, baby_changed) = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let wool_changed = data
                .sheep_retained_mut()
                .is_some_and(|state| state.set_sheared(false));
            let baby_changed = data
                .ageable_retained_mut()
                .is_some_and(|age| age.can_age_up() && age.age_up(60, false));
            (wool_changed, baby_changed)
        }
        _ => return true,
    };
    if wool_changed {
        entity.mark_synched_dirty(VANILLA_SHEEP_WOOL_DATA_ID);
    }
    if baby_changed {
        let previous = [entity.bb_width(), entity.bb_height()];
        crate::entity::general_living::publish_baby_change(entity);
        return crate::entity::living::dimensions_growth::resolve_in_store(
            entity,
            previous,
            level,
            Some(store),
        )
        .is_some();
    }
    true
}

fn block_type_id(state_id: i32) -> Option<i32> {
    vanilla_block_state_block_type_id(state_id)
}

const fn below(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y.wrapping_sub(1),
        ..pos
    }
}

const fn normalize_color(color: i8) -> i8 {
    if color >= 0 && color <= COLOR_MASK {
        color
    } else {
        0
    }
}

fn byte_entry(name: &str, value: i8) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(value),
    }
}

#[cfg(test)]
#[path = "sheep/tests.rs"]
mod tests;
