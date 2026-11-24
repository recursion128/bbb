use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::world::ConnectionId;
use bbb_protocol::NbtSummaryEntry;
use bbb_server_types::item_stack::RuntimeItemStack;

fn dirt_with_components(count: i32, components: Vec<NbtSummaryEntry>) -> RuntimeItemStack {
    RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:dirt".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(count),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(components),
            },
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .unwrap()
}

fn item_on_a_plate(stack: RuntimeItemStack, x: f64, z: f64) -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_item_entity(&mut store, Vec3d { x, y: 64.0, z }, stack)
        .expect("a freshly drawn UUID is free");
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    (store, level, id)
}

fn item_data_mut(store: &mut EntityStore, id: i32) -> &mut ItemEntityData {
    match &mut store.get_mut(id).expect("entity is live").data {
        EntityData::Item(data) => data,
        other => panic!("expected an item entity, found {other:?}"),
    }
}

#[test]
fn the_item_entity_size_and_gravity_come_from_the_generated_catalog() {
    assert_eq!(
        vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ITEM_ID),
        Some([0.25, 0.25])
    );
    assert_eq!(
        vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_ITEM_ID),
        Some(0.2125)
    );
    let (store, _level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_ITEM_ID);
    assert_eq!(
        entity.bounding_box().max_y - entity.bounding_box().min_y,
        0.25
    );
    assert_eq!(entity.gravity(), 0.04);
}

#[test]
fn an_item_entity_is_not_pickable() {
    let (store, _level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    let entity = store.get(id).expect("entity is live");
    assert!(!entity.is_pickable());
    assert!(!entity.can_be_hit_by_projectile());
}

#[test]
fn an_item_reads_the_friction_of_the_block_a_whole_block_below_it() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 10.6,
        z: 0.5,
    };
    let item = spawn_item_entity(&mut store, position, stack_of("minecraft:dirt", 1))
        .expect("a freshly drawn UUID is free");
    let falling = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        position,
        EntityData::FallingBlock(FallingBlockData::new(
            state_id("minecraft:sand"),
            pos(0, 10, 0),
        )),
    );
    assert_eq!(
        store
            .get(item)
            .expect("entity is live")
            .block_pos_below_that_affects_my_movement(),
        pos(0, 9, 0)
    );
    assert_eq!(
        store
            .get(falling)
            .expect("entity is live")
            .block_pos_below_that_affects_my_movement(),
        pos(0, 10, 0)
    );
}

#[test]
fn a_dropped_item_falls_under_gravity_and_keeps_the_airborne_drag() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.position().y, 68.0 - 0.04);
    assert_eq!(entity.delta_movement().y, -0.04 * 0.98_f64);
    assert!(!entity.on_ground());
}

#[test]
fn an_embedded_item_escapes_to_nearby_air_then_lands_on_solid_ground() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level.set(pos(8, 66, 8), state_id("minecraft:stone"));
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 8.85,
            y: 66.375,
            z: 8.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });

    for _ in 0..80 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }

    let entity = store.get(id).unwrap();
    assert!(entity.position().x >= 9.125);
    assert_eq!(entity.position().y, 64.0);
    assert!(entity.on_ground());
    for _ in 0..20 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(store.get(id).unwrap().position().y, 64.0);
}

#[test]
fn an_item_lands_on_the_block_below_and_stops_there() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 67.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    for _ in 0..80 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        if store.get(id).unwrap().on_ground() {
            break;
        }
    }
    assert!(store.get(id).unwrap().on_ground());
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.position().y, 64.0);
    assert_eq!(entity.delta_movement().y, 0.0);

    for _ in 0..20 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let entity = store.get(id).expect("entity is live");
    assert_eq!(entity.position().y, 64.0);
    assert!(entity.on_ground());
}

