use super::*;
use crate::entity::GeneralLivingData;
use crate::tests::{created_fixture_save, datapack_config, spawn_pinned_restore};
use bbb_protocol::packets::Vec3d;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_persistence::RegionCompression;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::Java26BuiltinDimension;
use serde_json::{json, Value};
use std::sync::Arc;

fn saved(stack: &RuntimeItemStack) -> Vec<bbb_protocol::NbtSummaryEntry> {
    let Some(bbb_protocol::NbtSummaryValue::Compound(fields)) = stack
        .write(crate::world::registry_runtime::item_stack_resolver_for_test())
        .unwrap()
    else {
        return Vec::new();
    };
    match fields.into_iter().find(|field| field.name == "components") {
        Some(bbb_protocol::NbtSummaryEntry {
            value: bbb_protocol::NbtSummaryValue::Compound(components),
            ..
        }) => components,
        _ => Vec::new(),
    }
}
fn saved_component(stack: &RuntimeItemStack, id: &str) -> Option<bbb_protocol::NbtSummaryValue> {
    saved(stack)
        .into_iter()
        .find(|entry| entry.name == id)
        .map(|entry| entry.value)
}
fn template_stack(world: &ServerWorld, id: &str) -> RuntimeItemStack {
    let template = &world.cold.config.datapacks.as_ref().unwrap().trades[id].gives;
    let source = serde_json::to_vec(&json!({
        "id": template.item_id, "count": template.count, "components": bbb_registry_codec::RegistryJsonValue::Object(template.components.clone()),
    }))
    .unwrap();
    let data = bbb_registry_codec::decode_direct_registry_json(&source).unwrap();
    let value = bbb_registry_codec::json_to_nbt(data.json().unwrap()).unwrap();
    RuntimeItemStack::try_read(&value, &world.item_stack_resolver()).unwrap()
}
fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
fn spawn(world: &mut ServerWorld) -> i32 {
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID).unwrap(),
            ),
        )
}
fn offers(world: &ServerWorld, id: i32) -> Option<&MerchantOffers> {
    match world.levels[&world.require_level_id(&key())]
        .entities
        .get(id)
        .unwrap()
        .data()
    {
        EntityData::GeneralLiving(data) => data.merchant_offers(),
        _ => unreachable!(),
    }
}
pub(in crate::world) fn config_with_sets(
    sets: [Value; 3],
    trades: &[(&str, Value)],
) -> super::super::ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let make = |id: String, value: Value| RawRegistryElement {
        id,
        source_pack: "test".into(),
        source_json: serde_json::to_vec(&value).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
                .unwrap(),
        ),
        codec_value: None,
    };
    startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:trade_set")
        .unwrap()
        .elements = ["buying", "uncommon", "common"]
        .into_iter()
        .zip(sets)
        .map(|(suffix, value)| make(format!("minecraft:wandering_trader/{suffix}"), value))
        .collect();
    startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:villager_trade")
        .unwrap()
        .elements = trades
        .iter()
        .map(|(id, value)| make((*id).into(), value.clone()))
        .collect();
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(startup);
    startup.trades =
        bbb_server_datapack::trades::load_trade_catalog(&startup.registries, &lookup).unwrap();
    config
}
pub(in crate::world) fn empty_set() -> Value {
    json!({"trades":[],"amount":0})
}
fn simple_trade(item: &str) -> Value {
    json!({"wants":{"id":"emerald"}, "gives":item})
}

