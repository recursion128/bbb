use std::sync::Arc;

use crate::fluid::with_property;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

use super::{Aabb, Vec3d};
use crate::world::{ServerWorld, ServerWorldConfig};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const POSITIONS: [BlockPos; 2] = [
    BlockPos { x: 8, y: 70, z: 8 },
    BlockPos {
        x: -8,
        y: -10,
        z: -8,
    },
];
const LOCAL: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn fixture() -> (ServerWorld, [i32; 2]) {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    for pos in POSITIONS {
        world
            .level_mut(DIM)
            .chunks
            .load_column(pos.x >> 4, pos.z >> 4);
    }
    let store = &mut world.level_mut(DIM).entities;
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d::default())
        .unwrap();
    let stand = crate::entity::armor_stand::spawn_summoned(store, Vec3d::default()).unwrap();
    for id in [cow, stand] {
        assert!(store.get(id).unwrap().living().is_some());
    }
    (world, [cow, stand])
}

fn block(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    world.set_raw_block_state_for_test(DIM, pos, state);
}

fn context(world: &mut ServerWorld, id: i32, pos: BlockPos, bottom: f64, shift: bool) {
    let body = world.level_mut(DIM).entities.get_mut(id).unwrap();
    body.set_pos(Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: bottom,
        z: f64::from(pos.z) + 0.5,
    });
    body.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
    assert_eq!(body.position().y, bottom);
    assert_eq!(
        body.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
        shift
    );
}

fn shapes(world: &mut ServerWorld, id: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    world.with_stored_hurt_level(&DIM.into(), |store, level| {
        level.block_collision_boxes_for_entity(store.get(id).unwrap(), pos)
    })
}

fn boots(world: &mut ServerWorld, id: i32, slot: EquipmentSlot, item: Option<&str>) {
    let stack = item.map(|item| crate::entity::item::plain_stack(item, 1).unwrap());
    let living = world
        .level_mut(DIM)
        .entities
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap();
    living.set_item_slot(slot, stack.clone());
    assert_eq!(living.item_by_slot(slot), stack.as_ref());
}

fn distance(world: &mut ServerWorld, id: i32, distance: f64) {
    let store = &mut world.level_mut(DIM).entities;
    let NbtSummaryValue::Compound(mut record) = store.command_data(id).unwrap() else {
        panic!("stored living body saves a compound");
    };
    record
        .iter_mut()
        .find(|entry| entry.name == "fall_distance")
        .unwrap()
        .value = NbtSummaryValue::Double(distance.to_bits());
    store.load_command_data(id, &record).unwrap();
    assert_eq!(store.get(id).unwrap().fall_distance(), distance);
}

