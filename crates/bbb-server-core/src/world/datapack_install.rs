//! Installing a rebuilt datapack generation into the running world.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::SystemChat;
use bbb_server_datapack::recipe::ingredient::IngredientTerm;
use bbb_server_datapack::recipe::{CookingKind, FuelValues, ItemTagIndex};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::trades::{parse_trade_set, TradeSet};
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock, RwLock};

use crate::commands;
use crate::enchantment_facts::{
    load_enchantment_definitions, load_enchantment_registry, load_enchantment_slots,
    load_enchantment_value_effects, load_enchantments_with_effect, load_equipment_drop_effects,
    EnchantmentDefinition, EnchantmentSlots, PREVENT_ARMOR_CHANGE_EFFECT,
    PREVENT_EQUIPMENT_DROP_EFFECT,
};
use crate::enchantment_location_effects::{
    load_enchantment_location_effects, load_enchantment_tick_effects, EnchantmentEffectTable,
    EnchantmentLocationTable, EnchantmentTickTable,
};
use crate::item_durability::{
    EnchantmentValueEffects, BLOCK_EXPERIENCE_COMPONENT, FISHING_LUCK_BONUS_COMPONENT,
    FISHING_TIME_REDUCTION_COMPONENT, ITEM_DAMAGE_COMPONENT, REPAIR_WITH_XP_COMPONENT,
};
use crate::registry_tags::RegistryTagSnapshot;

use super::{load_damage_type_registry, send, ReloadedDatapacks, ServerWorld, WorldEvent};
use crate::connection::ClientboundPlayPacket;

pub(in crate::world) type FurnaceAdmissionSets = (Arc<BTreeSet<i32>>, Arc<BTreeSet<i32>>);
type TradeSetCache = RwLock<FxHashMap<String, Result<Arc<TradeSet>, String>>>;

#[derive(Clone)]
pub(in crate::world) struct DatapackRuntimeTables {
    /// The `minecraft:enchantment` registry in `RegistryDataLoader` order,
    /// matching decoded `minecraft:enchantments` component `holder_id`s.
    pub(in crate::world) enchantment_registry: Arc<Vec<String>>,
    /// Maps enchantment resource id to its `holder_id` index.
    enchantment_index_by_id: Arc<FxHashMap<String, usize>>,
    /// Per-`holder_id` bitmask of `EquipmentSlot`s where the enchantment applies.
    enchantment_slot_masks: Arc<Vec<u8>>,
    pub(in crate::world) equipment_drop_effects:
        Arc<Vec<Option<crate::equipment_drop_effects::EquipmentDropEffects>>>,
    pub(in crate::world) enchantment_entity_effects:
        Arc<crate::enchantment_entity_effects::EnchantmentEntityEffectTable>,
    pub(in crate::world) enchantment_combat_effects:
        Arc<crate::enchantment_combat_effects::EnchantmentCombatTable>,
    /// `Enchantment.EnchantmentDefinition.slots()` in registry order.
    pub(in crate::world) enchantment_slots: Arc<Vec<EnchantmentSlots>>,
    pub(in crate::world) enchantment_location_effects: Arc<EnchantmentLocationTable>,
    pub(in crate::world) enchantment_tick_effects: Arc<EnchantmentTickTable>,
    /// Bitset of enchantment `holder_id`s present in `enchantment_location_effects`.
    location_effect_holders: Arc<Vec<u64>>,
    /// Bitset of enchantment `holder_id`s present in `enchantment_tick_effects`.
    tick_effect_holders: Arc<Vec<u64>>,
    pub(in crate::world) prevent_armor_change_enchantments: Arc<BTreeSet<String>>,
    /// Enchantment `holder_id`s carrying `PREVENT_ARMOR_CHANGE`, reused on container clicks.
    pub(in crate::world) armor_change_locks: Arc<BTreeSet<i32>>,
    /// Cached utility-menu enchantment definitions for this generation.
    enchantment_definitions: OnceLock<Arc<Vec<EnchantmentDefinition>>>,
    /// Cached furnace admission sets indexed by `Smelting`/`Blasting`/`Smoking`.
    furnace_admissions: [OnceLock<Option<FurnaceAdmissionSets>>; 3],
    /// Cached `minecraft:brewing_fuel` item ids for this generation.
    brewing_fuel_items: OnceLock<Option<Arc<BTreeSet<i32>>>>,
    /// Parsed `minecraft:trade_set` definitions cached by resource id for this generation.
    trade_sets: Arc<TradeSetCache>,
    pub(in crate::world) enchantment_item_damage: Arc<Vec<Option<EnchantmentValueEffects>>>,
    pub(in crate::world) enchantment_block_experience: Arc<Vec<Option<EnchantmentValueEffects>>>,
    pub(in crate::world) enchantment_repair_with_xp: Arc<Vec<Option<EnchantmentValueEffects>>>,
    pub(in crate::world) enchantment_fishing_time_reduction:
        Arc<Vec<Option<EnchantmentValueEffects>>>,
    pub(in crate::world) enchantment_fishing_luck_bonus: Arc<Vec<Option<EnchantmentValueEffects>>>,
    pub(in crate::world) enchantment_mob_experience: Arc<Vec<Option<EnchantmentValueEffects>>>,
}

