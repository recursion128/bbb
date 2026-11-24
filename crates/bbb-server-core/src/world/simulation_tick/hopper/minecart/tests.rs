use super::super::*;
use crate::entity::item::{spawn_item_entity, stack_count};
use crate::entity::{EntityData, EntityStore};

use bbb_protocol::packets::{BlockPos, Vec3d};

fn state_id(resource: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(resource)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn place(world: &mut ServerWorld, pos: BlockPos, resource: &str, properties: &[(&str, &str)]) {
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        resource,
        &properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
    .unwrap();
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z),
        Some(state)
    );
}
fn item(item_id: i32, count: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count,
        ..ItemStackSummary::empty()
    })
}
fn stack_of(resource: &str, count: i32) -> RuntimeItemStack {
    item(
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource)
            .unwrap()
            .id,
        count,
    )
}
fn store(world: &ServerWorld) -> &EntityStore {
    &world.level(Java26BuiltinDimension::Overworld).entities
}
fn store_mut(world: &mut ServerWorld) -> &mut EntityStore {
    &mut world.level_mut(Java26BuiltinDimension::Overworld).entities
}

use crate::fluid::with_property;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
};

use crate::entity::chest_boat::ChestBoatData;
use crate::entity::minecart_container::{
    minecart_container, MinecartContainerData, HOPPER_CONTAINER_SIZE,
};

const RAIL: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const CART: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn hopper_on_rail(rail_state: i32) -> (ServerWorld, crate::mailbox::WorldCommandReceiver, i32) {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    world.set_test_respawn_radius(0);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "minecart_hopper", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 4.5,
        },
    );
    place(
        &mut world,
        BlockPos { y: 63, ..RAIL },
        "minecraft:stone",
        &[],
    );
    if bbb_protocol::block_state_types::vanilla_block_state_property(rail_state, "powered")
        == Some("true")
    {
        place(
            &mut world,
            BlockPos { y: 63, ..RAIL },
            "minecraft:redstone_block",
            &[],
        );
    }
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        RAIL,
        rail_state,
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    let id = crate::entity::spawn_minecart_container(
        store_mut(&mut world),
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        CART,
        0.0,
    )
    .unwrap();
    (world, receiver, id)
}

fn drop_item(store: &mut EntityStore, position: Vec3d, stack: RuntimeItemStack) -> i32 {
    let id = spawn_item_entity(store, position, stack).expect("a fresh item entity");
    store
        .get_mut(id)
        .expect("item entity is live")
        .set_delta_movement(Vec3d::default());
    id
}

fn container(store: &EntityStore, id: i32) -> &MinecartContainerData {
    minecart_container(store.get(id).expect("entity is live").data())
        .expect("entity is a container minecart")
}

fn set_item(
    store: &mut EntityStore,
    id: i32,
    slot: usize,
    stack: Option<RuntimeItemStack>,
) -> std::result::Result<bool, String> {
    crate::entity::item_slots::set_container_item(
        store.get_mut(id).unwrap(),
        slot,
        stack.unwrap_or_else(RuntimeItemStack::empty),
    )
}

fn slot_counts(store: &EntityStore, id: i32) -> Vec<i32> {
    let data = container(store, id);
    (0..data.container_size())
        .map(|slot| data.item(slot).map_or(0, |stack| stack_count(&stack)))
        .collect()
}

fn item_stack(store: &EntityStore, id: i32) -> RuntimeItemStack {
    match store.get(id).expect("item entity is live").data() {
        EntityData::Item(data) => data.item().clone(),
        other => panic!("expected an item entity, found {other:?}"),
    }
}

/// `MinecartHopper.suckInItems`'s own scan (`MinecartHopper.java:109-114`):
/// the box is the cart's own, widened by a quarter on x and z only, so a drop
/// lying beside the rail is taken while `Hopper.SUCK_AABB` (`Hopper.java:8`)
/// never reaches down to it.
#[test]
fn a_drop_beside_the_cart_is_taken_through_the_inflated_bounding_box() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 3),
    );

    world.tick(receiver.drain());

    assert!(store(&world).get(item).is_none());
    assert_eq!(slot_counts(store(&world), id), [3, 0, 0, 0, 0]);
}

