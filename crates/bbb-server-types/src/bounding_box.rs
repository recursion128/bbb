//! `BoundingBox` (`BoundingBox.java`), the integer box structures, gametests
//! and `/clone` share.
//!
//! It is not `bbb-server-core`'s `geometry::Aabb`: that one is the `double` box entity
//! movement sweeps against, this one is a closed range of block coordinates
//! whose `maxX` is a cell rather than a face.

use bbb_protocol::packets::BlockPos;

/// A closed box of block coordinates.
///
/// The constructor's inverted-bounds repair (`BoundingBox.java:48-64`) is not a
/// separate arm here: nothing can build one of these except
/// [`BoundingBox::from_corners`], which sorts each axis the way
/// `fromCorners` (`:66-75`) does, so the repaired shape is the only shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    min: BlockPos,
    max: BlockPos,
}

impl BoundingBox {
    /// `BoundingBox.fromCorners` (`BoundingBox.java:66-75`).
    #[must_use]
    pub fn from_corners(a: BlockPos, b: BlockPos) -> Self {
        Self {
            min: BlockPos {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
                z: a.z.min(b.z),
            },
            max: BlockPos {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
                z: a.z.max(b.z),
            },
        }
    }

    #[must_use]
    pub const fn min(self) -> BlockPos {
        self.min
    }

    #[must_use]
    pub const fn max(self) -> BlockPos {
        self.max
    }

    /// `BoundingBox.getXSpan` (`:225-227`).
    #[must_use]
    pub const fn x_span(self) -> i32 {
        self.max.x - self.min.x + 1
    }

    /// `BoundingBox.getYSpan` (`:229-231`).
    #[must_use]
    pub const fn y_span(self) -> i32 {
        self.max.y - self.min.y + 1
    }

    /// `BoundingBox.getZSpan` (`:233-235`).
    #[must_use]
    pub const fn z_span(self) -> i32 {
        self.max.z - self.min.z + 1
    }

    /// `BoundingBox.getLength` (`:221-223`), which is one less than the span on
    /// every axis because it is an offset rather than a count.
    #[must_use]
    pub const fn length(self) -> BlockPos {
        BlockPos {
            x: self.max.x - self.min.x,
            y: self.max.y - self.min.y,
            z: self.max.z - self.min.z,
        }
    }

    /// `BoundingBox.isInside(int, int, int)` (`:217-219`).
    #[must_use]
    pub const fn is_inside(self, pos: BlockPos) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.z >= self.min.z
            && pos.z <= self.max.z
            && pos.y >= self.min.y
            && pos.y <= self.max.y
    }

    /// `BoundingBox.intersects(BoundingBox)` (`:114-121`).
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.max.x >= other.min.x
            && self.min.x <= other.max.x
            && self.max.z >= other.min.z
            && self.min.z <= other.max.z
            && self.max.y >= other.min.y
            && self.min.y <= other.max.y
    }

    /// `BlockPos.betweenClosed(minX, minY, minZ, maxX, maxY, maxZ)` in
    /// `Cursor3D`'s own order (`Cursor3D.java:30-40`): x fastest, then y, then
    /// z.
    pub fn positions(self) -> impl Iterator<Item = BlockPos> {
        (self.min.z..=self.max.z).flat_map(move |z| {
            (self.min.y..=self.max.y)
                .flat_map(move |y| (self.min.x..=self.max.x).map(move |x| BlockPos { x, y, z }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    #[test]
    fn from_corners_sorts_every_axis_independently() {
        let box_ = BoundingBox::from_corners(pos(4, -3, 9), pos(-1, 7, 2));
        assert_eq!(box_.min(), pos(-1, -3, 2));
        assert_eq!(box_.max(), pos(4, 7, 9));
        // `:225-235` and `:221-223`: the span counts cells, the length is the
        // offset between the corners.
        assert_eq!((box_.x_span(), box_.y_span(), box_.z_span()), (6, 11, 8));
        assert_eq!(box_.length(), pos(5, 10, 7));
    }

    #[test]
    fn a_single_cell_box_spans_one_and_has_zero_length() {
        let box_ = BoundingBox::from_corners(pos(3, 4, 5), pos(3, 4, 5));
        assert_eq!((box_.x_span(), box_.y_span(), box_.z_span()), (1, 1, 1));
        assert_eq!(box_.length(), pos(0, 0, 0));
        assert!(box_.is_inside(pos(3, 4, 5)));
        assert!(!box_.is_inside(pos(3, 4, 6)));
    }

    #[test]
    fn is_inside_is_closed_on_all_six_faces() {
        let box_ = BoundingBox::from_corners(pos(0, 0, 0), pos(2, 2, 2));
        for axis in 0..3 {
            for face in [-1, 3] {
                let mut probe = [1, 1, 1];
                probe[axis] = face;
                assert!(!box_.is_inside(pos(probe[0], probe[1], probe[2])));
            }
        }
        assert!(box_.is_inside(pos(0, 2, 1)));
        assert!(box_.is_inside(pos(2, 0, 2)));
    }

    #[test]
    fn intersects_is_inclusive_on_a_shared_face() {
        let box_ = BoundingBox::from_corners(pos(0, 0, 0), pos(2, 2, 2));
        let touching = BoundingBox::from_corners(pos(2, 2, 2), pos(4, 4, 4));
        let apart = BoundingBox::from_corners(pos(3, 0, 0), pos(4, 2, 2));
        assert!(box_.intersects(touching));
        assert!(touching.intersects(box_));
        assert!(!box_.intersects(apart));
        assert!(!apart.intersects(box_));
    }

    #[test]
    fn positions_walk_x_fastest_then_y_then_z() {
        let box_ = BoundingBox::from_corners(pos(0, 0, 0), pos(1, 1, 1));
        let walked: Vec<_> = box_.positions().collect();
        assert_eq!(
            walked,
            vec![
                pos(0, 0, 0),
                pos(1, 0, 0),
                pos(0, 1, 0),
                pos(1, 1, 0),
                pos(0, 0, 1),
                pos(1, 0, 1),
                pos(0, 1, 1),
                pos(1, 1, 1),
            ]
        );
    }
}
