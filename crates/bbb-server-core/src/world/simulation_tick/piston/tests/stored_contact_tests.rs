use super::*;

#[test]
fn piston_lava_destroys_named_loot_vehicles_through_the_original_cargo_owner() {
    piston_lava_named_loot_vehicle_story(false);
    piston_lava_named_loot_vehicle_story(true);
}

fn piston_lava_named_loot_vehicle_story(minecart: bool) {
    use crate::entity::EntityLifecycleEvent;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID;
    use bbb_server_datapack::loot::{LootElement, LootElementValue};
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    let table = "{type:'minecraft:chest',pools:[{rolls:1,entries:[{type:'minecraft:item',name:'minecraft:stone',functions:[{function:'set_count',count:3}]}]}]}";
    let bbb_protocol::ScannedCommandResourceOrId::Inline { value, .. } =
        bbb_protocol::scan_command_resource_or_id(table, 0).unwrap()
    else {
        unreachable!()
    };
    startup
        .loot
        .registries
        .iter_mut()
        .find(|row| row.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .push(LootElement {
            id: "bbb:vehicle_source".into(),
            source_pack: "test".into(),
            value: LootElementValue::Table(
                bbb_server_datapack::loot::parse_loot_table_nbt(&value, None).unwrap(),
            ),
        });
    startup
        .loot
        .registries
        .iter_mut()
        .find(|row| row.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .sort_by(|left, right| left.id.cmp(&right.id));
    let mut config = crate::tests::operator_config(&[("alice", 1)]);
    config.datapacks = Some(std::sync::Arc::new(startup));
    let (mut world, dimension, pos, ids) = batch_world_with_config(config);
    for id in ids {
        world
            .level_mut_by_id(&dimension)
            .entities
            .remove(id, crate::entity::RemovalReason::UnloadedToChunk);
    }
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 3.5,
        y: 64.0,
        z: 3.5,
    });
    world.update_player_query_section(ConnectionId(1));
    let resource = if minecart {
        "minecraft:chest_minecart"
    } else {
        "minecraft:oak_chest_boat"
    };
    let position = Vec3d {
        x: 5.25,
        y: 64.0,
        z: 0.5,
    };
    let vehicle_id = if minecart {
        crate::entity::minecart_container::spawn_minecart_container(
            &mut world.level_mut_by_id(&dimension).entities,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            position,
            0.0,
        )
    } else {
        crate::entity::chest_boat::spawn_chest_boat(
            &mut world.level_mut_by_id(&dimension).entities,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            position,
            0.0,
        )
    }
    .unwrap();
    assert!(
        world
            .hurt_stored_entity_by_command_for_test(
                Java26BuiltinDimension::Overworld,
                vehicle_id,
                "minecraft:generic",
                1.0,
                None
            )
            .unwrap()
            .succeeded
    );
    let mut events = Vec::new();
    assert_eq!(world.apply_chat_command(ConnectionId(1),
            &format!("data merge entity @e[type={resource},limit=1] {{CustomName:{{text:'Cargo'}},LootTable:'bbb:vehicle_source',LootTableSeed:19L}}"),
            &mut events), None);
    let body = world
        .level_by_id(&dimension)
        .entities
        .get(vehicle_id)
        .unwrap();
    assert!(body.custom_name().is_some());
    let (loot_table, damage) = match body.data() {
        crate::entity::EntityData::ChestBoat(data) => (
            data.loot_table_id().map(|id| id.to_string()),
            data.boat().damage(),
        ),
        crate::entity::EntityData::ChestMinecart(data) => (
            data.loot_table_id().map(|id| id.to_string()),
            data.minecart().damage(),
        ),
        _ => unreachable!(),
    };
    assert_eq!(loot_table.as_deref(), Some("bbb:vehicle_source"));
    assert_eq!(damage, 10.0);
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(6, 64, 0, batch_state("minecraft:lava").unwrap())
        .unwrap();
    world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    events.clear();
    world.tick_moving_piston(&dimension, pos, &mut events);
    assert!(world
        .level_by_id(&dimension)
        .entities
        .get(vehicle_id)
        .is_none());
    assert_eq!(
        world
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .piston_snapshot()
            .unwrap()
            .progress,
        0.5
    );
    let lifecycle = world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    let cargo_count: i32 = lifecycle
        .iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Added { entity, .. } => match entity.data() {
                crate::entity::EntityData::Item(data)
                    if data.item().item_resource_id() == Some("minecraft:stone") =>
                {
                    Some(data.item().count())
                }
                _ => None,
            },
            _ => None,
        })
        .sum();
    assert_eq!(cargo_count, 3);
    let vehicle = lifecycle
        .iter()
        .find_map(|event| match event {
            EntityLifecycleEvent::Added { entity, .. } => match entity.data() {
                crate::entity::EntityData::Item(data)
                    if data
                        .item()
                        .item_resource_id()
                        .is_some_and(|item| item == resource) =>
                {
                    Some(data.item())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let name = vehicle
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            &world.item_stack_resolver(),
        )
        .unwrap()
        .unwrap();
    assert!(bbb_protocol::canonical_snbt_string(&name).contains("Cargo"));
    assert_eq!(
        lifecycle
            .iter()
            .filter(|event| matches!(event,
            EntityLifecycleEvent::Removed { entity, .. } if entity.id() == vehicle_id))
            .count(),
        1
    );
}

#[test]
fn piston_cactus_destroys_a_named_slow_tnt_minecart_as_a_fresh_vehicle_item() {
    use crate::entity::{EntityData, EntityLifecycleEvent, RemovalReason};
    let mut config = crate::tests::operator_config(&[("alice", 1)]);
    config.datapacks = Some(crate::registry_sync::builtin_datapack_startup_shared());
    let (mut world, dimension, pos, old_ids) = batch_world_with_config(config);
    for id in old_ids {
        world
            .level_mut_by_id(&dimension)
            .entities
            .remove(id, RemovalReason::UnloadedToChunk);
    }
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 3.5,
        y: 64.0,
        z: 3.5,
    });
    world.update_player_query_section(ConnectionId(1));
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .configure_piston(
            batch_state("minecraft:stone").unwrap(),
            Direction::East,
            true,
            false
        ));
    let id = crate::entity::minecart::spawn_tnt_minecart(
        &mut world.level_mut_by_id(&dimension).entities,
        Vec3d {
            x: 5.25,
            y: 64.0,
            z: 0.5,
        },
        0.0,
    )
    .unwrap();
    let mut events = Vec::new();
    assert_eq!(world.apply_chat_command(ConnectionId(1),
            "data merge entity @e[type=minecraft:tnt_minecart,limit=1] {CustomName:{text:'Cactus cart'}}",
            &mut events), None);
    assert!(
        world
            .hurt_stored_entity_by_command_for_test(
                Java26BuiltinDimension::Overworld,
                id,
                "minecraft:generic",
                4.0,
                None,
            )
            .unwrap()
            .succeeded
    );
    let body = world.level_by_id(&dimension).entities.get(id).unwrap();
    assert!(body.custom_name().is_some());
    let EntityData::TntMinecart(data) = body.data() else {
        unreachable!()
    };
    assert_eq!(data.minecart().damage(), 40.0);
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(6, 64, 0, batch_state("minecraft:cactus").unwrap())
        .unwrap();
    world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    events.clear();
    world.tick_moving_piston(&dimension, pos, &mut events);
    assert!(world.level_by_id(&dimension).entities.get(id).is_none());
    assert!(!world
        .level_by_id(&dimension)
        .entities
        .has_pending_explosions());
    let lifecycle = world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    let drops: Vec<_> = lifecycle
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            EntityLifecycleEvent::Added { entity, .. } => match entity.data() {
                EntityData::Item(data)
                    if data
                        .item()
                        .item_resource_id()
                        .is_some_and(|item| item == "minecraft:tnt_minecart") =>
                {
                    Some((index, data.item()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].1.count(), 1);
    let name = drops[0]
        .1
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            &world.item_stack_resolver(),
        )
        .unwrap()
        .unwrap();
    assert!(bbb_protocol::canonical_snbt_string(&name).contains("Cactus cart"));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        game_events
            .iter()
            .filter(
                |event| event.game_event == crate::level_event::VanillaGameEvent::ENTITY_DIE
                    && event.source_entity_id == Some(id)
            )
            .count(),
        1
    );
    assert!(!game_events
        .iter()
        .any(|event| event.game_event == crate::level_event::VanillaGameEvent::EXPLODE));
}

