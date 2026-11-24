use super::*;
use bbb_protocol::packets::ChunkPos;

fn loaded_world() -> CommandTestWorld {
    CommandTestWorld::default().with_loaded_chunk(ChunkPos { x: 0, z: 0 })
}

fn stone() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap()
}

#[test]
fn execute_items_counts_stacks_across_entity_slots_and_filters_sources() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut fixture = world(&players);
    for command in [
        "item replace entity alice hotbar.0 with diamond 3",
        "item replace entity bob hotbar.0 with diamond 5",
        "item replace entity bob hotbar.1 with diamond 7",
    ] {
        assert_eq!(ran(command, &mut fixture).result, Some(1));
    }
    let counted = ran("execute if items entity @a hotbar.* diamond", &mut fixture);
    assert_eq!(counted.result, Some(15));
    assert_eq!(keys(&counted), ["commands.execute.conditional.pass_count"]);
    assert_eq!(arguments(&counted, 0), [NbtSummaryValue::Int(15)]);
    let unless = ran(
        "execute unless items entity @a hotbar.* diamond",
        &mut fixture,
    );
    assert_eq!(keys(&unless), ["commands.execute.conditional.fail_count"]);
    assert_eq!(arguments(&unless, 0), [NbtSummaryValue::Int(15)]);
    let absent = ran(
        "execute unless items entity @a hotbar.* emerald",
        &mut fixture,
    );
    assert_eq!(absent.result, Some(1));
    assert_eq!(keys(&absent), ["commands.execute.conditional.pass"]);
    let filtered = ran(
        "execute as @a if items entity @s hotbar.* diamond[count=3] run give @s dirt",
        &mut fixture,
    );
    assert_eq!(filtered.result, Some(1));
    assert_eq!(
        fixture.item_slot("alice", 1).unwrap().item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID)
    );
    assert_eq!(fixture.item_slot("bob", 1).unwrap().count(), 7);
    let missing = ran(
        "execute if items entity @e[type=pig] contents *",
        &mut fixture,
    );
    assert_eq!(keys(&missing), ["argument.entity.notfound.entity"]);
}

