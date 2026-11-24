use super::*;

use crate::entity::{EntityIdCounter, EntityStore, FurnaceMinecartData, MinecartData};
use bbb_protocol::entity_synched_data::VANILLA_FURNACE_MINECART_FUEL_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CHARCOAL_ID, VANILLA_ITEM_COAL_ID, VANILLA_ITEM_STICK_ID,
};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn interact(id: i32, hand: InteractionHand, secondary: bool) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id: id,
        hand,
        location: Vec3d::default(),
        using_secondary_action: secondary,
    })
}

fn spawn_cart(world: &mut ServerWorld, position: Vec3d, furnace: bool) -> i32 {
    let (kind, data) = if furnace {
        (
            VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
            EntityData::FurnaceMinecart(FurnaceMinecartData::new()),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_MINECART_ID,
            EntityData::Minecart(MinecartData::new()),
        )
    };
    world
        .spawn_stored_entity_for_test(DIMENSION, kind, position, data)
        .0
}

fn fuel_data(world: &ServerWorld, id: i32) -> FurnaceMinecartData {
    match world.stored_entity_for_test(DIMENSION, id).unwrap().data() {
        EntityData::FurnaceMinecart(data) => *data,
        other => panic!("expected furnace minecart, got {other:?}"),
    }
}

fn give(world: &mut ServerWorld, hand: InteractionHand, item: i32, count: i32) {
    *world
        .player_inventory_for_test(ConnectionId(1))
        .item_in_hand_mut(hand) = native_stack(ItemStackSummary {
        item_id: Some(item),
        count,
        component_patch: Default::default(),
    });
}

fn count(world: &mut ServerWorld, hand: InteractionHand) -> i32 {
    world
        .player_inventory_for_test(ConnectionId(1))
        .item_in_hand_mut(hand)
        .count()
}

#[test]
fn minecart_interaction_mounts_empty_cart_and_shift_dismounts_without_stealing_a_seat() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick_rate_mut().set_frozen(true);
    let position = world.publication().players[0].position;
    let cart = spawn_cart(&mut world, position, false);

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(cart, InteractionHand::MainHand, true),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None
    );
    let mounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(cart, InteractionHand::MainHand, false),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(cart)
    );
    assert!(mounted.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPassengers(update) }
            if update.vehicle_id == cart && update.passenger_ids == [entity_id_of(&world, 1)]
    )));
    assert!(mounted.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PlayerPosition(_)
        }
    )));
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        interact(cart, InteractionHand::OffHand, false),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        None
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(cart)
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..Default::default()
        }),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None
    );

    let far = spawn_cart(
        &mut world,
        Vec3d {
            x: position.x + 20.0,
            ..position
        },
        false,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        interact(far, InteractionHand::MainHand, false),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        None
    );
    let abilities = world.player_abilities_for_test(ConnectionId(2));
    world.set_player_mode_and_abilities_for_test(ConnectionId(2), GameType::Spectator, abilities);
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        interact(cart, InteractionHand::MainHand, false),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        None
    );
}

#[test]
fn minecart_interaction_fuels_both_hands_drives_tracks_lit_and_round_trips_save() {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    let player = world.publication().players[0].position;
    let rail = BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32 + 1,
    };
    world.set_block(
        DIMENSION,
        rail,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_RAIL_ID,
        )
        .unwrap(),
        SET_BLOCK_UPDATE_ALL,
    );
    let position = Vec3d {
        x: f64::from(rail.x) + 0.5,
        y: f64::from(rail.y) + 0.0625,
        z: f64::from(rail.z) + 0.5,
    };
    let cart = spawn_cart(&mut world, position, true);
    world.apply_level_entity_lifecycle_for_test(DIMENSION, &mut Vec::new());
    give(
        &mut world,
        InteractionHand::MainHand,
        VANILLA_ITEM_COAL_ID,
        2,
    );
    give(
        &mut world,
        InteractionHand::OffHand,
        VANILLA_ITEM_CHARCOAL_ID,
        2,
    );
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact(cart, hand, true),
        );
        assert_eq!(count(&mut world, hand), 1);
    }
    assert_eq!(fuel_data(&world, cart).fuel(), 7_200);
    assert_eq!(fuel_data(&world, cart).push().z, position.z - player.z);
    world.tick_rate_mut().set_frozen(false);
    let mut events = world.tick(receiver.drain()).events;
    events.extend(world.tick(receiver.drain()).events);
    let entity = world.stored_entity_for_test(DIMENSION, cart).unwrap();
    assert!(entity.position().z > position.z);
    assert!(fuel_data(&world, cart).has_fuel());
    assert!(fuel_data(&world, cart).fuel() < 7_200);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
            if update.id == cart && update.values.iter().any(|value|
                value.data_id == VANILLA_FURNACE_MINECART_FUEL_DATA_ID && value.value == EntityDataValueKind::Boolean(true))
    )));

    let before = fuel_data(&world, cart);
    let uuid = entity.uuid();
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut restored = EntityStore::new(99, EntityIdCounter::new());
    for write in request.entity_writes {
        let saved =
            Java26EntityChunkData::parse(write.document, write.chunk_x, write.chunk_z).unwrap();
        crate::entity::save::read_entity_chunk(&mut restored, &saved).unwrap();
    }
    let saved = restored
        .ids()
        .into_iter()
        .filter_map(|id| restored.get(id))
        .find(|entity| entity.uuid() == uuid)
        .unwrap();
    let EntityData::FurnaceMinecart(after) = saved.data() else {
        panic!("saved furnace cart")
    };
    assert_eq!(after.fuel(), before.fuel());
    assert_eq!(after.push(), before.push());
}