#[test]
fn a_moving_item_contacts_a_portal_crossed_by_its_path() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    )
    .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
    let entry = pos(2, 64, 0);
    let mut level = NativeLevel::new();
    assert!(level.world.set_block(
        DIMENSION,
        entry,
        state_id("minecraft:nether_portal"),
        2 | 16
    ));

    assert!(level.with_level(|ctx| crate::entity::item::tick(&mut entity, &mut store, ctx)));

    assert_eq!(entity.portal_process().unwrap().entry_position(), entry);
    assert!(entity.position().x > 3.0);
}

#[test]
fn a_grounded_item_is_dragged_by_the_friction_of_the_block_below_it() {
    for (block, friction) in [
        ("minecraft:stone", 0.6_f32),
        ("minecraft:ice", 0.98),
        ("minecraft:packed_ice", 0.98),
        ("minecraft:blue_ice", 0.989),
    ] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        for x in 0..=1 {
            let pos = pos(x, 63, 0);
            if level.world.block_state(DIMENSION, pos) != Some(state_id(block)) {
                level.set(pos, state_id(block));
            }
            assert_eq!(
                level.world.block_state(DIMENSION, pos),
                Some(state_id(block))
            );
        }
        let id = spawn_item_entity(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            stack_of("minecraft:dirt", 1),
        )
        .expect("a freshly drawn UUID is free");
        store
            .get_mut(id)
            .expect("entity is live")
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert!(store.get(id).expect("entity is live").on_ground());
        store
            .get_mut(id)
            .expect("entity is live")
            .set_delta_movement(Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            });
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(
            store.get(id).expect("entity is live").delta_movement().x,
            0.5 * f64::from(friction * 0.98_f32)
        );
    }
}

/// `Entity.collectColliders` (`Entity.java:1149-1152`) adds the border for any mover.
#[test]
fn an_item_stops_at_the_world_border() {
    let (mut store, mut level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    level.set_border(bbb_server_types::interaction::WorldBorder::from_settings(
        0.0, 0.0, 0.2, 5.0, 5, 15, 4.0, 0, 4.0, 0,
    ));
    store
        .get_mut(id)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 1.5,
            y: 0.0,
            z: 0.0,
        });
    for _ in 0..4 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let entity = store.get(id).expect("entity is live");
    assert!(
        entity.bounding_box().max_x <= 2.0,
        "{:?}",
        entity.position()
    );
}

/// `Entity.move` scales the step by `getBlockSpeedFactor` (`Entity.java:1021-1029`).
#[test]
fn an_item_on_soul_sand_is_slowed_by_its_speed_factor() {
    let travelled = |block: &str| {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        for x in 0..=3 {
            if level.world.block_state(DIMENSION, pos(x, 63, 0)) != Some(state_id(block)) {
                level.set(pos(x, 63, 0), state_id(block));
            }
        }
        let id = spawn_item_entity(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            stack_of("minecraft:dirt", 1),
        )
        .expect("a freshly drawn UUID is free");
        for _ in 0..3 {
            level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        }
        store
            .get_mut(id)
            .expect("entity is live")
            .set_delta_movement(Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0,
            });
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        store.get(id).expect("entity is live").delta_movement().x
    };
    let stone = travelled("minecraft:stone");
    let soul_sand = travelled("minecraft:soul_sand");
    assert!(
        (soul_sand - stone * 0.4).abs() < 1e-6,
        "stone {stone}, soul sand {soul_sand}"
    );
}

#[test]
fn submerged_items_rise_in_water_and_fire_resistant_items_rise_in_lava() {
    for (fluid, item) in [
        ("minecraft:water", "minecraft:dirt"),
        ("minecraft:lava", "minecraft:netherite_ingot"),
    ] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        for y in 64..=70 {
            for x in 1..=5 {
                for z in 1..=5 {
                    level.set(pos(x, y, z), state_id(fluid));
                }
            }
        }
        let start_y = 66.0;
        let id = spawn_item_entity(
            &mut store,
            Vec3d {
                x: 3.5,
                y: start_y,
                z: 3.5,
            },
            stack_of(item, 1),
        )
        .expect("a freshly drawn UUID is free");
        store
            .get_mut(id)
            .expect("entity is live")
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        for _ in 0..100 {
            level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        }
        let entity = store.get(id).expect("the item survives its fluid");
        assert!(
            entity.position().y > start_y + 1.0,
            "{item} rises in {fluid}"
        );
    }
}

