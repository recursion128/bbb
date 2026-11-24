//! Shared fluid height, flow, entity contact/pushing, and boat water queries.

use crate::collision::{vec3_length_sqr, vec3_normalize, BlockFace, CollisionBounds};

/// `Entity.getFluidInteractionBox` deflation (`Entity.java:4000-4001`).
pub const FLUID_BOX_MARGIN: f64 = 0.001;

/// `EntityFluidInteraction.update` shallow-current taper threshold (`EntityFluidInteraction.java:75`).
pub const FLUID_CURRENT_TAPER_HEIGHT: f64 = 0.4;

/// `FlowingFluid.getFlow` downward push added to falling fluids against a solid face (`FlowingFluid.java`).
pub const FLUID_DOWNWARD_FLOW_WEIGHT: f64 = 6.0;

/// `FlowingFluid.getFlow` falling-neighbour drop offset (`FlowingFluid.java`), `f32` precision.
pub const FLUID_FALLING_DROP_BIAS_F32: f32 = 8.0 / 9.0;

/// `EntityFluidInteraction.Tracker.applyCurrentTo` minimum accumulated current^2 (`EntityFluidInteraction.java:172`).
pub const FLUID_CURRENT_LENGTH_SQR_THRESHOLD: f64 = 1.0e-5_f32 as f64;

/// `Entity.updateFluidInteraction` water current push scale (`Entity.java:1568`).
pub const WATER_CURRENT_SCALE: f64 = 0.014;

/// `Entity.updateFluidInteraction` fast-lava (nether) push scale (`Entity.java:1572`).
pub const FAST_LAVA_CURRENT_SCALE: f64 = 0.007;

/// `Entity.updateFluidInteraction` overworld lava push scale (`Entity.java:1572`).
pub const LAVA_CURRENT_SCALE: f64 = 0.002_333_333_333_333_333_5;

/// `EntityFluidInteraction.Tracker.applyCurrentTo` resting horizontal threshold (`EntityFluidInteraction.java:182`).
pub const RESTING_CURRENT_DELTA: f64 = 0.003;

/// `EntityFluidInteraction.Tracker.applyCurrentTo` minimum resting impulse (`EntityFluidInteraction.java:183`).
pub const MIN_CURRENT_IMPULSE: f64 = RESTING_CURRENT_DELTA * 1.5;

/// Broad fluid tag family used by shared fluid physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidKind {
    Water,
    Lava,
}

/// Protocol-independent description of a non-empty fluid block cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidSample {
    pub kind: FluidKind,
    pub amount: u8,
    pub falling: bool,
    pub is_source: bool,
}

/// Neighbour cell summary passed into [`fluid_flow_vector`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidFlowCell {
    pub fluid: Option<FluidSample>,
    pub blocks_motion: bool,
}

/// `FlowingFluid.getOwnHeight` (`FlowingFluid.java`) as Java `float`.
#[inline]
pub fn fluid_own_height_f32(amount: u8) -> f32 {
    if amount == 0 {
        0.0
    } else {
        f32::from(amount) / 9.0
    }
}

/// `FlowingFluid.getOwnHeight` (`FlowingFluid.java`) in `f64` for client block/player surfaces.
#[inline]
pub fn fluid_own_height_f64(amount: u8) -> f64 {
    if amount == 0 {
        0.0
    } else {
        f64::from(amount) / 9.0
    }
}

/// `FlowingFluid.getHeight` (`FlowingFluid.java`) / `EmptyFluid` (`0.0F`) as Java `float`.
#[inline]
pub fn fluid_height_f32(fluid: Option<FluidSample>, same_kind_above: bool) -> f32 {
    let Some(sample) = fluid else {
        return 0.0;
    };
    if same_kind_above {
        1.0
    } else {
        fluid_own_height_f32(sample.amount)
    }
}

/// `FlowingFluid.getHeight` (`FlowingFluid.java`) in `f64`.
#[inline]
pub fn fluid_height_f64(fluid: Option<FluidSample>, same_kind_above: bool) -> f64 {
    let Some(sample) = fluid else {
        return 0.0;
    };
    if same_kind_above {
        1.0
    } else {
        fluid_own_height_f64(sample.amount)
    }
}

