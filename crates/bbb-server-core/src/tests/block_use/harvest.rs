//! Picking from a block: pumpkins, redstone ore, sweet berries and cave vines.

use super::*;

/// `PumpkinBlock.useItemOn` (`PumpkinBlock.java:38-74`) shears a pumpkin into
/// a carved pumpkin and drops `minecraft:carve/pumpkin`.
#[test]
fn shears_use_carves_a_pumpkin_and_drops_seeds() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let table = startup
        .loot
        .registries
        .iter_mut()
        .flat_map(|registry| &mut registry.elements)
        .find(|entry| entry.id == "minecraft:carve/pumpkin")
        .unwrap();
    let bbb_server_datapack::loot::LootElementValue::Table(table) = &mut table.value else {
        panic!("a block interaction loot table");
    };
    let condition = bbb_server_datapack::loot::parse_loot_condition(&serde_json::json!({
        "condition":"minecraft:entity_properties","entity":"interacting_entity",
        "predicate":{"type":"minecraft:player","location":{"block":{
            "nbt":"{id:'minecraft:chest',x:0,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"
        }}}
    }), None).unwrap();
    for pool in &mut table.pools {
        pool.conditions.push(condition.clone());
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let setup = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        0,
        "setblock 0 64 0 chest{Items:[{Slot:0b,id:\"minecraft:diamond\",count:3}]}",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&setup).0,
        Some(1)
    );
    let at = BlockPos { x: 0, y: 64, z: 3 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        pumpkin_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:shears");
    let carved = click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(carved_pumpkin_state("north"))
    );
    assert!(
        sounds_sent(&carved).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.pumpkin.carve")
                    .expect("a registered sound"),
            }),
        "{carved:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::SHEAR
                && event.pos == at
                && event.source_entity_id == Some(entity_id_of(&world, 1))
        }),
        "{events:?}"
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert_eq!(entities.len(), 1, "{entities:?}");
    let NbtSummaryValue::Compound(stack) = field(&entities[0], "Item") else {
        panic!("Item is a compound");
    };
    assert_eq!(
        field(stack, "id"),
        &NbtSummaryValue::string("minecraft:pumpkin_seeds")
    );
    assert_eq!(field(stack, "count"), &NbtSummaryValue::Int(4));
    let pos = doubles(field(&entities[0], "Pos"));
    assert_eq!(pos.len(), 3, "{pos:?}");
    assert!(
        (pos[0] - 0.5).abs() < 0.15 && (pos[1] - 64.1).abs() < 0.15 && (pos[2] - 2.85).abs() < 0.15,
        "{pos:?}"
    );
    let stats = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    assert!(stats.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AwardStats(update),
            ..
        } if update.stats.iter().any(|entry|
            entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                && entry.value_id == bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID
                && entry.amount == 1)
    )));
}

/// A horizontal click face becomes `CarvedPumpkinBlock.FACING` (`:50`).
#[test]
fn shears_use_faces_the_clicked_side_of_a_pumpkin() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        pumpkin_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:shears");
    let _ = click_face(&sender, &receiver, &mut world, at, Direction::East, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(carved_pumpkin_state("east"))
    );
}

/// Anything but shears falls to `super.useItemOn` (`:46-47`).
#[test]
fn a_block_item_use_does_not_carve_a_pumpkin() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let beside = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        pumpkin_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:stone");
    let _ = click_face(&sender, &receiver, &mut world, at, Direction::East, 1);
    assert_eq!(live_block(&mut world, at), Some(pumpkin_state()));
    assert_eq!(
        live_block(&mut world, beside),
        Some(block_state_id("minecraft:stone"))
    );
}

/// `RedStoneOreBlock.interact` lights with flags 3 (`RedStoneOreBlock.java:76-77`).
#[test]
fn item_use_lights_unlit_redstone_ore() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        redstone_ore_state("minecraft:redstone_ore", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:shears");
    let _ = click_face(&sender, &receiver, &mut world, at, Direction::Up, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(redstone_ore_state("minecraft:redstone_ore", true))
    );
}

/// A held `BlockItem` whose `canPlace` is true PASSes (`:69-71`).
#[test]
fn block_item_use_on_redstone_ore_lights_it_and_places_beside() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let beside = BlockPos { x: 1, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        redstone_ore_state("minecraft:deepslate_redstone_ore", false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:stone");
    let _ = click_face(&sender, &receiver, &mut world, at, Direction::East, 1);
    assert_eq!(
        live_block(&mut world, at),
        Some(redstone_ore_state("minecraft:deepslate_redstone_ore", true))
    );
    assert_eq!(
        live_block(&mut world, beside),
        Some(block_state_id("minecraft:stone"))
    );
}

/// `SweetBerryBushBlock.useItemOn` PASSes bone meal when age != 3
/// (`:110-112`), so the bone meal grows the bush instead of picking it.
#[test]
fn bone_meal_use_on_a_growing_sweet_berry_bush_grows_it() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        sweet_berry_state(2),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:bone_meal");
    let _ = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(sweet_berry_state(3)));
}

/// Age 3 falls to `super.useItemOn` (`:112`), then `useWithoutItem` picks.
#[test]
fn bone_meal_use_on_a_ripe_sweet_berry_bush_picks() {
    empty_hand_picks_sweet_berries(3, Some("minecraft:bone_meal"));
}

#[test]
fn empty_hand_use_picks_age_2_sweet_berries() {
    empty_hand_picks_sweet_berries(2, None);
}

#[test]
fn empty_hand_use_picks_age_3_sweet_berries() {
    empty_hand_picks_sweet_berries(3, None);
}

#[test]
fn empty_hand_use_does_not_pick_age_0_sweet_berries() {
    empty_hand_leaves_unripe_sweet_berries(0);
}

#[test]
fn empty_hand_use_does_not_pick_age_1_sweet_berries() {
    empty_hand_leaves_unripe_sweet_berries(1);
}

/// Head harvest writes berries false with flags 2 (`CaveVines.java:37-38`).
#[test]
fn empty_hand_use_picks_cave_vine_head_berries() {
    empty_hand_picks_cave_vines(cave_vines_head(3, true), cave_vines_head(3, false));
}

/// Plant harvest is the same `CaveVines.use` (`CaveVinesPlantBlock.java:49-52`).
#[test]
fn empty_hand_use_picks_cave_vine_plant_berries() {
    empty_hand_picks_cave_vines(cave_vines_plant(true), cave_vines_plant(false));
}

#[test]
fn empty_hand_use_does_not_pick_cave_vine_head_without_berries() {
    empty_hand_leaves_cave_vines(cave_vines_head(3, false));
}

#[test]
fn empty_hand_use_does_not_pick_cave_vine_plant_without_berries() {
    empty_hand_leaves_cave_vines(cave_vines_plant(false));
}