#[test]
fn the_pickup_delay_counts_down_but_the_infinite_one_does_not() {
    let (mut store, mut level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    item_data_mut(&mut store, id).set_default_pickup_delay();
    assert_eq!(item_state(&store, id).pickup_delay(), DEFAULT_PICKUP_DELAY);
    for expected in (0..DEFAULT_PICKUP_DELAY).rev() {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(item_state(&store, id).pickup_delay(), expected);
    }
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(item_state(&store, id).pickup_delay(), 0);

    item_data_mut(&mut store, id).set_never_pick_up();
    for _ in 0..5 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(item_state(&store, id).pickup_delay(), INFINITE_PICKUP_DELAY);
    }
}

#[test]
fn an_item_vanishes_on_the_six_thousandth_tick_and_not_before() {
    let (mut store, mut level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    let data = item_state(&store, id).clone().with_saved_fields(
        DEFAULT_HEALTH,
        LIFETIME - 2,
        0,
        None,
        None,
    );
    *item_data_mut(&mut store, id) = data;
    for _ in 0..1 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(item_state(&store, id).age(), LIFETIME - 1);
    assert!(store.get(id).is_some());
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(id).is_none(), "the item outlived its lifetime");
    assert!(store.is_empty());
}

#[test]
fn an_item_with_an_unlimited_lifetime_never_ages_and_never_vanishes() {
    let (mut store, mut level, id) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    item_data_mut(&mut store, id).set_unlimited_lifetime();
    for _ in 0..20 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(item_state(&store, id).age(), INFINITE_LIFETIME);
    assert!(store.get(id).is_some());
}

#[test]
fn two_equal_stacks_lying_together_conserve_their_items() {
    let (mut store, mut level, _first) = item_on_a_plate(dirt_with_components(1, vec![]), 0.5, 0.5);
    store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
    let explicit_default = dirt_with_components(
        1,
        vec![NbtSummaryEntry {
            name: "minecraft:rarity".into(),
            value: NbtSummaryValue::String("common".into()),
        }],
    );
    let second = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.9,
            y: 64.0,
            z: 0.5,
        },
        explicit_default.clone(),
    )
    .expect("a freshly drawn UUID is free");
    store
        .get_mut(second)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });

    for _ in 0..MERGE_RATE_STILL {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let remaining = store.ids();
    assert_eq!(remaining.len(), 1);
    assert_eq!(stack_count(item_state(&store, remaining[0]).item()), 2);
    assert_eq!(store.len(), 1);
    assert_eq!(
        item_state(&store, remaining[0])
            .item()
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                store.item_resolver(),
            )
            .unwrap(),
        Some(NbtSummaryValue::String("common".into()))
    );
}

