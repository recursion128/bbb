use super::*;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};
use serde_json::json;

fn number_table(attributes: &[&str], kind: &str) -> String {
    let values = attributes
        .iter()
        .map(|attribute| {
            json!({
                "type":"minecraft:environment_attribute", "attribute":attribute
            })
        })
        .collect::<Vec<_>>();
    json!({"type":kind,"pools":[{"rolls":1,"entries":[{
        "type":"minecraft:item","name":"minecraft:dirt","functions":[{
            "function":"minecraft:set_custom_model_data",
            "floats":{"mode":"replace_all","values":values}
        }]
    }]}]})
    .to_string()
}

fn bed_rule(color: &str) -> serde_json::Value {
    json!({"can_sleep":"when_dark","can_set_spawn":"always","explodes":false,
        "error_message":{"text":"bbb bed","color":color}})
}

fn background_music(delay: i32) -> serde_json::Value {
    json!({
        "default":{"sound":"minecraft:block.note_block.bell","min_delay":delay,
            "max_delay":120,"replace_current_music":true},
        "creative":{"sound":{"sound_id":"minecraft:block.note_block.bell","range":21.0},
            "min_delay":9,"max_delay":19},
        "underwater":{"sound":"minecraft:block.note_block.bell","min_delay":4,"max_delay":9}
    })
}

fn ambient_sounds(chance: f64) -> serde_json::Value {
    json!({
        "loop":{"sound_id":"minecraft:block.note_block.bell","range":19.0},
        "mood":{"sound":"minecraft:block.note_block.bell","tick_delay":80,
            "block_search_extent":8,"offset":2.0},
        "additions":[
            {"sound":"minecraft:block.note_block.bell","tick_chance":chance},
            {"sound":{"sound_id":"minecraft:block.note_block.bell","range":17.0},
                "tick_chance":0.125}
        ]
    })
}

fn item_particle(count: i32, marker: i32) -> serde_json::Value {
    json!({"type":"minecraft:item","item":{"id":"minecraft:goat_horn","count":count,
        "components":{"minecraft:instrument":"minecraft:ponder_goat_horn",
            "minecraft:custom_data":{"aaa":"first","bbb":marker.to_string()}}}})
}

fn dust_particle(scale: f64) -> serde_json::Value {
    json!({"type":"minecraft:dust","color":0x204060,"scale":scale})
}

fn ambient_particles(probability: f64) -> serde_json::Value {
    json!([
        {"particle":dust_particle(0.75),"probability":probability},
        {"particle":item_particle(2, 7),"probability":0.5}
    ])
}

fn particle_condition() -> serde_json::Value {
    json!({"condition":"minecraft:all_of","terms":[
        {"condition":"minecraft:environment_attribute_check",
            "attribute":"minecraft:visual/default_dripstone_particle","value":item_particle(2, 7)},
        {"condition":"minecraft:environment_attribute_check",
            "attribute":"minecraft:visual/ambient_particles","value":ambient_particles(0.25)}
    ]})
}

