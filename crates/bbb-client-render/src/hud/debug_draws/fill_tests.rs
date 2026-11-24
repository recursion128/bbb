use super::*;

#[test]
fn debug_lightmap_border_uses_fill_before_preview() {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    push_hud_debug_lightmap_preview(&mut vertices, &mut commands, PhysicalSize::new(320, 240));
    assert!(matches!(
        commands.as_slice(),
        [
            HudDrawCommand::Fill { start: 0, end: 6 },
            HudDrawCommand::LightmapPreview { start: 6, end: 12 }
        ]
    ));
    assert!(vertices[..6]
        .iter()
        .all(|vertex| vertex.tint == [0.0, 0.0, 0.0, 1.0]));
}

#[test]
fn debug_chart_lines_preserve_fill_tints_and_skip_empty_spans() {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(320, 240);
    let tint = [0.2, 0.4, 0.8, 0.05];
    push_hud_debug_chart_horizontal_line(&mut vertices, &mut commands, surface, 10, 0, 20, tint);
    push_hud_debug_chart_vertical_line(&mut vertices, &mut commands, surface, 10, 20, 0, tint);
    assert!(commands.is_empty());
    push_hud_debug_chart_horizontal_line(&mut vertices, &mut commands, surface, 10, 12, 20, tint);
    push_hud_debug_chart_vertical_line(&mut vertices, &mut commands, surface, 10, 20, 12, tint);
    assert!(commands
        .iter()
        .all(|command| matches!(command, HudDrawCommand::Fill { .. })));
    assert_eq!(vertices.len(), 12);
    assert!(vertices.iter().all(|vertex| vertex.tint == tint));
}
