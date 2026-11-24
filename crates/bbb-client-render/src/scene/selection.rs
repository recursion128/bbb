pub use crate::render_core::selection::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_line_sanitize_and_renderer_setter_clear_invalid_or_stale_lines() {
        let valid = DebugLine::new([0.0; 3], [1.0, 0.0, 0.0], [255; 4], 2.5);
        assert_eq!(sanitize_debug_lines(vec![valid]), [valid]);
        assert!(
            sanitize_debug_lines(vec![DebugLine::new([0.0; 3], [0.0; 3], [255; 4], 2.5,)])
                .is_empty()
        );
        assert!(sanitize_debug_lines(vec![DebugLine::new(
            [f32::NAN, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [255; 4],
            2.5,
        )])
        .is_empty());

        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_debug_lines(vec![valid]);
        assert_eq!(renderer.debug_lines, [valid]);
        renderer.set_debug_lines(Vec::new());
        assert!(renderer.debug_lines.is_empty());
    }

    #[test]
    fn debug_line_draws_through_item_entity_target_and_transparency_composite() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(64, 64) else {
            return;
        };
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer.set_level_frame_route(crate::LevelFrameRoute::ImprovedTransparency);
        renderer.set_debug_lines(vec![DebugLine::new(
            [-0.5, 0.0, 2.0],
            [0.5, 0.0, 2.0],
            [255; 4],
            2.5,
        )]);

        let pixels = renderer
            .render_offscreen_frame()
            .expect("debug line frame readback");
        let drew_white_line = (24..40).any(|y| {
            (16..48).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180
            })
        });
        assert!(drew_white_line, "debug line should reach the framebuffer");
        let corner = pixels.pixel(4, 4);
        assert!(
            corner[2] > 128 && corner[0] < 64,
            "background outside the line stays blue, got {corner:?}"
        );
    }
}
