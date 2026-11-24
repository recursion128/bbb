//! Vehicle container menus over the entity slot owner.

use super::*;
use crate::entity::{item_slots, Entity};
use bbb_server_types::interaction::is_within_entity_interaction_range;

fn vehicle_layout(entity: &Entity) -> Option<MenuLayout> {
    match item_slots::container_size(entity)? {
        crate::entity::minecart_container::CHEST_CONTAINER_SIZE => Some(MenuLayout::Chest {
            rows: CHEST_MENU_ROWS,
        }),
        crate::entity::minecart_container::HOPPER_CONTAINER_SIZE => Some(MenuLayout::Hopper),
        _ => None,
    }
}

fn vehicle_items(entity: &Entity) -> Option<Vec<RuntimeItemStack>> {
    (0..item_slots::container_size(entity)?)
        .map(|slot| item_slots::read(entity, slot as i32).ok().flatten())
        .collect()
}

impl ServerWorld {
    // ContainerEntity.java:93-112: player luck/context, raw vehicle position and lazy fill.
    pub(in crate::world) fn unpack_vehicle_container(
        &mut self,
        level: &LevelId,
        id: i32,
        opener: Option<ContainerLootOpener<'_>>,
    ) -> Result<Option<String>, String> {
        if !self
            .level_by_id(level)
            .entities
            .get(id)
            .is_some_and(item_slots::container_has_loot)
        {
            return Ok(None);
        }
        let (server_level, mut tables, facts, _) = self.level_and_drop_tables(level);
        let entities = &server_level.entities;
        let chunks = &mut server_level.chunks;
        let random = &server_level.random;
        let weather = &server_level.weather;
        let Some(entity) = entities.get(id) else {
            return Ok(None);
        };
        item_slots::unpack_vehicle_cargo(entity, |position, request, available| {
            crate::world::simulation_level::block_entity_loot::fill_unpacked_container(
                chunks,
                &mut tables,
                &facts,
                Some(crate::entity::store::EntityBodyReadView::new(
                    entities, None, None,
                )),
                weather,
                random,
                crate::loot_runtime::LootOrigin {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                },
                request,
                opener,
                available,
            )
        })
    }

    pub(in crate::world) fn open_vehicle_container(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(error) = self.open_vehicle_container_checked(connection_id, entity_id, events) {
            tracing::warn!(%error, "vehicle container did not open");
        }
    }

    fn open_vehicle_container_checked(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let resolver = self.item_stack_resolver();
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let Some(entity) = self.level_by_id(&dimension).entities.get(entity_id) else {
            return Ok(());
        };
        let Some(layout) = vehicle_layout(entity) else {
            return Ok(());
        };
        // AbstractMinecartContainer.java:128-135; AbstractChestBoat.java:153-159.
        if player.game_type == GameType::Spectator && item_slots::container_has_loot(entity) {
            return Ok(());
        }
        let title = entity
            .custom_name()
            .cloned()
            .or_else(|| {
                chest_menu_title(&format!(
                    "entity.{}",
                    entity.data().entity_resource_id().replace(':', ".")
                ))
            })
            .ok_or("vehicle title was refused")?;
        let view = self.loot_view_for_player(player);
        let luck = player
            .attributes
            .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID)
            .unwrap_or(0.0) as f32;
        if let Some(table) = self.unpack_vehicle_container(
            &dimension,
            entity_id,
            Some(ContainerLootOpener {
                luck,
                entity: LootEntity::Facts(&view),
            }),
        )? {
            self.trigger_generate_loot(connection_id, &table, events);
        }
        let Some(items) = self
            .level_by_id(&dimension)
            .entities
            .get(entity_id)
            .and_then(vehicle_items)
        else {
            return Ok(());
        };
        let Some(container_id) = self.begin_container_open(connection_id, events) else {
            return Ok(());
        };
        let Some(opened) = self.levels[connection_id]
            .inventory
            .prepare_vehicle_container(&resolver, container_id, layout, dimension, entity_id, items)
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        // Player.java:824-830: spectators open the provider without interacting with it.
        if self.levels.player_at(connection_id).game_type != GameType::Spectator {
            self.vehicle_container_event(connection_id, dimension, entity_id, true);
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenScreen(OpenScreen {
                container_id: opened.container_id,
                menu_type_id: opened.menu_type_id,
                title,
            }),
        ));
        self.initialize_player_menu_checked(connection_id, events)
    }

    pub(super) fn refresh_vehicle_container(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let Some((dimension, id)) = player.inventory.vehicle_container() else {
            return;
        };
        if dimension != player.dimension {
            return;
        }
        if let Some(items) = self
            .level_by_id(&dimension)
            .entities
            .get(id)
            .and_then(vehicle_items)
        {
            self.levels[connection_id]
                .inventory
                .refresh_container_items(items);
        }
    }

    pub(super) fn flush_vehicle_container(
        &mut self,
        connection_id: ConnectionId,
    ) -> CoreResult<()> {
        let player = self.levels.player_at(connection_id);
        let Some((dimension, id)) = player.inventory.vehicle_container() else {
            return Ok(());
        };
        if dimension != player.dimension {
            return Ok(());
        }
        let items = player.inventory.container_items();
        let Some(entity) = self.level_mut_by_id(&dimension).entities.get_mut(id) else {
            return Ok(());
        };
        for (slot, item) in items.into_iter().enumerate() {
            item_slots::set_container_item(entity, slot, item)
                .map_err(bbb_server_types::ServerWorldError::InvalidConfiguration)?;
        }
        Ok(())
    }

    // ContainerEntity.java:168-169: an existing vehicle within the entity range plus four.
    pub(super) fn vehicle_container_still_valid(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        let Some((dimension, id)) = player.inventory.vehicle_container() else {
            return false;
        };
        if dimension != player.dimension {
            return false;
        }
        let Some(entity) = self.level_by_id(&dimension).entities.get(id) else {
            return false;
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        entity.is_alive()
            && vehicle_layout(entity).is_some()
            && is_within_entity_interaction_range(
                eye,
                &entity.bounding_box(),
                player.entity_interaction_range(),
                4.0,
            )
    }

    pub(super) fn vehicle_container_event(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        entity_id: i32,
        opening: bool,
    ) {
        let player_id = self.levels.player_at(connection_id).entity_id();
        let level = self.level_mut_by_id(&dimension);
        let Some(entity) = level.entities.get(entity_id) else {
            return;
        };
        // MinecartChest.java:60-72; AbstractChestBoat.java:90-105,197-198.
        if vehicle_layout(entity)
            != Some(MenuLayout::Chest {
                rows: CHEST_MENU_ROWS,
            })
        {
            return;
        }
        let position = entity.position();
        let event = if opening {
            crate::level_event::VanillaGameEvent::CONTAINER_OPEN
        } else {
            crate::level_event::VanillaGameEvent::CONTAINER_CLOSE
        };
        level
            .event_sink
            .game_event_with_source_at(event, position, Some(player_id), None);
        if opening {
            self.anger_nearby_piglins(connection_id, true);
        }
    }
}
