//! The three furnace blocks: `AbstractFurnaceBlockEntity.serverTick`
//! (`AbstractFurnaceBlockEntity.java:138-206`) driven by the live block
//! ticker, and `AbstractFurnaceBlock`'s menu
//! (`AbstractFurnaceMenu.java:38-73`).

use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_BLAST_FURNACE_ID,
    VANILLA_BLOCK_COMPARATOR_ID, VANILLA_BLOCK_FURNACE_ID, VANILLA_BLOCK_SMOKER_ID,
};
use bbb_protocol::packets::{BlockPos, ContainerSetData, ItemStackSummary};

use crate::block_behavior::support::state_is_block;
use crate::block_entity::furnace::{FurnaceTickFacts, SLOT_FUEL, SLOT_INPUT};
use crate::block_entity::BlockEntity;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::award_experience;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::inventory::{FurnaceMenuKind, MenuLayout};
use bbb_server_datapack::recipe::{
    crafting_remainder, CookingKind, FuelValues, FurnaceFuelKind, ItemTagIndex,
};
use bbb_server_types::item_stack::RuntimeItemStack;
use std::collections::BTreeSet;
use std::sync::Arc;

use crate::world::{send, ServerWorld, WorldEvent};

use super::BlockUseContext;

const LIT: &str = "lit";

/// `AbstractFurnaceBlock`'s `useWithoutItem` owner: the generated class tag
/// names exactly the furnace, smoker and blast furnace.
#[must_use]
pub(super) fn is_furnace(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::AbstractFurnaceBlock)
}

/// `AbstractFurnaceBlock.useWithoutItem` (`:31-37`), whose body is
/// `openContainer` plus `InteractionResult.SUCCESS`.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_furnace_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

/// The `MenuProvider` a spectator reaches: `BaseEntityBlock.getMenuProvider`
/// (`BaseEntityBlock.java:30-33`) answering the block entity itself, which for
/// these three is a `BaseContainerBlockEntity`.
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_furnace_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

type FurnaceRecipesUsedSnapshot = (LevelId, Vec<(String, i32)>, Vec<RuntimeItemStack>);
type FurnaceAdmissionSets = (Arc<BTreeSet<i32>>, Arc<BTreeSet<i32>>);

