use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::merchant_trades::tests::{config_with_sets, empty_set};
use crate::world::{TickResult, WorldCommand};
use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_EMERALD_ID};
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_MERCHANT_ID;
use bbb_protocol::packets::{
    BlockPos, ContainerClick, ContainerCloseRequest, HashedStack, InteractEntity, ItemStackSummary,
    PlayServerbound, SelectTradeCommand, Vec3d,
};
use bbb_server_persistence::RegionCompression;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

const PLAYER: ConnectionId = ConnectionId(1);

fn count_of(stacks: &[RuntimeItemStack], item_id: i32) -> i32 {
    stacks
        .iter()
        .filter(|stack| stack.item_id() == Some(item_id))
        .map(RuntimeItemStack::count)
        .sum()
}

/// One wandering trader with one known offer, traded with through the session
/// packets the merchant screen sends: interact, select the trade, shift-click
/// the result until the offer runs out, close, then save and reload.
#[test]
fn a_session_trades_with_a_wandering_trader_until_the_offer_runs_out() {
    let mut config = config_with_sets(
        [
            json!({"trades": "test:apple", "amount": 1}),
            empty_set(),
            empty_set(),
        ],
        &[(
            "test:apple",
            json!({
                "wants": {"id": "emerald", "count": 2},
                "gives": {"id": "apple", "count": 3},
                "max_uses": 2,
                "xp": 1
            }),
        )],
    );
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup.advancements.advancements = [("test:trade", 2), ("test:wrong_trade", 3)]
        .into_iter()
        .map(
            |(id, uses)| bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&json!({
                    "criteria": {"traded": {
                        "trigger": "minecraft:villager_trade",
                        "conditions": {
                            "item": {"items": "minecraft:apple"},
                            "villager": [{
                                "condition": "minecraft:entity_properties",
                                "entity": "this",
                                "predicate": {"nbt": format!("{{Offers:{{Recipes:[{{uses:{uses}}}]}}}}")}
                            }]
                        }
                    }}
                }))
                .unwrap(),
            },
        )
        .collect();
    let root = std::env::temp_dir().join(format!("bbb-merchant-trading-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "merchant-trading");
    let candidate = storage.load_candidate().unwrap();
    let mut world = crate::tests::spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, PLAYER.0, "alice", 1);
    let play = |world: &mut ServerWorld, packet| -> TickResult {
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet,
            })
            .unwrap();
        world.tick(receiver.drain())
    };
    for (slot, count, rarity) in [
        (36, 2, None),
        (37, 3, Some("common")),
        (38, 7, Some("uncommon")),
    ] {
        let mut item = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_EMERALD_ID),
            count,
            component_patch: Default::default(),
        });
        if let Some(rarity) = rarity {
            item = item
                .with_component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                    Some(&NbtSummaryValue::String(rarity.into())),
                    &world.item_stack_resolver(),
                )
                .unwrap();
        }
        world.stock_menu_slot_for_test(PLAYER, slot, item.summary().clone());
    }
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let player_position = world.levels[PLAYER].position();
    let player_entity = world.levels[PLAYER].entity_id();
    let (trader, trader_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        Vec3d {
            x: player_position.x + 1.5,
            ..player_position
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID).unwrap(),
        ),
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());

    let opened = play(
        &mut world,
        PlayServerbound::Interact(InteractEntity {
            entity_id: trader,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let screen = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::OpenScreen(screen),
                ..
            } => Some(screen.clone()),
            _ => None,
        })
        .expect("mobInteract opens the merchant menu");
    assert_eq!(screen.menu_type_id, VANILLA_MENU_TYPE_MERCHANT_ID);
    let offers = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::MerchantOffers(offers),
                ..
            } => Some(offers.clone()),
            _ => None,
        })
        .expect("openTradingScreen sends the offers");
    assert_eq!(offers.container_id, screen.container_id);
    assert_eq!((offers.villager_level, offers.villager_xp), (1, 0));
    assert!(!offers.show_progress && !offers.can_restock);
    let [offer] = offers.offers.as_slice() else {
        panic!("one offer: {:?}", offers.offers)
    };
    assert_eq!(
        (offer.buy_a.item_id, offer.buy_a.count),
        (VANILLA_ITEM_EMERALD_ID, 2)
    );
    assert_eq!(
        (offer.sell.item_id, offer.sell.count),
        (Some(VANILLA_ITEM_APPLE_ID), 3)
    );
    assert_eq!((offer.uses, offer.max_uses, offer.xp), (0, 2, 1));
    assert_eq!(
        world.player_stat_for_test(PLAYER, custom_stat("minecraft:talked_to_villager")),
        1
    );
    assert_eq!(
        world
            .level_by_id(&world.require_level_id(&key))
            .entities
            .get(trader)
            .unwrap()
            .merchant_trading_player(),
        Some(player_entity)
    );

    // `MerchantMenu.tryMoveItems` pulls every emerald into the first payment
    // slot and `updateSellItem` fills the result.
    play(
        &mut world,
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
    );
    let menu = &world.levels[PLAYER].inventory;
    assert_eq!(
        count_of(&menu.container_items()[..1], VANILLA_ITEM_EMERALD_ID),
        5
    );
    assert_eq!(
        count_of(&menu.container_items()[2..], VANILLA_ITEM_APPLE_ID),
        3
    );
    assert_eq!(
        count_of(&menu.inventory_slots(), VANILLA_ITEM_EMERALD_ID),
        7
    );

    // Shift-clicking the result repeats while the result slot refills, which
    // is twice: the second trade uses the offer up.
    let state_id = world.levels[PLAYER].inventory.state_id();
    play(
        &mut world,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: screen.container_id,
            state_id,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let menu = &world.levels[PLAYER].inventory;
    assert_eq!(count_of(&menu.inventory_slots(), VANILLA_ITEM_APPLE_ID), 6);
    assert_eq!(
        count_of(&menu.container_items()[..2], VANILLA_ITEM_EMERALD_ID),
        1
    );
    assert!(
        menu.container_items()[2].is_empty(),
        "the offer is out of stock"
    );
    let stored = world
        .level_by_id(&world.require_level_id(&key))
        .entities
        .get(trader)
        .unwrap();
    assert_eq!(stored.merchant_offers().unwrap().offers()[0].uses(), 2);
    assert_eq!(
        world.player_stat_for_test(PLAYER, custom_stat("minecraft:traded_with_villager")),
        2
    );
    assert_eq!(
        world.player_stat_for_test(
            PLAYER,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, VANILLA_ITEM_APPLE_ID)
        ),
        6
    );
    assert!(world.player_advancement_is_done(PLAYER, "test:trade"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:wrong_trade"));
    // `WanderingTrader.rewardTradeXp`: one orb of 3..=6 per trade, which the
    // player may already have touched.
    let entities = &world.level_by_id(&world.require_level_id(&key)).entities;
    let orb_xp: i32 = entities
        .ids()
        .into_iter()
        .filter_map(|id| match entities.get(id)?.data() {
            EntityData::ExperienceOrb(orb) => Some(orb.value() * orb.count()),
            _ => None,
        })
        .sum();
    let rewarded = orb_xp + world.levels[PLAYER].experience.total();
    assert!((6..=12).contains(&rewarded), "{rewarded}");

    // `MerchantMenu.removed` hands the payment back and frees the trader.
    play(
        &mut world,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    assert_eq!(
        count_of(
            &world.levels[PLAYER].inventory.inventory_slots(),
            VANILLA_ITEM_EMERALD_ID
        ),
        8
    );
    assert_eq!(
        world
            .level_by_id(&world.require_level_id(&key))
            .entities
            .get(trader)
            .unwrap()
            .merchant_trading_player(),
        None
    );

    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    let restored = crate::tests::spawn_pinned_restore(config, &storage.load_candidate().unwrap());
    let entities = &restored
        .level_by_id(&restored.require_level_id(&key))
        .entities;
    let trader = entities
        .ids()
        .into_iter()
        .find(|id| entities.get(*id).unwrap().uuid() == trader_uuid)
        .unwrap();
    let offer = &entities
        .get(trader)
        .unwrap()
        .merchant_offers()
        .unwrap()
        .offers()[0];
    assert_eq!((offer.uses(), offer.max_uses()), (2, 2));
    assert!(offer.is_out_of_stock());
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn merchant_world_predicates_allow_session_open_and_lazy_saved_offers() {
    let config = config_with_sets(
        [
            json!({"trades": "test:clear_plains", "amount": 1}),
            empty_set(),
            empty_set(),
        ],
        &[(
            "test:clear_plains",
            json!({
                "wants": {"id": "emerald"},
                "gives": "apple",
                "merchant_predicate": {
                    "condition": "minecraft:all_of",
                    "terms": [
                        {"condition": "minecraft:weather_check", "raining": false},
                        {"condition": "minecraft:location_check", "predicate": {
                            "biomes": "minecraft:plains",
                            "light": {"light": {"min": 0, "max": 15}}
                        }}
                    ]
                }
            }),
        )],
    );
    let root =
        std::env::temp_dir().join(format!("bbb-merchant-environment-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "merchant-environment");
    let candidate = storage.load_candidate().unwrap();
    let mut world = crate::tests::spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, PLAYER.0, "alice", 1);
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let dimension = world.require_level_id(&key);
    let position = Vec3d {
        x: world.levels[PLAYER].position().x + 1.5,
        ..world.levels[PLAYER].position()
    };
    assert_eq!(
        crate::world::natural_spawn::fuzzy_biome_resource_id(
            &world.registries,
            &world.level_by_id(&dimension).chunks,
            bbb_server_worldgen::noise::biome::biome_zoom_seed(world.cold.seed),
            BlockPos {
                x: position.x.floor() as i32,
                y: position.y.floor() as i32,
                z: position.z.floor() as i32
            },
        ),
        Some("minecraft:plains"),
    );
    assert!(!world.level_by_id(&dimension).weather.is_raining());
    let spawn = |world: &mut ServerWorld| {
        world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            position,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID).unwrap(),
            ),
        )
    };
    let (trader, _) = spawn(&mut world);
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::Interact(InteractEntity {
                entity_id: trader,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        })
        .unwrap();
    let opened = world.tick(receiver.drain());
    assert!(
        opened.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::OpenScreen(screen), .. }
                if screen.menu_type_id == VANILLA_MENU_TYPE_MERCHANT_ID
        )),
        "the clear plains merchant opens its menu"
    );
    let offers = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::MerchantOffers(offers),
                ..
            } => Some(offers),
            _ => None,
        })
        .expect("the matching predicate supplies the offer");
    assert_eq!(offers.offers.len(), 1);
    assert_eq!(offers.offers[0].sell.item_id, Some(VANILLA_ITEM_APPLE_ID));

    let (uninitialized, uuid) = spawn(&mut world);
    assert!(world
        .level_by_id(&dimension)
        .entities
        .get(uninitialized)
        .unwrap()
        .merchant_offers()
        .is_none());
    let request = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let restored = crate::tests::spawn_pinned_restore(config, &storage.load_candidate().unwrap());
    let entities = &restored
        .level_by_id(&restored.require_level_id(&key))
        .entities;
    let saved = entities
        .ids()
        .into_iter()
        .find_map(|id| entities.get(id).filter(|entity| entity.uuid() == uuid))
        .unwrap();
    let offers = saved.merchant_offers().unwrap().offers();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].result().item_id(), Some(VANILLA_ITEM_APPLE_ID));

    world
        .level_mut_by_id(&dimension)
        .weather
        .prepare(bbb_server_persistence::Java26WeatherData {
            raining: true,
            ..Default::default()
        });
    let (wet_trader, _) = spawn(&mut world);
    world.initialize_merchant_offers(&key, wet_trader).unwrap();
    assert!(world
        .level_by_id(&dimension)
        .entities
        .get(wet_trader)
        .unwrap()
        .merchant_offers()
        .unwrap()
        .offers()
        .is_empty());
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