/// Checked `FlowingFluid.getHeight` returning `None` when the cell above is unknown.
#[inline]
pub fn checked_fluid_height_f32(
    fluid: Option<FluidSample>,
    above_kind: Option<Option<FluidKind>>,
) -> Option<f32> {
    let Some(sample) = fluid else {
        return Some(0.0);
    };
    let same_kind_above = above_kind? == Some(sample.kind);
    Some(if same_kind_above {
        1.0
    } else {
        fluid_own_height_f32(sample.amount)
    })
}

fn affects_flow(kind: FluidKind, other: Option<FluidSample>) -> bool {
    other.is_none_or(|sample| sample.kind == kind)
}

fn same_kind_own_height(kind: FluidKind, other: Option<FluidSample>) -> f32 {
    other
        .filter(|sample| sample.kind == kind)
        .map_or(0.0, |sample| fluid_own_height_f32(sample.amount))
}

/// `FlowingFluid.getFlow` (`FlowingFluid.java`).
///
/// `cell` resolves `[x, y, z]` (`None` fails closed for unknown block states) and
/// `is_solid_face` resolves `FlowingFluid.isSolidFace(level, neighborPos, direction)`.
pub fn fluid_flow_vector(
    pos: [i32; 3],
    fluid: FluidSample,
    mut cell: impl FnMut([i32; 3]) -> Option<FluidFlowCell>,
    mut is_solid_face: impl FnMut([i32; 3], BlockFace) -> Option<bool>,
) -> Option<[f64; 3]> {
    let self_height = fluid_own_height_f32(fluid.amount);
    let mut flow_x = 0.0_f64;
    let mut flow_z = 0.0_f64;
    for face in BlockFace::HORIZONTAL {
        let step = face.step();
        let neighbor_pos = [pos[0] + step[0], pos[1], pos[2] + step[2]];
        let neighbor = cell(neighbor_pos)?;
        if !affects_flow(fluid.kind, neighbor.fluid) {
            continue;
        }
        let mut neighbor_height = same_kind_own_height(fluid.kind, neighbor.fluid);
        let mut distance = 0.0_f32;
        if neighbor_height == 0.0 {
            if !neighbor.blocks_motion {
                let under = cell([neighbor_pos[0], neighbor_pos[1] - 1, neighbor_pos[2]])?;
                if affects_flow(fluid.kind, under.fluid) {
                    neighbor_height = same_kind_own_height(fluid.kind, under.fluid);
                    if neighbor_height > 0.0 {
                        distance = self_height - (neighbor_height - FLUID_FALLING_DROP_BIAS_F32);
                    }
                }
            }
        } else if neighbor_height > 0.0 {
            distance = self_height - neighbor_height;
        }
        if distance != 0.0 {
            flow_x += f64::from(step[0] as f32 * distance);
            flow_z += f64::from(step[2] as f32 * distance);
        }
    }

    let mut current = [flow_x, 0.0, flow_z];
    if fluid.falling {
        for face in BlockFace::HORIZONTAL {
            let step = face.step();
            let neighbor_pos = [pos[0] + step[0], pos[1], pos[2] + step[2]];
            let above_neighbor_pos = [neighbor_pos[0], neighbor_pos[1] + 1, neighbor_pos[2]];
            if is_solid_face(neighbor_pos, face)? || is_solid_face(above_neighbor_pos, face)? {
                current = vec3_normalize(current);
                current[1] -= FLUID_DOWNWARD_FLOW_WEIGHT;
                break;
            }
        }
    }
    Some(vec3_normalize(current))
}

/// Per-fluid accumulator mirroring `EntityFluidInteraction.Tracker`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FluidTrackerState {
    pub height: f64,
    pub eyes_inside: bool,
    pub accumulated_current: [f64; 3],
    pub current_count: u32,
}

impl FluidTrackerState {
    #[inline]
    pub fn is_in_fluid(self) -> bool {
        self.height > 0.0
    }

    #[inline]
    pub fn accumulate_current(&mut self, flow: [f64; 3]) {
        self.accumulated_current[0] += flow[0];
        self.accumulated_current[1] += flow[1];
        self.accumulated_current[2] += flow[2];
        self.current_count += 1;
    }

