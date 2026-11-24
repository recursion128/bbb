//! Server-owned `EyeOfEnder` state and lifecycle hooks.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID;
use bbb_protocol::entity_synched_data::VANILLA_EYE_OF_ENDER_ITEM_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SoundSource, Vec3d};
use bbb_protocol::NbtSummaryEntry;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::item::{plain_stack, stack_with_count};
use crate::entity::living::SynchedValue;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{base_tick, Entity, EntityData};
use bbb_server_types::geometry::Vec3Ops;

pub const DEFAULT_GRAVITY: f64 = 0.0;

#[derive(Debug, Clone, PartialEq)]
pub struct EyeOfEnderData {
    item: RuntimeItemStack,
    target: Option<Vec3d>,
    life: i32,
    survive_after_death: bool,
}

impl EyeOfEnderData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.target = previous.target;
        self.life = previous.life;
        self.survive_after_death = previous.survive_after_death;
    }

    pub fn new(item: RuntimeItemStack) -> Self {
        let item = (!item.is_empty())
            .then(|| stack_with_count(&item, 1))
            .flatten()
            .unwrap_or_else(default_item);
        Self {
            item,
            target: None,
            life: 0,
            survive_after_death: false,
        }
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn target(&self) -> Option<Vec3d> {
        self.target
    }

    pub const fn life(&self) -> i32 {
        self.life
    }

    pub const fn survive_after_death(&self) -> bool {
        self.survive_after_death
    }
}

fn default_item() -> RuntimeItemStack {
    plain_stack("minecraft:ender_eye", 1).expect("the built-in ender eye stack is valid")
}

pub fn spawn_eye_of_ender(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
        position,
        EntityData::EyeOfEnder(EyeOfEnderData::new(item)),
    )
}

pub fn signal_to(store: &mut EntityStore, entity_id: i32, target: Vec3d) -> bool {
    let Some(entity) = store.get(entity_id) else {
        return false;
    };
    if !matches!(entity.data(), EntityData::EyeOfEnder(_)) {
        return false;
    }
    let position = entity.position();
    let delta = Vec3d {
        x: target.x - position.x,
        y: target.y - position.y,
        z: target.z - position.z,
    };
    let horizontal_distance = delta.horizontal_distance_sqr().sqrt();
    let target = if horizontal_distance > 12.0 {
        Vec3d {
            x: position.x + delta.x / horizontal_distance * 12.0,
            y: position.y + 8.0,
            z: position.z + delta.z / horizontal_distance * 12.0,
        }
    } else {
        target
    };
    let survive_after_death = store.random().next_int(5) > 0;
    let entity = store
        .get_mut(entity_id)
        .expect("the validated eye remains in the store");
    let EntityData::EyeOfEnder(data) = &mut entity.data else {
        unreachable!("eye-of-ender signal dispatched a different entity")
    };
    data.target = Some(target);
    data.life = 0;
    data.survive_after_death = survive_after_death;
    true
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Result<Option<(Vec3d, RuntimeItemStack)>, ()> {
    if !base_tick(entity, store, level) {
        return Err(());
    }
    let old_movement = entity.delta_movement();
    let new_position = entity.position().plus(old_movement);
    let target = data(entity).target();
    if let Some(target) = target {
        entity.set_delta_movement(update_delta_movement(old_movement, new_position, target));
    }
    entity.set_pos(new_position);

    let (life, survive_after_death, item) = {
        let EntityData::EyeOfEnder(data) = &mut entity.data else {
            unreachable!("eye-of-ender tick dispatched a different entity")
        };
        data.life += 1;
        (data.life, data.survive_after_death, data.item.clone())
    };
    if life > 80 {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.ender_eye.death".to_string(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
        entity.discard();
        if survive_after_death {
            return Ok(Some((entity.position(), item)));
        } else {
            level.level_event(2003, entity.block_position(), 0);
        }
    }
    Ok(None)
}

fn data(entity: &Entity) -> &EyeOfEnderData {
    match entity.data() {
        EntityData::EyeOfEnder(data) => data,
        other => unreachable!("eye-of-ender operation dispatched a {other:?}"),
    }
}

fn update_delta_movement(old_movement: Vec3d, position: Vec3d, target: Vec3d) -> Vec3d {
    let horizontal_delta = Vec3d {
        x: target.x - position.x,
        y: 0.0,
        z: target.z - position.z,
    };
    let horizontal_length = horizontal_delta.horizontal_distance_sqr().sqrt();
    let old_horizontal_speed = old_movement.horizontal_distance_sqr().sqrt();
    let mut wanted_speed =
        old_horizontal_speed + 0.0025 * (horizontal_length - old_horizontal_speed);
    let mut movement_y = old_movement.y;
    if horizontal_length < 1.0 {
        wanted_speed *= 0.8;
        movement_y *= 0.8;
    }
    let wanted_movement_y = if position.y - old_movement.y < target.y {
        1.0
    } else {
        -1.0
    };
    let horizontal_scale = if horizontal_length > 0.0 {
        wanted_speed / horizontal_length
    } else {
        0.0
    };
    Vec3d {
        x: horizontal_delta.x * horizontal_scale,
        y: movement_y + (wanted_movement_y - movement_y) * 0.015,
        z: horizontal_delta.z * horizontal_scale,
    }
}

pub fn synched_values(entity: &Entity, data: &EyeOfEnderData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    let stack = data.item().summary().clone();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_EYE_OF_ENDER_ITEM_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        },
        is_default: data.item() == &default_item(),
    });
    rows
}

pub fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    data: &EyeOfEnderData,
    items: crate::entity::item::ItemResolver<'_>,
) {
    entries.push(NbtSummaryEntry {
        name: "Item".into(),
        value: crate::entity::item::write_saved_stack(data.item(), items),
    });
}

pub fn read_save_data(
    entries: &[NbtSummaryEntry],
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> EyeOfEnderData {
    let item = crate::entity::display::save_fields::optional_field(entries, "Item")
        .and_then(|value| crate::entity::item::read_saved_stack(value, items))
        .filter(|stack| !stack.is_empty())
        .map_or_else(default_item, |stack| stack.with_count(1));
    EyeOfEnderData::new(item)
}
