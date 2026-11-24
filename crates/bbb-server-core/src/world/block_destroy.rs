//! Breaking a block: the destroy-progress packet arms, the mining that
//! follows them, and everything the break leaves behind — drops, experience,
//! tool damage and the acknowledgement the client waits for.

use crate::world::LevelId;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_destroy_speed,
};
use bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::VANILLA_ITEM_DEBUG_STICK_ID;
use bbb_protocol::packets::{
    BlockDestruction, BlockPos, BlockUpdate, EntityEvent, InteractionHand, ItemStackSummary,
    PlayerAction, PlayerActionKind, Vec3d,
};
use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
    VANILLA_STAT_TYPE_ITEM_USED_ID,
};
use bbb_server_types::block_queries::game_type_may_build;

use crate::block_destroy::{
    block_destroy_progress, has_correct_tool_for_drops, StopOutcome, DESTROY_PROGRESS_RANGE_SQR,
    NO_DESTROY_STAGE,
};
use crate::block_entity::BlockEntity;
use crate::entity::block_drops::{loot_tool_for_stack, unresolved_tool_enchantments};
use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageSource};
use crate::entity::{award_experience, SpawnReason};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::inventory::can_destroy_block;
use crate::item_durability::{
    damage_and_max, mine_block, process_value_effects, repair_to, EnchantmentValueEffects,
    ENTITY_EVENT_MAINHAND_ITEM_BROKEN,
};
use crate::level_event::{
    PendingLevelEvent, VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK,
};
use crate::loot_runtime::{LootEntity, LootTool};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};
use bbb_server_types::food::cause_food_exhaustion;
use bbb_server_types::item_stack::RuntimeItemStack;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::StatKey;
use bbb_server_types::Result;

use super::creaking_heart_break::HeartXpPlayer;
use super::simulation_level::block_entity_loot::{DropBlockEntity, DropTable};
use super::simulation_level::pending_spawns::PendingEntitySpawns;
use super::{send, LevelCtx, PlayerState, ServerLevel, ServerWorld, WorldCloseReason, WorldEvent};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

mod beehive;
mod hit_block;
mod overrides;

impl ServerWorld {
    /// `ServerPlayerGameMode.destroyAndAck` (:253-260): a break that
    /// `destroyBlock` refuses is answered with a `ClientboundBlockUpdatePacket`
    /// that puts the block back on the client, and nothing is written.
    pub(super) fn destroy_and_ack(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.destroy_block(connection_id, pos, events) {
            // `:256-259`.
            self.send_block_update(connection_id, pos, events);
        }
    }

