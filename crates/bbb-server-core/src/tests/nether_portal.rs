use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_OBSIDIAN_ID, VANILLA_BLOCK_STONE_ID};

use super::*;
mod end;
mod gateway;
mod player;
fn portal_state(axis: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:nether_portal",
        &BTreeMap::from([("axis".to_string(), axis.to_string())]),
    )
    .expect("built-in nether portal state")
}

fn set_portal_blocks(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    positions: &[(BlockPos, i32)],
) {
    for &(pos, state) in positions {
        world.ensure_entity_transfer_full_chunk_for_test(dimension, (pos.x >> 4, pos.z >> 4));
        world.set_block(
            dimension,
            pos,
            state,
            crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
        );
    }
}

fn untouched_portal_book(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
) -> (BlockPos, bbb_server_types::item_stack::RuntimeItemStack) {
    let pos = BlockPos { x: 8, y: 80, z: 8 };
    let state = vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_CHISELED_BOOKSHELF_ID,
    )
    .unwrap();
    set_portal_blocks(world, dimension, &[(pos, state)]);
    let body = world.level_block_entity_for_test(dimension, pos).unwrap();
    let shelf = body.chiseled_bookshelf_mut().unwrap();
    assert!(shelf.set_slot(
        0,
        crate::tests::native_stack(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        }),
    ));
    (pos, shelf.original_items()[0].clone())
}

fn assert_portal_book_original(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    pos: BlockPos,
    original: &bbb_server_types::item_stack::RuntimeItemStack,
) {
    let shelf = world
        .level_block_entity_for_test(dimension, pos)
        .unwrap()
        .chiseled_bookshelf()
        .unwrap();
    assert_eq!(&shelf.original_items()[0], original);
    assert_eq!(original.count(), 1);
}

fn live_state(world: &ServerWorld, dimension: Java26BuiltinDimension, pos: BlockPos) -> i32 {
    world
        .chunk_store_for_test(dimension)
        .get_block_state(pos.x, pos.y, pos.z)
        .expect("the portal fixture cell is loaded")
}

fn creation_pos(
    origin: BlockPos,
    axis: crate::world::nether_portal::PortalAxis,
    width: i32,
    height: i32,
    perpendicular: i32,
) -> BlockPos {
    match axis {
        crate::world::nether_portal::PortalAxis::X => BlockPos {
            x: origin.x + width,
            y: origin.y + height,
            z: origin.z + perpendicular,
        },
        crate::world::nether_portal::PortalAxis::Z => BlockPos {
            x: origin.x - perpendicular,
            y: origin.y + height,
            z: origin.z + width,
        },
    }
}

fn portal_rectangle_cells(
    min_corner: BlockPos,
    axis: crate::world::nether_portal::PortalAxis,
) -> Vec<(BlockPos, i32)> {
    let state = portal_state(match axis {
        crate::world::nether_portal::PortalAxis::X => "x",
        crate::world::nether_portal::PortalAxis::Z => "z",
    });
    (0..2)
        .flat_map(|width| {
            (0..3).map(move |height| (creation_pos(min_corner, axis, width, height, 0), state))
        })
        .collect()
}

fn ensure_portal_rectangle_window(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    entry: BlockPos,
    axis: crate::world::nether_portal::PortalAxis,
) {
    for horizontal in -21..=21 {
        let pos = creation_pos(entry, axis, horizontal, 0, 0);
        world.ensure_entity_transfer_full_chunk_for_test(dimension, (pos.x >> 4, pos.z >> 4));
    }
}

fn nearby_portal_cell_count(world: &ServerWorld, dimension: Java26BuiltinDimension) -> usize {
    let x_state = portal_state("x");
    let z_state = portal_state("z");
    let chunks = world.chunk_store_for_test(dimension);
    (-20..=20)
        .flat_map(|x| (-20..=20).map(move |z| (x, z)))
        .flat_map(|(x, z)| (50..=90).map(move |y| (x, y, z)))
        .filter(|&(x, y, z)| {
            chunks
                .get_block_state(x, y, z)
                .is_some_and(|state| state == x_state || state == z_state)
        })
        .count()
}

