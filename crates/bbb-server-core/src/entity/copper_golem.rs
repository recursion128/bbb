pub(crate) mod transport;

use bbb_protocol::entity_synched_data::{
    VANILLA_COPPER_GOLEM_STATE_DATA_ID, VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_IDLE_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{
    EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind, ItemStackSummary, SoundSource,
    Vec3d,
};

use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData, EntityRegistryTagView, EntityStore, GeneralLivingData};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AnimationState {
    #[default]
    Idle,
    GettingItem,
    GettingNoItem,
    DroppingItem,
    DroppingNoItem,
}

impl AnimationState {
    pub(crate) fn synched_value(self) -> SynchedValue {
        let id = match self {
            Self::Idle => VANILLA_COPPER_GOLEM_STATE_IDLE_ID,
            Self::GettingItem => VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID,
            Self::GettingNoItem => VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
            Self::DroppingItem => VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
            Self::DroppingNoItem => VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
        };
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_COPPER_GOLEM_STATE_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COPPER_GOLEM_STATE_ID,
                value: EntityDataValueKind::EnumId {
                    serializer: EntityDataEnumSerializer::CopperGolemState,
                    id,
                },
            },
            is_default: self == Self::Idle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopperGolemState {
    pub(crate) animation: AnimationState,
    pub(crate) weather_state: i32,
    pub(crate) next_weather_age: i64,
}

impl Default for CopperGolemState {
    fn default() -> Self {
        Self {
            animation: AnimationState::Idle,
            weather_state: VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
            next_weather_age: -1,
        }
    }
}

impl CopperGolemState {
    pub(crate) fn weather_state_from_str(value: &str) -> i32 {
        match value {
            "exposed" => VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
            "weathered" => VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
            "oxidized" => VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
            _ => VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
        }
    }

    pub(crate) const fn weather_state_name(state: i32) -> &'static str {
        match state {
            VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => "exposed",
            VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => "weathered",
            VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => "oxidized",
            _ => "unaffected",
        }
    }

    pub(crate) fn read_save(&mut self, entries: &[bbb_protocol::NbtSummaryEntry]) {
        self.next_weather_age =
            crate::entity::save::optional_long(entries, "next_weather_age").unwrap_or(-1);
        let saved = entries.iter().find_map(|field| {
            if field.name == "weather_state" {
                if let bbb_protocol::NbtSummaryValue::String(value) = &field.value {
                    return Some(value);
                }
            }
            None
        });
        self.weather_state = match saved {
            Some(value) if value == "exposed" => VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
            Some(value) if value == "weathered" => VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
            Some(value) if value == "oxidized" => VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
            _ => VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
        };
    }

    pub(crate) fn write_save(&self, entries: &mut Vec<bbb_protocol::NbtSummaryEntry>) {
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: "next_weather_age".into(),
            value: bbb_protocol::NbtSummaryValue::Long(self.next_weather_age),
        });
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: "weather_state".into(),
            value: bbb_protocol::NbtSummaryValue::string(Self::weather_state_name(
                self.weather_state,
            )),
        });
    }

    pub(crate) fn owns_save_field(name: &str) -> bool {
        matches!(name, "next_weather_age" | "weather_state")
    }
}

pub(crate) fn set_animation_state(entity: &mut Entity, state: AnimationState) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data.copper_golem_animation() == Some(state) {
        return;
    }
    let Some(current) = data.copper_golem_state_mut() else {
        return;
    };
    current.animation = state;
    entity.mark_synched_dirty(VANILLA_COPPER_GOLEM_STATE_DATA_ID);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherChange {
    Waxed,
    Unwaxed,
    Scraped,
}

