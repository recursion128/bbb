//! Block loot evaluation and actual item spawn values.

use std::collections::BTreeMap;

use crate::random::GameRandom;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
use bbb_protocol::entity_types::{vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_ITEM_ID};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};

pub use crate::enchantment_facts::{builtin_enchantment_documents, builtin_enchantment_registry};
use crate::loot_runtime::{
    BlockDropParams, LootBlockEntity, LootBlockEntityDataAt, LootBlockStateAt, LootEntity,
    LootItemEnvironment, LootStack, LootTool,
};
use bbb_server_datapack::loot::LootRegistries;
use bbb_server_datapack::tag_loader::LoadedRegistryTags;

/// The bound `Mth.nextDouble(random, -0.25, 0.25)` is called with for each of
/// the three coordinates (`Block.java:420-422`).
const POP_RESOURCE_SPREAD: f64 = 0.25;

/// The half-block `popResource` centres the drop on (`Block.java:420-422`).
const BLOCK_CENTRE: f64 = 0.5;

pub struct BlockDropTables<'a> {
    /// `ReloadableServerRegistries.Holder`'s loot registry. `None` is a world
    /// whose runtime wired no datapack generation, which is not a table with
    /// no entry -- it is no registry at all -- and is reported rather than
    /// treated as an empty drop.
    pub tables: Option<&'a LootRegistries>,
    /// An unavailable recipe publication cannot answer ingredient membership.
    pub recipes: Option<RecipeDropFacts<'a>>,
    /// The generation's tag authority; `None` leaves membership unavailable.
    pub damage_type_tags: Option<Option<&'a LoadedRegistryTags>>,
}

pub struct BlockDropSource<'a> {
    pub tables: Option<&'a LootRegistries>,
}

impl BlockDropTables<'_> {
    fn source(&mut self) -> BlockDropSource<'_> {
        BlockDropSource {
            tables: self.tables,
        }
    }
}

/// The server-wide recipe surface one loot function reads.
///
/// `SmeltItemFunction.run` asks `context.getLevel().recipeAccess()`, which is
/// the same recipe manager every furnace reads; this server keeps it on the
/// startup generation rather than on a level, so the two halves travel
/// together here.
#[derive(Clone, Copy)]
pub struct RecipeDropFacts<'a> {
    pub recipes: &'a bbb_server_datapack::recipe::RecipeSet,
    pub recipe_index: &'a bbb_server_datapack::recipe::RecipeIndex,
    pub item_tags: &'a bbb_server_datapack::recipe::ItemTagIndex,
}

impl std::fmt::Debug for RecipeDropFacts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecipeDropFacts")
            .field("recipes", &self.recipes.recipes.len())
            .field("item_tags", &self.item_tags.len())
            .finish()
    }
}

/// One `getDrops` result waiting for `Block.popResource` to place it.
///
/// The stacks are already drawn: the loot generator has advanced and cannot be
/// asked again, so this is a value rather than a promise.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPopResource {
    pub pos: BlockPos,
    pub stacks: Vec<LootStack>,
}

/// One `level.addFreshEntity(new ItemEntity(..))` waiting for the entity store.
///
/// The store is not reachable from a ticking level -- `ItemEntity.tick` reads
/// and writes its neighbours through the same store -- so a spawn is recorded
/// here and performed by the driver, which is the same buffering the scheduled
/// ticks and the block events already go through.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingItemSpawn {
    /// The `ItemEntity` constructor's `x`, `y`, `z` (`ItemEntity.java:60-65`).
    pub position: Vec3d,
    pub item: bbb_server_types::item_stack::RuntimeItemStack,
    /// `Entity.spawnAtLocation`'s `setDefaultPickUpDelay` (`Entity.java:2137`).
    /// `false` is the bare `new ItemEntity(level, x, y, z, stack)`
    /// (`ItemEntity.java:60-65`), whose `pickupDelay` stays at its zero
    /// initialiser -- the constructor `LivingEntity.createWitherRose` uses
    /// (`LivingEntity.java:1472-1474`).
    pub default_pickup_delay: bool,
}

