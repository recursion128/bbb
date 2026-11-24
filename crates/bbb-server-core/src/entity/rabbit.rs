use crate::block_behavior::{crop, SET_BLOCK_UPDATE_CLIENTS};
use crate::connection::ClientboundPlayPacket;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
    VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_CARROTS_ID;
use bbb_protocol::builtin_enum_component_values::{
    VANILLA_RABBIT_VARIANT_BROWN_ID, VANILLA_RABBIT_VARIANT_EVIL_ID, VANILLA_RABBIT_VARIANT_VALUES,
};
use bbb_protocol::component::{component_translatable_nbt, ComponentStyle};
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::VANILLA_RABBIT_TYPE_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::living::{entity_block_jump_factor, SynchedValue};
use crate::entity::save::optional_int;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

const EVIL_ATTACK_POWER_MODIFIER: &str = "minecraft:evil";

#[derive(Debug, Clone, PartialEq)]
pub struct PendingCarrotRaid {
    pub(crate) rabbit: TargetRef,
    pub(crate) pos: BlockPos,
}

pub(crate) struct RabbitJumpInput {
    pub(crate) speed_modifier: f64,
    pub(crate) wanted_y: f64,
    pub(crate) next_node_y: Option<f64>,
}

// Rabbit.java:624-650,654-665.
pub(crate) fn raid_carrot(
    level: &mut LevelCtx<'_, '_>,
    store: &mut EntityStore,
    request: PendingCarrotRaid,
) {
    if !level.mob_griefing() {
        return;
    }
    let _ = store.with_entity_mut(request.rabbit.entity_id, |body, _store| {
        if body.uuid() != request.rabbit.uuid
            || body.entity_type_id() != VANILLA_ENTITY_TYPE_RABBIT_ID
            || !body.is_alive_with_health()
        {
            return;
        }
        let EntityData::GeneralLiving(data) = &mut body.data else {
            return;
        };
        let Some(retained) = data.rabbit_retained_mut() else {
            return;
        };
        if retained.more_carrot_ticks() > 0 {
            return;
        }
        let pos = request.pos;
        let Some(state) = level.block_state_checked(pos) else {
            return;
        };
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        if vanilla_block_state_block_type_id(state) != Some(VANILLA_BLOCK_CARROTS_ID)
            || !crop::is_max_age(state)
            || !level.block_state_checked(below).is_some_and(|support| {
                level.block_state_in_tag(support, "minecraft:supports_crops")
            })
        {
            return;
        }
        let Some(next) = crop::age(state).and_then(|age| crop::state_for_age(state, age - 1))
        else {
            return;
        };
        if !level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        ) {
            return;
        }
        retained.set_more_carrot_ticks(40);
        level.game_event_from_entity(
            crate::level_event::VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(body.id()),
            None,
        );
        level.level_event(2001, pos, state);
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RabbitRetainedState {
    variant: i32,
    more_carrot_ticks: i32,
}

impl RabbitRetainedState {
    pub(in crate::entity) fn from_saved(variant: i32, more_carrot_ticks: i32) -> Self {
        Self {
            variant,
            more_carrot_ticks,
        }
    }

    pub(crate) const fn variant(self) -> i32 {
        self.variant
    }

    pub(crate) const fn more_carrot_ticks(self) -> i32 {
        self.more_carrot_ticks
    }

    pub(crate) fn tick_food_cooldown(&mut self, random: &crate::random::GameRandom) {
        if self.more_carrot_ticks > 0 {
            self.more_carrot_ticks = (self.more_carrot_ticks - random.next_int(3)).max(0);
        }
    }

    fn set_variant(&mut self, variant: i32) -> bool {
        let changed = self.variant != variant;
        self.variant = variant;
        changed
    }

    fn set_more_carrot_ticks(&mut self, more_carrot_ticks: i32) {
        self.more_carrot_ticks = more_carrot_ticks;
    }
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "RabbitType" | "MoreCarrotTicks")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: RabbitRetainedState) {
    entries.extend([
        int_entry("RabbitType", state.variant),
        int_entry("MoreCarrotTicks", state.more_carrot_ticks),
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> RabbitRetainedState {
    let variant = optional_int(entries, "RabbitType")
        .filter(|value| known_variant(*value))
        .unwrap_or(VANILLA_RABBIT_VARIANT_BROWN_ID);
    RabbitRetainedState::from_saved(
        variant,
        optional_int(entries, "MoreCarrotTicks").unwrap_or(0),
    )
}

pub(crate) fn synched_values(state: RabbitRetainedState) -> [SynchedValue; 1] {
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_RABBIT_TYPE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(state.variant),
        },
        is_default: state.variant == VANILLA_RABBIT_VARIANT_BROWN_ID,
    }]
}

pub(crate) fn play_jump_sound(entity: &Entity, store: &mut EntityStore, pitch: f32) {
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.rabbit.jump".to_string(),
            source: crate::entity::living::living_sound_source(entity),
            volume: 1.0,
            pitch,
        });
    }
}

pub(crate) fn after_ai_step(entity: &mut Entity, store: &mut EntityStore) {
    let stop = store
        .mob_ai_mut(entity.id())
        .is_some_and(crate::entity::ai::mob::MobAiRuntime::rabbit_after_ai_step);
    if stop {
        if let Some(living) = entity.living_mut() {
            living.jumping = false;
        }
    }
}

