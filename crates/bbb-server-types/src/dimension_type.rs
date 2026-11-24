//! DimensionType vertical section geometry and built-in 26.1 dimension shapes.

/// The section height every dimension height and `min_y` is a multiple of
/// (`DimensionType.java:74-80`).
pub const SECTION_HEIGHT: i32 = 16;

/// The shape a level takes from its dimension type (`DimensionType.java:83-107`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionTypeShape {
    pub has_fixed_time: bool,
    pub has_skylight: bool,
    pub has_ceiling: bool,
    pub has_ender_dragon_fight: bool,
    pub coordinate_scale: f64,
    pub min_y: i32,
    pub height: i32,
    pub logical_height: i32,
}

impl DimensionTypeShape {
    /// The exclusive top of the level, `min_y + height`.
    pub const fn max_y_exclusive(&self) -> i32 {
        self.min_y + self.height
    }

    /// The number of 16-block sections a chunk column of this dimension holds.
    pub const fn section_count(&self) -> i32 {
        self.height / SECTION_HEIGHT
    }
}

/// Built-in 26.1 `data/minecraft/dimension_type/*.json` shapes without snapshot I/O.
pub const fn builtin_dimension_type_shape_facts(element_id: &str) -> Option<DimensionTypeShape> {
    if const_str_eq(element_id, "minecraft:overworld") {
        Some(DimensionTypeShape {
            has_fixed_time: false,
            has_skylight: true,
            has_ceiling: false,
            has_ender_dragon_fight: false,
            coordinate_scale: 1.0,
            min_y: -64,
            height: 384,
            logical_height: 384,
        })
    } else if const_str_eq(element_id, "minecraft:overworld_caves") {
        Some(DimensionTypeShape {
            has_fixed_time: false,
            has_skylight: true,
            has_ceiling: true,
            has_ender_dragon_fight: false,
            coordinate_scale: 1.0,
            min_y: -64,
            height: 384,
            logical_height: 384,
        })
    } else if const_str_eq(element_id, "minecraft:the_nether") {
        Some(DimensionTypeShape {
            has_fixed_time: true,
            has_skylight: false,
            has_ceiling: true,
            has_ender_dragon_fight: false,
            coordinate_scale: 8.0,
            min_y: 0,
            height: 256,
            logical_height: 128,
        })
    } else if const_str_eq(element_id, "minecraft:the_end") {
        Some(DimensionTypeShape {
            has_fixed_time: true,
            has_skylight: true,
            has_ceiling: false,
            has_ender_dragon_fight: true,
            coordinate_scale: 1.0,
            min_y: 0,
            height: 256,
            logical_height: 256,
        })
    } else {
        None
    }
}

const fn const_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
