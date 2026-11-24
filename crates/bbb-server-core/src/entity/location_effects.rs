use crate::world::simulation_level::LevelCtx;
use std::sync::Arc;

use crate::enchantment_entity_effects::{sample_float_provider, EnchantmentEntityEffect};
use crate::enchantment_facts::EnchantmentAttributeEffect;
use crate::enchantment_location_effects::{
    EnchantmentEffectTable, EnchantmentLocationTable, EnchantmentTickTable, LocationEffect,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_registry_codec::level_based_value::LevelBasedValue;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_worldgen::state_providers::BlockStateProviderExt;

use crate::entity::attributes::{AttributeMap, AttributeModifier, AttributeOperation};
use crate::entity::living::LivingEntityState;
use crate::entity::{Entity, EntityStore};

type SlotLocationEffects = Vec<(String, Vec<LocationEffect>)>;

/// Each slot's enchantments and their active effects, in insertion order
/// (`ObjectArraySet`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActiveLocationEffects {
    slots: Vec<(EquipmentSlot, SlotLocationEffects)>,
}

impl ActiveLocationEffects {
    fn enchantments(&mut self, slot: EquipmentSlot) -> &mut Vec<(String, Vec<LocationEffect>)> {
        let index = match self.slots.iter().position(|(entry, _)| *entry == slot) {
            Some(index) => index,
            None => {
                self.slots.push((slot, Vec::new()));
                self.slots.len() - 1
            }
        };
        &mut self.slots[index].1
    }