#[test]
fn piston_crystal_explosion_removes_both_crystals_with_one_explosion() {
    use crate::entity::RemovalReason;
    let (mut world, dimension, pos, old_ids) = batch_world();
    let builtin = Java26BuiltinDimension::Overworld;
    for x in -1..=1 {
        for z in -1..=1 {
            world.ensure_entity_transfer_full_chunk_for_test(builtin, (x, z));
        }
    }
    for id in old_ids {
        world
            .level_mut_by_id(&dimension)
            .entities
            .remove(id, RemovalReason::UnloadedToChunk);
    }
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 14.5,
        y: 80.0,
        z: 14.5,
    });
    world.update_player_query_section(ConnectionId(1));
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .configure_piston(
            batch_state("minecraft:stone").unwrap(),
            Direction::East,
            true,
            false
        ));
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(6, 64, 0, batch_state("minecraft:cactus").unwrap())
        .unwrap();
    let ids = [64.0, 64.75].map(|y| {
        crate::entity::end_crystal::spawn_end_crystal(
            &mut world.level_mut_by_id(&dimension).entities,
            Vec3d { x: 5.25, y, z: 0.5 },
            true,
        )
        .unwrap()
    });
    let mut events = Vec::new();
    world.tick_moving_piston(&dimension, pos, &mut events);
    let store = &world.level_by_id(&dimension).entities;
    for id in ids {
        assert!(store.get(id).is_none());
    }
    assert!(!store.has_pending_explosions());
    assert!(world
        .level_by_id(&dimension)
        .chunks
        .block_entities()
        .get(pos)
        .is_none());
    let (game_events, _) = world.game_events(builtin);
    assert_eq!(
        game_events
            .iter()
            .filter(|event| event.game_event == crate::level_event::VanillaGameEvent::EXPLODE)
            .count(),
        1
    );
}

