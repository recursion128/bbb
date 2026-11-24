use crate::commands::CommandPlayerTarget;
use crate::commands::{
    container_slot, team_player_display_name_nbt, CommandItemModifier, ItemCommand, ItemSlot,
};
use crate::world::registry_runtime::PlayerItemStackResolver;
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

impl ServerWorld {
    /// Runs a resolved `/item`: the count of changed slots and the line that
    /// reports them.
    pub(crate) fn run_item_command(
        &mut self,
        command: &ItemCommand,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(i32, NbtSummaryValue), NbtSummaryValue> {
        let resolver = self.item_stack_resolver();
        command.execute(&mut WorldItemAccess {
            world: self,
            command,
            resolver: &resolver,
            events,
        })
    }
    /// `None` skips invalid block slots; `Some` selects `/item` source/target errors.
    pub(crate) fn read_command_item_slot(
        &mut self,
        level: &bbb_server_types::save::Java26LevelKey,
        slot: &ItemSlot,
        target: Option<bool>,
    ) -> Result<Option<RuntimeItemStack>, NbtSummaryValue> {
        match slot {
            ItemSlot::Block { pos, slot } => {
                let level_id = self.require_level_id(level);
                let size = self.command_item_container_size(&level_id, *pos);
                if target.is_none()
                    && size.is_some_and(|size| {
                        usize::try_from(*slot).map_or(true, |slot| slot >= size)
                    })
                {
                    return Ok(None);
                }
                container_slot(size, *pos, *slot, target.unwrap_or(false))?;
                self.with_level(level_id, |level| {
                    level.get_block_entity_item(*pos, *slot as usize)
                })
                .map_err(failure)
            }
            ItemSlot::Stored {
                uuid, level, slot, ..
            } => {
                self.unpack_item_command_vehicle(&self.require_level_id(level), *uuid, *slot)
                    .map_err(failure)?;
                let entities = &self.level_by_id(&self.require_level_id(level)).entities;
                let Some(entity) = entities
                    .entity_id_for_uuid(*uuid)
                    .and_then(|id| entities.get(id))
                else {
                    return Ok(None);
                };
                crate::entity::item_slots::read(entity, *slot).map_err(failure)
            }
            ItemSlot::Player { player, slot } => {
                let Some(player) = self
                    .command_player_connection(player)
                    .and_then(|target| self.levels.player(target))
                else {
                    return Ok(None);
                };
                Ok(crate::commands::CommandInventory(&player.inventory).slot(*slot))
            }
        }
    }

    pub(crate) fn write_command_item_slot(
        &mut self,
        level_key: &bbb_server_types::save::Java26LevelKey,
        slot: &ItemSlot,
        stack: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> Result<bool, NbtSummaryValue> {
        match slot {
            ItemSlot::Block { pos, slot } => {
                container_slot(
                    self.command_item_container_size(&self.require_level_id(level_key), *pos),
                    *pos,
                    *slot,
                    true,
                )?;
                self.set_container_item_from_command(
                    self.require_level_id(level_key),
                    *pos,
                    *slot,
                    stack,
                )
                .map_err(failure)?;
                Ok(true)
            }
            ItemSlot::Stored {
                uuid, level, slot, ..
            } => {
                self.unpack_item_command_vehicle(&self.require_level_id(level), *uuid, *slot)
                    .map_err(failure)?;
                let result = self
                    .with_stored_hurt_level(
                        &self.require_level_id(level),
                        |entities, entity_level| {
                            let Some(id) = entities.entity_id_for_uuid(*uuid) else {
                                return Ok((false, None));
                            };
                            crate::entity::item_slots::write(
                                entities,
                                entity_level,
                                id,
                                *slot,
                                stack,
                            )
                        },
                    )
                    .map_err(failure)?;
                if let Some(pos) = result.1 {
                    self.item_frame_output_changed_by_key(&self.require_level_id(level), pos);
                }
                Ok(result.0)
            }
            ItemSlot::Player { player, slot } => self
                .set_entity_item_from_command(player.clone(), *slot, stack, events)
                .map_err(failure),
        }
    }

    fn command_item_container_size(
        &self,
        level: &crate::world::LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Option<usize> {
        self.level_by_id(level)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::container_slots)
            .map(|items| items.len())
    }

    pub(in crate::world) fn unpack_item_command_vehicle(
        &mut self,
        level: &crate::world::LevelId,
        uuid: uuid::Uuid,
        slot: i32,
    ) -> Result<(), String> {
        let entities = &self.level_by_id(level).entities;
        let Some(id) = entities.entity_id_for_uuid(uuid) else {
            return Ok(());
        };
        let valid = entities
            .get(id)
            .and_then(crate::entity::item_slots::container_size)
            .is_some_and(|size| usize::try_from(slot).is_ok_and(|slot| slot < size));
        if valid {
            self.unpack_vehicle_container(level, id, None)?;
        }
        Ok(())
    }
}

/// The live world as `/item` reads and writes slots.
pub(crate) struct WorldItemAccess<'a> {
    world: &'a mut ServerWorld,
    command: &'a ItemCommand,
    resolver: &'a PlayerItemStackResolver,
    events: &'a mut Vec<WorldEvent>,
}

impl WorldItemAccess<'_> {
    pub(crate) fn read(
        &mut self,
        slot: &ItemSlot,
        target: bool,
    ) -> Result<Option<RuntimeItemStack>, NbtSummaryValue> {
        self.world
            .read_command_item_slot(&self.command.level, slot, Some(target))
    }

    pub(crate) fn write(
        &mut self,
        slot: &ItemSlot,
        stack: RuntimeItemStack,
    ) -> Result<bool, NbtSummaryValue> {
        self.world
            .write_command_item_slot(&self.command.level, slot, stack, self.events)
    }

    pub(crate) fn display(
        &self,
        stack: &RuntimeItemStack,
    ) -> Result<NbtSummaryValue, NbtSummaryValue> {
        crate::commands::runtime_item_stack_display_name(stack, self.resolver)
    }

    pub(crate) fn player_name(&self, player: &CommandPlayerTarget) -> NbtSummaryValue {
        self.world
            .command_player_connection(player)
            .and_then(|target| self.world.levels.player(target))
            .map(|body| {
                team_player_display_name_nbt(
                    &crate::commands::CommandPlayerView {
                        name: (&body.profile.name).into(),
                        uuid: body.profile.uuid,
                    },
                    self.world.scoreboard(),
                )
            })
            .unwrap_or_else(|| component_literal_nbt(&player.name))
    }

    pub(crate) fn modify(
        &mut self,
        modifier: &CommandItemModifier,
        stack: RuntimeItemStack,
    ) -> Result<RuntimeItemStack, NbtSummaryValue> {
        let random = self.world.random.clone();
        self.world
            .apply_command_item_modifier(self.command, modifier, stack, &random)
    }
}

fn failure(error: impl std::fmt::Display) -> NbtSummaryValue {
    component_literal_nbt(error.to_string())
}

#[cfg(test)]
mod tests;
