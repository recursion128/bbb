//! The world's half of the utility menus: the player facts they read, the
//! enchanting table's offers and button, the anvil's damage, experience,
//! sounds and level events of a take, renaming and the beacon's powers.

use crate::world::LevelId;
use std::sync::{Arc, OnceLock};

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_ANVIL_ID, VANILLA_BLOCK_CHIPPED_ANVIL_ID, VANILLA_BLOCK_DAMAGED_ANVIL_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID;
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type;
use bbb_protocol::packets::{BlockPos, ContainerButtonClick, SetBeacon, Vec3d};
use bbb_server_datapack::loot::HolderSet;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::interaction::is_within_block_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::custom_stat;

use crate::block_behavior::weathering::with_properties_of;
use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use crate::entity::award_experience;
use crate::inventory::{
    EnchantmentOffers, MenuBroadcastMode, UtilityMenuEvent, UtilityMenuFacts, UtilityMenuKind,
    UtilityPlayerFacts, ENCHANTMENT_OFFER_COUNT,
};
use crate::random::GameRandom;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::registry_runtime::PlayerItemStackResolver;

use super::block_use::utility_menu::opens_utility_menu;
use super::spawner_minecarts::{select_spawner_enchantments, SpawnEnchantmentProviderEnvironment};
use super::{ServerWorld, WorldEvent, CONTAINER_DISTANCE_BUFFER};

/// `LevelEvent.SOUND_ANVIL_BROKEN`, `SOUND_ANVIL_USED`,
/// `SOUND_GRINDSTONE_USED` and `SOUND_SMITHING_TABLE_USED`.
const ANVIL_BROKEN_EVENT: i32 = 1029;
const ANVIL_USED_EVENT: i32 = 1030;
const GRINDSTONE_USED_EVENT: i32 = 1042;
const SMITHING_TABLE_USED_EVENT: i32 = 1044;
/// `AnvilMenu.onTake`'s damage chance (`AnvilMenu.java:103`).
const ANVIL_DAMAGE_CHANCE: f32 = 0.12;

struct EnchantingEnvironment<'a> {
    tags: &'a RegistryTagSnapshot,
    datapacks: Option<&'a DatapackStartup>,
    resolver: &'a PlayerItemStackResolver,
}

impl SpawnEnchantmentProviderEnvironment for EnchantingEnvironment<'_> {
    fn registry_tags(&self) -> Option<&RegistryTagSnapshot> {
        Some(self.tags)
    }

    fn datapacks(&self) -> Option<&DatapackStartup> {
        self.datapacks
    }

    fn item_holders(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.resolver
    }
}

impl ServerWorld {
    /// The datapack generation a utility menu opened now reads.
    pub(in crate::world) fn utility_menu_facts(&self) -> Arc<UtilityMenuFacts> {
        let registry_tags = self.registries.registry_tags_shared();
        Arc::new(UtilityMenuFacts {
            resolver: self.item_stack_resolver(),
            enchantments: self
                .datapack_tables
                .enchantment_definitions(self.cold.config.datapacks.as_deref(), &registry_tags),
            registry_tags,
        })
    }

    fn utility_menu_at(&self, connection_id: ConnectionId) -> Option<(UtilityMenuKind, BlockPos)> {
        let inventory = &self.levels.player(connection_id)?.inventory;
        let kind = inventory.utility_menu_kind()?;
        let pos = *inventory.open_container_anchor()?.positions.first()?;
        Some((kind, pos))
    }