/// What `Block.getDrops` reads besides the state: the blocks a
/// `location_check` samples, `LootContextParams.BLOCK_ENTITY`, and the item
/// environment a component-writing loot function applies through.
#[derive(Clone, Copy, Default)]
pub struct BlockDropView<'a> {
    pub dimension: Option<&'a str>,
    pub level: Option<&'a LootBlockStateAt<'a>>,
    pub block_entity_data: Option<&'a LootBlockEntityDataAt<'a>>,
    pub block_entity: Option<LootBlockEntity<'a>>,
    pub item_environment: Option<&'a dyn LootItemEnvironment>,
}

/// `Block.dropResources(state, level, pos)`'s first half
/// (`Block.java:391`): `getDrops(state, serverLevel, pos, null)`
/// (`Block.java:365-371`), which fills `TOOL` with `ItemStack.EMPTY` and never
/// fills `THIS_ENTITY`.
///
/// The stacks are queued rather than placed because `popResource` needs
/// `Level.random`; see this module's own documentation.
pub fn drop_resources(
    tables: &mut BlockDropTables<'_>,
    level_random: &GameRandom,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    requests: &mut Vec<PendingPopResource>,
) {
    drop_resources_with_tool(
        tables,
        level_random,
        view,
        pos,
        block_state_id,
        &LootTool::empty(),
        LootEntity::Absent,
        requests,
    );
}

