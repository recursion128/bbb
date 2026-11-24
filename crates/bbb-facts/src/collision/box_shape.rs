//! Protocol-independent voxel box-shape boolean operations and face fullness.

use super::{CollisionBounds, CollisionBox, COLLISION_EPSILON};
use crate::block_state_types::VanillaBlockSupportDirection;

const MAX_CELLS: usize = 1 << 20;

#[derive(Clone, Debug)]
pub struct VoxelBoxShape {
    coords: [Vec<f64>; 3],
    filled: Vec<bool>,
}

#[derive(Clone, Copy)]
enum Operation {
    Union,
    OnlyFirst,
}

struct Merger {
    coords: Vec<f64>,
    indices: Vec<(isize, isize)>,
}

impl VoxelBoxShape {
    pub fn empty() -> Self {
        Self {
            coords: std::array::from_fn(|_| vec![0.0]),
            filled: Vec::new(),
        }
    }

    // Shapes.java:60-120: a thin box is empty; unit-grid boxes retain rounded coordinates.
    pub fn from_box<B: CollisionBounds>(bounds: B) -> Option<Self> {
        let min = [bounds.min_x(), bounds.min_y(), bounds.min_z()];
        let max = [bounds.max_x(), bounds.max_y(), bounds.max_z()];
        if min.iter().chain(&max).any(|value| !value.is_finite())
            || (0..3).any(|axis| min[axis] > max[axis])
        {
            return None;
        }
        if (0..3).any(|axis| max[axis] - min[axis] < COLLISION_EPSILON) {
            return Some(Self::empty());
        }
        let bits = std::array::from_fn::<_, 3, _>(|axis| find_bits(min[axis], max[axis]));
        if bits.iter().any(Option::is_none) {
            return Some(Self {
                coords: std::array::from_fn(|axis| vec![min[axis], max[axis]]),
                filled: vec![true],
            });
        }
        let size = bits.map(|bits| 1_usize << bits.unwrap());
        if size == [1; 3] {
            return Some(Self {
                coords: std::array::from_fn(|_| vec![0.0, 1.0]),
                filled: vec![true],
            });
        }
        let coords = std::array::from_fn(|axis| {
            (0..=size[axis])
                .map(|i| i as f64 / size[axis] as f64)
                .collect()
        });
        let mut result = Self {
            coords,
            filled: vec![false; size.iter().product()],
        };
        let first = std::array::from_fn::<_, 3, _>(|axis| {
            (min[axis] * size[axis] as f64 + 0.5).floor() as usize
        });
        let last = std::array::from_fn::<_, 3, _>(|axis| {
            (max[axis] * size[axis] as f64 + 0.5).floor() as usize
        });
        for x in first[0]..last[0] {
            for y in first[1]..last[1] {
                for z in first[2]..last[2] {
                    result.set(x, y, z, true);
                }
            }
        }
        Some(result)
    }

    pub fn from_boxes<B: CollisionBounds>(boxes: &[B]) -> Option<Self> {
        let mut coords: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
        for bounds in boxes {
            let min = [bounds.min_x(), bounds.min_y(), bounds.min_z()];
            let max = [bounds.max_x(), bounds.max_y(), bounds.max_z()];
            if min.iter().chain(&max).any(|value| !value.is_finite())
                || (0..3).any(|axis| min[axis] > max[axis])
            {
                return None;
            }
            if (0..3).any(|axis| min[axis] == max[axis]) {
                continue;
            }
            for axis in 0..3 {
                coords[axis].extend([min[axis], max[axis]]);
            }
        }
        if coords[0].is_empty() {
            return Some(Self::empty());
        }
        for axis in &mut coords {
            axis.sort_by(f64::total_cmp);
            axis.dedup_by(|a, b| a.to_bits() == b.to_bits());
        }
        let sizes = coords.each_ref().map(|axis| axis.len() - 1);
        let cells = sizes
            .iter()
            .try_fold(1_usize, |count, size| count.checked_mul(*size))?;
        if cells > MAX_CELLS {
            return None;
        }
        let mut result = Self {
            coords,
            filled: vec![false; cells],
        };
        for bounds in boxes {
            let min = [bounds.min_x(), bounds.min_y(), bounds.min_z()];
            let max = [bounds.max_x(), bounds.max_y(), bounds.max_z()];
            if (0..3).any(|axis| min[axis] == max[axis]) {
                continue;
            }
            let first = std::array::from_fn::<_, 3, _>(|axis| {
                result.coords[axis]
                    .binary_search_by(|value| value.total_cmp(&min[axis]))
                    .unwrap()
            });
            let last = std::array::from_fn::<_, 3, _>(|axis| {
                result.coords[axis]
                    .binary_search_by(|value| value.total_cmp(&max[axis]))
                    .unwrap()
            });
            for x in first[0]..last[0] {
                for y in first[1]..last[1] {
                    for z in first[2]..last[2] {
                        result.set(x, y, z, true);
                    }
                }
            }
        }
        Some(result)
    }

