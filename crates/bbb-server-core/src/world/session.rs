//! A player's session on this world: the join handshake, the play-phase
//! packet arms that change what the session is set to, difficulty and game
//! mode, respawn and teleport, and the removal that ends it.

mod game_mode_ground;
mod lifecycle;
mod packets;
mod respawn;
mod rules_and_modes;
mod teleport;

#[cfg(test)]
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
#[cfg(test)]
use bbb_protocol::packets::GameProfile;
#[cfg(test)]
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{
    ChangeDifficulty, GameEvent, PlayerAbilities, PlayerPositionUpdate, Respawn, Vec3d,
};
#[cfg(test)]
use bbb_protocol::recipe_wire_types::VANILLA_RECIPE_BOOK_TYPES;
use bbb_protocol::RichTextComponent;

use crate::block_destroy::DestroyProgress;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::ExternalPlayerCanRide;
use bbb_server_datapack::damage::PlayerDamageState;
use bbb_server_types::experience::PlayerExperience as ExperienceState;
use bbb_server_types::food::FoodData;
#[cfg(test)]
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

#[cfg(test)]
use super::JoinContext;
#[cfg(test)]
use super::RecipeBookTypeSettings;
use super::{
    block_position, player_chunk, send, PlayerState, ServerWorld, WorldEvent, RULE_KEEP_INVENTORY,
    UNSENT_HEALTH,
};

/// `GameRules.PLAYER_MOVEMENT_CHECK` (`GameRules.java:63`), which
/// `shouldCheckPlayerMovement` (`ServerGamePacketListenerImpl.java:1202-1203`)
/// reads before the moved-too-quickly arm.
pub(super) const RULE_PLAYER_MOVEMENT_CHECK: &str = "minecraft:player_movement_check";
/// `GameRules.ELYTRA_MOVEMENT_CHECK` (`GameRules.java:34`), the second half of
/// the same gate for a fall-flying player.
pub(super) const RULE_ELYTRA_MOVEMENT_CHECK: &str = "minecraft:elytra_movement_check";

/// `ServerPlayer.readAdditionalSaveData` loads `Abilities` before
/// `ServerPlayerGameMode.setGameModeForPlayer` applies the selected mode
/// (`GameType.java:62-80`). Speeds are not mode-owned, and Creative leaves the
/// loaded `flying` bit alone.
fn abilities_after_game_type(
    mut abilities: PlayerAbilities,
    game_type: GameType,
) -> PlayerAbilities {
    match game_type {
        GameType::Creative => {
            abilities.can_fly = true;
            abilities.instabuild = true;
            abilities.invulnerable = true;
        }
        GameType::Spectator => {
            abilities.can_fly = true;
            abilities.instabuild = false;
            abilities.invulnerable = true;
            abilities.flying = true;
        }
        GameType::Survival | GameType::Adventure => {
            abilities.can_fly = false;
            abilities.instabuild = false;
            abilities.invulnerable = false;
            abilities.flying = false;
        }
    }
    abilities
}