#[test]
fn overworld_to_nether_scales_then_loads_the_complete_search_window() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let portal = BlockPos { x: 10, y: 64, z: 2 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[(portal, portal_state("x"))],
    );

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 80.0,
                y: 64.0,
                z: 16.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    assert_eq!(found.target.builtin(), Some(Java26BuiltinDimension::Nether));
    assert_eq!(found.approximate_exit, portal);
    assert_eq!(found.found_position, Some(portal));
    let rectangle = found.rectangle.unwrap();
    assert_eq!(rectangle.min_corner, portal);
    assert_eq!(rectangle.axis, crate::world::nether_portal::PortalAxis::X);
    assert_eq!((rectangle.width, rectangle.height), (1, 1));
    // POI-driven search loads only the candidate chunk and its rectangle
    // read window, not the entire 5x5 search square.
    assert!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .loaded_chunks()
            .count()
            <= 4
    );
}

#[test]
fn portal_at_the_search_edge_loads_its_complete_rectangle_read_window() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let portal = BlockPos {
        x: -16,
        y: 64,
        z: 0,
    };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[(portal, portal_state("x"))],
    );

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    assert_eq!(found.found_position, Some(portal));
    assert!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .loaded_chunks()
            .count()
            <= 5
    );
    assert!(world
        .chunk_store_for_test(Java26BuiltinDimension::Nether)
        .column(-3, 0)
        .is_some());
}

#[test]
fn nether_to_overworld_uses_radius_128_and_builds_the_exact_state_rectangle() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let min = BlockPos { x: 80, y: 63, z: 0 };
    let state = portal_state("x");
    let mut positions = Vec::new();
    for x in 80..82 {
        for y in 63..66 {
            positions.push((BlockPos { x, y, z: 0 }, state));
        }
    }
    set_portal_blocks(&mut world, Java26BuiltinDimension::Overworld, &positions);

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Nether)),
            Vec3d {
                x: 10.0,
                y: 64.0,
                z: 0.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    assert_eq!(found.approximate_exit, BlockPos { x: 80, y: 64, z: 0 });
    let rectangle = found.rectangle.unwrap();
    assert_eq!(rectangle.min_corner, min);
    assert_eq!(rectangle.axis, crate::world::nether_portal::PortalAxis::X);
    assert_eq!((rectangle.width, rectangle.height), (2, 3));
    // Overworld return search (radius 128) uses POI lookup instead of
    // synchronously generating all 19x19 = 361 chunks.
    assert!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .loaded_chunks()
            .count()
            <= 5
    );
}

#[test]
fn irregular_connected_portal_uses_the_largest_area_rectangle() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let state = portal_state("x");
    let positions = [
        BlockPos { x: 0, y: 63, z: 0 },
        BlockPos { x: 0, y: 64, z: 0 },
        BlockPos { x: 0, y: 65, z: 0 },
        BlockPos { x: 1, y: 64, z: 0 },
        BlockPos { x: 1, y: 65, z: 0 },
        BlockPos { x: 2, y: 64, z: 0 },
    ]
    .map(|pos| (pos, state));
    set_portal_blocks(&mut world, Java26BuiltinDimension::Nether, &positions);

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    let rectangle = found.rectangle.unwrap();
    assert_eq!(rectangle.min_corner, BlockPos { x: 0, y: 64, z: 0 });
    assert_eq!((rectangle.width, rectangle.height), (2, 2));
}

#[test]
fn closest_portal_uses_distance_before_y_and_ignores_the_other_axis_state() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let farther_low = BlockPos { x: 4, y: 60, z: 0 };
    let nearer_high = BlockPos { x: 2, y: 66, z: 0 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[
            (farther_low, portal_state("x")),
            (nearer_high, portal_state("z")),
        ],
    );

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    assert_eq!(found.found_position, Some(nearer_high));
    assert_eq!(
        found.rectangle.unwrap().axis,
        crate::world::nether_portal::PortalAxis::Z
    );
}