    fn take(&mut self, slot: EquipmentSlot, enchantment: &str) -> Option<Vec<LocationEffect>> {
        let enchantments = self.enchantments(slot);
        let index = enchantments.iter().position(|(id, _)| id == enchantment)?;
        Some(enchantments.remove(index).1)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|(_, enchantments)| enchantments.is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocationEffectsState {
    pub active: ActiveLocationEffects,
    pub last_block_pos: Option<BlockPos>,
}

impl EntityStore {
    pub fn set_enchantment_location_effects(&mut self, effects: Arc<EnchantmentLocationTable>) {
        self.enchantment_location_effects = effects;
    }

    #[must_use]
    pub fn enchantment_location_effects_shared(&self) -> Arc<EnchantmentLocationTable> {
        Arc::clone(&self.enchantment_location_effects)
    }

    pub fn set_enchantment_tick_effects(&mut self, effects: Arc<EnchantmentTickTable>) {
        self.enchantment_tick_effects = effects;
    }

    #[must_use]
    pub fn enchantment_tick_effects_shared(&self) -> Arc<EnchantmentTickTable> {
        Arc::clone(&self.enchantment_tick_effects)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WornEnchantment {
    pub slot: EquipmentSlot,
    pub enchantment: String,
    pub level: i32,
}

/// What a wearer lends the effects.
pub(crate) enum LocationEffectTarget<'a, 'l, 'w> {
    Stored(StoredHost<'a, 'l, 'w>),
    Player(crate::world::location_effects::PlayerHost<'a>),
}

impl LocationEffectTarget<'_, '_, '_> {
    fn attributes(&mut self) -> Option<&mut AttributeMap> {
        match self {
            Self::Stored(target) => target.attributes(),
            Self::Player(target) => target.attributes(),
        }
    }

    fn requirements_match(
        &mut self,
        enchantment: &str,
        level: i32,
        active: Option<bool>,
        condition: &LootCondition,
    ) -> Result<bool, String> {
        match self {
            Self::Stored(target) => {
                target.requirements_match(enchantment, level, active, condition)
            }
            Self::Player(target) => {
                target.requirements_match(enchantment, level, active, condition)
            }
        }
    }

    fn apply_entity_effect(
        &mut self,
        worn: &WornEnchantment,
        effect: &EnchantmentEntityEffect,
    ) -> Result<(), String> {
        match self {
            Self::Stored(target) => target.apply_entity_effect(worn, effect),
            Self::Player(target) => target.apply_entity_effect(worn, effect),
        }
    }
}

pub fn worn_location_enchantments<'a, T>(
    table: &EnchantmentEffectTable<T>,
    stacks: impl IntoIterator<Item = (EquipmentSlot, Vec<(String, i32)>)>,
    slots_of: impl Fn(&str) -> Option<&'a [EquipmentSlot]>,
) -> Vec<WornEnchantment> {
    let mut worn = Vec::new();
    for (slot, enchantments) in stacks {
        for (enchantment, level) in enchantments {
            if table.contains_key(&enchantment)
                && slots_of(&enchantment).is_some_and(|slots| slots.contains(&slot))
            {
                worn.push(WornEnchantment {
                    slot,
                    enchantment,
                    level,
                });
            }
        }
    }
    worn
}

/// `Enchantment.runLocationChangedEffects` for each of `worn`.
pub(crate) fn run_location_changed_effects(
    active: &mut ActiveLocationEffects,
    worn: &[WornEnchantment],
    table: &EnchantmentLocationTable,
    host: &mut LocationEffectTarget<'_, '_, '_>,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for item in worn {
        let effects = match table.get(&item.enchantment) {
            Some(Ok(effects)) => effects,
            Some(Err(detail)) => {
                failures.push(detail.clone());
                continue;
            }
            None => continue,
        };
        let mut active_effects = active.take(item.slot, &item.enchantment);
        for conditional in effects {
            let effect = &conditional.effect;
            let was_active = active_effects
                .as_ref()
                .is_some_and(|set| set.contains(effect));
            let matches = match &conditional.requirements {
                None => Ok(true),
                Some(condition) => host.requirements_match(
                    &item.enchantment,
                    item.level,
                    Some(was_active),
                    condition,
                ),
            };
            match matches {
                Ok(true) => {
                    if !was_active {
                        active_effects
                            .get_or_insert_with(Vec::new)
                            .push(effect.clone());
                    }
                    if let Err(detail) = changed_block(host, item, effect, !was_active) {
                        failures.push(detail);
                    }
                }
                Ok(false) => {
                    let removed = active_effects.as_mut().and_then(|set| {
                        let index = set.iter().position(|entry| entry == effect)?;
                        Some(set.remove(index))
                    });
                    if let Some(removed) = removed {
                        if let Err(detail) = deactivated(host, item, &removed) {
                            failures.push(detail);
                        }
                    }
                }
                Err(detail) => failures.push(detail),
            }
        }
        if let Some(set) = active_effects.filter(|set| !set.is_empty()) {
            active
                .enchantments(item.slot)
                .push((item.enchantment.clone(), set));
        }
    }
    joined(failures)
}

/// `Enchantment.stopLocationBasedEffects` for each of `worn`.
pub(crate) fn stop_location_based_effects(
    active: &mut ActiveLocationEffects,
    worn: &[WornEnchantment],
    host: &mut LocationEffectTarget<'_, '_, '_>,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for item in worn {
        for effect in active
            .take(item.slot, &item.enchantment)
            .unwrap_or_default()
        {
            if let Err(detail) = deactivated(host, item, &effect) {
                failures.push(detail);
            }
        }
    }
    joined(failures)
}

pub(crate) fn run_tick_effects(
    worn: &[WornEnchantment],
    table: &EnchantmentTickTable,
    host: &mut LocationEffectTarget<'_, '_, '_>,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for item in worn {
        let effects = match table.get(&item.enchantment) {
            Some(Ok(effects)) => effects,
            Some(Err(detail)) => {
                failures.push(detail.clone());
                continue;
            }
            None => continue,
        };
        for conditional in effects {
            let matches = match &conditional.requirements {
                None => Ok(true),
                Some(condition) => {
                    host.requirements_match(&item.enchantment, item.level, None, condition)
                }
            };
            match matches.and_then(|matches| {
                if matches {
                    host.apply_entity_effect(item, &conditional.effect)
                } else {
                    Ok(())
                }
            }) {
                Ok(()) => {}
                Err(detail) => failures.push(detail),
            }
        }
    }
    joined(failures)
}