impl PlayerState {
    /// `Entity.canRide` (Entity.java:2393-2395), projected at the Player owner
    /// boundary because relation storage can neither own nor tick a session.
    pub(super) fn external_player_can_ride(&self) -> ExternalPlayerCanRide {
        if self.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN) || self.boarding_cooldown > 0
        {
            ExternalPlayerCanRide::Denied
        } else {
            ExternalPlayerCanRide::Allowed
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn player_transition_state_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (
        Java26LevelKey,
        Vec3d,
        bool,
        i32,
        Option<super::ChunkTrackingView>,
    ) {
        let player = self.levels.player_at(connection_id);
        (
            self.level_key(player.dimension).clone(),
            player.position(),
            player.is_changing_dimension,
            player.boarding_cooldown,
            self.sessions[&connection_id].chunk_tracking_view,
        )
    }

    pub(crate) fn set_game_rule_for_test(
        &mut self,
        resource_id: &'static str,
        value: GameRuleValue,
    ) -> bool {
        self.admin.set_game_rule(resource_id, value)
    }

    pub(crate) fn game_rule_for_test(&self, resource_id: &str) -> Option<GameRuleValue> {
        self.admin.game_rule_value(resource_id)
    }

    pub(crate) fn set_player_position_for_test(
        &mut self,
        connection_id: ConnectionId,
        position: Vec3d,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test session is connected");
        player.set_position(position);
        player.first_good = position;
        player.last_good = position;
        player.set_on_ground(true);
        self.update_player_query_section(connection_id);
    }

    pub(crate) fn detached_player_for_test(
        entity_id: i32,
        position: Vec3d,
        game_type: GameType,
    ) -> PlayerState {
        thread_local! {
            static JOINED: PlayerState = {
                let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
                world.join(
                    ConnectionId(1),
                    GameProfile {
                        uuid: uuid::Uuid::nil(),
                        name: String::new(),
                        properties: Vec::new(),
                    },
                    Default::default(),
                    JoinContext::default(),
                    &mut Vec::new(),
                );
                world.levels.remove_player(ConnectionId(1)).unwrap()
            };
        }
        let uuid = uuid::Uuid::from_u128(entity_id as u128);
        let mut player = JOINED.with(Clone::clone);
        player.profile.uuid = uuid;
        player.profile.name = format!("player{entity_id}");
        player.body = crate::entity::Entity::new(
            entity_id,
            uuid,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position,
            crate::entity::EntityData::Player,
        );
        player.game_type = game_type;
        player
    }

    pub(crate) fn player_movement_validation_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (Vec3d, Option<i32>, Vec3d, Vec3d, i32, i32) {
        let player = self.levels.player_at(connection_id);
        (
            player.position(),
            player.awaiting_teleport,
            player.first_good,
            player.last_good,
            player.received_move_packets,
            player.known_move_packets,
        )
    }

    pub(crate) fn player_recipe_book_settings(
        &self,
        connection_id: ConnectionId,
    ) -> [RecipeBookTypeSettings; VANILLA_RECIPE_BOOK_TYPES.len()] {
        self.levels[connection_id].cold.recipe_book_settings
    }

    pub(crate) fn award_player_recipe_for_test(
        &mut self,
        connection_id: ConnectionId,
        id: impl Into<String>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test player is connected");
        player.inventory.recipe_book_mut().award(id);
    }

    pub(crate) fn player_knows_recipe_for_test(
        &self,
        connection_id: ConnectionId,
        id: &str,
    ) -> bool {
        self.levels[connection_id]
            .inventory
            .recipe_book()
            .contains(id)
    }

    pub(crate) fn player_recipe_is_highlighted_for_test(
        &self,
        connection_id: ConnectionId,
        id: &str,
    ) -> bool {
        self.levels[connection_id]
            .inventory
            .recipe_book()
            .to_be_displayed()
            .any(|candidate| candidate == id)
    }

    pub(crate) fn player_crafting_result_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> ItemStackSummary {
        self.levels[connection_id]
            .inventory
            .slot_item(0)
            .summary()
            .clone()
    }

    pub(crate) fn player_entity_id_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].entity_id()
    }

    pub(crate) fn player_arrow_count_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].cold.arrows.count
    }

    pub(crate) fn player_respawn_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> Option<super::PlayerRespawnConfig> {
        self.levels[connection_id].cold.respawn.clone()
    }

    pub(crate) fn external_player_relation_in_dimension_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        player_entity_id: i32,
    ) -> (Option<i32>, Option<i32>) {
        let entities = &self.level(dimension).entities;
        (
            entities.external_player_vehicle(player_entity_id),
            entities.external_player_root_vehicle(player_entity_id),
        )
    }

    pub(crate) fn set_player_mode_and_abilities_for_test(
        &mut self,
        connection_id: ConnectionId,
        game_type: GameType,
        abilities: PlayerAbilities,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test session is connected");
        player.game_type = game_type;
        player.abilities = abilities;
    }

    pub(crate) fn player_abilities_for_test(&self, connection_id: ConnectionId) -> PlayerAbilities {
        self.levels[connection_id].abilities
    }

    pub(crate) fn player_attributes_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> &crate::entity::AttributeMap {
        &self.levels[connection_id].attributes
    }

    pub(crate) fn set_player_attribute_base_for_test(
        &mut self,
        connection_id: ConnectionId,
        attribute_id: i32,
        value: f64,
    ) {
        self.levels[connection_id]
            .attributes
            .get_instance(attribute_id)
            .expect("the player supplier declares the attribute")
            .set_base_value(value);
    }

    pub(crate) fn prime_player_body_removal_for_test(&mut self, connection_id: ConnectionId) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test session is connected");
        let max_health = player.max_health();
        let damage = &mut player.damage;
        damage.set_health(0.0, max_health);
        damage.death_time = bbb_server_datapack::damage::DEATH_TICKS - 1;
    }

    pub(crate) fn set_player_boarding_cooldown_for_test(
        &mut self,
        connection_id: ConnectionId,
        ticks: i32,
    ) {
        self.levels[connection_id].boarding_cooldown = ticks;
    }

    pub(crate) fn player_shift_key_down_for_test(&self, connection_id: ConnectionId) -> bool {
        self.levels[connection_id].shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
    }

    pub(crate) fn set_player_fall_flying_for_test(
        &mut self,
        connection_id: ConnectionId,
        value: bool,
    ) {
        self.levels[connection_id].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, value);
    }

    pub(crate) fn set_player_dead_for_test(&mut self, connection_id: ConnectionId) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the test player is connected");
        let max_health = player.max_health();
        player.damage.set_health(0.0, max_health);
    }

    pub(crate) fn remove_player_for_test(
        &mut self,
        connection_id: ConnectionId,
    ) -> Vec<WorldEvent> {
        let mut events = Vec::new();
        self.remove_player(connection_id, &mut events);
        events
    }

    pub(crate) fn attach_external_player_for_test(
        &mut self,
        connection_id: ConnectionId,
        vehicle_id: i32,
    ) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("the test session is connected");
        let player_entity_id = player.entity_id();
        let dimension = player.dimension;
        let can_ride = player.external_player_can_ride();
        self.level_mut_by_id(&dimension)
            .entities
            .attach_external_player(player_entity_id, vehicle_id, can_ride)
    }

    pub(crate) fn detach_external_player_for_test(
        &mut self,
        connection_id: ConnectionId,
    ) -> Option<i32> {
        let player = self.levels.player(connection_id)?;
        let player_entity_id = player.entity_id();
        let dimension = player.dimension;
        self.level_mut_by_id(&dimension)
            .entities
            .detach_external_player(player_entity_id)
    }

    pub(crate) fn external_player_vehicle_state_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (i32, ExternalPlayerCanRide, Option<i32>, Option<i32>) {
        let player = self
            .levels
            .player(connection_id)
            .expect("the test session is connected");
        let entities = &self.level_by_id(&player.dimension).entities;
        (
            player.boarding_cooldown,
            player.external_player_can_ride(),
            entities.external_player_vehicle(player.entity_id()),
            entities.external_player_root_vehicle(player.entity_id()),
        )
    }

    pub(crate) fn player_tags_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> &[bbb_protocol::JavaString] {
        &self.levels[connection_id].cold.tags
    }

    pub(crate) fn set_player_tags_for_test(
        &mut self,
        connection_id: ConnectionId,
        tags: Vec<bbb_protocol::JavaString>,
    ) {
        self.levels[connection_id].cold.tags = tags;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{join_loaded, mailbox};
    use crate::world::{ServerWorldConfig, WorldCommand};
    use uuid::Uuid;

    #[test]
    fn graceful_shutdown_snapshots_online_players_for_shutdown_save() {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        world.set_test_respawn_radius(0);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

        sender.try_send(WorldCommand::Shutdown).unwrap();
        let stopped = world.tick(receiver.drain());
        assert_eq!(stopped.online_player_count, 0);
        assert!(stopped.events.iter().any(|event| matches!(
            event,
            WorldEvent::Close {
                connection_id: ConnectionId(1),
                ..
            }
        )));

        let save = world
            .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
            .unwrap();
        assert!(
            save.player_patches
                .iter()
                .any(|p| p.uuid == Uuid::from_u128(1)),
            "online player must be included in shutdown save: {:?}",
            save.player_patches
                .iter()
                .map(|p| p.uuid)
                .collect::<Vec<_>>()
        );
    }
}
