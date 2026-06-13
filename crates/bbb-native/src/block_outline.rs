use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_renderer::{SelectionBox, SelectionOutline};
use bbb_world::{BlockPos, BlockProbe, TerrainMaterialClass};

#[derive(Debug, Clone)]
pub(crate) struct BlockOutlineTarget {
    material: TerrainMaterialClass,
    outline: Option<BlockOutlineShape>,
}

impl BlockOutlineTarget {
    pub(crate) fn full_block(material: TerrainMaterialClass) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
        }
    }

    pub(crate) fn from_probe(probe: &BlockProbe) -> Self {
        Self {
            material: probe.material,
            outline: outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_box(material: TerrainMaterialClass, min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineShape::single(BlockOutlineBox { min, max })),
        }
    }

    pub(crate) fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        if !is_selectable_crosshair_material(self.material) {
            return None;
        }
        self.outline?.clip(eye, direction, max_distance, pos)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum BlockOutlineShape {
    Single(BlockOutlineBox),
    Multi(Vec<BlockOutlineBox>),
}

impl BlockOutlineShape {
    fn single(outline: BlockOutlineBox) -> Self {
        Self::Single(outline)
    }

    fn from_boxes(mut boxes: Vec<BlockOutlineBox>) -> Self {
        if boxes.len() == 1 {
            Self::Single(boxes.pop().expect("one outline box"))
        } else {
            Self::Multi(boxes)
        }
    }

    fn boxes(&self) -> &[BlockOutlineBox] {
        match self {
            Self::Single(outline) => std::slice::from_ref(outline),
            Self::Multi(boxes) => boxes,
        }
    }