const fn equipment_slot_bit(slot: EquipmentSlot) -> u8 {
    1u8 << (slot as u8)
}

fn holder_bitset<T>(registry: &[String], table: &EnchantmentEffectTable<T>) -> Vec<u64> {
    if table.is_empty() || registry.is_empty() {
        return Vec::new();
    }
    let mut bits = vec![0u64; registry.len().div_ceil(64)];
    for (holder_id, id) in registry.iter().enumerate() {
        if table.contains_key(id) {
            bits[holder_id / 64] |= 1u64 << (holder_id % 64);
        }
    }
    bits
}

fn bitset_contains(bits: &[u64], holder_id: usize) -> bool {
    bits.get(holder_id / 64)
        .is_some_and(|word| (word & (1u64 << (holder_id % 64))) != 0)
}

impl DatapackRuntimeTables {
    pub(in crate::world) fn load(datapacks: Option<&Arc<DatapackStartup>>) -> Arc<Self> {
        let deref = datapacks.map(Arc::as_ref);
        let enchantment_registry = load_enchantment_registry(deref);
        let enchantment_slots = load_enchantment_slots(deref);
        let enchantment_index_by_id = enchantment_slots
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.resource_id.clone(), index))
            .collect::<FxHashMap<_, _>>();
        let enchantment_slot_masks = enchantment_slots
            .iter()
            .map(|entry| {
                entry
                    .slots
                    .iter()
                    .fold(0u8, |mask, &slot| mask | equipment_slot_bit(slot))
            })
            .collect::<Vec<_>>();
        let enchantment_location_effects = load_enchantment_location_effects(deref);
        let enchantment_tick_effects = load_enchantment_tick_effects(deref);
        let location_effect_holders =
            holder_bitset(&enchantment_registry, &enchantment_location_effects);
        let tick_effect_holders = holder_bitset(&enchantment_registry, &enchantment_tick_effects);
        let prevent_armor_change_enchantments =
            load_enchantments_with_effect(deref, PREVENT_ARMOR_CHANGE_EFFECT);
        let armor_change_locks = (0_i32..)
            .zip(enchantment_slots.iter())
            .filter(|(_, enchantment)| {
                prevent_armor_change_enchantments.contains(&enchantment.resource_id)
            })
            .map(|(holder_id, _)| holder_id)
            .collect::<BTreeSet<_>>();
        Arc::new(Self {
            enchantment_registry,
            enchantment_index_by_id: Arc::new(enchantment_index_by_id),
            enchantment_slot_masks: Arc::new(enchantment_slot_masks),
            equipment_drop_effects: load_equipment_drop_effects(deref),
            enchantment_entity_effects:
                crate::enchantment_entity_effects::load_enchantment_entity_effects(deref),
            enchantment_combat_effects:
                crate::enchantment_combat_effects::load_enchantment_combat_effects(deref),
            enchantment_slots,
            enchantment_location_effects,
            enchantment_tick_effects,
            location_effect_holders: Arc::new(location_effect_holders),
            tick_effect_holders: Arc::new(tick_effect_holders),
            prevent_armor_change_enchantments,
            armor_change_locks: Arc::new(armor_change_locks),
            enchantment_definitions: OnceLock::new(),
            furnace_admissions: [OnceLock::new(), OnceLock::new(), OnceLock::new()],
            brewing_fuel_items: OnceLock::new(),
            trade_sets: Arc::new(RwLock::new(FxHashMap::default())),
            enchantment_item_damage: load_enchantment_value_effects(
                datapacks,
                ITEM_DAMAGE_COMPONENT,
            ),
            enchantment_block_experience: load_enchantment_value_effects(
                datapacks,
                BLOCK_EXPERIENCE_COMPONENT,
            ),
            enchantment_repair_with_xp: load_enchantment_value_effects(
                datapacks,
                REPAIR_WITH_XP_COMPONENT,
            ),
            enchantment_fishing_time_reduction: load_enchantment_value_effects(
                datapacks,
                FISHING_TIME_REDUCTION_COMPONENT,
            ),
            enchantment_fishing_luck_bonus: load_enchantment_value_effects(
                datapacks,
                FISHING_LUCK_BONUS_COMPONENT,
            ),
            enchantment_mob_experience: load_enchantment_value_effects(
                datapacks,
                crate::item_durability::MOB_EXPERIENCE_COMPONENT,
            ),
        })
    }

    pub(in crate::world) fn enchantment_holder_id(&self, id: &str) -> Option<usize> {
        self.enchantment_index_by_id.get(id).copied()
    }

    pub(in crate::world) fn enchantment_matches_slot(&self, id: &str, slot: EquipmentSlot) -> bool {
        self.enchantment_holder_id(id)
            .is_some_and(|holder_id| self.holder_matches_slot(holder_id, slot))
    }

    pub(in crate::world) fn holder_matches_slot(
        &self,
        holder_id: usize,
        slot: EquipmentSlot,
    ) -> bool {
        self.enchantment_slot_masks
            .get(holder_id)
            .is_some_and(|mask| (mask & equipment_slot_bit(slot)) != 0)
    }

    pub(in crate::world) fn has_tick_effects(&self) -> bool {
        !self.tick_effect_holders.is_empty()
    }

    pub(in crate::world) fn has_location_effects(&self) -> bool {
        !self.location_effect_holders.is_empty()
    }

    pub(in crate::world) fn holder_has_tick_effect(&self, holder_id: usize) -> bool {
        bitset_contains(&self.tick_effect_holders, holder_id)
    }

    pub(in crate::world) fn holder_has_location_effect(&self, holder_id: usize) -> bool {
        bitset_contains(&self.location_effect_holders, holder_id)
    }

    pub(in crate::world) fn enchantment_definitions(
        &self,
        datapacks: Option<&DatapackStartup>,
        registry_tags: &RegistryTagSnapshot,
    ) -> Arc<Vec<EnchantmentDefinition>> {
        Arc::clone(
            self.enchantment_definitions
                .get_or_init(|| load_enchantment_definitions(datapacks, registry_tags)),
        )
    }

    pub(in crate::world) fn furnace_admission_sets(
        &self,
        cooking_kind: CookingKind,
        datapacks: Option<&DatapackStartup>,
        tags: Option<&ItemTagIndex>,
        fuel_values: &FuelValues,
    ) -> Option<FurnaceAdmissionSets> {
        let slot = match cooking_kind {
            CookingKind::Smelting => 0,
            CookingKind::Blasting => 1,
            CookingKind::Smoking => 2,
            CookingKind::Campfire => return None,
        };
        let (datapacks, tags) = (datapacks?, tags?);
        self.furnace_admissions[slot]
            .get_or_init(|| {
                let mut inputs = BTreeSet::new();
                for element in &datapacks.recipes.recipes {
                    let Some(recipe) = element.recipe.cooking_of_kind(cooking_kind) else {
                        continue;
                    };
                    for term in &recipe.ingredient.terms {
                        match term {
                            IngredientTerm::Item(item) => {
                                inputs.insert(*item);
                            }
                            IngredientTerm::Tag(tag) => {
                                inputs.extend(tags.members(tag));
                            }
                        }
                    }
                }
                let fuels: BTreeSet<i32> = fuel_values.fuel_items().collect();
                Some((Arc::new(inputs), Arc::new(fuels)))
            })
            .clone()
    }

    pub(in crate::world) fn brewing_fuel_items(
        &self,
        tags: Option<&ItemTagIndex>,
    ) -> Option<Arc<BTreeSet<i32>>> {
        let tags = tags?;
        self.brewing_fuel_items
            .get_or_init(|| {
                Some(Arc::new(
                    tags.members("minecraft:brewing_fuel")
                        .collect::<BTreeSet<i32>>(),
                ))
            })
            .clone()
    }

    pub(in crate::world) fn trade_set(
        &self,
        startup: &DatapackStartup,
        id: &str,
    ) -> Result<Option<Arc<TradeSet>>, String> {
        if let Ok(cache) = self.trade_sets.read() {
            if let Some(entry) = cache.get(id) {
                return entry.clone().map(Some);
            }
        }
        let Some(raw) = startup.registries.element("minecraft:trade_set", id) else {
            return Ok(None);
        };
        let parsed = raw
            .json()
            .ok_or_else(|| format!("{id}: legacy registry JSON view is unavailable"))
            .and_then(|json| {
                parse_trade_set(json)
                    .map(Arc::new)
                    .map_err(|error| format!("{id}: {error}"))
            });
        if let Ok(mut cache) = self.trade_sets.write() {
            cache.insert(id.to_owned(), parsed.clone());
        }
        parsed.map(Some)
    }
}