/// `MerchantMenu.removed` (`MerchantMenu.java:150-169`) for a body that died
/// with the screen open: the payment drops with the body instead of vanishing
/// at the respawn.
#[test]
fn a_session_killed_while_trading_drops_its_payment_with_the_body() {
    let config = config_with_sets(
        [
            json!({"trades": "test:apple", "amount": 1}),
            empty_set(),
            empty_set(),
        ],
        &[(
            "test:apple",
            json!({
                "wants": {"id": "emerald", "count": 2},
                "gives": {"id": "apple", "count": 3},
                "max_uses": 2,
                "xp": 1
            }),
        )],
    );
    let root = std::env::temp_dir().join(format!("bbb-merchant-death-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "merchant-death");
    let mut world = crate::tests::spawn_pinned_restore(config, &storage.load_candidate().unwrap());
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, PLAYER.0, "alice", 1);
    let play = |world: &mut ServerWorld, packet| -> TickResult {
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet,
            })
            .unwrap();
        world.tick(receiver.drain())
    };
    world.stock_menu_slot_for_test(
        PLAYER,
        36,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_EMERALD_ID),
            count: 5,
            component_patch: Default::default(),
        },
    );
    let player_position = world.levels[PLAYER].position();
    let (trader, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        Vec3d {
            x: player_position.x + 1.5,
            ..player_position
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID).unwrap(),
        ),
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    play(
        &mut world,
        PlayServerbound::Interact(InteractEntity {
            entity_id: trader,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    play(
        &mut world,
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
    );
    assert_eq!(
        count_of(
            &world.levels[PLAYER].inventory.container_items()[..2],
            VANILLA_ITEM_EMERALD_ID
        ),
        5
    );

    world.hurt_connected_player(
        PLAYER,
        "minecraft:generic_kill",
        f32::MAX,
        None,
        None,
        None,
        &mut Vec::new(),
    );
    for _ in 0..21 {
        world.tick(receiver.drain());
    }
    let dropped: i32 = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let EntityData::Item(item) = entity.data() else {
                return None;
            };
            (item.item().item_id() == Some(VANILLA_ITEM_EMERALD_ID)).then(|| item.item().count())
        })
        .sum();
    assert_eq!(dropped, 5);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_saved_instrument_cost_accepts_the_default_horn_and_rejects_changed_or_removed_components() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_GOAT_HORN_ID;

    let config = config_with_sets([empty_set(), empty_set(), empty_set()], &[]);
    let root = std::env::temp_dir().join(format!("bbb-merchant-instrument-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "merchant-instrument");
    let mut world = crate::tests::spawn_pinned_restore(config, &storage.load_candidate().unwrap());
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, PLAYER.0, "alice", 1);
    let play = |world: &mut ServerWorld, packet| -> TickResult {
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet,
            })
            .unwrap();
        world.tick(receiver.drain())
    };
    let resolver = world.item_stack_resolver();
    let saved = bbb_protocol::nbt_summary::json_value_to_nbt_summary(&json!({
        "Recipes": [{
            "buy": {"id": "minecraft:goat_horn", "count": 1, "components": {
                "minecraft:instrument": "minecraft:ponder_goat_horn"
            }},
            "sell": {"id": "minecraft:apple", "count": 3},
            "maxUses": 1
        }]
    }))
    .unwrap();
    let offers = crate::entity::merchant::read_offers(&saved, &resolver).unwrap();
    assert_eq!(offers.offers().len(), 1);
    let mut trader_data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID).unwrap();
    assert!(trader_data.initialize_merchant_offers(offers));
    let player_position = world.levels[PLAYER].position();
    let (trader, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        Vec3d {
            x: player_position.x + 1.5,
            ..player_position
        },
        EntityData::GeneralLiving(trader_data),
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    play(
        &mut world,
        PlayServerbound::Interact(InteractEntity {
            entity_id: trader,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let horn = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
        count: 1,
        component_patch: Default::default(),
    });
    for instrument in [
        Some(NbtSummaryValue::String("minecraft:sing_goat_horn".into())),
        None,
    ] {
        let changed = horn
            .clone()
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                instrument.as_ref(),
                &resolver,
            )
            .unwrap();
        world.stock_menu_slot_for_test(PLAYER, 36, changed.summary().clone());
        play(
            &mut world,
            PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
        );
        let menu = &world.levels[PLAYER].inventory;
        assert!(menu.container_items()[0].is_empty());
        assert!(menu.container_items()[2].is_empty());
    }
    world.stock_menu_slot_for_test(PLAYER, 36, horn.summary().clone());
    play(
        &mut world,
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
    );
    let menu = &world.levels[PLAYER].inventory;
    assert_eq!(
        menu.container_items()[0].item_id(),
        Some(VANILLA_ITEM_GOAT_HORN_ID)
    );
    assert_eq!(
        menu.container_items()[2].item_id(),
        Some(VANILLA_ITEM_APPLE_ID)
    );
    let container_id = menu.container_id();
    let state_id = menu.state_id();
    play(
        &mut world,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let menu = &world.levels[PLAYER].inventory;
    assert!(menu
        .container_items()
        .iter()
        .all(RuntimeItemStack::is_empty));
    assert_eq!(count_of(&menu.inventory_slots(), VANILLA_ITEM_APPLE_ID), 3);
    assert_eq!(
        count_of(&menu.inventory_slots(), VANILLA_ITEM_GOAT_HORN_ID),
        0
    );
    let _ = std::fs::remove_dir_all(root);
}