    fn clip(
        &self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        self.boxes()
            .iter()
            .filter_map(|outline| outline.clip(eye, direction, max_distance, pos))
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    fn selection_outline(&self, pos: BlockPos) -> SelectionOutline {
        SelectionOutline::from_boxes(
            self.boxes()
                .iter()
                .map(|outline| selection_box_for_outline_box(pos, *outline)),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockOutlineBox {
    min: [f64; 3],
    max: [f64; 3],
}

impl BlockOutlineBox {
    const FULL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    const BOTTOM_SLAB: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 0.5, 1.0],
    };
    const TOP_SLAB: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    const CARPET: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0 / 16.0, 1.0],
    };
    const STAIR_NORTH_WEST_OCTET: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [0.5, 1.0, 0.5],
    };
    const STAIR_NORTH_HALF: Self = Self {
        min: [0.0, 0.5, 0.0],
        max: [1.0, 1.0, 0.5],
    };
    const STAIR_SOUTH_EAST_OCTET: Self = Self {
        min: [0.5, 0.5, 0.5],
        max: [1.0, 1.0, 1.0],
    };
    const PALE_MOSS_NORTH_LOW: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 10.0 / 16.0, 1.0 / 16.0],
    };
    const PALE_MOSS_EAST_LOW: Self = Self {
        min: [15.0 / 16.0, 0.0, 0.0],
        max: [1.0, 10.0 / 16.0, 1.0],
    };
    const PALE_MOSS_SOUTH_LOW: Self = Self {
        min: [0.0, 0.0, 15.0 / 16.0],
        max: [1.0, 10.0 / 16.0, 1.0],
    };
    const PALE_MOSS_WEST_LOW: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0 / 16.0, 10.0 / 16.0, 1.0],
    };
    const PALE_MOSS_NORTH_TALL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0 / 16.0],
    };
    const PALE_MOSS_EAST_TALL: Self = Self {
        min: [15.0 / 16.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    const PALE_MOSS_SOUTH_TALL: Self = Self {
        min: [0.0, 0.0, 15.0 / 16.0],
        max: [1.0, 1.0, 1.0],
    };
    const PALE_MOSS_WEST_TALL: Self = Self {
        min: [0.0, 0.0, 0.0],
        max: [1.0 / 16.0, 1.0, 1.0],
    };

    fn clip(
        self,
        eye: [f64; 3],
        direction: [f64; 3],
        max_distance: f64,
        pos: BlockPos,
    ) -> Option<BlockOutlineHit> {
        let min = [
            f64::from(pos.x) + self.min[0],
            f64::from(pos.y) + self.min[1],
            f64::from(pos.z) + self.min[2],
        ];
        let max = [
            f64::from(pos.x) + self.max[0],
            f64::from(pos.y) + self.max[1],
            f64::from(pos.z) + self.max[2],
        ];

        if contains_point(min, max, eye) {
            return Some(BlockOutlineHit {
                distance: 0.0,
                face: face_opposing_dominant_direction(direction),
                inside: true,
            });
        }

        let mut entry = f64::NEG_INFINITY;
        let mut exit = f64::INFINITY;
        let mut face = face_opposing_dominant_direction(direction);

        for axis in 0..3 {
            if direction[axis] == 0.0 {
                if eye[axis] < min[axis] || eye[axis] > max[axis] {
                    return None;
                }
                continue;
            }

            let t0 = (min[axis] - eye[axis]) / direction[axis];
            let t1 = (max[axis] - eye[axis]) / direction[axis];
            let (near, far, near_face) = if t0 <= t1 {
                (t0, t1, face_for_axis_delta(axis as u8, 1))
            } else {
                (t1, t0, face_for_axis_delta(axis as u8, -1))
            };

            if near > entry {
                entry = near;
                face = near_face;
            }
            exit = exit.min(far);
            if entry > exit {
                return None;
            }
        }

        if entry < 0.0 || entry > max_distance {
            return None;
        }

        Some(BlockOutlineHit {
            distance: entry,
            face,
            inside: false,
        })
    }

    fn invert_y(self) -> Self {
        Self {
            min: [self.min[0], 1.0 - self.max[1], self.min[2]],
            max: [self.max[0], 1.0 - self.min[1], self.max[2]],
        }
    }

    fn rotate_y_90(self) -> Self {
        Self {
            min: [1.0 - self.max[2], self.min[1], self.min[0]],
            max: [1.0 - self.min[2], self.max[1], self.max[0]],
        }
    }

    fn rotate_to_direction(self, direction: HorizontalDirection) -> Self {
        let mut rotated = self;
        for _ in 0..direction.quarter_turns_from_north() {
            rotated = rotated.rotate_y_90();
        }
        rotated
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlockOutlineHit {
    pub(crate) distance: f64,
    pub(crate) face: ProtocolDirection,
    pub(crate) inside: bool,
}

pub(crate) fn selection_outline_for_probe(probe: &BlockProbe) -> Option<SelectionOutline> {
    outline_shape_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| outline.selection_outline(probe.pos))
}

pub(crate) fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    selection_outline_for_box(pos, BlockOutlineBox::FULL)
}

fn selection_outline_for_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionOutline {
    let selection_box = selection_box_for_outline_box(pos, outline);
    SelectionOutline::from_box(selection_box.min, selection_box.max)
}

fn selection_box_for_outline_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionBox {
    SelectionBox {
        min: [
            pos.x as f32 + outline.min[0] as f32,
            pos.y as f32 + outline.min[1] as f32,
            pos.z as f32 + outline.min[2] as f32,
        ],
        max: [
            pos.x as f32 + outline.max[0] as f32,
            pos.y as f32 + outline.max[1] as f32,
            pos.z as f32 + outline.max[2] as f32,
        ],
    }
}

fn contains_point(min: [f64; 3], max: [f64; 3], point: [f64; 3]) -> bool {
    (0..3).all(|axis| point[axis] >= min[axis] && point[axis] <= max[axis])
}

fn outline_shape_for_block(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let block_name = block_name?;
    if block_name == "minecraft:snow" {
        return snow_layer_outline_box(properties).map(BlockOutlineShape::single);
    }
    if is_slab_block_name(block_name) {
        return match properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockOutlineShape::single(BlockOutlineBox::BOTTOM_SLAB)),
            Some("top") => Some(BlockOutlineShape::single(BlockOutlineBox::TOP_SLAB)),
            Some("double") => Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
            _ => None,
        };
    }
    if is_stair_block_name(block_name) {
        return stair_outline_shape(properties);
    }
    if block_name == "minecraft:pale_moss_carpet" {
        return pale_moss_carpet_outline_shape(properties);
    }
    if is_flat_carpet_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::CARPET));
    }
    Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            _ => None,
        }
    }

    fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    fn counter_clockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::East => Self::North,
            Self::South => Self::East,
            Self::West => Self::South,
        }
    }

    fn quarter_turns_from_north(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StairShapeKind {
    Straight,
    Outer,
    Inner,
}

fn stair_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let top = match properties.get("half").map(String::as_str)? {
        "bottom" => false,
        "top" => true,
        _ => return None,
    };
    let (kind, direction) = match properties.get("shape").map(String::as_str)? {
        "straight" => (StairShapeKind::Straight, facing),
        "outer_left" => (StairShapeKind::Outer, facing),
        "outer_right" => (StairShapeKind::Outer, facing.clockwise()),
        "inner_left" => (StairShapeKind::Inner, facing.counter_clockwise()),
        "inner_right" => (StairShapeKind::Inner, facing),
        _ => return None,
    };

    Some(BlockOutlineShape::from_boxes(
        stair_shape_boxes(kind, top)
            .into_iter()
            .map(|outline| outline.rotate_to_direction(direction))
            .collect(),
    ))
}

