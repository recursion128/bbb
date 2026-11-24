use super::*;
use bbb_protocol::game_types::GameType;

fn advancement(trigger: &str, conditions: Value, experience: i32) -> Value {
    json!({"criteria":{"used":{"trigger":trigger,"conditions":conditions}},
        "rewards":{"experience":experience}})
}

fn interaction_state(block: &str) -> i32 {
    let properties: &[(&str, &str)] = match block {
        "minecraft:lever" => &[("face", "floor"), ("facing", "north"), ("powered", "false")],
        "minecraft:chest" => &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
        _ => &[],
    };
    block_state(block, properties)
}

fn interaction(
    world: &mut ServerWorld,
    pos: BlockPos,
    hand: InteractionHand,
    sequence: i32,
) -> Vec<WorldEvent> {
    world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: CONNECTION,
                packet: PlayServerbound::UseItemOn(UseItemOn {
                    hand,
                    hit: BlockHitResult {
                        pos,
                        direction: Direction::Up,
                        cursor_x: 0.5,
                        cursor_y: 0.5,
                        cursor_z: 0.5,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence,
                }),
            },
        )])
        .events
}

#[test]
fn block_use_advancement_default_and_any_read_post_action_state_and_real_player() {
    let player = json!({"type":"minecraft:player","distance":{"absolute":{"max":0}},
        "nbt":"{SelectedItem:{id:'minecraft:stick',count:1}}",
        "location":{"block":{"nbt":"{id:'minecraft:chest',x:8,y:70,z:8,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
    });
    let location = json!([
        {"condition":"block_state_property","block":"minecraft:lever","properties":{"powered":"true"}},
        {"condition":"entity_properties","entity":"this","predicate":{
            "type":"minecraft:player","distance":{"absolute":{"min":1}},
            "nbt":"{SelectedItem:{id:'minecraft:stick',count:1}}"
        }},
        {"condition":"location_check","offsetX":1,"predicate":{"block":{"nbt":"{id:'minecraft:chest',x:9,y:70,z:10,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}}
    ]);
    let mut definitions = Vec::new();
    for (trigger, prefix) in [("default_block_use", "default"), ("any_block_use", "any")] {
        for (suffix, player) in [
            ("object", player.clone()),
            (
                "list",
                json!([{"condition":"entity_properties","entity":"this","predicate":player}]),
            ),
            ("wrong_object", json!({"type":"minecraft:zombie"})),
            (
                "wrong_list",
                json!([{"condition":"entity_properties","entity":"this","predicate":{"type":"minecraft:zombie"}}]),
            ),
        ] {
            definitions.push((
                format!("test:{prefix}_{suffix}"),
                advancement(trigger, json!({"player":player,"location":location}), 1),
            ));
        }
    }
    let definitions = definitions
        .iter()
        .map(|(id, body)| (id.as_str(), body.clone()))
        .collect::<Vec<_>>();
    let mut world = world(&definitions);
    teleport(&mut world, Java26BuiltinDimension::Overworld, 8.5, 8.5);
    player_loaded(&mut world);
    diamond_chest(&mut world, BlockPos { x: 8, y: 70, z: 8 });
    diamond_chest(&mut world, BlockPos { x: 9, y: 70, z: 10 });
    let at = beside_player(&mut world, interaction_state("minecraft:lever"));
    give(&mut world, "minecraft:diamond", 1);
    click(&mut world, at, 1);
    assert_eq!(xp(&world), 0);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(block_state(
            "minecraft:lever",
            &[("face", "floor"), ("facing", "north"), ("powered", "true")]
        ))
    );
    click(&mut world, at, 2);
    assert_eq!(xp(&world), 0);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(interaction_state("minecraft:lever"))
    );
    command(&mut world, "clear @s minecraft:diamond");
    give(&mut world, "minecraft:stick", 1);
    world.levels[CONNECTION]
        .cold
        .cooldowns
        .add_group_cooldown("minecraft:stick".into(), 100);
    click(&mut world, at, 3);
    assert_eq!(xp(&world), 4);
    for trigger in ["default", "any"] {
        for suffix in ["object", "list"] {
            assert!(
                world.player_advancement_is_done(CONNECTION, &format!("test:{trigger}_{suffix}"))
            );
            assert!(!world
                .player_advancement_is_done(CONNECTION, &format!("test:{trigger}_wrong_{suffix}")));
        }
    }
    click(&mut world, at, 4);
    click(&mut world, at, 5);
    assert_eq!(xp(&world), 4);
}

#[test]
fn block_use_advancement_item_block_use_has_post_consume_tool_and_no_default() {
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let mut world = world(&[
            (
                "test:any",
                advancement(
                    "any_block_use",
                    json!({"location":[
                        {"condition":"block_state_property","block":"minecraft:respawn_anchor","properties":{"charges":"1"}},
                        {"condition":"match_tool","predicate":{"items":"minecraft:glowstone","count":1}}
                    ]}),
                    2,
                ),
            ),
            (
                "test:old_count",
                advancement(
                    "any_block_use",
                    json!({"location":[
                        {"condition":"match_tool","predicate":{"items":"minecraft:glowstone","count":2}}
                    ]}),
                    3,
                ),
            ),
            (
                "test:default",
                advancement("default_block_use", json!({}), 5),
            ),
        ]);
        player_loaded(&mut world);
        command(&mut world, "gamemode survival");
        let at = beside_player(
            &mut world,
            block_state("minecraft:respawn_anchor", &[("charges", "0")]),
        );
        give(&mut world, "minecraft:glowstone", 2);
        if hand == InteractionHand::OffHand {
            let stack = world.levels[CONNECTION].inventory.selected_stack().clone();
            world.levels[CONNECTION]
                .inventory
                .set_item_in_hand(InteractionHand::MainHand, RuntimeItemStack::empty());
            world.levels[CONNECTION]
                .inventory
                .set_item_in_hand(hand, stack);
        }
        interaction(&mut world, at, hand, 1);
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, at),
            Some(block_state("minecraft:respawn_anchor", &[("charges", "1")]))
        );
        assert_eq!(
            crate::world::using_item::runtime_item_in_hand(
                world.levels.player_at(CONNECTION),
                hand
            )
            .count(),
            1
        );
        assert!(world.player_advancement_is_done(CONNECTION, "test:any"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:old_count"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:default"));
        assert_eq!(xp(&world), 2);
    }
}

