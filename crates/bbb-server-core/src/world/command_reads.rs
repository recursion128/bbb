//! Live entity reads for commands and world predicates.

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;

use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_runtime_facts, dimension_type_runtime_facts, DimensionTypeRuntimeFacts,
};
use bbb_server_types::save::Java26LevelKey;

use crate::chunk::ChunkStore;
use crate::commands::{
    CommandDatapack, CommandOrigin, CommandPackDescription, CommandPlayer, CommandStoredEntity,
};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{EntityRegistryTagView, EntityStore};
use crate::loot_runtime::{LootEntityFacts, LootEntityMovement, LootWeather, StoredLootEntity};
use crate::random::GameRandom;
use crate::world::LevelId;

use super::command_storage::ServerCommandStorage;
use super::level_blocks::LootChunks;
use super::level_keys::LevelNames;
use super::registry_runtime::PlayerItemStackResolver;
use super::scoreboard::Scoreboard;
use super::{PlayerState, ServerWorld};

/// The live players and every level's entities and blocks, with the board,
/// advancements and storage selectors and resolved components consult.
pub(crate) struct EntityReads<'a> {
    pub(in crate::world) names: &'a LevelNames,
    pub(in crate::world) levels: Vec<LevelRead<'a>>,
    // Chunk loans are invariant; the entity directory must remain covariant for board lookups.
    pub(in crate::world) chunk_inputs: Vec<(&'a Java26LevelKey, LootChunks<'a, 'a, 'a>)>,
    pub(in crate::world) maps: Option<&'a RefCell<&'a mut super::maps::ServerMapStore>>,
    pub(in crate::world) generate_structures: bool,
    pub(in crate::world) resolver: OnceCell<PlayerItemStackResolver>,
    pub(in crate::world) datapacks: Option<&'a Arc<bbb_server_datapack::startup::DatapackStartup>>,
    pub(in crate::world) item_tags: &'a bbb_server_datapack::recipe::ItemTagIndex,
    pub(in crate::world) registries: &'a crate::registry_sync::SynchronizedRegistries,
    pub(in crate::world) clocks: &'a super::world_clock::ServerWorldClocks,
    pub(in crate::world) world_seed: i64,
    pub(in crate::world) storage: &'a ServerCommandStorage,
    pub(crate) scoreboard: &'a Scoreboard,
    pub(crate) advancements: &'a bbb_server_datapack::player_advancements::AdvancementTree,
    pub(crate) loot: Option<&'a bbb_server_datapack::loot::LootRegistries>,
    /// The overworld's registry tags, which `type=#tag` reads for players.
    pub(crate) entity_tags: Option<&'a crate::registry_tags::RegistryTagSnapshot>,
    pub(in crate::world) enchantment_registry: &'a Arc<Vec<String>>,
    pub(in crate::world) enchantment_slots:
        &'a Arc<Vec<crate::enchantment_facts::EnchantmentSlots>>,
}

pub(in crate::world) struct LevelRead<'a> {
    pub(in crate::world) key: &'a Java26LevelKey,
    pub(in crate::world) entities: &'a EntityStore,
    pub(in crate::world) tags: Option<&'a crate::registry_tags::RegistryTagSnapshot>,
    pub(in crate::world) random: &'a GameRandom,
    pub(in crate::world) weather: LootWeather,
    pub(in crate::world) sky_darkening: Option<i32>,
    pub(in crate::world) raids: &'a super::raids::LevelRaids,
    pub(in crate::world) environment_attributes:
        crate::world::environment_attributes::LootEnvironmentContext<'a>,
}

