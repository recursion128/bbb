//! `DispenserBlock` / `DropperBlock` production paths.

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
use bbb_protocol::packets::{BlockPos, ItemStackSummary};

use super::{state_of, stone, NativeWorld};

mod armadillo;
mod copper_golem;
mod fire_charge;
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::item_stack::RuntimeItemStack;

const DROPPER: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const DEST: BlockPos = BlockPos { x: 0, y: 64, z: 1 };
const SIDE: BlockPos = BlockPos { x: 1, y: 64, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn dropper(facing: &str, triggered: bool) -> i32 {
    state_of(
        "minecraft:dropper",
        &[
            ("facing", facing),
            ("triggered", if triggered { "true" } else { "false" }),
        ],
    )
}

fn dispenser(facing: &str, triggered: bool) -> i32 {
    state_of(
        "minecraft:dispenser",
        &[
            ("facing", facing),
            ("triggered", if triggered { "true" } else { "false" }),
        ],
    )
}

fn triggered(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("triggered")
        .cloned()
}

fn stone_stack(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    }
}

fn empty_nine() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); 9]
}

fn empty_chest() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); 27]
}

/// A neighbour signal sets `TRIGGERED` and arms a four-tick delay; removing it
/// clears `TRIGGERED`.
#[test]
fn a_dropper_triggers_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    assert_eq!(triggered(world.state(DROPPER)).as_deref(), Some("true"));

    world.set(SIDE, super::air());
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    assert_eq!(triggered(world.state(DROPPER)).as_deref(), Some("false"));
}

/// `DropperBlock.dispenseFrom` (`DropperBlock.java:48-78`): one item moves
/// into the container the dropper faces.
#[test]
fn a_dropper_inserts_one_item_into_the_chest_it_faces() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, super::default_state("minecraft:chest"));
    let mut slots = empty_nine();
    slots[0] = stone_stack(4);
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_chest());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    let dropper_items = items(&mut world, DROPPER);
    assert_eq!(dropper_items[0].count, 3);
    let dest_items = items(&mut world, DEST);
    assert_eq!(dest_items[0].item_id, Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(dest_items[0].count, 1);
}

#[test]
fn an_empty_dispenser_clicks() {
    for is_dropper in [false, true] {
        let mut world = NativeWorld::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "alice", 1);
        world.set(FLOOR, stone());
        world.set(
            DROPPER,
            if is_dropper {
                dropper("south", false)
            } else {
                dispenser("south", false)
            },
        );
        set_items(&mut world, DROPPER, empty_nine());
        world.set(SIDE, super::default_state("minecraft:redstone_block"));
        world.neighbor_changed(
            DROPPER,
            block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
        );
        let mut clicked = false;
        for _ in 0..5 {
            let tick = world.world.tick(receiver.drain());
            clicked |= tick.events.iter().any(|event| {
                matches!(event,
                    crate::world::WorldEvent::Send {
                        packet: crate::connection::ClientboundPlayPacket::LevelEvent(update), ..
                    } if update.event_type == 1001 && update.pos == DROPPER
                )
            });
        }
        assert!(clicked);
        assert!(items(&mut world, DROPPER)
            .iter()
            .all(|stack| stack.count == 0));
        let activated = world.events().iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE
                && event.pos == DROPPER
        });
        assert_eq!(activated, !is_dropper);
    }
}

/// `ShulkerBoxBlockEntity.canPlaceItemThroughFace` (`:238-240`): a dropper
/// facing a shulker box will not insert a nested shulker-box item.
#[test]
fn a_dropper_does_not_insert_a_shulker_box_into_a_shulker_box() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, super::default_state("minecraft:shulker_box"));
    let mut slots = empty_nine();
    slots[0] = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID),
        count: 1,
        component_patch: Default::default(),
    };
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_chest());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(
        items(&mut world, DROPPER)[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_SHULKER_BOX_ID)
    );
    assert!(items(&mut world, DEST)[0].item_id.is_none());
}

fn empty_six() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); 6]
}

fn book_stack(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
        count,
        component_patch: Default::default(),
    }
}

fn chiseled_bookshelf() -> i32 {
    super::default_state("minecraft:chiseled_bookshelf")
}

