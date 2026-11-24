use super::*;

use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::block_types::VANILLA_BLOCK_INFESTED_STONE_ID;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn mining_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Creative,
        force_game_mode: true,
        admin_lists: operator_config(&[("miner", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "observer", 2);
    world.set_player_position_for_test(
        ConnectionId(2),
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 2.5,
        },
    );
    world.tick(receiver.drain());
    (sender, receiver, world)
}

fn infested_tool(world: &ServerWorld, silk_touch_level: Option<i32>) -> ItemStackSummary {
    let mut tool = ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                "minecraft:diamond_pickaxe",
            )
            .expect("diamond pickaxe")
            .id,
        ),
        count: 1,
        ..ItemStackSummary::empty()
    };
    tool.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    tool.component_patch.tool = Some(bbb_protocol::packets::ToolSummary {
        rules: vec![bbb_protocol::packets::ToolRuleSummary {
            block_registry_ids: vec![VANILLA_BLOCK_INFESTED_STONE_ID],
            block_tag: None,
            speed_bits: 1_000.0_f32.to_bits().into(),
            correct_for_drops: Some(true),
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    if let Some(level) = silk_touch_level {
        let holder_id = world
            .datapacks()
            .expect("datapack generation")
            .registries
            .registry("minecraft:enchantment")
            .expect("enchantment registry")
            .elements
            .iter()
            .position(|entry| entry.id == "minecraft:silk_touch")
            .expect("silk touch holder");
        tool.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
        tool.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: i32::try_from(holder_id).unwrap(),
            level,
        }];
    }
    tool
}

fn hold_tool(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    tool: ItemStackSummary,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: tool,
        }),
    );
}

fn mine(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: 64.0,
            z: f64::from(pos.z) - 2.5,
        },
    );
    world.tick(receiver.drain());
    assert!(world.set_block(
        DIM,
        pos,
        block_state_id("minecraft:infested_stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let result = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos,
            direction: Direction::Up,
            sequence,
        }),
    );
    assert_eq!(
        world.block_state(DIM, pos),
        Some(block_state_id("minecraft:air"))
    );
    result
}

fn silverfish_ids(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SILVERFISH_ID)
        })
        .collect()
}

fn assert_spawn_order(result: &TickResult, entity_id: i32) {
    let base = result
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::LevelEvent(packet)) if packet.event_type == 2001
            )
        })
        .expect("base block destroy event");
    let added = result
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::AddEntity(packet)) if packet.id == entity_id
            )
        })
        .expect("silverfish AddEntity");
    let animated = result
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::EntityEvent(packet))
                    if packet.entity_id == entity_id && packet.event_id == 20
            )
        })
        .expect("silverfish spawnAnim");
    assert!(base < added && added < animated, "{:?}", result.events);
}

#[test]
fn real_survival_tool_uses_current_infested_prevention_tag_generation() {
    let (sender, receiver, mut world) = mining_world();
    let bare_tool = infested_tool(&world, None);
    let silk_tool = infested_tool(&world, Some(1));
    hold_tool(&sender, &receiver, &mut world, bare_tool);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );

    let bare = mine(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 2, y: 64, z: 2 },
        1,
    );
    let spawned = silverfish_ids(&world);
    assert_eq!(spawned.len(), 1);
    assert_spawn_order(&bare, spawned[0]);
    assert_eq!(
        world
            .stored_entity_for_test(DIM, spawned[0])
            .unwrap()
            .position(),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5
        },
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode creative",
    );
    hold_tool(&sender, &receiver, &mut world, silk_tool);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    let loaded = world.level_registry_tags_for_test(DIM);
    assert!(loaded.contains(
        "minecraft:enchantment",
        "minecraft:prevents_infested_spawns",
        "minecraft:silk_touch"
    ));
    mine(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 3, y: 64, z: 2 },
        2,
    );
    assert_eq!(silverfish_ids(&world), spawned);

    // The server-thread reload install changes the level's current snapshot.
    world.install_level_registry_tags(Arc::new(RegistryTagSnapshot::new([(
        "minecraft:enchantment".to_owned(),
        "minecraft:prevents_infested_spawns".to_owned(),
        vec![],
    )])));
    assert!(!world.level_registry_tags_for_test(DIM).contains(
        "minecraft:enchantment",
        "minecraft:prevents_infested_spawns",
        "minecraft:silk_touch"
    ));
    let after_reload = mine(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 4, y: 64, z: 2 },
        3,
    );
    let now = silverfish_ids(&world);
    assert_eq!(now.len(), 2);
    let new_id = *now.iter().find(|id| !spawned.contains(id)).unwrap();
    assert_spawn_order(&after_reload, new_id);
    world.install_level_registry_tags(loaded);
}

#[test]
fn block_drops_rule_and_creative_break_suppress_session_silverfish() {
    let (sender, receiver, mut world) = mining_world();
    let tool = infested_tool(&world, None);
    hold_tool(&sender, &receiver, &mut world, tool);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule block_drops false",
    );
    assert_eq!(
        world.game_rule_for_test("minecraft:block_drops"),
        Some(GameRuleValue::Boolean(false)),
    );
    mine(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 2, y: 64, z: 2 },
        1,
    );
    assert!(silverfish_ids(&world).is_empty());

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule block_drops true",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode creative",
    );
    mine(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 3, y: 64, z: 2 },
        2,
    );
    assert!(silverfish_ids(&world).is_empty());
}