impl<'a> EntityReads<'a> {
    pub(crate) fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        self.registries.content_registry()
    }

    pub(crate) fn content_registry_arc(
        &self,
    ) -> Arc<bbb_protocol::content_registry::ContentRegistry> {
        Arc::clone(self.registries.content_registry_arc())
    }

    pub(crate) fn predicate_environment<'s>(
        &'s self,
        key: &Java26LevelKey,
        player_slots: &'s dyn Fn(
            uuid::Uuid,
            i32,
        ) -> Result<
            Option<crate::loot_runtime::LootTool>,
            crate::loot_runtime::LootRuntimeReason,
        >,
        player_nbt: &'s dyn Fn(
            uuid::Uuid,
        )
            -> Result<NbtSummaryValue, crate::loot_runtime::LootRuntimeReason>,
    ) -> Option<impl crate::loot_runtime::LootItemEnvironment + 's + use<'a, 's>> {
        use super::loot_context::entity_scoreboard::{LootEntityLevels, LootEntityScoreboard};

        let level = self.levels.iter().find(|level| level.key == key)?;
        let chunks = self
            .chunk_inputs
            .iter()
            .find(|(key, _)| *key == level.key)?
            .1;
        let environment =
            super::simulation_level::mob_death_loot::LevelLootEnvironment::for_world_predicates(
                chunks,
                self.resolver(),
                self.datapacks.map(|packs| packs.as_ref()),
                self.enchantment_registry.as_slice(),
                self.datapacks
                    .map(|packs| crate::entity::block_drops::RecipeDropFacts {
                        recipes: &packs.recipes,
                        recipe_index: &packs.recipe_index,
                        item_tags: self.item_tags,
                    }),
                self.clocks,
                self.storage,
                self.registries,
                self.world_seed,
                level.sky_darkening,
                level.environment_attributes,
                LootEntityScoreboard {
                    scoreboard: self.scoreboard,
                    names: self.names,
                    dimension: self.names.id(level.key)?,
                    bodies: Some(EntityBodyReadView::new(level.entities, None, None)),
                    other_levels: LootEntityLevels::Reads(&self.levels),
                },
            );
        let environment = match self.maps {
            Some(maps) => environment.with_maps(
                maps,
                self.world_seed,
                level.key.as_str(),
                self.generate_structures,
            ),
            None => environment,
        };
        Some(
            environment
                .with_player_slots(player_slots)
                .with_player_nbt(player_nbt)
                .with_advancement_tree(self.advancements),
        )
    }

    pub(crate) fn loot_vehicle_slots(
        &self,
        requested: &crate::entity::Entity,
        selected: &[i32],
    ) -> Result<
        Vec<bbb_server_types::item_stack::RuntimeItemStack>,
        crate::loot_runtime::LootRuntimeReason,
    > {
        use crate::loot_runtime::LootRuntimeReason;

        let (level, actual) = self
            .levels
            .iter()
            .find_map(|level| {
                let actual = level
                    .entities
                    .get(requested.id())
                    .filter(|actual| actual.uuid() == requested.uuid())?;
                Some((level, actual))
            })
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("vehicle slots need the actual source level/body".into())
            })?;
        let source_blocks = |pos| {
            let chunks = self.blocks(level.key)?;
            (chunks.is_in_world_bounds(pos) && chunks.has_chunk_at(pos))
                .then(|| chunks.block_state_at(pos))
        };
        let source_block_data = |pos| {
            self.blocks(level.key)
                .expect("served source chunks")
                .block_entity_loot_data(pos, Some(self.resolver()))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        };
        let player_slots = |uuid, slot| self.player_item_slot(uuid, slot);
        let player_nbt = |uuid| self.player_loot_nbt(uuid);
        let environment = self
            .predicate_environment(level.key, &player_slots, &player_nbt)
            .ok_or_else(|| {
                LootRuntimeReason::Catalog(
                    "vehicle slots need the actual source level environment".into(),
                )
            })?;
        super::simulation_level::block_entity_loot::vehicle_slots_with_environment(
            self.loot.ok_or_else(|| {
                LootRuntimeReason::Catalog("no loot registries are loaded".into())
            })?,
            Some(&environment),
            Some(level.key.as_str()),
            &source_blocks,
            &source_block_data,
            level.weather,
            level.random,
            Some(EntityBodyReadView::new(level.entities, None, None)),
            actual,
            selected,
        )
    }

    pub(crate) fn related_command_entities(
        &self,
        source: &crate::commands::CommandSource,
        relation: crate::commands::types::ExecuteRelation,
    ) -> Vec<crate::commands::types::CommandDamageEntityRef<'a>> {
        use crate::commands::types::{CommandDamageEntityRef as Ref, ExecuteRelation as Relation};
        use crate::commands::CommandSourceIdentity;
        use crate::entity::passenger::{
            direct_controlling_living_with_registry_tags, DirectLivingController,
        };
        use crate::entity::{EntityData, EntityRegistryTagView};

        if matches!(
            source.identity,
            CommandSourceIdentity::Server | CommandSourceIdentity::Block { .. }
        ) {
            return Vec::new();
        }
        let Some(level) = self.levels.iter().find(|level| {
            level.entities.entity_id_for_uuid(source.uuid).is_some()
                || level.entities.player_by_uuid(source.uuid).is_some()
        }) else {
            return Vec::new();
        };
        let store = level.entities;
        let Some(id) = store.entity_id_for_uuid(source.uuid).or_else(|| {
            store
                .player_by_uuid(source.uuid)
                .map(|player| player.entity_id())
        }) else {
            return Vec::new();
        };
        let body = store.get(id);
        let local = |id| self.command_entity_at(level, id);
        let resolve_living = |uuid| self.living_command_entity_by_uuid(uuid);
        let result = match relation {
            Relation::Vehicle => store.vehicle(id).and_then(local),
            Relation::Passengers => {
                return store.passengers(id).iter().filter_map(|id| local(*id)).collect();
            }
            Relation::Leasher => body.and_then(crate::entity::leash::leash_holder)
                .and_then(|id| self.levels.iter().find_map(|level| self.command_entity_at(level, id))),
            Relation::Controller => body.and_then(|entity| {
                let tags = self.blocks(level.key).expect("served source chunks").registry_tags_shared().map(EntityRegistryTagView::Frozen)
                    .unwrap_or(EntityRegistryTagView::Builtin26_1);
                let controller = direct_controlling_living_with_registry_tags(
                    entity, Some(store), &tags,
                    |id| store.player(id).filter(|player| !player.avatar_removed)
                        .map(ServerWorld::nearby_player_facts),
                ).flatten()?;
                let id = match controller {
                    DirectLivingController::Player(player) => player.entity_id,
                    DirectLivingController::Stored(id) => id,
                };
                local(id)
            }),
            Relation::Owner => body
                .and_then(|entity| entity.ownable_owner().flatten())
                .and_then(resolve_living),
            Relation::Target => body.and_then(|entity| {
                if let Some(interaction) = crate::entity::interaction::interaction(entity) {
                    let uuid = interaction.interaction()?.player;
                    return store.player_by_uuid(uuid).and_then(|player| local(player.entity_id()));
                }
                let target = EntityBodyReadView::new(store, Some(entity), None)
                    .qualified_mob_target(id)??;
                self.command_entity_by_uuid(target.uuid())
            }),
            Relation::Attacker => {
                if let Some(player) = store.player(id) {
                    player.cold.last_hurt_by_mob.and_then(resolve_living)
                } else {
                    body.and_then(|entity| {
                        if let Some(interaction) = crate::entity::interaction::interaction(entity) {
                            let uuid = interaction.attack()?.player;
                            return store.player_by_uuid(uuid).and_then(|player| local(player.entity_id()));
                        }
                        let reference = entity.living()?.last_hurt_by_mob()?;
                        reference.uuid().and_then(resolve_living)
                    })
                }
            }
            Relation::Origin => body.and_then(|entity| match entity.data() {
                EntityData::Item(data) => self.command_entity_by_uuid(data.thrower()?),
                EntityData::Tnt(data) => resolve_living(data.owner()?),
                EntityData::AreaEffectCloud(data) => resolve_living(data.owner()?),
                EntityData::EvokerFangs(data) => resolve_living(data.owner()?),
                EntityData::GeneralLiving(data) => self
                    .command_entity_by_uuid(data.vex_retained()?.owner()?)
                    .filter(|entity| match entity {
                        Ref::Player(_) => false,
                        Ref::Stored(stored) => matches!(stored.entity.data(), EntityData::GeneralLiving(data) if data.is_mob()),
                    }),
                data => self.command_entity_by_uuid(data.projectile_owner()?),
            }),
        };
        result.into_iter().collect()
    }

    /// `EntityReference.getEntity(level, LivingEntity.class)`
    /// (EntityReference.java:55-82): `getEntityInAnyDimension`, kept only when it
    /// is a living entity that has not been removed.
    pub(crate) fn living_command_entity_by_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
        use crate::commands::types::CommandDamageEntityRef as Ref;
        self.command_entity_by_uuid(uuid)
            .filter(|entity| match entity {
                Ref::Player(_) => true,
                Ref::Stored(stored) => stored.entity.living().is_some(),
            })
    }

    pub(crate) fn command_entity_by_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
        self.levels.iter().find_map(|level| {
            command_entity_for_uuid_in_store(
                level.entities,
                uuid,
                self.names,
                level.key,
                level.tags,
            )
        })
    }

    pub(crate) fn command_entity_by_id(
        &self,
        level: &Java26LevelKey,
        id: i32,
    ) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
        let level = self.levels.iter().find(|read| read.key == level)?;
        self.command_entity_at(level, id)
    }

    fn command_entity_at(
        &self,
        level: &LevelRead<'a>,
        id: i32,
    ) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
        command_entity_in_store(level.entities, id, self.names, level.key, level.tags)
    }

    /// Online players in connection order.
    pub(crate) fn players(&self) -> impl Iterator<Item = CommandPlayer<'a>> + Clone + 'a {
        let names = self.names;
        super::players_in_connection_order(
            self.levels
                .iter()
                .flat_map(|level| level.entities.players()),
        )
        .map(move |player| player.command_row(names.key(player.dimension)))
    }

    pub(crate) fn stored_entities(&self) -> impl Iterator<Item = CommandStoredEntity<'a>> + '_ {
        self.levels.iter().flat_map(|level| {
            let (key, store, tags) = (level.key, level.entities, level.tags);
            store.bodies().map(move |entity| CommandStoredEntity {
                entity,
                dimension: key,
                registry_tags: tags,
            })
        })
    }

    /// Spatial stored-entity query over sections intersecting `aabb` (using
    /// `ids_in_box_sections`, which already applies Vanilla's `in_walked_section` halo).
    pub(crate) fn stored_entities_in_box(
        &self,
        aabb: bbb_server_types::geometry::Aabb,
        level_filter: Option<&Java26LevelKey>,
    ) -> Vec<CommandStoredEntity<'a>> {
        let mut out = Vec::new();
        for level in &self.levels {
            if level_filter.is_some_and(|wanted| level.key != wanted) {
                continue;
            }
            let (key, store, tags) = (level.key, level.entities, level.tags);
            for id in store.ids_in_box_sections(aabb) {
                if let Some(entity) = store.get(id) {
                    out.push(CommandStoredEntity {
                        entity,
                        dimension: key,
                        registry_tags: tags,
                    });
                }
            }
        }
        out
    }

    /// `/data get entity` for the player or stored entity holding `uuid`.
    pub(crate) fn entity_data(&self, uuid: uuid::Uuid) -> Result<Option<NbtSummaryValue>, String> {
        if let Some(player) = self
            .levels
            .iter()
            .find_map(|level| level.entities.player_by_uuid(uuid))
        {
            let key = self.names.key(player.dimension);
            let root_vehicle = match self.levels.iter().find(|level| level.key == key) {
                Some(level) if !player.avatar_removed => {
                    crate::entity::save::write_external_player_root_vehicle_value(
                        level.entities,
                        player.entity_id(),
                    )
                    .map_err(|error| {
                        bbb_server_types::save::SaveError::InvalidNbt {
                            path: None,
                            detail: format!("player {} RootVehicle: {error}", player.profile.uuid),
                        }
                    })
                }
                _ => Ok(None),
            };
            return super::player_save::command_player_data_with_sources(
                player,
                key,
                self.resolver(),
                root_vehicle,
                super::player_ender_pearls::saved_player_ender_pearls(
                    player.profile.uuid,
                    self.levels.iter().map(|level| {
                        (
                            level.key,
                            crate::entity::store::EntityBodyReadView::new(
                                level.entities,
                                None,
                                None,
                            ),
                        )
                    }),
                ),
            )
            .map(Some)
            .map_err(|error| error.to_string());
        }
        let Some((store, id)) = self.levels.iter().find_map(|level| {
            level
                .entities
                .entity_id_for_uuid(uuid)
                .filter(|id| level.entities.get(*id).is_some())
                .map(|id| (level.entities, id))
        }) else {
            return Ok(None);
        };
        store
            .command_data(id)
            .map(Some)
            .map_err(|error| format!("entity command data: {error}"))
    }

    pub(crate) fn player_loot_nbt(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<NbtSummaryValue, crate::loot_runtime::LootRuntimeReason> {
        self.entity_data(uuid)
            .map_err(crate::loot_runtime::LootRuntimeReason::Catalog)?
            .ok_or_else(|| {
                crate::loot_runtime::LootRuntimeReason::Catalog(
                    "player NBT requires an online player".into(),
                )
            })
    }

    pub(crate) fn player_item_slot(
        &self,
        uuid: uuid::Uuid,
        slot: i32,
    ) -> Result<Option<crate::loot_runtime::LootTool>, crate::loot_runtime::LootRuntimeReason> {
        let (player, store) = self
            .levels
            .iter()
            .find_map(|level| {
                level
                    .entities
                    .player_by_uuid(uuid)
                    .map(|player| (player, level.entities))
            })
            .ok_or_else(|| {
                crate::loot_runtime::LootRuntimeReason::Catalog(
                    "player slots need an online player".into(),
                )
            })?;
        Ok(super::loot_context::player_nbt::read_player_slot(
            player,
            slot,
            store.enchantment_registry(),
        ))
    }

    pub(crate) fn player_loot_facts(&self, player: CommandPlayer<'_>) -> LootEntityFacts {
        let level = self
            .levels
            .iter()
            .find(|level| level.key == player.dimension)
            .expect("a selector player belongs to its served level");
        let player = level
            .entities
            .player_by_uuid(player.id)
            .expect("a selector player belongs to its served level");
        let mut facts = super::loot_context::player_loot_facts(
            player,
            self.enchantment_registry,
            self.enchantment_slots,
        );
        let tags = self
            .blocks(level.key)
            .expect("served source chunks")
            .registry_tags_shared()
            .map_or(
                EntityRegistryTagView::Builtin26_1,
                EntityRegistryTagView::Frozen,
            );
        let bodies = EntityBodyReadView::new(level.entities, None, None);
        facts.vehicle =
            super::loot_context::player_vehicle_loot_facts(player, bodies, Some(level.raids));
        facts.movement =
            ServerWorld::player_known_movement_in_bodies(player, bodies, &tags).map(|known| {
                LootEntityMovement {
                    known_movement: [known.x, known.y, known.z],
                    fall_distance: player.block_contact.fall_distance,
                }
            });
        facts
    }

    pub(crate) fn stored_loot_entity(
        &self,
        entity: CommandStoredEntity<'a>,
    ) -> StoredLootEntity<'a> {
        let level = self
            .levels
            .iter()
            .find(|level| level.key == entity.dimension)
            .expect("a selector entity belongs to a served level");
        StoredLootEntity {
            entity: entity.entity,
            bodies: Some(crate::entity::store::EntityBodyReadView::new(
                level.entities,
                None,
                None,
            )),
            raid_at_position: Some(level.raids.raid_at(entity.entity.block_position())),
        }
    }

    pub(crate) fn random(&self, key: &Java26LevelKey) -> &'a GameRandom {
        self.levels
            .iter()
            .find(|level| level.key == key)
            .expect("a command names a served level")
            .random
    }

    pub(crate) fn weather(&self, key: &Java26LevelKey) -> LootWeather {
        self.levels
            .iter()
            .find(|level| level.key == key)
            .expect("a selector entity belongs to a served level")
            .weather
    }

    pub(crate) fn resolver(&self) -> &PlayerItemStackResolver {
        self.resolver
            .get_or_init(|| PlayerItemStackResolver::new(self.datapacks, self.registries))
    }

    /// The blocks of the served level `key`.
    pub(crate) fn blocks(
        &self,
        key: &Java26LevelKey,
    ) -> Option<impl std::ops::Deref<Target = ChunkStore> + '_> {
        self.chunk_inputs
            .iter()
            .find(|(level, _)| *level == key)
            .map(|(_, chunks)| chunks.borrow())
    }

    /// `CommandStorage.get`, publishing the namespace on the read.
    pub(crate) fn storage(&self, id: &str) -> Option<NbtSummaryValue> {
        self.storage.read(id)
    }
}

