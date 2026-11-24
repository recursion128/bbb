use super::*;
use bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_CARRY_STATE_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 40, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn carried(world: &ServerWorld, id: i32) -> Option<NbtSummaryValue> {
    let data = world.stored_entity_command_data_for_test(DIM, id).unwrap();
    crate::loot_runtime::nbt_field(&data, "carriedBlockState").cloned()
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn carry_update(tick: &TickResult, id: i32, state: Option<i32>) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SetEntityData(update),
            } if update.id == id && update.values.iter().any(|row|
                row.data_id == VANILLA_ENDERMAN_CARRY_STATE_DATA_ID
                && row.value == EntityDataValueKind::OptionalBlockState(state)))
    })
}

fn cage_positions() -> impl Iterator<Item = BlockPos> {
    (6..=10)
        .flat_map(|x| (100..=103).flat_map(move |y| (6..=10).map(move |z| BlockPos { x, y, z })))
}

#[test]
fn enderman_takes_a_real_block_saves_its_carry_and_places_it_after_reload() {
    for resource in ["minecraft:dirt", "minecraft:brown_mushroom"] {
        carry_case(resource);
    }
}

fn carry_case(resource: &str) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 100_000;
    let root = std::env::temp_dir().join(format!("bbb-enderman-blocks-{}", uuid::Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Enderman blocks");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule random_tick_speed 0",
        "time set midnight",
        "weather clear",
        "gamemode spectator observer",
        "tp observer 8.5 105 8.5",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let top = if resource == "minecraft:dirt" {
        103
    } else {
        100
    };
    let support_resource = if resource == "minecraft:brown_mushroom" {
        "minecraft:mycelium"
    } else {
        "minecraft:stone"
    };
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("fill 5 99 5 11 99 11 {support_resource}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("fill 6 100 6 10 {top} 10 {resource}"),
    );
    for text in [
        "fill 7 100 7 9 104 9 minecraft:air",
        "summon minecraft:enderman 8.5 100 8.5 {PersistenceRequired:1b}",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let id = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_ENDERMAN_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
    let initial_health = health(&world, id);
    // A real attribute command limits wandering without disabling either block goal.
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    assert!(world
        .player_tracking_viewers_for_test(DIM, id)
        .contains(&ConnectionId(1)));
    let state = block_state_id(resource);
    let air = block_state_id("minecraft:air");
    let support_block =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(support_resource).unwrap();
    let support =
        bbb_protocol::block_state_types::vanilla_block_default_state_id(support_block.id).unwrap();
    let original_blocks: Vec<_> = cage_positions()
        .filter(|pos| world.block_state(DIM, *pos) == Some(state))
        .collect();
    assert!(!original_blocks.is_empty());
    let mut picked_up = false;
    let mut pickup_metadata = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..4000 {
        let tick = world.tick(receiver.drain());
        pickup_metadata |= carry_update(&tick, id, Some(state));
        picked_up = carried(&world, id).is_some();
        if picked_up && pickup_metadata {
            break;
        }
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(
        picked_up && pickup_metadata,
        "{resource}: natural pickup and tracking metadata: picked_up={picked_up}, pickup_metadata={pickup_metadata}"
    );
    let carried_block = carried(&world, id).unwrap();
    assert_eq!(
        crate::loot_runtime::nbt_field(&carried_block, "Name"),
        Some(&NbtSummaryValue::String(resource.into()))
    );
    let holes: Vec<_> = original_blocks
        .iter()
        .copied()
        .filter(|pos| world.block_state(DIM, *pos) != Some(state))
        .collect();
    assert_eq!(
        holes.len(),
        1,
        "{resource}: taking removes exactly one physical block"
    );
    let hole = holes[0];
    assert_eq!(world.block_state(DIM, hole), Some(air));
    assert_eq!(health(&world, id), initial_health);
    let request = world
        .save_request(1_786_000_020_000, 220, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(config, &candidate);
    restored.tick_rate_mut().set_frozen(true);
    let id = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(
        restored.stored_entity_for_test(DIM, id).unwrap().uuid(),
        uuid
    );
    assert_eq!(carried(&restored, id), Some(carried_block));
    assert_eq!(health(&restored, id), initial_health);
    assert_eq!(restored.block_state(DIM, hole), Some(air));
    let (joined, loaded) = join_loaded_ticks(&sender, &receiver, &mut restored, 1, "observer", 1);
    assert!(carry_update(&joined, id, Some(state)) || carry_update(&loaded, id, Some(state)));
    assert!(restored
        .player_tracking_viewers_for_test(DIM, id)
        .contains(&ConnectionId(1)));
    command(
        &sender,
        &receiver,
        &mut restored,
        &format!("fill 6 100 6 10 103 10 minecraft:air replace {resource}"),
    );
    assert!(cage_positions().all(|pos| restored.block_state(DIM, pos) == Some(air)));
    let mut placed = false;
    let mut clear_metadata = false;
    restored.tick_rate_mut().set_frozen(false);
    for _ in 0..60_000 {
        let tick = restored.tick(receiver.drain());
        clear_metadata |= carry_update(&tick, id, None);
        placed = carried(&restored, id).is_none();
        if placed && clear_metadata {
            break;
        }
    }
    restored.tick_rate_mut().set_frozen(true);
    assert!(
        placed && clear_metadata,
        "{resource}: natural placement clears tracking metadata: placed={placed}, clear_metadata={clear_metadata}"
    );
    let placed_blocks: Vec<_> = cage_positions()
        .filter(|pos| restored.block_state(DIM, *pos) == Some(state))
        .collect();
    assert_eq!(
        placed_blocks.len(),
        1,
        "the carried block becomes one real world block"
    );
    let pos = placed_blocks[0];
    assert_eq!(pos.y, 100);
    assert!((7..=9).contains(&pos.x) && (7..=9).contains(&pos.z));
    assert_eq!(
        restored.block_state(DIM, BlockPos { y: 99, ..pos }),
        Some(support)
    );
    assert_eq!(carried(&restored, id), None);
    assert_eq!(health(&restored, id), initial_health);
    assert_eq!(
        restored.stored_entity_for_test(DIM, id).unwrap().uuid(),
        uuid
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
