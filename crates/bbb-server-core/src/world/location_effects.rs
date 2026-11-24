use crate::connection::ConnectionId;
use crate::enchantment_entity_effects::EnchantmentEntityEffect;
use crate::entity::attributes::AttributeMap;
use crate::entity::location_effects::{
    run_location_changed_effects, run_tick_effects, stop_location_based_effects,
    ActiveLocationEffects, LocationEffectTarget, WornEnchantment,
};
use crate::loot_runtime::{
    matches_conditions, param_sets, LootBlockStateAt, LootEntities, LootEntity, LootOrigin,
    LootParams, LootWeather,
};
use crate::random::GameRandom;
use crate::world::LevelId;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::loot::{LootCondition, LootRegistries};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_worldgen::state_providers::BlockStateProviderExt;

use super::combat::{EnchantedItem, EnchantmentEffectEntity as SourceEntity};
use super::simulation_level::LevelCtx;
use super::{GameType, ServerWorld, WorldEvent};

/// `getOnPos(0.500001F)` (`Entity.java:988-990`).
const MOVEMENT_BLOCK_OFFSET: f32 = 0.500_001;

pub(in crate::world) struct LocationContextLevel<'a> {
    pub(in crate::world) vehicle_slots:
        Option<&'a crate::loot_runtime::params::LootVehicleSlots<'a>>,
    pub(in crate::world) item_environment: Option<&'a dyn crate::loot_runtime::LootItemEnvironment>,
    pub(in crate::world) loot_tables: Option<&'a LootRegistries>,
    pub(in crate::world) dimension: Option<&'a str>,
    pub(in crate::world) blocks: &'a LootBlockStateAt<'a>,
    pub(in crate::world) block_entity_data: &'a crate::loot_runtime::LootBlockEntityDataAt<'a>,
    pub(in crate::world) weather: LootWeather,
}

/// One requirement under `Enchantment.locationContext`, or `entityContext`
/// when the request carries no `active`.
pub(in crate::world) fn location_condition_matches(
    context: &LocationContextLevel<'_>,
    random: &GameRandom,
    request: crate::entity::location_effects::LocationRequirementRequest<'_>,
) -> Result<bool, String> {
    let origin = request
        .this
        .position()
        .ok_or("the wearer's position is unavailable")?;
    let params = LootParams {
        vehicle_slots: context.vehicle_slots,
        item_environment: context.item_environment,
        origin: Some(LootOrigin {
            x: origin.x,
            y: origin.y,
            z: origin.z,
        }),
        entities: LootEntities {
            this_entity: request.this,
            ..LootEntities::default()
        },
        enchantment_level: Some(request.enchantment_level),
        enchantment_active: request.active,
        dimension: context.dimension,
        level: Some(context.blocks),
        block_entity_data: Some(context.block_entity_data),
        weather: Some(context.weather),
        ..LootParams::new(match request.active {
            Some(_) => param_sets::ENCHANTED_LOCATION,
            None => param_sets::ENCHANTED_ENTITY,
        })
    };
    matches_conditions(
        context.loot_tables.ok_or("no loot registries are loaded")?,
        request.enchantment,
        std::slice::from_ref(request.condition),
        &params,
        random,
    )
    .map_err(|error| error.to_string())
}

/// The unit cell at `pos`, as `Shapes.block().move(pos)` bounds it.
fn cell(pos: BlockPos) -> bbb_server_types::geometry::Aabb {
    bbb_server_types::geometry::Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + 1.0,
        max_z: f64::from(pos.z) + 1.0,
    }
}

/// `EntityGetter.isUnobstructed` for the unit cell at `pos` over the level's
/// stored entities and session players.
pub(in crate::world) fn level_unobstructed(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let cell = cell(pos);
    let stored = level.structure_entities.as_ref().is_some_and(|context| {
        let store = &*context.store;
        let blocks = |entity: &crate::entity::Entity| {
            entity.removal_reason().is_none() && entity.blocks_building()
        };
        context
            .current
            .is_some_and(|entity| blocks(entity) && entity.bounding_box().intersects(cell))
            || store.first_entity_in_box(cell, blocks).is_some()
    });
    !stored && !level.players_in_box(cell).any(|player| !player.spectator)
}