/// `CopperGolem.playSpawnSound` (`CopperGolem.java:376-378`).
pub(crate) fn play_spawn_sound(entity: &Entity, store: &mut EntityStore) {
    if !entity.is_silent() {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.copper_golem.spawn".into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

pub(crate) fn weather_state(data: &GeneralLivingData) -> i32 {
    data.copper_golem_state()
        .map_or(VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID, |state| {
            state.weather_state
        })
}

pub(crate) fn set_weather_state(data: &mut GeneralLivingData, state_name: &str) -> bool {
    let Some(state) = data.copper_golem_state_mut() else {
        return false;
    };
    state.weather_state = CopperGolemState::weather_state_from_str(state_name);
    true
}

fn next_weather_age(data: &GeneralLivingData) -> i64 {
    data.copper_golem_state()
        .map_or(-1, |state| state.next_weather_age)
}

fn set_next_weather_age(data: &mut GeneralLivingData, age: i64) {
    if let Some(state) = data.copper_golem_state_mut() {
        state.next_weather_age = age;
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID {
        return true;
    }
    if !tick_weathering(entity, store, level) {
        return false;
    }
    if entity.removal_reason().is_some() {
        return true;
    }
    transport::tick(entity, store, level)
}

/// `CopperGolem.updateWeathering`, `canTurnToStatue` and `turnToStatue`
/// (`CopperGolem.java:261-306`); server weathering also runs with NoAI (`:192-201`).
fn tick_weathering(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return true;
    };
    let age = next_weather_age(data);
    if age == -2 {
        return true;
    }
    let random = level.level_random();
    if age == -1 {
        set_next_weather_age(
            data,
            level.game_time() + i64::from(random.next_int_between_inclusive(504000, 552000)),
        );
        return true;
    }
    let state = weather_state(data);
    if state != VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID {
        if level.game_time() >= age {
            let next = match state {
                VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID => "exposed",
                VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => "weathered",
                _ => "oxidized",
            };
            set_weather_state(data, next);
            set_next_weather_age(
                data,
                if next == "oxidized" {
                    0
                } else {
                    age + i64::from(random.next_int_between_inclusive(504000, 552000))
                },
            );
            entity.mark_synched_dirty(VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID);
        }
        return true;
    }
    let pos = entity.block_position();
    if !bbb_server_types::fluid::block_state_is_air(level.block_state(pos))
        || random.next_float() > 0.0058
    {
        return true;
    }
    let Some(block) = bbb_protocol::block_types::vanilla_block_type_for_resource_id(
        "minecraft:oxidized_copper_golem_statue",
    ) else {
        return false;
    };
    let Some(poses) = bbb_protocol::block_state_types::vanilla_block_property_values(
        block.resource_id,
        "copper_golem_pose",
    ) else {
        return false;
    };
    let Some(facing) = bbb_server_types::directions::horizontal_direction(entity.y_rot()) else {
        return false;
    };
    let Some(state) = crate::fluid::with_property(
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap(),
        "copper_golem_pose",
        poses[random.next_int(poses.len() as i32) as usize],
    )
    .and_then(|state| {
        crate::fluid::with_property(
            state,
            "facing",
            bbb_server_types::block_queries::direction_serialized_name(facing),
        )
    }) else {
        return false;
    };
    if !level.place_copper_golem_statue(pos, state, entity.custom_name()) {
        return false;
    }
    if crate::entity::mob_death_loot::drop_preserved_equipment(store, level, entity, false)
        .is_none()
    {
        return false;
    }
    let drop_lead = level.entity_drops();
    if !crate::entity::leash::drop_leash(entity, store, level, true, drop_lead) {
        return false;
    }
    entity.discard();
    if !entity.is_silent() {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position: entity.position(),
            resource_id: crate::block_behavior::copper_golem_statue::POSE_SOUND.into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    true
}

pub(crate) fn synched_values(entity: &Entity) -> Option<SynchedValue> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let state = weather_state(data);
    Some(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID,
            serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID,
            value: EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::WeatheringCopperState,
                id: state,
            },
        },
        is_default: state == VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    })
}

// CopperGolem.java:205-212.
pub(crate) fn return_held_item(
    store: &mut EntityStore,
    target_id: i32,
    hand: &ItemStackSummary,
    towards: Vec3d,
) -> bool {
    if hand.item_id.is_some() && hand.count > 0 {
        return false;
    }
    store
        .with_entity_mut(target_id, |target, store| {
            if target.entity_type_id() != VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
                || !crate::entity::name_tag::living_entity_is_alive(target)
            {
                return false;
            }
            let Some(stack) = target
                .living_mut()
                .and_then(|living| living.set_item_slot(EquipmentSlot::MainHand, None))
            else {
                return false;
            };
            crate::entity::item::throw_item(target, store, towards, stack);
            true
        })
        .unwrap_or(false)
}