    /// The player facts the open utility menu reads, and the enchanting
    /// table's offers when its inputs changed.
    pub(in crate::world) fn refresh_utility_menu(&mut self, connection_id: ConnectionId) {
        let Some((kind, pos)) = self.utility_menu_at(connection_id) else {
            return;
        };
        let player = self.levels.player_at(connection_id);
        let experience_level = player.experience.level();
        let infinite_materials = player.abilities.instabuild;
        let referenced_maps = if kind == UtilityMenuKind::CartographyTable {
            player
                .inventory
                .inventory_items()
                .iter()
                .chain(std::iter::once(player.inventory.carried()))
                .chain(player.inventory.container_item_slice().iter())
                .filter_map(bbb_server_types::item_stack::RuntimeItemStack::map_id)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let facts = UtilityPlayerFacts {
            experience_level,
            infinite_materials,
            maps: if kind == UtilityMenuKind::CartographyTable {
                self.maps.cartography_facts_for(referenced_maps)
            } else {
                Default::default()
            },
        };
        let inventory = &mut self.levels[connection_id].inventory;
        inventory.refresh_utility_player(facts);
        if inventory.take_enchantment_inputs_changed() {
            let offers = self.enchantment_offers(connection_id, pos);
            self.levels[connection_id]
                .inventory
                .set_enchantment_offers(offers);
        }
    }

    /// `AbstractContainerMenu.stillValid(access, player, block)`
    /// (`AbstractContainerMenu.java:93-95`); the anvils share `BlockTags.ANVIL`.
    pub(in crate::world) fn utility_menu_still_valid(&self, connection_id: ConnectionId) -> bool {
        let Some((kind, pos)) = self.utility_menu_at(connection_id) else {
            return true;
        };
        let player = self.levels.player_at(connection_id);
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        opens_utility_menu(self.level_by_id(&player.dimension).block_state(pos), kind)
            && is_within_block_interaction_range(
                eye,
                pos,
                player.block_interaction_range(),
                CONTAINER_DISTANCE_BUFFER,
            )
    }

    /// The beacon's three data slots (`BeaconBlockEntity.java:76-109`):
    /// levels and the encoded powers.
    pub(in crate::world) fn utility_menu_data(
        &self,
        connection_id: ConnectionId,
    ) -> Option<Vec<i32>> {
        let (kind, pos) = self.utility_menu_at(connection_id)?;
        if kind != UtilityMenuKind::Beacon {
            let inventory = &self.levels.player(connection_id)?.inventory;
            return inventory.utility_menu_data().map(<[i32]>::to_vec);
        }
        let dimension = self.player_level_id(connection_id);
        let beacon = self.get_block_entity(dimension, pos)?.beacon()?;
        let encode = |effect: Option<&str>| {
            effect
                .and_then(bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id)
                .map_or(0, |effect| effect.id + 1)
        };
        Some(vec![
            beacon.levels(),
            encode(beacon.primary_power()),
            encode(beacon.secondary_power()),
        ])
    }

    /// The world half of every result take the last click reached.
    pub(in crate::world) fn settle_utility_menu(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((_, pos)) = self.utility_menu_at(connection_id) else {
            return;
        };
        self.refresh_utility_menu(connection_id);
        let menu_events = self.levels[connection_id].inventory.take_utility_events();
        if menu_events.is_empty() {
            return;
        }
        let dimension = self.player_level_id(connection_id);
        let infinite = self.levels[connection_id].abilities.instabuild;
        // `lastSoundTime`: one take sound per tick however many takes a
        // quick-move chained.
        let mut take_sound = None;
        for event in menu_events {
            match event {
                UtilityMenuEvent::AnvilUsed { cost } => {
                    if !infinite {
                        let player = &mut self.levels[connection_id];
                        player.experience.give_experience_levels(-cost);
                        player.last_sent_exp = -1;
                    }
                    self.damage_anvil(dimension, pos, infinite);
                }
                UtilityMenuEvent::GrindstoneUsed { experience } => {
                    // `GrindstoneMenu.getExperienceAmount` (`GrindstoneMenu.java:79-89`).
                    let amount = if experience > 0 {
                        let half = (experience + 1) / 2;
                        half + self.random.next_int(half)
                    } else {
                        0
                    };
                    let center = Vec3d {
                        x: f64::from(pos.x) + 0.5,
                        y: f64::from(pos.y) + 0.5,
                        z: f64::from(pos.z) + 0.5,
                    };
                    let level = self.level_mut_by_id(&dimension);
                    let super::ServerLevel {
                        entities, random, ..
                    } = level;
                    let _ = award_experience(entities, random, center, amount);
                    level.event_sink.level_event(GRINDSTONE_USED_EVENT, pos, 0);
                }
                UtilityMenuEvent::Smithed { recipe } => {
                    self.award_utility_recipe(connection_id, recipe, events);
                    self.level_mut_by_id(&dimension).event_sink.level_event(
                        SMITHING_TABLE_USED_EVENT,
                        pos,
                        0,
                    );
                }
                UtilityMenuEvent::Stonecut { recipe } => {
                    self.award_utility_recipe(connection_id, recipe, events);
                    take_sound = Some("minecraft:ui.stonecutter.take_result");
                }
                UtilityMenuEvent::LoomUsed => {
                    take_sound = Some("minecraft:ui.loom.take_result");
                }
                UtilityMenuEvent::CartographyUsed => {
                    take_sound = Some("minecraft:ui.cartography_table.take_result");
                }
            }
        }
        if let Some(sound) = take_sound {
            self.play_block_sound(dimension, pos, sound, events);
        }
    }

    /// `AnvilMenu.onTake`'s block half (`AnvilMenu.java:100-114`).
    fn damage_anvil(&mut self, dimension: impl Into<LevelId>, pos: BlockPos, infinite: bool) {
        let dimension = dimension.into();
        let Some(state) = self.block_state(dimension, pos) else {
            return;
        };
        let damaged = !infinite
            && opens_utility_menu(state, UtilityMenuKind::Anvil)
            && self.random.next_float() < ANVIL_DAMAGE_CHANCE;
        if !damaged {
            self.level_mut_by_id(&dimension)
                .event_sink
                .level_event(ANVIL_USED_EVENT, pos, 0);
            return;
        }
        // `AnvilBlock.damage` (`AnvilBlock.java:105-111`).
        let next = match vanilla_block_state_block_type_id(state) {
            Some(VANILLA_BLOCK_ANVIL_ID) => {
                with_properties_of(VANILLA_BLOCK_CHIPPED_ANVIL_ID, state)
            }
            Some(VANILLA_BLOCK_CHIPPED_ANVIL_ID) => {
                with_properties_of(VANILLA_BLOCK_DAMAGED_ANVIL_ID, state)
            }
            _ => None,
        };
        self.with_level(dimension, |level| match next {
            Some(next) => {
                level.set_block(
                    pos,
                    next,
                    crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
                level.level_event(ANVIL_USED_EVENT, pos, 0);
            }
            None => {
                level.remove_block(pos);
                level.level_event(ANVIL_BROKEN_EVENT, pos, 0);
            }
        });
    }

    /// `RecipeCraftingHolder.awardUsedRecipes` for the smithing table and
    /// stonecutter.
    fn award_utility_recipe(
        &mut self,
        connection_id: ConnectionId,
        recipe: Option<String>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(recipe) = recipe else {
            return;
        };
        self.award_recipes(connection_id, &[recipe], events);
    }

    /// `EnchantmentMenu.slotsChanged` (`EnchantmentMenu.java:86-132`).
    fn enchantment_offers(&self, connection_id: ConnectionId, pos: BlockPos) -> EnchantmentOffers {
        let mut offers = EnchantmentOffers {
            costs: [0; ENCHANTMENT_OFFER_COUNT],
            clues: [-1; ENCHANTMENT_OFFER_COUNT],
            level_clues: [-1; ENCHANTMENT_OFFER_COUNT],
        };
        let player = self.levels.player_at(connection_id);
        let Some(item) = player.inventory.enchantable_input() else {
            return offers;
        };
        let seed = player.experience.enchantment_seed();
        let bookcases = self.enchanting_bookcases(connection_id, pos);
        let random = GameRandom::with_seed(i64::from(seed));
        for (slot, cost) in offers.costs.iter_mut().enumerate() {
            // `EnchantmentHelper.getEnchantmentCost` (`EnchantmentHelper.java:504-520`).
            let selected =
                random.next_int(8) + 1 + (bookcases >> 1) + random.next_int(bookcases + 1);
            *cost = match slot {
                0 => (selected / 3).max(1),
                1 => selected * 2 / 3 + 1,
                _ => selected.max(bookcases * 2),
            };
            if *cost < slot as i32 + 1 {
                *cost = 0;
            }
        }
        for slot in 0..ENCHANTMENT_OFFER_COUNT {
            if offers.costs[slot] <= 0 {
                continue;
            }
            let list = self.enchantment_list(&item, seed, slot, offers.costs[slot]);
            if list.is_empty() {
                continue;
            }
            let (holder, level) = list[random.next_int(list.len() as i32) as usize];
            offers.clues[slot] = holder as i32;
            offers.level_clues[slot] = level;
        }
        offers
    }

    /// `EnchantingTableBlock.isValidBookShelf` over `BOOKSHELF_OFFSETS`
    /// (`EnchantingTableBlock.java:32-50`), capped at fifteen.
    fn enchanting_bookcases(&self, connection_id: ConnectionId, pos: BlockPos) -> i32 {
        let level = self.level_by_id(&self.levels.player_at(connection_id).dimension);
        let tags = self.registries.registry_tags_shared();
        let in_tag = |pos: BlockPos, tag: &str| tags.block_state_in(tag, level.block_state(pos));
        let mut count = 0;
        for dx in -2..=2_i32 {
            for dy in 0..=1 {
                for dz in -2..=2_i32 {
                    if dx.abs() != 2 && dz.abs() != 2 {
                        continue;
                    }
                    let at = |x, y, z| BlockPos {
                        x: pos.x + x,
                        y: pos.y + y,
                        z: pos.z + z,
                    };
                    if in_tag(at(dx, dy, dz), "minecraft:enchantment_power_provider")
                        && in_tag(
                            at(dx / 2, dy, dz / 2),
                            "minecraft:enchantment_power_transmitter",
                        )
                    {
                        count += 1;
                    }
                }
            }
        }
        count.min(15)
    }

    /// `EnchantmentMenu.getEnchantmentList` (`EnchantmentMenu.java:187-200`)
    /// as enchantment holder ids and levels.
    fn enchantment_list(
        &self,
        item: &RuntimeItemStack,
        seed: i32,
        slot: usize,
        cost: i32,
    ) -> Vec<(usize, i32)> {
        let random = GameRandom::with_seed(i64::from(seed) + slot as i64);
        let tags = self.registries.registry_tags_shared();
        let resolver = self.item_stack_resolver();
        let environment = EnchantingEnvironment {
            tags: &tags,
            datapacks: self.cold.config.datapacks.as_deref(),
            resolver: &resolver,
        };
        static IN_ENCHANTING_TABLE: OnceLock<HolderSet> = OnceLock::new();
        let set = IN_ENCHANTING_TABLE
            .get_or_init(|| HolderSet::Tag("minecraft:in_enchanting_table".to_owned()));
        let mut list: Vec<(usize, i32)> =
            select_spawner_enchantments(&environment, item, set, cost, &random)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|selected| {
                    let holder = self
                        .datapack_tables
                        .enchantment_holder_id(&selected.resource_id)?;
                    Some((holder, selected.level))
                })
                .collect();
        if item.item_id() == Some(VANILLA_ITEM_BOOK_ID) && list.len() > 1 {
            list.remove(random.next_int(list.len() as i32) as usize);
        }
        list
    }

    /// `ServerGamePacketListenerImpl.handleContainerButtonClick` for a
    /// utility menu; `false` when another menu is open.
    pub(in crate::world) fn apply_utility_button_click(
        &mut self,
        connection_id: ConnectionId,
        click: &ContainerButtonClick,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some((kind, pos)) = self.utility_menu_at(connection_id) else {
            return false;
        };
        self.refresh_utility_menu(connection_id);
        let accepted = if kind == UtilityMenuKind::Enchantment {
            self.click_enchantment_button(connection_id, pos, click.button_id, events)
        } else {
            match self.levels[connection_id]
                .inventory
                .click_utility_button(click.button_id)
            {
                Ok(accepted) => accepted.unwrap_or(false),
                Err(error) => {
                    tracing::warn!("utility menu button: {error}");
                    false
                }
            }
        };
        if accepted {
            self.broadcast_player_menu(connection_id, MenuBroadcastMode::Changes, events);
            self.sync_menu_data(connection_id, false, events);
        }
        true
    }

    /// `EnchantmentMenu.clickMenuButton` (`EnchantmentMenu.java:135-185`).
    fn click_enchantment_button(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        button_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Ok(slot) = usize::try_from(button_id) else {
            return false;
        };
        if slot >= ENCHANTMENT_OFFER_COUNT {
            return false;
        }
        let player = self.levels.player_at(connection_id);
        let infinite = player.abilities.instabuild;
        let lapis_cost = button_id + 1;
        let item = player.inventory.utility_slot(0);
        let currency = player.inventory.utility_slot(1);
        let cost = player.inventory.enchantment_offers().costs[slot];
        let experience_level = player.experience.level();
        if (currency.is_empty() || currency.count() < lapis_cost) && !infinite {
            return false;
        }
        if cost <= 0
            || item.is_empty()
            || (experience_level < lapis_cost || experience_level < cost) && !infinite
        {
            return false;
        }
        let seed = player.experience.enchantment_seed();
        let list = self.enchantment_list(&item, seed, slot, cost);
        if list.is_empty() {
            return true;
        }
        let next_seed = self.random.next_u64() as i32;
        let player = &mut self.levels[connection_id];
        player
            .experience
            .on_enchantment_performed(lapis_cost, next_seed);
        player.last_sent_exp = -1;
        let enchanted = match player
            .inventory
            .apply_enchantments(&list, lapis_cost, infinite)
        {
            Ok(enchanted) => enchanted,
            Err(error) => {
                tracing::warn!("enchanting table: {error}");
                return true;
            }
        };
        player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:enchant_item"));
        player.inventory.set_enchantment_seed(next_seed);
        self.trigger_enchanted_item(connection_id, enchanted.summary(), lapis_cost, events);
        self.refresh_utility_menu(connection_id);
        let dimension = self.player_level_id(connection_id);
        let pitch = self.random.next_float() * 0.1 + 0.9;
        self.play_block_sound_as(
            dimension,
            pos,
            "minecraft:block.enchantment_table.use",
            (1.0, pitch),
            None,
            events,
        );
        true
    }

    /// `ServerGamePacketListenerImpl.handleRenameItem` (`:761-771`).
    pub(in crate::world) fn apply_rename_item(
        &mut self,
        connection_id: ConnectionId,
        name: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.utility_menu_at(connection_id).map(|(kind, _)| kind) != Some(UtilityMenuKind::Anvil)
            || !self.utility_menu_still_valid(connection_id)
        {
            return;
        }
        self.refresh_utility_menu(connection_id);
        match self.levels[connection_id].inventory.rename_anvil_item(name) {
            Ok(true) => {
                self.broadcast_player_menu(connection_id, MenuBroadcastMode::Changes, events);
                self.sync_menu_data(connection_id, false, events);
            }
            Ok(false) => {}
            Err(error) => tracing::warn!("anvil rename: {error}"),
        }
    }

    /// `ServerGamePacketListenerImpl.handleSetBeaconPacket` (`:774-784`) and
    /// `BeaconMenu.updateEffects` (`BeaconMenu.java:142-149`).
    pub(in crate::world) fn apply_set_beacon(
        &mut self,
        connection_id: ConnectionId,
        update: &SetBeacon,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((UtilityMenuKind::Beacon, pos)) = self.utility_menu_at(connection_id) else {
            return;
        };
        if !self.utility_menu_still_valid(connection_id)
            || !self.levels[connection_id].inventory.spend_beacon_payment()
        {
            return;
        }
        let dimension = self.player_level_id(connection_id);
        let resource = |effect: Option<i32>| {
            effect
                .and_then(vanilla_mob_effect_type)
                .map(|effect| effect.resource_id)
        };
        let Some(beacon) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::beacon_mut)
        else {
            return;
        };
        let has_beam = beacon.has_beam();
        beacon.set_powers(
            resource(update.primary_effect),
            resource(update.secondary_effect),
        );
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        if has_beam {
            self.play_block_sound(
                dimension,
                pos,
                "minecraft:block.beacon.power_select",
                events,
            );
        }
        self.broadcast_player_menu(connection_id, MenuBroadcastMode::Changes, events);
        self.sync_menu_data(connection_id, false, events);
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn set_experience_levels_for_test(
        &mut self,
        connection_id: ConnectionId,
        levels: i32,
    ) {
        self.levels[connection_id]
            .experience
            .set_experience_levels(levels);
    }

    pub(crate) fn experience_level_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].experience.level()
    }

    pub(crate) fn experience_total_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].experience.total()
    }
}