#[test]
fn merchant_builtin_catalog_generates_every_trade_with_typed_modifiers() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    let villager = {
        let level = world
            .levels
            .get_mut(&world.require_level_id(&key()))
            .unwrap();
        level.entities.spawn(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID)
                    .unwrap(),
            ),
        )
    };
    let resolver = world.item_stack_resolver();
    let level_id = world.require_level_id(&key());
    let ServerWorld {
        cold,
        levels,
        maps,
        datapack_tables,
        world_clocks,
        registries,
        scoreboard,
        ..
    } = &mut world;
    let super::super::WorldCold {
        config,
        command_storage,
        dimension_sky_lights,
        level_dimension_types,
        loot_item_tags,
        seed,
        ..
    } = &**cold;
    let enchantment_registry = &datapack_tables.enchantment_registry;
    let startup = config.datapacks.as_deref().unwrap();
    let (level, other_levels) = levels.split_mut(&level_id).unwrap();
    let super::super::ServerLevel {
        chunks,
        entities,
        weather,
        sky_darkening,
        ..
    } = level;
    let environment = TradeItemEnvironment {
        startup,
        tables: datapack_tables,
        registry_tags: chunks.registry_tags_shared(),
        resolver: &resolver,
        enchantments: enchantment_registry,
    };
    let chunks = RefCell::new(chunks);
    let maps = RefCell::new(maps);
    let level_environment = LevelLootEnvironment::for_world_predicates(
        LootChunks::Chunks(&chunks),
        &resolver,
        Some(startup),
        enchantment_registry,
        Some(crate::entity::block_drops::RecipeDropFacts {
            recipes: &startup.recipes,
            recipe_index: &startup.recipe_index,
            item_tags: loot_item_tags,
        }),
        world_clocks,
        command_storage,
        registries,
        *seed,
        sky_darkening.map(i32::from),
        dimension_sky_lights.loot_environment_context(
            level_dimension_types.get(&level_id).map(String::as_str),
            registries,
            world_clocks,
            ScalarWeather::from_level(weather),
        ),
        LootEntityScoreboard {
            scoreboard: scoreboard.board(),
            names: other_levels.names(),
            dimension: level_id,
            bodies: Some(EntityBodyReadView::new(entities, None, None)),
            other_levels: LootEntityLevels::Other(other_levels),
        },
    )
    .with_maps(&maps, *seed, "minecraft:overworld", false);
    let params = LootParams {
        origin: Some(LootOrigin {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        }),
        entities: LootEntities {
            this_entity: LootEntity::Stored(StoredLootEntity {
                entity: entities.get(villager).unwrap(),
                bodies: None,
                raid_at_position: None,
            }),
            ..Default::default()
        },
        additional_cost_component_allowed: true,
        item_environment: Some(&level_environment),
        ..LootParams::new(param_sets::VILLAGER_TRADE)
    };
    let random = GameRandom::with_seed(42);
    let (mut generated, mut refused) = (0, 0);
    for raw in &startup
        .registries
        .registry("minecraft:villager_trade")
        .unwrap()
        .elements
    {
        let trade = startup.trades.get(&raw.id).unwrap();
        let mut evaluator = LootEvaluator::new(&startup.loot, &raw.id, &params, &random).unwrap();
        let Some(offer) = generate_offer(&environment, &mut evaluator, trade)
            .unwrap_or_else(|error| panic!("{}: {error}", raw.id))
        else {
            // A plains villager is refused the other types' trades, and a
            // world without structures every exploration map.
            refused += 1;
            continue;
        };
        assert!(offer.base_cost_a().count() > 0);
        assert!(saved_component(offer.result(), "minecraft:additional_trade_cost").is_none());
        if raw.id.ends_with("emerald_enchanted_iron_pickaxe") {
            assert!(offer.base_cost_a().count() >= 6);
            assert!(saved_component(offer.result(), "minecraft:enchantments").is_some());
        }
        if raw.id.ends_with("emerald_long_invisibility_potion") {
            assert!(format!("{:?}", saved(offer.result())).contains("minecraft:long_invisibility"));
        }
        if raw.id.ends_with("emerald_dyed_leather_chestplate") {
            assert!(saved_component(offer.result(), "minecraft:dyed_color").is_some());
        }
        generated += 1;
    }
    assert!(generated > 300, "{generated} generated, {refused} refused");
    assert!(refused < 40, "{generated} generated, {refused} refused");
}