fn tags(world: &mut ServerWorld, members: Option<&[&str]>) {
    let tags = members.map(|members| {
        (
            "minecraft:entity_type".to_owned(),
            "minecraft:powder_snow_walkable_mobs".to_owned(),
            members.iter().map(|member| (*member).to_owned()).collect(),
        )
    });
    world
        .level_mut(DIM)
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

fn full() -> Vec<Aabb> {
    vec![Aabb::of_block(LOCAL)]
}

fn slab(height: f64) -> Vec<Aabb> {
    vec![Aabb {
        max_y: height,
        ..Aabb::of_block(LOCAL)
    }]
}

fn contains(boxes: &[Aabb], x: f64, y: f64, z: f64) -> bool {
    boxes.iter().any(|shape| {
        x >= shape.min_x
            && x < shape.max_x
            && y >= shape.min_y
            && y < shape.max_y
            && z >= shape.min_z
            && z < shape.max_z
    })
}

#[test]
fn stored_context_tests_static_controls_are_live_and_block_local() {
    let (mut world, bodies) = fixture();
    for pos in POSITIONS {
        for id in bodies {
            context(&mut world, id, pos, f64::from(pos.y) + 1.0, false);
            boots(
                &mut world,
                id,
                EquipmentSlot::Feet,
                Some("minecraft:leather_boots"),
            );
            distance(&mut world, id, 2.5_f64.next_up());
            for (name, expected) in [
                ("minecraft:stone", full()),
                ("minecraft:stone_slab", slab(0.5)),
                ("minecraft:air", Vec::new()),
            ] {
                block(&mut world, pos, state(name));
                assert_eq!(
                    shapes(&mut world, id, pos),
                    Some(expected),
                    "{pos:?}, id={id}, {name}"
                );
            }
        }
    }
}

#[test]
fn stored_context_tests_scaffolding_top_uses_strict_body_bottom_and_shift() {
    let (mut world, bodies) = fixture();
    let scaffold = with_property(
        with_property(state("minecraft:scaffolding"), "distance", "0").unwrap(),
        "bottom",
        "false",
    )
    .unwrap();
    // EntityCollisionContext.java:78-80; ScaffoldingBlock.java:137-146.
    for pos in POSITIONS {
        block(&mut world, pos, scaffold);
        let threshold = f64::from(pos.y) + 1.0 - f64::from(1.0e-5_f32);
        for id in bodies {
            for bottom in [
                threshold.next_down(),
                threshold,
                threshold.next_up(),
                f64::from(pos.y) + 1.0,
            ] {
                for shift in [false, true] {
                    context(&mut world, id, pos, bottom, shift);
                    let boxes = shapes(&mut world, id, pos).unwrap();
                    if bottom > threshold && !shift {
                        assert_eq!(boxes.len(), 7);
                        assert!(contains(&boxes, 0.5, 0.9, 0.5));
                        assert!(contains(&boxes, 0.0625, 0.25, 0.0625));
                        assert!(!contains(&boxes, 0.5, 0.5, 0.5));
                    } else {
                        assert!(
                            boxes.is_empty(),
                            "{pos:?}, id={id}, bottom={bottom}, shift={shift}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn stored_context_tests_scaffolding_bottom_requires_distance_and_lower_boundary() {
    let (mut world, bodies) = fixture();
    for pos in POSITIONS {
        let threshold = f64::from(pos.y) - f64::from(1.0e-5_f32);
        for distance in ["0", "1", "7"] {
            for bottom_flag in ["false", "true"] {
                let scaffold = with_property(
                    with_property(state("minecraft:scaffolding"), "distance", distance).unwrap(),
                    "bottom",
                    bottom_flag,
                )
                .unwrap();
                block(&mut world, pos, scaffold);
                for id in bodies {
                    for bottom in [
                        threshold.next_down(),
                        threshold,
                        threshold.next_up(),
                        f64::from(pos.y) + 0.5,
                    ] {
                        for shift in [false, true] {
                            context(&mut world, id, pos, bottom, shift);
                            let expected =
                                if distance != "0" && bottom_flag == "true" && bottom > threshold {
                                    slab(0.125)
                                } else {
                                    Vec::new()
                                };
                            assert_eq!(shapes(&mut world, id, pos), Some(expected), "{pos:?}, id={id}, distance={distance}, bottom={bottom_flag}, Y={bottom}, shift={shift}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn stored_context_tests_powder_distance_precedes_missing_tags_and_descending() {
    // PowderSnowBlock.java:116-131.
    let (mut world, bodies) = fixture();
    tags(&mut world, None);
    assert!(!world.level(DIM).chunks.registry_tags().unwrap().contains(
        "minecraft:entity_type",
        "minecraft:powder_snow_walkable_mobs",
        "minecraft:cow"
    ));
    for pos in POSITIONS {
        block(&mut world, pos, state("minecraft:powder_snow"));
        for id in bodies {
            for fall_distance in [2.5_f64.next_down(), 2.5, 2.5_f64.next_up(), 10.0] {
                distance(&mut world, id, fall_distance);
                for bottom in [f64::from(pos.y) - 0.5, f64::from(pos.y) + 2.0] {
                    for shift in [false, true] {
                        context(&mut world, id, pos, bottom, shift);
                        let expected = Some(if fall_distance > 2.5 {
                            slab(f64::from(0.9_f32))
                        } else {
                            Vec::new()
                        });
                        assert_eq!(
                            shapes(&mut world, id, pos),
                            expected,
                            "{pos:?}, id={id}, distance={fall_distance}, shift={shift}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn stored_context_tests_powder_nonmember_reads_only_leather_feet() {
    // PowderSnowBlock.java:139-145.
    let (mut world, bodies) = fixture();
    tags(&mut world, Some(&[]));
    for pos in POSITIONS {
        block(&mut world, pos, state("minecraft:powder_snow"));
        for id in bodies {
            context(&mut world, id, pos, f64::from(pos.y) + 1.0, false);
            for (slot, item, walks) in [
                (EquipmentSlot::Feet, None, false),
                (EquipmentSlot::Head, Some("minecraft:leather_boots"), false),
                (
                    EquipmentSlot::MainHand,
                    Some("minecraft:leather_boots"),
                    false,
                ),
                (EquipmentSlot::Feet, Some("minecraft:iron_boots"), false),
                (EquipmentSlot::Feet, Some("minecraft:leather_boots"), true),
            ] {
                boots(&mut world, id, EquipmentSlot::Feet, None);
                boots(&mut world, id, slot, item);
                assert_eq!(
                    shapes(&mut world, id, pos),
                    Some(if walks { full() } else { Vec::new() }),
                    "{pos:?}, id={id}, slot={slot:?}, item={item:?}"
                );
                boots(&mut world, id, slot, None);
            }
        }
    }
}

#[test]
fn stored_context_tests_powder_live_entity_tag_changes_existing_body_answer() {
    let (mut world, [cow, stand]) = fixture();
    for pos in POSITIONS {
        block(&mut world, pos, state("minecraft:powder_snow"));
        for id in [cow, stand] {
            context(&mut world, id, pos, f64::from(pos.y) + 1.0, false);
        }
        for (members, cow_walks, stand_walks) in [
            (&["minecraft:cow"][..], true, false),
            (&["minecraft:armor_stand"][..], false, true),
            (&[][..], false, false),
        ] {
            tags(&mut world, Some(members));
            for (id, walks) in [(cow, cow_walks), (stand, stand_walks)] {
                assert_eq!(
                    shapes(&mut world, id, pos),
                    Some(if walks { full() } else { Vec::new() }),
                    "{pos:?}, id={id}, members={members:?}"
                );
            }
        }
    }
}

#[test]
fn stored_context_tests_powder_walkable_still_requires_above_and_no_shift() {
    let (mut world, bodies) = fixture();
    tags(&mut world, Some(&[]));
    for pos in POSITIONS {
        block(&mut world, pos, state("minecraft:powder_snow"));
        let threshold = f64::from(pos.y) + 1.0 - f64::from(1.0e-5_f32);
        for id in bodies {
            boots(
                &mut world,
                id,
                EquipmentSlot::Feet,
                Some("minecraft:leather_boots"),
            );
            for bottom in [
                threshold.next_down(),
                threshold,
                threshold.next_up(),
                f64::from(pos.y) + 1.0,
            ] {
                for shift in [false, true] {
                    context(&mut world, id, pos, bottom, shift);
                    assert_eq!(
                        shapes(&mut world, id, pos),
                        Some(if bottom > threshold && !shift {
                            full()
                        } else {
                            Vec::new()
                        }),
                        "{pos:?}, id={id}, bottom={bottom}, shift={shift}"
                    );
                }
            }
        }
    }
}

#[test]
fn native_player_collision_uses_live_boots_fall_and_descending_context() {
    use crate::connection::ConnectionId;
    use crate::mailbox::world_command_mailbox;
    use crate::tests::join_loaded;

    let (sender, receiver) =
        world_command_mailbox(crate::mailbox::WorldMailboxLimits::default()).unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let pos = POSITIONS[0];
    world
        .level_mut(DIM)
        .chunks
        .load_column(pos.x >> 4, pos.z >> 4);
    block(&mut world, pos, state("minecraft:powder_snow"));
    let connection = ConnectionId(1);
    let player = world.levels.player_mut(connection).unwrap();
    {
        let mut position = player.position();
        position.y = f64::from(pos.y) + 1.0;
        player.set_position(position);
    }
    player.block_contact.fall_distance = 0.0;
    let feet = player
        .inventory
        .equipment_slots()
        .into_iter()
        .find(|(slot, _)| *slot == EquipmentSlot::Feet)
        .unwrap()
        .1;
    *player.inventory.slot_mut(feet).unwrap() =
        crate::tests::native_stack(bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_LEATHER_BOOTS_ID),
            count: 1,
            ..bbb_protocol::packets::ItemStackSummary::empty()
        });
    let shapes = |world: &ServerWorld| {
        super::player_block_collision_shape_checked(
            &world.level_by_id(&DIM.into()).chunks,
            world.levels.player_at(connection),
            pos,
        )
    };
    tags(&mut world, None);
    assert_eq!(shapes(&world), Some(full()));
    world.levels[connection].block_contact.fall_distance = 2.5_f64.next_up();
    assert_eq!(shapes(&world), Some(slab(f64::from(0.9_f32))));
    world.levels[connection].block_contact.fall_distance = 0.0;
    tags(&mut world, Some(&[]));
    assert_eq!(shapes(&world), Some(full()));
    world.levels[connection].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
    assert_eq!(shapes(&world), Some(Vec::new()));
    world.levels[connection].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, false);
    *world.levels[connection].inventory.slot_mut(feet).unwrap() =
        bbb_server_types::item_stack::RuntimeItemStack::empty();
    assert_eq!(shapes(&world), Some(Vec::new()));

    block(&mut world, pos, state("minecraft:scaffolding"));
    let top = shapes(&world).unwrap();
    assert!(!top.is_empty());
    world.levels[connection].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
    assert_eq!(shapes(&world), Some(Vec::new()));
    block(&mut world, pos, state("minecraft:stone"));
    assert_eq!(
        shapes(&world),
        super::block_collision_shape_checked(&world.level_by_id(&DIM.into()).chunks, pos, None)
    );
}

#[test]
fn checked_entity_block_reads_refuse_unloaded_columns_and_keep_height_filler() {
    let (mut world, _) = fixture();
    let pos = POSITIONS[0];
    block(&mut world, pos, state("minecraft:stone"));
    let min_y = world.level_by_id(&DIM.into()).chunks.min_y();
    let filler = state("minecraft:void_air");
    world.with_stored_hurt_level(&DIM.into(), |_, level| {
        let below_world = BlockPos {
            y: min_y - 1,
            ..pos
        };
        assert!(level.has_ticking_column_at(below_world));
        assert!(!level.has_chunk_at(below_world));
        assert!(!level.has_ticking_column_at(BlockPos { x: 100_000, ..pos }));
        assert_eq!(
            level.block_state_checked(pos),
            Some(state("minecraft:stone"))
        );
        assert_eq!(
            level.block_state_checked(BlockPos { x: 100_000, ..pos }),
            None
        );
        assert_eq!(
            level.block_state_checked(BlockPos {
                x: 100_000,
                y: min_y - 1,
                ..pos
            }),
            Some(filler)
        );
    });
}