/// `ChiseledBookShelfBlockEntity.acceptsItemType` (`:74-76`) plus
/// `getMaxStackSize` (`:69-71`): a dropper facing the shelf inserts a book
/// into the first vacant slot and refuses stone.
#[test]
fn a_dropper_inserts_a_book_into_a_chiseled_bookshelf() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, chiseled_bookshelf());
    let mut slots = empty_nine();
    slots[0] = book_stack(2);
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_six());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(items(&mut world, DROPPER)[0].count, 1);
    assert_eq!(
        items(&mut world, DEST)[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID)
    );
    assert_eq!(items(&mut world, DEST)[0].count, 1);
}

#[test]
fn a_dropper_does_not_insert_stone_into_a_chiseled_bookshelf() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, chiseled_bookshelf());
    let mut slots = empty_nine();
    slots[0] = stone_stack(2);
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_six());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(items(&mut world, DROPPER)[0].count, 2);
    assert!(items(&mut world, DEST)[0].item_id.is_none());
}

fn empty_one() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty()]
}

fn decorated_pot() -> i32 {
    super::default_state("minecraft:decorated_pot")
}

/// `DecoratedPotBlockEntity` is a one-slot `Container`; a dropper facing it
/// inserts one item.
#[test]
fn a_dropper_inserts_stone_into_a_decorated_pot() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, decorated_pot());
    let mut slots = empty_nine();
    slots[0] = stone_stack(2);
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_one());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(items(&mut world, DROPPER)[0].count, 1);
    assert_eq!(
        items(&mut world, DEST)[0].item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
}

fn empty_three() -> Vec<ItemStackSummary> {
    vec![ItemStackSummary::empty(); 3]
}

fn oak_shelf() -> i32 {
    super::default_state("minecraft:oak_shelf")
}

/// A dropper facing a shelf inserts one item into the first vacant slot.
#[test]
fn a_dropper_inserts_stone_into_a_shelf() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FLOOR, stone());
    world.set(DROPPER, dropper("south", false));
    world.set(DEST, oak_shelf());
    let mut slots = empty_nine();
    slots[0] = stone_stack(2);
    set_items(&mut world, DROPPER, slots);
    set_items(&mut world, DEST, empty_three());
    world.set(SIDE, super::default_state("minecraft:redstone_block"));
    world.neighbor_changed(
        DROPPER,
        block_type_of(super::default_state("minecraft:redstone_block")).unwrap(),
    );
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(items(&mut world, DROPPER)[0].count, 1);
    assert_eq!(
        items(&mut world, DEST)[0].item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
}

fn set_items(world: &mut NativeWorld, pos: BlockPos, stacks: Vec<ItemStackSummary>) {
    let stacks = stacks
        .into_iter()
        .map(RuntimeItemStack::from_summary)
        .collect();
    world.with_level(|level| level.set_container_items(pos, stacks).unwrap());
}

