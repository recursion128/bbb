use crate::noise::scattered::HorizontalDirection;

/// `BoundingBox.orientBox` (`BoundingBox.java:81-103`). SOUTH is the default.
#[allow(clippy::too_many_arguments)]
pub fn orient_box(
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    off_x: i32,
    off_y: i32,
    off_z: i32,
    width: i32,
    height: i32,
    depth: i32,
    direction: HorizontalDirection,
) -> [i32; 6] {
    let box_ = match direction {
        HorizontalDirection::South => [
            foot_x.wrapping_add(off_x),
            foot_y.wrapping_add(off_y),
            foot_z.wrapping_add(off_z),
            foot_x
                .wrapping_add(width)
                .wrapping_sub(1)
                .wrapping_add(off_x),
            foot_y
                .wrapping_add(height)
                .wrapping_sub(1)
                .wrapping_add(off_y),
            foot_z
                .wrapping_add(depth)
                .wrapping_sub(1)
                .wrapping_add(off_z),
        ],
        HorizontalDirection::North => [
            foot_x.wrapping_add(off_x),
            foot_y.wrapping_add(off_y),
            foot_z
                .wrapping_sub(depth)
                .wrapping_add(1)
                .wrapping_add(off_z),
            foot_x
                .wrapping_add(width)
                .wrapping_sub(1)
                .wrapping_add(off_x),
            foot_y
                .wrapping_add(height)
                .wrapping_sub(1)
                .wrapping_add(off_y),
            foot_z.wrapping_add(off_z),
        ],
        HorizontalDirection::West => [
            foot_x
                .wrapping_sub(depth)
                .wrapping_add(1)
                .wrapping_add(off_z),
            foot_y.wrapping_add(off_y),
            foot_z.wrapping_add(off_x),
            foot_x.wrapping_add(off_z),
            foot_y
                .wrapping_add(height)
                .wrapping_sub(1)
                .wrapping_add(off_y),
            foot_z
                .wrapping_add(width)
                .wrapping_sub(1)
                .wrapping_add(off_x),
        ],
        HorizontalDirection::East => [
            foot_x.wrapping_add(off_z),
            foot_y.wrapping_add(off_y),
            foot_z.wrapping_add(off_x),
            foot_x
                .wrapping_add(depth)
                .wrapping_sub(1)
                .wrapping_add(off_z),
            foot_y
                .wrapping_add(height)
                .wrapping_sub(1)
                .wrapping_add(off_y),
            foot_z
                .wrapping_add(width)
                .wrapping_sub(1)
                .wrapping_add(off_x),
        ],
    };
    normalize_box(box_)
}

/// `StructurePiece.makeBoundingBox` (`StructurePiece.java:72-77`).
pub fn make_bounding_box(
    x: i32,
    y: i32,
    z: i32,
    direction: HorizontalDirection,
    width: i32,
    height: i32,
    depth: i32,
) -> [i32; 6] {
    let (span_x, span_z) = if direction.is_z_axis() {
        (width, depth)
    } else {
        (depth, width)
    };
    normalize_box([
        x,
        y,
        z,
        x.wrapping_add(span_x).wrapping_sub(1),
        y.wrapping_add(height).wrapping_sub(1),
        z.wrapping_add(span_z).wrapping_sub(1),
    ])
}

fn normalize_box(box_: [i32; 6]) -> [i32; 6] {
    [
        box_[0].min(box_[3]),
        box_[1].min(box_[4]),
        box_[2].min(box_[5]),
        box_[0].max(box_[3]),
        box_[1].max(box_[4]),
        box_[2].max(box_[5]),
    ]
}

pub const fn min_x(box_: [i32; 6]) -> i32 {
    box_[0]
}

pub const fn min_y(box_: [i32; 6]) -> i32 {
    box_[1]
}

pub const fn min_z(box_: [i32; 6]) -> i32 {
    box_[2]
}

pub const fn max_x(box_: [i32; 6]) -> i32 {
    box_[3]
}

pub const fn max_y(box_: [i32; 6]) -> i32 {
    box_[4]
}

pub const fn max_z(box_: [i32; 6]) -> i32 {
    box_[5]
}

pub const fn x_span(box_: [i32; 6]) -> i32 {
    max_x(box_).wrapping_sub(min_x(box_)).wrapping_add(1)
}

/// `BoundingBox.getYSpan` (`BoundingBox.java:229-230`).
pub const fn y_span(box_: [i32; 6]) -> i32 {
    max_y(box_).wrapping_sub(min_y(box_)).wrapping_add(1)
}

pub const fn z_span(box_: [i32; 6]) -> i32 {
    max_z(box_).wrapping_sub(min_z(box_)).wrapping_add(1)
}

pub const fn intersects(left: [i32; 6], right: [i32; 6]) -> bool {
    max_x(left) >= min_x(right)
        && min_x(left) <= max_x(right)
        && max_z(left) >= min_z(right)
        && min_z(left) <= max_z(right)
        && max_y(left) >= min_y(right)
        && min_y(left) <= max_y(right)
}

pub fn union_boxes(pieces: &[[i32; 6]]) -> Option<[i32; 6]> {
    let mut iter = pieces.iter().copied();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        [
            min_x(acc).min(min_x(next)),
            min_y(acc).min(min_y(next)),
            min_z(acc).min(min_z(next)),
            max_x(acc).max(max_x(next)),
            max_y(acc).max(max_y(next)),
            max_z(acc).max(max_z(next)),
        ]
    }))
}