#[test]
fn execute_items_reads_full_components_defaults_and_boolean_terms_from_blocks() {
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut fixture = loaded_world().with_chest(pos);
    assert_eq!(
        ran(
            "item replace block 1 64 1 container.0 with diamond[custom_data={marker:{value:2}}] 7",
            &mut fixture
        )
        .result,
        Some(1)
    );
    let full = ran("execute if items block 1 64 1 container.* diamond[max_stack_size=64,custom_data={marker:{value:2}},!damage|count=1,count~{min:7,max:7}]", &mut fixture);
    assert_eq!(full.result, Some(7));
    let presence = ran(
        "execute if items block 1 64 1 container.* *[max_stack_size,custom_data,custom_data~{marker:{}}]",
        &mut fixture,
    );
    assert_eq!(presence.result, Some(7));
    let alternative = ran(
        "execute if items block 1 64 1 container.* *[!custom_data|max_stack_size=16]",
        &mut fixture,
    );
    assert_eq!(keys(&alternative), ["commands.execute.conditional.fail"]);
    let invalid = ran(
        "execute unless items block 1 64 1 container.53 *",
        &mut fixture,
    );
    assert_eq!(invalid.result, Some(1));
    let not_container = ran("execute if items block 0 64 0 container.* *", &mut fixture);
    assert_eq!(
        keys(&not_container),
        ["commands.item.source.not_a_container"]
    );
    assert_eq!(
        ran(
            "item replace block 1 64 1 container.1 with iron_sword[damage=2]",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran(
            "execute if items block 1 64 1 container.* *[damage~{damage:2}]",
            &mut fixture
        )
        .result,
        Some(1)
    );
    for (item, contents, predicate) in [
        (
            "stone",
            r#"attribute_modifiers=[{type:"minecraft:attack_damage",id:"test:power",amount:2.5d,operation:"add_value",slot:"mainhand"}]"#,
            r#"attribute_modifiers~{modifiers:{size:1,contains:[{attribute:"attack_damage",id:"test:power",amount:{min:2.0},operation:"add_value",slot:"mainhand"}]}}"#,
        ),
        (
            "diamond_chestplate",
            r#"trim={material:"minecraft:gold",pattern:"minecraft:sentry"}"#,
            r#"trim~{material:"gold",pattern:["sentry"]}"#,
        ),
        (
            "stone",
            r#"jukebox_playable="minecraft:cat""#,
            r#"jukebox_playable~{song:"cat"}"#,
        ),
        (
            "stone",
            r#"minecraft:villager/variant="minecraft:desert""#,
            r#"minecraft:villager/variant~["plains","desert"]"#,
        ),
        (
            "firework_star",
            r#"firework_explosion={shape:"star",has_trail:true}"#,
            r#"firework_explosion~{shape:"star",has_trail:true,has_twinkle:false}"#,
        ),
        (
            "firework_rocket",
            r#"fireworks={flight_duration:2,explosions:[{shape:"burst",has_twinkle:true},{shape:"star"}]}"#,
            r#"fireworks~{flight_duration:{min:1,max:2},explosions:{size:2,contains:[{shape:"burst",has_twinkle:true}],count:[{test:{has_trail:false},count:2}]}}"#,
        ),
        (
            "firework_rocket",
            "fireworks={}",
            "fireworks~{flight_duration:0,explosions:{size:0}}",
        ),
        (
            "writable_book",
            r#"writable_book_content={pages:[{raw:"draft",filtered:"safe"}]}"#,
            r#"writable_book_content~{pages:{contains:["draft"],size:1}}"#,
        ),
        (
            "shulker_box",
            r#"container=[{slot:2,item:{id:"minecraft:dirt",count:7}},{slot:26,item:{id:"minecraft:diamond",count:3}}]"#,
            r#"container~{items:{size:2,contains:[{items:"minecraft:dirt",count:7}],count:[{test:{items:"minecraft:diamond"},count:1}]}}"#,
        ),
        (
            "bundle",
            r#"bundle_contents=[{id:"minecraft:stone",count:4,components:{"minecraft:custom_data":{marker:2}}}]"#,
            r#"bundle_contents~{items:{size:1,contains:[{count:4,predicates:{"minecraft:custom_data":{marker:2}}}]}}"#,
        ),
        (
            "written_book",
            r#"written_book_content={title:"Title",author:"Writer",pages:[{text:"hello",bold:true}]}"#,
            r#"written_book_content~{author:"Writer",title:"Title",pages:{contains:[{text:"hello",bold:true}],size:1}}"#,
        ),
    ] {
        assert_eq!(
            ran(
                &format!("item replace block 1 64 1 container.2 with {item}[{contents}]"),
                &mut fixture,
            )
            .result,
            Some(1)
        );
        assert_eq!(
            ran(
                &format!("execute if items block 1 64 1 container.* *[{predicate}]"),
                &mut fixture,
            )
            .result,
            Some(1)
        );
    }
}

#[test]
fn execute_items_uses_published_tags_and_reports_unknown_arguments() {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    let tags = startup
        .tags
        .registries
        .iter_mut()
        .find(|tags| tags.registry_id == "minecraft:item")
        .unwrap();
    tags.tags
        .insert("example:treasure".into(), vec!["minecraft:diamond".into()]);
    tags.tags
        .insert("minecraft:planks".into(), vec!["minecraft:diamond".into()]);
    let mut fixture = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            datapacks: Some(std::sync::Arc::new(startup)),
            ..Default::default()
        },
        &[],
    )
    .with_chest(BlockPos { x: 1, y: 64, z: 1 });
    assert_eq!(
        ran(
            "item replace block 1 64 1 container.0 with diamond 5",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran(
            "item replace block 1 64 1 container.1 with oak_planks 2",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran(
            "execute if items block 1 64 1 container.* #example:treasure",
            &mut fixture
        )
        .result,
        Some(5)
    );
    assert_eq!(
        ran(
            "execute if items block 1 64 1 container.* #minecraft:planks",
            &mut fixture
        )
        .result,
        Some(5)
    );
    for (predicate, key) in [
        ("#example:missing", "arguments.item.tag.unknown"),
        ("example:missing", "argument.item.id.invalid"),
        ("*[example:missing]", "arguments.item.component.unknown"),
    ] {
        let run = ran(
            &format!("execute if items block 1 64 1 container.* {predicate}"),
            &mut fixture,
        );
        assert_eq!(keys(&run), [key, CURSOR_CONTEXT]);
    }
}

#[test]
fn execute_items_unpacks_lazy_container_loot_through_the_item_reader() {
    let config = crate::tests::datapack_config_with_loot_table(
        "minecraft:chests/igloo_chest",
        &serde_json::json!({
            "type":"minecraft:chest", "pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}]
        }),
    );
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let mut fixture = CommandTestWorld::new(config, &[]).with_chest(pos);
    assert_eq!(
        ran(
            "data merge block 1 64 1 {LootTable:'minecraft:chests/igloo_chest',LootTableSeed:7L}",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran("execute if data block 1 64 1 LootTable", &mut fixture).result,
        Some(1)
    );
    assert_eq!(
        ran(
            "execute if items block 1 64 1 container.* diamond",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran("execute unless data block 1 64 1 LootTable", &mut fixture).result,
        Some(1)
    );
    let count: i32 = (0..27)
        .map(|slot| fixture.container_item(pos, slot).unwrap().count())
        .sum();
    assert_eq!(count, 1);
}

#[test]
fn execute_in_scales_coordinates_and_writes_the_named_level() {
    let mut world = loaded_world()
        .with_position([8.0, 64.0, 16.0])
        .with_execute_chunk(Java26BuiltinDimension::Nether, (0, 0));
    let run = ran(
        "execute in the_nether run setblock ~ ~ ~ minecraft:stone",
        &mut world,
    );
    assert_eq!(keys(&run), ["commands.setblock.success"]);
    assert_eq!(
        world.execute_block(
            Java26BuiltinDimension::Nether,
            BlockPos { x: 1, y: 64, z: 2 }
        ),
        Some(stone())
    );
    assert_ne!(
        world.execute_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 1, y: 64, z: 2 }
        ),
        Some(stone())
    );
}

#[test]
fn execute_positioned_and_align_change_the_actual_write_position() {
    let mut world = loaded_world().with_position([8.5, 64.0, 8.25]);
    ran(
        "execute positioned ~5 ~ ~-1 run setblock ~ ~ ~ minecraft:stone",
        &mut world,
    );
    assert_eq!(
        world.execute_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 13, y: 64, z: 7 }
        ),
        Some(stone())
    );
    ran(
        "execute align x run setblock ~0.75 ~ ~ minecraft:stone",
        &mut world,
    );
    assert_eq!(
        world.execute_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 8, y: 64, z: 8 }
        ),
        Some(stone())
    );
}

