use super::*;
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::commands::{CommandPermission, CommandSource};
use crate::tests::mailbox;
use crate::world::ServerWorldConfig;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{BlockPos, PlayerRotation};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use crate::admin::NameAndId;
use crate::commands::CommandOrigin;

pub(crate) struct CommandTestWorld {
    pub(super) world: ServerWorld,
    pub(super) origin: CommandOrigin,
}

impl Default for CommandTestWorld {
    fn default() -> Self {
        Self::new(
            ServerWorldConfig {
                game_type: bbb_protocol::game_types::GameType::Survival,
                ..Default::default()
            },
            &[],
        )
    }
}

impl CommandTestWorld {
    pub(crate) fn new(config: ServerWorldConfig, players: &[NameAndId]) -> Self {
        Self::from_world(
            ServerWorld::new(config).expect("valid command world"),
            players,
        )
    }

    pub(super) fn from_world(mut world: ServerWorld, players: &[NameAndId]) -> Self {
        world.set_test_respawn_radius(0);
        let (sender, receiver) = mailbox();
        for (index, player) in players.iter().enumerate() {
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut world,
                u64::try_from(index).unwrap() + 1,
                &player
                    .name
                    .try_to_string()
                    .expect("fixture login name is valid UTF-8"),
                player.id.as_u128(),
            );
        }
        for player in world.levels.players_mut() {
            player.set_position(bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        }
        Self {
            world,
            origin: CommandOrigin {
                position: [0.0; 3],
                rotation: [0.0; 2],
                level: Java26BuiltinDimension::Overworld.into(),
                anchor: EntityAnchor::Feet,
                eye_height: None,
            },
        }
    }

    pub(crate) fn world(&self) -> &ServerWorld {
        &self.world
    }

    pub(crate) fn origin(&self) -> &CommandOrigin {
        &self.origin
    }

    /// Runs `command` to the end of its queue and answers with what the
    /// source was told and every effect it asked for.
    pub(crate) fn run(
        &mut self,
        command: &str,
        source: &CommandSource,
    ) -> crate::commands::CommandRun {
        bbb_protocol::JavaString::with_utf16(command, |units| self.run_units(units, source))
    }

    pub(crate) fn run_units(
        &mut self,
        command: &[u16],
        source: &CommandSource,
    ) -> crate::commands::CommandRun {
        let mut events = Vec::new();
        let mut report = super::super::server_functions::CommandContinuationReport::default();
        self.world.perform_command(
            std::sync::Arc::from(command),
            source,
            &self.origin,
            crate::commands::ResultCallback::Empty,
            Some(&mut report),
            &mut events,
        );
        self.world.flush_pending_disconnects(&mut events);
        let mut run = report.run;
        run.events = events;
        run
    }

    pub(crate) fn setup(&mut self, command: &str) {
        let source = CommandSource::server("console", CommandPermission::Owner);
        let run = self.run(command, &source);
        assert!(run.result.is_some(), "setup {command:?}: {run:?}");
    }

    pub(crate) fn with_position(mut self, position: [f64; 3]) -> Self {
        self.origin.position = position;
        self
    }
    pub(crate) fn with_rotation(mut self, rotation: [f32; 2]) -> Self {
        self.origin.rotation = rotation;
        self
    }
    pub(crate) fn with_level(mut self, level: Java26LevelKey) -> Self {
        self.origin.level = level;
        self
    }

    pub(crate) fn with_weather(
        mut self,
        weather: bbb_server_persistence::Java26WeatherData,
    ) -> Self {
        self.world.weather = weather;
        for level in self.world.level_ids() {
            self.world.level_mut_by_id(&level).weather.prepare(weather);
        }
        self
    }