fn scalar_config() -> crate::world::ServerWorldConfig {
    let mut config = loot_config(
        "bbb:scalar_values",
        &number_table(
            &[
                "minecraft:visual/fog_end_distance",
                "minecraft:gameplay/turtle_egg_hatch_chance",
                "minecraft:visual/sun_angle",
            ],
            "minecraft:chest",
        ),
    );
    for (name, attribute, activity) in [
        (
            "adult",
            "minecraft:gameplay/villager_activity",
            "minecraft:work",
        ),
        (
            "baby",
            "minecraft:gameplay/baby_villager_activity",
            "minecraft:play",
        ),
    ] {
        add_loot_table(
            &mut config,
            &format!("bbb:{name}_activity"),
            &json!({
                "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                    "condition":"minecraft:environment_attribute_check",
                    "attribute":attribute,"value":activity
                }],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}]
            })
            .to_string(),
        );
    }
    add_loot_table(
        &mut config,
        "bbb:scalar_condition",
        &json!({
            "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[
                {"condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:gameplay/can_start_raid","value":true},
                {"condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:visual/fog_end_distance","value":25.0}
            ],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}]
        })
        .to_string(),
    );
    add_loot_table(&mut config, "bbb:nonpositional_scalar", &json!({
        "type":"minecraft:empty","pools":[{"rolls":1,"conditions":[
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/fast_lava","value":false},
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/sky_light_level","value":15.0}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:emerald","functions":[{
            "function":"minecraft:set_custom_model_data","floats":{"mode":"replace_all","values":[{
                "type":"minecraft:environment_attribute","attribute":"minecraft:gameplay/sky_light_level"
            }]}
        }]}]}]
    }).to_string());
    add_loot_table(
        &mut config,
        "bbb:weather_values",
        &number_table(
            &[
                "minecraft:gameplay/sky_light_level",
                "minecraft:visual/sky_light_factor",
                "minecraft:visual/star_brightness",
            ],
            "minecraft:chest",
        ),
    );
    add_loot_table(
        &mut config,
        "bbb:weather_bees",
        &json!({
            "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                "condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/bees_stay_in_hive","value":true
            }],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}]
        })
        .to_string(),
    );
    add_loot_table(
        &mut config,
        "bbb:third_quarter",
        &json!({
            "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                "condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:visual/moon_phase","value":"third_quarter"
            }],"entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]}]
        })
        .to_string(),
    );
    for (name, value) in [
        ("open", json!(true)),
        ("closed", json!(false)),
        ("default", json!("default")),
    ] {
        add_loot_table(
            &mut config,
            &format!("bbb:eyeblossom_{name}"),
            &json!({
                "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                    "condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:gameplay/eyeblossom_open","value":value
                }],"entries":[{"type":"minecraft:item","name":"minecraft:iron_ingot"}]}]
            })
            .to_string(),
        );
    }
    for (name, explodes) in [("red_bed", false), ("exploding_bed", true)] {
        let mut value = bed_rule("red");
        value["explodes"] = json!(explodes);
        add_loot_table(
            &mut config,
            &format!("bbb:{name}"),
            &json!({
                "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                    "condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:gameplay/bed_rule","value":value
                }],"entries":[{"type":"minecraft:item","name":"minecraft:copper_ingot"}]}]
            })
            .to_string(),
        );
    }
    for (name, attribute, value, item) in [
        (
            "background_music",
            "minecraft:audio/background_music",
            background_music(20),
            "minecraft:redstone",
        ),
        (
            "ambient_sounds",
            "minecraft:audio/ambient_sounds",
            ambient_sounds(0.25),
            "minecraft:lapis_lazuli",
        ),
    ] {
        add_loot_table(
            &mut config,
            &format!("bbb:{name}"),
            &json!({
                "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[{
                    "condition":"minecraft:environment_attribute_check",
                    "attribute":attribute,"value":value
                }],"entries":[{"type":"minecraft:item","name":item}]}]
            })
            .to_string(),
        );
    }
    add_loot_table(
        &mut config,
        "bbb:color",
        &json!({
            "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[
                {"condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:visual/sky_color","value":"#305070"},
                {"condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:visual/cloud_color","value":"#806090c0"}
            ],"entries":[{"type":"minecraft:item","name":"minecraft:amethyst_shard"}]}]
        })
        .to_string(),
    );
    let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(startup);
    let predicate =
        bbb_server_datapack::loot::parse_loot_condition(&particle_condition(), Some(&lookup))
            .unwrap();
    let table = bbb_server_datapack::loot::parse_loot_table(
        &json!({
            "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[particle_condition()],
                "entries":[{"type":"minecraft:item","name":"minecraft:prismarine_crystals"}]}]
        }),
        Some(&lookup),
    )
    .unwrap();
    let modifier = bbb_server_datapack::loot::parse_loot_function(
        &json!({
            "function":"minecraft:set_count","count":7,"conditions":[particle_condition()]
        }),
        Some(&lookup),
    )
    .unwrap();
    for (registry_id, value) in [
        (
            "minecraft:predicate",
            LootElementValue::Predicate(Box::new(predicate)),
        ),
        ("minecraft:loot_table", LootElementValue::Table(table)),
        (
            "minecraft:item_modifier",
            LootElementValue::Modifier(modifier),
        ),
    ] {
        let registry = startup
            .loot
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == registry_id)
            .unwrap();
        registry.elements.push(LootElement {
            id: "bbb:particles".into(),
            source_pack: "test".into(),
            value,
        });
        registry.elements.sort_by(|a, b| a.id.cmp(&b.id));
    }
    let predicates = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:predicate")
        .unwrap();
    predicates.elements.push(LootElement {
        id: "bbb:scalar_ready".into(),
        source_pack: "test".into(),
        value: LootElementValue::Predicate(Box::new(
            bbb_server_datapack::loot::parse_loot_condition(
                &json!({
                    "condition":"minecraft:environment_attribute_check",
                    "attribute":"minecraft:gameplay/can_start_raid","value":true
                }),
                None,
            )
            .unwrap(),
        )),
    });
    predicates.elements.sort_by(|a, b| a.id.cmp(&b.id));
    for registry in &mut startup.registries.registries {
        for element in &mut registry.elements {
            let selected = match registry.registry_id.as_str() {
                "minecraft:dimension_type" => {
                    matches!(
                        element.id.as_str(),
                        "minecraft:overworld" | "minecraft:the_nether"
                    )
                }
                "minecraft:worldgen/biome" => {
                    matches!(element.id.as_str(), "minecraft:plains" | "minecraft:desert")
                }
                "minecraft:timeline" => element.id == "minecraft:early_game",
                _ => false,
            };
            if !selected {
                continue;
            }
            let mut value: serde_json::Value =
                serde_json::from_slice(&element.source_json).unwrap();
            match registry.registry_id.as_str() {
                "minecraft:dimension_type" if element.id == "minecraft:the_nether" => {
                    value["attributes"]
                        .as_object_mut()
                        .unwrap()
                        .remove("minecraft:visual/fog_color");
                }
                "minecraft:dimension_type" => {
                    value["timelines"] = json!(["minecraft:early_game"]);
                    for (key, v) in [
                        ("minecraft:visual/sky_color", json!("#204060")),
                        ("minecraft:visual/cloud_color", json!("#80406080")),
                        ("minecraft:visual/fog_color", json!(0x0080a0c0)),
                        ("minecraft:visual/sunrise_sunset_color", json!("#604080c0")),
                        ("minecraft:visual/sky_light_color", json!("#c0d0e0")),
                        ("minecraft:visual/water_fog_color", json!("#112233")),
                        ("minecraft:visual/block_light_tint", json!("#445566")),
                        ("minecraft:visual/night_vision_color", json!("#778899")),
                        ("minecraft:visual/ambient_light_color", json!("#102030")),
                        ("minecraft:gameplay/can_start_raid", json!(false)),
                        ("minecraft:visual/fog_end_distance", json!(10.0)),
                        ("minecraft:gameplay/turtle_egg_hatch_chance", json!(0.75)),
                        ("minecraft:visual/sun_angle", json!(20.0)),
                        ("minecraft:visual/moon_phase", json!("new_moon")),
                        ("minecraft:gameplay/sky_light_level", json!(15.0)),
                        ("minecraft:visual/sky_light_factor", json!(0.75)),
                        ("minecraft:visual/star_brightness", json!(0.75)),
                        ("minecraft:gameplay/bees_stay_in_hive", json!(false)),
                        (
                            "minecraft:gameplay/villager_activity",
                            json!("minecraft:panic"),
                        ),
                        (
                            "minecraft:gameplay/baby_villager_activity",
                            json!("minecraft:panic"),
                        ),
                    ] {
                        value["attributes"][key] = v;
                    }
                }
                "minecraft:worldgen/biome" => {
                    let desert = element.id == "minecraft:desert";
                    for key in [
                        "minecraft:visual/default_dripstone_particle",
                        "minecraft:visual/ambient_particles",
                    ] {
                        value["attributes"].as_object_mut().unwrap().remove(key);
                    }
                    if desert {
                        value["attributes"]["minecraft:visual/default_dripstone_particle"] =
                            item_particle(4, 8);
                        value["attributes"]["minecraft:visual/ambient_particles"] =
                            ambient_particles(0.75);
                    }
                    for (key, argument) in [
                        ("sky_color", if desert { "#203040" } else { "#102030" }),
                        (
                            "cloud_color",
                            if desert { "#00203040" } else { "#00102030" },
                        ),
                        (
                            "water_fog_color",
                            if desert { "#203040" } else { "#010203" },
                        ),
                        ("fog_color", "#000000"),
                        ("sunrise_sunset_color", "#00000000"),
                        ("sky_light_color", "#000000"),
                        ("block_light_tint", "#000000"),
                        ("night_vision_color", "#000000"),
                        ("ambient_light_color", "#000000"),
                    ] {
                        value["attributes"][format!("minecraft:visual/{key}")] =
                            json!({"modifier":"add","argument":argument});
                    }
                    value["attributes"]["minecraft:gameplay/can_start_raid"] = json!(desert);
                    value["attributes"]["minecraft:gameplay/villager_activity"] =
                        json!(if desert {
                            "minecraft:meet"
                        } else {
                            "minecraft:work"
                        });
                    for (key, delta) in [
                        (
                            "minecraft:visual/fog_end_distance",
                            if desert { 20.0 } else { 0.0 },
                        ),
                        (
                            "minecraft:gameplay/turtle_egg_hatch_chance",
                            if desert { 0.5 } else { 0.0 },
                        ),
                        (
                            "minecraft:visual/sun_angle",
                            if desert { -10.0 } else { 0.0 },
                        ),
                    ] {
                        value["attributes"][key] = json!({"modifier":"add","argument":delta});
                    }
                }
                _ => {
                    value = json!({"clock":"minecraft:overworld","period_ticks":24000,"tracks":{
                        "minecraft:visual/default_dripstone_particle":{
                            "keyframes":[{"ticks":0,"value":dust_particle(1.0)},{"ticks":12000,"value":item_particle(2, 7)}]},
                        "minecraft:visual/ambient_particles":{
                            "keyframes":[{"ticks":0,"value":ambient_particles(0.125)},{"ticks":12000,"value":ambient_particles(0.25)}]},
                        "minecraft:visual/sky_color":{"modifier":"subtract",
                            "keyframes":[{"ticks":0,"value":"#000000"},{"ticks":12000,"value":"#102030"}]},
                        "minecraft:visual/cloud_color":{"modifier":"multiply",
                            "keyframes":[{"ticks":0,"value":"#80808080"},{"ticks":12000,"value":"#ffffffff"}]},
                        "minecraft:audio/background_music":{
                            "keyframes":[{"ticks":0,"value":background_music(20)},{"ticks":12000,"value":background_music(21)}]},
                        "minecraft:audio/ambient_sounds":{
                            "keyframes":[{"ticks":0,"value":ambient_sounds(0.25)},{"ticks":12000,"value":ambient_sounds(0.5)}]},
                        "minecraft:gameplay/bed_rule":{
                            "keyframes":[{"ticks":0,"value":bed_rule("red")},{"ticks":12000,"value":bed_rule("blue")}]},
                        "minecraft:gameplay/eyeblossom_open":{
                            "keyframes":[{"ticks":0,"value":true},{"ticks":12000,"value":false}]},
                        "minecraft:visual/moon_phase":{
                            "keyframes":[{"ticks":0,"value":"third_quarter"},{"ticks":12000,"value":"first_quarter"}]},
                        "minecraft:gameplay/baby_villager_activity":{
                            "keyframes":[{"ticks":0,"value":"minecraft:play"},{"ticks":12000,"value":"minecraft:rest"}]},
                        "minecraft:gameplay/can_start_raid":{"modifier":"and",
                            "keyframes":[{"ticks":0,"value":false},{"ticks":12000,"value":true}]},
                        "minecraft:visual/fog_end_distance":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":-20.0},{"ticks":12000,"value":-5.0}]},
                        "minecraft:gameplay/turtle_egg_hatch_chance":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":-0.5},{"ticks":12000,"value":-0.25}]},
                        "minecraft:visual/sun_angle":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":0.0},{"ticks":12000,"value":40.0}]},
                        "minecraft:gameplay/sky_light_level":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":10.0}]},
                        "minecraft:visual/sky_light_factor":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":0.5}]},
                        "minecraft:visual/star_brightness":{"modifier":"add",
                            "keyframes":[{"ticks":0,"value":0.5}]}
                    }});
                }
            }
            element.source_json = serde_json::to_vec(&value).unwrap().into();
            element.source = Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(value)).unwrap(),
                )
                .unwrap(),
            );
            element.codec_value = if registry.registry_id == "minecraft:timeline" {
                let data =
                    bbb_registry_codec::decode_direct_registry_json(&element.source_json).unwrap();
                let decoded = bbb_registry_codec::decode_exact_synchronized_registry_element(
                    &registry.registry_id,
                    &data,
                )
                .unwrap();
                Some(decoded.into_canonical().unwrap())
            } else {
                None
            };
        }
    }
    config
}