#[test]
fn nearby_unequal_stacks_merge_without_losing_items_or_components() {
    for (component, merges) in [
        (
            NbtSummaryEntry {
                name: "minecraft:rarity".into(),
                value: NbtSummaryValue::String("common".into()),
            },
            true,
        ),
        (
            NbtSummaryEntry {
                name: "minecraft:rarity".into(),
                value: NbtSummaryValue::String("uncommon".into()),
            },
            false,
        ),
        (
            NbtSummaryEntry {
                name: "!minecraft:rarity".into(),
                value: NbtSummaryValue::Compound(vec![]),
            },
            false,
        ),
    ] {
        let larger_stack = dirt_with_components(3, vec![component]);
        let smaller_stack = dirt_with_components(1, vec![]);
        let (mut store, mut level, larger) = item_on_a_plate(larger_stack.clone(), 0.5, 0.5);
        store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
        let smaller = spawn_item_entity(
            &mut store,
            Vec3d {
                x: 0.9,
                y: 64.0,
                z: 0.5,
            },
            smaller_stack.clone(),
        )
        .expect("a freshly drawn UUID is free");
        store
            .get_mut(smaller)
            .expect("entity is live")
            .set_delta_movement(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        for _ in 0..MERGE_RATE_STILL {
            level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        }
        let remaining = store.ids();
        if merges {
            assert_eq!(remaining.len(), 1);
            let state = item_state(&store, remaining[0]);
            let stack = state.item();
            assert_eq!(stack.count(), 4);
            assert_eq!(
                stack
                    .component_nbt(
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                        store.item_resolver(),
                    )
                    .unwrap(),
                Some(NbtSummaryValue::String("common".into()))
            );
        } else {
            assert_eq!(remaining.len(), 2);
            assert_eq!(item_state(&store, larger).item(), &larger_stack);
            assert_eq!(item_state(&store, smaller).item(), &smaller_stack);
        }
    }
}

#[test]
fn merging_keeps_the_longer_pickup_delay_and_the_younger_age() {
    let (mut store, mut level, older) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.9, 0.5);
    for _ in 0..10 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let younger = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");
    assert_eq!(item_state(&store, older).age(), 10);
    assert_eq!(item_state(&store, younger).age(), 0);
    item_data_mut(&mut store, younger).set_pickup_delay(5);
    item_data_mut(&mut store, older).set_pickup_delay(20);

    let mut entity = store.detach(younger).expect("entity is live");
    merge_with_neighbours(&mut entity, &mut store);
    let discarded = entity.removal_reason();
    store.attach(entity);

    assert_eq!(discarded, Some(RemovalReason::Discarded));
    assert!(store.get(younger).is_none());
    let survivor = item_state(&store, older);
    assert_eq!(stack_count(survivor.item()), 2);
    assert_eq!(survivor.pickup_delay(), 20);
    assert_eq!(survivor.age(), 0);
}

#[test]
fn stacks_that_are_not_the_same_item_are_left_alone() {
    let (mut store, mut level, first) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    let second = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.9,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    )
    .expect("a freshly drawn UUID is free");
    store
        .get_mut(second)
        .expect("entity is live")
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    for _ in 0..MERGE_RATE_STILL {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert_eq!(store.len(), 2);
    assert_eq!(stack_count(item_state(&store, first).item()), 1);
    assert_eq!(stack_count(item_state(&store, second).item()), 1);
}

#[test]
fn a_full_stack_and_a_never_pick_up_stack_are_not_mergable() {
    let (mut store, _level, full) = item_on_a_plate(stack_of("minecraft:dirt", 64), 0.5, 0.5);
    assert!(!is_mergable(store.get(full).expect("entity is live")));
    let one = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.9,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");
    assert!(is_mergable(store.get(one).expect("entity is live")));

    let mut entity = store.detach(one).expect("entity is live");
    merge_with_neighbours(&mut entity, &mut store);
    store.attach(entity);
    assert_eq!(stack_count(item_state(&store, full).item()), 64);
    assert_eq!(stack_count(item_state(&store, one).item()), 1);

    item_data_mut(&mut store, one).set_never_pick_up();
    assert!(!is_mergable(store.get(one).expect("entity is live")));
}

