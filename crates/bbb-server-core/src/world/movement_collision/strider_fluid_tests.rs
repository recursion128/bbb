use std::sync::Arc;

use crate::entity::{EntityData, FallingBlockData, TntData};
use crate::fluid::with_property;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TNT_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::save::Java26BuiltinDimension;

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

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id).unwrap()
}

fn fixture() -> (ServerWorld, i32) {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    for pos in POSITIONS {
        world
            .level_mut(DIM)
            .chunks
            .load_column(pos.x >> 4, pos.z >> 4);
    }
    let id = world
        .level_mut(DIM)
        .entities
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, Vec3d::default())
        .unwrap();
    assert!(world
        .level(DIM)
        .entities
        .get(id)
        .unwrap()
        .living()
        .is_some());
    (world, id)
}

fn block(world: &mut ServerWorld, pos: BlockPos, state: i32) {
    world.set_raw_block_state_for_test(DIM, pos, state);
}

fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y + 1,
        ..pos
    }
}

fn context(world: &mut ServerWorld, id: i32, pos: BlockPos, bottom: f64, shift: bool) {
    let body = world.level_mut(DIM).entities.get_mut(id).unwrap();
    body.set_pos(Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: bottom,
        z: f64::from(pos.z) + 0.5,
    });
    body.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
}

fn shapes(world: &mut ServerWorld, id: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    world.with_stored_hurt_level(&DIM.into(), |store, level| {
        level.block_collision_boxes_for_entity(store.get(id).unwrap(), pos)
    })
}

fn tags(world: &mut ServerWorld, members: Option<&[&str]>) {
    let tags = members.map(|members| {
        (
            "minecraft:fluid".to_owned(),
            "minecraft:lava".to_owned(),
            members.iter().map(|member| (*member).to_owned()).collect(),
        )
    });
    world
        .level_mut(DIM)
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

fn half() -> Vec<Aabb> {
    vec![Aabb {
        max_y: 0.5,
        ..Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })
    }]
}

#[test]
fn strider_fluid_tests_strict_bottom_and_shift_use_local_half_surface() {
    let (mut world, id) = fixture();
    tags(&mut world, Some(&["minecraft:lava"]));
    // EntityCollisionContext.java:78-80; Strider.java:331-333.
    for pos in POSITIONS {
        block(&mut world, pos, state("minecraft:lava"));
        block(&mut world, above(pos), state("minecraft:air"));
        let boundary = f64::from(pos.y) + 0.5 - f64::from(1.0e-5_f32);
        for bottom in [
            boundary.next_down(),
            boundary,
            boundary.next_up(),
            f64::from(pos.y) + 0.5,
        ] {
            for shift in [false, true] {
                context(&mut world, id, pos, bottom, shift);
                assert_eq!(
                    shapes(&mut world, id, pos),
                    Some(if bottom > boundary { half() } else { vec![] }),
                    "pos={pos:?}, bottom={bottom}, shift={shift}"
                );
            }
        }
    }
}

#[test]
fn strider_fluid_tests_current_source_identity_uses_live_custom_lava_tag() {
    let (mut world, id) = fixture();
    let pos = POSITIONS[0];
    context(&mut world, id, pos, f64::from(pos.y) + 1.0, false);
    block(&mut world, above(pos), state("minecraft:air"));
    tags(&mut world, Some(&["minecraft:lava"]));
    block(&mut world, pos, state("minecraft:water"));
    assert_eq!(shapes(&mut world, id, pos), Some(vec![]));
    tags(&mut world, Some(&["minecraft:water"]));
    assert_eq!(shapes(&mut world, id, pos), Some(half()));
    block(&mut world, pos, state("minecraft:lava"));
    assert_eq!(shapes(&mut world, id, pos), Some(vec![]));
    tags(
        &mut world,
        Some(&["minecraft:flowing_lava", "minecraft:flowing_water"]),
    );
    for name in ["minecraft:water", "minecraft:lava"] {
        block(&mut world, pos, state(name));
        assert_eq!(shapes(&mut world, id, pos), Some(vec![]));
    }
    tags(
        &mut world,
        Some(&[
            "minecraft:lava",
            "minecraft:water",
            "minecraft:flowing_lava",
            "minecraft:flowing_water",
        ]),
    );
    for name in ["minecraft:water", "minecraft:lava"] {
        for level in 0..16 {
            block(
                &mut world,
                pos,
                with_property(state(name), "level", &level.to_string()).unwrap(),
            );
            assert_eq!(
                shapes(&mut world, id, pos),
                Some(if level == 0 { half() } else { vec![] }),
                "{name} level={level}"
            );
        }
    }
}