impl ServerWorld {
    /// `AbstractFurnaceBlock.useWithoutItem` (`AbstractFurnaceBlock.java:39-47`)
    /// over the three `openContainer` overrides
    /// (`FurnaceBlock.java:45-51`, `SmokerBlock.java:44-50`,
    /// `BlastFurnaceBlock.java:45-51`): the block entity's own menu, then the
    /// interaction statistic, both inside the `instanceof` test. The answer is
    /// `InteractionResult.SUCCESS` whatever happened (`:46`).
    pub(in crate::world) fn use_furnace_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let Some((_, menu_kind)) = furnace_menu_kinds(block_state_id) else {
            return true;
        };
        let is_furnace = self
            .has_container_block_entity(dimension, pos, |at| BlockEntity::furnace(at).is_some());
        self.open_furnace_menu(connection_id, dimension, pos, block_state_id, events);
        if is_furnace {
            self.award_opener_stat(
                connection_id,
                match menu_kind {
                    FurnaceMenuKind::Furnace => "minecraft:interact_with_furnace",
                    FurnaceMenuKind::Smoker => "minecraft:interact_with_smoker",
                    FurnaceMenuKind::BlastFurnace => "minecraft:interact_with_blast_furnace",
                },
            );
        }
        true
    }

    /// `AbstractFurnaceBlockEntity` as a `MenuProvider`: `FurnaceMenu` /
    /// `SmokerMenu` / `BlastFurnaceMenu` over the block entity's own three
    /// slots and its `ContainerData` (`AbstractFurnaceMenu.java:38-73`).
    ///
    /// The two admission sets come from the live recipe and tag generation
    /// (`:140-146`), so a menu without them is not this menu: no datapack
    /// generation, no menu.
    pub(in crate::world) fn open_furnace_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let Some((cooking_kind, menu_kind)) = furnace_menu_kinds(block_state_id) else {
            return false;
        };
        // `openContainer`'s `instanceof` test (`FurnaceBlock.java:45-51`): the
        // block entity at the position decides, not the block state.
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        if instances[0].container_slots().is_none() {
            return false;
        }
        let Some((inputs, fuels)) = self.furnace_admission_sets(cooking_kind) else {
            return false;
        };
        if !self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::Furnace { kind: menu_kind },
            furnace_menu_title(menu_kind),
            events,
        ) {
            return false;
        }
        self.levels[connection_id]
            .inventory
            .attach_furnace_admission(inputs, fuels);
        true
    }

    /// The `ContainerData` half of `AbstractContainerMenu.broadcastChanges`
    /// (`:204-207`) for an open menu whose block entity answers data slots: the
    /// values go out only when the client was told something else, which is
    /// what keeps a settled menu quiet. `full` is `sendAllDataToRemote`
    /// (`:157-178`), which resends all of them.
    pub(in crate::world) fn sync_menu_data(
        &mut self,
        connection_id: ConnectionId,
        full: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((container_id, values)) = self.open_menu_data(connection_id) else {
            return;
        };
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        for (slot, value) in values.iter().enumerate() {
            let Ok(id) = i16::try_from(slot) else {
                return;
            };
            // `ClientboundContainerSetDataPacket` carries a short; a wider
            // value is a datapack fact this server cannot state.
            let value = i16::try_from(*value).unwrap_or(0);
            let changed = if full {
                player.inventory.set_menu_data(id, value);
                true
            } else {
                player.inventory.menu_data_differs(id, value)
            };
            if changed {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::ContainerSetData(ContainerSetData {
                        container_id,
                        id,
                        value,
                    }),
                ));
            }
        }
    }

    /// `AbstractFurnaceBlockEntity.awardUsedRecipesAndPopExperience`
    /// (`AbstractFurnaceBlockEntity.java:334-343`): every recipe the block
    /// entity cooked since the last take is awarded to the taker, its
    /// experience pops where the taker stands, and the map is cleared.
    pub(in crate::world) fn furnace_award_used_recipes(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((dimension, used, ingredients)) = self.take_furnace_recipes_used(connection_id)
        else {
            return;
        };
        let Some(position) = self
            .levels
            .player(connection_id)
            .map(|player| player.position())
        else {
            return;
        };
        let resolved = self.cooking_recipes_used(&used);
        let recipe_ids: Vec<String> = resolved.iter().map(|(id, _, _)| id.clone()).collect();
        for (_, count, value) in resolved {
            self.pop_cooking_experience(dimension, position, count, value);
        }
        for recipe_id in &recipe_ids {
            self.trigger_recipe_crafted(
                connection_id,
                "minecraft:recipe_crafted",
                recipe_id,
                &ingredients,
                events,
            );
        }
        self.award_recipes(connection_id, &recipe_ids, events);
    }

    /// `AbstractFurnaceBlockEntity.getRecipesToAwardAndPopExperience`
    /// (`AbstractFurnaceBlockEntity.java:345-356`) for a `recipesUsed` map that
    /// already has no block entity left: the live cooking recipes it names, in
    /// map order, with the count each was cooked. The recipe list the method
    /// returns has no taker on the removal path, so only its experience
    /// matters here.
    pub(in crate::world) fn pop_removed_furnace_experience(
        &mut self,
        dimension: impl Into<LevelId>,
        position: bbb_protocol::packets::Vec3d,
        used: &[(String, i32)],
    ) {
        let dimension = dimension.into();
        for (_, count, value) in self.cooking_recipes_used(used) {
            self.pop_cooking_experience(dimension, position, count, value);
        }
    }

    /// `level.recipeAccess().byKey` (`:349-351`) over the cast to
    /// `AbstractCookingRecipe` (`:351`): an id the live generation no longer
    /// holds, or one that is not a cooking recipe, is skipped.
    fn cooking_recipes_used(&self, used: &[(String, i32)]) -> Vec<(String, i32, f32)> {
        let Some(datapacks) = self.datapacks() else {
            return Vec::new();
        };
        used.iter()
            .filter_map(|(id, count)| {
                let recipe = datapacks.recipes.recipe(id)?.recipe.cooking()?;
                Some((id.clone(), *count, recipe.experience))
            })
            .collect()
    }

    /// `AbstractFurnaceBlockEntity.createExperience` (`:358-366`):
    /// `Mth.floor(amount * value)`, plus one on a `Mth.frac(amount * value)`
    /// roll, then `ExperienceOrb.award(level, position, xpReward)` (`:365`).
    fn pop_cooking_experience(
        &mut self,
        dimension: impl Into<LevelId>,
        position: bbb_protocol::packets::Vec3d,
        count: i32,
        value: f32,
    ) {
        if count <= 0 || !value.is_finite() || value <= 0.0 {
            return;
        }
        let dimension = dimension.into();
        let product = count as f32 * value;
        if !product.is_finite() {
            return;
        }
        let mut reward = product.floor() as i32;
        if product.fract() != 0.0 {
            let random = &self.random;
            if random.next_float() < product.fract() {
                reward = reward.saturating_add(1);
            }
        }
        if reward <= 0 {
            return;
        }
        let level = self.level_mut_by_id(&dimension);
        let crate::world::ServerLevel {
            entities,
            random: world_random,
            ..
        } = level;
        let _ = award_experience(entities, world_random, position, reward);
    }

    /// The `recipesUsed` map of the open menu's furnace, taken out and marked
    /// changed the way `awardUsedRecipesAndPopExperience`'s last statement
    /// (`:342`) clears it.
    fn take_furnace_recipes_used(
        &mut self,
        connection_id: ConnectionId,
    ) -> Option<FurnaceRecipesUsedSnapshot> {
        let player = self.levels.player(connection_id)?;
        let [original] = player.inventory.container_instances() else {
            return None;
        };
        let original = original.clone();
        let dimension = self.player_level_id(connection_id);
        let mut body =
            original.resolve_instance(self.level_by_id(&dimension).chunks.block_entities());
        let furnace = body.furnace_mut()?;
        let used: Vec<(String, i32)> = furnace
            .recipes_used()
            .iter()
            .map(|(id, count)| (id.clone(), *count))
            .collect();
        if used.is_empty() {
            return None;
        }
        let ingredients = furnace.items();
        furnace.clear_recipes_used();
        self.with_level(dimension, |level| {
            level.publish_container_instance(&body);
            level.block_entity_set_changed(body.block_pos(), body.block_state());
        });
        Some((dimension, used, ingredients))
    }

    fn open_menu_data(&self, connection_id: ConnectionId) -> Option<(i32, Vec<i32>)> {
        let player = self.levels.player(connection_id)?;
        if player.inventory.utility_menu_kind().is_some() {
            let values = self.utility_menu_data(connection_id)?;
            return Some((player.inventory.container_id(), values));
        }
        let [original] = player.inventory.container_instances() else {
            return None;
        };
        let block_entity =
            original.resolve_instance(self.level_by_id(&player.dimension).chunks.block_entities());
        let values = if let Some(furnace) = block_entity.furnace() {
            vec![
                furnace.lit_time_remaining(),
                furnace.lit_total_time(),
                furnace.cooking_time_spent(),
                furnace.cooking_total_time(),
            ]
        } else if let Some(lectern) = block_entity.lectern() {
            // `LecternMenu` (`LecternMenu.java:22-36`): one data slot, the page.
            vec![lectern.page()]
        } else if player.inventory.is_crafter_menu() {
            let crafter = block_entity.crafter()?;
            let mut data = (0..9)
                .map(|slot| i32::from(crafter.is_slot_disabled(slot)))
                .collect::<Vec<_>>();
            data.push(crafter.triggered());
            data
        } else {
            let stand = block_entity.brewing_stand()?;
            vec![stand.brew_time(), stand.fuel()]
        };
        Some((player.inventory.container_id(), values))
    }
}