#[test]
fn an_item_entity_round_trips_through_the_entity_chunk_save_form() {
    let thrower = uuid::Uuid::from_u128(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10);
    let target = uuid::Uuid::from_u128(0x1112_1314_1516_1718_191a_1b1c_1d1e_1f20);
    let (mut store, mut level, id) = item_on_a_plate(stack_of("minecraft:diamond", 7), 0.5, 0.5);
    {
        let data = item_data_mut(&mut store, id);
        data.set_pickup_delay(11);
        data.set_thrower(Some(thrower));
        data.set_target(Some(target));
    }
    for _ in 0..3 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let before = store.get(id).expect("entity is live").clone();
    let before_data = item_state(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.entities().len(), 1);
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:item");

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).expect("entity is live");
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.entity_type_id(), before.entity_type_id());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert_eq!(after.on_ground(), before.on_ground());

    let after_data = item_state(&reloaded, ids[0]);
    assert_eq!(after_data.item(), before_data.item());
    assert_eq!(after_data.age(), before_data.age());
    assert_eq!(after_data.pickup_delay(), before_data.pickup_delay());
    assert_eq!(after_data.health(), before_data.health());
    assert_eq!(after_data.health(), DEFAULT_HEALTH);
    assert_eq!(after_data.thrower(), Some(thrower));
    assert_eq!(after_data.target(), Some(target));
    assert!((0.0..std::f32::consts::PI * 2.0).contains(&after_data.bob_offs()));
}

#[test]
fn the_item_fields_this_crate_writes_are_the_tags_the_official_schema_requires() {
    let (mut store, _level, id) = item_on_a_plate(stack_of("minecraft:diamond", 7), 0.5, 0.5);
    item_data_mut(&mut store, id).set_never_pick_up();
    item_data_mut(&mut store, id).set_unlimited_lifetime();
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    let record = document.entities().next().unwrap();
    let field = |name: &str| {
        record
            .raw()
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.value.clone())
    };
    assert_eq!(field("Health"), Some(NbtSummaryValue::Short(5)));
    assert_eq!(field("Age"), Some(NbtSummaryValue::Short(i16::MIN)));
    assert_eq!(field("PickupDelay"), Some(NbtSummaryValue::Short(i16::MAX)));
    assert_eq!(field("Thrower"), None);
    assert_eq!(field("Owner"), None);
    assert!(matches!(field("Item"), Some(NbtSummaryValue::Compound(_))));
}

#[test]
fn advancement_reward_drop_preserves_stack_motion_and_target_on_restore() {
    let position = Vec3d {
        x: 1.5,
        y: 80.25,
        z: 2.5,
    };
    let velocity = Vec3d {
        x: -0.2,
        y: 0.15,
        z: 0.3,
    };
    let target = Uuid::from_u128(17);
    let stack = stack_of("minecraft:diamond", 7);
    let mut store = EntityStore::new(42, EntityIdCounter::new());
    let id = crate::entity::spawn_advancement_reward_item(
        &mut store,
        position,
        stack.clone(),
        velocity,
        target,
    )
    .unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position(), position);
    assert_eq!(entity.delta_movement(), velocity);
    let data = item_state(&store, id);
    assert_eq!(data.item(), &stack);
    assert_eq!(data.pickup_delay(), 0);
    assert_eq!(data.target(), Some(target));
    assert_eq!(data.thrower(), None);

    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(document.entities().len(), 1);
    let mut restored = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).unwrap();
    assert_eq!(ids.len(), 1);
    let after = restored.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), store.get(id).unwrap().uuid());
    assert_eq!(after.position(), position);
    assert_eq!(after.delta_movement(), velocity);
    let after_data = item_state(&restored, ids[0]);
    assert_eq!(after_data.item(), &stack);
    assert_eq!(after_data.pickup_delay(), 0);
    assert_eq!(after_data.target(), Some(target));
    assert_eq!(after_data.thrower(), None);
}

#[test]
fn a_dropped_item_has_the_default_pickup_delay() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_dropped_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 40.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");

    assert_eq!(item_state(&store, id).pickup_delay(), DEFAULT_PICKUP_DELAY);
    let entity = store.get(id).unwrap();
    assert_eq!(entity.delta_movement().y, 0.2);
    assert!((-0.1..0.1).contains(&entity.delta_movement().x));
    assert!((-0.1..0.1).contains(&entity.delta_movement().z));
    assert!((0.0..360.0).contains(&entity.y_rot()));
    assert_eq!(entity.x_rot(), 0.0);
    assert!((0.0..std::f32::consts::PI * 2.0).contains(&item_state(&store, id).bob_offs()));
}

