use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_COPPER_GOLEM_STATE_DATA_ID, VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_server_types::item_stack::RuntimeItemStack;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SOURCE: BlockPos = BlockPos { x: 2, y: 65, z: 2 };
const DESTINATION: BlockPos = BlockPos { x: 10, y: 65, z: 2 };

fn chest(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    world.set_block(DIM, pos, state, SET_BLOCK_UPDATE_ALL);
    assert_eq!(world.block_state(DIM, pos), Some(state));
}

fn stack(world: &mut ServerWorld, pos: BlockPos, slot: usize) -> RuntimeItemStack {
    world
        .level_block_entity_for_test(DIM, pos)
        .unwrap()
        .container_slots()
        .unwrap()[slot]
        .clone()
}

fn flowers(world: &ServerWorld, count: i32, different: bool) -> RuntimeItemStack {
    let saved = bbb_protocol::parse_snbt_compound(if different {
        r#"{id:"minecraft:poppy",count:1,components:{"minecraft:custom_data":{kept:18}}}"#
    } else {
        r#"{id:"minecraft:poppy",count:1,components:{"minecraft:custom_data":{kept:17}}}"#
    })
    .unwrap();
    RuntimeItemStack::try_read(&saved, &world.item_stack_resolver())
        .unwrap()
        .with_count(count)
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.keep_alive_interval_ticks = 10_000;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stone = block_state_id("minecraft:stone");
    let air = block_state_id("minecraft:air");
    for x in 0..=14 {
        for z in 0..=6 {
            let floor = BlockPos { x, y: 64, z };
            world.set_block(DIM, floor, stone, SET_BLOCK_UPDATE_ALL);
            assert_eq!(world.block_state(DIM, floor), Some(stone));
            for y in 65..=68 {
                let pos = BlockPos { x, y, z };
                world.set_block(DIM, pos, air, SET_BLOCK_UPDATE_ALL);
                assert_eq!(world.block_state(DIM, pos), Some(air));
            }
        }
    }
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 5.5,
        },
    );
    chest(&mut world, SOURCE, "minecraft:waxed_copper_chest");
    chest(&mut world, DESTINATION, "minecraft:chest");
    let kind = VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        Vec3d {
            x: 1.5,
            y: 65.0,
            z: 2.5,
        },
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    (sender, receiver, world, id)
}

#[test]
fn a_copper_golem_carries_sixteen_native_items_and_deposits_them_in_a_real_chest() {
    let (sender, receiver, mut world, id) = fixture();
    let original_stack = flowers(&world, 20, false);
    let original = original_stack.summary().clone();
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[3] = original_stack;
    assert!(world
        .level_block_entity_for_test(DIM, SOURCE)
        .unwrap()
        .chest_mut()
        .unwrap()
        .set_items(items));
    let mut picked_up = false;
    let mut travelled = false;
    let mut getting = false;
    let mut dropping = false;
    let mut open_ticks = 0;
    for _ in 0..800 {
        let tick = world.tick(Vec::new());
        for event in &tick.events {
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } = event
            {
                if update.id == id {
                    for value in &update.values {
                        if value.data_id == VANILLA_COPPER_GOLEM_STATE_DATA_ID {
                            if let EntityDataValueKind::EnumId { id, .. } = value.value {
                                getting |= id == VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID;
                                dropping |= id == VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID;
                            }
                        }
                    }
                }
            }
        }
        let body = world
            .stored_entity_for_test(DIM, id)
            .expect("the golem stays alive");
        travelled |= body.position().x > 7.0;
        let held = body
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand);
        if !picked_up && held.is_some_and(|stack| stack.count() == 16) {
            let held = held.unwrap().clone();
            assert_eq!(
                held.summary().component_patch.custom_data,
                original.component_patch.custom_data
            );
            assert_eq!(stack(&mut world, SOURCE, 3).count(), 4);
            assert_eq!(
                saved_item_component(&world, &held, "minecraft:custom_data"),
                original.component_patch.custom_data.clone(),
            );
            picked_up = true;
        }
        if getting && !picked_up {
            open_ticks += i32::from(
                world
                    .level_block_entity_for_test(DIM, SOURCE)
                    .unwrap()
                    .chest()
                    .unwrap()
                    .open_count()
                    > 0,
            );
        }
        if stack(&mut world, DESTINATION, 0).count() == 16 {
            break;
        }
    }
    assert!(picked_up && travelled && getting && dropping && open_ticks > 10,
        "pickup={picked_up}, travel={travelled}, get={getting}, drop={dropping}, open_ticks={open_ticks}");
    let deposited = stack(&mut world, DESTINATION, 0);
    assert_eq!(deposited.count(), 16);
    assert_eq!(
        deposited.summary().component_patch.custom_data,
        original.component_patch.custom_data
    );
    assert_eq!(stack(&mut world, SOURCE, 3).count(), 4);
    world.tick_rate_mut().set_frozen(true);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 10.5,
            y: 65.0,
            z: 3.5,
        },
    );
    let opened = right_click(&sender, &receiver, &mut world, 1, DESTINATION, 1);
    let content = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetContent(content),
                ..
            } if content.container_id != 0 => Some(content),
            _ => None,
        })
        .expect("the actual destination chest opens its native menu");
    assert_eq!(content.items[0], deposited.summary().clone());
    let entity = world.level_block_entity_for_test(DIM, DESTINATION).unwrap();
    let record = save_native_fixture_block_entity(entity);
    let mut restored = load_native_fixture_block_entity(DESTINATION, entity.block_state(), &record);
    assert_eq!(
        restored.randomizable_container_mut().unwrap().1[0],
        deposited
    );
}

