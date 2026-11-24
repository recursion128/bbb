use super::*;
use crate::entity::{ChestBoatData, EntityData, MinecartContainerData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
};
use bbb_protocol::item_types::{VANILLA_ITEM_DIRT_ID, VANILLA_ITEM_STONE_ID};
use bbb_server_types::save::Java26LevelKey;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn visible_world() -> ServerWorld {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    for chunk in [(0, 0), (1, 0)] {
        world
            .level_mut_by_id(&world.require_level_id(&key()))
            .entities
            .apply_chunk_visibility(chunk, crate::entity::EntityVisibility::Tracked);
    }
    world
}

fn item(item_id: i32, count: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count,
        ..ItemStackSummary::empty()
    })
}

fn access<'a>(
    world: &'a mut ServerWorld,
    key: &Java26LevelKey,
    pos: BlockPos,
) -> WorldContainerAccess<'a> {
    let dimension = world.require_level_id(key);
    WorldContainerAccess {
        world,
        dimension,
        pos,
        facing: Direction::Down,
    }
}

fn cart(world: &mut ServerWorld, pos: Vec3d) -> i32 {
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            pos,
            EntityData::ChestMinecart(MinecartContainerData::new_chest()),
        )
}

fn boat(world: &mut ServerWorld, pos: Vec3d) -> i32 {
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            pos,
            EntityData::ChestBoat(ChestBoatData::new()),
        )
}

fn place(world: &mut ServerWorld, pos: BlockPos, resource: &str, properties: &[(&str, &str)]) {
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        resource,
        &properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
    )
    .unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
}

