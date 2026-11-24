use super::*;

#[test]
fn chat_text_backdrop_keeps_low_alpha_as_untextured_fill() {
    let mut vertices = Vec::new();
    let mut commands = Vec::new();
    let surface = PhysicalSize::new(320, 240);
    push_hud_text_backdrop(
        &mut vertices,
        &mut commands,
        surface,
        (10.0, 20.0),
        30,
        1.0,
        0.05,
        1.0,
    );
    assert!(matches!(
        commands.as_slice(),
        [HudDrawCommand::Fill { start: 0, end: 6 }]
    ));
    let alpha = hud_text_backdrop_alpha(0.05, 1.0);
    assert!(alpha > 0.0 && alpha < 0.1);
    assert!(vertices
        .iter()
        .all(|vertex| vertex.tint == [0.0, 0.0, 0.0, alpha]));
    push_hud_text_backdrop(
        &mut vertices,
        &mut commands,
        surface,
        (10.0, 20.0),
        30,
        1.0,
        0.0,
        1.0,
    );
    assert_eq!(vertices.len(), 6);
}