#[test]
fn block_use_advancement_placement_uses_clicked_cell_and_does_not_use_default() {
    let mut world = world(&[
        (
            "test:clicked",
            advancement(
                "any_block_use",
                json!({"location":[
                    {"condition":"block_state_property","block":"minecraft:stone"},
                    {"condition":"match_tool","predicate":{"items":"minecraft:dirt","count":1}}
                ]}),
                2,
            ),
        ),
        (
            "test:placed",
            advancement(
                "any_block_use",
                json!({"location":[
                    {"condition":"block_state_property","block":"minecraft:dirt"}
                ]}),
                3,
            ),
        ),
        (
            "test:default",
            advancement("default_block_use", json!({}), 5),
        ),
    ]);
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    let at = beside_player(&mut world, block_state("minecraft:stone", &[]));
    give(&mut world, "minecraft:dirt", 2);
    world.levels[CONNECTION]
        .cold
        .cooldowns
        .add_group_cooldown("minecraft:dirt".into(), 100);
    click(&mut world, at, 1);
    assert_eq!(xp(&world), 0);
    assert_eq!(
        world.levels[CONNECTION].inventory.selected_stack().count(),
        2
    );
    assert_eq!(
        world.block_state(
            Java26BuiltinDimension::Overworld,
            BlockPos { y: at.y + 1, ..at }
        ),
        Some(
            world
                .level(Java26BuiltinDimension::Overworld)
                .chunks
                .air_state_id()
        )
    );
    world.levels[CONNECTION].cold.cooldowns = Default::default();
    click(&mut world, at, 2);
    assert_eq!(
        world.block_state(
            Java26BuiltinDimension::Overworld,
            BlockPos { y: at.y + 1, ..at }
        ),
        Some(block_state("minecraft:dirt", &[]))
    );
    assert!(world.player_advancement_is_done(CONNECTION, "test:clicked"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:placed"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:default"));
    assert_eq!(xp(&world), 2);
}

