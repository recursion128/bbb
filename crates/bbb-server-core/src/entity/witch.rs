use crate::connection::ClientboundPlayPacket;
use crate::level_event::VanillaGameEvent;
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_WITCH_USING_ITEM_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{
    BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent, SoundSource, Vec3d,
};
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::ai::mob::target::{Target, TargetView};
use crate::entity::ai::mob::MobAiContext;
use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::damage::{StoredEntityDamageSource, StoredEntityHurtOutcome};
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_shoot::{movement_to_shoot, ProjectileShot};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::thrown_potion::{apply_registry_potion_in_level, registry_potion_stack};
use crate::entity::{spawn_splash_potion, Entity, EntityData};

const DRINKING_MODIFIER_ID: &str = "minecraft:drinking";
const POTION_USE_DURATION: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitchState {
    using_item: bool,
    using_time: i32,
    /// The game time of the last hurt, when its source was `#is_fire`.
    last_fire_damage_time: Option<i64>,
}

impl WitchState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        *self = *previous;
    }

    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if entity_type_id == VANILLA_ENTITY_TYPE_WITCH_ID {
            Some(Self {
                using_item: false,
                using_time: 0,
                last_fire_damage_time: None,
            })
        } else {
            None
        }
    }

    pub(crate) const fn using_item(self) -> bool {
        self.using_item
    }

    /// `getLastDamageSource().is(IS_FIRE)`: the source is forgotten 40 ticks
    /// after the hurt (`LivingEntity.java:1394-1399`).
    pub(crate) fn remembers_fire_damage(self, game_time: i64) -> bool {
        self.last_fire_damage_time
            .is_some_and(|time| game_time - time <= 40)
    }
}

pub(crate) fn synched_values(state: WitchState) -> Vec<SynchedValue> {
    vec![SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_WITCH_USING_ITEM_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(state.using_item),
        },
        is_default: !state.using_item,
    }]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelfPotion {
    WaterBreathing,
    FireResistance,
    Healing,
    Swiftness,
}