    /// `EntityFluidInteraction.Tracker.applyCurrentTo` (`EntityFluidInteraction.java:171-189`).
    pub fn apply_current(
        self,
        movement: [f64; 3],
        scale: f64,
        is_player: bool,
        step_ticks: f64,
    ) -> [f64; 3] {
        if self.current_count == 0
            || vec3_length_sqr(self.accumulated_current) < FLUID_CURRENT_LENGTH_SQR_THRESHOLD
        {
            return movement;
        }
        let direction = if is_player {
            let inv = 1.0 / f64::from(self.current_count);
            [
                self.accumulated_current[0] * inv,
                self.accumulated_current[1] * inv,
                self.accumulated_current[2] * inv,
            ]
        } else {
            vec3_normalize(self.accumulated_current)
        };
        let push = scale * step_ticks;
        let mut impulse = [
            direction[0] * push,
            direction[1] * push,
            direction[2] * push,
        ];
        let resting_delta = RESTING_CURRENT_DELTA * step_ticks;
        let min_impulse = MIN_CURRENT_IMPULSE * step_ticks;
        if movement[0].abs() < resting_delta
            && movement[2].abs() < resting_delta
            && vec3_length_sqr(impulse).sqrt() < min_impulse
        {
            let normalized = vec3_normalize(impulse);
            impulse = [
                normalized[0] * min_impulse,
                normalized[1] * min_impulse,
                normalized[2] * min_impulse,
            ];
        }
        [
            movement[0] + impulse[0],
            movement[1] + impulse[1],
            movement[2] + impulse[2],
        ]
    }
}

/// Combined water and lava trackers from `EntityFluidInteraction.update`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FluidContactScan {
    pub water: FluidTrackerState,
    pub lava: FluidTrackerState,
}

impl FluidContactScan {
    #[inline]
    pub const fn tracker(&self, kind: FluidKind) -> &FluidTrackerState {
        match kind {
            FluidKind::Water => &self.water,
            FluidKind::Lava => &self.lava,
        }
    }

    #[inline]
    pub const fn tracker_mut(&mut self, kind: FluidKind) -> &mut FluidTrackerState {
        match kind {
            FluidKind::Water => &mut self.water,
            FluidKind::Lava => &mut self.lava,
        }
    }
}

/// `EntityFluidInteraction.update` box scan (`EntityFluidInteraction.java:36-86`).
///
/// `bounds` is the already-deflated fluid interaction box. `fluid_at` returns `None` to
/// fail closed on unknown states, `Some(None)` for empty/non-fluid cells, or
/// `Some(Some((kind, fluid_height)))`. `flow_at` is queried only for overlapping cells
/// when `!ignore_current`.
pub fn scan_fluid_contact<M: CollisionBounds>(
    bounds: M,
    entity_y: f64,
    eye: [f64; 3],
    ignore_current: bool,
    mut fluid_at: impl FnMut([i32; 3]) -> Option<Option<(FluidKind, f64)>>,
    mut flow_at: impl FnMut([i32; 3], FluidKind) -> Option<[f64; 3]>,
) -> Option<FluidContactScan> {
    let mut scan = FluidContactScan::default();
    if !(bounds.min_x().is_finite()
        && bounds.min_y().is_finite()
        && bounds.min_z().is_finite()
        && bounds.max_x().is_finite()
        && bounds.max_y().is_finite()
        && bounds.max_z().is_finite())
    {
        return Some(scan);
    }
    let x0 = bounds.min_x().floor() as i32;
    let y0 = bounds.min_y().floor() as i32;
    let z0 = bounds.min_z().floor() as i32;
    let x1 = (bounds.max_x().ceil() as i32).saturating_sub(1);
    let y1 = (bounds.max_y().ceil() as i32).saturating_sub(1);
    let z1 = (bounds.max_z().ceil() as i32).saturating_sub(1);
    if fluid_cell_volume(x0, x1, y0, y1, z0, z1) > MAX_FLUID_SCAN_CELLS {
        return Some(scan);
    }
    let eye_x = eye[0].floor() as i32;
    let eye_y = eye[1];
    let eye_z = eye[2].floor() as i32;

    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let pos = [x, y, z];
                let Some((kind, fluid_height)) = fluid_at(pos)? else {
                    continue;
                };
                let fluid_bottom = f64::from(y);
                let fluid_top = fluid_bottom + fluid_height;
                if fluid_top < bounds.min_y() {
                    continue;
                }
                let tracker = scan.tracker_mut(kind);
                if x == eye_x && z == eye_z && eye_y >= fluid_bottom && eye_y <= fluid_top {
                    tracker.eyes_inside = true;
                }
                tracker.height = (fluid_top - entity_y).max(tracker.height);
                if !ignore_current {
                    let mut current = flow_at(pos, kind)?;
                    if tracker.height < FLUID_CURRENT_TAPER_HEIGHT {
                        current = [
                            current[0] * tracker.height,
                            current[1] * tracker.height,
                            current[2] * tracker.height,
                        ];
                    }
                    tracker.accumulate_current(current);
                }
            }
        }
    }
    Some(scan)
}