#[test]
fn block_use_advancement_only_consuming_unsuppressed_main_hand_has_default() {
    for (hand, sneaking, held, block, sequence, expected_default, expected_any) in [
        (
            InteractionHand::MainHand,
            false,
            false,
            "minecraft:lever",
            1,
            true,
            true,
        ),
        (
            InteractionHand::OffHand,
            false,
            false,
            "minecraft:lever",
            1,
            false,
            false,
        ),
        (
            InteractionHand::MainHand,
            true,
            true,
            "minecraft:lever",
            1,
            false,
            false,
        ),
        (
            InteractionHand::MainHand,
            true,
            false,
            "minecraft:lever",
            1,
            true,
            true,
        ),
        (
            InteractionHand::MainHand,
            false,
            false,
            "minecraft:stone",
            1,
            false,
            false,
        ),
        (
            InteractionHand::MainHand,
            false,
            false,
            "minecraft:lever",
            -1,
            false,
            false,
        ),
    ] {
        let mut world = world(&[
            (
                "test:default",
                advancement("default_block_use", json!({}), 2),
            ),
            ("test:any", advancement("any_block_use", json!({}), 3)),
        ]);
        player_loaded(&mut world);
        let at = beside_player(&mut world, interaction_state(block));
        if held {
            give(&mut world, "minecraft:stick", 1);
        }
        world.levels[CONNECTION].set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            sneaking,
        );
        let events = interaction(&mut world, at, hand, sequence);
        if sequence < 0 {
            assert!(events.iter().any(|event| matches!(
                event,
                WorldEvent::Close {
                    connection_id: CONNECTION,
                    reason: crate::world::WorldCloseReason::InvalidBlockInteraction
                }
            )));
            assert!(!events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: crate::connection::ClientboundPlayPacket::SetExperience(_),
                    ..
                }
            )));
            continue;
        }
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:default"),
            expected_default,
            "{hand:?} {sneaking} {held} {block} {sequence}"
        );
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:any"),
            expected_any,
            "{hand:?} {sneaking} {held} {block} {sequence}"
        );
    }
}

#[test]
fn block_use_advancement_spectator_requires_menu_and_never_uses_default() {
    for (block, consumes) in [("minecraft:stone", false), ("minecraft:chest", true)] {
        let mut world = world(&[
            (
                "test:default",
                advancement("default_block_use", json!({}), 2),
            ),
            ("test:any", advancement("any_block_use", json!({}), 3)),
        ]);
        player_loaded(&mut world);
        let at = beside_player(&mut world, interaction_state(block));
        command(&mut world, "gamemode spectator");
        assert_eq!(world.levels[CONNECTION].game_type, GameType::Spectator);
        click(&mut world, at, 1);
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:any"),
            consumes
        );
        assert!(!world.player_advancement_is_done(CONNECTION, "test:default"));
    }
}

#[test]
fn block_use_advancement_debug_stick_requires_consuming_success() {
    for (game_mode, block, consumes) in [
        ("survival", "minecraft:oak_log", false),
        ("creative", "minecraft:stone", false),
        ("creative", "minecraft:oak_log", true),
    ] {
        let mut world = world(&[
            (
                "test:any",
                advancement(
                    "any_block_use",
                    json!({"location":[
                        {"condition":"match_tool","predicate":{"items":"minecraft:debug_stick"}}
                    ]}),
                    2,
                ),
            ),
            (
                "test:legacy",
                advancement("item_used_on_block", json!({}), 3),
            ),
            (
                "test:default",
                advancement("default_block_use", json!({}), 5),
            ),
        ]);
        player_loaded(&mut world);
        command(&mut world, &format!("gamemode {game_mode}"));
        let state = if block == "minecraft:oak_log" {
            block_state(block, &[("axis", "y")])
        } else {
            block_state(block, &[])
        };
        let at = beside_player(&mut world, state);
        give(&mut world, "minecraft:debug_stick", 1);
        click(&mut world, at, 1);
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:any"),
            consumes,
            "{game_mode} {block}"
        );
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:legacy"),
            consumes,
            "{game_mode} {block}"
        );
        assert!(!world.player_advancement_is_done(CONNECTION, "test:default"));
        assert_eq!(xp(&world), if consumes { 5 } else { 0 });
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, at) != Some(state),
            consumes
        );
        click(&mut world, at, 2);
        assert_eq!(xp(&world), if consumes { 5 } else { 0 });
    }
}