#[test]
fn merchant_first_save_retry_and_filesystem_restore_preserve_offers_and_sequences() {
    let root = std::env::temp_dir().join(format!("bbb-merchant-offers-{}", std::process::id()));
    let storage = created_fixture_save(&root, "merchant-offers");
    let candidate = storage.load_candidate().unwrap();
    let pack = root.join("datapacks/raw");
    std::fs::create_dir_all(pack.join("data/minecraft/trade_set/wandering_trader")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/villager_trade")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), br#"{"pack":{"description":"raw trades","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for suffix in ["buying", "uncommon", "common"] {
        std::fs::write(pack.join(format!("data/minecraft/trade_set/wandering_trader/{suffix}.json")),
            br#"{"trades":["test:raw"],"amount":3,"allow_duplicates":true,"random_sequence":"test:raw","\ud800":"\udc00"}"#).unwrap();
    }
    std::fs::write(pack.join("data/test/villager_trade/raw.json"),
        br#"{"wants":{"id":"emerald","components":{"minecraft:custom_data":{"n":1,"flag":true,"\udc00":"\ud800"}}},"gives":{"id":"stone","components":{"minecraft:custom_name":{"text":"\ud800"},"minecraft:custom_data":{"n":1,"flag":true,"\udc00":"\ud800"}}},"\ud800":"\udc00"}"#).unwrap();
    let options = bbb_server_datapack::startup::DatapackStartupOptions {
        datapacks_dir: Some(root.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    };
    let mut config = datapack_config();
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap(),
    ));
    let mut world = spawn_pinned_restore(config, &candidate);
    let id = spawn(&mut world);
    let uuid = world.levels[&world.require_level_id(&key())]
        .entities
        .get(id)
        .unwrap()
        .uuid();
    assert!(offers(&world, id).is_none());
    let first = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    let generated = offers(&world, id).unwrap().clone();
    assert_eq!(generated.offers().len(), 9);
    let component =
        saved_component(generated.offers()[0].result(), "minecraft:custom_name").unwrap();
    let bbb_protocol::NbtSummaryValue::Compound(fields) = component else {
        panic!("name component");
    };
    let bbb_protocol::NbtSummaryValue::String(raw) = &fields
        .iter()
        .find(|field| field.name == "text")
        .unwrap()
        .value
    else {
        panic!("raw name text");
    };
    assert_eq!(raw.units(), [0xd800]);
    let check_data = |value: &bbb_protocol::NbtSummaryValue| {
        let bbb_protocol::NbtSummaryValue::Compound(fields) = value else {
            panic!("custom data");
        };
        for name in ["n", "flag"] {
            assert_eq!(
                fields
                    .iter()
                    .find(|field| field.name == name)
                    .unwrap()
                    .value,
                bbb_protocol::NbtSummaryValue::Byte(1)
            );
        }
        let bbb_protocol::NbtSummaryValue::String(raw) = &fields
            .iter()
            .find(|field| field.name.units() == [0xdc00])
            .unwrap()
            .value
        else {
            panic!("raw custom data");
        };
        assert_eq!(raw.units(), [0xd800]);
    };
    check_data(&saved_component(generated.offers()[0].result(), "minecraft:custom_data").unwrap());
    check_data(&generated.offers()[0].base_cost_a().components()[0].value);
    world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(offers(&world, id), Some(&generated));
    let blocked_region = storage.paths().entities_dir().join("r.0.0.mca");
    std::fs::create_dir_all(&blocked_region).unwrap();
    assert!(storage.save_snapshot(&candidate, &first).is_err());
    assert_eq!(offers(&world, id), Some(&generated));
    std::fs::remove_dir(&blocked_region).unwrap();
    let retry = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &retry).unwrap();
    world.acknowledge_save_success(&retry);
    let datapacks =
        Arc::new(bbb_server_datapack::startup::DatapackStartup::load(&options).unwrap());
    let bootstrap = storage.load_bootstrap().unwrap();
    let reloaded = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        bootstrap,
        &datapacks.registries,
    )
    .unwrap();
    assert_eq!(reloaded.random_sequences, retry.random_sequences);
    let mut config = datapack_config();
    config.datapacks = Some(datapacks);
    let mut restored = spawn_pinned_restore(config, &reloaded);
    assert_eq!(
        restored.cold.random_sequences.borrow().saved(),
        reloaded.random_sequences
    );
    let id = restored.levels[&restored.require_level_id(&key())]
        .entities
        .ids()
        .into_iter()
        .find(|id| {
            restored.levels[&restored.require_level_id(&key())]
                .entities
                .get(*id)
                .unwrap()
                .uuid()
                == uuid
        })
        .unwrap();
    assert_eq!(offers(&restored, id), Some(&generated));
    restored
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    drop(storage);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn merchant_sets_keep_order_and_duplicate_policy_and_rejected_candidates() {
    let config = config_with_sets(
        [
            json!({"trades":["test:reject", "test:apple"], "amount":3}),
            json!({"trades":"test:carrot", "amount":2,"allow_duplicates":true}),
            json!({"trades":"test:potato", "amount":1}),
        ],
        &[
            (
                "test:reject",
                json!({"wants":{"id":"emerald","count":0},"gives":"stick"}),
            ),
            (
                "test:apple",
                json!({
                    "wants":{"id":"emerald"}, "gives":"apple",
                    "merchant_predicate":{
                        "condition":"location_check", "offsetX":1,
                        "predicate":{"block":{
                            "blocks":"minecraft:chest",
                            "nbt":"{id:'minecraft:chest',x:1,y:65,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"
                        }}
                    }
                }),
            ),
            (
                "test:carrot",
                json!({"wants":{"id":"emerald"},"gives":{"id":"carrot","components":{"lock":{"predicates":{"minecraft:max_damage":{}}}}}}),
            ),
            ("test:potato", simple_trade("potato")),
        ],
    );
    let mut world = ServerWorld::new(config).unwrap();
    let level = world.require_level_id(&key());
    world
        .level_mut_by_id(&level)
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let (_, chest) = bbb_server_types::block_state_parse::read_block_state(
        "chest{Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}",
        0,
    )
    .unwrap();
    let writes = world
        .place_command_blocks(
            level,
            false,
            false,
            crate::commands::CommandBlockNeighbourPass::EveryPosition,
            &[crate::commands::CommandBlockPlacement {
                pos: bbb_protocol::packets::BlockPos { x: 1, y: 65, z: 0 },
                block: Some(chest),
            }],
        )
        .unwrap();
    assert_eq!(writes.placed, 1);
    let id = spawn(&mut world);
    world.initialize_merchant_offers(&key(), id).unwrap();
    let names: Vec<_> = offers(&world, id)
        .unwrap()
        .offers()
        .iter()
        .map(|offer| offer.result().item_resource_id().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "minecraft:apple",
            "minecraft:carrot",
            "minecraft:carrot",
            "minecraft:potato"
        ]
    );
    let first = offers(&world, id).unwrap().offers()[1].result().clone();
    assert!(first.same_item_same_components(offers(&world, id).unwrap().offers()[2].result()));
    let other = spawn(&mut world);
    world.initialize_merchant_offers(&key(), other).unwrap();
    assert!(first.same_item_same_components(offers(&world, other).unwrap().offers()[1].result()));
    let reloaded = bbb_server_datapack::startup::DatapackStartup::reload(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
        world.cold.config.datapacks.as_ref().unwrap(),
    )
    .unwrap();
    assert!(first.same_item_same_components(&{
        let template = &reloaded.trades["test:carrot"].gives;
        let source = serde_json::to_vec(&json!({
            "id": template.item_id, "count": template.count,
            "components": bbb_registry_codec::RegistryJsonValue::Object(template.components.clone()),
        }))
        .unwrap();
        let data = bbb_registry_codec::decode_direct_registry_json(&source).unwrap();
        RuntimeItemStack::try_read(
            &bbb_registry_codec::json_to_nbt(data.json().unwrap()).unwrap(),
            &world.item_stack_resolver(),
        )
        .unwrap()
    }));
}

#[test]
fn merchant_missing_sets_and_empty_offers_are_valid_and_do_not_reroll_live() {
    let mut config = config_with_sets([empty_set(), empty_set(), empty_set()], &[]);
    Arc::make_mut(config.datapacks.as_mut().unwrap())
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:trade_set")
        .unwrap()
        .elements
        .clear();
    let mut world = ServerWorld::new(config).unwrap();
    let id = spawn(&mut world);
    world.initialize_merchant_offers(&key(), id).unwrap();
    assert!(offers(&world, id).unwrap().is_empty());
    world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
}

#[test]
fn merchant_book_conversion_double_price_and_transient_cost_survive_modifier_chain() {
    let config = config_with_sets(
        [
            json!({"trades":"test:book","amount":1}),
            empty_set(),
            empty_set(),
        ],
        &[(
            "test:book",
            json!({
                "wants":{"id":"emerald"},"additional_wants":{"id":"paper","count":999},
                "gives":{"id":"book","count":3},
                "max_uses":0,"xp":-2,"reputation_discount":-0.5,
                "double_trade_price_enchantments":"unbreaking",
                "given_item_modifiers":[
                    {"function":"enchant_with_levels","levels":12,"options":"unbreaking","include_additional_cost_component":true},
                    {"function":"set_item","item":"enchanted_book"},
                    {"function":"set_potion","id":"long_invisibility"}
                ]
            }),
        )],
    );
    let mut world = ServerWorld::new(config).unwrap();
    let id = spawn(&mut world);
    world.initialize_merchant_offers(&key(), id).unwrap();
    let offer = &offers(&world, id).unwrap().offers()[0];
    assert_eq!(offer.base_cost_a().count(), 25);
    assert_eq!(offer.cost_b().unwrap().count(), 64);
    assert_eq!(offer.max_uses(), 1);
    assert_eq!(offer.xp(), 0);
    assert_eq!(offer.price_multiplier(), 0.0);
    assert_eq!(
        offer.result().item_resource_id().unwrap(),
        "minecraft:enchanted_book"
    );
    assert_eq!(offer.result().count(), 1);
    assert!(saved_component(offer.result(), "minecraft:additional_trade_cost").is_none());
}

#[test]
fn merchant_potion_modifier_retains_custom_contents() {
    let config = config_with_sets(
        [
            json!({"trades":"test:potion","amount":1}),
            empty_set(),
            empty_set(),
        ],
        &[(
            "test:potion",
            json!({
                "wants":{"id":"emerald"}, "gives":{"id":"potion","components":{"potion_contents":{"potion":"water","custom_color":123,"custom_name":"kept"}}},
                "given_item_modifiers":[{"function":"set_potion","id":"long_invisibility"}]
            }),
        )],
    );
    let mut world = ServerWorld::new(config).unwrap();
    let id = spawn(&mut world);
    world.initialize_merchant_offers(&key(), id).unwrap();
    let result = offers(&world, id).unwrap().offers()[0].result();
    let component = saved_component(result, "minecraft:potion_contents").unwrap();
    let bbb_protocol::NbtSummaryValue::Compound(entries) = component else {
        panic!("potion contents record")
    };
    assert!(entries.iter().any(|entry| entry.name == "custom_name"
        && entry.value == bbb_protocol::NbtSummaryValue::String("kept".into())));
    assert!(entries.iter().any(|entry| entry.name == "custom_color"
        && bbb_protocol::nbt_summary::codec_int_value(&entry.value) == Some(123)));
}

#[test]
fn merchant_enchantment_target_uses_result_item_and_respects_removed_component() {
    for enchanted_book in [true, false] {
        let gives = if enchanted_book {
            json!({"id":"enchanted_book","components":{"enchantable":{"value":20},"stored_enchantments":{"unbreaking":1},"lock":{"predicates":{"minecraft:max_damage":{}}}}})
        } else {
            json!({"id":"iron_pickaxe","components":{"!enchantments":{}}})
        };
        let mut config = config_with_sets(
            [
                json!({"trades":"test:tool","amount":1}),
                empty_set(),
                empty_set(),
            ],
            &[(
                "test:tool",
                json!({
                    "wants":{"id":"emerald"},"gives":gives,
                    "given_item_modifiers":[{"function":"enchant_with_levels","levels":20,"options":"unbreaking","include_additional_cost_component":true}]
                }),
            )],
        );
        if enchanted_book {
            let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
            let element = startup
                .registries
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:enchantment")
                .unwrap()
                .elements
                .iter_mut()
                .find(|element| element.id == "minecraft:unbreaking")
                .unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
            json["supported_items"] = json!("minecraft:enchanted_book");
            element.source_json = serde_json::to_vec(&json).unwrap().into();
            element.source = Some(
                bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap(),
            );
            let mut json: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
            json["primary_items"] = json!("minecraft:enchanted_book");
            element.source_json = serde_json::to_vec(&json).unwrap().into();
            element.source = Some(
                bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap(),
            );
            element.source_json = serde_json::to_vec(&element.json()).unwrap().into();
        }
        let mut world = ServerWorld::new(config).unwrap();
        let original = template_stack(&world, "test:tool");
        let id = spawn(&mut world);
        world.initialize_merchant_offers(&key(), id).unwrap();
        let offer = &offers(&world, id).unwrap().offers()[0];
        assert_eq!(offer.base_cost_a().count(), 21);
        if enchanted_book {
            let stored = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID;
            let reverted = offer
                .result()
                .clone()
                .with_component_nbt(
                    stored,
                    original
                        .component_nbt(stored, &world.item_stack_resolver())
                        .unwrap()
                        .as_ref(),
                    &world.item_stack_resolver(),
                )
                .unwrap();
            assert!(reverted.same_item_same_components(&original));
            let bbb_protocol::NbtSummaryValue::Compound(entries) =
                saved_component(offer.result(), "minecraft:stored_enchantments").unwrap()
            else {
                panic!("stored enchantments")
            };
            assert!(entries
                .iter()
                .any(|entry| entry.name == "minecraft:unbreaking"
                    && bbb_protocol::nbt_summary::codec_int_value(&entry.value).unwrap() > 1));
            assert!(saved_component(offer.result(), "minecraft:enchantments").is_none());
        } else {
            assert!(saved_component(offer.result(), "!minecraft:enchantments").is_some());
        }
    }
}

/// `SetComponentsFunction.run` (`SetComponentsFunction.java:29-32`) through a
/// real trade: `given_item_modifiers` runs over the generated stack, so a
/// component is set and a `!`-prefixed one is removed before the offer is
/// built.
#[test]
fn a_trade_item_modifier_sets_and_removes_components_on_the_generated_stack() {
    let config = config_with_sets(
        [
            empty_set(),
            json!({"trades": "test:sword", "amount": 1}),
            empty_set(),
        ],
        &[(
            "test:sword",
            json!({
                "wants": {"id": "emerald"},
                "gives": {"id":"diamond_sword","components":{"lock":{"predicates":{"minecraft:max_damage":{}}}}},
                "given_item_modifiers": [{
                    "function": "set_components",
                    "components": {
                        "minecraft:damage": 7,
                        "!minecraft:max_damage": {}
                    }
                }]
            }),
        )],
    );
    let mut world = ServerWorld::new(config).unwrap();
    let id = spawn(&mut world);
    world.initialize_merchant_offers(&key(), id).unwrap();
    let offer = &offers(&world, id).unwrap().offers()[0];
    let resolver = world.item_stack_resolver();
    let held = template_stack(&world, "test:sword");
    let lock = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID;
    let copied = held
        .clone()
        .with_component_nbt(
            lock,
            offer
                .result()
                .component_nbt(lock, &resolver)
                .unwrap()
                .as_ref(),
            &resolver,
        )
        .unwrap();
    assert!(held.same_item_same_components(&copied));
    let summary = offer.result().summary().clone();
    assert_eq!(
        summary.item_id,
        Some(
            vanilla_item_type_for_resource_id("minecraft:diamond_sword")
                .expect("a built-in item")
                .id
        )
    );
    assert_eq!(
        summary.component_patch.damage,
        Some(7),
        "`DataComponentPatch.CODEC`'s set arm"
    );
    assert!(
        summary.component_patch.removed_type_ids.contains(
            &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
        ),
        "the `!` arm removes the component the prototype declared: {:?}",
        summary.component_patch.removed_type_ids
    );
    assert_eq!(summary.component_patch.max_damage, None);
}