/// Hard upper bound on voxel cells scanned by a single fluid AABB query to prevent
/// pathological oversized entity bounds from freezing the simulation or render loop.
pub const MAX_FLUID_SCAN_CELLS: u64 = 262_144;

#[inline]
fn fluid_cell_volume(
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
) -> u64 {
    if min_x > max_x || min_y > max_y || min_z > max_z {
        return 0;
    }
    let dx = (i64::from(max_x) - i64::from(min_x) + 1) as u64;
    let dy = (i64::from(max_y) - i64::from(min_y) + 1) as u64;
    let dz = (i64::from(max_z) - i64::from(min_z) + 1) as u64;
    dx.saturating_mul(dy).saturating_mul(dz)
}

/// Tests whether any matching fluid column overlaps `bounds` deflated by [`FLUID_BOX_MARGIN`].
///
/// `fluid_height_at` returns `None` when a block state is unknown, `Some(None)` when absent or
/// non-matching, or `Some(Some(height))`. Returns `Some(true)` on the first overlap, `None`
/// if no overlap was found and at least one cell was unknown, or `Some(false)` otherwise.
pub fn aabb_in_fluid<M: CollisionBounds>(
    bounds: M,
    fluid_height_at: impl FnMut([i32; 3]) -> Option<Option<f64>>,
) -> Option<bool> {
    aabb_in_fluid_clamped(bounds, None, fluid_height_at)
}

/// Same as [`aabb_in_fluid`], optionally intersecting the scanned integer cell range with
/// `[clamp_min, clamp_max]` (for example dimension height and loaded chunk bounds).
pub fn aabb_in_fluid_clamped<M: CollisionBounds>(
    bounds: M,
    clamp: Option<([i32; 3], [i32; 3])>,
    mut fluid_height_at: impl FnMut([i32; 3]) -> Option<Option<f64>>,
) -> Option<bool> {
    let deflated = bounds.to_collision_box().deflated(FLUID_BOX_MARGIN);
    if !(deflated.min_x.is_finite()
        && deflated.min_y.is_finite()
        && deflated.min_z.is_finite()
        && deflated.max_x.is_finite()
        && deflated.max_y.is_finite()
        && deflated.max_z.is_finite())
    {
        return Some(false);
    }
    let mut min_x = deflated.min_x.floor() as i32;
    let mut min_y = deflated.min_y.floor() as i32;
    let mut min_z = deflated.min_z.floor() as i32;
    let mut max_x = (deflated.max_x.ceil() as i32).saturating_sub(1);
    let mut max_y = (deflated.max_y.ceil() as i32).saturating_sub(1);
    let mut max_z = (deflated.max_z.ceil() as i32).saturating_sub(1);
    if let Some((cmin, cmax)) = clamp {
        min_x = min_x.max(cmin[0]);
        min_y = min_y.max(cmin[1]);
        min_z = min_z.max(cmin[2]);
        max_x = max_x.min(cmax[0]);
        max_y = max_y.min(cmax[1]);
        max_z = max_z.min(cmax[2]);
    }
    let volume = fluid_cell_volume(min_x, max_x, min_y, max_y, min_z, max_z);
    if volume == 0 || volume > MAX_FLUID_SCAN_CELLS {
        return Some(false);
    }
    let mut unknown = false;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                match fluid_height_at([x, y, z]) {
                    None => unknown = true,
                    Some(None) => {}
                    Some(Some(height)) => {
                        if f64::from(y) + height >= deflated.min_y {
                            return Some(true);
                        }
                    }
                }
            }
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}

