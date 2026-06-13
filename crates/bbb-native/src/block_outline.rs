use std::collections::BTreeMap;

use bbb_protocol::packets::Direction as ProtocolDirection;
use bbb_renderer::SelectionOutline;
use bbb_world::{BlockPos, BlockProbe, TerrainMaterialClass};

#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockOutlineTarget {
    material: TerrainMaterialClass,
    outline: Option<BlockOutlineBox>,
}

impl BlockOutlineTarget {
    pub(crate) fn full_block(material: TerrainMaterialClass) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineBox::FULL),
        }
    }

    pub(crate) fn from_probe(probe: &BlockProbe) -> Self {
        Self {
            material: probe.material,
            outline: outline_box_for_block(probe.block_name.as_deref(), &probe.block_properties),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_box(material: TerrainMaterialClass, min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            material,
            outline: Some(BlockOutlineBox { min, max }),
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlockOutlineHit {
    pub(crate) distance: f64,
    pub(crate) face: ProtocolDirection,
    pub(crate) inside: bool,
}

pub(crate) fn selection_outline_for_probe(probe: &BlockProbe) -> Option<SelectionOutline> {
    outline_box_for_block(probe.block_name.as_deref(), &probe.block_properties)
        .map(|outline| selection_outline_for_box(probe.pos, outline))
}

pub(crate) fn selection_outline_for_block(pos: BlockPos) -> SelectionOutline {
    selection_outline_for_box(pos, BlockOutlineBox::FULL)
}

fn selection_outline_for_box(pos: BlockPos, outline: BlockOutlineBox) -> SelectionOutline {
    SelectionOutline {
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

fn outline_box_for_block(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineBox> {
    let block_name = block_name?;
    if block_name == "minecraft:snow" {
        return snow_layer_outline_box(properties);
    }
    if is_slab_block_name(block_name) {
        return match properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockOutlineBox::BOTTOM_SLAB),
            Some("top") => Some(BlockOutlineBox::TOP_SLAB),
            Some("double") => Some(BlockOutlineBox::FULL),
            _ => None,
        };
    }
    if is_flat_carpet_block_name(block_name) {
        return Some(BlockOutlineBox::CARPET);
    }
    Some(BlockOutlineBox::FULL)
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
    fn outline_box_uses_vanilla_slab_type_property() {
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &slab_properties("bottom")),
            Some(BlockOutlineBox::BOTTOM_SLAB)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:smooth_stone_slab"), &slab_properties("top")),
            Some(BlockOutlineBox::TOP_SLAB)
        );
        assert_eq!(
            outline_box_for_block(
                Some("minecraft:petrified_oak_slab"),
                &slab_properties("double"),
            ),
            Some(BlockOutlineBox::FULL)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:oak_slab"), &slab_properties("unexpected")),
            None
        );
    }

    #[test]
    fn outline_box_uses_vanilla_snow_layers_property() {
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(1)),
            Some(BlockOutlineBox {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 0.125, 1.0],
            })
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(8)),
            Some(BlockOutlineBox::FULL)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &BTreeMap::new()),
            None
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:snow"), &snow_properties(9)),
            None
        );
    }

    #[test]
    fn outline_box_uses_vanilla_flat_carpet_shape() {
        assert_eq!(
            outline_box_for_block(Some("minecraft:white_carpet"), &BTreeMap::new()),
            Some(BlockOutlineBox::CARPET)
        );
        assert_eq!(
            outline_box_for_block(Some("minecraft:moss_carpet"), &BTreeMap::new()),
            Some(BlockOutlineBox::CARPET)
        );
        assert!(!is_flat_carpet_block_name("minecraft:pale_moss_carpet"));
    }

    #[test]
    fn selection_outline_uses_slab_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                BlockOutlineBox::BOTTOM_SLAB,
            ),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.5, 5.0],
            }
        );
        assert_eq!(
            selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::TOP_SLAB,),
            SelectionOutline {
                min: [-2.0, 63.5, 4.0],
                max: [-1.0, 64.0, 5.0],
            }
        );
    }

    #[test]
    fn selection_outline_uses_snow_layer_bounds() {
        assert_eq!(
            selection_outline_for_box(
                BlockPos { x: -2, y: 63, z: 4 },
                snow_layer_outline_box(&snow_properties(3)).unwrap(),
            ),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.375, 5.0],
            }
        );
    }

    #[test]
    fn selection_outline_uses_flat_carpet_bounds() {
        assert_eq!(
            selection_outline_for_box(BlockPos { x: -2, y: 63, z: 4 }, BlockOutlineBox::CARPET),
            SelectionOutline {
                min: [-2.0, 63.0, 4.0],
                max: [-1.0, 63.0625, 5.0],
            }
        );
    }

    fn slab_properties(slab_type: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("type".to_string(), slab_type.to_string())])
    }

    fn snow_properties(layers: u8) -> BTreeMap<String, String> {
        BTreeMap::from([("layers".to_string(), layers.to_string())])
    }
}
