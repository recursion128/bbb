use super::ServerWorld;
use crate::entity::merchant::{generated_offers_stay_empty, is_abstract_villager};
use crate::entity::{Entity, EntityData, MerchantItemCost, MerchantOffer, MerchantOffers};
use crate::loot_runtime::{
    param_sets, LootBlockEntityDataAt, LootBlockStateAt, LootEntities, LootEntity, LootEvaluator,
    LootOrigin, LootParams, LootRuntimeReason, LootWeather, StoredLootEntity,
};
use crate::random::GameRandom;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_datapack::trades::{TradeCost, TradeHolderSet, VillagerTrade};
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::save::{SaveError, SaveResult};

mod items;
use super::environment_attributes::ScalarWeather;
use super::level_blocks::LootChunks;
use super::loot_context::entity_scoreboard::{LootEntityLevels, LootEntityScoreboard};
use super::simulation_level::mob_death_loot::LevelLootEnvironment;
use crate::entity::store::EntityBodyReadView;
use items::TradeItemEnvironment;
use std::cell::RefCell;

type TradeResult<T> = Result<T, String>;
const MAX_TRADE_DRAWS: usize = 65_536;
/// `VillagerData.MAX_VILLAGER_LEVEL` (`VillagerData.java:16`).
const MAX_VILLAGER_LEVEL: i32 = 5;

impl ServerWorld {
    pub fn initialize_merchant_offers(
        &mut self,
        dimension: &Java26LevelKey,
        entity_id: i32,
    ) -> TradeResult<()> {
        let id = self.level_id(dimension).ok_or("merchant level is absent")?;
        self.initialize_merchant_offers_id(&id, entity_id)
    }