/// Water-derived half of `AbstractBoat.Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatWaterStatus {
    InWater,
    UnderWater,
    UnderFlowingWater,
}

/// `AbstractBoat.isUnderwater` (`AbstractBoat.java:493-511`).
///
/// `water_at` returns `Some((height_f32, is_source))` when `[x, y, z]` holds water.
pub fn boat_is_underwater<M: CollisionBounds>(
    bounds: M,
    mut water_at: impl FnMut([i32; 3]) -> Option<(f32, bool)>,
) -> Option<BoatWaterStatus> {
    let max_y = bounds.max_y() + FLUID_BOX_MARGIN;
    let x0 = bounds.min_x().floor() as i32;
    let x1 = bounds.max_x().ceil() as i32;
    let y0 = bounds.max_y().floor() as i32;
    let y1 = max_y.ceil() as i32;
    let z0 = bounds.min_z().floor() as i32;
    let z1 = bounds.max_z().ceil() as i32;
    let mut under_water = false;
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                let Some((height, is_source)) = water_at([x, y, z]) else {
                    continue;
                };
                if max_y < f64::from(y as f32 + height) {
                    if !is_source {
                        return Some(BoatWaterStatus::UnderFlowingWater);
                    }
                    under_water = true;
                }
            }
        }
    }
    under_water.then_some(BoatWaterStatus::UnderWater)
}

/// `AbstractBoat.checkInWater` (`AbstractBoat.java:464-483`), returning `(in_water, water_level)`.
pub fn boat_check_in_water<M: CollisionBounds>(
    bounds: M,
    mut water_height_at: impl FnMut([i32; 3]) -> Option<f32>,
) -> (bool, f64) {
    let min_x = bounds.min_x().floor() as i32;
    let max_x = bounds.max_x().ceil() as i32;
    let min_y = bounds.min_y().floor() as i32;
    let max_y = (bounds.min_y() + FLUID_BOX_MARGIN).ceil() as i32;
    let min_z = bounds.min_z().floor() as i32;
    let max_z = bounds.max_z().ceil() as i32;
    let mut in_water = false;
    let mut water_level = -f64::MAX;
    for x in min_x..max_x {
        for y in min_y..max_y {
            for z in min_z..max_z {
                let Some(height) = water_height_at([x, y, z]) else {
                    continue;
                };
                let top = f64::from(y as f32 + height);
                water_level = water_level.max(top);
                in_water |= bounds.min_y() < top;
            }
        }
    }
    (in_water, water_level)
}

/// Combined water status and `water_level` from `AbstractBoat.isUnderwater` + `checkInWater`.
pub fn boat_water_status<M: CollisionBounds>(
    bounds: M,
    mut water_at: impl FnMut([i32; 3]) -> Option<(f32, bool)>,
) -> (Option<BoatWaterStatus>, f64) {
    if let Some(under) = boat_is_underwater(bounds, &mut water_at) {
        return (Some(under), bounds.max_y());
    }
    let (in_water, water_level) =
        boat_check_in_water(bounds, |pos| water_at(pos).map(|(height, _)| height));
    (in_water.then_some(BoatWaterStatus::InWater), water_level)
}