    pub fn is_empty(&self) -> bool {
        !self.filled.iter().any(|cell| *cell)
    }

    pub fn union(&self, other: &Self) -> Option<Self> {
        self.join(other, Operation::Union)?.optimized()
    }

    pub fn only_first(&self, other: &Self) -> Option<Self> {
        self.join(other, Operation::OnlyFirst)?.optimized()
    }

    // VoxelShape.java:96-99: optimize enumerates merged boxes and rebuilds their union.
    pub fn optimized(&self) -> Option<Self> {
        let mut result = Self::empty();
        for bounds in self.boxes() {
            result = result.join(&Self::from_box(bounds)?, Operation::Union)?;
        }
        Some(result)
    }

    pub fn intersects(&self, other: &Self) -> Option<bool> {
        if self.is_empty() || other.is_empty() {
            return Some(false);
        }
        let merges = std::array::from_fn::<_, 3, _>(|axis| {
            merge_coords(&self.coords[axis], &other.coords[axis], false, false)
        });
        let size = merges
            .each_ref()
            .map(|merge| merge.coords.len().saturating_sub(1));
        let cells = size
            .iter()
            .try_fold(1_usize, |count, size| count.checked_mul(*size))?;
        if cells > MAX_CELLS {
            return None;
        }
        for x in 0..size[0] {
            let (ax, bx) = merges[0].indices[x];
            for y in 0..size[1] {
                let (ay, by) = merges[1].indices[y];
                for z in 0..size[2] {
                    let (az, bz) = merges[2].indices[z];
                    if self.get(ax, ay, az) && other.get(bx, by, bz) {
                        return Some(true);
                    }
                }
            }
        }
        Some(false)
    }

    pub fn moved(&self, offset: [f64; 3]) -> Option<Self> {
        let coords = std::array::from_fn::<_, 3, _>(|axis| {
            self.coords[axis]
                .iter()
                .map(|value| value + offset[axis])
                .collect::<Vec<_>>()
        });
        if coords.iter().flatten().any(|value| !value.is_finite()) {
            return None;
        }
        Some(Self {
            coords,
            filled: self.filled.clone(),
        })
    }

    pub fn bounds(&self) -> Option<CollisionBox> {
        let mut boxes = self.boxes().into_iter();
        let mut bounds = boxes.next()?;
        for other in boxes {
            bounds.min_x = bounds.min_x.min(other.min_x);
            bounds.min_y = bounds.min_y.min(other.min_y);
            bounds.min_z = bounds.min_z.min(other.min_z);
            bounds.max_x = bounds.max_x.max(other.max_x);
            bounds.max_y = bounds.max_y.max(other.max_y);
            bounds.max_z = bounds.max_z.max(other.max_z);
        }
        Some(bounds)
    }

    fn join(&self, other: &Self, operation: Operation) -> Option<Self> {
        if self.is_empty() {
            return Some(if matches!(operation, Operation::Union) {
                other.clone()
            } else {
                Self::empty()
            });
        }
        if other.is_empty() {
            return Some(self.clone());
        }
        let second_only = matches!(operation, Operation::Union);
        let merges = std::array::from_fn::<_, 3, _>(|axis| {
            merge_coords(&self.coords[axis], &other.coords[axis], true, second_only)
        });
        let size = merges
            .each_ref()
            .map(|merge| merge.coords.len().saturating_sub(1));
        let cells = size
            .iter()
            .try_fold(1_usize, |count, size| count.checked_mul(*size))?;
        if cells > MAX_CELLS {
            return None;
        }
        let coords = merges.each_ref().map(|merge| merge.coords.clone());
        let mut result = Self {
            coords,
            filled: vec![false; cells],
        };
        for x in 0..size[0] {
            for y in 0..size[1] {
                for z in 0..size[2] {
                    let [(ax, bx), (ay, by), (az, bz)] = [
                        merges[0].indices[x],
                        merges[1].indices[y],
                        merges[2].indices[z],
                    ];
                    let a = self.get(ax, ay, az);
                    let b = other.get(bx, by, bz);
                    result.set(
                        x,
                        y,
                        z,
                        match operation {
                            Operation::Union => a || b,
                            Operation::OnlyFirst => a && !b,
                        },
                    );
                }
            }
        }
        Some(result)
    }

