//! `FlowerPotBlock.useItemOn` / `useWithoutItem` (`FlowerPotBlock.java:58-101`).

use bbb_protocol::packets::{DataComponentPatchSummary, ItemStackSummary};

use super::BlockUseContext;
use crate::block_behavior::flower_pot as flower_pot_block;
use crate::block_behavior::BlockUseResult;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::custom_stat;

pub(super) use flower_pot_block::is_flower_pot;

/// The `item_on:` column: [`ServerWorld::use_flower_pot_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_flower_pot_item_on(context)
}

/// The `without_item:` column: [`ServerWorld::use_flower_pot_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_flower_pot_without_item(context)
}

impl ServerWorld {
    /// `FlowerPotBlock.useItemOn` (`FlowerPotBlock.java:58-83`).
    pub(super) fn use_flower_pot_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
    ) -> BlockUseResult {
        let Some(new_pot) =
            flower_pot_block::potted_state_for_held_item(context.selected_item.item_id())
        else {
            return BlockUseResult::TryWithEmptyHand;
        };
        if !flower_pot_block::is_empty_pot(context.block_state_id) {
            return BlockUseResult::Consumed;
        }
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let hand = context.update.hand;
        let source_entity_id = self.levels[connection_id].entity_id();
        let instabuild = self.levels[connection_id].abilities.instabuild;
        self.set_block(dimension, pos, new_pot, SET_BLOCK_UPDATE_ALL);
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                pos,
                Some(source_entity_id),
                None,
            );
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:pot_flower"));
            player.inventory.consume_in_hand(hand, 1, instabuild);
        }
        BlockUseResult::Consumed
    }

    /// `FlowerPotBlock.useWithoutItem` (`FlowerPotBlock.java:86-101`).
    pub(super) fn use_flower_pot_without_item(&mut self, context: &BlockUseContext<'_>) -> bool {
        let resolver = self.item_stack_resolver();
        if flower_pot_block::is_empty_pot(context.block_state_id) {
            return true;
        }
        let Some(item_id) = flower_pot_block::potted_content_item_id(context.block_state_id) else {
            return true;
        };
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let source_entity_id = self.levels[connection_id].entity_id();
        let plant = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        let result = (|| {
            let mut plant = bbb_server_types::item_stack::RuntimeItemStack::from_summary(plant);
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            let instabuild = player.abilities.instabuild;
            player
                .inventory
                .add_anywhere(&resolver, &mut plant, instabuild)?;
            Ok::<_, bbb_server_types::ServerWorldError>(plant)
        })();
        let leftover = match result {
            Ok(leftover) => leftover,
            Err(error) => {
                tracing::warn!("flower pot item output: {error}");
                return false;
            }
        };
        if !leftover.is_empty() {
            self.drop_player_stack(connection_id, &leftover, false);
        }
        self.set_block(
            dimension,
            pos,
            flower_pot_block::empty_pot_state(),
            SET_BLOCK_UPDATE_ALL,
        );
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                pos,
                Some(source_entity_id),
                None,
            );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionId;
    use crate::entity::EntityData;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::ServerWorld;
    use crate::world::{
        ClientboundPlayPacket, JoinContext, ServerWorldConfig, WorldCommand, WorldEvent,
    };
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_protocol::game_types::GameType;
    use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
    use bbb_protocol::packets::{
        BlockHitResult, BlockPos, ClientInformation, Direction, GameProfile, InteractionHand,
        PlayServerbound, UseItemOn,
    };
    use bbb_server_types::stats::custom_stat;
    use uuid::Uuid;

    fn item_id(name: &str) -> i32 {
        vanilla_item_type_for_resource_id(name)
            .unwrap_or_else(|| panic!("{name} is a built-in item"))
            .id
    }

    fn default_state(name: &str) -> i32 {
        let entry = vanilla_block_type_for_resource_id(name)
            .unwrap_or_else(|| panic!("{name} is a built-in block"));
        vanilla_block_default_state_id(entry.id).expect("default state")
    }

    fn stack(name: &str, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id(name)),
            count,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn join_world(survival: bool) -> (ServerWorld, ConnectionId) {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: if survival {
                GameType::Survival
            } else {
                GameType::Creative
            },
            force_game_mode: survival,
            ..ServerWorldConfig::default()
        })
        .expect("fixture world");
        world.set_test_respawn_radius(0);
        let (sender, receiver) =
            world_command_mailbox(WorldMailboxLimits::default()).expect("mailbox");
        let connection_id = ConnectionId(1);
        sender
            .try_send(WorldCommand::Join {
                connection_id,
                profile: GameProfile {
                    uuid: Uuid::from_u128(1),
                    name: "alice".to_string(),
                    properties: Vec::new(),
                },
                client_information: ClientInformation::default(),
                context: JoinContext::default(),
            })
            .expect("join");
        let joined = world.tick(receiver.drain());
        let teleport_id = joined
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                    ..
                } => Some(update.id),
                _ => None,
            })
            .expect("join sends the spawn teleport");
        sender
            .try_send(WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::AcceptTeleportation { id: teleport_id },
            })
            .expect("accept teleport");
        let _ = world.tick(receiver.drain());
        sender
            .try_send(WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::PlayerLoaded,
            })
            .expect("player loaded");
        let _ = world.tick(receiver.drain());
        (world, connection_id)
    }

    fn pot_context(
        connection_id: ConnectionId,
        pos: BlockPos,
        block_state_id: i32,
        selected: &ItemStackSummary,
    ) -> BlockUseContext<'_> {
        BlockUseContext {
            connection_id,
            dimension: Java26BuiltinDimension::Overworld.into(),
            update: UseItemOn {
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
                sequence: 1,
            },
            block_state_id,
            selected_item: std::borrow::Cow::Owned(crate::tests::native_stack(selected.clone())),
        }
    }

    fn place_pot(world: &mut ServerWorld, pos: BlockPos, name: &str) -> i32 {
        let state = default_state(name);
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            SET_BLOCK_UPDATE_ALL,
        ));
        state
    }

    fn hold(world: &mut ServerWorld, connection_id: ConnectionId, name: &str, count: i32) {
        let held = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack(name, count));
        *world.levels[connection_id]
            .inventory
            .item_in_hand_mut(InteractionHand::MainHand) = held;
    }

    fn held_count(world: &ServerWorld, connection_id: ConnectionId) -> i32 {
        world.levels[connection_id]
            .inventory
            .selected_stack()
            .count()
    }

    fn inventory_has(world: &ServerWorld, connection_id: ConnectionId, name: &str) -> i32 {
        let want = Some(item_id(name));
        world.levels[connection_id]
            .inventory
            .inventory_slots()
            .iter()
            .filter(|slot| slot.item_id() == want)
            .map(|slot| slot.count())
            .sum()
    }

    fn saw_block_change(world: &ServerWorld, pos: BlockPos) -> bool {
        let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        events
            .iter()
            .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE && event.pos == pos)
    }

    fn pot_flower_stat(world: &ServerWorld, connection_id: ConnectionId) -> i32 {
        world.levels[connection_id]
            .cold
            .stats
            .value(custom_stat("minecraft:pot_flower"))
    }

    fn dropped_item_ids(world: &ServerWorld) -> Vec<String> {
        let level = world.level(Java26BuiltinDimension::Overworld);
        level
            .entities
            .ids()
            .into_iter()
            .filter_map(|id| {
                let entity = level.entities.get(id)?;
                let EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some(data.item().item_resource_id()?.to_string())
            })
            .collect()
    }

    #[test]
    fn flower_pot_use_item_on_pots_a_dandelion_and_awards_the_stat() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place_pot(&mut world, pos, "minecraft:flower_pot");
        hold(&mut world, connection_id, "minecraft:dandelion", 4);
        let selected = stack("minecraft:dandelion", 4);
        let result =
            world.use_flower_pot_item_on(&pot_context(connection_id, pos, empty, &selected));
        assert_eq!(result, BlockUseResult::Consumed);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(default_state("minecraft:potted_dandelion"))
        );
        assert_eq!(held_count(&world, connection_id), 3);
        assert!(saw_block_change(&world, pos));
        assert_eq!(pot_flower_stat(&world, connection_id), 1);
    }

    #[test]
    fn flower_pot_use_item_on_does_not_shrink_in_creative() {
        let (mut world, connection_id) = join_world(false);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place_pot(&mut world, pos, "minecraft:flower_pot");
        hold(&mut world, connection_id, "minecraft:oak_sapling", 2);
        let selected = stack("minecraft:oak_sapling", 2);
        let result =
            world.use_flower_pot_item_on(&pot_context(connection_id, pos, empty, &selected));
        assert_eq!(result, BlockUseResult::Consumed);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(default_state("minecraft:potted_oak_sapling"))
        );
        assert_eq!(held_count(&world, connection_id), 2);
        assert_eq!(pot_flower_stat(&world, connection_id), 1);
    }

    #[test]
    fn flower_pot_use_item_on_refuses_a_non_plant() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place_pot(&mut world, pos, "minecraft:flower_pot");
        hold(&mut world, connection_id, "minecraft:stick", 1);
        let selected = stack("minecraft:stick", 1);
        let result =
            world.use_flower_pot_item_on(&pot_context(connection_id, pos, empty, &selected));
        assert_eq!(result, BlockUseResult::TryWithEmptyHand);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(empty)
        );
        assert_eq!(held_count(&world, connection_id), 1);
        assert!(!saw_block_change(&world, pos));
        assert_eq!(pot_flower_stat(&world, connection_id), 0);
    }

    #[test]
    fn flower_pot_use_item_on_consumes_when_the_pot_is_already_filled() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let filled = place_pot(&mut world, pos, "minecraft:potted_poppy");
        hold(&mut world, connection_id, "minecraft:dandelion", 3);
        let selected = stack("minecraft:dandelion", 3);
        let result =
            world.use_flower_pot_item_on(&pot_context(connection_id, pos, filled, &selected));
        assert_eq!(result, BlockUseResult::Consumed);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(filled)
        );
        assert_eq!(held_count(&world, connection_id), 3);
        assert!(!saw_block_change(&world, pos));
        assert_eq!(pot_flower_stat(&world, connection_id), 0);
    }

    #[test]
    fn flower_pot_use_without_item_returns_the_plant() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let filled = place_pot(&mut world, pos, "minecraft:potted_dandelion");
        hold(&mut world, connection_id, "minecraft:stick", 1);
        let selected = ItemStackSummary::empty();
        assert!(world.use_flower_pot_without_item(&pot_context(
            connection_id,
            pos,
            filled,
            &selected,
        )));
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(default_state("minecraft:flower_pot"))
        );
        assert_eq!(
            inventory_has(&world, connection_id, "minecraft:dandelion"),
            1
        );
        assert!(saw_block_change(&world, pos));
    }

    #[test]
    fn flower_pot_use_without_item_drops_when_inventory_is_full() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let filled = place_pot(&mut world, pos, "minecraft:potted_oak_sapling");
        {
            let stone = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack(
                "minecraft:stone",
                64,
            ));
            let player = world.levels.player_mut(connection_id).expect("joined");
            for index in 0..player.inventory.inventory_slots().len() {
                player.inventory.set_inventory_slot(index, stone.clone());
            }
        }
        let selected = ItemStackSummary::empty();
        assert!(world.use_flower_pot_without_item(&pot_context(
            connection_id,
            pos,
            filled,
            &selected,
        )));
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(default_state("minecraft:flower_pot"))
        );
        assert_eq!(
            inventory_has(&world, connection_id, "minecraft:oak_sapling"),
            0
        );
        assert_eq!(dropped_item_ids(&world), ["minecraft:oak_sapling"]);
    }

    #[test]
    fn flower_pot_use_without_item_consumes_an_empty_pot() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place_pot(&mut world, pos, "minecraft:flower_pot");
        let selected = ItemStackSummary::empty();
        assert!(world.use_flower_pot_without_item(&pot_context(
            connection_id,
            pos,
            empty,
            &selected,
        )));
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(empty)
        );
        assert!(!saw_block_change(&world, pos));
    }
}