fn items(world: &mut NativeWorld, pos: BlockPos) -> Vec<ItemStackSummary> {
    world.with_level(|level| {
        let container = level.block_container(pos, true).unwrap();
        (0..container.size())
            .map(|slot| {
                level
                    .get_container_item(container, slot)
                    .unwrap()
                    .unwrap()
                    .into_summary()
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------
// `DispenseItemBehavior.bootStrap`: one representative per item family.
// ---------------------------------------------------------------------------

/// The cell a south-facing dispenser at `DROPPER` acts on.
const FRONT: BlockPos = DEST;
const UNDER_FRONT: BlockPos = BlockPos { x: 0, y: 63, z: 1 };

fn stack(id: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(id)
                .unwrap()
                .id,
        ),
        count,
        component_patch: Default::default(),
    }
}

/// A south-facing dispenser holding `held` in its first slot, fired by a
/// redstone block and given the four-tick delay plus one.
fn fire(world: &mut NativeWorld, held: ItemStackSummary) {
    world.set(FLOOR, stone());
    world.set(DROPPER, dispenser("south", false));
    let mut slots = empty_nine();
    slots[0] = held;
    set_items(world, DROPPER, slots);
    let redstone = super::default_state("minecraft:redstone_block");
    world.set(SIDE, redstone);
    world.neighbor_changed(DROPPER, block_type_of(redstone).unwrap());
    for _ in 0..5 {
        world.run_tick();
    }
}

fn held(world: &mut NativeWorld) -> ItemStackSummary {
    items(world, DROPPER)[0].clone()
}

fn entity_count(world: &NativeWorld, entity_type_id: i32) -> usize {
    world
        .entities()
        .iter()
        .filter(|entity| entity.entity_type_id() == entity_type_id)
        .count()
}

fn block_at(world: &mut NativeWorld, pos: BlockPos) -> Option<&'static str> {
    block_type_of(world.state(pos))
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .map(|entry| entry.resource_id)
}

/// `ProjectileDispenseBehavior` shoots arrows and throwables as entities.
#[test]
fn a_dispenser_shoots_arrows_and_snowballs() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    };
    for (item, entity_type_id) in [
        ("minecraft:arrow", VANILLA_ENTITY_TYPE_ARROW_ID),
        ("minecraft:snowball", VANILLA_ENTITY_TYPE_SNOWBALL_ID),
    ] {
        let mut world = NativeWorld::ticking(DROPPER);
        fire(&mut world, stack(item, 3));
        assert_eq!(entity_count(&world, entity_type_id), 1, "{item}");
        assert_eq!(held(&mut world).count, 2, "{item}");
        assert!(world.items().is_empty(), "{item} is not dropped as an item");
    }
}

/// `EquipmentDispenseItemBehavior` puts a helmet on the armor stand in front.
#[test]
fn a_dispenser_puts_a_helmet_on_an_armor_stand() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.command("summon minecraft:armor_stand 0.5 64 1.5");
    fire(&mut world, stack("minecraft:iron_helmet", 1));
    let helmet = world.entities().into_iter().find_map(|entity| {
        entity
            .living()?
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Head)
            .and_then(|stack| stack.item_resource_id())
    });
    assert_eq!(helmet, Some("minecraft:iron_helmet"));
    assert_eq!(held(&mut world).count, 0);
}

/// `SpawnEggItemBehavior` spawns the egg's mob in front.
#[test]
fn a_dispenser_hatches_a_spawn_egg() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    fire(&mut world, stack("minecraft:pig_spawn_egg", 2));
    assert_eq!(
        entity_count(
            &world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID
        ),
        1
    );
    assert_eq!(held(&mut world).count, 1);
}

/// The filled-bucket behaviour places water and keeps the empty bucket; the
/// empty-bucket behaviour takes the source back.
#[test]
fn a_dispenser_empties_and_fills_a_bucket() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    fire(&mut world, stack("minecraft:water_bucket", 1));
    assert_eq!(block_at(&mut world, FRONT), Some("minecraft:water"));
    assert_eq!(held(&mut world), stack("minecraft:bucket", 1));

    world.set(SIDE, super::air());
    world.neighbor_changed(DROPPER, block_type_of(stone()).unwrap());
    fire(&mut world, stack("minecraft:bucket", 1));
    assert_eq!(held(&mut world), stack("minecraft:water_bucket", 1));
    assert_eq!(block_at(&mut world, FRONT), Some("minecraft:air"));
}

/// The glass-bottle behaviour fills a water bottle from water in front.
#[test]
fn a_dispenser_fills_a_glass_bottle_from_water() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    world.set(FRONT, super::default_state("minecraft:water"));
    fire(&mut world, stack("minecraft:glass_bottle", 1));
    let filled = held(&mut world);
    assert_eq!(filled.item_id, stack("minecraft:potion", 1).item_id);
}

/// The flint-and-steel behaviour sets fire in front and wears the tool.
#[test]
fn a_dispenser_lights_fire_with_flint_and_steel() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    fire(&mut world, stack("minecraft:flint_and_steel", 1));
    assert_eq!(block_at(&mut world, FRONT), Some("minecraft:fire"));
    let tool = held(&mut world);
    assert_eq!(tool.count, 1);
    assert_eq!(
        crate::item_durability::damage_and_max(&tool).map(|(damage, _)| damage),
        Some(1)
    );
}