#[test]
fn the_merge_search_is_half_a_block_wide_and_not_one_bit_tall() {
    let far = {
        let (mut store, _level, near) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
        spawn_item_entity(
            &mut store,
            Vec3d {
                x: 1.6,
                y: 64.0,
                z: 0.5,
            },
            stack_of("minecraft:dirt", 1),
        )
        .expect("a freshly drawn UUID is free");
        let mut entity = store.detach(near).expect("entity is live");
        merge_with_neighbours(&mut entity, &mut store);
        let removed = entity.removal_reason();
        store.attach(entity);
        (removed, store.len())
    };
    assert_eq!(far, (None, 2));

    let above = {
        let (mut store, _level, near) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
        spawn_item_entity(
            &mut store,
            Vec3d {
                x: 0.5,
                y: 65.0,
                z: 0.5,
            },
            stack_of("minecraft:dirt", 1),
        )
        .expect("a freshly drawn UUID is free");
        let mut entity = store.detach(near).expect("entity is live");
        merge_with_neighbours(&mut entity, &mut store);
        let removed = entity.removal_reason();
        store.attach(entity);
        (removed, store.len())
    };
    assert_eq!(above, (None, 2));

    let (mut store, _level, near) = item_on_a_plate(stack_of("minecraft:dirt", 1), 0.5, 0.5);
    spawn_item_entity(
        &mut store,
        Vec3d {
            x: 1.0,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .expect("a freshly drawn UUID is free");
    let mut entity = store.detach(near).expect("entity is live");
    merge_with_neighbours(&mut entity, &mut store);
    let removed = entity.removal_reason();
    store.attach(entity);
    assert_eq!(removed, Some(RemovalReason::Discarded));
    assert_eq!(store.len(), 1);
}

fn pickup_world(
    uuid: u128,
) -> (
    NativeLevel,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "picker", uuid);
    level.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    level.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    (level, sender, receiver)
}

fn ground_item(
    level: &mut NativeLevel,
    stack: RuntimeItemStack,
    delay: i32,
    target: Option<Uuid>,
) -> i32 {
    let mut data = ItemEntityData::new(stack, 0.0);
    data.set_pickup_delay(delay);
    data.set_target(target);
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::Item(data),
    );
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_delta_movement(Vec3d::default());
    id
}

fn fill_inventory(level: &mut NativeLevel, selected_count: i32) {
    for slot in 9..=45 {
        level.world.stock_menu_slot_for_test(
            ConnectionId(1),
            slot,
            ItemStackSummary {
                item_id: Some(
                    bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:dirt")
                        .unwrap()
                        .id,
                ),
                count: if slot == 36 { selected_count } else { 64 },
                component_patch: Default::default(),
            },
        );
    }
}

#[test]
fn a_stack_kept_for_one_player_is_refused_to_every_other() {
    let (mut level, sender, receiver) = pickup_world(2);
    let id = ground_item(
        &mut level,
        stack_of("minecraft:dirt", 1),
        0,
        Some(Uuid::from_u128(1)),
    );
    level.world.tick(receiver.drain());
    assert_eq!(
        level
            .world
            .stored_item_stack_for_test(DIMENSION, id)
            .unwrap()
            .count(),
        1
    );
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 2, "owner", 1);
    level.world.set_player_position_for_test(
        ConnectionId(2),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    level.world.tick(receiver.drain());
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .player_inventory_for_test(ConnectionId(2))
        .inventory_slots()
        .iter()
        .any(|slot| slot.count() == 1));
}

