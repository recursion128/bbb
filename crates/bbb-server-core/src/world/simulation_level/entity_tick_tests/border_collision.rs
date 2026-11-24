use super::*;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::interaction::WorldBorder;

#[test]
fn teleport_border_level_projects_configured_dimension_snapshot() {
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
    let dimension = Java26BuiltinDimension::Nether.into();
    let border = WorldBorder::from_settings(2.25, -3.25, 0.2, 5.0, 5, 300, 10.0, 0, 10.0, 0);
    world.level_mut_by_id(&dimension).world_border = border;
    world.with_stored_hurt_level(&dimension, |_, level| {
        let projected = LevelCtx::world_border(level).unwrap();
        assert_eq!(projected, border);
        assert_ne!(projected, WorldBorder::DEFAULT);
        let bounds = Aabb {
            min_x: 7.0,
            max_x: 8.1,
            min_y: 64.0,
            max_y: 66.0,
            min_z: -3.5,
            max_z: -3.0,
        };
        assert!(!projected.no_collision_at(
            Vec3d {
                x: 7.55,
                y: 64.0,
                z: -3.25
            },
            bounds,
            0
        ));
    });
}

#[test]
fn teleport_border_has_chunk_at_reads_only_usable_column() {
    with_level(|level| {
        level.chunks.load_column(0, 0);
        assert!(level.chunks.is_ticking_full(0, 0));
        let min_y = level.chunks.min_y();
        let max_y = min_y + level.chunks.section_count() as i32 * 16;
        for y in [i32::MIN, min_y - 1, min_y, max_y, i32::MAX] {
            assert!(LevelCtx::has_ticking_column_at(
                level,
                BlockPos { x: 0, y, z: 0 }
            ));
            assert!(!LevelCtx::has_ticking_column_at(
                level,
                BlockPos {
                    x: 160_000,
                    y,
                    z: 0
                }
            ));
        }
        assert!(!level.has_chunk_at(BlockPos {
            x: 0,
            y: min_y - 1,
            z: 0
        }));
    });
}