/// `PackRepository.getAvailablePacks`, `getSelectedPacks` and the world's
/// `enabledFeatures`.
pub(crate) struct CommandDatapacks {
    pub(crate) available: Vec<CommandDatapack>,
    pub(crate) selected: Vec<CommandDatapack>,
    pub(crate) enabled_features: Vec<String>,
}

impl CommandDatapacks {
    /// `PackRepository.getPack` (`PackRepository.java:118-120`).
    pub(crate) fn pack(&self, id: &str) -> Option<&CommandDatapack> {
        self.available.iter().find(|pack| pack.id == id)
    }

    /// `Pack.equals` is location identity, so the id is it.
    pub(crate) fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|pack| pack.id == id)
    }

    /// `getRequestedFeatures().isSubsetOf(enabledFeatures())`
    /// (`DataPackCommand.java:242`, `:293`).
    pub(crate) fn features_available(&self, pack: &CommandDatapack) -> bool {
        pack.requested_features
            .iter()
            .all(|feature| self.enabled_features.contains(feature))
    }
}

pub(in crate::world) fn command_entity_for_uuid_in_store<'a>(
    store: &'a EntityStore,
    uuid: uuid::Uuid,
    names: &'a LevelNames,
    dimension: &'a Java26LevelKey,
    tags: Option<&'a crate::registry_tags::RegistryTagSnapshot>,
) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
    let id = store
        .entity_id_for_uuid(uuid)
        .or_else(|| store.player_by_uuid(uuid).map(|player| player.entity_id()))?;
    command_entity_in_store(store, id, names, dimension, tags)
}