#[test]
fn execute_positioned_over_reads_the_live_heightmap() {
    let mut world = loaded_world().with_position([8.5, 10.0, 8.25]);
    world.setup("setblock 8 70 8 minecraft:stone");
    ran(
        "execute positioned over world_surface run setblock ~ ~ ~ minecraft:stone",
        &mut world,
    );
    assert_eq!(
        world.execute_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 8, y: 71, z: 8 }
        ),
        Some(stone())
    );
}

#[test]
fn execute_rotation_and_facing_change_local_coordinates() {
    for command in [
        "execute rotated -90 0 run setblock ^ ^ ^1 minecraft:stone",
        "execute facing 1.0 64.0 0.0 run setblock ^ ^ ^1 minecraft:stone",
    ] {
        let mut world = loaded_world().with_position([0.0, 64.0, 0.0]);
        ran(command, &mut world);
        assert_eq!(
            world.execute_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x: 1, y: 64, z: 0 }
            ),
            Some(stone())
        );
    }
}

#[test]
fn execute_anchor_and_facing_entity_use_real_player_eyes() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut world = world(&players)
        .with_position([0.0, 64.0, 0.0])
        .with_player_position("alice", [0.0, 64.0, 0.0])
        .with_player_position("bob", [1.0, 64.0, 0.0]);
    ran(
        "execute facing entity bob eyes run teleport alice ~ ~ ~ ~ ~",
        &mut world,
    );
    let (_, rotation) = world.execute_player_pose("alice");
    assert!((rotation[0] + 90.0).abs() < 0.01);
    assert!(rotation[1] < -45.0);
    ran(
        "execute as alice at @s anchored eyes positioned ^ ^ ^ run teleport @s ~ ~ ~",
        &mut world,
    );
    assert!(world.execute_player_pose("alice").0[1] > 65.0);
}

