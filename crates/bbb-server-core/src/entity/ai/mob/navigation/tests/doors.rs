use super::*;

pub(super) fn door_corridor(resource_id: &str, open: bool) -> NativeLevel {
    let mut native = NativeLevel::new();
    for x in 0..=6 {
        for y in 1..=4 {
            for z in -1..=1 {
                if z != 0 || x == 0 || x == 6 || y >= 3 {
                    native.world.set_block(
                        crate::entity::tests::native_level::DIMENSION,
                        BlockPos { x, y: y + 63, z },
                        state("minecraft:stone"),
                        2 | 16,
                    );
                }
            }
        }
    }
    for (y, half) in [(1, "lower"), (2, "upper")] {
        let state = state_with(
            resource_id,
            &[
                ("facing", "north"),
                ("half", half),
                ("hinge", "left"),
                ("open", if open { "true" } else { "false" }),
                ("powered", "false"),
            ],
        );
        native.world.set_block(
            crate::entity::tests::native_level::DIMENSION,
            BlockPos {
                x: 3,
                y: y + 63,
                z: 0,
            },
            state,
            2 | 16,
        );
    }
    native
}

#[test]
fn closed_wooden_door_path_requires_open_and_pass_permissions() {
    let mut native = door_corridor("minecraft:oak_door", false);
    native.with_level(|level| {
        let door = BlockPos { x: 3, y: 64, z: 0 };
        assert_eq!(
            ground_path_type_from_state(level, door),
            Some(GroundPathType::DoorWoodClosed)
        );
        for open in [false, true] {
            for pass in [false, true] {
                let mut mob = snapshot(Vec3d {
                    x: 1.5,
                    y: 64.0,
                    z: 0.5,
                });
                mob.can_open_doors = open;
                mob.can_pass_doors = pass;
                let path = find_path(level, mob, BlockPos { x: 5, y: 64, z: 0 }, 16.0, 0, 512);
                assert_eq!(
                    path.as_ref().is_some_and(GroundPath::reached),
                    open && pass,
                    "open={open}, pass={pass}"
                );
                if open && pass {
                    assert!(path
                        .unwrap()
                        .nodes()
                        .iter()
                        .any(|n| n.pos == door && n.path_type == GroundPathType::WalkableDoor));
                }
            }
        }
    });
}

#[test]
fn open_wood_and_iron_door_paths_follow_pass_permission_only() {
    for resource_id in ["minecraft:oak_door", "minecraft:iron_door"] {
        let mut native = door_corridor(resource_id, true);
        native.with_level(|level| {
            assert_eq!(
                ground_path_type_from_state(level, BlockPos { x: 3, y: 64, z: 0 }),
                Some(GroundPathType::DoorOpen)
            );
            for open in [false, true] {
                for pass in [false, true] {
                    let mut mob = snapshot(Vec3d {
                        x: 1.5,
                        y: 64.0,
                        z: 0.5,
                    });
                    mob.can_open_doors = open;
                    mob.can_pass_doors = pass;
                    let path = find_path(level, mob, BlockPos { x: 5, y: 64, z: 0 }, 16.0, 0, 512);
                    assert_eq!(
                        path.as_ref().is_some_and(GroundPath::reached),
                        pass,
                        "{resource_id}, open={open}, pass={pass}"
                    );
                    if pass {
                        assert!(path
                            .unwrap()
                            .nodes()
                            .iter()
                            .any(|n| n.path_type == GroundPathType::DoorOpen));
                    }
                }
            }
        });
    }
}

#[test]
fn closed_iron_door_remains_impassable_with_open_permission() {
    let mut native = door_corridor("minecraft:iron_door", false);
    native.with_level(|level| {
        assert_eq!(
            ground_path_type_from_state(level, BlockPos { x: 3, y: 64, z: 0 }),
            Some(GroundPathType::DoorIronClosed)
        );
        for pass in [false, true] {
            let mut mob = snapshot(Vec3d {
                x: 1.5,
                y: 64.0,
                z: 0.5,
            });
            mob.can_open_doors = true;
            mob.can_pass_doors = pass;
            assert!(
                !find_path(level, mob, BlockPos { x: 5, y: 64, z: 0 }, 16.0, 0, 512)
                    .is_some_and(|p| p.reached())
            );
        }
    });
}