/// The bone-meal behaviour grows the crop in front.
#[test]
fn a_dispenser_bonemeals_a_crop() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, super::default_state("minecraft:farmland"));
    world.set(FRONT, state_of("minecraft:wheat", &[("age", "0")]));
    fire(&mut world, stack("minecraft:bone_meal", 2));
    let age = bbb_server_types::block_queries::int_property(world.state(FRONT), "age");
    assert!(age.is_some_and(|age| age > 0), "{age:?}");
    assert_eq!(held(&mut world).count, 1);
}

#[test]
fn a_dispenser_bonemeals_a_waterlogged_small_dripleaf() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, super::default_state("minecraft:clay"));
    world.set(
        FRONT,
        state_of(
            "minecraft:small_dripleaf",
            &[
                ("half", "lower"),
                ("facing", "east"),
                ("waterlogged", "true"),
            ],
        ),
    );
    let upper = BlockPos { y: 65, ..FRONT };
    world.set(
        upper,
        state_of(
            "minecraft:small_dripleaf",
            &[
                ("half", "upper"),
                ("facing", "east"),
                ("waterlogged", "true"),
            ],
        ),
    );
    world.set(BlockPos { y: 66, ..FRONT }, stone());

    fire(&mut world, stack("minecraft:bone_meal", 2));

    assert_eq!(
        world.state(FRONT),
        state_of(
            "minecraft:big_dripleaf_stem",
            &[("facing", "east"), ("waterlogged", "true")]
        )
    );
    assert_eq!(
        world.state(upper),
        state_of(
            "minecraft:big_dripleaf",
            &[
                ("facing", "east"),
                ("waterlogged", "true"),
                ("tilt", "none")
            ]
        )
    );
    assert_eq!(held(&mut world).count, 1);
}

#[test]
fn a_dispenser_bonemeals_glow_lichen_once() {
    use bbb_server_worldgen::multiface::{has_face, DIRECTIONS};

    let mut world = NativeWorld::ticking(DROPPER);
    for x in -1..=1 {
        for z in 0..=2 {
            world.set(BlockPos { x, y: 63, z }, stone());
        }
    }
    let lichen = crate::fluid::with_property(
        super::default_state("minecraft:glow_lichen"),
        "down",
        "true",
    )
    .unwrap();
    world.set(FRONT, lichen);
    fire(&mut world, stack("minecraft:bone_meal", 2));
    let mut attachments = 0;
    for x in -1..=1 {
        for y in 63..=65 {
            for z in 0..=2 {
                let state = world.state(BlockPos { x, y, z });
                if vanilla_block_state_identity(state)
                    .is_some_and(|identity| identity.name == "minecraft:glow_lichen")
                {
                    attachments += DIRECTIONS
                        .into_iter()
                        .filter(|&face| has_face(state, face))
                        .count();
                }
            }
        }
    }
    assert_eq!(attachments, 2);
    assert_eq!(held(&mut world).count, 1);
}

#[test]
fn a_dispenser_bonemeals_moss_carpet() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    world.set(BlockPos { y: 65, ..DROPPER }, stone());
    world.set(
        FRONT,
        state_of(
            "minecraft:pale_moss_carpet",
            &[
                ("bottom", "true"),
                ("north", "low"),
                ("east", "none"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    );
    fire(&mut world, stack("minecraft:bone_meal", 2));
    let topper = world.state(BlockPos { y: 65, ..FRONT });
    assert_eq!(
        crate::block_behavior::bool_property(topper, "bottom"),
        Some(false)
    );
    assert_eq!(
        crate::block_behavior::string_property(topper, "north").as_deref(),
        Some("low")
    );
    assert_eq!(held(&mut world).count, 1);
}

#[test]
fn a_dispenser_bonemeals_full_water() {
    let mut world = NativeWorld::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "alice", 1);
    world.set(UNDER_FRONT, stone());
    world.set(FRONT, super::default_state("minecraft:water"));
    world.set(FLOOR, stone());
    world.set(DROPPER, dispenser("south", false));
    let mut slots = empty_nine();
    slots[0] = stack("minecraft:bone_meal", 2);
    set_items(&mut world, DROPPER, slots);
    let redstone = super::default_state("minecraft:redstone_block");
    world.set(SIDE, redstone);
    world.neighbor_changed(DROPPER, block_type_of(redstone).unwrap());
    let mut growth_event = false;
    for _ in 0..5 {
        let tick = world.world.tick(receiver.drain());
        growth_event |= tick.events.iter().any(|event| {
            matches!(event,
                crate::world::WorldEvent::Send {
                    packet: crate::connection::ClientboundPlayPacket::LevelEvent(update), ..
                } if update.event_type == 1505 && update.pos == FRONT && update.data == 15
            )
        });
    }
    assert_eq!(block_at(&mut world, FRONT), Some("minecraft:seagrass"));
    assert_eq!(held(&mut world).count, 1);
    assert!(growth_event);
}

/// The TNT behaviour primes TNT in front.
#[test]
fn a_dispenser_primes_tnt() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    fire(&mut world, stack("minecraft:tnt", 1));
    assert_eq!(
        entity_count(
            &world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID
        ),
        1
    );
    assert_eq!(held(&mut world).count, 0);
}