fn admin_result(tick: &crate::world::TickResult, command: &str) -> Option<i32> {
    let outcome = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => Some(outcome),
            _ => None,
        })
        .expect("actual command outcome");
    assert!(!outcome.replayed, "{command}: {outcome:?}");
    let Ok(crate::admin::AdminResult::Command(report)) = &outcome.result else {
        panic!("{command}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{command}: {report:?}");
    report.result
}

#[test]
fn scalar_environment_commands_sample_real_biomes_clock_and_weather() {
    let mut world = ServerWorld::new(scalar_config()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let before = world.publication().tick;
    let setup = crate::tests::console_command(
        &sender,
        &receiver,
        &mut world,
        before,
        "scoreboard objectives add bbb_env dummy",
    );
    assert_eq!(admin_result(&setup, "scoreboard objective"), Some(1));
    let level = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    let biomes = ["minecraft:plains", "minecraft:desert"].map(|id| {
        world
            .registries
            .registry_element_index("minecraft:worldgen/biome", id)
            .unwrap()
    });
    let chunks = &mut world.level_mut_by_id(&level).chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    for (x, biome) in biomes.into_iter().enumerate() {
        chunks
            .column_mut(0, 0)
            .unwrap()
            .set_noise_biome(x as i32, 16, 0, biome)
            .unwrap();
    }
    let run = |world: &mut ServerWorld, command: &str| {
        if command.chars().count() > crate::admin::protocol::MAX_ADMIN_COMMAND_CHARS {
            let before = world.publication().tick;
            let reset = crate::tests::console_command(
                &sender,
                &receiver,
                world,
                before,
                "scoreboard players set #env bbb_env -1",
            );
            assert!(admin_result(&reset, "scoreboard result reset").is_some());
            let command = format!("execute store result score #env bbb_env run {command}");
            let (packet_id, payload) = bbb_protocol::packets::encode_play_chat_command(
                &bbb_protocol::packets::ChatCommand {
                    command: command.as_str(),
                },
            );
            let packet =
                bbb_protocol::packets::decode_play_serverbound(packet_id, &payload).unwrap();
            let before = world.publication().tick;
            sender
                .try_send(crate::world::WorldCommand::Play {
                    connection_id: ConnectionId(1),
                    packet,
                })
                .unwrap();
            let tick = world.tick(receiver.drain());
            assert!(tick.tick > before);
            assert_eq!(world.publication().tick, tick.tick);
            let result = world.scoreboard().score("#env", "bbb_env").unwrap().value;
            assert_ne!(result, -1, "{command}: {:?}", tick.events);
            return Some(result);
        }
        let before = world.publication().tick;
        let tick = crate::tests::console_command(&sender, &receiver, world, before, command);
        assert!(tick.tick > before);
        assert_eq!(world.publication().tick, tick.tick);
        admin_result(&tick, command)
    };
    let values = |world: &ServerWorld| {
        let resolver = world.item_stack_resolver();
        let data = world.levels[ConnectionId(1)].inventory.slot(0).unwrap().component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            &resolver).unwrap().unwrap();
        let Some(NbtSummaryValue::List(values)) = crate::loot_runtime::nbt_field(&data, "floats")
        else {
            panic!("{data:?}");
        };
        values
            .iter()
            .map(|value| match value {
                NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
                _ => panic!("{value:?}"),
            })
            .collect::<Vec<_>>()
    };
    let item_count = |world: &ServerWorld, item| {
        world.publication().players[0]
            .inventory
            .slots
            .iter()
            .filter(|slot| slot.item_id == Some(item))
            .map(|slot| slot.count)
            .sum::<i32>()
    };
    let check_colors = |world: &mut ServerWorld, x: f64, expected: [u32; 9]| {
        for ((name, argb), value) in [
            ("sky_color", false),
            ("cloud_color", true),
            ("fog_color", false),
            ("sunrise_sunset_color", true),
            ("sky_light_color", false),
            ("water_fog_color", false),
            ("block_light_tint", false),
            ("night_vision_color", false),
            ("ambient_light_color", false),
        ]
        .into_iter()
        .zip(expected)
        {
            let value = if argb {
                format!("'#{value:08x}'")
            } else {
                (value as i32).to_string()
            };
            assert_eq!(run(world, &format!("execute positioned {x} 64 0.5 if predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/{name}',value:{value}}} run give op minecraft:quartz 1")), Some(1));
        }
        assert_eq!(
            item_count(world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
            9
        );
        let wrong_alpha = expected[1] ^ 0x01000000;
        assert_eq!(run(world, &format!("execute positioned {x} 64 0.5 unless predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/cloud_color',value:'#{wrong_alpha:08x}'}} run give op minecraft:quartz 1")), Some(1));
        assert_eq!(
            item_count(world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
            10
        );
        run(world, "clear op minecraft:quartz");
    };
    run(&mut world, "tick freeze");
    run(&mut world, "weather clear");
    let clock = world
        .registries
        .registry_element_index("minecraft:world_clock", "minecraft:overworld")
        .unwrap() as usize;
    for (time, x, expected, diamonds) in [
        (0, 4.5, [10.0, 0.75, 10.0], 0),
        (12000, 0.5, [5.0, 0.5, 60.0], 0),
        (12000, 4.5, [25.0, 1.0, 50.0], 1),
    ] {
        run(&mut world, &format!("time set {time}"));
        assert_eq!(
            world.world_clock_states_for_test()[clock]
                .total_ticks
                .rem_euclid(24000),
            time
        );
        assert_eq!(run(&mut world,&format!("execute positioned {x} 64 0.5 run loot replace entity op hotbar.0 loot bbb:scalar_values")),Some(1));
        assert_eq!(values(&world), expected);
        run(&mut world, "clear op");
        assert_eq!(
            run(
                &mut world,
                &format!(
                    "execute positioned {x} 64 0.5 run loot give op loot bbb:scalar_condition"
                )
            ),
            Some(diamonds)
        );
        assert_eq!(
            item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
            diamonds
        );
        let (sky, cloud, water) = match (time, x) {
            (0, _) => (0xff4070a0, 0x40304860, 0xff315273),
            (_, 0.5) => (0xff204060, 0x805080b0, 0xff122436),
            _ => (0xff305070, 0x806090c0, 0xff315273),
        };
        check_colors(
            &mut world,
            x,
            [
                sky, cloud, 0x0080a0c0, 0x604080c0, 0xffc0d0e0, water, 0xff445566, 0xff778899,
                0xff102030,
            ],
        );
        for (attribute, expected, wrong) in [
            (
                "default_dripstone_particle",
                if time == 0 {
                    dust_particle(1.0)
                } else {
                    item_particle(2, 7)
                },
                if time == 0 {
                    dust_particle(0.75)
                } else {
                    item_particle(1, 7)
                },
            ),
            (
                "ambient_particles",
                ambient_particles(if time == 0 { 0.125 } else { 0.25 }),
                ambient_particles(0.75),
            ),
        ] {
            for (branch, value) in [("if", expected), ("unless", wrong)] {
                assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 {branch} predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/{attribute}',value:{value}}} run give op minecraft:quartz 1")), Some(1));
            }
        }
        let mut reversed = ambient_particles(if time == 0 { 0.125 } else { 0.25 });
        reversed.as_array_mut().unwrap().reverse();
        assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 unless predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/ambient_particles',value:{reversed}}} run give op minecraft:quartz 1")), Some(1));
        let branch = if time == 0 { "unless" } else { "if" };
        assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 {branch} predicate bbb:particles run give op minecraft:quartz 1")), Some(1));
        assert_eq!(
            item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
            6
        );
        run(&mut world, "clear op minecraft:quartz");
        for table in [
            "bbb:particles".to_owned(),
            json!({
                "type":"minecraft:chest","pools":[{"rolls":1,"conditions":[particle_condition()],
                    "entries":[{"type":"minecraft:item","name":"minecraft:prismarine_crystals"}]}]
            })
            .to_string(),
        ] {
            let count = i32::from(time != 0);
            assert_eq!(
                run(
                    &mut world,
                    &format!("execute positioned {x} 64 0.5 run loot give op loot {table}")
                ),
                Some(count)
            );
            assert_eq!(
                item_count(
                    &world,
                    bbb_protocol::item_types::VANILLA_ITEM_PRISMARINE_CRYSTALS_ID
                ),
                count
            );
            if count != 0 {
                run(&mut world, "clear op minecraft:prismarine_crystals");
            }
        }
        if time != 0 {
            for modifier in [
                "bbb:particles".to_owned(),
                json!({
                    "function":"minecraft:set_count","count":7,"conditions":[particle_condition()]
                })
                .to_string(),
            ] {
                run(
                    &mut world,
                    "item replace entity op hotbar.8 with minecraft:prismarine_crystals 1",
                );
                assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 run item modify entity op hotbar.8 {modifier}")), Some(1));
                assert_eq!(
                    item_count(
                        &world,
                        bbb_protocol::item_types::VANILLA_ITEM_PRISMARINE_CRYSTALS_ID
                    ),
                    7
                );
                run(&mut world, "clear op minecraft:prismarine_crystals");
            }
            for mut wrong in [item_particle(2, 8), item_particle(2, 7)] {
                if wrong["item"]["components"]["minecraft:custom_data"]["bbb"] == json!("7") {
                    wrong["item"]["components"]["minecraft:instrument"] =
                        json!("minecraft:sing_goat_horn");
                }
                assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 unless predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/default_dripstone_particle',value:{wrong}}} run give op minecraft:quartz 1")), Some(1));
            }
            let original = item_particle(2, 7).to_string();
            let reordered =
                original.replace("{\"aaa\":\"first\",\"bbb\":\"7\"}", "{bbb:'7',aaa:first}");
            assert_ne!(original, reordered);
            assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 if predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/default_dripstone_particle',value:{reordered}}} run give op minecraft:quartz 1")), Some(1));
            assert_eq!(
                item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
                3
            );
            run(&mut world, "clear op minecraft:quartz");
        }
        assert_eq!(
            run(
                &mut world,
                &format!("execute positioned {x} 64 0.5 run loot give op loot bbb:color")
            ),
            Some(diamonds)
        );
        assert_eq!(
            item_count(
                &world,
                bbb_protocol::item_types::VANILLA_ITEM_AMETHYST_SHARD_ID
            ),
            diamonds
        );
        let branch = if diamonds == 1 { "if" } else { "unless" };
        for predicate in [
            "bbb:scalar_ready",
            "{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/fog_end_distance',value:25.0f}",
        ] {
            assert_eq!(run(&mut world, &format!("execute positioned {x} 64 0.5 {branch} predicate {predicate} run give op minecraft:diamond 1")), Some(1));
        }
        assert_eq!(
            item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
            diamonds + 2
        );
        run(&mut world, "clear op");
        let mut emeralds = 0;
        for (age, matches) in [("adult", x == 0.5), ("baby", time == 0)] {
            let count = i32::from(matches);
            assert_eq!(
                run(
                    &mut world,
                    &format!(
                        "execute positioned {x} 64 0.5 run loot give op loot bbb:{age}_activity"
                    )
                ),
                Some(count)
            );
            emeralds += count;
            assert_eq!(
                item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID),
                emeralds
            );
        }
        let gold = i32::from(time == 0);
        assert_eq!(
            run(
                &mut world,
                &format!("execute positioned {x} 64 0.5 run loot give op loot bbb:third_quarter")
            ),
            Some(gold)
        );
        assert_eq!(
            item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_GOLD_INGOT_ID),
            gold
        );
        let mut iron = 0;
        for (name, matches) in [
            ("open", time == 0),
            ("closed", time != 0),
            ("default", false),
        ] {
            let count = i32::from(matches);
            assert_eq!(
                run(
                    &mut world,
                    &format!(
                        "execute positioned {x} 64 0.5 run loot give op loot bbb:eyeblossom_{name}"
                    )
                ),
                Some(count)
            );
            iron += count;
            assert_eq!(
                item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID),
                iron
            );
        }
        for (name, item) in [
            (
                "background_music",
                bbb_protocol::item_types::VANILLA_ITEM_REDSTONE_ID,
            ),
            (
                "ambient_sounds",
                bbb_protocol::item_types::VANILLA_ITEM_LAPIS_LAZULI_ID,
            ),
        ] {
            let count = i32::from(time == 0);
            assert_eq!(
                run(
                    &mut world,
                    &format!("execute positioned {x} 64 0.5 run loot give op loot bbb:{name}")
                ),
                Some(count)
            );
            assert_eq!(item_count(&world, item), count);
        }
        let mut copper = 0;
        for (name, matches) in [("red_bed", time == 0), ("exploding_bed", false)] {
            let count = i32::from(matches);
            assert_eq!(
                run(
                    &mut world,
                    &format!("execute positioned {x} 64 0.5 run loot give op loot bbb:{name}")
                ),
                Some(count)
            );
            copper += count;
            assert_eq!(
                item_count(
                    &world,
                    bbb_protocol::item_types::VANILLA_ITEM_COPPER_INGOT_ID
                ),
                copper
            );
        }
    }
    assert_eq!(
        run(
            &mut world,
            "loot replace entity op hotbar.0 loot bbb:nonpositional_scalar"
        ),
        Some(1)
    );
    assert_eq!(values(&world), [15.0]);
    run(&mut world, "gamerule advance_weather false");
    for (weather, wet, expected, darken) in [
        ("clear", false, [15.0, 1.0, 1.0], 0),
        ("rain", true, [15.0, 0.934375, 0.0], 0),
        ("thunder", true, [13.925781, 0.7173828, 0.0], 1),
    ] {
        run(&mut world, &format!("weather {weather}"));
        if wet {
            run(&mut world, "tick unfreeze");
            for _ in 0..140 {
                world.tick(receiver.drain());
            }
            run(&mut world, "tick freeze");
        }
        let actual_weather = &world.level_by_id(&level).weather;
        assert!(actual_weather.can_have_weather());
        assert_eq!(actual_weather.rain_level(), if wet { 1.0 } else { 0.0 });
        assert_eq!(
            actual_weather.thunder_level(),
            if weather == "thunder" { 1.0 } else { 0.0 }
        );
        run(&mut world, "time set 12000");
        let colors = match weather {
            "rain" => [0xff212931, 0x80364e66, 0x003f4f73, 0x601f3f73, 0xffaab5e9],
            "thunder" => [0xff0e1011, 0x800f1214, 0x001f2739, 0x600f1f39, 0xff9ba2f0],
            _ => [0xff204060, 0x805080b0, 0x0080a0c0, 0x604080c0, 0xffc0d0e0],
        };
        check_colors(
            &mut world,
            0.5,
            [
                colors[0], colors[1], colors[2], colors[3], colors[4], 0xff122436, 0xff445566,
                0xff778899, 0xff102030,
            ],
        );
        assert_eq!(
            run(
                &mut world,
                "loot replace entity op hotbar.0 loot bbb:weather_values"
            ),
            Some(1)
        );
        let actual_values = values(&world);
        assert_eq!(actual_values.len(), expected.len());
        for (actual, expected) in actual_values.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 0.00001,
                "{weather}: {actual} != {expected}"
            );
        }
        assert_eq!(world.level_by_id(&level).sky_darkening, Some(darken));
        run(&mut world, "clear op");
        assert_eq!(run(&mut world, &format!("execute if predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:gameplay/sky_light_level',value:{:.8}f}} run give op minecraft:diamond 1", expected[0])), Some(1));
        assert_eq!(
            item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
            1
        );
        assert_eq!(
            run(&mut world, "loot give op loot bbb:weather_bees"),
            Some(i32::from(wet))
        );
    }
    let dry = world.require_level_id(&Java26BuiltinDimension::Nether.into());
    assert!(!world.level_by_id(&dry).weather.can_have_weather());
    let plains = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .unwrap();
    let chunks = &mut world.level_mut_by_id(&dry).chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    chunks
        .column_mut(0, 0)
        .unwrap()
        .set_noise_biome(0, 16, 0, plains)
        .unwrap();
    let desert = world
        .registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:desert")
        .unwrap();
    world
        .level_mut_by_id(&dry)
        .chunks
        .column_mut(0, 0)
        .unwrap()
        .set_noise_biome(1, 16, 0, desert)
        .unwrap();
    for (x, particle, ambient) in [
        (
            0.5,
            json!({"type":"minecraft:dripping_dripstone_lava"}),
            json!([]),
        ),
        (4.5, item_particle(4, 8), ambient_particles(0.75)),
    ] {
        for (attribute, value) in [
            ("default_dripstone_particle", particle),
            ("ambient_particles", ambient),
        ] {
            assert_eq!(run(&mut world, &format!("execute in minecraft:the_nether positioned {x} 64 0.5 if predicate {{condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/{attribute}',value:{value}}} run give op minecraft:quartz 1")), Some(1));
        }
    }
    assert_eq!(
        item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
        4
    );
    run(&mut world, "clear op minecraft:quartz");
    assert_eq!(run(&mut world,"execute in minecraft:the_nether positioned 0.5 64 0.5 run loot replace entity op hotbar.0 loot bbb:weather_values"),Some(1));
    assert_eq!(values(&world), [4.0, 0.0, 0.0]);
    assert_eq!(run(&mut world, "execute in minecraft:the_nether positioned 0.5 64 0.5 if predicate {condition:'minecraft:environment_attribute_check',attribute:'minecraft:visual/fog_color',value:0} run give op minecraft:quartz 1"), Some(1));
    assert_eq!(
        item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_QUARTZ_ID),
        1
    );
    run(&mut world, "clear op");
    assert_eq!(run(&mut world,"execute in minecraft:the_nether if predicate {condition:'minecraft:environment_attribute_check',attribute:'minecraft:gameplay/sky_light_level',value:4.0f} run give op minecraft:diamond 1"),Some(1));
    assert_eq!(
        item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
        1
    );
    assert_eq!(
        run(
            &mut world,
            "execute in minecraft:the_nether positioned 0.5 64 0.5 run loot give op loot bbb:weather_bees"
        ),
        Some(0)
    );
    assert_eq!(run(&mut world, "execute in minecraft:the_nether positioned 0.5 64 0.5 run loot give op loot bbb:eyeblossom_default"), Some(1));
    assert_eq!(
        item_count(&world, bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID),
        1
    );
    let block_state = |name| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &std::collections::BTreeMap::new(),
        )
        .unwrap()
    };
    for (x, name) in [
        "minecraft:closed_eyeblossom",
        "minecraft:open_eyeblossom",
        "minecraft:potted_closed_eyeblossom",
        "minecraft:potted_open_eyeblossom",
    ]
    .into_iter()
    .enumerate()
    {
        let position = BlockPos {
            x: x as i32,
            y: 64,
            z: 0,
        };
        let state = block_state(name);
        let dimension = Java26BuiltinDimension::Nether;
        world.set_block(
            dimension,
            BlockPos { y: 63, ..position },
            block_state("minecraft:dirt"),
            2,
        );
        world.set_block(dimension, position, state, 2);
        assert_eq!(world.block_state(dimension, position), Some(state));
        world.with_level_for_test(dimension, |level| {
            crate::block_behavior::random_tick_block(level, position, state);
        });
        assert_eq!(world.block_state(dimension, position), Some(state));
    }
}