impl ServerWorld {
    /// The crafting menus' item tags: the generation's item tags plus its
    /// `#minecraft:curse` enchantments, or `None` without an item tag registry,
    /// where an invented empty index would silently mismatch tagged ingredients.
    pub(crate) fn crafting_item_tags_of(
        datapacks: Option<&bbb_server_datapack::startup::DatapackStartup>,
    ) -> Option<Arc<bbb_server_datapack::recipe::ItemTagIndex>> {
        let datapacks = datapacks?;
        let item_tags = datapacks.tags.registry("minecraft:item")?;
        Some(Arc::new(
            bbb_server_datapack::recipe::ItemTagIndex::from_loaded_with_content_registry(
                item_tags,
                datapacks.content_registry(),
            )
            .with_enchantment_curses(datapacks),
        ))
    }

    /// Installs a rebuilt datapack generation, or keeps the one in place.
    ///
    /// `MinecraftServer.reloadResources` replaces the resources on the server
    /// thread once the new generation is complete; a build that failed leaves
    /// the running one untouched and `ReloadCommand.reloadPacks` reports
    /// `commands.reload.failure`. Failure reaches everyone who could have asked
    /// for the reload, because the answer arrives long after the command did
    /// and the world no longer knows who that was.
    pub(super) fn install_datapacks(
        &mut self,
        startup: ReloadedDatapacks,
        events: &mut Vec<WorldEvent>,
    ) {
        match startup {
            ReloadedDatapacks::Built(startup) => {
                if self.cold.config.datapacks.as_ref().is_some_and(|previous| {
                    !previous
                        .registries
                        .same_frozen_registries(&startup.registries)
                }) {
                    self.broadcast(
                        ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                            commands::reload_failure(),
                            false,
                        )),
                        events,
                    );
                    return;
                }
                let Ok(registries) = self.registries.reload_tags(&startup) else {
                    self.broadcast(
                        ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                            commands::reload_failure(),
                            false,
                        )),
                        events,
                    );
                    return;
                };
                if self
                    .cold
                    .level_dimension_types
                    .values()
                    .any(|element_id| registries.dimension_type_index(element_id).is_none())
                {
                    self.broadcast(
                        ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                            commands::reload_failure(),
                            false,
                        )),
                        events,
                    );
                    return;
                }
                if bbb_protocol::packets::encode_play_clientbound_update_tags(
                    registries.published_tags(),
                )
                .is_err()
                {
                    self.broadcast(
                        ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                            commands::reload_failure(),
                            false,
                        )),
                        events,
                    );
                    return;
                }
                // Reloadable resources finish preparing before the server-thread install
                // (MinecraftServer.java:1504-1544; ReloadableServerResources.java:76-105).
                let Ok(noise_resources) = self.prepare_noise_reloadable_resources(&startup) else {
                    self.broadcast(
                        ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                            commands::reload_failure(),
                            false,
                        )),
                        events,
                    );
                    return;
                };
                let registry_tags = registries.registry_tags_shared();
                self.install_item_stack_registry_view(&registries);
                self.install_noise_reloadable_resources(noise_resources);
                self.install_level_registry_tags(registry_tags);
                let content_registry = registries.content_registry();
                self.cold.loot_item_tags = startup
                    .tags
                    .registry("minecraft:item")
                    .map(|tags| {
                        bbb_server_datapack::recipe::ItemTagIndex::from_loaded_with_content_registry(
                            tags,
                            content_registry,
                        )
                    })
                    .unwrap_or_default();
                self.cold.crafting_item_tags = Self::crafting_item_tags_of(Some(&*startup));
                self.cold.fuel_values =
                    bbb_server_datapack::recipe::FuelValues::vanilla_burn_times_with_content_registry(
                        &self.cold.loot_item_tags,
                        content_registry,
                    )
                    .values;
                self.cold.config.datapacks = Some(startup);
                self.cold.structure_templates.on_resource_reload();
                self.registries = registries;
                self.refresh_saved_column_resources();
                // `saveAll` then `reloadResources` (`MinecraftServer.java:1539-1540`,
                // `PlayerList.java:841-844`); the write-back is `to_data`, not the json.
                self.reload_player_advancement_stores();
                self.broadcast(
                    ClientboundPlayPacket::UpdateTags(self.registries.published_tags().clone()),
                    events,
                );
                let connections = self.levels.player_keys().copied().collect::<Vec<_>>();
                if let Some(datapacks) = self.cold.config.datapacks.clone() {
                    for connection_id in &connections {
                        let player = self
                            .levels
                            .player_mut(*connection_id)
                            .expect("connected player");
                        Self::reload_player_crafting_catalog(
                            &mut player.inventory,
                            &datapacks,
                            self.cold.crafting_item_tags.as_ref(),
                        );
                    }
                    // PlayerList.reloadResources (`PlayerList.java:841-855`).
                    for connection_id in connections {
                        events.push(send(
                            connection_id,
                            ClientboundPlayPacket::UpdateRecipes(
                                datapacks.recipe_displays.update_recipes().clone(),
                            ),
                        ));
                        self.send_initial_recipe_book(connection_id, events);
                    }
                }
                self.cold.damage_types =
                    load_damage_type_registry(self.cold.config.datapacks.as_deref());
                self.datapack_tables =
                    DatapackRuntimeTables::load(self.cold.config.datapacks.as_ref());
                // `MinecraftServer.reloadResources` hands the rebuilt library
                // to `ServerFunctionManager.replaceLibrary`
                // (ServerFunctionManager.java:78-81), which arms `postReload`
                // and so runs `minecraft:load` on the next tick.
                if let Some(datapacks) = self.cold.config.datapacks.clone() {
                    self.replace_function_library(&datapacks.functions);
                }
                let prevent_equipment_drop_enchantments = load_enchantments_with_effect(
                    self.cold.config.datapacks.as_deref(),
                    PREVENT_EQUIPMENT_DROP_EFFECT,
                );
                let prevent_armor_change_enchantments =
                    Arc::clone(&self.datapack_tables.prevent_armor_change_enchantments);
                let enchantment_attributes =
                    crate::enchantment_facts::load_enchantment_attribute_effects(
                        self.cold.config.datapacks.as_deref(),
                    );
                let enchantment_location_effects =
                    Arc::clone(&self.datapack_tables.enchantment_location_effects);
                let enchantment_tick_effects =
                    Arc::clone(&self.datapack_tables.enchantment_tick_effects);
                let weapon_effects = crate::enchantment_weapon_effects::load_weapon_effects(
                    self.cold.config.datapacks.as_ref(),
                );
                let item_publication =
                    crate::entity::ItemPublication::new(Arc::new(self.item_stack_resolver()));
                for level in self.levels.values_mut() {
                    level
                        .entities
                        .set_weapon_effects(Arc::clone(&weapon_effects));
                    level
                        .entities
                        .set_item_publication(item_publication.clone());
                    level
                        .entities
                        .set_enchantment_attribute_effects(Arc::clone(&enchantment_attributes));
                    level.entities.set_enchantment_location_effects(Arc::clone(
                        &enchantment_location_effects,
                    ));
                    level
                        .entities
                        .set_enchantment_tick_effects(Arc::clone(&enchantment_tick_effects));
                    level
                        .entities
                        .set_prevent_equipment_drop_enchantments(Arc::clone(
                            &prevent_equipment_drop_enchantments,
                        ));
                    level
                        .entities
                        .set_prevent_armor_change_enchantments(Arc::clone(
                            &prevent_armor_change_enchantments,
                        ));
                    level
                        .entities
                        .set_enchantment_slots(Arc::clone(&self.datapack_tables.enchantment_slots));
                }
            }
            ReloadedDatapacks::Failed(_) => self.broadcast(
                ClientboundPlayPacket::SystemChat(SystemChat::from_component(
                    commands::reload_failure(),
                    false,
                )),
                events,
            ),
        }
    }
}