/// `FurnaceBlockEntity.DEFAULT_NAME` / `SmokerBlockEntity.DEFAULT_NAME` /
/// `BlastFurnaceBlockEntity.DEFAULT_NAME`
/// (`FurnaceBlockEntity.java:12`, `SmokerBlockEntity.java:13`,
/// `BlastFurnaceBlockEntity.java:13`), the `MenuProvider.getDisplayName` the
/// open-screen packet carries.
fn furnace_menu_title(kind: FurnaceMenuKind) -> &'static str {
    match kind {
        FurnaceMenuKind::Furnace => "container.furnace",
        FurnaceMenuKind::Smoker => "container.smoker",
        FurnaceMenuKind::BlastFurnace => "container.blast_furnace",
    }
}

impl ServerWorld {
    /// One `BlockEntityTicker.tick` for a furnace, smoker or blast furnace
    /// (`AbstractFurnaceBlockEntity.java:138-206`).
    pub(in crate::world) fn tick_furnace(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(block_state_id) = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
        else {
            return;
        };
        let Some((cooking_kind, fuel_kind)) =
            vanilla_block_state_block_type_id(block_state_id).and_then(furnace_kinds)
        else {
            return;
        };
        let Some(furnace) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::furnace)
        else {
            return;
        };
        let items = furnace.original_items();
        let input = &items[SLOT_INPUT];
        let fuel = &items[SLOT_FUEL];
        let lit_remaining = furnace.lit_time_remaining();
        let cooking_spent = furnace.cooking_time_spent();
        if lit_remaining <= 0 && cooking_spent <= 0 && (input.is_empty() || fuel.is_empty()) {
            return;
        }
        let will_stay_lit = lit_remaining > 1;
        let need_recipe = !input.is_empty() && (will_stay_lit || !fuel.is_empty());
        let need_fuel = !will_stay_lit && !input.is_empty() && !fuel.is_empty();
        let recipe_hint = need_recipe.then(|| furnace.last_recipe_id()).flatten();
        let facts = match self.furnace_tick_facts(
            cooking_kind,
            fuel_kind,
            need_recipe.then_some(input),
            recipe_hint,
            need_fuel.then_some(fuel),
        ) {
            Ok(facts) => facts,
            Err(error) => {
                tracing::warn!("furnace tick: {error}");
                return;
            }
        };
        let resolver = self.item_stack_resolver();
        let Some(furnace) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::furnace_mut)
        else {
            return;
        };
        if let Some(matched_id) = facts.recipe_id.as_deref() {
            if furnace.last_recipe_id() != Some(matched_id) {
                furnace.set_last_recipe_id(Some(matched_id.to_owned()));
            }
        }
        let tick = match furnace.server_tick(&resolver, &facts) {
            Ok(tick) => tick,
            Err(error) => {
                tracing::warn!("furnace tick: {error}");
                return;
            }
        };
        if let Some(lit) = tick.lit_changed {
            // `:197-201`: `level.setBlock(pos, state.setValue(LIT, isLit), 3)`.
            let Some(next) = with_property(block_state_id, LIT, if lit { "true" } else { "false" })
            else {
                return;
            };
            self.set_block(dimension, pos, next, SET_BLOCK_UPDATE_ALL);
        }
        if tick.changed {
            // `:203-205`.
            self.ticked_block_entity_changed(dimension, pos);
        }
    }

    /// `BlockEntity.setChanged` (`BlockEntity.java:198-203`) for a ticker:
    /// marks the chunk dirty and lets comparators re-read the container.
    /// Building a level frame is skipped when no comparator can hear it.
    pub(in crate::world) fn ticked_block_entity_changed(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) {
        let chunks = &mut self.level_mut_by_id(dimension).chunks;
        chunks.block_entities_mut().set_changed(pos);
        let comparator_near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
            (1..=2).any(|step| {
                chunks
                    .get_block_state(pos.x + dx * step, pos.y, pos.z + dz * step)
                    .is_some_and(|state| state_is_block(state, VANILLA_BLOCK_COMPARATOR_ID))
            })
        });
        if !comparator_near {
            return;
        }
        let Some(state) = chunks.get_block_state(pos.x, pos.y, pos.z) else {
            return;
        };
        self.with_level(dimension, |level| {
            level.block_entity_set_changed(pos, state)
        });
    }

    /// `getTotalCookTime` (`AbstractFurnaceBlockEntity.java:251-254`) for a
    /// furnace's input slot, or `None` when the position is not a furnace.
    pub(in crate::world) fn furnace_input_cooking_time(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        input: Option<&RuntimeItemStack>,
    ) -> bbb_server_types::Result<Option<i32>> {
        let Some(block_state_id) = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
        else {
            return Ok(None);
        };
        Ok(input_cooking_time(
            block_state_id,
            input,
            self.datapacks(),
            self.item_tags(),
        ))
    }

    /// The recipe the input slot matches and the fuel's burn time, both from the
    /// live datapack generation.
    fn furnace_tick_facts(
        &self,
        cooking_kind: CookingKind,
        fuel_kind: FurnaceFuelKind,
        input: Option<&RuntimeItemStack>,
        recipe_hint: Option<&str>,
        fuel: Option<&RuntimeItemStack>,
    ) -> bbb_server_types::Result<FurnaceTickFacts> {
        let mut facts = FurnaceTickFacts::default();
        if (input.is_none() && fuel.is_none()) || self.item_tags().is_none() {
            return Ok(facts);
        }
        if let Some(input) = input {
            if let Some(element) =
                self.cooking_recipe(cooking_kind, &input.item_identity_summary(), recipe_hint)
            {
                if let Some(recipe) = element.recipe.cooking_of_kind(cooking_kind) {
                    facts.recipe_result = Some(self.assemble_cooking_recipe(recipe)?);
                    facts.recipe_cooking_time = recipe.cooking_time;
                    facts.recipe_id = Some(element.id.clone());
                }
            }
        }
        if let Some(fuel) = fuel {
            facts.burn_duration = self
                .cold
                .fuel_values
                .burn_duration_for(fuel_kind, &fuel.item_identity_summary());
            facts.fuel_remainder = fuel
                .item_id()
                .and_then(crafting_remainder)
                .map(RuntimeItemStack::from_summary);
        }
        Ok(facts)
    }
}

