//! `LivingEntity.dropAllDeathLoot` for a dying session (`LivingEntity.java:1480-1502`)
//! through `Player.dropEquipment` (`Player.java:552-567`) and the player's
//! experience reward (`:1604-1611`), plus the fall facts its death message reads.

use crate::commands::CommandDamageSourceEntity;
use crate::connection::ConnectionId;
use crate::entity::mob_death_loot::{
    MobLootAttacker, MobLootDamageSource, MobLootRequest, MobLootRoll,
};
use crate::entity::store::EntityBodyReadView;
use crate::loot_runtime::{LootEntities, LootEntity, LootOrigin};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_server_datapack::damage::{FallLocation, HurtSource};
use bbb_server_types::item_stack::RuntimeItemStack;

use bbb_facts::scoreboard_team_options::TeamVisibility;
use bbb_protocol::packets::SystemChat;
use bbb_protocol::RichTextComponent;

use super::player_packets::{accepts_player_message, broadcast_to};
use super::{ClientboundPlayPacket, PlayerState, ServerWorld, WorldEvent, RULE_KEEP_INVENTORY};

/// `EntityEquipment.dropAll` (`EntityEquipment.java:62-68`) walks its
/// `EnumMap` in `EquipmentSlot` ordinal order: offhand, then feet to head, as
/// `Inventory` indices.
const EQUIPMENT_DROP_ORDER: [usize; 5] = [40, 36, 37, 38, 39];
const MAIN_INVENTORY_SIZE: usize = 36;

impl ServerWorld {
    /// `LivingEntity.dropAllDeathLoot` (LivingEntity.java:1480-1488).
    ///
    /// The experience waits for the frame's game-event post, so a sculk
    /// catalyst hearing `ENTITY_DIE` can take it first.
    pub(super) fn drop_player_death_loot(
        &mut self,
        connection_id: ConnectionId,
        source: &HurtSource<'_>,
        causing: Option<&CommandDamageSourceEntity>,
        direct: Option<&CommandDamageSourceEntity>,
    ) {
        if self.game_rule_bool("minecraft:mob_drops", true) {
            self.drop_player_death_table(connection_id, source, causing, direct);
        }
        if !self.keep_inventory() {
            self.destroy_vanishing_cursed_items(connection_id);
            self.drop_whole_inventory(connection_id);
        }
        let player = self.levels.player_at(connection_id);
        let (dimension, entity_id, position) =
            (player.dimension, player.entity_id(), player.position());
        self.level_mut_by_id(&dimension)
            .event_sink
            .queue_death_experience(entity_id, position, None);
    }