pub(crate) fn jump_from_ground(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_RABBIT_ID {
        return false;
    }
    let in_lava = entity.is_in_lava();
    let fluid_height = entity.fluid_height(if in_lava {
        crate::entity::environment::FluidTag::Lava
    } else {
        crate::entity::environment::FluidTag::Water
    });
    if ((entity.is_in_water() && fluid_height > 0.0) || in_lava)
        && (!entity.on_ground()
            || fluid_height > crate::entity::living::fluid_jump_threshold(entity))
    {
        return false;
    }
    let ready = entity.on_ground() && entity.living().is_some_and(|living| living.jumping);
    if !ready {
        return true;
    }
    let input = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.rabbit_jump_input());
    let speed_modifier = input.as_ref().map_or(0.0, |input| input.speed_modifier);
    // Rabbit.java:138-173.
    let base = if entity.horizontal_collision()
        || input.as_ref().is_some_and(|input| {
            input.wanted_y > entity.position().y + 0.5
                || input
                    .next_node_y
                    .is_some_and(|y| y > entity.position().y + 0.5)
        }) {
        0.5
    } else if speed_modifier > 0.6 {
        0.3
    } else {
        0.2
    };
    let living = entity.living().expect("checked above");
    let jump_boost = living
        .active_effect("minecraft:jump_boost")
        .map_or(0.0, |effect| 0.1 * (effect.amplifier() as f32 + 1.0));
    let jump_power = living.attribute_value(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID) as f32
        * (base / 0.42)
        * entity_block_jump_factor(entity, level)
        + jump_boost;
    if jump_power > 1.0e-5 {
        let movement = entity.delta_movement();
        entity.set_delta_movement(bbb_protocol::packets::Vec3d {
            y: movement.y.max(f64::from(jump_power)),
            ..movement
        });
        entity.set_needs_sync(true);
    }
    let movement = entity.delta_movement();
    if speed_modifier > 0.0 && movement.x * movement.x + movement.z * movement.z < 0.01 {
        let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
        crate::entity::living::move_relative(
            entity,
            0.1,
            Vec3d {
                x: 0.0,
                y: if baby { 0.5 } else { 1.5 },
                z: 1.0,
            },
        );
    }
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: 1,
        }),
    );
    true
}

pub(crate) fn finish_load(entity: &mut Entity) {
    let variant = match entity.data() {
        EntityData::GeneralLiving(data) => data.rabbit_retained().map(RabbitRetainedState::variant),
        _ => None,
    };
    if let Some(variant) = variant {
        apply_variant_effects(entity, variant);
    }
}

/// `Rabbit.getVariant() == Rabbit.Variant.EVIL`, the killer bunny.
pub(crate) fn is_evil(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.rabbit_retained().is_some_and(|state| state.variant() == VANILLA_RABBIT_VARIANT_EVIL_ID))
}

pub(crate) fn set_variant(entity: &mut Entity, variant: i32) -> bool {
    if !VANILLA_RABBIT_VARIANT_VALUES
        .iter()
        .any(|entry| entry.id == variant)
    {
        return false;
    }
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .rabbit_retained_mut()
            .is_some_and(|state| state.set_variant(variant)),
        _ => return false,
    };
    apply_variant_effects(entity, variant);
    if changed {
        entity.mark_synched_dirty(VANILLA_RABBIT_TYPE_DATA_ID);
    }
    true
}

pub(crate) fn set_spawn_variant(entity: &mut Entity, variant: i32) -> bool {
    known_variant(variant) && {
        let owns_rabbit = matches!(
            entity.data(),
            EntityData::GeneralLiving(data) if data.rabbit_retained().is_some()
        );
        if !owns_rabbit {
            return false;
        }
        let _ = set_variant(entity, variant);
        true
    }
}

fn apply_variant_effects(entity: &mut Entity, variant: i32) {
    if variant == VANILLA_RABBIT_VARIANT_EVIL_ID {
        {
            let Some(living) = entity.living_mut() else {
                return;
            };
            if let Some(armor) = living
                .attributes_mut()
                .get_instance(VANILLA_ATTRIBUTE_ARMOR_ID)
            {
                armor.set_base_value(8.0);
            }
            if let Some(attack) = living
                .attributes_mut()
                .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            {
                attack.add_or_update_transient_modifier(AttributeModifier {
                    id: EVIL_ATTACK_POWER_MODIFIER.to_string(),
                    amount: 5.0,
                    operation: AttributeOperation::AddValue,
                });
            }
        }
        if entity.custom_name().is_none() {
            let component = component_translatable_nbt(
                "entity.minecraft.killer_bunny",
                Vec::new(),
                &ComponentStyle::default(),
            )
            .expect("static translatable component");
            entity.set_custom_name(Some(
                RichTextComponent::try_from_component_nbt(component)
                    .expect("static translatable component"),
            ));
        }
    } else if let Some(living) = entity.living_mut() {
        if let Some(attack) = living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
        {
            attack.remove_modifier(EVIL_ATTACK_POWER_MODIFIER);
        }
    }
}

fn known_variant(variant: i32) -> bool {
    VANILLA_RABBIT_VARIANT_VALUES
        .iter()
        .any(|entry| entry.id == variant)
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

#[cfg(test)]
#[path = "rabbit/tests.rs"]
mod tests;