impl SelfPotion {
    const fn resource_id(self) -> &'static str {
        match self {
            Self::WaterBreathing => "minecraft:water_breathing",
            Self::FireResistance => "minecraft:fire_resistance",
            Self::Healing => "minecraft:healing",
            Self::Swiftness => "minecraft:swiftness",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelfPotionInput {
    pub(crate) drinking: bool,
    pub(crate) using_time: i32,
    pub(crate) eye_in_water: bool,
    pub(crate) fire_threat: bool,
    pub(crate) has_water_breathing: bool,
    pub(crate) has_fire_resistance: bool,
    pub(crate) hurt: bool,
    pub(crate) has_speed: bool,
    pub(crate) distant_target: bool,
    pub(crate) silent: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SelfPotionStep {
    pub(crate) finish_drinking: bool,
    pub(crate) still_drinking: bool,
    pub(crate) using_time: i32,
    pub(crate) start: Option<SelfPotion>,
    pub(crate) drink_pitch: Option<f32>,
    pub(crate) event_15: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitchRuntime {
    heal_raider_cooldown: i32,
    /// Whether the last self-potion step let the witch target players.
    pub(crate) attack_players: bool,
}

impl WitchRuntime {
    pub(crate) const fn new() -> Self {
        Self {
            heal_raider_cooldown: 0,
            attack_players: true,
        }
    }

    pub(crate) const fn heal_raider_cooldown(self) -> i32 {
        self.heal_raider_cooldown
    }

    pub(crate) fn set_heal_raider_cooldown(&mut self, cooldown: i32) {
        self.heal_raider_cooldown = cooldown;
    }

    pub(crate) fn self_potion_step(
        &mut self,
        random: &GameRandom,
        input: SelfPotionInput,
    ) -> (bool, SelfPotionStep) {
        self.heal_raider_cooldown = self.heal_raider_cooldown.wrapping_sub(1);
        let mut step = SelfPotionStep {
            using_time: input.using_time,
            ..SelfPotionStep::default()
        };
        if input.drinking {
            step.using_time = input.using_time.wrapping_sub(1);
            step.finish_drinking = input.using_time <= 0;
            step.still_drinking = !step.finish_drinking;
        } else {
            let potion = if random.next_float() < 0.15
                && input.eye_in_water
                && !input.has_water_breathing
            {
                Some(SelfPotion::WaterBreathing)
            } else if random.next_float() < 0.15 && input.fire_threat && !input.has_fire_resistance
            {
                Some(SelfPotion::FireResistance)
            } else if random.next_float() < 0.05 && input.hurt {
                Some(SelfPotion::Healing)
            } else if random.next_float() < 0.5 && input.distant_target && !input.has_speed {
                Some(SelfPotion::Swiftness)
            } else {
                None
            };
            if potion.is_some() {
                step.start = potion;
                step.using_time = POTION_USE_DURATION;
                if !input.silent {
                    step.drink_pitch = Some(0.8 + random.next_float() * 0.4);
                }
            }
        }
        step.event_15 = random.next_float() < 7.5e-4;
        (self.heal_raider_cooldown <= 0, step)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RangedAttackAction {
    pub(crate) potion_resource_id: &'static str,
    pub(crate) direction: Vec3d,
    pub(crate) power: f32,
}

pub(crate) fn prepare_ranged_attack(
    context: &mut MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    target: &Target<'_, '_, '_>,
) {
    let EntityData::GeneralLiving(body) = targets.body_entity().data() else {
        unreachable!("Witch has general living data");
    };
    if body
        .witch_state()
        .expect("Witch has witch state")
        .using_item()
    {
        return;
    }
    let (health, has_slowness, has_poison, has_weakness) = if let Some(player) = target.player() {
        (
            player.health,
            player.has_slowness,
            player.has_poison,
            player.has_weakness,
        )
    } else {
        let living = target.living().expect("Witch attacks a living target");
        (
            living.health(),
            living.active_effect("minecraft:slowness").is_some(),
            living.active_effect("minecraft:poison").is_some(),
            living.active_effect("minecraft:weakness").is_some(),
        )
    };
    let raider = crate::entity::raider::is_raider(target.entity_type_id());
    let source = context.position();
    let position = target.position();
    let movement = target.delta_movement();
    let xd = position.x + movement.x - source.x;
    let yd = target.eye_y() - f64::from(1.1_f32) - source.y;
    let zd = position.z + movement.z - source.z;
    let distance = (xd * xd + zd * zd).sqrt();
    let potion_resource_id = if raider {
        if health <= 4.0 {
            "minecraft:healing"
        } else {
            "minecraft:regeneration"
        }
    } else if distance >= 8.0 && !has_slowness {
        "minecraft:slowness"
    } else if health >= 8.0 && !has_poison {
        "minecraft:poison"
    } else if distance <= 3.0 && !has_weakness && context.next_float() < 0.25 {
        "minecraft:weakness"
    } else {
        "minecraft:harming"
    };
    context.species.witch_action = Some(RangedAttackAction {
        potion_resource_id,
        direction: Vec3d {
            x: xd,
            y: yd + distance * 0.2,
            z: zd,
        },
        power: if distance <= 2.0 { 0.45 } else { 0.75 },
    });
    if raider {
        context.set_target(None, targets);
    }
}

pub(crate) fn before_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID || !entity.is_alive() {
        return true;
    }
    let id = entity.id();
    let (using_item, using_time, fire_damage) = match entity.data() {
        EntityData::GeneralLiving(data) => match data.witch_state() {
            Some(state) => (
                state.using_item,
                state.using_time,
                state.remembers_fire_damage(store.game_time()),
            ),
            None => return true,
        },
        _ => return true,
    };
    let living = entity.living().expect("Witch is living");
    let water_check_reached = if using_item {
        false
    } else {
        let Some(reached) = store
            .mob_ai_mut(id)
            .and_then(|runtime| runtime.witch_water_check_reached())
        else {
            return true;
        };
        reached
    };
    let eye_in_water = if water_check_reached {
        let Some(in_water) = eye_in_water(entity, level) else {
            return false;
        };
        in_water
    } else {
        false
    };
    let target = store
        .mob_ai(id)
        .and_then(|runtime| runtime.valid_target(entity, store, level));
    let input = SelfPotionInput {
        drinking: using_item,
        using_time,
        eye_in_water,
        fire_threat: entity.remaining_fire_ticks() > 0 || fire_damage,
        has_water_breathing: living.active_effect("minecraft:water_breathing").is_some(),
        has_fire_resistance: living.active_effect("minecraft:fire_resistance").is_some(),
        hurt: living.health() < living.max_health(),
        has_speed: living.active_effect("minecraft:speed").is_some(),
        distant_target: target
            .is_some_and(|target| entity.position().distance_to_sqr(target.position()) > 121.0),
        silent: entity.is_silent(),
    };
    let Some((attack_players, step)) = store
        .mob_ai_mut(id)
        .and_then(|runtime| runtime.witch_self_potion_step(input))
    else {
        return true;
    };
    store
        .mob_ai_mut(id)
        .expect("Witch retains Mob AI")
        .prepare_witch_targeting(attack_players);
    apply_self_potion_step(entity, store, level, step).is_some()
}

fn apply_self_potion_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    step: SelfPotionStep,
) -> Option<()> {
    if step.finish_drinking {
        let item = entity
            .living_mut()
            .and_then(|living| living.set_item_slot(EquipmentSlot::MainHand, None));
        if let Some(item) = item.as_ref() {
            apply_registry_potion_in_level(entity, store, level, item)?;
        }
        level.game_event(
            VanillaGameEvent::DRINK,
            entity.block_position(),
            Some(entity.id()),
        );
        remove_drinking_modifier(entity);
    }
    let mut using_item = step.still_drinking;
    if let Some(potion) = step.start {
        let Some(item) = registry_potion_stack("minecraft:potion", potion.resource_id()) else {
            return Some(());
        };
        entity
            .living_mut()
            .expect("Witch is living")
            .set_item_slot(EquipmentSlot::MainHand, Some(item));
        using_item = true;
        if let Some(pitch) = step.drink_pitch {
            store.play_entity_sound(PendingEntitySound {
                position: entity.position(),
                resource_id: "minecraft:entity.witch.drink".to_string(),
                source: SoundSource::Hostile,
                volume: 1.0,
                pitch,
            });
        }
        add_drinking_modifier(entity);
    }
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data.witch_state_mut().is_some_and(|state| {
            let changed = state.using_item != using_item;
            state.using_item = using_item;
            state.using_time = step.using_time;
            changed
        }),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_WITCH_USING_ITEM_DATA_ID);
    }
    if step.event_15 {
        store.broadcast_to_tracking(
            entity.id(),
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: entity.id(),
                event_id: 15,
            }),
        );
    }
    Some(())
}

