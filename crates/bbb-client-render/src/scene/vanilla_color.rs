//! `Mth.hsvToRgb` projection shared by the debug gizmo builders.

pub fn vanilla_hsv_rgb(hue: f32, saturation: f32, value: f32) -> [u8; 4] {
    // Keep `Mth.hsvToRgb`'s Java ordering: the sector is reduced modulo six
    // before it is subtracted from the unwrapped hue. Values above one
    // therefore clamp rather than becoming purely periodic.
    let sector = (hue * 6.0) as i32 % 6;
    let fraction = hue * 6.0 - sector as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - fraction * saturation);
    let t = value * (1.0 - (1.0 - fraction) * saturation);
    let (r, g, b) = match sector.rem_euclid(6) {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    [
        ((r * 255.0) as i32).clamp(0, 255) as u8,
        ((g * 255.0) as i32).clamp(0, 255) as u8,
        ((b * 255.0) as i32).clamp(0, 255) as u8,
        255,
    ]
}
