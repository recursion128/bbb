use std::collections::BTreeSet;

use bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT;
use bbb_protocol::block_types::VANILLA_BLOCK_TYPES;

use super::{BLOCK_USE_OWNERS, BLOCK_USE_ROWS};

/// The `owners:` column of the `block_use_dispatch!` table is the only list of
/// blocks this surface answers a right click for, and the three chains it
/// replaced had no such list at all -- each leg named its own blocks inline.
/// Nothing forces the column to agree with the generated catalog on its own,
/// so this holds it against [`bbb_protocol::block_types`]: every class the
/// table declares is one the catalog names as the owner of a `useItemOn`,
/// `useWithoutItem` or `getMenuProvider` override, and no class is declared
/// twice. A row that invents a class name, or that duplicates another row's,
/// fails here rather than quietly becoming a second source of truth.
#[test]
fn the_block_use_table_declares_unique_generated_override_owners() {
    let mut generated = BTreeSet::new();
    for entry in VANILLA_BLOCK_TYPES {
        if let Some(owner) = entry.use_item_on_override {
            generated.insert(format!("{owner:?}"));
        }
        if let Some(owner) = entry.use_without_item_override {
            generated.insert(format!("{owner:?}"));
        }
        if let Some(owner) = entry.menu_provider_override {
            generated.insert(format!("{owner:?}"));
        }
    }
    let mut declared = BTreeSet::new();
    for &owner in BLOCK_USE_OWNERS {
        assert!(
            generated.contains(owner),
            "{owner} owns no generated useItemOn, useWithoutItem or getMenuProvider override"
        );
        assert!(declared.insert(owner), "{owner} is declared twice");
    }
    assert_eq!(declared.len(), BLOCK_USE_OWNERS.len());
}

/// The three dispatchers walk the table in order and stop at the first
/// predicate that claims the clicked state, which is only the same answer the
/// three hand-written chains gave while no two rows can claim one block: the
/// chains ran in three different orders, so an overlap would have answered
/// differently depending on the leg. Every predicate is derived from a
/// generated class tag, so an overlap is a generator change rather than a
/// typo -- and this is where it surfaces.
#[test]
fn no_two_table_rows_claim_the_same_block_state() {
    for block_state_id in 0..VANILLA_BLOCK_STATE_COUNT {
        let claims = BLOCK_USE_ROWS
            .iter()
            .filter(|row| (row.matches)(block_state_id))
            .map(|row| row.owners)
            .collect::<Vec<_>>();
        assert!(
            claims.len() <= 1,
            "block state {block_state_id} is claimed by more than one row: {claims:?}"
        );
    }
}

