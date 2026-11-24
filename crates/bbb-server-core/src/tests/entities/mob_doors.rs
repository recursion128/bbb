use super::*;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_property,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SOURCE: BlockPos = BlockPos { x: 2, y: 65, z: 2 };
const DESTINATION: BlockPos = BlockPos { x: 11, y: 65, z: 2 };
const DOOR: BlockPos = BlockPos { x: 6, y: 65, z: 2 };

fn put(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    world.set_block(DIM, pos, state, SET_BLOCK_UPDATE_ALL);
    assert_eq!(world.block_state(DIM, pos), Some(state));
}

fn default_state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    vanilla_block_default_state_id(block.id).unwrap()
}

fn door_state(name: &str, half: &str) -> i32 {
    let properties = [
        ("facing", "east"),
        ("half", half),
        ("hinge", "left"),
        ("open", "false"),
        ("powered", "false"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
}

fn corridor(world: &mut ServerWorld, door_name: &str) -> BlockPos {
    let stone = block_state_id("minecraft:stone");
    let air = block_state_id("minecraft:air");
    for x in 0..=13 {
        for z in 0..=4 {
            put(world, BlockPos { x, y: 64, z }, stone);
            for y in 65..=68 {
                let wall = x == 0 || x == 13 || z == 0 || z == 4 || x == DOOR.x;
                put(world, BlockPos { x, y, z }, if wall { stone } else { air });
            }
        }
    }
    put(world, DOOR, door_state(door_name, "lower"));
    let upper = BlockPos {
        y: DOOR.y + 1,
        ..DOOR
    };
    put(world, upper, door_state(door_name, "upper"));
    upper
}

fn is_open(world: &mut ServerWorld, pos: BlockPos) -> bool {
    vanilla_block_state_property(world.block_state(DIM, pos).unwrap(), "open") == Some("true")
}

fn count(world: &mut ServerWorld, pos: BlockPos) -> i32 {
    world
        .level_block_entity_for_test(DIM, pos)
        .unwrap()
        .container_slots()
        .unwrap()[0]
        .count()
}

#[test]
fn a_copper_golem_transports_through_closed_wood_and_copper_doors_then_closes_them() {
    for door_name in ["minecraft:oak_door", "minecraft:waxed_copper_door"] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.keep_alive_interval_ticks = 10_000;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let upper = corridor(&mut world, door_name);
        put(
            &mut world,
            SOURCE,
            default_state("minecraft:waxed_copper_chest"),
        );
        put(&mut world, DESTINATION, default_state("minecraft:chest"));
        world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 9.5,
                y: 65.0,
                z: 3.5,
            },
        );
        let saved =
            bbb_protocol::parse_snbt_compound(r#"{id:"minecraft:poppy",count:20}"#).unwrap();
        let flowers = RuntimeItemStack::try_read(&saved, &world.item_stack_resolver()).unwrap();
        let mut items = vec![RuntimeItemStack::empty(); 27];
        items[0] = flowers;
        assert!(world
            .level_block_entity_for_test(DIM, SOURCE)
            .unwrap()
            .chest_mut()
            .unwrap()
            .set_items(items));
        let (id, _) = world.spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            Vec3d {
                x: 3.5,
                y: 65.0,
                z: 2.5,
            },
            crate::entity::EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID).unwrap(),
            ),
        );
        assert_eq!(
            vanilla_block_state_property(world.block_state(DIM, DOOR).unwrap(), "open"),
            Some("false")
        );
        let mut opened = false;
        let mut crossed = false;
        let mut closed_after_crossing = false;
        for _ in 0..1400 {
            world.tick(Vec::new());
            let position = world.stored_entity_for_test(DIM, id).unwrap().position();
            crossed |= position.x > 7.0;
            let lower_open =
                vanilla_block_state_property(world.block_state(DIM, DOOR).unwrap(), "open")
                    == Some("true");
            let upper_open =
                vanilla_block_state_property(world.block_state(DIM, upper).unwrap(), "open")
                    == Some("true");
            opened |= lower_open && upper_open;
            closed_after_crossing |= opened && crossed && !lower_open && !upper_open;
            if count(&mut world, DESTINATION) == 16 && closed_after_crossing {
                break;
            }
        }
        assert!(opened && crossed && closed_after_crossing,
            "{door_name}: opened={opened}, crossed={crossed}, closed={closed_after_crossing}; position={:?}",
            world.stored_entity_for_test(DIM, id).unwrap().position());
        assert_eq!(count(&mut world, SOURCE), 4, "{door_name}");
        assert_eq!(count(&mut world, DESTINATION), 16, "{door_name}");
    }
}

#[test]
fn an_unemployed_villager_opens_and_closes_the_door_to_its_job_site() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 10_000;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    console_command(&sender, &receiver, &mut world, 2, "time set noon");
    console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "gamerule advance_time false",
    );
    let upper = corridor(&mut world, "minecraft:oak_door");
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 9.5,
            y: 65.0,
            z: 3.5,
        },
    );
    put(&mut world, DESTINATION, default_state("minecraft:lectern"));
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 2.5,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_VILLAGER_ID).unwrap(),
        ),
    );
    let (mut opened, mut crossed, mut closed_after_crossing) = (false, false, false);
    for _ in 0..1200 {
        world.tick(receiver.drain());
        crossed |= world.stored_entity_for_test(DIM, id).unwrap().position().x > 7.0;
        let open = is_open(&mut world, DOOR);
        assert_eq!(open, is_open(&mut world, upper));
        opened |= open;
        closed_after_crossing |= opened && crossed && !open;
        if closed_after_crossing {
            break;
        }
    }
    assert!(
        opened && crossed && closed_after_crossing,
        "opened={opened}, crossed={crossed}, closed={closed_after_crossing}; position={:?}",
        world.stored_entity_for_test(DIM, id).unwrap().position()
    );
}
