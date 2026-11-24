use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer, restore_entity_writes};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    static NEXT_OPERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(20);
    let operation = NEXT_OPERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tick = console_command(sender, receiver, world, operation, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
}

fn tick(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    let tick = world.tick(receiver.drain());
    for event in tick.events {
        if let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ALICE,
                    packet: PlayServerbound::KeepAlive { id },
                },
            )
            .unwrap();
        }
    }
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn in_love(world: &ServerWorld, id: i32) -> i32 {
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, id)
    else {
        panic!("entity record");
    };
    match fields
        .iter()
        .find(|field| field.name == "InLove")
        .map(|field| &field.value)
    {
        Some(NbtSummaryValue::Int(value)) => *value,
        other => panic!("InLove: {other:?}"),
    }
}

fn feed(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    id: i32,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
}

fn prepare() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    let (id, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        Vec3d {
            x: 12.5,
            ..PLAYER_POSITION
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{IsImmuneToZombification:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:attack_damage base set 1"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "attribute alice minecraft:knockback_resistance base set 1",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "gamerule natural_health_regeneration false",
    );
    command(&sender, &receiver, &mut world, "team add no_collision");
    command(
        &sender,
        &receiver,
        &mut world,
        "team modify no_collision collisionRule never",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("team join no_collision {uuid}"),
    );
    for x in -16..=31 {
        for z in -16..=31 {
            world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    let config = datapack_config();
    let tags = config
        .datapacks
        .as_ref()
        .unwrap()
        .tags
        .registries
        .iter()
        .flat_map(|registry| {
            registry.tags.iter().map(|(name, members)| {
                let mut members = members.clone();
                if registry.registry_id == "minecraft:block"
                    && name == "minecraft:hoglin_repellents"
                {
                    members.push("minecraft:dirt".into());
                }
                (registry.registry_id.clone(), name.clone(), members)
            })
        });
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new(
        tags,
    )));
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                    "minecraft:crimson_fungus",
                )
                .unwrap()
                .id,
            ),
            count: 4,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    world.tick_rate_mut().set_frozen(false);
    (sender, receiver, world, id, uuid)
}

fn block(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    assert!(world.set_block(DIMENSION, pos, block_state_id(name), SET_BLOCK_UPDATE_ALL));
}

#[test]
fn tagged_repellent_stops_attacks_flees_and_pacifies_for_effective_ai_ticks() {
    let (sender, receiver, mut world, hoglin, uuid) = prepare();
    let before = health(&world);
    tick_until(&mut world, 180, |world| health(world) < before);
    assert!(
        position(&world, hoglin).x < 11.5,
        "the hoglin approached Alice"
    );
    let repellent = BlockPos { x: 7, y: 65, z: 8 };
    block(&mut world, repellent, "minecraft:dirt");
    tick_until(&mut world, 25, |world| target_of(world, hoglin).is_none());
    let pacified_health = health(&world);
    let start = position(&world, hoglin);
    for _ in 0..50 {
        tick(&sender, &receiver, &mut world);
    }
    let fled = position(&world, hoglin);
    let distance = |pos: Vec3d| ((pos.x - 7.5).powi(2) + (pos.z - 8.5).powi(2)).sqrt();
    assert!(
        distance(fled) > distance(start) + 3.0,
        "hoglin flees the repellent: {start:?} -> {fled:?}"
    );
    assert_eq!(health(&world), pacified_health);
    block(&mut world, repellent, "minecraft:air");
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 10.5 65 8.5"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..240 {
        tick(&sender, &receiver, &mut world);
    }
    feed(&sender, &receiver, &mut world, hoglin);
    assert_eq!(
        in_love(&world, hoglin),
        0,
        "NoAI pauses pacification and feeding remains blocked"
    );

    let request = world
        .save_request(1_786_000_008_000, 212, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    install_forced_ticket(&mut candidate);
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    restored.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    assert!(
        restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .is_some(),
        "a pacified hoglin writes a readable save"
    );
    drop(restored);

    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:attack_damage base set 1"),
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..130 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(health(&world), pacified_health);
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 10.5 65 8.5"),
    );
    feed(&sender, &receiver, &mut world, hoglin);
    assert_eq!(
        in_love(&world, hoglin),
        0,
        "180 effective ticks still blocks love"
    );
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..30 {
        tick(&sender, &receiver, &mut world);
    }
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 10.5 65 8.5"),
    );
    feed(&sender, &receiver, &mut world, hoglin);
    assert!(
        in_love(&world, hoglin) > 0,
        "210 effective ticks permits love again"
    );
    world.tick_rate_mut().set_frozen(false);
    tick_until(&mut world, 180, |world| health(world) < pacified_health);
}

#[test]
fn living_damage_clears_pacification_even_with_no_ai() {
    let (sender, receiver, mut world, hoglin, uuid) = prepare();
    let repellent = BlockPos { x: 11, y: 65, z: 8 };
    block(&mut world, repellent, "minecraft:dirt");
    tick(&sender, &receiver, &mut world);
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("tp {uuid} 10.5 65 8.5"),
    );
    block(&mut world, repellent, "minecraft:air");
    feed(&sender, &receiver, &mut world, hoglin);
    assert_eq!(in_love(&world, hoglin), 0);
    let before = world
        .stored_entity_for_test(DIMENSION, hoglin)
        .unwrap()
        .living()
        .unwrap()
        .health();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    assert!(
        world
            .stored_entity_for_test(DIMENSION, hoglin)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < before
    );
    feed(&sender, &receiver, &mut world, hoglin);
    assert!(
        in_love(&world, hoglin) > 0,
        "living damage allows love again"
    );
}