#[test]
fn finite_delays_allow_pickup_but_the_infinite_delay_does_not() {
    for delay in [DEFAULT_PICKUP_DELAY, 1, INFINITE_PICKUP_DELAY] {
        let (mut level, _sender, receiver) = pickup_world(1);
        let id = ground_item(&mut level, stack_of("minecraft:dirt", 1), delay, None);
        if delay == DEFAULT_PICKUP_DELAY {
            level.world.tick(receiver.drain());
            assert_eq!(
                level
                    .world
                    .stored_item_stack_for_test(DIMENSION, id)
                    .unwrap()
                    .count(),
                1
            );
            assert!(level
                .world
                .player_inventory_for_test(ConnectionId(1))
                .inventory_slots()
                .iter()
                .all(|stack| stack.item_id() != stack_of("minecraft:dirt", 1).item_id()));
        }
        for _ in 0..DEFAULT_PICKUP_DELAY + 1 {
            level.world.tick(receiver.drain());
        }
        assert_eq!(
            level.world.stored_entity_for_test(DIMENSION, id).is_some(),
            delay == INFINITE_PICKUP_DELAY
        );
    }
}

#[test]
fn a_stack_the_inventory_only_partly_takes_leaves_the_rest_on_the_ground() {
    let (mut level, _sender, receiver) = pickup_world(1);
    fill_inventory(&mut level, 63);
    let id = ground_item(&mut level, stack_of("minecraft:dirt", 4), 0, None);
    level.world.tick(receiver.drain());
    assert_eq!(
        level
            .world
            .player_inventory_for_test(ConnectionId(1))
            .inventory_slots()[0]
            .count(),
        64
    );
    assert_eq!(
        level
            .world
            .stored_item_stack_for_test(DIMENSION, id)
            .unwrap()
            .count(),
        3
    );
}

#[test]
fn an_inventory_that_takes_nothing_leaves_the_stack_untouched() {
    let (mut level, _sender, receiver) = pickup_world(1);
    fill_inventory(&mut level, 64);
    let before = level
        .world
        .player_inventory_for_test(ConnectionId(1))
        .clone();
    let id = ground_item(&mut level, stack_of("minecraft:diamond", 1), 0, None);
    level.world.tick(receiver.drain());
    assert_eq!(
        level
            .world
            .stored_item_stack_for_test(DIMENSION, id)
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        level.world.player_inventory_for_test(ConnectionId(1)),
        &before
    );
}

#[test]
fn a_pickup_hands_the_full_ground_stack_to_the_inventory() {
    let enchanted = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:diamond_sword"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:enchantments".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "minecraft:sharpness".into(),
                        value: NbtSummaryValue::Int(5),
                    }]),
                }]),
            },
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .unwrap();

    let (mut level, _sender, receiver) = pickup_world(1);
    let id = ground_item(&mut level, enchanted.clone(), 2, None);
    let mut events = Vec::new();
    for _ in 0..DEFAULT_PICKUP_DELAY + 4 {
        events.extend(level.world.tick(receiver.drain()).events);
        if level.world.stored_entity_for_test(DIMENSION, id).is_none() {
            break;
        }
    }
    assert!(events.iter().any(|event| match event {
        crate::world::WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet,
        } => match packet {
            crate::connection::ClientboundPlayPacket::ContainerSetSlot(slot) =>
                slot.item == *enchanted.summary(),
            crate::connection::ClientboundPlayPacket::ContainerSetContent(content) => content
                .items
                .iter()
                .any(|stack| stack == enchanted.summary()),
            _ => false,
        },
        _ => false,
    }));
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(level
        .world
        .player_inventory_for_test(ConnectionId(1))
        .inventory_slots()
        .iter()
        .any(|slot| slot == &enchanted));
    let saved = level
        .world
        .save_request(0, 0, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    let inventory = &saved
        .player_patches
        .iter()
        .find(|patch| patch.uuid == Uuid::from_u128(1))
        .unwrap()
        .created
        .inventory;
    assert!(inventory
        .runtime_slots()
        .iter()
        .any(|slot| slot == &enchanted));
    assert!(inventory.prepared_fields().is_some());
}