/// `PortalForcer.java:41-50`: two portals at the same distance and height
/// still yield an exit.
#[test]
fn equidistant_exit_portals_still_yield_one_exit() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let east = BlockPos { x: 3, y: 64, z: 0 };
    let west = BlockPos { x: -3, y: 64, z: 0 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[(east, portal_state("x")), (west, portal_state("x"))],
    );

    let found = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();

    let position = found.found_position.expect("an exit portal");
    assert!(position == east || position == west);
    assert!(found.rectangle.is_some());
}

#[test]
fn living_portal_transition_preserves_relative_position_and_resets_forward_offset() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source_min = BlockPos { x: 0, y: 64, z: 0 };
    let target_min = BlockPos {
        x: 10,
        y: 70,
        z: 10,
    };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Overworld,
        &portal_rectangle_cells(source_min, crate::world::nether_portal::PortalAxis::X),
    );
    ensure_portal_rectangle_window(
        &mut world,
        Java26BuiltinDimension::Overworld,
        source_min,
        crate::world::nether_portal::PortalAxis::X,
    );
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &portal_rectangle_cells(target_min, crate::world::nether_portal::PortalAxis::X),
    );
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.8,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    let transition = world
        .stored_nether_portal_transition_for_test(
            &crate::world::LevelId::from(Java26BuiltinDimension::Overworld),
            entity_id,
            source_min,
            &crate::world::LevelId::from(Java26BuiltinDimension::Nether),
            crate::world::nether_portal::PortalRectangle {
                min_corner: target_min,
                axis: crate::world::nether_portal::PortalAxis::X,
                width: 2,
                height: 3,
            },
        )
        .unwrap();

    assert_eq!(
        transition.position,
        Vec3d {
            x: 10.5,
            y: 70.0,
            z: 10.5,
        }
    );
    assert_eq!(transition.y_rot, 0.0);
    assert_eq!(transition.x_rot, 0.0);
    assert_eq!(transition.delta_movement, Vec3d::default());
    assert_eq!(transition.relatives_mask, 504);
}

#[test]
fn changed_exit_axis_rotates_the_relative_transition() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source_min = BlockPos { x: 0, y: 64, z: 0 };
    let target_min = BlockPos {
        x: 10,
        y: 70,
        z: 10,
    };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Overworld,
        &portal_rectangle_cells(source_min, crate::world::nether_portal::PortalAxis::X),
    );
    ensure_portal_rectangle_window(
        &mut world,
        Java26BuiltinDimension::Overworld,
        source_min,
        crate::world::nether_portal::PortalAxis::X,
    );
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &portal_rectangle_cells(target_min, crate::world::nether_portal::PortalAxis::Z),
    );
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.8,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    let transition = world
        .stored_nether_portal_transition_for_test(
            &crate::world::LevelId::from(Java26BuiltinDimension::Overworld),
            entity_id,
            source_min,
            &crate::world::LevelId::from(Java26BuiltinDimension::Nether),
            crate::world::nether_portal::PortalRectangle {
                min_corner: target_min,
                axis: crate::world::nether_portal::PortalAxis::Z,
                width: 2,
                height: 3,
            },
        )
        .unwrap();

    assert_eq!(
        transition.position,
        Vec3d {
            x: 10.5,
            y: 70.0,
            z: 10.5,
        }
    );
    assert_eq!(transition.y_rot, 90.0);
    assert_eq!(transition.relatives_mask, 504);
}