    pub(super) fn send_block_update(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = self.player_level_id(connection_id);
        if let Some(block_state_id) = self.block_state(dimension, pos) {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::BlockUpdate(BlockUpdate {
                    pos,
                    block_state_id,
                }),
            ));
        }
    }

    /// `ServerPlayerGameMode.destroyBlock` (`:262-302`): validate the current
    /// stack, `GameMasterBlock` permission and game mode, run
    /// `playerWillDestroy`, replace the state with its fluid, then apply tool
    /// and drop effects.
    pub(super) fn destroy_block(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.destroy_block_inner(connection_id, pos, events)
    }

    fn destroy_block_inner(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if !self.player_destroys_block(connection_id, pos, events) {
            return false;
        }
        let dimension = self.player_level_id(connection_id);
        let air_state_id = self.cold.fixture.air_state_id();
        // `:263`. A position outside the world answers `Blocks.VOID_AIR`
        // there and air here, which is the same block for everything below.
        let state = self.block_state(dimension, pos).unwrap_or(air_state_id);
        // `:270-273`.
        if crate::block_behavior::player_break::is_game_master_block(state)
            && !self.can_use_game_master_blocks(connection_id)
        {
            self.with_level(dimension, |level| {
                LevelCtx::send_block_updated_with_flags(
                    level,
                    pos,
                    state,
                    state,
                    SET_BLOCK_UPDATE_ALL,
                );
            });
            return false;
        }
        if self.player_block_action_restricted(connection_id, pos) {
            return false;
        }
        // `:268`: the object `playerDestroy` receives, which is the same one
        // `playerWillDestroy` worked on and `removeBlock` then emptied.
        let block_entity = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .cloned();
        // `:279`.
        let adjusted_state = self.player_will_destroy(connection_id, dimension, pos, state, events);
        // The held object as `playerWillDestroy` left it (a survival shulker
        // box has just unpacked its loot table).
        let mut block_entity = block_entity.map(|held| {
            held.resolve_instance(self.level_by_id(&dimension).chunks.block_entities())
        });
        let current = self.block_state(dimension, pos).unwrap_or(air_state_id);
        let replacement = block_state_fluid_state(current).create_legacy_block();
        let changed = self.set_block(dimension, pos, replacement, SET_BLOCK_UPDATE_ALL);
        // `:285-287`. `Block.destroy`'s only override is unreachable here; see
        // this method's own documentation.
        //
        if self.player_has_infinite_materials(connection_id) {
            return true;
        }
        // `:293-295`. `destroyedWith` is a copy taken *before* `mineBlock` runs
        // (`:294`); the original Native value must survive the live stack's
        // damage below.
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let destroyed_with = player.inventory.selected_stack().clone();
        let destroyed_with_summary = destroyed_with.summary();
        let can_destroy = match has_correct_tool_for_drops(
            destroyed_with_summary,
            adjusted_state,
            self.registries.registry_tags(),
        ) {
            Ok(can_destroy) => can_destroy,
            Err(error) => {
                // A generated fact this server is missing is not a reason for
                // the ground to stay empty quietly: the break still happens,
                // and the drop it should have produced is recorded.
                tracing::warn!("block state {adjusted_state} could not be asked whether the held stack is the correct tool for its drops: {error}");
                return true;
            }
        };
        // `:296`. The live main-hand stack, which is why the copy above was
        // taken first.
        self.mine_block_with_selected_item(connection_id, dimension, adjusted_state, events);
        // `:297-299`.
        if changed && can_destroy {
            self.player_destroy_override(
                connection_id,
                dimension,
                pos,
                adjusted_state,
                &destroyed_with,
                block_entity.as_ref(),
            );
            self.finish_beehive_player_destroy(
                connection_id,
                dimension,
                pos,
                adjusted_state,
                &destroyed_with,
                block_entity.as_mut(),
                events,
            );
        }
        true
    }

    pub(super) fn mine_block_with_selected_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let entity_id = player.entity_id();
        let held = player.inventory.selected_stack().clone();
        let mut stack = held.summary().clone();
        let Some(used_item_id) = stack.item_id else {
            return;
        };
        let Some(destroy_speed) = vanilla_block_state_destroy_speed(block_state_id) else {
            tracing::warn!("block state {block_state_id} could not be asked for its destroy speed, so the held tool's durability change is unknown");
            return;
        };
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let enchantments = Self::resolve_value_effects(&stack, &table);
        let has_infinite_materials = self.player_has_infinite_materials(connection_id);
        let in_fire_tag = match (
            vanilla_block_state_block_type_id(block_state_id),
            self.level_by_id(&dimension).chunks.registry_tags_shared(),
        ) {
            (Some(block_id), Some(tags)) => tags.contains_block("minecraft:fire", block_id),
            _ => crate::block_entity::vibration::builtin_block_state_in_tag(
                block_state_id,
                "minecraft:fire",
            ),
        };
        let outcome = {
            let random = &self.random;
            mine_block(
                &mut stack,
                destroy_speed,
                in_fire_tag,
                has_infinite_materials,
                &enchantments,
                random,
                &self.value_effect_level(&dimension),
            )
        };
        drop(enchantments);
        drop(table);
        if let Some(raw_new_damage) = outcome.raw_new_damage {
            self.trigger_item_durability_changed(connection_id, &held, raw_new_damage, events);
        }
        let edited = RuntimeItemStack::from_summary(stack);
        if let Some(player) = self.levels.player_mut(connection_id) {
            *player.inventory.selected_item_mut() = edited;
            if outcome.used {
                player.award_stat_once(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, used_item_id),
                );
            }
        }
        if let Some(broken_item_id) = outcome.broken_item_id {
            self.mainhand_item_broken(connection_id, entity_id, broken_item_id, events);
        }
    }

    pub(super) fn mainhand_item_broken(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        broken_item_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: ENTITY_EVENT_MAINHAND_ITEM_BROKEN,
            }),
            events,
        );
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_BROKEN_ID, broken_item_id),
            );
        }
    }

    pub(super) fn player_will_destroy(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> i32 {
        let dimension = dimension.into();
        if vanilla_block_state_block_type_id(block_state_id)
            == Some(VANILLA_BLOCK_CREAKING_HEART_ID)
        {
            let player = self
                .levels
                .player(connection_id)
                .expect("connection presence was checked");
            let player_uuid = player.profile.uuid;
            let player_entity_id = player.entity_id();
            let instabuild = player.abilities.instabuild;
            let spectator = player.game_type == GameType::Spectator;
            let attacker_views = self.session_attacker_views(Some(player_uuid), Some(player_uuid));
            let damage_type = self
                .cold
                .damage_types
                .iter()
                .find(|entry| entry.resource_id == "minecraft:player_attack")
                .cloned();
            self.creaking_heart_before_removal(
                dimension,
                pos,
                block_state_id,
                damage_type
                    .as_ref()
                    .map(|damage_type| StoredEntityDamageSource {
                        damage_type,
                        direct_entity_id: Some(player_entity_id),
                        causing_entity: Some(StoredDamageCausingEntity::Player {
                            uuid: player_uuid,
                            entity_id: player_entity_id,
                            instabuild,
                        }),
                        attacker_views: attacker_views.views(),
                    }),
                Some(HeartXpPlayer {
                    instabuild,
                    spectator,
                }),
            );
            // The base 2001 packet is sent directly below, so publish the
            // death journal first to preserve the subclass-before-super order.
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
        }
        let block_state_id = self.player_will_destroy_override(
            connection_id,
            dimension,
            pos,
            block_state_id,
            events,
        );
        // What the subclass arm queued leaves before the base's own packets.
        self.flush_level_events(&dimension, events);
        if !bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
            .is_some_and(|id| {
                matches!(
                    id,
                    bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID
                        | bbb_protocol::block_types::VANILLA_BLOCK_SOUL_FIRE_ID
                )
            })
        {
            self.broadcast_level_event_by_key(
                &dimension,
                Some(connection_id),
                PendingLevelEvent {
                    event_type: LEVEL_EVENT_PARTICLES_DESTROY_BLOCK,
                    pos,
                    data: block_state_id,
                },
                events,
            );
        }
        // `:520-522`.
        if self.player_break_angers_piglins(dimension, block_state_id) {
            self.anger_nearby_piglins(connection_id, false);
        }
        // `:524`.
        let player_entity_id = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id());
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_DESTROY,
                pos,
                player_entity_id,
                Some(block_state_id),
            );
        // `:525`.
        block_state_id
    }

    pub(super) fn player_destroy(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        (mined_block_type_id, block_state_id): (Option<i32>, i32),
        destroyed_with: &bbb_server_types::item_stack::RuntimeItemStack,
        block_entity: Option<&BlockEntity>,
    ) {
        let dimension = dimension.into();
        if let Some(player) = self.levels.player_mut(connection_id) {
            // `:478`. `Stats.BLOCK_MINED.get(this)` keys on the *block*, not
            // the state, which is `Block.getId`'s own registry id.
            if let Some(block_type_id) = mined_block_type_id {
                player.award_stat_once(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, block_type_id),
                );
            }
            cause_food_exhaustion(
                &mut player.food,
                player.abilities.invulnerable,
                bbb_server_types::food::EXHAUSTION_MINE,
            );
        }
        // `:480`.
        self.drop_resources_for_player(
            connection_id,
            dimension,
            pos,
            block_state_id,
            destroyed_with,
            block_entity,
        );
    }

    pub(super) fn spawn_after_break(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        tool: &ItemStackSummary,
        drop_experience: bool,
    ) {
        let dimension = dimension.into();
        let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
            return;
        };
        if crate::block_behavior::infested::host_state_by_infested(block_state_id).is_some()
            && self.game_rules.block_drops
        {
            self.spawn_block_silverfish(dimension, pos, tool);
        }
        if !drop_experience {
            return;
        }
        let Some(provider) = bbb_server_types::experience::block_break_experience(block_type_id)
        else {
            return;
        };
        let sampled = {
            let random = &self.random;
            provider.sample(|bound| random.next_int(bound))
        };
        let mut experience = sampled;
        if provider.is_enchantment_modified() {
            let effects = std::sync::Arc::clone(&self.datapack_tables.enchantment_block_experience);
            experience = crate::item_durability::process_value_effects(
                tool,
                sampled as f32,
                &Self::resolve_value_effects(tool, &effects),
                &self.random,
                &self.value_effect_level(&dimension),
            ) as i32;
        }
        // `:622`.
        if experience <= 0 {
            return;
        }
        self.pop_experience(dimension, pos, experience);
    }

    fn spawn_block_silverfish(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        tool: &ItemStackSummary,
    ) {
        let dimension = dimension.into();
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let unresolved =
            unresolved_tool_enchantments(tool, &self.datapack_tables.enchantment_registry);
        if !unresolved.is_empty() {
            tracing::warn!("infested block tool enchantment holder cannot be resolved");
            return;
        }
        let tool = loot_tool_for_stack(tool, &self.datapack_tables.enchantment_registry);
        for enchantment in tool.enchantments.keys() {
            if self
                .level_by_id(&dimension)
                .chunks
                .registry_tags()
                .is_some_and(|tags| {
                    tags.contains(
                        "minecraft:enchantment",
                        "minecraft:prevents_infested_spawns",
                        enchantment,
                    )
                })
            {
                return;
            }
        }
        if let Some(entity_id) = self.spawn_finalized_entity(
            dimension,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID,
            position,
            SpawnReason::Triggered,
            Some(0.0),
        ) {
            let entities = &mut self.level_mut_by_id(&dimension).entities;
            entities.broadcast_to_tracking(
                entity_id,
                crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: 20,
                }),
            );
        }
    }

    pub(super) fn pop_experience(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        amount: i32,
    ) {
        let dimension = dimension.into();
        if !self.game_rules.block_drops {
            return;
        }
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        let level = self.level_mut_by_id(&dimension);
        let ServerLevel {
            entities,
            random: world_random,
            ..
        } = level;
        let _ = award_experience(entities, world_random, position, amount);
    }

    ///
    /// The recursion at `:304` is a loop here; both terminate the same way,
    /// because each pass either repairs nothing (`repair == 0`, answering zero)
    /// or spends a strictly positive part of the amount.
    ///
    ///
    /// The draw is the player's own `RandomSource`, which this crate stands in
    /// for with the entity store's stream; see [`EntityStore::next_int`].
    pub(super) fn repair_player_items(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        amount: i32,
    ) -> i32 {
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_repair_with_xp);
        let Some(equipment) = self
            .levels
            .player(connection_id)
            .map(|player| player.inventory.equipment_slots().collect::<Vec<_>>())
        else {
            return amount;
        };
        let mut amount = amount;
        while let Some(player) = self.levels.player(connection_id) {
            // `:485-499`.
            let mut candidates: Vec<usize> = Vec::new();
            for (slot, index) in &equipment {
                let Some(stack) = player.inventory.slot(*index) else {
                    continue;
                };
                let stack = stack.summary();
                if !damage_and_max(stack).is_some_and(|(damage, _)| damage > 0) {
                    continue;
                }
                for (_, effects) in Self::resolve_value_effects(stack, &table) {
                    if effects.matching_slot(*slot) {
                        candidates.push(*index);
                    }
                }
            }
            if candidates.is_empty() {
                break;
            }
            let picked = {
                let entities = &mut self.level_mut_by_id(dimension).entities;
                candidates[usize::try_from(entities.random().next_int(
                    i32::try_from(candidates.len()).expect("an equipment walk is eight slots long"),
                ))
                .expect("nextInt answers a non-negative index")]
            };
            let Some(player) = self.levels.player(connection_id) else {
                break;
            };
            let held = player.inventory.inventory_slots()[picked].clone();
            let mut stack = held.summary().clone();
            let enchantments = Self::resolve_value_effects(&stack, &table);
            let to_repair = {
                let random = &self.random;
                process_value_effects(
                    &stack,
                    amount as f32,
                    &enchantments,
                    random,
                    &self.value_effect_level(&player.dimension),
                ) as i32
            }
            .max(0);
            let Some((damage, _)) = damage_and_max(&stack) else {
                break;
            };
            // `:298-299`.
            let repair = to_repair.min(damage);
            repair_to(&mut stack, damage - repair);
            let edited = RuntimeItemStack::from_summary(stack);
            if let Some(player) = self.levels.player_mut(connection_id) {
                if let Some(slot) = player.inventory.slot_mut(picked) {
                    *slot = edited;
                }
            }
            // `:300-307`. A zero repair -- which is what a fully repaired or a
            // zero-factor stack gives -- still consumes the whole amount.
            if repair <= 0 {
                amount = 0;
                break;
            }
            let remaining = amount - repair * amount / to_repair;
            if remaining <= 0 {
                amount = 0;
                break;
            }
            amount = remaining;
        }
        amount
    }

    /// The stack's `minecraft:enchantments` resolved against one effect table,
    /// in the order [`crate::entity::block_drops::loot_tool_for_stack`] documents.
    pub(super) fn resolve_value_effects<'a>(
        stack: &ItemStackSummary,
        table: &'a [Option<EnchantmentValueEffects>],
    ) -> Vec<(i32, &'a EnchantmentValueEffects)> {
        if stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
        {
            return Vec::new();
        }
        stack
            .component_patch
            .enchantments
            .iter()
            .filter_map(|entry| {
                let index = usize::try_from(entry.holder_id).ok()?;
                let effects = table.get(index)?.as_ref()?;
                Some((entry.level, effects))
            })
            .collect()
    }

    ///
    /// Built the same way [`Self::set_block`] is, because the loot walk can
    /// read blocks and the `popResource` half draws from `Level.random`: both
    /// need the level slice every mechanism tick runs through, and the item
    /// spawns it collects have to reach the same entity store.
    pub(super) fn drop_resources_with_tool(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        tool: &LootTool,
        block_entity: Option<&BlockEntity>,
    ) {
        let dimension = dimension.into();
        let breaker = self.loot_view_for_player(self.levels.player_at(connection_id));
        let players = self.nearby_players_by_key(&dimension);
        let mut leftover = PendingEntitySpawns::default();
        self.with_level_ctx(&dimension, &players, &mut leftover, |level, rest| {
            level
                .with_structure_entities(rest.entities, None)
                .drop_block_loot_at(
                    pos,
                    block_state_id,
                    DropTable::Block,
                    DropBlockEntity::Held(block_entity),
                    tool,
                    LootEntity::Facts(&breaker),
                );
        });
        // The frame the resource drop ran on is over.
        self.post_pending_game_events(&dimension);
        self.apply_pending_spawns(&dimension, leftover);
    }

    pub(super) fn player_block_action_restricted(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
    ) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if game_type_may_build(player.game_type) {
            return false;
        }
        // `:193-195`.
        if player.game_type == GameType::Spectator {
            return true;
        }
        // `:197-199` cannot answer true on this side: [`PlayerState::may_build`]
        // is derived from the same game type the first guard just refused, so
        // adventure is the only mode that reaches this line and its `mayBuild`
        // is false. It is written out because vanilla's `mayBuild` is a stored
        // `Abilities` bit rather than a derivation, and the two readings would
        // have to be told apart the moment this server stores one too.
        if player.may_build() {
            return false;
        }
        // `:201-202`. `ItemStack.isEmpty` is a missing item or a count that
        // reached zero, which is the same emptiness every other reader of a
        // stack in this crate tests for.
        let item = match player.inventory.selected_item() {
            Ok(item) => item,
            Err(error) => {
                tracing::warn!("the adventure block-action stack cannot project: {error}");
                return true;
            }
        };
        item.item_id.filter(|_| item.count > 0).is_none()
            || !self.adventure_mode_allows(&player.dimension, pos, &item, true)
    }

    pub(super) fn destroy_block_progress(
        &self,
        dimension: &LevelId,
        entity_id: i32,
        pos: BlockPos,
        stage: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        for (&connection_id, player) in self.levels.player_entries() {
            if self.player_level_id(connection_id) != *dimension || player.entity_id() == entity_id
            {
                continue;
            }
            let xd = f64::from(pos.x) - player.position().x;
            let yd = f64::from(pos.y) - player.position().y;
            let zd = f64::from(pos.z) - player.position().z;
            if xd * xd + yd * yd + zd * zd >= DESTROY_PROGRESS_RANGE_SQR {
                continue;
            }
            events.push(send(
                connection_id,
                ClientboundPlayPacket::BlockDestruction(BlockDestruction {
                    id: entity_id,
                    pos,
                    progress: stage as u8,
                }),
            ));
        }
    }

    ///
    /// Two of vanilla's refusals have no counterpart and are divergences:
    ///
    pub(super) fn start_destroy_block(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        // `:172-175`.
        if self.player_has_infinite_materials(connection_id) {
            self.destroy_and_ack(connection_id, pos, events);
            return Ok(());
        }
        // `:177-181`.
        if self.player_block_action_restricted(connection_id, pos) {
            self.send_block_update(connection_id, pos, events);
            return Ok(());
        }
        // `:183`, which vanilla writes before it knows whether the block will be
        // insta-mined.
        self.levels[connection_id]
            .cold
            .destroy
            .begin_progress_window();
        // `:184-199`. The initial `1.0F` is what an air block keeps, because the
        // whole block is skipped for one (`:186`).
        let dimension = self.player_level_id(connection_id);
        let standing = self
            .block_state(dimension, pos)
            .filter(|id| !block_state_is_air(*id));
        let progress = match standing {
            Some(block_state_id) => {
                // `:187-196`.
                self.on_hit_block(connection_id, dimension, pos, block_state_id, events);
                // `:197`; `:198` still reads the state from before it.
                self.attack_block(connection_id, dimension, pos, block_state_id);
                let player = self
                    .levels
                    .player(connection_id)
                    .expect("connection presence was checked");
                block_destroy_progress(
                    &player.inventory.selected_item()?,
                    block_state_id,
                    player.destroy_speed_facts(),
                    self.registries.registry_tags(),
                )?
            }
            None => 1.0,
        };
        // `:201-202`.
        if standing.is_some() && progress >= 1.0 {
            self.destroy_and_ack(connection_id, pos, events);
            return Ok(());
        }
        // `:204-214`.
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let entity_id = player.entity_id();
        let (interrupted, stage) = player.cold.destroy.start(pos, progress);
        if let Some(interrupted) = interrupted {
            self.send_block_update(connection_id, interrupted, events);
        }
        self.destroy_block_progress(
            &self.player_level_id(connection_id),
            entity_id,
            pos,
            stage,
            events,
        );
        Ok(())
    }

    ///
    /// `pos.equals(this.destroyPos)` (`:217`) is asked without looking at
    /// `isDestroyingBlock`, so a session that never started a break still
    /// matches the `BlockPos.ZERO` its `destroyPos` was initialised to
    /// (`:43`) — and then measures `ticksSpentDestroying` against a
    /// `destroyProgressStart` of zero, which is a long time. That is vanilla's
    /// own shape and it is reproduced rather than tidied: the only extra guard
    /// on this side is the interaction range every action already passes.
    pub(super) fn stop_destroy_block(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        if self.levels[connection_id].cold.destroy.destroy_pos() != pos {
            return Ok(());
        }
        // `:219-220`.
        let Some(block_state_id) = self
            .block_state(self.player_level_id(connection_id), pos)
            .filter(|id| !block_state_is_air(*id))
        else {
            return Ok(());
        };
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let entity_id = player.entity_id();
        let facts = player.destroy_speed_facts();
        let PlayerState {
            cold, inventory, ..
        } = player;
        let outcome = cold.destroy.stop(
            block_state_id,
            &inventory.selected_item()?,
            facts,
            self.registries.registry_tags(),
        )?;
        if outcome == StopOutcome::Destroy {
            // `:224` then `:225`.
            self.destroy_block_progress(
                &self.player_level_id(connection_id),
                entity_id,
                pos,
                NO_DESTROY_STAGE,
                events,
            );
            self.destroy_and_ack(connection_id, pos, events);
        }
        Ok(())
    }

    pub(super) fn abort_destroy_block(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let entity_id = player.entity_id();
        let outcome = player.cold.destroy.abort(pos);
        if let Some(mismatched) = outcome.mismatched {
            // `:243`, the arm that also logs the mismatch.
            self.destroy_block_progress(
                &self.player_level_id(connection_id),
                entity_id,
                mismatched,
                NO_DESTROY_STAGE,
                events,
            );
        }
        // `:247`.
        self.destroy_block_progress(
            &self.player_level_id(connection_id),
            entity_id,
            outcome.pos,
            NO_DESTROY_STAGE,
            events,
        );
    }

    /// `ServerGamePacketListenerImpl.handlePlayerAction`'s three block-break
    /// actions (:1304-1307): they all forward to
    /// `ServerPlayerGameMode.handleBlockBreakAction` and are acknowledged
    /// afterwards whatever it did (:1306).
    ///
    pub(super) fn apply_player_action(
        &mut self,
        connection_id: ConnectionId,
        action: PlayerAction,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if !matches!(
            action.action,
            PlayerActionKind::StartDestroyBlock
                | PlayerActionKind::StopDestroyBlock
                | PlayerActionKind::AbortDestroyBlock
        ) {
            self.apply_player_hand_action(connection_id, action.action, events);
            return None;
        }
        if action.sequence < 0 {
            return Some(WorldCloseReason::InvalidBlockInteraction);
        }
        if !self.levels[connection_id].client_loaded() {
            return None;
        }
        // `handleBlockBreakAction` (:150-157): out of reach is dropped, above
        // the build height is answered with the current state; either way the
        // sequence is acknowledged.
        let (_, max_y) = self.player_build_height(connection_id);
        if !self.player_reaches_block(connection_id, action.pos) || action.pos.y > max_y {
            if action.pos.y > max_y {
                self.send_block_update(connection_id, action.pos, events);
            }
            self.acknowledge_block_change(connection_id, action.sequence);
            return None;
        }
        if !self.is_in_world_bounds_by_key(&self.player_level_id(connection_id), action.pos) {
            self.acknowledge_block_change(connection_id, action.sequence);
            return None;
        }
        // A missing generated fact is this server's own failure rather than the
        // client's, so the action is declined and still acknowledged, the same
        // way `apply_use_item_on` declines an item it cannot classify.
        let declined = match action.action {
            PlayerActionKind::StartDestroyBlock => {
                if self.refuse_block_interaction(connection_id, action.pos, events) {
                    self.send_block_update(connection_id, action.pos, events);
                    Ok(())
                } else {
                    self.start_destroy_block(connection_id, action.pos, events)
                }
            }
            PlayerActionKind::StopDestroyBlock => {
                self.stop_destroy_block(connection_id, action.pos, events)
            }
            PlayerActionKind::AbortDestroyBlock => {
                self.abort_destroy_block(connection_id, action.pos, events);
                Ok(())
            }
            _ => unreachable!("hand actions returned above"),
        };
        if let Err(error) = declined {
            tracing::warn!("the block-break action was declined: {error}");
        }
        self.acknowledge_block_change(connection_id, action.sequence);
        None
    }

    pub(super) fn player_has_infinite_materials(&self, connection_id: ConnectionId) -> bool {
        self.levels[connection_id].abilities.instabuild
    }

    ///
    /// A missing generated fact is this server's fault rather than the
    /// client's, so it lets the break through — the same call the packet would
    /// have made had the catalog answered nothing about the stack.
    pub(super) fn player_destroys_block(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let item = match player.inventory.selected_item() {
            Ok(item) => item,
            Err(error) => {
                tracing::warn!("the block-destroy stack cannot project: {error}");
                return false;
            }
        };
        if item.count > 0 && item.item_id == Some(VANILLA_ITEM_DEBUG_STICK_ID) {
            self.debug_stick_interaction(
                connection_id,
                InteractionHand::MainHand,
                pos,
                false,
                events,
            );
            return false;
        }
        can_destroy_block(&item, player.abilities.instabuild).unwrap_or(true)
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn destroy_block_progress_uses_explicit_dimension_for_mob_sources_and_strict_range() {
        let (sender, receiver) =
            crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
                .unwrap();
        let mut world = ServerWorld::new(Default::default()).unwrap();
        for (id, name) in [(1, "near"), (2, "other_level"), (3, "edge"), (4, "inside")] {
            crate::tests::join_loaded(&sender, &receiver, &mut world, id, name, u128::from(id));
        }
        let overworld: LevelId = Java26BuiltinDimension::Overworld.into();
        let nether: LevelId = Java26BuiltinDimension::Nether.into();
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        for player in world.levels.players_mut() {
            player.set_position(Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            });
        }
        world.levels.move_player(ConnectionId(2), nether);
        {
            let mut position = world.levels[ConnectionId(3)].position();
            position.x = 32.0;
            world.levels[ConnectionId(3)].set_position(position);
        }
        {
            let mut position = world.levels[ConnectionId(4)].position();
            position.x = 31.999;
            world.levels[ConnectionId(4)].set_position(position);
        }
        let mob_id = i32::MAX;
        assert!(world
            .levels
            .players()
            .all(|player| player.entity_id() != mob_id));
        let mut events = Vec::new();
        world.destroy_block_progress(&overworld, mob_id, pos, -1, &mut events);
        let updates = |events: &[WorldEvent]| -> Vec<(ConnectionId, BlockDestruction)> {
            let mut updates = events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::Send {
                        connection_id,
                        packet: ClientboundPlayPacket::BlockDestruction(update),
                    } => Some((*connection_id, *update)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            updates.sort_by_key(|(connection_id, _)| *connection_id);
            updates
        };
        assert_eq!(
            updates(&events),
            vec![
                (
                    ConnectionId(1),
                    BlockDestruction {
                        id: mob_id,
                        pos,
                        progress: 255
                    }
                ),
                (
                    ConnectionId(4),
                    BlockDestruction {
                        id: mob_id,
                        pos,
                        progress: 255
                    }
                ),
            ]
        );
        events.clear();
        world.destroy_block_progress(&nether, mob_id, pos, 4, &mut events);
        assert_eq!(
            updates(&events),
            vec![(
                ConnectionId(2),
                BlockDestruction {
                    id: mob_id,
                    pos,
                    progress: 4
                }
            ),]
        );
        events.clear();
        let player_id = world.levels[ConnectionId(1)].entity_id();
        world.destroy_block_progress(&overworld, player_id, pos, 3, &mut events);
        assert_eq!(
            updates(&events),
            vec![(
                ConnectionId(4),
                BlockDestruction {
                    id: player_id,
                    pos,
                    progress: 3
                }
            ),]
        );
    }
}