fn command_entity_in_store<'a>(
    store: &'a EntityStore,
    id: i32,
    names: &'a LevelNames,
    dimension: &'a Java26LevelKey,
    tags: Option<&'a crate::registry_tags::RegistryTagSnapshot>,
) -> Option<crate::commands::types::CommandDamageEntityRef<'a>> {
    use crate::commands::types::CommandDamageEntityRef as Ref;
    if let Some(player) = store.player(id).filter(|player| !player.avatar_removed) {
        return Some(Ref::Player(player.command_row(names.key(player.dimension))));
    }
    let entity = store
        .get(id)
        .filter(|entity| entity.removal_reason().is_none())?;
    Some(Ref::Stored(CommandStoredEntity {
        entity,
        dimension,
        registry_tags: tags,
    }))
}

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

impl ServerWorld {
    /// `MinecraftServer.createCommandSourceStack`'s position, rotation and level
    /// against the compile-time fixture spawn (used by standalone unit tests).
    #[cfg(test)]
    pub(crate) fn console_origin() -> CommandOrigin {
        CommandOrigin {
            position: Self::console_position(),
            rotation: Self::CONSOLE_ROTATION,
            level: Self::CONSOLE_LEVEL.into(),
            anchor: EntityAnchor::Feet,
            eye_height: None,
        }
    }