/// `AbstractBoat.getWaterLevelAbove` (`AbstractBoat.java:392-414`).
pub fn boat_water_level_above<M: CollisionBounds>(
    bounds: M,
    last_yd: f64,
    mut water_height_at: impl FnMut([i32; 3]) -> Option<f32>,
) -> f32 {
    let min_x = bounds.min_x().floor() as i32;
    let max_x = bounds.max_x().ceil() as i32;
    let min_y = bounds.max_y().floor() as i32;
    let max_y = (bounds.max_y() - last_yd).ceil() as i32;
    let min_z = bounds.min_z().floor() as i32;
    let max_z = bounds.max_z().ceil() as i32;
    'rows: for y in min_y..max_y {
        let mut block_height = 0.0_f32;
        for x in min_x..max_x {
            for z in min_z..max_z {
                if let Some(height) = water_height_at([x, y, z]) {
                    block_height = block_height.max(height);
                }
                if block_height >= 1.0 {
                    continue 'rows;
                }
            }
        }
        if block_height < 1.0 {
            return y as f32 + block_height;
        }
    }
    max_y as f32 + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::CollisionBox;

    #[test]
    fn fluid_height_returns_full_when_same_fluid_is_above() {
        let flowing = Some(FluidSample {
            kind: FluidKind::Water,
            amount: 5,
            falling: false,
            is_source: false,
        });
        assert_eq!(fluid_height_f32(flowing, false), 5.0_f32 / 9.0_f32);
        assert_eq!(fluid_height_f32(flowing, true), 1.0);
        assert_eq!(fluid_height_f64(flowing, false), 5.0 / 9.0);
        assert_eq!(fluid_height_f64(flowing, true), 1.0);
        assert_eq!(checked_fluid_height_f32(None, None), Some(0.0));
        assert_eq!(checked_fluid_height_f32(flowing, None), None);
        assert_eq!(
            checked_fluid_height_f32(flowing, Some(Some(FluidKind::Water))),
            Some(1.0)
        );
    }

    #[test]
    fn fluid_flow_vector_handles_horizontal_drop_and_falling_wall() {
        let source = FluidSample {
            kind: FluidKind::Water,
            amount: 8,
            falling: false,
            is_source: true,
        };
        let lower = FluidSample {
            kind: FluidKind::Water,
            amount: 5,
            falling: false,
            is_source: false,
        };
        let flow = fluid_flow_vector(
            [0, 0, 0],
            source,
            |pos| {
                Some(FluidFlowCell {
                    fluid: (pos == [0, 0, 1]).then_some(lower),
                    blocks_motion: true,
                })
            },
            |_, _| Some(false),
        )
        .expect("complete fluid query");
        assert!((flow[2] - 1.0).abs() < 1e-12);
        assert_eq!(flow[0], 0.0);
        assert_eq!(flow[1], 0.0);

        let falling = FluidSample {
            falling: true,
            is_source: false,
            ..source
        };
        let downward = fluid_flow_vector(
            [0, 0, 0],
            falling,
            |_| {
                Some(FluidFlowCell {
                    fluid: None,
                    blocks_motion: true,
                })
            },
            |pos, _| Some(pos == [1, 0, 0]),
        )
        .expect("falling query");
        assert!((downward[1] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn apply_current_enforces_resting_minimum_impulse() {
        let mut tracker = FluidTrackerState::default();
        tracker.accumulate_current([1.0, 0.0, 0.0]);
        let next = tracker.apply_current([0.0; 3], 0.002, false, 1.0);
        assert_eq!(next[0], MIN_CURRENT_IMPULSE);
        assert_eq!(next[1], 0.0);
        assert_eq!(next[2], 0.0);
    }

    #[test]
    fn boat_water_status_matches_underwater_flowing_surface_and_level_above() {
        let box_under = CollisionBox::from_arrays([0.25, 1.0, 0.25], [0.75, 1.8, 0.75]);
        assert_eq!(
            boat_water_status(box_under, |pos| {
                (pos == [0, 1, 0]).then_some((8.0_f32 / 9.0_f32, false))
            }),
            (Some(BoatWaterStatus::UnderFlowingWater), 1.8)
        );
        let box_surface = CollisionBox::from_arrays([0.25, 0.8, 0.25], [0.75, 1.6, 0.75]);
        assert_eq!(
            boat_water_status(box_surface, |pos| {
                (pos == [0, 0, 0]).then_some((8.0_f32 / 9.0_f32, true))
            }),
            (Some(BoatWaterStatus::InWater), f64::from(8.0_f32 / 9.0_f32))
        );
        let level_above = boat_water_level_above(box_under, -1.0, |pos| {
            if pos == [0, 1, 0] {
                Some(1.0)
            } else if pos == [0, 2, 0] {
                Some(5.0_f32 / 9.0_f32)
            } else {
                None
            }
        });
        assert_eq!(level_above, 2.0_f32 + 5.0_f32 / 9.0_f32);
    }
}