#[test]
fn execute_as_at_runs_against_real_players() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut world = world(&players)
        .with_loaded_chunk(ChunkPos { x: 0, z: 0 })
        .with_player_position("alice", [2.0, 64.0, 2.0])
        .with_player_position("bob", [6.0, 64.0, 6.0]);
    ran(
        "execute as @a at @s run setblock ~ ~ ~ minecraft:stone",
        &mut world,
    );
    for coordinate in [2, 6] {
        assert_eq!(
            world.execute_block(
                Java26BuiltinDimension::Overworld,
                BlockPos {
                    x: coordinate,
                    y: 64,
                    z: coordinate
                }
            ),
            Some(stone())
        );
    }
    assert_eq!(
        ran("execute positioned as nobody run seed", &mut world).result,
        None
    );
    assert_eq!(
        ran_as(
            "execute as @s run seed",
            &CommandSource::server("console", CommandPermission::Owner),
            &mut world,
        )
        .result,
        None,
    );
}

#[test]
fn execute_modifiers_keep_their_argument_and_permission_errors() {
    let mut world = loaded_world();
    for (command, expected) in [
        (
            "execute in minecraft:the_aether run seed",
            vec!["argument.dimension.invalid"],
        ),
        (
            "execute in foo:bar:baz run seed",
            vec!["argument.id.invalid", CURSOR_CONTEXT],
        ),
        (
            "execute align xx run seed",
            vec!["arguments.swizzle.invalid", CURSOR_CONTEXT],
        ),
        (
            "execute positioned over world_surface_wg run seed",
            vec!["argument.enum.invalid", CURSOR_CONTEXT],
        ),
    ] {
        assert_eq!(keys(&ran(command, &mut world)), expected, "{command}");
    }
    assert_eq!(
        keys(&refused("execute run seed", CommandPermission::Moderator)),
        ["command.unknown.command", CURSOR_CONTEXT]
    );
}

#[test]
fn execute_dimension_suggestions_list_the_served_levels() {
    let world = CommandTestWorld::default();
    assert_eq!(
        texts(&completions_for(
            "execute in ",
            CommandPermission::Owner,
            &world
        )),
        [
            "minecraft:overworld",
            "minecraft:the_end",
            "minecraft:the_nether"
        ]
    );
}

#[test]
fn execute_block_and_dimension_conditions_keep_feedback_and_run_results() {
    let mut world = loaded_world();
    world.setup("setblock 1 64 1 minecraft:stone");
    for (command, expected) in [
        (
            "execute if block 1 64 1 minecraft:stone",
            "commands.execute.conditional.pass",
        ),
        (
            "execute unless block 1 64 1 minecraft:stone",
            "commands.execute.conditional.fail",
        ),
        (
            "execute if dimension minecraft:overworld",
            "commands.execute.conditional.pass",
        ),
        (
            "execute if dimension minecraft:the_nether",
            "commands.execute.conditional.fail",
        ),
    ] {
        assert_eq!(keys(&ran(command, &mut world)), [expected], "{command}");
    }
    assert_eq!(
        keys(&ran(
            "execute if block 1 64 1 minecraft:stone run seed",
            &mut world
        )),
        ["commands.seed.success"]
    );
    assert!(keys(&ran(
        "execute if block 1 64 1 minecraft:air run seed",
        &mut world
    ))
    .is_empty());
    assert_eq!(
        keys(&ran(
            "execute if block 100000 64 100000 minecraft:stone",
            &mut world
        )),
        ["argument.pos.unloaded"]
    );
}