/// `ShearsDispenseItemBehavior` shears the sheep in front.
#[test]
fn a_dispenser_shears_a_sheep() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    world.command("summon minecraft:sheep 0.5 64 1.5");
    fire(&mut world, stack("minecraft:shears", 1));
    assert!(world.items().iter().any(|(id, _)| id.ends_with("_wool")));
    assert_eq!(held(&mut world).count, 1);
}

#[test]
fn a_dispenser_shears_a_snow_golem() {
    use crate::connection::ClientboundPlayPacket;
    use crate::world::WorldEvent;
    use bbb_protocol::packets::{SoundEventHolder, SoundSource};
    let mut world = NativeWorld::new();
    let mut startup = crate::registry_sync::builtin_datapack_startup_shared();
    let registry = std::sync::Arc::make_mut(&mut startup)
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    let element = registry
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:shearing/snow_golem")
        .unwrap();
    let bbb_server_datapack::loot::LootElementValue::Table(table) = &mut element.value else {
        panic!("the snow golem shearing element is a table")
    };
    table.pools[0].conditions = vec![bbb_server_datapack::loot::parse_loot_condition(
        &serde_json::json!({
            "condition": "minecraft:location_check",
            "predicate": { "block": { "blocks": "minecraft:air" } }
        }),
        None,
    )
    .unwrap()];
    world.world = crate::world::ServerWorld::new(crate::world::ServerWorldConfig {
        datapacks: Some(startup),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "viewer", 1);
    world.set(FLOOR, stone());
    world.set(UNDER_FRONT, stone());
    crate::tests::console_command(&sender, &receiver, &mut world.world, 1, "forceload add 0 0");
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut world.world,
        2,
        "gamerule minecraft:mob_griefing false",
    );
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut world.world,
        3,
        "summon minecraft:snow_golem 0.5 64 1.5",
    );
    world.set(DROPPER, dispenser("south", false));
    let mut slots = empty_nine();
    slots[0] = stack("minecraft:shears", 1);
    set_items(&mut world, DROPPER, slots);
    let redstone = super::default_state("minecraft:redstone_block");
    world.set(SIDE, redstone);
    world.neighbor_changed(DROPPER, block_type_of(redstone).unwrap());
    let mut heard = false;
    for _ in 0..5 {
        let tick = world.world.tick(receiver.drain());
        heard |= tick.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if sound.source == SoundSource::Blocks
                    && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                        if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                            == Some("minecraft:entity.snow_golem.shear"))
            )
        });
    }
    assert!(heard);
    assert_eq!(
        world.items(),
        vec![("minecraft:carved_pumpkin".to_owned(), 1)]
    );
    assert_eq!(held(&mut world).count, 1);
    assert_eq!(held(&mut world).component_patch.damage, Some(1));
}

/// `ShulkerBoxDispenseBehavior` places the box in front.
#[test]
fn a_dispenser_places_a_shulker_box() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    fire(&mut world, stack("minecraft:red_shulker_box", 1));
    assert_eq!(
        block_at(&mut world, FRONT),
        Some("minecraft:red_shulker_box")
    );
    assert_eq!(held(&mut world).count, 0);
}