    pub(crate) fn with_rules(mut self, rules: &[(&str, GameRuleValue)]) -> Self {
        for (id, value) in rules {
            let value = match value {
                GameRuleValue::Boolean(value) => value.to_string(),
                GameRuleValue::Integer(value) => value.to_string(),
            };
            self.setup(&format!("gamerule {id} {value}"));
        }
        self
    }
    pub(crate) fn with_scoreboard(mut self, board: &super::super::scoreboard::Scoreboard) -> Self {
        self.world.scoreboard =
            super::super::server_scoreboard::ServerScoreboard::restore(&board.pack());
        self
    }
    pub(crate) fn with_player_game_type(
        mut self,
        name: &str,
        game_type: bbb_protocol::game_types::GameType,
    ) -> Self {
        self.setup(&format!("gamemode {} {name}", game_type.serialized_name()));
        self
    }
    pub(crate) fn with_player_experience(
        mut self,
        name: &str,
        experience: bbb_server_types::experience::PlayerExperience,
    ) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        player.experience = experience;
        self
    }
    pub(crate) fn with_player_position(mut self, name: &str, position: [f64; 3]) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        player.set_position(bbb_protocol::packets::Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        });
        self
    }
    pub(crate) fn with_player_tags(mut self, name: &str, tags: Vec<String>) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        player.cold.tags = tags.into_iter().map(Into::into).collect();
        self
    }
    pub(crate) fn with_player_boarding_restrictions(mut self, name: &str) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        player.set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            true,
        );
        player.boarding_cooldown = 60;
        self
    }
    pub(crate) fn with_player_pose(mut self, name: &str, pose: i32) -> Self {
        self.world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .set_pose(pose);
        self
    }
    pub(crate) fn with_player_attributes(
        mut self,
        name: &str,
        edit: impl FnOnce(&mut crate::entity::AttributeMap),
    ) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        edit(&mut player.attributes);
        self
    }
    pub(crate) fn with_entities_in(
        mut self,
        dimension: Java26LevelKey,
        entities: &[crate::entity::Entity],
    ) -> Self {
        let level = self
            .world
            .level_mut_by_id(&self.world.require_level_id(&dimension));
        for entity in entities {
            level
                .entities
                .restore_initialized(
                    entity.uuid(),
                    entity.entity_type_id(),
                    entity.position(),
                    entity.data().clone(),
                    |body| {
                        body.set_rot(entity.y_rot(), entity.x_rot());
                        body.set_remaining_fire_ticks(entity.remaining_fire_ticks());
                        for tag in entity.entity_tags() {
                            body.add_tag(tag);
                        }
                    },
                )
                .expect("a new entity uuid is admitted");
        }
        self
    }
    pub(crate) fn with_player_level(mut self, name: &str, level: Java26LevelKey) -> Self {
        let level = self.world.require_level_id(&level);
        let connection_id = self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .connection_id;
        self.world.levels.move_player(connection_id, level);
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn with_loaded_chunk(mut self, chunk: bbb_protocol::packets::ChunkPos) -> Self {
        self.world
            .ensure_entity_transfer_full_chunk_for_test_by_key(
                &self.world.require_level_id(&self.origin.level),
                (chunk.x, chunk.z),
            );
        self
    }
    pub(crate) fn with_block(mut self, pos: BlockPos, state: i32) -> Self {
        self.world
            .ensure_entity_transfer_full_chunk_for_test_by_key(
                &self.world.require_level_id(&self.origin.level),
                (pos.x >> 4, pos.z >> 4),
            );
        self.world.set_block(
            self.world.require_level_id(&self.origin.level),
            pos,
            state,
            SET_BLOCK_UPDATE_CLIENTS,
        );
        assert_eq!(
            self.world
                .level_by_id(&self.world.require_level_id(&self.origin.level))
                .chunks
                .get_block_state(pos.x, pos.y, pos.z),
            Some(state)
        );
        self
    }
    pub(crate) fn with_container_at(mut self, pos: BlockPos, items: &[RuntimeItemStack]) -> Self {
        let mut slots = self
            .world
            .container_items_at_by_key(&self.world.require_level_id(&self.origin.level), pos)
            .expect("fixture container");
        for (slot, stack) in items.iter().enumerate() {
            slots[slot] = stack.clone();
        }
        self.world
            .write_container_items_at_by_key(
                &self.world.require_level_id(&self.origin.level),
                pos,
                slots,
            )
            .unwrap();
        self
    }
    pub(crate) fn with_player_item(
        mut self,
        name: &str,
        slot: i16,
        stack: bbb_protocol::packets::ItemStackSummary,
    ) -> Self {
        let connection = *self
            .world
            .levels
            .player_entries()
            .find(|(_, player)| player.profile.name == name)
            .expect("fixture player")
            .0;
        self.world.stock_menu_slot_for_test(connection, slot, stack);
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn with_player_rotation(mut self, name: &str, rotation: [f32; 2]) -> Self {
        self.world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .set_rotation(PlayerRotation {
                y_rot: rotation[0],
                x_rot: rotation[1],
            });
        self
    }
    pub(crate) fn with_difficulty(mut self, difficulty: Difficulty) -> Self {
        self.world
            .set_difficulty_ignoring_lock(difficulty, &mut Vec::new());
        self
    }

    pub(crate) fn with_forced_chunks(mut self, chunks: &[bbb_protocol::packets::ChunkPos]) -> Self {
        self.world.set_chunks_forced(
            self.world.require_level_id(&self.origin.level.clone()),
            true,
            chunks,
        );
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn player_item(
        &self,
        name: &str,
        inventory_slot: usize,
    ) -> Option<bbb_protocol::packets::ItemStackSummary> {
        let connection = *self
            .world
            .levels
            .player_entries()
            .find(|(_, player)| player.profile.name == name)?
            .0;
        Some(
            self.world
                .player_inventory_slot_for_test(connection, inventory_slot),
        )
    }
}

impl CommandTestWorld {
    pub(crate) fn with_player_health(mut self, name: &str, health: f32) -> Self {
        self.world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .damage
            .health = health;
        self
    }
    pub(crate) fn with_loot(mut self, loot: &bbb_server_datapack::loot::LootRegistries) -> Self {
        let mut startup = self
            .world
            .cold
            .config
            .datapacks
            .as_deref()
            .cloned()
            .unwrap_or_else(|| (*crate::registry_sync::builtin_datapack_startup_shared()).clone());
        startup.loot = loot.clone();
        self.world.cold.config.datapacks = Some(std::sync::Arc::new(startup));
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn container_item(&self, pos: BlockPos, slot: usize) -> Option<RuntimeItemStack> {
        self.world
            .container_items_at_by_key(&self.world.require_level_id(&self.origin.level), pos)?
            .get(slot)
            .cloned()
    }
    pub(crate) fn item_slot(&self, name: &str, slot: i32) -> Option<RuntimeItemStack> {
        let player = self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)?;
        crate::commands::CommandInventory(&player.inventory).slot(slot)
    }
    pub(crate) fn with_selected_slot(mut self, name: &str, slot: u8) -> Self {
        self.world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .inventory
            .set_selected_slot(slot);
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn stored_id(&self, level: &Java26LevelKey, uuid: uuid::Uuid) -> Option<i32> {
        self.world
            .level_by_id(&self.world.require_level_id(level))
            .entities
            .entity_id_for_uuid(uuid)
    }

    pub(crate) fn stored_entity(
        &self,
        level: &Java26LevelKey,
        uuid: uuid::Uuid,
    ) -> Option<&crate::entity::Entity> {
        let entities = &self
            .world
            .level_by_id(&self.world.require_level_id(level))
            .entities;
        entities.get(entities.entity_id_for_uuid(uuid)?)
    }
}

impl CommandTestWorld {
    pub(crate) fn with_entities(self, entities: &[crate::entity::Entity]) -> Self {
        self.with_entities_in(Java26BuiltinDimension::Overworld.into(), entities)
    }
    pub(crate) fn storage(&self, id: &str) -> NbtSummaryValue {
        self.world.cold.command_storage.get(id).unwrap()
    }
    pub(crate) fn with_chest(self, pos: BlockPos) -> Self {
        let properties = std::collections::BTreeMap::from([
            ("facing".into(), "north".into()),
            ("type".into(), "single".into()),
            ("waterlogged".into(), "false".into()),
        ]);
        let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:chest",
            &properties,
        )
        .unwrap();
        self.with_block(pos, state)
    }
}

impl CommandTestWorld {
    pub(crate) fn with_profile_properties(
        mut self,
        name: &str,
        properties: &[bbb_protocol::packets::GameProfileProperty],
    ) -> Self {
        self.world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .profile
            .properties = properties.to_vec();
        self
    }
}

impl CommandTestWorld {
    pub(crate) fn scheduled_events(&self) -> Vec<(String, i64)> {
        let mut events = self
            .world
            .cold
            .scheduled_events
            .pack()
            .into_iter()
            .map(|event| (event.id.to_jdk_string(), event.trigger_time))
            .collect::<Vec<_>>();
        events.sort();
        events
    }
}

impl CommandTestWorld {
    pub(crate) fn with_debug_level(mut self, level: Java26LevelKey) -> Self {
        self.world
            .level_mut_by_id(&self.world.require_level_id(&level))
            .chunks
            .install_debug_view(
                bbb_server_datapack::debug_generator::DebugLevelSource::new(-64, 384).unwrap(),
            );
        self
    }
}

/// Readers for the state the server-setting commands write.
impl CommandTestWorld {
    pub(crate) fn default_game_type(&self) -> bbb_protocol::game_types::GameType {
        self.world.cold.default_game_type
    }

    pub(crate) fn auto_save(&self) -> bool {
        self.world.auto_save
    }

    pub(crate) fn idle_timeout(&self) -> i32 {
        self.world.cold.player_idle_timeout
    }

    pub(crate) fn world_spawn(&self) -> bbb_protocol::packets::SetDefaultSpawnPosition {
        self.world.default_spawn_position()
    }

    /// The forced respawn `/spawnpoint` wrote for `name`.
    pub(crate) fn respawn(&self, name: &str) -> Option<(Java26LevelKey, BlockPos, f32, f32)> {
        let player = self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        player
            .cold
            .respawn
            .as_ref()
            .filter(|respawn| respawn.forced)
            .map(|respawn| {
                (
                    respawn.dimension.clone(),
                    respawn.pos,
                    respawn.yaw,
                    respawn.pitch,
                )
            })
    }
}

impl CommandTestWorld {
    /// Who holds `tag`: players by name in join order, then stored entities
    /// by UUID in id order.
    pub(crate) fn tagged(&self, tag: &str) -> Vec<String> {
        let mut players: Vec<_> = self
            .world
            .levels
            .player_entries()
            .filter(|(_, player)| player.cold.tags.iter().any(|held| held == tag))
            .map(|(&id, player)| (id.0, player.profile.name.clone()))
            .collect();
        players.sort();
        let mut tagged: Vec<_> = players.into_iter().map(|(_, name)| name).collect();
        for level in self.world.levels.values() {
            for id in level.entities.ids() {
                let entity = level.entities.get(id).expect("listed id");
                if entity.entity_tags().iter().any(|held| held == tag) {
                    tagged.push(entity.uuid().as_u128().to_string());
                }
            }
        }
        tagged
    }
}
