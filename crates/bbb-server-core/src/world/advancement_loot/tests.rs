use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::{GameProfile, JoinContext, ServerWorldConfig};
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_datapack::loot::{parse_loot_table, LootElement, LootElementValue};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::json;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

const PLAYER: ConnectionId = ConnectionId(1);

fn startup() -> Arc<DatapackStartup> {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    Arc::clone(STARTUP.get_or_init(|| {
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None, saved_data_packs: None, safe_mode: false, limits: Default::default(),
        }).unwrap();
        let registry = startup.loot.registries.iter_mut().find(|registry| registry.registry_id == "minecraft:loot_table").unwrap();
        for (id, item, count) in [("apple", "apple", 1), ("diamond", "diamond", 1), ("stone", "stone", 4)] {
            registry.elements.push(LootElement {
                id: format!("test:{id}"), source_pack: "test".into(),
                value: LootElementValue::Table(parse_loot_table(&json!({
                    "type":"minecraft:advancement_reward", "pools":[{"rolls":1,"conditions":[{
                        "condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:player"}
                    }],"entries":[{
                        "type":"minecraft:item", "name":format!("minecraft:{item}"),
                        "functions":[{"function":"minecraft:set_count","count":count}]
                    }]}]
                }), None).unwrap()),
            });
        }
        for (id, sequence) in [("level_random", false), ("sequence_random", true), ("unsupported", false)] {
            let condition = if id == "unsupported" {
                json!({"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":"test:unavailable"}})
            } else { json!({"condition":"minecraft:random_chance","chance":0.0}) };
            let mut table = json!({"type":"minecraft:advancement_reward","pools":[{
                "rolls":1,"conditions":[condition],"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]
            }]});
            if sequence { table["random_sequence"] = json!("test:reward_random"); }
            registry.elements.push(LootElement { id:format!("test:{id}"), source_pack:"test".into(),
                value:LootElementValue::Table(parse_loot_table(&table, None).unwrap()) });
        }
        registry.elements.sort_by(|a,b| a.id.cmp(&b.id));
        Arc::new(startup)
    }))
}

fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.join(
        PLAYER,
        GameProfile {
            uuid: Uuid::from_u128(1),
            name: "reward".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    world.cold.config.datapacks = Some(startup());
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.abilities.instabuild = false;
    let _ = player.inventory.clear_all_items(None);
    let _ = player
        .inventory
        .broadcast_changes(crate::inventory::test_item_resolver(), None);
    world
}

fn stack(name: &str, count: i32) -> RuntimeItemStack {
    crate::tests::native_stack(ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .unwrap()
                .id,
        ),
        count,
        component_patch: Default::default(),
    })
}

fn grant(world: &mut ServerWorld, tables: &[&str]) -> Result<Vec<WorldEvent>, String> {
    let mut events = Vec::new();
    world.grant_advancement_loot(
        PLAYER,
        &tables.iter().map(|id| (*id).into()).collect::<Vec<_>>(),
        &mut events,
    )?;
    Ok(events)
}

#[test]
fn loot_grant_orders_tables_and_sounds_before_one_final_menu_broadcast() {
    let mut world = world();
    let events = grant(&mut world, &["test:apple", "test:diamond"]).unwrap();
    let inventory = world.levels[PLAYER].inventory.inventory_slots();
    assert_eq!(inventory[0], stack("minecraft:apple", 1));
    assert_eq!(inventory[1], stack("minecraft:diamond", 1));
    let packets = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet,
            } if *connection_id == PLAYER => Some(packet),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(matches!(packets[0], ClientboundPlayPacket::Sound(_)));
    assert!(matches!(packets[1], ClientboundPlayPacket::Sound(_)));
    assert_eq!(
        packets
            .iter()
            .filter(|packet| matches!(packet, ClientboundPlayPacket::Sound(_)))
            .count(),
        2
    );
    assert_eq!(
        packets
            .iter()
            .filter(|packet| matches!(packet, ClientboundPlayPacket::ContainerSetSlot(_)))
            .count(),
        2
    );
    assert!(packets[2..]
        .iter()
        .all(|packet| !matches!(packet, ClientboundPlayPacket::Sound(_))));
}

#[test]
fn loot_grant_drops_only_uninserted_remainder_and_creative_discards_overflow() {
    for (free_count, creative) in [(0, false), (2, false), (0, true)] {
        let mut world = world();
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.abilities.instabuild = creative;
        for index in 0..player.inventory.inventory_slots().len() {
            player
                .inventory
                .set_inventory_slot(index, stack("minecraft:dirt", 64));
        }
        *player.inventory.slot_mut(0).unwrap() = stack("minecraft:stone", 64 - free_count);
        let _ = player
            .inventory
            .broadcast_changes(crate::inventory::test_item_resolver(), None);
        let dimension = player.dimension;
        let events = grant(&mut world, &["test:stone"]).unwrap();
        let entities = &world.level_by_id(&dimension).entities;
        let drops = entities
            .ids()
            .into_iter()
            .filter_map(|id| match entities.get(id).unwrap().data() {
                crate::entity::EntityData::Item(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        if creative {
            assert!(drops.is_empty());
            assert!(events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(_),
                    ..
                }
            )));
        } else {
            assert_eq!(drops.len(), 1);
            assert_eq!(drops[0].target(), Some(Uuid::from_u128(1)));
            assert_eq!(drops[0].pickup_delay(), 0);
            assert_eq!(drops[0].thrower(), None);
            assert_eq!(
                crate::entity::item::stack_count(drops[0].item()),
                4 - free_count
            );
            assert_eq!(
                world.levels[PLAYER].inventory.inventory_slots()[0].count(),
                64
            );
        }
    }
}