#[test]
fn bookshelf_container_max_stack_size_is_one_book() {
    let mut world = visible_world();
    let dimension = key();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    place(
        &mut world,
        pos,
        "minecraft:chiseled_bookshelf",
        &[
            ("facing", "north"),
            ("slot_0_occupied", "false"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ],
    );
    let book = item(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID, 2);
    let mut access = access(&mut world, &dimension, pos);
    let container = access.container_at(pos).unwrap();
    assert_eq!(access.max_stack_size(container, &book).unwrap(), 1);
}

#[test]
fn container_lookup_uses_live_intersecting_entities_and_block_priority() {
    let mut world = visible_world();
    let dimension = key();
    let pos = BlockPos { x: 15, y: 64, z: 0 };
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let boat_id = boat(
        &mut world,
        Vec3d {
            x: 16.0,
            y: 64.0,
            z: 0.5,
        },
    );
    let cart_id = cart(
        &mut world,
        Vec3d {
            x: 15.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let removed = cart(
        &mut world,
        Vec3d {
            x: 15.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .get_mut(removed)
        .unwrap()
        .discard();
    let touching = cart(
        &mut world,
        Vec3d {
            x: 15.5,
            y: 65.0,
            z: 0.5,
        },
    );
    assert!(world
        .level_by_id(&world.require_level_id(&dimension))
        .entities
        .get(touching)
        .unwrap()
        .is_alive());
    assert!(
        matches!(access(&mut world, &dimension, pos).container_at(pos), Some(WorldContainer::Entity(id)) if id == cart_id || id == boat_id)
    );
    place(
        &mut world,
        pos,
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    assert!(matches!(
        access(&mut world, &dimension, pos).container_at(pos),
        Some(WorldContainer::Block(_))
    ));
}

#[test]
fn adding_items_to_a_chest_boat_merges_the_existing_slot() {
    let mut world = visible_world();
    let dimension = key();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let id = boat(
        &mut world,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let stack = item(VANILLA_ITEM_STONE_ID, 5);
    let expected = stack.with_count(8);
    let extra = item(VANILLA_ITEM_STONE_ID, 3);
    let mut access = access(&mut world, &dimension, pos);
    let target = access.container_at(pos).unwrap();
    assert!(!access.is_crafter(target));
    assert!(access
        .add_item(None, target, stack, Some(Direction::North))
        .unwrap()
        .is_empty());
    assert!(access
        .add_item(None, target, extra, Some(Direction::North))
        .unwrap()
        .is_empty());
    assert_eq!(
        item_slots::read(
            access
                .world
                .level_by_id(&access.dimension)
                .entities
                .get(id)
                .unwrap(),
            0,
        )
        .unwrap()
        .unwrap()
        .count(),
        8
    );
    assert_eq!(
        access.get_item(WorldContainer::Entity(id), 0).unwrap(),
        expected
    );
}

#[test]
fn entity_empty_is_lazy_and_first_insertion_fills_pending_loot() {
    let table = "minecraft:chests/igloo_chest";
    let config = crate::tests::datapack_config_with_loot_table(
        table,
        &serde_json::json!({
            "type": "minecraft:chest",
            "pools": [{"rolls": 1, "entries": [{
                "type": "minecraft:item", "name": "minecraft:stone",
                "functions": [{"function": "minecraft:set_count", "count": 2}]
            }]}]
        }),
    );
    let mut world = ServerWorld::new(config).unwrap();
    world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities
        .apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Tracked);
    let dimension = key();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let mut data = MinecartContainerData::new_chest();
    data.set_loot_table(table.to_string(), 26_101).unwrap();
    let id = world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            Vec3d {
                x: 0.25,
                y: 64.0,
                z: 0.75,
            },
            EntityData::ChestMinecart(data),
        );
    let stack = item(VANILLA_ITEM_DIRT_ID, 3);
    let mut access = access(&mut world, &dimension, pos);
    let target = access.container_at(pos).unwrap();
    assert!(access.is_empty(target).unwrap());
    assert!(crate::entity::minecart_container::minecart_container(
        access
            .world
            .level_by_id(&access.dimension)
            .entities
            .get(id)
            .unwrap()
            .data()
    )
    .unwrap()
    .loot_table_id()
    .is_some());
    assert!(access
        .add_item(None, target, stack, Some(Direction::Down))
        .unwrap()
        .is_empty());
    let mut totals = std::collections::BTreeMap::new();
    for slot in 0..access.size(target) {
        let stack = access.get_item(target, slot).unwrap();
        if let Some(item) = stack.item_id() {
            *totals.entry(item).or_insert(0) += stack.count();
        }
    }
    assert_eq!(
        totals,
        std::collections::BTreeMap::from([(VANILLA_ITEM_STONE_ID, 2), (VANILLA_ITEM_DIRT_ID, 3)])
    );
    assert!(crate::entity::minecart_container::minecart_container(
        access
            .world
            .level_by_id(&access.dimension)
            .entities
            .get(id)
            .unwrap()
            .data()
    )
    .unwrap()
    .loot_table_id()
    .is_none());
}

#[test]
fn hopper_rejection_preserves_entity_and_bookshelf_stacks() {
    let mut world = visible_world();
    let dimension = key();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    place(
        &mut world,
        pos,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    let dirt = item(VANILLA_ITEM_DIRT_ID, 63);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()
        .fill(dirt);
    let native = item(VANILLA_ITEM_STONE_ID, 70);
    let mut data = MinecartContainerData::new_chest();
    data.set_item(0, Some(native.clone())).unwrap();
    let id = world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            EntityData::ChestMinecart(data),
        );
    world.tick_hopper(&world.require_level_id(&dimension), pos);
    assert_eq!(
        access(&mut world, &dimension, pos)
            .get_item(WorldContainer::Entity(id), 0)
            .unwrap(),
        native
    );
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()[0] = RuntimeItemStack::empty();
    world.tick_hopper(&world.require_level_id(&dimension), pos);
    assert_eq!(
        access(&mut world, &dimension, pos)
            .get_item(WorldContainer::Entity(id), 0)
            .unwrap(),
        native.with_count(69)
    );
    let hopper = world
        .level_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities()
        .get(pos)
        .unwrap()
        .hopper()
        .unwrap();
    assert_eq!(hopper.items()[0].item_id(), Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(hopper.items()[0].count(), 1);
    assert_eq!(hopper.cooldown_time(), MOVE_ITEM_SPEED);
    let source = BlockPos { y: 65, ..pos };
    place(
        &mut world,
        source,
        "minecraft:chiseled_bookshelf",
        &[
            ("facing", "north"),
            ("slot_0_occupied", "true"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ],
    );
    let books = item(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID, 2);
    let level_id = world.require_level_id(&dimension);
    world
        .level_mut_by_id(&level_id)
        .chunks
        .block_entities_mut()
        .get_mut(source)
        .unwrap()
        .chiseled_bookshelf_mut()
        .unwrap()
        .items_mut()[0] = books.clone();
    let hopper = world
        .level_mut_by_id(&level_id)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap();
    hopper.set_cooldown(0);
    hopper.items_mut()[0] = books.with_count(63);
    world.tick_hopper(&level_id, pos);
    assert_eq!(
        world
            .level_by_id(&level_id)
            .chunks
            .block_entities()
            .get(source)
            .unwrap()
            .chiseled_bookshelf()
            .unwrap()
            .items()[0],
        books
    );
    assert_eq!(
        world
            .level_by_id(&level_id)
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .hopper()
            .unwrap()
            .items()[0],
        books.with_count(63)
    );
    world
        .level_mut_by_id(&level_id)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()[0] = RuntimeItemStack::empty();
    world.tick_hopper(&level_id, pos);
    assert!(world
        .level_by_id(&level_id)
        .chunks
        .block_entities()
        .get(source)
        .unwrap()
        .chiseled_bookshelf()
        .unwrap()
        .items()[0]
        .is_empty());
    assert_eq!(
        world
            .level_by_id(&level_id)
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .hopper()
            .unwrap()
            .items()[0],
        books
    );
}

#[test]
fn hopper_last_cargo_item_keeps_its_original_after_failed_pull_and_retry() {
    let mut world = visible_world();
    let dimension = key();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    place(
        &mut world,
        pos,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    let dirt = item(VANILLA_ITEM_DIRT_ID, 63);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()
        .fill(dirt);
    let native = item(VANILLA_ITEM_STONE_ID, 1);
    let mut data = MinecartContainerData::new_chest();
    data.set_item(0, Some(native.clone())).unwrap();
    let id = world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .spawn(
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            EntityData::ChestMinecart(data),
        );
    let minecart_slot = |world: &ServerWorld| {
        item_slots::read(
            world
                .level_by_id(&world.require_level_id(&dimension))
                .entities
                .get(id)
                .unwrap(),
            0,
        )
        .unwrap()
    };
    world.tick_hopper(&world.require_level_id(&dimension), pos);
    assert_eq!(minecart_slot(&world).unwrap().count(), 1);
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()[0] = RuntimeItemStack::empty();
    world.tick_hopper(&world.require_level_id(&dimension), pos);
    assert!(minecart_slot(&world).is_none_or(|stack| stack.is_empty()));
    let hopper = world
        .level_by_id(&world.require_level_id(&dimension))
        .chunks
        .block_entities()
        .get(pos)
        .unwrap()
        .hopper()
        .unwrap();
    assert_eq!(hopper.items()[0].item_id(), Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(hopper.items()[0].count(), 1);
}

const ITEM_HOPPER: BlockPos = BlockPos { x: 8, y: 64, z: 8 };

fn item_world() -> (ServerWorld, crate::mailbox::WorldCommandReceiver) {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    world.set_test_respawn_radius(0);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "hopper", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 11.5,
            y: 64.0,
            z: 8.5,
        },
    );
    place(
        &mut world,
        ITEM_HOPPER,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    (world, receiver)
}

fn dropped(world: &mut ServerWorld, y: f64, stack: RuntimeItemStack) -> i32 {
    let store = &mut world.level_mut(Java26BuiltinDimension::Overworld).entities;
    let id =
        crate::entity::item::spawn_item_entity(store, Vec3d { x: 8.5, y, z: 8.5 }, stack).unwrap();
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d::default());
    entity.set_no_gravity(true);
    id
}

fn dropped_stack(world: &ServerWorld, id: i32) -> Option<&RuntimeItemStack> {
    match world
        .level(Java26BuiltinDimension::Overworld)
        .entities
        .get(id)?
        .data()
    {
        EntityData::Item(data) => Some(data.item()),
        _ => None,
    }
}

fn hopper_mut(world: &mut ServerWorld) -> &mut crate::block_entity::hopper::HopperBlockEntity {
    world
        .level_mut(Java26BuiltinDimension::Overworld)
        .chunks
        .block_entities_mut()
        .get_mut(ITEM_HOPPER)
        .unwrap()
        .hopper_mut()
        .unwrap()
}

#[test]
fn dropped_stacks_keep_components_remainders_tracking_and_saved_inventory() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    let (mut world, receiver) = item_world();
    let stack = item(VANILLA_ITEM_STONE_ID, 5)
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            Some(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "hopper".into(),
                value: NbtSummaryValue::Int(42),
            }])),
            world.item_stack_resolver().as_dyn(),
        )
        .unwrap();
    let power = BlockPos {
        x: 7,
        ..ITEM_HOPPER
    };
    place(&mut world, power, "minecraft:redstone_block", &[]);
    let id = dropped(&mut world, 65.25, stack.clone());
    let announced = world.tick(receiver.drain());
    assert_eq!(dropped_stack(&world, id), Some(&stack));
    assert!(announced.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. } if add.id == id)));
    hopper_mut(&mut world)
        .items_mut()
        .fill(item(VANILLA_ITEM_DIRT_ID, 64));
    place(&mut world, power, "minecraft:air", &[]);
    world.tick(receiver.drain());
    assert_eq!(dropped_stack(&world, id), Some(&stack));
    hopper_mut(&mut world).items_mut()[0] = stack.with_count(62);
    let mut partial = world.tick(receiver.drain());
    partial.events.extend(world.tick(receiver.drain()).events);
    assert_eq!(dropped_stack(&world, id), Some(&stack.with_count(3)));
    assert_eq!(hopper_mut(&mut world).items()[0], stack.with_count(64));
    assert!(partial.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. } if update.id == id && update.values.iter().any(|value| matches!(&value.value, bbb_protocol::packets::EntityDataValueKind::ItemStack(item) if item.count == 3 && item.component_patch.custom_data == stack.summary().component_patch.custom_data)))));
    hopper_mut(&mut world).items_mut()[1] = RuntimeItemStack::empty();
    hopper_mut(&mut world).set_cooldown(0);
    let mut consumed = world.tick(receiver.drain());
    consumed.events.extend(world.tick(receiver.drain()).events);
    assert!(dropped_stack(&world, id).is_none());
    assert_eq!(hopper_mut(&mut world).items()[1], stack.with_count(3));
    assert!(hopper_mut(&mut world).is_on_cooldown());
    assert!(consumed.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::RemoveEntities(remove), .. } if remove.entity_ids.contains(&id))));
    let saved = world
        .block_entity_save_for_test(Java26BuiltinDimension::Overworld, ITEM_HOPPER)
        .unwrap();
    let state = world
        .level(Java26BuiltinDimension::Overworld)
        .chunks
        .get_block_state(8, 64, 8)
        .unwrap();
    let restored = BlockEntity::load_static_with_resolver(
        ITEM_HOPPER,
        state,
        &saved,
        world.item_stack_resolver().as_dyn(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(restored.hopper().unwrap().items()[1], stack.with_count(3));
}

#[test]
fn dropped_item_scan_obeys_source_and_collision_while_contact_reaches_inside() {
    let (mut world, receiver) = item_world();
    let above = BlockPos {
        y: 65,
        ..ITEM_HOPPER
    };
    let stack = item(VANILLA_ITEM_STONE_ID, 2);
    place(
        &mut world,
        above,
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let id = dropped(&mut world, 65.25, stack.clone());
    world.tick(receiver.drain());
    assert_eq!(dropped_stack(&world, id), Some(&stack));
    place(&mut world, above, "minecraft:stone", &[]);
    world.tick(receiver.drain());
    assert_eq!(dropped_stack(&world, id), Some(&stack));
    world.install_level_registry_tags(std::sync::Arc::new(
        crate::registry_tags::RegistryTagSnapshot::new([(
            "minecraft:block".into(),
            "minecraft:does_not_block_hoppers".into(),
            vec!["minecraft:stone".into()],
        )]),
    ));
    world.tick(receiver.drain());
    assert!(dropped_stack(&world, id).is_none());
    assert_eq!(hopper_mut(&mut world).items()[0], stack);
    world.install_level_registry_tags(std::sync::Arc::new(
        crate::registry_tags::RegistryTagSnapshot::new([]),
    ));
    hopper_mut(&mut world).set_cooldown(0);
    let contact = dropped(&mut world, 64.75, item(VANILLA_ITEM_DIRT_ID, 3));
    world.tick(receiver.drain());
    assert!(dropped_stack(&world, contact).is_none());
    assert_eq!(
        hopper_mut(&mut world).items()[1],
        item(VANILLA_ITEM_DIRT_ID, 3)
    );
}

fn block_state_at(world: &ServerWorld, pos: BlockPos) -> i32 {
    world
        .level(Java26BuiltinDimension::Overworld)
        .chunks
        .block_state_at(pos)
}

fn hopper_slot(world: &mut ServerWorld, pos: BlockPos) -> &mut RuntimeItemStack {
    &mut world
        .level_mut(Java26BuiltinDimension::Overworld)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .hopper_mut()
        .unwrap()
        .items_mut()[0]
}

/// `ComposterBlock.getContainer` (ComposterBlock.java:367-374): a hopper above
/// composts into it; a hopper below takes the bone meal and empties it.
#[test]
fn hoppers_fill_a_composter_from_above_and_take_bone_meal_from_below() {
    let mut world = visible_world();
    let level_id = world.require_level_id(&key());
    world
        .level_mut_by_id(&level_id)
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let above = BlockPos { x: 0, y: 65, z: 0 };
    let composter = BlockPos { x: 0, y: 64, z: 0 };
    place(
        &mut world,
        composter,
        "minecraft:composter",
        &[("level", "0")],
    );
    place(
        &mut world,
        above,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    *hopper_slot(&mut world, above) =
        item(bbb_protocol::item_types::VANILLA_ITEM_WHEAT_SEEDS_ID, 2);
    world.tick_hopper(&level_id, above);
    assert_eq!(
        crate::block_behavior::composter::fill_level(block_state_at(&world, composter)),
        1
    );
    assert_eq!(hopper_slot(&mut world, above).count(), 1);

    let ready = BlockPos { x: 2, y: 64, z: 0 };
    let below = BlockPos { x: 2, y: 63, z: 0 };
    place(&mut world, ready, "minecraft:composter", &[("level", "8")]);
    place(
        &mut world,
        below,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    world.tick_hopper(&level_id, below);
    assert_eq!(
        hopper_slot(&mut world, below).item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_BONE_MEAL_ID)
    );
    assert_eq!(
        crate::block_behavior::composter::fill_level(block_state_at(&world, ready)),
        0
    );
}

/// `JukeboxBlockEntity` is a `ContainerSingleItem` (JukeboxBlockEntity.java:143-150):
/// a hopper inserts a disc into an empty jukebox and the one below pulls it out.
#[test]
fn hoppers_insert_and_extract_jukebox_discs() {
    let mut world = visible_world();
    let level_id = world.require_level_id(&key());
    world
        .level_mut_by_id(&level_id)
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    let above = BlockPos { x: 0, y: 65, z: 0 };
    let jukebox = BlockPos { x: 0, y: 64, z: 0 };
    place(
        &mut world,
        jukebox,
        "minecraft:jukebox",
        &[("has_record", "false")],
    );
    place(
        &mut world,
        above,
        "minecraft:hopper",
        &[("facing", "down"), ("enabled", "true")],
    );
    let disc = bbb_protocol::item_types::VANILLA_ITEM_MUSIC_DISC_CAT_ID;
    *hopper_slot(&mut world, above) = item(disc, 1);
    world.tick_hopper(&level_id, above);
    assert!(crate::block_behavior::jukebox::has_record(block_state_at(
        &world, jukebox
    )));
    assert!(hopper_slot(&mut world, above).is_empty());

    // A playing jukebox powers the hopper under it, so the extraction runs
    // against a loaded disc that is not playing.
    let stopped = BlockPos { x: 2, y: 64, z: 0 };
    let below = BlockPos { x: 2, y: 63, z: 0 };
    place(
        &mut world,
        below,
        "minecraft:hopper",
        &[("facing", "east"), ("enabled", "true")],
    );
    place(
        &mut world,
        stopped,
        "minecraft:jukebox",
        &[("has_record", "true")],
    );
    let mut record = crate::tests::save_native_fixture_block_entity(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, stopped)
            .unwrap(),
    );
    record.push(bbb_protocol::NbtSummaryEntry {
        name: "RecordItem".into(),
        value: bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "id".into(),
                value: bbb_protocol::NbtSummaryValue::String("minecraft:music_disc_cat".into()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "count".into(),
                value: bbb_protocol::NbtSummaryValue::Int(1),
            },
        ]),
    });
    *world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, stopped)
        .unwrap() = crate::tests::load_native_fixture_block_entity(
        stopped,
        block_state_at(&world, stopped),
        &record,
    );
    world.tick_hopper(&level_id, below);
    assert_eq!(hopper_slot(&mut world, below).item_id(), Some(disc));
    assert!(!crate::block_behavior::jukebox::has_record(block_state_at(
        &world, stopped
    )));
}
