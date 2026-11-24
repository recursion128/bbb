//! The world half of trading with an `AbstractVillager`: `mobInteract`, the
//! merchant menu's view of the trader's offers, and what a trade owes.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::MerchantItemCost;
use crate::inventory::{
    ContainerAnchor, MenuLayout, MerchantMenuEvent, MerchantTrade, MerchantTradeCost,
};
use crate::level_event::VanillaGameEvent;
use crate::world::LevelId;
use bbb_protocol::item_types::{
    vanilla_item_type_for_resource_id, VANILLA_ITEM_VILLAGER_SPAWN_EGG_ID,
};
use bbb_protocol::packets::{
    InteractionHand, ItemCostComponentPredicateSummary, ItemCostSummary, ItemStackSummary,
    OpenScreen,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID;
use bbb_protocol::{NbtSummaryValue, RichTextComponent};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::interaction::is_within_entity_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::{custom_stat, StatKey};
use bbb_server_types::{Result as CoreResult, ServerWorldError};

use super::{send, ServerWorld, WorldEvent};

/// `AbstractVillager.stillValid`'s reach (`AbstractVillager.java:305-307`).
const MERCHANT_INTERACTION_BUFFER: f64 = 4.0;

/// `WanderingTrader.mobInteract` opens at level 1 (`WanderingTrader.java:120`)
/// with `AbstractVillager.getVillagerXp`'s 0 (`AbstractVillager.java:87-89`),
/// `WanderingTrader.showProgressBar`'s false (`WanderingTrader.java:102-104`) and
/// `Merchant.canRestock`'s false (`Merchant.java:33-35`).
const WANDERING_TRADER_STATUS: MerchantStatus = MerchantStatus {
    level: 1,
    xp: 0,
    show_progress: false,
    can_restock: false,
};

/// What `ClientboundMerchantOffersPacket` carries beside the offers.
struct MerchantStatus {
    level: i32,
    xp: i32,
    show_progress: bool,
    can_restock: bool,
}

impl ServerWorld {
    /// `WanderingTrader.mobInteract` (`WanderingTrader.java:107-127`) and
    /// `Villager.mobInteract` (`Villager.java:297-326`), reached once
    /// `Mob.interact`'s earlier branches passed (`Mob.java:1109-1112`).
    /// Answers whether the click was consumed.
    pub(in crate::world) fn merchant_mob_interact(
        &mut self,
        connection_id: ConnectionId,
        key: LevelId,
        trader_id: i32,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = self.levels.player_at(connection_id);
        let held = match hand {
            InteractionHand::MainHand => player.inventory.selected_stack().clone(),
            InteractionHand::OffHand => player.inventory.off_hand_stack().clone(),
        };
        let player_entity_id = player.entity_id();
        let player_uuid = player.profile.uuid;
        let hero_amplifier = player
            .active_effect_id(
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_HERO_OF_THE_VILLAGE_ID,
            )
            .map(|effect| effect.amplifier());
        let Some(trader) = self.level_by_id(&key).entities.get(trader_id) else {
            return false;
        };
        let villager = crate::entity::villager::is_villager(trader);
        if held.item_id() == Some(VANILLA_ITEM_VILLAGER_SPAWN_EGG_ID)
            || !trader.merchant_accepts_trade()
        {
            return false;
        }
        let trader_position = trader.position();
        let baby = villager
            && matches!(trader.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.ageable_retained().is_some_and(|age| age.is_baby()));
        if baby {
            let entities = &mut self.level_mut_by_id(&key).entities;
            if let Some(trader) = entities.get_mut(trader_id) {
                crate::entity::villager_life::set_unhappy(trader);
            }
            entities.play_villager_sound(trader_id, crate::entity::villager::VILLAGER_NO);
        } else {
            self.merchant_start_trading(
                connection_id,
                &key,
                trader_id,
                hand,
                (player_entity_id, player_uuid, hero_amplifier),
                events,
            );
        }
        // `Mob.java:1110-1112`: both SUCCESS and CONSUME consume the action.
        self.player_game_event_at(
            connection_id,
            VanillaGameEvent::ENTITY_INTERACT,
            trader_position,
            None,
        );
        true
    }

    fn merchant_start_trading(
        &mut self,
        connection_id: ConnectionId,
        key: &LevelId,
        trader_id: i32,
        hand: InteractionHand,
        (player_entity_id, player_uuid, hero_amplifier): (i32, uuid::Uuid, Option<i32>),
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(detail) = self.initialize_merchant_offers_id(key, trader_id) {
            tracing::warn!("merchant offers: {detail}");
            return;
        }
        let entities = &mut self.level_mut_by_id(key).entities;
        let Some(trader) = entities.get_mut(trader_id) else {
            return;
        };
        let villager = crate::entity::villager::is_villager(trader);
        let no_offers = trader
            .merchant_offers()
            .is_none_or(crate::entity::MerchantOffers::is_empty);
        if hand == InteractionHand::MainHand {
            if villager && no_offers {
                crate::entity::villager_life::set_unhappy(trader);
                entities.play_villager_sound(trader_id, crate::entity::villager::VILLAGER_NO);
            }
            self.levels[connection_id].award_stat(
                &mut self.scoreboard,
                custom_stat("minecraft:talked_to_villager"),
                1,
            );
        }
        if no_offers {
            return;
        }
        let trader = self
            .level_mut_by_id(key)
            .entities
            .get_mut(trader_id)
            .expect("the trader was just read");
        if villager {
            let reputation = crate::entity::villager::player_reputation(trader, player_uuid);
            crate::entity::villager::update_special_prices(trader, reputation, hero_amplifier);
        }
        trader.set_merchant_trading_player(Some(player_entity_id));
        self.open_trading_screen(connection_id, key, trader_id, events);
    }

    /// `Merchant.openTradingScreen` (`Merchant.java:37-45`) over
    /// `ServerPlayer.openMenu` (`ServerPlayer.java:1319-1342`).
    fn open_trading_screen(
        &mut self,
        connection_id: ConnectionId,
        key: &LevelId,
        trader_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let opened = (|| {
            let container_id = self
                .begin_container_open(connection_id, events)
                .ok_or_else(|| invalid("the previous menu did not close"))?;
            let title = self.merchant_title(key, trader_id)?;
            let (trades, wire) = self.merchant_offer_views(key, trader_id)?;
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            let prepared = player
                .inventory
                .prepare_reserved_menu(
                    &resolver,
                    None,
                    container_id,
                    MenuLayout::Merchant,
                    ContainerAnchor {
                        positions: Vec::new(),
                    },
                    vec![RuntimeItemStack::empty(); 3],
                )?
                .ok_or_else(|| invalid("the merchant menu was refused"))?;
            player.inventory.bind_merchant_trader(trader_id);
            player.inventory.refresh_merchant_trades(trades);
            Ok::<_, ServerWorldError>((prepared, title, wire))
        })();
        let (prepared, title, wire) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                self.release_merchant_trader(key, trader_id);
                tracing::warn!("merchant menu: {error}");
                return;
            }
        };
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenScreen(OpenScreen {
                container_id: prepared.container_id,
                menu_type_id: prepared.menu_type_id,
                title,
            }),
        ));
        self.initialize_player_menu(connection_id, events);
        self.send_merchant_offer_list(
            connection_id,
            key,
            trader_id,
            prepared.container_id,
            wire,
            events,
        );
    }

    /// `ServerPlayer.sendMerchantOffers` with the merchant's level, xp,
    /// progress bar and restock flag; an empty list sends nothing.
    fn send_merchant_offer_list(
        &self,
        connection_id: ConnectionId,
        key: &LevelId,
        trader_id: i32,
        container_id: i32,
        offers: Vec<bbb_protocol::packets::MerchantOffer>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(trader) = self.level_by_id(key).entities.get(trader_id) else {
            return;
        };
        let status = match trader.data() {
            crate::entity::EntityData::GeneralLiving(data)
                if crate::entity::villager::is_villager(trader) =>
            {
                MerchantStatus {
                    level: data
                        .villager_profession_and_level()
                        .map_or(1, |(_, level)| level),
                    xp: crate::entity::villager::villager_xp(trader).unwrap_or(0),
                    show_progress: true,
                    can_restock: true,
                }
            }
            _ => WANDERING_TRADER_STATUS,
        };
        if !offers.is_empty() {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::MerchantOffers(bbb_protocol::packets::MerchantOffers {
                    container_id,
                    offers,
                    villager_level: status.level,
                    villager_xp: status.xp,
                    show_progress: status.show_progress,
                    can_restock: status.can_restock,
                }),
            ));
        }
    }

    /// The world half of [`crate::entity::villager::PendingVillagerAction`].
    pub(in crate::world) fn apply_villager_action(
        &mut self,
        dimension: &LevelId,
        action: crate::entity::villager::PendingVillagerAction,
        events: &mut Vec<WorldEvent>,
    ) {
        use crate::entity::villager::PendingVillagerAction;
        let entity_id = match action {
            PendingVillagerAction::ReleasePoi {
                dimension,
                pos,
                memory,
                profession,
            } => {
                let Some(dimension) = bbb_server_types::save::Java26LevelKey::parse(&dimension)
                    .ok()
                    .and_then(|key| self.level_id(&key))
                    .filter(|id| self.has_level_id(id))
                else {
                    return;
                };
                if let Err(error) = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .restore_saved_column_input((pos[0] >> 4, pos[2] >> 4))
                {
                    tracing::warn!("villager POI restore: {error}");
                    return;
                }
                self.reconcile_saved_column_accesses(&dimension);
                let mut leftover =
                    super::simulation_level::pending_spawns::PendingEntitySpawns::default();
                self.with_level_ctx(&dimension, &[], &mut leftover, |level, _| {
                    crate::entity::villager::release_poi(memory, &profession, pos, level);
                });
                return;
            }
            PendingVillagerAction::GenerateOffers { entity_id } => {
                if let Err(detail) = self.initialize_merchant_offers_id(dimension, entity_id) {
                    tracing::warn!("villager offers: {detail}");
                }
                return;
            }
            PendingVillagerAction::IncreaseCareer { entity_id } => {
                if let Err(detail) = self.increase_villager_career(dimension, entity_id) {
                    tracing::warn!("villager career: {detail}");
                }
                entity_id
            }
            PendingVillagerAction::ResendOffers { entity_id } => entity_id,
            PendingVillagerAction::SpawnGolemIfNeeded { entity_id, needed } => {
                self.villager_spawn_golem_if_needed(dimension, entity_id, needed);
                return;
            }
        };
        let Some(player_entity) = self
            .level_by_id(dimension)
            .entities
            .get(entity_id)
            .and_then(crate::entity::Entity::merchant_trading_player)
        else {
            return;
        };
        let Some(connection_id) = self.levels.players().find_map(|player| {
            (player.entity_id() == player_entity
                && player.inventory.merchant_trader() == Some(entity_id))
            .then_some(player.connection_id)
        }) else {
            return;
        };
        let container_id = self.levels[connection_id].inventory.container_id();
        match self.merchant_offer_views(dimension, entity_id) {
            Ok((trades, wire)) => {
                let inventory = &mut self.levels[connection_id].inventory;
                inventory.refresh_merchant_trades(trades);
                inventory.take_merchant_events();
                self.send_merchant_offer_list(
                    connection_id,
                    dimension,
                    entity_id,
                    container_id,
                    wire,
                    events,
                );
            }
            Err(error) => tracing::warn!("merchant offers: {error}"),
        }
    }

    /// `Entity.getDisplayName` of the trader, the `SimpleMenuProvider` title.
    fn merchant_title(&self, key: &LevelId, trader_id: i32) -> CoreResult<RichTextComponent> {
        let trader = self
            .level_by_id(key)
            .entities
            .get(trader_id)
            .ok_or_else(|| invalid("the trader left its level"))?;
        let team = self
            .scoreboard
            .board()
            .players_team(&trader.uuid().to_string());
        RichTextComponent::try_from_component_nbt_with_registry(
            super::waypoints::command_stored_display_name(trader, team),
            self.plugins().content_registry(),
        )
        .map_err(|error| invalid(format!("trader name: {error}")))
    }

    /// The menu's working copy of `Merchant.getOffers()` and the
    /// `ClientboundMerchantOffersPacket` view of the same list.
    fn merchant_offer_views(
        &self,
        key: &LevelId,
        trader_id: i32,
    ) -> CoreResult<(
        Vec<MerchantTrade>,
        Vec<bbb_protocol::packets::MerchantOffer>,
    )> {
        let resolver = self.item_stack_resolver();
        let Some(offers) = self
            .level_by_id(key)
            .entities
            .get(trader_id)
            .and_then(crate::entity::Entity::merchant_offers)
        else {
            return Ok((Vec::new(), Vec::new()));
        };
        let mut trades = Vec::with_capacity(offers.offers().len());
        let mut wire = Vec::with_capacity(offers.offers().len());
        for offer in offers.offers() {
            let (cost_a, buy_a) = merchant_cost(&resolver, offer.base_cost_a(), |max| {
                offer.modified_cost_a_count(max)
            })?;
            let cost_b = offer
                .cost_b()
                .map(|cost| merchant_cost(&resolver, cost, |_| cost.count()))
                .transpose()?;
            let result = offer.result().clone();
            wire.push(bbb_protocol::packets::MerchantOffer {
                buy_a,
                sell: result.summary().clone(),
                buy_b: cost_b.as_ref().map(|(_, summary)| summary.clone()),
                is_out_of_stock: offer.is_out_of_stock(),
                uses: offer.uses(),
                max_uses: offer.max_uses(),
                xp: offer.xp(),
                special_price_diff: offer.special_price_diff(),
                price_multiplier: offer.price_multiplier(),
                demand: offer.demand(),
            });
            trades.push(MerchantTrade {
                cost_a,
                cost_b: cost_b.map(|(cost, _)| cost),
                result,
                uses: offer.uses(),
                max_uses: offer.max_uses(),
                xp: offer.xp(),
            });
        }
        Ok((trades, wire))
    }

    /// Menu-only view of `Merchant.getOffers()` without building wire packet copies.
    fn merchant_menu_trades(
        &self,
        key: &LevelId,
        trader_id: i32,
    ) -> CoreResult<Vec<MerchantTrade>> {
        let resolver = self.item_stack_resolver();
        let Some(offers) = self
            .level_by_id(key)
            .entities
            .get(trader_id)
            .and_then(crate::entity::Entity::merchant_offers)
        else {
            return Ok(Vec::new());
        };
        let mut trades = Vec::with_capacity(offers.offers().len());
        for offer in offers.offers() {
            let cost_a = merchant_trade_cost(&resolver, offer.base_cost_a(), |max| {
                offer.modified_cost_a_count(max)
            })?;
            let cost_b = offer
                .cost_b()
                .map(|cost| merchant_trade_cost(&resolver, cost, |_| cost.count()))
                .transpose()?;
            trades.push(MerchantTrade {
                cost_a,
                cost_b,
                result: offer.result().clone(),
                uses: offer.uses(),
                max_uses: offer.max_uses(),
                xp: offer.xp(),
            });
        }
        Ok(trades)
    }

    /// Re-reads the trader's offers into the open merchant menu, which is
    /// what `MerchantContainer` reads through `merchant.getOffers()`.
    pub(in crate::world) fn refresh_merchant_menu(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let Some(trader_id) = player.inventory.merchant_trader() else {
            return;
        };
        let key = player.dimension;
        match self.merchant_menu_trades(&key, trader_id) {
            Ok(trades) => {
                let inventory = &mut self.levels[connection_id].inventory;
                inventory.refresh_merchant_trades(trades);
                inventory.take_merchant_events();
            }
            Err(error) => tracing::warn!("merchant offers: {error}"),
        }
    }

    /// `MerchantMenu.stillValid` (`MerchantMenu.java:63-65`) over
    /// `AbstractVillager.stillValid` (`AbstractVillager.java:305-307`).
    pub(in crate::world) fn merchant_menu_still_valid(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        let Some(trader_id) = player.inventory.merchant_trader() else {
            return false;
        };
        let Some(trader) = self.level_by_id(&player.dimension).entities.get(trader_id) else {
            return false;
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        trader.merchant_trading_player() == Some(player.entity_id())
            && trader.is_alive()
            && trader.living().is_some_and(|living| living.health() > 0.0)
            && is_within_entity_interaction_range(
                eye,
                &trader.bounding_box(),
                player.entity_interaction_range(),
                MERCHANT_INTERACTION_BUFFER,
            )
    }

    /// `ServerGamePacketListenerImpl.handleSelectTrade` (`:962-973`).
    pub(super) fn apply_select_trade(
        &mut self,
        connection_id: ConnectionId,
        selection: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.levels[connection_id].inventory.is_merchant_menu()
            || !self.merchant_menu_still_valid(connection_id)
        {
            return;
        }
        self.refresh_merchant_menu(connection_id);
        let resolver = self.item_stack_resolver();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if let Err(error) = player
            .inventory
            .select_merchant_trade(&resolver, None, selection)
        {
            tracing::warn!("merchant selection: {error}");
        }
        self.settle_merchant_menu(connection_id, events);
    }

    /// The trader and player halves of what the last merchant menu operation
    /// reached, in its order: `onCraftedBy`'s statistic, `notifyTrade` with
    /// its `TRADE` trigger and `TRADED_WITH_VILLAGER`
    /// (`MerchantResultSlot.java:43-60`), and `notifyTradeUpdated`.
    pub(in crate::world) fn settle_merchant_menu(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let Some(trader_id) = player.inventory.merchant_trader() else {
            return;
        };
        let menu_events = player.inventory.take_merchant_events();
        if menu_events.is_empty() {
            return;
        }
        let results: Vec<_> = player
            .inventory
            .merchant_trades()
            .iter()
            .map(|trade| trade.result.summary().clone())
            .collect();
        let key = player.dimension;
        let player_uuid = player.profile.uuid;
        for event in menu_events {
            match event {
                MerchantMenuEvent::Crafted { item_id, count } => {
                    self.levels[connection_id].award_stat(
                        &mut self.scoreboard,
                        StatKey::new(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, item_id),
                        count,
                    );
                }
                MerchantMenuEvent::Traded { offer } => {
                    let entities = &mut self.level_mut_by_id(&key).entities;
                    if !entities.wandering_trader_notify_trade(trader_id, offer) {
                        entities.villager_notify_trade(trader_id, offer, Some(player_uuid));
                    }
                    self.trigger_villager_trade(
                        connection_id,
                        key,
                        trader_id,
                        &results[offer],
                        events,
                    );
                    self.levels[connection_id].award_stat(
                        &mut self.scoreboard,
                        custom_stat("minecraft:traded_with_villager"),
                        1,
                    );
                }
                MerchantMenuEvent::TradeUpdated { valid } => {
                    let entities = &mut self.level_mut_by_id(&key).entities;
                    if entities
                        .get(trader_id)
                        .is_some_and(crate::entity::villager::is_villager)
                    {
                        entities.villager_notify_trade_updated(trader_id, valid);
                    } else {
                        entities.wandering_trader_notify_trade_updated(trader_id, valid);
                    }
                }
            }
        }
        self.apply_level_entity_lifecycle_by_key(&key, events);
    }

    /// `MerchantMenu.removed`'s `setTradingPlayer(null)` (`MerchantMenu.java:152`).
    pub(in crate::world) fn release_merchant_trader(&mut self, key: &LevelId, trader_id: i32) {
        if let Some(trader) = self.level_mut_by_id(key).entities.get_mut(trader_id) {
            stop_trading(trader);
        }
    }

    /// The stored kinds whose `mobInteract` this module owns.
    pub(in crate::world) fn is_merchant_target(&self, key: &LevelId, entity_id: i32) -> bool {
        self.level_by_id(key)
            .entities
            .get(entity_id)
            .is_some_and(|entity| {
                crate::entity::merchant::is_abstract_villager(entity.entity_type_id())
            })
    }
}