#[test]
fn execute_entity_condition_counts_actual_players() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut world = world(&players);
    let run = ran("execute if entity @a", &mut world);
    assert_eq!(keys(&run), ["commands.execute.conditional.pass_count"]);
    assert_eq!(arguments(&run, 0), [NbtSummaryValue::Int(2)]);
    assert_eq!(
        keys(&ran("execute unless entity @a", &mut world)),
        ["commands.execute.conditional.fail_count"]
    );
}

#[test]
fn execute_score_conditions_read_the_live_scoreboard() {
    let mut world = CommandTestWorld::default();
    world.setup("scoreboard objectives add test dummy");
    world.setup("scoreboard players set #a test 7");
    world.setup("scoreboard players set #b test 3");
    for (condition, expected) in [
        ("#a test > #b test", "commands.execute.conditional.pass"),
        ("#a test < #b test", "commands.execute.conditional.fail"),
        ("#a test matches 5..9", "commands.execute.conditional.pass"),
        (
            "#missing test matches 1..",
            "commands.execute.conditional.fail",
        ),
    ] {
        assert_eq!(
            keys(&ran(&format!("execute if score {condition}"), &mut world)),
            [expected]
        );
    }
    assert_eq!(
        keys(&ran("execute if score #a missing matches 1", &mut world)),
        ["arguments.objective.notFound"]
    );
    assert_eq!(
        keys(&ran(
            "execute if score * test matches 1",
            CommandTestWorld::default().with_scoreboard_setup_for_execute()
        )),
        ["argument.scoreHolder.empty"]
    );
}

#[test]
fn execute_nbt_conditions_read_actual_storage_and_block_entities() {
    let mut world = loaded_world();
    world.setup("data merge storage demo:state {values:[1,2]}");
    let counted = ran("execute if data storage demo:state values[]", &mut world);
    assert_eq!(keys(&counted), ["commands.execute.conditional.pass_count"]);
    assert_eq!(arguments(&counted, 0), [NbtSummaryValue::Int(2)]);
    assert_eq!(
        keys(&ran(
            "execute if data storage demo:state absent",
            &mut world
        )),
        ["commands.execute.conditional.fail"]
    );
    world.setup("setblock 2 64 2 minecraft:chest");
    assert_eq!(
        keys(&ran("execute if data block 2 64 2 Items", &mut world)),
        ["commands.execute.conditional.pass_count"]
    );
    assert_eq!(
        keys(&ran("execute if data block 3 64 3 Items", &mut world)),
        ["commands.data.block.invalid"]
    );
    let suggestions = completions_for(
        "execute if data storage demo:",
        CommandPermission::Owner,
        &world,
    );
    assert!(texts(&suggestions).iter().any(|text| text == "demo:state"));
}

#[test]
fn execute_blocks_compares_real_regions_and_skips_masked_air() {
    let mut world = loaded_world();
    world.setup("setblock 1 64 1 minecraft:stone");
    world.setup("setblock 5 64 5 minecraft:stone");
    let matched = ran("execute if blocks 1 64 1 2 64 2 5 64 5 all", &mut world);
    assert_eq!(keys(&matched), ["commands.execute.conditional.pass_count"]);
    assert_eq!(arguments(&matched, 0), [NbtSummaryValue::Int(4)]);
    world.setup("setblock 6 64 6 minecraft:stone");
    assert_eq!(
        keys(&ran(
            "execute if blocks 1 64 1 2 64 2 5 64 5 all",
            &mut world
        )),
        ["commands.execute.conditional.fail"]
    );
    let masked = ran("execute if blocks 1 64 1 2 64 2 5 64 5 masked", &mut world);
    assert_eq!(arguments(&masked, 0), [NbtSummaryValue::Int(1)]);
    world = world.with_loaded_chunk(ChunkPos { x: 2, z: 2 });
    assert_eq!(
        keys(&ran(
            "execute if blocks 0 64 0 32 96 32 0 64 0 all",
            &mut world
        )),
        ["commands.execute.blocks.toobig"]
    );
    assert_eq!(
        keys(&ran(
            "execute if blocks 0 64 0 1 64 1 100000 64 100000 all",
            &mut world
        )),
        ["argument.pos.unloaded"]
    );
}