fn joined(failures: Vec<String>) -> Result<(), String> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn changed_block(
    host: &mut LocationEffectTarget<'_, '_, '_>,
    item: &WornEnchantment,
    effect: &LocationEffect,
    became_active: bool,
) -> Result<(), String> {
    match effect {
        LocationEffect::AllOf(effects) => {
            for effect in effects {
                changed_block(host, item, effect, became_active)?;
            }
            Ok(())
        }
        LocationEffect::Attribute(attribute) => {
            if became_active {
                let modifier = enchantment_attribute_modifier(attribute, item)?;
                if let Some(instance) = host
                    .attributes()
                    .and_then(|attributes| attributes.get_instance(attribute.attribute_id))
                {
                    // `AttributeMap.addTransientAttributeModifiers`.
                    instance.remove_modifier(&modifier.id);
                    instance.add_or_update_transient_modifier(modifier);
                }
            }
            Ok(())
        }
        LocationEffect::Entity(effect) => host.apply_entity_effect(item, effect),
    }
}

fn deactivated(
    host: &mut LocationEffectTarget<'_, '_, '_>,
    item: &WornEnchantment,
    effect: &LocationEffect,
) -> Result<(), String> {
    match effect {
        LocationEffect::AllOf(effects) => {
            for effect in effects {
                deactivated(host, item, effect)?;
            }
            Ok(())
        }
        LocationEffect::Attribute(attribute) => {
            if let Some(instance) = host
                .attributes()
                .and_then(|attributes| attributes.get_instance(attribute.attribute_id))
            {
                instance.remove_modifier(&attribute.modifier_id(item.slot));
            }
            Ok(())
        }
        // `EnchantmentEntityEffect` keeps the interface's empty `onDeactivated`.
        LocationEffect::Entity(_) => Ok(()),
    }
}

fn enchantment_attribute_modifier(
    effect: &EnchantmentAttributeEffect,
    item: &WornEnchantment,
) -> Result<AttributeModifier, String> {
    Ok(AttributeModifier {
        id: effect.modifier_id(item.slot),
        amount: f64::from(effect.amount.calculate(item.level)),
        operation: AttributeOperation::from_serialized_name(&effect.operation)
            .ok_or_else(|| format!("{} names operation {}", item.enchantment, effect.operation))?,
    })
}

#[must_use]
pub fn replace_disk_cells(
    radius: &LevelBasedValue,
    height: &LevelBasedValue,
    offset: [i32; 3],
    level: i32,
    position: Vec3d,
) -> Vec<BlockPos> {
    let center = BlockPos {
        x: (position.x.floor() as i32).wrapping_add(offset[0]),
        y: (position.y.floor() as i32).wrapping_add(offset[1]),
        z: (position.z.floor() as i32).wrapping_add(offset[2]),
    };
    let dist = radius.calculate(level) as i32;
    let height = height.calculate(level) as i32;
    let top = center.y.wrapping_add(height.wrapping_sub(1).min(0));
    let (min_x, max_x) = ordered(center.x.wrapping_sub(dist), center.x.wrapping_add(dist));
    let (min_y, max_y) = ordered(center.y, top);
    let (min_z, max_z) = ordered(center.z.wrapping_sub(dist), center.z.wrapping_add(dist));
    let radius_sqr = f64::from(dist) * f64::from(dist);
    let mut cells = Vec::new();
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // `Vec3i.distToCenterSqr` at `(x, pos.y + 0.5, z)`.
                let dx = f64::from(x) + 0.5 - position.x;
                let dz = f64::from(z) + 0.5 - position.z;
                if dx * dx + dz * dz < radius_sqr {
                    cells.push(BlockPos { x, y, z });
                }
            }
        }
    }
    cells
}

fn ordered(a: i32, b: i32) -> (i32, i32) {
    (a.min(b), a.max(b))
}

fn collect_stored_worn<'a, T>(
    table: &EnchantmentEffectTable<T>,
    registry: &[crate::enchantment_facts::EnchantmentSlots],
    stacks: impl Iterator<Item = (EquipmentSlot, &'a RuntimeItemStack)>,
) -> Vec<WornEnchantment> {
    if table.is_empty() {
        return Vec::new();
    }
    let mut worn = Vec::new();
    for (slot, stack) in stacks {
        let patch = &stack.summary().component_patch;
        if stack.is_empty()
            || patch.enchantments.is_empty()
            || !patch.added_type_ids.contains(
                &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            )
        {
            continue;
        }
        for entry in &patch.enchantments {
            let Ok(index) = usize::try_from(entry.holder_id) else {
                continue;
            };
            let Some(enchantment) = registry.get(index) else {
                continue;
            };
            if enchantment.slots.contains(&slot) && table.contains_key(&enchantment.resource_id) {
                worn.push(WornEnchantment {
                    slot,
                    enchantment: enchantment.resource_id.clone(),
                    level: entry.level,
                });
            }
        }
    }
    worn
}