#[test]
fn player_drop_uses_the_hand_position_velocity_and_pickup_delay() {
    let position = Vec3d {
        x: -9.5,
        y: 65.32,
        z: 12.5,
    };
    let velocity = Vec3d {
        x: -0.21,
        y: 0.13,
        z: 0.28,
    };
    let thrower = Uuid::from_u128(99);
    let stack = stack_of("minecraft:stone", 3);
    let mut store = EntityStore::new(42, EntityIdCounter::new());
    let id = crate::entity::spawn_player_dropped_item_entity(
        &mut store,
        position,
        stack.clone(),
        velocity,
        true,
        thrower,
    )
    .unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position(), position);
    assert_eq!(entity.delta_movement(), velocity);
    let data = item_state(&store, id);
    assert_eq!(data.item(), &stack);
    assert_eq!(data.pickup_delay(), crate::entity::item::DROP_PICKUP_DELAY);
    assert_eq!(data.thrower(), Some(thrower));
    assert_eq!(data.target(), None);

    let mut other = EntityStore::new(42, EntityIdCounter::new());
    let plain = crate::entity::spawn_player_dropped_item_entity(
        &mut other, position, stack, velocity, false, thrower,
    )
    .unwrap();
    assert_eq!(item_state(&other, plain).thrower(), None);
}

#[test]
fn only_an_item_entity_carrying_a_dampening_stack_dampens_vibrations() {
    let wool = EntityData::Item(ItemEntityData::new(
        stack_of("minecraft:white_wool", 1),
        0.0,
    ));
    let stone = EntityData::Item(ItemEntityData::new(stack_of("minecraft:stone", 1), 0.0));
    let dampening = |item_type_id: i32| {
        bbb_protocol::item_types::vanilla_item_type(item_type_id)
            .is_some_and(|item| item.resource_id == "minecraft:white_wool")
    };
    assert!(wool.dampens_vibrations(dampening));
    assert!(!stone.dampens_vibrations(dampening));
    assert!(!EntityData::FallingBlock(FallingBlockData::new(
        state_id("minecraft:sand"),
        pos(0, 1, 0)
    ))
    .dampens_vibrations(dampening));
}

#[test]
fn an_item_entity_answers_the_thrower_as_its_owner() {
    let thrower = uuid::Uuid::from_u128(7);
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let thrown = crate::entity::spawn_player_dropped_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
        Vec3d::default(),
        true,
        thrower,
    )
    .expect("a freshly drawn UUID is free");
    assert_eq!(
        store
            .get(thrown)
            .expect("entity is live")
            .data()
            .item_thrower(),
        Some(thrower)
    );

    let dropped = crate::entity::spawn_player_dropped_item_entity(
        &mut store,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
        Vec3d::default(),
        false,
        thrower,
    )
    .expect("a freshly drawn UUID is free");
    assert_eq!(
        store
            .get(dropped)
            .expect("entity is live")
            .data()
            .item_thrower(),
        None
    );
}

#[test]
fn an_item_resyncs_in_water_and_after_a_sharp_velocity_change() {
    let (mut store, mut level, id) = item_on_a_plate(dirt_with_components(1, Vec::new()), 0.5, 0.5);
    for _ in 0..5 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    store.get_mut(id).unwrap().set_needs_sync(false);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(
        !store.get(id).unwrap().needs_sync(),
        "a resting item stays quiet"
    );

    // A ground hit turns a fast fall into a small bounce.
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(id).unwrap().needs_sync());

    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    for y in 64..=66 {
        level.set(pos(0, y, 0), state_id("minecraft:water"));
    }
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        stack_of("minecraft:dirt", 1),
    )
    .unwrap();
    store.get_mut(id).unwrap().set_needs_sync(false);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(id).unwrap().needs_sync());
}