/// `HopperBlockEntity.getItemsAtAndAbove` (`:358-361`) reaches a full block
/// above the cart, which the inflated cart box does not.
#[test]
fn a_floating_drop_above_the_cart_is_taken_through_the_suck_box() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    );

    world.tick(receiver.drain());

    assert!(store(&world).get(item).is_none());
    assert_eq!(slot_counts(store(&world), id), [1, 0, 0, 0, 0]);
}

/// The suck box ends two blocks over the hopper's own level y
/// (`Block.column(16.0, 11.0, 32.0)`, `Hopper.java:8`).
#[test]
fn a_drop_over_the_top_of_the_suck_box_is_left_alone() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 0.5,
            y: 66.5,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    );

    world.tick(receiver.drain());

    assert_eq!(stack_count(&item_stack(store(&world), item)), 1);
    assert_eq!(slot_counts(store(&world), id), [0, 0, 0, 0, 0]);
}

/// `MinecartHopper.activateMinecart` (`:47-53`) closes the same tick's
/// `tryConsumeItems` gate (`:98`).
#[test]
fn a_powered_activator_rail_stops_the_cart_consuming_in_that_tick() {
    let powered = with_property(state_id("minecraft:activator_rail"), "powered", "true")
        .expect("a powered activator rail state");
    let (mut world, receiver, id) = hopper_on_rail(powered);
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    );

    world.tick(receiver.drain());

    assert!(!container(store(&world), id).is_enabled());
    assert_eq!(stack_count(&item_stack(store(&world), item)), 1);
    assert_eq!(slot_counts(store(&world), id), [0, 0, 0, 0, 0]);
}

/// Only `MinecartHopper` consumes; `MinecartChest` shares every other part of
/// `AbstractMinecartContainer`.
#[test]
fn a_chest_minecart_on_the_same_rail_consumes_nothing() {
    let (mut world, receiver, hopper_id) = hopper_on_rail(state_id("minecraft:rail"));
    store_mut(&mut world).remove(hopper_id, crate::entity::RemovalReason::Discarded);
    let id = crate::entity::spawn_minecart_container(
        store_mut(&mut world),
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        CART,
        0.0,
    )
    .unwrap();
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    );

    world.tick(receiver.drain());

    assert_eq!(stack_count(&item_stack(store(&world), item)), 1);
    assert!(container(store(&world), id).item(0).is_none());
}

/// `HopperBlockEntity.suckInItems`'s source-container branch (`:222-231`) over
/// `getEntityContainer` (`:395-400`): one item leaves the container above per
/// tick, and finding one suppresses the item-entity walk at `:236-241`.
#[test]
fn a_chest_minecart_above_hands_over_one_item_a_tick() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let source = crate::entity::spawn_minecart_container(
        store_mut(&mut world),
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        Vec3d { y: 65.0, ..CART },
        0.0,
    )
    .expect("a fresh chest minecart");
    set_item(
        store_mut(&mut world),
        source,
        0,
        Some(stack_of("minecraft:stone", 5)),
    )
    .expect("slot zero");
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    );

    world.tick(receiver.drain());

    assert_eq!(slot_counts(store(&world), id), [1, 0, 0, 0, 0]);
    assert_eq!(
        container(store(&world), source)
            .item(0)
            .map(|stack| stack_count(&stack)),
        Some(4),
        "one item per tick leaves the rest in the chest minecart"
    );
    assert_eq!(
        stack_count(&item_stack(store(&world), item)),
        1,
        "a source container above suppresses the item walk"
    );
}