pub(crate) fn check_changed_block(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let pos = entity.block_position();
    let Some(living) = entity.living_mut() else {
        return;
    };
    if living.location_effects.last_block_pos != Some(pos) {
        living.location_effects.last_block_pos = Some(pos);
        on_changed_block(entity, store, level);
    }
}

pub(crate) fn tick_effects(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if store.enchantment_tick_effects.is_empty() {
        return;
    }
    let Some(living) = entity.living() else {
        return;
    };
    let worn = collect_stored_worn(
        &store.enchantment_tick_effects,
        store.enchantment_registry(),
        VANILLA_EQUIPMENT_SLOTS.iter().filter_map(|entry| {
            living
                .item_by_slot(entry.slot)
                .map(|stack| (entry.slot, stack))
        }),
    );
    if worn.is_empty() {
        return;
    }
    let table = store.enchantment_tick_effects_shared();
    let _ = with_stored_host(entity, store, level, |_, _, host| {
        run_tick_effects(&worn, &table, host)
    });
}

pub(crate) fn on_changed_block(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if store.enchantment_location_effects.is_empty() {
        return;
    }
    let Some(living) = entity.living() else {
        return;
    };
    let worn = collect_stored_worn(
        &store.enchantment_location_effects,
        store.enchantment_registry(),
        VANILLA_EQUIPMENT_SLOTS.iter().filter_map(|entry| {
            living
                .item_by_slot(entry.slot)
                .map(|stack| (entry.slot, stack))
        }),
    );
    if worn.is_empty() {
        return;
    }
    let _ = with_stored_host(entity, store, level, |active, table, host| {
        run_location_changed_effects(active, &worn, table, host)
    });
}

pub(crate) fn equipment_changed(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    changed: &[(EquipmentSlot, Option<RuntimeItemStack>)],
    previous: &[Option<RuntimeItemStack>],
) {
    if store.enchantment_location_effects.is_empty() {
        return;
    }
    let previous = collect_stored_worn(
        &store.enchantment_location_effects,
        store.enchantment_registry(),
        changed
            .iter()
            .zip(previous)
            .filter_map(|((slot, _), previous)| Some((*slot, previous.as_ref()?))),
    );
    let current = collect_stored_worn(
        &store.enchantment_location_effects,
        store.enchantment_registry(),
        changed.iter().filter_map(|(slot, current)| {
            let current = current.as_ref()?;
            (!crate::entity::item_components::is_broken(current).unwrap_or(true))
                .then_some((*slot, current))
        }),
    );
    if previous.is_empty() && current.is_empty() {
        return;
    }
    let _ = with_stored_host(entity, store, level, |active, table, host| {
        let stopped = stop_location_based_effects(active, &previous, host);
        let ran = run_location_changed_effects(active, &current, table, host);
        stopped.and(ran)
    });
}

pub(crate) fn stop_equipped_slot(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slot: EquipmentSlot,
) -> Result<(), String> {
    if store.enchantment_location_effects.is_empty() {
        return Ok(());
    }
    let Some(stack) = entity.living().and_then(|living| living.item_by_slot(slot)) else {
        return Ok(());
    };
    let worn = collect_stored_worn(
        &store.enchantment_location_effects,
        store.enchantment_registry(),
        std::iter::once((slot, stack)),
    );
    if worn.is_empty() {
        return Ok(());
    }
    with_stored_host(entity, store, level, |active, _, host| {
        stop_location_based_effects(active, &worn, host)
    })
}