#[test]
fn execute_blocks_compares_block_entity_payloads_without_positions() {
    let mut world = loaded_world();
    world.setup("setblock 1 64 1 minecraft:chest");
    world.setup("setblock 5 64 5 minecraft:chest");
    assert_eq!(
        keys(&ran(
            "execute if blocks 1 64 1 1 64 1 5 64 5 all",
            &mut world
        )),
        ["commands.execute.conditional.pass_count"]
    );
    world.setup("data merge block 5 64 5 {CustomName:\"different\"}");
    assert_eq!(
        keys(&ran(
            "execute if blocks 1 64 1 1 64 1 5 64 5 all",
            &mut world
        )),
        ["commands.execute.conditional.fail"]
    );
}

#[test]
fn execute_biome_conditions_read_the_actual_column() {
    let mut world =
        CommandTestWorld::with_flat_data(&[]).with_loaded_chunk(ChunkPos { x: 0, z: 0 });
    assert_eq!(
        keys(&ran("execute if biome 1 64 1 minecraft:plains", &mut world)),
        ["commands.execute.conditional.pass"]
    );
    assert_eq!(
        keys(&ran(
            "execute if biome 1 64 1 minecraft:missing",
            &mut world
        )),
        ["argument.resource.not_found", CURSOR_CONTEXT]
    );
    assert_eq!(
        keys(&ran(
            "execute if biome 100000 64 100000 minecraft:plains",
            &mut world
        )),
        ["argument.pos.unloaded"]
    );
}