#[test]
fn piston_lava_moves_a_fire_immune_crystal_without_requiring_special_hurt() {
    let (mut world, dimension, pos, ids) = batch_world();
    for id in ids {
        world
            .level_mut_by_id(&dimension)
            .entities
            .remove(id, crate::entity::RemovalReason::UnloadedToChunk);
    }
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 3.5,
        y: 64.0,
        z: 3.5,
    });
    world.update_player_query_section(ConnectionId(1));
    let before = Vec3d {
        x: 5.25,
        y: 64.0,
        z: 0.5,
    };
    let id = crate::entity::end_crystal::spawn_end_crystal(
        &mut world.level_mut_by_id(&dimension).entities,
        before,
        true,
    )
    .unwrap();
    let uuid = world
        .level_by_id(&dimension)
        .entities
        .get(id)
        .unwrap()
        .uuid();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(6, 64, 0, batch_state("minecraft:lava").unwrap())
        .unwrap();
    world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    let mut events = Vec::new();
    world.tick_moving_piston(&dimension, pos, &mut events);
    let level = world.level_by_id(&dimension);
    let body = level.entities.get(id).unwrap();
    assert_eq!(body.uuid(), uuid);
    assert!(body.removal_reason().is_none());
    assert!(body.position().x > before.x);
    assert!(body.remaining_fire_ticks() <= 0);
    assert!(!level.entities.has_pending_explosions());
    assert_eq!(
        level
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .piston_snapshot()
            .unwrap()
            .progress,
        0.5
    );
    let lifecycle = world
        .level_mut_by_id(&dimension)
        .entities
        .drain_lifecycle_events();
    assert!(!lifecycle.iter().any(|event| matches!(event,
            crate::entity::EntityLifecycleEvent::Removed { entity, .. } if entity.id() == id)));
    assert!(!lifecycle
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Sound { .. })));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(!game_events
        .iter()
        .any(|event| event.game_event == crate::level_event::VanillaGameEvent::EXPLODE));
}