fn with_stored_host(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    body: impl FnOnce(
        &mut ActiveLocationEffects,
        &EnchantmentLocationTable,
        &mut LocationEffectTarget<'_, '_, '_>,
    ) -> Result<(), String>,
) -> Result<(), String> {
    let Some(living) = entity.living_mut() else {
        return Ok(());
    };
    let mut active = std::mem::take(&mut living.location_effects.active);
    let table = store.enchantment_location_effects_shared();
    let result = body(
        &mut active,
        &table,
        &mut LocationEffectTarget::Stored(StoredHost {
            entity: &mut *entity,
            store,
            level,
        }),
    );
    if let Some(living) = entity.living_mut() {
        living.location_effects.active = active;
    }
    result
}

pub(crate) struct StoredHost<'a, 'l, 'w> {
    entity: &'a mut Entity,
    store: &'a mut EntityStore,
    level: &'a mut LevelCtx<'l, 'w>,
}

impl StoredHost<'_, '_, '_> {
    fn attributes(&mut self) -> Option<&mut AttributeMap> {
        self.entity
            .living_mut()
            .map(LivingEntityState::attributes_mut)
    }

    fn requirements_match(
        &mut self,
        enchantment: &str,
        enchantment_level: i32,
        active: Option<bool>,
        condition: &LootCondition,
    ) -> Result<bool, String> {
        let this =
            crate::entity::mob_death_loot::stored_loot_entity(self.store, self.level, self.entity);
        self.level
            .location_effect_requirements(LocationRequirementRequest {
                this,
                enchantment,
                enchantment_level,
                active,
                condition,
            })
    }

    fn apply_entity_effect(
        &mut self,
        worn: &WornEnchantment,
        effect: &EnchantmentEntityEffect,
    ) -> Result<(), String> {
        apply_stored_entity_effect(self.entity, self.store, self.level, worn, effect)
    }
}

pub struct LocationRequirementRequest<'a> {
    pub this: crate::loot_runtime::LootEntity<'a>,
    pub enchantment: &'a str,
    pub enchantment_level: i32,
    pub active: Option<bool>,
    pub condition: &'a LootCondition,
}