    fn drop_player_death_table(
        &mut self,
        connection_id: ConnectionId,
        source: &HurtSource<'_>,
        causing: Option<&CommandDamageSourceEntity>,
        direct: Option<&CommandDamageSourceEntity>,
    ) {
        let player = self.levels.player_at(connection_id);
        let (dimension, position) = (player.dimension, player.position());
        let victim = self.loot_view_for_player(player);
        let killer_player = (player.cold.last_hurt_by_player_memory_time > 0)
            .then_some(player.cold.last_hurt_by_player)
            .flatten();
        let attacker = |entity: Option<&CommandDamageSourceEntity>| match entity {
            None => MobLootAttacker::Absent,
            Some(CommandDamageSourceEntity::Player { user, .. }) => {
                MobLootAttacker::SessionPlayer { uuid: user.id }
            }
            Some(CommandDamageSourceEntity::Stored { entity_id, .. }) => MobLootAttacker::Stored {
                entity_id: *entity_id,
            },
        };
        self.with_stored_hurt_level(&dimension, |entities, level| {
            let roll = level.mob_loot_drops(
                Some(EntityBodyReadView::new(entities, None, None)),
                MobLootRequest {
                    table_id: vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
                        .unwrap()
                        .default_loot_table
                        .unwrap(),
                    origin: LootOrigin {
                        x: position.x,
                        y: position.y,
                        z: position.z,
                    },
                    entities: LootEntities {
                        this_entity: LootEntity::Facts(&victim),
                        ..LootEntities::default()
                    },
                    attacking_entity: attacker(causing),
                    direct_attacking_entity: attacker(direct),
                    damage_source: MobLootDamageSource {
                        damage_type: &source.damage_type.resource_id,
                        is_direct: causing.map(CommandDamageSourceEntity::entity_id)
                            == direct.map(CommandDamageSourceEntity::entity_id),
                    },
                    killer_player,
                    seed: 0,
                },
            );
            if let MobLootRoll::Drawn(stacks) = roll {
                for stack in stacks {
                    let _ = level.spawn_runtime_item(
                        position,
                        bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack),
                        true,
                    );
                }
            }
        });
    }

    /// `Player.getBaseExperienceReward` (`Player.java:1604-1606`); a player is
    /// an always-experience dropper (`:1609-1611`).
    pub(super) fn player_experience_reward(&self, player: &PlayerState) -> i32 {
        if self.keep_inventory() || player.game_type == GameType::Spectator {
            0
        } else {
            (player.experience.level() * 7).min(100)
        }
    }

    fn keep_inventory(&self) -> bool {
        matches!(
            self.admin.game_rule_value(RULE_KEEP_INVENTORY),
            Some(GameRuleValue::Boolean(true))
        )
    }

    /// `FallLocation.getCurrentFallLocation` (FallLocation.java:35-43).
    pub(super) fn player_fall_location(&self, id: ConnectionId) -> Option<FallLocation> {
        let player = self.levels.player(id)?;
        let Some(pos) = player.block_contact.last_climbable_pos else {
            return player.was_touching_water.then_some(FallLocation::Water);
        };
        let block_id = vanilla_block_state_block_type_id(self.player_contact_state(id, pos)?)?;
        let trapdoor = self
            .player_registry_tags(id)
            .is_some_and(|tags| tags.contains_block("minecraft:trapdoors", block_id));
        Some(FallLocation::of_climbable_block(
            vanilla_block_type(block_id)?.resource_id,
            trapdoor,
        ))
    }

    /// `Player.destroyVanishingCursedItems` (`Player.java:560-567`): every
    /// slot whose stack carries a `prevent_equipment_drop` enchantment.
    fn destroy_vanishing_cursed_items(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let store = &self.level_by_id(&player.dimension).entities;
        let mut cursed = Vec::new();
        for (index, stack) in player.inventory.inventory_slots().iter().enumerate() {
            if stack.is_empty() {
                continue;
            }
            let tool = crate::entity::block_drops::loot_tool_for_runtime_stack(
                stack,
                &self.datapack_tables.enchantment_registry,
            );
            if tool
                .enchantments
                .keys()
                .any(|id| store.prevents_equipment_drop(id))
            {
                cursed.push(index);
            }
        }
        let inventory = &mut self.levels[connection_id].inventory;
        for index in cursed {
            inventory.set_inventory_slot(index, RuntimeItemStack::empty());
        }
    }

    /// `Inventory.dropAll` (`Inventory.java:444-454`): the main slots in
    /// order, each through `drop(stack, true, false)`, then the equipment.
    fn drop_whole_inventory(&mut self, connection_id: ConnectionId) {
        for index in (0..MAIN_INVENTORY_SIZE).chain(EQUIPMENT_DROP_ORDER) {
            let inventory = &mut self.levels[connection_id].inventory;
            let Some(stack) = inventory
                .slot(index)
                .filter(|stack| !stack.is_empty())
                .cloned()
            else {
                continue;
            };
            inventory.set_inventory_slot(index, RuntimeItemStack::empty());
            self.drop_player_stack_randomly(connection_id, &stack);
        }
    }

    /// `ServerPlayer.die`'s `Team.Visibility` dispatch (ServerPlayer.java:890-897)
    /// through `PlayerList.broadcastSystemToTeam` and
    /// `broadcastSystemToAllExceptTeam` (PlayerList.java:489-513).
    pub(super) fn broadcast_death_message(
        &self,
        connection_id: ConnectionId,
        message: RichTextComponent,
        events: &mut Vec<WorldEvent>,
    ) {
        let board = self.scoreboard.board();
        let team_name = |player: &PlayerState| {
            board
                .players_team(&player.profile.name)
                .map(|team| &team.name)
        };
        let team = board.players_team(&self.levels.player_at(connection_id).profile.name);
        let visibility = team.map_or(TeamVisibility::Always, |team| team.death_message_visibility);
        let team = team.map(|team| &team.name);
        let recipients = self
            .levels
            .player_entries()
            .filter(|&(&id, player)| {
                let reaches = match visibility {
                    TeamVisibility::Always => true,
                    TeamVisibility::HideForOtherTeams => {
                        id != connection_id && team_name(player) == team
                    }
                    TeamVisibility::HideForOwnTeam => team_name(player) != team,
                    TeamVisibility::Never => false,
                };
                reaches
                    && accepts_player_message(
                        player.cold.client_information.chat_visibility,
                        false,
                        false,
                    )
            })
            .map(|(&id, _)| id);
        broadcast_to(
            recipients,
            ClientboundPlayPacket::SystemChat(SystemChat::from_component(message, false)),
            events,
        );
    }
}
