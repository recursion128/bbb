use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::player_save::restored_player_from_save_patch;
use bbb_protocol::game_rules::GameRuleValue;

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: tool("minecraft:stone", count).item_id,
        count,
        component_patch: Default::default(),
    }
}

#[test]
fn inventory_advancement_suppressed_remote_still_awards_and_full_state_flushes() {
    let mut world = world(&[(
        "test:local",
        inventory(
            json!({"items":[{"items":"minecraft:stone"}]}),
            json!({"experience":7}),
        ),
    )]);
    let player = world.levels.player_mut(CONNECTION).unwrap();
    *player.inventory.slot_mut(9).unwrap() = crate::tests::native_stack(stone(1));
    player.inventory.suppress_remote_updates();
    let mut events = Vec::new();
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut events);
    assert_eq!(xp(&world), 7);
    assert!(events.is_empty());
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::FullState, &mut events);
    assert_eq!(xp(&world), 7);
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetContent(_),
            ..
        }
    )));
}

#[test]
fn inventory_advancement_restored_slots_award_before_initial_content_and_do_not_replay() {
    let definitions = [(
        "test:restore",
        inventory(
            json!({"items":[{"items":"minecraft:stone"}]}),
            json!({"experience":9,"recipes":["minecraft:oak_planks"]}),
        ),
    )];
    let mut original = world(&definitions);
    *original.levels[CONNECTION].inventory.slot_mut(9).unwrap() =
        crate::tests::native_stack(stone(1));
    let patch = original
        .player_save_patch(original.levels.player_at(CONNECTION))
        .unwrap();
    let mut restored = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup(&definitions)),
        ..Default::default()
    })
    .unwrap();
    let saved = restored_player_from_save_patch(&patch).unwrap();
    restored
        .cold
        .restored_players
        .insert(Uuid::from_u128(1), saved);
    let mut events = Vec::new();
    restored.join(
        CONNECTION,
        GameProfile {
            uuid: Uuid::from_u128(1),
            name: "inventory_player".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut events,
    );
    assert_eq!(xp(&restored), 9);
    restored.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut events);
    assert_eq!(xp(&restored), 9);
}

#[test]
fn inventory_advancement_respawn_registers_kept_slots_and_sends_initial_content() {
    let mut world = world(&[(
        "test:respawn",
        inventory(
            json!({"items":[{"items":"minecraft:stone"}]}),
            json!({"experience":4}),
        ),
    )]);
    world.set_game_rule_value(
        "minecraft:keep_inventory",
        GameRuleValue::Boolean(true),
        &mut Vec::new(),
    );
    let player = world.levels.player_mut(CONNECTION).unwrap();
    *player.inventory.slot_mut(9).unwrap() = crate::tests::native_stack(stone(1));
    player.damage.health = 0.0;
    let mut events = Vec::new();
    world.perform_respawn(CONNECTION, &mut events);
    assert_eq!(xp(&world), 4);
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetContent(_),
            ..
        }
    )));
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut events);
    assert_eq!(xp(&world), 4);
}

#[test]
fn inventory_advancement_reward_merge_does_not_send_the_outer_stale_count() {
    use bbb_server_datapack::loot::{parse_loot_table, LootElement, LootElementValue};
    let definitions = [
        (
            "test:merge",
            inventory(
                json!({"player":{"movement":{"x":{"min":4.0,"max":6.0}},
                    "effects":{"speed":{"amplifier":1,"visible":false}}},
                "items":[{"items":"minecraft:stone","count":1}]}),
                json!({"loot":["test:stone"]}),
            ),
        ),
        (
            "test:still",
            inventory(
                json!({"player":{"movement":{"x":0.0}},
                "items":[{"items":"minecraft:stone","count":1}]}),
                json!({"experience":17}),
            ),
        ),
    ];
    let mut data = (*startup(&definitions)).clone();
    let registry = data
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    registry.elements.push(LootElement {id: "test:stone".into(), source_pack: "test".into(), value: LootElementValue::Table(parse_loot_table(&json!({"type":"minecraft:advancement_reward","pools":[{"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this","predicate":{"movement":{"x":{"min":4.0,"max":6.0}},"effects":{"speed":{"amplifier":1,"visible":false}}}}],"entries":[{"type":"minecraft:item","name":"minecraft:stone"}]}]}), None).unwrap())});
    registry.elements.sort_by(|a, b| a.id.cmp(&b.id));
    let mut world = world(&definitions);
    world.cold.config.datapacks = Some(Arc::new(data));
    player_loaded(&mut world);
    effect(&mut world, "give @s minecraft:speed 60 1 true");
    let start = world.levels[CONNECTION].position();
    world.apply_play(
        CONNECTION,
        PlayServerbound::MovePlayer(bbb_protocol::packets::MovePlayer {
            position: Some(Vec3d {
                x: start.x + 0.25,
                ..start
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
        &mut Vec::new(),
    );
    let mut events = give(&mut world, "minecraft:stone", 1);
    assert_eq!(xp(&world), 0);
    assert_eq!(
        world.levels[CONNECTION].inventory.inventory_slots()[0].count(),
        2
    );
    let counts = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(packet),
                ..
            } if packet.item.item_id == stone(1).item_id => Some(packet.item.count),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(counts, [2]);
    events.clear();
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut events);
    assert!(events.is_empty());
}

#[test]
fn inventory_advancement_creative_prediction_still_fires_local_listener() {
    let mut world = world(&[(
        "test:creative",
        inventory(
            json!({"items":[{"items":"minecraft:stone"}]}),
            json!({"experience":6}),
        ),
    )]);
    world.set_player_game_mode(
        CONNECTION,
        bbb_protocol::game_types::GameType::Creative,
        &mut Vec::new(),
    );
    let events = world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            1,
            crate::mailbox::WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: CONNECTION,
                packet: PlayServerbound::SetCreativeModeSlot(
                    bbb_protocol::packets::SetCreativeModeSlot {
                        slot_num: 36,
                        item: stone(1),
                    },
                ),
            },
        )])
        .events;
    assert_eq!(xp(&world), 6);
    assert!(!events.iter().any(|event| matches!(event, WorldEvent::Send {packet: ClientboundPlayPacket::ContainerSetSlot(packet), ..} if packet.slot == 36)));
}
