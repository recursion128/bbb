//! Vanilla sky-light section occupancy gizmos.
//!
//! Owns `LightSectionDebugRenderer`'s 500 ms snapshot cache and the exact
//! `DiscreteVoxelShape.forAllEdges(..., true)` / `forAllFaces` traversal used
//! to turn the 21^3 section-type volume into merged lines and boundary quads.

use std::time::{Duration, Instant};

use bbb_renderer::{DebugCuboidFace, DebugFilledRect, DebugLine};
use bbb_world::{BlockPos, WorldStore};

use crate::{debug_entries::DebugScreenEntryId, input::ClientInputState};

pub(super) const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
pub(super) const RADIUS: i32 = 10;
pub(super) const LIGHT_AND_BLOCKS_FACE_COLOR: [u8; 4] = [255, 255, 0, 63];
pub(super) const LIGHT_ONLY_FACE_COLOR: [u8; 4] = [63, 31, 0, 31];
pub(super) const LIGHT_AND_BLOCKS_LINE_COLOR: [u8; 4] = [255, 255, 0, 255];
pub(super) const LIGHT_ONLY_LINE_COLOR: [u8; 4] = [63, 31, 0, 255];
pub(super) const LINE_WIDTH: f32 = 3.0;

const SIZE: i32 = RADIUS * 2 + 1;

#[derive(Debug, Default)]
pub(crate) struct DebugSkyLightSectionCache {
    last_update_at: Option<Instant>,
    rects: Vec<DebugFilledRect>,
    lines: Vec<DebugLine>,
}

impl DebugSkyLightSectionCache {
    pub(super) fn gizmos(
        &mut self,
        input: &ClientInputState,
        world: &WorldStore,
        now: Instant,
    ) -> (Vec<DebugFilledRect>, Vec<DebugLine>) {
        if !input.debug_screen_entry_enabled(
            DebugScreenEntryId::VisualizeSkyLightSections,
            input.show_only_reduced_info(world),
        ) || world.level_info().is_none()
        {
            self.reset();
            return (Vec::new(), Vec::new());
        }
        let Some(player_position) = world
            .local_player_pose()
            .map(|pose| [pose.position.x, pose.position.y, pose.position.z])
        else {
            self.reset();
            return (Vec::new(), Vec::new());
        };
        if !player_position
            .iter()
            .all(|coordinate| coordinate.is_finite())
        {
            self.reset();
            return (Vec::new(), Vec::new());
        }

        let refresh = self.last_update_at.is_none_or(|last_update_at| {
            now.saturating_duration_since(last_update_at) > REFRESH_INTERVAL
        });
        if refresh {
            self.last_update_at = Some(now);
            let player_block = BlockPos {
                x: player_position[0].floor() as i32,
                y: player_position[1].floor() as i32,
                z: player_position[2].floor() as i32,
            };
            let center_section = [
                player_block.x.div_euclid(16),
                player_block.y.div_euclid(16),
                player_block.z.div_euclid(16),
            ];
            let min_section = center_section.map(|coordinate| coordinate.saturating_sub(RADIUS));
            let (light_and_blocks, light) = section_shapes(world, min_section);
            self.lines =
                light_and_blocks.edges(min_section, LIGHT_AND_BLOCKS_LINE_COLOR, LINE_WIDTH);
            self.lines
                .extend(light.edges(min_section, LIGHT_ONLY_LINE_COLOR, LINE_WIDTH));
            self.rects = light_and_blocks.faces(min_section, LIGHT_AND_BLOCKS_FACE_COLOR);
            self.rects
                .extend(light.faces(min_section, LIGHT_ONLY_FACE_COLOR));
        }
        (self.rects.clone(), self.lines.clone())
    }

    fn reset(&mut self) {
        self.last_update_at = None;
        self.rects.clear();
        self.lines.clear();
    }
}