/// The carved-pumpkin behaviour completes a snow golem.
#[test]
fn a_dispenser_completes_a_snow_golem() {
    let mut world = NativeWorld::ticking(DROPPER);
    let snow = super::default_state("minecraft:snow_block");
    world.set(BlockPos { x: 0, y: 62, z: 1 }, snow);
    world.set(UNDER_FRONT, snow);
    fire(&mut world, stack("minecraft:carved_pumpkin", 1));
    assert_eq!(
        entity_count(
            &world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
        ),
        1
    );
    assert_eq!(held(&mut world).count, 0);
}

/// `MinecartDispenseItemBehavior` sets a cart on the rail in front, and
/// `BoatDispenseItemBehavior` a boat on the water in front.
#[test]
fn a_dispenser_places_minecarts_and_boats() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    };
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    world.set(FRONT, super::default_state("minecraft:rail"));
    fire(&mut world, stack("minecraft:minecart", 1));
    assert_eq!(entity_count(&world, VANILLA_ENTITY_TYPE_MINECART_ID), 1);
    assert_eq!(held(&mut world).count, 0);

    let mut world = NativeWorld::ticking(DROPPER);
    world.set(UNDER_FRONT, stone());
    world.set(FRONT, super::default_state("minecraft:water"));
    fire(&mut world, stack("minecraft:oak_boat", 1));
    assert_eq!(entity_count(&world, VANILLA_ENTITY_TYPE_OAK_BOAT_ID), 1);
    assert_eq!(held(&mut world).count, 0);
}

/// A behaviour that cannot act (flint and steel against stone) keeps the
/// item and clicks `1001`.
#[test]
fn a_failed_optional_behaviour_keeps_the_item() {
    let mut world = NativeWorld::ticking(DROPPER);
    world.set(FRONT, stone());
    fire(&mut world, stack("minecraft:flint_and_steel", 1));
    assert_eq!(held(&mut world), stack("minecraft:flint_and_steel", 1));
    assert!(world.items().is_empty());
}

#[test]
fn dispenser_honey_harvest_releases_only_when_the_hive_environment_and_exit_allow_it() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};
    use bbb_server_types::save::Java26BuiltinDimension;
    let dim = Java26BuiltinDimension::Overworld;
    let field = |name: &str, value| Entry {
        name: name.into(),
        value,
    };
    for item in ["minecraft:shears", "minecraft:glass_bottle"] {
        for condition in ["day", "night", "rain", "blocked", "override"] {
            let mut world = NativeWorld::ticking(DROPPER);
            if condition == "override" {
                let mut config = crate::tests::datapack_config();
                let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
                let dimension = startup
                    .registries
                    .registries
                    .iter_mut()
                    .find(|r| r.registry_id == "minecraft:dimension_type")
                    .unwrap()
                    .elements
                    .iter_mut()
                    .find(|e| e.id == "minecraft:overworld")
                    .unwrap();
                let mut value: serde_json::Value =
                    serde_json::from_slice(&dimension.source_json).unwrap();
                value["attributes"]["minecraft:gameplay/bees_stay_in_hive"] =
                    serde_json::json!(true);
                dimension.source_json = serde_json::to_vec(&value).unwrap().into();
                dimension.source = Some(
                    bbb_registry_codec::decode_direct_registry_json(
                        &serde_json::to_vec(&(value)).unwrap(),
                    )
                    .unwrap(),
                );
                world.world = crate::world::ServerWorld::new(config).unwrap();
                world.command("forceload add 0 0");
            }
            world.command(if condition == "night" {
                "time set midnight"
            } else {
                "time set day"
            });
            world.command(if condition == "rain" {
                "weather rain"
            } else {
                "weather clear"
            });
            world.set(
                FRONT,
                state_of(
                    "minecraft:beehive",
                    &[("facing", "east"), ("honey_level", "5")],
                ),
            );
            world.set(
                BlockPos { x: 1, ..FRONT },
                if condition == "blocked" {
                    stone()
                } else {
                    super::air()
                },
            );
            let saved = Nbt::Compound(vec![
                field(
                    "entity_data",
                    Nbt::Compound(vec![
                        field("id", Nbt::String("minecraft:bee".into())),
                        field(
                            "CustomName",
                            Nbt::String("{\"text\":\"hive occupant\"}".into()),
                        ),
                    ]),
                ),
                field("ticks_in_hive", Nbt::Int(37)),
                field("min_ticks_in_hive", Nbt::Int(600)),
            ]);
            world
                .world
                .level_block_entity_for_test(dim, FRONT)
                .unwrap()
                .beehive_mut()
                .unwrap()
                .load_additional(&[field("bees", Nbt::List(vec![saved.clone()]))]);
            fire(&mut world, stack(item, 1));
            assert_eq!(
                crate::block_behavior::beehive::honey_level(world.state(FRONT)),
                0,
                "{item}: {condition}"
            );
            let retained = world
                .world
                .level_block_entity_for_test(dim, FRONT)
                .unwrap()
                .beehive()
                .unwrap()
                .release_entries();
            let bees: Vec<_> = world
                .world
                .stored_entity_ids_for_test(dim)
                .into_iter()
                .filter(|id| {
                    world
                        .world
                        .stored_entity_for_test(dim, *id)
                        .is_some_and(|e| e.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID)
                })
                .collect();
            if condition == "day" {
                assert!(retained.is_empty());
                assert_eq!(bees.len(), 1, "{item}: {condition}");
            } else {
                assert_eq!(retained.len(), 1, "{item}: {condition}");
                let occupant = retained[0].as_ref().unwrap();
                assert!(occupant.ticks_in_hive > 37);
                let mut expected = saved.clone();
                let Nbt::Compound(entries) = &mut expected else {
                    unreachable!()
                };
                entries
                    .iter_mut()
                    .find(|entry| entry.name == "ticks_in_hive")
                    .unwrap()
                    .value = Nbt::Int(occupant.ticks_in_hive);
                assert_eq!(occupant.saved, expected);
                assert!(bees.is_empty(), "{item}: {condition}");
            }
            if item == "minecraft:glass_bottle" {
                assert_eq!(held(&mut world), stack("minecraft:honey_bottle", 1));
            } else {
                let combs: i32 = world
                    .items()
                    .into_iter()
                    .filter(|(id, _)| id == "minecraft:honeycomb")
                    .map(|(_, count)| count)
                    .sum();
                assert_eq!(combs, 3);
            }
        }
    }
}