    /// `AbstractVillager.getOffers` (`AbstractVillager.java:112-123`): a
    /// missing list is generated once from `updateTrades`.
    pub(super) fn initialize_merchant_offers_id(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> TradeResult<()> {
        let initialized = self
            .levels
            .get(dimension)
            .ok_or("merchant level is absent")?
            .entities
            .get(entity_id)
            .ok_or("merchant entity is absent")?
            .merchant_offers()
            .is_some();
        if initialized {
            return Ok(());
        }
        let offers = self.generate_trade_sets(dimension, entity_id)?;
        if !self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(entity_id)
            .expect("merchant retains its entity")
            .initialize_merchant_offers(MerchantOffers::new(offers))
        {
            return Err("merchant initialization state changed".into());
        }
        Ok(())
    }

    /// `Villager.increaseMerchantCareer` (`Villager.java:696-699`): the next
    /// level's trade set is added to the offers the villager already has.
    pub(super) fn increase_villager_career(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> TradeResult<()> {
        self.initialize_merchant_offers_id(dimension, entity_id)?;
        let entity = self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(entity_id)
            .ok_or("merchant entity is absent")?;
        let level = match entity.data() {
            EntityData::GeneralLiving(data) => {
                data.villager_profession_and_level()
                    .ok_or("entity is not a villager")?
                    .1
            }
            _ => return Err("entity is not a villager".into()),
        };
        crate::entity::general_living::set_villager_level(entity, level + 1);
        let added = self.generate_trade_sets(dimension, entity_id)?;
        if let Some(offers) = self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(entity_id)
            .and_then(crate::entity::Entity::merchant_offers_mut)
        {
            offers.extend(added);
        }
        Ok(())
    }

    /// `updateTrades` (`WanderingTrader.java:130-135`, `Villager.java:802-812`).
    fn generate_trade_sets(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> TradeResult<Vec<MerchantOffer>> {
        let resolver = self.item_stack_resolver();
        let ServerWorld {
            cold,
            levels,
            maps,
            datapack_tables,
            world_clocks,
            registries,
            scoreboard,
            ..
        } = self;
        let super::WorldCold {
            config,
            seed,
            generate_structures,
            command_storage,
            dimension_sky_lights,
            level_dimension_types,
            loot_item_tags,
            ..
        } = &mut **cold;
        let enchantment_registry = &datapack_tables.enchantment_registry;
        let startup = config
            .datapacks
            .as_deref()
            .ok_or("merchant generation needs loaded datapacks")?;
        let (level, other_levels) = levels
            .split_mut(dimension)
            .ok_or("merchant level is absent")?;
        let super::ServerLevel {
            dimension: level_key,
            chunks,
            entities,
            random,
            weather,
            sky_darkening,
            ..
        } = level;
        let entity = entities.get(entity_id).ok_or("merchant entity is absent")?;
        let EntityData::GeneralLiving(data) = entity.data() else {
            return Err("entity is not a merchant".into());
        };
        let sets: Vec<String> =
            if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
                ["buying", "uncommon", "common"]
                    .into_iter()
                    .map(|suffix| format!("minecraft:wandering_trader/{suffix}"))
                    .collect()
            } else {
                let (profession, level) = data
                    .villager_profession_and_level()
                    .ok_or("entity is not a merchant")?;
                villager_trade_set(profession, level).into_iter().collect()
            };
        let environment = TradeItemEnvironment {
            startup,
            tables: datapack_tables,
            registry_tags: chunks.registry_tags_shared(),
            resolver: &resolver,
            enchantments: enchantment_registry,
        };
        let bodies = EntityBodyReadView::new(entities, None, None);
        let chunks = RefCell::new(chunks);
        let maps = RefCell::new(maps);
        let level_environment = LevelLootEnvironment::for_world_predicates(
            LootChunks::Chunks(&chunks),
            &resolver,
            Some(startup),
            enchantment_registry,
            Some(crate::entity::block_drops::RecipeDropFacts {
                recipes: &startup.recipes,
                recipe_index: &startup.recipe_index,
                item_tags: loot_item_tags,
            }),
            world_clocks,
            command_storage,
            registries,
            *seed,
            sky_darkening.map(i32::from),
            dimension_sky_lights.loot_environment_context(
                level_dimension_types.get(dimension).map(String::as_str),
                registries,
                world_clocks,
                ScalarWeather::from_level(weather),
            ),
            LootEntityScoreboard {
                scoreboard: scoreboard.board(),
                names: other_levels.names(),
                dimension: *dimension,
                bodies: Some(bodies),
                other_levels: LootEntityLevels::Other(other_levels),
            },
        )
        .with_maps(&maps, *seed, level_key.as_str(), *generate_structures);
        let position = entity.position();
        let origin = LootOrigin {
            x: position.x,
            y: position.y,
            z: position.z,
        };
        let loaded_block_state = |pos| {
            super::level_blocks::LoadedBlocks {
                chunks: &chunks.borrow(),
            }
            .loaded_block_state(pos)
        };
        let block_entity_data = |pos| {
            chunks
                .borrow()
                .block_entity_loot_data(pos, Some(&resolver))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        };
        let params = LootParams {
            dimension: Some(level_key.as_str()),
            level: Some(&loaded_block_state as &LootBlockStateAt<'_>),
            block_entity_data: Some(&block_entity_data as &LootBlockEntityDataAt<'_>),
            origin: Some(origin),
            entities: LootEntities {
                this_entity: LootEntity::Stored(StoredLootEntity {
                    entity,
                    bodies: Some(bodies),
                    raid_at_position: None,
                }),
                ..Default::default()
            },
            weather: Some(LootWeather {
                raining: weather.is_raining(),
                thundering: weather.is_thundering(),
            }),
            additional_cost_component_allowed: true,
            item_environment: Some(&level_environment),
            ..LootParams::new(param_sets::VILLAGER_TRADE)
        };
        let mut offers = Vec::new();
        for id in &sets {
            add_offers_from_trade_set(&environment, id, &params, random, &mut offers)?;
        }
        Ok(offers)
    }

    // AbstractVillager.java:112-123, 172-179: materialize offers across an entity
    // and its recursive passenger tree before serialization.
    pub(in crate::world) fn materialize_merchant_offers_in_trees(
        &mut self,
        dimension: &LevelId,
        roots: impl IntoIterator<Item = i32>,
    ) -> TradeResult<()> {
        let pending = {
            let Some(level) = self.levels.get(dimension) else {
                return Ok(());
            };
            let mut stack: Vec<i32> = roots.into_iter().collect();
            let mut visited = rustc_hash::FxHashSet::default();
            let mut pending = Vec::new();
            while let Some(id) = stack.pop() {
                if !visited.insert(id) {
                    continue;
                }
                if level
                    .entities
                    .get(id)
                    .is_some_and(needs_generated_merchant_offers)
                {
                    pending.push(id);
                }
                for &passenger in level.entities.passengers(id) {
                    stack.push(passenger);
                }
            }
            pending
        };
        for id in pending {
            self.initialize_merchant_offers_id(dimension, id)?;
        }
        Ok(())
    }

    pub(in crate::world) fn materialize_merchant_offers_in_tree(
        &mut self,
        dimension: &LevelId,
        root_id: i32,
    ) -> TradeResult<()> {
        self.materialize_merchant_offers_in_trees(dimension, [root_id])
    }

    // AbstractVillager.java:112-123, 172-179.
    pub(crate) fn materialize_merchant_command_data(
        &mut self,
        level: &Java26LevelKey,
        entity_id: i32,
    ) -> Result<(), String> {
        let Some(dimension) = self.level_id(level) else {
            return Ok(());
        };
        self.materialize_merchant_offers_in_tree(&dimension, entity_id)
    }

    pub(super) fn materialize_saved_merchant_offers(&mut self) -> SaveResult<()> {
        let pending: Vec<_> = self
            .levels
            .iter()
            .flat_map(|(key, level)| {
                level
                    .entities
                    .bodies()
                    .filter(|&entity| needs_generated_merchant_offers(entity))
                    .map(move |entity| (key, entity.id()))
            })
            .collect();
        // AbstractVillager.java:172-182. Run before entity and RootVehicle snapshots.
        for (key, id) in pending {
            self.initialize_merchant_offers_id(&key, id)
                .map_err(|detail| SaveError::InvalidNbt {
                    path: None,
                    detail: format!("merchant {id}: {detail}"),
                })?;
        }
        Ok(())
    }
}

fn needs_generated_merchant_offers(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.merchant_offers().is_none()
        && is_abstract_villager(entity.entity_type_id())
        && !generated_offers_stay_empty(
            entity.entity_type_id(),
            data.villager_profession_and_level(),
        )
}

/// `VillagerProfession.getTrades` (`VillagerProfession.java:299-301`): every
/// trading profession maps levels 1-5 to `<profession>/level_<n>`
/// (`TradeSets.java:18-82`).
fn villager_trade_set(profession: &str, level: i32) -> Option<String> {
    let path = profession.strip_prefix("minecraft:")?;
    (!matches!(path, "none" | "nitwit") && (1..=MAX_VILLAGER_LEVEL).contains(&level))
        .then(|| format!("minecraft:{path}/level_{level}"))
}

fn add_offers_from_trade_set(
    environment: &TradeItemEnvironment<'_>,
    id: &str,
    params: &LootParams<'_>,
    random: &GameRandom,
    offers: &mut Vec<MerchantOffer>,
) -> TradeResult<()> {
    let Some(set) = environment.tables.trade_set(environment.startup, id)? else {
        return Ok(());
    };
    let mut candidates = holders(environment, &set.trades, "minecraft:villager_trade")?
        .into_iter()
        .map(|candidate| {
            let trade = environment
                .startup
                .trades
                .get(&candidate)
                .ok_or_else(|| format!("{id}: unknown trade {candidate}"))?;
            Ok((candidate, trade))
        })
        .collect::<TradeResult<Vec<_>>>()?;
    let mut evaluator = LootEvaluator::new(&environment.startup.loot, id, params, random)
        .map_err(|error| error.to_string())?;
    let count = evaluator
        .number_int(&set.amount, "amount")
        .map_err(|error| error.to_string())?
        .max(0) as usize;
    let mut found = 0;
    let mut draws = 0;
    // AbstractVillager.java:257-290. Failed offers still consume a no-duplicate candidate.
    while found < count && !candidates.is_empty() {
        if draws == MAX_TRADE_DRAWS {
            return Err(format!(
                "{id}: trade selection exceeded {MAX_TRADE_DRAWS} draws"
            ));
        }
        draws += 1;
        let bound = i32::try_from(candidates.len())
            .map_err(|_| format!("{id}: too many trade candidates"))?;
        let index = evaluator.random_int(bound) as usize;
        let (candidate, trade) = &candidates[index];
        let offer = generate_offer(environment, &mut evaluator, trade)
            .map_err(|error| format!("{candidate}: {error}"))?;
        if !set.allow_duplicates {
            candidates.remove(index);
        }
        if let Some(offer) = offer {
            offers.push(offer);
            found += 1;
        }
    }
    Ok(())
}

fn generate_offer(
    environment: &TradeItemEnvironment<'_>,
    evaluator: &mut LootEvaluator<'_>,
    trade: &VillagerTrade,
) -> TradeResult<Option<MerchantOffer>> {
    // VillagerTrade.java:162-205.
    if let Some(predicate) = &trade.merchant_predicate {
        if !evaluator
            .condition(predicate, "merchant_predicate")
            .map_err(|error| error.to_string())?
        {
            return Ok(None);
        }
    }
    let mut result = environment.template(&trade.gives)?;
    for (index, function) in trade.given_item_modifiers.iter().enumerate() {
        result = evaluator
            .function_runtime(function, result, &format!("given_item_modifiers[{index}]"))
            .map_err(|error| error.to_string())?;
        if result.is_empty() {
            return Ok(None);
        }
    }
    let mut additional = result.additional_trade_cost().unwrap_or(0);
    result = result.with_additional_trade_cost(None);
    if let Some(set) = &trade.double_trade_price_enchantments {
        let doubled = holders(environment, set, "minecraft:enchantment")?;
        if result
            .summary()
            .component_patch
            .stored_enchantments
            .iter()
            .any(|entry| {
                usize::try_from(entry.holder_id)
                    .ok()
                    .and_then(|index| environment.enchantments.get(index))
                    .is_some_and(|id| doubled.contains(id))
            })
        {
            additional = additional.wrapping_mul(2);
        }
    }
    let Some(wants) = item_cost(environment, evaluator, &trade.wants, additional)? else {
        return Ok(None);
    };
    let additional_wants = match &trade.additional_wants {
        Some(cost) => match item_cost(environment, evaluator, cost, 0)? {
            Some(cost) => Some(cost),
            None => return Ok(None),
        },
        None => None,
    };
    let max_uses = evaluator
        .number_int(&trade.max_uses, "max_uses")
        .map_err(|error| error.to_string())?
        .max(1);
    let xp = evaluator
        .number_int(&trade.xp, "xp")
        .map_err(|error| error.to_string())?
        .max(0);
    let discount = evaluator
        .number_float(&trade.reputation_discount, "reputation_discount")
        .map_err(|error| error.to_string())?
        .max(0.0);
    Ok(Some(MerchantOffer::new(
        wants,
        additional_wants,
        result,
        max_uses,
        xp,
        discount,
    )))
}

fn item_cost(
    environment: &TradeItemEnvironment<'_>,
    evaluator: &mut LootEvaluator<'_>,
    cost: &TradeCost,
    additional: i32,
) -> TradeResult<Option<MerchantItemCost>> {
    use bbb_registry_codec::PersistentItemStackHolderResolver as _;
    let content_registry = environment.resolver.content_registry();
    let item_id = vanilla_item_type_for_resource_id(&cost.item_id)
        .map(|item| item.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(&cost.item_id)
                .map(|item| item.id)
        })
        .ok_or_else(|| format!("unknown cost item {}", cost.item_id))?;
    let prototype = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    };
    // TradeCost.java:36-38.
    let count = evaluator
        .number_int(&cost.count, "cost.count")
        .map_err(|error| error.to_string())?
        .wrapping_add(additional)
        .clamp(
            0,
            bbb_server_types::item_stack::rules::max_stack_size_with_registry(
                &prototype,
                content_registry,
            )
            .map_err(|error| error.to_string())?,
        );
    if count == 0 {
        return Ok(None);
    }
    let components = environment.cost_components(&cost.components)?;
    MerchantItemCost::new(cost.item_id.clone(), count, components)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn holders(
    environment: &TradeItemEnvironment<'_>,
    set: &TradeHolderSet,
    registry: &str,
) -> TradeResult<Vec<String>> {
    match set {
        TradeHolderSet::Items(items) => Ok(items.clone()),
        TradeHolderSet::Tag(tag) => Ok(environment
            .registry_tags
            .as_ref()
            .map(|tags| {
                tags.members(registry, tag)
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default()),
    }
}

#[cfg(test)]
pub(super) mod tests;
