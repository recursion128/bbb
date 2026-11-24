use std::collections::BTreeSet;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::save::SaveResult;

use crate::chunk::blending::{generation_block_state_is_tagged, BlendingBorderTickWitness};

use super::invalid_schema;
use bbb_server_types::chunk_data::ChunkColumn;

pub(super) fn generate_border_ticks(
    target: (i32, i32),
    column: &ChunkColumn,
    postprocessing_positions: &mut Vec<(i32, i32, i32)>,
    heightmaps: &FeatureHeightmaps,
    witness: &BlendingBorderTickWitness,
    registry_tags: Option<&RegistryTagSnapshot>,
) -> SaveResult<()> {
    let path = format!("FEATURES blending border [{},{}]", target.0, target.1);
    let origin_x = target
        .0
        .checked_mul(16)
        .ok_or_else(|| invalid_schema(&path, "target block x origin overflows"))?;
    let origin_z = target
        .1
        .checked_mul(16)
        .ok_or_else(|| invalid_schema(&path, "target block z origin overflows"))?;
    let upper_border_y = witness
        .old_max_y
        .checked_add(1)
        .ok_or_else(|| invalid_schema(&path, "old-generation upper border overflows"))?;
    let lower_border_y = witness
        .old_min_y
        .checked_sub(1)
        .ok_or_else(|| invalid_schema(&path, "old-generation lower border overflows"))?;
    let mut positions = postprocessing_positions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    // `Blender.generateBorderTicks` scans x-major, z-minor and each old-area
    // plane in lower-to-upper declaration order (`Blender.java:265-283`).
    for local_x in 0..16 {
        for local_z in 0..16 {
            for y in [
                lower_border_y,
                witness.old_min_y,
                witness.old_max_y,
                upper_border_y,
            ] {
                mark_border_tick(
                    column,
                    postprocessing_positions,
                    &mut positions,
                    [origin_x, origin_z],
                    [local_x, y, local_z],
                    registry_tags,
                )?;
            }
        }
    }

    // Direction.Plane.HORIZONTAL is NORTH, EAST, SOUTH, WEST. Each edge uses
    // the target MOTION_BLOCKING height (`Blender.java:286-301`).
    let edges = [
        (0, 15, 0, 0),
        (15, 15, 0, 15),
        (0, 15, 15, 15),
        (0, 0, 0, 15),
    ];
    for (index, (min_x, max_x, min_z, max_z)) in edges.iter().copied().enumerate() {
        if witness.neighbor_old[index] {
            continue;
        }
        for local_x in min_x..=max_x {
            for local_z in min_z..=max_z {
                let first_available = heightmaps
                    .first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, local_x, local_z)
                    .ok_or_else(|| invalid_schema(&path, "missing MOTION_BLOCKING height"))?;
                let max_y = border_edge_max_y(witness.old_max_y, first_available, &path)?;
                for y in witness.old_min_y..max_y {
                    mark_border_tick(
                        column,
                        postprocessing_positions,
                        &mut positions,
                        [origin_x, origin_z],
                        [local_x, y, local_z],
                        registry_tags,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn border_edge_max_y(old_max_y: i32, first_available: i32, path: &str) -> SaveResult<i32> {
    // ChunkAccess.getHeight returns highest-taken, not first-available
    // (`ChunkAccess.java:182-194`; `Heightmap.java:110-118`).
    let highest_taken = first_available
        .checked_sub(1)
        .ok_or_else(|| invalid_schema(path, "MOTION_BLOCKING highest-taken underflows"))?;
    old_max_y
        .min(highest_taken)
        .checked_add(1)
        .ok_or_else(|| invalid_schema(path, "border height overflows"))
}

fn mark_border_tick(
    column: &ChunkColumn,
    postprocessing_positions: &mut Vec<(i32, i32, i32)>,
    positions: &mut BTreeSet<(i32, i32, i32)>,
    [origin_x, origin_z]: [i32; 2],
    [local_x, y, local_z]: [i32; 3],
    registry_tags: Option<&RegistryTagSnapshot>,
) -> SaveResult<()> {
    let Some(state) = column.get_block_state(local_x, y, local_z) else {
        return Ok(());
    };
    // `Blender.generateBorderTick` marks live leaves and non-empty fluids independently
    // (`Blender.java:308-316`).
    if !generation_block_state_is_tagged(registry_tags, "minecraft:leaves", state)
        && block_state_fluid_state(state).is_empty()
    {
        return Ok(());
    }
    let position = (
        origin_x
            .checked_add(local_x)
            .ok_or_else(|| invalid_schema("FEATURES blending border", "block x overflows"))?,
        y,
        origin_z
            .checked_add(local_z)
            .ok_or_else(|| invalid_schema("FEATURES blending border", "block z overflows"))?,
    );
    if positions.insert(position) {
        postprocessing_positions.push(position);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::registry_tags::RegistryTagSnapshot;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_server_types::chunk_data::ChunkColumn;

    use super::*;

    fn state(name: &str) -> i32 {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    }

    fn target() -> (ChunkColumn, Vec<(i32, i32, i32)>) {
        let mut column = ChunkColumn::new(-16, 4, state("minecraft:air"), 0);
        let leaves = state("minecraft:oak_leaves");
        let water = state("minecraft:water");
        let stone = state("minecraft:stone");
        for y in [-1, 0, 15, 16] {
            column.set_block_state(1, y, 1, leaves).unwrap();
        }
        column.set_block_state(2, 0, 2, water).unwrap();
        column.set_block_state(3, 4, 0, stone).unwrap();
        column.set_block_state(3, 1, 0, leaves).unwrap();
        column.set_block_state(3, 17, 0, leaves).unwrap();
        column.set_block_state(0, 2, 0, leaves).unwrap();
        (column, vec![(32, 2, -48)])
    }

    fn registry_tags(leaves: &[&str]) -> RegistryTagSnapshot {
        RegistryTagSnapshot::new([(
            "minecraft:block".to_owned(),
            "minecraft:leaves".to_owned(),
            leaves.iter().map(|member| (*member).to_owned()).collect(),
        )])
    }

    #[test]
    fn blending_border_ticks_preserve_plane_edge_height_and_dedup_order() {
        let (chunk, mut positions) = target();
        let heightmaps = FeatureHeightmaps::prime(&chunk);
        let registry_tags = registry_tags(&["minecraft:oak_leaves"]);
        generate_border_ticks(
            (2, -3),
            &chunk,
            &mut positions,
            &heightmaps,
            &BlendingBorderTickWitness {
                old_min_y: 0,
                old_max_y: 15,
                neighbor_old: [false, true, true, false],
            },
            Some(&registry_tags),
        )
        .unwrap();

        let positions = &positions;
        assert_eq!(positions[0], (32, 2, -48));
        assert_eq!(
            positions[1..5],
            [(33, -1, -47), (33, 0, -47), (33, 15, -47), (33, 16, -47)]
        );
        assert!(positions.contains(&(34, 0, -46)));
        assert!(positions.contains(&(35, 1, -48)));
        assert!(!positions.contains(&(35, 17, -48)));
        assert_eq!(
            positions.iter().filter(|&&pos| pos == (32, 2, -48)).count(),
            1
        );
    }

    #[test]
    fn blending_border_ticks_skip_same_age_edges_but_keep_old_target_planes() {
        let (chunk, mut positions) = target();
        let heightmaps = FeatureHeightmaps::prime(&chunk);
        let registry_tags = registry_tags(&["minecraft:oak_leaves"]);
        generate_border_ticks(
            (2, -3),
            &chunk,
            &mut positions,
            &heightmaps,
            &BlendingBorderTickWitness {
                old_min_y: 0,
                old_max_y: 15,
                neighbor_old: [true; 4],
            },
            Some(&registry_tags),
        )
        .unwrap();

        assert!(!positions.as_slice().contains(&(35, 1, -48)));
        assert!(positions.as_slice().contains(&(33, -1, -47)));
        assert!(positions.as_slice().contains(&(34, 0, -46)));
    }

    #[test]
    fn blending_border_ticks_use_highest_taken_for_edge_exclusive_bound() {
        assert_eq!(border_edge_max_y(15, 5, "test").unwrap(), 5);
        assert_eq!(border_edge_max_y(3, 9, "test").unwrap(), 4);
    }

    fn border_positions(registry_tags: Option<&RegistryTagSnapshot>) -> Vec<(i32, i32, i32)> {
        let (chunk, mut positions) = target();
        let heightmaps = FeatureHeightmaps::prime(&chunk);
        generate_border_ticks(
            (2, -3),
            &chunk,
            &mut positions,
            &heightmaps,
            &BlendingBorderTickWitness {
                old_min_y: 0,
                old_max_y: 15,
                neighbor_old: [true; 4],
            },
            registry_tags,
        )
        .unwrap();
        positions.as_slice().to_vec()
    }

    #[test]
    fn blending_border_ticks_use_live_leaves_and_keep_fluid_independent() {
        let leaf_position = (33, -1, -47);
        let fluid_position = (34, 0, -46);
        let builtin = border_positions(None);
        assert!(builtin.contains(&leaf_position));
        assert!(builtin.contains(&fluid_position));

        let empty = registry_tags(&[]);
        let empty_positions = border_positions(Some(&empty));
        assert!(!empty_positions.contains(&leaf_position));
        assert!(empty_positions.contains(&fluid_position));

        let replacement = registry_tags(&["minecraft:oak_leaves"]);
        let replacement_positions = border_positions(Some(&replacement));
        assert!(replacement_positions.contains(&leaf_position));
        assert!(replacement_positions.contains(&fluid_position));
    }
}