/// `CopperGolem.mobInteract`, `shear` and `readyForShearing`
/// (`CopperGolem.java:217-225`, `:429-438`); antenna slot is SADDLE (`:87`).
pub(crate) fn shear(
    store: &mut EntityStore,
    target_id: i32,
    hand: &ItemStackSummary,
    source: SoundSource,
    tags: &EntityRegistryTagView,
) -> bool {
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    if target.entity_type_id() != VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
        || !crate::entity::name_tag::living_entity_is_alive(target)
    {
        return false;
    }
    let Some(stack) = target
        .living()
        .and_then(|living| living.equipment().get(EquipmentSlot::Saddle))
    else {
        return false;
    };
    let Some(item) = stack
        .item_id()
        .and_then(bbb_protocol::item_types::vanilla_item_type)
    else {
        return false;
    };
    if !tags.contains(
        "minecraft:item",
        "minecraft:shearable_from_copper_golem",
        item.resource_id,
    ) {
        return false;
    }
    let dropped = stack.clone();
    let position = target.position();
    store
        .get_mut(target_id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Saddle, None);
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position,
        resource_id: "minecraft:entity.copper_golem.shear".into(),
        source,
        volume: 1.0,
        pitch: 1.0,
    });
    crate::entity::item::spawn_dropped_item_entity(
        store,
        Vec3d {
            y: position.y + 1.5,
            ..position
        },
        dropped,
    );
    true
}

fn previous_weather_state(data: &GeneralLivingData) -> Option<&'static str> {
    match weather_state(data) {
        VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID => Some("unaffected"),
        VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID => Some("exposed"),
        VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID => Some("weathered"),
        _ => None,
    }
}

/// `CopperGolem.thunderHit` (`CopperGolem.java:454-465`), once per bolt: steps back one
/// weather state and restarts the weathering clock, unwaxing the golem.
pub(crate) fn thunder_hit(entity: &mut Entity) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(previous) = previous_weather_state(data) else {
        return;
    };
    set_weather_state(data, previous);
    set_next_weather_age(data, -1);
    entity.mark_synched_dirty(VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID);
}

/// `CopperGolem.mobInteract` waxing and scraping (`CopperGolem.java:231-253`), using the
/// retained `next_weather_age` save field (`:169-176`).
pub(crate) fn weather_interaction(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut crate::entity::leash::LeashInteractingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Option<WeatherChange> {
    let axe = player
        .hand_stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .is_some_and(|item| tags.contains("minecraft:item", "minecraft:axes", item.resource_id));
    let target = store.get_mut(target_id)?;
    if target.entity_type_id() != VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
        || !target.is_alive_with_health()
    {
        return None;
    }
    let position = target.position();
    let crate::entity::EntityData::GeneralLiving(data) = &mut target.data else {
        return None;
    };
    let waxed = next_weather_age(data) == -2;
    let change = if player.hand_stack.item_id
        == Some(bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID)
        && !waxed
    {
        WeatherChange::Waxed
    } else if waxed && axe {
        WeatherChange::Unwaxed
    } else if axe {
        let previous = previous_weather_state(data)?;
        set_weather_state(data, previous);
        WeatherChange::Scraped
    } else {
        return None;
    };
    set_next_weather_age(
        data,
        if change == WeatherChange::Waxed {
            -2
        } else {
            -1
        },
    );
    if change == WeatherChange::Scraped {
        target.mark_synched_dirty(VANILLA_COPPER_GOLEM_WEATHER_STATE_DATA_ID);
    }
    if change != WeatherChange::Waxed {
        // `CopperGolem.java:238,248` plays `SoundEvents.AXE_SCRAPE` for both unwaxing and scraping.
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position,
            resource_id: "minecraft:item.axe.scrape".into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    } else {
        let _ = crate::entity::animal_feeding::use_mob_player_item(
            &mut crate::entity::animal_feeding::FeedingPlayer {
                hand_stack: &mut *player.hand_stack,
                uuid: player.uuid,
                infinite_materials: player.infinite_materials,
            },
            &mut player.mob_item_used,
        );
    }
    Some(change)
}