#[test]
fn minecart_interaction_rejects_wrong_full_fuel_and_reads_reloaded_tags_with_creative_consumption()
{
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    let player = world.publication().players[0].position;
    let cart = spawn_cart(
        &mut world,
        Vec3d {
            z: player.z + 1.0,
            ..player
        },
        true,
    );
    give(
        &mut world,
        InteractionHand::MainHand,
        VANILLA_ITEM_COAL_ID,
        10,
    );
    for _ in 0..8 {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact(cart, InteractionHand::MainHand, false),
        );
    }
    let full = fuel_data(&world, cart);
    assert_eq!(full.fuel(), 28_800);
    let moved_player = Vec3d {
        x: player.x + 1.0,
        ..player
    };
    world.set_player_position_for_test(ConnectionId(1), moved_player);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(cart, InteractionHand::MainHand, false),
    );
    assert_eq!(count(&mut world, InteractionHand::MainHand), 2);
    assert_eq!(fuel_data(&world, cart).push(), full.push());
    give(
        &mut world,
        InteractionHand::OffHand,
        VANILLA_ITEM_STICK_ID,
        2,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(cart, InteractionHand::OffHand, false),
    );
    assert_eq!(count(&mut world, InteractionHand::OffHand), 2);
    assert_eq!(fuel_data(&world, cart).push(), full.push());

    let fresh = spawn_cart(
        &mut world,
        Vec3d {
            z: player.z + 1.0,
            ..player
        },
        true,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(fresh, InteractionHand::OffHand, false),
    );
    assert_eq!(fuel_data(&world, fresh).fuel(), 0);
    assert_eq!(fuel_data(&world, fresh).push(), Vec3d::default());
    assert_eq!(count(&mut world, InteractionHand::OffHand), 2);
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new([
        (
            "minecraft:item".into(),
            "minecraft:furnace_minecart_fuel".into(),
            vec!["minecraft:stick".into()],
        ),
    ])));
    let mut abilities = world.player_abilities_for_test(ConnectionId(1));
    abilities.instabuild = true;
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Creative, abilities);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(fresh, InteractionHand::OffHand, false),
    );
    assert_eq!(fuel_data(&world, fresh).fuel(), 3_600);
    assert_eq!(count(&mut world, InteractionHand::OffHand), 2);
    world.install_level_registry_tags(Arc::new(
        crate::registry_tags::RegistryTagSnapshot::default(),
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(fresh, InteractionHand::OffHand, false),
    );
    assert_eq!(fuel_data(&world, fresh).fuel(), 3_600);
    assert_eq!(count(&mut world, InteractionHand::OffHand), 2);
}

#[test]
fn a_rider_pressing_forward_pushes_a_stopped_minecart_along_its_rail() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    let player = world.publication().players[0].position;
    let rail = BlockPos {
        x: player.x.floor() as i32,
        y: player.y.floor() as i32,
        z: player.z.floor() as i32 + 1,
    };
    world.set_block(
        DIMENSION,
        rail,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_RAIL_ID,
        )
        .unwrap(),
        SET_BLOCK_UPDATE_ALL,
    );
    let start = Vec3d {
        x: f64::from(rail.x) + 0.5,
        y: f64::from(rail.y) + 0.0625,
        z: f64::from(rail.z) + 0.5,
    };
    let cart = spawn_cart(&mut world, start, false);
    world.apply_level_entity_lifecycle_for_test(DIMENSION, &mut Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(cart, InteractionHand::MainHand, false),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(cart)
    );
    world.tick_rate_mut().set_frozen(false);
    let cart_z = |world: &ServerWorld| {
        world
            .stored_entity_for_test(DIMENSION, cart)
            .unwrap()
            .position()
            .z
    };
    for _ in 0..5 {
        world.tick(receiver.drain());
    }
    let resting = cart_z(&world);
    assert!((resting - start.z).abs() < 1.0e-6);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            forward: true,
            ..Default::default()
        }),
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    assert!((cart_z(&world) - resting).abs() > 0.01);
}
