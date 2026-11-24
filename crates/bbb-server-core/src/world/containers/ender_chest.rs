use super::*;
use crate::block_behavior::ender_chest as ender_block;
use crate::world::LevelId;

impl ServerWorld {
    pub(in crate::world) fn use_ender_chest_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let Some(chest) = self
            .get_block_entity(dimension, pos)
            .filter(|chest| chest.ender_chest().is_some())
            .cloned()
        else {
            return true;
        };
        let above = relative(pos, Direction::Up);
        if self.block_state(dimension, above).is_some_and(|state| {
            crate::redstone::is_redstone_conductor_with_registry(
                state,
                self.plugins().content_registry(),
            )
        }) {
            return true;
        }
        let level_key = self.level_key(dimension).clone();
        if self.levels.player(connection_id).is_some() {
            match self.open_ender_chest_menu(connection_id, level_key, chest, pos, events) {
                Ok(()) => {
                    if let Some(player) = self.levels.player_mut(connection_id) {
                        player.award_stat_once(
                            &mut self.scoreboard,
                            custom_stat("minecraft:open_enderchest"),
                        );
                    }
                    self.anger_nearby_piglins(connection_id, true);
                }
                Err(error) => tracing::warn!(%error, "ender chest menu did not open"),
            }
        }
        true
    }

    fn open_ender_chest_menu(
        &mut self,
        connection_id: ConnectionId,
        level_key: crate::world::Java26LevelKey,
        chest: BlockEntity,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> std::result::Result<(), String> {
        let resolver = self.item_stack_resolver();
        let container_id = self
            .begin_container_open(connection_id, events)
            .ok_or_else(|| "old menu close was refused".to_owned())?;
        self.levels[connection_id]
            .inventory
            .set_active_ender_chest(Some((level_key, chest)));
        let opened = match self.levels[connection_id]
            .inventory
            .prepare_ender_chest_menu(&resolver, None, container_id, pos)
        {
            Ok(Some(opened)) => opened,
            Ok(None) => {
                self.levels[connection_id]
                    .inventory
                    .set_active_ender_chest(None);
                return Err("ender chest menu prepare was refused".to_owned());
            }
            Err(error) => {
                self.levels[connection_id]
                    .inventory
                    .set_active_ender_chest(None);
                return Err(format!("ender chest prepare: {error}"));
            }
        };
        self.change_ender_chest_openers(connection_id, true);
        let title = chest_menu_title("container.enderchest")
            .ok_or_else(|| "ender chest title was refused".to_owned())?;
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

    pub(super) fn change_ender_chest_openers(&mut self, target: ConnectionId, opening: bool) {
        let Some(player) = self.levels.player(target) else {
            return;
        };
        if player.game_type == GameType::Spectator {
            return;
        }
        let Some((dimension, active)) = player.inventory.active_ender_chest() else {
            return;
        };
        let pos = active.block_pos();
        let range = player.block_interaction_range();
        let entity_id = player.entity_id();
        self.with_level(self.require_level_id(&dimension), |level| {
            let Some(current) = level.chunks.block_entities().get(pos) else {
                return false;
            };
            if !current.same_position_and_type(&active) {
                return false;
            }
            let state = current.block_state();
            ender_block::change_openers(level, pos, state, entity_id, range, opening)
        });
    }

    // `Container.stillValidBlockEntity` (`Container.java:94-101`) checks the
    // chest's own level plus `player.isWithinBlockInteractionRange`.
    pub(super) fn ender_chest_still_valid(&self, connection_id: ConnectionId) -> CoreResult<bool> {
        let player = self.levels.player_at(connection_id);
        let Some((dimension, active)) = player.inventory.active_ender_chest() else {
            return Ok(false);
        };
        let pos = active.block_pos();
        let Some(current) = self
            .level_by_id(&self.require_level_id(&dimension))
            .chunks
            .block_entities()
            .get(pos)
        else {
            return Ok(false);
        };
        Ok(current.same_position_and_type(&active)
            && is_within_block_interaction_range(
                player.eye_position()?,
                pos,
                player.block_interaction_range(),
                CONTAINER_DISTANCE_BUFFER,
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::SET_BLOCK_UPDATE_ALL;
    use crate::tests::{join_loaded, mailbox, operator_config};
    use crate::world::WorldCommand;
    use bbb_protocol::packets::{
        BlockHitResult, BlockPos, Direction, InteractionHand, PlayServerbound, UseItemOn,
    };
    use bbb_server_types::save::Java26BuiltinDimension;

    #[test]
    fn opening_second_ender_chest_closes_first_before_tracking_second() {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
        world.set_test_respawn_radius(0);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let dimension = Java26BuiltinDimension::Overworld;
        let first = BlockPos { x: 0, y: 64, z: 1 };
        let second = BlockPos { x: 1, y: 64, z: 1 };
        let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_ENDER_CHEST_ID,
        )
        .unwrap();
        assert!(world.set_block(dimension, first, state, SET_BLOCK_UPDATE_ALL));
        assert!(world.set_block(dimension, second, state, SET_BLOCK_UPDATE_ALL));

        let use_block = |world: &mut ServerWorld, pos: BlockPos, sequence: i32| {
            sender
                .try_send(WorldCommand::Play {
                    connection_id: ConnectionId(1),
                    packet: PlayServerbound::UseItemOn(UseItemOn {
                        hand: InteractionHand::MainHand,
                        hit: BlockHitResult {
                            pos,
                            direction: Direction::Up,
                            cursor_x: 0.5,
                            cursor_y: 0.5,
                            cursor_z: 0.5,
                            inside: false,
                            world_border_hit: false,
                        },
                        sequence,
                    }),
                })
                .unwrap();
            let _ = world.tick(receiver.drain());
        };

        use_block(&mut world, first, 1);
        assert_eq!(
            world
                .get_block_entity(dimension, first)
                .unwrap()
                .ender_chest()
                .unwrap()
                .open_count(),
            1
        );
        use_block(&mut world, second, 2);
        assert_eq!(
            world
                .get_block_entity(dimension, first)
                .unwrap()
                .ender_chest()
                .unwrap()
                .open_count(),
            0
        );
        assert_eq!(
            world
                .get_block_entity(dimension, second)
                .unwrap()
                .ender_chest()
                .unwrap()
                .open_count(),
            1
        );
        assert_eq!(
            world
                .player_inventory_for_test(ConnectionId(1))
                .active_ender_chest()
                .map(|(_, chest)| chest.block_pos()),
            Some(second)
        );
    }
}
