//! What a block item meets in the cell it would fill: the entities standing
//! there, the soil and light the new block needs, and whether the clicked
//! block yields its own cell.

use super::*;

const BUILDER_AT: Vec3d = Vec3d {
    x: 6.5,
    y: 66.0,
    z: 6.5,
};

fn builder_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("builder", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Creative,
        PlayerAbilities::for_game_type(GameType::Creative),
    );
    place_player(&sender, &receiver, &mut world, 1, BUILDER_AT, None);
    (sender, receiver, world)
}

fn set(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in block {name}"));
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
        .expect("every block has a default state");
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    assert_eq!(live_block(world, pos), Some(state), "{name}");
}

fn block_name(world: &mut ServerWorld, pos: BlockPos) -> Option<&'static str> {
    live_block(world, pos)
        .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .map(|block| block.resource_id)
}

/// `CropBlock.canSurvive` (`CropBlock.java:145-151`) and
/// `MushroomBlock.canSurvive` (`MushroomBlock.java:83-87`) let seeds and
/// mushrooms plant from the hand.
#[test]
fn seeds_and_mushrooms_plant_on_their_soil() {
    let (sender, receiver, mut world) = builder_world();
    let soil = BlockPos { x: 6, y: 65, z: 4 };
    let planted = BlockPos { y: 66, ..soil };
    for (sequence, (item, support, block)) in [
        (
            "minecraft:wheat_seeds",
            "minecraft:farmland",
            "minecraft:wheat",
        ),
        (
            "minecraft:carrot",
            "minecraft:farmland",
            "minecraft:carrots",
        ),
        (
            "minecraft:potato",
            "minecraft:farmland",
            "minecraft:potatoes",
        ),
        (
            "minecraft:beetroot_seeds",
            "minecraft:farmland",
            "minecraft:beetroots",
        ),
        (
            "minecraft:torchflower_seeds",
            "minecraft:farmland",
            "minecraft:torchflower_crop",
        ),
        (
            "minecraft:brown_mushroom",
            "minecraft:mycelium",
            "minecraft:brown_mushroom",
        ),
        (
            "minecraft:red_mushroom",
            "minecraft:podzol",
            "minecraft:red_mushroom",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        set(&mut world, soil, support);
        set(&mut world, planted, "minecraft:air");
        select_placement_item(&sender, &receiver, &mut world, item);
        let sequence = i32::try_from(sequence).unwrap() + 1;
        place_block_item(
            &sender,
            &receiver,
            &mut world,
            soil,
            Direction::Up,
            1.0,
            sequence,
        );
        assert_eq!(block_name(&mut world, planted), Some(block), "{item}");
    }

    // Seeds still need the soil.
    set(&mut world, soil, "minecraft:stone");
    set(&mut world, planted, "minecraft:air");
    select_placement_item(&sender, &receiver, &mut world, "minecraft:wheat_seeds");
    place_block_item(&sender, &receiver, &mut world, soil, Direction::Up, 1.0, 20);
    assert_eq!(block_name(&mut world, planted), Some("minecraft:air"));
}

/// `BlockItem.canPlace` (`BlockItem.java:134-142`) asks
/// `CollisionGetter.isUnobstructed`, which every entity that `blocksBuilding`
/// refuses: the builder itself, a mob and a TNT minecart
/// (`AbstractMinecart.java:102`). An item entity does not.
#[test]
fn block_items_are_refused_in_cells_entities_stand_in() {
    let (sender, receiver, mut world) = builder_world();
    select_placement_item(&sender, &receiver, &mut world, "minecraft:stone");

    // The builder stands in (6, 66, 6).
    let wall = BlockPos { x: 6, y: 66, z: 5 };
    set(&mut world, wall, "minecraft:stone");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        wall,
        Direction::South,
        0.5,
        1,
    );
    assert_eq!(
        block_name(&mut world, BlockPos { z: 6, ..wall }),
        Some("minecraft:air")
    );

    let pig_floor = BlockPos { x: 4, y: 65, z: 4 };
    let cart_floor = BlockPos { x: 8, y: 65, z: 4 };
    let item_floor = BlockPos { x: 6, y: 65, z: 3 };
    for floor in [pig_floor, cart_floor, item_floor] {
        set(&mut world, floor, "minecraft:stone");
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:pig 4.5 66 4.5 {NoAI:1b}",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:tnt_minecart 8.5 66 4.5",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:item 6.5 66 3.5 {Item:{id:\"minecraft:dirt\",count:1},PickupDelay:32767}",
    );
    for (sequence, floor, expected) in [
        (2, pig_floor, "minecraft:air"),
        (3, cart_floor, "minecraft:air"),
        (4, item_floor, "minecraft:stone"),
    ] {
        place_block_item(
            &sender,
            &receiver,
            &mut world,
            floor,
            Direction::Up,
            1.0,
            sequence,
        );
        assert_eq!(
            block_name(&mut world, BlockPos { y: 66, ..floor }),
            Some(expected),
            "{floor:?}"
        );
    }
}

/// `GrowingPlantBodyBlock.canBeReplaced` (`GrowingPlantBodyBlock.java:97-100`)
/// and the base rule for the heads keep these plants in their cell, so a block
/// clicked against one goes next to it.
#[test]
fn a_block_clicked_against_a_growing_plant_goes_next_to_it() {
    let (sender, receiver, mut world) = builder_world();
    select_placement_item(&sender, &receiver, &mut world, "minecraft:stone");
    let plant = BlockPos { x: 6, y: 66, z: 4 };
    let beside = BlockPos { z: 3, ..plant };
    for (sequence, (name, support)) in [
        ("minecraft:kelp", BlockPos { y: 65, ..plant }),
        ("minecraft:kelp_plant", BlockPos { y: 65, ..plant }),
        ("minecraft:twisting_vines", BlockPos { y: 65, ..plant }),
        (
            "minecraft:twisting_vines_plant",
            BlockPos { y: 65, ..plant },
        ),
        ("minecraft:weeping_vines", BlockPos { y: 67, ..plant }),
        ("minecraft:weeping_vines_plant", BlockPos { y: 67, ..plant }),
        ("minecraft:cave_vines", BlockPos { y: 67, ..plant }),
        ("minecraft:cave_vines_plant", BlockPos { y: 67, ..plant }),
    ]
    .into_iter()
    .enumerate()
    {
        set(&mut world, support, "minecraft:stone");
        set(&mut world, plant, name);
        set(&mut world, beside, "minecraft:air");
        let sequence = i32::try_from(sequence).unwrap() + 1;
        place_block_item(
            &sender,
            &receiver,
            &mut world,
            plant,
            Direction::North,
            0.5,
            sequence,
        );
        assert_eq!(block_name(&mut world, plant), Some(name));
        assert_eq!(
            block_name(&mut world, beside),
            Some("minecraft:stone"),
            "{name}"
        );
        set(&mut world, support, "minecraft:air");
    }
}