#[test]
fn nonliving_forward_offset_moves_to_the_closest_collision_free_height() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source_min = BlockPos { x: 0, y: 64, z: 0 };
    let target_min = BlockPos {
        x: 10,
        y: 70,
        z: 10,
    };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Overworld,
        &portal_rectangle_cells(source_min, crate::world::nether_portal::PortalAxis::X),
    );
    ensure_portal_rectangle_window(
        &mut world,
        Java26BuiltinDimension::Overworld,
        source_min,
        crate::world::nether_portal::PortalAxis::X,
    );
    let mut target = portal_rectangle_cells(target_min, crate::world::nether_portal::PortalAxis::X);
    target.push((
        BlockPos {
            x: 10,
            y: 70,
            z: 11,
        },
        vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap(),
    ));
    set_portal_blocks(&mut world, Java26BuiltinDimension::Nether, &target);
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 1.0,
        },
        crate::entity::EntityData::Item(crate::entity::ItemEntityData::new(
            crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
            0.0,
        )),
    );
    let transition = world
        .stored_nether_portal_transition_for_test(
            &crate::world::LevelId::from(Java26BuiltinDimension::Overworld),
            entity_id,
            source_min,
            &crate::world::LevelId::from(Java26BuiltinDimension::Nether),
            crate::world::nether_portal::PortalRectangle {
                min_corner: target_min,
                axis: crate::world::nether_portal::PortalAxis::X,
                width: 2,
                height: 3,
            },
        )
        .unwrap();

    assert_eq!(transition.position.x, 10.5);
    assert_eq!(transition.position.y, 70.0);
    assert_eq!(transition.position.z, 10.875);
}

/// VehicleEntity.java:115-117: a minecart arrives with a 10-tick cooldown.
#[test]
fn a_minecart_through_a_nether_portal_gets_the_vehicle_cooldown() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(source, entry).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &[
            (
                BlockPos { y: 63, ..entry },
                vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap(),
            ),
            (entry, portal_state("x")),
        ],
    );
    let (_, uuid) = world.spawn_stored_entity_for_test(
        source,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
    );
    let mut arrived = None;
    for _ in 0..120 {
        world.tick(Vec::new());
        arrived = world.live_entity_id_for_uuid_for_test(target, uuid);
        if arrived.is_some() {
            break;
        }
    }
    let id = arrived.expect("the minecart reaches the Nether");
    let cooldown = world
        .stored_entity_for_test(target, id)
        .unwrap()
        .portal_cooldown();
    assert!((9..=10).contains(&cooldown), "{cooldown}");
}

#[test]
fn stored_nether_portal_processor_commits_creation_transfer_and_ticket_in_world_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(source, entry).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &[
            (
                BlockPos { y: 63, ..entry },
                vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap(),
            ),
            (entry, portal_state("x")),
        ],
    );
    let (entity_id, uuid) = world.spawn_stored_entity_for_test(
        source,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    world.tick(Vec::new());
    assert!(world.stored_entity_for_test(source, entity_id).is_some());
    world.tick(Vec::new());

    assert!(world.stored_entity_for_test(source, entity_id).is_none());
    let replacement = world
        .stored_entity_ids_for_test(target)
        .into_iter()
        .find_map(|id| {
            world
                .stored_entity_for_test(target, id)
                .filter(|entity| entity.uuid() == uuid)
        })
        .expect("the replacement is published in the Nether");
    assert_eq!(replacement.portal_cooldown(), 300);
    assert_eq!(nearby_portal_cell_count(&world, target), 6);
    let ticket_chunk = (
        replacement.block_position().x >> 4,
        replacement.block_position().z >> 4,
    );
    assert!(world
        .saved_chunk_tickets_for_test(target)
        .iter()
        .any(|(chunk, level, ticks)| *chunk == ticket_chunk
            && *level == 30
            && *ticks == Some(299)));
}