fn section_shapes(
    world: &WorldStore,
    min_section: [i32; 3],
) -> (SectionVoxelShape, SectionVoxelShape) {
    // SectionState counts all 26 neighbours, including one cell beyond the
    // rendered 21^3 volume. Sample that padded data volume once, then derive
    // every LIGHT_ONLY cell locally instead of repeating 27 chunk lookups per
    // rendered section.
    let padded_min = min_section.map(|coordinate| coordinate.saturating_sub(1));
    let mut data = SectionVoxelShape::new(SIZE + 2);
    for z in 0..SIZE + 2 {
        for y in 0..SIZE + 2 {
            for x in 0..SIZE + 2 {
                let section = [
                    padded_min[0].saturating_add(x),
                    padded_min[1].saturating_add(y),
                    padded_min[2].saturating_add(z),
                ];
                if world.debug_sky_light_section_has_data(section) {
                    data.fill(x, y, z);
                }
            }
        }
    }

    let mut light_and_blocks = SectionVoxelShape::new(SIZE);
    let mut light = SectionVoxelShape::new(SIZE);
    // `LightSectionDebugRenderer.SectionData`: x advances fastest, then y,
    // then z, matching the nested z/y/x fill loops.
    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let has_blocks = data.is_full(x + 1, y + 1, z + 1);
                let has_light = (-1..=1).any(|offset_z| {
                    (-1..=1).any(|offset_y| {
                        (-1..=1).any(|offset_x| {
                            data.is_full(x + 1 + offset_x, y + 1 + offset_y, z + 1 + offset_z)
                        })
                    })
                });
                if has_blocks {
                    light_and_blocks.fill(x, y, z);
                }
                if has_light {
                    light.fill(x, y, z);
                }
            }
        }
    }
    (light_and_blocks, light)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SectionVoxelShape {
    size: i32,
    full: Vec<bool>,
}

impl SectionVoxelShape {
    fn new(size: i32) -> Self {
        let cell_count = usize::try_from(size)
            .ok()
            .and_then(|size| size.checked_mul(size)?.checked_mul(size))
            .unwrap_or(0);
        Self {
            size,
            full: vec![false; cell_count],
        }
    }

    fn fill(&mut self, x: i32, y: i32, z: i32) {
        if let Some(index) = self.index(x, y, z) {
            self.full[index] = true;
        }
    }

    fn is_full(&self, x: i32, y: i32, z: i32) -> bool {
        self.index(x, y, z)
            .and_then(|index| self.full.get(index))
            .copied()
            .unwrap_or(false)
    }

    fn is_full_wide(&self, transform: AxisCycle, a: i32, b: i32, c: i32) -> bool {
        let [x, y, z] = transform.cycle_coords(a, b, c);
        self.is_full(x, y, z)
    }

    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 || x >= self.size || y >= self.size || z >= self.size {
            return None;
        }
        let size = usize::try_from(self.size).ok()?;
        let x = usize::try_from(x).ok()?;
        let y = usize::try_from(y).ok()?;
        let z = usize::try_from(z).ok()?;
        y.checked_mul(size)?
            .checked_add(x)?
            .checked_mul(size)?
            .checked_add(z)
    }

    fn edges(&self, min_section: [i32; 3], color: [u8; 4], width: f32) -> Vec<DebugLine> {
        let mut lines = Vec::new();
        for transform in [AxisCycle::None, AxisCycle::Forward, AxisCycle::Backward] {
            self.for_all_axis_edges(transform, |from, to| {
                lines.push(DebugLine::new(
                    section_grid_to_world(min_section, from),
                    section_grid_to_world(min_section, to),
                    color,
                    width,
                ));
            });
        }
        lines
    }

    /// `DiscreteVoxelShape.forAllAxisEdges(..., mergeNeighbors=true)`.
    fn for_all_axis_edges(&self, transform: AxisCycle, mut emit: impl FnMut([i32; 3], [i32; 3])) {
        let inverse = transform.inverse();
        for a in 0..=self.size {
            for b in 0..=self.size {
                let mut last_start = None;
                for c in 0..=self.size {
                    let mut full_sectors = 0;
                    let mut odd_sectors = 0;
                    for da in 0..=1 {
                        for db in 0..=1 {
                            if self.is_full_wide(inverse, a + da - 1, b + db - 1, c) {
                                full_sectors += 1;
                                odd_sectors ^= da ^ db;
                            }
                        }
                    }
                    let is_edge = full_sectors == 1
                        || full_sectors == 3
                        || full_sectors == 2 && odd_sectors & 1 == 0;
                    if is_edge {
                        last_start.get_or_insert(c);
                    } else if let Some(start) = last_start.take() {
                        emit(
                            inverse.cycle_coords(a, b, start),
                            inverse.cycle_coords(a, b, c),
                        );
                    }
                }
            }
        }
    }

    fn faces(&self, min_section: [i32; 3], color: [u8; 4]) -> Vec<DebugFilledRect> {
        let mut rects = Vec::new();
        for transform in [AxisCycle::None, AxisCycle::Forward, AxisCycle::Backward] {
            self.for_all_axis_faces(transform, |face, cell| {
                let section =
                    std::array::from_fn(|axis| min_section[axis].saturating_add(cell[axis]));
                let min = section.map(|coordinate| coordinate.saturating_mul(16) as f32);
                let max = min.map(|coordinate| coordinate + 16.0);
                rects.push(DebugFilledRect::cuboid_face(min, max, face, color));
            });
        }
        rects
    }

    /// `DiscreteVoxelShape.forAllAxisFaces`.
    fn for_all_axis_faces(
        &self,
        transform: AxisCycle,
        mut emit: impl FnMut(DebugCuboidFace, [i32; 3]),
    ) {
        let inverse = transform.inverse();
        let axis = inverse.cycle_axis(Axis::Z);
        let (negative, positive) = axis.faces();
        for a in 0..self.size {
            for b in 0..self.size {
                let mut last_full = false;
                for c in 0..=self.size {
                    let full = c != self.size && self.is_full_wide(inverse, a, b, c);
                    if !last_full && full {
                        emit(negative, inverse.cycle_coords(a, b, c));
                    }
                    if last_full && !full {
                        emit(positive, inverse.cycle_coords(a, b, c - 1));
                    }
                    last_full = full;
                }
            }
        }
    }
}