    fn sizes(&self) -> [usize; 3] {
        self.coords.each_ref().map(|coords| coords.len() - 1)
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let [_, ny, nz] = self.sizes();
        (x * ny + y) * nz + z
    }

    fn get(&self, x: isize, y: isize, z: isize) -> bool {
        let size = self.sizes();
        if x < 0
            || y < 0
            || z < 0
            || x as usize >= size[0]
            || y as usize >= size[1]
            || z as usize >= size[2]
        {
            return false;
        }
        self.filled[self.index(x as usize, y as usize, z as usize)]
    }

    fn set(&mut self, x: usize, y: usize, z: usize, value: bool) {
        let index = self.index(x, y, z);
        self.filled[index] = value;
    }

    // BitSetDiscreteVoxelShape.java:160-202: Z runs expand along X, then Y, in Y/X/Z walk order.
    pub fn boxes(&self) -> Vec<CollisionBox> {
        let [nx, ny, nz] = self.sizes();
        let mut shape = self.clone();
        let mut result = Vec::new();
        for y in 0..ny {
            for x in 0..nx {
                let mut z = 0;
                while z < nz {
                    if !shape.get(x as isize, y as isize, z as isize) {
                        z += 1;
                        continue;
                    }
                    let first_z = z;
                    while z < nz && shape.get(x as isize, y as isize, z as isize) {
                        z += 1;
                    }
                    let mut last_x = x + 1;
                    while last_x < nx
                        && (first_z..z)
                            .all(|cz| shape.get(last_x as isize, y as isize, cz as isize))
                    {
                        last_x += 1;
                    }
                    let mut last_y = y + 1;
                    while last_y < ny
                        && (x..last_x).all(|cx| {
                            (first_z..z)
                                .all(|cz| shape.get(cx as isize, last_y as isize, cz as isize))
                        })
                    {
                        last_y += 1;
                    }
                    for cx in x..last_x {
                        for cy in y..last_y {
                            for cz in first_z..z {
                                shape.set(cx, cy, cz, false);
                            }
                        }
                    }
                    result.push(CollisionBox {
                        min_x: self.coords[0][x],
                        min_y: self.coords[1][y],
                        min_z: self.coords[2][first_z],
                        max_x: self.coords[0][last_x],
                        max_y: self.coords[1][last_y],
                        max_z: self.coords[2][z],
                    });
                }
            }
        }
        result
    }