/// `allow_entering_nether_using_portals` false: `getPortalDestination` still
/// builds the Nether exit, `isAllowedToEnterPortal` then refuses the teleport
/// and its ticket (`Entity.java:2546-2551`, `ServerLevel.java:1871-1873`), and
/// the portal cooldown stays set.
#[test]
fn a_refused_nether_entry_builds_the_exit_but_keeps_the_entity() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert!(world.set_game_rule_for_test(
        "minecraft:allow_entering_nether_using_portals",
        GameRuleValue::Boolean(false),
    ));
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    world.place_portal_ticket(source, entry).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &[
            (
                BlockPos { y: 63, ..entry },
                vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap(),
            ),
            (entry, portal_state("x")),
        ],
    );
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        source,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    world.tick(Vec::new());
    world.tick(Vec::new());

    let stayed = world
        .stored_entity_for_test(source, entity_id)
        .expect("the refused entity stays in the Overworld");
    assert_eq!(stayed.portal_cooldown(), 300);
    assert!(world.stored_entity_ids_for_test(target).is_empty());
    assert_eq!(nearby_portal_cell_count(&world, target), 6);
    assert!(world.saved_chunk_tickets_for_test(target).is_empty());
}

#[test]
fn passenger_throwable_contact_transfers_only_requester_in_the_same_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let source_min = BlockPos { x: 0, y: 64, z: 0 };
    let target_min = BlockPos {
        x: 10,
        y: 70,
        z: 10,
    };
    world.place_portal_ticket(source, source_min).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &portal_rectangle_cells(source_min, crate::world::nether_portal::PortalAxis::X),
    );
    set_portal_blocks(
        &mut world,
        target,
        &portal_rectangle_cells(target_min, crate::world::nether_portal::PortalAxis::X),
    );
    let (vehicle_id, requester_ids, sibling_id) = world
        .spawn_stored_throwable_passengers_for_test(
            source,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            1,
            true,
        )
        .expect("the forced passenger tree is published");
    let requester_id = requester_ids[0];
    let sibling_id = sibling_id.expect("the trailing sibling is published");
    let requester_uuid = world
        .stored_entity_for_test(source, requester_id)
        .unwrap()
        .uuid();

    world.tick(Vec::new());

    assert!(world.stored_entity_for_test(source, vehicle_id).is_some());
    assert!(world.stored_entity_for_test(source, requester_id).is_none());
    assert_eq!(
        world.stored_passengers_for_test(source, vehicle_id),
        &[sibling_id]
    );
    assert_eq!(
        world
            .stored_entity_for_test(source, sibling_id)
            .expect("the later sibling remains in the source")
            .tick_count(),
        1
    );
    let replacement = world
        .stored_entity_ids_for_test(target)
        .into_iter()
        .find_map(|id| {
            world
                .stored_entity_for_test(target, id)
                .filter(|entity| entity.uuid() == requester_uuid)
        })
        .expect("the passenger requester is published in the Nether");
    assert!(replacement.portal_cooldown() > 0);
}

#[test]
fn two_passenger_throwables_transfer_in_preorder_during_one_root_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let source_min = BlockPos { x: 0, y: 64, z: 0 };
    let target_min = BlockPos {
        x: 10,
        y: 70,
        z: 10,
    };
    world.place_portal_ticket(source, source_min).unwrap();
    set_portal_blocks(
        &mut world,
        source,
        &portal_rectangle_cells(source_min, crate::world::nether_portal::PortalAxis::X),
    );
    set_portal_blocks(
        &mut world,
        target,
        &portal_rectangle_cells(target_min, crate::world::nether_portal::PortalAxis::X),
    );
    let (vehicle_id, requester_ids, sibling_id) = world
        .spawn_stored_throwable_passengers_for_test(
            source,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            2,
            false,
        )
        .expect("both forced passengers are published");
    assert!(sibling_id.is_none());
    let requester_uuids = requester_ids
        .iter()
        .map(|id| world.stored_entity_for_test(source, *id).unwrap().uuid())
        .collect::<Vec<_>>();

    world.tick(Vec::new());

    assert!(world.stored_entity_for_test(source, vehicle_id).is_some());
    assert!(world
        .stored_passengers_for_test(source, vehicle_id)
        .is_empty());
    for (requester_id, requester_uuid) in requester_ids.into_iter().zip(requester_uuids) {
        assert!(world.stored_entity_for_test(source, requester_id).is_none());
        let replacement = world
            .stored_entity_ids_for_test(target)
            .into_iter()
            .find_map(|id| {
                world
                    .stored_entity_for_test(target, id)
                    .filter(|entity| entity.uuid() == requester_uuid)
            })
            .expect("each passenger requester is published in the Nether");
        assert!(replacement.portal_cooldown() > 0);
    }
}