/// `Villager.stopTrading` (`Villager.java:351-354`, `AbstractVillager.java:197-199`),
/// also run by `AbstractVillager.teleport` (`:192-195`); the open menu then
/// fails `stillValid` and closes.
pub(in crate::world) fn stop_trading(trader: &mut crate::entity::Entity) {
    trader.set_merchant_trading_player(None);
    crate::entity::villager::reset_special_prices(trader);
}

/// Menu-only half of [`merchant_cost`]: skips cloning components into a wire
/// [`ItemCostSummary`].
fn merchant_trade_cost(
    resolver: &impl PersistentItemStackHolderResolver,
    cost: &MerchantItemCost,
    count: impl FnOnce(i32) -> i32,
) -> CoreResult<MerchantTradeCost> {
    let content_registry = resolver.content_registry();
    let item_id = vanilla_item_type_for_resource_id(cost.item_resource_id())
        .map(|item| item.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(cost.item_resource_id())
                .map(|item| item.id)
        })
        .ok_or_else(|| invalid(format!("unknown cost item {}", cost.item_resource_id())))?;
    let exact_components = (!cost.components().is_empty())
        .then(|| {
            bbb_registry_codec::read_component_patch(
                &NbtSummaryValue::Compound(cost.components().to_vec()),
                resolver,
            )
        })
        .transpose()
        .map_err(|error| invalid(format!("merchant cost: {error}")))?;
    let template = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: exact_components.clone().unwrap_or_default().into(),
    };
    let max_stack_size = bbb_server_types::item_stack::rules::max_stack_size_with_registry(
        &template,
        content_registry,
    )?;
    Ok(MerchantTradeCost {
        item_id,
        count: count(max_stack_size),
        exact_components,
    })
}

