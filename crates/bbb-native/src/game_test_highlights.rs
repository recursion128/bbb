//! Immutable render extraction for vanilla GameTest block highlights.

use bbb_renderer::{DebugFilledBox, DebugTextBillboard};
use bbb_world::{BlockPos, WorldStore};

const MARKER_PADDING: f32 = 0.02;
const MARKER_COLOR: [u8; 4] = [0, 255, 0, 96];
const LABEL_COLOR: [u8; 4] = [255; 4];
const LABEL_SCALE: f32 = 0.16;
const LABEL_HEIGHT: f32 = 1.2;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct GameTestHighlightGizmos {
    pub(crate) filled_boxes: Vec<DebugFilledBox>,
    pub(crate) text_billboards: Vec<DebugTextBillboard>,
}

pub(crate) fn game_test_highlight_gizmos(world: &WorldStore) -> GameTestHighlightGizmos {
    let mut output = GameTestHighlightGizmos::default();
    for (absolute_pos, marker) in world.game_test_highlight_markers().iter() {
        output
            .filled_boxes
            .push(game_test_highlight_box(*absolute_pos));
        output.text_billboards.push(game_test_highlight_label(
            *absolute_pos,
            marker.relative_pos,
        ));
    }
    output
}

fn game_test_highlight_box(pos: BlockPos) -> DebugFilledBox {
    DebugFilledBox {
        min: [
            pos.x as f32 - MARKER_PADDING,
            pos.y as f32 - MARKER_PADDING,
            pos.z as f32 - MARKER_PADDING,
        ],
        max: [
            pos.x as f32 + 1.0 + MARKER_PADDING,
            pos.y as f32 + 1.0 + MARKER_PADDING,
            pos.z as f32 + 1.0 + MARKER_PADDING,
        ],
        color: MARKER_COLOR,
    }
}

fn game_test_highlight_label(absolute_pos: BlockPos, relative_pos: BlockPos) -> DebugTextBillboard {
    let mut label = DebugTextBillboard::centered(
        [
            absolute_pos.x as f32 + 0.5,
            absolute_pos.y as f32 + LABEL_HEIGHT,
            absolute_pos.z as f32 + 0.5,
        ],
        format!("{}, {}, {}", relative_pos.x, relative_pos.y, relative_pos.z),
        LABEL_COLOR,
    );
    label.scale = LABEL_SCALE;
    label.always_on_top = true;
    label
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, GameTestHighlightPos};

    use super::*;

    #[test]
    fn game_test_highlight_gizmos_match_vanilla_geometry_color_and_text() {
        let mut world = WorldStore::new();
        world.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: ProtocolBlockPos { x: 4, y: -5, z: 6 },
                relative_pos: ProtocolBlockPos { x: -7, y: 8, z: -9 },
            },
            100,
        );
        world.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: ProtocolBlockPos {
                    x: -10,
                    y: 11,
                    z: 12,
                },
                relative_pos: ProtocolBlockPos {
                    x: 13,
                    y: 14,
                    z: 15,
                },
            },
            200,
        );

        let gizmos = game_test_highlight_gizmos(&world);

        assert_eq!(
            gizmos.filled_boxes,
            [
                DebugFilledBox {
                    min: [-10.02, 10.98, 11.98],
                    max: [-8.98, 12.02, 13.02],
                    color: [0, 255, 0, 96],
                },
                DebugFilledBox {
                    min: [3.98, -5.02, 5.98],
                    max: [5.02, -3.98, 7.02],
                    color: [0, 255, 0, 96],
                },
            ]
        );
        assert_eq!(gizmos.text_billboards.len(), 2);
        assert_eq!(
            gizmos.text_billboards[0],
            DebugTextBillboard {
                position: [-9.5, 12.2, 12.5],
                text: "13, 14, 15".to_string(),
                color: [255; 4],
                scale: 0.16,
                adjust_left: None,
                always_on_top: true,
            }
        );
        assert_eq!(
            gizmos.text_billboards[1],
            DebugTextBillboard {
                position: [4.5, -3.8, 6.5],
                text: "-7, 8, -9".to_string(),
                color: [255; 4],
                scale: 0.16,
                adjust_left: None,
                always_on_top: true,
            }
        );
    }

    #[test]
    fn game_test_highlight_gizmos_are_empty_without_active_markers() {
        assert_eq!(
            game_test_highlight_gizmos(&WorldStore::new()),
            GameTestHighlightGizmos::default()
        );
    }
}