#[test]
fn missing_exit_creates_source_axis_portal_with_the_complete_frame() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Overworld,
        &[(entry, portal_state("z"))],
    );
    let (book_pos, book_original) =
        untouched_portal_book(&mut world, Java26BuiltinDimension::Nether);
    let mut search = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            entry,
        )
        .unwrap();
    assert_eq!(
        search.source_axis,
        crate::world::nether_portal::PortalAxis::Z
    );
    assert!(search.rectangle.is_none());

    let staged = world
        .prepare_nether_portal_creation_for_test(&mut search)
        .unwrap()
        .expect("the flat target can host a portal");
    let rectangle = staged.rectangle;
    let portal = portal_state("z");
    assert_eq!(staged.write_trace.len(), 20);
    assert!(staged.write_trace[..14]
        .iter()
        .all(|(_, _, flags)| *flags == crate::fluid::SET_BLOCK_UPDATE_ALL));
    assert!(staged.write_trace[14..]
        .iter()
        .all(|(_, _, flags)| *flags == 18));
    assert_eq!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Nether)
            .get_block_state(
                rectangle.min_corner.x,
                rectangle.min_corner.y,
                rectangle.min_corner.z,
            ),
        Some(portal)
    );
    assert!(world
        .saved_chunk_tickets_for_test(Java26BuiltinDimension::Nether)
        .is_empty());

    assert_portal_book_original(
        &mut world,
        Java26BuiltinDimension::Nether,
        book_pos,
        &book_original,
    );
    let obsidian = vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap();
    for width in -1..3 {
        for height in -1..4 {
            let pos = creation_pos(rectangle.min_corner, rectangle.axis, width, height, 0);
            if width == -1 || width == 2 || height == -1 || height == 3 {
                assert_eq!(
                    live_state(&world, Java26BuiltinDimension::Nether, pos),
                    obsidian
                );
            } else {
                assert_eq!(
                    live_state(&world, Java26BuiltinDimension::Nether, pos),
                    portal
                );
            }
        }
    }
    assert!(world
        .saved_chunk_tickets_for_test(Java26BuiltinDimension::Nether)
        .is_empty());
}

#[test]
fn creation_prefers_a_farther_full_site_over_the_origin_partial_site() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let obstruction = BlockPos { x: 0, y: 64, z: 1 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[(
            obstruction,
            vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap(),
        )],
    );
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    let mut search = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            entry,
        )
        .unwrap();

    let staged = world
        .prepare_nether_portal_creation_for_test(&mut search)
        .unwrap()
        .expect("the surrounding flat terrain has full sites");

    assert_eq!(staged.rectangle.min_corner, BlockPos { x: 0, y: 64, z: -1 });
    assert_ne!(staged.rectangle.min_corner, entry);
}