    /// `MinecraftServer.createCommandSourceStack` (`MinecraftServer.java:1677-1697`):
    /// lower corner of the live `default_spawn`, facing south, in its respawn
    /// dimension.
    pub(crate) fn live_console_origin(&self) -> CommandOrigin {
        let pos = super::player_packets::block_position(self.cold.default_spawn.position);
        let level = if self.has_level_id(&self.cold.default_spawn.dimension) {
            self.level_key(self.cold.default_spawn.dimension).clone()
        } else {
            Self::CONSOLE_LEVEL.into()
        };
        CommandOrigin {
            position: [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)],
            rotation: Self::CONSOLE_ROTATION,
            level,
            anchor: EntityAnchor::Feet,
            eye_height: None,
        }
    }

    /// `GameRules.get`, falling back to the rule's registered default for a
    /// rule this world's feature set did not register (`GameRuleMap.reset`).
    pub(crate) fn game_rule(
        &self,
        resource_id: &str,
    ) -> Option<bbb_protocol::game_rules::GameRuleValue> {
        self.admin.game_rule_value(resource_id).or_else(|| {
            bbb_protocol::game_rules::vanilla_game_rule(resource_id)
                .map(|entry| entry.default_value)
        })
    }

    /// [`Self::game_rule`] over a rule whose type is an int.
    pub(crate) fn integer_game_rule(&self, resource_id: &str) -> i32 {
        match self.game_rule(resource_id) {
            Some(bbb_protocol::game_rules::GameRuleValue::Integer(value)) => value,
            _ => unreachable!("{resource_id} is an integer rule"),
        }
    }

    pub(crate) fn with_entity_reads<R>(&mut self, f: impl FnOnce(&EntityReads<'_>) -> R) -> R {
        let tags = self
            .levels
            .iter()
            .map(|(id, level)| (id, level.chunks.registry_tags_shared()))
            .collect::<Vec<_>>();
        let (names, entries) = self.levels.iter_mut_with_names();
        let mut levels = Vec::new();
        let mut chunks = Vec::new();
        for (id, level) in entries {
            levels.push(LevelRead {
                key: names.key(id),
                entities: &level.entities,
                tags: tags
                    .iter()
                    .find(|(key, _)| *key == id)
                    .and_then(|(_, tags)| tags.as_deref()),
                random: &level.random,
                weather: LootWeather {
                    raining: level.weather.is_raining(),
                    thundering: level.weather.is_thundering(),
                },
                sky_darkening: level.sky_darkening.map(i32::from),
                raids: &level.raids,
                environment_attributes: self.cold.dimension_sky_lights.loot_environment_context(
                    self.cold.level_dimension_types.get(&id).map(String::as_str),
                    &self.registries,
                    &self.world_clocks,
                    super::environment_attributes::ScalarWeather::from_level(&level.weather),
                ),
            });
            chunks.push((names.key(id), RefCell::new(&mut level.chunks)));
        }
        let maps = RefCell::new(&mut self.maps);
        let reads = EntityReads {
            names,
            levels,
            chunk_inputs: chunks
                .iter()
                .map(|(key, chunks)| (*key, LootChunks::Chunks(chunks)))
                .collect(),
            maps: Some(&maps),
            generate_structures: self.cold.generate_structures,
            resolver: OnceCell::new(),
            datapacks: self.cold.config.datapacks.as_ref(),
            item_tags: &self.cold.loot_item_tags,
            registries: &self.registries,
            clocks: &self.world_clocks,
            world_seed: self.cold.seed,
            storage: &self.cold.command_storage,
            scoreboard: self.scoreboard.board(),
            advancements: &self.cold.advancement_tree,
            loot: self
                .cold
                .config
                .datapacks
                .as_deref()
                .map(|startup| &startup.loot),
            entity_tags: tags
                .iter()
                .find(|(id, _)| {
                    *id == bbb_server_types::save::Java26BuiltinDimension::Overworld.into()
                })
                .and_then(|(_, tags)| tags.as_deref()),
            enchantment_registry: &self.datapack_tables.enchantment_registry,
            enchantment_slots: &self.datapack_tables.enchantment_slots,
        };
        f(&reads)
    }

    pub(crate) fn entity_reads(&self) -> EntityReads<'_> {
        EntityReads {
            names: &self.levels.names,
            levels: self
                .levels
                .iter()
                .map(|(id, level)| LevelRead {
                    key: self.level_key(id),
                    entities: &level.entities,
                    tags: level.chunks.registry_tags(),
                    random: &level.random,
                    weather: LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    },
                    sky_darkening: level.sky_darkening.map(i32::from),
                    raids: &level.raids,
                    environment_attributes: self
                        .cold
                        .dimension_sky_lights
                        .loot_environment_context(
                            self.cold.level_dimension_types.get(&id).map(String::as_str),
                            &self.registries,
                            &self.world_clocks,
                            super::environment_attributes::ScalarWeather::from_level(
                                &level.weather,
                            ),
                        ),
                })
                .collect(),
            chunk_inputs: self
                .levels
                .iter()
                .map(|(id, level)| (self.level_key(id), LootChunks::Shared(&level.chunks)))
                .collect(),
            maps: None,
            generate_structures: self.cold.generate_structures,
            resolver: OnceCell::new(),
            datapacks: self.cold.config.datapacks.as_ref(),
            item_tags: &self.cold.loot_item_tags,
            registries: &self.registries,
            clocks: &self.world_clocks,
            world_seed: self.cold.seed,
            storage: &self.cold.command_storage,
            scoreboard: self.scoreboard.board(),
            advancements: &self.cold.advancement_tree,
            loot: self
                .cold
                .config
                .datapacks
                .as_deref()
                .map(|startup| &startup.loot),
            entity_tags: self.biome_tags(),
            enchantment_registry: &self.datapack_tables.enchantment_registry,
            enchantment_slots: &self.datapack_tables.enchantment_slots,
        }
    }

    /// The online players in connection order, as commands list them.
    pub(crate) fn command_players(&self) -> impl Iterator<Item = CommandPlayer<'_>> + Clone {
        super::players_in_connection_order(self.levels.players())
            .map(|player| player.command_row(self.level_key(player.dimension)))
    }

    pub(crate) fn command_player_by_id(&self, id: uuid::Uuid) -> Option<CommandPlayer<'_>> {
        self.command_players().find(|player| player.id == id)
    }

    pub(crate) fn player_count(&self) -> usize {
        self.levels.player_count()
    }

    /// The served level filed under `key`.
    pub(crate) fn command_level(&self, key: &Java26LevelKey) -> Option<LevelId> {
        self.level_id(key)
    }

    /// `CommandSourceStack.getLevel()`'s blocks.
    pub(crate) fn command_blocks(&self, key: &Java26LevelKey) -> &ChunkStore {
        &self.level_by_id(&self.require_level_id(key)).chunks
    }

    /// `CommandSourceStack.getLevel().getWorldBorder()`.
    pub(crate) fn command_world_border(
        &self,
        key: &Java26LevelKey,
    ) -> bbb_server_types::interaction::WorldBorder {
        self.level_by_id(&self.require_level_id(key)).world_border
    }

    /// `ExecuteCommand.isChunkLoaded` (`ExecuteCommand.java:458-462`):
    /// `getChunkNow` present, `getFullStatus() == ENTITY_TICKING`, and
    /// `areEntitiesLoaded`.
    pub(crate) fn is_execute_chunk_loaded(
        &self,
        key: &Java26LevelKey,
        pos: bbb_protocol::packets::BlockPos,
    ) -> bool {
        let chunk = (pos.x >> 4, pos.z >> 4);
        self.level_id(key)
            .and_then(|id| self.levels.get(&id))
            .is_some_and(|level| {
                level
                    .chunk_lifecycle
                    .loading_levels
                    .get(&chunk)
                    .is_some_and(|&ticket| ticket <= super::CHUNK_ENTITY_TICKING_LEVEL)
                    && level.chunks.is_ticking_full(chunk.0, chunk.1)
                    && level.chunk_lifecycle.entities_loaded(chunk)
            })
    }

    pub(crate) fn registry_element_id(&self, registry: &str, id: i32) -> Option<&str> {
        self.registries.registry_element_id(registry, id)
    }

    /// The custom boss bars, each with the online players it shows to.
    #[cfg(test)]
    pub(crate) fn boss_bars(&self) -> Vec<crate::commands::CommandBossBarState> {
        let online: Vec<_> = self
            .levels
            .players()
            .map(|player| crate::admin::NameAndId {
                id: player.profile.uuid,
                name: (&player.profile.name).into(),
            })
            .collect();
        self.cold.boss_events.command_snapshot(&online)
    }

    pub(crate) fn has_boss_bar(&self, id: &str) -> bool {
        self.cold.boss_events.contains(id)
    }

    pub(crate) fn boss_bar_count(&self) -> usize {
        self.cold.boss_events.len()
    }

    pub(crate) fn boss_bar_ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.cold.boss_events.ids()
    }

    pub(crate) fn boss_bar_state(&self, id: &str) -> Option<&crate::commands::CommandBossBarState> {
        self.cold.boss_events.state(id)
    }

    pub(crate) fn boss_bar_states(
        &self,
    ) -> impl Iterator<Item = &crate::commands::CommandBossBarState> + '_ {
        self.cold.boss_events.states()
    }

    pub(crate) fn boss_bar_online_players(
        &self,
        id: &str,
    ) -> Option<(
        &crate::commands::CommandBossBarState,
        Vec<crate::admin::NameAndId>,
    )> {
        let state = self.cold.boss_events.state(id)?;
        let online = self
            .levels
            .players()
            .filter(|player| state.players.contains(&player.profile.uuid))
            .map(|player| crate::admin::NameAndId {
                id: player.profile.uuid,
                name: (&player.profile.name).into(),
            })
            .collect();
        Some((state, online))
    }

    pub(crate) fn advancement_tree(
        &self,
    ) -> &bbb_server_datapack::player_advancements::AdvancementTree {
        &self.cold.advancement_tree
    }

    /// `CommandStorage.get`, publishing the namespace on the read.
    pub(crate) fn storage_data(&self, id: &str) -> Option<NbtSummaryValue> {
        self.cold.command_storage.read(id)
    }

    /// `MinecraftServer.getProfileCache()` plus the online players.
    pub(crate) fn known_profiles(&self) -> Vec<crate::admin::NameAndId> {
        self.admin.known_profiles(&self.online_players())
    }

    pub(crate) fn admin_lists(&self) -> &crate::admin::StoredUserLists {
        self.admin.lists()
    }

    pub(crate) fn seed(&self) -> i64 {
        self.cold.seed
    }

    pub(crate) fn tick_times_nanos(&self) -> &[i64] {
        &self.cold.tick_times_nanos
    }

    /// The `minecraft:damage_type` registry in registry order.
    pub(crate) fn damage_types(&self) -> &[bbb_server_datapack::damage::DamageTypeFacts] {
        &self.cold.damage_types
    }

    pub(crate) fn enchantments(
        &self,
    ) -> Arc<Vec<bbb_server_datapack::enchantment::EnchantmentFacts>> {
        use bbb_server_datapack::enchantment::{load_enchantment_facts, EnchantmentFacts};
        use bbb_server_datapack::startup::DatapackStartup;
        use std::sync::{Mutex, OnceLock, Weak};

        type CachedPackEnchantments = (Weak<DatapackStartup>, Arc<Vec<EnchantmentFacts>>);
        static BUILTIN: OnceLock<Arc<Vec<EnchantmentFacts>>> = OnceLock::new();
        static PACK_CACHE: OnceLock<Mutex<Option<CachedPackEnchantments>>> = OnceLock::new();

        let Some(packs) = self.cold.config.datapacks.as_ref() else {
            return Arc::clone(BUILTIN.get_or_init(|| load_enchantment_facts(None)));
        };
        let mut cache = PACK_CACHE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current = Arc::downgrade(packs);
        if let Some((cached_packs, facts)) = cache.as_ref() {
            if cached_packs.ptr_eq(&current) {
                return Arc::clone(facts);
            }
        }
        let facts = load_enchantment_facts(Some(packs.as_ref()));
        *cache = Some((current, Arc::clone(&facts)));
        facts
    }

    pub(crate) fn debug_trace_directory(&self) -> &std::path::Path {
        &self.cold.config.debug_trace_directory
    }

    pub(crate) fn registry_contains(&self, registry: &str, id: &str) -> bool {
        self.registries
            .registry_element_index(registry, id)
            .is_some()
    }

    pub(crate) fn biome_tag_exists(&self, tag: &str) -> bool {
        self.biome_tags()
            .is_some_and(|tags| tags.tag_exists(BIOME_REGISTRY, tag))
    }

    pub(crate) fn biome_in_tag(&self, tag: &str, biome: &str) -> bool {
        self.biome_tags()
            .is_some_and(|tags| tags.contains(BIOME_REGISTRY, tag, biome))
    }

    /// The served levels' keys in level order.
    pub(crate) fn command_level_keys(&self) -> impl Iterator<Item = &Java26LevelKey> {
        self.levels.iter().map(|(id, _)| self.level_key(id))
    }

    /// `TimerQueue.getEventsIds()`.
    pub(crate) fn scheduled_event_ids(&self) -> Vec<bbb_protocol::JavaString> {
        self.cold
            .scheduled_events
            .project()
            .into_iter()
            .map(|event| event.id)
            .collect()
    }

    /// `RandomSequences` keys, which `/random` suggests.
    pub(crate) fn random_sequence_ids(&self) -> Vec<String> {
        self.cold
            .random_sequences
            .borrow()
            .entries()
            .map(|(key, _)| key.to_string())
            .collect()
    }

    /// `CommandStorage.keys()`.
    pub(crate) fn storage_keys(&self) -> Vec<String> {
        self.cold.command_storage.keys().collect()
    }

    pub(crate) fn scoreboard(&self) -> &Scoreboard {
        self.scoreboard.board()
    }

    /// The overworld's datapack tags, which `#biome` and `type=#tag` read.
    pub(crate) fn biome_tags(&self) -> Option<&crate::registry_tags::RegistryTagSnapshot> {
        self.level_if_present(bbb_server_types::save::Java26BuiltinDimension::Overworld)
            .and_then(|level| level.chunks.registry_tags())
    }

    /// `level.dimensionTypeRegistration()` and the facts it registers.
    pub(crate) fn command_dimension_type(
        &self,
        key: &Java26LevelKey,
    ) -> (&str, DimensionTypeRuntimeFacts) {
        let element_id = self
            .cold
            .level_dimension_types
            .get(&self.require_level_id(key))
            .expect("every served level has a dimension type");
        let facts = match self.cold.config.datapacks.as_deref() {
            Some(startup) => dimension_type_runtime_facts(&startup.registries, element_id),
            None => builtin_dimension_type_runtime_facts(element_id),
        }
        .expect("a served level has a loaded dimension type");
        (element_id, facts)
    }

    pub(crate) fn registry_tag_ids(&self, registry: &str) -> impl Iterator<Item = &str> {
        self.registries.registry_tags().tag_ids(registry)
    }

    /// Ids of one published dynamic registry, in registry order.
    pub(crate) fn registry_ids(&self, registry: &str) -> Vec<String> {
        (0..self.registries.registry_len(registry).unwrap_or(0))
            .filter_map(|index| {
                self.registries
                    .registry_element_id(registry, index as i32)
                    .map(str::to_owned)
            })
            .collect()
    }

    /// Ids of one datapack registry as loaded at startup.
    pub(crate) fn startup_registry_ids(&self, registry: &str) -> Vec<String> {
        self.cold
            .config
            .datapacks
            .as_deref()
            .and_then(|startup| startup.registries.registry(registry))
            .map(|registry| {
                registry
                    .elements
                    .iter()
                    .map(|element| element.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn recipe_ids(&self) -> Vec<String> {
        self.cold
            .config
            .datapacks
            .as_deref()
            .map(|startup| {
                startup
                    .recipes
                    .recipes
                    .iter()
                    .map(|element| element.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn template_ids(&self) -> Vec<String> {
        self.cold.structure_templates.list_templates(
            self.cold
                .config
                .datapacks
                .as_deref()
                .map(|startup| &startup.structures),
        )
    }

    pub(crate) fn check_item_modifier(
        &self,
        function: &bbb_server_datapack::loot::LootFunction,
    ) -> Result<(), crate::loot_runtime::LootRuntimeError> {
        let builtin;
        let startup = match self.cold.config.datapacks.as_deref() {
            Some(startup) => startup,
            None => {
                builtin = crate::registry_sync::builtin_datapack_startup_shared();
                &builtin
            }
        };
        crate::loot_runtime::check_modifier_codec(function, &startup.loot)
    }

    /// One loot registry's elements, empty without a datapack.
    pub(crate) fn loot_elements(
        &self,
        registry: &str,
    ) -> &[bbb_server_datapack::loot::LootElement] {
        self.cold
            .config
            .datapacks
            .as_deref()
            .and_then(|startup| startup.loot.registry(registry))
            .map_or(&[][..], |registry| registry.elements.as_slice())
    }

    pub(super) fn player_save_without_id(
        &self,
        player: &PlayerState,
        item_stack_resolver: &PlayerItemStackResolver,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        super::player_save::player_save_without_id_with_sources(
            player,
            self.level_key(player.dimension),
            item_stack_resolver,
            self.player_root_vehicle(player),
            self.player_ender_pearls(player.profile.uuid),
        )
    }

    pub(super) fn command_player_data(
        &self,
        player: &PlayerState,
        item_stack_resolver: &PlayerItemStackResolver,
    ) -> bbb_server_types::Result<NbtSummaryValue> {
        super::player_save::command_player_data_with_sources(
            player,
            self.level_key(player.dimension),
            item_stack_resolver,
            self.player_root_vehicle(player),
            self.player_ender_pearls(player.profile.uuid),
        )
    }

    fn player_root_vehicle(
        &self,
        player: &PlayerState,
    ) -> bbb_server_types::save::SaveResult<Option<bbb_server_persistence::Java26RootVehicle>> {
        if player.avatar_removed {
            return Ok(None);
        }
        crate::entity::save::write_external_player_root_vehicle_value(
            &self.level_by_id(&player.dimension).entities,
            player.entity_id(),
        )
        .map_err(|error| bbb_server_types::save::SaveError::InvalidNbt {
            path: None,
            detail: format!("player {} RootVehicle: {error}", player.profile.uuid),
        })
    }

    /// `PackRepository.getAvailablePacks` and `getSelectedPacks` plus the
    /// world's `enabledFeatures`, as the `/datapack` readers ask for them.
    pub(crate) fn command_datapacks(&self) -> CommandDatapacks {
        match self.cold.config.datapacks.as_deref() {
            Some(startup) => CommandDatapacks {
                available: startup.available.iter().map(command_datapack).collect(),
                selected: startup
                    .listed_enabled()
                    .into_iter()
                    .map(command_datapack)
                    .collect(),
                enabled_features: startup.enabled_features.iter().cloned().collect(),
            },
            None => CommandDatapacks {
                available: vec![vanilla_command_datapack()],
                selected: vec![vanilla_command_datapack()],
                enabled_features: bbb_server_datapack::startup::default_enabled_features_26_1()
                    .into_iter()
                    .collect(),
            },
        }
    }
}

fn vanilla_command_datapack() -> CommandDatapack {
    CommandDatapack {
        id: "vanilla".to_string(),
        source: bbb_server_datapack::pack::ServerPackSource::BuiltIn,
        description: CommandPackDescription::Vanilla,
        requested_features: Vec::new(),
    }
}

fn command_datapack(
    listing: &bbb_server_datapack::pack_repository::DatapackListing,
) -> CommandDatapack {
    CommandDatapack {
        id: listing.id.clone(),
        source: listing.source,
        description: if listing.id == "vanilla" {
            CommandPackDescription::Vanilla
        } else {
            CommandPackDescription::Literal(listing.description_text())
        },
        requested_features: listing.requested_features.iter().cloned().collect(),
    }
}

impl PlayerState {
    pub(in crate::world) fn command_row<'a>(
        &'a self,
        dimension: &'a Java26LevelKey,
    ) -> CommandPlayer<'a> {
        CommandPlayer {
            id: self.profile.uuid,
            name: &self.profile.name,
            game_type: self.game_type,
            position: [self.position().x, self.position().y, self.position().z],
            rotation: [self.rotation().y_rot, self.rotation().x_rot],
            pose_id: self.pose_id,
            eye_height: self
                .eye_height()
                .expect("player eye height is in the catalog"),
            dimension,
            address: self.cold.remote_address.as_deref(),
            entity_id: self.entity_id(),
            damage: self.damage,
            removed: self.avatar_removed,
            tags: &self.cold.tags,
            camera: self.cold.camera.as_ref(),
            experience: self.experience,
            attributes: &self.attributes,
            inventory: crate::commands::CommandInventory(&self.inventory),
            effects: &self.effects,
            advancements: &self.cold.advancements,
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    /// `/data get entity` for every player and stored entity, for the tests
    /// that pin what an entity's NBT carries.
    pub(crate) fn command_entity_data_snapshot_for_test(
        &self,
    ) -> Vec<crate::commands::CommandEntityDataState> {
        let reads = self.entity_reads();
        reads
            .players()
            .map(|player| player.id)
            .chain(reads.stored_entities().map(|stored| stored.entity.uuid()))
            .map(|uuid| crate::commands::CommandEntityDataState {
                uuid,
                data: reads
                    .entity_data(uuid)
                    .expect("entity command data")
                    .expect("the uuid names a live entity"),
            })
            .collect()
    }
}

#[cfg(test)]
mod test_fixture;
#[cfg(test)]
pub(crate) use test_fixture::CommandTestWorld;

#[cfg(test)]
mod test_clocks;

#[cfg(test)]
mod test_functions;

#[cfg(test)]
mod test_scalar;

#[cfg(test)]
mod test_execute;
#[cfg(test)]
mod test_server;

#[cfg(test)]
mod test_datapacks;

#[cfg(test)]
mod test_bossbar;
#[cfg(test)]
mod test_waypoint;