/// One `ItemCost`: its template stack (`ItemCost.java:47-49`),
/// the menu's test, and the `ItemCost.STREAM_CODEC` view (`:26-28`).
fn merchant_cost(
    resolver: &impl PersistentItemStackHolderResolver,
    cost: &MerchantItemCost,
    count: impl FnOnce(i32) -> i32,
) -> CoreResult<(MerchantTradeCost, ItemCostSummary)> {
    let content_registry = resolver.content_registry();
    let item_id = vanilla_item_type_for_resource_id(cost.item_resource_id())
        .map(|item| item.id)
        .or_else(|| {
            content_registry
                .item_by_resource_id(cost.item_resource_id())
                .map(|item| item.id)
        })
        .ok_or_else(|| invalid(format!("unknown cost item {}", cost.item_resource_id())))?;
    let exact_components = (!cost.components().is_empty())
        .then(|| {
            let patch = bbb_registry_codec::read_component_patch(
                &NbtSummaryValue::Compound(cost.components().to_vec()),
                resolver,
            )
            .map_err(|error| invalid(format!("merchant cost: {error}")))?;
            if !patch.removed_type_ids.is_empty() {
                return Err(invalid(
                    "merchant cost: exact component predicate cannot remove components",
                ));
            }
            Ok(patch)
        })
        .transpose()?;
    let template = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: exact_components.clone().unwrap_or_default().into(),
    });
    let max_stack_size = bbb_server_types::item_stack::rules::max_stack_size_with_registry(
        template.summary(),
        content_registry,
    )?;
    let summary = ItemCostSummary {
        item_id,
        count: cost.count(),
        component_predicate: ItemCostComponentPredicateSummary {
            components: template.into_summary().component_patch,
        },
    };
    Ok((
        MerchantTradeCost {
            item_id,
            count: count(max_stack_size),
            exact_components,
        },
        summary,
    ))
}

fn invalid(detail: impl Into<String>) -> ServerWorldError {
    ServerWorldError::InvalidConfiguration(detail.into())
}

#[cfg(test)]
mod tests;