/// `Block.dropResources(state, level, pos, blockEntity, breaker, tool)`'s first
/// half (`Block.java:412`): `getDrops(state, serverLevel, pos, blockEntity,
/// breaker, tool)` (`Block.java:373-386`), whose
/// `withOptionalParameter(BLOCK_ENTITY, ..)` is [`BlockDropView::block_entity`].
#[allow(clippy::too_many_arguments)]
pub fn drop_resources_with_tool(
    tables: &mut BlockDropTables<'_>,
    level_random: &GameRandom,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    tool: &LootTool,
    breaker: LootEntity<'_>,
    requests: &mut Vec<PendingPopResource>,
) {
    drop_resources_with_source(
        tables.source(),
        level_random,
        view,
        pos,
        block_state_id,
        tool,
        breaker,
        requests,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn drop_resources_with_source(
    tables: BlockDropSource<'_>,
    level_random: &GameRandom,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    tool: &LootTool,
    breaker: LootEntity<'_>,
    requests: &mut Vec<PendingPopResource>,
) {
    // `if (level instanceof ServerLevel serverLevel)` (`Block.java:390`,
    // `:411`). This crate has no client level, so the test is always true.
    let Some(loot) = tables.tables else {
        return;
    };
    let Some(block_state_id) = loot_block_state(block_state_id, &view) else {
        return;
    };
    let params = BlockDropParams {
        block_state_id,
        pos,
        dimension: view.dimension,
        level: view.level,
        block_entity_data: view.block_entity_data,
        tool,
        breaker,
        explosion_radius: None,
        block_entity: view.block_entity,
        item_environment: view.item_environment,
    };
    if let Ok(stacks) = crate::loot_runtime::block_drops(loot, &params, level_random) {
        requests.push(PendingPopResource { pos, stacks })
    }
}

/// `MovingPistonBlock.getDrops` (`MovingPistonBlock.java:95-98`) answers with
/// the moved state's drops, and nothing without its block entity.
fn loot_block_state(block_state_id: i32, view: &BlockDropView<'_>) -> Option<i32> {
    if bbb_server_types::block_queries::block_type_of(block_state_id)
        != Some(bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
    {
        return Some(block_state_id);
    }
    view.block_entity
        .as_ref()?
        .block_entity
        .piston_snapshot()
        .map(|moving| moving.moved_state)
}

/// Collects one block before `ServerExplosion.interactWithBlocks` starts its
/// `popResource` loop (`ServerExplosion.java:214-225`), with the block entity
/// `BlockBehaviour.onExplosionHit` read before the removal
/// (`BlockBehaviour.java:184`).
///
/// `publication` is what `StackCollector.tryMerge`'s identity test
/// (`ServerExplosion.java:316-329`) reads the two stacks' facts through, so two
/// holder-named stacks compare as the same stack the way vanilla's
/// `ItemStack.isSameItemSameComponents` does.
#[allow(clippy::too_many_arguments)]
pub fn collect_explosion_resources(
    tables: &mut BlockDropTables<'_>,
    level_random: &GameRandom,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    explosion_radius: Option<f32>,
    breaker: LootEntity<'_>,
    collectors: &mut Vec<PendingPopResource>,
) {
    collect_explosion_resources_with_source(
        tables.source(),
        level_random,
        view,
        pos,
        block_state_id,
        explosion_radius,
        breaker,
        collectors,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn collect_explosion_resources_with_source(
    tables: BlockDropSource<'_>,
    level_random: &GameRandom,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    explosion_radius: Option<f32>,
    breaker: LootEntity<'_>,
    collectors: &mut Vec<PendingPopResource>,
) {
    let Some(loot) = tables.tables else {
        return;
    };
    let Some(block_state_id) = loot_block_state(block_state_id, &view) else {
        return;
    };
    let tool = LootTool::empty();
    let params = BlockDropParams {
        block_state_id,
        pos,
        dimension: view.dimension,
        level: view.level,
        block_entity_data: view.block_entity_data,
        tool: &tool,
        breaker,
        explosion_radius,
        block_entity: view.block_entity,
        item_environment: view.item_environment,
    };
    if let Ok(stacks) = crate::loot_runtime::block_drops(loot, &params, level_random) {
        for stack in stacks {
            let _ = append_explosion_stack(collectors, pos, stack);
        }
    }
}

fn append_explosion_stack(
    collectors: &mut Vec<PendingPopResource>,
    pos: BlockPos,
    input: LootStack,
) -> bbb_server_types::Result<()> {
    let mut input =
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(input).into_summary();
    let limit = crate::inventory::max_stack_size(&input)?;
    for collector in collectors.iter_mut() {
        let Some(to) = collector.stacks.first_mut() else {
            continue;
        };
        if !crate::inventory::is_same_item_same_components(to, &input)
            || to.count + input.count > limit
        {
            continue;
        }
        let transferred =
            (crate::inventory::max_stack_size(to)?.min(16) - to.count).min(input.count);
        to.count += transferred;
        input.count -= transferred;
        if bbb_registry_codec::item_stack_summary_is_empty(&input) {
            break;
        }
    }
    if !bbb_registry_codec::item_stack_summary_is_empty(&input) {
        collectors.push(PendingPopResource {
            pos,
            stacks: vec![input],
        });
    }
    Ok(())
}

/// `Block.dropFromBlockInteractLootTable` (`Block.java:266-285`) for one named
/// table, then the same `popResource` queue [`drop_resources_with_tool`] uses.
/// No dynamic drop is registered on this path.
#[allow(clippy::too_many_arguments)]
pub fn drop_named_table(
    tables: &mut BlockDropTables<'_>,
    level_random: &GameRandom,
    table_id: &str,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    tool: &LootTool,
    breaker: LootEntity<'_>,
    requests: &mut Vec<PendingPopResource>,
) {
    drop_named_table_with_source(
        tables.source(),
        level_random,
        table_id,
        view,
        pos,
        block_state_id,
        tool,
        breaker,
        requests,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn drop_named_table_with_source(
    tables: BlockDropSource<'_>,
    level_random: &GameRandom,
    table_id: &str,
    view: BlockDropView<'_>,
    pos: BlockPos,
    block_state_id: i32,
    tool: &LootTool,
    breaker: LootEntity<'_>,
    requests: &mut Vec<PendingPopResource>,
) {
    let Some(loot) = tables.tables else {
        return;
    };
    let params = BlockDropParams {
        block_state_id,
        pos,
        dimension: view.dimension,
        level: view.level,
        block_entity_data: view.block_entity_data,
        tool,
        breaker,
        explosion_radius: None,
        block_entity: view.block_entity,
        item_environment: view.item_environment,
    };
    if let Ok(stacks) =
        crate::loot_runtime::named_table_drops(loot, table_id, &params, level_random)
    {
        requests.push(PendingPopResource { pos, stacks })
    }
}

/// The `LootContextParams.TOOL` value one held stack stands for.
///
/// `enchantment_registry` is the `minecraft:enchantment` registry in the order
/// its ids were handed out, because that order is what the `holder_id` on a
/// decoded `minecraft:enchantments` component means. It is a datapack registry
/// (`RegistryDataLoader.WORLDGEN_REGISTRIES`), so the order is the loaded
/// generation's and never the `Enchantments.java` bootstrap order the generated
/// `bbb_protocol::enchantment_types` catalog carries — the same trap
/// [`bbb_server_datapack::damage::damage_type_registry`] documents for damage types.
///
/// A holder id the registry has no row for contributes nothing: it is an
/// enchantment this server cannot name, and naming it wrong would silently
/// answer a `match_tool` or an `apply_bonus` with the wrong level. That
/// absence is checked by the caller rather than here, because a stack is
/// projected before it is known whether any table reads it.
#[must_use]
pub fn loot_tool_for_stack(stack: &ItemStackSummary, enchantment_registry: &[String]) -> LootTool {
    let patch = &stack.component_patch;
    // A patch that *removed* `minecraft:enchantments` leaves the stack with no
    // component at all, which `getOrDefault` reads as `ItemEnchantments.EMPTY`
    // (`EnchantmentHelper.java:50-53`).
    let removed = patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    let mut enchantments = BTreeMap::new();
    if !removed {
        for entry in &patch.enchantments {
            let Ok(index) = usize::try_from(entry.holder_id) else {
                continue;
            };
            let Some(resource_id) = enchantment_registry.get(index) else {
                continue;
            };
            // `ItemEnchantments`' constructor refuses a level outside `0..=255`
            // (`ItemEnchantments.java:46-51`), so a component that carries one
            // is not a component any vanilla codec built.
            if !(0..=255).contains(&entry.level) {
                continue;
            }
            enchantments.insert(resource_id.clone(), entry.level);
        }
    }
    LootTool {
        item_id: stack.item_id,
        count: stack.count,
        enchantments,
        stack: bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack.clone()),
    }
}

pub fn loot_tool_for_runtime_stack(
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
    enchantment_registry: &[String],
) -> LootTool {
    if stack.is_empty() {
        return LootTool::empty();
    }
    loot_tool_for_stack(stack.summary(), enchantment_registry)
}

/// The holder ids in a stack's `minecraft:enchantments` component that
/// `enchantment_registry` has no row for.
///
/// Empty is the healthy answer. A non-empty one is a tool whose enchantments
/// this server cannot name, which means a `match_tool` or an `apply_bonus`
/// reading them would answer as though they were absent; the caller records it
/// rather than dropping the wrong loot silently.
#[must_use]
pub fn unresolved_tool_enchantments(
    stack: &ItemStackSummary,
    enchantment_registry: &[String],
) -> Vec<i32> {
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
        .filter(|entry| {
            usize::try_from(entry.holder_id)
                .ok()
                .and_then(|index| enchantment_registry.get(index))
                .is_none()
        })
        .map(|entry| entry.holder_id)
        .collect()
}

/// `getDrops(..).forEach(stack -> popResource(level, pos, stack))`
/// (`Block.java:391`) for every request [`drop_resources`] queued, in the
/// order it queued them.
pub fn pop_resources(
    requests: &mut Vec<PendingPopResource>,
    random: &GameRandom,
    block_drops: bool,
    spawns: &mut Vec<PendingItemSpawn>,
) {
    for request in requests.drain(..) {
        for stack in &request.stacks {
            pop_resource(request.pos, stack, random, block_drops, spawns);
        }
    }
}

/// `Block.popResource(level, pos, itemStack)` (`Block.java:417-424`) followed
/// by the private `popResource` it delegates to (`Block.java:442-448`).
///
/// The order of the two halves is load-bearing for the generator: the three
/// `Mth.nextDouble` draws are made by the *public* method, before the private
/// one tests the stack and the game rule, so an empty stack and a world with
/// `block_drops` off both still advance `Level.random` by three doubles.
pub fn pop_resource(
    pos: BlockPos,
    stack: &LootStack,
    random: &GameRandom,
    block_drops: bool,
    spawns: &mut Vec<PendingItemSpawn>,
) {
    // `:418`. `EntityType.ITEM.getHeight()` is the registry row's height, which
    // the generated entity catalog already carries.
    let half_height = f64::from(item_entity_height()) / 2.0;
    // `:420-422`.
    let x = f64::from(pos.x) + BLOCK_CENTRE + next_spread(random);
    let y = f64::from(pos.y) + BLOCK_CENTRE + next_spread(random) - half_height;
    let z = f64::from(pos.z) + BLOCK_CENTRE + next_spread(random);
    // `:443`. `ItemStack.isEmpty` and `GameRules.BLOCK_DROPS`
    // (`gamerules/GameRules.java:29`) are both tested after the draws.
    if bbb_registry_codec::item_stack_summary_is_empty(stack) || !block_drops {
        return;
    }
    let position = Vec3d { x, y, z };
    add_fresh_item_entity(position, stack, spawns);
}

/// The three lines `Block.popResource` (`Block.java:444-446`) and
/// `Entity.spawnAtLocation` (`Entity.java:2136-2138`) both run: construct the
/// `ItemEntity`, give it the default pickup delay, add it to the level.
///
/// Only the projection can fail. `ItemEntity` stores its stack through
/// `ItemStack.CODEC` (`ItemEntity.java:320-322`) and the loot runtime produces
/// the network projection, so the two have to be joined here; a stack whose
/// components cannot be written back is reported rather than dropped, because
/// silently placing a *different* stack on the ground would be worse than
/// placing none.
pub fn add_fresh_item_entity(
    position: Vec3d,
    stack: &ItemStackSummary,
    spawns: &mut Vec<PendingItemSpawn>,
) {
    add_item_entity(position, stack, true, spawns);
}

/// [`add_fresh_item_entity`] without the default pickup delay, for the one
/// caller that builds a bare `ItemEntity`: `LivingEntity.createWitherRose`'s
/// rose (`LivingEntity.java:1472-1474`).
pub fn add_fresh_item_entity_without_pickup_delay(
    position: Vec3d,
    stack: &ItemStackSummary,
    spawns: &mut Vec<PendingItemSpawn>,
) {
    add_item_entity(position, stack, false, spawns);
}

fn add_item_entity(
    position: Vec3d,
    stack: &ItemStackSummary,
    default_pickup_delay: bool,
    spawns: &mut Vec<PendingItemSpawn>,
) {
    let item = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack.clone());
    if !item.is_empty() {
        spawns.push(PendingItemSpawn {
            position,
            item,
            default_pickup_delay,
        });
    }
}

/// `Containers.dropContents(level, pos, container)` (`Containers.java:11-19`)
/// for a block entity whose record is going away: the slot-aligned stacks its
/// `preRemoveSideEffects` handed back, planned and spawned with
/// `Containers.dropItemStack`'s own draws (`Containers.java:32-48`).
///
/// A slot whose stack cannot be projected still takes the three position draws
/// vanilla draws for every slot it is handed, and is not dropped rather than
/// becoming a different stack on the ground.
pub fn drop_container_contents(
    store: &mut crate::entity::EntityStore,
    position: Vec3d,
    slots: &[Option<bbb_server_types::item_stack::RuntimeItemStack>],
) -> bool {
    let planned = crate::entity::item::plan_container_drops(store, position, slots);
    crate::entity::item::spawn_planned_container_drops(store, planned)
}

/// `Mth.nextDouble(random, -0.25, 0.25)` (`Mth.java:153-155`) for the one
/// range `popResource` uses, where `min < max` so the ternary always takes the
/// second arm.
fn next_spread(random: &GameRandom) -> f64 {
    random.next_double() * (POP_RESOURCE_SPREAD * 2.0) - POP_RESOURCE_SPREAD
}

/// `EntityType.ITEM.getHeight()`, read off the generated 26.1 entity catalog
/// rather than written down.
#[must_use]
pub fn item_entity_height() -> f32 {
    let [_width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ITEM_ID)
        .expect("the generated catalog carries minecraft:item");
    height
}

#[cfg(test)]
pub mod tests;
