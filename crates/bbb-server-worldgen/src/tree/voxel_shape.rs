//! Discrete tree voxel data and exposed boundary faces.

use bbb_protocol::packets::{BlockPos, Direction};

/// `BoundingBox` (`BoundingBox.java:48-54`): inclusive block bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub max_z: i32,
}

impl BoundingBox {
    /// `BoundingBox.encapsulatingPositions` (`BoundingBox.java:127-136`): the
    /// box of the first position (`BoundingBox.java:44-46`) grown by
    /// `encapsulate` over the rest (`BoundingBox.java:173-181`).
    pub fn encapsulating_positions(mut positions: impl Iterator<Item = BlockPos>) -> Option<Self> {
        let first = positions.next()?;
        let mut result = Self {
            min_x: first.x,
            min_y: first.y,
            min_z: first.z,
            max_x: first.x,
            max_y: first.y,
            max_z: first.z,
        };
        for pos in positions {
            result.min_x = result.min_x.min(pos.x);
            result.min_y = result.min_y.min(pos.y);
            result.min_z = result.min_z.min(pos.z);
            result.max_x = result.max_x.max(pos.x);
            result.max_y = result.max_y.max(pos.y);
            result.max_z = result.max_z.max(pos.z);
        }
        Some(result)
    }

    /// `BoundingBox.inflatedBy(x, y, z)` (`BoundingBox.java:207-211`).
    pub fn inflated_by(self, inflate_x: i32, inflate_y: i32, inflate_z: i32) -> Self {
        Self {
            min_x: self.min_x - inflate_x,
            min_y: self.min_y - inflate_y,
            min_z: self.min_z - inflate_z,
            max_x: self.max_x + inflate_x,
            max_y: self.max_y + inflate_y,
            max_z: self.max_z + inflate_z,
        }
    }

    /// `BoundingBox.isInside` (`BoundingBox.java:213-219`).
    pub fn is_inside(&self, pos: BlockPos) -> bool {
        pos.x >= self.min_x
            && pos.x <= self.max_x
            && pos.z >= self.min_z
            && pos.z <= self.max_z
            && pos.y >= self.min_y
            && pos.y <= self.max_y
    }

    /// `BoundingBox.getXSpan` (`BoundingBox.java:225-227`).
    pub fn x_span(&self) -> i32 {
        self.max_x - self.min_x + 1
    }

    /// `BoundingBox.getYSpan` (`BoundingBox.java:229-231`).
    pub fn y_span(&self) -> i32 {
        self.max_y - self.min_y + 1
    }

    /// `BoundingBox.getZSpan` (`BoundingBox.java:233-235`).
    pub fn z_span(&self) -> i32 {
        self.max_z - self.min_z + 1
    }
}

/// `BitSetDiscreteVoxelShape` (`BitSetDiscreteVoxelShape.java:15-21`) as far
/// as the tree uses it: `fill`, `isFull` and `forAllFaces`.
#[derive(Debug, Clone)]
pub struct BitSetDiscreteVoxelShape {
    x_size: i32,
    y_size: i32,
    z_size: i32,
    storage: Vec<bool>,
}

impl BitSetDiscreteVoxelShape {
    pub fn new(x_size: i32, y_size: i32, z_size: i32) -> Self {
        Self {
            x_size,
            y_size,
            z_size,
            storage: vec![false; (x_size * y_size * z_size) as usize],
        }
    }

    /// `BitSetDiscreteVoxelShape.getIndex` (`BitSetDiscreteVoxelShape.java:71-73`).
    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        ((x * self.y_size + y) * self.z_size + z) as usize
    }

    /// `BitSetDiscreteVoxelShape.isFull` (`BitSetDiscreteVoxelShape.java:76-78`).
    pub fn is_full(&self, x: i32, y: i32, z: i32) -> bool {
        self.storage[self.index(x, y, z)]
    }

    /// `BitSetDiscreteVoxelShape.fill` (`BitSetDiscreteVoxelShape.java:80-90`)
    /// with the bounds bookkeeping the tree never reads.
    pub fn fill(&mut self, x: i32, y: i32, z: i32) {
        let index = self.index(x, y, z);
        self.storage[index] = true;
    }

    /// `DiscreteVoxelShape.forAllFaces` (`DiscreteVoxelShape.java:222-226`):
    /// `forAllAxisFaces` (`DiscreteVoxelShape.java:228-262`) under the three
    /// `AxisCycle`s, each walking the cycled axes and emitting the negative
    /// face on entering a run of full cells and the positive face on leaving
    /// it.
    ///
    /// `AxisCycle.NONE` keeps the axes (`AxisCycle.java:4-24`), `FORWARD`'s
    /// inverse `BACKWARD` reads `(x, y, z)` as `(y, z, x)`
    /// (`AxisCycle.java:46-66`), and `BACKWARD`'s inverse `FORWARD` reads them
    /// as `(z, x, y)` (`AxisCycle.java:25-45`), so the walks are Z faces with
    /// x outermost, Y faces with z outermost, then X faces with y outermost.
    pub fn for_all_faces(&self, mut consumer: impl FnMut(Direction, i32, i32, i32)) {
        // `AxisCycle.NONE`: a over x, b over y, c over z; faces north/south.
        self.for_all_axis_faces(
            self.x_size,
            self.y_size,
            self.z_size,
            |a, b, c| (a, b, c),
            Direction::North,
            Direction::South,
            &mut consumer,
        );
        // `AxisCycle.FORWARD` (inverse `BACKWARD`): a over z, b over x, c over
        // y; faces down/up.
        self.for_all_axis_faces(
            self.z_size,
            self.x_size,
            self.y_size,
            |a, b, c| (b, c, a),
            Direction::Down,
            Direction::Up,
            &mut consumer,
        );
        // `AxisCycle.BACKWARD` (inverse `FORWARD`): a over y, b over z, c over
        // x; faces west/east.
        self.for_all_axis_faces(
            self.y_size,
            self.z_size,
            self.x_size,
            |a, b, c| (c, a, b),
            Direction::West,
            Direction::East,
            &mut consumer,
        );
    }

    /// One `DiscreteVoxelShape.forAllAxisFaces` pass
    /// (`DiscreteVoxelShape.java:228-262`); `to_xyz` is the inverse cycle.
    #[allow(clippy::too_many_arguments)]
    fn for_all_axis_faces(
        &self,
        a_size: i32,
        b_size: i32,
        c_size: i32,
        to_xyz: impl Fn(i32, i32, i32) -> (i32, i32, i32),
        negative: Direction,
        positive: Direction,
        consumer: &mut impl FnMut(Direction, i32, i32, i32),
    ) {
        for a in 0..a_size {
            for b in 0..b_size {
                let mut last_full = false;
                for c in 0..=c_size {
                    let full = c != c_size && {
                        let (x, y, z) = to_xyz(a, b, c);
                        self.is_full(x, y, z)
                    };
                    if !last_full && full {
                        let (x, y, z) = to_xyz(a, b, c);
                        consumer(negative, x, y, z);
                    }
                    if last_full && !full {
                        let (x, y, z) = to_xyz(a, b, c - 1);
                        consumer(positive, x, y, z);
                    }
                    last_full = full;
                }
            }
        }
    }
}