/// Dispensed TNT and shulker boxes work in a datapack level too, and the
/// placed box takes the item's `block_state` (`BlockItem.java:120-132`).
#[test]
fn a_dispenser_in_a_datapack_level_primes_tnt_and_places_a_stated_shulker_box() {
    use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
    let mut candidate = crate::tests::restore_candidate(0, None, Vec::new());
    let custom = Java26LevelKey::parse("example:moon").unwrap();
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let stem = candidate.world_gen_settings.dimensions[&overworld].clone();
    candidate
        .world_gen_settings
        .dimensions
        .insert(custom.clone(), stem);
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let mut world =
        crate::world::ServerWorld::restore(crate::world::ServerWorldConfig::default(), &candidate)
            .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    for (id, command) in [
        "forceload add 0 0",
        "setblock 0 64 0 dispenser[facing=south]{Items:[{Slot:0b,id:\"minecraft:tnt\",count:1}]}",
        "setblock 4 64 0 dispenser[facing=south]{Items:[{Slot:0b,id:\"minecraft:red_shulker_box\",count:1,components:{\"minecraft:block_state\":{facing:\"down\"}}}]}",
        "setblock 1 64 0 minecraft:redstone_block",
        "setblock 5 64 0 minecraft:redstone_block",
    ]
    .into_iter()
    .enumerate()
    {
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut world,
            id as u64 + 1,
            &format!("execute in example:moon run {command}"),
        );
    }
    for _ in 0..5 {
        world.tick(receiver.drain());
    }
    let level = world.require_level_id(&custom);
    assert_eq!(
        world.block_state(level, BlockPos { x: 4, y: 64, z: 1 }),
        Some(state_of("minecraft:red_shulker_box", &[("facing", "down")]))
    );
    let primed = crate::tests::console_command(
        &sender,
        &receiver,
        &mut world,
        10,
        "execute in example:moon if entity @e[type=minecraft:tnt]",
    );
    assert_eq!(crate::tests::console_command_answer(&primed).0, Some(1));
}