#[test]
fn the_two_use_columns_cover_the_generated_catalog_across_both_entry_points() {
    use std::collections::BTreeMap;

    use crate::block_behavior::{
        block_behavior_owner, BlockBehaviorOwner, BLOCK_BEHAVIOR_OWNER_HOOKS,
    };
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;

    fn column_owners(column: &str) -> std::collections::HashSet<BlockBehaviorOwner> {
        BLOCK_BEHAVIOR_OWNER_HOOKS
            .iter()
            .find_map(|(hook, owners)| (*hook == column).then_some(*owners))
            .expect("the block table declares the column")
            .iter()
            .map(|(owner, _)| *owner)
            .collect()
    }

    let item_on_owners = column_owners("use_item_on");
    let without_item_owners = column_owners("use_without_item");
    let mut open_item_on: BTreeMap<String, usize> = BTreeMap::new();
    let mut open_without_item: BTreeMap<String, usize> = BTreeMap::new();
    for entry in VANILLA_BLOCK_TYPES {
        let Some(state) = vanilla_block_default_state_id(entry.id) else {
            continue;
        };
        let row = BLOCK_USE_ROWS.iter().find(|row| (row.matches)(state));
        let behavior_owner = block_behavior_owner(state);
        if let Some(class) = entry.use_item_on_override {
            let world_leg = row.is_some_and(|row| row.item_on.is_some());
            let block_leg = behavior_owner.is_some_and(|owner| item_on_owners.contains(&owner));
            assert!(
                !(world_leg && block_leg),
                "{} answers useItemOn on both entry points",
                entry.resource_id
            );
            if !world_leg && !block_leg {
                *open_item_on.entry(format!("{class:?}")).or_default() += 1;
            }
        }
        if let Some(class) = entry.use_without_item_override {
            let world_leg = row.is_some_and(|row| row.without_item.is_some());
            let block_leg =
                behavior_owner.is_some_and(|owner| without_item_owners.contains(&owner));
            assert!(
                !(world_leg && block_leg),
                "{} answers useWithoutItem on both entry points",
                entry.resource_id
            );
            if !world_leg && !block_leg {
                *open_without_item.entry(format!("{class:?}")).or_default() += 1;
            }
        }
    }
    let open_item_on: Vec<(&str, usize)> = open_item_on
        .iter()
        .map(|(class, count)| (class.as_str(), *count))
        .collect();
    assert_eq!(open_item_on, OPEN_USE_ITEM_ON_CLASSES);
    let open_without_item: Vec<(&str, usize)> = open_without_item
        .iter()
        .map(|(class, count)| (class.as_str(), *count))
        .collect();
    assert_eq!(open_without_item, OPEN_USE_WITHOUT_ITEM_CLASSES);
}

const OPEN_USE_ITEM_ON_CLASSES: &[(&str, usize)] = &[];

/// The `useWithoutItem` classes neither entry point answers, on the same
/// terms: the test and test-instance blocks wait on their own surfaces.
const OPEN_USE_WITHOUT_ITEM_CLASSES: &[(&str, usize)] =
    &[("TestBlock", 1), ("TestInstanceBlock", 1)];

#[test]
fn placement_reads_loaded_noise_light_without_loading_missing_columns() {
    use super::{placement_raw_brightness, BlockPos, ChunkStore};
    use crate::chunk::noise_generator::overworld_program;
    let mut store = ChunkStore::new_overworld_carvers_with_structure_generation(
        overworld_program(true, 0),
        false,
    )
    .unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    let min_y = store.min_y();
    let section_count = store.section_count();
    let columns_before = store.column_count();
    let underground = BlockPos {
        x: 8,
        y: min_y,
        z: 8,
    };
    let sky = BlockPos {
        x: 8,
        y: min_y + i32::try_from(section_count).unwrap() * 16 - 1,
        z: 8,
    };
    assert_eq!(placement_raw_brightness(&store, underground, 0), 0);
    assert_eq!(placement_raw_brightness(&store, sky, 0), 15);
    assert_eq!(placement_raw_brightness(&store, sky, 7), 8);
    let missing = BlockPos { x: 16000, ..sky };
    assert!(!store.is_loaded(missing.x >> 4, missing.z >> 4));
    let _ = placement_raw_brightness(&store, missing, 0);
    assert!(!store.is_loaded(missing.x >> 4, missing.z >> 4));
    assert_eq!(store.column_count(), columns_before);
}

