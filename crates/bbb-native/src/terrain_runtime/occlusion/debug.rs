//! Debug gizmo projection for the native section graph.

use bbb_renderer::{
    terrain::TerrainFace, CameraFrustum, DebugFilledBox, DebugLine, DebugTextBillboard,
};

use super::{direction_step, SectionPosition, TerrainSectionOcclusionGraph};

const VISIBILITY_LINE_COLOR: [u8; 4] = [255, 0, 0, 255];
const VISIBILITY_FILL_COLOR: [u8; 4] = [229, 229, 0, 51];
const OCTREE_CLOSE_LABEL_COLOR: [u8; 4] = [0, 255, 0, 255];
const OCTREE_LABEL_COLOR: [u8; 4] = [255; 4];
const OCTREE_LABEL_SCALE: f32 = 4.8;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct ChunkSectionDebugGizmos {
    pub(crate) octree_lines: Vec<DebugLine>,
    pub(crate) octree_text_billboards: Vec<DebugTextBillboard>,
    pub(crate) culling_lines: Vec<DebugLine>,
    pub(crate) culling_filled_boxes: Vec<DebugFilledBox>,
}

impl TerrainSectionOcclusionGraph {
    pub(crate) fn debug_gizmos(
        &self,
        frustum: CameraFrustum,
        show_paths: bool,
        show_octree: bool,
        show_visibility: bool,
    ) -> ChunkSectionDebugGizmos {
        let mut gizmos = ChunkSectionDebugGizmos::default();
        if !self.valid {
            return gizmos;
        }
        if show_octree {
            self.append_octree_gizmos(frustum, &mut gizmos);
        }
        if show_paths || show_visibility {
            self.append_culling_gizmos(frustum, show_paths, show_visibility, &mut gizmos);
        }
        gizmos
    }

    fn append_octree_gizmos(&self, frustum: CameraFrustum, output: &mut ChunkSectionDebugGizmos) {
        let mut leaf_count = 0_u32;
        for visit in self.octree_visits(frustum, 32) {
            let size = (i64::from(visit.max_exclusive[0]) - i64::from(visit.min[0]))
                .saturating_div(16)
                .max(1);
            if visit.section.is_some() {
                leaf_count = leaf_count.saturating_add(1);
                let mut label = DebugTextBillboard::centered(
                    bounds_center(visit.min, visit.max_exclusive),
                    leaf_count.to_string(),
                    if visit.is_close {
                        OCTREE_CLOSE_LABEL_COLOR
                    } else {
                        OCTREE_LABEL_COLOR
                    },
                );
                label.scale = OCTREE_LABEL_SCALE;
                output.octree_text_billboards.push(label);
            }

            let deflate = 0.1 * visit.depth as f32;
            let min = visit.min.map(|coordinate| coordinate as f32 + deflate);
            let max = visit
                .max_exclusive
                .map(|coordinate| coordinate as f32 - deflate);
            output.octree_lines.extend(DebugLine::cuboid(
                min,
                max,
                octree_color(size.saturating_add(5), visit.fully_visible),
                DebugLine::DEFAULT_WIDTH,
            ));
        }
    }

    fn append_culling_gizmos(
        &self,
        frustum: CameraFrustum,
        show_paths: bool,
        show_visibility: bool,
        output: &mut ChunkSectionDebugGizmos,
    ) {
        for position in self.visible_sections(frustum) {
            let Some(node) = self.nodes.get(&position).copied() else {
                continue;
            };
            if show_paths {
                let center = section_center(position);
                let color = if node.step == 0 {
                    [0, 0, 0, 255]
                } else {
                    crate::remote_debug::vanilla_hsv_rgb(node.step as f32 / 50.0, 0.9, 0.9)
                };
                for direction in TerrainFace::ALL {
                    if !node.has_source_direction(direction) {
                        continue;
                    }
                    let step = direction_step(direction);
                    output.culling_lines.push(DebugLine::new(
                        center,
                        std::array::from_fn(|axis| center[axis] - 16.0 * step[axis] as f32),
                        color,
                        DebugLine::DEFAULT_WIDTH,
                    ));
                }
            }

            let Some(visibility) = self.section_visibility.get(&position).copied() else {
                continue;
            };
            if !show_visibility || !visibility.has_renderable_layers {
                continue;
            }
            let origin = position.origin();
            let mut blocked_pairs = 0;
            for from in TerrainFace::ALL {
                for to in TerrainFace::ALL {
                    if visibility.faces_can_see_each_other(from, to) {
                        continue;
                    }
                    blocked_pairs += 1;
                    output.culling_lines.push(DebugLine::new(
                        face_center(origin, from),
                        face_center(origin, to),
                        VISIBILITY_LINE_COLOR,
                        DebugLine::DEFAULT_WIDTH,
                    ));
                }
            }
            if blocked_pairs > 0 {
                output.culling_filled_boxes.push(DebugFilledBox {
                    min: origin.map(|coordinate| coordinate as f32 + 0.5),
                    max: origin.map(|coordinate| coordinate as f32 + 15.5),
                    color: VISIBILITY_FILL_COLOR,
                });
            }
        }
    }
}

fn section_center(position: SectionPosition) -> [f32; 3] {
    position.origin().map(|coordinate| coordinate as f32 + 8.0)
}

fn face_center(origin: [i32; 3], face: TerrainFace) -> [f32; 3] {
    let step = direction_step(face);
    std::array::from_fn(|axis| origin[axis] as f32 + 8.0 + 8.0 * step[axis] as f32)
}