fn apply_stored_entity_effect(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    worn: &WornEnchantment,
    effect: &EnchantmentEntityEffect,
) -> Result<(), String> {
    match effect {
        EnchantmentEntityEffect::AllOf(effects) => {
            for effect in effects {
                apply_stored_entity_effect(entity, store, level, worn, effect)?;
            }
            Ok(())
        }
        EnchantmentEntityEffect::RunFunction(function) => {
            use crate::commands::types::CommandDamageEntityRef;
            use crate::commands::{CommandOrigin, CommandPermission, CommandStoredEntity};
            let position = entity.position();
            let dimension = level.dimension_key();
            let executing = CommandDamageEntityRef::Stored(CommandStoredEntity {
                entity,
                dimension,
                registry_tags: None,
            })
            .as_command_source(CommandPermission::GameMaster);
            let origin = CommandOrigin {
                position: [position.x, position.y, position.z],
                rotation: [entity.y_rot(), entity.x_rot()],
                level: dimension.clone(),
                anchor: bbb_protocol::entity_anchors::EntityAnchor::Feet,
                eye_height: Some(entity.eye_height()),
            };
            store.queue_command(crate::entity::EntityCommand::EnchantmentFunction {
                function: function.clone(),
                executing,
                origin,
            });
            Ok(())
        }
        EnchantmentEntityEffect::ChangeItemDamage(amount) => {
            let change = amount.calculate(worn.level) as i32;
            crate::entity::equipment_wear::hurt_and_break(entity, store, level, worn.slot, change)
                .map(|_| ())
        }
        EnchantmentEntityEffect::ReplaceBlock { .. }
        | EnchantmentEntityEffect::SetBlockProperties { .. } => {
            let mut level = level.with_entity_context(
                crate::entity::level::EntityBlockContext::new(Some(store), Some(entity)),
            );
            crate::world::location_effects::apply_block_effect(
                &mut level,
                effect,
                entity.position(),
                Some(entity.id()),
            )
        }
        EnchantmentEntityEffect::ReplaceDisk {
            radius,
            height,
            offset,
            predicate,
            block_state,
            trigger_game_event,
        } => {
            let event = replace_disk_game_event(trigger_game_event.as_deref())?;
            let position = entity.position();
            for pos in replace_disk_cells(radius, height, *offset, worn.level, position) {
                if let Some(predicate) = predicate {
                    let unobstructed = |pos| stored_unobstructed(entity, store, level, pos);
                    if !level.block_predicate_test(predicate, pos, &unobstructed)? {
                        continue;
                    }
                }
                let state = {
                    let level =
                        level.with_entity_context(crate::entity::level::EntityBlockContext::new(
                            Some(&mut *store),
                            Some(entity),
                        ));
                    block_state
                        .get_state(&level, &mut level.level_random(), pos)
                        .ok_or("the block state provider cannot sample this position")?
                };
                let placed = level.set_entity_block(
                    crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
                    pos,
                    state,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
                if let (true, Some(event)) = (placed, event) {
                    level.game_event(event, pos, Some(entity.id()));
                }
            }
            Ok(())
        }
        EnchantmentEntityEffect::PlaySound {
            sounds,
            volume,
            pitch,
        } => {
            if entity.is_silent() || sounds.is_empty() {
                return Ok(());
            }
            let index = (worn.level - 1).clamp(0, sounds.len() as i32 - 1) as usize;
            let random = store.random().clone();
            let volume = sample_float_provider(volume, || random.next_float());
            let pitch = sample_float_provider(pitch, || random.next_float());
            crate::entity::sound_variants::play(
                store,
                sounds[index].clone(),
                entity.position(),
                crate::entity::living::living_sound_source(entity),
                volume,
                pitch,
            );
            Ok(())
        }
        EnchantmentEntityEffect::SpawnParticles(particles) => {
            let bounds = entity.bounding_box();
            let dimensions = [
                (bounds.max_x - bounds.min_x) as f32,
                (bounds.max_y - bounds.min_y) as f32,
            ];
            let movement = known_movement(
                entity,
                crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(entity),
                    level.detached_bodies(),
                ),
            )
            .ok_or("the known movement of an entity a player controls is not tracked here")?;
            let position = entity.position();
            let random = store.random().clone();
            let packet = particles.packet(position, dimensions, movement, &random);
            store.send_particles(packet);
            Ok(())
        }
        // `ApplyExhaustion.apply` only feeds a player (`ApplyExhaustion.java:18-22`).
        EnchantmentEntityEffect::ApplyExhaustion(_) => Ok(()),
        EnchantmentEntityEffect::ApplyMobEffect { .. }
        | EnchantmentEntityEffect::ApplyImpulse { .. }
        | EnchantmentEntityEffect::DamageEntity { .. }
        | EnchantmentEntityEffect::Explode(_)
        | EnchantmentEntityEffect::Ignite { .. }
        | EnchantmentEntityEffect::SummonEntity { .. } => {
            store.queue_command(crate::entity::EntityCommand::EnchantmentEntityEffect {
                entity_id: entity.id(),
                slot: worn.slot,
                level: worn.level,
                effect: effect.clone(),
                position: entity.position(),
            });
            Ok(())
        }
        EnchantmentEntityEffect::Unmodelled(effect_type) => Err(format!(
            "{effect_type} as a location effect of a stored entity has no owner"
        )),
    }
}

pub fn known_movement(
    entity: &Entity,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
) -> Option<Vec3d> {
    crate::world::ServerWorld::stored_known_movement_in_bodies(entity, bodies)
}

/// `ReplaceDisk.triggerGameEvent` as one of the events this server raises.
pub fn replace_disk_game_event(
    event: Option<&str>,
) -> Result<Option<crate::level_event::VanillaGameEvent>, String> {
    let place = crate::level_event::VanillaGameEvent::BLOCK_PLACE;
    match event {
        None => Ok(None),
        Some(id) if id == place.resource_id() => Ok(Some(place)),
        Some(id) => Err(format!("replace_disk game event {id} has no owner")),
    }
}

fn stored_unobstructed(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> bool {
    let cell = bbb_server_types::geometry::Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + 1.0,
        max_z: f64::from(pos.z) + 1.0,
    };
    let blocks =
        |candidate: &Entity| candidate.removal_reason().is_none() && candidate.blocks_building();
    if blocks(entity) && entity.bounding_box().intersects(cell) {
        return false;
    }
    if store.first_entity_in_box(cell, blocks).is_some() {
        return false;
    }
    !level.players_in_box(cell).any(|player| !player.spectator)
}