#[test]
fn strider_fluid_tests_above_source_flowing_and_waterlogged_share_family() {
    let (mut world, id) = fixture();
    tags(&mut world, Some(&["minecraft:water", "minecraft:lava"]));
    let wet_slab = with_property(state("minecraft:oak_slab"), "waterlogged", "true").unwrap();
    for pos in POSITIONS {
        context(&mut world, id, pos, f64::from(pos.y) + 1.0, true);
        for current in ["minecraft:water", "minecraft:lava"] {
            block(&mut world, pos, state(current));
            for (name, blocked) in [
                ("minecraft:water", current == "minecraft:water"),
                ("minecraft:lava", current == "minecraft:lava"),
            ] {
                for level in 0..16 {
                    block(
                        &mut world,
                        above(pos),
                        with_property(state(name), "level", &level.to_string()).unwrap(),
                    );
                    assert_eq!(
                        shapes(&mut world, id, pos),
                        Some(if blocked { vec![] } else { half() }),
                        "current={current}, above={name} level={level}, {pos:?}"
                    );
                }
            }
            for (above_state, blocked) in [
                (state("minecraft:air"), false),
                (state("minecraft:stone"), false),
                (wet_slab, current == "minecraft:water"),
                (state("minecraft:seagrass"), current == "minecraft:water"),
            ] {
                block(&mut world, above(pos), above_state);
                assert_eq!(
                    shapes(&mut world, id, pos),
                    Some(if blocked { vec![] } else { half() })
                );
            }
        }
    }
}

#[test]
fn strider_fluid_tests_ordinary_living_and_nonliving_have_no_liquid_shape() {
    let (mut world, strider) = fixture();
    let pos = POSITIONS[0];
    let store = &mut world.level_mut(DIM).entities;
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d::default())
        .unwrap();
    let stand = crate::entity::armor_stand::spawn_summoned(store, Vec3d::default()).unwrap();
    let item = crate::entity::spawn_item_entity(
        store,
        Vec3d::default(),
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    let orb = crate::entity::spawn_experience_orb(store, Vec3d::default(), 1).unwrap();
    let tnt = store.spawn(
        VANILLA_ENTITY_TYPE_TNT_ID,
        Vec3d::default(),
        EntityData::Tnt(TntData::with_saved_fields(
            50,
            state("minecraft:tnt"),
            None,
            4.0,
        )),
    );
    let falling = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d::default(),
        EntityData::FallingBlock(FallingBlockData::new(state("minecraft:sand"), pos)),
    );
    tags(&mut world, None);
    block(&mut world, above(pos), i32::MAX);
    for id in [cow, stand, item, orb, tnt, falling] {
        context(&mut world, id, pos, f64::from(pos.y) + 1.0, true);
        for name in ["minecraft:lava", "minecraft:water"] {
            block(&mut world, pos, state(name));
            assert_eq!(shapes(&mut world, id, pos), Some(vec![]), "id={id}, {name}");
        }
    }
    context(&mut world, strider, pos, f64::from(pos.y) + 1.0, false);
    assert_eq!(shapes(&mut world, strider, pos), Some(vec![]));
}

#[test]
fn strider_fluid_tests_empty_and_piston_moved_empty_context_stay_empty() {
    let (mut world, id) = fixture();
    let pos = POSITIONS[0];
    context(&mut world, id, pos, f64::from(pos.y) + 1.0, false);
    tags(&mut world, Some(&["minecraft:lava", "minecraft:water"]));
    block(&mut world, above(pos), state("minecraft:air"));
    for name in ["minecraft:lava", "minecraft:water"] {
        block(&mut world, pos, state(name));
        assert_eq!(shapes(&mut world, id, pos), Some(half()));
        assert_eq!(
            world.with_stored_hurt_level(&DIM.into(), |_, level| {
                level.block_collision_boxes_empty(pos)
            }),
            Some(vec![])
        );
        block(&mut world, pos, state("minecraft:moving_piston"));
        let chunks = &mut world.level_mut(DIM).chunks;
        chunks.block_entities_mut().add_and_register(
            crate::block_entity::new_block_entity(pos, state("minecraft:moving_piston")).unwrap(),
        );
        assert!(chunks
            .block_entities_mut()
            .get_mut(pos)
            .unwrap()
            .configure_piston(state(name), Direction::East, true, false));
        assert_eq!(shapes(&mut world, id, pos), Some(vec![]));
    }
}