fn bounds_center(min: [i32; 3], max_exclusive: [i32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| (min[axis] as f32 + max_exclusive[axis] as f32) * 0.5)
}

fn octree_color(size: i64, fully_visible: bool) -> [u8; 4] {
    [
        color_channel(octree_color_component(size, 0.3)),
        color_channel(octree_color_component(size, 0.8)),
        color_channel(octree_color_component(size, 0.5)),
        color_channel(if fully_visible { 0.4 } else { 1.0 }),
    ]
}

fn octree_color_component(size: i64, multiplier: f32) -> f32 {
    (multiplier * size as f32).fract() * 0.9 + 0.1
}

fn color_channel(value: f32) -> u8 {
    (value * 255.0).floor().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use bbb_renderer::{terrain::TerrainSectionVisibility, CameraPose};
    use bbb_world::ChunkPos;

    use super::*;
    use crate::terrain_runtime::occlusion::{ResidentSection, TerrainCoordinateSet};

    fn section(x: i32, y: i32, z: i32, visibility: TerrainSectionVisibility) -> ResidentSection {
        ResidentSection {
            position: SectionPosition { x, y, z },
            visibility,
        }
    }

    fn loaded_square(radius: i32) -> TerrainCoordinateSet<ChunkPos> {
        (-radius..=radius)
            .flat_map(|z| (-radius..=radius).map(move |x| ChunkPos { x, z }))
            .collect()
    }

    fn frustum() -> CameraFrustum {
        CameraFrustum::from_pose(
            CameraPose {
                position: [8.0, 0.0, 8.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 8.0,
            },
            1.0,
            120.0,
        )
    }

    #[test]
    fn chunk_section_debug_paths_follow_source_direction_and_step_color() {
        let sections = [
            section(
                0,
                0,
                0,
                TerrainSectionVisibility::all_visible(0, true, false),
            ),
            section(
                0,
                0,
                1,
                TerrainSectionVisibility::all_visible(0, true, false),
            ),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, true);

        let gizmos = graph.debug_gizmos(frustum(), true, false, false);
        assert_eq!(
            gizmos.culling_lines,
            [DebugLine::new(
                [8.0, 8.0, 24.0],
                [8.0, 8.0, 8.0],
                crate::remote_debug::vanilla_hsv_rgb(1.0 / 50.0, 0.9, 0.9),
                DebugLine::DEFAULT_WIDTH,
            )]
        );
        assert!(gizmos.culling_filled_boxes.is_empty());
        assert!(gizmos.octree_lines.is_empty());
    }

    #[test]
    fn chunk_section_debug_visibility_emits_blocked_pairs_and_fill() {
        let sections = [section(
            0,
            0,
            0,
            TerrainSectionVisibility::none_visible(0, true, false),
        )];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, true);

        let gizmos = graph.debug_gizmos(frustum(), false, false, true);
        assert_eq!(gizmos.culling_lines.len(), 36);
        assert!(gizmos
            .culling_lines
            .iter()
            .all(|line| line.color == VISIBILITY_LINE_COLOR));
        assert_eq!(
            gizmos.culling_lines[TerrainFace::Down.index()],
            DebugLine::new(
                [8.0, 0.0, 8.0],
                [8.0, 0.0, 8.0],
                VISIBILITY_LINE_COLOR,
                DebugLine::DEFAULT_WIDTH,
            )
        );
        assert_eq!(
            gizmos.culling_filled_boxes,
            [DebugFilledBox {
                min: [0.5; 3],
                max: [15.5; 3],
                color: VISIBILITY_FILL_COLOR,
            }]
        );
    }

    #[test]
    fn chunk_section_debug_octree_numbers_camera_ordered_leaves() {
        let sections = [
            section(
                0,
                0,
                0,
                TerrainSectionVisibility::all_visible(0, true, false),
            ),
            section(
                0,
                0,
                1,
                TerrainSectionVisibility::all_visible(0, true, false),
            ),
            section(
                1,
                0,
                1,
                TerrainSectionVisibility::all_visible(0, true, false),
            ),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, false);
        let visits = graph.octree_visits(frustum(), 32);

        let gizmos = graph.debug_gizmos(frustum(), false, true, false);
        assert_eq!(gizmos.octree_lines.len(), visits.len() * 12);
        assert_eq!(
            gizmos
                .octree_text_billboards
                .iter()
                .map(|label| (
                    label.text.as_str(),
                    label.position,
                    label.scale,
                    label.color
                ))
                .collect::<Vec<_>>(),
            vec![
                ("1", [8.0, 8.0, 8.0], 4.8, OCTREE_CLOSE_LABEL_COLOR),
                ("2", [8.0, 8.0, 24.0], 4.8, OCTREE_CLOSE_LABEL_COLOR),
                ("3", [24.0, 8.0, 24.0], 4.8, OCTREE_CLOSE_LABEL_COLOR),
            ]
        );
        assert_eq!(octree_color(6, true), [209, 209, 25, 102]);
        assert_eq!(octree_color(6, false), [209, 209, 25, 255]);
        assert_eq!(
            crate::remote_debug::vanilla_hsv_rgb(1.02, 0.9, 0.9),
            [229, 255, 22, 255],
            "path colors preserve Mth.hsvToRgb's unwrapped-hue clamp quirk"
        );
    }
}