pub(crate) fn apply_block_effect(
    level: &mut LevelCtx<'_, '_>,
    effect: &EnchantmentEntityEffect,
    position: Vec3d,
    source: Option<i32>,
) -> Result<(), String> {
    let (offset, trigger_game_event) = match effect {
        EnchantmentEntityEffect::ReplaceBlock {
            offset,
            trigger_game_event,
            ..
        }
        | EnchantmentEntityEffect::SetBlockProperties {
            offset,
            trigger_game_event,
            ..
        } => (*offset, trigger_game_event.as_deref()),
        _ => return Err("not a single-block enchantment effect".to_string()),
    };
    let event = trigger_game_event
        .map(|id| {
            crate::level_event::VanillaGameEvent::from_resource_id(id)
                .ok_or_else(|| format!("enchantment game event {id} has no owner"))
        })
        .transpose()?;
    let pos = BlockPos {
        x: (position.x.floor() as i32).wrapping_add(offset[0]),
        y: (position.y.floor() as i32).wrapping_add(offset[1]),
        z: (position.z.floor() as i32).wrapping_add(offset[2]),
    };
    let state = match effect {
        EnchantmentEntityEffect::ReplaceBlock {
            predicate,
            block_state,
            ..
        } => {
            if let Some(predicate) = predicate {
                if !level
                    .block_predicate_test(predicate, pos, &|pos| level_unobstructed(level, pos))?
                {
                    return Ok(());
                }
            }
            block_state
                .get_state(level, &mut level.level_random(), pos)
                .ok_or("the block state provider cannot sample this position")?
        }
        EnchantmentEntityEffect::SetBlockProperties { properties, .. } => {
            use bbb_protocol::block_state_types::{
                vanilla_block_property_kind, vanilla_block_property_value_rank,
                vanilla_block_state_block_type_id, vanilla_block_state_with_property,
                VanillaBlockPropertyKind,
            };
            let old = level.block_state(pos);
            let mut state = old;
            let Some(block) = vanilla_block_state_block_type_id(old)
                .and_then(bbb_protocol::block_types::vanilla_block_type)
            else {
                return Ok(());
            };
            for (name, value) in properties {
                let canonical;
                let value = match vanilla_block_property_kind(block.resource_id, name) {
                    Some(kind @ VanillaBlockPropertyKind::Integer { .. }) => {
                        let Some(rank) = vanilla_block_property_value_rank(kind, value) else {
                            continue;
                        };
                        canonical = rank.to_string();
                        &canonical
                    }
                    _ => value,
                };
                if let Some(modified) = vanilla_block_state_with_property(state, name, value) {
                    state = modified;
                }
            }
            if state == old {
                return Ok(());
            }
            state
        }
        _ => unreachable!(),
    };
    if level.set_block(
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    ) {
        if let Some(event) = event {
            level.game_event(event, pos, source);
        }
    }
    Ok(())
}

impl ServerWorld {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn replace_disk(
        &mut self,
        dimension: &LevelId,
        effect: &EnchantmentEntityEffect,
        level: i32,
        target: SourceEntity,
        position: Vec3d,
    ) -> Result<(), String> {
        let EnchantmentEntityEffect::ReplaceDisk {
            radius,
            height,
            offset,
            predicate,
            block_state,
            trigger_game_event,
        } = effect
        else {
            return Err("not a replace_disk effect".to_string());
        };
        let event = crate::entity::location_effects::replace_disk_game_event(
            trigger_game_event.as_deref(),
        )?;
        let source = match target {
            SourceEntity::Session(id) => self.levels.player(id).map(|player| player.entity_id()),
            SourceEntity::Stored(id) => Some(id),
        };
        let cells = crate::entity::location_effects::replace_disk_cells(
            radius, height, *offset, level, position,
        );
        self.with_level(*dimension, |level| {
            for pos in cells {
                if let Some(predicate) = predicate {
                    let unobstructed = |pos| level_unobstructed(level, pos);
                    if !level.block_predicate_test(predicate, pos, &unobstructed)? {
                        continue;
                    }
                }
                let state = block_state
                    .get_state(level, &mut level.level_random(), pos)
                    .ok_or("the block state provider cannot sample this position")?;
                if level.set_block(
                    pos,
                    state,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                    crate::neighbor_updater::UPDATE_LIMIT,
                ) {
                    if let Some(event) = event {
                        level
                            .event_sink
                            .game_event_with_source(event, pos, source, None);
                    }
                }
            }
            Ok(())
        })
    }