/// The slots a hopper reaches on one face of a furnace
/// (`AbstractFurnaceBlockEntity.java:46-48`, `:257-273`, `:304-314`).
pub(in crate::world) struct FurnaceHopperFace {
    insert_slots: &'static [usize],
    take_slots: &'static [usize],
    fuel_slot_has_bucket: bool,
}

/// `SLOTS_FOR_UP` / `SLOTS_FOR_DOWN` / `SLOTS_FOR_SIDES`
/// (`AbstractFurnaceBlockEntity.java:46-48`).
const SLOTS_FOR_UP: &[usize] = &[crate::block_entity::furnace::SLOT_INPUT];
const SLOTS_FOR_DOWN: &[usize] = &[
    crate::block_entity::furnace::SLOT_RESULT,
    crate::block_entity::furnace::SLOT_FUEL,
];
const SLOTS_FOR_SIDES: &[usize] = &[crate::block_entity::furnace::SLOT_FUEL];

impl FurnaceHopperFace {
    pub(in crate::world) fn slots(&self) -> &[usize] {
        self.insert_slots
    }

    /// `canPlaceItemThroughFace` (`:266-268`) over `canPlaceItem` (`:304-314`).
    pub(in crate::world) fn allows_insert(
        &self,
        slot: usize,
        item: &ItemStackSummary,
        is_fuel: bool,
    ) -> bool {
        self.insert_slots.contains(&slot) && self.allows_place(slot, item, is_fuel)
    }

