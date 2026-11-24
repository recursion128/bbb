use serde::{Deserialize, Serialize};

pub const SELECTION_OUTLINE_ALPHA: f32 = 102.0 / 255.0;
const DEFAULT_SELECTION_LINE_WIDTH: f32 = 1.0;

const fn default_selection_line_width() -> f32 {
    DEFAULT_SELECTION_LINE_WIDTH
}

fn is_default_selection_line_width(width: &f32) -> bool {
    width.to_bits() == DEFAULT_SELECTION_LINE_WIDTH.to_bits()
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// The render phase selected from Vanilla's baked block-state material flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockSelectionPhase {
    BeforeTranslucent,
    AfterTranslucent,
}

/// Immutable per-frame block-hit outline projection.
///
/// Native control resolves block shape, resource-generation material flags, surface-scaled width,
/// and accessibility style before this crosses into the renderer. The renderer owns only GPU
/// residency and pass placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSelectionRenderState {
    pub boxes: Vec<SelectionBox>,
    pub phase: BlockSelectionPhase,
    pub line_width: f32,
    pub high_contrast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionColoredBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub color: [f32; 4],
    #[serde(
        default = "default_selection_line_width",
        skip_serializing_if = "is_default_selection_line_width"
    )]
    pub line_width: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionLine {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub color: [f32; 4],
    #[serde(
        default = "default_selection_line_width",
        skip_serializing_if = "is_default_selection_line_width"
    )]
    pub width: f32,
}

/// One world-space line submitted through vanilla's gizmo `lines()` or
/// `linesTranslucent()` surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DebugLine {
    pub from: [f32; 3],
    pub to: [f32; 3],
    /// RGBA8 after extracting the components from vanilla's ARGB color.
    pub color: [u8; 4],
    /// Screen-space width in pixels. `GizmoStyle.stroke(...)` defaults to 2.5.
    pub width: f32,
}

/// `CuboidGizmo`'s `ARGB.multiply` operands for a colored-corner stroke, in
/// vanilla's `-34953` / `-8913033` / `-8947713` order.
const CORNER_X_AXIS_TINT: [u8; 4] = [0xFF, 0x77, 0x77, 0xFF];
const CORNER_Y_AXIS_TINT: [u8; 4] = [0x77, 0xFF, 0x77, 0xFF];
const CORNER_Z_AXIS_TINT: [u8; 4] = [0x77, 0x77, 0xFF, 0xFF];

/// `ARGB.multiply`: per-channel `lhs * rhs / 255`.
fn multiply_rgba(lhs: [u8; 4], rhs: [u8; 4]) -> [u8; 4] {
    std::array::from_fn(|channel| (u32::from(lhs[channel]) * u32::from(rhs[channel]) / 255) as u8)
}

impl DebugLine {
    pub const DEFAULT_WIDTH: f32 = 2.5;

    pub fn new(from: [f32; 3], to: [f32; 3], color: [u8; 4], width: f32) -> Self {
        Self {
            from,
            to,
            color,
            width,
        }
    }

    /// Emits `CuboidGizmo`'s twelve stroke edges in vanilla order.
    pub fn cuboid(min: [f32; 3], max: [f32; 3], color: [u8; 4], width: f32) -> [Self; 12] {
        Self::cuboid_edges(min, max, color, width, false)
    }

    /// `CuboidGizmo` with `coloredCornerStroke`: the three edges meeting at the
    /// min corner are multiplied by the axis tints `0xFFFF7777` / `0xFF77FF77`
    /// / `0xFF7777FF`, marking +X/+Y/+Z.
    pub fn cuboid_with_colored_corner(
        min: [f32; 3],
        max: [f32; 3],
        color: [u8; 4],
        width: f32,
    ) -> [Self; 12] {
        Self::cuboid_edges(min, max, color, width, true)
    }