#[test]
fn a_sitting_cat_blocks_transport_then_the_golem_merges_only_matching_components() {
    let (_sender, _receiver, mut world, id) = fixture();
    let original = flowers(&world, 20, false);
    let different = flowers(&world, 4, true);
    let matching = flowers(&world, 63, false);
    let mut source = vec![RuntimeItemStack::empty(); 27];
    source[0] = original.clone();
    assert!(world
        .level_block_entity_for_test(DIM, SOURCE)
        .unwrap()
        .chest_mut()
        .unwrap()
        .set_items(source));
    let mut destination = vec![RuntimeItemStack::empty(); 27];
    destination[0] = different.clone();
    destination[1] = matching;
    assert!(world
        .level_block_entity_for_test(DIM, DESTINATION)
        .unwrap()
        .chest_mut()
        .unwrap()
        .set_items(destination));
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
    let (cat, _) = world.spawn_stored_entity_for_test(
        DIM,
        kind,
        Vec3d {
            x: 2.5,
            y: 65.875,
            z: 2.5,
        },
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
    );
    let mut record = super::horse_interaction::record(&world, cat);
    for (name, value) in [
        ("NoAI", NbtSummaryValue::Byte(1)),
        ("Sitting", NbtSummaryValue::Byte(1)),
    ] {
        record.retain(|entry| entry.name != name);
        record.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
    world.load_stored_entity_record_for_test(DIM, cat, &record);
    for _ in 0..240 {
        world.tick(Vec::new());
    }
    assert_eq!(stack(&mut world, SOURCE, 0), original);
    assert!(world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::MainHand)
        .is_none());
    assert_eq!(stack(&mut world, DESTINATION, 0), different);
    assert!(world.remove_stored_entity_for_test(DIM, cat, crate::entity::RemovalReason::Discarded,));
    for _ in 0..1000 {
        world.tick(Vec::new());
        if stack(&mut world, DESTINATION, 2).count() == 15 {
            break;
        }
    }
    assert_eq!(stack(&mut world, SOURCE, 0), original.with_count(4));
    assert_eq!(stack(&mut world, DESTINATION, 0), different);
    assert_eq!(stack(&mut world, DESTINATION, 1), original.with_count(64));
    assert_eq!(stack(&mut world, DESTINATION, 2), original.with_count(15));
}

/// `CopperGolem.actuallyHurt` (`CopperGolem.java:448-451`) drops the golem's
/// chest animation.
#[test]
fn a_hurt_copper_golem_returns_to_idle() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 4.5,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID).unwrap(),
        ),
    );
    let golem = world.stored_entity_mut_for_test(DIM, id).unwrap();
    crate::entity::copper_golem::set_animation_state(
        golem,
        crate::entity::copper_golem::AnimationState::GettingItem,
    );
    let outcome = world
        .hurt_stored_entity_by_command_for_test(DIM, id, "minecraft:generic", 1.0, None)
        .unwrap();
    assert!(outcome.succeeded);
    let crate::entity::EntityData::GeneralLiving(data) =
        world.stored_entity_for_test(DIM, id).unwrap().data()
    else {
        unreachable!()
    };
    assert_eq!(
        data.copper_golem_animation(),
        Some(crate::entity::copper_golem::AnimationState::Idle)
    );
}
