use super::entity_scoreboard::LootEntityLevels;
use super::*;
use crate::entity::store::EntityBodyReadView;
use crate::entity::EntityStore;
use crate::loot_runtime::LootRuntimeReason;
use crate::world::LevelId;
use bbb_protocol::NbtSummaryValue;

pub(in crate::world) struct PlayerLootNbtSources<'a> {
    pub snapshot: Option<&'a EntityNbtReadSnapshot>,
    pub resolver: Option<&'a PlayerItemStackResolver>,
    pub dimension: Option<&'a LevelId>,
    pub bodies: Option<EntityBodyReadView<'a>>,
    pub other_levels: LootEntityLevels<'a>,
}

impl PlayerLootNbtSources<'_> {
    fn player(&self, uuid: Uuid) -> Option<(&PlayerState, &EntityStore)> {
        self.bodies
            .and_then(|bodies| {
                let store = bodies.store();
                store.player_by_uuid(uuid).map(|player| (player, store))
            })
            .or_else(|| match self.other_levels {
                LootEntityLevels::Stores(stores) => stores.iter().find_map(|(_, store)| {
                    store.player_by_uuid(uuid).map(|player| (player, *store))
                }),
                LootEntityLevels::Other(levels) => levels.iter().find_map(|(_, level)| {
                    level
                        .entities
                        .player_by_uuid(uuid)
                        .map(|player| (player, &level.entities))
                }),
                LootEntityLevels::World(levels) => levels.values().find_map(|level| {
                    level
                        .entities
                        .player_by_uuid(uuid)
                        .map(|player| (player, &level.entities))
                }),
                LootEntityLevels::Reads(levels) => levels.iter().find_map(|level| {
                    level
                        .entities
                        .player_by_uuid(uuid)
                        .map(|player| (player, level.entities))
                }),
            })
    }

    pub(in crate::world) fn slot(
        &self,
        uuid: Uuid,
        slot: i32,
    ) -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason> {
        let (player, store) = self.player(uuid).ok_or_else(|| {
            LootRuntimeReason::Catalog("player slots need an online player".into())
        })?;
        Ok(read_player_slot(player, slot, store.enchantment_registry()))
    }

    pub(in crate::world) fn read(&self, uuid: Uuid) -> Result<NbtSummaryValue, LootRuntimeReason> {
        let missing = |fact| LootRuntimeReason::Catalog(format!("player loot NBT needs {fact}"));
        read_player_nbt_with_levels(
            uuid,
            self.snapshot
                .ok_or_else(|| missing("the NBT publication"))?
                .level_names(),
            self.resolver
                .ok_or_else(|| missing("the item publication"))?,
            self.dimension,
            self.bodies,
            self.other_levels,
        )
    }
}

pub(in crate::world) fn read_player_nbt_with_levels<'a>(
    uuid: Uuid,
    names: &'a crate::world::level_keys::LevelNames,
    resolver: &PlayerItemStackResolver,
    dimension: Option<&LevelId>,
    current_bodies: Option<EntityBodyReadView<'a>>,
    other_levels: super::entity_scoreboard::LootEntityLevels<'a>,
) -> Result<NbtSummaryValue, LootRuntimeReason> {
    let missing = |fact| LootRuntimeReason::Catalog(format!("player loot NBT needs {fact}"));
    let player = current_bodies
        .map(|bodies| bodies.store())
        .into_iter()
        .chain(other_levels.stores(names).map(|(_, store)| store))
        .find_map(|store| store.player_by_uuid(uuid))
        .ok_or_else(|| missing("an online player"))?;
    let bodies = if dimension == Some(&player.dimension) {
        current_bodies.ok_or_else(|| missing("the current entity bodies"))?
    } else {
        let store = other_levels
            .stores(names)
            .find(|(dimension, _)| *dimension == player.dimension)
            .map(|(_, store)| store)
            .ok_or_else(|| missing("the player's level entities"))?;
        EntityBodyReadView::new(store, None, None)
    };
    let root_vehicle = if player.avatar_removed {
        Ok(None)
    } else {
        crate::entity::save::write_external_player_root_vehicle_value_with_view(
            bodies,
            player.entity_id(),
        )
        .map_err(|error| bbb_server_types::save::SaveError::InvalidNbt {
            path: None,
            detail: format!("player {} RootVehicle: {error}", player.profile.uuid),
        })
    };
    super::super::player_save::command_player_data_with_sources(
        player,
        names.key(player.dimension),
        resolver,
        root_vehicle,
        super::super::player_ender_pearls::saved_player_ender_pearls(
            player.profile.uuid,
            current_bodies
                .zip(dimension)
                .map(|(bodies, dimension)| (names.key(*dimension), bodies))
                .into_iter()
                .chain(other_levels.stores(names).map(|(dimension, store)| {
                    (
                        names.key(dimension),
                        EntityBodyReadView::new(store, None, None),
                    )
                })),
        ),
    )
    .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
}

pub(in crate::world) fn read_player_slot(
    player: &PlayerState,
    slot: i32,
    enchantments: &[crate::enchantment_facts::EnchantmentSlots],
) -> Option<crate::loot_runtime::LootTool> {
    read_inventory_slot(&player.inventory, slot, enchantments)
}

pub(in crate::world) fn read_inventory_slot(
    inventory: &crate::inventory::PlayerInventoryMenu,
    slot: i32,
    enchantments: &[crate::enchantment_facts::EnchantmentSlots],
) -> Option<crate::loot_runtime::LootTool> {
    crate::commands::CommandInventory(inventory)
        .slot(slot)
        .map(|stack| {
            crate::entity::mob_death_loot::loot_tool_for_persistent_stack(&stack, enchantments)
        })
}

impl ServerWorld {
    pub(in crate::world) fn player_loot_nbt(
        &self,
        uuid: Uuid,
    ) -> Result<NbtSummaryValue, LootRuntimeReason> {
        let player = self
            .player_reference(uuid)
            .and_then(|connection| self.levels.player(connection))
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("player NBT requires an online player".into())
            })?;
        // NbtPredicate.java:42-55: saveWithoutId plus a nonempty SelectedItem.
        self.command_player_data(player, &self.item_stack_resolver())
            .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
    }

    pub(in crate::world) fn player_loot_slot(
        &self,
        current: Option<&PlayerState>,
        uuid: Uuid,
        slot: i32,
    ) -> Result<Option<crate::loot_runtime::LootTool>, LootRuntimeReason> {
        let player = current
            .filter(|player| player.profile.uuid == uuid)
            .or_else(|| {
                self.levels
                    .values()
                    .find_map(|level| level.entities.player_by_uuid(uuid))
            })
            .ok_or_else(|| {
                LootRuntimeReason::Catalog("player slots need an online player".into())
            })?;
        Ok(read_player_slot(
            player,
            slot,
            &self.datapack_tables.enchantment_slots,
        ))
    }
}