#[test]
fn execute_loot_predicate_resolves_names_inline_values_and_boolean_results() {
    use bbb_server_datapack::loot::{
        parse_loot_condition, LoadedLootRegistry, LootElement, LootElementValue, LootRegistries,
    };
    let loot = LootRegistries {
        registries: vec![LoadedLootRegistry {
            registry_id: "minecraft:predicate".into(),
            elements: vec![LootElement {
                id: "test:always".into(),
                source_pack: "test".into(),
                value: LootElementValue::Predicate(Box::new(
                    parse_loot_condition(
                        &serde_json::json!({
                            "condition":"minecraft:random_chance", "chance":1.0
                        }),
                        None,
                    )
                    .unwrap(),
                )),
            }],
            tags: None,
        }],
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let mut fixture = world(&[user("alice", 1)]).with_loot(&loot);
    let console = CommandSource::server("console", CommandPermission::Owner);
    for (command, expected, key) in [
        ("execute if predicate test:always", Some(1), "commands.execute.conditional.pass"),
        ("execute unless predicate test:always", None, "commands.execute.conditional.fail"),
        ("execute if predicate [{condition:'minecraft:reference',name:'test:always'},{condition:'minecraft:random_chance',chance:1.0f}]", Some(1), "commands.execute.conditional.pass"),
        ("execute unless predicate {condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player'}}", Some(1), "commands.execute.conditional.pass"),
    ] {
        let run = ran_as(command, &console, &mut fixture);
        assert_eq!(run.result, expected, "{command}: {:?}", keys(&run));
        assert_eq!(keys(&run), [key], "{command}");
    }
    assert_eq!(
        ran(
            "execute if predicate test:always run give alice diamond 3",
            &mut fixture
        )
        .result,
        Some(1)
    );
    let filtered = ran(
        "execute unless predicate test:always run give alice dirt",
        &mut fixture,
    );
    assert!(filtered.feedback.is_empty());
    assert_eq!(fixture.item_slot("alice", 0).unwrap().count(), 3);
    assert!(fixture.item_slot("alice", 1).unwrap().is_empty());
    let missing = ran("execute if predicate test:missing", &mut fixture);
    assert_eq!(
        keys(&missing),
        ["argument.resource_or_id.no_such_element", CURSOR_CONTEXT]
    );
    assert_eq!(
        arguments(&missing, 0),
        [
            NbtSummaryValue::String("test:missing".into()),
            NbtSummaryValue::String("minecraft:predicate".into()),
        ]
    );
}

#[test]
fn execute_loot_predicate_keeps_source_entity_origin_and_level_independent() {
    let players = [user("alice", 1), user("bob", 2)];
    let nether = level_key(Java26BuiltinDimension::Nether);
    let pos = BlockPos { x: 1, y: 64, z: 1 };
    let dirt = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
    )
    .unwrap();
    let stand = stored_entity(
        10,
        100,
        [5.0, 64.0, 1.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    let mut fixture = world(&players)
        .with_player_position("alice", [1.0, 64.0, 1.0])
        .with_player_position("bob", [5.0, 64.0, 1.0])
        .with_entities(&[stand])
        .with_block(pos, stone())
        .with_level(nether)
        .with_block(pos, dirt)
        .with_level(level_key(Java26BuiltinDimension::Overworld));
    for command in [
        "execute as bob positioned 1.0 64.0 1.0 if predicate {condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player',distance:{absolute:4}}}",
        "execute as @e[type=armor_stand] positioned 1.0 64.0 1.0 if predicate {condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:armor_stand',distance:{absolute:4}}}",
        "execute in minecraft:the_nether positioned 1.0 64.0 1.0 as alice if predicate [{condition:'minecraft:location_check',predicate:{position:{x:1,y:64,z:1},block:{blocks:'minecraft:dirt'}}},{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:player',location:{block:{blocks:'minecraft:dirt'}}}}]",
        "execute positioned 1.0 64.0 1.0 unless predicate {condition:'minecraft:location_check',predicate:{block:{blocks:'minecraft:dirt'}}}",
    ] {
        let run = ran(command, &mut fixture);
        assert_eq!(run.result, Some(1), "{command}: {:?}", keys(&run));
    }
}

#[test]
fn execute_loot_predicate_reads_the_execution_levels_weather() {
    let mut fixture =
        CommandTestWorld::default().with_weather(bbb_server_persistence::Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 4000,
            thunder_time: 4000,
            raining: true,
            thundering: true,
        });
    let console = CommandSource::server("console", CommandPermission::Owner);
    for command in [
        "execute if predicate {condition:'minecraft:weather_check',raining:true,thundering:true}",
        "execute in minecraft:the_nether if predicate {condition:'minecraft:weather_check',raining:false,thundering:false}",
        "execute in minecraft:the_nether unless predicate {condition:'minecraft:weather_check',raining:true}",
    ] {
        let run = ran_as(command, &console, &mut fixture);
        assert_eq!(run.result, Some(1), "{command}: {:?}", keys(&run));
    }
}

#[test]
fn execute_loot_location_predicate_reads_current_block_entity_metadata() {
    let mut fixture = loaded_world();
    fixture.setup("setblock 1 64 1 chest{CustomName:'Treasure'}");
    fixture.setup("item replace block 1 64 1 container.0 with diamond 3");
    let console = CommandSource::server("console", CommandPermission::Owner);
    let predicate = "{condition:'minecraft:location_check',offsetY:-1,predicate:{block:{nbt:{id:'minecraft:chest',x:1,y:64,z:1,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]},components:{'minecraft:container':[{slot:0,item:{id:'minecraft:diamond',count:3}}]},predicates:{'minecraft:container':{items:{contains:[{items:'minecraft:diamond',count:3}]}},'minecraft:custom_name':{}}}}}";
    let command = format!("execute positioned 1 65 1 if predicate {predicate}");
    let run = ran_as(&command, &console, &mut fixture);
    assert_eq!(run.result, Some(1), "{command}: {run:?}");

    fixture.setup("item replace block 1 64 1 container.0 with diamond 4");
    let command = format!("execute positioned 1 65 1 unless predicate {predicate}");
    let run = ran_as(&command, &console, &mut fixture);
    assert_eq!(run.result, Some(1), "{command}: {run:?}");
}

#[test]
fn execute_loot_location_predicates_use_published_block_and_campfire_tags() {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    let tags = startup
        .tags
        .registries
        .iter_mut()
        .find(|tags| tags.registry_id == "minecraft:block")
        .unwrap();
    for tag in ["example:chosen", "minecraft:logs"] {
        tags.tags.insert(tag.into(), vec!["minecraft:stone".into()]);
    }
    tags.tags.insert("minecraft:campfires".into(), vec![]);
    let mut fixture = CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            datapacks: Some(std::sync::Arc::new(startup)),
            ..Default::default()
        },
        &[],
    )
    .with_block(BlockPos { x: 1, y: 64, z: 1 }, stone());
    fixture.setup("setblock 1 63 1 campfire[lit=true]");
    let console = CommandSource::server("console", CommandPermission::Owner);
    for command in [
        "execute positioned 1 64 1 if predicate [{condition:'minecraft:location_check',predicate:{block:{blocks:'#example:chosen'}}},{condition:'minecraft:location_check',predicate:{block:{blocks:'#minecraft:logs'}}}]",
        "execute positioned 1 64 1 unless predicate {condition:'minecraft:location_check',predicate:{smokey:true}}",
    ] {
        let run = ran_as(command, &console, &mut fixture);
        assert_eq!(run.result, Some(1), "{command}: {run:?}");
    }
}