fn stair_shape_boxes(kind: StairShapeKind, top: bool) -> Vec<BlockOutlineBox> {
    let mut boxes = match kind {
        StairShapeKind::Straight => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_HALF,
        ],
        StairShapeKind::Outer => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_WEST_OCTET,
        ],
        StairShapeKind::Inner => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_HALF,
            BlockOutlineBox::STAIR_SOUTH_EAST_OCTET,
        ],
    };

    if top {
        for outline in &mut boxes {
            *outline = outline.invert_y();
        }
    }
    boxes
}

fn pale_moss_carpet_outline_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let mut boxes = Vec::with_capacity(5);
    match properties.get("bottom").map(String::as_str)? {
        "true" => boxes.push(BlockOutlineBox::CARPET),
        "false" => {}
        _ => return None,
    }

    for (property, low, tall) in [
        (
            "north",
            BlockOutlineBox::PALE_MOSS_NORTH_LOW,
            BlockOutlineBox::PALE_MOSS_NORTH_TALL,
        ),
        (
            "east",
            BlockOutlineBox::PALE_MOSS_EAST_LOW,
            BlockOutlineBox::PALE_MOSS_EAST_TALL,
        ),
        (
            "south",
            BlockOutlineBox::PALE_MOSS_SOUTH_LOW,
            BlockOutlineBox::PALE_MOSS_SOUTH_TALL,
        ),
        (
            "west",
            BlockOutlineBox::PALE_MOSS_WEST_LOW,
            BlockOutlineBox::PALE_MOSS_WEST_TALL,
        ),
    ] {
        match properties.get(property).map(String::as_str)? {
            "none" => {}
            "low" => boxes.push(low),
            "tall" => boxes.push(tall),
            _ => return None,
        }
    }

    if boxes.is_empty() {
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    } else {
        Some(BlockOutlineShape::from_boxes(boxes))
    }
}

fn snow_layer_outline_box(properties: &BTreeMap<String, String>) -> Option<BlockOutlineBox> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    Some(BlockOutlineBox {
        min: [0.0, 0.0, 0.0],
        max: [1.0, f64::from(layers) / 8.0, 1.0],
    })
}

fn is_slab_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_slab"))
}

fn is_stair_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_stairs"))
}

fn is_flat_carpet_block_name(block_name: &str) -> bool {
    let Some(path) = block_name.strip_prefix("minecraft:") else {
        return false;
    };
    matches!(
        path,
        "white_carpet"
            | "orange_carpet"
            | "magenta_carpet"
            | "light_blue_carpet"
            | "yellow_carpet"
            | "lime_carpet"
            | "pink_carpet"
            | "gray_carpet"
            | "light_gray_carpet"
            | "cyan_carpet"
            | "purple_carpet"
            | "blue_carpet"
            | "brown_carpet"
            | "green_carpet"
            | "red_carpet"
            | "black_carpet"
            | "moss_carpet"
    )
}

fn is_selectable_crosshair_material(material: TerrainMaterialClass) -> bool {
    matches!(
        material,
        TerrainMaterialClass::Opaque
            | TerrainMaterialClass::Cutout
            | TerrainMaterialClass::Translucent
    )
}

fn face_for_axis_delta(axis: u8, delta: i32) -> ProtocolDirection {
    match (axis, delta.signum()) {
        (0, 1) => ProtocolDirection::West,
        (0, -1) => ProtocolDirection::East,
        (1, 1) => ProtocolDirection::Down,
        (1, -1) => ProtocolDirection::Up,
        (2, 1) => ProtocolDirection::North,
        (2, -1) => ProtocolDirection::South,
        _ => ProtocolDirection::North,
    }
}