    pub(in crate::world) fn allows_place(
        &self,
        slot: usize,
        item: &ItemStackSummary,
        is_fuel: bool,
    ) -> bool {
        if slot == crate::block_entity::furnace::SLOT_RESULT {
            return false;
        }
        if slot != crate::block_entity::furnace::SLOT_FUEL {
            return true;
        }
        is_fuel
            || (item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
                && !self.fuel_slot_has_bucket)
    }

    /// `canTakeItemThroughFace` (`:271-273`): a hopper under a furnace takes
    /// from the fuel and output slots, and never takes a non-bucket out of the
    /// fuel slot.
    pub(in crate::world) fn allows_take(&self, slot: usize, item: &ItemStackSummary) -> bool {
        if !self.take_slots.contains(&slot) {
            return false;
        }
        slot != crate::block_entity::furnace::SLOT_FUEL
            || item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
            || item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_WATER_BUCKET_ID)
    }
}

impl ServerWorld {
    /// The face a hopper reaches a furnace through, or `None` when the position
    /// is not a furnace. `face` is the furnace's own side: the hopper's facing
    /// points at the furnace, so the side it touches is the opposite one.
    pub(in crate::world) fn furnace_hopper_face(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        face: bbb_protocol::packets::Direction,
    ) -> Option<FurnaceHopperFace> {
        let block_state_id = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)?;
        let (_, _) = vanilla_block_state_block_type_id(block_state_id).and_then(furnace_kinds)?;
        let fuel_slot_has_bucket = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::furnace)
            .is_some_and(|furnace| {
                furnace.original_items()[crate::block_entity::furnace::SLOT_FUEL].item_id()
                    == Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
            });
        let (insert_slots, take_slots) = match face {
            bbb_protocol::packets::Direction::Up => (SLOTS_FOR_UP, &[] as &[usize]),
            bbb_protocol::packets::Direction::Down => (SLOTS_FOR_DOWN, SLOTS_FOR_DOWN),
            _ => (SLOTS_FOR_SIDES, &[] as &[usize]),
        };
        Some(FurnaceHopperFace {
            insert_slots,
            take_slots,
            fuel_slot_has_bucket,
        })
    }

    /// The two admission sets `AbstractFurnaceMenu` builds for one cooking
    /// kind (`AbstractFurnaceMenu.java:140-146`): every item a recipe of that
    /// kind accepts, and every item `FuelValues` names as fuel.
    pub(in crate::world) fn furnace_admission_sets(
        &self,
        cooking_kind: CookingKind,
    ) -> Option<FurnaceAdmissionSets> {
        self.datapack_tables.furnace_admission_sets(
            cooking_kind,
            self.datapacks(),
            self.item_tags(),
            &self.cold.fuel_values,
        )
    }

    /// `level.fuelValues()` (`AbstractFurnaceBlockEntity.java:164`) over the
    /// live item tags.
    pub(in crate::world) fn fuel_values(&self) -> Option<&FuelValues> {
        self.item_tags().map(|_| &self.cold.fuel_values)
    }

    /// `RecipeManager.getRecipeFor` (`RecipeManager.java:131-146`) over one
    /// cooking type using the type-partitioned `recipe_index` and optional
    /// per-furnace last-recipe hint (`AbstractFurnaceBlockEntity.java:63`).
    fn cooking_recipe(
        &self,
        cooking_kind: CookingKind,
        input: &ItemStackSummary,
        hint: Option<&str>,
    ) -> Option<&bbb_server_datapack::recipe::RecipeElement> {
        cooking_recipe(
            self.datapacks(),
            self.item_tags(),
            cooking_kind,
            input,
            hint,
        )
    }
}

