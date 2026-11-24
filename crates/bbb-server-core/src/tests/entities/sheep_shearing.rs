//! `Sheep.mobInteract`'s shearing branch on the real packet path.

use super::*;

fn shearing_location_check_config() -> ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(
        config
            .datapacks
            .as_mut()
            .expect("the test config carries the built-in datapack"),
    );
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .expect("the datapack carries the loot-table registry");
    let element = registry
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:shearing/sheep")
        .expect("the built-in datapack carries the sheep shearing table");
    let bbb_server_datapack::loot::LootElementValue::Table(table) = &mut element.value else {
        panic!("the sheep shearing element is a table")
    };
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    table.pools[0].conditions =
        vec![
            bbb_server_datapack::loot::parse_loot_condition(&serde_json::json!({
                "condition": "minecraft:location_check",
                "predicate": { "block": { "blocks": "minecraft:air" } }
            }), None)
            .expect("the test location condition is valid"),
            bbb_server_datapack::loot::parse_loot_condition(&serde_json::json!({
                "condition": "minecraft:location_check",
                "offsetX": 1,
                "predicate": { "block": {
                    "blocks": "minecraft:chest",
                    "nbt": format!(
                        "{{id:\"minecraft:chest\",x:{},y:{y},z:{},Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
                        x + 1, z + 1,
                    )
                } }
            }), None)
            .expect("the test block-entity condition is valid"),
        ];
    config
}

pub(super) fn enchant_for_entity_test(
    world: &ServerWorld,
    stack: &mut ItemStackSummary,
    enchantment: &str,
    level: i32,
) {
    let holder_id = world
        .datapacks()
        .expect("the entity test world has datapacks")
        .registries
        .registry("minecraft:enchantment")
        .expect("the datapack carries the enchantment registry")
        .elements
        .iter()
        .position(|entry| entry.id == enchantment)
        .unwrap_or_else(|| panic!("the datapack carries {enchantment}"));
    stack.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: i32::try_from(holder_id).unwrap(),
        level,
    }];
    let type_id = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
    if !stack.component_patch.added_type_ids.contains(&type_id) {
        stack.component_patch.added_type_ids.push(type_id);
    }
}

#[test]
fn live_sheep_shearing_drops_wool_then_emits_shear_and_damages_the_tool() {
    let (sender, receiver) = mailbox();
    let mut config = shearing_location_check_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x5eea);
    let player_position = world.publication().players[0].position;
    let setup = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!(
            "setblock {} {} {} minecraft:chest{{Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
            player_position.x.floor() as i32 + 1,
            player_position.y.floor() as i32,
            player_position.z.floor() as i32 + 1,
        ),
    );
    assert_eq!(console_command_answer(&setup).0, Some(1));
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let (sheep_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        sheep_type,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(sheep_type).unwrap(),
        ),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    let before_events = world.game_events(Java26BuiltinDimension::Overworld).1;

    let sheared = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: sheep_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );

    let sheep = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, sheep_id)
        .expect("the sheep remains live");
    assert!(matches!(
        sheep.data(),
        crate::entity::EntityData::GeneralLiving(data)
            if data.sheep_is_sheared() == Some(true)
    ));
    let wool = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            item.item().item_id()
        })
        .collect::<Vec<_>>();
    assert!(!wool.is_empty());
    assert!(wool.iter().all(|id| {
        bbb_protocol::item_types::vanilla_item_type(*id)
            .is_some_and(|item| item.resource_id == "minecraft:white_wool")
    }));
    let held = &sheared.snapshot.players[0].inventory.slots[0];
    assert_eq!(held.component_patch.damage, Some(1));
    assert!(sheared.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } if sound.source == SoundSource::Players
            && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(registry_id)
                    == Some("minecraft:entity.sheep.shear"))
    )));
    assert!(sheared.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityAnimation(animation),
            ..
        } if animation.id == world.player_entity_id_for_test(ConnectionId(1))
            && animation.action == 0
    )));
    let (game_events, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, before_events + 2);
    assert!(game_events.iter().any(|event| {
        event.game_event == VanillaGameEvent::ENTITY_INTERACT
            && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))
            && event.pos == sheep.block_position()
    }));
    assert!(game_events.iter().any(|event| {
        event.game_event == VanillaGameEvent::SHEAR
            && event.source_entity_id == Some(world.player_entity_id_for_test(ConnectionId(1)))
            && event.pos == sheep.block_position()
    }));
}

#[test]
fn live_creative_sheep_shearing_keeps_the_tool_undamaged() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x5eed);
    let player_position = world.publication().players[0].position;
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let (sheep_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        sheep_type,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(sheep_type).unwrap(),
        ),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );

    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: sheep_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );

    assert_eq!(
        result.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage,
        None
    );
    assert!(matches!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, sheep_id)
            .expect("the sheep remains live")
            .data(),
        crate::entity::EntityData::GeneralLiving(data)
            if data.sheep_is_sheared() == Some(true)
    ));
}

#[test]
fn live_sheep_shearing_uses_level_random_for_unbreaking() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x5eeb);
    let player_position = world.publication().players[0].position;
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let sheep_ids = (0..8)
        .map(|_| {
            world
                .spawn_stored_entity_for_test(
                    Java26BuiltinDimension::Overworld,
                    sheep_type,
                    Vec3d {
                        z: player_position.z + 1.0,
                        ..player_position
                    },
                    crate::entity::EntityData::GeneralLiving(
                        crate::entity::GeneralLivingData::new(sheep_type).unwrap(),
                    ),
                )
                .0
        })
        .collect::<Vec<_>>();
    let mut shears = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    enchant_for_entity_test(&world, &mut shears, "minecraft:unbreaking", 3);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, shears);

    let mut damage = 0;
    for sheep_id in sheep_ids {
        let result = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: sheep_id,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
        damage = result.snapshot.players[0].inventory.slots[0]
            .component_patch
            .damage
            .unwrap_or(0);
    }
    assert!(
        damage > 0 && damage < 8,
        "level-three unbreaking reduced eight durability attempts to {damage}"
    );
}

#[test]
fn live_sheep_shearing_breaks_the_last_point_and_awards_the_stat() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x5eec);
    let player_position = world.publication().players[0].position;
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let (sheep_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        sheep_type,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(sheep_type).unwrap(),
        ),
    );
    let shears_id = bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID;
    let max_damage = crate::inventory::item_prototype_int_component(
        shears_id,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    )
    .expect("shears are damageable");
    let mut patch = DataComponentPatchSummary {
        damage: Some(max_damage - 1),
        ..DataComponentPatchSummary::default()
    };
    patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(shears_id),
            count: 1,
            component_patch: patch.into(),
        },
    );

    let result = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: sheep_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );

    assert_eq!(
        result.snapshot.players[0].inventory.slots[0],
        ItemStackSummary::empty()
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
                shears_id,
            ),
        ),
        1
    );
}