fn add_drinking_modifier(entity: &mut Entity) {
    let Some(instance) = entity.living_mut().and_then(|living| {
        living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    }) else {
        return;
    };
    instance.remove_modifier(DRINKING_MODIFIER_ID);
    let _ = instance.add_transient_modifier(AttributeModifier {
        id: DRINKING_MODIFIER_ID.to_string(),
        amount: -0.25,
        operation: AttributeOperation::AddValue,
    });
}

fn remove_drinking_modifier(entity: &mut Entity) {
    if let Some(instance) = entity.living_mut().and_then(|living| {
        living
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    }) {
        instance.remove_modifier(DRINKING_MODIFIER_ID);
    }
}

fn eye_in_water(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<bool> {
    let eye_height = entity.eye_height();
    let eye_y = entity.position().y + f64::from(eye_height);
    let pos = BlockPos {
        x: entity.position().x.floor() as i32,
        y: eye_y.floor() as i32,
        z: entity.position().z.floor() as i32,
    };
    let state = level.block_state(pos);
    if vanilla_block_state_block_type_id(state).is_none()
        && level.content_registry().block_by_state_id(state).is_none()
    {
        return None;
    }
    let fluid = block_state_fluid_state(state);
    if !fluid_state_tag_membership(level, fluid, "minecraft:water") {
        return Some(false);
    }
    let above = level.block_state(BlockPos {
        y: pos.y + 1,
        ..pos
    });
    if vanilla_block_state_block_type_id(above).is_none()
        && level.content_registry().block_by_state_id(above).is_none()
    {
        return None;
    }
    let height = if block_state_fluid_state(above).family() == fluid.family() {
        1.0
    } else {
        f64::from(fluid.own_height())
    };
    Some(eye_y <= f64::from(pos.y) + height)
}

pub(crate) fn apply_ranged_attack(
    entity: &Entity,
    store: &mut EntityStore,
    action: RangedAttackAction,
) {
    let Some(item) = registry_potion_stack("minecraft:splash_potion", action.potion_resource_id)
    else {
        return;
    };
    let movement = movement_to_shoot(store, action.direction, action.power, 8.0);
    let position = Vec3d {
        y: entity.position().y + f64::from(entity.eye_height()) - f64::from(0.1_f32),
        ..entity.position()
    };
    let Some(_) = spawn_splash_potion(
        store,
        position,
        ProjectileShot::new(movement),
        item,
        Some(entity.uuid()),
    ) else {
        return;
    };
    if !entity.is_silent() {
        let pitch = store
            .mob_ai_mut(entity.id())
            .map_or(1.0, crate::entity::ai::mob::MobAiRuntime::witch_throw_pitch);
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.witch.throw".to_string(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
}

pub(crate) fn damage_multiplier(entity: &Entity, source: StoredEntityDamageSource<'_>) -> f32 {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID {
        return 1.0;
    }
    if source
        .causing_entity
        .is_some_and(|causing| causing.entity_id() == entity.id())
    {
        return 0.0;
    }
    if tag_contains(
        "minecraft:witch_resistant_to",
        source.damage_type.resource_id.as_str(),
    ) {
        0.15
    } else {
        1.0
    }
}

#[cfg(test)]
pub(crate) fn remember_damage(
    entity: &mut Entity,
    store: &mut EntityStore,
    source: StoredEntityDamageSource<'_>,
    outcome: StoredEntityHurtOutcome,
    health_damage: bool,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID {
        return;
    }
    if !outcome.succeeded {
        return;
    }
    let is_fire = tag_contains("minecraft:is_fire", source.damage_type.resource_id.as_str());
    let game_time = store.game_time();
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(witch) = data.witch_state_mut() {
            witch.last_fire_damage_time = is_fire.then_some(game_time);
        }
    }
    if health_damage && !entity.is_silent() {
        let pitch = store
            .mob_ai_mut(entity.id())
            .map_or(1.0, |runtime| runtime.voice_pitch(false));
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: if entity.is_alive() {
                "minecraft:entity.witch.hurt"
            } else {
                "minecraft:entity.witch.death"
            }
            .to_string(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
}

pub(crate) fn remember_damage_with_is_fire(
    entity: &mut Entity,
    outcome: StoredEntityHurtOutcome,
    is_fire: bool,
    game_time: i64,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID || !outcome.succeeded {
        return;
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(witch) = data.witch_state_mut() {
            witch.last_fire_damage_time = is_fire.then_some(game_time);
        }
    }
}

fn tag_contains(tag_id: &str, member: &str) -> bool {
    match vanilla_persistent_registry_tag_members_26_1("minecraft:damage_type", tag_id) {
        PersistentRegistryTagMembers::Present(members) => {
            members.iter().any(|value| value == member)
        }
        PersistentRegistryTagMembers::Absent | PersistentRegistryTagMembers::Unavailable => false,
    }
}

pub(crate) fn tick_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITCH_ID {
        return;
    }
    let pitch = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.tick_mob_ambient_pitch(entity.is_alive()));
    if let Some(pitch) = pitch.filter(|_| !entity.is_silent()) {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.witch.ambient".to_string(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
}

#[cfg(test)]
mod tests;
