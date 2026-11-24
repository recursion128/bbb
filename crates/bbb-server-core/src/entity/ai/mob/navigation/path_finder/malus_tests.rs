use super::super::malus::PathfindingMalus;
use super::*;
use crate::entity::tests::native_level::{state, NativeLevel};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
};
use bbb_protocol::packets::Vec3d;

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y: y + 63, z }
}
fn mob(type_id: i32) -> MobPathingSnapshot {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    MobPathingSnapshot {
        creaking_home_pos: None,
        amphibious: false,
        frog: false,
        inherits_malus: false,
        position,
        block_position: pos(0, 1, 0),
        bounding_box: bbb_server_types::geometry::Aabb::of_entity(position, 0.6, 1.95),
        width: 0.6,
        height: 1.95,
        on_ground: true,
        in_water: false,
        in_liquid: false,
        on_fire: false,
        can_float: false,
        can_walk_over_fences: false,
        can_stand_on_lava: false,
        can_open_doors: false,
        can_pass_doors: true,
        is_passenger: false,
        speed: 0.25,
        target_health: None,
        follow_range: 16.0,
        step_height: 0.6,
        max_fall_distance: 3,
        malus: PathfindingMalus::for_entity_type(type_id),
    }
}

#[test]
fn ravager_leaf_malus_opens_the_same_corridor_rejected_for_pillager() {
    let mut native = NativeLevel::new();
    for z in -16..=16 {
        for y in 1..=4 {
            native.set(pos(2, y, z), state("minecraft:oak_leaves"));
        }
    }
    let target = pos(4, 1, 0);
    let ravager = mob(VANILLA_ENTITY_TYPE_RAVAGER_ID);
    let pillager = mob(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    assert!(native
        .with_level(|level| find_path(level, ravager, target, 16.0, 0, 1024))
        .unwrap()
        .reached());
    assert!(!native
        .with_level(|level| find_path(level, pillager, target, 16.0, 0, 1024))
        .is_some_and(|path| path.reached()));
    assert_eq!(
        native
            .with_level(|level| node_at(
                level,
                ravager,
                pos(2, 1, 0),
                &mut WalkPathCache::default()
            ))
            .unwrap()
            .public
            .cost_malus,
        0.0
    );
    assert!(native
        .with_level(|level| accepted_start(
            level,
            pillager,
            pos(2, 1, 0),
            &mut WalkPathCache::default()
        ))
        .is_none());
}

#[test]
fn cow_fire_malus_refuses_fire_and_routes_around_its_risk() {
    let mut native = NativeLevel::new();
    native.set(pos(2, 1, 0), state("minecraft:fire"));
    let cow = mob(VANILLA_ENTITY_TYPE_COW_ID);
    assert!(native
        .with_level(|level| accepted_node(
            level,
            cow,
            pos(2, 1, 0),
            0,
            64.0,
            GroundPathType::Walkable,
            (1, 0),
            &mut WalkPathCache::default(),
        ))
        .is_none());
    assert_eq!(
        native
            .with_level(|level| node_at(level, cow, pos(1, 1, 0), &mut WalkPathCache::default()))
            .unwrap()
            .public
            .cost_malus,
        16.0
    );
    let path = native
        .with_level(|level| find_path(level, cow, pos(4, 1, 0), 16.0, 0, 1024))
        .unwrap();
    assert!(path.reached());
    assert!(path.nodes().iter().all(|n| !matches!(
        n.path_type,
        GroundPathType::Fire | GroundPathType::FireInNeighbor
    )));
}

#[test]
fn runtime_malus_controls_water_fall_and_closed_node_costs() {
    let mut native = NativeLevel::new();
    native.set(pos(1, 1, 0), state("minecraft:water"));
    native.set(pos(1, 2, 0), state("minecraft:water"));
    let mut mob = mob(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    mob.malus.set(GroundPathType::Water, 3.0);
    let water = native
        .with_level(|level| {
            first_non_water_below(
                level,
                mob,
                pos(1, 3, 0),
                None,
                &mut WalkPathCache::default(),
            )
        })
        .unwrap();
    assert_eq!(water.public.cost_malus, 3.0);
    let mut native = NativeLevel::new();
    mob.malus.set(GroundPathType::Walkable, 7.0);
    assert_eq!(
        native
            .with_level(|level| ground_below(
                level,
                mob,
                pos(0, 3, 0),
                &mut WalkPathCache::default()
            ))
            .unwrap()
            .public
            .cost_malus,
        7.0
    );
    mob.malus.set(GroundPathType::Walkable, -4.0);
    assert_eq!(
        native
            .with_level(|level| ground_below(
                level,
                mob,
                pos(0, 3, 0),
                &mut WalkPathCache::default()
            ))
            .unwrap()
            .public
            .cost_malus,
        -1.0
    );
    native.set(pos(2, 1, 0), state("minecraft:oak_fence"));
    mob.malus.set(GroundPathType::Fence, -9.0);
    let closed = native
        .with_level(|level| {
            accepted_node(
                level,
                mob,
                pos(2, 1, 0),
                0,
                64.0,
                GroundPathType::Walkable,
                (1, 0),
                &mut WalkPathCache::default(),
            )
        })
        .unwrap();
    assert!(closed.closed);
    assert_eq!(
        closed.public.cost_malus,
        GroundPathType::Fence.default_malus()
    );
}

#[test]
fn volume_malus_selects_negative_then_highest_and_respects_large_mob_cap() {
    let mut native = NativeLevel::new();
    native.set(pos(1, 1, 0), state("minecraft:water"));
    let mut mob = mob(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    mob.width = 1.2;
    mob.height = 0.5;
    mob.malus.set(GroundPathType::Water, 20.0);
    mob.malus.set(GroundPathType::WaterBorder, 4.0);
    assert_eq!(
        native.with_level(|level| {
            path_type_for_mob(level, mob, pos(0, 1, 0), &mut WalkPathCache::default())
        }),
        Some(GroundPathType::BigMobsCloseToDanger)
    );
    mob.malus.set(GroundPathType::BigMobsCloseToDanger, 20.0);
    assert_eq!(
        native.with_level(|level| {
            path_type_for_mob(level, mob, pos(0, 1, 0), &mut WalkPathCache::default())
        }),
        Some(GroundPathType::Water)
    );
    mob.malus.set(GroundPathType::WaterBorder, -3.0);
    assert_eq!(
        native.with_level(|level| {
            path_type_for_mob(level, mob, pos(0, 1, 0), &mut WalkPathCache::default())
        }),
        Some(GroundPathType::WaterBorder)
    );
    for y in -64..=63 {
        native.world.set_block(
            crate::entity::tests::native_level::DIMENSION,
            BlockPos { x: 0, y, z: 0 },
            state("minecraft:air"),
            2 | 16,
        );
    }
    native.set(pos(1, 1, 0), state("minecraft:air"));
    mob.malus = PathfindingMalus::default();
    assert_eq!(
        native.with_level(|level| {
            path_type_for_mob(level, mob, pos(0, 1, 0), &mut WalkPathCache::default())
        }),
        Some(GroundPathType::Walkable)
    );
    mob.width = 0.6;
    assert_eq!(
        native.with_level(|level| {
            path_type_for_mob(level, mob, pos(0, 1, 0), &mut WalkPathCache::default())
        }),
        Some(GroundPathType::Open)
    );
}

#[test]
fn runtime_malus_above_the_current_node_controls_jump_admission() {
    let mut native = NativeLevel::new();
    native.set(pos(1, 1, 0), state("minecraft:stone"));
    let mut mob = mob(VANILLA_ENTITY_TYPE_PILLAGER_ID);
    let current = native
        .with_level(|level| node_at(level, mob, pos(0, 1, 0), &mut WalkPathCache::default()))
        .unwrap()
        .public
        .cost_malus;
    assert!(native
        .with_level(|level| neighbors(level, mob, pos(0, 1, 0), current))
        .iter()
        .any(|n| n.public.pos == pos(1, 2, 0)));
    mob.malus.set(GroundPathType::Open, -1.0);
    assert!(!native
        .with_level(|level| neighbors(level, mob, pos(0, 1, 0), current))
        .iter()
        .any(|n| n.public.pos == pos(1, 2, 0)));
}

#[test]
fn constructor_malus_opens_or_closes_a_hazard_strip() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_FOX_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    for (block, y, entity_type, reached) in [
        ("minecraft:lava", 0, VANILLA_ENTITY_TYPE_BLAZE_ID, true),
        (
            "minecraft:lava",
            0,
            VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
            true,
        ),
        ("minecraft:lava", 0, VANILLA_ENTITY_TYPE_ZOMBIE_ID, false),
        (
            "minecraft:sweet_berry_bush",
            1,
            VANILLA_ENTITY_TYPE_FOX_ID,
            true,
        ),
        (
            "minecraft:sweet_berry_bush",
            1,
            VANILLA_ENTITY_TYPE_COW_ID,
            false,
        ),
        ("minecraft:water", 0, VANILLA_ENTITY_TYPE_ENDERMAN_ID, false),
        ("minecraft:water", 0, VANILLA_ENTITY_TYPE_ZOMBIE_ID, true),
    ] {
        let mut native = NativeLevel::new();
        for z in -16..=16 {
            if y == 1 {
                native.set(pos(2, 0, z), state("minecraft:grass_block"));
            }
            native.set(pos(2, y, z), state(block));
        }
        let path = native
            .with_level(|level| find_path(level, mob(entity_type), pos(4, 1, 0), 16.0, 0, 1024));
        assert_eq!(
            path.is_some_and(|path| path.reached()),
            reached,
            "{block} {entity_type}"
        );
    }
}

#[test]
fn wolf_refuses_to_stand_on_powder_snow() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID;
    let mut native = NativeLevel::new();
    native.set(pos(2, 0, 0), state("minecraft:powder_snow"));
    let wolf = mob(VANILLA_ENTITY_TYPE_WOLF_ID);
    let cow = mob(VANILLA_ENTITY_TYPE_COW_ID);
    let mut top = |mob| {
        native
            .with_level(|level| node_at(level, mob, pos(2, 1, 0), &mut WalkPathCache::default()))
            .unwrap()
            .public
    };
    let wolf_top = top(wolf);
    assert_eq!(wolf_top.path_type, GroundPathType::OnTopOfPowderSnow);
    assert_eq!(wolf_top.cost_malus, -1.0);
    assert_eq!(top(cow).cost_malus, 0.0);
    assert!(!native
        .with_level(|level| find_path(level, wolf, pos(2, 1, 0), 16.0, 0, 1024))
        .is_some_and(|path| path.reached()));
}