    pub(in crate::world) fn apply_single_block_effect(
        &mut self,
        dimension: &LevelId,
        effect: &EnchantmentEntityEffect,
        target: SourceEntity,
        position: Vec3d,
    ) -> Result<(), String> {
        let source = match target {
            SourceEntity::Session(id) => self.levels.player(id).map(|player| player.entity_id()),
            SourceEntity::Stored(id) => Some(id),
        };
        self.with_level(*dimension, |level| {
            apply_block_effect(level, effect, position, source)
        })
    }

    pub(in crate::world) fn tick_player_enchantment_effects(
        &mut self,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.datapack_tables.has_tick_effects() {
            return;
        }
        let mut ticking = Vec::new();
        for (&id, player) in self.levels.player_entries() {
            if !player.body_tick_enabled {
                continue;
            }
            let worn = self.player_worn_matching(
                super::datapack_install::DatapackRuntimeTables::holder_has_tick_effect,
                player.equipment_slots(),
            );
            if !worn.is_empty() {
                ticking.push((id, worn));
            }
        }
        if ticking.is_empty() {
            return;
        }
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_tick_effects);
        for (id, worn) in ticking {
            let result = self.with_player_host(id, events, |_, _, host| {
                run_tick_effects(&worn, &table, host)
            });
            if let Err(detail) = result {
                tracing::warn!("location-changed enchantment effects: {detail}");
            }
        }
    }

    pub(in crate::world) fn player_on_changed_block(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if self
            .levels
            .player(id)
            .is_some_and(|player| player.game_type != GameType::Spectator)
        {
            self.run_player_location_effects(id, events);
        }
    }

    pub(in crate::world) fn player_on_changed_block_checked(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        if self.levels.player(id)?.game_type == GameType::Spectator {
            return Some(());
        }
        self.run_player_location_effects_checked(id, events).ok()
    }

    pub(in crate::world) fn check_player_changed_block(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player_mut(id) else {
            return;
        };
        let pos = super::block_position(player.position());
        if player.cold.location_effects.last_block_pos != Some(pos) {
            player.cold.location_effects.last_block_pos = Some(pos);
            self.player_on_changed_block(id, events);
        }
    }

    pub(in crate::world) fn game_mode_location_effects(
        &mut self,
        id: ConnectionId,
        was_spectator: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        match self.levels.player(id).map(|player| player.game_type) {
            Some(GameType::Spectator) => self.stop_player_location_effects(id, events),
            Some(_) if was_spectator => self.run_player_location_effects(id, events),
            _ => {}
        }
    }

    fn run_player_location_effects(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let result = self.run_player_location_effects_checked(id, events);
        if let Err(detail) = result {
            tracing::warn!("location-changed enchantment effects: {detail}");
        }
    }

    fn run_player_location_effects_checked(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let player = self.levels.player(id).ok_or("the session left")?;
        if !self.datapack_tables.has_location_effects() {
            return Ok(());
        }
        let worn = self.player_worn_matching(
            super::datapack_install::DatapackRuntimeTables::holder_has_location_effect,
            player.equipment_slots(),
        );
        if worn.is_empty() {
            return Ok(());
        }
        self.with_player_host(id, events, |active, table, host| {
            run_location_changed_effects(active, &worn, table, host)
        })
    }

    pub(in crate::world) fn stop_player_slot_location_effects_checked(
        &mut self,
        id: ConnectionId,
        slot: EquipmentSlot,
        current: &RuntimeItemStack,
    ) -> Result<(), String> {
        let _player = self.levels.player(id).ok_or("the session left")?;
        if !self.datapack_tables.has_location_effects() {
            return Ok(());
        }
        let worn = self.player_worn_matching(
            super::datapack_install::DatapackRuntimeTables::holder_has_location_effect,
            std::iter::once((slot, current)),
        );
        if worn.is_empty() {
            return Ok(());
        }
        self.with_player_host(id, &mut Vec::new(), |active, _, host| {
            stop_location_based_effects(active, &worn, host)
        })
    }

    fn stop_player_location_effects(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        if !self.datapack_tables.has_location_effects() {
            return;
        }
        let Some(player) = self.levels.player(id) else {
            return;
        };
        let worn = self.player_worn_matching(
            super::datapack_install::DatapackRuntimeTables::holder_has_location_effect,
            player.equipment_slots(),
        );
        if worn.is_empty() {
            return;
        }
        let result = self.with_player_host(id, events, |active, _, host| {
            stop_location_based_effects(active, &worn, host)
        });
        if let Err(detail) = result {
            tracing::warn!("location-changed enchantment effects: {detail}");
        }
    }

    pub(in crate::world) fn player_location_equipment_changed(
        &mut self,
        id: ConnectionId,
        changes: &[(EquipmentSlot, RuntimeItemStack, RuntimeItemStack)],
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.datapack_tables.has_location_effects() {
            return;
        }
        let unbroken = |stack: &RuntimeItemStack| {
            crate::item_durability::damage_and_max(stack.summary())
                .is_none_or(|(damage, max)| damage < max)
        };
        let previous = self.player_worn_matching(
            super::datapack_install::DatapackRuntimeTables::holder_has_location_effect,
            changes.iter().map(|(slot, previous, _)| (*slot, previous)),
        );
        let current = self.player_worn_matching(
            super::datapack_install::DatapackRuntimeTables::holder_has_location_effect,
            changes
                .iter()
                .filter(|(_, _, current)| unbroken(current))
                .map(|(slot, _, current)| (*slot, current)),
        );
        if previous.is_empty() && current.is_empty() {
            return;
        }
        let result = self.with_player_host(id, events, |active, table, host| {
            let stopped = stop_location_based_effects(active, &previous, host);
            let ran = run_location_changed_effects(active, &current, table, host);
            stopped.and(ran)
        });
        if let Err(detail) = result {
            tracing::warn!("location-changed enchantment effects: {detail}");
        }
    }

    fn player_worn_matching<'a>(
        &self,
        is_holder_in_table: impl Fn(&super::datapack_install::DatapackRuntimeTables, usize) -> bool,
        stacks: impl Iterator<Item = (EquipmentSlot, &'a RuntimeItemStack)>,
    ) -> Vec<WornEnchantment> {
        let tables = &*self.datapack_tables;
        let mut worn = Vec::new();
        for (slot, stack) in stacks {
            let patch = &stack.summary().component_patch;
            if stack.is_empty()
                || patch.enchantments.is_empty()
                || patch.removed_type_ids.contains(
                    &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                )
            {
                continue;
            }
            for entry in &patch.enchantments {
                if !(0..=255).contains(&entry.level) {
                    continue;
                }
                let Ok(holder_id) = usize::try_from(entry.holder_id) else {
                    continue;
                };
                if !is_holder_in_table(tables, holder_id)
                    || !tables.holder_matches_slot(holder_id, slot)
                {
                    continue;
                }
                let Some(enchantment) = tables.enchantment_registry.get(holder_id) else {
                    continue;
                };
                worn.push(WornEnchantment {
                    slot,
                    enchantment: enchantment.clone(),
                    level: entry.level,
                });
            }
        }
        worn
    }

    fn with_player_host(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
        body: impl FnOnce(
            &mut ActiveLocationEffects,
            &crate::enchantment_location_effects::EnchantmentLocationTable,
            &mut LocationEffectTarget<'_, '_, '_>,
        ) -> Result<(), String>,
    ) -> Result<(), String> {
        let Some(player) = self.levels.player_mut(id) else {
            return Ok(());
        };
        let mut active = std::mem::take(&mut player.cold.location_effects.active);
        let dimension = player.dimension;
        let table = self
            .level_by_id(&dimension)
            .entities
            .enchantment_location_effects_shared();
        let result = body(
            &mut active,
            &table,
            &mut LocationEffectTarget::Player(PlayerHost {
                world: self,
                id,
                dimension,
                events,
            }),
        );
        if let Some(player) = self.levels.player_mut(id) {
            player.cold.location_effects.active = active;
        }
        result
    }

    /// `THIS_ENTITY` of `locationContext` for a session.
    fn player_location_view(
        &self,
        id: ConnectionId,
    ) -> Option<crate::loot_runtime::LootEntityFacts> {
        let mut view = self.session_loot_view(id)?;
        let player = &self.levels.player(id).expect("the player is online");
        let movement_block = self
            .player_support_pos_at(
                id,
                player.position(),
                player.block_contact.supporting,
                MOVEMENT_BLOCK_OFFSET,
            )
            .map(|pos| [pos.x, pos.y, pos.z]);
        view.movement_block = movement_block;
        if player.avatar_removed {
            view.vehicle = Some(None);
        }
        Some(view)
    }

    /// One `locationContext` requirement with the session as `this`, or an
    /// `entityContext` one when `active` is `None`.
    pub(in crate::world) fn session_requirements_match(
        &mut self,
        id: ConnectionId,
        enchantment: &str,
        enchantment_level: i32,
        active: Option<bool>,
        condition: &LootCondition,
    ) -> Result<bool, String> {
        let view = self.player_location_view(id).ok_or("the session left")?;
        let player = self.levels.player(id).ok_or("the session left")?;
        let dimension = player.dimension;
        let player_id = player.entity_id();
        let param_set = match active {
            Some(_) => param_sets::ENCHANTED_LOCATION,
            None => param_sets::ENCHANTED_ENTITY,
        };
        self.with_entity_predicate_level(
            dimension,
            param_set,
            move |bodies, tables, random, base| {
                bodies.store().player(player_id).ok_or("the session left")?;
                let context = LocationContextLevel {
                    vehicle_slots: base.vehicle_slots,
                    item_environment: base.item_environment,
                    loot_tables: tables,
                    dimension: base.dimension,
                    blocks: base.level.ok_or("predicate level blocks unavailable")?,
                    block_entity_data: base
                        .block_entity_data
                        .ok_or("predicate block data unavailable")?,
                    weather: base.weather.ok_or("predicate weather unavailable")?,
                };
                location_condition_matches(
                    &context,
                    random,
                    crate::entity::location_effects::LocationRequirementRequest {
                        this: LootEntity::Facts(&view),
                        enchantment,
                        enchantment_level,
                        active,
                        condition,
                    },
                )
            },
        )
    }
}

