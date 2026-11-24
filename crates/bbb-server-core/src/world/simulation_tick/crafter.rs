use super::hopper::{WorldContainer, WorldContainerAccess};
use super::*;
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::fluid::with_property;
use crate::world::LevelId;
use bbb_server_datapack::recipe::{CraftingInput, ItemTagIndex};
use bbb_server_types::item_stack::RuntimeItemStack;

#[cfg(test)]
mod tests;

impl ServerWorld {
    // CrafterBlock.java:149-181: assemble and output before consuming the original nine slots.
    pub(super) fn tick_crafter(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let result = self.craft_from_block(dimension, pos, state, events);
        if let Err(error) = result {
            tracing::warn!("crafter: {error}");
        }
    }

    fn craft_from_block(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<()> {
        let Some(instance) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .filter(|entity| entity.crafter().is_some())
            .cloned()
        else {
            return Ok(());
        };
        let items = instance
            .crafter()
            .expect("captured crafter")
            .items()
            .to_vec();
        let fail = |error: String| bbb_server_types::ServerWorldError::InvalidConfiguration(error);
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .ok_or_else(|| fail("crafter has no datapack generation".into()))?;
        let resolver = self.item_stack_resolver();
        let input = CraftingInput::of(
            3,
            3,
            items.iter().map(|item| item.summary().clone()).collect(),
        );
        let built_tags;
        let tags: &ItemTagIndex = if let Some(cached) = self.cold.crafting_item_tags.as_deref() {
            cached
        } else {
            built_tags = startup
                .tags
                .registry("minecraft:item")
                .map(|tags| {
                    ItemTagIndex::from_loaded_with_content_registry(
                        tags,
                        self.registries.content_registry(),
                    )
                })
                .unwrap_or_default()
                .with_enchantment_curses(&startup);
            &built_tags
        };
        let Some(recipe) = startup.recipe_index.get_recipe_for_crafting(
            &startup.recipes,
            &resolver,
            &input,
            None,
            tags,
            self.maps.crafting_map_data(&input),
        ) else {
            self.crafter_level_event(dimension, pos, 1050, 0, events);
            return Ok(());
        };
        let result = RuntimeItemStack::from_summary(recipe.recipe.assemble_crafting(&input, tags));
        if result.is_empty() {
            self.crafter_level_event(dimension, pos, 1050, 0, events);
            return Ok(());
        }
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::crafter_mut)
            .expect("live crafter")
            .set_crafting_ticks_remaining(6);
        self.set_block(
            dimension,
            pos,
            with_property(state, "crafting", "true").expect("crafter crafting"),
            SET_BLOCK_UPDATE_CLIENTS,
        );
        let result = self.maps.post_process_crafted_map(result, &resolver)?;
        let facing = crafter_front(state);
        self.dispense_crafted_item(
            dimension,
            (pos, facing),
            result,
            &recipe.id,
            &instance,
            events,
        )?;
        for item in recipe.recipe.remaining_items(&input) {
            let item = RuntimeItemStack::from_summary(item);
            if !item.is_empty() {
                self.dispense_crafted_item(
                    dimension,
                    (pos, facing),
                    item,
                    &recipe.id,
                    &instance,
                    events,
                )?;
            }
        }
        if let Some(crafter) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .filter(|current| current.same_position_and_type(&instance))
            .and_then(BlockEntity::crafter_mut)
        {
            consume_inputs(crafter.items_mut())?;
        }
        self.with_level(dimension, |level| {
            level.block_entity_set_changed(pos, state);
        });
        Ok(())
    }

    // CrafterBlock.java:196-227: insertion is quiet; only physical leftovers award criteria and events.
    fn dispense_crafted_item(
        &mut self,
        dimension: &LevelId,
        (pos, facing): (BlockPos, Direction),
        mut remaining: RuntimeItemStack,
        recipe: &str,
        instance: &BlockEntity,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<()> {
        let source = self.with_level(dimension, |level| level.block_container(pos, true));
        let mut access = WorldContainerAccess {
            world: self,
            dimension: *dimension,
            pos,
            facing,
        };
        if let Some(target) =
            access.container_at(bbb_server_types::directions::relative(pos, facing))
        {
            let from = source.map(WorldContainer::Block);
            let side = Some(bbb_server_types::directions::opposite(facing));
            if access.is_crafter(target)
                || remaining.count() > access.max_stack_size(target, &remaining)?
            {
                while !remaining.is_empty() {
                    let rest = access.add_item(from, target, remaining.with_count(1), side)?;
                    if !rest.is_empty() {
                        break;
                    }
                    remaining.set_count(remaining.count() - 1);
                }
            } else {
                while !remaining.is_empty() {
                    let old = remaining.count();
                    remaining = access.add_item(from, target, remaining, side)?;
                    if remaining.count() == old {
                        break;
                    }
                }
            }
        }
        if remaining.is_empty() {
            return Ok(());
        }
        let step = bbb_server_types::directions::relative(BlockPos { x: 0, y: 0, z: 0 }, facing);
        let center = crafter_center(pos);
        let position = Vec3d {
            x: center.x + f64::from(step.x) * 0.7,
            y: center.y + f64::from(step.y) * 0.7,
            z: center.z + f64::from(step.z) * 0.7,
        };
        self.spawn_dispensed_stack(dimension, position, facing, 6, &remaining)?;
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        let bounds = bbb_server_types::geometry::Aabb {
            min_x: center.x - 8.5,
            min_y: center.y - 8.5,
            min_z: center.z - 8.5,
            max_x: center.x + 8.5,
            max_y: center.y + 8.5,
            max_z: center.z + 8.5,
        };
        let players = self
            .level_by_id(dimension)
            .entities
            .players_in_box_sections(bounds)
            .into_iter()
            .filter(|id| {
                self.levels.player(*id).is_some_and(|player| {
                    &player.dimension == dimension
                        && !player.avatar_removed
                        && player.game_type != GameType::Spectator
                        && player
                            .bounding_box()
                            .is_ok_and(|body| body.intersects(bounds))
                })
            })
            .collect::<Vec<_>>();
        for player in players {
            // Reward functions run synchronously and may edit or retire this exact BE.
            let current = self
                .level_by_id(dimension)
                .chunks
                .block_entities()
                .get(pos)
                .filter(|current| current.same_position_and_type(instance))
                .cloned()
                .unwrap_or_else(|| instance.clone());
            let ingredients = current
                .crafter()
                .expect("captured crafter")
                .items()
                .to_vec();
            self.trigger_recipe_crafted(
                player,
                "minecraft:crafter_recipe_crafted",
                recipe,
                &ingredients,
                events,
            );
        }
        self.crafter_level_event(dimension, pos, 1049, 0, events);
        self.crafter_level_event(dimension, pos, 2010, i32::from(facing.id()), events);
        Ok(())
    }

    fn crafter_level_event(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        id: i32,
        data: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        self.level_mut_by_id(dimension)
            .event_sink
            .level_event(id, pos, data);
        self.flush_level_events(dimension, events);
    }
}

fn crafter_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

fn crafter_front(state: i32) -> Direction {
    match crate::block_behavior::string_property(state, "orientation")
        .as_deref()
        .and_then(|value| value.split('_').next())
    {
        Some("down") => Direction::Down,
        Some("up") => Direction::Up,
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        _ => Direction::North,
    }
}

fn consume_inputs(
    items: &mut [bbb_server_types::item_stack::RuntimeItemStack],
) -> bbb_server_types::Result<()> {
    for item in items {
        if !item.is_empty() {
            item.set_count(item.count() - 1);
        }
    }
    Ok(())
}
