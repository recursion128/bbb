use super::*;

fn hero_config(condition: Option<serde_json::Value>) -> ServerWorldConfig {
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    let element = registry
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:gameplay/hero_of_the_village/unemployed_gift")
        .unwrap();
    element.value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type": "minecraft:gift",
            "pools": [{"rolls": 1, "conditions": condition.into_iter().collect::<Vec<_>>(), "entries": [{
                "type": "minecraft:item", "name": "minecraft:diamond",
                "functions": [{"function": "minecraft:set_components",
                    "components": {"minecraft:max_stack_size": 7}}]
            }]}]
        }), None)
        .unwrap(),
    );
    config
}

fn gifts(village: &Village) -> Vec<i32> {
    village.world.stored_entity_ids_for_test(OVERWORLD).into_iter().filter(|id|
        matches!(village.entity(*id).map(|entity| entity.data()),
            Some(EntityData::Item(item)) if item.item().item_resource_id() == Some("minecraft:diamond")))
        .collect()
}

fn movement_speed(village: &mut Village, villager: i32, speed: f64) {
    let uuid = village.entity(villager).unwrap().uuid();
    village.command(&format!(
        "attribute {uuid} minecraft:movement_speed base set {speed}"
    ));
}

#[test]
fn a_villager_approaches_a_hero_and_throws_the_datapack_gift_with_its_components() {
    let mut village = Village::with_config(hero_config(None));
    village.command("time set noon");
    village.command("gamerule advance_time false");
    village.command("gamerule mob_drops false");
    village.command("effect give mayor minecraft:hero_of_the_village infinite");
    let villager = village.villager(village.at(8, 0));
    movement_speed(&mut village, villager, 0.0);
    village.tick_until(630, |_| false);
    assert!(gifts(&village).is_empty());
    let before = village.entity(villager).unwrap().position();
    movement_speed(&mut village, villager, 0.5);
    let player = village.world.publication().players[0].position;
    let mut closest = f64::MAX;
    let gifted = village.tick_until(500, |village| {
        closest = closest.min(village.distance(villager, player));
        !gifts(village).is_empty()
    });
    assert!(
        gifted,
        "villager {:?}, player {:?}, before {:?}, closest {closest}",
        village.entity(villager).unwrap().position(),
        village.world.publication().players[0].position,
        before
    );
    assert!(closest < 5.0);
    let gift = village.entity(gifts(&village)[0]).unwrap();
    let EntityData::Item(item) = gift.data() else {
        unreachable!()
    };
    assert_eq!(item.item().max_stack_size().unwrap(), 7);
    let player = village.world.publication().players[0].position;
    assert!(gift.delta_movement().x * (player.x - gift.position().x) > 0.0);
}

#[test]
fn a_hero_gift_reads_block_nbt_at_the_offset_givers_position() {
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let chest = BlockPos {
        x: x + 3,
        y: y - 2,
        z,
    };
    let condition = serde_json::json!({
        "condition":"minecraft:location_check", "offsetY":-2,
        "predicate":{"block":{"nbt":format!(
            "{{id:'minecraft:chest',x:{},y:{},z:{},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}",
            chest.x, chest.y, chest.z
        )}}
    });
    let table = serde_json::json!({"type":"minecraft:gift","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[condition, {"condition":"minecraft:weather_check","raining":false}],
         "entries":[{"type":"minecraft:item","name":"minecraft:diamond",
             "functions":[{"function":"minecraft:set_components",
                 "components":{"minecraft:max_stack_size":7}}]}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-hero-gift-weather-{}", Uuid::new_v4()));
    let pack = root.join("gift");
    let directory = pack.join("data/minecraft/loot_table/gameplay/hero_of_the_village");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"hero gift weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        directory.join("unemployed_gift.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        game_type: GameType::Creative,
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    };
    std::fs::remove_dir_all(root).unwrap();
    for raining in [false, true] {
        let mut village = Village::with_config(config.clone());
        village.command("time set noon");
        village.command("gamerule advance_time false");
        village.command("gamerule mob_drops false");
        village.command("effect give mayor minecraft:hero_of_the_village infinite");
        for (index, command) in [
            format!("setblock {} {} {} chest", chest.x, chest.y, chest.z),
            format!(
                "item replace block {} {} {} container.0 with diamond 3",
                chest.x, chest.y, chest.z
            ),
        ]
        .iter()
        .enumerate()
        {
            let result = console_command(
                &village.sender,
                &village.receiver,
                &mut village.world,
                100 + index as u64,
                command,
            );
            let answer = console_command_answer(&result);
            assert_eq!(answer.0, Some(1), "{command}: {answer:?}");
        }
        village.command(if raining {
            "weather rain 100000t"
        } else {
            "weather clear 100000t"
        });
        assert!(village.tick_until(200, |village| village
            .world
            .level_weather(OVERWORLD)
            .is_raining()
            == raining));
        let villager = village.villager(village.at(0, 0));
        movement_speed(&mut village, villager, 0.0);
        assert!(village.tick_until(800, |village| village
            .world
            .stored_items_for_test(OVERWORLD)
            .iter()
            .any(|item| item.2 == "minecraft:apple")));
        let mut actual: Vec<_> = village
            .world
            .stored_items_for_test(OVERWORLD)
            .into_iter()
            .map(|item| (item.2, item.3))
            .collect();
        actual.sort();
        let mut expected = vec![("minecraft:apple".to_owned(), 1)];
        if !raining {
            expected.push(("minecraft:diamond".to_owned(), 1));
        }
        assert_eq!(actual, expected);
        if !raining {
            let gift = village.entity(gifts(&village)[0]).unwrap();
            let EntityData::Item(item) = gift.data() else {
                unreachable!()
            };
            assert_eq!(item.item().count(), 1);
            assert_eq!(item.item().max_stack_size().unwrap(), 7);
        }
    }
}

#[test]
fn a_nearer_nonhero_and_removing_the_hero_effect_prevent_gifts() {
    let mut village = Village::with_config(hero_config(None));
    village.command("time set noon");
    village.command("gamerule advance_time false");
    join_loaded(
        &village.sender,
        &village.receiver,
        &mut village.world,
        2,
        "hero",
        0x7122,
    );
    let hero = village.center(village.at(4, 0));
    village.command(&format!("tp hero {} {} {}", hero.x, hero.y, hero.z));
    village.command("effect give hero minecraft:hero_of_the_village infinite");
    let villager = village.villager(village.at(0, 0));
    movement_speed(&mut village, villager, 0.0);
    village.tick_until(800, |_| false);
    assert!(gifts(&village).is_empty());
    let hero = village.center(village.at(8, 0));
    village.command(&format!("tp hero {} {} {}", hero.x, hero.y, hero.z));
    village.command("gamemode spectator mayor");
    village.tick_until(630, |_| false);
    assert!(gifts(&village).is_empty());
    village.command("effect clear hero minecraft:hero_of_the_village");
    movement_speed(&mut village, villager, 0.5);
    village.tick_until(500, |_| false);
    assert!(gifts(&village).is_empty());
}
