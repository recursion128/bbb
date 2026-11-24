//! Player mounting and furnace fueling through the live Interact packet path.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID;
use bbb_protocol::packets::InteractEntity;

use crate::connection::ConnectionId;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::passenger::RelationAdmission;

use super::{GameType, ServerWorld, WorldEvent};

impl ServerWorld {
    pub(super) fn apply_minecart_interact(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        if player.game_type == GameType::Spectator {
            return;
        }
        let dimension = player.dimension;
        let kind = self
            .level_by_id(&dimension)
            .entities
            .get(packet.entity_id)
            .expect("the interaction target was checked")
            .entity_type_id();

        // Minecart.java:25-40 does not delegate to Entity.interact's leash branches.
        if kind == VANILLA_ENTITY_TYPE_MINECART_ID {
            if !packet.using_secondary_action
                && self
                    .level_by_id(&dimension)
                    .entities
                    .passengers(packet.entity_id)
                    .is_empty()
            {
                let stack = super::using_item::runtime_item_in_hand(
                    self.levels.player_at(connection_id),
                    packet.hand,
                )
                .clone();
                if self.start_external_player_ride_checked(
                    connection_id,
                    &dimension,
                    packet.entity_id,
                    RelationAdmission::Normal,
                    None,
                    events,
                ) == Some(true)
                {
                    // `Player.interactOn` (`Player.java:834-840`) triggers on successful mount.
                    self.trigger_player_entity_interaction(
                        connection_id,
                        "minecraft:player_interacted_with_entity",
                        dimension,
                        packet.entity_id,
                        stack.summary(),
                        events,
                    );
                }
            }
            return;
        }

        // MinecartFurnace.java:109-129 consumes only accepted fuel, including in the off hand.
        let stack = super::using_item::runtime_item_in_hand(player, packet.hand).clone();
        let player_position = player.position();
        let infinite_materials = player.abilities.instabuild;
        let membership = stack
            .item_resource_id()
            .filter(|_| !stack.is_empty())
            .is_some_and(|item| {
                self.level_by_id(&dimension)
                    .chunks
                    .registry_tags()
                    .map_or_else(
                        || {
                            EntityRegistryTagView::Builtin26_1.contains(
                                "minecraft:item",
                                "minecraft:furnace_minecart_fuel",
                                item,
                            )
                        },
                        |tags| {
                            tags.contains("minecraft:item", "minecraft:furnace_minecart_fuel", item)
                        },
                    )
            });
        let accepted = crate::entity::minecart::add_furnace_minecart_fuel_if_member(
            self.level_mut_by_id(&dimension)
                .entities
                .get_mut(packet.entity_id)
                .expect("the interacted cart remains live"),
            player_position,
            membership,
        );
        if accepted && !infinite_materials {
            self.levels[connection_id]
                .inventory
                .item_in_hand_mut(packet.hand)
                .shrink(1);
        }
        self.trigger_player_entity_interaction(
            connection_id,
            "minecraft:player_interacted_with_entity",
            dimension,
            packet.entity_id,
            stack.summary(),
            events,
        );
    }
}