#[test]
fn repeated_same_kind_execute_conditionals_ending_on_leaf_are_not_dropped() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut fixture = world(&players);
    fixture.setup("scoreboard objectives add obj dummy");
    fixture.setup("scoreboard players set a obj 1");
    fixture.setup("scoreboard players set b obj 2");

    // Repeated `if score ... matches` ending on a conditional leaf: when the
    // first `if` modifier fails, the fork drops all contexts so `store` is
    // untouched (`None`); when the first passes and the terminal leaf fails,
    // the leaf triggers `onFailure` (`Some(0)`); when both pass, `Some(1)`.
    let fail_first = ran(
        "execute store success score out obj if score a obj matches 99 if score b obj matches 2",
        &mut fixture,
    );
    assert_eq!(fail_first.result, None, "{fail_first:?}");
    assert_eq!(
        ran("scoreboard players get out obj", &mut fixture).result,
        None
    );

    let fail_second = ran(
        "execute store success score out obj if score a obj matches 1 if score b obj matches 99",
        &mut fixture,
    );
    assert_eq!(fail_second.result, None, "{fail_second:?}");
    assert_eq!(
        ran("scoreboard players get out obj", &mut fixture).result,
        Some(0)
    );

    let pass_both = ran(
        "execute store success score out obj if score a obj matches 1 if score b obj matches 2",
        &mut fixture,
    );
    assert_eq!(pass_both.result, Some(1), "{pass_both:?}");
    assert_eq!(
        ran("scoreboard players get out obj", &mut fixture).result,
        Some(1)
    );

    // Repeated `if entity` ending on a conditional leaf.
    let entity_fail_first = ran(
        "execute if entity @e[name=nobody] if entity @a",
        &mut fixture,
    );
    assert_eq!(entity_fail_first.result, None, "{entity_fail_first:?}");
    assert!(keys(&entity_fail_first).is_empty(), "{entity_fail_first:?}");

    let entity_pass_both = ran("execute if entity alice if entity @a", &mut fixture);
    assert_eq!(entity_pass_both.result, Some(2), "{entity_pass_both:?}");
}

#[test]
fn chained_execute_facing_entity_resolves_each_stage_target_separately() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut fixture = world(&players)
        .with_player_position("alice", [10.0, 64.0, 0.0])
        .with_player_position("bob", [0.0, 64.0, 10.0]);
    // First `facing entity alice feet` points -90 deg (east, +X), so `^ ^ ^10`
    // moves to (10, 64, 0) instead of (0, 64, 10). Then `facing entity bob feet`
    // turns toward bob. If the first stage mistakenly read the last `targets`
    // argument (`bob`), positioned `^ ^ ^10` would land near (0, 64, 10).
    let run = ran(
        "execute positioned 0 64 0 facing entity alice feet positioned ^ ^ ^10 facing entity bob feet if entity @a[distance=..0.5,name=alice]",
        &mut fixture,
    );
    assert_eq!(run.result, Some(1), "{run:?}");
}