/// The same source branch reaches `AbstractChestBoat`, the other
/// `ContainerEntity` this store holds.
#[test]
fn a_chest_boat_above_is_a_source_container_too() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let source = crate::entity::spawn_chest_boat(
        store_mut(&mut world),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        Vec3d { y: 65.0, ..CART },
        0.0,
    )
    .expect("a fresh chest boat");
    set_item(
        store_mut(&mut world),
        source,
        0,
        Some(stack_of("minecraft:stone", 2)),
    )
    .expect("slot zero");

    world.tick(receiver.drain());

    assert_eq!(slot_counts(store(&world), id), [1, 0, 0, 0, 0]);
    assert_eq!(
        chest_boat_data(store(&world), source)
            .item(0)
            .map(|stack| stack_count(&stack)),
        Some(1)
    );
}

fn chest_boat_data(store: &EntityStore, id: i32) -> &ChestBoatData {
    crate::entity::chest_boat::chest_boat(store.get(id).expect("entity is live").data())
        .expect("entity is a chest boat")
}

/// A full cart leaves its source inventory unchanged.
#[test]
fn a_full_hopper_leaves_the_source_unchanged() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    for slot in 0..HOPPER_CONTAINER_SIZE {
        set_item(
            store_mut(&mut world),
            id,
            slot,
            Some(stack_of("minecraft:stone", 64)),
        )
        .expect("a hopper slot");
    }
    let source = crate::entity::spawn_minecart_container(
        store_mut(&mut world),
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        Vec3d { y: 65.0, ..CART },
        0.0,
    )
    .expect("a fresh chest minecart");
    set_item(
        store_mut(&mut world),
        source,
        0,
        Some(stack_of("minecraft:stone", 1)),
    )
    .expect("slot zero");

    world.tick(receiver.drain());

    assert_eq!(slot_counts(store(&world), id), [64; HOPPER_CONTAINER_SIZE]);
    assert_eq!(
        container(store(&world), source)
            .item(0)
            .map(|stack| stack_count(&stack)),
        Some(1),
        "the one-item source slot is restored whole"
    );
}

/// A drop carrying more than its item's ceiling loses the excess to the
/// `limitSize` in `ContainerEntity.setChestVehicleItem` (`:147-151`).
#[test]
fn an_overstacked_drop_is_clamped_to_the_item_ceiling() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:diamond_sword", 10),
    );

    world.tick(receiver.drain());

    assert!(store(&world).get(item).is_none());
    assert_eq!(slot_counts(store(&world), id), [1, 0, 0, 0, 0]);
}

/// `HopperBlockEntity.addItem(Container, ItemEntity)` (`:267-280`): a drop the
/// hopper can only partly take stays on the ground holding the remainder.
#[test]
fn a_partly_taken_drop_keeps_the_remainder_on_the_ground() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    for slot in 0..HOPPER_CONTAINER_SIZE - 1 {
        set_item(
            store_mut(&mut world),
            id,
            slot,
            Some(stack_of("minecraft:stone", 64)),
        )
        .expect("a hopper slot");
    }
    set_item(
        store_mut(&mut world),
        id,
        HOPPER_CONTAINER_SIZE - 1,
        Some(stack_of("minecraft:stone", 63)),
    )
    .expect("the last hopper slot");
    let item = drop_item(
        store_mut(&mut world),
        Vec3d {
            x: 1.2,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 5),
    );

    world.tick(receiver.drain());

    assert_eq!(slot_counts(store(&world), id), [64, 64, 64, 64, 64]);
    assert_eq!(stack_count(&item_stack(store(&world), item)), 4);
}

const SOURCE: BlockPos = BlockPos { y: 65, ..RAIL };

fn block_source(world: &mut ServerWorld, resource: &str) -> WorldContainer {
    let properties: &[(&str, &str)] = match resource {
        "minecraft:chest" => &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
        "minecraft:furnace" => &[("facing", "north"), ("lit", "false")],
        _ => &[],
    };
    place(world, SOURCE, resource, properties);
    let dimension = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    WorldContainerAccess {
        world,
        dimension,
        pos: SOURCE,
        facing: Direction::Down,
    }
    .container_at(SOURCE)
    .unwrap()
}