#[test]
fn narrow_border_creates_fallback_platform_in_place() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.set_world_border_for_test(
        Java26BuiltinDimension::Nether,
        bbb_server_types::interaction::WorldBorder::from_settings(
            0.0, 0.0, 0.2, 5.0, 5, 15, 1.0, 0, 1.0, 0,
        ),
    );
    let entry = BlockPos { x: 0, y: 64, z: 0 };
    let cleared = BlockPos { x: -1, y: 70, z: 1 };
    set_portal_blocks(
        &mut world,
        Java26BuiltinDimension::Nether,
        &[(
            cleared,
            vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap(),
        )],
    );
    let mut search = world
        .prepare_nether_portal_search(
            &world.require_level_id(&level_key(Java26BuiltinDimension::Overworld)),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            entry,
        )
        .unwrap();
    let staged = world
        .prepare_nether_portal_creation_for_test(&mut search)
        .unwrap()
        .expect("the fallback height exists");
    assert_eq!(staged.rectangle.min_corner, BlockPos { x: -1, y: 70, z: 0 });
    assert_eq!(staged.write_trace.len(), 44);
    assert!(staged.write_trace[..38]
        .iter()
        .all(|(_, _, flags)| *flags == crate::fluid::SET_BLOCK_UPDATE_ALL));
    assert!(staged.write_trace[38..]
        .iter()
        .all(|(_, _, flags)| *flags == 18));
    let rectangle = staged.rectangle;
    let obsidian = vanilla_block_default_state_id(VANILLA_BLOCK_OBSIDIAN_ID).unwrap();
    assert_eq!(
        live_state(&world, Java26BuiltinDimension::Nether, cleared),
        bbb_server_types::fluid::air_block_state_id()
    );
    assert_eq!(
        live_state(
            &world,
            Java26BuiltinDimension::Nether,
            creation_pos(rectangle.min_corner, rectangle.axis, 0, -1, 1),
        ),
        obsidian
    );
}