fn section_grid_to_world(min_section: [i32; 3], grid: [i32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        min_section[axis]
            .saturating_add(grid[axis])
            .saturating_mul(16) as f32
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn faces(self) -> (DebugCuboidFace, DebugCuboidFace) {
        match self {
            Self::X => (DebugCuboidFace::West, DebugCuboidFace::East),
            Self::Y => (DebugCuboidFace::Down, DebugCuboidFace::Up),
            Self::Z => (DebugCuboidFace::North, DebugCuboidFace::South),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisCycle {
    None,
    Forward,
    Backward,
}

impl AxisCycle {
    fn inverse(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }

    fn cycle_coords(self, x: i32, y: i32, z: i32) -> [i32; 3] {
        match self {
            Self::None => [x, y, z],
            Self::Forward => [z, x, y],
            Self::Backward => [y, z, x],
        }
    }

    fn cycle_axis(self, axis: Axis) -> Axis {
        match (self, axis) {
            (Self::None, axis) => axis,
            (Self::Forward, Axis::X) => Axis::Y,
            (Self::Forward, Axis::Y) => Axis::Z,
            (Self::Forward, Axis::Z) => Axis::X,
            (Self::Backward, Axis::X) => Axis::Z,
            (Self::Backward, Axis::Y) => Axis::X,
            (Self::Backward, Axis::Z) => Axis::Y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_voxel_emits_twelve_merged_edges_and_six_faces() {
        let mut shape = SectionVoxelShape::new(3);
        shape.fill(1, 1, 1);
        let lines = shape.edges([0; 3], [1, 2, 3, 255], LINE_WIDTH);
        assert_eq!(lines.len(), 12);
        assert!(lines
            .iter()
            .all(|line| line.color == [1, 2, 3, 255] && line.width == LINE_WIDTH));
        let rects = shape.faces([0; 3], [4, 5, 6, 7]);
        assert_eq!(rects.len(), 6);
        assert!(rects.iter().all(|rect| rect.color == [4, 5, 6, 7]));
    }

    #[test]
    fn adjacent_voxels_merge_edges_and_drop_internal_faces() {
        let mut shape = SectionVoxelShape::new(3);
        shape.fill(0, 1, 1);
        shape.fill(1, 1, 1);
        let lines = shape.edges([-1, 0, 0], [255; 4], LINE_WIDTH);
        assert_eq!(lines.len(), 12);
        assert!(lines
            .iter()
            .any(|line| { line.from == [-16.0, 16.0, 16.0] && line.to == [16.0, 16.0, 16.0] }));
        assert_eq!(shape.faces([-1, 0, 0], [255; 4]).len(), 10);
    }
}