fn face_opposing_dominant_direction(direction: [f64; 3]) -> ProtocolDirection {
    let ax = direction[0].abs();
    let ay = direction[1].abs();
    let az = direction[2].abs();
    if ax >= ay && ax >= az {
        if direction[0] >= 0.0 {
            ProtocolDirection::West
        } else {
            ProtocolDirection::East
        }
    } else if ay >= az {
        if direction[1] >= 0.0 {
            ProtocolDirection::Down
        } else {
            ProtocolDirection::Up
        }
    } else if direction[2] >= 0.0 {
        ProtocolDirection::North
    } else {
        ProtocolDirection::South
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_shape_uses_vanilla_slab_type_property() {
        assert_eq!(
            outline_shape_for_block(Some("minecraft:oak_slab"), &slab_properties("bottom")),
            Some(BlockOutlineShape::single(BlockOutlineBox::BOTTOM_SLAB))
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:smooth_stone_slab"), &slab_properties("top")),
            Some(BlockOutlineShape::single(BlockOutlineBox::TOP_SLAB))
        );
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:petrified_oak_slab"),
                &slab_properties("double"),
            ),
            Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:oak_slab"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:oak_slab"), &slab_properties("unexpected")),
            None
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_snow_layers_property() {
        assert_eq!(
            outline_shape_for_block(Some("minecraft:snow"), &snow_properties(1)),
            Some(BlockOutlineShape::single(BlockOutlineBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 0.125, 1.0],
            }))
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:snow"), &snow_properties(8)),
            Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:snow"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:snow"), &snow_properties(9)),
            None
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_stair_straight_boxes() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "bottom", "straight"),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::BOTTOM_SLAB,
                BlockOutlineBox::STAIR_NORTH_HALF,
            ]))
        );
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("east", "bottom", "straight"),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::BOTTOM_SLAB,
                BlockOutlineBox {
                    min: [0.5, 0.5, 0.0],
                    max: [1.0, 1.0, 1.0],
                },
            ]))
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_top_stair_boxes() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "top", "straight"),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::TOP_SLAB,
                BlockOutlineBox {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 0.5, 0.5],
                },
            ]))
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_stair_corner_boxes() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "bottom", "outer_right"),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::BOTTOM_SLAB,
                BlockOutlineBox {
                    min: [0.5, 0.5, 0.0],
                    max: [1.0, 1.0, 0.5],
                },
            ]))
        );
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "bottom", "inner_left"),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::BOTTOM_SLAB,
                BlockOutlineBox {
                    min: [0.0, 0.5, 0.0],
                    max: [0.5, 1.0, 1.0],
                },
                BlockOutlineBox {
                    min: [0.5, 0.5, 0.0],
                    max: [1.0, 1.0, 0.5],
                },
            ]))
        );
    }

    #[test]
    fn outline_shape_rejects_invalid_stair_properties() {
        assert_eq!(
            outline_shape_for_block(Some("minecraft:oak_stairs"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "middle", "straight"),
            ),
            None
        );
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("north", "bottom", "sideways"),
            ),
            None
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_flat_carpet_shape() {
        assert_eq!(
            outline_shape_for_block(Some("minecraft:white_carpet"), &BTreeMap::new()),
            Some(BlockOutlineShape::single(BlockOutlineBox::CARPET))
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:moss_carpet"), &BTreeMap::new()),
            Some(BlockOutlineShape::single(BlockOutlineBox::CARPET))
        );
        assert!(!is_flat_carpet_block_name("minecraft:pale_moss_carpet"));
    }

    #[test]
    fn outline_shape_uses_vanilla_pale_moss_carpet_boxes() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:pale_moss_carpet"),
                &pale_moss_properties(
                    true,
                    [
                        ("north", "low"),
                        ("east", "tall"),
                        ("south", "none"),
                        ("west", "none"),
                    ],
                ),
            ),
            Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::CARPET,
                BlockOutlineBox::PALE_MOSS_NORTH_LOW,
                BlockOutlineBox::PALE_MOSS_EAST_TALL,
            ]))
        );
    }

    #[test]
    fn outline_shape_uses_vanilla_pale_moss_empty_shape_fallback() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:pale_moss_carpet"),
                &pale_moss_properties(
                    false,
                    [
                        ("north", "none"),
                        ("east", "none"),
                        ("south", "none"),
                        ("west", "none"),
                    ],
                ),
            ),
            Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
        );
    }

    #[test]
    fn outline_shape_rejects_invalid_pale_moss_properties() {
        assert_eq!(
            outline_shape_for_block(
                Some("minecraft:pale_moss_carpet"),
                &pale_moss_properties(
                    true,
                    [
                        ("north", "low"),
                        ("east", "unexpected"),
                        ("south", "none"),
                        ("west", "none"),
                    ],
                ),
            ),
            None
        );
        assert_eq!(
            outline_shape_for_block(Some("minecraft:pale_moss_carpet"), &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn selection_outline_uses_slab_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                BlockOutlineBox::BOTTOM_SLAB,
            ),
            SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.5, 5.0])
        );
        assert_eq!(
            selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::TOP_SLAB,),
            SelectionOutline::from_box([-2.0, 63.5, 4.0], [-1.0, 64.0, 5.0])
        );
    }

    #[test]
    fn selection_outline_uses_snow_layer_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                snow_layer_outline_box(&snow_properties(3)).unwrap(),
            ),
            SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.375, 5.0])
        );
    }

    #[test]
    fn selection_outline_uses_flat_carpet_bounds() {
        assert_eq!(
            selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::CARPET),
            SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 63.0625, 5.0])
        );
    }

    #[test]
    fn selection_outline_preserves_pale_moss_multi_box_shape() {
        let shape = pale_moss_carpet_outline_shape(&pale_moss_properties(
            true,
            [
                ("north", "low"),
                ("east", "tall"),
                ("south", "none"),
                ("west", "none"),
            ],
        ))
        .unwrap();

        assert_eq!(
            shape.selection_outline(BlockPos { x: -2, y: 63, z: 4 }),
            SelectionOutline::from_boxes([
                SelectionBox {
                    min: [-2.0, 63.0, 4.0],
                    max: [-1.0, 63.0625, 5.0],
                },
                SelectionBox {
                    min: [-2.0, 63.0, 4.0],
                    max: [-1.0, 63.625, 4.0625],
                },
                SelectionBox {
                    min: [-1.0625, 63.0, 4.0],
                    max: [-1.0, 64.0, 5.0],
                },
            ])
        );
    }

    #[test]
    fn multi_box_outline_clip_uses_nearest_hit() {
        let target = BlockOutlineTarget {
            material: TerrainMaterialClass::Opaque,
            outline: Some(BlockOutlineShape::from_boxes(vec![
                BlockOutlineBox::PALE_MOSS_EAST_TALL,
                BlockOutlineBox::PALE_MOSS_NORTH_LOW,
            ])),
        };

        assert_eq!(
            target.clip(
                [0.5, 0.5, -1.0],
                [0.0, 0.0, 1.0],
                4.5,
                BlockPos { x: 0, y: 0, z: 0 },
            ),
            Some(BlockOutlineHit {
                distance: 1.0,
                face: ProtocolDirection::North,
                inside: false,
            })
        );
    }

    #[test]
    fn stair_outline_clip_hits_step_face_inside_block() {
        let target = BlockOutlineTarget {
            material: TerrainMaterialClass::Opaque,
            outline: outline_shape_for_block(
                Some("minecraft:oak_stairs"),
                &stair_properties("south", "bottom", "straight"),
            ),
        };

        assert_eq!(
            target.clip(
                [0.5, 0.62, -1.0],
                [0.0, 0.0, 1.0],
                4.5,
                BlockPos { x: 0, y: 0, z: 0 },
            ),
            Some(BlockOutlineHit {
                distance: 1.5,
                face: ProtocolDirection::North,
                inside: false,
            })
        );
    }

    fn slab_properties(slab_type: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("type".to_string(), slab_type.to_string())])
    }

    fn snow_properties(layers: u8) -> BTreeMap<String, String> {
        BTreeMap::from([("layers".to_string(), layers.to_string())])
    }

    fn stair_properties(facing: &str, half: &str, shape: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("facing".to_string(), facing.to_string()),
            ("half".to_string(), half.to_string()),
            ("shape".to_string(), shape.to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ])
    }

    fn pale_moss_properties(bottom: bool, sides: [(&str, &str); 4]) -> BTreeMap<String, String> {
        let mut properties = BTreeMap::from([("bottom".to_string(), bottom.to_string())]);
        for (name, value) in sides {
            properties.insert(name.to_string(), value.to_string());
        }
        properties
    }
}