#[test]
fn portal_ticks_carry_an_online_rider_and_publish_the_destination_seat() {
    use crate::entity::portal::PortalKind;
    use crate::world::nether_portal::PortalAxis;
    use bbb_protocol::packets::movement::{
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
        PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
    };

    for kind in [PortalKind::Nether, PortalKind::End, PortalKind::EndGateway] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "rider", 1);
        drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
        join_loaded(&sender, &receiver, &mut world, 2, "observer", 2);
        let source = Java26BuiltinDimension::Overworld;
        let target = match kind {
            PortalKind::Nether => Java26BuiltinDimension::Nether,
            PortalKind::End => Java26BuiltinDimension::End,
            PortalKind::EndGateway => source,
        };
        let entry = BlockPos { x: 0, y: 64, z: 0 };
        let exit = match kind {
            PortalKind::Nether => BlockPos { x: 0, y: 64, z: 0 },
            PortalKind::End => BlockPos {
                x: 100,
                y: 50,
                z: 0,
            },
            PortalKind::EndGateway => BlockPos { x: 64, y: 64, z: 0 },
        };
        assert!(world.teleport_player(
            ConnectionId(2),
            crate::world::TeleportTransition {
                new_level: target.into(),
                position: Vec3d {
                    x: f64::from(exit.x) + 0.5,
                    y: f64::from(exit.y),
                    z: 0.5
                },
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut Vec::new()
        ));
        drain_chunk_deliveries(&sender, &receiver, &mut world, 2);
        world.place_portal_ticket(source, entry).unwrap();
        let state = match kind {
            PortalKind::Nether => {
                world.place_portal_ticket(target, exit).unwrap();
                set_portal_blocks(
                    &mut world,
                    target,
                    &portal_rectangle_cells(exit, PortalAxis::Z),
                );
                portal_state("x")
            }
            PortalKind::End => vanilla_block_state_id_by_name_and_properties(
                "minecraft:end_portal",
                &BTreeMap::new(),
            )
            .unwrap(),
            PortalKind::EndGateway => vanilla_block_state_id_by_name_and_properties(
                "minecraft:end_gateway",
                &BTreeMap::new(),
            )
            .unwrap(),
        };
        set_portal_blocks(&mut world, source, &[(entry, state)]);
        if kind == PortalKind::EndGateway {
            world
                .level_block_entity_for_test(source, entry)
                .unwrap()
                .end_gateway_mut()
                .unwrap()
                .set_exit_position(exit, true);
        }
        let (vehicle, uuid) = world.spawn_stored_entity_for_test(
            source,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
        );
        end::clear_cooldown(&mut world, source, vehicle);
        assert!(world.attach_external_player_for_test(ConnectionId(1), vehicle));
        let mut events = world.tick(receiver.drain()).events;
        events.extend(world.tick(receiver.drain()).events);
        assert_eq!(world.player_dimension(ConnectionId(1)), target, "{kind:?}");
        let arrived = world
            .live_entity_id_for_uuid_for_test(target, uuid)
            .expect("the ridden vehicle reaches its portal destination");
        let (_, _, mounted, root) = world.external_player_vehicle_state_for_test(ConnectionId(1));
        assert_eq!((mounted, root), (Some(arrived), Some(arrived)), "{kind:?}");
        let player_id = world.player_entity_id_for_test(ConnectionId(1));
        assert_eq!(
            world.stored_passengers_for_test(target, arrived),
            [player_id]
        );
        if source == target {
            assert!(events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::TeleportEntity(update), }
                    if update.id == arrived && update.relatives_mask == 0 && (update.position.x - 64.5).abs() < 0.01
            )), "the non-controlling rider receives the vehicle's absolute arrival");
            assert_eq!(arrived, vehicle);
            assert!(
                (world
                    .stored_entity_for_test(target, arrived)
                    .unwrap()
                    .position()
                    .x
                    - 64.5)
                    .abs()
                    < 0.01
            );
        } else {
            assert!(world.stored_entity_for_test(source, vehicle).is_none());
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::Respawn(_),
                    }
                )),
                "{kind:?}"
            );
        }
        let mut relatives = PLAYER_RELATIVE_DELTA_X
            | PLAYER_RELATIVE_DELTA_Y
            | PLAYER_RELATIVE_DELTA_Z
            | PLAYER_RELATIVE_ROTATE_DELTA
            | PLAYER_RELATIVE_X_ROT;
        if kind != PortalKind::End {
            relatives |= PLAYER_RELATIVE_Y_ROT;
        }
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::PlayerPosition(update), }
                if update.relatives_mask == relatives
        )), "{kind:?}: the rider receives the portal rotation and velocity rules");
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(2), packet: ClientboundPlayPacket::SetPassengers(update), }
                if update.vehicle_id == arrived && update.passenger_ids == [player_id]
        )), "{kind:?}: the destination observer sees the seat");
        assert_eq!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::LevelEvent(event), }
                if event.event_type == 1032
        )), kind != PortalKind::EndGateway, "{kind:?}");
        if kind == PortalKind::End {
            let exit = world
                .stored_entity_for_test(target, arrived)
                .unwrap()
                .block_position();
            set_portal_blocks(
                &mut world,
                target,
                &[(
                    exit,
                    vanilla_block_state_id_by_name_and_properties(
                        "minecraft:end_portal",
                        &BTreeMap::new(),
                    )
                    .unwrap(),
                )],
            );
            end::clear_cooldown(&mut world, target, arrived);
            assert_eq!(
                world
                    .stored_entity_for_test(target, arrived)
                    .unwrap()
                    .portal_cooldown(),
                0
            );
            let mut refused = Vec::new();
            for _ in 0..10 {
                refused.extend(world.tick(receiver.drain()).events);
                if world
                    .stored_entity_for_test(target, arrived)
                    .is_none_or(|entity| entity.portal_cooldown() >= 290)
                {
                    break;
                }
            }
            assert!(
                world
                    .stored_entity_for_test(target, arrived)
                    .is_some_and(|entity| entity.portal_cooldown() >= 290),
                "the mounted vehicle attempted the End exit"
            );
            assert_eq!(world.player_dimension(ConnectionId(1)), target);
            assert!(
                world.stored_entity_for_test(target, arrived).is_some(),
                "the vehicle cannot take a new player past the first End credits"
            );
            assert_eq!(
                world
                    .external_player_vehicle_state_for_test(ConnectionId(1))
                    .2,
                Some(arrived)
            );
            assert!(!refused.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::Respawn(_),
                }
            )));
        }
    }
}
