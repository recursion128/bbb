use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::LevelId;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use std::collections::BTreeMap;

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_stack;
#[cfg(test)]
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::living::mob_effects::{mob_effects_predicate_matches, ActiveMobEffects};
use crate::inventory::{MenuBroadcastEvent, MenuBroadcastMode};
use crate::loot_runtime::{
    check_conditions, check_location_predicate, location_predicate_matches_in_world,
    matches_conditions, matches_item_predicate, param_sets, LootBlockStateAt, LootContextParamSet,
    LootDamageSource, LootDamageSourceFacts, LootEntities, LootEntity, LootEntityFacts,
    LootEntityFlags, LootEvaluator, LootOrigin, LootParams, LootTool, LootWeather,
    StoredLootEntity,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_protocol::JavaString;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::advancement::{
    parse_inventory_changed_condition, parse_recipe_unlocked_condition, InventoryChangedCondition,
};
use bbb_server_datapack::loot::{
    parse_item_predicate_source as parse_item_predicate,
    parse_location_predicate_source as parse_location_predicate,
    parse_loot_condition_source as parse_loot_condition,
    parse_mob_effects_predicate_source as parse_mob_effects_predicate, DamageSourcePredicate,
    DistancePredicate, DoubleBounds, EntityPredicate, IntBounds, ItemPredicate, LocationPredicate,
    LootCondition, MobEffectInstancePredicate,
};
use bbb_server_datapack::player_advancements::AdvancementTree;
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
#[cfg(test)]
use serde_json::Value;

mod bee_nest_destroyed;
mod block_contact;
mod block_use;
mod bred_animals;
mod brewed_potion;
mod construct_beacon;
mod cured_zombie_villager;
mod enchanted_item;
mod fishing_rod_hooked;
mod generate_loot;
mod hurt;
mod item_durability;
mod item_use;
mod killed_by_arrow;
mod lightning;
mod pattern_summon;
mod picked_up_item;
mod player;
mod recipe_crafted;
mod spear_mobs;
mod target_hit;
mod used_ender_eye;

pub(super) use generate_loot::namespaced as namespaced_loot_table;
pub(in crate::world) use hurt::HurtTriggerDetails;
pub(super) use item_use::{CONSUME_ITEM, FILLED_BUCKET, SHOT_CROSSBOW, USED_TOTEM, USING_ITEM};

use super::level_blocks::LoadedBlocks;
use super::{PlayerState, ServerWorld, WorldEvent};

const VILLAGER_TRADE: &str = "minecraft:villager_trade";

type Compiled<T> = BTreeMap<usize, BTreeMap<JavaString, T>>;

fn insert_compiled<T>(
    table: &mut Compiled<T>,
    index: usize,
    name: &JavaString,
    condition: Result<T, String>,
) {
    let condition = condition.unwrap_or_else(|error| {
        panic!("validated advancement criterion {index}/{name} failed to compile: {error}")
    });
    table
        .entry(index)
        .or_default()
        .insert(name.clone(), condition);
}

// ServerAdvancementManager.java:53-59: `advancement.validate` logs and retains.
pub(super) fn warn_condition_check(
    owner: &str,
    conditions: &[LootCondition],
    param_set: &'static LootContextParamSet,
) {
    if let Err(error) = check_conditions(owner, conditions, param_set) {
        tracing::warn!("advancement validation problem in {owner}: {error}");
    }
}

struct InventoryAdvancementCondition {
    inventory: InventoryChangedCondition,
    player: Vec<LootCondition>,
}

#[derive(Default)]
pub(super) struct AdvancementTriggers {
    recipes: BTreeMap<String, Vec<(usize, JavaString, Vec<LootCondition>)>>,
    inventory: Compiled<InventoryAdvancementCondition>,
    dimension_changes: Compiled<DimensionChangeCondition>,
    distance: BTreeMap<String, Compiled<TravelDistanceCondition>>,
    /// Keyed by trigger id: the `ItemUsedOnLocationTrigger` consumers the
    /// built-in pack names share one condition shape and one parameter set.
    item_used_on_location: BTreeMap<String, Compiled<ItemUsedOnLocationCondition>>,
    player: BTreeMap<String, Compiled<Vec<LootCondition>>>,
    effects_changed: Compiled<EffectsChangedCondition>,
    levitation: Compiled<LevitationCondition>,
    /// Keyed by trigger id: every `KilledTrigger` consumer reads the same two
    /// terms off the same side of one kill.
    killed: BTreeMap<String, Compiled<EntityContextCondition>>,
    /// `TameAnimalTrigger` has one consumer and one term, so it needs no key of
    /// its own beside the criterion index.
    tamed: Compiled<EntityContextCondition>,
    summoned: Compiled<EntityContextCondition>,
    bred: Compiled<bred_animals::BredCondition>,
    brewed_potion: Compiled<brewed_potion::BrewedPotionCondition>,
    cured_zombie_villager: Compiled<cured_zombie_villager::CuredCondition>,
    lightning_strike: Compiled<lightning::StrikeCondition>,
    channeled_lightning: Compiled<lightning::VictimsCondition>,
    killed_by_arrow: Compiled<killed_by_arrow::KilledCondition>,
    target_hit: Compiled<target_hit::TargetHitCondition>,
    entity_hurt_player: Compiled<hurt::HurtCondition>,
    player_hurt_entity: Compiled<hurt::HurtCondition>,
    traded: Compiled<EntityContextCondition>,
    interacted: BTreeMap<String, Compiled<EntityContextCondition>>,
    /// Keyed by trigger id: `consume_item` and `using_item` share one shape.
    item_criteria: BTreeMap<String, Compiled<item_use::ItemCondition>>,
    generate_loot: Compiled<generate_loot::LootTableCondition>,
    block_contact: BTreeMap<String, Compiled<block_contact::ContactCondition>>,
    construct_beacon: Compiled<construct_beacon::BeaconCondition>,
    block_use: BTreeMap<String, Compiled<block_use::BlockUseCondition>>,
    recipe_crafted: BTreeMap<String, Compiled<recipe_crafted::RecipeCraftedCondition>>,
    picked_up_item: Compiled<picked_up_item::PickedUpItemCondition>,
    enchanted_item: Compiled<enchanted_item::EnchantedItemCondition>,
    item_durability: Compiled<item_durability::ItemDurabilityCondition>,
    bee_nest_destroyed: Compiled<bee_nest_destroyed::BeeNestDestroyedCondition>,
    used_ender_eye: Compiled<used_ender_eye::EyeCondition>,
    spear_mobs: Compiled<spear_mobs::SpearMobsCondition>,
    fishing_rod_hooked: Compiled<fishing_rod_hooked::HookedCondition>,
    item_tags: Option<ItemTagIndex>,
}

impl AdvancementTriggers {
    pub(super) fn compile(tree: &AdvancementTree, startup: Option<&DatapackStartup>) -> Self {
        let Some(startup) = startup else {
            return Self::default();
        };
        let registry_lookup = FrozenDatapackRegistryLookup::from_startup(startup);
        let lookup = Some(&registry_lookup);
        let mut compiled = Self {
            item_tags: startup.tags.registry("minecraft:item").map(|tags| {
                ItemTagIndex::from_loaded_with_content_registry(tags, startup.content_registry())
            }),
            ..Self::default()
        };
        for element in &startup.advancements.advancements {
            let Some(index) = tree.index_of(&element.id) else {
                continue;
            };
            for criterion in &element.advancement.criteria {
                let conditions = V::Json(&criterion.conditions);
                match criterion.trigger.as_str() {
                    hurt::ENTITY_HURT_PLAYER => {
                        insert_compiled(
                            &mut compiled.entity_hurt_player,
                            index,
                            &criterion.name,
                            hurt::HurtCondition::parse(conditions, &criterion.trigger, lookup),
                        );
                    }
                    hurt::PLAYER_HURT_ENTITY => {
                        insert_compiled(
                            &mut compiled.player_hurt_entity,
                            index,
                            &criterion.name,
                            hurt::HurtCondition::parse(conditions, &criterion.trigger, lookup),
                        );
                    }
                    bee_nest_destroyed::BEE_NEST_DESTROYED => {
                        insert_compiled(
                            &mut compiled.bee_nest_destroyed,
                            index,
                            &criterion.name,
                            bee_nest_destroyed::BeeNestDestroyedCondition::parse(
                                conditions, lookup,
                            ),
                        );
                    }
                    recipe_crafted::RECIPE_CRAFTED | recipe_crafted::CRAFTER_RECIPE_CRAFTED => {
                        insert_compiled(
                            compiled
                                .recipe_crafted
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            recipe_crafted::RecipeCraftedCondition::parse(
                                conditions,
                                &criterion.trigger,
                                lookup,
                            ),
                        );
                    }
                    block_use::DEFAULT_BLOCK_USE | block_use::ANY_BLOCK_USE => {
                        insert_compiled(
                            compiled
                                .block_use
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            block_use::BlockUseCondition::parse(
                                conditions,
                                &criterion.trigger,
                                lookup,
                            ),
                        );
                    }
                    brewed_potion::BREWED_POTION => {
                        insert_compiled(
                            &mut compiled.brewed_potion,
                            index,
                            &criterion.name,
                            brewed_potion::BrewedPotionCondition::parse(conditions, lookup),
                        );
                    }
                    construct_beacon::CONSTRUCT_BEACON => {
                        insert_compiled(
                            &mut compiled.construct_beacon,
                            index,
                            &criterion.name,
                            construct_beacon::BeaconCondition::parse(conditions, lookup),
                        );
                    }
                    block_contact::SLIDE_DOWN_BLOCK | block_contact::ENTER_BLOCK => {
                        insert_compiled(
                            compiled
                                .block_contact
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            block_contact::ContactCondition::compile(
                                criterion
                                    .block_contact
                                    .as_ref()
                                    .expect("the advancement loader decoded block contact"),
                                &criterion.trigger,
                                lookup,
                            ),
                        );
                    }
                    "minecraft:recipe_unlocked" => {
                        let condition = parse_recipe_unlocked_condition(&criterion.conditions)
                            .expect("the advancement loader validated this trigger");
                        let player = construct_beacon::player_conditions(
                            condition.player.as_ref().map(V::Json),
                            "minecraft:recipe_unlocked",
                            lookup,
                        )
                        .unwrap_or_else(|error| {
                            panic!(
                                "validated advancement criterion {}/{} failed to compile: {error}",
                                element.id, criterion.name
                            )
                        });
                        compiled.recipes.entry(condition.recipe).or_default().push((
                            index,
                            criterion.name.clone(),
                            player,
                        ));
                    }
                    "minecraft:inventory_changed" => {
                        let condition = parse_inventory_changed_condition(&criterion.conditions)
                            .expect("the advancement loader validated this trigger");
                        let condition = construct_beacon::player_conditions(
                            condition.player.as_ref().map(V::Json),
                            "minecraft:inventory_changed",
                            lookup,
                        )
                        .map(|player| InventoryAdvancementCondition {
                            inventory: condition,
                            player,
                        });
                        insert_compiled(&mut compiled.inventory, index, &criterion.name, condition);
                    }
                    "minecraft:changed_dimension" => {
                        insert_compiled(
                            &mut compiled.dimension_changes,
                            index,
                            &criterion.name,
                            dimension_change_condition(conditions, lookup),
                        );
                    }
                    "minecraft:nether_travel"
                    | "minecraft:fall_from_height"
                    | "minecraft:fall_after_explosion"
                    | "minecraft:ride_entity_in_lava" => {
                        insert_compiled(
                            compiled
                                .distance
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            travel_distance_condition(conditions, &criterion.trigger, lookup),
                        );
                    }
                    "minecraft:placed_block"
                    | "minecraft:item_used_on_block"
                    | "minecraft:allay_drop_item_on_block" => {
                        insert_compiled(
                            compiled
                                .item_used_on_location
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            item_used_on_location_condition(conditions, lookup),
                        );
                    }
                    "minecraft:tick"
                    | "minecraft:location"
                    | "minecraft:hero_of_the_village"
                    | "minecraft:voluntary_exile"
                    | "minecraft:slept_in_bed"
                    | "minecraft:avoid_vibration"
                    | "minecraft:started_riding" => {
                        insert_compiled(
                            compiled
                                .player
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            construct_beacon::player_conditions(
                                conditions.field("player"),
                                &criterion.trigger,
                                lookup,
                            ),
                        );
                    }
                    "minecraft:effects_changed" => {
                        insert_compiled(
                            &mut compiled.effects_changed,
                            index,
                            &criterion.name,
                            effects_changed_condition(conditions, lookup),
                        );
                    }
                    "minecraft:levitation" => {
                        insert_compiled(
                            &mut compiled.levitation,
                            index,
                            &criterion.name,
                            levitation_condition(conditions, lookup),
                        );
                    }
                    "minecraft:player_killed_entity"
                    | "minecraft:entity_killed_player"
                    | "minecraft:kill_mob_near_sculk_catalyst" => {
                        insert_compiled(
                            compiled
                                .killed
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            killed_condition(conditions, lookup),
                        );
                    }
                    bred_animals::BRED_ANIMALS => {
                        insert_compiled(
                            &mut compiled.bred,
                            index,
                            &criterion.name,
                            bred_animals::BredCondition::parse(conditions, lookup),
                        );
                    }
                    cured_zombie_villager::CURED_ZOMBIE_VILLAGER => {
                        insert_compiled(
                            &mut compiled.cured_zombie_villager,
                            index,
                            &criterion.name,
                            cured_zombie_villager::CuredCondition::parse(conditions, lookup),
                        );
                    }
                    lightning::LIGHTNING_STRIKE => {
                        insert_compiled(
                            &mut compiled.lightning_strike,
                            index,
                            &criterion.name,
                            lightning::StrikeCondition::parse(conditions, lookup),
                        );
                    }
                    lightning::CHANNELED_LIGHTNING => {
                        insert_compiled(
                            &mut compiled.channeled_lightning,
                            index,
                            &criterion.name,
                            lightning::VictimsCondition::parse(conditions, lookup),
                        );
                    }
                    killed_by_arrow::KILLED_BY_ARROW => {
                        insert_compiled(
                            &mut compiled.killed_by_arrow,
                            index,
                            &criterion.name,
                            killed_by_arrow::KilledCondition::parse(conditions, lookup),
                        );
                    }
                    target_hit::TARGET_HIT => {
                        insert_compiled(
                            &mut compiled.target_hit,
                            index,
                            &criterion.name,
                            target_hit::TargetHitCondition::parse(conditions, lookup),
                        );
                    }
                    "minecraft:summoned_entity" => {
                        insert_compiled(
                            &mut compiled.summoned,
                            index,
                            &criterion.name,
                            single_entity_condition(conditions, lookup),
                        );
                    }
                    "minecraft:tame_animal" => {
                        insert_compiled(
                            &mut compiled.tamed,
                            index,
                            &criterion.name,
                            single_entity_condition(conditions, lookup),
                        );
                    }
                    "minecraft:player_interacted_with_entity"
                    | "minecraft:player_sheared_equipment" => {
                        let condition = single_entity_condition(conditions, lookup).and_then(
                            |mut condition| {
                                condition.item = item_criterion(conditions.field("item"))?;
                                Ok(condition)
                            },
                        );
                        insert_compiled(
                            compiled
                                .interacted
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            condition,
                        );
                    }
                    VILLAGER_TRADE => {
                        insert_compiled(
                            &mut compiled.traded,
                            index,
                            &criterion.name,
                            traded_condition(conditions, lookup),
                        );
                    }
                    fishing_rod_hooked::FISHING_ROD_HOOKED => {
                        insert_compiled(
                            &mut compiled.fishing_rod_hooked,
                            index,
                            &criterion.name,
                            fishing_rod_hooked::HookedCondition::parse(conditions, lookup),
                        );
                    }
                    spear_mobs::SPEAR_MOBS => {
                        insert_compiled(
                            &mut compiled.spear_mobs,
                            index,
                            &criterion.name,
                            spear_mobs::SpearMobsCondition::parse(conditions, lookup),
                        );
                    }
                    used_ender_eye::USED_ENDER_EYE => {
                        insert_compiled(
                            &mut compiled.used_ender_eye,
                            index,
                            &criterion.name,
                            used_ender_eye::EyeCondition::parse(conditions, lookup),
                        );
                    }
                    CONSUME_ITEM | USING_ITEM | USED_TOTEM | FILLED_BUCKET | SHOT_CROSSBOW => {
                        insert_compiled(
                            compiled
                                .item_criteria
                                .entry(criterion.trigger.clone())
                                .or_default(),
                            index,
                            &criterion.name,
                            item_use::ItemCondition::parse(conditions, lookup),
                        );
                    }
                    picked_up_item::THROWN_ITEM_PICKED_UP_BY_ENTITY
                    | picked_up_item::THROWN_ITEM_PICKED_UP_BY_PLAYER => {
                        insert_compiled(
                            &mut compiled.picked_up_item,
                            index,
                            &criterion.name,
                            picked_up_item::picked_up_item_condition(conditions, lookup),
                        );
                    }
                    item_durability::ITEM_DURABILITY_CHANGED => {
                        insert_compiled(
                            &mut compiled.item_durability,
                            index,
                            &criterion.name,
                            item_durability::ItemDurabilityCondition::parse(conditions, lookup),
                        );
                    }
                    enchanted_item::ENCHANTED_ITEM => {
                        insert_compiled(
                            &mut compiled.enchanted_item,
                            index,
                            &criterion.name,
                            enchanted_item::EnchantedItemCondition::parse(conditions, lookup),
                        );
                    }
                    generate_loot::GENERATE_LOOT => {
                        insert_compiled(
                            &mut compiled.generate_loot,
                            index,
                            &criterion.name,
                            generate_loot::LootTableCondition::parse(conditions, lookup),
                        );
                    }
                    "minecraft:impossible" => {}
                    other => unreachable!("unhandled validated advancement trigger {other}"),
                }
            }
        }
        compiled
    }
}

impl ServerWorld {
    // AbstractContainerMenu.java:194-229 runs each listener before synchronizing its slot.
    pub(super) fn broadcast_player_menu(
        &mut self,
        connection_id: ConnectionId,
        mode: MenuBroadcastMode,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let mut cursor = player.inventory.begin_menu_broadcast(mode);
        while let Some(player) = self.levels.player_mut(connection_id) {
            let event = match player.inventory.next_menu_broadcast(
                &resolver,
                Some(&mut self.maps),
                &mut cursor,
            ) {
                Ok(event) => event,
                Err(error) => {
                    tracing::warn!("player menu broadcast: {error}");
                    return;
                }
            };
            match event {
                Some(MenuBroadcastEvent::InventoryChanged { changed_item, .. }) => {
                    self.trigger_inventory_changed(connection_id, &changed_item, events);
                    self.refresh_open_container(connection_id);
                }
                Some(MenuBroadcastEvent::Remote(sync)) => {
                    Self::send_menu_sync(connection_id, [sync], events)
                }
                None => break,
            }
        }
    }

    pub(super) fn broadcast_player_menu_checked(
        &mut self,
        target: ConnectionId,
        mode: MenuBroadcastMode,
        events: &mut Vec<WorldEvent>,
    ) -> std::result::Result<(), String> {
        if !self.levels.contains_player(target) {
            return Err("menu source player is unavailable".to_owned());
        }
        self.broadcast_player_menu(target, mode, events);
        Ok(())
    }

    pub(super) fn initialize_player_menu(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(error) = self.initialize_player_menu_checked(connection_id, events) {
            tracing::warn!("{error}");
        }
    }

    pub(super) fn initialize_player_menu_checked(
        &mut self,
        target: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> std::result::Result<(), String> {
        self.broadcast_player_menu_checked(target, MenuBroadcastMode::ListenersOnly, events)?;
        let content = self
            .levels
            .player_mut(target)
            .ok_or_else(|| "menu source player is unavailable".to_owned())?
            .inventory
            .send_all_data_to_remote()
            .map_err(|error| format!("initialize player menu: {error}"))?;
        Self::send_menu_sync(target, [content], events);
        self.sync_menu_data(target, true, events);
        Ok(())
    }

    pub(super) fn trigger_recipe_unlocked(
        &mut self,
        connection_id: ConnectionId,
        recipe: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(candidates) = self.cold.advancement_triggers.recipes.get(recipe) else {
            return;
        };
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let matches_player = self.player_matcher(connection_id, "minecraft:recipe_unlocked");
        let matching = candidates
            .iter()
            .filter(|(index, criterion, condition)| {
                player.cold.advancements.is_listening(
                    "minecraft:recipe_unlocked",
                    *index,
                    criterion,
                ) && matches_player(condition)
            })
            .map(|(index, criterion, _)| (*index, criterion.clone()))
            .collect();
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// The `CriteriaTriggers.EFFECTS_CHANGED.trigger` all three of
    /// `ServerPlayer.onEffectAdded` (`ServerPlayer.java:1644`),
    /// `onEffectUpdated` (`:1651`) and `onEffectsRemoved` (`:1665`) close on,
    /// measured against the effect map the change has already left behind.
    pub(super) fn trigger_effects_changed(
        &mut self,
        connection_id: ConnectionId,
        source: Option<(LevelId, SourceEntity)>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let mut matching = Vec::new();
        let mut player_facts = None;
        for (index, criterion) in player
            .cold
            .advancements
            .trigger_listeners("minecraft:effects_changed")
        {
            if let Some(condition) = self
                .cold
                .advancement_triggers
                .effects_changed
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            {
                if !condition.matches(&player.effects) {
                    continue;
                }
                if let Some(conditions) = &condition.source {
                    let Some((dimension, source)) = source else {
                        continue;
                    };
                    let Some(matches) = self.entity_matcher(
                        connection_id,
                        &player.dimension,
                        "minecraft:effects_changed",
                    ) else {
                        continue;
                    };
                    let matched = match source {
                        SourceEntity::Session(id) => self
                            .session_loot_view(id)
                            .is_some_and(|facts| matches(LootEntity::Facts(&facts), conditions)),
                        SourceEntity::Stored(id) => {
                            self.levels.get(&dimension).is_some_and(|level| {
                                level.entities.get(id).is_some_and(|entity| {
                                    matches(
                                        LootEntity::Stored(StoredLootEntity {
                                            entity,
                                            bodies: Some(
                                                crate::entity::store::EntityBodyReadView::new(
                                                    &level.entities,
                                                    None,
                                                    None,
                                                ),
                                            ),
                                            raid_at_position: Some(
                                                level.raids.raid_at(entity.block_position()),
                                            ),
                                        }),
                                        conditions,
                                    )
                                })
                            })
                        }
                    };
                    if !matched {
                        continue;
                    }
                }
                if !condition.player.is_empty() {
                    let (Some(startup), Some(level)) = (
                        self.cold.config.datapacks.as_ref(),
                        self.levels.get(&player.dimension),
                    ) else {
                        continue;
                    };
                    let facts =
                        player_facts.get_or_insert_with(|| self.loot_view_for_player(player));
                    let blocks = LoadedBlocks {
                        chunks: &level.chunks,
                    };
                    let block_at = |pos| blocks.loaded_block_state(pos);
                    let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
                    let player_nbt = |uuid| self.player_loot_nbt(uuid);
                    let item_environment = self
                        .predicate_loot_environment(level, player.dimension)
                        .with_player_slots(&player_slots)
                        .with_player_nbt(&player_nbt);
                    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                        self.world_vehicle_slots(Some(player), requested, selected)
                    };
                    let params = LootParams {
                        dimension: Some(level.dimension.as_str()),
                        item_environment: Some(&item_environment),
                        vehicle_slots: Some(&vehicle_slots),
                        origin: Some(LootOrigin {
                            x: player.position().x,
                            y: player.position().y,
                            z: player.position().z,
                        }),
                        entities: LootEntities {
                            this_entity: LootEntity::Facts(facts),
                            ..Default::default()
                        },
                        level: Some(&block_at),
                        block_entity_data: Some(&|pos| {
                            blocks
                                .chunks
                                .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                                .map_err(|error| {
                                    crate::loot_runtime::LootRuntimeReason::Catalog(
                                        error.to_string(),
                                    )
                                })
                        }),
                        weather: Some(LootWeather {
                            raining: level.weather.is_raining(),
                            thundering: level.weather.is_thundering(),
                        }),
                        ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
                    };
                    if matches_conditions(
                        &startup.loot,
                        "minecraft:effects_changed",
                        &condition.player,
                        &params,
                        &level.random,
                    ) != Ok(true)
                    {
                        continue;
                    }
                }
                matching.push((index, criterion.clone()));
            }
        }
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// `ServerPlayer.tick`'s levitation arm (`ServerPlayer.java:600-602`): for
    /// as long as the window `onEffectAdded` opened stands, the criterion is
    /// offered every tick over the distance and the tick count since it opened.
    fn trigger_levitation(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let Some(start) = player.cold.levitation_start_position else {
            return;
        };
        let end = player.position();
        let duration = player.tick_count - player.cold.levitation_start_tick;
        let matches_player = self.player_matcher(connection_id, "minecraft:levitation");
        let mut matching = Vec::new();
        for (index, criterion) in player
            .cold
            .advancements
            .trigger_listeners("minecraft:levitation")
        {
            if let Some(condition) = self
                .cold
                .advancement_triggers
                .levitation
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            {
                // `TriggerInstance.matches` (`LevitationTrigger.java:37-41`).
                if condition.distance.matches_delta([
                    (start.x - end.x) as f32,
                    (start.y - end.y) as f32,
                    (start.z - end.z) as f32,
                ]) && condition.duration.matches(duration)
                    && matches_player(&condition.player)
                {
                    matching.push((index, criterion.clone()));
                }
            }
        }
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// `ServerPlayer.triggerDimensionChangeTriggers` (`ServerPlayer.java:1143-1154`),
    /// which the cross-level arm of `teleport` runs at `:1123` -- after the
    /// arriving level already holds the session, so the position the travel
    /// distance is measured to is the one it landed at.
    pub(super) fn trigger_dimension_change(
        &mut self,
        connection_id: ConnectionId,
        from: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let to = player.dimension;
        let (from_key, to_key) = (self.level_key(*from).clone(), self.level_key(to).clone());
        self.trigger_changed_dimension(connection_id, &from_key, &to_key, events);
        // `:1147-1149`.
        if from.builtin() == Some(Java26BuiltinDimension::Nether)
            && to.builtin() == Some(Java26BuiltinDimension::Overworld)
        {
            self.trigger_nether_travel(connection_id, events);
        }
        // `:1151-1153`.
        if to.builtin() != Some(Java26BuiltinDimension::Nether) {
            if let Some(player) = self.levels.player_mut(connection_id) {
                player.cold.entered_nether_position = None;
            }
        }
    }

    /// `CriteriaTriggers.CHANGED_DIMENSION.trigger` alone.
    pub(super) fn trigger_changed_dimension(
        &mut self,
        connection_id: ConnectionId,
        from: &Java26LevelKey,
        to: &Java26LevelKey,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let matches_player = self.player_matcher(connection_id, "minecraft:changed_dimension");
        let mut matching = Vec::new();
        for (index, criterion) in player
            .cold
            .advancements
            .trigger_listeners("minecraft:changed_dimension")
        {
            if let Some(condition) = self
                .cold
                .advancement_triggers
                .dimension_changes
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            {
                if condition.matches(from, to) && matches_player(&condition.player) {
                    matching.push((index, criterion.clone()));
                }
            }
        }
        // `:1146`, whose whole award runs before `:1147` reads the travel.
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// `CriteriaTriggers.NETHER_TRAVEL` through `DistanceTrigger.trigger`
    /// (`DistanceTrigger.java:18-21`), which measures the session's position
    /// now against the one it entered the Nether at.
    fn trigger_nether_travel(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let Some(start) = player.cold.entered_nether_position else {
            return;
        };
        self.trigger_distance(connection_id, "minecraft:nether_travel", start, events);
    }

    pub(in crate::world) fn trigger_distance(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        start: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_distance_with_cause(connection_id, trigger, start, None, events);
    }

    fn trigger_distance_with_cause(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        start: Vec3d,
        cause: Option<&LootEntityFacts>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let Some(compiled) = self.cold.advancement_triggers.distance.get(trigger) else {
            return;
        };
        let end = player.position();
        // Only a `start_position` reads the level, so a criterion without one
        // is answered whether or not there is a view of it.
        let level = self.levels.get(&player.dimension);
        let environment =
            level.map(|level| self.predicate_loot_environment(level, player.dimension));
        let blocks = level.map(|level| LoadedBlocks {
            chunks: &level.chunks,
        });
        let loaded_block_state = blocks
            .as_ref()
            .map(|blocks| move |pos| blocks.loaded_block_state(pos));
        let resolver = level.map(|level| level.entities.item_resolver());
        let block_entity_data = level.map(|level| {
            move |pos| {
                level
                    .chunks
                    .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                    .map_err(|error| {
                        crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                    })
            }
        });
        let matches_player = self.player_matcher(connection_id, trigger);
        let matches_cause = self.entity_matcher(connection_id, &player.dimension, trigger);
        let mut matching = Vec::new();
        for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
            let Some(condition) = compiled
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            // `DistanceTrigger.TriggerInstance.matches`
            // (`DistanceTrigger.java:52-57`): the start position is read at the
            // level the session arrived in, before the distance between the two
            // positions is measured.
            let started = condition.start_position.as_ref().is_none_or(|predicate| {
                location_predicate_matches_in_world(
                    predicate,
                    [start.x, start.y, start.z],
                    Some(self.level_key(player.dimension).as_str()),
                    loaded_block_state
                        .as_ref()
                        .map(|read| read as &LootBlockStateAt<'_>),
                    block_entity_data
                        .as_ref()
                        .map(|read| read as &crate::loot_runtime::LootBlockEntityDataAt<'_>),
                    None,
                    None,
                    None,
                    None,
                    resolver,
                    environment.as_ref().map(|environment| {
                        environment as &dyn crate::loot_runtime::LootItemEnvironment
                    }),
                ) == Ok(true)
            });
            if started
                && condition.distance.matches_delta([
                    (start.x - end.x) as f32,
                    (start.y - end.y) as f32,
                    (start.z - end.z) as f32,
                ])
                && condition.cause.as_ref().is_none_or(|predicate| {
                    cause.is_some_and(|cause| {
                        matches_cause
                            .as_ref()
                            .is_some_and(|matches| matches(LootEntity::Facts(cause), predicate))
                    })
                })
                && matches_player(&condition.player)
            {
                matching.push((index, criterion.clone()));
            }
        }
        drop(matches_player);
        drop(matches_cause);
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// `ItemUsedOnLocationTrigger.trigger`
    /// (`ItemUsedOnLocationTrigger.java:34-45`) for whichever of its consumers
    /// `trigger` names: the position's live state, the player and the copy of
    /// the used stack, bound to `LootContextParamSets.ADVANCEMENT_LOCATION`.
    pub(super) fn trigger_item_used_on_location(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        pos: BlockPos,
        tool: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .next()
            .is_none()
        {
            return;
        }
        let dimension = player.dimension;
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return;
        };
        let tool = loot_tool_for_stack(tool, &self.datapack_tables.enchantment_registry);
        // `:36`. `Level.getBlockState` answers void air outside the build
        // height rather than nothing, and the origin is the cell's centre
        // (`:38`).
        let block_state_id = self
            .block_state_by_key(&dimension, pos)
            .unwrap_or_else(|| self.level_by_id(&dimension).chunks.air_state_id());
        let player = self.levels.player_at(connection_id);
        let Some(matching) = self.with_advancement_player_params(player, |level, params| {
            let params = LootParams {
                origin: Some(LootOrigin::at_center_of(pos)),
                block_state_id: Some(block_state_id),
                tool: Some(&tool),
                weather: None,
                param_set: param_sets::ADVANCEMENT_LOCATION,
                ..params
            };
            let matches_player = self.player_matcher(connection_id, trigger);
            let compiled = self
                .cold
                .advancement_triggers
                .item_used_on_location
                .get(trigger);
            let mut matching = Vec::new();
            for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
                let Some(condition) = compiled
                    .and_then(|criteria| criteria.get(&index))
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                // `TriggerInstance.matches` (`:114-116`), whose absent list is
                // true before the context is read.
                if matches_conditions(
                    &startup.loot,
                    trigger,
                    &condition.location,
                    &params,
                    &level.random,
                ) == Ok(true)
                    && matches_player(&condition.player)
                {
                    matching.push((index, criterion.clone()));
                }
            }
            matching
        }) else {
            return;
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// `KilledTrigger.trigger` (`KilledTrigger.java:21-24`) for whichever of its
    /// consumers `trigger` names: the other side of the kill bound to
    /// `LootContextParamSets.ADVANCEMENT_ENTITY` at the position of the session
    /// the criterion belongs to (`EntityPredicate.java:182-188`), and the blow
    /// that ended the kill (`DamageSourcePredicate.java:28-46`).
    pub(super) fn trigger_killed(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        entity: KillEntity,
        killing_blow: KillingBlow,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection_id,
            trigger,
            EntityContextTable::Killed,
            EntityContextTarget::Captured(entity),
            (Some(killing_blow), None),
            events,
        );
    }

    /// `TameAnimalTrigger.trigger` (`TameAnimalTrigger.java:20-23`): the same
    /// `EntityPredicate.createContext(player, animal)` the kill family builds
    /// for the entity on the other side, with no second term to read.
    pub(super) fn trigger_tame_animal(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection_id,
            "minecraft:tame_animal",
            EntityContextTable::Tamed,
            EntityContextTarget::Stored(dimension, entity_id),
            (None, None),
            events,
        );
    }

    // SummonedEntityTrigger.java:20-22,25-33,41-42.
    pub(super) fn trigger_summoned_entity(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection_id,
            "minecraft:summoned_entity",
            EntityContextTable::Summoned,
            EntityContextTarget::Stored(dimension, entity_id),
            (None, None),
            events,
        );
    }

    /// `TradeTrigger.trigger` (`TradeTrigger.java:21-24`) over the offer's
    /// result, fired from `AbstractVillager.notifyTrade` (`AbstractVillager.java:139`).
    pub(super) fn trigger_villager_trade(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        entity_id: i32,
        result: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection_id,
            VILLAGER_TRADE,
            EntityContextTable::Traded,
            EntityContextTarget::Stored(dimension, entity_id),
            (None, Some(result)),
            events,
        );
    }

    // PlayerInteractTrigger.java:21-34; Entity.java:2287 supplies the removed equipment.
    pub(in crate::world) fn trigger_player_entity_interaction(
        &mut self,
        connection: ConnectionId,
        trigger: &str,
        dimension: LevelId,
        entity_id: i32,
        item: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection,
            trigger,
            EntityContextTable::Interacted,
            EntityContextTarget::Stored(dimension, entity_id),
            (None, Some(item)),
            events,
        );
    }

    pub(in crate::world) fn trigger_captured_entity_interaction(
        &mut self,
        connection: ConnectionId,
        entity: KillEntity,
        item: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_entity_context(
            connection,
            "minecraft:player_interacted_with_entity",
            EntityContextTable::Interacted,
            EntityContextTarget::Captured(entity),
            (None, Some(item)),
            events,
        );
    }

    fn trigger_entity_context(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        table: EntityContextTable,
        entity: EntityContextTarget,
        (killing_blow, item): (Option<KillingBlow>, Option<&ItemStackSummary>),
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .next()
            .is_none()
        {
            return;
        }
        let dimension = player.dimension;
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return;
        };
        let tool = item
            .map(|stack| loot_tool_for_stack(stack, &self.datapack_tables.enchantment_registry));
        let matching = {
            let ServerWorld { levels, cold, .. } = &*self;
            let advancement_triggers = &cold.advancement_triggers;
            let (Some(player), Some(level)) =
                (levels.player(connection_id), levels.get(&dimension))
            else {
                return;
            };
            let blocks = LoadedBlocks {
                chunks: &level.chunks,
            };
            let origin = LootOrigin {
                x: player.position().x,
                y: player.position().y,
                z: player.position().z,
            };
            let block_at = |pos| blocks.loaded_block_state(pos);
            let weather = LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            };
            let player_facts = self.loot_view_for_player(player);
            let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
            let player_nbt = |uuid| self.player_loot_nbt(uuid);
            let item_environment = self
                .predicate_loot_environment(level, dimension)
                .with_player_slots(&player_slots)
                .with_player_nbt(&player_nbt);
            let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                self.world_vehicle_slots(Some(player), requested, selected)
            };
            let player_params = LootParams {
                dimension: Some(level.dimension.as_str()),
                item_environment: Some(&item_environment),
                vehicle_slots: Some(&vehicle_slots),
                origin: Some(origin),
                entities: LootEntities {
                    this_entity: LootEntity::Facts(&player_facts),
                    ..Default::default()
                },
                level: Some(&block_at),
                block_entity_data: Some(&|pos| {
                    blocks
                        .chunks
                        .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                }),
                weather: Some(weather),
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            let target = match &entity {
                EntityContextTarget::Captured(entity) => entity.loot(),
                EntityContextTarget::Stored(dimension, entity_id) => {
                    let Some(target_level) = levels.get(dimension) else {
                        return;
                    };
                    let Some(entity) = target_level.entities.get(*entity_id) else {
                        return;
                    };
                    LootEntity::Stored(StoredLootEntity {
                        entity,
                        bodies: Some(crate::entity::store::EntityBodyReadView::new(
                            &target_level.entities,
                            None,
                            None,
                        )),
                        raid_at_position: Some(target_level.raids.raid_at(entity.block_position())),
                    })
                }
            };
            let params = LootParams {
                dimension: Some(level.dimension.as_str()),
                item_environment: Some(&item_environment),
                vehicle_slots: Some(&vehicle_slots),
                origin: Some(origin),
                entities: LootEntities {
                    this_entity: target,
                    ..Default::default()
                },
                level: Some(&block_at),
                block_entity_data: Some(&|pos| {
                    blocks
                        .chunks
                        .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                }),
                weather: Some(weather),
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            let compiled = match table {
                EntityContextTable::Killed => advancement_triggers.killed.get(trigger),
                EntityContextTable::Tamed => Some(&advancement_triggers.tamed),
                EntityContextTable::Summoned => Some(&advancement_triggers.summoned),
                EntityContextTable::Traded => Some(&advancement_triggers.traded),
                EntityContextTable::Interacted => advancement_triggers.interacted.get(trigger),
            };
            let mut matching = Vec::new();
            for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
                let Some(condition) = compiled
                    .and_then(|criteria| criteria.get(&index))
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                if matches_conditions(
                    &startup.loot,
                    trigger,
                    &condition.player,
                    &player_params,
                    &level.random,
                ) != Ok(true)
                {
                    continue;
                }
                // `:127`, before `:129` reads the context. A trigger whose
                // instance has no killing-blow field compiles none, so the
                // pairing never drops a term a pack declared.
                if let (Some(predicate), Some(killing_blow)) =
                    (&condition.killing_blow, &killing_blow)
                {
                    let damage_params = LootParams {
                        param_set: param_sets::ENTITY,
                        damage_source: LootDamageSource::Present(LootDamageSourceFacts {
                            damage_type: &killing_blow.damage_type,
                            is_direct: killing_blow.is_direct,
                            tags: Some(
                                startup
                                    .tags
                                    .registry(bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY),
                            ),
                            causing: killing_blow.causing_entity.loot(),
                            direct: killing_blow.direct_entity.as_ref().map(KillEntity::loot),
                        }),
                        ..params
                    };
                    if LootEvaluator::new(&startup.loot, trigger, &damage_params, &level.random)
                        .and_then(|mut evaluator| evaluator.condition(predicate, "killing_blow"))
                        != Ok(true)
                    {
                        continue;
                    }
                }
                // `:129`, whose absent list is true before the context is read.
                if matches_conditions(
                    &startup.loot,
                    trigger,
                    &condition.entity,
                    &params,
                    &level.random,
                ) != Ok(true)
                {
                    continue;
                }
                // `TradeTrigger.java:47` reads the item after the villager.
                let item_matches = match (&condition.item, &tool, &advancement_triggers.item_tags) {
                    (None, _, _) => true,
                    (Some(predicate), Some(tool), Some(tags)) => {
                        matches_item_predicate(
                            predicate,
                            tool,
                            tags,
                            level.entities.item_resolver(),
                        ) == Ok(true)
                    }
                    (Some(_), _, _) => false,
                };
                if item_matches {
                    matching.push((index, criterion.clone()));
                }
            }
            matching
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }

    /// One entity id a damage source names, resolved the way
    /// `DamageSource.getEntity()` holds it: a session through the player list,
    /// every other entity through the level that owns it.
    pub(super) fn killed_entity_of_id(&self, dimension: &LevelId, entity_id: i32) -> KillEntity {
        if let Some(facts) = self
            .connection_by_entity_id(entity_id)
            .and_then(|connection_id| self.session_loot_view(connection_id))
        {
            return KillEntity::Facts(Box::new(facts));
        }
        self.killed_entity_of_stored(dimension, entity_id)
    }

    /// One stored entity by id. An id the level no longer holds is an entity
    /// with nothing readable rather than an absent one: the source named it.
    pub(super) fn killed_entity_of_stored(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> KillEntity {
        let resolver = self.item_stack_resolver();
        let level = self.level_by_id(dimension);
        level
            .entities
            .get(entity_id)
            .map_or(KillEntity::Opaque, |entity| {
                KillEntity::Facts(Box::new(
                    StoredLootEntity {
                        entity,
                        bodies: Some(crate::entity::store::EntityBodyReadView::new(
                            &level.entities,
                            None,
                            None,
                        )),
                        raid_at_position: Some(level.raids.raid_at(entity.block_position())),
                    }
                    .snapshot(resolver.as_dyn()),
                ))
            })
    }

    fn trigger_inventory_changed(
        &mut self,
        connection_id: ConnectionId,
        changed: &RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners("minecraft:inventory_changed")
            .next()
            .is_none()
        {
            return;
        }
        let Some(tags) = self.cold.advancement_triggers.item_tags.as_ref() else {
            return;
        };
        let slots = player.inventory.inventory_slots();
        let publication = self
            .level_by_id(&player.dimension)
            .entities
            .item_publication();
        let mut occupied = 0;
        let mut full = 0;
        let mut empty = 0;
        for stack in &slots {
            if stack.is_empty() {
                empty += 1;
            } else {
                occupied += 1;
                if stack.max_stack_size().is_ok_and(|max| stack.count() >= max) {
                    full += 1;
                }
            }
        }
        let mut matching = Vec::new();
        let mut predicate_matches = Vec::new();
        let mut tools = None;
        let mut changed_tool = None;
        let empty_tool = LootTool::empty();
        for (index, criterion) in player
            .cold
            .advancements
            .trigger_listeners("minecraft:inventory_changed")
        {
            let Some(condition) = self
                .cold
                .advancement_triggers
                .inventory
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            let inventory = &condition.inventory;
            if !inventory.slots.full.matches(full)
                || !inventory.slots.empty.matches(empty)
                || !inventory.slots.occupied.matches(occupied)
            {
                continue;
            }
            let inventory_tools = if inventory.items.len() > 1 {
                tools
                    .get_or_insert_with(|| {
                        slots
                            .iter()
                            .filter(|stack| !stack.is_empty())
                            .map(|stack| {
                                loot_tool_for_stack(
                                    stack.summary(),
                                    &self.datapack_tables.enchantment_registry,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .as_slice()
            } else {
                &[]
            };
            let changed_item = if inventory.items.len() == 1 && !changed.is_empty() {
                changed_tool.get_or_insert_with(|| {
                    loot_tool_for_stack(
                        changed.summary(),
                        &self.datapack_tables.enchantment_registry,
                    )
                })
            } else {
                &empty_tool
            };
            let matched = inventory_condition_matches(
                inventory,
                [occupied, full, empty],
                inventory_tools,
                changed_item,
                !changed.is_empty(),
                tags,
                publication.resolver(),
            );
            if matched != Ok(true) {
                continue;
            }
            if condition.player.is_empty() {
                matching.push((index, criterion.clone()));
            } else {
                predicate_matches.push((index, criterion, &condition.player));
            }
        }
        if !predicate_matches.is_empty() {
            if let (Some(startup), Some(level)) = (
                self.cold.config.datapacks.as_ref(),
                self.levels.get(&player.dimension),
            ) {
                // SimpleCriterionTrigger.java:43-59: player predicates read the actual player context.
                let facts = self.loot_view_for_player(player);
                let blocks = LoadedBlocks {
                    chunks: &level.chunks,
                };
                let block_at = |pos| blocks.loaded_block_state(pos);
                let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
                let player_nbt = |uuid| self.player_loot_nbt(uuid);
                let item_environment = self
                    .predicate_loot_environment(level, player.dimension)
                    .with_player_slots(&player_slots)
                    .with_player_nbt(&player_nbt);
                let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                    self.world_vehicle_slots(Some(player), requested, selected)
                };
                let block_entity_data = |pos| {
                    blocks
                        .chunks
                        .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                };
                let params = LootParams {
                    dimension: Some(level.dimension.as_str()),
                    item_environment: Some(&item_environment),
                    vehicle_slots: Some(&vehicle_slots),
                    origin: Some(LootOrigin {
                        x: player.position().x,
                        y: player.position().y,
                        z: player.position().z,
                    }),
                    entities: LootEntities {
                        this_entity: LootEntity::Facts(&facts),
                        ..Default::default()
                    },
                    level: Some(&block_at),
                    block_entity_data: Some(&block_entity_data),
                    weather: Some(LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    }),
                    ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
                };
                for (index, criterion, player_conditions) in predicate_matches {
                    if matches_conditions(
                        &startup.loot,
                        "minecraft:inventory_changed",
                        player_conditions,
                        &params,
                        &level.random,
                    ) == Ok(true)
                    {
                        matching.push((index, criterion.clone()));
                    }
                }
            }
        }
        self.apply_advancement_matches(connection_id, matching, events);
    }

    fn with_advancement_player_params<R>(
        &self,
        player: &PlayerState,
        run: impl FnOnce(&super::ServerLevel, LootParams<'_>) -> R,
    ) -> Option<R> {
        let level = self.levels.get(&player.dimension)?;
        let entity = self.loot_view_for_player(player);
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
        let player_nbt = |uuid| self.player_loot_nbt(uuid);
        // Keep the live environment outside the criterion loop.
        let item_environment = self
            .predicate_loot_environment(level, player.dimension)
            .with_player_slots(&player_slots)
            .with_player_nbt(&player_nbt);
        let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
            self.world_vehicle_slots(Some(player), requested, selected)
        };
        let params = LootParams {
            dimension: Some(level.dimension.as_str()),
            item_environment: Some(&item_environment),
            vehicle_slots: Some(&vehicle_slots),
            origin: Some(LootOrigin {
                x: player.position().x,
                y: player.position().y,
                z: player.position().z,
            }),
            entities: LootEntities {
                this_entity: LootEntity::Facts(&entity),
                ..Default::default()
            },
            level: Some(&|pos| blocks.loaded_block_state(pos)),
            block_entity_data: Some(&|pos| {
                blocks
                    .chunks
                    .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                    .map_err(|error| {
                        crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                    })
            }),
            weather: Some(LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            }),
            ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
        };
        Some(run(level, params))
    }

    fn apply_advancement_matches(
        &mut self,
        connection_id: ConnectionId,
        matching: Vec<(usize, JavaString)>,
        events: &mut Vec<WorldEvent>,
    ) {
        for (index, criterion) in matching {
            self.award_advancement_criterion(
                connection_id,
                index,
                &criterion,
                self.cold.wall_clock_millis,
                events,
            );
        }
    }
}

/// `ChangeDimensionTrigger.TriggerInstance` (`ChangeDimensionTrigger.java:23-32`).
struct DimensionChangeCondition {
    player: Vec<LootCondition>,
    from: Option<Java26LevelKey>,
    to: Option<Java26LevelKey>,
}

impl DimensionChangeCondition {
    // `:54-56`.
    fn matches(&self, from: &Java26LevelKey, to: &Java26LevelKey) -> bool {
        self.from.as_ref().is_none_or(|key| key == from)
            && self.to.as_ref().is_none_or(|key| key == to)
    }
}

/// `ItemUsedOnLocationTrigger.TriggerInstance`
/// (`ItemUsedOnLocationTrigger.java:47-55`).
struct ItemUsedOnLocationCondition {
    player: Vec<LootCondition>,
    location: Vec<LootCondition>,
}

enum EntityContextTarget {
    Captured(KillEntity),
    Stored(LevelId, i32),
}

/// Which compiled table one [`EntityContextCondition`] lookup reads. The kill
/// family is keyed by trigger id because three consumers share one shape;
/// taming, summoning and trading each have a single consumer.
#[derive(Clone, Copy)]
enum EntityContextTable {
    Killed,
    Tamed,
    Summoned,
    Traded,
    Interacted,
}

/// One criterion whose whole instance is a predicate over a second entity in
/// the player's own context: `KilledTrigger.TriggerInstance`
/// (`KilledTrigger.java:26-35`), which also reads the blow, and
/// `TameAnimalTrigger.TriggerInstance` (`TameAnimalTrigger.java:25-33`), which
/// declares no such field.
struct EntityContextCondition {
    player: Vec<LootCondition>,
    entity: Vec<LootCondition>,
    killing_blow: Option<Box<LootCondition>>,
    /// `TradeTrigger.TriggerInstance.item` (`TradeTrigger.java:32`).
    item: ItemCriterion,
}

/// One entity a kill names: the other side of the kill
/// (`KilledTrigger.java:22`) or one slot of its killing blow
/// (`DamageSource.java:59-65`), as far as the kill site can state it.
#[derive(Clone)]
pub(super) enum KillEntity {
    /// The slot answered `null`, which `EntityPredicate.matches` rejects before
    /// it reads a field (`EntityPredicate.java:92-95`).
    Null,
    /// The entity is present and nothing of it is readable.
    Opaque,
    /// The entity read before the kill site released the world.
    Facts(Box<LootEntityFacts>),
}

impl KillEntity {
    /// The value the `LootContextParams` key binds.
    fn loot<'a>(&'a self) -> LootEntity<'a> {
        match self {
            Self::Null => LootEntity::Absent,
            Self::Opaque => LootEntity::Opaque,
            Self::Facts(facts) => LootEntity::Facts(facts),
        }
    }
}

/// `KilledTrigger.trigger`'s `killingBlow` (`KilledTrigger.java:21-23`).
#[derive(Clone)]
pub(super) struct KillingBlow {
    /// `DamageSource.getType()`'s registry id, which is what the `tags` term
    /// tests membership against.
    pub(super) damage_type: String,
    /// `DamageSource.getDirectEntity()`; `None` is a slot the site cannot
    /// state, which turns down a criterion naming it instead of answering it.
    pub(super) direct_entity: Option<KillEntity>,
    /// `DamageSource.getEntity()`'s known entity or null slot.
    pub(super) causing_entity: KillEntity,
    /// `DamageSource.isDirect()` (`DamageSource.java:30-32`); `None` when the
    /// direct slot is unknown.
    pub(super) is_direct: Option<bool>,
}

/// `EntityFlagsPredicate.matches` (`EntityFlagsPredicate.java:36-71`) over a session.
pub(in crate::world) fn session_loot_entity_flags(player: &PlayerState) -> LootEntityFlags {
    LootEntityFlags {
        on_ground: Some(player.on_ground()),
        // Entity.java:2586-2589: the server fire clock determines burning.
        on_fire: Some(player.remaining_fire_ticks > 0),
        // Entity.java:2631-2633: sneaking reads the actual pose.
        crouching: Some(
            player.pose_id == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
        ),
        sprinting: Some(player.shared_flags & VANILLA_ENTITY_SHARED_FLAG_SPRINTING != 0),
        swimming: Some(
            player.shared_flags
                & bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING
                != 0,
        ),
        in_water: Some(
            player
                .fluid_interaction
                .is_in_fluid(crate::entity::FluidTag::Water),
        ),
        fall_flying: Some(player.shared_flags & VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING != 0),
        ability_flying: Some(player.abilities.flying),
        // `Player` overrides neither `LivingEntity.isBaby` nor the
        // `DATA_BABY_ID` families, so it answers the base false.
        baby: Some(false),
    }
}

/// `DistanceTrigger.TriggerInstance` (`DistanceTrigger.java:23-32`).
struct TravelDistanceCondition {
    player: Vec<LootCondition>,
    start_position: Option<LocationPredicate>,
    distance: DistancePredicate,
    cause: Option<Vec<LootCondition>>,
}

/// `EffectsChangedTrigger.TriggerInstance` (`EffectsChangedTrigger.java:26-35`).
struct EffectsChangedCondition {
    player: Vec<LootCondition>,
    effects: Option<Vec<(String, MobEffectInstancePredicate)>>,
    source: Option<Vec<LootCondition>>,
}

impl EffectsChangedCondition {
    // `EffectsChangedTrigger.java:47-51`.
    fn matches(&self, effects: &ActiveMobEffects) -> bool {
        self.effects.as_ref().is_none_or(|predicate| {
            mob_effects_predicate_matches(predicate, |id| {
                Ok::<_, std::convert::Infallible>(effects.get(id))
            })
            .unwrap()
        })
    }
}

/// `LevitationTrigger.TriggerInstance` (`LevitationTrigger.java:21-30`).
struct LevitationCondition {
    player: Vec<LootCondition>,
    distance: DistancePredicate,
    duration: IntBounds,
}

fn dimension_change_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<DimensionChangeCondition, String> {
    Ok(DimensionChangeCondition {
        player: construct_beacon::player_conditions(
            conditions.field("player"),
            "minecraft:changed_dimension",
            lookup,
        )?,
        from: level_key(conditions.field("from"))?,
        to: level_key(conditions.field("to"))?,
    })
}

fn item_used_on_location_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<ItemUsedOnLocationCondition, String> {
    Ok(ItemUsedOnLocationCondition {
        player: construct_beacon::player_conditions(
            conditions.field("player"),
            "minecraft:item_used_on_location",
            lookup,
        )?,
        // `TriggerInstance.validate` (`:119-122`).
        location: context_predicate(
            conditions.field("location"),
            "location",
            param_sets::ADVANCEMENT_LOCATION,
            lookup,
        )?,
    })
}

fn killed_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<EntityContextCondition, String> {
    // `:127` reads the killing blow before `:129` reads the entity context.
    let killing_blow = conditions
        .field("killing_blow")
        .map(|value| {
            for field in ["direct_entity", "source_entity"] {
                if let Some(predicate) = value.field(field) {
                    validate_predicate_source(predicate)?;
                }
            }
            let condition = LootCondition::DamageSourceProperties(Some(Box::new(
                DamageSourcePredicate::decode(value).map_err(|error| error.to_string())?,
            )));
            warn_condition_check(
                "killing_blow",
                std::slice::from_ref(&condition),
                param_sets::ENTITY,
            );
            Ok::<_, String>(Box::new(condition))
        })
        .transpose()?;
    Ok(EntityContextCondition {
        player: construct_beacon::player_conditions(conditions.field("player"), "player", lookup)?,
        entity: construct_beacon::player_conditions(conditions.field("entity"), "entity", lookup)?,
        killing_blow,
        item: None,
    })
}

/// `TameAnimalTrigger.TriggerInstance.CODEC` (`TameAnimalTrigger.java:27-33`):
/// the player predicate and one `EntityPredicate` over the tamed animal. The
/// record has no killing-blow field. SummonedEntityTrigger.java:25-33 uses
/// the same player/entity shape.
fn single_entity_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<EntityContextCondition, String> {
    Ok(EntityContextCondition {
        player: construct_beacon::player_conditions(conditions.field("player"), "player", lookup)?,
        entity: construct_beacon::player_conditions(conditions.field("entity"), "entity", lookup)?,
        killing_blow: None,
        item: None,
    })
}

/// `TradeTrigger.TriggerInstance.CODEC` (`TradeTrigger.java:28-35`): the
/// player predicate, an `EntityPredicate` over the villager, and the result's
/// An `ItemPredicate` decoded once; one that does not decode never matches.
pub(super) type ItemCriterion = Option<ItemPredicate>;

pub(super) fn item_criterion(value: Option<V<'_>>) -> Result<ItemCriterion, String> {
    value
        .map(parse_item_predicate)
        .transpose()
        .map_err(|error| error.to_string())
}

/// `ItemPredicate`.
fn traded_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<EntityContextCondition, String> {
    Ok(EntityContextCondition {
        player: construct_beacon::player_conditions(conditions.field("player"), "player", lookup)?,
        entity: construct_beacon::player_conditions(
            conditions.field("villager"),
            "villager",
            lookup,
        )?,
        killing_blow: None,
        item: item_criterion(conditions.field("item"))?,
    })
}

/// One `EntityPredicate` as the condition list the loot runtime evaluates.
///
/// `EntityPredicate.wrap` (`EntityPredicate.java:83-86`) is
/// `LootItemEntityPropertyCondition.hasProperties(THIS, predicate)`, so the
/// terms this crate refuses inside an `entity_properties` condition are exactly
/// the ones it refuses here.
fn entity_predicate_conditions(
    predicate: V<'_>,
    owner: &str,
) -> Result<Vec<LootCondition>, String> {
    if !predicate.is_compound() {
        return Err("entity predicate must be an object".into());
    }
    validate_predicate_source(predicate)?;
    let conditions = vec![LootCondition::EntityProperties {
        entity: bbb_registry_codec::loot_context_types::VanillaLootContextParam::ThisEntity,
        predicate: Some(Box::new(
            EntityPredicate::decode(predicate).map_err(|error| error.to_string())?,
        )),
    }];
    warn_condition_check(owner, &conditions, param_sets::ADVANCEMENT_ENTITY);
    Ok(conditions)
}

fn validate_predicate_source(value: V<'_>) -> Result<(), String> {
    bbb_registry_codec::validate_registry_source_input(
        value,
        bbb_registry_codec::PersistentNbtDecodeLimits {
            max_nodes: usize::MAX,
            ..bbb_registry_codec::PersistentNbtDecodeLimits::DEFAULT
        },
    )
    .map_err(|error| error.to_string())
}

/// `ContextAwarePredicate.CODEC` (`ContextAwarePredicate.java:13-15`) is a
/// condition list, and an absent one is the empty `Util.allOf`. The check
/// beside it is where a term naming a parameter the set does not carry is
/// reported.
fn context_predicate(
    value: Option<V<'_>>,
    field: &str,
    param_set: &'static LootContextParamSet,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<LootCondition>, String> {
    let conditions = match value {
        None => Vec::new(),
        Some(value) => bbb_server_datapack::loot::codec::list(value, |value| {
            parse_loot_condition(value, lookup)
        })
        .map_err(|error| error.to_string())?,
    };
    warn_condition_check(field, &conditions, param_set);
    Ok(conditions)
}

fn travel_distance_condition(
    conditions: V<'_>,
    trigger: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<TravelDistanceCondition, String> {
    let start_position = conditions
        .field("start_position")
        .map(parse_location_predicate)
        .transpose()
        .map_err(|error| error.to_string())?;
    if let Some(start_position) = &start_position {
        if let Err(error) = check_location_predicate(start_position) {
            tracing::warn!("advancement validation problem in {trigger}.start_position: {error}");
        }
    }
    Ok(TravelDistanceCondition {
        player: construct_beacon::player_conditions(conditions.field("player"), trigger, lookup)?,
        start_position,
        distance: conditions
            .field("distance")
            .map(DistancePredicate::decode)
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default(),
        cause: (trigger == "minecraft:fall_after_explosion")
            .then(|| conditions.field("cause"))
            .flatten()
            .map(|value| construct_beacon::player_conditions(Some(value), trigger, lookup))
            .transpose()?,
    })
}

fn effects_changed_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<EffectsChangedCondition, String> {
    Ok(EffectsChangedCondition {
        player: construct_beacon::player_conditions(
            conditions.field("player"),
            "minecraft:effects_changed",
            lookup,
        )?,
        effects: conditions
            .field("effects")
            .map(|value| parse_mob_effects_predicate(value).map_err(|error| error.to_string()))
            .transpose()?,
        source: conditions
            .field("source")
            .map(|value| {
                construct_beacon::player_conditions(
                    Some(value),
                    "minecraft:effects_changed",
                    lookup,
                )
            })
            .transpose()?,
    })
}

fn levitation_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LevitationCondition, String> {
    Ok(LevitationCondition {
        player: construct_beacon::player_conditions(
            conditions.field("player"),
            "minecraft:levitation",
            lookup,
        )?,
        distance: conditions
            .field("distance")
            .map(DistancePredicate::decode)
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default(),
        duration: int_bounds(conditions.field("duration"))?,
    })
}

fn int_bounds(value: Option<V<'_>>) -> Result<IntBounds, String> {
    value
        .map(IntBounds::decode)
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(|error| error.to_string())
}

/// `ResourceKey.codec(Registries.DIMENSION)`, which is one resource location.
fn level_key(value: Option<V<'_>>) -> Result<Option<Java26LevelKey>, String> {
    value
        .map(|value| {
            value
                .utf8_string()
                .and_then(|raw| Java26LevelKey::parse(&raw).ok())
                .ok_or_else(|| format!("{value:?} is not a dimension key"))
        })
        .transpose()
}

fn double_bounds(value: Option<V<'_>>) -> Result<DoubleBounds, String> {
    value
        .map(DoubleBounds::decode)
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(|error| error.to_string())
}

// InventoryChangeTrigger.java:84-110 uses only changedItem for a single item predicate.
fn inventory_condition_matches(
    condition: &InventoryChangedCondition,
    [occupied, full, empty]: [i32; 3],
    inventory: &[LootTool],
    changed: &LootTool,
    changed_nonempty: bool,
    tags: &ItemTagIndex,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<bool, String> {
    if !condition.slots.full.matches(full)
        || !condition.slots.empty.matches(empty)
        || !condition.slots.occupied.matches(occupied)
    {
        return Ok(false);
    }
    match condition.items.as_slice() {
        [] => Ok(true),
        [predicate] => {
            if !changed_nonempty {
                return Ok(false);
            }
            matches_item_predicate(predicate, changed, tags, resolver)
                .map_err(|error| format!("inventory_changed item predicate: {error:?}"))
        }
        predicates => {
            let mut remaining = predicates.iter().collect::<Vec<_>>();
            for stack in inventory {
                if stack.count <= 0
                    || stack.item_id.is_none()
                    || stack.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_AIR_ID)
                {
                    continue;
                }
                let mut next = Vec::new();
                for predicate in remaining {
                    if !matches_item_predicate(predicate, stack, tags, resolver)
                        .map_err(|error| format!("inventory_changed item predicate: {error:?}"))?
                    {
                        next.push(predicate);
                    }
                }
                remaining = next;
                if remaining.is_empty() {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests;