/// `SmeltingRecipe` / `BlastingRecipe` / `SmokingRecipe` (`RecipeType.java:9-12`).
fn furnace_kinds(block_type_id: i32) -> Option<(CookingKind, FurnaceFuelKind)> {
    match block_type_id {
        VANILLA_BLOCK_FURNACE_ID => Some((CookingKind::Smelting, FurnaceFuelKind::Furnace)),
        VANILLA_BLOCK_BLAST_FURNACE_ID => {
            Some((CookingKind::Blasting, FurnaceFuelKind::BlastFurnace))
        }
        VANILLA_BLOCK_SMOKER_ID => Some((CookingKind::Smoking, FurnaceFuelKind::Smoker)),
        _ => None,
    }
}

/// The cooking type and the menu a block's own state names
/// (`FurnaceMenu.java:10-14`, `SmokerMenu.java:10-14`,
/// `BlastFurnaceMenu.java:10-14`).
fn furnace_menu_kinds(block_state_id: i32) -> Option<(CookingKind, FurnaceMenuKind)> {
    let block_type_id = vanilla_block_state_block_type_id(block_state_id)?;
    Some(match furnace_kinds(block_type_id)?.0 {
        CookingKind::Smelting => (CookingKind::Smelting, FurnaceMenuKind::Furnace),
        CookingKind::Blasting => (CookingKind::Blasting, FurnaceMenuKind::BlastFurnace),
        CookingKind::Smoking => (CookingKind::Smoking, FurnaceMenuKind::Smoker),
        CookingKind::Campfire => return None,
    })
}

pub(in crate::world) fn input_cooking_time(
    state: i32,
    input: Option<&RuntimeItemStack>,
    datapacks: Option<&bbb_server_datapack::startup::DatapackStartup>,
    tags: Option<&ItemTagIndex>,
) -> Option<i32> {
    let (kind, _) = vanilla_block_state_block_type_id(state).and_then(furnace_kinds)?;
    let input = input?;
    Some(
        cooking_recipe(datapacks, tags, kind, &input.item_identity_summary(), None)
            .and_then(|element| element.recipe.cooking_of_kind(kind))
            .map_or(
                crate::block_entity::furnace::DEFAULT_COOKING_TIME,
                |recipe| recipe.cooking_time,
            ),
    )
}

fn cooking_recipe<'a>(
    datapacks: Option<&'a bbb_server_datapack::startup::DatapackStartup>,
    tags: Option<&ItemTagIndex>,
    kind: CookingKind,
    input: &ItemStackSummary,
    hint: Option<&str>,
) -> Option<&'a bbb_server_datapack::recipe::RecipeElement> {
    let datapacks = datapacks?;
    datapacks
        .recipe_index
        .get_recipe_for_cooking(&datapacks.recipes, kind, input, hint, tags?)
}