    fn cuboid_edges(
        min: [f32; 3],
        max: [f32; 3],
        color: [u8; 4],
        width: f32,
        colored_corner: bool,
    ) -> [Self; 12] {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        let corner = |axis_tint: [u8; 4]| {
            if colored_corner {
                multiply_rgba(color, axis_tint)
            } else {
                color
            }
        };
        [
            Self::new(
                [x0, y0, z0],
                [x1, y0, z0],
                corner(CORNER_X_AXIS_TINT),
                width,
            ),
            Self::new(
                [x0, y0, z0],
                [x0, y1, z0],
                corner(CORNER_Y_AXIS_TINT),
                width,
            ),
            Self::new(
                [x0, y0, z0],
                [x0, y0, z1],
                corner(CORNER_Z_AXIS_TINT),
                width,
            ),
            Self::new([x1, y0, z0], [x1, y1, z0], color, width),
            Self::new([x1, y1, z0], [x0, y1, z0], color, width),
            Self::new([x0, y1, z0], [x0, y1, z1], color, width),
            Self::new([x0, y1, z1], [x0, y0, z1], color, width),
            Self::new([x0, y0, z1], [x1, y0, z1], color, width),
            Self::new([x1, y0, z1], [x1, y0, z0], color, width),
            Self::new([x0, y1, z1], [x1, y1, z1], color, width),
            Self::new([x1, y0, z1], [x1, y1, z1], color, width),
            Self::new([x1, y1, z0], [x1, y1, z1], color, width),
        ]
    }

    pub fn sanitized(self) -> Option<Self> {
        let finite_positions = self
            .from
            .iter()
            .chain(self.to.iter())
            .all(|coordinate| coordinate.is_finite());
        let nonzero = self.from != self.to;
        (finite_positions && nonzero && self.width.is_finite() && self.width > 0.0).then_some(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionPoint {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionTextLabel {
    pub position: [f32; 3],
    pub text: String,
    pub color: [f32; 4],
    #[serde(default, skip_serializing_if = "is_false")]
    pub centered: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionOutline {
    pub boxes: Vec<SelectionBox>,
    #[serde(default)]
    pub colored_boxes: Vec<SelectionColoredBox>,
    #[serde(default)]
    pub lines: Vec<SelectionLine>,
    #[serde(default)]
    pub points: Vec<SelectionPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_labels: Vec<SelectionTextLabel>,
}

impl SelectionOutline {
    pub fn from_box(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            boxes: vec![SelectionBox { min, max }],
            colored_boxes: Vec::new(),
            lines: Vec::new(),
            points: Vec::new(),
            text_labels: Vec::new(),
        }
    }

    pub fn from_boxes(boxes: impl IntoIterator<Item = SelectionBox>) -> Self {
        Self {
            boxes: boxes.into_iter().collect(),
            colored_boxes: Vec::new(),
            lines: Vec::new(),
            points: Vec::new(),
            text_labels: Vec::new(),
        }
    }

    pub fn from_colored_boxes_and_lines(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
    ) -> Self {
        Self::from_colored_boxes_lines_and_points(
            colored_boxes,
            lines,
            std::iter::empty::<SelectionPoint>(),
        )
    }

    pub fn from_colored_boxes_lines_and_points(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
        points: impl IntoIterator<Item = SelectionPoint>,
    ) -> Self {
        Self::from_colored_boxes_lines_points_and_labels(
            colored_boxes,
            lines,
            points,
            std::iter::empty::<SelectionTextLabel>(),
        )
    }

    pub fn from_colored_boxes_lines_points_and_labels(
        colored_boxes: impl IntoIterator<Item = SelectionColoredBox>,
        lines: impl IntoIterator<Item = SelectionLine>,
        points: impl IntoIterator<Item = SelectionPoint>,
        text_labels: impl IntoIterator<Item = SelectionTextLabel>,
    ) -> Self {
        Self {
            boxes: Vec::new(),
            colored_boxes: colored_boxes.into_iter().collect(),
            lines: lines.into_iter().collect(),
            points: points.into_iter().collect(),
            text_labels: text_labels.into_iter().collect(),
        }
    }
}
