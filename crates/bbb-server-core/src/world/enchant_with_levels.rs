//! Shared enchantment selection and Native item writes for loot and trades.

use crate::loot_runtime::native_enchantments::upgrade_enchantment_level;
use crate::loot_runtime::{
    LootItemEnvironment, LootRandomEnchantment, LootRuntimeReason, LootStack,
};
use crate::random::GameRandom;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::RegistryJsonValue as Value;
use bbb_server_datapack::loot::HolderSet;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::spawner_minecarts::{select_spawner_enchantments, SpawnEnchantmentProviderEnvironment};

pub(in crate::world) trait EnchantWithLevelsItemWorld:
    SpawnEnchantmentProviderEnvironment
{
    // EnchantmentHelper.java:529-534 selects registry elements when options are absent.
    fn enchantments(&self) -> Option<&[String]>;
}

/// `EnchantmentHelper.enchantItem` (`EnchantmentHelper.java:537-548`) for the
/// cost `EnchantWithLevelsFunction.run` sampled (`:77`): `selectEnchantment`
/// over `options`, the book conversion, and one `ItemStack.enchant` per
/// selected instance.
pub(in crate::world) fn enchant_item(
    environment: &(impl EnchantWithLevelsItemWorld + LootItemEnvironment),
    stack: LootStack,
    levels: i32,
    options: Option<&HolderSet>,
    random: &GameRandom,
) -> Result<LootStack, LootRuntimeReason> {
    let stack = RuntimeItemStack::from_summary(stack);
    let all;
    let options = match options {
        Some(options) => options,
        None => {
            let enchantments = environment
                .enchantments()
                .ok_or_else(|| refusal("the level lists no minecraft:enchantment registry"))?;
            all = HolderSet::Ids(enchantments.to_vec());
            &all
        }
    };
    let selected = if stack.is_empty() {
        Vec::new()
    } else {
        select_spawner_enchantments(environment, &stack, options, levels, random)
            .ok_or_else(|| refusal("unavailable enchantment selection inputs"))?
            .into_iter()
            .map(|entry| (entry.resource_id, entry.level))
            .collect()
    };
    write_selected_enchantments(environment, stack, selected).map(RuntimeItemStack::into_summary)
}

fn write_selected_enchantments(
    environment: &impl LootItemEnvironment,
    mut stack: RuntimeItemStack,
    selected: Vec<(String, i32)>,
) -> Result<RuntimeItemStack, LootRuntimeReason> {
    // EnchantmentHelper.java:537-547: even an empty selection converts a book to a fresh stack.
    if stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID) {
        stack = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        });
    }
    // EnchantmentHelper.java:55-65: selection precedes the absent-component no-op.
    if selected.is_empty() {
        return Ok(stack);
    }
    let Some(values) = stack
        .enchantments(environment.item_resolver())
        .map_err(|error| refusal(error.to_string()))?
    else {
        return Ok(stack);
    };
    let mut fields = values.into_owned();
    for (id, level) in selected {
        let holder_id = environment
            .item_resolver()
            .holder_id("minecraft:enchantment", &id)
            .ok_or_else(|| refusal(format!("unknown enchantment {id}")))?;
        // ItemStack.java:995-997 upgrades selected keys while preserving the rest of the held map.
        upgrade_enchantment_level(&mut fields, holder_id, level);
    }
    stack
        .with_enchantments(fields, environment.item_resolver())
        .map_err(|error| refusal(error.to_string()))
}

pub(in crate::world) fn random_enchantment_candidates(
    environment: &impl EnchantWithLevelsItemWorld,
    stack: &ItemStackSummary,
    options: Option<&HolderSet>,
    only_compatible: bool,
) -> Result<Vec<LootRandomEnchantment>, LootRuntimeReason> {
    let ids = match options {
        Some(options) => {
            super::spawner_minecarts::spawner_enchantment_holders(environment, options)
        }
        None => environment
            .enchantments()
            .ok_or_else(|| refusal("unavailable enchantment registry"))?
            .to_vec(),
    };
    let datapacks = environment
        .datapacks()
        .ok_or_else(|| refusal("unavailable enchantment definitions"))?;
    let item = stack.item_id.and_then(vanilla_item_type);
    let check =
        only_compatible && stack.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID);
    let mut result = Vec::new();
    for id in ids {
        let definition = datapacks
            .registries
            .element("minecraft:enchantment", &id)
            .and_then(|element| element.json())
            .ok_or_else(|| refusal("unavailable enchantment definition"))?;
        // Enchantment.java:186-187: canEnchant checks supported_items, not primary_items.
        if check
            && !item.is_some_and(|item| {
                definition.get("supported_items").is_some_and(|set| {
                    super::spawner_minecarts::json_holder_set_contains(
                        environment,
                        "minecraft:item",
                        set,
                        item.resource_id,
                    )
                })
            })
        {
            continue;
        }
        result.push(LootRandomEnchantment {
            id,
            min_level: 1,
            max_level: definition
                .get("max_level")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| refusal("unavailable enchantment max level"))?,
        });
    }
    Ok(result)
}

pub(in crate::world) fn with_random_enchantment(
    environment: &impl LootItemEnvironment,
    stack: LootStack,
    enchantment: &LootRandomEnchantment,
    level: i32,
) -> Result<LootStack, LootRuntimeReason> {
    let stack = RuntimeItemStack::from_summary(stack);
    write_selected_enchantments(environment, stack, vec![(enchantment.id.clone(), level)])
        .map(RuntimeItemStack::into_summary)
}

fn refusal(detail: impl Into<String>) -> LootRuntimeReason {
    LootRuntimeReason::UnsupportedField {
        owner: "item modifier",
        detail: detail.into(),
    }
}