    // VoxelShape.java:179-195: equal distances retain the first canonical box.
    pub fn closest_point(&self, point: [f64; 3]) -> Option<[f64; 3]> {
        let mut closest = None;
        let mut distance = f64::INFINITY;
        for bounds in self.boxes() {
            let candidate = [
                clamp(point[0], bounds.min_x, bounds.max_x),
                clamp(point[1], bounds.min_y, bounds.max_y),
                clamp(point[2], bounds.min_z, bounds.max_z),
            ];
            let dx = point[0] - candidate[0];
            let dy = point[1] - candidate[1];
            let dz = point[2] - candidate[2];
            let d = dx * dx + dy * dy + dz * dz;
            if closest.is_none() || d < distance {
                closest = Some(candidate);
                distance = d;
            }
        }
        closest
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value.is_nan() {
        value
    } else if value > max || (value == 0.0 && max == 0.0 && max.is_sign_negative()) {
        max
    } else {
        value
    }
}

fn find_bits(min: f64, max: f64) -> Option<u32> {
    if min < -COLLISION_EPSILON || max > 1.0000001 {
        return None;
    }
    (0..=3).find(|bits| {
        let intervals = f64::from(1_u32 << bits);
        [min, max].iter().all(|value| {
            let value = value * intervals;
            (value - (value + 0.5).floor()).abs() < COLLISION_EPSILON * intervals
        })
    })
}

// IndirectMerger.java:15-66: first coordinates win near equality; later indices replace the merged cell.
fn merge_coords(first: &[f64], second: &[f64], first_only: bool, second_only: bool) -> Merger {
    if first[first.len() - 1] < second[0] - COLLISION_EPSILON {
        return non_overlapping(first, second, false);
    }
    if second[second.len() - 1] < first[0] - COLLISION_EPSILON {
        return non_overlapping(second, first, true);
    }
    if first.len() == second.len()
        && first
            .iter()
            .zip(second)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    {
        return Merger {
            coords: first.to_vec(),
            indices: (0..first.len() - 1)
                .map(|i| (i as isize, i as isize))
                .collect(),
        };
    }
    let mut result = Merger {
        coords: Vec::new(),
        indices: Vec::new(),
    };
    let (mut a, mut b) = (0, 0);
    let mut last = f64::NAN;
    while a < first.len() || b < second.len() {
        let exhausted_a = a >= first.len();
        let exhausted_b = b >= second.len();
        let chose_first = !exhausted_a && (exhausted_b || first[a] < second[b] + COLLISION_EPSILON);
        let value = if chose_first {
            let value = first[a];
            a += 1;
            if !first_only && (b == 0 || exhausted_b) {
                continue;
            }
            value
        } else {
            let value = second[b];
            b += 1;
            if !second_only && (a == 0 || exhausted_a) {
                continue;
            }
            value
        };
        let indices = (a as isize - 1, b as isize - 1);
        if matches!(
            last.partial_cmp(&(value - COLLISION_EPSILON)),
            None | Some(std::cmp::Ordering::Less)
        ) {
            result.coords.push(value);
            result.indices.push(indices);
            last = value;
        } else if let Some(previous) = result.indices.last_mut() {
            *previous = indices;
        }
    }
    if result.coords.is_empty() {
        result.coords.push(0.0);
        result.indices.push((-1, -1));
    }
    result
}

fn non_overlapping(lower: &[f64], upper: &[f64], swap: bool) -> Merger {
    let mut coords = lower.to_vec();
    coords.extend_from_slice(upper);
    let indices = (0..lower.len())
        .map(|i| (i as isize, -1))
        .chain((0..upper.len() - 1).map(|i| (lower.len() as isize - 1, i as isize)))
        .map(|(a, b)| if swap { (b, a) } else { (a, b) })
        .collect();
    Merger { coords, indices }
}

pub fn collision_face_full<B: CollisionBounds>(
    boxes: &[B],
    face: VanillaBlockSupportDirection,
) -> Option<bool> {
    if boxes.is_empty() {
        return Some(false);
    }
    let (axis, coordinate) = match face {
        VanillaBlockSupportDirection::Down => (1, COLLISION_EPSILON),
        VanillaBlockSupportDirection::Up => (1, 1.0 - COLLISION_EPSILON),
        VanillaBlockSupportDirection::North => (2, COLLISION_EPSILON),
        VanillaBlockSupportDirection::South => (2, 1.0 - COLLISION_EPSILON),
        VanillaBlockSupportDirection::West => (0, COLLISION_EPSILON),
        VanillaBlockSupportDirection::East => (0, 1.0 - COLLISION_EPSILON),
    };
    if let [single] = boxes {
        let min = [single.min_x(), single.min_y(), single.min_z()];
        let max = [single.max_x(), single.max_y(), single.max_z()];
        if !(min[0].is_finite()
            && min[1].is_finite()
            && min[2].is_finite()
            && max[0].is_finite()
            && max[1].is_finite()
            && max[2].is_finite())
        {
            return None;
        }
        if min[axis] > coordinate || max[axis] <= coordinate {
            return Some(false);
        }
        let (a1, a2) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        if min[a1] <= max[a1] && min[a2] <= max[a2] {
            return Some(
                min[a1].abs() <= COLLISION_EPSILON
                    && (max[a1] - 1.0).abs() <= COLLISION_EPSILON
                    && min[a2].abs() <= COLLISION_EPSILON
                    && (max[a2] - 1.0).abs() <= COLLISION_EPSILON,
            );
        }
    }
    // VoxelShape.java:216-229: project the union of the face's slice, including multi-box faces.
    let projected: Vec<_> = boxes
        .iter()
        .filter_map(|bounds| {
            let mut min = [bounds.min_x(), bounds.min_y(), bounds.min_z()];
            let mut max = [bounds.max_x(), bounds.max_y(), bounds.max_z()];
            if min[axis] > coordinate || max[axis] <= coordinate {
                return None;
            }
            min[axis] = 0.0;
            max[axis] = 1.0;
            Some(CollisionBox::from_arrays(min, max))
        })
        .collect();
    if projected.is_empty() {
        return Some(false);
    }
    let unit = VoxelBoxShape::from_box(CollisionBox::UNIT_CUBE)?;
    let face = VoxelBoxShape::from_boxes(&projected)?;
    Some(unit.only_first(&face)?.is_empty() && face.only_first(&unit)?.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(min_x: f64, max_x: f64) -> CollisionBox {
        CollisionBox {
            min_x,
            max_x,
            min_y: 10.0,
            max_y: 12.0,
            min_z: 10.0,
            max_z: 12.0,
        }
    }

    fn shape(bounds: CollisionBox) -> VoxelBoxShape {
        VoxelBoxShape::from_box(bounds).unwrap()
    }

    #[test]
    fn box_shape_difference_closest_keeps_the_first_equal_distance_box() {
        let allowed = shape(bounds(8.0, 12.0));
        let obstacle = shape(bounds(9.5, 10.5));
        let free = allowed.only_first(&obstacle).unwrap();
        assert_eq!(free.boxes(), vec![bounds(8.0, 9.5), bounds(10.5, 12.0)]);
        assert_eq!(
            free.closest_point([10.0, 11.0, 11.0]),
            Some([9.5, 11.0, 11.0])
        );
        assert!(allowed.only_first(&allowed).unwrap().is_empty());
    }

    #[test]
    fn box_shape_coordinate_merge_closes_only_sub_epsilon_gaps() {
        for (gap, open) in [(0.5e-7, false), (2.0e-7, true)] {
            let allowed = shape(bounds(10.0, 12.0));
            let obstacles = shape(bounds(10.0, 11.0))
                .union(&shape(bounds(11.0 + gap, 12.0)))
                .unwrap();
            let free = allowed.only_first(&obstacles).unwrap();
            let point = [11.0 + gap / 2.0, 11.0, 11.0];
            assert_eq!(free.closest_point(point), open.then_some(point));
        }
    }

    #[test]
    fn box_shape_unit_grid_rounding_and_thin_box_follow_create() {
        let unit = CollisionBox::UNIT_CUBE;
        let mut near = unit;
        near.min_x = 0.5e-7;
        near.max_y = 1.0 - 0.5e-7;
        assert_eq!(shape(near).boxes(), vec![unit]);
        near.max_x = near.min_x + 0.5e-7;
        assert!(shape(near).is_empty());
        near.max_x = f64::INFINITY;
        assert!(VoxelBoxShape::from_box(near).is_none());
    }

    #[test]
    fn box_shape_canonical_merge_expands_z_then_x_then_y() {
        let a = shape(CollisionBox {
            min_x: 10.0,
            min_y: 10.0,
            min_z: 10.0,
            max_x: 11.0,
            max_y: 12.0,
            max_z: 12.0,
        });
        let b = shape(CollisionBox {
            min_x: 11.0,
            min_y: 10.0,
            min_z: 10.0,
            max_x: 12.0,
            max_y: 11.0,
            max_z: 11.0,
        });
        assert_eq!(
            a.union(&b).unwrap().boxes(),
            vec![
                CollisionBox {
                    min_x: 10.0,
                    min_y: 10.0,
                    min_z: 10.0,
                    max_x: 11.0,
                    max_y: 12.0,
                    max_z: 12.0,
                },
                CollisionBox {
                    min_x: 11.0,
                    min_y: 10.0,
                    min_z: 10.0,
                    max_x: 12.0,
                    max_y: 11.0,
                    max_z: 11.0,
                },
            ]
        );
    }

    #[test]
    fn box_shape_union_and_difference_keep_holes_and_empty_identities() {
        let a = shape(bounds(10.0, 11.0));
        let b = shape(bounds(12.0, 13.0));
        let union = a.union(&b).unwrap();
        assert_eq!(union.boxes(), vec![bounds(10.0, 11.0), bounds(12.0, 13.0)]);
        assert_eq!(union.only_first(&a).unwrap().boxes(), b.boxes());
        assert_eq!(a.union(&VoxelBoxShape::empty()).unwrap().boxes(), a.boxes());
        assert_eq!(
            a.only_first(&VoxelBoxShape::empty()).unwrap().boxes(),
            a.boxes()
        );
        assert_eq!(VoxelBoxShape::empty().closest_point([0.0, 0.0, 0.0]), None);
    }

    #[test]
    fn box_shape_grid_expansion_refuses_before_allocating_the_product() {
        let a = VoxelBoxShape {
            coords: std::array::from_fn(|_| (0..=100).map(|i| f64::from(i) * 2.0).collect()),
            filled: vec![true; 1_000_000],
        };
        let b = VoxelBoxShape {
            coords: std::array::from_fn(|_| (0..=100).map(|i| f64::from(i) * 2.0 + 1.0).collect()),
            filled: vec![true; 1_000_000],
        };
        assert!(a.union(&b).is_none());
    }

    #[test]
    fn box_shape_identical_and_disjoint_mergers_preserve_epsilon_thick_slabs() {
        let slab = shape(bounds(0.0, COLLISION_EPSILON));
        let other = shape(bounds(2.0, 3.0));
        assert_eq!(slab.union(&slab).unwrap().boxes(), slab.boxes());
        assert_eq!(
            slab.union(&other).unwrap().boxes(),
            vec![bounds(0.0, COLLISION_EPSILON), bounds(2.0, 3.0)]
        );
        assert_eq!(
            other.union(&slab).unwrap().boxes(),
            slab.union(&other).unwrap().boxes()
        );
        assert_eq!(slab.only_first(&other).unwrap().boxes(), slab.boxes());
        assert!(slab.only_first(&slab).unwrap().is_empty());
    }

    #[test]
    fn box_shape_unit_grid_fast_path_and_ieee_clamp_keep_native_values() {
        let unit = CollisionBox::UNIT_CUBE;
        let thin = CollisionBox {
            min_x: -0.5e-7,
            max_x: 0.5e-7,
            ..unit
        };
        assert_eq!(shape(thin).boxes(), vec![unit]);
        let negative_zero = shape(bounds(-1.0, -0.0));
        let point = negative_zero.closest_point([0.0, 11.0, 11.0]).unwrap();
        assert_eq!(point[0].to_bits(), (-0.0_f64).to_bits());
        assert!(negative_zero.closest_point([f64::NAN, 11.0, 11.0]).unwrap()[0].is_nan());
    }

    #[test]
    fn box_shape_intersection_mergers_keep_native_epsilon_overlap() {
        let a = shape(bounds(10.0, 11.0));
        for (start, hit) in [
            (11.0, false),
            (11.0 - 0.5e-7, false),
            (11.0 - 2.0e-7, true),
            (12.0, false),
        ] {
            let b = shape(bounds(start, 12.5));
            assert_eq!(a.intersects(&b), Some(hit));
            assert_eq!(b.intersects(&a), Some(hit));
        }
        let slab = shape(bounds(0.0, COLLISION_EPSILON));
        assert_eq!(slab.intersects(&slab), Some(true));
        assert_eq!(a.intersects(&VoxelBoxShape::empty()), Some(false));
        assert_eq!(VoxelBoxShape::empty().intersects(&a), Some(false));
    }

    #[test]
    fn box_shape_movement_retains_local_grid_without_recreating_world_coordinates() {
        let a = shape(CollisionBox {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 0.5,
            max_y: 0.5,
            max_z: 0.5,
        });
        let offset = [0.5e-7, 10.0, -10.0];
        assert_eq!(
            a.moved(offset).unwrap().bounds(),
            Some(a.bounds().unwrap().moved(offset))
        );
        assert!(a.moved([f64::INFINITY, offset[1], offset[2]]).is_none());
        assert_eq!(VoxelBoxShape::empty().bounds(), None);
    }
}