fn source_access(world: &mut ServerWorld) -> WorldContainerAccess<'_> {
    let dimension = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    WorldContainerAccess {
        world,
        dimension,
        pos: SOURCE,
        facing: Direction::Down,
    }
}

#[test]
fn block_source_preserves_components_remainder_full_cart_and_saved_inventory() {
    use crate::entity::save::{read_entity_chunk, write_entity_chunk};
    use crate::entity::EntityIdCounter;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let source = block_source(&mut world, "minecraft:chest");
    place(
        &mut world,
        BlockPos { y: 66, ..RAIL },
        "minecraft:stone",
        &[],
    );
    let stack = stack_of("minecraft:stone", 5)
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            Some(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "cart".into(),
                value: NbtSummaryValue::Int(42),
            }])),
            world.item_stack_resolver().as_dyn(),
        )
        .unwrap();
    source_access(&mut world)
        .set_item(source, 0, stack.clone())
        .unwrap();
    for slot in 0..HOPPER_CONTAINER_SIZE {
        set_item(
            store_mut(&mut world),
            id,
            slot,
            Some(stack_of("minecraft:dirt", 64)),
        )
        .unwrap();
    }
    world.tick(receiver.drain());
    assert_eq!(
        source_access(&mut world).get_item(source, 0).unwrap(),
        stack
    );
    set_item(store_mut(&mut world), id, 0, None).unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        source_access(&mut world).get_item(source, 0).unwrap(),
        stack.with_count(4)
    );
    assert_eq!(
        container(store(&world), id).item(0).as_deref(),
        Some(&stack.with_count(1))
    );
    let saved = write_entity_chunk(store(&world), (0, 0)).unwrap();
    let mut restored = EntityStore::new(7, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &saved).unwrap();
    assert!(ids.into_iter().any(|id| matches!(restored.get(id).unwrap().data(), EntityData::HopperMinecart(data) if data.item(0).as_deref() == Some(&stack.with_count(1)))));
}

#[test]
fn furnace_source_only_exposes_slots_extractable_from_below() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    let source = block_source(&mut world, "minecraft:furnace");
    source_access(&mut world)
        .set_item(source, 0, stack_of("minecraft:iron_ore", 3))
        .unwrap();
    source_access(&mut world)
        .set_item(source, 2, stack_of("minecraft:iron_ingot", 2))
        .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        container(store(&world), id).item(0).as_deref(),
        Some(&stack_of("minecraft:iron_ingot", 1))
    );
    assert_eq!(
        source_access(&mut world).get_item(source, 0).unwrap(),
        stack_of("minecraft:iron_ore", 3)
    );
    assert_eq!(
        source_access(&mut world).get_item(source, 2).unwrap(),
        stack_of("minecraft:iron_ingot", 1)
    );
}

#[test]
fn empty_block_source_suppresses_upper_scan_and_keeps_cart_box_pickup() {
    let (mut world, receiver, id) = hopper_on_rail(state_id("minecraft:rail"));
    block_source(&mut world, "minecraft:chest");
    let upper = drop_item(
        store_mut(&mut world),
        Vec3d { y: 65.5, ..CART },
        stack_of("minecraft:stone", 2),
    );
    let beside = drop_item(
        store_mut(&mut world),
        Vec3d { x: 1.2, ..CART },
        stack_of("minecraft:dirt", 3),
    );
    world.tick(receiver.drain());
    assert_eq!(
        item_stack(store(&world), upper),
        stack_of("minecraft:stone", 2)
    );
    assert!(store(&world).get(beside).is_none());
    assert_eq!(
        container(store(&world), id).item(0).as_deref(),
        Some(&stack_of("minecraft:dirt", 3))
    );
}