#[test]
fn live_placement_coral_candidates_schedule_only_dry_death_and_execute_it() {
    use crate::block_behavior::BlockPlacementContext;
    use crate::connection::ConnectionId;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::{ServerWorld, ServerWorldConfig};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
    use bbb_protocol::packets::BlockPos;
    use bbb_server_types::chunk_ticks::BlockTickType;
    use bbb_server_types::save::Java26BuiltinDimension;
    fn state(name: &str) -> i32 {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    }
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let coral = state("minecraft:tube_coral_block");
    let item = vanilla_item_type_for_resource_id("minecraft:tube_coral_block")
        .unwrap()
        .id;
    let dry = BlockPos { x: 8, y: 70, z: 8 };
    let wet = BlockPos { x: 12, ..dry };
    world.set_block(
        dimension,
        BlockPos { x: 13, ..dry },
        state("minecraft:water"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    assert_eq!(
        world.placed_block_state(
            ConnectionId(1),
            dimension,
            BlockPlacementContext::from(wet),
            Some(item)
        ),
        Some(coral)
    );
    assert!(!world
        .level(dimension)
        .chunks
        .has_scheduled_tick(wet, BlockTickType::of_block_state(coral).unwrap()));
    assert_eq!(
        world.placed_block_state(
            ConnectionId(1),
            dimension,
            BlockPlacementContext::from(dry),
            Some(item)
        ),
        Some(coral)
    );
    assert!(world
        .level(dimension)
        .chunks
        .has_scheduled_tick(dry, BlockTickType::of_block_state(coral).unwrap()));
    world.set_raw_block_state_for_test(dimension, dry, coral);
    world.set_raw_block_state_for_test(dimension, wet, coral);
    world.set_game_time_for_test(world.game_time() + 100);
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(dimension, dry),
        Some(state("minecraft:dead_tube_coral_block"))
    );
    assert_eq!(world.block_state(dimension, wet), Some(coral));
}

#[test]
fn live_placement_growing_plants_select_legal_heads_connected_bodies_and_full_water() {
    use crate::block_behavior::{growing_plant, BlockPlacementContext};
    use crate::connection::ConnectionId;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::{ServerWorld, ServerWorldConfig};
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
    use bbb_protocol::packets::{BlockPos, Direction};
    use bbb_server_types::directions::{opposite, relative};
    use bbb_server_types::save::Java26BuiltinDimension;
    fn state(name: &str) -> i32 {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    }
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 8, y: 70, z: 8 };
    for (name, body, growth) in [
        (
            "minecraft:twisting_vines",
            "minecraft:twisting_vines_plant",
            Direction::Up,
        ),
        (
            "minecraft:weeping_vines",
            "minecraft:weeping_vines_plant",
            Direction::Down,
        ),
        (
            "minecraft:cave_vines",
            "minecraft:cave_vines_plant",
            Direction::Down,
        ),
        ("minecraft:kelp", "minecraft:kelp_plant", Direction::Up),
    ] {
        let item = vanilla_item_type_for_resource_id(if name == "minecraft:cave_vines" {
            "minecraft:glow_berries"
        } else {
            name
        })
        .unwrap()
        .id;
        let growth_pos = relative(pos, growth);
        world.set_block(
            dimension,
            relative(pos, opposite(growth)),
            state("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        world.set_block(
            dimension,
            growth_pos,
            state("minecraft:air"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        world.set_block(
            dimension,
            pos,
            state(if name == "minecraft:kelp" {
                "minecraft:water"
            } else {
                "minecraft:air"
            }),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        let head = world
            .placed_block_state(
                ConnectionId(1),
                dimension,
                BlockPlacementContext::from(pos),
                Some(item),
            )
            .unwrap();
        assert!(growing_plant::is_head(head));
        let age = bbb_server_types::block_queries::int_property(head, "age").unwrap();
        assert!((0..growing_plant::PLACEMENT_AGE_BOUND).contains(&age));
        for neighbor in [state(name), state(body)] {
            world.set_raw_block_state_for_test(dimension, growth_pos, neighbor);
            assert_eq!(
                world.placed_block_state(
                    ConnectionId(1),
                    dimension,
                    BlockPlacementContext::from(pos),
                    Some(item)
                ),
                Some(state(body))
            );
        }
        if name == "minecraft:kelp" {
            for replaced in [
                state("minecraft:air"),
                crate::fluid::with_property(state("minecraft:water"), "level", "1").unwrap(),
            ] {
                world.set_raw_block_state_for_test(dimension, pos, replaced);
                assert_eq!(
                    world.placed_block_state(
                        ConnectionId(1),
                        dimension,
                        BlockPlacementContext::from(pos),
                        Some(item)
                    ),
                    None
                );
            }
        }
    }
}