pub(crate) struct PlayerHost<'a> {
    world: &'a mut ServerWorld,
    id: ConnectionId,
    dimension: LevelId,
    events: &'a mut Vec<WorldEvent>,
}

impl PlayerHost<'_> {
    pub(crate) fn attributes(&mut self) -> Option<&mut AttributeMap> {
        self.world
            .levels
            .player_mut(self.id)
            .map(|player| &mut player.attributes)
    }

    pub(crate) fn requirements_match(
        &mut self,
        enchantment: &str,
        enchantment_level: i32,
        active: Option<bool>,
        condition: &LootCondition,
    ) -> Result<bool, String> {
        self.world.session_requirements_match(
            self.id,
            enchantment,
            enchantment_level,
            active,
            condition,
        )
    }

    pub(crate) fn apply_entity_effect(
        &mut self,
        worn: &WornEnchantment,
        effect: &EnchantmentEntityEffect,
    ) -> Result<(), String> {
        let position = self
            .world
            .levels
            .player(self.id)
            .ok_or("the session left")?
            .position();
        self.world.apply_entity_effect(
            &self.dimension,
            effect,
            worn.level,
            &mut EnchantedItem::equipment(SourceEntity::Session(self.id), worn.slot, None),
            SourceEntity::Session(self.id),
            position,
            None,
            self.events,
        )
    }
}
